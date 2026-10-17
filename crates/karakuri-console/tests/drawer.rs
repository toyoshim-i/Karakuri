//! Tests for narrow-viewport responsive overlay drawers, drawer tabs, and inspector degeneration.

mod common;

use common::{arranged, id_of, SMALLEST};
use karakuri_console::panel::{DrawerSide, Panel};
use karakuri_console::view::rearrange;
use karakuri_console::MINIMUM_VIEWPORT;
use karakuri_layout::{Point, Rect};

const CANVAS: (u32, u32) = (1280, 720);

fn narrow_panel(width: f32) -> Panel {
    arranged(
        Rect {
            w: width,
            ..SMALLEST
        },
        CANVAS,
    )
}

#[test]
fn narrow_viewport_collapses_side_panes_and_exposes_drawer_tabs() {
    let panel = narrow_panel(700.0);
    assert!(panel.is_narrow_body());

    let left = id_of(panel.layout(), "left-pane");
    let right = id_of(panel.layout(), "right-pane");

    // Both panes are set aside in the normal layout below minimum viewport width.
    assert!(panel.layout().is_set_aside(left));
    assert!(panel.layout().is_set_aside(right));
    assert!(!panel.layout().visible(left));
    assert!(!panel.layout().visible(right));

    // Both protruding drawer tabs are present.
    let left_tab = panel
        .drawer_tab_rect(DrawerSide::Left)
        .expect("left tab present");
    let right_tab = panel
        .drawer_tab_rect(DrawerSide::Right)
        .expect("right tab present");

    assert_eq!(left_tab.w, 24.0);
    assert_eq!(left_tab.h, 64.0);
    assert_eq!(right_tab.w, 24.0);
    assert_eq!(right_tab.h, 64.0);

    // Left tab is at left edge, right tab is at right edge of body, both bottom-aligned.
    let body = panel.body_rect().expect("body rect exists");
    assert_eq!(left_tab.x, body.x);
    assert_eq!(right_tab.x, (body.x + body.w - 24.0).max(body.x));
    assert_eq!(left_tab.y, (body.y + body.h - 64.0).max(body.y));
    assert_eq!(right_tab.y, (body.y + body.h - 64.0).max(body.y));

    // Hit-testing drawer tabs.
    let left_center = Point::new(left_tab.x + left_tab.w * 0.5, left_tab.y + left_tab.h * 0.5);
    let right_center = Point::new(
        right_tab.x + right_tab.w * 0.5,
        right_tab.y + right_tab.h * 0.5,
    );
    assert_eq!(panel.drawer_tab_at(left_center), Some(DrawerSide::Left));
    assert_eq!(panel.drawer_tab_at(right_center), Some(DrawerSide::Right));
}

#[test]
fn toggling_left_drawer_solves_overlay_and_shifts_tab() {
    let mut panel = narrow_panel(700.0);
    let left = id_of(panel.layout(), "left-pane");

    // Open left drawer
    let toggled = panel.toggle_drawer(DrawerSide::Left);
    assert_eq!(toggled, Some(DrawerSide::Left));
    assert_eq!(panel.active_drawer(), Some(DrawerSide::Left));

    rearrange(&mut panel, CANVAS);
    assert!(panel.layout().overlay().is_some());
    let (ov_id, ov_rect) = panel.layout().overlay().unwrap();
    assert_eq!(ov_id, left);
    assert!(panel.layout().is_in_overlay(left));
    assert!(panel.layout().visible(left));

    // Tab protrudes from the right edge of the overlay drawer
    let tab = panel
        .drawer_tab_rect(DrawerSide::Left)
        .expect("left tab present when open");
    assert_eq!(tab.x, ov_rect.x + ov_rect.w);

    // Close left drawer
    let toggled = panel.toggle_drawer(DrawerSide::Left);
    assert_eq!(toggled, None);
    assert_eq!(panel.active_drawer(), None);

    rearrange(&mut panel, CANVAS);
    assert!(panel.layout().overlay().is_none());
    assert!(!panel.layout().visible(left));
}

#[test]
fn toggling_right_drawer_solves_overlay_and_shifts_tab() {
    let mut panel = narrow_panel(700.0);
    let right = id_of(panel.layout(), "right-pane");

    // Open right drawer
    let toggled = panel.toggle_drawer(DrawerSide::Right);
    assert_eq!(toggled, Some(DrawerSide::Right));
    assert_eq!(panel.active_drawer(), Some(DrawerSide::Right));

    rearrange(&mut panel, CANVAS);
    assert!(panel.layout().overlay().is_some());
    let (ov_id, ov_rect) = panel.layout().overlay().unwrap();
    assert_eq!(ov_id, right);
    assert!(panel.layout().is_in_overlay(right));
    assert!(panel.layout().visible(right));

    // Tab protrudes to the left of the overlay drawer
    let tab = panel
        .drawer_tab_rect(DrawerSide::Right)
        .expect("right tab present when open");
    assert_eq!(tab.x, ov_rect.x - tab.w);

    // Close right drawer
    panel.set_active_drawer(None);
    rearrange(&mut panel, CANVAS);
    assert!(panel.layout().overlay().is_none());
    assert!(!panel.layout().visible(right));
}

#[test]
fn single_pane_inspector_degenerates_below_threshold() {
    let panel = narrow_panel(430.0);
    let inspector_1 = id_of(panel.layout(), "inspector-1");
    let inspector_2 = id_of(panel.layout(), "inspector-2");

    // Under 430px viewport (e.g. iPhone portrait), inspector width is < 623px threshold,
    // so inspector-2 is set aside and inspector-1 takes the full width.
    assert!(panel.layout().is_set_aside(inspector_2));
    assert!(!panel.layout().visible(inspector_2));
    assert!(panel.layout().visible(inspector_1));

    // In a wide viewport (e.g. 1440px), inspector has plenty of room, so both panes are visible.
    let wide = narrow_panel(1440.0);
    assert!(!wide.layout().is_set_aside(inspector_2));
    assert!(wide.layout().visible(inspector_1));
    assert!(wide.layout().visible(inspector_2));
}

#[test]
fn wide_viewport_clears_active_drawer() {
    let mut panel = narrow_panel(700.0);
    panel.set_active_drawer(Some(DrawerSide::Left));
    rearrange(&mut panel, CANVAS);
    assert!(panel.active_drawer().is_some());

    // Resizing to wide viewport automatically dismisses drawer and restores tiling
    panel.layout_mut().set_viewport(Rect {
        w: MINIMUM_VIEWPORT.0 + 100.0,
        ..SMALLEST
    });
    rearrange(&mut panel, CANVAS);
    assert_eq!(panel.active_drawer(), None);
    assert!(panel.layout().overlay().is_none());
    assert!(!panel.is_narrow_body());
}
