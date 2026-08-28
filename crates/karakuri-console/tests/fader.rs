//! **The mixer's two faders, played with a pointer.**
//!
//! `mixer.rs` is where a strip's rectangles are and which of them are
//! controls. This is what a *hand* does to the two that are, end to end: the
//! knob is taken hold of and the track is not, the value does not jump, both
//! ends of the travel are exactly reachable, the drag emits one operation of
//! the vocabulary for the right deck, it keeps the pointer while the pointer
//! wanders off the strip, the cursor stays what it was, the console remembers
//! none of it, and a drag that changed no value is owed no frame.
//!
//! **None of it needs a device**, and only one thing about it needs `egui`:
//! the strips are laid out with the type in them, which is `mixer.rs`'s own
//! opening. What a fader emits is `karakuri-operation`'s, which has no
//! dependencies at all.
//!
//! # Where this stops, and what carries on in the example
//!
//! Everything here ends at the operation. **Turning it into a `Record` and
//! applying it to a deck is the harness's** — `examples/panel.rs`, where there
//! is a deck to apply it to — and the round trip that closes the loop on a
//! real `Deck` is under that file's `mod gpu`. What is asserted here is the
//! half that says the console did *not* close it for itself: after a whole
//! drag, the bay drawn from the same strips is the same bay.

mod common;

use common::{drawn_once, near, rect_of, PLAUSIBLE};
use karakuri_console::input::{claim, Claim};
use karakuri_console::panel::{Dragged, Grab, InHand, Knob, Panel, Released, GRAB};
use karakuri_console::repaint::{Change, Repaint};
use karakuri_console::room::{size, Room};
use karakuri_console::view::{mixer, Level, Mask, Mixer, Strip, StripBox, Tally, View};
use karakuri_layout::Point;
use karakuri_operation::{BlendMode, Operation};

/// Four strips, so that *which deck a drag named* is a question with four
/// wrong answers rather than one. The values are apart from each other and
/// none of them is at an end, so a fader that moved the wrong strip's control
/// or read the wrong strip's value says so.
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
        })
        .collect()
}

/// A panel at a viewport, solved, with a context that has drawn once — the
/// pair `mixer.rs` and `transport.rs` both open with.
fn console() -> (Panel, egui::Context) {
    let mut panel = Panel::new(PLAUSIBLE.w, PLAUSIBLE.h);
    panel.solve();
    (panel, drawn_once())
}

fn bay<'a>(panel: &Panel, ctx: &egui::Context, strips: &'a [Strip]) -> Mixer<'a> {
    mixer(ctx, panel.layout(), strips).expect("the mixer bay draws its strips")
}

fn point(p: egui::Pos2) -> Point {
    Point::new(p.x, p.y)
}

/// Where a strip's two knobs are, at the values that strip is carrying.
fn knobs(at: StripBox, strip: &Strip) -> (egui::Rect, egui::Rect) {
    (at.trim_at(strip.gain).knob, at.fader_at(strip.opacity).knob)
}

/// **What one fader's travel is, worked out here from the stylesheet's own
/// numbers rather than asked of the crate** — so that a value read back wrong
/// is two numbers disagreeing rather than one function agreeing with itself.
///
/// The trim's fill runs its track edge to edge (`.fader b`), and the tall
/// one's sits [`size::VFADER_INSET`] inside its well on every side
/// (`.vfader b`).
fn travel(at: StripBox, knob: Knob) -> f32 {
    match knob {
        Knob::Trim => at.trim.width(),
        Knob::Fader => at.fader.height() - size::VFADER_INSET * 2.0,
    }
}

/// Take a knob in hand, at `p`, and return what the panel now holds. Panics
/// where `p` is not on a knob, which is the failure worth reading.
fn take(panel: &mut Panel, bay: &Mixer, p: egui::Pos2) -> Grab {
    let grab = bay
        .grab(point(p))
        .unwrap_or_else(|| panic!("nothing to take hold of at {p:?}"));
    panel.grab(point(p), grab);
    grab
}

/// **One strip as it is painted**: where every shape a whole frame put wholly
/// inside `rect` ended up. `mixer.rs`'s own helper, and written again here for
/// the reason that one gives.
///
/// The **bounds** of each shape rather than the shape, because what a value
/// moves is where a mark is — a fill that grew, a knob that slid — and a
/// failure that prints fifteen rectangles is one a reader can act on where a
/// failure that prints fifteen glyph-by-glyph galleys is not.
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

/// **A press on the knob takes it; a press on the track does nothing.**
///
/// A fader at 0.3 whose top is clicked must not jump to 1.0 — that is a change
/// to the mix nobody asked for, made on stage — so the track is not a target
/// at all, and this asserts it at both ends of both tracks and over the whole
/// strip.
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
        assert_eq!((took.deck(), took.knob()), (deck, Knob::Trim));
        let took = bay.grab(point(fader.center())).expect("the fader's knob");
        assert_eq!((took.deck(), took.knob()), (deck, Knob::Fader));

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

/// **The whole strip, swept**, which is the other direction: exactly the two
/// knobs and nothing else in it answers a grab.
///
/// A grid rather than named points, because *the track is not a target* is a
/// claim about every point on it and the interesting failure — a hit test that
/// widened to the track, or to the strip — is one this catches and the named
/// points above might walk between.
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

/// **A press keeps whatever it took hold at, so the value does not jump.**
///
/// The press is deliberately *off* the knob's centre, which is where the
/// mistake shows: a drag that mapped the pointer straight onto the track would
/// move the value by the distance between the pointer and the knob's middle
/// the instant the button went down, and a hand that meant to nudge a fader
/// would find it somewhere else.
///
/// Two assertions, and they are not the same one:
///
/// 1. A move to the point the press was made at emits **nothing** — the value
///    is where it was.
/// 2. A move of `n` pixels asks for the value `n` pixels away **from where the
///    knob was**, not from where the pointer is.
#[test]
fn the_grab_keeps_its_offset_so_the_value_does_not_jump() {
    let (mut panel, ctx) = console();
    let strips = strips();
    let bay = bay(&panel, &ctx, &strips);
    let strip = &strips[0];
    let at = bay.strip(0);
    let (_, fader) = knobs(at, strip);
    let span = travel(at, Knob::Fader);

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

    // Ten pixels up the track is ten pixels' worth of value up from where the
    // fader **was**, and a fader fills from the bottom.
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

/// **The bottom of the travel is exactly 0.0 and the top is exactly 1.0**, on
/// both faders — `karakuri-midi`'s `GAIN_RANGE` argument reached from the
/// other side: *"chosen so that both ends of a fader are exact … a fader whose
/// top is unity is what a fader means."*
///
/// Exactly, not nearly: this compares with `==`, because a mixer whose faders
/// cannot be matched is the failure that argument is about, and 0.999997 is
/// not unity.
///
/// **And a gain above 1.0 cannot be asked for**, which is ADR-0178's recorded
/// gap rather than a ceiling this control invented: the trim is drawn over
/// `[0, 1]` while `Deck::set_gain` is deliberately unclamped for an HDR mix.
/// What is asserted is that the top of the drag is unity and stays unity,
/// however far past the end the pointer runs.
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
    panel.released();

    // The tall one: a column, filling from the **bottom**, and its fill sits
    // `VFADER_INSET` inside the well at both ends.
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

/// **The right operation, for the right deck, from the right control.**
///
/// Four strips and two knobs each, so a fader that named slot 0 whatever it
/// was dragged, or emitted a gain for the opacity fader, fails eight ways.
/// This is `karakuri-operation`'s first customer, and what it asserts is the
/// whole of what a GUI component is: pointer motion into a number, aimed at a
/// named target.
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
            panel.released(),
            Some(Released::Let {
                deck,
                knob: Knob::Trim
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
            panel.released(),
            Some(Released::Let {
                deck,
                knob: Knob::Fader
            })
        );
    }
}

// ---------------------------------------------------------------------------
// The claim, the cursor, and the boundary underneath
// ---------------------------------------------------------------------------

/// **A fader in hand keeps its claim wherever the pointer has wandered to**,
/// which is `input`'s rule 1 covering the second kind of drag without a word
/// being added to it.
///
/// The pointer is run right out of the strip, out of the bay, across the panel
/// and off the viewport, and every event is still the panel's — including a
/// wheel, which is a claim withheld rather than an action taken. Then the
/// button comes up and the same point is `egui`'s again, which is what says
/// the claim was the *gesture's* and not the position's.
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
        assert_eq!(claim(&mut panel, &ctx, &strips, away), Claim::Egui);
        // The knob is the panel's, by rule 3.
        assert_eq!(
            claim(&mut panel, &ctx, &strips, point(held)),
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
            claim(&mut panel, &ctx, &strips, wandered),
            Claim::Panel,
            "the fader lost its claim at {wandered:?}"
        );
    }

    // The release is decided before it is performed, and afterwards the
    // pointer where it is standing is egui's again.
    assert_eq!(claim(&mut panel, &ctx, &strips, away), Claim::Panel);
    assert!(panel.released().is_some());
    assert_eq!(claim(&mut panel, &ctx, &strips, away), Claim::Egui);
}

/// **A fader drag draws no resize cursor**, wherever the pointer has got to.
///
/// `View::cursor` draws one from the boundary in hand, or from the boundary
/// the pointer is over. A fader is neither: it is a gesture that resizes
/// nothing, and the pointer running across a boundary in the middle of one is
/// the ordinary case, not the odd one. So the cursor stays the arrow it is
/// everywhere else on this panel — and the assertion is positive rather than
/// *not a resize*, because "not one of two" is a claim that survives the
/// cursor going missing altogether.
///
/// The control beside it is a **boundary** drag doing the opposite from the
/// same position, which is what says the cursor still works at all.
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
    panel.released();
    assert_eq!(cursor(&mut panel), egui::CursorIcon::ResizeHorizontal);
}

/// **No knob is inside a boundary's grab**, which is what keeps rule 2 ahead
/// of rule 3 from ever costing anything in this bay.
///
/// ADR-0176 measured the Outputs chip against `GRAB` rather than asserting in
/// prose that it cleared it, and this is that measurement over every knob of
/// every strip: a boundary gets **first refusal**, so a knob inside one would
/// simply be dead — a control drawn where a press drags a divider instead, and
/// nothing on screen saying so.
///
/// **The question is `Layout::hit`'s and not `claim`'s**, and that distinction
/// is the whole test: `claim` answers `Claim::Panel` for a boundary *and* for
/// a control, so a knob swallowed by a grab would go on answering `Panel` and
/// a test written on it would pass with the defect in place. It was, and it
/// did — `GRAB` was widened to 60 and the first draft of this test did not
/// notice.
///
/// It carries the same guard on itself the Outputs test does: the bay's own
/// bottom edge *is* inside a grab, so it cannot pass by the grab having gone
/// missing.
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
                     control is dead there, and `input`'s rule 2 is what would have to change"
                );
                assert_eq!(
                    claim(&mut panel, &ctx, &strips, point(probe)),
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

/// **A fader that moved a number the console kept would be a second copy of
/// the deck's state**, and this is the assertion that there is none.
///
/// The bay is a function of the strips it is handed and of the arrangement.
/// So a whole drag — press, several moves, release — must leave the bay drawn
/// from the *same* strips **identical**: the fills, the knobs and everything
/// else in the same places, because the deck has not been told anything yet
/// and the console has nothing of its own to show.
///
/// **The frame is drawn as well as the bay laid out**, because the two are not
/// the same claim: a console that patched the values on its way into the paint
/// pass would leave every rectangle where it was and move every mark that is
/// painted from one.
///
/// And then the other direction, which is what makes the rest meaningful: the
/// value written into the strip by whoever owns the deck **does** move the
/// knob, so the bay is following something — just not the panel.
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

    // **In the middle of the gesture**, which is where a kept value lives:
    // the drag has asked for twenty different opacities and nobody has
    // applied any of them, so the strip is still drawn at the one it was
    // handed.
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

    assert!(panel.released().is_some());

    // Nothing was applied, so nothing moved. Asked again, from the same
    // values, it is the same bay.
    let after = mixer(&ctx, panel.layout(), &strips).expect("a bay");
    assert_eq!(
        after, before,
        "the bay changed after a drag that told nobody anything, so the console is keeping \
         a value of its own"
    );
    assert_eq!(after.strip(0).fader_at(strips[0].opacity), was);

    // And the strip **as painted**, which is where a value patched on the way
    // into the frame would show and the rectangles above would not.
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
        travel(at, Knob::Fader) * 0.05
    ));
}

// ---------------------------------------------------------------------------
// The frame a drag is owed
// ---------------------------------------------------------------------------

/// **A drag that moved no value asks for no frame**, and one that moved a
/// value asks for one.
///
/// `Change::Emitted` is the arm, and this is why it is not
/// `Change::Pointer(Claim::Panel)`: that one is `Repaint::Now` for every event
/// the panel claimed, and a fader held against the top of its track while the
/// pointer runs on is a claimed event per pointer sample with nothing on
/// screen changing for any of them.
///
/// It is driven through a real drag rather than by building the two `Change`s
/// by hand, because the claim is about what the drag *returned*.
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
    panel.released();
    take(&mut panel, &bay, fader.center());
    assert_eq!(panel.moved(point(fader.center())), None);
}
