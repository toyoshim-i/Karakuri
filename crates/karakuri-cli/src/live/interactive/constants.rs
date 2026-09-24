use super::super::*;

/// One press of a gain key. Linear and additive, because a fader is: the
/// operator wants the same physical move to mean the same amount everywhere,
/// not a proportion of wherever the slot happens to be.
pub(crate) const GAIN_STEP: f32 = 0.1;

/// Quantization intervals for scheduled fades, represented as beat counts and labels.
pub(crate) const QUANTA: [(f64, &str); 3] =
    [(4.0, "the next bar"), (1.0, "the next beat"), (0.0, "now")];

/// How long a scheduled fade lasts, in beats. A bar, half a bar, two bars, and
/// a cut — the four an operator reaches for, in the order they are reached for.
pub(crate) const FADE_BEATS: [f64; 4] = [4.0, 2.0, 8.0, 0.0];

/// Wipe transition mask shapes, rotation angles, and user-facing labels in cycle order.
pub(crate) const MASK_SHAPES: [(MaskKind, f32, &str); 6] = [
    (MaskKind::None, 0.0, "off — `c` needs a shape"),
    (MaskKind::Linear, 0.0, "linear, left to right"),
    (
        MaskKind::Linear,
        std::f32::consts::FRAC_PI_2,
        "linear, bottom to top",
    ),
    (
        MaskKind::Linear,
        std::f32::consts::FRAC_PI_4,
        "linear, diagonal",
    ),
    (
        MaskKind::Linear,
        -std::f32::consts::FRAC_PI_4,
        "linear, the other diagonal",
    ),
    (MaskKind::Radial, 0.0, "an iris"),
];

/// One press of an opacity key. Additive for the same reason as [`GAIN_STEP`],
/// and clamped to `[0, 1]` where gain is not: opacity is a proportion of a
/// blend and there is no such thing as 1.4 of one, while gain is a level into
/// an HDR mix and values above 1.0 are ordinary.
pub(crate) const OPACITY_STEP: f32 = 0.1;

/// One press of an exposure key, as a factor. Multiplicative, because exposure
/// is: a stop is a ratio, and an additive step would be enormous at 0.1 and
/// invisible at 8.0. This is a quarter of a stop, near enough.
pub(crate) const EXPOSURE_STEP: f32 = 1.189_207;

/// Interactive bounds for keyboard exposure adjustments.
pub(crate) const EXPOSURE_MIN: f32 = 1.0 / 64.0;
pub(crate) const EXPOSURE_MAX: f32 = 64.0;

/// Clamps exposure to safe positive interactive bounds.
pub(crate) fn clamp_exposure(exposure: f32) -> f32 {
    exposure.clamp(EXPOSURE_MIN, EXPOSURE_MAX)
}

/// One press of the scrub keys, in beats. A quarter beat — a sixteenth of a bar
/// in four — which is small enough to place a hit by ear and large enough to
/// hear one press.
pub(crate) const SCRUB_BEATS: f64 = 0.25;

/// Clamps gain to a non-negative lower bound.
pub(crate) fn clamp_gain(gain: f32) -> f32 {
    gain.max(0.0)
}

/// The tone map cycle `t` steps through. Pure so the cycle — and that it
/// returns to where it started — is checkable without a window.
pub(crate) fn next_tonemap(op: TonemapOp) -> TonemapOp {
    match op {
        TonemapOp::Clamp => TonemapOp::Reinhard,
        TonemapOp::Reinhard => TonemapOp::Aces,
        TonemapOp::Aces => TonemapOp::AgX,
        TonemapOp::AgX => TonemapOp::Clamp,
    }
}

/// Translates an operation into records or returns a refusal message explaining why it was not performed.
pub(crate) fn answered(operation: &Operation, current: &Current) -> Result<Vec<Record>, String> {
    let title = operation.title();
    match karakuri_operation_record::written(operation, current) {
        Written::Records(records) => Ok(records),
        Written::Silent(silent) => Err(format!(
            "{}, and this program performs an operation by writing the records it converts \
             to. `karakuri-cli` has no control for this one — the instrument, `cargo run -p \
             karakuri`, is the surface that answers it.",
            not_performed(title, silent.why())
        )),
        // Owed operation with no current record representation.
        Written::Owed(owed) => Err(not_performed(title, owed.why())),
        // Operation refused by policy or current context.
        Written::Refused(refusal) => Err(not_performed(title, &refusal.why())),
    }
}

/// Formats a confirmation message for an operation performed on the current frame.
pub(crate) fn performed_at_the_frame(title: &str) -> String {
    format!(
        "`{title}` was performed on the frame it arrived on, where the same operation from a \
         key or a mapped control is performed. Anything it started rather than finished is \
         reported where it lands: ask `swap_outcome` for a rebuild, and a scheduled move \
         arrives on the grid."
    )
}

/// Whether `at` names a renderer of a slot that draws with `count` of them, and
/// the sentence if it does not. [`slot_in_range`]'s companion, returning the
/// refusal rather than a bool because both callers print it.
pub(crate) fn renderer_in_range(slot: usize, at: usize, count: usize) -> Result<(), String> {
    if at < count {
        Ok(())
    } else {
        Err(no_such_renderer(slot, at, count))
    }
}

/// Status label for a watchdog-stopped overloaded slot (ADR-0316).
/// Returns `"overloaded "` if true, or empty string otherwise.
pub(crate) fn stopped_tag(overloaded: bool) -> &'static str {
    match overloaded {
        true => "overloaded ",
        false => "",
    }
}

/// Four-character fixed-width status line tag for a slot's residency and park state.
pub(crate) fn residency_tag(residency: Residency, parked: bool) -> &'static str {
    match residency {
        Residency::Live => "LIVE",
        Residency::Priming => "prim",
        Residency::Allocated if parked => "park",
        Residency::Allocated => "off ",
    }
}

/// Human-readable description of a slot's residency and parked state.
pub(crate) fn residency_name(residency: Residency, parked: bool) -> &'static str {
    match residency {
        Residency::Live => "live",
        Residency::Priming => "priming (warming, off air)",
        Residency::Allocated if parked => "parked (asked to prime, waiting for room)",
        Residency::Allocated => "allocated (off air)",
    }
}

/// Returns true if `slot` is a valid index within `slot_count`.
pub(crate) fn slot_in_range(slot: usize, slot_count: usize) -> bool {
    u8::try_from(slot)
        .ok()
        .is_some_and(|slot| DeckSlot::new(slot, slot_count).is_some())
}
