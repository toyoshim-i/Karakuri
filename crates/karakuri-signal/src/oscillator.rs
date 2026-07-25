//! The local oscillator: the single source of truth for phase and tempo.
//!
//! Rendering reads this and never an external clock. External tempo input, when
//! it exists, is a correction applied here — so a dropped or jittering source
//! degrades the correction rather than the clock.

/// Phase and tempo, advanced by simulation steps rather than by wall time.
pub struct Oscillator {
    #[allow(dead_code)]
    bpm: f32,
    #[allow(dead_code)]
    phase: f32,
}

impl Oscillator {
    pub fn new(_bpm: f32) -> Oscillator {
        todo!("R1: oscillator")
    }
}
