//! Zero-allocation session replay and stream reading using memory-mapped I/O.

use std::fs;
use std::path::Path;

use crate::record::Record;
use crate::store::StoreError;

/// Scanned index entry for a tick record in a session stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TickIndexEntry {
    /// 1-based line number in the session file.
    pub line_number: usize,
    /// Byte offset where this tick line starts in the file.
    pub byte_offset: usize,
    /// Byte length of the tick line (excluding trailing newline / carriage return).
    pub byte_len: usize,
    /// 0-based sequential tick index (0th tick, 1st tick, ...).
    pub tick_index: usize,
    /// Steps advanced by this tick (from `"steps": N`, default 1).
    pub steps: u8,
    /// Cumulative sum of steps up to and including this tick.
    pub total_steps: u64,
    /// Optional timestamp / pts if present on the tick record.
    pub timestamp: Option<u64>,
}

/// An iterator yielding zero-copy byte slices (`&[u8]`) for each line.
pub struct RawLines<'a> {
    slice: &'a [u8],
}

impl<'a> Iterator for RawLines<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        if self.slice.is_empty() {
            return None;
        }
        let (line, rest) = match self.slice.iter().position(|&b| b == b'\n') {
            Some(idx) => (&self.slice[..idx], &self.slice[idx + 1..]),
            None => (self.slice, &self.slice[self.slice.len()..]),
        };
        self.slice = rest;
        let trimmed = if line.ends_with(b"\r") {
            &line[..line.len() - 1]
        } else {
            line
        };
        Some(trimmed)
    }
}

impl<'a> std::iter::FusedIterator for RawLines<'a> {}

/// Memory-mapped stream reader providing zero-copy line iteration, fast tick scanning,
/// and record deserialization.
pub struct MmapStreamReader {
    mmap: Option<memmap2::Mmap>,
}

impl MmapStreamReader {
    /// Open a file at `path` and create a read-only memory map.
    ///
    /// Empty (0-byte) files are handled safely without error.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StoreError> {
        let file = fs::File::open(path.as_ref())?;
        Self::from_file(&file)
    }

    /// Create a reader from an existing `File` handle.
    pub fn from_file(file: &fs::File) -> Result<Self, StoreError> {
        let len = file.metadata()?.len();
        let mmap = if len == 0 {
            None
        } else {
            // Safety: The file is mapped read-only for session replay.
            Some(unsafe { memmap2::Mmap::map(file)? })
        };
        Ok(Self { mmap })
    }

    /// Return the entire underlying buffer as a byte slice.
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        self.mmap.as_deref().unwrap_or(&[])
    }

    /// Return the entire underlying buffer as a UTF-8 string slice if valid.
    pub fn as_str(&self) -> Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(self.as_bytes())
    }

    /// Total size in bytes.
    #[inline]
    pub fn len(&self) -> usize {
        self.as_bytes().len()
    }

    /// Whether the buffer is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.as_bytes().is_empty()
    }

    /// Zero-allocation line slicing yielding borrowed `&[u8]` line slices without copying.
    pub fn lines_raw(&self) -> RawLines<'_> {
        RawLines {
            slice: self.as_bytes(),
        }
    }

    /// Zero-allocation line slicing yielding borrowed `&[u8]` line slices (alias for `lines_raw`).
    pub fn lines_bytes(&self) -> RawLines<'_> {
        self.lines_raw()
    }

    /// Zero-allocation line slicing yielding borrowed `&str` line slices without copying.
    pub fn lines(&self) -> impl Iterator<Item = &str> + '_ {
        self.lines_raw()
            .filter_map(|line| std::str::from_utf8(line).ok())
    }

    /// Detect the schema version from line 0.
    ///
    /// Reports the header version if present, otherwise returns 1 (legacy unversioned).
    pub fn schema_version(&self) -> u32 {
        match self.lines_raw().next() {
            Some(first_line) => {
                if let Ok(Record::Header { version }) = serde_json::from_slice::<Record>(first_line)
                {
                    version
                } else {
                    1
                }
            }
            None => 1,
        }
    }

    /// Decode records directly from the mmap buffer.
    pub fn records(&self) -> impl Iterator<Item = Result<Record, StoreError>> + '_ {
        self.lines_raw().enumerate().map(|(i, line)| {
            serde_json::from_slice::<Record>(line).map_err(|source| StoreError::Record {
                line: i + 1,
                source,
            })
        })
    }

    /// Fast tick indexing / scanning: scans byte slices for tick markers and extracts
    /// tick timestamps/counters without full JSON object deserialization of intermediate lines.
    pub fn iter_ticks(&self) -> impl Iterator<Item = TickIndexEntry> + '_ {
        let base_ptr = self.as_bytes().as_ptr() as usize;
        let mut tick_counter: usize = 0;
        let mut total_steps: u64 = 0;

        self.lines_raw()
            .enumerate()
            .filter_map(move |(line_idx, line)| {
                if is_tick_line(line) {
                    let steps = extract_steps(line);
                    let timestamp = extract_timestamp(line);
                    let byte_offset = (line.as_ptr() as usize).saturating_sub(base_ptr);
                    let byte_len = line.len();
                    let entry = TickIndexEntry {
                        line_number: line_idx + 1,
                        byte_offset,
                        byte_len,
                        tick_index: tick_counter,
                        steps,
                        total_steps: total_steps + (steps as u64),
                        timestamp,
                    };
                    tick_counter += 1;
                    total_steps += steps as u64;
                    Some(entry)
                } else {
                    None
                }
            })
    }

    /// Scan and collect all ticks into a `Vec<TickIndexEntry>`.
    pub fn scan_ticks(&self) -> Vec<TickIndexEntry> {
        self.iter_ticks().collect()
    }
}

/// Fast check if a line is a `Record::Tick` without full JSON deserialization.
fn is_tick_line(line: &[u8]) -> bool {
    if !line.windows(6).any(|w| w == b"\"tick\"") {
        return false;
    }
    let mut i = 0;
    while i + 3 <= line.len() {
        if &line[i..i + 3] == b"\"t\"" {
            let mut j = i + 3;
            while j < line.len() && line[j].is_ascii_whitespace() {
                j += 1;
            }
            if j < line.len() && line[j] == b':' {
                j += 1;
                while j < line.len() && line[j].is_ascii_whitespace() {
                    j += 1;
                }
                if j + 6 <= line.len() && &line[j..j + 6] == b"\"tick\"" {
                    return true;
                }
            }
        }
        i += 1;
    }
    false
}

/// Extract `"steps": N` from a tick byte slice without full JSON parsing.
fn extract_steps(line: &[u8]) -> u8 {
    let mut i = 0;
    while i + 7 <= line.len() {
        if &line[i..i + 7] == b"\"steps\"" {
            let mut j = i + 7;
            while j < line.len() && line[j].is_ascii_whitespace() {
                j += 1;
            }
            if j < line.len() && line[j] == b':' {
                j += 1;
                while j < line.len() && line[j].is_ascii_whitespace() {
                    j += 1;
                }
                let mut steps: u8 = 0;
                let mut found_digit = false;
                while j < line.len() && line[j].is_ascii_digit() {
                    found_digit = true;
                    steps = steps.saturating_mul(10).saturating_add(line[j] - b'0');
                    j += 1;
                }
                if found_digit {
                    return steps;
                }
            }
        }
        i += 1;
    }
    1
}

/// Extract optional timestamp / pts / time from a tick byte slice without full JSON parsing.
fn extract_timestamp(line: &[u8]) -> Option<u64> {
    for pattern in &[
        b"\"timestamp\"".as_slice(),
        b"\"ts\"".as_slice(),
        b"\"pts\"".as_slice(),
        b"\"time\"".as_slice(),
    ] {
        let pat_len = pattern.len();
        let mut i = 0;
        while i + pat_len <= line.len() {
            if &line[i..i + pat_len] == *pattern {
                let mut j = i + pat_len;
                while j < line.len() && line[j].is_ascii_whitespace() {
                    j += 1;
                }
                if j < line.len() && line[j] == b':' {
                    j += 1;
                    while j < line.len() && line[j].is_ascii_whitespace() {
                        j += 1;
                    }
                    let mut ts: u64 = 0;
                    let mut found = false;
                    while j < line.len() && line[j].is_ascii_digit() {
                        found = true;
                        ts = ts
                            .saturating_mul(10)
                            .saturating_add((line[j] - b'0') as u64);
                        j += 1;
                    }
                    if found {
                        return Some(ts);
                    }
                }
            }
            i += 1;
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_lines_iteration() {
        let data = b"line 1\nline 2\r\nline 3\n";
        let lines: Vec<&str> = RawLines { slice: data }
            .map(|l| std::str::from_utf8(l).unwrap())
            .collect();
        assert_eq!(lines, vec!["line 1", "line 2", "line 3"]);
    }

    #[test]
    fn fast_tick_line_detection() {
        assert!(is_tick_line(br#"{"t":"tick","steps":1}"#));
        assert!(is_tick_line(br#"{"t": "tick", "steps": 2}"#));
        assert!(is_tick_line(br#"{"steps":1,"t":"tick"}"#));
        assert!(!is_tick_line(br#"{"t":"set","id":"s","v":1}"#));
        assert!(!is_tick_line(
            br#"{"t":"param","key":"tick_delay","value":1.0}"#
        ));
    }

    #[test]
    fn fast_steps_and_timestamp_extraction() {
        assert_eq!(extract_steps(br#"{"t":"tick","steps":3}"#), 3);
        assert_eq!(extract_steps(br#"{"t":"tick"}"#), 1);
        assert_eq!(
            extract_timestamp(br#"{"t":"tick","steps":1,"timestamp":123456789}"#),
            Some(123456789)
        );
        assert_eq!(
            extract_timestamp(br#"{"t":"tick","steps":1,"ts":42}"#),
            Some(42)
        );
        assert_eq!(extract_timestamp(br#"{"t":"tick","steps":1}"#), None);
    }
}
