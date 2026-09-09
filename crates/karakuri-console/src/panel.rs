//! The panel's model: the arrangement, the drag in progress, and the
//! operations, each of which names what it acts on ([`Op`]).
//!
//! This is what a view drives. It holds a [`Layout`] and a pointer, and
//! nothing else: no window, no device, no toolkit. Everything here runs on a
//! machine with no graphics adapter, which is what lets *what a pointer does
//! to a divider* be answered by a test rather than by looking at it.
//!
//! # An operation returns what happened, not a line to print
//!
//! [`press`](Panel::press), [`moved`](Panel::moved),
//! [`released`](Panel::released) and [`op`](Panel::op) each return a value
//! saying what they did — [`Pressed`], [`Dragged`], [`Released`],
//! [`Outcome`] — and none of them formats a sentence. That is the one thing
//! the model deliberately does not do: `examples/layout.rs` turns a
//! [`Dragged`] into the line it prints, an egui view turns the same value into
//! whatever it draws, and neither has to parse the other's English. It is also
//! what lets a test assert that a stop held a drag, rather than counting the
//! strings a drag produced.
//!
//! # Nothing here shadows the layout
//!
//! Every question [`Layout`] can answer is asked of it, and it now answers
//! every one this file used to answer for itself. A folded region is
//! [`Layout::is_collapsed`], a solo is [`Layout::soloed`], the split enclosing
//! a region is [`Layout::parent`], the children a divider index counts are
//! [`Layout::placed_children`], where a boundary is is [`Layout::boundary`],
//! every boundary is [`Layout::boundaries`], and where a drag landed is what
//! [`Layout::set_divider`] returned.
//!
//! **Nothing here stores a fact about the arrangement.** [`Node`] is the tree
//! flattened for a caller that iterates it, and the only thing it carries that
//! the arena does not is how deep the flattening got.
//!
//! The two things held across events are the pointer and the drag — what is in
//! hand and where along it the pointer took hold. Both are the pointer's state
//! and not the layout's.
//!
//! # Three kinds of drag, and one thing in hand
//!
//! A boundary is one of them; a mixer strip's fader is the second; a Set
//! carried out of the Library bay is the third, and they are **one `Option`**
//! ([`Drag`], private) rather than three. That is not tidiness:
//! [`crate::input`]'s rule 1 — *a drag in hand keeps its claim, wherever the
//! pointer has wandered to* — asks [`dragging`](Panel::dragging), and a second
//! `Option` beside the first would be four states with two of them impossible
//! and one rule that had to remember to ask about both. One thing in hand
//! means rule 1 covers a fader by construction on the day it is written, and
//! it covered the carry the same way.
//!
//! **What the three disagree about is what a drag moves.** A boundary drag
//! moves the arrangement, which is this crate's, so [`moved`](Panel::moved)
//! writes it and reports where it landed — **including a fold**: a pane pulled
//! out past its own minimum is closed, and one whose closed edge is pulled
//! back in is opened, both performed here and reported as
//! [`Dragged::Pane`] (ADR-0300). A fader drag moves the **engine's**
//! value, which this crate does not have and cannot reach (ADR-0156) — so it
//! writes nothing at all and reports a [`karakuri_operation::Operation`],
//! which is the whole of what a GUI component is for: pointer motion into a
//! number, sent to a target, and the value read back and drawn (ADR-0180).
//! Nothing here remembers what the value became. The strip is drawn from what
//! the deck says on the next frame, and a number kept here would be a second
//! copy of the deck's state.
//!
//! **A carry moves nothing at all until it is let go**, and it is the one of
//! the three whose *destination* is part of what it asks for. A fader knows
//! which deck at the press — the knob is on a strip — so
//! [`Released::Let`] carries no value and no target. A carry knows only what
//! it picked up; which deck it lands on is whatever strip the pointer is over
//! when the button comes up, so [`released`](Panel::released) is handed that
//! answer the way [`grab`](Panel::grab) is handed a [`Grab`]: derived by
//! [`crate::view`], which is where a strip's geometry is, and never
//! re-derived here. See [`Released::Dropped`] and [`Released::Nowhere`].
//!
//! # Solve once, then read
//!
//! [`Layout::rect`], [`Layout::hit`] and [`Layout::boundary`] each carry a
//! `debug_assert!` that the layout is not dirty, so an operation followed by a
//! read is a panic in a debug build. Everything here that reads calls [`solve`](Panel::solve)
//! first, which on a frame where nothing moved is a flag test, and every
//! operation leaves the layout clean behind it.

use karakuri_layout::{Axis, Hit, Layout, NodeId, Point, Rect};
use karakuri_operation::{Bloom, Cut, Feedback, Operation, ParamAt, ParamValue, RgbShift};

/// How far either side of a boundary still grabs it. Wider than any divider
/// the console draws, which is [`Layout::hit`]'s whole argument for taking a
/// grab at all: a 9px gap is not a target a hand finds.
pub const GRAB: f32 = 6.0;

/// How far short of the ask a boundary has to land before a **stop** is what
/// put it there, in pixels.
///
/// A drag that nothing stops still does not land exactly where it was asked:
/// [`Layout::set_divider`] writes a size — or, for a flexible pair, a weight
/// chosen to reproduce one — and reads the position back out of the next
/// solve, so a landing is a few ten-thousandths of a pixel off its ask by
/// arithmetic alone. This is the line between that and a constraint, and there
/// is nothing between the two to be careful about: it is two orders of
/// magnitude above single-precision noise at these coordinates and an order
/// below anything an eye or a display can resolve, so a `held` is a stop and
/// nothing else.
const STOPPED: f32 = 0.05;

/// **How far a drag has to go on past a stop before it is pulling a region out
/// through its own edge**, rather than leaning on a boundary that will not
/// move, in pixels.
///
/// [`STOPPED`] is where *held* begins and this is where *pulling* does, and
/// there is a real difference between them: a boundary against its stop is the
/// ordinary end of a drag and an operator meets it every time they make a pane
/// as narrow as it goes. Closing the pane at the first hundredth of a pixel
/// past that would make the fold an accident of where the drag ended.
///
/// **[`GRAB`], rather than a number chosen here.** It is this console's one
/// statement of how far from a boundary a hand is still on it, and past it the
/// pointer is further from the boundary it is holding than the distance at
/// which it could have taken hold of it at all — so it is no longer pressing
/// on the boundary, it is pulling the region out from behind it. A second
/// number would be a second answer to a question the panel has already
/// answered, which is
/// [ADR-0295](../../../docs/adr/0295-the-grip-is-the-fold-and-a-panes-outer-edge-is-the-other-one.md)'s
/// own argument for `GRAB` one control along.
///
/// It reads the same both ways: **`GRAB` past the stop closes the pane, and
/// `GRAB` in from its closed edge opens it again.**
const PULLED_THROUGH: f32 = GRAB;

/// How far the boundary has to have moved since the last thing this drag said
/// before it is worth saying anything again, in pixels.
///
/// **A drag reports what the layout did, not what the pointer asked**, and at
/// sixty asks a second the difference is a readout a person can follow against
/// one they cannot. Half a pixel is the smallest move that can change what is
/// drawn — below it the boundary is on the same physical pixel it was — so
/// this reports every move a viewer could see and none that they could not. A
/// stop starting or stopping to hold the drag is reported whatever the
/// distance: that is a change in *what is happening*, not in where the
/// boundary is.
const WORTH_SAYING: f32 = 0.5;

/// One node of the arrangement, flattened in tree order.
///
/// **Two facts, and both of them are about the flattening rather than about
/// the node.** Everything else — whether it is a view, what encloses it, where
/// it solved to — is [`Layout`]'s to answer and is asked of it by `id`. This
/// carried a `parent` until [`Layout::parent`] existed, which is the shape of
/// the mistake: a second copy of the arena, rebuilt whenever the layout was
/// replaced, and wrong the day the arena can insert or remove.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Node {
    pub id: NodeId,
    /// How deep in the tree, for a caller that indents. The one thing tree
    /// order costs to work out and the arena does not store.
    pub depth: usize,
}

/// **What the pointer has hold of**, and there is exactly one of it — see the
/// module documentation for why this is one `Option` on [`Panel`] and not two.
enum Drag {
    Boundary(Boundary),
    Fader(Fading),
    Carry(Carrying),
}

/// A boundary in hand: which one, and where along it the pointer took hold.
struct Boundary {
    split: NodeId,
    index: usize,
    axis: Axis,
    /// Pointer coordinate minus the boundary's, at the moment of the press.
    /// Subtracted from every later coordinate so the boundary does not jump to
    /// the pointer on the first move.
    offset: f32,
    /// Where the boundary was when this drag last reported anything, and
    /// whether what it reported was that a stop was holding it.
    ///
    /// **A drag is reported when the boundary moves, not when the pointer
    /// does.** A pointer dragged on past a stop asks for a new position sixty
    /// times a second and the boundary does not move for any of them; a report
    /// per ask is a flood that says the same thing every time, and it is worst
    /// exactly where a person is looking hardest. So a stop is reported once,
    /// and the next [`Dragged`] is the one where the boundary moves again.
    said: Option<f32>,
    held: bool,
    /// **Whether this gesture has already closed a pane or opened one.**
    ///
    /// One drag asks for one of them. Without this the two conditions are each
    /// other's inverse the instant either fires — a pane closed at `GRAB` past
    /// its stop puts its own edge under a pointer that is `GRAB` the other
    /// side of it, which is the open condition, met on the very next move —
    /// and the pane would flicker at sixty asks a second for as long as the
    /// button was down.
    ///
    /// **So it is once per gesture, and the way back is another gesture.**
    /// That is [`Fading::said`]'s rule about a drag that has already said what
    /// it had to say, on a thing that happens once rather than on a value.
    acted: bool,
}

/// A fader in hand: the control, its track and the grab — and **the last value
/// this drag asked for**.
struct Fading {
    grab: Grab,
    /// What this gesture last emitted, or `None` before it has emitted
    /// anything.
    ///
    /// **It is what was *said*, not what the deck is at**, exactly as
    /// [`Boundary::said`] is. The difference matters and is the whole of what
    /// keeps this crate off the engine's state: nothing reads this to draw
    /// anything, and nothing hands it back out. It exists so that a pointer
    /// dragged past the end of a track — which asks for 1.0 sixty times a
    /// second and moves nothing — emits one operation and then none, which is
    /// what lets [`crate::repaint`] answer that a drag which changed no value
    /// is owed no frame. The seed is the same rule at the other end of the
    /// gesture: a press that moves nothing emits nothing.
    ///
    /// A boundary needs [`WORTH_SAYING`] here and a fader does not: half a
    /// pixel is a threshold on a *position*, and this is an exact comparison
    /// of the value that would be sent. Every value that differs is a real
    /// change to the mix, however small.
    said: Option<f32>,
}

/// **A Set in hand**, on its way from a Library row to a mixer strip.
///
/// # It is a payload, where the other two are a control
///
/// [`Boundary`] and [`Fading`] each name **which control** the hand took hold
/// of, resolved at the press, and every later event asks that control what the
/// pointer now means. This names neither a control nor a place: what a carry
/// holds is *the thing being carried*, and the control it is going to is not
/// known until the button comes up.
///
/// **So the payload is a `String` and this is the one drag that is not
/// `Copy`.** A Set is named by whatever the store called it — the row the host
/// handed [`crate::view::View::library`] — and `karakuri-operation` names one
/// the same way, because it is a leaf crate that cannot say anything narrower
/// (`Operation::LoadSet`). That is what takes [`Released`] out of `Copy` with
/// it, and it stops there: nothing else in this module gains a payload, and
/// [`Pressed`] never sees a carry at all — [`Panel::carry`] is
/// [`Panel::grab`]'s door and reports nothing back.
///
/// **Nothing is remembered about where it came from.** The row it was taken
/// off is [`crate::view::View`]'s cursor, which the press moves and which
/// outlives the gesture; a copy of it here would be a second answer to *which
/// row is the cursor on* for as long as the button is down.
struct Carrying {
    /// Which Set, by the name the listing carried.
    set: String,
}

/// **Which fader a hand has hold of**, named after the operation each of them
/// is a translator for.
///
/// The strip's two are two controls and not one, because `karakuri-engine`'s
/// `mix.rs` says they are two things: *opacity at zero silences under every
/// blend mode, and gain at zero does not silence `over`*. See
/// [`crate::view::Strip::gain`].
///
/// # The deck is carried here, and [`Knob::Out`] is why
///
/// It was an argument beside this — `operation(deck, value)` — and a `deck`
/// beside a knob that has none would be a slot number invented to satisfy a
/// signature. The master out is one level on the whole fold rather than on a
/// member of it (`karakuri_engine::deck::Deck::set_out`, *"Not per slot"*), so
/// **which deck** is part of what a knob *is* rather than something every
/// knob has. A variant that carries it and one that does not is the compiler
/// holding that, where an `Option<u8>` would leave `Trim` free to be `None`.
#[derive(Debug, Clone, PartialEq)]
pub enum Knob {
    /// The trim — the mock's `.trim .fader`, lying down —
    /// [`Operation::SetGain`].
    Trim {
        /// Which deck's, which is the manual's word for what the code calls a
        /// slot — the vocabulary's own choice, recorded in ADR-0180: *"The
        /// manual's deck is the code's slot … the vocabulary takes the
        /// manual's word, since the manual is its specification."*
        deck: u8,
    },
    /// The fader — `.vfader`, standing up — [`Operation::SetOpacity`].
    Fader {
        /// Which deck's, on [`Knob::Trim`]'s terms.
        deck: u8,
    },
    /// **The master out** — the Master bay's `.fader`, lying down, and the one
    /// knob on this panel that names no deck: it is the level the composited
    /// frame leaves the mix at, at the entry to the master chain
    /// ([`Operation::SetMasterOut`], and
    /// `docs/adr/0224-out-and-exposure-are-two-levels-that-multiply-in-different-places.md`
    /// for why it is not the tone mapper's exposure).
    Out,
    /// **The feedback pass's amount** — the first of the master chain's three
    /// rows, and the one that carries a second value.
    ///
    /// `cut` is not what this knob moves: it is what the row is *at*, carried
    /// so that the operation a drag asks for is the whole of what the pass is
    /// set to. [`Knob::Param`] carries its range for the same reason — a
    /// track position is not an operation until something beside it says what
    /// it means.
    Feedback {
        /// Which cut of the previous frame the pass is reading, from the row
        /// this knob was laid out from.
        cut: Cut,
    },
    /// **The bloom pass's amount** — the chain's second row.
    Bloom,
    /// **The rgb shift pass's amount** — the chain's third row.
    RgbShift,
    /// **A published parameter's fader**, in an Inspector pane —
    /// [`Operation::WriteParam`]. It is the one knob that names something
    /// inside a Set rather than a level on it, which is why it carries a name
    /// and this enum is no longer `Copy`: a parameter is addressed by name,
    /// which is ADR-0280's own reason for `mix::Change`, and the address is
    /// `karakuri-operation`'s so there is one spelling of it.
    Param {
        /// Which deck's, on [`Knob::Trim`]'s terms.
        deck: u8,
        /// The control the interface published. A `None` node is a wildcard,
        /// and `Set::write_param` is where ADR-0223's authority refusal meets
        /// it.
        param: ParamAt,
        /// What it was published over. [`Knob::operation`]'s `value` is a
        /// track position and this is what turns it into a number.
        range: [f32; 2],
    },
}

impl Knob {
    /// **The operation this knob asks for at `value`**, and the whole of the
    /// translation a fader performs.
    pub fn operation(&self, value: f32) -> Operation {
        match self {
            Knob::Trim { deck } => Operation::SetGain {
                deck: *deck,
                gain: value,
            },
            Knob::Fader { deck } => Operation::SetOpacity {
                deck: *deck,
                opacity: value,
            },
            Knob::Out => Operation::SetMasterOut { out: value },
            // **The whole of what the pass is set to, and never a step.** The
            // cut rides along because the amount alone is not a picture: the
            // same 0.5 is a one-frame echo under `mix` and a compounding trail
            // under `exit`.
            Knob::Feedback { cut } => Operation::SetFeedback {
                params: Feedback {
                    amount: value * Feedback::MAX,
                    cut: *cut,
                },
            },
            Knob::Bloom => Operation::SetBloom {
                params: Bloom { amount: value },
            },
            Knob::RgbShift => Operation::SetRgbShift {
                params: RgbShift { amount: value },
            },
            // `crate::view::Param::at` inverted, which is the relation
            // `Grab::value` has to `crate::view::filled` one field along.
            Knob::Param {
                deck,
                param,
                range: [low, high],
            } => Operation::WriteParam {
                deck: *deck,
                param: param.clone(),
                value: ParamValue::Scalar(low + (high - low) * value.clamp(0.0, 1.0)),
            },
        }
    }

    /// **Which deck this knob is on**, or `None` for the one that is on none.
    ///
    /// For a caller with a sentence to print. Nothing routes through it: the
    /// deck is inside the operation already, which is [`Dragged::Fader`]'s
    /// rule about one value and not two.
    pub fn deck(&self) -> Option<u8> {
        match self {
            Knob::Trim { deck } | Knob::Fader { deck } | Knob::Param { deck, .. } => Some(*deck),
            Knob::Out | Knob::Feedback { .. } | Knob::Bloom | Knob::RgbShift => None,
        }
    }
}

/// **A fader taken in hand**: which deck's which control, the track its value
/// rides, and where along the knob the pointer took hold.
///
/// # It is built by the view and held by the model, and that is the seam
///
/// A boundary's geometry is the layout's, so [`press`](Panel::press) hit-tests
/// it here and [`moved`](Panel::moved) re-reads it every move. **A fader's is
/// not.** Where a strip's tracks are is [`crate::view::mixer`]'s answer, it
/// depends on the values the harness handed in *and* on a text shaper for the
/// words in the same strip, and this module has neither and takes neither. So
/// the view resolves a press to one of these and the model takes it in hand —
/// the same shape [`crate::view::Outputs::op`] has, where the derivation that
/// claims a press is asked a second time to say what the press meant, rather
/// than copied.
///
/// The consequence is stated rather than hidden: **a drag maps the pointer
/// against the track it took hold of**, so a window resized under a fader that
/// is being held goes on using the track the press was made on until the
/// button comes up. A hand cannot do both at once, and the alternative — this
/// module re-deriving a strip's geometry — is the toolkit inside the model.
#[derive(Debug, Clone, PartialEq)]
pub struct Grab {
    knob: Knob,
    axis: Axis,
    /// The track's **zero** end along the axis: the left of a row, and the
    /// *bottom* of a column, because a fader stands up.
    zero: f32,
    /// How far the knob's centre travels between the ends. Positive.
    travel: f32,
    /// Pointer coordinate less the knob's centre, at the moment of the press.
    /// Subtracted from every later coordinate so **the value does not jump to
    /// the pointer** on the first move — [`Boundary::offset`]'s rule on a
    /// value instead of on a position.
    offset: f32,
}

impl Grab {
    /// A fader taken hold of. `travel` is the length of track the knob's
    /// centre moves along, and a track with none is refused: a control with
    /// nowhere to go is not one a hand has hold of.
    pub fn new(knob: Knob, axis: Axis, zero: f32, travel: f32, offset: f32) -> Option<Grab> {
        match travel > 0.0 {
            true => Some(Grab {
                knob,
                axis,
                zero,
                travel,
                offset,
            }),
            false => None,
        }
    }

    /// Which control this is, and which deck's where it is a deck's.
    pub fn knob(&self) -> Knob {
        self.knob.clone()
    }

    /// **What a pointer at `coord` puts this fader at**, on `[0, 1]`.
    ///
    /// `crate::view`'s `filled` inverted, and it runs the same two ways: a row
    /// fills from the left and a column from the **bottom**.
    ///
    /// # Both ends are exactly reachable
    ///
    /// A knob dragged to the floor gives exactly `0.0` and one dragged to the
    /// top exactly `1.0` — the subtraction is zero at one end and the division
    /// is `travel / travel` at the other, and a pointer past either end is
    /// clamped to it. That is `karakuri-midi`'s `GAIN_RANGE` argument reached
    /// from the other side: *"`[0, 1]` … chosen so that both ends of a fader
    /// are exact … a fader whose top is unity is what a fader means."*
    ///
    /// **A gain above 1.0 cannot be asked for here, and that is a known gap**
    /// rather than a ceiling: `Deck::set_gain` is deliberately unclamped
    /// because the mix is HDR, and ADR-0178 records that this control is drawn
    /// over `[0, 1]` and cannot show the difference between 1.0 and 3.0. The
    /// top of this drag is exactly unity, so what the gap costs is *reach*
    /// rather than *precision*.
    pub fn value(&self, coord: f32) -> f32 {
        let edge = coord - self.offset;
        unit(match self.axis {
            Axis::Row => (edge - self.zero) / self.travel,
            Axis::Column => (self.zero - edge) / self.travel,
        })
    }
}

/// **A value on `[0, 1]`, and a NaN is zero.**
///
/// `f32::clamp` passes a NaN straight through, which would be a `NaN`-wide
/// fill on a track and a `NaN` in an operation aimed at the mix. The engine
/// takes the same reading one level down — *"a fader whose value is not a
/// number is a broken control, and of the two available readings … only one of
/// them is a fader"* — and this is where a value that did not come through
/// `Deck::set_opacity` takes it: a reading handed to [`crate::view`] by a
/// caller, and a pointer coordinate handed to [`Grab::value`].
pub(crate) fn unit(at: f32) -> f32 {
    match at.is_nan() {
        true => 0.0,
        false => at.clamp(0.0, 1.0),
    }
}

/// What an operation acts on. Named as operations rather than as keys, so a
/// caller with no keyboard can ask for one.
///
/// # An operation carries what it acts on, and the pointer is a way of
/// choosing that
///
/// **Every operation here used to mean *the region under the pointer***:
/// `Fold` was *fold whatever [`Panel::cursor`] is over*, and the model
/// resolved the pointer itself, once per operation. That reads fine while the
/// keyboard is the only surface, and it is wrong as soon as anything else asks
/// — because *under the pointer* is not part of what folding **is**. It is one
/// way of naming which region to fold.
///
/// The console's Outputs row is the case that proves it. Its one control folds
/// `program-view` — the picture — whatever the pointer happens to be over,
/// because the pointer is over the control, at the other end of the panel. A
/// pointer-relative vocabulary has no way to say that: it would need a second
/// operation meaning *fold that one instead*, and then two names for one act.
///
/// So the four operations that act on a node take the node, and the four that
/// act on the arrangement as a whole take nothing:
///
/// - [`Fold`](Op::Fold), [`Unfold`](Op::Unfold),
///   [`FoldEnclosing`](Op::FoldEnclosing) and [`Solo`](Op::Solo) name a
///   [`NodeId`].
/// - [`UnfoldAll`](Op::UnfoldAll), [`Unsolo`](Op::Unsolo),
///   [`Reset`](Op::Reset) and [`Report`](Op::Report) have nothing to name.
///
/// **Resolving the pointer is the caller's**, and it is [`Panel::under`]: the
/// harness does it for a key press, and the Outputs control does not need it.
/// That also moves two answers out of [`Outcome`] and into the caller, where
/// they were always about the pointer rather than about the operation — *the
/// pointer is on a divider* and *there is nothing under the pointer* are
/// things a resolution finds, not things a fold reports.
///
/// # Folding and unfolding are not mirror images, and that is the point
///
/// [`Fold`](Op::Fold) folds one node and nothing else. [`Unfold`](Op::Unfold)
/// makes one node **visible**, which takes its collapsed ancestors with it and
/// a solo that is hiding it. **Hiding a thing hides one thing; showing a thing
/// means showing the way to it** — the ordinary shape of revealing a node in a
/// tree, and the reason the two are not each other's inverse.
///
/// The alternative is `expand(id)` and nothing else, and the console's one
/// control is what settles it against: with the Program bay folded around the
/// picture, the Outputs dot is dark, and a press that expanded the picture
/// inside a still-folded bay would light nothing, move nothing and say
/// nothing. `docs/manual/console.html` rules on exactly that shape, in the
/// note about this very row — *"Nothing is refused here, so nothing has to be
/// explained: a control that quietly declines the last of something is a rule
/// an operator can only find by experiment."* A control that appears not to
/// respond is that rule with no words at all.
///
/// [`UnfoldAll`](Op::UnfoldAll) keeps its own meaning and is not folded into
/// this: it is what reaches a fold **nobody has a name for**, and `Unfold`
/// undoes only what stands between one named node and the screen.
///
/// # Fold and unfold are two operations, and there is no toggle
///
/// A toggle is an affordance built **over** two operations by whoever draws it
/// — the Outputs dot asks for [`Fold`](Op::Fold) when the picture is on and
/// [`Unfold`](Op::Unfold) when it is off — and it is not one itself. The
/// vocabulary is what every surface routes into: a MIDI map with a button per
/// direction, an MCP call that says which one it wants, and a keyboard, all
/// have to be able to say *fold this* and mean it. `Fold` folded-or-unfolded
/// could never be asked for the direction it will take, which is the same
/// reason a `Toggle` variant is not here.
///
/// It cost nothing where the pointer is the surface: a folded region has no
/// rectangle, so the pointer could never be over one, so `f` on the keyboard
/// only ever folded. What changes is that a *named* target can be folded
/// already, and now the caller says which way it means.
///
/// # This is not `karakuri_operation::Operation`, and what keeps it apart is a
/// name
///
/// The vocabulary names the same five things — `FoldBay`, `FoldPane`,
/// `Unfold`, `Solo`, `MoveBoundary` — and names a region by `String`, because
/// it is a leaf crate with no dependencies and cannot say [`NodeId`]. A
/// `NodeId` has a private field and only a `Layout` hands one out, so the two
/// cannot be one type: a caller holding a layout has the handle, and a caller
/// with none has the name.
///
/// **These operations write no record**, so nothing downstream of this module
/// changes when they move — `karakuri-operation-record` answers
/// `Silent(Surface)` for all four. Routing into the vocabulary here would mean
/// the panel *accepting* a named region rather than emitting one, and the
/// survey that costed it is
/// [ADR-0197](../../../docs/adr/0197-the-consoles-op-stays-and-what-blocks-it-is-the-page-rather-than-the-code.md).
/// Three things are in the way and every one is a question about
/// `docs/manual/operations.html`: [`Reset`](Op::Reset) and
/// [`Report`](Op::Report) have no row on it, *Move a boundary* is a drag and
/// never an operation, and **two splits in this arrangement have no name** —
/// the root column and the body row — which a pointer reaches through
/// `Hit::Divider { split, .. }` and a `String` cannot say at all.
/// `tests/vocabulary.rs` asserts all three, both ways round.
///
/// [`FoldEnclosing`](Op::FoldEnclosing) was counted as a fourth, for naming
/// its target by relation where every row names one outright. It is not one:
/// naming a target by relation is the caller's and not the operation's — the
/// rule `Operation::Crossfade` states at its own variant, and the one
/// ADR-0175 applied here — so what is left on this side is
/// `Fold(self.layout.parent(id))`, the fold the page already specifies on a
/// node this model works out. **The page owes it no row.**
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    /// Fold this region away.
    Fold(NodeId),
    /// **Make this region visible**, undoing whatever stands between it and
    /// the screen: `id` itself, every collapsed ancestor of it, and a solo
    /// that is keeping it off screen.
    ///
    /// **Not the same as [`UnfoldAll`](Op::UnfoldAll)**: a sink turned back on
    /// brings back *that* picture and the way to it, and nothing else that
    /// happens to be folded. **Not the mirror of [`Fold`](Op::Fold)** either —
    /// see the note above.
    Unfold(NodeId),
    /// Fold the split enclosing this region.
    FoldEnclosing(NodeId),
    /// Unfold everything folded. A folded region has no rectangle, so the
    /// pointer cannot reach it to unfold it.
    UnfoldAll,
    /// Solo this region.
    Solo(NodeId),
    /// Undo the solo.
    Unsolo,
    /// A fresh arrangement, at the same viewport.
    Reset,
    /// Every node and where it solved to.
    ///
    /// **No surface binds a key to this, and none has since 2026-08-31.**
    /// `crates/karakuri/src/main.rs` bound `p` to it and printed the table;
    /// `docs/manual/operations.html` specifies `p` as half of *Nudge the
    /// latency offset*, a badge naming two keys with one of them bound would
    /// be a badge that lies, and a panel diagnostic asked what shortcut it
    /// needed had no answer worth the letter. So the letter went and the
    /// operation stayed.
    ///
    /// **What still reads it is this crate's own suite**, and neither test is
    /// a test *of* this variant:
    /// [`tests/panel.rs`](../../tests/panel.rs) asks for a report immediately
    /// after a solo with nothing solving in between, which is what catches
    /// [`Panel::op`] leaving a solve owed — it is the only operation that
    /// reads every rectangle, so it is the only one that can; and
    /// [`tests/repaint.rs`](../../tests/repaint.rs) asks it because
    /// [`Outcome::Report`] is the one outcome that carries a `Vec` and must
    /// still answer [`crate::repaint::Repaint::Never`], which is the case
    /// ADR-0164's still-panel clause is most exposed to.
    ///
    /// **And it is not the startup legend's table.** That one is each
    /// region's *min and max* — the constraints, printed once, before
    /// anything has been dragged. This is each region's solved rectangle and
    /// whether it is folded, at whatever moment it is asked. Nothing else in
    /// the system answers the second.
    ///
    /// `tests/vocabulary.rs` goes on pinning that no row on
    /// `docs/manual/operations.html` names it, which is permanent
    /// (ADR-0205) and is about the page rather than about a key.
    Report,
}

/// What a press found under the pointer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Pressed {
    /// A boundary is now in hand, and every later [`Panel::moved`] moves it.
    Grabbed {
        split: NodeId,
        index: usize,
        axis: Axis,
        /// Where the boundary was, along the split's axis.
        at: f32,
        /// Where along it the pointer took hold: the pointer's coordinate less
        /// the boundary's.
        offset: f32,
    },
    /// A divider the tree cannot produce a pair for, so there is nothing to
    /// drag. Nothing is in hand.
    NoPair { split: NodeId, index: usize },
    /// The interior of a region.
    Region { id: NodeId, rect: Rect },
    /// Outside the viewport, or somewhere no visible region claims.
    Nothing,
}

/// **What a move with something in hand did**, and the two arms are the two
/// kinds of drag — see the module documentation.
///
/// Produced only when something happened. For a boundary that is when it
/// moved, or when the stop holding it changed: a drag is reported when the
/// layout does something, not when the pointer does, so a pointer held against
/// a stop reports once and then nothing. For a fader it is when the **value**
/// changed, exactly: a pointer dragged on past the end of a track asks for the
/// end sixty times a second and the value is already there.
#[derive(Debug, Clone, PartialEq)]
pub enum Dragged {
    /// A boundary moved.
    Boundary {
        split: NodeId,
        index: usize,
        axis: Axis,
        /// Where the drag asked the boundary to go: the pointer's coordinate
        /// along the axis, less the offset it grabbed at.
        asked: f32,
        /// Where it went, which is what [`Layout::set_divider`] returned.
        landed: f32,
        /// `Some(by)` when a stop held it, where `by` is `landed - asked` —
        /// how far short of the ask, and which way. `None` when it landed
        /// where it was asked to.
        held: Option<f32>,
    },
    /// **A boundary drag closed a pane, or brought one back** — and the [`Op`]
    /// this module **performed**, rather than one it is handing over.
    ///
    /// [`Op::Fold`] where a pane was pulled out through its own edge, and
    /// [`Op::Unfold`] where a closed pane's edge was pulled back in. It names
    /// the node, so a caller with a sentence to print has everything it needs
    /// and reads the rest off [`Panel::layout`].
    ///
    /// # Performed here, where [`Fader`](Dragged::Fader) is not, and the module
    /// documentation is why
    ///
    /// *A boundary drag moves the arrangement, which is this crate's, so
    /// `moved` writes it and reports where it landed; a fader drag moves the
    /// engine's value, which this crate does not have and cannot reach.* A
    /// fold is the arrangement, so it is written here and this says what was
    /// written. The alternative — handing an unperformed `Op` back and waiting
    /// for the caller — would leave the layout disagreeing with itself for one
    /// event, on the pane the *next* move is about to ask a question of.
    ///
    /// # Why it is an [`Op`] at all, rather than a size the drag wrote
    ///
    /// A drag that only zeroed the pane's extent would leave nothing
    /// [`Layout::is_collapsed`] could answer yes to, so `z`
    /// ([`Op::UnfoldAll`]) would not bring it back — and `z` is the way back
    /// the manual promises for everything that folds. Going through
    /// [`Op::Fold`] is what makes the close a **fold** rather than a very
    /// small pane, and it costs nothing: these operations write no session
    /// record either way (`karakuri-operation-record` answers
    /// `Silent::Surface` for all four).
    Pane(Op),
    /// **A fader turned the drag into one operation of the vocabulary**, which
    /// is the whole of what it did.
    ///
    /// Nothing was written anywhere: the value is the engine's, this crate has
    /// no engine (ADR-0156), and *applying* it is the caller's — a record, and
    /// the same record every other surface's control ends in
    /// ([P-0090](../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
    ///
    /// **One value and not two.** Which deck and which control are inside the
    /// operation already, and a copy of either beside it would be two
    /// statements about one thing — [`crate::view::StripBox`]'s rule about a
    /// fill stored beside its track.
    Fader(Operation),
}

/// What a release left behind: the boundary that was in hand, and where it
/// came to rest.
///
/// [`Panel::released`] returns `None` when nothing was in hand, because a
/// release that lets go of nothing did nothing.
///
/// **Not `Copy`, since [`Released::Dropped`] landed**, and that is the
/// `String` in a Set's name arriving here rather than a decision taken about
/// this enum: an operation naming a Set carries the name, `karakuri-operation`
/// is a leaf crate that spells one as a `String`, and a release that answered
/// with anything less would be this module inventing an id type for a store it
/// cannot see (ADR-0156). Nothing else here changed shape — see [`Carrying`].
#[derive(Debug, Clone, PartialEq)]
pub enum Released {
    Rests {
        split: NodeId,
        index: usize,
        /// Along the split's axis.
        at: f32,
    },
    /// The boundary is no longer there — an operation during the drag folded
    /// one of the pair away.
    Gone { split: NodeId, index: usize },
    /// **A fader was let go**, and which one — the deck inside the [`Knob`]
    /// where it is a deck's.
    ///
    /// **There is no value here, and that is the decision rather than an
    /// omission.** Where a boundary comes to rest is read back out of the
    /// layout, which owns it; where a fader comes to rest is the *deck's*, and
    /// the last thing this drag asked for was emitted when it was asked for.
    /// A value reported here would be this crate answering a question about
    /// somebody else's state, which is the second copy the whole module is
    /// written to refuse.
    Let { knob: Knob },
    /// **A carried Set was let go over a rectangle that names a deck**, and
    /// the load it asks for.
    ///
    /// **Two sets of rectangles name one**: a mixer strip
    /// ([`crate::view::Mixer::dropped`]) and a deck preview cell
    /// ([`crate::view::ProgramBay::dropped`]), which is `console.html`'s
    /// *"the same command rather than a second command"* and arrives here as
    /// the one `deck` the caller resolved. Which of the two it was is not in
    /// this value and is not owed one: the operation names a deck and a Set,
    /// and a copy of the route beside it would be a third operand nothing
    /// downstream can use
    /// ([ADR-0273](../../../docs/adr/0273-the-carry-lands-on-two-sets-of-rectangles-and-wears-a-face.md)).
    ///
    /// **The one release on this panel that carries a value, and
    /// [`Let`](Released::Let) above is why it is not a contradiction.** A
    /// fader emitted every value it asked for while it was moving, so a value
    /// here would be a second copy of the last of them; a carry emits nothing
    /// while it moves, because a Set half-way to a strip has not been loaded
    /// anywhere. The drop is the whole of what the gesture asks for, so it is
    /// asked for here or nowhere.
    ///
    /// **Which deck is the caller's answer and not this module's**, handed to
    /// [`Panel::released`] — see there, and [`Grab`] for the same seam at the
    /// other end of a gesture.
    ///
    /// **Nothing is refused.** A drop on a deck that is live replaces what the
    /// room is watching and this still asks for it: *"what may be asked for is
    /// the instrument's to decide"* (`docs/manual/console.html`, *How a Set
    /// reaches a deck*), and the gate is where that answer is
    /// ([P-0090](../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
    /// **This drag has no reading of residency at all** — a strip's `tally` is
    /// not asked, here or in [`crate::view::Mixer::dropped`] — so there is
    /// nowhere for a second rule to hide.
    Dropped(Operation),
    /// **A carried Set was let go over nothing**, and nothing is asked for.
    ///
    /// The third outcome a release can have, and it exists because a carry is
    /// the one gesture on this panel that can be **cancelled**: a boundary
    /// dragged off its track still lands somewhere legal and a fader dragged
    /// past its end is still at its end, so both of them come to rest whatever
    /// the pointer did. A row let go over the transport row, over another bay,
    /// or off the viewport has nowhere to land, and a load aimed at the
    /// nearest strip would be a deck nobody pointed at. **A cell whose letter
    /// names no slot is one of those places**: the row is four cells whatever
    /// the deck holds, so the fourth cell of a three-slot deck is a rectangle
    /// with nothing behind the letter on it
    /// ([`crate::view::ProgramBay::dropped`]).
    ///
    /// **The Set is named because the caller has a sentence to say**, and
    /// saying nothing at all is the failure this arm is against: a gesture
    /// that is picked up, carried and then silently forgotten reads as a
    /// panel that missed the press.
    Nowhere { set: String },
}

/// Whether a node is drawing, and if not, why not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Visible,
    /// Folded itself.
    Folded,
    /// Not folded, but inside something that is.
    InsideAFold,
}

/// One node of the arrangement and where it solved to. What [`Op::Report`]
/// produces, one per [`Node`], in the same order.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    pub id: NodeId,
    pub depth: usize,
    pub rect: Rect,
    pub state: Visibility,
}

/// What an [`Op`] did.
///
/// One enum over every operation rather than one type each, because a caller
/// dispatches an `Op` from a key or a message and matches the outcome in the
/// same place. Which variant an operation can produce is on the operation.
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    /// [`Op::Fold`], [`Op::Unfold`] or [`Op::FoldEnclosing`]: `folded` is what
    /// the node **is** afterwards, read back out of the layout rather than
    /// assumed from the operation, and `root` that what folded was the root —
    /// so the panel is now empty, and only [`Op::UnfoldAll`] brings it back.
    Folded {
        id: NodeId,
        folded: bool,
        root: bool,
    },
    /// [`Op::UnfoldAll`]: everything that was folded, in tree order. Empty
    /// when nothing was.
    Unfolded(Vec<NodeId>),
    /// [`Op::Solo`].
    Soloed(NodeId),
    /// [`Op::Unsolo`]. `was` is false when there was no solo to undo, and
    /// nothing changed.
    Unsoloed { was: bool },
    /// [`Op::Reset`].
    Reset,
    /// [`Panel::restore`]: a saved arrangement is in, at the viewport the
    /// window already had.
    ///
    /// **No name**, where the operation that asked for it carries one: this
    /// crate never saw the name. It was handed a `Layout` by whoever read the
    /// store, and an outcome that repeated a name back would be repeating the
    /// caller's own argument to it.
    Restored,
    /// [`Op::Report`].
    Report(Vec<Placement>),
    /// The operation had nothing to act on: [`Op::FoldEnclosing`] on the root,
    /// which is the one node with nothing enclosing it.
    ///
    /// **It used to be the answer to *nothing under the pointer* as well**,
    /// and that half of it moved out to the caller with the pointer itself —
    /// see [`Op`] and [`Panel::under`]. What is left is the case that is about
    /// the arrangement rather than about where a hand is.
    Nothing,
}

/// **What the pointer has hold of**, and there is only ever one of them —
/// [`Panel::in_hand`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InHand {
    /// A boundary, and the axis it runs along — which is what a resize cursor
    /// is drawn from, and the whole of why the axis is here.
    Boundary(Axis),
    /// A fader. **No axis**, because there is no resize cursor for a value:
    /// see [`Panel::in_hand`].
    Fader,
    /// **A Set**, carried out of the Library bay. No axis for the fader's
    /// reason and no payload for [`Panel::cursor`]'s: which Set is in hand is
    /// the drag's, and a caller holding a copy of it would be shadowing the
    /// drag it is about to ask to end.
    ///
    /// **It is a third answer rather than a second `Fader`**, and what needs
    /// it is a caller deciding what a *move* meant: a move with a fader in
    /// hand can emit an operation and a move with a carry in hand never can,
    /// so a window loop that could not tell them apart would answer the
    /// repaint question for one of them wrongly. `crates/karakuri/src/main.rs`
    /// is where that is asked.
    Carrying,
}

/// The console panel: the arrangement, the pointer, and the drag in hand.
pub struct Panel {
    layout: Layout,
    /// The tree, flattened, rebuilt whenever the layout is.
    nodes: Vec<Node>,
    cursor: Point,
    drag: Option<Drag>,
}

impl Panel {
    /// The console's arrangement, in a viewport of `width` by `height`.
    pub fn new(width: f32, height: f32) -> Panel {
        let mut panel = Panel {
            layout: crate::layout(),
            nodes: Vec::new(),
            cursor: Point::new(-1.0, -1.0),
            drag: None,
        };
        panel.rebuild();
        panel.set_viewport(width, height);
        panel
    }

    /// The arrangement, to read. Every read of a rectangle wants
    /// [`solve`](Panel::solve) called first.
    pub fn layout(&self) -> &Layout {
        &self.layout
    }

    pub fn layout_mut(&mut self) -> &mut Layout {
        &mut self.layout
    }

    /// The tree, flattened in tree order.
    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    /// Where the pointer is. An [`Op`] acts on whatever this is over.
    pub fn cursor(&self) -> Point {
        self.cursor
    }

    /// Whether **anything** is in hand — a boundary or a fader.
    ///
    /// It exists for [`crate::input::claim`]: a drag in progress keeps the
    /// pointer whatever the pointer is currently over, so the rule has to be
    /// able to ask. It answers for both kinds because there is one thing in
    /// hand, which is why rule 1 covers a fader held against its top while the
    /// pointer runs on across two bays without a word being added to it.
    pub fn dragging(&self) -> bool {
        self.in_hand().is_some()
    }

    /// **What the pointer has hold of**, or `None` where nothing is in hand.
    ///
    /// # It replaced a `drag_axis() -> Option<Axis>`, and the second drag is
    /// why
    ///
    /// That method answered *the axis of the boundary in hand*, and it had one
    /// caller: [`crate::view::View`]'s cursor, which draws a resize cursor
    /// from it. A fader drag runs along an axis too and **must not** produce
    /// one — a resize cursor over a gesture that resizes nothing — so its
    /// honest answer there is `None`.
    ///
    /// **And a `None` that means *not a boundary* cannot be told from a
    /// `None` that means *nothing at all*, which is the distinction both of
    /// its readers need.** The cursor has to know that a fader in hand
    /// suppresses the hit test, or it flicks a resize cursor on the moment a
    /// fader held against its top lets the pointer wander across a boundary;
    /// and a window loop has to know that a move which emitted no operation
    /// was a fader that did not change rather than a pointer with nothing in
    /// hand, or every mouse move on the panel answers a repaint question meant
    /// for a drag. One question with three answers, rather than a predicate
    /// beside an `Option`.
    ///
    /// **Which fader it is stays private**, on [`Panel::cursor`]'s terms: a
    /// caller holding it would be shadowing the drag, and it is not needed —
    /// the operation a move emits carries the deck and the control already.
    pub fn in_hand(&self) -> Option<InHand> {
        Some(match self.drag.as_ref()? {
            Drag::Boundary(b) => InHand::Boundary(b.axis),
            Drag::Fader(_) => InHand::Fader,
            Drag::Carry(_) => InHand::Carrying,
        })
    }

    /// Move the pointer without dragging anything.
    /// [`moved`](Panel::moved) is the same thing with a boundary in hand.
    pub fn set_cursor(&mut self, p: Point) {
        self.cursor = p;
    }

    /// Walk the arrangement and flatten it. Order is the tree's, so a caller
    /// that colours or indents by position reads top to bottom.
    fn rebuild(&mut self) {
        let mut nodes = Vec::new();
        walk(&self.layout, self.layout.root(), 0, &mut nodes);
        self.nodes = nodes;
    }

    /// The panel fills `width` by `height` from the origin. A resize is not an
    /// edit: it produces smaller rectangles and stores nothing, so a window
    /// dragged small and large again comes back to the arrangement it left.
    pub fn set_viewport(&mut self, width: f32, height: f32) {
        self.layout.set_viewport(Rect::new(0.0, 0.0, width, height));
    }

    /// The one solve. Everything that reads calls this first; on a frame where
    /// nothing changed it is a flag test.
    pub fn solve(&mut self) {
        self.layout.solve();
    }

    /// **Take a region out of the layout, or put it back, for a reason that is
    /// not the operator's** — [`Layout::set_aside`], and returning **whether
    /// the bit changed**.
    ///
    /// The bit is the drawer's and never the operator's
    /// ([ADR-0183](../../../docs/adr/0183-a-node-is-out-of-the-layout-for-two-reasons-and-they-are-two-bits.md)),
    /// so this is not an [`Op`]: nothing an operator presses reaches it, it is
    /// not what a saved arrangement carries, and [`Outcome`] has no word for
    /// it. What is here rather than in `crate::view` is the `&mut Layout` this
    /// panel keeps to itself.
    ///
    /// **The answer is what the write did and not what the caller asked**,
    /// which is [`crate::repaint`]'s rule about an operation arm: the value is
    /// re-derived and re-written every frame, and a frame that wrote the value
    /// the node already carried changed nothing and is owed nothing. See
    /// [`crate::view::rearrange`], which is the one caller.
    pub fn set_aside(&mut self, id: NodeId, aside: bool) -> bool {
        let moved = self.layout.is_set_aside(id) != aside;
        self.layout.set_aside(id, aside);
        moved
    }

    /// The two regions a boundary is between. A split is often unnamed — the
    /// console's body row is, deliberately — so a split and an index alone do
    /// not say which boundary a pointer has hold of, and the pair does.
    ///
    /// [`Layout::placed_children`] is what an index counts, and this is that
    /// twice: the readout wants both names, and asking for the pair is what
    /// every caller of it was doing.
    ///
    /// **A closed pane is one of them**, which is the whole of how a drag
    /// reaches a region that has no rectangle: the pane is placed at zero
    /// extent with its divider still beside it, so the pair either side of
    /// that divider names it (ADR-0300).
    pub fn pair(&self, split: NodeId, index: usize) -> Option<(NodeId, NodeId)> {
        let mut children = self.layout.placed_children(split).skip(index);
        Some((children.next()?, children.next()?))
    }

    // -- input ----------------------------------------------------------

    /// The pointer went down at `p`. A press in a gap takes the boundary in
    /// hand; anywhere else it only says what is there.
    pub fn press(&mut self, p: Point) -> Pressed {
        self.solve();
        self.cursor = p;
        match self.layout.hit(p, GRAB) {
            Hit::Divider { split, index } => {
                let axis = self.layout.axis(split).expect("a divider is on a split");
                let Some(gap) = self.layout.boundary(split, index) else {
                    return Pressed::NoPair { split, index };
                };
                let boundary = axis.origin(gap);
                let offset = axis.coord(p) - boundary;
                self.drag = Some(Drag::Boundary(Boundary {
                    split,
                    index,
                    axis,
                    offset,
                    said: None,
                    held: false,
                    acted: false,
                }));
                Pressed::Grabbed {
                    split,
                    index,
                    axis,
                    at: boundary,
                    offset,
                }
            }
            Hit::View(id) => Pressed::Region {
                id,
                rect: self.layout.rect(id),
            },
            Hit::Nothing => Pressed::Nothing,
        }
    }

    /// **Take a mixer strip's fader in hand.** [`press`](Panel::press)'s other
    /// door, and it is a second one because a fader's geometry is not the
    /// layout's — see [`Grab`], which is what the view resolves a press into.
    ///
    /// **A press on the knob and on nothing else.** A press on the *track*,
    /// off the knob, never reaches this: [`crate::view::Mixer::grab`] answers
    /// `None` for it, and that is a decision rather than a gap in the hit
    /// test. A fader at 0.3 whose top is clicked would jump to 1.0 — a change
    /// to the mix nobody asked for, made on stage, on a control whose whole
    /// job is that it does what a hand did. The knob is where the hand is.
    ///
    /// **The press itself moves nothing.** The value this drag starts from is
    /// the value the knob is already at — which is [`Grab::value`] at the
    /// press, by construction of the offset — so a press and a release with no
    /// motion between them emits nothing at all. That is worth the line it
    /// costs: `Deck::set_gain` cancels whatever was moving the control ("the
    /// operator wins"), so an operation emitted for a press that moved nothing
    /// would stop a running transition by being touched.
    ///
    /// It reports nothing back. What a press found is what the caller just
    /// resolved for itself, and [`Pressed`] answering it a second time would
    /// be the model repeating the view.
    pub fn grab(&mut self, p: Point, grab: Grab) {
        self.cursor = p;
        let said = Some(grab.value(grab.axis.coord(p)));
        self.drag = Some(Drag::Fader(Fading { grab, said }));
    }

    /// **Take a Set out of the Library bay.** [`grab`](Panel::grab)'s door with
    /// a payload instead of a control — see [`Carrying`].
    ///
    /// **The view resolves which Set and this holds it**, which is [`Grab`]'s
    /// seam over a listing instead of over a track: where the rows are is
    /// [`crate::view::LibraryBay`]'s answer and depends on what the store said
    /// this frame, and this module has no listing and takes none.
    /// [`crate::view::LibraryBay::take`] is what a press resolves to.
    ///
    /// **The press itself asks for nothing at all**, which is
    /// [`grab`](Panel::grab)'s rule read one bay along and is a stronger
    /// statement here: a fader's press emits nothing because the value has not
    /// moved, and a carry's emits nothing because *there is no operation yet*
    /// — a Set names a load only once a deck is named too, and no deck is
    /// named until the button comes up. So a press and a release with no
    /// motion between them is a row picked up and put down, and the deck under
    /// it at the release is the whole of what decides otherwise.
    ///
    /// It reports nothing back, for [`grab`](Panel::grab)'s reason: what the
    /// press found is what the caller just resolved for itself.
    pub fn carry(&mut self, p: Point, set: String) {
        self.cursor = p;
        self.drag = Some(Drag::Carry(Carrying { set }));
    }

    /// A move with something in hand, and what it did — see [`Dragged`].
    /// `None` where nothing is in hand, and `None` where this move changed
    /// nothing.
    ///
    /// **Absolute, both ways.** A boundary takes the pointer's coordinate
    /// along the split's axis, less the offset it grabbed at, straight into
    /// [`Layout::set_divider`]; a fader takes the same coordinate through
    /// [`Grab::value`]. Nothing accumulates in either, which is what makes a
    /// drag past a stop — or past the end of a track — and back come home
    /// exactly.
    ///
    /// **A carry answers `None` to every move**, and that is the third kind of
    /// drag rather than a case this forgot: nothing has happened, because
    /// nothing happens until the Set is let go somewhere. [`Dragged`] has no
    /// arm for it and is not owed one — an arm meaning *a hand is carrying
    /// something and nothing happened* is `None` with a name on it, and a
    /// caller that wants to draw the carry asks [`in_hand`](Panel::in_hand),
    /// which is the question *is a gesture in progress* and already exists.
    pub fn moved(&mut self, p: Point) -> Option<Dragged> {
        self.cursor = p;
        match self.drag.as_ref()? {
            Drag::Boundary(_) => self.moved_boundary(p),
            Drag::Fader(_) => self.moved_fader(p),
            Drag::Carry(_) => None,
        }
    }

    /// A move with a boundary in hand.
    ///
    /// # A pane is closed by pulling its boundary out, and opened by pulling it
    /// in
    ///
    /// The overshoot a stop keeps — `landed - asked` — is the whole of what
    /// this reads. Its **sign** says which side of the boundary the pointer is
    /// pressing into, its **size** says how far past the stop the hand has gone
    /// on, and whether that side is already [`Layout::is_closed`] says whether
    /// the drag is closing it or opening it. See [`PULLED_THROUGH`] for the
    /// distance and [`Boundary::acted`] for why it happens once.
    ///
    /// **Only a region whose arrangement says a fold leaves its edge behind is
    /// closed this way** ([`Layout::keeps_its_edge`]), and only one that is
    /// **at its own minimum** — a boundary held by a *neighbour's* maximum is
    /// not one that has run this region out of room, and folding it there would
    /// be a fold nobody was asking for.
    fn moved_boundary(&mut self, p: Point) -> Option<Dragged> {
        let Some(Drag::Boundary(drag)) = self.drag.as_ref() else {
            return None;
        };
        let (split, index, axis, offset) = (drag.split, drag.index, drag.axis, drag.offset);
        let acted = drag.acted;
        let asked = axis.coord(p) - offset;
        let landed = self.layout.set_divider(split, index, asked);
        // `set_divider` solves before it returns, so the reads below are of a
        // clean layout.
        let by = landed - asked;
        let held = by.abs() >= STOPPED;

        if let Some(op) = (!acted)
            .then(|| self.pane_pulled(split, index, axis, by))
            .flatten()
        {
            self.op(op);
            if let Op::Unfold(pane) = op {
                self.open_at_minimum(split, index, pane);
            }
            self.solve();
            let at = self
                .layout
                .boundary(split, index)
                .map(|gap| axis.origin(gap));
            if let Some(Drag::Boundary(drag)) = self.drag.as_mut() {
                drag.acted = true;
                // **Where the boundary is now, said once** — by this very
                // value. The fold moved it to the pane's own edge, or out to
                // the pane's minimum, and [`Dragged::Pane`] is this drag
                // saying so; a second report of the same position, from the
                // next of sixty pointer events a second, would be the flood
                // [`Boundary::said`] exists to stop. `held`, because the
                // boundary is not following the pointer and will not until
                // the hand comes back to it.
                drag.said = at;
                drag.held = true;
            }
            return Some(Dragged::Pane(op));
        }

        let Some(Drag::Boundary(drag)) = self.drag.as_mut() else {
            return None;
        };
        let say = match drag.said {
            None => true,
            Some(said) => (landed - said).abs() >= WORTH_SAYING || drag.held != held,
        };
        if !say {
            return None;
        }
        drag.said = Some(landed);
        drag.held = held;

        Some(Dragged::Boundary {
            split,
            index,
            axis,
            asked,
            landed,
            held: held.then_some(by),
        })
    }

    /// **Put a pane a drag has just brought back at the smallest extent it
    /// declares**, whichever side of the boundary it is on.
    ///
    /// *"A pane reopened by dragging inward comes back at its declared
    /// minimum, not at whatever it was before"* — because the hand that opened
    /// it is at the window's edge, and a pane that sprang back to the 340 it
    /// was months ago would jump out from under the pointer. What it was is
    /// still stored, and `z` still brings that back.
    ///
    /// **The ask is the far end and the layout's own clamp is what stops it.**
    /// [`Layout::set_divider`] clamps a drag to the pair's combined bounds, so
    /// asking for the boundary to go all the way to the pane's own side lands
    /// it at exactly the pane's minimum — or at the most the pair can give it,
    /// where the neighbour's own bounds allow less. Working the position out
    /// here instead would be a second copy of that clamp, one term of which is
    /// the *other* region's.
    fn open_at_minimum(&mut self, split: NodeId, index: usize, pane: NodeId) {
        self.solve();
        let Some((a, _)) = self.pair(split, index) else {
            return;
        };
        let position = match pane == a {
            true => f32::NEG_INFINITY,
            false => f32::INFINITY,
        };
        self.layout.set_divider(split, index, position);
        self.solve();
    }

    /// **What a drag at `asked` is asking of the pair either side of this
    /// boundary, where what it is asking is a fold**, or `None` for the
    /// ordinary case of a boundary that is simply being moved.
    ///
    /// Reads, and writes nothing: the caller performs the [`Op`] so that a
    /// fold is a fold wherever it comes from.
    ///
    /// **`by` is what the stop kept — `landed - asked`, after the drag has
    /// been made.** Asked against the boundary's position *before* the move
    /// would answer a different question on the first event of a gesture: a
    /// drag that runs from a pane's full width to well past its minimum in one
    /// pointer event would find the pane not yet at its stop and decline the
    /// fold it plainly asked for. After the drag, the pane is at whatever the
    /// clamp allowed and the overshoot is exactly what the clamp kept.
    ///
    /// A closed pane's boundary never moves — [`Layout::set_divider`] refuses
    /// it, because a closed pane's extent is not the boundary's to give away —
    /// so for the opening case `landed` is the boundary where it stands and
    /// `by` is how far in from it the pointer has gone.
    fn pane_pulled(&mut self, split: NodeId, index: usize, axis: Axis, by: f32) -> Option<Op> {
        let (a, b) = self.pair(split, index)?;

        // **Opening comes first**, because a boundary with a closed pane beside
        // it is not a boundary anybody can be pushing against: there is nothing
        // on that side to run out of room.
        if self.layout.is_closed(a) && by <= -PULLED_THROUGH {
            return Some(Op::Unfold(a));
        }
        if self.layout.is_closed(b) && by >= PULLED_THROUGH {
            return Some(Op::Unfold(b));
        }
        let (into, pulled) = match by >= 0.0 {
            true => (a, by),
            false => (b, -by),
        };
        if pulled < PULLED_THROUGH || !self.layout.keeps_its_edge(into) {
            return None;
        }
        // At its own minimum, and not merely stopped: `set_divider` clamps
        // against the pair's *combined* bounds, so a boundary can be held by
        // the far side's maximum with this side nowhere near its floor. The
        // extent is read after the drag, which is what makes the whole of a
        // one-event drag count towards it.
        let (min, _) = self.layout.bounds(into);
        let extent = axis.extent(self.layout.rect(into));
        (extent <= min + STOPPED).then_some(Op::Fold(into))
    }

    /// A move with a fader in hand: the pointer becomes a value, and the value
    /// becomes an operation. **Nothing is written**, here or anywhere in this
    /// crate — see [`Dragged::Fader`].
    fn moved_fader(&mut self, p: Point) -> Option<Dragged> {
        let Some(Drag::Fader(fading)) = self.drag.as_mut() else {
            return None;
        };
        let value = fading.grab.value(fading.grab.axis.coord(p));
        // **Exactly, and not within a threshold.** See `Fading::said`: a
        // boundary's half a pixel is about what is worth *printing* at sixty
        // asks a second, and every value that differs here is a different mix.
        if fading.said == Some(value) {
            return None;
        }
        fading.said = Some(value);
        Some(Dragged::Fader(fading.grab.knob.operation(value)))
    }

    /// The pointer went up. `None` where nothing was in hand.
    ///
    /// # `onto` is the destination, resolved by whoever can resolve it
    ///
    /// **Which deck the rectangle under the pointer names** — a mixer strip
    /// or one of the four deck preview cells — or `None` for neither, and
    /// `None` for every release that is not a carry, because the other two
    /// drags have no destination to name. A boundary comes to rest where the
    /// layout put it and the layout is asked; a fader's rest is the deck's and
    /// nobody is asked at all ([`Released::Let`]).
    ///
    /// **It is an argument for the reason [`Grab`] is one.** A strip's
    /// geometry is [`crate::view::mixer`]'s answer — it depends on the values
    /// the harness handed in and on a text shaper for the words in the same
    /// strip — and this module has neither and takes neither. The alternative
    /// is the toolkit inside the model. So the view is asked, as
    /// [`crate::view::Outputs::op`] is asked, at a release instead of at a
    /// press: [`crate::view::Mixer::dropped`] is the derivation, and it is the
    /// same laid-out bay the frame drew.
    ///
    /// **One door and not two.** A carry could have had a `dropped(onto)` of
    /// its own beside a `released()` that never takes a destination, and then
    /// a caller that reached for the wrong one would cancel every drop in
    /// silence. Here a caller that cannot answer passes `None`, which is the
    /// honest outcome for a release that landed on nothing anyway.
    pub fn released(&mut self, onto: Option<u8>) -> Option<Released> {
        match self.drag.take()? {
            Drag::Boundary(drag) => {
                self.solve();
                let (split, index) = (drag.split, drag.index);
                // `Gone` is what an operation during the drag leaves behind:
                // fold either side of the boundary and there is no longer a
                // pair for this index, which is [`Layout::boundary`] returning
                // `None` rather than a coordinate for something else.
                Some(match self.layout.boundary(split, index) {
                    Some(gap) => Released::Rests {
                        split,
                        index,
                        at: drag.axis.origin(gap),
                    },
                    None => Released::Gone { split, index },
                })
            }
            // No solve: a fader drag never touched the layout, so there is
            // nothing owed and nothing to read back out of it.
            Drag::Fader(fading) => Some(Released::Let {
                knob: fading.grab.knob.clone(),
            }),
            // No solve either, and for the same reason: a carry moved nothing
            // here. `onto` was resolved against the layout the *caller* had
            // already solved to hit-test it, which is the one solve there is.
            Drag::Carry(carrying) => Some(match onto {
                // **The whole of what the gesture asks for**, built here for
                // `Knob::operation`'s reason: the translation from what a hand
                // did into one operation of the vocabulary is the model's, and
                // the two operands are the payload and the destination.
                Some(deck) => Released::Dropped(Operation::LoadSet {
                    deck,
                    set: carrying.set,
                }),
                None => Released::Nowhere { set: carrying.set },
            }),
        }
    }

    /// **What the pointer is over, for a caller about to name an
    /// [`Op`]'s target.**
    ///
    /// The resolution [`op`](Panel::op) used to do for itself, out where it
    /// belongs: *the region under the pointer* is how a keyboard chooses what
    /// to fold, and it is not part of what folding means — see [`Op`]. A
    /// caller with a control under the pointer instead of a region, or with no
    /// pointer at all, never asks this.
    ///
    /// **No grab**, which is [`Layout::hit`] with a zero-width divider: this
    /// answers *what did the operator mean*, and a boundary's six pixels
    /// either side exist so that a **hand** can find a nine-pixel gap. An
    /// operation that took them would fold the wrong region six pixels from
    /// every edge. [`press`](Panel::press) is the other one and it uses
    /// [`GRAB`], because that one is the hand.
    pub fn under(&mut self) -> Hit {
        self.solve();
        self.layout.hit(self.cursor, 0.0)
    }

    /// Act.
    pub fn op(&mut self, op: Op) -> Outcome {
        self.solve();
        let outcome = match op {
            // Two directions and no toggle, and the answer is read back out of
            // the layout rather than assumed: `folded` is what the node is
            // now, so an operation that found the node already there says the
            // truth rather than the intent.
            Op::Fold(id) => {
                self.layout.collapse(id);
                self.folded(id)
            }
            Op::Unfold(id) => {
                // **The solo first, because it is what the expanding would
                // otherwise be undone by.** A solo collapses everything it did
                // not name and keeps the flags it replaced; `unsolo` puts
                // exactly those back — so expanding under a live solo leaves
                // `Layout::soloed` naming a region that is no longer the only
                // one on screen, and the next unsolo quietly throws the expand
                // away. Dropped only when the solo is what is hiding `id`: a
                // solo that already has it on screen is not this operation's
                // business.
                if self.layout.is_soloed() && !self.layout.visible(id) {
                    self.layout.unsolo();
                }
                // The node and the way to it. `expand` on a node that is not
                // collapsed changes nothing and marks nothing dirty, so this
                // is the path that was folded and no more than it.
                let mut node = Some(id);
                while let Some(step) = node {
                    self.layout.expand(step);
                    node = self.layout.parent(step);
                }
                self.folded(id)
            }
            Op::FoldEnclosing(id) => match self.layout.parent(id) {
                Some(split) => {
                    self.layout.collapse(split);
                    self.folded(split)
                }
                // The root is the one node with nothing enclosing it.
                None => Outcome::Nothing,
            },
            Op::UnfoldAll => {
                let folded: Vec<NodeId> = self
                    .nodes
                    .iter()
                    .map(|n| n.id)
                    .filter(|id| self.layout.is_collapsed(*id))
                    .collect();
                for id in &folded {
                    self.layout.expand(*id);
                }
                Outcome::Unfolded(folded)
            }
            Op::Solo(id) => {
                self.layout.solo(id);
                Outcome::Soloed(id)
            }
            Op::Unsolo => {
                let was = self.layout.is_soloed();
                self.layout.unsolo();
                Outcome::Unsoloed { was }
            }
            Op::Reset => {
                let viewport = self.layout.viewport();
                self.layout = crate::layout();
                self.layout.set_viewport(viewport);
                self.rebuild();
                self.drag = None;
                Outcome::Reset
            }
            Op::Report => {
                // No solve of its own: the one at the top of this method is
                // the caller's, and a second here would hide an operation that
                // left one owed rather than catch it. See the test.
                Outcome::Report(
                    self.nodes
                        .iter()
                        .map(|n| Placement {
                            id: n.id,
                            depth: n.depth,
                            rect: self.layout.rect(n.id),
                            state: match (self.layout.is_collapsed(n.id), self.layout.visible(n.id))
                            {
                                (true, _) => Visibility::Folded,
                                (false, false) => Visibility::InsideAFold,
                                (false, true) => Visibility::Visible,
                            },
                        })
                        .collect(),
                )
            }
        };
        self.solve();
        outcome
    }

    /// **Put a saved arrangement in**, at the viewport this window already
    /// has.
    ///
    /// It is [`Op::Reset`]'s arm with the arrangement handed in rather than
    /// built: the viewport is carried across, the tree is flattened again and
    /// any drag in hand is dropped, because the node a hand had hold of is not
    /// a node of this arrangement. **The viewport in `layout` is discarded**,
    /// and that is the decision rather than an omission — an arrangement
    /// carries the window it was saved at, and a console arranged on a laptop
    /// would otherwise come back on a projector with the laptop's margin round
    /// it.
    ///
    /// # Why this is a method and not a ninth [`Op`]
    ///
    /// An [`Op`] names a [`NodeId`] or nothing at all, and it is `Copy` and
    /// `Eq` because every one of its eight variants is a handle or a word. A
    /// restore's payload is a whole arrangement, which no key press, no map
    /// line and no pointer can produce — **only a third party holding a store
    /// can**, and this crate has no store and cannot have one (ADR-0156). So
    /// the operator's operation is
    /// `karakuri_operation::Operation::RestoreArrangement { name }`, whoever
    /// holds the store turns that name into a `Layout`, and this is where the
    /// `Layout` lands. `tests/vocabulary.rs` records the same thing from the
    /// other side: the row is in its `NO_OP` list, with the reason.
    ///
    /// **Nothing here reads or refuses the bytes.** A file that disagrees with
    /// itself never becomes a `Layout` at all —
    /// `karakuri_layout::Layout`'s own `TryFrom<Wire>` refuses it
    /// (`docs/adr/0158-a-saved-arrangement-that-disagrees-with-itself-is-refused-not-repaired.md`)
    /// — so by the time one arrives here it is an arrangement, and a second
    /// check would be a second answer to a question that has one.
    pub fn restore(&mut self, layout: Layout) -> Outcome {
        let viewport = self.layout.viewport();
        self.layout = layout;
        self.layout.set_viewport(viewport);
        self.rebuild();
        self.drag = None;
        self.solve();
        Outcome::Restored
    }

    /// What a fold or an unfold left behind, read out of the layout. One
    /// place, because the three operations that fold differ in what they aim
    /// at and not at all in what they report.
    fn folded(&self, id: NodeId) -> Outcome {
        Outcome::Folded {
            id,
            folded: self.layout.is_collapsed(id),
            root: id == self.layout.root(),
        }
    }
}

fn walk(layout: &Layout, id: NodeId, depth: usize, out: &mut Vec<Node>) {
    out.push(Node { id, depth });
    for child in layout.children(id) {
        walk(layout, *child, depth + 1, out);
    }
}
