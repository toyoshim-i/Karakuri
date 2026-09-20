use super::*;
use karakuri_environment::compile::sort_compiled;
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

/// **A head written for a two-slot deck splits back into the material and the
/// deck**, and the material still loads clean.
///
/// The writer's own output through the reader's own rule. `session_head` writes
/// the head slot's Set file, `session::head` puts the deck's records after it,
/// and `session::split` is what `--replay` sorts the two with — so a head whose
/// deck records leaked into the material, or whose material leaked into the
/// deck, fails here rather than on the way back from a set.
///
/// What this cannot check is the *reading* of a live deck — `held_deck` takes a
/// `Deck` and a deck takes a device. The values below are written by hand for
/// that reason, and the far end of the claim is
/// `karakuri-cli/tests/replay.rs`'s two-slot head driven through the binary.
#[test]
fn a_two_slot_head_splits_into_the_material_and_the_deck() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let dir = tempfile::tempdir().expect("tempdir");
    let store = karakuri_store::store::Store::open(dir.path()).expect("store");

    let mut args = parse(&[]).expect("parses");
    args.sets = vec![(
        Named::bare(root.join("examples/coil_vortex.kir")),
        vec![Named::bare(root.join("examples/star_flares.kir"))],
    )];
    args.store = dir.path().to_path_buf();
    let (material, placed) = sort_slot(0, &args.sets[0].0, &args.sets[0].1);
    // The same material in the second slot, which is what this program's own
    // deck holds when it is given one `--set`: four slots of one pair.
    let nodes: Vec<_> = placed
        .iter()
        .map(|node| {
            (
                karakuri_environment::meta::layer_of(node.layer),
                node.index,
                node.hash(),
            )
        })
        .collect();

    let head = session::head(
        session_head(&args, &[placed], &material.l1s, &store, "two"),
        &session::Held {
            canvas: (640, 360),
            look: args.look,
            master_out: 1.0,
            master_chain: Vec::new(),
            slots: (0..2)
                .map(|_| session::SlotHeld {
                    nodes: nodes.clone(),
                    gain: 1.0,
                    opacity: 1.0,
                    blend: karakuri_engine::deck::Blend::Add,
                    residency: karakuri_engine::deck::Residency::Live,
                    policy: karakuri_operation::SlotPolicy::Auto,
                    mask: karakuri_engine::deck::Mask::default(),
                    transport: karakuri_engine::transport::Transport::default(),
                })
                .collect(),
        },
    );

    let stream = session::split(head);
    // The material loads the way `--replay` loads it, with nothing of the deck's
    // among it: a `gain` reaching `from_lines` comes back as a note.
    let loaded = setfile::from_lines(&store, "two", &stream.head)
        .expect("the head a recording writes is a head a replay can load");
    assert!(loaded.notes.is_empty(), "{:?}", loaded.notes);
    assert_eq!(loaded.l1s[0].kind, karakuri_ir::Kind::L1);

    let named: Vec<usize> = stream
        .opening
        .iter()
        .filter_map(|record| match record {
            karakuri_store::record::Record::Procedure { slot, .. } => Some(slot.index()),
            _ => None,
        })
        .collect();
    assert_eq!(
        named,
        vec![1, 1],
        "the second slot's two nodes, and none for the first — whose Set file is the \
         material above"
    );
    assert!(stream.frames.is_empty(), "a head is not a frame");
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
