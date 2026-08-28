//! **When the panel is drawn again, and when the window sleeps.**
//!
//! [P-0072](../../../docs/principles/0072-a-still-panel-costs-nothing-and-what-moves-declares-its-price.md)'s
//! first clause and nothing else: *a panel with nothing changing on it is paid
//! for once and not again.* A window loop that drives itself pays that price on
//! every frame nobody is touching, and the price is the panel's rather than a
//! fixed one:
//! [ADR-0164](../../../docs/adr/0164-the-panel-is-budgeted-rather-than-forbidden-to-allocate.md)
//! measured 184 allocations and 226.2 kB a frame with every bay empty, and the
//! panel as it now stands — a live picture, the preview row, the mixer bay, the
//! transport and the outputs row — reads 525 and 694.3 kB in the middle of
//! nine runs on 2026-08-26, which spread 524 to 538 (`examples/panel.rs` takes
//! it, and holds its own quoted figure against every run of it). This is the
//! decision that stops those frames being drawn at all.
//!
//! **One region declares a cost and a staleness now, and the scheduler still
//! does not exist.** [`crate::view::View::declares`] is the declaration —
//! the mixer bay, while a residency request has not landed or a fade has not
//! run — and [`Change::Animating`] carries the soonest staleness out of it and
//! turns that into a deadline. The **cost** reaches nothing here and is not
//! meant to: both schedulability conditions are arithmetic over the
//! declarations and `tests/schedulable.rs` asserts them, which is what
//! P-0072 asks for in place of a stage discovering them. What is still absent
//! is arbitration — nothing chooses between two regions, because with one
//! region declaring there is nothing to choose between.
//!
//! # Why this is a module and not a line beside each handler
//!
//! Because a panel that **under**-repaints is far worse than one that
//! over-repaints, and it fails silently: a control left on screen showing a
//! value that is no longer true says nothing anywhere, and there is no
//! assertion to write against it in a window — a stale pixel is not an error.
//! So the decision is made a value a test can ask for, away from an event loop
//! that cannot be called from a test at all. That is the seam
//! [`crate::panel`] came out of the window loop through, used again.
//!
//! [`Change`] is therefore one list of **everything that can change what the
//! console shows**. A path that changes the screen and reaches no repaint is
//! then a missing arm, which the compiler finds, rather than a forgotten line
//! at one of a dozen call sites, which nothing finds.
//!
//! # Which way it errs, and where
//!
//! Towards drawing. Every arm below that could be argued either way is
//! [`Repaint::Now`], and each is on a gesture the operator is making — P-0072
//! budgets the frames nobody is touching and says outright that *what the
//! operator does costs what it costs*. What is **not** allowed to err that way
//! is a frame with nobody touching the window, which is the whole of the
//! clause being implemented, and no arm here produces one.
//!
//! # Who else asks for frames
//!
//! Two more, and neither is a [`Change`]:
//!
//! - **`egui`, for an event it consumed.** `egui_winit::EventResponse::repaint`
//!   is true for every window event `egui` reacts to at all, so an event routed
//!   to `egui` ([`Claim::Egui`]) already earns its frame there, and this
//!   returns [`Repaint::Never`] for it rather than asking a second time.
//! - **`egui`, after a delay it names.** It animates, it blinks a text cursor,
//!   it fades a tooltip in, and it says so as a `repaint_delay` on the frame's
//!   `ViewportOutput`. [`Repaint::asked`] is that number, and the delay is
//!   **honoured rather than collapsed to now** — repaint immediately for a
//!   250 ms animation and the animation becomes a spin at whatever rate the
//!   loop can manage, which is exactly the cost this module exists to stop
//!   paying.

use std::time::Duration;

use karakuri_operation::Operation;

use crate::input::Claim;
use crate::panel::Outcome;

/// When the panel wants to be on screen again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Repaint {
    /// Nothing on screen is changing. Sleep until something arrives — this is
    /// P-0072's first clause, and it is the answer to most of what happens.
    Never,
    /// What is on screen is not what should be. Draw at the first
    /// opportunity.
    Now,
    /// `egui` asked to be drawn again after this long, and it is naming a
    /// *deadline* rather than asking for a frame: draw then, and not before.
    After(Duration),
}

impl Repaint {
    /// Whether a frame is owed at all.
    pub const fn wanted(self) -> bool {
        !matches!(self, Repaint::Never)
    }

    /// What `egui` asked for, from the frame's
    /// `ViewportOutput::repaint_delay`.
    ///
    /// Its two sentinels are `egui`'s own and are documented there rather than
    /// guessed at: `Duration::MAX` is what a pass that wants nothing leaves
    /// behind, and zero is *"schedule a repaint immediately"*. Anything
    /// between the two is a deadline and is kept as one.
    pub fn asked(delay: Duration) -> Repaint {
        if delay == Duration::MAX {
            Repaint::Never
        } else if delay.is_zero() {
            Repaint::Now
        } else {
            Repaint::After(delay)
        }
    }

    /// The sooner of two answers.
    ///
    /// **The direction is the whole of the safety argument**: this can only
    /// bring a frame forward, never push one back, so combining the panel's
    /// answer with `egui`'s cannot lose either. [`Repaint::Never`] loses to
    /// everything and [`Repaint::Now`] beats everything, which leaves two
    /// deadlines to compare and the shorter wins.
    pub fn soonest(self, other: Repaint) -> Repaint {
        match (self, other) {
            (Repaint::Now, _) | (_, Repaint::Now) => Repaint::Now,
            (Repaint::Never, o) => o,
            (s, Repaint::Never) => s,
            (Repaint::After(a), Repaint::After(b)) => Repaint::After(a.min(b)),
        }
    }
}

/// **Everything that can change what the console shows, in one list.**
///
/// A list rather than a `bool` returned from a dozen places, because the
/// failure being prevented is an omission. Something new the panel reacts to
/// is a new variant here, and the `match` in [`Change::repaint`] does not
/// compile until it has been decided; a new `request_redraw()` beside a
/// handler is a decision nobody can find and no test can reach.
///
/// The operation arm carries what the model **returned** rather than what the
/// operator pressed, which is the difference between *a key was pressed* and
/// *something moved*: `z` with nothing folded reached the model and changed
/// nothing, and it earns no frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Change<'a> {
    /// The pointer moved, or a button went down or up, with `claim` saying who
    /// got the event ([`crate::input`]).
    Pointer(Claim),
    /// A wheel turned, with `claim` saying who got it.
    Wheeled(Claim),
    /// An operation ran — a fold, a solo, a reset, a report — and this is what
    /// it did.
    Operated(&'a Outcome),
    /// The room was toggled.
    ///
    /// **Its own variant because no outcome says so.** The room is the view's
    /// and not the model's: every colour on the panel changes and nothing in
    /// the arrangement moves, so a repaint decision asked only of
    /// [`crate::panel`] would leave the whole console in the other room's
    /// palette until something else happened to move. This is the key that
    /// changes what is drawn without touching the pointer.
    Room,
    /// **The Program bay rearranged itself**, with `moved` saying whether it
    /// actually did.
    ///
    /// The four deck previews go under the picture on a narrow bay and down
    /// the sides of a wide one, whichever leaves the picture larger
    /// ([ADR-0182](../../../docs/adr/0182-the-program-bays-body-arranges-itself-for-the-larger-picture.md)),
    /// and `crate::view::rearrange` is what decides it and writes the one bit
    /// that follows — the row is set aside beside the picture and put back
    /// under it.
    ///
    /// **Its own variant because nothing else on this list is it.** It is not
    /// an [`Outcome`]: no operator asked for it, nothing is saved, and the
    /// model was not operated on. It is not [`Change::Viewport`] either, even
    /// though a resize is what usually causes one — the bay rearranges itself
    /// off *its own* rectangle, so a drag on a boundary, a fold that gives it
    /// the height, a solo, and a canvas of a different shape all reach it
    /// without the window changing size at all. The window is the wrong thing
    /// to name it after, and naming a change after its usual cause is how a
    /// path with an unusual cause reaches no repaint.
    ///
    /// **`moved` is what the model returned and not what the caller asked**,
    /// which is exactly the operation arm's rule — `z` with nothing folded
    /// earns no frame. It matters more here than there: this is the one
    /// [`Change`] a caller raises **every frame** rather than on a gesture,
    /// because the bit is re-derived from the geometry every frame, so an arm
    /// that answered [`Repaint::Now`] regardless would ask for a frame on
    /// every frame and would cost the still panel the whole of P-0072's first
    /// clause.
    Rearranged {
        /// Whether the bit actually changed — `crate::view::rearrange`'s
        /// answer.
        moved: bool,
    },
    /// **A control the panel draws translated a gesture into an operation**,
    /// with `Some(op)` for one asked for and `None` for a gesture that asked
    /// for nothing.
    ///
    /// The mixer's two faders are what raise it: a drag on one emits
    /// [`karakuri_operation::Operation::SetGain`] or `SetOpacity`, the caller
    /// turns it into a record and applies it to the deck (P-0028), and the
    /// strip is drawn from what the deck says on the next frame.
    ///
    /// # Why it is not [`Change::Pointer`], which already covers the event
    ///
    /// Because that arm cannot say *nothing changed*, and here that case is
    /// real and common. A fader held against the top of its track while the
    /// pointer runs on asks for 1.0 sixty times a second; the value is already
    /// 1.0, nothing on the panel is different, and `Change::Pointer(Panel)` is
    /// [`Repaint::Now`] for every one of them.
    ///
    /// # Why it *may* be decided from what the drag returned, where the
    /// boundary's may not
    ///
    /// The pointer arm says outright that it is deliberately not decided from
    /// `Panel::moved`'s `Option`, because that one is `None` for a boundary
    /// that moved less than half a pixel — a threshold for what is worth
    /// *printing*, and half a logical pixel is a whole physical one on a 2x
    /// display, so a repaint decided from it leaves a boundary drawn where it
    /// no longer is.
    ///
    /// **A fader's `None` is an exact comparison of the value that would be
    /// sent**, not a threshold on a position: there is no distance below which
    /// the panel would look the same, because the same value *is* the same
    /// picture. So this arm is the [`Change::Rearranged`] shape — what the
    /// model returned, and a frame owed only where something happened — rather
    /// than the pointer's.
    ///
    /// The `Some` arm errs towards drawing, which is this module's stated
    /// direction: the caller is the one that applies the operation, and this
    /// cannot know that it did. It is a frame on a gesture an operator is
    /// making, which P-0072 does not budget.
    Emitted(Option<&'a Operation>),
    /// **Something on the panel is moving**, with the soonest staleness any
    /// live region on it declares — and `None` for a panel where nothing is.
    ///
    /// [`crate::view::View::animating`] is the answer, and it is the whole
    /// content of this arm: **the view is what knows the rate**, so the
    /// deadline is a number it hands over rather than one this module keeps.
    /// A rate written here would be a presentation's constant living where the
    /// presentation is not, and a roll redrawn at the wrong rate is a silent
    /// failure of exactly the kind this module's own opening describes.
    ///
    /// # Why it is a deadline and not a frame
    ///
    /// [`Repaint::After`] names *draw then, and not before*. A once-a-second
    /// animation asking for a frame instead is an animation that becomes a
    /// spin at whatever rate the loop can manage — which is what
    /// [`Repaint::asked`] already refuses to do to `egui`'s own delays, for
    /// the same reason.
    ///
    /// # Why it is raised every frame rather than on a gesture
    ///
    /// It is [`Change::Rearranged`]'s shape: a bit re-derived from the model
    /// every frame, and `None` has to be [`Repaint::Never`] or the still panel
    /// is gone. Nothing an operator does raises it — a slot is parked because
    /// the *governor* has not found room, which happens between frames and
    /// reaches this console through no event at all. So the frame that draws
    /// the panel is the only place that can notice, and the arm that answers
    /// for a panel with nothing pending is the one carrying P-0072's first
    /// clause.
    ///
    /// **A parked slot ends the still panel for as long as it is parked**, and
    /// that is the honest cost rather than an accident: `egui` is immediate
    /// mode, so what repaints is the panel and not the chip
    /// ([ADR-0188](../../../docs/adr/0188-a-pending-transition-says-it-is-pending-and-no-surface-holds-the-rule.md)).
    Animating(Option<Duration>),
    /// The window resized, or the display's scale factor changed: the
    /// arrangement is re-solved into a different viewport, so every rectangle
    /// on the panel is a new one.
    ///
    /// A scale change is here beside a resize rather than left to the resize
    /// that usually follows it, because *usually* is a platform's habit and
    /// not a guarantee, and the failure if it does not follow is a panel drawn
    /// at the wrong size with nothing saying so.
    Viewport,
}

impl Change<'_> {
    /// **The repaint decision, and the whole of it.**
    pub fn repaint(&self) -> Repaint {
        match self {
            // **A pointer event the panel claimed always earns a frame.**
            // `crate::input`'s rule hands the panel one in exactly two cases:
            // a boundary is in hand, or the pointer is within `GRAB` of one.
            // In the first the boundary is moving under the pointer. In the
            // second `crate::view::View::cursor` is drawing the resize cursor
            // from the hit, and the pointer crossing into or out of that band
            // is what changes it — so the frame is owed on the way in and on
            // the way out.
            //
            // **It is deliberately not decided from what `Panel::moved`
            // returned.** That is `None` for a boundary that moved less than
            // half a pixel, which is `panel`'s `WORTH_SAYING` — a threshold
            // for what is worth *printing* at sixty asks a second. Half a
            // logical pixel is a whole physical one on a 2x display, so a
            // repaint decided from that `Option` leaves the boundary drawn
            // where it no longer is: an under-repaint, silent, and in the
            // middle of the one gesture an operator is watching closely.
            //
            // A button the panel claimed is the start or the end of that
            // gesture and is drawn for the same reason — and where it is not
            // (a press over a boundary that never becomes a drag) it is one
            // frame on the operator's own action, which P-0072 does not
            // budget.
            Change::Pointer(Claim::Panel) => Repaint::Now,

            // `egui` has the event, and `EventResponse::repaint` is its
            // answer; asking again here would be a second one.
            Change::Pointer(Claim::Egui) | Change::Wheeled(Claim::Egui) => Repaint::Never,

            // The panel does nothing whatever with a wheel. `input::claim`
            // routes one to it only so that a wheel in the middle of a drag
            // cannot reach `egui` — which is a claim withheld, not an action
            // taken, and nothing on screen moves for it.
            Change::Wheeled(Claim::Panel) => Repaint::Never,

            Change::Operated(outcome) => match outcome {
                // A fold moves every region in the split it happened in, and
                // the siblings absorb what it gave up — folding the left pane
                // widens the centre. So the frame is owed for the panel and
                // not for the region named, which is why nothing here is
                // finer-grained than "draw".
                Outcome::Folded { .. } | Outcome::Soloed(_) | Outcome::Reset => Repaint::Now,
                // Both of these are asked speculatively — `u` with no solo,
                // `z` with nothing folded — and both say which it was.
                Outcome::Unsoloed { was } => match was {
                    true => Repaint::Now,
                    false => Repaint::Never,
                },
                Outcome::Unfolded(ids) => match ids.is_empty() {
                    false => Repaint::Now,
                    true => Repaint::Never,
                },
                // A report is a print and nothing else, and `Nothing` is an
                // operation that had nothing to act on — `g` on the root,
                // which is the one node with no split enclosing it. Neither
                // moved anything on screen, and **these are the arms that make
                // the clause true**: a key that reaches the model and changes
                // nothing costs nothing.
                //
                // Two more used to be here — the pointer on a divider, and
                // nothing under the pointer — and they left with the pointer
                // itself when an operation started naming its target
                // (`panel::Op`). They are not gone: they are what
                // `Panel::under` answers, in the caller, *before* an operation
                // is emitted, so those two keys now reach no `Change` at all
                // and are stiller than they were.
                Outcome::Report(_) | Outcome::Nothing => Repaint::Never,
            },

            // **A rearrangement that happened is every rectangle in the
            // Program bay being a new one** — the picture's and all four
            // cells' — which is `Change::Viewport`'s argument one bay down,
            // and the layout is dirty besides, so anything holding a rectangle
            // from before it is holding a stale one. It is drawn on the frame
            // that discovered it, and this arm is what covers the frames that
            // did not: a rearrangement decided anywhere but inside a pass is
            // otherwise a panel drawn in the arrangement it left.
            //
            // **A rearrangement that did not happen is the still panel**, and
            // this is the only arm on the list asked on every frame rather
            // than on a gesture — see the variant, where that argument is
            // written out.
            Change::Rearranged { moved } => match moved {
                true => Repaint::Now,
                false => Repaint::Never,
            },

            // See the variant: `None` is a drag that asked for nothing,
            // which is a pointer that moved over a value that did not.
            Change::Emitted(operation) => match operation {
                Some(_) => Repaint::Now,
                None => Repaint::Never,
            },

            // See the variant. The rate is the view's and arrives with the
            // change; `None` is a panel with nothing moving on it, and it is
            // `Never` rather than a long deadline because a panel that is
            // still is not a panel that is slow.
            Change::Animating(staleness) => match staleness {
                Some(staleness) => Repaint::After(*staleness),
                None => Repaint::Never,
            },

            Change::Room | Change::Viewport => Repaint::Now,
        }
    }
}
