use super::*;

mod feedback;
mod procedure;
mod reading;

pub(crate) use feedback::{refusal, unperformed, unwritten};
pub(crate) use procedure::{
    asked_for, base_material, derived_material, overlaying, put_back, slot_salt,
};
pub(crate) use reading::reading;

/// A press on a strip or a deck key, applied to the console's own pointer, and
/// what to say about it. `None` for every operation that is not it.
///
/// `Operation::SelectDeck` *"writes no record, and is the reason every other
/// variant names its deck instead of meaning the selected one"*, so there is
/// nothing on the deck for [`apply`] to move and the surface that emits it is
/// what performs it (ADR-0198). This is that performance, and it is one line
/// beside [`arrangement`]'s for the same reason: the alternative is a second
/// route into the view.
///
/// A deck the mixer has no strip for is refused, and `View::select` is where
/// that rule lives — the ring would be drawn nowhere and the library's pill
/// would name a deck a load could not reach. It is said here rather than
/// swallowed, because a key that does nothing and a key that is not bound are
/// the same experience.
pub(crate) fn pointed(view: &mut View, operation: &Operation) -> Option<String> {
    let Operation::SelectDeck { deck } = *operation else {
        return None;
    };
    let letter = deck_letter(deck);
    if usize::from(deck) >= view.mixer.len() {
        return Some(format!(
            "  select: deck {letter} refused — this deck has {} slot{}, and a selection with no \
             strip under it is a ring drawn nowhere and a `load` pill naming a deck the press \
             could not reach",
            view.mixer.len(),
            match view.mixer.len() {
                1 => "",
                _ => "s",
            }
        ));
    }
    view.select(deck);
    Some(format!(
        "  select: deck {letter} -> SelectDeck {{ deck: {deck} }} -> no record, and that is \
         settled: it is a surface's own pointer. The keys are addressed here, and the library's \
         foot reads `load -> {letter}`"
    ))
}

/// Applies inspector pane target selection to UI view state (ADR-0338, Principle 0083).
pub(crate) fn pointed_pane(view: &mut View, operation: &Operation) -> Option<String> {
    let Operation::PointPane { pane, deck } = operation else {
        return None;
    };
    let Some(at) = view::PANE_NAMES.iter().position(|name| name == pane) else {
        return Some(format!(
            "  pane: `{pane}` refused — this console's panes are {}",
            view::PANE_NAMES.join(" and ")
        ));
    };
    let letter = deck_letter(*deck);
    if !view.point_pane(at, *deck) && usize::from(*deck) >= view.mixer.len() {
        return Some(format!(
            "  pane: `{pane}` -> deck {letter} refused — this deck has {} slot{}, and a pane \
             pointed at one it has not got is a head naming a deck with nothing under it",
            view.mixer.len(),
            match view.mixer.len() {
                1 => "",
                _ => "s",
            }
        ));
    }
    Some(format!(
        "  pane: `{pane}` -> deck {letter} -> no record, and that is settled: a pane's target is \
         a surface's own pointer. The keys stay where they are and the pane next door does not \
         move"
    ))
}

/// Applies transition configuration updates to the UI view state.
pub(crate) fn scheduled(view: &mut View, operation: &Operation) -> Option<String> {
    let Operation::SetTransition { setting } = operation else {
        return None;
    };
    if !view.set_transition(*setting) {
        let at = view.transition();
        return Some(format!(
            "  transition: {setting:?} refused or already there — the row is on `{}`, `{}`, \
             `{}`, and a pill draws only what its own cycle names",
            at.shape_word(),
            at.quantum_word(),
            at.length_word()
        ));
    }
    let at = view.transition();
    Some(format!(
        "  transition: {setting:?} -> no record, and that is settled: it is a surface's own \
         setting. The next fade, crossfade or wipe is `{}` on the `{}`, over {} beat{}",
        at.shape_word(),
        at.quantum_word(),
        at.length,
        match at.length == 1.0 {
            true => "",
            false => "s",
        }
    ))
}

/// A candidate kept, applied to the lane, and what to say about it. `None` for
/// every operation that is not it.
///
/// [`scheduled`]'s shape one bay over, and for its reason: `written` answers
/// `Silent(Silent::Surface)` for `Operation::KeepCandidate`, so there is no
/// record for [`apply`] to move a deck with and the surface that draws the row
/// is what performs the press. What it changes is one line in one list.
///
/// Nothing else moves, and that is the operation rather than a shortfall. The
/// version is where it was, the store holds every version it held, and the
/// picture is the picture. What a keep says is that a person has looked at this
/// node and is done with it — `console.html`'s *Accepting settles the node and
/// changes nothing on screen*.
///
/// A row that is not there is said rather than swallowed, which is
/// [`pointed`]'s rule: nothing this window emits can reach it — the control is
/// the row and a row that is not drawn takes no press — so a line here is a
/// mapped controller or an MCP call arriving at a node with no candidate on it,
/// the day either reaches this row.
pub(crate) fn kept(view: &mut View, operation: &Operation) -> Option<String> {
    let Operation::KeepCandidate { deck, node } = operation else {
        return None;
    };
    let slot = usize::from(*deck);
    let addr = node_addr(ir_layer(node.layer), node.index);
    let before = view.staging.len();
    view.staging
        .retain(|row| row.deck != slot || row.at != Some(*node));
    let letter = deck_letter(*deck);
    if view.staging.len() == before {
        return Some(format!(
            "  keep: deck {letter} {addr} has no candidate row — nothing was outstanding on \
             that node, and the lane is as it was"
        ));
    }
    Some(format!(
        "  keep: deck {letter} {addr} -> KeepCandidate -> no record, and that is settled: the \
         material already changed and its `procedure` record was written at the swap. The row \
         leaves the lane and nothing else moves; {} still waiting",
        match view.staging.len() {
            0 => "nothing".to_owned(),
            n => format!(
                "{n} row{}",
                match n {
                    1 => "",
                    _ => "s",
                }
            ),
        }
    ))
}

/// The record, applied to the deck, and what to say about it.
///
/// This is not the half ADR-0185 promised to delete, and it did not go with it.
/// Turning an `Operation` into a `Record` was the shortcut — that function is
/// gone and [`written`] answers instead
/// ([ADR-0194](../../../docs/adr/0194-where-an-operation-becomes-a-record-is-a-crate-that-depends-on-both.md)).
/// Turning a record into a *deck movement* is a different job and is the
/// harness's by design: `karakuri-operation-record` has no engine and never
/// will, so somebody who owns a deck has to decode.
///
/// `karakuri-cli`'s `mix::change` is the real decoder and it does two things
/// this does not: it refuses a slot the deck has not got, with the same
/// sentence every other surface refuses one with, and it turns a record into a
/// `Change` that a caller applies. This is the shortest path from the records
/// [`written`] answers with to the setters they name.
///
/// The line it returns is the loop closing, printed so that it can be read
/// rather than inferred: the operation, the record, and what the deck says
/// afterwards — which is where the next frame's strip comes from.
///
/// # It takes the look as well as the deck, and that is not a second target
///
/// `Record::Look` is the one record here that does not name a slot: the look is
/// what *every* sink is drawn under, so it is `Engine::look` rather than
/// anything on the deck ([`Engine::look`], and `karakuri_engine::frame::Look`
/// for why the master out is deliberately not in it). Handing both in is what
/// keeps this one function the only place a record becomes a movement — a
/// second `apply_look` beside it would be the second route into the engine that
/// P-0090 exists to refuse.
pub(crate) fn apply(
    record: &Record,
    deck: &mut Deck,
    look: &mut Look,
    chain: &mut Vec<karakuri_engine::SlotSpec>,
) -> Option<String> {
    // **A slot the deck has not got is refused rather than indexed**, and the
    // guard is [`held`] rather than a closure here, because the key arms in
    // `window_event` need the same answer one step earlier: a press reads the
    // trim it is stepping from before it can name where it is going, so a
    // guard on the record alone would be a read that panicked on its way to a
    // refusal. The argument for refusing at all is at that function.
    match *record {
        Record::Gain { slot, value } => {
            let slot = held(deck, slot.0)?;
            deck.set_gain(slot, value);
            Some(format!(
                "  fader: deck {} trim -> SetGain {{ deck: {slot}, gain: {value:.3} }} \
                 -> Record::Gain -> deck.gain({slot}) = {:.3}",
                deck_letter(slot.0),
                deck.gain(slot)
            ))
        }
        Record::Opacity { slot, value } => {
            let slot = held(deck, slot.0)?;
            deck.set_opacity(slot, value);
            Some(format!(
                "  fader: deck {} fader -> SetOpacity {{ deck: {slot}, opacity: {value:.3} }} \
                 -> Record::Opacity -> deck.opacity({slot}) = {:.3}",
                deck_letter(slot.0),
                deck.opacity(slot)
            ))
        }
        Record::Mute { slot, muted } => {
            let slot = held(deck, slot.0)?;
            deck.set_mute(slot, muted);
            Some(format!(
                "  mute: deck {} -> SetMute {{ deck: {slot}, mute: {muted} }} \
                 -> Record::Mute -> deck.is_muted({slot}) = {}",
                deck_letter(slot.0),
                deck.is_muted(slot)
            ))
        }
        Record::Solo { slot, soloed } => {
            let slot = held(deck, slot.0)?;
            deck.set_solo(slot, soloed);
            Some(format!(
                "  solo: deck {} -> SetSolo {{ deck: {slot}, solo: {soloed} }} \
                 -> Record::Solo -> deck.is_soloed({slot}) = {}",
                deck_letter(slot.0),
                deck.is_soloed(slot)
            ))
        }
        Record::Online { slot, online } => {
            let slot = held(deck, slot.0)?;
            deck.set_online(slot, online);
            Some(format!(
                "  online: deck {} -> SetOnline {{ deck: {slot}, online: {online} }} \
                 -> Record::Online -> deck.is_online({slot}) = {}",
                deck_letter(slot.0),
                deck.is_online(slot)
            ))
        }
        // **The mode comes back off the wire name, and an unknown one is
        // refused rather than defaulted.** `Record::Blend` carries a `String`
        // because what a mode is allowed to be is the engine's to say, so this
        // is the engine saying it — `mix::change` refuses the same way, with
        // the sentence `karakuri-cli`'s `no_such_blend` writes. Nothing in
        // this file can produce a name the engine has not got, since the chip
        // only ever emits one of `BlendMode::ALL`, so this is the guard rather
        // than the message.
        Record::Blend { slot, ref mode } => {
            let slot = held(deck, slot.0)?;
            let blend = Blend::from_name(mode)?;
            deck.set_blend(slot, blend);
            Some(format!(
                "  blend: deck {} -> SetBlendMode {{ deck: {slot}, blend: {mode} }} \
                 -> Record::Blend -> deck.blend({slot}) = {}",
                deck_letter(slot.0),
                deck.blend(slot).name()
            ))
        }
        // Set residency and trigger budget governor check (ADR-0191).
        Record::Residency { slot, ref level } => {
            let slot = held(deck, slot.0)?;
            let residency = mix::parse_residency(level)?;
            deck.set_residency(slot, residency);
            deck.govern();
            Some(format!(
                "  tally: deck {} -> SetResidency {{ deck: {slot}, residency: {level} }} \
                 -> Record::Residency -> deck.requested_residency({slot}) = {:?}, \
                 deck.residency({slot}) = {:?}",
                deck_letter(slot.0),
                deck.requested_residency(slot),
                deck.residency(slot)
            ))
        }
        Record::Policy { slot, ref policy } => {
            let slot = held(deck, slot.0)?;
            Some(format!(
                "  policy: deck {} -> Record::Policy {{ policy: {policy} }}",
                deck_letter(slot.0)
            ))
        }
        // **The whole mask, because the record is a state and not an ask.**
        // `Record::Mask` carries a shape, an angle, a position and a softness,
        // and `Deck::set_mask` is what it decodes to — the engine says so at
        // that setter. Reaching for `Deck::set_mask_shape` instead, to keep a
        // running wipe alive, would be this file decoding a record by picking
        // two fields out of it and dropping the softness on the floor: a
        // second route to the deck, where P-0090 is that every control ends in
        // the same record. **So a shape press stops a wipe on that deck**, and
        // that is not a fault here — it is the honest limit
        // `docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md`
        // states about the record stream, met by the first surface to make the
        // press
        // ([ADR-0203](../../../docs/adr/0203-the-mask-chip-carries-the-angle-it-does-not-control.md)).
        //
        // **The shape comes back off the wire name**, refused rather than
        // defaulted, exactly as the blend's and the residency's do. Nothing in
        // this file can produce a name the engine has not got, since the chip
        // only ever emits one of `WipeKind`'s three and the record is written
        // from `WipeKind::name`.
        Record::Mask {
            slot,
            ref kind,
            angle,
            position,
            softness,
        } => {
            let slot = held(deck, slot.0)?;
            let shape = MaskKind::from_name(kind)?;
            deck.set_mask(slot, Mask::new(shape, angle, position, softness));
            Some(format!(
                "  mask: deck {} -> SetMaskShape {{ deck: {slot}, kind: {kind}, \
                 angle: {angle:.3} }} -> Record::Mask -> deck.mask({slot}) = {} \
                 at {:.3} rad, front at {:.3}",
                deck_letter(slot.0),
                deck.mask(slot).kind().name(),
                deck.mask(slot).angle(),
                deck.mask(slot).position()
            ))
        }
        // **The whole look, because the record is a state and not an ask.**
        // `Record::Look` carries the operator, the exposure and the white
        // point together for its own stated reason — a stream that set a level
        // without naming the operator would describe a look nobody can
        // reconstruct — and each of the two controls asks for one of the three
        // (ADR-0192). The other two arrive here already filled in from the
        // reading [`reading`] took, so this writes what it is given and picks
        // nothing out of it, exactly as the mask arm does.
        //
        // **The operator comes back off the wire name, refused rather than
        // defaulted**, as the blend's, the residency's and the shape's do.
        // Nothing in this file can produce a name the engine has not got: the
        // capsule only ever emits one of `Tonemap`'s four and the record is
        // written from `Tonemap::name`, which is the same lower-case spelling
        // `mix::op_wire_name` parses.
        //
        // **No `set_tonemap` call.** `compose` uploads the tone-map uniform
        // every frame from the `Committed` the closure hands back, so writing
        // the field *is* the write — and a `Present::set_tonemap` here would be
        // a second writer, with the last one each frame winning.
        Record::Look {
            ref op,
            exposure,
            white_point,
        } => {
            let op = mix::parse_op(op)?;
            *look = Look {
                op,
                exposure,
                white_point,
            };
            Some(format!(
                "  look: -> Record::Look {{ op: {op_name}, exposure: {exposure:.3}, \
                 white_point: {white_point:.3} }} -> every sink is drawn under {op_name} at \
                 exposure {exposure:.3}",
                op_name = mix::op_wire_name(op)
            ))
        }
        // **A scheduled move, and the first record this window applies that
        // does not land now.** `Record::Transition` carries the slot, which
        // control is moving, where it ends up, the beat it starts on, how long
        // it lasts and the shape it eases with — and the one thing it does not
        // carry is where the move starts *from*, because that is where the
        // control already is at the moment the record is applied. Reading it
        // here rather than off the record is what makes a replay fade from
        // where the run did, and it is `karakuri-cli`'s `schedule_from`, in
        // the one place this window needs it.
        //
        // **It landed with the `go` capsule, and until then the window drew a
        // wipe's other records and dropped this one on the floor**: the
        // arriving deck was masked to nothing and put on air, and the front
        // never travelled. A record with no arm here is silent — the `_` at
        // the foot of this match — which is why the gap was invisible.
        //
        // **The two names come back off the wire, refused rather than
        // defaulted**, as the blend's, the residency's and the sync mode's do:
        // a control this engine has not got and a curve it cannot ease with
        // are both a record from a stream this build does not understand, and
        // guessing at either would schedule a move nobody wrote.
        Record::Transition {
            slot,
            ref control,
            to,
            start,
            beats,
            ref curve,
        } => {
            let slot = held(deck, slot.0)?;
            let control = Control::from_name(control)?;
            let curve = karakuri_engine::binding::Curve::parse(curve)?;
            let from = match control {
                Control::Gain => deck.gain(slot),
                Control::Opacity => deck.opacity(slot),
                Control::MaskPosition => deck.mask(slot).position(),
            };
            deck.schedule(karakuri_engine::Transition::new(
                slot.index(),
                control,
                from,
                to,
                start,
                beats,
                curve,
            ));
            Some(format!(
                "  transition: deck {} {} {from:.3} -> {to:.3} -> Record::Transition {{ \
                 start: {start:.3}, beats: {beats:.3}, curve: {} }} -> \
                 deck.transitions_on({slot}) = {}",
                deck_letter(slot.0),
                control.name(),
                curve.name(),
                deck.transitions_on(slot).count()
            ))
        }
        // **One level on the whole fold, and the one arm here that names no
        // slot at all.** `Record::MasterOut` carries a number and nothing
        // else, so unlike the look and the mask there is no other half of it
        // to fill in from what is running — which is why the reading below
        // has no arm for this control (ADR-0224).
        //
        // **The engine clamps and this does not.** `Deck::set_out` floors at
        // zero and is deliberately open above 1.0, through the same
        // `clamp_gain` the per-slot gain goes through, because the mix is HDR
        // and this level is applied to values a tone mapper has not seen —
        // [P-0064](../../../docs/principles/0064-the-pipeline-is-linear-hdr-and-srgb-is-encoded-once-at-final-output.md).
        // A clamp here would be a second opinion about a range the setter
        // already holds, which is the rule the whole conversion is written
        // under.
        //
        // **No `cancel` to worry about**, where the gain and the fader each
        // stop whatever was moving them: a `Control` is per slot and this is
        // not, so nothing in the engine can be moving it and there is nothing
        // for a hand to win against.
        // **The one record that reaches inside a Set**, and the row the
        // Inspector bay was blocked on. `Deck::write_param` is the public road
        // and it compiles nothing — the map is packed into the uniform by the
        // next `Set::prepare`, so the value is on screen on the next frame.
        //
        // **Decoded by `mix::change` rather than here**, which is the one arm
        // in this function that does not take the short path, and the reason is
        // the expansion: a `vec3` value is three writes under the component
        // keys ADR-0268 made, and spelling that a second time in this file is
        // the drift `karakuri-operation-record` exists to end. It is also what
        // refuses a slot this deck has not got, in the sentence every other
        // surface refuses one with — so `held` is not asked first here.
        Record::Ride { .. } => {
            let writes = match karakuri_environment::mix::change(record, deck.slot_count()) {
                Ok(Some(karakuri_environment::mix::Change::Ride { slot, writes })) => {
                    let mut reached = 0;
                    for write in &writes {
                        match deck.write_param(EngineSlot(slot as u8), write) {
                            Ok(n) => reached += n,
                            Err(refused) => return Some(format!("  {refused}")),
                        }
                    }
                    (slot, writes, reached)
                }
                Ok(_) => return None,
                Err(refused) => return Some(format!("  {refused}")),
            };
            let (slot, writes, reached) = writes;
            match reached {
                0 => Some(format!(
                    "  {}",
                    karakuri_environment::no_such_param(slot, &writes[0].key)
                )),
                _ => Some(format!(
                    "  knob: deck {} -> WriteParam -> Record::Ride -> {} on {reached} node(s)",
                    deck_letter(slot as u8),
                    writes
                        .iter()
                        .map(|w| format!("{} = {:.3}", w.key, w.value))
                        .collect::<Vec<_>>()
                        .join(", ")
                )),
            }
        }
        // **What is driving a knob, and the taking of it back** — the other
        // record that reaches inside a Set, and the one the Inspector's
        // sensitivity row was blocked on. `Deck::bind` and `Deck::unbind` are
        // the public roads and neither compiles anything: a binding is
        // resolved by the next `Set::prepare`, so an attachment is riding on
        // the next frame.
        //
        // **Decoded by `mix::change` rather than here**, which is the ride's
        // reason one arm up: the three diagnostics a binding owes — a `bpm`
        // source, an `octaves` without `fbm`, a generator on a binding that is
        // not to `noise` — belong to `setfile::binding_from_record` and are
        // said in its words on every route, so spelling them a second time
        // here is the drift `karakuri-operation-record` exists to end.
        Record::Source { .. } => {
            let (slot, key, bound) = match karakuri_environment::mix::change(
                record,
                deck.slot_count(),
            ) {
                Ok(Some(karakuri_environment::mix::Change::Source {
                    slot,
                    layer,
                    index,
                    key,
                    binding,
                })) => match binding {
                    Some(binding) => {
                        let signal = binding.signal.clone();
                        let curve = binding.curve.name();
                        let range = binding.range;
                        match deck.bind(EngineSlot(slot as u8), binding) {
                                karakuri_engine::set::Bound::Yes => (
                                    slot,
                                    key,
                                    format!(
                                        "{signal} through {curve} onto [{:.2}, {:.2}]",
                                        range[0], range[1]
                                    ),
                                ),
                                karakuri_engine::set::Bound::NoSuchParam => {
                                    return Some(format!(
                                        "  {}",
                                        karakuri_environment::no_such_param(slot, &key)
                                    ))
                                }
                                karakuri_engine::set::Bound::NoSuchControl => {
                                    return Some(format!(
                                        "  slot {slot}: `{signal}` is not a control this Set                                          publishes"
                                    ))
                                }
                            }
                    }
                    None => match deck.unbind(EngineSlot(slot as u8), layer, index, &key) {
                        true => (slot, key, "nothing — taken back".to_string()),
                        false => {
                            return Some(format!("  slot {slot}: nothing was driving `{key}`"))
                        }
                    },
                },
                Ok(_) => return None,
                Err(refused) => return Some(format!("  {refused}")),
            };
            Some(format!(
                "  source: deck {} -> Record::Source -> `{key}` is driven by {bound}",
                deck_letter(slot as u8)
            ))
        }
        // **Who may move one node**, and the writer ADR-0211 said the engine
        // owed. It is not enforcement: nothing writes a parameter on an
        // agent's behalf here, so what a level reaches today is
        // `Set::write_param`'s wildcard refusal — a bare-name control over
        // nodes that no longer agree is refused whole from the next press.
        Record::Authority { .. } => {
            let (slot, level) = match karakuri_environment::mix::change(record, deck.slot_count()) {
                Ok(Some(karakuri_environment::mix::Change::Authority {
                    slot,
                    layer,
                    index,
                    authority,
                })) => match deck.set_authority(EngineSlot(slot as u8), layer, index, authority) {
                    true => (slot, authority),
                    false => {
                        return Some(format!(
                            "  slot {slot}: no node {}:{index} to speak for",
                            karakuri_environment::meta::layer_name(
                                karakuri_environment::meta::layer_of(layer),
                            ),
                        ))
                    }
                },
                Ok(_) => return None,
                Err(refused) => return Some(format!("  {refused}")),
            };
            Some(format!(
                "  authority: deck {} -> SetAuthority -> Record::Authority -> {}",
                deck_letter(slot as u8),
                level.name()
            ))
        }
        Record::MasterOut { value } => {
            deck.set_out(value);
            Some(format!(
                "  master: out -> SetMasterOut {{ out: {value:.3} }} -> Record::MasterOut -> \
                 deck.out() = {:.3}, at the entry to the master chain",
                deck.out()
            ))
        }
        // **The chain itself, one record for all three passes.** Written whole
        // for `Record::Look`'s reason and applied whole: the value lands on
        // [`Engine::chain`] and the frame loop hands it to
        // `Present::set_chain`, so nothing here touches a uniform. That is the
        // look arm's arrangement one pass upstream, and it is why there is no
        // `set_chain` call in this function.
        //
        // **The cut comes back off the wire word, refused rather than
        // defaulted**, as the tone map operator and the blend mode do: a cut
        // this engine has not got is a stream saying something this build
        // cannot draw, and a default would silently play the other picture.
        //
        // **No clamp here either.** `Chain::clamped` is the wall and it is
        // inside the setter, so a stream carrying 4.0 meets the same ceiling
        // a fader does.
        Record::MasterChain(ref want) => {
            let mut slots = Vec::with_capacity(want.slots.len());
            for slot in &want.slots {
                slots.push(karakuri_engine::SlotSpec {
                    procedure: slot.procedure.clone(),
                    cut: match &slot.cut {
                        None => None,
                        Some(word) => Some(Cut::parse(word)?),
                    },
                    params: slot.params.clone(),
                });
            }
            let said = format!(
                "  master: chain -> Record::MasterChain -> {} slot{} — {}",
                slots.len(),
                if slots.len() == 1 { "" } else { "s" },
                if slots.is_empty() {
                    "the frame is the mix".to_string()
                } else {
                    slots
                        .iter()
                        .map(|s| {
                            let cut = s
                                .cut
                                .map(|c| format!(" ({})", c.name()))
                                .unwrap_or_default();
                            format!("{}{cut}", &s.procedure[..s.procedure.len().min(14)])
                        })
                        .collect::<Vec<_>>()
                        .join(", ")
                }
            );
            *chain = slots;
            Some(said)
        }
        // **The whole of one slot's clock, because the record is a state and
        // not an ask.** `Record::Transport` carries the sync mode, the anchor
        // and the scrub together for a stated reason — a scrub position
        // without the mode and the anchor beside it *"would replay a slot onto
        // a grid it was never on"* — and `Deck::set_transport` is what it
        // decodes to. The mode and the anchor arrive here unchanged, from the
        // reading [`reading`] took a moment earlier; only the scrub has moved.
        //
        // **The mode comes back off the wire name, refused rather than
        // defaulted**, as the blend's, the residency's, the shape's and the
        // operator's do. Nothing in this file can produce a name the engine
        // has not got: the scrub only ever writes back the mode it just read
        // off the same deck, and the record is written from `Sync::name`.
        //
        // **`set_transport` refuses, and the refusal is dropped here on
        // purpose.** It refuses a mode this slot's material cannot honour, and
        // a scrub cannot present one — it names the mode the slot is already
        // in, which the slot is in because the engine allowed it. What it
        // guards against is the case the engine names at `sync_allowed`: a
        // swap that puts accumulating material into a slot that is beat-synced
        // makes the mode it is *already in* unavailable, *"and whatever wires
        // swapping to this owes it"*. **Both slots are watched now, so that
        // case is reachable**: save an accumulating procedure into a
        // beat-synced slot and the next scrub is refused. What should be
        // printed then is the deck's own refusal rather than this arm's line,
        // and it is what this owes.
        Record::Transport {
            slot,
            ref sync,
            anchor_bpm,
            scrub_beats,
        } => {
            let slot = held(deck, slot.0)?;
            let mode = EngineSync::from_name(sync)?;
            deck.set_transport(slot, mode, anchor_bpm, scrub_beats)
                .ok()?;
            Some(format!(
                "  scrub: deck {} -> ScrubDeck {{ deck: {slot} }} -> Record::Transport {{ \
                 sync: {sync}, anchor_bpm: {anchor_bpm:.1}, scrub_beats: {scrub_beats:+.2} }} \
                 -> deck.transport({slot}).scrub_beats() = {:+.2} beats",
                deck_letter(slot.0),
                deck.transport(slot).scrub_beats()
            ))
        }
        // **A choice and not a position, so nothing interpolates and nothing
        // is left running.** `Record::Select` carries the slot, the renderer
        // and the instant alone — *"half way to renderer 2" does not name a
        // picture* — and `Deck::schedule_selection` is what it decodes to: a
        // later selection on a slot replaces the earlier one, which is
        // `Deck::schedule`'s own rule for a fade.
        //
        // **The renderer is not checked here and is checked at the beat.** The
        // Set in a slot can change under a hot swap between the schedule and
        // the instant, so a selection that no longer names a renderer is
        // dropped where it is applied. The *slot* is checked, by `held`,
        // because a deck does not change size.
        //
        // **It says nothing about a slot that overdraws**, and that is carried
        // rather than refused: `Record::Select` names an edge into the Set's
        // L5 and a slot built without a composite fold has none, so refusing
        // it would make a replay fail on a line describing a performance that
        // happened.
        Record::Select {
            slot,
            renderer,
            start,
        } => {
            let slot = held(deck, slot.0)?;
            deck.schedule_selection(Selection::new(slot.index(), renderer as usize, start));
            Some(format!(
                "  renderer: deck {} -> SelectRenderer {{ renderer: {renderer} }} -> \
                 Record::Select {{ start: {start:.3} }} -> deck.selections_on({slot}) = {} \
                 armed, landing on the beat grid",
                deck_letter(slot.0),
                deck.selections_on(slot).count()
            ))
        }
        // Apply tempo correction to deck signals bus (ADR-0278).
        Record::Tempo { bpm, shift, .. } => {
            let mut signals = *deck.signals();
            karakuri_environment::audio::apply_tempo(&mut signals, record);
            deck.set_signals(signals);
            Some(format!(
                "  tempo: -> Record::Tempo {{ bpm: {bpm:.1}, shift: {shift:+.3} }} -> \
                 deck.signals().oscillator().bpm() = {:.1}, from now on and without moving a \
                 beat that has already happened",
                deck.signals().oscillator().bpm()
            ))
        }
        _ => None,
    }
}
