use super::*;

/// Determines if the deck displayed in a pane is currently on air based on mixer strips.
pub(crate) fn on_air(strips: &[Strip], deck: usize) -> bool {
    strips
        .get(deck)
        .is_some_and(|strip| residency(strip.tally) == Residency::Live)
}

/// Rendering context for an inspector pane within a layout frame.
#[derive(Clone, Copy)]
pub(crate) struct InspectorIntoCtx<'a> {
    pub at: &'a InspectorPane,
    pub pane: &'a Pane,
    pub on_air: bool,
    pub policy: SlotPolicy,
    pub naming: Option<&'a str>,
    pub target: Option<PaneTarget>,
}

/// Type alias for [`InspectorIntoCtx`] conforming to ADR-0210 naming standards.
#[allow(dead_code)]
pub(crate) type InspectorRenderCtx<'a> = InspectorIntoCtx<'a>;

/// Everything is clipped to the pane, which is what makes the overflow
/// safe: a group that fits and a name that does not are the same clip, and it
/// is the same `with_clip_rect` the picture, a preview cell and the library's
/// list are each drawn inside.
pub(crate) fn inspector_into(ui: &Ui, pal: &Palette, ctx: InspectorIntoCtx<'_>) {
    let InspectorIntoCtx {
        at,
        pane,
        on_air,
        policy,
        naming,
        target,
    } = ctx;
    // Hit-test and paint keep pill before words to preserve spacing in narrow panes.
    let keep = keep_pill(ui.ctx(), at, pane);
    let mcp = slot_mcp_pill(ui.ctx(), at, pane, policy);
    // Visible parameter count readout (P-0082, Rule 4).
    let count = pane_count(
        ui.ctx(),
        at,
        pane,
        naming,
        mcp.as_ref().map(|pill| pill.pill),
    );
    // Remaining width in the half-head for the deck name and material, clipped if too long.
    let words = match count
        .map(|c| c.min.x)
        .or(mcp.map(|pill| pill.pill.min.x))
        .or(keep.map(|pill| pill.pill.min.x))
    {
        Some(x) => Rect::from_min_max(
            at.head.min,
            Pos2::new(x - size::HALF_HEAD_GAP, at.head.max.y),
        ),
        None => at.head,
    };
    let painter = ui.painter().with_clip_rect(words);
    let label = painter.layout_job(span_at(head_label(naming), size::BASE, pal.faint));
    let y = at.head.center().y - label.size().y * 0.5;
    painter.galley(
        Pos2::new(at.head.min.x + size::HALF_HEAD_PAD_X, y),
        label,
        pal.faint,
    );
    // Editable deck name text field.
    if let Some(named) = deck_name(
        ui.ctx(),
        at,
        pane,
        naming,
        mcp.as_ref().map(|pill| pill.pill),
    ) {
        // Tint the deck name field while being actively edited to highlight text entry.
        let text = match naming {
            Some(typed) => naming_text_in_head(pane, typed),
            None => showing_text(pane),
        };
        editable_text_field(&painter, pal, named.name, &text, naming.is_some(), 3);
    }
    // `.half-head`'s own `border-bottom`, the bottom pixel of the row — drawn
    // through the whole head rather than through the words' clip, which stops
    // one gap short of the pill.
    let painter = ui.painter().with_clip_rect(at.head);
    let rule = at.head.max.y - size::HAIRLINE * 0.5;
    painter.line_segment(
        [
            Pos2::new(at.head.min.x, rule),
            Pos2::new(at.head.max.x, rule),
        ],
        Stroke::new(size::HAIRLINE, pal.hair),
    );
    // Draw pulldown chevron indicator in faint ink next to the target name.
    if let Some(target) = target {
        let mark = target.chevron;
        chevron_down(&painter, mark, pal.faint);
    }
    // Paint the keep pill, lit with tally pink if the deck is currently on air.
    if let Some(pill) = keep {
        match on_air {
            true => on_pill_at(ui, pal, pill.pill, KEEP_LABEL),
            false => pill_at(ui, pal, pill.pill, KEEP_LABEL),
        }
    }
    if let Some(pill) = mcp {
        match policy {
            SlotPolicy::On => armed_pill_at(ui, pal, pill.pill, pill.policy.pill_word()),
            SlotPolicy::Off => pill_at(ui, pal, pill.pill, pill.policy.pill_word()),
            SlotPolicy::Auto => {
                if on_air {
                    pill_at(ui, pal, pill.pill, pill.policy.pill_word());
                } else {
                    armed_pill_at(ui, pal, pill.pill, pill.policy.pill_word());
                }
            }
        }
    }
    // Paint visible parameter count in faint ink.
    if let Some(rect) = count {
        let galley = painter.layout_job(span_at(&count_text(at, pane), size::BASE, pal.faint));
        painter.galley(
            Pos2::new(rect.min.x, rect.center().y - galley.size().y * 0.5),
            galley,
            pal.faint,
        );
    }

    // Render deck controls subheader if space permits.
    if let Some(head) = deck_head(ui.ctx(), at, pane) {
        deck_head_into(ui, pal, &head, pane);
    }

    // Clip body contents so scrolled node groups do not bleed into header areas.
    let painter = ui.painter().with_clip_rect(at.body);
    for index in at.drawn(&pane.nodes) {
        let node = &pane.nodes[index];
        let rect = at.group(&pane.nodes, index);
        node_into(&painter, pal, rect, node);
        // Draw hairline border between adjacent node groups (omitted on the last node).
        if index + 1 < pane.nodes.len() {
            let rule = rect.max.y + size::HAIRLINE * 0.5;
            painter.line_segment(
                [Pos2::new(rect.min.x, rule), Pos2::new(rect.max.x, rule)],
                Stroke::new(size::HAIRLINE, pal.hair),
            );
        }
    }
}
