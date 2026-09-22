//! Rendering, painting, and cycle transitions for channel strips and controls.

use egui::text::LayoutJob;
use egui::{Color32, CornerRadius, Pos2, Rect, Stroke, StrokeKind, Ui};
use karakuri_operation::{BlendMode, Residency, WipeKind};

use super::super::super::*;
use super::super::*;
use super::layout::*;
use super::state::*;

/// A scheduled move on a laid-out fader: destination mark and interpolation band (ADR-0206).
pub(crate) fn reach(now: Fader, to: Fader, rolled: f32) -> Reach {
    let (from, dest) = (now.knob.center(), to.knob.center());
    match now.axis {
        Axis::Row => {
            let head = from.x + (dest.x - from.x) * rolled;
            Reach {
                mark: Rect::from_center_size(
                    Pos2::new(dest.x, now.track.center().y),
                    egui::vec2(size::HAIRLINE, now.knob.height()),
                ),
                band: Rect::from_min_max(
                    Pos2::new(from.x.min(head), now.fill.min.y),
                    Pos2::new(from.x.max(head), now.fill.max.y),
                ),
            }
        }
        Axis::Column => {
            let head = from.y + (dest.y - from.y) * rolled;
            Reach {
                mark: Rect::from_center_size(
                    Pos2::new(now.track.center().x, dest.y),
                    egui::vec2(now.knob.width(), size::HAIRLINE),
                ),
                band: Rect::from_min_max(
                    Pos2::new(now.fill.min.x, from.y.min(head)),
                    Pos2::new(now.fill.max.x, from.y.max(head)),
                ),
            }
        }
    }
}

/// The next blend mode round the cycle, wrapping from the last back to the
/// first — and the whole of the affordance the blend chip is.
pub fn after(blend: BlendMode) -> BlendMode {
    match blend {
        BlendMode::Add => BlendMode::Over,
        BlendMode::Over => BlendMode::Max,
        BlendMode::Max => BlendMode::Add,
    }
}

/// The next residency round the cycle, wrapping from the last back to the first
/// — the whole of the affordance the tally chip is, and the order the mock's
/// own tooltip lists: *"one of three residencies — live, priming, allocated"*.
pub(crate) fn next(tally: Tally) -> Tally {
    match tally {
        Tally::Live => Tally::Priming,
        Tally::Priming => Tally::Allocated,
        Tally::Allocated => Tally::Live,
    }
}

/// The console's word for a residency, as the vocabulary's — and it is the
/// whole of what the console has to know about the difference.
pub(crate) fn residency(tally: Tally) -> Residency {
    match tally {
        Tally::Live => Residency::Live,
        Tally::Priming => Residency::Priming,
        Tally::Allocated => Residency::Allocated,
    }
}

/// The next mask shape round the cycle, wrapping from the last back to the
/// first — the whole of the affordance the mask mini is.
pub(crate) fn next_shape(mask: Mask) -> Mask {
    match mask {
        Mask::None => Mask::Linear,
        Mask::Linear => Mask::Radial,
        Mask::Radial => Mask::None,
    }
}

/// The console's word for a mask shape, as the vocabulary's — [`residency`] one
/// control along, and for its reason exactly.
pub(crate) fn wipe_kind(mask: Mask) -> WipeKind {
    match mask {
        Mask::None => WipeKind::None,
        Mask::Linear => WipeKind::Linear,
        Mask::Radial => WipeKind::Radial,
    }
}

/// `.strip-name` as one laid-out run: `overflow: hidden; text-overflow:
/// ellipsis; white-space: nowrap` is exactly one row, broken anywhere, with an
/// ellipsis standing for what did not fit.
fn name_job(name: &str, width: f32, colour: Color32) -> LayoutJob {
    let mut job = span_at(name, size::STRIP_NAME_SIZE, colour);
    job.wrap = egui::epaint::text::TextWrapping {
        max_width: width,
        max_rows: 1,
        break_anywhere: true,
        overflow_character: Some('…'),
    };
    job
}

/// One strip, term for term from `style.css`:
pub(crate) fn strip_into(ui: &Ui, pal: &Palette, strip: &Strip, at: StripBox, phase: Phase) {
    let painter = ui
        .painter()
        .with_clip_rect(at.rect.expand(size::STRIP_GAP * 0.5));
    painter.rect_filled(
        at.rect,
        CornerRadius::same(size::STRIP_RADIUS as u8),
        pal.well,
    );

    if !strip.name.is_empty() {
        let galley = painter.layout_job(name_job(&strip.name, at.name.width(), pal.dim));
        centre_galley(&painter, at.name, galley, pal.dim);
    }

    solo_button_into(&painter, pal, at.solo, strip.is_soloed);
    mute_button_into(&painter, pal, at.mute, strip.is_muted);

    let galley = painter.layout_job(span_at(TRIM_LABEL, size::TRIM_LABEL_SIZE, pal.faint));
    painter.galley(
        Pos2::new(
            at.trim_label.min.x,
            at.trim_label.center().y - galley.size().y * 0.5,
        ),
        galley,
        pal.faint,
    );
    let rolled = roll_at(phase);
    fader_into(
        &painter,
        pal,
        at.trim_at(strip.gain),
        false,
        strip
            .gain_pending()
            .map(|to| at.trim_reach(strip.gain, to, rolled)),
    );
    fader_into(
        &painter,
        pal,
        at.fader_at(strip.opacity),
        strip.tally == Tally::Live,
        strip
            .opacity_pending()
            .map(|to| at.fader_reach(strip.opacity, to, rolled)),
    );
    meter_into(&painter, pal, at.meter, strip.level.map(|l| at.meter_at(l)));

    let galley = painter.layout_job(span_at(
        &format!("{:.2}", strip.opacity),
        size::STRIP_NUM_SIZE,
        pal.text,
    ));
    centre_galley(&painter, at.num, galley, pal.text);

    mini_into(&painter, pal, at.blend, true, |painter, colour| {
        let galley = painter.layout_job(span_at(strip.blend.name(), size::MINI_SIZE, colour));
        centre_galley(painter, at.blend, galley, colour);
    });
    mini_into(&painter, pal, at.mask, false, |painter, colour| {
        mask_mark(painter, at.mask.center(), colour, strip.mask);
    });
}

/// The meter: the well, the mean's column and the peak's mark.
fn meter_into(painter: &egui::Painter, pal: &Palette, well: Rect, meter: Option<Meter>) {
    let radius = CornerRadius::same((well.width() * 0.5) as u8);
    painter.rect_filled(well, radius, pal.well);
    painter.rect_stroke(
        well,
        radius,
        Stroke::new(size::HAIRLINE, pal.hair),
        StrokeKind::Inside,
    );
    let Some(meter) = meter else {
        return;
    };
    let painter = painter.with_clip_rect(meter.well);
    gradient(&painter, meter.fill, Axis::Column, pal.mint, pal.sun, false);
    painter.rect_filled(meter.peak, CornerRadius::ZERO, pal.pink);
}

/// Draws the mask shape glyph (none, linear split, or radial center dot).
pub fn mask_mark(painter: &egui::Painter, centre: Pos2, colour: Color32, mask: Mask) {
    let r = size::MINI_SIZE * 0.5;
    match mask {
        Mask::None => {}
        Mask::Linear => {
            painter
                .with_clip_rect(Rect::from_min_max(
                    Pos2::new(centre.x, centre.y - r),
                    Pos2::new(centre.x + r, centre.y + r),
                ))
                .circle_filled(centre, r, colour);
        }
        Mask::Radial => {
            painter.circle_filled(centre, r * 0.5, colour);
        }
    }
    painter.circle_stroke(centre, r, Stroke::new(size::HAIRLINE, colour));
}

/// Renders the SOLO toggle button on the mixer channel strip.
fn solo_button_into(painter: &egui::Painter, pal: &Palette, rect: Rect, active: bool) {
    let radius = CornerRadius::same((rect.height() * 0.5) as u8);
    let (fill, stroke_col, ink) = match active {
        true => (
            super::super::widgets::fader::tint(pal.sun, 25),
            pal.sun,
            pal.sun,
        ),
        false => (pal.well, pal.line, pal.dim),
    };
    painter.rect_filled(rect, radius, fill);
    painter.rect_stroke(
        rect,
        radius,
        Stroke::new(size::HAIRLINE, stroke_col),
        StrokeKind::Inside,
    );
    let galley = painter.layout_job(span_at("S", size::TALLY_SIZE, ink));
    centre_galley(painter, rect, galley, ink);
}

/// Renders the MUTE toggle button on the mixer channel strip.
fn mute_button_into(painter: &egui::Painter, pal: &Palette, rect: Rect, active: bool) {
    let radius = CornerRadius::same((rect.height() * 0.5) as u8);
    let (fill, stroke_col, ink) = match active {
        true => (
            super::super::widgets::fader::tint(pal.pink, 25),
            pal.pink,
            pal.pink,
        ),
        false => (pal.well, pal.line, pal.faint),
    };
    painter.rect_filled(rect, radius, fill);
    painter.rect_stroke(
        rect,
        radius,
        Stroke::new(size::HAIRLINE, stroke_col),
        StrokeKind::Inside,
    );
    let galley = painter.layout_job(span_at("M", size::TALLY_SIZE, ink));
    centre_galley(painter, rect, galley, ink);
}
