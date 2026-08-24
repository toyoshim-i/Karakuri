//! What the console draws, and who gets a pointer event.
//!
//! None of this needs a window or a device: what to draw is a walk of the
//! arrangement, and who gets an event is a hit test. The one test here that
//! does need a device is not here at all — it is `examples/panel.rs`'s
//! `gpu::egui_paints_the_console_onto_a_device`, under `mod gpu` like every
//! other one in the workspace.

mod common;

use common::{id_of, rect_of, PLAUSIBLE};
use karakuri_console::input::{claim, Claim};
use karakuri_console::panel::Panel;
use karakuri_console::room::{Palette, Room};
use karakuri_console::view::{plan_into, region, Kind, Placed, REGIONS};
use karakuri_layout::Point;

/// The seven bays, read off the mock's `.bay-head`s — `console.html` heads
/// exactly these and nothing else.
const BAYS: &[&str] = &[
    "library",
    "staging",
    "program",
    "inspector",
    "mixer",
    "master",
    "sequencer",
];

/// The transport and the outputs, which carry `class="bay"` for the card
/// styling and no `.bay-head` at all (ADR-0159).
const ROWS: &[&str] = &["transport", "outputs"];

fn planned(panel: &mut Panel) -> Vec<Placed> {
    let mut out = Vec::new();
    plan_into(panel, &mut out);
    out
}

// ---------------------------------------------------------------------------
// What is drawn
// ---------------------------------------------------------------------------

/// **Every leaf of the arrangement is asked to be drawn, and none is skipped.**
///
/// The failure this exists for is silent: a region left out of the table
/// leaves a hole in the panel with nothing anywhere saying so, and the hole is
/// the ground showing through, which is what a divider looks like. So this
/// asks the arrangement rather than the table — every visible leaf has to be
/// in the plan — and then asks the other way, so a region invented in the
/// table that the arrangement does not have is caught too.
#[test]
fn every_leaf_of_the_arrangement_is_drawn_and_none_is_skipped() {
    let mut panel = Panel::new(PLAUSIBLE.w, PLAUSIBLE.h);
    let placed = planned(&mut panel);
    let layout = panel.layout();

    for node in panel.nodes() {
        if layout.is_view(node.id) && layout.visible(node.id) {
            assert!(
                placed.iter().any(|p| p.id == node.id),
                "{:?} is a leaf of the arrangement and nothing draws it",
                layout.name(node.id)
            );
        }
    }

    // The inspector is the one bay that is a split: its head sits above the
    // two panes that tile it, so it is drawn and they are drawn inside it.
    let inspector = id_of(layout, "inspector");
    assert!(
        placed.iter().any(|p| p.id == inspector),
        "the inspector bay's own head is not drawn"
    );

    // And nothing else. `left-pane`, `centre` and `right-pane` are splits an
    // operator folds, not things with a face.
    for p in &placed {
        assert!(
            layout.is_view(p.id) || p.id == inspector,
            "{:?} is a split and is being drawn as a region",
            layout.name(p.id)
        );
        assert_eq!(
            layout.name(p.id),
            Some(p.region.name),
            "a placed region carries a name the arrangement does not agree with"
        );
    }

    // Ten leaves and the inspector, which is every row of the table.
    assert_eq!(placed.len(), REGIONS.len());

    // Tree order, which is what puts the inspector's card under its panes.
    let at = |name: &str| placed.iter().position(|p| p.region.name == name).unwrap();
    assert!(at("inspector") < at("inspector-1"));
    assert!(at("inspector-1") < at("inspector-2"));
}

/// **A bay gets a head and a row does not.**
///
/// Seven bays, two rows and two panes, and the title of each bay is the mock's
/// own word for it — so a bay renamed in the manual and not here shows the old
/// word on the face of the panel, which is exactly what ADR-0159 is about.
#[test]
fn a_bay_gets_a_head_and_a_row_does_not() {
    for name in BAYS {
        match region(name)
            .unwrap_or_else(|| panic!("no region named {name}"))
            .kind
        {
            Kind::Bay { title, .. } => assert_eq!(
                title.to_lowercase(),
                *name,
                "the bay head says {title} and the arrangement calls it {name}"
            ),
            other => panic!("{name} is a bay in the mock and a {other:?} here"),
        }
    }
    for name in ROWS {
        assert_eq!(
            region(name).unwrap().kind,
            Kind::Row,
            "{name} has no heading in the mock and is being given one"
        );
    }
    for name in ["inspector-1", "inspector-2"] {
        assert_eq!(
            region(name).unwrap().kind,
            Kind::Pane,
            "{name} sits inside the Inspector bay and has no head of its own"
        );
    }
    assert_eq!(
        REGIONS
            .iter()
            .filter(|r| matches!(r.kind, Kind::Bay { .. }))
            .count(),
        BAYS.len(),
        "the bay head has seven call sites, which is the whole of why it is a component"
    );
}

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

/// **The palette answers for both rooms**, and answers differently in each.
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

/// Where the panel draws the one control it draws: the `solo` pill in the
/// Program bay's head, near the right end of it. **The place a boundary and a
/// widget are closest**, and so the place the rule is worth stating.
fn on_the_solo_pill(panel: &mut Panel) -> Point {
    panel.solve();
    let program = rect_of(panel.layout(), "program");
    Point::new(program.x + program.w - 24.0, program.y + 14.0)
}

/// **A pointer on a boundary reaches the panel, and `egui` does not see it.**
#[test]
fn a_pointer_on_a_boundary_is_the_panels() {
    let mut panel = Panel::new(PLAUSIBLE.w, PLAUSIBLE.h);
    let p = on_a_boundary(&mut panel);
    assert_eq!(claim(&mut panel, p), Claim::Panel);
}

/// **A pointer anywhere else is `egui`'s** — including on the one thing the
/// panel draws that a hand would reach for.
#[test]
fn a_pointer_off_a_boundary_is_eguis() {
    let mut panel = Panel::new(PLAUSIBLE.w, PLAUSIBLE.h);
    panel.solve();

    let library = rect_of(panel.layout(), "library");
    let middle = Point::new(library.x + library.w * 0.5, library.y + library.h * 0.5);
    assert_eq!(claim(&mut panel, middle), Claim::Egui);

    let pill = on_the_solo_pill(&mut panel);
    assert_eq!(
        claim(&mut panel, pill),
        Claim::Egui,
        "the solo pill is not on a boundary, so it is egui's"
    );

    // Outside the window entirely: nothing to grab, so egui's.
    assert_eq!(claim(&mut panel, Point::new(-40.0, -40.0)), Claim::Egui);
}

/// **A drag in hand keeps its claim, wherever the pointer wanders.**
///
/// This is the one that is a bug waiting to happen. A claim re-decided from
/// the pointer on every event hands the middle of a drag to `egui` the moment
/// the pointer leaves the six pixels either side of the boundary — which it
/// does immediately, because a drag is how a boundary gets anywhere — and then
/// two things think they are dragging. The release matters just as much: asked
/// after `released`, the claim sees no drag and hands `egui` a button-up it
/// never saw the button-down for.
#[test]
fn a_drag_in_hand_keeps_its_claim_wherever_the_pointer_goes() {
    let mut panel = Panel::new(PLAUSIBLE.w, PLAUSIBLE.h);
    let start = on_a_boundary(&mut panel);
    let pill = on_the_solo_pill(&mut panel);
    panel.solve();
    let library = rect_of(panel.layout(), "library");
    let middle = Point::new(library.x + library.w * 0.5, library.y + library.h * 0.5);

    // Before the press, the two elsewhere-points are egui's.
    assert_eq!(claim(&mut panel, pill), Claim::Egui);
    assert_eq!(claim(&mut panel, middle), Claim::Egui);

    panel.press(start);
    assert!(panel.dragging());

    for wandered in [pill, middle, Point::new(-500.0, 4000.0), start] {
        assert_eq!(
            claim(&mut panel, wandered),
            Claim::Panel,
            "a boundary is in hand and the claim was given away at {wandered:?}"
        );
        panel.moved(wandered);
    }

    // The release is still the panel's, and it has to be asked before
    // `released` takes the drag out of hand.
    assert_eq!(claim(&mut panel, middle), Claim::Panel);
    panel.released();
    assert!(!panel.dragging());

    // And afterwards the claim is back where it was.
    assert_eq!(claim(&mut panel, pill), Claim::Egui);
    assert_eq!(claim(&mut panel, middle), Claim::Egui);
    let boundary = on_a_boundary(&mut panel);
    assert_eq!(claim(&mut panel, boundary), Claim::Panel);
}
