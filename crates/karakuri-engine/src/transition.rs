//! Transitions: a mix control moving over musical time.
//!
//! `docs/roadmap.md` asks for M2's "transitions as first-class objects, not
//! just crossfade", and the object it turned out to want is smaller than a
//! crossfade rather than larger: **one control, one destination, one musical
//! duration, one curve.** A crossfade is two of them issued together, a fade-in
//! is one, and a cut on the bar is one with a duration of zero — so the thing
//! that is first-class is the *scheduled move*, and every gesture anyone names
//! is made of those. A `Crossfade` type would have been one gesture with the
//! other three left out.
//!
//! ## On the beat clock, and what that costs
//!
//! The roadmap's three clocks say the middle one — beat and bar, half a second
//! to four — is for "variant switching, parameter morphs, transitions". The
//! transport got there first: `Sync::Beat` already derives a slot's `t` from
//! the grid, which is engine machinery running on that clock rather than a
//! procedure reading a signal. What is new here is *scheduling*: the transport
//! follows the grid continuously, and this is the first thing that arranges to
//! happen at a named instant on it.
//!
//! **A transition is a function of `beats` and nothing else.** Not of wall
//! time, not of frames: the oscillator's musical position is driven by the tick
//! stream, so the same records produce the same fade, frame for frame, on a
//! machine running at a different rate. That is also why a tempo correction
//! mid-fade is *correct* rather than a glitch — eight beats is eight beats, and
//! a fade that ignored a tempo change would be the only thing on the deck still
//! running at the old one.
//!
//! ## What is recorded, and what is not
//!
//! The schedule is one record. **The fade it produces is not recorded at all**,
//! and that is the point rather than an omission: a value per frame would be
//! 216,000 records an hour, and it would be recording something the grid
//! already determines. This is the same shape `tick` has — the engine advances
//! by a step count and everything downstream is a function of it — and it is
//! what the three-clock model means by the runtime *selecting* rather than
//! computing.
//!
//! `from` is deliberately absent from the record, and is read where the
//! transition is **scheduled** rather than where it starts. Both are
//! defensible and the first is not implementable: the start is a musical
//! instant, so "the value at the start" would be captured on the first *frame*
//! at or after it, and a machine running at a different rate would capture it
//! at a different beat. That is exactly the property this module exists to
//! have. Scheduling-time capture is a plain function of the records that came
//! before, and the case it would get wrong — something moving the control
//! between the schedule and the start — cannot arise, because the only things
//! that write a control are a hand (which cancels) and another transition
//! (which replaces).
//!
//! **A scheduled transition does not touch its control until it starts.** It
//! would be one line shorter to have it hold the control at `from` in the
//! meantime, and that line would freeze a fader for up to a bar between the
//! press and the music.
//!
//! ## The operator wins
//!
//! Moving a control by hand cancels whatever transition was moving it. A fader
//! that fought back would be the worst control on the deck: the one place an
//! operator reaches when something is wrong is the same place an automatic
//! thing is writing. This is the same rule M6 states for agents — "manual
//! intervention instantly demoting that layer's agent to `Suggest`" — arriving
//! early because the first automatic writer arrived early.

use crate::binding::Curve;

/// Which mix control a transition moves.
///
/// The three that are a number an operator moves. Blend mode, residency and a
/// mask's *shape* are not here and should not be: they are choices rather than
/// positions, and "half way to `over`" does not name a picture. A cut between
/// them is a transition of duration zero on a control that is here, which is
/// how it is done.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Control {
    /// The level the material arrives at.
    Gain,
    /// The fader across the blend.
    Opacity,
    /// **How far a mask's front has travelled**, `[0, 1]`.
    ///
    /// This is what makes a wipe a transition rather than a mode: an incoming
    /// layer under `over`, with a linear mask, and one scheduled move carrying
    /// this from 0 to 1. Neither half had to know about the other — the
    /// transition moves a number and the mask reads one.
    ///
    /// Moving it on a slot with no mask does nothing, which is the honest
    /// answer rather than a special case: the shape is what decides whether a
    /// position means anything, and `MaskKind::None` reveals everything at
    /// every position.
    MaskPosition,
}

impl Control {
    /// Every control there is, in the order they are documented.
    pub const ALL: [Control; 3] = [Control::Gain, Control::Opacity, Control::MaskPosition];

    /// The wire and status-line spelling. A match rather than a table, so a
    /// control added to the enum does not compile until it has a name.
    pub fn name(self) -> &'static str {
        match self {
            Control::Gain => "gain",
            Control::Opacity => "opacity",
            Control::MaskPosition => "mask",
        }
    }

    /// The spelling back, or `None`. Derived from [`Control::name`] over
    /// [`Control::ALL`], so the two directions cannot disagree.
    pub fn from_name(name: &str) -> Option<Control> {
        Control::ALL.iter().copied().find(|c| c.name() == name)
    }
}

/// One scheduled move.
///
/// Construct with [`Transition::new`], which is where `from` is captured.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transition {
    slot: usize,
    control: Control,
    from: f32,
    to: f32,
    /// The musical instant it begins, on the session oscillator's beat count.
    start: f64,
    /// How long it lasts, in beats. Zero is a cut.
    beats: f64,
    curve: Curve,
}

impl Transition {
    /// A move from `from` to `to`, beginning at `start` beats and lasting
    /// `beats` of them.
    ///
    /// `start` is absolute rather than relative, and the reason is replay: "in
    /// two bars" is a different instant depending on when it is read, where a
    /// beat count is the same instant on every run. Quantising *to* the next
    /// bar is [`quantise`]'s job, and it happens once, where the operator asked.
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
            // A start nothing can reach is a control pinned forever, since
            // `finished` is never true past it. The record path refuses these
            // before they get here — see `karakuri-cli`'s `mix` module — so
            // this is the backstop for an engine caller rather than the gate.
            start: if start.is_finite() { start } else { 0.0 },
            // Zero for a negative duration, which is a transition that ends
            // before it begins, and for an infinite one, which is a fade that
            // never arrives. Both are arithmetic rather than intent, and a cut
            // is the reading that is not a surprise.
            beats: if beats.is_finite() { beats.max(0.0) } else { 0.0 },
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

    /// Where the control should be at this musical position, or **`None`
    /// before the transition starts** — which means "leave it alone" rather
    /// than "hold it at `from`".
    ///
    /// The difference is a bar of frozen fader. A fade scheduled for the next
    /// bar must not take the control away from the operator in the meantime,
    /// and a crossfade's incoming half — scheduled while the slot is still at
    /// silence — must not be pinned at the value it had when the key was
    /// pressed.
    ///
    /// Comparing against `beats` rather than against the previous frame is what
    /// makes this survive a tempo correction, which moves the grid under a
    /// transition that has not started: there is nothing to arm and so nothing
    /// to arm at the wrong instant or twice.
    pub fn value_at(&self, beats: f64) -> Option<f32> {
        // Strictly before, because the instant itself belongs to the
        // transition: a cut scheduled on beat 10 has to have cut *at* beat 10,
        // and `<=` here would leave it alone on the beat it was asked for.
        if beats < self.start {
            return None;
        }
        if self.beats <= 0.0 || beats >= self.start + self.beats {
            return Some(self.to);
        }
        let t = (beats - self.start) / self.beats;
        let shaped = self.curve.apply(t as f32);
        // Both ends are exact, and neither is exact *because of this line*: the
        // two guards above answer them, and the lerp only ever runs strictly
        // inside. Written this way round anyway because it is the form whose
        // endpoints are exact if the guards are ever loosened.
        Some(self.from + (self.to - self.from) * shaped)
    }

    /// Whether this position is past the end. A finished transition is dropped
    /// rather than kept writing the value it already wrote.
    pub fn finished(&self, beats: f64) -> bool {
        beats >= self.start + self.beats
    }
}

/// The next musical instant on a grid of `quantum` beats, at or after `beats`.
///
/// `quantum` of 0 is "now" and returns `beats` — an operator who wants a cut
/// does not want to wait for the bar. 1 is the next beat, 4 the next bar in
/// four. A transition scheduled on a quantum lands where the music is rather
/// than where the hand was, which is the whole reason a transition is scheduled
/// at all rather than started.
///
/// **At or after, and the boundary is `>=` rather than `>`**: scheduling
/// exactly on the beat means *this* beat, not the next one. The alternative
/// puts a full bar between a perfectly timed press and anything happening.
pub fn quantise(beats: f64, quantum: f64) -> f64 {
    if !(quantum.is_finite() && quantum > 0.0) || !beats.is_finite() {
        return beats;
    }
    let next = (beats / quantum).ceil() * quantum;
    // `ceil` of an exact multiple is itself, which is the `>=` above; the guard
    // is against a float that landed a hair below one.
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

    /// **Both ends are exact.** A fade that stops a hair short of silence
    /// leaves a layer contributing something the operator asked to be gone, and
    /// one that stops short of unity leaves a slot quietly below every other.
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

    /// **Before it starts it leaves the control alone**, rather than holding
    /// it at `from`.
    ///
    /// The difference is a bar of frozen fader: with a quantum of one bar, a
    /// scheduled fade is armed for up to four beats before it is due, and
    /// pinning the control through that would take it away from the operator
    /// between the press and the music. It is also what makes a crossfade's
    /// incoming half work at all — that one is scheduled while the slot is
    /// still at silence and must not be pinned at the value it had when the
    /// key went down.
    #[test]
    fn a_transition_that_has_not_started_leaves_its_control_alone() {
        let t = fade(0.25, 0.75, 10.0, 4.0, Curve::Lin);
        assert_eq!(t.value_at(0.0), None);
        assert_eq!(t.value_at(9.999), None);
        assert!(!t.finished(9.999));
        // Including when the grid jumps backwards past it, which a phase
        // correction does.
        assert_eq!(t.value_at(-5.0), None);
    }

    /// A duration of zero is a cut: nothing before the instant, everything
    /// after. The obvious arithmetic divides by it, so this is the case that
    /// has to be answered rather than computed.
    #[test]
    fn a_duration_of_zero_is_a_cut_rather_than_a_division() {
        let t = fade(0.0, 1.0, 10.0, 0.0, Curve::Smooth);
        assert_eq!(t.value_at(9.9), None);
        assert_eq!(t.value_at(10.0), Some(1.0));
        assert!(t.finished(10.0));
        assert_eq!(t.value_at(10.1), Some(1.0));
    }

    /// **A negative duration is a cut too, not a transition running
    /// backwards.** It arrives from arithmetic — a duration computed from two
    /// beats in the wrong order — and the reading that is not a surprise is the
    /// one a zero would give.
    #[test]
    fn a_negative_or_absent_duration_is_a_cut() {
        for beats in [-4.0, f64::NAN, f64::NEG_INFINITY, f64::INFINITY] {
            let t = fade(0.0, 1.0, 10.0, beats, Curve::Lin);
            assert_eq!(t.beats(), 0.0, "{beats}");
            assert_eq!(t.value_at(10.0), Some(1.0), "{beats}");
        }
    }

    /// **A start nothing can reach is a control pinned forever**, since
    /// `finished` is never true past it and nothing but a hand would clear it.
    /// The record path refuses these; this is the backstop for an engine
    /// caller, and it is here because the failure it prevents is silent.
    #[test]
    fn a_start_that_is_not_a_number_becomes_one() {
        for start in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let t = fade(0.0, 1.0, start, 4.0, Curve::Lin);
            assert!(t.start().is_finite(), "{start}");
            assert!(t.finished(t.start() + 4.0), "{start}: never finishes");
        }
    }

    /// The curve shapes the middle and only the middle, and `smooth` really
    /// does ease: halfway through, a linear fade is halfway and a smooth one is
    /// too, but a quarter of the way in they differ.
    #[test]
    fn the_curve_shapes_the_middle() {
        let lin = fade(0.0, 1.0, 0.0, 4.0, Curve::Lin);
        let smooth = fade(0.0, 1.0, 0.0, 4.0, Curve::Smooth);
        let at = |t: &Transition, b: f64| t.value_at(b).expect("started");
        assert!((at(&lin, 1.0) - 0.25).abs() < 1e-6);
        assert!((at(&lin, 2.0) - 0.5).abs() < 1e-6);
        assert!((at(&smooth, 2.0) - 0.5).abs() < 1e-6);
        // Eased in: a quarter of the way through, less than a quarter of the
        // way there.
        assert!(at(&smooth, 1.0) < at(&lin, 1.0));
        assert!(at(&smooth, 3.0) > at(&lin, 3.0));
    }

    /// A fade downwards is the same object, and both ends are still exact —
    /// which matters more in this direction, since the bottom is silence.
    #[test]
    fn a_fade_out_reaches_silence_exactly() {
        let t = fade(1.0, 0.0, 0.0, 8.0, Curve::Smooth);
        assert_eq!(t.value_at(8.0), Some(0.0));
        let middle = t.value_at(4.0).expect("started");
        assert!(middle > 0.0 && middle < 1.0);
    }

    /// **Scheduling exactly on the beat means this beat.** The alternative puts
    /// a whole bar between a perfectly timed press and anything happening,
    /// which is the one thing a quantised control must not do.
    #[test]
    fn quantising_lands_on_the_next_boundary_and_on_this_one_when_it_is_exact() {
        assert_eq!(quantise(8.0, 4.0), 8.0);
        assert_eq!(quantise(8.25, 4.0), 12.0);
        assert_eq!(quantise(11.999, 4.0), 12.0);
        assert_eq!(quantise(0.5, 1.0), 1.0);
        // A quantum of zero is "now", which is what a cut wants.
        assert_eq!(quantise(8.25, 0.0), 8.25);
        // And a grid that has gone backwards past zero still lands on a
        // boundary rather than off the grid.
        assert_eq!(quantise(-1.5, 1.0), -1.0);
    }

    /// Every control round-trips its own name, so one added to the enum and not
    /// to the wire vocabulary is a record that fails to decode rather than one
    /// that decodes as the wrong control.
    #[test]
    fn every_control_has_a_wire_name_that_parses_back() {
        for control in Control::ALL {
            assert_eq!(Control::from_name(control.name()), Some(control));
        }
        assert_eq!(Control::from_name("blend"), None);
    }
}
