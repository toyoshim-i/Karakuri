use super::*;

pub mod arrangement;
pub mod audio_in;
pub mod look;
pub mod tempo;
pub mod theme;
pub mod tracker;

pub use arrangement::*;
pub(super) use arrangement::{arrangement_into, learn_into, map_into};
pub(super) use audio_in::audio_in_into;
pub use audio_in::*;
pub(super) use look::look_into;
pub(crate) use look::next_tonemap;
pub use look::*;
pub(super) use tempo::transport_into;
pub use tempo::*;
pub(super) use theme::theme_into;
pub use theme::*;
pub(super) use tracker::tracker_into;
pub use tracker::*;

impl View {
    /// Declares redraw staleness and budget for the transport row when visible and active (ADR-0193, ADR-0283, P-0094).
    pub(super) fn transport_declares(&self, layout: &karakuri_layout::Layout) -> Option<Declared> {
        let row = layout
            .find("transport")
            .is_some_and(|id| layout.visible(id));
        (row && self.transport.is_some()).then_some(Declared {
            region: "transport",
            cost: PANEL_PASS,
            staleness: BEAT_STALENESS,
            // Staleness and moves_in match as transport advances continuously per frame (ADR-0283, P-0094).
            moves_in: BEAT_STALENESS,
        })
    }
}
