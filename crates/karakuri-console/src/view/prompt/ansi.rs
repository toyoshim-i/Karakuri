//! ANSI terminal escape sequence parser, color rendition, and screen buffer.

use egui::Color32;

/// Character cell style including foreground, background, and text decorations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CellStyle {
    pub fg: Option<Color32>,
    pub bg: Option<Color32>,
    pub bold: bool,
    pub dim: bool,
    pub italic: bool,
    pub underline: bool,
    pub invert: bool,
}

/// A single character cell on the terminal screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    pub ch: char,
    pub style: CellStyle,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            ch: ' ',
            style: CellStyle::default(),
        }
    }
}

/// ANSI terminal escape events for cursor manipulation, line clearing, and character output.
#[derive(Debug, PartialEq, Eq)]
pub enum AnsiEvent {
    Print(char),
    Newline,
    CarriageReturn,
    Backspace,
    Tab,
    ClearLine(u8),
    ClearDisplay(u8),
    CursorUp(usize),
    CursorDown(usize),
    CursorForward(usize),
    CursorBack(usize),
    CursorCol(usize),
    CursorRow(usize),
    CursorPos(usize, usize),
    CursorNextLine(usize),
    CursorPrevLine(usize),
    EraseChars(usize),
    DeleteChars(usize),
    InsertChars(usize),
    SaveCursor,
    RestoreCursor,
    CursorVisible(bool),
    AlternateScreen(bool),
    Sgr(Vec<u16>),
}

#[derive(Debug, Default, PartialEq, Eq)]
enum AnsiState {
    #[default]
    Normal,
    Escape,
    Csi,
    Osc,
    Charset,
}

/// Streaming parser for ANSI escape sequences converting characters into terminal events.
#[derive(Debug, Default)]
pub struct AnsiParser {
    state: AnsiState,
    csi_params: String,
}

impl AnsiParser {
    pub fn parse_char(&mut self, c: char) -> Option<AnsiEvent> {
        match self.state {
            AnsiState::Normal => {
                if c == '\x1b' {
                    self.state = AnsiState::Escape;
                    None
                } else if c == '\r' {
                    Some(AnsiEvent::CarriageReturn)
                } else if c == '\n' {
                    Some(AnsiEvent::Newline)
                } else if c == '\x08' {
                    Some(AnsiEvent::Backspace)
                } else if c == '\t' {
                    Some(AnsiEvent::Tab)
                } else if c.is_control() {
                    None
                } else {
                    Some(AnsiEvent::Print(c))
                }
            }
            AnsiState::Escape => match c {
                '[' => {
                    self.state = AnsiState::Csi;
                    self.csi_params.clear();
                    None
                }
                ']' => {
                    self.state = AnsiState::Osc;
                    None
                }
                '(' | ')' | '*' | '+' => {
                    self.state = AnsiState::Charset;
                    None
                }
                '7' => {
                    self.state = AnsiState::Normal;
                    Some(AnsiEvent::SaveCursor)
                }
                '8' => {
                    self.state = AnsiState::Normal;
                    Some(AnsiEvent::RestoreCursor)
                }
                'M' => {
                    self.state = AnsiState::Normal;
                    Some(AnsiEvent::CursorUp(1))
                }
                'E' => {
                    self.state = AnsiState::Normal;
                    Some(AnsiEvent::CursorNextLine(1))
                }
                _ => {
                    self.state = AnsiState::Normal;
                    None
                }
            },
            AnsiState::Csi => {
                if c.is_ascii_digit() || c == ';' || c == '?' || c == '<' || c == '>' || c == '$' {
                    self.csi_params.push(c);
                    None
                } else if ('@'..='~').contains(&c) {
                    self.state = AnsiState::Normal;
                    self.dispatch_csi(c)
                } else {
                    None
                }
            }
            AnsiState::Osc => {
                if c == '\x07' || c == '\x1b' {
                    self.state = AnsiState::Normal;
                }
                None
            }
            AnsiState::Charset => {
                self.state = AnsiState::Normal;
                None
            }
        }
    }

    fn dispatch_csi(&self, cmd: char) -> Option<AnsiEvent> {
        if self.csi_params.starts_with('?') {
            let num: u32 = self.csi_params[1..]
                .split(';')
                .next()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            return match cmd {
                'h' => match num {
                    25 => Some(AnsiEvent::CursorVisible(true)),
                    47 | 1049 => Some(AnsiEvent::AlternateScreen(true)),
                    _ => None,
                },
                'l' => match num {
                    25 => Some(AnsiEvent::CursorVisible(false)),
                    47 | 1049 => Some(AnsiEvent::AlternateScreen(false)),
                    _ => None,
                },
                _ => None,
            };
        }

        match cmd {
            'm' => {
                let params = if self.csi_params.is_empty() {
                    vec![0]
                } else {
                    self.csi_params
                        .split(';')
                        .map(|s| s.parse::<u16>().unwrap_or(0))
                        .collect()
                };
                Some(AnsiEvent::Sgr(params))
            }
            'K' => {
                let n: u8 = self.csi_params.parse().unwrap_or(0);
                Some(AnsiEvent::ClearLine(n))
            }
            'J' => {
                let n: u8 = self.csi_params.parse().unwrap_or(0);
                Some(AnsiEvent::ClearDisplay(n))
            }
            'A' => {
                let n: usize = self.csi_params.parse().unwrap_or(1).max(1);
                Some(AnsiEvent::CursorUp(n))
            }
            'B' => {
                let n: usize = self.csi_params.parse().unwrap_or(1).max(1);
                Some(AnsiEvent::CursorDown(n))
            }
            'C' => {
                let n: usize = self.csi_params.parse().unwrap_or(1).max(1);
                Some(AnsiEvent::CursorForward(n))
            }
            'D' => {
                let n: usize = self.csi_params.parse().unwrap_or(1).max(1);
                Some(AnsiEvent::CursorBack(n))
            }
            'E' => {
                let n: usize = self.csi_params.parse().unwrap_or(1).max(1);
                Some(AnsiEvent::CursorNextLine(n))
            }
            'F' => {
                let n: usize = self.csi_params.parse().unwrap_or(1).max(1);
                Some(AnsiEvent::CursorPrevLine(n))
            }
            'G' | '`' => {
                let n: usize = self.csi_params.parse().unwrap_or(1).max(1);
                Some(AnsiEvent::CursorCol(n))
            }
            'd' => {
                let n: usize = self.csi_params.parse().unwrap_or(1).max(1);
                Some(AnsiEvent::CursorRow(n))
            }
            'H' | 'f' => {
                let mut parts = self.csi_params.split(';');
                let r: usize = parts
                    .next()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(1)
                    .max(1);
                let col: usize = parts
                    .next()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(1)
                    .max(1);
                Some(AnsiEvent::CursorPos(r, col))
            }
            'P' => {
                let n: usize = self.csi_params.parse().unwrap_or(1).max(1);
                Some(AnsiEvent::DeleteChars(n))
            }
            '@' => {
                let n: usize = self.csi_params.parse().unwrap_or(1).max(1);
                Some(AnsiEvent::InsertChars(n))
            }
            'X' => {
                let n: usize = self.csi_params.parse().unwrap_or(1).max(1);
                Some(AnsiEvent::EraseChars(n))
            }
            's' => Some(AnsiEvent::SaveCursor),
            'u' => Some(AnsiEvent::RestoreCursor),
            _ => None,
        }
    }
}

/// Decodes standard 16 ANSI colors into egui Color32.
pub fn ansi_16_color(idx: u16, bright: bool) -> Color32 {
    match (idx, bright) {
        (0, false) => Color32::from_rgb(0, 0, 0),       // Black
        (1, false) => Color32::from_rgb(205, 49, 49),   // Red
        (2, false) => Color32::from_rgb(13, 188, 121),  // Green
        (3, false) => Color32::from_rgb(229, 229, 16),  // Yellow
        (4, false) => Color32::from_rgb(36, 114, 200),  // Blue
        (5, false) => Color32::from_rgb(188, 63, 188),  // Magenta
        (6, false) => Color32::from_rgb(17, 168, 205),  // Cyan
        (7, false) => Color32::from_rgb(229, 229, 229), // White
        (0, true) => Color32::from_rgb(102, 102, 102),  // Bright Black (Gray)
        (1, true) => Color32::from_rgb(241, 76, 76),    // Bright Red
        (2, true) => Color32::from_rgb(35, 209, 139),   // Bright Green
        (3, true) => Color32::from_rgb(245, 245, 67),   // Bright Yellow
        (4, true) => Color32::from_rgb(59, 142, 234),   // Bright Blue
        (5, true) => Color32::from_rgb(214, 112, 214),  // Bright Magenta
        (6, true) => Color32::from_rgb(41, 184, 219),   // Bright Cyan
        (7, true) => Color32::from_rgb(255, 255, 255),  // Bright White
        _ => Color32::from_rgb(229, 229, 229),
    }
}

/// Decodes 256 ANSI colors into egui Color32.
pub fn ansi_256_color(n: u8) -> Color32 {
    if n < 16 {
        ansi_16_color((n % 8) as u16, n >= 8)
    } else if n < 232 {
        let idx = n - 16;
        let r = (idx / 36) * 51;
        let g = ((idx % 36) / 6) * 51;
        let b = (idx % 6) * 51;
        Color32::from_rgb(r, g, b)
    } else {
        let v = (n - 232) * 10 + 8;
        Color32::from_rgb(v, v, v)
    }
}

/// Updates cell styling according to ANSI SGR parameter tokens.
pub fn apply_sgr(params: &[u16], style: &mut CellStyle) {
    if params.is_empty() {
        *style = CellStyle::default();
        return;
    }
    let mut i = 0;
    while i < params.len() {
        match params[i] {
            0 => *style = CellStyle::default(),
            1 => style.bold = true,
            2 => style.dim = true,
            3 => style.italic = true,
            4 => style.underline = true,
            7 => style.invert = true,
            22 => {
                style.bold = false;
                style.dim = false;
            }
            23 => style.italic = false,
            24 => style.underline = false,
            27 => style.invert = false,
            30..=37 => style.fg = Some(ansi_16_color(params[i] - 30, false)),
            38 => {
                if i + 2 < params.len() && params[i + 1] == 5 {
                    style.fg = Some(ansi_256_color(params[i + 2] as u8));
                    i += 2;
                } else if i + 4 < params.len() && params[i + 1] == 2 {
                    let r = params[i + 2] as u8;
                    let g = params[i + 3] as u8;
                    let b = params[i + 4] as u8;
                    style.fg = Some(Color32::from_rgb(r, g, b));
                    i += 4;
                }
            }
            39 => style.fg = None,
            40..=47 => style.bg = Some(ansi_16_color(params[i] - 40, false)),
            48 => {
                if i + 2 < params.len() && params[i + 1] == 5 {
                    style.bg = Some(ansi_256_color(params[i + 2] as u8));
                    i += 2;
                } else if i + 4 < params.len() && params[i + 1] == 2 {
                    let r = params[i + 2] as u8;
                    let g = params[i + 3] as u8;
                    let b = params[i + 4] as u8;
                    style.bg = Some(Color32::from_rgb(r, g, b));
                    i += 4;
                }
            }
            49 => style.bg = None,
            90..=97 => style.fg = Some(ansi_16_color(params[i] - 90, true)),
            100..=107 => style.bg = Some(ansi_16_color(params[i] - 100, true)),
            _ => {}
        }
        i += 1;
    }
}

/// Backup buffer storing rows and cursor coordinates for alternate screen switching.
type ScreenBackup = (Vec<Vec<Cell>>, (usize, usize));

/// Buffer holding terminal styled character cells and active cursor state.
#[derive(Debug)]
pub struct Scrollback {
    /// Rendered lines of styled character cells.
    pub rows: Vec<Vec<Cell>>,
    /// Active cursor coordinates (row, col) (0-indexed).
    pub cursor: (usize, usize),
    /// Whether the terminal cursor is currently visible.
    pub cursor_visible: bool,
    /// Active graphic style for upcoming characters.
    pub current_style: CellStyle,
    /// Stashed cursor position from SaveCursor.
    saved_cursor: Option<(usize, usize)>,
    /// Alternate screen buffer backup.
    alt_screen: Option<ScreenBackup>,
    /// Streaming ANSI parser state.
    parser: AnsiParser,
}

impl Default for Scrollback {
    fn default() -> Self {
        Self {
            rows: vec![Vec::new()],
            cursor: (0, 0),
            cursor_visible: true,
            current_style: CellStyle::default(),
            saved_cursor: None,
            alt_screen: None,
            parser: AnsiParser::default(),
        }
    }
}

impl Scrollback {
    /// Maximum scrollback rows retained in memory.
    pub const MAX_LINES: usize = 2048;

    /// Dispatches an ANSI terminal event to update buffer lines and cursor.
    pub fn handle_event(&mut self, ev: AnsiEvent) {
        if self.rows.is_empty() {
            self.rows.push(Vec::new());
        }
        match ev {
            AnsiEvent::Print(c) => {
                while self.rows.len() <= self.cursor.0 {
                    self.rows.push(Vec::new());
                }
                let row = &mut self.rows[self.cursor.0];
                if self.cursor.1 >= row.len() {
                    row.resize(self.cursor.1, Cell::default());
                    row.push(Cell {
                        ch: c,
                        style: self.current_style,
                    });
                } else {
                    row[self.cursor.1] = Cell {
                        ch: c,
                        style: self.current_style,
                    };
                }
                self.cursor.1 += 1;
            }
            AnsiEvent::Newline => {
                self.cursor.0 += 1;
                while self.rows.len() <= self.cursor.0 {
                    self.rows.push(Vec::new());
                }
                if self.rows.len() > Self::MAX_LINES {
                    self.rows.remove(0);
                    self.cursor.0 = self.cursor.0.saturating_sub(1);
                }
            }
            AnsiEvent::CarriageReturn => {
                self.cursor.1 = 0;
            }
            AnsiEvent::Backspace => {
                self.cursor.1 = self.cursor.1.saturating_sub(1);
            }
            AnsiEvent::Tab => {
                self.cursor.1 = (self.cursor.1 / 8 + 1) * 8;
            }
            AnsiEvent::ClearLine(mode) => {
                if self.cursor.0 < self.rows.len() {
                    match mode {
                        2 => {
                            self.rows[self.cursor.0].clear();
                        }
                        1 => {
                            let row = &mut self.rows[self.cursor.0];
                            for cell in row.iter_mut().take(self.cursor.1 + 1) {
                                *cell = Cell::default();
                            }
                        }
                        _ => {
                            self.rows[self.cursor.0].truncate(self.cursor.1);
                        }
                    }
                }
            }
            AnsiEvent::ClearDisplay(mode) => match mode {
                2 | 3 => {
                    self.rows.clear();
                    self.rows.push(Vec::new());
                    self.cursor = (0, 0);
                }
                1 => {
                    for r in 0..self.cursor.0.min(self.rows.len()) {
                        self.rows[r].clear();
                    }
                    if self.cursor.0 < self.rows.len() {
                        let row = &mut self.rows[self.cursor.0];
                        for cell in row.iter_mut().take(self.cursor.1 + 1) {
                            *cell = Cell::default();
                        }
                    }
                }
                _ => {
                    if self.cursor.0 < self.rows.len() {
                        self.rows[self.cursor.0].truncate(self.cursor.1);
                        self.rows.truncate(self.cursor.0 + 1);
                    }
                }
            },
            AnsiEvent::CursorUp(n) => {
                self.cursor.0 = self.cursor.0.saturating_sub(n);
            }
            AnsiEvent::CursorDown(n) => {
                let target = (self.cursor.0 + n).min(Self::MAX_LINES);
                while self.rows.len() <= target {
                    self.rows.push(Vec::new());
                }
                self.cursor.0 = target;
            }
            AnsiEvent::CursorForward(n) => {
                self.cursor.1 += n;
            }
            AnsiEvent::CursorBack(n) => {
                self.cursor.1 = self.cursor.1.saturating_sub(n);
            }
            AnsiEvent::CursorCol(n) => {
                self.cursor.1 = n.saturating_sub(1);
            }
            AnsiEvent::CursorRow(n) => {
                let target = n.saturating_sub(1).min(Self::MAX_LINES);
                while self.rows.len() <= target {
                    self.rows.push(Vec::new());
                }
                self.cursor.0 = target;
            }
            AnsiEvent::CursorPos(r, col) => {
                let target_row = r.saturating_sub(1).min(Self::MAX_LINES);
                while self.rows.len() <= target_row {
                    self.rows.push(Vec::new());
                }
                self.cursor.0 = target_row;
                self.cursor.1 = col.saturating_sub(1);
            }
            AnsiEvent::CursorNextLine(n) => {
                let target = (self.cursor.0 + n).min(Self::MAX_LINES);
                while self.rows.len() <= target {
                    self.rows.push(Vec::new());
                }
                self.cursor.0 = target;
                self.cursor.1 = 0;
            }
            AnsiEvent::CursorPrevLine(n) => {
                self.cursor.0 = self.cursor.0.saturating_sub(n);
                self.cursor.1 = 0;
            }
            AnsiEvent::EraseChars(n) => {
                if self.cursor.0 < self.rows.len() {
                    let row = &mut self.rows[self.cursor.0];
                    for idx in self.cursor.1..(self.cursor.1 + n).min(row.len()) {
                        row[idx] = Cell::default();
                    }
                }
            }
            AnsiEvent::DeleteChars(n) => {
                if self.cursor.0 < self.rows.len() {
                    let row = &mut self.rows[self.cursor.0];
                    if self.cursor.1 < row.len() {
                        let remove_count = n.min(row.len() - self.cursor.1);
                        row.drain(self.cursor.1..self.cursor.1 + remove_count);
                    }
                }
            }
            AnsiEvent::InsertChars(n) => {
                if self.cursor.0 < self.rows.len() {
                    let row = &mut self.rows[self.cursor.0];
                    if self.cursor.1 <= row.len() {
                        row.splice(
                            self.cursor.1..self.cursor.1,
                            std::iter::repeat_n(Cell::default(), n),
                        );
                    }
                }
            }
            AnsiEvent::SaveCursor => {
                self.saved_cursor = Some(self.cursor);
            }
            AnsiEvent::RestoreCursor => {
                if let Some(pos) = self.saved_cursor {
                    while self.rows.len() <= pos.0 {
                        self.rows.push(Vec::new());
                    }
                    self.cursor = pos;
                }
            }
            AnsiEvent::CursorVisible(vis) => {
                self.cursor_visible = vis;
            }
            AnsiEvent::AlternateScreen(enable) => {
                if enable {
                    if self.alt_screen.is_none() {
                        let current_rows = std::mem::take(&mut self.rows);
                        let current_cursor = self.cursor;
                        self.alt_screen = Some((current_rows, current_cursor));
                        self.rows = vec![Vec::new()];
                        self.cursor = (0, 0);
                    }
                } else if let Some((saved_rows, saved_cursor)) = self.alt_screen.take() {
                    self.rows = saved_rows;
                    self.cursor = saved_cursor;
                }
            }
            AnsiEvent::Sgr(params) => {
                apply_sgr(&params, &mut self.current_style);
            }
        }
    }

    /// Pushes a single character into the scrollback buffer.
    pub fn push_char(&mut self, ch: char) {
        if let Some(ev) = self.parser.parse_char(ch) {
            self.handle_event(ev);
        }
    }

    /// Appends a plain text string into the scrollback buffer.
    pub fn push_str(&mut self, s: &str) {
        for c in s.chars() {
            self.push_char(c);
        }
    }

    /// Returns plain string representation of lines.
    pub fn lines(&self) -> Vec<String> {
        self.rows
            .iter()
            .map(|row| {
                let s: String = row.iter().map(|c| c.ch).collect();
                s.trim_end().to_string()
            })
            .collect()
    }

    /// Returns active cursor coordinates (row, col).
    pub fn cursor(&self) -> (usize, usize) {
        self.cursor
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_sgr_truecolor_and_reset() {
        let mut sb = Scrollback::default();
        // Foreground RGB (219, 177, 49) + char 'A' + Reset
        sb.push_str("\x1b[38;2;219;177;49mA\x1b[mB");
        assert_eq!(sb.rows.len(), 1);
        assert_eq!(sb.rows[0].len(), 2);
        assert_eq!(sb.rows[0][0].ch, 'A');
        assert_eq!(
            sb.rows[0][0].style.fg,
            Some(Color32::from_rgb(219, 177, 49))
        );
        assert_eq!(sb.rows[0][1].ch, 'B');
        assert_eq!(sb.rows[0][1].style.fg, None);
    }

    #[test]
    fn cursor_positioning_expands_rows() {
        let mut sb = Scrollback::default();
        // Position at row 5, col 3 and print 'X'
        sb.push_str("\x1b[5;3HX");
        assert_eq!(sb.cursor, (4, 3));
        assert!(sb.rows.len() >= 5);
        assert_eq!(sb.rows[4][2].ch, 'X');
    }

    #[test]
    fn cursor_visibility_tracking() {
        let mut sb = Scrollback::default();
        assert!(sb.cursor_visible);
        sb.push_str("\x1b[?25l");
        assert!(!sb.cursor_visible);
        sb.push_str("\x1b[?25h");
        assert!(sb.cursor_visible);
    }

    #[test]
    fn alternate_screen_buffer_switching() {
        let mut sb = Scrollback::default();
        sb.push_str("Main Screen");
        assert_eq!(sb.lines()[0], "Main Screen");

        // Enter alternate screen
        sb.push_str("\x1b[?1049h");
        assert_eq!(sb.lines()[0], "");
        sb.push_str("Alt Screen");
        assert_eq!(sb.lines()[0], "Alt Screen");

        // Exit alternate screen
        sb.push_str("\x1b[?1049l");
        assert_eq!(sb.lines()[0], "Main Screen");
    }

    #[test]
    fn erase_in_display_mode_zero() {
        let mut sb = Scrollback::default();
        sb.push_str("line 1\nline 2\nline 3");
        assert_eq!(sb.rows.len(), 3);

        // Move to line 2, col 1 and clear to end of screen
        sb.push_str("\x1b[2;1H\x1b[J");
        assert_eq!(sb.rows.len(), 2);
        assert_eq!(sb.rows[1].len(), 0);
    }

    #[test]
    fn test_row_col() {
        let mut sb = Scrollback::default();
        sb.push_str("\n\x1b[5G▀");
        let chars: String = sb.rows[1].iter().map(|c| c.ch).collect();
        println!("CHARS: {:?}", chars);
        assert_eq!(chars, "    ▀");
    }
}
