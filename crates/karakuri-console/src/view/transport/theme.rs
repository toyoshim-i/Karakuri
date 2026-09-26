//! Transport row theme selector pill and dropdown menu.

use egui::{vec2, Color32, CornerRadius, FontFamily, FontId, Pos2, Rect, Stroke, StrokeKind, Ui};
use karakuri_layout::Point;

use super::*;
use crate::room::{size, Palette, ThemeMode};
use crate::view::widgets::card::{card_row_text, popup_card};
use crate::view::widgets::fader::tint;
use crate::view::widgets::glyph::CHEVRON_W;

/// Pill prefix label for theme selection.
pub const THEME_LABEL: &str = "theme";

/// User interaction outcome from pressing inside theme pill or menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeAsk {
    /// Toggle the dropdown menu open/closed.
    Toggle,
    /// Select a specific theme mode.
    Select(ThemeMode),
    /// Dismiss / shut the dropdown menu.
    Shut,
}

/// Layout and geometry for the theme selector pill.
#[derive(Debug, Clone, PartialEq)]
pub struct ThemePill {
    pub pill: Rect,
    pub text: Rect,
    pub chevron: Rect,
    pub menu: Option<Rect>,
    pub mode: ThemeMode,
}

impl ThemePill {
    /// Whether `p` is on the pill itself.
    pub fn hit(&self, p: Point) -> bool {
        self.pill.contains(Pos2::new(p.x, p.y))
    }

    /// Whether `p` is anywhere this control owns (the pill, or the menu while down).
    pub fn owns(&self, p: Point) -> bool {
        let at = Pos2::new(p.x, p.y);
        self.pill.contains(at) || self.menu.is_some_and(|m| m.contains(at))
    }

    /// Returns the bounding box for the row at `index` in the dropdown menu.
    pub fn row(&self, index: usize) -> Option<Rect> {
        let menu = self.menu?;
        if index >= ThemeMode::ALL.len() {
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

    /// Which theme mode `p` is on, or `None` if on padding or outside.
    pub fn item(&self, p: Point) -> Option<ThemeMode> {
        let at = Pos2::new(p.x, p.y);
        for (index, &mode) in ThemeMode::ALL.iter().enumerate() {
            if let Some(r) = self.row(index) {
                if r.contains(at) {
                    return Some(mode);
                }
            }
        }
        None
    }

    /// Hit-tests a click at `p` against the pill or open menu items.
    pub fn ask(&self, open: bool, p: Point) -> Option<ThemeAsk> {
        if self.hit(p) {
            Some(ThemeAsk::Toggle)
        } else if open {
            match self.item(p) {
                Some(mode) => Some(ThemeAsk::Select(mode)),
                None => Some(ThemeAsk::Shut),
            }
        } else {
            None
        }
    }
}

/// Measures and positions the theme selector pill in the transport row.
#[allow(clippy::too_many_arguments)]
pub fn theme_pill(
    ctx: &egui::Context,
    layout: &karakuri_layout::Layout,
    values: Option<Transport>,
    audio: Option<&AudioIn>,
    tracker: Option<Tracker>,
    map: Option<&MapPill>,
    arr: &Arrangement,
    mode: ThemeMode,
    open: bool,
) -> Option<ThemePill> {
    let row = transport(ctx, layout, values)?;
    let strip = to_egui(layout.rect(layout.find("transport")?));

    let width = |text: &str| {
        ctx.fonts_mut(|f| {
            f.layout_no_wrap(
                text.to_owned(),
                FontId::new(size::BASE, FontFamily::Proportional),
                Color32::PLACEHOLDER,
            )
            .size()
            .x
        })
    };

    let label = format!("{THEME_LABEL} · {}", mode.word());
    let text_w = width(&label);
    let pill_w = size::PILL_PAD_X * 2.0 + text_w + size::SINK_GAP + CHEVRON_W;
    let mid = strip.center().y;

    // Anchor after arrangement pill, map pill, learn pill, tracker group, audio-in, or bar.
    let after = match (
        arrangement(ctx, layout, values, audio, tracker, map, arr),
        map_pill(ctx, layout, values, audio, tracker, map),
        learn_pill(ctx, layout, values, audio, tracker, map, false),
        tracker_group(ctx, layout, values, audio, tracker),
        audio_in(ctx, layout, values, audio),
    ) {
        (Some(pill), _, _, _, _) => pill.pill.max.x,
        (None, Some(pill), _, _, _) => pill.pill.max.x,
        (None, None, Some(learn), _, _) => learn.pill.max.x,
        (None, None, None, Some(group), _) => group.double.max.x,
        (None, None, None, None, Some(before)) => before.pill.max.x,
        (None, None, None, None, None) => row.bar.max.x,
    };

    let right = row.frame.min.x - size::TRANSPORT_GAP;
    let pill = Rect::from_min_size(
        Pos2::new(right - pill_w, mid - size::PILL_H * 0.5),
        vec2(pill_w, size::PILL_H),
    );

    if !strip.contains_rect(pill) || pill.min.x < after + size::TRANSPORT_GAP {
        return None;
    }

    let text = Rect::from_min_size(
        Pos2::new(pill.min.x + size::PILL_PAD_X, mid - size::PILL_H * 0.5),
        vec2(text_w, size::PILL_H),
    );
    let chevron = Rect::from_center_size(
        Pos2::new(pill.max.x - size::PILL_PAD_X - CHEVRON_W * 0.5, mid),
        vec2(CHEVRON_W, CHEVRON_W),
    );

    let menu = if open {
        let menu_w = 90.0f32.max(pill_w);
        let menu_h = size::LIB_LIST_PAD * 2.0 + size::LIB_ROW_H * ThemeMode::ALL.len() as f32;
        Some(Rect::from_min_size(
            Pos2::new(pill.min.x, pill.max.y + size::PILL_GAP),
            vec2(menu_w, menu_h),
        ))
    } else {
        None
    };

    Some(ThemePill {
        pill,
        text,
        chevron,
        menu,
        mode,
    })
}

/// Paints the theme selector pill and its dropdown menu if open.
pub(crate) fn theme_into(ui: &Ui, pal: &Palette, pill: &ThemePill, mode: ThemeMode, open: bool) {
    let painter = ui.painter();
    let border_color = if open { pal.lav } else { pal.line };
    painter.rect_stroke(
        pill.pill,
        CornerRadius::same((size::PILL_H * 0.5) as u8),
        Stroke::new(size::HAIRLINE, border_color),
        StrokeKind::Inside,
    );

    let text_color = if open { pal.text } else { pal.dim };
    let label = format!("{THEME_LABEL} · {}", mode.word());
    let galley = painter.layout_no_wrap(
        label,
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

    chevron_down(painter, pill.chevron, text_color);

    let Some(card) = pill.menu else {
        return;
    };
    popup_card(painter, pal, card);

    let hover_pos = ui.input(|i| i.pointer.hover_pos());

    for (index, &item_mode) in ThemeMode::ALL.iter().enumerate() {
        let Some(row) = pill.row(index) else {
            continue;
        };
        let is_selected = item_mode == mode;
        let is_hovered = hover_pos.is_some_and(|pos| row.contains(pos));

        if is_hovered {
            painter.rect_filled(row, CornerRadius::same(3), tint(pal.lav, 24));
        }

        let item_color = if is_selected {
            pal.mint
        } else if is_hovered {
            pal.text
        } else {
            pal.dim
        };

        card_row_text(painter, row, item_mode.word(), item_color);

        if is_selected {
            let dot_x = row.max.x - 8.0;
            let mid_y = row.center().y;
            painter.circle_filled(Pos2::new(dot_x, mid_y), 2.5, pal.mint);
        }
    }
}
