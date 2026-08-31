//! **Where an operation becomes a record.**
//!
//! [P-0028](../../../docs/principles/0028-every-control-ends-in-the-same-record.md)
//! is *every control ends in the same record*: a panel fader, a key press, a
//! mapped MIDI message and an MCP call are the same thing exactly because all
//! four write the same [`Record`] and the deck is moved by the decode. A
//! surface emits an [`Operation`] and applies nothing
//! ([ADR-0185](../../../docs/adr/0185-a-fader-translates-a-drag-into-an-operation-and-applies-nothing.md));
//! this is the step on the other side of that seam, and until it existed there
//! was nowhere for it to happen — the panel program, then
//! `karakuri-console/examples/panel.rs` and now `crates/karakuri/src/main.rs`,
//! built three records by hand and said so.
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
//! [`written`] is **one exhaustive match over every operation in the vocabulary**, which is
//! what makes the classification a fact rather than an intention: an operation
//! added to the vocabulary does not compile here until somebody has said what
//! it writes. The three answers are the three groups the survey found:
//!
//! - [`Written::Records`] — it writes these, in this order. Seventeen
//!   operations, eight of which need no reading at all.
//! - [`Written::Silent`] — it writes none, and that is settled. Thirty-two,
//!   for [`Silent`]'s four different reasons.
//! - [`Written::Owed`] — it writes one and this build cannot make it.
//!   Fifteen, for [`Owed`]'s three different reasons.
//!
//! **`Owed` is not a refusal and not an error.** It is a gap this crate
//! declares about itself, in the shape `karakuri_operation::Undecided` is: a
//! caller that meets one has met a question nobody has answered, and printing
//! it is more use than a silent no-op. Twelve of the fifteen are the
//! vocabulary's own `Undecided` rows, and eight of those twelve are the master
//! chain's three effects and the sequencer's five — two bays the manual
//! specifies and nothing holds.
//!
//! # What this crate deliberately cannot do
//!
//! **No engine.** Quantising a beat onto the grid is
//! `karakuri_engine::transition::quantise`, engaging a sync mode is
//! `karakuri_engine::transport::Transport::engaged`, and neither is reachable
//! from here — which is right, because a crate that pulled `wgpu` in would be
//! unreachable from every surface again. Anything an operation's record needs
//! that is arithmetic rather than a value has to arrive inside [`Current`], and
//! the three operations whose record needs the beat tracker or a mask no
//! operation names are [`Owed::NotSettled`] until somebody decides who supplies
//! the rest.
//!
//! **Scheduling a move was four of those six and three of the four are now a
//! reading.** A fade, a crossfade and a renderer selection each need the
//! instant they land on, the length and the shape, and none of those is
//! arithmetic once the instant is: `quantise` is computed where the operator
//! asked, once, and its answer arrives here as [`Current::transition`].
//! Nothing here divides by a quantum, which is the rule the tempo is read
//! under one paragraph down — the arithmetic stays in the crate that owns the
//! grid and what crosses the seam is its result.
//!
//! **The wipe is the fourth and is still owed**, which is the difference
//! between a gesture that is a scheduled move and one that only contains one:
//! `Operation::Wipe`'s six records include the shape the front takes, which is
//! `Operation::SetTransition`'s third setting, and the soft edge, which no
//! operation names at all. Those are a `Record::Mask`'s and reach a record
//! through `Operation::SetMaskShape`; what a wipe still owes is who says they
//! are its.
//!
//! **`Operation::SetSync` was the seventh and is now a reading**, which is what
//! that sentence looks like when it is paid. `Transport::engaged` decides what
//! engaging a mode means — the anchor is the session tempo and the scrub is
//! cleared — and the only part of it that is arithmetic is a clamp into
//! `karakuri_signal::oscillator::BPM_RANGE` that **cannot fire**: an
//! `Oscillator`'s tempo is written in exactly two places and both clamp into
//! that same range, so the tempo a session can report is already inside it and
//! the clamp is the identity on every one of them. What arrives inside
//! [`Current`] is therefore the tempo itself, unclamped and uncomputed, and the
//! agreement with the engine's policy is held by a test rather than by a
//! dependency — see [`Current::tempo`].
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
/// whole: a scrub that wrote a scrub position without the sync mode and the anchor
/// beside it would replay a slot onto a grid it was never on.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transport {
    pub sync: karakuri_operation::Sync,
    pub anchor_bpm: f32,
    /// Signed and unbounded — the one value in this format meant to go
    /// backwards.
    pub scrub_beats: f64,
}

/// **What the next scheduled move means**, which is what
/// [`Operation::FadeDeck`], [`Operation::Crossfade`] and
/// [`Operation::SelectRenderer`] each need and none of them carries.
///
/// **It is the surface's, and that is the decision this type is.** The quantum
/// a move starts on and the length it lasts are `Operation::SetTransition`'s,
/// which writes no record at all because it is a setting deciding what the
/// *next* fade means — *"These change nothing you can see and write nothing to
/// the stream"*. An operation says what it wants and never how it is
/// scheduled, so a fade carrying its own quantum would be two answers to one
/// question: the operator would set a length with `j` and a fade would arrive
/// with a different one, and nothing anywhere would say which of them was the
/// setting. What is scheduled is the operation's; when and how long is what
/// the surface it was asked on already decided, and this is that decision
/// handed over.
///
/// **This reading is a completion in [`Look`]'s sense, not a preference.**
/// [`Record::Transition`] is written whole — a slot, a control, a
/// destination, an instant, a length and a curve — and `Operation::FadeDeck`
/// carries two of the six. The other four are here for exactly ADR-0192's
/// reason: a stream that scheduled a fade without saying when it lands or how
/// long it takes would describe a move nobody can reconstruct.
///
/// **The wipe shape is deliberately not here**, although it is
/// `SetTransition`'s third setting. What this type holds is what
/// `Record::Transition` and [`Record::Select`] need and the operation does not
/// carry; a shape is a `Record::Mask`'s and reaches a record through
/// [`Operation::SetMaskShape`], which converts already. A field for it would
/// be a value no arm reads.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transition {
    /// **The musical instant the move lands on, in beats**, absolute on the
    /// session's beat count.
    ///
    /// Absolute and never relative, which is [`Record::Transition`]'s own
    /// rule: *"a relative instant is a different instant depending on when it
    /// is read and a beat count is the same one on every run"*. Quantising to
    /// the next bar happens where the operator asked, once —
    /// `karakuri_engine::transition::quantise` is the one grid there is and
    /// this crate cannot reach it, so what arrives here is that function's
    /// answer rather than the quantum it was asked with.
    ///
    /// **A caller with no opinion about the grid has one to give, and ASAP
    /// needed nothing invented.** `quantise` documents a quantum of 0 as
    /// *"now"* and hands the beat count back unchanged, so a surface that
    /// wants a cut passes the instant it is at and the move is due the moment
    /// it is read — `Transition::value_at` and `Selection::due` are both `>=`,
    /// so the instant itself belongs to the move. That is why this is one
    /// `f64` and not an `Option` or a sum with an `Asap` arm: the grid already
    /// spells *now*, and a second spelling of it here would be a value the
    /// engine would have to be taught to read back.
    pub start: f64,
    /// **How long the move lasts, in beats. Zero is a cut**, which is
    /// [`Record::Transition`]'s own reading of it and is what
    /// `karakuri-cli`'s length cycle offers as its fourth position.
    ///
    /// Read by a fade and a crossfade and **not by a selection**:
    /// `Record::Select` has no length and no curve, because *"half way to
    /// renderer 2" does not name a picture*. A selection reads [`start`] alone
    /// and the other two fields are beside the point of it rather than absent
    /// from it, which is why this is one reading and not two.
    ///
    /// [`start`]: Transition::start
    pub beats: f64,
    /// **The shape the move takes**, and the one field here that is not on any
    /// surface at all.
    ///
    /// No operation names a curve for a fade — `karakuri-cli` holds one
    /// constant and says of it that it is *"Not on a key"*, because the other
    /// three curves are for signals and a fade wants easing and nothing else.
    /// So it is carried here for [`Look::white_point`]'s reason exactly: the
    /// record is written whole, and a conversion that dropped it would have to
    /// invent a shape for a move somebody else chose.
    ///
    /// The vocabulary's [`karakuri_operation::Curve`] rather than a wire name,
    /// which is [`Look::tonemap`]'s rule — a `String` here would make this
    /// crate the place a typo arrives.
    pub curve: karakuri_operation::Curve,
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
/// **That day came, and [`Current::transition`] is what it looks like paid.**
/// A length is nothing a deck holds and nothing a deck can be asked for, which
/// is why the sentence above was written as a prediction; what settled it was
/// deciding *whose* the setting is rather than finding somewhere to read it
/// from. It is the surface's, it sits here beside `tempo`, `transport`, `look`
/// and `mask`, and the four operations that were owed a
/// scheduled move are three conversions and a wipe now.
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
/// [`Current::tempo`] and [`Current::transition`] are the two readings here
/// that belong to no deck at all — one is the room's and one is the surface's,
/// and a crossfade naming two decks reads the same transition for both halves
/// because that is what makes it one gesture.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Current {
    pub look: Option<Look>,
    pub transport: Option<Transport>,
    pub mask: Option<Mask>,
    /// **The tempo the room is going at**, in BPM, which is what
    /// [`Operation::SetSync`] anchors a slot to.
    ///
    /// Engaging a mode anchors the slot at the session tempo so that
    /// **engaging sync never moves the picture**: the material is at 1x at that
    /// instant and stays there until the room's tempo does.
    /// `karakuri_engine::transport::Transport::engaged` is where that is
    /// decided, in its own words *"so that a caller building a record of the
    /// change and a caller applying one agree by construction"*, and this is
    /// the one value it takes.
    ///
    /// **A tempo an oscillator reported, and not a number somebody had.**
    /// `Transport::engaged` clamps the anchor into
    /// `karakuri_signal::oscillator::BPM_RANGE`; a
    /// `karakuri_signal::oscillator::Oscillator` writes its tempo in exactly
    /// two places — construction and a correction — and both clamp into that
    /// same range, so the clamp is the identity on every tempo a session can
    /// report and the record carries the tempo verbatim. That is why nothing
    /// here clamps: a second clamp would be a second opinion about a range the
    /// oscillator already holds, which is this crate's rule about `SetGain` one
    /// control along.
    ///
    /// **It is a convention, and this is where it says so**
    /// ([P-0026](../../../docs/principles/0026-a-guarantee-is-structural-or-it-is-a-convention-that-says-so.md)).
    /// The structural half is the reading: `karakuri_environment::mix`'s
    /// `current_tempo` takes the oscillator rather than an `f32`, so a caller
    /// cannot hand in a tempo the range has never held without writing one
    /// down. The half that is not structural — that the anchor is this tempo
    /// and the scrub is zero — is held against `Transport::engaged` itself by
    /// `mix`'s `a_sync_mode_writes_exactly_what_the_engine_would_engage`, in
    /// the one crate that can see both.
    pub tempo: Option<f32>,
    /// **What the next scheduled move means**, which is what a fade, a
    /// crossfade and a renderer selection are each half of. See
    /// [`Transition`], which carries the whole argument for why it is the
    /// surface's and not the operation's.
    ///
    /// **It is a convention on [`Current::tempo`]'s terms, and the structural
    /// half is the same shape.** `karakuri_environment::mix`'s
    /// `current_transition` takes the session's oscillator and a quantum
    /// rather than a beat count, so a caller cannot hand in an instant the
    /// grid never had without writing one down — and a caller with no opinion
    /// about the grid gives a quantum of 0, which `quantise` answers with the
    /// beat it is on. What is not structural is that this is the *setting* the
    /// operator last chose rather than one the gesture invented, and that is
    /// each surface's to keep: there is nothing to read it back from, which is
    /// the whole reason it is handed in.
    pub transition: Option<Transition>,
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
    /// [`Current::tempo`] — the session's, and so one of the two that names no
    /// deck.
    Tempo,
    /// [`Current::transition`] — the surface's, and the other one. A caller
    /// that meets this forgot a setting it is already holding rather than a
    /// reading it would have had to take off the engine, which makes it the
    /// one of the five answered by remembering rather than by looking.
    Transition,
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
    /// `karakuri_operation::Undecided`, at twelve variants, each with its
    /// question written at its own definition. Nothing can be written down
    /// here that is not already decided there.
    ///
    /// **Eight of the twelve arrived together and are two whole bays**: the
    /// master chain's three effects and the sequencer's five, specified on
    /// the manual's page before anything holds a chain or a pattern. They are
    /// the largest thing this answer has ever been asked about at once, and
    /// they are the reason the arm below says which of them will owe a record
    /// and which nobody can yet say owes one.
    Undecided,
    /// **Its record is not a function of values alone, and who supplies the
    /// rest is undecided.** Three operations, and they divide into two shapes:
    /// moving the grid (a tap, an octave shift) needs the beat tracker rather
    /// than a value and may be refused by it; and a wipe needs the shape its
    /// front takes, which is `Operation::SetTransition`'s third setting, and
    /// the soft edge, which no operation names anywhere.
    ///
    /// **It was six, and the four that left went in two different ways.**
    /// Setting a sync mode was held here because the anchor goes through the
    /// engine's clamp, so whether the record carried what was asked for or
    /// what was clamped looked like a decision about the bytes on disk. It is
    /// not one: the two are the same number for every tempo an oscillator can
    /// report, and what the record carries was decided at `Transport::engaged`
    /// with the reason written at it. A fade, a crossfade and a renderer
    /// selection were held for a different question — *whose* the quantum and
    /// the length are — and that one had an answer rather than a computation
    /// behind it: they are the surface's. Both times what was actually missing
    /// was a reading; [`Current::tempo`] is the first and
    /// [`Current::transition`] is the second.
    ///
    /// **The wipe stayed, and that is the line between the two answers.** Its
    /// scheduled move is a fade's exactly; what it also carries is a mask, and
    /// nothing has said whether the shape `SetTransition` holds is the wipe's
    /// to write. Deciding that is one sentence and it is not this one.
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
            Owed::NotRead(Reading::Tempo) => "the session tempo its anchor comes from was not read",
            Owed::NotRead(Reading::Transition) => {
                "the transition settings its move is scheduled by were not handed over"
            }
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
    /// A list rather than one record, and **`Operation::Crossfade` is what it
    /// was built for**: four records — the incoming deck silenced, put on air,
    /// and the two scheduled moves — out of one press, which is the whole of
    /// what P-0028 claims. One control, however many records the deck needs to
    /// be told.
    ///
    /// It was written before anything answered more than one, against the day
    /// a gesture would, and the prediction is what held: nothing about this
    /// shape changed when the crossfade landed. `Operation::Wipe` is six for
    /// the same reason — five until the mask took a row for its shape and a
    /// row for its position, each of which writes a whole `Record::Mask`
    /// (ADR-0201) — and is still [`Owed::NotSettled`], which was never about
    /// the shape of this answer.
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
        // Eight, and every one of them is a control whose record carries
        // exactly what the operation carries. Six are the arms
        // `karakuri-cli`'s `mix::gain_record` and its neighbours were, moved to
        // where a console can reach them; the seventh, an authority, never had
        // one anywhere, and nor did the eighth.
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
        // **One value across, and it is the arm that says the master out is
        // not per slot.** Every reading in [`Current`] is a completion — the
        // two thirds of a look a press did not name, the half of a mask, the
        // position a scrub adds to — and there is nothing here to complete:
        // `Record::MasterOut` is one number and the operation carries it.
        // That is what puts this beside the faders rather than beside the
        // exposure, which is the control it is otherwise nearest
        // ([ADR-0224](../../../docs/adr/0224-out-and-exposure-are-two-levels-that-multiply-in-different-places.md)).
        //
        // **And no clamp**, which is this crate's rule at `SetGain` one arm
        // up: the level is floored at zero and open above 1.0, the engine's
        // `clamp_gain` is where that is decided, and a second opinion here
        // would be a range written down twice.
        Operation::SetMasterOut { out } => one(Record::MasterOut { value: *out }),
        // **A node address and a word, and nothing else** — which is what puts
        // this in the group that needs no reading, beside the faders rather
        // than beside the mask. `Record::Authority` is written whole by the
        // operation alone: there is no other half of it to fill in from what is
        // running, the way `Record::Look` and `Record::Mask` have one.
        //
        // The `layer` is the one thing that is translated, and
        // [`store_layer`] is where — this crate is the only one that sees both
        // spellings of the list.
        Operation::SetAuthority {
            deck,
            node,
            authority,
        } => one(Record::Authority {
            slot: *deck,
            layer: store_layer(node.layer),
            index: node.index,
            authority: authority.name().to_string(),
        }),
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
        // Four readings, and each operation names exactly which of them it
        // reads. The look pair is ADR-0192's whole argument made executable.
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
        // instrument can set a position, so the record's absolute scrub is
        // the one it is at plus the amount asked for. Which is exactly why it
        // needs a reading where `SetGain` does not.
        Operation::ScrubDeck { deck, beats } => match current.transport {
            Some(transport) => one(Record::Transport {
                slot: *deck,
                sync: transport.sync.name().to_string(),
                anchor_bpm: transport.anchor_bpm,
                scrub_beats: transport.scrub_beats + *beats,
            }),
            None => Written::Owed(Owed::NotRead(Reading::Transport)),
        },
        // **Absolute, and none of it comes from where the slot was.** Engaging
        // a mode anchors the slot at the session tempo and clears the scrub,
        // which is `karakuri_engine::transport::Transport::engaged`'s policy —
        // *"the record carries the anchor and the scrub explicitly, and this is
        // the one place that decides what they are when an operator engages a
        // mode by hand"* — and so the sibling arm above is the one this reads
        // *unlike*: a scrub adds to what the slot holds, and a mode replaces
        // all three. Reading the running transport here and keeping its scrub
        // would put a slot back on a position a song ago the moment it was
        // brought to the grid.
        //
        // **The anchor is the tempo, with no clamp and no arithmetic.** The
        // clamp `Transport::engaged` applies cannot fire on a tempo an
        // oscillator reported; [`Current::tempo`] carries the argument and the
        // test that holds it.
        //
        // **Nothing here reads a clock**, which is what makes this record
        // replayable at all
        // ([P-0002](../../../docs/principles/0002-simulation-time-comes-from-a-record-never-from-a-clock.md)):
        // the anchor is written down at the instant the operator asked, and
        // `Transport::set` puts it back without recomputing it from the machine
        // the replay is running on.
        Operation::SetSync { deck, sync } => match current.tempo {
            Some(bpm) => one(Record::Transport {
                slot: *deck,
                sync: sync.name().to_string(),
                anchor_bpm: bpm,
                scrub_beats: 0.0,
            }),
            None => Written::Owed(Owed::NotRead(Reading::Tempo)),
        },

        // ----- What it schedules, given the surface's settings -------------
        //
        // Three, and the reading they share is [`Current::transition`]: the
        // instant the move lands on, how long it lasts and the shape it
        // takes. None of the three is on the operation, because an operation
        // says what it wants and never how it is scheduled.
        //
        // **The fader and never the trim.** `FadeDeck` says of itself that it
        // is opacity only — *"a gain fade is in the record vocabulary and has
        // no control"* — because opacity is what silences a deck under every
        // blend mode, where a gain of zero under `over` is a black card that
        // still covers what is beneath it.
        Operation::FadeDeck { deck, to } => match current.transition {
            Some(transition) => one(fade(*deck, *to, transition)),
            None => Written::Owed(Owed::NotRead(Reading::Transition)),
        },
        // **One gesture, four records, and the order is the picture.**
        //
        // The incoming deck is silenced *first* and put on air second, and
        // both halves are load-bearing: a deck comes up at full opacity and
        // going off air does not lower it, so putting one on air without
        // silencing it first shows it at full immediately — up to a bar
        // before the fade it is supposed to arrive on, which is a cut with a
        // decorative fade attached. And a fade to something that is not being
        // composited is a fade to black, so it does have to go on air.
        //
        // **The put-on-air is unconditional**, which is the one place this
        // differs from the keyboard gesture it replaces: `karakuri-cli` wrote
        // the residency only when the deck was not already live. A record is
        // what a replay reconstructs a performance from, and a conversion that
        // skipped it would be reading a deck state this crate cannot see —
        // there is no residency in [`Current`] and adding one would be a
        // reading taken to omit a record rather than to write one.
        // `Operation::Crossfade` says four records at its own definition, and
        // four is what a replay gets.
        //
        // **Both halves read the same transition**, which is what makes them
        // one gesture without being one type: they share a start and a length,
        // and `karakuri_engine::transition` is where the argument for that
        // lives — the first-class thing is the move, and every gesture anyone
        // names is made of those.
        Operation::Crossfade { from, to } => match current.transition {
            Some(transition) => Written::Records(vec![
                Record::Opacity {
                    slot: *to,
                    value: 0.0,
                },
                Record::Residency {
                    slot: *to,
                    level: karakuri_operation::Residency::Live.name().to_string(),
                },
                fade(*from, 0.0, transition),
                fade(*to, 1.0, transition),
            ]),
            None => Written::Owed(Owed::NotRead(Reading::Transition)),
        },
        // **A choice and not a position, so it reads the instant and nothing
        // else.** `Record::Select` carries no length and no curve — *"half way
        // to renderer 2" does not name a picture* — and it is still the same
        // reading rather than a narrower one, because what a surface holds is
        // one setting: a selection scheduled from a quantum an operator set
        // with `n` lands where the fades land, which is the point of setting
        // it.
        Operation::SelectRenderer { deck, renderer } => match current.transition {
            Some(transition) => one(Record::Select {
                slot: *deck,
                renderer: *renderer,
                start: transition.start,
            }),
            None => Written::Owed(Owed::NotRead(Reading::Transition)),
        },

        // ----- Owed: the tracker, and a mask nobody has assigned -----------
        //
        // **A wipe is here and its three neighbours are not**, which is the
        // line [`Owed::NotSettled`] draws: its scheduled move is a fade's
        // exactly, and the five records around it include a shape that is
        // `Operation::SetTransition`'s and a soft edge no operation names.
        Operation::Wipe { .. }
        | Operation::TapBeat
        | Operation::ScaleGrid { .. } => Written::Owed(Owed::NotSettled),

        // ----- Owed: the vocabulary's own open questions -------------------
        //
        // **Every operation whose payload is `karakuri_operation::Undecided`
        // is here, and `Operation::MoveBoundary` is why that is a rule rather
        // than a coincidence.** A divider position is as plainly a surface's
        // own state as the four folds it sits beside on the page, and it is
        // still *here* instead of in `Silent::Surface`: while the vocabulary
        // says what an operation acts on is an open question, what it writes
        // cannot be answered either, and this is the arm that says so out
        // loud.
        Operation::MoveBoundary { .. }
        | Operation::WalkHistory { .. }
        | Operation::WatchFiles { .. }
        | Operation::RouteFrame { .. }
        // **The master chain's three effects will owe a record**, which is
        // why calling them silent would be wrong rather than merely early:
        // ADR-0227 keeps a chain's *levels while it is being played* in the
        // session stream — *"the same way a Set's gain does"* — and
        // `Record::MasterOut` says of itself that *"there is nothing else
        // about the master chain a stream can say yet … it grows the day an
        // effect lands in the chain."* Nothing can be written here because no
        // effect and no parameter of one exists to name, and that record
        // declines to invent them for the same reason this arm declines to:
        // *"a record kept for a thing that does not exist would be inventing
        // its contents."*
        | Operation::SetFeedback { .. }
        | Operation::SetBloom { .. }
        | Operation::SetRgbShift { .. }
        // **The sequencer's five are open one place further out**, and land
        // here for a different reason. ADR-0227 refuses the session stream a
        // *pattern* — a lane is a fifth route (ADR-0222), so what a pattern
        // does already lands as `Record::Opacity` and its kin, sixteen a bar,
        // and a pattern record beside them would be *"the cause written down
        // next to every one of its consequences."* That is an argument about
        // a pattern's contents rather than about what editing one writes, and
        // neither `Silent` arm can carry the difference today: `Surface` would
        // call a pattern the console's own state, where ADR-0227 makes it
        // library data under the store on a Set's and an arrangement's terms;
        // and `NoRecord` would call the stream's silence a settled gap, where
        // nothing about these has been settled at all. So they wait on a
        // pattern here, which is exactly the sentence `Owed::Undecided`
        // prints.
        | Operation::SetStep { .. }
        | Operation::SetLaneMute { .. }
        | Operation::PointLane { .. }
        | Operation::SetPatternGrid { .. }
        | Operation::SelectPattern { .. } => Written::Owed(Owed::Undecided),

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
        // **A saved arrangement did not change this arm**, which was written
        // as a prediction before the other two members existed and is kept
        // because it held. The record such a family needs is the panel's own,
        // not the session's — the same reason `Operation::SizeWindow` is here
        // beside it.
        | Operation::ResetArrangement
        // **A save writes a file and no record, and those are two different
        // kinds of nothing.** This arm answers what goes into the *session
        // stream*, and an arrangement deliberately does not: ADR-0221 put it
        // in a fourth place under the store —
        // `arrangements/<name>.arrangement.json` — precisely **because** it is
        // not something a replay reconstructs anything from, and a session
        // replayed on a different window would otherwise arrive carrying
        // somebody else's panel. So the disk behind these two is beside the
        // point of this answer rather than an argument against it.
        //
        // **Not `Silent::OnLanding`, and that is the arm to think about.**
        // `Operation::SaveSet` is there, and it looks like the same shape: an
        // ask, a disk, a write that may be refused. The difference is that a
        // Set save *has* a record — `Record::Save` — and the only question
        // that arm answers is *when* it is written, which is at the landing
        // rather than at the press. Nothing in the session vocabulary is an
        // arrangement, so there is no record here whose timing could be at
        // issue; putting these two in `OnLanding` would promise a caller a
        // record that arrives later, and none ever arrives.
        //
        // **Not `Silent::NoRecord` either**, which is where an operation goes
        // when the record vocabulary has no row for what it does and that is a
        // gap. This is the opposite: the vocabulary having no row for an
        // arrangement is the decision (ADR-0221 §2, and ADR-0208 §4 before
        // it), taken against `Record`-in-the-stream by name. A gap is
        // something somebody still owes; this is settled, which is what
        // `Silent::Surface` says and `NoRecord` would deny.
        //
        // **Restoring is the same answer for the same reason**, and it is the
        // one that makes the reading obvious: it moves every fold, every
        // divider and the solo at once, which is `ResetArrangement`'s own
        // sentence with a name in it.
        //
        // **Choosing a scope is the same answer arrived at from the other
        // side.** It changes which library the bay is reading and nothing
        // about what any deck is playing, so a replay that reconstructed it
        // would be putting somebody else's browsing on the screen — which is
        // the arrangement's argument again. And it is settled rather than
        // owed: `docs/manual/operations.html` says the scopes are *"the one
        // thing about the library that is not closed"*, so there is no record
        // shape a stream could hold for it that would not have to grow every
        // time an operator adds a directory. `Undecided` is on the payload and
        // not on this answer — what a scope is *named* by is open, and that
        // nothing in the stream names one is not.
        | Operation::SaveArrangement { .. }
        | Operation::RestoreArrangement { .. }
        | Operation::SelectScope { .. }
        | Operation::SizeWindow { .. } => Written::Silent(Silent::Surface),

        // ----- Silent: it asks rather than changes -------------------------
        Operation::ListSets { .. }
        | Operation::ReadSet { .. }
        | Operation::ReadProcedure { .. }
        | Operation::SwapOutcome => Written::Silent(Silent::Question),

        // ----- Silent: the record is written where the work lands ----------
        //
        // `RestoreProcedure` is beside `WriteProcedure` because it **is** one:
        // putting a node's previous version back is the same check, the same
        // worker and the same frame boundary, so the `Record::Procedure` is
        // written at the swap and can itself be rolled back. What it restores
        // is the file rather than the picture, which is the gap the staging
        // lane exists to show — a rolled-back build leaves the previous *Set*
        // on screen and the over-budget *file* on disk, and nothing else in
        // the instrument says so.
        Operation::SaveSet { .. }
        | Operation::WriteProcedure { .. }
        | Operation::RestoreProcedure { .. } => Written::Silent(Silent::OnLanding),

        // ----- Silent: the surface holds the state ------------------------
        //
        // **Keeping a candidate writes nothing because nothing is pending.**
        // The material already changed: a write lands, `install_if_ready` puts
        // a finished build in at the next frame boundary, and the
        // `Record::Procedure` for it was written at that swap. What a keep
        // settles is the *lane* — this node is no longer one an operator has
        // still to look at — and that is a surface's own state in exactly the
        // sense the four *Arranging the console* rows are.
        Operation::KeepCandidate { .. } => Written::Silent(Silent::Surface),

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

/// **One scheduled move on a deck's fader**, which is a fade and is half of a
/// crossfade.
///
/// A function rather than three copies, for the reason a crossfade is two
/// calls to it: the two halves have to agree about the start, the length and
/// the shape or they are not one gesture, and a second spelling of this record
/// is exactly the drift this crate exists to end.
fn fade(slot: u8, to: f32, transition: Transition) -> Record {
    Record::Transition {
        slot,
        control: OPACITY.to_string(),
        to,
        start: transition.start,
        beats: transition.beats,
        curve: transition.curve.name().to_string(),
    }
}

/// **The wire name of the control a fade moves**, and the one spelling in this
/// crate with no list of its own behind it.
///
/// `Record::Transition`'s `control` is `gain`, `opacity` or `mask`, and that
/// list is `karakuri_engine::transition::Control` — the engine's, and
/// unreachable from here. The vocabulary does not own a copy because no
/// operation names a control: `Operation::FadeDeck` *is* the opacity one and
/// says so at its own definition, and the gain fade the record vocabulary
/// allows has no operation at all. So there is nothing here for a match to be
/// exhaustive over, and what stands in for one is a test in
/// `karakuri-environment`'s `mix` — the one place that sees this literal and
/// the engine's list at once, exactly as the mode and level names one group up
/// are checked there.
const OPACITY: &str = "opacity";

/// **The vocabulary's layer as the store's**, which is the one list this crate
/// has to translate between rather than carry.
///
/// A function and not a `From` impl, for the reason `karakuri-cli`'s
/// `mix::blend_mode` and its neighbours are functions: both types are foreign
/// here, so the orphan rule forbids the impl outright
/// (`docs/adr/0194-…`). A match rather than a cast, for
/// `karakuri_operation::BlendMode::name`'s reason — a kind added to either list
/// does not compile until somebody says what it is on the other side, which is
/// the guarantee the two copies of this list are kept honest by.
///
/// **This crate is where it belongs.** It is the only place in the workspace
/// that sees `karakuri_operation::Layer` and `karakuri_store::record::Layer` at
/// once; `karakuri-cli`'s `mcp.rs` translates between the vocabulary's and
/// `karakuri_ir::Kind`, which is a third spelling and a different pair.
fn store_layer(layer: karakuri_operation::Layer) -> karakuri_store::record::Layer {
    use karakuri_operation::Layer as From;
    use karakuri_store::record::Layer as To;
    match layer {
        From::L1 => To::L1,
        From::L2 => To::L2,
        From::L3 => To::L3,
        From::L4 => To::L4,
        From::Field => To::Field,
    }
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
             scrub it moved from was invented"
        );
    }

    /// **The master out is a function of the operation and nothing else**, and
    /// this is the property that keeps it out of the group above.
    ///
    /// It is the arm most likely to be written as a completion by whoever adds
    /// the second thing to the master chain: it sits between the look pair and
    /// the mask pair in every list, and both of those are records written
    /// whole out of an operation that names a part of one. `Record::MasterOut`
    /// carries one number and the operation carries it, so a reading here
    /// would be a value nobody asked about — and a `Current::default()` that
    /// answered `Owed` would make the console's only route to this level a
    /// question printed instead of a level moved.
    ///
    /// **And nothing is clamped**, which is this crate's rule at `SetGain`:
    /// `Deck::set_out` floors at zero and is deliberately open above 1.0
    /// because the mix is HDR, so a level of 3.0 arrives on disk as 3.0 and
    /// the engine is the one place that range is decided.
    #[test]
    fn a_master_out_is_written_from_the_operation_alone() {
        assert_eq!(
            records(written(
                &Operation::SetMasterOut { out: 0.25 },
                &Current::default()
            )),
            vec![Record::MasterOut { value: 0.25 }],
            "the master out asked for a reading, or wrote something other than the level it \
             was handed — it names no deck and completes no record, so `Current::default()` \
             is everything it needs"
        );
        // The look that is running is beside the point rather than absent, so
        // a conversion that had started reading one would be caught writing a
        // different record here as well as the same one above.
        let current = Current {
            look: Some(look()),
            ..Current::default()
        };
        assert_eq!(
            records(written(&Operation::SetMasterOut { out: 3.0 }, &current)),
            vec![Record::MasterOut { value: 3.0 }],
            "a master out of 3.0 was clamped, or the look that is running reached the record \
             — the level is open above 1.0 and the engine is where that is decided"
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
    fn a_scrub_adds_to_the_scrub_the_slot_is_at() {
        let current = Current {
            transport: Some(Transport {
                sync: Sync::Beat,
                anchor_bpm: 128.0,
                scrub_beats: -1.5,
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
                scrub_beats: -1.25,
            }],
            "a scrub wrote somewhere other than where the slot was plus what was asked \
             for — an absolute record built from a relative operation has to read the \
             scrub it is moving from"
        );
    }

    /// **Engaging a mode is the sibling of a scrub and reads nothing of the
    /// slot**, which is what this asserts by giving it a slot to read.
    ///
    /// `ScrubDeck` above adds to the position the deck holds; `SetSync`
    /// replaces the whole transport, because that is what
    /// `karakuri_engine::transport::Transport::engaged` decides engaging a
    /// mode *means* — anchor at the session tempo, scrub cleared, *"a slot
    /// brought back to the grid should be on the grid, not on wherever it was
    /// scrubbed to a song ago"*. So a reading of a slot that is at -1.5 beats
    /// against an anchor of 128 must not leak into a record written at 126.
    #[test]
    fn engaging_a_mode_anchors_at_the_session_tempo_and_clears_the_scrub() {
        let current = Current {
            tempo: Some(126.0),
            // Deliberately present, deliberately different, and deliberately
            // ignored.
            transport: Some(Transport {
                sync: Sync::Free,
                anchor_bpm: 128.0,
                scrub_beats: -1.5,
            }),
            ..Current::default()
        };
        let written = written(
            &Operation::SetSync {
                deck: 2,
                sync: Sync::Beat,
            },
            &current,
        );
        assert_eq!(
            records(written),
            vec![Record::Transport {
                slot: 2,
                sync: "beat".to_string(),
                anchor_bpm: 126.0,
                scrub_beats: 0.0,
            }],
            "engaging a mode wrote something other than the session tempo as the anchor \
             with the scrub cleared — either the tempo was not what the slot was anchored \
             to, or the position it was scrubbed to survived being brought to the grid"
        );
    }

    /// **The tempo is a reading and not a default**, on
    /// [`a_reading_that_was_not_taken_is_owed_rather_than_guessed`]'s terms
    /// exactly: a conversion that anchored at 120 because that is a common
    /// tempo would put a slot on a grid the room was never on, silently, and
    /// a replay would reproduce it faithfully.
    #[test]
    fn a_sync_mode_with_no_tempo_read_is_owed_rather_than_anchored_at_a_guess() {
        assert_eq!(
            written(
                &Operation::SetSync {
                    deck: 0,
                    sync: Sync::Tempo
                },
                &Current::default()
            ),
            Written::Owed(Owed::NotRead(Reading::Tempo)),
            "a sync mode with no session tempo read came back with a record, which means \
             the tempo it anchored at was invented"
        );
    }

    /// **Seven operations need no reading at all**, and a caller that has none
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

    /// **An authority converts with no reading, and carries the node it names.**
    ///
    /// Its own test rather than a fourth assertion in
    /// [`the_faders_records_need_no_reading`], because what it pins is not the
    /// absence of a reading but the **address**: this is the only conversion
    /// here that turns a `NodeAt` into a record's `(layer, index)`, and
    /// `store_layer` is a second spelling of a list that has to stay in step.
    /// A conversion that dropped the index would put every renderer's authority
    /// on the first one; one that mistranslated the layer would put an L4's on
    /// an L1.
    #[test]
    fn an_authority_carries_the_node_it_names_and_needs_no_reading() {
        assert_eq!(
            records(written(
                &Operation::SetAuthority {
                    deck: 2,
                    node: karakuri_operation::NodeAt {
                        layer: karakuri_operation::Layer::L4,
                        index: 1,
                    },
                    authority: karakuri_operation::Authority::Suggesting,
                },
                &Current::default()
            )),
            vec![Record::Authority {
                slot: 2,
                layer: karakuri_store::record::Layer::L4,
                index: 1,
                authority: "suggesting".to_string(),
            }],
            "an authority landed on another node, another deck slot or another \
             level than the one it named"
        );

        // A `kind Field` node takes one too, which is the arm most easily lost
        // in a translation: it addresses no node in the rendering sense and its
        // params are still an operator's to ride.
        assert_eq!(
            records(written(
                &Operation::SetAuthority {
                    deck: 0,
                    node: karakuri_operation::NodeAt {
                        layer: karakuri_operation::Layer::Field,
                        index: 0,
                    },
                    authority: karakuri_operation::Authority::Manual,
                },
                &Current::default()
            )),
            vec![Record::Authority {
                slot: 0,
                layer: karakuri_store::record::Layer::Field,
                index: 0,
                authority: "manual".to_string(),
            }]
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
        assert_eq!(karakuri_operation::Authority::Manual.name(), "manual");
        assert_eq!(
            karakuri_operation::Authority::Suggesting.name(),
            "suggesting"
        );
        assert_eq!(karakuri_operation::Authority::Automatic.name(), "automatic");
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

    /// The transition settings nothing else in these tests happens to be: an
    /// instant that is not zero and not a whole bar, a length that is not the
    /// default and a curve that is not the first in the list — so a conversion
    /// filling any of the three in from thin air is visible rather than
    /// coincidentally right.
    fn transition() -> Transition {
        Transition {
            start: 37.0,
            beats: 6.0,
            curve: karakuri_operation::Curve::Smooth,
        }
    }

    /// **A fade lands on the instant and over the length the surface chose**,
    /// which is the whole of what settling this conversion decided: the
    /// quantum and the length are `Operation::SetTransition`'s, that operation
    /// writes no record, and they reach the stream through the reading rather
    /// than through the fade.
    ///
    /// **Opacity and never gain**, which is `Operation::FadeDeck`'s own
    /// sentence: a fade to zero has to silence the deck under every blend
    /// mode, and a gain of zero under `over` is a black card that still
    /// covers.
    #[test]
    fn a_fade_lands_on_the_instant_and_the_length_the_surface_chose() {
        let current = Current {
            transition: Some(transition()),
            ..Current::default()
        };
        assert_eq!(
            records(written(&Operation::FadeDeck { deck: 2, to: 0.0 }, &current)),
            vec![Record::Transition {
                slot: 2,
                control: "opacity".to_string(),
                to: 0.0,
                start: 37.0,
                beats: 6.0,
                curve: "smooth".to_string(),
            }],
            "a fade wrote a move on another control, at another instant, over another \
             length or in another shape than the settings it was handed — every one of \
             those four is the surface's and none of them is on the operation"
        );
    }

    /// **A crossfade is four records and both halves share the move.**
    ///
    /// The order is the picture and not a preference: the arriving deck is
    /// silenced *before* it is put on air, because a deck comes up at full
    /// opacity and going off air does not lower it — putting one on air first
    /// shows it at full immediately, up to a bar before the fade it is
    /// supposed to arrive on. And a fade to something that is not composited
    /// is a fade to black, so it does have to go on air.
    ///
    /// **The two moves share a start and a length**, which is what makes this
    /// one gesture without being one type. A conversion that read the settings
    /// twice could not be caught by an equality on one record; it is caught by
    /// asserting the pair.
    #[test]
    fn a_crossfade_is_four_records_and_both_halves_share_the_move() {
        let current = Current {
            transition: Some(transition()),
            ..Current::default()
        };
        assert_eq!(
            records(written(&Operation::Crossfade { from: 0, to: 1 }, &current)),
            vec![
                Record::Opacity {
                    slot: 1,
                    value: 0.0,
                },
                Record::Residency {
                    slot: 1,
                    level: "live".to_string(),
                },
                Record::Transition {
                    slot: 0,
                    control: "opacity".to_string(),
                    to: 0.0,
                    start: 37.0,
                    beats: 6.0,
                    curve: "smooth".to_string(),
                },
                Record::Transition {
                    slot: 1,
                    control: "opacity".to_string(),
                    to: 1.0,
                    start: 37.0,
                    beats: 6.0,
                    curve: "smooth".to_string(),
                },
            ],
            "a crossfade wrote something other than the four records its operation \
             names, in another order, or gave its two halves different instants — the \
             arriving deck is silenced before it is put on air, and the two moves are \
             one gesture exactly because they share a start and a length"
        );
    }

    /// **A selection is a cut, so it reads the instant and nothing else.**
    ///
    /// `Record::Select` has no length and no curve because *"half way to
    /// renderer 2" does not name a picture*, and this is what says the
    /// conversion agrees: the same operation against two readings that differ
    /// in every field but the start writes the same record.
    #[test]
    fn a_selection_is_a_cut_and_reads_the_instant_alone() {
        let select = Operation::SelectRenderer {
            deck: 3,
            renderer: 2,
        };
        let expected = vec![Record::Select {
            slot: 3,
            renderer: 2,
            start: 37.0,
        }];
        let current = Current {
            transition: Some(transition()),
            ..Current::default()
        };
        assert_eq!(
            records(written(&select, &current)),
            expected,
            "a selection landed on another renderer, another deck or another instant \
             than the one it was handed"
        );
        let other = Current {
            transition: Some(Transition {
                start: 37.0,
                beats: 0.0,
                curve: karakuri_operation::Curve::Lin,
            }),
            ..Current::default()
        };
        assert_eq!(
            records(written(&select, &other)),
            expected,
            "the length or the shape of a fade reached a selection's record — a \
             selection is a choice and a choice is a cut"
        );
    }

    /// **A cut is an instant that is now and a length of zero**, and both
    /// halves of it arrive rather than being invented.
    ///
    /// `karakuri_engine::transition::quantise` documents a quantum of 0 as
    /// *"now"* and hands the beat count straight back, so a surface asking for
    /// a cut has nothing to say that the grid does not already spell: the
    /// start is the beat the session is on. This crate never sees the quantum
    /// — that is `karakuri_environment::mix`'s
    /// `a_quantum_of_zero_starts_the_move_on_the_beat_it_was_asked_on`, in the
    /// crate that owns the grid — and what it must not do is round, floor or
    /// otherwise improve the instant it was handed.
    #[test]
    fn a_cut_is_the_instant_it_was_handed_and_a_length_of_zero() {
        let current = Current {
            // The beat count a session was at, unrounded on purpose: a
            // conversion that quantised anything would move it.
            transition: Some(Transition {
                start: 12.375,
                beats: 0.0,
                curve: karakuri_operation::Curve::Smooth,
            }),
            ..Current::default()
        };
        assert_eq!(
            records(written(&Operation::FadeDeck { deck: 1, to: 1.0 }, &current)),
            vec![Record::Transition {
                slot: 1,
                control: "opacity".to_string(),
                to: 1.0,
                start: 12.375,
                beats: 0.0,
                curve: "smooth".to_string(),
            }],
            "a cut was moved onto a grid or given a length — a quantum of 0 is `now`, \
             and an operator who asked for a cut is waiting for nothing"
        );
    }

    /// **A surface that handed in no settings is told which reading it
    /// forgot**, rather than getting a cut it did not ask for.
    ///
    /// This is [`a_reading_that_was_not_taken_is_owed_rather_than_guessed`] on
    /// the reading that is not read off anything: a default of zero would be a
    /// perfectly plausible `Transition` — a start of 0 is in the past and a
    /// length of 0 is a cut — so a surface that forgot its settings would get
    /// every fade as an instant jump and nothing anywhere would say so. All
    /// three are asserted, because the failure is the conversion's and not one
    /// operation's.
    #[test]
    fn a_surface_that_handed_in_no_settings_is_told_which_reading_it_forgot() {
        for operation in [
            Operation::FadeDeck { deck: 1, to: 0.0 },
            Operation::Crossfade { from: 0, to: 1 },
            Operation::SelectRenderer {
                deck: 0,
                renderer: 1,
            },
        ] {
            assert_eq!(
                written(&operation, &Current::default()),
                Written::Owed(Owed::NotRead(Reading::Transition)),
                "`{operation:?}` with no transition settings handed in came back with \
                 something other than the reading it is missing — a default here is a \
                 cut at beat zero, which is a move nobody asked for and a replay would \
                 reproduce faithfully"
            );
        }
        // And the sentence names the settings rather than something missing,
        // which is what `NotRead` is for.
        assert_ne!(
            Owed::NotRead(Reading::Transition).why(),
            Owed::NotSettled.why(),
            "a surface that forgot its settings is told the same thing as one that met \
             a question nobody has answered"
        );
    }

    /// **A file is not a record, and the whole arrangement family says so the
    /// same way.**
    ///
    /// Saving one writes `arrangements/<name>.arrangement.json` and nothing
    /// into the session stream, which is the answer that is easy to get wrong
    /// in two directions: `Silent::OnLanding` would promise a `Record` that
    /// arrives when the write lands and none ever does, and
    /// `Silent::NoRecord` would call the decision a gap. Asserted for all
    /// three members together, because what makes the answer right is that
    /// they are one family — a restore is a reset with a name in it.
    #[test]
    fn keeping_an_arrangement_writes_a_file_and_no_record() {
        for operation in [
            Operation::ResetArrangement,
            Operation::SaveArrangement {
                name: "four_deck".to_string(),
            },
            Operation::RestoreArrangement {
                name: "four_deck".to_string(),
            },
        ] {
            assert_eq!(
                written(&operation, &Current::default()),
                Written::Silent(Silent::Surface),
                "`{operation:?}` did not answer `Silent(Surface)`. An arrangement is the \
                 console's own state and lives in a fourth place under the store rather than \
                 in the session stream (ADR-0221) — a save writes a file, and a file is not a \
                 record whose timing `OnLanding` could be about, nor a gap `NoRecord` could be \
                 about"
            );
        }
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
            written(&Operation::Wipe { from: 0, to: 1 }, &Current::default()),
            Written::Owed(Owed::NotSettled),
            "a wipe writes six records and cannot be written here — saying it is silent \
             would lose a wipe an operator asked for"
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
