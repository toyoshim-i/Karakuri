use super::*;

fn watch_on(dir: &std::path::Path) -> Watch {
    Watch::new(
        0,
        crate::compile::Named::bare(dir.join("a.kir")),
        vec![crate::compile::Named::bare(dir.join("b.kir"))],
        karakuri_engine::set::Layering::Overdraw,
        None,
        Some(4096),
        1,
        vec![1],
        karakuri_engine::camera::Orbit::default(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
    )
}

/// A save that changed no bytes is not an edit. Under an mtime comparison
/// `touch`, a formatter, or a `git checkout` of the branch already checked out
/// each restarts the visual from `t` zero for nothing.
#[test]
fn rewriting_identical_bytes_is_not_a_change() {
    // **A directory of its own, not a fixed name under `/tmp`.** Two runs
    // of this suite at once — a `pre-push` hook beside a terminal, say —
    // shared the fixed one, and each deleted the other's files mid-test.
    // It failed once in a whole-workspace run and passed every time it was
    // run alone, which is what that shape looks like from the outside.
    let tmp = tempfile::tempdir().expect("temp dir");
    let dir = tmp.path();
    std::fs::write(dir.join("a.kir"), "proc a {}").expect("write");
    std::fs::write(dir.join("b.kir"), "proc b {}").expect("write");

    let w = watch_on(dir);
    let before = w.stamp();
    std::thread::sleep(Duration::from_millis(10));
    std::fs::write(dir.join("a.kir"), "proc a {}").expect("rewrite");
    assert_eq!(before, w.stamp(), "identical bytes read as a change");

    std::fs::write(dir.join("a.kir"), "proc a { }").expect("edit");
    assert_ne!(before, w.stamp(), "a real edit read as unchanged");
}

/// The examples this suite sorts, copied into a directory of their own so that
/// a watcher can be built over paths that do not exist yet.
///
/// Built before the files are written, which is what makes writing them the
/// edit it wakes on: a missing file stamps as `None`, and appearing is a change
/// like any other. The alternative is editing a file's text, which would make
/// the two paths sort different bytes.
fn watch_over(dir: &std::path::Path, files: &[&str]) -> (Watch, Vec<PathBuf>) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let paths: Vec<PathBuf> = files.iter().map(|f| dir.join(f)).collect();
    let watch = Watch::new(
        0,
        crate::compile::Named::bare(paths[0].clone()),
        paths[1..]
            .iter()
            .cloned()
            .map(crate::compile::Named::bare)
            .collect(),
        karakuri_engine::set::Layering::Overdraw,
        None,
        Some(4096),
        1,
        vec![1],
        karakuri_engine::camera::Orbit::default(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
    );
    for (file, path) in files.iter().zip(&paths) {
        std::fs::copy(root.join("examples").join(file), path).expect("copy an example");
    }
    (watch, paths)
}

/// Polled until it answers, or four intervals, whichever is first. One poll
/// sees the change and the next acts on it — see "Debouncing" — so nothing by
/// the fourth is nothing at all.
fn polled(watch: &mut Watch) -> Option<Polled> {
    std::iter::repeat_with(|| watch.poll())
        .take(4)
        .flatten()
        .next()
}

/// [`polled`], for the tests that are about what a build states.
///
/// A refusal panics with its diagnostics rather than reading as nothing. A
/// watcher that stopped compiling would otherwise turn every assertion below
/// into `expect("a build")` on a `None`, which says the files never settled and
/// is the wrong end of the failure entirely.
fn rebuild(watch: &mut Watch) -> Option<Request> {
    match polled(watch)? {
        Polled::Build(request) => Some(request),
        Polled::Refused(refusal) => panic!(
            "the checker turned `{}` down: {}",
            refusal.label,
            refusal.said.join("; ")
        ),
    }
}

/// A `.kir` the checker turns down is an answer and not a silence, and the
/// answer carries every diagnostic the checker had.
///
/// This watcher printed its diagnostics and returned `None` until 2026-09-08,
/// and `None` is what a poll that saw nothing returns — so the operator's
/// newest edit disagreeing with the picture was said on a terminal and reached
/// no surface at all (`docs/adr/0310-…`). The assertion is therefore about the
/// *shape* of the answer first and its contents second: a refusal, with the
/// file it is about and with what the checker said in it.
///
/// The negative control is the same watcher afterwards. A version that refused
/// everything would pass every assertion above the repair; the repair is what
/// says the refusal was about the bytes.
///
/// The fixture is prepended to rather than substituted in. Both files are ones
/// this product can rewrite — `write_procedure` reaches them over MCP — so what
/// is written here has to break them whatever they contain, which a line that
/// is not a declaration does and a substitution does not
/// (`docs/contributing.md` §3).
#[test]
fn a_file_the_checker_turns_down_is_a_refusal_carrying_its_diagnostics() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let (mut watch, paths) = watch_over(tmp.path(), &["drift_shell.kir", "soft_points.kir"]);
    rebuild(&mut watch).expect("the examples this slot was pointed at compile");

    let good = std::fs::read_to_string(&paths[0]).expect("read the head");
    std::fs::write(&paths[0], format!("not a declaration\n{good}")).expect("break the head");
    let refusal = match polled(&mut watch) {
        Some(Polled::Refused(refusal)) => refusal,
        Some(Polled::Build(request)) => {
            panic!(
                "a file that does not check was built as `{}`",
                request.label
            )
        }
        None => panic!(
            "a file that does not check produced nothing at all, which is what a poll \
                 that saw no edit produces"
        ),
    };
    assert!(
        refusal.label.contains("drift_shell"),
        "the refusal names `{}` and the file that does not check is drift_shell.kir",
        refusal.label
    );
    assert!(
        !refusal.said.is_empty(),
        "a refusal with nothing in it is the silence this replaced"
    );
    // Where, which stage, and what — the head of what the terminal is
    // printing, which is `compile::Diagnostics::said`'s own claim.
    assert!(
        refusal.said[0].starts_with("1:1: parse: "),
        "the first diagnostic is `{}` and the broken line is the first one",
        refusal.said[0]
    );
    assert!(
        refusal.said.iter().all(|line| !line.contains('\n')),
        "a diagnostic on this list is a line, and one of these is a block: {:?}",
        refusal.said
    );

    // **The negative control.** The same watcher, the same slot, the file
    // as it was: a build, which is what says the refusal above was the
    // bytes and not this watcher having given up.
    std::fs::write(&paths[0], &good).expect("repair the head");
    let request = rebuild(&mut watch).expect("the repaired file compiles");
    assert!(
        request.label.contains("drift_shell"),
        "the repaired slot built `{}`",
        request.label
    );
}

/// A rebuild restates the salts the slot is running at, rather than leaving
/// them to be derived where the Set is built.
///
/// A slot filled by `--load-set` runs at the salts its file recorded, and
/// nothing in a `.kir` says what they are. A request that left them out hands
/// `Set::build_many` an empty list, which derives from the ordinal — so every
/// element in the slot would change colour on the next save of a file that had
/// nothing to do with the geometry, and the Set an operator loaded would stop
/// being the Set they loaded partway through an edit.
#[test]
fn a_rebuild_restates_the_salts_the_slot_is_running_at() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let tmp = tempfile::tempdir().expect("temp dir");
    // Two geometries, so that "each keeps its own" is a claim at all.
    let files = ["drift_shell.kir", "lattice_shell.kir", "soft_points.kir"];
    let paths: Vec<PathBuf> = files.iter().map(|f| tmp.path().join(f)).collect();
    // Numbers no derivation produces, so a request that derived its own
    // cannot pass by accident.
    let salts: Vec<u32> = vec![0x0bad_cafe, 0x1234_5678];
    let mut watch = Watch::new(
        0,
        crate::compile::Named::bare(paths[0].clone()),
        paths[1..]
            .iter()
            .cloned()
            .map(crate::compile::Named::bare)
            .collect(),
        karakuri_engine::set::Layering::Overdraw,
        None,
        Some(4096),
        salts[0],
        salts.clone(),
        karakuri_engine::camera::Orbit::default(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
    );
    for (file, path) in files.iter().zip(&paths) {
        std::fs::copy(root.join("examples").join(file), path).expect("copy an example");
    }

    let request = rebuild(&mut watch).expect("a build");
    assert_eq!(request.l1s.len(), 2, "two geometries in the slot");
    assert_eq!(
        request.salts,
        vec![Some(salts[0]), Some(salts[1])],
        "a rebuild handed the engine salts other than the ones the slot is running at"
    );
}

/// The values a slot was aimed with are stated once, and a later rebuild states
/// none of them — which is the whole of what
/// `karakuri_engine::Set::carry_moved_from` needed from this side.
///
/// It is the one field on a request that stopped being restated, and the reason
/// is that it is the one the engine does not re-derive: a salt, a camera, a
/// fold and an edge all come back at some default the moment a request stops
/// naming them, and a parameter value does not — the outgoing Set is holding
/// it. Restating them made a knob unturnable, and worst exactly where an
/// operator is most likely to be turning one: a slot pointed at a Set file
/// carries every declaration of every node here, because that is what a live
/// save writes, so every parameter went back to the file on the next save of
/// any `.kir` and nothing an operator did to a live Set survived it.
///
/// Both halves are asserted from one watcher, because either alone is the wrong
/// rule: a watcher that never stated them would build the aimed Set without the
/// values it was aimed with, and one that always stated them is what this
/// replaces.
#[test]
fn the_values_a_slot_was_aimed_with_are_stated_once() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let tmp = tempfile::tempdir().expect("temp dir");
    let files = ["drift_shell.kir", "soft_points.kir"];
    let paths: Vec<PathBuf> = files.iter().map(|f| tmp.path().join(f)).collect();
    for (file, path) in files.iter().zip(&paths) {
        std::fs::copy(root.join("examples").join(file), path).expect("copy an example");
    }
    let named = |at: usize| crate::compile::Named::bare(paths[at].clone());
    let aimed_with = vec![karakuri_engine::ParamWrite::everywhere("exposure", 0.125)];

    let (aim, aimed) = std::sync::mpsc::channel();
    let mut watch = Watch::new(
        0,
        named(0),
        vec![named(1)],
        karakuri_engine::set::Layering::Overdraw,
        None,
        Some(4096),
        1,
        vec![1],
        karakuri_engine::camera::Orbit::default(),
        // **What this watcher was constructed with, and it is never
        // stated**: the caller built the live Set with these and handed it
        // over, so the first save owes the operator whatever is on the Set
        // rather than whatever the flags said before the run started.
        aimed_with.clone(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
    )
    .aimed_by(aimed);

    std::fs::write(&paths[0], format!("// a save\n{}", read(&paths[0]))).expect("edit");
    let request = rebuild(&mut watch).expect("a build");
    assert!(
        request.params.is_empty(),
        "a save restated the values the run was started with, so a knob ridden \
             since could not survive it"
    );

    aim.send(Aim {
        head: named(0),
        rest: vec![named(1)],
        layering: karakuri_engine::set::Layering::Overdraw,
        live: None,
        capacity: Some(4096),
        seed_salt: 1,
        salts: vec![1],
        camera: karakuri_engine::camera::Orbit::default(),
        overrides: aimed_with.clone(),
        published: Vec::new(),
        bindings: Vec::new(),
        edges: Vec::new(),
        authorities: Vec::new(),
        set: None,
    })
    .expect("the watcher is alive");
    let request = rebuild(&mut watch).expect("the aim's own build");
    assert_eq!(
        request.params.len(),
        1,
        "the aim's own build has nothing to inherit from — the Set it describes \
             does not exist yet — so it is the one build that has to state them"
    );

    std::fs::write(&paths[0], format!("// another save\n{}", read(&paths[0]))).expect("edit");
    let request = rebuild(&mut watch).expect("a build");
    assert!(
        request.params.is_empty(),
        "the aim's values were stated a second time, which pins every one of \
             them to the file for the rest of the run"
    );
}

fn read(path: &std::path::Path) -> String {
    std::fs::read_to_string(path).expect("read back")
}

/// A rebuild restates the camera the slot is aimed with, rather than leaving
/// the Set it builds to start from `Orbit::default()`.
///
/// Nothing in a `.kir` says where the built-in orbit is pointing — a `camera`
/// record does, and `--load-set` is what brings one in. So a request that left
/// this out handed `Set::build_many` a Set aimed at the defaults, and the first
/// save of any file in the slot re-aimed a camera the operator had loaded, with
/// nothing said. The live save then recorded `Set::camera` faithfully, which is
/// how a wrong picture turned into a wrong file.
#[test]
fn a_rebuild_restates_the_camera_the_slot_is_aimed_with() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let tmp = tempfile::tempdir().expect("temp dir");
    let files = ["drift_shell.kir", "soft_points.kir"];
    let paths: Vec<PathBuf> = files.iter().map(|f| tmp.path().join(f)).collect();
    // Six numbers no default produces, so a request that let the camera be
    // re-derived cannot pass by accident.
    let aimed = karakuri_engine::camera::Orbit {
        radius: 3.25,
        speed: 0.75,
        height: -1.5,
        fov_y: 0.9,
        near: 0.25,
        far: 250.0,
    };
    let mut watch = Watch::new(
        0,
        crate::compile::Named::bare(paths[0].clone()),
        paths[1..]
            .iter()
            .cloned()
            .map(crate::compile::Named::bare)
            .collect(),
        karakuri_engine::set::Layering::Overdraw,
        None,
        Some(4096),
        1,
        vec![1],
        aimed,
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
    );
    for (file, path) in files.iter().zip(&paths) {
        std::fs::copy(root.join("examples").join(file), path).expect("copy an example");
    }

    let request = rebuild(&mut watch).expect("a build");
    // All six, not the two a `camera` record spells: a rebuild that carried
    // half of them would be as wrong as one that carried none, and quieter.
    let six =
        |o: &karakuri_engine::camera::Orbit| (o.radius, o.speed, o.height, o.fov_y, o.near, o.far);
    assert_eq!(
        six(&request.camera),
        six(&aimed),
        "a rebuild asked for a Set aimed somewhere other than where the slot is aimed"
    );
}

/// A rebuild restates the layering and the fold the slot was loaded with,
/// rather than leaving either to be worked out again.
///
/// A Set file records both — a `merge` record and its `live` — so a slot filled
/// by `--load-set` can be compositing, and folded to one renderer, without
/// `--merge` ever having been typed. A rebuild that re-derived the layering
/// from the flag would drop the slot to overdraw on the first save of any
/// `.kir` in it: the L5 gone, every renderer back over one attachment, and the
/// selection with it. That is `Watch::camera`'s failure in a register where the
/// picture does not come back — and a request that carried the layering but not
/// the fold would be half of it, a slot compositing every alternative at once.
///
/// Both are asserted from one rebuild, because both travel on one request and
/// either alone is not the Set that was loaded.
#[test]
fn a_rebuild_restates_the_layering_and_the_fold_the_slot_was_loaded_with() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let tmp = tempfile::tempdir().expect("temp dir");
    // Two renderers over one geometry, which is what there has to be for a
    // fold to be a choice at all.
    let files = ["drift_shell.kir", "soft_points.kir", "soft_points.kir"];
    let paths: Vec<PathBuf> = (0..files.len())
        .map(|at| tmp.path().join(format!("{at}.kir")))
        .collect();
    let mut watch = Watch::new(
        0,
        crate::compile::Named::bare(paths[0].clone()),
        paths[1..]
            .iter()
            .cloned()
            .map(crate::compile::Named::bare)
            .collect(),
        // What a composited Set file loads as — no flag was typed here,
        // which is the whole point.
        karakuri_engine::set::Layering::Composite,
        Some(1),
        Some(4096),
        1,
        vec![1],
        karakuri_engine::camera::Orbit::default(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
    );
    for (file, path) in files.iter().zip(&paths) {
        std::fs::copy(root.join("examples").join(file), path).expect("copy an example");
    }

    let request = rebuild(&mut watch).expect("a build");
    assert_eq!(
        request.layering,
        karakuri_engine::set::Layering::Composite,
        "a rebuild asked for a Set that overdraws, and the slot was compositing"
    );
    assert_eq!(
        request.live,
        Some(1),
        "a rebuild asked for a Set folding every renderer, and the slot was folded to one"
    );
}

/// A rebuild restates the names and the edges the slot is wired with.
///
/// A watcher used to be handed bare paths, so every rebuild called each node
/// whatever its procedure declared — harmless while a name only printed. It
/// stops being harmless twice over now: an `edge` names the node that declares
/// a slot and the node bound to it, so a rebuild that dropped the names
/// resolves against spellings that are no longer there, and one that dropped
/// the edges leaves the slot unbound — which is refused outright. Either way
/// the save that lands is a build that will not build, with the picture frozen
/// at whatever startup produced.
#[test]
fn a_rebuild_restates_the_names_and_edges_the_slot_is_wired_with() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let tmp = tempfile::tempdir().expect("temp dir");
    let files = [
        "lattice_shell.kir",
        "sphere_shell.kir",
        "morph.kir",
        "soft_points.kir",
    ];
    let paths: Vec<PathBuf> = files.iter().map(|f| tmp.path().join(f)).collect();
    // Named as the command line names them, with the far geometry carrying
    // the name the edge points with.
    let named: Vec<crate::compile::Named> = paths
        .iter()
        .zip(["near", "far", "morph", "draw"])
        .map(|(path, name)| crate::compile::Named {
            name: Some(name.to_string()),
            path: path.clone(),
        })
        .collect();
    let edges = vec![karakuri_engine::set::Edge {
        node: "morph".to_string(),
        slot: "far".into(),
        to: "far".to_string(),
    }];
    let mut watch = Watch::new(
        0,
        named[0].clone(),
        named[1..].to_vec(),
        karakuri_engine::set::Layering::Overdraw,
        None,
        Some(32768),
        1,
        vec![1, 2],
        karakuri_engine::camera::Orbit::default(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        edges.clone(),
        Vec::new(),
    );
    for (file, path) in files.iter().zip(&paths) {
        std::fs::copy(root.join("examples").join(file), path).expect("copy an example");
    }

    let request = rebuild(&mut watch).expect("a build");
    assert_eq!(
        request.names.l1s,
        vec![Some("near".to_string()), Some("far".to_string())],
        "a rebuild renamed the geometries the edge points at"
    );
    assert_eq!(request.names.l2s, vec![Some("morph".to_string())]);
    assert_eq!(
        request.edges, edges,
        "a rebuild dropped the wiring, so the slot it rebuilds is unbound"
    );
}

/// A rebuild restates the authority the slot's nodes were handed.
///
/// The failure this is against is the one every field beside it is against, and
/// it is the quietest of them: an operator grants a node to an agent, somebody
/// saves a `.kir` in that slot, and the rebuild hands back a Set where every
/// node is `Authority::Manual` again — the default, and the only default it
/// could be. Nothing refuses it, because a rebuild is not a surface, and
/// nothing looks wrong, because an arrangement about who may write a param does
/// not draw. The grant is simply gone.
///
/// Asserted over a request rather than over a Set, which is as far as this side
/// goes: applying the list is `HotSwap`'s, tested where it lives, and reaching
/// a built Set from here would want a device. What is this watcher's to get
/// wrong is whether the list survives the rebuild at all, and that is exactly
/// what this reads.
///
/// Two levels and two layers, neither of them the default. A test that granted
/// one node `Manual` would pass against a request that dropped the list
/// entirely, since that is where the build would land anyway.
#[test]
fn a_rebuild_restates_the_authority_the_slots_nodes_were_handed() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let tmp = tempfile::tempdir().expect("temp dir");
    // Two geometries and a renderer, so that a wrong index and a wrong
    // layer are both failures this can see.
    let files = ["drift_shell.kir", "lattice_shell.kir", "soft_points.kir"];
    let paths: Vec<PathBuf> = files.iter().map(|f| tmp.path().join(f)).collect();
    let authorities = vec![
        karakuri_engine::swap::AuthorityAt::new(
            karakuri_ir::Kind::L1,
            1,
            karakuri_engine::set::Authority::Automatic,
        ),
        karakuri_engine::swap::AuthorityAt::new(
            karakuri_ir::Kind::L4,
            0,
            karakuri_engine::set::Authority::Suggesting,
        ),
    ];
    let mut watch = Watch::new(
        0,
        crate::compile::Named::bare(paths[0].clone()),
        paths[1..]
            .iter()
            .cloned()
            .map(crate::compile::Named::bare)
            .collect(),
        karakuri_engine::set::Layering::Overdraw,
        None,
        Some(4096),
        1,
        vec![1, 2],
        karakuri_engine::camera::Orbit::default(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        authorities.clone(),
    );
    for (file, path) in files.iter().zip(&paths) {
        std::fs::copy(root.join("examples").join(file), path).expect("copy an example");
    }

    let request = rebuild(&mut watch).expect("a build");
    assert_eq!(
        request.authorities, authorities,
        "a rebuild dropped the grants, so the next save takes back every node \
             an operator gave away"
    );
}

/// The startup path and the rebuild path answer "which layer is this file on,
/// and which node of that layer" identically, which is the whole reason
/// [`crate::compile::sort_compiled`] is one function rather than a match in
/// each of them.
///
/// The two used to hold a copy each and had already drifted: the rebuild took
/// its head for the L1 whatever the file declared, so a slot spelled with a
/// deformer first sorted one way at startup and another way on the first save.
/// Asserted as agreement rather than as two expected answers, because what has
/// to hold is that they are the same answer — an expected answer written twice
/// is the drift again, in the tests.
#[test]
fn a_rebuild_addresses_a_slot_exactly_as_startup_did() {
    // A whole stack, head first and deliberately not an L1: every layer, and
    // two of the three that can hold several, so a wrong *index* fails here
    // as loudly as a wrong layer.
    let files = [
        "swirl_warp.kir",
        "drift_shell.kir",
        "beat_jump.kir",
        "melt_blob.kir",
        "soft_points.kir",
        "lattice_shell.kir",
        "kaleidoscope.kir",
        "glass_shell.kir",
    ];
    let tmp = tempfile::tempdir().expect("temp dir");
    let (watch, paths) = watch_over(tmp.path(), &files);

    let store_dir = tempfile::tempdir().expect("temp dir");
    let store =
        std::sync::Arc::new(karakuri_store::store::Store::open(store_dir.path()).expect("store"));
    let (tx, rx) = std::sync::mpsc::channel();
    let mut watch = watch.storing_to(store, tx);
    let request = rebuild(&mut watch).expect("the stack compiles, so it rebuilds");

    let rest: Vec<crate::compile::Named> = paths[1..]
        .iter()
        .map(|p| crate::compile::Named::bare(p.clone()))
        .collect();
    let (material, placed) =
        crate::compile::sort_slot(0, &crate::compile::Named::bare(paths[0].clone()), &rest);

    let procs = |checked: &[karakuri_ir::typed::Checked]| {
        checked
            .iter()
            .map(|c| c.name.clone())
            .collect::<Vec<String>>()
    };
    assert_eq!(
        request
            .l1s
            .iter()
            .map(|(c, _)| c.name.clone())
            .collect::<Vec<String>>(),
        procs(&material.l1s),
        "the two paths disagree about the geometries"
    );
    assert_eq!(procs(&request.l2s), procs(&material.l2s), "the deformers");
    assert_eq!(procs(&request.l4s), procs(&material.l4s), "the renderers");
    assert_eq!(procs(&request.l3s), procs(&material.l3s), "the cameras");
    assert_eq!(
        procs(&request.fields),
        procs(&material.fields),
        "the fields"
    );

    // **The addresses, file by file**, which is the half a request cannot
    // show: a `procedure` record names a node by `(layer, index)`, and the
    // history files a version under the same pair.
    let built = rx.try_recv().expect("a recorded build is reported");
    let rebuilt: Vec<(&str, u32)> = built
        .nodes
        .iter()
        .map(|(layer, index, _)| (*layer, *index))
        .collect();
    let started: Vec<(&str, u32)> = placed
        .iter()
        .map(|node| (crate::setfile::kind_name(node.layer), node.index))
        .collect();
    assert_eq!(
        rebuilt, started,
        "the two paths address the same files as different nodes"
    );
    // The premise, stated so that a rewrite of the file list cannot quietly
    // turn this back into a test about a slot whose head is its L1.
    assert_eq!(
        started[0],
        ("L2", 0),
        "this asserts nothing unless the head is a file that is not the L1"
    );
}

/// A rebuild that cannot be assembled leaves the running Set alone. This is the
/// one thing the two sorting paths do differently, and the reason
/// [`crate::compile::sort_compiled`] hands back a sentence rather than exiting:
/// a startup with no picture has nothing to keep showing, and an operator
/// editing a slot into an illegal shape has a picture on stage.
#[test]
fn a_slot_that_cannot_be_assembled_leaves_the_running_set_alone() {
    // A stack that assembles, and then one that cannot — so this fails if
    // the refusal stopped happening *and* if it started happening to
    // everything. **Two cameras is the first kind and used to be the
    // second**: a slot holds as many as its files declare, and which
    // renderer draws from which is an `edge`.
    for (files, buildable) in [
        (
            &["drift_shell.kir", "beat_jump.kir", "soft_points.kir"][..],
            true,
        ),
        (
            &[
                "drift_shell.kir",
                "beat_jump.kir",
                "beat_jump.kir",
                "soft_points.kir",
            ][..],
            true,
        ),
        // Nothing that draws: a Set with no renderer has no frame to give.
        (&["drift_shell.kir", "swirl_warp.kir"][..], false),
    ] {
        let tmp = tempfile::tempdir().expect("temp dir");
        // Two of the same file need two names, or the copy is one file.
        let unique: Vec<String> = files
            .iter()
            .enumerate()
            .map(|(n, f)| format!("{n}_{f}"))
            .collect();
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let paths: Vec<PathBuf> = unique.iter().map(|f| tmp.path().join(f)).collect();
        let mut watch = Watch::new(
            0,
            crate::compile::Named::bare(paths[0].clone()),
            paths[1..]
                .iter()
                .cloned()
                .map(crate::compile::Named::bare)
                .collect(),
            karakuri_engine::set::Layering::Overdraw,
            None,
            Some(4096),
            1,
            vec![1],
            karakuri_engine::camera::Orbit::default(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        );
        for (file, path) in files.iter().zip(&paths) {
            std::fs::copy(root.join("examples").join(file), path).expect("copy an example");
        }
        assert_eq!(
            rebuild(&mut watch).is_some(),
            buildable,
            "{files:?} rebuilt the wrong way"
        );
    }
}

/// A missing file is stable rather than a change every interval, so deleting
/// one does not put the watcher into a recompile loop against a path that is
/// not there. A re-point builds the material it was aimed at, on the poll it
/// arrives, and the new files are not then seen as an edit.
///
/// This is how a Set reaches a *running* deck, and the whole reason it is a
/// re-point rather than an install: `Deck::install` is documented as
/// deliberately unreachable from a key or a surface, because *"a live run
/// changes its material by editing a file and letting the worker build it,
/// which is what the budget watchdog is attached to"*. So the failure this
/// catches is a load that goes nowhere — an aim taken and no request made — and
/// the deck goes on playing what it was with nothing said.
///
/// The second half is the debounce not eating it. A save is acted on the poll
/// *after* the one that saw it, because an editor writing in place leaves a
/// file truncated for a moment. An aim has no such moment, and a re-point that
/// went through the debounce would then be seen a second time as a change and
/// built twice — so the poll after the build has to be quiet.
#[test]
fn an_aim_points_the_slot_at_what_it_names_and_is_not_debounced() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let tmp = tempfile::tempdir().expect("temp dir");
    let copy = |file: &str| {
        let path = tmp.path().join(file);
        std::fs::copy(root.join("examples").join(file), &path).expect("copy an example");
        path
    };
    // Two stacks that share their renderer, so what changes between them
    // is the geometry and the label says which one is running.
    let was = copy("drift_shell.kir");
    let now = copy("lattice_shell.kir");
    let renderer = copy("soft_points.kir");

    let (aim, aimed) = std::sync::mpsc::channel();
    let mut watch = Watch::new(
        0,
        crate::compile::Named::bare(was),
        vec![crate::compile::Named::bare(renderer.clone())],
        karakuri_engine::set::Layering::Overdraw,
        None,
        None,
        7,
        vec![7],
        karakuri_engine::camera::Orbit::default(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
    )
    .aimed_by(aimed);

    // Nothing has changed and nothing was aimed, so nothing is requested —
    // the floor under everything below, because a watcher that requested a
    // build every poll would pass the assertion after it by accident.
    assert!(
        watch.poll().is_none(),
        "a watcher nobody touched asked for a build"
    );

    // **A salt and a capacity no derivation here produces**, so that a
    // re-point which kept the outgoing slot's values cannot pass: the
    // symptom of that is invisible on the load and arrives at the first
    // later save.
    aim.send(Aim {
        head: crate::compile::Named::bare(now),
        rest: vec![crate::compile::Named::bare(renderer)],
        layering: karakuri_engine::set::Layering::Composite,
        live: Some(0),
        capacity: Some(2048),
        seed_salt: 0x0bad_cafe,
        salts: vec![0x0bad_cafe],
        camera: karakuri_engine::camera::Orbit::default(),
        overrides: Vec::new(),
        published: Vec::new(),
        bindings: Vec::new(),
        edges: Vec::new(),
        authorities: Vec::new(),
        set: None,
    })
    .expect("the watcher is still here");

    let Some(Polled::Build(request)) = watch.poll() else {
        panic!("the aim is built on the poll it arrives");
    };
    assert!(
        request.label.contains("lattice"),
        "the slot was aimed at `lattice_shell.kir` and built `{}`",
        request.label
    );
    assert_eq!(
        request.salts,
        vec![Some(0x0bad_cafe)],
        "the build kept the salts the slot was running at instead of the ones it was aimed \
             with"
    );
    assert_eq!(request.layering, karakuri_engine::set::Layering::Composite);
    assert_eq!(request.live, Some(0));
    assert_eq!(
        request.l1s[0].1, 2048,
        "the build kept the capacity the slot was running at"
    );

    assert!(
        watch.poll().is_none(),
        "the files it was aimed at were then seen as an edit, so the load built twice"
    );
}

/// A library load moves which Set the versions after it are filed under, which
/// is the half of `ADR-0276` a re-point owes.
///
/// The failure this is written against is silent and is only readable
/// afterwards: a watcher that took the aim's files and left its Set id behind
/// goes on filing every later version under the Set the slot was running
/// *before* the load — a name, and nothing in the layout to say it is wrong, so
/// *what versions has this Set had* answers with somebody else's edits.
///
/// The aim points at the files already being watched, on purpose: the bytes do
/// not change, so the only thing that can make this version a new one is the id
/// in the dedup key. It is the `record` clause that says a chain is a node *of
/// a Set*, asserted from the watcher's side.
#[test]
fn a_re_point_files_the_versions_after_it_under_the_set_it_loaded() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let dir = tmp.path().join("scratch");
    std::fs::create_dir_all(&dir).expect("a directory to watch");
    let store = tmp.path().join("store");

    let (watch, paths) = watch_over(&dir, &["drift_shell.kir", "soft_points.kir"]);
    let shared = crate::history::Snapshots::shared(&store);
    let (aim, aimed) = std::sync::mpsc::channel();
    // **Launched on material no Set names**, which is what every slot of
    // `crates/karakuri` launches on and what `record` writes `None` for.
    let mut watch = watch.snapshotting_to(shared.clone(), None).aimed_by(aimed);

    rebuild(&mut watch).expect("the files appearing is an edit like any other");
    let before = crate::history::list(&store, 32).expect("the history lists back");
    assert!(
        before.versions.iter().all(|v| v.set.is_none()),
        "a run that has loaded nothing filed a version under a Set: {:?}",
        before.versions
    );
    assert!(
        !before.versions.is_empty(),
        "nothing was snapshotted at all, so what follows would pass against a \
             watcher that never records"
    );

    aim.send(Aim {
        head: crate::compile::Named::bare(paths[0].clone()),
        rest: vec![crate::compile::Named::bare(paths[1].clone())],
        layering: karakuri_engine::set::Layering::Overdraw,
        live: None,
        capacity: Some(4096),
        seed_salt: 1,
        salts: vec![1],
        camera: karakuri_engine::camera::Orbit::default(),
        overrides: Vec::new(),
        published: Vec::new(),
        bindings: Vec::new(),
        edges: Vec::new(),
        authorities: Vec::new(),
        set: Some("star_vortex".to_string()),
    })
    .expect("the watcher is still here");
    watch
        .poll()
        .expect("the aim is built on the poll it arrives");

    let after = crate::history::list(&store, 32).expect("the history lists back");
    let of_set: Vec<&crate::history::Version> = after
        .versions
        .iter()
        .filter(|v| v.set.as_deref() == Some("star_vortex"))
        .collect();
    assert!(
        !of_set.is_empty(),
        "the slot was loaded with `star_vortex` and its first version was filed \
             under the material the run started on: {:?}",
        after.versions
    );
    assert!(
        of_set
            .iter()
            .all(|v| v.file.to_string_lossy().contains("@star_vortex")),
        "the id is read back off the name, and the name does not carry it: {:?}",
        of_set
    );
}

#[test]
fn a_missing_file_is_stable() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let w = watch_on(tmp.path());
    assert_eq!(w.stamp(), [None, None]);
    assert_eq!(w.stamp(), w.stamp());
}
