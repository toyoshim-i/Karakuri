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

/// Number of Inspector panes configured in the layout (2).
pub const PANES: usize = 2;

/// Layout identifiers for the Inspector panes in display order.
pub const PANE_NAMES: [&str; PANES] = ["inspector-1", "inspector-2"];

/// Default deck targets for Inspector panes (deck A for pane 1, deck B for pane 2).
pub const PANE_DECKS: [u8; PANES] = [0, 1];

/// Inspector pane display state passed across the host seam per ADR-0156.
#[derive(Debug, Clone, PartialEq)]
pub struct Pane {
    /// Index of the deck currently targeted by this pane.
    pub deck: usize,
    /// What that deck is playing, which is the same name the deck's mixer strip
    /// carries and comes from the same place — see [`Strip::name`], and the short
    /// of it is that a `Set` has no name of its own and only whoever built it knows
    /// what to call it.
    pub material: String,
    /// What this deck's clock is locked to: `karakuri_engine::transport::Sync` as
    /// the vocabulary's copy of the same three.
    pub sync: Sync,
    /// Which [`SYNCS`] modes are supported by this deck's material per [P-0090] and ADR-0156.
    pub allows: [bool; SYNCS.len()],
    /// The tempo the deck was engaged at, which is what its rate is measured
    /// against. Drawn only under [`Sync::Tempo`] and [`Sync::Beat`] — see
    /// [`anchor_letter`].
    pub anchor_bpm: f32,
    /// Playback scrub offset in beats, drawn under [`Sync::Beat`].
    pub scrub_beats: f64,
    /// Whether layer compositing is active on this deck per [P-0090] and [ADR-0314].
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
