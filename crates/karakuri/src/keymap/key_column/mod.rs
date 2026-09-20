//! The key column of the manual, against the keys this window binds.
//!
//! [ADR-0213](../../../docs/adr/0213-the-interface-milestones-meter-is-the-panel-column-and-has-means-an-operator-reaches-it.md)
//! defined the *panel* column of `docs/manual/operations.html` — `has`
//! means an operator running the instrument reaches the operation — and
//! said nothing about the other three. The *key* column then stopped being
//! well defined, because
//! [ADR-0214](../../../docs/adr/0214-the-program-moves-out-of-the-cli-and-two-thin-binaries-sit-over-it.md)
//! gave this workspace a second keyboard: `karakuri-cli` binds thirty-nine
//! keys and this program binds every key in [`crate::KEYS`], seven
//! letters mean different things on the two, and a badge saying
//! `key f g` did not say whose.
//! (Nine when ADR-0220 was written; the library's load route added seven —
//! the four that select a deck, the two that walk the library cursor, and
//! `l`. The four are also the one place where the two keyboards
//! agree, because a deck is a slot number and there was nothing to
//! translate — `esc` was the other until 2026-09-09. Eight when the audio session landed and `b`,
//! `,` and `.` joined them, and seven since `p` stopped being the panel's
//! report and became the latency offset the page specifies — the one
//! letter this column has ever taken *back* from the panel, and the pair
//! `o` and `p` agree on both keyboards now. Eight since 2026-09-09,
//! when `esc` stopped quitting here and became ADR-0259's *up one level*:
//! it is the one key the two keyboards agreed on that they no longer do,
//! and the command line's is still its own — `karakuri-cli` is test tooling
//! and the instrument's principles do not bind it (ADR-0242).)
//!
//! The page now says whose, in its legend: the key column is the
//! instrument's keyboard, which is `window_event`'s `match` on
//! `key.logical_key` — the ten literal keys through
//! [`super::KEY_BINDINGS`] since 2026-09-10, the grammar's own four
//! through the guard beside it. That is ADR-0213's definition one column along — the
//! property is *the operator at the panel presses it*, and *which cargo
//! target binds a letter* is the shape ([P-0087](../../../docs/principles/0087-name-the-property-never-the-shape.md)).
//! This is the check that definition owes, both ways round, in the shape
//! `karakuri-environment/src/mcp.rs` uses for the MCP column and
//! `karakuri-console/tests/panel_column.rs` for the panel one.
//!
//! # Why the check is here and can be nowhere else
//!
//! The keys are in this file, and nothing in this workspace may depend on
//! this package — it is a binary with no library target on purpose, as
//! the crate header says: *a surface is where the buck stops*. The two
//! files that check the panel column both stop at exactly this boundary and
//! say so: `panel_column.rs` — *"reachability is a property of
//! `crates/karakuri/src/main.rs` … and this crate takes no device and
//! cannot depend on that binary (ADR-0156). So this file checks the
//! necessary half and not the sufficient one"* — and `vocabulary.rs` the
//! same. A key column check has that problem twice over, because the other
//! keyboard is in `karakuri-cli`, which no crate can depend on either.
//!
//! So it is a unit test in the binary that holds the keys. It cannot be an
//! integration test under `crates/karakuri/tests/`, because a package with
//! no library target has nothing for one to `use`; the arms are reachable
//! only from inside this file's own `#[cfg(test)]`.
//!
//! The command line's keyboard is not this file's and not this column's.
//! `karakuri-cli` documents its own keys in `BINDINGS` and has its own test
//! that every key `Live::key` acts on is in it. Nothing here reads that
//! package, and a second copy of its list here would be the thing
//! [`docs/contributing.md` §4](../../../docs/contributing.md)
//! forbids.
//!
//! # What it cannot see, and which way each one fails
//!
//! - A key `window_event` dispatches that is declared in none of
//!   [`bound`]'s four sources — [`super::KEY_BINDINGS`],
//!   [`GRAMMAR_KEYS`], [`DIGIT`] and [`NAME_ENTRY_KEY`] — through `egui`'s
//!   own shortcut handling, say. Invisible to [`bound`], and a *false
//!   negative*: it cannot fail the direction that says every bound key is
//!   on the page, and it surfaces from the other direction the moment
//!   somebody marks that row built. This one key wider than it was:
//!   the ten keys of [`super::KEY_BINDINGS`] cannot drift from what
//!   `window_event` dispatches — the same array is both, so there is
//!   nothing left to scan for and nothing left to miss — but
//!   [`GRAMMAR_KEYS`] and [`NAME_ENTRY_KEY`] are declared facts about
//!   `crate::grammar` and the two letter-taking flows rather than
//!   anything read out of them, so a key those stop binding, or start
//!   binding a different one, is invisible here exactly as it always was
//!   for [`DIGIT`].
//! - This file does not press a key. It reads the table, reads the
//!   grammar guard, and reads the page. That `Op::Solo` actually solos is
//!   `karakuri-console/tests/vocabulary.rs`'s, which asks a running `Panel`;
//!   that a binding is reached at all is what
//!   `tests::a_drag_through_the_window_loops_own_routing_never_reaches_egui`
//!   asks about the pointer, and nothing asks it for keys.
//!   `egui` sees every key before `window_event`'s `match` does, and if it ever
//!   grew a focused widget that consumed one, the binding would still be
//!   here and this file would go on claiming an operator reaches it. That
//!   is the sufficient half, and it is not checked here either — one
//!   boundary further out than the two files above stop at.
//! - Which rows a key lands on is written down rather than derived, in
//!   [`ROWS`]. It has to be: `Op::Fold` folds a bay or a pane depending on
//!   what the pointer is over, and only the page separates those two rows.
//!   A wrong entry is a wrong claim, and it cannot be *quietly* wrong —
//!   both assertions below read the same list, so an entry naming a row
//!   that is not marked built fails one and a badge naming a key no entry
//!   claims fails the other.
//! # The two arrangement rows are reached through the control that lists
//! them
//!
//! *Save the arrangement* and *Put a saved arrangement back* each carry a
//! name the operator picked, and no bare letter is bound to either: a key
//! press cannot type a name, and nothing at one says *which* arrangement.
//! What reaches them is the arrangement pill's menu, addressed — `enter`
//! puts the menu down, a digit names a row of it, and `enter` on that row
//! saves under the name in use or puts a filed arrangement back. Saving
//! with no name in use opens the field, which takes the keyboard whole
//! while it is asking (ADR-0259, ADR-0350). So both rows carry
//! `enter &middot; in the Transport` in the key column of [`PAGE`], and
//! [`NAME_ENTRY_KEY`] is the rub-out that flow binds.
//!
//! - Only the key column. The panel column is
//!   `karakuri-console`'s two files, the MCP column is
//!   `karakuri-environment/src/mcp.rs`, and the MIDI column is checked by
//!   nothing — which this file says rather than being read as covering
//!   it.

pub(super) mod data;
pub(super) mod scanner;
#[cfg(test)]
mod tests;

pub(crate) use data::*;
pub(crate) use scanner::*;
