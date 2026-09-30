//! Transport row Model Context Protocol (MCP) server configuration pill and dropdown card.

use egui::{vec2, Color32, CornerRadius, FontFamily, FontId, Pos2, Rect, Stroke, StrokeKind, Ui};
use karakuri_layout::Point;

use super::*;
use crate::room::{size, Palette, ThemeMode};
use crate::view::widgets::card::{card_row_text, popup_card};
use crate::view::widgets::field::CARET;
use crate::view::widgets::glyph::{chevron_down, CHEVRON_H, CHEVRON_W};

/// Default address:port used when none is specified.
pub const DEFAULT_MCP_ADDR: &str = "127.0.0.1:4040";

/// Number of interactive rows in the MCP configuration dropdown menu.
pub const MCP_MENU_ROWS: usize = 5;

/// Quick preset address:port options in the menu.
pub const PRESET_ADDRS: &[&str] = &["127.0.0.1:4040", "127.0.0.1:8000", "127.0.0.1:0"];

/// Active dropdown menu state for the MCP server pill.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum McpServerMenu {
    /// Card is closed.
    Shut,
    /// Card is open displaying server options and presets.
    Open,
    /// Card is open with inline text editing for address:port.
    Editing(String),
}

/// Transport row MCP server runtime state and configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpServer {
    /// Configured or bound address:port string (e.g. "127.0.0.1:4040").
    pub addr: String,
    /// Whether the server is currently listening.
    pub running: bool,
    /// Popover menu state.
    pub menu: McpServerMenu,
}

impl McpServer {
    /// Creates a new MCP server configuration defaulted to stopped at 127.0.0.1:4040.
    pub fn new() -> Self {
        Self {
            addr: DEFAULT_MCP_ADDR.to_string(),
            running: false,
            menu: McpServerMenu::Shut,
        }
    }

    /// Whether the dropdown card is open.
    pub fn open(&self) -> bool {
        !matches!(self.menu, McpServerMenu::Shut)
    }

    /// Opens the dropdown card.
    pub fn opened(&mut self) {
        self.menu = McpServerMenu::Open;
    }

    /// Closes the dropdown card.
    pub fn shut(&mut self) {
        self.menu = McpServerMenu::Shut;
    }

    /// Returns the active text editing buffer if currently editing address:port.
    pub fn editing(&self) -> Option<&str> {
        match &self.menu {
            McpServerMenu::Editing(text) => Some(text),
            _ => None,
        }
    }

    /// Enters inline address:port editing mode initialized with the current address.
    pub fn start_editing(&mut self) {
        self.menu = McpServerMenu::Editing(self.addr.clone());
    }

    /// Types a character into the inline editing buffer.
    pub fn typed(&mut self, c: char) -> bool {
        match &mut self.menu {
            McpServerMenu::Editing(buf) if !c.is_control() => {
                buf.push(c);
                true
            }
            _ => false,
        }
    }

    /// Rubs out the last character in the inline editing buffer.
    pub fn rubbed_out(&mut self) -> bool {
        match &mut self.menu {
            McpServerMenu::Editing(buf) => buf.pop().is_some(),
            _ => false,
        }
    }

    /// Returns the number of visible rows in the open menu.
    pub fn rows(&self) -> usize {
        match self.menu {
            McpServerMenu::Open => MCP_MENU_ROWS,
            _ => 0,
        }
    }
}

impl Default for McpServer {
    fn default() -> Self {
        Self::new()
    }
}

/// User interaction outcome from pressing inside MCP pill or menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum McpAsk {
    /// Toggle the dropdown menu open/closed.
    ToggleMenu,
    /// Toggle server start/stop.
    ToggleRunning,
    /// Start inline editing of address:port.
    EditAddr,
    /// Pick a quick preset address:port.
    SetAddr(String),
    /// Dismiss / shut the dropdown menu.
    Shut,
}

/// Layout and geometry for the MCP server pill.
#[derive(Debug, Clone, PartialEq)]
pub struct McpServerPill {
    pub pill: Rect,
    pub text: Rect,
    pub chevron: Rect,
    pub menu: Option<Rect>,
    pub rows: usize,
}

impl McpServerPill {
    /// Whether `p` is on the pill itself.
    pub fn hit(&self, p: Point) -> bool {
        self.pill.contains(Pos2::new(p.x, p.y))
    }

    /// Whether `p` is anywhere this control owns (the pill or open card).
    pub fn owns(&self, p: Point) -> bool {
        let at = Pos2::new(p.x, p.y);
        self.pill.contains(at) || self.menu.is_some_and(|m| m.contains(at))
    }

    /// Returns the bounding box for the row at `index` in the dropdown card.
    pub fn row(&self, index: usize) -> Option<Rect> {
        let menu = self.menu?;
        if index >= self.rows {
            return None;
        }
        let top = menu.min.y
            + size::LIB_LIST_PAD
            + size::LIB_ROW_H * index as f32
            + if index > 1 { size::HAIRLINE } else { 0.0 };
        Some(Rect::from_min_size(
            Pos2::new(menu.min.x + size::LIB_LIST_PAD, top),
            vec2(menu.width() - size::LIB_LIST_PAD * 2.0, size::LIB_ROW_H),
        ))
    }

    /// Hit-tests a click at `p` against the pill or open menu items.
    pub fn ask(&self, mcp: &McpServer, p: Point) -> Option<McpAsk> {
        if self.hit(p) {
            return Some(if mcp.open() {
                McpAsk::Shut
            } else {
                McpAsk::ToggleMenu
            });
        }
        let menu = self.menu?;
        if !menu.contains(Pos2::new(p.x, p.y)) {
            return Some(McpAsk::Shut);
        }
        if mcp.editing().is_some() {
            return None;
        }
        let at = Pos2::new(p.x, p.y);
        for index in 0..self.rows {
            if let Some(r) = self.row(index) {
                if r.contains(at) {
                    return Some(match index {
                        0 => McpAsk::ToggleRunning,
                        1 => McpAsk::EditAddr,
                        2 => McpAsk::SetAddr(PRESET_ADDRS[0].to_string()),
                        3 => McpAsk::SetAddr(PRESET_ADDRS[1].to_string()),
                        4 => McpAsk::SetAddr(PRESET_ADDRS[2].to_string()),
                        _ => McpAsk::Shut,
                    });
                }
            }
        }
        Some(McpAsk::Shut)
    }
}

/// Formats the pill text depending on server status.
fn pill_label(mcp: &McpServer) -> String {
    if mcp.running {
        format!("mcp · {}", mcp.addr)
    } else {
        "mcp · off".to_string()
    }
}

/// Measures and positions the MCP server configuration pill in the transport row.
#[allow(clippy::too_many_arguments)]
pub fn mcp_server_pill(
    ctx: &egui::Context,
    layout: &karakuri_layout::Layout,
    values: Option<Transport>,
    audio: Option<&AudioIn>,
    tracker: Option<Tracker>,
    map: Option<&MapPill>,
    arr: &Arrangement,
    mcp: &McpServer,
    theme_mode: ThemeMode,
    theme_open: bool,
) -> Option<McpServerPill> {
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

    let label = pill_label(mcp);
    let text_w = width(&label);
    let pill_w = size::PILL_PAD_X * 2.0 + text_w + size::SINK_GAP + CHEVRON_W;
    let mid = strip.center().y;

    let right = match theme_pill(
        ctx, layout, values, audio, tracker, map, arr, theme_mode, theme_open,
    ) {
        Some(theme) => theme.pill.min.x - size::TRANSPORT_GAP,
        None => row.frame.min.x - size::TRANSPORT_GAP,
    };

    let pill = Rect::from_min_size(
        Pos2::new(right - pill_w, mid - size::PILL_H * 0.5),
        vec2(pill_w, size::PILL_H),
    );

    // Anchor after arrangement, map, learn, tracker, audio-in, or bar.
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

    if !strip.contains_rect(pill) || pill.min.x < after + size::TRANSPORT_GAP {
        return None;
    }

    let text = Rect::from_min_size(
        Pos2::new(pill.min.x + size::PILL_PAD_X, mid - size::PILL_H * 0.5),
        vec2(text_w, size::PILL_H),
    );
    let chevron = Rect::from_center_size(
        Pos2::new(pill.max.x - size::PILL_PAD_X - CHEVRON_W * 0.5, mid),
        vec2(CHEVRON_W, CHEVRON_H),
    );

    let menu = if mcp.open() {
        let menu_w = 200.0f32.max(pill_w);
        let menu_h = if mcp.editing().is_some() {
            size::LIB_LIST_PAD * 2.0 + size::LIB_ROW_H
        } else {
            size::LIB_LIST_PAD * 2.0 + size::LIB_ROW_H * MCP_MENU_ROWS as f32 + size::HAIRLINE
        };
        let viewport = to_egui(layout.viewport());
        Some(held_inside(
            &viewport,
            pill.min.x,
            pill.max.y + size::PILL_GAP,
            menu_w,
            menu_h,
        ))
    } else {
        None
    };

    Some(McpServerPill {
        pill,
        text,
        chevron,
        menu,
        rows: if mcp.editing().is_some() {
            0
        } else {
            MCP_MENU_ROWS
        },
    })
}

/// Paints the MCP server pill and dropdown menu card into the egui context.
pub(crate) fn mcp_server_into(ui: &Ui, pal: &Palette, pill: &McpServerPill, mcp: &McpServer) {
    let painter = ui.painter();
    let border_color = if mcp.running { pal.mint } else { pal.line };
    painter.rect_stroke(
        pill.pill,
        CornerRadius::same((size::PILL_H * 0.5) as u8),
        Stroke::new(size::HAIRLINE, border_color),
        StrokeKind::Inside,
    );

    let text_color = if mcp.running { pal.mint } else { pal.dim };
    let galley = painter.layout_no_wrap(
        pill_label(mcp),
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

    if let Some(typed) = mcp.editing() {
        let field = Rect::from_min_size(
            Pos2::new(
                card.min.x + size::LIB_LIST_PAD,
                card.min.y + size::LIB_LIST_PAD,
            ),
            vec2(card.width() - size::LIB_LIST_PAD * 2.0, size::LIB_ROW_H),
        );
        let text = format!("{typed}{CARET}");
        card_row_text(painter, field, &text, pal.text);
        return;
    }

    for index in 0..pill.rows {
        if let Some(row) = pill.row(index) {
            match index {
                0 => {
                    let label = if mcp.running {
                        "■ stop mcp server"
                    } else {
                        "● start mcp server"
                    };
                    let color = if mcp.running { pal.sun } else { pal.mint };
                    card_row_text(painter, row, label, color);
                }
                1 => {
                    let label = format!("addr · {}  (edit)", mcp.addr);
                    card_row_text(painter, row, &label, pal.text);
                    let rule_y = row.max.y + size::HAIRLINE * 0.5;
                    painter.line_segment(
                        [
                            Pos2::new(card.min.x + size::LIB_LIST_PAD, rule_y),
                            Pos2::new(card.max.x - size::LIB_LIST_PAD, rule_y),
                        ],
                        Stroke::new(size::HAIRLINE, pal.hair),
                    );
                }
                2 => {
                    let label = format!("preset · {}", PRESET_ADDRS[0]);
                    card_row_text(painter, row, &label, pal.dim);
                }
                3 => {
                    let label = format!("preset · {}", PRESET_ADDRS[1]);
                    card_row_text(painter, row, &label, pal.dim);
                }
                4 => {
                    let label = format!("preset · {}", PRESET_ADDRS[2]);
                    card_row_text(painter, row, &label, pal.dim);
                }
                _ => {}
            }
        }
    }
}
