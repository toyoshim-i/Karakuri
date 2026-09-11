//! **Where an operation becomes a record.**
//!
//! [P-0090](../../../docs/principles/0090-a-surface-offers-it-never-decides.md)
//! puts every control at the same record: a panel fader, a key press, a
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
//! - [`Written::Records`] — it writes these, in this order. Eighteen
//!   operations, eight of which need no reading at all.
//! - [`Written::Silent`] — it writes none, and that is settled. Thirty-six,
//!   for [`Silent`]'s four different reasons.
//! - [`Written::Owed`] — it writes one and this build cannot make it. Nine,
//!   for [`Owed`]'s three different reasons.
//!
//! **`Owed` is not a refusal and not an error.** It is a gap this crate
//! declares about itself, in the shape `karakuri_operation::Undecided` is: a
//! caller that meets one has met a question nobody has answered, and printing
//! it is more use than a silent no-op. Four of the nine are the vocabulary's
//! own `Undecided` rows.
//!
//! **The sequencer's five left on 2026-09-09**, and they are the largest thing
//! this answer has stopped being asked about at once. They were `Owed` because
//! nothing held a pattern; a pattern is library data under the store on the
//! arrangement's terms now (ADR-0227, ADR-0320) and a lane's writes are its
//! record (ADR-0322), so editing one writes a file and no record — which is
//! the arrangement family's answer and is where they went, five rows down into
//! [`Silent::Surface`].
//!
//! # What this crate deliberately cannot do
//!
//! **No engine.** Quantising a beat onto the grid is
//! `karakuri_engine::transition::quantise`, engaging a sync mode is
//! `karakuri_engine::transport::Transport::engaged`, and neither is reachable
//! from here — which is right, because a crate that pulled `wgpu` in would be
//! unreachable from every surface again. Anything an operation's record needs
//! that is arithmetic rather than a value has to arrive inside [`Current`], and
//! the two operations whose record needs the beat tracker are
//! [`Owed::NotSettled`] until somebody decides who supplies the rest.
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
//! **The wipe was the fourth and is now a reading too**, and it took three
//! rather than one. Its scheduled move is a fade's exactly; what it also
//! carries is a mask, and the two values that mask needed had no owner. Both
//! have one now. The shape its front takes is the transition row's — it is
//! `Operation::SetTransition`'s third setting, the one the `z` key writes, and
//! it reaches the conversion inside [`Current::transition`] beside the instant
//! and the length that operation's other two settings are. The soft edge is
//! read off the mask already running on the deck being wiped in, which is what
//! this crate does for `Operation::SetMaskShape` already and is
//! [`Current::mask`]. No operation names a softness and none was added.
//!
//! **The third of the wipe's readings is taken so that two of its records can
//! be left out**, which is the only one on [`Current`] that works that way.
//! Where the deck arriving is already under `over`, or already live, a wipe
//! that wrote those records anyway would overwrite a blend mode the operator
//! chose — so [`Current::mix`] is what the arm asks before it writes them, and
//! `karakuri-cli`'s `c` no longer has to hold that decision itself.
//!
//! **The other answer was that the shape is the deck's**, written by
//! `Operation::SetMaskShape` and read back by a wipe like any other mask
//! value, and it lost on the company the setting keeps: the quantum and the
//! length are the surface's own state handed over in [`Current::transition`],
//! and there is no reason the third setting of one operation should travel a
//! different road from the other two. It would also have left the `z` key's
//! shape half with nothing to do.
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
use karakuri_store::record::{DeckSlot, Record};

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

/// **What the master chain is set to**, which is what each of
/// [`Operation::SetFeedback`], [`Operation::SetBloom`] and
/// [`Operation::SetRgbShift`] needs the other two thirds of.
///
/// [`Look`]'s arrangement exactly, and for ADR-0192's reason:
/// `Record::MasterChain` is written whole — the ordered list of slots — and
/// each of the three operations asks for one parameter of one slot. A bloom
/// amount written without the rest of the list beside it would describe a chain
/// a replay cannot put back.
///
/// **The list is the record's shape and the three fields above it are the
/// rows'.** `karakuri-console`'s Master bay still draws *Feedback*, *Bloom* and
/// *RGB shift*, so the three amounts are read back for it; what a press writes
/// is a slot of the list, found by the procedure's content address. The rows
/// retire in M5.16's second pass
/// (`docs/adr/0340-kind-l5-is-written-and-the-master-chain-is-an-ordered-list-of-them.md`
/// §7) and the three fields go with them.
///
/// The vocabulary's [`karakuri_operation::Cut`] rather than a wire word, for
/// [`Look::tonemap`]'s reason: a `String` here would make this crate the place
/// a typo arrives.
#[derive(Debug, Clone, PartialEq)]
pub struct Chain {
    /// How much of the retained frame comes back, and which frame that is.
    /// One field pair rather than two fields for
    /// `karakuri_operation::Feedback`'s reason.
    pub feedback: karakuri_operation::Feedback,
    pub bloom: f32,
    pub rgb_shift: f32,
    /// **The chain that is running**, in the record's own shape, so that a
    /// press on one row rewrites one slot and leaves every other where it is.
    pub slots: Vec<karakuri_store::record::ChainSlot>,
    /// **Where the three rows point.** See [`Shipped`].
    pub shipped: Shipped,
}

/// **The content address of each shipped procedure a Master bay row still
/// names**, so that a press on a row resolves to a slot of the list.
///
/// **It is handed in rather than computed here**, because an address is a hash
/// of bytes and this crate holds none: the host reads `examples/feedback.kir`
/// and the other two, puts them in the store when first used the way a Set's
/// sources are, and hands the three addresses down with the reading.
///
/// **A row naming a procedure the chain has not got appends one**, which is the
/// *for now* in ADR-0340 §7: with a list, a surface asking for *feedback* is
/// asking for a pass that may not be in the chain, and until the rows retire
/// the honest answer to *turn feedback up* is a chain with feedback in it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Shipped {
    pub feedback: String,
    pub bloom: String,
    pub rgb_shift: String,
}

impl Chain {
    /// **The list, with one slot's one parameter moved** — appending the slot
    /// where the chain has not got it. See [`Shipped`].
    ///
    /// `cut` is written only where the caller has one to write, which is
    /// feedback's row alone: a slot whose procedure declares no `retains` is
    /// refused a cut by the engine, so writing one would be writing a record
    /// this build refuses to obey.
    fn moved(
        &self,
        procedure: &str,
        cut: Option<&karakuri_operation::Cut>,
        key: &str,
        value: f32,
    ) -> Vec<karakuri_store::record::ChainSlot> {
        let mut slots = self.slots.clone();
        let at = match slots.iter().position(|s| s.procedure == procedure) {
            Some(at) => at,
            None => {
                slots.push(karakuri_store::record::ChainSlot {
                    procedure: procedure.to_string(),
                    cut: None,
                    params: Default::default(),
                });
                slots.len() - 1
            }
        };
        slots[at].params.insert(key.to_string(), value);
        if let Some(cut) = cut {
            slots[at].cut = Some(cut.name().to_string());
        }
        slots
    }
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
/// **The wipe shape is here, and the paragraph this replaces said it
/// deliberately was not.** That paragraph is worth keeping as the position it
/// was rather than deleting, because it was an argument and not an oversight:
/// what this type holds is what a record needs and the operation does not
/// carry, a shape is a `Record::Mask`'s, it reaches a record through
/// [`Operation::SetMaskShape`] which converts already — *"a field for it would
/// be a value no arm reads"*.
///
/// **What overturned it is the company the setting keeps.** The wipe shape is
/// `SetTransition`'s third setting, and the other two are already here: the
/// quantum [`start`] comes from and the length [`beats`] is. All three are one
/// operation's, that operation writes no record, and each of them decides what
/// the *next* move means — so there was never a reason for the third to travel
/// a different road from the first two, and sending it by
/// `Operation::SetMaskShape` would have made a deck's *current* mask the place
/// a surface's *next* setting was kept. The last sentence stopped being true
/// the moment [`Operation::Wipe`] read it: [`wipe_kind`] and [`wipe_angle`]
/// are what its front takes, and the arm reads them.
///
/// **The shape a deck is wearing is still a mask's**, which is the half of the
/// old position that survives whole. `Operation::SetMaskShape` writes that and
/// nothing here changes it; what is on this type is the shape the next wipe
/// will *make* it, which is a different value with a different owner. The soft
/// edge went the other way for the same reason — no operation names one, so a
/// wipe reads it off the mask that is running ([`Current::mask`]) rather than
/// carrying one here.
///
/// [`start`]: Transition::start
/// [`beats`]: Transition::beats
/// [`wipe_kind`]: Transition::wipe_kind
/// [`wipe_angle`]: Transition::wipe_angle
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
    /// **The shape a wipe's front takes**, and the third of
    /// [`karakuri_operation::TransitionSetting`]'s three settings — the one
    /// the `z` key writes, where [`start`] comes from `n`'s quantum and
    /// [`beats`] is `j`'s length.
    ///
    /// **Read by [`Operation::Wipe`] and by nothing else here**, which is what
    /// makes it the one field on this type that is not read by all three of
    /// the operations the other fields serve: a fade moves a fader and a
    /// selection is a cut, and neither has a front. It is beside them for the
    /// reason they are here at all — it is a *setting*, a surface's own state
    /// deciding what the next gesture means, and the operation that runs the
    /// gesture never carries one.
    ///
    /// **Not the shape a deck's mask is wearing**, which is the distinction
    /// [`Operation::SetMaskShape`] draws at its own definition: that operation
    /// names a deck and changes what its layer reaches *now*; this is what the
    /// next wipe will make it. The two are the same three values and different
    /// facts, and the wipe arm writes the second over the first.
    ///
    /// [`start`]: Transition::start
    /// [`beats`]: Transition::beats
    pub wipe_kind: karakuri_operation::WipeKind,
    /// **Which way that front runs, in radians.**
    ///
    /// Half of one setting rather than a setting of its own —
    /// `TransitionSetting::WipeShape` carries a kind and an angle together —
    /// because an angle is what makes one linear front a different picture
    /// from another, and the other two shapes ignore it exactly as a mask
    /// does. A surface that offers a curated list of shapes rather than a dial
    /// is choosing pairs out of this, which is
    /// [`karakuri_operation::WipeKind`]'s own note and a keyboard's
    /// compromise rather than the operation's.
    pub wipe_angle: f32,
}

/// **Where the deck a gesture arrives on already sits in the mix** — how it
/// meets what is under it, and whether it is in the picture at all.
///
/// **Read by [`Operation::Wipe`] and by nothing else here**, and it is the one
/// reading on this type taken so that a record can be left *out*. The others
/// complete a record the operation could not say whole — a tone map nobody
/// named, a soft edge no operation carries — and this one answers a question
/// the arm asks before it writes: *is the deck arriving already there?* A wipe
/// puts the deck it reveals under `over` and on air, and writing either at a
/// deck that is already there is where a choice the operator made gets written
/// over
/// ([P-0094](../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)).
///
/// **One reading and not two.** A blend mode and a residency are two
/// operations, two records and two rows of the manual, and they are one
/// reading because no caller can be holding one and not the other: they are
/// read off the same deck in the same breath — `Deck::blend(slot)` beside
/// `Deck::residency(slot)` — so a second [`Reading`] would name a state no
/// surface can be in, and the arm would have to pick an order between two
/// [`Owed::NotRead`] answers for one lookup.
///
/// The vocabulary's [`karakuri_operation::BlendMode`] and
/// [`karakuri_operation::Residency`] rather than wire names, which is
/// [`Look::tonemap`]'s rule one type up: a `String` here would make this crate
/// the place a typo arrives, and these two are compared rather than printed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mix {
    /// **How the deck meets what is under it**, as the deck reports it.
    ///
    /// What a wipe does with it is at the arm: `over` is written where this is
    /// [`karakuri_operation::BlendMode::Add`] — where a slot starts, which is
    /// `BlendMode::ALL`'s own sentence — and nothing is written where the
    /// operator has moved it, because a wipe under a mode they chose is a
    /// different picture and a legitimate one.
    pub blend: karakuri_operation::BlendMode,
    /// **Whether the deck is in the picture at all**, as the deck reports it
    /// rather than as it was asked for — the governor may hold a slot below
    /// the request, which is [`karakuri_operation::Residency`]'s own note, and
    /// what a wipe needs to know is whether the put-on-air it is about to
    /// write would change anything.
    pub residency: karakuri_operation::Residency,
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
/// [`Current::mask`] is the mask of that same deck, on the same terms — with
/// one operation that names two, and it names them in an order that answers
/// the question rather than raising it. [`Operation::Wipe`] carries a `from`
/// and a `to`, and everything it writes is written about the `to`: the deck
/// arriving is the one that wears the mask, so the mask handed in for a wipe
/// is that deck's and the `from` is read for nothing at all. A caller that
/// handed in the covered deck's mask would give the arriving deck somebody
/// else's soft edge, which is the one value of the four a wipe does not
/// otherwise say.
/// [`Current::mix`] is that same deck's on those same terms, and for the wipe
/// it is the arriving deck's for the arriving deck's reason: what the two
/// records it governs would change is the picture the wipe is putting on air.
/// [`Current::tempo`] and [`Current::transition`] are the two readings here
/// that belong to no deck at all — one is the room's and one is the surface's,
/// and a crossfade naming two decks reads the same transition for both halves
/// because that is what makes it one gesture.
/// **Not `Copy` since the chain became a list**, which is the one thing that
/// changed about this struct on 2026-09-10: [`Chain`] carries the slots the
/// record is written from, and a `Vec` cannot be copied. Every reading here is
/// still made where the press is answered and read once.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Current {
    pub look: Option<Look>,
    /// **What the master chain is running at**, which is what each of its
    /// three rows needs the other two thirds of. See [`Chain`], and
    /// [`Current::look`] for the shape.
    ///
    /// **It belongs to no deck**, which puts it beside `look` rather than
    /// beside `transport`: the chain reads what the fold produced.
    pub master_chain: Option<Chain>,
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
    /// (`docs/contributing.md` §4).
    /// The structural half is the reading: `karakuri_environment::mix`'s
    /// `current_tempo` takes the oscillator rather than an `f32`, so a caller
    /// cannot hand in a tempo the range has never held without writing one
    /// down. The half that is not structural — that the anchor is this tempo
    /// and the scrub is zero — is held against `Transport::engaged` itself by
    /// `mix`'s `a_sync_mode_writes_exactly_what_the_engine_would_engage`, in
    /// the one crate that can see both.
    pub tempo: Option<f32>,
    /// **What the next scheduled move means**, which is what a fade, a
    /// crossfade, a renderer selection and a wipe are each half of. See
    /// [`Transition`], which carries the whole argument for why it is the
    /// surface's and not the operation's — including the wipe shape, which is
    /// the fourth field, is read by the wipe alone, and is the one the type
    /// once said out loud it would never carry.
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
    /// **Where the deck arriving already sits in the mix**, and the one
    /// reading here taken so that a record can be left *out* rather than
    /// written. See [`Mix`]. [`Operation::Wipe`] is its only reader, and it is
    /// read for the deck [`Current::mask`] is read for — the one arriving,
    /// never the one covered.
    pub mix: Option<Mix>,
}

/// Which reading an [`Owed::NotRead`] wanted, so a caller can say what it did
/// not hand over rather than that something was missing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reading {
    /// [`Current::look`].
    Look,
    /// [`Current::master_chain`] — the master chain that is running, and the
    /// other reading here that names no deck.
    MasterChain,
    /// [`Current::transport`], for the deck the operation names.
    Transport,
    /// [`Current::mask`], for the deck the operation names — and for
    /// [`Operation::Wipe`], which names two, the deck arriving.
    Mask,
    /// [`Current::tempo`] — the session's, and so one of the two that names no
    /// deck.
    Tempo,
    /// [`Current::transition`] — the surface's, and the other one. A caller
    /// that meets this forgot a setting it is already holding rather than a
    /// reading it would have had to take off the engine, which makes it the
    /// one of the six answered by remembering rather than by looking.
    Transition,
    /// [`Current::mix`] — the blend mode and the residency of the deck the
    /// operation names, which for [`Operation::Wipe`] is the deck arriving.
    /// The one of the six a caller takes so that records can be left out
    /// rather than written, which is what makes it the one whose absence
    /// would not be visible in a record: a caller that forgot it and got a
    /// wipe anyway would get the two records omitting it exists to omit.
    Mix,
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
    ///
    /// **This arm's members are `gap` in the MCP column of
    /// `docs/manual/operations.html`, and the reason is this sentence read on
    /// the model's side**: a model has no window, so a route into a surface's
    /// own state is a route into a window the model is not looking at
    /// (`docs/adr/0315-a-model-has-no-window-so-the-twelve-surface-rows-mcp-badges-are-gap.md`).
    /// **That is not a loosening of what a write is.** They are writes, rule
    /// 01 still asks all four routes of a write, and what changed is one
    /// column's answer on twelve rows rather than the rule —
    /// `docs/adr/0281-…` names the loosening that would have made them
    /// optional per route and declines it. **This answer is not the reason
    /// either**: `Silent::Surface` says what a replay reconstructs, and a
    /// badge is a claim about a surface, so the two agree on the fact and are
    /// held apart on purpose.
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
    /// `Operation::SetProperty` names a deck and [`Record::Capacity`] has no
    /// `slot`, because a Set does not know what fader it is under.
    ///
    /// **`Operation::WriteParam` and `Operation::AttachSignal` were the two
    /// examples this paragraph used and neither is here now**, which is worth
    /// keeping because it says what this answer means. Nothing about a knob
    /// turn or an attachment changed: what changed is that [`Record::Ride`]
    /// and [`Record::Source`] were written, so the session vocabulary now
    /// carries both acts at a deck slot's address while [`Record::Param`] and
    /// [`Record::Bind`] go on being the Set file's. Every row left in this
    /// group is one record short in exactly that way, and none of them is a
    /// decision that the act writes nothing — see
    /// `docs/adr/0280-a-parameter-written-to-a-live-set-is-a-session-record.md`
    /// and
    /// `docs/adr/0319-an-attachment-is-a-session-record-and-taking-a-parameter-back-removes-it.md`.
    NoRecord,
    /// **Publishing is not the performance.** Where a frame *went* is not part
    /// of the frame, so switching an output on or off writes nothing — and
    /// this is a decision rather than a gap.
    ///
    /// Three things say it and none of them is about the record format.
    /// `docs/plugins.md` says a sink writes no record.
    /// `docs/adr/0171-the-deck-advances-and-each-sink-either-gets-the-frame-or-misses-it.md`
    /// says an operator who turns every output off has stopped the publishing
    /// and not the instrument — the deck advances either way, so two runs
    /// differing only in which sinks were on took the same steps and drew the
    /// same frames. And
    /// `docs/principles/0086-a-procedure-knows-only-what-it-declares.md` says
    /// the render size is not part of the picture, so an output's size is not
    /// part of it either: a replay on a machine with different outputs
    /// reproduces the same picture at its own sizes, and the session `canvas`
    /// record is what it draws at when nothing else says
    /// (`docs/adr/0246-the-render-size-belongs-to-the-output-and-the-sessions-canvas-is-only-its-default.md`).
    ///
    /// **Held apart from [`Silent::Surface`] on purpose.** An output is not a
    /// surface's own state — a projector window is not the console — and every
    /// member of that arm is `gap` in the MCP column because a model has no
    /// window, where a model may ask for an output once the *inputs and
    /// outputs* class is open. Filing this there would have made that sentence
    /// false and taken a route away from a caller that has one.
    Published,
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
            Silent::Published => {
                "it says where a frame goes, and where a frame goes is not part of the frame"
            }
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
    /// `karakuri_operation::Undecided`, at three variants, each with its
    /// question written at its own definition, and two of the three answer
    /// here: `Operation::SelectScope` is the third and is
    /// [`Silent::Surface`]'s, because a mark on a chip is a surface's own
    /// state whether or not the payload can say which chip. Nothing can be
    /// written down here that is not already decided there.
    ///
    /// **Eight of them once arrived together and were two whole bays**: the
    /// master chain's three effects and the sequencer's five, specified on the
    /// manual's page before anything held a chain or a pattern. They were the
    /// largest thing this answer was ever asked about at once, and both bays
    /// left on 2026-09-09 — the chain's three into a record they can now name,
    /// the sequencer's five into [`Silent::Surface`], because a pattern turned
    /// out to be library data under the store on the arrangement's terms and a
    /// lane's writes turned out to be its record.
    Undecided,
    /// **Its record is not a function of values alone, and who supplies the
    /// rest is undecided.** Two operations, and they are one shape: moving the
    /// grid — a tap, an octave shift — needs the beat tracker rather than a
    /// value, and may be refused by it.
    ///
    /// **It was six, and the four that left first went in two different
    /// ways.** Setting a sync mode was held here because the anchor goes
    /// through the engine's clamp, so whether the record carried what was
    /// asked for or what was clamped looked like a decision about the bytes on
    /// disk. It is not one: the two are the same number for every tempo an
    /// oscillator can report, and what the record carries was decided at
    /// `Transport::engaged` with the reason written at it. A fade, a crossfade
    /// and a renderer selection were held for a different question — *whose*
    /// the quantum and the length are — and that one had an answer rather than
    /// a computation behind it: they are the surface's. Both times what was
    /// actually missing was a reading; [`Current::tempo`] is the first and
    /// [`Current::transition`] is the second.
    ///
    /// **The wipe was the fifth, and it is the one that took three
    /// readings.**
    /// It stayed here after the other four left, and the sentence that stood
    /// in this place said why: its scheduled move is a fade's exactly, what it
    /// also carries is a mask, and *"nothing has said whether the shape
    /// `SetTransition` holds is the wipe's to write"*. Something has now.
    /// The shape is the transition row's and arrives in
    /// [`Current::transition`] beside the quantum and the length, which are
    /// the same operation's other two settings; the soft edge is read off the
    /// mask running on the deck being wiped in, which is [`Current::mask`] and
    /// is what `Operation::SetMaskShape` already does with it. Neither was a
    /// computation, and for the third time what was missing was a reading.
    /// [`Current::mix`] is its third and arrived after the other two, for a
    /// different job: it is read so that the blend mode and the put-on-air can
    /// be left out where the deck arriving is already there.
    ///
    /// **What is left here is the one shape that is not a reading.** A tap's
    /// record is the beat lock's answer — the tapped tempo, the phase error
    /// against the oscillator, the output lag — and no value a surface holds
    /// determines it, which is why these two did not leave with the other
    /// four.
    NotSettled,
}

impl Owed {
    /// One sentence, for a caller that prints it. **A gap said out loud**, not
    /// a failure: the sentence names the question rather than blaming the
    /// caller, except for [`Owed::NotRead`], which is the caller's.
    pub fn why(self) -> &'static str {
        match self {
            Owed::NotRead(Reading::Look) => "the look that is running was not read",
            Owed::NotRead(Reading::MasterChain) => {
                "the master chain that is running was not read"
            }
            Owed::NotRead(Reading::Transport) => {
                "the transport of the deck it names was not read"
            }
            Owed::NotRead(Reading::Mask) => "the mask of the deck it names was not read",
            Owed::NotRead(Reading::Tempo) => "the session tempo its anchor comes from was not read",
            Owed::NotRead(Reading::Transition) => {
                "the transition settings its move is scheduled by were not handed over"
            }
            Owed::NotRead(Reading::Mix) => {
                "the blend mode and residency of the deck it names were not read"
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
    /// what P-0090 claims. One control, however many records the deck needs to
    /// be told.
    ///
    /// It was written before anything answered more than one, against the day
    /// a gesture would, and the prediction is what held: nothing about this
    /// shape changed when the crossfade landed, and nothing changed when the
    /// wipe followed it. `Operation::Wipe` is **six at most** for the same
    /// reason — five until the mask took a row for its shape and a row for its
    /// position, each of which writes a whole `Record::Mask` (ADR-0201) — and
    /// the arm writes them in the order the picture needs: the shape, the
    /// front at 0, the opacity, the blend mode, the put-on-air, and the move
    /// that carries the front to 1.
    ///
    /// **Never empty, and not always the same length.** The wipe is four, five
    /// or six: its blend mode and its put-on-air are written only where the
    /// deck arriving is not already there, so that a wipe does not overwrite a
    /// mode the operator chose (see the arm, and [`Mix`]). The order above is a
    /// property of what the list *does* hold rather than of how long it is —
    /// whatever is written is written in it, because the front has to be at 0
    /// before the move that carries it across is scheduled and a reader
    /// applies these in sequence.
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
            slot: DeckSlot(*deck),
            value: *gain,
        }),
        Operation::SetOpacity { deck, opacity } => one(Record::Opacity {
            slot: DeckSlot(*deck),
            value: *opacity,
        }),
        // `Record::Blend` carries the mode as a `String` — *"what a mode is
        // allowed to be is the engine's to say"* — so this is where a named
        // destination becomes a wire name.
        Operation::SetBlendMode { deck, blend } => one(Record::Blend {
            slot: DeckSlot(*deck),
            mode: blend.name().to_string(),
        }),
        // **The request, never the effective level.** The governor recomputes
        // the second every pass from the budget of the machine that is
        // running, which is why only the request is an operation and only the
        // request is a record.
        Operation::SetResidency { deck, residency } => one(Record::Residency {
            slot: DeckSlot(*deck),
            level: residency.name().to_string(),
        }),
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
            slot: DeckSlot(*deck),
            at: karakuri_store::record::NodeAddress {
                layer: store_layer(node.layer),
                index: node.index,
            },
            authority: authority.name().to_string(),
        }),
        // **A knob turn, and it needs no reading either.** This was
        // [`Silent::NoRecord`] and the reason given was not that a knob is
        // unworthy of a record: `Record::Param` is a *Set file's* and has no
        // `slot`, so there was nowhere in the vocabulary to put the deck this
        // operation names.
        // [`Record::Ride`] is that somewhere, and
        // [P-0092](../../../docs/principles/0092-the-same-inputs-produce-the-same-frame.md)
        // is what settled that there had to be one — *"mutating a live Set in
        // place … opens a hole in the record stream and loses replay, undo, A/B
        // comparison and session recording together"*. See
        // `docs/adr/0280-a-parameter-written-to-a-live-set-is-a-session-record.md`.
        //
        // **The wildcard crosses as an absence and not as an invented layer.**
        // `ParamAt::node` is `Option<NodeAddress>` and a bare key names no node, so
        // it names no layer either; `Record::Ride`'s `at` is the same
        // `Option`, which is the reason that record carries a
        // `karakuri_store::record::NodeAddress` rather than `Record::Param`'s
        // `layer` beside an `index`. A conversion that had to fill a `layer` in
        // for a wildcard would be writing down a placeholder, which is what
        // `Record::Param`'s `layer` was and is still living with.
        //
        // **The refusal is not taken here**, and could not be: a bare key over
        // nodes that are not under one authority is refused by
        // `karakuri_engine::set::Set::write_param`, which is the one place that
        // decides it and is unreachable from this crate by charter. So this
        // writes the record the operator asked for and the write is refused
        // where it lands — which is `Operation::SelectRenderer`'s shape
        // already: `written` answers a `Record::Select` for a renderer the Set
        // may no longer have, and the applier is what says so.
        Operation::WriteParam { deck, param, value } => one(Record::Ride {
            slot: DeckSlot(*deck),
            at: param.node.map(|node| karakuri_store::record::NodeAddress {
                layer: store_layer(node.layer),
                index: node.index,
            }),
            key: param.key.clone(),
            value: store_value(*value),
        }),
        // **An attachment, and the take-back that removes it — one record and
        // an `Option`.** `Record::Source` is `Record::Bind`'s session twin on
        // exactly the terms `Record::Ride` is `Record::Param`'s: a `bind` says
        // what a Set *is* and carries no deck slot, and this says what an
        // operator did to one deck slot at one instant. See
        // `docs/adr/0319-an-attachment-is-a-session-record-and-taking-a-parameter-back-removes-it.md`.
        //
        // **The address crosses whole and nothing is invented.**
        // `karakuri_operation::BindAt` is a layer, an optional index and a
        // key, which is `Record::Source`'s address field for field — the
        // reason that operation carries a `BindAt` rather than a `ParamAt` is
        // that a `ParamAt`'s wildcard names no layer, and a conversion filling
        // one in would be writing the placeholder `Record::Param`'s `layer`
        // already is. The `layer` is translated by [`store_layer`], which is
        // the one thing translated on this route as it is on the authority's.
        //
        // **No `noise`, and that is the operation's own decision read
        // through.** A generator's kind, rate, stream and octaves describe the
        // *source* and not the attachment, so `Operation::AttachSignal` does
        // not carry them; an absent `noise` on the record means *the default
        // generator* rather than *no generator*, which is what a `bind` naming
        // `noise` and saying nothing else has always meant. A caller that
        // wants a non-default generator states it in a Set file or on a
        // `--bind`, and this is where that limit is visible.
        Operation::AttachSignal {
            deck,
            param,
            signal,
            curve,
            range,
        } => one(Record::Source {
            slot: DeckSlot(*deck),
            layer: store_layer(param.layer),
            index: param.index,
            key: param.key.clone(),
            source: Some(karakuri_store::record::Source {
                signal: signal.clone(),
                curve: curve.name().to_string(),
                range: *range,
                noise: None,
            }),
        }),
        // **The same record with nothing in it, which is the whole of the
        // take-back.** Two records — an `attach` and a `detach` — would be two
        // `t`s the projection drops for one reason and the applier has to keep
        // in step by care; one record whose payload is present or absent is a
        // thing nothing can write down half-detached, which is
        // `docs/contributing.md` §4's structural tier and `NodeAddress`'s argument
        // one record along.
        //
        // **It removes rather than suspends**, and the reason is that there is
        // no suspended state anywhere to record: `karakuri_engine::binding::Binding`
        // carries none, and *not driving this parameter* is already written as
        // the absence — P-0084's blend writes the param's own value when
        // nothing is attached.
        Operation::TakeParamBack { deck, param } => one(Record::Source {
            slot: DeckSlot(*deck),
            layer: store_layer(param.layer),
            index: param.index,
            key: param.key.clone(),
            source: None,
        }),
        // **A free-running tempo being stated**, which is `Record::Tempo`'s own
        // words for a correction with no shift and no confidence: *"The first
        // one in a stream is what sets the session tempo … a correction with
        // `shift` 0.0 and `confidence` 0.0 is a free-running tempo being
        // stated."* P-0090 names `--bpm` as the flag with no record behind it;
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
        // **The master chain's three, and they are the look pair's argument
        // with a list in it.** `Record::MasterChain` is written whole, each
        // operation names one parameter of one slot, and every other slot comes
        // from the chain that is running — so a press on the bloom row cannot
        // put the feedback back where a default left it. **Which slot** is the
        // procedure's content address, which is the *for now* ADR-0340 §7
        // names: the three rows retire in M5.16's second pass and are answered
        // here in the meantime by finding — or appending — the slot whose
        // procedure is the matching shipped one. These carried
        // `karakuri_operation::Undecided` and answered `Owed::Undecided` until
        // 2026-09-09, on the ground that *"a record kept for a thing that does
        // not exist would be inventing its contents"* (ADR-0227); the chain
        // exists now and the contents are three amounts and a cut. See
        // `docs/adr/0317-…`.
        //
        // **And no clamp**, on `SetMasterOut`'s terms: the range is the one the
        // procedure declared and the wall is `karakuri_engine::master::Slot`,
        // where the record is applied, so every route in meets one.
        Operation::SetFeedback { params } => match &current.master_chain {
            Some(chain) => one(Record::MasterChain(karakuri_store::record::Chain {
                slots: chain.moved(
                    &chain.shipped.feedback,
                    Some(&params.cut),
                    "amount",
                    params.amount,
                ),
            })),
            None => Written::Owed(Owed::NotRead(Reading::MasterChain)),
        },
        Operation::SetBloom { params } => match &current.master_chain {
            Some(chain) => one(Record::MasterChain(karakuri_store::record::Chain {
                slots: chain.moved(&chain.shipped.bloom, None, "amount", params.amount),
            })),
            None => Written::Owed(Owed::NotRead(Reading::MasterChain)),
        },
        Operation::SetRgbShift { params } => match &current.master_chain {
            Some(chain) => one(Record::MasterChain(karakuri_store::record::Chain {
                slots: chain.moved(&chain.shipped.rgb_shift, None, "amount", params.amount),
            })),
            None => Written::Owed(Owed::NotRead(Reading::MasterChain)),
        },
        // **Half a mask each, and the record is whole.** The half that was
        // not asked for comes from the mask that is running, exactly as the
        // look pair fills in the two thirds a control change cannot say —
        // ADR-0192, one layer down. Which half is the ask is what decides
        // whether a running move is cancelled, and that is the deck's rule
        // rather than this crate's: `Record::Mask` carries a state and not an
        // ask, so nothing downstream of here can tell the two apart. See
        // `docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md`.
        Operation::SetMaskShape { deck, kind, angle } => match current.mask {
            Some(mask) => one(Record::Mask {
                slot: DeckSlot(*deck),
                kind: kind.name().to_string(),
                angle: *angle,
                position: mask.position,
                softness: mask.softness,
            }),
            None => Written::Owed(Owed::NotRead(Reading::Mask)),
        },
        Operation::SetMaskPosition { deck, position } => match current.mask {
            Some(mask) => one(Record::Mask {
                slot: DeckSlot(*deck),
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
                slot: DeckSlot(*deck),
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
        // ([P-0092](../../../docs/principles/0092-the-same-inputs-produce-the-same-frame.md)):
        // the anchor is written down at the instant the operator asked, and
        // `Transport::set` puts it back without recomputing it from the machine
        // the replay is running on.
        Operation::SetSync { deck, sync } => match current.tempo {
            Some(bpm) => one(Record::Transport {
                slot: DeckSlot(*deck),
                sync: sync.name().to_string(),
                anchor_bpm: bpm,
                scrub_beats: 0.0,
            }),
            None => Written::Owed(Owed::NotRead(Reading::Tempo)),
        },

        // ----- What it schedules, given the surface's settings -------------
        //
        // Four, and the reading they share is [`Current::transition`]: the
        // instant the move lands on, how long it lasts and the shape it
        // takes. None of the four is on the operation, because an operation
        // says what it wants and never how it is scheduled.
        //
        // **The wipe is the fourth and takes a second reading beside it**,
        // which is what its arm at the end of this group is about: a fade
        // moves a fader that is already there, and a wipe has to put a mask on
        // the deck first.
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
                    slot: DeckSlot(*to),
                    value: 0.0,
                },
                Record::Residency {
                    slot: DeckSlot(*to),
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
                slot: DeckSlot(*deck),
                renderer: *renderer,
                start: transition.start,
            }),
            None => Written::Owed(Owed::NotRead(Reading::Transition)),
        },
        // **One gesture, six records at most, and it is the largest thing
        // here.**
        //
        // A wipe is a mask and one scheduled move, and nothing in either half
        // knows about the other: the transition moves a number and the mask
        // reads one. So the deck arriving is given the shape at position 0 —
        // revealing nothing — put on air under `over` so that what it reveals
        // *hides* what is beneath, and then one move carries the front from 0
        // to 1. **Two of the six are written only where they would change
        // something**, which is the paragraph below and why this is *at most*:
        // four, five or six, always in the order they are listed in.
        //
        // **Everything is written about `to`, and `from` is read for
        // nothing.** The deck being covered is not touched: it is revealed
        // away from rather than moved, which is why this operation names two
        // decks and writes about one. That is also what decides whose mask
        // [`Current::mask`] is — the arriving deck's, said at [`Current`].
        //
        // **The two readings, and where each of the six fields comes from.**
        // The shape and the angle are [`Current::transition`]'s
        // `wipe_kind` and `wipe_angle` — `Operation::SetTransition`'s third
        // setting, the surface's own, arriving beside the instant and the
        // length that are its other two. The soft edge is the running mask's
        // and nothing else's: no operation names a softness, so it is read off
        // the deck exactly as `Operation::SetMaskShape` one group up reads it,
        // and a wipe that invented one would rewrite a value nobody asked
        // about ([`Mask::softness`], [`Look::white_point`]).
        //
        // **Two `Record::Mask` and not one**, which is what routing the mask
        // honestly costs rather than an accident: the shape and the front are
        // two operations (ADR-0201) and each of them writes the record whole,
        // because a `Record::Mask` is a *state* and not an ask. The first
        // keeps the front where the deck already had it, which is
        // `SetMaskShape`'s arm; the second restates the shape the first chose
        // and puts the front at 0, which is `SetMaskPosition`'s arm reading
        // the mask the record before it just wrote. They land in that order,
        // so the front is at 0 before the move that carries it to 1 is
        // scheduled.
        //
        // **The blend and the put-on-air are written only where they change
        // something**, which is what the third reading is for: [`Current::mix`]
        // is taken so that a record can be left *out*, and this is the arm
        // that leaves it. `over` is written only where the deck arriving is
        // still at `add` — where a slot starts, which is
        // `karakuri_operation::BlendMode::ALL`'s own sentence — and the
        // put-on-air only where the deck is not already live.
        //
        // **Because the mode is the operator's and a wipe under it is a
        // picture.** Under `add` the same gesture is a wipe *on* rather than a
        // wipe *over*, and under `max` it is a third thing; both are
        // legitimate, and a gesture that forced `over` every time would take
        // the choice away from the hand that made it
        // ([P-0094](../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md):
        // a rule protecting a performance may not remove what the performance
        // is played with). That is what makes `m` in front of `c` mean
        // something, and the sentence is here rather than on a surface because
        // this is where it is now decided: `karakuri-cli`'s `c` held it while
        // it built these records itself, and it builds none.
        //
        // **The crossfade one gesture back is not the same case and did not
        // change.** Its put-on-air is unconditional because a `residency live`
        // for a slot already live decodes to a state it is already in; what
        // that would have cost here is a blend mode somebody chose, which is a
        // different picture rather than the same one restated.
        //
        // **The move is on the mask's front and not on a fader**, which is
        // the one place this differs from every other scheduled move here:
        // [`MASK`] rather than [`OPACITY`], and `fade` is not what builds it.
        //
        // **A shape of `WipeKind::None` is refused by the surface and not
        // here.** `Operation::Wipe` says *"Refused with no shape chosen"* and
        // [`Written`] has three answers, none of which is a refusal — the
        // shape is the surface's own setting, so the surface is where a wipe
        // with nothing to move is turned away, before it asks.
        Operation::Wipe { from: _, to } => {
            match (current.transition, current.mask, current.mix) {
                (Some(transition), Some(mask), Some(mix)) => {
                    let mut records = vec![
                        Record::Mask {
                            slot: DeckSlot(*to),
                            kind: transition.wipe_kind.name().to_string(),
                            angle: transition.wipe_angle,
                            position: mask.position,
                            softness: mask.softness,
                        },
                        Record::Mask {
                            slot: DeckSlot(*to),
                            kind: transition.wipe_kind.name().to_string(),
                            angle: transition.wipe_angle,
                            position: 0.0,
                            softness: mask.softness,
                        },
                        Record::Opacity {
                            slot: DeckSlot(*to),
                            value: 1.0,
                        },
                    ];
                    if mix.blend == karakuri_operation::BlendMode::Add {
                        records.push(Record::Blend {
                            slot: DeckSlot(*to),
                            mode: karakuri_operation::BlendMode::Over.name().to_string(),
                        });
                    }
                    if mix.residency != karakuri_operation::Residency::Live {
                        records.push(Record::Residency {
                            slot: DeckSlot(*to),
                            level: karakuri_operation::Residency::Live.name().to_string(),
                        });
                    }
                    records.push(Record::Transition {
                        slot: DeckSlot(*to),
                        control: MASK.to_string(),
                        to: 1.0,
                        start: transition.start,
                        beats: transition.beats,
                        curve: transition.curve.name().to_string(),
                    });
                    Written::Records(records)
                }
                // **The settings first**, because that is the reading a caller
                // is holding rather than one it would have had to take off the
                // engine — see [`Reading::Transition`]. A surface that handed
                // in neither is told about the one it forgot before the one it
                // did not look up. The mix comes last for the other half of
                // that rule: it is the reading a caller looks up *and* the one
                // whose absence a record would not show, so a caller missing
                // two is told about the one it can fix from memory first.
                (None, _, _) => Written::Owed(Owed::NotRead(Reading::Transition)),
                (_, None, _) => Written::Owed(Owed::NotRead(Reading::Mask)),
                (_, _, None) => Written::Owed(Owed::NotRead(Reading::Mix)),
            }
        }

        // ----- Owed: the tracker ------------------------------------------
        //
        // **Two, and the wipe that used to be the third has gone**, which is
        // the line [`Owed::NotSettled`] draws seen from the side that is left:
        // what a tap and an octave shift owe is not a value any surface holds
        // but the beat lock's answer — a tapped tempo, a phase error, an
        // output lag — and no reading added to [`Current`] would make either
        // of them a function of values. The wipe was never that shape: what it
        // was missing was two readings and an owner for each, which is exactly
        // what the four before it were missing.
        Operation::TapBeat | Operation::ScaleGrid { .. } => Written::Owed(Owed::NotSettled),

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
        //
        // **`Operation::WalkHistory` left on 2026-09-10**, the way
        // `Operation::RouteFrame` left below: by having its payload decided
        // rather than by this rule bending. It carries the Set it is a walk of
        // now, so *what it acts on* is answered and *what it writes* could be
        // asked — and the answer is `Silent::Question`, three arms down
        // (`docs/adr/0342-a-walk-names-the-set-it-is-of-and-the-two-rows-beside-it-are-gap.md`).
        Operation::MoveBoundary { .. } | Operation::WatchFiles { .. } => {
            Written::Owed(Owed::Undecided)
        }

        // ----- Silent: publishing is not the performance -------------------
        //
        // **A member of the group above left it on 2026-09-09**, and it left
        // by having its payload decided rather than by that rule changing:
        // `Operation::RouteFrame` carries a `karakuri_operation::Output` and a
        // `bool` now, so *what it acts on* is answered and *what it writes*
        // could be asked. The answer is nothing, and it is an argument rather
        // than a gap — see [`Silent::Published`], which is where the three
        // reasons are.
        Operation::RouteFrame { .. } => Written::Silent(Silent::Published),

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
        // **A star is this room's statement about the library, and
        // `console.html` refused the other two places by name.** In the Set
        // file it travels with the material and has to be *written*, so a Set
        // would jump to the top of a listing ordered by when it was made and a
        // favourite would be indistinguishable from an edit. In the session it
        // is a thing that happened at a moment: a press with `rec` off would
        // keep nothing, and a replay would hand the favourite back as an event
        // rather than as something that is true. So it is kept beside the Sets,
        // in the store, and nothing in the stream carries it.
        //
        // **`Surface` rather than `NoRecord`, and that is the arm to think
        // about**, because a star does not live in this console the way a fold
        // does — it is on the disk and it outlives the run. It is here for
        // `Operation::SaveArrangement`'s reason one line up: `NoRecord` is
        // where an operation goes when the record vocabulary has no row for
        // what it does **and that is a gap**, and this is the opposite. The
        // vocabulary having no row for a favourite is the decision, taken
        // against the stream by name on `docs/manual/console.html` and recorded
        // in `docs/adr/0299-…`. Nothing is owed, and a replay reconstructs
        // nothing from a star: a session played back in somebody else's room
        // would otherwise arrive carrying this room's attention.
        | Operation::SetFavourite { .. }
        // **A library write, and it is here for the arrangement's reason
        // rather than for a new one.** `Operation::KeepProcedure` puts one
        // node's source under `<store>/procedures/` — an operator's own act,
        // which is what makes that tier exist
        // ([P-0096](../../../docs/principles/0096-the-operators-library-is-written-by-an-operators-own-act.md))
        // — and nothing in the session vocabulary is a library write. The arm
        // above already covers *a file under the store*, because
        // `SaveArrangement` put it here, and its sentence transfers word for
        // word: *"a save writes a file, and a file is not a record whose timing
        // `OnLanding` could be about, nor a gap `NoRecord` could be about."*
        //
        // **Not `Silent::OnLanding`, where `Operation::SaveSet` beside it is.**
        // A Set save *has* a record — `Record::Save` — and that arm answers
        // only *when* it is written. There is no record here whose timing could
        // be at issue and none ever arrives.
        //
        // **Not `Silent::NoRecord`**: that arm is where the record vocabulary
        // has no row for what an operation does **and that is a gap**. This is
        // the opposite — a replay reconstructs nothing from a library, and a
        // session played back in somebody else's room would otherwise arrive
        // writing into it
        // (`docs/adr/0338-a-procedure-is-a-row-of-the-library-and-one-loaded-over-a-layer-makes-a-set-with-no-name.md`).
        | Operation::KeepProcedure { .. }
        // **A pane's target is a pointer this console owns**, beside its scroll
        // position, the deck selection and the library cursor — none of which
        // the stream carries, for `Operation::SelectDeck`'s reason four lines
        // up. A replay that reconstructed it would be putting somebody else's
        // attention on the screen.
        | Operation::PointPane { .. }
        | Operation::SizeWindow { .. }
        // **The sequencer's five, and they are the second family of the kind
        // the arrangement made this arm hold.** They answered
        // `Owed(Undecided)` until 2026-09-09 on two reasons, and both are
        // gone.
        //
        // **The first was refuted in this file already.** It read: *"`Surface`
        // would call a pattern the console's own state, where ADR-0227 makes
        // it library data under the store on a Set's and an arrangement's
        // terms."* An **arrangement** is library data under the store on those
        // same terms and it is four lines up, and the test that pins it says
        // why in a sentence that transfers word for word — *"a save writes a
        // file, and a file is not a record whose timing `OnLanding` could be
        // about, nor a gap `NoRecord` could be about."* So this arm already
        // covers *a file under the store*, because the arrangement put it
        // here; nothing is widened to fit these five.
        //
        // **The second was that nothing about them had been settled**, and
        // four records are what settled it (ADR-0320 to ADR-0323). Read as
        // settling the stream's silence rather than deferring it, ADR-0227's
        // refusal of a pattern record *is* this answer: a lane is a fifth
        // route (ADR-0222), so what a pattern does already lands in the stream
        // as `Record::Opacity` and `Record::Ride` — its writes are its record
        // — and a pattern record beside them would be *"the cause written down
        // next to every one of its consequences."*
        //
        // **Not `Silent::NoRecord`**, for `Operation::SaveArrangement`'s
        // reason two paragraphs up: that arm is where an operation goes when
        // the record vocabulary has no row for what it does **and that is a
        // gap**, and this is the opposite — the vocabulary having no row for a
        // pattern is the decision, taken against the stream by name.
        //
        // **Not `Silent::OnLanding`**: nothing in the session vocabulary is a
        // pattern, so there is no record here whose *timing* could be at
        // issue, and none ever arrives.
        //
        // **What a replay does is the other half of the same answer**
        // (ADR-0322): the sequencer does not run on a replay, because its
        // consequences are already in the stream verbatim — `tick`'s and
        // `audio`'s arrangement, derived from the world when live and read
        // back when not.
        | Operation::SetStep { .. }
        | Operation::SetLaneMute { .. }
        | Operation::PointLane { .. }
        | Operation::SetPatternGrid { .. }
        | Operation::SelectPattern { .. } => Written::Silent(Silent::Surface),

        // ----- Silent: it asks rather than changes -------------------------
        //
        // **`FilterLibrary` is `ListSets`' answer and not a fourth kind of
        // silence.** The six toggles decide which populations the bay's listing
        // is drawn from — the store's Sets, the store's procedures, the
        // presets' — so the press is a narrowed *ask* of the library and the
        // answer goes back to the surface that asked, exactly as `holds` and
        // `layer` do on the row beside it. Nothing on any deck moves.
        //
        // **A walk is a listing of the store and nothing else** — the fifth
        // member of this arm since 2026-09-10, and it arrived from
        // `Owed(Undecided)` rather than from anywhere else. *What versions has
        // this Set had* is `karakuri_environment::history::list` narrowed to
        // one id, opening no file and moving no deck; **landing on one of them
        // is `Operation::RestoreProcedure`**, which is a write, writes
        // `Record::Procedure` where the swap lands, and is a row of its own on
        // the page (ADR-0308). `docs/adr/0281-…`'s consequence *"Walk the edit
        // history is a write"* was written while this payload was going to be
        // *a revision to land on*; that half moved to the landing's row the
        // same day and this one is what is left, which is the question
        // (`docs/adr/0342-a-walk-names-the-set-it-is-of-and-the-two-rows-beside-it-are-gap.md`).
        //
        // **The chip press that asks for it is not this answer's business.**
        // Marking a scope is a surface's own state and it is `SelectScope`'s
        // arm above; what a walk *writes* is nothing, whoever asked.
        Operation::ListSets { .. }
        | Operation::FilterLibrary { .. }
        | Operation::ReadSet { .. }
        | Operation::ReadProcedure { .. }
        | Operation::WalkHistory { .. }
        | Operation::SwapOutcome => Written::Silent(Silent::Question),

        // ----- Silent: the record is written where the work lands ----------
        //
        // `RestoreProcedure` is beside `WriteProcedure` because it **is** one:
        // putting a node's previous version back is the same check, the same
        // worker and the same frame boundary, so the `Record::Procedure` is
        // written at the swap and the restored version is judged there like any
        // other. **It is also the way out of a stopped slot** (ADR-0316):
        // nothing puts a version back on its own any more, so this operation is
        // what a person reaches for when a build has stopped a slot for cost —
        // and because it writes the file as well as moving the picture, the two
        // agree afterwards. **The row reporting the stop is where they reach
        // it**, since ADR-0326: the Staging lane's `back` capsule asks for it
        // with `Revision::Previous`, one step and never a cursor, where the
        // Library bay's `history` rows ask for it with `Revision::Picked`.
        // Neither arm changes the answer here, because this arm reads no
        // payload.
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
        //
        // **That is what lets the whole row be the control** (ADR-0326): the
        // press moves nothing, writes nothing and takes a line off a list, so
        // it is the one of that row's two acts that can sit on a target a
        // pointer lands on by accident. The act that writes a file is the
        // capsule inside it.
        Operation::KeepCandidate { .. } => Written::Silent(Silent::Surface),

        // ----- Silent: nothing in the session vocabulary carries it --------
        //
        // `WireInput` and `SetProperty` are the two that are
        // not simply absent: `Record::Edge`,
        // `Record::Capacity`, `Record::Seed` and
        // `Record::Camera` all exist and are a **Set file's**, with no `slot`
        // to carry the deck the operation names. `SetCompositing` is the same
        // shape — `Record::Merge` is what a *Set* says about its layering, and
        // *"nothing in a stream says that the Set in slot 3 composites."*
        //
        // **`WriteParam` was the third and `AttachSignal` the fourth, and both
        // have left this group**, which is what the shape of the answer always
        // was: the reason was never that a knob or an attachment is unworthy of
        // a record, it was that no record could carry the deck.
        // `Record::Ride` and `Record::Source` carry it, and what is still here
        // is what nobody has written the session's twin of yet.
        //
        // **`WireInput` and `Publish` have panel controls now**, and the two
        // are not the same kind of silence — which is worth holding apart here
        // because both read as *nothing is written* from the outside
        // (`docs/adr/0329-…`).
        //
        // **`WireInput` is `SetProperty`'s shape**: `Record::Edge` exists, is a
        // **Set file's** statement about a Set and carries no slot, so a
        // session cannot say *the Set in slot 3 wires `far` to `sphere_shell`*
        // — and a **keep** writes the run's edges into the file it saves, so
        // what a hand asked for is recorded the moment the deck is kept.
        //
        // **`Publish` is the other shape and is the weaker one**: nothing in
        // this format says what a Set publishes, in a Set file or in a session.
        // So a narrowing is reproduced by no replay **and by no keep**, and it
        // lives in the run that made it. That is the first group's first
        // sentence — *a row the record format has never had* — met by a control
        // rather than by a tool, and it is a gap named on both manual pages
        // rather than a decision that the act is unworthy of a record.
        //
        // **`SetProperty` has two panel controls now and still writes
        // nothing**, on `SetCompositing`'s argument below and in its company:
        // the deck head's capacity chip and its `re-salt` capsule each move one
        // field of the aim and let the worker rebuild the slot, which is a
        // re-point rather than anything the session vocabulary can say
        // (`docs/adr/0328-the-inspectors-deck-head-steps-a-slots-capacity-and-re-salts-it.md`).
        // What it costs is the same sentence: a session replayed does not come
        // back at the capacity a hand stepped to or the salt a hand pressed
        // for. **A keep does**, which is the half worth knowing — `capacity`
        // and `seed` are Set-file records, so what a hand asked for is written
        // down the moment the deck is kept, and the gap is the session stream's
        // alone.
        //
        // **`SetCompositing` has a panel control now and still writes
        // nothing**, and it is worth saying why the first did not move the
        // second. The console's deck head folds a slot's renderers by
        // *re-aiming* it — the layering is one field of the description its
        // watcher is pointed at, so the window restates the rest and the worker
        // rebuilds the slot
        // (`docs/adr/0314-a-control-that-moves-a-field-of-the-aim-re-aims-the-slot-and-the-rebuild-is-the-write.md`).
        // That is precisely what `Operation::LoadSet` two lines up already
        // does, and it answers `Silent(NoRecord)` for the same reason: a
        // re-point is not something the session vocabulary can say. What it
        // costs is real and is written in that record — a session replayed
        // does not come back compositing where a hand asked for it — and it is
        // the load's cost rather than a new one.
        //
        // **`LoadProcedure` is `LoadSet`'s answer and its cost unchanged in
        // size.** It re-points the same slot through the same watcher, with one
        // file replaced instead of every file, so it is a re-point and a
        // re-point is not something the session vocabulary can say. What it
        // costs is the load's cost reached from one more control: a session
        // replayed does not come back with the layer a hand swapped, and it
        // never came back with the Set a hand loaded either.
        //
        // **What it does *not* lose is the version.** The rebuild compiles, and
        // compiling is the gate, so every version written after the swap is
        // kept — filed under the Set the slot started from, because the
        // procedure load leaves `watch::Aim`'s `set` where it is
        // (`docs/adr/0304-the-set-a-version-is-filed-under-rides-the-aim-that-re-points-the-slot.md`).
        // That is the half of the maintainer's answer this arm is the other
        // side of: the derived Set has no name and its history is not lost with
        // it.
        Operation::SetLatencyOffset { .. }
        | Operation::AttachBeatSource { .. }
        | Operation::LoadSet { .. }
        | Operation::LoadProcedure { .. }
        | Operation::SetCompositing { .. }
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
        slot: DeckSlot(slot),
        control: OPACITY.to_string(),
        to,
        start: transition.start,
        beats: transition.beats,
        curve: transition.curve.name().to_string(),
    }
}

/// **The wire name of the control a fade moves**, and one of the two spellings
/// in this crate with no list of its own behind it.
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

/// **The wire name of the control a wipe moves**, and the second of those two.
///
/// [`OPACITY`]'s paragraph word for word, one control along:
/// `Operation::Wipe` *is* the mask-position one and says so at its own
/// definition — *"one scheduled move carrying the front to 1"* — so no
/// operation names this control either and there is still no list here to be
/// exhaustive over. It is checked in the same place and by the same argument.
///
/// **`mask` and not `mask-position`**, which is the engine's spelling and the
/// only one that decodes: `karakuri_engine::transition::Control::name` is what
/// reads it back, and a move on a control that failed to decode is a wipe that
/// replays as nothing at all.
const MASK: &str = "mask";

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
/// **The vocabulary's parameter value as the store's**, and the second list
/// this crate translates between rather than carries.
///
/// [`store_layer`]'s paragraph word for word, one type along: both are foreign
/// here so the orphan rule refuses a `From`, and a match rather than a cast so
/// that a width added to either list stops the build until somebody says what
/// it is on the other side.
///
/// **The two lists are the same three widths and stay that way**, because a
/// parameter is driven one component at a time and a wide value is what a
/// person or a model writes on one line
/// ([ADR-0268](../../../docs/adr/0268-a-vector-parameter-is-driven-one-component-at-a-time.md)).
/// Nothing is expanded here: a `vec3` becomes one [`Record::Ride`] carrying
/// three numbers, exactly as it becomes one `param` line in a Set file, and the
/// reader that has the Set in hand is what turns it into three writes.
fn store_value(value: karakuri_operation::ParamValue) -> karakuri_store::record::Value {
    use karakuri_operation::ParamValue as From;
    use karakuri_store::record::Value as To;
    match value {
        From::Scalar(v) => To::Scalar(v),
        From::Vec2(v) => To::Vec2(v),
        From::Vec3(v) => To::Vec3(v),
    }
}

fn store_layer(layer: karakuri_operation::Layer) -> karakuri_store::record::Layer {
    use karakuri_operation::Layer as From;
    use karakuri_store::record::Layer as To;
    match layer {
        From::L1 => To::L1,
        From::L2 => To::L2,
        From::L3 => To::L3,
        From::L4 => To::L4,
        From::Field => To::Field,
        From::L5 => To::L5,
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
    /// A chain of the three shipped procedures with every value distinct, so a
    /// record that copied the wrong one is a failing assertion rather than a
    /// coincidence. The addresses are short stand-ins for the real content
    /// addresses: what this crate does with one is compare it, never resolve
    /// it.
    fn chain() -> Chain {
        Chain {
            feedback: karakuri_operation::Feedback {
                amount: 0.34,
                cut: karakuri_operation::Cut::Exit,
            },
            bloom: 0.6,
            rgb_shift: 0.25,
            slots: vec![
                slot("sha256:feedback", Some("exit"), 0.34),
                slot("sha256:bloom", None, 0.6),
                slot("sha256:rgb_shift", None, 0.25),
            ],
            shipped: Shipped {
                feedback: "sha256:feedback".into(),
                bloom: "sha256:bloom".into(),
                rgb_shift: "sha256:rgb_shift".into(),
            },
        }
    }

    fn slot(procedure: &str, cut: Option<&str>, amount: f32) -> karakuri_store::record::ChainSlot {
        karakuri_store::record::ChainSlot {
            procedure: procedure.to_string(),
            cut: cut.map(str::to_string),
            params: [("amount".to_string(), amount)].into_iter().collect(),
        }
    }

    fn chain_record(slots: Vec<karakuri_store::record::ChainSlot>) -> Record {
        Record::MasterChain(karakuri_store::record::Chain { slots })
    }

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

    /// **The chain that is running is what fills in the rows nobody
    /// pressed**, which is the look pair's claim with one more row in it: a
    /// press on the bloom row must not put the feedback back where a default
    /// left it.
    #[test]
    fn a_bloom_press_keeps_the_feedback_and_the_shift_that_are_running() {
        let current = Current {
            master_chain: Some(chain()),
            ..Current::default()
        };
        let written = written(
            &Operation::SetBloom {
                params: karakuri_operation::Bloom { amount: 0.6 },
            },
            &current,
        );
        assert_eq!(
            records(written),
            vec![chain_record(vec![
                slot("sha256:feedback", Some("exit"), 0.34),
                slot("sha256:bloom", None, 0.6),
                slot("sha256:rgb_shift", None, 0.25),
            ])],
            "a bloom press rewrote a slot it did not name — the record carries the \
             whole list and only the bloom slot was asked for"
        );
    }

    /// **Feedback carries two of the four**, and the cut is one of them: the
    /// same amount is a one-frame echo under `mix` and a compounding trail
    /// under `exit`, so a surface that could move the amount without saying
    /// the cut would be asking for a picture it had not named.
    #[test]
    fn a_feedback_press_carries_its_cut_and_keeps_the_other_two_passes() {
        let current = Current {
            master_chain: Some(chain()),
            ..Current::default()
        };
        let written = written(
            &Operation::SetFeedback {
                params: karakuri_operation::Feedback {
                    amount: 0.9,
                    cut: karakuri_operation::Cut::Mix,
                },
            },
            &current,
        );
        assert_eq!(
            records(written),
            vec![chain_record(vec![
                slot("sha256:feedback", Some("mix"), 0.9),
                slot("sha256:bloom", None, 0.6),
                slot("sha256:rgb_shift", None, 0.25),
            ])]
        );
    }

    /// **And the third row is the other two's arm.** Worth its own test
    /// because it is the row whose figure used to be a dash: an amount of zero
    /// is a value that reaches a record, not a row with nothing to say.
    #[test]
    fn an_rgb_shift_press_writes_a_zero_rather_than_nothing() {
        let current = Current {
            master_chain: Some(chain()),
            ..Current::default()
        };
        let written = written(
            &Operation::SetRgbShift {
                params: karakuri_operation::RgbShift { amount: 0.0 },
            },
            &current,
        );
        assert_eq!(
            records(written),
            vec![chain_record(vec![
                slot("sha256:feedback", Some("exit"), 0.34),
                slot("sha256:bloom", None, 0.6),
                slot("sha256:rgb_shift", None, 0.0),
            ])]
        );
    }

    /// **A row naming a procedure the chain has not got appends a slot**, which
    /// is the *for now* in ADR-0340 §7: with a list, *feedback* is a pass that
    /// may not be in the chain, and until the three rows retire the honest
    /// answer to *turn feedback up* is a chain with feedback in it. It lands at
    /// the end, which is where a drop on the chain lands one too.
    #[test]
    fn a_row_naming_a_procedure_the_chain_has_not_got_appends_a_slot() {
        let mut chain = chain();
        chain.slots = vec![slot("sha256:rgb_shift", None, 0.25)];
        let current = Current {
            master_chain: Some(chain),
            ..Current::default()
        };
        let written = written(
            &Operation::SetFeedback {
                params: karakuri_operation::Feedback {
                    amount: 0.5,
                    cut: karakuri_operation::Cut::Mix,
                },
            },
            &current,
        );
        assert_eq!(
            records(written),
            vec![chain_record(vec![
                slot("sha256:rgb_shift", None, 0.25),
                slot("sha256:feedback", Some("mix"), 0.5),
            ])],
            "a row naming a procedure the chain has not got wrote a chain without it"
        );
    }

    /// **A chain that was not read is said, never defaulted** — the look
    /// pair's rule at the row below it. A default chain here would let a
    /// bloom press silently zero a trail somebody set a moment earlier.
    #[test]
    fn a_master_row_with_no_chain_read_is_owed_it_rather_than_given_a_default() {
        for operation in [
            Operation::SetFeedback {
                params: karakuri_operation::Feedback::default(),
            },
            Operation::SetBloom {
                params: karakuri_operation::Bloom::default(),
            },
            Operation::SetRgbShift {
                params: karakuri_operation::RgbShift::default(),
            },
        ] {
            assert_eq!(
                written(&operation, &Current::default()),
                Written::Owed(Owed::NotRead(Reading::MasterChain)),
                "{operation:?} invented a chain nobody read"
            );
        }
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

    /// **A deck that is nowhere the wipe is about to put it**: still at the
    /// blend mode a slot starts in, and not on air.
    ///
    /// So both of the two records a wipe writes conditionally are written
    /// against this fixture, and a wipe is its full six — which is what makes
    /// [`a_wipe_leaves_a_mode_the_operator_chose_and_a_deck_already_on_air`]
    /// the other half of one statement rather than a second subject. A fixture
    /// already under `over` would have hidden the omission behind a record
    /// that says the same thing.
    fn mix() -> Mix {
        Mix {
            blend: karakuri_operation::BlendMode::Add,
            residency: karakuri_operation::Residency::Allocated,
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
                slot: DeckSlot(2),
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
                slot: DeckSlot(2),
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
                slot: DeckSlot(2),
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
                slot: DeckSlot(2),
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
                slot: DeckSlot(3),
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
                slot: DeckSlot(1),
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
                slot: DeckSlot(0),
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
    /// here that turns a `karakuri_operation::NodeAddress` into a
    /// `karakuri_store::record::NodeAddress`, and `store_layer` is a second
    /// spelling of a list that has to stay in step.
    /// A conversion that dropped the index would put every renderer's authority
    /// on the first one; one that mistranslated the layer would put an L4's on
    /// an L1.
    #[test]
    fn an_authority_carries_the_node_it_names_and_needs_no_reading() {
        assert_eq!(
            records(written(
                &Operation::SetAuthority {
                    deck: 2,
                    node: karakuri_operation::NodeAddress {
                        layer: karakuri_operation::Layer::L4,
                        index: 1,
                    },
                    authority: karakuri_operation::Authority::Suggesting,
                },
                &Current::default()
            )),
            vec![Record::Authority {
                slot: DeckSlot(2),
                at: karakuri_store::record::NodeAddress {
                    layer: karakuri_store::record::Layer::L4,
                    index: 1,
                },
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
                    node: karakuri_operation::NodeAddress {
                        layer: karakuri_operation::Layer::Field,
                        index: 0,
                    },
                    authority: karakuri_operation::Authority::Manual,
                },
                &Current::default()
            )),
            vec![Record::Authority {
                slot: DeckSlot(0),
                at: karakuri_store::record::NodeAddress {
                    layer: karakuri_store::record::Layer::Field,
                    index: 0,
                },
                authority: "manual".to_string(),
            }]
        );
    }

    /// **An attachment writes a record, and the take-back is the same record
    /// with nothing in it.**
    ///
    /// Both answered `Silent(NoRecord)` until
    /// `docs/adr/0319-an-attachment-is-a-session-record-and-taking-a-parameter-back-removes-it.md`,
    /// and the reason was the one `WriteParam` had: `Record::Bind` is a Set
    /// file's and has no `slot`. So what is asserted here is the deck arriving
    /// in the record — an attachment made on slot 3 that came back saying
    /// nothing about slot 3 is the whole defect back — and that the two
    /// operations write **one** `t` rather than two, which is what makes them
    /// one fact for the projection to drop.
    #[test]
    fn an_attachment_and_a_take_back_are_one_record_with_and_without_a_source() {
        let attached = records(written(
            &Operation::AttachSignal {
                deck: 3,
                param: karakuri_operation::BindAt {
                    layer: karakuri_operation::Layer::L1,
                    index: Some(2),
                    key: "turbulence".to_string(),
                },
                signal: "energy".to_string(),
                curve: karakuri_operation::Curve::Pow2,
                range: [0.1, 2.4],
            },
            &Current::default(),
        ));
        assert_eq!(
            attached,
            vec![Record::Source {
                slot: DeckSlot(3),
                layer: karakuri_store::record::Layer::L1,
                index: Some(2),
                key: "turbulence".to_string(),
                source: Some(karakuri_store::record::Source {
                    signal: "energy".to_string(),
                    curve: "pow2".to_string(),
                    range: [0.1, 2.4],
                    // **Absent means the default generator and not the absence
                    // of one**, which is what a `bind` naming `noise` and
                    // saying nothing else has always meant. The operation does
                    // not carry a generator, because kind, rate, stream and
                    // octaves describe the *source* rather than the
                    // attachment.
                    noise: None,
                }),
            }],
            "an attachment landed on another node, another deck slot or another shape \
             than the one it named"
        );

        let taken = records(written(
            &Operation::TakeParamBack {
                deck: 3,
                param: karakuri_operation::BindAt {
                    layer: karakuri_operation::Layer::L1,
                    index: Some(2),
                    key: "turbulence".to_string(),
                },
            },
            &Current::default(),
        ));
        assert_eq!(
            taken,
            vec![Record::Source {
                slot: DeckSlot(3),
                layer: karakuri_store::record::Layer::L1,
                index: Some(2),
                key: "turbulence".to_string(),
                source: None,
            }],
            "a take-back is not the attachment's record with its attachment absent"
        );

        // **The address crosses whole, and a wildcard stays a wildcard.** A
        // binding with no index is the layer's — every node of it declaring
        // the key — and an `index` invented here would narrow it to one node
        // silently.
        let wild = records(written(
            &Operation::TakeParamBack {
                deck: 0,
                param: karakuri_operation::BindAt {
                    layer: karakuri_operation::Layer::L4,
                    index: None,
                    key: "exposure".to_string(),
                },
            },
            &Current::default(),
        ));
        assert_eq!(
            wild,
            vec![Record::Source {
                slot: DeckSlot(0),
                layer: karakuri_store::record::Layer::L4,
                index: None,
                key: "exposure".to_string(),
                source: None,
            }]
        );
    }

    /// **A knob turn writes a record, and it needs no reading either.**
    ///
    /// This operation answered `Silent(NoRecord)` until
    /// `docs/adr/0280-a-parameter-written-to-a-live-set-is-a-session-record.md`,
    /// and the reason given was never that a knob is unworthy of one: it was
    /// that `Record::Param` is a Set file's and has no `slot`. So what is
    /// asserted here is the deck arriving in the record — a write on slot 2
    /// that came back saying nothing about slot 2 would be the whole defect
    /// back again.
    #[test]
    fn a_knob_turn_carries_the_deck_the_set_is_playing_in() {
        assert_eq!(
            records(written(
                &Operation::WriteParam {
                    deck: 2,
                    param: karakuri_operation::ParamAt {
                        node: Some(karakuri_operation::NodeAddress {
                            layer: karakuri_operation::Layer::L4,
                            index: 1,
                        }),
                        key: "glow.x".to_string(),
                    },
                    value: karakuri_operation::ParamValue::Scalar(0.4),
                },
                &Current::default()
            )),
            vec![Record::Ride {
                slot: DeckSlot(2),
                at: Some(karakuri_store::record::NodeAddress {
                    layer: karakuri_store::record::Layer::L4,
                    index: 1,
                }),
                key: "glow.x".to_string(),
                value: karakuri_store::record::Value::Scalar(0.4),
            }],
            "a write landed on another node, another deck slot or another value \
             than the one it named"
        );
    }

    /// **A bare key crosses as an absence and never as an invented address.**
    ///
    /// `ParamAt::node` is `None` for a wildcard, which means it names no layer
    /// either — and `Record::Ride`'s `at` is the same `Option`, which is the
    /// whole reason that record carries a `NodeAddress` rather than
    /// `Record::Param`'s `layer` beside an `index`. A conversion that filled a
    /// layer in here would be writing a placeholder that a reader then has to
    /// be told to ignore, which is exactly the wart `Record::Param` is still
    /// living with.
    ///
    /// **And a wide value crosses whole.** A `vec3` is one line a person or a
    /// model writes and the reader with the Set in hand expands it, on
    /// `param`'s terms exactly (ADR-0268); nothing here invents `glow.x`.
    #[test]
    fn a_wildcard_write_names_no_node_and_therefore_no_layer() {
        assert_eq!(
            records(written(
                &Operation::WriteParam {
                    deck: 0,
                    param: karakuri_operation::ParamAt {
                        node: None,
                        key: "glow".to_string(),
                    },
                    value: karakuri_operation::ParamValue::Vec3([0.4, 0.7, 1.0]),
                },
                &Current::default()
            )),
            vec![Record::Ride {
                slot: DeckSlot(0),
                at: None,
                key: "glow".to_string(),
                value: karakuri_store::record::Value::Vec3([0.4, 0.7, 1.0]),
            }],
            "a bare key came back addressed, or a wide value came back expanded"
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

    /// **A free-running tempo being stated**, which closes the gap P-0090
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
            // And a front shape that is neither the mask fixture's nor the
            // first in the list, for the same reason: a wipe that took its
            // shape off the deck instead of off the settings is visible here
            // rather than coincidentally right.
            wipe_kind: karakuri_operation::WipeKind::Linear,
            wipe_angle: 0.75,
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
                slot: DeckSlot(2),
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
                    slot: DeckSlot(1),
                    value: 0.0,
                },
                Record::Residency {
                    slot: DeckSlot(1),
                    level: "live".to_string(),
                },
                Record::Transition {
                    slot: DeckSlot(0),
                    control: "opacity".to_string(),
                    to: 0.0,
                    start: 37.0,
                    beats: 6.0,
                    curve: "smooth".to_string(),
                },
                Record::Transition {
                    slot: DeckSlot(1),
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
            slot: DeckSlot(3),
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
                // The front shape a wipe would take, which a selection has no
                // front to apply it to: the fill is here so that this fixture
                // differs from the last one in the two fields the assertion is
                // about and in nothing else.
                ..transition()
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
                ..transition()
            }),
            ..Current::default()
        };
        assert_eq!(
            records(written(&Operation::FadeDeck { deck: 1, to: 1.0 }, &current)),
            vec![Record::Transition {
                slot: DeckSlot(1),
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

    /// **A wipe is six records, in the order the picture needs**, and this is
    /// the whole of what settling that conversion decided: the shape its front
    /// takes is the transition row's and arrives beside the instant and the
    /// length, and the soft edge is read off the mask that is running.
    ///
    /// **Every field is asserted against a fixture nothing else here is**, so
    /// a value taken from the wrong side is visible: the shape and the angle
    /// are [`transition`]'s and *not* [`mask`]'s, and the softness is
    /// [`mask`]'s and is on no surface at all. A conversion that read the
    /// shape off the deck would write `radial` at 1.25 here, which is the
    /// losing answer spelled out as a failure.
    ///
    /// **The two `Record::Mask` are not one**, and the first is not
    /// redundant: it is `SetMaskShape`'s record — the shape asked for and the
    /// front left where the deck had it — and the second is
    /// `SetMaskPosition`'s, restating that shape with the front at 0. That is
    /// the pair `karakuri-cli`'s `c` wrote through two `operate` calls, and
    /// what this holds is that the change of route did not change a record.
    #[test]
    fn a_wipe_is_a_mask_at_the_front_and_one_move_carrying_it_across() {
        let current = Current {
            transition: Some(transition()),
            mask: Some(mask()),
            mix: Some(mix()),
            ..Current::default()
        };
        assert_eq!(
            records(written(&Operation::Wipe { from: 3, to: 1 }, &current)),
            vec![
                Record::Mask {
                    slot: DeckSlot(1),
                    kind: "linear".to_string(),
                    angle: 0.75,
                    position: 0.4,
                    softness: 0.02,
                },
                Record::Mask {
                    slot: DeckSlot(1),
                    kind: "linear".to_string(),
                    angle: 0.75,
                    position: 0.0,
                    softness: 0.02,
                },
                Record::Opacity {
                    slot: DeckSlot(1),
                    value: 1.0,
                },
                Record::Blend {
                    slot: DeckSlot(1),
                    mode: "over".to_string(),
                },
                Record::Residency {
                    slot: DeckSlot(1),
                    level: "live".to_string(),
                },
                Record::Transition {
                    slot: DeckSlot(1),
                    control: "mask".to_string(),
                    to: 1.0,
                    start: 37.0,
                    beats: 6.0,
                    curve: "smooth".to_string(),
                },
            ],
            "the six records a wipe writes are not the mask, the front, the opacity, \
             the blend, the put-on-air and the move — in that order, about the deck \
             arriving, with the shape off the transition row and the soft edge off the \
             deck"
        );
    }

    /// **The deck being covered is read for nothing**, which is what makes a
    /// two-deck operation write about one of them.
    ///
    /// Everything a wipe writes is the arriving deck's: the covered one is
    /// revealed away from rather than moved, and a record naming it would be a
    /// change to a deck the gesture does not touch.
    #[test]
    fn a_wipe_writes_about_the_deck_arriving_and_never_the_one_covered() {
        let current = Current {
            transition: Some(transition()),
            mask: Some(mask()),
            mix: Some(mix()),
            ..Current::default()
        };
        for record in records(written(&Operation::Wipe { from: 3, to: 1 }, &current)) {
            let slot = match record {
                Record::Mask { slot, .. }
                | Record::Opacity { slot, .. }
                | Record::Blend { slot, .. }
                | Record::Residency { slot, .. }
                | Record::Transition { slot, .. } => slot,
                other => panic!("a wipe wrote {other:?}, which is not one of its six"),
            };
            assert_eq!(
                slot,
                DeckSlot(1),
                "a wipe wrote a record about slot {slot} — it names two decks and \
                 writes about the one arriving"
            );
        }
    }

    /// **A wipe with the settings but no mask is told it is the mask**, which
    /// is the second of its two readings and the one that is read off the
    /// deck.
    ///
    /// The softness is the value at stake: no operation names one, so a
    /// conversion with no mask in front of it would have to invent a soft
    /// edge for a front somebody else chose — which is
    /// [`a_mask_that_was_not_read_is_owed_rather_than_defaulted`] arriving at
    /// the gesture that moves the front rather than at the two that set it.
    #[test]
    fn a_wipe_with_no_mask_read_is_owed_the_soft_edge_rather_than_given_one() {
        let current = Current {
            transition: Some(transition()),
            ..Current::default()
        };
        assert_eq!(
            written(&Operation::Wipe { from: 0, to: 1 }, &current),
            Written::Owed(Owed::NotRead(Reading::Mask)),
            "a wipe with no mask read came back with something other than the reading \
             it is missing — a default soft edge is a value nobody asked about, written \
             over one somebody may have"
        );
    }

    /// **A wipe onto a deck that is already there writes neither the blend
    /// mode nor the put-on-air**, which is the affordance `m` in front of `c`
    /// is, said as a test.
    ///
    /// The mode is the operator's: a wipe under `max` — or under `add` — is a
    /// wipe *on* rather than a wipe *over*, a different picture and a
    /// legitimate one, and a gesture that forced `over` every time would take
    /// it back from the hand that chose it
    /// ([P-0094](../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)).
    /// The put-on-air is the same shape with nothing at stake but the byte: a
    /// deck already live is told so again.
    ///
    /// **Four records rather than six, in the same order.** What the list
    /// drops it drops from the middle, and the front is still at 0 before the
    /// move that carries it across — which is the sentence
    /// [`Written::Records`] gained when the wipe stopped being one length.
    #[test]
    fn a_wipe_leaves_a_mode_the_operator_chose_and_a_deck_already_on_air() {
        let current = Current {
            transition: Some(transition()),
            mask: Some(mask()),
            mix: Some(Mix {
                blend: karakuri_operation::BlendMode::Max,
                residency: karakuri_operation::Residency::Live,
            }),
            ..Current::default()
        };
        let written = records(written(&Operation::Wipe { from: 3, to: 1 }, &current));
        assert!(
            !written
                .iter()
                .any(|record| matches!(record, Record::Blend { .. } | Record::Residency { .. })),
            "a wipe onto a deck already under a mode the operator chose and already \
             live wrote a blend or a residency anyway — `m` in front of `c` means \
             nothing if the gesture writes `over` over it: {written:?}"
        );
        assert_eq!(
            written,
            vec![
                Record::Mask {
                    slot: DeckSlot(1),
                    kind: "linear".to_string(),
                    angle: 0.75,
                    position: 0.4,
                    softness: 0.02,
                },
                Record::Mask {
                    slot: DeckSlot(1),
                    kind: "linear".to_string(),
                    angle: 0.75,
                    position: 0.0,
                    softness: 0.02,
                },
                Record::Opacity {
                    slot: DeckSlot(1),
                    value: 1.0,
                },
                Record::Transition {
                    slot: DeckSlot(1),
                    control: "mask".to_string(),
                    to: 1.0,
                    start: 37.0,
                    beats: 6.0,
                    curve: "smooth".to_string(),
                },
            ],
            "a wipe that leaves the mix alone is the mask, the front at 0, the opacity \
             and the move — in the order the six are in, with the two that change \
             nothing missing rather than the rest reordered"
        );
    }

    /// **A deck already under `over` is told it is live and nothing else**,
    /// which is the pair one at a time rather than together.
    ///
    /// The two conditions are independent and this is what says so: a deck
    /// wearing the mode the wipe wants but sitting off air needs the
    /// put-on-air and nothing else. Five records, and the one that is missing
    /// is the one that would have restated a mode.
    #[test]
    fn a_wipe_writes_the_put_on_air_alone_for_a_deck_already_under_over() {
        let current = Current {
            transition: Some(transition()),
            mask: Some(mask()),
            mix: Some(Mix {
                blend: karakuri_operation::BlendMode::Over,
                residency: karakuri_operation::Residency::Priming,
            }),
            ..Current::default()
        };
        let written = records(written(&Operation::Wipe { from: 3, to: 1 }, &current));
        assert_eq!(
            written.len(),
            5,
            "a wipe onto a deck already under `over` and not yet live is five records \
             — the two conditions are separate and one of them fired: {written:?}"
        );
        assert_eq!(
            written[3],
            Record::Residency {
                slot: DeckSlot(1),
                level: "live".to_string(),
            },
            "the record a wipe writes for a deck already under `over` is not the \
             put-on-air, or it is not where the order puts it: {written:?}"
        );
    }

    /// **A wipe with no mix read is owed it rather than given the records it
    /// would have left out**, which is [`Current`]'s every-field-optional rule
    /// meeting the one reading taken so that a record can be omitted.
    ///
    /// The failure this catches is quiet in a way the other four are not: a
    /// default of *the deck is at `add` and off air* is a perfectly plausible
    /// `Mix` and a wipe built on it writes six perfectly plausible records —
    /// one of which is a blend mode nobody chose, written over one somebody
    /// did. There is nothing in the stream afterwards that says a reading was
    /// missing, which is why this answers rather than assumes.
    #[test]
    fn a_wipe_with_no_mix_read_is_owed_it_rather_than_writing_over_a_chosen_mode() {
        let current = Current {
            transition: Some(transition()),
            mask: Some(mask()),
            ..Current::default()
        };
        assert_eq!(
            written(&Operation::Wipe { from: 0, to: 1 }, &current),
            Written::Owed(Owed::NotRead(Reading::Mix)),
            "a wipe with the settings and the mask but no mix read came back with \
             something other than the reading it is missing — a default here is a \
             `blend over` for a deck the operator may have put under `add`"
        );
        assert_ne!(
            Owed::NotRead(Reading::Mix).why(),
            Owed::NotRead(Reading::Mask).why(),
            "the two readings a wipe takes off the deck it names say the same sentence, \
             so a caller is told which of them to go and read only by luck"
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
    /// four are asserted, because the failure is the conversion's and not one
    /// operation's.
    ///
    /// **The wipe is the fourth and is the one that could answer two
    /// things.** It reads the settings and the mask, and with neither handed
    /// in it names the settings — the reading a caller is holding rather than
    /// one it would have had to look up.
    #[test]
    fn a_surface_that_handed_in_no_settings_is_told_which_reading_it_forgot() {
        for operation in [
            Operation::FadeDeck { deck: 1, to: 0.0 },
            Operation::Crossfade { from: 0, to: 1 },
            Operation::SelectRenderer {
                deck: 0,
                renderer: 1,
            },
            Operation::Wipe { from: 0, to: 1 },
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

    /// **Editing a pattern writes a file's worth of nothing, exactly as
    /// keeping an arrangement does.**
    ///
    /// The five answered `Owed(Undecided)` until 2026-09-09, and the objection
    /// at the arm was that `Silent::Surface` *"would call a pattern the
    /// console's own state, where ADR-0227 makes it library data under the
    /// store."* The test above is the refutation: an arrangement is library
    /// data under the store on the same terms and answers `Silent(Surface)`,
    /// and the sentence that pins it transfers word for word. So this asserts
    /// the second family of the same kind, all five together, because what
    /// makes the answer right is that they are one family — a step, a mute, a
    /// target, a mode and a bank are five edits to one pattern.
    ///
    /// **What a lane *does* is not silent and is not asserted here**: a lane
    /// emits `Operation::SetOpacity` and `Operation::WriteParam`, whose
    /// records are `Record::Opacity` and `Record::Ride`, and those are what a
    /// replay reads back (ADR-0322).
    #[test]
    fn editing_a_pattern_writes_a_file_and_no_record() {
        for operation in [
            Operation::SetStep {
                pattern: 0,
                lane: 0,
                step: 4,
                on: true,
            },
            Operation::SetLaneMute {
                pattern: 0,
                lane: 0,
                muted: true,
            },
            Operation::PointLane {
                pattern: 0,
                target: karakuri_operation::LaneTarget::Fader { deck: 0 },
            },
            Operation::SetPatternGrid {
                pattern: 0,
                grid: karakuri_operation::StepMode::Eighth,
            },
            Operation::SelectPattern { pattern: 1 },
        ] {
            assert_eq!(
                written(&operation, &Current::default()),
                Written::Silent(Silent::Surface),
                "`{operation:?}` did not answer `Silent(Surface)`. A pattern is library data \
                 under the store on the arrangement's terms (ADR-0227, ADR-0320) and what a \
                 lane does reaches the stream as its own writes (ADR-0322) — so editing one \
                 writes a file and no record, which is the arrangement family's sentence and \
                 not a widening of this arm"
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
            written(&Operation::TapBeat, &Current::default()),
            Written::Owed(Owed::NotSettled),
            "a tap moves the beat tracker and cannot be written here — saying it is \
             silent would lose a tap an operator asked for"
        );
        assert_eq!(
            written(
                &Operation::MoveBoundary {
                    boundary: karakuri_operation::Undecided
                },
                &Current::default()
            ),
            Written::Owed(Owed::Undecided),
            "moving a boundary is the vocabulary's own open question and not this crate's"
        );
    }

    /// **A walk asks and changes nothing**, and it answers here rather than in
    /// `Owed(Undecided)` because the payload it was waiting for arrived: it
    /// names the Set it is a walk of
    /// (`docs/adr/0342-a-walk-names-the-set-it-is-of-and-the-two-rows-beside-it-are-gap.md`).
    ///
    /// **Landing is the other row and is not silent in this way.**
    /// `Operation::RestoreProcedure` is `Silent(OnLanding)` — a procedure
    /// change whose `Record::Procedure` is written where the swap lands — so a
    /// walk answering `Question` cannot be read as the store's history being
    /// outside the stream.
    #[test]
    fn a_walk_asks_and_a_landing_writes() {
        for set in [None, Some("night01".to_string())] {
            assert_eq!(
                written(&Operation::WalkHistory { set }, &Current::default()),
                Written::Silent(Silent::Question),
                "walking one Set's versions is a listing of the store: it opens no file, \
                 moves no deck, and a question writes no record"
            );
        }
        assert_eq!(
            written(
                &Operation::RestoreProcedure {
                    deck: 0,
                    revision: karakuri_operation::Revision::Picked(
                        "20260908-143052-271_slot0_L4_beat_strokes".to_string()
                    ),
                },
                &Current::default()
            ),
            Written::Silent(Silent::OnLanding),
            "landing a version is a procedure change and its record is written where the \
             swap lands — the walk is the listing it was picked out of"
        );
    }
}
