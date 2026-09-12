//! Clock synchronization and transport mapping for deck slots.
//!
//! Maps session time and musical grids to a slot's simulation clock (`t`). Supports
//! free-running clocks ([`Sync::Free`]), tempo-rate scaling ([`Sync::Tempo`]), and
//! absolute beat-position locking ([`Sync::Beat`]).

use karakuri_signal::Oscillator;

use crate::set::MAX_STEPS;

/// Clock synchronization mode for a deck slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sync {
    /// Advances with session steps directly.
    Free,
    /// Advances scaled by room tempo relative to the anchor tempo.
    Tempo,
    /// Synchronizes directly with musical beat position.
    Beat,
}

impl Sync {
    /// All available sync modes.
    pub const ALL: [Sync; 3] = [Sync::Free, Sync::Tempo, Sync::Beat];

    /// Returns the string identifier for this mode.
    pub fn name(self) -> &'static str {
        match self {
            Sync::Free => "free",
            Sync::Tempo => "tempo",
            Sync::Beat => "beat",
        }
    }

    /// Parses a sync mode name, returning `None` if unrecognized.
    pub fn from_name(name: &str) -> Option<Sync> {
        Sync::ALL.into_iter().find(|s| s.name() == name)
    }
}

/// Reason a sync mode cannot be applied to a particular set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// [`Sync::Beat`] requested on non-closed-form (accumulating) material.
    NotClosedForm,
    /// [`Sync::Tempo`] requested on material that already samples musical beats.
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

/// Clock progression action for a slot during a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Advance {
    /// Step forward by the given step count.
    Steps(u8),
    /// Seek the simulation clock directly to the given step count.
    SeekTo(u64),
}

/// Maps the session clock to an individual deck slot's simulation time.
#[derive(Debug, Clone, Copy)]
pub struct Transport {
    sync: Sync,
    anchor_bpm: f32,
    /// Offset in musical beats applied under [`Sync::Beat`].
    scrub_beats: f64,
    /// Sub-step fractional accumulator for tempo scaling.
    carry: f32,
}

impl Default for Transport {
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

    /// Returns whether `sync` is compatible with the given set characteristics.
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
    /// Engages `sync` with `session_bpm` as the anchor tempo, resetting scrub and carry.
    pub fn engage(&mut self, sync: Sync, session_bpm: f32) {
        *self = Transport::engaged(sync, session_bpm);
    }

    /// Creates a new transport configured for `sync` anchored at `session_bpm`.
    pub fn engaged(sync: Sync, session_bpm: f32) -> Transport {
        Transport {
            sync,
            anchor_bpm: clamp_anchor(session_bpm),
            scrub_beats: 0.0,
            carry: 0.0,
        }
    }

    /// Updates sync parameters directly from stored or recorded values.
    pub fn set(&mut self, sync: Sync, anchor_bpm: f32, scrub_beats: f64) {
        self.sync = sync;
        self.anchor_bpm = clamp_anchor(anchor_bpm);
        self.scrub_beats = scrub_beats;
        self.carry = 0.0;
    }

    /// Adjusts the scrub beat offset by `beats`.
    pub fn scrub(&mut self, beats: f64) {
        self.scrub_beats += beats;
    }

    /// Sets the reference anchor tempo in BPM.
    pub fn set_anchor_bpm(&mut self, bpm: f32) {
        self.anchor_bpm = clamp_anchor(bpm);
    }

    pub fn set_scrub_beats(&mut self, beats: f64) {
        self.scrub_beats = beats;
    }

    /// Computes the clock advance or seek operation for the current frame.
    pub fn advance(&mut self, session_steps: u8, grid: &Oscillator, dt: f32) -> Advance {
        match self.sync {
            Sync::Free => Advance::Steps(session_steps),
            Sync::Tempo => {
                let rate = grid.bpm() / self.anchor_bpm;
                self.carry += f32::from(session_steps) * rate;
                let whole = self.carry.floor().max(0.0);
                self.carry -= whole;
                let steps = (whole as u32).min(u32::from(MAX_STEPS)) as u8;
                if whole as u32 > u32::from(MAX_STEPS) {
                    self.carry = 0.0;
                }
                Advance::Steps(steps)
            }
            Sync::Beat => {
                let beats = grid.beats() + self.scrub_beats;
                let seconds = beats * 60.0 / f64::from(self.anchor_bpm);
                let target = (seconds / f64::from(dt)).round();
                Advance::SeekTo(if target > 0.0 { target as u64 } else { 0 })
            }
        }
    }
}

/// Clamps a tempo value to the valid oscillator BPM range.
fn clamp_anchor(bpm: f32) -> f32 {
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

    #[test]
    fn beat_sync_lands_on_a_step_count_from_the_rooms_position() {
        let mut t = Transport::default();
        t.engage(Sync::Beat, 120.0);
        let grid = grid_at(120.0, 60);
        assert!((grid.beats() - 2.0).abs() < 1e-6);
        assert_eq!(t.advance(1, &grid, DT), Advance::SeekTo(60));
    }

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
        t.scrub(-1000.0);
        assert_eq!(t.advance(1, &grid, DT), Advance::SeekTo(0));
    }

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

    #[test]
    fn each_mode_is_refused_for_its_own_reason() {
        assert_eq!(Transport::allows(Sync::Tempo, false, false), Ok(()));
        assert_eq!(
            Transport::allows(Sync::Beat, false, false),
            Err(Refusal::NotClosedForm)
        );
        assert_eq!(Transport::allows(Sync::Tempo, true, false), Ok(()));
        assert_eq!(Transport::allows(Sync::Beat, true, false), Ok(()));
        assert_eq!(
            Transport::allows(Sync::Tempo, true, true),
            Err(Refusal::AlreadyOnTheGrid)
        );
        assert_eq!(Transport::allows(Sync::Beat, true, true), Ok(()));
        assert_eq!(
            Transport::allows(Sync::Tempo, false, true),
            Err(Refusal::AlreadyOnTheGrid)
        );
        assert_eq!(
            Transport::allows(Sync::Beat, false, true),
            Err(Refusal::NotClosedForm)
        );
        for closed_form in [true, false] {
            for reads_beats in [true, false] {
                assert_eq!(
                    Transport::allows(Sync::Free, closed_form, reads_beats),
                    Ok(())
                );
            }
        }
    }

    #[test]
    fn every_sync_mode_survives_its_own_name() {
        for sync in Sync::ALL {
            assert_eq!(Sync::from_name(sync.name()), Some(sync));
        }
        assert_eq!(Sync::from_name("cooling"), None);
    }

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
