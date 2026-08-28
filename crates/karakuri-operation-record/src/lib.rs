//! **Where an operation becomes a record.**
//!
//! [P-0028](../../../docs/principles/0028-every-control-ends-in-the-same-record.md)
//! is *every control ends in the same record*: a panel fader, a key press, a
//! mapped MIDI message and an MCP call are the same thing exactly because all
//! four write the same [`Record`] and the deck is moved by the decode. A
//! surface emits an [`Operation`] and applies nothing
//! ([ADR-0185](../../../docs/adr/0185-a-fader-translates-a-drag-into-an-operation-and-applies-nothing.md));
//! this is the step on the other side of that seam, and until it existed there
//! was nowhere for it to happen — `karakuri-console/examples/panel.rs` built
//! three records by hand and said so.
//!
//! # Why it is a crate and not a `From` impl
//!
//! **The conversion is not pure.** `Operation::SetExposure { exposure }`
//! becomes [`Record::Look`], and that record carries the tone map operator and
//! the white point too, because a record is what a replay reconstructs a
//! session from and a stream that set an exposure without naming the operator
//! would describe a look nobody can reconstruct
//! ([ADR-0192](../../../docs/adr/0192-an-operation-asks-for-what-a-surface-can-say-and-the-record-stays-whole.md)).
//! The exposure alone does not determine the record — **the look that is
//! running does**. The mask pair is the same shape: `Record::Mask` is a shape,
//! an angle, a position and a softness, and each of
//! `Operation::SetMaskShape` and `Operation::SetMaskPosition` asks for a part
//! of it (ADR-0201). So the conversion takes an operation *and a reading of what
//! is current*, which is [`Current`], and a function of two arguments is not a
//! `From`.
//!
//! # Why it is a crate of its own
//!
//! It needs [`karakuri_operation`] and [`karakuri_store`], and **neither of
//! them may grow the other**. `karakuri-operation` has no dependencies at all
//! and that is its charter (ADR-0180) — every surface has to be able to depend
//! on it, so a `karakuri-store` under it is a serialiser a MIDI map pays for.
//! `karakuri-store` holding it would make the record layer know the
//! vocabulary, and a replay reading a stream on a headless machine has no
//! surfaces and no asks in it at all.
//!
//! `karakuri-cli` is what ADR-0180 named, and it cannot be it: that crate has
//! **no library target**, so the console's example cannot reach it — which is
//! the whole reason that example wrote three records again rather than calling
//! `mix::gain_record`. A crate that depends on two leaves costs nothing to
//! anything that does not want it, and both leaves stay leaves. The
//! alternatives, and what each of them costs, are
//! [ADR-0194](../../../docs/adr/0194-where-an-operation-becomes-a-record-is-a-crate-that-depends-on-both.md).
//!
//! # What it answers, and the three answers are the survey
//!
//! [`written`] is **one exhaustive match over all 49 operations**, which is
//! what makes the classification a fact rather than an intention: an operation
//! added to the vocabulary does not compile here until somebody has said what
//! it writes. The three answers are the three groups the survey found:
//!
//! - [`Written::Records`] — it writes these, in this order. Eleven operations,
//!   six of which need no reading at all.
//! - [`Written::Silent`] — it writes none, and that is settled. Twenty-seven,
//!   for [`Silent`]'s four different reasons.
//! - [`Written::Owed`] — it writes one and this build cannot make it. Eleven,
//!   for [`Owed`]'s three different reasons.
//!
//! **`Owed` is not a refusal and not an error.** It is a gap this crate
//! declares about itself, in the shape `karakuri_operation::Undecided` is: a
//! caller that meets one has met a question nobody has answered, and printing
//! it is more use than a silent no-op. Four of the eleven are the vocabulary's
//! own `Undecided` rows.
//!
//! # What this crate deliberately cannot do
//!
//! **No engine.** Quantising a beat onto the grid is
//! `karakuri_engine::transition::quantise`, clamping an anchor tempo is
//! `Transport::engaged`, and neither is reachable from here — which is right,
//! because a crate that pulled `wgpu` in would be unreachable from every
//! surface again. Anything an operation's record needs that is arithmetic
//! rather than a value has to arrive inside [`Current`], and the seven
//! operations whose record needs the grid are [`Owed::NotSettled`] until
//! somebody decides who computes it.
//!
//! **No clamps.** `karakuri-cli` clamps a gain before it asks, not after, and
//! that is where a clamp belongs: `Operation::SetGain`'s `gain` is open on
//! purpose — *"Not clamped at 1.0 — the mix is HDR"* — so a conversion that
//! clamped would be a second opinion about a range the surface already has.

use karakuri_operation::Operation;
use karakuri_store::record::Record;

/// **The output look that is running**, which is what
/// [`Operation::SetTonemap`] and [`Operation::SetExposure`] each need the
/// other two thirds of.
///
/// The vocabulary's [`karakuri_operation::Tonemap`] rather than a wire name,
/// because a reading is read back from a running engine by a harness that
/// already owns one translation per list, and a `String` here would make this
/// crate the place a typo arrives.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Look {
    pub tonemap: karakuri_operation::Tonemap,
    pub exposure: f32,
    /// In [`Record::Look`] and on no surface — see
    /// `karakuri_operation::Operation::SetExposure`. Carried here because the
    /// record is written whole and a conversion that dropped it would rewrite
    /// a value nobody asked about.
    pub white_point: f32,
}

/// **The mask a deck's layer is wearing**, which is what
/// [`Operation::SetMaskShape`] and [`Operation::SetMaskPosition`] each need
/// the other half of.
///
/// [`Record::Mask`] is written whole — a shape, an angle, a position and a
/// softness — and each of the two operations asks for a part of it, which is
/// [`Look`]'s arrangement exactly and for ADR-0192's reason: a record is what a
/// replay reconstructs a session from, an operation is what a surface can say.
/// A shape written without the position beside it would send the front back to
/// wherever a default put it, mid-wipe.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mask {
    pub kind: karakuri_operation::WipeKind,
    /// Which way a linear front runs, in radians.
    pub angle: f32,
    /// How far it has travelled, `[0, 1]`.
    pub position: f32,
    /// **In [`Record::Mask`] and on no surface** — one constant in
    /// `karakuri-cli`, which says of itself that it is not a key. Carried here
    /// for [`Look::white_point`]'s reason: the record is written whole and a
    /// conversion that dropped it would rewrite a value nobody asked about.
    pub softness: f32,
}

/// **What one deck's clock is doing**, which is what
/// [`Operation::ScrubDeck`] needs to say where it moved *to*.
///
/// All three, because [`Record::Transport`] carries all three and is written
/// whole: a scrub that wrote an offset without the sync mode and the anchor
/// beside it would replay a slot onto a grid it was never on.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transport {
    pub sync: karakuri_operation::Sync,
    pub anchor_bpm: f32,
    /// Signed and unbounded — the one value in this format meant to go
    /// backwards.
    pub offset_beats: f64,
}

/// **What is running, at the instant the operation arrives.**
///
/// A plain struct of values rather than a trait the caller implements, and the
/// reason is the survey rather than taste: part of what a record needs is
/// *not readable from anything*. The quantum, the length of a fade and the
/// wipe shape are `Operation::SetTransition`'s, and that operation **writes no
/// record at all** — it is a surface's own setting deciding what the *next*
/// move means. A trait over "the deck" could not answer for them; a value
/// handed in can, the day somebody decides whose they are.
///
/// **Every field is optional, and [`Current::default`] means *I read
/// nothing*.** That is load-bearing. A default `Look` would let
/// `SetExposure` write a tone map operator nobody chose — the exact failure
/// ADR-0192 rejected `cc 20 -> exposure aces` for, where *"nudging exposure
/// silently overwrites a tone map somebody chose with `t` a moment earlier"*.
/// A reading that is absent answers [`Owed::NotRead`] instead, which is a
/// sentence a caller can print rather than a record nobody asked for.
///
/// **Whose transport.** [`Current::transport`] is the transport of the deck
/// *the operation names*, and no other: every operation here that needs one
/// names exactly one deck, so a map from deck to transport would be a
/// container built to be indexed once.
/// [`Current::mask`] is the mask of that same deck, on the same terms.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Current {
    pub look: Option<Look>,
    pub transport: Option<Transport>,
    pub mask: Option<Mask>,
}

/// Which reading an [`Owed::NotRead`] wanted, so a caller can say what it did
/// not hand over rather than that something was missing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reading {
    /// [`Current::look`].
    Look,
    /// [`Current::transport`], for the deck the operation names.
    Transport,
    /// [`Current::mask`], for the deck the operation names.
    Mask,
}

/// **Why an operation writes no record**, and there are four different
/// reasons — which is why this is a type and not a `None`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Silent {
    /// **A surface's own state.** Which deck the keys are addressed to, what
    /// is folded, what a window is sized to, what the *next* fade will mean.
    /// `Operation::SelectDeck` says of itself that it writes no record, and it
    /// is the reason every other operation names its deck rather than meaning
    /// the selected one.
    Surface,
    /// **It asks rather than changes.** Reading a Set, listing the store,
    /// reading a procedure, finding out what a write did. A record is what a
    /// replay reconstructs a performance from and a question changes no
    /// performance.
    Question,
    /// **The record is written where the work lands, not where it was asked
    /// for.** `Record::Save` is written at the frame the save landed because a
    /// record written at the key press would claim a file the disk went on to
    /// refuse; `Record::Procedure` is written when a swap lands or is rolled
    /// back. The operation is the ask, and the ask is not the event.
    OnLanding,
    /// **Nothing in the session vocabulary carries it.** Two different shapes
    /// of nothing live here and both are gaps rather than decisions: a row the
    /// record format has never had — the latency offset, a beat source, a
    /// published interface, loading a Set into a running deck — and a row
    /// whose record is a **Set file's** and has nowhere to put a deck.
    /// `Operation::WriteParam` names a deck and [`Record::Param`] has no
    /// `slot`, because a Set does not know what fader it is under.
    NoRecord,
}

impl Silent {
    /// One sentence, for a caller that prints it.
    pub fn why(self) -> &'static str {
        match self {
            Silent::Surface => "it is a surface's own state and writes no record",
            Silent::Question => "it asks rather than changes, and a question writes no record",
            Silent::OnLanding => {
                "its record is written where the work lands, not where it was asked for"
            }
            Silent::NoRecord => "nothing in the session record vocabulary carries it",
        }
    }
}

/// **Why a record this operation owes cannot be made here**, and the three
/// reasons are three different questions for three different people.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Owed {
    /// **The reading it needs was not in the [`Current`] handed in.** The
    /// caller's to fix, and the only one of the three that is.
    NotRead(Reading),
    /// **The vocabulary itself says what this acts on is open** —
    /// `karakuri_operation::Undecided`, at four variants, each with its
    /// question written at its own definition. Nothing can be written down
    /// here that is not already decided there.
    Undecided,
    /// **Its record is not a function of values alone, and who supplies the
    /// rest is undecided.** Seven operations, and they divide cleanly:
    /// scheduling one (a fade, a crossfade, a wipe, a renderer selection)
    /// needs the grid's position quantised onto a musical instant, plus the
    /// quantum and the length that `Operation::SetTransition` sets and no
    /// record carries; moving the grid (a tap, an octave shift) needs the beat
    /// tracker rather than a value, and may be refused by it; and setting a
    /// sync mode needs a session tempo and the engine's anchor clamp, so
    /// whether the record carries what was asked or what was clamped is a
    /// decision about the bytes on disk.
    NotSettled,
}

impl Owed {
    /// One sentence, for a caller that prints it. **A gap said out loud**, not
    /// a failure: the sentence names the question rather than blaming the
    /// caller, except for [`Owed::NotRead`], which is the caller's.
    pub fn why(self) -> &'static str {
        match self {
            Owed::NotRead(Reading::Look) => "the look that is running was not read",
            Owed::NotRead(Reading::Transport) => {
                "the transport of the deck it names was not read"
            }
            Owed::NotRead(Reading::Mask) => "the mask of the deck it names was not read",
            Owed::Undecided => "what it acts on is an open question in the vocabulary itself",
            Owed::NotSettled => "the record it writes is not a function of values alone, and who supplies the rest is undecided",
        }
    }
}

/// **What one operation writes into the record stream.**
#[derive(Debug, Clone, PartialEq)]
pub enum Written {
    /// The records it writes, **in the order they must be written**, and never
    /// empty.
    ///
    /// A list rather than one record, although every conversion built today
    /// answers exactly one. `Operation::Crossfade` is four records and
    /// `Operation::Wipe` is six — five until the mask took a row for its
    /// shape and a row for its position, each of which writes a whole
    /// `Record::Mask` (ADR-0201) — that is what `karakuri-cli`'s `crossfade`
    /// and `wipe` already do, and it is the whole of what P-0028 claims: one
    /// control, however many records the deck needs to be told. Both are
    /// [`Owed::NotSettled`] for a different reason, and the shape of this
    /// answer is not the thing keeping them out.
    Records(Vec<Record>),
    /// It writes none, and that is settled. See [`Silent`].
    Silent(Silent),
    /// It writes one and this build cannot make it. See [`Owed`].
    ///
    /// **Not an error.** Nothing here failed; a question was met that nobody
    /// has answered.
    Owed(Owed),
}

/// **One operation, and what is running, as the records it writes.**
///
/// The match is exhaustive over `Operation` on purpose, in
/// `karakuri_store::record::Record::vocabulary`'s shape and for its reason: an
/// operation added to the vocabulary stops the build here until somebody says
/// what it writes, so the classification cannot drift the way a wildcard arm
/// would let it.
pub fn written(operation: &Operation, current: &Current) -> Written {
    match operation {
        // ----- What it writes, with no reading at all ----------------------
        //
        // Six, and every one of them is a control whose record carries exactly
        // what the operation carries. These are the arms `karakuri-cli`'s
        // `mix::gain_record` and its neighbours were, moved to where a console
        // can reach them.
        Operation::SetGain { deck, gain } => one(Record::Gain {
            slot: *deck,
            value: *gain,
        }),
        Operation::SetOpacity { deck, opacity } => one(Record::Opacity {
            slot: *deck,
            value: *opacity,
        }),
        // `Record::Blend` carries the mode as a `String` — *"what a mode is
        // allowed to be is the engine's to say"* — so this is where a named
        // destination becomes a wire name.
        Operation::SetBlendMode { deck, blend } => one(Record::Blend {
            slot: *deck,
            mode: blend.name().to_string(),
        }),
        // **The request, never the effective level.** The governor recomputes
        // the second every pass from the budget of the machine that is
        // running, which is why only the request is an operation and only the
        // request is a record.
        Operation::SetResidency { deck, residency } => one(Record::Residency {
            slot: *deck,
            level: residency.name().to_string(),
        }),
        Operation::SetPreview { showing } => one(Record::Preview { slot: *showing }),
        // **A free-running tempo being stated**, which is `Record::Tempo`'s own
        // words for a correction with no shift and no confidence: *"The first
        // one in a stream is what sets the session tempo … a correction with
        // `shift` 0.0 and `confidence` 0.0 is a free-running tempo being
        // stated."* P-0028 names `--bpm` as the flag with no record behind it;
        // this is the record it was waiting for, and nothing routes through it
        // yet.
        Operation::SetFreeRunTempo { bpm } => one(Record::Tempo {
            bpm: *bpm,
            shift: 0.0,
            confidence: 0.0,
        }),

        // ----- What it writes, given a reading ----------------------------
        //
        // Three, and each one names exactly what it reads. The look pair is
        // ADR-0192's whole argument made executable.
        Operation::SetTonemap { tonemap } => match current.look {
            Some(look) => one(Record::Look {
                op: tonemap.name().to_string(),
                exposure: look.exposure,
                white_point: look.white_point,
            }),
            None => Written::Owed(Owed::NotRead(Reading::Look)),
        },
        Operation::SetExposure { exposure } => match current.look {
            Some(look) => one(Record::Look {
                op: look.tonemap.name().to_string(),
                exposure: *exposure,
                white_point: look.white_point,
            }),
            None => Written::Owed(Owed::NotRead(Reading::Look)),
        },
        // **Half a mask each, and the record is whole.** The half that was
        // not asked for comes from the mask that is running, exactly as the
        // look pair fills in the two thirds a control change cannot say —
        // ADR-0192, one layer down. Which half is the ask is what decides
        // whether a running move is cancelled, and that is the deck's rule
        // rather than this crate's: `Record::Mask` carries a state and not an
        // ask, so nothing downstream of here can tell the two apart. See
        // `docs/principles/0078-the-operator-wins-and-an-automatic-writer-yields-to-a-hand.md`.
        Operation::SetMaskShape { deck, kind, angle } => match current.mask {
            Some(mask) => one(Record::Mask {
                slot: *deck,
                kind: kind.name().to_string(),
                angle: *angle,
                position: mask.position,
                softness: mask.softness,
            }),
            None => Written::Owed(Owed::NotRead(Reading::Mask)),
        },
        Operation::SetMaskPosition { deck, position } => match current.mask {
            Some(mask) => one(Record::Mask {
                slot: *deck,
                kind: mask.kind.name().to_string(),
                angle: mask.angle,
                position: *position,
                softness: mask.softness,
            }),
            None => Written::Owed(Owed::NotRead(Reading::Mask)),
        },
        // **Relative, and the one operation here that is** — nothing in the
        // instrument can set a position, so the record's absolute offset is
        // the one it is at plus the amount asked for. Which is exactly why it
        // needs a reading where `SetGain` does not.
        Operation::ScrubDeck { deck, beats } => match current.transport {
            Some(transport) => one(Record::Transport {
                slot: *deck,
                sync: transport.sync.name().to_string(),
                anchor_bpm: transport.anchor_bpm,
                offset_beats: transport.offset_beats + *beats,
            }),
            None => Written::Owed(Owed::NotRead(Reading::Transport)),
        },

        // ----- Owed: the grid, the tracker, and the anchor clamp -----------
        Operation::FadeDeck { .. }
        | Operation::Crossfade { .. }
        | Operation::Wipe { .. }
        | Operation::SelectRenderer { .. }
        | Operation::TapBeat
        | Operation::ScaleGrid { .. }
        | Operation::SetSync { .. } => Written::Owed(Owed::NotSettled),

        // ----- Owed: the vocabulary's own open questions -------------------
        Operation::MoveBoundary { .. }
        | Operation::WalkHistory { .. }
        | Operation::WatchFiles { .. }
        | Operation::RouteFrame { .. } => Written::Owed(Owed::Undecided),

        // ----- Silent: a surface's own state -------------------------------
        Operation::SelectDeck { .. }
        | Operation::SetTransition { .. }
        | Operation::FoldBay { .. }
        | Operation::FoldPane { .. }
        | Operation::Unfold { .. }
        | Operation::Solo { .. }
        // **The console's own arrangement, which is a surface's state in
        // exactly the sense this arm is for**: `Silent::Surface` names *what
        // is folded* among its examples, and a reset is every fold, every
        // divider and the solo at once. It is not `Silent::NoRecord` — that
        // arm is for state a *performance* has and the record vocabulary
        // cannot carry, and an arrangement is not something a replay
        // reconstructs anything from. It is not `Owed` either: nothing is
        // missing, because there is nothing to write.
        //
        // **A saved arrangement would not change this arm.** The record it
        // would need is the panel's own, not the session's — the same reason
        // `Operation::SizeWindow` is here beside it.
        | Operation::ResetArrangement
        | Operation::SizeWindow { .. } => Written::Silent(Silent::Surface),

        // ----- Silent: it asks rather than changes -------------------------
        Operation::ListSets { .. }
        | Operation::ReadSet { .. }
        | Operation::ReadProcedure { .. }
        | Operation::SwapOutcome => Written::Silent(Silent::Question),

        // ----- Silent: the record is written where the work lands ----------
        Operation::SaveSet { .. } | Operation::WriteProcedure { .. } => {
            Written::Silent(Silent::OnLanding)
        }

        // ----- Silent: nothing in the session vocabulary carries it --------
        //
        // `WriteParam`, `AttachSignal`, `WireInput` and `SetProperty` are the
        // four that are not simply absent: `Record::Param`, `Record::Bind`,
        // `Record::Edge`, `Record::Capacity`, `Record::Seed` and
        // `Record::Camera` all exist and are a **Set file's**, with no `slot`
        // to carry the deck the operation names. `SetCompositing` is the same
        // shape — `Record::Merge` is what a *Set* says about its layering, and
        // *"nothing in a stream says that the Set in slot 3 composites."*
        Operation::SetLatencyOffset { .. }
        | Operation::AttachBeatSource { .. }
        | Operation::LoadSet { .. }
        | Operation::SetCompositing { .. }
        | Operation::WriteParam { .. }
        | Operation::AttachSignal { .. }
        | Operation::TakeParamBack { .. }
        | Operation::WireInput { .. }
        | Operation::Publish { .. }
        | Operation::SetProperty { .. }
        | Operation::TransferSet { .. }
        | Operation::RecordSession { .. }
        | Operation::Quit => Written::Silent(Silent::NoRecord),
    }
}

/// The common answer: one record, and the list that carries it.
fn one(record: Record) -> Written {
    Written::Records(vec![record])
}

#[cfg(test)]
mod tests {
    use super::*;
    use karakuri_operation::{BlendMode, Residency, Sync, Tonemap};

    /// A look nothing else in these tests happens to be: an operator that is
    /// not the first in the list and a white point that is not the default, so
    /// a conversion filling either in from thin air is visible rather than
    /// coincidentally right.
    fn look() -> Look {
        Look {
            tonemap: Tonemap::AgX,
            exposure: 0.25,
            white_point: 4.0,
        }
    }

    fn records(written: Written) -> Vec<Record> {
        match written {
            Written::Records(records) => {
                assert!(
                    !records.is_empty(),
                    "`Records` is documented as never empty"
                );
                records
            }
            other => panic!("expected records, got {other:?}"),
        }
    }

    /// **The whole reason this is not a `From` impl.**
    ///
    /// `-`, `=` and a MIDI control change turn the exposure alone;
    /// `Record::Look` carries the operator and the white point beside it
    /// because a replay reconstructs a session from the record and *"a stream
    /// that set the exposure without saying which operator it applies to would
    /// be describing a look nobody can reconstruct"* (ADR-0192). The operator
    /// is **filled in from the look that is running**, and this is what says
    /// so.
    #[test]
    fn an_exposure_keeps_the_operator_that_is_running() {
        let current = Current {
            look: Some(look()),
            ..Current::default()
        };
        let written = written(&Operation::SetExposure { exposure: 2.5 }, &current);
        assert_eq!(
            records(written),
            vec![Record::Look {
                op: "agx".to_string(),
                exposure: 2.5,
                white_point: 4.0,
            }],
            "an exposure change rewrote the tone map operator or the white point — \
             the record carries all three and only the exposure was asked for"
        );
    }

    /// The other half, and it fails apart from the first: `t` names an
    /// operator and says nothing about the level going into it, so the
    /// exposure and the white point come from the reading.
    #[test]
    fn a_tone_map_keeps_the_exposure_that_is_running() {
        let current = Current {
            look: Some(look()),
            ..Current::default()
        };
        let written = written(
            &Operation::SetTonemap {
                tonemap: Tonemap::Reinhard,
            },
            &current,
        );
        assert_eq!(
            records(written),
            vec![Record::Look {
                op: "reinhard".to_string(),
                exposure: 0.25,
                white_point: 4.0,
            }],
            "a tone map change rewrote the exposure or the white point — the record \
             carries all three and only the operator was asked for"
        );
    }

    /// **A reading that was not taken is said, never defaulted.**
    ///
    /// This is the failure ADR-0192 rejected `cc 20 -> exposure aces` for:
    /// a conversion that filled the operator in from a default would have
    /// every exposure nudge silently overwrite a tone map somebody chose a
    /// moment earlier, sixty times a second. `Current::default()` means *I
    /// read nothing*, and the answer to it is a question rather than a record.
    #[test]
    fn a_reading_that_was_not_taken_is_owed_rather_than_guessed() {
        assert_eq!(
            written(
                &Operation::SetExposure { exposure: 2.5 },
                &Current::default()
            ),
            Written::Owed(Owed::NotRead(Reading::Look)),
            "an exposure with no look read came back with a record — which means the \
             operator in it was invented"
        );
        assert_eq!(
            written(
                &Operation::ScrubDeck {
                    deck: 1,
                    beats: 0.25
                },
                &Current::default()
            ),
            Written::Owed(Owed::NotRead(Reading::Transport)),
            "a scrub with no transport read came back with a record — which means the \
             offset it moved from was invented"
        );
    }

    /// A mask nothing else in these tests happens to be: a shape that is not
    /// the default, an angle nobody would reach for, a front part way across
    /// and a soft edge — so a conversion filling any of the three it was not
    /// asked for from thin air is visible rather than coincidentally right.
    fn mask() -> Mask {
        Mask {
            kind: karakuri_operation::WipeKind::Radial,
            angle: 1.25,
            position: 0.4,
            softness: 0.02,
        }
    }

    /// **The shape is asked for and the front is kept**, which is the mask
    /// half of ADR-0192's argument: `Record::Mask` is written whole and only
    /// the shape was asked for.
    ///
    /// A conversion that put the front back to a default here would send a
    /// running wipe to the start every time somebody chose a different shape.
    #[test]
    fn a_shape_keeps_the_front_where_it_is() {
        let current = Current {
            mask: Some(mask()),
            ..Current::default()
        };
        let written = written(
            &Operation::SetMaskShape {
                deck: 2,
                kind: karakuri_operation::WipeKind::Linear,
                angle: 0.0,
            },
            &current,
        );
        assert_eq!(
            records(written),
            vec![Record::Mask {
                slot: 2,
                kind: "linear".to_string(),
                angle: 0.0,
                position: 0.4,
                softness: 0.02,
            }],
            "choosing a shape moved the front or changed the soft edge — the record \
             carries all four and only the shape and its angle were asked for"
        );
    }

    /// The other half, and it fails apart from the first: a control change
    /// carries a position and says nothing about a shape, so the shape and the
    /// angle come from the reading.
    ///
    /// This is the row that exists **because** a control change can only set —
    /// a conversion that reset the shape here would turn every fader move into
    /// a layer silently unmasked.
    #[test]
    fn a_front_keeps_the_shape_that_is_running() {
        let current = Current {
            mask: Some(mask()),
            ..Current::default()
        };
        let written = written(
            &Operation::SetMaskPosition {
                deck: 2,
                position: 1.0,
            },
            &current,
        );
        assert_eq!(
            records(written),
            vec![Record::Mask {
                slot: 2,
                kind: "radial".to_string(),
                angle: 1.25,
                position: 1.0,
                softness: 0.02,
            }],
            "moving the front rewrote the shape, the angle or the soft edge — the record \
             carries all four and only the position was asked for"
        );
    }

    /// **A mask that was not read is said, never defaulted.**
    ///
    /// Its own test rather than a third assertion beside the look and the
    /// transport, because the failure it names is the mask's: a conversion
    /// that defaulted would answer `none` for a shape nobody chose, and a
    /// `Record::Mask` saying `none` takes the mask off the layer — so a
    /// caller that read nothing would not get a refusal, it would get a wipe
    /// silently undone.
    #[test]
    fn a_mask_that_was_not_read_is_owed_rather_than_defaulted() {
        assert_eq!(
            written(
                &Operation::SetMaskPosition {
                    deck: 1,
                    position: 0.5
                },
                &Current::default()
            ),
            Written::Owed(Owed::NotRead(Reading::Mask)),
            "a front moved with no mask read came back with a record — which means the \
             shape in it was invented"
        );
        assert_eq!(
            written(
                &Operation::SetMaskShape {
                    deck: 1,
                    kind: karakuri_operation::WipeKind::Radial,
                    angle: 0.0,
                },
                &Current::default()
            ),
            Written::Owed(Owed::NotRead(Reading::Mask)),
            "a shape chosen with no mask read came back with a record — which means the \
             front in it was invented"
        );
    }

    /// **A scrub moves from where the slot is**, which is why it is the one
    /// operation in the vocabulary that is relative: nothing in the instrument
    /// can set a position. The record is absolute, so the conversion is the
    /// addition.
    #[test]
    fn a_scrub_adds_to_the_offset_the_slot_is_at() {
        let current = Current {
            transport: Some(Transport {
                sync: Sync::Beat,
                anchor_bpm: 128.0,
                offset_beats: -1.5,
            }),
            ..Current::default()
        };
        let written = written(
            &Operation::ScrubDeck {
                deck: 2,
                beats: 0.25,
            },
            &current,
        );
        assert_eq!(
            records(written),
            vec![Record::Transport {
                slot: 2,
                sync: "beat".to_string(),
                anchor_bpm: 128.0,
                offset_beats: -1.25,
            }],
            "a scrub wrote somewhere other than where the slot was plus what was asked \
             for — an absolute record built from a relative operation has to read the \
             offset it is moving from"
        );
    }

    /// **Six operations need no reading at all**, and a caller that has none
    /// to give still gets its record. `karakuri-console`'s panel is exactly
    /// that caller: three faders, no engine, and `Current::default()`.
    #[test]
    fn the_faders_records_need_no_reading() {
        assert_eq!(
            records(written(
                &Operation::SetGain { deck: 3, gain: 2.0 },
                &Current::default()
            )),
            vec![Record::Gain {
                slot: 3,
                value: 2.0
            }]
        );
        assert_eq!(
            records(written(
                &Operation::SetOpacity {
                    deck: 1,
                    opacity: 0.5
                },
                &Current::default()
            )),
            vec![Record::Opacity {
                slot: 1,
                value: 0.5
            }]
        );
        assert_eq!(
            records(written(
                &Operation::SetBlendMode {
                    deck: 0,
                    blend: BlendMode::Over
                },
                &Current::default()
            )),
            vec![Record::Blend {
                slot: 0,
                mode: "over".to_string()
            }],
            "a fader whose record needed a reading would be a console control that \
             cannot be converted without an engine, which is the seam ADR-0185 opened"
        );
    }

    /// **The wire spellings, which are the cost the vocabulary pays.**
    ///
    /// `karakuri-operation` owns copies of lists `karakuri-engine` already
    /// holds, and a record carries the *name*: a mode, a level or an operator
    /// spelled differently here from the way the engine reads it back is a
    /// record that decodes to a refusal on replay and to nothing at all in the
    /// mix. The engine is not reachable from this crate, so this asserts the
    /// literals and `karakuri-cli` — the one crate that sees both lists — is
    /// where they are checked against the engine's own.
    #[test]
    fn a_record_carries_the_name_the_store_is_read_back_with() {
        assert_eq!(BlendMode::Add.name(), "add");
        assert_eq!(BlendMode::Over.name(), "over");
        assert_eq!(BlendMode::Max.name(), "max");
        assert_eq!(Residency::Live.name(), "live");
        assert_eq!(Residency::Priming.name(), "priming");
        assert_eq!(Residency::Allocated.name(), "allocated");
        assert_eq!(Sync::Free.name(), "free");
        assert_eq!(Sync::Tempo.name(), "tempo");
        assert_eq!(Sync::Beat.name(), "beat");
        assert_eq!(Tonemap::Clamp.name(), "clamp");
        assert_eq!(Tonemap::Reinhard.name(), "reinhard");
        assert_eq!(Tonemap::Aces.name(), "aces");
        assert_eq!(Tonemap::AgX.name(), "agx");
        assert_eq!(karakuri_operation::WipeKind::None.name(), "none");
        assert_eq!(karakuri_operation::WipeKind::Linear.name(), "linear");
        assert_eq!(karakuri_operation::WipeKind::Radial.name(), "radial");
    }

    /// **A free-running tempo being stated**, which closes the gap P-0028
    /// names: *"`--bpm` exists and the v0.2 vocabulary has no tempo record."*
    /// It has one now, and `Record::Tempo`'s own documentation says what shape
    /// a statement takes rather than a correction — no shift, no confidence.
    #[test]
    fn a_free_run_tempo_is_a_correction_that_corrects_nothing() {
        assert_eq!(
            records(written(
                &Operation::SetFreeRunTempo { bpm: 174.0 },
                &Current::default()
            )),
            vec![Record::Tempo {
                bpm: 174.0,
                shift: 0.0,
                confidence: 0.0,
            }],
            "a stated tempo carried a phase shift or a confidence — it is a statement \
             rather than an estimate, and a shift would move a beat nobody moved"
        );
    }

    /// **The three answers are three different things**, and a caller that
    /// collapsed them would tell an operator that folding a bay failed.
    #[test]
    fn nothing_written_is_never_the_same_as_nothing_decided() {
        assert_eq!(
            written(&Operation::SelectDeck { deck: 1 }, &Current::default()),
            Written::Silent(Silent::Surface),
            "selecting a deck is a surface's own state and settled — not a gap"
        );
        assert_eq!(
            written(
                &Operation::FadeDeck { deck: 1, to: 0.0 },
                &Current::default()
            ),
            Written::Owed(Owed::NotSettled),
            "a fade writes a `transition` record and cannot be written here — saying \
             it is silent would lose a fade an operator asked for"
        );
        assert_eq!(
            written(
                &Operation::WalkHistory {
                    step: karakuri_operation::Undecided
                },
                &Current::default()
            ),
            Written::Owed(Owed::Undecided),
            "walking the edit history is the vocabulary's own open question and not \
             this crate's"
        );
    }
}
