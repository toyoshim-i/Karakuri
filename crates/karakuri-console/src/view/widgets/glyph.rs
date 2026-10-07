//! Glyph and vector mark drawing primitives.

use super::super::*;
use egui::{Color32, Painter, Pos2, Rect, Stroke};

/// The `▾` at the end of a pill or pulldown, drawn rather than typed.
///
/// Half the type it sits beside wide and half of that tall, which is about what
/// the glyph's ink measures at [`size::BASE`].
pub const CHEVRON_W: f32 = size::BASE * 0.5;
pub const CHEVRON_H: f32 = CHEVRON_W * 0.5;

/// Draws a small down-pointing triangle (`▾`) inside `rect`.
pub fn chevron_down(painter: &Painter, rect: Rect, colour: Color32) {
    painter.add(egui::Shape::convex_polygon(
        vec![
            rect.left_top(),
            rect.right_top(),
            Pos2::new(rect.center().x, rect.max.y),
        ],
        colour,
        Stroke::NONE,
    ));
}

/// An arrow triangle pointing forward or backward.
pub fn arrow_mark(painter: &Painter, centre: Pos2, across: f32, colour: Color32, back: bool) {
    let r = across * 0.5;
    let point = match back {
        true => centre.x - r,
        false => centre.x + r,
    };
    let base = match back {
        true => centre.x + r,
        false => centre.x - r,
    };
    painter.add(egui::Shape::convex_polygon(
        vec![
            Pos2::new(point, centre.y),
            Pos2::new(base, centre.y - r),
            Pos2::new(base, centre.y + r),
        ],
        colour,
        Stroke::NONE,
    ));
}

/// Draws a circular reload/restore arrow glyph centered at `center`.
pub fn reload_glyph(painter: &Painter, center: Pos2, r: f32, color: Color32) {
    let stroke = Stroke::new(1.2, color);
    // Draw 3/4 circle arc (approx 270 degrees, from -45 to 225 deg).
    let steps = 12;
    let mut prev = None;
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let angle = -std::f32::consts::PI * 0.25 + std::f32::consts::PI * 1.5 * t;
        let pt = Pos2::new(center.x + r * angle.cos(), center.y + r * angle.sin());
        if let Some(p0) = prev {
            painter.line_segment([p0, pt], stroke);
        }
        prev = Some(pt);
    }
    // Arrowhead pointing along the tangent at the end of the arc
    if let Some(tip) = prev {
        let s = r * 0.7;
        painter.add(egui::Shape::convex_polygon(
            vec![
                tip,
                Pos2::new(tip.x - s, tip.y - s * 0.3),
                Pos2::new(tip.x - s * 0.3, tip.y + s),
            ],
            color,
            Stroke::NONE,
        ));
    }
}
