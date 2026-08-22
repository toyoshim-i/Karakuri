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

// Every test here takes a device, so the whole file is one `mod gpu` — the
// prefix `cargo test -- --skip gpu::` filters on. The convention, and the test
// that enforces it, are in `tests/gpu_tests_are_under_mod_gpu.rs`.
mod gpu {
    use karakuri_engine::probe::MeasurementMethod;
    use karakuri_engine::{Gpu, Params, Points, Probe};

    const RESOLUTION: (u32, u32) = (256, 256);

    /// A device, or a failure that says so.
    ///
    /// **This used to skip.** No adapter at all is an environment fact rather
    /// than a defect in the probe, and printing and returning let a sandboxed
    /// machine get a run out of the rest of the file. What made that a bad
    /// trade is that the three tests below then reported *success* for having
    /// measured nothing — on the one machine where the answer mattered most.
    /// The whole file is now `mod gpu`, so a machine without an adapter says
    /// `--skip gpu::` and skips it out loud, at the runner, once; a test that
    /// does its own skipping says it into a log nobody reads.
    fn gpu() -> Gpu {
        Gpu::headless().expect("no GPU available")
    }

    #[test]
    fn a_trivial_measurement_reports_its_capacity_and_resolution() {
        let gpu = gpu();

        let mut probe = Probe::new(&gpu.device, &gpu.queue, gpu.timestamps, RESOLUTION);

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

    /// **This test caught a real defect, and it was not in this test.**
    ///
    /// It failed intermittently and resisted reproduction; the margin printed below
    /// was added so the next occurrence would be readable instead of a mystery.
    /// When it fired, it said this:
    ///
    /// ```text
    /// measured via GpuTimestamp: light 0.103 ms, heavy 0.095 ms, ratio 0.9x
    /// ```
    ///
    /// Two million points at point size 40, measured at a tenth of a millisecond,
    /// and **lighter than sixty-four points**. Every guess until then had been
    /// about the host clock and contention; the numbers said the host clock was
    /// not involved. On the runs that failed, the adapter advertised
    /// `TIMESTAMP_QUERY`, calibration passed, and the timestamps were meaningless
    /// — the abstract warning `README.md` carries, arriving.
    ///
    /// **The defect was the calibration's guard.** It was a constant floor of
    /// 0.1 ms against a workload costing tens of milliseconds, so a reading of
    /// 0.095 ms cleared it by five microseconds while measuring nothing. See
    /// `Probe::plausible`: the guard is now a ratio against what the host clock
    /// saw of the same submission, checked on every measurement rather than once
    /// at construction, and a probe that catches its adapter lying stops trusting
    /// it for good rather than for that reading.
    ///
    /// The threshold below was never moved and the comparison never reshaped. A
    /// test loosened to stop reporting this would have hidden it.
    ///
    /// Normal: light ≈ 1.3 ms, heavy ≈ 60 ms.
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
        let gpu = gpu();
        let mut probe = Probe::new(&gpu.device, &gpu.queue, gpu.timestamps, RESOLUTION);

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

        // Every run, not only failing ones — a number nobody records is a number
        // nobody can compare the next failure against. See this test's doc.
        eprintln!(
            "measured via {:?}: light {:.3} ms, heavy {:.3} ms, ratio {:.1}x",
            light_measurement.method,
            light_measurement.ms,
            heavy_measurement.ms,
            heavy_measurement.ms / light_measurement.ms,
        );
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
        let gpu = gpu();

        let mut probe = Probe::new(&gpu.device, &gpu.queue, false, RESOLUTION);
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
}
