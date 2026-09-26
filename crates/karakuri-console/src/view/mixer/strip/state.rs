//! Strip state, animation phase, fader and meter readings for the Mixer bay.

use std::time::Duration;

use egui::Rect;

use super::super::*;

/// `.trim .lbl`: the `g`, which is the only word in this bay that is neither
/// the deck's nor the engine's — it is the mock's.
pub(crate) const TRIM_LABEL: &str = "g";

/// Global animation phase duration shared by all animated controls (ADR-0156, ADR-0190, P-0091, P-0092).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Phase(Duration);

impl Phase {
    /// The origin, and what a console with no clock behind it is at — every test in
    /// this crate, and the first frame of a window.
    pub const ZERO: Phase = Phase(Duration::ZERO);

    /// A phase from the interval that has elapsed since whoever owns the clock
    /// started counting. Where the origin is does not matter and is not asked:
    /// every presentation is periodic in it.
    pub const fn since(elapsed: Duration) -> Phase {
        Phase(elapsed)
    }

    /// Returns the fractional position `[0, 1)` of this phase within `period`.
    pub fn cycle(self, period: Duration) -> f32 {
        let period = period.as_secs_f64();
        match period > 0.0 {
            true => (self.0.as_secs_f64() / period).fract() as f32,
            false => 0.0,
        }
    }
}

/// Animation loop duration for pending control transitions (1000 ms) (ADR-0190, ADR-0206).
pub const ROLL_PERIOD: Duration = Duration::from_millis(1000);

/// Duration of movement within each roll period (400 ms travel, 600 ms rest) (P-0087).
pub const ROLL_TRAVEL: Duration = Duration::from_millis(400);

/// Peak travel displacement fraction toward target for pending roll animations (0.4).
pub const ROLL_REACH: f32 = 0.4;

/// How many steps the travel is drawn in, which is the only reason
/// [`ROLL_STALENESS`] is a number at all.
pub(crate) const ROLL_STEPS: u32 = 12;

/// Refresh interval deadline for roll animations (P-0091).
pub const ROLL_STALENESS: Duration =
    Duration::from_micros(ROLL_TRAVEL.as_millis() as u64 * 1000 / ROLL_STEPS as u64);

/// Pending presentation displacement at `phase`: `0.0` at rest, [`ROLL_REACH`] at peak.
/// Raised cosine over [`ROLL_TRAVEL`], zero for remainder of [`ROLL_PERIOD`].
pub fn roll_at(phase: Phase) -> f32 {
    let travel = ROLL_TRAVEL.as_secs_f32() / ROLL_PERIOD.as_secs_f32();
    let t = phase.cycle(ROLL_PERIOD);
    match t < travel {
        true => ROLL_REACH * 0.5 * (1.0 - (t / travel * std::f32::consts::TAU).cos()),
        false => 0.0,
    }
}

/// Duration until the next presentation movement from `phase` (ADR-0283).
///
/// Returns [`ROLL_STALENESS`] during travel, or the remaining duration until the
/// next travel window begins, clamped to at least [`ROLL_STALENESS`].
pub fn roll_moves_in(phase: Phase) -> Duration {
    let travel = ROLL_TRAVEL.as_secs_f32() / ROLL_PERIOD.as_secs_f32();
    let t = phase.cycle(ROLL_PERIOD);
    match t < travel {
        true => ROLL_STALENESS,
        // `cycle` is `[0, 1)`, so this is positive and no longer than the
        // whole period.
        false => ROLL_PERIOD.mul_f32(1.0 - t).max(ROLL_STALENESS),
    }
}

/// Layer mask shape selection for a channel strip ([`Mask::None`], [`Mask::Linear`], [`Mask::Radial`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mask {
    /// No mask: the layer reaches the whole frame. The mock's `◯`.
    None,
    /// A straight edge across the frame — the mock's `◑`, which is what a hard edge
    /// down the middle of a circle looks like.
    Linear,
    /// A circle. The same outline with a filled centre, which is the same family as
    /// the mock's two and is the shape a radial mask makes.
    Radial,
}

/// Audio/video meter level readings for a channel strip (mean luminance and peak) (ADR-0156).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Level {
    /// Mean luminance over the whole frame — *"the figure to match faders on"*, and
    /// what the meter's column is as tall as.
    pub mean: f32,
    /// The brightest single texel. What the peak mark sits on.
    pub peak: f32,
}

/// Channel strip display state (faders, meters, residency, and masks) (ADR-0156, ADR-0187, ADR-0195, ADR-0203, ADR-0206, ADR-0290, P-0090).
#[derive(Debug, Clone, PartialEq)]
pub struct Strip {
    /// What the deck is playing, in `.strip-name`.
    pub name: String,
    /// Where the slot sits — `Deck::residency`, which is the effective residency
    /// and not `requested_residency`.
    pub tally: Tally,
    /// What was asked for — `Deck::requested_residency`, the other half of the pair
    /// [`Strip::tally`] is one of, and what a press on the chip counts from
    /// ([`Mixer::tally`]).
    pub requested: Tally,
    /// The trim — `Deck::gain`: linear, floored at zero, and deliberately open
    /// above 1.0 because the mix is HDR.
    pub gain: f32,
    /// Where a scheduled move is taking the trim, or `None` for a trim nothing is
    /// moving — `Deck::transitions_on(slot)`'s `Control::Gain` entry, and its
    /// `Transition::to`.
    pub gain_to: Option<f32>,
    /// The fader — `Deck::opacity`: a proportion in `[0, 1]`, and the one control
    /// that silences a slot under every blend mode, which is what makes it the way
    /// out of material that has gone NaN.
    pub opacity: f32,
    /// Where a scheduled move is taking the fader, or `None` for a fader nothing is
    /// moving — `Deck::transitions_on(slot)`'s `Control::Opacity` entry, on
    /// [`Strip::gain_to`]'s terms and for its reasons.
    pub opacity_to: Option<f32>,
    /// The blend in force, as one of the vocabulary's three — and the word drawn on
    /// the chip is [`BlendMode::name`].
    pub blend: BlendMode,
    /// The mask in force (`Deck::mask(slot).kind()`).
    pub mask: Mask,
    /// The angle the mask is already wearing — `Deck::mask(slot).angle()`, in
    /// radians. Read to build an operation, and drawn nowhere.
    pub mask_angle: f32,
    /// What the slot's meter last read, or `None` for no reading at all.
    pub level: Option<Level>,
    /// Whether this channel is currently muted.
    pub is_muted: bool,
    /// Whether this channel is currently soloed.
    pub is_soloed: bool,
}

impl Strip {
    /// Where this slot has been asked to go and has not reached, or `None` if settled.
    ///
    /// Returns `Some(requested)` when `requested != tally` (P-0087, ADR-0188).
    pub fn pending(&self) -> Option<Tally> {
        match self.requested == self.tally {
            true => None,
            false => Some(self.requested),
        }
    }

    /// Where the trim is going and has not got to, or `None` for a trim that is
    /// where it has been asked to be.
    pub fn gain_pending(&self) -> Option<f32> {
        self.gain_to
            .filter(|to| unit(*to) != unit(self.gain))
            .map(unit)
    }

    /// Where the fader is going and has not got to, on [`Strip::gain_pending`]'s
    /// terms.
    pub fn opacity_pending(&self) -> Option<f32> {
        self.opacity_to
            .filter(|to| unit(*to) != unit(self.opacity))
            .map(unit)
    }
}

impl Default for Strip {
    fn default() -> Self {
        Strip {
            name: String::new(),
            tally: Tally::Allocated,
            requested: Tally::Allocated,
            gain: 0.0,
            gain_to: None,
            opacity: 0.0,
            opacity_to: None,
            blend: BlendMode::Add,
            mask: Mask::None,
            mask_angle: 0.0,
            level: None,
            is_muted: false,
            is_soloed: false,
        }
    }
}

/// A fader, laid out: the track, the length of it the value fills, and the knob
/// sitting on the value.
///
/// One type and one derivation for both of a strip's faders — see [`fader`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fader {
    /// The well: `.fader`'s 5px capsule lying down, or `.vfader`'s 17px one
    /// standing up.
    pub track: Rect,
    /// Which way it runs. [`Axis::Row`] fills from the left and [`Axis::Column`]
    /// fills from the bottom, because a fader stands up.
    pub axis: Axis,
    /// What the value fills, from the track's own zero.
    pub fill: Rect,
    /// The knob, centred on the fill's moving edge.
    pub knob: Rect,
    /// How far the knob's centre moves between the two ends: the track less the
    /// inset the fill sits inside it by, along the axis.
    pub travel: f32,
}

/// A meter, laid out: the well, the column the mean fills, and the peak's mark.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Meter {
    /// `.vmeter`'s 6px capsule.
    pub well: Rect,
    /// [`Level::mean`], up from the bottom.
    pub fill: Rect,
    /// [`Level::peak`], a [`size::VMETER_PEAK_H`] bar across the well.
    pub peak: Rect,
}

/// A scheduled move on a fader, laid out: the mark on the destination, and how
/// far this frame's attempt to get there has reached.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Reach {
    /// The destination, as a hairline across the track at the position the knob
    /// would sit at if the move had landed.
    pub mark: Rect,
    /// What the reach has covered at this displacement: from the value's own edge
    /// toward [`Reach::mark`], and never as far as it — [`ROLL_REACH`] of the way
    /// at the top of the travel, nothing at rest.
    pub band: Rect,
}
