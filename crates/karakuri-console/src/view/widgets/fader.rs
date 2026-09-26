use super::super::*;
use egui::epaint::text::{FontId, LayoutJob};
use egui::{Color32, CornerRadius, FontFamily, Pos2, Rect, Stroke, StrokeKind, Ui};

// ---------------------------------------------------------------------------
// Fader, meter, and shared drawing primitives
// ---------------------------------------------------------------------------

/// How much of a track a value fills from zero (left for row, bottom for column).
pub(crate) fn filled(track: Rect, axis: Axis, at: f32) -> Rect {
    let at = unit(at);
    match axis {
        Axis::Row => Rect::from_min_max(
            track.min,
            Pos2::new(track.min.x + track.width() * at, track.max.y),
        ),
        Axis::Column => Rect::from_min_max(
            Pos2::new(track.min.x, track.max.y - track.height() * at),
            track.max,
        ),
    }
}

/// Computes fader layout (track, fill, knob) along `axis` given value `at`.
///
/// The knob is centred on the fill's moving edge, with custom inset and knob dimensions.
pub(crate) fn fader(track: Rect, axis: Axis, at: f32, inset: f32, knob: egui::Vec2) -> Fader {
    let inside = track.shrink(inset);
    let fill = filled(inside, axis, at);
    let edge = match axis {
        Axis::Row => Pos2::new(fill.max.x, track.center().y),
        Axis::Column => Pos2::new(track.center().x, fill.min.y),
    };
    Fader {
        track,
        axis,
        fill,
        knob: Rect::from_center_size(edge, knob),
        travel: match axis {
            Axis::Row => inside.width(),
            Axis::Column => inside.height(),
        },
    }
}

/// Hit-tests a fader knob at pointer `p`, returning the drag offset from knob center.
pub(crate) fn grabbed(fader: Fader, knob: Knob, p: Pos2) -> Option<Grab> {
    if !fader.knob.contains(p) {
        return None;
    }
    // A row fills from the left and a column fills from the bottom.
    let (zero, edge, coord) = match fader.axis {
        Axis::Row => (fader.fill.min.x, fader.fill.max.x, p.x),
        Axis::Column => (fader.fill.max.y, fader.fill.min.y, p.y),
    };
    Grab::new(knob, fader.axis, zero, fader.travel, coord - edge)
}

/// A plain span at a size that is not the console's [`size::BASE`] — this bay
/// has four of them, which is why it takes one.
pub(crate) fn span_at(text: &str, size: f32, colour: Color32) -> LayoutJob {
    LayoutJob::simple_singleline(
        text.to_owned(),
        FontId::new(size, FontFamily::Proportional),
        colour,
    )
}

/// Draws the drop target outline around `rect` when carrying a Set.
///
/// Indicates where a drop will land per [ADR-0265](../../../../docs/adr/0265-a-release-names-the-deck-and-nothing-is-refused.md).
pub(crate) fn drop_ring(ui: &Ui, pal: &Palette, at: Rect, radius: f32) {
    ui.painter().rect_stroke(
        at,
        CornerRadius::same(radius as u8),
        Stroke::new(size::DROP_RING, pal.text),
        StrokeKind::Outside,
    );
}

/// Draws the focus indicator on the bay addressed by key presses.
///
/// Uses an accent line along the boundary for headless rows or an inset frame for headed bays.
pub(crate) fn wfocus_into(ui: &Ui, pal: &Palette, at: Rect) {
    let painter = ui.painter();
    let stroke = Stroke::new(1.5, pal.lav);
    if at.height() > size::HEAD_H + 2.0 {
        // Headless rows: Transport at the top, Outputs at the bottom.
        if at.min.y <= 1.0 {
            // Transport: crisp accent line along the bottom border of the bar.
            let y = at.max.y - 0.75;
            painter.line_segment([Pos2::new(at.min.x, y), Pos2::new(at.max.x, y)], stroke);
        } else {
            // Outputs: crisp accent line along the top border of the bar.
            let y = at.min.y + 0.75;
            painter.line_segment([Pos2::new(at.min.x, y), Pos2::new(at.max.x, y)], stroke);
        }
    } else {
        // Headed bay: clean inset accent framing the bay head with matching top radius.
        let radius = CornerRadius {
            nw: size::BAY_RADIUS as u8,
            ne: size::BAY_RADIUS as u8,
            sw: 0,
            se: 0,
        };
        painter.rect_stroke(at, radius, stroke, StrokeKind::Inside);
    }
}

/// The focus indicator on a folded bay's card, framing all four rounded corners.
pub(crate) fn folded_wfocus_into(ui: &Ui, pal: &Palette, at: Rect) {
    let stroke = Stroke::new(1.5, pal.lav);
    let radius = CornerRadius::same(size::BAY_RADIUS as u8);
    ui.painter()
        .rect_stroke(at, radius, stroke, StrokeKind::Inside);
}

/// Paints a fader component: the well background, gradient fill, and knob.
///
/// If `live` is true, renders a pink rim and halo indicating on-air status.
pub(crate) fn fader_into(
    painter: &egui::Painter,
    pal: &Palette,
    fader: Fader,
    live: bool,
    reach: Option<Reach>,
) {
    slider_track_into(painter, pal, fader.track, fader.fill, fader.axis);

    // Lavender destination wash over the fill when moving towards an armed target (ADR-0164).
    if let Some(reach) = reach.filter(|reach| positive(reach.band)) {
        painter.rect_filled(reach.band, CornerRadius::ZERO, tint(pal.lav, 20));
    }

    let knob_r = CornerRadius::same((fader.knob.width().min(fader.knob.height()) * 0.5) as u8);
    if live {
        painter.add(
            egui::epaint::Shadow {
                offset: [0, 0],
                blur: size::VFADER_KNOB_GLOW,
                spread: 0,
                color: pal.glow_pink,
            }
            .as_shape(fader.knob, knob_r),
        );
    }
    painter.rect_filled(fader.knob, knob_r, pal.panel);
    painter.rect_stroke(
        fader.knob,
        knob_r,
        Stroke::new(
            size::HAIRLINE,
            match live {
                true => pal.pink,
                false => pal.line,
            },
        ),
        StrokeKind::Outside,
    );

    // Destination tick mark on top of knob indicating target position.
    if let Some(reach) = reach {
        painter.rect_filled(reach.mark, CornerRadius::ZERO, pal.lav);
    }
}

/// Draws a `.mini` capsule widget with 1px border.
pub(crate) fn mini_into(
    painter: &egui::Painter,
    pal: &Palette,
    rect: Rect,
    sel: bool,
    contents: impl FnOnce(&egui::Painter, Color32),
) {
    let radius = CornerRadius::same((size::MINI_H * 0.5) as u8);
    match sel {
        true => {
            painter.rect_filled(rect, radius, tint(pal.lav, 15));
        }
        false => {
            painter.rect_stroke(
                rect,
                radius,
                Stroke::new(size::HAIRLINE, pal.line),
                StrokeKind::Inside,
            );
        }
    }
    contents(
        painter,
        match sel {
            true => pal.lav,
            false => pal.faint,
        },
    );
}

/// Renders a two-colour gradient mesh along `axis`.
///
/// If `capsule` is true, rounds the ends into circles.
pub(crate) fn gradient(
    painter: &egui::Painter,
    rect: Rect,
    axis: Axis,
    from: Color32,
    to: Color32,
    capsule: bool,
) {
    if !(rect.width() > 0.0 && rect.height() > 0.0) {
        return;
    }
    let painter = painter.with_clip_rect(rect);
    // Gradients run left-to-right for rows, and bottom-to-top for columns.
    let colour_at = |p: Pos2| match axis {
        Axis::Row => match p.x <= rect.center().x {
            true => from,
            false => to,
        },
        Axis::Column => match p.y >= rect.center().y {
            true => from,
            false => to,
        },
    };
    let mut mesh = egui::epaint::Mesh::default();
    for corner in [
        rect.left_top(),
        rect.right_top(),
        rect.right_bottom(),
        rect.left_bottom(),
    ] {
        mesh.colored_vertex(corner, colour_at(corner));
    }
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    painter.add(egui::Shape::mesh(mesh));

    if capsule {
        let r = rect.width().min(rect.height()) * 0.5;
        let (start, end) = match axis {
            Axis::Row => (
                Pos2::new(rect.min.x + r, rect.center().y),
                Pos2::new(rect.max.x - r, rect.center().y),
            ),
            Axis::Column => (
                Pos2::new(rect.center().x, rect.max.y - r),
                Pos2::new(rect.center().x, rect.min.y + r),
            ),
        };
        painter.circle_filled(start, r, from);
        painter.circle_filled(end, r, to);
    }
}

/// `color-mix(in srgb, X n%, transparent)`, as the alpha it is: `n`% of 255,
/// rounded. The mock's own wash behind an armed control, a live tally and a
/// selected mini, and `room`'s documentation is where the equivalence is
/// argued.
pub(crate) fn tint(colour: Color32, percent: u8) -> Color32 {
    let alpha = ((percent as u32 * 255 + 50) / 100) as u8;
    Color32::from_rgba_unmultiplied(colour.r(), colour.g(), colour.b(), alpha)
}
