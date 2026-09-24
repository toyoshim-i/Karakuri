use super::*;

/// Semantic consistency checks and default material assignments.
pub(crate) fn validate_args(args_out: &mut Args, size_given: bool) -> Result<(), String> {
    if args_out.sets.is_empty() && args_out.load_set.is_none() {
        // A demonstration that needs a particular scene brings it, because one
        // that asks the operator to assemble it first is not a demonstration.
        // Only when nothing else named material: `--set` still wins.
        match args_out.demo.map(Demo::deck).filter(|d| !d.is_empty()) {
            Some(deck) => args_out.sets = deck,
            // Default demo scene: examples/coil_vortex.kir with star_flares.kir renderer.
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
    if args_out.audio_in.is_some() && (args_out.render_to.is_some() || args_out.seq_to.is_some()) {
        return Err(
            "`--audio-in` with `--render` or `--seq` — an offscreen run takes no live input, \
             because its output has to be a function of its arguments. Drop one of them"
                .to_string(),
        );
    }
    if args_out.midi_in.is_some() && (args_out.render_to.is_some() || args_out.seq_to.is_some()) {
        return Err(
            "`--midi-in` with `--render` or `--seq` — an offscreen run takes no live input, \
             because its output has to be a function of its arguments. Drop one of them"
                .to_string(),
        );
    }
    let offscreen = args_out.render_to.is_some() || args_out.seq_to.is_some();
    // Reject interactive control surfaces for offscreen renders or replays.
    if args_out.mcp.is_some() && (offscreen || args_out.replay.is_some()) {
        return Err(
            "`--mcp` with `--render`, `--seq` or `--replay` — an offscreen run takes no \
             live input, because its output has to be a function of its arguments"
                .to_string(),
        );
    }
    // Reject external tempo sources for offscreen renders or replays.
    if args_out.tempo_source.is_some() && (offscreen || args_out.replay.is_some()) {
        return Err(
            "`--tempo-source` with `--render`, `--seq` or `--replay` — an offscreen run \
             takes no live input, and a replay follows the grid the session recorded"
                .to_string(),
        );
    }
    if args_out.midi_map.is_some() && args_out.midi_in.is_none() {
        return Err(
            "`--midi-map` with no `--midi-in` — there is no surface for the map to be of"
                .to_string(),
        );
    }
    // Window preview size is invalid without a preview window.
    if size_given && (offscreen || args_out.replay.is_some()) {
        return Err(
            "`--size` sets the preview window, and this run has no window. \
             Use `--canvas` for what is rendered"
                .to_string(),
        );
    }
    // Recording live timeline is invalid for non-interactive runs.
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

/// Validates canvas dimensions against GPU texture limits.
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

/// Returns the pseudo-random seed for the given deck slot.
pub(crate) fn seed_for(slot: usize) -> u32 {
    SEED.wrapping_add((slot as u32).wrapping_mul(0x9E37_79B9))
}

/// Returns salts for each geometry slot, using recorded values or engine defaults.
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

/// Opens the store at `args.store`, exiting on error.
pub(crate) fn open_store(args: &Args) -> karakuri_store::store::Store {
    match karakuri_store::store::Store::open(&args.store) {
        Ok(store) => store,
        Err(e) => {
            eprintln!("karakuri-cli: store `{}`: {e}", args.store.display());
            std::process::exit(1);
        }
    }
}

/// Summarizes sets present in the store as a formatted string.
pub(crate) fn listed_sets(
    store: &karakuri_store::store::Store,
    root: &std::path::Path,
) -> Result<String, String> {
    let mut sets =
        setfile::summarise(store).map_err(|e| format!("store `{}`: {e}", root.display()))?;
    if sets.is_empty() {
        return Ok(format!(
            "no sets in `{}` — nothing has been kept here yet. `--save-set ID` writes \
             one, and so does the `k` key during a run.\n",
            root.display()
        ));
    }
    // Sort most recent first, breaking ties by id.
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

/// Formats a listing of stored sets at the given root directory.
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

/// Bundles a Set and its dependencies into a standalone NDJSON payload.
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

/// Imports a Set file or bundled archive into the store.
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

/// Returns the camera configuration recorded in a loaded Set file for slot 0.
pub(crate) fn recorded_camera(args: &Args, slot: usize) -> Option<karakuri_engine::camera::Orbit> {
    match (slot, args.from_set.as_ref()) {
        (0, Some(from_set)) => from_set.camera,
        _ => None,
    }
}

/// Resolves whether a slot composites or overdraws renderers.
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

/// Returns the layering mode recorded in a loaded Set file for slot 0.
pub(crate) fn recorded_layering(args: &Args, slot: usize) -> karakuri_engine::set::Layering {
    match (slot, args.from_set.as_ref()) {
        (0, Some(from_set)) => from_set.layering,
        _ => karakuri_engine::set::Layering::Overdraw,
    }
}

/// Returns the live renderer index recorded in a loaded Set file for slot 0.
pub(crate) fn recorded_live(args: &Args, slot: usize) -> Option<u32> {
    match (slot, args.from_set.as_ref()) {
        (0, Some(from_set)) => from_set.live,
        _ => None,
    }
}
