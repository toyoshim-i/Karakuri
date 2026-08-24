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
        // **`_from_env` is load-bearing**, and its absence is invisible.
        // `new_without_display_handle` — one word shorter, otherwise the same
        // call, same arguments, same type — ignores `WGPU_BACKEND` entirely.
        // With it, setting that variable does nothing and says nothing: the
        // run produces a plausible number on the default backend and the
        // operator writes down the one they asked for. Do not shorten this
        // back.
        //
        // Otherwise every field defaulted and no display handle. The handle is
        // the one field wgpu 30 added, and it is unused on Metal, Vulkan and
        // DX12 — only GLES needs it, and on Wayland it is required to present
        // at all. So a GL backend on Linux is where this constructor stops
        // being the right one, and the handle to pass there is winit's
        // `OwnedDisplayHandle`.
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
                // Bucketed limits exist to blunt fingerprinting where wgpu is
                // exposed to untrusted content. Nothing here is, and bucketing
                // would round `adapter.limits()` down below what the device
                // actually offers.
                apply_limit_buckets: false,
            })
            .await?;

        // Two features, and asking for only the first is a trap. `TIMESTAMP_QUERY`
        // alone permits timestamps at pass boundaries — the `timestamp_writes`
        // field of a pass descriptor — and nothing else. Writing one directly
        // into an encoder, which is how you time a span that is not exactly one
        // pass, additionally needs `TIMESTAMP_QUERY_INSIDE_ENCODERS`. Without it
        // `write_timestamp` does not produce ticks, and the symptom is a
        // measurement that succeeds and reads zero rather than one that fails:
        // a probe returning 0.0 ms looks like a very fast shader.
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
                // Nothing above asks for an `EXPERIMENTAL_`-prefixed feature,
                // and the token is the one thing that would let one through.
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

    /// Blocking acquisition with no surface, for tests and offscreen probes.
    pub fn headless() -> Result<Gpu, GpuError> {
        pollster::block_on(Gpu::new(None))
    }
}
