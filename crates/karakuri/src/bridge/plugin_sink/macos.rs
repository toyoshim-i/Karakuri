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
                other => return Err(format!("unsupported format for output plugin: {other:?}")),
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
