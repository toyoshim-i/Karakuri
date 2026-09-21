use std::time::Duration;

use super::*;
use crate::budget::{Declared, PANEL_PASS};
use crate::view::sequencer::{step_moves_in, STEP_STALENESS};

// ---------------------------------------------------------------------------
// View frame budget declarations and animation staleness
// ---------------------------------------------------------------------------

impl View {
    /// Every live region that is declaring this frame, each with what one update of
    /// it costs, how stale it may get, and when its picture is next different from
    /// the one on screen.
    ///
    /// The first two are P-0091's and are constants of the presentation; the third
    /// is [`crate::budget::Declared::moves_in`], it is a function of the frame, and
    /// it exists because *how often must this be drawn* and *is this moving now*
    /// are two questions and only one of them was being asked
    /// ([ADR-0283](../../../../docs/adr/0283-a-region-declares-when-its-picture-next-changes-not-that-something-is-pending.md)).
    ///
    /// # This is P-0091's naming, and the unit is a region
    ///
    ///
    /// [P-0091](../../../../docs/principles/0091-cost-is-known-before-it-is-paid.md):
    /// *"Anything that must be live names two numbers — what its update costs, and
    /// how stale it may get in milliseconds."* This is that naming, and
    /// [`crate::budget`] holds the numbers with the arguments for where each came
    /// from. A region is redrawn whole or not at all, so what appears here is a
    /// node of the arrangement — by the name every surface addresses it by — and
    /// never an animation, a control or a slice of a frame.
    ///
    /// # Two regions, at two rates, for two different reasons
    ///
    /// The transport row, whenever the beat grid is drawn: the light travels the
    /// grid once a bar and it is the panel's continuous motion, which
    /// [P-0094](../../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)
    /// says is how a stopped panel announces itself. It declares [`BEAT_STALENESS`]
    /// and it does not ask whether anything is pending, which is the whole point of
    /// it: a signal that only ran while something was happening would be quiet
    /// exactly when the panel had gone quiet.
    ///
    /// The mixer bay, while something in it is outstanding. Inside it the tally's
    /// word rolls toward a residency that has not been granted and each of a
    /// strip's two faders reaches toward a value a transition has not reached yet —
    /// three presentations at one rate, off one [`Phase`], so they are one term and
    /// not three
    /// ([ADR-0190](../../../../docs/adr/0190-the-parked-tally-rolls-because-two-lamps-do-not-fit-in-fifty-three-pixels.md),
    /// [ADR-0206](../../../../docs/adr/0206-a-fader-marks-where-it-is-going-and-keeps-reaching-for-it.md)).
    /// A second *rate* would be a second declaration; a second *user* of one rate
    /// is not — which is why the beat is a second entry here and the two faders are
    /// not.
    ///
    /// This is the first frame on which either sum has two terms, and
    /// `tests/schedulable.rs` reads them: `Σ (cost / staleness)` is 0.0889 against
    /// 1.0, and `max(cost)` is still one number because both regions declare the
    /// same whole panel pass (ADR-0210).
    ///
    /// # Pending is not enough: the region that shows it has to be laid out
    ///
    /// A region that is not on screen declares nothing, however much is pending
    /// behind it. The strips are rewritten every frame from the deck, so *is
    /// anything pending* is a fact about the deck; P-0091 is about what must be
    /// live, and a bay the operator has folded away is not live. A declaration made
    /// for it buys a repaint of something nobody can see — measured, before this
    /// asked: with the picture and the preview row folded the window sat at 28.7 to
    /// 29.0 frames a second, and folding the mixer bay on top of that moved the
    /// price of a frame and not the rate. It draws nothing at all now. The
    /// alternative — declare it anyway and let a scheduler drop it — is
    /// [ADR-0193](../../../../docs/adr/0193-a-region-that-is-not-laid-out-declares-nothing-rather-than-being-dropped-later.md),
    /// which is where it lost, and it lost on the sentence being false rather than
    /// unaffordable: a region nobody can see is not showing anything, so it cannot
    /// be showing anything out of date.
    ///
    /// This is
    /// [P-0073](../../../../docs/principles/0073-a-node-claims-only-what-its-visible-content-can-use.md)
    /// in time rather than in space — *a node claims only what its visible content
    /// can use*, where what is claimed is a share of the frame budget rather than a
    /// share of the viewport.
    ///
    /// The question is asked of the layout, which already answers it.
    /// [`Layout::visible`](karakuri_layout::Layout::visible) is ADR-0183's
    /// disjunction — the operator's fold and the drawing's `set_aside`, read as one
    /// — walked up the ancestors, so a folded right pane takes the mixer with it
    /// and no second derivation of *is this laid out* is written here. It is not
    /// [`mixer`]'s `None`, which is a different question and a stricter one: that
    /// answers *can the strips be laid out in this rectangle this pass*, and says
    /// no for a window merely too small and for the frame before `egui` has fonts.
    /// Neither is a reason to stop the roll — only being out of the layout is — and
    /// the layout answers this one without a solve, which matters because the fold
    /// is applied and this is asked before the next one.
    ///
    /// # Nothing arbitrates between two of these
    ///
    /// ADR-0164's second half is a scheduler and there is not one. What this feeds
    /// is [`View::animating`], which takes the soonest staleness and nothing else,
    /// and `tests/schedulable.rs`, which sums over whatever this answers and
    /// asserts the two conditions the principle states.
    pub fn declares(&self, layout: &karakuri_layout::Layout) -> impl Iterator<Item = Declared> {
        // An array rather than a `Vec`, so asking what the panel declares
        // allocates nothing on a path that is walked every frame — and so that
        // the second live region was one more element rather than a change of
        // shape, which is what it turned out to be. In `REGIONS`' order, so
        // the declarations read down the panel.
        [
            self.transport_declares(layout),
            self.mixer_declares(layout),
            self.sequencer_declares(layout),
        ]
        .into_iter()
        .flatten()
    }

    /// What the Sequencer bay declares: the step's staleness while it has a
    /// playhead to move *and* the bay is laid out, and nothing otherwise — with the
    /// deadline taken from where the beat has got to inside the current step.
    ///
    /// # Three conditions, and the third is what makes it honest
    ///
    /// The bay is laid out, which is ADR-0193 and is [`View::mixer_declares`]'s
    /// first condition word for word.
    ///
    /// There is a pattern behind it. [`View::sequencer`] is `None` for a console
    /// with no session, and [`sequencer`] draws nothing then.
    ///
    /// And it has a lane. The picture that moves here is the playhead column, which
    /// stands over the rows — so a pattern with no lanes has nothing for it to
    /// stand on and this bay is a head and a ruler that do not move. A declaration
    /// made for it would buy frames that redraw a still picture, which is the whole
    /// of what
    /// [ADR-0283](../../../../docs/adr/0283-a-region-declares-when-its-picture-next-changes-not-that-something-is-pending.md)
    /// is about. A muted lane still counts: the mute stops the lane *writing*, and
    /// the column goes on crossing its cells.
    ///
    /// # The deadline is the music's and the rate is not
    ///
    /// [`step_moves_in`] is a function of `beats` and the tempo, so the deadline
    /// moves with the grid the way the steps do; [`STEP_STALENESS`] is a constant
    /// at the mock's tempo, so the sums `tests/schedulable.rs` asserts do not
    /// become a function of how fast the music is (ADR-0212).
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
