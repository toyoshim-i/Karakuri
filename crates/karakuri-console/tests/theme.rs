//! Tests for the transport row theme selector pill and modal dropdown menu.

mod common;

use common::mock_transport;
use karakuri_console::egui;
use karakuri_console::room::{Room, ThemeMode};
use karakuri_console::view::{theme_pill, ModalOverlay, ThemeAsk, View};
use karakuri_layout::Point;

#[test]
fn theme_mode_cycling_and_resolution() {
    assert_eq!(ThemeMode::Auto.next(), ThemeMode::Day);
    assert_eq!(ThemeMode::Day.next(), ThemeMode::Night);
    assert_eq!(ThemeMode::Night.next(), ThemeMode::Auto);

    assert_eq!(
        ThemeMode::Auto.resolve(Some(egui::Theme::Dark)),
        Room::Night
    );
    assert_eq!(ThemeMode::Auto.resolve(Some(egui::Theme::Light)), Room::Day);
    assert_eq!(ThemeMode::Auto.resolve(None), Room::Day);

    assert_eq!(ThemeMode::Day.resolve(Some(egui::Theme::Dark)), Room::Day);
    assert_eq!(
        ThemeMode::Night.resolve(Some(egui::Theme::Light)),
        Room::Night
    );
}

#[test]
fn theme_pill_geometry_and_placement() {
    let (panel, ctx) = common::console(common::PLAUSIBLE);
    let transport = Some(mock_transport());
    let view = View::new(Room::Day);

    let row = karakuri_console::view::transport(&ctx, panel.layout(), transport).expect("a row");
    let pill = theme_pill(
        &ctx,
        panel.layout(),
        transport,
        view.audio.as_ref(),
        view.tracker,
        view.map.as_ref(),
        &view.arrangement,
        view.theme_mode,
        false,
    )
    .expect("theme pill to be placed");

    assert!(pill.pill.max.x < row.frame.min.x);
    assert!(pill.pill.min.x > row.bar.max.x);
    assert_eq!(pill.mode, ThemeMode::Day);
}

#[test]
fn theme_pill_hit_testing_and_dropdown() {
    let (panel, ctx) = common::console(common::PLAUSIBLE);
    let transport = Some(mock_transport());
    let view = View::new(Room::Day);

    let pill_shut = theme_pill(
        &ctx,
        panel.layout(),
        transport,
        view.audio.as_ref(),
        view.tracker,
        view.map.as_ref(),
        &view.arrangement,
        ThemeMode::Auto,
        false,
    )
    .expect("theme pill");

    let pill_center = Point::new(pill_shut.pill.center().x, pill_shut.pill.center().y);
    assert_eq!(pill_shut.ask(false, pill_center), Some(ThemeAsk::Toggle));

    let outside = Point::new(pill_shut.pill.min.x - 20.0, pill_shut.pill.min.y);
    assert_eq!(pill_shut.ask(false, outside), None);

    let pill_open = theme_pill(
        &ctx,
        panel.layout(),
        transport,
        view.audio.as_ref(),
        view.tracker,
        view.map.as_ref(),
        &view.arrangement,
        ThemeMode::Auto,
        true,
    )
    .expect("theme pill open");

    assert!(pill_open.menu.is_some());
    assert_eq!(pill_open.ask(true, pill_center), Some(ThemeAsk::Toggle));

    for (index, expected_mode) in [ThemeMode::Auto, ThemeMode::Day, ThemeMode::Night]
        .iter()
        .enumerate()
    {
        let row_rect = pill_open.row(index).expect("row rect");
        let row_point = Point::new(row_rect.center().x, row_rect.center().y);
        assert_eq!(
            pill_open.ask(true, row_point),
            Some(ThemeAsk::Select(*expected_mode))
        );
    }

    assert_eq!(pill_open.ask(true, outside), Some(ThemeAsk::Shut));
}

#[test]
fn theme_modal_overlay_lifecycle() {
    let mut view = View::new(Room::Day);
    assert_eq!(view.active_overlay(), None);
    assert!(!view.has_modal_overlay());

    view.theme_menu_open = true;
    assert_eq!(view.active_overlay(), Some(ModalOverlay::ThemeMenu));
    assert!(view.has_modal_overlay());

    let dismissed = view.dismiss_modal_overlay();
    assert!(dismissed);
    assert!(!view.theme_menu_open);
    assert_eq!(view.active_overlay(), None);
    assert!(!view.has_modal_overlay());
}
