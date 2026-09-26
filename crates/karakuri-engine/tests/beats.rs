//! Integration tests verifying that the tempo grid `beats` ambient is readable
//! from IR, advances per substep, matches `t` at 60 bpm, and remains continuous
//! across tempo corrections.

// Every test here takes a device, so the whole file is one `mod gpu` — the
// prefix `cargo test -- --skip gpu::` filters on. The convention, and the test
// that enforces it, are in `tests/gpu_tests_are_under_mod_gpu.rs`.
#[path = "common/mod.rs"]
mod common;

mod gpu {
    pub(super) use super::common::compile;
    use karakuri_engine::{Gpu, Present, Set, Signals, VideoSource};

    const WIDTH: u32 = 128;
    const HEIGHT: u32 = 128;
    const CAPACITY: u32 = 1024;
    const SEED: u32 = 19274;
    const DT: f32 = 1.0 / 60.0;

    /// Ring geometry with angular position driven by `beats` for grid tracking tests.
    const RING: &str = r#"
proc beat_ring {
  kind     L1
  topology points
  capacity [1024, 262144] = 1024

  emit position, age

  element {
    let a = beats * 6.2831853 + hash1(seed) * 1.2;
    let r = 1.2 + hash1(seed + 7u) * 0.8;
    position = vec3(cos(a) * r, sin(a) * r, 0.0);
    age      = age + dt;
  }
}
"#;

    /// Probe geometry recording instantaneous clock (`position.x`) and accumulated time (`age`).
    const CLOCK_PROBE: &str = r#"
proc clock_probe {
  kind     L1
  topology points
  capacity [1024, 262144] = 1024

  emit position, age

  element {
    position = vec3(t, 0.0, 0.0);
    age      = age + t;
  }
}
"#;

    /// The same ring driven by `t` instead. At 60 bpm one beat is one second, so
    /// the grid and the clock are the same number and these two must draw the same
    /// frame — see `beats_and_t_name_the_same_instant`.
    const RING_ON_T: &str = r#"
proc t_ring {
  kind     L1
  topology points
  capacity [1024, 262144] = 1024

  emit position, age

  element {
    let a = t * 6.2831853 + hash1(seed) * 1.2;
    let r = 1.2 + hash1(seed + 7u) * 0.8;
    position = vec3(cos(a) * r, sin(a) * r, 0.0);
    age      = age + dt;
  }
}
"#;

    /// Accumulating geometry driven by `beats`. The running sum records every
    /// intermediate evaluation, enabling detection of per-substep advances.
    const ACCUMULATE: &str = r#"
proc accumulate_beats {
  kind     L1
  topology points
  capacity [1024, 262144] = 1024

  emit position, age

  element {
    let spread = vec3(hash1(seed) - 0.5, hash1(seed + 3u) - 0.5, 0.0) * 4.0;
    position = position + (vec3(beats, 0.5, 0.0 - beats) * 4.0 + spread) * dt;
    age      = age + dt;
  }
}
"#;

    const L4: &str = r#"
proc plain_points {
  kind  L4
  blend additive

  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_rate = 0.03125;
  }

  fragment {
    let d = length(point_coord * 2.0 - 1.0);
    color = vec4(1.0, 0.9, 0.7, max(0.0, 1.0 - d));
  }
}
"#;

    fn build(gpu: &Gpu, l1: &str) -> Set {
        let mut set = Set::build(
            &gpu.device,
            &gpu.queue,
            &compile(l1),
            &compile(L4),
            CAPACITY,
            SEED,
        )
        .expect("the pair is compatible and the capacity is in range");
        set.resize(&gpu.device, WIDTH, HEIGHT);
        set
    }

    /// Verifies session clock advances prior to Set advancement matching `Frame::render`.
    /// that skipped that would leave the grid at zero and every test here would
    /// pass against a `beats` that never moved.
    fn frame(gpu: &Gpu, set: &mut Set, signals: &mut Signals, steps: u8) -> Vec<u16> {
        signals.advance(steps, DT);
        set.prepare(&gpu.queue, steps, signals);

        let present = Present::new(&gpu.device, wgpu::TextureFormat::Rgba16Float, WIDTH, HEIGHT);
        let bytes_per_row = WIDTH * 8;
        let readback = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("beats readback"),
            size: u64::from(bytes_per_row * HEIGHT),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        set.render(&mut encoder, present.hdr_view(), steps);
        encoder.copy_texture_to_buffer(
            present.hdr_texture().as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(HEIGHT),
                },
            },
            wgpu::Extent3d {
                width: WIDTH,
                height: HEIGHT,
                depth_or_array_layers: 1,
            },
        );
        gpu.queue.submit([encoder.finish()]);
        set.commit();

        let slice = readback.slice(..);
        slice.map_async(wgpu::MapMode::Read, |r| r.expect("map"));
        gpu.device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("poll");
        let data = slice.get_mapped_range().expect("map");
        let out = data
            .chunks_exact(2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .collect();
        drop(data);
        readback.unmap();
        out
    }

    fn lit(pixels: &[u16]) -> usize {
        pixels
            .chunks_exact(4)
            .filter(|p| p[0] != 0 || p[1] != 0 || p[2] != 0)
            .count()
    }

    // ---------------------------------------------------------------------------

    /// Verifies that procedure output tracks changes in session tempo.
    #[test]
    fn the_picture_follows_the_tempo() {
        let gpu = Gpu::headless().expect("no GPU available");
        const FRAMES: usize = 60;

        let run = |bpm: f32| -> Vec<u16> {
            let mut set = build(&gpu, RING);
            let mut signals = Signals::new(bpm, u64::from(SEED));
            let mut last = Vec::new();
            for _ in 0..FRAMES {
                last = frame(&gpu, &mut set, &mut signals, 1);
            }
            // The two runs reached the same simulation time by the same route, so
            // anything that differs below differs because of the grid.
            assert!((set.time() - FRAMES as f32 * DT).abs() < 1e-6);
            last
        };

        let (slow, fast) = (run(120.0), run(150.0));
        assert!(
            lit(&slow) > 50,
            "the material drew nothing, so this comparison is two black frames"
        );
        assert_ne!(
            slow, fast,
            "the tempo changed and the picture did not — nothing readable from IR carries it"
        );
    }

    /// Verifies that at 60 bpm, `beats` and `t` evaluate to the identical instant.
    #[test]
    fn beats_and_t_name_the_same_instant() {
        let gpu = Gpu::headless().expect("no GPU available");
        const FRAMES: usize = 40;

        let run = |src: &str| -> Vec<u16> {
            let mut set = build(&gpu, src);
            // Sixty beats a minute: one beat, one second.
            let mut signals = Signals::new(60.0, u64::from(SEED));
            let mut last = Vec::new();
            for _ in 0..FRAMES {
                last = frame(&gpu, &mut set, &mut signals, 1);
            }
            last
        };

        let (on_beats, on_t) = (run(RING), run(RING_ON_T));
        assert!(lit(&on_beats) > 50, "the material drew nothing");
        assert_eq!(
            on_beats, on_t,
            "at 60 bpm `beats` and `t` are the same number and did not draw the same frame"
        );
    }

    /// Verifies that substeps evaluate against the Set's clock in exact arithmetic.
    #[test]
    fn the_instants_a_substep_reads_are_the_sets_own_clock() {
        let gpu = Gpu::headless().expect("no GPU available");
        // Deliberately uneven, and deliberately not starting at 1: a frame of four
        // has to land on four consecutive instants, and the frame after it has to
        // resume from the next one rather than from wherever it likes.
        const FRAMES: [u8; 5] = [1, 4, 2, 1, 3];

        let mut set = build(&gpu, CLOCK_PROBE);
        let mut signals = Signals::new(120.0, u64::from(SEED));
        // Running f32 accumulator in substep order matching engine precision.
        let (mut n, mut expected_age) = (0u64, 0.0f32);
        for steps in FRAMES {
            let _ = frame(&gpu, &mut set, &mut signals, steps);
            for _ in 0..steps {
                n += 1;
                expected_age += n as f32 * DT;
            }
        }

        let bytes = set.read_elements(&gpu.device, &gpu.queue);
        let layout = set.element_layout();
        let stride = layout.stride as usize;
        let at = |off: u32, i: usize| -> f32 {
            let a = i * stride + off as usize;
            f32::from_le_bytes(bytes[a..a + 4].try_into().expect("four bytes"))
        };
        let (pos, age) = (layout.offset_of("position"), layout.offset_of("age"));

        let last = n as f32 * DT;
        assert_eq!(set.time(), last, "the Set's own clock is not `steps * dt`");
        for i in 0..CAPACITY as usize {
            assert_eq!(
                at(pos, i),
                last,
                "element {i} read {} as the last substep's instant, not {last}",
                at(pos, i)
            );
            assert_eq!(
                at(age, i),
                expected_age,
                "element {i} accumulated {} over the substeps, not {expected_age}",
                at(age, i)
            );
        }
    }

    /// Verifies that `beats` advances per substep rather than per frame.
    #[test]
    fn an_accumulating_procedure_reading_beats_is_substep_invariant() {
        let gpu = Gpu::headless().expect("no GPU available");

        let run = |steps: u8, frames: usize| -> Vec<u16> {
            let mut set = build(&gpu, ACCUMULATE);
            let mut signals = Signals::new(128.0, u64::from(SEED));
            let mut last = Vec::new();
            for _ in 0..frames {
                last = frame(&gpu, &mut set, &mut signals, steps);
            }
            assert_eq!(
                (set.time() * 60.0).round() as u64,
                (steps as usize * frames) as u64
            );
            last
        };

        let (split, merged) = (run(1, 21), run(3, 7));
        assert!(lit(&split) > 50, "the material drew nothing");
        assert_eq!(
            split, merged,
            "a frame rate drop moved the beat: `beats` is advancing per frame, not per substep"
        );
    }

    /// Verifies that tempo corrections bend the rate continuously without jumping the beat grid.
    #[test]
    fn a_tempo_correction_does_not_jump_the_grid() {
        let gpu = Gpu::headless().expect("no GPU available");

        let mut set = build(&gpu, RING);
        let mut signals = Signals::new(120.0, u64::from(SEED));
        for _ in 0..30 {
            frame(&gpu, &mut set, &mut signals, 1);
        }
        let before = frame(&gpu, &mut set, &mut signals, 1);
        let beats_before = signals.oscillator().beats();

        // A tempo change with no phase shift, which is what a trim mostly is.
        signals.correct(126.0, 0.0);
        assert!(
            (signals.oscillator().beats() - beats_before).abs() < 1e-9,
            "the correction moved the current beat rather than the rate"
        );

        let after = frame(&gpu, &mut set, &mut signals, 1);
        assert_ne!(before, after, "nothing advanced at all");

        // One frame at 126 bpm is 0.035 of a beat, so the ring turned by that much
        // and no more. Compared against a whole extra turn — the shape a
        // discontinuity would take — by asking that most pixels agree.
        let same = before.iter().zip(&after).filter(|(a, b)| a == b).count();
        assert!(
            same * 100 / before.len() > 90,
            "the frame after a tempo correction shares only {}% of its pixels with the one \
         before: the grid jumped rather than bent",
            same * 100 / before.len()
        );
    }
}
