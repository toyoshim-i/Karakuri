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
