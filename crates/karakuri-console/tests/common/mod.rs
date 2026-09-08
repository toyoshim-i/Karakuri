//! What has to be true of the console's arrangement at any viewport, and the
//! two viewports the tests use.
//!
//! `karakuri-layout` has a checker very like this one in its own
//! `tests/common/mod.rs`. It is a test module rather than part of the crate,
//! so it cannot be reached from here and this is written again — see the
//! report; it is the one duplication in this crate.

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
/// mock's program header is showing and near the 1900 width the manual's
/// "763 pixels tall" is worked out at.
pub const PLAUSIBLE: Rect = Rect {
    x: 0.0,
    y: 0.0,
    w: 1920.0,
    h: 1080.0,
};

/// The smallest window this arrangement is claimed to work at, and every digit
/// of it is derived rather than picked.
///
/// **1244 wide** is the width that gives the centre the **484** the mock's own
/// narrowest console gives it, at the tracks this panel has: `.console`'s
/// `min-width: 1010px` less its 10px of padding either side is 990, and the
/// mock's `.body-grid` spends that on `218px minmax(340px, 1fr) 268px` with
/// two 10px gaps — so 484 in the middle. The panel's outer tracks are 340 and
/// 400 (ADR-0239) and the mock's stylesheet still carries 218 and 268, so the
/// same 484 costs 340 + 400 + 20 more: **1244**. The 484 is what is being
/// held, and what it buys is the inspector: each pane is 237.5, and the
/// `.param` grid (`15px 88px 1fr 58px`, 8px gaps, 22 of padding — 207 before
/// the fader has any width) fits in one. **`min-width: 1010px` is the stale
/// number of the two** — read against this panel's tracks it gives a centre
/// of 230 — and it is the mock's own, so it is not this file's to move.
///
/// **658.5 high** is the sum of the column-axis minima: the transport's 48,
/// the body row's 556.5 — the row of three columns — the outputs row's 34 and
/// the two 10px dividers. The body row's 556.5 is the **centre's**: the
/// Program bay at the 395 the mock draws it, the divider's 10 and the
/// inspector's 151.5. It was the right pane's 530 — a mixer that cannot lose a
/// strip (316), a master chain of one effect (94), a sequencer of one lane
/// (100) and two dividers — until the manual gave the inspector a deck head,
/// and it grew again by the 17 the preview captions added to the Program bay.
/// `arrangement.rs` recomputes both numbers from the tree, so this is a claim
/// rather than a copy.
///
/// Not smaller, because below either figure the panel is no longer the panel
/// this console describes. The two figures fail differently and it is worth
/// keeping them apart: **658.5 is where the solve stops honouring minima** and
/// scales everything down together — which is the right behaviour and is not a
/// panel anyone can work on — while 1244 is a claim about the *content*, and
/// the solve goes on honouring every minimum down to
/// [`karakuri_console::MINIMUM_VIEWPORT`]'s 777. That constant is the window's
/// floor and this is the console's claim; ADR-0272 is why they are two numbers
/// and ADR-0279 is why the floor is 777 rather than the 692 it was written at:
/// the centre's declared minimum stopped being `.body-grid`'s CSS track and
/// became the width two inspector panes need to draw a parameter fader.
pub const SMALLEST: Rect = Rect {
    x: 0.0,
    y: 0.0,
    w: 1244.0,
    h: 658.5,
};

/// **An `egui` context that has drawn once.**
///
/// What laying out the Outputs row takes, and it is a real requirement rather
/// than a test's ceremony: the chip's width is the width of the name in it, so
/// asking where the control is means asking `egui` to lay that name out, and
/// `Context::fonts` is *"not valid until first call to `Context::run()`"*. A
/// window loop has drawn thousands of frames before a hand arrives; a test has
/// to say so in one line.
///
/// The texture delta is cleared because `epaint` panics if one is dropped
/// unapplied — there is no renderer here to apply it to, which is the whole of
/// what makes this a test and not a window.
pub fn drawn_once() -> egui::Context {
    let ctx = egui::Context::default();
    let mut out = ctx.run_ui(egui::RawInput::default(), |_| {});
    out.textures_delta.clear();
    ctx
}

/// **A panel at `viewport` with the Program bay arranged for `canvas`** —
/// which is what a frame does before it reads a single rectangle
/// ([`karakuri_console::view::rearrange`]), and therefore what every rectangle
/// on the console is read from.
///
/// A [`Panel`] rather than a [`Layout`] because the rearrangement writes a bit
/// that only `Panel` may write — see `Panel::set_aside`. `panel.layout()` is
/// what the readers here take.
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

/// The rectangle of a named region. Panics with the name rather than
/// unwrapping a `None`, because a missing region is the failure worth reading.
pub fn rect_of(l: &Layout, name: &str) -> Rect {
    l.rect(id_of(l, name))
}

/// The id of a named region.
pub fn id_of(l: &Layout, name: &str) -> NodeId {
    l.find(name)
        .unwrap_or_else(|| panic!("the arrangement has no region named {name}"))
}

/// Assert everything that must be true of any solve, at any viewport:
/// no rectangle is negative, every visible one is inside the viewport, and a
/// split's children tile it exactly — no overlap, the gaps are the divider and
/// sit only between visible children.
///
/// The one permitted shortfall is the solve's own: where every visible child
/// is already at its maximum, what is left is trailing space. This
/// arrangement has no maximum that can be reached with room to spare, so this
/// asserts the stricter thing and would notice one appearing.
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

/// The extent a subtree needs along `axis` before the solve stops honouring
/// what it declares: a split laid out along that axis needs its children's
/// minima plus the dividers between them, and one laid out across it needs the
/// widest of theirs.
///
/// A node's own `min` is stated along its **parent's** axis, so it is read by
/// the parent that arranges it and never by the node itself — which is why
/// this takes the larger of what a child declares and what its own contents
/// imply, rather than trusting either alone.
///
/// **The model does not compute this.** A split's minimum is a number it is
/// given, not a function of its children's, which is why the arrangement
/// writes the body row's out by hand and why this exists to check it.
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

/// Whether a child takes extent, a divider of its own and the bounds it
/// declares — which is the question every geometric assertion here is about,
/// and it is **not** whether the operator folded it.
///
/// A node the console has set aside is out of the layout by the other bit
/// ([ADR-0183](../../../../docs/adr/0183-a-node-is-out-of-the-layout-for-two-reasons-and-they-are-two-bits.md))
/// and is out of it just as completely, so `is_collapsed` here would count a
/// phantom divider before a region that is not there and would hold a region
/// with no extent to its 72px minimum. `karakuri-layout`'s own checker has the
/// same function for the same reason; this is the duplication this file's
/// header already names.
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

/// **A view with these strips in it and nothing else** — the argument
/// `karakuri_console::input::claim` takes, built where a test used to hand it a
/// slice.
///
/// The rule hit-tests six controls now and two of them read values a caller
/// wrote: a fader's knob sits on the fill's moving edge, and the arrangement
/// pill is as wide as the name in it. `claim` takes the whole `View` so that
/// what it hit-tests is what `View::draw` painted, and this is that view for a
/// console with no store behind it — the default arrangement, nothing filed
/// and the menu shut. Pass `&[]` for one with no deck behind it either, which
/// is every test here that is not about the mixer.
pub fn showing(strips: &[karakuri_console::view::Strip]) -> karakuri_console::view::View {
    let mut view = karakuri_console::view::View::new(karakuri_console::room::Room::Day);
    view.mixer = strips.to_vec();
    view
}

/// **A console with an engine behind it**: the mock's transport, and nothing
/// else written to.
///
/// `docs/manual/console.html`'s `.transport` — `128.0 BPM`, the first beat of
/// bar 37 lit, and `58 fps · 12.4/16.6 ms`. Bar 37 beat 0 is 36 whole bars of
/// four beats, which is the 144.
///
/// **It is here rather than in one test file because the arrangement pill
/// needs it to exist at all.** The pill sits one `.transport` gap after
/// `bar 37`, so `view::arrangement` answers `None` for a console with no
/// engine behind it — the row draws nothing there, and a control in a row that
/// is not drawn is not a control. Any test that presses the pill therefore has
/// to hand in a transport, and `tests/vocabulary.rs` is one that has nothing
/// else to do with a tempo.
///
/// The reading itself is [`mock_transport`], which is what `tests/transport.rs`
/// and `tests/arrangement_pill.rs` ask for: three files wanting one console's
/// tempo is one console's tempo, written once
/// (`docs/contributing.md` §4).
pub fn running() -> karakuri_console::view::View {
    let mut view = karakuri_console::view::View::new(karakuri_console::room::Room::Day);
    view.transport = Some(mock_transport());
    view
}

/// **The mock's own transport, as numbers**, and the one copy of them.
///
/// `docs/manual/console.html`'s `.transport`: `128.0 BPM`, the first beat of
/// bar 37 lit, and `58 fps · 12.4/16.6 ms`. **Bar 37 beat 0 is 36 whole bars of
/// four beats — 144 — and that is the one figure here written rather than
/// derived**, from the mock's own `bar 37`.
pub fn mock_transport() -> karakuri_console::view::Transport {
    karakuri_console::view::Transport {
        bpm: 128.0,
        beats: 144.0,
        beats_per_bar: 4,
        fps: Some(58.0),
        frame_ms: 12.4,
        budget_ms: Some(16.6),
        // **The `landed` capsule at the end of the row**, which the mock
        // draws `armed`: the last procedure written is on screen.
        health: Some(karakuri_console::view::Stage::Landed),
    }
}
