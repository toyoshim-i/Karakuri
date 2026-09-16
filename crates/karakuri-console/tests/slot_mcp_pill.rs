//! The slot MCP policy pill in the Inspector pane head:
//! click-to-cycle between Auto, On, and Off.

mod common;

use common::{drawn_once, near, PLAUSIBLE};
use karakuri_console::panel::Panel;
use karakuri_console::room::size;
use karakuri_console::view::{
    inspector, keep_pill, pane_count, slot_mcp_pill, Pane, SlotPolicy, PANES, SYNCS,
};
use karakuri_layout::{Point, Rect};
use karakuri_operation::Sync;

fn mock() -> Pane {
    Pane {
        deck: 0,
        material: "drift_night".to_owned(),
        sync: Sync::Tempo,
        allows: [true; SYNCS.len()],
        anchor_bpm: 128.0,
        scrub_beats: 0.0,
        composite: true,
        aimed: None,
        nodes: Vec::new(),
    }
}

fn console(viewport: Rect) -> (Panel, egui::Context) {
    let mut panel = Panel::new(viewport.w, viewport.h);
    panel.solve();
    (panel, drawn_once())
}

#[test]
fn slot_mcp_pill_is_laid_out_to_the_left_of_keep_pill() {
    let pane = mock();
    let (panel, ctx) = console(PLAUSIBLE);
    for index in 0..PANES {
        let at_pane = inspector(panel.layout(), index, &pane, 0.0).expect("a pane with room in it");
        let keep = keep_pill(&ctx, &at_pane, &pane).expect("keep pill");
        let mcp = slot_mcp_pill(&ctx, &at_pane, &pane, SlotPolicy::Auto).expect("slot mcp pill");

        assert!(
            near(mcp.pill.max.x, keep.pill.min.x - size::PILL_GAP),
            "mcp pill max.x ({}) should be PILL_GAP ({}) left of keep.min.x ({})",
            mcp.pill.max.x,
            size::PILL_GAP,
            keep.pill.min.x
        );
        assert_eq!(mcp.deck, pane.deck);
        assert_eq!(mcp.policy, SlotPolicy::Auto);
    }
}

#[test]
fn slot_mcp_pill_cycles_policies() {
    assert_eq!(SlotPolicy::Auto.next(), SlotPolicy::On);
    assert_eq!(SlotPolicy::On.next(), SlotPolicy::Off);
    assert_eq!(SlotPolicy::Off.next(), SlotPolicy::Auto);

    assert_eq!(SlotPolicy::Auto.pill_word(), "mcp · auto");
    assert_eq!(SlotPolicy::On.pill_word(), "mcp · on");
    assert_eq!(SlotPolicy::Off.pill_word(), "mcp · off");
}

#[test]
fn slot_mcp_pill_hit_tests_accurately() {
    let pane = mock();
    let (panel, ctx) = console(PLAUSIBLE);
    let at_pane = inspector(panel.layout(), 0, &pane, 0.0).expect("pane");
    let mcp = slot_mcp_pill(&ctx, &at_pane, &pane, SlotPolicy::On).expect("mcp pill");

    let center = Point::new(mcp.pill.center().x, mcp.pill.center().y);
    assert!(mcp.hit(center), "center should hit");

    let outside = Point::new(mcp.pill.min.x - 10.0, mcp.pill.min.y - 10.0);
    assert!(!mcp.hit(outside), "outside should not hit");
}

#[test]
fn pane_count_does_not_overlap_slot_mcp_pill() {
    let pane = mock();
    let (panel, ctx) = console(PLAUSIBLE);
    for policy in [SlotPolicy::Auto, SlotPolicy::On, SlotPolicy::Off] {
        for index in 0..PANES {
            let at_pane =
                inspector(panel.layout(), index, &pane, 0.0).expect("a pane with room in it");
            let mcp = slot_mcp_pill(&ctx, &at_pane, &pane, policy).expect("slot mcp pill");
            let count =
                pane_count(&ctx, &at_pane, &pane, None, Some(mcp.pill)).expect("pane count");

            assert!(
                count.max.x <= mcp.pill.min.x - size::HALF_HEAD_GAP,
                "pane_count max.x ({}) must be <= mcp.min.x - HALF_HEAD_GAP ({})",
                count.max.x,
                mcp.pill.min.x - size::HALF_HEAD_GAP
            );
            assert!(
                !count.intersects(mcp.pill),
                "pane_count ({:?}) and slot_mcp_pill ({:?}) must never overlap",
                count,
                mcp.pill
            );
        }
    }
}
