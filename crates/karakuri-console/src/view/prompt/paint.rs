//! Rendering for the Prompt bay terminal body, header pill, and dropdown menu.

use egui::epaint::text::FontId;
use egui::{FontFamily, Pos2, Rect, Stroke, Ui};
use karakuri_layout::Layout;

use super::cli::{CliPreset, CliSelection};
use super::head::{prompt_menu_rect, prompt_pill, MENU_ITEM_H, MENU_PAD_X, MENU_PAD_Y};
use super::state::PromptState;
use crate::room::{size, Palette};
use crate::view::to_egui;
use crate::view::widgets::card::popup_card;
use crate::view::widgets::head::head_box;
use crate::view::widgets::pills::pill_into;

/// Paints the Prompt bay header selector pill.
pub fn prompt_head_into(ui: &Ui, pal: &Palette, bay_rect: Rect, state: &PromptState) {
    let pill = prompt_pill(ui.ctx(), bay_rect, &state.selection);
    let label = state.selection.pill_label();
    let armed = state.menu_open || !state.selection.is_unselected();
    pill_into(ui, pal, pill, &label, armed);
}

/// Paints the floating CLI preset dropdown menu (Rule 2 modal overlay).
pub fn prompt_menu_into(ui: &Ui, pal: &Palette, layout: &Layout, state: &PromptState) {
    let Some(id) = layout.find("prompt") else {
        return;
    };
    let bay_rect = to_egui(layout.rect(id));
    let viewport = to_egui(layout.viewport());
    let pill = prompt_pill(ui.ctx(), bay_rect, &state.selection);
    let menu = prompt_menu_rect(pill, viewport);

    popup_card(ui.painter(), pal, menu);
    let painter = ui.painter().with_clip_rect(menu);
    let font_id = FontId::new(size::BASE, FontFamily::Monospace);

    let mut item_top = menu.min.y + MENU_PAD_Y;

    // 1. Presets
    for preset in CliPreset::ALL {
        let is_selected = matches!(&state.selection, CliSelection::Preset(p) if p == preset);
        let available = preset.is_available();
        let mid_y = item_top + MENU_ITEM_H * 0.5;

        let text_color = if available {
            if is_selected {
                pal.pink
            } else {
                pal.text
            }
        } else {
            pal.faint
        };

        let label = preset.display_name();
        let text_pos = Pos2::new(menu.min.x + MENU_PAD_X, mid_y - size::BASE * 0.5);
        let galley = painter.layout_no_wrap(label.to_owned(), font_id.clone(), text_color);
        let text_w = galley.size().x;
        painter.galley(text_pos, galley, text_color);

        // Strikethrough for unavailable CLIs (~~...~~)
        if !available {
            let line_y = mid_y;
            let strike_start = Pos2::new(text_pos.x, line_y);
            let strike_end = Pos2::new(text_pos.x + text_w, line_y);
            painter.line_segment([strike_start, strike_end], Stroke::new(1.0, pal.faint));
        }

        // Active indicator dot for currently selected CLI
        if is_selected {
            let dot_x = menu.max.x - MENU_PAD_X - 4.0;
            painter.circle_filled(Pos2::new(dot_x, mid_y), 3.0, pal.pink);
        }

        item_top += MENU_ITEM_H;
    }

    // 2. Custom command option
    let is_custom_selected = matches!(&state.selection, CliSelection::Custom(_));
    let mid_y = item_top + MENU_ITEM_H * 0.5;
    let custom_color = if is_custom_selected {
        pal.pink
    } else {
        pal.text
    };

    let text_pos = Pos2::new(menu.min.x + MENU_PAD_X, mid_y - size::BASE * 0.5);
    let galley = painter.layout_no_wrap("custom...".to_owned(), font_id.clone(), custom_color);
    painter.galley(text_pos, galley, custom_color);

    if is_custom_selected {
        let dot_x = menu.max.x - MENU_PAD_X - 4.0;
        painter.circle_filled(Pos2::new(dot_x, mid_y), 3.0, pal.pink);
    }
}

/// Paints the internal body of the Prompt bay (terminal area).
pub fn prompt_into(ui: &Ui, pal: &Palette, bay_rect: Rect, state: &PromptState) {
    let head = head_box(bay_rect);
    let body_top = head.max.y;
    if body_top >= bay_rect.max.y {
        return;
    }

    let body_rect = Rect::from_min_max(Pos2::new(bay_rect.min.x, body_top), bay_rect.max);

    let painter = ui.painter().with_clip_rect(body_rect);
    let font_id = FontId::new(size::BASE, FontFamily::Monospace);
    let pad_x = size::HEAD_PAD_X;
    let pad_y = 10.0;
    let mut cursor_y = body_rect.min.y + pad_y;

    // Terminal header info line
    let title = "karakuri agent terminal (m9)";
    let title_galley = painter.layout_no_wrap(title.to_owned(), font_id.clone(), pal.faint);
    painter.galley(
        Pos2::new(body_rect.min.x + pad_x, cursor_y),
        title_galley,
        pal.faint,
    );
    cursor_y += 18.0;

    // Status or instructions
    match &state.selection {
        CliSelection::Unselected => {
            let msg = "select an agent cli to start";
            let msg_galley = painter.layout_no_wrap(msg.to_owned(), font_id.clone(), pal.faint);
            painter.galley(
                Pos2::new(body_rect.min.x + pad_x, cursor_y),
                msg_galley,
                pal.faint,
            );
            cursor_y += 18.0;
        }
        CliSelection::Preset(preset) => {
            let status = format!("session: {} (ready)", preset.display_name());
            let status_galley = painter.layout_no_wrap(status, font_id.clone(), pal.pink);
            painter.galley(
                Pos2::new(body_rect.min.x + pad_x, cursor_y),
                status_galley,
                pal.pink,
            );
            cursor_y += 18.0;
        }
        CliSelection::Custom(cmd) => {
            let status = format!("session: custom [{}] (ready)", cmd);
            let status_galley = painter.layout_no_wrap(status, font_id.clone(), pal.pink);
            painter.galley(
                Pos2::new(body_rect.min.x + pad_x, cursor_y),
                status_galley,
                pal.pink,
            );
            cursor_y += 18.0;
        }
    }

    // Command prompt line
    let prompt_str = "> _";
    let prompt_galley = painter.layout_no_wrap(prompt_str.to_owned(), font_id, pal.text);
    painter.galley(
        Pos2::new(body_rect.min.x + pad_x, cursor_y),
        prompt_galley,
        pal.text,
    );
}
