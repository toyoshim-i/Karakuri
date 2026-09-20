use super::*;

pub mod header;
pub mod params;
pub mod wiring;

pub use header::*;
pub use params::*;
pub use wiring::*;

pub(crate) use header::next_sync;

// ---------------------------------------------------------------------------
// The Inspector
// ---------------------------------------------------------------------------

/// How many panes the inspector has, which is `lib.rs`'s [`arrangement`] and
/// the mock's `.insp-split` read as one number: the CSS is
/// `grid-template-columns: 1fr 9px 1fr`, two tracks and the bar between them,
/// and the arrangement builds `inspector-1` and `inspector-2` to match.
///
/// A [`Spec`](karakuri_layout::Spec) builds a
/// [`Layout`](karakuri_layout::Layout) once and the arena has no insert, so the
/// count is settled at build time — the mock's `2 up ▾` is an operator choosing
/// it while running, and that is a control this pass does not add.
pub const PANES: usize = 2;

/// The arrangement's name for each pane, in the order the mock draws them.
///
/// Public for [`DECK_LETTERS`]'s reason: a harness that says *which pane* has
/// to say it in the names the arrangement addresses them by, and a second list
/// written out there would go on saying `inspector-1` the day this one does
/// not.
pub const PANE_NAMES: [&str; PANES] = ["inspector-1", "inspector-2"];

/// Which deck each pane opens pointed at — the first pane at deck A and the
/// second at deck B, which is the mock's own two heads.
///
/// It is what this console did before the pulldown existed, written down as a
/// *default* rather than left as the host's habit: the panes used to be filled
/// slot by slot, so a fourth slot could not be looked at at all. See
/// [`View::pane_deck`], which is the pointer this seeds.
///
/// Not a reading of anything, so a console with fewer strips than this names
/// opens with a pane pointed at a deck the mixer draws none for — which is a
/// pane with no nodes in it and is the honest state, exactly as
/// [`View::selection`]'s deck A is on a console with no deck behind it.
///
/// [`View::pane_deck`]: View::pane_deck
pub const PANE_DECKS: [u8; PANES] = [0, 1];

/// What one pane of the inspector is showing, handed in by whoever has a deck —
/// the same seam [`Strip`] crosses, one bay along.
///
/// `src/` takes no device and no engine (ADR-0156), so nothing here asks a
/// `Set` anything: every field is a value somebody who *can* ask read off one
/// and wrote down. `crates/karakuri` is where that reading is, and it is where
/// the two omissions below are decided as well.
#[derive(Debug, Clone, PartialEq)]
pub struct Pane {
    /// Which deck this pane is pointed at, as an index into [`DECK_LETTERS`] — the
    /// mock's `deck A`.
    ///
    /// The pane is pointed rather than choosing, and that is the mock's `showing …
    /// ▾` not being drawn: the chooser is a control and this pass adds none, so
    /// whoever fills this says which deck each pane shows.
    ///
    /// And it is not [`View::selection`], which this console does now keep. That is
    /// *the* deck — one value, what a key press is addressed to, drawn as one ring
    /// — and there are two panes: a chooser here picks a deck to *look at* while
    /// the keys stay where they were, which is the whole of why the mock draws a
    /// caret in each pane head and a ring on one strip. So this waits on a per-pane
    /// pointer nothing keeps, and reading the deck selection into it would fold two
    /// facts into one and make the second pane a copy of the first.
    pub deck: usize,
    /// What that deck is playing, which is the same name the deck's mixer strip
    /// carries and comes from the same place — see [`Strip::name`], and the short
    /// of it is that a `Set` has no name of its own and only whoever built it knows
    /// what to call it.
    pub material: String,
    /// What this deck's clock is locked to: `karakuri_engine::transport::Sync` as
    /// the vocabulary's copy of the same three.
    pub sync: Sync,
    /// Which of [`SYNCS`] this deck's material can honour, in that order — what the
    /// sync chip's cycle skips over.
    ///
    /// The answer and not the two facts it is computed from, which is the seam
    /// every other field here crosses read one step further along.
    /// `karakuri_engine::deck::Deck::sync_allowed` is what decides it, off whether
    /// the Set in the slot is closed form and whether it reads `beats`, and `src/`
    /// has no engine (ADR-0156) — so whoever owns one asks it three times and
    /// writes the three answers here. Handing in the two properties instead would
    /// put a third copy of `Transport::allows`' rule in a crate that owns no
    /// material, and a control is not the authority on what it may ask for
    /// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
    ///
    /// A property of the Set, so it moves when a build lands in the slot — the
    /// engine says so at `sync_allowed`, and it is why this is read beside
    /// [`Pane::sync`] rather than once.
    ///
    /// All three `true` is the whole of *nothing is refused*, which is what a
    /// console with no engine behind it and every test in this crate that does not
    /// say otherwise hands in; `Free` is refused by no material at all, so the
    /// first entry is never `false` in a reading anything took.
    pub allows: [bool; SYNCS.len()],
    /// The tempo the deck was engaged at, which is what its rate is measured
    /// against. Drawn only under [`Sync::Tempo`] and [`Sync::Beat`] — see
    /// [`anchor_letter`].
    pub anchor_bpm: f32,
    /// The scrub's own value, in beats, signed. Drawn only under [`Sync::Beat`],
    /// *"since that is the only mode that reads the offset"*.
    ///
    /// In beats and one deck's, where the transport row's offset is in milliseconds
    /// and is the whole instrument's — `style.css` says the unit is what tells them
    /// apart, *"so neither is ever drawn without one"*, and the mock's own `+0.25`
    /// is what this is drawn as.
    pub scrub_beats: f64,
    /// Whether this deck's Set folds its renderers into one result or overdraws
    /// them: `karakuri_engine::set::Layering`, as a bit.
    ///
    /// What the deck is doing, and what a press names the other of. The chip's
    /// tooltip is *"Click to overdraw them instead"*, and this is both the word
    /// [`deck_head_into`] draws and the state [`DeckHead::composite`] reads to say
    /// which layering a press is asking for — a destination and never a flip
    /// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
    ///
    /// It said the press was not a control until 2026-09-09, on the grounds that
    /// layering is a *build* decision in the engine — `Set::layering` answers off
    /// whether the Set was built with a merge, and nothing writes it afterwards.
    /// Both halves of that are still true and the conclusion was wrong: a rebuild
    /// is what this instrument already does to change what a slot is running, and
    /// the layering is one field of the aim a watcher is pointed at, so a press
    /// re-aims the slot and the worker rebuilds it — the route a library load
    /// takes, judged against the budget like any other build
    /// ([ADR-0314](../../../../docs/adr/0314-a-control-that-moves-a-field-of-the-aim-re-aims-the-slot-and-the-rebuild-is-the-write.md)).
    /// It is a readout of what landed rather than of what was asked for, which is
    /// `Mixer::residency`'s division: the build may still be rolled back, and the
    /// Staging lane is what says so.
    pub composite: bool,
    /// The two fields of this slot's aim the deck head can move, or `None` on a
    /// deck with no geometry to size and no randomness to seed — see [`Aimed`],
    /// which is where the argument is.
    pub aimed: Option<Aimed>,
    /// The node groups, in node order, which is the order a Set addresses its own
    /// nodes in.
    pub nodes: Vec<Node>,
}

mod dispatch;
mod layout;
mod pane;
mod render;
#[cfg(test)]
mod tests;

pub(crate) use layout::content_h;
#[cfg(test)]
pub(crate) use layout::pane_box;
pub use layout::*;
pub use pane::*;
pub(crate) use render::*;
