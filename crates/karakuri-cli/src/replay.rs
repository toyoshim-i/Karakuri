use super::*;

/// What a replay renders at: the stream's, or the flag's, or a refusal — plus
/// a line to print when the answer is not simply what was performed.
///
/// **A function rather than four arms inside `replay_session`** for the reason
/// every decoder in this program is one: the decision needs a stream, a store
/// and a GPU to reach otherwise, and a decision nothing can test is a decision
/// that drifts. It is also the shape the refusal has to have — it cannot live
/// in `parse_args_from`, because at parse time nothing has read the stream.
pub(crate) fn replay_canvas(
    recorded: Option<(u32, u32)>,
    flag: (u32, u32),
    flag_given: bool,
) -> Result<((u32, u32), Option<String>), String> {
    match (recorded, flag_given) {
        // The stream knows, so the flag is refused rather than obeyed:
        // honouring it would render a session at a size it never ran at while
        // reporting a faithful replay.
        (Some(_), true) => Err(
            "`--canvas` with `--replay` — this session records what it rendered at, \
             and a replay is at that size or it is not a replay"
                .to_string(),
        ),
        (Some(size), false) => Ok((size, None)),
        // **The flag is the only way out for a stream that predates the record
        // or was written by hand.** Refusing it unconditionally forced every
        // such session to the default while saying "the session records what it
        // rendered at" about one that does not.
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

/// How many elements a Set gets: what was asked for, or what the procedure asks
/// for itself.
///
/// **A `.kir` declares `capacity [min, max] = default` and the default was never
/// used.** The range was enforced and the default silently lost to
/// `--capacity`'s own, so a procedure written for 131072 elements ran at 262144
/// unless somebody knew to say so — and an example whose point is visible only
/// at the count it was written for did not show its point. The same shape as
/// `--size` overriding a canvas: a general flag with a default beating a
/// specific declaration that meant it.
pub(crate) fn capacity_for(args: &Args, l1: &karakuri_ir::typed::Checked) -> u32 {
    if args.capacity_given {
        return args.capacity;
    }
    l1.capacity
        .map_or(args.capacity, |declared| declared.default)
}

/// **What a replay says about a `save` it passed over**, in one place.
///
/// A save reaches a replay two ways — inside a frame, and after the last tick —
/// and each used to spell this sentence for itself. Two literals of one
/// sentence is the shape this codebase keeps removing, and the only test
/// reaching either of them asserted that the output contained "save" and the
/// id, so the two were free to drift apart without anything failing. `ir-spec`
/// requires that a replay say *which* effects outside the stream it skipped;
/// what it does not require is that the answer depend on where in the stream
/// the record sat.
fn skipped_save(slot: DeckSlot, id: &str) -> String {
    format!(
        "  a `save` of slot {slot} was skipped: a replay writes no Set files. \
         The material it named is set `{id}`"
    )
}

/// **What to say about the records after the last tick.**
///
/// Said rather than dropped, on the same terms as everything else here: a
/// session that ended between frames recorded what the operator last did, and
/// nothing renders it because there is no frame it belongs to.
///
/// **A `save` among them is named**, which counting alone did not do. The rule
/// in `docs/ir-spec.md` is that a replay says *which* effects outside the stream
/// it skipped, and this path was obeying half of it: a `save` inside a frame was
/// named and a `save` after the last tick was folded into a number. That is the
/// wrong half to lose, because the last thing an operator does before quitting
/// is press `k` — a save at the end of a set lands here rather than inside a
/// frame, so the same key press was reported two different ways depending on
/// whether another frame followed it.
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

/// **Render a recorded session.** The material comes from the stream's head and
/// every frame advances by the `tick` that was recorded, so nothing here reads
/// a clock — which is the whole claim: a replay and the run it came from are
/// the same sequence of frames.
///
/// Offscreen only. A window would add a clock back at the one place a replay
/// must not have one: `RedrawRequested` arrives when the display says so, and a
/// replay's frames belong to the stream.
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
    // **`--seq` creates its directory, and a replay reaching it did not.**
    // `render::to_sequence` does this and `render::replay` goes straight to the
    // driven loop beside it, so `--replay --seq` into a directory that does not
    // exist failed on the first frame it tried to write — after rendering every
    // frame before it.
    if let Some(dir) = &args.seq_to {
        if let Err(e) = std::fs::create_dir_all(dir) {
            eprintln!("karakuri-cli: {}: {e}", dir.display());
            std::process::exit(1);
        }
    }
    let gpu = Gpu::headless().expect("no GPU");
    // **The stream's, not the flag's** — `--canvas` with `--replay` is refused
    // for this reason. A session written by this program always carries one;
    // the fallback is for a stream that predates the record or was written by
    // hand, and it is named rather than assumed because a replay at the wrong
    // size is a replay of different pixels.
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

    // **The file's own, per geometry, and derived only where it recorded
    // none.** A replay that re-derived them would be a replay of the material
    // in different colours, which is exactly the failure the seed record was
    // added to stop.
    let salts = salts_for(seed_for(0), &loaded.salts, loaded.l1s.len());
    // **Read once, and handed to both the Set and every rebuild the stream
    // asks for**, which is `build_deck`'s rule and `recorded_camera`'s reason.
    let layering = layering_for(args, 0, loaded.layering);
    let live = loaded.live;
    let mut set = build(
        &gpu,
        &loaded.l1s,
        // **The chain the file recorded**, which is the whole of what was
        // played: the deformers in chain order, the camera if it had one, the
        // field if it had one. A head that names none of them replays from the
        // built-in orbit, exactly as every session recorded before a Set file
        // could carry a chain does.
        &loaded.l2s,
        &loaded.l3s,
        &loaded.fields,
        &loaded.l4s,
        // **The head's own layering**, which it records — so a session whose
        // slot 0 was compositing replays compositing, and the `select` records
        // in the stream land on a fold that is there to be selected in. A head
        // that says nothing is a Set that overdrew, which is what saying
        // nothing has always meant; `--merge 0` beside a replay still turns it
        // on, on `layering_for`'s terms, which is what lets a session recorded
        // before the record existed be replayed as it was played.
        layering,
        // **The names and edges the file recorded.** A replay is the material
        // as it was played, and an edge is part of the material: a slot the
        // file bound and a replay did not would be a Set that will not build,
        // and one bound to a different geometry is a different picture.
        &loaded.names,
        &loaded.edges,
        // The file's number per geometry when it recorded one, and otherwise
        // the procedure's own declared default — never the flag's, which is a
        // general default beating a specific declaration that meant it.
        &capacities_for(args, &loaded.l1s, &loaded.capacities),
        &mut vec![false; loaded.bindings.len()],
        &loaded.params,
        &loaded.bindings,
        // **A Set file does not record an interface yet**, on the same terms it
        // records neither a chain nor a camera: it names an L1, its renderers,
        // their values and their bindings. Publishing nothing is
        // publishing everything, so a replay shows the whole console — which is
        // the safe direction, since an interface is about attention.
        &[],
        // The Set's own seed — what an L3 reads — is the first geometry's,
        // which is what one number can hold and what a file recording one seed
        // has always meant by it.
        salts.first().copied().unwrap_or_else(|| seed_for(0)),
        &salts,
        loaded.camera,
        // **The fold the head was left with.** A `select` record later in the
        // stream moves it, exactly as it did live; this is where the run
        // started, and a replay that began with every renderer live would be a
        // replay of a different first frame.
        live,
    );
    set.resize(&gpu.device, w, h);
    // **A deck of one, from one Set file**, which is what a replay can build
    // today: a session stream names the slots its records act on but carries no
    // way to say what was in them, since a Set file describes one Set and there
    // is no record for "this deck held these four". So a record naming slot 1
    // or beyond is reported and skipped — see `apply_replayed` — and a session
    // recorded on a full deck replays the first slot's performance rather than
    // the deck's. That is a hole in the *stream's* vocabulary rather than in
    // this function, and it is the same hole `gain`, `blend`, `residency` and
    // `preview` all fall into.
    let mut deck = Deck::new(&gpu.device, vec![HotSwap::fixed(set)], w, h);
    deck.set_signals(Signals::new(args.bpm, u64::from(SEED)));

    let frames = u32::try_from(stream.frames.len()).unwrap_or(u32::MAX);
    eprintln!(
        "replaying session `{id}`: {frames} frames, {w}x{h}, {} at exposure {:.2} -> {}",
        op_name(args.look.op),
        args.look.exposure,
        out.display()
    );

    // What each slot is playing, as the stream names it. Seeded from nothing:
    // the head already built the Sets a run started with, so the first
    // `procedure` record is the first *change*.
    // The L1, and the renderers by index — a slot draws with one geometry and
    // however many L4s, so the second half is a list rather than a slot.
    let mut playing: Vec<(
        Option<karakuri_store::hash::Hash>,
        Vec<Option<karakuri_store::hash::Hash>>,
    )> = vec![(None, Vec::new()); deck.slot_count()];
    // A `look` record in the stream moves this, so it is **returned** from the
    // driver each frame rather than handed to the renderer once before the run.
    // It used to be both, and the parameter is the one that won: a session where
    // the operator changed the tone mapper or the exposure replayed under
    // whatever it started with, however many `look` records the stream carried.
    // The `--look` and `--exposure` flags are only the value it starts at, on
    // the same terms as a live run.
    let mut look = args.look;
    // **And the master chain, on the same terms and from nothing.** There is no
    // flag for it — a flag writes into a record it does not invent (ADR-0046)
    // and no flag names a slot of this chain — so a replay starts with an empty
    // chain, which draws nothing at all, and the stream's first `master_chain`
    // record is the first *change*. That is exactly what `playing` above is
    // seeded from nothing for.
    //
    // **Handed on only where a record moved it**, which is `render::replay`'s
    // own rule for the third value: putting a chain on the `Present` may mean
    // compiling procedures, so the frames nothing changed hand `None`.
    let mut chain: Vec<karakuri_engine::SlotSpec> = Vec::new();
    let mut chain_moved = false;
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
        // **The shipped three first, then the store**, which is the order
        // `mix::resolve_procedure` states: a chain of presets replays with no
        // store at all, and any other slot's source is one a session's
        // `procedure` records already put there. An address nothing holds is
        // refused **with the address in the message**, which is what a replay
        // meeting a procedure the store does not have is owed (ADR-0340).
        &|address| mix::resolve_procedure(Some(&store), address),
        |i, deck| {
            let frame = &stream.frames[i as usize];
            // **Procedures are gathered and applied together, after the rest.**
            // A slot's procedures arrive as a group and a Set is built from all
            // of them — rebuilding on the first would compile an L1 against the
            // L4 it is replacing, which is a composition the performance never
            // had and may not even check.
            //
            // **A slot's renderer list is replaced, not merged.** A rebuild
            // restates the whole stack, so the records in this frame are the
            // whole stack; keeping what was there would leave a stale renderer
            // behind whenever a slot went from three to one, and nothing in the
            // vocabulary can say "one fewer" on its own.
            let mut changed: Vec<usize> = Vec::new();
            let mut restated: Vec<usize> = Vec::new();
            for record in &frame.before {
                if let karakuri_store::record::Record::Procedure {
                    slot,
                    at,
                    proc_hash,
                } = record
                {
                    let (layer, index) = (&at.layer, &at.index);
                    let slot = slot.index();
                    if slot >= playing.len() {
                        eprintln!(
                            "  a `procedure` for slot {slot} was skipped: this replay \
                             builds a deck of one"
                        );
                        continue;
                    }
                    match layer {
                        Layer::L1 => playing[slot].0 = Some(*proc_hash),
                        // **Indexed, and the list grows to fit.** A stack's
                        // renderers arrive as one `procedure` record each,
                        // numbered in draw order; a stream from before stacks
                        // existed carries index 0 and lands in the same place
                        // the old code put it.
                        Layer::L4 => {
                            // The first L4 record for this slot in this frame
                            // starts the list over — see "replaced, not merged"
                            // above.
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
                            continue;
                        }
                    }
                    if !changed.contains(&slot) {
                        changed.push(slot);
                    }
                    continue;
                }
                // **A replay is a sandbox, and this is the record that makes
                // that a rule rather than a description.** Every other record
                // here describes the deck, and obeying it is what replaying
                // means; a `save` describes a file in a store, and obeying it
                // would mean writing into an id that already exists in a
                // library nobody asked this run to touch — so `--replay` would
                // stop being a function from a stream to some frames.
                //
                // **Said, not silently dropped.** `load_set` prints every note
                // it could not honour and the session writer counts the batches
                // it lost; a replay that skipped an outside effect in silence
                // would be the one place in this program where something
                // happened and nothing said so. The id is named because it is
                // what an operator would go and load by hand.
                if let karakuri_store::record::Record::Save { slot, id } = record {
                    eprintln!("{}", skipped_save(*slot, id));
                    continue;
                }
                apply_replayed(deck, &mut look, &mut chain, &mut chain_moved, record);
            }
            for slot in changed {
                match rebuild(&gpu, &store, &playing[slot], args, slot, layering, live) {
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

/// Build the Set a slot's `procedure` records name.
///
/// **Every node or nothing.** A `procedure` record names one node, and a Set is
/// built from all of them — so until the L1 and every renderer have been seen
/// there is nothing to build, and a slot whose stream only ever names some of
/// them keeps what the head gave it. That is not a corner case: the writer emits
/// the whole stack, but a stream from a future version might name only what
/// changed.
///
/// A **gap** in the renderer list is refused on the same terms rather than
/// closed up: index 2 without index 1 describes a stack with a hole in it, and
/// silently shifting the third renderer into second place would change draw
/// order.
fn rebuild(
    gpu: &Gpu,
    store: &karakuri_store::store::Store,
    playing: &(
        Option<karakuri_store::hash::Hash>,
        Vec<Option<karakuri_store::hash::Hash>>,
    ),
    args: &Args,
    slot: usize,
    // **The layering and the selection the head was built at**, restated here
    // rather than re-derived — `watch::Watch::layering` states the rule and
    // this is the replay's copy of the same hazard: a `procedure` record names
    // a swapped-in procedure and says nothing about how the slot's renderers
    // meet, so a rebuild that worked it out again would put a composited
    // session onto overdraw at the first swap, with a `select` landing on a
    // fold that is no longer there.
    //
    // **A later `select` in the stream moves the selection and this does not
    // know it**, which is the same thing a live rebuild does to a slot the
    // operator pressed `r` on. It is the head's fold, restated; a stream that
    // selected again after the swap selects again on replay.
    layering: karakuri_engine::set::Layering,
    live: Option<u32>,
) -> Result<Set, String> {
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
    // Checked again rather than trusted. The store is content-addressed, so
    // these are the exact bytes that compiled during the performance — but a
    // build this program can refuse is a build it must refuse, and the
    // diagnostics belong on the terminal either way.
    let l1 = compile::check(&source(l1_hash)?).map_err(|report| format!("L1:\n{report}"))?;
    let l4s = l4_hashes
        .iter()
        .flatten()
        .map(|h| compile::check(&source(h)?).map_err(|report| format!("L4:\n{report}")))
        .collect::<Result<Vec<_>, String>>()?;
    Ok(build(
        gpu,
        std::slice::from_ref(&l1),
        // Artifacts recorded by a session, which stores an L1 and its
        // renderers — see the note at the other `build` call site.
        &[],
        &[],
        &[],
        &l4s,
        layering,
        &Names::default(),
        // **No node here declares a slot**, because there is no L2 here at all:
        // a `procedure` record names an L1 and its renderers.
        &[],
        // Nothing recorded: a `procedure` record names a swapped-in procedure
        // and carries no capacity, so each source runs at what it declares.
        &capacities_for(args, std::slice::from_ref(&l1), &[]),
        &mut vec![false; args.bindings.len()],
        &args.overrides,
        &args.bindings,
        &[],
        seed_for(slot),
        // **Nothing recorded, on the same terms as the capacity above.** A
        // `procedure` record names a swapped-in procedure and carries no salt,
        // so the one source is salted from the slot's seed and its ordinal.
        &salts_for(seed_for(slot), &[], 1),
        None,
        live,
    ))
}

/// One record from a session, applied to a replaying deck.
///
/// Deliberately the same decoders the live path uses — `mix::change` and
/// `audio::apply_tempo` — because that is the whole point of the arrangement:
/// what drove the engine live and what drives it on replay are the same
/// function, so they cannot come apart.
///
/// **`look` and `chain` are carried out rather than applied here**, and for one
/// reason each: the look is the present pass's and this function has no
/// `Present`, and the chain is the same one pass earlier. Both are what the
/// driver returns per frame — `render::replay`'s own paragraph is why — so a
/// record that moves either moves it for the frame it lands in and every frame
/// after it, which is what a mid-session change means.
fn apply_replayed(
    deck: &mut Deck,
    look: &mut Look,
    chain: &mut Vec<karakuri_engine::SlotSpec>,
    chain_moved: &mut bool,
    record: &karakuri_store::record::Record,
) {
    // The two the signal bus takes, through the same decoders the live path
    // uses. `audio` is what makes a replay reproduce what the room sounded
    // like: without it a binding to `energy` would replay at the confidence
    // the bus invents rather than at what a microphone heard.
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
            // The same check the live path makes, against the same fact, and
            // this is the path it is actually likely on: a session replayed
            // against material that has since lost a renderer.
            let count = deck.slot(EngineSlot(slot as u8)).set().inputs().len();
            match renderer_in_range(slot, renderer, count) {
                Ok(()) => deck.schedule_selection(karakuri_engine::transition::Selection::new(
                    slot, renderer, start,
                )),
                Err(refusal) => eprintln!("  {refusal} — skipped"),
            }
        }
        // **The knob turn, replayed.** One record is one or three writes, and
        // each of them is reported where it lands on nothing: a session
        // replayed against material that has since lost the parameter is the
        // path this is actually likely on, exactly as it is for the selection
        // above.
        Ok(Some(mix::Change::Ride { slot, writes })) => {
            for write in &writes {
                match deck.write_param(EngineSlot(slot as u8), write) {
                    Ok(0) => eprintln!("  {} — skipped", no_such_param(slot, &write.key)),
                    Ok(_) => {}
                    Err(refused) => eprintln!("  {refused}"),
                }
            }
        }
        // **Governed, exactly as the live path governs.** `set_residency`
        // writes the request *and* grants it, and only a governor pass
        // re-derives what the deck is actually doing against the budget of the
        // machine it is on. A replay used to skip the pass entirely, so every
        // request was granted — which is precisely what `Record::Residency`'s
        // documentation says must not happen, since it records the request and
        // never the effective level for the reason that a session recorded on a
        // fast machine and replayed on a slow one has to re-derive it.
        //
        // Latent until a session can carry a deck: a replay builds one slot, and
        // one slot does not exhaust a budget. Closed anyway, because the reason
        // it was invisible is that the two paths were different code.
        // **An attachment, replayed** — and the take-back with it, because
        // they are one record. A binding is what `Set::prepare` resolves on
        // every frame, so this is on screen at the next one and compiles
        // nothing, exactly as the ride above it does.
        //
        // A name the material has since lost is reported and skipped, on the
        // ride's terms: `Bound` says which half of the attachment was not
        // there, which is the whole content of the message.
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
        // **Who may move one node, replayed.** A node the material has since
        // lost is said and passed over, which is `Request::authorities`' rule
        // at the other end of the same fact.
        Ok(Some(mix::Change::Authority {
            slot,
            layer,
            index,
            authority,
        })) => {
            if !deck.set_authority(EngineSlot(slot as u8), layer, index, authority) {
                eprintln!(
                    "  slot {slot}: no node {}:{index} to make {} — skipped",
                    karakuri_environment::setfile::layer_name(
                        karakuri_environment::setfile::layer_of(layer)
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
        // **The two ends of the master chain**, and the level is the one that
        // reaches the deck: it is applied where the mix *writes* the
        // composited frame. The chain's settings reach the `Present` the
        // driver holds, so they are carried out the way the look is
        // (ADR-0224, ADR-0317).
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

/// Build a scheduled move out of a decoded record and the value the control is
/// at **now**.
///
/// The `from` end is read here rather than carried in the record, which is the
/// whole of why this function exists and is shared by the live path and the
/// replay path: both have to read it at the same point in the stream or a
/// replay would fade from somewhere the run did not. `session::split` puts a
/// key press between two ticks into that frame's `before` list, which is the
/// position it was applied at live, so they do.
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
