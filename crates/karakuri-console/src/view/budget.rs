use std::time::Duration;

use super::*;
use crate::budget::{Declared, PANEL_PASS};
use crate::view::sequencer::{step_moves_in, STEP_STALENESS};

// ---------------------------------------------------------------------------
// View frame budget declarations and animation staleness
// ---------------------------------------------------------------------------

impl View {
    /// Declares frame budget (cost, staleness, next change) for visible live regions.
    ///
    /// Hidden/folded regions declare nothing (ADR-0193, P-0073). Pacing and staleness
    /// follow P-0091, P-0094, ADR-0164, ADR-0190, ADR-0206, ADR-0210, and ADR-0283.
    pub fn declares(&self, layout: &karakuri_layout::Layout) -> impl Iterator<Item = Declared> {
        // Stack array avoids per-frame allocations; follows `REGIONS` layout order.
        [
            self.transport_declares(layout),
            self.mixer_declares(layout),
            self.sequencer_declares(layout),
        ]
        .into_iter()
        .flatten()
    }

    /// Declares sequencer step staleness if visible, active, and containing lanes.
    ///
    /// Frame deadline tracks beat progress (ADR-0212, ADR-0283) while staleness
    /// remains constant. Bay must be visible and have active lanes (ADR-0193).
    fn sequencer_declares(&self, layout: &karakuri_layout::Layout) -> Option<Declared> {
        let bay = layout
            .find("sequencer")
            .is_some_and(|id| layout.visible(id));
        let reading = self.sequencer.as_ref()?;
        let transport = self.transport.as_ref()?;
        (bay && !reading.pattern.lanes().is_empty()).then(|| Declared {
            region: "sequencer",
            cost: PANEL_PASS,
            staleness: STEP_STALENESS,
            moves_in: step_moves_in(reading.pattern.mode(), transport.beats, transport.bpm),
        })
    }

    /// Returns the duration until the soonest visual change across all live regions, or `None` if static.
    ///
    /// See ADR-0164 and ADR-0283 for frame pacing and animation declarations.
    pub fn animating(&self, layout: &karakuri_layout::Layout) -> Option<Duration> {
        self.declares(layout).map(|live| live.moves_in).min()
    }
}
