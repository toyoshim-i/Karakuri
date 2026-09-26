use super::*;

/// Verifies refused wipe diagnostics explain why the operation was blocked and what must change (P-0083).
#[test]
fn a_refused_wipe_says_which_refusal_it_was_and_where_to_go_next() {
    let no_shape = refusal(&Go::NoShape, 4);
    assert!(
        no_shape.contains("shape"),
        "the refusal for an unchosen shape does not say what is missing: `{no_shape}`"
    );
    assert!(
        no_shape.contains("pill"),
        "the refusal for an unchosen shape does not say where one is picked, so an \
         operator is told no and not told where to go: `{no_shape}`"
    );

    let alone = refusal(&Go::NoOtherDeck, 1);
    assert!(
        alone.contains('1') && alone.contains("strip"),
        "the refusal for a mixer with nowhere to wipe from does not carry the count \
         that is the constraint: `{alone}`"
    );
    assert!(
        alone.contains("deck"),
        "the refusal for a mixer with nowhere to wipe from does not say what would \
         have to change: `{alone}`"
    );
    // The plural moves with the count, which is this file's rule for every
    // sentence that carries one.
    assert!(refusal(&Go::NoOtherDeck, 0).contains("0 strips"));
    assert!(refusal(&Go::NoOtherDeck, 1).contains("1 strip,"));

    assert_ne!(
        no_shape, alone,
        "a wipe with no shape chosen and a wipe with nowhere to come from came out of \
         this window as the same sentence"
    );
}

/// Verifies that operations missing necessary current readings evaluate to `Written::Owed` rather than silent drops (ADR-0194, ADR-0201).
#[test]
fn an_operation_whose_record_is_owed_is_said_rather_than_swallowed() {
    // Owed, and `NotSettled` is the reason: a tap's record is the beat
    // lock's answer — a tapped tempo, a phase error, an output lag — and
    // none of it is a value a `Current` carries, so nobody has said what
    // it writes here.
    let tap = Operation::TapBeat;
    let owed = written(&tap, &Current::default());
    assert_eq!(
        owed,
        Written::Owed(Owed::NotSettled),
        "a tap is not owed any more — this test names the operation it does, and \
         the one it names has to still be one nobody can write"
    );
    let said = unwritten(&tap, &owed).expect(
        "a tap owes a record and this window said nothing at all — a press whose \
         record nobody has decided how to write reads, in silence, exactly like a \
         press that did not work",
    );
    assert!(
        said.contains("TapBeat") && said.contains(Owed::NotSettled.why()),
        "the window said `{said}`, which does not name both the operation and the \
         question it is waiting on"
    );

    // Verifies mask position conversions report missing mask context when passed incomplete readings (ADR-0341).
    let front = Operation::SetMaskPosition {
        deck: 1,
        position: 0.5,
    };
    let unread = written(&front, &Current::default());
    assert_eq!(
        unread,
        Written::Owed(karakuri_operation_record::Owed::NotRead(
            karakuri_operation_record::Reading::Mask
        )),
        "a mask position with no mask handed in came back with something \
         other than the reading it is missing — a default here is a shape and an \
         angle nobody chose written over the ones a deck is wearing"
    );
    let told = unwritten(&front, &unread).expect(
        "a mask position this window cannot write said nothing at all, so a control \
         that emitted one would read exactly like a control that did not work",
    );
    assert!(
        told.contains("SetMaskPosition")
            && told.contains(Owed::NotRead(karakuri_operation_record::Reading::Mask).why()),
        "the window said `{told}`, which does not name both the operation and the \
         reading it did not get"
    );
    // `FadeDeck` is scheduled against active transition settings and returns `Records`.
    assert_ne!(
        written(
            &Operation::FadeDeck { deck: 1, to: 0.0 },
            &Current {
                transition: Some(karakuri_operation_record::Transition {
                    start: 0.0,
                    beats: 4.0,
                    curve: karakuri_operation::Curve::Smooth,
                    wipe_kind: karakuri_operation::WipeKind::None,
                    wipe_angle: 0.0,
                }),
                ..Current::default()
            }
        ),
        Written::Owed(Owed::NotRead(
            karakuri_operation_record::Reading::Transition
        )),
        "a fade handed the transition settings this window now holds is still owed \
         them, so the reading this panel supplies is not the one the conversion wants"
    );
    assert_ne!(
        told, said,
        "a reading this window forgot and a record nobody has decided how to write \
         read as the same sentence"
    );

    // Silent, and settled: which deck the keys are addressed to is a
    // surface's own state and there is nothing to write.
    let select = Operation::SelectDeck { deck: 1 };
    let silent = written(&select, &Current::default());
    assert_eq!(silent, Written::Silent(Silent::Surface));
    let settled = unwritten(&select, &silent).expect(
        "selecting a deck writes no record and the window said nothing about it \
         either, so a press on such a control would leave no trace at all",
    );
    assert!(
        settled.contains(Silent::Surface.why()),
        "the window said `{settled}`, which does not say why there is no record"
    );

    // **And the two are different sentences.** Collapsing them is the
    // failure this whole test is about at one remove: a harness that
    // printed one line for both would tell an operator that an undecided
    // fade is as settled as a deck selection.
    assert_ne!(
        said, settled,
        "a record nobody can write yet and a record nobody needs to write came out \
         of this window as the same sentence"
    );

    // A record's line is `apply`'s — it says the record *and* what the
    // deck holds afterwards — so this says nothing about that case.
    // Otherwise one press prints twice.
    let gain = Operation::SetGain { deck: 0, gain: 0.5 };
    assert_eq!(
        unwritten(&gain, &written(&gain, &Current::default())),
        None,
        "an operation that wrote a record was also announced as writing none"
    );
}

/// Verifies scheduling fades over sequencer-controlled faders is rejected with explanatory diagnostics (ADR-0322, ADR-0323, P-0092).
#[test]
fn a_move_on_a_fader_a_lane_holds_is_refused_and_said_as_a_decision() {
    let fade = Operation::FadeDeck { deck: 1, to: 0.0 };
    let current = Current {
        transition: Some(karakuri_operation_record::Transition {
            start: 8.0,
            beats: 4.0,
            curve: karakuri_operation::Curve::Smooth,
            wipe_kind: karakuri_operation::WipeKind::None,
            wipe_angle: 0.0,
        }),
        lanes: Some(karakuri_operation_record::Lanes {
            held: vec![(3, karakuri_operation::LaneTarget::Fader { deck: 1 })],
        }),
        ..Current::default()
    };
    let refused = written(&fade, &current);
    assert_eq!(
        refused,
        Written::Refused(karakuri_operation_record::Refusal { lane: 3, deck: 1 }),
        "a fade onto a deck whose fader a lane holds was converted into records — the \
         lane cancels the fade within one step and a replay, which runs no sequencer, \
         would run it"
    );
    let told = unwritten(&fade, &refused).expect(
        "a fade this window refused said nothing at all, so a key that emitted one \
         reads exactly like a key that is not bound",
    );
    assert!(
        told.contains(&karakuri_operation_record::Refusal { lane: 3, deck: 1 }.why()),
        "the window said `{told}`, which is not the sentence the refusal is worded in \
         — the next attempt is to mute the lane it names"
    );
    let gap = unwritten(
        &Operation::TapBeat,
        &written(&Operation::TapBeat, &Current::default()),
    )
    .expect("a tap owes a record and this window says so");
    assert_ne!(
        told, gap,
        "a decision taken and a gap nobody has closed came out of this window as the \
         same sentence"
    );

    // Scans source to ensure reading extracts fader bank assignments from the armed pattern.
    const APPLY: &str = include_str!("../../bridge/handlers/apply/reading.rs");
    let apply: String = APPLY.split_whitespace().collect();
    for wanted in [
        "banks:&karakuri_pattern::Banks,",
        "banks.pattern().held()",
        // The field of the `Current` this window builds, and not a mention of
        // the word in a comment above it.
        "mix,lanes,}",
    ] {
        assert!(
            apply.contains(wanted),
            "`reading` no longer carries `{wanted}` — the lanes reading is how a \
             scheduled move meets the lane holding its fader, and a field left out \
             refuses nothing and says nothing"
        );
    }
}

/// Verifies that layer procedure overrides update the derived material label string (ADR-0228, ADR-0338).
#[test]
fn the_strip_reads_the_base_and_the_procedure_written_over_it() {
    assert_eq!(
        derived_material(
            &base_material(Some("drift_night"), "coil_vortex + star_flares"),
            "orbit_wide"
        ),
        "drift_night + orbit_wide"
    );
    // **A slot nobody has loaded a Set onto**: no id names what it is
    // running, so the base is the pair the run opened with.
    assert_eq!(
        derived_material(
            &base_material(None, "coil_vortex + star_flares"),
            "orbit_wide"
        ),
        "coil_vortex + star_flares + orbit_wide"
    );
}

#[test]
fn a_procedure_load_replaces_one_file_and_keeps_the_base_set() {
    let root = scratch_dir("procedure-load");
    Store::open(&root).expect("a store to keep in");
    std::fs::write(
        root.join(Store::PROCEDURES).join("orbit_wide.kir"),
        "proc orbit_wide {\n  kind L3\n}\n",
    )
    .expect("a kept procedure");
    // The slot's own two files, written where a watcher would be looking:
    // an L1 and an L4, which is the pair every run opens on.
    let l1 = karakuri_environment::scratch::place(&root, "A0-drift_shell", "kind L1\n")
        .expect("the geometry");
    let l4 = karakuri_environment::scratch::place(&root, "A1-star_flares", "kind L4\n")
        .expect("the renderer");

    let (tx, rx) = std::sync::mpsc::channel();
    let mut aim = Aiming::new(
        tx,
        watch::Aim {
            head: karakuri_environment::compile::Named {
                name: Some("shell".into()),
                path: l1.clone(),
            },
            rest: vec![karakuri_environment::compile::Named {
                name: Some("flares".into()),
                path: l4.clone(),
            }],
            layering: Layering::Composite,
            live: Some(2),
            capacity: Some(2048),
            seed_salt: 9,
            salts: vec![9],
            camera: karakuri_engine::camera::Orbit {
                radius: 3.5,
                ..karakuri_engine::camera::Orbit::default()
            },
            overrides: Vec::new(),
            published: Vec::new(),
            bindings: Vec::new(),
            edges: vec![karakuri_engine::set::Edge {
                node: "flares".to_string(),
                slot: "shape".into(),
                to: "shell".to_string(),
            }],
            authorities: Vec::new(),
            set: Some("drift_night".to_owned()),
        },
        karakuri_mcp::Slots::unpointed(),
        0,
    );

    // **The deck holds no camera, so the procedure is added as node 0 of
    // its kind** — the case the row is for.
    let line = overlaying(&root, None, 0, &mut aim, "orbit_wide").expect("the load was refused");
    assert!(
        line.contains("orbit_wide") && line.contains("kept") && line.contains("L3"),
        "{line}"
    );
    let sent = rx.try_recv().expect("no aim was sent");
    assert_eq!(sent.head.path, l1, "the geometry moved");
    assert_eq!(sent.rest.len(), 2, "the slot does not hold three nodes now");
    assert_eq!(sent.rest[0].path, l4, "the renderer moved");
    assert_eq!(
        std::fs::read_to_string(&sent.rest[1].path).expect("the camera's file"),
        "proc orbit_wide {\n  kind L3\n}\n",
        "the file the aim names is not the procedure's own bytes"
    );
    assert_eq!(
        sent.rest[1].name.as_deref(),
        Some("orbit_wide"),
        "a node added by this row is not named after it"
    );

    // **Everything else restated**, which is `Aiming::changed`'s single
    // derivation — the layering, the capacity, the salts, the camera and
    // the wiring come back as the slot's own.
    assert_eq!(sent.layering, Layering::Composite);
    assert_eq!(sent.capacity, Some(2048));
    assert_eq!(sent.salts, vec![9]);
    assert_eq!(sent.camera.radius, 3.5);
    assert_eq!(sent.edges.len(), 1);
    // **And the Set it is filed under does not move**, which is what keeps
    // the snapshot every compile takes alive (ADR-0304, ADR-0308).
    assert_eq!(sent.set.as_deref(), Some("drift_night"));

    // **A second load of the same kind lands on the node the first one
    // added**, which is *the first node of that kind* read a second time:
    // the slot still holds three nodes.
    std::fs::write(
        root.join(Store::PROCEDURES).join("tunnel_eye.kir"),
        "  kind L3\n",
    )
    .expect("a second camera");
    overlaying(&root, None, 0, &mut aim, "tunnel_eye").expect("the second load was refused");
    let sent = rx.try_recv().expect("no second aim was sent");
    assert_eq!(
        sent.rest.len(),
        2,
        "the second camera was added beside the first"
    );
    assert_eq!(
        std::fs::read_to_string(&sent.rest[1].path).expect("the camera's file"),
        "  kind L3\n"
    );
    assert_eq!(
        sent.rest[1].name.as_deref(),
        Some("orbit_wide"),
        "the replaced node did not keep the name the edges resolve against"
    );

    // **A renderer replaces the renderer that is there** — `L4:0`, and the
    // geometry does not move.
    std::fs::write(
        root.join(Store::PROCEDURES).join("hard_dots.kir"),
        "kind L4\n",
    )
    .expect("a renderer");
    overlaying(&root, None, 0, &mut aim, "hard_dots").expect("the renderer load was refused");
    let sent = rx.try_recv().expect("no third aim was sent");
    assert_eq!(sent.head.path, l1, "the geometry moved on a renderer load");
    assert_eq!(sent.rest.len(), 2);
    assert_eq!(
        sent.rest[0].name.as_deref(),
        Some("flares"),
        "the renderer did not keep its node name"
    );
    assert_ne!(
        sent.rest[0].path, l4,
        "the renderer's file was not replaced"
    );

    // **A name neither tier holds is refused with the name back**, and
    // nothing is sent.
    let why = overlaying(&root, None, 0, &mut aim, "no_such_thing")
        .expect_err("a name nothing holds was loaded");
    assert!(
        why.contains("no_such_thing") && why.contains("procedures"),
        "{why}"
    );
    assert!(rx.try_recv().is_err(), "a refused load sent an aim");

    // **A `.kir` that declares no kind is refused too**, because there is
    // no layer to write it over.
    std::fs::write(
        root.join(Store::PROCEDURES).join("mute.kir"),
        "// nothing\n",
    )
    .expect("a procedure with no kind");
    let why = overlaying(&root, None, 0, &mut aim, "mute")
        .expect_err("a procedure with no kind was loaded");
    assert!(why.contains("declares no `kind`"), "{why}");

    std::fs::remove_dir_all(&root).expect("clean up");
}

#[test]
fn a_composite_press_re_aims_the_slot_and_restates_the_rest_of_its_aim() {
    let (tx, rx) = std::sync::mpsc::channel();
    let mut aims = vec![Aiming::new(
        tx,
        watch::Aim {
            head: karakuri_environment::compile::Named {
                name: Some("grid".into()),
                path: std::path::PathBuf::from("A0-grid.kir"),
            },
            rest: vec![karakuri_environment::compile::Named::bare("A1-points.kir")],
            // **Overdrawing**, so the press below asks for the other one
            // and the assertion is about a field that moved.
            layering: Layering::Overdraw,
            live: Some(2),
            capacity: Some(2048),
            seed_salt: 9,
            salts: vec![9],
            // **A camera nobody's default produces**, so the assertion
            // below is about a value that was carried rather than one that
            // happens to coincide with `Orbit::default()`.
            camera: karakuri_engine::camera::Orbit {
                radius: 3.5,
                ..karakuri_engine::camera::Orbit::default()
            },
            overrides: Vec::new(),
            published: Vec::new(),
            bindings: Vec::new(),
            edges: vec![karakuri_engine::set::Edge {
                node: "warp".to_string(),
                slot: "shape".into(),
                to: "field".to_string(),
            }],
            authorities: Vec::new(),
            set: Some("night01".to_owned()),
        },
        karakuri_mcp::Slots::unpointed(),
        0,
    )];

    let line = composited(
        &mut aims,
        &Operation::SetCompositing {
            deck: 0,
            compositing: true,
        },
    )
    .expect("the arm answered nothing for the operation it is for");
    assert!(
        line.contains("composite") && line.contains("recompiling"),
        "the answer does not say what was asked for or that the slot rebuilds: {line}"
    );

    let aim = rx.try_recv().expect("the watcher was not re-aimed at all");
    assert_eq!(
        aim.layering,
        Layering::Composite,
        "the press did not move the one field it is about"
    );
    // **The thirteen that did not move.** Each of these is a symptom
    // somebody would meet on the next save rather than on this press.
    assert_eq!(aim.head.name.as_deref(), Some("grid"));
    assert_eq!(aim.rest.len(), 1);
    assert_eq!(aim.live, Some(2), "the fold was silently un-selected");
    assert_eq!(aim.capacity, Some(2048), "the capacity came back as none");
    assert_eq!(aim.seed_salt, 9);
    assert_eq!(aim.salts, vec![9], "the salts would repaint every element");
    // `Orbit` is not `PartialEq`, so the field the camera's own loss shows
    // in is what this reads — `Watch::camera`'s symptom is a slot back at
    // `Orbit::default()`, and a radius nobody could have written is what
    // tells the two apart.
    assert_eq!(
        aim.camera.radius, 3.5,
        "the camera came back at its default"
    );
    assert_eq!(aim.edges.len(), 1, "the run's wiring was dropped");
    assert_eq!(
        aim.set.as_deref(),
        Some("night01"),
        "the press dropped the Set the slot is running, so every version written after it \
         would be filed under none"
    );
    assert_eq!(aims[0].at.layering, Layering::Composite);

    // **Asking for the layering the slot is now in sends nothing**, because
    // a re-aim rebuilds the whole slot and this one would land on the same
    // picture (P-0091).
    let line = composited(
        &mut aims,
        &Operation::SetCompositing {
            deck: 0,
            compositing: true,
        },
    )
    .expect("the arm answered nothing");
    assert!(
        line.contains("already"),
        "the answer does not say the slot is already set that way: {line}"
    );
    assert!(
        rx.try_recv().is_err(),
        "a press asking for the state the slot is in recompiled it"
    );

    // **And the second press restates what the first one left**, which is
    // what keeping `Aiming::at` buys: back to overdraw, with the layering
    // read off the aim this program is holding rather than off the launch
    // pair.
    composited(
        &mut aims,
        &Operation::SetCompositing {
            deck: 0,
            compositing: false,
        },
    )
    .expect("the arm answered nothing");
    let aim = rx.try_recv().expect("the second press re-aimed nothing");
    assert_eq!(aim.layering, Layering::Overdraw);
    assert_eq!(aim.set.as_deref(), Some("night01"));

    // A slot this deck has not got, in the one sentence every surface
    // refuses one in.
    let why = composited(
        &mut aims,
        &Operation::SetCompositing {
            deck: 3,
            compositing: true,
        },
    )
    .expect("a slot the deck has not got answered nothing");
    assert!(
        why.contains(&karakuri_environment::no_such_slot(3, 1)),
        "the refusal is not the one every other surface gives: {why}"
    );
    assert!(rx.try_recv().is_err(), "a refused press re-aimed a watcher");

    // And it answers `None` for everything that is not its operation, so
    // the dispatch above can call it on every press.
    assert!(composited(&mut aims, &Operation::Quit).is_none());
}
