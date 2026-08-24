//! **When the panel is drawn again, and when the window sleeps.**
//!
//! [P-0072](../../../docs/principles/0072-a-still-panel-costs-nothing-and-what-moves-declares-its-price.md)'s
//! first clause and nothing else: *a panel with nothing changing on it is paid
//! for once and not again.* A window loop that drives itself pays
//! [ADR-0164](../../../docs/adr/0164-the-panel-is-budgeted-rather-than-forbidden-to-allocate.md)'s
//! measured 184 allocations and 226.2 kB on every frame nobody is touching;
//! this is the decision that stops those frames being drawn at all.
//!
//! **The declarations, the scheduler and the two schedulability conditions are
//! not here.** Nothing is live on the panel yet to declare a cost or a
//! staleness, and a scheduler with no clients is an abstraction with no call
//! sites. They arrive with the first bay that needs them.
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

            Change::Room | Change::Viewport => Repaint::Now,
        }
    }
}
