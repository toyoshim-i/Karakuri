//! Scheduled transitions for mix parameters across musical time.
//!
//! Provides types for scheduling moves of scalar controls ([`Control`]) such as
//! gain, opacity, or mask position over a duration measured in musical beats.
//! Also provides [`Selection`] for scheduling active renderer choices on musical boundaries.

use crate::binding::Curve;

/// Identifies which mix control a transition moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Control {
    /// Signal gain factor.
    Gain,
    /// Blend opacity.
    Opacity,
    /// Mask edge position in the range [0, 1].
    MaskPosition,
}

impl Control {
    /// All available control variants.
    pub const ALL: [Control; 3] = [Control::Gain, Control::Opacity, Control::MaskPosition];

    /// Returns the wire and display name for this control.
    pub fn name(self) -> &'static str {
        match self {
            Control::Gain => "gain",
            Control::Opacity => "opacity",
            Control::MaskPosition => "mask",
        }
    }

    /// Parses a control name, returning `None` if unrecognized.
    pub fn from_name(name: &str) -> Option<Control> {
        Control::ALL.iter().copied().find(|c| c.name() == name)
    }
}

/// A scheduled control parameter movement over musical time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transition {
    slot: usize,
    control: Control,
    from: f32,
    to: f32,
    /// Start position on the beat clock.
    start: f64,
    /// Duration in beats. Zero represents an immediate cut.
    beats: f64,
    curve: Curve,
}

impl Transition {
    /// Creates a new transition from `from` to `to`, beginning at `start` beats.
    pub fn new(
        slot: usize,
        control: Control,
        from: f32,
        to: f32,
        start: f64,
        beats: f64,
        curve: Curve,
    ) -> Transition {
        Transition {
            slot,
            control,
            from,
            to,
            start: if start.is_finite() { start } else { 0.0 },
            beats: if beats.is_finite() {
                beats.max(0.0)
            } else {
                0.0
            },
            curve,
        }
    }

    pub fn slot(&self) -> usize {
        self.slot
    }

    pub fn control(&self) -> Control {
        self.control
    }

    pub fn to(&self) -> f32 {
        self.to
    }

    pub fn start(&self) -> f64 {
        self.start
    }

    pub fn beats(&self) -> f64 {
        self.beats
    }

    pub fn curve(&self) -> Curve {
        self.curve
    }

    /// Evaluates the control value at `beats`, or returns `None` before the transition starts.
    pub fn value_at(&self, beats: f64) -> Option<f32> {
        if beats < self.start {
            return None;
        }
        if self.beats <= 0.0 || beats >= self.start + self.beats {
            return Some(self.to);
        }
        let t = (beats - self.start) / self.beats;
        let shaped = self.curve.apply(t as f32);
        Some(self.from + (self.to - self.from) * shaped)
    }

    /// Returns whether this transition has completed at `beats`.
    pub fn finished(&self, beats: f64) -> bool {
        beats >= self.start + self.beats
    }
}

/// A scheduled switch of the active renderer in a slot's set at a specific beat position.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Selection {
    slot: usize,
    renderer: usize,
    start: f64,
}

impl Selection {
    /// Creates a selection landing at `start` beats.
    pub fn new(slot: usize, renderer: usize, start: f64) -> Selection {
        Selection {
            slot,
            renderer,
            start: if start.is_finite() { start } else { 0.0 },
        }
    }

    pub fn slot(&self) -> usize {
        self.slot
    }

    pub fn renderer(&self) -> usize {
        self.renderer
    }

    pub fn start(&self) -> f64 {
        self.start
    }

    /// Returns whether this selection is due at or after `beats`.
    pub fn due(&self, beats: f64) -> bool {
        beats >= self.start
    }
}

/// Calculates the next musical instant on a grid of `quantum` beats at or after `beats`.
///
/// If `quantum` is zero or non-positive, returns `beats` unchanged.
pub fn quantise(beats: f64, quantum: f64) -> f64 {
    if !(quantum.is_finite() && quantum > 0.0) || !beats.is_finite() {
        return beats;
    }
    let next = (beats / quantum).ceil() * quantum;
    if next < beats {
        next + quantum
    } else {
        next
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fade(from: f32, to: f32, start: f64, beats: f64, curve: Curve) -> Transition {
        Transition::new(0, Control::Opacity, from, to, start, beats, curve)
    }

    #[test]
    fn a_transition_is_exact_at_both_ends_under_every_curve() {
        for curve in crate::binding::CURVES {
            let t = fade(0.25, 0.75, 10.0, 4.0, curve);
            assert_eq!(t.value_at(10.0), Some(0.25), "{}", curve.name());
            assert_eq!(t.value_at(14.0), Some(0.75), "{}", curve.name());
            // And past the end it stays there rather than continuing.
            assert_eq!(t.value_at(99.0), Some(0.75), "{}", curve.name());
        }
    }

    #[test]
    fn a_transition_that_has_not_started_leaves_its_control_alone() {
        let t = fade(0.25, 0.75, 10.0, 4.0, Curve::Lin);
        assert_eq!(t.value_at(0.0), None);
        assert_eq!(t.value_at(9.999), None);
        assert!(!t.finished(9.999));
        assert_eq!(t.value_at(-5.0), None);
    }

    #[test]
    fn a_duration_of_zero_is_a_cut_rather_than_a_division() {
        let t = fade(0.0, 1.0, 10.0, 0.0, Curve::Smooth);
        assert_eq!(t.value_at(9.9), None);
        assert_eq!(t.value_at(10.0), Some(1.0));
        assert!(t.finished(10.0));
        assert_eq!(t.value_at(10.1), Some(1.0));
    }

    #[test]
    fn a_negative_or_absent_duration_is_a_cut() {
        for beats in [-4.0, f64::NAN, f64::NEG_INFINITY, f64::INFINITY] {
            let t = fade(0.0, 1.0, 10.0, beats, Curve::Lin);
            assert_eq!(t.beats(), 0.0, "{beats}");
            assert_eq!(t.value_at(10.0), Some(1.0), "{beats}");
        }
    }

    #[test]
    fn a_start_that_is_not_a_number_becomes_one() {
        for start in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let t = fade(0.0, 1.0, start, 4.0, Curve::Lin);
            assert!(t.start().is_finite(), "{start}");
            assert!(t.finished(t.start() + 4.0), "{start}: never finishes");
        }
    }

    #[test]
    fn the_curve_shapes_the_middle() {
        let lin = fade(0.0, 1.0, 0.0, 4.0, Curve::Lin);
        let smooth = fade(0.0, 1.0, 0.0, 4.0, Curve::Smooth);
        let at = |t: &Transition, b: f64| t.value_at(b).expect("started");
        assert!((at(&lin, 1.0) - 0.25).abs() < 1e-6);
        assert!((at(&lin, 2.0) - 0.5).abs() < 1e-6);
        assert!((at(&smooth, 2.0) - 0.5).abs() < 1e-6);
        assert!(at(&smooth, 1.0) < at(&lin, 1.0));
        assert!(at(&smooth, 3.0) > at(&lin, 3.0));
    }

    #[test]
    fn a_fade_out_reaches_silence_exactly() {
        let t = fade(1.0, 0.0, 0.0, 8.0, Curve::Smooth);
        assert_eq!(t.value_at(8.0), Some(0.0));
        let middle = t.value_at(4.0).expect("started");
        assert!(middle > 0.0 && middle < 1.0);
    }

    #[test]
    fn quantising_lands_on_the_next_boundary_and_on_this_one_when_it_is_exact() {
        assert_eq!(quantise(8.0, 4.0), 8.0);
        assert_eq!(quantise(8.25, 4.0), 12.0);
        assert_eq!(quantise(11.999, 4.0), 12.0);
        assert_eq!(quantise(0.5, 1.0), 1.0);
        assert_eq!(quantise(8.25, 0.0), 8.25);
        assert_eq!(quantise(-1.5, 1.0), -1.0);
    }

    #[test]
    fn a_selection_lands_on_the_grid_rather_than_when_it_was_asked_for() {
        let now = 9.25;
        let s = Selection::new(1, 2, quantise(now, 4.0));
        assert_eq!(s.start(), 12.0);
        assert!(!s.due(now));
        assert!(!s.due(10.0));
        assert!(!s.due(11.999));
        assert!(s.due(12.0));
        assert!(s.due(99.0));
        assert!(Selection::new(1, 2, quantise(now, 0.0)).due(now));
    }

    #[test]
    fn a_selection_start_that_is_not_a_number_becomes_one() {
        for start in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let s = Selection::new(0, 0, start);
            assert!(s.start().is_finite(), "{start}");
            assert!(s.due(s.start()), "{start}: never lands");
        }
    }

    #[test]
    fn every_control_has_a_wire_name_that_parses_back() {
        for control in Control::ALL {
            assert_eq!(Control::from_name(control.name()), Some(control));
        }
        assert_eq!(Control::from_name("blend"), None);
    }
}
