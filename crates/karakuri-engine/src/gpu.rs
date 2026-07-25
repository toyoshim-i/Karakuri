//! Device acquisition.
//!
//! Kept separate from any window so that the whole render path can be exercised
//! headlessly: an offscreen `Rgba16Float` target and a readback prove far more
//! than a screenshot does, and they run in a test.

#[derive(Debug, thiserror::Error)]
pub enum GpuError {
    #[error("no suitable adapter: {0}")]
    Adapter(#[from] wgpu::RequestAdapterError),
    #[error("device request failed: {0}")]
    Device(#[from] wgpu::RequestDeviceError),
}

pub struct Gpu {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    /// Whether `TIMESTAMP_QUERY` was available. Any change that touches
    /// performance is supposed to come with a GPU-timestamp measurement, so it
    /// is requested whenever the adapter offers it.
    pub timestamps: bool,
}

impl Gpu {
    pub async fn new(compatible_surface: Option<&wgpu::Surface<'_>>) -> Result<Gpu, GpuError> {
        Gpu::from_instance(Gpu::instance(), compatible_surface).await
    }

    /// The instance has to exist before the surface, and the surface before the
    /// adapter that must be compatible with it, so a windowed caller needs the
    /// three steps apart.
    pub fn instance() -> wgpu::Instance {
        wgpu::Instance::new(&wgpu::InstanceDescriptor::default())
    }

    pub async fn from_instance(
        instance: wgpu::Instance,
        compatible_surface: Option<&wgpu::Surface<'_>>,
    ) -> Result<Gpu, GpuError> {
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface,
            })
            .await?;

        let timestamps = adapter
            .features()
            .contains(wgpu::Features::TIMESTAMP_QUERY);
        let mut required_features = wgpu::Features::empty();
        if timestamps {
            required_features |= wgpu::Features::TIMESTAMP_QUERY;
        }

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("karakuri"),
                required_features,
                required_limits: adapter.limits(),
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::Off,
            })
            .await?;

        Ok(Gpu {
            instance,
            adapter,
            device,
            queue,
            timestamps,
        })
    }

    /// Blocking acquisition with no surface, for tests and offscreen probes.
    pub fn headless() -> Result<Gpu, GpuError> {
        pollster::block_on(Gpu::new(None))
    }
}
