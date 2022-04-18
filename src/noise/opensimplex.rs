use std::iter::once;

use bevy::{
    prelude::*,
    render::{
        render_resource::*, 
        renderer::{RenderDevice, RenderQueue}
    },
    core::{cast_slice, Pod}, utils::Instant,
};

use crate::world::chunk::BUFFER_SIZE;


struct SimplexCumputeBuffers {
    pos_buffer: Buffer,
    values_buffer: Buffer,
}

impl SimplexCumputeBuffers {
    fn new_empty(render_device: &RenderDevice, buffer_size: u64) -> Self{
        let pos_buffer = render_device.create_buffer(&BufferDescriptor {
            label: Some("simplex pos buffer"),
            size: std::mem::size_of::<Vec4>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let values_buffer = render_device.create_buffer(&BufferDescriptor {
            label: Some("simplex values buffer"),
            size: std::mem::size_of::<f32>() as u64 * buffer_size,
            usage: BufferUsages::STORAGE | BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        
        Self {pos_buffer, values_buffer}
    }
}

pub struct  OpenSimplex {
    buffer_bind_group_layout: BindGroupLayout,
    simplex_pipeline: ComputePipeline,
    compute_buffers: SimplexCumputeBuffers,
}

impl OpenSimplex {
    pub fn compute_chunk(
        &self,
        pos: Vec3,
        render_device: &RenderDevice,
        render_queue: &RenderQueue,
    ) -> [f32; BUFFER_SIZE]{

        let start = Instant::now();

        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &self.buffer_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: self.compute_buffers.pos_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: self.compute_buffers.values_buffer.as_entire_binding()
                },
            ],
        });

        let pos = &[Vec4::new(pos.x, pos.y, pos.z, 0.0)];
        let bytes: &[u8] = cast_slice(pos);
        render_queue.write_buffer(&self.compute_buffers.pos_buffer, 0, &bytes[..]);

        let mut command_encoder = render_device.create_command_encoder(&CommandEncoderDescriptor { label: Some("simplex command encoder") });
        {
            let mut pass = command_encoder.begin_compute_pass(&ComputePassDescriptor::default());
            pass.set_pipeline(&self.simplex_pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch(4, 4, 4)
        }
        render_queue.submit(once(command_encoder.finish()));
        
        let mut values: [f32; BUFFER_SIZE] = [0.0; BUFFER_SIZE];
    
        {
            let slice = &self.compute_buffers.values_buffer.slice(..);
            render_device.map_buffer(slice, MapMode::Read);
            let buff_out = &slice.get_mapped_range()[..];
            let buff_out: &[f32] = cast_slice(buff_out);
            
            for i in 0..BUFFER_SIZE {
                values[i] = buff_out[i];
            }
            let elapsed = start.elapsed();
        }
        self.compute_buffers.values_buffer.unmap();
        

        values
    }
}

impl FromWorld for OpenSimplex {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.get_resource::<RenderDevice>().unwrap();
        
       
        let shader_source = include_str!("../../assets/shaders/noise.wgsl");
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
                            ty: BufferBindingType::Uniform, 
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
                    }
                ]
            });

        let compute_buffers = SimplexCumputeBuffers::new_empty(render_device, BUFFER_SIZE as u64);

        let pipeline_layout = render_device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&buffer_bind_group_layout],
            push_constant_ranges: &[],
        });
        
        let simplex_pipeline = render_device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
        });

        Self {
            buffer_bind_group_layout,
            simplex_pipeline,
            compute_buffers,
        }
    }
}