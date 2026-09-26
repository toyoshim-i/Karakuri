//! Integration tests for GPU timing probes, timestamp query verification, and host fallbacks.

// Every test here takes a device, so the whole file is one `mod gpu` — the
// prefix `cargo test -- --skip gpu::` filters on. The convention, and the test
// that enforces it, are in `tests/gpu_tests_are_under_mod_gpu.rs`.
mod gpu {
    use karakuri_engine::probe::MeasurementMethod;
    use karakuri_engine::{Gpu, Params, Points, Probe};

    const RESOLUTION: (u32, u32) = (256, 256);

    /// Returns a headless GPU instance or panics if unavailable.
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

    /// Verifies that significantly heavier workloads produce proportionately larger cost measurements.
    #[test]
    fn a_heavier_workload_measures_as_heavier() {
        // Crank instance count and point scale across orders of magnitude to verify cost tracking.
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
        // Force timestamps: false to verify degradation to HostWallClock measurement.
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

#[test]
fn degeneracy_detector_identifies_all_zero_alpha() {
    use karakuri_engine::{check_degeneracy, Degeneracy};

    // Rgba8Unorm with zero alpha
    let data_rgba8 = vec![255, 128, 64, 0, 100, 200, 50, 0];
    assert_eq!(
        check_degeneracy(&data_rgba8, wgpu::TextureFormat::Rgba8Unorm),
        Some(Degeneracy::AllZeroAlpha)
    );

    // Rgba16Float with zero alpha (1.0f in f16 is 0x3C00, 0.0f in f16 is 0x0000)
    let mut data_f16 = Vec::new();
    for _ in 0..4 {
        data_f16.extend_from_slice(&0x3C00u16.to_le_bytes()); // R = 1.0
        data_f16.extend_from_slice(&0x3C00u16.to_le_bytes()); // G = 1.0
        data_f16.extend_from_slice(&0x3C00u16.to_le_bytes()); // B = 1.0
        data_f16.extend_from_slice(&0x0000u16.to_le_bytes()); // A = 0.0
    }
    assert_eq!(
        check_degeneracy(&data_f16, wgpu::TextureFormat::Rgba16Float),
        Some(Degeneracy::AllZeroAlpha)
    );
}

#[test]
fn degeneracy_detector_identifies_pure_black() {
    use karakuri_engine::{check_degeneracy, Degeneracy};

    // Rgba8Unorm with RGB == 0 and Alpha == 255
    let data_rgba8 = vec![0, 0, 0, 255, 0, 0, 0, 255];
    assert_eq!(
        check_degeneracy(&data_rgba8, wgpu::TextureFormat::Rgba8Unorm),
        Some(Degeneracy::PureBlack)
    );

    // Rgba16Float with RGB == 0.0 and Alpha == 1.0
    let mut data_f16 = Vec::new();
    for _ in 0..4 {
        data_f16.extend_from_slice(&0x0000u16.to_le_bytes()); // R = 0.0
        data_f16.extend_from_slice(&0x0000u16.to_le_bytes()); // G = 0.0
        data_f16.extend_from_slice(&0x0000u16.to_le_bytes()); // B = 0.0
        data_f16.extend_from_slice(&0x3C00u16.to_le_bytes()); // A = 1.0
    }
    assert_eq!(
        check_degeneracy(&data_f16, wgpu::TextureFormat::Rgba16Float),
        Some(Degeneracy::PureBlack)
    );
}

#[test]
fn degeneracy_detector_identifies_nan_and_inf() {
    use karakuri_engine::{check_degeneracy, Degeneracy};

    // Half-float NaN: exponent all 1s (0x7C00), mantissa nonzero (e.g. 0x7E00)
    let mut data_f16_nan = Vec::new();
    data_f16_nan.extend_from_slice(&0x7E00u16.to_le_bytes()); // R = NaN
    data_f16_nan.extend_from_slice(&0x3C00u16.to_le_bytes()); // G = 1.0
    data_f16_nan.extend_from_slice(&0x3C00u16.to_le_bytes()); // B = 1.0
    data_f16_nan.extend_from_slice(&0x3C00u16.to_le_bytes()); // A = 1.0
    assert_eq!(
        check_degeneracy(&data_f16_nan, wgpu::TextureFormat::Rgba16Float),
        Some(Degeneracy::NaNDetected)
    );

    // Half-float Inf: exponent all 1s (0x7C00), mantissa zero (0x7C00)
    let mut data_f16_inf = Vec::new();
    data_f16_inf.extend_from_slice(&0x3C00u16.to_le_bytes()); // R = 1.0
    data_f16_inf.extend_from_slice(&0x7C00u16.to_le_bytes()); // G = Inf
    data_f16_inf.extend_from_slice(&0x3C00u16.to_le_bytes()); // B = 1.0
    data_f16_inf.extend_from_slice(&0x3C00u16.to_le_bytes()); // A = 1.0
    assert_eq!(
        check_degeneracy(&data_f16_inf, wgpu::TextureFormat::Rgba16Float),
        Some(Degeneracy::NaNDetected)
    );

    // Rgba32Float NaN
    let mut data_f32 = Vec::new();
    data_f32.extend_from_slice(&f32::NAN.to_le_bytes());
    data_f32.extend_from_slice(&1.0f32.to_le_bytes());
    data_f32.extend_from_slice(&1.0f32.to_le_bytes());
    data_f32.extend_from_slice(&1.0f32.to_le_bytes());
    assert_eq!(
        check_degeneracy(&data_f32, wgpu::TextureFormat::Rgba32Float),
        Some(Degeneracy::NaNDetected)
    );
}

#[test]
fn degeneracy_detector_healthy_frame_returns_none() {
    use karakuri_engine::check_degeneracy;

    let data_rgba8 = vec![255, 128, 64, 255, 100, 200, 50, 255];
    assert_eq!(
        check_degeneracy(&data_rgba8, wgpu::TextureFormat::Rgba8Unorm),
        None
    );

    let mut data_f16 = Vec::new();
    for _ in 0..4 {
        data_f16.extend_from_slice(&0x3C00u16.to_le_bytes()); // R = 1.0
        data_f16.extend_from_slice(&0x3C00u16.to_le_bytes()); // G = 1.0
        data_f16.extend_from_slice(&0x3C00u16.to_le_bytes()); // B = 1.0
        data_f16.extend_from_slice(&0x3C00u16.to_le_bytes()); // A = 1.0
    }
    assert_eq!(
        check_degeneracy(&data_f16, wgpu::TextureFormat::Rgba16Float),
        None
    );
}
