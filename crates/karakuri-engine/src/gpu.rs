//! Device acquisition and initialization.
//!
//! Supports both windowed presentation and headless rendering.

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
    /// Indicates whether GPU timestamp queries are supported by the adapter.
    pub timestamps: bool,
}

impl Gpu {
    pub async fn new(compatible_surface: Option<&wgpu::Surface<'_>>) -> Result<Gpu, GpuError> {
        Gpu::from_instance(Gpu::instance(), compatible_surface).await
    }

    /// Creates a new `wgpu::Instance` from environment variables.
    pub fn instance() -> wgpu::Instance {
        // Uses `new_without_display_handle_from_env` to respect `WGPU_BACKEND`.
        wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env())
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
                // Disable limit bucketing to preserve the device's native limits.
                apply_limit_buckets: false,
            })
            .await?;

        // Query timestamp support: TIMESTAMP_QUERY permits pass-level timestamps,
        // while TIMESTAMP_QUERY_INSIDE_ENCODERS permits encoder-level timestamps.
        let available = adapter.features();
        let timestamps = available.contains(wgpu::Features::TIMESTAMP_QUERY);
        let mut required_features = wgpu::Features::empty();
        if timestamps {
            required_features |= wgpu::Features::TIMESTAMP_QUERY;
        }
        if available.contains(wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS) {
            required_features |= wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS;
        }

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("karakuri"),
                required_features,
                required_limits: adapter.limits(),
                memory_hints: wgpu::MemoryHints::Performance,
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
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

    /// Acquires a GPU device synchronously without a surface for tests and headless execution.
    pub fn headless() -> Result<Gpu, GpuError> {
        pollster::block_on(Gpu::new(None))
    }
}
