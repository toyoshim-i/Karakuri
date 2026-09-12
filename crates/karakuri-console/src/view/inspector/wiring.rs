use super::super::*;
use super::*;

// ---------------------------------------------------------------------------
// The Inspector's wiring, nodes and uses lines
// ---------------------------------------------------------------------------

/// What a slot is built at, in the two fields the deck head offers: how many
/// elements each of its geometries runs at, and the salt its randomness comes
/// from.
///
/// # It is a reading somebody else took, which is why the candidates are here
///
/// Neither number can be worked out on this side. The ladder is the powers of
/// two inside the range the *material* declares — `capacity [min, max] =
/// default`, which is a `.kir`'s statement and reaches this crate through
/// whoever read the Set — and the salt is the next in the slot's own
/// deterministic sequence, which is the engine's arithmetic over the salt the
/// slot is actually running. So both cross the seam as answers rather than as
/// the facts they are computed from, exactly as [`Pane::allows`] does and for
/// the same reason: a control is not the authority on what it may ask for
/// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)),
/// and this crate holds no engine and no store (ADR-0156).
///
/// The salt is handed over rather than invented, which is the half worth
/// stating twice. A console that reached for a random number would produce a
/// picture no later run could produce again
/// ([P-0092](../../../../docs/principles/0092-the-same-inputs-produce-the-same-frame.md));
/// a console handed the next value of a sequence names a destination the way
/// every other control here does.
/// `docs/adr/0328-the-inspectors-deck-head-steps-a-slots-capacity-and-re-salts-it.md`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Aimed {
    /// What the slot is running at, which is the number the chip reads.
    ///
    /// One number for the slot, because that is what a re-aim carries: a Set
    /// holding two geometries runs both at a stated capacity, and where none is
    /// stated this is the first geometry's own declared default — the same reading
    /// ADR-0228 records the aim taking.
    pub capacity: u32,
    /// Whether [`Aimed::capacity`] was asked for, rather than being what the
    /// material declares for itself. It is the chip's lit state and nothing else:
    /// the number is drawn either way, and this says who chose it.
    pub stated: bool,
    /// The numbers a press steps through, ascending — the powers of two inside the
    /// range every one of this deck's geometries accepts.
    ///
    /// Empty is a chip that is drawn and claims nothing, which is the arrangement
    /// an inert scrub is already in: a deck whose geometries declare no range in
    /// common has no capacity a re-aim could send that all of them would build at,
    /// and there is nothing here to offer.
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
    /// Who may move this node, or `None` where the group is more than one node and
    /// so has no one value.
    ///
    /// The second case is the mock's own `L4 renderers` head: authority is set per
    /// node (ADR-0211, ADR-0216) and that head stands over every renderer the Set
    /// has, so a chip on it would be one of *n* answers drawn as *the* answer. A
    /// Set with one renderer has one node under that head and the chip is drawn.
    ///
    /// It carries the node's address beside the level, and it is one field rather
    /// than two. A press on a chip has to say which node it is about, and the two
    /// are absent together — a head with no one answer has no one node either — so
    /// a pair of `Option`s would be *present or absent as a unit* held true by
    /// prose, which is `karakuri_store::record::NodeAddress`'s argument one crate
    /// along and `docs/contributing.md` §4's structural tier.
    pub authority: Option<NodeAuthority>,
    /// The node whose source a `keep` on this head would write, or `None`
    /// on a head with nothing to keep.
    ///
    /// Two heads carry no capsule and both are the rule rather than an
    /// omission (`docs/adr/0338-…`, decision 4):
    ///
    /// - A head standing over more than one node, which is the mock's
    ///   folded `L4 renderers`. It is [`Node::authority`]'s own absence one
    ///   control along and for its sentence: one capsule over three renderers
    ///   would be one of three answers drawn as *the* answer. Open the fold
    ///   and each renderer has its own.
    /// - The built-in camera, which is a node with no procedure behind it:
    ///   a Set declaring no `kind L3` holds the built-in orbit at `L3:0`
    ///   (`docs/ir-spec.md`, *Several cameras*), and there is no source to
    ///   write. The mock draws that absence too, and this pass reproduces it
    ///   rather than drawing a capsule that refuses.
    ///
    /// It is a field beside [`Node::authority`] rather than that field read
    /// again, because the two absences are not the same set: the built-in
    /// camera *has* an authority and has nothing to keep. A `bool` beside the
    /// address would be *present or absent as a unit* held true by prose,
    /// which is [`NodeAuthority`]'s own argument, so the address and the
    /// having-one are one `Option`.
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

/// One input a node's procedure declares, and the node filling it — the mock's
/// `.uses` line, under the node head and above that node's rows.
///
/// # The slot is the procedure's word and the node is the Set's
///
/// `uses far : Geometry` names `far` and stops, because a part that names the
/// parts around it is bound to one Set and stops being a library part
/// ([P-0086](../../../../docs/principles/0086-a-procedure-knows-only-what-it-declares.md),
/// [ADR-0152](../../../../docs/adr/0152-a-kir-names-a-slot-and-the-set-names-the-nodes.md)).
/// So `slot` is the declaration's own word and `to` is a node's name, which is
/// exactly `karakuri_engine::set::Edge`'s two halves and exactly what
/// [`Operation::WireInput`](karakuri_operation::Operation::WireInput) carries.
///
/// # The candidates are a reading somebody else took
///
/// [`Uses::candidates`] is the list the card offers, handed across the seam
/// like [`Pane::allows`] and [`View::holds`] before it: which nodes are of the
/// kind this input takes is a question about the Set, and this crate holds no
/// engine (ADR-0156). What is offered is not what may be reached — a name the
/// Set cannot use is refused where the Set is built, in the sentence a model's
/// `wire_input` meets
/// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
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

/// Which node a group's head is, and who may move it.
///
/// The address is `karakuri_operation::NodeAddress`, carried over from whoever
/// read the Set and deliberately not parsed back out of [`Node::addr`]: that
/// field is the mock's `L1:0`, a display string in the layer word
/// `docs/ir-spec.md` owns, and reading an address back out of what is drawn is
/// the shape *a statement is held true by the thing it describes* forbids
/// ([ADR-0286](../../../../docs/adr/0286-a-parameter-row-writes-the-control-it-draws-and-carries-the-range-rather-than-the-position.md)).
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
    /// Whether this is the one that reaches the screen, and it is only ever true
    /// under [`Pane::composite`]: *"On, a deck's renderers fold into one result and
    /// one of them is live; off, they are all overdrawn."* Under overdraw every
    /// renderer draws, so marking one would assert a choice the layering does not
    /// make.
    pub live: bool,
}

/// The three levels a node head shows, in the order it shows them.
///
/// `karakuri_operation::Authority` deliberately carries no `ALL` — *"no map
/// target names an authority … it arrives with the first reader"* — and this is
/// that first reader, so the list is written here rather than there. The order
/// is the vocabulary's own declaration order, most restrictive first, which is
/// also the mock's `man / sug / auto`.
///
/// A fourth level cannot slip past it: [`auth_word`] is a `match`, so a level
/// added to the vocabulary does not compile until it has a word here, and
/// `every_authority_the_vocabulary_names_is_on_the_node_head` holds this array
/// against that match.
///
/// Public since the chips became controls: `crate::input::PROBES` says how many
/// controls a node head's chips are, and a `3` written there would be a second
/// answer to a question this array already gives. The list is the console's and
/// the count is one reading of it — [`PANE_NAMES`]' reason for being public,
/// one list along.
pub const AUTHORITIES: [Authority; 3] = [
    Authority::Manual,
    Authority::Suggesting,
    Authority::Automatic,
];

/// The node head's abbreviation for one level, which is the console's own word
/// and deliberately not `Authority::name`: the vocabulary spells these
/// *manual*, *suggesting* and *automatic* because that is what a record
/// carries, and the manual's node head reads `man / sug / auto`.
///
/// A `match` for [`blend_mode`](crate::view)'s reason one crate along: a fourth
/// level in the vocabulary stops the build here rather than drawing a blank
/// chip.
pub(crate) fn auth_word(authority: Authority) -> &'static str {
    match authority {
        Authority::Manual => "man",
        Authority::Suggesting => "sug",
        Authority::Automatic => "auto",
    }
}

/// One `uses` line's control, laid out: the capsule naming the node that fills
/// the input, and the card of candidates under it.
///
/// # One derivation, and the card hangs *down*
///
/// [`View::draw`] paints these rectangles and [`crate::input::claim`] hit-tests
/// them, which is [`DeckHead`]'s rule and [`Load`]'s. The card hangs down from
/// the capsule where the Library bay's hangs up, and the difference is where
/// each control sits: that one is in the *foot* of a bay and this one is inside
/// a pane's body, with the rest of the pane under it. It is held inside the
/// viewport for [`Load::list`]'s reason, so a line near the bottom of a short
/// console draws its card over what is above it rather than off the edge.
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

/// How tall one node group is: its head, the renderer row if it has one, and a
/// [`size::PARAM_H`] row per parameter.
///
/// The hairline between two groups is not in here and is added by whoever
/// stacks them — `.node-group`'s `border-bottom` is `0` on the last of them, so
/// *n* groups carry *n - 1* rules, which is [`size::PREVIEW_GAP`]'s reading of
/// a gap one axis along, and [`content_h`] is where that sum is taken.
pub(crate) fn group_h(node: &Node) -> f32 {
    size::NODE_HEAD_H
        + uses_h(node)
        + match node.renderers.is_empty() {
            true => 0.0,
            false => size::REND_ROW_H,
        }
        + node.params.iter().map(rows_h).sum::<f32>()
}

/// How tall a node's declared inputs come to: one [`size::USES_H`] line each,
/// and nothing at all on the nodes that declare none — which is most of them,
/// and is why this is an addend rather than a row every group carries.
///
/// One function because [`group_h`] sums it and [`param_rect`] and
/// [`uses_rect`] walk past it, which is [`rows_h`]'s own reason one row down.
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

/// Where a `uses` line's capsule is, inside the line — the one derivation
/// [`InspectorPane::uses_line`] hit-tests and [`uses_into`] paints, which is
/// [`rend_chips`]' arrangement one row down and its reason: the same arithmetic
/// written twice is a capsule drawn where a hand cannot reach it.
///
/// The name, the gap and the `▾` — which is drawn rather than typed, so it is a
/// width here and a mark at the paint, exactly as the Outputs row's pill and
/// the Library bay's pulldown already spell it. `.uses`'s `.sep` puts it
/// against the right of the line, which is the node head's arrangement one row
/// up and the deck head's fold two bays over.
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

/// Where a node group's renderer row is: `.rend-row` under `.node-head`, the
/// full width of the group and [`size::REND_ROW_H`] tall.
///
/// One formula, because the row is painted *and* pressed. [`node_into`] walks a
/// group from the top and [`InspectorPane::select_renderer`] asks where the
/// chips in it are; the same arithmetic written twice is two answers that can
/// disagree, which is [`deck_head`]'s rule one row up — *the derivation that
/// draws a control is the one that hit-tests it*.
///
/// Asked only where [`Node::renderers`] is not empty. On a group that has none
/// the rectangle it answers is where the first parameter row goes, which is
/// [`group_h`]'s own arithmetic read the other way.
pub(crate) fn rend_row_in(group: Rect, node: &Node) -> Rect {
    let top = group.min.y + size::NODE_HEAD_H + uses_h(node);
    Rect::from_min_max(
        Pos2::new(group.min.x, top),
        Pos2::new(group.max.x, top + size::REND_ROW_H),
    )
}

/// Whether this group's renderer chips are a choice a press can make, and it is
/// [`Renderer::live`]'s own condition asked of the row rather than of one chip:
/// the deck composites, and it holds more than one renderer.
///
/// The manual's row carries the whole of it — *"Only where the deck composites
/// and holds two or more"* — and [`InspectorPane::select_renderer`] is where
/// the argument for drawing the other two cases and claiming neither is
/// written.
pub(crate) fn a_choice(pane: &Pane, node: &Node) -> bool {
    pane.composite && node.renderers.len() > 1
}

/// One renderer chip's width: the name at [`size::BASE`] inside
/// [`size::REND_PAD_X`] either side, which is what `.rend` is as wide as. Its
/// `border: 1px solid var(--c-line)` is counted in [`size::REND_H`] down the
/// chip and not across it, exactly as `.mini`'s is in [`deck_head`].
///
/// Asked of `egui` rather than derived, for [`library::chip_width`]'s reason
/// one bay along: a capsule is as wide as the word in it, and the only thing
/// that knows how wide a word is is the thing that will paint it.
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

/// Every renderer chip and its box, left to right in draw order — the same walk
/// [`rend_row_into`] paints and [`InspectorPane::select_renderer`] hit-tests,
/// so the capsule a press lands on is the capsule the wash is drawn in.
///
/// [`LibraryBay::chips`]' arrangement one bay along, down to the reason it is
/// an iterator: a `Vec` of rectangles would be an allocation on a path asked
/// once per pointer event and once per frame.
///
/// The index is the renderer's own, in draw order — the numbering
/// `Operation::SelectRenderer`, `--param L4:1:…` and a `select` record all use,
/// so what comes out of a press is a position in the Set rather than a position
/// in whatever this row managed to draw.
///
/// The boxes are not clipped and the paint is. `.rend-row` wraps in the mock
/// and this console draws one row of it (see [`rend_row_into`]), so a chip past
/// the pane's right edge is yielded whole here and held to the part of it that
/// is drawn where the press is answered.
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

/// Where each of the three authority chips goes on a node head, right-aligned
/// inside the head's own padding.
///
/// One derivation, asked twice — [`auth_into`] paints these and
/// [`InspectorPane::set_authority`] hit-tests them, which is [`rend_chips`]'
/// rule one row up: two copies of where a chip is would be a chip painted where
/// a hand cannot press it. The three were drawn and unclaimed from 2026-08-29
/// until the writer existed, and the running sum inside the painter was exactly
/// the shape a press had nowhere to ask about.
///
/// Right-aligned, so the whole row has to be measured before the first chip can
/// be placed: `.node-head`'s `.sep` pushes `.auth` to the end of the flex row.
///
/// # The node is taken because the `keep` capsule is at the same end
///
/// `.node-head` ends `.sep, .auth, .mini` — the capsule is hard against the
/// head's padding and the chips are stepped back from it — so where a chip goes
/// depends on whether this head carries one ([`node_keep`]). The trim is inside
/// this function rather than at its callers, because a caller that forgot it
/// would place three chips over the capsule, and the paint and the hit-test
/// would agree with each other and disagree with the mock. Two callers each
/// applying it correctly is a rule held by prose, which is exactly what
/// `docs/contributing.md` §4's structural tier is against.
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

/// Where a node head's `keep` capsule goes, or `None` on a head that carries
/// none.
///
/// The mock's `.node-head` is a flex row of the address, the name, a `.sep`,
/// the `.auth` chips and then `<span class="mini">keep</span>` — so the capsule
/// is hard against the head's right-hand padding and the chips are stepped back
/// from it by [`size::NODE_HEAD_GAP`], which is `.node-head`'s own `gap: 7px`.
/// That is why this is derived before [`auth_chips`] rather than beside it: the
/// chips are laid out inside what this leaves.
///
/// `None` on the two heads that carry no capsule — [`Node::keep`], where the
/// rule is written — and `None` on a head with no room for it, which is
/// [`keep_pill`]'s rule one row down: *a control that does not fit in the row
/// it is drawn in is no control at all, rather than half of one*.
///
/// One derivation, asked twice — [`node_into`] paints it and
/// [`InspectorPane::keep_procedure`] hit-tests it, which is [`auth_chips`]' own
/// rule: two copies of where a capsule is would be a capsule painted where a
/// hand cannot press it.
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

/// What is left of a node head for the authority chips — the head, less the
/// `keep` capsule and the gap before it where there is one.
///
/// One function because [`auth_into`] paints the chips and
/// [`InspectorPane::set_authority`] hit-tests them, and a head trimmed in one
/// of the two would be three chips drawn where a hand cannot press them. It is
/// [`node_keep`]'s other half: the two controls at the right of this row are
/// laid out from the right, the capsule first.
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
    // **The `keep` capsule at the right of the head**, and the chips above are
    // laid out inside what it leaves — [`node_keep`], which is where both
    // halves of that arithmetic are. A head that carries none draws none,
    // which is [`Node::keep`]'s two cases rather than a capsule that refuses.
    if let Some(pill) = node_keep(painter.ctx(), head, node) {
        // **A plain `.mini` and never `.sel`**, which is the mock's own and is
        // [`KeepPill`]'s note one row up read on a node: a keep is a press and
        // not a setting, so there is nothing here for a wash to be *on*. The
        // pane head's capsule carries one because it reads the deck's
        // residency, and a node has none.
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

    // **The inputs this node declares, under its head and above its rows**,
    // for the same reason and asked the same way: [`uses_rect`] and
    // [`uses_chip_in`] are where that arithmetic is written, and this paints
    // what they answer.
    for (index, uses) in node.uses.iter().enumerate() {
        uses_into(painter, pal, uses_rect(rect, index), uses);
    }

    // **Where the renderer row is, asked rather than measured here**: a press
    // has to resolve to the same rectangle the chips were drawn in, and
    // [`rend_row_in`] is the one place that arithmetic is written.
    if !node.renderers.is_empty() {
        rend_row_into(painter, pal, rend_row_in(rect, node), &node.renderers);
    }
    // **Where a row is, asked rather than accumulated.** The running sum this
    // loop used to keep was a second answer to the same question the moment a
    // press had to be resolved to a row — see [`param_rect`].
    for (index, param) in node.params.iter().enumerate() {
        param_into(painter, pal, param_rect(rect, node, index), param);
        // **Where the sensitivity row is, asked rather than measured here**,
        // which is the renderer row's rule one level up: a press has to
        // resolve to the same rectangle the chips were drawn in.
        if let (Some(row), Some(source)) = (sens_rect(rect, node, index), param.bound.as_ref()) {
            sens_into(painter, pal, row, source);
        }
    }
}

/// `man / sug / auto`, right-aligned on the node head, with the one the node is
/// on filled: `.auth span.sel`'s `color: var(--c-mint)` over a 15% wash of it,
/// and the other two in `var(--c-faint)` with no box at all.
///
/// All three and not only the one, which is the manual's own row: *"`man / sug
/// / auto` on each node head, never a global mode."* What is drawn is which of
/// the three this node is on, and all three are claimed: each names a
/// destination, which is what an operation on this panel is
/// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)),
/// and pressing the one a node is already on asks for what it already has — the
/// renderer row's rule one row down, and the anchor's two bays over. A chip
/// that stopped being pressable the moment it lit would take the claim out from
/// under a hand.
///
/// A head that folds more than one node draws none, which is `Node::authority`
/// being `None`: authority is per node, so one chip over three renderers would
/// be one of three answers drawn as *the* answer and a press on it would set
/// three nodes at once.
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

/// The renderer chips, one per renderer the Set has, from the left.
///
/// The live one carries `.rend.sel`: `color: var(--c-pink)` over a 15% wash of
/// it, no border, and the pink halo `box-shadow: 0 0 9px var(--c-glowp)` —
/// which is the same 9 the lit beat and a live fader knob carry. The rest are
/// `.rend`'s `border: 1px solid var(--c-line)` around `var(--c-dim)`.
///
/// One row and what fits of it. `.rend-row` wraps in the mock and the console
/// does not: a wrapped row is a group taller than [`group_h`] said it was, and
/// the pane's own arithmetic is what says whether a group is drawn at all. A
/// chip past the right-hand edge is clipped, which is the same answer the pane
/// gives a group past the bottom.
///
/// Where each chip goes is [`rend_chips`]', so this paints and derives nothing
/// — the change this pass made to it, and [`deck_head_into`]'s rule one row up:
/// the row used to be a running sum here and a press had nowhere to ask what it
/// had landed on.
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

/// One `uses` line: what the procedure calls the input at the left, and the
/// capsule naming the node filling it at the right.
///
/// The chevron is drawn rather than typed, which is [`CHEVRON_W`]'s reason
/// wherever this console draws a pulldown — the Outputs row's pill, the
/// arrangement pill and the Library bay's deck capsule all carry the same mark.
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
    painter.add(egui::Shape::convex_polygon(
        vec![
            chevron.left_top(),
            chevron.right_top(),
            Pos2::new(chevron.center().x, chevron.max.y),
        ],
        pal.dim,
        Stroke::NONE,
    ));
}

/// A `uses` line's card, painted — one row per node the input may be wired to,
/// with the one it is wired to now drawn in the panel's own text colour.
///
/// Where everything goes is [`UsesLine`]'s, so this paints and derives nothing,
/// which is [`deck_list_into`]'s own sentence one bay along. Drawn from
/// [`View::draw`] after the bays for that card's reason: it hangs out of a line
/// inside a pane and over the groups under it.
///
/// The node already wired is not in the list, so the *marked* row here is never
/// one of them — the ink says nothing about the current wiring and every row is
/// a change. That is why this is one colour where the deck pulldown's list is
/// two: a pulldown names where the *next* press lands and this one names where
/// the input goes.
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
        let galley = painter.layout_no_wrap(
            name.clone(),
            FontId::new(size::BASE, FontFamily::Proportional),
            pal.dim,
        );
        painter.galley(
            Pos2::new(
                row.min.x + size::LIB_ROW_PAD_X,
                row.center().y - galley.size().y * 0.5,
            ),
            galley,
            pal.dim,
        );
    }
}
