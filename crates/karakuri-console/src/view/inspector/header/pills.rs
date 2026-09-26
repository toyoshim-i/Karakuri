use super::super::*;

/// The `keep` pill in a pane head, which files a Set under a timestamp per [P-0090], [ADR-0287], and [ADR-0128].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KeepPill {
    /// The capsule, which is what a press has to land in. The mock gives the whole
    /// pill the click and so does this — [`ArrangementPill::pill`]'s own reading.
    pub pill: Rect,
    /// Which deck this pill keeps, as [`Pane::deck`] — carried so that a press
    /// answers with the deck it was measured for, which is [`DeckHead::deck`]'s
    /// reason one row down.
    pub deck: usize,
}

impl KeepPill {
    /// Whether `p` is on the capsule, which is the whole of what this control owns:
    /// there is no menu under it and no second target beside it.
    pub fn hit(&self, p: karakuri_layout::Point) -> bool {
        self.pill.contains(Pos2::new(p.x, p.y))
    }

    /// Returns the [`Operation::SaveSet`] requested by a press at `p`, or `None`.
    pub fn keep(&self, p: karakuri_layout::Point) -> Option<Operation> {
        self.hit(p).then_some(Operation::SaveSet {
            deck: self.deck as u8,
            // Save with id: None to generate timestamp-based name.
            id: None,
        })
    }
}

/// Derives the position of the pane head `keep` pill, or `None` if it does not fit.
pub fn keep_pill(ctx: &egui::Context, at: &InspectorPane, pane: &Pane) -> Option<KeepPill> {
    // Fonts are not valid until `egui` has run a pass, exactly as in
    // [`deck_head`] — and on the frame before the first one there is nothing
    // drawn here to press.
    if ctx.cumulative_pass_nr() == 0 {
        return None;
    }
    let head = at.head;
    let w = pill_width(ctx, KEEP_LABEL);
    let pill = Rect::from_min_size(
        Pos2::new(
            head.max.x - size::HALF_HEAD_PAD_X - w,
            head.min.y + size::HALF_HEAD_PAD_Y,
        ),
        egui::vec2(w, size::PILL_H),
    );
    // Measured against header content box to ensure pill fits between paddings.
    let room = head.width() - size::HALF_HEAD_PAD_X * 2.0;
    (positive(head) && pill.width() <= room).then_some(KeepPill {
        pill,
        deck: pane.deck,
    })
}

/// The slot's MCP policy pill in a pane's head, laid out.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SlotMcpPill {
    pub pill: Rect,
    pub deck: usize,
    pub policy: SlotPolicy,
}

impl SlotMcpPill {
    pub fn hit(&self, p: karakuri_layout::Point) -> bool {
        self.pill.contains(Pos2::new(p.x, p.y))
    }
}

pub fn slot_mcp_pill(
    ctx: &egui::Context,
    at: &InspectorPane,
    pane: &Pane,
    policy: SlotPolicy,
) -> Option<SlotMcpPill> {
    if ctx.cumulative_pass_nr() == 0 {
        return None;
    }
    let head = at.head;
    let right = match keep_pill(ctx, at, pane) {
        Some(keep) => keep.pill.min.x - size::PILL_GAP,
        None => head.max.x - size::HALF_HEAD_PAD_X,
    };
    let w = pill_width(ctx, policy.pill_word());
    let pill = Rect::from_min_size(
        Pos2::new(right - w, head.min.y + size::HALF_HEAD_PAD_Y),
        egui::vec2(w, size::PILL_H),
    );
    let min_left = head.min.x + size::HALF_HEAD_PAD_X;
    (positive(head) && pill.min.x >= min_left).then_some(SlotMcpPill {
        pill,
        deck: pane.deck,
        policy,
    })
}

/// The word in the capsule, which is the mock's own and is the row's name in
/// the panel column of [every
/// operation](../../../../docs/manual/operations.html).
pub(crate) const KEEP_LABEL: &str = "keep";
