//! Tests for ModalOverlay (P43) and hover tooltip suppression (P44).

use karakuri_console::hover;
use karakuri_console::panel::Panel;
use karakuri_console::room::Room;
use karakuri_console::view::{ModalOverlay, View};
use karakuri_layout::Point;

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
