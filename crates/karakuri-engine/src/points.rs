//! Standalone point cloud rendering pipeline.

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use crate::camera::Orbit;
use crate::video_source::VideoSource;

/// Uniform buffer layout matching `shaders/points.wgsl`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct Uniforms {
    view_proj: [[f32; 4]; 4],
    viewport: [f32; 2],
    t: f32,
    dt: f32,
    capacity: u32,
    seed_salt: u32,
    point_scale: f32,
    hue: f32,
    exposure: f32,
    falloff: f32,
    _pad: [f32; 2],
}

/// Configurable render parameters for the points pipeline.
#[derive(Debug, Clone, Copy)]
pub struct Params {
    pub point_scale: f32,
    pub hue: f32,
    pub exposure: f32,
    pub falloff: f32,
}

impl Default for Params {
    fn default() -> Params {
        Params {
            point_scale: 6.0,
            hue: 0.6,
            exposure: 1.4,
            falloff: 2.0,
        }
    }
}

pub struct Points {
    pipeline: wgpu::RenderPipeline,
    uniforms: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    capacity: u32,
    viewport: [f32; 2],
    seed_salt: u32,
    /// Simulation time in seconds.
    t: f32,
    dt: f32,
    pub params: Params,
    pub camera: Orbit,
}

impl Points {
    /// Six vertices per quad element (expanding point primitives into billboard quads).
    const VERTICES_PER_ELEMENT: u32 = 6;

    pub fn new(device: &wgpu::Device, capacity: u32, seed_salt: u32) -> Points {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("points"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/points.wgsl").into()),
        });

        let uniforms = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("points uniforms"),
            contents: bytemuck::bytes_of(&Uniforms::zeroed()),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("points"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("points"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniforms.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("points"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("points"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba16Float,
                    // Additive blend state accumulating RGB luminance and alpha coverage.
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        Points {
            pipeline,
            uniforms,
            bind_group,
            capacity,
            viewport: [1.0, 1.0],
            seed_salt,
            t: 0.0,
            dt: 1.0 / 60.0,
            params: Params::default(),
            camera: Orbit::default(),
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.viewport = [width.max(1) as f32, height.max(1) as f32];
    }

    /// Returns current simulation time in seconds.
    pub fn time(&self) -> f32 {
        self.t
    }

    fn uniforms(&self) -> Uniforms {
        let aspect = self.viewport[0] / self.viewport[1];
        Uniforms {
            view_proj: self.camera.view_proj(self.t, aspect),
            viewport: self.viewport,
            t: self.t,
            dt: self.dt,
            capacity: self.capacity,
            seed_salt: self.seed_salt,
            point_scale: self.params.point_scale,
            hue: self.params.hue,
            exposure: self.params.exposure,
            falloff: self.params.falloff,
            _pad: [0.0; 2],
        }
    }

    /// Updates simulation time and writes uniform buffer data.
    pub fn prepare(&mut self, queue: &wgpu::Queue, steps: u8) {
        self.t += self.dt * f32::from(steps);
        queue.write_buffer(&self.uniforms, 0, bytemuck::bytes_of(&self.uniforms()));
    }
}

impl VideoSource for Points {
    fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        _steps: u8,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("points"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    // Clear to transparent black to start coverage accumulation at zero.
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.draw(0..Self::VERTICES_PER_ELEMENT, 0..self.capacity);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uniforms_are_sixteen_byte_aligned() {
        // Verifies uniform buffer size is aligned to 16 bytes for WebGPU compliance.
        assert_eq!(std::mem::size_of::<Uniforms>() % 16, 0);
        assert_eq!(std::mem::size_of::<Uniforms>(), 112);
    }
}
