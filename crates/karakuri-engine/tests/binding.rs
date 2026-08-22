//! Bindings, through the whole path: a `Deck` frame, a real `Set`, and the
//! uniform write at the end of it.
//!
//! The semantics — the curve vocabulary, the confidence blend, what an unknown
//! signal does — are asserted where they live, in `binding.rs`'s own tests,
//! because none of them needs a GPU and a test that needs one is a test that
//! gets skipped. What is asserted *here* is everything those cannot see:
//!
//! - a binding actually reaches the parameter a Set writes, and reaches the
//!   spawn accumulator too, which is a second consumer of the same value;
//! - the manual value survives, so a param that is bound and also given a
//!   `--param` has an answer that does not depend on frame ordering;
//! - the deck advances **one** oscillator, once per frame, so two Live slots
//!   see the same phase;
//! - the same tick sequence and the same seed produce the same parameter
//!   values, bit for bit, through a noise binding.

use karakuri_engine::binding::{blend, Curve, Signals};
use karakuri_engine::deck::{Deck, Residency};
use karakuri_engine::swap::HotSwap;
use karakuri_engine::{Binding, Gpu, Present, Set};
use karakuri_ir::typed::Checked;
use karakuri_ir::Kind;
use karakuri_signal::{AudioFrame, NoiseConfig, NoiseKind};

const WIDTH: u32 = 128;
const HEIGHT: u32 = 128;
const CAPACITY: u32 = 4096;
const SEED: u32 = 19_274;
const BPM: f32 = 120.0;

/// 120 bpm at `dt = 1/60` is exactly 30 frames to the beat.
const FRAMES_PER_BEAT: usize = 30;

/// A `spawn` block, so that `spawn_rate` is a real param with the accumulator
/// behind it — that second consumer is the whole reason `docs/ir-spec.md`
/// answers irregular spawning with a noise binding.
const L1: &str = r#"
proc sparks {
  kind     L1
  topology points
  capacity [1024, 262144] = 4096

  param radius     : float [0.1, 8.0]      = 2.5
  param spawn_rate : float [0.0, 40000.0]  = 1000.0

  emit position, age

  spawn {
    let u = hash1(seed);
    let v = hash1(seed + 1000u);
    position = sphere_point(u, v) * radius;
    age      = 0.0;
  }

  element {
    position = position;
    age      = age + dt;
    if age > 4.0 { kill(); }
  }
}
"#;

const L4: &str = r#"
proc soft_points {
  kind  L4
  blend additive

  consumes position

  param point_scale : float [0.5, 40.0] = 8.0
  param hue         : float [0.0, 1.0]  = 0.6

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_size = point_scale;
  }

  fragment {
    let d = length(point_coord * 2.0 - 1.0);
    let a = max(0.0, 1.0 - d);
    let c = hsv_to_rgb(vec3(hue, 0.7, 1.0));
    color = vec4(c, a);
  }
}
"#;

/// The same L4, except that it declares a `radius` of its own — a name `L1`
/// already declares. Nothing in the IR forbids that: the two procedures are
/// written independently and neither can see the other's parameter names.
const L4_CLASHING: &str = r#"
proc soft_points {
  kind  L4
  blend additive

  consumes position

  param point_scale : float [0.5, 40.0] = 8.0
  param radius      : float [0.1, 8.0]  = 7.5

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_size = point_scale * radius;
  }

  fragment {
    let d = length(point_coord * 2.0 - 1.0);
    color = vec4(vec3(max(0.0, 1.0 - d)), 1.0);
  }
}
"#;

fn compile(src: &str) -> Checked {
    let proc = karakuri_ir::parse(src).unwrap_or_else(|errs| panic!("{}", render(&errs, src)));
    let checked =
        karakuri_ir::check::check(&proc).unwrap_or_else(|errs| panic!("{}", render(&errs, src)));
    karakuri_ir::cost::estimate(&checked).unwrap_or_else(|errs| panic!("{}", render(&errs, src)));
    checked
}

fn render(errs: &[karakuri_ir::IrError], src: &str) -> String {
    errs.iter()
        .map(|e| e.render(src))
        .collect::<Vec<_>>()
        .join("\n")
}

fn build(gpu: &Gpu) -> Set {
    Set::build(
        &gpu.device,
        &gpu.queue,
        &compile(L1),
        &compile(L4),
        CAPACITY,
        SEED,
    )
    .expect("the pair is compatible and the capacity is in range")
}

fn deck_of(gpu: &Gpu, sets: Vec<Set>, seed: u64) -> (Deck, Present) {
    let present = Present::new(&gpu.device, wgpu::TextureFormat::Rgba16Float, WIDTH, HEIGHT);
    let swaps = sets.into_iter().map(HotSwap::fixed).collect();
    let mut deck = Deck::new(&gpu.device, swaps, WIDTH, HEIGHT);
    deck.set_signals(Signals::new(BPM, seed));
    (deck, present)
}

fn frame(gpu: &Gpu, deck: &mut Deck, present: &Present, steps: u8) {
    let mut f = deck.begin_frame(&gpu.device, &gpu.queue);
    f.render(present.hdr_view(), present.size(), steps);
    f.finish();
    gpu.device.poll(wgpu::PollType::Wait).expect("poll");
}

/// The mix, read back. `Rgba16Float` in and out with one slot at unity gain
/// has no rounding anywhere in it, so these comparisons are exact.
fn readback(gpu: &Gpu, texture: &wgpu::Texture) -> Vec<u16> {
    let (width, height) = (texture.width(), texture.height());
    let bytes_per_row = width * 8;
    assert_eq!(bytes_per_row % 256, 0, "row pitch must be 256-byte aligned");

    let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("binding readback"),
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
    gpu.device.poll(wgpu::PollType::Wait).expect("poll");
    let data = slice.get_mapped_range();
    let out = data
        .chunks_exact(2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .collect();
    drop(data);
    buffer.unmap();
    out
}

fn value_of(deck: &Deck, slot: usize, key: &str) -> f32 {
    deck.slot(slot)
        .set()
        .bound()
        .find(|(name, _)| *name == key)
        .unwrap_or_else(|| panic!("`{key}` is not bound on slot {slot}"))
        .1
}

/// **The signal path reads no clock, and now part of it lives here.**
///
/// `karakuri-signal` has a standing scan of its own source for exactly this
/// (`tests/no_clock_access.rs`), because "rendering reads only the local
/// oscillator" is an invariant a doc comment cannot hold. This change moved a
/// piece of that path into `karakuri-engine`: `binding.rs` owns the session
/// oscillator and is what a future edit would reach for if it wanted to smooth
/// a binding over real time. The scan cannot cover the whole crate — `swap.rs`
/// measures frame intervals on purpose, and must — so it covers the one file
/// whose whole claim is that it does not.
#[test]
fn the_binding_path_never_reads_a_clock() {
    const FORBIDDEN: &[&str] = &[
        "Instant::now",
        "SystemTime::now",
        "std::time::Instant",
        "std::time::SystemTime",
        "chrono::Utc::now",
        "chrono::Local::now",
        "web_time",
    ];
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/binding.rs");
    let source = std::fs::read_to_string(&path).expect("read src/binding.rs");
    // The scan is worth nothing if the file moved out from under it.
    assert!(
        source.contains("fn resolve"),
        "{} is not the binding source any more",
        path.display()
    );
    for needle in FORBIDDEN {
        assert!(
            !source.contains(needle),
            "{} contains {needle:?} — a binding's whole input is the tick \
             sequence and the seed, or the record stream stops reproducing it",
            path.display()
        );
    }
}

// -- measured signals ------------------------------------------------------

/// **The property this whole slice exists for.** One binding, one name, two
/// providers: measured, it decides the parameter outright; invented, it moves
/// it a tenth as far — and not one line of the binding changed to make that
/// true, because the only thing that differs is the confidence that came back
/// from `sample`.
///
/// No GPU: this is the arithmetic, and the tests above already show the same
/// arithmetic reaching a uniform.
#[test]
fn a_measured_signal_moves_a_param_fully_where_the_invented_one_moves_a_tenth() {
    let mut signals = Signals::new(BPM, u64::from(SEED));
    signals.advance(7, 1.0 / 60.0);

    // Whatever the invented `energy` happens to be at this instant. The
    // measured frame is given *the same value*, so the only difference between
    // the two runs is how much it is believed.
    let invented_sample = signals.sample("energy");
    assert_eq!(
        invented_sample.confidence, 0.1,
        "energy should be invented here"
    );

    let manual = 1.0;
    let range = [0.0, 10.0];
    let mut binding = Binding::new(Kind::L1, "turbulence", "energy", Curve::Lin, range);
    let quiet = binding.resolve(&signals, manual);

    signals.set_audio(Some(AudioFrame {
        energy: invented_sample.value,
        onset: 0.0,
        bands: [0.0; 8],
        band_count: 8,
        confidence: 1.0,
    }));
    let measured = binding.resolve(&signals, manual);

    // Measured: the signal decides it outright.
    assert_eq!(measured, binding.map(invented_sample.value));
    // Invented: a tenth of the same distance.
    let ratio = (quiet - manual) / (measured - manual);
    assert!(
        (measured - manual).abs() > 1.0,
        "the reference move is too small to measure a tenth of"
    );
    assert!(
        (ratio - 0.1).abs() < 1e-5,
        "the invented signal moved {ratio} of the measured one's distance, not 0.1"
    );
}

/// A silent room is not an absent microphone, at the parameter. Measured
/// silence pins the param at the bottom of its range — because it *is* the
/// bottom, and the measurement says so — where no microphone leaves it a tenth
/// of the way from its own value.
#[test]
fn measured_silence_takes_a_param_somewhere_no_microphone_never_does() {
    let mut signals = Signals::new(BPM, u64::from(SEED));
    signals.advance(7, 1.0 / 60.0);
    let manual = 6.0;
    let range = [0.0, 10.0];
    let mut binding = Binding::new(Kind::L1, "turbulence", "energy", Curve::Lin, range);

    let no_microphone = binding.resolve(&signals, manual);

    signals.set_audio(Some(AudioFrame::silent(8)));
    let silence = binding.resolve(&signals, manual);
    assert_eq!(silence, range[0], "measured silence should reach the floor");
    assert!(
        (no_microphone - manual).abs() < (silence - manual).abs(),
        "no microphone moved the param further than a measured silence did"
    );

    // And an interface pulled out mid-set: the same zeroes, no confidence, and
    // the param is handed back to whoever set it — exactly, not nearly.
    signals.set_audio(Some(AudioFrame::nothing(8)));
    assert_eq!(binding.resolve(&signals, manual), manual);
}

/// A measurement going stale hands the parameter back gradually rather than
/// dropping it. Half-believed is half way, by the same `lerp` everything else
/// uses.
#[test]
fn a_staling_measurement_gives_the_param_back_in_proportion() {
    let mut signals = Signals::new(BPM, u64::from(SEED));
    signals.advance(7, 1.0 / 60.0);
    let manual = 4.0;
    let mut binding = Binding::new(Kind::L1, "turbulence", "energy", Curve::Lin, [0.0, 10.0]);

    let fresh = AudioFrame {
        energy: 0.9,
        onset: 0.0,
        bands: [0.0; 8],
        band_count: 8,
        confidence: 1.0,
    };
    signals.set_audio(Some(fresh));
    let full = binding.resolve(&signals, manual);

    let mut previous = full;
    for confidence in [0.75, 0.5, 0.25, 0.0] {
        signals.set_audio(Some(fresh.with_confidence(confidence)));
        let value = binding.resolve(&signals, manual);
        assert!(
            (value - manual).abs() < (previous - manual).abs(),
            "confidence {confidence} did not give more of the param back"
        );
        previous = value;
    }
    assert_eq!(previous, manual, "no confidence left should be no effect");
}

/// A signal with no provider is untouched by any of this. Every name the audio
/// frame does not measure answers exactly what it answered before there was a
/// microphone in the room — including the bands past the ones measured.
#[test]
fn measuring_something_does_not_disturb_a_signal_nobody_measures() {
    let mut without = Signals::new(BPM, u64::from(SEED));
    without.advance(7, 1.0 / 60.0);
    let mut with = without;
    with.set_audio(Some(AudioFrame {
        energy: 0.9,
        onset: 0.4,
        bands: [0.5; 8],
        band_count: 2,
        confidence: 1.0,
    }));

    let manual = 2.5;
    for name in [
        "beat",
        "bar",
        "bpm",
        "band4",
        "band7",
        "nothing_provides_this",
    ] {
        let mut a = Binding::new(Kind::L1, "turbulence", name, Curve::Pow2, [0.0, 10.0]);
        let mut b = Binding::new(Kind::L1, "turbulence", name, Curve::Pow2, [0.0, 10.0]);
        assert_eq!(
            a.resolve(&without, manual),
            b.resolve(&with, manual),
            "`{name}` changed when something else started being measured"
        );
    }

    // ...while the two names that *are* measured did change, or the assertion
    // above would hold for a frame that was being ignored entirely.
    for name in ["energy", "band1"] {
        let mut a = Binding::new(Kind::L1, "turbulence", name, Curve::Pow2, [0.0, 10.0]);
        let mut b = Binding::new(Kind::L1, "turbulence", name, Curve::Pow2, [0.0, 10.0]);
        assert_ne!(
            a.resolve(&without, manual),
            b.resolve(&with, manual),
            "`{name}` is supposed to be measured here"
        );
    }
}

// Eleven of the sixteen tests here take a device; the five above do not, because
// what they check is the binding table before anything is bound to a uniform.
// Keeping the split means `cargo test -p karakuri-engine -- --skip gpu::` still
// answers something about bindings — see
// `tests/gpu_tests_are_under_mod_gpu.rs` for the convention and its guard.
mod gpu {
    use super::*;

    /// A binding reaches the parameter, through a real frame, and moves it with
    /// the session oscillator's phase rather than merely over time.
    #[test]
    fn a_beat_binding_moves_a_param_in_time_with_the_deck_oscillator() {
        let gpu = Gpu::headless().expect("no GPU");
        let mut set = build(&gpu);
        assert_eq!(
            set.set_param("radius", 2.5),
            1,
            "the param must be declared for the write to mean anything"
        );
        assert!(
            set.bind(Binding::new(
                Kind::L1,
                "radius",
                "beat",
                Curve::Lin,
                [1.0, 5.0]
            ))
            .attached(),
            "`radius` is a declared L1 param"
        );
        let (mut deck, present) = deck_of(&gpu, vec![set], 1);

        let mut values = Vec::new();
        for _ in 0..(FRAMES_PER_BEAT * 3) {
            frame(&gpu, &mut deck, &present, 1);
            values.push(value_of(&deck, 0, "radius"));
        }

        // It moves at all, and across most of the range it was given.
        let min = values.iter().cloned().fold(f32::INFINITY, f32::min);
        let max = values.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        assert!(
            min < 1.5 && max > 4.5,
            "a bound param spanned only {min}..{max} of [1, 5]"
        );

        // And it moves at the *oscillator's* period. A param drifting for any
        // other reason would fail this while passing the span check above.
        for i in 0..FRAMES_PER_BEAT {
            let a = values[i];
            let b = values[i + FRAMES_PER_BEAT];
            assert!(
                (a - b).abs() < 1e-3,
                "frame {i} and one beat later differ: {a} vs {b}"
            );
        }

        // It is the deck's own phase, not a second clock: the value equals what
        // the deck's `beat` signal says at the instant of the frame just rendered.
        let beat = deck.signals().sample("beat");
        assert_eq!(beat.confidence, 1.0, "beat comes off the local oscillator");
        let expected = blend(2.5, 1.0 + 4.0 * beat.value, 1.0);
        assert_eq!(*values.last().expect("frames were rendered"), expected);
    }
    /// **What a binding writes reaches the shader**, and reaches it by the same
    /// path a `--param` override takes.
    ///
    /// Every other test here reads the value a binding resolved to, which is one
    /// step short of the claim: a `prepare` that resolved bindings correctly and
    /// then packed the manual values into the uniform would pass all of them.
    /// So this one compares rendered pixels, in both directions and in both
    /// uniform buffers — L1's `radius` and L4's `hue`:
    ///
    /// - a Set with a param bound to a constant renders **bit for bit** the same
    ///   as a Set with that param simply set to the same number, which is what
    ///   "the same path a `--param` takes" means;
    /// - and both differ from the same Set left at its manual value, which is
    ///   what stops the first comparison from passing on two identical blanks.
    #[test]
    fn a_bound_param_reaches_the_shader_by_the_same_path_a_param_override_takes() {
        let gpu = Gpu::headless().expect("no GPU");

        // A constant range and a certain signal, so the written value does not
        // depend on the phase the comparison happened to be taken at.
        // A steady spawn rate, so there is material on screen to compare at all.
        const RATE: f32 = 20_000.0;
        let bound_to = |key: &str, layer, value: f32, manual: f32| {
            let mut set = build(&gpu);
            assert_eq!(
                set.set_param("spawn_rate", RATE),
                1,
                "the param must be declared for the write to mean anything"
            );
            assert_eq!(
                set.set_param(key, manual),
                1,
                "the param must be declared for the write to mean anything"
            );
            assert!(set
                .bind(Binding::new(layer, key, "beat", Curve::Lin, [value, value]))
                .attached());
            set
        };
        let set_to = |key: &str, value: f32| {
            let mut set = build(&gpu);
            assert_eq!(
                set.set_param("spawn_rate", RATE),
                1,
                "the param must be declared for the write to mean anything"
            );
            assert_eq!(
                set.set_param(key, value),
                1,
                "the param must be declared for the write to mean anything"
            );
            set
        };
        let render_of = |set: Set| {
            let (mut deck, present) = deck_of(&gpu, vec![set], 6);
            for _ in 0..4 {
                frame(&gpu, &mut deck, &present, 1);
            }
            readback(&gpu, present.hdr_texture())
        };

        for (key, layer, bound, manual) in
            [("radius", Kind::L1, 5.5, 2.5), ("hue", Kind::L4, 0.05, 0.6)]
        {
            let by_binding = render_of(bound_to(key, layer, bound, manual));
            let by_override = render_of(set_to(key, bound));
            let unbound = render_of(set_to(key, manual));

            assert_ne!(
                by_binding, unbound,
                "`{key}` bound to {bound} rendered the same as `{key}` left at {manual}: \
             the binding never reached the uniform"
            );
            assert_eq!(
                by_binding, by_override,
                "`{key}` bound to {bound} and `{key}` set to {bound} rendered differently: \
             a binding is supposed to be the same uniform write an override is"
            );
        }
    }
    /// `spawn_rate` has a **second** consumer — the spawn accumulator, which is
    /// not a uniform — and a binding has to reach that one too. A `spawn_rate`
    /// whose uniform took the bound value while its accumulator took the manual
    /// one would be the same param meaning two things in one frame, and it is the
    /// exact case `docs/ir-spec.md`'s Spawn timing rests on.
    #[test]
    fn a_binding_reaches_the_spawn_accumulator_and_not_only_the_uniform() {
        let gpu = Gpu::headless().expect("no GPU");

        // Nothing spawns at all without the binding: `spawn_rate` is zero by hand.
        let mut unbound = build(&gpu);
        assert_eq!(
            unbound.set_param("spawn_rate", 0.0),
            1,
            "the param must be declared for the write to mean anything"
        );
        let (mut deck, present) = deck_of(&gpu, vec![unbound], 5);
        for _ in 0..30 {
            frame(&gpu, &mut deck, &present, 1);
        }
        assert_eq!(
            deck.slot(0).set().live_count(&gpu.device, &gpu.queue),
            0,
            "a manual spawn_rate of zero spawned something"
        );

        // The same Set, the same manual zero, plus a certain binding pinned to a
        // constant rate: the range's two ends are equal, so the count depends on
        // the binding being read rather than on the phase it was read at.
        let mut bound = build(&gpu);
        assert_eq!(
            bound.set_param("spawn_rate", 0.0),
            1,
            "the param must be declared for the write to mean anything"
        );
        assert!(bound
            .bind(Binding::new(
                Kind::L1,
                "spawn_rate",
                "beat",
                Curve::Lin,
                [3000.0, 3000.0]
            ))
            .attached());
        let (mut deck, present) = deck_of(&gpu, vec![bound], 5);
        for _ in 0..30 {
            frame(&gpu, &mut deck, &present, 1);
        }
        // 3000 per second for half a second, give or take the accumulator's carry.
        let live = deck.slot(0).set().live_count(&gpu.device, &gpu.queue);
        assert!(
            (1400..=1600).contains(&live),
            "the spawn accumulator produced {live} elements, not the bound rate's ~1500"
        );
    }
    /// A param that is bound **and** given a manual value. The manual value is the
    /// base of the blend and is never overwritten, so the answer does not depend
    /// on which of the two happened last.
    #[test]
    fn a_manual_value_is_kept_and_blended_from_rather_than_overwritten() {
        let gpu = Gpu::headless().expect("no GPU");
        let mut set = build(&gpu);
        // What `--param radius=7.0` does.
        assert_eq!(
            set.set_param("radius", 7.0),
            1,
            "the param must be declared for the write to mean anything"
        );
        // An *invented* signal, so the manual value keeps 90% of the weight and
        // its survival is observable in the written value rather than only in the
        // map it came from.
        assert!(set
            .bind(Binding::new(
                Kind::L1,
                "radius",
                "energy",
                Curve::Lin,
                [0.0, 1.0]
            ))
            .attached());
        let (mut deck, present) = deck_of(&gpu, vec![set], 2);

        for _ in 0..10 {
            frame(&gpu, &mut deck, &present, 1);
        }

        let manual = deck.slot(0).set().param("radius").expect("declared");
        assert_eq!(manual, 7.0, "the binding overwrote the manual value");

        let energy = deck.signals().sample("energy");
        assert_eq!(energy.confidence, 0.1, "energy is supposed to be invented");
        assert_eq!(
            value_of(&deck, 0, "radius"),
            blend(7.0, energy.value, energy.confidence)
        );
        // Ten percent of the way from 7.0 towards a value in [0, 1]: still far
        // nearer the manual value than the signal, which is what confidence 0.1
        // has to mean for this not to be theatre.
        assert!(value_of(&deck, 0, "radius") > 6.2);
    }
    /// A signal no provider has ever heard of. No failure, no panic, and the param
    /// stands exactly where it was put — bit for bit, across a real frame.
    #[test]
    fn a_signal_with_no_provider_leaves_the_param_where_it_was_put() {
        let gpu = Gpu::headless().expect("no GPU");
        let mut set = build(&gpu);
        assert_eq!(
            set.set_param("radius", 3.25),
            1,
            "the param must be declared for the write to mean anything"
        );
        assert!(set
            .bind(Binding::new(
                Kind::L1,
                "radius",
                "mic_level",
                Curve::Pow2,
                [100.0, 200.0]
            ))
            .attached());
        let (mut deck, present) = deck_of(&gpu, vec![set], 3);

        for _ in 0..20 {
            frame(&gpu, &mut deck, &present, 1);
            assert_eq!(
                value_of(&deck, 0, "radius"),
                3.25,
                "a signal with no provider moved a param"
            );
        }
        assert_eq!(deck.slot(0).set().param("radius").expect("declared"), 3.25);
    }
    /// The same tick sequence and the same seed reproduce every bound value bit
    /// for bit, through a noise binding — so a binding is inside the determinism
    /// invariant rather than beside it.
    #[test]
    fn the_same_ticks_and_seed_reproduce_a_noise_binding_bit_for_bit() {
        let gpu = Gpu::headless().expect("no GPU");
        // Ragged on purpose: the tick sequence, not the frame count, is what has
        // to be reproduced.
        let ticks = [1u8, 2, 1, 1, 3, 1, 2, 4, 1];

        let run = |seed: u64| {
            let mut set = build(&gpu);
            set.bind(
                Binding::new(
                    Kind::L1,
                    "spawn_rate",
                    "noise",
                    Curve::Lin,
                    [4000.0, 16000.0],
                )
                .with_noise(NoiseConfig {
                    kind: NoiseKind::Fbm { octaves: 4 },
                    rate: 0.5,
                    stream: 3,
                }),
            )
            .attached();
            set.bind(Binding::new(
                Kind::L4,
                "hue",
                "bar",
                Curve::Smooth,
                [0.0, 1.0],
            ))
            .attached();
            let (mut deck, present) = deck_of(&gpu, vec![set], seed);
            let mut out = Vec::new();
            for steps in ticks {
                frame(&gpu, &mut deck, &present, steps);
                out.push(value_of(&deck, 0, "spawn_rate").to_bits());
                out.push(value_of(&deck, 0, "hue").to_bits());
            }
            out
        };

        assert_eq!(run(7), run(7), "two identical runs disagreed");
        assert_ne!(run(7), run(8), "changing the seed changed nothing");
    }
    /// One oscillator per deck, not one per Set: two Live slots carrying the same
    /// binding write the same value, on the same frame.
    #[test]
    fn every_live_slot_reads_the_same_session_phase() {
        let gpu = Gpu::headless().expect("no GPU");
        let sets: Vec<Set> = (0..2)
            .map(|_| {
                let mut set = build(&gpu);
                assert_eq!(
                    set.set_param("radius", 2.0),
                    1,
                    "the param must be declared for the write to mean anything"
                );
                assert!(set
                    .bind(Binding::new(
                        Kind::L1,
                        "radius",
                        "beat",
                        Curve::Pow2,
                        [1.0, 5.0]
                    ))
                    .attached());
                set
            })
            .collect();
        let (mut deck, present) = deck_of(&gpu, sets, 4);

        for _ in 0..17 {
            frame(&gpu, &mut deck, &present, 1);
            assert_eq!(
                value_of(&deck, 0, "radius"),
                value_of(&deck, 1, "radius"),
                "two slots disagreed about the session's phase"
            );
        }

        // And an off-air slot does not advance a phase of its own while parked:
        // the clock is the session's, so it rejoins the beat the deck is on rather
        // than resuming one it kept to itself.
        deck.set_residency(1, Residency::Allocated);
        let parked = value_of(&deck, 1, "radius");
        for _ in 0..7 {
            frame(&gpu, &mut deck, &present, 1);
        }
        assert_eq!(
            value_of(&deck, 1, "radius"),
            parked,
            "an Allocated slot kept resolving bindings"
        );
        deck.set_residency(1, Residency::Live);
        frame(&gpu, &mut deck, &present, 1);
        assert_eq!(
            value_of(&deck, 0, "radius"),
            value_of(&deck, 1, "radius"),
            "a slot brought back on air did not rejoin the session's phase"
        );
    }
    /// The session clock advances by exactly what the material does, on the two
    /// frames where "exactly" is not "whatever was passed in".
    ///
    /// `Set::prepare` clamps `steps` to `MAX_STEPS` — past that the simulation is
    /// allowed to fall behind rather than catch up — so the oscillator has to clamp
    /// identically or a loaded frame moves the beat further than the elements it is
    /// supposed to be in time with. And a zero-step frame must move neither. Both
    /// are silent when wrong: the picture keeps updating and the drift only shows
    /// up as a beat landing in the wrong place after a load spike.
    #[test]
    fn the_session_clock_advances_by_the_same_clamped_steps_the_slots_do() {
        let gpu = Gpu::headless().expect("no GPU");

        let bound = |gpu: &Gpu| {
            let mut set = build(gpu);
            assert_eq!(
                set.set_param("radius", 2.0),
                1,
                "the param must be declared for the write to mean anything"
            );
            assert!(set
                .bind(Binding::new(
                    Kind::L1,
                    "radius",
                    "beat",
                    Curve::Lin,
                    [1.0, 5.0]
                ))
                .attached());
            set
        };

        // Over the cap and at it: the frames the two decks run are different
        // numbers, and the phase they reach has to be the same one.
        let over = karakuri_engine::set::MAX_STEPS + 3;
        let (mut fast, present_a) = deck_of(&gpu, vec![bound(&gpu)], 8);
        let (mut capped, present_b) = deck_of(&gpu, vec![bound(&gpu)], 8);
        for _ in 0..12 {
            frame(&gpu, &mut fast, &present_a, over);
            frame(
                &gpu,
                &mut capped,
                &present_b,
                karakuri_engine::set::MAX_STEPS,
            );
            assert_eq!(
                value_of(&fast, 0, "radius"),
                value_of(&capped, 0, "radius"),
                "a frame of {over} steps moved the session clock further than the {} \
             steps its slots took",
                karakuri_engine::set::MAX_STEPS
            );
        }

        // A zero-step frame renders and measures, and moves nothing.
        let held = value_of(&capped, 0, "radius");
        for _ in 0..5 {
            frame(&gpu, &mut capped, &present_b, 0);
            assert_eq!(
                value_of(&capped, 0, "radius"),
                held,
                "a zero-step frame advanced the session clock"
            );
        }
    }
    /// **`Binding::layer` is only as real as the map it indexes**, and now the map
    /// is indexed by it.
    ///
    /// `Set::params` used to be one flat `name -> value` map across both
    /// procedures, built by chaining L1's params and L4's into one collection. A
    /// name both layers declared was therefore one value: the second declaration's
    /// default silently overwrote the first's, and a binding on either one blended
    /// from a base the *other* procedure declared. A binding could not fix that from
    /// where it sits, so the collision was refused at build time — which was the
    /// right call while the map was flat, and which also forbade several renderers
    /// over one geometry, since every L4 in `examples/` declares `exposure`.
    ///
    /// There is a map per node now, so this asserts what the refusal was standing
    /// in for. `sparks` declares `radius = 2.5` and `soft_points` declares
    /// `radius = 7.5`, and the pair has to keep both.
    ///
    /// **The bindings are what make it a real claim rather than a bookkeeping one.**
    /// Both are attached to a signal nothing provides, which comes back with
    /// confidence 0.0, and step 4 of the binding path then writes the param's own
    /// value unchanged — so each binding resolves to exactly the default of the node
    /// it names. One shared map would give both the same number.
    #[test]
    fn a_param_name_two_nodes_declare_is_two_values_one_per_node() {
        let gpu = Gpu::headless().expect("no GPU");
        let mut set = Set::build(
            &gpu.device,
            &gpu.queue,
            &compile(L1),
            &compile(L4_CLASHING),
            CAPACITY,
            SEED,
        )
        .expect("two nodes may each declare a `radius`");

        let mut declared: Vec<(Kind, f32)> = set
            .params()
            .filter(|(_, _, name, _)| *name == "radius")
            .map(|(layer, _, _, value)| (layer, value))
            .collect();
        declared.sort_by_key(|(layer, _)| format!("{layer:?}"));
        assert_eq!(
            declared,
            vec![(Kind::L1, 2.5), (Kind::L4, 7.5)],
            "the two declarations of `radius` did not survive as two values"
        );

        // A name with no address is every node that declares it — one knob, both
        // layers — which is what a `--param` and a `param` record ask for.
        assert_eq!(
            set.set_param("radius", 4.0),
            2,
            "a bare name must move both"
        );
        assert_eq!(
            set.set_param("point_scale", 9.0),
            1,
            "only the L4 declares it"
        );
        assert_eq!(set.set_param("nothing_declares_this", 1.0), 0);

        // Back to two distinct values by rebuilding, since a bare name cannot set
        // them apart — which is the addressed write the record vocabulary still
        // owes, and deliberately not this commit's business.
        let mut set = Set::build(
            &gpu.device,
            &gpu.queue,
            &compile(L1),
            &compile(L4_CLASHING),
            CAPACITY,
            SEED,
        )
        .expect("two nodes may each declare a `radius`");
        for layer in [Kind::L1, Kind::L4] {
            assert!(
                set.bind(Binding::new(
                    layer,
                    "radius",
                    "nothing_measures_this",
                    Curve::Lin,
                    [0.0, 100.0]
                ))
                .attached(),
                "`radius` is declared on {layer:?}"
            );
        }

        let (mut deck, present) = deck_of(&gpu, vec![set], 1);
        frame(&gpu, &mut deck, &present, 1);

        let resolved = |layer: Kind| -> f32 {
            deck.slot(0)
                .set()
                .bindings()
                .iter()
                .find(|b| b.layer == layer)
                .expect("both layers are bound")
                .value()
        };
        assert_eq!(
            (resolved(Kind::L1), resolved(Kind::L4)),
            (2.5, 7.5),
            "a binding blended from the other node's declaration of `radius`"
        );
    }
    /// `bind` refuses a param the layer does not declare, rather than attaching a
    /// binding that writes nowhere. Nothing here asks whether a *provider* exists
    /// — this is about the artifact's own declaration.
    #[test]
    fn binding_a_param_the_layer_does_not_declare_is_refused() {
        let gpu = Gpu::headless().expect("no GPU");
        let mut set = build(&gpu);

        assert!(!set
            .bind(Binding::new(
                Kind::L1,
                "no_such_param",
                "beat",
                Curve::Lin,
                [0.0, 1.0]
            ))
            .attached());
        // `hue` is L4's, so an L1 binding to it must not attach.
        assert!(!set
            .bind(Binding::new(
                Kind::L1,
                "hue",
                "beat",
                Curve::Lin,
                [0.0, 1.0]
            ))
            .attached());
        assert!(set
            .bind(Binding::new(
                Kind::L4,
                "hue",
                "beat",
                Curve::Lin,
                [0.0, 1.0]
            ))
            .attached());
        assert_eq!(set.bindings().len(), 1);

        // A second binding on the same param replaces the first rather than
        // stacking behind it, so the write never depends on attachment order.
        assert!(set
            .bind(Binding::new(
                Kind::L4,
                "hue",
                "bar",
                Curve::Sqrt,
                [0.0, 1.0]
            ))
            .attached());
        assert_eq!(set.bindings().len(), 1);
        assert_eq!(set.bindings()[0].signal, "bar");
    }
    /// The measured path reaches a real uniform, not only the arithmetic: the same
    /// `energy` binding on a built Set writes a different value once a frame is
    /// measured, through `Set::prepare` and the deck, with nothing else changed.
    #[test]
    fn a_measured_signal_reaches_the_uniform_a_param_override_would_write() {
        let gpu = Gpu::headless().expect("no GPU");
        let mut set = build(&gpu);
        assert!(set
            .bind(Binding::new(
                Kind::L1,
                "radius",
                "energy",
                Curve::Lin,
                [0.5, 8.0]
            ))
            .attached());
        let (mut deck, present) = deck_of(&gpu, vec![set], u64::from(SEED));

        frame(&gpu, &mut deck, &present, 1);
        let invented = value_of(&deck, 0, "radius");

        let mut signals = *deck.signals();
        signals.set_audio(Some(AudioFrame {
            energy: 1.0,
            onset: 0.0,
            bands: [0.0; 8],
            band_count: 8,
            confidence: 1.0,
        }));
        deck.set_signals(signals);
        frame(&gpu, &mut deck, &present, 1);
        let measured = value_of(&deck, 0, "radius");

        assert_eq!(
            measured, 8.0,
            "a measured energy of 1.0 should write the top of the range"
        );
        assert!(
            (measured - invented).abs() > 1.0,
            "the measured frame changed nothing: {invented} then {measured}"
        );
    }
}
