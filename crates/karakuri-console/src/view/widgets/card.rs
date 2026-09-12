//! Card drawing primitives for console bays and floating overlays.

use egui::{
    Color32, CornerRadius, FontFamily, FontId, Painter, Pos2, Rect, Stroke, StrokeKind, Ui,
};

use super::super::*;

/// The standard overlay card radius.
pub const CARD_RADIUS: u8 = 8;

/// A bay's card: `.bay`'s panel fill, 11px radius and drop shadow.
pub fn bay_card(ui: &Ui, pal: &Palette, rect: Rect) {
    let radius = CornerRadius::same(size::BAY_RADIUS as u8);
    ui.painter().add(pal.shadow.as_shape(rect, radius));
    ui.painter().rect_filled(rect, radius, pal.panel);
}

/// An overlay popup card: drop shadow, panel fill, and hairline stroke with standard 8px radius.
pub fn popup_card(painter: &Painter, pal: &Palette, rect: Rect) {
    let radius = CornerRadius::same(CARD_RADIUS);
    painter.add(pal.shadow.as_shape(rect, radius));
    painter.rect_filled(rect, radius, pal.panel);
    painter.rect_stroke(
        rect,
        radius,
        Stroke::new(size::HAIRLINE, pal.line),
        StrokeKind::Inside,
    );
}

/// Standard row text layout helper for popup cards.
pub fn card_row_text(painter: &Painter, rect: Rect, text: &str, colour: Color32) {
    let galley = painter.layout_no_wrap(
        text.to_owned(),
        FontId::new(size::BASE, FontFamily::Proportional),
        colour,
    );
    painter.galley(
        Pos2::new(
            rect.min.x + size::LIB_ROW_PAD_X,
            rect.center().y - galley.size().y * 0.5,
        ),
        galley,
        colour,
    );
}
