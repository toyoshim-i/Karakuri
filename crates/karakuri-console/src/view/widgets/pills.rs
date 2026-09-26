use super::super::*;

// ---------------------------------------------------------------------------
// The four class pills: what a model may reach, and the hand that opens it
// ---------------------------------------------------------------------------

/// The word on a class's pill while the class is off.
pub(crate) const MCP_OFF: &str = "mcp · off";

/// And the word while it is on.
pub(crate) const MCP_ON: &str = "mcp · on";

/// Which of the two a class reads as.
pub fn mcp_word(open: bool) -> &'static str {
    match open {
        true => MCP_ON,
        false => MCP_OFF,
    }
}

/// Maps a permission [`Class`] to the region name where its pill is drawn.
pub fn opens(class: Class) -> &'static str {
    match class {
        Class::LiveDeck => "program",
        Class::MixFaders => "mixer",
        Class::MasterEffects => "master",
        Class::InputsAndOutputs => "outputs",
    }
}

/// Returns the [`Class`] opened by a given region, or `None` if the region has none.
pub fn class_at(region: &str) -> Option<Class> {
    Class::ALL
        .iter()
        .copied()
        .find(|class| opens(*class) == region)
}

/// A permission class pill: bounding capsule, target class, and current open state.
///
/// Modifying the map configuration returns a value per [ADR-0236](../../../../docs/adr/0236-a-map-is-the-layer-between-a-surface-and-the-vocabulary-and-the-audit-is-one-of-the-things-it-does.md), [ADR-0156](../../../../docs/adr/0156-the-console-names-nothing-in-karakuri-environment.md), and [ADR-0235](../../../../docs/adr/0235-nothing-is-disabled-and-the-call-is-refused.md).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct McpPill {
    /// The control: the capsule a press has to land in, which is the rectangle
    /// [`bay_head`] or [`outputs_into`] painted.
    pub pill: Rect,
    /// The class it opens.
    pub class: Class,
    /// Whether that class is open — read out of the opening handed in, and kept
    /// nowhere here.
    pub open: bool,
}

impl McpPill {
    /// Returns a new [`Open`] set with this class toggled.
    pub fn next(&self, open: Open) -> Open {
        open.with(self.class, !self.open)
    }

    /// Whether `p` is on the control.
    pub fn hit(&self, p: karakuri_layout::Point) -> bool {
        self.pill.contains(Pos2::new(p.x, p.y))
    }
}

/// Derives the class pill for `class` if visible, handling headless Outputs per [ADR-0159](../../../../docs/adr/0159-outputs-is-a-strip-and-never-a-bay.md).
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

/// The mock's `box-shadow: 0 0 9px var(--c-glow)` on `.pill.armed`, as the blur
/// it is.
pub(crate) const ARMED_GLOW: u8 = 9;

/// `color-mix(in srgb, var(--c-mint) 14%, transparent)`, as the percentage
/// [`tint`] takes.
pub(crate) const ARMED_WASH: u8 = 14;

/// The mock's `box-shadow: 0 0 10px var(--c-glowp)` on `.pill.on`, as the blur
/// it is — one pixel wider than [`ARMED_GLOW`] and in the pink halo rather than
/// the mint one, which is the same pair [`crate::room::size::TALLY_GLOW`] is on
/// the other side of this bay.
pub(crate) const ON_GLOW: u8 = 10;

/// `color-mix(in srgb, var(--c-pink) 16%, transparent)`, as the percentage
/// [`tint`] takes. Two points heavier than [`ARMED_WASH`], which is the mock's
/// own difference between *armed* and *this is the press that does it*.
pub(crate) const ON_WASH: u8 = 16;

/// Width of a `.pill` holding `text`, including padding.
pub(crate) fn pill_width(ctx: &egui::Context, text: &str) -> f32 {
    if ctx.cumulative_pass_nr() == 0 {
        return size::PILL_PAD_X * 2.0;
    }
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

/// Lays out bay head pills right to left, invoking `place` for each capsule.
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

/// Paints a pill capsule, either standard or armed (e.g. for an open class pill).
pub(crate) fn pill_into(ui: &Ui, pal: &Palette, rect: Rect, text: &str, armed: bool) {
    if text == "building" {
        building_pill_at(ui, pal, rect);
    } else {
        match armed {
            true => armed_pill_at(ui, pal, rect, text),
            false => pill_at(ui, pal, rect, text),
        }
    }
}

/// A distinct in-flight build pill drawn in `pal.sun` to indicate background compilation.
pub(crate) fn building_pill_at(ui: &Ui, pal: &Palette, rect: Rect) {
    let painter = ui.painter();
    let radius = CornerRadius::same((rect.height() * 0.5) as u8);
    painter.rect_filled(rect, radius, tint(pal.sun, ARMED_WASH));
    painter.rect_stroke(
        rect,
        radius,
        Stroke::new(size::HAIRLINE, tint(pal.sun, 128)),
        StrokeKind::Inside,
    );
    let galley = painter.layout_no_wrap(
        "building".to_owned(),
        FontId::new(size::BASE, FontFamily::Proportional),
        pal.sun,
    );
    painter.galley(
        Pos2::new(
            rect.min.x + size::PILL_PAD_X,
            rect.center().y - galley.size().y * 0.5,
        ),
        galley,
        pal.sun,
    );
}

/// `.pill.armed`: no border, a `--c-mint` word over a wash of the same, and the
/// `box-shadow: 0 0 9px var(--c-glow)` that goes with it.
pub(crate) fn armed_pill_at(ui: &Ui, pal: &Palette, rect: Rect, text: &str) {
    let painter = ui.painter();
    // Capsule corner radius using half the box height.
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

/// Paints a `.pill.on` capsule in pink with glow indicating an active state (e.g. go, rec, keep).
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
