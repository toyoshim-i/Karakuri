use super::*;
use karakuri_environment::compile::sort_compiled;
use karakuri_environment::mix::{op_wire_name, TONEMAPS};
use karakuri_signal::NoiseKind;

fn parse(args: &[&str]) -> Result<Args, String> {
    match parse_args_from(args.iter().map(|s| s.to_string())) {
        Ok(ParseOutcome::Run(args)) => Ok(*args),
        Ok(ParseOutcome::Help) => panic!("expected Args, got --help"),
        Ok(ParseOutcome::ListSets(_)) => panic!("expected Args, got --list-sets"),
        Ok(ParseOutcome::Package { .. }) => panic!("expected Args, got --package"),
        Ok(ParseOutcome::TakeIn { .. }) => panic!("expected Args, got --take-in"),
        Err(e) => Err(e),
    }
}

// -- --list-sets -----------------------------------------------------

/// `--list-sets` is not a run, and the type says so.
///
/// The flag prints what the store holds and stops: no window, no adapter, no
/// compile, no Set built. That is enforced by there being no [`Args`] at all on
/// this path — [`parse_args_from`] answers with [`ParseOutcome::ListSets`],
/// which carries a store path and nothing else, so everything `main` does with
/// an `Args` is unreachable rather than merely skipped. A `bool` on `Args`
/// would have needed a check above every early return in `main` and would have
/// been wrong the day somebody added one more.
///
/// `--store` on either side of it, because an operator types the flags in
/// whatever order they think of them, and a listing of the default store when
/// `--store` was given would be a listing of the wrong library.
#[test]
fn list_sets_prints_and_is_never_a_run() {
    for spelling in [
        vec!["--list-sets", "--store", "/tmp/library"],
        vec!["--store", "/tmp/library", "--list-sets"],
    ] {
        match parse_args_from(spelling.iter().map(|s| s.to_string())) {
            Ok(ParseOutcome::ListSets(root)) => {
                assert_eq!(
                    root,
                    PathBuf::from("/tmp/library"),
                    "{spelling:?} listed a store nobody asked for"
                );
            }
            Ok(ParseOutcome::Run(_)) => panic!(
                "{spelling:?} came back as a run: a listing would then compile the \
                 default material and open a window to print a directory"
            ),
            Ok(ParseOutcome::Help) => panic!("{spelling:?} came back as --help"),
            Ok(ParseOutcome::Package { .. }) | Ok(ParseOutcome::TakeIn { .. }) => {
                panic!("{spelling:?} came back as a transfer")
            }
            Err(e) => panic!("{spelling:?} was refused: {e}"),
        }
    }
}

/// Neither `--package` nor `--take-in` is a run, and the type says so for
/// [`ParseOutcome::ListSets`]'s reason: there is no [`Args`] on either path, so
/// the compile, the deck, the window and the adapter request are unreachable
/// rather than merely skipped. `--take-in` does reach the checker — that is how
/// a metadata card gets written — and the check pass needs no device.
///
/// `--store` on either side of each, because an operator types the flags in
/// whatever order they think of them, and a Set packaged out of the default
/// store when `--store` was given would be a package of the wrong library.
#[test]
fn package_and_take_in_print_and_are_never_a_run() {
    for spelling in [
        vec!["--package", "night01", "--store", "/tmp/library"],
        vec!["--store", "/tmp/library", "--package", "night01"],
    ] {
        match parse_args_from(spelling.iter().map(|s| s.to_string())) {
            Ok(ParseOutcome::Package { store, id }) => {
                assert_eq!(store, PathBuf::from("/tmp/library"), "{spelling:?}");
                assert_eq!(id, "night01", "{spelling:?}");
            }
            Ok(ParseOutcome::Run(_)) => panic!(
                "{spelling:?} came back as a run: writing a package to stdout would then \
                 compile material and open a window to do it"
            ),
            other => panic!(
                "{spelling:?} came back as something else: {}",
                named(&other)
            ),
        }
    }
    for spelling in [
        vec!["--take-in", "sent.kbset", "--store", "/tmp/library"],
        vec!["--store", "/tmp/library", "--take-in", "sent.kbset"],
    ] {
        match parse_args_from(spelling.iter().map(|s| s.to_string())) {
            Ok(ParseOutcome::TakeIn { store, file }) => {
                assert_eq!(store, PathBuf::from("/tmp/library"), "{spelling:?}");
                assert_eq!(file, PathBuf::from("sent.kbset"), "{spelling:?}");
            }
            Ok(ParseOutcome::Run(_)) => panic!(
                "{spelling:?} came back as a run: taking a file into the store would \
                 then build a Set and open a window"
            ),
            other => panic!(
                "{spelling:?} came back as something else: {}",
                named(&other)
            ),
        }
    }
}

/// Which outcome, for a panic message. `ParseOutcome` holds GPU-adjacent config
/// and is deliberately not `Debug`; see `value_tests::parse`.
fn named(outcome: &Result<ParseOutcome, String>) -> String {
    match outcome {
        Ok(ParseOutcome::Run(_)) => "a run".to_string(),
        Ok(ParseOutcome::Help) => "--help".to_string(),
        Ok(ParseOutcome::ListSets(_)) => "--list-sets".to_string(),
        Ok(ParseOutcome::Package { .. }) => "--package".to_string(),
        Ok(ParseOutcome::TakeIn { .. }) => "--take-in".to_string(),
        Err(e) => format!("a refusal: {e}"),
    }
}

/// A store with two Sets in it, written at times this test decides.
///
/// A Set file carries no time — the mtime is the only record of when one was
/// saved — so a fixture that means to test an order has to say what the times
/// are rather than hope two writes land in different seconds.
fn library(root: &std::path::Path) -> karakuri_store::store::Store {
    let store = karakuri_store::store::Store::open(root).expect("store");
    let hash = store.put_artifact(b"not compiled here").expect("put");
    let slot = |layer: Layer, index: u32, name: &str| {
        karakuri_store::ndjson::Line::new(Record::Slot {
            at: karakuri_store::record::NodeAddress { layer, index },
            name: Some(name.to_string()),
            proc_hash: hash,
        })
    };
    store
        .write_set(
            "older",
            &[slot(Layer::L1, 0, "shell"), slot(Layer::L4, 0, "dots")],
        )
        .expect("set");
    store
        .write_set(
            "newer",
            &[
                slot(Layer::L1, 0, "shell"),
                slot(Layer::L2, 0, "bend"),
                slot(Layer::L4, 0, "dots"),
                slot(Layer::L4, 1, "strokes"),
            ],
        )
        .expect("set");
    for (id, secs) in [("older", 1_000u64), ("newer", 2_000)] {
        let file = std::fs::OpenOptions::new()
            .write(true)
            .open(root.join("sets").join(format!("{id}.kbset")))
            .expect("open the set file");
        file.set_times(
            std::fs::FileTimes::new()
                .set_modified(std::time::UNIX_EPOCH + Duration::from_secs(secs)),
        )
        .expect("set the mtime");
    }
    store
}

/// One line per Set: the id, when it was written, and what it holds.
///
/// The id is what goes next to `--load-set`, the time is what an operator looks
/// for a keeper by, and the layers are enough to tell two Sets apart without
/// opening either. Most recent first, for the reason the MCP listing is: *what
/// did I just save* is the question.
///
/// A layer nothing is on is left out rather than printed as a zero — most Sets
/// are on three of the five, and a line of zeroes reads as something missing.
#[test]
fn the_listing_is_a_line_per_set_most_recent_first() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().join("store");
    let store = library(&root);
    let said = listed_sets(&store, &root).expect("the listing");

    let lines: Vec<&str> = said.lines().collect();
    assert_eq!(lines.len(), 2, "one line per set, and no more: {said}");
    assert!(
        lines[0].starts_with("newer") && lines[1].starts_with("older"),
        "the sets are not most recent first: {said}"
    );
    assert!(
        lines[0].contains("1 L1, 1 L2, 2 L4") && !lines[0].contains("L3"),
        "the line does not say what the set holds by layer, or counts a layer \
         nothing is on: {said}"
    );
    assert!(
        lines[1].contains("1 L1, 1 L4"),
        "the line does not say what the set holds by layer: {said}"
    );
    // The times are the ones the fixture set, in the operator's own clock,
    // and they are what tells two takes of one evening apart.
    for (line, secs) in lines.iter().zip([2_000u64, 1_000]) {
        let written = setfile::written_at(std::time::UNIX_EPOCH + Duration::from_secs(secs));
        assert!(
            line.contains(&written),
            "the line does not say when the set was written: {line}"
        );
    }
}

/// An empty store is an ordinary answer and says where sets come from — not a
/// blank terminal, which reads as a flag that did nothing.
#[test]
fn an_empty_store_says_so_rather_than_printing_nothing() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().join("store");
    let store = karakuri_store::store::Store::open(&root).expect("store");
    let said = listed_sets(&store, &root).expect("the listing");
    assert!(
        said.contains("no sets in") && said.contains("--save-set"),
        "an empty store printed something an operator cannot act on: {said:?}"
    );

    // **And a path with no store at it is a different answer, and creates
    // nothing.** `Store::open` would establish the layout under it, so a
    // listing of a mistyped `--store` would answer "empty" and leave a
    // directory behind saying so — a read-only flag that writes.
    let missing = dir.path().join("nowhere");
    let said = listed_sets_at(&missing).expect("the listing");
    assert!(
        said.contains("no store at") && said.contains("--store"),
        "a path with no store at it was not told apart from an empty one: {said:?}"
    );
    assert!(
        !missing.exists(),
        "a listing created the store it was asked to read"
    );
}

/// `--package` takes a `.kset` in and packages it, which is the other moment of
/// the one operation this flag already was — and the moment the flag is now
/// named for.
///
/// What is being checked here is the *route* and not the resolution — that is
/// `karakuri-environment`'s, tested there — so this asserts the two things only
/// this file decides: that a value ending in `.kset` is read as a path to an
/// authoring file rather than looked up as an id, and that what comes back is a
/// bundle, sources and all, on the standard output a shell can redirect.
///
/// And that the store it writes into is established, unlike the id half:
/// resolving is a write, so a `--store` an operator named has to exist by the
/// time the first artifact lands.
#[test]
fn package_takes_an_authoring_file_in_and_packages_it() {
    let dir = tempfile::tempdir().expect("tempdir");
    let l1 = "proc ring { kind L1 }\n";
    let l4 = "proc points { kind L4 }\n";
    std::fs::write(dir.path().join("l1.kir"), l1).expect("write");
    std::fs::write(dir.path().join("l4.kir"), l4).expect("write");
    let kset = dir.path().join("night.kset");
    std::fs::write(
        &kset,
        "{\"t\":\"set\",\"id\":\"night\",\"v\":1}\n\
         {\"t\":\"part\",\"layer\":\"L1\",\"path\":\"l1.kir\"}\n\
         {\"t\":\"part\",\"layer\":\"L4\",\"path\":\"l4.kir\"}\n",
    )
    .expect("write");

    let root = dir.path().join("store");
    let said = packaged_set(&root, kset.to_str().expect("utf-8")).expect("the packaging");

    assert!(
        !said.contains("\"t\":\"part\""),
        "a bundle carries no part: {said}"
    );
    assert_eq!(
        said.matches("\"t\":\"slot\"").count(),
        2,
        "each part became a slot naming an address: {said}"
    );
    for source in [l1, l4] {
        let hash = karakuri_store::hash::Hash::of(source.as_bytes());
        assert!(
            said.contains(&hash.to_string()),
            "the slot names the address of the bytes on disk: {said}"
        );
        assert!(
            karakuri_store::store::Store::open(&root)
                .expect("store")
                .get_artifact(&hash)
                .is_ok(),
            "the store this wrote into holds the part: {said}"
        );
    }
    assert!(
        said.contains("\"t\":\"src\""),
        "the sources are inlined, which is what makes it a bundle: {said}"
    );

    // **And an id is still an id.** The extension is the whole of what
    // tells the two apart, so a value without one is looked up in the store
    // and says so when it is not there.
    let refused = packaged_set(&root, "night").expect_err("no set is filed under that id");
    assert!(refused.contains("night"), "{refused}");
}

/// `--take-in` takes both of a Set's forms, and the extension is what says
/// which — the same sentence [`packaged_set`] reads, from the other end.
///
/// A `.kbset` is already resolved and is taken in as it stands; that is what
/// `karakuri-environment`'s own tests cover. What only this file decides is the
/// `.kset` half: a value ending in `.kset` is resolved against its own
/// directory first and then taken in, so an operator who was sent an authoring
/// file beside its parts does not have to package it to themselves before they
/// can keep it.
///
/// And it lands the same store as packaging it would, which is why the route is
/// `bundle_authored` and not `resolve` alone: the artifacts are there, the Set
/// is filed under the id the file carries, and each source has the metadata
/// card a take-in writes.
#[test]
fn take_in_resolves_an_authoring_file_and_then_takes_it_in() {
    let dir = tempfile::tempdir().expect("tempdir");
    // Sources that actually compile, because a take-in runs the checker to
    // write a metadata card and a source it will not compile is stored
    // without one.
    let l1 = r#"
proc ring {
  kind     L1
  topology points
  capacity [1024, 262144] = 4096

  emit position

  element {
position = vec3(cos(t), sin(t), 0.0);
  }
}
"#;
    let l4 = r#"
proc points {
  kind  L4
  blend additive

  consumes position

  vertex {
clip       = camera * vec4(position, 1.0);
point_rate = 0.016;
  }

  fragment {
color = vec4(1.0, 1.0, 1.0, 1.0);
  }
}
"#;
    std::fs::write(dir.path().join("l1.kir"), l1).expect("write");
    std::fs::write(dir.path().join("l4.kir"), l4).expect("write");
    let kset = dir.path().join("night.kset");
    std::fs::write(
        &kset,
        "{\"t\":\"set\",\"id\":\"night\",\"v\":1}\n\
         {\"t\":\"part\",\"layer\":\"L1\",\"path\":\"l1.kir\"}\n\
         {\"t\":\"part\",\"layer\":\"L4\",\"path\":\"l4.kir\"}\n",
    )
    .expect("write");

    let root = dir.path().join("store");
    let said = taken_in_file(&root, &kset).expect("the take-in");
    assert!(
        said.contains("`night`") && said.contains("2 sources stored"),
        "the report does not say what was taken in: {said}"
    );
    assert!(
        said.contains("2 metadata cards written"),
        "a take-in writes a card per source, whichever form it came in: {said}"
    );

    let store = karakuri_store::store::Store::open(&root).expect("store");
    assert!(
        store
            .list_sets()
            .expect("sets")
            .iter()
            .any(|e| e.id == "night"),
        "the set is not filed under the id the file carries"
    );
    for source in [l1, l4] {
        let hash = karakuri_store::hash::Hash::of(source.as_bytes());
        assert!(
            store.get_artifact(&hash).is_ok(),
            "the store does not hold the part the authoring file named"
        );
    }

    // **And the id in the file is still somebody else's word.** Taking the
    // same authoring file in twice is the refusal a `.kbset` gets, for the
    // same reason: nothing here was typed by the operator.
    let refused = taken_in_file(&root, &kset).expect_err("the id is taken");
    assert!(
        refused.contains("already in this store"),
        "a second take-in overwrote a Set: {refused}"
    );
}

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

/// A saved Set records what the run was drawing, which is the one promise the
/// format makes and the one it was breaking.
///
/// `--save-set` wrote `args.capacity` — the flag's number, or its default where
/// no flag was given — while the run asks each procedure for the default *it*
/// declares. A lattice written for 32768 elements was saved as 262144 and
/// loaded back a larger, smeared version of itself.
///
/// It was invisible while `--load-set` also ignored what the file said: the
/// number was wrong on the way out and wrong again on the way in, and the two
/// cancelled. Teaching the loader to honour a recorded capacity is what made
/// them disagree out loud, which is the ordinary way a pair of compensating
/// errors is found.
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

// -- defaults and the positional pair --------------------------------

/// A session recorded with no flags carries its material and replays.
///
/// The head used to be the `--load-set` file or nothing, and "or nothing" meant
/// a timeline of ticks with no material under it: `--replay` refused it with
/// "has no L1 slot" long after the set was over, and nothing said so at the
/// time. This is the round trip that catches it — the head is written the way
/// the recorder writes it and read back the way the replay reads it, so a head
/// that describes nothing fails here rather than on stage.
#[test]
fn a_session_head_carries_the_material_a_replay_needs() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let l1 = root.join("examples/drift_shell.kir");
    let l4 = root.join("examples/soft_points.kir");
    let dir = tempfile::tempdir().expect("tempdir");
    let store = karakuri_store::store::Store::open(dir.path()).expect("store");

    let mut args = parse(&[]).expect("parses");
    args.sets = vec![(Named::bare(&l1), vec![Named::bare(&l4)])];
    // **And the edge that makes it buildable.** `morph` takes a geometry it
    // calls `far` and a Set that does not say which one is refused where it
    // is built — so a head recorded without it is a head no replay can
    // open, which is exactly the failure this test is about.
    args.edges = vec![karakuri_engine::set::Edge {
        node: "morph".to_string(),
        slot: "far".into(),
        to: "sphere_shell".to_string(),
    }];
    args.store = dir.path().to_path_buf();
    let (material, placed) = sort_slot(0, &args.sets[0].0, &args.sets[0].1);

    let head = session_head(&args, &[placed], &material.l1s, &store, "a_set");
    assert!(!head.is_empty(), "the head describes nothing");

    // Read back the way `--replay` reads it, which is the whole claim: the
    // artifacts resolve out of the store and both slots are there.
    let loaded = setfile::from_lines(&store, "a_set", &head)
        .expect("the head a recording writes is a head a replay can load");
    assert_eq!(loaded.l1s[0].kind, karakuri_ir::Kind::L1);
    assert_eq!(loaded.l4s[0].kind, karakuri_ir::Kind::L4);
    assert!(loaded.notes.is_empty(), "{:?}", loaded.notes);
}

/// A session opens with a Set file, so what a Set file cannot hold is a
/// performance that cannot be recorded.
///
/// A cube morphing into a sphere is two geometries, an L2 that pairs them and a
/// renderer — a chain `--set` has spelled for a while and the head could not
/// carry: `session_head` wrote every path after the first as an `L4` slot, so
/// `--record-session` refused it outright rather than recording a performance
/// nobody could replay. This is that chain through the head and back, layer by
/// layer.
#[test]
fn a_session_head_carries_a_whole_chain_and_not_just_a_pair() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let dir = tempfile::tempdir().expect("tempdir");
    let store = karakuri_store::store::Store::open(dir.path()).expect("store");

    let mut args = parse(&[]).expect("parses");
    args.sets = vec![(
        Named::bare(root.join("examples/lattice_shell.kir")),
        vec![
            Named::bare(root.join("examples/sphere_shell.kir")),
            Named::bare(root.join("examples/morph.kir")),
            Named::bare(root.join("examples/soft_points.kir")),
        ],
    )];
    args.store = dir.path().to_path_buf();
    let (material, placed) = sort_slot(0, &args.sets[0].0, &args.sets[0].1);

    let head = session_head(&args, &[placed], &material.l1s, &store, "a_chain");
    let loaded = setfile::from_lines(&store, "a_chain", &head)
        .expect("the head a recording writes is a head a replay can load");
    assert_eq!(
        loaded
            .l1s
            .iter()
            .map(|c| c.name.as_str())
            .collect::<Vec<_>>(),
        ["lattice_shell", "sphere_shell"],
        "the geometries, in the order they were named"
    );
    assert_eq!(
        loaded
            .l2s
            .iter()
            .map(|c| c.name.as_str())
            .collect::<Vec<_>>(),
        ["morph"],
        "the deformer that pairs them"
    );
    assert_eq!(
        loaded
            .l4s
            .iter()
            .map(|c| c.name.as_str())
            .collect::<Vec<_>>(),
        ["soft_points"]
    );
    assert_eq!(loaded.edges, args.edges, "and the wiring between them");
    assert!(loaded.notes.is_empty(), "{:?}", loaded.notes);
}

/// One slot's examples, compiled, as [`sort_compiled`] takes them — with the
/// text each was compiled from, which is what a node is addressed by.
fn compiled(files: &[&str]) -> Vec<(Named, karakuri_ir::typed::Checked, std::sync::Arc<str>)> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    files
        .iter()
        .map(|file| {
            let path = root.join(file);
            let (checked, src) = compile::load(&path).expect("the examples compile");
            (
                Named::bare(path),
                checked,
                std::sync::Arc::from(src.as_str()),
            )
        })
        .collect()
}

/// A slot that cannot be assembled refuses with a sentence, and the two callers
/// decide what to do about it — a startup prints it and stops, a rebuild prints
/// it and leaves the Set that is running alone. That difference is the only
/// thing the two paths do differently, and it is the reason this sort was
/// written twice before it was written once.
///
/// The sentence names the file, because the file is what an operator can fix;
/// the slot is prefixed by whichever caller is reporting it.
#[test]
fn a_slot_refuses_nothing_that_draws_and_keeps_a_second_camera_and_field() {
    // Matched rather than `expect_err`, which would want `Material` to be
    // `Debug` — a derive on a production type to print something no test
    // reaching here ever prints.
    let refused = |files: &[&str]| match sort_compiled(compiled(files)) {
        Err(e) => e,
        Ok(_) => panic!("this slot cannot be assembled"),
    };
    // **A second camera is not a refusal, and this is where that stopped
    // being one.** It said a slot looks from one viewpoint, which was true
    // of the plumbing and not of the material: a renderer declares `uses
    // view : Camera` and an `edge` names which node fills it, so two
    // cameras are two nodes addressed as `L3:0` and `L3:1`.
    let (two_cameras, placed_cameras) = match sort_compiled(compiled(&[
        "drift_shell.kir",
        "beat_jump.kir",
        "beat_jump.kir",
        "soft_points.kir",
    ])) {
        Ok(sorted) => sorted,
        Err(e) => panic!("two cameras are two nodes: {e}"),
    };
    assert_eq!(two_cameras.l3s.len(), 2, "both were kept");
    assert_eq!(
        placed_cameras
            .iter()
            .filter(|p| p.layer == karakuri_ir::Kind::L3)
            .map(|p| (p.layer, p.index))
            .collect::<Vec<_>>(),
        [(karakuri_ir::Kind::L3, 0), (karakuri_ir::Kind::L3, 1)],
        "the cameras are numbered from 0 with no gaps"
    );
    // **A second field is not a refusal, and this is where that stopped
    // being one.** The sort was the last thing in the tree saying a Set
    // holds one, and it said so about the plumbing rather than about the
    // material: two fields are two nodes, addressed as `Field:0` and
    // `Field:1` and bound by name.
    let two_fields = sort_compiled(compiled(&[
        "drift_shell.kir",
        "melt_blob.kir",
        "melt_blob.kir",
        "soft_points.kir",
    ]));
    let (material, placed) = match two_fields {
        Ok(sorted) => sorted,
        Err(e) => panic!("two fields are two nodes: {e}"),
    };
    assert_eq!(material.fields.len(), 2, "both were kept");
    // **At its own index**, which is what `--param Field:1:x` and a Set
    // file's `slot` record both address it by. The second used to be
    // dropped, and before the refusal above it was dropped silently.
    let addresses: Vec<(karakuri_ir::Kind, u32)> = placed
        .iter()
        .filter(|p| p.layer == karakuri_ir::Kind::Field)
        .map(|p| (p.layer, p.index))
        .collect();
    assert_eq!(
        addresses,
        [(karakuri_ir::Kind::Field, 0), (karakuri_ir::Kind::Field, 1)],
        "the fields are numbered from 0 with no gaps"
    );
    let no_renderer = refused(&["drift_shell.kir", "swirl_warp.kir"]);
    assert!(no_renderer.contains("nothing here draws"), "{no_renderer}");
    // The file the slot was spelled with, which is the one an operator looks
    // at first — and by the time this is said the list has been sorted past.
    assert!(no_renderer.contains("drift_shell.kir"), "{no_renderer}");

    // ...and one of each is a slot, so none of the above is a refusal of
    // cameras and fields as such.
    let one_of_each = sort_compiled(compiled(&[
        "drift_shell.kir",
        "beat_jump.kir",
        "melt_blob.kir",
        "soft_points.kir",
    ]));
    assert!(
        one_of_each.is_ok(),
        "one camera and one field is a slot, not a refusal"
    );
}

/// Any part of a `--set` may carry a name. A name addresses the node from every
/// surface that can reach one, and it is written where the file is spelled
/// because it belongs to the *use* — the same lattice twice is one `proc` name
/// and two nodes.
#[test]
fn a_set_may_name_any_of_its_files() {
    let args = parse(&["--set", "near=a.kir,far=b.kir,c.kir"]).expect("parses");
    assert_eq!(
        args.sets,
        vec![(
            Named {
                name: Some("near".to_string()),
                path: PathBuf::from("a.kir")
            },
            vec![
                Named {
                    name: Some("far".to_string()),
                    path: PathBuf::from("b.kir")
                },
                Named::bare("c.kir"),
            ]
        )]
    );
}

/// Two written names that collide are refused rather than resolved, and this is
/// the *early* refusal — the one that names the slot, before a GPU is asked for
/// anything. `Set::build_many` refuses the same thing again where every node is
/// in hand, which is where a derived name could also collide with a written
/// one.
#[test]
fn two_nodes_cannot_share_a_name() {
    let names = Names {
        l1s: vec![Some("near".to_string())],
        l4s: vec![Some("near".to_string())],
        ..Names::default()
    };
    let e = names.check_unique().expect_err("refused");
    assert!(e.contains("both called `near`"), "{e}");

    // And the scope is the slot, because a Set is what holds the nodes.
    let fine = Names {
        l1s: vec![Some("near".to_string())],
        l4s: vec![Some("draw".to_string())],
        ..Names::default()
    };
    assert!(fine.check_unique().is_ok());
}

/// A layer spelling cannot be a node's name, because `--param` tells the two
/// forms apart by what follows the first colon. A node called `L4` would make
/// `--param L4:0:x` ambiguous with a node named `L4` holding a param called
/// `0`.
#[test]
fn a_node_name_is_refused_where_it_would_be_read_as_something_else() {
    for bad in ["L1", "L4", "Field"] {
        let e = parse(&["--set", &format!("{bad}=a.kir,b.kir")]).expect_err("refused");
        assert!(e.contains("is a layer"), "{e}");
    }
    // A name is an address, so what may be in one is narrow and the
    // refusal names the character rather than quietly rewriting it: a
    // silently renamed node is one a `--param` written against it stops
    // finding.
    let e = parse(&["--set", "a b=a.kir,b.kir"]).expect_err("refused");
    assert!(e.contains("not allowed"), "{e}");
    let e = parse(&["--set", "=a.kir,b.kir"]).expect_err("refused");
    assert!(e.contains("empty"), "{e}");
    let e = parse(&["--set", "near=,b.kir"]).expect_err("refused");
    assert!(e.contains("no file after it"), "{e}");
}

#[test]
fn no_arguments_is_the_default_pair() {
    let args = parse(&[]).expect("should parse");
    assert_eq!(
        args.sets,
        vec![(
            Named::bare("examples/coil_vortex.kir"),
            vec![Named::bare("examples/star_flares.kir")],
        )],
        "the pair `examples/star_vortex.kset` names, which is what a run that said \
         nothing opens on since ADR-0271"
    );
}

/// A demonstration brings the scene it is about. `--demo lines` fades each slot
/// out in turn so the other is seen alone, and on a one-slot deck that shows
/// the picture and then an empty frame — the demonstration would run, look like
/// it worked, and demonstrate nothing.
///
/// Two slots, therefore, even though a stack would fit in one. This was briefly
/// rewritten to a single slot holding both renderers, on the grounds that it is
/// the shape the milestone made possible — which broke it, because the script
/// takes one slot at a time out of the mix and a fader is per slot rather than
/// per renderer. Slot 0 carries the stack, which is what the milestone actually
/// buys here: the same two draws, over one simulation instead of two.
#[test]
fn the_lines_demo_supplies_its_own_two_slot_deck() {
    let args = parse(&["--demo", "lines"]).expect("should parse");
    assert_eq!(
        args.sets,
        vec![
            (
                Named::bare("examples/drift_shell.kir"),
                vec![
                    Named::bare("examples/soft_points.kir"),
                    Named::bare("examples/drift_streaks.kir"),
                ],
            ),
            (
                Named::bare("examples/drift_shell.kir"),
                vec![Named::bare("examples/drift_streaks.kir")],
            ),
        ],
        "there has to be more than one slot for taking one away to show anything"
    );
    // **Three moments, one per slot plus the return to the mix**, and each
    // moment is the keys it takes: a digit to focus the slot and `F` to
    // fade it out, `G` to bring the previous one back. A deck of a
    // different size leaves the script out of phase, which is what this
    // counts.
    let fades = DEMO_LINES_SCRIPT.iter().filter(|(_, k)| *k == 'F').count();
    let restores = DEMO_LINES_SCRIPT.iter().filter(|(_, k)| *k == 'G').count();
    assert_eq!(
        (fades, restores),
        (args.sets.len(), args.sets.len()),
        "the script fades one slot out per slot in the deck and brings each back; \
         a deck of a different size leaves it out of phase"
    );
    let focused: Vec<char> = DEMO_LINES_SCRIPT
        .iter()
        .filter(|(_, k)| k.is_ascii_digit())
        .map(|(_, k)| *k)
        .collect();
    assert_eq!(
        focused,
        vec!['1', '0'],
        "a fade acts on the focused slot, so every `F` needs the digit that says which"
    );
}

/// And supplies it rather than imposing it: material named on the command line
/// still wins, so `--demo lines` over someone else's pair shows their material
/// and not the examples.
#[test]
fn material_on_the_command_line_beats_a_demos_own_deck() {
    let args = parse(&["--demo", "lines", "a.kir", "b.kir"]).expect("should parse");
    assert_eq!(
        args.sets,
        vec![(Named::bare("a.kir"), vec![Named::bare("b.kir")])]
    );
}

/// The transport demonstration works on whatever is loaded, so it brings
/// nothing and the ordinary default still applies. The control for the two
/// above: a `deck()` that answered the same for every demonstration would
/// satisfy them and break this.
#[test]
fn the_transport_demo_leaves_the_default_pair_alone() {
    let args = parse(&["--demo", "transport"]).expect("should parse");
    assert_eq!(
        args.sets,
        vec![(
            Named::bare("examples/coil_vortex.kir"),
            vec![Named::bare("examples/star_flares.kir")],
        )]
    );
}

/// Both scripts must end after their last press, or the loop restarts
/// mid-gesture and what a watcher sees depends on when they looked.
#[test]
fn every_demo_script_finishes_before_it_loops() {
    for demo in [Demo::Transport, Demo::Lines] {
        let last = demo.script().last().expect("a script with entries").0;
        assert!(
            demo.loop_seconds() > last,
            "{demo:?} loops at {}s, before its last press at {last}s",
            demo.loop_seconds()
        );
    }
}

#[test]
fn a_bare_positional_pair_still_works() {
    let args = parse(&["a.kir", "b.kir"]).expect("should parse");
    assert_eq!(
        args.sets,
        vec![(Named::bare("a.kir"), vec![Named::bare("b.kir")])]
    );
}

#[test]
fn one_positional_file_is_rejected() {
    let err = parse(&["a.kir"]).unwrap_err();
    assert!(err.contains("1 file argument"), "message: {err}");
}

#[test]
fn three_positional_files_are_rejected() {
    let err = parse(&["a.kir", "b.kir", "c.kir"]).unwrap_err();
    assert!(err.contains("3 file argument"), "message: {err}");
}

#[test]
fn a_positional_pair_becomes_the_last_slot_after_set() {
    let args = parse(&["--set", "a.kir,b.kir", "c.kir", "d.kir"]).expect("should parse");
    assert_eq!(
        args.sets,
        vec![
            (Named::bare("a.kir"), vec![Named::bare("b.kir")]),
            (Named::bare("c.kir"), vec![Named::bare("d.kir")]),
        ]
    );
}

// -- --set, adversarially ---------------------------------------------

#[test]
fn set_with_no_value_fails() {
    let err = parse(&["--set"]).unwrap_err();
    assert!(err.contains("--set"), "message: {err}");
}

#[test]
fn set_with_one_path_and_no_comma_fails() {
    let err = parse(&["--set", "a.kir"]).unwrap_err();
    assert!(err.contains("--set a.kir"), "message: {err}");
}

/// A bare `--param` is a wildcard and stays one. Every node declaring the name
/// — "the Set's `exposure`", one knob moving both renderers — which is what
/// this flag has always meant, so the address arriving must not quietly turn it
/// into "node 0".
#[test]
fn a_param_with_no_address_reaches_every_node_declaring_it() {
    let args = parse(&["--param", "exposure=2.5"]).expect("should parse");
    assert_eq!(
        args.overrides,
        vec![ParamWrite::everywhere("exposure", 2.5)],
        "a bare name must stay unaddressed"
    );
}

/// And the prefix is what sets two renderers apart, which a bare name cannot do
/// by construction.
#[test]
fn a_param_can_address_one_renderer() {
    let args = parse(&["--param", "L4:1:exposure=2.5", "--param", "L1:0:radius=3.0"])
        .expect("should parse");
    assert_eq!(
        args.overrides,
        vec![
            ParamWrite::at(karakuri_ir::Kind::L4, 1, "exposure", 2.5),
            ParamWrite::at(karakuri_ir::Kind::L1, 0, "radius", 3.0),
        ]
    );
}

/// A half-address is refused rather than read as a name with a colon in it. The
/// address is `layer:index:` present or absent as a unit, and a param name
/// cannot contain a colon, so there is nothing else `L4:exposure` can be trying
/// to say.
#[test]
fn a_half_written_param_address_fails() {
    for bad in [
        "L4:exposure=2.5",
        "L4:x:exposure=2.5",
        "L9:0:exposure=2.5",
        ":0:e=1",
        "=2.5",
    ] {
        let err = parse(&["--param", bad]).unwrap_err();
        assert!(
            err.contains("--param"),
            "`{bad}` was accepted or misreported: {err}"
        );
    }
}

/// `--bind` needed no new grammar at all: its fields are the record's, so the
/// address is one more field. Absent is a wildcard there too.
#[test]
fn a_bind_can_address_one_renderer_and_defaults_to_all_of_them() {
    let all =
        parse(&["--bind", "layer=L4,key=exposure,signal=beat,range=0..1"]).expect("should parse");
    assert_eq!(
        all.bindings[0].index, None,
        "a bind with no index is the layer's"
    );

    let one = parse(&[
        "--bind",
        "layer=L4,index=2,key=exposure,signal=beat,range=0..1",
    ])
    .expect("should parse");
    assert_eq!(one.bindings[0].index, Some(2));

    let err = parse(&["--bind", "layer=L4,index=x,key=e,signal=beat,range=0..1"]).unwrap_err();
    assert!(err.contains("index"), "a bad index is not named: {err}");
}

/// A third path is a second renderer, and there is no new syntax for it.
///
/// This used to be an error, and the reasoning was sound while a Set was a
/// pair: `b.kir,c.kir` had been silently accepted as one literal L4 filename,
/// so a stray comma got blamed on a missing file. Now a Set holds a list, and
/// one comma-separated list read as one L1 and however many L4s is exactly what
/// the command line should look like — *no new syntax at all*. See
/// `docs/adr/0154-a-third-path-on-set-is-a-second-renderer-and-there-is-no-new-syntax.md`.
#[test]
fn set_with_three_paths_is_one_geometry_and_two_renderers() {
    let args = parse(&["--set", "a.kir,b.kir,c.kir"]).expect("should parse");
    assert_eq!(
        args.sets,
        vec![(
            Named::bare("a.kir"),
            vec![Named::bare("b.kir"), Named::bare("c.kir")],
        )],
        "the first path is the geometry and the rest are renderers, in draw order"
    );
}

/// The stray comma the rule above used to catch is still caught, because an
/// empty part is not a filename: `a.kir,b.kir,` names a renderer with no name.
#[test]
fn set_with_a_trailing_comma_fails() {
    let err = parse(&["--set", "a.kir,b.kir,"]).unwrap_err();
    assert!(err.contains("--set"), "message: {err}");
}

#[test]
fn set_with_an_empty_value_fails() {
    let err = parse(&["--set", ""]).unwrap_err();
    assert!(err.contains("--set"), "message: {err}");
}

#[test]
fn set_with_an_empty_side_fails() {
    assert!(parse(&["--set", ",b.kir"]).is_err());
    assert!(parse(&["--set", "a.kir,"]).is_err());
}

#[test]
fn set_given_five_times_exceeds_max_slots() {
    let err = parse(&[
        "--set", "a,b", "--set", "c,d", "--set", "e,f", "--set", "g,h", "--set", "i,j",
    ])
    .unwrap_err();
    assert!(err.contains("5 Sets"), "message: {err}");
}

#[test]
fn a_bare_pair_alongside_four_sets_exceeds_max_slots() {
    let err = parse(&[
        "--set", "a,b", "--set", "c,d", "--set", "e,f", "--set", "g,h", "i.kir", "j.kir",
    ])
    .unwrap_err();
    assert!(err.contains("5 Sets"), "message: {err}");
}

#[test]
fn set_given_exactly_four_times_is_allowed() {
    let args = parse(&[
        "--set", "a,b", "--set", "c,d", "--set", "e,f", "--set", "g,h",
    ])
    .expect("4 sets should fit MAX_SLOTS");
    assert_eq!(args.sets.len(), 4);
}

// -- --exposure, adversarially -----------------------------------------

#[test]
fn exposure_missing_value_fails() {
    assert!(parse(&["--exposure"]).is_err());
}

#[test]
fn exposure_non_number_fails() {
    let err = parse(&["--exposure", "bright"]).unwrap_err();
    assert!(err.contains("--exposure bright"), "message: {err}");
}

#[test]
fn exposure_negative_fails() {
    assert!(parse(&["--exposure", "-2"]).is_err());
}

#[test]
fn exposure_zero_fails() {
    assert!(parse(&["--exposure", "0"]).is_err());
}

#[test]
fn exposure_positive_is_accepted_unclamped() {
    // Parsing does not clamp — only the interactive control does, on
    // purpose, so a batch render can ask for something extreme. `100`
    // is well outside the interactive `-`/`=` bound.
    let args = parse(&["--exposure", "100"]).expect("should parse");
    assert_eq!(args.look.exposure, 100.0);
}

// -- --tonemap, adversarially -------------------------------------------

#[test]
fn tonemap_bad_name_fails() {
    let err = parse(&["--tonemap", "bloom"]).unwrap_err();
    assert!(err.contains("--tonemap bloom"), "message: {err}");
}

#[test]
fn tonemap_every_documented_name_is_accepted() {
    for (name, op) in [
        ("clamp", TonemapOp::Clamp),
        ("reinhard", TonemapOp::Reinhard),
        ("aces", TonemapOp::Aces),
        ("agx", TonemapOp::AgX),
    ] {
        let args = parse(&["--tonemap", name]).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(args.look.op, op, "{name}");
    }
}

// -- --bind ---------------------------------------------------------------

/// The spec's own example, field for field. This is the mapping the flag
/// exists to preserve: when a Set file can be loaded, each `field=value`
/// here becomes the JSON field of the same name and nothing else changes.
///
/// ```ndjson
/// {"t":"bind","layer":"L1","key":"spawn_rate","signal":"noise",
///  "noise":{"kind":"perlin","rate":0.5,"stream":3},"curve":"lin","range":[4000,16000]}
/// ```
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

/// `octaves` belongs to `fbm` and, per `docs/ir-spec.md`, is "ignored by the
/// other three kinds". What it must never do is decide the kind: a
/// `noise.kind=white` that came back as `fbm` is a different generator from the
/// one the operator named, chosen silently, in a flag whose whole stated reason
/// for refusing unknown fields is that silence.
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

/// A binding to `bpm` is pinned at the top of its range for the whole run,
/// because a tempo is not a `[0, 1]` signal and the curve clamps it. That is
/// `--param key=HIGH` spelled at four times the length, and nothing on the
/// outside distinguishes it from a binding that is working — the status line
/// shows a number, and the number never moves. `bpm` is a signal the spec
/// lists, so a generator will reach for it; the refusal is what tells it to
/// reach for `beat` instead.
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

/// What a Set file recorded is what the run uses, and an ordinal fills in the
/// rest.
///
/// The two halves are one function on purpose: this is what a slot is built
/// with *and* what `--save-set` writes down, so a file cannot record a salt the
/// run was not using — the failure [`saving_capacities`] was fixed after, one
/// field along. An unsaved `--set` is the ordinals, which is what makes saving
/// a no-op on the picture and reloading a reproduction of it.
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

/// The flag and the file agree about compositing in either order, and leaving
/// the flag off takes nothing away from a file that records a merge.
///
/// `--merge` can only turn compositing *on* — there is no spelling that turns
/// it off, because overdraw is what saying nothing means — so `--load-set X
/// --merge 0` on a composited file is two ways of asking for one thing rather
/// than a contest. The case that decides the rule is the third: a composited
/// file loaded with no flag. Reading the missing flag as "overdraw" would make
/// every saved variant pool come back unfoldable, which is exactly the state
/// the `merge` record was added to end.
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

/// What a live save writes as the merge's `live`, read off the Set.
///
/// `k` and the MCP tool write what is on screen, and the fold is one of the
/// things only the Set knows: there is no flag that selects a renderer.
/// Every-input-live is `None` and not renderer 0 — a Set nobody has selected in
/// has made no choice to record, and a one-renderer Set is that case rather
/// than a selection of its only renderer.
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

// -- key-handling logic, separated from winit and the GPU ---------------

#[test]
fn slot_bounds_reject_the_slot_count_itself_and_beyond() {
    // The neighbour of the fixed off-by-one: digits are indices, so the
    // valid range is `0..slot_count`, and `slot_count` itself is already
    // out of range.
    assert!(slot_in_range(0, 4));
    assert!(slot_in_range(3, 4));
    assert!(!slot_in_range(4, 4));
    assert!(!slot_in_range(9, 4));
    // A deck of one: only slot 0 is valid, matching the default run.
    assert!(slot_in_range(0, 1));
    assert!(!slot_in_range(1, 1));
}

/// A parked slot is `Residency::Allocated`, exactly like a slot nobody asked
/// about, and the operator's request is the only thing that tells them apart.
/// Showing one as the other is not a cosmetic loss: it says a standing request
/// was discarded, when the governor is reconsidering it every pass and will
/// grant it the moment a slot comes off air.
#[test]
fn a_parked_slot_does_not_read_as_one_nobody_asked_about() {
    assert_ne!(
        residency_tag(Residency::Allocated, true),
        residency_tag(Residency::Allocated, false)
    );
    assert_ne!(
        residency_name(Residency::Allocated, true),
        residency_name(Residency::Allocated, false)
    );
    // Parked is off air, so it must not read as either of the two states
    // that are not: a park shown as `prim` claims warming that is not
    // happening, and shown as `LIVE` claims a slot on air.
    for parked in [true, false] {
        assert_ne!(
            residency_tag(Residency::Allocated, parked),
            residency_tag(Residency::Priming, parked)
        );
        assert_ne!(
            residency_tag(Residency::Allocated, parked),
            residency_tag(Residency::Live, parked)
        );
    }
    // `parked` is only ever true of an Allocated slot — `Deck::is_parked`
    // says so — but the tag is fixed-width regardless of what it is asked,
    // because the columns after it are positional.
    for residency in [Residency::Live, Residency::Priming, Residency::Allocated] {
        for parked in [true, false] {
            assert_eq!(residency_tag(residency, parked).len(), 4);
        }
    }
}

/// A stopped slot says so on the status line, and no other slot does.
///
/// A version over the frame budget stays in the slot and the slot stops
/// updating (ADR-0316), so the line an operator reads in the dark shows `LIVE`
/// beside a simulation clock that is not moving. The word is the only thing
/// that separates that from a bug, and it is the maintainer's: *"rolled
/// backが分かりにくい"*.
///
/// Three things, and each is a way of getting it wrong. It is the engine's own
/// word rather than a fourth spelling of the same state, which is what the
/// lane, the health capsule and `swap_outcome` all say. It is separated from
/// what follows it, or the fader beside it runs into it. And a slot that is not
/// stopped adds nothing at all, because the default status line is what a run
/// that is behaving prints and a column reading `running` four times is four
/// columns of nothing to read.
#[test]
fn a_stopped_slot_is_the_only_one_the_status_line_says_anything_about() {
    assert_eq!(stopped_tag(false), "");
    assert!(
        stopped_tag(true).starts_with("overloaded"),
        "the status line does not use the word every other surface uses"
    );
    assert!(
        stopped_tag(true).ends_with(' '),
        "the word runs into the column after it"
    );
    // And it is not a residency, which is the one reading it must not take:
    // a stopped slot keeps whichever of the four it had.
    for residency in [Residency::Live, Residency::Priming, Residency::Allocated] {
        for parked in [true, false] {
            assert_ne!(
                residency_tag(residency, parked).trim(),
                stopped_tag(true).trim(),
                "the stopped word is spelled like a residency"
            );
        }
    }
}

#[test]
fn gain_floors_at_zero_but_has_no_ceiling() {
    assert_eq!(clamp_gain(-5.0), 0.0);
    assert_eq!(clamp_gain(0.0), 0.0);
    assert_eq!(clamp_gain(1.0), 1.0);
    assert_eq!(clamp_gain(1000.0), 1000.0, "HDR gain is not capped at 1.0");
}

#[test]
fn exposure_clamps_into_a_finite_positive_range() {
    assert_eq!(clamp_exposure(0.0), EXPOSURE_MIN);
    assert_eq!(clamp_exposure(-5.0), EXPOSURE_MIN);
    assert_eq!(clamp_exposure(1_000_000.0), EXPOSURE_MAX);
    assert_eq!(clamp_exposure(1.0), 1.0);
}

/// The cycle visits every operator and closes — and, because `next_tonemap` is
/// an exhaustive match while `TONEMAPS` is a hand-written list, this is also
/// what checks the list is complete. An operator added to the enum forces a new
/// arm in the cycle; if `TONEMAPS` is not updated with it the two disagree
/// here, before it can reach a `look` record that spells a name nothing parses.
#[test]
fn tonemap_cycles_through_all_four_and_back_to_the_start() {
    let start = TonemapOp::Clamp;
    let mut op = start;
    let mut seen = vec![op];
    for _ in 0..TONEMAPS.len() - 1 {
        op = next_tonemap(op);
        seen.push(op);
    }
    assert_eq!(
        seen,
        vec![
            TonemapOp::Clamp,
            TonemapOp::Reinhard,
            TonemapOp::Aces,
            TonemapOp::AgX,
        ]
    );
    assert_eq!(next_tonemap(op), start, "the cycle must close");
    assert_eq!(
        seen.len(),
        TONEMAPS.len(),
        "the cycle and `TONEMAPS` disagree about how many operators there are"
    );
    for op in TONEMAPS {
        assert!(seen.contains(&op), "{} is not in the cycle", op_name(op));
    }
}

/// Both spellings of every operator are distinct from every other's, so a
/// `look` record cannot name two operators and `--tonemap` cannot resolve to
/// the wrong one. Two arms of `spellings` sharing a wire name would compile and
/// would make `parse_op` return whichever came first.
#[test]
fn no_two_tonemap_operators_share_a_spelling() {
    for op in TONEMAPS {
        assert_eq!(
            parse_op(op_wire_name(op)),
            Some(op),
            "`{}` does not parse back to itself",
            op_wire_name(op)
        );
    }
    let mut wire: Vec<&str> = TONEMAPS.iter().map(|op| op_wire_name(*op)).collect();
    wire.sort_unstable();
    let before = wire.len();
    wire.dedup();
    assert_eq!(before, wire.len(), "two operators share a wire spelling");
}

/// A `save` after the last tick is named, not counted.
///
/// The `ir-spec` rule is that a replay says which effects outside the stream it
/// skipped, and a save at the end of a set is the one that lands after the last
/// tick rather than inside a frame. Paired with its control — a trailing record
/// that is *not* a save gets the count and nothing more, so the assertion is
/// about the `save` arm rather than about a function that prints an extra line
/// whatever it is given.
#[test]
fn a_trailing_save_is_named_and_not_only_counted() {
    let notes = trailing_notes(&[
        Record::Gain {
            slot: DeckSlot(1),
            value: 0.5,
        },
        Record::Save {
            slot: DeckSlot(2),
            id: "20260816-143052-271".to_string(),
        },
    ]);
    assert_eq!(notes.len(), 2, "the skipped save was not named: {notes:?}");
    assert!(
        notes[0].starts_with("2 records after the last tick"),
        "{notes:?}"
    );
    assert!(
        notes[1].contains("slot 2") && notes[1].contains("20260816-143052-271"),
        "a skipped save has to name the slot and the set an operator would \
         go and load: {notes:?}"
    );

    // **The control.** Nothing to name and the count stands alone.
    let counted = trailing_notes(&[Record::Gain {
        slot: DeckSlot(1),
        value: 0.5,
    }]);
    assert_eq!(
        counted.len(),
        1,
        "a trailing record that reaches nothing outside the stream is counted \
         and no more: {counted:?}"
    );
    assert!(counted[0].starts_with("1 record after"), "{counted:?}");
}

#[cfg(test)]
mod value_tests {
    use super::*;

    /// The error, or `None` if the arguments were accepted. `ParseOutcome` holds
    /// GPU-adjacent config and is not `Debug`, and giving it one just so a test can
    /// print it would be the tail wagging the dog.
    fn parse(args: &[&str]) -> Option<String> {
        parse_args_from(args.iter().map(|s| s.to_string())).err()
    }

    /// Every flag that takes a value refuses a bad one rather than keeping its
    /// default. The silence is the bug: a run that quietly used 240 frames is
    /// indistinguishable from one that honoured the `--frames` that was typed.
    #[test]
    fn a_flag_given_a_value_it_cannot_use_says_so() {
        for (args, expect) in [
            (vec!["--frames", "24O"], "--frames"),
            (vec!["--capacity", "lots"], "--capacity"),
            (vec!["--budget-ms", "soon"], "--budget-ms"),
            (vec!["--size", "1280"], "--size"),
            (vec!["--size", "1280x"], "--size"),
            (vec!["--canvas", "1920"], "--canvas"),
            (vec!["--canvas", "x1080"], "--canvas"),
            (vec!["--param", "turbulence"], "--param"),
        ] {
            let err = parse(&args).unwrap_or_else(|| panic!("{args:?} was accepted"));
            assert!(err.contains(expect), "{args:?} -> {err}");
        }
    }

    /// Zero is a typo, not a size. Every texture descriptor downstream takes
    /// `max(1)` to stay legal, so `--canvas 1920x0` would have rendered a frame one
    /// texel tall and reported the size it was asked for — the same silence as a
    /// `--frames 24O` that renders 240.
    #[test]
    fn an_extent_with_a_zero_side_is_refused_rather_than_clamped() {
        for args in [
            vec!["--canvas", "1920x0"],
            vec!["--canvas", "0x1080"],
            vec!["--canvas", "0x0"],
            vec!["--size", "0x720"],
        ] {
            let err = parse(&args).unwrap_or_else(|| panic!("{args:?} was accepted"));
            assert!(err.contains("zero"), "{args:?} -> {err}");
        }
    }

    /// `--size` is the preview window's, and these three runs have no window.
    ///
    /// Refused rather than ignored because of what it used to mean: it was the
    /// render size, so a reader typing `--render out.png --size 1920x1080` means
    /// `--canvas`. Quietly rendering at the default instead would be the worst of
    /// the three outcomes — a PNG at a size nobody asked for, with nothing on
    /// stderr.
    #[test]
    fn the_preview_windows_size_is_refused_where_there_is_no_window() {
        for args in [
            vec!["--render", "out.png", "--size", "1920x1080"],
            vec!["--seq", "frames", "--size", "1920x1080"],
            vec![
                "--replay",
                "s",
                "--render",
                "out.png",
                "--size",
                "1920x1080",
            ],
        ] {
            let err = parse(&args).unwrap_or_else(|| panic!("{args:?} was accepted"));
            assert!(err.contains("no window"), "{args:?} -> {err}");
        }
        // And it is accepted wherever a window exists, including beside
        // `--canvas`: the two describe different things and were split so they
        // could be given together.
        assert!(parse(&["--size", "800x600"]).is_none());
        assert!(parse(&["--size", "800x600", "--canvas", "1920x1080"]).is_none());
        assert!(parse(&["--render", "out.png", "--canvas", "1920x1080"]).is_none());
    }

    /// A surface with nobody at it, and one more reason besides.
    ///
    /// The other refusals are "an offscreen run takes no live input". This one is
    /// that plus the sharper version: a port that can rewrite a procedure
    /// mid-render is the opposite of an output that is a function of its arguments.
    #[test]
    fn an_mcp_port_is_refused_where_there_is_no_run_to_drive() {
        for args in [
            vec!["--mcp", "8000", "--render", "out.png"],
            vec!["--mcp", "8000", "--seq", "frames/"],
            vec!["--mcp", "8000", "--replay", "a", "--render", "out.png"],
        ] {
            let err = parse(&args).unwrap_or_else(|| panic!("{args:?} was accepted"));
            assert!(err.contains("--mcp"), "{args:?} -> {err}");
        }
        assert!(parse(&["--mcp", "8000"]).is_none());
        assert!(parse(&["--mcp", "8000", "--watch"]).is_none());
        // A port is a number, and a flag given a value it cannot use says so
        // rather than keeping a default.
        assert!(parse(&["--mcp", "eight-thousand"]).is_some());
    }

    /// A tempo source is a live input, and an offscreen run has none.
    ///
    /// The replay half has its own reason and it is the stronger one: a replay
    /// follows the grid the session recorded, so a live source would be overwriting
    /// the performance it is supposed to be reproducing.
    #[test]
    fn a_tempo_source_is_refused_where_there_is_no_performance_to_follow() {
        for args in [
            vec!["--tempo-source", "helper", "--render", "out.png"],
            vec!["--tempo-source", "helper", "--seq", "frames/"],
            vec![
                "--tempo-source",
                "helper",
                "--replay",
                "a",
                "--render",
                "out.png",
            ],
        ] {
            let err = parse(&args).unwrap_or_else(|| panic!("{args:?} was accepted"));
            assert!(err.contains("--tempo-source"), "{args:?} -> {err}");
        }
        assert!(parse(&["--tempo-source", "helper"]).is_none());
        assert!(parse(&["--tempo-source", "helper", "--audio-in", "default"]).is_none());
    }

    /// A recorder that never gets built is refused rather than dropped.
    ///
    /// It used to exit 0 having written the PNG and no session at all — no warning,
    /// no file — because the recorder is only constructed on the path that opens a
    /// window. The worst shape of failure this program has: not a wrong output but
    /// a missing one, reported as success, from a flag whose whole purpose is to
    /// leave something behind.
    #[test]
    fn recording_is_refused_where_there_is_no_performance() {
        for args in [
            vec!["--render", "out.png", "--record-session", "s"],
            vec!["--seq", "frames", "--record-session", "s"],
            vec![
                "--replay",
                "a",
                "--render",
                "out.png",
                "--record-session",
                "s",
            ],
        ] {
            let err = parse(&args).unwrap_or_else(|| panic!("{args:?} was accepted"));
            assert!(err.contains("--record-session"), "{args:?} -> {err}");
        }
        // A live run is where it belongs, and it does **not** need `--load-set`:
        // the material is saved under `ID-material` and put at the head. The
        // help text said otherwise long after that stopped being true.
        assert!(parse(&["--record-session", "s"]).is_none());
        assert!(parse(&["--record-session", "s", "--load-set", "base"]).is_none());
    }

    /// A replay renders at the size the session recorded, and `--canvas` is refused
    /// only when there is something to refuse it against.
    ///
    /// The refusal was unconditional and at parse time, which was wrong for exactly
    /// the streams that need the flag: a session written before this record existed
    /// carries no size, so the flag was rejected with the words "the session
    /// records what it rendered at" and the replay then ran at the untouched
    /// default. That is a regression against the old `--size`, which could set it.
    #[test]
    fn a_replay_refuses_the_canvas_flag_only_when_the_stream_has_one() {
        let performed = Some((1280, 720));
        let flag = (640, 480);

        assert_eq!(
            replay_canvas(performed, flag, false),
            Ok(((1280, 720), None)),
            "the stream's size, and nothing to say about it"
        );

        let refusal = replay_canvas(performed, flag, true).expect_err("was accepted");
        assert!(refusal.contains("--canvas"), "{refusal}");

        // No record: the flag is the way out, and either way it is named,
        // because a replay at a size nobody can vouch for must not look like a
        // faithful one.
        let (size, note) = replay_canvas(None, flag, true).expect("the flag is allowed");
        assert_eq!(size, flag);
        assert!(note.expect("says so").contains("640x480"));

        let (size, note) = replay_canvas(None, flag, false).expect("falls back");
        assert_eq!(size, flag);
        assert!(note.expect("says so").contains("guess"));

        // And the flag reaches parsing at all, which the old refusal blocked.
        assert!(parse(&["--replay", "s", "--render", "out.png", "--canvas", "640x480"]).is_none());
    }

    /// A flag at the end of the line has no value, and the message says which flag
    /// rather than blaming whatever came before it.
    #[test]
    fn a_flag_with_nothing_after_it_says_which_flag() {
        for flag in [
            "--render",
            "--seq",
            "--frames",
            "--size",
            "--set",
            "--exposure",
        ] {
            let err = parse(&[flag]).unwrap_or_else(|| panic!("`{flag}` alone was accepted"));
            assert!(
                err.contains(flag) && err.contains("needs a value"),
                "{flag} -> {err}"
            );
        }
    }

    /// `--render` with the path forgotten used to open a window: `None` fell
    /// through to the interactive branch, so a batch script with a typo hung
    /// instead of failing.
    #[test]
    fn a_flag_does_not_swallow_the_next_flag() {
        let err = parse(&["--render", "--frames", "10"]).expect("`--render --frames` was accepted");
        assert!(err.contains("--render") && err.contains("not one"), "{err}");
    }

    /// A negative number is a value, not an option — the check is "looks like a
    /// flag", and `-1.5` does not.
    #[test]
    fn a_negative_number_still_reads_as_a_value() {
        let err = parse(&["--exposure", "-1.5"]).expect("a negative exposure was accepted");
        assert!(
            err.contains("positive"),
            "rejected for the wrong reason: {err}"
        );
    }
}

/// What a live save actually writes, through a Set that has been built and then
/// played with.
///
/// Everything else about saving is checked against flags — see the `--save-set`
/// tests above and `setfile`'s own. The claim a *live* save makes is a
/// different one and no flag can stand in for it: what reaches the file is what
/// is on screen, after a parameter has moved and after the file under a slot
/// has moved without it. So these build a real Set, disturb it the way a run
/// disturbs one, and read the file back through the loader `--load-set` uses.
#[cfg(test)]
mod live_save_tests {
    use super::*;
    // The card writer, reached only by the test that pins what it says when a
    // disk refuses it.
    use karakuri_environment::meta::put_meta;
    use std::path::Path;

    /// Integration tests get the *package* as their working directory, and unit
    /// tests get it too — so `examples/` is two levels up from here.
    fn workspace() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("workspace root")
    }

    fn example(name: &str) -> String {
        std::fs::read_to_string(workspace().join("examples").join(name)).expect("an example")
    }

    /// Write `src` into `dir` and hand back the path, so a test can edit a
    /// procedure the way an operator does — by replacing the file.
    fn kir(dir: &tempfile::TempDir, name: &str, src: &str) -> PathBuf {
        let path = dir.path().join(name);
        std::fs::write(&path, src).expect("write");
        path
    }

    /// One slot's files, compiled and sorted exactly as a run compiles them — so
    /// `Placed` here carries the layers and indices the real thing carries.
    fn slot(paths: &[PathBuf]) -> (Material, Vec<Placed>) {
        let rest: Vec<Named> = paths[1..].iter().cloned().map(Named::bare).collect();
        sort_slot(0, &Named::bare(paths[0].clone()), &rest)
    }

    /// A Set built from that material, at the capacities, salts and camera given —
    /// which is what a run hands `build`, and what a save has to read back off the
    /// Set rather than off these arguments.
    ///
    /// `camera` is an `Option` because `build`'s is: `None` is a slot no `camera`
    /// record reached, which leaves the Set at the built-in orbit's defaults. See
    /// `recorded_camera`.
    fn set_of(
        gpu: &Gpu,
        material: &Material,
        capacities: &[u32],
        salts: &[u32],
        overrides: &[ParamWrite],
        camera: Option<karakuri_engine::camera::Orbit>,
    ) -> Set {
        let mut attached = vec![false; 0];
        build(
            gpu,
            &material.l1s,
            &material.l2s,
            &material.l3s,
            &material.fields,
            &material.l4s,
            karakuri_engine::set::Layering::Overdraw,
            &material.names,
            &[],
            capacities,
            &mut attached,
            overrides,
            &[],
            &[],
            salts[0],
            salts,
            camera,
            // No selection: these Sets overdraw, and a fold nobody built has
            // no renderer to be folded to. See `recorded_live`.
            None,
        )
    }

    /// What a run does to one slot before its first frame: the slot recorded as
    /// running the material that was compiled for it.
    ///
    /// Through [`Running::at_launch`] rather than around it, for the reason
    /// [`save_and_load`] goes through [`Save::run`]: assembling the seeding by hand
    /// here would be a second copy of it, and the whole of what these tests are
    /// about is that there is only one.
    ///
    /// No store root, because there is no store. A windowed run touches one when a
    /// save happens and not before — see [`Running::at_launch`].
    fn launched(placed: &[Placed]) -> Running {
        Running::at_launch(&[placed.to_vec()], 1)
    }

    /// One node as the watcher hands it over when a build lands: the source in the
    /// store, and its address beside the hash.
    fn stored(store: &karakuri_store::store::Store, layer: &'static str, src: &str) -> Landed {
        (layer, 0, store.put_artifact(src.as_bytes()).expect("put"))
    }

    type Landed = (&'static str, u32, karakuri_store::hash::Hash);

    /// Gather, write, and read back — the whole of what pressing `k` does, minus
    /// the thread and the channel.
    ///
    /// Through [`Save::run`] rather than around it, so that what a test exercises
    /// is the function the spawned thread calls. Assembling the same three steps by
    /// hand here would be a second copy of the save, and a test of a copy is a test
    /// of nothing.
    fn save_and_load(store_root: &Path, id: &str, set: &Set, sources: Sources) -> setfile::Loaded {
        Save {
            slot: 0,
            asked: Asked::Operator,
            id: id.to_string(),
            root: store_root.to_path_buf(),
            sources,
            values: playing_values(set, &[]),
        }
        .run()
        .expect("the Set file is written");
        let store = karakuri_store::store::Store::open(store_root).expect("store");
        setfile::load(&store, id).expect("and reads back")
    }

    /// A geometry with a negative default, which is the declaration the fold exists
    /// for and the one no example in the tree carries.
    ///
    /// `drift_shell` with one param added and read, so the procedure still
    /// compiles, still draws, and now declares a default that is `Unary { Neg, Lit
    /// }` rather than a literal.
    fn signed_l1() -> String {
        let src = example("drift_shell.kir");
        let with_param = src.replace(
            "  param drift      : float [0.0, 2.0] = 0.6",
            "  param drift      : float [0.0, 2.0] = 0.6\n  \
             param signed     : float [-1.0, 1.0] = -0.35",
        );
        assert_ne!(with_param, src, "the example's params moved");
        let with_use = with_param.replace(
            "    position = p;",
            "    position = p + vec3(signed, 0.0, 0.0);",
        );
        assert_ne!(with_use, with_param, "the example's element block moved");
        with_use
    }

    /// A stored artifact has a card beside it, and the card says what the `.kir`
    /// declares — every param with its range, the capacity range and its default,
    /// and the emitted attributes.
    ///
    /// `docs/ir-spec.md`'s metadata section long described records that nothing
    /// wrote and nothing read, so every claim it made was unenforced prose. This is
    /// the half a compile pass can produce, pinned against a source a reader can
    /// check it against by eye.
    ///
    /// Including a negative default, which is the one declaration a second reader
    /// of defaults gets wrong — see
    /// [`the_engine_and_the_metadata_writer_cannot_disagree_about_a_default`],
    /// which is the other half of that and needs a GPU. This one needs none:
    /// putting an artifact is a compile and two files.
    ///
    /// Through [`Placed::put`] rather than around it. That is the funnel every
    /// stored artifact goes through — the watcher's builds, a recorded run's
    /// seeding, and `--save-set` — so a test that assembled the write by hand would
    /// be a test of a copy.
    #[test]
    fn a_stored_artifact_has_a_card_saying_what_its_source_declares() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = vec![
            kir(&dir, "a.kir", &signed_l1()),
            kir(&dir, "r.kir", &example("soft_points.kir")),
        ];
        let (_material, placed) = slot(&paths);
        let store =
            karakuri_store::store::Store::open(dir.path().join("store")).expect("a store opens");

        let hash = placed[0].put(&store).expect("the artifact is stored");
        let card = store
            .read_meta(&hash)
            .expect("an artifact that was put has a card beside it");
        let records: Vec<Record> = card.iter().map(|l| l.record().clone()).collect();

        assert_eq!(
            records.first(),
            Some(&Record::Meta {
                hash,
                name: "drift_shell".to_string(),
                kind: Layer::L1,
                v: 1,
            }),
            "a card's head names the artifact it describes and the procedure's \
             own name"
        );

        let declared: Vec<(&str, f32, f32, Option<f32>)> = records
            .iter()
            .filter_map(|r| match r {
                Record::ParamDecl {
                    key,
                    ty,
                    min,
                    max,
                    default,
                } => {
                    assert_eq!(ty, "float", "`{key}` is declared a float in the source");
                    Some((key.as_str(), *min, *max, *default))
                }
                _ => None,
            })
            .collect();
        assert_eq!(
            declared,
            vec![
                ("radius", 0.1, 8.0, Some(2.6)),
                ("turbulence", 0.0, 3.0, Some(1.1)),
                ("swirl", 0.0, 2.0, Some(0.35)),
                ("drift", 0.0, 2.0, Some(0.6)),
                // The one the fold exists for: `= -0.35` is a negation of a
                // literal, and a card that read `Expr::Lit` alone would carry
                // no `default` here at all.
                ("signed", -1.0, 1.0, Some(-0.35)),
            ],
            "the params a card declares are not the source's, in the source's order"
        );

        assert!(
            records.contains(&Record::CapacityDecl {
                min: 4096,
                max: 1048576,
                default: 262144,
            }),
            "the declared capacity range is not on the card: {records:?}"
        );
        assert!(
            records.contains(&Record::Emit {
                attrs: vec![
                    "position".to_string(),
                    "velocity".to_string(),
                    "age".to_string(),
                ],
            }),
            "the emitted attributes are not on the card: {records:?}"
        );
    }

    /// A card that will not write is said out loud and does not fail the save.
    ///
    /// The policy [`Placed::put`] states: the artifact is the thing, its card is
    /// derived from the `.kir` plus a compile pass and regenerates on the next one,
    /// so a store holding the source and no card holds everything that cannot be
    /// recovered — and failing the put instead would lose an operator a save over a
    /// file nothing has read yet. Both halves were prose until here, and the half
    /// that rots quietly is the second: a `put` that started returning the card's
    /// error would be caught by any test that saves, while a `put_meta` that
    /// swallowed it would be caught by none.
    ///
    /// The card's own path taken by a directory, because that is the one failure
    /// that reaches the card and nothing else. A read-only store root would fail
    /// `put_artifact` first and prove the opposite thing. The path is spelled here
    /// the way `Store::meta_path` spells it — it is private — as `karakuri-store`'s
    /// own metadata tests spell it.
    #[test]
    fn a_card_that_will_not_write_is_said_and_does_not_fail_the_save() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = vec![
            kir(&dir, "a.kir", &example("drift_shell.kir")),
            kir(&dir, "r.kir", &example("soft_points.kir")),
        ];
        let (_material, placed) = slot(&paths);
        let root = dir.path().join("store");
        let store = karakuri_store::store::Store::open(&root).expect("a store opens");

        let hash = placed[0].hash();
        std::fs::create_dir(root.join(format!("{}.meta.ndjson", hash.short(64))))
            .expect("the card's path is taken");

        assert_eq!(
            placed[0]
                .put(&store)
                .expect("the save is not failed by a card"),
            hash,
            "a put that survived its card came back with another artifact"
        );
        assert!(
            store.get_artifact(&hash).is_ok(),
            "the source did not reach the store, so nothing here was recoverable"
        );
        assert!(
            store.read_meta(&hash).is_err(),
            "the card was written after all, and this test proves nothing"
        );

        // The same call the put makes, read back rather than watched on a
        // terminal — see [`put_meta`], which returns what it printed for this.
        let said = put_meta(&store, &hash, &placed[0].meta)
            .expect("a card that would not write is reported");
        assert!(
            said.contains(&hash.short(12)),
            "the operator is not told which artifact: {said}"
        );
        assert!(
            said.contains("the artifact is stored"),
            "the operator is not told the save survived: {said}"
        );
    }

    /// A build whose sources never reached the store is still a swap.
    ///
    /// The watcher compiles, fails to `put_artifact`, says so, and returns the
    /// `Request` anyway — so the build goes on screen with no address anybody can
    /// name. `Live::took_up` used to answer that by returning before touching
    /// [`Running`] at all, which left the slot reading as the version it was
    /// running *before* the one on screen: `k` wrote that version down and a
    /// recorded run put `procedure` records naming it into the stream, with nothing
    /// anywhere saying the file and the picture had come apart.
    ///
    /// A slot showing a version nobody can name has no address, and saying so is
    /// the whole of the repair: `None` is what a save refuses on and what a record
    /// is not written from, and both of those are better than a hash that points at
    /// the wrong material.
    ///
    /// No GPU and no window: the transition is the whole subject, which is what
    /// [`Running`] being its own type is for.
    #[test]
    fn a_build_whose_sources_never_reached_the_store_is_still_a_swap() {
        let dir = tempfile::tempdir().expect("tempdir");
        let at_start = example("drift_shell.kir");
        let renderer = example("soft_points.kir");
        let paths = vec![kir(&dir, "a.kir", &at_start), kir(&dir, "r.kir", &renderer)];
        let (_material, placed) = slot(&paths);
        let root = dir.path().join("store");
        let store = karakuri_store::store::Store::open(&root).expect("store");

        let launch: Vec<Landed> = vec![
            ("L1", 0, karakuri_store::hash::Hash::of(at_start.as_bytes())),
            ("L4", 0, karakuri_store::hash::Hash::of(renderer.as_bytes())),
        ];

        let mut running = launched(&placed);
        assert_eq!(
            running.playing(0),
            Some(&launch),
            "the slot did not start on what it launched with, so nothing below \
             is about the transition"
        );

        // Build A: it compiled, the watcher stored it, the swap landed.
        let build_a = at_start.replace("proc drift_shell", "proc build_a");
        let a: Vec<Landed> = vec![
            stored(&store, "L1", &build_a),
            stored(&store, "L4", &renderer),
        ];
        running.landed(0, Some(a.clone()));

        assert_eq!(
            running.playing(0),
            Some(&a),
            "a build that was stored is not what the slot reads as running"
        );

        // Build B: it compiled, the store refused it, and the deck installed it
        // regardless — which is what `watch::Watch::poll` does, and what makes
        // this a state the run can actually be in.
        running.landed(0, None);
        assert!(
            running.playing(0).is_none(),
            "a slot showing a version nothing can name reported an address, and \
             whatever it reported is not what is on screen"
        );

        // **And the next build that can be named puts the slot back on an
        // address**, which is what makes `None` a state rather than a dead end:
        // nothing has to remember A for the slot to become savable again.
        let c: Vec<Landed> = vec![
            stored(
                &store,
                "L1",
                &at_start.replace("proc drift_shell", "proc build_c"),
            ),
            stored(&store, "L4", &renderer),
        ];
        running.landed(0, Some(c.clone()));
        assert_eq!(
            running.playing(0),
            Some(&c),
            "a build after an unnameable one did not put the slot back on an address"
        );
    }

    /// A windowed run creates no store until something is saved.
    ///
    /// `docs/manual.md` promises that a run which cannot be edited "copies nothing
    /// and creates no directory", and the launch seeding falsified it: it opened
    /// the store — which `create_dir_all`s the root and its subdirectories — and
    /// wrote one artifact per node, so a plain `karakuri-cli examples/...` left a
    /// `.karakuri` behind in whatever directory it was run from.
    ///
    /// The seeding costs nothing because an address is not a file. The second
    /// assertion is what makes the first mean something: the slot knows exactly
    /// what it is running, it simply has not written it anywhere. The bytes reach
    /// the store at the moment a file names them — see [`Sources::into_nodes`],
    /// which
    /// [`a_rewrite_between_the_compile_and_the_first_frame_cannot_reach_a_save`]
    /// reads back off the disk.
    #[test]
    fn a_windowed_run_creates_no_store_until_something_is_saved() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = vec![
            kir(&dir, "a.kir", &example("drift_shell.kir")),
            kir(&dir, "r.kir", &example("soft_points.kir")),
        ];
        let (_material, placed) = slot(&paths);
        let root = dir.path().join("store");

        let running = launched(&placed);
        assert!(
            !root.exists(),
            "a run that was asked to draw a window and nothing else created a \
             store, which `docs/manual.md` promises it does not"
        );
        assert_eq!(
            running.playing(0).map(Vec::len),
            Some(2),
            "the slot has no address, so `k` would refuse — which is not the \
             way to create no directory"
        );
    }

    /// A recorded run puts every slot's launch sources where a replay looks.
    ///
    /// A replay rebuilds each slot by reading its sources back out of the store,
    /// and a record naming bytes nobody kept is the same silence as no record at
    /// all.
    ///
    /// What the seeding was argued from has moved, and the seeding is left where it
    /// is rather than removed on this test's authority: the reader named was a
    /// rollback onto the launch version, which emitted `procedure` records naming
    /// these hashes, and ADR-0316 removed rollbacks. Whether a recorded run still
    /// owes the store its launch bytes is a decision about the record vocabulary
    /// and is the maintainer's; what this pins is that the seeding does what it
    /// says.
    ///
    /// Slot 1, deliberately. `session_head` writes slot 0's material and says out
    /// loud that a session stream cannot describe a deck, so slot 0 would pass on
    /// the head's I/O alone and prove nothing about the seeding. A `procedure`
    /// record, unlike a head, does address any slot.
    ///
    /// Paired with its control: nothing is in the store before the seeding, so this
    /// is about the seeding rather than about a store that had the bytes from
    /// somewhere else.
    #[test]
    fn a_recorded_run_puts_its_launch_sources_where_a_replay_looks() {
        let dir = tempfile::tempdir().expect("tempdir");
        let at_start = example("drift_shell.kir");
        let renderer = example("soft_points.kir");
        // Slot 1's geometry is a different procedure, so the two slots are two
        // addresses rather than one shared by a content-addressed store.
        let other = at_start.replace("proc drift_shell", "proc slot_one_shell");
        let first = vec![kir(&dir, "a.kir", &at_start), kir(&dir, "r.kir", &renderer)];
        let second = vec![kir(&dir, "b.kir", &other), kir(&dir, "s.kir", &renderer)];
        let (_m0, slot0) = slot(&first);
        let (_m1, slot1) = slot(&second);
        let placed = vec![slot0, slot1];

        let root = dir.path().join("store");
        let store = karakuri_store::store::Store::open(&root).expect("store");
        // **The control.** Opening a store does not fill it, so everything
        // below is about the seeding.
        for nodes in &placed {
            for node in nodes {
                assert!(
                    store.get_artifact(&node.hash()).is_err(),
                    "the store already held this run's sources, so the seeding \
                     below could not be what put them there"
                );
            }
        }

        seed_store_for_replay(&store, &placed);

        // **Slot 1's launch addresses, which are what it is recorded as running
        // until something rebuilds it.**
        let running = Running::at_launch(&placed, 2);
        let at_launch = running
            .playing(1)
            .expect("slot 1 launched with files behind it")
            .clone();
        for (_, _, hash) in &at_launch {
            assert!(
                store.get_artifact(hash).is_ok(),
                "a record naming slot 1's material names a source this session's \
                 replay cannot resolve, so the replay rebuilds nothing where the \
                 run showed the version it launched with"
            );
        }
    }

    /// Every `Kind` survives the round trip a saved node makes.
    ///
    /// A node's layer is written out by [`setfile::kind_name`] and read back by
    /// [`layer_named`], and nothing pinned them as inverses. A sixth `Kind` would
    /// have compiled — `kind_name`'s match is exhaustive and would be updated,
    /// `layer_named`'s ends in a wildcard and would not — and surfaced as `a node
    /// on layer ... cannot be saved` at the moment an operator pressed `k`, which
    /// is the worst place in the program to find out.
    ///
    /// The `match` is what makes this exhaustive, not the array. A list can fall
    /// one short in silence; a seventh variant stops this file compiling, which is
    /// the failure that was wanted in place of the runtime one. The array is
    /// `Kind::ALL` so that the walk cannot fall short either — the sixth kind is
    /// exactly the case where a hand-written list and the match beside it would
    /// have disagreed.
    #[test]
    fn every_kind_survives_the_round_trip_a_saved_node_makes() {
        use karakuri_ir::Kind;
        for kind in Kind::ALL {
            match kind {
                Kind::L1 | Kind::L2 | Kind::L3 | Kind::L4 | Kind::Field | Kind::L5 => {}
            }
            assert_eq!(
                layer_named(setfile::kind_name(kind)),
                Some(kind),
                "a node on this layer is written into a Set file under a name \
                 nothing reads back, so saving it refuses at the key press"
            );
        }
    }

    /// A slot with nothing in the store behind it saves nothing, rather than
    /// writing a file describing no Set.
    ///
    /// The `--load-set` without `--watch` case: the material came out of the store
    /// by hash with no files behind it, and `placed` is empty for that slot. There
    /// is no version to name, which is a different thing from naming the wrong one,
    /// and `Live::save_set` refuses it by name — see the refusal there, which says
    /// which set the run came from and what would make the slot savable.
    #[test]
    fn a_slot_with_no_sources_of_its_own_has_nothing_to_save() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path().join("store");
        let running = Running::at_launch(&[Vec::new()], 1);
        assert!(
            live_sources(running.playing(0), &[]).is_empty(),
            "a slot with no files behind it has no sources to write"
        );
        assert!(
            !root.exists(),
            "a deck with no files behind it opened a store to write nothing into"
        );
    }

    /// The two refusals a save can meet name the way out of them.
    ///
    /// Both reach a model now as well as a terminal, which is what makes them worth
    /// a test: `Live::save_set` needs a window and a GPU, so the sentences live in
    /// free functions and this is where they are checked. The `--mcp` half is a
    /// correction that has already been made once — this refusal named `--watch`
    /// alone for as long as `--mcp` also made a slot savable, and sent operators to
    /// restart a set for a flag they did not need.
    #[test]
    fn a_save_that_cannot_happen_says_why_and_what_would_change_it() {
        let loaded = nothing_to_save(2, Some("night01"), true);
        assert!(loaded.contains("`night01`"), "{loaded}");
        assert!(loaded.contains("--watch"), "{loaded}");
        assert!(
            loaded.contains("--mcp"),
            "a slot `--mcp` alone would have made savable was told to restart for \
             `--watch`: {loaded}"
        );

        // A slot that has files behind it and nothing in the store is a
        // different fact, and neither flag is the answer to it.
        let unstored = nothing_to_save(2, Some("night01"), false);
        assert!(
            !unstored.contains("night01"),
            "a slot with files of its own was told it came out of a set file: {unstored}"
        );

        // **The range, said without underflowing on a deck with no slots.** The
        // arm that cannot happen is the one that stops saying so quietly.
        assert_eq!(no_such_slot(9, 4), "no slot 9: this deck holds slots 0-3");
        assert!(no_such_slot(0, 0).contains("holds none"));
    }

    /// A renderer the slot does not draw with, refused in the shared words, and
    /// refused at the boundary rather than one past it.
    ///
    /// The sentence is pinned with an `assert_eq!` against [`no_such_renderer`]
    /// rather than a `contains`, which is the lesson [`no_such_slot`] paid for:
    /// four spellings of one refusal lived side by side because every test asked
    /// only whether the range appeared in it. Both surfaces that can meet this —
    /// the key press and a replayed `select` record — go through
    /// [`renderer_in_range`], so pinning it here pins both.
    #[test]
    fn a_renderer_a_slot_does_not_draw_with_is_refused_with_the_range_it_missed() {
        assert_eq!(
            renderer_in_range(1, 3, 3).expect_err("renderer 3 of three"),
            "no renderer 3: slot 1 draws with renderers 0-2"
        );
        // The boundary either side of it, which is where the off-by-one would
        // live: the last renderer is `count - 1` and it is legal.
        assert!(renderer_in_range(1, 2, 3).is_ok());
        assert!(renderer_in_range(0, 0, 1).is_ok());
        assert!(renderer_in_range(0, 1, 1).is_err());
        // And the arm that cannot happen, said without underflowing.
        assert_eq!(
            no_such_renderer(0, 0, 0),
            "no renderer 0: slot 0 draws with none"
        );
    }

    /// This file's own source, at compile time. The scanner below reads
    /// [`Live::key`] out of it rather than being told what the keys are — the shape
    /// `karakuri-engine/tests/gpu_tests_are_under_mod_gpu.rs` set: read the
    /// checked-in source, and carry a floor so the scan cannot silently match
    /// nothing.
    const SOURCE: &str = include_str!("live.rs");

    /// How [`BINDINGS`] spells the arms whose pattern is a name rather than a
    /// character. Nothing in `Key::Named(NamedKey::Escape)` says `esc`, so this one
    /// mapping cannot be derived and is stated — but it is stated as a *table*, and
    /// a `NamedKey` arm missing from it fails
    /// [`every_key_the_live_path_acts_on_is_documented`] by name rather than being
    /// passed over. That is the whole difference from the list of characters that
    /// used to be here: a named key added tomorrow is a test failure that says
    /// which key, not a silence.
    const NAMED_KEY_SPELLINGS: &[(&str, &str)] = &[("Escape", "esc"), ("Space", "space")];

    /// The end of the character literal starting at `at`, or `None` if what is
    /// there is not one.
    ///
    /// A lifetime has to be told from a literal — `'a` is one, `'a'` and `'\n'` are
    /// the other — because getting it wrong lets a `'"'` open a string that
    /// swallows the rest of the file. `'\''` is why an escape cannot simply look
    /// for the next quote: the escaped quote *is* the next quote.
    fn char_literal_end(src: &str, at: usize) -> Option<usize> {
        let b = src.as_bytes();
        if b.get(at) != Some(&b'\'') {
            return None;
        }
        if b.get(at + 1) == Some(&b'\\') {
            // A two-byte escape — `\\`, `\'`, `\n`, `\0` — closes at `at + 3`.
            if b.get(at + 3) == Some(&b'\'') {
                return Some(at + 4);
            }
            // `\x41`, `\u{2026}`: the first quote after the escape. Bounds
            // first, so a file ending mid-literal is `None` and not a panic.
            if at + 3 > src.len() {
                return None;
            }
            return src[at + 3..].find('\'').map(|n| at + 3 + n + 1);
        }
        let c = src[at + 1..].chars().next()?;
        let close = at + 1 + c.len_utf8();
        (b.get(close) == Some(&b'\'')).then_some(close + 1)
    }

    /// The character a literal's inside spells, the way rustc reads it.
    ///
    /// Panics rather than returning nothing on a spelling it does not know: a key
    /// quietly dropped here is a key quietly undocumented, which is the exact
    /// failure this file is closing.
    fn unescape(lit: &str) -> char {
        let mut cs = lit.chars();
        let first = cs.next().expect("a character literal is not empty");
        if first != '\\' {
            return first;
        }
        match cs.next().expect("an escape has a second character") {
            'n' => '\n',
            'r' => '\r',
            't' => '\t',
            '0' => '\0',
            c @ ('\\' | '\'' | '"') => c,
            _ => panic!("`'{lit}'` is a key spelling this scanner cannot read"),
        }
    }

    /// Comments and string literals blanked to spaces, keeping length and line
    /// structure. Character literals are left exactly as written — they are the
    /// payload.
    ///
    /// Load-bearing in both directions. Comments are prose and prose is full of
    /// apostrophes; string literals hold `"{BINDINGS}"` and every message the arms
    /// print. Cut down from the same function in
    /// `karakuri-engine/tests/gpu_tests_are_under_mod_gpu.rs`; this file has no
    /// block comments and no raw strings, and if one arrives the floors below are
    /// what refuses the mis-scan.
    fn blank_comments_and_strings(src: &str) -> String {
        let b = src.as_bytes();
        let mut out = b.to_vec();
        let mut i = 0;
        while i < b.len() {
            if b[i..].starts_with(b"//") {
                let end = src[i..].find('\n').map_or(b.len(), |n| i + n);
                for c in out[i..end].iter_mut() {
                    *c = b' ';
                }
                i = end;
                continue;
            }
            if b[i] == b'"' {
                let start = i;
                i += 1;
                while i < b.len() {
                    match b[i] {
                        b'\\' => i += 2,
                        b'"' => {
                            i += 1;
                            break;
                        }
                        _ => i += 1,
                    }
                }
                let end = i.min(b.len());
                for c in out[start..end].iter_mut() {
                    if *c != b'\n' {
                        *c = b' ';
                    }
                }
                continue;
            }
            if let Some(end) = char_literal_end(src, i) {
                i = end;
                continue;
            }
            i += 1;
        }
        String::from_utf8(out).expect("blanking only ever writes spaces")
    }

    /// [`Live::key`]'s body, blanked, found by its signature.
    ///
    /// The signature and not a line number, and not the name alone — `key` is a
    /// common word. It is spelled with `concat!` so the joined needle exists
    /// nowhere in this file except the function it names: a source scanner that
    /// finds itself is the classic way one of these comes back green. That it
    /// occurs exactly once is asserted, so a renamed or reformatted signature fails
    /// here instead of leaving the scan with nothing to read.
    fn live_key_body() -> String {
        let blanked = blank_comments_and_strings(SOURCE);
        let needle = concat!("fn ", "key(&mut self, key: &Key) -> bool {");
        assert_eq!(
            blanked.matches(needle).count(),
            1,
            "`{needle}` is not in this file exactly once — the scanner is \
             reading nothing, or reading the wrong function"
        );
        let open = blanked.find(needle).expect("just counted one") + needle.len();
        let b = blanked.as_bytes();
        let (mut i, mut depth) = (open, 1usize);
        let end = loop {
            if i >= b.len() {
                panic!("`{needle}` never closes");
            }
            // A braced character literal is a key like any other: stepped over
            // whole, so binding `{` could not open a block here.
            if let Some(skip) = char_literal_end(&blanked, i) {
                i = skip;
                continue;
            }
            match b[i] {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        break i;
                    }
                }
                _ => {}
            }
            i += 1;
        };
        blanked[open..end].to_string()
    }

    /// One arm of [`Live::key`]'s match, as its pattern reads.
    enum Arm {
        /// A character it acts on: `'x' => …`.
        Char(char),
        /// A span of them: `'0'..='3' => …`.
        Range(char, char),
        /// A `NamedKey`, whose pattern is a name and not a character at all.
        Named(String),
    }

    /// Every arm of [`Live::key`], read off the pattern side of each `=>` in its
    /// body.
    ///
    /// The pattern side and not the line, because everything after the first `=>`
    /// is the arm's *body* — where `unwrap_or('\0')` lives, and `\0` is not a
    /// binding. Line by line, because a pattern and its `=>` share a line; an arm
    /// body that grew an `=>` of its own would be read as a pattern, which can only
    /// ever demand documentation for a key nobody binds, and that fails loudly
    /// rather than passing quietly.
    fn live_key_arms(body: &str) -> Vec<Arm> {
        let mut arms = Vec::new();
        for line in body.lines() {
            let Some(cut) = line.find("=>") else {
                continue;
            };
            let pattern = &line[..cut];
            for (at, marker) in pattern.match_indices("NamedKey::") {
                let name: String = pattern[at + marker.len()..]
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                if !name.is_empty() {
                    arms.push(Arm::Named(name));
                }
            }
            let mut chars = Vec::new();
            let mut i = 0;
            while i < pattern.len() {
                match char_literal_end(pattern, i) {
                    Some(end) => {
                        chars.push(unescape(&pattern[i + 1..end - 1]));
                        i = end;
                    }
                    None => i += 1,
                }
            }
            // `'0'..='3'` is one binding spanning four keys, not two bindings.
            if pattern.contains("..=") {
                assert_eq!(
                    chars.len(),
                    2,
                    "`{pattern}` is a range with {} endpoints",
                    chars.len()
                );
                arms.push(Arm::Range(chars[0], chars[1]));
            } else {
                arms.extend(chars.into_iter().map(Arm::Char));
            }
        }
        arms
    }

    /// The key column of [`BINDINGS`]: two spaces, the keys, then the gap before
    /// the description. Keys are documented in pairs where they come in pairs — `[
    /// ]`, `u i`, `h ?` — so it is the column that is read and not the first
    /// character of a line.
    ///
    /// A column and never a substring of the whole constant, which is the trap this
    /// text is laid out to defuse: `?` occurs in the prose of the `, .` line, so
    /// `BINDINGS.contains("?")` is true whether or not `?` is bound to anything.
    /// See [`a_key_named_only_in_prose_is_not_documented`].
    fn documented_keys(bindings: &str) -> Vec<&str> {
        bindings
            .lines()
            .filter_map(|line| line.strip_prefix("  "))
            .filter(|line| !line.starts_with(' '))
            .flat_map(|line| line.split("  ").next().unwrap_or_default().split(' '))
            .filter(|key| !key.is_empty())
            .collect()
    }

    /// Every key `Live::key` acts on is in [`BINDINGS`], which is the only thing
    /// standing between a control and being undiscoverable: there is no on-screen
    /// UI, and this text is both what `--help` prints and what `h` does.
    ///
    /// The keys are *read out of the match arms* — see [`live_key_body`] — and not
    /// restated here. The version of this test that restated them iterated a
    /// hard-coded array of 32 characters, so what it enforced was "these 32 keys
    /// are documented"; `'h' | '?'` had been a live arm with no entry in the column
    /// the whole time and this test passed on every run. A check weaker than its
    /// own name is worse than no check, because the name is what stops anyone
    /// looking again — nobody re-reads a green
    /// `every_key_the_live_path_acts_on_is_documented`.
    ///
    /// Which is also why the counts are asserted. A scanner whose pattern stops
    /// matching finds nothing and then passes everything, and that is the same
    /// failure a second time.
    #[test]
    fn every_key_the_live_path_acts_on_is_documented() {
        let documented = documented_keys(BINDINGS);
        let (mut chars, mut ranges, mut named) = (0, 0, 0);

        for arm in live_key_arms(&live_key_body()) {
            match arm {
                Arm::Char(c) => {
                    chars += 1;
                    let key = c.to_string();
                    assert!(
                        documented.contains(&key.as_str()),
                        "`{c}` is a key `Live::key` acts on and has no entry in the \
                         bindings — an operator has no way to find it"
                    );
                }
                // Documented as the span it is, `0-3`, and the span is built
                // from the arm's own endpoints rather than being spelled here.
                Arm::Range(from, to) => {
                    ranges += 1;
                    let key = format!("{from}-{to}");
                    assert!(
                        documented.contains(&key.as_str()),
                        "`{key}` is a range `Live::key` acts on and has no entry in \
                         the bindings"
                    );
                }
                // A named arm carries no character to look for, so its spelling
                // comes from the one table there is — and an unknown name stops
                // the run rather than being skipped.
                Arm::Named(name) => {
                    named += 1;
                    let (_, spelling) = NAMED_KEY_SPELLINGS
                        .iter()
                        .find(|(known, _)| *known == name)
                        .unwrap_or_else(|| {
                            panic!(
                                "`NamedKey::{name}` is a key `Live::key` acts on and \
                                 NAMED_KEY_SPELLINGS does not say how the bindings \
                                 spell it"
                            )
                        });
                    assert!(
                        documented.contains(spelling),
                        "`{spelling}` (`NamedKey::{name}`) has no entry in the bindings"
                    );
                }
            }
        }

        // Floors, not counts. They are what `Live::key` actually holds today —
        // 32 characters, one range, two named keys — rather than a round number
        // under them, because a control surface is small enough that losing one
        // key is news and the scan going quiet is the thing being guarded
        // against. Three of them because they fail apart: a signature change
        // gives no arms at all, a broken literal reader gives named arms and no
        // characters, and a `NamedKey` renamed away gives characters and no
        // named ones. Raise them when a key is added; lowering one is a claim
        // that a control was deliberately removed.
        //
        // **It was 33 and is 32**, and that is the claim being made: ADR-0240
        // retired *Choose what the output shows*, so `v` is not a key any
        // more. The floor came down with the control rather than the control
        // being kept alive to hold a number up.
        assert!(
            chars >= 32,
            "only {chars} character keys read out of `Live::key` — the scan is not \
             seeing the match arms"
        );
        assert!(
            ranges >= 1,
            "no `'a'..='b'` arm read out of `Live::key` — slot focus is one"
        );
        assert!(
            named >= 2,
            "only {named} `NamedKey` arms read out of `Live::key` — escape and space \
             are two"
        );
    }

    /// A character that occurs only in a binding's prose is not documented.
    ///
    /// The reason [`documented_keys`] parses a column instead of asking
    /// `BINDINGS.contains(key)`, and it is not hypothetical: `?` appears inside the
    /// `, .` entry's description, so the substring form of this check would have
    /// called `?` documented while it was bound to nothing. That version would have
    /// looked stronger than the hard-coded array it replaced and enforced less.
    #[test]
    fn a_key_named_only_in_prose_is_not_documented() {
        // `?` twice in prose and never in the column: once in the description
        // beside a key, once on the continuation line under it. Both are places
        // the real text puts it, and each one is a different way the parse
        // could go wrong.
        let prose = concat!(
            "keys:\n",
            "  s          print the status line — the tracker writes ? there when\n",
            "             it is unsure, and ? again if it stays unsure\n",
        );
        // What a substring check sees, and it is a lie.
        assert!(prose.contains('?'));
        assert!(
            !documented_keys(prose).contains(&"?"),
            "a character in a description was counted as a documented key"
        );
        // The column, where a real binding lives, still reads.
        assert!(documented_keys(prose).contains(&"s"));
        // And in the real text `?` is now in the column, not only the prose.
        assert!(documented_keys(BINDINGS).contains(&"?"));
    }

    /// Every method of [`Live`] that writes a record without going through
    /// [`Live::operate`], with the reason its operation cannot convert.
    ///
    /// This is the key handler's form of the single arm [`Live::run_surface`] keeps
    /// for `TapBeat`: a table rather than a comment, so a key that grows a second
    /// derivation of a record is a failing test naming the function instead of a
    /// line nobody reads. Every operation named here answers `Owed::NotSettled`,
    /// which is asserted separately — see
    /// [`the_keys_that_keep_their_own_path_are_the_ones_whose_record_is_not_settled`]
    /// — so an entry that stops being owed fails there rather than lingering here
    /// as a stale excuse.
    const OWED_RECORD_PATHS: &[(&str, &str)] = &[(
        "performed",
        "the route itself: this is where `written`'s records are written, and \
         `Live::operate` is the wrapper over it for a caller with nobody waiting on an \
         answer",
    )];

    /// The method a byte offset falls inside, read off the nearest `fn` above it at
    /// `impl` indentation.
    ///
    /// Four spaces and not any `fn `, because a closure or a nested helper would
    /// otherwise answer for the method it sits in. The needle is spelled with a
    /// leading newline so a `fn` inside an expression cannot match.
    fn enclosing_method(blanked: &str, at: usize) -> &str {
        let start = blanked[..at]
            .rfind("\n    fn ")
            .expect("every record written in this file is inside a method")
            + "\n    fn ".len();
        let name_end = blanked[start..]
            .find(|c: char| !c.is_alphanumeric() && c != '_')
            .map_or(blanked.len(), |n| start + n);
        &blanked[start..name_end]
    }

    /// A key reaches the deck through [`Live::operate`], or it is one of the owed
    /// ones and says why.
    ///
    /// The route `karakuri-midi` took is *operation → `written` → record*, and the
    /// reason it could take it whole is that every operation a map line produces
    /// converts. A key handler cannot: three of its operations owe a record nobody
    /// can write yet, and routing one of those through `operate` would print the
    /// gap where the gesture used to be. It was seven, and four have left —
    /// `SetSync`, then `FadeDeck`, `Crossfade` and `SelectRenderer` together when
    /// the transition settings became a reading — each a row deleted here rather
    /// than kept, which is what the last loop below is for. So the ones that keep
    /// their own path are named here rather than left to be noticed, and this is
    /// what stops the list growing by accident — a record built beside the
    /// conversion is exactly the drift `karakuri-operation-record` exists to end,
    /// and `mix::gain_record`, `mix::opacity_record` and their neighbours went one
    /// at a time as each operation landed.
    ///
    /// Read out of the checked-in source, in the shape
    /// [`every_key_the_live_path_acts_on_is_documented`] set, with a floor so a
    /// scan that stops matching fails instead of passing everything.
    #[test]
    fn every_record_written_outside_operate_is_a_path_whose_conversion_is_owed() {
        let blanked = blank_comments_and_strings(SOURCE);
        let needle = concat!("self.", "record(");
        let mut reached: Vec<&str> = Vec::new();
        for (at, _) in blanked.match_indices(needle) {
            let name = enclosing_method(&blanked, at);
            assert!(
                OWED_RECORD_PATHS.iter().any(|(known, _)| *known == name),
                "`Live::{name}` writes a record without going through `Live::operate`, \
                 and OWED_RECORD_PATHS does not say why its operation cannot convert — \
                 a surface that derives a record beside the conversion is the drift \
                 `karakuri-operation-record` exists to end"
            );
            if !reached.contains(&name) {
                reached.push(name);
            }
        }
        // A floor rather than a count, and a low one: what is guarded against
        // is the scan going quiet, which would let every direct write through.
        // **One, where it was five, then four, then two.** `cycle_sync` was
        // the fifth; `fade_slot` and `cycle_renderer` were the third and
        // fourth and went together when the transition settings became a
        // reading; `wipe` was the second and went when the front shape joined
        // them. What is left is `operate` itself, which is the route rather
        // than a path around it — so this floor is now as low as it can go,
        // and the loop below is what actually keeps the table honest. A floor
        // above what is left would fail as a dead scan on the day a row was
        // correctly deleted — which is the one failure a guard against a dead
        // scan must not invent.
        assert!(
            !reached.is_empty(),
            "no method reads as a record writer — the scan is not seeing `Live`'s \
             bodies, and `Live::operate` itself is one: {reached:?}"
        );
        // And nothing in the table is a leftover. A path whose last direct
        // write moved to `operate` is a row to delete, not a permission to
        // keep.
        for (name, why) in OWED_RECORD_PATHS {
            assert!(
                reached.contains(name),
                "OWED_RECORD_PATHS excuses `Live::{name}` — {why} — and it writes no \
                 record of its own any more, so the row outlived what it was for"
            );
        }
    }

    /// The keys that keep their own path are exactly the ones whose record is not
    /// settled, and this is what will say so the day one changes.
    ///
    /// `b` and `, .` reach the beat tracker where they stand because `written`
    /// answers `Owed::NotSettled` for the operation each of them names. `y`, `f g`,
    /// `x`, `r` and `c` are the five that have already gone, and every one of them
    /// went the way this test names: `SetSync` stopped being owed when the
    /// conversion took a session tempo, `FadeDeck`, `Crossfade` and
    /// `SelectRenderer` stopped when it took the transition settings, and `Wipe`
    /// stopped when the front shape went over with them and the soft edge turned
    /// out to be the arriving deck's — so each key moved through [`Live::operate`]
    /// and its line here came out, taking `Live::fade_slot` and the last hand-built
    /// record with it. That is a statement about `karakuri-operation-record` rather
    /// than about this file, so it is checked against that crate: the day somebody
    /// settles one of these conversions, this fails and names the key that is now
    /// due to move through [`Live::operate`] — the promise `run_surface`'s
    /// `TapBeat` arm makes, kept by a test rather than by anyone remembering.
    #[test]
    fn the_keys_that_keep_their_own_path_are_the_ones_whose_record_is_not_settled() {
        use karakuri_operation_record::Owed;
        let owed = [
            ("b", Operation::TapBeat),
            (
                ", .",
                Operation::ScaleGrid {
                    by: karakuri_operation::GridScale::Double,
                },
            ),
        ];
        for (keys, operation) in owed {
            assert_eq!(
                karakuri_operation_record::written(&operation, &Current::default()),
                Written::Owed(Owed::NotSettled),
                "`{}` no longer owes its record, so `{keys}` reaches the deck by a \
                 derivation of its own where `Live::operate` would now do — see \
                 `Live::key`'s survey",
                operation.title()
            );
        }
    }

    /// A save still being written when the run ends is waited for, so the `save`
    /// record promised for every save that reached the disk
    /// (`docs/adr/0120-a-record-may-reach-outside-the-stream.md`) is in the stream.
    ///
    /// The window is small and it is exactly the one an operator is in: press `k`,
    /// read that it took, quit. The wait is bounded — see [`SAVE_WAIT`] — and the
    /// bound is what the second half of this checks: a save that never reports back
    /// costs the quit the bound and no more.
    #[test]
    fn a_save_in_flight_at_the_end_of_a_run_is_waited_for() {
        let (tx, rx) = std::sync::mpsc::channel();
        let late = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(120));
            tx.send(Saved {
                slot: 0,
                asked: Asked::Operator,
                id: "late".to_string(),
                outcome: Ok(()),
                // A hand pressed the key; nobody is waiting on a socket.
                reply: None,
            })
        });
        let landed = drained_saves(&rx, 1, Instant::now() + SAVE_WAIT);
        late.join().expect("the save thread").expect("it sent");
        assert_eq!(
            landed.len(),
            1,
            "the run quit before the save it was told to wait for reported back"
        );

        // **The bound, held against a save that never arrives.** The sender is
        // still alive, so there is nothing but the deadline to end this.
        let (_alive, rx) = std::sync::mpsc::channel::<Saved>();
        let started = Instant::now();
        let landed = drained_saves(&rx, 1, started + Duration::from_millis(80));
        assert!(landed.is_empty());
        assert!(
            started.elapsed() < Duration::from_secs(1),
            "a save that never reports back held the quit past its deadline"
        );
    }

    /// Both halves of the save path sit above everything in [`Live::frame`] that
    /// could stop them, which is the whole of what makes an answer to a waiting
    /// client independent of there being a swapchain.
    ///
    /// [`Live::run_requests`] argues this for itself: a client asking to keep what
    /// is playing should not be waiting on a surface. The *outcome* drain used to
    /// sit below `frame::compose`, past three returns, so the argument was made and
    /// then half applied. On a window latched to `frame::Skip::Fault`, or returning
    /// `Outdated` every frame, the request was taken and the save thread wrote the
    /// file successfully — and the client waited out `mcp::SAVE_REPLY` to be told
    /// the outcome was neither success nor failure about a save already on disk,
    /// the terminal never said "saved as set X", and the `save` record promised for
    /// every save that reached the disk was withheld from the stream until the run
    /// quit.
    ///
    /// This used to demand an early return and assert the calls were above it.
    /// There is no longer one: a sink with no target stopped being a reason to
    /// leave the function when a frame stopped belonging to one sink — see
    /// `frame`'s module documentation — so a frame that publishes nowhere now runs
    /// to the end. Demanding a `return` would make this test fail for the reason
    /// the defect it guards was fixed, which is the wrong question. What it asks
    /// instead is the property that outlives either shape: the two calls come above
    /// `frame::compose`, which is the GPU work and everything that has ever been a
    /// reason to bail out, and above the first `return` if one ever comes back. The
    /// second half is dormant today and is the half that matters on the day it is
    /// not.
    ///
    /// Read off the source, and that is the honest description of what this can
    /// reach. `Live::frame` needs a window, a GPU and an event loop, and the two
    /// replay defects this project found both lived in this one function's
    /// statement order
    /// (`docs/adr/0078-a-frame-that-is-discarded-must-not-already-have-been-recorded.md`).
    /// The defect here is a statement order too, and this asserts it where it is,
    /// rather than asserting nothing and calling it untestable. Comment lines are
    /// dropped first, so prose about returning cannot stand in for a `return`.
    #[test]
    fn a_frame_attends_to_its_saves_before_anything_can_stop_them() {
        let source = include_str!("live.rs");
        let body = source
            .split_once("\n    pub(crate) fn frame(&mut self) {")
            .or_else(|| source.split_once("\n    fn frame(&mut self) {"))
            .expect("`Live::frame` is no longer spelled that way")
            .1;
        let body = body
            .split("\n    fn ")
            .next()
            .expect("the end of `Live::frame`");
        let code: Vec<&str> = body
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect();
        let code = code.join("\n");

        // The end of the function if it never returns early, which is what it
        // does today — every position below is genuinely above it either way.
        let first_return = code.find("return").unwrap_or(code.len());
        let composed = code
            .find("frame::compose(")
            .expect("`Live::frame` no longer composes a frame, and this test is about where");
        for call in ["self.run_requests();", "self.finished_saves();"] {
            let at = code
                .find(call)
                .unwrap_or_else(|| panic!("`Live::frame` no longer calls `{call}`"));
            assert!(
                at < composed,
                "`{call}` is below `frame::compose` in `Live::frame`: a save's outcome has \
                 nothing to do with whether there is a surface to draw on"
            );
            assert!(
                at < first_return,
                "`{call}` is below an early return in `Live::frame`: a frame that publishes \
                 nowhere takes saves and never answers for them"
            );
        }
    }

    // The eight that build a Set to save from. A live save is read out of a running
    // deck, and there is no deck without a device — the ones above test the record
    // and the bookkeeping around what a slot is running, and take none.
    // See `karakuri-engine/tests/gpu_tests_are_under_mod_gpu.rs`.
    mod gpu {
        use super::*;

        /// One fold, so the card and the uniform cannot disagree.
        ///
        /// The number in a `param_decl` and the number the engine loads into a node's
        /// uniform are the same declaration read twice, and until this commit there was
        /// exactly one reader of it — private to `karakuri-engine`, with a note saying
        /// a second evaluator elsewhere would agree with the shader by coincidence. The
        /// metadata writer is that elsewhere. So the fold moved to
        /// `karakuri_ir::Param::default_scalar` and both call it, and this is what
        /// fails if either grows a reader of its own: a card claiming `0.0` where the
        /// run loaded `-0.35` describes a procedure nobody ran, and nothing downstream
        /// could say which of the two was wrong.
        ///
        /// Both directions. Every default the card states must be the value the built
        /// Set is running, *and* every value the Set is running must be stated — one of
        /// those alone passes when a writer silently drops the declaration it cannot
        /// fold.
        ///
        /// The Set is built with no overrides and no Set file, so what a node holds is
        /// exactly what its `.kir` declared.
        #[test]
        fn the_engine_and_the_metadata_writer_cannot_disagree_about_a_default() {
            let gpu = Gpu::headless().expect("no GPU");
            let dir = tempfile::tempdir().expect("tempdir");
            let paths = vec![
                kir(&dir, "a.kir", &signed_l1()),
                kir(&dir, "r.kir", &example("soft_points.kir")),
            ];
            let (material, placed) = slot(&paths);
            let set = set_of(&gpu, &material, &[16384], &[3], &[], None);
            let store = karakuri_store::store::Store::open(dir.path().join("store"))
                .expect("a store opens");
            let hash = placed[0].put(&store).expect("the artifact is stored");

            let mut on_the_card: Vec<(String, f32)> = store
                .read_meta(&hash)
                .expect("a card beside the artifact")
                .iter()
                .filter_map(|l| match l.record() {
                    Record::ParamDecl {
                        key,
                        default: Some(v),
                        ..
                    } => Some((key.clone(), *v)),
                    _ => None,
                })
                .collect();
            let mut in_the_uniform: Vec<(String, f32)> = set
                .params()
                .filter(|(layer, index, ..)| *layer == karakuri_ir::Kind::L1 && *index == 0)
                .map(|(_, _, key, value)| (key.to_string(), value))
                .collect();
            on_the_card.sort_by(|a, b| a.0.cmp(&b.0));
            in_the_uniform.sort_by(|a, b| a.0.cmp(&b.0));

            assert_eq!(
                on_the_card, in_the_uniform,
                "the card and the uniform read the same declaration and came back with \
             different numbers, which means there are two folds again"
            );
            // Not a vacuous agreement: both have to have folded the negation. Two
            // readers that both dropped it would be equal and both wrong.
            assert!(
                on_the_card.contains(&("signed".to_string(), -0.35)),
                "neither reader folded the negation, so they agree about nothing: \
             {on_the_card:?}"
            );
        }
        /// A live save writes a file that loads back into the same material — and does
        /// it for a Set of *two* geometries, which is where the numbers stop being
        /// interchangeable.
        ///
        /// Two geometries at two different capacities and two different salts, because
        /// that is the shape a single number cannot describe: a saver reaching for
        /// `Set::capacity` gets the sum, which is neither geometry's, and the file it
        /// writes is refused for having one capacity where the Set has two.
        /// `Set::source_capacities` is the reading that is per geometry, which is what
        /// the record is.
        #[test]
        fn a_live_save_reads_back_into_the_material_it_was_taken_from() {
            let gpu = Gpu::headless().expect("no GPU");
            let dir = tempfile::tempdir().expect("tempdir");
            let l1 = example("drift_shell.kir");
            let paths = vec![
                kir(&dir, "a.kir", &l1),
                kir(
                    &dir,
                    "b.kir",
                    &l1.replace("proc drift_shell", "proc drift_two"),
                ),
                kir(&dir, "r.kir", &example("soft_points.kir")),
            ];
            let (material, placed) = slot(&paths);
            // Different from each other and from the declared default, so that a
            // file which recorded either the sum or the declaration is visibly
            // wrong rather than accidentally right.
            let capacities = [8192, 16384];
            let salts = [11, 22];
            let set = set_of(&gpu, &material, &capacities, &salts, &[], None);

            let root = dir.path().join("store");
            let running = launched(&placed);
            let loaded = save_and_load(
                &root,
                "live",
                &set,
                live_sources(running.playing(0), &placed),
            );

            assert_eq!(
                loaded
                    .l1s
                    .iter()
                    .map(|c| c.name.as_str())
                    .collect::<Vec<_>>(),
                ["drift_shell", "drift_two"],
                "the geometries came back as something else"
            );
            assert_eq!(
                loaded
                    .l4s
                    .iter()
                    .map(|c| c.name.as_str())
                    .collect::<Vec<_>>(),
                ["soft_points"]
            );
            assert_eq!(
                loaded.capacities,
                vec![Some(8192), Some(16384)],
                "each geometry's own capacity is what the file has to carry"
            );
            assert_eq!(
                loaded.salts,
                vec![Some(11), Some(22)],
                "each geometry's own salt is what the file has to carry"
            );
        }
        /// A save after a rebuild records the camera the slot was loaded with, and not
        /// the built-in orbit's defaults.
        ///
        /// This is the whole path and deliberately not a piece of it: a slot aimed by a
        /// `camera` record, the watcher a `--watch` run gives it, an edit to a file,
        /// the build worker, the swap — and then the same `playing_values` the `k` key
        /// reads through. Every link in it was correct on its own while the chain
        /// silently re-aimed the slot, because the one that was missing was the request
        /// in the middle: `Set::build_many` starts every Set from `Orbit::default()`,
        /// so the rebuilt Set was aimed at the defaults and the saver recorded exactly
        /// what it found. That is why the loss stopped being a wrong picture and became
        /// a file — the operator's next preset was written with a camera nobody had
        /// chosen.
        ///
        /// Driven through `HotSwap` rather than by calling the worker's code, for the
        /// reason [`save_and_load`] goes through [`Save::run`]: a rebuild assembled by
        /// hand here would be a second copy of the rebuild, and a test of a copy is a
        /// test of nothing. `begin_frame` is the only place a build is installed, and
        /// it is enough on its own — nothing here has to render.
        #[test]
        fn a_save_after_a_rebuild_records_the_camera_the_slot_was_loaded_with() {
            let gpu = Gpu::headless().expect("no GPU");
            let dir = tempfile::tempdir().expect("tempdir");
            let l1 = example("drift_shell.kir");
            let paths = vec![
                kir(&dir, "a.kir", &l1),
                kir(&dir, "r.kir", &example("soft_points.kir")),
            ];
            let (material, _placed) = slot(&paths);
            // Six numbers no default produces. A `camera` record spells two of them
            // and the loader fills the rest from `Orbit::default()`, so a Set file
            // cannot actually deliver these four — they are here because what is
            // under test is the *carrying*, and a value that differs in every field
            // says which fields were carried.
            let aimed = karakuri_engine::camera::Orbit {
                radius: 3.25,
                speed: 0.75,
                height: -1.5,
                fov_y: 0.9,
                near: 0.25,
                far: 250.0,
            };
            let capacity = 8192;
            let salts = [11];
            let set = set_of(&gpu, &material, &[capacity], &salts, &[], Some(aimed));

            // The watcher `build_deck` gives the slot, stating what the slot is
            // running at — the camera among it, from the one reading the Set above
            // was built with.
            let watcher = watch::Watch::new(
                0,
                Named::bare(paths[0].clone()),
                vec![Named::bare(paths[1].clone())],
                karakuri_engine::set::Layering::Overdraw,
                None,
                Some(capacity),
                salts[0],
                salts.to_vec(),
                aimed,
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
            );
            let mut swap = HotSwap::new(
                &gpu.device,
                &gpu.queue,
                set,
                DEFAULT_BUDGET_MS,
                Box::new(watcher),
            );

            // The edit an operator makes, in the one form that changes nothing
            // about the material: the watcher compares contents, so a comment is
            // enough to make this a save it wakes on — and it keeps the rebuilt Set
            // the same Set, so the only thing that can differ is what was carried.
            std::fs::write(&paths[0], format!("{l1}\n// an edit\n")).expect("edit the geometry");

            // Bounded, and generous: this covers two poll intervals of debouncing,
            // four compiler stages, two shader modules and a whole-capacity upload,
            // on whatever machine is running the suite.
            let deadline = Instant::now() + Duration::from_secs(30);
            let mut seen: Vec<String> = Vec::new();
            loop {
                swap.begin_frame(&gpu.device);
                let mut swapped = false;
                for event in swap.events() {
                    swapped |= matches!(event, Event::Swapped { .. });
                    seen.push(event.to_string());
                }
                // **Read at the swap and not a frame later.** The outgoing Set is
                // aimed correctly too, so anything that put it back would put the
                // right camera back and hide exactly the defect this is about.
                if swapped {
                    break;
                }
                assert!(
                    Instant::now() < deadline,
                    "the edit never rebuilt; saw {seen:?}"
                );
                std::thread::sleep(Duration::from_millis(5));
            }

            let recorded = playing_values(swap.set(), &[]).camera;
            let six = |o: &karakuri_engine::camera::Orbit| {
                (o.radius, o.speed, o.height, o.fov_y, o.near, o.far)
            };
            assert_eq!(
                six(&recorded),
                six(&aimed),
                "the save after a rebuild would have written a camera the operator never aimed"
            );
        }
        /// A save after a parameter moved records the moved value.
        ///
        /// The Set is built with `radius` at 1.0, which is what a `--param radius=1.0`
        /// would have put there, and then moved to 2.6 the way a record moves it
        /// mid-run. The file has to say 2.6: a writer holding its own copy of what the
        /// run was started with records a number nobody has seen, which is the failure
        /// `saving_capacities` was fixed over, arriving one surface further along.
        ///
        /// Addressed rather than wildcard, because that is what the Set holds — a value
        /// per node — and because it is the only form that can be checked against the
        /// node that has it.
        #[test]
        fn a_live_save_records_a_param_where_the_run_moved_it_to() {
            let gpu = Gpu::headless().expect("no GPU");
            let dir = tempfile::tempdir().expect("tempdir");
            let paths = vec![
                kir(&dir, "a.kir", &example("drift_shell.kir")),
                kir(&dir, "r.kir", &example("soft_points.kir")),
            ];
            let (material, placed) = slot(&paths);
            let flag = ParamWrite::everywhere("radius", 1.0);
            let mut set = set_of(
                &gpu,
                &material,
                &[4096],
                &[7],
                std::slice::from_ref(&flag),
                None,
            );
            assert_eq!(
                set.param("radius"),
                Some(1.0),
                "the run started at the flag"
            );

            // What a `param` record does mid-set, through the one entry point both
            // a record and a key press come through.
            set.write_param(&ParamWrite::everywhere("radius", 2.6))
                .expect("every node of a Set nobody has spoken for is manual");

            let root = dir.path().join("store");
            let running = launched(&placed);
            let loaded = save_and_load(
                &root,
                "moved",
                &set,
                live_sources(running.playing(0), &placed),
            );

            let radius: Vec<&ParamWrite> =
                loaded.params.iter().filter(|p| p.key == "radius").collect();
            assert_eq!(
                radius.len(),
                1,
                "one geometry declares `radius`, so one line records it: {radius:?}"
            );
            assert_eq!(
                radius[0].value, 2.6,
                "the file recorded the value the run was started with, not the one it \
             was playing"
            );
            assert_eq!(
                radius[0].at,
                Some((karakuri_ir::Kind::L1, 0)),
                "a param is recorded against the node that declares it"
            );
        }
        // **Two tests stood here and were deleted with the state they were
        // about** (ADR-0316). Both were a save taken after a rollback: a build
        // compiled, was refused for cost, and the engine put the previous Set
        // back — so the path held one version and the picture another, and a
        // save that read the path wrote down material nobody had seen. Nothing
        // is put back now, so in that case the path and the picture agree and
        // neither test could be set up. The rule they held — **a save records
        // the version in the slot, whatever the path holds** — is unchanged and
        // is what the two tests below assert, through the states that do still
        // separate the two: a run with no watcher, and an edit that arrives
        // between the compile and the first frame.

        /// A run with no watcher saves the version it is still drawing, however far the
        /// file underneath it has moved.
        ///
        /// Nothing picks a `.kir` up without `--watch`: an editor writing over the
        /// path, an `--mcp` write with no watcher behind it — which the program's
        /// `mcp.rs` says out loud when it takes one — and an edit that failed to
        /// compile all leave the same state, and it is a state that never resolves on
        /// its own. The disk moved and the picture did not.
        ///
        /// This is why the launch addresses are seeded for every windowed run and not
        /// only an editable one. There is no watcher here to hash anything later, so a
        /// run gated on `editable()` would have nothing to save from and would go on
        /// answering with the path. The bytes behind those addresses reach the store
        /// here, at the save — see
        /// [`a_windowed_run_creates_no_store_until_something_is_saved`] for the other
        /// half of that, which is that nothing reaches it before.
        #[test]
        fn a_run_with_no_watcher_saves_the_version_it_is_still_drawing() {
            let gpu = Gpu::headless().expect("no GPU");
            let dir = tempfile::tempdir().expect("tempdir");
            let at_start = example("drift_shell.kir");
            let paths = vec![
                kir(&dir, "a.kir", &at_start),
                kir(&dir, "r.kir", &example("soft_points.kir")),
            ];
            let (material, placed) = slot(&paths);
            let set = set_of(&gpu, &material, &[4096], &[7], &[], None);
            let root = dir.path().join("store");

            let running = launched(&placed);

            // Something else rewrote the file. Nothing in this run is watching it,
            // so no build is requested, nothing lands, and the deck goes on drawing
            // what it built at startup for as long as the run lasts.
            std::fs::write(
                &paths[0],
                at_start.replace("proc drift_shell", "proc edited_outside"),
            )
            .expect("the edit nothing picked up");

            let kept = save_and_load(
                &root,
                "kept",
                &set,
                live_sources(running.playing(0), &placed),
            );
            assert_eq!(
                kept.l1s[0].name, "drift_shell",
                "the save read the path and wrote down an edit this run never \
             compiled, let alone drew"
            );
        }
        /// A `.kir` rewritten between the compile and the first frame cannot reach a
        /// save.
        ///
        /// The window is real and it is not short: between `sort_slot` and the first
        /// frame sit the adapter request, the deck build, `measure_slots`, and the
        /// audio, MIDI, tempo and MCP server starts. The launch seeding used to
        /// `std::fs::read` each path again at the end of that, so anything rewriting a
        /// file in between moved the slot's address onto bytes the deck had never
        /// compiled. If the rewrite did not compile, no watcher ever corrected it — `k`
        /// then wrote a Set naming a procedure that had never been on screen, and the
        /// file did not load back at all.
        ///
        /// Distinct from
        /// [`a_run_with_no_watcher_saves_the_version_it_is_still_drawing`], which
        /// rewrites the file after the run is under way. This one rewrites it inside
        /// the startup sequence, which is the window a second read opens and carrying
        /// the bytes closes.
        ///
        /// Two assertions, the first crisp and the second end to end: the address the
        /// slot reports, and the file that comes back off the disk.
        #[test]
        fn a_rewrite_between_the_compile_and_the_first_frame_cannot_reach_a_save() {
            let gpu = Gpu::headless().expect("no GPU");
            let dir = tempfile::tempdir().expect("tempdir");
            let at_start = example("drift_shell.kir");
            let paths = vec![
                kir(&dir, "a.kir", &at_start),
                kir(&dir, "r.kir", &example("soft_points.kir")),
            ];
            // The compile. Everything the run says about these nodes from here on
            // is a function of the bytes this read.
            let (material, placed) = slot(&paths);
            let set = set_of(&gpu, &material, &[4096], &[7], &[], None);

            // Startup is not finished. Something rewrites the file — a formatter on
            // save, an editor, a model over MCP that connected as the server came
            // up — and this one does not compile, so nothing will ever correct it.
            std::fs::write(
                &paths[0],
                at_start.replace("proc drift_shell", "proc rewritten_between {{{"),
            )
            .expect("the rewrite inside the startup sequence");

            let root = dir.path().join("store");
            let running = launched(&placed);
            let sources = live_sources(running.playing(0), &placed);
            assert_eq!(
                sources.0[0].hash,
                karakuri_store::hash::Hash::of(at_start.as_bytes()),
                "the slot is addressed by bytes this run never compiled, so what it \
             saves is a version that was never on screen"
            );

            let kept = save_and_load(&root, "kept", &set, sources);
            assert_eq!(
                kept.l1s[0].name, "drift_shell",
                "the save wrote down the rewrite rather than the material the deck \
             was built from"
            );
        }
    }
}

/// What `wire_input` asks the render loop for, and what the loop does with it.
///
/// The three points [`mcp::WireRequest`] states are the three these defend:
/// replace keyed on the input, re-aim the slot so it rebuilds, and answer once
/// at the frame it was applied on. Everything below [`rewired`] needs is a list
/// of edges and a channel — no window, no GPU and no `Deck` — which is why that
/// function is not a method; the one test here that goes through the socket is
/// the one about the *answer*, because only a real client can be told anything.
#[cfg(test)]
mod wire_tests {
    use super::*;
    use std::io::{BufRead, Write};

    fn edge(node: &str, slot: &str, to: &str) -> karakuri_engine::set::Edge {
        karakuri_engine::set::Edge {
            node: node.to_string(),
            slot: slot.into(),
            to: to.to_string(),
        }
    }

    /// A watcher's worth of aim, with nothing in it that matters here except the
    /// edges: what these tests read off a re-point is the wiring it carries and
    /// whether one arrived at all.
    fn aimed(edges: Vec<karakuri_engine::set::Edge>) -> watch::Aim {
        watch::Aim {
            head: Named::bare("head.kir"),
            rest: Vec::new(),
            layering: karakuri_engine::set::Layering::Overdraw,
            live: None,
            capacity: None,
            seed_salt: 7,
            salts: vec![7],
            camera: karakuri_engine::camera::Orbit::default(),
            overrides: Vec::new(),
            published: Vec::new(),
            bindings: Vec::new(),
            edges,
            authorities: Vec::new(),
            set: None,
        }
    }

    /// One watched slot, and the end a re-point arrives on.
    fn watcher(
        edges: Vec<karakuri_engine::set::Edge>,
    ) -> (Aiming, std::sync::mpsc::Receiver<watch::Aim>) {
        let (aim, aimed_at) = std::sync::mpsc::channel();
        (
            Aiming {
                aim,
                at: aimed(edges),
            },
            aimed_at,
        )
    }

    /// An edge is replaced on the input it binds, and no other edge moves.
    ///
    /// The replacement is keyed on `(node, slot)` — the node that declares the
    /// input and what its procedure calls it — so a second `uses` on the *same
    /// node* is a different entry, and so is the same input name on a different
    /// node. A key that were only the node would silently unbind `morph.near` when
    /// `morph.far` was rewired; one that were only the input name would do it
    /// across nodes. Both are here, because both would build a Set nobody asked for
    /// and neither shows up on the call that did it.
    #[test]
    fn a_wire_replaces_the_edge_on_the_input_it_binds_and_leaves_every_other_alone() {
        let mut edges = vec![
            edge("morph", "far", "sphere_shell"),
            edge("morph", "near", "lattice"),
            edge("veil", "far", "torus"),
        ];
        let (aiming, aims) = watcher(edges.clone());
        let mut slots = vec![Some(aiming)];

        let said = rewired(
            &[(0, edge("morph", "far", "drift_shell"))],
            &mut edges,
            &mut slots,
            1,
        );

        assert_eq!(said.len(), 1);
        assert!(said[0].is_ok(), "{:?}", said[0]);
        assert_eq!(
            edges.len(),
            3,
            "the run is wired with {} edges after replacing one of three: {edges:?}",
            edges.len()
        );
        assert!(
            edges.contains(&edge("morph", "far", "drift_shell")),
            "the edge asked for is not in the run's wiring: {edges:?}"
        );
        assert!(
            edges.contains(&edge("morph", "near", "lattice")),
            "rewiring `morph.far` took `morph.near` with it: {edges:?}"
        );
        assert!(
            edges.contains(&edge("veil", "far", "torus")),
            "rewiring `morph.far` took `veil.far` with it — the key is the node and the \
             input, not the input alone: {edges:?}"
        );
        // And what the watcher was handed is the same list, not the one it
        // started with: a rebuild restates its own edges, so a re-aim that
        // carried the old wiring would put the old edge back on the next build.
        let sent = aims.try_recv().expect("the slot was not re-aimed");
        assert_eq!(sent.edges, edges, "the re-aim carried a different wiring");
    }

    /// A second wire on one input replaces rather than appends.
    ///
    /// `SetError::SlotBoundTwice` refuses two edges on one input where the Set is
    /// built, so an append would make the *second* call on an input a refusal — a
    /// model that changed its mind would have wired the slot into a state it cannot
    /// build and cannot leave. The two calls are separate frames here, which is the
    /// ordinary case; the same thing on one frame is the test below.
    #[test]
    fn a_second_wire_on_one_input_replaces_rather_than_appends() {
        let mut edges = vec![edge("morph", "far", "sphere_shell")];
        let (aiming, aims) = watcher(edges.clone());
        let mut slots = vec![Some(aiming)];

        for to in ["drift_shell", "lattice_shell"] {
            let said = rewired(&[(0, edge("morph", "far", to))], &mut edges, &mut slots, 1);
            assert!(said[0].is_ok(), "{:?}", said[0]);
        }

        assert_eq!(
            edges,
            vec![edge("morph", "far", "lattice_shell")],
            "one input is bound {} times: a Set built from this is refused for \
             `SlotBoundTwice` and the second call is where a model loses its way back",
            edges.len()
        );
        // Both frames re-aimed the slot, and the second one carries the second
        // edge — the first would leave the picture on the wiring the model
        // changed its mind about.
        let sent: Vec<watch::Aim> = aims.try_iter().collect();
        assert_eq!(
            sent.len(),
            2,
            "one frame's rewiring did not reach the watcher"
        );
        assert_eq!(sent[1].edges, vec![edge("morph", "far", "lattice_shell")]);
    }

    /// Two wires on one input on one frame: both are applied in order, the later
    /// one is what the run holds, and the earlier one is told so.
    ///
    /// One aim goes out per slot after every edge on the frame is in the list, so
    /// the rebuild carries the wiring the frame ended with rather than an
    /// intermediate one. And the reply to the overwritten request says it was
    /// overwritten: it did write its edge, so a refusal would be false, and a bare
    /// "wired" would be a true sentence about a state the run no longer held by the
    /// end of the frame it was sent on.
    #[test]
    fn two_wires_on_one_input_in_one_frame_leave_the_run_wired_with_the_later_one() {
        let mut edges = vec![edge("morph", "far", "sphere_shell")];
        let (aiming, aims) = watcher(edges.clone());
        let mut slots = vec![Some(aiming)];

        let said = rewired(
            &[
                (0, edge("morph", "far", "drift_shell")),
                (0, edge("morph", "far", "lattice_shell")),
            ],
            &mut edges,
            &mut slots,
            1,
        );

        assert_eq!(
            edges,
            vec![edge("morph", "far", "lattice_shell")],
            "the run is wired with something other than the last edge of the frame"
        );
        let first = said[0].as_ref().expect("the first was applied");
        assert!(
            first.contains("replaced it with `lattice_shell`"),
            "the overwritten request was told its edge stands: {first}"
        );
        assert!(
            said[1]
                .as_ref()
                .expect("the second was applied")
                .contains("recompiling"),
            "the edge the run kept was not reported as rebuilding: {:?}",
            said[1]
        );
        let sent: Vec<watch::Aim> = aims.try_iter().collect();
        assert_eq!(
            sent.len(),
            1,
            "two edges on one frame re-aimed the slot {} times: a frame rebuilds once, at \
             the wiring it ended with",
            sent.len()
        );
        assert_eq!(sent[0].edges, vec![edge("morph", "far", "lattice_shell")]);
    }

    /// A slot this deck does not hold is refused, and nothing is rewired.
    ///
    /// The MCP server checks the number against its `Slots` before it sends, so a
    /// model meets the refusal there — this is the guard that does not depend on
    /// the surface that asked having one, which is exactly what `Live::save_set`
    /// says about the same check. The run's wiring is the part worth asserting: a
    /// refusal that had already edited the list would leave the deck wired by a
    /// call it said it had refused.
    #[test]
    fn a_wire_naming_a_slot_this_deck_does_not_hold_is_refused_and_nothing_is_rewired() {
        let mut edges = vec![edge("morph", "far", "sphere_shell")];
        let (aiming, aims) = watcher(edges.clone());
        let mut slots = vec![Some(aiming)];

        // With one that *is* applied in front of it, on the same input: a
        // refusal writes nothing, so the request before it must not be told
        // that a later one replaced its edge.
        let said = rewired(
            &[
                (0, edge("morph", "far", "lattice_shell")),
                (4, edge("morph", "far", "drift_shell")),
            ],
            &mut edges,
            &mut slots,
            1,
        );

        let refusal = said[1]
            .as_ref()
            .expect_err("a slot 4 of a one-slot deck was accepted");
        assert!(refusal.contains("no slot 4"), "{refusal}");
        assert!(refusal.contains("nothing was rewired"), "{refusal}");
        assert_eq!(
            edges,
            vec![edge("morph", "far", "lattice_shell")],
            "a refused request edited the run's wiring anyway"
        );
        let kept = said[0].as_ref().expect("the slot 0 request was applied");
        assert!(
            !kept.contains("replaced it with"),
            "a request that was refused was reported as having replaced the edge in front \
             of it: {kept}"
        );
        let sent: Vec<watch::Aim> = aims.try_iter().collect();
        assert_eq!(
            sent.len(),
            1,
            "a refused request re-aimed a watcher, so a slot is rebuilding for a call that \
             was told nothing happened"
        );
        assert_eq!(sent[0].edges, vec![edge("morph", "far", "lattice_shell")]);
    }

    /// A slot with no watcher is not a failure, and the reply says what did not
    /// happen.
    ///
    /// A run without `--watch` has nothing that rebuilds. The edge is still the
    /// run's — a `save_set` records it — so a refusal would be false; and a bare
    /// "wired" would let a model wait for a picture that is never going to change.
    /// `mcp::wire_input` adds the same fact from its side, where it is the only
    /// thing that knows how the run was started.
    #[test]
    fn a_wire_on_a_slot_with_no_watcher_is_applied_and_says_nothing_rebuilds() {
        let mut edges = Vec::new();
        let mut slots: Vec<Option<Aiming>> = vec![None];

        let said = rewired(
            &[(0, edge("morph", "far", "drift_shell"))],
            &mut edges,
            &mut slots,
            1,
        );

        let line = said[0]
            .as_ref()
            .expect("a run without a watcher refused an edge");
        assert!(
            line.contains("no watcher") && line.contains("save_set"),
            "{line}"
        );
        assert_eq!(edges, vec![edge("morph", "far", "drift_shell")]);
    }

    /// The frame drains the edges, and not only the saves.
    ///
    /// This is the failure the whole surface was in when `wire_input` landed: the
    /// tool was finished, the channel was there, and the render loop answered none
    /// of it — so every call waited out `WIRE_REPLY` and came back with a true
    /// sentence saying nothing had been rewired. Everything else here tests what
    /// [`rewired`] decides; nothing else tests that a frame ever asks it.
    /// `Live::run_requests` needs a window and a GPU, so this is read off the
    /// source, which is
    /// `a_frame_attends_to_its_saves_before_anything_can_stop_them`'s own technique
    /// and for its reason: asserting where a statement is beats asserting nothing
    /// and calling it untestable. Comment lines are dropped first, so prose about
    /// draining cannot stand in for a drain.
    #[test]
    fn a_frame_takes_the_edges_a_client_asked_for_and_not_only_the_saves() {
        let source = include_str!("live.rs");
        let body = source
            .split_once("\n    fn run_requests(&mut self) {")
            .expect("`Live::run_requests` is no longer spelled that way")
            .1;
        let body = body
            .split("\n    }")
            .next()
            .expect("the end of `Live::run_requests`");
        let code: Vec<&str> = body
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect();
        let code = code.join("\n");
        for call in ["mcp.wires()", "self.rewire(wires)"] {
            assert!(
                code.contains(call),
                "`Live::run_requests` does not `{call}`: a `wire_input` call on this run \
                 waits out its deadline and is told the loop never took the edge"
            );
        }
    }

    // -- over the socket -------------------------------------------------

    /// One tool call, over TCP exactly as a client makes it. The answer is the
    /// thing being tested, so nothing here shares a channel with the server — it is
    /// the wire.
    fn call(port: u16, name: &str, args: serde_json::Value) -> (bool, String) {
        let body = serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": {"name": name, "arguments": args},
        })
        .to_string();
        let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).expect("connect");
        stream
            .set_read_timeout(Some(Duration::from_secs(20)))
            .expect("read timeout");
        stream
            .write_all(
                format!(
                    "POST / HTTP/1.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
                    body.len()
                )
                .as_bytes(),
            )
            .expect("write");
        let mut reader = std::io::BufReader::new(stream);
        let mut line = String::new();
        reader.read_line(&mut line).expect("status line");
        let mut length = 0usize;
        loop {
            let mut header = String::new();
            reader.read_line(&mut header).expect("header");
            let header = header.trim_end();
            if header.is_empty() {
                break;
            }
            if let Some((name, value)) = header.split_once(':') {
                if name.eq_ignore_ascii_case("content-length") {
                    length = value.trim().parse().unwrap_or(0);
                }
            }
        }
        let mut body = vec![0u8; length];
        std::io::Read::read_exact(&mut reader, &mut body).expect("body");
        let reply: serde_json::Value =
            serde_json::from_slice(&body).expect("the server answered something that is not JSON");
        let result = &reply["result"];
        (
            result["isError"].as_bool().unwrap_or(true),
            result["content"][0]["text"]
                .as_str()
                .unwrap_or("")
                .to_string(),
        )
    }

    /// A `wire_input` call is answered at the frame the edge was applied on, and
    /// the run is wired with it.
    ///
    /// End to end: the call goes over the socket, the request crosses
    /// `mcp::Reporter::wires`, a stand-in frame loop applies it with the same
    /// [`rewired`] the real one calls, and the answer comes back down the
    /// connection the model is holding open. Without the drain this is exactly the
    /// failure that was here: the tool waits out `WIRE_REPLY` and says *the render
    /// loop had not taken this edge*, which is true, is loud, and is not a
    /// rewiring.
    ///
    /// The frame loop is a thread rather than a `Live` because a `Live` needs a
    /// window and a GPU. What it stands in for is the drain and the answer, and
    /// those are the whole of `Live::rewire`.
    #[test]
    fn a_wire_request_is_answered_at_the_frame_it_was_applied_on() {
        let dir = tempfile::tempdir().expect("tempdir");
        let l1 = dir.path().join("l1.kir");
        std::fs::write(&l1, "proc probe { kind L1 }").expect("fixture");
        let reporter = mcp::serve(
            0,
            mcp::Slots::of(vec![(l1, Vec::new())]),
            dir.path().join("store"),
            true,
            karakuri_environment::Opening::closed(),
        )
        .expect("serve");
        let port = reporter.port();

        // What the run is wired with, shared so the test can read it back —
        // the render loop's own copy is `Live::edges` and nothing else holds
        // one.
        let edges = Arc::new(std::sync::Mutex::new(vec![edge(
            "morph",
            "far",
            "sphere_shell",
        )]));
        let (aiming, aims) = watcher(edges.lock().expect("fresh mutex").clone());
        let run = edges.clone();
        // The thread never ends, which is what keeps the reporter alive: a
        // dropped reporter is a run that has quit, and the tool has a
        // different true sentence for that.
        std::thread::spawn(move || {
            let mut slots = vec![Some(aiming)];
            loop {
                let asked: Vec<mcp::WireRequest> = reporter.wires().collect();
                if !asked.is_empty() {
                    let mut wires = Vec::new();
                    let mut replies = Vec::new();
                    for mcp::WireRequest { slot, edge, reply } in asked {
                        wires.push((slot, edge));
                        replies.push(reply);
                    }
                    let mut held = run.lock().expect("the run's wiring");
                    let said = rewired(&wires, &mut held, &mut slots, 1);
                    drop(held);
                    for (reply, said) in replies.into_iter().zip(said) {
                        reply.settled(said);
                    }
                }
                std::thread::sleep(Duration::from_millis(2));
            }
        });

        let (failed, said) = call(
            port,
            "wire_input",
            serde_json::json!({"slot": 0, "node": "morph", "input": "far", "to": "drift_shell"}),
        );
        assert!(!failed, "the call came back as a failure: {said}");
        assert!(
            said.contains("wired `morph.far=drift_shell`"),
            "the client was told something other than what the loop did: {said}"
        );
        assert_eq!(
            *edges.lock().expect("the run's wiring"),
            vec![edge("morph", "far", "drift_shell")],
            "the client was answered and the run is not wired with the edge"
        );
        assert_eq!(
            aims.try_recv().expect("the slot was not re-aimed").edges,
            vec![edge("morph", "far", "drift_shell")],
            "the edge was written and nothing was asked to rebuild with it"
        );
    }

    /// An `operate` naming an operation this program cannot perform comes back as a
    /// failure, and one it can perform comes back as `ok`.
    ///
    /// The two halves are one claim and are asserted together, because a refusal
    /// that refused everything would pass the first on its own. `Star a Set` is
    /// performed on the panel by `favourite` and by nothing here; `Gain` is the
    /// fader every surface has, and both cross the same drain.
    ///
    /// Why it matters more than a wrong line on a terminal: the client is a model,
    /// it reports what it is told to the person sitting there, and *"`Star a Set`
    /// was performed"* for a star that landed nowhere is the plausible wrong answer
    /// [P-0094](../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)
    /// is written against. ADR-0334 refused it for the panel in as many words and
    /// this surface answered `ok` all the same, which is the defect
    /// [ADR-0341](../../../docs/adr/0341-a-route-that-answers-is-built-and-a-send-that-ends-in-a-dialog-is-gap.md)
    /// left behind here.
    ///
    /// End to end: the call goes over the socket, the audit runs on the server's
    /// own thread, the request crosses `mcp::Reporter::operations`, and a stand-in
    /// frame loop answers it with the same [`answered`] and the same
    /// [`performed_at_the_frame`] [`Live::run_operations`] answers with. The loop
    /// is a thread rather than a `Live` for
    /// `a_wire_request_is_answered_at_the_frame_it_was_applied_on`'s reason — a
    /// `Live` needs a window and a GPU — and what it stands in for is the drain and
    /// the answer. `Live::run_operations`' one arm this does not carry is
    /// `Operation::TapBeat`, which is not a conversion and has its own performer.
    ///
    /// Watched to fail with `answered`'s `Silent` arm answering `Ok(Vec::new())`,
    /// which is what this program did before: the star comes back as a success
    /// carrying *was performed*, and nothing was starred.
    #[test]
    fn an_operation_this_program_has_no_control_for_is_refused_over_the_wire() {
        let dir = tempfile::tempdir().expect("tempdir");
        let l1 = dir.path().join("l1.kir");
        std::fs::write(&l1, "proc probe { kind L1 }").expect("fixture");
        // **One class open, and it is the fader's.** A `SetGain` the audit
        // refused would never reach the drain at all, and what is under test
        // here is the answer a call gets *after* the audit has passed it — the
        // star needs nothing opened, because `Standing::Open` is its whole
        // audit (ADR-0301, ADR-0341).
        let opening = karakuri_environment::Opening::closed();
        opening.set(
            karakuri_operation::gate::Open::CLOSED
                .with(karakuri_operation::gate::Class::MixFaders, true),
        );
        let reporter = mcp::serve(
            0,
            mcp::Slots::of(vec![(l1, Vec::new())]),
            dir.path().join("store"),
            true,
            opening,
        )
        .expect("serve");
        let port = reporter.port();

        // What the loop wrote, so that *nothing was performed* is asserted as
        // well as *the call failed*. The thread never ends, which is what keeps
        // the reporter alive: a dropped reporter is a run that has quit, and
        // the tool has a different true sentence for that.
        let written = Arc::new(std::sync::Mutex::new(Vec::new()));
        let taken = written.clone();
        std::thread::spawn(move || loop {
            for mcp::OperateRequest { operation, reply } in reporter.operations() {
                // `Current::default()` is every reading absent, which is what
                // this stand-in has: it holds no deck. Neither operation below
                // asks for one — a gain carries everything its record says.
                match answered(&operation, &Current::default()) {
                    Ok(records) => {
                        taken.lock().expect("what the loop wrote").extend(records);
                        reply.settled(Ok(performed_at_the_frame(operation.title())));
                    }
                    Err(said) => reply.settled(Err(said)),
                }
            }
            std::thread::sleep(Duration::from_millis(2));
        });

        let (failed, said) = call(
            port,
            "operate",
            serde_json::json!({
                "operation": "Star a Set, or take the star off",
                "with": {"set": "a_set", "favourite": true},
            }),
        );
        assert!(
            failed,
            "a star nothing on this surface performs was answered as a success: {said}"
        );
        assert!(
            said.contains("`Star a Set, or take the star off` was not performed"),
            "the refusal does not name the operation a model asked for: {said}"
        );
        assert!(
            said.contains("has no control for this one"),
            "the refusal does not say this program cannot perform it: {said}"
        );
        assert!(
            written.lock().expect("what the loop wrote").is_empty(),
            "the call was refused and the loop wrote a record anyway"
        );

        let (failed, said) = call(
            port,
            "operate",
            serde_json::json!({"operation": "Gain", "with": {"deck": 0, "gain": 0.8}}),
        );
        assert!(
            !failed,
            "a fader this surface does perform was refused with it: {said}"
        );
        assert!(
            said.contains("was performed on the frame it arrived on"),
            "the client was told something other than what the loop did: {said}"
        );
        assert_eq!(
            *written.lock().expect("what the loop wrote"),
            vec![Record::Gain {
                slot: DeckSlot(0),
                value: 0.8
            }],
            "the client was answered `ok` and the loop wrote something else"
        );
    }
}
