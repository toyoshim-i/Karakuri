use super::*;

/// Maps a keyboard event to a UI grammar press (space, enter, arrows, or digit; ADR-0333).
pub(crate) fn grammar(key: &Key<&str>, alt: bool, ctrl: bool) -> Option<focus::Press> {
    match key {
        Key::Named(NamedKey::Space) => Some(focus::Press::Space),
        Key::Named(NamedKey::Enter) => {
            if alt {
                Some(focus::Press::AltEnter)
            } else if ctrl {
                Some(focus::Press::CtrlEnter)
            } else {
                Some(focus::Press::Enter)
            }
        }
        Key::Named(NamedKey::ArrowUp) => Some(focus::Press::Arrow(focus::Arrow::Up)),
        Key::Named(NamedKey::ArrowDown) => Some(focus::Press::Arrow(focus::Arrow::Down)),
        Key::Named(NamedKey::ArrowLeft) => Some(focus::Press::Arrow(focus::Arrow::Left)),
        Key::Named(NamedKey::ArrowRight) => Some(focus::Press::Arrow(focus::Arrow::Right)),
        Key::Character(text) => digit(text).map(focus::Press::Digit),
        _ => None,
    }
}

/// Parses a single ASCII decimal digit from a character key event, returning its integer value.
pub(crate) fn digit(text: &str) -> Option<usize> {
    let mut chars = text.chars();
    let one = chars.next()?;
    if chars.next().is_some() {
        return None;
    }
    one.to_digit(10).map(|digit| digit as usize)
}

/// Steps the exposure value up or down along the track grid or resets to default (ADR-0259).
pub(crate) fn exposure_key(step: Step, from: f32) -> f32 {
    let at = view::unit_of(from);
    match step {
        Step::Down => view::exposure_at(at - 1.0 / view::EXPOSURE_TRACK_W),
        Step::Up => view::exposure_at(at + 1.0 / view::EXPOSURE_TRACK_W),
        Step::Default => 1.0,
    }
}

/// Adjustment step size for tempo adjustments in beats per minute (ADR-0350).
pub(crate) const TEMPO_STEP_BPM: f32 = 1.0;

/// Calculates the new free-run tempo after applying a key step adjustment, floored at 1.0 BPM.
pub(crate) fn tempo_key(step: Step, from: f32) -> f32 {
    let asked = match step {
        Step::Down => from - TEMPO_STEP_BPM,
        Step::Up => from + TEMPO_STEP_BPM,
        Step::Default => from,
    };
    asked.max(1.0)
}

/// Calculates the new audio latency offset in milliseconds after applying a key step adjustment.
pub(crate) fn offset_key(step: Step, from: f32) -> f32 {
    match step {
        Step::Down => from - audio::LATENCY_OFFSET_STEP_MS,
        Step::Up => from + audio::LATENCY_OFFSET_STEP_MS,
        Step::Default => 0.0,
    }
}

/// Converts the engine's `Look` into the console view representation.
pub(crate) fn look(look: &Look) -> view::Look {
    view::Look {
        tonemap: mix::tonemap(look.op),
        exposure: look.exposure,
    }
}
