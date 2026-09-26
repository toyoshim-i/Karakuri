//! Mixer fader pointer manipulation: grab mechanics, travel bounds, operation emission, and repaint requests.

mod common;

use common::{drawn_once, near, point, rect_of, showing, PLAUSIBLE};
use karakuri_console::input::{claim, Claim};
use karakuri_console::panel::{Dragged, Grab, InHand, Knob, Panel, Released, GRAB};
use karakuri_console::repaint::{Change, Repaint};
use karakuri_console::room::{size, Room};
use karakuri_console::view::{mixer, Level, Mask, Mixer, Strip, StripBox, Tally, View};
use karakuri_layout::Point;
use karakuri_operation::{BlendMode, Operation};

/// Four strips, so that *which deck a drag named* is a question with four wrong
/// answers rather than one. The values are apart from each other and none of
/// them is at an end, so a fader that moved the wrong strip's control or read
/// the wrong strip's value says so.
fn strips() -> Vec<Strip> {
    ["drift_night", "lattice_veil", "glass_shell", "slow_tide"]
        .into_iter()
        .enumerate()
        .map(|(slot, name)| Strip {
            name: name.to_owned(),
            tally: Tally::Live,
            // Settled: the request and the effective residency agree, so
            // nothing in these strips is pending and nothing rolls. What a
            // strip whose two halves disagree draws is `parked.rs`.
            requested: Tally::Live,
            gain: 0.2 + 0.15 * slot as f32,
            gain_to: None,
            opacity: 0.8 - 0.15 * slot as f32,
            opacity_to: None,
            blend: BlendMode::Over,
            mask: Mask::None,
            mask_angle: 0.0,
            level: Some(Level {
                mean: 0.5,
                peak: 0.6,
            }),
            is_muted: false,
            is_soloed: false,
        })
        .collect()
}

use common::default_console as console;

fn bay<'a>(panel: &Panel, ctx: &egui::Context, strips: &'a [Strip]) -> Mixer<'a> {
    mixer(ctx, panel.layout(), strips).expect("the mixer bay draws its strips")
}

/// Where a strip's two knobs are, at the values that strip is carrying.
fn knobs(at: StripBox, strip: &Strip) -> (egui::Rect, egui::Rect) {
    (at.trim_at(strip.gain).knob, at.fader_at(strip.opacity).knob)
}

/// Computes fader travel from style constants, accounting for vertical inset (`.vfader b`).
fn travel(at: StripBox, knob: Knob) -> f32 {
    match knob {
        Knob::Trim { .. } => at.trim.width(),
        Knob::Fader { .. } => at.fader.height() - size::VFADER_INSET * 2.0,
        // The Master bay's, which are no strip's and have a file of their own:
        // `tests/master.rs` measures those tracks off the bay they are in.
        Knob::Out | Knob::Chain { .. } => {
            panic!("a Master bay knob is not one of a strip's two faders")
        }
        // The Inspector's, which is no strip's either: `tests/param_fader.rs`
        // measures that track off the pane it is in.
        Knob::Param { .. } => panic!("a parameter fader is not one of a strip's two faders"),
    }
}

/// Take a knob in hand, at `p`, and return what the panel now holds. Panics
/// where `p` is not on a knob, which is the failure worth reading.
fn take(panel: &mut Panel, bay: &Mixer, p: egui::Pos2) -> Grab {
    let grab = bay
        .grab(point(p))
        .unwrap_or_else(|| panic!("nothing to take hold of at {p:?}"));
    panel.grab(point(p), grab.clone());
    grab
}

/// Returns painted shape bounding rects within a strip for verification.
fn drawn(panel: &mut Panel, strips: &[Strip], rect: egui::Rect) -> Vec<egui::Rect> {
    let ctx = drawn_once();
    let mut view = View::new(Room::Day);
    view.mixer = strips.to_vec();
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| view.draw(ui, panel));
    out.textures_delta.clear();
    out.shapes
        .into_iter()
        .map(|clipped| clipped.shape.visual_bounding_rect())
        .filter(|bounds| bounds.is_finite() && rect.contains_rect(*bounds))
        .collect()
}

/// The value an emitted operation carries, whichever of the two it is.
fn asked(dragged: Option<Dragged>) -> Option<f32> {
    match dragged? {
        Dragged::Fader(Operation::SetGain { gain, .. }) => Some(gain),
        Dragged::Fader(Operation::SetOpacity { opacity, .. }) => Some(opacity),
        other => panic!("a fader drag reported {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// The knob, and not the track
// ---------------------------------------------------------------------------

/// Pressing a fader knob grabs it, while pressing the track does nothing.
#[test]
fn the_knob_is_grabbed_and_the_track_is_not() {
    let (panel, ctx) = console();
    let strips = strips();
    let bay = bay(&panel, &ctx, &strips);

    for (slot, strip) in strips.iter().enumerate() {
        let at = bay.strip(slot);
        let (trim, fader) = knobs(at, strip);
        let deck = slot as u8;

        let took = bay.grab(point(trim.center())).expect("the trim's knob");
        assert_eq!(took.knob(), Knob::Trim { deck });
        let took = bay.grab(point(fader.center())).expect("the fader's knob");
        assert_eq!(took.knob(), Knob::Fader { deck });

        // Both ends of both tracks, and the strip's own furniture. Every one
        // of these is inside the bay and none of them is a knob.
        let off = [
            egui::pos2(at.trim.min.x + 1.0, at.trim.center().y),
            egui::pos2(at.trim.max.x - 1.0, at.trim.center().y),
            egui::pos2(at.fader.center().x, at.fader.min.y + 1.0),
            egui::pos2(at.fader.center().x, at.fader.max.y - 1.0),
            at.meter.center(),
            at.tally.center(),
            at.num.center(),
        ];
        for p in off {
            // The guard on the assertion below: a point that turned out to be
            // *on* a knob would make it pass for the wrong reason.
            if trim.contains(p) || fader.contains(p) {
                continue;
            }
            assert!(
                bay.grab(point(p)).is_none(),
                "a press at {p:?} in strip {slot} took hold of something, and it is not a knob"
            );
        }
    }
}

/// Sweeping across strip coordinates confirms only knobs respond to grab hits.
#[test]
fn only_the_two_knobs_in_a_strip_answer_a_grab() {
    let (panel, ctx) = console();
    let strips = strips();
    let bay = bay(&panel, &ctx, &strips);
    let strip = &strips[1];
    let at = bay.strip(1);
    let (trim, fader) = knobs(at, strip);

    let mut on_a_knob = 0;
    let mut grabbed = 0;
    let steps = 60;
    for i in 0..=steps {
        for j in 0..=steps {
            let p = egui::pos2(
                at.rect.min.x + at.rect.width() * i as f32 / steps as f32,
                at.rect.min.y + at.rect.height() * j as f32 / steps as f32,
            );
            let knob = trim.contains(p) || fader.contains(p);
            on_a_knob += usize::from(knob);
            let took = bay.grab(point(p));
            grabbed += usize::from(took.is_some());
            assert_eq!(
                took.is_some(),
                knob,
                "a point at {p:?} is {} a knob and the grab said {}",
                match knob {
                    true => "on",
                    false => "off",
                },
                took.is_some()
            );
        }
    }
    // The sweep found both knobs, or it asserted nothing at all.
    assert!(
        on_a_knob > 0 && grabbed == on_a_knob,
        "the sweep never landed on a knob, so it is not measuring what it was written for"
    );
}

// ---------------------------------------------------------------------------
// The value does not jump
// ---------------------------------------------------------------------------

/// Dragging preserves grab offset relative to knob center to avoid value jumping.
#[test]
fn the_grab_keeps_its_offset_so_the_value_does_not_jump() {
    let (mut panel, ctx) = console();
    let strips = strips();
    let bay = bay(&panel, &ctx, &strips);
    let strip = &strips[0];
    let at = bay.strip(0);
    let (_, fader) = knobs(at, strip);
    let span = travel(at, Knob::Fader { deck: 0 });

    // Three pixels below the knob's centre, and still on the knob — a knob is
    // `VFADER_KNOB_H` tall, so this is inside it and is where a hand lands.
    let held = egui::pos2(fader.center().x, fader.center().y + 3.0);
    assert!(fader.contains(held), "the press is not on the knob");
    take(&mut panel, &bay, held);

    assert_eq!(
        panel.moved(point(held)),
        None,
        "a press that moved nothing changed the value, so the grab did not keep its offset"
    );

    // Moving 10px up the track shifts value upward from initial position (fills from bottom).
    let moved = egui::pos2(held.x, held.y - 10.0);
    let value = asked(panel.moved(point(moved))).expect("a move up the track asks for a value");
    assert!(
        near(value, strip.opacity + 10.0 / span),
        "a drag ten pixels up a {span}-pixel track asked for {value}, and the fader was at {}",
        strip.opacity
    );
}

// ---------------------------------------------------------------------------
// Both ends are exact
// ---------------------------------------------------------------------------

/// Fader travel strictly spans `0.0` to `1.0` (ADR-0178).
#[test]
fn both_ends_of_both_faders_are_exactly_reachable() {
    let (mut panel, ctx) = console();
    let strips = strips();
    let bay = bay(&panel, &ctx, &strips);
    let strip = &strips[0];
    let at = bay.strip(0);
    let (trim, fader) = knobs(at, strip);

    // The trim: a row, filling from the left, so the floor is the track's left
    // edge and the ceiling its right.
    take(&mut panel, &bay, trim.center());
    let floor = egui::pos2(at.trim.min.x, trim.center().y);
    let ceiling = egui::pos2(at.trim.max.x, trim.center().y);
    assert_eq!(
        asked(panel.moved(point(floor))),
        Some(0.0),
        "the bottom of the trim is not exactly zero"
    );
    assert_eq!(
        asked(panel.moved(point(ceiling))),
        Some(1.0),
        "the top of the trim is not exactly one"
    );
    // Past the end is the end, and it is the same exact one — so a hand that
    // overshoots does not leave the trim at 0.9999.
    assert_eq!(
        asked(panel.moved(point(egui::pos2(ceiling.x + 400.0, ceiling.y)))),
        None,
        "a drag past the top of the trim asked for a second, different, top"
    );
    assert_eq!(
        asked(panel.moved(point(egui::pos2(floor.x - 400.0, floor.y)))),
        Some(0.0)
    );
    panel.released(None);

    // Vertical fader column fills from bottom and is inset by VFADER_INSET at both ends.
    take(&mut panel, &bay, fader.center());
    let floor = egui::pos2(fader.center().x, at.fader.max.y - size::VFADER_INSET);
    let ceiling = egui::pos2(fader.center().x, at.fader.min.y + size::VFADER_INSET);
    assert_eq!(
        asked(panel.moved(point(floor))),
        Some(0.0),
        "the bottom of the fader is not exactly zero"
    );
    assert_eq!(
        asked(panel.moved(point(ceiling))),
        Some(1.0),
        "the top of the fader is not exactly one"
    );
    assert_eq!(
        asked(panel.moved(point(egui::pos2(ceiling.x, ceiling.y - 400.0)))),
        None,
        "a drag past the top of the fader asked for a second, different, top"
    );
}

// ---------------------------------------------------------------------------
// What the drag emits
// ---------------------------------------------------------------------------

/// Fader drags emit the appropriate operation and deck index for trim and volume controls.
#[test]
fn a_drag_emits_the_right_operation_for_the_right_deck() {
    let (mut panel, ctx) = console();
    let strips = strips();
    let bay = bay(&panel, &ctx, &strips);

    for (slot, strip) in strips.iter().enumerate() {
        let at = bay.strip(slot);
        let (trim, fader) = knobs(at, strip);
        let deck = slot as u8;

        // The trim, dragged to the top of its track: exactly unity.
        take(&mut panel, &bay, trim.center());
        let to = egui::pos2(at.trim.max.x, trim.center().y);
        assert_eq!(
            panel.moved(point(to)),
            Some(Dragged::Fader(Operation::SetGain { deck, gain: 1.0 })),
            "the trim of strip {slot} emitted the wrong operation"
        );
        assert_eq!(
            panel.released(None),
            Some(Released::Let {
                knob: Knob::Trim { deck }
            })
        );

        // The fader, dragged to the floor: exactly zero, and an opacity rather
        // than a gain.
        take(&mut panel, &bay, fader.center());
        let to = egui::pos2(fader.center().x, at.fader.max.y);
        assert_eq!(
            panel.moved(point(to)),
            Some(Dragged::Fader(Operation::SetOpacity { deck, opacity: 0.0 })),
            "the fader of strip {slot} emitted the wrong operation"
        );
        assert_eq!(
            panel.released(None),
            Some(Released::Let {
                knob: Knob::Fader { deck }
            })
        );
    }
}

// ---------------------------------------------------------------------------
// The claim, the cursor, and the boundary underneath
// ---------------------------------------------------------------------------

/// An in-hand fader retains pointer capture beyond the strip and viewport bounds.
#[test]
fn a_drag_in_hand_keeps_its_claim_while_the_pointer_leaves_the_strip() {
    let (mut panel, ctx) = console();
    let strips = strips();
    let away = {
        let bay = bay(&panel, &ctx, &strips);
        let at = bay.strip(0);
        let (_, fader) = knobs(at, &strips[0]);
        let held = fader.center();

        // Before anything is in hand, the middle of the picture is egui's.
        let away = rect_of(panel.layout(), "program-view");
        let away = Point::new(away.x + away.w * 0.5, away.y + away.h * 0.5);
        assert_eq!(
            claim(&mut panel, &ctx, &showing(&strips), away),
            Claim::Egui
        );
        // The knob is the panel's, by rule 4.
        assert_eq!(
            claim(&mut panel, &ctx, &showing(&strips), point(held)),
            Claim::Panel,
            "the knob is not claimed, so a press on it would go to egui"
        );
        take(&mut panel, &bay, held);
        away
    };

    for wandered in [
        away,
        Point::new(0.0, 0.0),
        Point::new(-500.0, -500.0),
        Point::new(PLAUSIBLE.w + 900.0, PLAUSIBLE.h + 900.0),
    ] {
        panel.moved(wandered);
        assert_eq!(
            claim(&mut panel, &ctx, &showing(&strips), wandered),
            Claim::Panel,
            "the fader lost its claim at {wandered:?}"
        );
    }

    // The release is decided before it is performed, and afterwards the
    // pointer where it is standing is egui's again.
    assert_eq!(
        claim(&mut panel, &ctx, &showing(&strips), away),
        Claim::Panel
    );
    assert!(panel.released(None).is_some());
    assert_eq!(
        claim(&mut panel, &ctx, &showing(&strips), away),
        Claim::Egui
    );
}

/// Fader drag maintains standard arrow cursor and suppresses boundary resize cursors.
#[test]
fn the_cursor_during_a_fader_drag_is_not_a_resize() {
    let strips = strips();
    let cursor = |panel: &mut Panel| {
        let ctx = drawn_once();
        let mut view = View::new(Room::Day);
        view.mixer = strips.clone();
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| view.draw(ui, panel));
        out.textures_delta.clear();
        out.platform_output.cursor_icon
    };

    let (mut panel, ctx) = console();
    let (over_a_boundary, knob) = {
        let bay = bay(&panel, &ctx, &strips);
        let (_, fader) = knobs(bay.strip(0), &strips[0]);
        // The gap between the right pane and the centre, at the mixer's own
        // height: a boundary the pointer can be over while a fader is held.
        let pane = rect_of(panel.layout(), "right-pane");
        let mixer = rect_of(panel.layout(), "mixer");
        (
            Point::new(pane.x - 1.0, mixer.y + mixer.h * 0.5),
            fader.center(),
        )
    };

    // The control: with nothing in hand, that point is a boundary and the
    // cursor says so.
    panel.set_cursor(over_a_boundary);
    assert_eq!(
        cursor(&mut panel),
        egui::CursorIcon::ResizeHorizontal,
        "the pointer is not over a vertical boundary, so this test cannot show that a \
         fader drag suppresses one"
    );

    // And with a fader in hand, standing on exactly the same point.
    let bay = bay(&panel, &ctx, &strips);
    take(&mut panel, &bay, knob);
    panel.moved(over_a_boundary);
    assert_eq!(
        cursor(&mut panel),
        egui::CursorIcon::Default,
        "a fader drag over a boundary drew a resize cursor for a gesture that resizes nothing"
    );
    assert_eq!(panel.in_hand(), Some(InHand::Fader));

    // Let go, and the boundary has its cursor back.
    panel.released(None);
    assert_eq!(cursor(&mut panel), egui::CursorIcon::ResizeHorizontal);
}

/// Verifies that no fader knob is placed within a boundary grab zone.
#[test]
fn no_knob_is_inside_a_boundarys_grab() {
    let (mut panel, ctx) = console();
    let strips = strips();
    let bay = bay(&panel, &ctx, &strips);

    for (slot, strip) in strips.iter().enumerate() {
        let at = bay.strip(slot);
        let (trim, fader) = knobs(at, strip);
        for (knob, what) in [(trim, "the trim's knob"), (fader, "the fader's knob")] {
            // The corners as well as the centre: a grab is a band either side
            // of a boundary, so it is an edge that reaches one first.
            for probe in [
                knob.left_top(),
                knob.right_top(),
                knob.left_bottom(),
                knob.right_bottom(),
                knob.center(),
            ] {
                assert!(
                    !matches!(
                        panel.layout().hit(point(probe), GRAB),
                        karakuri_layout::Hit::Divider { .. }
                    ),
                    "a boundary grabs {probe:?}, which is on {what} of strip {slot} — the \
                     control is dead there, and `input`'s rule 3 is what would have to change"
                );
                assert_eq!(
                    claim(&mut panel, &ctx, &showing(&strips), point(probe)),
                    Claim::Panel,
                    "{what} of strip {slot} is not the panel's at {probe:?}"
                );
            }
        }
    }

    // The guard: the ground under the bay *is* inside a boundary's grab, which
    // is what says the answers above are the clearance and not the grab having
    // gone.
    let region = rect_of(panel.layout(), "mixer");
    let below = Point::new(region.x + region.w * 0.5, region.y + region.h + GRAB * 0.5);
    assert!(
        matches!(
            panel.layout().hit(below, GRAB),
            karakuri_layout::Hit::Divider { .. }
        ),
        "the ground under the mixer is not in the grab of the boundary there, so this test \
         is no longer measuring the clearance it was written for"
    );
    assert!(
        bay.grab(below).is_none(),
        "a point in the ground under the bay took hold of a knob"
    );
}

// ---------------------------------------------------------------------------
// The console remembers nothing
// ---------------------------------------------------------------------------

/// Fader movements alter no internal console state until external updates are received.
#[test]
fn the_value_the_strip_draws_comes_back_from_the_deck() {
    let (mut panel, ctx) = console();
    let strips = strips();
    let before = bay(&panel, &ctx, &strips);
    let at = before.strip(0);
    let (_, fader) = knobs(at, &strips[0]);
    let was = at.fader_at(strips[0].opacity);
    let painted = drawn(&mut panel, &strips, at.rect);

    let bay = bay(&panel, &ctx, &strips);
    take(&mut panel, &bay, fader.center());
    for step in 1..=20 {
        panel.moved(point(egui::pos2(
            fader.center().x,
            fader.center().y + step as f32,
        )));
    }

    // During active drag before values are applied, the strip renders at its model value.
    assert_eq!(
        drawn(&mut panel, &strips, at.rect),
        painted,
        "the strip followed the drag rather than the deck: the console is drawing a value \
         it kept for itself"
    );
    assert_eq!(
        mixer(&ctx, panel.layout(), &strips)
            .expect("a bay")
            .strip(0),
        at,
        "the bay moved while a drag was in hand and nothing had been applied"
    );

    assert!(panel.released(None).is_some());

    // Nothing was applied, so nothing moved. Asked again, from the same
    // values, it is the same bay.
    let after = mixer(&ctx, panel.layout(), &strips).expect("a bay");
    assert_eq!(
        after, before,
        "the bay changed after a drag that told nobody anything, so the console is keeping \
         a value of its own"
    );
    assert_eq!(after.strip(0).fader_at(strips[0].opacity), was);

    // Verify painted rendering matches initial state.
    assert_eq!(
        drawn(&mut panel, &strips, at.rect),
        painted,
        "the strip was painted differently after a drag nobody was told about"
    );

    // And the value the *caller* writes is the one drawn — the deck's seat in
    // this crate, and the only one there is.
    let mut moved = strips.clone();
    moved[0].opacity = 0.05;
    let now = mixer(&ctx, panel.layout(), &moved).expect("a bay");
    assert_ne!(
        now.strip(0).fader_at(moved[0].opacity),
        was,
        "the strip did not follow the value it was handed"
    );
    assert!(near(
        now.strip(0).fader_at(moved[0].opacity).fill.height(),
        travel(at, Knob::Fader { deck: 0 }) * 0.05
    ));
}

// ---------------------------------------------------------------------------
// The frame a drag is owed
// ---------------------------------------------------------------------------

/// Fader drags request frames only when the underlying value actually changes.
#[test]
fn a_drag_that_moves_nothing_asks_for_no_frame() {
    let (mut panel, ctx) = console();
    let strips = strips();
    let bay = bay(&panel, &ctx, &strips);
    let at = bay.strip(0);
    let (_, fader) = knobs(at, &strips[0]);

    take(&mut panel, &bay, fader.center());

    // A move that changed the value: a frame is owed, because the deck is
    // about to be told and the strip is drawn from what it says.
    let top = egui::pos2(fader.center().x, at.fader.min.y);
    let moved = panel.moved(point(top));
    let operation = match &moved {
        Some(Dragged::Fader(operation)) => operation,
        other => panic!("a drag up the track reported {other:?}"),
    };
    assert_eq!(Change::Emitted(Some(operation)).repaint(), Repaint::Now);

    // And a move that did not: the pointer runs on past the top, asks for 1.0
    // again, and the panel says nothing at all.
    for step in 1..=5 {
        let past = egui::pos2(top.x, top.y - 40.0 * step as f32);
        assert_eq!(
            panel.moved(point(past)),
            None,
            "a drag past the top of the track asked for a value it was already at"
        );
        assert_eq!(Change::Emitted(None).repaint(), Repaint::Never);
    }

    // The press itself is the same rule: taking hold of a knob and letting go
    // without moving tells nobody anything.
    panel.released(None);
    take(&mut panel, &bay, fader.center());
    assert_eq!(panel.moved(point(fader.center())), None);
}
