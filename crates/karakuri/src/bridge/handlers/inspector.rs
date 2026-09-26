use super::*;

/// Formats a node's display address (e.g. `L1:0`), shared by Inspector and Staging bays.
pub(crate) fn node_addr(layer: Layer, index: u32) -> String {
    format!("{}:{index}", layer_word(layer))
}

/// Returns the address prefix string for a layer according to `docs/ir-spec.md`.
pub(crate) fn layer_word(layer: Layer) -> &'static str {
    match layer {
        Layer::L1 => "L1",
        Layer::L2 => "L2",
        Layer::L3 => "L3",
        Layer::L4 => "L4",
        Layer::Field => "F",
        // Layer L5 address format matches user input syntax (e.g. `L5:0`).
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
        // Deck head build chips, reflecting landed running capacity, declaration, and salt (ADR-0328).
        let declared = set.declared_capacities();
        let running = set.source_capacities();
        let salts = set.source_salts();
        let aimed = match (running.first(), declared.first(), salts.first()) {
            (Some(&capacity), Some(&[_, _, default]), Some(&salt)) => Some(view::Aimed {
                capacity,
                // Lit indicates running capacity differs from the material's declared default.
                stated: capacity != default,
                capacities: capacity_ladder(declared),
                salt: karakuri_engine::set::derived_salt(salt, 1),
            }),
            // Decks without geometry omit capacity and seed chips.
            _ => None,
        };

        // Number published controls sequentially by interface position for MIDI learn indexing.
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

        // Built-in orbit camera is always the highest-indexed L3 node and has no source to keep.
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
            // Associate node address with its authority level (`view::NodeAuthority`).
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
                    // Omit authority chip when multiple renderers share a folded head (ADR-0216).
                    _ => None,
                };
                // Folded renderer head only displays keep capsule if exactly one renderer exists (ADR-0338).
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
                // Nodes with editable source files expose a keep address (excluding built-in camera).
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
            // Sort published params first in interface order, followed by unlisted params.
            params.sort_by_key(|param| param.ord);
            nodes.push(view::Node {
                addr: layer_word(Layer::L4).to_owned(),
                name: RENDERERS_NODE.to_owned(),
                authority: renderer_authority,
                keep: renderer_keep,
                renderers,
                // Folded renderer head represents multiple nodes and takes no direct `uses` lines (ADR-0216).
                uses: Vec::new(),
                params,
            });
        }

        let placed: usize = nodes.iter().map(|node| node.params.len()).sum();
        if placed < published.len() + unplaced {
            // Warn on unplaced controls whose target node is ambiguous across multiple groups.
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
            // Query engine whether each sync mode is allowed for this slot (P-0090).
            // Matches `EngineSync::ALL` ordering.
            allows: EngineSync::ALL.map(|mode| deck.sync_allowed(addr, mode).is_ok()),
            anchor_bpm: transport.anchor_bpm(),
            scrub_beats: transport.scrub_beats(),
            composite,
            aimed,
            nodes,
        });
    }
}

/// Returns the declared input `uses` lines and candidate wiring targets for a node (ADR-0152, P-0090).
///
/// Candidates include other nodes on the same layer, excluding self-loops.
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

/// Returns ascending power-of-two capacities valid across all geometries in a deck (ADR-0328, P-0090).
///
/// Folds ranges into their mutual intersection, returning empty if ranges do not overlap.
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

/// Handles a compositing mode toggle for a deck slot (ADR-0046, ADR-0314, P-0091).
///
/// Updates the slot's aim layering; refuses no-op requests that match current state to avoid recompilation.
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

/// Handles a capacity chip change for a deck slot (ADR-0228, ADR-0314, ADR-0328, P-0090).
///
/// Sets slot-wide capacity override and triggers background recompilation. Refuses redundant requests.
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

/// Performs input rewiring for a deck slot via [`rewired`] (ADR-0329, P-0085, P-0090).
///
/// Re-aims the watcher to update wiring edges and triggers recompilation.
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
    // Single rewire request yields exactly one outcome.
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
