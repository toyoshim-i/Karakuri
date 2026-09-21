use super::*;

// -- edges -----------------------------------------------------------

/// `--edge <node>.<slot>=<geometry>` writes the record it stands for, and every
/// part of the spelling names something.
///
/// The `.` is the same dot the procedure reads the slot through and the `=` is
/// `--set`'s, so `--edge morph.far=sphere` and `far.position` in the `deform`
/// are visibly one spelling.
#[test]
fn an_edge_names_a_node_a_slot_and_the_geometry_bound_to_it() {
    let args = parse(&["--edge", "morph.far=sphere_shell"]).expect("parses");
    assert_eq!(
        args.edges,
        vec![karakuri_engine::set::Edge {
            node: "morph".to_string(),
            slot: "far".into(),
            to: "sphere_shell".to_string(),
        }]
    );

    // **The last dot, not the first.** A node name may hold one — nothing
    // refuses `--set my.morph=morph.kir` — and a slot is a `.kir`
    // identifier, which cannot.
    let args = parse(&["--edge", "my.morph.far=sphere"]).expect("parses");
    assert_eq!(args.edges[0].node, "my.morph");
    assert_eq!(args.edges[0].slot, "far".into());
}

/// A malformed edge is refused rather than dropped, on `--param`'s terms: an
/// edge that was silently discarded looks exactly like a slot nobody bound, and
/// the refusal for *that* would name the file rather than the command line that
/// misspelt it.
#[test]
fn a_malformed_edge_says_so() {
    for spelled in [
        // No geometry after the `=`.
        "morph.far",
        // No slot: a sentence about a node, which there is no such thing as.
        "morph=sphere",
        // Every part names something.
        "morph.=sphere",
        ".far=sphere",
        "morph.far=",
    ] {
        let err = parse(&["--edge", spelled])
            .expect_err(&format!("`{spelled}` is not an edge"))
            .to_string();
        assert!(err.contains("--edge"), "`{spelled}` -> {err}");
    }
}

// -- audio -----------------------------------------------------------

#[test]
fn audio_is_off_unless_asked_for_and_takes_a_device_name() {
    assert_eq!(parse(&[]).expect("parses").audio_in, None);
    assert_eq!(
        parse(&["--audio-in", "default"]).expect("parses").audio_in,
        Some("default".to_string())
    );
    assert_eq!(
        parse(&["--audio-in", "Scarlett"]).expect("parses").audio_in,
        Some("Scarlett".to_string())
    );
    // A flag with nothing after it takes the next flag as its value in the
    // shape this CLI was fixed for once already.
    assert!(parse(&["--audio-in", "--watch"]).is_err());
    assert!(parse(&["--audio-in"]).is_err());
}

/// Verifies that saving a Set records actual operating capacities rather than fallback defaults.
#[test]
fn a_saved_set_records_the_capacity_the_run_was_drawing() {
    let small = compile::check(
        "proc small { kind L1 capacity [4096, 262144] = 32768 topology points \
         emit position element { position = vec3(0.0); } }",
    )
    .expect("compiles");
    let large = compile::check(
        "proc large { kind L1 capacity [4096, 1048576] = 131072 topology points \
         emit position element { position = vec3(0.0); } }",
    )
    .expect("compiles");

    let args = parse(&[]).expect("parses");
    assert_eq!(
        saving_capacities(&args, &[small.clone(), large.clone()]),
        vec![32768, 131072],
        "the file has to say what each geometry drew, not what the flag defaults to"
    );

    // And `--capacity` is still the operator overriding every source, so
    // that is what a file saved under it records.
    let forced = parse(&["--capacity", "8192"]).expect("parses");
    assert_eq!(
        saving_capacities(&forced, &[small, large]),
        vec![8192, 8192]
    );
}

/// Each source runs at the capacity its own procedure declares. The build asked
/// `capacity_for` once, about the first L1, and handed the answer to every
/// source — so a grid declared at 131072 ran at 32768 because it was loaded
/// beside a cube that declared that.
///
/// Nothing could have caught it downstream: the number came from a real
/// declaration, so it was inside *somebody's* range, and the picture is a grid
/// with fewer points in it, which is a thing a grid can be.
#[test]
fn every_source_runs_at_the_capacity_it_declares() {
    let small = compile::check(
        "proc small { kind L1 capacity [4096, 262144] = 32768 topology points \
         emit position element { position = vec3(0.0); } }",
    )
    .expect("compiles");
    let large = compile::check(
        "proc large { kind L1 capacity [4096, 1048576] = 131072 topology points \
         emit position element { position = vec3(0.0); } }",
    )
    .expect("compiles");

    let args = parse(&[]).expect("parses");
    assert_eq!(
        capacities_for(&args, &[small.clone(), large.clone()], &[]),
        vec![32768, 131072]
    );
    // Order is not what decides it, which is the half a first-one-wins
    // implementation gets right by accident half the time.
    assert_eq!(
        capacities_for(&args, &[large.clone(), small.clone()], &[]),
        vec![131072, 32768]
    );

    // `--capacity` still overrides all of them: the operator asking for a
    // number is asking about the Set, not about one file in it.
    let forced = parse(&["--capacity", "8192"]).expect("parses");
    assert_eq!(
        capacities_for(&forced, &[small.clone(), large.clone()], &[]),
        vec![8192, 8192]
    );

    // **And a Set file's own number wins over both**, per geometry: the
    // file is where that geometry's count was decided, and a Set that came
    // back at another size is a Set that was not saved. Where the file said
    // nothing, the declaration underneath still answers.
    assert_eq!(
        capacities_for(&args, &[small, large], &[Some(16384), None]),
        vec![16384, 131072]
    );
}

// -- midi ------------------------------------------------------------

#[test]
fn midi_is_off_unless_asked_for_and_takes_a_port_name() {
    assert_eq!(parse(&[]).expect("parses").midi_in, None);
    assert_eq!(parse(&[]).expect("parses").midi_map, None);
    assert_eq!(
        parse(&["--midi-in", "nanoKONTROL"])
            .expect("parses")
            .midi_in,
        Some("nanoKONTROL".to_string())
    );
    // An empty selector is "the first port there is", which is a real
    // answer rather than a missing value — the one plugged-in surface is
    // the common case and should need no name.
    assert_eq!(
        parse(&["--midi-in", ""]).expect("parses").midi_in,
        Some(String::new())
    );
    assert!(parse(&["--midi-in", "--watch"]).is_err());
    assert!(parse(&["--midi-in"]).is_err());
    // With a port, so what refuses this is `--midi-map` swallowing the next
    // flag rather than the map-with-no-port rule getting there first. That
    // is the difference between this line and an assertion that cannot
    // fail.
    assert!(parse(&["--midi-in", "", "--midi-map", "--watch"]).is_err());
    assert!(parse(&["--midi-in", "", "--midi-map"]).is_err());
}

/// A map with no port is a file nothing reads, and the likely cause is a
/// forgotten `--midi-in`. Refused where it can be said rather than loaded,
/// checked and silently unused.
#[test]
fn a_midi_map_with_no_port_is_refused() {
    let message = parse(&["--midi-map", "surface.map"]).expect_err("a map with nothing to map");
    assert!(message.contains("--midi-in"), "{message}");
    assert!(parse(&["--midi-map", "surface.map", "--midi-in", ""]).is_ok());
}

/// An offscreen run has nobody at the surface, on exactly the terms it has no
/// microphone.
#[test]
fn midi_and_an_offscreen_render_are_refused_together() {
    let message = parse(&["--midi-in", "", "--render", "out.png"]).expect_err("should be refused");
    assert!(message.contains("--midi-in"), "{message}");
    assert!(parse(&["--midi-in", "", "--seq", "frames/"]).is_err());
    assert!(parse(&["--midi-in", ""]).is_ok());
}

/// An offscreen run is a function of its arguments, so a live input is refused
/// rather than accepted and ignored.
#[test]
fn audio_and_an_offscreen_render_are_refused_together() {
    let message =
        parse(&["--audio-in", "default", "--render", "out.png"]).expect_err("should be refused");
    assert!(message.contains("--audio-in"), "{message}");
    assert!(parse(&["--audio-in", "default", "--seq", "frames/"]).is_err());
    // ...and either alone is fine.
    assert!(parse(&["--audio-in", "default"]).is_ok());
    assert!(parse(&["--render", "out.png"]).is_ok());
}

#[test]
fn the_latency_offset_defaults_takes_a_sign_and_is_validated() {
    assert_eq!(
        parse(&[]).expect("parses").latency_offset_ms,
        audio::DEFAULT_LATENCY_OFFSET_MS
    );
    assert_eq!(
        parse(&["--latency-offset-ms", "35"])
            .expect("parses")
            .latency_offset_ms,
        35.0
    );
    // **Negative is a value, not a typo.** A room whose sound arrives after
    // its picture is corrected for by turning this below zero, and there is
    // no other control that can: nothing at this end sees the PA or the
    // projector.
    assert_eq!(
        parse(&["--latency-offset-ms", "-45"])
            .expect("a negative offset is a room, not a mistake")
            .latency_offset_ms,
        -45.0
    );
    // Refused rather than clamped or defaulted: an offset silently changed
    // is an offset the operator spends the first song chasing.
    for bad in ["1000", "-1000", "twenty", ""] {
        assert!(
            parse(&["--latency-offset-ms", bad]).is_err(),
            "`--latency-offset-ms {bad}` was accepted"
        );
    }
}
