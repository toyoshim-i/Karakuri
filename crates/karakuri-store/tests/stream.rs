//! Integration tests for zero-allocation stream reading and schema versioning.

use std::fs;

use karakuri_store::record::{DeckSlot, CURRENT_SCHEMA_VERSION};
use karakuri_store::{
    detect_version, migrate_to_current, Line, MmapStreamReader, Record, StoreError,
};
use tempfile::tempdir;

#[test]
fn record_header_serialization_and_deserialization() {
    let header = Record::Header { version: 2 };
    let json = serde_json::to_string(&header).expect("Header serializes");
    assert_eq!(json, r#"{"t":"header","version":2}"#);

    let deserialized: Record = serde_json::from_str(&json).expect("Header deserializes");
    assert_eq!(deserialized, header);
    assert_eq!(CURRENT_SCHEMA_VERSION, 2);
}

#[test]
fn legacy_unversioned_detection_and_migration() {
    let legacy_lines = vec![
        Line::new(Record::Set {
            id: "legacy_session".into(),
            v: 1,
        }),
        Line::new(Record::Gain {
            slot: DeckSlot(0),
            value: 0.5,
        }),
        Line::new(Record::Tick { steps: 1 }),
    ];

    // Legacy unversioned detected as version 1
    assert_eq!(detect_version(&legacy_lines), 1);

    // Automated migration to current schema version
    let migrated = migrate_to_current(&legacy_lines);
    assert_eq!(detect_version(&migrated), CURRENT_SCHEMA_VERSION);
    assert_eq!(migrated.len(), 4);
    assert_eq!(
        migrated[0].record(),
        &Record::Header {
            version: CURRENT_SCHEMA_VERSION
        }
    );
    assert_eq!(migrated[1].record(), legacy_lines[0].record());
    assert_eq!(migrated[2].record(), legacy_lines[1].record());
    assert_eq!(migrated[3].record(), legacy_lines[2].record());

    // Migration is idempotent if already version 2
    let migrated_again = migrate_to_current(&migrated);
    assert_eq!(migrated_again, migrated);
}

#[test]
fn mmap_stream_reader_reads_session_and_indexes_ticks() {
    let dir = tempdir().unwrap();
    let session_path = dir.path().join("test_session.ndjson");

    let content = concat!(
        r#"{"t":"header","version":2}"#,
        "\n",
        r#"{"t":"set","id":"session_alpha","v":1}"#,
        "\n",
        r#"{"t":"slot","layer":"L1","index":0,"proc":"sha256:0000000000000000000000000000000000000000000000000000000000000000"}"#,
        "\n",
        r#"{"t":"tick","steps":1,"timestamp":1000}"#,
        "\n",
        r#"{"t":"gain","slot":0,"value":0.75}"#,
        "\n",
        r#"{"t":"tick","steps":2,"timestamp":1033}"#,
        "\n",
        r#"{"t":"tick","steps":1}"#,
        "\n",
    );

    fs::write(&session_path, content).unwrap();

    let reader = MmapStreamReader::open(&session_path).expect("open session with mmap");
    assert!(!reader.is_empty());
    assert_eq!(reader.len(), content.len());
    assert_eq!(reader.schema_version(), 2);

    // Zero-allocation line slicing
    let raw_lines: Vec<&[u8]> = reader.lines_raw().collect();
    assert_eq!(raw_lines.len(), 7);
    let str_lines: Vec<&str> = reader.lines().collect();
    assert_eq!(str_lines.len(), 7);
    assert_eq!(str_lines[0], r#"{"t":"header","version":2}"#);
    assert_eq!(str_lines[3], r#"{"t":"tick","steps":1,"timestamp":1000}"#);

    // Fast tick scanning / indexing
    let ticks = reader.scan_ticks();
    assert_eq!(ticks.len(), 3);

    // First tick
    assert_eq!(ticks[0].line_number, 4);
    assert_eq!(ticks[0].tick_index, 0);
    assert_eq!(ticks[0].steps, 1);
    assert_eq!(ticks[0].total_steps, 1);
    assert_eq!(ticks[0].timestamp, Some(1000));
    assert_eq!(
        &reader.as_bytes()[ticks[0].byte_offset..ticks[0].byte_offset + ticks[0].byte_len],
        br#"{"t":"tick","steps":1,"timestamp":1000}"#
    );

    // Second tick
    assert_eq!(ticks[1].line_number, 6);
    assert_eq!(ticks[1].tick_index, 1);
    assert_eq!(ticks[1].steps, 2);
    assert_eq!(ticks[1].total_steps, 3);
    assert_eq!(ticks[1].timestamp, Some(1033));
    assert_eq!(
        &reader.as_bytes()[ticks[1].byte_offset..ticks[1].byte_offset + ticks[1].byte_len],
        br#"{"t":"tick","steps":2,"timestamp":1033}"#
    );

    // Third tick
    assert_eq!(ticks[2].line_number, 7);
    assert_eq!(ticks[2].tick_index, 2);
    assert_eq!(ticks[2].steps, 1);
    assert_eq!(ticks[2].total_steps, 4);
    assert_eq!(ticks[2].timestamp, None);
    assert_eq!(
        &reader.as_bytes()[ticks[2].byte_offset..ticks[2].byte_offset + ticks[2].byte_len],
        br#"{"t":"tick","steps":1}"#
    );

    // Full records iterator directly from buffer
    let records: Result<Vec<Record>, StoreError> = reader.records().collect();
    let records = records.expect("all lines decode successfully");
    assert_eq!(records.len(), 7);
    assert_eq!(records[0], Record::Header { version: 2 });
    assert_eq!(
        records[1],
        Record::Set {
            id: "session_alpha".into(),
            v: 1
        }
    );
    assert_eq!(records[3], Record::Tick { steps: 1 });
    assert_eq!(
        records[4],
        Record::Gain {
            slot: DeckSlot(0),
            value: 0.75
        }
    );
    assert_eq!(records[5], Record::Tick { steps: 2 });
    assert_eq!(records[6], Record::Tick { steps: 1 });
}

#[test]
fn mmap_stream_reader_handles_empty_file() {
    let dir = tempdir().unwrap();
    let empty_path = dir.path().join("empty.ndjson");
    fs::write(&empty_path, b"").unwrap();

    let reader = MmapStreamReader::open(&empty_path).expect("open empty file");
    assert!(reader.is_empty());
    assert_eq!(reader.len(), 0);
    assert_eq!(reader.schema_version(), 1);
    assert_eq!(reader.lines_raw().count(), 0);
    assert_eq!(reader.lines().count(), 0);
    assert_eq!(reader.scan_ticks().len(), 0);
    assert_eq!(reader.records().count(), 0);
}

#[test]
fn mmap_stream_reader_reports_malformed_record() {
    let dir = tempdir().unwrap();
    let broken_path = dir.path().join("broken.ndjson");
    fs::write(
        &broken_path,
        b"{\"t\":\"header\",\"version\":2}\n{\"t\":\"tick\",\"steps\":\n",
    )
    .unwrap();

    let reader = MmapStreamReader::open(&broken_path).unwrap();
    let mut iter = reader.records();
    assert_eq!(iter.next().unwrap().unwrap(), Record::Header { version: 2 });
    match iter.next().unwrap() {
        Err(StoreError::Record { line, .. }) => assert_eq!(line, 2),
        other => panic!("expected record error on line 2, got {other:?}"),
    }
}
