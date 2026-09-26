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
use crate::view::widgets::glyph::{chevron_down, CHEVRON_H, CHEVRON_W};
use crate::view::widgets::head::head_box;
use crate::view::widgets::pills::pill_into;

/// Paints the Prompt bay header selector pill with vector chevron mark.
pub fn prompt_head_into(ui: &Ui, pal: &Palette, bay_rect: Rect, state: &PromptState) {
    let pill = prompt_pill(ui.ctx(), bay_rect, &state.selection);
    let label = state.selection.pill_label();
    let armed = state.menu_open || !state.selection.is_unselected();
    pill_into(ui, pal, pill, &label, armed);

    let chevron_rect = Rect::from_center_size(
        Pos2::new(
            pill.max.x - size::PILL_PAD_X - CHEVRON_W * 0.5,
            pill.center().y,
        ),
        egui::vec2(CHEVRON_W, CHEVRON_H),
    );
    let chevron_color = if armed { pal.mint } else { pal.dim };
    chevron_down(ui.painter(), chevron_rect, chevron_color);
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

/// Paints the internal body of the Prompt bay with interactive cursor-anchored terminal.
pub fn prompt_into(
    ui: &mut Ui,
    pal: &Palette,
    bay_rect: Rect,
    state: &PromptState,
    _is_focused: bool,
) {
    use egui::epaint::text::TextFormat;
    use egui::text::LayoutJob;

    let head = head_box(bay_rect);
    let body_top = head.max.y;
    if body_top >= bay_rect.max.y {
        return;
    }

    let body_rect = Rect::from_min_max(Pos2::new(bay_rect.min.x, body_top), bay_rect.max);
    let font_id = FontId::new(PROMPT_FONT_SIZE, FontFamily::Monospace);

    let mut terminal_ui = ui.new_child(egui::UiBuilder::new().max_rect(body_rect));
    terminal_ui.spacing_mut().item_spacing.y = 1.0;

    egui::ScrollArea::vertical()
        .stick_to_bottom(true)
        .auto_shrink([false, false])
        .show(&mut terminal_ui, |ui| {
            ui.add_space(2.0);
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
                CliSelection::Preset(_) | CliSelection::Custom(_) => {
                    if let Some(session) = state.active_session() {
                        // Dynamic PTY window size adjustment based on bay geometry
                        let cols = ((body_rect.width() - 8.0) / 6.0).max(20.0) as u16;
                        let rows = ((body_rect.height() - 8.0) / 12.0).max(5.0) as u16;
                        session.resize(rows, cols);

                        // Direct interactive keyboard streaming to PTY stdin while in capture mode
                        if state.is_captured() {
                            let events = ui.input(|i| i.events.clone());
                            for ev in events {
                                match ev {
                                    egui::Event::Key {
                                        key,
                                        pressed: true,
                                        modifiers,
                                        ..
                                    } => {
                                        if modifiers.ctrl {
                                            match key {
                                                egui::Key::C => {
                                                    let _ = session.send_bytes(b"\x03");
                                                }
                                                egui::Key::D => {
                                                    let _ = session.send_bytes(b"\x04");
                                                }
                                                egui::Key::Z => {
                                                    let _ = session.send_bytes(b"\x1a");
                                                }
                                                egui::Key::L => {
                                                    let _ = session.send_bytes(b"\x0c");
                                                }
                                                egui::Key::U => {
                                                    let _ = session.send_bytes(b"\x15");
                                                }
                                                egui::Key::W => {
                                                    let _ = session.send_bytes(b"\x17");
                                                }
                                                egui::Key::A => {
                                                    let _ = session.send_bytes(b"\x01");
                                                }
                                                egui::Key::E => {
                                                    let _ = session.send_bytes(b"\x05");
                                                }
                                                egui::Key::R => {
                                                    let _ = session.send_bytes(b"\x12");
                                                }
                                                egui::Key::K => {
                                                    let _ = session.send_bytes(b"\x0b");
                                                }
                                                _ => {}
                                            }
                                        } else {
                                            match key {
                                                egui::Key::Enter => {
                                                    let _ = session.send_bytes(b"\r");
                                                }
                                                egui::Key::Backspace => {
                                                    let _ = session.send_bytes(b"\x7f");
                                                }
                                                egui::Key::Escape => {
                                                    let _ = session.send_bytes(b"\x1b");
                                                }
                                                egui::Key::ArrowUp => {
                                                    let _ = session.send_bytes(b"\x1b[A");
                                                }
                                                egui::Key::ArrowDown => {
                                                    let _ = session.send_bytes(b"\x1b[B");
                                                }
                                                egui::Key::ArrowRight => {
                                                    let _ = session.send_bytes(b"\x1b[C");
                                                }
                                                egui::Key::ArrowLeft => {
                                                    let _ = session.send_bytes(b"\x1b[D");
                                                }
                                                egui::Key::Home => {
                                                    let _ = session.send_bytes(b"\x1b[H");
                                                }
                                                egui::Key::End => {
                                                    let _ = session.send_bytes(b"\x1b[F");
                                                }
                                                egui::Key::PageUp => {
                                                    let _ = session.send_bytes(b"\x1b[5~");
                                                }
                                                egui::Key::PageDown => {
                                                    let _ = session.send_bytes(b"\x1b[6~");
                                                }
                                                egui::Key::Delete => {
                                                    let _ = session.send_bytes(b"\x1b[3~");
                                                }
                                                _ => {}
                                            }
                                        }
                                    }
                                    egui::Event::Text(text) => {
                                        let _ = session.send_bytes(text.as_bytes());
                                    }
                                    _ => {}
                                }
                            }
                        }

                        // Activate capture mode when console body is clicked
                        let pointer_clicked = ui.input(|i| i.pointer.primary_clicked());
                        if pointer_clicked {
                            if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
                                if body_rect.contains(pos) {
                                    state.set_captured(true);
                                }
                            }
                        }

                        let cell_rows = session.rows();
                        let cursor = session.cursor();
                        let cursor_visible = session.is_cursor_visible();
                        let is_captured = state.is_captured();

                        // Render each terminal row with formatted ANSI spans and visible cursor
                        let total_rows = cell_rows.len().max(cursor.0 + 1);
                        for r_idx in 0..total_rows {
                            let empty_row = Vec::new();
                            let row = cell_rows.get(r_idx).unwrap_or(&empty_row);
                            let is_cursor_row = cursor_visible && r_idx == cursor.0;
                            let max_col = if is_cursor_row {
                                row.len().max(cursor.1 + 1)
                            } else {
                                row.len()
                            };

                            let mut job = LayoutJob::default();
                            let mut current_span = String::new();
                            let mut current_format: Option<TextFormat> = None;

                            for c_idx in 0..max_col {
                                let (ch, cell_style) = if c_idx < row.len() {
                                    (row[c_idx].ch, row[c_idx].style)
                                } else {
                                    (' ', Default::default())
                                };

                                let is_cursor_cell = is_cursor_row && c_idx == cursor.1;

                                let mut format = TextFormat {
                                    font_id: font_id.clone(),
                                    color: cell_style.fg.unwrap_or(pal.text),
                                    background: cell_style.bg.unwrap_or(egui::Color32::TRANSPARENT),
                                    italics: cell_style.italic,
                                    underline: if cell_style.underline {
                                        Stroke::new(1.0, cell_style.fg.unwrap_or(pal.text))
                                    } else {
                                        Stroke::NONE
                                    },
                                    ..Default::default()
                                };

                                if cell_style.invert {
                                    std::mem::swap(&mut format.color, &mut format.background);
                                    if format.color == egui::Color32::TRANSPARENT {
                                        format.color = pal.panel;
                                    }
                                }
                                if cell_style.dim {
                                    format.color = tint(format.color, 140);
                                }

                                if is_cursor_cell {
                                    if is_captured {
                                        format.background = pal.mint;
                                        format.color = pal.panel;
                                    } else {
                                        format.background = tint(pal.dim, 90);
                                        format.color = pal.text;
                                    }
                                }

                                if let Some(ref active_fmt) = current_format {
                                    if active_fmt == &format {
                                        current_span.push(ch);
                                    } else {
                                        job.append(&current_span, 0.0, active_fmt.clone());
                                        current_span.clear();
                                        current_span.push(ch);
                                        current_format = Some(format);
                                    }
                                } else {
                                    current_span.push(ch);
                                    current_format = Some(format);
                                }
                            }

                            if let Some(active_fmt) = current_format {
                                if !current_span.is_empty() {
                                    job.append(&current_span, 0.0, active_fmt);
                                }
                            }

                            if job.text.is_empty() {
                                ui.label(egui::RichText::new(" ").font(font_id.clone()));
                            } else {
                                ui.label(job);
                            }
                        }
                    }
                }
            }
            ui.add_space(2.0);
        });
}
