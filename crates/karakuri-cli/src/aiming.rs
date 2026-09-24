use super::*;

/// Tracks the active node state currently executing in each deck slot.
pub(crate) struct Running {
    pub(crate) playing: Vec<Option<Nodes>>,
}

impl Running {
    /// Seeds running state for each slot from placed nodes compiled at launch.
    pub(crate) fn at_launch(placed: &[Vec<Placed>], slots: usize) -> Running {
        let mut playing: Vec<Option<Nodes>> = vec![None; slots];
        for (slot, nodes) in placed.iter().enumerate().take(slots) {
            if nodes.is_empty() {
                continue;
            }
            playing[slot] = Some(stored_nodes(nodes.iter().map(Placed::node).collect()));
        }
        Running { playing }
    }

    /// What `slot` is running, or `None` for a slot whose sources are not in the
    /// store.
    pub(crate) fn playing(&self, slot: usize) -> Option<&Nodes> {
        self.playing.get(slot).and_then(Option::as_ref)
    }

    /// Updates and returns the running nodes for `slot` after a build completion.
    pub(crate) fn landed(&mut self, slot: usize, nodes: Option<Nodes>) -> Option<Nodes> {
        self.playing[slot] = nodes;
        self.playing[slot].clone()
    }
}

/// [`setfile::Node`]s as the addresses [`Nodes`] holds — the layer spelled the
/// way a record spells it, so a slot seeded at launch and a slot the watcher
/// rebuilt are the same shape.
pub(crate) fn stored_nodes(nodes: Vec<setfile::Node>) -> Nodes {
    nodes
        .into_iter()
        .map(|node| (setfile::kind_name(node.layer), node.index, node.hash))
        .collect()
}

/// Reads current playback configuration and state from a running `Set` into `setfile::Owned`.
pub(crate) fn playing_values(
    set: &karakuri_engine::Set,
    edges: &[karakuri_engine::set::Edge],
) -> setfile::Owned {
    setfile::Owned {
        // Filled where a store is open, and nowhere else.
        nodes: Vec::new(),
        capacities: set.source_capacities(),
        params: set
            .params()
            .map(|(layer, index, key, value)| {
                karakuri_engine::ParamWrite::at(layer, index, key, value)
            })
            .collect(),
        bindings: set.bindings().to_vec(),
        edges: edges.to_vec(),
        // Uses `Set::orbit` to preserve dynamic camera modifications (ADR-0318).
        camera: set.orbit(),
        layering: set.layering(),
        live: selected_renderer(set.inputs()),
        seeds: set.source_salts().to_vec(),
    }
}

/// Returns the index of the selected live renderer if exactly one input is active.
pub(crate) fn selected_renderer(inputs: &[karakuri_engine::mix::Input]) -> Option<u32> {
    if inputs.iter().all(|input| input.live) {
        return None;
    }
    let mut live = inputs.iter().enumerate().filter(|(_, input)| input.live);
    match (live.next(), live.next()) {
        (Some((at, _)), None) => Some(at as u32),
        _ => None,
    }
}

/// Tracks the watcher channel and target configuration for a deck slot.
pub(crate) struct Aiming {
    /// Sender channel connected to the watcher's `aimed_by` endpoint.
    pub(crate) aim: std::sync::mpsc::Sender<watch::Aim>,
    /// Last target configuration dispatched to the watcher.
    pub(crate) at: watch::Aim,
}

impl Aiming {
    /// Dispatches updated edge wiring to the slot watcher.
    pub(crate) fn re_aim(&mut self, edges: Vec<karakuri_engine::set::Edge>) -> Result<(), ()> {
        self.at.edges = edges;
        self.aim.send(restated(&self.at)).map_err(|_| ())
    }
}

/// Clones all fields of `watch::Aim` for re-pointing.
pub(crate) fn restated(aim: &watch::Aim) -> watch::Aim {
    let watch::Aim {
        head,
        rest,
        layering,
        live,
        capacity,
        seed_salt,
        salts,
        camera,
        overrides,
        published,
        bindings,
        edges,
        authorities,
        set,
    } = aim;
    watch::Aim {
        head: head.clone(),
        rest: rest.clone(),
        layering: *layering,
        live: *live,
        capacity: *capacity,
        seed_salt: *seed_salt,
        salts: salts.clone(),
        camera: *camera,
        overrides: overrides.clone(),
        published: published.clone(),
        bindings: bindings.clone(),
        edges: edges.clone(),
        authorities: authorities.clone(),
        set: set.clone(),
    }
}

/// Applies edge re-wiring requests to deck slots and dispatches re-aim updates.
pub(crate) fn rewired(
    asked: &[(usize, karakuri_engine::set::Edge)],
    edges: &mut Vec<karakuri_engine::set::Edge>,
    aims: &mut [Option<Aiming>],
    slot_count: usize,
) -> Vec<Result<String, String>> {
    let mut said: Vec<Option<Result<String, String>>> = asked.iter().map(|_| None).collect();
    let mut named: Vec<usize> = Vec::new();
    for (at, (slot, edge)) in asked.iter().enumerate() {
        if !slot_in_range(*slot, slot_count) {
            said[at] = Some(Err(format!(
                "{}, and nothing was rewired",
                no_such_slot(*slot, slot_count)
            )));
            continue;
        }
        edges.retain(|held| !(held.node == edge.node && held.slot == edge.slot));
        edges.push(edge.clone());
        if !named.contains(slot) {
            named.push(*slot);
        }
    }
    let rebuilding: Vec<(usize, Option<bool>)> = named
        .into_iter()
        .map(|slot| {
            let state = aims
                .get_mut(slot)
                .and_then(Option::as_mut)
                .map(|aiming| aiming.re_aim(edges.clone()).is_ok());
            (slot, state)
        })
        .collect();
    for (at, (slot, edge)) in asked.iter().enumerate() {
        if said[at].is_some() {
            continue;
        }
        let mut line = format!(
            "slot {slot}: wired `{}.{}={}`",
            edge.node, edge.slot, edge.to
        );
        let over = asked[at + 1..].iter().find(|(later_slot, later)| {
            slot_in_range(*later_slot, slot_count)
                && later.node == edge.node
                && later.slot == edge.slot
        });
        if let Some((_, later)) = over {
            let _ = write!(
                line,
                ", and a later request on this frame replaced it with `{}` — the run is \
                 wired with that one and it is what the rebuild carries",
                later.to
            );
        }
        let state = rebuilding
            .iter()
            .find(|(named, _)| named == slot)
            .and_then(|(_, state)| *state);
        let tail = match state {
            Some(true) => {
                " — the slot is recompiling with it, and `swap_outcome` says what the build \
                 made of it"
            }
            Some(false) => {
                " — this slot's build worker has ended, so nothing will rebuild: the edge is \
                 the run's from here on and a `save_set` of this slot records it"
            }
            None => {
                " — this slot has no watcher, so nothing rebuilds: what is on air was built \
                 with the wiring the run started with, and a `save_set` of this slot records \
                 the edge"
            }
        };
        line.push_str(tail);
        said[at] = Some(Ok(line));
    }
    said.into_iter().map(Option::unwrap).collect()
}

/// Builds the deck, hot-swap watchers, and signal oscillator for all slots.
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_deck(
    gpu: &Gpu,
    procs: &[Material],
    args: &Args,
    watch: bool,
    meters: bool,
    width: u32,
    height: u32,
    stored: Option<(
        std::sync::Arc<karakuri_store::store::Store>,
        std::sync::mpsc::Sender<watch::Built>,
    )>,
    snapshots: Option<history::Shared>,
) -> (Deck, Vec<Option<Aiming>>) {
    let mut attached = vec![false; args.bindings.len()];
    // Resolve salts once per slot for both Set construction and watcher rebuilds.
    let salts: Vec<Vec<u32>> = procs
        .iter()
        .enumerate()
        .map(|(slot, material)| {
            salts_for(
                seed_for(slot),
                recorded_salts(args, slot),
                material.l1s.len(),
            )
        })
        .collect();
    let built: Vec<(HotSwap, Option<Aiming>)> = procs
        .iter()
        .enumerate()
        .map(|(slot, material)| {
            let (l1, l2s, l3s, fields, l4s) = (
                material.l1s.as_slice(),
                &material.l2s,
                material.l3s.as_slice(),
                material.fields.as_slice(),
                &material.l4s,
            );
            let camera = recorded_camera(args, slot);
            let layering = layering_for(args, slot, recorded_layering(args, slot));
            let live = recorded_live(args, slot);
            let set = build(
                gpu,
                l1,
                l2s,
                l3s,
                fields,
                l4s,
                layering,
                &material.names,
                &args.edges,
                &capacities_for(args, l1, recorded_capacities(args, slot)),
                &mut attached,
                &args.overrides,
                &args.bindings,
                &args.published,
                salts[slot]
                    .first()
                    .copied()
                    .unwrap_or_else(|| seed_for(slot)),
                &salts[slot],
                camera,
                live,
            );
            if watch {
                let at = watch::Aim {
                    head: args.sets[slot].0.clone(),
                    rest: args.sets[slot].1.clone(),
                    layering,
                    live,
                    capacity: args.capacity_given.then_some(args.capacity),
                    seed_salt: salts[slot]
                        .first()
                        .copied()
                        .unwrap_or_else(|| seed_for(slot)),
                    salts: salts[slot].clone(),
                    camera: camera.unwrap_or_default(),
                    overrides: args.overrides.clone(),
                    published: args.published.clone(),
                    bindings: args.bindings.clone(),
                    edges: args.edges.clone(),
                    authorities: Vec::new(),
                    set: match slot {
                        0 => args.load_set.clone(),
                        _ => None,
                    },
                };
                let (aim, aimed) = std::sync::mpsc::channel();
                let swap = HotSwap::new(&gpu.device, &gpu.queue, set, args.budget_ms, {
                    let watcher = watch::Watch::new(
                        slot,
                        at.head.clone(),
                        at.rest.clone(),
                        at.layering,
                        at.live,
                        at.capacity,
                        at.seed_salt,
                        at.salts.clone(),
                        at.camera,
                        at.overrides.clone(),
                        at.published.clone(),
                        at.bindings.clone(),
                        at.edges.clone(),
                        at.authorities.clone(),
                    )
                    .aimed_by(aimed);
                    let watcher = match &snapshots {
                        Some(shared) => watcher.snapshotting_to(shared.clone(), at.set.clone()),
                        None => watcher,
                    };
                    Box::new(match &stored {
                        Some((store, tx)) => watcher.storing_to(store.clone(), tx.clone()),
                        None => watcher,
                    })
                });
                (swap, Some(Aiming { aim, at }))
            } else {
                (HotSwap::fixed(set), None)
            }
        })
        .collect();
    let (swaps, aims): (Vec<HotSwap>, Vec<Option<Aiming>>) = built.into_iter().unzip();
    let mut deck = Deck::new(&gpu.device, swaps, width, height);
    deck.set_signals(Signals::new(args.bpm, u64::from(SEED)));
    if meters {
        deck.enable_meters(&gpu.device);
    }
    // Describe bindings that attached to slots.
    for (binding, _) in args.bindings.iter().zip(&attached).filter(|(_, on)| **on) {
        eprintln!("  {}", describe(binding, deck.signals()));
    }
    (deck, aims)
}

/// One binding, in a line, ending with what it will do rather than only what it
/// says.
fn describe(binding: &Binding, signals: &Signals) -> String {
    // Published controls resolve directly with full confidence.
    let confidence = if binding.signal.starts_with(CONTROL_PREFIX) {
        1.0
    } else if binding.signal == NOISE_SIGNAL {
        signals.noise(&binding.noise.unwrap_or_default()).confidence
    } else {
        signals.sample(&binding.signal).confidence
    };
    let effect = if binding.signal.starts_with(CONTROL_PREFIX) {
        "the published control decides it outright".to_string()
    } else if confidence >= 1.0 {
        "the signal decides it outright".to_string()
    } else {
        format!(
            "it moves {:.0}% of the way and the param's own value holds the rest",
            confidence * 100.0
        )
    };
    format!(
        "bind {:?} {} <- {} through {} onto [{}, {}] — confidence {confidence:.2}, so {effect}",
        binding.layer,
        binding.key,
        binding.signal,
        binding.curve.name(),
        binding.range[0],
        binding.range[1],
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn build(
    gpu: &Gpu,
    l1s: &[karakuri_ir::typed::Checked],
    l2s: &[karakuri_ir::typed::Checked],
    // The cameras, in node order — see `Material::l3s`. Empty leaves the Set
    // looking from the built-in orbit.
    l3s: &[karakuri_ir::typed::Checked],
    // The fields, in node order — see `Material::fields`. Empty for a Set that
    // evaluates none.
    fields: &[karakuri_ir::typed::Checked],
    l4s: &[karakuri_ir::typed::Checked],
    layering: karakuri_engine::set::Layering,
    // What each node is called — see `Names`.
    names: &Names,
    // Which node fills each declared input slot — see `Args::edges`. Every one
    // the run was given, including any about another slot's Set, which the
    // engine passes over.
    edges: &[karakuri_engine::set::Edge],
    // One per entry in `l1s`, in the same order — see `capacities_for`.
    capacities: &[u32],
    // Set to `true` for each binding that attached, and left alone otherwise.
    // One flag per binding, and a caller building several slots ORs them: a
    // binding is worth describing if it attached *anywhere*.
    attached: &mut [bool],
    overrides: &[ParamWrite],
    bindings: &[Binding],
    published: &[karakuri_engine::set::Published],
    seed: u32,
    // One per entry in `l1s`, in the same order — see `salts_for`.
    salts: &[u32],
    camera: Option<karakuri_engine::camera::Orbit>,
    // Which renderer the Set comes up folded to — see `recorded_live`. `None`
    // leaves every input live, which is what `Set::build_many` builds and what
    // a Set nobody has selected in is.
    live: Option<u32>,
) -> Set {
    let deform: Vec<&karakuri_ir::typed::Checked> = l2s.iter().collect();
    let look: Vec<&karakuri_ir::typed::Checked> = l3s.iter().collect();
    let shapes: Vec<&karakuri_ir::typed::Checked> = fields.iter().collect();
    let draw: Vec<&karakuri_ir::typed::Checked> = l4s.iter().collect();
    assert_eq!(
        l1s.len(),
        capacities.len(),
        "one capacity per geometry source"
    );
    let sources: Vec<(&karakuri_ir::typed::Checked, u32)> =
        l1s.iter().zip(capacities.iter().copied()).collect();
    assert_eq!(l1s.len(), salts.len(), "one salt per geometry source");
    let assigned: Vec<Option<u32>> = salts.iter().copied().map(Some).collect();
    match Set::build_many(
        &gpu.device,
        &gpu.queue,
        &sources,
        &deform,
        &look,
        &shapes,
        &draw,
        layering,
        seed,
        &assigned,
        karakuri_engine::set::Wiring {
            l1s: &names.l1s,
            l2s: &names.l2s,
            l3s: &names.l3s,
            l4s: &names.l4s,
            fields: &names.fields,
            edges,
        },
    ) {
        Ok(mut set) => {
            eprintln!("  nodes: {}", set.node_names().join(", "));
            // Applies recorded camera configuration into the camera node (ADR-0318).
            if let Some(camera) = camera {
                set.aim_camera(camera);
            }
            // Restores recorded renderer selection for composited sets.
            if let Some(at) = live {
                if !set.select_renderer(at as usize) {
                    eprintln!(
                        "  this set selects renderer {at} and has {} — every renderer is \
                         live",
                        l4s.len()
                    );
                }
            }
            for write in overrides {
                match set.write_param(write) {
                    Ok(0) => eprintln!("  no parameter named `{}`, ignoring", write.key),
                    Ok(_) => {}
                    Err(refused) => eprintln!("  {refused}"),
                }
            }
            // Publish controls before resolving bindings.
            for control in published {
                let name = control.name.clone();
                if let Err(e) = set.publish(control.clone()) {
                    eprintln!("  `{name}` is not published: {e}");
                }
            }
            for (at, binding) in bindings.iter().enumerate() {
                let (layer, key) = (binding.layer, binding.key.clone());
                match set.bind(binding.clone()) {
                    karakuri_engine::set::Bound::Yes => attached[at] = true,
                    karakuri_engine::set::Bound::NoSuchParam => {
                        eprintln!("  no {layer:?} parameter named `{key}` to bind, ignoring");
                    }
                    karakuri_engine::set::Bound::NoSuchControl => eprintln!(
                        "  `{}` is not published by this Set, so {layer:?} `{key}` is \
                         not bound",
                        binding.signal
                    ),
                }
            }
            set
        }
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}
