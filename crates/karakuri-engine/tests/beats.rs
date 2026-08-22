//! `beats`: the session's tempo grid, readable from IR.
//!
//! Without it a procedure has `t` and nothing else, so **the picture runs at
//! wall time whatever the music does** — a tempo change moves the beat grid
//! every binding is sampled on and moves nothing that is drawn. That is the
//! gap this closes, and the first test below is the whole of it.
//!
//! The rest is what makes it usable rather than merely present:
//!
//! - `beats` and `t` name the **same instant**, exactly, so a procedure reading
//!   both is not reading two clocks;
//! - it is **per substep**, like `t`, so a frame rate drop does not move the
//!   beat;
//! - and it is **continuous across a tempo correction**, so following the room
//!   does not mean jumping whenever the tracker trims.
//!
//! The material is deliberately minimal. What is under test is a number
//! reaching a shader and being the right number; anything that looks good
//! would only make a failure harder to read.

// Every test here takes a device, so the whole file is one `mod gpu` — the
// prefix `cargo test -- --skip gpu::` filters on. The convention, and the test
// that enforces it, are in `tests/gpu_tests_are_under_mod_gpu.rs`.
mod gpu {
    use karakuri_engine::{Gpu, Present, Set, Signals, VideoSource};
    use karakuri_ir::typed::Checked;

    const WIDTH: u32 = 128;
    const HEIGHT: u32 = 128;
    const CAPACITY: u32 = 1024;
    const SEED: u32 = 19274;
    const DT: f32 = 1.0 / 60.0;

    /// Points on a ring whose angle is `beats`, so one whole turn is one beat and
    /// the frame says directly where on the grid the session is.
    ///
    /// No `spawn` block, so every element is live from frame zero and nothing here
    /// depends on compaction. `position` is written outright rather than
    /// accumulated: this is the closed-form shape, which is what the transport will
    /// want and what makes a difference between two runs a difference in `beats`
    /// and not in their histories.
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

    /// Neither a ring nor anything to look at: two readings of the clock, in the
    /// two shapes that pin different things. `position.x` is written outright, so it
    /// keeps only the last substep's instant; `age` accumulates, so it keeps the sum
    /// over every substep that has ever run. See
    /// `the_instants_a_substep_reads_are_the_sets_own_clock`.
    ///
    /// No `spawn` block and no `kill()`, so every slot is live from frame zero and
    /// stays in it — the values can be read back by index with no compaction between
    /// this and them.
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

    /// Accumulating, and the term accumulated depends on `beats`, so the sum
    /// records every instant it was evaluated at rather than only the last one.
    /// That is what makes it able to detect a `beats` that advances once per frame
    /// instead of once per substep — the closed-form ring above cannot, because an
    /// intermediate value is overwritten.
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
    point_size = 4.0;
  }

  fragment {
    let d = length(point_coord * 2.0 - 1.0);
    color = vec4(1.0, 0.9, 0.7, max(0.0, 1.0 - d));
  }
}
"#;

    fn compile(src: &str) -> Checked {
        let proc = karakuri_ir::parse(src).unwrap_or_else(|e| panic!("{}", render(&e, src)));
        let checked =
            karakuri_ir::check::check(&proc).unwrap_or_else(|e| panic!("{}", render(&e, src)));
        karakuri_ir::cost::estimate(&checked).unwrap_or_else(|e| panic!("{}", render(&e, src)));
        checked
    }

    fn render(errs: &[karakuri_ir::IrError], src: &str) -> String {
        errs.iter()
            .map(|e| e.render(src))
            .collect::<Vec<_>>()
            .join("\n")
    }

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

    /// One frame, driven the way `Frame::render` drives one: **the session clock
    /// advances first, by exactly what the Set is about to advance by.** A helper
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

        let slice = readback.slice(..);
        slice.map_async(wgpu::MapMode::Read, |r| r.expect("map"));
        gpu.device.poll(wgpu::PollType::Wait).expect("poll");
        let data = slice.get_mapped_range();
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

    /// **The picture follows the tempo**, which is the whole reason this ambient
    /// exists.
    ///
    /// The same procedure, the same seed, the same number of frames, and the same
    /// `t` at the end of them — only the session's tempo differs. Without `beats`
    /// there is no number a procedure can read that carries a tempo at all, so the
    /// two runs would be identical and the music could do what it liked.
    ///
    /// **The tempos are chosen so the two runs do not land on the same angle**, and
    /// that is not a detail. The ring is closed form: `position` is written outright
    /// every step, so only the final instant reaches the buffer and every earlier
    /// one is overwritten. 120 and 180 bpm over one second are two whole turns and
    /// three — indistinguishable at the end of it, and a test that picked them would
    /// report that the tempo does nothing. 120 against 150 is two turns against two
    /// and a half, which is half a turn apart.
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

    /// **`beats` and `t` name the same instant.**
    ///
    /// At 60 bpm a beat is a second, so the grid and the clock are numerically the
    /// same and two procedures differing only in which they read must draw the same
    /// frame — bit for bit, not nearly.
    ///
    /// That exactness is the point and it is not free. The session oscillator
    /// accumulates its `t` in f64 while a Set derives its own as an f32 product of
    /// a step count; the two agree to about seven digits. `beats` is therefore
    /// computed from the Set's `t` rather than from the oscillator's position, so
    /// the two ambients cannot drift apart no matter how long the session runs.
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

    /// **The anchor: the instants a substep actually reads are the Set's own
    /// clock**, as absolute numbers rather than as a comparison between two runs.
    ///
    /// Every other test in this file — and in `generated.rs`, `lifecycle.rs` and
    /// `priming.rs` — compares one run against another. That is the right shape for
    /// invariance claims and it is blind to one whole class of defect: shift *every*
    /// substep's `t` by a whole `dt` and both sides of every comparison shift with
    /// it, so all 43 suites stay green while the clock a procedure reads is off by a
    /// frame. Found exactly that way, by injecting the shift into whichever function
    /// derives the per-substep instant — `Set::write_step_args` when this was
    /// written, `Set::prepare_on` since the node split — and watching nothing fail.
    ///
    /// So this asserts against arithmetic instead. `position.x` is written outright,
    /// so it holds the **last** substep's instant and must equal [`Set::time`].
    /// `age` accumulates `t`, so it holds the **sum over every substep since frame
    /// zero** and must equal `dt * (1 + 2 + ... + n)` — which pins where the
    /// sequence starts as well as where it ends, and pins it across a frame of four
    /// steps as well as a frame of one.
    ///
    /// The two halves are not redundant *arithmetically*: a defect giving every
    /// substep the frame's final instant leaves `position.x` correct and moves `age`
    /// from `66·dt` to `76·dt`. They are partly redundant in *practice*, because
    /// freezing the instant freezes `beats` with it and
    /// `an_accumulating_procedure_reading_beats_is_substep_invariant` catches that
    /// too — which is worth knowing rather than worth removing: that test would stop
    /// covering it the moment `beats` stopped being derived from this `t`.
    #[test]
    fn the_instants_a_substep_reads_are_the_sets_own_clock() {
        let gpu = Gpu::headless().expect("no GPU available");
        // Deliberately uneven, and deliberately not starting at 1: a frame of four
        // has to land on four consecutive instants, and the frame after it has to
        // resume from the next one rather than from wherever it likes.
        const FRAMES: [u8; 5] = [1, 4, 2, 1, 3];

        let mut set = build(&gpu, CLOCK_PROBE);
        let mut signals = Signals::new(120.0, u64::from(SEED));
        // The same arithmetic the engine does, in the same order and the same
        // precision: `t_at(n)` is `n as f32 * dt`, and the accumulator is a running
        // f32 sum in substep order. A reference computed in f64 would differ in the
        // last bits and the test would need a tolerance wide enough to hide a real
        // defect.
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

    /// **Per substep, like `t`.**
    ///
    /// Twenty-one steps reached two ways: twenty-one frames of one, and seven of
    /// three. A `beats` advanced once per frame would give the three-step frames
    /// one musical instant where they should have three, and an accumulating
    /// procedure records the difference — the ring above would not, because it
    /// overwrites `position` and only the last instant survives.
    ///
    /// This is the same shape as `generated.rs`'s test for `t`, and for the same
    /// reason: a frame rate drop must not move the beat.
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

    /// **A tempo correction bends the rate and does not move a beat that has
    /// already happened.**
    ///
    /// The oscillator guarantees that and `oscillator.rs` asserts it directly; what
    /// this checks is that the guarantee survives the trip into a shader, because
    /// the alternative is a picture that jumps every time the tracker trims. A trim
    /// is a few thousandths of a beat and arrives several times a second — a
    /// `beats` that recomputed history on every one would shimmer.
    ///
    /// Continuity is asserted where it can be seen: the frame straight after the
    /// correction differs from the one before it by about what one frame of
    /// movement is, rather than by a whole turn of the ring.
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
