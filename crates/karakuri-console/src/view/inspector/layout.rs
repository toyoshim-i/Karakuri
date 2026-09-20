use super::*;

/// How tall everything in a pane comes to: [`group_h`] over every node, with a
/// [`size::HAIRLINE`] between two of them.
///
/// One function because two callers must agree. [`pane_box`] clamps the
/// position in force against it and [`View::scroll_by`] clamps the stored one,
/// and the same sum written twice is two answers to how far a pane scrolls.
pub(crate) fn content_h(nodes: &[Node]) -> f32 {
    let mut total = 0.0;
    for (index, node) in nodes.iter().enumerate() {
        if index > 0 {
            total += size::HAIRLINE;
        }
        total += group_h(node);
    }
    total
}

/// One pane of the Inspector, derived — see [`InspectorPane`] for what is drawn
/// here and for the nine things in the mock's pane that are not.
///
/// `index` is which pane, into [`PANE_NAMES`]. `scroll` is the position that
/// pane is scrolled to — [`View::scroll_in`], the console's own state and not a
/// reading — and it is taken here rather than applied by the painter, because a
/// control is hit-tested off the derivation that draws it and an offset added
/// on one side of that seam and not the other is two answers about where a knob
/// is. It arrives unclamped and leaves clamped: [`InspectorPane::scroll`] is
/// what is in force, and nothing is written back (P-0082). `layout` must be
/// solved: [`Layout::rect`](karakuri_layout::Layout::rect) refuses to answer
/// from a dirty one. Like [`library`] this asks `egui` for nothing: every box
/// in the pane is either the full width of the pane or a track of the mock's
/// own grid, so no rectangle here is the width of the type in it.
///
/// `None` where there is no pane by that name, and `None` where there is no
/// room for the two heads — which is [`picture_rect`]'s rule stated on a pane.
/// A console with no deck behind it hands over no panes at all, and what the
/// bay draws then is its card, its head and the bar between the two panes,
/// exactly as it did before this pass — [`mixer`]'s rule, one bay along.
///
/// The bay head is taken off the top here and not in [`pane_box`], which is
/// what [`mixer::strips_row`] and [`library::library_box`] do one bay along:
/// the head is painted *over* the region rather than laid out beside it, so
/// every body in this file starts at `region.min.y + size::HEAD_H` and the
/// arithmetic under it is written as if the head were not there. A pane is the
/// one body in the arrangement whose region is not the bay's own —
/// `inspector-1` is a child of the split — and that is what hid this: the pane
/// is the full height of the bay, head included, so a `.half-head` drawn at
/// `region.min` lands on top of the word `Inspector`.
pub fn inspector(
    layout: &karakuri_layout::Layout,
    index: usize,
    pane: &Pane,
    scroll: f32,
) -> Option<InspectorPane> {
    let region = to_egui(layout.rect(layout.find(PANE_NAMES.get(index)?)?));
    let under_head = Rect::from_min_max(
        Pos2::new(region.min.x, region.min.y + size::HEAD_H),
        region.max,
    );
    pane_box(under_head, &pane.nodes, scroll)
}

/// The arithmetic of a pane, away from the layout it reads.
///
/// Term for term from `style.css`:
///
/// - `.half-head { padding: 5px 10px; border-bottom: 1px solid var(--c-hair) }`
///   — a [`size::HALF_HEAD_H`] row along the top of the pane, its rule the
///   bottom pixel of it.
/// - `.deck-head { padding: 5px 10px }` — a [`size::DECK_HEAD_H`] row under
///   it, with no rule of its own: *"Its box is `.node-head`'s without the
///   tint"*, and the tint is what separates it from the group below.
/// - what is left is the node groups', stacked from the top.
///
/// The leftover is the last group's and not the pane's, which is the
/// opposite of what [`library::library_box`] does with its foot, and the reason is the
/// same read the other way: the mock's pane is a flow with nothing under the
/// groups at all, so there is no row for a leftover to sit under. It shows as
/// the bay's own card below the last group, which is every other empty body in
/// this pass.
pub(crate) fn pane_box(region: Rect, nodes: &[Node], scroll: f32) -> Option<InspectorPane> {
    let head = Rect::from_min_max(
        region.min,
        Pos2::new(region.max.x, region.min.y + size::HALF_HEAD_H),
    );
    let deck_head = Rect::from_min_max(
        Pos2::new(region.min.x, head.max.y),
        Pos2::new(region.max.x, head.max.y + size::DECK_HEAD_H),
    );
    let body = Rect::from_min_max(Pos2::new(region.min.x, deck_head.max.y), region.max);
    // **Narrower than a parameter row's own padding is no pane**, which is
    // [`library::library_box`]'s width check with the mock's own indent in it. There is
    // no matching check down the pane: a pane too short for a group draws its
    // two heads and no group, which is what `shown` answers.
    if body.width() <= size::PARAM_PAD_L + size::PARAM_PAD_R {
        return None;
    }
    // **And a pane too short for its two heads is no pane**, which is what
    // this function's caller promises. `positive` is not enough on its own:
    // the two heads are stated heights, so they stay positive while running
    // off the bottom of a region shorter than their sum.
    if !positive(head) || !positive(deck_head) || deck_head.max.y > region.max.y {
        return None;
    }
    // How tall the whole stack is, from the one function `View::scroll_by`
    // clamps the stored position against as well.
    let content = content_h(nodes);
    // **The clamp is here and the store is not touched.** `max(0.0)` is what
    // a pane taller than its content answers — there is nothing to scroll
    // through, so the position in force is the top whatever an operator once
    // spun the wheel to, and the position they spun to is still where they
    // left it when the pane comes back (P-0082, ADR-0250).
    let scroll = scroll.clamp(0.0, (content - body.height()).max(0.0));
    // **How many are whole**, which is the readout's number and not the walk's
    // — `InspectorPane::drawn` is the walk. A group is whole when both its
    // edges are inside the body: the top one after the scroll has been taken
    // off, and the bottom one before the body's own.
    let mut shown = 0;
    let mut top = -scroll;
    for (index, node) in nodes.iter().enumerate() {
        let rule = match index {
            0 => 0.0,
            _ => size::HAIRLINE,
        };
        top += rule;
        let bottom = top + group_h(node);
        if top >= -EPSILON && bottom <= body.height() + EPSILON {
            shown += 1;
        }
        top = bottom;
    }
    Some(InspectorPane {
        head,
        deck_head,
        body,
        scroll,
        content,
        shown,
    })
}

/// What counts as touching an edge, for [`pane_box`]'s *is this group whole* —
/// a hair either way, because both sides of that comparison are sums of `f32`
/// constants and a group that exactly fills the body would otherwise be counted
/// or not by the last bit of a float.
const EPSILON: f32 = 0.001;
