//! Tests for ModalOverlay (P43) and hover tooltip suppression (P44).

mod common;

use std::time::Duration;

use karakuri_console::hover::{self, Hover, Tip};
use karakuri_console::input::Claim;
use karakuri_console::panel::Panel;
use karakuri_console::room::Room;
use karakuri_console::view::{
    transport, AddChoice, AudioIn, Mask, ModalOverlay, Pane, Strip, Tally, View, SYNCS,
};
use karakuri_layout::Point;
use karakuri_operation::{BlendMode, Sync};

#[test]
fn modal_overlay_default_is_none() {
    let view = View::new(Room::Day);
    assert_eq!(view.active_overlay(), None);
    assert!(!view.has_modal_overlay());
}

#[test]
fn modal_overlay_arrangement_open_and_dismiss() {
    let mut view = View::new(Room::Day);
    view.arrangement.opened();
    assert_eq!(view.active_overlay(), Some(ModalOverlay::Arrangement));
    assert!(view.has_modal_overlay());

    let dismissed = view.dismiss_modal_overlay();
    assert!(dismissed);
    assert_eq!(view.active_overlay(), None);
    assert!(!view.has_modal_overlay());
}

#[test]
fn modal_overlay_naming_set_open_and_dismiss() {
    let mut view = View::new(Room::Day);
    view.name_set(0);
    assert_eq!(view.active_overlay(), Some(ModalOverlay::NamingSet(0)));
    assert!(view.has_modal_overlay());

    let dismissed = view.dismiss_modal_overlay();
    assert!(dismissed);
    assert_eq!(view.active_overlay(), None);
    assert!(!view.has_modal_overlay());
}

#[test]
fn modal_overlay_suppresses_hover_tooltips() {
    let mut view = View::new(Room::Day);
    let mut panel = Panel::new(1280.0, 720.0);
    panel.solve();
    let ctx = karakuri_console::egui::Context::default();
    let p = Point::new(100.0, 100.0);

    // When an overlay is active, hover::resolve returns None unconditionally
    view.arrangement.opened();
    assert!(view.has_modal_overlay());
    assert_eq!(hover::resolve(&panel, &ctx, &view, p), None);

    let dismissed = view.dismiss_modal_overlay();
    assert!(dismissed);
    assert!(!view.has_modal_overlay());
}

// -- a card that comes down over a tip -------------------------------------

use common::default_console as console;

/// A console every one of the nine cards can come down on: an engine behind the
/// transport, a mixer, a listing, two panes, an audio pill and something for the
/// Master bay's `+ add` to offer.
fn loaded() -> View {
    let mut view = View::new(Room::Day);
    view.transport = Some(common::mock_transport());
    view.mixer = vec![Strip {
        name: "set 0".to_owned(),
        tally: Tally::Live,
        requested: Tally::Live,
        gain: 1.0,
        gain_to: None,
        opacity: 1.0,
        opacity_to: None,
        blend: BlendMode::Over,
        mask: Mask::None,
        mask_angle: 0.25,
        level: None,
        is_muted: false,
        is_soloed: false,
    }];
    view.library = vec!["one".to_owned(), "two".to_owned()];
    view.inspector = vec![Pane {
        deck: 0,
        material: "drift_night".to_owned(),
        sync: Sync::Beat,
        allows: [true; SYNCS.len()],
        anchor_bpm: 128.0,
        scrub_beats: 0.0,
        composite: true,
        aimed: None,
        nodes: Vec::new(),
    }];
    let mut audio = AudioIn::NONE;
    audio.inputs = vec!["Scarlett 2i2".to_owned()];
    view.audio = Some(audio);
    view.chain_add = vec![AddChoice {
        procedure: "bafy".to_owned(),
        words: "bloom".to_owned(),
        retains: false,
    }];
    view
}

/// The middle of the tempo figure — a tipped control, reached the way the window
/// reaches it.
fn on_tempo(panel: &Panel, ctx: &egui::Context, view: &View) -> Point {
    let row =
        transport(ctx, panel.layout(), view.transport).expect("a row with an engine behind it");
    Point::new(row.bpm.center().x, row.bpm.center().y)
}

/// Run one pass and let the layer paint into it — `tests/hover.rs`'s helper.
fn paint(ctx: &egui::Context, hover: &mut Hover, panel: &Panel, view: &View, now: Duration) {
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
        hover.paint(ui, panel, view, now)
    });
    out.textures_delta.clear();
}

/// A tip on screen, and a card put down by a key or by a press that moved the
/// pointer nowhere: the tip goes on the frame the card is drawn on, and the layer
/// asks for no frame of its own afterwards.
///
/// `resolve` alone does not answer this. It is asked on a move, and there is no
/// move here.
#[test]
fn a_card_that_comes_down_over_a_tip_takes_it_off_the_next_frame() {
    let (panel, ctx) = console();
    let mut view = loaded();
    let at = on_tempo(&panel, &ctx, &view);
    let dwell = Hover::dwell(&ctx);
    let mut hover = Hover::new();

    hover.moved(Claim::Panel, &panel, &ctx, &view, at, Duration::ZERO);
    paint(&ctx, &mut hover, &panel, &view, dwell);
    assert!(
        hover.showing().is_some(),
        "the tip is up before the card comes down"
    );

    // The card comes down with no pointer motion behind it.
    view.arrangement.opened();
    assert!(view.has_modal_overlay());
    paint(&ctx, &mut hover, &panel, &view, dwell * 2);
    assert_eq!(
        hover.showing(),
        None,
        "a tip painted over a card that is down"
    );
    assert_eq!(
        hover.owed(&ctx, dwell * 2),
        Tip::Still,
        "the layer asks for a frame for a tip it may not draw"
    );
    assert_eq!(
        hover.resting_on().map(|tip| tip.control),
        None,
        "the layer is still resting on a control under a card"
    );
}

/// A dwell that is running when the card comes down is not owed either, and the
/// tip comes back only once the pointer rests on the control again — exactly as
/// if it had left the window and come back.
#[test]
fn a_suppressed_tip_returns_on_the_next_rest_and_not_on_the_dismissal() {
    let (panel, ctx) = console();
    let mut view = loaded();
    let at = on_tempo(&panel, &ctx, &view);
    let dwell = Hover::dwell(&ctx);
    let mut hover = Hover::new();

    // A dwell running rather than a tip up, which is the other half of the
    // stretch `resolve` never sees.
    hover.moved(Claim::Panel, &panel, &ctx, &view, at, Duration::ZERO);
    assert_eq!(hover.owed(&ctx, Duration::ZERO), Tip::Dwelling(dwell));
    view.arrangement.opened();
    paint(&ctx, &mut hover, &panel, &view, Duration::ZERO);
    assert_eq!(
        hover.owed(&ctx, Duration::ZERO),
        Tip::Still,
        "a dwell still running under a card"
    );

    // Dismissed, and the pointer has not moved: the words do not come back on
    // their own.
    assert!(view.dismiss_modal_overlay());
    paint(&ctx, &mut hover, &panel, &view, dwell * 2);
    assert_eq!(
        hover.showing(),
        None,
        "the tip came back with no dwell behind it"
    );
    assert_eq!(hover.owed(&ctx, dwell * 2), Tip::Still);

    // The pointer rests on the control again, and that is a new dwell.
    assert_eq!(
        hover.moved(Claim::Panel, &panel, &ctx, &view, at, dwell * 2),
        Tip::Still
    );
    assert_eq!(hover.owed(&ctx, dwell * 2), Tip::Dwelling(dwell));
    paint(&ctx, &mut hover, &panel, &view, dwell * 2);
    assert_eq!(hover.showing(), None, "a tip up with no dwell behind it");
    paint(&ctx, &mut hover, &panel, &view, dwell * 3);
    assert!(
        hover.showing().is_some(),
        "the tip did not come back once the card was gone and the pointer rested again"
    );
}

/// One card and the one way in to it: the variant it is expected to answer as,
/// and the call that puts it down.
type Opener = (ModalOverlay, fn(&mut View));

/// Every one of the nine cards suppresses a tip, and `View::has_modal_overlay` is
/// the one derivation of *is a card down*: a variant added to `ModalOverlay` and
/// left out of that answer fails here.
#[test]
fn every_modal_overlay_suppresses_hover_tooltips() {
    let (panel, ctx) = console();
    let at = on_tempo(&panel, &ctx, &loaded());
    let openers: [Opener; 9] = [
        (ModalOverlay::Arrangement, |view| view.arrangement.opened()),
        (ModalOverlay::AudioIn, |view| {
            view.audio.as_mut().expect("an audio pill").opened();
        }),
        (ModalOverlay::NamingSet(0), |view| view.name_set(0)),
        (ModalOverlay::LibraryTargetDeck, |view| {
            assert!(view.open_target(), "the list did not come down");
        }),
        (ModalOverlay::LibraryRowMenu, |view| {
            assert!(view.open_menu(0), "the menu did not come down");
        }),
        (ModalOverlay::InspectorWiring(0, 0, 0), |view| {
            assert!(view.open_wiring(0, 0, 0), "the card did not come down");
        }),
        (ModalOverlay::InspectorPaneTarget(0), |view| {
            assert!(view.open_pane_target(0), "the card did not come down");
        }),
        (ModalOverlay::SequencerLaneChooser, |view| {
            assert!(view.open_lane(), "the chooser did not come down");
        }),
        (ModalOverlay::MasterChainAddChooser, |view| {
            assert!(view.open_chain_add(), "the chooser did not come down");
        }),
    ];

    for (overlay, open) in openers {
        let mut view = loaded();
        assert!(
            hover::resolve(&panel, &ctx, &view, at).is_some(),
            "the tempo figure has no tip to suppress"
        );

        open(&mut view);
        assert_eq!(
            view.active_overlay(),
            Some(overlay),
            "{} is not the card that came down",
            overlay.name()
        );
        assert_eq!(
            hover::resolve(&panel, &ctx, &view, at),
            None,
            "a tip resolved under {}",
            overlay.name()
        );

        assert!(view.dismiss_modal_overlay());
        assert!(
            hover::resolve(&panel, &ctx, &view, at).is_some(),
            "the tip did not come back once {} was gone",
            overlay.name()
        );
    }
}
