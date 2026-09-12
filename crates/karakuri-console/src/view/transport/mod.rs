use super::*;

pub mod arrangement;
pub mod audio_in;
pub mod look;
pub mod tempo;
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
pub(super) use tracker::tracker_into;
pub use tracker::*;

impl View {
    /// What the transport row declares: the beat's staleness for as long as the
    /// grid is being drawn, and nothing when it is not.
    ///
    /// # Two conditions, and neither of them is *pending*
    ///
    /// The row is laid out, which is ADR-0193 asked of a row rather than of a bay —
    /// [`Layout::visible`](karakuri_layout::Layout::visible) answers for
    /// `transport` exactly as it answers for `mixer`, and a folded row is not
    /// showing a beat, so it cannot be showing one out of date. The transport is a
    /// direct child of the unnamed root, so the only thing that encloses it is the
    /// root itself (ADR-0204).
    ///
    /// There are values behind it. [`View::transport`] is `None` for a console with
    /// no engine, and [`transport`] draws no row at all then — there is no light on
    /// a grid that is not there. That is the same seam as [`View::picture`] and not
    /// a second rule.
    ///
    /// Nothing else is asked, and that is the declaration's whole content. P-0094's
    /// forced clause is that *something is moving continuously while the console is
    /// live*, so a beat that declared only while something was pending would be the
    /// signal going quiet at the moment it is worth having. It is also why this is
    /// not folded into [`View::mixer_declares`]: two rates, two deadlines, two
    /// regions.
    ///
    /// # The health capsule declares nothing, and that is the right answer rather
    /// than an omission
    ///
    /// [`Transport::health`] changes when a build lands, is thrown out or fails to
    /// assemble, which is a person saving a file — so its own `moves_in` is *not
    /// until something happens*, and a region cannot declare that. It does not have
    /// to: the unit of declaration is the region and not the presentation
    /// ([`Declared`] — *"a second rate would be a second declaration; a second user
    /// of one rate is not"*), and the region it is in is already drawn at
    /// [`BEAT_STALENESS`] for as long as there is a row at all. A staleness written
    /// for the capsule would be a rate for something that does not move (ADR-0283),
    /// and it could only ever be slower than the beat's, so it would change nothing
    /// a window does — [`View::animating`] takes the soonest.
    pub(super) fn transport_declares(&self, layout: &karakuri_layout::Layout) -> Option<Declared> {
        let row = layout
            .find("transport")
            .is_some_and(|id| layout.visible(id));
        (row && self.transport.is_some()).then_some(Declared {
            region: "transport",
            cost: PANEL_PASS,
            staleness: BEAT_STALENESS,
            // **The one region whose two numbers are the same number**, and
            // that is what an honest *I move at this rate* looks like: the
            // light is on the grid at [`Transport::beats`], the harness
            // advances the session by one step on every frame it composes, so
            // every frame this declaration buys draws the light somewhere it
            // was not (ADR-0283). There is no rest to find and nothing to
            // gate — P-0094 is the rule that says there had better not be.
            moves_in: BEAT_STALENESS,
        })
    }
}
