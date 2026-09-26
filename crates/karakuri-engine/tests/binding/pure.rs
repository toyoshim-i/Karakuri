use super::common::*;

/// Verifies via AST/source scanning that binding resolution contains no wall-clock access.
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

/// Verifies that measured signals drive parameters at full confidence while synthetic signals scale with low confidence.
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

/// Audio signals in `audio.` namespace and semantic band names (`sub`, `bass`, `mid`, `air`, `onset`, etc.)
/// resolve and drive parameters through the binding path.
#[test]
fn audio_signals_and_semantic_bands_drive_parameters() {
    let mut signals = Signals::new(BPM, u64::from(SEED));
    signals.advance(7, 1.0 / 60.0);

    let frame = AudioFrame {
        energy: 0.8,
        onset: 0.95,
        bands: [0.9, 0.7, 0.5, 0.4, 0.3, 0.25, 0.2, 0.15],
        band_count: 8,
        confidence: 1.0,
    };
    signals.set_audio(Some(frame));

    let cases = [
        ("audio.energy", 0.8),
        ("energy", 0.8),
        ("audio.onset", 0.95),
        ("onset", 0.95),
        ("audio.sub", 0.9),
        ("sub", 0.9),
        ("audio.bass", 0.7),
        ("bass", 0.7),
        ("audio.mid", 0.4),
        ("mid", 0.4),
        ("audio.air", 0.15),
        ("air", 0.15),
        ("audio.band0", 0.9),
        ("band0", 0.9),
        ("audio.band7", 0.15),
        ("band7", 0.15),
    ];

    let manual = 0.0;
    let range = [0.0, 1.0];
    for (signal_name, expected_val) in cases {
        let mut binding = Binding::new(Kind::L1, "turbulence", signal_name, Curve::Lin, range);
        let resolved = binding.resolve(&signals, manual);
        assert!(
            (resolved - expected_val).abs() < 1e-5,
            "signal `{signal_name}` resolved to {resolved}, expected {expected_val}"
        );
    }
}
