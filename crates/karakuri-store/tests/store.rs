//! Integration tests for `Store` and the ndjson I/O, run against a
//! temporary directory — never the user's `library/`.

use std::fs;
use std::time::{Duration, SystemTime};

use karakuri_store::{project, Hash, Layer, Line, Record, Store, StoreError, Value};
use tempfile::tempdir;

fn param(layer: Layer, key: &str, value: f32) -> Record {
    Record::Param {
        layer,
        index: None,
        key: key.into(),
        value: Value::Scalar(value),
    }
}

#[test]
fn open_establishes_layout_and_is_idempotent() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("library");

    Store::open(&root).unwrap();
    // Calling it again must not fail or disturb anything already there.
    Store::open(&root).unwrap();

    assert!(root.is_dir());
    assert!(root.join("thumbnails").is_dir());
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
        Line::new(Record::Set {
            id: "drift_01".into(),
            v: 1,
        }),
        Line::new(Record::Slot {
            layer: Layer::L1,
            index: 0,
            name: None,
            proc_hash,
        }),
        Line::new(Record::Capacity {
            layer: Layer::L1,
            index: 0,
            value: 524288,
        }),
        Line::new(param(Layer::L1, "radius", 2.4)),
        Line::new(Record::Seed {
            stream: Layer::L1,
            index: 0,
            value: 19274,
        }),
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
        Line::new(Record::Set {
            id: "drift_01".into(),
            v: 1,
        }),
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

    let tick_count = records
        .iter()
        .filter(|r| matches!(r, Record::Tick { .. }))
        .count();
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
    assert_eq!(
        lines[1].as_str(),
        r#"{"t":"phrase","at":4.0,"marker":"drop"}"#
    );

    // Writing it back must not drop or corrupt the unknown line.
    store.write_set("with_unknown", &lines).unwrap();
    let contents = fs::read_to_string(&path).unwrap();
    assert_eq!(contents, original);
}

/// **A Set file may say that it composites, and `Store::write_set` takes it.**
///
/// The record this test is about is the layering — the one thing a Set knew
/// about itself that its file could not say, so a composited Set saved and
/// loaded back overdrew and `Set::select_renderer` came back with nothing to
/// select between.
///
/// **Written back byte for byte**, which is the claim the optional field
/// makes: a Set nobody has selected in writes the bare `{"t":"merge"}`, and a
/// `"live":null` on every one of them would be a changed file for a fact
/// nobody stated.
#[test]
fn write_set_accepts_a_merge_record() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    let proc_hash = Hash::of(b"proc p { kind L4 }");
    let lines = vec![
        Line::new(Record::Set {
            id: "morph_01".into(),
            v: 1,
        }),
        Line::new(Record::Slot {
            layer: Layer::L4,
            index: 0,
            name: None,
            proc_hash,
        }),
        Line::new(Record::Slot {
            layer: Layer::L4,
            index: 1,
            name: None,
            proc_hash,
        }),
        Line::new(Record::Merge { live: Some(1) }),
    ];
    store.write_set("morph_01", &lines).unwrap();

    let read_back = store.read_set("morph_01").unwrap();
    assert_eq!(
        read_back.last().unwrap().record(),
        &Record::Merge { live: Some(1) }
    );

    let contents = fs::read_to_string(dir.path().join("sets").join("morph_01.set.ndjson")).unwrap();
    assert!(
        contents.contains(r#"{"t":"merge","live":1}"#),
        "the merge did not go to disk as the line the spec prints: {contents}"
    );

    // And the same Set with nobody selected in it: the presence of the record
    // is the whole statement, so the line carries nothing else.
    let unselected = vec![
        Line::new(Record::Set {
            id: "morph_02".into(),
            v: 1,
        }),
        Line::new(Record::Merge { live: None }),
    ];
    store.write_set("morph_02", &unselected).unwrap();
    let contents = fs::read_to_string(dir.path().join("sets").join("morph_02.set.ndjson")).unwrap();
    assert!(
        contents.contains("{\"t\":\"merge\"}\n"),
        "an unselected merge wrote something beyond its own presence: {contents}"
    );
}

/// **A `merge` line carrying a key this build does not know reads back as
/// itself and writes back with the key still on it.**
///
/// Both halves of the format's forward-compatibility rule, on the newest
/// record: an unknown `t` is ignored, and so is an unknown key inside a record
/// whose `t` *is* known. This record is the one that will meet the second
/// half first — `gain`, `opacity`, `blend` and `mask` are named in its doc as
/// the fields it does not have yet, so the build that grows them is the build
/// whose files this one has to read.
#[test]
fn a_merge_line_keeps_a_key_this_build_does_not_know() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    let path = dir.path().join("sets").join("from_later.set.ndjson");
    let original = concat!(
        r#"{"t":"set","id":"morph_01","v":1}"#,
        "\n",
        // What a build that had grown the per-input row would write.
        r#"{"t":"merge","live":1,"gain":0.5}"#,
        "\n",
    );
    fs::write(&path, original).unwrap();

    let lines = store.read_set("from_later").unwrap();
    assert_eq!(lines.len(), 2, "a line was dropped rather than passed over");
    assert_eq!(
        lines[1].record(),
        &Record::Merge { live: Some(1) },
        "a known `t` carrying an unknown key did not read back as itself"
    );

    // And the key survives the write, because a `Line` writes the text it came
    // from — the same thing that keeps an unknown `t` verbatim.
    store.write_set("from_later", &lines).unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), original);
}

/// **A projection keeps the layering and still drops the selection.**
///
/// The end-to-end half of `project`'s own test. A `merge` is what the Set
/// *is*, so it folds and is written; a `select` is addressed to a **deck
/// slot**, and nothing in a session says which slots composite nor which deck
/// slot the Set at the head of the stream was played in — so a projection
/// cannot tell whether a selection it meets is about the Set it is writing,
/// and folding one in would be guessing.
///
/// It matters here rather than only in the unit test because a `select` that
/// reached this far would not merely be wrong, it would fail the save:
/// `write_set` refuses a record that is not Set state.
#[test]
fn save_session_as_set_keeps_a_merge_and_drops_a_selection() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    let session = vec![
        Line::new(Record::Set {
            id: "morph_01".into(),
            v: 1,
        }),
        // The head's, because a session stream begins with a Set file's
        // records — so this is a stream saying as much about layering as a
        // stream can say.
        Line::new(Record::Merge { live: None }),
        Line::new(Record::Tick { steps: 1 }),
        Line::new(Record::Select {
            slot: 0,
            renderer: 1,
            start: 8.0,
        }),
        Line::new(Record::Tick { steps: 1 }),
    ];

    store.save_session_as_set("morph_01", &session).unwrap();
    let set = store.read_set("morph_01").unwrap();

    let records: Vec<_> = set.iter().map(|l| l.record().clone()).collect();
    assert_eq!(
        records,
        vec![
            Record::Set {
                id: "morph_01".into(),
                v: 1
            },
            Record::Merge { live: None },
        ],
        "the projection did not keep the layering and drop the selection"
    );
}

#[test]
fn write_set_rejects_a_tick() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    let lines = vec![
        Line::new(Record::Set {
            id: "drift_01".into(),
            v: 1,
        }),
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
        Line::new(Record::Set {
            id: "drift_01".into(),
            v: 1,
        }),
        Line::new(Record::Slot {
            layer: Layer::L1,
            index: 0,
            name: None,
            proc_hash,
        }),
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

/// **A metadata record is refused from a Set file**, and refused in its own
/// words.
///
/// `param_decl` and `param` are one letter apart in a hand-edited file and
/// nothing alike in meaning: one says a knob exists and what it may be turned
/// between, the other says what this Set turned it to. A Set file carrying the
/// first would describe an artifact rather than a Set — and a *reader* meeting
/// one would take a declaration for a value.
///
/// **The gate is [`Record::is_metadata`] and not `!is_set_state`**, which is
/// what the separate error variant is here to pin: these records are not Set
/// state and would fall through the older check, but the sentence it says is
/// about time, and telling an operator a `capacity_decl` was rejected for
/// carrying time sends them looking in the wrong place.
///
/// All four, because the refusal is about the group and a check written against
/// one of them would pass while the other three went to disk.
#[test]
fn write_set_rejects_an_artifacts_metadata() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    for (name, record) in [
        (
            "meta",
            Record::Meta {
                hash: Hash::of(b"proc p { kind L1 }"),
                name: "p".into(),
                kind: Layer::L1,
                v: 1,
            },
        ),
        (
            "param_decl",
            Record::ParamDecl {
                key: "radius".into(),
                ty: "float".into(),
                min: 0.1,
                max: 8.0,
                default: Some(2.0),
            },
        ),
        (
            "capacity_decl",
            Record::CapacityDecl {
                min: 65536,
                max: 1048576,
                default: 262144,
            },
        ),
        (
            "emit",
            Record::Emit {
                attrs: vec!["position".into()],
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
            Err(StoreError::MetaInSet { index }) => assert_eq!(index, 1),
            other => panic!("a `{name}` record in a Set file was accepted: {other:?}"),
        }
        assert!(!dir
            .path()
            .join("sets")
            .join(format!("{name}.set.ndjson"))
            .exists());
    }
}

/// **Both halves of the metadata decoder's forward-compatibility rule**, on a
/// file the store did not write.
///
/// `docs/ir-spec.md` states them for this file separately from the Set file's,
/// because it is a separate list read by a separate decoder: an unknown `t` is
/// ignored, and so is an unknown key inside a record whose `t` *is* known.
///
/// The second is the one a *removed* key needs, and the specification names the
/// case — `perf` carried a `bytes_per_element` and does not any more. `perf`
/// itself has no producer yet, so the removed key is put where a known `t` can
/// hold it: a `param_decl` written by a build that recorded something this one
/// does not. Reading it must yield the declaration and pass over the key,
/// rather than failing the whole file over a field nobody asks for.
///
/// Ignoring costs nothing here because the file is derived: a key that still
/// means something comes back on the next regeneration.
#[test]
fn a_metadata_file_survives_an_unknown_t_and_an_unknown_key() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    let hash = store.put_artifact(b"proc drift_shell { kind L1 }").unwrap();
    let path = dir.path().join(format!("{}.meta.ndjson", hash.short(64)));
    let original = format!(
        concat!(
            r#"{{"t":"meta","hash":"{hash}","name":"drift_shell","kind":"L1","v":1}}"#,
            "\n",
            // A `t` this build has never heard of — `origin` is specified and
            // has no producer, so a file written by something that generates
            // procedures will carry exactly this.
            r#"{{"t":"origin","prompt":"organic drifting shell","seed":19274}}"#,
            "\n",
            // A known `t` carrying a key this build does not know, which is the
            // removed-key case.
            r#"{{"t":"param_decl","key":"radius","type":"float","min":0.1,"max":8.0,"#,
            r#""default":2.0,"units":"metres"}}"#,
            "\n",
        ),
        hash = hash
    );
    fs::write(&path, &original).unwrap();

    let lines = store.read_meta(&hash).unwrap();
    assert_eq!(lines.len(), 3, "a line was dropped rather than passed over");
    assert_eq!(
        lines[1].record(),
        &Record::Unknown,
        "an unrecognised `t` was not ignored"
    );
    assert_eq!(
        lines[2].record(),
        &Record::ParamDecl {
            key: "radius".into(),
            ty: "float".into(),
            min: 0.1,
            max: 8.0,
            default: Some(2.0),
        },
        "a known `t` carrying an unknown key did not read back as itself"
    );

    // And writing it back preserves both — the unknown line verbatim, because
    // re-serialising `Record::Unknown` would fabricate a line that never
    // existed, and the known one without the key it never held.
    store.write_meta(&hash, &lines).unwrap();
    let contents = fs::read_to_string(&path).unwrap();
    assert!(
        contents.contains(r#"{"t":"origin","prompt":"organic drifting shell","seed":19274}"#),
        "the unknown line did not survive the write: {contents}"
    );
}

// ---------------------------------------------------------------------------
// Enumeration: `list_sets` and `list_artifacts`.
// ---------------------------------------------------------------------------

/// The smallest thing that is a legal Set file, for tests that care about the
/// file's name and not about what is in it.
fn a_set() -> Vec<Line> {
    vec![Line::new(Record::Set {
        id: "drift_01".into(),
        v: 1,
    })]
}

/// **The order is by id, and it is the same order twice.**
///
/// `read_dir` hands entries back in whatever order the filesystem stored them,
/// which on APFS is creation order and elsewhere is a hash bucket walk — so a
/// listing that forwards it looks stable on the machine it was written on and
/// reorders itself on someone else's. The ids here are written in an order that
/// is neither sorted nor reverse-sorted, so an implementation that forgot to
/// sort would have to be lucky twice to pass.
#[test]
fn list_sets_orders_by_id_and_repeats_that_order() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    for id in ["mike", "zulu", "drift_01", "01_drift", "alpha"] {
        store.write_set(id, &a_set()).unwrap();
    }

    let ids: Vec<String> = store
        .list_sets()
        .unwrap()
        .into_iter()
        .map(|e| e.id)
        .collect();
    assert_eq!(ids, ["01_drift", "alpha", "drift_01", "mike", "zulu"]);

    // Unchanged store, same sequence — including the entries' write times.
    assert_eq!(store.list_sets().unwrap(), store.list_sets().unwrap());
}

/// **The time reported is the file's, and it is the axis "the one I saved
/// last" is asked along.**
///
/// The two mtimes are stamped rather than observed, because a test that writes
/// two files and asserts the second is newer asserts nothing on a filesystem
/// whose clock ticks once a second. Stamping also puts the recency order at
/// odds with the id order, which is the point: the listing comes back by id,
/// and one `sort_by_key` on the field turns it into the answer an operator
/// wanted.
#[test]
fn list_sets_carries_the_time_the_file_was_written() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    // `alpha` sorts first and was saved last.
    let older = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000_000);
    let newer = SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    for (id, stamp) in [("alpha", newer), ("zulu", older)] {
        store.write_set(id, &a_set()).unwrap();
        let path = dir.path().join("sets").join(format!("{id}.set.ndjson"));
        fs::File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_times(fs::FileTimes::new().set_modified(stamp))
            .unwrap();
    }

    let listed = store.list_sets().unwrap();
    assert_eq!(
        listed.iter().map(|e| e.written).collect::<Vec<_>>(),
        [newer, older],
        "the reported time is not the file's modification time"
    );

    let mut by_recency = listed;
    by_recency.sort_by_key(|e| e.written);
    assert_eq!(
        by_recency.last().unwrap().id,
        "alpha",
        "sorting the listing by its own time did not recover the save order"
    );
}

/// **Only `<id>.set.ndjson` is a Set.** Everything else that can end up in that
/// directory — a `.tmp` from a write that died, an editor's backup, a
/// subdirectory — is somebody else's file, and reporting one as a Set means
/// handing back an id [`Store::read_set`] cannot open. The `.tmp` case is not
/// hypothetical: `write_atomic` puts one there under exactly that name for the
/// length of every write.
#[test]
fn list_sets_skips_what_the_layout_does_not_claim() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let sets = dir.path().join("sets");

    store.write_set("drift_01", &a_set()).unwrap();
    fs::write(sets.join("drift_01.set.ndjson.tmp"), b"half a write").unwrap();
    fs::write(sets.join("notes.txt"), b"reminder").unwrap();
    fs::write(sets.join("drift_02.ndjson"), b"wrong suffix").unwrap();
    fs::write(sets.join("drift_03.set"), b"wrong suffix").unwrap();
    fs::create_dir(sets.join("archive.set.ndjson")).unwrap();

    let ids: Vec<String> = store
        .list_sets()
        .unwrap()
        .into_iter()
        .map(|e| e.id)
        .collect();
    assert_eq!(ids, ["drift_01"]);
}

/// The same claim for artifacts, plus the ordering that makes a listing
/// reproducible. Expected order is built from the hex spellings rather than
/// from `Hash`'s own `Ord`, so the test says "ascending hex" rather than
/// agreeing with whatever the implementation sorted by.
#[test]
fn list_artifacts_orders_by_hash_and_repeats_that_order() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    let mut expected: Vec<String> = ["proc a {}", "proc b {}", "proc c {}", "proc d {}"]
        .iter()
        .map(|s| store.put_artifact(s.as_bytes()).unwrap().short(64))
        .collect();
    expected.sort();

    let listed: Vec<String> = store
        .list_artifacts()
        .unwrap()
        .into_iter()
        .map(|e| e.hash.short(64))
        .collect();
    assert_eq!(listed, expected);
    assert_eq!(
        store.list_artifacts().unwrap(),
        store.list_artifacts().unwrap()
    );
}

/// **An artifact without a card is an ordinary artifact.**
///
/// `read_meta` already says so, and this is the same statement made in bulk:
/// the uncarded one is listed, not skipped and not an error, and the flag is
/// the difference. A caller that had to discover this by calling `read_meta`
/// per artifact would be reading an ordinary answer out of an error variant,
/// once per artifact, opening a file each time to do it.
#[test]
fn list_artifacts_flags_the_carded_and_the_uncarded() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    let carded = store.put_artifact(b"proc carded { kind L1 }").unwrap();
    let bare = store.put_artifact(b"proc bare { kind L1 }").unwrap();
    store
        .write_meta(
            &carded,
            &[Line::new(Record::Meta {
                hash: carded,
                name: "carded".into(),
                kind: Layer::L1,
                v: 1,
            })],
        )
        .unwrap();

    let listed = store.list_artifacts().unwrap();
    assert_eq!(listed.len(), 2, "the uncarded artifact was dropped");
    let flag = |h| {
        listed
            .iter()
            .find(|e| e.hash == h)
            .unwrap_or_else(|| panic!("{h} missing from the listing"))
            .has_meta
    };
    assert!(flag(carded), "an artifact with a card was reported bare");
    assert!(!flag(bare), "an artifact with no card was reported carded");
}

/// **A card is not an artifact, and neither is a directory.**
///
/// Both live under the same root as the `.kir` files, so both are in front of
/// any implementation that lists that directory. The stray card is the case
/// `write_meta` documents — it writes one without checking the artifact exists
/// — and counting it would put a hash in the library that `get_artifact` cannot
/// serve. The directory is named `<hash>.kir` on purpose: the suffix and the
/// hash both check out, and it is still not an artifact.
#[test]
fn list_artifacts_reports_neither_a_stray_card_nor_a_directory() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    let real = store.put_artifact(b"proc real { kind L1 }").unwrap();
    let never_put = Hash::of(b"proc never_put { kind L1 }");
    store.write_meta(&never_put, &[]).unwrap();
    fs::create_dir(
        dir.path()
            .join(format!("{}.kir", Hash::of(b"a directory").short(64))),
    )
    .unwrap();

    let listed = store.list_artifacts().unwrap();
    assert_eq!(
        listed.iter().map(|e| e.hash).collect::<Vec<_>>(),
        [real],
        "something that is not an artifact was listed as one"
    );
}

/// **A name this store would not have written is not an artifact**, and none
/// of these may panic on the way to being ignored.
///
/// The uppercase case is the subtle one: it parses to a valid address, whose
/// `.kir` path is then the lowercase spelling — so listing it would report a
/// hash `get_artifact` immediately fails to find. The non-UTF-8 name is the one
/// that punishes an `unwrap` on `to_str`, and nothing stops a user from
/// creating it.
#[test]
fn list_artifacts_skips_names_this_store_would_not_have_written() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    let real = store.put_artifact(b"proc real { kind L1 }").unwrap();
    // Uppercase hex, under an address nothing was ever put at — writing the
    // real artifact's address in uppercase would land on the real artifact's
    // own file on a case-insensitive volume, which is most of macOS.
    let shouty = Hash::of(b"proc shouty { kind L1 }")
        .short(64)
        .to_uppercase();
    fs::write(dir.path().join(format!("{shouty}.kir")), b"upper case hex").unwrap();
    fs::write(dir.path().join("proc_drift_shell.kir"), b"not hex at all").unwrap();
    fs::write(
        dir.path().join(format!("{}.kir", "z".repeat(64))),
        b"64 non-hex",
    )
    .unwrap();
    fs::write(
        dir.path().join(format!("{}.kir", real.short(32))),
        b"too short",
    )
    .unwrap();
    fs::write(dir.path().join(".kir"), b"no stem").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        // Best-effort: APFS validates filenames and refuses this outright
        // (EILSEQ), so on macOS the case cannot be staged at all and the
        // listing is left to prove the rest. On a filesystem that does allow
        // it — ext4, and every volume this store might be kept on over a
        // network share — the file lands and an `unwrap` on `to_str` dies here.
        let name = std::ffi::OsStr::from_bytes(b"\xff\xfe.kir");
        let _ = fs::write(dir.path().join(name), b"not utf-8");
    }

    let listed = store.list_artifacts().unwrap();
    assert_eq!(listed.iter().map(|e| e.hash).collect::<Vec<_>>(), [real]);
    // And what was listed is what the store can actually serve.
    assert_eq!(
        store.get_artifact(&listed[0].hash).unwrap(),
        b"proc real { kind L1 }"
    );
}

/// A store with nothing in it lists nothing — including the three
/// subdirectories `Store::open` just made, which sit in the artifact root and
/// are not artifacts.
#[test]
fn listing_an_empty_store_is_empty_and_not_an_error() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    assert_eq!(store.list_sets().unwrap(), []);
    assert_eq!(store.list_artifacts().unwrap(), []);
}

/// **A store whose directory went away is an error, not an empty library.**
///
/// The two answers look alike and mean opposite things: one says nothing is
/// kept, the other says we could not find out. An operator who is told the
/// first will generate the thing they already had.
#[test]
fn listing_a_removed_directory_is_an_error() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    store.write_set("drift_01", &a_set()).unwrap();
    store.put_artifact(b"proc p { kind L1 }").unwrap();

    fs::remove_dir_all(dir.path().join("sets")).unwrap();
    match store.list_sets() {
        Err(StoreError::Io(e)) => assert_eq!(e.kind(), std::io::ErrorKind::NotFound),
        other => panic!("expected an io error for a missing sets/, got {other:?}"),
    }

    fs::remove_dir_all(dir.path()).unwrap();
    match store.list_artifacts() {
        Err(StoreError::Io(e)) => assert_eq!(e.kind(), std::io::ErrorKind::NotFound),
        other => panic!("expected an io error for a missing root, got {other:?}"),
    }
}

/// **Listing reads names, never contents.**
///
/// A library of two thousand Sets should cost one directory read, and the way
/// that claim is checked without timing anything is to make the contents
/// unreadable: every file here fails to parse, and the listing has to come back
/// whole anyway. The `read_set` at the end is what keeps the test honest — it
/// proves the bytes really are unparsable, so the listing's success means it
/// never looked.
#[test]
fn listing_reads_names_and_not_contents() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    fs::write(
        dir.path().join("sets").join("garbage.set.ndjson"),
        b"this is not ndjson\n",
    )
    .unwrap();
    let hash = store.put_artifact(b"proc p { kind L1 }").unwrap();
    fs::write(
        dir.path().join(format!("{}.meta.ndjson", hash.short(64))),
        b"neither is this\n",
    )
    .unwrap();

    let sets = store.list_sets().unwrap();
    assert_eq!(
        sets.iter().map(|e| e.id.as_str()).collect::<Vec<_>>(),
        ["garbage"]
    );
    let artifacts = store.list_artifacts().unwrap();
    assert_eq!(artifacts.len(), 1);
    assert!(
        artifacts[0].has_meta,
        "an unparsable card is still a card — listing does not read it"
    );

    assert!(
        matches!(store.read_set("garbage"), Err(StoreError::Record { .. })),
        "the file parsed after all, so listing it proved nothing"
    );
}
