//! The window loop's own keyboard: [`KEY_BINDINGS`], the ten literal keys
//! `window_event` dispatches through it (`tab`, `esc`, `g`, `z`, `r`, `k`, `b`,
//! `,`, `.`, `n`), the [`KeyCtx`] each one's action takes instead of `&mut
//! App`, and the free function behind each row. [`key_column`] is the unit test
//! that holds the table against `docs/manual/operations.html` directly, and the
//! manual's own key column against the four keys `crate::grammar` answers for
//! beside it.
//!
//! Split out of `main.rs` on 2026-09-11, continuing that file's own `mod`-based
//! decomposition — `karakuri-console/src/view.rs`'s recent bay-by-bay split,
//! one binary crate along. `crate::session`'s `Sessions` and `Keeping` were the
//! first piece out; this is the second.
//!
//! Not `window_event`'s dispatch call site, which stays in `main.rs`: the
//! lookup into [`KEY_BINDINGS`], the [`KeyCtx`] built from `self`'s fields, and
//! the `match` on [`KeyAction`] that calls through it all read this table from
//! the other side of the crate boundary, exactly as `main.rs` already reads
//! `crate::session::Sessions` and `crate::session::Keeping`. Not `App`, not
//! `Readout` and its translator cluster, not `fn main` — separate, larger work
//! still ahead of it.
//!
//! [`key_column`]: key_column

use std::time::Instant;

use karakuri_console::focus;
use karakuri_console::panel::Op;
use karakuri_console::repaint::{Change, Repaint};
use karakuri_environment::Asked;
use karakuri_operation::{GridScale, Operation};
use winit::keyboard::{Key, NamedKey};

use crate::session::{Keeping, Sessions};
use crate::{Acted, App, Costs, Gfx, Readout};

/// A key `window_event` binds outside the grammar guard — the ~ten literal
/// `Key::Character("…")` and `Key::Named(NamedKey::…)` arms the `match` on
/// `key.logical_key` used to spell inline, one each in [`KEY_BINDINGS`] now.
///
/// Why a table and not the arms it replaced: those differed only in which key
/// they answered to and what they did about it, and a bare `match` said that in
/// five hundred lines a test could read only by re-parsing this file as text —
/// `key_column::bound`'s old shape, which stopped at the first `#[cfg(test)]`
/// and broke on a stray comment or a reordered arm. The facts are the same;
/// they are data now, and `key_column::bound` reads them as data instead of as
/// this file's own source.
///
/// Not the grammar guard, which stays exactly the `match` arm it always was —
/// `named if grammar(&named).is_some()` — because it is already table-driven
/// one layer down, through `karakuri_console::focus`, and binds a different key
/// in every bay it reaches. Nothing here duplicates it.
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

/// Exactly the fields a bound key's action needs, and not `self`.
///
/// `window_event` is already holding a live `&mut Gfx` reborrowed out of
/// `self.gfx` by the time a key is dispatched (see its own opening lines), so
/// an action taking `&mut App` would have to borrow all of `self` a second time
/// and collide with that borrow. `App::performed` and `App::wants` solved the
/// same problem the same way, by naming the individual fields they touch
/// instead of taking `&mut self` — this is that solution collected into one
/// struct because ten call sites named the same eight fields.
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

/// What pressing a bound key does, once [`KEY_BINDINGS`] has found its entry.
///
/// The arms this table replaced were not one shape: `Op::FoldEnclosing`,
/// `Op::UnfoldAll` and `Op::Reset` fell through to `self.readout.op(op)` below
/// the old `match`, and the rest — a view moved, an `Operation` emitted, a
/// frame asked for — handled the whole press themselves and returned. Rather
/// than force one payload on bodies that were never one shape, each variant
/// here carries the function pointer for the shape it is.
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

/// One row of the window loop's own keyboard, checked against
/// `docs/manual/operations.html` directly by `key_column`'s tests rather than
/// through a second list nothing holds against the page.
pub(crate) struct KeyBinding {
    pub(crate) key: BoundKey,
    /// The word this key is bound under, both in `key_column::KEYS`'s legend and in
    /// the page's own key-column badges — `docs/manual/operations.html`'s spelling,
    /// not `winit`'s.
    ///
    /// Read only by `key_column`'s tests — dispatch itself never asks this table
    /// what a key is *called*, only which one `matches`.
    #[cfg_attr(not(test), allow(dead_code))]
    legend: &'static str,
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
    title: Option<&'static str>,
    pub(crate) action: KeyAction,
}

/// Every key `window_event` binds outside the grammar guard.
///
/// Ten entries for the ten literal arms `window_event`'s `match` used to spell
/// — `tab`, `esc`, `g`, `z`, `r`, `k`, `b`, `,`, `.`, `n` — the same ten
/// `key_column::KEYS` prints beside the eight the grammar guard answers for.
/// What each one reaches on `docs/manual/operations.html` is
/// [`KeyBinding::title`], checked in `key_column` rather than assumed.
mod actions;
#[cfg(test)]
pub(crate) mod key_column;
mod table;

pub(crate) use actions::*;
pub(crate) use table::*;
