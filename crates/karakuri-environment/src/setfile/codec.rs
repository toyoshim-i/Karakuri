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

/// Validates that a Set configuration contains at least one geometry and one renderer,
/// exact capacity and seed counts per geometry, and contiguous node indices per layer.
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

/// Serializes a Set definition to the content-addressed store.
///
/// Operator saves target the preset library; agent/MCP saves write to `<store>/sandbox/`.
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
    // Sort nodes deterministically by layer ordinal and slot index.
    let mut ordered: Vec<&Node> = nodes.iter().collect();
    ordered.sort_by_key(|n| (layer_ordinal(n.layer), n.index));
    for node in ordered {
        lines.push(Line::new(Record::Slot {
            at: NodeAddress {
                layer: layer_of(node.layer),
                index: node.index,
            },
            // Emit custom node name only when explicitly assigned.
            name: node.name.clone(),
            proc_hash: node.hash,
        }));
    }
    // Built-in orbit camera occupies the slot immediately following procedure cameras.
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
    // Sort parameter writes by target address and key for deterministic serialization.
    // Built-in orbit placement parameters are omitted here and serialized on the camera record.
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
            // None target address represents a wildcard write affecting all nodes with matching key.
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
    // Persist edges referencing nodes by declared names rather than transient runtime indices.
    for edge in edges {
        lines.push(Line::new(Record::Edge {
            node: edge.node.clone(),
            slot: edge.slot.as_str().into(),
            to: edge.to.clone(),
        }));
    }
    // Record merge configuration only when composite layering is enabled.
    if layering == Layering::Composite {
        lines.push(Line::new(Record::Merge { live }));
    }
    // Record active built-in camera placement parameters at its assigned L3 slot index.
    lines.push(Line::new(Record::Camera {
        kind: "orbit".to_string(),
        index: builtin_camera,
        radius: camera.radius,
        speed: camera.speed,
        height: camera.height,
    }));
    // Record random seed per L1 geometry slot in index order.
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

/// Reads and decodes a Set file by ID, preferring inlined bundle sources over store lookups.
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
                // Record slot mapping and preserve optional node name for wiring edge resolution.
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
            // Authoring `part` records must be resolved to content-addressed `slot` records before load.
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
            // Size element buffers for the specified L1 geometry slot index.
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
            // Deferred expansion: param records depend on procedure declarations
            // (e.g. multi-component vectors). Absent `at` acts as wildcard for all nodes.
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
                // Camera index must point to the built-in orbit slot rather than a procedure camera.
                camera_index = *index;
                // Orbit camera placement properties (radius, speed, height).
                camera = Some(Orbit {
                    radius: *radius,
                    speed: *speed,
                    height: *height,
                    ..Orbit::default()
                });
            }
            // Composite layering mode and optional solo live renderer selection.
            Record::Merge { live: at } => {
                layering = Layering::Composite;
                live = *at;
            }
            // Geometry salt seed assigned by slot index.
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
            | Record::Mute { .. }
            | Record::Solo { .. }
            | Record::Online { .. }
            | Record::Blend { .. }
            | Record::Residency { .. }
            | Record::Look { .. }
            | Record::MasterOut { .. }
            | Record::MasterChain(_)
            | Record::Canvas { .. }
            | Record::Procedure { .. }
            | Record::Authority { .. }
            // Session-level live control rides are skipped during Set loading.
            | Record::Ride { .. }
            // Session-level signal source attachments are skipped during Set loading.
            | Record::Source { .. }
            | Record::Transport { .. }
            | Record::Transition { .. }
            // Session-level timeline selections are skipped during Set loading.
            | Record::Select { .. }
            | Record::Mask { .. }
            | Record::Save { .. }
            | Record::Policy { .. } => notes.push(
                "a record that belongs to a session rather than to a Set was skipped".to_string(),
            ),
            // Artifact declaration records are skipped during Set loading.
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
    // Verify that camera placement settings apply to the built-in orbit slot.
    if camera.is_some() && camera_index as usize != l3_srcs.len() {
        notes.push(format!(
            "camera at L3:{camera_index} was skipped: the built-in orbit is this Set's L3:{}, \
             and a camera that is a procedure produces its own state",
            l3_srcs.len()
        ));
        camera = None;
    }
    // Validate live renderer selection against available L4 count.
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
    // Check for excess capacity records exceeding declared geometry count.
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

    // Expands parameter and binding records per component declaration width (ADR-0268).
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
        // Identify declared vector parameter width for expansion or diagnostic reporting.
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
        // Every declaration must match the vector width.
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
    // Reject bindings targeting unresolved multi-component vector parameters.
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
