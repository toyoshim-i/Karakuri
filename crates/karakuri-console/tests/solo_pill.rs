//! **The `solo` pill in the Program bay's head: the console's fourteenth
//! control, and the first one it draws inside a bay head.**
//!
//! `docs/manual/operations.html`'s *Solo a region* names `solo` in the panel
//! column, and `docs/manual/console.html` is where that word is a place: the
//! Program bay's head carries one pill and its tooltip says what it does —
//! *"Solo the program view: the panel folds away and only the picture is left,
//! which is also how you capture this window."* So the region is the picture's
//! and the control is that capsule, and neither is chosen here.
//!
//! # What this file is for, control by control
//!
//! - **The capsule is where a bay head's pills go**, which is the claim that
//!   makes one derivation out of the painting and the pressing: `bay_head`
//!   paints from `head_pills` and [`program_head`] reads it, so a pill drawn
//!   somewhere a press cannot land is a failure here rather than something an
//!   operator finds.
//! - **A press asks for the solo, and a second press undoes it** — two
//!   operations chosen from the layout, which is `Outputs::op`'s rule and not
//!   a toggle.
//! - **It names the picture and never the pointer**, which is the one way it
//!   differs from `s` on the keyboard.
//! - **The boundary above it keeps 0.75 of it**, and that is the number this
//!   file exists to state. See [`the_boundary_above_the_bay_keeps_the_top_of_the_capsule`].
//! - **A bay that is not laid out has no pill**, which is `mask.rs`'s
//!   sentence one bay over: a control in a bay with no room for it is not a
//!   control.

mod common;

use common::{drawn_once, near, rect_of, showing, PLAUSIBLE, SMALLEST};
use karakuri_console::input::{claim, Claim};
use karakuri_console::panel::{Op, Outcome, Panel, GRAB};
use karakuri_console::room::size;
use karakuri_console::view::{program_head, MOCK_CANVAS};
use karakuri_layout::{Hit, Point};

/// A console at `viewport`, arranged for the mock's canvas and drawn once —
/// which is what a `.pill`'s width takes.
fn console(viewport: karakuri_layout::Rect) -> (Panel, egui::Context) {
    (common::arranged(viewport, MOCK_CANVAS), drawn_once())
}

fn point(p: egui::Pos2) -> Point {
    Point::new(p.x, p.y)
}

// ---------------------------------------------------------------------------
// Where it is
// ---------------------------------------------------------------------------

/// **The capsule is a `.pill` in the Program bay's head, right-aligned past
/// the grip** — the arithmetic `bay_head` paints by, asserted off the
/// derivation a press is hit-tested against.
///
/// The two cannot be two boxes: `head_pills` is what places them and both
/// callers ask it. What this pins is that the box it places is the mock's —
/// `.pill`'s height, centred in a 27-tall head, its right edge one
/// `PILL_GAP` in from the grip and the grip `HEAD_PAD_X` in from the bay.
#[test]
fn the_capsule_is_a_pill_in_the_bay_head() {
    for viewport in [SMALLEST, PLAUSIBLE] {
        let (mut panel, ctx) = console(viewport);
        panel.solve();
        let bay = rect_of(panel.layout(), "program");
        let head = program_head(&ctx, panel.layout()).expect("the Program bay draws its pill");

        assert!(
            near(head.solo.height(), size::PILL_H),
            "the capsule is {} tall and a `.pill` is {}",
            head.solo.height(),
            size::PILL_H
        );
        // Centred in the head, which is `align-items: center` on the row.
        let mid = bay.y + size::HEAD_H * 0.5;
        assert!(
            near(head.solo.center().y, mid),
            "the capsule's middle is at {} and the head's is at {mid}",
            head.solo.center().y
        );
        // The grip is at the right edge, and the pill one `PILL_GAP` inside
        // it. The grip's own width is the console's, so what is asserted is
        // that the pill is left of the grip's gap by *something* and inside
        // the head — the exact dot geometry is `grip_dots`'s and is not this
        // file's to transcribe.
        assert!(
            head.solo.max.x < bay.x + bay.w - size::HEAD_PAD_X - size::PILL_GAP,
            "the capsule runs into the grip: it ends at {} and the head's padding starts at {}",
            head.solo.max.x,
            bay.x + bay.w - size::HEAD_PAD_X
        );
        let head_box =
            egui::Rect::from_min_size(egui::pos2(bay.x, bay.y), egui::vec2(bay.w, size::HEAD_H));
        assert!(
            head_box.contains_rect(head.solo),
            "the capsule {:?} is not inside the head {head_box:?}",
            head.solo
        );
    }
}

/// **The pill names the picture, and it is the picture the arrangement
/// names** — not the bay it is drawn in, and not whatever the pointer is
/// over.
#[test]
fn the_pill_names_the_picture_and_never_the_pointer() {
    let (mut panel, ctx) = console(PLAUSIBLE);
    panel.solve();
    let picture = panel
        .layout()
        .find("program-view")
        .expect("the arrangement names the picture");
    let head = program_head(&ctx, panel.layout()).expect("the Program bay draws its pill");
    assert_eq!(head.id, picture);
    assert_eq!(head.op(), Op::Solo(picture));
}

// ---------------------------------------------------------------------------
// What a press asks for
// ---------------------------------------------------------------------------

/// **A press solos the picture and a second press undoes it**, and both are
/// performed here rather than named: a control that soloed and could not undo
/// it would be half of *Solo a region*, and the undo is the half no other
/// gesture can reach — a solo takes every other control off the screen.
#[test]
fn a_press_solos_the_picture_and_a_second_press_undoes_it() {
    let (mut panel, ctx) = console(PLAUSIBLE);
    panel.solve();
    let picture = panel.layout().find("program-view").expect("the picture");
    let library = panel.layout().find("library").expect("the library");
    assert!(
        panel.layout().visible(library),
        "the library is already off the screen, so soloing the picture would prove nothing"
    );

    let head = program_head(&ctx, panel.layout()).expect("a pill");
    assert_eq!(panel.op(head.op()), Outcome::Soloed(picture));
    panel.solve();
    assert!(
        !panel.layout().visible(library),
        "the picture is soloed and the library is still on the screen"
    );

    // The head derived again, on the console the solo left — the pill has
    // moved with the bay, and a remembered capsule would be a press somewhere
    // it no longer is.
    let head = program_head(&ctx, panel.layout())
        .expect("the pill is still drawn with the picture soloed, and it is the only one left");
    assert!(
        head.soloed,
        "the pill did not read the solo back off the layout"
    );
    assert_eq!(head.op(), Op::Unsolo);
    assert_eq!(panel.op(head.op()), Outcome::Unsoloed { was: true });
    panel.solve();
    assert!(
        panel.layout().visible(library),
        "undoing the solo left the library folded"
    );
}

/// **A press off the capsule asks for nothing and is not claimed** — the rule
/// every control here lives by: a control claims what it acts on and no more.
#[test]
fn a_press_off_the_pill_asks_for_nothing_and_is_not_claimed() {
    let (mut panel, ctx) = console(PLAUSIBLE);
    panel.solve();
    let head = program_head(&ctx, panel.layout()).expect("a pill");
    // Just left of the capsule, in the head's own ground, and just below it,
    // in the bay's body.
    let beside = egui::pos2(head.solo.min.x - 6.0, head.solo.center().y);
    let under = egui::pos2(head.solo.center().x, head.solo.max.y + 8.0);
    for off in [beside, under] {
        assert!(
            !head.hit(point(off)),
            "the pill claims {off:?}, which is off the capsule"
        );
        assert_eq!(
            claim(&mut panel, &ctx, &showing(&[]), point(off)),
            Claim::Egui,
            "{off:?} is off every control and off every boundary, so it is egui's"
        );
    }
}

// ---------------------------------------------------------------------------
// The boundary gets first refusal, and this is the first control it reaches
// ---------------------------------------------------------------------------

/// **The first control on this console that does not clear a boundary's grab,
/// and the number is 0.75 of a pixel.**
///
/// `karakuri_console::input` states the hazard and every other control's test
/// answers it the same way — the Outputs sink clears by 1.75, the deck head's
/// chips by 10, the Master bay's out by 37.75. This one does not, and the
/// reason is geometry neither the rule nor this crate chose: a bay head is
/// `HEAD_H` = 27 and a `.pill` is `PILL_H` = 16.5 centred in it, so there is
/// (27 - 16.5) / 2 = **5.25** of head above the capsule; the Program bay is
/// the first child of the centre column, so its top edge is the body row's,
/// and the boundary between the transport row and the body grabs `GRAB` = 6
/// past it.
///
/// **So this asserts the overlap rather than its absence**, in both
/// directions: the capsule's top edge is the boundary's and its centre and
/// bottom are the panel's. `input`'s rule 3 is what decides that, it decides
/// it the same way every time, and the hazard the rule was written for —
/// *"both do think they are dragging"* — does not arise. What is lost is the
/// sliver.
///
/// **It fails if the sliver grows**, which is what it is for: a shorter bay
/// head, a taller pill or a wider `GRAB` each make more of the control dead,
/// and the fix then is a change to `docs/manual/console.html` or to the rule,
/// deliberately, rather than a nudge.
#[test]
fn the_boundary_above_the_bay_keeps_the_top_of_the_capsule() {
    for viewport in [SMALLEST, PLAUSIBLE] {
        let (mut panel, ctx) = console(viewport);
        panel.solve();
        let bay = rect_of(panel.layout(), "program");
        let head = program_head(&ctx, panel.layout()).expect("a pill");

        // The clearance, stated against the constant it does not beat.
        let clearance = (size::HEAD_H - size::PILL_H) * 0.5;
        assert!(near(clearance, 5.25), "the clearance is {clearance}");
        assert!(near(head.solo.min.y - bay.y, clearance));
        assert!(
            clearance < GRAB,
            "the capsule now clears the grab by {clearance} against {GRAB} — this test is \
             the wrong shape for that, and the right shape is the one `tests/outputs.rs` has"
        );
        let lost = GRAB - clearance;
        assert!(
            near(lost, 0.75),
            "the boundary keeps {lost} of the capsule and the arithmetic above says 0.75"
        );

        // The top edge is the boundary's...
        assert!(
            matches!(
                panel.layout().hit(point(head.solo.center_top()), GRAB),
                Hit::Divider { .. }
            ),
            "the top of the capsule is not inside the boundary's band, so the whole of this \
             test's arithmetic is about something that is not happening"
        );
        // ...and everything from `lost` down is the control's, which is what
        // says the press an operator makes lands on it.
        let below = egui::pos2(head.solo.center().x, head.solo.min.y + lost + 0.5);
        for probe in [below, head.solo.center(), head.solo.center_bottom()] {
            assert!(
                !matches!(panel.layout().hit(point(probe), GRAB), Hit::Divider { .. }),
                "a boundary grabs {probe:?}, which is past the sliver it is owed"
            );
            assert!(head.hit(point(probe)), "the pill does not claim {probe:?}");
            assert_eq!(
                claim(&mut panel, &ctx, &showing(&[]), point(probe)),
                Claim::Panel,
                "the panel does not get a press at {probe:?}, which is on its own control"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// A bay that is not laid out
// ---------------------------------------------------------------------------

/// **A folded Program bay has no pill to press**, whichever fold hid it — the
/// bay itself, or the column that encloses it. And unfolding puts it back,
/// because nothing here is a latch.
#[test]
fn a_folded_program_bay_has_no_pill_to_press() {
    for enclosing in [false, true] {
        let (mut panel, ctx) = console(PLAUSIBLE);
        panel.solve();
        let bay = panel.layout().find("program").expect("the Program bay");
        let where_it_was = program_head(&ctx, panel.layout())
            .expect("a pill before anything is folded")
            .solo
            .center();

        let folds = match enclosing {
            true => panel.layout().find("centre").expect("the centre column"),
            false => bay,
        };
        panel.op(Op::Fold(folds));
        panel.solve();
        assert!(
            !panel.layout().visible(bay),
            "folding {} left the Program bay laid out",
            match enclosing {
                true => "the centre column",
                false => "the bay",
            }
        );
        assert!(
            program_head(&ctx, panel.layout()).is_none(),
            "a folded Program bay still laid its pill out (enclosing: {enclosing})"
        );

        panel.op(Op::Unfold(folds));
        panel.solve();
        assert!(
            program_head(&ctx, panel.layout()).is_some(),
            "unfolding the bay left the pill dead (enclosing: {enclosing})"
        );
        // And in the same place, so the answer above is the fold rather than
        // the control having moved.
        assert!(near(
            program_head(&ctx, panel.layout())
                .expect("a pill")
                .solo
                .center()
                .y,
            where_it_was.y
        ));
    }
}

/// **The floor**: a console that has never drawn has no fonts, so it has no
/// pill — and a test suite that got `None` everywhere would pass every
/// assertion above by never finding a control at all.
#[test]
fn a_console_that_has_never_drawn_has_no_pill() {
    let panel = common::arranged(PLAUSIBLE, MOCK_CANVAS);
    assert!(
        program_head(&egui::Context::default(), panel.layout()).is_none(),
        "a pill was laid out before `egui` had any fonts"
    );
}
