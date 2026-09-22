use super::*;

// Message parsing tests

fn round_trip(line: &str) -> Message {
    let message: Message = serde_json::from_str(line).expect("parse");
    let back = serde_json::to_string(&message).expect("serialise");
    let again: Message = serde_json::from_str(&back).expect("reparse");
    assert_eq!(message, again);
    message
}

/// The three lines the module doc prints, parsed and written back.
///
/// The doc is the specification — the helper is another repository and may be
/// another language, so it implements what is written here rather than
/// importing a type. That only works if what is written here is what this build
/// actually reads, which is what this test is.
#[test]
fn the_lines_the_doc_prints_are_the_lines_this_reads() {
    assert_eq!(
        round_trip(r#"{"t":"hello","v":1,"source":"ableton-link"}"#),
        Message::Hello {
            v: 1,
            source: "ableton-link".to_string()
        }
    );
    assert_eq!(
        round_trip(r#"{"t":"grid","bpm":128.0,"beat":1024.25,"clock_us":881234567}"#),
        Message::Grid {
            bpm: 128.0,
            beat: 1024.25,
            clock_us: 881_234_567
        }
    );
    assert_eq!(
        round_trip(r#"{"t":"status","peers":2,"playing":true}"#),
        Message::Status {
            peers: 2,
            playing: true
        }
    );
}

/// A newer helper saying something this build has no use for is ignored rather
/// than fatal, and so is a field it has never heard of.
///
/// This is the normal case rather than the exceptional one, which is the whole
/// argument for a self-describing format here: the helper is released on its
/// own schedule from its own repository, so the versions being out of step is
/// what usually happens.
#[test]
fn a_newer_helper_is_understood_as_far_as_it_goes() {
    assert_eq!(
        round_trip(r#"{"t":"quantum","beats":4.0}"#),
        Message::Unknown,
        "an unknown message type must be skippable, not fatal"
    );
    assert_eq!(
        round_trip(r#"{"t":"grid","bpm":128.0,"beat":1.0,"clock_us":1,"confidence":0.9}"#),
        Message::Grid {
            bpm: 128.0,
            beat: 1.0,
            clock_us: 1
        },
        "an unknown field must be ignored, not fatal"
    );
    // And a field with a default may be absent entirely.
    assert_eq!(
        round_trip(r#"{"t":"status","peers":0}"#),
        Message::Status {
            peers: 0,
            playing: false
        }
    );
}

/// The greeting is checked once and names both versions when it fails.
///
/// A shared library rots loudly at compile time; a separately released program
/// rots quietly at run time, so the one thing that must not happen is a
/// mismatch that looks like a working connection. See `docs/plugins.md`.
#[test]
fn a_version_this_build_cannot_read_is_refused_by_name() {
    let ours = accept(&Message::Hello {
        v: PROTOCOL_VERSION,
        source: "ableton-link".into(),
    });
    assert_eq!(ours, Ok("ableton-link".to_string()));

    let theirs = accept(&Message::Hello {
        v: PROTOCOL_VERSION + 1,
        source: "ableton-link".into(),
    });
    let Err(refusal) = theirs else {
        panic!("a future version was accepted");
    };
    let said = refusal.to_string();
    assert!(
        said.contains(&format!("v{}", PROTOCOL_VERSION + 1)),
        "{said}"
    );
    assert!(said.contains(&format!("v{PROTOCOL_VERSION}")), "{said}");

    assert_eq!(
        accept(&Message::Status {
            peers: 1,
            playing: false
        }),
        Err(Refusal::NoGreeting),
        "a source that starts talking without introducing itself is refused"
    );
}

// Runner tests

/// A fake source as a script on disk.
///
/// A script file rather than `sh -c '...'` because [`Source::open`] takes a
/// program and space-separated arguments and deliberately does not parse quotes
/// — so a test that needed quoting would be testing a shell this program does
/// not have.
///
/// What these tests prove and do not prove is worth stating. They exercise the
/// spawn, the handshake, the reader thread, the clock offset and the poll. They
/// say nothing whatever about Ableton Link, which is a network protocol nothing
/// here speaks. Only a second peer can say that.
fn fake(lines: &str) -> (Source, tempfile::TempDir) {
    let (source, dir) = try_fake(lines);
    (source.expect("open"), dir)
}

fn try_fake(lines: &str) -> (Result<Source, String>, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let script = dir.path().join("source.sh");
    std::fs::write(&script, lines).expect("write script");
    let command = format!("sh {}", script.display());
    (Source::open(&command), dir)
}

/// Poll until an anchor arrives or the patience runs out. Polling rather than
/// sleeping a guessed amount: the reader is a thread and a fixed sleep is
/// either flaky or slow.
fn wait_for_anchor(source: &mut Source) -> Option<Anchor> {
    for _ in 0..200 {
        if let Some(anchor) = source.poll().anchor {
            return Some(anchor);
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    None
}

/// The happy path: greet, then anchor.
#[test]
fn an_anchor_arrives_and_extrapolates_from_its_own_timestamp() {
    let (mut source, _dir) = fake(concat!(
        "echo '{\"t\":\"hello\",\"v\":1,\"source\":\"fake\"}'\n",
        "echo '{\"t\":\"grid\",\"bpm\":120.0,\"beat\":100.0,\"clock_us\":0}'\n",
        "echo '{\"t\":\"status\",\"peers\":3,\"playing\":true}'\n",
        "sleep 5\n",
    ));
    assert_eq!(source.name(), "fake");

    let anchor = wait_for_anchor(&mut source).expect("no anchor arrived");
    for _ in 0..50 {
        source.poll();
        if source.peers() == 3 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert_eq!(source.peers(), 3);
    assert_eq!(source.playing(), Some(true));

    // 120 bpm is two beats a second, so a second past the anchor is beat
    // 102. **This is the arithmetic that makes the transport's delay
    // irrelevant**: the anchor is read from its own timestamp rather than
    // from when it happened to arrive.
    let a_second_later = anchor.at_us + 1_000_000;
    assert!(
        (anchor.beat_at(a_second_later) - 102.0).abs() < 1e-6,
        "beat {} at one second past the anchor",
        anchor.beat_at(a_second_later)
    );
    source.close();
}

/// A source that dies says so once.
///
/// The caller prints that once and does nothing else — the grid holds where it
/// was, because the last tempo a source gave is still the best anyone has and a
/// beat that jumped because a helper crashed would be worse than one that
/// merely stopped being corrected. What is asserted here is the part this
/// module owns: that a caller polling every frame cannot be handed the news
/// more than once.
#[test]
fn a_source_that_dies_is_reported_once() {
    let (mut source, _dir) = fake("echo '{\"t\":\"hello\",\"v\":1,\"source\":\"fake\"}'\n");
    let mut said = Vec::new();
    // **Polled until the child is gone, not a fixed number of times.** How
    // long a `sh` takes to print a line and exit is the machine's business,
    // and a count that was ample alone failed once in a whole-workspace run
    // where a dozen test binaries were competing for the same cores. The
    // ceiling is a stall guard, not a timing assumption.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while source.ended().is_none() && std::time::Instant::now() < deadline {
        source.poll();
        if let Some(why) = source.unreported_end() {
            said.push(why.to_string());
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    assert!(source.ended().is_some(), "the source outlived its script");
    // And then a caller that keeps polling — which is what a frame loop
    // does — is not handed the news again, which is the whole subject.
    for _ in 0..50 {
        source.poll();
        if let Some(why) = source.unreported_end() {
            said.push(why.to_string());
        }
    }
    assert_eq!(said.len(), 1, "reported {said:?}");
    source.close();
}

/// Verifies that the clock offset is estimated from the message with minimal delay,
/// tested against a non-zero remote epoch.
#[test]
fn the_offset_is_estimated_from_the_least_delayed_message() {
    let epoch = 1_000_000_000_i64;
    let (mut source, _dir) = fake(&format!(
        concat!(
            "echo '{{\"t\":\"hello\",\"v\":1,\"source\":\"fake\"}}'\n",
            "echo '{{\"t\":\"grid\",\"bpm\":120.0,\"beat\":0.0,\"clock_us\":{}}}'\n",
            "sleep 5\n",
        ),
        epoch
    ));
    let anchor = wait_for_anchor(&mut source).expect("no anchor");
    // Our clock starts at zero, theirs at a billion, so the offset is about
    // minus a billion and the anchor lands near *our* now rather than a
    // billion microseconds in the future.
    assert!(
        anchor.at_us.abs() < 2_000_000,
        "the anchor landed at {} — the source's epoch was not removed",
        anchor.at_us
    );
    source.close();
}

/// One forward clock step must not pin the estimate forever.
///
/// A running minimum can only fall, so a single message reading ahead of us — a
/// helper on a clock that counts suspend, one reporting milliseconds — used to
/// latch the offset there permanently and place every later anchor that far in
/// the past. The window forgets.
#[test]
fn a_single_clock_step_does_not_latch_the_offset() {
    let far_ahead = 3_600_000_000_i64;
    let mut lines = String::from("echo '{\"t\":\"hello\",\"v\":1,\"source\":\"fake\"}'\n");
    lines.push_str("echo '{\"t\":\"grid\",\"bpm\":120.0,\"beat\":0.0,\"clock_us\":0}'\n");
    // The liar, once.
    lines.push_str(&format!(
        "echo '{{\"t\":\"grid\",\"bpm\":120.0,\"beat\":1.0,\"clock_us\":{far_ahead}}}'\n"
    ));
    // Then enough honest ones to fill the window past the liar.
    for i in 0..(OFFSET_WINDOW + 4) {
        lines.push_str(&format!(
            "echo '{{\"t\":\"grid\",\"bpm\":120.0,\"beat\":{}.0,\"clock_us\":0}}'\n",
            i + 2
        ));
    }
    lines.push_str("sleep 5\n");

    let (mut source, _dir) = fake(&lines);
    let mut last = None;
    for _ in 0..300 {
        if let Some(anchor) = source.poll().anchor {
            last = Some(anchor);
        }
        if source.offset_us.is_some() && last.is_some_and(|a| a.beat >= 4.0) {
            std::thread::sleep(std::time::Duration::from_millis(20));
            while let Some(anchor) = source.poll().anchor {
                last = Some(anchor);
            }
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let anchor = last.expect("no anchor");
    assert!(
        anchor.at_us.abs() < 5_000_000,
        "the anchor landed at {} — one forward step pinned the offset",
        anchor.at_us
    );
    source.close();
}

/// A message that was written long before it arrived must not drag the
/// estimate, which is the whole reason the estimate is a minimum.
///
/// The window test above catches a clock reading *ahead*; this catches the case
/// the minimum exists for — a message delayed on its way here. Without both,
/// inverting `min` to `max` leaves the suite green, which is exactly what a
/// review found it doing.
#[test]
fn a_message_that_arrived_late_does_not_drag_the_offset() {
    let stale = -30_000_000_i64;
    let mut lines = String::from("echo '{\"t\":\"hello\",\"v\":1,\"source\":\"fake\"}'\n");
    // One anchor stamped thirty seconds ago — a message that sat somewhere
    // on its way here. Its delta is thirty seconds larger than everyone
    // else's, and a maximum would take it.
    lines.push_str(&format!(
        "echo '{{\"t\":\"grid\",\"bpm\":120.0,\"beat\":0.0,\"clock_us\":{stale}}}'\n"
    ));
    for i in 1..8 {
        lines.push_str(&format!(
            "echo '{{\"t\":\"grid\",\"bpm\":120.0,\"beat\":{i}.0,\"clock_us\":0}}'\n"
        ));
    }
    lines.push_str("sleep 5\n");

    let (mut source, _dir) = fake(&lines);
    let mut last = None;
    for _ in 0..300 {
        if let Some(anchor) = source.poll().anchor {
            last = Some(anchor);
        }
        if last.is_some_and(|a: Anchor| a.beat >= 7.0) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let anchor = last.expect("no anchor");
    assert!(
        anchor.at_us.abs() < 5_000_000,
        "the anchor landed at {} — a thirty-second-old message set the offset",
        anchor.at_us
    );
    source.close();
}

/// A number that cannot be used is counted, not applied and not fatal.
///
/// `clock_us` near `i64::MIN` used to panic on the frame path in a debug build;
/// a non-finite tempo used to be smuggled as far as the oscillator.
#[test]
fn an_unusable_anchor_is_counted_and_dropped() {
    let (mut source, _dir) = fake(concat!(
        "echo '{\"t\":\"hello\",\"v\":1,\"source\":\"fake\"}'\n",
        "echo '{\"t\":\"grid\",\"bpm\":0.0,\"beat\":1.0,\"clock_us\":0}'\n",
        "echo '{\"t\":\"grid\",\"bpm\":120.0,\"beat\":1.0,\"clock_us\":-9223372036854775808}'\n",
        "echo '{\"t\":\"grid\",\"bpm\":120.0,\"beat\":5.0,\"clock_us\":0}'\n",
        "sleep 5\n",
    ));
    let mut last = None;
    for _ in 0..300 {
        if let Some(anchor) = source.poll().anchor {
            last = Some(anchor);
        }
        if last.is_some_and(|a: Anchor| a.beat == 5.0) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert_eq!(last.expect("no anchor").beat, 5.0);
    assert_eq!(source.rejected(), 1, "the zero tempo was not counted");
    source.close();
}

/// The first anchor aligns; the ones after it are bounded.
///
/// The bound is the difference between a wrong reading and a wrong show.
/// Without it a source with a broken clock moves the grid by whatever it says,
/// instantly, at full confidence — which is exactly what the in-process tracker
/// has never been allowed to do.
#[test]
fn the_first_anchor_aligns_and_the_rest_are_trimmed() {
    let (mut source, _dir) = fake(concat!(
        "echo '{\"t\":\"hello\",\"v\":1,\"source\":\"fake\"}'\n",
        "sleep 5\n",
    ));
    let anchor = Anchor {
        bpm: 120.0,
        beat: 1024.25,
        at_us: 0,
    };

    // Our grid is at 7.0 and the room is at 1024.25: the first anchor moves
    // the whole way, and modulo `ALIGN_MODULUS` so the count stays small.
    let Correction::Align { shift, bpm } = source.correction(anchor, 7.0, 0) else {
        panic!("the first anchor did not align");
    };
    assert_eq!(bpm, 120.0);
    assert!(
        (7.0 + shift - 0.25).abs() < 1e-9,
        "landed at {}",
        7.0 + shift
    );
    // 1024.25 mod 1024 is 0.25, and 1024 is a multiple of four — so the
    // bar phase is the room's even though the number is not.
    assert!(
        ((1024.25_f64).rem_euclid(4.0) - (0.25_f64).rem_euclid(4.0)).abs() < 1e-9,
        "the alignment moved the downbeat"
    );

    // A second anchor claiming we are a hundred beats out is *not* obeyed:
    // it is trimmed, and only sustained disagreement re-aligns.
    for _ in 0..(RELOCK_EVIDENCE - 1) {
        let Correction::Trim { shift, .. } = source.correction(anchor, 924.25, 0) else {
            panic!("a single large disagreement re-aligned");
        };
        assert!(
            shift.abs() <= TRIM_LIMIT_BEATS + 1e-9,
            "trim of {shift} beats is past the limit"
        );
    }
    // The third in a row is news rather than noise.
    assert!(
        matches!(
            source.correction(anchor, 924.25, 0),
            Correction::Align { .. }
        ),
        "sustained disagreement never re-aligned"
    );
    source.close();
}

/// An ordinary small disagreement is passed through untouched — the clamp must
/// not be a permanent brake on following the room.
#[test]
fn a_small_disagreement_is_followed_exactly() {
    let (mut source, _dir) = fake(concat!(
        "echo '{\"t\":\"hello\",\"v\":1,\"source\":\"fake\"}'\n",
        "sleep 5\n",
    ));
    let anchor = Anchor {
        bpm: 128.0,
        beat: 100.0,
        at_us: 0,
    };
    let _ = source.correction(anchor, 0.0, 0); // the alignment
    let Correction::Trim { shift, .. } = source.correction(anchor, 100.0 - 0.01, 0) else {
        panic!("a small disagreement did not trim");
    };
    assert!((shift - 0.01).abs() < 1e-9, "0.01 beats became {shift}");
    source.close();
}

/// A version this build cannot read is refused at open, before a frame.
#[test]
fn a_source_speaking_another_version_never_opens() {
    let (result, _dir) = try_fake(concat!(
        "echo '{\"t\":\"hello\",\"v\":99,\"source\":\"fake\"}'\n",
        "sleep 5\n",
    ));
    // `let Err(..) else` rather than `expect_err`, which would want a
    // `Debug` on `Source` that exists only for this line.
    let Err(error) = result else {
        panic!("a future version was accepted");
    };
    assert!(error.contains("v99"), "{error}");
    assert!(error.contains(&format!("v{PROTOCOL_VERSION}")), "{error}");
}

/// Noise on the stream is stepped over rather than fatal — a helper that logged
/// to the wrong stream must not end the session.
#[test]
fn a_line_that_is_not_a_message_is_stepped_over() {
    let (mut source, _dir) = fake(concat!(
        "echo '{\"t\":\"hello\",\"v\":1,\"source\":\"fake\"}'\n",
        "echo 'listening on en0'\n",
        "echo '{\"t\":\"grid\",\"bpm\":90.0,\"beat\":7.0,\"clock_us\":0}'\n",
        "sleep 5\n",
    ));
    assert_eq!(wait_for_anchor(&mut source).expect("no anchor").beat, 7.0);
    source.close();
}
