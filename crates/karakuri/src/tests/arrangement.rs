use super::*;
use karakuri_console::panel::Panel;
use karakuri_console::view::{Scope, View};
use karakuri_environment::Asked;
use karakuri_operation::Operation;

/// A folder let go on this window points the bay at it, marks the chip and
/// lists what is in it — all three on the pass the drop arrives on.
///
/// The three are one act by ADR-0275's third policy: `dropped_files` is visible
/// for one pass and then gone, so a bay that had set the directory and waited
/// for the chip to be pressed would be holding a path nothing will hand it
/// again. It is asserted as three outcomes of one call for that reason.
///
/// And what the listing holds is Sets and not parts, which is the `presets`
/// scope's rule one chip along: both spellings of a Set file are rows, a `.kir`
/// is not, and a subdirectory named like a Set file belongs to whoever made it.
///
/// A CPU test: a directory and a `View`.
#[test]
fn a_folder_dropped_on_the_window_points_the_bay_at_it() {
    let root = scratch_dir("folder-drop");
    let store = root.join("store");
    let handed = root.join("from-somebody");
    std::fs::create_dir_all(&handed).expect("a folder to drop");
    for name in ["night01.kbset", "sketch.kset", "adrift.kbset"] {
        std::fs::write(handed.join(name), "").expect("a Set file");
    }
    // What a folder scope must not list: a part, and a directory wearing a
    // Set file's name.
    std::fs::write(handed.join("blur.kir"), "").expect("a part");
    std::fs::create_dir_all(handed.join("unpacked.kbset")).expect("a directory");

    let mut view = View::new(Room::Day);
    view.scopes = Scope::ALL.to_vec();
    assert!(view.select_scope(Scope::MySets));
    let mut folder: Option<std::path::PathBuf> = None;

    let said = folder_dropped(&mut view, &mut folder, &store, None, &[handed.as_path()])
        .expect("a drop of one folder was answered");

    assert_eq!(folder.as_deref(), Some(handed.as_path()));
    assert_eq!(
        view.folder.as_deref(),
        Some(handed.display().to_string().as_str()),
        "the panel was not handed the line to draw in the `.path` row"
    );
    assert_eq!(
        view.scope(),
        Some(Scope::Folder),
        "the drop set the directory and left another chip marked"
    );
    assert_eq!(
        view.library,
        vec![
            "adrift".to_owned(),
            "night01".to_owned(),
            "sketch".to_owned()
        ],
        "the folder scope lists {:?}",
        view.library
    );
    assert!(
        said.contains(&handed.display().to_string())
            && said.contains("the `folder` chip is marked"),
        "the drop said `{said}`"
    );

    // **And the row is not drawn as a hover**: the release is what was
    // read, so nothing is on its way in afterwards.
    assert_eq!(view.pointed().map(|at| at.incoming), Some(false));

    // A second drop re-points it, which is the whole of *re-pointing the
    // bay is another drop*.
    let empty = root.join("empty");
    std::fs::create_dir_all(&empty).expect("a second folder");
    let said = folder_dropped(&mut view, &mut folder, &store, None, &[empty.as_path()])
        .expect("a second drop was answered");
    assert_eq!(folder.as_deref(), Some(empty.as_path()));
    assert!(view.library.is_empty(), "{:?}", view.library);
    assert!(
        said.contains(why_nothing(Scope::Folder, true, false)),
        "a folder holding no Set said `{said}`"
    );
    // **The chip is marked on this one too**, and the sentence says so:
    // the second drop moved the listing without moving the mark, which is
    // the one case a *whether it moved* answer would have got backwards.
    assert!(
        said.contains("the `folder` chip is marked"),
        "a second drop said `{said}`"
    );

    std::fs::remove_dir_all(&root).expect("clean up");
}

/// The two refusals ADR-0275 writes, and what they leave behind.
///
/// - One path, and it has to be a directory. A file is refused naming
///   what was dropped rather than read as the folder it sits in, which
///   would point this bay at a directory nobody pointed at; a `.kbset` is
///   refused with the press that *does* take a Set in, because a file
///   landing on this window has no row under it.
/// - More than one path is refused, and all of them are, counting what
///   arrived: a multi-item drag is one pass with several entries, so there
///   is no first to act on and nothing says which was aimed at.
///
/// Every one of them says where the library is still pointed, which is
/// the half that makes a refusal readable at a glance (P-0083): a bay that
/// went on listing what it listed and a bay that quietly moved are the
/// same drawing.
///
/// A CPU test: a directory and a `View`.
#[test]
fn a_drop_that_is_not_one_folder_is_refused_and_the_bay_keeps_what_it_had() {
    let root = scratch_dir("folder-refusal");
    let store = root.join("store");
    let handed = root.join("from-somebody");
    std::fs::create_dir_all(&handed).expect("a folder to drop");
    std::fs::write(handed.join("night01.kbset"), "").expect("a Set file");
    let second = root.join("another");
    std::fs::create_dir_all(&second).expect("a second folder");
    let loose = root.join("notes.txt");
    std::fs::write(&loose, "").expect("a file to drop");
    let set = root.join("handover.kbset");
    std::fs::write(&set, "").expect("a Set file to drop");

    let mut view = View::new(Room::Day);
    view.scopes = Scope::ALL.to_vec();
    let mut folder: Option<std::path::PathBuf> = None;
    folder_dropped(&mut view, &mut folder, &store, None, &[handed.as_path()])
        .expect("the folder this bay is pointed at");
    let listed = view.library.clone();

    // A file, and it is not read as the directory it sits in.
    let said = folder_dropped(&mut view, &mut folder, &store, None, &[loose.as_path()])
        .expect("a file was answered");
    assert!(
        said.contains("notes.txt") && said.contains("folder"),
        "the refusal an operator reads is `{said}`"
    );
    // **And it says where the bay is still pointed**, which is the half
    // that makes the refusal readable rather than the operator having to
    // look at the row to find out whether anything moved.
    assert!(
        said.contains(&format!("still pointed at `{}`", handed.display())),
        "the refusal does not say where the library is still pointed: `{said}`"
    );

    // A Set file, and the refusal carries the press that takes one in.
    let said = folder_dropped(&mut view, &mut folder, &store, None, &[set.as_path()])
        .expect("a Set file was answered");
    assert!(
        said.contains("handover.kbset") && said.contains("row"),
        "a `.kbset` let go on the window said `{said}`"
    );

    // Two folders at once, and neither of them is taken.
    let said = folder_dropped(
        &mut view,
        &mut folder,
        &store,
        None,
        &[handed.as_path(), second.as_path()],
    )
    .expect("two paths were answered");
    assert!(
        said.contains('2') && said.contains("none of them"),
        "two folders at once said `{said}`"
    );

    // A path that is not there at all, which is a third thing and is said
    // as one: whether it is a folder was never learned.
    let gone = root.join("no-such-folder");
    let said = folder_dropped(&mut view, &mut folder, &store, None, &[gone.as_path()])
        .expect("a path that is not there was answered");
    assert!(
        said.contains("no-such-folder") && said.contains("could not be examined"),
        "a path that is not there said `{said}`"
    );

    // **And after all four the bay is where it was**, which every one of
    // them said it would be.
    assert_eq!(folder.as_deref(), Some(handed.as_path()));
    assert_eq!(view.library, listed);
    assert_eq!(view.scope(), Some(Scope::Folder));

    // Nothing dropped is not a refusal and is not an answer.
    assert_eq!(
        folder_dropped(&mut view, &mut folder, &store, None, &[]),
        None
    );

    std::fs::remove_dir_all(&root).expect("clean up");
}

/// A folder over the window reads in the `.path` row, and two read as none.
///
/// The hover is the one thing about this bay that a frame does — `egui` clones
/// the hovered files onto every pass while a drag lasts — so this is the whole
/// of what a pass owes it: the path where there is one to draw, nothing where a
/// release would set nothing, and no allocation where neither has changed.
///
/// A CPU test: a `View` and a list of paths.
#[test]
fn a_folder_over_the_window_reads_in_the_path_row_and_two_read_as_none() {
    let one = karakuri_console::egui::HoveredFile {
        path: Some(std::path::PathBuf::from("/Volumes/stick/handover")),
        mime: String::new(),
    };
    let two = karakuri_console::egui::HoveredFile {
        path: Some(std::path::PathBuf::from("/Volumes/stick/another")),
        mime: String::new(),
    };

    let mut view = View::new(Room::Day);
    folder_over(&mut view, std::slice::from_ref(&one));
    assert_eq!(view.incoming.as_deref(), Some("/Volumes/stick/handover"));
    assert_eq!(view.pointed().map(|at| at.incoming), Some(true));

    // **Two at once say nothing**, because there is no path a release
    // would set — and picking the first would be the choice the refusal
    // above exists to refuse.
    folder_over(&mut view, &[one.clone(), two]);
    assert_eq!(view.incoming, None);

    // Out of the window again, and what was chosen is what is drawn.
    view.folder = Some("/Users/somebody/sets".to_owned());
    folder_over(&mut view, std::slice::from_ref(&one));
    assert_eq!(
        view.pointed().map(|at| at.path),
        Some("/Volumes/stick/handover")
    );
    folder_over(&mut view, &[]);
    assert_eq!(
        view.pointed(),
        Some(karakuri_console::view::Pointed {
            path: "/Users/somebody/sets",
            incoming: false,
        })
    );
}

/// A load writes the Set's procedures where the slot's watcher is looking and
/// aims it there, and it touches no deck at all.
///
/// This is the whole of what `Operation::LoadSet` needed, and what it is *not*
/// is the claim: `Deck::install` is the one function that puts a built Set in a
/// slot and is documented as deliberately unreachable from a key or a surface,
/// because *"a live run changes its material by editing a file and letting the
/// worker build it, which is what the budget watchdog is attached to"*. So this
/// asserts files and an aim. A load that built a Set here would be a picture
/// nothing measured, in a slot the watchdog never got to judge.
///
/// Three things beyond *it happened*, and each is a wrong load that looks
/// right. The scratch name carries the deck letter and the node's place,
/// because `scratch::place` overwrites by name and two decks loading Sets whose
/// procedures share one would silently become one file — the second load moving
/// the first deck on its watcher's next poll. The aim restates the layering,
/// the fold, the capacity and the salts the *file* recorded rather than the
/// ones the slot was running at, because that is the failure every `Watch`
/// field is documented against and it does not show on the load: it shows on
/// the first save afterwards. And the node names are the file's, because an
/// `edge` resolves against them.
///
/// A CPU test: a store is a directory, and nothing here takes a device.
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
    // **The id the versions after this load are filed under.** The slot was
    // running material no Set names, and it is running `night01` now; an
    // aim that left this at `None` would go on writing this Set's edits
    // into the history under no Set at all, which is a chain that answers
    // *what versions has `night01` had* with nothing (ADR-0276).
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

/// A load moves what the MCP server resolves an address against, and it moves
/// nothing else's.
///
/// This program built the `mcp::Slots` it handed the server out of the launch
/// working copies and never wrote it again. A library load writes new scratch
/// files and re-points that slot's watcher at them (ADR-0228), so from the
/// first load onwards every address the server resolved was the layout the deck
/// had stopped running: `read_procedure` answered about the wrong material,
/// `write_procedure` wrote a file no watcher was polling and reported that it
/// was being built, and a node the loaded Set does hold was refused for not
/// existing. None of the three fails — they are plausible wrong answers on the
/// surface whose reader is a program in a loop (`docs/principles/0094-…`).
/// ADR-0308 recorded it and worked around it for the landing alone.
///
/// The assertion is `Slots::file`, which is the walk the server writes through:
/// `write_procedure` and `read_procedure` resolve an address with
/// `Slots::path`, and `file` is that same private walk with the layer taken as
/// a word. So the path asserted here is the path the server would write to.
///
/// Three things. The loaded slot resolves to the new scratch file and not to
/// the launch copy; a renderer the launch pair had and the loaded Set has not
/// is refused naming what the slot holds now (P-0083); and the slot nobody
/// loaded onto has not moved, because a publication per slot that overwrote the
/// deck would be a worse defect than the one being fixed.
///
/// A CPU test: a store and a scratch are directories, and nothing here takes a
/// device.
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

/// The requirement this program was failing: the same preset loaded into every
/// slot, and each deck watching its own separate copy in its own place.
///
/// Every slot used to be handed [`Sources`] itself — the two paths the operator
/// typed — so four watchers polled two files. One save rebuilt four slots, and
/// since a parked slot's trial never reaches a verdict, three of the four rows
/// it put in the Staging lane stayed there for the rest of the run. That
/// symptom is this defect's, not the lane's.
///
/// Four things, and the third is the one the requirement is about. The copies
/// are under the scratch and not where the operator pointed; the four decks
/// hold eight distinct files rather than two shared ones; an edit made through
/// deck B's L1 moves deck B and no other deck; and the file the operator named
/// is not written to at all.
///
/// The version every deck starts on is in the history before the window opens,
/// and it is filed under no Set.
///
/// Two claims, and the second is the one that is a decision. That there is a
/// seed at all is ADR-0089's — a first edit whose predecessor was never written
/// down cannot be walked back — and this program had no history at all until it
/// was given one, so a run's whole night of edits was kept nowhere. That every
/// row reads `None` is ADR-0276's: the material is a pair somebody typed, and
/// filing it under `Sources::material` would put rows under a Set no listing
/// can ever match.
///
/// Every node of every slot, and the count is derived from the copies rather
/// than written here — a slot is an L1 and a renderer, and each deck runs from
/// its own pair, so a seed that filed one deck or one node would leave the
/// others' first edits with nothing behind them.
///
/// A CPU test: a store and a scratch are directories, and nothing here takes a
/// device.
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

/// A root of this test's own, cleaned of whatever a previous run left.
fn arrangement_root(what: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!(
        "karakuri-arrangement-{what}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    root
}

/// A panel with something folded and something soloed — the two things an
/// arrangement is kept for, and the two a reset forgets.
fn arranged(width: f32, height: f32) -> Panel {
    let mut panel = Panel::new(width, height);
    let staging = panel.layout().find("staging").expect("a staging bay");
    panel.op(Op::Fold(staging));
    let mixer = panel.layout().find("mixer").expect("a mixer bay");
    panel.op(Op::Solo(mixer));
    panel.solve();
    panel
}

/// The bytes are at the path `karakuri-store`'s own header claims, read off
/// that path and not through the store that wrote them.
///
/// This is the assertion ADR-0221 §4 says the store's suite had to spell out
/// rather than leave to a round trip: *"a format test is not a location test"*,
/// because a defect that files the arrangement in the wrong directory entirely
/// is invisible to a test that writes and reads through the same wrong path.
/// The same hole is open one layer up — this file chooses the name it hands
/// over — so the same assertion is made here, about
/// `arrangements/<name>.arrangement.json` under the store's root.
#[test]
fn an_arrangement_is_kept_at_the_path_the_stores_header_names() {
    let root = arrangement_root("kept");
    let mut panel = arranged(1280.0, 720.0);

    let said = arrangement(
        &root,
        &mut panel,
        &mut view::Arrangement::default(),
        &Operation::SaveArrangement {
            name: "four_deck".to_owned(),
        },
    )
    .expect("a save is one of the two operations this route answers for");

    let at = root.join("arrangements").join("four_deck.arrangement.json");
    let bytes = std::fs::read(&at).unwrap_or_else(|e| {
        panic!(
            "nothing at {} after `{said}` — a saved arrangement is one path component of \
             name, one of what it is, and one of the format it is in, under the store's \
             fourth directory: {e}",
            at.display()
        )
    });

    // And what is at that path is this panel's arrangement rather than
    // some other document that happens to be there.
    let back: Layout =
        serde_json::from_slice(&bytes).expect("the bytes at that path are an arrangement");
    assert!(
        back.is_soloed(),
        "the file at {} did not carry the solo the panel was saved with",
        at.display()
    );

    // Nothing else was created under the store: an arrangement is a fourth
    // thing beside `sets/`, `sessions/` and the artifacts, and not one of
    // them.
    assert!(
        !root.join("sets").join("four_deck.kbset").exists(),
        "the arrangement was filed as a Set"
    );

    std::fs::remove_dir_all(&root).expect("clean up");
}

/// What comes back is the arrangement that was kept, in the window it arrives
/// in — which is `Op::Reset` carrying the viewport across, with the arrangement
/// handed in rather than built.
///
/// The two viewports differ in both axes on purpose: an arrangement carries the
/// viewport it was saved at, so a restore that took the file's would open a
/// console arranged on a desktop inside a smaller window with every rectangle
/// past the edge.
#[test]
fn an_arrangement_put_back_arrives_in_the_window_this_one_already_has() {
    let root = arrangement_root("back");
    let mut saved = arranged(1920.0, 1080.0);
    arrangement(
        &root,
        &mut saved,
        &mut view::Arrangement::default(),
        &Operation::SaveArrangement {
            name: "night_b".to_owned(),
        },
    )
    .expect("a save");

    // A window of a different size, with nothing folded and nothing soloed.
    let mut window = Panel::new(1280.0, 720.0);
    let staging = window.layout().find("staging").expect("a staging bay");
    assert!(!window.layout().is_collapsed(staging));

    let said = arrangement(
        &root,
        &mut window,
        &mut view::Arrangement::default(),
        &Operation::RestoreArrangement {
            name: "night_b".to_owned(),
        },
    )
    .expect("a restore");

    let now = window.layout();
    assert_eq!(
        (now.viewport().w, now.viewport().h),
        (1280.0, 720.0),
        "`{said}` — the arrangement brought the window it was saved at with it. The window \
         is the operator's and never the file's"
    );
    assert!(
        now.is_soloed() && now.is_collapsed(now.find("staging").expect("staging")),
        "`{said}` — the fold and the solo did not come back, so what was put back is not \
         what was kept"
    );

    std::fs::remove_dir_all(&root).expect("clean up");
}

/// A name nothing is filed under is said back, and the console does not move.
///
/// The refusal that matters most in this family: `read_arrangement` never falls
/// back to the built-in, so an operator who mistyped a name is told the name
/// rather than watching their console reset (ADR-0221 §2). Asked three ways,
/// because the three failures send an operator to three different places — no
/// store at all, no such name, and a file that will not read back.
#[test]
fn a_name_nothing_is_filed_under_is_said_back_and_nothing_resets() {
    let root = arrangement_root("refused");
    let mut panel = arranged(1280.0, 720.0);
    let kept: Vec<bool> = panel
        .nodes()
        .iter()
        .map(|n| panel.layout().is_collapsed(n.id))
        .collect();
    let unchanged = |panel: &Panel, said: &str| {
        let now: Vec<bool> = panel
            .nodes()
            .iter()
            .map(|n| panel.layout().is_collapsed(n.id))
            .collect();
        assert_eq!(
            now, kept,
            "`{said}` and the arrangement moved — a refusal that resets the console is the \
             one thing `read_arrangement` promises never to do"
        );
        assert!(
            panel.layout().is_soloed(),
            "`{said}` and the solo went — see above"
        );
    };

    // 1. No store at all, and asking a question does not make one.
    let said = arrangement(
        &root,
        &mut panel,
        &mut view::Arrangement::default(),
        &Operation::RestoreArrangement {
            name: "four_deck".to_owned(),
        },
    )
    .expect("a restore");
    assert!(
        said.contains("four_deck"),
        "the refusal did not say the name back: {said}"
    );
    assert!(
        !root.exists(),
        "putting an arrangement back that is not there created a store at {}",
        root.display()
    );
    unchanged(&panel, &said);

    // 2. A store, and no such name in it.
    Store::open(&root).expect("a store");
    let said = arrangement(
        &root,
        &mut panel,
        &mut view::Arrangement::default(),
        &Operation::RestoreArrangement {
            name: "four_deck".to_owned(),
        },
    )
    .expect("a restore");
    assert!(
        said.contains("four_deck"),
        "the refusal did not say the name back: {said}"
    );
    unchanged(&panel, &said);

    // 3. A file filed under the name that is not an arrangement. Refused
    //    whole rather than repaired (ADR-0158), and told apart from
    //    *there is no such arrangement*, which is the distinction the
    //    sentence carries.
    Store::open(&root)
        .expect("a store")
        .write_arrangement("four_deck", b"{\"nodes\":[]}")
        .expect("bytes the store does not have to understand");
    let said = arrangement(
        &root,
        &mut panel,
        &mut view::Arrangement::default(),
        &Operation::RestoreArrangement {
            name: "four_deck".to_owned(),
        },
    )
    .expect("a restore");
    assert!(
        said.contains("disagrees with itself"),
        "a file that will not read back was reported as a missing arrangement, which sends \
         an operator looking for a name they typed correctly: {said}"
    );
    unchanged(&panel, &said);

    std::fs::remove_dir_all(&root).expect("clean up");
}

// -- the arrangement pill's half of the family ----------------------

/// A name that is not one path component is refused here, which is the
/// authority the pill deliberately does not hold (P-0090).
///
/// The negative control is the point: a check that refused everything would
/// pass an assertion that only ever looked for a refusal, so the names that
/// must be *accepted* are asserted beside the ones that must not
/// (`docs/contributing.md` §3).
#[test]
fn a_typed_arrangement_name_is_refused_where_the_file_is_written() {
    for good in ["night", "four_deck", "set-2", "A9"] {
        assert!(
            checked_name(good).is_ok(),
            "`{good}` is letters, digits, `-` and `_`, and was refused"
        );
    }
    for (bad, why) in [
        ("", "nothing was typed"),
        ("../../elsewhere", "a path"),
        ("night deck", "a space"),
        ("night.json", "a suffix of its own"),
    ] {
        let refusal = checked_name(bad).expect_err(&format!("`{bad}` is {why} and was kept"));
        assert!(
            refusal.starts_with("arrangement: "),
            "the refusal does not say what it is about: {refusal}"
        );
        assert!(
            bad.is_empty() || refusal.contains(bad),
            "the refusal does not say the name back, so an operator cannot see what \
             they typed: {refusal}"
        );
    }
}

/// The name in use follows the file and never the press.
///
/// A save that landed and a restore that landed each make that arrangement the
/// one in use, so the pill names it; a save that was refused leaves the pill
/// saying what it said, because nothing under that name is on the disk. And the
/// menu's listing gains the new name only where a file appeared.
#[test]
fn the_pill_names_the_arrangement_only_once_the_file_is_there() {
    let root = arrangement_root("in-use");
    let mut panel = arranged(1280.0, 720.0);
    let mut arr = view::Arrangement::NONE;

    // Refused: the name is not one path component, so nothing was filed
    // and nothing is in use.
    let said = arrangement(
        &root,
        &mut panel,
        &mut arr,
        &Operation::SaveArrangement {
            name: "night/one".to_owned(),
        },
    )
    .expect("a save is one of the operations this route answers for");
    assert!(said.contains("holds `/`"), "{said}");
    assert_eq!(arr.name, None, "a refused save put a name on the pill");
    assert!(arr.filed.is_empty());

    // Kept: in use, and listed.
    arrangement(
        &root,
        &mut panel,
        &mut arr,
        &Operation::SaveArrangement {
            name: "night".to_owned(),
        },
    )
    .expect("a save");
    assert_eq!(arr.name.as_deref(), Some("night"));
    assert_eq!(arr.filed, vec!["night".to_owned()]);

    // A restore of a name nothing is filed under is refused where the
    // bytes are, and leaves the pill alone.
    let said = arrangement(
        &root,
        &mut panel,
        &mut arr,
        &Operation::RestoreArrangement {
            name: "rehearsal".to_owned(),
        },
    )
    .expect("a restore");
    assert!(said.contains("rehearsal"), "{said}");
    assert_eq!(
        arr.name.as_deref(),
        Some("night"),
        "a refused restore moved the name the pill is showing"
    );

    // And one that is filed does put it in use.
    arrangement(
        &root,
        &mut panel,
        &mut arr,
        &Operation::RestoreArrangement {
            name: "night".to_owned(),
        },
    )
    .expect("a restore");
    assert_eq!(arr.name.as_deref(), Some("night"));

    std::fs::remove_dir_all(&root).expect("clean up");
}

/// A reset takes the name off the pill, whichever surface asked.
///
/// `r` and the menu's *start a new one* are one operation and `Readout::op` is
/// where both arrive, so this is asserted through the method rather than
/// through either control: the default arrangement is what is on screen and the
/// default has no name.
#[test]
fn a_reset_leaves_the_pill_naming_no_file() {
    let mut readout = Readout::new(1280.0, 720.0);
    readout.view.arrangement.name = Some("night".to_owned());

    // An operation that is not a reset leaves it alone, which is what says
    // the clearing is the reset's and not every operation's.
    let staging = readout.panel.layout().find("staging").expect("staging");
    readout.op(Op::Fold(staging));
    assert_eq!(readout.view.arrangement.name.as_deref(), Some("night"));

    assert_eq!(readout.op(Op::Reset), Outcome::Reset);
    assert_eq!(
        readout.view.arrangement.name, None,
        "the console was reset to the default and the pill still names a file"
    );
}

/// What the load control's five asks do to this program, and that a pick moves
/// this bay's mark and nothing else (ADR-0305).
///
/// [`every_ask_the_pill_makes_is_acted_on_and_shuts_the_menu`]'s shape one bay
/// along, and it is here rather than in `karakuri-console` for the reason that
/// test is: `Readout::aimed` is the host's half of the seam, and the console's
/// own tests cannot reach it.
///
/// The claim a reader will doubt is the third one. *Surely picking a deck
/// selects it* — and it must not: `Operation::SelectDeck` moves the keys, and
/// this mark is the one that is allowed to name another deck. So the selection
/// is read before and after.
#[test]
fn every_ask_the_load_control_makes_is_acted_on_and_moves_only_this_bays_mark() {
    let mut readout = Readout::new(1440.0, 900.0);
    readout.panel.solve();
    readout.view.mixer = (0..4)
        .map(|slot| view::Strip {
            name: format!("slot{slot}"),
            tally: view::Tally::Live,
            requested: view::Tally::Live,
            gain: 0.5,
            gain_to: None,
            opacity: 0.5,
            opacity_to: None,
            blend: BlendMode::Over,
            mask: view::Mask::None,
            mask_angle: 0.0,
            level: None,
            is_muted: false,
            is_soloed: false,
        })
        .collect();
    assert!(readout.view.select(1), "the keys did not go to deck B");

    assert_eq!(readout.aimed(Aim::Open), Acted::Nothing);
    assert!(readout.view.target_open(), "the list did not come down");
    assert_eq!(readout.aimed(Aim::Shut), Acted::Nothing);
    assert!(!readout.view.target_open());

    // **A pick names a deck, asks for nothing, and puts the list away.**
    readout.aimed(Aim::Open);
    assert_eq!(readout.aimed(Aim::Deck(2)), Acted::Nothing);
    assert_eq!(
        readout.view.target_deck(),
        2,
        "the pick did not aim the load"
    );
    assert_eq!(
        readout.view.selection(),
        1,
        "a pick in the pulldown moved the deck selection, which is the one thing this \
         control must not do"
    );
    assert!(
        !readout.view.target_open(),
        "the list stayed down after a pick"
    );

    // **And the load goes down the path the key and the drop take.**
    let want = Operation::LoadSet {
        deck: 2,
        set: "drift_night".to_owned(),
    };
    assert_eq!(
        readout.aimed(Aim::Load(want.clone())),
        Acted::Emitted(Some(want)),
        "the load did not go down the path every other emitted operation takes"
    );
    assert_eq!(
        readout.aimed(Aim::NoSet),
        Acted::Nothing,
        "a press with no Set under the cursor emitted something"
    );
}

/// What the menu's five asks do to this program, and that every one of them
/// that acts shuts the card.
#[test]
fn every_ask_the_pill_makes_is_acted_on_and_shuts_the_menu() {
    let mut readout = Readout::new(1280.0, 720.0);
    readout.view.arrangement.filed = vec!["night".to_owned()];

    assert!(matches!(readout.arranged(Ask::Open), Acted::Nothing));
    assert!(readout.view.arrangement.open());
    assert!(matches!(readout.arranged(Ask::Shut), Acted::Nothing));
    assert!(!readout.view.arrangement.open());

    readout.arranged(Ask::Open);
    assert!(matches!(readout.arranged(Ask::Name), Acted::Nothing));
    assert_eq!(
        readout.view.arrangement.naming(),
        Some(""),
        "the one item that asks for letters left nothing asking for any"
    );

    readout.arranged(Ask::Open);
    let did = readout.arranged(Ask::Panel(Op::Reset));
    assert!(
        matches!(did, Acted::Operated(Outcome::Reset)),
        "the reset was not performed: {did:?}"
    );
    assert!(
        !readout.view.arrangement.open(),
        "the card is still standing"
    );

    readout.arranged(Ask::Open);
    let want = Operation::RestoreArrangement {
        name: "night".to_owned(),
    };
    let did = readout.arranged(Ask::Operation(want.clone()));
    assert_eq!(
        did,
        Acted::Emitted(Some(want)),
        "the operation did not go down the path every other emitted operation takes"
    );
    assert!(!readout.view.arrangement.open());
}

/// The four presses the Sequencer bay performs, and the one thing this window
/// does with a pattern that no test in `karakuri-console` can see: that bay
/// hands back an operation and applies nothing, so this is the other side of
/// that seam.
///
/// A press names a bank, and a bank this session does not have is refused and
/// said out loud — [`pointed`]'s rule, and the reason each arm answers a
/// sentence rather than `None`.
#[test]
fn a_sequencer_press_moves_the_pattern_it_names() {
    let mut banks = demonstration_banks();
    let mut playhead = karakuri_pattern::Playhead::default();
    assert!(
        banks.pattern().lanes()[0].muted(),
        "a run starts with the lane muted, so nothing writes deck A's fader until a hand asks"
    );
    // The mute, taken back.
    assert!(sequenced(
        &mut banks,
        &mut playhead,
        &View::new(Room::Day),
        &Operation::SetLaneMute {
            pattern: 0,
            lane: 0,
            muted: false,
        }
    )
    .is_some());
    assert!(!banks.pattern().lanes()[0].muted());
    // A step, set rather than flipped.
    for on in [true, true, false] {
        assert!(sequenced(
            &mut banks,
            &mut playhead,
            &View::new(Room::Day),
            &Operation::SetStep {
                pattern: 0,
                lane: 0,
                step: 4,
                on,
            }
        )
        .is_some());
        assert_eq!(
            banks.pattern().lanes()[0].slot_on(4),
            on,
            "a press asks for a state, so asking twice for the same one leaves it there"
        );
    }
    // A slot the mode press must not touch, and it is slot 5 — an odd one,
    // which an eighth does not read at all and which is therefore the slot
    // a store sized to the count would have thrown away.
    assert!(sequenced(
        &mut banks,
        &mut playhead,
        &View::new(Room::Day),
        &Operation::SetStep {
            pattern: 0,
            lane: 0,
            step: 5,
            on: true,
        }
    )
    .is_some());
    // The mode, which changes a reading and forgets where the playhead
    // was, because the index it remembered is about a reading that has
    // gone.
    playhead.advance(banks.pattern(), 0.0);
    assert!(playhead.at().is_some());
    assert!(sequenced(
        &mut banks,
        &mut playhead,
        &View::new(Room::Day),
        &Operation::SetPatternGrid {
            pattern: 0,
            grid: karakuri_operation::StepMode::Eighth,
        }
    )
    .is_some());
    assert_eq!(banks.pattern().mode().count(), 8);
    assert_eq!(
        playhead.at(),
        None,
        "a mode press is the same bar at another width, so the next poll is a boundary"
    );
    assert!(
        banks.pattern().lanes()[0].slot_on(5),
        "and the sixteen slots underneath are untouched — including the odd ones an eighth \
         does not read"
    );
    // A bank, and one this session does not have.
    assert!(sequenced(
        &mut banks,
        &mut playhead,
        &View::new(Room::Day),
        &Operation::SelectPattern { pattern: 3 }
    )
    .is_some());
    assert_eq!(banks.armed(), 3);
    assert!(
        banks.pattern().is_empty(),
        "bank 3 is one of the three empty ones"
    );
    let refused = sequenced(
        &mut banks,
        &mut playhead,
        &View::new(Room::Day),
        &Operation::SelectPattern { pattern: 9 },
    )
    .expect("a press this session cannot perform says so rather than going quiet");
    assert!(refused.contains("there is no bank 9"));
    assert_eq!(banks.armed(), 3, "and a refused press moves nothing");
}

/// A lane arrives with its two levels filled in from the range the console was
/// published, and it arrives muted.
///
/// This is the half of `+ lane` no test in `karakuri-console` can see: that bay
/// hands back `Operation::PointLane { pattern, target }` and appends nothing,
/// and the payload carries no levels — a fader's are 1.0 and 0.0 and a
/// parameter's are the range `View::inspector` holds, which is the same reading
/// the chooser drew its items from (ADR-0320, ADR-0327).
#[test]
fn a_pointed_lane_takes_its_levels_from_the_published_range() {
    let mut banks = karakuri_pattern::Banks::default();
    let mut playhead = karakuri_pattern::Playhead::default();
    let param = karakuri_operation::ParamAt {
        node: Some(karakuri_operation::NodeAddress {
            layer: karakuri_operation::Layer::L2,
            index: 0,
        }),
        key: "twist".to_owned(),
    };
    let mut view = View::new(Room::Day);
    view.inspector = vec![view::Pane {
        deck: 1,
        material: "lattice_veil".to_owned(),
        sync: karakuri_operation::Sync::Free,
        allows: [true; view::SYNCS.len()],
        anchor_bpm: 128.0,
        scrub_beats: 0.0,
        composite: false,
        // Not this test's row: the deck head's two build chips are
        // drawn from this and nothing here is about them.
        aimed: None,
        nodes: vec![view::Node {
            addr: "L2:0".to_owned(),
            name: "warp".to_owned(),
            authority: None,
            // Not this test's control either: nothing here presses a
            // node head's `keep`.
            keep: None,
            uses: Vec::new(),
            renderers: Vec::new(),
            params: vec![view::Param {
                ord: Some(1),
                name: "twist".to_owned(),
                value: 1.0,
                // **Not `[0, 1]`**, so a range that was read and a pair
                // that was assumed cannot look alike.
                range: [0.25, 4.0],
                param: param.clone(),
                bound: None,
            }],
        }],
    }];

    // A fader's pair is the gate, and no reading is consulted for it.
    let line = sequenced(
        &mut banks,
        &mut playhead,
        &view,
        &Operation::PointLane {
            pattern: 0,
            target: karakuri_operation::LaneTarget::Fader { deck: 0 },
        },
    )
    .expect("a lane press says what it did");
    assert!(line.contains("on 1 off 0"), "a fader's gate: {line}");
    let lane = &banks.pattern().lanes()[0];
    assert_eq!((lane.on(), lane.off()), (1.0, 0.0));
    assert!(
        lane.muted(),
        "a lane arrives with every slot off, and an off step writes `off` — so an unmuted \
         fader lane would hold its deck at zero from the press"
    );

    // A parameter's pair is the published range, top then bottom.
    assert!(sequenced(
        &mut banks,
        &mut playhead,
        &view,
        &Operation::PointLane {
            pattern: 0,
            target: karakuri_operation::LaneTarget::Param {
                deck: 1,
                param: param.clone(),
            },
        }
    )
    .is_some());
    let lane = &banks.pattern().lanes()[1];
    assert_eq!(
        (lane.on(), lane.off()),
        (4.0, 0.25),
        "an on step writes the top of the published range and an off step the bottom"
    );

    // **A control this console holds no row for is refused and said**,
    // rather than defaulted into two numbers nobody chose.
    let refused = sequenced(
        &mut banks,
        &mut playhead,
        &view,
        &Operation::PointLane {
            pattern: 0,
            target: karakuri_operation::LaneTarget::Param {
                deck: 3,
                param: param.clone(),
            },
        },
    )
    .expect("a press this window cannot perform says so rather than going quiet");
    assert!(refused.contains("no published range"), "{refused}");
    assert_eq!(
        banks.pattern().lanes().len(),
        2,
        "and a refused press appends nothing"
    );
}

/// A lane is taken out by its position, the lanes after it move up, and an index
/// this pattern has not got is refused in `no_such_slot`'s own sentence.
///
/// The other half of the minus at the end of a lane's row: the bay hands back
/// `Operation::RemoveLane { pattern, lane }` and takes nothing out itself, so
/// this is where the pattern actually loses the row (ADR-0352's property read on
/// a lane).
#[test]
fn a_removed_lane_takes_its_steps_with_it_and_the_rest_move_up() {
    let mut banks = karakuri_pattern::Banks::default();
    let mut playhead = karakuri_pattern::Playhead::default();
    let view = View::new(Room::Day);
    for deck in 0..3 {
        assert!(sequenced(
            &mut banks,
            &mut playhead,
            &view,
            &Operation::PointLane {
                pattern: 0,
                target: karakuri_operation::LaneTarget::Fader { deck },
            }
        )
        .is_some());
    }
    assert_eq!(banks.pattern().lanes().len(), 3);

    let line = sequenced(
        &mut banks,
        &mut playhead,
        &view,
        &Operation::RemoveLane {
            pattern: 0,
            lane: 1,
        },
    )
    .expect("a lane press says what it did");
    assert!(line.contains("taken out"), "{line}");
    assert!(
        line.contains("no record"),
        "a pattern is not a session record: {line}"
    );
    assert_eq!(banks.pattern().lanes().len(), 2);
    assert_eq!(
        banks.pattern().lanes()[1].target(),
        &karakuri_operation::LaneTarget::Fader { deck: 2 },
        "the lanes after the removed one move up, which is what a lane index means"
    );

    // **An index this pattern has not got is refused in the words every
    // surface refuses an address in**: the thing named, then what there was to
    // name (`karakuri_environment::no_such_slot`).
    let refused = sequenced(
        &mut banks,
        &mut playhead,
        &view,
        &Operation::RemoveLane {
            pattern: 0,
            lane: 7,
        },
    )
    .expect("a press this window cannot perform says so rather than going quiet");
    assert!(
        refused.contains("no lane 7") && refused.contains("holds lanes 0-1"),
        "{refused}"
    );
    assert_eq!(
        banks.pattern().lanes().len(),
        2,
        "and a refused press takes nothing out"
    );

    // A bank this session does not hold is the other refusal, and it is the
    // one every arm of this handler makes.
    let refused = sequenced(
        &mut banks,
        &mut playhead,
        &view,
        &Operation::RemoveLane {
            pattern: 9,
            lane: 0,
        },
    )
    .expect("a press naming no bank says so");
    assert!(
        refused.contains("not a bank this session holds"),
        "{refused}"
    );

    // **The lane that is driving comes out too**, and nothing refuses it: a
    // lane's writes are its whole record, so they stop here (ADR-0322).
    assert!(sequenced(
        &mut banks,
        &mut playhead,
        &view,
        &Operation::SetLaneMute {
            pattern: 0,
            lane: 0,
            muted: false,
        }
    )
    .is_some());
    assert_eq!(
        banks.pattern().held().next(),
        Some((0, &karakuri_operation::LaneTarget::Fader { deck: 0 })),
        "an unmuted lane holds the control it drives"
    );
    let line = sequenced(
        &mut banks,
        &mut playhead,
        &view,
        &Operation::RemoveLane {
            pattern: 0,
            lane: 0,
        },
    )
    .expect("a lane press says what it did");
    assert!(line.contains("It was driving"), "{line}");
    assert_eq!(
        banks.pattern().held().count(),
        0,
        "and nothing holds deck A's fader once the lane that did is gone"
    );

    // The last lane out leaves an empty pattern, and the refusal then says
    // there is nothing to name.
    assert!(sequenced(
        &mut banks,
        &mut playhead,
        &view,
        &Operation::RemoveLane {
            pattern: 0,
            lane: 0,
        }
    )
    .is_some());
    assert!(banks.pattern().is_empty());
    let refused = sequenced(
        &mut banks,
        &mut playhead,
        &view,
        &Operation::RemoveLane {
            pattern: 0,
            lane: 0,
        },
    )
    .expect("a press on an empty pattern says so");
    assert!(refused.contains("this pattern holds none"), "{refused}");
}

/// A finished name is one operation of the vocabulary, and the card is gone
/// before it is emitted — whether or not the name is any good, since the
/// refusal is said out loud by `checked_name` and a card left standing over it
/// would be the panel asking again without saying the answer.
#[test]
fn a_finished_name_is_the_save_the_menu_would_have_asked_for() {
    let mut readout = Readout::new(1280.0, 720.0);
    assert_eq!(
        readout.named(),
        Acted::Nothing,
        "a console with nothing being typed committed a name"
    );

    readout.arranged(Ask::Name);
    for c in "four_deck".chars() {
        assert!(readout.view.arrangement.typed(c));
    }
    assert_eq!(
        readout.named(),
        Acted::Emitted(Some(Operation::SaveArrangement {
            name: "four_deck".to_owned()
        }))
    );
    assert!(!readout.view.arrangement.open());

    // An empty name is emitted as one and refused where the file is
    // written, rather than being swallowed here.
    readout.arranged(Ask::Name);
    assert_eq!(
        readout.named(),
        Acted::Emitted(Some(Operation::SaveArrangement {
            name: String::new()
        }))
    );
}

/// The menu's list is the store's, and a store that is not there is listed as
/// nothing and is not created — `library`'s two rules over the fourth
/// directory.
#[test]
fn the_menu_lists_the_store_and_makes_none() {
    let root = arrangement_root("listing");
    assert!(
        arrangements(&root).is_empty(),
        "a store that is not there listed something"
    );
    assert!(
        !root.exists(),
        "listing the arrangements created a store at {}",
        root.display()
    );

    let mut panel = arranged(1280.0, 720.0);
    for name in ["rehearsal", "four_deck"] {
        keep_arrangement(&root, &panel, name);
    }
    panel.solve();
    assert_eq!(
        arrangements(&root),
        vec!["four_deck".to_owned(), "rehearsal".to_owned()],
        "the menu lists what the store holds, in the order the store sorts it"
    );

    std::fs::remove_dir_all(&root).expect("clean up");
}
