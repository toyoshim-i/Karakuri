use super::super::*;
use super::pills::*;

/// The letter of the deck a pane is pointed at, or `?` for a pane pointed past
/// the end of [`DECK_LETTERS`] — which is a caller's error and not a state, and
/// is drawn rather than panicked for [`showing_text`]'s reason: a head is a
/// readout and a readout does not stop a frame.
fn deck_letter(pane: &Pane) -> &'static str {
    DECK_LETTERS.get(pane.deck).copied().unwrap_or("?")
}

/// What the pane head reads: the mock's `deck A · drift_night`.
pub(crate) fn showing_text(pane: &Pane) -> String {
    format!("deck {} · {}", deck_letter(pane), pane.material)
}

/// Run text while taking letters for a deck (`deck A · glass_sh▏`).
pub(crate) fn naming_text_in_head(pane: &Pane, typed: &str) -> String {
    format!("deck {} · {typed}{CARET}", deck_letter(pane))
}

/// The word the mock puts in front of it.
const SHOWING_LABEL: &str = "showing";

/// Prefix label (`keep`) displayed while renaming a deck Set in the pane head.
const NAMING_LABEL: &str = "keep as";

/// The word this head has in front of its run, which is the one thing about the
/// row that says whether it is reading or asking.
pub(crate) fn head_label(naming: Option<&str>) -> &'static str {
    match naming {
        Some(_) => NAMING_LABEL,
        None => SHOWING_LABEL,
    }
}

/// Laid-out deck name run in a pane head, supporting renaming gestures per [ADR-0292](../../../../docs/adr/0292-the-pane-heads-name-takes-letters-and-the-keep-capsule-stays-a-stamp.md) and ADR-0128.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DeckName {
    /// The run as it is painted, clipped to what the head has room for — see
    /// [`deck_name`]. A press has to land in this and nowhere else.
    pub name: Rect,
    /// Where the mock's `▾` goes, one `.half-head` gap after the run. Drawn by
    /// nobody and claimed by nobody: it is the chooser's place, held so that this
    /// control's edge is a measured thing rather than a comment.
    pub chevron: Rect,
    /// Which deck this head names, as [`Pane::deck`] — carried for
    /// [`KeepPill::deck`]'s reason one capsule along.
    pub deck: usize,
}

impl DeckName {
    /// Whether `p` is on the run, which is the whole of what this control owns: the
    /// label to its left is a readout, and the rectangle to its right is the
    /// chooser's.
    pub fn hit(&self, p: karakuri_layout::Point) -> bool {
        self.name.contains(Pos2::new(p.x, p.y))
    }
}

/// Count readout (`n of m` node groups) displayed in a pane head per [ADR-0259](../../../../docs/adr/0259-the-keyboard-is-addressed-to-the-bay-that-has-focus-and-a-global-letter-is-a-convenience-or-the-operators-own.md).
pub fn count_text(at: &InspectorPane, pane: &Pane) -> String {
    format!("{} of {}", at.shown, pane.nodes.len())
}

/// Derives the position of the pane head group count readout, or `None` if insufficient room per [ADR-0279](../../../../docs/adr/0279-the-centre-is-two-parameter-rows-wide-because-a-pane-that-cannot-draw-a-fader-is-not-a-minimum.md).
pub fn pane_count(
    ctx: &egui::Context,
    at: &InspectorPane,
    pane: &Pane,
    naming: Option<&str>,
    mcp: Option<Rect>,
) -> Option<Rect> {
    // Fonts are not valid until `egui` has run a pass — [`keep_pill`]'s guard,
    // and before the first one there is no head painted to read.
    if ctx.cumulative_pass_nr() == 0 {
        return None;
    }
    let head = at.head;
    if !positive(head) {
        return None;
    }
    let run = |text: &str| {
        ctx.fonts_mut(|f| {
            f.layout_no_wrap(
                text.to_owned(),
                FontId::new(size::BASE, FontFamily::Proportional),
                Color32::PLACEHOLDER,
            )
            .size()
            .x
        })
    };
    let right = match mcp {
        Some(mcp_rect) => mcp_rect.min.x - size::HALF_HEAD_GAP,
        None => match keep_pill(ctx, at, pane) {
            Some(pill) => pill.pill.min.x - size::HALF_HEAD_GAP,
            None => head.max.x - size::HALF_HEAD_PAD_X,
        },
    };
    // Measure against run start so narrow heads prioritize deck name over count.
    let left = head.min.x + size::HALF_HEAD_PAD_X + run(head_label(naming)) + size::HALF_HEAD_GAP;
    let w = run(&count_text(at, pane));
    let top = head.min.y + size::HALF_HEAD_PAD_Y;
    (right - w >= left).then(|| {
        Rect::from_min_max(
            Pos2::new(right - w, top),
            Pos2::new(right, top + size::PILL_H),
        )
    })
}

/// Layout derivation for the deck name text run in the pane head.
///
/// Clips the text run before trailing controls (`▾` chevron, MCP pill, count, and keep capsule).
/// Returns `None` if the head has no positive size or if the name is clipped completely.
pub fn deck_name(
    ctx: &egui::Context,
    at: &InspectorPane,
    pane: &Pane,
    naming: Option<&str>,
    mcp: Option<Rect>,
) -> Option<DeckName> {
    // Fonts are not valid until `egui` has run a pass, exactly as in
    // [`keep_pill`] — and on the frame before the first one there is nothing
    // drawn here to press.
    if ctx.cumulative_pass_nr() == 0 {
        return None;
    }
    let head = at.head;
    if !positive(head) {
        return None;
    }
    let run = |text: &str| {
        ctx.fonts_mut(|f| {
            f.layout_no_wrap(
                text.to_owned(),
                FontId::new(size::BASE, FontFamily::Proportional),
                Color32::PLACEHOLDER,
            )
            .size()
            .x
        })
    };
    // Determine the right bound based on trailing head elements.
    let limit = match pane_count(ctx, at, pane, naming, mcp) {
        Some(count) => count.min.x - size::HALF_HEAD_GAP,
        None => match mcp {
            Some(pill) => pill.min.x - size::HALF_HEAD_GAP,
            None => match keep_pill(ctx, at, pane) {
                Some(pill) => pill.pill.min.x - size::HALF_HEAD_GAP,
                None => head.max.x,
            },
        },
    };
    // Reserve space for the chevron chooser (ADR-0292).
    let limit = limit - (CHEVRON_W + size::HALF_HEAD_GAP);
    let left = head.min.x + size::HALF_HEAD_PAD_X + run(head_label(naming)) + size::HALF_HEAD_GAP;
    let text = match naming {
        Some(typed) => naming_text_in_head(pane, typed),
        None => showing_text(pane),
    };
    let right = (left + run(&text)).min(limit);
    if right <= left {
        return None;
    }
    let top = head.min.y + size::HALF_HEAD_PAD_Y;
    let name = Rect::from_min_max(Pos2::new(left, top), Pos2::new(right, top + size::PILL_H));
    Some(DeckName {
        chevron: Rect::from_min_size(
            Pos2::new(
                name.max.x + size::HALF_HEAD_GAP,
                name.center().y - CHEVRON_H * 0.5,
            ),
            egui::vec2(CHEVRON_W, CHEVRON_H),
        ),
        name,
        deck: pane.deck,
    })
}

/// State for text input when renaming a deck in a pane head per [P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Naming {
    /// Which pane head the field is in, as an index into [`View::inspector`] and so
    /// into [`PANE_NAMES`].
    pub pane: usize,
    pub(crate) typed: String,
}

impl Naming {
    /// What has been typed so far. The caret is drawn after it and there is no
    /// selection: this is a name, not a document — [`Arrangement::naming`]'s own
    /// sentence.
    pub fn typed(&self) -> &str {
        &self.typed
    }
}
