//! Key bindings and dispatch table for window event handling.
//!
//! Maps keyboard shortcuts to panel actions, focus transitions, or handled operations.

use std::time::Instant;

use karakuri_console::focus;
use karakuri_console::panel::Op;
use karakuri_console::repaint::{Change, Repaint};
use karakuri_environment::Asked;
use karakuri_operation::{GridScale, Operation};
use winit::keyboard::{Key, NamedKey};

use crate::session::{Keeping, Sessions};
use crate::{Acted, App, Costs, Gfx, Readout};

/// A bound key representation (named key or character literal) outside grammar guards.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BoundKey {
    Named(NamedKey),
    Character(&'static str),
}

impl BoundKey {
    /// Whether `key`, as `window_event` hands it in from
    /// `key.logical_key.as_ref()`, is this one.
    pub(crate) fn matches(self, key: &Key<&str>) -> bool {
        match (self, key) {
            (BoundKey::Named(named), Key::Named(other)) => named == *other,
            (BoundKey::Character(text), Key::Character(other)) => text == *other,
            _ => false,
        }
    }
}

/// Context fields required by bound key actions to avoid reborrow conflicts with `&mut App`.
pub(crate) struct KeyCtx<'a> {
    pub(crate) readout: &'a mut Readout,
    pub(crate) egui_due: &'a mut Option<Instant>,
    pub(crate) costs: &'a mut Costs,
    pub(crate) recording: &'a mut Sessions,
    pub(crate) keeping: &'a mut Keeping,
    pub(crate) store: &'a std::path::Path,
    pub(crate) started: Instant,
    pub(crate) shift: bool,
}

/// Action dispatched by a bound key: panel operation, handled side-effect, or focus move.
#[derive(Clone, Copy)]
pub(crate) enum KeyAction {
    /// Produces the `panel::Op` that falls through to `self.readout.op(op)`, or
    /// `None` where the press has nothing to act on (`g` off every bay).
    Panel(fn(&mut KeyCtx) -> Option<Op>),
    /// Handles the whole press — asks for whatever frame it owes, if any — and the
    /// window loop returns immediately after calling it.
    Handled(fn(&mut KeyCtx, &mut Gfx)),
    /// Moves focus between bays or within a bay and returns whether focus moved.
    Focus(fn(&mut KeyCtx) -> bool),
}

impl std::fmt::Debug for KeyAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyAction::Panel(_) => write!(f, "Panel(fn)"),
            KeyAction::Handled(_) => write!(f, "Handled(fn)"),
            KeyAction::Focus(_) => write!(f, "Focus(fn)"),
        }
    }
}

/// One row of the window loop's own keyboard, checked against
/// `docs/manual/operations.html` directly by `key_column`'s tests rather than
/// through a second list nothing holds against the page.
#[derive(Debug, Clone, Copy)]
pub(crate) struct KeyBinding {
    pub(crate) key: BoundKey,
    /// Human-readable key name matching documentation and key-column tests.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) legend: &'static str,
    /// The bay a badge naming this key is addressed to — `Some(focus::ANY)` for the
    /// one key of this table that is not global, `None` for every other one. Read
    /// only by `key_column`'s tests, for `legend`'s reason.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) bay: Option<&'static str>,
    /// The `<h3>` title this key reaches on the page, or `None` for a view-only
    /// action the page does not specify as an operation — moving focus, abandoning
    /// a name, the room's colours. Read only by `key_column`'s tests, for
    /// `legend`'s reason.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) title: Option<&'static str>,
    pub(crate) action: KeyAction,
    /// Whether this bay-scoped binding has been explicitly promoted by the user
    /// to be active globally across all bays (with collision warnings).
    pub(crate) globalize: bool,
}

/// Action handlers for table key bindings.
mod actions;
pub(crate) mod custom;
#[cfg(test)]
pub(crate) mod key_column;
mod table;

pub(crate) use actions::*;
pub(crate) use custom::*;
pub(crate) use table::*;
