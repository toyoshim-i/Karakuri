//! Video output sink streaming zero-copy GPU frames to an out-of-process plugin.
//! On macOS, frames render to an `IOSurface`-backed Metal texture forwarded via pipe (ADR-0358, docs/plugins.md).

use karakuri_engine::{Gpu, Sink, Skip};
use karakuri_environment::output_plugin::OutputPlugin;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

/// An output plugin sink feeding composited frames to an out-of-process helper.
pub(crate) struct PluginSink {
    plugin: OutputPlugin,
    #[cfg(target_os = "macos")]
    surface: macos::MacOsSurface,
    #[cfg(target_os = "macos")]
    #[allow(dead_code)]
    render_target: wgpu::Texture,
    #[cfg(target_os = "macos")]
    render_target_view: wgpu::TextureView,
    #[cfg(target_os = "macos")]
    flip_pipeline: wgpu::RenderPipeline,
    #[cfg(target_os = "macos")]
    flip_bind_group: wgpu::BindGroup,
    #[cfg(target_os = "windows")]
    surface: windows::WindowsSurface,
    #[cfg(target_os = "windows")]
    child_surface_id: u64,
    #[allow(dead_code)]
    frame_index: u64,
    width: u32,
    height: u32,
}

impl PluginSink {
    /// Opens the specified plugin command and sets up zero-copy GPU surface sharing.
    #[allow(dead_code)]
    pub(crate) fn open(
        gpu: &Gpu,
        command: &str,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
    ) -> Result<Self, String> {
        #[cfg(target_os = "macos")]
        {
            let surface = macos::MacOsSurface::new(gpu, width, height, format)?;
            let plugin = OutputPlugin::open(command, "iosurface", width, height, "bgra8unorm")
                .map_err(|e| format!("{e}"))?;

            let shader = gpu
                .device
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("plugin sink flip shader"),
                    source: wgpu::ShaderSource::Wgsl(
                        r#"
@group(0) @binding(0) var src_tex: texture_2d<f32>;
@group(0) @binding(1) var src_sampler: sampler;

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs(@builtin(vertex_index) i: u32) -> VsOut {
    let uv = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    var out: VsOut;
    out.clip = vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
    // Invert vertical texture coordinate so Syphon clients (which follow OpenGL bottom-up
    // conventions) display the frame right-side up.
    out.uv = vec2<f32>(uv.x, uv.y);
    return out;
}

@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> {
    return textureSample(src_tex, src_sampler, in.uv);
}
"#
                        .into(),
                    ),
                });

            let bind_group_layout =
                gpu.device
                    .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                        label: Some("plugin sink flip bind group layout"),
                        entries: &[
                            wgpu::BindGroupLayoutEntry {
                                binding: 0,
                                visibility: wgpu::ShaderStages::FRAGMENT,
                                ty: wgpu::BindingType::Texture {
                                    sample_type: wgpu::TextureSampleType::Float {
                                        filterable: true,
                                    },
                                    view_dimension: wgpu::TextureViewDimension::D2,
                                    multisampled: false,
                                },
                                count: None,
                            },
                            wgpu::BindGroupLayoutEntry {
                                binding: 1,
                                visibility: wgpu::ShaderStages::FRAGMENT,
                                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                                count: None,
                            },
                        ],
                    });

            let pipeline_layout =
                gpu.device
                    .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: Some("plugin sink flip pipeline layout"),
                        bind_group_layouts: &[Some(&bind_group_layout)],
                        immediate_size: 0,
                    });

            let flip_pipeline =
                gpu.device
                    .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                        label: Some("plugin sink flip pipeline"),
                        layout: Some(&pipeline_layout),
                        vertex: wgpu::VertexState {
                            module: &shader,
                            entry_point: Some("vs"),
                            compilation_options: wgpu::PipelineCompilationOptions::default(),
                            buffers: &[],
                        },
                        fragment: Some(wgpu::FragmentState {
                            module: &shader,
                            entry_point: Some("fs"),
                            compilation_options: wgpu::PipelineCompilationOptions::default(),
                            targets: &[Some(wgpu::ColorTargetState {
                                format,
                                blend: None,
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

            let sampler = gpu.device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("plugin sink flip sampler"),
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            });

            let render_target = gpu.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("plugin sink intermediate render target"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            let render_target_view =
                render_target.create_view(&wgpu::TextureViewDescriptor::default());

            let flip_bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("plugin sink flip bind group"),
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&render_target_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                ],
            });

            Ok(Self {
                plugin,
                surface,
                render_target,
                render_target_view,
                flip_pipeline,
                flip_bind_group,
                frame_index: 0,
                width,
                height,
            })
        }
        #[cfg(target_os = "windows")]
        {
            let surface = windows::WindowsSurface::new(gpu, width, height, format)?;
            // The plugin copies from this surface into a texture of the format
            // named here, and a copy between formats is a silent no-op.
            let format_name = match format.remove_srgb_suffix() {
                wgpu::TextureFormat::Rgba8Unorm => "rgba8unorm",
                _ => "bgra8unorm",
            };
            let plugin = OutputPlugin::open(command, "dxgi", width, height, format_name)
                .map_err(|e| format!("{e}"))?;

            let child_surface_id = unsafe {
                use ::windows::Win32::Foundation::{
                    DuplicateHandle, DUPLICATE_SAME_ACCESS, HANDLE,
                };
                use ::windows::Win32::System::Threading::GetCurrentProcess;

                let child_proc = HANDLE(plugin.child_raw_handle());
                let mut target_handle = HANDLE::default();
                DuplicateHandle(
                    GetCurrentProcess(),
                    surface.handle,
                    child_proc,
                    &mut target_handle,
                    0,
                    false,
                    DUPLICATE_SAME_ACCESS,
                )
                .map_err(|e| format!("DuplicateHandle to child plugin failed: {e}"))?;
                target_handle.0 as usize as u64
            };

            Ok(Self {
                plugin,
                surface,
                child_surface_id,
                frame_index: 0,
                width,
                height,
            })
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            let _ = (gpu, command, width, height, format);
            Err(
                "output plugin sinks currently require macOS (IOSurface) or Windows (DXGI)"
                    .to_string(),
            )
        }
    }

    /// Access underlying plugin telemetry.
    #[allow(dead_code)]
    pub(crate) fn telemetry(&self) -> karakuri_environment::output_plugin::PluginTelemetry {
        self.plugin.telemetry()
    }

    /// Server name published by the plugin.
    #[allow(dead_code)]
    pub(crate) fn server_name(&self) -> &str {
        self.plugin.server_name()
    }

    /// Returns whether the plugin process is alive.
    #[allow(dead_code)]
    pub(crate) fn is_alive(&self) -> bool {
        self.plugin.is_alive()
    }
}

impl Sink for PluginSink {
    fn acquire(&mut self, _gpu: &Gpu) -> Result<(), Skip> {
        if !self.plugin.is_alive() {
            return Err(Skip::Fault("output plugin process is not running".into()));
        }
        Ok(())
    }

    fn view(&self) -> &wgpu::TextureView {
        #[cfg(target_os = "macos")]
        {
            &self.render_target_view
        }
        #[cfg(target_os = "windows")]
        {
            &self.surface.view
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            unreachable!("unsupported plugin sink platform")
        }
    }

    fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn after_draw(&mut self, encoder: &mut wgpu::CommandEncoder) {
        #[cfg(target_os = "macos")]
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("output plugin flip pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.surface.view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.flip_pipeline);
            pass.set_bind_group(0, &self.flip_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = encoder;
        }
    }

    fn present(&mut self, _gpu: &Gpu) -> Result<(), String> {
        #[cfg(target_os = "macos")]
        {
            self.plugin.send_frame(
                self.frame_index,
                u64::from(self.surface.surface_id),
                self.width,
                self.height,
            );
            self.frame_index += 1;
        }
        #[cfg(target_os = "windows")]
        {
            let _ = _gpu.device.poll(wgpu::PollType::wait_indefinitely());
            self.plugin.send_frame(
                self.frame_index,
                self.child_surface_id,
                self.width,
                self.height,
            );
            self.frame_index += 1;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
