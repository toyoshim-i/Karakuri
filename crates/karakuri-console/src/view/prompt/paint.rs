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

/// Terminal and input font size in the Prompt bay.
pub const PROMPT_FONT_SIZE: f32 = 10.0;

/// Paints the internal body of the Prompt bay (terminal area with scrollback and prompt bar).
pub fn prompt_into(ui: &mut Ui, pal: &Palette, bay_rect: Rect, state: &PromptState) {
    let head = head_box(bay_rect);
    let body_top = head.max.y;
    if body_top >= bay_rect.max.y {
        return;
    }

    let body_rect = Rect::from_min_max(Pos2::new(bay_rect.min.x, body_top), bay_rect.max);
    let input_h = 24.0;
    let sep_y = (body_rect.max.y - input_h).max(body_rect.min.y);

    let output_rect = Rect::from_min_max(body_rect.min, Pos2::new(body_rect.max.x, sep_y));
    let input_rect = Rect::from_min_max(Pos2::new(body_rect.min.x, sep_y), body_rect.max);

    let font_id = FontId::new(PROMPT_FONT_SIZE, FontFamily::Monospace);

    // 1. Output scrollback area
    let mut output_ui = ui.new_child(egui::UiBuilder::new().max_rect(output_rect));
    output_ui.spacing_mut().item_spacing.y = 2.0;
    egui::ScrollArea::vertical()
        .stick_to_bottom(true)
        .auto_shrink([false, false])
        .show(&mut output_ui, |ui| {
            ui.add_space(4.0);
            match &state.selection {
                CliSelection::Unselected => {
                    ui.label(
                        egui::RichText::new("karakuri agent terminal (m9)")
                            .font(font_id.clone())
                            .color(pal.faint),
                    );
                    ui.label(
                        egui::RichText::new("select an agent cli above to start")
                            .font(font_id.clone())
                            .color(pal.faint),
                    );
                }
                CliSelection::Preset(preset) => {
                    if let Some(session) = state.active_session() {
                        let lines = session.lines();
                        if lines.is_empty() {
                            let label = if session.is_running() {
                                format!("session: {} (running)", preset.display_name())
                            } else {
                                format!("session: {} (ready)", preset.display_name())
                            };
                            ui.label(
                                egui::RichText::new(label)
                                    .font(font_id.clone())
                                    .color(pal.pink),
                            );
                        } else {
                            for line in lines {
                                ui.label(
                                    egui::RichText::new(line)
                                        .font(font_id.clone())
                                        .color(pal.text),
                                );
                            }
                        }
                    }
                }
                CliSelection::Custom(cmd) => {
                    if let Some(session) = state.active_session() {
                        let lines = session.lines();
                        if lines.is_empty() {
                            ui.label(
                                egui::RichText::new(format!("session: custom [{}] (ready)", cmd))
                                    .font(font_id.clone())
                                    .color(pal.pink),
                            );
                        } else {
                            for line in lines {
                                ui.label(
                                    egui::RichText::new(line)
                                        .font(font_id.clone())
                                        .color(pal.text),
                                );
                            }
                        }
                    }
                }
            }
            ui.add_space(4.0);
        });

    // 2. Separator line above prompt input
    ui.painter().line_segment(
        [
            Pos2::new(input_rect.min.x, input_rect.min.y),
            Pos2::new(input_rect.max.x, input_rect.min.y),
        ],
        Stroke::new(size::HAIRLINE, pal.hair),
    );

    // 3. Prompt input bar
    let mut input_ui = ui.new_child(egui::UiBuilder::new().max_rect(input_rect));
    input_ui.horizontal_centered(|ui| {
        ui.add_space(6.0);
        ui.label(
            egui::RichText::new(">")
                .font(font_id.clone())
                .color(pal.pink),
        );

        let mut buf_guard = state.input_buffer.lock().ok();
        if let Some(ref mut buf) = buf_guard {
            let edit = egui::TextEdit::singleline(&mut **buf)
                .font(font_id)
                .text_color(pal.text)
                .hint_text("Ask agent...")
                .frame(egui::Frame::NONE)
                .desired_width(f32::INFINITY);

            let response = ui.add(edit);

            let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
            if enter_pressed && response.has_focus() {
                let text = std::mem::take(&mut **buf);
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    if let Some(session) = state.active_session() {
                        let _ = session.send_line(trimmed);
                    }
                }
                response.request_focus();
            }
        }
    });
}
