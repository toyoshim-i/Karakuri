//! Schema versioning and migration for `.kbset` and session ndjson streams.

use crate::ndjson::Line;
use crate::record::{Record, CURRENT_SCHEMA_VERSION};

/// Detect the schema version of an ndjson stream.
///
/// - If the first line is `Record::Header { version }`, report that version.
/// - If the first line is not a header (or if the stream is empty), detect as Version 1 (legacy unversioned).
pub fn detect_version(lines: &[Line]) -> u32 {
    match lines.first() {
        Some(line) => match line.record() {
            Record::Header { version } => *version,
            _ => 1,
        },
        None => 1,
    }
}

/// Ensure a `Record::Header { version: CURRENT_SCHEMA_VERSION }` is present at line 0,
/// preserving all records.
///
/// - If line 0 is already `Record::Header { version: CURRENT_SCHEMA_VERSION }`, returns the lines as-is.
/// - If line 0 is a `Record::Header` with a different version, replaces it with `CURRENT_SCHEMA_VERSION`
///   and preserves the rest of the lines.
/// - If line 0 is not a header, prepends `Record::Header { version: CURRENT_SCHEMA_VERSION }`
///   and preserves all lines.
pub fn migrate_to_current(lines: &[Line]) -> Vec<Line> {
    match lines.first() {
        Some(line) => match line.record() {
            Record::Header { version } if *version == CURRENT_SCHEMA_VERSION => lines.to_vec(),
            Record::Header { .. } => {
                let mut migrated = Vec::with_capacity(lines.len());
                migrated.push(Line::new(Record::Header {
                    version: CURRENT_SCHEMA_VERSION,
                }));
                migrated.extend_from_slice(&lines[1..]);
                migrated
            }
            _ => {
                let mut migrated = Vec::with_capacity(lines.len() + 1);
                migrated.push(Line::new(Record::Header {
                    version: CURRENT_SCHEMA_VERSION,
                }));
                migrated.extend_from_slice(lines);
                migrated
            }
        },
        None => vec![Line::new(Record::Header {
            version: CURRENT_SCHEMA_VERSION,
        })],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unversioned_detected_as_v1() {
        let lines = vec![Line::new(Record::Set {
            id: "ambient".into(),
            v: 1,
        })];
        assert_eq!(detect_version(&lines), 1);
    }

    #[test]
    fn empty_detected_as_v1() {
        assert_eq!(detect_version(&[]), 1);
    }

    #[test]
    fn header_detected_correctly() {
        let lines = vec![
            Line::new(Record::Header { version: 2 }),
            Line::new(Record::Set {
                id: "ambient".into(),
                v: 1,
            }),
        ];
        assert_eq!(detect_version(&lines), 2);
    }

    #[test]
    fn migration_prepends_header_for_legacy() {
        let lines = vec![Line::new(Record::Set {
            id: "legacy".into(),
            v: 1,
        })];
        let migrated = migrate_to_current(&lines);
        assert_eq!(migrated.len(), 2);
        assert_eq!(
            migrated[0].record(),
            &Record::Header {
                version: CURRENT_SCHEMA_VERSION
            }
        );
        assert_eq!(migrated[1].record(), lines[0].record());
        assert_eq!(detect_version(&migrated), CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn migration_is_idempotent_if_already_current() {
        let lines = vec![
            Line::new(Record::Header {
                version: CURRENT_SCHEMA_VERSION,
            }),
            Line::new(Record::Set {
                id: "current".into(),
                v: 1,
            }),
        ];
        let migrated = migrate_to_current(&lines);
        assert_eq!(migrated, lines);
    }

    #[test]
    fn migration_updates_old_header_version() {
        let lines = vec![
            Line::new(Record::Header { version: 1 }),
            Line::new(Record::Set {
                id: "old".into(),
                v: 1,
            }),
        ];
        let migrated = migrate_to_current(&lines);
        assert_eq!(migrated.len(), 2);
        assert_eq!(
            migrated[0].record(),
            &Record::Header {
                version: CURRENT_SCHEMA_VERSION
            }
        );
        assert_eq!(migrated[1].record(), lines[1].record());
    }
}
