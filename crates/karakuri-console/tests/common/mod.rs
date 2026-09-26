//! Layout assertions for console arrangement across viewports.
//!
//! Mirrors `karakuri-layout` test assertions locally for crate isolation.

#![allow(dead_code)] // Each test file uses a subset.

use karakuri_layout::{Axis, Layout, NodeId, Rect};

/// Pixel coordinates in the hundreds, where a single-precision sum stays well
/// under a thousandth of a pixel out. The same tolerance `karakuri-layout`'s
/// own tests use, for the same reason.
pub const EPS: f32 = 1e-3;

pub fn near(a: f32, b: f32) -> bool {
    (a - b).abs() <= EPS
}

/// A plausible window: a 1920x1080 desktop, which is also the resolution the
/// mock's program header is showing and near the 1900 width the manual's "763
/// pixels tall" is worked out at.
pub const PLAUSIBLE: Rect = Rect {
    x: 0.0,
    y: 0.0,
    w: 1920.0,
    h: 1080.0,
};

/// Viewport matching the mock specification (1244 x 658.5) (ADR-0239, ADR-0272, ADR-0279).
///
/// Width provides 484px center for inspector panes; height covers transport, body, and outputs minima.
pub const SMALLEST: Rect = Rect {
    x: 0.0,
    y: 0.0,
    w: 1244.0,
    h: 658.5,
};

/// Initializes an `egui` context with one initial frame to ensure font metrics are valid.
///
/// Clears unapplied texture deltas to avoid `epaint` drop panics during testing.
pub fn drawn_once() -> egui::Context {
    let ctx = egui::Context::default();
    let mut out = ctx.run_ui(egui::RawInput::default(), |_| {});
    out.textures_delta.clear();
    ctx
}

/// Returns a [`Panel`] arranged at `viewport` for `canvas` via [`karakuri_console::view::rearrange`].
pub fn arranged(viewport: Rect, canvas: (u32, u32)) -> karakuri_console::panel::Panel {
    let mut panel = karakuri_console::panel::Panel::new(viewport.w, viewport.h);
    karakuri_console::view::rearrange(&mut panel, canvas);
    panel
}

/// A solved layout at `viewport`.
pub fn solved(viewport: Rect) -> Layout {
    let mut layout = karakuri_console::layout();
    layout.set_viewport(viewport);
    layout.solve();
    layout
}

/// A solved panel at `viewport`.
pub fn console_panel(viewport: Rect) -> karakuri_console::panel::Panel {
    let mut panel = karakuri_console::panel::Panel::new(viewport.w, viewport.h);
    panel.solve();
    panel
}

/// A solved panel paired with a context that has drawn once at `viewport`.
pub fn console(viewport: Rect) -> (karakuri_console::panel::Panel, egui::Context) {
    (console_panel(viewport), drawn_once())
}

/// A solved panel paired with a context that has drawn once at `PLAUSIBLE`.
pub fn default_console() -> (karakuri_console::panel::Panel, egui::Context) {
    console(PLAUSIBLE)
}

/// Convert an `egui::Pos2` into a `karakuri_layout::Point`.
pub fn point(p: egui::Pos2) -> karakuri_layout::Point {
    karakuri_layout::Point::new(p.x, p.y)
}

/// Alias for `point` for tests that name it `at`.
pub fn at(p: egui::Pos2) -> karakuri_layout::Point {
    point(p)
}

/// Every name in the arrangement, in arena order.
pub fn names(l: &Layout) -> Vec<String> {
    let mut out = Vec::new();
    walk(l, l.root(), &mut |id| {
        if let Some(name) = l.name(id) {
            out.push(name.to_owned());
        }
    });
    out
}

/// Every rectangle in the arrangement, in arena order — what "the same
/// arrangement" is compared as.
pub fn rects(l: &Layout) -> Vec<Rect> {
    let mut out = Vec::new();
    walk(l, l.root(), &mut |id| out.push(l.rect(id)));
    out
}

pub fn walk(l: &Layout, id: NodeId, f: &mut impl FnMut(NodeId)) {
    f(id);
    for c in l.children(id) {
        walk(l, *c, f);
    }
}

/// The rectangle of a named region. Panics with the name rather than unwrapping
/// a `None`, because a missing region is the failure worth reading.
pub fn rect_of(l: &Layout, name: &str) -> Rect {
    l.rect(id_of(l, name))
}

/// The id of a named region.
pub fn id_of(l: &Layout, name: &str) -> NodeId {
    l.find(name)
        .unwrap_or_else(|| panic!("the arrangement has no region named {name}"))
}

/// Asserts layout validity: non-negative bounds, viewport containment, and exact tiling without overlaps.
pub fn assert_sane(l: &Layout) {
    check(l, l.root());
}

fn check(l: &Layout, id: NodeId) {
    let r = l.rect(id);
    assert!(
        r.w >= 0.0 && r.h >= 0.0,
        "negative rectangle {r:?} at {:?}",
        l.name(id)
    );

    if l.visible(id) {
        let v = l.viewport();
        assert!(
            r.x >= v.x - EPS
                && r.y >= v.y - EPS
                && r.x + r.w <= v.x + v.w + EPS
                && r.y + r.h <= v.y + v.h + EPS,
            "{:?} at {r:?} escapes the viewport {v:?}",
            l.name(id)
        );
    }

    if let Some(axis) = l.axis(id) {
        assert_tiles(l, id, axis);
        for c in l.children(id) {
            check(l, *c);
        }
    }
}

fn assert_tiles(l: &Layout, id: NodeId, axis: Axis) {
    let parent = l.rect(id);
    let divider = l.divider(id).unwrap();
    let (origin, extent) = along(axis, parent);

    let mut cursor = origin;
    let mut visible = 0;
    let mut gaps = 0;

    for c in l.children(id) {
        let r = l.rect(*c);
        let (o, e) = along(axis, r);

        let (po, pe) = across(axis, parent);
        let (co, ce) = across(axis, r);
        assert!(
            near(po, co) && near(pe, ce),
            "{:?} does not span its parent across the axis: {r:?} in {parent:?}",
            l.name(*c)
        );

        if !laid_out(l, *c) {
            assert!(
                near(e, 0.0),
                "{:?} is out of the layout and took {e}",
                l.name(*c)
            );
        }

        // Closed regions take no extent but preserve their tiling divider edge (ADR-0300).
        if !l.is_placed(*c) {
            continue;
        }

        if visible > 0 {
            assert!(
                near(o - cursor, divider),
                "the gap before {:?} is {} and not the divider {divider}",
                l.name(*c),
                o - cursor
            );
            gaps += 1;
        } else {
            assert!(
                near(o, origin),
                "the first visible child of {:?} starts at {o}, not {origin}",
                l.name(id)
            );
        }
        visible += 1;
        cursor = o + e;
    }

    if visible == 0 {
        return;
    }
    assert_eq!(
        gaps,
        visible - 1,
        "wrong number of dividers in {:?}",
        l.name(id)
    );
    assert!(
        near(cursor, origin + extent),
        "the children of {:?} end at {cursor} rather than {}",
        l.name(id),
        origin + extent
    );
}

/// Assert every visible region is within the `[min, max]` it declares, along
/// the axis it declares them in.
pub fn assert_within_bounds(l: &Layout) {
    bounds(l, l.root());
}

fn bounds(l: &Layout, id: NodeId) {
    let Some(axis) = l.axis(id) else {
        return;
    };
    for c in l.children(id) {
        if laid_out(l, *c) {
            let (lo, hi) = l.bounds(*c);
            let (_, e) = along(axis, l.rect(*c));
            assert!(
                e >= lo - EPS,
                "{:?} solved to {e}, under its minimum of {lo}",
                l.name(*c)
            );
            assert!(
                e <= hi + EPS,
                "{:?} solved to {e}, over its maximum of {hi}",
                l.name(*c)
            );
        }
        bounds(l, *c);
    }
}

/// Computes minimum extent required along `axis` from child minima and dividers.
pub fn implied_min(l: &Layout, id: NodeId, axis: Axis) -> f32 {
    match l.axis(id) {
        None => 0.0,
        Some(inner) if inner == axis => {
            let kids = l.children(id);
            let sum: f32 = kids
                .iter()
                .map(|c| l.bounds(*c).0.max(implied_min(l, *c, axis)))
                .sum();
            sum + l.divider(id).unwrap() * (kids.len() as f32 - 1.0)
        }
        Some(_) => l
            .children(id)
            .iter()
            .map(|c| implied_min(l, *c, axis))
            .fold(0.0, f32::max),
    }
}

/// Checks if a node is actively participating in layout rather than set aside or folded (ADR-0183).
pub fn laid_out(l: &Layout, id: NodeId) -> bool {
    !l.is_collapsed(id) && !l.is_set_aside(id)
}

fn along(axis: Axis, r: Rect) -> (f32, f32) {
    match axis {
        Axis::Row => (r.x, r.w),
        Axis::Column => (r.y, r.h),
    }
}

fn across(axis: Axis, r: Rect) -> (f32, f32) {
    match axis {
        Axis::Row => (r.y, r.h),
        Axis::Column => (r.x, r.w),
    }
}

/// Constructs a [`View`] containing given mixer strips with default arrangement for hit-testing.
pub fn showing(strips: &[karakuri_console::view::Strip]) -> karakuri_console::view::View {
    let mut view = karakuri_console::view::View::new(karakuri_console::room::Room::Day);
    view.mixer = strips.to_vec();
    view
}

/// Constructs a test console with mock engine transport state (128 BPM, bar 37 beat 0).
pub fn running() -> karakuri_console::view::View {
    let mut view = karakuri_console::view::View::new(karakuri_console::room::Room::Day);
    view.transport = Some(mock_transport());
    view
}

/// Mock transport constants: 128.0 BPM, bar 37 beat 0 (144 beats), 58 fps.
pub fn mock_transport() -> karakuri_console::view::Transport {
    karakuri_console::view::Transport {
        bpm: 128.0,
        beats: 144.0,
        beats_per_bar: 4,
        fps: Some(58.0),
        frame_ms: 12.4,
        budget_ms: Some(16.6),
        // The chain's term, which the mock's transport row does not draw: the
        // mock's chain is empty and a chain that costs nothing draws nothing.
        chain_ms: None,
        // The landed health capsule at the end of the row, rendered as armed.
        health: Some(karakuri_console::view::Stage::Landed),
        rec: None,
    }
}
