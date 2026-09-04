//! The deck: several Sets resident, one to four composited.
//!
//! What is asserted here is what the deck promises and what the
//! rest of the system is built on top of, in the order the module doc on
//! `deck.rs` argues them:
//!
//! - a deck of one is a bare Set, **bit for bit**, so every single-Set
//!   expectation elsewhere in this repository still means something;
//! - gain is linear, per slot, and applied to that slot's own target before
//!   the sum rather than to the sum;
//! - `over` hides what is under it and `add` does not, which is the whole of
//!   what a blend mode buys, and `max` stacks without summing;
//! - the fader silences a slot under every mode and the level does not, which
//!   is the asymmetry `Blend::silent_at` records;
//! - `Allocated` keeps its state — a slot taken off air does not advance and
//!   resumes where it stopped;
//! - the same ticks and the same seeds composite to the same pixels;
//! - a hot swap in one slot is invisible to every other slot.
//!
//! The frame guard is asserted where it lives: a `compile_fail` doc test on
//! `Deck::begin_frame`, because a claim about what the borrow checker refuses
//! is worth exactly what a compiler says about it and nothing that can be
//! written here is the same claim.
//!
//! Everything is a pixel comparison against a readback of the mix, and most of
//! them are exact. That is deliberate: `Rgba16Float` in and `Rgba16Float` out
//! at unity gain and full opacity has no rounding in it anywhere, so "close enough"
//! would be hiding a real defect rather than tolerating a real error. The one
//! test that cannot be exact says why.

// Every test here takes a device, so the whole file is one `mod gpu` — the
// prefix `cargo test -- --skip gpu::` filters on. The convention, and the test
// that enforces it, are in `tests/gpu_tests_are_under_mod_gpu.rs`.
mod gpu {
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    use karakuri_engine::binding::Curve;
    use karakuri_engine::deck::{Blend, Deck, Mask, MaskKind, Residency};
    use karakuri_engine::swap::{Event, HotSwap, Request};
    use karakuri_engine::transition::{quantise, Control, Selection, Transition};
    use karakuri_engine::{Gpu, Present, Set, Signals, VideoSource};
    use karakuri_ir::typed::Checked;

    const WIDTH: u32 = 256;
    const HEIGHT: u32 = 256;
    const CAPACITY: u32 = 4096;

    /// Two seeds, so that two slots hold visibly different material and a mix of
    /// them is not the same picture twice.
    const SEED_A: u32 = 19274;
    const SEED_B: u32 = 88888;

    /// The capacity a swapped-in Set is built at, differing from [`CAPACITY`] so
    /// that which Set is live in a slot is observable from outside.
    const SWAPPED: u32 = 8192;

    /// A budget no frame in these tests will come near: they are about the deck,
    /// not about the watchdog, and a rollback firing in the middle of one would
    /// be measuring the host's mood.
    const GENEROUS_MS: f32 = 10_000.0;

    const PATIENCE: Duration = Duration::from_secs(30);

    const L1: &str = r#"
proc static_shell {
  kind     L1
  topology points
  capacity [1024, 262144] = 4096

  param radius : float [0.1, 8.0] = 2.5

  emit position, age

  element {
    let u = hash1(seed);
    let v = hash1(seed + 1000u);
    position = rot_y(sphere_point(u, v) * radius, t * 0.3);
    age      = age + dt;
  }
}
"#;

    const L4: &str = r#"
proc soft_points {
  kind  L4
  blend additive

  consumes position

  param exposure : float [0.0, 8.0] = 1.0

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_rate = 0.015625;
  }

  fragment {
    let d = length(point_coord * 2.0 - 1.0);
    color = vec4(vec3(1.0, 1.0, 1.0) * exposure, max(0.0, 1.0 - d));
  }
}
"#;

    /// A second renderer over the same geometry, differing only in its name.
    ///
    /// Which is the whole point: two ways of drawing one simulation is what a
    /// Set holds several L4s *for*, and what selecting between them is about.
    /// A name of its own because a name addresses a node and two nodes cannot
    /// share one.
    const L4_B: &str = r#"
proc harder_points {
  kind  L4
  blend additive

  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_rate = 0.0078125;
  }

  fragment {
    color = vec4(1.0, 0.5, 0.25, 1.0);
  }
}
"#;

    /// The same shape, rendering a NaN. `sqrt` of a negative is a procedure that
    /// parses, type-checks, costs, compiles and runs — nothing in the pipeline
    /// rejects it, and a generated L4 reaches this by dividing by a parameter or
    /// normalizing a zero vector as easily as by this. What a slot holding one
    /// must not be able to do is reach a mix it was faded out of.
    const L4_NAN: &str = r#"
proc nan_points {
  kind  L4
  blend additive

  consumes position

  param exposure : float [0.0, 8.0] = 1.0

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_rate = 0.015625;
  }

  fragment {
    let d   = length(point_coord * 2.0 - 1.0);
    let bad = sqrt(0.0 - exposure);
    color = vec4(vec3(bad, bad, bad), max(0.0, 1.0 - d));
  }
}
"#;

    /// **A layer that covers**: big black sprites at full coverage.
    ///
    /// Black *and* opaque is the pair that separates the modes with nothing else
    /// moving. Its colour contribution is zero under every mode — `blend additive`
    /// multiplies colour by the sprite's own alpha on the way into the slot target,
    /// and zero times anything is zero — so whatever the mix does with this layer
    /// is entirely what it did with the coverage. Under `add` it is invisible;
    /// under `over` it is a hole.
    const L4_CARD: &str = r#"
proc opaque_card {
  kind  L4
  blend additive

  consumes position

  param exposure : float [0.0, 8.0] = 1.0

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_rate = 0.1875;
  }

  fragment {
    color = vec4(vec3(0.0, 0.0, 0.0) * exposure, 1.0);
  }
}
"#;

    /// The ordinary material, drawn wide enough to be under the card everywhere it
    /// covers. Used where a test needs the two layers to actually overlap rather
    /// than to overlap wherever the seeds happened to put them.
    const L4_WIDE: &str = r#"
proc wide_points {
  kind  L4
  blend additive

  consumes position

  param exposure : float [0.0, 8.0] = 1.0

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_rate = 0.1875;
  }

  fragment {
    let d = length(point_coord * 2.0 - 1.0);
    color = vec4(vec3(1.0, 1.0, 1.0) * exposure, max(0.0, 1.0 - d));
  }
}
"#;

    /// The same card, writing a coverage that is **not a coverage** — one per
    /// spelling of "outside `[0, 1]`", named by what the alpha expression is.
    ///
    /// Nothing in this pipeline stops any of them: the IR calls `color` linear RGB
    /// with straight alpha, says values above 1.0 are expected, and no pass clamps
    /// what a `fragment` block assigns. Each parses, type-checks, costs, compiles
    /// and runs, and a generated L4 reaches all four by dividing by a parameter as
    /// easily as by writing the constant. Black, so that whatever the mix does with
    /// one of them is what it did with the coverage and not with the colour.
    ///
    /// `exposure` defaults to 1.0 and every expression is written against it, so
    /// none of them folds to a constant the compiler could reject before it runs.
    const OVERDRAWN_ALPHA: [(&str, &str); 4] = [
        ("above one", "1.5 * exposure"),
        ("negative", "0.0 - exposure"),
        ("infinite", "exposure / 0.0"),
        ("NaN", "sqrt(0.0 - exposure)"),
    ];

    fn overdrawn_card(alpha: &str) -> String {
        format!(
            r#"
proc overdrawn_card {{
  kind  L4
  blend additive

  consumes position

  param exposure : float [0.0, 8.0] = 1.0

  vertex {{
    clip       = camera * vec4(position, 1.0);
    point_rate = 0.1875;
  }}

  fragment {{
    color = vec4(vec3(0.0, 0.0, 0.0) * exposure, {alpha});
  }}
}}
"#
        )
    }

    /// **A wash over the whole frame**, for the mask tests.
    ///
    /// Every other fixture here draws a sphere in the middle, and a mask's ends are
    /// at the *edges*: a front that stops short of the corner, or a radial one that
    /// stops at the inscribed circle, is invisible against material that never
    /// reaches either. Four such defects walked past the first version of those
    /// tests for exactly that reason.
    ///
    /// The vertex stage ignores `position` and puts every sprite at the origin, so
    /// one of them covers a target of [`MASK_SIZE`]. It still `consumes position`,
    /// because an L4 is compiled against its L1's element layout and dropping the
    /// attribute would change what is being tested.
    const L4_WASH: &str = r#"
proc wash {
  kind  L4
  blend additive

  consumes position

  param exposure : float [0.0, 8.0] = 1.0

  vertex {
    let ignored = position;
    clip       = vec4(0.0, 0.0, 0.0, 1.0);
    // Twice the frame's height, so one sprite covers a square target with
    // margin to spare. `point_rate` is a fraction of the height, so this is
    // the one fixture here whose number does not depend on [`MASK_SIZE`].
    point_rate = 2.0;
  }

  fragment {
    color = vec4(vec3(1.0, 1.0, 1.0) * exposure, 1.0);
  }
}
"#;

    /// The target the mask tests render at. Small, because [`L4_WASH`] overdraws
    /// the whole frame once per element and the number that matters is coverage
    /// rather than resolution.
    const MASK_SIZE: u32 = 64;

    /// A deck whose slot 1 covers the frame, at [`MASK_SIZE`].
    fn wash_deck(gpu: &Gpu) -> Deck {
        let swaps = vec![
            HotSwap::fixed(build(gpu, SEED_A, CAPACITY)),
            HotSwap::fixed(build_with(gpu, L4_WASH, SEED_B, CAPACITY)),
        ];
        let mut deck = Deck::new(&gpu.device, swaps, MASK_SIZE, MASK_SIZE);
        deck.resize(&gpu.device, MASK_SIZE, MASK_SIZE);
        deck
    }

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

    fn build(gpu: &Gpu, seed: u32, capacity: u32) -> Set {
        build_with(gpu, L4, seed, capacity)
    }

    fn build_with(gpu: &Gpu, l4: &str, seed: u32, capacity: u32) -> Set {
        let mut set = Set::build(
            &gpu.device,
            &gpu.queue,
            &compile(L1),
            &compile(l4),
            capacity,
            seed,
        )
        .expect("the pair is compatible and the capacity is in range");
        set.resize(&gpu.device, WIDTH, HEIGHT);
        set
    }

    /// A deck of fixed Sets — no worker, so nothing can ever swap and the run is a
    /// pure function of the ticks it is given. Every test but the last one uses
    /// this, because a build landing halfway through would be a second variable.
    fn deck_of(gpu: &Gpu, seeds: &[u32]) -> Deck {
        deck_of_at(gpu, seeds, WIDTH, HEIGHT)
    }

    fn deck_of_at(gpu: &Gpu, seeds: &[u32], width: u32, height: u32) -> Deck {
        let swaps = seeds
            .iter()
            .map(|&seed| HotSwap::fixed(build(gpu, seed, CAPACITY)))
            .collect();
        Deck::new(&gpu.device, swaps, width, height)
    }

    /// One frame, shaped the way a caller has to shape it: the guard owns the
    /// encoder, so there is no other shape available.
    ///
    /// The `poll` afterwards is the harness standing in for vsync, exactly as in
    /// `tests/hot_swap.rs`: it bounds a headless loop that would otherwise queue
    /// thousands of command buffers ahead of the GPU. It is the submit-and-wait
    /// the render thread must never do, and it is not inside the frame.
    fn frame(gpu: &Gpu, deck: &mut Deck, present: &Present, steps: u8) {
        let mut f = deck.begin_frame(&gpu.device, &gpu.queue);
        f.render(present.hdr_view(), present.size(), steps);
        f.finish();
        gpu.device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("poll");
    }

    /// One frame of a bare Set, the way `karakuri-cli` and every earlier test
    /// record one. The comparison in
    /// [`a_deck_of_one_is_a_bare_set_bit_for_bit`] is only worth something if this
    /// is the *old* path rather than a second spelling of the new one.
    fn bare_frame(gpu: &Gpu, set: &mut Set, present: &Present, steps: u8) {
        // No bindings on these Sets, so the session clock is inert here and
        // nothing reads a signal; the real one belongs to the deck, and
        // `tests/binding.rs` is where it is asserted.
        set.prepare(&gpu.queue, steps, &Signals::default());
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        set.render(&mut encoder, present.hdr_view(), steps);
        gpu.queue.submit([encoder.finish()]);
        gpu.device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("poll");
    }

    /// The raw `f16` bits of an `Rgba16Float` texture, four per texel. Raw rather
    /// than decoded so that an exact comparison is a comparison of bits and not of
    /// two float expressions that happen to agree.
    fn readback(gpu: &Gpu, texture: &wgpu::Texture) -> Vec<u16> {
        let (width, height) = (texture.width(), texture.height());
        let bytes_per_row = width * 8;
        assert_eq!(bytes_per_row % 256, 0, "row pitch must be 256-byte aligned");

        let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("deck readback"),
            size: u64::from(bytes_per_row * height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        gpu.queue.submit([encoder.finish()]);

        let slice = buffer.slice(..);
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
        buffer.unmap();
        out
    }

    /// `f16` bits to `f32`, for the one test that has to do arithmetic on what it
    /// read back rather than compare it. Written out rather than pulled in as a
    /// dependency: it is fifteen lines and this is the only caller.
    fn f16(bits: u16) -> f32 {
        let sign = f32::from_bits(u32::from(bits & 0x8000) << 16);
        let exponent = (bits >> 10) & 0x1f;
        let mantissa = bits & 0x03ff;
        let magnitude = match exponent {
            // Subnormal, including zero.
            0 => f32::from(mantissa) * 2.0f32.powi(-24),
            // Infinity or NaN, told apart by the mantissa. An HDR target may
            // legitimately hold either; the tests that call this either assert it
            // does not, or are about what happens when it does.
            0x1f if mantissa == 0 => f32::INFINITY,
            0x1f => f32::NAN,
            e => (1.0 + f32::from(mantissa) / 1024.0) * 2.0f32.powi(i32::from(e) - 15),
        };
        if sign.is_sign_negative() {
            -magnitude
        } else {
            magnitude
        }
    }

    fn decode(pixels: &[u16]) -> Vec<f32> {
        pixels.iter().copied().map(f16).collect()
    }

    fn lit(pixels: &[u16]) -> usize {
        pixels
            .chunks_exact(4)
            .filter(|p| p[0] != 0 || p[1] != 0 || p[2] != 0)
            .count()
    }

    /// Simulation steps a Set has taken, recovered as an integer. `Set::time` is
    /// `steps * dt`, and comparing that against a float expression meaning the
    /// same thing is exactly the last-bit trap `Set` derives `t` from an integer
    /// counter to avoid — see `tests/hot_swap.rs`, which does this for the same
    /// reason.
    fn steps_taken(set: &Set) -> u64 {
        (set.time() * 60.0).round() as u64
    }

    // ---------------------------------------------------------------------------

    /// **A deck of one behaves exactly as a bare Set does today.**
    ///
    /// The load-bearing test of the whole slice, and the reason it asserts bit
    /// equality rather than similarity: every single-Set expectation in
    /// `tests/generated.rs`, `tests/lifecycle.rs` and `tests/hot_swap.rs` is about
    /// the path a bare Set takes, and they only keep meaning anything about the
    /// deck if the deck reproduces that path exactly. It can be exact — the mix
    /// reads the texel under the fragment with `textureLoad`, adds it to a zeroed
    /// accumulator at a gain and an opacity of exactly 1.0, and writes an `f16`
    /// that came from an `f16`.
    ///
    /// **The material here writes a coverage in `[0, 1]`, which is the one thing
    /// this comparison assumes.** The mix saturates what it reads into that range
    /// and a bare Set's target holds whatever L4 accumulated, so an L4 writing an
    /// alpha of 1.5 makes the two disagree in alpha and nowhere else — see
    /// `Blend::Add`. Every expectation this test exists to protect is about
    /// colour.
    ///
    /// A failure here is not a tolerance to widen. It means the mix is filtering,
    /// or resampling, or applying something it should not.
    #[test]
    fn a_deck_of_one_is_a_bare_set_bit_for_bit() {
        let gpu = Gpu::headless().expect("no GPU available");

        let bare_present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
        let mut bare = build(&gpu, SEED_A, CAPACITY);
        for _ in 0..12 {
            bare_frame(&gpu, &mut bare, &bare_present, 1);
        }
        let expected = readback(&gpu, bare_present.hdr_texture());

        let deck_present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
        let mut deck = deck_of(&gpu, &[SEED_A]);
        for _ in 0..12 {
            frame(&gpu, &mut deck, &deck_present, 1);
        }
        let mixed = readback(&gpu, deck_present.hdr_texture());

        assert!(
            lit(&expected) > 100,
            "the bare Set drew nothing, so this test would pass on two black frames"
        );
        // And it has to be a *bright* frame, not merely a non-empty one. The colour
        // invariant is that values above 1.0 are expected and are what feeds
        // bloom — see
        // `docs/principles/0064-the-pipeline-is-linear-hdr-and-srgb-is-encoded-once-at-final-output.md`
        // — so a mix that only agreed on [0, 1] would be agreeing on the
        // uninteresting half. Asserted rather than assumed, because the material
        // this test renders is free to get dimmer later and take the property with
        // it silently.
        let brightest = decode(&expected).into_iter().fold(0.0f32, f32::max);
        assert!(
            brightest > 1.0,
            "the bare Set peaked at {brightest}, so bit equality was only checked \
         below 1.0 — the range this pipeline is HDR for is untested"
        );
        assert_eq!(
            mixed, expected,
            "a deck of one slot at unity gain is not the bare Set it composites"
        );
        assert_eq!(steps_taken(deck.slot(0).set()), 12);
    }

    /// **A slot faded to silence cannot take the mix with it, under any blend
    /// mode.**
    ///
    /// A fader at silence has to be a *skip*, not a blend at zero, for the same
    /// reason `Allocated` is: `0.0 * x` is zero only for finite `x`. A slot's own
    /// target is allowed to hold a NaN — `sqrt` of a negative is a procedure that
    /// passes every stage of this pipeline — and one blended at a zero fader would
    /// otherwise put a NaN in every channel of the composite, wiping out every
    /// other slot — see
    /// `docs/adr/0040-a-gain-of-zero-means-no-contribution-so-the-slot-is-skipped.md`.
    /// It is **the** reason the operator has a fader at all.
    ///
    /// The comparison is against the same deck with that slot `Allocated`, which is
    /// the path that was already exact, so this asserts the two ways of silencing a
    /// slot agree.
    ///
    /// Opacity is the fader here, and every mode is tried, because
    /// [`Blend::silent_at`] is the only thing standing between a NaN and the mix
    /// and a mode it forgot would be a slot that cannot be turned off. Gain gets
    /// the same treatment under the two modes where it silences at all —
    /// `zero_gain_silences_add_and_max_and_still_covers_under_over` is where that
    /// list comes from. **Under `over`, gain does not silence and a NaN gets
    /// through**; that is what the fader is for and it is deliberately not asserted
    /// here, because pinning it would read as a promise that NaN reaches the mix.
    #[test]
    fn a_slot_faded_to_silence_cannot_take_the_mix_with_it() {
        let gpu = Gpu::headless().expect("no GPU available");

        let run = |silence: Residency, blend: Blend, gain: f32, opacity: f32| -> Vec<u16> {
            let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
            let mut deck = Deck::new(
                &gpu.device,
                vec![
                    HotSwap::fixed(build(&gpu, SEED_A, CAPACITY)),
                    HotSwap::fixed(build_with(&gpu, L4_NAN, SEED_B, CAPACITY)),
                ],
                WIDTH,
                HEIGHT,
            );
            deck.set_residency(1, silence);
            deck.set_blend(1, blend);
            deck.set_gain(1, gain);
            deck.set_opacity(1, opacity);
            for _ in 0..12 {
                frame(&gpu, &mut deck, &present, 1);
            }
            if silence == Residency::Live {
                // The faded slot is only interesting if its target really does
                // hold a NaN. (Parked, it has never stepped, so what it draws is
                // its zeroed element state and there is no NaN in it to check.)
                let own = readback(&gpu, deck.slot_target(1));
                assert!(
                    decode(&own).iter().any(|v| v.is_nan()),
                    "the NaN slot rendered no NaN, so this test is asserting nothing"
                );
            }
            readback(&gpu, present.hdr_texture())
        };

        // Off air: skipped by residency, and exact.
        let parked = run(Residency::Allocated, Blend::Add, 1.0, 1.0);
        assert!(lit(&parked) > 100, "the surviving slot drew nothing");

        let nans = |mix: &[u16]| decode(mix).iter().filter(|v| v.is_nan()).count();

        for blend in Blend::ALL {
            let faded = run(Residency::Live, blend, 1.0, 0.0);
            assert_eq!(
                faded,
                parked,
                "a slot at opacity 0.0 under `{}` reached the mix ({} NaN channels), while \
             the same slot taken off air did not",
                blend.name(),
                nans(&faded)
            );
        }
        for blend in [Blend::Add, Blend::Max] {
            let faded = run(Residency::Live, blend, 0.0, 1.0);
            assert_eq!(
                faded,
                parked,
                "a slot at gain 0.0 under `{}` reached the mix ({} NaN channels)",
                blend.name(),
                nans(&faded)
            );
        }
    }

    /// **A mask at either end is exact: nothing, or everything.**
    ///
    /// Both matter and for different reasons. At the top, a wipe is a transition
    /// carrying `position` to 1.0, so a corner left half-lit would be a wipe that
    /// never finished. At the bottom, `Blend::silent_at` *skips* a layer whose mask
    /// reveals nothing — which is only sound if it really is nothing, and skipping
    /// is what keeps a NaN out of the mix.
    ///
    /// Compared against the fader, which is the path that was already exact: a
    /// masked-out slot must render exactly what the same slot at opacity 0 renders,
    /// and a fully revealed one exactly what it renders with no mask at all. The
    /// material is [`L4_WASH`], because a mask's ends are at the edges of the frame
    /// and the sphere every other fixture draws never gets there.
    #[test]
    fn a_mask_at_either_end_is_exactly_nothing_or_exactly_everything() {
        let gpu = Gpu::headless().expect("no GPU available");

        let run = |mask: Option<Mask>, opacity: f32| -> Vec<u16> {
            let present = Present::new(&gpu.device, Present::HDR_FORMAT, MASK_SIZE, MASK_SIZE);
            let mut deck = wash_deck(&gpu);
            deck.set_opacity(1, opacity);
            if let Some(mask) = mask {
                deck.set_mask(1, mask);
            }
            for _ in 0..4 {
                frame(&gpu, &mut deck, &present, 1);
            }
            readback(&gpu, present.hdr_texture())
        };

        let unmasked = run(None, 1.0);
        let silent = run(None, 0.0);
        assert!(lit(&unmasked) > 100, "the deck drew nothing");
        assert_ne!(unmasked, silent, "the two references are the same picture");
        // The wash has to reach every texel, or an end that is wrong at the edge
        // is an end nothing here can see.
        assert_eq!(
            lit(&unmasked),
            (MASK_SIZE * MASK_SIZE) as usize,
            "the wash does not cover the frame, so a mask's edges are untested"
        );

        for kind in [MaskKind::Linear, MaskKind::Radial] {
            // Every angle, because a linear front's normalisation is per-direction
            // and one that overshot would show at one angle and not another.
            for angle in [0.0, 0.7, std::f32::consts::FRAC_PI_2, 2.4, -0.7] {
                assert_eq!(
                    run(Some(Mask::new(kind, angle, 1.0, 0.3)), 1.0),
                    unmasked,
                    "{} at {angle} rad, fully open, is not the unmasked frame",
                    kind.name()
                );
                assert_eq!(
                    run(Some(Mask::new(kind, angle, 0.0, 0.3)), 1.0),
                    silent,
                    "{} at {angle} rad, fully closed, is not a silent slot",
                    kind.name()
                );
            }
        }
    }

    /// **A mask that reveals nothing is a skip, not a multiply by zero.**
    ///
    /// The only way to see the difference, and the reason the skip is there: a
    /// slot's target may hold a NaN — `sqrt` of a negative is a procedure that
    /// passes every stage of this pipeline — and `0.0 * NaN` is NaN. With clean
    /// material a mask at position 0 and a skipped layer are the same picture, so
    /// this is the case that tells them apart, exactly as it does for the fader.
    ///
    /// It is also what makes a mask a third escape from broken material, beside
    /// residency and the fader.
    #[test]
    fn a_mask_that_reveals_nothing_keeps_a_nan_out_of_the_mix() {
        let gpu = Gpu::headless().expect("no GPU available");

        let run = |silence: Residency, mask: Mask| -> Vec<u16> {
            let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
            let mut deck = Deck::new(
                &gpu.device,
                vec![
                    HotSwap::fixed(build(&gpu, SEED_A, CAPACITY)),
                    HotSwap::fixed(build_with(&gpu, L4_NAN, SEED_B, CAPACITY)),
                ],
                WIDTH,
                HEIGHT,
            );
            deck.set_residency(1, silence);
            deck.set_mask(1, mask);
            for _ in 0..12 {
                frame(&gpu, &mut deck, &present, 1);
            }
            if silence == Residency::Live {
                assert!(
                    decode(&readback(&gpu, deck.slot_target(1)))
                        .iter()
                        .any(|v| v.is_nan()),
                    "the NaN slot rendered no NaN, so this test is asserting nothing"
                );
            }
            readback(&gpu, present.hdr_texture())
        };

        let parked = run(Residency::Allocated, Mask::default());
        assert!(lit(&parked) > 100, "the surviving slot drew nothing");

        for kind in [MaskKind::Linear, MaskKind::Radial] {
            let closed = run(Residency::Live, Mask::new(kind, 0.4, 0.0, 0.1));
            let nans = decode(&closed).iter().filter(|v| v.is_nan()).count();
            assert_eq!(
                closed,
                parked,
                "a slot masked to nothing under `{}` reached the mix ({nans} NaN channels), \
             while the same slot taken off air did not",
                kind.name()
            );
        }
    }

    /// **A mask in the middle shapes the frame rather than dimming it.**
    ///
    /// The difference between a mask and a fader, and the only assertion that can
    /// tell them apart: half way across, part of the frame is exactly what it would
    /// be with the slot present and part exactly what it would be without. A fader
    /// at 0.5 is neither, everywhere.
    #[test]
    fn a_mask_half_way_leaves_one_part_untouched_and_removes_another() {
        let gpu = Gpu::headless().expect("no GPU available");

        let run = |mask: Option<Mask>, opacity: f32| -> Vec<f32> {
            let present = Present::new(&gpu.device, Present::HDR_FORMAT, MASK_SIZE, MASK_SIZE);
            let mut deck = wash_deck(&gpu);
            deck.set_opacity(1, opacity);
            if let Some(mask) = mask {
                deck.set_mask(1, mask);
            }
            for _ in 0..4 {
                frame(&gpu, &mut deck, &present, 1);
            }
            decode(&readback(&gpu, present.hdr_texture()))
        };

        let silent = run(None, 0.0);
        // A hard front straight up the middle, left to right, on a slot at half
        // opacity — so "revealed" and "unmasked" are different pictures and the
        // mask cannot be mistaken for the fader that is also on.
        let halfway = run(Some(Mask::new(MaskKind::Linear, 0.0, 0.5, 0.0)), 0.5);
        let faded = run(None, 0.5);

        // **Only where the slot actually contributes.** Where it drew nothing, the
        // masked frame, the faded one and the silent one all agree, and counting
        // those would drown the claim in background.
        let mut hidden = 0;
        let mut revealed = 0;
        let mut between = 0;
        for i in (0..halfway.len()).filter(|i| i % 4 != 3) {
            if faded[i] == silent[i] {
                continue;
            }
            if halfway[i] == silent[i] {
                hidden += 1;
            } else if halfway[i] == faded[i] {
                revealed += 1;
            } else {
                between += 1;
            }
        }

        assert!(
            hidden > 100,
            "the mask removed the slot from {hidden} of the channels it drew, so the front \
         is not on the frame"
        );
        assert!(
            revealed > 100,
            "the mask left the slot in {revealed} of the channels it drew, so it is hiding \
         everything rather than shaping"
        );
        // A hard edge, so every contributing channel is on one side or the other.
        // A *fader* would put all of them in `between`, which is the difference
        // this test exists to see.
        assert!(
            between * 20 < hidden + revealed,
            "{between} channels are neither the revealed picture nor the hidden one, \
         against {} that are — a hard-edged mask is one or the other",
            hidden + revealed
        );
        // And the front is where it was asked for: half the covered frame, either
        // side. A wipe that finished early would still be "one or the other".
        let split = hidden as f32 / (hidden + revealed) as f32;
        assert!(
            (split - 0.5).abs() < 0.1,
            "the front left {split:.2} of the frame hidden rather than half, so it is not \
         where `position` says"
        );
    }

    /// **A wipe is a mask and one scheduled move**, and neither had to know about
    /// the other.
    ///
    /// The claim the whole design rests on: `Control::MaskPosition` carries the
    /// front, the mask reads a number, and the picture between the two ends is
    /// neither of them. Checked at three points, because a wipe that jumped would
    /// pass a two-point test.
    #[test]
    fn a_wipe_is_a_transition_carrying_a_masks_front() {
        let gpu = Gpu::headless().expect("no GPU available");
        let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
        let mut deck = deck_of(&gpu, &[SEED_A, SEED_B]);
        deck.set_signals(Signals::new(120.0, 1));
        deck.set_blend(1, Blend::Over);
        deck.set_mask(1, Mask::new(MaskKind::Linear, 0.0, 0.0, 0.02));

        let start = deck.signals().oscillator().beats();
        deck.schedule(Transition::new(
            1,
            Control::MaskPosition,
            0.0,
            1.0,
            start,
            4.0,
            Curve::Lin,
        ));

        let mut fronts = Vec::new();
        for i in 0..121 {
            frame(&gpu, &mut deck, &present, 1);
            if i % 30 == 0 {
                fronts.push(deck.mask(1).position());
            }
        }
        // Monotone and strictly moving, which a jump would not be.
        for pair in fronts.windows(2) {
            assert!(
                pair[1] > pair[0],
                "the front went backwards or stood still: {fronts:?}"
            );
        }
        assert_eq!(deck.mask(1).position(), 1.0, "the wipe did not finish");
        // The shape survived: a move carries the position and leaves the kind
        // alone, which is why `set_mask_shape` does not cancel a transition.
        assert_eq!(deck.mask(1).kind(), MaskKind::Linear);
        assert_eq!(deck.transitions_on(1).count(), 0);
    }

    /// **The operator wins on the position, and the shape is not a hand on
    /// it.**
    ///
    /// The two halves of a mask answer a scheduled move differently, and this
    /// is what says so in both directions at once — because a cancel written
    /// into the wrong one of the two setters passes every other test in this
    /// file. `set_mask_position` writes the number `Control::MaskPosition` is
    /// carrying, so a hand on it stops the move
    /// (`docs/principles/0078-the-operator-wins-and-an-automatic-writer-yields-to-a-hand.md`);
    /// `set_mask_shape` writes no position, so there is nothing under its hand
    /// and a shape chosen mid-wipe changes what is being wiped rather than
    /// stopping it.
    ///
    /// No frame is drawn: the question is what the deck holds, and
    /// `Deck::transitions_on` is where the answer is.
    #[test]
    fn a_hand_on_the_front_stops_the_wipe_and_a_hand_on_the_shape_does_not() {
        let gpu = Gpu::headless().expect("no GPU available");
        let mut deck = deck_of(&gpu, &[SEED_A, SEED_B]);
        deck.set_signals(Signals::new(120.0, 1));
        let wipe = |deck: &mut Deck| {
            deck.set_mask_shape(1, MaskKind::Linear, 0.0);
            deck.set_mask_position(1, 0.0);
            let start = deck.signals().oscillator().beats();
            deck.schedule(Transition::new(
                1,
                Control::MaskPosition,
                0.0,
                1.0,
                start,
                4.0,
                Curve::Lin,
            ));
        };

        wipe(&mut deck);
        deck.set_mask_shape(1, MaskKind::Radial, 0.0);
        assert_eq!(
            deck.transitions_on(1).count(),
            1,
            "choosing a shape mid-wipe cancelled the move — a shape writes no position, \
             so it is not a hand on the control the transition is carrying, and a wipe \
             that stopped because somebody changed what it wipes with is a move nobody \
             withdrew"
        );
        assert_eq!(
            deck.mask(1).kind(),
            MaskKind::Radial,
            "the shape did not land"
        );

        wipe(&mut deck);
        deck.set_mask_position(1, 0.75);
        assert_eq!(
            deck.transitions_on(1).count(),
            0,
            "a hand on the front left the move running — the transition writes that same \
             number every frame, so it would take the front straight back and the \
             operator would be holding a control that fights back"
        );
        assert_eq!(deck.mask(1).position(), 0.75, "the front did not land");
    }

    /// A Set drawn by two renderers over one simulation, composited.
    ///
    /// The shape a selection is about: one L1, two L4s, and an L5 folding
    /// them — which is what gives each renderer an edge with a `live` flag on
    /// it.
    fn set_of_two_renderers(gpu: &Gpu) -> Set {
        let (l1, a, b) = (compile(L1), compile(L4), compile(L4_B));
        let mut set = Set::build_many(
            &gpu.device,
            &gpu.queue,
            &[(&l1, CAPACITY)],
            &[],
            &[],
            &[],
            &[&a, &b],
            karakuri_engine::set::Layering::Composite,
            SEED_A,
            &[],
            karakuri_engine::set::Wiring::default(),
        )
        .expect("one geometry and two renderers over it");
        set.resize(&gpu.device, WIDTH, HEIGHT);
        set
    }

    /// **A scheduled selection lands on the beat it was given and not before.**
    ///
    /// The claim that makes a selection a *musical* control rather than a key
    /// press: it is quantised once, where the operator asked, and every frame
    /// until that beat leaves the Set exactly as it was. A selection applied
    /// where it was scheduled would pass every arithmetic test there is and
    /// still be the wrong instrument — the picture would change on the hand
    /// rather than on the bar.
    ///
    /// The queue is asserted as well as the edges, because the two failures
    /// look alike from outside: a selection that never lands and one that
    /// lands and is applied again every frame afterwards both leave the right
    /// renderer live.
    #[test]
    fn a_scheduled_selection_lands_on_the_beat_it_was_given_and_not_before() {
        let gpu = Gpu::headless().expect("no GPU available");
        let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
        let mut deck = Deck::new(
            &gpu.device,
            vec![HotSwap::fixed(set_of_two_renderers(&gpu))],
            WIDTH,
            HEIGHT,
        );
        // 120 bpm and dt of 1/60 is two beats a second, so a frame is 1/30 of a
        // beat and the bar below is 120 frames away rather than an unknown
        // number of them.
        deck.set_signals(Signals::new(120.0, 1));
        // Off the boundary first, so that quantising has somewhere to go: on
        // beat 0 the next bar *is* now, which is correct and is not this test.
        frame(&gpu, &mut deck, &present, 1);

        let now = deck.signals().oscillator().beats();
        let start = quantise(now, 4.0);
        assert!(start > now, "the fixture is already on the bar");
        deck.schedule_selection(Selection::new(0, 1, start));

        let live = |deck: &Deck| -> Vec<usize> {
            deck.slot(0)
                .set()
                .inputs()
                .iter()
                .enumerate()
                .filter(|(_, input)| input.live)
                .map(|(at, _)| at)
                .collect()
        };
        assert_eq!(live(&deck), vec![0, 1], "a Set comes up folding both");

        while deck.signals().oscillator().beats() < start {
            assert_eq!(
                live(&deck),
                vec![0, 1],
                "the selection landed at {} beats, before the {start} it was given",
                deck.signals().oscillator().beats()
            );
            assert_eq!(deck.selections_on(0).count(), 1, "the selection is armed");
            frame(&gpu, &mut deck, &present, 1);
        }

        // The frame that crossed the instant applied it: exactly one live, and
        // it is the one that was asked for.
        assert_eq!(live(&deck), vec![1]);
        assert_eq!(
            deck.selections_on(0).count(),
            0,
            "a selection that has landed is still queued, and would be applied \
             over whatever moves the edges next"
        );
    }

    /// **A scheduled fade moves the fader on the beat grid and nowhere else.**
    ///
    /// The claim that makes a transition reproducible: it is a function of the
    /// session's beat count, so two runs given the same ticks fade identically —
    /// and a run at a different frame rate reaching the same beat is at the same
    /// point in the fade. Checked against the arithmetic rather than against a
    /// second run of the deck, which would agree with any implementation.
    #[test]
    fn a_scheduled_fade_is_a_function_of_the_beat_count() {
        let gpu = Gpu::headless().expect("no GPU available");
        let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
        let mut deck = deck_of(&gpu, &[SEED_A]);
        // 120 bpm and dt of 1/60 is exactly two beats a second, so a frame is
        // 1/30 of a beat and the arithmetic below is exact.
        deck.set_signals(Signals::new(120.0, 1));

        let start = deck.signals().oscillator().beats();
        deck.schedule(Transition::new(
            0,
            Control::Opacity,
            1.0,
            0.0,
            start,
            4.0,
            Curve::Lin,
        ));

        // 120 frames is four beats: the whole fade.
        for i in 0..120 {
            let beats = deck.signals().oscillator().beats();
            let expected = 1.0 - (beats - start) as f32 / 4.0;
            assert!(
                (deck.opacity(0) - expected).abs() < 1e-6,
                "frame {i} at {beats} beats: the fader is {} rather than {expected}",
                deck.opacity(0)
            );
            frame(&gpu, &mut deck, &present, 1);
        }
        // Exactly at silence, and the transition gone rather than still writing.
        assert_eq!(deck.opacity(0), 0.0);
        assert_eq!(
            deck.transitions_on(0).count(),
            0,
            "a finished fade is still scheduled"
        );
    }

    /// **A hand on the fader wins.**
    ///
    /// The one place an operator reaches when something is wrong is the one place
    /// an automatic thing is writing, so a transition that kept going after a
    /// manual move would be the worst control on the deck. Asserted for both ways
    /// of touching it, since either is what a hand does.
    #[test]
    fn moving_a_control_by_hand_cancels_the_transition_moving_it() {
        let gpu = Gpu::headless().expect("no GPU available");
        let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
        let mut deck = deck_of(&gpu, &[SEED_A]);
        deck.set_signals(Signals::new(120.0, 1));

        let start = deck.signals().oscillator().beats();
        deck.schedule(Transition::new(
            0,
            Control::Opacity,
            1.0,
            0.0,
            start,
            8.0,
            Curve::Lin,
        ));
        for _ in 0..30 {
            frame(&gpu, &mut deck, &present, 1);
        }
        let mid = deck.opacity(0);
        assert!(mid > 0.0 && mid < 1.0, "the fade did not start: {mid}");

        deck.set_opacity(0, 0.75);
        for _ in 0..60 {
            frame(&gpu, &mut deck, &present, 1);
        }
        assert_eq!(
            deck.opacity(0),
            0.75,
            "the fade kept writing after the fader was moved by hand"
        );
        assert_eq!(deck.transitions_on(0).count(), 0);

        // And the other control's transition is untouched by the wrong fader:
        // cancelling has to be per control, or a gain move would stop an opacity
        // fade and an operator would never find out why.
        deck.schedule(Transition::new(
            0,
            Control::Gain,
            1.0,
            0.0,
            start,
            8.0,
            Curve::Lin,
        ));
        deck.set_opacity(0, 0.5);
        assert_eq!(
            deck.transitions_on(0).count(),
            1,
            "the gain fade was cancelled too"
        );

        // **The gain half of the rule, which this test claimed and did not check.**
        // `[`, `]` and `\` all end in `set_gain`, so a gain fade that kept writing
        // after one of them would be a control fighting the hand on it.
        let start = deck.signals().oscillator().beats();
        deck.schedule(Transition::new(
            0,
            Control::Gain,
            1.0,
            0.0,
            start,
            8.0,
            Curve::Lin,
        ));
        for _ in 0..30 {
            frame(&gpu, &mut deck, &present, 1);
        }
        let mid = deck.gain(0);
        assert!(mid > 0.0 && mid < 1.0, "the gain fade did not start: {mid}");
        deck.set_gain(0, 2.0);
        for _ in 0..60 {
            frame(&gpu, &mut deck, &present, 1);
        }
        assert_eq!(
            deck.gain(0),
            2.0,
            "the gain fade kept writing after the level was moved by hand"
        );
        assert_eq!(deck.transitions_on(0).count(), 0);
    }

    /// **A move onto a slot the deck does not have is refused where it is asked
    /// for**, not three seconds later inside a frame.
    ///
    /// `advance_transitions` indexes the slots directly, so an unchecked schedule
    /// is a panic on the render thread at some unrelated moment. `set_gain` and
    /// `set_opacity` panic at the call site; this joins them.
    #[test]
    #[should_panic(expected = "no slot 3")]
    fn scheduling_a_move_onto_a_slot_that_is_not_there_is_refused_at_the_call() {
        let gpu = Gpu::headless().expect("no GPU available");
        let mut deck = deck_of(&gpu, &[SEED_A]);
        deck.schedule(Transition::new(
            3,
            Control::Opacity,
            1.0,
            0.0,
            0.0,
            4.0,
            Curve::Lin,
        ));
    }

    /// **The composite sees this frame's fader, not the last one's.**
    ///
    /// A transition that ran *after* the mix was recorded would put every fade one
    /// frame late — invisible in a four-beat fade and exactly wrong in a cut, which
    /// is the case this uses. Compared against a deck whose fader was moved by hand
    /// before the frame, which is the path that was already exact: the two are the
    /// same picture if and only if the scheduled cut landed on the frame it was
    /// scheduled for.
    ///
    /// The comparison it replaces was `assert_ne!` against an earlier frame, which
    /// this material passes with no transition scheduled at all — it rotates on `t`.
    #[test]
    fn a_scheduled_cut_lands_on_the_frame_it_was_scheduled_for() {
        let gpu = Gpu::headless().expect("no GPU available");
        const LEAD: usize = 12;

        // The reference: the same deck, the same ticks, the fader moved by hand
        // before the frame in question.
        let by_hand = {
            let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
            let mut deck = deck_of(&gpu, &[SEED_A, SEED_B]);
            deck.set_signals(Signals::new(120.0, 1));
            for _ in 0..LEAD {
                frame(&gpu, &mut deck, &present, 1);
            }
            deck.set_opacity(1, 0.0);
            frame(&gpu, &mut deck, &present, 1);
            readback(&gpu, present.hdr_texture())
        };

        // The same run, with the cut scheduled for the beat that frame lands on.
        let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
        let mut deck = deck_of(&gpu, &[SEED_A, SEED_B]);
        deck.set_signals(Signals::new(120.0, 1));
        for _ in 0..LEAD {
            frame(&gpu, &mut deck, &present, 1);
        }
        // Where the clock stands. The next frame advances it first, so this
        // instant is already past by the time the transition is read — which is
        // the frame it is due on, and the frame the composite has to see it on.
        let cut_at = deck.signals().oscillator().beats();
        deck.schedule(Transition::new(
            1,
            Control::Opacity,
            1.0,
            0.0,
            cut_at,
            0.0,
            Curve::Lin,
        ));
        frame(&gpu, &mut deck, &present, 1);
        let scheduled = readback(&gpu, present.hdr_texture());

        assert_eq!(
            deck.opacity(1),
            0.0,
            "the cut did not land on the frame it was scheduled for"
        );
        assert_eq!(
            scheduled, by_hand,
            "the mix on the frame of a scheduled cut is not the mix of the same fader moved \
         by hand — the composite is reading a fader the transition has not written yet"
        );
    }

    /// **A scheduled move cannot reach a value a hand could not.**
    ///
    /// It writes the slot's field directly rather than through `set_gain` and
    /// `set_opacity`, because those cancel it — so the clamps they carry have to be
    /// applied on the way past, or a `transition` record would be the one path into
    /// the mix with no bound on it. An opacity above 1.0 makes an `over` layer
    /// subtract more than it covers.
    #[test]
    fn a_scheduled_move_is_clamped_the_way_a_manual_one_is() {
        let gpu = Gpu::headless().expect("no GPU available");
        let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
        let mut deck = deck_of(&gpu, &[SEED_A]);
        deck.set_signals(Signals::new(120.0, 1));
        let start = deck.signals().oscillator().beats();

        deck.schedule(Transition::new(
            0,
            Control::Opacity,
            1.0,
            4.0,
            start,
            0.0,
            Curve::Lin,
        ));
        deck.schedule(Transition::new(
            0,
            Control::Gain,
            1.0,
            -3.0,
            start,
            0.0,
            Curve::Lin,
        ));
        frame(&gpu, &mut deck, &present, 1);

        assert_eq!(deck.opacity(0), 1.0, "a scheduled fader passed 1.0");
        assert_eq!(deck.gain(0), 0.0, "a scheduled level went negative");
    }

    /// **A crossfade is two scheduled moves**, and what makes that a crossfade
    /// rather than two fades is that they share a start and a length.
    ///
    /// The mix is checked rather than the fields: halfway through, the outgoing
    /// slot is dimmer than it was and the incoming one is brighter, and the frame
    /// carries both. That is the whole of what a crossfade is, and it needed no type
    /// of its own.
    #[test]
    fn a_crossfade_is_two_moves_sharing_a_start_and_a_length() {
        let gpu = Gpu::headless().expect("no GPU available");
        let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
        let mut deck = deck_of(&gpu, &[SEED_A, SEED_B]);
        deck.set_signals(Signals::new(120.0, 1));
        deck.set_opacity(1, 0.0);
        for _ in 0..12 {
            frame(&gpu, &mut deck, &present, 1);
        }

        let start = deck.signals().oscillator().beats();
        deck.schedule(Transition::new(
            0,
            Control::Opacity,
            1.0,
            0.0,
            start,
            4.0,
            Curve::Smooth,
        ));
        deck.schedule(Transition::new(
            1,
            Control::Opacity,
            0.0,
            1.0,
            start,
            4.0,
            Curve::Smooth,
        ));

        // Two beats: halfway, where both are somewhere in the middle.
        for _ in 0..60 {
            frame(&gpu, &mut deck, &present, 1);
        }
        let a = deck.opacity(0);
        let b = deck.opacity(1);
        assert!(a > 0.0 && a < 1.0 && b > 0.0 && b < 1.0, "{a} / {b}");
        assert!(
            (a + b - 1.0).abs() < 0.05,
            "a smooth crossfade should be near unity through the middle: {a} + {b}"
        );

        // Two more beats: the other end, exactly.
        for _ in 0..60 {
            frame(&gpu, &mut deck, &present, 1);
        }
        assert_eq!(deck.opacity(0), 0.0);
        assert_eq!(deck.opacity(1), 1.0);
        assert_eq!(
            deck.transitions_on(0).count() + deck.transitions_on(1).count(),
            0
        );
    }

    /// **`over` hides what is under it and `add` does not**, which is the whole of
    /// what the blend vocabulary buys.
    ///
    /// Slot 1 is [`L4_CARD`] — black, opaque, and contributing no colour at all —
    /// so the two modes differ by exactly one thing: whether the coverage it drew
    /// is allowed to take the layer under it away. The expectation is not a
    /// direction but a number, read from the card's own target:
    ///
    /// ```text
    ///   add:   A + 0        = A
    ///   over:  0 + A*(1 - c)         c = the card's coverage at that texel
    /// ```
    ///
    /// Inexact for the same single reason as `gain_is_linear_...`: the GPU works in
    /// `f32` and rounds once to `f16` on write, while the expectation is computed
    /// in `f32` from values already rounded to `f16`.
    #[test]
    fn over_hides_what_is_under_it_and_add_does_not() {
        let gpu = Gpu::headless().expect("no GPU available");

        let run = |blend: Blend| -> (Vec<f32>, Vec<f32>) {
            let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
            let mut deck = Deck::new(
                &gpu.device,
                vec![
                    HotSwap::fixed(build(&gpu, SEED_A, CAPACITY)),
                    HotSwap::fixed(build_with(&gpu, L4_CARD, SEED_B, CAPACITY)),
                ],
                WIDTH,
                HEIGHT,
            );
            deck.set_blend(1, blend);
            for _ in 0..12 {
                frame(&gpu, &mut deck, &present, 1);
            }
            (
                decode(&readback(&gpu, present.hdr_texture())),
                decode(&readback(&gpu, deck.slot_target(1))),
            )
        };

        let (added, card) = run(Blend::Add);
        let (overed, _) = run(Blend::Over);

        // The card has to actually cover something, or every assertion below is
        // `A == A` and this test says nothing. Alpha is the fourth channel.
        let covered = card.iter().skip(3).step_by(4).filter(|c| **c > 0.5).count();
        assert!(
            covered > 100,
            "the card covered only {covered} texels, so there is nothing for `over` to hide"
        );

        const TOLERANCE: f32 = 1.0 / 1024.0;
        let close = |x: f32, y: f32| (x - y).abs() <= TOLERANCE * (1.0 + x.abs().max(y.abs()));

        let mut misses = 0;
        let mut worst = 0.0f32;
        let mut hidden = 0;
        // Colour only: the fourth channel is coverage, and what the mix does with
        // coverage is the same under every mode by construction.
        for i in (0..added.len()).filter(|i| i % 4 != 3) {
            let coverage = card[(i / 4) * 4 + 3];
            let expected = added[i] * (1.0 - coverage);
            if !close(overed[i], expected) {
                misses += 1;
                worst = worst.max((overed[i] - expected).abs());
            }
            if !close(overed[i], added[i]) {
                hidden += 1;
            }
        }

        assert_eq!(
            misses,
            0,
            "`over` is not `A*(1 - coverage)`: {misses} of {} colour channels disagree, \
         worst by {worst}",
            added.len()
        );
        // The other half, and the half that fails if `over` silently stayed `add`:
        // the two modes have to differ somewhere, or the first assertion passed
        // only because the coverage was zero everywhere it looked.
        assert!(
            hidden > 100,
            "only {hidden} channels distinguish `over` from `add`, so this run cannot tell \
         the two modes apart"
        );
    }

    /// **`max` stacks without summing.**
    ///
    /// Two lit slots. Under `add` the mix is `A + B`; under `max` it is the larger
    /// of the two per channel, which is what makes four layers of the same bright
    /// material stay that bright instead of reaching four times it. `A` and `B` are
    /// measured on their own — same deck, same seeds, same ticks, one slot off air
    /// each time — so both are sampled at the same `t` as the mix.
    #[test]
    fn max_takes_the_larger_of_two_layers_rather_than_their_sum() {
        let gpu = Gpu::headless().expect("no GPU available");

        let run = |blend: Blend, off_air: Option<usize>| -> Vec<f32> {
            let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
            let mut deck = deck_of(&gpu, &[SEED_A, SEED_B]);
            deck.set_blend(1, blend);
            if let Some(slot) = off_air {
                deck.set_residency(slot, Residency::Allocated);
            }
            for _ in 0..12 {
                frame(&gpu, &mut deck, &present, 1);
            }
            decode(&readback(&gpu, present.hdr_texture()))
        };

        let a = run(Blend::Add, Some(1));
        let b = run(Blend::Add, Some(0));
        let summed = run(Blend::Add, None);
        let maxed = run(Blend::Max, None);

        const TOLERANCE: f32 = 1.0 / 1024.0;
        let close = |x: f32, y: f32| (x - y).abs() <= TOLERANCE * (1.0 + x.abs().max(y.abs()));

        let mut misses = 0;
        let mut worst = 0.0f32;
        let mut distinct = 0;
        for i in (0..maxed.len()).filter(|i| i % 4 != 3) {
            let expected = a[i].max(b[i]);
            if !close(maxed[i], expected) {
                misses += 1;
                worst = worst.max((maxed[i] - expected).abs());
            }
            // Where both layers are lit, `max` is strictly less than `add`. If
            // nowhere is, the two slots never overlap and the comparison above is
            // `A + 0` against `max(A, 0)`, which agree.
            if !close(maxed[i], summed[i]) {
                distinct += 1;
            }
        }

        assert_eq!(
            misses,
            0,
            "`max` is not the per-channel maximum: {misses} of {} colour channels disagree, \
         worst by {worst}",
            maxed.len()
        );
        assert!(
            distinct > 100,
            "only {distinct} channels distinguish `max` from `add`, so the two slots barely \
         overlap and this run cannot tell them apart"
        );
    }

    /// **An opacity outside `[0, 1]` is clamped where the engine takes it**, not
    /// where a key press produces it.
    ///
    /// `karakuri-cli` clamps at the key so that the `opacity` record carries the
    /// value that took effect, but a record is also how a *replay* drives the deck,
    /// and a stream is allowed to say anything. Past 1.0 an `over` layer subtracts
    /// more than it covers; below 0.0 it adds what it should have hidden. Unlike
    /// gain — a level into an HDR mix, deliberately open above 1.0 — every value
    /// outside this range has exactly one sensible reading, so it is clamped rather
    /// than refused.
    ///
    /// NaN silences, which is the third value a fader can carry and the one with no
    /// obvious reading: the two available are "this slot goes dark" and "the whole
    /// mix goes dark".
    #[test]
    fn an_opacity_a_record_could_carry_is_clamped_to_a_fader() {
        let gpu = Gpu::headless().expect("no GPU available");
        let mut deck = deck_of(&gpu, &[SEED_A]);

        for (asked, expected) in [
            (3.0, 1.0),
            (1.0, 1.0),
            (0.5, 0.5),
            (0.0, 0.0),
            (-2.0, 0.0),
            (f32::INFINITY, 1.0),
            (f32::NEG_INFINITY, 0.0),
            (f32::NAN, 0.0),
        ] {
            deck.set_opacity(0, asked);
            assert_eq!(
                deck.opacity(0),
                expected,
                "an opacity of {asked} reached the mix as {}",
                deck.opacity(0)
            );
        }
    }

    /// **A gain a record could carry is floored at zero, and a NaN reads as zero.**
    ///
    /// The same hole as the one above and it needed the same answer: `karakuri-cli`
    /// floors at the key press, which says plainly that a negative gain is wrong,
    /// but a replayed `{"t":"gain","slot":1,"value":-2.0}` does not go through a
    /// key press. Unbounded *above*, unlike opacity, because gain is a level into
    /// an HDR mix and 4.0 is an ordinary thing to want.
    ///
    /// The NaN case is the one that cannot be recovered from. A NaN gain puts a NaN
    /// in every channel of the mix from one slot, and unlike the material's own
    /// NaN — which the fader skips past — no fader undoes a gain that has already
    /// multiplied by one.
    #[test]
    fn a_gain_a_record_could_carry_is_floored_but_not_ceilinged() {
        let gpu = Gpu::headless().expect("no GPU available");
        let mut deck = deck_of(&gpu, &[SEED_A]);

        for (asked, expected) in [
            (4.0, 4.0),
            (1.0, 1.0),
            (0.0, 0.0),
            (-2.0, 0.0),
            (f32::INFINITY, f32::INFINITY),
            (f32::NEG_INFINITY, 0.0),
            (f32::NAN, 0.0),
        ] {
            deck.set_gain(0, asked);
            assert_eq!(
                deck.gain(0),
                expected,
                "a gain of {asked} reached the mix as {}",
                deck.gain(0)
            );
        }
    }

    /// **An alpha that is not a coverage cannot invert the mix or NaN it.**
    ///
    /// `over` is `A*(1 - covered)`, so a coverage of 1.5 turns hiding into
    /// *subtracting*, a coverage of 2 or more turns it into amplifying with the
    /// sign flipped, and a NaN takes every channel of the frame. Nothing in the
    /// pipeline bounds what an L4 writes to alpha — see [`OVERDRAWN_ALPHA`] — and
    /// before blend modes existed that did not matter, because the channel was
    /// written by nothing and read by nothing. It is load-bearing now, which is why
    /// the mix saturates on the way in rather than trusting the material.
    ///
    /// **All four spellings of "not a coverage", not only the one that motivated
    /// the fix.** Above one is the case that reads as a hiding layer subtracting;
    /// negative and infinite are the same arithmetic further along; and NaN is the
    /// one the saturation catches only because it is written as a comparison rather
    /// than as `clamp`, whose behaviour on a NaN operand WGSL leaves to the
    /// backend. A test that ran only the finite case would pass on a backend where
    /// the NaN case renders a blank frame.
    ///
    /// Three claims per spelling. No colour channel of the mix is a NaN, none is
    /// negative, and the mix's own coverage stays in range — the last one being
    /// what says the saturation is where it belongs, since that value is what the
    /// slot above this one is composited against.
    #[test]
    fn an_alpha_that_is_not_a_coverage_cannot_invert_the_mix_or_nan_it() {
        let gpu = Gpu::headless().expect("no GPU available");

        for (name, alpha) in OVERDRAWN_ALPHA {
            let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
            let mut deck = Deck::new(
                &gpu.device,
                vec![
                    HotSwap::fixed(build_with(&gpu, L4_WIDE, SEED_A, CAPACITY)),
                    HotSwap::fixed(build_with(&gpu, &overdrawn_card(alpha), SEED_B, CAPACITY)),
                ],
                WIDTH,
                HEIGHT,
            );
            deck.set_blend(1, Blend::Over);
            for _ in 0..12 {
                frame(&gpu, &mut deck, &present, 1);
            }

            // The material has to actually be out of range, or this is a test of
            // ordinary coverage under a frightening name.
            let own = decode(&readback(&gpu, deck.slot_target(1)));
            let bad = own
                .iter()
                .skip(3)
                .step_by(4)
                .filter(|a| !(0.0..=1.0).contains(*a))
                .count();
            assert!(
                bad > 100,
                "the {name} card left only {bad} texels outside a coverage of [0, 1], so \
             nothing here is being saturated"
            );

            let mixed = decode(&readback(&gpu, present.hdr_texture()));
            let nan = (0..mixed.len())
                .filter(|i| i % 4 != 3)
                .filter(|&i| mixed[i].is_nan())
                .count();
            assert_eq!(
                nan, 0,
                "{nan} colour channels of the mix are NaN under the {name} card, so an alpha \
             nothing draws with reached every channel of the frame"
            );
            let negative = (0..mixed.len())
                .filter(|i| i % 4 != 3)
                .filter(|&i| mixed[i] < 0.0)
                .count();
            assert_eq!(
                negative, 0,
                "{negative} colour channels of the mix are negative under the {name} card, so \
             the coverage turned `over` from hiding into subtracting"
            );
            let unbounded = mixed
                .iter()
                .skip(3)
                .step_by(4)
                .filter(|a| !(0.0..=1.0).contains(*a))
                .count();
            assert_eq!(
                unbounded, 0,
                "{unbounded} texels of the mix carry a coverage outside [0, 1] under the \
             {name} card, which is what the next slot in the stack would be composited \
             against"
            );
        }
    }

    /// **Opacity moves the mix at settings between silence and full, under every
    /// mode.**
    ///
    /// Every other test here pins the fader at 0.0 or 1.0, where a composite that
    /// ignored `opacity` outright is indistinguishable from one that honours it —
    /// 0.0 is the skip, which [`Blend::silent_at`] decides on the host, and 1.0 is
    /// the identity. So without this test the one control this whole slice exists
    /// to make real has nothing saying it does anything.
    ///
    /// Each mode gets its own reference, and none of them is a second run of the
    /// composite at a different fader:
    ///
    /// ```text
    ///   add:   a half fader at gain g is bit-identical to a full fader at g/2
    ///   over:  A*(1 - o*c)              c = the card's coverage
    ///   max:   mix(A, max(A, B), o)     A and B measured on their own
    /// ```
    #[test]
    fn opacity_moves_the_mix_at_settings_between_zero_and_one() {
        let gpu = Gpu::headless().expect("no GPU available");
        const HALF: f32 = 0.5;
        const GAIN: f32 = 1.4;

        // --- `add`: **in colour**, opacity is the same multiply gain is, so it can
        // be checked against gain exactly. This is the collapse the deck's two
        // numbers used to be justified by, asserted rather than asserted about.
        //
        // Colour and not the whole texel, because the collapse stops at the alpha
        // channel: opacity scales coverage and gain does not, so the same picture
        // under the two settings carries a different coverage. That is the
        // difference between a fader and a level, showing up in the one channel
        // where `add` cannot hide it.
        let add_run = |gain: f32, opacity: f32| -> Vec<f32> {
            let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
            let mut deck = deck_of(&gpu, &[SEED_A, SEED_B]);
            deck.set_gain(1, gain);
            deck.set_opacity(1, opacity);
            for _ in 0..12 {
                frame(&gpu, &mut deck, &present, 1);
            }
            decode(&readback(&gpu, present.hdr_texture()))
        };
        let colour = |mix: &[f32]| -> Vec<f32> {
            (0..mix.len())
                .filter(|i| i % 4 != 3)
                .map(|i| mix[i])
                .collect()
        };

        let half_fader = colour(&add_run(GAIN, HALF));
        let half_gain = colour(&add_run(GAIN * HALF, 1.0));
        let full = colour(&add_run(GAIN, 1.0));
        assert_ne!(
            full,
            half_gain,
            "gain {GAIN} and gain {} render the same colours, so the comparison below is \
         vacuous",
            GAIN * HALF
        );
        assert_eq!(
            half_fader, half_gain,
            "under `add`, a fader at {HALF} is not the multiply a gain at the same factor is"
        );

        // --- `over` and `max` share the numeric comparison, and it is inexact for
        // the single reason the other numeric tests here are: `f32` on the GPU,
        // rounded once to `f16` on write, against an expectation computed in `f32`
        // from values already rounded.
        const TOLERANCE: f32 = 1.0 / 1024.0;
        let close = |x: f32, y: f32| (x - y).abs() <= TOLERANCE * (1.0 + x.abs().max(y.abs()));

        // --- `over`: the card contributes no colour, so a half fader has to leave
        // exactly half the hole a full one does.
        let over_run = |blend: Blend, opacity: f32| -> (Vec<f32>, Vec<f32>) {
            let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
            let mut deck = Deck::new(
                &gpu.device,
                vec![
                    HotSwap::fixed(build(&gpu, SEED_A, CAPACITY)),
                    HotSwap::fixed(build_with(&gpu, L4_CARD, SEED_B, CAPACITY)),
                ],
                WIDTH,
                HEIGHT,
            );
            deck.set_blend(1, blend);
            deck.set_opacity(1, opacity);
            for _ in 0..12 {
                frame(&gpu, &mut deck, &present, 1);
            }
            (
                decode(&readback(&gpu, present.hdr_texture())),
                decode(&readback(&gpu, deck.slot_target(1))),
            )
        };
        // `add` at full opacity is `A + 0`, which is `A`.
        let (bare, card) = over_run(Blend::Add, 1.0);
        let (half_hole, _) = over_run(Blend::Over, HALF);

        let mut misses = 0;
        let mut worst = 0.0f32;
        let mut moved = 0;
        for i in (0..bare.len()).filter(|i| i % 4 != 3) {
            let coverage = card[(i / 4) * 4 + 3];
            let expected = bare[i] * (1.0 - HALF * coverage);
            if !close(half_hole[i], expected) {
                misses += 1;
                worst = worst.max((half_hole[i] - expected).abs());
            }
            if !close(half_hole[i], bare[i]) {
                moved += 1;
            }
        }
        assert_eq!(
            misses,
            0,
            "under `over`, a fader at {HALF} is not `A*(1 - {HALF}*coverage)`: {misses} of {} \
         colour channels disagree, worst by {worst}",
            bare.len()
        );
        assert!(
            moved > 100,
            "a fader at {HALF} under `over` moved only {moved} colour channels, so this run \
         cannot see the fader at all"
        );

        // --- `max`: a crossfade *into* the maximum rather than a switch to it, so
        // a half fader is halfway between the layer under it and the maximum.
        let max_run = |blend: Blend, opacity: f32, off_air: Option<usize>| -> Vec<f32> {
            let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
            let mut deck = deck_of(&gpu, &[SEED_A, SEED_B]);
            deck.set_blend(1, blend);
            deck.set_opacity(1, opacity);
            if let Some(slot) = off_air {
                deck.set_residency(slot, Residency::Allocated);
            }
            for _ in 0..12 {
                frame(&gpu, &mut deck, &present, 1);
            }
            decode(&readback(&gpu, present.hdr_texture()))
        };
        let a = max_run(Blend::Add, 1.0, Some(1));
        let b = max_run(Blend::Add, 1.0, Some(0));
        let halfway = max_run(Blend::Max, HALF, None);

        let mut misses = 0;
        let mut worst = 0.0f32;
        let mut moved = 0;
        for i in (0..halfway.len()).filter(|i| i % 4 != 3) {
            let expected = a[i] + HALF * (a[i].max(b[i]) - a[i]);
            if !close(halfway[i], expected) {
                misses += 1;
                worst = worst.max((halfway[i] - expected).abs());
            }
            if !close(halfway[i], a[i]) {
                moved += 1;
            }
        }
        assert_eq!(
            misses,
            0,
            "under `max`, a fader at {HALF} is not halfway to the maximum: {misses} of {} \
         colour channels disagree, worst by {worst}",
            halfway.len()
        );
        assert!(
            moved > 100,
            "a fader at {HALF} under `max` moved only {moved} colour channels away from the \
         layer under it, so this run cannot see the fader"
        );
    }

    /// **Zero gain silences `add` and `max`, and still covers under `over`.**
    ///
    /// The other half of [`Blend::silent_at`] — the fader's half is asserted
    /// against material that has gone NaN, in
    /// `a_slot_faded_to_silence_cannot_take_the_mix_with_it`, because that is where
    /// a skip and a multiply by zero stop agreeing.
    ///
    /// Here the material is clean and the asymmetry is what is being pinned: a
    /// layer at zero level contributes no colour, so under `add` and `max` it is
    /// not there at all — and under `over` it is a black card, which covers. Slot 1
    /// is [`L4_CARD`], so that difference is most of the frame rather than a few
    /// bits.
    #[test]
    fn zero_gain_silences_add_and_max_and_still_covers_under_over() {
        let gpu = Gpu::headless().expect("no GPU available");

        let run = |residency: Residency, blend: Blend, gain: f32, opacity: f32| -> Vec<u16> {
            let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
            let mut deck = Deck::new(
                &gpu.device,
                vec![
                    HotSwap::fixed(build(&gpu, SEED_A, CAPACITY)),
                    HotSwap::fixed(build_with(&gpu, L4_CARD, SEED_B, CAPACITY)),
                ],
                WIDTH,
                HEIGHT,
            );
            deck.set_residency(1, residency);
            deck.set_blend(1, blend);
            deck.set_gain(1, gain);
            deck.set_opacity(1, opacity);
            for _ in 0..12 {
                frame(&gpu, &mut deck, &present, 1);
            }
            readback(&gpu, present.hdr_texture())
        };

        let parked = run(Residency::Allocated, Blend::Add, 1.0, 1.0);
        assert!(lit(&parked) > 100, "the surviving slot drew nothing");

        // The level silences where a layer contributing no colour contributes
        // nothing at all...
        for blend in [Blend::Add, Blend::Max] {
            assert_eq!(
                run(Residency::Live, blend, 0.0, 1.0),
                parked,
                "a slot at gain 0.0 under `{}` reached the mix",
                blend.name()
            );
        }
        // ...and does not under `over`, where zero gain is a black card and a black
        // card covers. Asserted rather than left as a comment, because it is the
        // one place the two faders stop being interchangeable and an operator
        // reaching for the wrong one gets a frame that goes dark instead of a
        // layer that goes away.
        //
        // **Colour channels only.** A whole-buffer `assert_ne!` would pass on the
        // alpha channel alone — coverage composes whatever the colour mode does, so
        // a slot that reached the mix and changed nothing visible still moves it —
        // and this claim is about what the picture does.
        let dark = decode(&run(Residency::Live, Blend::Over, 0.0, 1.0));
        let bright = decode(&parked);
        let darkened = (0..dark.len())
            .filter(|i| i % 4 != 3)
            .filter(|&i| dark[i] < bright[i])
            .count();
        assert!(
            darkened > 100,
            "a zero-gain `over` layer darkened only {darkened} colour channels, so it has \
         stopped covering — which would make gain and opacity the same control again"
        );
    }

    /// **Two slots at gain 1.0 and 0.0 render what slot 0 alone renders.**
    ///
    /// Exact, for the same reason as above with one addition: `acc + 0.0 * x` is
    /// `acc` for every finite `x`, so a silenced slot contributes nothing at all
    /// rather than something below a threshold.
    ///
    /// Note what is *not* silenced: the slot is still `Live`, so it is still
    /// stepped and still rendered into its own target. Gain is a mixer fader, not
    /// a residency level, and conflating the two is how a fader move would come to
    /// cost a simulation.
    #[test]
    fn a_slot_at_zero_gain_contributes_nothing_to_the_mix() {
        let gpu = Gpu::headless().expect("no GPU available");

        let alone_present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
        let mut alone = deck_of(&gpu, &[SEED_A]);
        for _ in 0..12 {
            frame(&gpu, &mut alone, &alone_present, 1);
        }
        let expected = readback(&gpu, alone_present.hdr_texture());

        let both_present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
        let mut both = deck_of(&gpu, &[SEED_A, SEED_B]);
        both.set_gain(1, 0.0);
        for _ in 0..12 {
            frame(&gpu, &mut both, &both_present, 1);
        }
        let mixed = readback(&gpu, both_present.hdr_texture());

        assert_eq!(
            mixed, expected,
            "a slot at zero gain reached the mix anyway"
        );
        // The silenced slot ran regardless: gain is not residency.
        assert_eq!(steps_taken(both.slot(1).set()), 12);
        assert_eq!(both.live_slots(), 2);
    }

    /// **Each Live slot renders into its own HDR target**, and the mix is a sum of
    /// exactly those targets.
    ///
    /// This is the decision the module doc defends — additive-only would allow one
    /// shared target, blend modes and masks will not — so it is worth an assertion
    /// rather than only a comment. Slot 0's own target is checked against what a
    /// deck of one holding the same Set mixes, which is the same picture by
    /// definition if and only if the slot rendered alone into somewhere of its
    /// own; two Sets sharing a target would have summed there instead, and slot
    /// 0's target would hold the sum.
    #[test]
    fn each_live_slot_renders_into_its_own_target() {
        let gpu = Gpu::headless().expect("no GPU available");
        let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
        let mut deck = deck_of(&gpu, &[SEED_A, SEED_B]);
        for _ in 0..12 {
            frame(&gpu, &mut deck, &present, 1);
        }
        let first = readback(&gpu, deck.slot_target(0));
        let second = readback(&gpu, deck.slot_target(1));

        assert!(
            lit(&first) > 100 && lit(&second) > 100,
            "a slot drew nothing"
        );
        assert_ne!(
            first, second,
            "two slots at different seeds hold the same target contents"
        );

        let solo_present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
        let mut solo = deck_of(&gpu, &[SEED_A]);
        for _ in 0..12 {
            frame(&gpu, &mut solo, &solo_present, 1);
        }
        assert_eq!(
            first,
            readback(&gpu, solo_present.hdr_texture()),
            "slot 0's target holds something other than slot 0's own render"
        );
    }

    /// A resize reallocates every slot target and rebinds the mix, and the result
    /// is the deck it would have been at that size all along.
    ///
    /// The failure this catches is a bind group left pointing at the old,
    /// differently sized textures: `textureLoad` out of range is defined to return
    /// zero rather than to fault, so the symptom would be a mix that is correct in
    /// one corner and black everywhere else. Silent, and only visible on a window
    /// that has been dragged.
    #[test]
    fn resizing_the_deck_reallocates_and_rebinds() {
        // 512 * 8 bytes is a 256-byte-aligned row, which the readback needs, and a
        // different aspect ratio from 256x256, which puts the Sets' cameras on the
        // new viewport as well as the targets.
        const WIDE: u32 = 512;
        const TALL: u32 = 256;

        let gpu = Gpu::headless().expect("no GPU available");

        let grown_present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDE, TALL);
        let mut grown = deck_of(&gpu, &[SEED_A, SEED_B]);
        grown.resize(&gpu.device, WIDE, TALL);
        for _ in 0..12 {
            frame(&gpu, &mut grown, &grown_present, 1);
        }
        let after_resize = readback(&gpu, grown_present.hdr_texture());

        let native_present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDE, TALL);
        let mut native = deck_of_at(&gpu, &[SEED_A, SEED_B], WIDE, TALL);
        for _ in 0..12 {
            frame(&gpu, &mut native, &native_present, 1);
        }

        assert!(lit(&after_resize) > 100, "the resized deck drew nothing");
        assert_eq!(
            after_resize,
            readback(&gpu, native_present.hdr_texture()),
            "a resized deck is not the deck it would have been at that size"
        );
        assert_eq!(grown.slot_target(0).width(), WIDE);
        assert_eq!(grown.slot_target(1).height(), TALL);
    }

    /// **Gain is linear, and applied per slot before the sum rather than to the
    /// sum.**
    ///
    /// The two are only distinguishable with more than one slot at more than one
    /// gain, and they differ by an entire slot's contribution:
    ///
    /// ```text
    ///   before (what this asserts):   2*A + B
    ///   after  (what it must not be): 2*(A + B)
    /// ```
    ///
    /// So `A` and `B` are measured on their own — same deck, same seeds, same
    /// ticks, one slot silenced each time, so both are sampled at the same `t` as
    /// the mix is — and the mix is checked against the first expression and
    /// against the second.
    ///
    /// This is the one comparison here that cannot be exact. The GPU sums in `f32`
    /// and rounds once, to `f16`, on write; the expectation is computed in `f32`
    /// from values that are already `f16`. The gap is that single rounding, which
    /// is 2^-11 relative, so the tolerance is 2^-10 — tight enough that the
    /// alternative hypothesis misses it by three orders of magnitude, which the
    /// second half of the test asserts rather than assumes.
    #[test]
    fn gain_is_linear_and_applied_before_the_composite() {
        let gpu = Gpu::headless().expect("no GPU available");
        const STEPS: usize = 12;

        let run = |gains: [f32; 2]| -> Vec<f32> {
            let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
            let mut deck = deck_of(&gpu, &[SEED_A, SEED_B]);
            deck.set_gain(0, gains[0]);
            deck.set_gain(1, gains[1]);
            for _ in 0..STEPS {
                frame(&gpu, &mut deck, &present, 1);
            }
            decode(&readback(&gpu, present.hdr_texture()))
        };

        let a = run([1.0, 0.0]);
        let b = run([0.0, 1.0]);
        let mixed = run([2.0, 1.0]);

        assert!(
            a.iter().all(|v| v.is_finite()) && b.iter().all(|v| v.is_finite()),
            "a slot rendered an infinity, which makes the arithmetic below meaningless"
        );

        // One f16 rounding of the result, and nothing else.
        const TOLERANCE: f32 = 1.0 / 1024.0;
        let close = |x: f32, y: f32| (x - y).abs() <= TOLERANCE * (1.0 + x.abs().max(y.abs()));

        let mut before_misses = 0;
        let mut after_misses = 0;
        let mut worst = 0.0f32;
        // **Colour only.** The fourth channel is coverage rather than a colour —
        // `1 - prod(1 - a_i)`, composed as `over` under every blend mode — and gain
        // deliberately does not reach it: turning a layer's level down dims what it
        // draws and does not change what it covers. So alpha is neither `2*A + B`
        // nor `2*(A + B)`, and including it here would be asserting linearity of a
        // channel this deck promises is not linear.
        for i in (0..mixed.len()).filter(|i| i % 4 != 3) {
            let before = 2.0 * a[i] + b[i];
            let after = 2.0 * (a[i] + b[i]);
            if !close(mixed[i], before) {
                before_misses += 1;
                worst = worst.max((mixed[i] - before).abs());
            }
            if !close(mixed[i], after) {
                after_misses += 1;
            }
        }

        assert_eq!(
            before_misses,
            0,
            "the mix is not 2*A + B: {before_misses} of {} channels disagree, worst by {worst}",
            mixed.len()
        );
        // The second half of the claim. Without this the first half would also
        // pass on an all-black frame, or on one where B never contributed
        // anything — in either case `2*A + B` and `2*(A + B)` are the same number
        // and the test would be asserting nothing.
        assert!(
            after_misses > 100,
            "only {after_misses} channels distinguish `2*A + B` from `2*(A + B)`, so this run \
         could not have detected gain being applied to the mix instead of to the slot"
        );
    }

    /// A frame the way a sink sees it: the canvas through the present pass —
    /// tone mapped, then sRGB encoded by the hardware — into a target of the
    /// canvas's own size, so `Present::draw`'s letterbox is the whole
    /// attachment exactly as it is on every offscreen render.
    ///
    /// Only the master-out tests below need it. Everything else in this file
    /// reads the HDR target, because everything else is about the mix; these
    /// are about the difference between the mix's output and the picture drawn
    /// from it, and that difference is this pass.
    fn shown(gpu: &Gpu, present: &Present) -> Vec<u8> {
        let target = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("shown"),
            size: wgpu::Extent3d {
                width: WIDTH,
                height: HEIGHT,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = target.create_view(&Default::default());

        let bytes_per_row = WIDTH * 4;
        assert_eq!(bytes_per_row % 256, 0, "row pitch must be 256-byte aligned");
        let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("shown readback"),
            size: u64::from(bytes_per_row * HEIGHT),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        present.draw(&mut encoder, &view, (WIDTH, HEIGHT));
        encoder.copy_texture_to_buffer(
            target.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
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

        let slice = buffer.slice(..);
        slice.map_async(wgpu::MapMode::Read, |r| r.expect("map"));
        gpu.device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("poll");
        let data = slice.get_mapped_range().expect("map");
        let out = data.to_vec();
        drop(data);
        buffer.unmap();
        out
    }

    /// One run of a fixed deck under one master out and one exposure: the
    /// composited frame as written, and the picture drawn from it.
    ///
    /// A `Present` per run, at a **window's** surface format rather than
    /// [`Present::HDR_FORMAT`] — every other test in this file renders and
    /// never draws, so this is the only place the encode's own target has to
    /// exist. The HDR target is the same either way; the format only decides
    /// what `Present::draw` may be pointed at.
    fn under(gpu: &Gpu, out: f32, exposure: f32) -> (Vec<u16>, Vec<u8>) {
        const STEPS: usize = 12;
        let present = Present::new(
            &gpu.device,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            WIDTH,
            HEIGHT,
        );
        present.set_tonemap(&gpu.queue, karakuri_engine::TonemapOp::Aces, exposure, 1.0);
        let mut deck = deck_of(gpu, &[SEED_A, SEED_B]);
        deck.set_out(out);
        for _ in 0..STEPS {
            frame(gpu, &mut deck, &present, 1);
        }
        (readback(gpu, present.hdr_texture()), shown(gpu, &present))
    }

    /// How many values two readbacks of the same shape disagree on. Counted
    /// rather than reported as a boolean, because every assertion below is
    /// about *how much* of the frame moved: one texel differing is a driver
    /// having a bad day and a hundred thousand is a level.
    fn differing<T: PartialEq>(a: &[T], b: &[T]) -> usize {
        assert_eq!(a.len(), b.len(), "two readbacks of different shapes");
        a.iter().zip(b).filter(|(x, y)| x != y).count()
    }

    /// **The master out is a level on the composited frame, and coverage is not
    /// a level.**
    ///
    /// Half the master out is half the colour, exactly: the mix folds in `f32`
    /// and rounds once on write, and scaling by a power of two commutes with
    /// that rounding for every normal `f16`, so the tolerance here is one
    /// subnormal step rather than a relative one. The fourth channel is
    /// coverage — `1 - prod(1 - a_i)`, composed as `over` under every blend
    /// mode — and it is asserted **bit for bit unchanged**, which is the same
    /// asymmetry `gain` has: turning a level down dims what a frame draws and
    /// does not change what it covers.
    #[test]
    fn the_master_out_scales_the_composited_frame_and_leaves_its_coverage_alone() {
        let gpu = Gpu::headless().expect("no GPU available");

        let (full, _) = under(&gpu, 1.0, 1.0);
        let (half, _) = under(&gpu, 0.5, 1.0);

        assert!(
            lit(&full) > 100,
            "the deck drew nothing, so this test would pass on two black frames"
        );
        let bright = decode(&full);
        let brightest = (0..bright.len())
            .filter(|i| i % 4 != 3)
            .fold(0.0f32, |m, i| m.max(bright[i]));
        assert!(
            brightest > 1.0,
            "the deck peaked at {brightest}, so the master out was only checked below 1.0 — \
             the range this pipeline is HDR for is untested"
        );

        // Half of a normal `f16` is exact; a value already in the subnormal
        // range can round by half a subnormal ulp, which is 2^-25.
        const TOLERANCE: f32 = 1e-7;
        let scaled = decode(&half);
        let mut misses = 0;
        let mut worst = 0.0f32;
        for i in (0..scaled.len()).filter(|i| i % 4 != 3) {
            let want = 0.5 * bright[i];
            if (scaled[i] - want).abs() > TOLERANCE {
                misses += 1;
                worst = worst.max((scaled[i] - want).abs());
            }
        }
        assert_eq!(
            misses, 0,
            "the master out is not a multiply on the folded colour: {misses} channels disagree, \
             worst by {worst}"
        );

        // Both halves of the claim need a witness. Without this one, a master
        // out that did nothing at all would satisfy the arithmetic above on
        // every black texel and be caught only where the frame is lit.
        let moved = (0..scaled.len())
            .filter(|i| i % 4 != 3 && scaled[*i] != bright[*i])
            .count();
        assert!(
            moved > 100,
            "only {moved} colour channels moved when the master out was halved, so this run \
             could not have detected it being ignored"
        );

        let coverage: Vec<u16> = full.iter().skip(3).step_by(4).copied().collect();
        let after: Vec<u16> = half.iter().skip(3).step_by(4).copied().collect();
        assert_eq!(
            differing(&coverage, &after),
            0,
            "the master out reached the alpha channel, which is coverage rather than a level"
        );
    }

    /// **The two levels multiply in different places, and this is where the
    /// difference is visible today.** The master out is applied where the mix
    /// *writes* the composited frame; the tone mapper's exposure is applied
    /// where the present pass *reads* it. So halving the master out moves the
    /// HDR target and halving the exposure leaves it bit for bit identical —
    /// while both move the picture drawn from it.
    ///
    /// That last assertion is the one that keeps this test honest. Without it,
    /// an exposure that had been quietly disconnected would pass the middle
    /// assertion perfectly, and the test would be reporting "the two are in
    /// different places" on the strength of one of them doing nothing at all.
    ///
    /// **This is the whole of what `docs/adr/0224-out-and-exposure-are-two-levels-that-multiply-in-different-places.md`
    /// can assert until the master chain exists**, and it is enough: a build
    /// that folded the two into one multiplication — either of the rejected
    /// options — fails here, whichever end it folded them at.
    #[test]
    fn the_master_out_is_at_the_chains_entry_and_exposure_is_at_the_tonemaps_input() {
        let gpu = Gpu::headless().expect("no GPU available");

        let (base_frame, base_shown) = under(&gpu, 1.0, 1.0);
        let (out_frame, out_shown) = under(&gpu, 0.5, 1.0);
        let (exposed_frame, exposed_shown) = under(&gpu, 1.0, 0.5);

        assert!(
            lit(&base_frame) > 100,
            "the deck drew nothing, so every comparison below is between two black frames"
        );

        let by_out = differing(&base_frame, &out_frame);
        assert!(
            by_out > 100,
            "the master out moved only {by_out} of {} values in the composited frame, so it is \
             not being applied where that frame is written",
            base_frame.len()
        );
        assert_eq!(
            differing(&base_frame, &exposed_frame),
            0,
            "the tone mapper's exposure changed the composited frame, so it is being applied \
             upstream of where the present pass reads it"
        );

        let shown_by_exposure = differing(&base_shown, &exposed_shown);
        assert!(
            shown_by_exposure > 100,
            "the exposure moved only {shown_by_exposure} of {} bytes of the picture, so the \
             frame it left untouched proves nothing",
            base_shown.len()
        );
        let shown_by_out = differing(&base_shown, &out_shown);
        assert!(
            shown_by_out > 100,
            "the master out moved only {shown_by_out} of {} bytes of the picture",
            base_shown.len()
        );
    }

    /// **The cost this separation was taken at, held against the code.**
    ///
    /// With nothing between the two multiplications, a master out of 0.5 and an
    /// exposure of 0.5 produce the same picture: the master chain the two
    /// levels are the ends of is empty, so `out` then `exposure` is one product
    /// and the order of it is not observable. The maintainer chose that cost
    /// over the other one — a single value costs nothing today and costs the
    /// separation the day a master effect lands between them.
    ///
    /// **This test is deleted the day the chain has an effect in it**, not
    /// weakened: it asserts today's cost rather than a property worth keeping,
    /// and a feedback pass between the two ends makes it false on purpose. That
    /// is what makes it the tripwire — the ADR's undone work fails a test
    /// instead of going quiet.
    ///
    /// **A byte rather than nothing**, because the two paths round differently
    /// in principle: the master out is stored to `f16` before the exposure
    /// reads it, so a texel already in the subnormal range can round on the way
    /// through. On this machine (Metal, 2026-08-30) the two pictures are
    /// **byte-identical** — the difference is 0 and the allowance is unused.
    #[test]
    fn with_nothing_in_the_master_chain_the_two_levels_are_the_same_picture() {
        let gpu = Gpu::headless().expect("no GPU available");

        let (_, base_shown) = under(&gpu, 1.0, 1.0);
        let (_, out_shown) = under(&gpu, 0.5, 1.0);
        let (_, exposed_shown) = under(&gpu, 1.0, 0.5);

        let moved = differing(&base_shown, &out_shown);
        assert!(
            moved > 100,
            "neither level changed the picture, so the agreement below is between two frames \
             nothing happened to"
        );

        let worst = out_shown
            .iter()
            .zip(&exposed_shown)
            .map(|(a, b)| a.abs_diff(*b))
            .max()
            .expect("a picture has bytes in it");
        let differ = differing(&out_shown, &exposed_shown);
        assert!(
            worst <= 1,
            "a master out of 0.5 and an exposure of 0.5 drew different pictures — {differ} bytes \
             apart, worst by {worst}. Either something now sits between the two multiplications, \
             in which case this test has done its job and goes, or one of them is not a level."
        );
    }

    /// **`Allocated` keeps its state.** A slot taken off air does not advance while
    /// it is off, and resumes where it stopped when it comes back — it does not
    /// restart, and it does not quietly catch up.
    ///
    /// `t` is the sharpest witness available: simulation time only moves through
    /// `Set::prepare`, and a slot that is not `Live` is never handed one. That is
    /// the same property `swap.rs` already leans on to park an outgoing Set across
    /// a watchdog window, which is why it costs nothing to have here.
    ///
    /// This is two of the three residency levels. `Priming` — stepping
    /// hidden at a reduced rate — is the next slice and needs the budget governor
    /// to be worth having.
    #[test]
    fn a_slot_taken_off_air_keeps_its_t_and_resumes_where_it_stopped() {
        let gpu = Gpu::headless().expect("no GPU available");
        let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
        let mut deck = deck_of(&gpu, &[SEED_A, SEED_B]);

        for _ in 0..10 {
            frame(&gpu, &mut deck, &present, 1);
        }
        assert_eq!(steps_taken(deck.slot(0).set()), 10);
        assert_eq!(steps_taken(deck.slot(1).set()), 10);

        deck.set_residency(1, Residency::Allocated);
        assert_eq!(deck.live_slots(), 1);
        assert_eq!(deck.slot_count(), 2, "going off air does not free the slot");

        // Substepped while it is away, so that "did not advance" is a claim about
        // steps rather than about frames.
        for _ in 0..10 {
            frame(&gpu, &mut deck, &present, 3);
        }
        assert_eq!(
            steps_taken(deck.slot(0).set()),
            40,
            "the on-air slot did not step normally while the other was parked"
        );
        assert_eq!(
            steps_taken(deck.slot(1).set()),
            10,
            "an Allocated slot advanced: something is calling `prepare` on it"
        );

        // What the mix shows while it is away is what the remaining slot shows.
        let off_air = readback(&gpu, present.hdr_texture());
        let solo_present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
        let mut solo = deck_of(&gpu, &[SEED_A]);
        for _ in 0..10 {
            frame(&gpu, &mut solo, &solo_present, 1);
        }
        for _ in 0..10 {
            frame(&gpu, &mut solo, &solo_present, 3);
        }
        assert_eq!(
            off_air,
            readback(&gpu, solo_present.hdr_texture()),
            "an Allocated slot was still reaching the mix"
        );

        deck.set_residency(1, Residency::Live);
        for _ in 0..5 {
            frame(&gpu, &mut deck, &present, 1);
        }
        assert_eq!(
            steps_taken(deck.slot(1).set()),
            15,
            "the returning slot did not resume from where it was parked"
        );
        assert_eq!(steps_taken(deck.slot(0).set()), 45);
        assert_eq!(deck.live_slots(), 2);
    }

    // ---------------------------------------------------------------------------
    // P-0080: an operator can see a slot's own material without putting it on air
    //
    // Three tests, one per case the requirement names, each asserting on the
    // slot's own target — `Deck::slot_target` — because that is the texture a
    // console cell samples through `Deck::slot_view`. What the mix does with the
    // slot is a separate question and is asserted separately in each.
    // ---------------------------------------------------------------------------

    /// **A running slot's own target holds its own texels, with no fader on
    /// them.**
    ///
    /// The first of P-0080's three clauses, and the one that decides where a
    /// monitor may sample from. `gain`, `opacity`, `blend` and `mask` are edge
    /// properties applied in `Composite`, so a slot's target is upstream of all
    /// four — which is what lets a cell show *the level the material arrives at*
    /// rather than the level the operator has already set.
    ///
    /// Asserted as bit equality between a slot faded to silence and the same slot
    /// at unity, in the same deck on the same ticks. It is exact because nothing
    /// between `Set::draw` and the readback rounds; "close enough" here would
    /// tolerate a fader that had leaked upstream by a hair.
    ///
    /// **And the mix is checked to differ**, or the whole thing would pass with
    /// the fader deleted: two identical slot targets prove nothing if the fader
    /// was never applied anywhere.
    #[test]
    fn a_running_slot_shows_its_own_texels_with_no_fader_on_them() {
        let gpu = Gpu::headless().expect("no GPU available");

        let run = |gain: f32, opacity: f32, mask: Mask| -> (Vec<u16>, Vec<u16>) {
            let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
            let mut deck = deck_of(&gpu, &[SEED_A, SEED_B]);
            deck.set_gain(1, gain);
            deck.set_opacity(1, opacity);
            deck.set_mask(1, mask);
            for _ in 0..8 {
                frame(&gpu, &mut deck, &present, 1);
            }
            (
                readback(&gpu, deck.slot_target(1)),
                readback(&gpu, present.hdr_texture()),
            )
        };

        let (open, open_mix) = run(1.0, 1.0, Mask::default());
        assert!(
            lit(&open) > 100,
            "the slot drew nothing, so this test is asserting nothing"
        );

        for (gain, opacity, mask, what) in [
            (0.0, 1.0, Mask::default(), "gain at silence"),
            (1.0, 0.0, Mask::default(), "opacity at silence"),
            (0.25, 0.5, Mask::default(), "both faders part way down"),
            (
                1.0,
                1.0,
                Mask::new(MaskKind::Linear, 0.0, 0.5, 0.0),
                "a linear mask half across",
            ),
        ] {
            let (own, mix) = run(gain, opacity, mask);
            assert_eq!(
                own, open,
                "{what} changed what the slot drew into its own target — a fader is an \
                 edge property and belongs in the composite, and a cell sampling this \
                 texture would be showing the operator the level they already set"
            );
            assert_ne!(
                mix, open_mix,
                "{what} left the mix unchanged, so the comparison above is between two \
                 settings neither of which does anything"
            );
        }
    }

    /// **An off-air slot is drawn into its own target every frame, and never
    /// stepped.**
    ///
    /// P-0080's second and third clauses together: *whatever its residency*, and
    /// *without changing it*. The slot an operator most needs to look at is the
    /// one that is not on air yet, and the look may not move it
    /// ([P-0082](../../../docs/principles/0082-looking-never-writes-back.md)).
    ///
    /// **The resize is what makes this a test of *this* frame's draw.** A slot
    /// target persists, so a parked slot that was Live a moment ago keeps the last
    /// picture in it and "the target is lit" would pass with the draw deleted —
    /// that is ADR-0072's own correction to itself, made after it shipped the
    /// wrong reason. `Deck::resize` reallocates every target, so the frame after
    /// one is the only frame on which the target's contents can only have come
    /// from a draw recorded on it.
    ///
    /// Both off-air levels are covered, because they are two branches:
    /// `Allocated` draws and never steps, `Priming` steps on the governor's rate
    /// and draws on every frame regardless.
    #[test]
    fn an_off_air_slot_is_drawn_every_frame_and_never_stepped() {
        let gpu = Gpu::headless().expect("no GPU available");

        for residency in [Residency::Allocated, Residency::Priming] {
            let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
            let mut deck = deck_of(&gpu, &[SEED_A, SEED_B]);

            // Live first, so the slot has element state to draw. A Set that has
            // never stepped draws black, which is the case the next test is
            // about and would make this one unable to tell a draw from no draw.
            for _ in 0..8 {
                frame(&gpu, &mut deck, &present, 1);
            }
            deck.set_residency(1, residency);
            // One in a thousand, so that across the frames below the Priming
            // slot steps on the first and on none of the rest — the draw has to
            // be there on the frames it does not step.
            deck.set_prime_one_in(1, 1000);
            for _ in 0..4 {
                frame(&gpu, &mut deck, &present, 1);
            }
            let parked_at = steps_taken(deck.slot(1).set());

            // **The target is thrown away and remade**, so nothing in it can be
            // left over from when the slot was Live.
            deck.resize(&gpu.device, WIDTH / 2, HEIGHT / 2);
            let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH / 2, HEIGHT / 2);
            assert_eq!(
                lit(&readback(&gpu, deck.slot_target(1))),
                0,
                "the reallocated target came back with something in it, so the assertion \
                 below cannot tell a fresh draw from a stale one"
            );

            frame(&gpu, &mut deck, &present, 1);
            assert!(
                lit(&readback(&gpu, deck.slot_target(1))) > 100,
                "a slot at {residency:?} drew nothing into its own target on the frame \
                 after a resize, so its console cell is dark at exactly the moment an \
                 operator is deciding whether to bring the slot up (P-0080)"
            );

            // And the look did not move it. `Allocated` may not have stepped at
            // all; `Priming` may have stepped only on the frames the governor's
            // rate allows, which at one in a thousand is the frame it entered
            // Priming and no other.
            let after = steps_taken(deck.slot(1).set());
            match residency {
                Residency::Allocated => assert_eq!(
                    after, parked_at,
                    "drawing an Allocated slot advanced it — looking never writes back \
                     (P-0082)"
                ),
                _ => assert_eq!(
                    after, parked_at,
                    "a Priming slot stepped on a frame its rate did not allow, so the \
                     draw is stepping it"
                ),
            }

            // The mix is still only the Live slot's, which is the other half:
            // drawn is not mixed.
            let mixed = readback(&gpu, present.hdr_texture());
            let solo_present =
                Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH / 2, HEIGHT / 2);
            let mut solo = deck_of_at(&gpu, &[SEED_A], WIDTH / 2, HEIGHT / 2);
            for _ in 0..13 {
                frame(&gpu, &mut solo, &solo_present, 1);
            }
            assert_eq!(
                mixed,
                readback(&gpu, solo_present.hdr_texture()),
                "a slot at {residency:?} reached the mix — being drawn is not being mixed"
            );
        }
    }

    /// **A slot whose build was rejected shows what is still running, and a slot
    /// with nothing behind the draw shows black.**
    ///
    /// The two failure cases P-0080 has to answer honestly, and the answer is not
    /// the same for both because the states are not the same.
    ///
    /// **A rejected build changes nothing** — `swap.rs` is explicit that the
    /// running Set keeps running, with its `t` and its live count untouched — so
    /// the honest picture is the material that is still there, and it is drawn on
    /// the frame after the rejection exactly as on the frame before. Anything else
    /// would be the cell inventing a state the deck is not in. What says a build
    /// was refused is `Event::Rejected`, which the Staging lane draws; the cell's
    /// job is the picture.
    ///
    /// **A Set that has never stepped has no element state**, so its draw is a
    /// pass over zeroed buffers: every element at the origin, which comes out as
    /// a handful of texels in the middle of an otherwise black frame. Near-black
    /// rather than exactly black, and the number is asserted against the Live
    /// neighbour on the same frame rather than against a constant, because what
    /// matters is that it carries no material. That is the cold end of a slot and
    /// it is why priming exists — take a candidate down, warm it, look at it.
    ///
    /// **Black-because-drawn and black-because-nothing-drew are told apart by the
    /// resize.** `Deck::resize` reallocates the target and wgpu hands it back
    /// zeroed, so the count is checked at zero *before* the frame and above zero
    /// after it: the pass ran, and what it put there is the cold Set's own answer
    /// rather than a leftover.
    #[test]
    fn a_rejected_build_shows_what_is_still_running_and_a_cold_slot_shows_black() {
        let gpu = Gpu::headless().expect("no GPU available");
        let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);

        // Slot 1 takes builds and is off air; slot 0 is the Live neighbour that
        // proves a frame was recorded at all.
        let _ = present;
        let (tx, rx) = mpsc::channel();
        let mut deck = Deck::new(
            &gpu.device,
            vec![
                HotSwap::fixed(build(&gpu, SEED_A, CAPACITY)),
                HotSwap::new(
                    &gpu.device,
                    &gpu.queue,
                    build(&gpu, SEED_B, CAPACITY),
                    GENEROUS_MS,
                    Box::new(rx),
                ),
            ],
            WIDTH,
            HEIGHT,
        );

        // --- the cold slot -------------------------------------------------
        // Off air from the first frame, so it has never stepped.
        deck.set_residency(1, Residency::Allocated);
        deck.resize(&gpu.device, WIDTH / 2, HEIGHT / 2);
        let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH / 2, HEIGHT / 2);
        assert_eq!(
            lit(&readback(&gpu, deck.slot_target(1))),
            0,
            "the reallocated target came back with something in it, so nothing below \
             can tell a fresh draw from a stale one"
        );
        frame(&gpu, &mut deck, &present, 1);
        assert_eq!(
            steps_taken(deck.slot(1).set()),
            0,
            "the cold slot stepped, so it is not cold and this half asserts nothing"
        );
        let cold = lit(&readback(&gpu, deck.slot_target(1)));
        let neighbour = lit(&readback(&gpu, deck.slot_target(0)));
        assert!(
            neighbour > 100,
            "the Live neighbour is dark too, so the frame recorded nothing at all and \
             nothing below is the cold slot's own answer"
        );
        assert!(
            cold > 0,
            "no pass was recorded for the cold slot: its target is exactly what the \
             resize left, so its cell would be showing a reallocation rather than a Set"
        );
        assert!(
            cold * 50 < neighbour,
            "a Set that has never stepped drew {cold} lit texels against the neighbour's \
             {neighbour}: it has no element state, so anything approaching material here \
             did not come from the material"
        );

        // --- the rejected build --------------------------------------------
        // Warm the slot first, so there is a picture for a rejection to leave
        // alone: `Priming` steps it out of the room.
        deck.set_residency(1, Residency::Priming);
        for _ in 0..8 {
            frame(&gpu, &mut deck, &present, 1);
        }
        let warmed = steps_taken(deck.slot(1).set());
        assert!(
            warmed > 0,
            "the slot did not warm, so there is nothing to keep"
        );
        deck.set_residency(1, Residency::Allocated);
        frame(&gpu, &mut deck, &present, 1);
        let before = readback(&gpu, deck.slot_target(1));
        assert!(lit(&before) > 100, "the warmed slot drew nothing");

        // A capacity outside the L1's declared range: the worker builds it and
        // `Set::build` refuses, which is `Event::Rejected` and not a swap.
        const REFUSED: u32 = 1;
        tx.send(Request {
            names: karakuri_engine::swap::RequestNames::default(),
            edges: Vec::new(),
            id: 1,
            l1s: vec![(compile(L1), REFUSED)],
            l2s: Vec::new(),
            l3s: Vec::new(),
            fields: Vec::new(),
            layering: karakuri_engine::set::Layering::Overdraw,
            live: None,
            published: Vec::new(),
            l4s: vec![compile(L4)],
            seed_salt: SEED_B,
            camera: karakuri_engine::camera::Orbit::default(),
            salts: Vec::new(),
            params: Vec::new(),
            bindings: Vec::new(),
            authorities: Vec::new(),
            label: "slot 1, refused".to_string(),
        })
        .expect("worker alive");

        let started = Instant::now();
        let mut rejected = false;
        while !rejected {
            frame(&gpu, &mut deck, &present, 1);
            for event in deck.events(1) {
                match event {
                    Event::Rejected { .. } => rejected = true,
                    other => panic!("the refused build did not come back as a rejection: {other}"),
                }
            }
            assert!(
                started.elapsed() < PATIENCE,
                "waited {PATIENCE:?} for the rejection and it never arrived"
            );
        }

        assert_eq!(
            steps_taken(deck.slot(1).set()),
            warmed,
            "the rejected build moved the running Set's clock"
        );
        // The target is thrown away, so what comes back can only be this
        // frame's draw of the Set that survived the rejection.
        deck.resize(&gpu.device, WIDTH, HEIGHT);
        let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
        frame(&gpu, &mut deck, &present, 1);
        assert!(
            lit(&readback(&gpu, deck.slot_target(1))) > 100,
            "a slot whose build was refused went dark, so the cell says the material is \
             gone when nothing changed at all — the running Set is still running"
        );
    }

    /// **The same tick sequence and the same seeds composite to the same pixels.**
    ///
    /// The ticks are deliberately uneven. A run of identical steps would pass even
    /// if the deck were advancing slots by whatever each one felt like, since they
    /// would all feel like the same thing; varying `steps` frame to frame is what
    /// makes "every Live slot advances by the same `steps` from the same tick" the
    /// thing being tested.
    ///
    /// Bit equality, not similarity. Floating-point addition is not associative,
    /// so a composite whose order depended on a `HashMap`, or on which slot last
    /// had a build land on it, would show up here — which is the whole reason the
    /// order is the slot index and the shader's sum is unrolled.
    #[test]
    fn the_same_ticks_and_seeds_composite_bit_identically() {
        let gpu = Gpu::headless().expect("no GPU available");
        const TICKS: [u8; 12] = [1, 2, 1, 3, 1, 1, 4, 2, 1, 3, 2, 1];

        let run = || -> Vec<u16> {
            let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
            let mut deck = deck_of(&gpu, &[SEED_A, SEED_B, SEED_A + 1]);
            deck.set_gain(0, 1.5);
            deck.set_gain(1, 0.75);
            deck.set_opacity(2, 0.5);
            for steps in TICKS {
                frame(&gpu, &mut deck, &present, steps);
            }
            readback(&gpu, present.hdr_texture())
        };

        let first = run();
        let second = run();
        assert!(lit(&first) > 100, "the deck drew nothing to compare");
        assert_eq!(
            first, second,
            "two runs of the same ticks and the same seeds composited differently"
        );
    }

    /// **Per-slot hot swap still works, and a swap in one slot does not disturb
    /// another.**
    ///
    /// A slot is the unit that gets replaced — that is what it means for each slot
    /// to own its own `HotSwap` rather than for the deck to own one over all of
    /// them. The neighbouring slot must come through with its `t`, its element
    /// buffers and its live count untouched, exactly as the running Set does
    /// through a failed build in `tests/hot_swap.rs`.
    #[test]
    fn a_swap_in_one_slot_leaves_the_other_slot_alone() {
        let gpu = Gpu::headless().expect("no GPU available");
        let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);

        let (tx, rx) = mpsc::channel();
        let swapping = HotSwap::new(
            &gpu.device,
            &gpu.queue,
            build(&gpu, SEED_A, CAPACITY),
            GENEROUS_MS,
            Box::new(rx),
        );
        let steady = HotSwap::fixed(build(&gpu, SEED_B, CAPACITY));
        let mut deck = Deck::new(&gpu.device, vec![swapping, steady], WIDTH, HEIGHT);

        let mut frames = 0u64;
        for _ in 0..10 {
            frame(&gpu, &mut deck, &present, 1);
            frames += 1;
        }
        let neighbour_live_before = deck.slot(1).set().live_count(&gpu.device, &gpu.queue);

        tx.send(Request {
            names: karakuri_engine::swap::RequestNames::default(),
            edges: Vec::new(),
            id: 1,
            l1s: vec![(compile(L1), SWAPPED)],
            l2s: Vec::new(),
            l3s: Vec::new(),
            fields: Vec::new(),
            layering: karakuri_engine::set::Layering::Overdraw,
            live: None,
            published: Vec::new(),
            l4s: vec![compile(L4)],
            seed_salt: SEED_A,
            camera: karakuri_engine::camera::Orbit::default(),
            salts: Vec::new(),
            params: Vec::new(),
            bindings: Vec::new(),
            authorities: Vec::new(),
            label: "slot 0, second".to_string(),
        })
        .expect("worker alive");

        let started = Instant::now();
        let mut swapped = false;
        while !swapped {
            frame(&gpu, &mut deck, &present, 1);
            frames += 1;
            swapped = deck.events(0).any(|e| matches!(e, Event::Swapped { .. }));
            assert!(
                started.elapsed() < PATIENCE,
                "waited {PATIENCE:?} for the swap and it never landed"
            );
        }
        // Frames kept coming while the build was in flight, which is what "the
        // worker does not block the render loop" looks like from outside — and it
        // has to keep being true with N slots, since `Deck::begin_frame` polls
        // every one of them.
        assert!(
            frames > 11,
            "only {frames} frames were produced; the deck waited for the build"
        );

        assert_eq!(
            deck.slot(0).set().capacity(),
            SWAPPED,
            "the swap reported success but slot 0 is still the old Set"
        );
        // Cold, as every V1 swap is: a new procedure means new buffers. Warming
        // one out of sight is Priming, and it is not in this slice.
        assert_eq!(
            steps_taken(deck.slot(0).set()),
            1,
            "the swapped-in Set inherited a `t`"
        );

        assert_eq!(
            deck.slot(1).set().capacity(),
            CAPACITY,
            "the swap reached the neighbouring slot"
        );
        assert_eq!(
            steps_taken(deck.slot(1).set()),
            frames,
            "the neighbouring slot's clock did not advance normally across the swap"
        );
        assert_eq!(
            deck.slot(1).set().live_count(&gpu.device, &gpu.queue),
            neighbour_live_before,
            "the swap disturbed the neighbouring slot's element buffers"
        );
        assert_eq!(
            deck.live_slots(),
            2,
            "the swap changed which slots are live"
        );
        assert!(
            deck.events(1).next().is_none(),
            "the untouched slot reported an event"
        );
    }

    /// **The watchdog does not judge a Set that is not on screen.**
    ///
    /// `Deck::begin_frame` gives every slot its frame boundary, off-air ones
    /// included — a build has to be able to land on a slot that is not showing,
    /// and retired Sets have to keep reaching the worker. What an off-air slot
    /// must *not* get is a watchdog sample: it renders nothing, so the frame
    /// interval the deck is producing is entirely the other slots' cost. Judging
    /// against it accepts a candidate on a budget it never spent, and — with a
    /// tight budget and busy neighbours, which is what this asserts because it is
    /// the deterministic direction — rolls one back for cost it never caused.
    ///
    /// The budget here is zero, so nothing can pass it. While the slot is parked
    /// no verdict may arrive at all; the moment it goes Live, one must.
    #[test]
    fn an_off_air_slot_is_not_judged_against_its_neighbours_frames() {
        /// `WARMUP_FRAMES + JUDGE_FRAMES` in `swap.rs`, and slack. Private there,
        /// so this is a duplicate — if it drifts, the test gets weaker rather than
        /// wrong, because it would stop being enough frames for a verdict and the
        /// second half would catch that.
        const A_FULL_WINDOW: usize = 8 + 30 + 12;

        let gpu = Gpu::headless().expect("no GPU available");
        let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);

        let (tx, rx) = mpsc::channel();
        let parked = HotSwap::new(
            &gpu.device,
            &gpu.queue,
            build(&gpu, SEED_B, CAPACITY),
            0.0,
            Box::new(rx),
        );
        let mut deck = Deck::new(
            &gpu.device,
            vec![HotSwap::fixed(build(&gpu, SEED_A, CAPACITY)), parked],
            WIDTH,
            HEIGHT,
        );
        deck.set_residency(1, Residency::Allocated);

        tx.send(Request {
            names: karakuri_engine::swap::RequestNames::default(),
            edges: Vec::new(),
            id: 1,
            l1s: vec![(compile(L1), SWAPPED)],
            l2s: Vec::new(),
            l3s: Vec::new(),
            fields: Vec::new(),
            layering: karakuri_engine::set::Layering::Overdraw,
            live: None,
            published: Vec::new(),
            l4s: vec![compile(L4)],
            seed_salt: SEED_B,
            camera: karakuri_engine::camera::Orbit::default(),
            salts: Vec::new(),
            params: Vec::new(),
            bindings: Vec::new(),
            authorities: Vec::new(),
            label: "off air".to_string(),
        })
        .expect("worker alive");

        let verdict = |deck: &mut Deck| -> Option<String> {
            deck.events(1).find_map(|e| match e {
                Event::Accepted {
                    label, median_ms, ..
                } => Some(format!("Accepted `{label}` at {median_ms:.3} ms")),
                Event::RolledBack {
                    label, median_ms, ..
                } => Some(format!("RolledBack `{label}` at {median_ms:.3} ms")),
                _ => None,
            })
        };

        // The build still lands on the parked slot: that is wanted, and the rest
        // of the test is about nothing else happening to it.
        let started = Instant::now();
        while deck.slot(1).set().capacity() != SWAPPED {
            frame(&gpu, &mut deck, &present, 1);
            assert!(
                verdict(&mut deck).is_none(),
                "a verdict arrived before the build even landed"
            );
            assert!(
                started.elapsed() < PATIENCE,
                "waited {PATIENCE:?} for the off-air build and it never installed"
            );
        }

        for _ in 0..A_FULL_WINDOW {
            frame(&gpu, &mut deck, &present, 1);
            if let Some(v) = verdict(&mut deck) {
                panic!(
                    "the watchdog reached a verdict — {v} — on a slot that rendered nothing; \
                 the interval it measured is the neighbouring slot's cost"
                );
            }
        }
        assert_eq!(
            steps_taken(deck.slot(1).set()),
            0,
            "the parked slot was stepped"
        );

        // On air, it is judged — against a budget of zero, so it goes.
        deck.set_residency(1, Residency::Live);
        let mut on_air = None;
        for _ in 0..A_FULL_WINDOW {
            frame(&gpu, &mut deck, &present, 1);
            if let Some(v) = verdict(&mut deck) {
                on_air = Some(v);
                break;
            }
        }
        assert!(
            on_air.is_some_and(|v| v.starts_with("RolledBack")),
            "a Live slot's candidate was never judged, so the first half of this test \
         would pass on a watchdog that had simply stopped working"
        );
    }

    // ---------------------------------------------------------------------------
    // Measured, reported.
    // ---------------------------------------------------------------------------

    /// What a slot costs, and what the composite costs. **Printed, not asserted.**
    ///
    /// `docs/contributing.md`'s working style asks for a GPU-timestamp measurement on any
    /// change touching the frame path, and says in the same breath that timestamps
    /// do not work on the machine this was developed on — `probe.rs` documents an
    /// enormous workload resolving to zero. So this is a host clock around
    /// submit-and-wait, on the same terms as every other number in this
    /// repository: coarse, biased high, and real. Run with
    /// `cargo test -p karakuri-engine --test deck -- --nocapture --ignored`.
    ///
    /// Five configurations at the CLI's own defaults, so the numbers are
    /// comparable with the other host-clock figures in this repository rather
    /// than being a measurement of
    /// a toy: a bare Set, a deck of one, a deck of four, and a deck of four with
    /// one slot Live — with and without the three off-air draws. Bare against
    /// deck-of-one isolates the composite pass, since the simulation either side
    /// of it is identical. Deck-of-one against deck-of-four is what a slot costs,
    /// which is dominated by the Set and not by the mix.
    ///
    /// **The last pair is what P-0080 costs**, and it is why it is measured here
    /// rather than argued in a comment. Every slot is drawn on every frame so
    /// that its console cell has something in it, which on the panel's own deck
    /// is one Live slot and three off-air draws. `Residency::Allocated` is the
    /// after; `Residency::Priming` at one step in a very large number is the
    /// before, since the governor's rate makes it step on no frame in the window
    /// and the branch is otherwise the same — no, it is not: it draws too. So the
    /// before is **a deck of one**, which is exactly what a four-slot deck with
    /// three dark slots used to cost on the frame path: three slots that neither
    /// stepped, drew, nor reached the composite. The difference between that line
    /// and the four-slots-one-Live line is the whole of the bill.
    ///
    /// `#[ignore]`d because four simulations at capacity 262144 is a real
    /// workload, and `cargo test` should not be one.
    #[test]
    #[ignore = "a measurement, not a check; run with --ignored --nocapture"]
    fn the_cost_of_a_slot_and_of_the_composite_are_measured_and_reported() {
        const CAP: u32 = 262_144;
        const W: u32 = 1280;
        const H: u32 = 720;
        const WARMUP: usize = 60;
        const MEASURED: usize = 120;

        let gpu = Gpu::headless().expect("no GPU available");

        let summarize = |label: &str, mut xs: Vec<f32>| {
            xs.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
            eprintln!(
                "  {label:<24} n={:<4} median {:.3} ms   worst {:.3} ms",
                xs.len(),
                xs[xs.len() / 2],
                xs[xs.len() - 1]
            );
        };

        let present = Present::new(&gpu.device, Present::HDR_FORMAT, W, H);

        // Bare Set, no deck: the path every earlier test measures.
        let mut bare = Set::build(
            &gpu.device,
            &gpu.queue,
            &compile(L1),
            &compile(L4),
            CAP,
            SEED_A,
        )
        .expect("the pair is compatible");
        bare.resize(&gpu.device, W, H);
        let mut bare_ms = Vec::new();
        for i in 0..WARMUP + MEASURED {
            let at = Instant::now();
            bare_frame(&gpu, &mut bare, &present, 1);
            if i >= WARMUP {
                bare_ms.push(at.elapsed().as_secs_f32() * 1_000.0);
            }
        }

        let deck_ms = |seeds: &[u32]| -> Vec<f32> {
            let swaps = seeds
                .iter()
                .map(|&seed| {
                    let mut set = Set::build(
                        &gpu.device,
                        &gpu.queue,
                        &compile(L1),
                        &compile(L4),
                        CAP,
                        seed,
                    )
                    .expect("the pair is compatible");
                    set.resize(&gpu.device, W, H);
                    HotSwap::fixed(set)
                })
                .collect();
            let mut deck = Deck::new(&gpu.device, swaps, W, H);
            let mut out = Vec::new();
            for i in 0..WARMUP + MEASURED {
                let at = Instant::now();
                frame(&gpu, &mut deck, &present, 1);
                if i >= WARMUP {
                    out.push(at.elapsed().as_secs_f32() * 1_000.0);
                }
            }
            out
        };
        let one = deck_ms(&[SEED_A]);
        let four = deck_ms(&[SEED_A, SEED_B, SEED_A + 1, SEED_B + 1]);
        let one_live = {
            let seeds = [SEED_A, SEED_B, SEED_A + 1, SEED_B + 1];
            let swaps = seeds
                .iter()
                .map(|&seed| {
                    let mut set = Set::build(
                        &gpu.device,
                        &gpu.queue,
                        &compile(L1),
                        &compile(L4),
                        CAP,
                        seed,
                    )
                    .expect("the pair is compatible");
                    set.resize(&gpu.device, W, H);
                    HotSwap::fixed(set)
                })
                .collect();
            let mut deck = Deck::new(&gpu.device, swaps, W, H);
            // Warmed on air first, then parked: three slots with material in
            // them, drawn and not stepped, which is the panel's own state.
            for _ in 0..8 {
                frame(&gpu, &mut deck, &present, 1);
            }
            for slot in 1..4 {
                deck.set_residency(slot, Residency::Allocated);
            }
            let mut out = Vec::new();
            for i in 0..WARMUP + MEASURED {
                let at = Instant::now();
                frame(&gpu, &mut deck, &present, 1);
                if i >= WARMUP {
                    out.push(at.elapsed().as_secs_f32() * 1_000.0);
                }
            }
            out
        };

        let deck_mb = 4.0 * f64::from(W) * f64::from(H) * 8.0 / 1_048_576.0;
        eprintln!(
            "\nframe times at capacity {CAP}, {W}x{H}, host clock around submit-and-wait \
         (four slot targets at 8 bytes a texel is {deck_mb:.1} MB):"
        );
        summarize("bare Set, no deck", bare_ms);
        summarize("deck of one", one);
        summarize("deck of four", four);
        summarize("four, one Live", one_live);
        eprintln!(
            "  `deck of one` is what `four, one Live` cost before every slot was drawn: \n               the three off-air slots did nothing at all on the frame path. The gap between \n               those two lines is what P-0080 costs on this machine."
        );
        eprintln!();
    }

    /// What filling a deck slot's **preview cell** costs, two ways. **Printed,
    /// not asserted.**
    ///
    /// A cell in the program bay is **112 x 63** — `karakuri-console`'s
    /// `view::preview_cells` derives the width from `(466 - three 6px gaps) / 4
    /// = 112`, and 112 at 16:9 is 63 — while a slot renders at the deck's
    /// canvas, 1280x720. Two ways to get one into the other, and this measures
    /// both rather than arguing them:
    ///
    /// - **downsample**: present the 1280x720 target into the cell. No extra
    ///   draw. The horizontal stride is `1280 / 112` = 11.4 source texels at 8
    ///   bytes each, so consecutive output texels fall in different cache lines.
    /// - **re-render**: draw the Set again into a 112x63 target. Better
    ///   locality, and **not fewer primitives**: the point count is the
    ///   capacity either way, 262144.
    ///
    /// **The filter decides how many taps the downsample takes, and this one
    /// takes four.** `Present`'s sampler is `Linear`/`Linear` over a texture
    /// with `mip_level_count: 1`, so there is no mip chain to fall back on and
    /// a fragment reads a 2x2 neighbourhood, not an 11x11 box. The downsample
    /// therefore *undersamples* — it is bilinear point-picking with aliasing,
    /// not a box filter — and it does not read the 1.8 MB the source occupies:
    /// 7056 output texels at four taps is 28k taps, scattered.
    ///
    /// **`point_rate` is a fraction of the target's height, so a small target
    /// has fewer fragments per sprite — until a sprite reaches a pixel.**
    /// `karakuri-codegen`'s L4 expansion scales the quad by the rate rather than
    /// by a pixel count, so a sprite is the same share of the frame at every
    /// size. This paragraph read that a 112x63 render therefore rasterises
    /// roughly `(63/720)²` of the fragments a 1280x720 one does, and that is no
    /// longer true at the bottom of the sweep: a quad below a pixel is floored
    /// at one pixel and dimmed rather than dropped (ADR-0245), so a `Points`
    /// procedure's fragment count bottoms out at one per element
    /// instead of falling with the area. At 112x63 this material's sprites are
    /// about a third of a pixel across, so the cell render sits entirely in that
    /// floored regime.
    ///
    /// **The sweep's lit-texel column is not the check for that**, and reading
    /// it as one would be a mistake. At capacity 262144 the coverage saturates —
    /// 231 points land on each lit texel at 112x63 — so the column reports the
    /// material's silhouette rather than any sprite's extent, and it comes out
    /// near 16% at every size for that reason. What a sprite's extent does at
    /// two target sizes is asserted to the texel in `tests/lines.rs`, on one
    /// element, where nothing saturates.
    ///
    /// Same method as its neighbour
    /// [`the_cost_of_a_slot_and_of_the_composite_are_measured_and_reported`]:
    /// host clock around submit-and-wait, the same 60-frame warm-up and
    /// 120-frame window, medians and worst rather than means. Two departures,
    /// both because this compares configurations against each other rather
    /// than reporting them one at a time:
    ///
    /// - **the configurations are interleaved, one frame each per round, and
    ///   the order rotates.** Run as blocks, the first block measured a cold
    ///   GPU and the last a hot one: 1280x720 came out 9.3 ms as the first
    ///   block and 7.3 ms as the last, which is a quarter of the number and
    ///   none of it the configuration. Those two figures are from the run that
    ///   found the hazard, under the pixel-size semantics; the hazard is the
    ///   point and it is not sensitive to either.
    /// - **each present writes into its own cell texture**, so "did this pass
    ///   write anything" can still be asked of each of them at the end. A
    ///   target that persists between frames is the hazard the pixel tests
    ///   above defeat by resizing; here every destination is blackened before
    ///   the loop and counted after it.
    ///
    /// One configuration is **nothing at all** — an empty command buffer,
    /// submitted and waited on. A host clock around submit-and-wait pays for a
    /// round trip whether or not there is work in it, and the present passes
    /// here are small enough that the round trip is most of what is timed:
    /// 0.13 ms of the 0.49 ms. Every present figure worth quoting is net of
    /// that line.
    ///
    /// **Run it alone.** `cargo test` runs the two benchmarks in this file on
    /// two threads and one GPU, and every number in both comes out about 40%
    /// high; `--test-threads=1`, or a name filter, is part of the method.
    ///
    /// **What it found, so that the next reader need not run it.** The
    /// downsample is under a millisecond net of the floor and the 11.4-texel
    /// stride costs almost nothing — 0.883 ms against 0.805 ms for the same
    /// present with no scaling at all.
    ///
    /// **The cost order has reversed twice, and the second time it reversed
    /// back.** Under the old pixel-size semantics, re-rendering into the cell
    /// was 17.5 ms against 9.3 ms for the whole 1280x720 frame. `point_rate`
    /// turned that around — 7.6 ms against 10.5 ms, the sweep climbing with the
    /// target — and the one-pixel floor turned it back. Measured here as a
    /// **pair**, one machine and one sitting, with the floor removed and
    /// restored, because the older figures are another machine's and a
    /// difference between them would be unreadable:
    ///
    /// | target | no floor | floored |
    /// |---|---|---|
    /// | 112x63 | 7.338 ms | 10.573 ms |
    /// | 224x126 | 8.209 ms | 10.700 ms |
    /// | 448x252 | 10.239 ms | 10.036 ms |
    /// | 640x360 | 10.693 ms | 9.860 ms |
    /// | 1280x720 | 11.687 ms | 10.014 ms |
    ///
    /// The draw-only halves say it without the compute in them: 4.797 ms against
    /// 8.103 ms at 112x63, and 7.48 ms either way at 1280x720. **Rendering at
    /// cell size went from 0.6x the whole 720p frame to 1.1x of it**, which is
    /// what a floor of one fragment per element does to a target of 7056 texels
    /// holding 262144 elements.
    ///
    /// **The 1280x720 row is the control, and it moved 1.7 ms.** The floor is
    /// inert there — this material is 1.4 to 4 pixels across at that size — so
    /// that gap is this machine's run-to-run agreement rather than an effect of
    /// anything. Read the shape of the sweep and not the rows.
    ///
    /// **It is still not the way to fill a cell, and the reason has changed.**
    /// The reason used to be that most of this material's sprites cover no pixel
    /// centre at 112x63 and vanish — which was a defect in the renderer rather
    /// than a fact about cells, and ADR-0245 is where it went. What stands in its
    /// place is the plain cost: the full-size target is rendered anyway, so
    /// downsampling costs line 2 alone, 0.489 ms net of the floor, against a
    /// second 10.573 ms pass. **What fills a cell is open again** in the sense
    /// that the correctness argument is spent; the cost argument now points the
    /// same way, and the panel downsamples today.
    ///
    /// All of that is a claim about point sprites and not about every L4 — a
    /// fullscreen node such as `examples/field_march.kir` has a fragment count
    /// that *is* the pixel count, and nothing here measures one.
    ///
    /// `#[ignore]`d for the same reason its neighbour is: capacity 262144,
    /// eleven configurations.
    #[test]
    #[ignore = "a measurement, not a check; run with --ignored --nocapture"]
    fn the_cost_of_filling_a_preview_cell_is_measured_and_reported() {
        /// The panel's own material, rather than this file's fixtures: the
        /// question is about what a slot on the console costs.
        const PANEL_L1: &str = include_str!("../../../examples/drift_shell.kir");
        const PANEL_L4: &str = include_str!("../../../examples/soft_points.kir");

        const CAP: u32 = 262_144;
        const W: u32 = 1280;
        const H: u32 = 720;
        const CELL_W: u32 = 112;
        const CELL_H: u32 = 63;
        const WARMUP: usize = 60;
        const MEASURED: usize = 120;

        /// The sizes between the cell and the canvas, for the sweep. The two
        /// ends of it are configurations 0 and 2 and are not repeated here.
        const BETWEEN: [(u32, u32); 3] = [(224, 126), (448, 252), (640, 360)];

        const LABELS: [&str; 11] = [
            "1  render 1280x720",
            "2  present 1280x720 -> cell",
            "3  render 112x63",
            "4  present cell -> cell 1:1",
            "1b draw only, 1280x720",
            "1c step only, no draw",
            "3b draw only, 112x63",
            "   render 224x126",
            "   render 448x252",
            "   render 640x360",
            "0  empty submit + poll",
        ];

        let gpu = Gpu::headless().expect("no GPU available");

        let sorted = |xs: &[f32]| {
            let mut xs = xs.to_vec();
            xs.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
            xs
        };
        let summarize = |label: &str, xs: &[f32]| {
            let xs = sorted(xs);
            eprintln!(
                "  {label:<30} n={:<4} median {:.3} ms   worst {:.3} ms",
                xs.len(),
                xs[xs.len() / 2],
                xs[xs.len() - 1]
            );
        };
        let median = |xs: &[f32]| sorted(xs)[xs.len() / 2];

        let make_set = |width: u32, height: u32| -> Set {
            let mut set = Set::build(
                &gpu.device,
                &gpu.queue,
                &compile(PANEL_L1),
                &compile(PANEL_L4),
                CAP,
                SEED_A,
            )
            .expect("the panel's own pair is compatible");
            set.resize(&gpu.device, width, height);
            set
        };

        let make_cell = |label: &'static str| -> (wgpu::Texture, wgpu::TextureView) {
            let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width: CELL_W,
                    height: CELL_H,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: Present::HDR_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            let view = texture.create_view(&Default::default());
            (texture, view)
        };

        // Black, so that "this pass wrote something" can be told from "the
        // texture still holds what something else put there".
        let blacken = |view: &wgpu::TextureView| {
            let mut encoder = gpu.device.create_command_encoder(&Default::default());
            drop(encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("blacken"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
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
            }));
            gpu.queue.submit([encoder.finish()]);
            gpu.device
                .poll(wgpu::PollType::wait_indefinitely())
                .expect("poll");
        };

        // [`readback`] above requires a 256-aligned row and a 112-wide
        // `Rgba16Float` row is 896 bytes, so this pads the pitch and walks the
        // padding back off. Counting lit texels is all it is for.
        let lit_texels = |texture: &wgpu::Texture| -> u32 {
            let (width, height) = (texture.width(), texture.height());
            let pitch = (width * 8).div_ceil(256) * 256;
            let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("cell readback"),
                size: u64::from(pitch * height),
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            let mut encoder = gpu.device.create_command_encoder(&Default::default());
            encoder.copy_texture_to_buffer(
                texture.as_image_copy(),
                wgpu::TexelCopyBufferInfo {
                    buffer: &buffer,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(pitch),
                        rows_per_image: Some(height),
                    },
                },
                wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
            );
            gpu.queue.submit([encoder.finish()]);
            let slice = buffer.slice(..);
            slice.map_async(wgpu::MapMode::Read, |r| r.expect("map"));
            gpu.device
                .poll(wgpu::PollType::wait_indefinitely())
                .expect("poll");
            let data = slice.get_mapped_range().expect("map");
            let mut lit = 0;
            for y in 0..height as usize {
                for x in 0..width as usize {
                    let at = y * pitch as usize + x * 8;
                    // The three colour channels only: the present pass writes
                    // alpha 1.0 everywhere and would count every texel.
                    if data[at..at + 6].iter().any(|&b| b != 0) {
                        lit += 1;
                    }
                }
            }
            drop(data);
            buffer.unmap();
            lit
        };

        let canvas = Present::new(&gpu.device, Present::HDR_FORMAT, W, H);
        let cell_canvas = Present::new(&gpu.device, Present::HDR_FORMAT, CELL_W, CELL_H);
        let (down_cell, down_view) = make_cell("downsampled cell");
        let (flat_cell, flat_view) = make_cell("1:1 cell");

        let mut big = make_set(W, H);
        let mut small = make_set(CELL_W, CELL_H);
        let mut between: Vec<(Present, Set)> = BETWEEN
            .iter()
            .map(|&(w, h)| {
                (
                    Present::new(&gpu.device, Present::HDR_FORMAT, w, h),
                    make_set(w, h),
                )
            })
            .collect();

        for view in [
            canvas.hdr_view(),
            cell_canvas.hdr_view(),
            &down_view,
            &flat_view,
        ] {
            blacken(view);
        }
        for (target, _) in &between {
            blacken(target.hdr_view());
        }

        let submit = |encoder: wgpu::CommandEncoder| {
            gpu.queue.submit([encoder.finish()]);
            gpu.device
                .poll(wgpu::PollType::wait_indefinitely())
                .expect("poll");
        };

        // One frame of one configuration, submitted and waited on. All ten in
        // one closure because several of them share a Set and cannot each hold
        // their own mutable borrow of it.
        let mut run = |cfg: usize| {
            let mut encoder = gpu.device.create_command_encoder(&Default::default());
            match cfg {
                // The slot at the deck's canvas: the baseline, and the same
                // path `bare_frame` takes.
                0 => {
                    big.prepare(&gpu.queue, 1, &Signals::default());
                    big.render(&mut encoder, canvas.hdr_view(), 1);
                }
                // The downsample. The canvas holds a real picture from
                // configuration 0, so this is not sampling a texture that has
                // only ever been fast-cleared.
                1 => canvas.draw(&mut encoder, &down_view, (CELL_W, CELL_H)),
                // The same Set rendered straight into a cell-sized target.
                2 => {
                    small.prepare(&gpu.queue, 1, &Signals::default());
                    small.render(&mut encoder, cell_canvas.hdr_view(), 1);
                }
                // A present that is not a downsample: 112x63 into 112x63,
                // viewport 1:1, so the difference from configuration 1 is what
                // the 11.4-texel stride costs and nothing else.
                3 => cell_canvas.draw(&mut encoder, &flat_view, (CELL_W, CELL_H)),
                // The raster half alone — an off-air slot's own path, drawn
                // every frame and never stepped.
                4 => big.draw(&mut encoder, canvas.hdr_view()),
                // The compute half alone: no target, no draw.
                5 => {
                    big.prepare(&gpu.queue, 1, &Signals::default());
                    big.step(&mut encoder, 1);
                }
                6 => small.draw(&mut encoder, cell_canvas.hdr_view()),
                // The sweep: everything held fixed but the target size.
                7..=9 => {
                    let (target, set) = &mut between[cfg - 7];
                    set.prepare(&gpu.queue, 1, &Signals::default());
                    set.render(&mut encoder, target.hdr_view(), 1);
                }
                // Nothing at all, submitted and waited on: the floor this
                // harness can measure. A present pass costing "tens of
                // microseconds" is unreadable here unless it is read against
                // this line, because a host clock around submit-and-wait is
                // paying for a round trip either way.
                _ => {}
            }
            submit(encoder);
        };

        let mut times: Vec<Vec<f32>> = vec![Vec::with_capacity(MEASURED); LABELS.len()];
        for i in 0..WARMUP + MEASURED {
            // Rotated, so no configuration is permanently the one that follows
            // the heaviest.
            for j in 0..LABELS.len() {
                let cfg = (j + i) % LABELS.len();
                let at = Instant::now();
                run(cfg);
                if i >= WARMUP {
                    times[cfg].push(at.elapsed().as_secs_f32() * 1_000.0);
                }
            }
        }

        eprintln!(
            "\nfilling one {CELL_W}x{CELL_H} preview cell from a {W}x{H} slot, \
             capacity {CAP}, `examples/drift_shell.kir` + `examples/soft_points.kir`,\n\
             host clock around submit-and-wait, interleaved, \
             {WARMUP} rounds of warm-up discarded:"
        );
        summarize(LABELS[10], &times[10]);
        for cfg in 0..4 {
            summarize(LABELS[cfg], &times[cfg]);
        }
        eprintln!("  and, to split line 1:");
        for cfg in 4..7 {
            summarize(LABELS[cfg], &times[cfg]);
        }
        eprintln!(
            "  the compute half is {:.3} ms of line 1's {:.3} ms and the raster half \
             {:.3} ms;\n               rendering at cell size leaves {:.3} ms, which is \
             {:.1}x the whole 720p frame.",
            median(&times[5]),
            median(&times[0]),
            median(&times[4]),
            median(&times[2]),
            median(&times[2]) / median(&times[0]),
        );

        eprintln!(
            "  the same Set and the same {CAP} points, target size swept — \
             the cost follows the target, and the lit share does not:"
        );
        let sweep = [
            (
                CELL_W,
                CELL_H,
                median(&times[2]),
                lit_texels(cell_canvas.hdr_texture()),
            ),
            (
                BETWEEN[0].0,
                BETWEEN[0].1,
                median(&times[7]),
                lit_texels(between[0].0.hdr_texture()),
            ),
            (
                BETWEEN[1].0,
                BETWEEN[1].1,
                median(&times[8]),
                lit_texels(between[1].0.hdr_texture()),
            ),
            (
                BETWEEN[2].0,
                BETWEEN[2].1,
                median(&times[9]),
                lit_texels(between[2].0.hdr_texture()),
            ),
            (W, H, median(&times[0]), lit_texels(canvas.hdr_texture())),
        ];
        for (w, h, ms, lit) in sweep {
            eprintln!(
                "    {w:>4}x{h:<4} {ms:>7.3} ms   {lit} lit of {} ({:.1}% of the target) \
                 — {:.0} points per lit texel",
                w * h,
                100.0 * f64::from(lit) / f64::from(w * h),
                f64::from(CAP) / f64::from(lit.max(1)),
            );
        }

        eprintln!(
            "  net of the floor, the downsample is {:.3} ms and the 1:1 present {:.3} ms.",
            median(&times[1]) - median(&times[10]),
            median(&times[3]) - median(&times[10]),
        );

        let (down_lit, flat_lit) = (lit_texels(&down_cell), lit_texels(&flat_cell));
        eprintln!(
            "  the two cells came back {down_lit} and {flat_lit} lit of {}; \
             zero would mean a present pass wrote nothing and was timed anyway.",
            CELL_W * CELL_H
        );
        assert!(
            down_lit > 0 && flat_lit > 0,
            "a present pass wrote nothing, so its timing is not a timing of the present"
        );
        eprintln!();
    }

    /// A mix target that is not the deck's size is refused rather than mixed.
    ///
    /// The composite reads its sources with `textureLoad`, and an out-of-range
    /// `textureLoad` is *defined* to return zero — so resizing `Present` and
    /// forgetting the deck would produce a black frame, every frame, with nothing
    /// logged. A panic at the call is louder than a picture that is quietly wrong,
    /// and this is a programming error rather than an input error: no `.kir` and
    /// no record stream can reach it.
    #[test]
    #[should_panic(expected = "resize both")]
    fn a_mix_target_of_the_wrong_size_is_refused() {
        let gpu = Gpu::headless().expect("no GPU available");
        let mut deck = deck_of(&gpu, &[1]);
        let present = Present::new(
            &gpu.device,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            WIDTH,
            HEIGHT,
        );

        // The deck moves, the target does not — the direction a window resize
        // takes if only one of the two handlers is wired.
        deck.resize(&gpu.device, WIDTH * 2, HEIGHT * 2);

        let mut frame = deck.begin_frame(&gpu.device, &gpu.queue);
        frame.render(present.hdr_view(), present.size(), 1);
    }
}
