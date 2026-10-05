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

/// Extracts selected text from terminal cell rows given normalized start and end coordinates.
pub fn extract_selected_text(
    cell_rows: &[Vec<super::ansi::Cell>],
    start: (usize, usize),
    end: (usize, usize),
) -> String {
    let (start, end) = if start <= end {
        (start, end)
    } else {
        (end, start)
    };
    let mut lines = Vec::new();
    let max_r = end.0.min(cell_rows.len().saturating_sub(1));
    for (r, row) in cell_rows.iter().enumerate().take(max_r + 1).skip(start.0) {
        let c_start = if r == start.0 {
            start.1.min(row.len())
        } else {
            0
        };
        let c_end = if r == end.0 {
            end.1.min(row.len())
        } else {
            row.len()
        };
        if c_start < c_end {
            let line: String = (c_start..c_end).map(|c| row[c].ch).collect();
            lines.push(line.trim_end().to_string());
        } else {
            lines.push(String::new());
        }
    }
    lines.join("\n")
}

/// Checks whether the cell at (r, c) falls inside the normalized selection range.
pub fn is_cell_selected(r: usize, c: usize, start: (usize, usize), end: (usize, usize)) -> bool {
    let (start, end) = if start <= end {
        (start, end)
    } else {
        (end, start)
    };
    if r < start.0 || r > end.0 {
        return false;
    }
    if start.0 == end.0 {
        return c >= start.1 && c < end.1;
    }
    if r == start.0 {
        c >= start.1
    } else if r == end.0 {
        c < end.1
    } else {
        true
    }
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
        let label = super::cli::default_custom_command();
        let galley = painter.layout_no_wrap(label.to_owned(), font_id.clone(), custom_color);
        painter.galley(text_pos, galley, custom_color);

        if is_custom_selected {
            let dot_x = custom_rect.max.x - 8.0;
            painter.circle_filled(Pos2::new(dot_x, mid_y), 3.0, pal.pink);
        }
    }
}

/// Copies text to clipboard across desktop and WASM web environments.
pub fn copy_to_clipboard(ctx: &egui::Context, text: &str) {
    ctx.copy_text(text.to_string());
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(win) = web_sys::window() {
            let _ = win.navigator().clipboard().write_text(text);
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
    terminal_ui.style_mut().interaction.selectable_labels = false;

    let scroll_id_salt = "prompt_terminal_scroll";
    let scroll_id = terminal_ui.id().with(egui::IdSalt::new(scroll_id_salt));
    let has_selection = state.selection_range().is_some() || state.selection_anchor().is_some();
    let scroll_state =
        egui::scroll_area::State::load(terminal_ui.ctx(), scroll_id).unwrap_or_default();
    let scroll_y = scroll_state.offset.y;

    egui::ScrollArea::vertical()
        .id_salt(scroll_id_salt)
        .stick_to_bottom(!has_selection)
        .auto_shrink([false, false])
        .show(&mut terminal_ui, |ui| {
            ui.add_space(2.0);
            match &state.selection {
                CliSelection::Unselected => {
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new("karakuri agent terminal (m9)")
                                .font(font_id.clone())
                                .color(pal.faint),
                        )
                        .selectable(false),
                    );
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new("select an agent cli above to start")
                                .font(font_id.clone())
                                .color(pal.faint),
                        )
                        .selectable(false),
                    );
                }
                CliSelection::Preset(_) | CliSelection::Custom(_) => {
                    if let Some(session) = state.active_session() {
                        let cell_rows = session.rows();

                        // Measure true glyph metrics including CJK fallbacks to prevent cumulative row drift
                        let (char_w, row_h) = ui.fonts_mut(|f| {
                            let char_w = f.glyph_width(&font_id, 'M').max(6.0);
                            let sample = f.layout_no_wrap(
                                "Mあ".to_string(),
                                font_id.clone(),
                                egui::Color32::WHITE,
                            );
                            let sample_h = sample.size().y.ceil();
                            let row_h = f.row_height(&font_id).max(sample_h).max(12.0);
                            (char_w, row_h)
                        });

                        // Dynamic PTY window size adjustment based on bay geometry
                        let cols = ((body_rect.width() - 8.0) / char_w).max(20.0) as u16;
                        let rows = ((body_rect.height() - 8.0) / row_h).max(5.0) as u16;
                        session.resize(rows, cols);

                        // Mouse drag selection handling across the terminal cell grid
                        let hover_pos = ui
                            .input(|i| i.pointer.hover_pos().or_else(|| i.pointer.interact_pos()));
                        if let Some(pos) = hover_pos {
                            let pressed = ui.input(|i| i.pointer.primary_pressed());
                            let down = ui.input(|i| i.pointer.primary_down());
                            let released = ui.input(|i| i.pointer.primary_released());
                            let secondary_clicked = ui.input(|i| i.pointer.secondary_clicked());

                            let is_dragging = down && state.selection_anchor().is_some();
                            let is_ending_drag = released && state.selection_anchor().is_some();
                            let is_in_bay = body_rect.contains(pos);

                            // Right-click (secondary click) copies selection or full terminal content
                            if secondary_clicked && is_in_bay {
                                if let Some((start, end)) = state.selection_range() {
                                    let text = extract_selected_text(&cell_rows, start, end);
                                    if !text.is_empty() {
                                        copy_to_clipboard(ui.ctx(), &text);
                                    }
                                } else if state.is_captured() {
                                    let max_r = cell_rows.len().saturating_sub(1);
                                    let max_c = cell_rows.last().map(|r| r.len()).unwrap_or(0);
                                    let text =
                                        extract_selected_text(&cell_rows, (0, 0), (max_r, max_c));
                                    if !text.is_empty() {
                                        copy_to_clipboard(ui.ctx(), &text);
                                    }
                                }
                            }

                            if is_in_bay || is_dragging || is_ending_drag {
                                ui.ctx().set_cursor_icon(egui::CursorIcon::Text);

                                // Auto-scroll terminal when dragging near or past bay vertical boundaries
                                let mut effective_scroll_y = scroll_y;
                                if is_dragging {
                                    let scroll_edge = 20.0;
                                    if pos.y < body_rect.min.y + scroll_edge {
                                        let dist = (body_rect.min.y + scroll_edge - pos.y).max(1.0);
                                        let speed = (dist * 0.8).clamp(8.0, 40.0);
                                        ui.scroll_with_delta(egui::Vec2::new(0.0, speed));
                                        effective_scroll_y = (effective_scroll_y - speed).max(0.0);
                                        ui.ctx().request_repaint();
                                    } else if pos.y > body_rect.max.y - scroll_edge {
                                        let dist =
                                            (pos.y - (body_rect.max.y - scroll_edge)).max(1.0);
                                        let speed = (dist * 0.8).clamp(8.0, 40.0);
                                        ui.scroll_with_delta(egui::Vec2::new(0.0, -speed));
                                        effective_scroll_y += speed;
                                        ui.ctx().request_repaint();
                                    }
                                }

                                let clamped_x =
                                    pos.x.clamp(body_rect.min.x + 4.0, body_rect.max.x - 4.0);
                                let rel_x = (clamped_x - (body_rect.min.x + 4.0)).max(0.0);
                                let clamped_y = pos.y.clamp(body_rect.min.y, body_rect.max.y);
                                let line_pitch = row_h + ui.spacing().item_spacing.y;
                                let top_padding = 2.0; // matching ui.add_space(2.0)
                                let rel_y = (clamped_y - body_rect.min.y + effective_scroll_y
                                    - top_padding)
                                    .max(0.0);
                                let col = (rel_x / char_w) as usize;
                                let row = ((rel_y / line_pitch) as usize)
                                    .min(cell_rows.len().saturating_sub(1));

                                if pressed && is_in_bay {
                                    state.set_selection_anchor(Some((row, col)));
                                    state.set_selection_range(None);
                                } else if is_dragging {
                                    if let Some(anchor) = state.selection_anchor() {
                                        if anchor != (row, col) {
                                            state.set_selection_range(Some((anchor, (row, col))));
                                        }
                                    }
                                } else if is_ending_drag {
                                    if let Some(anchor) = state.selection_anchor() {
                                        if anchor == (row, col) {
                                            state.clear_selection();
                                            state.set_captured(true);
                                        } else {
                                            state.set_selection_range(Some((anchor, (row, col))));
                                            state.set_selection_anchor(None);
                                        }
                                    }
                                }
                            }
                        }

                        // Direct interactive keyboard streaming to PTY stdin while in capture mode
                        if state.is_captured() {
                            super::input::handle_terminal_events(ui, state, &session, &cell_rows);
                        }

                        // Global copy shortcut when Prompt bay has an active text selection or terminal focus
                        let has_copy_event =
                            ui.input(|i| i.events.iter().any(|e| matches!(e, egui::Event::Copy)));
                        let copy_shortcut = ui.input(|i| {
                            let is_mac = i.modifiers.command || i.modifiers.mac_cmd;
                            let is_ctrl = i.modifiers.ctrl;
                            (is_mac || is_ctrl) && i.key_pressed(egui::Key::C)
                        });
                        if copy_shortcut || has_copy_event {
                            if let Some((start, end)) = state.selection_range() {
                                let text = extract_selected_text(&cell_rows, start, end);
                                if !text.is_empty() {
                                    copy_to_clipboard(ui.ctx(), &text);
                                }
                            } else if state.is_captured() {
                                // If captured but no range is selected, copy all terminal output
                                let max_r = cell_rows.len().saturating_sub(1);
                                let max_c = cell_rows.last().map(|r| r.len()).unwrap_or(0);
                                let text =
                                    extract_selected_text(&cell_rows, (0, 0), (max_r, max_c));
                                if !text.is_empty() {
                                    copy_to_clipboard(ui.ctx(), &text);
                                }
                            }
                        }

                        let cursor = session.cursor();
                        let cursor_visible = session.is_cursor_visible();
                        let is_captured = state.is_captured();
                        let active_preedit = state.preedit();

                        // Report precise IME cursor position to egui so OS displays candidate popup anchored to cursor
                        if is_captured {
                            let line_pitch = row_h + ui.spacing().item_spacing.y;
                            let preedit_len = active_preedit
                                .as_ref()
                                .map(|s| s.chars().count())
                                .unwrap_or(0);
                            let effective_col = cursor.1 + preedit_len;
                            let cursor_x = body_rect.min.x + 4.0 + (effective_col as f32 * char_w);
                            let cursor_y = (body_rect.min.y + 2.0 + (cursor.0 as f32 * line_pitch)
                                - scroll_y)
                                .max(body_rect.min.y);
                            let cursor_rect = Rect::from_min_size(
                                Pos2::new(cursor_x, cursor_y),
                                egui::Vec2::new(char_w, row_h),
                            );
                            ui.ctx().output_mut(|o| {
                                o.ime = Some(egui::output::IMEOutput {
                                    rect: cursor_rect,
                                    cursor_rect,
                                    purpose: egui::IMEPurpose::Terminal,
                                    should_interrupt_composition: false,
                                });
                            });
                        }

                        // Render each terminal row with formatted ANSI spans, visible cursor, and inline IME preedit
                        let total_rows = cell_rows.len().max(cursor.0 + 1);
                        for r_idx in 0..total_rows {
                            let empty_row = Vec::new();
                            let row = cell_rows.get(r_idx).unwrap_or(&empty_row);
                            let is_cursor_row = cursor_visible && r_idx == cursor.0;
                            let has_preedit = is_cursor_row
                                && active_preedit
                                    .as_ref()
                                    .map(|s| !s.is_empty())
                                    .unwrap_or(false);
                            let max_col = if is_cursor_row {
                                row.len().max(cursor.1 + 1)
                            } else {
                                row.len()
                            };

                            let mut job = LayoutJob::default();
                            let mut current_span = String::new();
                            let mut current_format: Option<TextFormat> = None;

                            for c_idx in 0..max_col {
                                // Inline IME preedit composition rendering at cursor position
                                if has_preedit && c_idx == cursor.1 {
                                    if let Some(ref preedit_str) = active_preedit {
                                        if let Some(active_fmt) = current_format.take() {
                                            if !current_span.is_empty() {
                                                job.append(&current_span, 0.0, active_fmt);
                                                current_span.clear();
                                            }
                                        }
                                        let preedit_fmt = TextFormat {
                                            font_id: font_id.clone(),
                                            color: pal.mint,
                                            background: tint(pal.mint, 40),
                                            underline: Stroke::new(1.5, pal.mint),
                                            ..Default::default()
                                        };
                                        job.append(preedit_str, 0.0, preedit_fmt);
                                    }
                                }

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
                                    format.color = pal.dim;
                                }

                                // In light themes (e.g. DAY), bright gray or white text is unreadable against white backgrounds.
                                // Adapt light text colors to pal.text for high contrast.
                                let is_light_theme = pal.panel.r() > 128
                                    && pal.panel.g() > 128
                                    && pal.panel.b() > 128;
                                if is_light_theme && format.background == egui::Color32::TRANSPARENT
                                {
                                    let lum = 0.299 * format.color.r() as f32
                                        + 0.587 * format.color.g() as f32
                                        + 0.114 * format.color.b() as f32;
                                    if lum > 190.0 {
                                        format.color = pal.text;
                                    }
                                }

                                if is_cursor_cell && !has_preedit {
                                    if is_captured {
                                        format.background = pal.mint;
                                        format.color = pal.panel;
                                    } else {
                                        format.background = tint(pal.dim, 90);
                                        format.color = pal.text;
                                    }
                                }

                                // Apply text selection highlight
                                if state.selection_range().is_some_and(|(start, end)| {
                                    is_cell_selected(r_idx, c_idx, start, end)
                                }) {
                                    format.background = pal.mint;
                                    format.color = pal.panel;
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
                                job.append(
                                    " ",
                                    0.0,
                                    TextFormat {
                                        font_id: font_id.clone(),
                                        ..Default::default()
                                    },
                                );
                            }
                            ui.add(egui::Label::new(job).selectable(false));
                        }
                    }
                }
            }
            ui.add_space(2.0);
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::prompt::ansi::Cell;

    #[test]
    fn test_extract_selected_text_single_and_multi_line() {
        let rows = vec![
            "Hello, world!"
                .chars()
                .map(|ch| Cell {
                    ch,
                    style: Default::default(),
                })
                .collect::<Vec<_>>(),
            "Karakuri Console"
                .chars()
                .map(|ch| Cell {
                    ch,
                    style: Default::default(),
                })
                .collect::<Vec<_>>(),
            "Third line text"
                .chars()
                .map(|ch| Cell {
                    ch,
                    style: Default::default(),
                })
                .collect::<Vec<_>>(),
        ];

        // Single line selection
        let sel = extract_selected_text(&rows, (0, 0), (0, 5));
        assert_eq!(sel, "Hello");

        // Multi-line selection
        let sel_multi = extract_selected_text(&rows, (0, 7), (1, 8));
        assert_eq!(sel_multi, "world!\nKarakuri");

        // Reversed coordinates selection (drag backwards)
        let sel_rev = extract_selected_text(&rows, (1, 8), (0, 7));
        assert_eq!(sel_rev, "world!\nKarakuri");
    }

    #[test]
    fn test_is_cell_selected() {
        // Single row selection from col 2 to 5
        assert!(!is_cell_selected(0, 1, (0, 2), (0, 5)));
        assert!(is_cell_selected(0, 2, (0, 2), (0, 5)));
        assert!(is_cell_selected(0, 4, (0, 2), (0, 5)));
        assert!(!is_cell_selected(0, 5, (0, 2), (0, 5)));

        // Multi row selection (row 1, col 3 to row 3, col 2)
        assert!(!is_cell_selected(0, 5, (1, 3), (3, 2)));
        assert!(is_cell_selected(1, 3, (1, 3), (3, 2)));
        assert!(is_cell_selected(1, 10, (1, 3), (3, 2)));
        assert!(is_cell_selected(2, 0, (1, 3), (3, 2)));
        assert!(is_cell_selected(3, 1, (1, 3), (3, 2)));
        assert!(!is_cell_selected(3, 2, (1, 3), (3, 2)));
        assert!(!is_cell_selected(4, 0, (1, 3), (3, 2)));
    }

    #[test]
    fn test_preedit_state_lifecycle() {
        let state = PromptState::default();
        assert_eq!(state.preedit(), None);

        // Setting active composition text
        state.set_preedit(Some("とうきょう".to_string()));
        assert_eq!(state.preedit(), Some("とうきょう".to_string()));

        // Updating to converted kanji
        state.set_preedit(Some("東京".to_string()));
        assert_eq!(state.preedit(), Some("東京".to_string()));

        // Clearing on commit
        state.clear_preedit();
        assert_eq!(state.preedit(), None);
    }
}
