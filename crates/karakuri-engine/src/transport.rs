//! Transport: what a deck slot's clock does with the session's.
//!
//! This is the mapping from session time to a slot's `t`, so that material can
//! be run at a rate, held, or scrubbed — tape-style fast-forward and rewind,
//! locked to the beat grid. It is **driven by a position rather than by a tempo
//! and a phase** — see
//! `docs/adr/0057-the-transport-is-driven-by-position-not-by-tempo-and-phase.md`:
//! a position can reverse and jump and a tempo cannot. Audio analysis supplies one that
//! only ever moves forward; a deck link supplies one that does not, and arrives
//! at the same entry point.
//!
//! ## Two mechanisms, because there are two kinds of material
//!
//! | | Accumulating | Closed form |
//! |---|---|---|
//! | What can be done to its clock | advanced, at a rate | **set**, to anything |
//! | So the transport offers | [`Sync::Tempo`] | that, and [`Sync::Beat`] |
//!
//! A closed-form procedure's state at `t` is a pure function of `seed`, `t` and
//! its parameters, so landing on a position costs one element pass. An
//! accumulating one integrates, and reversing a sum is not slow but impossible;
//! all it can be given is a rate. That is why [`Sync::Beat`] is refused on
//! accumulating material — see [`Transport::allows`] — and it is refused where
//! the operator asks for it, not where it would silently misbehave.
//!
//! ## The three modes
//!
//! - [`Sync::Free`] — the slot advances by whatever the session advanced by.
//!   What every slot did before this module existed.
//! - [`Sync::Tempo`] — the slot advances by that, scaled by
//!   `bpm / anchor_bpm`. The room speeds up, the material speeds up. **Rate
//!   only**: it never moves the slot to a position, so it never has to know
//!   where the material "should" be.
//! - [`Sync::Beat`] — the slot's clock is a function of the room's musical
//!   position. It jumps, it reverses, it is a lock rather than a follow.
//!
//! ## The anchor tempo, and why the material cannot supply it
//!
//! Beat sync has to answer "one beat of music is how many seconds of material",
//! and **material has no intrinsic tempo**: a `.kir` declares parameters and a
//! capacity, not a bar length. So the answer is a per-slot dial,
//! [`Transport::anchor_bpm`], and its default is the session tempo at the
//! moment sync was engaged. That default is chosen so that engaging it changes
//! nothing visible — the material is running at 1× at that instant and stays
//! there until the room's tempo moves.
//!
//! ## `beats`-reading material rides the slot's clock
//!
//! `Ambient::Beats` is read at the slot's own `t`, so a slot at 2× sees the grid
//! at 2× — it behaves as tape, which is what fast-forward should look like.
//! The consequence is that material *written against* the grid already follows
//! the room, and scaling its clock by the tempo as well makes it follow twice:
//! roughly the square of the tempo ratio, which reads as a broken artifact
//! rather than as two controls doing one job. [`Sync::Tempo`] is therefore
//! refused on material that reads `beats`, on the same terms as [`Sync::Beat`]
//! on accumulating material, and the check pass is what knows.
//!
//! Beat sync on such material is fine and is the interesting case: the slot is
//! *positioned* by the room, and the material reads the grid at that position.

use karakuri_signal::Oscillator;

use crate::set::MAX_STEPS;

/// What a slot's clock is locked to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sync {
    /// Wall time — the session's step count, verbatim.
    Free,
    /// The room's tempo, as a rate. Advances only forward.
    Tempo,
    /// The room's musical position. Jumps and reverses.
    Beat,
}

impl Sync {
    /// Every mode, in the order a control cycles them.
    pub const ALL: [Sync; 3] = [Sync::Free, Sync::Tempo, Sync::Beat];

    /// The wire and display spelling. A match rather than a lookup, so a mode
    /// added to the enum does not compile until it has one.
    pub fn name(self) -> &'static str {
        match self {
            Sync::Free => "free",
            Sync::Tempo => "tempo",
            Sync::Beat => "beat",
        }
    }

    pub fn from_name(name: &str) -> Option<Sync> {
        Sync::ALL.into_iter().find(|s| s.name() == name)
    }
}

/// Why a mode is not available for a Set. Carried rather than reduced to a
/// bool, because "greyed out" is only useful next to a reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// [`Sync::Beat`] on accumulating material. A position lock has to be able
    /// to land on a position, and this material can only be run forward.
    NotClosedForm,
    /// [`Sync::Tempo`] on material that reads `beats`. It already follows the
    /// room; scaling its clock as well would make it follow twice.
    AlreadyOnTheGrid,
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Refusal::NotClosedForm => write!(
                f,
                "the material accumulates, so it can be run forward but not placed — \
                 beat sync is a position lock"
            ),
            Refusal::AlreadyOnTheGrid => write!(
                f,
                "the material reads `beats`, so it already follows the room — tempo sync \
                 as well would make it follow twice"
            ),
        }
    }
}

/// What the deck should do with a slot this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Advance {
    /// Step forward by this many, the ordinary way. Never more than
    /// [`MAX_STEPS`].
    Steps(u8),
    /// Put the clock at this step count and evaluate there — one
    /// [`Set::seek`](crate::set::Set::seek), one prepare, one pass.
    SeekTo(u64),
}

/// One slot's mapping from the session's clock to its own.
#[derive(Debug, Clone, Copy)]
pub struct Transport {
    sync: Sync,
    anchor_bpm: f32,
    /// The operator's scrub, in beats, added to the room's position under
    /// [`Sync::Beat`]. Signed and unbounded: scrubbing is the one control here
    /// that is *meant* to go backwards.
    scrub_beats: f64,
    /// Fractional steps carried between frames under [`Sync::Tempo`], so a rate
    /// that does not divide the step count still advances at the right average
    /// rate. The same accumulator shape as spawn quantisation, for the same
    /// reason.
    carry: f32,
}

impl Default for Transport {
    /// Free-running: what every slot did before this module existed, so a deck
    /// nothing has configured behaves exactly as it used to.
    fn default() -> Transport {
        Transport {
            sync: Sync::Free,
            anchor_bpm: crate::binding::DEFAULT_BPM,
            scrub_beats: 0.0,
            carry: 0.0,
        }
    }
}

impl Transport {
    pub fn sync(&self) -> Sync {
        self.sync
    }

    pub fn anchor_bpm(&self) -> f32 {
        self.anchor_bpm
    }

    pub fn scrub_beats(&self) -> f64 {
        self.scrub_beats
    }

    /// **Whether a mode may be used on material with these properties**, and
    /// why not when it may not.
    ///
    /// A pure function of the two facts the check pass records, so a surface
    /// can grey out a control before the operator presses it and say why
    /// afterwards. Nothing here consults a Set; the deck passes the two flags
    /// in, which is what lets this be tested without a GPU.
    pub fn allows(sync: Sync, closed_form: bool, reads_beats: bool) -> Result<(), Refusal> {
        match sync {
            Sync::Free => Ok(()),
            Sync::Tempo if reads_beats => Err(Refusal::AlreadyOnTheGrid),
            Sync::Tempo => Ok(()),
            Sync::Beat if !closed_form => Err(Refusal::NotClosedForm),
            Sync::Beat => Ok(()),
        }
    }

    /// Engage a mode. `session_bpm` becomes the anchor whenever the mode
    /// changes, so **engaging sync never moves the picture**: the material is at
    /// 1× at that instant and stays there until the room's tempo does.
    ///
    /// Re-engaging the mode a slot is already in re-anchors it, which is how an
    /// operator says "call *this* the reference tempo" without a second
    /// control.
    ///
    /// The scrub is cleared with it. A slot brought back to the grid
    /// should be on the grid, not on wherever it was scrubbed to a song ago.
    pub fn engage(&mut self, sync: Sync, session_bpm: f32) {
        *self = Transport::engaged(sync, session_bpm);
    }

    /// What engaging a mode *means*, as a value rather than as a mutation.
    ///
    /// The policy lives here so that a caller building a record of the change
    /// and a caller applying one agree by construction: the record carries the
    /// anchor and the scrub explicitly, and this is the one place that decides
    /// what they are when an operator engages a mode by hand.
    pub fn engaged(sync: Sync, session_bpm: f32) -> Transport {
        Transport {
            sync,
            anchor_bpm: clamp_anchor(session_bpm),
            scrub_beats: 0.0,
            carry: 0.0,
        }
    }

    /// Set all three at once, for a caller applying a record. **Nothing is
    /// re-anchored and nothing is cleared** — the record says what the values
    /// are, and a replay that recomputed one of them from the machine it is
    /// running on would not be a replay.
    ///
    /// The carry is reset, because it is not state a record carries: it is a
    /// fraction of a step, its whole life is one frame either side, and
    /// recording it would put the smallest thing in the system into the format.
    pub fn set(&mut self, sync: Sync, anchor_bpm: f32, scrub_beats: f64) {
        self.sync = sync;
        self.anchor_bpm = clamp_anchor(anchor_bpm);
        self.scrub_beats = scrub_beats;
        self.carry = 0.0;
    }

    /// Move the scrub, in beats. Signed; nothing clamps it, because a position
    /// far outside the session is a position, and the seek below clamps at zero
    /// where it has to.
    ///
    /// Does nothing under [`Sync::Free`] and [`Sync::Tempo`] — those are rates,
    /// and a rate has no position to scrub. The caller is expected to say so
    /// rather than let the key read as broken.
    pub fn scrub(&mut self, beats: f64) {
        self.scrub_beats += beats;
    }

    /// Set the anchor directly, for a caller replaying a record. Clamped into
    /// the same range [`Transport::engage`] clamps into.
    pub fn set_anchor_bpm(&mut self, bpm: f32) {
        self.anchor_bpm = clamp_anchor(bpm);
    }

    pub fn set_scrub_beats(&mut self, beats: f64) {
        self.scrub_beats = beats;
    }

    /// **What this slot does this frame.**
    ///
    /// `session_steps` is what the deck advanced the session clock by, already
    /// clamped, `grid` is the session's oscillator positioned at this frame's
    /// last substep, and `dt` the fixed simulation step.
    ///
    /// **The slot's current position is deliberately not an input.** A seek
    /// computes where the slot should be and says so; comparing that against
    /// where it is would be the beginning of a rule about when a jump is worth
    /// making, and there is no such rule — a lock that sometimes declined to
    /// lock is worse than one that always does.
    pub fn advance(&mut self, session_steps: u8, grid: &Oscillator, dt: f32) -> Advance {
        match self.sync {
            Sync::Free => Advance::Steps(session_steps),
            Sync::Tempo => {
                // The rate the room is going at, relative to what this material
                // calls 1×. Accumulated rather than rounded per frame, so a
                // rate of 1.5 gives one step and then two rather than one and
                // then one.
                let rate = grid.bpm() / self.anchor_bpm;
                self.carry += f32::from(session_steps) * rate;
                let whole = self.carry.floor().max(0.0);
                self.carry -= whole;
                // **Clamped, and the clamp is not a rounding error.** Past
                // `MAX_STEPS` the simulation is allowed to fall behind rather
                // than catch up — the step-args buffer holds that many entries
                // and is sized at build time — so a rate above 4× is a rate
                // this mode cannot deliver. The carry is dropped with it rather
                // than accumulating a debt that would be repaid as a lurch the
                // moment the tempo came back down.
                let steps = (whole as u32).min(u32::from(MAX_STEPS)) as u8;
                if whole as u32 > u32::from(MAX_STEPS) {
                    self.carry = 0.0;
                }
                Advance::Steps(steps)
            }
            Sync::Beat => {
                // Where the room is, plus wherever the operator scrubbed to,
                // converted to material seconds by the anchor and then to a
                // step count — **a step count, not a time**. `t` is derived
                // from an integer counter and nothing may write it directly, or
                // two runs reaching the same instant stop being the same point
                // in the session.
                let beats = grid.beats() + self.scrub_beats;
                let seconds = beats * 60.0 / f64::from(self.anchor_bpm);
                let target = (seconds / f64::from(dt)).round();
                Advance::SeekTo(if target > 0.0 { target as u64 } else { 0 })
            }
        }
    }
}

/// The anchor is a tempo and is held inside the same range every other tempo in
/// this system is, for the same reason: a zero would divide the material's rate
/// by nothing.
fn clamp_anchor(bpm: f32) -> f32 {
    // NaN is the only value `clamp` cannot handle, and it is handled the way
    // `Oscillator`'s own tempo clamp handles it: the low bound rather than a
    // poisoned rate. An infinity clamps to an end of the range like any other
    // out-of-range number.
    if !bpm.is_nan() {
        bpm.clamp(
            *karakuri_signal::oscillator::BPM_RANGE.start(),
            *karakuri_signal::oscillator::BPM_RANGE.end(),
        )
    } else {
        *karakuri_signal::oscillator::BPM_RANGE.start()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 60.0;

    fn grid_at(bpm: f32, frames: usize) -> Oscillator {
        let mut osc = Oscillator::new(bpm);
        for _ in 0..frames {
            osc.advance(1, DT);
        }
        osc
    }

    /// Free is what a slot did before this module existed, and a deck that
    /// configures nothing must behave exactly as it used to.
    #[test]
    fn free_passes_the_sessions_own_step_count_through() {
        let mut t = Transport::default();
        assert_eq!(t.sync(), Sync::Free);
        for steps in [0u8, 1, 2, 4] {
            assert_eq!(
                t.advance(steps, &grid_at(128.0, 10), DT),
                Advance::Steps(steps)
            );
        }
    }

    /// **Engaging sync must not move the picture**, which is what the anchor
    /// defaulting to the session tempo buys: at that instant the rate is
    /// exactly 1 and the slot advances by what it would have anyway.
    #[test]
    fn engaging_tempo_sync_leaves_the_rate_at_one() {
        let mut t = Transport::default();
        t.engage(Sync::Tempo, 137.0);
        assert_eq!(t.anchor_bpm(), 137.0);
        assert_eq!(
            t.advance(1, &grid_at(137.0, 30), DT),
            Advance::Steps(1),
            "the material moved the moment sync was engaged"
        );
    }

    /// A rate that does not divide the step count still advances at the right
    /// average rate — the carry, which is the whole reason it is a field rather
    /// than a local.
    #[test]
    fn a_fractional_rate_averages_out_rather_than_rounding_every_frame() {
        let mut t = Transport::default();
        t.engage(Sync::Tempo, 100.0);
        let grid = grid_at(150.0, 1); // 1.5x
        let mut total = 0u32;
        for _ in 0..100 {
            match t.advance(1, &grid, DT) {
                Advance::Steps(n) => total += u32::from(n),
                Advance::SeekTo(_) => panic!("tempo sync must never seek"),
            }
        }
        assert_eq!(total, 150, "1.5x over 100 frames is 150 steps, not {total}");
    }

    /// **Past `MAX_STEPS` the simulation falls behind rather than catching up**,
    /// and the debt is dropped rather than repaid: a carry that kept growing
    /// while the tempo was above 4x would be spent as a lurch the moment it came
    /// back down, which is worse than the material having run slow.
    #[test]
    fn a_rate_past_the_step_cap_falls_behind_without_building_a_debt() {
        let mut t = Transport::default();
        t.engage(Sync::Tempo, 30.0);
        let fast = grid_at(300.0, 1); // 10x
        for _ in 0..50 {
            assert_eq!(t.advance(1, &fast, DT), Advance::Steps(MAX_STEPS));
        }
        // Back to 1x, and the very next frame is one step — not a burst.
        t.set_anchor_bpm(300.0);
        assert_eq!(t.advance(1, &fast, DT), Advance::Steps(1));
    }

    /// Beat sync lands on a **step count** derived from the room's position, so
    /// the material's `t` stays an integer multiple of `dt` — the invariant
    /// substepping rests on.
    #[test]
    fn beat_sync_lands_on_a_step_count_from_the_rooms_position() {
        let mut t = Transport::default();
        t.engage(Sync::Beat, 120.0);
        // Two beats in at 120 bpm is one second of room, and the anchor says
        // one second of material: 60 steps.
        let grid = grid_at(120.0, 60);
        // Not exact, and it is the f32 `dt` rather than anything here: sixty
        // additions of the f32 1/60 in f64 come to slightly over one second.
        assert!((grid.beats() - 2.0).abs() < 1e-6);
        assert_eq!(t.advance(1, &grid, DT), Advance::SeekTo(60));
    }

    /// The anchor is what converts musical position into material seconds, so
    /// halving it doubles how far the same music has carried the material.
    #[test]
    fn the_anchor_scales_how_far_the_music_carries_the_material() {
        let grid = grid_at(120.0, 60);
        let mut slow = Transport::default();
        slow.engage(Sync::Beat, 120.0);
        let mut fast = Transport::default();
        fast.engage(Sync::Beat, 60.0);
        assert_eq!(slow.advance(1, &grid, DT), Advance::SeekTo(60));
        assert_eq!(
            fast.advance(1, &grid, DT),
            Advance::SeekTo(120),
            "a lower anchor means the material is authored slower, so the same two beats \
             have to carry it twice as far"
        );
    }

    /// **The scrub goes backwards**, which is the one control here that is meant
    /// to, and it stops at zero rather than wrapping into a huge step count.
    #[test]
    fn scrubbing_reverses_and_stops_at_the_start() {
        let grid = grid_at(120.0, 60);
        let mut t = Transport::default();
        t.engage(Sync::Beat, 120.0);
        assert_eq!(t.advance(1, &grid, DT), Advance::SeekTo(60));

        t.scrub(-1.0);
        assert_eq!(
            t.advance(1, &grid, DT),
            Advance::SeekTo(30),
            "a beat back at 120 bpm is half a second, so thirty steps"
        );
        // Far enough back to leave the session behind. A `u64` cast of a
        // negative float is the trap here: it saturates to zero in Rust, but
        // the clamp is written out rather than relied upon.
        t.scrub(-1000.0);
        assert_eq!(t.advance(1, &grid, DT), Advance::SeekTo(0));
    }

    /// Engaging clears the scrub. A slot brought back to the grid should be on
    /// the grid, not on wherever it was scrubbed to a song ago.
    #[test]
    fn engaging_a_mode_clears_the_scrub() {
        let grid = grid_at(120.0, 60);
        let mut t = Transport::default();
        t.engage(Sync::Beat, 120.0);
        t.scrub(-1.0);
        assert_eq!(t.advance(1, &grid, DT), Advance::SeekTo(30));
        t.engage(Sync::Beat, 120.0);
        assert_eq!(t.advance(1, &grid, DT), Advance::SeekTo(60));
    }

    /// **The two refusals, and they are not the same refusal.** Beat sync needs
    /// material that can be placed; tempo sync needs material that is not
    /// already following the room. A Set can be refused one, the other, both,
    /// or neither, and every combination is reachable.
    #[test]
    fn each_mode_is_refused_for_its_own_reason() {
        // Ordinary accumulating material: rate yes, position no.
        assert_eq!(Transport::allows(Sync::Tempo, false, false), Ok(()));
        assert_eq!(
            Transport::allows(Sync::Beat, false, false),
            Err(Refusal::NotClosedForm)
        );
        // Closed form written against `t`: both.
        assert_eq!(Transport::allows(Sync::Tempo, true, false), Ok(()));
        assert_eq!(Transport::allows(Sync::Beat, true, false), Ok(()));
        // Closed form written against the grid: position yes, rate no —
        // scaling its clock would make it follow the tempo twice.
        assert_eq!(
            Transport::allows(Sync::Tempo, true, true),
            Err(Refusal::AlreadyOnTheGrid)
        );
        assert_eq!(Transport::allows(Sync::Beat, true, true), Ok(()));
        // Accumulating and written against the grid: neither.
        assert_eq!(
            Transport::allows(Sync::Tempo, false, true),
            Err(Refusal::AlreadyOnTheGrid)
        );
        assert_eq!(
            Transport::allows(Sync::Beat, false, true),
            Err(Refusal::NotClosedForm)
        );
        // Free is never refused. It is what a slot does when nothing is
        // arranged, so a Set that could be given no other mode still runs.
        for closed_form in [true, false] {
            for reads_beats in [true, false] {
                assert_eq!(
                    Transport::allows(Sync::Free, closed_form, reads_beats),
                    Ok(())
                );
            }
        }
    }

    /// Every mode round-trips through its name, so a record cannot spell one
    /// this build does not have and be silently read as another.
    #[test]
    fn every_sync_mode_survives_its_own_name() {
        for sync in Sync::ALL {
            assert_eq!(Sync::from_name(sync.name()), Some(sync));
        }
        assert_eq!(Sync::from_name("cooling"), None);
    }

    /// A zero anchor would divide the material's rate by nothing. Held inside
    /// the same range every other tempo in this system is.
    #[test]
    fn the_anchor_cannot_be_zero_or_nan() {
        let mut t = Transport::default();
        t.engage(Sync::Tempo, 0.0);
        assert_eq!(
            t.anchor_bpm(),
            *karakuri_signal::oscillator::BPM_RANGE.start()
        );
        t.engage(Sync::Tempo, f32::NAN);
        assert!(t.anchor_bpm().is_finite());
        t.set_anchor_bpm(f32::INFINITY);
        assert_eq!(
            t.anchor_bpm(),
            *karakuri_signal::oscillator::BPM_RANGE.end()
        );
    }
}
