//! Offscreen GPU probe measurement for candidates.
//!
//! Measures candidate execution timing using GPU timestamp queries when available and calibrated,
//! falling back to host wall-clock timing if timestamp queries are unsupported or unreliable.

use std::time::Instant;

use crate::present::Present;
use crate::video_source::VideoSource;

/// Visual degeneracy detected from rendered texture readback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Degeneracy {
    NaNDetected,
    AllZeroAlpha,
    PureBlack,
}

/// Helper to detect visual degeneracies (zero alpha, pure black screen, NaN/Inf) from texture pixel data.
/// Returns `Some(Degeneracy)` if a visual degeneracy is detected, or `None` if the frame is healthy.
pub fn check_degeneracy(texture_data: &[u8], format: wgpu::TextureFormat) -> Option<Degeneracy> {
    if texture_data.is_empty() {
        return None;
    }

    match format {
        wgpu::TextureFormat::Rgba16Float => {
            let pixel_size = 8;
            if texture_data.len() < pixel_size {
                return None;
            }
            let mut all_zero_alpha = true;
            let mut all_zero_rgb = true;
            let mut nan_detected = false;

            for chunk in texture_data.chunks_exact(pixel_size) {
                let r = u16::from_le_bytes([chunk[0], chunk[1]]);
                let g = u16::from_le_bytes([chunk[2], chunk[3]]);
                let b = u16::from_le_bytes([chunk[4], chunk[5]]);
                let a = u16::from_le_bytes([chunk[6], chunk[7]]);

                // In IEEE 754 half-float:
                // (bits & 0x7C00) == 0x7C00 indicates NaN or Inf
                if (r & 0x7C00) == 0x7C00
                    || (g & 0x7C00) == 0x7C00
                    || (b & 0x7C00) == 0x7C00
                    || (a & 0x7C00) == 0x7C00
                {
                    nan_detected = true;
                    break;
                }

                if (a & 0x7FFF) != 0 {
                    all_zero_alpha = false;
                }
                if (r & 0x7FFF) != 0 || (g & 0x7FFF) != 0 || (b & 0x7FFF) != 0 {
                    all_zero_rgb = false;
                }
            }

            if nan_detected {
                Some(Degeneracy::NaNDetected)
            } else if all_zero_alpha {
                Some(Degeneracy::AllZeroAlpha)
            } else if all_zero_rgb {
                Some(Degeneracy::PureBlack)
            } else {
                None
            }
        }
        wgpu::TextureFormat::Rgba32Float => {
            let pixel_size = 16;
            if texture_data.len() < pixel_size {
                return None;
            }
            let mut all_zero_alpha = true;
            let mut all_zero_rgb = true;
            let mut nan_detected = false;

            for chunk in texture_data.chunks_exact(pixel_size) {
                let r = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                let g = f32::from_le_bytes([chunk[4], chunk[5], chunk[6], chunk[7]]);
                let b = f32::from_le_bytes([chunk[8], chunk[9], chunk[10], chunk[11]]);
                let a = f32::from_le_bytes([chunk[12], chunk[13], chunk[14], chunk[15]]);

                if r.is_nan()
                    || r.is_infinite()
                    || g.is_nan()
                    || g.is_infinite()
                    || b.is_nan()
                    || b.is_infinite()
                    || a.is_nan()
                    || a.is_infinite()
                {
                    nan_detected = true;
                    break;
                }

                if a != 0.0 {
                    all_zero_alpha = false;
                }
                if r != 0.0 || g != 0.0 || b != 0.0 {
                    all_zero_rgb = false;
                }
            }

            if nan_detected {
                Some(Degeneracy::NaNDetected)
            } else if all_zero_alpha {
                Some(Degeneracy::AllZeroAlpha)
            } else if all_zero_rgb {
                Some(Degeneracy::PureBlack)
            } else {
                None
            }
        }
        wgpu::TextureFormat::R16Float => {
            let pixel_size = 2;
            if texture_data.len() < pixel_size {
                return None;
            }
            let mut all_zero = true;
            let mut nan_detected = false;

            for chunk in texture_data.chunks_exact(pixel_size) {
                let v = u16::from_le_bytes([chunk[0], chunk[1]]);
                if (v & 0x7C00) == 0x7C00 {
                    nan_detected = true;
                    break;
                }
                if (v & 0x7FFF) != 0 {
                    all_zero = false;
                }
            }

            if nan_detected {
                Some(Degeneracy::NaNDetected)
            } else if all_zero {
                Some(Degeneracy::PureBlack)
            } else {
                None
            }
        }
        wgpu::TextureFormat::R32Float => {
            let pixel_size = 4;
            if texture_data.len() < pixel_size {
                return None;
            }
            let mut all_zero = true;
            let mut nan_detected = false;

            for chunk in texture_data.chunks_exact(pixel_size) {
                let v = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                if v.is_nan() || v.is_infinite() {
                    nan_detected = true;
                    break;
                }
                if v != 0.0 {
                    all_zero = false;
                }
            }

            if nan_detected {
                Some(Degeneracy::NaNDetected)
            } else if all_zero {
                Some(Degeneracy::PureBlack)
            } else {
                None
            }
        }
        wgpu::TextureFormat::Rgba8Unorm | wgpu::TextureFormat::Rgba8UnormSrgb => {
            let pixel_size = 4;
            if texture_data.len() < pixel_size {
                return None;
            }
            let mut all_zero_alpha = true;
            let mut all_zero_rgb = true;

            for chunk in texture_data.chunks_exact(pixel_size) {
                let r = chunk[0];
                let g = chunk[1];
                let b = chunk[2];
                let a = chunk[3];

                if a != 0 {
                    all_zero_alpha = false;
                }
                if r != 0 || g != 0 || b != 0 {
                    all_zero_rgb = false;
                }
            }

            if all_zero_alpha {
                Some(Degeneracy::AllZeroAlpha)
            } else if all_zero_rgb {
                Some(Degeneracy::PureBlack)
            } else {
                None
            }
        }
        wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb => {
            let pixel_size = 4;
            if texture_data.len() < pixel_size {
                return None;
            }
            let mut all_zero_alpha = true;
            let mut all_zero_rgb = true;

            for chunk in texture_data.chunks_exact(pixel_size) {
                let b = chunk[0];
                let g = chunk[1];
                let r = chunk[2];
                let a = chunk[3];

                if a != 0 {
                    all_zero_alpha = false;
                }
                if r != 0 || g != 0 || b != 0 {
                    all_zero_rgb = false;
                }
            }

            if all_zero_alpha {
                Some(Degeneracy::AllZeroAlpha)
            } else if all_zero_rgb {
                Some(Degeneracy::PureBlack)
            } else {
                None
            }
        }
        wgpu::TextureFormat::R8Unorm => {
            if texture_data.iter().all(|&b| b == 0) {
                Some(Degeneracy::PureBlack)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Timing mechanism used to obtain a Measurement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeasurementMethod {
    /// GPU timestamp queries bracketed around the candidate render pass.
    GpuTimestamp,
    /// Host wall-clock timing around queue submission and device synchronization.
    HostWallClock,
}

/// Execution time measurement recorded by a Probe.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Measurement {
    /// Wall time for one frame of this candidate in milliseconds.
    pub ms: f32,
    /// Measurement method used to record elapsed time.
    pub method: MeasurementMethod,
    /// Candidate element capacity during measurement.
    pub capacity: u32,
    /// Offscreen render resolution during measurement.
    pub resolution: (u32, u32),
}

/// Internal GPU resources for timestamp query bracketing.
struct GpuQuery {
    /// Nanoseconds per timestamp tick.
    period_ns: f32,
    /// Timestamp query set holding begin and end query slots.
    query_set: wgpu::QuerySet,
    resolve_buffer: wgpu::Buffer,
    readback_buffer: wgpu::Buffer,
    /// Compute pipeline dispatched to anchor timestamp writes.
    marker_pipeline: wgpu::ComputePipeline,
}

/// Offline benchmark harness that renders a VideoSource to an offscreen target.
pub struct Probe {
    resolution: (u32, u32),
    method: MeasurementMethod,
    target_view: wgpu::TextureView,
    #[allow(dead_code)]
    target: wgpu::Texture,
    gpu: Option<GpuQuery>,
}

impl Probe {
    /// Offscreen render target format.
    const FORMAT: wgpu::TextureFormat = Present::HDR_FORMAT;

    /// Number of sample executions taken per measurement run.
    const SAMPLES: u32 = 16;

    /// Calibration render target dimension in pixels.
    const CALIBRATION_SIZE: u32 = 512;
    /// Fragment-shader loop iterations in the calibration workload.
    const CALIBRATION_ITERATIONS: u32 = 2_000;
    /// Minimum expected duration for the calibration workload in nanoseconds.
    const CALIBRATION_MIN_NS: f64 = 100_000.0;
    /// Required consecutive successful calibration attempts before timestamp queries are trusted.
    const CALIBRATION_ATTEMPTS: u32 = 10;
    /// Maximum allowed ratio between host time and GPU timestamp time.
    const PLAUSIBILITY_RATIO: f64 = 4.0;
    /// Minimum host duration in nanoseconds required to evaluate the plausibility ratio.
    const PLAUSIBILITY_FLOOR_NS: f64 = 5_000_000.0;

    /// Creates a new Probe configured for the given offscreen resolution.
    ///
    /// Validates GPU timestamp queries against a calibration workload if requested,
    /// falling back to host wall-clock timing if calibration fails.
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        timestamps: bool,
        resolution: (u32, u32),
    ) -> Probe {
        let (width, height) = resolution;
        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("probe target"),
            size: wgpu::Extent3d {
                width: width.max(1),
                height: height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let target_view = target.create_view(&Default::default());

        let gpu = if timestamps {
            Some(Self::make_gpu_query(device, queue))
        } else {
            None
        };

        let method = match &gpu {
            Some(gpu_query) if Self::calibrate(device, queue, gpu_query) => {
                MeasurementMethod::GpuTimestamp
            }
            _ => MeasurementMethod::HostWallClock,
        };

        Probe {
            resolution,
            method,
            target_view,
            target,
            gpu,
        }
    }

    /// Reallocates the offscreen render target to a new resolution while preserving calibration state.
    pub fn resize(&mut self, device: &wgpu::Device, resolution: (u32, u32)) {
        if self.resolution == resolution {
            return;
        }
        let (width, height) = resolution;
        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("probe target"),
            size: wgpu::Extent3d {
                width: width.max(1),
                height: height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        self.target_view = target.create_view(&Default::default());
        self.target = target;
        self.resolution = resolution;
    }

    /// Returns the current offscreen target resolution.
    pub fn resolution(&self) -> (u32, u32) {
        self.resolution
    }

    /// Returns the active measurement method for this probe.
    pub fn method(&self) -> MeasurementMethod {
        self.method
    }

    fn make_gpu_query(device: &wgpu::Device, queue: &wgpu::Queue) -> GpuQuery {
        let query_set = device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("probe timestamps"),
            ty: wgpu::QueryType::Timestamp,
            count: 2,
        });

        // One `u64` tick count per query. `resolve_query_set` writes 8 bytes
        // per query; the destination offset (0 here) has to be a multiple of
        // `QUERY_RESOLVE_BUFFER_ALIGNMENT`, which 0 trivially is.
        let query_bytes = 2 * 8;
        let resolve_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("probe resolve"),
            size: query_bytes,
            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let readback_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("probe readback"),
            size: query_bytes,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let marker_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("probe marker"),
            source: wgpu::ShaderSource::Wgsl("@compute @workgroup_size(1) fn main() {}".into()),
        });
        let marker_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("probe marker"),
            layout: None,
            module: &marker_shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        GpuQuery {
            period_ns: queue.get_timestamp_period(),
            query_set,
            resolve_buffer,
            readback_buffer,
            marker_pipeline,
        }
    }

    /// Brackets an encoder action between timestamp marker passes and returns tick deltas and host time.
    fn bracket(
        gpu: &GpuQuery,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        middle: impl FnOnce(&mut wgpu::CommandEncoder),
    ) -> (u64, u64, f64) {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("probe sample"),
        });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("probe begin"),
                timestamp_writes: Some(wgpu::ComputePassTimestampWrites {
                    query_set: &gpu.query_set,
                    beginning_of_pass_write_index: Some(0),
                    end_of_pass_write_index: None,
                }),
            });
            pass.set_pipeline(&gpu.marker_pipeline);
            pass.dispatch_workgroups(1, 1, 1);
        }
        middle(&mut encoder);
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("probe end"),
                timestamp_writes: Some(wgpu::ComputePassTimestampWrites {
                    query_set: &gpu.query_set,
                    beginning_of_pass_write_index: None,
                    end_of_pass_write_index: Some(1),
                }),
            });
            pass.set_pipeline(&gpu.marker_pipeline);
            pass.dispatch_workgroups(1, 1, 1);
        }
        encoder.resolve_query_set(&gpu.query_set, 0..2, &gpu.resolve_buffer, 0);
        encoder.copy_buffer_to_buffer(
            &gpu.resolve_buffer,
            0,
            &gpu.readback_buffer,
            0,
            gpu.resolve_buffer.size(),
        );
        let started = Instant::now();
        queue.submit([encoder.finish()]);

        let slice = gpu.readback_buffer.slice(..);
        slice.map_async(wgpu::MapMode::Read, |r| r.expect("map probe readback"));
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("poll");
        let host_ns = started.elapsed().as_secs_f64() * 1_000_000_000.0;
        let data = slice.get_mapped_range().expect("map probe readback");
        let begin = u64::from_le_bytes(data[0..8].try_into().expect("8-byte chunk"));
        let end = u64::from_le_bytes(data[8..16].try_into().expect("8-byte chunk"));
        drop(data);
        gpu.readback_buffer.unmap();

        (begin, end, host_ns)
    }

    /// Checks whether a measured GPU timestamp delta is plausible compared to host elapsed time.
    fn plausible(gpu_ns: f64, host_ns: f64) -> bool {
        host_ns < Self::PLAUSIBILITY_FLOOR_NS || gpu_ns * Self::PLAUSIBILITY_RATIO >= host_ns
    }

    /// Validates GPU timestamp queries against a known heavy calibration shader.
    fn calibrate(device: &wgpu::Device, queue: &wgpu::Queue, gpu: &GpuQuery) -> bool {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("probe calibration"),
            source: wgpu::ShaderSource::Wgsl(
                format!(
                    "\
struct VsOut {{
    @builtin(position) clip: vec4<f32>,
}};

@vertex
fn vs(@builtin(vertex_index) i: u32) -> VsOut {{
    var pos = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    var out: VsOut;
    out.clip = vec4<f32>(pos[i], 0.0, 1.0);
    return out;
}}

@fragment
fn fs() -> @location(0) vec4<f32> {{
    var x: f32 = 0.0001;
    for (var i: u32 = 0u; i < {iterations}u; i = i + 1u) {{
        x = sin(x) + cos(x * 1.0001);
    }}
    return vec4<f32>(x, x, x, 1.0);
}}
",
                    iterations = Self::CALIBRATION_ITERATIONS,
                )
                .into(),
            ),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("probe calibration"),
            bind_group_layouts: &[],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("probe calibration"),
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
                targets: &[Some(Self::FORMAT.into())],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("probe calibration target"),
            size: wgpu::Extent3d {
                width: Self::CALIBRATION_SIZE,
                height: Self::CALIBRATION_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let target_view = target.create_view(&Default::default());

        for _ in 0..Self::CALIBRATION_ATTEMPTS {
            let (begin, end, host_ns) = Self::bracket(gpu, device, queue, |encoder| {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("probe calibration work"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &target_view,
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
                pass.set_pipeline(&pipeline);
                pass.draw(0..3, 0..1);
            });

            let believable = end
                .checked_sub(begin)
                .map(|delta| delta as f64 * f64::from(gpu.period_ns))
                .is_some_and(|delta_ns| {
                    delta_ns >= Self::CALIBRATION_MIN_NS && Self::plausible(delta_ns, host_ns)
                });
            if !believable {
                return false;
            }
        }
        true
    }

    #[allow(clippy::too_many_arguments)]
    fn run_gpu(
        gpu: &GpuQuery,
        target_view: &wgpu::TextureView,
        resolution: (u32, u32),
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        source: &mut dyn VideoSource,
        steps: u8,
        capacity: u32,
    ) -> Result<Measurement, f64> {
        let mut deltas_ns: Vec<f64> = Vec::with_capacity(Self::SAMPLES as usize - 1);
        let mut host_ns: Vec<f64> = Vec::with_capacity(Self::SAMPLES as usize - 1);
        for i in 0..Self::SAMPLES {
            let (begin, end, host) = Self::bracket(gpu, device, queue, |encoder| {
                source.render(encoder, target_view, steps);
            });
            if i == 0 {
                continue;
            }
            host_ns.push(host);
            if let Some(delta) = end.checked_sub(begin) {
                deltas_ns.push(delta as f64 * f64::from(gpu.period_ns));
            }
        }
        assert!(
            !deltas_ns.is_empty(),
            "every sample produced a non-monotonic timestamp pair after calibration passed; \
             the adapter's timestamp queries cannot be trusted right now"
        );
        deltas_ns.sort_by(|a, b| a.partial_cmp(b).expect("timestamp deltas are finite"));
        let median_ns = deltas_ns[deltas_ns.len() / 2];
        host_ns.sort_by(|a, b| a.partial_cmp(b).expect("elapsed times are finite"));
        let host_median_ns = host_ns[host_ns.len() / 2];

        if !Self::plausible(median_ns, host_median_ns) {
            return Err(host_median_ns);
        }

        Ok(Measurement {
            ms: (median_ns / 1_000_000.0) as f32,
            method: MeasurementMethod::GpuTimestamp,
            capacity,
            resolution,
        })
    }

    /// Times a single sample using host wall-clock elapsed time.
    fn sample_host(
        target_view: &wgpu::TextureView,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        source: &mut dyn VideoSource,
        steps: u8,
    ) -> f64 {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("probe sample (host clock)"),
        });
        source.render(&mut encoder, target_view, steps);
        let start = Instant::now();
        queue.submit([encoder.finish()]);
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("poll");
        start.elapsed().as_secs_f64() * 1_000.0
    }

    fn run_host(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        source: &mut dyn VideoSource,
        steps: u8,
        capacity: u32,
    ) -> Measurement {
        let mut samples_ms: Vec<f64> = Vec::with_capacity(Self::SAMPLES as usize - 1);
        for i in 0..Self::SAMPLES {
            let ms = Self::sample_host(&self.target_view, device, queue, source, steps);
            if i == 0 {
                continue;
            }
            samples_ms.push(ms);
        }
        samples_ms.sort_by(|a, b| a.partial_cmp(b).expect("elapsed times are finite"));
        let median_ms = samples_ms[samples_ms.len() / 2];

        Measurement {
            ms: median_ms as f32,
            method: MeasurementMethod::HostWallClock,
            capacity,
            resolution: self.resolution,
        }
    }

    /// Measures candidate execution time across multiple samples and returns the median result.
    pub fn run(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        source: &mut dyn VideoSource,
        steps: u8,
        capacity: u32,
    ) -> Measurement {
        match (&self.gpu, self.method) {
            (Some(gpu), MeasurementMethod::GpuTimestamp) => {
                match Self::run_gpu(
                    gpu,
                    &self.target_view,
                    self.resolution,
                    device,
                    queue,
                    source,
                    steps,
                    capacity,
                ) {
                    Ok(measurement) => measurement,
                    Err(_) => {
                        self.method = MeasurementMethod::HostWallClock;
                        self.run_host(device, queue, source, steps, capacity)
                    }
                }
            }
            _ => self.run_host(device, queue, source, steps, capacity),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies that implausibly fast GPU readings are rejected by the plausibility check.
    #[test]
    fn the_reading_that_got_through_a_constant_floor_does_not_get_through_this() {
        let lying_ns = 95_000.0;
        let host_ns = 60_000_000.0;
        assert!(
            !Probe::plausible(lying_ns, host_ns),
            "the reading this check exists for was believed"
        );
        // And the shape of the old bar, for the record: it cleared the floor
        // by all of five microseconds.
        assert!(lying_ns < Probe::CALIBRATION_MIN_NS);
    }

    /// An honest measurement is believed even when the host clock is much
    /// larger, because it always is: the host figure carries submission, two
    /// marker passes, a mapping and a poll. The ratio is loose for that reason
    /// and only has to catch a factor of hundreds.
    #[test]
    fn an_honest_measurement_survives_the_hosts_overhead() {
        // A GPU that did the work in a quarter of what the host observed.
        assert!(Probe::plausible(15_000_000.0, 60_000_000.0));
        // And one that did it in most of it.
        assert!(Probe::plausible(55_000_000.0, 60_000_000.0));
    }

    /// Verifies that the plausibility ratio check is bypassed below the floor duration.
    #[test]
    fn a_cheap_candidate_is_not_judged_against_its_own_overhead() {
        // 0.05 ms of GPU work inside a 1 ms submit-and-wait: a ratio of twenty,
        // and entirely normal.
        assert!(Probe::plausible(50_000.0, 1_000_000.0));
        // The floor is where that stops being excused.
        assert!(!Probe::plausible(
            50_000.0,
            Probe::PLAUSIBILITY_FLOOR_NS * 2.0
        ));
    }
}
