//! Tests for Program bay resolution selector pill and dropdown menu component (ADR-0387).

mod common;

use common::{at, console, PLAUSIBLE};
use karakuri_console::egui;
use karakuri_console::input::{claim, Claim};
use karakuri_console::room::Room;
use karakuri_console::view::{resolution_pill, to_egui, ModalOverlay, ResolutionAsk, View};
use karakuri_layout::Point;

fn sample_resolutions() -> Vec<((u32, u32), String)> {
    vec![
        ((1280, 720), "1280x720 (720p)".into()),
        ((1920, 1080), "1920x1080 (1080p)".into()),
        ((2560, 1440), "2560x1440 (1440p)".into()),
    ]
}

#[test]
fn resolution_pill_measurement_and_guard() {
    let fresh = egui::Context::default();
    let bay_rect = egui::Rect::from_min_size(egui::Pos2::new(10.0, 10.0), egui::vec2(600.0, 400.0));
    let viewport = bay_rect;
    let res = sample_resolutions();

    // Guard: before first Context::run(), fonts are uninitialized, returns None.
    assert!(resolution_pill(&fresh, bay_rect, viewport, &res, 0, false).is_none());
    assert!(resolution_pill(&fresh, bay_rect, viewport, &[], 0, false).is_none());

    let (panel, ctx) = console(PLAUSIBLE);
    let id = panel.layout().find("program").expect("program bay exists");
    let bay = to_egui(panel.layout().rect(id));
    let vp = to_egui(panel.layout().viewport());

    let pill = resolution_pill(&ctx, bay, vp, &res, 0, false).expect("pill measured");
    assert!(pill.pill.width() > 50.0);
    assert!(pill.pill.height() > 15.0);
    assert!(pill.menu.is_none());

    // When open, menu rectangle is calculated within viewport.
    let open_pill = resolution_pill(&ctx, bay, vp, &res, 0, true).expect("open pill measured");
    let menu = open_pill.menu.expect("menu rect exists when open");
    assert!(menu.width() >= 160.0);
    assert!(menu.min.y >= open_pill.pill.max.y);
}

#[test]
fn resolution_pill_hit_testing_and_ask() {
    let (panel, ctx) = console(PLAUSIBLE);
    let id = panel.layout().find("program").expect("program bay exists");
    let bay = to_egui(panel.layout().rect(id));
    let vp = to_egui(panel.layout().viewport());
    let res = sample_resolutions();

    let closed = resolution_pill(&ctx, bay, vp, &res, 0, false).expect("pill measured");
    let pill_center = at(closed.pill.center());
    assert!(closed.hit(pill_center));
    assert_eq!(
        closed.ask(false, res.len(), pill_center),
        Some(ResolutionAsk::Toggle)
    );

    let outside = Point::new(closed.pill.max.x + 50.0, closed.pill.max.y + 50.0);
    assert_eq!(closed.ask(false, res.len(), outside), None);

    let opened = resolution_pill(&ctx, bay, vp, &res, 1, true).expect("open pill measured");
    assert_eq!(
        opened.ask(true, res.len(), pill_center),
        Some(ResolutionAsk::Toggle)
    );

    // Hit test items in open menu
    let row0 = opened.row(0, res.len()).expect("row 0");
    let row1 = opened.row(1, res.len()).expect("row 1");
    let row2 = opened.row(2, res.len()).expect("row 2");

    assert_eq!(
        opened.ask(true, res.len(), at(row0.center())),
        Some(ResolutionAsk::Select(0))
    );
    assert_eq!(
        opened.ask(true, res.len(), at(row1.center())),
        Some(ResolutionAsk::Select(1))
    );
    assert_eq!(
        opened.ask(true, res.len(), at(row2.center())),
        Some(ResolutionAsk::Select(2))
    );

    // Outside click while open emits Shut to dismiss
    assert_eq!(
        opened.ask(true, res.len(), outside),
        Some(ResolutionAsk::Shut)
    );
}

#[test]
fn resolution_claim_integration() {
    let (mut panel, ctx) = console(PLAUSIBLE);
    let mut view = View::new(Room::Day);
    view.output_resolutions = sample_resolutions();
    view.output_resolution_selected = 0;

    let id = panel.layout().find("program").expect("program bay exists");
    let bay = to_egui(panel.layout().rect(id));
    let vp = to_egui(panel.layout().viewport());

    let pill = resolution_pill(&ctx, bay, vp, &view.output_resolutions, 0, false).expect("pill");
    let center = at(pill.pill.center());

    // Clicks on resolution pill claim Panel
    assert_eq!(claim(&mut panel, &ctx, &view, center), Claim::Panel);

    // Opening resolution menu claims Panel for the whole console (Rule 2 modal overlay)
    view.resolution_menu_open = true;
    assert!(view.has_modal_overlay());
    assert_eq!(view.active_overlay(), Some(ModalOverlay::ResolutionMenu));
    assert_eq!(
        claim(&mut panel, &ctx, &view, Point::new(10.0, 10.0)),
        Claim::Panel
    );

    // Dismissal shuts the overlay
    assert!(view.dismiss_modal_overlay());
    assert!(!view.resolution_menu_open);
    assert!(!view.has_modal_overlay());
}
