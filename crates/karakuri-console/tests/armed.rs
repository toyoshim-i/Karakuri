//! Scheduled move presentation on a fader (P-0087, ADR-0206).
//!
//! An armed transition draws a hairline destination mark and an animating fill
//! reaching toward it without altering current deck values or moving off-phase.

mod common;

use std::time::Duration;

use common::{drawn_once, near, PLAUSIBLE};
use karakuri_console::panel::{Op, Panel};
use karakuri_console::repaint::{Change, Repaint};
use karakuri_console::room::{size, Room};
use karakuri_console::view::{
    mixer, roll_at, Level, Mask, Phase, Reach, Strip, StripBox, Tally, View, ROLL_PERIOD,
    ROLL_REACH, ROLL_STALENESS,
};
use karakuri_layout::NodeId;
use karakuri_operation::BlendMode;

/// A strip with nothing scheduled on either fader — the ordinary case, and what
/// every other test file in this crate is made of. Settled in the tally's sense
/// too, so that nothing in it rolls and the only thing that can move is what
/// this file is about.
fn settled() -> Strip {
    Strip {
        name: "drift_night".to_owned(),
        tally: Tally::Live,
        requested: Tally::Live,
        gain: 0.44,
        gain_to: None,
        opacity: 0.30,
        opacity_to: None,
        blend: BlendMode::Over,
        mask: Mask::Linear,
        mask_angle: 0.0,
        level: Some(Level {
            mean: 0.12,
            peak: 0.12,
        }),
        is_muted: false,
        is_soloed: false,
    }
}

/// A fade armed on the fader: at 0.30 and asked for `to`, which is half the
/// track away and back again depending on which way the caller points it.
fn armed(to: f32) -> Strip {
    Strip {
        opacity_to: Some(to),
        ..settled()
    }
}

/// The console's arrangement at a plausible window, solved.
fn arrangement() -> Panel {
    let mut panel = Panel::new(PLAUSIBLE.w, PLAUSIBLE.h);
    panel.solve();
    panel
}

fn node(panel: &Panel, name: &str) -> NodeId {
    panel
        .layout()
        .find(name)
        .unwrap_or_else(|| panic!("the arrangement names `{name}`"))
}

/// One strip's box, off a panel at a plausible window.
fn box_of(strips: &[Strip]) -> StripBox {
    let panel = arrangement();
    let ctx = drawn_once();
    mixer(&ctx, panel.layout(), strips)
        .expect("the mixer bay draws its strips")
        .strip(0)
}

/// Painted rectangles and text galley count inside one strip.
fn painted(strips: Vec<Strip>, phase: Phase) -> (StripBox, Vec<egui::Rect>, usize) {
    let at = box_of(&strips);
    let mut panel = arrangement();
    let mut view = View::new(Room::Day);
    view.mixer = strips;
    view.phase = phase;
    let ctx = drawn_once();
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| view.draw(ui, &mut panel));
    out.textures_delta.clear();
    // The strip and half the gap around it, which is the clip `strip_into`
    // paints under: a knob is meant to stand proud of its track, and so is the
    // mark that stands for one.
    let inside = at.rect.expand(size::STRIP_GAP * 0.5);
    let mut rects = Vec::new();
    let mut words = 0;
    for clipped in out.shapes {
        match clipped.shape {
            egui::Shape::Rect(rect) if inside.contains_rect(rect.rect) => rects.push(rect.rect),
            egui::Shape::Text(text) if inside.contains(text.pos) => words += 1,
            _ => {}
        }
    }
    (at, rects, words)
}

/// Whether a frame painted this rectangle, to the tolerance every other
/// geometric claim in this crate is made at.
fn drew(rects: &[egui::Rect], want: egui::Rect) -> bool {
    rects.iter().any(|rect| {
        near(rect.min.x, want.min.x)
            && near(rect.min.y, want.min.y)
            && near(rect.max.x, want.max.x)
            && near(rect.max.y, want.max.y)
    })
}

/// The one-pixel marks in a frame — the shape this presentation adds, told
/// apart by being [`size::HAIRLINE`] across and nothing else.
fn marks(rects: &[egui::Rect]) -> Vec<egui::Rect> {
    rects
        .iter()
        .copied()
        .filter(|rect| near(rect.height(), size::HAIRLINE) || near(rect.width(), size::HAIRLINE))
        .collect()
}

/// Derives the value under a mark from its vertical position within fader travel.
fn value_under(at: &StripBox, mark: egui::Rect) -> f32 {
    let travel = at.fader.height() - size::VFADER_INSET * 2.0;
    (at.fader.max.y - size::VFADER_INSET - mark.center().y) / travel
}

/// How long the band is along the axis it runs on.
fn length(reach: Reach, vertical: bool) -> f32 {
    match vertical {
        true => reach.band.height(),
        false => reach.band.width(),
    }
}

// ---------------------------------------------------------------------------
// 1. The destination is drawn, and only where there is one
// ---------------------------------------------------------------------------

/// An armed fader marks its destination at [`StripBox::fader_reach`], while a settled fader draws no mark.
#[test]
fn an_armed_fader_marks_its_destination_and_a_settled_one_marks_nothing() {
    for to in [0.8, 0.0] {
        let (at, rects, _) = painted(vec![armed(to)], Phase::ZERO);
        let want = at.fader_reach(0.30, to, roll_at(Phase::ZERO)).mark;
        assert!(
            drew(&rects, want),
            "a fader armed for {to} painted no mark at {want:?}"
        );
        assert!(
            near(want.center().y, at.fader_at(to).knob.center().y),
            "the mark is at {} and the knob at {to} would be at {}",
            want.center().y,
            at.fader_at(to).knob.center().y
        );
        // And it is the knob's own width, so it always has the two pixels of
        // strip either side that the knob stands proud on.
        assert!(
            near(want.width(), size::VFADER_KNOB_W),
            "the mark is {} wide and the knob is {}",
            want.width(),
            size::VFADER_KNOB_W
        );
    }

    // Nothing scheduled: no mark anywhere in the strip, at any phase.
    for millis in [0, 100, 200, 700] {
        let (_, rects, _) = painted(vec![settled()], Phase::since(Duration::from_millis(millis)));
        let found = marks(&rects);
        assert!(
            found.is_empty(),
            "a strip with nothing scheduled painted {} hairline mark(s) at {millis} ms: {found:?}",
            found.len()
        );
    }
}

// ---------------------------------------------------------------------------
// 2. The truth is not moved
// ---------------------------------------------------------------------------

/// Verifies the drawn value matches the deck's own across phases without being shifted by the mark (P-0087).
#[test]
fn the_value_drawn_is_the_decks_own_at_every_phase() {
    let (_, settled_rects, _) = painted(vec![settled()], Phase::ZERO);
    for millis in [0, 100, 200, 399, 700] {
        let phase = Phase::since(Duration::from_millis(millis));
        for to in [0.8, 0.0] {
            let (_, rects, _) = painted(vec![armed(to)], phase);
            for want in &settled_rects {
                assert!(
                    drew(&rects, *want),
                    "a fader armed for {to} at {millis} ms stopped painting {want:?}, \
                     which is the value the deck is at"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 3. The destination is on the surface, and it is not a word
// ---------------------------------------------------------------------------

/// Verifies that destination position is rendered directly on the track rather
/// than via text galleys (ADR-0188).
#[test]
fn the_destination_is_read_off_the_track_rather_than_out_of_a_word() {
    let (_, _, settled_words) = painted(vec![settled()], Phase::ZERO);
    for to in [0.8, 0.55, 0.0] {
        let (at, rects, words) = painted(vec![armed(to)], Phase::ZERO);
        let mark = marks(&rects)
            .into_iter()
            .find(|rect| near(rect.width(), size::VFADER_KNOB_W))
            .expect("the fader marks where it is going");
        assert!(
            near(value_under(&at, mark), to),
            "the mark stands on {} and the fade is going to {to}",
            value_under(&at, mark)
        );
        assert_eq!(
            words, settled_words,
            "an armed strip painted {words} runs of type and a settled one paints \
             {settled_words}: the destination is being said in words"
        );
    }
}

// ---------------------------------------------------------------------------
// 4. The reach never lands
// ---------------------------------------------------------------------------

/// The reach band animates toward the destination up to [`ROLL_REACH`] without landing, resting at zero.
#[test]
fn the_reach_never_covers_the_gap_and_rests_at_nothing() {
    let at = box_of(&[settled()]);
    for to in [0.8, 0.0] {
        let gap = (at.fader_at(to).knob.center().y - at.fader_at(0.30).knob.center().y).abs();
        for millis in (0..1000).step_by(25) {
            let phase = Phase::since(Duration::from_millis(millis));
            let reach = at.fader_reach(0.30, to, roll_at(phase));
            let covered = length(reach, true);
            assert!(
                covered <= ROLL_REACH * gap + common::EPS,
                "the band covered {covered} of a {gap} gap at {millis} ms, and the reach is \
                 {ROLL_REACH}"
            );
            assert!(
                !reach.band.contains(reach.mark.center()),
                "the band reached its own mark at {millis} ms, which is what arriving looks like"
            );
            // Which side of the value it lies on is the direction of the move.
            match to > 0.30 {
                true => {
                    assert!(reach.band.min.y <= at.fader_at(0.30).knob.center().y + common::EPS)
                }
                false => {
                    assert!(reach.band.max.y >= at.fader_at(0.30).knob.center().y - common::EPS)
                }
            }
        }
        // At rest there is no band at all, and the frame paints none.
        let rest = Phase::since(Duration::from_millis(700));
        assert!(
            near(length(at.fader_reach(0.30, to, roll_at(rest)), true), 0.0),
            "the band has a length while the reach is at rest"
        );
        let (_, rects, _) = painted(vec![armed(to)], rest);
        let bands = rects
            .iter()
            .filter(|rect| at.fader.expand(size::HAIRLINE).contains_rect(**rect))
            .filter(|rect| !near(rect.height(), size::HAIRLINE))
            .count();
        assert!(
            bands <= 2,
            "a resting fader painted {bands} shapes inside its track, where the well and the \
             fill are two"
        );
    }
}

// ---------------------------------------------------------------------------
// 5. One phase, panel-wide
// ---------------------------------------------------------------------------

/// Both faders and the chip reach off the same phase as a fraction of their travel (ADR-0190).
#[test]
fn both_faders_reach_off_the_one_phase() {
    let at = box_of(&[settled()]);
    for millis in [0, 100, 200, 300, 399, 700] {
        let phase = Phase::since(Duration::from_millis(millis));
        let rolled = roll_at(phase);

        let trim_gap = (at.trim_at(0.9).knob.center().x - at.trim_at(0.44).knob.center().x).abs();
        let fader_gap =
            (at.fader_at(0.8).knob.center().y - at.fader_at(0.30).knob.center().y).abs();
        let trim = length(at.trim_reach(0.44, 0.9, rolled), false) / trim_gap;
        let fader = length(at.fader_reach(0.30, 0.8, rolled), true) / fader_gap;

        assert!(
            near(trim, rolled) && near(fader, rolled),
            "at {millis} ms the trim has covered {trim} of its gap and the fader {fader}, \
             and the phase says {rolled}"
        );
    }
    // **And the frame paints both of them off the one number**, which is the
    // half the arithmetic above cannot see: a strip with a fade on each fader
    // draws two bands, and they are the two this file computed at the same
    // displacement.
    let phase = Phase::since(Duration::from_millis(200));
    let both = Strip {
        gain_to: Some(0.9),
        opacity_to: Some(0.8),
        ..settled()
    };
    let (at, rects, _) = painted(vec![both], phase);
    for want in [
        at.trim_reach(0.44, 0.9, roll_at(phase)).band,
        at.fader_reach(0.30, 0.8, roll_at(phase)).band,
    ] {
        assert!(
            drew(&rects, want),
            "a strip with a fade on each fader painted no band at {want:?}, \
             so the two are not reading one phase"
        );
    }

    // And it is a phase: one period later is the same picture.
    let quarter = Phase::since(Duration::from_millis(100));
    assert!(
        near(
            roll_at(Phase::since(ROLL_PERIOD + Duration::from_millis(100))),
            roll_at(quarter)
        ),
        "one period later is a different displacement, so this is not a phase"
    );
}

// ---------------------------------------------------------------------------
// 6. What the panel asks for
// ---------------------------------------------------------------------------

/// An armed fade requests frame deadlines only while armed and drawable, and nothing when folded or unmoving (ADR-0193).
#[test]
fn the_panel_asks_for_a_deadline_only_while_a_move_is_armed_and_drawable() {
    let mut panel = arrangement();
    let mut view = View::new(Room::Day);

    view.mixer = vec![settled(), settled()];
    assert_eq!(
        view.animating(panel.layout()),
        None,
        "a deck with nothing scheduled declared a staleness"
    );
    assert_eq!(
        Change::Animating(view.animating(panel.layout())).repaint(),
        Repaint::Never,
        "a still panel asked for a frame"
    );

    // Scheduled to where it already is: nothing is pending that anyone could
    // see, and the strip says so.
    view.mixer[1].opacity_to = Some(view.mixer[1].opacity);
    assert_eq!(
        view.mixer[1].opacity_pending(),
        None,
        "a fade to the value the control is already at is pending"
    );
    assert_eq!(
        view.animating(panel.layout()),
        None,
        "a fade to where the fader already is declared a staleness"
    );

    // A move the trim cannot draw: both ends clamp to the top of the track.
    view.mixer[1].opacity_to = None;
    view.mixer[1].gain = 1.5;
    view.mixer[1].gain_to = Some(2.0);
    assert_eq!(
        view.mixer[1].gain_pending(),
        None,
        "a move between two values the trim draws in one place is pending"
    );
    assert_eq!(
        view.animating(panel.layout()),
        None,
        "a move the track cannot draw declared a staleness"
    );

    // A real one, on either fader, and the panel is live for as long as it is.
    view.mixer[1].gain = 0.2;
    assert_eq!(
        view.animating(panel.layout()),
        Some(ROLL_STALENESS),
        "a fade armed on the trim did not declare the reach's staleness"
    );
    assert_eq!(
        Change::Animating(view.animating(panel.layout())).repaint(),
        Repaint::After(ROLL_STALENESS),
        "an armed fade did not ask for a deadline"
    );
    view.mixer[1].gain_to = None;
    view.mixer[0].opacity_to = Some(0.9);
    assert_eq!(
        view.animating(panel.layout()),
        Some(ROLL_STALENESS),
        "a fade armed on the fader did not declare the reach's staleness"
    );

    // Two armed moves and a parked chip are one phase and one deadline, not
    // three.
    view.mixer[1].gain_to = Some(0.9);
    view.mixer[1].requested = Tally::Priming;
    view.mixer[1].tally = Tally::Allocated;
    assert_eq!(view.animating(panel.layout()), Some(ROLL_STALENESS));

    // Folded away, the bay declares nothing — and unfolding declares again,
    // because the answer is re-derived rather than latched.
    let bay = node(&panel, "mixer");
    panel.op(Op::Fold(bay));
    assert_eq!(
        view.animating(panel.layout()),
        None,
        "an armed fade declared a staleness with its bay folded away"
    );
    panel.op(Op::Unfold(bay));
    assert_eq!(
        view.animating(panel.layout()),
        Some(ROLL_STALENESS),
        "unfolding the bay left the reach declared dead while a fade is still armed"
    );

    // And the fade landing puts the panel back to sleep.
    view.mixer[0].opacity_to = None;
    view.mixer[1].gain_to = None;
    view.mixer[1].requested = view.mixer[1].tally;
    assert_eq!(
        view.animating(panel.layout()),
        None,
        "everything landed and the panel is still asking for frames"
    );
}
