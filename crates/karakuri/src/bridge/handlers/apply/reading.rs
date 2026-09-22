use super::*;

/// The reading an operation's record needs, taken off the deck it names.
///
/// [`written`] builds `Record::Mask` whole — a shape, an angle, a position and
/// a softness — out of an operation that names two of the four, and the other
/// two come from a reading of the mask that is running (ADR-0201). This is that
/// reading, and it is the harness's because the deck is (ADR-0156, ADR-0194).
///
/// `Current::default()` is *I read nothing*, and it is still the answer for
/// four of this panel's emitting controls: a gain, an opacity, a blend mode and
/// a residency each carry everything their record carries, so handing a reading
/// in would be this file inventing a value. The mask mini is one of those that
/// need one, and it needs it for the deck the operation *names* rather than for
/// the deck the pointer is over — which is `Reading::Mask`'s own wording and
/// the reason this takes the operation and not a slot.
///
/// The two look controls are two of the other three, and they read one thing
/// between them: the look that is running. Each names a third of `Record::Look`
/// and the other two thirds come from here — which is [`Reading::Look`]'s own
/// wording and the reason the reading is taken for the operation rather than
/// per control.
///
/// The scrub's two arrows are the fourth, and the reading they take is the one
/// thing on this list that is not a completion: see the arm.
///
/// The sync chip and the anchor are the fifth and sixth, and they read the one
/// thing here that belongs to no deck: the session tempo. That arm used to be
/// absent and the two controls used to print a question instead of moving
/// anything — see [`unwritten`] for what the question turned out to be.
///
/// The softness is read back, where `karakuri-cli`'s `mix::current_mask`
/// substitutes its own `MASK_SOFTNESS`: that program writes wipes and has a
/// softness of its own to write, and this window has never written one. What is
/// read back here is therefore what is actually on the slot, and reading it
/// back is what stops a press rewriting it — the same argument the angle's is,
/// one field along.
///
/// The `go` capsule is the last of them and it is the one that reads three: a
/// wipe is written against the transition settings, the mask of the deck
/// arriving and where that deck already sits in the mix. The first is the
/// console's own — `settings` is what [`View::transition`] holds and what the
/// row's three pills move — and the other two are the deck's, taken for the
/// `to` slot and never for the `from`, which is `Current::mask`'s own wording:
/// everything a wipe writes is about the deck arriving.
///
/// A slot the deck has not got answers `None`, and [`written`] then says the
/// reading was owed rather than indexing something that is not there — the
/// guard [`apply`] has, at the other end of the same press.
///
/// The last reading is the session's four banks, and it is the one that is not
/// about a deck at all: which lanes of the armed pattern hold which controls,
/// so that a scheduled move on a fader a lane holds is refused before any
/// record is written (ADR-0323).
pub(crate) fn reading(
    operation: &Operation,
    deck: &Deck,
    look: &Look,
    chain: &[karakuri_engine::SlotSpec],
    settings: TransitionSettings,
    banks: &karakuri_pattern::Banks,
) -> Current {
    // The whole chain, for whichever slot was asked for: `Record::MasterChain`
    // carries the whole list and a press names one slot of it, so the chain
    // that is running is handed in and `written` takes the slots the press did
    // not name — and answers the position that is not in it.
    let master_chain = match *operation {
        Operation::SetChainParam { .. }
        | Operation::AddChainEffect { .. }
        | Operation::RemoveChainEffect { .. } => Some(mix::current_chain(chain)),
        _ => None,
    };
    let look = match *operation {
        // **Two thirds of the record, for whichever third was asked for.**
        // `SetTonemap` carries an operator and `SetExposure` a level, and
        // `Record::Look` needs all three — so the running look is handed in
        // and `written` takes the two the press did not name. That is
        // ADR-0192's argument executable in this file: the operation carries
        // what a surface can say and the translator completes the record.
        // `white_point` is on no surface at all, so it survives every press by
        // arriving here and going straight back out.
        Operation::SetTonemap { .. } | Operation::SetExposure { .. } => {
            Some(karakuri_operation_record::Look {
                tonemap: mix::tonemap(look.op),
                exposure: look.exposure,
                white_point: look.white_point,
            })
        }
        _ => None,
    };
    // **The scrub is the third reading, and it is the only one that is
    // relative.** `Record::Transport` is absolute — a sync mode, an anchor and
    // a scrub position — and `ScrubDeck` names an amount, so the record is
    // where the slot already is plus what was asked for. The reading is
    // therefore not a completion of a record the way the look's and the mask's
    // are: it is the left-hand side of an addition, and without it the
    // conversion answers `Owed(NotRead(Transport))` rather than starting a
    // deck's scrub from zero.
    //
    // **All three fields, because the record is written whole.** A scrub that
    // wrote a position without the mode and the anchor beside it *"would
    // replay a slot onto a grid it was never on"* —
    // `karakuri_operation_record::Transport` says so at its own definition —
    // and the two it does not touch survive the press by arriving here and
    // going straight back out, which is the white point's arrangement one
    // reading up.
    let transport = match *operation {
        Operation::ScrubDeck { deck: slot, .. } => {
            EngineSlot::new(slot, deck.slot_count()).map(|slot| {
                let transport = deck.transport(slot);
                karakuri_operation_record::Transport {
                    sync: mix::sync(transport.sync()),
                    anchor_bpm: transport.anchor_bpm(),
                    scrub_beats: transport.scrub_beats(),
                }
            })
        }
        _ => None,
    };
    // **The tempo the room is going at, and nothing about a slot.** Engaging
    // a sync mode anchors the slot at the session tempo so that the picture
    // does not move at the instant it goes on the grid, which is
    // `karakuri_engine::transport::Transport::engaged`'s policy and the whole
    // of what this reading is for. `mix::current_tempo` takes the oscillator
    // rather than an `f32`, so this window cannot hand in a tempo the session
    // never ran at — and it reads the grid rather than a clock, which is what
    // lets the record be replayed
    // ([P-0092](../../../docs/principles/0092-the-same-inputs-produce-the-same-frame.md)).
    //
    // **No slot check, unlike the three below.** The tempo is the session's,
    // so a `SetSync` naming a slot the deck has not got is a record with a slot
    // out of range rather than a reading that could not be taken, and
    // `apply`'s own guard is what says so at the other end of the press.
    let tempo = match *operation {
        Operation::SetSync { .. } => Some(mix::current_tempo(deck.signals().oscillator())),
        _ => None,
    };
    // **Three operations read this and one of them names two decks.** A wipe
    // writes `Record::Mask` for the deck *arriving* — twice, at the front's
    // present position and then at 0 — so the mask handed over is `to`'s and
    // `from` is read for nothing at all. Handing in the covered deck's would
    // put somebody else's soft edge on the front that is about to cross the
    // frame, which is `Current::mask`'s own sentence and `karakuri-cli`'s
    // `Live::operate` arm exactly.
    //
    // **`SetMaskPosition` was missing from this list until 2026-09-10**, and
    // it is the one arm ADR-0334 recorded as a defect rather than a scope:
    // both halves of a mask write `Record::Mask` whole, so a position needs
    // the shape, the angle and the soft edge it does not name, and without
    // this `written` answered `Owed(NotRead(Mask))` and nothing moved. It was
    // reachable from a mapped controller before it was reachable from a model,
    // so the hole was a MIDI knob that did nothing as well as a call that
    // could not be accepted. `karakuri-cli`'s arm has always named all three,
    // which is what makes this a slip in one file rather than a decision.
    let mask = match *operation {
        Operation::SetMaskShape { deck: slot, .. }
        | Operation::SetMaskPosition { deck: slot, .. }
        | Operation::Wipe { to: slot, .. } => {
            EngineSlot::new(slot, deck.slot_count()).map(|slot| {
                let mask = deck.mask(slot);
                karakuri_operation_record::Mask {
                    kind: wipe_kind(mask.kind()),
                    angle: mask.angle(),
                    position: mask.position(),
                    softness: mask.softness(),
                }
            })
        }
        _ => None,
    };
    // **Every field named, and no `..Current::default()` behind them.** Every
    // reading the type carries is answered here, so a fill would be dead — and
    // the day it grows one more, this stops compiling and somebody has to say
    // whether this window can take it, rather than a `None` arriving silently.
    //
    // **The count that used to be in this sentence is gone rather than
    // corrected.** It said four where there are three and named a fifth that
    // would be a fourth, which is a figure nothing checks going stale in the
    // one comment whose whole argument is that the compiler does the checking.
    //
    // **The transition settings used to be answered `None` here, on the
    // grounds that this panel drew no control that set any of them.** That
    // sentence was true of a window with no transition row in it and is not
    // true of this one: the row is drawn, its three pills emit
    // `Operation::SetTransition`, and `View::transition` is the model of
    // record for what they arrive at. So the reading is taken, and it is the
    // one here that is read off the *console* rather than off the deck —
    // a quantum, a length and a wipe shape are a surface's setting deciding
    // what the next move means, and this surface now holds one.
    //
    // **Four operations, which is every one that schedules a move**, and it
    // is `karakuri-cli`'s `Live::operate` arm exactly: only the wipe has a
    // control on this panel today, and the other three are answered because
    // what the reading *is* does not depend on which surface asked. A
    // conversion that came back `Owed(NotRead(Transition))` for a fade the
    // day a fader learned to schedule one would be this arm having to be
    // found again.
    //
    // `mix::current_transition` takes the oscillator and the quantum rather
    // than an instant, so the start is
    // `karakuri_engine::transition::quantise`'s answer and this file cannot
    // hand in a beat the grid was never on — the arrangement `mix::current_tempo`
    // is in one reading up
    // ([P-0092](../../../docs/principles/0092-the-same-inputs-produce-the-same-frame.md)).
    // `mix::FADE_CURVE` is the shape every scheduled fade takes, and it is
    // asked for by name rather than spelled here because two surfaces easing
    // one fade differently is a value a replay carries.
    let transition = match *operation {
        Operation::FadeDeck { .. }
        | Operation::Crossfade { .. }
        | Operation::SelectRenderer { .. }
        | Operation::Wipe { .. } => Some(mix::current_transition(
            deck.signals().oscillator(),
            settings.quantum,
            settings.length,
            mix::FADE_CURVE,
            mask_kind(settings.kind),
            settings.angle,
        )),
        _ => None,
    };
    // **Where the deck a wipe is arriving on already sits in the mix**, and
    // the one reading here taken so that a record can be left *out* rather
    // than written. A wipe puts that deck under `over` and on air, and both
    // are a state it may be in already: under `add` the same gesture is a wipe
    // *on* rather than a wipe *over*, a different picture and a legitimate
    // one, so the mode is left where the operator put it. The condition is the
    // conversion's and what it needs to hold it is this
    // ([P-0094](../../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)).
    //
    // **What the deck reports rather than what it was asked for**, which is
    // `Deck::residency`'s answer: the governor may hold a slot below the
    // request, and what a wipe needs to know is whether the put-on-air it is
    // about to write would change anything.
    let mix = match *operation {
        Operation::Wipe { to: slot, .. } => EngineSlot::new(slot, deck.slot_count())
            .map(|slot| mix::current_mix(deck.blend(slot), deck.residency(slot))),
        _ => None,
    };
    // Collect held lanes for armed sequencer pattern (ADR-0323).
    let lanes = Some(karakuri_operation_record::Lanes {
        held: banks
            .pattern()
            .held()
            .map(|(at, target)| (at, target.clone()))
            .collect(),
    });
    Current {
        look,
        master_chain,
        mask,
        transport,
        tempo,
        transition,
        mix,
        lanes,
    }
}
