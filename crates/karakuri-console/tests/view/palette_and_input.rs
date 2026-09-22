use super::view_common::*;

// ---------------------------------------------------------------------------
// The palette
// ---------------------------------------------------------------------------

/// Every colour in a palette, so a transcription that left one behind is one
/// comparison rather than fourteen.
fn colours(p: &Palette) -> Vec<(&'static str, egui::Color32)> {
    vec![
        ("ground", p.ground),
        ("panel", p.panel),
        ("well", p.well),
        ("line", p.line),
        ("hair", p.hair),
        ("text", p.text),
        ("dim", p.dim),
        ("faint", p.faint),
        ("mint", p.mint),
        ("pink", p.pink),
        ("lav", p.lav),
        ("sun", p.sun),
        ("tint", p.tint),
        ("glow", p.glow),
        ("glow_pink", p.glow_pink),
    ]
}

/// Perceived lightness, near enough to tell a white room from a black one.
fn luma(c: egui::Color32) -> f32 {
    (0.2126 * c.r() as f32 + 0.7152 * c.g() as f32 + 0.0722 * c.b() as f32) / 255.0
}

/// The palette answers for both rooms, and answers differently in each.
///
/// The failure worth catching is a colour transcribed once and left: the two
/// blocks in `style.css` are fifteen near-identical lines each, and one line
/// copied from the wrong block is invisible until somebody switches rooms in a
/// dark hall.
#[test]
fn the_palette_answers_for_both_rooms() {
    let day = Room::Day.palette();
    let night = Room::Night.palette();

    for ((name, a), (_, b)) in colours(&day).into_iter().zip(colours(&night)) {
        assert_ne!(
            a, b,
            "--c-{name} is the same colour in both rooms, so one of them was not transcribed"
        );
    }

    // "Pastel on white by day and lit on black in a dark hall."
    assert!(luma(day.ground) > 0.8, "the day ground is not white");
    assert!(luma(day.text) < 0.3, "the day ink is not dark");
    assert!(luma(night.ground) < 0.1, "the night ground is not black");
    assert!(luma(night.text) > 0.8, "the night ink is not light");

    // A bay is a card standing off the ground, in both rooms. This is what
    // makes an empty bay visible at all, which is the whole of this pass.
    assert!(luma(day.panel) > luma(day.ground));
    assert!(luma(night.panel) > luma(night.ground));

    // The room key toggles, and toggling twice is where it started.
    assert_eq!(Room::Day.other(), Room::Night);
    assert_eq!(Room::Night.other().other(), Room::Night);
    assert_eq!(Room::Day.palette(), Palette::DAY);
    assert_eq!(Room::Night.palette(), Palette::NIGHT);
}

// ---------------------------------------------------------------------------
// The input rule
// ---------------------------------------------------------------------------

/// The middle of the first boundary the arrangement has.
fn on_a_boundary(panel: &mut Panel) -> Point {
    panel.solve();
    let layout = panel.layout();
    let (split, index) = layout.boundaries().next().expect("no boundary at all");
    let gap = layout.boundary(split, index).expect("no pair");
    Point::new(gap.x + gap.w * 0.5, gap.y + gap.h * 0.5)
}

/// Returns coordinates within the program bay's solo pill near the boundary.
fn on_the_solo_pill(panel: &mut Panel) -> Point {
    panel.solve();
    let program = rect_of(panel.layout(), "program");
    Point::new(program.x + program.w - 24.0, program.y + 14.0)
}

/// A pointer on a boundary reaches the panel, and `egui` does not see it.
#[test]
fn a_pointer_on_a_boundary_is_the_panels() {
    let mut panel = Panel::new(PLAUSIBLE.w, PLAUSIBLE.h);
    let p = on_a_boundary(&mut panel);
    assert_eq!(
        claim(&mut panel, &drawn_once(), &showing(&[]), p),
        Claim::Panel
    );
}

/// A pointer off a boundary and off every control is `egui`'s — and the `solo`
/// pill is the case that says which of the two rules answered.
///
/// The pill used to be in the first half of that sentence: it was the one thing
/// the panel drew that a hand would reach for, and a press on it was `egui`'s
/// because nothing here answered it. It is a control now, so it is the panel's
/// under rule 4 — not under rule 3, which is the distinction this test is for,
/// and the point is far enough from the boundary above the bay that only rule 4
/// can be giving it away.
#[test]
fn a_pointer_off_a_boundary_is_eguis() {
    let ctx = drawn_once();
    let mut panel = Panel::new(PLAUSIBLE.w, PLAUSIBLE.h);
    panel.solve();

    let library = rect_of(panel.layout(), "library");
    let middle = Point::new(library.x + library.w * 0.5, library.y + library.h * 0.5);
    assert_eq!(claim(&mut panel, &ctx, &showing(&[]), middle), Claim::Egui);

    let pill = on_the_solo_pill(&mut panel);
    assert!(
        !matches!(
            panel.layout().hit(pill, karakuri_console::panel::GRAB),
            karakuri_layout::Hit::Divider { .. }
        ),
        "the point taken for the solo pill is on a boundary, so what claims it below says \
         nothing about the control"
    );
    assert_eq!(
        claim(&mut panel, &ctx, &showing(&[]), pill),
        Claim::Panel,
        "the solo pill is a control and no boundary grabs this point, so rule 4 is what \
         gives the press to the panel"
    );

    // Outside the window entirely: nothing to grab, so egui's.
    assert_eq!(
        claim(&mut panel, &ctx, &showing(&[]), Point::new(-40.0, -40.0)),
        Claim::Egui
    );
}

/// A drag in hand keeps its claim, wherever the pointer wanders.
///
/// This is the one that is a bug waiting to happen. A claim re-decided from the
/// pointer on every event hands the middle of a drag to `egui` the moment the
/// pointer leaves the six pixels either side of the boundary — which it does
/// immediately, because a drag is how a boundary gets anywhere — and then two
/// things think they are dragging. The release matters just as much: asked
/// after `released`, the claim sees no drag and hands `egui` a button-up it
/// never saw the button-down for.
///
/// The `solo` pill is in the wander now rather than in the before and after,
/// because it stopped being an elsewhere-point the day it became a control:
/// rule 1 has to beat rule 4 as well as rule 3, and a point that is the panel's
/// either way cannot say whether the drag kept its claim. What carries that
/// half is the library's middle, which is on no control and on no boundary, and
/// it goes `egui` -> panel -> `egui` across the gesture.
#[test]
fn a_drag_in_hand_keeps_its_claim_wherever_the_pointer_goes() {
    let ctx = drawn_once();
    let mut panel = Panel::new(PLAUSIBLE.w, PLAUSIBLE.h);
    let start = on_a_boundary(&mut panel);
    let pill = on_the_solo_pill(&mut panel);
    panel.solve();
    let library = rect_of(panel.layout(), "library");
    let middle = Point::new(library.x + library.w * 0.5, library.y + library.h * 0.5);

    // Before the press: the elsewhere-point is egui's, and the pill is the
    // panel's for rule 4 rather than for anything this test is about.
    assert_eq!(claim(&mut panel, &ctx, &showing(&[]), middle), Claim::Egui);
    assert_eq!(claim(&mut panel, &ctx, &showing(&[]), pill), Claim::Panel);

    panel.press(start);
    assert!(panel.dragging());

    for wandered in [pill, middle, Point::new(-500.0, 4000.0), start] {
        assert_eq!(
            claim(&mut panel, &ctx, &showing(&[]), wandered),
            Claim::Panel,
            "a boundary is in hand and the claim was given away at {wandered:?}"
        );
        panel.moved(wandered);
    }

    // The release is still the panel's, and it has to be asked before
    // `released` takes the drag out of hand.
    assert_eq!(claim(&mut panel, &ctx, &showing(&[]), middle), Claim::Panel);
    panel.released(None);
    assert!(!panel.dragging());

    // And afterwards the claim is back where it was.
    assert_eq!(claim(&mut panel, &ctx, &showing(&[]), middle), Claim::Egui);
    assert_eq!(claim(&mut panel, &ctx, &showing(&[]), pill), Claim::Panel);
    let boundary = on_a_boundary(&mut panel);
    assert_eq!(
        claim(&mut panel, &ctx, &showing(&[]), boundary),
        Claim::Panel
    );
}
