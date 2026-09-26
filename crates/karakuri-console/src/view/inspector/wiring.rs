use super::*;

// ---------------------------------------------------------------------------
// The Inspector's wiring, nodes and uses lines
// ---------------------------------------------------------------------------

/// Slot configuration in the deck head, providing capacity options and deterministic salt.
///
/// See ADR-0328 for details on deck head capacity stepping and re-salting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Aimed {
    /// How many items can be rendered simultaneously across all renderers.
    pub capacity: u32,
    /// Whether [`Aimed::capacity`] was asked for, rather than being what the
    /// material declares for itself. It is the chip's lit state and nothing else:
    /// the number is drawn either way, and this says who chose it.
    pub stated: bool,
    /// Capacity values for each renderer instance, indexed by renderer index.
    pub capacities: Vec<u32>,
    /// The salt `re-salt` asks for: the next in this slot's own sequence, derived
    /// from the salt it is running.
    pub salt: u32,
}

/// One node group: the head that names a node and says who may move it, and
/// whatever is under it.
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    /// The mock's `.addr` — `L1:0`, `L2:0`, `L4`. Written by whoever read the Set,
    /// because the layer names are `karakuri-ir`'s `Kind` and this crate depends on
    /// neither it nor the engine.
    pub addr: String,
    /// What the node is called: `Set::node_names`, or the mock's `renderers` for
    /// the group that folds several.
    pub name: String,
    /// Authority level and address of this node, or `None` if grouping multiple nodes (ADR-0211, ADR-0216).
    pub authority: Option<NodeAuthority>,
    /// Address of node whose source code can be preserved via keep, or `None` (ADR-0338).
    pub keep: Option<NodeAddress>,
    /// The mock's `.rend-row`: every renderer this Set has, and which of them is
    /// live. Empty on every group that is not the renderers'.
    pub renderers: Vec<Renderer>,
    /// The inputs this node's procedure declares, and what fills each — empty on
    /// every node that declares none, which is most of them.
    pub uses: Vec<Uses>,
    /// The published controls that belong to this node.
    pub params: Vec<Param>,
}

/// Declared input dependency for a node and its currently wired source (ADR-0152, [P-0086](../../../../docs/principles/0086-a-procedure-knows-only-what-it-declares.md), [P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Uses {
    /// What the procedure calls this input — `far` in `--edge morph.far=…`.
    pub slot: karakuri_operation::InputPort,
    /// The node filling it, by name. There is no unfilled state: a Set with an
    /// empty input does not build, so this control replaces and never clears.
    pub to: String,
    /// The nodes a pick may name, in node order — this deck's nodes of the kind the
    /// input takes, with the declaring node left out of its own list.
    pub candidates: Vec<String>,
}

/// Node identifier and authority mode displayed on a node group header (ADR-0286).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeAuthority {
    pub at: NodeAddress,
    /// Who may move it — `manual` where nobody has spoken for it, which is what a
    /// node nobody has spoken for is rather than a placeholder.
    pub level: Authority,
}

/// One renderer chip.
#[derive(Debug, Clone, PartialEq)]
pub struct Renderer {
    pub name: String,
    /// Indicates whether this renderer is currently active and reaching output under composite mode.
    pub live: bool,
}

/// Available node authority levels in order of increasing autonomy (`man / sug / auto`).
pub const AUTHORITIES: [Authority; 3] = [
    Authority::Manual,
    Authority::Suggesting,
    Authority::Automatic,
];

/// Returns the abbreviated display label for a node authority level.
pub(crate) fn auth_word(authority: Authority) -> &'static str {
    match authority {
        Authority::Manual => "man",
        Authority::Suggesting => "sug",
        Authority::Automatic => "auto",
    }
}

/// Layout geometry for a node `uses` wiring line and its candidate dropdown card.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UsesLine {
    /// The whole line, the full width of the group — what [`uses_into`] paints into
    /// and what a press has to be inside before any of it is asked.
    pub row: Rect,
    /// The capsule naming the node filling this input, at the right of the line. A
    /// press on it opens the card; a press on it while the card is down is the
    /// host's to read as *shut it*, which is [`Load`]'s arrangement.
    pub chip: Rect,
    /// How many candidates the card offers, which is [`Uses::candidates`]' length
    /// while the card is down and zero while it is shut — [`Load::rows`]' shape and
    /// its reason: [`UsesLine::row_at`] cannot hand out a rectangle for a card
    /// nobody opened.
    pub rows: usize,
}

impl UsesLine {
    /// Whether `p` is on the capsule.
    pub fn hit_chip(&self, p: karakuri_layout::Point) -> bool {
        self.chip.contains(Pos2::new(p.x, p.y))
    }

    /// The card under the capsule, or `None` while it is shut — and `None` for a
    /// line with no candidate to offer, which is a deck holding one node of the
    /// kind this input takes: the node already wired is left out of its own list,
    /// so there is nothing to pick and no card to open.
    pub fn list(&self, viewport: Rect) -> Option<Rect> {
        if self.rows == 0 {
            return None;
        }
        let height = size::LIB_LIST_PAD * 2.0 + size::LIB_ROW_H * self.rows as f32;
        let width = self
            .chip
            .width()
            .max(size::LIB_ROW_H + (size::LIB_ROW_PAD_X + size::LIB_LIST_PAD) * 2.0);
        Some(held_inside(
            &viewport,
            self.chip.min.x,
            self.chip.max.y + size::PILL_GAP,
            width,
            height,
        ))
    }

    /// Where the `index`th candidate's row is, or `None` off the end and `None`
    /// while the card is shut.
    pub fn row_at(&self, viewport: Rect, index: usize) -> Option<Rect> {
        if index >= self.rows {
            return None;
        }
        let list = self.list(viewport)?;
        let top = list.min.y + size::LIB_LIST_PAD + size::LIB_ROW_H * index as f32;
        Some(Rect::from_min_max(
            Pos2::new(list.min.x + size::LIB_LIST_PAD, top),
            Pos2::new(list.max.x - size::LIB_LIST_PAD, top + size::LIB_ROW_H),
        ))
    }

    /// Which candidate `p` is on, or `None` off every row.
    pub fn picked(&self, viewport: Rect, p: karakuri_layout::Point) -> Option<usize> {
        let at = Pos2::new(p.x, p.y);
        (0..self.rows).find(|index| {
            self.row_at(viewport, *index)
                .is_some_and(|row| row.contains(at))
        })
    }
}

/// Total height of a node group including its header, optional renderer row, inputs, and parameters.
pub(crate) fn group_h(node: &Node) -> f32 {
    size::NODE_HEAD_H
        + uses_h(node)
        + match node.renderers.is_empty() {
            true => 0.0,
            false => size::REND_ROW_H,
        }
        + node.params.iter().map(rows_h).sum::<f32>()
}

/// Total height of the declared `uses` input rows for a node.
pub(crate) fn uses_h(node: &Node) -> f32 {
    node.uses.len() as f32 * size::USES_H
}

/// Where a node group's `index`th `uses` line goes: under `.node-head`, the
/// full width of the group and [`size::USES_H`] tall.
pub(crate) fn uses_rect(group: Rect, index: usize) -> Rect {
    let top = group.min.y + size::NODE_HEAD_H + index as f32 * size::USES_H;
    Rect::from_min_max(
        Pos2::new(group.min.x, top),
        Pos2::new(group.max.x, top + size::USES_H),
    )
}

/// Computes the bounding rectangle for a `uses` chip within its line.
pub(crate) fn uses_chip_in(ctx: &egui::Context, row: Rect, uses: &Uses) -> Rect {
    let text_w = ctx.fonts_mut(|f| {
        f.layout_no_wrap(
            uses.to.clone(),
            FontId::new(size::USES_SIZE, FontFamily::Proportional),
            Color32::PLACEHOLDER,
        )
        .size()
        .x
    });
    let width = size::PILL_PAD_X * 2.0 + text_w + size::SINK_GAP + CHEVRON_W;
    Rect::from_min_size(
        Pos2::new(
            row.max.x - size::PARAM_PAD_R - width,
            row.center().y - size::USES_CHIP_H * 0.5,
        ),
        egui::vec2(width, size::USES_CHIP_H),
    )
}

/// Computes the bounding rectangle for the renderer selection row within a node group.
pub(crate) fn rend_row_in(group: Rect, node: &Node) -> Rect {
    let top = group.min.y + size::NODE_HEAD_H + uses_h(node);
    Rect::from_min_max(
        Pos2::new(group.min.x, top),
        Pos2::new(group.max.x, top + size::REND_ROW_H),
    )
}

/// Returns true if renderer selection is interactive (composite mode with 2+ renderers).
pub(crate) fn a_choice(pane: &Pane, node: &Node) -> bool {
    pane.composite && node.renderers.len() > 1
}

/// Measures rendered width of a renderer chip label including padding and borders.
fn rend_width(ctx: &egui::Context, name: &str) -> f32 {
    ctx.fonts_mut(|f| {
        f.layout_no_wrap(
            name.to_owned(),
            FontId::new(size::BASE, FontFamily::Proportional),
            Color32::PLACEHOLDER,
        )
        .size()
        .x
    }) + size::REND_PAD_X * 2.0
}

/// Iterates over each renderer chip bounding box from left to right in draw order.
///
/// Yields `(index, Rect)` without heap allocations.
pub fn rend_chips<'a>(
    ctx: &'a egui::Context,
    row: Rect,
    renderers: &'a [Renderer],
) -> impl Iterator<Item = (usize, Rect)> + 'a {
    let mut x = row.min.x + size::REND_ROW_PAD_L;
    // One padding down from the top of the row, which is where `.rend-row`
    // puts it: its padding is `3px 10px 6px 12px`, so a chip is not centred in
    // the row and the space under it is twice the space over it.
    let top = row.min.y + size::REND_ROW_PAD_T;
    renderers.iter().enumerate().map(move |(index, rend)| {
        let chip = Rect::from_min_size(
            Pos2::new(x, top),
            egui::vec2(rend_width(ctx, &rend.name), size::REND_H),
        );
        x += chip.width() + size::REND_GAP;
        (index, chip)
    })
}

/// Calculates layout rectangles for the three authority chips right-aligned on a node head.
pub fn auth_chips(
    ctx: &egui::Context,
    head: Rect,
    node: &Node,
) -> impl Iterator<Item = (Authority, Rect)> {
    let head = auth_head(ctx, head, node);
    let widths: Vec<f32> = AUTHORITIES
        .into_iter()
        .map(|level| auth_width(ctx, level))
        .collect();
    let total: f32 = widths.iter().sum::<f32>() + size::AUTH_GAP * (AUTHORITIES.len() - 1) as f32;
    let mut x = head.max.x - size::NODE_HEAD_PAD_X - total;
    let top = head.center().y - size::AUTH_H * 0.5;
    AUTHORITIES
        .into_iter()
        .zip(widths)
        .map(move |(level, w)| {
            let rect = Rect::from_min_size(Pos2::new(x, top), egui::vec2(w, size::AUTH_H));
            x += w + size::AUTH_GAP;
            (level, rect)
        })
        .collect::<Vec<_>>()
        .into_iter()
}

/// Calculates the bounding rectangle for a node head's `keep` capsule, if present.
pub fn node_keep(ctx: &egui::Context, head: Rect, node: &Node) -> Option<Rect> {
    node.keep?;
    // Fonts are not valid until `egui` has run a pass — [`keep_pill`]'s guard,
    // and before the first one there is nothing drawn here to press.
    if ctx.cumulative_pass_nr() == 0 {
        return None;
    }
    let w = ctx.fonts_mut(|f| {
        f.layout_no_wrap(
            KEEP_LABEL.to_owned(),
            FontId::new(size::MINI_SIZE, FontFamily::Proportional),
            Color32::PLACEHOLDER,
        )
        .size()
        .x
    }) + size::MINI_PAD_X * 2.0;
    let pill = Rect::from_min_size(
        Pos2::new(
            head.max.x - size::NODE_HEAD_PAD_X - w,
            head.center().y - size::MINI_H * 0.5,
        ),
        egui::vec2(w, size::MINI_H),
    );
    // Measured against the head's content box, exactly as [`keep_pill`] is:
    // the capsule is placed from the right-hand padding, so what it runs off
    // is the left one.
    let room = head.width() - size::NODE_HEAD_PAD_X * 2.0;
    (positive(head) && w <= room).then_some(pill)
}

/// Computes the remaining node head rectangle available for authority chips after reserving keep space.
fn auth_head(ctx: &egui::Context, head: Rect, node: &Node) -> Rect {
    match node_keep(ctx, head, node) {
        Some(keep) => Rect::from_min_max(
            head.min,
            Pos2::new(
                keep.min.x - size::NODE_HEAD_GAP + size::NODE_HEAD_PAD_X,
                head.max.y,
            ),
        ),
        None => head,
    }
}

/// One authority chip's width: its word at [`size::AUTH_SIZE`] inside `.auth
/// span`'s `padding: 0 5px`.
fn auth_width(ctx: &egui::Context, level: Authority) -> f32 {
    ctx.fonts_mut(|f| {
        f.layout_no_wrap(
            auth_word(level).to_owned(),
            FontId::new(size::AUTH_SIZE, FontFamily::Proportional),
            Color32::PLACEHOLDER,
        )
        .size()
        .x
    }) + size::AUTH_PAD_X * 2.0
}

/// One node group: the head, the renderer row where there is one, and a row per
/// parameter.
pub(crate) fn node_into(painter: &egui::Painter, pal: &Palette, rect: Rect, node: &Node) {
    let head = Rect::from_min_max(
        rect.min,
        Pos2::new(rect.max.x, rect.min.y + size::NODE_HEAD_H),
    );
    // `.node-head`'s `background: var(--c-tint)`, which is the wash that tells
    // a head from the rows under it — the same tint the *allocated* tally
    // carries.
    painter.rect_filled(head, CornerRadius::ZERO, pal.tint);
    let mut x = head.min.x + size::NODE_HEAD_PAD_X;
    let addr = painter.layout_job(span_at(&node.addr, size::BASE, pal.lav));
    let w = addr.size().x;
    painter.galley(
        Pos2::new(x, head.center().y - addr.size().y * 0.5),
        addr,
        pal.lav,
    );
    x += w + size::NODE_HEAD_GAP;
    let name = painter.layout_job(span_at(&node.name, size::BASE, pal.dim));
    painter.galley(
        Pos2::new(x, head.center().y - name.size().y * 0.5),
        name,
        pal.dim,
    );
    if let Some(authority) = node.authority {
        auth_into(painter, pal, head, node, authority.level);
    }
    // Paint node keep capsule at right edge of node head if present.
    if let Some(pill) = node_keep(painter.ctx(), head, node) {
        // Node head keep capsule is drawn in standard mini pill styling without selection wash.
        let galley = painter.layout_no_wrap(
            KEEP_LABEL.to_owned(),
            FontId::new(size::MINI_SIZE, FontFamily::Proportional),
            pal.faint,
        );
        mini_into(painter, pal, pill, false, |painter, colour| {
            painter.galley(
                Pos2::new(
                    pill.min.x + size::MINI_PAD_X,
                    pill.center().y - galley.size().y * 0.5,
                ),
                galley,
                colour,
            );
        });
    }

    // Declared inputs positioned below the head and above parameter rows.
    for (index, uses) in node.uses.iter().enumerate() {
        uses_into(painter, pal, uses_rect(rect, index), uses);
    }

    // Renderer selection row positioned via `rend_row_in`.
    if !node.renderers.is_empty() {
        rend_row_into(painter, pal, rend_row_in(rect, node), &node.renderers);
    }
    // Parameter rows positioned via `param_rect`.
    for (index, param) in node.params.iter().enumerate() {
        param_into(painter, pal, param_rect(rect, node, index), param);
        // Sensitivity row positioned via `sens_rect`.
        if let (Some(row), Some(source)) = (sens_rect(rect, node, index), param.bound.as_ref()) {
            sens_into(painter, pal, row, source);
        }
    }
}

/// Paints right-aligned `man / sug / auto` authority chips, highlighting the active level ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
fn auth_into(
    painter: &egui::Painter,
    pal: &Palette,
    head: Rect,
    node: &Node,
    authority: Authority,
) {
    for (level, rect) in auth_chips(painter.ctx(), head, node) {
        let galley = painter.layout_no_wrap(
            auth_word(level).to_owned(),
            FontId::new(size::AUTH_SIZE, FontFamily::Proportional),
            pal.faint,
        );
        let sel = level == authority;
        let colour = match sel {
            true => pal.mint,
            false => pal.faint,
        };
        if sel {
            painter.rect_filled(
                rect,
                // `border-radius: 999px` on a box this short is a capsule.
                CornerRadius::same((size::AUTH_H * 0.5) as u8),
                tint(pal.mint, 15),
            );
        }
        painter.galley(
            Pos2::new(
                rect.min.x + size::AUTH_PAD_X,
                rect.center().y - galley.size().y * 0.5,
            ),
            galley,
            colour,
        );
    }
}

/// Paints renderer selection chips across the row, highlighting the active renderer with pink glow.
fn rend_row_into(painter: &egui::Painter, pal: &Palette, row: Rect, renderers: &[Renderer]) {
    for (index, rect) in rend_chips(painter.ctx(), row, renderers) {
        let rend = &renderers[index];
        let galley = painter.layout_no_wrap(
            rend.name.clone(),
            FontId::new(size::BASE, FontFamily::Proportional),
            pal.dim,
        );
        let radius = CornerRadius::same((size::REND_H * 0.5) as u8);
        match rend.live {
            true => {
                painter.add(
                    egui::epaint::Shadow {
                        offset: [0, 0],
                        blur: size::TALLY_GLOW - 1,
                        spread: 0,
                        color: pal.glow_pink,
                    }
                    .as_shape(rect, radius),
                );
                painter.rect_filled(rect, radius, tint(pal.pink, 15));
            }
            false => {
                painter.rect_stroke(
                    rect,
                    radius,
                    Stroke::new(size::HAIRLINE, pal.line),
                    StrokeKind::Inside,
                );
            }
        }
        painter.galley(
            Pos2::new(
                rect.min.x + size::REND_PAD_X,
                rect.center().y - galley.size().y * 0.5,
            ),
            galley,
            match rend.live {
                true => pal.pink,
                false => pal.dim,
            },
        );
    }
}

/// Paints a node input `uses` line with slot name, chevron, and connected node capsule.
fn uses_into(painter: &egui::Painter, pal: &Palette, row: Rect, uses: &Uses) {
    let painter = painter.with_clip_rect(row);
    let word = painter.layout_job(span_at(
        &format!("uses {}", uses.slot),
        size::USES_SIZE,
        pal.dim,
    ));
    painter.galley(
        Pos2::new(
            row.min.x + size::PARAM_PAD_L,
            row.center().y - word.size().y * 0.5,
        ),
        word,
        pal.dim,
    );
    let chip = uses_chip_in(painter.ctx(), row, uses);
    painter.rect_stroke(
        chip,
        CornerRadius::same((chip.height() * 0.5) as u8),
        Stroke::new(size::HAIRLINE, pal.line),
        StrokeKind::Inside,
    );
    let name = painter.layout_job(span_at(&uses.to, size::USES_SIZE, pal.dim));
    painter.galley(
        Pos2::new(
            chip.min.x + size::PILL_PAD_X,
            chip.center().y - name.size().y * 0.5,
        ),
        name,
        pal.dim,
    );
    // The `▾`, drawn as the same triangle every pulldown on this console draws:
    // `CHEVRON_W` across and `CHEVRON_H` deep, centred in the padding at the
    // capsule's right.
    let chevron = Rect::from_center_size(
        Pos2::new(
            chip.max.x - size::PILL_PAD_X - CHEVRON_W * 0.5,
            chip.center().y,
        ),
        egui::vec2(CHEVRON_W, CHEVRON_H),
    );
    chevron_down(&painter, chevron, pal.dim);
}

/// Paints the candidate node selection card dropdown for an open `uses` input line.
pub fn uses_card_into(
    ui: &Ui,
    pal: &Palette,
    line: &UsesLine,
    uses: &Uses,
    card: Rect,
    room: Rect,
) {
    let painter = ui.painter();
    popup_card(painter, pal, card);
    let painter = painter.with_clip_rect(card);
    for (index, name) in uses.candidates.iter().enumerate().take(line.rows) {
        let Some(row) = line.row_at(room, index) else {
            continue;
        };
        card_row_text(&painter, row, name, pal.dim);
    }
}
