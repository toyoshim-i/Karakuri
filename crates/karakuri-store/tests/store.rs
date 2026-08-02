//! Integration tests for `Store` and the ndjson I/O, run against a
//! temporary directory — never the user's `library/`.

use std::fs;

use karakuri_store::{project, Hash, Layer, Line, Record, Store, StoreError, Value};
use tempfile::tempdir;

fn param(layer: Layer, key: &str, value: f32) -> Record {
    Record::Param { layer, key: key.into(), value: Value::Scalar(value) }
}

#[test]
fn open_establishes_layout_and_is_idempotent() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("library");

    Store::open(&root).unwrap();
    // Calling it again must not fail or disturb anything already there.
    Store::open(&root).unwrap();

    assert!(root.is_dir());
    assert!(root.join("previews").is_dir());
    assert!(root.join("sets").is_dir());
    assert!(root.join("sessions").is_dir());
}

#[test]
fn put_get_round_trips() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    let source = b"proc drift_shell { kind L1 }";
    let hash = store.put_artifact(source).unwrap();
    let back = store.get_artifact(&hash).unwrap();

    assert_eq!(back, source);
}

#[test]
fn put_is_content_addressed_and_idempotent() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    let source = b"proc drift_shell { kind L1 }";
    let hash1 = store.put_artifact(source).unwrap();
    let hash2 = store.put_artifact(source).unwrap();
    assert_eq!(hash1, hash2);

    // The file on disk must still hold exactly the original bytes: a
    // second put must not rewrite (or corrupt) what is there.
    let back = store.get_artifact(&hash1).unwrap();
    assert_eq!(back, source);

    // Different content gets a different address.
    let other_hash = store.put_artifact(b"proc other { kind L1 }").unwrap();
    assert_ne!(hash1, other_hash);
}

#[test]
fn put_writes_directly_under_root_by_bare_hex() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let hash = store.put_artifact(b"proc p { kind L1 }").unwrap();

    let expected = dir.path().join(format!("{}.kir", hash.short(64)));
    assert!(expected.is_file(), "expected artifact at {expected:?}");
}

#[test]
fn get_missing_artifact_is_not_found() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    let hash = Hash::of(b"never written");
    match store.get_artifact(&hash) {
        Err(StoreError::NotFound(h)) => assert_eq!(h, hash),
        other => panic!("expected NotFound, got {other:?}"),
    }
}

#[test]
fn set_file_round_trips() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    let proc_hash = Hash::of(b"proc p { kind L1 }");
    let lines = vec![
        Line::new(Record::Set { id: "drift_01".into(), v: 1 }),
        Line::new(Record::Slot { layer: Layer::L1, proc_hash }),
        Line::new(Record::Capacity { layer: Layer::L1, value: 524288 }),
        Line::new(param(Layer::L1, "radius", 2.4)),
        Line::new(Record::Seed { stream: Layer::L1, value: 19274 }),
    ];

    store.write_set("drift_01", &lines).unwrap();
    let read_back = store.read_set("drift_01").unwrap();

    let records: Vec<_> = read_back.iter().map(|l| l.record().clone()).collect();
    let expected: Vec<_> = lines.iter().map(|l| l.record().clone()).collect();
    assert_eq!(records, expected);
}

#[test]
fn session_stream_round_trips_including_ticks() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    let lines = vec![
        Line::new(Record::Set { id: "drift_01".into(), v: 1 }),
        Line::new(Record::Tick { steps: 1 }),
        Line::new(Record::Tick { steps: 1 }),
        Line::new(param(Layer::L1, "radius", 2.6)),
        Line::new(Record::Tick { steps: 1 }),
    ];

    store.write_session("live_2026", &lines).unwrap();
    let read_back = store.read_session("live_2026").unwrap();

    let records: Vec<_> = read_back.iter().map(|l| l.record().clone()).collect();
    let expected: Vec<_> = lines.iter().map(|l| l.record().clone()).collect();
    assert_eq!(records, expected);

    let tick_count = records.iter().filter(|r| matches!(r, Record::Tick { .. })).count();
    assert_eq!(tick_count, 3);
}

#[test]
fn malformed_line_reports_its_line_number() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    // Write a file by hand: two good lines around a broken one.
    let path = dir.path().join("sessions").join("broken.ndjson");
    fs::write(
        &path,
        concat!(
            r#"{"t":"set","id":"drift_01","v":1}"#,
            "\n",
            r#"{"t":"tick","steps":"#, // truncated / malformed JSON
            "\n",
            r#"{"t":"tick","steps":1}"#,
            "\n",
        ),
    )
    .unwrap();

    match store.read_session("broken") {
        Err(StoreError::Record { line, .. }) => assert_eq!(line, 2),
        other => panic!("expected a Record error on line 2, got {other:?}"),
    }
}

#[test]
fn unknown_records_survive_a_read_write_round_trip() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    let path = dir.path().join("sets").join("with_unknown.set.ndjson");
    let original = concat!(
        r#"{"t":"set","id":"drift_01","v":1}"#,
        "\n",
        r#"{"t":"phrase","at":4.0,"marker":"drop"}"#, // a record type this store does not know
        "\n",
        r#"{"t":"seed","stream":"L1","value":19274}"#,
        "\n",
    );
    fs::write(&path, original).unwrap();

    let lines = store.read_set("with_unknown").unwrap();
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[1].record(), &Record::Unknown);
    // The unknown line's original text is preserved verbatim, not
    // reduced to `{"t":"unknown"}`.
    assert_eq!(lines[1].as_str(), r#"{"t":"phrase","at":4.0,"marker":"drop"}"#);

    // Writing it back must not drop or corrupt the unknown line.
    store.write_set("with_unknown", &lines).unwrap();
    let contents = fs::read_to_string(&path).unwrap();
    assert_eq!(contents, original);
}

#[test]
fn write_set_rejects_a_tick() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    let lines = vec![
        Line::new(Record::Set { id: "drift_01".into(), v: 1 }),
        Line::new(Record::Tick { steps: 1 }),
    ];

    match store.write_set("bad", &lines) {
        Err(StoreError::TickInSet { index }) => assert_eq!(index, 1),
        other => panic!("expected TickInSet, got {other:?}"),
    }

    // And nothing should have been written.
    assert!(!dir.path().join("sets").join("bad.set.ndjson").exists());
}

#[test]
fn save_session_as_set_projects_and_persists() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    let proc_hash = Hash::of(b"proc p { kind L1 }");
    let session = vec![
        Line::new(Record::Set { id: "drift_01".into(), v: 1 }),
        Line::new(Record::Slot { layer: Layer::L1, proc_hash }),
        Line::new(param(Layer::L1, "radius", 2.0)),
        Line::new(Record::Tick { steps: 1 }),
        Line::new(Record::Tick { steps: 1 }),
        Line::new(param(Layer::L1, "radius", 2.6)), // repeated edit to the same key
        Line::new(Record::Tick { steps: 1 }),
    ];

    store.save_session_as_set("drift_01", &session).unwrap();
    let set = store.read_set("drift_01").unwrap();

    // No ticks in the persisted Set.
    assert!(set.iter().all(|l| l.record().is_set_state()));
    // Last write wins: one radius record, holding the later value.
    let radius_records: Vec<_> = set
        .iter()
        .filter(|l| matches!(l.record(), Record::Param { key, .. } if key == "radius"))
        .collect();
    assert_eq!(radius_records.len(), 1);
    assert_eq!(radius_records[0].record(), &param(Layer::L1, "radius", 2.6));
}

#[test]
fn projection_folds_repeated_edits_last_write_wins() {
    let session = vec![
        Line::new(param(Layer::L1, "radius", 1.0)),
        Line::new(param(Layer::L1, "radius", 2.0)),
        Line::new(param(Layer::L1, "radius", 3.0)),
        Line::new(Record::Tick { steps: 1 }),
    ];

    let set = project(&session);
    assert_eq!(set.len(), 1);
    assert_eq!(set[0].record(), &param(Layer::L1, "radius", 3.0));
}

/// The same rule for the two records audio added. A Set file carries no time,
/// and both of these are what one frame measured or decided — so the writer
/// refuses them for the same reason it refuses a tick, rather than for the
/// narrower reason its error variant is named after.
#[test]
fn write_set_rejects_an_audio_frame_and_a_tempo_correction() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    for (name, record) in [
        (
            "audio",
            Record::Audio {
                energy: 0.4,
                onset: 0.0,
                bands: vec![0.1, 0.2],
                confidence: 1.0,
            },
        ),
        (
            "tempo",
            Record::Tempo {
                bpm: 128.0,
                shift: 0.0,
                confidence: 0.9,
            },
        ),
    ] {
        let lines = vec![
            Line::new(Record::Set {
                id: "drift_01".into(),
                v: 1,
            }),
            Line::new(record),
        ];
        match store.write_set(name, &lines) {
            Err(StoreError::TickInSet { index }) => assert_eq!(index, 1),
            other => panic!("a `{name}` record in a Set file was accepted: {other:?}"),
        }
        assert!(!dir
            .path()
            .join("sets")
            .join(format!("{name}.set.ndjson"))
            .exists());
    }
}
