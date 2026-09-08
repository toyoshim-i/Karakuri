//! **The transport row's look controls: the console's seventh and eighth, and
//! its first that a press sets outright.**
//!
//! Seven things, and the first two are why this is its own file rather than a
//! few more assertions in `arrangement_pill.rs`:
//!
//! 1. Where the two controls are, derived from the pill's own right edge.
//! 2. **That both clear every boundary's grab**, which is
//!    `tests/arrangement_pill.rs`'s arithmetic over two more controls and is
//!    never inherited from it: the row is 48, both targets are 16.5, so the
//!    clearance is 15.75 against a `GRAB` of 6 — measured here, and the
//!    exposure control's target is the *track grown to a line's height*
//!    rather than the 5px track, which is the number that matters.
//! 3. **That the capsule does not move under the word it names**, which is
//!    what stops the exposure track walking away as the tone map is cycled.
//! 4. That the capsule cycles all four operators and comes back, once each.
//! 5. **What a press on the track asks for**: the value at the point it
//!    landed, exactly at both ends and exactly 1.00 in the middle.
//! 6. That one pixel of that track is one press of an exposure key, which is
//!    the whole of why it is 48 wide.
//! 7. **The route a window loop actually takes** — `claim`, then the
//!    derivation that drew the control, then the operation — which is
//!    `tests/vocabulary.rs`'s third pass over these two controls.
//!
//! None of it needs a window, a device or a disk. It does need `egui`'s fonts,
//! because the capsule is as wide as the widest name it can hold — see
//! `common::drawn_once` — a `Transport`, because the group is measured from
//! the arrangement pill and the pill from the bar, and a `Look`, because a
//! console with no engine behind it draws no look at all.

mod common;

use common::{drawn_once, near, rect_of, PLAUSIBLE, SMALLEST};
use karakuri_console::input::{claim, Claim};
use karakuri_console::panel::{Panel, GRAB};
use karakuri_console::room::{size, Room};
use karakuri_console::view::{
    arrangement, exposure_at, look, unit_of, Arrangement, Look, LookRow, Transport, View,
    EXPOSURE_MAX, EXPOSURE_MIN, EXPOSURE_TRACK_W,
};
use karakuri_layout::{Point, Rect};
use karakuri_operation::{Operation, Tonemap};

/// **The mock's own transport, as numbers** — `transport.rs`'s, which is where
/// the argument for each of them is. The look group is measured from the
/// arrangement pill and the pill from the bar, so a row is needed to have
/// either.
fn mock() -> Transport {
    common::mock_transport()
}

/// **The mock's own look**: `tone · aces` at `exp 1.00`, which is what
/// `docs/manual/console.html`'s transport draws and what
/// `crates/karakuri/src/main.rs`'s `LOOK` opens a window under.
fn mock_look() -> Look {
    Look {
        tonemap: Tonemap::Aces,
        exposure: 1.0,
    }
}

/// A view with an engine behind it, that look in front of it, and the default
/// arrangement — what `View::draw` paints from and what `claim` hit-tests, one
/// value.
fn view(at: Look) -> View {
    let mut view = View::new(Room::Day);
    view.transport = Some(mock());
    view.look = Some(at);
    view
}

/// A panel at a viewport, solved, with a context that has drawn once — the
/// pair every test here starts from, and `arrangement_pill.rs`'s own opening.
fn console(viewport: Rect) -> (Panel, egui::Context) {
    let mut panel = Panel::new(viewport.w, viewport.h);
    panel.solve();
    (panel, drawn_once())
}

/// The laid-out group at that look, on a solved console.
fn group(panel: &Panel, ctx: &egui::Context, at: Look) -> LookRow {
    look(
        ctx,
        panel.layout(),
        Some(mock()),
        None,
        None,
        &Arrangement::NONE,
        Some(at),
    )
    .expect("the transport row draws the look controls")
}

/// A `karakuri_layout` point, from `egui`'s.
fn at(p: egui::Pos2) -> Point {
    Point::new(p.x, p.y)
}

/// Every operator, which is what this file walks the cycle against. The order
/// is deliberately **not** the cycle's: what is asserted below is that the
/// cycle visits each of these once, and a list written in the cycle's own
/// order could not tell that from a cycle that had lost one.
const EVERY: [Tonemap; 4] = [
    Tonemap::AgX,
    Tonemap::Clamp,
    Tonemap::Aces,
    Tonemap::Reinhard,
];

// ---------------------------------------------------------------------------
// Where the controls are
// ---------------------------------------------------------------------------

/// **The group is the row's own geometry**, laid out from the arrangement
/// pill's right edge — which is the mock's flex row with the six controls it
/// does not draw taken out.
///
/// `.transport`'s `gap` is 14 between the row's items, and the exposure
/// label, its track and its figure are one item at `.trim`'s own 5.
#[test]
fn the_look_group_is_the_rows_own_geometry() {
    let (panel, ctx) = console(SMALLEST);
    let strip = rect_of(panel.layout(), "transport");
    let pill = arrangement(
        &ctx,
        panel.layout(),
        Some(mock()),
        None,
        None,
        &Arrangement::NONE,
    )
    .expect("the row draws the pill");
    let row = group(&panel, &ctx, mock_look());

    assert!(
        near(row.tone.min.x, pill.pill.max.x + size::TRANSPORT_GAP),
        "the capsule starts {} after the arrangement pill and `.transport`'s gap is {}",
        row.tone.min.x - pill.pill.max.x,
        size::TRANSPORT_GAP
    );
    assert!(
        near(row.tone.height(), size::PILL_H),
        "the capsule is {} tall and a pill is 11 at line-height 1.5",
        row.tone.height()
    );
    assert!(near(row.tone.center().y, strip.y + strip.h * 0.5));

    // The exposure group: `exp`, one `.trim` gap, the track, one more, the
    // figure — and the whole of it one `.transport` gap after the capsule.
    assert!(near(row.label.min.x, row.tone.max.x + size::TRANSPORT_GAP));
    assert!(near(row.track.min.x, row.label.max.x + size::TRIM_GAP));
    assert!(near(row.value.min.x, row.track.max.x + size::TRIM_GAP));
    assert!(
        near(row.track.width(), EXPOSURE_TRACK_W) && near(row.track.height(), size::FADER_H),
        "the track is {} by {} and `.fader` at 48 wide is {EXPOSURE_TRACK_W} by {}",
        row.track.width(),
        row.track.height(),
        size::FADER_H
    );
    assert!(near(row.track.center().y, strip.y + strip.h * 0.5));

    // The whole of it is inside the row, and clear of the frame readout at the
    // other end.
    let frame = karakuri_console::view::transport(&ctx, panel.layout(), Some(mock()))
        .expect("a row")
        .frame;
    let bounds =
        egui::Rect::from_min_size(egui::pos2(strip.x, strip.y), egui::vec2(strip.w, strip.h));
    assert!(
        bounds.contains_rect(row.tone),
        "the capsule escapes the row"
    );
    assert!(bounds.contains_rect(row.grip), "the target escapes the row");
    assert!(
        row.value.max.x + size::TRANSPORT_GAP <= frame.min.x,
        "the figure ends at {} and the frame readout starts at {}",
        row.value.max.x,
        frame.min.x
    );
}

/// **The fill is where the value is**, from the left, and at 1.00 it is
/// exactly half the track — which is what *1.0 at the middle* means and is the
/// only reading under which both halves of the range are usable.
#[test]
fn the_fill_is_the_level_and_unity_is_the_middle() {
    let (panel, ctx) = console(PLAUSIBLE);
    let row = group(&panel, &ctx, mock_look());
    assert!(near(row.fill.min.x, row.track.min.x));
    assert!(
        near(row.fill.width(), row.track.width() * 0.5),
        "1.00 filled {} of a {} track and the middle is half of it",
        row.fill.width(),
        row.track.width()
    );

    let floor = group(&panel, &ctx, at_exposure(EXPOSURE_MIN));
    assert!(near(floor.fill.width(), 0.0));
    let ceiling = group(&panel, &ctx, at_exposure(EXPOSURE_MAX));
    assert!(near(ceiling.fill.width(), row.track.width()));
}

fn at_exposure(exposure: f32) -> Look {
    Look {
        exposure,
        ..mock_look()
    }
}

/// **The capsule is the widest of the four whatever word is in it**, which is
/// the tally chip's rule and not the blend chip's: a target sized to the word
/// it shows moves under the hand pressing it, and here it would take the
/// exposure track with it.
#[test]
fn the_capsule_does_not_move_under_the_word_it_names() {
    let (panel, ctx) = console(PLAUSIBLE);
    let first = group(&panel, &ctx, mock_look());
    for tonemap in EVERY {
        let row = group(
            &panel,
            &ctx,
            Look {
                tonemap,
                ..mock_look()
            },
        );
        assert_eq!(
            row.tone,
            first.tone,
            "the capsule is a different box showing `{}`, so cycling the tone map moves the \
             control that is being pressed",
            tonemap.name()
        );
        assert_eq!(
            row.track,
            first.track,
            "the exposure track moved when the tone map was `{}`, so setting the level means \
             chasing a control that walked away",
            tonemap.name()
        );
        // The words are inside the capsule and centred in it, which is what
        // *as wide as the widest* comes to for the other three.
        assert!(
            row.tone.contains_rect(row.tone_text),
            "`{}` is painted outside its own capsule",
            tonemap.name()
        );
        assert!(near(row.tone_text.center().x, row.tone.center().x));
    }
}

/// **The figure is after the track and it is the one thing here that moves**,
/// which is why it is last: only the `.sep` is behind it.
#[test]
fn only_the_figure_moves_with_the_value() {
    let (panel, ctx) = console(PLAUSIBLE);
    let unity = group(&panel, &ctx, mock_look());
    let wide = group(&panel, &ctx, at_exposure(EXPOSURE_MAX));
    assert_eq!(wide.tone, unity.tone);
    assert_eq!(wide.track, unity.track);
    assert_eq!(wide.grip, unity.grip);
    assert!(
        wide.value.width() > unity.value.width(),
        "`64.00` and `1.00` laid out to the same width, so this test cannot tell whether the \
         figure moves anything"
    );
}

// ---------------------------------------------------------------------------
// The claim rule
// ---------------------------------------------------------------------------

/// **Both controls clear every boundary's grab**, measured here and never
/// inherited from the arrangement pill three items to their left.
///
/// The row is 48. The tone map's capsule is a `.pill` at 16.5, centred, so
/// there is (48 - 16.5) / 2 = **15.75** of row above it and 15.75 below. The
/// exposure track is 5 tall, which is not a target a hand finds, so what a
/// press is tested against is that track grown to a line's height — 16.5, and
/// therefore **15.75** as well. Both against a `GRAB` of **6**.
///
/// **That the two numbers agree is a fact about two boxes being one height**,
/// not a number either of them inherited: this asserts each off its own
/// rectangle, so a change to either box fails here rather than being covered
/// by the other.
///
/// **So it fails if either control moves, if the row gets shorter, or if
/// `GRAB` widens past 15.75** — and the last is the point: the fix then is to
/// change the rule in `input`, deliberately.
#[test]
fn both_controls_clear_every_boundarys_grab() {
    for viewport in [SMALLEST, PLAUSIBLE] {
        let (mut panel, ctx) = console(viewport);
        let row = group(&panel, &ctx, mock_look());
        let strip = rect_of(panel.layout(), "transport");
        let view = view(mock_look());

        for (target, what) in [
            (row.tone, "the tone map capsule"),
            (row.grip, "the exposure track"),
        ] {
            let above = target.min.y - strip.y;
            let below = strip.y + strip.h - target.max.y;
            assert!(
                near(above, 15.75) && near(below, 15.75),
                "{what} has {above} of row above it and {below} below, and the row's 48 around \
                 a box of 16.5 is 15.75 either side"
            );
            assert!(
                above > GRAB,
                "{what} has {above} of row above it and a boundary grabs {GRAB} — the control \
                 is inside a boundary's grab, and `input`'s rule is what has to change"
            );
            for probe in [
                target.left_top(),
                target.right_top(),
                target.left_bottom(),
                target.right_bottom(),
                target.center(),
                target.center_top(),
                target.center_bottom(),
            ] {
                assert!(
                    !matches!(
                        panel.layout().hit(at(probe), GRAB),
                        karakuri_layout::Hit::Divider { .. }
                    ),
                    "a boundary grabs {probe:?}, which is on {what}"
                );
                assert_eq!(
                    claim(&mut panel, &ctx, &view, at(probe)),
                    Claim::Panel,
                    "the panel does not get a press at {probe:?}, which is on {what}"
                );
            }
        }

        // The band under them is still the boundary's, which is what says the
        // clearance is a clearance and not the grab having gone missing.
        assert!(
            matches!(
                panel
                    .layout()
                    .hit(Point::new(row.track.center().x, strip.y + strip.h), GRAB),
                karakuri_layout::Hit::Divider { .. }
            ),
            "the bottom edge of the transport row is not in the grab of the boundary under it, \
             so this test is no longer measuring the clearance it was written for"
        );
    }
}

/// **A control claims what it acts on and no more.** The gap between the
/// capsule and the track, the `exp` label and the figure are not controls, and
/// a press on any of them is `egui`'s.
#[test]
fn nothing_beside_the_two_controls_is_claimed() {
    let (mut panel, ctx) = console(PLAUSIBLE);
    let row = group(&panel, &ctx, mock_look());
    let view = view(mock_look());

    assert_eq!(
        claim(&mut panel, &ctx, &view, at(row.tone.center())),
        Claim::Panel
    );
    assert_eq!(
        claim(&mut panel, &ctx, &view, at(row.grip.center())),
        Claim::Panel
    );
    for (probe, what) in [
        (row.label.center(), "the `exp` label"),
        (row.value.center(), "the figure"),
        (
            egui::pos2(
                row.tone.max.x + size::TRANSPORT_GAP * 0.5,
                row.tone.center().y,
            ),
            "the gap between the capsule and the label",
        ),
        (
            egui::pos2(row.track.min.x - 1.0, row.track.center().y),
            "one pixel to the left of the track",
        ),
        (
            egui::pos2(row.track.max.x + 1.0, row.track.center().y),
            "one pixel to the right of the track",
        ),
    ] {
        assert_eq!(
            claim(&mut panel, &ctx, &view, at(probe)),
            Claim::Egui,
            "{what} is being claimed as a control the panel acts on"
        );
    }
}

/// **Before anything has been drawn there is no control**, and neither is
/// there without an engine or without a look: a capsule is as wide as the
/// names it can hold, and the group's place is measured from a row that a
/// console with no tempo does not draw.
#[test]
fn a_control_that_has_not_been_drawn_is_not_there() {
    let mut panel = Panel::new(PLAUSIBLE.w, PLAUSIBLE.h);
    panel.solve();

    let fresh = egui::Context::default();
    assert_eq!(fresh.cumulative_pass_nr(), 0);
    assert_eq!(
        look(
            &fresh,
            panel.layout(),
            Some(mock()),
            None,
            None,
            &Arrangement::NONE,
            Some(mock_look())
        ),
        None
    );

    let ctx = drawn_once();
    assert_eq!(
        look(
            &ctx,
            panel.layout(),
            None,
            None,
            None,
            &Arrangement::NONE,
            Some(mock_look())
        ),
        None,
        "the look controls were drawn in a transport row that is not there"
    );
    assert_eq!(
        look(
            &ctx,
            panel.layout(),
            Some(mock()),
            None,
            None,
            &Arrangement::NONE,
            None
        ),
        None,
        "a console with no engine behind it drew a look, which is a reading nothing took"
    );
}

// ---------------------------------------------------------------------------
// What a press asks for
// ---------------------------------------------------------------------------

/// **The capsule cycles all four and comes back, once each** — which is
/// `tests/blend.rs`'s measurement over four values instead of three, and is
/// what keeps `next_tonemap`'s order and the list it is a copy of from
/// drifting.
///
/// The step is asserted **as an operation**, so what is checked is the thing a
/// map or a model would be offered: `SetTonemap` naming the destination, never
/// a step.
#[test]
fn the_capsule_cycles_every_operator_once_and_wraps() {
    let (panel, ctx) = console(PLAUSIBLE);
    let mut seen = Vec::new();
    let mut at_now = Tonemap::Clamp;
    for _ in 0..EVERY.len() {
        let row = group(
            &panel,
            &ctx,
            Look {
                tonemap: at_now,
                ..mock_look()
            },
        );
        let asked = row
            .tonemap(at(row.tone.center()))
            .expect("a press on the capsule");
        let Operation::SetTonemap { tonemap } = asked else {
            panic!("a press on the tone map capsule asked for {asked:?}")
        };
        assert_ne!(
            tonemap, at_now,
            "the capsule asked for the operator that is already running, so a press does nothing"
        );
        seen.push(tonemap);
        at_now = tonemap;
    }
    assert_eq!(
        at_now,
        Tonemap::Clamp,
        "four presses from `clamp` did not come back to it, so the cycle is not four long"
    );
    for tonemap in EVERY {
        assert_eq!(
            seen.iter().filter(|seen| **seen == tonemap).count(),
            1,
            "`{}` is reached {} times in one turn of the cycle, so the capsule cannot reach \
             every operator in four presses",
            tonemap.name(),
            seen.iter().filter(|seen| **seen == tonemap).count()
        );
    }
}

/// **A press on the track names where it landed**, exactly at both ends and
/// exactly 1.00 in the middle — which is what makes the two halves of the
/// range readable and is the whole of *click to set it outright*.
#[test]
fn a_press_on_the_track_asks_for_the_value_under_it() {
    let (panel, ctx) = console(PLAUSIBLE);
    let row = group(&panel, &ctx, mock_look());
    let asked = |x: f32| match row.exposure(at(egui::pos2(x, row.track.center().y))) {
        Some(Operation::SetExposure { exposure }) => exposure,
        other => panic!("a press at {x} on the track asked for {other:?}"),
    };
    assert_eq!(
        asked(row.track.min.x),
        EXPOSURE_MIN,
        "the left-hand end of the track is not exactly a sixty-fourth"
    );
    assert_eq!(
        asked(row.track.max.x),
        EXPOSURE_MAX,
        "the right-hand end of the track is not exactly sixty-four"
    );
    assert_eq!(
        asked(row.track.center().x),
        1.0,
        "the middle of the track is not exactly unity, so there is no place a hand can put \
         the exposure back where it started"
    );

    // Monotone, and every value inside the span.
    let mut last = 0.0;
    for step in 0..=EXPOSURE_TRACK_W as u32 {
        let value = asked(row.track.min.x + step as f32);
        assert!(
            value > last,
            "the track is not monotone at {step} of the way along it"
        );
        assert!((EXPOSURE_MIN..=EXPOSURE_MAX).contains(&value));
        last = value;
    }
}

/// **One pixel of track is one press of an exposure key**, which is the whole
/// of why the track is 48 wide: a hand pointing at it can ask for exactly the
/// values a keyboard stepping a quarter stop at a time can ask for.
///
/// `karakuri-cli`'s `EXPOSURE_STEP` is `1.189_207`, which is a quarter stop,
/// and it is restated here rather than shared because it is private to that
/// binary — the assertion is what stops the two drifting from this side.
#[test]
fn one_pixel_of_track_is_one_press_of_an_exposure_key() {
    const STEP: f32 = 1.189_207;
    let (panel, ctx) = console(PLAUSIBLE);
    let row = group(&panel, &ctx, mock_look());
    assert!(
        near(row.track.width(), 48.0),
        "the track is {} wide and twelve stops at four presses a stop is 48",
        row.track.width()
    );
    let asked = |x: f32| match row.exposure(at(egui::pos2(x, row.track.center().y))) {
        Some(Operation::SetExposure { exposure }) => exposure,
        other => panic!("a press at {x} on the track asked for {other:?}"),
    };
    for step in 0..EXPOSURE_TRACK_W as u32 {
        let here = asked(row.track.min.x + step as f32);
        let next = asked(row.track.min.x + step as f32 + 1.0);
        let ratio = next / here;
        assert!(
            (ratio - STEP).abs() < 1e-4,
            "a pixel at {step} along the track is a factor of {ratio} and a key press is \
             {STEP} — the two surfaces cannot reach the same values"
        );
    }
}

/// **The two controls do not overlap**, so a press is one question or the
/// other and never both.
#[test]
fn a_press_is_one_control_or_the_other() {
    let (panel, ctx) = console(PLAUSIBLE);
    let row = group(&panel, &ctx, mock_look());
    let on_tone = at(row.tone.center());
    let on_track = at(row.grip.center());
    assert!(row.tonemap(on_tone).is_some() && row.exposure(on_tone).is_none());
    assert!(row.exposure(on_track).is_some() && row.tonemap(on_track).is_none());
    assert!(row.owns(on_tone) && row.owns(on_track));

    let off = at(egui::pos2(row.value.center().x, row.value.center().y));
    assert!(
        !row.owns(off) && row.tonemap(off).is_none() && row.exposure(off).is_none(),
        "the figure beside the track is being read as a control"
    );
}

// ---------------------------------------------------------------------------
// The value and the track are one derivation
// ---------------------------------------------------------------------------

/// **A point on the track and a value on it are inverses**, so what a press
/// asks for and where the fill is drawn cannot come apart.
#[test]
fn the_track_and_the_value_are_inverses() {
    for step in 0..=48 {
        let unit = step as f32 / 48.0;
        let back = unit_of(exposure_at(unit));
        assert!(
            (back - unit).abs() < 1e-5,
            "{unit} of the way along the track is {} and reads back at {back}",
            exposure_at(unit)
        );
    }
    assert_eq!(exposure_at(0.0), EXPOSURE_MIN);
    assert_eq!(exposure_at(1.0), EXPOSURE_MAX);
    assert_eq!(exposure_at(0.5), 1.0);
}

/// **A value past either end is drawn at that end**, which is what a
/// `--exposure 200` reaches the panel as: the flag is deliberately not held to
/// the interactive bounds, and a fill that ran off the track would be a
/// picture of a value nobody can point at. The figure beside it is what says
/// the number then.
#[test]
fn a_level_past_the_ends_is_drawn_at_the_end_and_said_in_the_figure() {
    assert_eq!(unit_of(EXPOSURE_MAX * 4.0), 1.0);
    assert_eq!(unit_of(EXPOSURE_MIN / 4.0), 0.0);
    assert_eq!(unit_of(0.0), 0.0, "zero is a floor and not a NaN");
    assert_eq!(unit_of(-1.0), 0.0, "a negative level is a floor");
    assert_eq!(unit_of(f32::NAN), 0.0, "a NaN level is a floor");

    let (panel, ctx) = console(PLAUSIBLE);
    let over = group(&panel, &ctx, at_exposure(EXPOSURE_MAX * 4.0));
    assert!(near(over.fill.width(), over.track.width()));
    assert!(
        over.value.width() > 0.0,
        "a level past the end of the track has no figure beside it, so nothing says what it is"
    );
}

// ---------------------------------------------------------------------------
// The route a window loop takes
// ---------------------------------------------------------------------------

/// **The whole route, as the window loop drives it**: `claim` first, then the
/// derivation that drew the control asked a second time, then the operation.
///
/// This is `tests/vocabulary.rs`'s third pass over these two controls, and it
/// is here rather than there because these rows are not in *Arranging the
/// console* — they are `karakuri_operation::Operation`s, so `panel_column.rs`
/// is what reads their badges and this is what demonstrates that a press
/// reaches them.
///
/// **It does not claim reachability.** Whether a claimed press becomes one of
/// these operations is `crates/karakuri/src/main.rs`'s, which this crate
/// cannot depend on (ADR-0156); the sufficient half is a test in that binary.
#[test]
fn a_press_reaches_both_operations_the_way_the_window_loop_reaches_them() {
    let (mut panel, ctx) = console(PLAUSIBLE);
    let view = view(mock_look());

    for (probe, expected) in [
        (
            group(&panel, &ctx, mock_look()).tone.center(),
            Operation::SetTonemap {
                tonemap: Tonemap::AgX,
            },
        ),
        (
            group(&panel, &ctx, mock_look()).grip.center(),
            Operation::SetExposure { exposure: 1.0 },
        ),
    ] {
        assert_eq!(
            claim(&mut panel, &ctx, &view, at(probe)),
            Claim::Panel,
            "`claim` gives a press at {probe:?} to `egui`, so no route into the operation \
             exists however the control is drawn"
        );
        let row = look(
            &ctx,
            panel.layout(),
            view.transport,
            None,
            None,
            &view.arrangement,
            view.look,
        )
        .expect("the same group `claim` hit-tested");
        let asked = row
            .tonemap(at(probe))
            .or_else(|| row.exposure(at(probe)))
            .expect("a press the panel claimed on a control it draws asks for something");
        assert_eq!(
            asked, expected,
            "the press was claimed and asked for something else, so the control that claims a \
             press and the control that acts on it have come apart"
        );
    }
}
