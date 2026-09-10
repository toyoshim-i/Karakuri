//! **The `keep` capsule in an Inspector pane's head: the one control this bay
//! has that performs rather than sets.**
//!
//! Six things, and the first two are why this is its own file rather than a
//! few more assertions in `deck_head.rs`:
//!
//! 1. Where the capsule sits, as `.half-head`'s flex row lays it out — hard
//!    against the right-hand padding, one padding down from the top rather
//!    than centred in a row whose rule is inside it.
//! 2. **That it clears every boundary's grab**, which is the deck head's
//!    arithmetic one row up, measured here against the pane divider and the
//!    bay's own edges.
//! 3. That a press asks to keep **this pane's** deck — not the selection,
//!    which is what the key `k` keeps.
//! 4. That it files under no name, which is
//!    [ADR-0287](../../../docs/adr/0287-the-keep-pill-files-under-a-stamp-because-the-consoles-one-letter-taking-flow-is-an-arrangements-name.md).
//! 5. That a head with no room for the capsule draws none rather than half of
//!    one — `deck_head`'s rule, on a control that is one capsule.
//! 6. That a press off the capsule asks for nothing at all.
//! 7. **That the wash says the deck this pane is showing is on air**, which
//!    is what `docs/manual/console.html` now says the mock's lit capsule is
//!    reading, and is asserted off the paint pass rather than off a flag.
//!
//! None of it needs a window, a device or a disk. It does need `egui`'s fonts,
//! because a capsule is as wide as the word in it — see `common::drawn_once`.
//!
//! **What is not here and cannot be**: that `input::claim` gives the panel a
//! press on this capsule. That is a row in `input::PROBES` and it is the
//! registration half of this control, which lives in files this test's author
//! does not own; until it lands a press here reaches `egui`.

mod common;

use common::{drawn_once, near, rect_of, PLAUSIBLE, SMALLEST};
use karakuri_console::panel::{Panel, GRAB};
use karakuri_console::room::size;

/// The word in the capsule, which `view::KEEP_LABEL` is not public as.
const KEEP_WORD: &str = "keep";
use karakuri_console::room::Room;
use karakuri_console::view::{
    inspector, keep_pill, InspectorPane, Mask, Pane, Strip, Tally, View, PANES, PANE_NAMES, SYNCS,
};
use karakuri_layout::{Point, Rect};
use karakuri_operation::{BlendMode, Operation, Sync};

/// **The mock's own deck A**, which is the pane the mock draws this capsule
/// lit in. What the wash reads is on the page now — the deck this pane is
/// showing is on air — and the last test in this file is where that is held.
fn mock() -> Pane {
    Pane {
        deck: 0,
        material: "drift_night".to_owned(),
        sync: Sync::Tempo,
        allows: [true; SYNCS.len()],
        anchor_bpm: 128.0,
        scrub_beats: 0.0,
        composite: true,
        // Not this test's row: the deck head's two build chips are
        // drawn from this and nothing here is about them.
        aimed: None,
        nodes: Vec::new(),
    }
}

/// That pane pointed at another deck, which is the only thing two of these
/// tests vary.
fn showing(deck: usize) -> Pane {
    Pane { deck, ..mock() }
}

/// A panel at a viewport, solved, with a context that has drawn once.
fn console(viewport: Rect) -> (Panel, egui::Context) {
    let mut panel = Panel::new(viewport.w, viewport.h);
    panel.solve();
    (panel, drawn_once())
}

/// A `karakuri_layout` point, from `egui`'s.
fn at(p: egui::Pos2) -> Point {
    Point::new(p.x, p.y)
}

/// **How wide the word in the capsule is**, asked of the same fonts the paint
/// asks. The formula is `.pill`'s own — `padding: 0 8px` around text at
/// `BASE` — and it is written here rather than imported so that a change to
/// `pill_width` has to be made twice before it can pass unnoticed.
fn word(ctx: &egui::Context) -> f32 {
    ctx.fonts_mut(|f| {
        f.layout_no_wrap(
            "keep".to_owned(),
            egui::FontId::new(size::BASE, egui::FontFamily::Proportional),
            egui::Color32::PLACEHOLDER,
        )
        .size()
        .x
    })
}

// ---------------------------------------------------------------------------
// Where the control is
// ---------------------------------------------------------------------------

/// **`.sep`'s `flex: 1` puts it hard against the head's right-hand padding**,
/// and `align-items: center` puts it in the middle of the head's *content*
/// box — which is one `.half-head` padding down from the top, not the row's
/// own middle: the `border-bottom` is inside the row, so the two differ by
/// half a pixel and the scope row one bay along already says which is right.
#[test]
fn the_capsule_is_hard_against_the_heads_right_hand_padding() {
    let pane = mock();
    for viewport in [SMALLEST, PLAUSIBLE] {
        let (panel, ctx) = console(viewport);
        for index in 0..PANES {
            let at_pane =
                inspector(panel.layout(), index, &pane, 0.0).expect("a pane with room in it");
            let pill = keep_pill(&ctx, &at_pane, &pane).expect("a head with room for its capsule");
            let head = at_pane.head;

            assert!(
                near(pill.pill.max.x, head.max.x - size::HALF_HEAD_PAD_X),
                "the capsule ends {} from the right of the head and `.half-head`'s padding is {}",
                head.max.x - pill.pill.max.x,
                size::HALF_HEAD_PAD_X
            );
            assert!(
                near(pill.pill.min.y, head.min.y + size::HALF_HEAD_PAD_Y),
                "the capsule is {} down from the top of the head and `.half-head`'s padding is {}",
                pill.pill.min.y - head.min.y,
                size::HALF_HEAD_PAD_Y
            );
            assert!(
                near(pill.pill.height(), size::PILL_H),
                "the capsule is {} tall and `.pill` is `BASE` at `LINE`, which is {}",
                pill.pill.height(),
                size::PILL_H
            );
            assert!(
                near(pill.pill.width(), word(&ctx) + size::PILL_PAD_X * 2.0),
                "the capsule is {} wide and the word in it is {} inside `padding: 0 8px`",
                pill.pill.width(),
                word(&ctx)
            );
            assert!(
                head.contains_rect(pill.pill),
                "the capsule is not inside the head it is drawn in"
            );
            assert_eq!(pill.deck, pane.deck);
        }
    }
}

/// **A head with no room for the capsule draws none**, which is `deck_head`'s
/// rule one row down: a control that does not fit in the row it is drawn in is
/// no control at all, rather than half of one.
///
/// The narrow head is built here rather than solved for, because the panel's
/// own minimum is far wider than this — the same reason `deck_head.rs` builds
/// its narrow row by hand.
#[test]
fn a_head_too_narrow_for_the_capsule_draws_none() {
    let pane = mock();
    let (panel, ctx) = console(PLAUSIBLE);
    let at_pane = inspector(panel.layout(), 0, &pane, 0.0).expect("a pane with room in it");
    let full = at_pane.head;
    // Exactly wide enough for the capsule inside its two paddings, and one
    // pixel narrower than that.
    let width = size::HALF_HEAD_PAD_X * 2.0 + word(&ctx) + size::PILL_PAD_X * 2.0;
    let narrowed = |w: f32| InspectorPane {
        head: egui::Rect::from_min_size(full.min, egui::vec2(w, full.height())),
        ..at_pane
    };
    assert!(
        keep_pill(&ctx, &narrowed(width), &pane).is_some(),
        "a head exactly wide enough for the capsule drew none, so this test cannot tell a fit \
         from a refusal"
    );
    assert_eq!(
        keep_pill(&ctx, &narrowed(width - 1.0), &pane),
        None,
        "a head one pixel too narrow drew the capsule anyway, and `inspector_into`'s clip is what \
         would cut it in half"
    );
}

/// **Before the first frame there is nothing here to press.** `Context::fonts`
/// is not valid until a pass has run, and a capsule is as wide as the word in
/// it — the same guard `deck_head` and every other measured control carry.
#[test]
fn a_console_that_has_not_drawn_has_no_capsule() {
    let pane = mock();
    let (panel, _) = console(PLAUSIBLE);
    let at_pane = inspector(panel.layout(), 0, &pane, 0.0).expect("a pane with room in it");
    assert_eq!(keep_pill(&egui::Context::default(), &at_pane, &pane), None);
}

// ---------------------------------------------------------------------------
// The claim rule
// ---------------------------------------------------------------------------

/// **The capsule clears every boundary's grab**, measured off its own
/// rectangle: the nearest boundary is the pane divider, and what it has to
/// clear sideways is `.half-head`'s right-hand padding of 10 against a `GRAB`
/// of 6. Down the head it is the bay head above it and the whole of the pane
/// below.
#[test]
fn the_capsule_clears_every_boundarys_grab() {
    let pane = mock();
    for viewport in [SMALLEST, PLAUSIBLE] {
        let (panel, ctx) = console(viewport);
        for (index, name) in PANE_NAMES.iter().enumerate() {
            let at_pane =
                inspector(panel.layout(), index, &pane, 0.0).expect("a pane with room in it");
            let pill = keep_pill(&ctx, &at_pane, &pane).expect("a head with room for its capsule");

            let clearance = at_pane.head.max.x - pill.pill.max.x;
            assert!(
                clearance > GRAB,
                "the capsule is {clearance} in from the pane's right edge and a boundary grabs \
                 {GRAB} — the control is inside a boundary's grab, and `input`'s rule is what \
                 has to change"
            );
            let bay = rect_of(panel.layout(), name);
            let above = pill.pill.min.y - bay.y;
            assert!(
                near(above, size::HEAD_H + size::HALF_HEAD_PAD_Y),
                "the capsule is {above} down from the pane's top edge, and the bay head plus \
                 this row's padding come to {}",
                size::HEAD_H + size::HALF_HEAD_PAD_Y
            );
            assert!(above > GRAB, "the capsule has {above} of pane above it");

            for probe in [
                pill.pill.left_top(),
                pill.pill.right_top(),
                pill.pill.left_bottom(),
                pill.pill.right_bottom(),
                pill.pill.center(),
            ] {
                assert!(
                    !matches!(
                        panel.layout().hit(at(probe), GRAB),
                        karakuri_layout::Hit::Divider { .. }
                    ),
                    "a boundary grabs {probe:?}, which is on the keep capsule of pane {index}"
                );
                assert!(
                    pill.hit(at(probe)),
                    "{probe:?} is a corner of the capsule and the capsule says it is not on it"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// What a press asks for
// ---------------------------------------------------------------------------

/// **A press keeps the deck this pane is showing, and files it under no
/// name** — the same call the key `k` makes, and the deck the pill was
/// measured for rather than the one the selection is on (ADR-0287).
#[test]
fn a_press_keeps_this_panes_deck_under_no_name() {
    let (panel, ctx) = console(PLAUSIBLE);
    for deck in 0..4 {
        let pane = showing(deck);
        let at_pane = inspector(panel.layout(), 0, &pane, 0.0).expect("a pane with room in it");
        let pill = keep_pill(&ctx, &at_pane, &pane).expect("a head with room for its capsule");
        assert_eq!(
            pill.keep(at(pill.pill.center())),
            Some(Operation::SaveSet {
                deck: deck as u8,
                id: None
            }),
            "the capsule in a pane showing deck {deck} kept another deck, or typed a name"
        );
    }
}

/// **Two panes keep two decks**, which is the whole of what a per-pane control
/// is: the key `k` keeps the selection because a bare press cannot say which,
/// and a capsule drawn inside a pane can.
#[test]
fn two_panes_keep_the_two_decks_they_are_showing() {
    let (panel, ctx) = console(PLAUSIBLE);
    let panes = [showing(0), showing(2)];
    let kept: Vec<Operation> = panes
        .iter()
        .enumerate()
        .map(|(index, pane)| {
            let at_pane =
                inspector(panel.layout(), index, pane, 0.0).expect("a pane with room in it");
            let pill = keep_pill(&ctx, &at_pane, pane).expect("a head with room for its capsule");
            pill.keep(at(pill.pill.center()))
                .expect("a press on the capsule")
        })
        .collect();
    assert_eq!(
        kept,
        vec![
            Operation::SaveSet { deck: 0, id: None },
            Operation::SaveSet { deck: 2, id: None }
        ]
    );
}

/// **A press off the capsule asks for nothing**, which is *a control claims
/// what it acts on and no more* — the words to the left of it are a readout,
/// and the head around it is not a target.
#[test]
fn a_press_off_the_capsule_asks_for_nothing() {
    let pane = mock();
    let (panel, ctx) = console(PLAUSIBLE);
    let at_pane = inspector(panel.layout(), 0, &pane, 0.0).expect("a pane with room in it");
    let pill = keep_pill(&ctx, &at_pane, &pane).expect("a head with room for its capsule");
    let head = at_pane.head;
    for probe in [
        // The words, at the left of the head.
        egui::Pos2::new(head.min.x + size::HALF_HEAD_PAD_X, head.center().y),
        // The gap between them and the capsule.
        egui::Pos2::new(pill.pill.min.x - size::HALF_HEAD_GAP * 0.5, head.center().y),
        // The padding to the right of it.
        egui::Pos2::new(head.max.x - size::HALF_HEAD_PAD_X * 0.5, head.center().y),
        // The deck head under it.
        at_pane.deck_head.center(),
    ] {
        assert!(
            !pill.hit(at(probe)),
            "the capsule claims {probe:?}, which is not on it"
        );
        assert_eq!(pill.keep(at(probe)), None);
    }
}

// ---------------------------------------------------------------------------
// The wash
// ---------------------------------------------------------------------------

/// Every shape the console paints **wholly inside** `rect`, on one frame.
///
/// `transport.rs`'s helper, written again here for the reason that one gives:
/// `egui` tessellates on the CPU and the device only ever sees the result, so
/// a whole frame through `Context::run_ui` is all a treatment takes to read.
/// Containment rather than intersection, so the card behind the pane is not
/// counted as a thing drawn in the capsule.
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

/// A strip whose tally is `residency`, which is the Mixer bay's reading of a
/// deck and the one this capsule's wash is taken from.
fn strip(residency: Tally) -> Strip {
    Strip {
        name: "drift_night".to_owned(),
        tally: residency,
        requested: residency,
        gain: 0.44,
        gain_to: None,
        opacity: 0.30,
        opacity_to: None,
        blend: BlendMode::Over,
        mask: Mask::Linear,
        mask_angle: 0.0,
        level: None,
    }
}

/// A console showing two panes over a deck whose two slots have the tallies
/// given, and nothing else running.
fn showing_two(tallies: [Tally; 2]) -> View {
    let mut view = View::new(Room::Day);
    view.mixer = tallies.iter().map(|t| strip(*t)).collect();
    view.inspector = vec![showing(0), showing(1)];
    view
}

/// **The wash is the deck's residency and not the pane's position**, which is
/// what `docs/manual/console.html` says it reads: *the deck this pane is
/// showing is on air, in the same pink a tally on air is drawn in*.
///
/// Asserted by painting, because the treatment is the whole of the claim: a
/// `.pill.on` is `--c-pink` over a pink wash with a halo, and a plain `.pill`
/// has neither. Counted as *is any shape in the capsule filled `--c-pink`* —
/// the word's galley is pink in one treatment and `--c-dim` in the other, and
/// the wash behind it is a pink mix in one and nothing in the other.
///
/// **Both directions**, and the second is the one that would catch a console
/// lighting the first pane because it is first: deck 1 live and deck 0 parked
/// puts the wash on the *second* capsule.
#[test]
fn the_wash_follows_the_deck_on_air_and_not_the_pane() {
    let pal = Room::Day.palette();
    for (tallies, want) in [
        ([Tally::Live, Tally::Allocated], [true, false]),
        ([Tally::Allocated, Tally::Live], [false, true]),
        ([Tally::Live, Tally::Live], [true, true]),
        ([Tally::Allocated, Tally::Allocated], [false, false]),
    ] {
        for (index, want) in want.iter().enumerate() {
            let mut panel = Panel::new(PLAUSIBLE.w, PLAUSIBLE.h);
            panel.solve();
            let ctx = drawn_once();
            let pane = showing(index);
            let at = inspector(panel.layout(), index, &pane, 0.0)
                .unwrap_or_else(|| panic!("pane {index} is drawn at a plausible window"));
            let pill = keep_pill(&ctx, &at, &pane)
                .unwrap_or_else(|| panic!("pane {index} draws its capsule"))
                .pill;
            let mut view = showing_two(tallies);
            let mut panel = Panel::new(PLAUSIBLE.w, PLAUSIBLE.h);
            panel.solve();
            let lit = shapes_inside(&mut view, &mut panel, pill)
                .iter()
                .any(|shape| match shape {
                    // **The word, and not the wash.** `on_pill_at`'s halo is
                    // blurred wider than the capsule and would be filtered out
                    // by `contains_rect`, and its ground is a `--c-pink` mix
                    // rather than the colour itself. The word is `--c-pink` in
                    // one treatment and `--c-dim` in the other, it is inside
                    // the capsule by construction, and it is the one thing a
                    // reader actually sees change.
                    egui::Shape::Text(text) => {
                        text.galley.job.text == KEEP_WORD && text.fallback_color == pal.pink
                    }
                    _ => false,
                });
            assert_eq!(
                lit,
                *want,
                "with tallies {tallies:?}, pane {index}'s keep capsule is drawn {} and the page \
                 says the wash is the deck's residency",
                match lit {
                    true => "lit",
                    false => "plain",
                }
            );
        }
    }
}
