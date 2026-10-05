//! Interactive terminal line editing primitives, word boundaries, and ANSI redraw synchronization.

use karakuri_console::view::prompt::ansi::Scrollback;

use super::PROMPT_LABEL;

/// Finds the preceding word boundary index in UTF-8 character space.
pub fn prev_word_boundary(s: &str, char_idx: usize) -> usize {
    let chars: Vec<char> = s.chars().collect();
    if char_idx == 0 || chars.is_empty() {
        return 0;
    }
    let mut i = char_idx.min(chars.len());
    while i > 0 && chars[i - 1].is_whitespace() {
        i -= 1;
    }
    while i > 0 && !chars[i - 1].is_whitespace() {
        i -= 1;
    }
    i
}

/// Finds the next word boundary index in UTF-8 character space.
pub fn next_word_boundary(s: &str, char_idx: usize) -> usize {
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();
    if char_idx >= len {
        return len;
    }
    let mut i = char_idx;
    while i < len && !chars[i].is_whitespace() {
        i += 1;
    }
    while i < len && chars[i].is_whitespace() {
        i += 1;
    }
    i
}

/// Inserts a string at the specified character index in UTF-8 space.
pub fn insert_str_at(s: &mut String, char_idx: usize, insert: &str) {
    let mut new_s = String::with_capacity(s.len() + insert.len());
    let mut inserted = false;
    for (i, ch) in s.chars().enumerate() {
        if i == char_idx {
            new_s.push_str(insert);
            inserted = true;
        }
        new_s.push(ch);
    }
    if !inserted {
        new_s.push_str(insert);
    }
    *s = new_s;
}

/// Removes a single character at the specified character index. Returns true if removed.
pub fn remove_char_at(s: &mut String, char_idx: usize) -> bool {
    let char_count = s.chars().count();
    if char_idx >= char_count {
        return false;
    }
    let mut new_s = String::with_capacity(s.len());
    for (i, ch) in s.chars().enumerate() {
        if i != char_idx {
            new_s.push(ch);
        }
    }
    *s = new_s;
    true
}

/// Removes a half-open range `[start, end)` of characters from the string.
pub fn remove_char_range(s: &mut String, start: usize, end: usize) {
    let mut new_s = String::with_capacity(s.len());
    for (i, ch) in s.chars().enumerate() {
        if i < start || i >= end {
            new_s.push(ch);
        }
    }
    *s = new_s;
}

/// Redraws the terminal prompt line with full ANSI line clearing and cursor repositioning.
pub fn redraw_prompt_line(sb: &mut Scrollback, buffer: &str, cursor_pos: usize) {
    let total_chars = buffer.chars().count();
    let clamped_cursor = cursor_pos.min(total_chars);
    let back_chars = total_chars - clamped_cursor;

    let mut seq = String::new();
    seq.push_str("\r\x1b[2K");
    seq.push_str(PROMPT_LABEL);
    seq.push_str(buffer);
    if back_chars > 0 {
        seq.push_str(&format!("\x1b[{}D", back_chars));
    }
    sb.push_str(&seq);
}
