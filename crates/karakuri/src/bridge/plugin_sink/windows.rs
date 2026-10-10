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
    #[allow(dead_code)]
    pub surface_id: u64,
    #[allow(dead_code)]
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    #[allow(dead_code)]
    pub width: u32,
    #[allow(dead_code)]
    pub height: u32,
    _d3d11_device: Option<ID3D11Device>,
    _d3d11_texture: Option<ID3D11Texture2D>,
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
            other => return Err(format!("unsupported format for output plugin: {other:?}")),
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
                let factory: IDXGIFactory1 =
                    CreateDXGIFactory1().map_err(|e| format!("CreateDXGIFactory1 failed: {e}"))?;
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
                    Some(a) => (
                        Some(a),
                        windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_UNKNOWN,
                    ),
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
                    MiscFlags: (D3D11_RESOURCE_MISC_SHARED.0
                        | D3D11_RESOURCE_MISC_SHARED_NTHANDLE.0)
                        as u32,
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
                    .CreateSharedHandle(None, GENERIC_ALL.0, None)
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
                    _d3d11_texture: Some(d3d11_texture),
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
                    _d3d11_texture: None,
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
