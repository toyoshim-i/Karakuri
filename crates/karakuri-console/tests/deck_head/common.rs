pub(crate) use super::common::{at, console, drawn_once, near, rect_of, PLAUSIBLE, SMALLEST};
pub(crate) use karakuri_console::egui;
pub(crate) use karakuri_console::input::{claim, Claim};
pub(crate) use karakuri_console::panel::{Panel, GRAB};
pub(crate) use karakuri_console::room::{size, Room};
pub(crate) use karakuri_console::view::{
    deck_head, inspector, Aimed, DeckHead, InspectorPane, Pane, View, PANES, PANE_NAMES,
    SCRUB_BEATS, SYNCS,
};
pub(crate) use karakuri_layout::Point;
pub(crate) use karakuri_operation::{Operation, Sync};

/// Mock deck B configuration: beat-synced and engaged at 128 BPM with active scrub arrows.
pub(crate) fn mock() -> Pane {
    Pane {
        deck: 1,
        material: "lattice_veil".to_owned(),
        sync: Sync::Beat,
        allows: [true; SYNCS.len()],
        anchor_bpm: 128.0,
        scrub_beats: 0.25,
        composite: false,
        aimed: Some(aimed()),
        nodes: Vec::new(),
    }
}

/// Mock build chips for deck B based on `examples/lattice_shell.kir`.
///
/// Includes power-of-two capacity ladder within [4096, 262144] and sample salt.
pub(crate) fn aimed() -> Aimed {
    Aimed {
        capacity: 32768,
        stated: false,
        capacities: LADDER.to_vec(),
        salt: NEXT_SALT,
    }
}

/// The powers of two inside `lattice_shell`'s declared `[4096, 262144]`.
pub(crate) const LADDER: [u32; 7] = [4096, 8192, 16384, 32768, 65536, 131072, 262144];

/// The salt the mock's host hands the `re-salt` capsule.
pub(crate) const NEXT_SALT: u32 = 0x9E37_79B9;

/// That pane in some other mode, which is the only thing most of these tests
/// vary.
pub(crate) fn at_sync(sync: Sync) -> Pane {
    Pane { sync, ..mock() }
}

/// A view with a deck behind it — two panes, both showing `pane` — which is
/// what `View::draw` paints from and what `claim` hit-tests, one value.
pub(crate) fn view(pane: &Pane) -> View {
    let mut view = View::new(Room::Day);
    view.inspector = vec![pane.clone(); PANES];
    view
}

/// The laid-out pane, and the laid-out chips of its deck head inside it.
pub(crate) fn chips(
    panel: &Panel,
    ctx: &egui::Context,
    index: usize,
    pane: &Pane,
) -> (InspectorPane, DeckHead) {
    let at = inspector(panel.layout(), index, pane, 0.0).expect("a pane with room in it");
    let head = deck_head(ctx, &at, pane).expect("a deck head with room for its chips");
    (at, head)
}

/// Every mode, and the order is deliberately not the cycle's: what is asserted
/// below is that the cycle visits each of these once, and a list in the cycle's
/// own order could not tell that from a cycle that had lost one.
pub(crate) const EVERY: [Sync; 3] = [Sync::Beat, Sync::Free, Sync::Tempo];
