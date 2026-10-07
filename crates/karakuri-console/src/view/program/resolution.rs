//! Program bay resolution selector pill and dropdown menu component.

use egui::{vec2, Color32, CornerRadius, FontFamily, FontId, Pos2, Rect, Stroke, StrokeKind, Ui};
use karakuri_layout::Point;

use super::*;
use crate::room::{size, Palette};
use crate::view::widgets::card::{card_row_text, popup_card};
use crate::view::widgets::fader::tint;
use crate::view::widgets::glyph::{chevron_down, CHEVRON_H, CHEVRON_W};

/// Minimum width of the floating resolution dropdown menu.
pub const MENU_MIN_W: f32 = 160.0;

/// User interaction outcome from pressing inside resolution pill or menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolutionAsk {
    /// Toggle the dropdown menu open/closed.
    Toggle,
    /// Select a specific resolution index.
    Select(usize),
    /// Dismiss / shut the dropdown menu.
    Shut,
}

/// Layout and geometry for the resolution selector pill and dropdown card.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolutionPill {
    pub pill: Rect,
    pub text: Rect,
    pub chevron: Rect,
    pub menu: Option<Rect>,
}

impl ResolutionPill {
    /// Whether `p` is on the pill itself.
    pub fn hit(&self, p: Point) -> bool {
        self.pill.contains(Pos2::new(p.x, p.y))
    }

    /// Whether `p` is anywhere this control owns (the pill, or the menu while open).
    pub fn owns(&self, p: Point) -> bool {
        let at = Pos2::new(p.x, p.y);
        self.pill.contains(at) || self.menu.is_some_and(|m| m.contains(at))
    }

    /// Returns the bounding box for the row at `index` in the dropdown menu.
    pub fn row(&self, index: usize, count: usize) -> Option<Rect> {
        let menu = self.menu?;
        if index >= count {
            return None;
        }
        Some(Rect::from_min_size(
            Pos2::new(
                menu.min.x + size::LIB_LIST_PAD,
                menu.min.y + size::LIB_LIST_PAD + size::LIB_ROW_H * index as f32,
            ),
            vec2(menu.width() - size::LIB_LIST_PAD * 2.0, size::LIB_ROW_H),
        ))
    }

    /// Which item index `p` is on, or `None` if outside.
    pub fn item(&self, count: usize, p: Point) -> Option<usize> {
        let at = Pos2::new(p.x, p.y);
        for index in 0..count {
            if let Some(r) = self.row(index, count) {
                if r.contains(at) {
                    return Some(index);
                }
            }
        }
        None
    }

    /// Hit-tests a click at `p` against the pill or open menu items.
    pub fn ask(&self, open: bool, count: usize, p: Point) -> Option<ResolutionAsk> {
        if self.hit(p) {
            Some(ResolutionAsk::Toggle)
        } else if open {
            match self.item(count, p) {
                Some(idx) => Some(ResolutionAsk::Select(idx)),
                None => Some(ResolutionAsk::Shut),
            }
        } else {
            None
        }
    }
}

/// Measures and positions the resolution selector pill and dropdown card in the Program bay.
pub fn resolution_pill(
    ctx: &egui::Context,
    bay_rect: Rect,
    viewport: Rect,
    resolutions: &[((u32, u32), String)],
    selected: usize,
    open: bool,
) -> Option<ResolutionPill> {
    if ctx.cumulative_pass_nr() == 0 || resolutions.is_empty() {
        return None;
    }
    let head = head_box(bay_rect);
    let mid = head.center().y;
    let label = resolutions
        .get(selected)
        .map(|(_, l)| l.as_str())
        .unwrap_or("---");

    let font_id = FontId::new(size::BASE, FontFamily::Proportional);
    let galley = ctx.fonts_mut(|f| f.layout_no_wrap(label.into(), font_id, Color32::WHITE));
    let text_w = galley.size().x;

    let title_w = ctx.fonts_mut(|f| {
        f.layout_no_wrap(
            "PROGRAM".into(),
            FontId::new(size::HEAD_SIZE, FontFamily::Proportional),
            Color32::WHITE,
        )
        .size()
        .x
    });

    let pill_w = size::PILL_PAD_X * 2.0 + text_w + size::SINK_GAP + CHEVRON_W;
    let left = head.min.x + size::HEAD_PAD_X + title_w + 12.0;
    let pill = Rect::from_min_size(
        Pos2::new(left, mid - size::PILL_H * 0.5),
        vec2(pill_w, size::PILL_H),
    );

    let text = Rect::from_min_size(
        Pos2::new(pill.min.x + size::PILL_PAD_X, pill.min.y),
        vec2(text_w, size::PILL_H),
    );

    let chevron = Rect::from_center_size(
        Pos2::new(
            pill.max.x - size::PILL_PAD_X - CHEVRON_W * 0.5,
            pill.center().y,
        ),
        vec2(CHEVRON_W, CHEVRON_H),
    );

    let menu = if open && !resolutions.is_empty() {
        let count = resolutions.len();
        let menu_w = MENU_MIN_W.max(pill.width());
        let menu_h = size::LIB_LIST_PAD * 2.0 + size::LIB_ROW_H * count as f32;

        let min_x = pill
            .min
            .x
            .clamp(viewport.min.x + 8.0, viewport.max.x - 8.0 - menu_w);
        let min_y = if pill.max.y + 4.0 + menu_h <= viewport.max.y - 8.0 {
            pill.max.y + 4.0
        } else {
            (pill.min.y - 4.0 - menu_h).max(viewport.min.y + 8.0)
        };
        Some(Rect::from_min_size(
            Pos2::new(min_x, min_y),
            vec2(menu_w, menu_h),
        ))
    } else {
        None
    };

    Some(ResolutionPill {
        pill,
        text,
        chevron,
        menu,
    })
}

/// Paints the resolution selector pill in the Program bay header (pure rendering).
pub fn resolution_pill_into(
    ui: &mut Ui,
    pal: &Palette,
    pill: &ResolutionPill,
    resolutions: &[((u32, u32), String)],
    selected: usize,
    open: bool,
) {
    if resolutions.is_empty() {
        return;
    }
    let painter = ui.painter();

    // 1. Pill container border & shape
    let border_color = if open { pal.mint } else { pal.line };
    painter.rect_stroke(
        pill.pill,
        CornerRadius::same((size::PILL_H * 0.5) as u8),
        Stroke::new(size::HAIRLINE, border_color),
        StrokeKind::Inside,
    );

    // 2. Pill label text
    let label = resolutions
        .get(selected)
        .map(|(_, l)| l.as_str())
        .unwrap_or("---");
    let text_color = if open { pal.text } else { pal.dim };
    let galley = painter.layout_no_wrap(
        label.into(),
        FontId::new(size::BASE, FontFamily::Proportional),
        text_color,
    );
    painter.galley(
        Pos2::new(
            pill.text.min.x,
            pill.text.center().y - galley.size().y * 0.5,
        ),
        galley,
        text_color,
    );

    // 3. Chevron glyph
    let chevron_color = if open { pal.mint } else { pal.dim };
    chevron_down(painter, pill.chevron, chevron_color);
}

/// Paints the floating dropdown menu card above bays in the modal overlay pass (pure rendering).
pub fn resolution_menu_into(
    ui: &mut Ui,
    pal: &Palette,
    pill: &ResolutionPill,
    resolutions: &[((u32, u32), String)],
    selected: usize,
) {
    let Some(menu_rect) = pill.menu else {
        return;
    };
    let painter = ui.painter();
    popup_card(painter, pal, menu_rect);

    let hover_pos = ui.input(|i| i.pointer.hover_pos());
    let card_painter = painter.with_clip_rect(menu_rect);

    for (index, (_res, item_label)) in resolutions.iter().enumerate() {
        let Some(row_rect) = pill.row(index, resolutions.len()) else {
            continue;
        };
        let is_selected = index == selected;
        let is_hovered = hover_pos.is_some_and(|pos| row_rect.contains(pos));

        if is_hovered {
            card_painter.rect_filled(row_rect, CornerRadius::same(3), tint(pal.mint, 22));
        }

        let item_color = if is_selected {
            pal.mint
        } else if is_hovered {
            pal.text
        } else {
            pal.dim
        };

        card_row_text(&card_painter, row_rect, item_label, item_color);

        if is_selected {
            let dot_x = row_rect.max.x - 8.0;
            let mid_y = row_rect.center().y;
            card_painter.circle_filled(Pos2::new(dot_x, mid_y), 2.5, pal.mint);
        }
    }
}

/// Paints both the resolution selector pill and floating dropdown menu (pure rendering).
pub fn resolution_into(
    ui: &mut Ui,
    pal: &Palette,
    pill: &ResolutionPill,
    resolutions: &[((u32, u32), String)],
    selected: usize,
    open: bool,
) {
    resolution_pill_into(ui, pal, pill, resolutions, selected, open);
    if open {
        resolution_menu_into(ui, pal, pill, resolutions, selected);
    }
}
