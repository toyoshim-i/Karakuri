//! Stage 7 exercised headlessly: a probe measurement has to be connected to
//! real GPU cost, not merely shaped like one, so `a_heavier_workload_measures_as_heavier`
//! is the load-bearing test here — everything else can pass by returning a
//! plausible constant, that one cannot.
//!
//! These tests run on whichever [`MeasurementMethod`] this machine's adapter
//! actually earns (see `probe.rs`'s module doc, "Calibration"): they never
//! skip based on `Gpu::timestamps`, because `Probe::new` always produces a
//! working, labelled measurement — GPU timestamps when calibration trusts
//! them, a host-clock fallback when it does not. On this crate's own
//! development machine (Metal, Apple M4 Pro) calibration currently never
//! survives ten consecutive checks, so every run here exercises the
//! `HostWallClock` path; on an adapter whose timestamps genuinely work, the
//! same tests exercise `GpuTimestamp` instead. Either way the assertions
//! hold, which is the point of labelling the method rather than skipping
//! around it.

use karakuri_engine::probe::MeasurementMethod;
use karakuri_engine::{Gpu, Params, Points, Probe};

const RESOLUTION: (u32, u32) = (256, 256);

/// `Gpu::headless()` fails when there is no adapter at all, which happens on
/// some CI and sandboxed machines. That is an environment fact, not a defect
/// in the probe, so tests skip rather than fail when it happens.
fn gpu() -> Option<Gpu> {
    match Gpu::headless() {
        Ok(gpu) => Some(gpu),
        Err(err) => {
            eprintln!("skipping: no GPU available ({err})");
            None
        }
    }
}

#[test]
fn a_trivial_measurement_reports_its_capacity_and_resolution() {
    let Some(gpu) = gpu() else { return };

    let probe = Probe::new(&gpu.device, &gpu.queue, gpu.timestamps, RESOLUTION);

    let capacity = 1024;
    let mut points = Points::new(&gpu.device, capacity, 19274);
    points.resize(RESOLUTION.0, RESOLUTION.1);
    points.prepare(&gpu.queue, 1);

    let measurement = probe.run(&gpu.device, &gpu.queue, &mut points, 1, capacity);

    eprintln!("measured via {:?}", measurement.method);
    assert_eq!(measurement.capacity, capacity);
    assert_eq!(measurement.resolution, RESOLUTION);
    assert!(
        measurement.ms.is_finite() && measurement.ms >= 0.0,
        "ms = {}",
        measurement.ms
    );
}

#[test]
fn a_heavier_workload_measures_as_heavier() {
    // Additive point sprites: cost scales with instance count (more quads to
    // shade) and with quad size (more fragments per quad). Cranking both by
    // orders of magnitude makes the two measurements unmistakably different,
    // which is the point of this test — it is the one that actually proves
    // `ms` tracks real GPU work rather than a plausible constant. It has to
    // pass however this machine ends up measuring (see the module doc): the
    // host-clock fallback is coarser than a working GPU timestamp, but a
    // workload this much heavier still has to show up in it, or the number
    // is not connected to reality on either path.
    //
    // One `Probe` measures both candidates, deliberately: `Probe::new` binds
    // `resolution` but not `capacity` for exactly this reason (see its doc)
    // — two independently-constructed probes could in principle land on
    // different measurement methods, which would make comparing their
    // numbers meaningless.
    let Some(gpu) = gpu() else { return };
    let probe = Probe::new(&gpu.device, &gpu.queue, gpu.timestamps, RESOLUTION);

    let light_capacity = 64;
    let heavy_capacity = 2_000_000;

    let mut light = Points::new(&gpu.device, light_capacity, 19274);
    light.resize(RESOLUTION.0, RESOLUTION.1);
    light.prepare(&gpu.queue, 1);

    let mut heavy = Points::new(&gpu.device, heavy_capacity, 19274);
    heavy.resize(RESOLUTION.0, RESOLUTION.1);
    // Push point size to the top of the spec's declared range too, so the
    // heavy candidate is heavier both in instance count and in overdraw.
    heavy.params = Params {
        point_scale: 40.0,
        ..heavy.params
    };
    heavy.prepare(&gpu.queue, 1);

    let light_measurement = probe.run(&gpu.device, &gpu.queue, &mut light, 1, light_capacity);
    let heavy_measurement = probe.run(&gpu.device, &gpu.queue, &mut heavy, 1, heavy_capacity);

    eprintln!("measured via {:?}", light_measurement.method);
    assert_eq!(light_measurement.method, heavy_measurement.method);
    assert!(
        heavy_measurement.ms > light_measurement.ms * 2.0,
        "expected the heavy workload to measure unmistakably heavier: \
         light {} ms, heavy {} ms (via {:?})",
        light_measurement.ms,
        heavy_measurement.ms,
        light_measurement.method,
    );
}

#[test]
fn unavailable_timestamps_fall_back_to_a_labelled_host_measurement() {
    // The seam: pass `timestamps: false` directly rather than deriving it
    // from `Gpu::timestamps`, so this path is reachable regardless of what
    // the machine running the test actually supports. `Probe::new` never
    // fails — it degrades honestly instead, and the returned `Measurement`
    // says so via `method` rather than silently standing in a host number
    // for a GPU one.
    let Some(gpu) = gpu() else { return };

    let probe = Probe::new(&gpu.device, &gpu.queue, false, RESOLUTION);
    let capacity = 1024;
    let mut points = Points::new(&gpu.device, capacity, 19274);
    points.resize(RESOLUTION.0, RESOLUTION.1);
    points.prepare(&gpu.queue, 1);

    let measurement = probe.run(&gpu.device, &gpu.queue, &mut points, 1, capacity);

    assert_eq!(
        measurement.method,
        MeasurementMethod::HostWallClock,
        "no timestamp feature was offered, so this must be the labelled fallback, \
         never a Measurement dressed up as a GPU one"
    );
    assert!(measurement.ms.is_finite() && measurement.ms >= 0.0);
}
