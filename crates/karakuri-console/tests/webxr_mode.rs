//! Tests for WebXR presentation mode pill and dropdown menu interaction in Outputs row (ADR-0387).

mod common;

use common::{at, console, near, rect_of, PLAUSIBLE};
use karakuri_console::room::size;
use karakuri_console::view::{outputs_with_plugin_name, ModalOverlay, View, XrAsk, XrSessionMode};
use karakuri_operation::gate::Open;

/// WebXR mode pill appears beside WebXR plugin sink chip and supports dropdown interaction.
#[test]
fn webxr_mode_pill_and_dropdown_lifecycle() {
    let (panel, ctx) = console(PLAUSIBLE);
    let region = rect_of(panel.layout(), "outputs");
    let box_of = egui::Rect::from_min_size(
        egui::pos2(region.x, region.y),
        egui::vec2(region.w, region.h),
    );

    // 1. When plugin is not WebXR, no xr_pill is produced
    let default_row = outputs_with_plugin_name(
        &ctx,
        panel.layout(),
        Open::CLOSED,
        true,
        None,
        XrSessionMode::Vr,
        false,
    )
    .expect("outputs row");
    assert_eq!(default_row.xr_pill, None);

    // 2. When plugin is WebXR and available, xr_pill is placed between WebXR (more[1]) and NDI (more[2])
    let row = outputs_with_plugin_name(
        &ctx,
        panel.layout(),
        Open::CLOSED,
        true,
        Some("WebXR"),
        XrSessionMode::Vr,
        false,
    )
    .expect("outputs row with WebXR");

    let xr_pill = row.xr_pill.expect("xr_pill is present for WebXR plugin");
    assert_eq!(xr_pill.mode, XrSessionMode::Vr);
    assert!(box_of.contains_rect(xr_pill.pill));
    assert!(near(
        xr_pill.pill.min.x,
        row.more[1].chip.max.x + size::OUTPUTS_GAP
    ));
    assert!(near(
        row.more[2].chip.min.x,
        xr_pill.pill.max.x + size::OUTPUTS_GAP
    ));

    // 3. Hit testing: clicking pill toggles menu open
    let pill_center = at(xr_pill.pill.center());
    assert_eq!(xr_pill.ask(false, pill_center), Some(XrAsk::Toggle));

    // 4. When open, menu displays VR and MR options
    let open_row = outputs_with_plugin_name(
        &ctx,
        panel.layout(),
        Open::CLOSED,
        true,
        Some("WebXR"),
        XrSessionMode::Vr,
        true,
    )
    .expect("outputs row with open menu");
    let open_pill = open_row.xr_pill.expect("open xr_pill");
    assert!(open_pill.menu.is_some());

    // Selecting MR (row index 1)
    let mr_row_center = at(open_pill.row(1).expect("MR row").center());
    assert_eq!(
        open_pill.ask(true, mr_row_center),
        Some(XrAsk::Select(XrSessionMode::Mr))
    );

    // Clicking outside shuts menu
    let outside = at(egui::pos2(region.x + 5.0, region.y + 5.0));
    assert_eq!(open_pill.ask(true, outside), Some(XrAsk::Shut));

    // 5. ModalOverlay integration and dismissal
    let mut view = View::new(karakuri_console::room::Room::Day);
    view.xr_mode = XrSessionMode::Vr;
    view.xr_menu_open = true;
    assert_eq!(view.active_overlay(), Some(ModalOverlay::XrModeMenu));
    assert!(view.dismiss_modal_overlay());
    assert!(!view.xr_menu_open);
    assert_eq!(view.active_overlay(), None);
}
