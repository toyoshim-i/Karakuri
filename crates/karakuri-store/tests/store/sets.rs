use super::*;

fn param(key: &str, value: f32) -> Record {
    Record::Param {
        at: None,
        key: key.into(),
        value: Value::Scalar(value),
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
            at: NodeAddress {
                layer: Layer::L1,
                index: 0,
            },
            name: None,
            proc_hash,
        }),
        Line::new(Record::Capacity {
            at: NodeAddress {
                layer: Layer::L1,
                index: 0,
            },
            value: 524288,
        }),
        Line::new(param("radius", 2.4)),
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
        Line::new(param("radius", 2.6)),
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
            at: NodeAddress {
                layer: Layer::L4,
                index: 0,
            },
            name: None,
            proc_hash,
        }),
        Line::new(Record::Slot {
            at: NodeAddress {
                layer: Layer::L4,
                index: 1,
            },
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
            slot: karakuri_store::record::DeckSlot(0),
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
            at: NodeAddress {
                layer: Layer::L1,
                index: 0,
            },
            name: None,
            proc_hash,
        }),
        Line::new(param("radius", 2.0)),
        Line::new(Record::Tick { steps: 1 }),
        Line::new(Record::Tick { steps: 1 }),
        Line::new(param("radius", 2.6)), // repeated edit to the same key
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
    assert_eq!(radius_records[0].record(), &param("radius", 2.6));
}

#[test]
fn projection_folds_repeated_edits_last_write_wins() {
    let session = vec![
        Line::new(param("radius", 1.0)),
        Line::new(param("radius", 2.0)),
        Line::new(param("radius", 3.0)),
        Line::new(Record::Tick { steps: 1 }),
    ];

    let set = project(&session);
    assert_eq!(set.len(), 1);
    assert_eq!(set[0].record(), &param("radius", 3.0));
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
            at: NodeAddress {
                layer: Layer::L1,
                index: 0,
            },
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
pub fn a_set() -> Vec<Line> {
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
