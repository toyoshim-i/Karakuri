//! Generic ndjson line I/O shared by Set files and session streams.
//!
//! `Record::Unknown` is the format's forward-compatibility escape hatch — an
//! unrecognised `t` deserialises to it rather than failing — but the variant
//! carries no data. Serialising it back out would not reproduce the line that
//! produced it; it would produce `{"t":"unknown"}`, discarding whatever the
//! original line actually said. That is worse than dropping the line outright,
//! because it looks like fidelity while corrupting the record.
//!
//! [`Line`] fixes this without touching `record.rs`: it carries the exact
//! source text alongside the parsed [`Record`], and writing always emits that
//! text back verbatim. A record built in memory (never read from a file) has no
//! "original" text, so [`Line::new`] serialises it once, at construction, and
//! that becomes its text from then on.

use std::fs;
use std::path::Path;

use crate::record::Record;
use crate::store::StoreError;

/// One line of an ndjson file: the record it parses to, plus the text it came
/// from.
#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    record: Record,
    raw: String,
}

impl Line {
    /// Wrap a record constructed in memory. Serialises it immediately to fix its
    /// on-disk text.
    ///
    /// Do not use this for [`Record::Unknown`] — that variant only ever arises from
    /// parsing a line the store does not understand, and serialising it fresh would
    /// fabricate a `{"t":"unknown"}` line that never existed. Round trip unknown
    /// lines through [`read`] and [`write`] instead, which preserve their original
    /// text.
    pub fn new(record: Record) -> Line {
        let raw = serde_json::to_string(&record).expect("Record serialises to ndjson");
        Line { record, raw }
    }

    fn parse(raw: &str, line_no: usize) -> Result<Line, StoreError> {
        let record: Record = serde_json::from_str(raw).map_err(|source| StoreError::Record {
            line: line_no,
            source,
        })?;
        Ok(Line {
            record,
            raw: raw.to_string(),
        })
    }

    /// The parsed record.
    pub fn record(&self) -> &Record {
        &self.record
    }

    /// The record, taken back out.
    ///
    /// For a writer that has serialised a line and wants the record's own buffers
    /// back rather than freeing them — `karakuri-environment`'s session recorder
    /// returns an audio record's band `Vec` to the frame path this way, so nothing
    /// allocates one per frame.
    pub fn into_record(self) -> Record {
        self.record
    }

    /// The exact text this line will write back as.
    pub fn as_str(&self) -> &str {
        &self.raw
    }
}

/// Read an ndjson file into its lines, one record per line.
///
/// A blank line has no record to parse and is reported as malformed, same as
/// any other unparsable line, carrying its 1-based line number.
pub fn read(path: &Path) -> Result<Vec<Line>, StoreError> {
    let text = fs::read_to_string(path)?;
    text.lines()
        .enumerate()
        .map(|(i, line)| Line::parse(line, i + 1))
        .collect()
}

/// Write lines back out, one record per line, LF-terminated.
pub fn write(path: &Path, lines: &[Line]) -> Result<(), StoreError> {
    let mut buf = String::new();
    for line in lines {
        buf.push_str(line.as_str());
        buf.push('\n');
    }
    write_atomic(path, buf.as_bytes())
}

/// Write `bytes` to `path` by writing a sibling temp file and renaming it into
/// place, so a reader never observes a partially written file and a crash
/// mid-write never corrupts whatever was already there.
pub(crate) fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
    let mut tmp_name = path.file_name().unwrap_or_default().to_os_string();
    tmp_name.push(".tmp");
    let tmp_path = path.with_file_name(tmp_name);
    fs::write(&tmp_path, bytes)?;
    fs::rename(&tmp_path, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::Layer;

    #[test]
    fn unknown_line_preserves_original_text() {
        let line = Line::parse(r#"{"t":"phrase","at":4.0}"#, 1).unwrap();
        assert_eq!(line.record(), &Record::Unknown);
        assert_eq!(line.as_str(), r#"{"t":"phrase","at":4.0}"#);
    }

    #[test]
    fn new_line_serialises_immediately() {
        // The `index` a seed gained addresses the node it salts, and 0 is
        // written as nothing — so this assertion is the same bytes it always
        // was, which is the whole claim the field's `skip_serializing_if`
        // makes, seen from the layer that writes the lines out.
        let line = Line::new(Record::Seed {
            stream: Layer::L1,
            index: 0,
            value: 7,
        });
        assert_eq!(line.as_str(), r#"{"t":"seed","stream":"L1","value":7}"#);
    }
}
