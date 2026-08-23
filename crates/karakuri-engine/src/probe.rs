//! Stage 7: probe measurement.
//!
//! Cost estimation may be conservative, and when it wrongly lets something
//! through this is what catches it. A candidate runs offscreen at its real
//! capacity with its real parameters, timed on the GPU, and stage 8 promotes it
//! into the live pool only if the measurement fits the frame budget.
//!
//! This is also the authority the artifact's own `perf` record defers to for
//! L4, where a per-element figure would be a fiction: point sprite cost is
//! dominated by fill rate, so only a measurement at known resolution and known
//! parameters means anything.
//!
//! ## What is measured, and what the probe accepts
//!
//! A `Probe` does not know about `Points`, `Set`, or any other pipeline — it
//! measures whatever a [`VideoSource`] records into the encoder it is handed.
//! `VideoSource` is exactly the "record commands into an encoder" seam this
//! needs: it is the same interface `karakuri-codegen`'s generated `Set` will
//! implement, and every future rendering style is required to implement it to
//! reach the screen at all (see `video_source.rs`). Measuring through it means
//! the probe times the *exact* commands promotion would later run for real,
//! not a stand-in shaped like them. Accepting a bare closure instead would
//! measure something adjacent to reality; accepting `VideoSource` measures
//! reality itself.
//!
//! ## Why GPU timestamps, not a host clock
//!
//! The host clock is not an acceptable substitute for what this number is
//! used for, when GPU timestamps are actually available. `queue.submit`
//! returns once work is enqueued, not once it has run, so a host-side
//! stopwatch around it mixes in CPU scheduling noise, driver queuing
//! behaviour, and whatever else the OS scheduler was doing — none of which
//! has anything to do with the shader stage 8 is deciding whether to
//! promote. `wgpu::QuerySet` timestamp queries are written by the GPU itself
//! at the point execution reaches them, so the difference between two
//! queries is GPU wall time and nothing else — on an adapter where they
//! actually work. See "Calibration" below for what happens when one
//! advertises support and does not deliver it, which is not a hypothetical:
//! it is what this crate's own development machine does.
//!
//! ## How the candidate gets bracketed
//!
//! `Gpu` requests `wgpu::Features::TIMESTAMP_QUERY` (and opportunistically
//! `TIMESTAMP_QUERY_INSIDE_ENCODERS`; see `gpu.rs`). `TIMESTAMP_QUERY` alone
//! covers a query set of type `Timestamp` and the `timestamp_writes` field on
//! a render- or compute-pass descriptor; writing a timestamp mid-pass or
//! directly on a `CommandEncoder` outside any pass needs the additional,
//! not-always-available features `TIMESTAMP_QUERY_INSIDE_PASSES` /
//! `TIMESTAMP_QUERY_INSIDE_ENCODERS`. This module only relies on the one
//! feature guaranteed by `timestamps: bool` (see [`Probe::new`]), so it works
//! on any adapter that advertises `TIMESTAMP_QUERY` at all.
//!
//! `VideoSource::render` opens and closes its own pass and gives the caller
//! no way to reach into its descriptor, so `Probe` cannot attach
//! `timestamp_writes` to the candidate's own pass. Instead it brackets the
//! `render` call with two minimal compute passes that request a timestamp
//! write — one whose `beginning_of_pass_write_index` fires at the start of a
//! pass recorded immediately before `render`, one whose
//! `end_of_pass_write_index` fires at the end of a pass recorded immediately
//! after — and each dispatches one workgroup of a shader that does nothing.
//! The dispatch is not a nicety: measured empirically against this crate's
//! development adapter (Metal, Apple M4 Pro), a pass with a
//! `timestamp_writes` descriptor but no actual command in it resolves to an
//! all-zero query. This matches a documented Metal driver quirk with counter
//! sampling on empty encoders (`wgpu-hal`'s own Metal backend carries the
//! same workaround for its `write_timestamp`; see
//! `wgpu-hal-*/src/metal/command.rs`).
//!
//! ## One submission per sample, not `SAMPLES` in one
//!
//! An earlier version of this measured all `SAMPLES` back-to-back inside a
//! single command buffer, on the theory that passes within one buffer
//! execute strictly in submission order. That theory holds for ordering but
//! not for latency: on a tile-based GPU the driver is free to pipeline and
//! defer the actual execution of a run of small encoders, so most
//! "back-to-back" samples read as near-zero and the real cost of the run
//! shows up as one enormous outlier wherever the driver happens to flush its
//! backlog. `run_gpu` therefore submits one command buffer per sample and
//! calls `device.poll(PollType::wait_indefinitely())` before starting the next, forcing the
//! GPU to actually finish each sample's work before the next is recorded.
//! This is the "submit and wait" pattern the render thread must never do per
//! frame — the render-thread invariants are about the *live* frame loop, and
//! a probe run is an offline background measurement, so paying `SAMPLES`
//! host round trips here is the appropriate place to spend that cost.
//!
//! Even isolated, a sample's `end` tick occasionally reads at or before its
//! `begin` tick — counter-sampling noise, not a real negative GPU duration.
//! `run_gpu` drops those pairs rather than clamping them to a misleadingly
//! fast zero, which is why `SAMPLES` is generous: the representative value
//! has to survive losing a sample or two this way.
//!
//! ## Calibration: GPU timestamps do not work on this development machine
//!
//! **State this plainly, because it is easy to rediscover the hard way and
//! costly each time:** on the adapter this crate has been developed and
//! tested against (Metal, Apple M4 Pro, macOS), `wgpu::Features::TIMESTAMP_QUERY`
//! and `TIMESTAMP_QUERY_INSIDE_ENCODERS` are both advertised and both
//! enabled, and timestamp queries do not reliably produce usable ticks. The
//! failure is not a clean, permanent "always zero" — it is worse than that:
//! a deliberately heavy, unchanging calibration workload (half a billion
//! transcendental fragment-shader ops, described below) was observed to
//! resolve to a plausible nonzero delta on some attempts and a zero or
//! non-monotonic one on others, run to run and even attempt to attempt
//! within the same process, with the encoder-level `write_timestamp` path
//! and the pass-boundary `timestamp_writes` path both affected. `docs/contributing.md`'s
//! invariant that "any change touching performance comes with a
//! GPU-timestamp measurement" is currently unenforceable on this machine —
//! not because the code here is wrong, but because the adapter's advertised
//! feature does not deliver it consistently enough to promote anything on.
//!
//! Trusting the feature flag alone would make that failure silent and
//! dangerous: a probe that returns `0.0` ms reads as an extremely fast
//! shader, and stage 8 promotes on that number. Trusting a single calibration
//! attempt is not much better, given the flakiness above — this crate's own
//! adapter was observed passing a lone `> 0` check and then failing the
//! identical check moments later. So [`Probe::new`] does neither: it runs a
//! deliberately heavy calibration workload — a full-screen triangle whose
//! fragment shader spins a 2000-iteration transcendental loop, bracketed
//! through the exact same mechanism [`Probe::run`]'s GPU path uses (see
//! [`Probe::bracket`]) — and requires `CALIBRATION_ATTEMPTS` consecutive
//! runs of it to each clear `CALIBRATION_MIN_NS`, a floor comfortably below
//! what that workload should cost on any real GPU and comfortably above
//! single-tick noise, before ever calling the timestamps trustworthy. If
//! calibration fails, `Probe` falls back to a host-clock measurement around
//! `queue.submit` + `device.poll(PollType::wait_indefinitely())` — a real number, and a
//! distinctly worse one (it includes submission and synchronization
//! overhead, and cannot isolate the render pass from anything else in its
//! command buffer) — and every [`Measurement`] this probe produces carries a
//! [`MeasurementMethod`] saying which kind of number it is, exactly so that
//! whatever promotes on it knows not to trust a `HostWallClock` figure the
//! way it would trust `GpuTimestamp`. This is the same shape as the signal
//! bus carrying a confidence rather than synthesizing a value and passing it
//! off as measured.

use std::time::Instant;

use crate::present::Present;
use crate::video_source::VideoSource;

/// How a [`Measurement`]'s `ms` was obtained. Carried alongside the number
/// because the two methods are not comparably trustworthy — see "Calibration"
/// in the module doc.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeasurementMethod {
    /// GPU timestamp queries, verified against a calibration workload known
    /// to take a measurable amount of GPU time. This is the number stage 8
    /// should promote on.
    GpuTimestamp,
    /// The adapter advertised timestamp support but a calibration workload
    /// that cannot genuinely complete in zero time resolved to a zero (or
    /// non-monotonic) delta anyway, so timestamp queries are not trusted on
    /// this device regardless of what it claims. The measurement instead
    /// wraps `queue.submit` + `device.poll(PollType::wait_indefinitely())` in a host clock:
    /// a real number, but one that includes submission and synchronization
    /// overhead and cannot isolate one pass from anything else recorded in
    /// its command buffer, so it reads coarser and biased high relative to a
    /// working `GpuTimestamp` figure.
    HostWallClock,
}

/// What a probe run measured.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Measurement {
    /// Wall time for one frame of this candidate. From GPU timestamp queries
    /// when `method` is [`MeasurementMethod::GpuTimestamp`]; from the host
    /// clock, and coarser, when it is [`MeasurementMethod::HostWallClock`].
    pub ms: f32,
    /// How `ms` was obtained. Read this before trusting `ms` the way a real
    /// GPU timestamp deserves to be trusted.
    pub method: MeasurementMethod,
    /// The capacity the measurement was taken at. A measurement without one is
    /// not comparable to anything.
    pub capacity: u32,
    /// The offscreen resolution it was taken at, which decides L4 overdraw.
    pub resolution: (u32, u32),
}

/// GPU-side resources for timestamp bracketing, present only when the
/// adapter advertised `wgpu::Features::TIMESTAMP_QUERY`. Calibration in
/// [`Probe::new`] decides whether they can actually be trusted; see the
/// module doc.
struct GpuQuery {
    /// Nanoseconds per timestamp tick, from `Queue::get_timestamp_period`.
    /// Fixed for the adapter, so it is read once up front rather than on
    /// every sample.
    period_ns: f32,
    /// Two slots — begin and end — reused for every sample. A sample's pass
    /// resets its own query range before writing, and each sample is fully
    /// resolved and read back before the next is recorded, so reuse is safe.
    query_set: wgpu::QuerySet,
    resolve_buffer: wgpu::Buffer,
    readback_buffer: wgpu::Buffer,
    /// A one-thread no-op, dispatched inside each bracketing pass so the
    /// timestamp write it requests actually lands — see the module doc.
    marker_pipeline: wgpu::ComputePipeline,
}

/// Runs a candidate offscreen and times it, preferring GPU timestamp queries
/// and falling back to a labelled host-clock measurement when the adapter's
/// queries do not survive calibration — see the module doc.
///
/// Every GPU resource the probe itself needs — the offscreen target and,
/// when timestamps are available at all, the query set, the resolve buffer,
/// the mappable readback buffer, and the marker pipeline used to bracket the
/// candidate — is created once in [`Probe::new`], including the one-off
/// calibration dispatch. [`Probe::run`] only records commands and reads back
/// results, so it allocates nothing and compiles nothing; it is safe to call
/// from a context with the render thread's "never allocate, never compile"
/// invariants in mind, modulo whatever the caller-supplied [`VideoSource`]
/// itself does (a `VideoSource` that allocates mid-`render` is not this
/// module's contract to enforce), and modulo the submit-and-wait pattern
/// `run` itself uses — appropriate for an offline probe, never for the
/// render thread's frame loop; see the module doc.
pub struct Probe {
    resolution: (u32, u32),
    method: MeasurementMethod,
    target_view: wgpu::TextureView,
    /// Kept alive alongside `target_view`: nothing reads it back, only the
    /// view is used as a render attachment, but the texture is what owns the
    /// underlying allocation.
    #[allow(dead_code)]
    target: wgpu::Texture,
    /// `Some` whenever the adapter advertised `TIMESTAMP_QUERY`, regardless
    /// of whether calibration ended up trusting it — calibration itself
    /// needs these resources to run its check.
    gpu: Option<GpuQuery>,
}

impl Probe {
    /// Offscreen target format. The pipeline has exactly one render target
    /// format in V1 — every `VideoSource` renders `Rgba16Float` — so this
    /// reuses `Present::HDR_FORMAT` rather than taking a parameter with a
    /// single call site.
    const FORMAT: wgpu::TextureFormat = Present::HDR_FORMAT;

    /// Samples taken per [`Probe::run`].
    ///
    /// One run is not one measurement: a single sample is not representative
    /// of anything but itself, and a mean across samples is skewed hard by
    /// the first one, which pays for pipeline and cache state settling that
    /// every later frame gets for free. `run` takes `SAMPLES` isolated
    /// timings, discards the first as that cold outlier and (on the GPU
    /// path) any that came back non-monotonic, and reports the **median** of
    /// what is left — a median rather than a mean because it takes one
    /// scheduling hiccup on any single remaining sample to skew a mean, and
    /// the whole point of measuring on the GPU instead of the host clock is
    /// to keep exactly that kind of noise out of the number stage 8 promotes
    /// on. `SAMPLES` is generous precisely because discards shrink the
    /// usable pool.
    const SAMPLES: u32 = 16;

    /// Calibration render target size. Calibration times a `render` pass —
    /// not a `compute` dispatch — because that is what every real
    /// `VideoSource` records and what `run`'s GPU path actually brackets;
    /// see the note on [`Probe::calibrate`] about why the two are not
    /// interchangeable for this purpose on at least one real adapter.
    const CALIBRATION_SIZE: u32 = 512;
    /// Fragment-shader loop iterations in the calibration workload. Combined
    /// with `CALIBRATION_SIZE`, this is `512 * 512 * 2000` ≈ half a billion
    /// transcendental ops — large enough that no real GPU could complete it
    /// in zero time, so a zero (or non-monotonic) bracketed delta can only
    /// mean the timestamp mechanism itself is not delivering.
    const CALIBRATION_ITERATIONS: u32 = 2_000;
    /// Minimum plausible duration for the calibration workload, in
    /// nanoseconds. Comfortably below what `CALIBRATION_SIZE` ×
    /// `CALIBRATION_ITERATIONS` of transcendental math should take on any
    /// real GPU, and comfortably above single-tick noise: a flat `> 0` check
    /// let through readings that were not reproducible — this crate's own
    /// development adapter passed a `> 0` check on one calibration attempt
    /// and failed it moments later from a freshly constructed `Probe`
    /// running the identical workload, which a working timestamp mechanism
    /// would not do. Requiring a real floor, and requiring every attempt to
    /// clear it (see [`Probe::calibrate`]) rather than just one, is what
    /// turned that flakiness into a consistent, correct verdict.
    const CALIBRATION_MIN_NS: f64 = 100_000.0;
    /// Consecutive attempts calibration must clear, every one, before the
    /// adapter is trusted. Not a small number: this crate's own development
    /// adapter was observed clearing a `> 0` check on some attempts and
    /// failing it on others for the identical, unchanging workload, so a
    /// short streak is not enough to tell a genuinely working mechanism from
    /// one that is flaky in a way that happens to pass a couple of times in
    /// a row. Ten in a row still costs a small, one-time fraction of a
    /// second at `Probe::new` time and is cheap insurance against exactly
    /// the false positive that made an earlier, laxer version of this check
    /// unreliable.
    const CALIBRATION_ATTEMPTS: u32 = 10;
    /// How far below the host clock a GPU timestamp may be before it is not
    /// believed: the host number divided by this. Four is loose on purpose —
    /// the host figure carries submission, two marker passes, a buffer mapping
    /// and a poll, and on a fast GPU those can genuinely be most of it. What it
    /// has to catch is not a factor of two, it is the factor of six hundred
    /// that a lying adapter produced.
    const PLAUSIBILITY_RATIO: f64 = 4.0;
    /// Below this the host figure is mostly its own overhead and the ratio says
    /// nothing, so the check does not apply. Five milliseconds: submission and
    /// synchronisation are well under a millisecond, so a figure this size is
    /// dominated by real work.
    const PLAUSIBILITY_FLOOR_NS: f64 = 5_000_000.0;

    /// Allocate a probe at a given offscreen `resolution`.
    ///
    /// `resolution` is fixed here because it sizes the offscreen target this
    /// allocates up front; `capacity` is not part of this constructor at
    /// all, because it belongs to a candidate rather than to the probe
    /// infrastructure — it is [`Probe::run`]'s parameter instead, so one
    /// `Probe` (and the one, sometimes non-trivial, calibration decision it
    /// makes — see below) can measure several candidates' worth of
    /// capacities at the same resolution without recalibrating for each.
    /// That matters beyond convenience: calibration was observed to be
    /// occasionally flaky in isolation (see [`Probe::calibrate`]), so two
    /// independently-constructed probes measuring the same adapter moments
    /// apart could land on different [`MeasurementMethod`]s and make their
    /// `Measurement`s incomparable; one `Probe` measuring both candidates
    /// cannot disagree with itself.
    ///
    /// `timestamps` is `Gpu::timestamps` in real use — whether
    /// `wgpu::Features::TIMESTAMP_QUERY` was available when the device was
    /// requested. It is taken as a plain `bool` rather than read from a `Gpu`
    /// internally so that the degraded path is reachable without needing
    /// hardware or a driver that actually lacks the feature: a caller (or a
    /// test) can force it. When it is `true`, `new` still runs a calibration
    /// check before trusting it — see "Calibration" in the module doc — so
    /// the returned `Probe`'s actual measurement method can end up
    /// [`MeasurementMethod::HostWallClock`] even when `timestamps` was
    /// `true`. This never fails: a candidate can always be measured somehow,
    /// and [`Measurement::method`] says how.
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

    /// Bracket a chunk of GPU work — recorded by `middle` into the same
    /// encoder — between a begin marker pass and an end marker pass, submit,
    /// wait for the GPU to finish, and read back the two timestamps as a
    /// `(begin, end)` tick pair. Both [`Probe::calibrate`] and real sampling
    /// go through this, so calibration proves the exact mechanism `run`'s
    /// GPU path uses rather than a different one that happens to look
    /// similar — an earlier version of this module calibrated with both
    /// timestamp writes on a single pass, which turned out to behave more
    /// reliably on this adapter than the two-separate-passes shape `render`'s
    /// opaque pass boundary forces here, and so passed calibration while the
    /// real path underneath it was still not producing usable deltas. See
    /// "Calibration" in the module doc.
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
        // **The same submission, timed twice.** The host clock around
        // submit-and-wait is coarse and biased *high* — it includes
        // submission, the two marker passes, the mapping and the poll — which
        // is exactly what makes it useful here: it is an upper bound on the
        // GPU time, and a timestamp delta far below it is a timestamp that is
        // not measuring this work. Free, because the wait happens anyway.
        let started = Instant::now();
        queue.submit([encoder.finish()]);

        // Wait for this sample to fully complete on the GPU before the next
        // one is recorded — see "One submission per sample" in the module
        // doc for why a pipelined batch cannot be trusted here.
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

    /// **Whether a GPU timestamp delta is plausible against what the host saw
    /// of the same submission.**
    ///
    /// The check that a fixed floor could not be. `CALIBRATION_MIN_NS` was
    /// 0.1 ms against a workload costing tens of milliseconds — a thousand
    /// times too low — and an adapter returning readings of about a tenth of a
    /// millisecond cleared it while measuring nothing. A constant cannot be set
    /// safely here: too low and it passes garbage, too high and it fails a
    /// genuinely fast GPU. A second measurement of the *same work* has no such
    /// problem, and its bias is in the direction that makes it usable — the
    /// host clock includes submission and synchronisation, so it can only
    /// over-state, and a GPU delta far *below* it is the only shape this can
    /// flag.
    ///
    /// **Only when the host number is large enough for its overhead to be a
    /// minor part of it**, which is why [`Probe::PLAUSIBILITY_FLOOR_NS`]
    /// exists. On a cheap candidate the host figure is mostly submit-and-wait,
    /// so the ratio means nothing — and nothing is at stake either: cheap
    /// material measured as cheap is the right answer whichever clock said so.
    /// The check bites exactly where a wrong answer is dangerous, which is
    /// expensive material measured as nearly free.
    fn plausible(gpu_ns: f64, host_ns: f64) -> bool {
        host_ns < Self::PLAUSIBILITY_FLOOR_NS || gpu_ns * Self::PLAUSIBILITY_RATIO >= host_ns
    }

    /// Bracket a deliberately heavy `render` pass through [`Probe::bracket`]
    /// — the exact mechanism `run`'s GPU path uses — and check that the
    /// result is unambiguously nonzero. See "Calibration" in the module doc
    /// for why the feature flag alone cannot be trusted.
    ///
    /// This times a full-screen triangle with a fragment shader that spins a
    /// transcendental loop, not a compute dispatch. That distinction is not
    /// cosmetic: an earlier version of this calibrated with compute work and
    /// passed reliably on this crate's development adapter (Metal, Apple M4
    /// Pro) while `run_gpu` bracketing an actual `render` pass still measured
    /// unrelated candidates as indistinguishable from each other. Every real
    /// `VideoSource` records a render pass, so calibration has to bracket one
    /// too, or it is validating a mechanism `run` does not actually use.
    ///
    /// Requires every one of `CALIBRATION_ATTEMPTS` to clear
    /// `CALIBRATION_MIN_NS`, not just one: this workload's cost does not
    /// change between attempts, so a real timestamp mechanism should report
    /// a plausible duration every time, and a single passing attempt among
    /// failing ones is exactly the flaky signal `CALIBRATION_MIN_NS`'s own
    /// doc comment describes, not evidence the adapter can be trusted.
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

            // **Against what the host saw of the same submission, not against
            // a constant.** The constant was the defect: `CALIBRATION_MIN_NS`
            // is a thousandth of what this workload costs, so an adapter
            // returning about a tenth of a millisecond cleared it while
            // measuring nothing at all. The floor is kept as a second, weaker
            // guard — a delta below it is not believable on any hardware — but
            // the ratio is what decides.
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
                // Cold: pays for pipeline and cache state settling every
                // later sample gets for free. Not representative.
                continue;
            }
            // `end` is occasionally observed at or before `begin` —
            // counter-sampling noise from the driver, not a real negative
            // duration (GPU wall time cannot run backwards). Clamping that
            // to zero would enter a spuriously "instant" measurement into
            // the pool; dropping it keeps the representative value built
            // only from pairs the hardware actually reported as ordered.
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

        // **Checked again here, and not only at calibration.** The adapter that
        // produced this check passed calibration and then measured two million
        // point sprites at less than a tenth of a millisecond — the flakiness
        // this module already documents is attempt to attempt within one
        // process, so a verdict taken once at construction does not hold for
        // the life of a probe.
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

    /// Time one host-clock sample: wrap `source.render`'s submission and the
    /// wait for it to finish with `Instant`. Coarser than a GPU timestamp by
    /// construction — see [`MeasurementMethod::HostWallClock`] — but a real
    /// number rather than an invented one.
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
                continue; // Cold, as above.
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

    /// Run the candidate `SAMPLES` times offscreen and return the
    /// representative measurement, labelled with how it was obtained (see
    /// [`MeasurementMethod`]).
    ///
    /// On the [`MeasurementMethod::GpuTimestamp`] path, what is included is
    /// the GPU work `source.render` records — its render pass(es), clears,
    /// draws — plus the small, roughly constant cost of the two bracketing
    /// marker passes described in the module doc; what is excluded is
    /// anything the caller did before calling `run` (a `VideoSource::prepare`
    /// step that uploads uniforms via `queue.write_buffer`, for instance,
    /// runs on the queue timeline before this encoder and is not part of
    /// it), the cost of `queue.submit`, and the final present/sRGB pass —
    /// `Present::draw` is a separate pipeline this never calls. On the
    /// [`MeasurementMethod::HostWallClock`] path, submission and
    /// synchronization overhead are included too, and nothing separates the
    /// render pass from anything else that happened to be timed around it.
    /// The first of the `SAMPLES` timings is always discarded before
    /// computing the representative value; see the note on `SAMPLES`.
    ///
    /// `capacity` is recorded on the returned [`Measurement`] but otherwise
    /// unused here — it is `source`'s business, not the probe's, to have
    /// been constructed at that capacity; see [`Probe::new`] for why it is
    /// this method's parameter rather than the constructor's.
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
                    // **Distrusted for the rest of this probe's life, not just
                    // for this measurement.** The two methods are not
                    // comparable — the module doc says so and the governor sums
                    // them — so a probe that answered with a GPU number and
                    // then a host number would hand the budget two figures on
                    // different scales and no way to tell. Falling back for
                    // good keeps every number a probe produces comparable with
                    // every other, which is the property an admission decision
                    // actually rests on.
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

    /// **The reading that got through, against the check written for it.**
    ///
    /// `tests/probe.rs` recorded what the adapter actually produced when it
    /// lied: two million point sprites at 0.095 ms while the same submission
    /// took tens of milliseconds on the host clock. The old guard was a
    /// constant floor of 0.1 ms, so 0.095 ms sat *just under* the only bar
    /// there was — and a floor set high enough to catch it would fail a
    /// genuinely fast GPU.
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

    /// **Below the floor the check does not apply**, and that is deliberate:
    /// on a cheap candidate the host figure is mostly its own overhead, so the
    /// ratio would fail an honest measurement. Nothing is at stake there —
    /// cheap material measured as cheap is right whichever clock said it — and
    /// the check is aimed at the case that is dangerous, expensive material
    /// measured as nearly free.
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
