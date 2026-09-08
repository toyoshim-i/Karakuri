//! **The tracker group's other three controls: the latency offset, the tap and
//! the octave.**
//!
//! They are the ninth, tenth and eleventh controls this console draws, and they
//! finish the group `docs/manual/style.css` puts round exactly four things:
//! *"the pill that says whether there is an audio input, the latency offset …
//! the tap, and the octave. None of them means anything without an input, so
//! they are one item at the pills' own 5px rather than four at the row's 14."*
//! [`audio_in`] is the head of it and `tests/audio_in.rs` is that pill's file;
//! this is the rest.
//!
//! Eight things, and the first three are why this is its own file rather than
//! more assertions in `tests/look.rs`:
//!
//! 1. Where the three are, laid end to end from the audio-in pill's right edge
//!    at the group's own gap — and that the arrangement pill has moved along by
//!    exactly this group and one `.transport` gap, which is what a flex row is.
//! 2. **That all three clear every boundary's grab**, which is
//!    `tests/look.rs`'s arithmetic over three more controls and is never
//!    inherited from it. The octave's chips are the shortest targets in this
//!    row — `OCTAVE_H` is 15.5 where a pill is 16.5 — so the clearance is
//!    measured against them rather than against the tap.
//! 3. **That the refused half of the octave is drawn and not claimed**, which
//!    is `DeckHead::arrow`'s rule one bay over: *"the arrows keep their shape
//!    when they are refused"*, and a chip that vanished would move the row
//!    under the hand every time the grid crossed 100 or 120 BPM.
//! 4. What a press on each of them asks for.
//! 5. **That one pixel of the offset track is one press of `o` or `p`**, which
//!    is the whole of why it is 80 wide.
//! 6. That the ends are exactly ∓200 ms and the middle exactly zero.
//! 7. **That a console with nothing open draws no offset at all**, and that the
//!    tap closes up behind it — the page's own answer to a track with no value
//!    to point at.
//! 8. **The route a window loop actually takes** — `claim`, then the derivation
//!    that drew the control, then the operation.
//!
//! None of it needs a window, a device or a disk. It does need `egui`'s fonts,
//! because the tap capsule is as wide as the word in it and the offset's figure
//! is as wide as the number in it — see `common::drawn_once`.

mod common;

use common::{drawn_once, near, rect_of, PLAUSIBLE, SMALLEST};
use karakuri_console::input::{claim, Claim};
use karakuri_console::panel::{Panel, GRAB};
use karakuri_console::room::{size, Room};
use karakuri_console::view::{
    arrangement, audio_in, offset_at, tracker_group, unit_of_offset, Arrangement, AudioIn, Tracker,
    TrackerGroup, Transport, View, LATENCY_OFFSET_MAX_MS, LATENCY_OFFSET_MIN_MS,
    LATENCY_OFFSET_STEP_MS, OFFSET_TRACK_W,
};
use karakuri_layout::{Point, Rect};
use karakuri_operation::{GridScale, Operation};

/// **The mock's own transport, as numbers** — `transport.rs`'s, which is where
/// the argument for each of them is. The group is measured from the audio-in
/// pill and the pill from the bar, so a row is needed to have any of it.
fn mock() -> Transport {
    common::mock_transport()
}

/// **The mock's own tracker**: `offset −15 ms`, `½` live and `×2` refused.
///
/// The refusal is the mock's and it is not a choice made here:
/// `docs/manual/console.html` draws `×2` `.idle` at 128.0 and says why in as
/// many words — *"128.0 doubled is 256 and the tracker searches 60 to 200
/// BPM"*. Half of 128 is 64, which is inside it.
fn mock_tracker() -> Tracker {
    Tracker {
        offset_ms: Some(-15.0),
        halve: true,
        double: false,
    }
}

/// **A console that has been told about audio and found something**, which is
/// what the mock draws: the group's head is a pill and the three below are laid
/// out from it.
fn heard() -> AudioIn {
    let mut audio = AudioIn::NONE;
    audio.device = Some("Scarlett 2i2".to_owned());
    audio.inputs = vec!["Scarlett 2i2".to_owned()];
    audio
}

/// A view with an engine behind it, an input open and a tracker reading — what
/// `View::draw` paints from and what `claim` hit-tests, one value.
fn view(at: Tracker) -> View {
    let mut view = View::new(Room::Day);
    view.transport = Some(mock());
    view.audio = Some(heard());
    view.tracker = Some(at);
    view
}

/// A panel at a viewport, solved, with a context that has drawn once.
fn console(viewport: Rect) -> (Panel, egui::Context) {
    let mut panel = Panel::new(viewport.w, viewport.h);
    panel.solve();
    (panel, drawn_once())
}

/// The laid-out group at that reading, on a solved console.
fn laid_out(panel: &Panel, ctx: &egui::Context, at: Tracker) -> TrackerGroup {
    tracker_group(ctx, panel.layout(), Some(mock()), Some(&heard()), Some(at))
        .expect("the transport row draws the tracker group")
}

fn at(p: egui::Pos2) -> Point {
    Point::new(p.x, p.y)
}

// ---------------------------------------------------------------------------
// Where they are
// ---------------------------------------------------------------------------

/// **The mock's own order, at the mock's own gaps**, and the arrangement pill
/// moved along by exactly this group.
///
/// The four things `.tracker` holds are `audio-in`, the offset, the tap and the
/// octave, laid end to end at `PILL_GAP`; the group ends at the octave's second
/// half and what follows it is a `TRANSPORT_GAP` away, because what ended is a
/// whole group.
#[test]
fn the_group_is_laid_out_the_way_the_mock_lays_it_out() {
    let (panel, ctx) = console(PLAUSIBLE);
    let pill = audio_in(&ctx, panel.layout(), Some(mock()), Some(&heard()))
        .expect("the audio-in pill heads the group");
    let group = laid_out(&panel, &ctx, mock_tracker());
    let offset = group.offset.expect("an open session is holding an offset");

    assert!(
        near(offset.label.min.x, pill.pill.max.x + size::PILL_GAP),
        "the offset starts at {} and the audio-in pill ends at {}, which is not the group's \
         own {} gap",
        offset.label.min.x,
        pill.pill.max.x,
        size::PILL_GAP
    );
    assert!(
        near(offset.track.min.x, offset.label.max.x + size::TRIM_GAP),
        "the track does not sit one `.trim` gap after the word"
    );
    assert!(
        near(offset.value.min.x, offset.track.max.x + size::TRIM_GAP),
        "the figure does not sit one `.trim` gap after the track"
    );
    assert!(
        near(group.tap.min.x, offset.value.max.x + size::PILL_GAP),
        "the tap does not follow the offset at the group's gap"
    );
    assert!(
        near(group.halve.min.x, group.tap.max.x + size::PILL_GAP),
        "the octave does not follow the tap at the group's gap"
    );
    assert!(
        near(group.double.min.x, group.halve.max.x + size::OCTAVE_GAP),
        "the two halves of the octave are not `.octave`'s own {} apart",
        size::OCTAVE_GAP
    );

    let after = arrangement(
        &ctx,
        panel.layout(),
        Some(mock()),
        Some(&heard()),
        Some(mock_tracker()),
        &Arrangement::NONE,
    )
    .expect("the arrangement pill");
    assert!(
        near(after.pill.min.x, group.double.max.x + size::TRANSPORT_GAP),
        "the arrangement pill starts at {} and the group ends at {}: what ended is a whole \
         group, so the gap is the row's {} and not the group's",
        after.pill.min.x,
        group.double.max.x,
        size::TRANSPORT_GAP
    );
}

/// **The track is exactly as wide as the presses that cross it.**
///
/// Eighty five-millisecond steps between −200 and +200, and eighty pixels — so
/// a pointer can ask for every value `o` and `p` can reach and neither surface
/// can reach one the other cannot. It is the exposure track's own property one
/// control along, and it is what decides the width rather than looks.
#[test]
fn one_pixel_of_the_track_is_one_press_of_a_key() {
    let span = LATENCY_OFFSET_MAX_MS - LATENCY_OFFSET_MIN_MS;
    assert_eq!(span, 400.0);
    assert_eq!(OFFSET_TRACK_W, span / LATENCY_OFFSET_STEP_MS);
    assert_eq!(OFFSET_TRACK_W, 80.0);

    // A pixel along is a press along, at both ends and in the middle.
    for pixel in [0.0_f32, 1.0, 40.0, 79.0] {
        let here = offset_at(pixel / OFFSET_TRACK_W);
        let next = offset_at((pixel + 1.0) / OFFSET_TRACK_W);
        assert!(
            near(next - here, LATENCY_OFFSET_STEP_MS),
            "pixel {pixel} to {} is {} ms, and one press is {LATENCY_OFFSET_STEP_MS}",
            pixel + 1.0,
            next - here
        );
    }
}

/// **The two ends are exactly the two ends, and the middle is exactly zero.**
///
/// Linear rather than the exposure's logarithm, because an offset is a
/// difference and not a ratio — and the symmetry is what makes the middle
/// exact.
#[test]
fn the_ends_are_the_ends_and_the_middle_is_zero() {
    assert_eq!(offset_at(0.0), LATENCY_OFFSET_MIN_MS);
    assert_eq!(offset_at(1.0), LATENCY_OFFSET_MAX_MS);
    assert_eq!(offset_at(0.5), 0.0);
    assert_eq!(unit_of_offset(0.0), 0.5);
    assert_eq!(unit_of_offset(LATENCY_OFFSET_MIN_MS), 0.0);
    assert_eq!(unit_of_offset(LATENCY_OFFSET_MAX_MS), 1.0);
    // Past either end is drawn at that end, and the figure is what says the
    // number — `unit_of`'s rule one control along.
    assert_eq!(unit_of_offset(-1000.0), 0.0);
    assert_eq!(unit_of_offset(1000.0), 1.0);
}

/// **All three clear every boundary's grab**, which is rule 3's ordinary price
/// and is measured here rather than inherited.
///
/// The transport row is 48 tall and the shortest target in the group is an
/// octave chip at `OCTAVE_H` — 15.5 — so the clearance above and below is
/// 16.25 against a `GRAB` of 6. The offset's target is the **track grown to a
/// line's height** and not the 5px track, which is the number that matters.
#[test]
fn the_three_clear_every_boundary() {
    for viewport in [SMALLEST, PLAUSIBLE] {
        let (mut panel, ctx) = console(viewport);
        let group = laid_out(&panel, &ctx, mock_tracker());
        let offset = group.offset.expect("an offset");
        let strip = rect_of(panel.layout(), "transport");
        let view = view(mock_tracker());

        for (target, what) in [
            (offset.grip, "the offset track's band"),
            (group.tap, "the tap capsule"),
            (group.halve, "the octave's `½`"),
            (group.double, "the octave's `×2`"),
        ] {
            assert!(
                target.min.y - strip.y >= GRAB,
                "{what} is {} from the top of the row and a grab is {GRAB}",
                target.min.y - strip.y
            );
            assert!(
                (strip.y + strip.h) - target.max.y >= GRAB,
                "{what} is {} from the bottom of the row and a grab is {GRAB}",
                (strip.y + strip.h) - target.max.y
            );
        }
        // And the panel agrees: a press in the middle of each is the panel's.
        for (target, what) in [
            (offset.grip, "the offset track's band"),
            (group.tap, "the tap capsule"),
            (group.halve, "the octave's `½`"),
        ] {
            assert_eq!(
                claim(&mut panel, &ctx, &view, at(target.center())),
                Claim::Panel,
                "`claim` gives a press on {what} to `egui`, so no route into the operation \
                 exists however the control is drawn"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// What a press asks for
// ---------------------------------------------------------------------------

/// **Each of the three, pressed, names its own operation.**
#[test]
fn a_press_on_each_of_the_three_asks_for_its_operation() {
    let (panel, ctx) = console(PLAUSIBLE);
    let group = laid_out(&panel, &ctx, mock_tracker());
    let offset = group.offset.expect("an offset");

    assert_eq!(
        group.tapped(at(group.tap.center())),
        Some(Operation::TapBeat),
        "a press on the tap capsule does not ask for a tap"
    );
    assert_eq!(
        group.octave(at(group.halve.center())),
        Some(Operation::ScaleGrid {
            by: GridScale::Halve
        }),
        "a press on `½` does not ask for the grid to be halved"
    );

    // The track, at three points that can be named exactly.
    for (x, ms, what) in [
        (offset.track.min.x, LATENCY_OFFSET_MIN_MS, "the left end"),
        (offset.track.center().x, 0.0, "the middle"),
        (offset.track.max.x, LATENCY_OFFSET_MAX_MS, "the right end"),
    ] {
        let asked = group
            .nudge(Point::new(x, offset.grip.center().y))
            .unwrap_or_else(|| panic!("a press on {what} of the track asks for nothing"));
        assert_eq!(
            asked,
            Operation::SetLatencyOffset { ms },
            "{what} of the track does not ask for {ms} ms"
        );
    }

    // And nothing outside the three, which is `a control claims what it acts
    // on and no more`: the gap between the tap and the octave is nobody's.
    let between = Point::new(
        (group.tap.max.x + group.halve.min.x) * 0.5,
        group.tap.center().y,
    );
    assert!(
        !group.owns(between),
        "the gap between two controls is claimed"
    );
}

/// **The refused half is drawn, and it is not claimed.**
///
/// `DeckHead::arrow`'s rule one bay over, and the mock's own picture: `×2` at
/// 128.0 keeps its shape and its tooltip says why. The rectangle is still
/// there — this is not the assertion passing on a control that went away.
#[test]
fn the_refused_half_of_the_octave_is_drawn_and_not_claimed() {
    let (mut panel, ctx) = console(PLAUSIBLE);
    let reading = mock_tracker();
    let group = laid_out(&panel, &ctx, reading);
    assert!(
        group.double.width() > 0.0,
        "the refused half has no rectangle at all, so what follows would pass on its absence"
    );
    assert_eq!(
        group.octave(at(group.double.center())),
        None,
        "a press on the refused half asks for a move the beat lock would turn down"
    );
    assert!(
        !group.owns(at(group.double.center())),
        "the refused half is claimed, so the press is taken off `egui` and then dropped"
    );
    let view = view(reading);
    assert_eq!(
        claim(&mut panel, &ctx, &view, at(group.double.center())),
        Claim::Egui,
        "`claim` takes a press on an inert chip off `egui` and acts on nothing"
    );

    // The other way round, at a tempo where doubling is what is available.
    let slow = Tracker {
        halve: false,
        double: true,
        ..reading
    };
    let other = laid_out(&panel, &ctx, slow);
    assert!(
        other.halve.width() > 0.0,
        "the half that is refused at this tempo has no rectangle either"
    );
    assert_eq!(other.octave(at(other.halve.center())), None);
    assert_eq!(
        other.octave(at(other.double.center())),
        Some(Operation::ScaleGrid {
            by: GridScale::Double
        })
    );
}

// ---------------------------------------------------------------------------
// What is not drawn, and why
// ---------------------------------------------------------------------------

/// **With nothing open there is no offset**, and the tap closes up behind it.
///
/// An offset belongs to a session and `karakuri_environment::audio`'s default
/// is where the *next* one starts, so a track drawn on a console with nothing
/// open would be pointing at a number nobody chose. The tap and the octave are
/// drawn either way, because what they ask for does not depend on a reading.
#[test]
fn with_nothing_open_there_is_no_offset_track() {
    let (panel, ctx) = console(PLAUSIBLE);
    let pill =
        audio_in(&ctx, panel.layout(), Some(mock()), Some(&heard())).expect("the audio-in pill");
    let shut = Tracker {
        offset_ms: None,
        ..mock_tracker()
    };
    let group = laid_out(&panel, &ctx, shut);
    assert_eq!(
        group.offset, None,
        "a track was drawn at a value nothing holds"
    );
    assert_eq!(
        group.nudge(at(group.tap.center())),
        None,
        "a press asked the offset to become something on a console with no session"
    );
    assert!(
        near(group.tap.min.x, pill.pill.max.x + size::PILL_GAP),
        "the tap did not close up behind the offset that is not drawn"
    );
}

/// **A console nobody has told anything about the tracker draws none of it**,
/// and the arrangement pill is where it was before this group existed.
///
/// `View::audio`'s rule one control to the left: *empty is a state and unasked
/// is not*.
#[test]
fn a_console_told_nothing_about_the_tracker_draws_none_of_it() {
    let (panel, ctx) = console(PLAUSIBLE);
    assert_eq!(
        tracker_group(&ctx, panel.layout(), Some(mock()), Some(&heard()), None),
        None,
        "a console that was told nothing about the tracker drew a tap anyway"
    );
    let pill =
        audio_in(&ctx, panel.layout(), Some(mock()), Some(&heard())).expect("the audio-in pill");
    let after = arrangement(
        &ctx,
        panel.layout(),
        Some(mock()),
        Some(&heard()),
        None,
        &Arrangement::NONE,
    )
    .expect("the arrangement pill");
    assert!(
        near(after.pill.min.x, pill.pill.max.x + size::TRANSPORT_GAP),
        "with no tracker group the arrangement pill does not fall back to the pill's edge"
    );

    // And no row means none of it either, which is the whole module's rule.
    assert_eq!(
        tracker_group(
            &ctx,
            panel.layout(),
            None,
            Some(&heard()),
            Some(mock_tracker())
        ),
        None,
        "the group was drawn in a transport row that is not there"
    );
}

// ---------------------------------------------------------------------------
// The route a window loop takes
// ---------------------------------------------------------------------------

/// **`claim`, then the derivation, then the operation** — the three steps a
/// window loop takes on a press, over each of the three controls.
///
/// `tests/look.rs`'s last pass over two more controls: it is the whole route
/// and not the pieces, because a control can be claimed and never asked, and
/// that is exactly the defect `karakuri/src/main.rs`'s press-handler check
/// exists for.
#[test]
fn the_route_a_press_takes_reaches_the_operation() {
    let (mut panel, ctx) = console(PLAUSIBLE);
    let reading = mock_tracker();
    let view = view(reading);
    let group = laid_out(&panel, &ctx, reading);
    let offset = group.offset.expect("an offset");

    for (point, expected, what) in [
        (group.tap.center(), Operation::TapBeat, "the tap capsule"),
        (
            group.halve.center(),
            Operation::ScaleGrid {
                by: GridScale::Halve,
            },
            "the octave's `½`",
        ),
        (
            offset.grip.center(),
            Operation::SetLatencyOffset { ms: 0.0 },
            "the middle of the offset track",
        ),
    ] {
        let p = at(point);
        assert_eq!(
            claim(&mut panel, &ctx, &view, p),
            Claim::Panel,
            "{what} is not claimed, so a window would hand the press to `egui`"
        );
        let asked = tracker_group(
            &ctx,
            panel.layout(),
            view.transport,
            view.audio.as_ref(),
            view.tracker,
        )
        .and_then(|group| {
            group
                .tapped(p)
                .or_else(|| group.octave(p))
                .or_else(|| group.nudge(p))
        });
        assert_eq!(
            asked,
            Some(expected),
            "a press on {what} did not reach the operation the page names"
        );
    }
}
