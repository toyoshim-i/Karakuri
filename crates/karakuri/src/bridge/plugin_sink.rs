//! Video output sink streaming zero-copy GPU frames to an out-of-process plugin.
//!
//! On macOS, frames are rendered into an `IOSurface`-backed Metal texture, and the
//! resulting `IOSurfaceID` is forwarded to the plugin process (e.g. Syphon) over a
//! non-blocking pipe (ADR-0358, docs/plugins.md).

use karakuri_engine::{Gpu, Sink, Skip};
use karakuri_environment::output_plugin::OutputPlugin;

#[cfg(target_os = "macos")]
mod macos {
    use std::ffi::c_void;

    use core_foundation::base::TCFType;
    use core_foundation::boolean::CFBoolean;
    use core_foundation::dictionary::CFDictionary;
    use core_foundation::number::CFNumber;
    use core_foundation::string::CFString;
    use objc2::runtime::ProtocolObject;
    use objc2_io_surface::IOSurfaceRef;
    use objc2_metal::{
        MTLDevice, MTLPixelFormat, MTLStorageMode, MTLTextureDescriptor, MTLTextureType,
        MTLTextureUsage,
    };

    use super::Gpu;

    #[link(name = "IOSurface", kind = "framework")]
    extern "C" {
        fn IOSurfaceCreate(
            properties: core_foundation::dictionary::CFDictionaryRef,
        ) -> *mut IOSurfaceRef;
        fn IOSurfaceGetID(buffer: *const IOSurfaceRef) -> u32;
        #[allow(dead_code)]
        pub(crate) fn IOSurfaceLookup(csid: u32) -> *mut IOSurfaceRef;
        pub(crate) fn CFRelease(cf: *const c_void);
    }

    pub struct MacOsSurface {
        surface_ref: *mut IOSurfaceRef,
        pub surface_id: u32,
        #[allow(dead_code)]
        pub texture: wgpu::Texture,
        pub view: wgpu::TextureView,
        #[allow(dead_code)]
        pub width: u32,
        #[allow(dead_code)]
        pub height: u32,
    }

    impl MacOsSurface {
        pub fn new(
            gpu: &Gpu,
            width: u32,
            height: u32,
            format: wgpu::TextureFormat,
        ) -> Result<Self, String> {
            unsafe {
                let mtl_format = match format.remove_srgb_suffix() {
                    wgpu::TextureFormat::Bgra8Unorm => {
                        if format.is_srgb() {
                            MTLPixelFormat::BGRA8Unorm_sRGB
                        } else {
                            MTLPixelFormat::BGRA8Unorm
                        }
                    }
                    wgpu::TextureFormat::Rgba8Unorm => {
                        if format.is_srgb() {
                            MTLPixelFormat::RGBA8Unorm_sRGB
                        } else {
                            MTLPixelFormat::RGBA8Unorm
                        }
                    }
                    other => {
                        return Err(format!("unsupported format for output plugin: {other:?}"))
                    }
                };

                let k_width = CFString::new("IOSurfaceWidth");
                let k_height = CFString::new("IOSurfaceHeight");
                let k_bytes_per_elem = CFString::new("IOSurfaceBytesPerElement");
                let k_bytes_per_row = CFString::new("IOSurfaceBytesPerRow");
                let k_alloc_size = CFString::new("IOSurfaceAllocSize");
                let k_pixel_format = CFString::new("IOSurfacePixelFormat");
                let k_is_global = CFString::new("IOSurfaceIsGlobal");

                let v_width = CFNumber::from(width as i32);
                let v_height = CFNumber::from(height as i32);
                let v_bytes_per_elem = CFNumber::from(4i32);
                let bytes_per_row = (width * 4).div_ceil(64) * 64;
                let v_bytes_per_row = CFNumber::from(bytes_per_row as i32);
                let alloc_size = bytes_per_row * height;
                let v_alloc_size = CFNumber::from(alloc_size as i32);
                // 'BGRA' = 0x42475241
                let v_pixel_format = CFNumber::from(0x42475241i32);
                let v_is_global = CFBoolean::true_value();

                let pairs = [
                    (k_width.as_CFType(), v_width.as_CFType()),
                    (k_height.as_CFType(), v_height.as_CFType()),
                    (k_bytes_per_elem.as_CFType(), v_bytes_per_elem.as_CFType()),
                    (k_bytes_per_row.as_CFType(), v_bytes_per_row.as_CFType()),
                    (k_alloc_size.as_CFType(), v_alloc_size.as_CFType()),
                    (k_pixel_format.as_CFType(), v_pixel_format.as_CFType()),
                    (k_is_global.as_CFType(), v_is_global.as_CFType()),
                ];
                let dict = CFDictionary::from_CFType_pairs(&pairs);
                let surface_ref = IOSurfaceCreate(dict.as_concrete_TypeRef());
                if surface_ref.is_null() {
                    return Err("IOSurfaceCreate returned null".to_string());
                }

                let surface_id = IOSurfaceGetID(surface_ref);

                // Access Metal device from wgpu-hal
                let hal_device = gpu
                    .device
                    .as_hal::<wgpu_hal::api::Metal>()
                    .ok_or_else(|| "wgpu device does not have Metal HAL backend".to_string())?;
                let metal_device: &ProtocolObject<dyn MTLDevice> = hal_device.raw_device();

                let desc =
                    MTLTextureDescriptor::texture2DDescriptorWithPixelFormat_width_height_mipmapped(
                        mtl_format,
                        width as usize,
                        height as usize,
                        false,
                    );
                desc.setUsage(MTLTextureUsage::RenderTarget | MTLTextureUsage::ShaderRead);
                desc.setStorageMode(MTLStorageMode::Shared);

                let raw_texture =
                    metal_device.newTextureWithDescriptor_iosurface_plane(&desc, &*surface_ref, 0);
                let raw_texture = match raw_texture {
                    Some(t) => t,
                    None => {
                        CFRelease(surface_ref as *const c_void);
                        return Err("Metal failed to wrap IOSurface in MTLTexture".to_string());
                    }
                };

                let hal_texture = wgpu_hal::metal::Device::texture_from_raw(
                    raw_texture,
                    format,
                    MTLTextureType::Type2D,
                    1,
                    1,
                    wgpu_hal::CopyExtent {
                        width,
                        height,
                        depth: 1,
                    },
                    None,
                );

                let texture_desc = wgpu::TextureDescriptor {
                    label: Some("output plugin IOSurface texture"),
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
                };

                let texture = gpu.device.create_texture_from_hal::<wgpu_hal::api::Metal>(
                    hal_texture,
                    &texture_desc,
                    wgpu::TextureUses::UNINITIALIZED,
                );
                let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

                Ok(Self {
                    surface_ref,
                    surface_id,
                    texture,
                    view,
                    width,
                    height,
                })
            }
        }
    }

    impl Drop for MacOsSurface {
        fn drop(&mut self) {
            unsafe {
                if !self.surface_ref.is_null() {
                    CFRelease(self.surface_ref as *const c_void);
                }
            }
        }
    }
}

#[cfg(target_os = "windows")]
mod windows {
    use super::Gpu;
    use windows::core::Interface;
    use windows::Win32::Foundation::{CloseHandle, GENERIC_ALL, HANDLE, HMODULE};
    use windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_HARDWARE;
    use windows::Win32::Graphics::Direct3D11::*;
    use windows::Win32::Graphics::Direct3D12::*;
    use windows::Win32::Graphics::Dxgi::Common::*;
    use windows::Win32::Graphics::Dxgi::{
        CreateDXGIFactory1, IDXGIAdapter, IDXGIFactory1, IDXGIResource1,
    };

    pub struct WindowsSurface {
        pub handle: HANDLE,
        pub surface_id: u64,
        #[allow(dead_code)]
        pub texture: wgpu::Texture,
        pub view: wgpu::TextureView,
        #[allow(dead_code)]
        pub width: u32,
        #[allow(dead_code)]
        pub height: u32,
        _d3d11_device: Option<ID3D11Device>,
    }

    impl WindowsSurface {
        pub fn new(
            gpu: &Gpu,
            width: u32,
            height: u32,
            format: wgpu::TextureFormat,
        ) -> Result<Self, String> {
            let dxgi_format = match format.remove_srgb_suffix() {
                wgpu::TextureFormat::Bgra8Unorm => DXGI_FORMAT_B8G8R8A8_UNORM,
                wgpu::TextureFormat::Rgba8Unorm => DXGI_FORMAT_R8G8B8A8_UNORM,
                other => {
                    return Err(format!("unsupported format for output plugin: {other:?}"))
                }
            };

            let size = wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            };

            // 1. Try Vulkan backend via D3D11 shared handle interop
            unsafe {
                if let Some(hal_vulkan) = gpu.device.as_hal::<wgpu_hal::api::Vulkan>() {
                    let adapter_name = gpu.adapter.get_info().name;
                    let factory: IDXGIFactory1 = CreateDXGIFactory1()
                        .map_err(|e| format!("CreateDXGIFactory1 failed: {e}"))?;
                    let mut matching_adapter: Option<IDXGIAdapter> = None;
                    let mut i = 0;
                    while let Ok(adapter1) = factory.EnumAdapters1(i) {
                        if let Ok(desc) = adapter1.GetDesc1() {
                            let len = desc
                                .Description
                                .iter()
                                .position(|&c| c == 0)
                                .unwrap_or(desc.Description.len());
                            let desc_name = String::from_utf16_lossy(&desc.Description[..len]);
                            if desc_name.contains(&adapter_name) || adapter_name.contains(&desc_name) {
                                if let Ok(adapter) = adapter1.cast::<IDXGIAdapter>() {
                                    matching_adapter = Some(adapter);
                                    break;
                                }
                            }
                        }
                        i += 1;
                    }

                    let (p_adapter, driver_type) = match matching_adapter.as_ref() {
                        Some(a) => (Some(a), windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_UNKNOWN),
                        None => (None, D3D_DRIVER_TYPE_HARDWARE),
                    };

                    let mut d3d11_device: Option<ID3D11Device> = None;
                    let mut d3d11_context: Option<ID3D11DeviceContext> = None;
                    D3D11CreateDevice(
                        p_adapter,
                        driver_type,
                        HMODULE::default(),
                        D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                        None,
                        D3D11_SDK_VERSION,
                        Some(&mut d3d11_device),
                        None,
                        Some(&mut d3d11_context),
                    )
                    .map_err(|e| format!("D3D11CreateDevice failed: {e}"))?;

                    let d3d11_device = d3d11_device
                        .ok_or_else(|| "D3D11CreateDevice returned null device".to_string())?;

                    let tex_desc = D3D11_TEXTURE2D_DESC {
                        Width: width,
                        Height: height,
                        MipLevels: 1,
                        ArraySize: 1,
                        Format: dxgi_format,
                        SampleDesc: DXGI_SAMPLE_DESC {
                            Count: 1,
                            Quality: 0,
                        },
                        Usage: D3D11_USAGE_DEFAULT,
                        BindFlags: (D3D11_BIND_RENDER_TARGET.0 | D3D11_BIND_SHADER_RESOURCE.0) as u32,
                        CPUAccessFlags: 0,
                        MiscFlags: (D3D11_RESOURCE_MISC_SHARED.0 | D3D11_RESOURCE_MISC_SHARED_NTHANDLE.0) as u32,
                    };

                    let mut d3d11_texture: Option<ID3D11Texture2D> = None;
                    d3d11_device
                        .CreateTexture2D(&tex_desc, None, Some(&mut d3d11_texture))
                        .map_err(|e| format!("D3D11 CreateTexture2D failed: {e}"))?;

                    let d3d11_texture = d3d11_texture
                        .ok_or_else(|| "CreateTexture2D returned null texture".to_string())?;

                    let dxgi_resource1: IDXGIResource1 = d3d11_texture
                        .cast()
                        .map_err(|e| format!("cast to IDXGIResource1 failed: {e}"))?;

                    let handle = dxgi_resource1
                        .CreateSharedHandle(
                            None,
                            GENERIC_ALL.0,
                            None,
                        )
                        .map_err(|e| format!("CreateSharedHandle failed: {e}"))?;

                    let hal_desc = wgpu_hal::TextureDescriptor {
                        label: Some("output plugin DXGI shared texture (Vulkan D3D11)"),
                        size,
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format,
                        usage: wgpu::TextureUses::COLOR_TARGET | wgpu::TextureUses::RESOURCE,
                        memory_flags: wgpu_hal::MemoryFlags::empty(),
                        view_formats: vec![],
                    };

                    let hal_texture = hal_vulkan
                        .texture_from_d3d11_shared_handle(handle, &hal_desc)
                        .map_err(|e| format!("Vulkan failed to import D3D11 shared handle: {e:?}"))?;

                    let texture_desc = wgpu::TextureDescriptor {
                        label: Some("output plugin DXGI shared texture"),
                        size,
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format,
                        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                            | wgpu::TextureUsages::TEXTURE_BINDING,
                        view_formats: &[],
                    };

                    let texture = gpu.device.create_texture_from_hal::<wgpu_hal::api::Vulkan>(
                        hal_texture,
                        &texture_desc,
                        wgpu::TextureUses::UNINITIALIZED,
                    );
                    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

                    return Ok(Self {
                        handle,
                        surface_id: handle.0 as usize as u64,
                        texture,
                        view,
                        width,
                        height,
                        _d3d11_device: Some(d3d11_device),
                    });
                }

                // 2. Try DirectX 12 backend
                if let Some(hal_dx12) = gpu.device.as_hal::<wgpu_hal::api::Dx12>() {
                    let d3d12_device = hal_dx12.raw_device();

                    let heap_properties = D3D12_HEAP_PROPERTIES {
                        Type: D3D12_HEAP_TYPE_DEFAULT,
                        CPUPageProperty: D3D12_CPU_PAGE_PROPERTY_UNKNOWN,
                        MemoryPoolPreference: D3D12_MEMORY_POOL_UNKNOWN,
                        CreationNodeMask: 0,
                        VisibleNodeMask: 0,
                    };

                    let resource_desc = D3D12_RESOURCE_DESC {
                        Dimension: D3D12_RESOURCE_DIMENSION_TEXTURE2D,
                        Alignment: 0,
                        Width: width as u64,
                        Height: height,
                        DepthOrArraySize: 1,
                        MipLevels: 1,
                        Format: dxgi_format,
                        SampleDesc: DXGI_SAMPLE_DESC {
                            Count: 1,
                            Quality: 0,
                        },
                        Layout: D3D12_TEXTURE_LAYOUT_UNKNOWN,
                        Flags: D3D12_RESOURCE_FLAG_ALLOW_RENDER_TARGET
                            | D3D12_RESOURCE_FLAG_ALLOW_SIMULTANEOUS_ACCESS,
                    };

                    let mut raw_resource: Option<ID3D12Resource> = None;
                    d3d12_device
                        .CreateCommittedResource(
                            &heap_properties,
                            D3D12_HEAP_FLAG_SHARED,
                            &resource_desc,
                            D3D12_RESOURCE_STATE_COMMON,
                            None,
                            &mut raw_resource,
                        )
                        .map_err(|e| format!("CreateCommittedResource failed: {e}"))?;

                    let raw_resource = raw_resource
                        .ok_or_else(|| "CreateCommittedResource produced null resource".to_string())?;

                    let handle = d3d12_device
                        .CreateSharedHandle(&raw_resource, None, GENERIC_ALL.0, None)
                        .map_err(|e| format!("CreateSharedHandle failed: {e}"))?;

                    let surface_id = handle.0 as usize as u64;

                    let hal_texture = wgpu_hal::dx12::Device::texture_from_raw(
                        raw_resource,
                        format,
                        wgpu::TextureDimension::D2,
                        size,
                        1,
                        1,
                    );

                    let texture_desc = wgpu::TextureDescriptor {
                        label: Some("output plugin DXGI shared texture"),
                        size,
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format,
                        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                            | wgpu::TextureUsages::TEXTURE_BINDING,
                        view_formats: &[],
                    };

                    let texture = gpu.device.create_texture_from_hal::<wgpu_hal::api::Dx12>(
                        hal_texture,
                        &texture_desc,
                        wgpu::TextureUses::UNINITIALIZED,
                    );
                    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

                    return Ok(Self {
                        handle,
                        surface_id,
                        texture,
                        view,
                        width,
                        height,
                        _d3d11_device: None,
                    });
                }
            }

            Err(
                "current GPU device does not support DXGI shared handle surface (requires Vulkan with external memory or DirectX 12)"
                    .to_string(),
            )
        }
    }

    impl Drop for WindowsSurface {
        fn drop(&mut self) {
            unsafe {
                if self._d3d11_device.is_none() && !self.handle.is_invalid() {
                    let _ = CloseHandle(self.handle);
                }
            }
        }
    }
}

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
    frame_index: u64,
    width: u32,
    height: u32,
}

impl PluginSink {
    /// Opens the specified plugin command and sets up zero-copy GPU surface sharing.
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
            let plugin = OutputPlugin::open(command, "dxgi", width, height, "bgra8unorm")
                .map_err(|e| format!("{e}"))?;

            Ok(Self {
                plugin,
                surface,
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
            self.plugin.send_frame(
                self.frame_index,
                self.surface.surface_id,
                self.width,
                self.height,
            );
            self.frame_index += 1;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod gpu {
        use super::*;

        #[test]
        #[cfg(target_os = "macos")]
        fn macos_surface_creates_and_allocates_iosurface_id() {
            let gpu = Gpu::headless().expect("no GPU");
            let surface = macos::MacOsSurface::new(
                &gpu,
                640,
                480,
                wgpu::TextureFormat::Bgra8Unorm.add_srgb_suffix(),
            )
            .expect("MacOsSurface");
            assert!(surface.surface_id > 0, "IOSurfaceID must be non-zero");
            assert_eq!(surface.width, 640);
            assert_eq!(surface.height, 480);

            // Verify lookup by ID succeeds (cross-process lookup simulation)
            unsafe {
                let looked_up = macos::IOSurfaceLookup(surface.surface_id);
                assert!(
                    !looked_up.is_null(),
                    "IOSurfaceLookup must succeed for globally shared IOSurface"
                );
                macos::CFRelease(looked_up as *const std::ffi::c_void);
            }
        }

        #[test]
        #[cfg(target_os = "windows")]
        fn windows_surface_creates_and_allocates_dxgi_shared_handle() {
            let Ok(gpu) = Gpu::headless() else {
                eprintln!("headless GPU initialization failed; skipping");
                return;
            };
            println!("Running windows_surface test on backend: {:?}", gpu.adapter.get_info().backend);
            println!("Adapter name: {}", gpu.adapter.get_info().name);
            println!("Adapter has VULKAN_EXTERNAL_MEMORY_WIN32: {}", gpu.adapter.features().contains(wgpu::Features::VULKAN_EXTERNAL_MEMORY_WIN32));
            println!("Device has VULKAN_EXTERNAL_MEMORY_WIN32: {}", gpu.device.features().contains(wgpu::Features::VULKAN_EXTERNAL_MEMORY_WIN32));
            let surface = windows::WindowsSurface::new(
                &gpu,
                640,
                480,
                wgpu::TextureFormat::Bgra8Unorm.add_srgb_suffix(),
            )
            .expect("WindowsSurface");
            assert!(
                surface.surface_id > 0,
                "DXGI shared handle must be non-zero"
            );
            assert_eq!(surface.width, 640);
            assert_eq!(surface.height, 480);
        }

        #[test]
        #[cfg(target_os = "macos")]
        fn plugin_sink_integration_with_karakuri_syphon_binary() {
            let candidates = [
                "../Karakuri-syphon/target/debug/karakuri-syphon",
                "../../Karakuri-syphon/target/debug/karakuri-syphon",
                "../../../Karakuri-syphon/target/debug/karakuri-syphon",
                "/Users/toyoshim/Work/GitHub/Karakuri/Karakuri-syphon/target/debug/karakuri-syphon",
            ];
            let mut command = None;
            for c in candidates {
                let p = std::path::Path::new(c);
                if p.exists() {
                    command = Some(
                        p.canonicalize()
                            .unwrap_or_else(|_| p.to_path_buf())
                            .to_string_lossy()
                            .into_owned(),
                    );
                    break;
                }
            }
            let Some(command) = command else {
                return;
            };

            let gpu = Gpu::headless().expect("no GPU");
            let mut sink = PluginSink::open(
                &gpu,
                &command,
                1280,
                720,
                wgpu::TextureFormat::Bgra8Unorm.add_srgb_suffix(),
            )
            .expect("PluginSink::open");
            assert!(sink.is_alive());
            assert_eq!(sink.server_name(), "Karakuri");
            assert_eq!(sink.size(), (1280, 720));

            let present = karakuri_engine::Present::new(
                &gpu.device,
                wgpu::TextureFormat::Bgra8Unorm.add_srgb_suffix(),
                1280,
                720,
            );
            let mut encoder = gpu
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
            present.draw(&mut encoder, sink.view(), sink.size());
            sink.after_draw(&mut encoder);
            gpu.queue.submit([encoder.finish()]);

            sink.acquire(&gpu).expect("acquire");
            sink.present(&gpu).expect("present frame 0");

            let t = sink.telemetry();
            assert_eq!(t.host_dropped, 0);

            drop(sink);
        }

        #[test]
        #[cfg(target_os = "windows")]
        fn plugin_sink_integration_with_karakuri_spout_binary() {
            let candidates = [
                "../Karakuri-spout/target/debug/karakuri-spout.exe",
                "../../Karakuri-spout/target/debug/karakuri-spout.exe",
                "../../../Karakuri-spout/target/debug/karakuri-spout.exe",
            ];
            let mut command = None;
            for c in candidates {
                let p = std::path::Path::new(c);
                if p.exists() {
                    command = Some(
                        p.canonicalize()
                            .unwrap_or_else(|_| p.to_path_buf())
                            .to_string_lossy()
                            .into_owned(),
                    );
                    break;
                }
            }
            let Some(command) = command else {
                return;
            };

            std::env::remove_var("WGPU_BACKEND");
            let Ok(gpu) = Gpu::headless() else {
                return;
            };
            println!("Running Spout integration test with backend: {:?}", gpu.adapter.get_info().backend);

            let mut sink = PluginSink::open(
                &gpu,
                &command,
                1280,
                720,
                wgpu::TextureFormat::Bgra8Unorm.add_srgb_suffix(),
            )
            .expect("PluginSink::open");
            assert!(sink.is_alive());
            assert_eq!(sink.server_name(), "Karakuri");
            assert_eq!(sink.size(), (1280, 720));

            let present = karakuri_engine::Present::new(
                &gpu.device,
                wgpu::TextureFormat::Bgra8Unorm.add_srgb_suffix(),
                1280,
                720,
            );
            let mut encoder = gpu
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
            present.draw(&mut encoder, sink.view(), sink.size());
            sink.after_draw(&mut encoder);
            gpu.queue.submit([encoder.finish()]);

            sink.acquire(&gpu).expect("acquire");
            sink.present(&gpu).expect("present frame 0");

            let t = sink.telemetry();
            assert_eq!(t.host_dropped, 0);

            drop(sink);
        }
    }
}
