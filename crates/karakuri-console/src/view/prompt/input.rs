//! Keyboard input and terminal shortcut event handling for the Prompt bay.

use egui::Ui;

use super::ansi::Cell;
use super::paint::{copy_to_clipboard, extract_selected_text};
use super::session::TerminalSession;
use super::state::PromptState;

/// Processes keyboard and clipboard events for an active, captured terminal session.
pub fn handle_terminal_events(
    ui: &Ui,
    state: &PromptState,
    session: &TerminalSession,
    cell_rows: &[Vec<Cell>],
) {
    let events = ui.input(|i| i.events.clone());
    for ev in events {
        match ev {
            egui::Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } => {
                let is_mac = modifiers.command || modifiers.mac_cmd;
                let is_ctrl = modifiers.ctrl;
                let is_alt = modifiers.alt;

                // ⌘C on Mac or Ctrl+C with active text selection: copy selected text
                let is_copy = (is_mac && key == egui::Key::C)
                    || (is_ctrl && key == egui::Key::C && state.selection_range().is_some());

                if is_copy {
                    if let Some((start, end)) = state.selection_range() {
                        let text = extract_selected_text(cell_rows, start, end);
                        if !text.is_empty() {
                            copy_to_clipboard(ui.ctx(), &text);
                        }
                    }
                    continue;
                }

                // ⌘A on Mac or Ctrl+A: select all terminal text
                let is_select_all =
                    (is_mac && key == egui::Key::A) || (is_ctrl && key == egui::Key::A);
                if is_select_all {
                    let max_r = cell_rows.len().saturating_sub(1);
                    let max_c = cell_rows.last().map(|r| r.len()).unwrap_or(0);
                    state.set_selection_range(Some(((0, 0), (max_r, max_c))));
                    continue;
                }

                // OS-tailored Text Editing Shortcuts:
                // 1. Delete line to start: ⌘Backspace on Mac, Ctrl+U on Mac/PC
                if (is_mac && key == egui::Key::Backspace) || (is_ctrl && key == egui::Key::U) {
                    let _ = session.send_bytes(b"\x15");
                    continue;
                }

                // 2. Delete word before cursor: ⌥Backspace on Mac, Ctrl+Backspace on PC, Ctrl+W
                if (is_alt && key == egui::Key::Backspace)
                    || (!is_mac && is_ctrl && key == egui::Key::Backspace)
                    || (is_ctrl && key == egui::Key::W)
                {
                    let _ = session.send_bytes(b"\x17");
                    continue;
                }

                // 3. Delete word after cursor: ⌥Delete on Mac, Ctrl+Delete on PC
                if (is_alt && key == egui::Key::Delete)
                    || (!is_mac && is_ctrl && key == egui::Key::Delete)
                {
                    let _ = session.send_bytes(b"\x1bd");
                    continue;
                }

                // 4. Delete line to end: ⌘K on Mac, Ctrl+K on Mac/PC
                if (is_mac && key == egui::Key::K) || (is_ctrl && key == egui::Key::K) {
                    let _ = session.send_bytes(b"\x0b");
                    continue;
                }

                // 5. Jump to start of line: ⌘Left on Mac, Home, Ctrl+A
                if (is_mac && key == egui::Key::ArrowLeft)
                    || (key == egui::Key::Home)
                    || (is_ctrl && key == egui::Key::A)
                {
                    let _ = session.send_bytes(b"\x1b[H");
                    continue;
                }

                // 6. Jump to end of line: ⌘Right on Mac, End, Ctrl+E
                if (is_mac && key == egui::Key::ArrowRight)
                    || (key == egui::Key::End)
                    || (is_ctrl && key == egui::Key::E)
                {
                    let _ = session.send_bytes(b"\x1b[F");
                    continue;
                }

                // 7. Jump word backward: ⌥Left on Mac, Ctrl+Left on PC
                if (is_alt && key == egui::Key::ArrowLeft)
                    || (!is_mac && is_ctrl && key == egui::Key::ArrowLeft)
                {
                    let _ = session.send_bytes(b"\x1bb");
                    continue;
                }

                // 8. Jump word forward: ⌥Right on Mac, Ctrl+Right on PC
                if (is_alt && key == egui::Key::ArrowRight)
                    || (!is_mac && is_ctrl && key == egui::Key::ArrowRight)
                {
                    let _ = session.send_bytes(b"\x1bf");
                    continue;
                }

                // 9. Undo: ⌘Z on Mac, Ctrl+Z on PC
                if (is_mac && key == egui::Key::Z) || (is_ctrl && key == egui::Key::Z) {
                    let _ = session.send_bytes(b"\x1f");
                    continue;
                }

                // 10. Cut: ⌘X on Mac, Ctrl+X on PC
                if (is_mac && key == egui::Key::X) || (is_ctrl && key == egui::Key::X) {
                    if let Some((start, end)) = state.selection_range() {
                        let text = extract_selected_text(cell_rows, start, end);
                        if !text.is_empty() {
                            copy_to_clipboard(ui.ctx(), &text);
                        }
                        state.clear_selection();
                    }
                    continue;
                }

                if is_ctrl {
                    match key {
                        egui::Key::C => {
                            let _ = session.send_bytes(b"\x03");
                        }
                        egui::Key::D => {
                            let _ = session.send_bytes(b"\x04");
                        }
                        egui::Key::L => {
                            let _ = session.send_bytes(b"\x0c");
                        }
                        egui::Key::R => {
                            let _ = session.send_bytes(b"\x12");
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
                        egui::Key::Tab => {
                            let _ = session.send_bytes(b"\t");
                        }
                        _ => {}
                    }
                }
            }
            egui::Event::Text(text) => {
                let _ = session.send_bytes(text.as_bytes());
            }
            egui::Event::Ime(egui::ImeEvent::Commit(text)) => {
                let _ = session.send_bytes(text.as_bytes());
            }
            egui::Event::Paste(text) => {
                let _ = session.send_bytes(text.as_bytes());
            }
            _ => {}
        }
    }
}
