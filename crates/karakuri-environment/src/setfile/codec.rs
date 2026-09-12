use std::collections::BTreeMap;

use karakuri_engine::camera::Orbit;
use karakuri_engine::set::Layering;
use karakuri_engine::{Binding, ParamWrite};
use karakuri_ir::typed::Checked;
use karakuri_ir::Kind;
use karakuri_store::hash::Hash;
use karakuri_store::ndjson::Line;
use karakuri_store::record::{Layer, NodeAddress, Record, Value};
use karakuri_store::store::Store;

use crate::compile::Names;
use crate::meta::{kind_name, kind_of, layer_name, layer_of};
use crate::Asked;

use super::binding::{
    binding_from_record, layer_from_ordinal, layer_ordinal, record_from_binding, ParamKey,
    ParamRecord,
};
use super::bundle::part_at;
use super::types::{Loaded, Node, Saving, VERSION};

/// **Refused rather than written into a file that cannot be read back.**
///
/// This used to refuse an L2, an L3 or a `kind Field` outright, because a Set
/// file recorded "the L1, and every other path": everything else went out as an
/// `L4` slot, and reloading as `slot L4 needs a L4 procedure, got Field` was the
/// *good* case — the bad one was an L2 whose `deform` the loader handed to a
/// renderer. The format has always had a slot per layer, so a chain is now
/// written as the chain it is and none of that is left to refuse.
///
/// What is left is what a Set *is*, and what the projection can fold. A file
/// with no geometry or nothing to draw describes no Set; two nodes at one
/// address fold to one node — see `key_for` in `project.rs` — and a gap in an
/// index describes a chain with a hole in it, which [`from_lines`] refuses on
/// the way back in rather than closing up.
fn refuse_unwritable(nodes: &[Node], capacities: &[u32], seeds: &[u32]) -> Result<(), String> {
    let count = |layer: Kind| nodes.iter().filter(|n| n.layer == layer).count();
    let geometries = count(Kind::L1);
    if geometries == 0 {
        return Err(
            "none of these files declares `kind L1`, and a Set is a geometry and the nodes \
             over it"
                .to_string(),
        );
    }
    if count(Kind::L4) == 0 {
        return Err(
            "none of these files declares `kind L4`, and a Set with no renderer has no frame \
             to give"
                .to_string(),
        );
    }
    if capacities.len() != geometries {
        return Err(format!(
            "{geometries} geometr{} and {} capacit{} — a capacity sizes one geometry, so \
             there is exactly one per L1 node",
            if geometries == 1 { "y" } else { "ies" },
            capacities.len(),
            if capacities.len() == 1 { "y" } else { "ies" },
        ));
    }
    // The same shape as the capacities, and for the same reason: a salt
    // randomises one geometry, so a file that carried a different number of
    // them would be a file where which grid is which colour depends on how a
    // reader lines two lists up.
    if seeds.len() != geometries {
        return Err(format!(
            "{geometries} geometr{} and {} seed{} — a seed salts one geometry, so \
             there is exactly one per L1 node",
            if geometries == 1 { "y" } else { "ies" },
            seeds.len(),
            if seeds.len() == 1 { "" } else { "s" },
        ));
    }
    for layer in [Kind::L1, Kind::L2, Kind::L3, Kind::L4, Kind::Field] {
        let mut indices: Vec<u32> = nodes
            .iter()
            .filter(|n| n.layer == layer)
            .map(|n| n.index)
            .collect();
        indices.sort_unstable();
        if let Some((at, index)) = indices
            .iter()
            .enumerate()
            .find(|(at, index)| **index != *at as u32)
        {
            return Err(format!(
                "the {} nodes are numbered {indices:?}: index {index} is the {}, and a \
                 layer's nodes run from 0 with no gaps and no repeats",
                kind_name(layer),
                match *index < at as u32 {
                    true =>
                        "second node at that address, which is one node in the file that \
                             comes back",
                    false => "far side of a gap, which is a chain with a hole in it",
                },
            ));
        }
    }
    Ok(())
}

/// **Write a Set file.**
///
/// The file references its sources by hash rather than carrying them, so a Set
/// file is a few dozen lines a human can read rather than a copy of the
/// material. Content addressing means saving the same procedure twice stores it
/// once.
///
/// **Putting them there is the caller's** — see [`Node`] for why the writer
/// stopped reading files. What this owes is that every hash it writes was
/// already stored, and it cannot check that without reading the store back,
/// which is the caller's promise instead.
///
/// **`asked` is who the save belongs to, and it decides the directory.** An
/// operator's own act writes the library; a save asked for over MCP writes
/// `<store>/sandbox/` — [`crate::Asked`] and
/// [P-0096](../../../docs/principles/0096-the-operators-library-is-written-by-an-operators-own-act.md).
/// It is an argument at every call site rather than a default here, because a
/// writer that reached the operator's library by saying nothing is exactly the
/// shape that rule exists against.
pub fn save(store: &Store, asked: Asked, id: &str, set: Saving<'_>) -> Result<(), String> {
    let Saving {
        nodes,
        capacities,
        params,
        bindings,
        edges,
        camera,
        layering,
        live,
        seeds,
    } = set;
    refuse_unwritable(nodes, capacities, seeds)?;
    let mut lines = vec![Line::new(Record::Set {
        id: id.to_string(),
        v: VERSION,
    })];
    // **One `slot` record per node, in node order** — the geometries, the
    // deformers, the camera, the renderers, then the field. Several on a layer
    // is how the format says a stack or a chain, and it needed no new record to
    // say it: a file holding one geometry and one renderer is the two lines it
    // always was, in the order it always had them.
    //
    // Sorted here rather than trusted from the caller, so that the file is a
    // function of the Set rather than of the order somebody walked it in.
    let mut ordered: Vec<&Node> = nodes.iter().collect();
    ordered.sort_by_key(|n| (layer_ordinal(n.layer), n.index));
    for node in ordered {
        lines.push(Line::new(Record::Slot {
            at: NodeAddress {
                layer: layer_of(node.layer),
                index: node.index,
            },
            // **Written only where the operator wrote one.** A name belongs to
            // the use rather than to the procedure, so a bare path has none to
            // record, and an absent name is written as nothing — which is what
            // keeps a file this build saves byte for byte the file it saved
            // before the field existed.
            name: node.name.clone(),
            proc_hash: node.hash,
        }));
    }
    // **Which node of the L3 layer the built-in orbit is**: after every camera
    // procedure this file names, which is `0` where it names none. Worked out
    // once and read twice — by the `param` run below, which leaves that node's
    // three to the `camera` record, and by the `camera` record itself, which
    // says which node it is about. Two derivations of one index is how the two
    // would come to disagree about which node they were talking round.
    let builtin_camera = nodes.iter().filter(|n| n.layer == Kind::L3).count() as u32;
    // On L1, because that is the layer whose element buffers a capacity sizes,
    // and one per geometry, because that is what it sizes: each source declares
    // its own range and one number cannot serve two of them.
    for (index, value) in capacities.iter().enumerate() {
        lines.push(Line::new(Record::Capacity {
            at: NodeAddress {
                layer: Layer::L1,
                index: index as u32,
            },
            value: *value,
        }));
    }
    // Sorted, so saving the same state twice produces the same file. A
    // `HashMap`'s order is not a property anything should depend on, and a Set
    // file that differed run to run would make every diff meaningless.
    // Keyed by the address as well as the name, so a wildcard write and a
    // write addressed at one node are two lines rather than one overwriting the
    // other. `None` sorts first, which puts the Set-wide value above the
    // narrower ones that override it — the order a reader wants.
    //
    // **The built-in camera's three are skipped here, and the `camera` record
    // below is where they are written.** They are that node's parameters and
    // `Set::params` reports them as such
    // (`docs/adr/0318-the-built-in-cameras-three-placement-numbers-are-parameter-rows.md`),
    // so without this a file would carry `radius` twice — once as a `param` at
    // `L3:n` and once on the `camera` line — and two spellings of one fact
    // leave a reader asking which a writer meant by choosing the other
    // (`docs/contributing.md` §4). It is the `slot` rule one level down: the
    // built-in has no `slot` record because it has no procedure to reference,
    // and no `param` records because the record that describes it carries its
    // values.
    //
    // **Here rather than in each caller.** Both save paths hand over
    // `Set::params` whole, and a filter in two callers is a filter one of them
    // forgets — the writer is what knows the format, so the writer is what
    // knows which node the format spells another way.
    let ordered: BTreeMap<ParamKey<'_>, f32> = params
        .iter()
        .filter(|w| w.at != Some((Kind::L3, builtin_camera)))
        .map(|w| {
            let at = w.at.map(|(layer, i)| (layer_ordinal(layer), i));
            ((at, w.key.as_str()), w.value)
        })
        .collect();
    for ((at, key), value) in ordered {
        lines.push(Line::new(Record::Param {
            // **`at` is `None` for an unaddressed write**, which still means
            // every node declaring the name — see `Record::Param` and
            // `node_or_every_node`, which is what lets this stay `None`
            // rather than reinventing the placeholder layer the format used
            // to need.
            at: at.map(|(l, i)| NodeAddress {
                layer: layer_from_ordinal(l),
                index: i,
            }),
            key: key.to_string(),
            value: Value::Scalar(value),
        }));
    }
    for binding in bindings {
        lines.push(Line::new(record_from_binding(binding)));
    }
    // **After the slots, because an edge is written in terms of what they
    // name.** A reader that has met every `slot` record has every name in hand
    // — including the ones nobody wrote, which are derived from the procedure
    // and so are a function of the artifact the slot references.
    //
    // Written as given rather than resolved to addresses. An edge is between
    // *names* and that is the whole of why it exists: an address moves when the
    // list is reordered, which is the failure `--set` position 1 was.
    for edge in edges {
        lines.push(Line::new(Record::Edge {
            node: edge.node.clone(),
            slot: edge.slot.as_str().into(),
            to: edge.to.clone(),
        }));
    }
    // **Written only where the Set composites, because the record's presence
    // is the whole statement.** A Set that overdraws writes no line here: there
    // is no boolean field for one to say `false` with, and a file that could
    // spell overdrawing two ways — absent, and present-and-false — would leave
    // a reader asking what a writer meant by choosing the other. So an
    // overdrawing Set is the file it always was, byte for byte.
    //
    // **After the edges and before the camera**, which is the order
    // `docs/ir-spec.md` shows the format in. It is a node with no procedure
    // described by a record of its own, exactly as the built-in camera below
    // is, so it is written beside it.
    //
    // **`live` only here.** A selection is carried by the record that says the
    // Set composites, so an overdrawing Set's edges — which exist and which
    // nothing reads — leave nothing behind: that is `Record::Select`'s
    // position, unchanged, and the reason `Set::select_renderer` is silently
    // ineffective there rather than refused.
    if layering == Layering::Composite {
        lines.push(Line::new(Record::Merge { live }));
    }
    // **The built-in camera is the last node of the L3 layer**, and the index
    // is written for the same reason a `seed`'s and a `capacity`'s are: the
    // record describes one producer, and a layer that holds several needs to
    // say which. It is `L3:0` in a Set whose files declare no camera procedure,
    // which is every Set written before they could — and zero is not written,
    // so those files are the line they always were.
    //
    // **Its three placement numbers, and they are the built-in camera node's
    // parameters** — so what is written here is `Set::orbit`, the map's
    // answer, and not the `Orbit` a Set was last *stated* with. A caller that
    // handed over the field would save the number nobody has been moving.
    lines.push(Line::new(Record::Camera {
        kind: "orbit".to_string(),
        index: builtin_camera,
        radius: camera.radius,
        speed: camera.speed,
        height: camera.height,
    }));
    // On L1, because that is the layer whose randomness a salt moves, and one
    // per geometry, because that is what it salts. **Recorded rather than left
    // to be derived**, which is what `docs/ir-spec.md` asks for: a value
    // derived from a position in `--set` changes when the list is reordered,
    // and a value read back from here does not. Index 0 is absent from the
    // record, so a Set of one geometry is the line it always was.
    for (index, value) in seeds.iter().enumerate() {
        lines.push(Line::new(Record::Seed {
            stream: Layer::L1,
            index: index as u32,
            value: u64::from(*value),
        }));
    }

    match asked {
        Asked::Operator => store.write_set(id, &lines),
        Asked::Model => store.write_sandbox_set(id, &lines),
    }
    .map_err(|e| format!("writing set `{id}`: {e}"))
}

/// **Read a Set file back into what the engine takes.**
///
/// A source is resolved from the inlined `src` records when the file carries
/// them — the bundled form — and from the store otherwise. Bundling wins
/// because a file that carries its own source is meant to be readable on a
/// machine whose store has never seen it.
pub fn load(store: &Store, id: &str) -> Result<Loaded, String> {
    let lines = store
        .read_set(id)
        .map_err(|e| format!("reading set `{id}`: {e}"))?;
    from_lines(store, id, &lines)
}

/// The decode, over lines that are already in hand. Split out so a test can
/// build a file in memory and so a session stream's head can be loaded the same
/// way once anything writes one.
pub fn from_lines(store: &Store, id: &str, lines: &[Line]) -> Result<Loaded, String> {
    let mut notes = Vec::new();
    // Every layer's slots by index, the layers in the order [`layer_ordinal`]
    // gives them. `None` is a gap — an index nothing claimed — which is refused
    // below rather than silently closed up.
    let mut slots: [Vec<Option<Hash>>; 5] = Default::default();
    // What each of those is called, in the same shape and by the same index, so
    // a name and the artifact it belongs to are placed by one statement.
    let mut slot_names: [Vec<Option<String>>; 5] = Default::default();
    // The edges the file recorded, in the order it recorded them.
    let mut edges: Vec<karakuri_engine::set::Edge> = Vec::new();
    let mut inlined: BTreeMap<Hash, BTreeMap<u32, String>> = BTreeMap::new();
    // What each geometry runs at, by index, growing as the file names them.
    let mut capacities: Vec<Option<u32>> = Vec::new();
    // **The `param` records as written**, expanded into [`ParamWrite`]s once
    // the procedures are checked — see the `Record::Param` arm.
    let mut param_records: Vec<ParamRecord> = Vec::new();
    let mut params = Vec::new();
    let mut bindings = Vec::new();
    let mut camera = None;
    // **Whether the file said this Set composites, and which renderer it left
    // selected.** `Overdraw` until a `merge` record says otherwise, because
    // that is what the record's absence has always meant — every file written
    // before it existed says overdraw by saying nothing.
    let mut layering = Layering::Overdraw;
    let mut live: Option<u32> = None;
    // Which camera node the `camera` record above was about, checked against
    // the file's L3 slots once every one of them has been met.
    let mut camera_index = 0;
    // What each geometry is salted with, by index, growing as the file names
    // them — the same shape as `capacities`, because a `seed` is addressed the
    // same way and for the same reason.
    let mut salts: Vec<Option<u32>> = Vec::new();
    let mut file_id = id.to_string();

    for line in lines {
        match line.record() {
            Record::Header { .. } => {}
            Record::Set { id, v } => {
                if *v != VERSION {
                    notes.push(format!(
                        "the file says version {v} and this build writes {VERSION}; \
                         reading it anyway"
                    ));
                }
                file_id = id.clone();
            }
            Record::Slot {
                at,
                name,
                proc_hash,
            } => {
                let (layer, index) = (&at.layer, &at.index);
                // **A name is carried now, where it used to be reported and
                // dropped.** "Nothing this build points at a node by name" was
                // true until an `edge` did: an edge names the node that
                // declares a slot and the node bound to it, so a Set whose
                // names were dropped on the way in is a Set whose edges resolve
                // against the wrong spellings — or against none.
                //
                // **Placed by layer and index, and every layer reads the same
                // way.** A second `slot` on a layer is a second node — another
                // geometry, another deformer in the chain, another renderer over
                // the same points — rather than a correction of the first, and
                // the index says which, so the records need not arrive in order
                // and the projection can fold them without one.
                //
                // Three of the five layers used to be dropped with a note saying
                // a Set file records an L1 and its renderers, and the L1's own
                // index was dropped beside them: the format could always say a
                // chain and this loader could not read one back, so a cube
                // morphing into a sphere could be played and not kept.
                let at = *index as usize;
                let ordinal = layer_ordinal(kind_of(*layer)) as usize;
                let layer_slots = &mut slots[ordinal];
                if layer_slots.len() <= at {
                    layer_slots.resize(at + 1, None);
                }
                if layer_slots[at].is_some() {
                    notes.push(format!(
                        "two {} slots both claim index {at}; the later one is used",
                        layer_name(*layer)
                    ));
                }
                layer_slots[at] = Some(*proc_hash);
                let layer_names = &mut slot_names[ordinal];
                if layer_names.len() <= at {
                    layer_names.resize(at + 1, None);
                }
                layer_names[at] = name.clone();
            }
            // **A node this file has not resolved, in a file whose whole
            // claim is that everything in it is.** Refused, and it is the one
            // record here refused rather than noted.
            //
            // Every other line this decoder cannot honour is reported and
            // skipped, because skipping one leaves the Set whole — a `gain`
            // belongs to a deck, a `param_decl` to an artifact, and a Set built
            // without either is the Set the file describes. A `part` is a
            // **node**: skip it and the chain comes up a geometry short, or
            // with a hole at an index nothing claims, and what an operator gets
            // is a Set that draws the wrong picture rather than a file that was
            // refused. That is precisely the "checking clean and coming up
            // short later" this reader exists not to do.
            //
            // Nor is it resolved here, which is the other tempting answer. This
            // function is handed lines and an id; the directory an authoring
            // file's paths are relative to is not among them, and a decoder
            // that went looking for one would be resolving the filesystem at
            // the moment of a swap — the one thing `.kbset` exists to promise
            // it never does (ADR-0231). [`resolve`](crate::setfile::resolve) is where a `part` becomes a
            // `slot`, before the file is a store's.
            Record::Part {
                layer,
                index,
                name,
                path,
            } => {
                return Err(format!(
                    "set `{file_id}`: {} names its `.kir` by the path `{path}`, and a resolved \
                     Set file names every node by content address — a file carrying a `part` is \
                     the authoring form, which is resolved against its own directory before it \
                     reaches a store rather than while a Set is being swapped in",
                    part_at(*layer, *index, name.as_deref(), path)
                ))
            }
            Record::Src { hash, line, s } => {
                inlined.entry(*hash).or_default().insert(*line, s.clone());
            }
            // **Keyed by the geometry it sizes.** A capacity addressed at the
            // second source used to be reported and dropped, because the engine
            // held one number per Set; it holds one per source now, so the
            // number reaches the geometry the file wrote it against rather than
            // resizing the wrong one.
            Record::Capacity { at, value } => match at.layer {
                Layer::L1 => {
                    let at = at.index as usize;
                    if capacities.len() <= at {
                        capacities.resize(at + 1, None);
                    }
                    if capacities[at].is_some() {
                        notes.push(format!(
                            "two capacities both claim L1 index {at}; the later one is used"
                        ));
                    }
                    capacities[at] = Some(*value);
                }
                other => notes.push(format!(
                    "capacity on {} was skipped: a capacity sizes a geometry, and the \
                     geometries are L1's",
                    layer_name(other)
                )),
            },
            // **Held rather than turned into writes here.** What a `param`
            // record becomes depends on what the procedures declare — a
            // `vec3` value against a `vec3` param is three writes, one per
            // component — and the procedures are not in hand until every
            // `slot` record has been met and checked. So the records are
            // collected in file order and expanded below, where the
            // declarations are.
            //
            // `at` absent is the wildcard, and `at`'s own type is what keeps a
            // record with no node meaning *every* node regardless of what
            // placeholder `layer` the wire line carries.
            Record::Param { at, key, value } => param_records.push((
                at.map(|at| (kind_of(at.layer), at.index)),
                key.clone(),
                *value,
            )),
            // **Carried as written, both ends.** Whether the nodes it names
            // are in this Set is not a question this decoder can answer — a
            // name nobody wrote is derived where the Set is built — so it is
            // asked there, once, rather than here and again there.
            Record::Edge { node, slot, to } => edges.push(karakuri_engine::set::Edge {
                node: node.clone(),
                slot: slot.as_str().into(),
                to: to.clone(),
            }),
            record @ Record::Bind { .. } => match binding_from_record(record) {
                Ok(binding) => bindings.push(binding),
                // Reported and skipped rather than failing the load: one
                // unusable binding is not a reason to refuse the material, and
                // the note says exactly which parameter will not move.
                Err(message) => notes.push(format!("{message} — skipped")),
            },
            Record::Camera {
                kind,
                index,
                radius,
                speed,
                height,
            } => {
                if kind != "orbit" {
                    notes.push(format!(
                        "camera kind `{kind}` is not one this engine has; using an orbit"
                    ));
                }
                // **The index is checked against where the built-in actually
                // is**, which is after the camera procedures the file names —
                // so it is `0` in a file that names none, which is every file
                // written before this record carried an index. An index naming
                // a camera that *is* a procedure is said rather than applied:
                // such a node writes its own six numbers every frame and would
                // overwrite these before the first draw.
                //
                // Checked after the loop, where the `slot` records have all
                // been met: a record's meaning here depends on how many
                // cameras the file names, and the count is only complete at
                // the end.
                camera_index = *index;
                // **The record carries the three placement numbers**, which are
                // the three an operator can move: they are the camera node's
                // parameters
                // (`docs/adr/0318-the-built-in-cameras-three-placement-numbers-are-parameter-rows.md`).
                // The lens three take their declared defaults, and that is not
                // the gap the same sentence used to name — nothing anywhere
                // moves them, so there is nothing about them a save could lose.
                // It carried two until 2026-09-09, and `height` absent reads as
                // the default the file meant by leaving it out.
                camera = Some(Orbit {
                    radius: *radius,
                    speed: *speed,
                    height: *height,
                    ..Orbit::default()
                });
            }
            // **This Set composites its renderers**, which is the one thing a
            // Set knew about itself that this file could not say. The record's
            // presence is the whole statement — there is no field to read —
            // and `live` beside it is which renderer the fold is left with.
            //
            // **Absent `live` is every input live and emphatically not node
            // 0**: reading it as 0 would silence every renderer but the first
            // in every composited Set that was saved without a selection, which
            // is a picture nobody asked for. Checked against the renderers the
            // file names after the loop, where the count is complete.
            Record::Merge { live: at } => {
                layering = Layering::Composite;
                live = *at;
            }
            // **One per geometry, and each reaches the source it names.** This
            // took index 0's and reported the rest, because the engine took one
            // salt per Set and derived each source's from it. It takes them per
            // source now, so a Set of two grids comes back with the colours it
            // was saved with however its records are ordered — which is the
            // whole of what recording a salt buys over deriving one.
            Record::Seed {
                stream,
                index,
                value,
            } => match *stream {
                Layer::L1 => {
                    let at = *index as usize;
                    if salts.len() <= at {
                        salts.resize(at + 1, None);
                    }
                    if salts[at].is_some() {
                        notes.push(format!(
                            "two seeds both claim L1 index {at}; the later one is used"
                        ));
                    }
                    salts[at] = Some(*value as u32);
                }
                other => notes.push(format!(
                    "seed on {} was skipped: a seed salts a geometry, and the geometries \
                     are L1's",
                    layer_name(other)
                )),
            },
            // Not a Set file's, and each for its own reason — see
            // `Record::is_set_state`.
            Record::Tick { .. }
            | Record::Audio { .. }
            | Record::Tempo { .. }
            | Record::Gain { .. }
            | Record::Opacity { .. }
            | Record::Blend { .. }
            | Record::Residency { .. }
            | Record::Look { .. }
            // The whole fold's own level, which is a deck's and never a
            // Set's: it describes what the mix produced rather than one of
            // the things that went into it. See `Record::MasterOut`.
            | Record::MasterOut { .. }
            // And what that fold's output is put through, one pass along: a
            // Set file carrying the master chain's slots would reconfigure the
            // master the moment it was loaded into any deck. See
            // `Record::MasterChain`.
            | Record::MasterChain(_)
            | Record::Canvas { .. }
            | Record::Procedure { .. }
            // An authority names a node the way the records above it do, and
            // is still not a Set file's: it says which agent an operator let at
            // that node during a performance, and a Set file obeying one would
            // hand the node over on every load. See `Record::Authority`.
            | Record::Authority { .. }
            // **A `ride` is a `param` that names a deck slot**, and that is
            // the whole of why it is here: what an operator turned during a
            // performance is a session's fact, and a Set file that obeyed one
            // would restore a knob position wherever it was next loaded, in
            // whatever slot it landed in. The value an operator ended on
            // reaches a Set file the way every other value does — through
            // `save`, off the live Set. See `Record::Ride`.
            | Record::Ride { .. }
            // **A `source` is a `bind` that names a deck slot**, on the
            // `ride` above's terms one field along: what an operator attached
            // to a knob during a performance is a session's fact, and a Set
            // file obeying one would attach a signal wherever that file was
            // next loaded and into whatever slot it landed in. The
            // attachments a Set ends up with reach a Set file as `bind`
            // records, through `save`, off the live Set. See `Record::Source`.
            | Record::Source { .. }
            | Record::Transport { .. }
            | Record::Transition { .. }
            // A `select` names a renderer of a *deck slot* and schedules it at
            // an instant, and nothing in a session says which slot's Set
            // composites or which slot a folded Set was played in — so a
            // stream folded down cannot tell whether a selection it meets is
            // about the Set being written. Which renderer a *Set* is left
            // folded to is `merge`'s `live`, above. See `Record::Select`.
            | Record::Select { .. }
            | Record::Mask { .. }
            // A `save` is here for a second reason as well as that one: it
            // names a Set file, and this *is* the Set file reader. Obeying it
            // would be a load that goes looking for another file.
            | Record::Save { .. } => notes.push(
                "a record that belongs to a session rather than to a Set was skipped".to_string(),
            ),
            // **An artifact's card, in a Set file.** `Store::write_set`
            // refuses to write one, so this is a hand-edited or hand-assembled
            // file — and it is skipped with a sentence of its own rather than
            // under the session one above, because the confusion it comes from
            // is a different confusion: `param_decl` says what a procedure
            // offers and `param` says what this Set turned it to, and an
            // operator who wrote the first meaning the second wants to be told
            // which one they wrote.
            Record::Meta { .. }
            | Record::ParamDecl { .. }
            | Record::CapacityDecl { .. }
            | Record::Emit { .. } => notes.push(
                "a record that belongs to an artifact's metadata rather than to a Set was \
                 skipped — a Set file records what a value is, not what a procedure declares"
                    .to_string(),
            ),
            Record::Unknown => notes.push(
                "a record type this build does not know was skipped, as the format says to"
                    .to_string(),
            ),
        }
    }

    let source_of = |layer: &str, hash: &Hash| -> Result<String, String> {
        if let Some(lines) = inlined.get(hash) {
            // Bundled: the file carries its own source, so it reads on a
            // machine whose store has never seen this artifact.
            return Ok(lines.values().cloned().collect::<Vec<_>>().join("\n"));
        }
        let bytes = store.get_artifact(hash).map_err(|e| {
            format!(
                "set `{file_id}`: {layer} is `{}` and the store does not have it ({e}); \
                 a Set file references its procedures by hash, so the artifact has to be \
                 in the store or inlined in the file",
                hash.short(12)
            )
        })?;
        String::from_utf8(bytes).map_err(|e| format!("set `{file_id}`: {layer} is not UTF-8: {e}"))
    };

    // One layer's sources, in index order. A gap is not a missing artifact:
    // index 2 with no index 1 describes a chain with a hole in it, and closing
    // it up would silently change draw order or what deforms what.
    let sources = |layer: Kind| -> Result<Vec<String>, String> {
        slots[layer_ordinal(layer) as usize]
            .iter()
            .enumerate()
            .map(|(at, hash)| match hash {
                Some(hash) => source_of(kind_name(layer), hash),
                None => Err(format!(
                    "set `{file_id}` names a {} at index {at} but none before it",
                    kind_name(layer)
                )),
            })
            .collect()
    };
    let l1_srcs = sources(Kind::L1)?;
    let l2_srcs = sources(Kind::L2)?;
    let l3_srcs = sources(Kind::L3)?;
    let l4_srcs = sources(Kind::L4)?;
    let field_srcs = sources(Kind::Field)?;
    // **The built-in camera is the last node of the L3 layer**, after the
    // procedures the file names — so a record about any earlier one is about a
    // camera that produces its own six numbers every frame, and applying these
    // to it would be overwritten before the first draw. Reported and dropped,
    // which is what this file does with everything it cannot honour.
    if camera.is_some() && camera_index as usize != l3_srcs.len() {
        notes.push(format!(
            "camera at L3:{camera_index} was skipped: the built-in orbit is this Set's L3:{}, \
             and a camera that is a procedure produces its own state",
            l3_srcs.len()
        ));
        camera = None;
    }
    // **A selection names a renderer this Set draws with**, checked here for
    // the `camera` index's reason one block above: what the number means
    // depends on how many renderers the file names, and that count is only
    // complete once every `slot` record has been met. Reported and dropped
    // rather than refused — the Set is whole, and a fold with every input live
    // is the state it would have come up in anyway.
    if live.is_some_and(|at| at as usize >= l4_srcs.len()) {
        notes.push(format!(
            "the merge selects renderer {} and this file names {} — every renderer \
             is left live",
            live.unwrap_or_default(),
            l4_srcs.len()
        ));
        live = None;
    }
    if l1_srcs.is_empty() {
        return Err(format!("set `{file_id}` has no L1 slot"));
    }
    if l4_srcs.is_empty() {
        return Err(format!("set `{file_id}` has no L4 slot"));
    }
    // **Nothing is capped here any more.** The last two refusals in this
    // function were about the plumbing rather than about the material and said
    // so; a renderer names the slot it draws through and an `edge` names which
    // camera fills it, so several cameras are several nodes with names of their
    // own and there is nothing for this to arbitrate. A file naming two now
    // loads two.
    // A capacity for a geometry the file does not name has nothing to size.
    // Said rather than dropped, because it is the file describing a source that
    // is not there — a `slot` record that went missing, most likely.
    for (at, value) in capacities.iter().enumerate().skip(l1_srcs.len()) {
        if let Some(value) = value {
            notes.push(format!(
                "capacity {value} on L1 index {at} was skipped: this file names {} \
                 geometr{}",
                l1_srcs.len(),
                if l1_srcs.len() == 1 { "y" } else { "ies" }
            ));
        }
    }
    capacities.resize(l1_srcs.len(), None);

    let check = |srcs: &[String]| {
        srcs.iter()
            .map(|src| crate::compile::check(src))
            .collect::<Result<Vec<_>, _>>()
    };
    let l1s = check(&l1_srcs)?;
    let l2s = check(&l2_srcs)?;
    let l3s = check(&l3_srcs)?;
    let l4s = check(&l4_srcs)?;
    let fields = check(&field_srcs)?;

    // -- What a `param` and a `bind` mean, now that the declarations are in hand -
    //
    // **A parameter is driven one component at a time**
    // ([ADR-0268](../../../docs/adr/0268-a-vector-parameter-is-driven-one-component-at-a-time.md)),
    // so the engine holds `glow.x`, `glow.y` and `glow.z` where a `.kir`
    // declares one `vec3 glow`. That is the whole of what this section turns a
    // record into — and the reason it is here rather than in the loop above is
    // that the width comes from the *declaration*, which the loop does not have.
    let layers: [(Kind, &[Checked]); 5] = [
        (Kind::L1, &l1s),
        (Kind::L2, &l2s),
        (Kind::L3, &l3s),
        (Kind::L4, &l4s),
        (Kind::Field, &fields),
    ];
    // **Every declaration of `key` the address reaches, as its declared type.**
    // Empty means nothing in this Set declares it — which is not an error here:
    // a scalar write against a name a regenerated artifact no longer has is
    // reported by the engine at build time and should not take the load down.
    let declared_as = |at: Option<(Kind, u32)>, key: &str| -> Vec<karakuri_ir::Ty> {
        let mut out = Vec::new();
        for (kind, procs) in layers {
            for (index, proc) in procs.iter().enumerate() {
                if at.is_some_and(|(k, i)| k != kind || i as usize != index) {
                    continue;
                }
                if let Some(p) = proc.params.iter().find(|p| p.name == key) {
                    out.push(p.ty);
                }
            }
        }
        out
    };
    // The keys a declaration of `key` at `width` is addressed by, for a
    // sentence that has to name them — `` `glow.x`, `glow.y`, `glow.z` ``.
    let component_list = |key: &str, width: usize| -> String {
        (0..width)
            .map(|i| format!("`{}`", karakuri_ir::component_key(key, i)))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let type_list = |tys: &[karakuri_ir::Ty]| -> String {
        let mut names: Vec<&str> = tys.iter().map(|t| t.name()).collect();
        names.sort_unstable();
        names.dedup();
        names
            .iter()
            .map(|n| format!("`{n}`"))
            .collect::<Vec<_>>()
            .join(" and ")
    };
    for (at, key, value) in param_records {
        let tys = declared_as(at, &key);
        let components: Vec<f32> = value.components().to_vec();
        // The first declaration this write reaches that is a vector, if any —
        // which is only used to name the components in a refusal, so the first
        // is as good as any. A wildcard reaches every node declaring the name,
        // and two procedures may declare one name at two widths: a Set the
        // format can describe and one expansion cannot serve.
        let vector_width = tys
            .iter()
            .filter_map(|t| t.param_components())
            .find(|w| *w > 1);
        if let [value] = components[..] {
            match vector_width {
                // A single number against a declaration that is three: it
                // names no component, and picking one would be inventing an
                // address the file did not write.
                Some(width) => notes.push(format!(
                    "param `{key}` was skipped: it is declared {} and a vector parameter is \
                     driven one component at a time — this Set addresses it as {}, and one \
                     number names none of them",
                    type_list(&tys),
                    component_list(&key, width)
                )),
                // A scalar onto a scalar, or onto a name nothing here declares
                // — which includes a component key a `save` wrote, since what a
                // procedure declares is `glow` and never `glow.x`.
                None => params.push(ParamWrite { at, key, value }),
            }
            continue;
        }
        let width = components.len();
        // **Every declaration has to be that width**, and there has to be one.
        // Expanding against nothing would file writes under keys no procedure
        // has, which is the "checks clean and comes up short later" this reader
        // exists not to do.
        if !tys.is_empty() && tys.iter().all(|t| t.param_components() == Some(width)) {
            for (i, value) in components.into_iter().enumerate() {
                params.push(ParamWrite {
                    at,
                    key: karakuri_ir::component_key(&key, i),
                    value,
                });
            }
            continue;
        }
        notes.push(format!(
            "param `{key}` was skipped: the file writes a `vec{width}` and {}",
            match tys.is_empty() {
                true => format!("no procedure in this Set declares `{key}`"),
                false => format!("this Set declares it {}", type_list(&tys)),
            }
        ));
    }
    // **A binding drives one number, so a bare vector key has nothing to
    // land on.** `Set::bind` refuses it — `declared_names` carries the
    // components and not the declaration — and a refusal that arrives as
    // `Bound::NoSuchParam` at build time says the parameter does not exist,
    // which is not what is wrong. Said here instead, where the declaration is,
    // and with the keys that would work in the sentence
    // ([P-0083](../../../docs/principles/0083-a-refusal-carries-what-the-next-attempt-needs.md)).
    bindings.retain(|binding: &Binding| {
        let at = binding.index.map(|i| (binding.layer, i));
        let tys = declared_as(at, &binding.key);
        let Some(width) = tys
            .iter()
            .filter_map(|t| t.param_components())
            .find(|w| *w > 1)
        else {
            return true;
        };
        notes.push(format!(
            "bind {}={} — `{}` is declared {} and a binding resolves to one number; this Set \
             addresses it as {} — skipped",
            layer_name(layer_of(binding.layer)),
            binding.key,
            binding.key,
            type_list(&tys),
            component_list(&binding.key, width)
        ));
        false
    });

    // Node order, which is what [`Loaded::srcs`] promises: the same order the
    // procedures above are chained in, so the two walk in step.
    let srcs = l1_srcs
        .into_iter()
        .chain(l2_srcs)
        .chain(l3_srcs)
        .chain(l4_srcs)
        .chain(field_srcs)
        .collect();

    // **Trimmed to the nodes that came back**, so the two lists cannot
    // disagree: a name past the end of its layer belongs to a `slot` record the
    // refusals above have already dealt with.
    let mut names = Names {
        l1s: std::mem::take(&mut slot_names[layer_ordinal(Kind::L1) as usize]),
        l2s: std::mem::take(&mut slot_names[layer_ordinal(Kind::L2) as usize]),
        l3s: std::mem::take(&mut slot_names[layer_ordinal(Kind::L3) as usize]),
        l4s: std::mem::take(&mut slot_names[layer_ordinal(Kind::L4) as usize]),
        fields: std::mem::take(&mut slot_names[layer_ordinal(Kind::Field) as usize]),
    };
    names.l1s.truncate(l1s.len());
    names.l2s.truncate(l2s.len());
    names.l3s.truncate(l3s.len());
    names.l4s.truncate(l4s.len());
    names.fields.truncate(fields.len());

    Ok(Loaded {
        id: file_id,
        l1s,
        l2s,
        l3s,
        fields,
        l4s,
        srcs,
        capacities,
        params,
        bindings,
        names,
        edges,
        camera,
        layering,
        live,
        salts,
        notes,
    })
}
