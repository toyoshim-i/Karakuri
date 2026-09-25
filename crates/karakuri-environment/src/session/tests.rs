use super::*;
use karakuri_store::record::{DeckSlot, Value};

fn set_line() -> Line {
    Line::new(Record::Set {
        id: "s".into(),
        v: 1,
    })
}

/// The head stops at the first tick. A `param` before it is what the Set was
/// built with; the same record after it is an edit an operator made, and
/// applying that one at the start would be a different performance.
#[test]
fn state_after_the_first_tick_is_an_edit_and_not_the_head() {
    let param = |v: f32| {
        Line::new(Record::Param {
            at: None,
            key: "radius".into(),
            value: Value::Scalar(v),
        })
    };
    let session = split(vec![
        set_line(),
        param(1.0),
        Line::new(Record::Tick { steps: 1 }),
        param(2.0),
        Line::new(Record::Tick { steps: 2 }),
    ]);

    assert_eq!(
        session.head.len(),
        2,
        "the set and the param before the tick"
    );
    assert_eq!(session.frames.len(), 2);
    assert!(session.frames[0].before.is_empty());
    assert_eq!(session.frames[0].steps, 1);
    assert_eq!(
        session.frames[1].before.len(),
        1,
        "the second param is an edit on the second frame"
    );
    assert_eq!(session.frames[1].steps, 2);
}

/// Verifies that frame records (edits, audio, and tick terminator) partition
/// correctly into their corresponding frame rather than shifting to the next frame.
#[test]
fn a_frames_own_records_land_in_that_frame_and_not_the_next() {
    let audio = |energy: f32| {
        Line::new(Record::Audio {
            energy,
            onset: 0.0,
            bands: vec![energy; 4],
            confidence: 1.0,
        })
    };
    let session = split(vec![
        set_line(),
        audio(0.1),
        Line::new(Record::Tick { steps: 1 }),
        audio(0.9),
        Line::new(Record::Tick { steps: 1 }),
    ]);

    assert_eq!(session.frames.len(), 2);
    assert!(
        session.trailing.is_empty(),
        "a stream that ends on its tick leaves nothing over"
    );
    for (i, expected) in [0.1f32, 0.9].into_iter().enumerate() {
        match session.frames[i].before.as_slice() {
            [Record::Audio { energy, .. }] => assert_eq!(
                *energy, expected,
                "frame {i} got the audio of another frame"
            ),
            other => panic!("frame {i} carries {other:?}"),
        }
    }
}

/// Verifies that canvas dimensions are recovered via `Session::canvas` even when stored as frame edits.
#[test]
fn the_canvas_is_found_before_the_first_frame_renders() {
    let session = split(vec![
        set_line(),
        Line::new(Record::Canvas {
            width: 1920,
            height: 1080,
        }),
        Line::new(Record::Tick { steps: 1 }),
    ]);

    assert_eq!(
        session.head.len(),
        1,
        "the canvas is the session's, so the head is the set alone"
    );
    assert_eq!(session.canvas(), (Some((1920, 1080)), 0));
}

/// Verifies that canvas dimensions are captured even for sessions that terminate without rendering a frame.
#[test]
fn a_session_with_no_tick_still_carries_its_canvas() {
    let session = split(vec![
        set_line(),
        Line::new(Record::Canvas {
            width: 1920,
            height: 1080,
        }),
    ]);
    assert!(session.frames.is_empty());
    assert_eq!(session.canvas(), (Some((1920, 1080)), 0));
}

/// Verifies that a session without canvas records returns `None` rather than falling back to default dimensions.
#[test]
fn a_stream_without_a_canvas_has_no_opinion_about_its_size() {
    let session = split(vec![set_line(), Line::new(Record::Tick { steps: 1 })]);
    assert_eq!(session.canvas(), (None, 0));
}

/// Verifies that multiple canvas declarations are counted as extraneous rather than silently applied.
#[test]
fn later_canvases_are_counted_rather_than_obeyed() {
    let session = split(vec![
        set_line(),
        Line::new(Record::Canvas {
            width: 1920,
            height: 1080,
        }),
        Line::new(Record::Tick { steps: 1 }),
        Line::new(Record::Canvas {
            width: 640,
            height: 480,
        }),
        Line::new(Record::Tick { steps: 1 }),
        Line::new(Record::Canvas {
            width: 800,
            height: 600,
        }),
        Line::new(Record::Tick { steps: 1 }),
    ]);

    assert_eq!(
        session.canvas(),
        (Some((1920, 1080)), 2),
        "the first is the performance's, and the other two are reported"
    );
}

/// Records after the last tick are kept. A session that ended between frames
/// still recorded what the operator last did.
#[test]
fn records_after_the_last_tick_are_not_lost() {
    let session = split(vec![
        set_line(),
        Line::new(Record::Tick { steps: 1 }),
        Line::new(Record::Gain {
            slot: DeckSlot(0),
            value: 0.5,
        }),
    ]);
    assert_eq!(session.frames.len(), 1);
    assert_eq!(session.trailing.len(), 1);
}

/// Verifies that the recording buffer capacity remains constant across frames without heap reallocation.
#[test]
fn pushing_records_never_grows_the_batch() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(dir.path()).expect("store");
    let mut recorder = Recorder::open(&store, "s", &[]).expect("open");

    let capacity = recorder.batch.capacity();
    // Several batches' worth, so the hand-off and the return trip both
    // happen many times over.
    for _ in 0..BATCH * 8 {
        recorder.push(Record::Tick { steps: 1 });
        assert_eq!(
            recorder.batch.capacity(),
            capacity,
            "the batch grew, so a frame allocated"
        );
    }
    // Verify that frame batches dropped under load are accounted for without unbounded growth.
    let w = recorder.finish().expect("finish");
    assert!(w.records > 0, "nothing reached the file");
    assert_eq!(
        w.records + w.dropped_batches * BATCH as u64,
        (BATCH * 8) as u64,
        "records went missing without being counted as dropped"
    );
}

/// At a frame's pace nothing is dropped, which is the case that actually
/// happens: a batch is a second or two of a session and the writer has that
/// long to put 256 short lines on a disk.
#[test]
fn a_session_at_a_frames_pace_loses_nothing() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(dir.path()).expect("store");
    let mut recorder = Recorder::open(&store, "s", &[]).expect("open");
    // Three batches' worth, handed over with the writer given a moment
    // between them — far less of a moment than a frame is.
    for i in 0..BATCH * 3 {
        recorder.push(Record::Tick { steps: 1 });
        if i % 64 == 0 {
            std::thread::yield_now();
        }
    }
    let w = recorder.finish().expect("finish");
    assert_eq!(w.dropped_batches, 0, "a paced session lost batches");
    assert_eq!(w.records, (BATCH * 3) as u64);
}

/// Verifies that audio buffers are exchanged with preallocated shells rather than cloned.
#[test]
fn pushing_audio_takes_the_buffer_rather_than_copying_it() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(dir.path()).expect("store");
    let mut recorder = Recorder::open(&store, "s", &[]).expect("open");

    let mut mine = Record::Audio {
        energy: 0.5,
        onset: 0.0,
        bands: Vec::with_capacity(MAX_BANDS),
        confidence: 1.0,
    };
    let Record::Audio { bands, .. } = &mut mine else {
        unreachable!()
    };
    bands.extend_from_slice(&[0.1, 0.2, 0.3]);
    let was = bands.as_ptr();

    recorder.push_audio(&mut mine);

    let Record::Audio { bands, energy, .. } = &mine else {
        unreachable!()
    };
    assert_ne!(
        bands.as_ptr(),
        was,
        "the caller kept its buffer, so the record was copied rather than taken"
    );
    assert!(bands.is_empty(), "the shell left behind is not empty");
    assert_eq!(*energy, 0.0, "the shell left behind carries a measurement");

    let w = recorder.finish().expect("finish");
    assert_eq!((w.records, w.dropped_audio), (1, 0));
    // And what reached the file is what was measured, not the shell.
    let session = split(store.read_session("s").expect("read"));
    assert_eq!(session.trailing.len(), 1);
    let Record::Audio { energy, bands, .. } = &session.trailing[0] else {
        panic!("not an audio record")
    };
    assert_eq!(*energy, 0.5);
    assert_eq!(bands, &[0.1, 0.2, 0.3]);
}

/// Shells circulate. A frame path that ran out would allocate one per frame,
/// which is the thing this whole arrangement exists to prevent — so the writer
/// returns each band buffer after serialising it, and a long run never asks for
/// a new one.
#[test]
fn audio_shells_come_back_from_the_writer_and_are_reused() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(dir.path()).expect("store");
    let mut recorder = Recorder::open(&store, "s", &[]).expect("open");

    let mut mine = Record::Audio {
        energy: 0.25,
        onset: 0.0,
        bands: Vec::with_capacity(MAX_BANDS),
        confidence: 1.0,
    };
    // Far more frames than there are shells, paced so the writer keeps up.
    for i in 0..BATCH * 2 {
        if let Record::Audio { bands, .. } = &mut mine {
            bands.clear();
            bands.extend_from_slice(&[0.1, 0.2]);
        }
        recorder.push_audio(&mut mine);
        if i % 32 == 0 {
            std::thread::yield_now();
        }
    }
    let w = recorder.finish().expect("finish");
    assert_eq!(
        w.dropped_audio, 0,
        "the frame path ran out of shells, so it would have had to allocate"
    );
    assert_eq!(w.records, (BATCH * 2) as u64);
}

/// A `Held` for a deck of `slots`, at values nothing else in this file uses,
/// so an assertion naming one is naming the value the head carried.
fn held(slots: usize) -> Held {
    Held {
        canvas: (640, 360),
        look: karakuri_engine::Look {
            op: karakuri_engine::present::TonemapOp::Aces,
            exposure: 1.25,
            white_point: 4.0,
        },
        master_out: 0.75,
        master_chain: Vec::new(),
        slots: (0..slots)
            .map(|slot| SlotHeld {
                nodes: vec![
                    (Layer::L1, 0, Hash::of(format!("l1 {slot}").as_bytes())),
                    (Layer::L4, 0, Hash::of(format!("l4a {slot}").as_bytes())),
                    (Layer::L4, 1, Hash::of(format!("l4b {slot}").as_bytes())),
                ],
                gain: 0.1 * slot as f32,
                opacity: 0.5,
                blend: Blend::Over,
                residency: Residency::Priming,
                policy: karakuri_operation::SlotPolicy::Auto,
                mask: Mask::default(),
                transport: Transport::default(),
            })
            .collect(),
    }
}

/// Verifies that the head record encompasses complete deck state, slot assignments, and master chain.
#[test]
fn the_head_names_every_slot_and_the_chain() {
    let head = head(vec![set_line()], &held(3));
    let records: Vec<Record> = head.iter().map(|line| line.record().clone()).collect();

    let procedures: Vec<usize> = records
        .iter()
        .filter_map(|r| match r {
            Record::Procedure { slot, .. } => Some(slot.index()),
            _ => None,
        })
        .collect();
    assert_eq!(
        procedures,
        vec![1, 1, 1, 2, 2, 2],
        "every node of every slot but the head's, and none for the head's"
    );

    for slot in 0..3 {
        let about: Vec<&Record> = records
            .iter()
            .filter(|r| match r {
                Record::Gain { slot: s, .. }
                | Record::Opacity { slot: s, .. }
                | Record::Blend { slot: s, .. }
                | Record::Residency { slot: s, .. }
                | Record::Policy { slot: s, .. }
                | Record::Mask { slot: s, .. }
                | Record::Transport { slot: s, .. } => s.index() == slot,
                _ => false,
            })
            .collect();
        assert_eq!(
            about.len(),
            7,
            "slot {slot}: the seven the deck holds per slot, always and not only \
             where they differ from a fresh deck"
        );
    }

    assert!(
        records.contains(&Record::Canvas {
            width: 640,
            height: 360
        }),
        "no canvas: a replay reads it before it allocates anything"
    );
    assert!(
        records.contains(&Record::MasterOut { value: 0.75 }),
        "no level at the chain's entry"
    );
    assert!(
        records.iter().any(|r| matches!(r, Record::MasterChain(_))),
        "no `master_chain`: a Set file carries nothing for the chain, so the head \
         is the only thing that can put it back"
    );
    assert!(
        records.iter().any(|r| matches!(r, Record::Look { .. })),
        "no look"
    );
}

/// A head goes through [`split`] as a head: the Set file's records on one side,
/// the deck's on the other, and no frame in between.
#[test]
fn a_head_round_trips_through_split() {
    let written = head(vec![set_line()], &held(2));
    let deck_records = written.len() - 1;
    let session = split(written);

    assert_eq!(
        session.head.len(),
        1,
        "the material is the Set file's records and nothing else"
    );
    assert_eq!(
        session.opening.len(),
        deck_records,
        "every record the head wrote about the deck is in `opening`"
    );
    assert!(session.frames.is_empty(), "a head is not a frame");
    assert!(session.trailing.is_empty());
    assert_eq!(
        session.canvas(),
        (Some((640, 360)), 0),
        "the canvas is read out of the head"
    );
}

/// Verifies that pre-tick measurement records are attributed to frame 0 rather than the head.
#[test]
fn the_first_frames_measurement_is_not_the_heads() {
    let session = split(vec![
        set_line(),
        Line::new(Record::Gain {
            slot: DeckSlot(0),
            value: 0.5,
        }),
        Line::new(Record::Audio {
            energy: 0.4,
            onset: 0.0,
            bands: vec![0.1; 4],
            confidence: 1.0,
        }),
        Line::new(Record::Tick { steps: 1 }),
    ]);
    assert_eq!(session.head.len(), 1, "the Set file's record");
    assert_eq!(session.opening.len(), 1, "the deck's gain");
    assert_eq!(
        session.frames[0].before.len(),
        1,
        "the measurement stayed with the frame it was measured in"
    );
    assert!(matches!(session.frames[0].before[0], Record::Audio { .. }));
}

/// Verifies that all call sites instantiating `Recorder::open` generate head data via `session::head`.
#[test]
fn every_recorder_is_opened_over_a_head_this_module_wrote() {
    fn walk(dir: &std::path::Path, into: &mut Vec<std::path::PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, into);
            } else if path.extension().is_some_and(|e| e == "rs") {
                into.push(path);
            }
        }
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root");
    let mut files = Vec::new();
    walk(&root.join("crates"), &mut files);
    // This module's own tests open recorders over heads written by hand, to
    // check the recorder rather than the head.
    let defines = root.join("crates/karakuri-environment/src/session");
    let mut opened = 0;
    for path in files {
        if path.starts_with(&defines) {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("read source");
        if !text.contains("Recorder::open(") {
            continue;
        }
        opened += 1;
        assert!(
            text.contains("session::head("),
            "{}: opens a session recorder over a head it assembled itself — a head \
             says what the deck held, and `session::head` is the one place that \
             sentence is spelled",
            path.display()
        );
    }
    assert_eq!(
        opened, 2,
        "expected the two surfaces that record a session — `karakuri-cli`'s \
         `--record-session` and the console's `rec` pill. A third is welcome and owes \
         this count a line; fewer means the scan has stopped finding either"
    );
}

/// What was pushed is what the file holds, in order.
#[test]
fn a_recorded_session_reads_back_as_what_was_pushed() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(dir.path()).expect("store");
    let mut recorder = Recorder::open(&store, "s", &[set_line()]).expect("open");
    for steps in [1u8, 2, 1, 4] {
        recorder.push(Record::Tick { steps });
    }
    let w = recorder.finish().expect("finish");
    assert_eq!((w.records, w.dropped_batches), (4, 0));

    let session = split(store.read_session("s").expect("read"));
    assert_eq!(session.head.len(), 1);
    assert_eq!(
        session.frames.iter().map(|f| f.steps).collect::<Vec<_>>(),
        vec![1, 2, 1, 4]
    );
}
