//! Filter inputs and editable text fields.

use super::super::*;
use egui::{CornerRadius, Painter, Pos2, Rect, Stroke, StrokeKind};

/// The text editing caret glyph used in interactive name and filter fields.
pub const CARET: char = '▏';

/// A capsule filter input field: hairline inside border, text clipping, and left-padded text.
pub fn filter_field(painter: &Painter, pal: &Palette, rect: Rect, text: &str) {
    painter.rect_stroke(
        rect,
        CornerRadius::same((rect.height() * 0.5) as u8),
        Stroke::new(size::HAIRLINE, pal.line),
        StrokeKind::Inside,
    );
    let painter = painter.with_clip_rect(rect);
    let galley = painter.layout_job(span_at(text, size::BASE, pal.faint));
    painter.galley(
        Pos2::new(
            rect.min.x + size::FIELD_PAD_X,
            rect.center().y - galley.size().y * 0.5,
        ),
        galley,
        pal.faint,
    );
}

/// An editable text field with optional focus background tint and centered typography.
pub fn editable_text_field(
    painter: &Painter,
    pal: &Palette,
    rect: Rect,
    text: &str,
    editing: bool,
    radius: u8,
) {
    if editing {
        painter.rect_filled(rect, CornerRadius::same(radius), pal.tint);
    }
    let galley = painter.layout_job(span_at(text, size::BASE, pal.text));
    painter.galley(
        Pos2::new(rect.min.x, rect.center().y - galley.size().y * 0.5),
        galley,
        pal.text,
    );
}
