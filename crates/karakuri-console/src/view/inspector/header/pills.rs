use super::super::super::*;
use super::super::*;

/// The `keep` pill in a pane's head, laid out — the capsule at the right of
/// `.half-head`, and the one control in this bay that performs rather than
/// sets.
///
/// # It keeps the pane's deck, and `k` keeps the selection
///
/// The mock draws one of these per pane and the tooltip names the pane's own
/// deck: *"Keep deck A as a Set, exactly as it is on screen."* So this carries
/// [`Pane::deck`] the way [`DeckHead::deck`] does, and a press answers with the
/// deck the pill was measured for — a pill in the second pane keeps that pane's
/// deck while the selection stays where the operator put it. The key `k` keeps
/// *the selected deck*, because a bare key press cannot say which, and the two
/// are one operation asked for from two ends
/// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
///
/// # What it files it under
///
/// [`Operation::SaveSet`] with no id, which is the same call the key makes and
/// is a decision rather than an omission
/// ([ADR-0287](../../../../docs/adr/0287-the-keep-pill-files-under-a-stamp-because-the-consoles-one-letter-taking-flow-is-an-arrangements-name.md)).
/// What the store does with a `None` is `karakuri_environment::accepted_save`'s
/// convention — a stamp, because *"an operator looks for the time they saved
/// it"*.
///
/// The reason has changed and the decision has not. ADR-0287 argued the `None`
/// from there being one letter-taking flow on this console and it being an
/// arrangement's; there are two now, and the second is the name in the head
/// beside this capsule
/// ([ADR-0292](../../../../docs/adr/0292-the-pane-heads-name-takes-letters-and-the-keep-capsule-stays-a-stamp.md)).
/// What holds the capsule at `None` from here on is
/// [ADR-0128](../../../../docs/adr/0128-a-set-saved-under-a-name-the-caller-chose-overwrites.md)
/// rather than the absence of a field: *"an operator's own act gets the name it
/// asked for; a key press cannot type one and takes a stamp"*. This is the
/// press that types nothing, so this is the one that takes the stamp — see
/// [`DeckName`] for the one that does not.
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

    /// What a press at `p` asks for, or `None` off the capsule.
    ///
    /// The same derivation [`crate::input::claim`] hit-tests, asked a second time
    /// rather than copied — [`DeckHead::sync`]'s arrangement, and the reason is the
    /// same one row up: the pill that claims a press and the pill that acts on it
    /// cannot come apart.
    ///
    /// It refuses nothing. What a keep costs and whether the store will take it are
    /// the instrument's answers rather than this surface's, and the operation is
    /// *"on a worker"* on the page it is specified on — the press leaves and the
    /// answer arrives later.
    pub fn keep(&self, p: karakuri_layout::Point) -> Option<Operation> {
        self.hit(p).then_some(Operation::SaveSet {
            deck: self.deck as u8,
            // **`None`, and it is the payload saying so rather than this
            // control inventing a stamp** — the sentence
            // `crates/karakuri/src/main.rs` already writes over the `k` arm,
            // and the same one: *"a caller that can type a name is not made to
            // take a timestamp"*, and this control is not one of them.
            id: None,
        })
    }
}

/// The pane head's pill, derived — [`inspector`] answers where the head is and
/// this answers where the capsule in it is, which is [`deck_head`]'s division
/// one row down.
///
/// `.sep`'s `flex: 1` puts it hard against the head's right-hand padding, and
/// one padding down from the top rather than centred in the row: the rule at
/// the bottom is inside `.half-head`, so the row's middle is half a pixel below
/// the middle of its content box — which is the scope row's own note one bay
/// along, on a row built the same way.
///
/// # What it costs to ask
///
/// One galley lookup per pane, for the word in the capsule, on a pointer event
/// and on a frame — [`deck_head`]'s three beside it, and paid the same way.
///
/// # `None` is a head that cannot hold it
///
/// [`deck_head`]'s rule and [`look`]'s: *a control that does not fit in the row
/// it is drawn in is no control at all, rather than half of one*. The words to
/// its left are a readout and are clipped; the pill is a target and is not
/// drawn where it would be cut.
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
    // **Measured against `.half-head`'s content box and not against the row**,
    // because a flex item cannot be laid out inside its parent's padding: the
    // capsule is placed from the right-hand padding, so what it runs off is
    // the left one, and a head with less room between its two paddings than
    // the word needs draws none. [`positive`] is what says the head is a row
    // at all — a pane with no height has one that is not.
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
