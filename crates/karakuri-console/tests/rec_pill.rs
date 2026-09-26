//! Verification for the transport `rec` pill toggle (ADR-0289).
//!
//! Tests layout positioning at the end of the transport row, grab clearing,
//! press event emission for starting/stopping recordings, and visual states.

mod common;

use common::{console, drawn_once, near, rect_of, PLAUSIBLE};
use karakuri_console::input::{claim, Claim};
use karakuri_console::panel::{Panel, GRAB};
use karakuri_console::room::{size, Room};
use karakuri_console::view::{transport, Rec, Stage, Transport, TransportRow, View};
use karakuri_layout::Point;
use karakuri_operation::{Operation, Recording};

/// The mock's own transport, as numbers — `transport.rs`'s, which is where the
/// argument for each of them is, with the mock's `rec` pill added: the capsule
/// is drawn in `.pill.on` on that page, which is a recording running.
fn mock() -> Transport {
    Transport {
        rec: Some(Rec::Running),
        ..common::mock_transport()
    }
}

/// The row, drawn with `values`.
fn row(panel: &Panel, ctx: &egui::Context, values: Transport) -> TransportRow {
    transport(ctx, panel.layout(), Some(values)).expect("the transport row draws its readouts")
}

/// The `transport` region, as an `egui` rectangle.
fn strip(panel: &Panel) -> egui::Rect {
    let r = rect_of(panel.layout(), "transport");
    egui::Rect::from_min_size(egui::pos2(r.x, r.y), egui::vec2(r.w, r.h))
}

/// The middle of a rectangle, as the panel's own point type.
fn middle(r: egui::Rect) -> Point {
    Point {
        x: r.center().x,
        y: r.center().y,
    }
}

// ---------------------------------------------------------------------------
// Where it is
// ---------------------------------------------------------------------------

/// The rec pill anchors the right end of the transport row, positioned against padding.
#[test]
fn the_rec_pill_takes_the_rows_right_padding_and_everything_else_backs_away_from_it() {
    let (panel, ctx) = console(PLAUSIBLE);
    let strip = strip(&panel);
    let row = row(&panel, &ctx, mock());

    let rec = row
        .rec
        .expect("a console told about recording draws the pill");
    assert!(
        near(strip.max.x - rec.max.x, size::TRANSPORT_PAD_X),
        "the pill ends {} from the right edge and the padding is {}",
        strip.max.x - rec.max.x,
        size::TRANSPORT_PAD_X
    );
    assert!(near(rec.height(), size::PILL_H));
    assert!(
        near(rec.center().y, strip.center().y),
        "the pill is not centred in the row"
    );

    // The health capsule is one `.transport` gap before it, and the frame
    // readout one gap before that — which is the row the mock draws, read from
    // the right.
    let health = row.health.expect("the mock draws `landed`");
    assert!(
        near(rec.min.x - health.max.x, size::TRANSPORT_GAP),
        "the capsule ends {} before the pill and the gap is {}",
        rec.min.x - health.max.x,
        size::TRANSPORT_GAP
    );
    assert!(
        near(health.min.x - row.frame.max.x, size::TRANSPORT_GAP),
        "the frame readout ends {} before the capsule",
        health.min.x - row.frame.max.x
    );

    // And nothing in the row overlaps anything else in it: the pill was added
    // to an end that was already full.
    for (a, name_a) in [
        (row.bpm, "the tempo"),
        (row.grid, "the beat grid"),
        (row.bar, "the bar"),
        (row.frame, "the frame readout"),
        (health, "the health capsule"),
        (rec, "the rec pill"),
    ] {
        for (b, name_b) in [
            (row.bpm, "the tempo"),
            (row.grid, "the beat grid"),
            (row.bar, "the bar"),
            (row.frame, "the frame readout"),
            (health, "the health capsule"),
            (rec, "the rec pill"),
        ] {
            if name_a == name_b {
                continue;
            }
            assert!(
                !a.intersects(b),
                "{name_a} and {name_b} overlap: {a:?} against {b:?}"
            );
        }
    }
}

/// Pill width remains constant across recording states so controls do not shift under presses.
#[test]
fn the_pill_does_not_move_when_the_recording_starts() {
    let (panel, ctx) = console(PLAUSIBLE);
    let idle = row(
        &panel,
        &ctx,
        Transport {
            rec: Some(Rec::Idle),
            ..mock()
        },
    );
    let running = row(
        &panel,
        &ctx,
        Transport {
            rec: Some(Rec::Running),
            ..mock()
        },
    );
    assert_eq!(idle.rec, running.rec);
    assert_eq!(idle.health, running.health);
    assert_eq!(idle.frame, running.frame);
}

/// When recording state is None, the rec pill is omitted and health capsule takes right padding.
#[test]
fn a_console_nobody_told_draws_no_pill_and_the_capsule_has_the_padding_back() {
    let (panel, ctx) = console(PLAUSIBLE);
    let strip = strip(&panel);
    let untold = row(
        &panel,
        &ctx,
        Transport {
            rec: None,
            ..mock()
        },
    );
    assert_eq!(untold.rec, None);
    let health = untold.health.expect("the mock draws `landed`");
    assert!(
        near(strip.max.x - health.max.x, size::TRANSPORT_PAD_X),
        "with no pill the capsule ends {} from the right edge and the padding is {}",
        strip.max.x - health.max.x,
        size::TRANSPORT_PAD_X
    );

    // And with neither, the frame readout has it — the row two controls ago,
    // which is what a run with nothing written and nothing recorded draws.
    let bare = row(
        &panel,
        &ctx,
        Transport {
            health: None,
            rec: None,
            ..mock()
        },
    );
    assert_eq!(bare.rec, None);
    assert_eq!(bare.health, None);
    assert!(near(strip.max.x - bare.frame.max.x, size::TRANSPORT_PAD_X));
}

// ---------------------------------------------------------------------------
// The grab, and the claim
// ---------------------------------------------------------------------------

/// The pill clears all boundary grab zones both vertically and from the right console edge.
#[test]
fn the_pill_clears_every_boundarys_grab() {
    let (mut panel, ctx) = console(PLAUSIBLE);
    let strip = strip(&panel);
    let row = row(&panel, &ctx, mock());
    let rec = row.rec.expect("a pill");

    let above = rec.min.y - strip.min.y;
    let below = strip.max.y - rec.max.y;
    assert!(
        above > GRAB && below > GRAB,
        "the pill has {above} of row above it and {below} below, against a grab of {GRAB}"
    );
    assert!(
        strip.max.x - rec.max.x > GRAB,
        "the pill is {} from the row's right edge, against a grab of {GRAB}",
        strip.max.x - rec.max.x
    );

    // And the press actually reaches the panel, which is the fact the
    // arithmetic is about.
    let mut view = View::new(Room::Day);
    view.transport = Some(mock());
    assert_eq!(
        claim(&mut panel, &ctx, &view, middle(rec)),
        Claim::Panel,
        "a press in the middle of the pill did not reach the panel"
    );
}

/// Presses adjacent to the pill fall through to egui, ensuring strict hit boundaries.
#[test]
fn a_press_in_the_gap_before_the_pill_is_not_the_pills() {
    let (mut panel, ctx) = console(PLAUSIBLE);
    let mut view = View::new(Room::Day);
    view.transport = Some(mock());
    let row = row(&panel, &ctx, mock());
    let rec = row.rec.expect("a pill");
    let beside = Point {
        x: rec.min.x - size::TRANSPORT_GAP * 0.5,
        y: rec.center().y,
    };
    assert_eq!(row.record(beside), None);
    assert_eq!(claim(&mut panel, &ctx, &view, beside), Claim::Egui);
}

/// A console nobody told claims nothing there, because there is nothing there:
/// the pill is not drawn, so the point it would have been at is panel ground.
#[test]
fn a_console_nobody_told_claims_no_press_where_the_pill_would_be() {
    let (mut panel, ctx) = console(PLAUSIBLE);
    let told = row(&panel, &ctx, mock());
    let at = middle(told.rec.expect("a pill"));

    let mut view = View::new(Room::Day);
    view.transport = Some(Transport {
        rec: None,
        ..mock()
    });
    let untold = row(
        &panel,
        &ctx,
        Transport {
            rec: None,
            ..mock()
        },
    );
    assert!(!untold.on_rec(at));
    assert_eq!(untold.record(at), None);
    assert_eq!(claim(&mut panel, &ctx, &view, at), Claim::Egui);
}

// ---------------------------------------------------------------------------
// What a press asks for
// ---------------------------------------------------------------------------

/// Clicking the pill toggles between `StartRecording` and `StopRecording` based on current state.
#[test]
fn a_press_starts_a_recording_and_a_press_stops_the_one_running() {
    let (panel, ctx) = console(PLAUSIBLE);

    let idle = row(
        &panel,
        &ctx,
        Transport {
            rec: Some(Rec::Idle),
            ..mock()
        },
    );
    let at = middle(idle.rec.expect("a pill"));
    assert_eq!(
        idle.record(at),
        Some(Operation::RecordSession {
            recording: Recording::Start { id: None }
        }),
        "a press with nothing running has to ask to begin one"
    );

    let running = row(
        &panel,
        &ctx,
        Transport {
            rec: Some(Rec::Running),
            ..mock()
        },
    );
    assert_eq!(
        running.record(at),
        Some(Operation::RecordSession {
            recording: Recording::Stop
        }),
        "a press with something running has to ask to end it"
    );
}

/// Starting a recording provides `id: None` to create a fresh session without reuse (ADR-0289).
#[test]
fn a_start_files_under_no_id_so_every_press_is_a_fresh_recording() {
    let (panel, ctx) = console(PLAUSIBLE);
    let row = row(
        &panel,
        &ctx,
        Transport {
            rec: Some(Rec::Idle),
            ..mock()
        },
    );
    let at = middle(row.rec.expect("a pill"));
    match row.record(at) {
        Some(Operation::RecordSession {
            recording: Recording::Start { id },
        }) => assert_eq!(
            id, None,
            "the pill named a recording and it cannot type one"
        ),
        other => panic!("a press on an idle pill asked for {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// That it is drawn
// ---------------------------------------------------------------------------

/// Every shape a frame put inside `rect`.
fn shapes_inside(view: &mut View, panel: &mut Panel, rect: egui::Rect) -> Vec<egui::Shape> {
    let ctx = drawn_once();
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| view.draw(ui, panel));
    out.textures_delta.clear();
    out.shapes
        .into_iter()
        .filter(|clipped| {
            let bounds = clipped.shape.visual_bounding_rect();
            bounds.is_finite() && rect.contains_rect(bounds)
        })
        .map(|clipped| clipped.shape)
        .collect()
}

/// Validates rendered shapes and colors for default outline and active filled styles.
#[test]
fn the_pill_is_drawn_and_says_which_state_it_is_in() {
    let mut panel = Panel::new(PLAUSIBLE.w, PLAUSIBLE.h);
    panel.solve();
    let mut view = View::new(Room::Day);

    for (rec, filled) in [(Rec::Running, true), (Rec::Idle, false)] {
        let values = Transport {
            health: Some(Stage::Landed),
            rec: Some(rec),
            ..mock()
        };
        view.transport = Some(values);
        let capsule = transport(&drawn_once(), panel.layout(), Some(values))
            .expect("a row")
            .rec
            .expect("a pill");
        let shapes = shapes_inside(&mut view, &mut panel, capsule);

        assert!(
            shapes.iter().any(|shape| matches!(
                shape,
                egui::Shape::Text(text) if text.galley.text() == "rec"
            )),
            "the {rec:?} pill drew no word: {shapes:?}"
        );
        // The record status dot is rendered as a geometric circle rather than text glyph.
        assert!(
            shapes
                .iter()
                .any(|shape| matches!(shape, egui::Shape::Circle(_))),
            "the {rec:?} pill drew no mark: {shapes:?}"
        );
        assert_eq!(
            filled,
            shapes.iter().any(|shape| matches!(
                shape,
                egui::Shape::Rect(rect) if rect.fill != egui::Color32::TRANSPARENT
            )),
            "the {rec:?} pill's treatment is wrong: {shapes:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// The route a window loop takes
// ---------------------------------------------------------------------------

/// Verifies the input claim, derivation, and operation sequence for press handling.
#[test]
fn the_route_a_window_takes_reaches_the_operation() {
    let (mut panel, ctx) = console(PLAUSIBLE);
    let mut view = View::new(Room::Day);
    view.transport = Some(mock());

    let at = middle(row(&panel, &ctx, mock()).rec.expect("a pill"));
    assert_eq!(claim(&mut panel, &ctx, &view, at), Claim::Panel);
    let asked = transport(&ctx, panel.layout(), view.transport)
        .and_then(|row| row.record(at))
        .expect("the derivation that drew the pill answers the press that claimed it");
    assert_eq!(
        asked,
        Operation::RecordSession {
            recording: Recording::Stop
        }
    );
}
