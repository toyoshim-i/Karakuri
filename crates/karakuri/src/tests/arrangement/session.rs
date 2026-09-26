use super::*;

/// Verifies that loading a Set writes procedures into the scratch directory and updates slot aiming.
#[test]
fn a_load_writes_the_sets_procedures_into_the_scratch_and_aims_the_slot_there() {
    use karakuri_environment::setfile;
    use karakuri_store::Hash;

    let root = std::env::temp_dir().join(format!(
        "karakuri-load-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let store = Store::open(&root).expect("a store to load from");

    // Two real procedures, so the load goes through the checker the way a
    // Set out of the library does. `lattice_shell` is chosen for its name:
    // it is what the scratch file has to be called after.
    let examples = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let sources: Vec<(karakuri_store::Layer, String)> = [
        (karakuri_store::Layer::L1, "lattice_shell.kir"),
        (karakuri_store::Layer::L4, "soft_points.kir"),
    ]
    .into_iter()
    .map(|(layer, file)| {
        let src = std::fs::read_to_string(examples.join(file)).expect("an example");
        (layer, src)
    })
    .collect();
    let nodes: Vec<setfile::Node> = sources
        .iter()
        .map(|(layer, src)| setfile::Node {
            hash: {
                let hash = Hash::of(src.as_bytes());
                store.put_artifact(src.as_bytes()).expect("store a source");
                hash
            },
            layer: match layer {
                karakuri_store::Layer::L1 => karakuri_ir::Kind::L1,
                _ => karakuri_ir::Kind::L4,
            },
            index: 0,
            // A name the file wrote, which is what a rebuild has to call
            // the node — not the procedure's own.
            name: Some(match layer {
                karakuri_store::Layer::L1 => "grid".to_owned(),
                _ => "draw".to_owned(),
            }),
        })
        .collect();
    // Values no default here produces, so an aim that kept the slot's own
    // cannot pass by accident.
    let seeds = [0x0bad_cafeu32];
    setfile::save(
        &store,
        Asked::Operator,
        "night01",
        setfile::Saving {
            nodes: &nodes,
            capacities: &[2048],
            params: &[],
            bindings: &[],
            edges: &[],
            camera: &karakuri_engine::camera::Orbit::default(),
            layering: Layering::Composite,
            live: Some(0),
            seeds: &seeds,
        },
    )
    .expect("write the Set file");

    // Deck B, so the letter in the scratch name is not the first one and a
    // hard-coded `A` fails here.
    let (tx, rx) = std::sync::mpsc::channel();
    // **An [`Aiming`] and not a bare sender**, because a load keeps where it
    // pointed the watcher — see the assertion at the end of this test.
    let mut aiming = Aiming::new(
        tx,
        watch::Aim {
            head: karakuri_environment::compile::Named::bare("nowhere.kir"),
            rest: Vec::new(),
            layering: Layering::Overdraw,
            live: None,
            capacity: None,
            seed_salt: 0,
            salts: Vec::new(),
            camera: karakuri_engine::camera::Orbit::default(),
            overrides: Vec::new(),
            published: Vec::new(),
            bindings: Vec::new(),
            edges: Vec::new(),
            authorities: Vec::new(),
            // **Running material no Set names**, which is where every slot
            // of this program starts and what the launch seed files under.
            set: None,
        },
        // What this test asserts is the *aim*; where the layout is
        // published is `a_load_moves_what_the_mcp_server_resolves_against`.
        karakuri_mcp::Slots::unpointed(),
        ASKED_TO_PRIME,
    );
    let line = loading(
        &root,
        ASKED_TO_PRIME,
        slot_salt(ASKED_TO_PRIME),
        &mut aiming,
        "night01",
    )
    .unwrap_or_else(|e| panic!("the load failed: {e}"));
    assert!(
        line.contains("deck B"),
        "the line does not say where: {line}"
    );

    let aim = rx.try_recv().expect("the slot was aimed at something");
    assert_eq!(aim.rest.len(), 1, "the renderer did not travel with it");
    assert_eq!(
        aim.head.name.as_deref(),
        Some("grid"),
        "the head is not called what the file called it, so an edge would not resolve"
    );
    for (named, (_, src)) in std::iter::once(&aim.head)
        .chain(aim.rest.iter())
        .zip(&sources)
    {
        let name = named
            .path
            .file_name()
            .and_then(|n| n.to_str())
            .expect("a file name");
        assert!(
            name.starts_with("B0-") || name.starts_with("B1-"),
            "`{name}` carries neither the deck nor the node, so two decks would share it"
        );
        assert_eq!(
            std::fs::read_to_string(&named.path).expect("the scratch file"),
            *src,
            "the watcher is pointed at a file that is not the Set's source"
        );
    }

    assert_eq!(aim.layering, Layering::Composite, "the file's layering");
    assert_eq!(aim.live, Some(0), "the file's fold");
    assert_eq!(aim.capacity, Some(2048), "the file's capacity");
    assert_eq!(aim.seed_salt, seeds[0], "the file's seed");
    assert_eq!(aim.salts, vec![seeds[0]], "the file's salts");
    assert!(
        aim.authorities.is_empty(),
        "a Set file carries no grant, so a load must hand none over"
    );
    // Associates subsequent edits with the loaded Set ID in history tracking (ADR-0276).
    assert_eq!(
        aim.set.as_deref(),
        Some("night01"),
        "the load did not tell the watcher which Set the slot is running"
    );

    // **And the load kept where it pointed the watcher**, which is what a
    // later rewiring restates the other twelve fields from: an `Aiming` that
    // sent an aim and left `at` behind would re-aim this slot at the pair
    // the run launched with. See [`Aiming`].
    assert_eq!(
        aiming.at.head.name.as_deref(),
        Some("grid"),
        "the load sent an aim and did not keep it"
    );
    assert_eq!(aiming.at.live, Some(0), "the kept aim is not the sent one");
    assert_eq!(
        aiming.at.set.as_deref(),
        Some("night01"),
        "the kept aim does not carry the Set, so the next rewiring would restate none"
    );

    // And a Set that is not there is a sentence with nothing sent: the
    // deck goes on playing what it was.
    let e = loading(&root, ON_AIR, slot_salt(ON_AIR), &mut aiming, "nothing01")
        .expect_err("a Set that is not in the store");
    assert!(e.contains("nothing01"), "the refusal does not name it: {e}");
    assert!(
        rx.try_recv().is_err(),
        "a load that failed aimed the slot anyway"
    );
    assert_eq!(
        aiming.at.live,
        Some(0),
        "a load that failed moved where the watcher is pointed"
    );

    std::fs::remove_dir_all(&root).expect("clean up");
}

/// Verifies loading a library Set updates MCP address resolution to new scratch paths (ADR-0228, ADR-0308, P-0083).
/// Unloaded slots remain intact and requests for absent renderers are rejected.
#[test]
fn a_load_moves_what_the_mcp_server_resolves_against() {
    use karakuri_environment::setfile;
    use karakuri_store::Hash;

    let root = scratch_dir("mcp-load");
    let store = Store::open(&root).expect("a store to load from");

    // A Set of two nodes, through the checker exactly as a load out of the
    // library goes. `soft_points` is the renderer's name, which is what the
    // scratch file is called after.
    let examples = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let nodes: Vec<setfile::Node> = [
        (karakuri_ir::Kind::L1, "lattice_shell.kir"),
        (karakuri_ir::Kind::L4, "soft_points.kir"),
    ]
    .into_iter()
    .map(|(layer, file)| {
        let src = std::fs::read_to_string(examples.join(file)).expect("an example");
        let hash = Hash::of(src.as_bytes());
        store.put_artifact(src.as_bytes()).expect("store a source");
        setfile::Node {
            hash,
            layer,
            index: 0,
            name: None,
        }
    })
    .collect();
    setfile::save(
        &store,
        Asked::Operator,
        "night02",
        setfile::Saving {
            nodes: &nodes,
            capacities: &[2048],
            params: &[],
            bindings: &[],
            edges: &[],
            camera: &karakuri_engine::camera::Orbit::default(),
            layering: Layering::Overdraw,
            live: None,
            seeds: &[0x0bad_cafe],
        },
    )
    .expect("write the Set file");

    // The launch layout: two decks, and the one about to be loaded onto
    // holds **two** renderers, so `L4:1` is a real address before the press
    // and the refusal asserted below is a change rather than a constant.
    let launch = |name: &str, kind: &str| {
        let path = root.join(name);
        std::fs::write(&path, format!("proc launched {{\n  kind {kind}\n}}\n"))
            .expect("a launch copy");
        path
    };
    let a_l1 = launch("A0-launch.kir", "L1");
    let a_l4 = launch("A1-launch.kir", "L4");
    let b_l1 = launch("B0-launch.kir", "L1");
    let b_l4 = launch("B1-launch.kir", "L4");
    let b_l4_second = launch("B2-launch.kir", "L4");
    let pointing = karakuri_mcp::Slots::of(vec![
        (a_l1.clone(), vec![a_l4.clone()]),
        (b_l1, vec![b_l4.clone(), b_l4_second.clone()]),
    ]);
    assert_eq!(
        pointing
            .file(ASKED_TO_PRIME, "L4", 0)
            .expect("the launch L4"),
        b_l4,
        "the fixture does not start on the launch copies"
    );

    // The slot's watcher, pointed where the run launched it. `Aiming::new`
    // publishes that, which is what a window remade does too.
    let (tx, _rx) = std::sync::mpsc::channel();
    let mut aiming = Aiming::new(
        tx,
        watch::Aim {
            head: karakuri_environment::compile::Named::bare(&b_l4),
            rest: Vec::new(),
            layering: Layering::Overdraw,
            live: None,
            capacity: None,
            seed_salt: 0,
            salts: Vec::new(),
            camera: karakuri_engine::camera::Orbit::default(),
            overrides: Vec::new(),
            published: Vec::new(),
            bindings: Vec::new(),
            edges: Vec::new(),
            authorities: Vec::new(),
            set: None,
        },
        pointing.clone(),
        ASKED_TO_PRIME,
    );

    loading(
        &root,
        ASKED_TO_PRIME,
        slot_salt(ASKED_TO_PRIME),
        &mut aiming,
        "night02",
    )
    .unwrap_or_else(|e| panic!("the load failed: {e}"));

    // **The new scratch file, and not the launch copy.**
    let landed = pointing
        .file(ASKED_TO_PRIME, "L4", 0)
        .expect("the loaded Set's renderer");
    assert_eq!(
        landed,
        root.join(karakuri_environment::scratch::DIR)
            .join("B1-soft_points.kir"),
        "a write addressed to deck B's renderer would not reach the file its \
         watcher is polling"
    );
    assert_ne!(
        landed, b_l4,
        "the address resolved against the layout the deck stopped running"
    );
    assert_eq!(
        pointing
            .file(ASKED_TO_PRIME, "L1", 0)
            .expect("the loaded Set's geometry"),
        root.join(karakuri_environment::scratch::DIR)
            .join("B0-lattice_shell.kir")
    );

    // **The second renderer is gone, and the refusal says what is there
    // now** rather than reporting a range the deck stopped holding.
    let refused = pointing
        .file(ASKED_TO_PRIME, "L4", 1)
        .expect_err("`night02` holds one renderer");
    assert_eq!(
        refused, "slot 1 holds one L4 and `index` is 1",
        "the refusal does not name what the deck holds now"
    );

    // **And nothing else moved.** A publication that wrote the deck rather
    // than the slot would be a worse defect than the one this fixes.
    assert_eq!(
        pointing.file(ON_AIR, "L4", 0).expect("deck A is untouched"),
        a_l4,
        "the load moved a deck nobody named"
    );

    std::fs::remove_dir_all(&root).expect("clean up");
}

/// Verifies each deck maintains isolated scratch copies and seeds initial history versions under no Set (ADR-0089, ADR-0276).
#[test]
fn the_launch_versions_are_filed_before_a_window() {
    let root = scratch_dir("seeded");
    let named = shipped();
    let (_, running) =
        working_copies(&root, &named, SLOTS).unwrap_or_else(|e| panic!("no copies: {e}"));

    let _shared = seeded(&root, &running);

    let listing = karakuri_environment::history::list(&root, 64).expect("the history lists");
    let nodes: usize = running.len() * 2;
    assert_eq!(
        listing.versions.len(),
        nodes,
        "a deck's starting version is missing, so its first edit has nothing to be \
         walked back to: {:?}",
        listing.versions
    );
    assert!(
        listing.versions.iter().all(|v| v.set.is_none()),
        "a slot launched on a typed pair filed its version under a Set: {:?}",
        listing.versions
    );
    for slot in 0..running.len() {
        let of_slot: Vec<&karakuri_environment::history::Version> =
            listing.versions.iter().filter(|v| v.slot == slot).collect();
        assert_eq!(
            of_slot.len(),
            2,
            "deck {} filed {} of its two nodes",
            deck_letter(slot as u8),
            of_slot.len()
        );
        assert!(
            of_slot.iter().any(|v| v.layer == "L1") && of_slot.iter().any(|v| v.layer == "L4"),
            "deck {}'s two versions are not its geometry and its renderer: {of_slot:?}",
            deck_letter(slot as u8)
        );
    }

    std::fs::remove_dir_all(&root).expect("clean up");
}

/// A CPU test: a scratch is a directory and nothing here takes a device.
#[test]
fn every_deck_runs_from_its_own_copy_and_an_edit_moves_one_deck() {
    let root = scratch_dir("own-copy");
    let named = shipped();
    let (dir, running) =
        working_copies(&root, &named, SLOTS).unwrap_or_else(|e| panic!("no copies: {e}"));

    assert_eq!(
        running.len(),
        SLOTS,
        "a deck of {SLOTS} slots got {running:?}"
    );
    assert_eq!(dir, root.join(karakuri_environment::scratch::DIR));

    // 1. Nothing a deck holds points at what the operator typed.
    for (slot, pair) in running.iter().enumerate() {
        for path in [&pair.l1, &pair.l4] {
            assert!(
                path.starts_with(&dir),
                "deck {} still runs from {}",
                deck_letter(slot as u8),
                path.display()
            );
        }
    }

    // 2. Eight files and not two, and each says which deck it belongs to.
    let mut every: Vec<&std::path::PathBuf> = running.iter().flat_map(|p| [&p.l1, &p.l4]).collect();
    let held = every.len();
    every.sort();
    every.dedup();
    assert_eq!(
        every.len(),
        held,
        "{SLOTS} decks on one pair share a file, so an edit cannot reach one of them"
    );
    for (slot, pair) in running.iter().enumerate() {
        let letter = deck_letter(slot as u8);
        for (at, path) in [&pair.l1, &pair.l4].into_iter().enumerate() {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .expect("a file name");
            assert!(
                name.starts_with(&format!("{letter}{at}-")),
                "`{name}` carries neither the deck nor the node's place"
            );
        }
    }

    // 3. **The claim.** An edit in one place moves one deck.
    let before = std::fs::read_to_string(&running[ON_AIR].l1).expect("deck A's L1");
    std::fs::write(&running[ASKED_TO_PRIME].l1, "deck B only").expect("edit deck B");
    assert_eq!(
        std::fs::read_to_string(&running[ASKED_TO_PRIME].l1).expect("read"),
        "deck B only"
    );
    for (slot, deck) in running.iter().enumerate().take(SLOTS) {
        if slot == ASKED_TO_PRIME {
            continue;
        }
        assert_eq!(
            std::fs::read_to_string(&deck.l1).expect("read"),
            before,
            "editing deck B moved deck {} as well",
            deck_letter(slot as u8)
        );
    }

    // 4. And the preset is what it was, which is the whole reason the
    // scratch exists: three shipped presets were replaced in one session.
    assert_eq!(
        std::fs::read_to_string(&named.l1).expect("the preset"),
        before,
        "the file the operator named was written to"
    );

    std::fs::remove_dir_all(&root).expect("clean up");
}

/// The program has to say it. Four decks on one preset are four files whose
/// names an operator cannot guess and cannot tell apart by content — at startup
/// they hold the same bytes — so the startup print names the directory and then
/// one file pair per deck.
#[test]
fn the_startup_print_names_one_file_per_deck() {
    let root = scratch_dir("own-copy-said");
    let (dir, running) = working_copies(&root, &shipped(), SLOTS).expect("copies");
    let said = running_from(&dir, &running);

    assert!(
        said.contains(&dir.display().to_string()),
        "the print does not say where: {said}"
    );
    for (slot, pair) in running.iter().enumerate() {
        let letter = deck_letter(slot as u8);
        assert!(
            said.contains(&format!("deck {letter}:")),
            "deck {letter} is not in the print: {said}"
        );
        for path in [&pair.l1, &pair.l4] {
            let name = path.file_name().and_then(|n| n.to_str()).expect("a name");
            assert!(
                said.contains(name),
                "`{name}` is a file the deck runs from and the print does not name it: \
                 {said}"
            );
        }
    }
    // **And the four lines are four different answers.** A print that
    // named the same two files under all four decks would be a print an
    // operator cannot act on — which is exactly what this program said
    // while every deck watched the pair that was typed.
    let lines: Vec<&str> = said
        .lines()
        .filter(|line| line.trim_start().starts_with("deck "))
        .collect();
    assert_eq!(lines.len(), SLOTS, "one line per deck, and got {lines:?}");
    let named: Vec<&str> = lines
        .iter()
        .map(|line| line.split_once(':').expect("`deck A: files`").1)
        .collect();
    let mut distinct = named.clone();
    distinct.sort_unstable();
    distinct.dedup();
    assert_eq!(
        distinct.len(),
        named.len(),
        "two decks were told to open the same file: {named:?}"
    );

    // And it says the thing an operator will otherwise read as a bug.
    assert!(
        said.contains("not written to"),
        "the print does not say the named paths are left alone: {said}"
    );

    std::fs::remove_dir_all(&root).expect("clean up");
}
