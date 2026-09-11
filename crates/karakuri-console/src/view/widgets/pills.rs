use super::super::*;

// ---------------------------------------------------------------------------
// The four class pills: what a model may reach, and the hand that opens it
// ---------------------------------------------------------------------------

/// **The word on a class's pill while the class is shut**, in the mock's own
/// spelling. `docs/manual/console.html` draws all four of them this way and
/// draws none of them any other way, because every class starts shut and shut
/// is the console the page draws.
pub(crate) const MCP_SHUT: &str = "mcp · shut";

/// **And the word while it is open**, which `docs/manual/console.html`
/// specifies under *what a model is refused, and where a class opens*: *"open,
/// it reads `mcp · open` and is drawn armed"*. It is the other half of a
/// sentence its four tooltips write — *"Click to open the class; click again to
/// shut it"* — rather than a second control. A pill reading `shut` in both
/// states would be a control that never answers a press, which is the one thing
/// those tooltips rule out.
pub(crate) const MCP_OPEN: &str = "mcp · open";

/// **Which of the two a class reads as.** Two constants and this, so that the
/// word a hand presses, the word [`crate::input`] measures the capsule by, and
/// the word a program prints in a legend are one string.
///
/// Exported for the third of those: the answer to *what does this pill say* is
/// this crate's and nobody else's, and a caller that wrote the words out again
/// would be the copy that goes stale — which is the defect `karakuri`'s startup
/// legend has already been repaired of once.
pub fn mcp_word(open: bool) -> &'static str {
    match open {
        true => MCP_OPEN,
        false => MCP_SHUT,
    }
}

/// **Where a class's pill is drawn**, as the arrangement's own name for the
/// region.
///
/// [`Class::bay`] is the same answer in the words a *refusal* says it in —
/// `Program`, `Mixer`, `Master`, `Outputs` — and these are the regions those
/// name. A `match` rather than a lowercase of that function, so a fifth class
/// stops the build here instead of looking for a region nobody has drawn;
/// `tests/mcp_pill.rs` asserts the two answers agree, in both directions.
pub fn opens(class: Class) -> &'static str {
    match class {
        Class::LiveDeck => "program",
        Class::MixFaders => "mixer",
        Class::MasterEffects => "master",
        Class::InputsAndOutputs => "outputs",
    }
}

/// **The class a region opens**, or `None` for the nine regions that open none.
///
/// **A bay carrying no class draws no pill**, which is the answer that commits
/// to neither of the two ADR-0235 leaves open: it lists *"whether a bay that
/// carries no class draws the indicator at all"* as undecided, and the manual
/// says why it matters — *"an indicator that is present everywhere reads as a
/// state wherever it is absent"*. Drawing nothing at the Library, the
/// Inspector, Staging and the Sequencer is what the page draws.
pub fn class_at(region: &str) -> Option<Class> {
    Class::ALL
        .iter()
        .copied()
        .find(|class| opens(*class) == region)
}

/// **A class's pill, derived**: the capsule, the class it opens, and whether
/// that class is open now.
///
/// # It has no `op`, and that is the decision rather than an omission
///
/// [`Outputs::op`] and [`ProgramHead::op`] both answer *what does a press ask
/// for* with a named [`Op`], and every other control on this panel answers with
/// an [`Operation`]. **This one answers with neither**, and
/// [ADR-0236](../../../../docs/adr/0236-a-map-is-the-layer-between-a-surface-and-the-vocabulary-and-the-audit-is-one-of-the-things-it-does.md)
/// is why: the opening is **configuration of the map** — the layer every
/// surface reaches the vocabulary through — and not a member of the vocabulary
/// the map addresses. The rule it draws is narrower than *map configuration is
/// never an operation*, because [`Operation::PointLane`] already is one: **a
/// setting that decides whether a surface may reach a class of operations
/// cannot itself be one of those operations**, since rule 01 would then make it
/// reachable from the surface it governs, and a permission an actor can grant
/// itself is not a permission.
///
/// So a press hands back a **value** — [`McpPill::next`] — and whoever holds
/// the run's `karakuri_environment::Opening` writes it there. This crate takes
/// no such handle and names nothing in that package (ADR-0156); what it does
/// name is [`Open`], which is `karakuri-operation`'s and is the leaf every
/// surface already depends on.
///
/// # Refused rather than hidden, which is why the pill is only ever a pill
///
/// Nothing on this console is turned off while a class is shut. ADR-0235:
/// *"the list never shortens and the call is refused"*, and the manual says it
/// of the operator too — *"Nothing here is ever refused to a hand."* So this
/// control draws a word and changes no other control's state, and a reader
/// looking for the half of it that greys something out will not find one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct McpPill {
    /// **The control**: the capsule a press has to land in, which is the
    /// rectangle [`bay_head`] or [`outputs_into`] painted.
    pub pill: Rect,
    /// The class it opens.
    pub class: Class,
    /// Whether that class is open — read out of the opening handed in, and kept
    /// nowhere here.
    pub open: bool,
}

impl McpPill {
    /// **What a press asks for**, and it is a state rather than a direction:
    /// the opening handed in, with this one class set the other way.
    ///
    /// [`Open::with`] *"names a state and never a direction"* (P-0090), and the
    /// toggle is this method choosing which state it means — the same
    /// affordance-over-a-named-thing [`Outputs::op`] and [`ProgramHead::op`]
    /// have, one layer out of the vocabulary.
    ///
    /// **It takes the opening rather than holding it**, so a press writes the
    /// other three classes back exactly as they were: a control that returned a
    /// bare `bool` would leave the caller to compose the value, which is the
    /// one place three classes could quietly be shut by a press on the fourth.
    pub fn next(&self, open: Open) -> Open {
        open.with(self.class, !self.open)
    }

    /// Whether `p` is on the control.
    pub fn hit(&self, p: karakuri_layout::Point) -> bool {
        self.pill.contains(Pos2::new(p.x, p.y))
    }
}

/// **One of the four class pills, derived**, or `None` where there is none to
/// press — before the first frame, with the bay folded, off a solo somewhere
/// else, or in a bay too short to hold its own head.
///
/// **Three of the four sit in a bay head** and come out of [`head_capsule`],
/// which is [`head_pills`] asked a second time rather than copied. **The fourth
/// sits in a row that has no head at all** — [`Kind::Outputs`] is a headless
/// strip (ADR-0159) — so it comes out of [`outputs`], beside the word that
/// stands in for a head. `Class::opened_at` is the gate saying the same thing
/// in the words a refusal uses: three are *the head of the … bay* and one is
/// *the Outputs row, which has no head*.
///
/// `layout` must be solved. `ctx` is asked for the type, because a `.pill` is
/// as wide as the word in it — and the two words are not the same width, which
/// is why this takes the opening rather than reading it back off anything.
pub fn mcp_pill(
    ctx: &egui::Context,
    layout: &karakuri_layout::Layout,
    class: Class,
    open: Open,
) -> Option<McpPill> {
    // Fonts are not valid until `egui` has run a pass, exactly as in
    // [`outputs`] and [`program_head`]: a press before the first frame is a
    // press on a control that has never been drawn.
    if ctx.cumulative_pass_nr() == 0 {
        return None;
    }
    let name = opens(class);
    let node = layout.find(name)?;
    // The region itself, a column folded around it, or a solo somewhere else:
    // one question for every ancestor, which is [`program_head`]'s own guard.
    if !layout.visible(node) {
        return None;
    }
    let pill = match head_of(region(name)?) {
        Some(head) => head_capsule(
            ctx,
            to_egui(layout.rect(node)),
            &head,
            open,
            mcp_word(open.holds(class)),
        )?,
        None => outputs(ctx, layout, open)?.mcp,
    };
    Some(McpPill {
        pill,
        class,
        open: open.holds(class),
    })
}

// ---------------------------------------------------------------------------
// Pill appearance and geometry helpers
// ---------------------------------------------------------------------------

/// **The mock's `box-shadow: 0 0 9px var(--c-glow)` on `.pill.armed`**, as the
/// blur it is.
pub(crate) const ARMED_GLOW: u8 = 9;

/// **`color-mix(in srgb, var(--c-mint) 14%, transparent)`**, as the percentage
/// [`tint`] takes.
pub(crate) const ARMED_WASH: u8 = 14;

/// **The mock's `box-shadow: 0 0 10px var(--c-glowp)` on `.pill.on`**, as the
/// blur it is — one pixel wider than [`ARMED_GLOW`] and in the pink halo
/// rather than the mint one, which is the same pair
/// [`crate::room::size::TALLY_GLOW`] is on the other side of this bay.
pub(crate) const ON_GLOW: u8 = 10;

/// **`color-mix(in srgb, var(--c-pink) 16%, transparent)`**, as the percentage
/// [`tint`] takes. Two points heavier than [`ARMED_WASH`], which is the mock's
/// own difference between *armed* and *this is the press that does it*.
pub(crate) const ON_WASH: u8 = 16;

/// The width of a `.pill` holding `text`: the run at [`size::BASE`], plus
/// `.pill`'s `padding: 0 8px` either side.
///
/// `ctx` rather than a `Ui`, because [`program_head`] is a derivation and has
/// no painter -- the same reason [`mixer`] and [`outputs`] measure their words
/// off the context.
pub(crate) fn pill_width(ctx: &egui::Context, text: &str) -> f32 {
    let run = ctx.fonts_mut(|f| {
        f.layout_no_wrap(
            text.to_owned(),
            FontId::new(size::BASE, FontFamily::Proportional),
            Color32::PLACEHOLDER,
        )
        .size()
        .x
    });
    run + size::PILL_PAD_X * 2.0
}

/// **Where a bay head's pills go**, right to left from the right edge of the
/// head: the grip first where the mock draws one, then the pills in reverse,
/// one [`size::PILL_GAP`] apart. `place` is called once per pill, with its
/// index in `pills` and the capsule it occupies.
///
/// **One derivation for a painted pill and a pressed one.** [`bay_head`]
/// paints from it and [`program_head`] hit-tests from it, which is the
/// arrangement every other control on this console already has
/// ([`crate::input`]): the derivation that draws a control is asked a second
/// time rather than copied, so the capsule an operator sees and the capsule a
/// press lands on cannot come apart.
pub(crate) fn head_pills(
    ctx: &egui::Context,
    rect: Rect,
    pills: &[&str],
    grip: bool,
    mut place: impl FnMut(usize, Rect),
) {
    let head = head_box(rect);
    let mid = head.center().y;
    let mut right = head.max.x - size::HEAD_PAD_X;
    if grip {
        right -= GRIP_W + size::PILL_GAP;
    }
    for (index, pill) in pills.iter().enumerate().rev() {
        let w = pill_width(ctx, pill);
        place(
            index,
            Rect::from_min_size(
                Pos2::new(right - w, mid - size::PILL_H * 0.5),
                egui::vec2(w, size::PILL_H),
            ),
        );
        right -= w + size::PILL_GAP;
    }
}

/// **One capsule in either of the mock's two treatments**: the ordinary
/// `.pill`, or `.pill.armed` where what it names is live.
///
/// The only caller that ever asks for the second is a class pill that is open,
/// and it is the mock's own class rather than an invention here -- the audio-in
/// pill already carries it, and its documentation is where the argument was
/// first written: *"a pill that was only lit would leave which room is being
/// heard unanswered, and a pill that only carried a name would make a dead
/// input and a live one look alike at the distance a panel is read from."* An
/// opening is the same pair of questions -- *which class* and *is it open* --
/// so it gets the same pair of answers.
///
/// **`docs/manual/console.html` draws the shut state only, because every class
/// starts shut, and specifies the open one in words**: *what a model is
/// refused, and where a class opens* says the pill reads `mcp · open` and is
/// drawn armed, and makes that argument in its own terms. Both the word and the
/// treatment are the page's.
pub(crate) fn pill_into(ui: &Ui, pal: &Palette, rect: Rect, text: &str, armed: bool) {
    match armed {
        true => armed_pill_at(ui, pal, rect, text),
        false => pill_at(ui, pal, rect, text),
    }
}

/// `.pill.armed`: no border, a `--c-mint` word over a wash of the same, and the
/// `box-shadow: 0 0 9px var(--c-glow)` that goes with it.
pub(crate) fn armed_pill_at(ui: &Ui, pal: &Palette, rect: Rect, text: &str) {
    let painter = ui.painter();
    // `border-radius: 999px` on a box this short is a capsule, drawn as half
    // the box's own height rather than half [`size::PILL_H`] — the transition
    // row's pills count `.pill`'s border and are two pixels taller
    // ([`size::XPILL_H`]), and a radius read off the constant would leave
    // those three with a corner rather than a capsule.
    let radius = CornerRadius::same((rect.height() * 0.5) as u8);
    painter.add(
        egui::epaint::Shadow {
            offset: [0, 0],
            blur: ARMED_GLOW,
            spread: 0,
            color: pal.glow,
        }
        .as_shape(rect, radius),
    );
    painter.rect_filled(rect, radius, tint(pal.mint, ARMED_WASH));
    let galley = painter.layout_no_wrap(
        text.to_owned(),
        FontId::new(size::BASE, FontFamily::Proportional),
        pal.mint,
    );
    painter.galley(
        Pos2::new(
            rect.min.x + size::PILL_PAD_X,
            rect.center().y - galley.size().y * 0.5,
        ),
        galley,
        pal.mint,
    );
}

/// `.pill.on`: [`armed_pill_at`] in the pink rather than the mint, with the
/// heavier wash and the one-pixel-wider halo the mock gives it — no border, a
/// `--c-pink` word over `color-mix(in srgb, var(--c-pink) 16%, transparent)`,
/// and `box-shadow: 0 0 10px var(--c-glowp)`.
///
/// **The console's second treatment for a lit capsule, and the two say
/// different things.** `.pill.armed` is *this setting is chosen*; `.pill.on`
/// is drawn in the same pink a tally on air is, and what it says is *this is
/// live*: the press that runs the transition on the `go` it was written for,
/// the recording that is running on the `rec` capsule, and the deck a pane is
/// showing being on air on the Inspector's `keep`. **The mock does not reserve
/// it for `go`**, which this said until 2026-09-08 and which the mock has
/// contradicted in two rows since before it was written. Callers:
/// [`transition_into`] and [`inspector_into`].
pub(crate) fn on_pill_at(ui: &Ui, pal: &Palette, rect: Rect, text: &str) {
    let painter = ui.painter();
    // Half the box's own height, for [`armed_pill_at`]'s reason: this capsule
    // is an `XPILL_H` and not a `PILL_H`.
    let radius = CornerRadius::same((rect.height() * 0.5) as u8);
    painter.add(
        egui::epaint::Shadow {
            offset: [0, 0],
            blur: ON_GLOW,
            spread: 0,
            color: pal.glow_pink,
        }
        .as_shape(rect, radius),
    );
    painter.rect_filled(rect, radius, tint(pal.pink, ON_WASH));
    let galley = painter.layout_no_wrap(
        text.to_owned(),
        FontId::new(size::BASE, FontFamily::Proportional),
        pal.pink,
    );
    painter.galley(
        Pos2::new(
            rect.min.x + size::PILL_PAD_X,
            rect.center().y - galley.size().y * 0.5,
        ),
        galley,
        pal.pink,
    );
}

/// One `.pill`, painted into the capsule [`head_pills`] laid out for it.
pub(crate) fn pill_at(ui: &Ui, pal: &Palette, rect: Rect, text: &str) {
    let painter = ui.painter();
    painter.rect_stroke(
        rect,
        // `border-radius: 999px` on a box this short is a capsule, drawn as
        // half the box's own height — [`armed_pill_at`]'s reason.
        CornerRadius::same((rect.height() * 0.5) as u8),
        Stroke::new(1.0, pal.line),
        StrokeKind::Inside,
    );
    let galley = painter.layout_no_wrap(
        text.to_owned(),
        FontId::new(size::BASE, FontFamily::Proportional),
        pal.dim,
    );
    painter.galley(
        Pos2::new(
            rect.min.x + size::PILL_PAD_X,
            rect.center().y - galley.size().y * 0.5,
        ),
        galley,
        pal.dim,
    );
}
