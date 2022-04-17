use bevy::{
    prelude::*, 
    math::Vec4Swizzles, 
    render::{
        render_resource::*, 
        mesh::Indices,
        renderer::{RenderDevice, RenderQueue}
    }, 
    core::{cast_slice, Pod}, tasks::{ComputeTaskPool, AsyncComputeTaskPool, Task}, pbr::wireframe::WireframeConfig,
};

use bytemuck::Zeroable;
use opensimplex_noise_rs::OpenSimplexNoise;
use std::{iter::once, sync::Arc, ops::RangeInclusive};
use std::time::Instant;
use futures_lite::future;


pub const AXIS_SIZE: usize = 32;
pub const BUFFER_SIZE: usize = AXIS_SIZE * AXIS_SIZE * AXIS_SIZE;

// big brain bit masks and shifts
pub const Y_MASK: usize = 0b_0111_1100_0000_0000;
pub const Z_MASK: usize = 0b_0000_0011_1110_0000;
pub const X_MASK: usize = 0b_0000_0000_0001_1111;

pub const Y_SHIFT: usize = 10;
pub const Z_SHIFT: usize = 5;
pub const X_SHIFT: usize = 0;


pub fn to_index(local: IVec3) -> usize {
    (local.x << X_SHIFT | local.y << Y_SHIFT | local.z << Z_SHIFT) as usize
}

fn from_index(index: usize) -> IVec3 {
    IVec3::new(
        ((index & X_MASK) >> X_SHIFT) as i32,
        ((index & Y_MASK) >> Y_SHIFT) as i32,
        ((index & Z_MASK) >> Z_SHIFT) as i32,
    )
}
//         e6
//     7-------6
//    /|      /|
// e7/ |e11e5/ |e10
//  4--|e4--5  |
//e8|  3--e2|--2
//  | /e3 e9| /e1
//  0-------1
//      e0

#[derive(Component, Clone, Debug, Copy)]
pub struct Chunk {
    points: [f32; BUFFER_SIZE],
    dirty: bool,
}


impl Chunk {
    pub fn new(points: [f32; BUFFER_SIZE], dirty: bool) -> Self { Self { points, dirty } }

    pub fn new_empty() -> Self {
        Self {points: [-1.0; BUFFER_SIZE], dirty: false}
    }
}

#[derive(Bundle)]
pub struct ChunkBundle {
    pub chunk: Chunk,

    #[bundle]
    pub pbr: PbrBundle,
}

#[derive(Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
pub struct Triangle {
    pub a: Vec4,
    pub b: Vec4,
    pub c: Vec4,
}

pub struct ChunkPlugin;

impl Plugin for ChunkPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<ChunkPipeline>()
            .insert_resource(Arc::new(OpenSimplexNoise::new(Some(69420))))
            .insert_resource(ChunkSpawnTimer(Timer::from_seconds(1.0, true)))
            .add_system_to_stage(CoreStage::First, assign_generated_chunks)
            .add_system_to_stage(CoreStage::PreUpdate, chunk_generation_system)
            .add_system_to_stage(CoreStage::Update, compute_mesh)
            .add_system_to_stage(CoreStage::PostUpdate, spawn_chunk_system);
    }
}


struct ChunkCumputeBuffers {
    point_buffer: Buffer,
    atomics_buffer: Buffer,
    triangle_buffer: Buffer,
}

impl ChunkCumputeBuffers {
    fn new_empty(render_device: &RenderDevice) -> Self{
        let point_buffer = render_device.create_buffer(&BufferDescriptor {
            label: None,
            size: (std::mem::size_of::<f32>() * BUFFER_SIZE) as u64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let atomics_buffer = render_device.create_buffer(&BufferDescriptor {
            label: None,
            size: (std::mem::size_of::<u32>() * 1) as u64,
            usage: BufferUsages::STORAGE | BufferUsages::MAP_READ| BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let triangle_buffer = render_device.create_buffer(&BufferDescriptor {
            label: None,
            size: (std::mem::size_of::<Triangle>()as u64) * BUFFER_SIZE as u64 * 4,
            usage: BufferUsages::STORAGE | BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        Self {point_buffer, atomics_buffer, triangle_buffer}
    }
}

//add derive Deref in future (when in main bevy release)

pub struct  ChunkPipeline {
    buffer_bind_group_layout: BindGroupLayout,
    march_pipeline: ComputePipeline,
}

impl FromWorld for ChunkPipeline {
    fn from_world(world: &mut World) -> Self {
        // let world = world.cell();
        // let asset_server = world.get_resource::<AssetServer>().unwrap();
        let render_device = world.get_resource::<RenderDevice>().unwrap();
        
       
        let shader_source = include_str!("../../assets/shaders/marchig_cubes.wgsl");
        let shader = render_device.create_shader_module(&ShaderModuleDescriptor {
            label: None,
            source: ShaderSource::Wgsl(shader_source.into()),
        });

        let buffer_bind_group_layout =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: None,
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer { 
                            ty: BufferBindingType::Storage { read_only: true }, 
                            has_dynamic_offset: false, 
                            min_binding_size: None
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer { 
                            ty: BufferBindingType::Storage { read_only: false }, 
                            has_dynamic_offset: false, 
                            min_binding_size: None
                        },
                        count: None,
                    }, 
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer { 
                            ty: BufferBindingType::Storage { read_only: false }, 
                            has_dynamic_offset: false, 
                            min_binding_size: None
                        },
                        count: None,
                    }   
                ]
            });

        let chunk_buffers = ChunkCumputeBuffers::new_empty(render_device);
        
        
    
        
        let pipeline_layout = render_device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&buffer_bind_group_layout],
            push_constant_ranges: &[],
        });
        
        let march_pipeline = render_device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "march",
        });
        
        world.insert_resource(chunk_buffers);

        ChunkPipeline {
            buffer_bind_group_layout,
            march_pipeline,
          }
    }
}

fn compute_mesh(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    pipeline: Res<ChunkPipeline>,
    chunk_buffers: Res<ChunkCumputeBuffers>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut query: Query<(&mut Chunk, &Handle<Mesh>)>,
) {
    let mut tri_count = 0;

    let start = Instant::now();
    let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
        label: None,
        layout: &pipeline.buffer_bind_group_layout,
        entries: &[
            BindGroupEntry {
                binding: 0,
                resource: chunk_buffers.point_buffer.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 1,
                resource: chunk_buffers.atomics_buffer.as_entire_binding()
            },
            BindGroupEntry {
                binding: 2,
                resource: chunk_buffers.triangle_buffer.as_entire_binding()
            }
        ],
    });
    for (mut chunk, mesh_handle) in query.iter_mut() {
        if !chunk.dirty {continue;}
        let bytes: &[u8] = cast_slice(&chunk.points);
        render_queue.write_buffer(&chunk_buffers.point_buffer, 0, &bytes[..]);


        render_queue.write_buffer(&chunk_buffers.atomics_buffer, 0, cast_slice(&[0]));

        let mut command_encoder = render_device.create_command_encoder(&CommandEncoderDescriptor { label: Some("mesh command encoder") });
        {
            let mut pass = command_encoder.begin_compute_pass(&ComputePassDescriptor::default());
            pass.set_pipeline(&pipeline.march_pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch(4, 4, 4)
        }
        render_queue.submit(once(command_encoder.finish()));

        let slice = &chunk_buffers.atomics_buffer.slice(..);
        render_device.map_buffer(slice, MapMode::Read);
        let tri_head: u32 = cast_slice(&slice.get_mapped_range()[..])[0];
        chunk_buffers.atomics_buffer.unmap();
        
        let range = 0..std::mem::size_of::<Triangle>() * tri_head as usize;
        let slice = &chunk_buffers.triangle_buffer.slice(..) ;
        render_device.map_buffer(slice, MapMode::Read);
        let triangles: Vec<Triangle> = Vec::from(cast_slice(&slice.get_mapped_range()[range]));
        chunk_buffers.triangle_buffer.unmap();
        
        tri_count += triangles.len();

        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
        
        let mut vertices: Vec<[f32; 3]> = Vec::new();
        triangles.iter().for_each(|x| {
            vertices.append(&mut vec![
                x.a.xyz().to_array(),
                x.b.xyz().to_array(),
                x.c.xyz().to_array(),
                ]);
            });
            
        let length = vertices.len() as u32;
        let indices = (0..length as u32).collect::<Vec<u32>>();
        let uvs: Vec<[f32; 2]> = vec![[0.0, 0.0]; length as usize];
        mesh.set_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
        mesh.compute_flat_normals();
        mesh.set_indices(Some(Indices::U32(indices)));
        mesh.set_attribute(Mesh::ATTRIBUTE_UV_0, uvs);

        *meshes.get_mut(mesh_handle).unwrap() = mesh;
        chunk.dirty = false;
    }

    let elapsed = start.elapsed();
    if elapsed.as_millis() < 1 {return}
    println!("Mesh took: {:.2?} for {} triangles", elapsed, tri_count);
}

struct ChunkSpawnTimer(Timer);

fn spawn_chunk_system(
    mut commands: Commands,
    cameras: Query<&Transform, With<Camera>>,
    chunks: Query<&Transform, With<Chunk>>,
    mut wireframe_config: ResMut<WireframeConfig>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    time: Res<Time>,
    mut timer: ResMut<ChunkSpawnTimer>,
) {
    if !timer.0.tick(time.delta()).just_finished() {return}

    let mut chunk_positions: Vec<Vec3> = Vec::new();
    chunks.for_each(|x| chunk_positions.push((x.translation / (AXIS_SIZE-1) as f32).floor()));

    for transform in cameras.iter() {
        let cam_position = (transform.translation / (AXIS_SIZE-1) as f32).floor();
        let range_h: RangeInclusive<i32> = -5..=5;
        let range_v: RangeInclusive<i32> = -1..=2;
        
        for x in range_h.clone() {
            for y in range_v.clone() {
                for z in range_h.clone() {
                    let pos  = cam_position + Vec3::new(x as f32, y as f32, z as f32);
                    
                    if !chunk_positions.contains(&pos) {
                        commands.spawn_bundle(ChunkBundle {
                            chunk: Chunk::new_empty(),

                            pbr: PbrBundle {
                                mesh: meshes.add(Mesh::new(PrimitiveTopology::TriangleList)),
                                transform: Transform::from_xyz((AXIS_SIZE-1) as f32  * pos.x , (AXIS_SIZE-1) as f32 * pos.y as f32, (AXIS_SIZE-1) as f32 * pos.z),
                                material: materials.add(Color::DARK_GREEN.into()),
                                ..Default::default()
                            },
                        });
                    }
                }
            }
        }
    }
}

fn chunk_generation_system(
    query: Query<(Entity, &Transform), Added<Chunk>,>,
    pool: Res<AsyncComputeTaskPool>,
    key: Res<Input<KeyCode>>,
    simplex: Res<Arc<OpenSimplexNoise>>,
    mut commands: Commands
) {
    // if !key.just_pressed(KeyCode::G){
    //     return;
    // }
    if query.is_empty() {return}
    
    for (entity, transform) in query.iter() {
        let simplex = simplex.clone();
        let transform = transform.clone();
        let task = pool.spawn(async move {
            let mut points = [0.0f32 ;BUFFER_SIZE];
            for i in 0..BUFFER_SIZE-1 {
                points[i] = calc_iso(transform.translation + from_index(i).as_vec3(), &simplex);
            }
            Chunk {
                points,
                dirty: true,
            }
        });
        commands.entity(entity).insert(task);
    }

    // query.par_for_each_mut(&pool, 32, |(mut chunk, transform)| {
    //     for i in 0..BUFFER_SIZE-1 {
    //         chunk.points[i] = calc_iso(transform.translation + from_index(i).as_vec3(), &simplex);
    //     }
    //     chunk.dirty = true; 
    // });

}

fn assign_generated_chunks(
    mut commands: Commands,
    mut gen_tasks: Query<(Entity, &mut Chunk, &mut Task<Chunk>)>,
) {
    for (entity, mut chunk, mut task) in gen_tasks.iter_mut() {
        if let Some(new_chunk) = future::block_on(future::poll_once(&mut *task)) {
            chunk.points = new_chunk.points;
            chunk.dirty = new_chunk.dirty;
            commands.entity(entity).remove::<Task<Chunk>>();
        }
    }
}

fn calc_iso(ws: Vec3, simplex: &OpenSimplexNoise) -> f32{
    let mut density = -ws.y;

    let mut freq = 0.015;
    let mut amplitude = 10.0;
    for _ in 0..=9 {
        density += (simplex.eval_3d(ws.x as f64 * freq, ws.y as f64 * freq, ws.z as f64 * freq) as f32 + 1.0) * amplitude;
        freq *= 2.0;
        amplitude *= 0.5;
    }
    density
}