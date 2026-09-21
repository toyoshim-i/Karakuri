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
    let store =
        karakuri_store::store::Store::open(dir.path().join("store")).expect("a store opens");
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
    let six =
        |o: &karakuri_engine::camera::Orbit| (o.radius, o.speed, o.height, o.fov_y, o.near, o.far);
    assert_eq!(
        six(&recorded),
        six(&aimed),
        "the save after a rebuild would have written a camera the operator never aimed"
    );
}
/// Asserts that saving after a parameter modification records the updated value
/// attributed to the specific declaring node.
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

    let radius: Vec<&ParamWrite> = loaded.params.iter().filter(|p| p.key == "radius").collect();
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
// Under ADR-0316, a save records the version currently active in the slot,
// regardless of on-disk file modifications.

/// Asserts that a run without a watcher saves the shader versions currently drawn,
/// even if underlying source files on disk have changed.
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
