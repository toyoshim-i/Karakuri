use super::*;

/// Whether the deck a pane is showing is on air, off the Mixer bay's own
/// reading of it.
///
/// [`Strip::tally`] through [`residency`], which is the one place a tally
/// becomes a residency on this console — a second reading of it here would be
/// two statements about one fact, and the mock draws the pane's `keep` and the
/// strip's tally in one pink for exactly the reason that they are one fact.
///
/// A deck with no strip is not on air, which is a state rather than a
/// fallback: [`View::mixer`] is as long as the deck has slots, so a pane
/// pointed past the end is pointed at nothing, and nothing is not live.
pub(crate) fn on_air(strips: &[Strip], deck: usize) -> bool {
    strips
        .get(deck)
        .is_some_and(|strip| residency(strip.tally) == Residency::Live)
}

/// One pane of the Inspector, painted.
///
/// Where everything goes is [`inspector`]'s, so this paints and derives
/// nothing but the position of one chip after another along a row, which is
/// what a flex row is.
///
/// Term for term from `style.css`:
///
/// - `.half-head` — `color: var(--c-faint)` for the label, `.what`'s
///   `color: var(--c-text)` for the deck and its material, over a
///   `border-bottom: 1px solid var(--c-hair)`.
/// - `.deck-head` — a `.mini` for the sync mode, `.anchor` at
///   [`size::ANCHOR_SIZE`] beside it, and the fold's `.mini` pushed to the
///   right by `.sep`'s `flex: 1`.
/// - `.node-head` — `background: var(--c-tint)`, `.addr`'s
///   `color: var(--c-lav)`, the name in `var(--c-dim)`, and `.auth`'s three
///   words at the right.
/// - `.rend-row` — `.rend` chips, the live one in `var(--c-pink)` over a 15%
///   wash of it.
/// - `.param` — the mock's four tracks, with the fader taking what the other
///   three leave.
///
/// Context for rendering an inspector pane into a UI layout.
#[derive(Clone, Copy)]
pub(crate) struct InspectorIntoCtx<'a> {
    pub at: &'a InspectorPane,
    pub pane: &'a Pane,
    pub on_air: bool,
    pub policy: SlotPolicy,
    pub naming: Option<&'a str>,
    pub target: Option<PaneTarget>,
}

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
    // **Derived here and hit-tested by `claim` off the same call**, and asked
    // before the words are painted rather than after: `.half-head` is a flex
    // row with `.sep` between them, so the readout is what gives way when the
    // pane is narrow and the pill keeps its place. `None` is a head with no
    // room for the capsule, which draws none — see [`keep_pill`].
    let keep = keep_pill(ui.ctx(), at, pane);
    let mcp = slot_mcp_pill(ui.ctx(), at, pane, policy);
    // **And the count beside it**, which is what rule 04 asks of a pane that
    // is showing part of itself — derived here off the same head and painted
    // below, exactly as the capsule is. See [`pane_count`].
    let count = pane_count(
        ui.ctx(),
        at,
        pane,
        naming,
        mcp.as_ref().map(|pill| pill.pill),
    );
    // What is left of the head for the two words: everything up to whatever is
    // next along the row, one `.half-head` gap short of it. A name too long for
    // that is clipped, which is the row's own answer to a long name either way
    // — the head is a clip rectangle and there is no ellipsis in this console
    // to draw.
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
    // **The run, from the same derivation `claim` hit-tests** — the mock's
    // `.what`, and the console's second letter-taking flow while a name is
    // going into it. `None` is a head with no room to paint any of it, which
    // is [`deck_name`]'s own refusal and leaves the label alone in the row.
    if let Some(named) = deck_name(
        ui.ctx(),
        at,
        pane,
        naming,
        mcp.as_ref().map(|pill| pill.pill),
    ) {
        // **A ground under the field while it is asking, and none while it is
        // reading.** A caret says letters are going *somewhere*; the tint says
        // where, which is the one thing a run of text in a row of readouts
        // cannot say for itself. It is `.node-head`'s own `--c-tint`, so the
        // console spends no new colour on it — and the pink a capsule is lit
        // in is deliberately not reached for here, because that pink means
        // *on air* two controls away.
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
    // **The pulldown's mark, after the run and inside the head's own clip** —
    // the mock's `▾` beside `deck A · drift_night`, drawn rather than typed
    // for [`CHEVRON_W`]'s reason. It is painted in the label's ink rather than
    // the run's: the mark is a control and the name beside it is a readout,
    // and the console draws every `▾` it has in `--c-faint`.
    if let Some(target) = target {
        let mark = target.chevron;
        chevron_down(&painter, mark, pal.faint);
    }
    // **The mock draws the first pane's `keep` as `.pill.on` and the second
    // pane's as a plain `.pill`**, and what the lit one reads is now on the
    // page: the deck this pane is *showing* is on air. Deck A in the mock is
    // on air *and* holds the selection *and* is the first pane, and the wash
    // is the first of the three for two reasons the console already holds —
    // `.pill.on`'s pink *is* the pink a tally on air is drawn in
    // ([`on_pill_at`]), and the selection is drawn in lavender everywhere
    // else on this panel, so a pink wash meaning *selected* would be the one
    // colour on the console saying two things.
    //
    // **It is handed in rather than asked here**, which is `mixer_into`'s
    // `marked` and `selection` one bay over: residency is the *mixer's*
    // reading of a deck — [`Strip::tally`] — and a second derivation of it in
    // this bay would be two statements about one fact.
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
    // **The count, in the label's own ink**: `.half-head`'s `color:
    // var(--c-faint)`, which is what the mock gives every readout in this row
    // and what the Library foot gives its own `5 of 27`. It is painted inside
    // the head's clip and not the words' — the words stop short of it.
    if let Some(rect) = count {
        let galley = painter.layout_job(span_at(&count_text(at, pane), size::BASE, pal.faint));
        painter.galley(
            Pos2::new(rect.min.x, rect.center().y - galley.size().y * 0.5),
            galley,
            pal.faint,
        );
    }

    // **Derived here and hit-tested by `claim` off the same call**, which is
    // the rule every other control on this panel is drawn under. `None` is a
    // row too narrow to hold its chips, and it draws none rather than half of
    // each — see [`deck_head`].
    if let Some(head) = deck_head(ui.ctx(), at, pane) {
        deck_head_into(ui, pal, &head, pane);
    }

    // **The clip is what makes a scrolled pane safe**, and it is the same
    // rectangle [`InspectorPane::grip`] refuses a press outside: a group cut
    // by the top edge is painted with its head under the deck head and clipped
    // away there, and a press on the part that is not on screen reaches
    // nothing.
    let painter = ui.painter().with_clip_rect(at.body);
    for index in at.drawn(&pane.nodes) {
        let node = &pane.nodes[index];
        let rect = at.group(&pane.nodes, index);
        node_into(&painter, pal, rect, node);
        // `.node-group`'s `border-bottom: 1px solid var(--c-hair)`, which
        // `:last-child` does not carry — so it goes *between* two groups, and
        // the last node's is not drawn whether or not the pane is scrolled far
        // enough to have it on screen. It is `nodes.len()` and no longer the
        // count of what is drawn, because a group cut by the bottom edge has
        // a rule under it and the next group is what it separates from.
        if index + 1 < pane.nodes.len() {
            let rule = rect.max.y + size::HAIRLINE * 0.5;
            painter.line_segment(
                [Pos2::new(rect.min.x, rule), Pos2::new(rect.max.x, rule)],
                Stroke::new(size::HAIRLINE, pal.hair),
            );
        }
    }
}
