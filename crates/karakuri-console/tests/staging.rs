//! **The Staging lane: a card, a head, and nothing else — at every window,
//! and whatever else is handed to the console.**
//!
//! This bay is the one whose first pass under
//! [ADR-0200](../../../docs/adr/0200-a-bays-first-pass-draws-the-values-that-exist-and-omits-the-rest.md)
//! draws no body at all, and the argument is written out in `view`'s module
//! documentation: a row is a **node whose newest version has not been
//! settled**, the program this panel is drawn by builds both its deck slots
//! with `HotSwap::fixed`, and no `swap::Event` of any variant is emitted in
//! any run of it. So there is no candidate to list, and the page's own empty
//! state is *"no row, no placeholder, and no standing sentence"*.
//!
//! **A decision to draw nothing needs a test more than a decision to draw
//! something does**, which is why this file exists rather than a line in
//! `view.rs`: nothing about deleting the argument would make a suite fail, and
//! the three things that would be added back are the three the page refuses.
//! So this counts what lands in the bay against a bay that is known to have no
//! body, and asks the same question at the two windows the console is claimed
//! to work at.
//!
//! Three things:
//!
//! 1. **The bay draws the shapes of a bay with no body**, asserted against the
//!    Sequencer bay — the lane's twin in this console's furniture: a title, no
//!    pill, no grip, and nothing in its body at all. It is a claim about the
//!    paint pass, so it is made by drawing a frame and counting.
//! 2. **Nothing the console is handed puts anything there.** The Library's
//!    names, the mixer's strips, the inspector's panes and a picture are all
//!    written onto the `View` and the lane is the same two shapes after, which
//!    is what says the bay reads none of them.
//! 3. **Nothing in it is a control**, stated rather than inferred from the
//!    absence of a hit test, and carrying a negative control because a bay
//!    with nothing in it has no defect to be run against.
//!
//! None of it needs a window or a device.

mod common;

use common::{drawn_once, rect_of, PLAUSIBLE, SMALLEST};
use karakuri_console::input::{claim, Claim};
use karakuri_console::panel::Panel;
use karakuri_console::room::Room;
use karakuri_console::view::{Kind, Level, Pane, Strip, Tally, View, PANES};
use karakuri_layout::{Point, Rect};

/// A panel at a viewport, solved — the pair every test here starts from.
fn console(viewport: Rect) -> Panel {
    let mut panel = Panel::new(viewport.w, viewport.h);
    panel.solve();
    panel
}

/// `egui`'s rectangle, from `karakuri_layout`'s.
fn to_egui(r: Rect) -> egui::Rect {
    egui::Rect::from_min_size(egui::pos2(r.x, r.y), egui::vec2(r.w, r.h))
}

/// One mixer strip, live and settled — `mixer.rs`'s own, with everything this
/// file does not care about at rest.
fn strip(name: &str) -> Strip {
    Strip {
        name: name.to_owned(),
        tally: Tally::Live,
        requested: Tally::Live,
        gain: 0.72,
        gain_to: None,
        opacity: 1.0,
        opacity_to: None,
        blend: karakuri_operation::BlendMode::Add,
        mask: karakuri_console::view::Mask::None,
        mask_angle: 0.0,
        level: Some(Level {
            mean: 0.74,
            peak: 0.82,
        }),
    }
}

/// Every shape the console paints **wholly inside** `rect`, on one frame.
///
/// `library.rs`'s helper, and `transport.rs` is where the reasoning is written
/// out: containment rather than intersection, so the panel's ground and the
/// card's drop shadow are not counted as things drawn in the bay.
fn shapes_inside(view: &mut View, panel: &mut Panel, rect: egui::Rect) -> usize {
    let ctx = drawn_once();
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| view.draw(ui, panel));
    out.textures_delta.clear();
    out.shapes
        .into_iter()
        .filter(|clipped| {
            let bounds = clipped.shape.visual_bounding_rect();
            bounds.is_finite() && rect.contains_rect(bounds)
        })
        .count()
}

// ---------------------------------------------------------------------------
// The bay has no body
// ---------------------------------------------------------------------------

/// **The Staging lane draws what a bay with no body draws, and the Sequencer
/// bay is what that is.**
///
/// Not a count of shapes — `egui` is free to tessellate a card differently
/// tomorrow — but two bays held against each other. The Sequencer is the
/// lane's exact twin in the console's own furniture: a title, no pill, **no
/// grip**, and nothing in its body at all. `library.rs` holds the Library
/// against the Master for the same reason and picks a different twin because
/// the Library has a grip and the Master has one; a Staging bay held against
/// the Master would be six grip dots short and the assertion would be about
/// the grip.
///
/// So the two draw the same shapes, or Staging is drawing something a lane
/// with no candidate does not have: a `.cand` row, a placeholder, or the
/// head's `2 waiting` where there is nothing to count.
///
/// Asked at both windows, because a bay that started drawing a body would be
/// most likely to do it at the taller one.
#[test]
fn the_staging_lane_draws_no_body() {
    let mut asked = 0;
    for viewport in [PLAUSIBLE, SMALLEST] {
        let mut panel = console(viewport);
        let staging = to_egui(rect_of(panel.layout(), "staging"));
        let twin = to_egui(rect_of(panel.layout(), "sequencer"));

        let mut view = View::new(Room::Day);
        let lane = shapes_inside(&mut view, &mut panel, staging);
        let bare = shapes_inside(&mut view, &mut panel, twin);
        assert_eq!(
            lane, bare,
            "at {}x{} the Staging bay draws {lane} shapes and the Sequencer bay, which has \
             the same head and no body at all, draws {bare}",
            viewport.w, viewport.h
        );
        // A guard, so this cannot pass by both bays being off the panel and
        // drawing nothing at all: a card and a head is at least two shapes.
        assert!(
            bare >= 2,
            "the Sequencer bay drew {bare} shapes, which is not a card and a head"
        );
        asked += 1;
    }
    assert_eq!(asked, 2, "not every window was asked");
}

/// **Nothing the console is handed reaches the lane.**
///
/// The four things a caller writes onto a `View` are the Library's names, the
/// mixer's strips, the Inspector's panes and the canvas; a fifth, the picture,
/// takes a device and is `None` in every test in this crate. A console with
/// all four filled is the fullest this crate can make one, and the lane is the
/// same bay it was empty — which is what says the row it would draw waits on a
/// producer none of the four is.
#[test]
fn a_full_console_stages_nothing() {
    let mut panel = console(PLAUSIBLE);
    let staging = to_egui(rect_of(panel.layout(), "staging"));

    let mut view = View::new(Room::Day);
    let empty = shapes_inside(&mut view, &mut panel, staging);

    view.library = ["drift_night", "lattice_veil", "glass_shell"]
        .iter()
        .map(|s| (*s).to_owned())
        .collect();
    view.mixer = vec![strip("drift_night"), strip("lattice_veil")];
    view.inspector = vec![
        Pane {
            deck: 0,
            material: "drift_night".to_owned(),
            sync: karakuri_operation::Sync::Free,
            anchor_bpm: 128.0,
            scrub_beats: 0.0,
            composite: false,
            nodes: Vec::new(),
        };
        PANES
    ];
    let full = shapes_inside(&mut view, &mut panel, staging);
    assert_eq!(
        full,
        empty,
        "the console was handed {} names, {} strips and {} panes and the Staging bay went from \
         {empty} shapes to {full}",
        view.library.len(),
        view.mixer.len(),
        view.inspector.len()
    );
}

/// **The lane's head carries no pill**, asked of the plan rather than of the
/// paint pass.
///
/// The mock's Staging head reads `2 waiting` and this one reads nothing, for
/// the reason `view`'s module documentation gives: the number is not a queue
/// depth — a finished build is installed at a frame boundary rather than held
/// for a verdict — so it would be a count of unsettled nodes, and it waits on
/// exactly what the rows wait on. It is stated here as well as in the paint
/// count above because a pill is one shape and a shape count is not where a
/// reader would look for it.
#[test]
fn the_lane_head_counts_nothing() {
    let region = karakuri_console::view::region("staging").expect("the lane is a region");
    let Kind::Bay { title, pills, grip } = region.kind else {
        panic!("the Staging lane is a bay with a head");
    };
    assert_eq!(title, "Staging");
    assert_eq!(pills, &[] as &[&str], "the head is counting something");
    assert!(!grip, "the lane is not the bay in its column that absorbs");
}

// ---------------------------------------------------------------------------
// Nothing in it is a control
// ---------------------------------------------------------------------------

/// **Nothing in the Staging bay takes a press**, stated rather than inferred
/// from the absence of a hit test.
///
/// `library.rs`'s test one bay up, and the inset is its inset for its reason:
/// a boundary is claimed for a drag from `GRAB` either side of it, and that is
/// the panel taking a *divider* rather than anything in the bay.
#[test]
fn nothing_in_the_staging_lane_is_a_control() {
    let mut panel = console(PLAUSIBLE);
    let ctx = drawn_once();
    let region = to_egui(rect_of(panel.layout(), "staging"));
    let strips = Vec::new();

    let inset = karakuri_console::panel::GRAB + 2.0;
    let points = [
        egui::pos2(region.min.x + inset, region.min.y + inset),
        egui::pos2(region.max.x - inset, region.min.y + inset),
        egui::pos2(region.min.x + inset, region.max.y - inset),
        egui::pos2(region.max.x - inset, region.max.y - inset),
        region.center(),
    ];
    let mut asked = 0;
    for p in points {
        assert_eq!(
            claim(&mut panel, &ctx, &strips, Point::new(p.x, p.y)),
            Claim::Egui,
            "the console took the pointer at {p:?}, which is inside the Staging bay"
        );
        asked += 1;
    }
    // A guard, so this cannot pass by testing nothing.
    assert_eq!(asked, points.len(), "not every point was asked");

    // **The negative control**, and it is the reason this test is not the
    // trivially-passing kind [P-0025](../../../docs/principles/0025-a-test-meant-to-catch-something-is-run-against-the-defect.md)
    // is about: `claim` cannot be reached with a defect in the bay, because
    // there is nothing in the bay to break. So the same call is asked at the
    // one point around here it does *not* answer `Egui` for — the boundary the
    // lane shares with the Library above it, which the panel takes for a drag
    // — and a `claim` that answered `Egui` everywhere would fail here.
    let above = region.min.y - karakuri_console::panel::GRAB * 0.5;
    assert_eq!(
        claim(
            &mut panel,
            &ctx,
            &strips,
            Point::new(region.center().x, above)
        ),
        Claim::Panel,
        "the boundary over the Staging bay is not the panel's, so `claim` is answering \
         `Egui` for every point it is asked"
    );
}
