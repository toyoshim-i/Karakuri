use super::*;

/// Total height of all groups in a pane, with hairpins between them.
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

/// Derives layout for Inspector pane `index`, clamping scroll offset per [P-0082].
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

/// Lays out pane components (half-head, deck-head, and content area) within `rect`.
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
    // Clamps scroll offset without modifying stored state per P-0082 and ADR-0250.
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
