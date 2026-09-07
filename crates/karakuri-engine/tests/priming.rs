//! Priming: a deck slot warming its element buffers out of the mix.
//!
//! What is asserted here is the claim `deck.rs` makes in "Every off-air slot
//! runs, and a preview is what it is for", in the order that module argues it:
//!
//! - an off-air slot **steps every frame, at the room's tempo** — its `t` keeps
//!   up with the Live slots' — and reaches the mix in no way at all;
//! - **Priming and Allocated do the same thing**, so a slot nobody asked about
//!   warms exactly as one somebody asked for does;
//! - **a slot off air for thirty frames and then put on air is bit-for-bit the
//!   slot that was on air for thirty-one**, which is what makes its cell a
//!   preview of what putting it on air would look like;
//! - and it is visibly different from a slot that has never stepped, or none of
//!   the above would be worth anything;
//! - determinism survives all of it, including a slot that primes for a while
//!   and then goes Live;
//! - and **the signal an off-air slot reads is its own clock's, not the
//!   frame's** — so a Set that arrived late warms into the same material as one
//!   that was there from the start, and a slot that is not behind reads exactly
//!   what it would have read on air.
//!
//! The last pair needs both halves and the second is the sharper one: two
//! warming runs agree with each other under any clock that is a function of the
//! step index, including one a whole frame early. Only a comparison against a
//! *Live* run says which instant is the right one.
//!
//! **A reduced rate used to be half of this file and is gone.** Priming could
//! be slowed to one step in `n` by the governor, and four tests here asserted
//! what that did and did not change. ADR-0269 retired the rate: a drawn slot
//! steps every frame, every slot is drawn, and a preview off the room's tempo
//! is not a preview of anything.
//!
//! The material is deliberately an *accumulating* procedure — elements drift
//! outward from the origin at a fixed rate — because a closed-form one would
//! look identical warm and cold and every test below would pass on a deck that
//! never primed anything. Cold, every element is at the origin and the frame is
//! one small blob; warm, they are on a sphere. That gap is what "shows warmed
//! state" is measured by.

// Every test here takes a device, so the whole file is one `mod gpu` — the
// prefix `cargo test -- --skip gpu::` filters on. The convention, and the test
// that enforces it, are in `tests/gpu_tests_are_under_mod_gpu.rs`.
mod gpu {
    use karakuri_engine::binding::Curve;
    use karakuri_engine::deck::{Deck, Residency};
    use karakuri_engine::set::DT;
    use karakuri_engine::swap::HotSwap;
    use karakuri_engine::{Binding, Gpu, Present, Set, Signals};
    use karakuri_ir::typed::Checked;
    use karakuri_ir::Kind;

    const WIDTH: u32 = 256;
    const HEIGHT: u32 = 256;
    const CAPACITY: u32 = 4096;

    const SEED_A: u32 = 19274;
    const SEED_B: u32 = 88888;

    /// **Accumulating on purpose.** `position = position + ...` reads what it
    /// emits, so the state at `t` is the sum of every step taken to get there and
    /// nothing but running it forward produces it. `karakuri-ir` classifies this
    /// as not closed form, which is what makes it a legitimate thing to prime;
    /// `tests/governor.rs` asserts the other side.
    ///
    /// Cold, every element sits at the origin — a Set comes up with every
    /// attribute zeroed for a procedure with no `spawn` block, bar the `seed` and
    /// `birth_frac` the engine seeds — so the frame is one blob. After thirty steps they are on a
    /// sphere of radius `30 * dt * 4`, about two units.
    const CREEP: &str = r#"
proc creep {
  kind     L1
  topology points
  capacity [1024, 262144] = 4096

  param speed : float [0.0, 16.0] = 4.0

  emit position, age

  element {
    let dir = sphere_point(hash1(seed), hash1(seed + 1000u));
    position = position + dir * dt * speed;
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

    fn build(gpu: &Gpu, seed: u32) -> Set {
        let mut set = Set::build(
            &gpu.device,
            &gpu.queue,
            &compile(CREEP),
            &compile(L4),
            CAPACITY,
            seed,
        )
        .expect("the pair is compatible and the capacity is in range");
        set.resize(&gpu.device, WIDTH, HEIGHT);
        set
    }

    /// A deck of fixed Sets: no worker, so nothing can swap and a run is a pure
    /// function of the ticks it is given.
    fn deck_of(gpu: &Gpu, seeds: &[u32]) -> Deck {
        let swaps = seeds
            .iter()
            .map(|&seed| HotSwap::fixed(build(gpu, seed)))
            .collect();
        Deck::new(&gpu.device, swaps, WIDTH, HEIGHT)
    }

    fn frame(gpu: &Gpu, deck: &mut Deck, present: &Present, steps: u8) {
        let mut f = deck.begin_frame(&gpu.device, &gpu.queue);
        f.render(present.hdr_view(), present.size(), steps);
        f.finish();
        gpu.device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("poll");
    }

    /// Raw `f16` bits, four per texel — raw so that an exact comparison compares
    /// bits rather than two float expressions that happen to agree.
    fn readback(gpu: &Gpu, texture: &wgpu::Texture) -> Vec<u16> {
        let (width, height) = (texture.width(), texture.height());
        let bytes_per_row = width * 8;
        assert_eq!(bytes_per_row % 256, 0, "row pitch must be 256-byte aligned");

        let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("priming readback"),
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

    fn lit(pixels: &[u16]) -> usize {
        pixels
            .chunks_exact(4)
            .filter(|p| p[0] != 0 || p[1] != 0 || p[2] != 0)
            .count()
    }

    /// Simulation steps taken, recovered as an integer. `Set::time` is
    /// `steps * dt`, and comparing that against a float expression meaning the same
    /// thing is the last-bit trap `Set` derives `t` from an integer counter to
    /// avoid.
    fn steps_taken(set: &Set) -> u64 {
        (set.time() * 60.0).round() as u64
    }

    // ---------------------------------------------------------------------------

    /// **A Priming slot steps, and contributes nothing to the mix.**
    ///
    /// Both halves matter and each would pass alone on a broken implementation.
    /// "Steps" alone passes on a slot that is also being composited — which is just
    /// Live under another name. "Contributes nothing" alone passes on a slot that
    /// is doing nothing at all — which is Allocated under another name. The mix is
    /// compared bit for bit against a deck holding only the other slot, on the same
    /// ticks, so the priming slot's absence from it is exact rather than small.
    ///
    /// **Drawn into its own target and absent from the mix are the two halves of
    /// one separation**, and this test now holds both. It used to assert that a
    /// Priming slot rendered nowhere at all, which was true of the engine and
    /// wrong of the instrument: the slot an operator is warming is the one they
    /// need to see. Every slot is drawn into its own target now
    /// (ADR-0258), and *drawn* and *mixed* are two questions.
    #[test]
    fn a_priming_slot_advances_its_t_and_contributes_nothing_to_the_mix() {
        let gpu = Gpu::headless().expect("no GPU available");
        const FRAMES: usize = 20;

        let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
        let mut deck = deck_of(&gpu, &[SEED_A, SEED_B]);
        deck.set_residency(1, Residency::Priming);
        assert_eq!(deck.live_slots(), 1);
        assert_eq!(deck.priming_slots(), 1);

        for _ in 0..FRAMES {
            frame(&gpu, &mut deck, &present, 1);
        }
        let mixed = readback(&gpu, present.hdr_texture());

        assert_eq!(
            steps_taken(deck.slot(1).set()),
            FRAMES as u64,
            "a Priming slot did not advance its `t`; nothing is calling `prepare` on it \
         and it is Allocated under another name"
        );
        assert_eq!(
            steps_taken(deck.slot(0).set()),
            FRAMES as u64,
            "the Live slot did not step normally alongside a priming one"
        );

        let solo_present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
        let mut solo = deck_of(&gpu, &[SEED_A]);
        for _ in 0..FRAMES {
            frame(&gpu, &mut solo, &solo_present, 1);
        }
        let expected = readback(&gpu, solo_present.hdr_texture());

        assert!(
            lit(&expected) > 100,
            "the remaining slot drew nothing, so this comparison is two black frames"
        );
        assert_eq!(
            mixed, expected,
            "a Priming slot reached the mix — it is stepped and drawn into its own \
         target, and neither of those may put it in the room"
        );
        // **And it *did* draw into its own target**, which is the other half of
        // the separation and used to be asserted the other way round. Priming
        // itself needs no draw — the warming is in the element buffers and not
        // in the pixels, which is what "Every off-air slot runs, and a preview
        // is what it is for" in
        // `deck.rs` argues — but a warming slot is exactly the one an operator
        // wants to look at before bringing it up, so every slot is drawn into
        // its own target whatever its residency
        // (`docs/adr/0258-the-look-comes-before-the-fader-so-a-cell-draws-every-slot-and-says-which-nothing-it-is.md`).
        // The mix comparison above is what says that draw does not leak into
        // the room.
        assert!(
            lit(&readback(&gpu, deck.slot_target(1))) > 100,
            "a Priming slot drew nothing into its own target, so its console cell is \
         dark at exactly the moment an operator is deciding whether to bring it up"
        );
        // No level, and that is the residency and not the draw: a level is what
        // a fader is read against, a fader acts on what reaches the mix, and
        // this slot reaches none of it. See `Deck::level`.
        assert!(deck.level(1).is_none());
    }

    /// **An off-air slot steps every frame, at the room's tempo, whatever its
    /// residency.**
    ///
    /// This is ADR-0269 stated as a `t`. Both off-air slots take the same steps
    /// the Live one takes, off the same ticks — a slot nobody asked about as
    /// much as one that was asked for and granted — because both of them are
    /// drawn into a cell an operator judges material by, and a cell drawn from
    /// a slot nothing is stepping is a still.
    ///
    /// **Uneven ticks rather than one step a frame**, so that "every frame" is
    /// asserted as *the same steps the room took* rather than as a frame count
    /// that a slot stepping once a frame would satisfy by accident.
    ///
    /// Before ADR-0269 the Allocated slot read zero here and the Priming one
    /// read whatever rate the last `set_prime_one_in` had left.
    #[test]
    fn an_off_air_slot_steps_every_frame_at_the_rooms_tempo() {
        let gpu = Gpu::headless().expect("no GPU available");
        const TICKS: [u8; 8] = [1, 2, 1, 3, 1, 1, 4, 2];
        let total: u64 = TICKS.iter().map(|&s| u64::from(s)).sum();

        let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
        let mut deck = deck_of(&gpu, &[SEED_A, SEED_B, SEED_A + 1]);
        deck.set_residency(1, Residency::Priming);
        deck.set_residency(2, Residency::Allocated);

        for steps in TICKS {
            frame(&gpu, &mut deck, &present, steps);
        }

        assert_eq!(
            steps_taken(deck.slot(0).set()),
            total,
            "the Live slot did not take the ticks it was given"
        );
        assert_eq!(
            steps_taken(deck.slot(1).set()),
            total,
            "a Priming slot did not keep the room's tempo"
        );
        assert_eq!(
            steps_taken(deck.slot(2).set()),
            total,
            "an Allocated slot did not step; its cell is a still rather than a preview \
         of what putting it on air would look like"
        );
        // And neither of them reached the mix, which is what residency still
        // decides.
        assert_eq!(deck.live_slots(), 1);
        assert!(deck.level(1).is_none() && deck.level(2).is_none());
    }

    /// **An off-air slot's cell shows the material running, not the still it
    /// went off air on**, which is the whole of what ADR-0269 is for.
    ///
    /// **Warmed on air first, and that is what makes the assertion sharp.** A
    /// slot taken off air after a few frames has a picture in its target
    /// already, so the failure this catches is not a black cell — it is a cell
    /// that goes on showing the same picture, which is what an operator was
    /// being asked to judge a candidate on and which looks like a working
    /// preview until you watch it.
    ///
    /// Compared as pixels bit for bit rather than as a count: the claim is that
    /// the picture *changed*, and a count could hold across two different
    /// pictures. The lit-pixel ratio is asserted beside it so that "changed" is
    /// the material spreading rather than a bit somewhere.
    #[test]
    fn an_off_air_slots_cell_shows_material_running_rather_than_a_still() {
        let gpu = Gpu::headless().expect("no GPU available");
        const WARM: usize = 8;
        const FRAMES: usize = 30;

        let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
        let mut deck = deck_of(&gpu, &[SEED_A, SEED_B]);
        for _ in 0..WARM {
            frame(&gpu, &mut deck, &present, 1);
        }
        deck.set_residency(1, Residency::Allocated);
        frame(&gpu, &mut deck, &present, 1);
        let went_off_air = readback(&gpu, deck.slot_target(1));
        assert!(
            lit(&went_off_air) > 100,
            "the slot's own target was dark when it went off air, so this comparison \
         is two black frames"
        );

        for _ in 0..FRAMES {
            frame(&gpu, &mut deck, &present, 1);
        }
        let later = readback(&gpu, deck.slot_target(1));

        assert_ne!(
            later, went_off_air,
            "an off-air slot's cell is the same picture {FRAMES} frames later — it is \
         showing the still it went off air on rather than the material running, \
         which is what an operator is deciding from"
        );
        let (before, after) = (lit(&went_off_air), lit(&later));
        assert!(
            after > before,
            "the cell changed but did not spread: {before} lit pixels against {after}, \
         where this material drifts outward and an off-air slot is supposed to be \
         drifting with the room"
        );
    }

    /// **An off-air slot that goes Live shows warmed state**, and the warming is
    /// exactly the warming being on air would have done — at either off-air
    /// residency.
    ///
    /// Four runs of the same material, same seed, same ticks, differing only in
    /// what the slot was doing for the first thirty frames:
    ///
    /// ```text
    ///   primed   30 frames Priming,   then 1 Live   →  t = 31/60
    ///   resting  30 frames Allocated, then 1 Live   →  t = 31/60
    ///   always   31 frames Live                     →  t = 31/60
    ///   cold                          1 frame Live  →  t =  1/60
    /// ```
    ///
    /// `primed == always` and `resting == always`, **bit for bit**. The first is
    /// what justifies warming out of the mix at all: every one of a Set's states
    /// is L1's, so running L1 without compositing reaches the identical state,
    /// and nothing about the frames a slot spent off air is recoverable from the
    /// picture afterwards. The second is ADR-0269 — the two off-air residencies
    /// are one behaviour, and a slot nobody asked about warms exactly as a slot
    /// somebody asked for does. It can be exact because it is the same
    /// arithmetic in the same order; a tolerance here would hide a real
    /// difference rather than absorb a rounding one.
    ///
    /// **`cold` is a fresh deck rather than a parked slot**, and it used to be
    /// the second. Thirty frames Allocated *was* a cold slot, because Allocated
    /// did not step; it is a warm one now, so keeping it as the control would
    /// have made this test compare warm against warm and pass on an engine that
    /// warmed nothing at all.
    ///
    /// `primed != cold`, by a lot and in the direction warmth predicts: cold,
    /// every element is still at the origin and the frame is one blob; warm,
    /// they are spread over a sphere. Asserted as a ratio of lit pixels rather
    /// than as inequality, because two frames of an accumulating procedure
    /// differ at *some* bit almost whatever happens, and the point is that the
    /// difference is the warming.
    #[test]
    fn an_off_air_slot_that_goes_live_shows_warmed_state() {
        let gpu = Gpu::headless().expect("no GPU available");
        const WARM: usize = 30;

        // One slot, so the mix is that slot and there is nothing else in the frame
        // to explain a difference.
        let run = |before: Residency, live_frames: usize| -> (Vec<u16>, u64) {
            let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
            let mut deck = deck_of(&gpu, &[SEED_A]);
            deck.set_residency(0, before);
            for _ in 0..WARM {
                frame(&gpu, &mut deck, &present, 1);
            }
            deck.set_residency(0, Residency::Live);
            for _ in 0..live_frames {
                frame(&gpu, &mut deck, &present, 1);
            }
            (
                readback(&gpu, present.hdr_texture()),
                steps_taken(deck.slot(0).set()),
            )
        };

        let (primed, primed_t) = run(Residency::Priming, 1);
        let (resting, resting_t) = run(Residency::Allocated, 1);
        // The control: never off air at all.
        let (always, always_t) = run(Residency::Live, 1);
        // The other control: one frame on a deck that has never run.
        let (cold, cold_t) = {
            let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
            let mut deck = deck_of(&gpu, &[SEED_A]);
            frame(&gpu, &mut deck, &present, 1);
            (
                readback(&gpu, present.hdr_texture()),
                steps_taken(deck.slot(0).set()),
            )
        };

        assert_eq!(primed_t, WARM as u64 + 1);
        assert_eq!(always_t, WARM as u64 + 1);
        assert_eq!(
            resting_t,
            WARM as u64 + 1,
            "the Allocated slot did not step while it was off air"
        );
        assert_eq!(cold_t, 1);

        assert!(
            lit(&always) > 200,
            "the warmed material lit only {} pixels, so the ratio below is measuring \
         noise",
            lit(&always)
        );
        assert_eq!(
            primed,
            always,
            "a slot primed for {WARM} frames and then put on air is not the slot that \
         was on air for {} — priming is not reaching the same state that being \
         live reaches",
            WARM + 1
        );
        assert_eq!(
            resting,
            always,
            "a slot left at Allocated for {WARM} frames and then put on air is not the \
         slot that was on air for {} — an off-air slot is supposed to run whether \
         or not anybody asked for it (ADR-0269)",
            WARM + 1
        );

        // And the warming is visible, or the equality above would be satisfied by a
        // Priming slot that did nothing at all in a deck where being Live also did
        // nothing.
        let (warm_px, cold_px) = (lit(&primed), lit(&cold));
        assert!(
            warm_px > cold_px * 4,
            "warmed lit {warm_px} pixels and cold lit {cold_px}; cold is supposed to be \
         every element still stacked at the origin and warm a sphere, so this run \
         could not tell a primed slot from an unprimed one"
        );
    }

    /// **Determinism survives priming**, including the transition out of it.
    ///
    /// Uneven ticks, a slot that primes for a while and then goes on air, and a
    /// second slot live throughout. Two runs, bit for bit. A priming decision
    /// that read a clock shows up here — verified by making the step decision
    /// genuinely random, which this fails on.
    ///
    /// What it does **not** cover, despite being the file's determinism test: the
    /// governor's index order. `govern` is not called here and is not on the frame
    /// path, and randomising the order slots are *stepped* in is invisible to a
    /// pixel comparison anyway — slots do not interact, each renders into its own
    /// target, and the composite reads them by index afterwards. The governor's
    /// order is asserted directly in `tests/governor.rs`, where it is observable.
    #[test]
    fn priming_then_going_live_reproduces_bit_identically() {
        let gpu = Gpu::headless().expect("no GPU available");
        const TICKS: [u8; 12] = [1, 2, 1, 3, 1, 1, 4, 2, 1, 3, 2, 1];

        let run = || -> Vec<u16> {
            let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
            let mut deck = deck_of(&gpu, &[SEED_A, SEED_B]);
            deck.set_residency(1, Residency::Priming);
            for steps in TICKS {
                frame(&gpu, &mut deck, &present, steps);
            }
            // On air part-way through, at whatever `t` it warmed to.
            deck.set_residency(1, Residency::Live);
            deck.set_gain(1, 0.75);
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
            "two runs of the same ticks, the same seeds and the same priming schedule \
         composited differently"
        );
    }

    /// **A Set that arrived late warms into what it would have warmed into had
    /// it been there from the start.**
    ///
    /// A binding is resolved on every step and its value comes off the
    /// oscillator, so a Set whose `t` is behind the session's — which is any Set
    /// a build installs part-way through a set — would sample its bound param at
    /// instants its own clock never reached if it were handed the room's phase.
    /// `Set::prepare_warming` hands it the session's grid *held back by its lag
    /// in steps* instead, so what it warms into does not depend on when it
    /// arrived.
    ///
    /// The lag is manufactured through the session's clock rather than through
    /// the slot's, because a slot's clock is only advanced by the frame loop:
    /// the deck is given an oscillator that has already run, which is the same
    /// arithmetic a mid-session install produces from the other side.
    ///
    /// **This used to be the rate test.** The lag it covered was the governor's
    /// — a slot stepping one frame in four falls three steps behind every four —
    /// and there is no such rate any more (ADR-0269). The lag that is left is
    /// this one, and it is the only one `Clock::Local` still exists for.
    ///
    /// The two sequences are compared step for step rather than at the end,
    /// because the state a Set accumulates is a function of the whole sequence
    /// and two sequences can meet at the last value without having agreed
    /// anywhere else. The distinct-value assertion is not decoration: `beat`
    /// bound onto a range this test could have written as `[x, x]`, or a run
    /// shorter than a beat, would make both sequences constant and the
    /// comparison would hold against any implementation at all.
    #[test]
    fn a_set_that_is_behind_the_session_warms_into_the_same_material() {
        let gpu = Gpu::headless().expect("no GPU available");
        /// Steps each run warms for. Two beats at 120 bpm, so the bound value
        /// sweeps its whole range twice and a phase error of any size shows.
        const STEPS: usize = 60;
        /// How far behind the session the late Set starts — long enough that
        /// the phase it would read on the room's clock is nowhere near the one
        /// it reads on its own.
        const LAG: usize = 37;

        // `speed` drives an accumulating `position`, so this is a binding whose
        // value the warmed state actually depends on rather than one that is
        // merely written to a uniform.
        let warm = |lag: usize| -> Vec<f32> {
            let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
            let mut set = build(&gpu, SEED_A);
            assert!(
                set.bind(Binding::new(
                    Kind::L1,
                    "speed",
                    "beat",
                    Curve::Lin,
                    [0.0, 8.0]
                ))
                .attached(),
                "`speed` is a declared L1 param of CREEP"
            );
            let mut deck = Deck::new(&gpu.device, vec![HotSwap::fixed(set)], WIDTH, HEIGHT);
            let mut signals = Signals::new(120.0, u64::from(SEED_A));
            for _ in 0..lag {
                signals.advance(1, DT);
            }
            deck.set_signals(signals);
            deck.set_residency(0, Residency::Priming);

            let mut seen = Vec::with_capacity(STEPS);
            let mut taken = 0;
            while taken < STEPS as u64 {
                frame(&gpu, &mut deck, &present, 1);
                let set = deck.slot(0).set();
                if steps_taken(set) != taken {
                    taken = steps_taken(set);
                    seen.push(set.bound().next().expect("one binding").1);
                }
            }
            seen
        };

        let (on_time, late) = (warm(0), warm(LAG));

        assert_eq!(
            on_time.len(),
            STEPS,
            "the on-time run did not take a step a frame"
        );
        assert!(
            on_time.iter().any(|&v| v != on_time[0]),
            "every value in the run is {}, so this comparison would hold against \
         an implementation that read any phase at all",
            on_time[0]
        );
        assert_eq!(
            on_time, late,
            "a Set that started {LAG} steps behind the session read a different signal \
         than the same Set that started with it — when a Set arrived is deciding \
         what it warms into"
        );
    }

    /// **A slot that is not behind reads exactly what being on air reads.**
    ///
    /// The sharper half of the claim above, and the one that catches the error
    /// rate-invariance alone cannot see: two warming runs agree with each other
    /// under any clock that is a function of the step index, including one a frame
    /// early and one at the wrong scale entirely. Only a Live run says *which*
    /// instant is the right one.
    ///
    /// A slot that came up with the deck steps whenever the session steps, so
    /// its lag is zero and it must read the session's oscillator itself — not a
    /// position recomputed from its own `t`, which costs an f32 rounding the
    /// session's `t` never took and diverges inside a second. Bit for bit, on
    /// every step, because the identity `priming_then_going_live_reproduces_bit_
    /// identically` rests on is bit-exact and a bound Set has to keep it too.
    #[test]
    fn a_slot_that_is_not_behind_reads_the_same_signal_as_being_on_air() {
        let gpu = Gpu::headless().expect("no GPU available");
        /// Three beats at 120 bpm. Long enough that an f32-derived clock has
        /// diverged — it first does so around step 29 — several times over.
        const STEPS: usize = 90;

        let run = |residency: Residency| -> Vec<f32> {
            let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
            let mut set = build(&gpu, SEED_A);
            assert!(set
                .bind(Binding::new(
                    Kind::L1,
                    "speed",
                    "beat",
                    Curve::Lin,
                    [0.0, 8.0]
                ))
                .attached());
            let mut deck = Deck::new(&gpu.device, vec![HotSwap::fixed(set)], WIDTH, HEIGHT);
            deck.set_signals(Signals::new(120.0, u64::from(SEED_A)));
            deck.set_residency(0, residency);

            (0..STEPS)
                .map(|_| {
                    frame(&gpu, &mut deck, &present, 1);
                    deck.slot(0).set().bound().next().expect("one binding").1
                })
                .collect()
        };

        let (live, warming) = (run(Residency::Live), run(Residency::Priming));
        assert!(
            live.iter().any(|&v| v != live[0]),
            "the bound value never moved, so this comparison holds against anything"
        );
        let differs = live
            .iter()
            .zip(&warming)
            .position(|(a, b)| a != b)
            .map(|i| {
                format!(
                    "first at step {i}: live {} vs warming {}",
                    live[i], warming[i]
                )
            });
        assert!(
            differs.is_none(),
            "a slot warming off air read a different signal than the same slot on \
         air — {}",
            differs.unwrap_or_default()
        );
    }
}
