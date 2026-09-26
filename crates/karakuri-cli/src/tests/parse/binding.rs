use super::*;

// -- --bind ---------------------------------------------------------------

/// Verifies that `--bind` accurately populates all fields of a binding record.
#[test]
fn bind_carries_every_field_of_the_record() {
    let args = parse(&[
        "--bind",
        "layer=L1,key=spawn_rate,signal=noise,curve=lin,range=4000..16000,\
         noise.kind=perlin,noise.rate=0.5,noise.stream=3",
    ])
    .expect("should parse");
    assert_eq!(args.bindings.len(), 1);
    let b = &args.bindings[0];
    assert_eq!(b.layer, karakuri_ir::Kind::L1);
    assert_eq!(b.key, "spawn_rate");
    assert_eq!(b.signal, "noise");
    assert_eq!(b.curve, Curve::Lin);
    assert_eq!(b.range, [4000.0, 16000.0]);
    assert_eq!(
        b.noise,
        Some(NoiseConfig {
            kind: NoiseKind::Perlin,
            rate: 0.5,
            stream: 3,
        })
    );
}

#[test]
fn bind_defaults_the_curve_and_the_noise_generator_but_nothing_else() {
    let plain =
        parse(&["--bind", "layer=L4,key=hue,signal=beat,range=0..1"]).expect("should parse");
    assert_eq!(plain.bindings[0].curve, Curve::Lin);
    assert_eq!(
        plain.bindings[0].noise, None,
        "only a noise signal gets one"
    );

    let noise = parse(&["--bind", "layer=L1,key=spawn_rate,signal=noise,range=0..1"])
        .expect("should parse");
    assert_eq!(
        noise.bindings[0].noise,
        Some(NoiseConfig::default()),
        "a noise signal with no generator named is the default generator, not none"
    );
}

#[test]
fn bind_octaves_reaches_fbm_in_either_order() {
    for fields in [
        "layer=L1,key=radius,signal=noise,range=0..1,noise.kind=fbm,noise.octaves=6",
        "layer=L1,key=radius,signal=noise,range=0..1,noise.octaves=6,noise.kind=fbm",
    ] {
        let args = parse(&["--bind", fields]).expect("should parse");
        assert_eq!(
            args.bindings[0].noise.expect("a generator").kind,
            NoiseKind::Fbm { octaves: 6 },
            "{fields}"
        );
    }
    // `fbm` with nothing said about octaves is the record's default rather
    // than a rejection — the spec's own example omits it.
    let args = parse(&[
        "--bind",
        "layer=L1,key=radius,signal=noise,range=0..1,noise.kind=fbm",
    ])
    .expect("should parse");
    assert_eq!(
        args.bindings[0].noise.expect("a generator").kind,
        NoiseKind::Fbm {
            octaves: setfile::DEFAULT_OCTAVES
        }
    );
}

/// Verifies that octaves fields cannot change the noise generator kind.
#[test]
fn bind_octaves_never_silently_replaces_the_kind_that_was_named() {
    for fields in [
        "layer=L1,key=r,signal=noise,range=0..1,noise.kind=white,noise.octaves=6",
        "layer=L1,key=r,signal=noise,range=0..1,noise.octaves=6,noise.kind=white",
        "layer=L1,key=r,signal=noise,range=0..1,noise.kind=perlin,noise.octaves=6",
        // No kind at all: `perlin` is the default, and an octave count is
        // as meaningless against it as against an explicit one.
        "layer=L1,key=r,signal=noise,range=0..1,noise.octaves=6",
    ] {
        let err = match parse(&["--bind", fields]) {
            Ok(args) => panic!(
                "`{fields}` was accepted as {:?}",
                args.bindings[0].noise.expect("a generator").kind
            ),
            Err(e) => e,
        };
        assert!(err.contains("noise.octaves"), "{fields} -> {err}");
    }
}

/// Every way of getting it wrong says which part was wrong. A `--bind` that
/// quietly took a default would be a parameter that does not move and no way to
/// find out why — the silence every other flag here was fixed for.
#[test]
fn a_malformed_bind_is_refused_and_says_what_it_could_not_use() {
    for (fields, expect) in [
        ("key=hue,signal=beat,range=0..1", "no `layer=`"),
        ("layer=L4,signal=beat,range=0..1", "no `key=`"),
        ("layer=L4,key=hue,range=0..1", "no `signal=`"),
        ("layer=L4,key=hue,signal=beat", "no `range="),
        (
            "layer=L4,key=hue,signal=beat,curve=expo,range=0..1",
            "expected lin, pow2, sqrt, smooth",
        ),
        ("layer=L4,key=hue,signal=beat,range=0-1", "LOW..HIGH"),
        ("layer=L4,key=hue,signal=beat,range=low..high", "LOW..HIGH"),
        (
            "layer=L4,key=hue,signal=beat,curv=lin,range=0..1",
            "unknown field `curv`",
        ),
        (
            "layer=L4,key=hue,signal=beat,range=0..1,noise.rate=2",
            "needs `signal=noise`",
        ),
        (
            "layer=L1,key=r,signal=noise,range=0..1,noise.rate=fast",
            "cycles per beat",
        ),
        ("layer=L4,key=hue,beat", "is not `field=value`"),
    ] {
        let err = match parse(&["--bind", fields]) {
            Ok(_) => panic!("`{fields}` was accepted"),
            Err(e) => e,
        };
        assert!(err.contains(expect), "{fields} -> {err}");
    }
}

/// Verifies that binding directly to bpm is rejected with guidance to use beat or bar.
#[test]
fn binding_bpm_is_refused_and_names_the_signal_to_use_instead() {
    let err = parse(&["--bind", "layer=L1,key=radius,signal=bpm,range=1..5"]).unwrap_err();
    assert!(err.contains("bpm"), "{err}");
    assert!(
        err.contains("beat"),
        "the refusal does not say what to use: {err}"
    );

    // The two that do carry the tempo in the range a binding needs are
    // still accepted, or the refusal above would just be a ban on tempo.
    for signal in ["beat", "bar"] {
        parse(&[
            "--bind",
            &format!("layer=L1,key=radius,signal={signal},range=1..5"),
        ])
        .unwrap_or_else(|e| panic!("`{signal}` was refused: {e}"));
    }
}

// -- --bpm ----------------------------------------------------------------

#[test]
fn bpm_defaults_and_refuses_a_tempo_it_cannot_use() {
    assert_eq!(parse(&[]).expect("should parse").bpm, DEFAULT_BPM);
    assert_eq!(parse(&["--bpm", "128"]).expect("should parse").bpm, 128.0);
    for bad in ["0", "-4", "fast"] {
        let err = parse(&["--bpm", bad]).unwrap_err();
        assert!(err.contains("--bpm"), "{bad} -> {err}");
    }
}

// -- unknown options ------------------------------------------------------

#[test]
fn an_unknown_dash_option_fails_rather_than_becoming_a_path() {
    let err = parse(&["--wtach"]).unwrap_err();
    assert!(err.contains("unknown option"), "message: {err}");
}

// -- seeds --------------------------------------------------------------

#[test]
fn slot_zero_keeps_the_original_seed() {
    assert_eq!(seed_for(0), SEED);
}

/// Verifies that recorded salts take precedence and unrecorded salts are derived from the seed.
#[test]
fn a_recorded_salt_wins_and_an_unrecorded_one_is_derived() {
    let seed = seed_for(0);
    let derived = |at| karakuri_engine::set::derived_salt(seed, at);

    // A bare `--set a.kir,b.kir`: nothing recorded anything, so both are
    // the ordinals — and source 0's is the Set's seed unchanged, which is
    // what keeps a one-geometry run the run it always was.
    assert_eq!(salts_for(seed, &[], 2), vec![derived(0), derived(1)]);
    assert_eq!(salts_for(seed, &[], 1), vec![seed]);

    // A Set file that recorded both. Neither is an ordinal, and neither
    // moves when the geometries change places — which is the whole point.
    assert_eq!(salts_for(seed, &[Some(11), Some(22)], 2), vec![11, 22]);

    // And one that recorded fewer salts than the Set has geometries: an
    // older file, where a `seed` salted the Set rather than a source. What
    // it named keeps its colours and the rest are derived, which is what
    // one number could ever have meant.
    assert_eq!(salts_for(seed, &[Some(11)], 2), vec![11, derived(1)]);
}

/// Verifies that `--merge` flags and Set file layering agree without conflict.
#[test]
fn the_flag_and_the_file_agree_about_compositing_in_either_order() {
    use karakuri_engine::set::Layering::{Composite, Overdraw};
    let composited = FromSet {
        layering: Composite,
        ..FromSet::default()
    };
    let flagged = parse(&["a.kir", "b.kir", "--merge", "0"]).expect("args");
    let bare = parse(&["a.kir", "b.kir"]).expect("args");
    // Parsed again rather than cloned: `Args` is not `Clone`, and it is the
    // same two spellings either way.
    let loaded_args = || parse(&["a.kir", "b.kir"]).expect("args");
    let flagged_args = || parse(&["a.kir", "b.kir", "--merge", "0"]).expect("args");

    // The flag alone, which is what every composited slot was before a Set
    // file could say so.
    assert_eq!(
        layering_for(&flagged, 0, recorded_layering(&flagged, 0)),
        Composite
    );
    // The file alone: `--load-set` of a Set that was saved compositing.
    let mut loaded = loaded_args();
    loaded.from_set = Some(composited.clone());
    assert_eq!(
        layering_for(&loaded, 0, recorded_layering(&loaded, 0)),
        Composite,
        "a composited Set file loaded back overdrawing because no --merge was typed"
    );
    // Both, in one run.
    let mut both = flagged_args();
    both.from_set = Some(composited);
    assert_eq!(
        layering_for(&both, 0, recorded_layering(&both, 0)),
        Composite
    );
    // Neither.
    assert_eq!(
        layering_for(&bare, 0, recorded_layering(&bare, 0)),
        Overdraw
    );
    // **And the file is slot 0's.** `--load-set` fills that slot and every
    // other comes from `--set`, so a composited file says nothing about
    // slot 1 — where only the flag can.
    assert_eq!(
        layering_for(&loaded, 1, recorded_layering(&loaded, 1)),
        Overdraw
    );
}

/// Verifies that live saving accurately records active selection state from the Set.
#[test]
fn a_saved_fold_is_the_selection_the_set_is_holding() {
    let live = karakuri_engine::mix::Input::unity();
    let dark = karakuri_engine::mix::Input {
        live: false,
        ..karakuri_engine::mix::Input::unity()
    };
    // Nobody has selected: every input live, so there is nothing to write.
    assert_eq!(selected_renderer(&[live, live, live]), None);
    // One renderer, never selected in — "all of them" and "one of them" at
    // once, and it is the first.
    assert_eq!(selected_renderer(&[live]), None);
    // A selection, which is the one shape `mix::select` leaves.
    assert_eq!(selected_renderer(&[dark, live, dark]), Some(1));
    assert_eq!(selected_renderer(&[live, dark]), Some(0));
    // Shapes nothing can produce: recorded as unselected rather than as a
    // guess at which of them was meant.
    assert_eq!(selected_renderer(&[dark, dark]), None);
    assert_eq!(selected_renderer(&[live, live, dark]), None);
}

#[test]
fn every_slot_gets_a_distinct_seed() {
    let seeds: Vec<u32> = (0..MAX_SLOTS).map(seed_for).collect();
    for i in 0..seeds.len() {
        for j in (i + 1)..seeds.len() {
            assert_ne!(seeds[i], seeds[j], "slots {i} and {j} collide");
        }
    }
}
