//! Program bay head `solo` pill layout, operation dispatch, and boundary grab interactions.

mod common;

use common::{drawn_once, near, point, rect_of, showing, PLAUSIBLE, SMALLEST};
use karakuri_console::input::{claim, Claim};
use karakuri_console::panel::{Op, Outcome, Panel, GRAB};
use karakuri_console::room::size;
use karakuri_console::view::{program_head, MOCK_CANVAS};
use karakuri_layout::Hit;
use karakuri_operation::gate::Open;

/// A console at `viewport`, arranged for the mock's canvas and drawn once —
/// which is what a `.pill`'s width takes.
fn console(viewport: karakuri_layout::Rect) -> (Panel, egui::Context) {
    (common::arranged(viewport, MOCK_CANVAS), drawn_once())
}

// ---------------------------------------------------------------------------
// Where it is
// ---------------------------------------------------------------------------

/// The solo pill sits inside the 27px Program bay head, right-aligned before the grip.
#[test]
fn the_capsule_is_a_pill_in_the_bay_head() {
    for viewport in [SMALLEST, PLAUSIBLE] {
        let (mut panel, ctx) = console(viewport);
        panel.solve();
        let bay = rect_of(panel.layout(), "program");
        let head = program_head(&ctx, panel.layout(), Open::CLOSED)
            .expect("the Program bay draws its pill");

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
        // The pill is inset from the right grip by `PILL_GAP` and fully enclosed in the head.
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

/// The pill names the picture, and it is the picture the arrangement names —
/// not the bay it is drawn in, and not whatever the pointer is over.
#[test]
fn the_pill_names_the_picture_and_never_the_pointer() {
    let (mut panel, ctx) = console(PLAUSIBLE);
    panel.solve();
    let picture = panel
        .layout()
        .find("program-view")
        .expect("the arrangement names the picture");
    let head =
        program_head(&ctx, panel.layout(), Open::CLOSED).expect("the Program bay draws its pill");
    assert_eq!(head.id, picture);
    assert_eq!(head.op(), Op::Solo(picture));
}

// ---------------------------------------------------------------------------
// What a press asks for
// ---------------------------------------------------------------------------

/// A press solos the picture and a second press undoes it, and both are
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

    let head = program_head(&ctx, panel.layout(), Open::CLOSED).expect("a pill");
    assert_eq!(panel.op(head.op()), Outcome::Soloed(picture));
    panel.solve();
    assert!(
        !panel.layout().visible(library),
        "the picture is soloed and the library is still on the screen"
    );

    // The head derived again, on the console the solo left — the pill has
    // moved with the bay, and a remembered capsule would be a press somewhere
    // it no longer is.
    let head = program_head(&ctx, panel.layout(), Open::CLOSED)
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

/// A press off the capsule asks for nothing and is not claimed — the rule every
/// control here lives by: a control claims what it acts on and no more.
#[test]
fn a_press_off_the_pill_asks_for_nothing_and_is_not_claimed() {
    let (mut panel, ctx) = console(PLAUSIBLE);
    panel.solve();
    let head = program_head(&ctx, panel.layout(), Open::CLOSED).expect("a pill");
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

/// Boundary `GRAB` overlaps the top 0.75px of the pill; clicks on the sliver drag the boundary.
#[test]
fn the_boundary_above_the_bay_keeps_the_top_of_the_capsule() {
    for viewport in [SMALLEST, PLAUSIBLE] {
        let (mut panel, ctx) = console(viewport);
        panel.solve();
        let bay = rect_of(panel.layout(), "program");
        let head = program_head(&ctx, panel.layout(), Open::CLOSED).expect("a pill");

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

/// A folded Program bay has no pill to press, whichever fold hid it — the bay
/// itself, or the column that encloses it. And unfolding puts it back, because
/// nothing here is a latch.
#[test]
fn a_folded_program_bay_has_no_pill_to_press() {
    for enclosing in [false, true] {
        let (mut panel, ctx) = console(PLAUSIBLE);
        panel.solve();
        let bay = panel.layout().find("program").expect("the Program bay");
        let where_it_was = program_head(&ctx, panel.layout(), Open::CLOSED)
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
            program_head(&ctx, panel.layout(), Open::CLOSED).is_none(),
            "a folded Program bay still laid its pill out (enclosing: {enclosing})"
        );

        panel.op(Op::Unfold(folds));
        panel.solve();
        assert!(
            program_head(&ctx, panel.layout(), Open::CLOSED).is_some(),
            "unfolding the bay left the pill dead (enclosing: {enclosing})"
        );
        // And in the same place, so the answer above is the fold rather than
        // the control having moved.
        assert!(near(
            program_head(&ctx, panel.layout(), Open::CLOSED)
                .expect("a pill")
                .solo
                .center()
                .y,
            where_it_was.y
        ));
    }
}

/// The floor: a console that has never drawn has no fonts, so it has no pill —
/// and a test suite that got `None` everywhere would pass every assertion above
/// by never finding a control at all.
#[test]
fn a_console_that_has_never_drawn_has_no_pill() {
    let panel = common::arranged(PLAUSIBLE, MOCK_CANVAS);
    assert!(
        program_head(&egui::Context::default(), panel.layout(), Open::CLOSED).is_none(),
        "a pill was laid out before `egui` had any fonts"
    );
}
