use bevy::{
    prelude::*, 
    math::Vec4Swizzles, 
    render::{
        render_resource::{PrimitiveTopology, Buffer, BindGroup, ShaderModuleDescriptor, ShaderSource, BindGroupLayoutDescriptor, BindGroupLayoutEntry, ShaderStages, BindingType, BufferBindingType, BufferSize, BindGroupLayout, PipelineLayoutDescriptor, ComputePipelineDescriptor, ComputePipeline, BindGroupDescriptor, BindGroupEntry, BufferUsages, BufferDescriptor, ComputePassDescriptor, CommandEncoderDescriptor, MapMode}, 
        mesh::{Indices, VertexAttributeValues}, 
        RenderApp, 
        render_graph::{RenderGraph, self}, renderer::{RenderDevice, RenderQueue}, RenderStage, render_component::ExtractComponent}, core_pipeline::node::MAIN_PASS_DEPENDENCIES, ecs::{entity::{self, Entities}, component::Components, archetype::Archetypes}, core::{cast_slice, Pod}};
use bytemuck::Zeroable;
use opensimplex_noise_rs::OpenSimplexNoise;
use std::{borrow::Cow, num::NonZeroU32, iter::once};

use super::marching_cubes_tables::{TRI_TABLE, CORNER_INDEX_AFROM_EDGE, CORNER_INDEX_BFROM_EDGE};

pub const AXIS_SIZE: usize = 32;
pub const BUFFER_SIZE: usize = AXIS_SIZE * AXIS_SIZE * AXIS_SIZE;

// big brain bit masks and shifts
pub const Y_MASK: usize = 0b_0111_1100_0000_0000;
pub const Z_MASK: usize = 0b_0000_0011_1110_0000;
pub const X_MASK: usize = 0b_0000_0000_0001_1111;

pub const Y_SHIFT: usize = 10;
pub const Z_SHIFT: usize = 5;
pub const X_SHIFT: usize = 0;

pub const ISO_LEVEL: f32 = 0.0;


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

pub fn gen_cube(start: Vec3) -> [Vec3; 8] {
    [
        /*0*/start,
        /*1*/Vec3::new(start.x + 1.0, start.y, start.z),
        /*2*/Vec3::new(start.x + 1.0, start.y, start.z + 1.0),
        /*3*/Vec3::new(start.x, start.y, start.z + 1.0),
        /*4*/Vec3::new(start.x, start.y + 1.0, start.z),
        /*5*/Vec3::new(start.x + 1.0, start.y + 1.0, start.z),
        /*6*/Vec3::new(start.x + 1.0, start.y + 1.0, start.z + 1.0),
        /*7*/Vec3::new(start.x, start.y + 1.0, start.z + 1.0),
    ]
}

fn interpolate_verts(v1: Vec4, v2: Vec4) -> Vec3{
    let t = (ISO_LEVEL - v1.w) / (v2.w - v1.w);
    v1.xyz() + t * (v2.xyz() - v1.xyz())
}

#[derive(Component)]
pub struct DirtyChunk;

#[derive(Component, Clone, Debug, Copy)]
pub struct Chunk {
    points: [f32; BUFFER_SIZE],
    dirty: bool,
}


impl Chunk {
    pub fn new_empty() -> Self {
        Self {points: [-1.0; BUFFER_SIZE], dirty: false}
    }

    pub fn get_cube(self, pos: Vec3) -> [Vec4; 8] {
        
        let cube = gen_cube(pos);
        cube.map(|x| { Vec4::new(x.x, x.y, x.z, self.points[to_index(x.as_ivec3())]) })
    
    }

    pub fn set_point(mut self, pos: Vec3, value: f32) {
        self.points[to_index(pos.as_ivec3())] = value;
        self.dirty = true;
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
        println!("{:?}", std::mem::size_of::<Triangle>());

        app
            .init_resource::<ChunkPipeline>()
            // .add_system(march_cubes_system)
            .add_system_to_stage(CoreStage::PreUpdate, set_points_system)
            .add_system_to_stage(CoreStage::Update, compute_mesh);

            // .add_system_to_stage(RenderStage::Extract, extract_chunks)
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
            usage: BufferUsages::STORAGE | BufferUsages::MAP_READ,
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
    
    for (mut chunk, mesh_handle) in query.iter_mut() {
        if !chunk.dirty {continue;}
        let bytes: &[u8] = cast_slice(&chunk.points);
        render_queue.write_buffer(&chunk_buffers.point_buffer, 0, &bytes[..]);

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
        
        let mut command_encoder = render_device.create_command_encoder(&CommandEncoderDescriptor { label: Some("mesh command encoder") });
        {
            let mut pass = command_encoder.begin_compute_pass(&ComputePassDescriptor::default());
            pass.set_pipeline(&pipeline.march_pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch(8, 8, 8)
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
        
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
        
        let mut vertices: Vec<[f32; 3]> = Vec::new();
        triangles.iter().for_each(|x| {
            vertices.append(&mut vec![
                x.a.xyz().to_array(),
                x.b.xyz().to_array(),
                x.c.xyz().to_array(),
                ])
            });
            
        // println!("{:?}", vertices);
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

}





// struct DispatchChunk;

// impl render_graph::Node for DispatchChunk {

//     fn run(
//         &self,
//         _graph: &mut render_graph::RenderGraphContext,
//         render_context: &mut bevy::render::renderer::RenderContext,
//         world: &World,
//     ) -> Result<(), render_graph::NodeRunError> {
//         let pipeline = world.get_resource::<ChunkPipeline>().unwrap();
//         let group = &world.get_resource::<ChunkBindGroup>().unwrap().0;

//         let mut pass = render_context
//             .command_encoder
//             .begin_compute_pass(&ComputePassDescriptor::default());
        
//         pass.set_pipeline(&pipeline.march_pipeline);
        
//         {
//             pass.set_bind_group(0, group, &[]);
//             pass.dispatch(8, 8, 8)
//         }

//         Ok(())
//     }
// }







// fn march_cubes_system(
//     mut query: Query<(&Handle<Mesh>, &mut Chunk)>,
//     mut meshes: ResMut<Assets<Mesh>>,
// ) {
//     for (mesh_handle, mut chunk) in query.iter_mut() {
//         if !chunk.dirty {continue;}
//         let mut vertices: Vec<[f32; 3]> = Vec::new();

//         for i in 0..BUFFER_SIZE-1 {
//                 if from_index(i).max_element() >= AXIS_SIZE as i32 -1 {continue;}
//                 let points = chunk.clone().get_cube(from_index(i).as_vec3());
//                 let mut triangles: Vec<Triangle> = Vec::with_capacity(4);


//                 let mut index = 0;

//                 if points[0].w >= ISO_LEVEL {index |= 1}
//                 if points[1].w >= ISO_LEVEL {index |= 2}
//                 if points[2].w >= ISO_LEVEL {index |= 4}
//                 if points[3].w >= ISO_LEVEL {index |= 8}
//                 if points[4].w >= ISO_LEVEL {index |= 16}
//                 if points[5].w >= ISO_LEVEL {index |= 32}
//                 if points[6].w >= ISO_LEVEL {index |= 64}
//                 if points[7].w >= ISO_LEVEL {index |= 128}

//                 if index == 0 { continue; }

//                 for i in (0..15).step_by(3) {

//                     if TRI_TABLE[index][i] == -1 {
//                         break;
//                     }

//                     let a0 = CORNER_INDEX_AFROM_EDGE[TRI_TABLE[index][i] as usize];
//                     let b0 = CORNER_INDEX_BFROM_EDGE[TRI_TABLE[index][i] as usize];

//                     let a1 = CORNER_INDEX_AFROM_EDGE[TRI_TABLE[index][i+1] as usize];
//                     let b1 = CORNER_INDEX_BFROM_EDGE[TRI_TABLE[index][i+1] as usize];

//                     let a2 = CORNER_INDEX_AFROM_EDGE[TRI_TABLE[index][i+2] as usize];
//                     let b2 = CORNER_INDEX_BFROM_EDGE[TRI_TABLE[index][i+2] as usize];
//                     let triangle = Triangle{
//                         a: interpolate_verts(points[a0 as usize], points[b0 as usize]),
//                         b: interpolate_verts(points[a1 as usize], points[b1 as usize]),
//                         c: interpolate_verts(points[a2 as usize], points[b2 as usize]),
//                     };
//                     triangles.push(triangle);

//                     // vertices.push(interpolate_verts(points[a0 as usize], points[b0 as usize]).into());
//                     // vertices.push(interpolate_verts(points[a1 as usize], points[b1 as usize]).into());
//                     // vertices.push(interpolate_verts(points[a2 as usize], points[b2 as usize]).into());
//                 }

//                 triangles.iter().for_each(|x| {
//                     vertices.append(&mut vec![
//                         x.a.to_array(),
//                         x.b.to_array(),
//                         x.c.to_array()
//                     ])
//                 });
//         }
//         let length = vertices.len() as u32;
//         let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
//         let indices = (0..length).collect::<Vec<u32>>();
//         let uvs: Vec<[f32; 2]> = vec![[0.0, 0.0]; length as usize];


//         mesh.set_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
//         mesh.compute_flat_normals();
//         mesh.set_indices(Some(Indices::U32(indices)));
//         mesh.set_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        

        
//         *meshes.get_mut(mesh_handle.id).unwrap() = mesh;
//         chunk.dirty = false;
//     }
// }

fn set_points_system(
    mut query: Query<(&mut Chunk, &Transform)>,
    key: Res<Input<KeyCode>>,
) {
    if !key.just_pressed(KeyCode::G){
        return;
    }
    let simplex = OpenSimplexNoise::new(Some(69420));
    for (mut chunk, transform) in query.iter_mut() {

        for i in 0..BUFFER_SIZE-1 {
            chunk.points[i] = calc_iso(transform.translation + from_index(i).as_vec3(), &simplex);
        }
        chunk.dirty = true;   
    }

    println!("density calculated")
}

fn calc_iso(ws: Vec3, simplex: &OpenSimplexNoise) -> f32{
    let mut density = -ws.y;

    let mut freq = 0.005;
    let mut amplitude = 15.0;
    for _ in 0..=8 {
        density += (simplex.eval_3d(ws.x as f64 * freq, ws.y as f64 * freq, ws.z as f64 * freq) as f32 + 1.0) * amplitude;
        freq *= 2.0;
        amplitude *= 0.5;
    }
    density
}