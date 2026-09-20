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

/// A `tick` is a terminator, not a header, and that is what decides where a
/// frame's own measurements land.
///
/// Written in the order `Live::frame` emits them — the edits an operator made,
/// then the audio that frame heard, then the tick that closes it — every record
/// reaches the frame it was produced during. The audio used to go out *after*
/// the tick, which put frame N's reading in front of frame N+1: live, frame N
/// rendered with what frame N heard; replayed, with what N−1 heard. One frame
/// late, every frame, in the two signals every binding is driven by.
///
/// What this test cannot see is the writer. `Live::frame` needs a window, so
/// the order it pushes in is checked by reading it and this checks only that
/// `split` honours that order once written. Swap the two pushes back and
/// nothing here goes red — said out loud rather than left for someone to assume
/// otherwise.
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

/// The canvas is read out of the stream, not out of the head.
///
/// It is session state, so `split` puts it in the *first frame's* edits rather
/// than in the head — and a replay needs it strictly earlier than that, before
/// it allocates anything. The two facts together are why [`Session::canvas`]
/// exists instead of a field on the head.
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

/// A session that never drew a frame still carries its canvas.
///
/// The record is written before the first tick, so with no tick at all there is
/// no `Frame` to hold it and it lands in `trailing` — the one place a scan over
/// frames alone cannot see. A run closed during startup produces exactly this
/// stream.
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

/// A stream with no canvas says so rather than answering with a size.
///
/// `None` and "the default" have to stay distinguishable here: the caller
/// prints a line saying the size is a guess, and a `Session::canvas` that
/// helpfully returned 1920x1080 would make that line unwritable.
#[test]
fn a_stream_without_a_canvas_has_no_opinion_about_its_size() {
    let session = split(vec![set_line(), Line::new(Record::Tick { steps: 1 })]);
    assert_eq!(session.canvas(), (None, 0));
}

/// Later ones are counted, not obeyed and not swallowed.
///
/// This program writes exactly one, at the head — a canvas change would be a
/// GPU reallocation mid-run. So a second one means a stream something else
/// wrote, and the count is what lets a replay say it did not honour it.
/// Silently taking the first would be indistinguishable from a replay that had
/// followed every one.
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

/// The frame path allocates nothing. A batch is handed away the moment it is
/// full and an empty one comes back, so the buffer's capacity — and therefore
/// its pointer — never changes.
///
/// A counting allocator would be the direct assertion and cannot be used here:
/// `#[global_allocator]` is per binary and this is one. The capacity is the
/// observable consequence, and it is not a proxy — a `Vec` that grew would
/// report a larger one.
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
    // **Drops are expected here and are the second half of the claim.**
    // Nothing paces this loop, so it offers batches far faster than a disk
    // takes them — which is precisely the case the design is about. What
    // must hold is that the frame path did not grow, did not block, and
    // *counted* what was lost. A recorder that silently skipped would pass
    // the capacity assertion above and hand back a file that looked whole.
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

/// The audio record is swapped, not copied, which is the whole reason it can be
/// recorded from a frame at all.
///
/// The caller's record comes back with a *different* band buffer — the shell's
/// — and the one it had went into the stream. Pointer identity is the
/// observable form of that: a clone would leave the caller's own buffer where
/// it was.
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

/// **The head names every slot of the deck and the chain**, which is the whole
/// of what a session stream could not say.
///
/// One Set file for slot 0 and a `procedure` record per node of every other
/// slot — never one for slot 0, whose Set file already carries its nodes *and*
/// the params, bindings and seeds a `procedure` record cannot. Then the mix,
/// for every slot and not only for the ones that differ from a fresh deck: a
/// replay that had to know the deck's defaults would be a second derivation of
/// them.
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

/// **A measurement before the first tick belongs to the first frame**, not to
/// the head.
///
/// A frame writes its edits, then what it heard, then the tick that closes it —
/// so the `audio` line in front of the very first tick is frame 0's
/// measurement. Sorting the head by position alone would move it into the head
/// and replay frame 0 at what the bus invents rather than at what the room
/// heard.
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

/// **Every program that opens a recorder writes its head through [`head`].**
///
/// A recorder takes a head and appends nothing to it afterwards, so whatever
/// is handed to [`Recorder::open`] *is* the head of that stream. Two surfaces
/// record sessions — `karakuri-cli`'s `--record-session` and the console's
/// `rec` pill — and what a head has to say is the same sentence for both: what
/// every slot of the deck held. A second assembly of that sentence is a second
/// answer to it, which is what this file's two writers used to be.
///
/// **A source scan, because the round trip it would rather be cannot be run.**
/// Both writers need a deck and a deck needs a device; `karakuri-cli`'s live
/// path needs a *window* on top of that, which is why `--record-session` is
/// refused alongside `--render`, `--seq` and `--replay`. So the far end of the
/// claim is checked where it can be — `karakuri-cli/tests/replay.rs` drives a
/// hand-written two-slot head through the binary — and this is the near end.
///
/// What it cannot see is a writer that calls [`head`] and then appends records
/// of its own. It is coarse in the safe direction: the failure it refuses is
/// the one that already happened.
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
