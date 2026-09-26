//! Prompt bay header pill, dropdown card geometry, and input hit-testing.

use egui::{vec2, Pos2, Rect};
use karakuri_layout::Point;

use super::cli::{CliPreset, CliSelection};
use super::state::PromptState;
use crate::room::size;
use crate::view::widgets::fold_grip::GRIP_W;
use crate::view::widgets::head::head_box;
use crate::view::widgets::pills::pill_width;

/// Bay title for Prompt bay.
pub const PROMPT_TITLE: &str = "Prompt";

/// Menu item row height in logical pixels.
pub const MENU_ITEM_H: f32 = 18.0;

/// Menu card vertical padding.
pub const MENU_PAD_Y: f32 = 6.0;

/// Menu card horizontal padding.
pub const MENU_PAD_X: f32 = 8.0;

/// Menu card total width.
pub const MENU_CARD_W: f32 = 130.0;

/// Total items in the dropdown menu (9 presets + custom).
pub const MENU_ITEM_COUNT: usize = CliPreset::ALL.len() + 1;

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
    let w = pill_width(ctx, &label);
    let right = head.max.x - size::HEAD_PAD_X - (GRIP_W + size::PILL_GAP);
    let mid = head.center().y;
    Rect::from_min_size(
        Pos2::new(right - w, mid - size::PILL_H * 0.5),
        vec2(w, size::PILL_H),
    )
}

/// Calculates the bounding rectangle of the floating dropdown menu.
pub fn prompt_menu_rect(pill_rect: Rect, viewport: Rect) -> Rect {
    let h = MENU_PAD_Y * 2.0 + MENU_ITEM_H * (MENU_ITEM_COUNT as f32);
    let mut min_x = pill_rect.max.x - MENU_CARD_W;
    if min_x < viewport.min.x + 8.0 {
        min_x = viewport.min.x + 8.0;
    }
    let min_y = pill_rect.max.y + 4.0;
    Rect::from_min_size(Pos2::new(min_x, min_y), vec2(MENU_CARD_W, h))
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
            let rel_y = pos.y - (menu.min.y + MENU_PAD_Y);
            if rel_y >= 0.0 {
                let index = (rel_y / MENU_ITEM_H) as usize;
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
