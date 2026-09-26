//! Repaint scheduling and idle sleep policies for the console panel.
//!
//! Implements ADR-0164's still-panel policy, mapping state transitions ([`Change`]) to repaint
//! deadlines ([`Repaint`]) (ADR-0210, ADR-0283, ADR-0290).

use std::time::Duration;

use karakuri_operation::Operation;

use crate::input::Claim;
use crate::panel::Outcome;

/// When the panel wants to be on screen again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Repaint {
    /// Nothing on screen is changing. Sleep until something arrives — this is
    /// ADR-0164's still-panel clause, and it is the answer to most of what happens.
    Never,
    /// What is on screen is not what should be. Draw at the first opportunity.
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

    /// Converts `egui`'s requested `repaint_delay` into a [`Repaint`] deadline or sentinel.
    pub fn asked(delay: Duration) -> Repaint {
        if delay == Duration::MAX {
            Repaint::Never
        } else if delay.is_zero() {
            Repaint::Now
        } else {
            Repaint::After(delay)
        }
    }

    /// Returns the earlier of two repaint requests, prioritizing sooner deadlines.
    pub fn soonest(self, other: Repaint) -> Repaint {
        match (self, other) {
            (Repaint::Now, _) | (_, Repaint::Now) => Repaint::Now,
            (Repaint::Never, o) => o,
            (s, Repaint::Never) => s,
            (Repaint::After(a), Repaint::After(b)) => Repaint::After(a.min(b)),
        }
    }
}

/// Exhaustive enumeration of events that may change the console display and trigger a repaint.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Change<'a> {
    /// The pointer moved, or a button went down or up, with `claim` saying who got
    /// the event ([`crate::input`]).
    Pointer(Claim),
    /// Scroll wheel event with routing `claim` and whether content actually scrolled.
    Wheeled(Claim, bool),
    /// An operation ran — a fold, a solo, a reset, a report — and this is what it
    /// did.
    Operated(&'a Outcome),
    /// Text input into arrangement name pill, with `moved` indicating buffer modification.
    Naming(bool),
    /// Internal pointer movement (e.g. library cursor or deck selection), noting if position changed.
    Pointed(bool),
    /// Palette/room toggle requiring immediate panel repaint with new room colors.
    Room,
    /// Program bay layout rearrangement between vertical and horizontal preview positions.
    ///
    /// `moved` indicates whether the internal layout configuration actually changed.
    /// See ADR-0182 and ADR-0164.
    Rearranged {
        /// Whether the layout configuration changed.
        moved: bool,
    },
    /// Control gesture emitted an operation (`Some`), or had no effect (`None`) (P-0090, ADR-0210).
    Emitted(Option<&'a Operation>),
    /// Active animation with duration until the next visual change, or `None` if idle (ADR-0164, ADR-0188, ADR-0212, ADR-0283).
    Animating(Option<Duration>),
    /// Hover layer state changes: remaining dwell duration, dismissal, or still (ADR-0283, ADR-0330).
    Tip(crate::hover::Tip),
    /// Viewport size or display scale change requiring re-solving layout rectangles.
    Viewport,
}

impl Change<'_> {
    /// The repaint decision, and the whole of it.
    pub fn repaint(&self) -> Repaint {
        match self {
            // Panel-claimed pointer interaction (boundary drag or cursor hover band) repaints immediately (ADR-0210).
            Change::Pointer(Claim::Panel) => Repaint::Now,

            // `egui` has the event, and `EventResponse::repaint` is its
            // answer; asking again here would be a second one.
            Change::Pointer(Claim::Egui) | Change::Wheeled(Claim::Egui, _) => Repaint::Never,

            // Wheel scroll repaints only if content actually moved.
            Change::Wheeled(Claim::Panel, moved) => match moved {
                true => Repaint::Now,
                false => Repaint::Never,
            },

            Change::Operated(outcome) => match outcome {
                // Operations modifying visible layout structure trigger an immediate repaint.
                Outcome::Folded { .. }
                | Outcome::Soloed(_)
                | Outcome::Reset
                | Outcome::Restored => Repaint::Now,
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
                // Reports and no-op actions change nothing on screen and require no repaint (ADR-0164).
                Outcome::Report(_) | Outcome::Nothing => Repaint::Never,
            },

            // Layout rearrangement repaints immediately if layout changed; otherwise idle.
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

            // See the variant. The deadline is the view's and arrives with the
            // change; `None` is a panel with nothing moving on it, and it is
            // `Never` rather than a long deadline because a panel that is
            // still is not a panel that is slow.
            Change::Animating(moves_in) => match moves_in {
                Some(moves_in) => Repaint::After(*moves_in),
                None => Repaint::Never,
            },

            // Tooltip dwell schedules deadline; dismissal repaints immediately; stationary tip sleeps.
            Change::Tip(tip) => match tip {
                crate::hover::Tip::Dwelling(left) => Repaint::After(*left),
                crate::hover::Tip::Gone => Repaint::Now,
                crate::hover::Tip::Still => Repaint::Never,
            },

            Change::Room | Change::Viewport => Repaint::Now,

            // The caret moved, or a letter landed beside it. Nothing at all
            // where the buffer did not change — the key reached the console
            // and the console draws exactly what it drew.
            Change::Naming(moved) => match moved {
                true => Repaint::Now,
                false => Repaint::Never,
            },

            // A pointer moved, or a key asked it to and it was already at the
            // end of what it can point at. See the variant.
            Change::Pointed(moved) => match moved {
                true => Repaint::Now,
                false => Repaint::Never,
            },
        }
    }
}
