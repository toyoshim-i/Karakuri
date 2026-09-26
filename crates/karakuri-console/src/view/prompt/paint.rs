//! Rendering for the Prompt bay terminal body, header pill, and dropdown menu.

use egui::epaint::text::FontId;
use egui::{CornerRadius, FontFamily, Pos2, Rect, Stroke, Ui};
use karakuri_layout::Layout;

use super::cli::{CliPreset, CliSelection};
use super::head::{
    prompt_item_rect, prompt_menu_rect, prompt_pill, MENU_COL_GAP, MENU_COL_W, MENU_PAD_X,
    MENU_PAD_Y,
};
use super::state::PromptState;
use crate::room::{size, Palette};
use crate::view::to_egui;
use crate::view::widgets::card::popup_card;
use crate::view::widgets::fader::tint;
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

    // Subtle vertical separator rule between column 0 and column 1
    let sep_x = menu.min.x + MENU_PAD_X + MENU_COL_W + MENU_COL_GAP * 0.5;
    painter.line_segment(
        [
            Pos2::new(sep_x, menu.min.y + MENU_PAD_Y),
            Pos2::new(sep_x, menu.max.y - MENU_PAD_Y),
        ],
        Stroke::new(size::HAIRLINE, pal.hair),
    );

    // Track pointer hover position for interactive item highlighting
    let hover_pos = ui.input(|i| i.pointer.hover_pos());

    // 1. Presets
    for (index, preset) in CliPreset::ALL.iter().enumerate() {
        let Some(item_rect) = prompt_item_rect(menu, index) else {
            continue;
        };

        let is_selected = matches!(&state.selection, CliSelection::Preset(p) if p == preset);
        let available = preset.is_available();
        let is_hovered = hover_pos.is_some_and(|pos| item_rect.contains(pos));
        let mid_y = item_rect.center().y;

        // Hover highlight wash behind item
        if is_hovered && available {
            painter.rect_filled(item_rect, CornerRadius::same(3), tint(pal.pink, 22));
        } else if is_hovered && !available {
            painter.rect_filled(item_rect, CornerRadius::same(3), tint(pal.faint, 12));
        }

        let text_color = if available {
            if is_selected || is_hovered {
                pal.pink
            } else {
                pal.text
            }
        } else {
            pal.faint
        };

        let label = preset.display_name();
        let text_pos = Pos2::new(item_rect.min.x + 6.0, mid_y - size::BASE * 0.5);
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
            let dot_x = item_rect.max.x - 8.0;
            painter.circle_filled(Pos2::new(dot_x, mid_y), 3.0, pal.pink);
        }
    }

    // 2. Custom command option
    let custom_index = CliPreset::ALL.len();
    if let Some(custom_rect) = prompt_item_rect(menu, custom_index) {
        let is_custom_selected = matches!(&state.selection, CliSelection::Custom(_));
        let is_custom_hovered = hover_pos.is_some_and(|pos| custom_rect.contains(pos));
        let mid_y = custom_rect.center().y;

        if is_custom_hovered {
            painter.rect_filled(custom_rect, CornerRadius::same(3), tint(pal.pink, 22));
        }

        let custom_color = if is_custom_selected || is_custom_hovered {
            pal.pink
        } else {
            pal.text
        };

        let text_pos = Pos2::new(custom_rect.min.x + 6.0, mid_y - size::BASE * 0.5);
        let galley = painter.layout_no_wrap("custom...".to_owned(), font_id.clone(), custom_color);
        painter.galley(text_pos, galley, custom_color);

        if is_custom_selected {
            let dot_x = custom_rect.max.x - 8.0;
            painter.circle_filled(Pos2::new(dot_x, mid_y), 3.0, pal.pink);
        }
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
