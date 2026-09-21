use super::*;

/// A node's address as the mock's `.addr` spells it — `L1:0`, `L2:0`, `L4:0`.
///
/// [`layer_word`] is the layer half and this is the whole of it, written once
/// because two bays draw it: the Inspector's node heads and, since 2026-09-09,
/// the Staging lane's rows.
pub(crate) fn node_addr(layer: Layer, index: u32) -> String {
    format!("{}:{index}", layer_word(layer))
}

/// The layer half of a node's address, as the mock's `.addr` spells it —
/// `L1:0`, `L2:0`, `L4`.
///
/// `karakuri_ir::Kind` carries no name of its own, and `karakuri-cli`'s
/// `--publish name=L4:0:key` parser is in a package with no library target, so
/// there is nothing to call. The five words are `docs/ir-spec.md`'s and this is
/// a match for [`blend_mode`]'s reason: a sixth kind stops the build here
/// rather than drawing an address nothing can be typed back in.
pub(crate) fn layer_word(layer: Layer) -> &'static str {
    match layer {
        Layer::L1 => "L1",
        Layer::L2 => "L2",
        Layer::L3 => "L3",
        Layer::L4 => "L4",
        Layer::Field => "F",
        // **A bare letter like the four above and unlike `F`'s neighbour**,
        // which is the address a press types back in — `L5:0`, on
        // `karakuri_environment::setfile::layer_ordinal`'s numbering.
        Layer::L5 => "L5",
    }
}

/// A node's layer as the vocabulary names one. Delegated to
/// [`karakuri_environment::meta::op_layer_of`] as the single source of truth.
pub(crate) fn asked_layer(layer: Layer) -> karakuri_operation::Layer {
    karakuri_environment::meta::op_layer_of(layer)
}

/// And back, delegated to [`karakuri_environment::meta::kind_of_op`].
pub(crate) fn ir_layer(layer: karakuri_operation::Layer) -> Layer {
    karakuri_environment::meta::kind_of_op(layer)
}

/// Resolves which `(Layer, u32)` node a published control targets, or `None` if ambiguous/unmatched (ADR-0200, ADR-0318, P-0090).
pub(crate) fn node_of(set: &Set, control: &Published) -> Option<(Layer, u32)> {
    if let Some(at) = control.at {
        return Some(at);
    }
    let mut declaring = set.landing_of(&control.key).into_iter();
    let first = declaring.next()?;
    match declaring.next() {
        None => Some(first),
        Some(_) => None,
    }
}

/// Finds the active signal binding for a given node parameter, if bound (ADR-0191, ADR-0286).
pub(crate) fn source_of(set: &Set, layer: Layer, index: u32, key: &str) -> Option<view::Source> {
    let binding = set
        .bindings()
        .iter()
        .find(|b| b.layer == layer && b.key == key && b.covers(index as usize))?;
    Some(view::Source {
        signal: binding.signal.clone(),
        curve: mix::curve(binding.curve),
        range: binding.range,
        at: karakuri_operation::BindAt {
            layer: asked_layer(binding.layer),
            index: binding.index,
            key: binding.key.clone(),
        },
    })
}

/// Populates the inspector panes from the active decks and aim state (ADR-0156, ADR-0191, ADR-0319).
///
/// Reads published controls, node metadata, authority settings, and bindings for each targeted deck slot.
pub(crate) fn inspector(
    deck: &Deck,
    names: &[String],
    // The wiring edges targeting each slot, used to render node `uses` lines.
    aims: &[Aiming],
    // Slot index targeted by each pane (ADR-0338).
    targets: [u8; view::PANES],
    out: &mut Vec<view::Pane>,
) {
    out.clear();
    // Populate panes in target order, stopping if target points to an unallocated deck slot.
    for target in targets {
        let slot = usize::from(target);
        if slot >= deck.slot_count() {
            break;
        }
        // The strip's name for the same slot, and for [`mixer`]'s reason: a
        // pane head reads `deck A · drift_night`, and after a load that is the
        // Set the operator chose rather than the pair the run opened with.
        let material = names.get(slot).map_or("", String::as_str);
        let addr = EngineSlot(target);
        let set = deck.slot(addr).set();
        let transport = deck.transport(addr);
        let composite = set.layering() == Layering::Composite;
        // **The deck head's two build chips**, and all three readings are of
        // what **landed** rather than of what was asked: the number the slot is
        // running at, the declaration it is measured against, and the salt the
        // next one is derived from. That is the fold's own division one field
        // over — a build may still be rolled back, and the Staging lane is what
        // says so — and it is what lets this be read off a `Set` with no aim in
        // sight (ADR-0328).
        let declared = set.declared_capacities();
        let running = set.source_capacities();
        let salts = set.source_salts();
        let aimed = match (running.first(), declared.first(), salts.first()) {
            (Some(&capacity), Some(&[_, _, default]), Some(&salt)) => Some(view::Aimed {
                capacity,
                // **Lit says the deck is not on what its material declares**,
                // which is the fact a chip can state from what landed. *An aim
                // carries a number* is the other candidate and is a reading of
                // what was asked: a hand that steps round to the declared
                // default would leave the chip lit over a slot running exactly
                // what its files say.
                stated: capacity != default,
                capacities: capacity_ladder(declared),
                salt: karakuri_engine::set::derived_salt(salt, 1),
            }),
            // **A deck with no geometry has neither chip**, which is the state
            // this `Option` is: there is no element count to size and no
            // randomness to seed.
            _ => None,
        };

        // Every published control, resolved to the node it belongs to and
        // numbered by its position in the interface — which is the number a
        // MIDI control is learned against, so it counts the controls that were
        // published and not the rows that could be placed.
        let published = set.published();
        // Controls declared by the material's default interface (ADR-0100, ADR-0329).
        // Matches by (address, key) pair to distinguish wildcards from addressed controls (ADR-0318).
        let declared = set.declared_interface();
        let mut rows: Vec<(Option<(Layer, u32)>, view::Param)> = Vec::new();
        for (at, control) in published.iter().enumerate() {
            // Addressed controls evaluate by address to resolve duplicates correctly (ADR-0318).
            let value = set
                .value_at(control.at, &control.key)
                .unwrap_or(control.range[0]);
            let node = node_of(set, control);
            rows.push((
                node,
                view::Param {
                    ord: Some(at + 1),
                    name: control.name.clone(),
                    value,
                    range: control.range,
                    param: karakuri_operation::ParamAt {
                        node: control
                            .at
                            .map(|(layer, index)| karakuri_operation::NodeAddress {
                                layer: asked_layer(layer),
                                index,
                            }),
                        key: control.key.clone(),
                    },
                    bound: node
                        .and_then(|(layer, index)| source_of(set, layer, index, &control.key)),
                },
            ));
        }
        // Include declared controls omitted from published interface (ADR-0329).
        let mut unplaced = 0;
        for control in declared {
            if published
                .iter()
                .any(|shown| shown.at == control.at && shown.key == control.key)
            {
                continue;
            }
            unplaced += 1;
            let node = node_of(set, &control);
            rows.push((
                node,
                view::Param {
                    ord: None,
                    name: control.name.clone(),
                    value: set
                        .value_at(control.at, &control.key)
                        .unwrap_or(control.range[0]),
                    range: control.range,
                    param: karakuri_operation::ParamAt {
                        node: control
                            .at
                            .map(|(layer, index)| karakuri_operation::NodeAddress {
                                layer: asked_layer(layer),
                                index,
                            }),
                        key: control.key.clone(),
                    },
                    bound: node
                        .and_then(|(layer, index)| source_of(set, layer, index, &control.key)),
                },
            ));
        }

        // **Which L3 node is the built-in camera**, which is the one node on a
        // pane with no procedure behind it and so nothing to keep.
        //
        // **The last camera node, always** — `Set::cameras`: *"Never empty,
        // and the last one is always the built-in orbit."* A Set whose files
        // declare no `kind L3` holds it at `L3:0`, and one that declares two
        // holds it at `L3:2`; either way it is the highest index on that
        // layer, so this is a `max` rather than a check for an empty layer.
        //
        // **Asked here rather than of the store**, because what a `keep` needs
        // is *is there a source at all*, and the Set is what knows. The bytes
        // themselves are the host's to find at the press — `Keeping::playing`
        // — and a pane that named a node the store cannot answer for would be
        // a capsule refusing after it was drawn.
        let builtin_camera = set
            .node_names()
            .iter()
            .filter_map(|name| set.node_named(name))
            .filter(|(layer, _)| *layer == Layer::L3)
            .map(|(_, index)| index)
            .max();
        let mut nodes: Vec<view::Node> = Vec::new();
        let mut renderers: Vec<view::Renderer> = Vec::new();
        let mut renderer_nodes = 0;
        let mut renderer_authority = None;
        let mut renderer_keep = None;
        for name in set.node_names() {
            let Some((layer, index)) = set.node_named(name) else {
                continue;
            };
            // **The address goes with the level**, because a press on a chip
            // has to say which node it is about and the two are absent
            // together — `view::NodeAuthority`, which is why this is one field
            // over there rather than two.
            let authority = set
                .authority(layer, index)
                .map(|level| view::NodeAuthority {
                    at: karakuri_operation::NodeAddress {
                        layer: asked_layer(layer),
                        index,
                    },
                    level: mix::authority(level),
                });
            let params = |layer: Layer, index: u32| {
                rows.iter()
                    .filter(|(at, _)| *at == Some((layer, index)))
                    .map(|(_, param)| param.clone())
                    .collect::<Vec<_>>()
            };
            if layer == Layer::L4 {
                // L4 renderers fold into one group displaying chips for live state.
                renderers.push(view::Renderer {
                    name: name.clone(),
                    live: composite
                        && set
                            .inputs()
                            .get(index as usize)
                            .is_some_and(|edge| edge.live),
                });
                renderer_nodes += 1;
                renderer_authority = match renderer_nodes {
                    1 => authority,
                    // **More than one node under one head has no one
                    // authority**, and authority is per node (ADR-0216). The
                    // chip is dropped rather than showing the first of them.
                    _ => None,
                };
                // **And no capsule either, for that sentence** — one `keep` on
                // a head standing over three renderers would keep one of the
                // three and say nothing about which (ADR-0338, decision 4).
                // Open the fold and each renderer has its own; a Set with one
                // renderer has one node under that head and carries the
                // capsule like any other.
                renderer_keep = match renderer_nodes {
                    1 => Some(karakuri_operation::NodeAddress {
                        layer: asked_layer(layer),
                        index,
                    }),
                    _ => None,
                };
                continue;
            }
            nodes.push(view::Node {
                addr: node_addr(layer, index),
                name: name.clone(),
                authority,
                // **Every node but the built-in camera has a source to keep**,
                // which is what `builtin_camera` above answers. The address
                // rides with it rather than beside it, for `view::Node::keep`'s
                // own reason: a head with nothing to keep has no node either.
                keep: (layer != Layer::L3 || Some(index) != builtin_camera).then_some(
                    karakuri_operation::NodeAddress {
                        layer: asked_layer(layer),
                        index,
                    },
                ),
                renderers: Vec::new(),
                uses: match aims.get(slot) {
                    Some(aiming) => uses_of(set, &aiming.at.edges, name),
                    None => Vec::new(),
                },
                params: params(layer, index),
            });
        }
        if renderer_nodes > 0 {
            // The mock's address for the folded head is the bare layer, with
            // no index — because it is not one node's.
            let mut params: Vec<view::Param> = Vec::new();
            for index in 0..renderer_nodes {
                params.extend(
                    rows.iter()
                        .filter(|(at, _)| *at == Some((Layer::L4, index)))
                        .map(|(_, param)| param.clone()),
                );
            }
            // **Published first and in interface order, then the ones off the
            // list.** `Option`'s ordering puts `None` last, which is the order
            // the rows are drawn in within a group and is the reason the sort
            // is on the whole field rather than on a position: a number a
            // reader is counting down should not step over a gap.
            params.sort_by_key(|param| param.ord);
            nodes.push(view::Node {
                addr: layer_word(Layer::L4).to_owned(),
                name: RENDERERS_NODE.to_owned(),
                authority: renderer_authority,
                keep: renderer_keep,
                renderers,
                // **The folded renderer head takes none**, and it is the same
                // reason its authority chip is dropped where it stands over
                // more than one node: a `uses` line names *one* node's
                // declaration, and this head is not a node. A renderer that
                // declares an input is reachable from the file and from a
                // model, and the panel says so rather than drawing one of
                // several answers as the answer (ADR-0216's shape).
                uses: Vec::new(),
                params,
            });
        }

        let placed: usize = nodes.iter().map(|node| node.params.len()).sum();
        if placed < published.len() + unplaced {
            // **Said rather than swallowed**, for the reason every other
            // omission in this file is said: a pane short of a row looks
            // exactly like a Set that published fewer. See `node_of` — a
            // wildcard over two or more nodes belongs to two or more groups,
            // and the mock draws no row outside one.
            println!(
                "inspector: deck {} publishes {} controls and {} of them name no one node, so \
                 they have no group to sit in and are not drawn",
                DECK_LETTERS.get(slot).copied().unwrap_or("?"),
                published.len(),
                published.len() - placed
            );
        }

        out.push(view::Pane {
            deck: slot,
            material: material.to_owned(),
            sync: mix::sync(transport.sync()),
            // **What the sync chip's cycle skips over, asked of the engine
            // three times.** `Deck::sync_allowed` is *"what a surface greys a
            // control out on, and it answers before anything is pressed"* —
            // whether the Set in this slot is closed form and whether it reads
            // `beats`, put through `Transport::allows`. The console is handed
            // the three answers rather than the two properties, because what
            // may be asked for is not a surface's to work out (P-0090) and a
            // third copy of that rule in a crate with no material in it is a
            // rule that can start disagreeing.
            //
            // **`EngineSync::ALL` is in `SYNCS`' order**, which is what makes
            // this array line up with the field it fills;
            // `the_two_crates_walk_the_sync_modes_in_one_order` is what says
            // so rather than this comment.
            allows: EngineSync::ALL.map(|mode| deck.sync_allowed(addr, mode).is_ok()),
            anchor_bpm: transport.anchor_bpm(),
            scrub_beats: transport.scrub_beats(),
            composite,
            aimed,
            nodes,
        });
    }
}

/// The `uses` lines one node draws: every input its procedure declares, with
/// the node filling each.
///
/// # The edges *are* the declarations, and that is forced rather than chosen
///
/// Nothing on a built `Set` says which inputs a node declares — the `uses`
/// declaration is read at `Set::validate` and dropped — and nothing has to,
/// because an unbound declared input is refused where the Set is built
/// (ADR-0152: *"`If there is exactly one, use it` is the implicit rule the
/// whole item exists to remove, and the refusal names the slot"*). So a slot
/// that is *running* has an edge for every input it declares, and the run's
/// edge list filtered to the nodes this Set holds is that list exactly. A
/// reader on the engine would be a second answer to a question the refusal
/// already settles.
///
/// # The candidates are the nodes on the layer the input already reaches
///
/// A `uses` slot has a type — `Geometry`, `Field`, `Camera`, `Source` — and the
/// build refused anything else, so the node currently wired is of the right
/// kind by construction and its layer is the kind. The candidates are the other
/// nodes on that layer, in node order, which is a list every entry of which the
/// build accepts.
///
/// It is inference and it is honest about being it. What this cannot do is
/// offer a kind the input takes and the deck currently reaches by no edge — a
/// `Field` input on a deck holding one field has an empty list, and the card
/// does not open. That is a control offering less than the language allows
/// rather than more, which is the side of P-0090 to be wrong on: a name this
/// misses is still reachable from a model and from `--edge`.
///
/// The declaring node is not in its own list. A node wired to itself is a cycle
/// the build refuses, and offering it would be offering a refusal.
pub(crate) fn uses_of(
    set: &karakuri_engine::set::Set,
    edges: &[karakuri_engine::set::Edge],
    node: &str,
) -> Vec<view::Uses> {
    edges
        .iter()
        .filter(|edge| edge.node == node)
        .filter_map(|edge| {
            let (kind, _) = set.node_named(&edge.to)?;
            let candidates = set
                .node_names()
                .iter()
                .filter(|name| name.as_str() != node && name.as_str() != edge.to)
                .filter(|name| set.node_named(name).is_some_and(|(at, _)| at == kind))
                .cloned()
                .collect();
            Some(view::Uses {
                slot: edge.slot.as_str().into(),
                to: edge.to.clone(),
                candidates,
            })
        })
        .collect()
}

/// What a slot's capacity chip steps through: the powers of two every one of
/// this deck's geometries would accept, ascending.
///
/// # The intersection, because a re-aim sends one number
///
/// `watch::Aim::capacity` is one `Option<u32>` for the whole slot —
/// `--capacity`'s own field, which *"overrides every source"* — so a Set
/// holding two geometries builds both at whatever this asks for, and a number
/// only one of them declares is a build the other refuses. The fold is
/// `lo.max(min)`, `hi.min(max)`, which is `declared`'s arithmetic one bay over
/// where two nodes publish one key: the range is the part every declarer
/// accepts and never any one of them on its own.
///
/// An empty intersection is an empty list, and that is a real state rather than
/// an unreachable one: two geometries whose declared ranges do not overlap have
/// no capacity a single re-aim could send. The chip is then drawn and claims
/// nothing, which is what `view::Aimed::capacities` says at the field.
///
/// Powers of two, and nothing here says why they are the right rungs — that is
/// the console's affordance and its record (`docs/adr/0328-…`); what this owes
/// is that every rung it offers is one the engine will build, which is
/// `Set::declared_capacities` being the same declaration
/// `karakuri_engine::set::capacity_in_range` refuses against
/// ([P-0090](../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
pub(crate) fn capacity_ladder(declared: &[[u32; 3]]) -> Vec<u32> {
    let Some(lo) = declared.iter().map(|at| at[0]).max() else {
        return Vec::new();
    };
    let Some(hi) = declared.iter().map(|at| at[1]).min() else {
        return Vec::new();
    };
    // `0..32` and not `0..=32`: `1u32 << 32` is undefined, and 2^31 is the
    // largest power of two a `u32` capacity can be.
    (0..32)
        .map(|k| 1u32 << k)
        .filter(|rung| (lo..=hi).contains(rung))
        .collect()
}

/// What the mock calls the group its renderer chips sit under. Not a node name
/// — every L4 node has one of those and they are the chips themselves — but the
/// head over all of them, which the mock writes as `L4 renderers`.
pub(crate) const RENDERERS_NODE: &str = "renderers";

/// A deck's renderers folded or overdrawn, performed — the Inspector deck
/// head's fold pressed, and `None` for every operation that is not one.
///
/// # It is [`played`]'s shape with one field instead of every field
///
/// A library load re-points a slot at a different Set's files; this re-points a
/// slot at *the files it is already on*, with the layering changed. Both are
/// one `Aiming::changed`, both are judged by the same watchdog, and neither
/// touches the deck — see [`Aiming::changed`], where the argument is, and
/// `docs/adr/0314-…`, which is the record.
///
/// `written` answers `Silent(NoRecord)`, exactly as it does for
/// `Operation::LoadSet`, so the surface that names it is the surface that
/// performs it and there is nothing for [`apply`] to do. What a session stream
/// has for a layering is `Record::Merge`, which is a Set file's statement about
/// a Set and carries no slot; nothing in the vocabulary says *the Set in slot 3
/// composites*, and inventing a record here would be inventing the record
/// stream (ADR-0046's rule, met from the panel).
///
/// # A press that asks for the state the slot is in is refused rather than sent
///
/// Not because asking twice is wrong — [`DeckHead::compositing`] names a
/// destination, and naming the one you are on is how the anchor beside it
/// re-anchors — but because *here* it would buy a recompile of the whole slot
/// and change nothing about the picture, which is a cost paid for nothing
/// ([P-0091](../../../docs/principles/0091-cost-is-known-before-it-is-paid.md)).
/// The chip cannot produce one, since it reads the state the frame drew; MIDI,
/// a key or a model can, the day any of them names this operation.
///
/// Every failure is a sentence and none of them moves anything: a slot the deck
/// has not got, or a build worker that has gone.
///
/// A free function over the aims and not over [`Gfx`], which is [`rewired`]'s
/// arrangement and its reason: the whole of what this decides is the field, the
/// refusal and the sentence, and none of the three needs a window, a device or
/// a `Deck` to check. [`played`] beside it takes the program because a load
/// reads a store and writes the strip's name; this one touches nothing but the
/// aim.
pub(crate) fn composited(aims: &mut [Aiming], operation: &Operation) -> Option<String> {
    let Operation::SetCompositing { deck, compositing } = operation else {
        return None;
    };
    let slot = usize::from(*deck);
    let letter = deck_letter(*deck);
    let count = aims.len();
    let Some(aim) = aims.get_mut(slot) else {
        return Some(format!(
            "  composite: {}, and nothing was re-aimed",
            karakuri_environment::no_such_slot(slot, count)
        ));
    };
    let want = match compositing {
        true => karakuri_engine::set::Layering::Composite,
        false => karakuri_engine::set::Layering::Overdraw,
    };
    let word = match compositing {
        true => "composite",
        false => "overdraw",
    };
    if aim.at.layering == want {
        return Some(format!(
            "  composite: deck {letter} is already set to {word} its renderers — nothing was \
             sent, because a re-aim rebuilds the whole slot and this one would land on the same \
             picture"
        ));
    }
    match aim.changed(|at| at.layering = want) {
        Ok(()) => Some(format!(
            "  composite: deck {letter} re-aimed to {word} its renderers — the slot is \
             recompiling on the worker, and the staging lane says whether the build landed, was \
             overloaded or did not compile"
        )),
        Err(()) => Some(format!(
            "  composite: deck {letter} will {word} its renderers from the next build on, but \
             this slot's build worker has ended, so nothing will rebuild and what is on that \
             deck is still running"
        )),
    }
}

/// A deck's element count moved, performed — the Inspector deck head's capacity
/// chip pressed, and `None` for every operation that is not one.
///
/// # It is [`composited`]'s shape with a different field of the aim
///
/// A capacity is one field of the description a slot's watcher is pointed at,
/// so this restates the other thirteen and sends it and the worker recompiles
/// the slot off the render thread — the route ADR-0228 opened and ADR-0314
/// walked, and the reason a *setter* on `Set` was never what this waited on.
/// `written` answers `Silent(NoRecord)` for `SetProperty` exactly as it does
/// for `SetCompositing` and `LoadSet`, so there is nothing for [`apply`] to do:
/// `Record::Capacity` is a Set file's statement about a Set and carries no
/// slot. A session replayed therefore does not come back at a capacity a hand
/// stepped to — the load's cost, unchanged in size; a deck kept does, since a
/// keep writes one `capacity` record per geometry off what the Set is running
/// at (`docs/adr/0328-…`).
///
/// # It is the whole slot, and that is the aim's shape rather than a shortcut
///
/// `watch::Aim::capacity` is one `Option<u32>` and is `--capacity`'s own field:
/// *"`--capacity` overrides every source"*. So a Set holding two geometries
/// runs both at this number. That is ADR-0228's recorded limit met from the
/// asking side rather than worked around, and it is why
/// `karakuri_operation::Property::Capacity` names no node.
///
/// A press asking for the capacity the slot is already aimed at is refused with
/// a sentence and nothing is sent, on the fold's terms: it would buy a
/// recompile of the whole slot and land on the same picture. The chip cannot
/// produce one — its step is strictly above what the slot is running — and a
/// model can, since `set_property` names the number outright and arrives here
/// as an `Acted::Emitted` like any press. That is why the guard is here and not
/// in the console: what may be asked for is not a surface's to decide, so every
/// way in meets the same wall in the same sentence
/// ([P-0090](../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
///
/// A free function over the aims, for [`composited`]'s reason: the field, the
/// refusal and the sentence are the whole of what it decides.
pub(crate) fn resized(aims: &mut [Aiming], operation: &Operation) -> Option<String> {
    let Operation::SetProperty {
        deck,
        property: karakuri_operation::Property::Capacity { elements },
    } = operation
    else {
        return None;
    };
    let slot = usize::from(*deck);
    let letter = deck_letter(*deck);
    let count = aims.len();
    let Some(aim) = aims.get_mut(slot) else {
        return Some(format!(
            "  capacity: {}, and nothing was re-aimed",
            karakuri_environment::no_such_slot(slot, count)
        ));
    };
    if aim.at.capacity == Some(*elements) {
        return Some(format!(
            "  capacity: deck {letter} is already aimed at {elements} elements a geometry — \
             nothing was sent, because a re-aim rebuilds the whole slot and this one would land \
             on the same picture"
        ));
    }
    match aim.changed(|at| at.capacity = Some(*elements)) {
        Ok(()) => Some(format!(
            "  capacity: deck {letter} re-aimed to {elements} elements a geometry — the slot is \
             recompiling on the worker, and the staging lane says whether the build landed, was \
             overloaded or did not compile. A number outside what a geometry declares is refused \
             there, by name and with the range"
        )),
        Err(()) => Some(format!(
            "  capacity: deck {letter} will run at {elements} elements a geometry from the next \
             build on, but this slot's build worker has ended, so nothing will rebuild and what \
             is on that deck is still running"
        )),
    }
}

/// An input rewired, performed — a pick out of a `uses` line's card, and `None`
/// for every operation that is not one.
///
/// # It is the route a model's `wire_input` already takes, reached from a press
///
/// [`rewired`] is the whole of what a rewiring decides — the run's wiring, the
/// re-aim and the sentence — and it was written for the MCP surface, one
/// request per frame, with a slot number it does not trust. A press is one
/// request of exactly that shape, so this hands it one and prints what comes
/// back: the panel and a model rewire through one function, and a defect in
/// either is a defect in both rather than in whichever was tried
/// ([P-0085](../../../docs/principles/0085-take-the-mechanism-that-exists-and-pay-the-bill-now.md)).
///
/// Nothing is validated here. The card offers nodes the pane could see and a
/// name the Set cannot use is refused where the Set is *built*, by name and
/// with what the Set does hold — which is the wall every way in meets, in one
/// sentence
/// ([P-0090](../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
///
/// `written` answers `Silent(NoRecord)` for `WireInput` and still does:
/// `Record::Edge` is a Set file's statement about a Set and carries no slot, so
/// a session replayed does not come back rewired where a hand asked for it —
/// and a keep does, since a keep writes the run's edges into the file it saves.
/// That is `SetProperty`'s division and `LoadSet`'s hole, not a new one
/// (`docs/adr/0329-…`).
pub(crate) fn wired_input(
    edges: &mut Vec<karakuri_engine::set::Edge>,
    aims: &mut [Aiming],
    slot_count: usize,
    operation: &Operation,
) -> Option<String> {
    let Operation::WireInput {
        deck,
        node,
        slot,
        to,
    } = operation
    else {
        return None;
    };
    let asked = [(
        usize::from(*deck),
        karakuri_engine::set::Edge {
            node: node.clone(),
            slot: slot.as_str().into(),
            to: to.clone(),
        },
    )];
    // **One request, so one answer** — `rewired` answers per request and this
    // hands it exactly one, which is why the `into_iter().next()` below cannot
    // be an empty list.
    let said = rewired(&asked, edges, aims, slot_count)
        .into_iter()
        .next()?;
    Some(match said {
        Ok(line) => format!("  wire: {line}"),
        Err(line) => format!("  wire: {line}"),
    })
}

/// Handles narrowing or widening the published interface of a deck (ADR-0280, ADR-0329).
///
/// Updates the slot's aim with the new list of published controls, triggering a background rebuild.
pub(crate) fn attended(aims: &mut [Aiming], operation: &Operation) -> Option<String> {
    let Operation::Publish { deck, controls } = operation else {
        return None;
    };
    let slot = usize::from(*deck);
    let letter = deck_letter(*deck);
    let count = aims.len();
    let Some(aim) = aims.get_mut(slot) else {
        return Some(format!(
            "  publish: {}, and nothing was re-aimed",
            karakuri_environment::no_such_slot(slot, count)
        ));
    };
    let published: Vec<karakuri_engine::set::Published> = controls
        .iter()
        .map(|control| karakuri_engine::set::Published {
            name: control.name.clone(),
            at: control.node.map(|node| (ir_layer(node.layer), node.index)),
            key: control.key.clone(),
            range: control.range,
        })
        .collect();
    let shown = published.len();
    match aim.changed(|at| at.published = published) {
        Ok(()) => Some(format!(
            "  publish: deck {letter} re-aimed to publish {shown} control{} — the slot is              recompiling on the worker, and the staging lane says whether the build landed, was              overloaded or did not compile. What is off the interface is still written by              `--param`, by a `param` record and by a model naming its address",
            match shown {
                1 => "",
                _ => "s",
            }
        )),
        Err(()) => Some(format!(
            "  publish: deck {letter} will publish {shown} control{} from the next build on, but              this slot's build worker has ended, so nothing will rebuild and what is on that              deck is still running",
            match shown {
                1 => "",
                _ => "s",
            }
        )),
    }
}

/// Handles re-seeding a deck's randomness salt (P-0087, P-0092).
///
/// Updates the seed salt on the aim and clears individual geometry salts to force derivation.
pub(crate) fn re_salted(aims: &mut [Aiming], operation: &Operation) -> Option<String> {
    let Operation::SetProperty {
        deck,
        property: karakuri_operation::Property::Seed { salt },
    } = operation
    else {
        return None;
    };
    let slot = usize::from(*deck);
    let letter = deck_letter(*deck);
    let count = aims.len();
    let Some(aim) = aims.get_mut(slot) else {
        return Some(format!(
            "  re-salt: {}, and nothing was re-aimed",
            karakuri_environment::no_such_slot(slot, count)
        ));
    };
    match aim.changed(|at| {
        at.seed_salt = *salt;
        at.salts.clear();
    }) {
        Ok(()) => Some(format!(
            "  re-salt: deck {letter} re-aimed to seed {salt} — the slot is recompiling on the \
             worker, and its randomness moves while its structure does not. The staging lane says \
             whether the build landed, was overloaded or did not compile"
        )),
        Err(()) => Some(format!(
            "  re-salt: deck {letter} will be seeded from {salt} from the next build on, but this \
             slot's build worker has ended, so nothing will rebuild and what is on that deck is \
             still running"
        )),
    }
}
