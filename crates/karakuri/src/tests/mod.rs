use super::*;

pub(crate) use self as tests;
pub(crate) use crate::session::Sessions;

use std::time::{Duration, Instant};

use karakuri_console::focus::Step;
use karakuri_console::input::Claim;
use karakuri_console::panel::Panel;
use karakuri_console::view::{
    look as look_row, mixer as mixer_bay, picture_rect, preview_rects, tracker_group, Picture,
    Reading, RowKind, Scope, Tracker, TransitionSettings, View, DECKS, SCRUB_BEATS, SYNCS,
};
use karakuri_console::{egui, egui_wgpu};
use karakuri_engine::set::Layering;
use karakuri_engine::transport::Sync as EngineSync;
use karakuri_engine::{
    compose, Blend, Committed, Cut, Deck, DeckSlot as EngineSlot, Event, Gpu, Look, Mask, MaskKind,
    Residency, Sink, Skip, TonemapOp,
};
use karakuri_environment::{audio, mix, setfile, watch, Asked, Opening};
use karakuri_layout::{Layout, Point};
use karakuri_operation::{BeatSource, BlendMode, GridScale, Operation, SetTransfer, Undecided};
use karakuri_operation_record::{not_performed, written, Current, Owed, Silent, Written};
use karakuri_store::record::{DeckSlot, Record};
use karakuri_store::store::Store;
use winit::event::WindowEvent;

mod focus_keys;
mod gpu;
mod inspector_mcp;
mod mixer_solo_mute;
mod outputs_row;
mod press_handler;

mod keeps;

mod history;
pub(crate) use self::history::scratch_dir;

mod arrangement;

mod frames;
pub(crate) use self::frames::drawn_once;

mod view_interaction;

/// A control's operation becomes the record every other surface's control ends
/// in, and this is the half of that which needs no device.
///
/// This test is older than the conversion it now checks, and that is the point
/// of it. It was written against the hand-written `record` this file used to
/// carry, asserting term for term what `karakuri-cli`'s `mix::gain_record` and
/// `mix::opacity_record` already wrote. That function is deleted and
/// [`written`] answers instead (ADR-0185's promise, kept where ADR-0194 put the
/// home) — every expectation below is unchanged, so if the crate's conversion
/// disagreed with the one that was deleted, this is what says so.
///
/// `mix::gain_record` is deleted too, by the same record and for the stronger
/// reason: the conversion *is* the derivation now, and two of them is the drift
/// `mix.rs` exists to end. The comments below name it where it stood, because
/// what this test compares against is the record that function wrote rather
/// than the function.
///
/// And the other direction: an operation this program has no control for writes
/// no record here either, and the answer says *which* kind of nothing rather
/// than a bare `None` — which is the whole of what the three answers buy.
#[test]
fn a_controls_operation_becomes_the_record_the_cli_would_have_written() {
    assert_eq!(
        only_record(&Operation::SetGain {
            deck: 2,
            gain: 0.75
        }),
        // What `mix::gain_record(2, 0.75)` wrote, before ADR-0194 deleted
        // it in favour of this conversion.
        Record::Gain {
            slot: DeckSlot(2),
            value: 0.75
        }
    );
    assert_eq!(
        only_record(&Operation::SetOpacity {
            deck: 0,
            opacity: 0.25
        }),
        // `mix::opacity_record(0, 0.25)`.
        Record::Opacity {
            slot: DeckSlot(0),
            value: 0.25
        }
    );
    // **Every mode of the cycle, because a chip that emits three
    // operations has three records to write** — and the mode is a wire
    // name, so a mode that reached `Record::Blend` misspelled would be
    // refused by the engine on the way back rather than here.
    for (deck, blend) in BlendMode::ALL.into_iter().enumerate() {
        let deck = deck as u8;
        assert_eq!(
            only_record(&Operation::SetBlendMode { deck, blend }),
            // `mix::blend_record(deck, blend)`.
            Record::Blend {
                slot: DeckSlot(deck),
                mode: blend.name().to_owned(),
            },
            "`{}` did not become the record `mix::blend_record` writes",
            blend.name()
        );
        // And the engine reads its own name back, which is what says the
        // two lists are the same three words rather than two spellings of
        // them.
        assert_eq!(
            Blend::from_name(blend.name()),
            Some(blend_mode_back(blend)),
            "the engine does not know the vocabulary's `{}`",
            blend.name()
        );
    }

    // **Every residency of the cycle**, for the same reason as the blend:
    // one chip emitting three operations has three records to write. The
    // spelling is the wire's — `mix::residency_wire_name`'s three words,
    // which are deliberately not the status line's `LIVE`/`prim`/`park`
    // and not the chip's `live`/`prim`/`alloc` either, so a record written
    // in the chip's vocabulary would decode as nothing at all.
    for (deck, (residency, level)) in [
        (karakuri_operation::Residency::Live, "live"),
        (karakuri_operation::Residency::Priming, "priming"),
        (karakuri_operation::Residency::Allocated, "allocated"),
    ]
    .into_iter()
    .enumerate()
    {
        let deck = deck as u8;
        assert_eq!(
            only_record(&Operation::SetResidency { deck, residency }),
            // `mix::residency_record(deck, residency)`.
            Record::Residency {
                slot: DeckSlot(deck),
                level: level.to_owned(),
            },
            "{residency:?} did not become the record `mix::residency_record` writes"
        );
        // And it reads back as the level it named, which is what says the
        // two spellings are one list rather than two.
        assert_eq!(
            mix::parse_residency(level),
            Some(residency_back(residency)),
            "the wire spelling `{level}` does not come back as {residency:?}"
        );
    }

    // The vocabulary is larger than what this program reaches: five controls
    // writing five records. A record invented for the other 45 would be
    // somebody deciding what they mean — and the answer is now *which*
    // nothing rather than `None`, because a surface's own state and a
    // record nobody can write yet are not the same silence.
    assert_eq!(
        written(&Operation::Solo { region: None }, &Current::default()),
        Written::Silent(Silent::Surface)
    );
    assert_eq!(
        written(&Operation::SelectDeck { deck: 1 }, &Current::default()),
        Written::Silent(Silent::Surface)
    );
}

/// The one record an operation writes, for the tests that know there is exactly
/// one.
///
/// For the four whose record needs no reading at all, which is where
/// `Current::default()` — *I read nothing* — is the honest answer. A conversion
/// that answered anything but a single record for one of those four is this
/// file's assumption breaking rather than a test needing a helper, which is why
/// the panic says so.
///
/// The mask's operation is not one of them and must not be passed here: its
/// record is written out of the operation *and* a reading of the running mask
/// (ADR-0201), so it would come back `Owed(NotRead)` and this would panic —
/// correctly, and saying which operation. What the mask's tests hand in is a
/// reading, through [`reading`] where there is a deck and by hand where there
/// is not.
pub(super) fn only_record(operation: &Operation) -> Record {
    match written(operation, &Current::default()) {
        Written::Records(records) if records.len() == 1 => records.into_iter().next().unwrap(),
        other => panic!(
            "a control's operation did not write exactly one record: \
             {operation:?} -> {other:?}"
        ),
    }
}

/// A refused wipe says which refusal it was and where the next attempt is made,
/// rather than *refused*.
///
/// `karakuri_console::view::Go` answers which of the two it is because the
/// console is what can see it; the sentence is this window's, and
/// [P-0083](../../../docs/principles/0083-a-refusal-carries-what-the-next-attempt-needs.md)
/// is what it owes: the constraint and the numbers, never a bare no. A press on
/// `go` that printed nothing would read exactly like a press on the card beside
/// it, which is the failure the whole `Go` type exists to prevent.
///
/// The two are asserted to be different sentences, for
/// [`an_operation_whose_record_is_owed_is_said_rather_than_swallowed`]'s reason
/// one test up: a window that printed one line for both would tell an operator
/// with four decks and no shape that they need a second deck.
///
/// Not word for word. What has to hold is that each names what would have to
/// change — the shape pill for one, a second deck for the other — and that the
/// count is in the one whose count is the constraint.
#[test]
fn a_refused_wipe_says_which_refusal_it_was_and_where_to_go_next() {
    let no_shape = refusal(&Go::NoShape, 4);
    assert!(
        no_shape.contains("shape"),
        "the refusal for an unchosen shape does not say what is missing: `{no_shape}`"
    );
    assert!(
        no_shape.contains("pill"),
        "the refusal for an unchosen shape does not say where one is picked, so an \
         operator is told no and not told where to go: `{no_shape}`"
    );

    let alone = refusal(&Go::NoOtherDeck, 1);
    assert!(
        alone.contains('1') && alone.contains("strip"),
        "the refusal for a mixer with nowhere to wipe from does not carry the count \
         that is the constraint: `{alone}`"
    );
    assert!(
        alone.contains("deck"),
        "the refusal for a mixer with nowhere to wipe from does not say what would \
         have to change: `{alone}`"
    );
    // The plural moves with the count, which is this file's rule for every
    // sentence that carries one.
    assert!(refusal(&Go::NoOtherDeck, 0).contains("0 strips"));
    assert!(refusal(&Go::NoOtherDeck, 1).contains("1 strip,"));

    assert_ne!(
        no_shape, alone,
        "a wipe with no shape chosen and a wipe with nowhere to come from came out of \
         this window as the same sentence"
    );
}

/// An operation whose record nobody can write yet does not silently do nothing,
/// and it is not the same event as one that writes no record on purpose.
///
/// This is what the third answer is *for*, and the cheap harness is the one
/// that treats *not `Records`* as a no-op. A press that emitted `TapBeat` would
/// then look exactly like a press that emitted `SelectDeck` — nothing printed
/// and nothing moved — and an operator would read the first as *the tap did not
/// take* when what happened is *nobody has decided what a tap writes* (`Owed`
/// is a question, not an error: ADR-0194).
///
/// The operation this names has had to change twice, which is the test doing
/// what it says on the line below. It was `FadeDeck`, which stopped being owed
/// the day the transition settings became a reading; it was then `Wipe`, which
/// stopped the day the front shape went over with them and the soft edge turned
/// out to be the arriving deck's. It is now `Operation::TapBeat` — and that one
/// is a different shape rather than the next in a queue: what a tap owes is the
/// beat lock's answer and not a value any surface holds, so no reading added to
/// `Current` closes it.
///
/// The second half has been re-pointed once, and the reason is worth reading.
/// It was `FadeDeck`, on the grounds that this panel held no transition
/// settings to hand over — and the day the transition row was wired into this
/// window that stopped being true, without this test going red: it builds a
/// `Current::default()` by hand, so it went on passing while its own sentence
/// had become false. That is the failure mode `docs/contributing.md` §3 is
/// about, met from the wrong side.
///
/// It is `Operation::SetMaskPosition` against a reading nobody took, and the
/// second half stopped being about this window on 2026-09-10. Its record is
/// `Record::Mask` written whole and it needs the shape, the angle and the
/// softness it does not name (ADR-0201). Until that day [`reading`]'s mask arm
/// answered for `SetMaskShape` and for a wipe's arriving deck and for nothing
/// else, so this *was* a gap in this file — which is what ADR-0334 recorded and
/// ADR-0341 closed with one arm.
///
/// What it asserts now is the third answer itself, which is why the operation
/// did not have to change a third time: handed a `Current` with no mask in it —
/// a reading that was not taken, whatever the reason — the conversion says
/// *which* reading is missing rather than sending a front back to wherever a
/// default put it, mid-wipe. That the real reading is now taken is asserted
/// where there *is* a deck,
/// `gpu::the_go_pill_runs_a_wipe_against_the_settings_the_row_is_on`, which is
/// the half a test with no device cannot make.
///
/// Neither sentence is asserted word for word. What has to hold is that the
/// window says something, that it names the operation and the reason, and that
/// the two answers are two different sentences.
#[test]
fn an_operation_whose_record_is_owed_is_said_rather_than_swallowed() {
    // Owed, and `NotSettled` is the reason: a tap's record is the beat
    // lock's answer — a tapped tempo, a phase error, an output lag — and
    // none of it is a value a `Current` carries, so nobody has said what
    // it writes here.
    let tap = Operation::TapBeat;
    let owed = written(&tap, &Current::default());
    assert_eq!(
        owed,
        Written::Owed(Owed::NotSettled),
        "a tap is not owed any more — this test names the operation it does, and \
         the one it names has to still be one nobody can write"
    );
    let said = unwritten(&tap, &owed).expect(
        "a tap owes a record and this window said nothing at all — a press whose \
         record nobody has decided how to write reads, in silence, exactly like a \
         press that did not work",
    );
    assert!(
        said.contains("TapBeat") && said.contains(Owed::NotSettled.why()),
        "the window said `{said}`, which does not name both the operation and the \
         question it is waiting on"
    );

    // **And the other answer, which is a reading nobody took rather than a
    // record nobody has decided.** A mask position converts, and what it
    // needs is the rest of the mask — the shape, the angle and the soft
    // edge `Record::Mask` is written whole out of. Handed a reading with
    // no mask in it, the conversion says *which reading* was not handed
    // over rather than sending a front back to wherever a default put it,
    // and the sentence has to be a different one from the tap's above or
    // the two answers read alike. **This window took no mask for a
    // position until 2026-09-10** and that was the gap this half named;
    // it takes one now (ADR-0341), so what is left here is the third
    // answer itself, asserted against a `Current` built by hand.
    let front = Operation::SetMaskPosition {
        deck: 1,
        position: 0.5,
    };
    let unread = written(&front, &Current::default());
    assert_eq!(
        unread,
        Written::Owed(karakuri_operation_record::Owed::NotRead(
            karakuri_operation_record::Reading::Mask
        )),
        "a mask position with no mask handed in came back with something \
         other than the reading it is missing — a default here is a shape and an \
         angle nobody chose written over the ones a deck is wearing"
    );
    let told = unwritten(&front, &unread).expect(
        "a mask position this window cannot write said nothing at all, so a control \
         that emitted one would read exactly like a control that did not work",
    );
    assert!(
        told.contains("SetMaskPosition")
            && told.contains(Owed::NotRead(karakuri_operation_record::Reading::Mask).why()),
        "the window said `{told}`, which does not name both the operation and the \
         reading it did not get"
    );
    // **And the one it replaced is not owed any more**, which is the half
    // that would have caught this test going quietly stale: a fade is
    // scheduled against settings this console holds now, so `FadeDeck` is
    // no longer a case of *a reading this window does not have*. If this
    // ever fails, the second half above has a candidate again and somebody
    // has to say which of the two this test is about.
    assert_ne!(
        written(
            &Operation::FadeDeck { deck: 1, to: 0.0 },
            &Current {
                transition: Some(karakuri_operation_record::Transition {
                    start: 0.0,
                    beats: 4.0,
                    curve: karakuri_operation::Curve::Smooth,
                    wipe_kind: karakuri_operation::WipeKind::None,
                    wipe_angle: 0.0,
                }),
                ..Current::default()
            }
        ),
        Written::Owed(Owed::NotRead(
            karakuri_operation_record::Reading::Transition
        )),
        "a fade handed the transition settings this window now holds is still owed \
         them, so the reading this panel supplies is not the one the conversion wants"
    );
    assert_ne!(
        told, said,
        "a reading this window forgot and a record nobody has decided how to write \
         read as the same sentence"
    );

    // Silent, and settled: which deck the keys are addressed to is a
    // surface's own state and there is nothing to write.
    let select = Operation::SelectDeck { deck: 1 };
    let silent = written(&select, &Current::default());
    assert_eq!(silent, Written::Silent(Silent::Surface));
    let settled = unwritten(&select, &silent).expect(
        "selecting a deck writes no record and the window said nothing about it \
         either, so a press on such a control would leave no trace at all",
    );
    assert!(
        settled.contains(Silent::Surface.why()),
        "the window said `{settled}`, which does not say why there is no record"
    );

    // **And the two are different sentences.** Collapsing them is the
    // failure this whole test is about at one remove: a harness that
    // printed one line for both would tell an operator that an undecided
    // fade is as settled as a deck selection.
    assert_ne!(
        said, settled,
        "a record nobody can write yet and a record nobody needs to write came out \
         of this window as the same sentence"
    );

    // A record's line is `apply`'s — it says the record *and* what the
    // deck holds afterwards — so this says nothing about that case.
    // Otherwise one press prints twice.
    let gain = Operation::SetGain { deck: 0, gain: 0.5 };
    assert_eq!(
        unwritten(&gain, &written(&gain, &Current::default())),
        None,
        "an operation that wrote a record was also announced as writing none"
    );
}

/// A scheduled move on a fader a lane of the armed pattern holds is refused,
/// says so in the one sentence, and that sentence is not the gap's
/// (ADR-0323).
///
/// Three halves, and the third is the one that could go quietly stale. The
/// first is the answer: a fade over a held fader comes back `Refused` and
/// carries no record, which is the clause a replay depends on — a record
/// written live would be replayed by a run with no sequencer in it (ADR-0322,
/// P-0092). The second is this window's line for it, which has to be a
/// different sentence from an `Owed`: a gap nobody has closed and a decision
/// taken read alike otherwise.
///
/// The third is the reading. [`reading`] cannot be called here — it takes a
/// `Deck` and this binary has no device — so the source is scanned for the
/// banks arriving and for the field being filled from them. Without that, this
/// whole test passes against a `Current` built by hand while the window
/// schedules moves over lanes in silence, which is the failure
/// `docs/contributing.md` §3 is about.
#[test]
fn a_move_on_a_fader_a_lane_holds_is_refused_and_said_as_a_decision() {
    let fade = Operation::FadeDeck { deck: 1, to: 0.0 };
    let current = Current {
        transition: Some(karakuri_operation_record::Transition {
            start: 8.0,
            beats: 4.0,
            curve: karakuri_operation::Curve::Smooth,
            wipe_kind: karakuri_operation::WipeKind::None,
            wipe_angle: 0.0,
        }),
        lanes: Some(karakuri_operation_record::Lanes {
            held: vec![(3, karakuri_operation::LaneTarget::Fader { deck: 1 })],
        }),
        ..Current::default()
    };
    let refused = written(&fade, &current);
    assert_eq!(
        refused,
        Written::Refused(karakuri_operation_record::Refusal { lane: 3, deck: 1 }),
        "a fade onto a deck whose fader a lane holds was converted into records — the \
         lane cancels the fade within one step and a replay, which runs no sequencer, \
         would run it"
    );
    let told = unwritten(&fade, &refused).expect(
        "a fade this window refused said nothing at all, so a key that emitted one \
         reads exactly like a key that is not bound",
    );
    assert!(
        told.contains(&karakuri_operation_record::Refusal { lane: 3, deck: 1 }.why()),
        "the window said `{told}`, which is not the sentence the refusal is worded in \
         — the next attempt is to mute the lane it names"
    );
    let gap = unwritten(
        &Operation::TapBeat,
        &written(&Operation::TapBeat, &Current::default()),
    )
    .expect("a tap owes a record and this window says so");
    assert_ne!(
        told, gap,
        "a decision taken and a gap nobody has closed came out of this window as the \
         same sentence"
    );

    // **And the reading this window hands over.** `reading` takes the banks
    // and fills the field from the armed pattern; either half missing is a
    // refusal that silently never happens.
    // Read with the whitespace taken out, so that a reformat of the file is
    // not a failing test and a line wrapped by `cargo fmt` is not a silence.
    const APPLY: &str = include_str!("../bridge/handlers/apply.rs");
    let apply: String = APPLY.split_whitespace().collect();
    for wanted in [
        "banks:&karakuri_pattern::Banks,",
        "banks.pattern().held()",
        // The field of the `Current` this window builds, and not a mention of
        // the word in a comment above it.
        "mix,lanes,}",
    ] {
        assert!(
            apply.contains(wanted),
            "`reading` no longer carries `{wanted}` — the lanes reading is how a \
             scheduled move meets the lane holding its fader, and a field left out \
             refuses nothing and says nothing"
        );
    }
}

/// What a model asking over `--mcp` is told about an operation this window
/// refused, owed or performed — and *performed* is one of the three answers
/// rather than all of them (ADR-0131, P-0083, ADR-0315).
///
/// The defect this pins was one sentence and no branch: the drain answered
/// ``was performed on the frame it arrived on`` for every operation it took,
/// so a model that asked for a fade on a fader an unmuted lane of the armed
/// pattern holds was told the move was running. Nothing was scheduled, nothing
/// moved, the lane still held the fader, and the one place that said so was
/// this run's terminal — which a model does not have (ADR-0315). It then asked
/// for the next thing.
///
/// Four answers, and the first two are the repair:
///
/// - A refusal is the refusal's own sentence, in the wording
///   `karakuri-cli` answers a refusal in. The `assert_eq!` is against
///   [`not_performed`] rather than a spelling written out here, which is
///   `no_such_slot`'s lesson (ADR-0131): four spellings of one refusal lived
///   side by side because every test asked only whether the range appeared in
///   it. Pinning both programs to the one function is what makes them one
///   sentence — there is no second string to drift from.
/// - A gap is the gap's sentence and not the refusal's, which is
///   [`unwritten`]'s distinction carried onto the socket.
/// - A record is *performed*, unchanged.
/// - **And so is a `Silent`**, which is where this program's answer differs
///   from `karakuri-cli`'s and is deliberate: that program performs an
///   operation by writing records, so a `Silent` is one it has no control for;
///   this one has the control. `SelectDeck` moves the ring, and answering
///   *nothing on this run changed* for it would be this fix writing the defect
///   it repairs the other way round.
///
/// The fifth half is the wiring, because none of the four enters the drain.
/// `App::operated` takes a `Gfx` and an `ActiveEventLoop` and this binary has
/// neither, so the source is scanned for the conversion being carried out of
/// `App::performed` and for the drain consulting it — without that, every
/// assertion above passes against a function nothing calls, which is
/// `docs/contributing.md` §3's whole subject.
#[test]
fn a_model_is_answered_the_refusal_rather_than_told_its_move_was_performed() {
    let fade = Operation::FadeDeck { deck: 1, to: 0.0 };
    let held = Current {
        transition: Some(karakuri_operation_record::Transition {
            start: 8.0,
            beats: 4.0,
            curve: karakuri_operation::Curve::Smooth,
            wipe_kind: karakuri_operation::WipeKind::None,
            wipe_angle: 0.0,
        }),
        lanes: Some(karakuri_operation_record::Lanes {
            held: vec![(3, karakuri_operation::LaneTarget::Fader { deck: 1 })],
        }),
        ..Current::default()
    };
    let refusal = karakuri_operation_record::Refusal { lane: 3, deck: 1 };
    let refused = written(&fade, &held);
    assert_eq!(
        refused,
        Written::Refused(refusal),
        "a fade onto a deck whose fader a lane holds was converted into records, so the \
         answer this test is about is not the one being asserted"
    );
    let said = unperformed(fade.title(), &refused).expect(
        "a fade this window refused answered a model nothing at all, so the drain falls \
         through to `was performed` for a move that was never scheduled",
    );
    assert_eq!(
        said,
        not_performed(fade.title(), &refusal.why()),
        "the window answered `{said}`, which is not the sentence `karakuri-cli` answers \
         the same refusal in — one mistake, one explanation, whichever program a model \
         came through"
    );
    assert!(
        !said.contains("was performed"),
        "a refused move was reported to a model as performed: `{said}`"
    );

    // **A gap, and it is not the refusal's sentence.** A scrub with no
    // transport read is the reading this window fails to take when the deck it
    // names is not one it holds.
    let scrub = Operation::ScrubDeck {
        deck: 0,
        beats: 0.25,
    };
    let gap = written(&scrub, &Current::default());
    assert_eq!(
        gap,
        Written::Owed(Owed::NotRead(karakuri_operation_record::Reading::Transport)),
        "a scrub with no transport read no longer owes a record, so the gap this test is \
         about is not the one being asserted"
    );
    let owed = unperformed(scrub.title(), &gap)
        .expect("a scrub this window could not convert answered a model nothing at all");
    assert_eq!(
        owed,
        not_performed(
            scrub.title(),
            Owed::NotRead(karakuri_operation_record::Reading::Transport).why()
        ),
        "the window answered `{owed}`, which is not the words the crate that owes the \
         record says the gap in"
    );
    assert_ne!(
        owed, said,
        "a decision taken and a gap nobody has closed went back over the socket as the \
         same sentence"
    );

    // **A record is performed, and so is a `Silent`.** `None` here is the
    // drain falling through to the sentence that says so.
    let gain = Operation::SetGain { deck: 0, gain: 0.5 };
    assert_eq!(
        unperformed(gain.title(), &written(&gain, &Current::default())),
        None,
        "an operation that wrote a record was answered as though nothing on this run \
         changed"
    );
    let select = Operation::SelectDeck { deck: 0 };
    let silent = written(&select, &Current::default());
    assert_eq!(
        silent,
        Written::Silent(Silent::Surface),
        "a deck selection no longer writes no record, so the arm this test is about is \
         not the one being asserted"
    );
    assert_eq!(
        unperformed(select.title(), &silent),
        None,
        "an operation this window performs on its own surface — the ring moves — was \
         answered `nothing on this run changed`, which is the defect this test is about \
         written the other way round"
    );

    // **And the drain, read out of its own source**, because nothing above
    // enters it: the conversion has to be carried out of `App::performed` and
    // consulted before the `Ok` is built, and either half missing is four
    // green assertions over a function with no caller.
    // Read with the whitespace taken out, so that a reformat of the file is
    // not a failing test and a line wrapped by `cargo fmt` is not a silence.
    const APP: &str = include_str!("../app/mod.rs");
    let app: String = APP.split_whitespace().collect();
    for wanted in [
        // Carried out of the performer, at the one place the readings the
        // conversion needs are true (ADR-0323).
        "converted=Some(written);",
        // And consulted by the drain, ahead of the sentence that says it was
        // performed.
        "unperformed(title,written)",
        "reply.settled(Err(refused));",
    ] {
        assert!(
            app.contains(wanted),
            "`App::operated` no longer carries `{wanted}` — the outcome not reaching the \
             reply is a model told its refused move was performed, with every assertion \
             in this test still green"
        );
    }
}

/// The deck head's two operations, as far as this program can take them without
/// a device — and they go the same distance now, which is the point.
///
/// They used to go different distances: a scrub became a record and a sync mode
/// did not, and the second half of that is what `tests/panel_column.rs`'s one
/// exemption rested on — the chip's badge stayed `plan` because an operator who
/// pressed it reached the emission and not the move. That test said the day it
/// stopped being true it would stop being true here, and this is here.
///
/// The two are still not the same conversion, and that is what the second half
/// asserts. A scrub is relative and reads the transport it moves from; a mode
/// is absolute and reads the session tempo, replacing the anchor and clearing
/// the scrub. A sync mode that came out carrying the position the slot was
/// scrubbed to would be the two conversions having been made one.
#[test]
fn the_deck_heads_two_operations_go_different_distances() {
    // **The scrub is relative, so the record is where the slot is plus
    // what was asked for.** The reading is handed in by hand here for
    // `reading`'s reason at the mask: there is no deck in this test
    // binary, and what is being checked is the arithmetic rather than the
    // read.
    let current = Current {
        transport: Some(karakuri_operation_record::Transport {
            sync: karakuri_operation::Sync::Beat,
            anchor_bpm: 128.0,
            scrub_beats: -1.5,
        }),
        ..Current::default()
    };
    // **The amount is the console's own constant**, not a figure written
    // again here: the arrow that emits it and the record that carries it
    // are one number or the panel and the deck disagree about how far a
    // press goes.
    let scrub = Operation::ScrubDeck {
        deck: 1,
        beats: SCRUB_BEATS,
    };
    assert_eq!(
        written(&scrub, &current),
        Written::Records(vec![Record::Transport {
            slot: DeckSlot(1),
            sync: "beat".to_owned(),
            anchor_bpm: 128.0,
            scrub_beats: -1.25,
        }]),
        "a press of the deck head's forward arrow, from -1.50, did not come out at -1.25 — \
         so the record is not the offset the deck holds plus the amount the arrow asks for"
    );
    // **And the reading is what makes it one**: without it the conversion
    // says so rather than starting the deck's scrub from zero, which is
    // why `reading` has an arm for this operation at all.
    assert_eq!(
        written(&scrub, &Current::default()),
        Written::Owed(Owed::NotRead(karakuri_operation_record::Reading::Transport)),
        "a scrub with no transport read came back with a record, which means it invented \
         the position it moved from"
    );
    // **The mode goes out as a wire name and the engine reads its own name
    // back**, which is what `apply` does with it and is the blend chip's
    // assertion one control along.
    for sync in SYNCS {
        assert_eq!(
            EngineSync::from_name(sync.name()).map(mix::sync),
            Some(sync),
            "the engine does not know the vocabulary's `{}`",
            sync.name()
        );
    }

    // **A sync mode anchors at the session tempo and starts on the
    // grid.** The reading handed in is the same one the scrub used —
    // anchored at 128 and scrubbed to -1.5 — and none of it may survive:
    // `Transport::engaged` clears the scrub because *"a slot brought back
    // to the grid should be on the grid, not on wherever it was scrubbed
    // to a song ago"*, and the anchor is the room's tempo rather than the
    // one the slot was last locked to.
    let set = Operation::SetSync {
        deck: 1,
        sync: karakuri_operation::Sync::Beat,
    };
    let engaged = Current {
        tempo: Some(126.0),
        ..current
    };
    assert_eq!(
        written(&set, &engaged),
        Written::Records(vec![Record::Transport {
            slot: DeckSlot(1),
            sync: "beat".to_owned(),
            anchor_bpm: 126.0,
            scrub_beats: 0.0,
        }]),
        "a press of the deck head's sync chip, in a room at 126 bpm, did not come out \
         anchored at 126 with the scrub cleared — either the slot's old anchor survived \
         being re-engaged, or the position it was scrubbed to did"
    );
    // **And the reading is what makes it one.** Without the tempo the
    // conversion says so rather than anchoring at a guess, which is the
    // scrub's own arrangement two assertions up and the reason `reading`
    // has an arm for this operation at all.
    assert_eq!(
        written(&set, &Current::default()),
        Written::Owed(Owed::NotRead(karakuri_operation_record::Reading::Tempo)),
        "a sync mode with no session tempo read came back with a record, which means the \
         tempo it anchored the deck at was invented"
    );
    let said = unwritten(&set, &written(&set, &Current::default())).expect(
        "a sync chip press with no tempo read said nothing at all — a press that reads, in \
         silence, exactly like a press that did not work",
    );
    assert!(
        said.contains("SetSync")
            && said.contains(Owed::NotRead(karakuri_operation_record::Reading::Tempo).why()),
        "the window said `{said}`, which does not name both the operation and the reading \
         it did not get"
    );
}

/// The two crates walk the sync modes in one order, which is what makes
/// `view::Pane::allows` line up with the field it fills.
///
/// [`inspector`] builds that array by mapping `EngineSync::ALL` and the console
/// reads it by indexing [`SYNCS`], so the two orders are one order or the panel
/// skips the wrong mode — silently, and only on material that refuses
/// something. Two arrays cannot be made one by a comment.
#[test]
fn the_two_crates_walk_the_sync_modes_in_one_order() {
    assert_eq!(EngineSync::ALL.len(), SYNCS.len());
    for (index, mode) in EngineSync::ALL.into_iter().enumerate() {
        assert_eq!(
            mix::sync(mode),
            SYNCS[index],
            "`EngineSync::ALL[{index}]` is `{}` and the console's `SYNCS[{index}]` is \
             `{}` — the deck head's cycle would skip the wrong mode",
            mode.name(),
            SYNCS[index].name()
        );
    }
}

/// [`tally`] the other way round, for the assertion above alone — the
/// vocabulary's residency as the engine's, so that the round trip through the
/// wire name can be compared against something.
fn residency_back(residency: karakuri_operation::Residency) -> Residency {
    match residency {
        karakuri_operation::Residency::Live => Residency::Live,
        karakuri_operation::Residency::Priming => Residency::Priming,
        karakuri_operation::Residency::Allocated => Residency::Allocated,
    }
}

/// [`blend_mode`] the other way round, for the assertion above alone — which is
/// why it is here and not beside it: nothing the program *runs* needs to go
/// this direction, and a conversion in `src` with one test as its only caller
/// would be an abstraction with no second call site.
fn blend_mode_back(blend: BlendMode) -> Blend {
    match blend {
        BlendMode::Add => Blend::Add,
        BlendMode::Over => Blend::Over,
        BlendMode::Max => Blend::Max,
    }
}

/// Anything that makes texels this frame keeps the loop awake, and the list is
/// closed.
///
/// [`live`] decides whether the loop asks for another frame, and it is the one
/// decision in this file that has already been got wrong twice in the same
/// direction. The first time it was set once and never cleared, so folding the
/// picture away left the window drawing at full rate — found by an operator on
/// another machine following this file's own instructions, which said the
/// window goes quiet, and getting 270 frames. The second time it was the
/// picture alone, which is the same failure with a preview under it: fold the
/// picture and deck A goes on auditioning while the loop stops asking for
/// frames, so the panel keeps changing and nothing draws it.
///
/// So the assertion is over every sink, not over the one this program fills: a
/// cell nobody has wired up yet is asserted live all the same, because the
/// failure is a sink left out of the list rather than a sink that is off.
///
/// It needs no device: an `egui::TextureId` is a number, and what is being
/// asserted is a rule about `Option`s.
#[test]
fn anything_that_makes_texels_keeps_the_loop_awake() {
    let some = Picture {
        id: egui::TextureId::User(0),
        rect: egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(16.0, 9.0)),
    };
    let mut view = View::new(Room::Day);

    // Nothing is making texels, so the loop has no reason of its own to
    // draw and `ControlFlow::Wait` gets to block.
    assert!(!live(&view), "an empty panel was called live");

    // The picture, which is what this rule used to be the whole of.
    view.picture = Some(some);
    assert!(live(&view), "a live picture did not keep the loop awake");

    // **The case the picture-alone rule gets wrong**: the picture folded
    // away with deck A still auditioning under it.
    view.picture = None;
    view.previews[0] = Some(some);
    assert!(
        live(&view),
        "the picture is folded away and deck A is still rendering, and the loop was \
         told to sleep — which is the window that kept drawing 270 frames after it \
         was said to have gone quiet"
    );

    // And the list is closed: every cell counts, including the three this
    // program leaves off, because the bug is a sink that is not read here.
    for deck in 0..DECKS {
        let mut view = View::new(Room::Day);
        view.previews[deck] = Some(some);
        assert!(
            live(&view),
            "deck {deck} is rendering and the loop was told to sleep"
        );
    }

    // The other direction, which costs frames rather than pixels: with
    // every sink off the loop stops asking.
    view.previews[0] = None;
    assert!(
        !live(&view),
        "nothing is rendering and the loop stayed awake"
    );
}

/// Two paths or none, and anything else is a refusal rather than a guess.
///
/// [`sources_from`] is the whole of this program's command line and this is
/// what stops it growing a second one. The mistake it will actually be given is
/// *one* path — a Set is two files and reads like one thing — and that is
/// refused by name rather than paired with a default renderer, because a
/// program that silently supplied half the material would draw something nobody
/// asked for and say nothing about it.
///
/// A CPU test: nothing here opens a file, and a path that does not exist is
/// still a path. What is behind one is [`checked`]'s to complain about.
#[test]
fn a_set_is_two_paths_or_none_and_anything_else_is_refused() {
    let of = |args: &[&str]| sources_from(args.iter().map(|a| (*a).to_string()));

    let bare = of(&[]).expect("no arguments is the pair the preset library ships");
    assert_eq!(bare.sources.l1, shipped().l1);
    assert_eq!(bare.sources.l4, shipped().l4);
    assert!(
        bare.sources.l1.is_file() && bare.sources.l4.is_file(),
        "the default pair is not on the disk at {} and {}, so a bare run cannot draw",
        bare.sources.l1.display(),
        bare.sources.l4.display()
    );

    let named = of(&["a/geo.kir", "b/ren.kir"]).expect("two paths are a Set");
    assert_eq!(named.sources.l1, std::path::PathBuf::from("a/geo.kir"));
    assert_eq!(named.sources.l4, std::path::PathBuf::from("b/ren.kir"));
    assert_eq!(
        named.sources.material(),
        "geo + ren",
        "the strip is not named after what was actually loaded"
    );

    let one = of(&["a/geo.kir"]).expect_err(
        "one path was read as a Set, so this program would have invented the other half",
    );
    assert!(
        one.contains("a/geo.kir"),
        "the refusal `{one}` does not name the path it refused"
    );
    assert!(
        of(&["a.kir", "b.kir", "c.kir"]).is_err(),
        "three paths were read as a Set"
    );

    // **An empty message is `--help`**, which is the one arm that is a
    // request rather than a mistake — [`main`] prints [`USAGE`] to stdout
    // and exits 0 on it, and prints it to stderr and exits 2 on every
    // other. A refusal that came back empty would be a silent exit.
    assert_eq!(of(&["--help"]).err(), Some(String::new()));
    assert_eq!(of(&["-h"]).err(), Some(String::new()));
    assert!(
        !one.is_empty(),
        "a refusal came back with no sentence in it, which `main` reads as `--help`"
    );
}

/// `--mcp` takes a port, and it is refused in the three ways a flag with a
/// value is refused.
///
/// The first two are [`value_for`]'s and are the two the other flags already
/// meet — a flag at the end of the line does not fall back to a default, and a
/// flag whose value is the next flag does not eat it. The third is
/// [`number_for`]'s and is new here, because this is the first flag on this
/// command line that takes a number: a port that is not a port is a mistake on
/// the command line, and a run that started serving on some other number would
/// be the wrong kind of helpful.
#[test]
fn the_mcp_flag_takes_a_port_and_is_refused_the_three_ways_a_valued_flag_is() {
    let read = |args: &[&str]| sources_from(args.iter().map(|a| a.to_string()).collect::<Vec<_>>());

    let launch = read(&["--mcp", "8000"]).expect("a port is a port");
    assert_eq!(launch.mcp, Some(8000));
    // **On either side of the pair, like the two flags beside it.** An
    // operator types the flags in whatever order they think of them.
    let pair = shipped();
    let (l1, l4) = (pair.l1.display().to_string(), pair.l4.display().to_string());
    let launch = read(&[&l1, &l4, "--mcp", "0"]).expect("after the pair");
    assert_eq!(launch.mcp, Some(0), "a port after the pair");
    let launch = read(&["--mcp", "0", &l1, &l4]).expect("before the pair");
    assert_eq!(launch.mcp, Some(0), "a port before the pair");

    // And a run that does not ask serves nothing rather than a default port.
    assert_eq!(
        read(&[&l1, &l4]).expect("no flag").mcp,
        None,
        "a run that did not ask for a server was given one"
    );

    // The end of the line: nothing after the flag.
    let why = read(&["--mcp"]).expect_err("a flag with nothing after it");
    assert!(why.contains("--mcp"), "the refusal does not name it: {why}");
    assert!(
        why.contains("needs a value"),
        "the refusal is not the one the other flags give: {why}"
    );

    // The next flag is not a value: `--mcp --store x` must blame `--mcp`
    // rather than reading `--store` as a port and then blaming `x` for
    // being an unknown option.
    let why = read(&["--mcp", "--store", "somewhere"]).expect_err("a flag as a value");
    assert!(
        why.contains("--mcp") && why.contains("--store"),
        "the refusal does not say which flag ate which: {why}"
    );

    // And a value that is not a number.
    let why = read(&["--mcp", "eight-thousand"]).expect_err("a port that is not one");
    assert!(
        why.contains("eight-thousand") && why.contains("a port number"),
        "the refusal does not say what was expected: {why}"
    );
}

/// A wire request reaches the slot's watcher, and the rest of that watcher's
/// aim is restated with it.
///
/// The three points `mcp::WireRequest` owes, checked without a window: the edge
/// is replaced rather than appended and keyed on the input, the slot is
/// re-aimed with the run's whole wiring, and a slot this deck has not got is
/// refused in the one sentence every surface refuses one in.
///
/// The other fields are the point of the second assertion. An `Aim` is every
/// field of a slot's identity, and a rewiring that restated only the edges
/// would come back with the outgoing slot's camera, fold and salts — a defect
/// that shows on the *next* build rather than on the rewiring, which is why it
/// is asserted here rather than left to be seen. The Set the slot is running is
/// among them, and it is the one whose symptom is not a picture at all:
/// versions filed under the wrong Set, or under none.
#[test]
fn a_wire_request_reaches_the_slots_watcher_with_the_rest_of_its_aim_restated() {
    let edge = |node: &str, slot: &str, to: &str| karakuri_engine::set::Edge {
        node: node.to_string(),
        slot: slot.into(),
        to: to.to_string(),
    };
    let (tx, rx) = std::sync::mpsc::channel();
    let mut aims = vec![Aiming::new(
        tx,
        watch::Aim {
            head: karakuri_environment::compile::Named {
                name: Some("grid".into()),
                path: std::path::PathBuf::from("A0-grid.kir"),
            },
            rest: vec![karakuri_environment::compile::Named::bare("A1-points.kir")],
            layering: Layering::Composite,
            live: Some(0),
            capacity: Some(2048),
            seed_salt: 9,
            salts: vec![9],
            camera: karakuri_engine::camera::Orbit::default(),
            overrides: Vec::new(),
            published: Vec::new(),
            bindings: Vec::new(),
            edges: Vec::new(),
            authorities: Vec::new(),
            // **A slot running a Set**, which is what makes the assertion
            // below about `restated` rather than about a default.
            set: Some("night01".to_owned()),
        },
        karakuri_mcp::Slots::unpointed(),
        0,
    )];
    let mut edges = Vec::new();

    let said = rewired(
        &[(0, edge("warp", "shape", "field"))],
        &mut edges,
        &mut aims,
        1,
    );
    assert_eq!(said.len(), 1);
    let line = said[0].as_ref().expect("the slot is in range");
    assert!(
        line.contains("warp.shape=field") && line.contains("recompiling"),
        "the answer does not say what was wired or that anything rebuilds: {line}"
    );
    let aim = rx.try_recv().expect("the watcher was not re-aimed at all");
    assert_eq!(aim.edges, vec![edge("warp", "shape", "field")]);
    // **The fields that are not the edges.**
    assert_eq!(aim.head.name.as_deref(), Some("grid"));
    assert_eq!(
        aim.set.as_deref(),
        Some("night01"),
        "the rewiring dropped the Set the slot is running, so every version \
         written after it would be filed under none"
    );
    assert_eq!(aim.live, Some(0), "the fold was silently un-selected");
    assert_eq!(aim.capacity, Some(2048), "the capacity came back as none");
    assert_eq!(aim.salts, vec![9], "the salts would repaint every element");
    assert_eq!(aim.layering, Layering::Composite);

    // **The same input again is a replacement and not a second edge**,
    // because `SetError::SlotBoundTwice` refuses two edges on one input
    // where the Set is built — an append would make a model unable to
    // change its mind.
    let said = rewired(
        &[(0, edge("warp", "shape", "other"))],
        &mut edges,
        &mut aims,
        1,
    );
    assert!(said[0].is_ok(), "{:?}", said[0]);
    assert_eq!(
        edges,
        vec![edge("warp", "shape", "other")],
        "the run is wired with both, and the Set will refuse to build"
    );
    let aim = rx.try_recv().expect("the second request re-aimed nothing");
    assert_eq!(aim.edges, vec![edge("warp", "shape", "other")]);
    // And the aim the watcher is pointed at moved with it, so a third
    // request restates the second rather than the first.
    assert_eq!(aims[0].at.edges, vec![edge("warp", "shape", "other")]);

    // A slot this deck has not got, in the one sentence.
    let said = rewired(
        &[(3, edge("warp", "shape", "field"))],
        &mut edges,
        &mut aims,
        1,
    );
    let why = said[0].as_ref().expect_err("slot 3 of a deck of one");
    assert_eq!(
        why,
        &format!(
            "{}, and nothing was rewired",
            karakuri_environment::no_such_slot(3, 1)
        ),
        "the refusal is not the one every other surface gives"
    );
    assert_eq!(
        edges,
        vec![edge("warp", "shape", "other")],
        "a refused request wrote an edge anyway"
    );
    assert!(
        rx.try_recv().is_err(),
        "a refused request re-aimed a watcher"
    );
}

/// A press on the Inspector deck head's fold re-aims the slot, and the rest of
/// that watcher's aim is restated with it.
///
/// The test above one operation along, and it is the same property for the same
/// reason: a `watch::Aim` is every field of a slot's identity, so an arm that
/// changed the layering and left the rest behind would come back with the
/// outgoing slot's fold, capacity, salts, camera and Set — on the *next* build
/// rather than on the press, which is the hardest version of it to see
/// (ADR-0228, ADR-0314).
///
/// `Aiming::at` is what the second half asserts against. A press that sent an
/// aim and left `at` behind would leave the next re-aim restating the layering
/// the run launched with, so the third assertion here is that a *second* press
/// comes back to where the first one put it rather than to where the run
/// started.
///
/// No window, no device and no `Deck` — `composited` is a free function over
/// the aims for exactly this. A procedure loaded over a layer re-aims the slot
/// with exactly one file replaced, and leaves `Aim::set` where it is —
/// ADR-0338's decision 3, at the seam it crosses.
///
/// Three things it would be wrong about silently: the position it lands on (the
/// first node of that kind), the file it puts there (the procedure's own bytes,
/// in the deck's scratch), and everything else about the aim, which has to come
/// back restated rather than defaulted. The fourth is the one the maintainer
/// answered: the versions this slot writes from here on go on being filed under
/// the Set it started from.
///
/// No window, no device and no `Deck` — `overlaying` takes the aim. The strip
/// reads `<base> + <kir>` once a layer has been written over what a deck is
/// playing, and the base is the Set it is filed under — or the launch pair
/// where it is filed under none (ADR-0338).
#[test]
fn the_strip_reads_the_base_and_the_procedure_written_over_it() {
    assert_eq!(
        derived_material(
            &base_material(Some("drift_night"), "coil_vortex + star_flares"),
            "orbit_wide"
        ),
        "drift_night + orbit_wide"
    );
    // **A slot nobody has loaded a Set onto**: no id names what it is
    // running, so the base is the pair the run opened with.
    assert_eq!(
        derived_material(
            &base_material(None, "coil_vortex + star_flares"),
            "orbit_wide"
        ),
        "coil_vortex + star_flares + orbit_wide"
    );
}

#[test]
fn a_procedure_load_replaces_one_file_and_keeps_the_base_set() {
    let root = scratch_dir("procedure-load");
    Store::open(&root).expect("a store to keep in");
    std::fs::write(
        root.join(Store::PROCEDURES).join("orbit_wide.kir"),
        "proc orbit_wide {\n  kind L3\n}\n",
    )
    .expect("a kept procedure");
    // The slot's own two files, written where a watcher would be looking:
    // an L1 and an L4, which is the pair every run opens on.
    let l1 = karakuri_environment::scratch::place(&root, "A0-drift_shell", "kind L1\n")
        .expect("the geometry");
    let l4 = karakuri_environment::scratch::place(&root, "A1-star_flares", "kind L4\n")
        .expect("the renderer");

    let (tx, rx) = std::sync::mpsc::channel();
    let mut aim = Aiming::new(
        tx,
        watch::Aim {
            head: karakuri_environment::compile::Named {
                name: Some("shell".into()),
                path: l1.clone(),
            },
            rest: vec![karakuri_environment::compile::Named {
                name: Some("flares".into()),
                path: l4.clone(),
            }],
            layering: Layering::Composite,
            live: Some(2),
            capacity: Some(2048),
            seed_salt: 9,
            salts: vec![9],
            camera: karakuri_engine::camera::Orbit {
                radius: 3.5,
                ..karakuri_engine::camera::Orbit::default()
            },
            overrides: Vec::new(),
            published: Vec::new(),
            bindings: Vec::new(),
            edges: vec![karakuri_engine::set::Edge {
                node: "flares".to_string(),
                slot: "shape".into(),
                to: "shell".to_string(),
            }],
            authorities: Vec::new(),
            set: Some("drift_night".to_owned()),
        },
        karakuri_mcp::Slots::unpointed(),
        0,
    );

    // **The deck holds no camera, so the procedure is added as node 0 of
    // its kind** — the case the row is for.
    let line = overlaying(&root, None, 0, &mut aim, "orbit_wide").expect("the load was refused");
    assert!(
        line.contains("orbit_wide") && line.contains("kept") && line.contains("L3"),
        "{line}"
    );
    let sent = rx.try_recv().expect("no aim was sent");
    assert_eq!(sent.head.path, l1, "the geometry moved");
    assert_eq!(sent.rest.len(), 2, "the slot does not hold three nodes now");
    assert_eq!(sent.rest[0].path, l4, "the renderer moved");
    assert_eq!(
        std::fs::read_to_string(&sent.rest[1].path).expect("the camera's file"),
        "proc orbit_wide {\n  kind L3\n}\n",
        "the file the aim names is not the procedure's own bytes"
    );
    assert_eq!(
        sent.rest[1].name.as_deref(),
        Some("orbit_wide"),
        "a node added by this row is not named after it"
    );

    // **Everything else restated**, which is `Aiming::changed`'s single
    // derivation — the layering, the capacity, the salts, the camera and
    // the wiring come back as the slot's own.
    assert_eq!(sent.layering, Layering::Composite);
    assert_eq!(sent.capacity, Some(2048));
    assert_eq!(sent.salts, vec![9]);
    assert_eq!(sent.camera.radius, 3.5);
    assert_eq!(sent.edges.len(), 1);
    // **And the Set it is filed under does not move**, which is what keeps
    // the snapshot every compile takes alive (ADR-0304, ADR-0308).
    assert_eq!(sent.set.as_deref(), Some("drift_night"));

    // **A second load of the same kind lands on the node the first one
    // added**, which is *the first node of that kind* read a second time:
    // the slot still holds three nodes.
    std::fs::write(
        root.join(Store::PROCEDURES).join("tunnel_eye.kir"),
        "  kind L3\n",
    )
    .expect("a second camera");
    overlaying(&root, None, 0, &mut aim, "tunnel_eye").expect("the second load was refused");
    let sent = rx.try_recv().expect("no second aim was sent");
    assert_eq!(
        sent.rest.len(),
        2,
        "the second camera was added beside the first"
    );
    assert_eq!(
        std::fs::read_to_string(&sent.rest[1].path).expect("the camera's file"),
        "  kind L3\n"
    );
    assert_eq!(
        sent.rest[1].name.as_deref(),
        Some("orbit_wide"),
        "the replaced node did not keep the name the edges resolve against"
    );

    // **A renderer replaces the renderer that is there** — `L4:0`, and the
    // geometry does not move.
    std::fs::write(
        root.join(Store::PROCEDURES).join("hard_dots.kir"),
        "kind L4\n",
    )
    .expect("a renderer");
    overlaying(&root, None, 0, &mut aim, "hard_dots").expect("the renderer load was refused");
    let sent = rx.try_recv().expect("no third aim was sent");
    assert_eq!(sent.head.path, l1, "the geometry moved on a renderer load");
    assert_eq!(sent.rest.len(), 2);
    assert_eq!(
        sent.rest[0].name.as_deref(),
        Some("flares"),
        "the renderer did not keep its node name"
    );
    assert_ne!(
        sent.rest[0].path, l4,
        "the renderer's file was not replaced"
    );

    // **A name neither tier holds is refused with the name back**, and
    // nothing is sent.
    let why = overlaying(&root, None, 0, &mut aim, "no_such_thing")
        .expect_err("a name nothing holds was loaded");
    assert!(
        why.contains("no_such_thing") && why.contains("procedures"),
        "{why}"
    );
    assert!(rx.try_recv().is_err(), "a refused load sent an aim");

    // **A `.kir` that declares no kind is refused too**, because there is
    // no layer to write it over.
    std::fs::write(
        root.join(Store::PROCEDURES).join("mute.kir"),
        "// nothing\n",
    )
    .expect("a procedure with no kind");
    let why = overlaying(&root, None, 0, &mut aim, "mute")
        .expect_err("a procedure with no kind was loaded");
    assert!(why.contains("declares no `kind`"), "{why}");

    std::fs::remove_dir_all(&root).expect("clean up");
}

#[test]
fn a_composite_press_re_aims_the_slot_and_restates_the_rest_of_its_aim() {
    let (tx, rx) = std::sync::mpsc::channel();
    let mut aims = vec![Aiming::new(
        tx,
        watch::Aim {
            head: karakuri_environment::compile::Named {
                name: Some("grid".into()),
                path: std::path::PathBuf::from("A0-grid.kir"),
            },
            rest: vec![karakuri_environment::compile::Named::bare("A1-points.kir")],
            // **Overdrawing**, so the press below asks for the other one
            // and the assertion is about a field that moved.
            layering: Layering::Overdraw,
            live: Some(2),
            capacity: Some(2048),
            seed_salt: 9,
            salts: vec![9],
            // **A camera nobody's default produces**, so the assertion
            // below is about a value that was carried rather than one that
            // happens to coincide with `Orbit::default()`.
            camera: karakuri_engine::camera::Orbit {
                radius: 3.5,
                ..karakuri_engine::camera::Orbit::default()
            },
            overrides: Vec::new(),
            published: Vec::new(),
            bindings: Vec::new(),
            edges: vec![karakuri_engine::set::Edge {
                node: "warp".to_string(),
                slot: "shape".into(),
                to: "field".to_string(),
            }],
            authorities: Vec::new(),
            set: Some("night01".to_owned()),
        },
        karakuri_mcp::Slots::unpointed(),
        0,
    )];

    let line = composited(
        &mut aims,
        &Operation::SetCompositing {
            deck: 0,
            compositing: true,
        },
    )
    .expect("the arm answered nothing for the operation it is for");
    assert!(
        line.contains("composite") && line.contains("recompiling"),
        "the answer does not say what was asked for or that the slot rebuilds: {line}"
    );

    let aim = rx.try_recv().expect("the watcher was not re-aimed at all");
    assert_eq!(
        aim.layering,
        Layering::Composite,
        "the press did not move the one field it is about"
    );
    // **The thirteen that did not move.** Each of these is a symptom
    // somebody would meet on the next save rather than on this press.
    assert_eq!(aim.head.name.as_deref(), Some("grid"));
    assert_eq!(aim.rest.len(), 1);
    assert_eq!(aim.live, Some(2), "the fold was silently un-selected");
    assert_eq!(aim.capacity, Some(2048), "the capacity came back as none");
    assert_eq!(aim.seed_salt, 9);
    assert_eq!(aim.salts, vec![9], "the salts would repaint every element");
    // `Orbit` is not `PartialEq`, so the field the camera's own loss shows
    // in is what this reads — `Watch::camera`'s symptom is a slot back at
    // `Orbit::default()`, and a radius nobody could have written is what
    // tells the two apart.
    assert_eq!(
        aim.camera.radius, 3.5,
        "the camera came back at its default"
    );
    assert_eq!(aim.edges.len(), 1, "the run's wiring was dropped");
    assert_eq!(
        aim.set.as_deref(),
        Some("night01"),
        "the press dropped the Set the slot is running, so every version written after it \
         would be filed under none"
    );
    assert_eq!(aims[0].at.layering, Layering::Composite);

    // **Asking for the layering the slot is now in sends nothing**, because
    // a re-aim rebuilds the whole slot and this one would land on the same
    // picture (P-0091).
    let line = composited(
        &mut aims,
        &Operation::SetCompositing {
            deck: 0,
            compositing: true,
        },
    )
    .expect("the arm answered nothing");
    assert!(
        line.contains("already"),
        "the answer does not say the slot is already set that way: {line}"
    );
    assert!(
        rx.try_recv().is_err(),
        "a press asking for the state the slot is in recompiled it"
    );

    // **And the second press restates what the first one left**, which is
    // what keeping `Aiming::at` buys: back to overdraw, with the layering
    // read off the aim this program is holding rather than off the launch
    // pair.
    composited(
        &mut aims,
        &Operation::SetCompositing {
            deck: 0,
            compositing: false,
        },
    )
    .expect("the arm answered nothing");
    let aim = rx.try_recv().expect("the second press re-aimed nothing");
    assert_eq!(aim.layering, Layering::Overdraw);
    assert_eq!(aim.set.as_deref(), Some("night01"));

    // A slot this deck has not got, in the one sentence every surface
    // refuses one in.
    let why = composited(
        &mut aims,
        &Operation::SetCompositing {
            deck: 3,
            compositing: true,
        },
    )
    .expect("a slot the deck has not got answered nothing");
    assert!(
        why.contains(&karakuri_environment::no_such_slot(3, 1)),
        "the refusal is not the one every other surface gives: {why}"
    );
    assert!(rx.try_recv().is_err(), "a refused press re-aimed a watcher");

    // And it answers `None` for everything that is not its operation, so
    // the dispatch above can call it on every press.
    assert!(composited(&mut aims, &Operation::Quit).is_none());
}

/// The two flags say where this program's data is, and either may sit on either
/// side of the pair.
///
/// The order half is the one an operator meets: they type the flags in whatever
/// order they think of them, and `karakuri-cli` accepts `--store` before or
/// after its own command for exactly this reason
/// (`list_sets_prints_and_is_never_a_run`). A parser that matched on the
/// argument slice — which is what this one was — can only ever accept one of
/// the two spellings.
///
/// And the pair still wins, which is the claim [`Sources`]'s doc makes about
/// these flags not being a second material vocabulary: `--presets` moves what a
/// run with *no* paths opens on and reaches nothing else, so a line with both a
/// library and a pair plays the pair.
///
/// Not quite a CPU test, and this is what changed: resolving a presets root is
/// existence checks on real directories. The library it names is this
/// workspace's own `examples/`, which is on the disk whenever these tests run
/// at all.
#[test]
fn the_two_flags_say_where_the_data_is_and_may_sit_on_either_side_of_the_pair() {
    let of = |args: &[&str]| sources_from(args.iter().map(|a| (*a).to_string()));
    let library = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let library = library
        .to_str()
        .expect("this workspace's path is not utf-8");

    // **The default store is the shared constant**, which is the whole of
    // what deleting `const STORE` was for: this asserts the two programs
    // read one directory rather than two that look alike.
    assert_eq!(
        of(&[]).expect("a bare run").store,
        std::path::PathBuf::from(karakuri_environment::places::STORE),
        "a run that said nothing about a store did not get the shared default"
    );

    for spelling in [
        vec!["--store", "/tmp/library", "a/geo.kir", "b/ren.kir"],
        vec!["a/geo.kir", "b/ren.kir", "--store", "/tmp/library"],
        vec!["a/geo.kir", "--store", "/tmp/library", "b/ren.kir"],
    ] {
        let launch = of(&spelling).unwrap_or_else(|why| panic!("{spelling:?}: {why}"));
        assert_eq!(
            launch.store,
            std::path::PathBuf::from("/tmp/library"),
            "{spelling:?} read a store nobody asked for"
        );
        assert_eq!(
            launch.sources.l1,
            std::path::PathBuf::from("a/geo.kir"),
            "{spelling:?} lost the pair to the flag"
        );
        assert_eq!(launch.sources.l4, std::path::PathBuf::from("b/ren.kir"));
    }

    // `--presets` with no pair: it is what the pair defaults to, and the
    // resolution reports it as typed rather than as something found.
    let told = of(&["--presets", library]).expect("a library that is there");
    assert_eq!(
        told.sources.l1,
        std::path::Path::new(library).join("coil_vortex.kir")
    );
    assert_eq!(
        told.sources.l4,
        std::path::Path::new(library).join("star_flares.kir")
    );
    assert_eq!(
        told.presets.as_ref().map(|presets| presets.found),
        Some(karakuri_environment::places::Found::Given),
        "a `--presets` an operator typed was reported as a place this program went \
         looking in"
    );

    // And with a pair, on either side: the pair wins and the library is
    // still the one that was named.
    for spelling in [
        vec!["--presets", library, "a/geo.kir", "b/ren.kir"],
        vec!["a/geo.kir", "b/ren.kir", "--presets", library],
    ] {
        let launch = of(&spelling).unwrap_or_else(|why| panic!("{spelling:?}: {why}"));
        assert_eq!(
            launch.sources.l1,
            std::path::PathBuf::from("a/geo.kir"),
            "{spelling:?}: `--presets` overrode the paths the operator named, which \
             would make it a second way of saying what plays"
        );
        assert_eq!(launch.sources.l4, std::path::PathBuf::from("b/ren.kir"));
        assert_eq!(
            launch.presets.map(|presets| presets.dir),
            Some(std::path::PathBuf::from(library)),
            "{spelling:?} lost the library it was given"
        );
    }

    // A `--presets` that is not there is refused rather than searched
    // past, and the sentence is `places`' own — one refusal, whichever
    // program the operator reached it from.
    let missing = std::path::Path::new(library).join("no-such-library");
    let why = of(&["--presets", missing.to_str().expect("utf-8")])
        .expect_err("a `--presets` that is not there was accepted");
    assert_eq!(why, karakuri_environment::places::no_presets_at(&missing));

    // A flag with nothing after it, and a flag whose value is the next
    // flag. Neither falls back and neither swallows.
    for (spelling, wanted) in [
        (vec!["--presets"], "`--presets` needs a value"),
        (vec!["--store"], "`--store` needs a value"),
        (
            vec!["--presets", "--store", "/tmp/library"],
            "`--presets` was given no value — `--store` is an option, not one",
        ),
    ] {
        assert_eq!(
            of(&spelling).as_ref().err().map(String::as_str),
            Some(wanted),
            "{spelling:?}"
        );
    }

    // **An unknown option is not a path**, which is the mistake a typo
    // actually makes: without this, `--prests DIR` becomes a two-path Set
    // and is reported as a file that will not open.
    let typo = of(&["--prests", library]).expect_err("an unknown option was read as half of a Set");
    assert_eq!(typo, "unknown option `--prests`");
    assert!(
        !typo.is_empty(),
        "a refusal came back with no sentence in it, which `main` reads as `--help`"
    );
}

/// The capacity is the L1's own declaration, read off the `Checked`.
///
/// It was `const CAPACITY: u32 = 262144` here — `drift_shell.kir`'s declared
/// default, transcribed — for as long as this file could only ever load that
/// one file. It takes a path now, so a transcription would be right about one
/// `.kir` and silently wrong about every other: a procedure written for 131072
/// elements would run at 262144 and nothing would say so.
///
/// It is not `karakuri_ir::DEFAULT_CAPACITY` either, which is the language
/// default for a file that declared nothing and is what `check_header` makes
/// unreachable for an L1 that passed checking. The number below is asserted
/// rather than derived on purpose, and it is the reference workload's rather
/// than this program's: `docs/contributing.md` §1 names
/// `examples/drift_cloud.kset` at 1280x720, and 262144 is what that Set's L1
/// declares. It used to be asserted of whatever a bare `cargo run -p karakuri`
/// opened on, which coupled the workload to the demo and is ADR-0270. What is
/// still asserted of the shipped pair is that its capacity is read from its own
/// file, which is a different property and the one this test is named for.
#[test]
fn the_capacity_is_the_l1s_own_declaration_and_the_l4_declares_none() {
    let sources = shipped();

    // **The reference workload, pinned by name.** `drift_cloud.kset` is the
    // Set `docs/contributing.md` §1 names, and this is its L1. That the
    // `.kset` names these two parts is checked where every shipped Set is
    // composed, in `karakuri-cli`'s `examples` suite, so it is not
    // transcribed twice here.
    let reference = checked(&sources.l1.with_file_name("drift_shell.kir"));
    let pinned = reference
        .capacity
        .expect("an L1 that passed contract checking always carries a capacity");
    assert_eq!(
        pinned.default, 262_144,
        "`examples/drift_cloud.kset`'s L1 no longer declares the capacity every \
         host-clock figure in this repository was taken at, and \
         `docs/contributing.md` §1 names it as the one reference workload \
         (ADR-0270)"
    );

    // **And the pair this program opens on, checked for a per-file read and
    // not for a number.** ADR-0270 split these: which pair is the default is
    // a demo decision, and what it may not do is run at something other than
    // what its own file declares.
    let l1 = checked(&sources.l1);
    let declared = l1
        .capacity
        .expect("an L1 that passed contract checking always carries a capacity");
    assert!(
        declared.contains(declared.default),
        "the file's own default is outside the range the same file declares"
    );

    assert_eq!(capacity_of(&l1), declared.default);

    assert!(
        checked(&sources.l4).capacity.is_none(),
        "the renderer declares a capacity — `Set::build` is handed the L1's, and \
         two declarations would be two answers to how many elements there are"
    );

    // **A second L1, and it is the one that tells the two mistakes apart.**
    // The pin above is `drift_shell.kir` at 262144, which is also
    // `karakuri_ir::DEFAULT_CAPACITY` — so that assertion passes just as
    // well against a [`capacity_of`] that ignored the file and returned the
    // language default. `strand_shell.kir` declares 131072 and says why in
    // the file (512 strands x 256 samples), and it is what that defect
    // fails on. It is kept although the shipped pair no longer declares the
    // language default either (ADR-0271 moved it to `coil_vortex.kir` at
    // 10240): which pair is the default is a demo decision, and a test that
    // can only tell a per-file read from a constant while the demo happens
    // to be off the constant is a test that goes quiet the next time the
    // demo moves.
    let other = checked(&sources.l1.with_file_name("strand_shell.kir"));
    assert_eq!(
        capacity_of(&other),
        131_072,
        "a second procedure did not run at what it declares — the capacity is being \
         read from somewhere other than the file"
    );
    assert_ne!(
        capacity_of(&other),
        karakuri_ir::DEFAULT_CAPACITY,
        "the second procedure declares the language default, so this test can no \
         longer tell a per-file read from a constant — pick another `.kir`"
    );
}

/// The pair a bare run plays, for the tests that need one on the disk.
///
/// [`Sources::under`] takes a preset library and does not go looking for one;
/// this is the going-looking, and in a test binary the answer is always the
/// last candidate — the workspace this file was compiled in, which is also the
/// tree the test is run from. That is the development entry doing exactly what
/// it is for, and it is why these tests can assert the pair is on the disk
/// without an install anywhere.
///
/// A function rather than an `impl Default` on [`Sources`], because a `Default`
/// is what baked the build machine's own tree into a shipped binary: a type
/// whose default value is a search of the filesystem invites exactly that call
/// from production, and a production caller now has to say which library it
/// means.
///
/// One `.kir`, parsed and checked, for the tests that need a `Checked` and no
/// window.
///
/// Reachable from test modules because it is at the file's own scope.
///
/// `karakuri-environment`'s own five stages and not a sixth spelling. This used
/// to be a hand-rolled parse-then-check, which is what the run itself used to
/// build a slot from; the run compiles through
/// [`karakuri_environment::compile::sort_slot`] now, because that is the one
/// place that keeps the bytes a node's address is derived from
/// ([`karakuri_environment::compile::Placed::source`]). What is left here is a
/// test helper, and a test helper with its own compiler would be a second
/// answer to *does this file check* the day either moved.
#[cfg(test)]
pub(crate) fn checked(path: &std::path::Path) -> karakuri_ir::typed::Checked {
    match karakuri_environment::compile::load(path) {
        Ok((checked, _)) => checked,
        Err(report) => panic!("{report}"),
    }
}

#[cfg(test)]
pub(crate) fn shipped() -> Sources {
    let presets = karakuri_environment::places::presets(None)
        .expect("nothing was typed, so there is no typed path to refuse")
        .expect(
            "no preset library was found from the test binary, so the workspace tree this \
             test compiled in has no `examples/` in it",
        );
    Sources::under(&presets.dir)
}

/// The shipped pair in every slot, for the tests that build an [`Engine`].
///
/// A *run* may not do this — [`working_copies`] is what a run calls, and its
/// whole point is that no two slots watch one file — and this helper is not a
/// way back to that. It is legal here for the reason the copies exist: nothing
/// in these tests edits a `.kir`, no watcher of theirs ever sees a change, and
/// a test that materialised into a temporary store would be asserting the
/// copies rather than the thing it is about. The one test that *is* about the
/// copies calls `working_copies` and is named after the claim.
#[cfg(test)]
pub(crate) fn shipped_slots() -> Vec<Sources> {
    std::iter::repeat_n(shipped(), SLOTS).collect()
}

/// The reference workload's pair, for the tests whose claim is about a cost
/// rather than about what this program opens on.
///
/// `docs/contributing.md` §1 names `examples/drift_cloud.kset` —
/// `drift_shell.kir` at the 262144 elements it declares, with `soft_points.kir`
/// — and this resolves those two out of the same preset library [`shipped`]
/// answers from. It is deliberately not [`shipped_slots`], and the two were one
/// value until 2026-09-07.
///
/// What separated them is a test going quiet rather than red.
/// [`ADR-0271`](../../../docs/adr/0271-the-panel-opens-on-the-demo-rather-than-on-the-reference-workloads-pair.md)
/// moved the default pair to `examples/star_vortex.kset`'s two parts, which are
/// closed-form and 10240 elements.
/// `gpu::the_budget_parks_a_deck_and_the_strip_carries_both_residencies` then
/// measured 1.8 ms a slot against a 2.7 ms headroom and the governor answered
/// `NoPrimingNeeded` — a closed-form Set with nothing to warm — so the park the
/// test is named for was still a park and no longer the budget's. Which pair a
/// bare run opens on is a demo decision (ADR-0270); whether the budget refuses
/// a second Live slot is not, and it needs material chosen for its cost.
#[cfg(test)]
pub(crate) fn reference() -> Sources {
    let shipped = shipped();
    Sources {
        l1: shipped.l1.with_file_name("drift_shell.kir"),
        l4: shipped.l4.with_file_name("soft_points.kir"),
    }
}

#[cfg(test)]
pub(crate) fn empty_keeping() -> Keeping {
    let (_, built) = std::sync::mpsc::channel();
    let (save_tx, saves) = std::sync::mpsc::channel();
    let (send_tx, sends) = std::sync::mpsc::channel();
    let (keep_tx, keeps) = std::sync::mpsc::channel();
    Keeping {
        mcp: None,
        playing: Playing {
            playing: Vec::new(),
        },
        built,
        pending: Vec::new(),
        saves,
        save_tx,
        sends,
        send_tx,
        keeps,
        keep_tx,
        in_flight: 0,
    }
}

/// **A surface is made by the instance its adapter came from, and this
/// program has one.** `routed` opened the projector's surface on a fresh
/// `Gpu::instance()` and then asked it about `gfx.gpu.adapter`, which belongs
/// to the instance `resumed` made — a resource the fresh instance does not
/// hold, so `wgpu-core` aborted inside the `winit` mouse callback with no
/// sentence anywhere the moment the projector chip was pressed. Nothing can
/// open that window in a test (ADR-0324), so the wiring is pinned by reading
/// the source: exactly one `Gpu::instance()` in this crate, in `resumed`, and
/// every `create_surface` after it on the instance the `Gpu` keeps.
#[test]
fn every_surface_is_made_by_the_instance_the_adapter_came_from() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut fresh = Vec::new();
    let mut surfaces = Vec::new();
    let mut stack = vec![root.clone()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                if path.file_name().is_some_and(|n| n == "tests") {
                    continue;
                }
                stack.push(path);
                continue;
            }
            if path.extension().is_none_or(|e| e != "rs") {
                continue;
            }
            let src = std::fs::read_to_string(&path).unwrap();
            let rel = path.strip_prefix(&root).unwrap().display().to_string();
            for (i, line) in src.lines().enumerate() {
                let code = line.split("//").next().unwrap_or("");
                if code.contains("Gpu::instance()") {
                    fresh.push(format!("{rel}:{}", i + 1));
                }
                if code.contains("create_surface(") {
                    surfaces.push((format!("{rel}:{}", i + 1), code.trim().to_string()));
                }
            }
        }
    }
    assert_eq!(
        fresh,
        vec!["app/handler/mod.rs:83".to_string()],
        "a second wgpu::Instance would hold none of the first one's adapters: {fresh:?}"
    );
    assert!(!surfaces.is_empty(), "no surface is made anywhere");
    for (at, code) in &surfaces {
        assert!(
            code.contains("instance.create_surface(")
                || code.contains("gfx.gpu.instance.create_surface("),
            "{at}: a surface made off something other than the adapter's own instance: `{code}`"
        );
        assert!(
            !code.contains("Gpu::instance().create_surface("),
            "{at}: a surface on a fresh instance, whose adapter is another instance's: `{code}`"
        );
    }
}

/// **The picture format is a value read off a surface, and never a constant.**
/// It was `const PICTURE_FORMAT: TextureFormat = Rgba8UnormSrgb`, and no Metal
/// surface offers that format — so `routed` refused to open the projector on
/// every macOS run, naming a format the machine was never going to have. The
/// format is now read off the console's own surface in `resumed` and threaded
/// from there (ADR-0361). Nothing can open that window in a test (ADR-0324),
/// so both halves are pinned by reading the source: no 8-bit sRGB format is
/// named anywhere outside `tests/`, and the projector's check is against the
/// value the surface gave.
#[test]
fn a_picture_format_is_a_value_read_off_a_surface() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut named = Vec::new();
    let mut compared = Vec::new();
    let mut stack = vec![root.clone()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                if path.file_name().is_some_and(|n| n == "tests") {
                    continue;
                }
                stack.push(path);
                continue;
            }
            if path.extension().is_none_or(|e| e != "rs") {
                continue;
            }
            let src = std::fs::read_to_string(&path).unwrap();
            let rel = path.strip_prefix(&root).unwrap().display().to_string();
            for (i, line) in src.lines().enumerate() {
                let code = line.split("//").next().unwrap_or("");
                for spelling in ["Rgba8UnormSrgb", "Bgra8UnormSrgb"] {
                    if code.contains(spelling) {
                        named.push(format!("{rel}:{}: `{}`", i + 1, code.trim()));
                    }
                }
                if code.contains("caps.formats.contains(") {
                    compared.push((format!("{rel}:{}", i + 1), code.trim().to_string()));
                }
            }
        }
    }
    assert!(
        named.is_empty(),
        "an 8-bit sRGB format named in the source is a guess about a display that Metal \
         already falsifies — read it off the surface instead: {named:?}"
    );
    assert_eq!(
        compared.len(),
        1,
        "the projector's format check is the one place a surface's formats are asked for a \
         member, and it has moved or multiplied: {compared:?}"
    );
    let (at, code) = &compared[0];
    assert!(
        at.starts_with("app/operations.rs"),
        "{at}: the projector's format check has moved out of `routed`: `{code}`"
    );
    assert!(
        code.contains("gfx.picture_format"),
        "{at}: the projector is checked against something other than the format the console's \
         surface gave: `{code}`"
    );
}
