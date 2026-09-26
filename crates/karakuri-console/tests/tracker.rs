//! Layout and hit-testing for tracker controls: latency offset, tap tempo, and octave chips.
//!
//! Validates positioning, grab clearances, disabled chip retention, offset track scaling,
//! and session-dependent visibility.

mod common;

use common::{at, console, near, rect_of, PLAUSIBLE, SMALLEST};
use karakuri_console::input::{claim, Claim};
use karakuri_console::panel::{Panel, GRAB};
use karakuri_console::room::{size, Room};
use karakuri_console::view::{
    arrangement, audio_in, offset_at, tracker_group, unit_of_offset, Arrangement, AudioIn, Tracker,
    TrackerGroup, Transport, View, LATENCY_OFFSET_MAX_MS, LATENCY_OFFSET_MIN_MS,
    LATENCY_OFFSET_STEP_MS, OFFSET_TRACK_W,
};
use karakuri_layout::Point;
use karakuri_operation::{GridScale, Operation};

/// The mock's own transport, as numbers — `transport.rs`'s, which is where the
/// argument for each of them is. The group is measured from the audio-in pill
/// and the pill from the bar, so a row is needed to have any of it.
fn mock() -> Transport {
    common::mock_transport()
}

/// Configures mock tracker state with active offset, active half-tempo, and disabled double-tempo.
fn mock_tracker() -> Tracker {
    Tracker {
        offset_ms: Some(-15.0),
        halve: true,
        double: false,
    }
}

/// A console that has been told about audio and found something, which is what
/// the mock draws: the group's head is a pill and the three below are laid out
/// from it.
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

/// The laid-out group at that reading, on a solved console.
fn laid_out(panel: &Panel, ctx: &egui::Context, at: Tracker) -> TrackerGroup {
    tracker_group(ctx, panel.layout(), Some(mock()), Some(&heard()), Some(at))
        .expect("the transport row draws the tracker group")
}

// ---------------------------------------------------------------------------
// Where they are
// ---------------------------------------------------------------------------

/// Places tracker controls sequentially using `PILL_GAP` and offsets subsequent groups by `TRANSPORT_GAP`.
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
        None,
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

/// Sizing the track to 80px maps 1px to each 5ms step between -200ms and +200ms.
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

/// Linear interpolation maps track ends exactly to -200ms and +200ms with midpoint at 0ms.
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

/// All tracker controls clear boundary grab regions with vertical clearances exceeding 16px.
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

/// Each of the three, pressed, names its own operation.
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

/// The disabled octave half retains its rendered shape without claiming pointer presses.
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

/// Without an active session, the offset track is omitted and remaining controls shift left.
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

/// When tracker state is omitted, the entire tracker group is hidden and adjacent controls shift left.
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

/// Verifies the full press processing sequence: claim, control derivation, and operation dispatch.
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
