//! Prompt bay header pill, dropdown card geometry, and input hit-testing.

use egui::{vec2, Pos2, Rect};
use karakuri_layout::Point;

use super::cli::{CliPreset, CliSelection};
use super::state::PromptState;
use crate::room::size;
use crate::view::widgets::fold_grip::GRIP_W;
use crate::view::widgets::glyph::CHEVRON_W;
use crate::view::widgets::head::head_box;
use crate::view::widgets::pills::pill_width;

/// Bay title for Prompt bay.
pub const PROMPT_TITLE: &str = "Prompt";

/// Menu item row height in logical pixels.
pub const MENU_ITEM_H: f32 = 18.0;

/// Menu card vertical padding.
pub const MENU_PAD_Y: f32 = 6.0;

/// Menu card horizontal padding.
pub const MENU_PAD_X: f32 = 6.0;

/// Number of columns in the dropdown menu.
pub const MENU_COLS: usize = 2;

/// Width of each column.
pub const MENU_COL_W: f32 = 112.0;

/// Gap between columns.
pub const MENU_COL_GAP: f32 = 4.0;

/// Menu card total width (2 columns + padding + gap).
pub const MENU_CARD_W: f32 =
    MENU_PAD_X * 2.0 + (MENU_COL_W * MENU_COLS as f32) + (MENU_COL_GAP * (MENU_COLS - 1) as f32);

/// Total items in the dropdown menu (presets + custom).
pub const MENU_ITEM_COUNT: usize = CliPreset::ALL.len() + 1;

/// Rows per column in the multi-column menu.
pub const MENU_ROWS_PER_COL: usize = MENU_ITEM_COUNT.div_ceil(MENU_COLS);

/// User interaction outcome from pressing inside Prompt bay header or menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptAsk {
    /// Toggle the dropdown menu open/closed.
    ToggleMenu,
    /// Select a preset agent CLI session.
    SelectPreset(CliPreset),
    /// Select a custom CLI session.
    SelectCustom,
    /// Dismiss / shut the dropdown menu.
    Shut,
}

/// Calculates the bounding rectangle of the CLI selector pill in the bay header.
pub fn prompt_pill(ctx: &egui::Context, bay_rect: Rect, selection: &CliSelection) -> Rect {
    let head = head_box(bay_rect);
    let label = selection.pill_label();
    let text_w = pill_width(ctx, &label);
    let w = text_w + size::SINK_GAP + CHEVRON_W;
    let right = head.max.x - size::HEAD_PAD_X - (GRIP_W + size::PILL_GAP);
    let mid = head.center().y;
    Rect::from_min_size(
        Pos2::new(right - w, mid - size::PILL_H * 0.5),
        vec2(w, size::PILL_H),
    )
}

/// Calculates the bounding rectangle of the floating dropdown menu.
///
/// Flips upward if there is insufficient room below the pill in the viewport.
pub fn prompt_menu_rect(pill_rect: Rect, viewport: Rect) -> Rect {
    let w = MENU_CARD_W;
    let h = MENU_PAD_Y * 2.0 + MENU_ITEM_H * (MENU_ROWS_PER_COL as f32);

    let mut min_x = pill_rect.max.x - w;
    if min_x < viewport.min.x + 8.0 {
        min_x = viewport.min.x + 8.0;
    }
    if min_x + w > viewport.max.x - 8.0 {
        min_x = viewport.max.x - 8.0 - w;
    }

    // Prefer opening downward; if it would clip the viewport bottom, flip upward above the pill.
    let min_y = if pill_rect.max.y + 4.0 + h <= viewport.max.y - 8.0 {
        pill_rect.max.y + 4.0
    } else if pill_rect.min.y - 4.0 - h >= viewport.min.y + 8.0 {
        pill_rect.min.y - 4.0 - h
    } else {
        (viewport.max.y - 8.0 - h).max(viewport.min.y + 8.0)
    };

    Rect::from_min_size(Pos2::new(min_x, min_y), vec2(w, h))
}

/// Computes the item rectangle for an item index (0..MENU_ITEM_COUNT).
pub fn prompt_item_rect(menu_rect: Rect, index: usize) -> Option<Rect> {
    if index >= MENU_ITEM_COUNT {
        return None;
    }
    let col = index / MENU_ROWS_PER_COL;
    let row = index % MENU_ROWS_PER_COL;
    let x = menu_rect.min.x + MENU_PAD_X + (col as f32) * (MENU_COL_W + MENU_COL_GAP);
    let y = menu_rect.min.y + MENU_PAD_Y + (row as f32) * MENU_ITEM_H;
    Some(Rect::from_min_size(
        Pos2::new(x, y),
        vec2(MENU_COL_W, MENU_ITEM_H),
    ))
}

/// Hit-tests a point against the Prompt bay header pill and dropdown menu.
pub fn prompt_ask(
    ctx: &egui::Context,
    bay_rect: Rect,
    viewport: Rect,
    state: &PromptState,
    p: Point,
) -> Option<PromptAsk> {
    let pill = prompt_pill(ctx, bay_rect, &state.selection);
    let pos = Pos2::new(p.x, p.y);

    if state.menu_open {
        let menu = prompt_menu_rect(pill, viewport);
        if menu.contains(pos) {
            let rel_x = pos.x - (menu.min.x + MENU_PAD_X);
            let rel_y = pos.y - (menu.min.y + MENU_PAD_Y);
            if rel_x >= 0.0 && rel_y >= 0.0 {
                let col_pitch = MENU_COL_W + MENU_COL_GAP;
                let col = (rel_x / col_pitch) as usize;
                let in_col_x = rel_x - (col as f32) * col_pitch;
                let row = (rel_y / MENU_ITEM_H) as usize;

                if col < MENU_COLS && in_col_x <= MENU_COL_W && row < MENU_ROWS_PER_COL {
                    let index = col * MENU_ROWS_PER_COL + row;
                    if index < CliPreset::ALL.len() {
                        let preset = CliPreset::ALL[index];
                        if preset.is_available() {
                            return Some(PromptAsk::SelectPreset(preset));
                        }
                        // Unavailable presets are disabled / cannot be selected
                        return None;
                    } else if index == CliPreset::ALL.len() {
                        return Some(PromptAsk::SelectCustom);
                    }
                }
            }
            return None;
        }

        if pill.contains(pos) {
            return Some(PromptAsk::ToggleMenu);
        }

        // Click outside open menu dismisses it (Rule 2)
        return Some(PromptAsk::Shut);
    }

    if pill.contains(pos) {
        return Some(PromptAsk::ToggleMenu);
    }

    None
}
