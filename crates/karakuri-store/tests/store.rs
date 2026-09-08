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
    // **Established even though nothing has written one**, so an operator
    // looking for what a model kept meets an empty directory rather than a
    // missing one — which is itself the answer.
    assert!(root.join("sandbox").is_dir());
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

    let path = dir.path().join("sets").join("with_unknown.kbset");
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

    let contents = fs::read_to_string(dir.path().join("sets").join("morph_01.kbset")).unwrap();
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
    let contents = fs::read_to_string(dir.path().join("sets").join("morph_02.kbset")).unwrap();
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

    let path = dir.path().join("sets").join("from_later.kbset");
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
    assert!(!dir.path().join("sets").join("bad.kbset").exists());
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
            .join(format!("{name}.kbset"))
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
            .join(format!("{name}.kbset"))
            .exists());
    }
}

/// **An authoring file's `part` is refused from a Set file**, in a third
/// sentence of its own, and nothing is written.
///
/// This is the check that makes `.kbset` mean something. Everything in
/// `sets/` is already resolved — that is what lets a swap happen on a frame
/// boundary with nothing left to look up — and a `part` names its `.kir` by a
/// relative path, so a Set file carrying one would resolve the filesystem at
/// the moment of the exchange, against a directory that may be somebody else's
/// machine's. See ADR-0231.
///
/// **The gate is `Record::is_authoring`, not `!is_set_state`**, which is what
/// the third error variant pins: a `part` falls through the older check too,
/// and the sentence that check says is about time. An operator who wrote an
/// authoring file into a store wants to be told which of the two forms they
/// have, not sent looking for a `tick` they did not write.
#[test]
fn write_set_rejects_an_authoring_files_part() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    let lines = vec![
        Line::new(Record::Set {
            id: "authored".into(),
            v: 1,
        }),
        Line::new(Record::Part {
            layer: Layer::L1,
            index: 0,
            name: None,
            path: "drift_shell.kir".into(),
        }),
    ];
    match store.write_set("authored", &lines) {
        Err(StoreError::PartInSet { index }) => assert_eq!(index, 1),
        other => panic!("a `part` record in a Set file was accepted: {other:?}"),
    }
    assert!(!dir.path().join("sets").join("authored.kbset").exists());

    // And the sentence names both forms, because which one the file is is the
    // whole of what the operator has to know.
    let said = store.write_set("authored", &lines).unwrap_err().to_string();
    assert!(said.contains(".kset"), "{said}");
    assert!(said.contains(".kbset"), "{said}");
}

/// **A `part` is dropped by the projection rather than passed through it.**
///
/// A session stream has no writer that puts one in, so meeting one means a
/// hand-assembled stream — and passing it through would fail a whole save at
/// `write_set` on a line that belongs to another file. Dropped, on the terms
/// every other foreign record here is dropped, and *not* folded onto the
/// `slot` at the same address: a fold that kept whichever came last would
/// resolve half the time to an unresolved path claiming to be an address.
#[test]
fn a_part_is_dropped_by_the_projection() {
    let hash = Hash::of(b"proc p { kind L1 }");
    let session = vec![
        Line::new(Record::Set {
            id: "s".into(),
            v: 1,
        }),
        Line::new(Record::Slot {
            layer: Layer::L1,
            index: 0,
            name: None,
            proc_hash: hash,
        }),
        Line::new(Record::Part {
            layer: Layer::L1,
            index: 0,
            name: None,
            path: "drift_shell.kir".into(),
        }),
    ];
    let folded = project::project(&session);
    assert_eq!(folded.len(), 2, "the part is gone: {folded:?}");
    assert!(
        matches!(folded[1].record(), Record::Slot { proc_hash, .. } if *proc_hash == hash),
        "and the slot at the same address kept its address: {:?}",
        folded[1].record()
    );
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
        let path = dir.path().join("sets").join(format!("{id}.kbset"));
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

/// **Only `<id>.kbset` is a Set.** Everything else that can end up in that
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
    fs::write(sets.join("drift_01.kbset.tmp"), b"half a write").unwrap();
    fs::write(sets.join("notes.txt"), b"reminder").unwrap();
    fs::write(sets.join("drift_02.ndjson"), b"wrong suffix").unwrap();
    fs::write(sets.join("drift_03.set"), b"wrong suffix").unwrap();
    fs::create_dir(sets.join("archive.kbset")).unwrap();

    let ids: Vec<String> = store
        .list_sets()
        .unwrap()
        .into_iter()
        .map(|e| e.id)
        .collect();
    assert_eq!(ids, ["drift_01"]);
}

/// **A file under `sets/` that does not carry `.kbset` has no id at all**, and
/// an id is the only route a Set has to a deck: nothing can ask for what cannot
/// be named.
///
/// That is the store's invariant rather than a tidiness rule. A swap happens on
/// a frame boundary and an over-budget Set rolls back on its own (P-0094),
/// which holds only because nothing is left to resolve at the moment of the
/// swap — so the two files here are exactly the two that would break it: an
/// authoring `.kset`, which names its parts by relative path and would send a
/// load walking the filesystem mid-swap, and a Set written by a build that
/// spelled the suffix `.set.ndjson`, whose contents nothing has ever checked
/// against the claim the new name makes.
///
/// **The `.set.ndjson` half is the one worth writing down, because it is
/// silent.** The file is still there, still readable text, and simply stops
/// being listed — no error, nothing to notice but an id that used to be in the
/// list. Nothing repairs it either, which this asserts: renaming a file the
/// store did not write would be guessing that its contents are already
/// resolved.
#[test]
fn a_file_without_the_suffix_has_no_id_and_no_route_to_a_deck() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let sets = dir.path().join("sets");

    store.write_set("kept", &a_set()).unwrap();

    // What an earlier build left behind: a perfectly parseable Set file, which
    // is the point — nothing here opens it to find that out.
    let earlier = sets.join("morph01.set.ndjson");
    fs::write(&earlier, "{\"t\":\"set\",\"id\":\"morph01\",\"v\":1}\n").unwrap();
    // And the form the extension exists to keep out of the store.
    let authoring = sets.join("beside_its_parts.kset");
    fs::write(
        &authoring,
        "{\"t\":\"set\",\"id\":\"beside_its_parts\",\"v\":1}\n",
    )
    .unwrap();

    let ids: Vec<String> = store
        .list_sets()
        .unwrap()
        .into_iter()
        .map(|e| e.id)
        .collect();
    assert_eq!(
        ids,
        ["kept"],
        "a file under `sets/` with no `.kbset` suffix was listed as a Set"
    );

    // No id, so no route to a deck: the name each file suggests reaches nothing.
    for id in ["morph01", "beside_its_parts"] {
        assert!(
            store.read_set(id).is_err(),
            "`{id}` read back, so the name on disk was an id after all"
        );
    }

    // And both are still where their owner put them.
    assert!(
        earlier.exists() && authoring.exists(),
        "a file was repaired away"
    );
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
        dir.path().join("sets").join("garbage.kbset"),
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

// # Arrangements
//
// The console's own shape, filed under a name the operator picked, in the
// fourth place under the store root. See ADR-0221.
//
// **These tests build a real `Layout` and put it through a real file**, rather
// than a JSON fixture that resembles one. The store's half is bytes and the
// format's half is `karakuri-layout`'s, and the claim worth checking is the one
// that spans them: what an operator arranged comes back. A fixture would prove
// only that `fs::write` works.

/// An arrangement with the two things that have historically not survived a
/// wire: an unbounded maximum, which JSON cannot spell, and a fold, which is
/// the flag a solo replaces. Both are in the console's own arrangement, so this
/// is a small stand-in for it rather than an exotic case.
fn an_arrangement() -> karakuri_layout::Layout {
    use karakuri_layout::{Rect, Spec};

    let mut l = karakuri_layout::Layout::new(Spec::row(
        4.0,
        vec![
            Spec::view("library").fixed(240.0).min(120.0).max(400.0),
            Spec::column(
                4.0,
                vec![
                    Spec::view("program").flex(3.0),
                    Spec::view("mixer").fixed(180.0),
                ],
            )
            .named("centre")
            .flex(1.0),
            Spec::view("inspector").fixed(300.0),
        ],
    ));
    l.set_viewport(Rect::new(0.0, 0.0, 1280.0, 720.0));
    l.solve();
    l
}

/// Every rectangle, in a fixed order, so two arrangements can be compared as
/// what they draw rather than as what they store.
fn drawn(l: &karakuri_layout::Layout) -> Vec<(Option<String>, karakuri_layout::Rect, (f32, f32))> {
    fn walk(
        l: &karakuri_layout::Layout,
        id: karakuri_layout::NodeId,
        out: &mut Vec<(Option<String>, karakuri_layout::Rect, (f32, f32))>,
    ) {
        out.push((l.name(id).map(str::to_string), l.rect(id), l.bounds(id)));
        for child in l.children(id).to_vec() {
            walk(l, child, out);
        }
    }
    let mut out = Vec::new();
    walk(l, l.root(), &mut out);
    out
}

#[test]
fn an_arrangement_round_trips_through_a_file() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    let mut before = an_arrangement();
    let mixer = before.find("mixer").unwrap();
    before.collapse(mixer);
    before.solve();

    store
        .write_arrangement("four_deck", serde_json::to_vec(&before).unwrap().as_slice())
        .unwrap();
    let text = store.read_arrangement("four_deck").unwrap();
    let after: karakuri_layout::Layout = serde_json::from_slice(&text).unwrap();

    assert_eq!(drawn(&before), drawn(&after), "the file changed the panel");
    assert_eq!(before.viewport(), after.viewport());
    assert!(
        after.is_collapsed(after.find("mixer").unwrap()),
        "a fold did not survive the file"
    );
    // The unbounded maxima are the reason this asserts `bounds` at all: JSON
    // has no spelling for an infinity, and one lost on the way out comes back
    // as a region that will not grow.
    assert_eq!(
        after.bounds(after.find("program").unwrap()).1,
        f32::INFINITY
    );
}

#[test]
fn a_soloed_arrangement_comes_back_soloed_and_still_undoes() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    let open = an_arrangement();
    let mut before = an_arrangement();
    let program = before.find("program").unwrap();
    before.solo(program);
    before.solve();

    store
        .write_arrangement("solo", serde_json::to_vec(&before).unwrap().as_slice())
        .unwrap();
    let mut after: karakuri_layout::Layout =
        serde_json::from_slice(&store.read_arrangement("solo").unwrap()).unwrap();

    assert!(after.is_soloed(), "the solo did not survive the file");
    after.unsolo();
    after.solve();
    assert_eq!(
        drawn(&after),
        drawn(&open),
        "the arrangement the solo was covering did not survive the file"
    );
}

#[test]
fn an_arrangement_is_the_bytes_it_was_given_and_nothing_appended() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    // One document, no trailing newline. A store that added one would be
    // editing a format it has just declared it does not parse.
    let written = br#"{"nodes":[],"root":0}"#;
    store.write_arrangement("bytes", written).unwrap();
    assert_eq!(store.read_arrangement("bytes").unwrap(), written);
    assert_eq!(
        fs::read(
            dir.path()
                .join("arrangements")
                .join("bytes.arrangement.json")
        )
        .unwrap(),
        written
    );
}

#[test]
fn a_name_saved_twice_is_the_second_arrangement() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    store.write_arrangement("desk", b"first").unwrap();
    store.write_arrangement("desk", b"second").unwrap();

    assert_eq!(store.read_arrangement("desk").unwrap(), b"second");
    assert_eq!(store.list_arrangements().unwrap().len(), 1);
}

#[test]
fn reading_an_arrangement_nobody_saved_names_it() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    match store.read_arrangement("four_deck") {
        Err(StoreError::NoArrangement(name)) => assert_eq!(name, "four_deck"),
        other => panic!("expected a named refusal, got {other:?}"),
    }
    // And it says the name back, because that is the whole of what an operator
    // who mistyped one needs.
    assert_eq!(
        store.read_arrangement("four-deck").unwrap_err().to_string(),
        "no arrangement named `four-deck`"
    );
}

#[test]
fn an_arrangement_and_a_set_of_the_same_name_are_two_files() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    store.write_set("tonight", &a_set()).unwrap();
    store.write_arrangement("tonight", b"{}").unwrap();

    assert_eq!(store.read_set("tonight").unwrap(), a_set());
    assert_eq!(store.read_arrangement("tonight").unwrap(), b"{}");
    assert_eq!(
        store
            .list_sets()
            .unwrap()
            .iter()
            .map(|e| e.id.as_str())
            .collect::<Vec<_>>(),
        ["tonight"]
    );
    assert_eq!(
        store
            .list_arrangements()
            .unwrap()
            .iter()
            .map(|e| e.name.as_str())
            .collect::<Vec<_>>(),
        ["tonight"]
    );
}

#[test]
fn list_arrangements_orders_by_name_and_skips_what_the_layout_does_not_claim() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    for name in ["wide", "four_deck", "a-b"] {
        store.write_arrangement(name, b"{}").unwrap();
    }
    let arrangements = dir.path().join("arrangements");
    // An editor's backup, a write that died, and somebody's directory.
    fs::write(arrangements.join("four_deck.arrangement.json~"), b"{}").unwrap();
    fs::write(arrangements.join("wide.arrangement.json.tmp"), b"{}").unwrap();
    fs::write(arrangements.join("notes.txt"), b"{}").unwrap();
    fs::create_dir(arrangements.join("old.arrangement.json")).unwrap();

    let listed: Vec<String> = store
        .list_arrangements()
        .unwrap()
        .into_iter()
        .map(|e| e.name)
        .collect();
    assert_eq!(listed, ["a-b", "four_deck", "wide"]);
    // Repeatable, which is the whole reason the key is the name rather than
    // the time three files written in one millisecond all share.
    assert_eq!(
        store.list_arrangements().unwrap(),
        store.list_arrangements().unwrap()
    );
}

#[test]
fn an_arrangement_carries_when_it_was_written_and_an_empty_store_carries_none() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    assert_eq!(store.list_arrangements().unwrap(), Vec::new());

    let before = SystemTime::now() - Duration::from_secs(2);
    store.write_arrangement("desk", b"{}").unwrap();
    let entry = store.list_arrangements().unwrap().pop().unwrap();
    assert_eq!(entry.name, "desk");
    assert!(entry.written > before, "the write time is not the file's");

    // The directory taken away under the store is an error, not "nothing kept"
    // — the caller asked what is there and there is no answer.
    fs::remove_dir_all(dir.path().join("arrangements")).unwrap();
    assert!(matches!(
        store.list_arrangements(),
        Err(StoreError::Io { .. })
    ));
}

#[test]
fn open_establishes_the_arrangements_directory_beside_the_other_three() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("library");
    Store::open(&root).unwrap();
    assert!(root.join("arrangements").is_dir());

    // A store written by a build that had no arrangements gains the directory
    // the first time this one opens it.
    fs::remove_dir_all(root.join("arrangements")).unwrap();
    Store::open(&root).unwrap();
    assert!(root.join("arrangements").is_dir());
}

/// **A Set written into the sandbox lands there and nowhere else**, and the
/// library is written by the other method.
///
/// The store half of
/// `docs/principles/0096-the-operators-library-is-written-by-an-operators-own-act.md`:
/// `sets/` is the operator's library, so a save asked for over MCP is written
/// with `write_sandbox_set` and the library is left exactly as it was.
///
/// **The negative control is the second half.** A test that only checked the
/// sandbox file appeared would pass against a writer that wrote both, which is
/// the failure that matters here — the operator's preset gone. So this asserts
/// what is *not* in `sets/` and then that the same id put through
/// `write_set` does land there, which is what stops the whole thing passing
/// against a store that writes everything into one directory.
#[test]
fn a_sandbox_set_is_written_to_sandbox_and_the_library_is_untouched() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    let lines = vec![Line::new(Record::Set {
        id: "night01".into(),
        v: 1,
    })];
    store.write_sandbox_set("night01", &lines).unwrap();

    assert!(
        dir.path().join("sandbox").join("night01.kbset").exists(),
        "a save asked for over MCP did not reach the sandbox"
    );
    assert!(
        !dir.path().join("sets").join("night01.kbset").exists(),
        "a save asked for over MCP wrote the operator's library"
    );
    assert!(
        matches!(store.read_set("night01"), Err(StoreError::Io(_))),
        "the library answered for a set only the sandbox holds"
    );

    // The control: the same id, the same lines, through the library's own
    // writer. Without this the test above would pass against a store whose two
    // writers were one.
    store.write_set("night01", &lines).unwrap();
    assert!(dir.path().join("sets").join("night01.kbset").exists());
}

/// **The sandbox refuses what the library refuses**, because what may be in a
/// Set file is a property of the format and not of the directory.
///
/// One of the three is enough to check that the shared scan is reached — the
/// three sentences and the order they are asked in are `write_set`'s own tests
/// above — and a `part` is the one chosen because it is the check that makes
/// `.kbset` mean *already resolved*, which a sandbox file claims by carrying
/// the extension.
#[test]
fn a_sandbox_set_refuses_a_part_the_way_the_library_does() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    let lines = vec![
        Line::new(Record::Set {
            id: "night01".into(),
            v: 1,
        }),
        Line::new(Record::Part {
            layer: Layer::L1,
            index: 0,
            name: None,
            path: "drift_shell.kir".into(),
        }),
    ];

    match store.write_sandbox_set("night01", &lines) {
        Err(StoreError::PartInSet { index }) => assert_eq!(index, 1),
        other => panic!("expected PartInSet, got {other:?}"),
    }
    assert!(!dir.path().join("sandbox").join("night01.kbset").exists());
}

/// **A store nobody has starred in answers with nothing, and is not an error.**
///
/// `favourites.json` is not established by `Store::open` the way the four
/// directories are, because an empty file and no file say the same thing and
/// only one of them is a write into a store somebody only wanted to read.
#[test]
fn an_unstarred_store_has_no_favourites_and_no_file() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    assert!(store.favourites().unwrap().is_empty());
    assert!(!dir.path().join(Store::FAVOURITES_FILE).exists());
}

/// **A star round-trips, and a second press of the same state writes nothing.**
///
/// The `false` back is the state already being the one asked for, which is what
/// keeps `Operation::SetFavourite` a state rather than a toggle: pressing
/// *star this* twice says the same thing twice, and the second one must not
/// touch the file.
#[test]
fn a_star_round_trips_and_the_same_state_twice_writes_nothing() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    store
        .write_set(
            "drift_night",
            &[Line::new(Record::Set {
                id: "drift_night".into(),
                v: 1,
            })],
        )
        .unwrap();

    assert!(store.set_favourite("drift_night", true).unwrap());
    assert_eq!(
        store.favourites().unwrap().into_iter().collect::<Vec<_>>(),
        vec!["drift_night".to_string()]
    );
    assert!(
        !store.set_favourite("drift_night", true).unwrap(),
        "starring what is already starred reported a write"
    );

    assert!(store.set_favourite("drift_night", false).unwrap());
    assert!(store.favourites().unwrap().is_empty());
    assert!(
        !store.set_favourite("drift_night", false).unwrap(),
        "unstarring what is not starred reported a write"
    );
}

/// **Starring a Set this store does not hold is refused with the id back**, and
/// nothing is written — the star is a control on a row, and a row is a Set the
/// store holds.
#[test]
fn starring_a_set_the_store_does_not_hold_is_refused() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    match store.set_favourite("nothing_here", true) {
        Err(StoreError::NoSet(id)) => assert_eq!(id, "nothing_here"),
        other => panic!("expected NoSet, got {other:?}"),
    }
    assert!(!dir.path().join(Store::FAVOURITES_FILE).exists());
}

/// **A stale mark survives the Set leaving and can be taken off**, which is the
/// whole of what happens to one.
///
/// Nothing in this program deletes or renames a Set, so a mark goes stale only
/// when a hand removes the file — and the answer is that the id stays, a
/// listing that intersects it with `list_sets` draws no row for it, and the
/// star can still be taken off without the Set coming back first.
#[test]
fn a_star_outlives_the_set_and_can_still_be_taken_off() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    store
        .write_set(
            "gone",
            &[Line::new(Record::Set {
                id: "gone".into(),
                v: 1,
            })],
        )
        .unwrap();
    store.set_favourite("gone", true).unwrap();

    fs::remove_file(dir.path().join("sets").join("gone.kbset")).unwrap();

    assert!(
        store.favourites().unwrap().contains("gone"),
        "a question pruned a mark, and a question writes nothing"
    );
    assert!(
        store.list_sets().unwrap().is_empty(),
        "the listing this is intersected with still holds the Set"
    );
    assert!(
        store.set_favourite("gone", false).unwrap(),
        "a stale mark could not be taken off without the Set coming back"
    );
    assert!(store.favourites().unwrap().is_empty());
}

/// **A favourites file that will not parse is said out loud**, because a
/// silently empty answer reads exactly like a library nobody has starred in and
/// the whole of `my sets` would go quiet with nothing to notice.
#[test]
fn a_favourites_file_that_is_not_a_list_of_ids_is_an_error() {
    let dir = tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    fs::write(dir.path().join(Store::FAVOURITES_FILE), b"{ not a list }").unwrap();

    match store.favourites() {
        Err(StoreError::Favourites { .. }) => {}
        other => panic!("expected Favourites, got {other:?}"),
    }
}
