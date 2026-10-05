//! Interactive TUI menu selection for discovered local LLM models.

use super::config::HarnessConfig;
use super::probe::DiscoveredModel;

#[derive(Debug, Clone)]
pub struct ModelMenu {
    pub items: Vec<DiscoveredModel>,
    pub selected_index: usize,
    pub is_active: bool,
    lines_rendered: usize,
}

pub enum MenuAction {
    None,
    Redraw,
    Selected(DiscoveredModel),
    Cancelled,
}

impl ModelMenu {
    pub fn new(items: Vec<DiscoveredModel>, current_config: &HarnessConfig) -> Self {
        // Try to pre-select currently active model if present in items
        let selected_index = items
            .iter()
            .position(|m| {
                m.model_id == current_config.model && m.base_url == current_config.endpoint
            })
            .unwrap_or(0);

        Self {
            items,
            selected_index,
            is_active: true,
            lines_rendered: 0,
        }
    }

    /// Renders the initial menu display string.
    pub fn render_initial(&mut self, current_config: &HarnessConfig) -> String {
        let mut out = String::new();
        out.push_str("\r\n\x1b[1;36mSelect a model:\x1b[0m\r\n");
        let (lines, count) = self.render_items(current_config);
        out.push_str(&lines);
        out.push_str(
            "\r\n\x1b[2m(Use ↑/↓ or 1-9 to navigate, Enter to select, Esc to cancel)\x1b[0m\r\n",
        );
        self.lines_rendered = count + 3;
        out
    }

    /// Renders an in-place update when selection changes using ANSI cursor movements.
    pub fn render_update(&mut self, current_config: &HarnessConfig) -> String {
        let mut out = String::new();
        // Move cursor up to the first item row:
        // lines_rendered = count + 3 (header + items + blank/help)
        // From after help row, move up (count + 2) rows to header bottom
        let rows_up = self.items.len() + 2;
        out.push_str(&format!("\x1b[{}A\r", rows_up));

        let (lines, _) = self.render_items(current_config);
        out.push_str(&lines);

        // Move back down to the line after help text
        out.push_str(
            "\r\n\x1b[2m(Use ↑/↓ or 1-9 to navigate, Enter to select, Esc to cancel)\x1b[0m\r\n",
        );
        out
    }

    fn render_items(&self, current_config: &HarnessConfig) -> (String, usize) {
        let mut out = String::new();
        let count = self.items.len();

        for (i, item) in self.items.iter().enumerate() {
            let is_sel = i == self.selected_index;
            let is_cur =
                item.model_id == current_config.model && item.base_url == current_config.endpoint;

            out.push_str("\x1b[2K\r"); // clear line
            if is_sel {
                out.push_str("  \x1b[1;32m❯ ");
                if i < 9 {
                    out.push_str(&format!("{}. ", i + 1));
                } else {
                    out.push_str("   ");
                }
                out.push_str(&format!(
                    "\x1b[1;37m{:<22}\x1b[0m \x1b[2m({} · {})\x1b[0m",
                    item.model_id, item.server_name, item.base_url
                ));
            } else {
                out.push_str("    ");
                if i < 9 {
                    out.push_str(&format!("\x1b[2m{}.\x1b[0m ", i + 1));
                } else {
                    out.push_str("   ");
                }
                out.push_str(&format!(
                    "\x1b[37m{:<22}\x1b[0m \x1b[2m({} · {})\x1b[0m",
                    item.model_id, item.server_name, item.base_url
                ));
            }

            if is_cur {
                out.push_str(" \x1b[33m[active]\x1b[0m");
            }
            out.push_str("\r\n");
        }

        (out, count)
    }

    /// Handles keyboard input events while menu is active.
    pub fn handle_key(&mut self, bytes: &[u8]) -> MenuAction {
        if self.items.is_empty() {
            return MenuAction::Cancelled;
        }

        // Up arrow: \x1b[A
        if bytes == b"\x1b[A" {
            if self.selected_index > 0 {
                self.selected_index -= 1;
            } else {
                self.selected_index = self.items.len() - 1;
            }
            return MenuAction::Redraw;
        }

        // Down arrow: \x1b[B
        if bytes == b"\x1b[B" {
            if self.selected_index + 1 < self.items.len() {
                self.selected_index += 1;
            } else {
                self.selected_index = 0;
            }
            return MenuAction::Redraw;
        }

        // Direct number selection: 1..=9
        if bytes.len() == 1 {
            let b = bytes[0];
            if (b'1'..=b'9').contains(&b) {
                let idx = (b - b'1') as usize;
                if idx < self.items.len() {
                    self.selected_index = idx;
                    return MenuAction::Selected(self.items[self.selected_index].clone());
                }
            }
        }

        // Enter: \r or \n
        if bytes == b"\r" || bytes == b"\n" {
            return MenuAction::Selected(self.items[self.selected_index].clone());
        }

        // Esc or Ctrl+C
        if bytes == b"\x1b" || bytes == b"\x03" {
            return MenuAction::Cancelled;
        }

        MenuAction::None
    }
}
