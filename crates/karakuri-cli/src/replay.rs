use super::*;

/// Resolves canvas dimensions for replay based on recorded canvas or cli flag overrides.
pub(crate) fn replay_canvas(
    recorded: Option<(u32, u32)>,
    flag: (u32, u32),
    flag_given: bool,
) -> Result<((u32, u32), Option<String>), String> {
    match (recorded, flag_given) {
        // The stream knows, so the flag is refused rather than obeyed.
        (Some(_), true) => Err(
            "`--canvas` with `--replay` — this session records what it rendered at, \
             and a replay is at that size or it is not a replay"
                .to_string(),
        ),
        (Some(size), false) => Ok((size, None)),
        // Fallback for sessions predating canvas records or written manually.
        (None, true) => Ok((
            flag,
            Some(format!(
                "no `canvas` record: replaying at the {}x{} `--canvas` asked for",
                flag.0, flag.1
            )),
        )),
        (None, false) => Ok((
            flag,
            Some(format!(
                "no `canvas` record and no `--canvas`: replaying at {}x{}, which is a \
                 guess rather than what was performed",
                flag.0, flag.1
            )),
        )),
    }
}

/// Resolves element capacity from cli flag or procedure default declaration.
pub(crate) fn capacity_for(args: &Args, l1: &karakuri_ir::typed::Checked) -> u32 {
    if args.capacity_given {
        return args.capacity;
    }
    l1.capacity
        .map_or(args.capacity, |declared| declared.default)
}

/// Formats a diagnostic note for a skipped `save` record during replay.
fn skipped_save(slot: DeckSlot, id: &str) -> String {
    format!(
        "  a `save` of slot {slot} was skipped: a replay writes no Set files. \
         The material it named is set `{id}`"
    )
}

/// Formats diagnostic notes for unhandled records occurring after the last tick.
pub(crate) fn trailing_notes(trailing: &[Record]) -> Vec<String> {
    if trailing.is_empty() {
        return Vec::new();
    }
    let mut notes = vec![format!(
        "{} record{} after the last tick belong to no frame and are not replayed",
        trailing.len(),
        if trailing.len() == 1 { "" } else { "s" }
    )];
    notes.extend(trailing.iter().filter_map(|record| match record {
        Record::Save { slot, id } => Some(skipped_save(*slot, id)),
        _ => None,
    }));
    notes
}

/// Renders a recorded session deterministically offscreen.
pub(crate) fn replay_session(args: &Args, id: &str) {
    let store = open_store(args);
    let lines = match store.read_session(id) {
        Ok(lines) => lines,
        Err(e) => {
            eprintln!("karakuri-cli: session `{id}`: {e}");
            std::process::exit(1);
        }
    };
    let stream = session::split(lines);
    let loaded = match setfile::from_lines(&store, id, &stream.head) {
        Ok(loaded) => loaded,
        Err(e) => {
            eprintln!("karakuri-cli: session `{id}`: {e}");
            std::process::exit(1);
        }
    };
    for note in &loaded.notes {
        eprintln!("  {note}");
    }
    for note in trailing_notes(&stream.trailing) {
        eprintln!("  {note}");
    }

    let Some(out) = args.render_to.clone().or(args.seq_to.clone()) else {
        eprintln!("karakuri-cli: --replay renders offscreen; give --render FILE or --seq DIR");
        std::process::exit(1);
    };
    let sequence = args.seq_to.is_some();
    // Ensure output directory exists when writing image sequences.
    if let Some(dir) = &args.seq_to {
        if let Err(e) = std::fs::create_dir_all(dir) {
            eprintln!("karakuri-cli: {}: {e}", dir.display());
            std::process::exit(1);
        }
    }
    let gpu = Gpu::headless().expect("no GPU");
    // Replay canvas size is dictated by the stream rather than flags.
    let (recorded_canvas, later_canvases) = stream.canvas();
    let (w, h) = match replay_canvas(recorded_canvas, args.canvas, args.canvas_given) {
        Ok((size, note)) => {
            if let Some(note) = note {
                eprintln!("  {note}");
            }
            size
        }
        Err(refusal) => {
            eprintln!("karakuri-cli: session `{id}`: {refusal}");
            std::process::exit(1);
        }
    };
    if later_canvases > 0 {
        eprintln!(
            "  {later_canvases} later `canvas` record{} ignored — the canvas is fixed for a run",
            if later_canvases == 1 { "" } else { "s" }
        );
    }

    // The size came out of the stream rather than off the command line, so this
    // can refuse a session recorded on a machine with a larger limit than the
    // one replaying it — which is the case a flag check could never have caught.
    check_canvas(&gpu.device, w, h);

    // Seed salts: file's own per geometry, derived only if unrecorded.
    let salts = salts_for(seed_for(0), &loaded.salts, loaded.l1s.len());
    let layering = layering_for(args, 0, loaded.layering);
    let live = loaded.live;
    let mut set = build(
        &gpu,
        &loaded.l1s,
        // The chain recorded in the session head.
        &loaded.l2s,
        &loaded.l3s,
        &loaded.fields,
        &loaded.l4s,
        // Layering recorded in head (defaults to Overdraw).
        layering,
        // Names and edges recorded in head.
        &loaded.names,
        &loaded.edges,
        // Capacities per geometry from head or procedure defaults.
        &capacities_for(args, &loaded.l1s, &loaded.capacities),
        &mut vec![false; loaded.bindings.len()],
        &loaded.params,
        &loaded.bindings,
        // An empty published controls list exposes the full interface by default.
        &[],
        // First geometry salt sets L3 seed.
        salts.first().copied().unwrap_or_else(|| seed_for(0)),
        &salts,
        loaded.camera,
        live,
    );
    set.resize(&gpu.device, w, h);
    // Build deck sized to the highest slot indexed in the head.
    // of every other slot, so this is the highest slot any of them names plus
    // one. A stream that names none is a deck of one, which is every session
    // written before a head could say what a deck held.
    let named = slots_named(&stream.opening);
    // What each slot is playing, as the stream names it, seeded from what the
    // head named. The L1, and the renderers by index — a slot draws with one
    // geometry and however many L4s, so the second half is a list.
    let mut playing: Vec<(
        Option<karakuri_store::hash::Hash>,
        Vec<Option<karakuri_store::hash::Hash>>,
    )> = vec![(None, Vec::new()); named];
    let mut opened: Vec<usize> = Vec::new();
    let mut restated: Vec<usize> = Vec::new();
    for record in &stream.opening {
        if matches!(record, karakuri_store::record::Record::Procedure { .. }) {
            place_procedure(record, &mut playing, &mut opened, &mut restated);
        }
    }
    // **What every Set this replay builds is built against**, gathered once:
    // the flags a replay was typed with for what the stream cannot say, and the
    // *stream's* own table for everything it can.
    let settings = Settings {
        args,
        layering,
        live,
        params: &loaded.params,
        bindings: &loaded.bindings,
    };
    let mut swaps = vec![HotSwap::fixed(set)];
    for (slot, playing) in playing.iter().enumerate().take(named).skip(1) {
        match rebuild(&gpu, &store, playing, slot, &settings) {
            Ok(mut set) => {
                set.resize(&gpu.device, w, h);
                swaps.push(HotSwap::fixed(set));
            }
            // Failure to rebuild an opening slot is fatal during replay initialization.
            Err(e) => {
                eprintln!(
                    "karakuri-cli: session `{id}`: slot {slot}: {e} — the head names what \
                     this deck held and a replay builds all of it or none"
                );
                std::process::exit(1);
            }
        }
    }
    let mut deck = Deck::new(&gpu.device, swaps, w, h);
    deck.set_signals(Signals::new(args.bpm, u64::from(SEED)));

    let frames = u32::try_from(stream.frames.len()).unwrap_or(u32::MAX);
    eprintln!(
        "replaying session `{id}`: {frames} frames, {w}x{h}, {} at exposure {:.2} -> {}",
        op_name(args.look.op),
        args.look.exposure,
        out.display()
    );

    // Look record updates the tone mapper or exposure across subsequent frames.
    let mut look = args.look;
    // Master chain reconstructed from head record.
    let mut chain: Vec<karakuri_engine::SlotSpec> = Vec::new();
    let mut chain_moved = false;
    // Apply head configuration to the deck prior to rendering frame 0.
    for record in &stream.opening {
        match record {
            karakuri_store::record::Record::Procedure { .. } => {}
            karakuri_store::record::Record::Save { slot, id } => {
                eprintln!("{}", skipped_save(*slot, id))
            }
            record => apply_replayed(&mut deck, &mut look, &mut chain, &mut chain_moved, record),
        }
    }
    let result = render::replay(
        &gpu,
        &mut deck,
        w,
        h,
        frames,
        |i| {
            if sequence {
                Some(out.join(format!("{i:05}.png")))
            } else if i + 1 == frames {
                Some(out.clone())
            } else {
                None
            }
        },
        &|address| mix::resolve_procedure(Some(&store), address),
        |i, deck| {
            let frame = &stream.frames[i as usize];
            // Procedures in a frame are gathered and rebuilt atomically.
            let mut changed: Vec<usize> = Vec::new();
            let mut restated: Vec<usize> = Vec::new();
            for record in &frame.before {
                if matches!(record, karakuri_store::record::Record::Procedure { .. }) {
                    place_procedure(record, &mut playing, &mut changed, &mut restated);
                    continue;
                }
                // Save records are skipped with a diagnostic notice during replay sandboxing.
                if let karakuri_store::record::Record::Save { slot, id } = record {
                    eprintln!("{}", skipped_save(*slot, id));
                    continue;
                }
                apply_replayed(deck, &mut look, &mut chain, &mut chain_moved, record);
            }
            for slot in changed {
                match rebuild(&gpu, &store, &playing[slot], slot, &settings) {
                    Ok(set) => deck.install(&gpu.device, EngineSlot(slot as u8), set),
                    Err(e) => eprintln!("  slot {slot}: {e} — it keeps what it had"),
                }
            }
            (
                frame.steps,
                look,
                chain_moved.then(|| {
                    chain_moved = false;
                    chain.clone()
                }),
            )
        },
    );
    if let Err(e) = result {
        eprintln!("karakuri-cli: {e}");
        std::process::exit(1);
    }
}

/// Returns the number of slots named in the session head, clamped to MAX_SLOTS.
fn slots_named(opening: &[Record]) -> usize {
    let widest = opening
        .iter()
        .filter_map(|record| match record {
            Record::Procedure { slot, .. } => Some(slot.index() + 1),
            _ => None,
        })
        .max()
        .unwrap_or(1);
    if widest > karakuri_engine::deck::MAX_SLOTS {
        eprintln!(
            "  the head names {widest} slots and a deck holds {} — the rest are skipped",
            karakuri_engine::deck::MAX_SLOTS
        );
    }
    widest.clamp(1, karakuri_engine::deck::MAX_SLOTS)
}

/// Places a `procedure` record into the slot's playing configuration.
fn place_procedure(
    record: &Record,
    playing: &mut [(
        Option<karakuri_store::hash::Hash>,
        Vec<Option<karakuri_store::hash::Hash>>,
    )],
    changed: &mut Vec<usize>,
    restated: &mut Vec<usize>,
) {
    let Record::Procedure {
        slot,
        at,
        proc_hash,
    } = record
    else {
        return;
    };
    let (layer, index) = (&at.layer, &at.index);
    let slot = slot.index();
    if slot >= playing.len() {
        eprintln!(
            "  a `procedure` for slot {slot} was skipped: the head named {} slot{}",
            playing.len(),
            if playing.len() == 1 { "" } else { "s" }
        );
        return;
    }
    match layer {
        Layer::L1 => playing[slot].0 = Some(*proc_hash),
        Layer::L4 => {
            if !restated.contains(&slot) {
                restated.push(slot);
                playing[slot].1.clear();
            }
            let at = *index as usize;
            if playing[slot].1.len() <= at {
                playing[slot].1.resize(at + 1, None);
            }
            playing[slot].1[at] = Some(*proc_hash);
        }
        other => {
            eprintln!("  a `procedure` for {other:?} was skipped");
            return;
        }
    }
    if !changed.contains(&slot) {
        changed.push(slot);
    }
}

/// Configuration settings for constructing Set instances during replay.
struct Settings<'a> {
    /// Command line arguments for capacities and defaults.
    args: &'a Args,
    /// Layering and selection state from the session head.
    layering: karakuri_engine::set::Layering,
    live: Option<u32>,
    /// Parameter and binding tables from the session head.
    params: &'a [karakuri_engine::ParamWrite],
    bindings: &'a [karakuri_engine::Binding],
}

/// Rebuilds a Set for a slot from recorded procedure hashes and settings.
fn rebuild(
    gpu: &Gpu,
    store: &karakuri_store::store::Store,
    playing: &(
        Option<karakuri_store::hash::Hash>,
        Vec<Option<karakuri_store::hash::Hash>>,
    ),
    slot: usize,
    settings: &Settings,
) -> Result<Set, String> {
    let Settings {
        args,
        layering,
        live,
        params,
        bindings,
    } = settings;
    let (layering, live) = (*layering, *live);
    let (Some(l1_hash), l4_hashes) = playing else {
        return Err("its L1 has not been named".into());
    };
    if l4_hashes.is_empty() || l4_hashes.iter().any(Option::is_none) {
        return Err("not every one of its renderers has been named".into());
    }
    let source = |hash: &karakuri_store::hash::Hash| -> Result<String, String> {
        let bytes = store
            .get_artifact(hash)
            .map_err(|e| format!("reading `{hash}`: {e}"))?;
        String::from_utf8(bytes).map_err(|e| format!("`{hash}` is not text: {e}"))
    };
    let l1 = compile::check(&source(l1_hash)?).map_err(|report| format!("L1:\n{report}"))?;
    let l4s = l4_hashes
        .iter()
        .flatten()
        .map(|h| compile::check(&source(h)?).map_err(|report| format!("L4:\n{report}")))
        .collect::<Result<Vec<_>, String>>()?;
    Ok(build(
        gpu,
        std::slice::from_ref(&l1),
        &[],
        &[],
        &[],
        &l4s,
        layering,
        &Names::default(),
        &[],
        &capacities_for(args, std::slice::from_ref(&l1), &[]),
        &mut vec![false; bindings.len()],
        params,
        bindings,
        &[],
        seed_for(slot),
        &salts_for(seed_for(slot), &[], 1),
        None,
        live,
    ))
}

/// Applies a single session record to the replaying deck.
fn apply_replayed(
    deck: &mut Deck,
    look: &mut Look,
    chain: &mut Vec<karakuri_engine::SlotSpec>,
    chain_moved: &mut bool,
    record: &karakuri_store::record::Record,
) {
    match record {
        karakuri_store::record::Record::Tempo { .. } => {
            let mut signals = *deck.signals();
            audio::apply_tempo(&mut signals, record);
            deck.set_signals(signals);
            return;
        }
        karakuri_store::record::Record::Audio { .. } => {
            let mut signals = *deck.signals();
            signals.set_audio(audio::audio_frame(record));
            deck.set_signals(signals);
            return;
        }
        _ => {}
    }
    match mix::change(record, deck.slot_count()) {
        Ok(Some(mix::Change::Gain { slot, value })) => {
            deck.set_gain(EngineSlot(slot as u8), value)
        }
        Ok(Some(mix::Change::Opacity { slot, value })) => {
            deck.set_opacity(EngineSlot(slot as u8), value)
        }
        Ok(Some(mix::Change::Blend { slot, mode })) => {
            deck.set_blend(EngineSlot(slot as u8), mode)
        }
        Ok(Some(mix::Change::Mask { slot, mask })) => deck.set_mask(EngineSlot(slot as u8), mask),
        Ok(Some(mix::Change::Transition {
            slot,
            control,
            to,
            start,
            beats,
            curve,
        })) => deck.schedule(schedule_from(deck, slot, control, to, start, beats, curve)),
        Ok(Some(mix::Change::Select {
            slot,
            renderer,
            start,
        })) => {
            let count = deck.slot(EngineSlot(slot as u8)).set().inputs().len();
            match renderer_in_range(slot, renderer, count) {
                Ok(()) => deck.schedule_selection(karakuri_engine::transition::Selection::new(
                    slot, renderer, start,
                )),
                Err(refusal) => eprintln!("  {refusal} — skipped"),
            }
        }
        // Parameter write updates.
        Ok(Some(mix::Change::Ride { slot, writes })) => {
            for write in &writes {
                match deck.write_param(EngineSlot(slot as u8), write) {
                    Ok(0) => eprintln!("  {} — skipped", no_such_param(slot, &write.key)),
                    Ok(_) => {}
                    Err(refused) => eprintln!("  {refused}"),
                }
            }
        }
        // Signal binding and unbinding updates.
        Ok(Some(mix::Change::Source {
            slot,
            layer,
            index,
            key,
            binding,
        })) => match binding {
            Some(binding) => match deck.bind(EngineSlot(slot as u8), binding) {
                karakuri_engine::set::Bound::Yes => {}
                karakuri_engine::set::Bound::NoSuchParam => {
                    eprintln!("  {} — skipped", no_such_param(slot, &key))
                }
                karakuri_engine::set::Bound::NoSuchControl => eprintln!(
                    "  slot {slot}: `{key}` is bound to a control this Set does not publish —                      skipped"
                ),
            },
            None => {
                if !deck.unbind(EngineSlot(slot as u8), layer, index, &key) {
                    eprintln!("  slot {slot}: nothing was driving `{key}` — skipped");
                }
            }
        },
        Ok(Some(mix::Change::Authority {
            slot,
            layer,
            index,
            authority,
        })) => {
            if !deck.set_authority(EngineSlot(slot as u8), layer, index, authority) {
                eprintln!(
                    "  slot {slot}: no node {}:{index} to make {} — skipped",
                    karakuri_environment::meta::layer_name(
                        karakuri_environment::meta::layer_of(layer),
                    ),
                    authority.name()
                );
            }
        }
        Ok(Some(mix::Change::Residency { slot, level })) => {
            deck.set_residency(EngineSlot(slot as u8), level);
            report_governing(&deck.govern(), "residency");
        }
        Ok(Some(mix::Change::Look(l))) => *look = l,
        // Master bus and chain settings.
        Ok(Some(mix::Change::MasterOut(value))) => deck.set_out(value),
        Ok(Some(mix::Change::MasterChain(slots))) => {
            *chain = slots;
            *chain_moved = true;
        }
        Ok(Some(mix::Change::Transport {
            slot,
            sync,
            anchor_bpm,
            scrub_beats,
        })) => {
            if let Err(refusal) =
                deck.set_transport(EngineSlot(slot as u8), sync, anchor_bpm, scrub_beats)
            {
                eprintln!("  slot {slot}: {} sync refused — {refusal}", sync.name());
            }
        }
        Ok(None) => {}
        Err(message) => eprintln!("  {message} — skipped"),
    }
}

/// Constructs a transition from the decoded change and current control value.
pub(crate) fn schedule_from(
    deck: &Deck,
    slot: usize,
    control: karakuri_engine::transition::Control,
    to: f32,
    start: f64,
    beats: f64,
    curve: Curve,
) -> karakuri_engine::Transition {
    use karakuri_engine::transition::Control;
    let addr = EngineSlot(slot as u8);
    let from = match control {
        Control::Gain => deck.gain(addr),
        Control::Opacity => deck.opacity(addr),
        Control::MaskPosition => deck.mask(addr).position(),
    };
    karakuri_engine::Transition::new(slot, control, from, to, start, beats, curve)
}
