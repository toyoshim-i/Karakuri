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

/// What that same run reads while the head is taking letters — `deck A ·
/// glass_sh▏`, with [`CARET`] after it as the arrangement's field has.
///
/// The deck stays and the material goes. What is being typed is the name this
/// deck's material will be filed under, so the run says which deck is being
/// filed for the whole of the gesture — and the half of it that is replaced is
/// exactly the half a name is. A field that had cleared the run would take the
/// one word that says *whose* name this is off the screen at the moment an
/// operator is looking hardest at it.
pub(crate) fn naming_text_in_head(pane: &Pane, typed: &str) -> String {
    format!("deck {} · {typed}{CARET}", deck_letter(pane))
}

/// The word the mock puts in front of it.
const SHOWING_LABEL: &str = "showing";

/// The word in front of the run while the head is taking letters, where
/// [`SHOWING_LABEL`] is the word in front of it the rest of the time.
///
/// The row stops being a readout the moment letters are going into it, and the
/// label is the only thing that can say what they are *for*: they name the Set
/// the capsule at the other end of the same row files. It is [`KEEP_LABEL`]'s
/// own word rather than a new one, which is [`SAVE_ITEM_ASKING`]'s arrangement
/// three bays along — the thing that asks for something says so in the verb it
/// is about to perform.
const NAMING_LABEL: &str = "keep as";

/// The word this head has in front of its run, which is the one thing about the
/// row that says whether it is reading or asking.
pub(crate) fn head_label(naming: Option<&str>) -> &'static str {
    match naming {
        Some(_) => NAMING_LABEL,
        None => SHOWING_LABEL,
    }
}

/// The name in a pane head, laid out — the mock's `.what`, and this console's
/// second letter-taking flow
/// ([ADR-0292](../../../../docs/adr/0292-the-pane-heads-name-takes-letters-and-the-keep-capsule-stays-a-stamp.md)).
///
/// # A press on it names the Set, and the capsule beside it goes on stamping
///
/// [ADR-0128](../../../../docs/adr/0128-a-set-saved-under-a-name-the-caller-chose-overwrites.md)
/// is what puts two routes on one row: *"an operator's own act gets the name it
/// asked for; a key press cannot type one and takes a stamp"*. The `keep`
/// capsule is the second of those and is unchanged — [`KeepPill::keep`] emits
/// `id: None` exactly as ADR-0287 decided — and this is the first: a press here
/// puts the head into [`Naming`], and the commit is [`View::named_set`]'s `id:
/// Some(typed)`.
///
/// # What it does not claim, and that is the whole of its right-hand edge
///
/// The mock's head is `showing`, the name, `▾`, `.sep`, `keep`. The `▾` is the
/// chooser — *point this pane at another deck* — which is [`Pane::deck`]'s
/// per-pane pointer and is still not a control this console has (ADR-0200). It
/// is not drawn, and this derivation reserves [`DeckName::chevron`] for it
/// anyway: the target is the run's own ink and stops there, so the day the
/// chooser lands it takes the rectangle beside the name rather than taking it
/// *back*. A name target that had run to the capsule would have swallowed the
/// chooser's place before anybody drew it, and a press meant for the caret
/// would be a press that re-points the pane.
///
/// # The run is one target and is deliberately not two
///
/// `deck A · drift_night` is one `.what` in the mock and one galley here.
/// Claiming the material and leaving `deck A ·` a readout would be a boundary
/// inside a run of text with nothing on screen drawing it, which is the
/// opposite of *a control claims what it acts on and no more*: what this acts
/// on is the name display, and the name display is the whole run.
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

/// What the count at the right of a pane head reads — `n of m`, the node groups
/// this pane is showing whole out of the ones the deck has.
///
/// The Library foot's `5 of 27` counted on this bay's items rather than on that
/// one's rows, which is what
/// [ADR-0259](../../../../docs/adr/0259-the-keyboard-is-addressed-to-the-bay-that-has-focus-and-a-global-letter-is-a-convenience-or-the-operators-own.md)
/// says a pane's items are: *"Items are its panes; a pane's controls are its
/// deck head and its node groups"*. A percentage would be a number about a
/// rectangle, and what an operator counts is groups.
pub fn count_text(at: &InspectorPane, pane: &Pane) -> String {
    format!("{} of {}", at.shown, pane.nodes.len())
}

/// The count in a pane head, derived — where the run goes, or `None` for a head
/// with no room for it between the label and the capsule.
///
/// It is a readout: nothing hit-tests it, it names no operation, and it carries
/// no row on [every operation](../../../../docs/manual/operations.html) — the
/// mixer head's `3 of 3 · page 1` one bay along, and the reason is the same one
/// that keeps the Library's cursor off that page. What it is *for* is rule 04 —
/// *"A list that showed you part of itself says so and says how much"* — which
/// is the whole of what a scrolled pane owes a reader, and is why this is
/// derived beside the two controls in the row rather than painted wherever
/// there happened to be space.
///
/// # Where it sits, and what gives way to what
///
/// `.half-head` is a flex row: the label and the run are at the left, `.sep`
/// takes what is over, and the capsule is hard against the right-hand padding.
/// This goes one [`size::HALF_HEAD_GAP`] to the left of the capsule — the mixer
/// head's order, where the readout is left of the pill — and the run to its
/// left is what gives way when the pane is narrowed, because the run is the one
/// thing in the row that is clipped rather than dropped.
///
/// `None` is a head that cannot hold it, which is [`keep_pill`]'s rule read on
/// a readout: measured against the room between the label and the capsule, so a
/// head that would have to draw this over the words draws none of it. A pane at
/// the declared minimum of 208 has room for all three
/// ([ADR-0279](../../../../docs/adr/0279-the-centre-is-two-parameter-rows-wide-because-a-pane-that-cannot-draw-a-fader-is-not-a-minimum.md)),
/// so this `None` is a pane below what the arrangement admits rather than a
/// state rule 04 is broken in.
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
    // Where the run beside it would start: the label inside the left padding,
    // and one gap. This is measured against that rather than against the
    // head's edge so that a head narrow enough to want the room for its words
    // keeps it — the words are what says *which deck*, and a count of groups
    // on a pane whose deck has gone unnamed is a number about nothing.
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

/// A pane head taking letters, and the whole of the console's second
/// letter-taking flow's state.
///
/// # One at a time, and it carries which head it is in
///
/// [`Menu::Naming`] is the first flow and it is one because a menu is one; this
/// is one because the keyboard is one. Whoever holds the keys takes them whole
/// while a name is being asked for — `s` is an `s` in a name and not a solo —
/// so two open fields would be two places one keystroke could go, with nothing
/// on the panel saying which. So this is an `Option` on the console and not a
/// field per pane, and it names the pane the field is drawn in.
///
/// # The buffer is a `String` this crate owns and does not check
///
/// [`Menu::Naming`]'s rule, unchanged and for its reason: a name that is not
/// one path component is refused where the file is written, in one sentence, by
/// whoever writes it — the surface owns the affordance and never the authority
/// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
/// A head that quietly dropped the characters it did not like would be a rule
/// an operator could only find by experiment.
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
