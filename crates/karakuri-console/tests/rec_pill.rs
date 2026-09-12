//! The `rec` pill: the transport row's one control, and a record/stop toggle.
//!
//! `docs/manual/console.html` draws one capsule at the very end of
//! `.transport`, after `landed`, and `docs/manual/operations.html` gives it one
//! row — *Record the session*. It is one control with two ends: a press starts
//! a recording and a press stops one, and which of those a press is is what the
//! pill is already showing.
//!
//! Seven things, and the first two are why this is its own file rather than
//! more assertions in `tests/transport.rs`:
//!
//! 1. Where it is: last in the row, against the row's right padding, with the
//! health capsule and the frame readout laid out backwards from it. That end of
//! the row belonged to the capsule until this pill existed, and the two
//! readings have to be one derivation. 2. That a console nobody told draws no
//! pill at all, and that the row is then exactly the row it was — which is what
//! keeps `tests/transport.rs` describing a console with no program behind it.
//! 3. That it clears every boundary's grab, which is every control on this
//! console's own debt and is never inherited from the control beside it. 4.
//! What a press on it asks for, at both ends of the toggle — and that the
//! payload is read off the state the pill was drawn from, so the capsule an
//! operator is looking at and the operation the press names cannot come apart.
//! 5. That it is drawn, in the mock's two treatments and in no third one: a
//! rectangle is not a drawing, so the mark and the word are read off the frame.
//! 6. That a start files under no id, which is what makes each one a fresh
//! recording (ADR-0289). 7. The route a window loop actually takes — `claim`,
//! then the derivation that drew the control, then the operation.
//!
//! None of it needs a window, a device or a disk. It does need `egui`'s fonts,
//! because the pill is as wide as the word in it — see `common::drawn_once`.

mod common;

use common::{drawn_once, near, rect_of, PLAUSIBLE};
use karakuri_console::input::{claim, Claim};
use karakuri_console::panel::{Panel, GRAB};
use karakuri_console::room::{size, Room};
use karakuri_console::view::{transport, Rec, Stage, Transport, TransportRow, View};
use karakuri_layout::{Point, Rect};
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

/// A panel at a viewport, solved, with a context that has drawn once.
fn console(viewport: Rect) -> (Panel, egui::Context) {
    let mut panel = Panel::new(viewport.w, viewport.h);
    panel.solve();
    (panel, drawn_once())
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

/// The pill ends the row, and everything after the `.sep` is laid out backwards
/// from it.
///
/// The mock's order at that end is `landed`, then `● rec`, hard against
/// `.transport`'s own padding. Until this control existed the capsule took that
/// padding and `transport.rs` asserts that it does where there is no pill; this
/// is the same assertion one item further right, and the two together are what
/// says the right-hand end has one derivation rather than two.
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

/// The pill is the same width at both ends of the toggle, so it does not move
/// under the hand that is pressing it.
///
/// The mock writes one mark and one word and changes only the treatment between
/// the two states, so this is a fact about the derivation rather than a
/// coincidence of two strings — and a pill that grew when a recording started
/// would take the health capsule and the frame readout with it, on the frame
/// after the press.
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

/// A console nobody has told about recording draws no pill, and the row it
/// draws is the row it drew before this control existed.
///
/// `None` is *nobody said* rather than *not recording* — `View::audio`'s
/// distinction one group along — and the whole of what it buys is here: the
/// health capsule has the right padding back, which is exactly what
/// `tests/transport.rs` asserts of a console with no program behind it.
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

/// The pill clears every boundary's grab, which is `input.rs`'s rule 4 and the
/// debt every control on this console owes.
///
/// The row is 48 and a pill is 16.5, centred, so there is (48 − 16.5) / 2 =
/// 15.75 of row above the capsule and 15.75 below, against a [`GRAB`] of 6.
/// That is the tap capsule's own arithmetic one group along, and it is measured
/// here rather than inherited: this pill is at the *other* end of the row, so
/// what it also has to clear is the right-hand edge of the console — and it is
/// [`size::TRANSPORT_PAD_X`]'s 12 in from it, which is twice the grab.
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

/// A press beside the pill is `egui`'s, which is what says the claim is the
/// pill's rectangle and not the row's.
///
/// The point is one gap to the left of the capsule, which is the space the
/// `.sep` leaves between it and the health capsule — panel ground with nothing
/// drawn on it.
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

/// A press asks for the other end of the toggle, and the payload is read off
/// the state the pill was drawn from.
///
/// This is the whole of the decision the control is: one capsule, two
/// operations, and which one a press is is a fact about what the operator can
/// already see. A `Start` that could be pressed while something was recording,
/// or a `Stop` while nothing was, would be a control whose picture and whose
/// effect are two different statements.
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

/// A start names no id, which is what makes every press a fresh recording
/// rather than a second head in a stream that already exists.
///
/// `Store::append_session` appends and `session::split` sets `started` at the
/// first tick and never clears it, so an id typed twice is a stream read back
/// as edits. A capsule types no name and the payload says so — ADR-0289, and
/// the `keep` capsule's own arrangement one bay over.
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

/// The pill is painted, and in the mock's two treatments.
///
/// A rectangle is not a drawing: the row would satisfy every geometric
/// assertion above with a `transport_into` that laid the capsule out and never
/// painted it. So this reads the frame — the word, the mark beside it, and
/// which of `.pill` and `.pill.on` the capsule is wearing.
///
/// `rect_filled` against `rect_stroke` is the difference, exactly as the health
/// capsule's own test reads it: the `on` treatment fills the capsule with a
/// wash of the pink and the plain one draws a hairline round nothing.
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
        // **The mark is drawn rather than typed**, which is `Mask`'s rule: the
        // mock's `&#9679;` is a glyph nobody here chose a font for, so it is a
        // circle. A pill that had typed it would draw a second galley and no
        // circle at all.
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

/// `claim`, then the derivation, then the operation — the three steps
/// `crates/karakuri/src/main.rs` takes on a press, in that order.
///
/// The seam this stands under is the one `mod press_handler` in that file is
/// about: a control drawn and claimed here has to be *asked* there, and this is
/// the half of it that can be checked without a window.
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
