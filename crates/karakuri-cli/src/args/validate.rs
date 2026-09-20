use super::*;

/// Semantic consistency checks and default material assignments.
pub(crate) fn validate_args(args_out: &mut Args, size_given: bool) -> Result<(), String> {
    if args_out.sets.is_empty() && args_out.load_set.is_none() {
        // A demonstration that needs a particular scene brings it, because one
        // that asks the operator to assemble it first is not a demonstration.
        // Only when nothing else named material: `--set` still wins.
        match args_out.demo.map(Demo::deck).filter(|d| !d.is_empty()) {
            Some(deck) => args_out.sets = deck,
            // **`examples/star_vortex.kset`'s two parts**, and the choice is a
            // demo one: what a run that named nothing is worth looking at.
            // ADR-0270 freed it — the reference workload is a named Set rather
            // than whatever this line says — and ADR-0271 spent it. The `.kset`
            // itself is not loaded here because this is two paths; its two
            // `bind` records are what a bare run does not get.
            None => args_out.sets.push((
                Named::bare("examples/coil_vortex.kir"),
                vec![Named::bare("examples/star_flares.kir")],
            )),
        }
    }
    // Refused rather than resolved: a Set file describes the material, and two
    // `.kir` paths describe the material, and a run given both has been told
    // two different things about what to play.
    if args_out.load_set.is_some() && !args_out.sets.is_empty() {
        return Err(
            "--load-set names the material and so do the `.kir` paths beside it; give one \
             or the other"
                .to_string(),
        );
    }
    if args_out.load_set.is_some() && args_out.save_set.is_some() {
        return Err(
            "--load-set and --save-set in one run: it would rewrite what it just read".to_string(),
        );
    }
    // An offscreen run is a function of its inputs — that is why it never
    // watches files either. Accepting `--audio-in` here and quietly ignoring it
    // would produce a PNG sequence whose bindings all sat at a tenth effect
    // with nothing to say why.
    if args_out.audio_in.is_some() && (args_out.render_to.is_some() || args_out.seq_to.is_some()) {
        return Err(
            "`--audio-in` with `--render` or `--seq` — an offscreen run takes no live input, \
             because its output has to be a function of its arguments. Drop one of them"
                .to_string(),
        );
    }
    // The same, and it is the same argument: a surface is a pair of hands, and
    // an offscreen render has nobody at it.
    if args_out.midi_in.is_some() && (args_out.render_to.is_some() || args_out.seq_to.is_some()) {
        return Err(
            "`--midi-in` with `--render` or `--seq` — an offscreen run takes no live input, \
             because its output has to be a function of its arguments. Drop one of them"
                .to_string(),
        );
    }
    let offscreen = args_out.render_to.is_some() || args_out.seq_to.is_some();
    // **`--mcp` with `--load-set` used to be refused here**, because a Set
    // built from the store had no procedure files on disk for a model to read
    // or rewrite. The scratch is where they go now: `--load-set` writes its two
    // procedures there like any other material, so a saved Set is editable and
    // the round trip — save, load, edit, save — closes. See `scratch::place`.
    // A surface with nobody at it, on the same terms as the other two — and
    // one more reason besides: an offscreen run is a function of its arguments,
    // and a port that can rewrite a procedure mid-render is the opposite of
    // that.
    if args_out.mcp.is_some() && (offscreen || args_out.replay.is_some()) {
        return Err(
            "`--mcp` with `--render`, `--seq` or `--replay` — an offscreen run takes no \
             live input, because its output has to be a function of its arguments"
                .to_string(),
        );
    }
    // Third of the same kind, and the same argument: a tempo source is another
    // machine's clock, and an offscreen run's output has to be a function of
    // its arguments. A replay has a stronger reason still — it follows the grid
    // the session recorded, so a live source would be overwriting the
    // performance it is supposed to be reproducing.
    if args_out.tempo_source.is_some() && (offscreen || args_out.replay.is_some()) {
        return Err(
            "`--tempo-source` with `--render`, `--seq` or `--replay` — an offscreen run \
             takes no live input, and a replay follows the grid the session recorded"
                .to_string(),
        );
    }
    // A map with no port is a file nothing reads, and the likely cause is a
    // forgotten `--midi-in` rather than a deliberate one.
    if args_out.midi_map.is_some() && args_out.midi_in.is_none() {
        return Err(
            "`--midi-map` with no `--midi-in` — there is no surface for the map to be of"
                .to_string(),
        );
    }
    // `--size` is the preview window's and an offscreen run has no window. It
    // used to be the render size too, and that is exactly the confusion being
    // removed: a reader who types `--render out.png --size 1920x1080` today
    // means `--canvas`, and quietly rendering at 1280x720 because `--size` no
    // longer reaches the canvas would be the worst of the three outcomes.
    if size_given && (offscreen || args_out.replay.is_some()) {
        return Err(
            "`--size` sets the preview window, and this run has no window. \
             Use `--canvas` for what is rendered"
                .to_string(),
        );
    }
    // **Not the argument the two above make.** `--record-session` is an output,
    // so "an offscreen run takes no live input" does not reach it. The reason is
    // that there is no performance here to record: an offscreen run advances one
    // step a frame and every edit it makes is a flag, so the stream would be
    // `--save-set`'s output followed by a constant — a file that looks like a
    // timeline and is a re-encoding of the command line.
    //
    // It was silently dropped instead, which is the failure the recorder's own
    // construction site has a comment warning against: a run that continued
    // without the recorder is "a performance nobody can replay and nothing
    // saying so". That guard fires when the *file* cannot be opened, and did
    // nothing when the flag never reached it at all — exit 0, no warning, no
    // session.
    if args_out.record_session.is_some() && (offscreen || args_out.replay.is_some()) {
        return Err(
            "`--record-session` with `--render`, `--seq` or `--replay` — there is no \
             performance to record: an offscreen run is a function of its arguments, and \
             the stream would say only what they already say"
                .to_string(),
        );
    }
    if args_out.sets.len() > MAX_SLOTS {
        return Err(format!(
            "{} Sets, and the deck holds {MAX_SLOTS}",
            args_out.sets.len()
        ));
    }
    Ok(())
}

/// Refuse a canvas the GPU cannot make a texture of, by name.
///
/// Not in `extent`, because the number it is checked against is the adapter's
/// rather than the format's: `max_texture_dimension_2d` is 8192 on some
/// machines and 16384 on others, so a canvas is legal or not depending on what
/// is running the run. That makes it the earliest point *after* a device exists
/// rather than the latest point before one does.
///
/// The alternative is what happened before: a panic out of `create_texture`
/// naming a wgpu limit, from inside a call stack that says nothing about
/// `--canvas`. Every other refusal in this program names the flag.
pub(crate) fn check_canvas(device: &wgpu::Device, width: u32, height: u32) {
    let limit = device.limits().max_texture_dimension_2d;
    if width > limit || height > limit {
        eprintln!(
            "karakuri-cli: `--canvas {width}x{height}` — this GPU renders at most \
             {limit}x{limit}"
        );
        std::process::exit(1);
    }
}

/// Each slot's seed, derived from its index alone so that two slots given the
/// same pair are not the same picture twice — and so that the derivation is a
/// pure function of the deck layout rather than of anything measured. Slot 0 is
/// [`SEED`] unchanged, so a one-Set run renders exactly what it always did.
pub(crate) fn seed_for(slot: usize) -> u32 {
    SEED.wrapping_add((slot as u32).wrapping_mul(0x9E37_79B9))
}

/// What each of a slot's geometries is salted with, in the order its L1
/// procedures were given — one number per geometry.
///
/// `recorded` is what a Set file said, per geometry, and it wins where it said
/// anything. That is the whole of what recording a salt buys over deriving one:
/// `docs/ir-spec.md` asks for a value assigned when a source is added and read
/// back from the stream forever after, so that reordering `--set` stops
/// changing which grid gets which randomness. Empty for a slot no file filled,
/// which is every slot but slot 0.
///
/// Derived where nothing recorded one, by the engine's own fallback rather than
/// by a formula spelled out a second time here — and the spec licenses that
/// squarely: *where it came from stops mattering once it is recorded*. A bare
/// `--set` run is salted by ordinal and looks exactly as it always did; the
/// moment `--save-set` writes these numbers down they stop being derived, which
/// is why this is also what the writer asks. One function, so the file cannot
/// record a salt the run was not using — the shape [`capacities_for`] was fixed
/// into after recording the flag's number and drawing another.
pub(crate) fn salts_for(seed: u32, recorded: &[Option<u32>], geometries: usize) -> Vec<u32> {
    (0..geometries)
        .map(|at| {
            recorded
                .get(at)
                .copied()
                .flatten()
                .unwrap_or_else(|| karakuri_engine::set::derived_salt(seed, at))
        })
        .collect()
}

/// Open the store, or stop with the reason. Both directions need one and
/// neither can do anything useful without it.
pub(crate) fn open_store(args: &Args) -> karakuri_store::store::Store {
    match karakuri_store::store::Store::open(&args.store) {
        Ok(store) => store,
        Err(e) => {
            eprintln!("karakuri-cli: store `{}`: {e}", args.store.display());
            std::process::exit(1);
        }
    }
}

/// What the store holds, one line per Set — the operator's half of what
/// `list_sets` tells a model.
///
/// It prints and stops. No window, no adapter, no compile, no Set built: asking
/// what is in a library is not a run, and a flag that opened a GPU to answer it
/// would be unusable over ssh on the machine the library is on. That is why it
/// is answered out of [`parse_args_from`] — see [`ParseOutcome`] — rather than
/// somewhere down `main` where it would have to be kept above every early
/// return by hand.
///
/// The same summary the MCP tool renders, from [`setfile::summarise`]: one
/// derivation, two renderings. What a node is called here is what `read_set`
/// calls it, because the answer comes from one function — an operator reading a
/// line here and a model reading a block there are looking at one library and
/// must be told one thing about it.
///
/// A compact line and not a block: the question is *which of these do I want*,
/// and what answers it is the id to type next to `--load-set`, when it was
/// saved, and enough of what it holds to tell two of them apart. What each node
/// declares is `read_set`'s answer, over MCP, on one Set at a time.
pub(crate) fn listed_sets(
    store: &karakuri_store::store::Store,
    root: &std::path::Path,
) -> Result<String, String> {
    let mut sets =
        setfile::summarise(store).map_err(|e| format!("store `{}`: {e}", root.display()))?;
    if sets.is_empty() {
        // Not an error and not silence: an empty store is what a store looks
        // like before anything has been kept in it, and the answer says where
        // sets come from rather than leaving a blank terminal to be read as a
        // failure.
        return Ok(format!(
            "no sets in `{}` — nothing has been kept here yet. `--save-set ID` writes \
             one, and so does the `k` key during a run.\n",
            root.display()
        ));
    }
    // **Most recent first, breaking ties by id.** `--save-set` twice in one
    // second gives two files one mtime on a coarse filesystem clock, and a sort
    // whose keys tie leaves the order to whatever `read_dir` said — so two runs
    // of this flag over an untouched store would print two different lists. The
    // id is unique by construction, which makes the order total.
    sets.sort_by(|a, b| b.written.cmp(&a.written).then_with(|| a.id.cmp(&b.id)));
    let width = sets.iter().map(|set| set.id.len()).max().unwrap_or(0);
    let mut out = String::new();
    for set in &sets {
        let _ = write!(
            out,
            "{:<width$}  {}  ",
            set.id,
            setfile::written_at(set.written)
        );
        match &set.unreadable {
            // Listed and named rather than dropped: a file in `sets/` that will
            // not read is the one thing here an operator has to go and look at.
            Some(why) => {
                let _ = writeln!(out, "unreadable: {why}");
            }
            None if set.nodes.is_empty() => {
                let _ = writeln!(out, "no material: it holds no `slot` record");
            }
            None => {
                let _ = writeln!(out, "{}", holdings(&set.nodes));
            }
        }
    }
    Ok(out)
}

/// The listing for a store path — opening nothing that is not there.
///
/// `Store::open` establishes the layout under a root that does not exist yet,
/// which is right for a run about to write into it and wrong here: a flag whose
/// whole promise is that it only reads must not leave a directory behind to say
/// that a library is empty. A path with no store at it is an answer, and it is
/// a different one from a store with no sets in it — the first is very often a
/// mistyped `--store`.
pub(crate) fn listed_sets_at(root: &std::path::Path) -> Result<String, String> {
    if !root.exists() {
        return Ok(format!(
            "no store at `{}` — nothing has ever been kept there, and nothing was \
             created to find that out. Check `--store`, or keep something with \
             `--save-set ID`.\n",
            root.display()
        ));
    }
    let store = karakuri_store::store::Store::open(root)
        .map_err(|e| format!("store `{}`: {e}", root.display()))?;
    listed_sets(&store, root)
}

/// One Set, with every source it names inlined, as the text to print.
///
/// A package goes to standard output, which is what this returning a `String`
/// is: it is a file you *send somebody* — one self-contained patch that loads
/// in a store which has never held the material — so it belongs where
/// `karakuri-cli --package night01 > night01.kbset` puts it. A store directory
/// would need a naming rule of its own for it, and a second copy of a Set
/// sitting beside the Set is a second answer to which of them is the file.
///
/// Opening nothing that is not there, on [`listed_sets_at`]'s terms: a flag
/// that only reads must not leave a store behind to report that a Set is
/// missing from it. Here it is an error rather than an answer, because a bundle
/// of a Set that does not exist is not a bundle.
///
/// Or an authoring file, and that is one flag rather than two. ADR-0229 part 4
/// settles that packaging is *"one operation, two moments"* — loading an
/// authoring file *is* packaging it, and packaging for distribution is the same
/// resolution done ahead of time — so the flag grew the other moment instead of
/// a second flag beside it, and `docs/manual/operations.html` keeps the one row
/// it always had. And it is called packaging because of exactly that: the half
/// that takes a `.kset` resolves, stores and writes, so an export is only one
/// of the two things this flag does — see [`ParseOutcome::Package`]. Which of
/// the two is decided by the extension, on exactly the terms ADR-0231 made it
/// load-bearing for: a value ending in `.kset` is a path to an authoring file,
/// and anything else is an id in the store. Nothing else could decide it — an
/// id and a relative path are both bare words — and this is the same sentence
/// the store already reads a name with.
///
/// The authoring half opens a store that is not there, where the id half
/// refuses to. Resolving is a *write*: each part is read from disk, hashed and
/// put in the store as an artifact, so the store has to exist by the time the
/// first one lands, and establishing the layout under a root an operator named
/// is what every other writing path here does — see [`taken_in_file`]. The id
/// half is still a pure read and still leaves nothing behind.
pub(crate) fn packaged_set(root: &std::path::Path, named: &str) -> Result<String, String> {
    let lines = if named.ends_with(setfile::AUTHORING_SUFFIX) {
        let store = karakuri_store::store::Store::open(root)
            .map_err(|e| format!("store `{}`: {e}", root.display()))?;
        setfile::bundle_authored(&store, std::path::Path::new(named))?
    } else {
        if !root.exists() {
            return Err(format!(
                "no store at `{}`, so there is no set `{named}` to package — check `--store`",
                root.display()
            ));
        }
        let store = karakuri_store::store::Store::open(root)
            .map_err(|e| format!("store `{}`: {e}", root.display()))?;
        setfile::bundle(&store, named)?
    };
    Ok(lines
        .iter()
        .map(|line| format!("{}\n", line.as_str()))
        .collect())
}

/// A Set file somebody sent you, into this store, and the report of what
/// happened.
///
/// Both of a Set's forms, and the extension is the whole of what tells them
/// apart — [`packaged_set`]'s rule, read from the other end, and ADR-0231's. A
/// `.kbset` is already resolved and carries its own sources, so it is read and
/// taken in as it stands. A `.kset` is the authoring form: it names its parts
/// by relative path, so it goes through `setfile::resolve` first — behind the
/// same wall, refusing the same escapes, absolute paths, `..` and symlinks
/// alike.
///
/// Resolved *and* inlined rather than resolved alone, which is
/// `setfile::bundle_authored`, so that taking a `.kset` in lands the same store
/// as packaging it and taking the result in. `setfile::unbundle` writes a
/// metadata card for each source the lines carry; handing it resolved lines
/// with nothing inlined would file the Set and leave every artifact cardless —
/// one operation reaching two different stores by two routes, which is the
/// disagreement a second spelling always is.
///
/// The store *is* opened where it is not there, unlike [`packaged_set`]'s id
/// half: this is a write, and establishing the layout under a root an operator
/// named is what every other writing path here does.
pub(crate) fn taken_in_file(
    root: &std::path::Path,
    file: &std::path::Path,
) -> Result<String, String> {
    let store = karakuri_store::store::Store::open(root)
        .map_err(|e| format!("store `{}`: {e}", root.display()))?;
    let named = file
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    let lines = if named.ends_with(setfile::AUTHORING_SUFFIX) {
        setfile::bundle_authored(&store, file)?
    } else {
        karakuri_store::ndjson::read(file)
            .map_err(|e| format!("reading `{}`: {e}", file.display()))?
    };
    // `Somebody`: this binary has no `--presets`, so it cannot tell a shipped
    // file from any other typed path. ADR-0347.
    setfile::unbundle(&store, setfile::CameFrom::Somebody, &lines)
}

/// What a Set holds, by layer and in the order the layers compose: `2 L1, 1 L2,
/// 3 L4`. A layer nothing is on is left out rather than printed as a zero,
/// because most Sets are on three of the six and a line of zeroes reads as
/// something missing.
pub(crate) fn holdings(nodes: &[setfile::NodeSummary]) -> String {
    [
        Layer::L1,
        Layer::L2,
        Layer::L3,
        Layer::L4,
        Layer::Field,
        Layer::L5,
    ]
    .iter()
    .filter_map(|layer| {
        let n = nodes.iter().filter(|node| node.layer == *layer).count();
        (n > 0).then(|| format!("{n} {}", setfile::layer_name(*layer)))
    })
    .collect::<Vec<_>>()
    .join(", ")
}

/// What a loaded Set file said its geometries run at, for the slot it filled.
///
/// Slot 0 and nothing else, on the same terms as its seed and its camera:
/// `--load-set` fills that slot and every other slot comes from `--set`.
pub(crate) fn recorded_capacities(args: &Args, slot: usize) -> &[Option<u32>] {
    match (slot, args.from_set.as_ref()) {
        (0, Some(from_set)) => &from_set.capacities,
        _ => &[],
    }
}

/// What a loaded Set file said its geometries are salted with, for the slot it
/// filled. Slot 0 and nothing else, on the same terms as its capacities.
pub(crate) fn recorded_salts(args: &Args, slot: usize) -> &[Option<u32>] {
    match (slot, args.from_set.as_ref()) {
        (0, Some(from_set)) => &from_set.salts,
        _ => &[],
    }
}

/// Where a loaded Set file aimed the built-in camera, for the slot it filled.
/// Slot 0 and nothing else, on the same terms as its capacities and its salts —
/// and `None` for a file that recorded no `camera` record, or one whose record
/// named a camera that is a procedure and was reported and dropped on the way
/// in.
///
/// One reading, used twice. The Set built at startup takes it and so does the
/// watcher that restates it on every rebuild; working it out in two places is
/// how a rebuild came to aim somewhere the startup did not.
pub(crate) fn recorded_camera(args: &Args, slot: usize) -> Option<karakuri_engine::camera::Orbit> {
    match (slot, args.from_set.as_ref()) {
        (0, Some(from_set)) => from_set.camera,
        _ => None,
    }
}

/// Whether a slot composites its renderers or overdraws them — the one reading,
/// from the flag and the file together.
///
/// Either saying so is enough, and that is a decision rather than a
/// coincidence. `--merge N` can only turn compositing *on*: there is no
/// spelling that turns it off, because the record's absence is what overdraw is
/// and a flag that could say `false` would be a second spelling of not typing
/// it. So `--load-set X --merge 0` on a file that already records a `merge` is
/// two ways of asking for the same thing, and a file that records none plus
/// `--merge 0` is the flag adding what the file did not say. The one case that
/// could have been a contest — a composited file with `--merge` *left off* — is
/// not one: leaving a flag off is not a statement, and treating it as one would
/// make a loaded preset silently overdraw exactly as it did before this record
/// existed.
///
/// One reading, used everywhere, on [`recorded_camera`]'s terms: the Set built
/// at startup takes it, the watcher that restates it on every rebuild takes it,
/// and the writer that saves the slot back out takes it. Working it out in two
/// places is how a rebuild came to aim a camera where the startup did not.
pub(crate) fn layering_for(
    args: &Args,
    slot: usize,
    recorded: karakuri_engine::set::Layering,
) -> karakuri_engine::set::Layering {
    if recorded == karakuri_engine::set::Layering::Composite || args.merge.contains(&slot) {
        karakuri_engine::set::Layering::Composite
    } else {
        karakuri_engine::set::Layering::Overdraw
    }
}

/// What a loaded Set file said about its layering, for the slot it filled —
/// [`recorded_camera`]'s shape and its rule: slot 0 and nothing else, since
/// `--load-set` fills that slot and every other comes from `--set`.
///
/// Separate from [`layering_for`] because the replay path has a recorded
/// layering in hand without ever touching `args.from_set` — its Set file is the
/// head of a session stream — and both readings have to meet the flag through
/// the same function. That is [`recorded_capacities`] and [`capacities_for`]'s
/// split, for its reason.
pub(crate) fn recorded_layering(args: &Args, slot: usize) -> karakuri_engine::set::Layering {
    match (slot, args.from_set.as_ref()) {
        (0, Some(from_set)) => from_set.layering,
        _ => karakuri_engine::set::Layering::Overdraw,
    }
}

/// Which renderer a loaded Set file left folded to, for the slot it filled.
/// Slot 0 and nothing else, on the same terms as its camera and its capacities
/// — and `None` for a file that recorded no selection, which is every input
/// live and is the state a Set nobody selected in comes up in.
///
/// No flag stands beside this one. There is no `--select`: a selection is
/// something an operator makes with `r` while watching, so the only thing that
/// can put one in before the first frame is a file that recorded one.
pub(crate) fn recorded_live(args: &Args, slot: usize) -> Option<u32> {
    match (slot, args.from_set.as_ref()) {
        (0, Some(from_set)) => from_set.live,
        _ => None,
    }
}
