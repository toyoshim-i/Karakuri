//! Continuous slider tracks and scrubbers.

use super::super::*;
use egui::{CornerRadius, Painter, Rect, Stroke, StrokeKind};

/// A continuous slider track: well fill, hairline inset stroke, and gradient fill.
pub fn slider_track_into(painter: &Painter, pal: &Palette, track: Rect, fill: Rect, axis: Axis) {
    let radius = CornerRadius::same((track.width().min(track.height()) * 0.5) as u8);
    painter.rect_filled(track, radius, pal.well);
    painter.rect_stroke(
        track,
        radius,
        Stroke::new(size::HAIRLINE, pal.hair),
        StrokeKind::Inside,
    );
    gradient(painter, fill, axis, pal.mint, pal.lav, true);
}
