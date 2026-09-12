use super::*;

// ---------------------------------------------------------------------------
// The Master bay
// ---------------------------------------------------------------------------

/// The word at the head of the Master bay, in the source's own capitalisation
/// for [`Kind::Bay`]'s reason: the mock upper-cases in CSS, and that is done at
/// paint time so the word a reader searches for is the word in the source.
pub(super) const MASTER_TITLE: &str = "Master";

/// `.master-row`'s first item: the `out` before the track, which is the bay's
/// own word for the level and not the engine's — `Deck::out` is what it moves.
const MASTER_LABEL: &str = "out";

/// The out row, laid out: the word, the fader and the figure.
///
/// # One derivation, for [`Outputs`]' reason
///
/// [`View::draw`] paints exactly these rectangles and [`crate::input::claim`]
/// hit-tests exactly this knob. Two copies of the arithmetic is a knob painted
/// where a hand cannot take hold of it.
///
/// # It is the bay's whole body, and the rest of the bay is not built
///
/// `docs/manual/console.html` draws three effects under this row — feedback,
/// bloom and rgb shift — and the word `master` appears nowhere in
/// `karakuri-engine` except at the level this row moves
/// ([ADR-0224](../../../../docs/adr/0224-out-and-exposure-are-two-levels-that-multiply-in-different-places.md)).
/// There is no chain, so there is nothing to draw a chain from: a row of
/// effects over machinery that does not exist is the scaffolding this module's
/// documentation refuses, and the bay's card shows through under this row
/// exactly as it does in every other empty body.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MasterRow {
    /// The faint `out` at the head of the row.
    pub label: Rect,
    /// The fader: `.fader`'s 5px well lying down, what the level fills of it, and
    /// the knob centred on the fill's moving edge.
    pub fader: Fader,
    /// `1.00`, at the far end of the row and in a box that does not move.
    ///
    /// As wide as the widest reading this control can ask for rather than as wide
    /// as the one it is showing, which is [`LookRow::tone`]'s rule met by a figure
    /// instead of by a word: the track between the label and this box is what is
    /// left over, so a figure that changed width as it was dragged would take the
    /// track — and the knob on it — with it.
    pub value: Rect,
    /// The value these rectangles were measured from, carried for
    /// [`LookRow::values`]' reason: whoever measured the type and whoever paints it
    /// are one statement.
    pub out: f32,
    /// The master chain's three rows, in the chain's own order — feedback, bloom,
    /// rgb shift — and `None` for one there is no room for.
    ///
    /// A fixed three and not a list, because the chain is fixed: three built-in
    /// passes, all of them loaded, in that order
    /// ([ADR-0317](../../../../docs/adr/0317-the-master-chain-is-three-fixed-passes-and-feedback-reads-either-cut.md)).
    /// A `Vec` here would be a claim that the count can change, which is the `+
    /// add` row's question and is not this one's.
    ///
    /// A row drops out from the bottom up when the bay is short, on
    /// [`mixer::strips_row`]'s rule: the arrangement's own minimum for this bay
    /// keeps room for the out row and one effect, so a bay at its minimum draws
    /// one.
    pub fx: [Option<FxRow>; 3],
}

/// Which pass of the master chain a row is, in the chain's order.
///
/// The order is the engine's and is not a preference: feedback reads the
/// previous frame, bloom spreads what is over the knee, and rgb shift is last
/// so the other two are seen through it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fx {
    Feedback,
    Bloom,
    RgbShift,
}

impl Fx {
    /// The three, in the order the bay draws them.
    pub const ALL: [Fx; 3] = [Fx::Feedback, Fx::Bloom, Fx::RgbShift];

    /// The word the row draws, which is the mock's own and the operations page's
    /// heading in lower case.
    pub fn name(self) -> &'static str {
        match self {
            Fx::Feedback => "feedback",
            Fx::Bloom => "bloom",
            Fx::RgbShift => "rgb shift",
        }
    }
}

/// One effect row of the master chain, laid out: the dot, the word, the
/// feedback row's cut chip, the track and the figure.
///
/// `.fx` in `docs/manual/console.html` — a well with `FX_PAD_X` either side and
/// `FX_PAD_Y` above and below, its items [`size::FX_GAP`] apart, the track
/// taking what is left between the word and the figure exactly as the out row's
/// does.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FxRow {
    /// Which pass, which is what decides the word and the operation.
    pub fx: Fx,
    /// The well the row is drawn in.
    pub well: Rect,
    /// `.fx .dot`: lit when this pass is in the frame.
    pub dot: Rect,
    /// The word.
    pub name: Rect,
    /// The cut chip, on the feedback row and on neither of the others — a `.mini`,
    /// the mixer's own blend chip: the same 9px word inside the same padding,
    /// because it is the same thing, one value of a closed list shown and cycled.
    ///
    /// `None` is *this row has no second parameter*, and it is the shape that says
    /// so: an always-present rectangle nobody draws would be a control two rows can
    /// be pressed on.
    pub cut: Option<Rect>,
    /// The track.
    pub fader: Fader,
    /// The figure, in a box as wide as the widest reading — [`MasterRow::value`]'s
    /// rule and its reason.
    pub value: Rect,
    /// What this pass is set to, `[0, 1]` of its own reach. Carried for
    /// [`MasterRow::out`]'s reason.
    pub amount: f32,
    /// Which cut the feedback pass is reading. Carried on every row because the
    /// chip is laid out from it and the operation a drag asks for needs it;
    /// meaningless on the other two, which is why only the feedback row has a chip
    /// to draw it in.
    pub reading: karakuri_operation::Cut,
    /// Whether this pass is in the frame at all. An amount of zero is not a pass
    /// multiplying by nothing: no pass is recorded, so the row is dim and the dot
    /// is out.
    pub runs: bool,
}

impl FxRow {
    /// What a drag on this row asks for. The amount is the track's position, and
    /// the feedback row's knob carries the cut beside it because the operation is
    /// the whole of what the pass is set to.
    pub fn knob(&self) -> Knob {
        match self.fx {
            Fx::Feedback => Knob::Feedback { cut: self.reading },
            Fx::Bloom => Knob::Bloom,
            Fx::RgbShift => Knob::RgbShift,
        }
    }

    /// What a press on the cut chip asks for, or `None` where `p` is not on one —
    /// which is every point of the two rows that have no chip.
    ///
    /// The next cut and not a step, which is the difference between the affordance
    /// and the operation: the chip cycles because a surface may, and what it emits
    /// names where the pass is going
    /// (`docs/principles/0090-a-surface-offers-it-never-decides.md`). The list is
    /// two long and the cycle is this crate's arithmetic over it, exactly as the
    /// blend chip's is.
    pub fn chip(&self, p: karakuri_layout::Point) -> Option<Operation> {
        let chip = self.cut?;
        if !chip.contains(Pos2::new(p.x, p.y)) {
            return None;
        }
        let at = karakuri_operation::Cut::ALL
            .iter()
            .position(|c| *c == self.reading)
            .unwrap_or(0);
        let next = karakuri_operation::Cut::ALL[(at + 1) % karakuri_operation::Cut::ALL.len()];
        Some(Operation::SetFeedback {
            params: karakuri_operation::Feedback {
                amount: self.amount * karakuri_operation::Feedback::MAX,
                cut: next,
            },
        })
    }
}

/// What the master chain is running at, as the Master bay reads it.
///
/// `karakuri_engine::master::Chain` mirrored into this crate for the reason
/// every mirrored list here is mirrored: the panel depends on the vocabulary
/// and on no engine. Four values, because a row that drew one without the
/// others could not build the operation the whole chain's record is made from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Chain {
    /// `[0, 1]` of `karakuri_operation::Feedback::MAX`, which is the track's own
    /// position — a fader draws a position and the operation carries the amount.
    pub feedback: f32,
    pub cut: karakuri_operation::Cut,
    pub bloom: f32,
    pub rgb_shift: f32,
}

impl MasterRow {
    /// What a press at `p` takes hold of, or `None` where there is nothing under it
    /// that a hand can move.
    ///
    /// # It is the knob, and the track is deliberately not a target
    ///
    /// [`Mixer::grab`]'s rule, and this control is the one on the panel it is most
    /// obviously right for: a master out at 0.3 whose track was clicked would put
    /// the whole programme at 1.0, on stage, because a hand landed three pixels off
    /// a knob.
    ///
    /// It is not [`LookRow::exposure`]'s rule, and the two do not disagree. That
    /// control has no handle drawn and no gesture to be mid-way through, so a press
    /// is the whole of it. This one has a handle — the mock draws the `.fader s`
    /// here and deliberately draws none on the exposure track — and a handle that
    /// jumped to the pointer would be a lie about what a handle is.
    ///
    /// # One derivation, asked twice
    ///
    /// [`crate::input::claim`] asks this and so does the caller that acts on the
    /// press, exactly as [`Mixer::grab`] is. The value is part of the geometry: the
    /// knob sits on the fill's moving edge, so where it is depends on what the deck
    /// said this frame, and this is the same reading the row was laid out from.
    pub fn grab(&self, p: karakuri_layout::Point) -> Option<Grab> {
        let at = Pos2::new(p.x, p.y);
        grabbed(self.fader, Knob::Out, at).or_else(|| {
            self.fx
                .iter()
                .flatten()
                .find_map(|row| grabbed(row.fader, row.knob(), at))
        })
    }

    /// What a press on the feedback row's cut chip asks for, or `None`. The bay's
    /// one control that is not a fader — see [`FxRow::chip`].
    pub fn chip(&self, p: karakuri_layout::Point) -> Option<Operation> {
        self.fx.iter().flatten().find_map(|row| row.chip(p))
    }

    /// Whether `p` is on one of the things here a hand can move, which is what
    /// [`crate::input::claim`] asks — the four knobs and the cut chip, and not the
    /// tracks under them.
    pub fn owns(&self, p: karakuri_layout::Point) -> bool {
        self.grab(p).is_some() || self.chip(p).is_some()
    }
}

/// The Master bay's out row, derived: the word, the fader and the figure.
///
/// # Where it sits
///
/// `docs/manual/console.html`'s `.master-body` is a column inside the bay,
/// under the head, inset by [`size::MASTER_PAD_X`] either side and
/// [`size::MASTER_PAD_TOP`] from the head; `.master-row` is a flex row of three
/// items, [`size::MASTER_GAP`] apart, with the fader taking what is left
/// between the label and the figure. That is the mock term for term, and the
/// arrangement's own minimum for this bay in `lib.rs` is written from the same
/// numbers — *"bay head 27, `.master-body` padding 8 + 10, the out row 16.5"*.
///
/// # The figure's box is fixed and the track is what flexes
///
/// The mock gives the fader `flex: 1` and puts the figure after it, so the
/// track's far end is wherever the figure begins. A figure sized to what it
/// says would therefore move the track *while the track is being dragged*,
/// which is [`look`]'s own argument about the exposure's number met here by a
/// control that has a handle: there the figure could simply go last, here it is
/// between the track and the bay's edge. So the box is as wide as the widest
/// reading this control can ask for.
///
/// The widest is measured and not assumed: all ten `d.dd` strings are laid out
/// and the widest of them wins, because whether `0.00` is wider than `1.11` is
/// a fact about whatever font the room is drawn in and not one to take on trust
/// ([`docs/contributing.md`](../../../../docs/contributing.md) §1). Ten cached
/// layouts of four characters, on a pointer event and on a frame.
///
/// A reading outside `[0, 1]` is the one case it does not cover, and it is
/// stated rather than guarded: `Deck::set_out` is open above 1.0 and this drag
/// tops out at exactly 1.00, so nothing can put a fifth character in the box
/// today. If something does, the figure is right-aligned and grows back over
/// the track's end rather than out past the bay's padding — which keeps the row
/// inside the card, and is the reason it is right-aligned rather than the
/// reason the box is this wide.
///
/// # None where there is nothing to draw
///
/// `None` for a console with no engine behind it — which is every test in this
/// crate that does not hand a level in — and `None` for a bay with no room for
/// the row, which is [`mixer::strips_row`]'s rule one bay up: folded away,
/// soloed away, or a window too small.
///
/// # What it costs to ask
///
/// Eleven galley lookups: the `out` label, and the ten `d.dd` strings the
/// widest is taken over. The ten are what buys a track that does not move under
/// a hand, and they are ten *cached* layouts of four characters. The reading
/// itself is not among them, which is the whole of why the box does not move:
/// nothing in this derivation lays out the level. Paid on a pointer event and
/// on a frame, and a console with no level behind it pays none of it: the
/// `out?` is the first line.
///
/// `layout` must be solved: [`Layout::rect`] refuses to answer from a dirty
/// one.
pub fn master(
    ctx: &egui::Context,
    layout: &karakuri_layout::Layout,
    out: Option<f32>,
    chain: Option<Chain>,
) -> Option<MasterRow> {
    let out = out?;
    // Fonts are not valid until `egui` has run a pass, exactly as in `mixer`,
    // `outputs` and `transport`.
    if ctx.cumulative_pass_nr() == 0 {
        return None;
    }
    let region = to_egui(layout.rect(layout.find("master")?));
    let width = |text: &str| {
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
    let row = Rect::from_min_size(
        Pos2::new(
            region.min.x + size::MASTER_PAD_X,
            region.min.y + size::HEAD_H + size::MASTER_PAD_TOP,
        ),
        egui::vec2(
            region.width() - size::MASTER_PAD_X * 2.0,
            size::MASTER_ROW_H,
        ),
    );
    if row.width() <= 0.0 || !region.contains_rect(row) {
        return None;
    }

    // **The widest `d.dd` there is**, which is every reading this control can
    // ask for and is measured rather than assumed: whether `0.00` is wider
    // than `1.11` is a fact about a font, so all ten are laid out and the
    // widest wins. Nothing here reads the level, which is the point — see the
    // paragraph above.
    let widest = (0..10)
        .map(|d| width(&format!("{d}.{d}{d}")))
        .fold(0.0, f32::max);

    let label = Rect::from_min_size(row.min, egui::vec2(width(MASTER_LABEL), row.height()));
    let value = Rect::from_min_size(
        Pos2::new(row.max.x - widest, row.min.y),
        egui::vec2(widest, row.height()),
    );
    let mid = row.center().y;
    let track = Rect::from_min_max(
        Pos2::new(label.max.x + size::MASTER_GAP, mid - size::FADER_H * 0.5),
        Pos2::new(value.min.x - size::MASTER_GAP, mid + size::FADER_H * 0.5),
    );
    // **A track with no length is no control**, which is `Grab::new`'s own
    // refusal one crate layer down and `strip_box`'s rule one bay up: a bay
    // narrow enough that the word and the figure meet has nothing left to
    // draw a fader in, and half a fader is worse than none.
    if track.width() <= 0.0 {
        return None;
    }

    // **The three effect rows, under the out row and off the same width.**
    // `None` for a chain nothing is behind — a console with no engine draws
    // the out row it was handed a level for and nothing under it — and `None`
    // per row for one the bay is too short to hold.
    let mut fx = [None, None, None];
    if let Some(chain) = chain {
        let mut top = row.max.y + size::MASTER_STACK_GAP;
        for (at, kind) in Fx::ALL.into_iter().enumerate() {
            let well = Rect::from_min_size(
                Pos2::new(row.min.x, top),
                egui::vec2(row.width(), size::FX_H),
            );
            if !region.contains_rect(well) {
                break;
            }
            fx[at] = fx_row(ctx, kind, well, chain, widest, &width);
            top = well.max.y + size::MASTER_STACK_GAP;
        }
    }

    Some(MasterRow {
        label,
        // `.fader b` fills its 5px track edge to edge, so there is no inset —
        // the trim's arrangement, and not `.vfader`'s.
        fader: fader(
            track,
            Axis::Row,
            out,
            0.0,
            egui::vec2(size::FADER_KNOB_W, size::FADER_KNOB_H),
        ),
        value,
        out,
        fx,
    })
}

/// One effect row, laid out inside `well`.
///
/// The out row's arrangement one line down and inside a padded well: the dot,
/// the word, the feedback row's chip, the track taking what is left, and the
/// figure in a box that does not move. `widest` is the out row's own
/// measurement of the widest `d.dd`, passed in rather than taken again — the
/// figures are the same shape and one measurement is what keeps the two rows'
/// boxes the same width.
fn fx_row(
    ctx: &egui::Context,
    fx: Fx,
    well: Rect,
    chain: Chain,
    widest: f32,
    width: &dyn Fn(&str) -> f32,
) -> Option<FxRow> {
    // The chip's word is 9px where everything else on the row is `BASE`, so
    // this row needs the one measurement `master`'s own closure cannot give it.
    let width_at = |text: &str, size: f32| {
        ctx.fonts_mut(|f| {
            f.layout_no_wrap(
                text.to_owned(),
                FontId::new(size, FontFamily::Proportional),
                Color32::PLACEHOLDER,
            )
            .size()
            .x
        })
    };
    let amount = match fx {
        Fx::Feedback => chain.feedback,
        Fx::Bloom => chain.bloom,
        Fx::RgbShift => chain.rgb_shift,
    };
    let inner = Rect::from_min_max(
        Pos2::new(well.min.x + size::FX_PAD_X, well.min.y + size::FX_PAD_Y),
        Pos2::new(well.max.x - size::FX_PAD_X, well.max.y - size::FX_PAD_Y),
    );
    if inner.width() <= 0.0 {
        return None;
    }
    let mid = inner.center().y;
    let dot = Rect::from_center_size(
        Pos2::new(inner.min.x + size::FX_DOT * 0.5, mid),
        egui::vec2(size::FX_DOT, size::FX_DOT),
    );
    let name = Rect::from_min_size(
        Pos2::new(dot.max.x + size::FX_GAP, inner.min.y),
        egui::vec2(width(fx.name()), inner.height()),
    );
    // **The chip is on the feedback row and on neither of the others**, and it
    // is as wide as the wider of the two words rather than as wide as the one
    // it is showing — the figure's rule at the other end of the row, for the
    // same reason: a chip that changed width when it was pressed would move
    // the track it sits beside.
    let cut = (fx == Fx::Feedback).then(|| {
        // **A `.mini`, the mixer's own blend chip** — the same 9px word inside
        // the same padding and the same border, because it is the same thing:
        // one value of a closed list, shown and cycled.
        let word = karakuri_operation::Cut::ALL
            .iter()
            .map(|c| width_at(c.name(), size::MINI_SIZE))
            .fold(0.0, f32::max);
        let chip = word + size::MINI_PAD_X * 2.0 + size::HAIRLINE * 2.0;
        Rect::from_center_size(
            Pos2::new(name.max.x + size::FX_GAP + chip * 0.5, mid),
            egui::vec2(chip, size::MINI_H),
        )
    });
    let value = Rect::from_min_size(
        Pos2::new(inner.max.x - widest, inner.min.y),
        egui::vec2(widest, inner.height()),
    );
    let after = cut.map_or(name.max.x, |c| c.max.x);
    let track = Rect::from_min_max(
        Pos2::new(after + size::FX_GAP, mid - size::FADER_H * 0.5),
        Pos2::new(value.min.x - size::FX_GAP, mid + size::FADER_H * 0.5),
    );
    // **A track with no length is no control** — [`master`]'s own refusal, and
    // a row without one is not drawn at all rather than drawn half.
    if track.width() <= 0.0 {
        return None;
    }
    Some(FxRow {
        fx,
        well,
        dot,
        name,
        cut,
        fader: fader(
            track,
            Axis::Row,
            amount,
            0.0,
            egui::vec2(size::FADER_KNOB_W, size::FADER_KNOB_H),
        ),
        value,
        amount,
        reading: chain.cut,
        // **The one thing a row says that is not a number**: an amount of zero
        // is the pass not being recorded at all, so the row is dim and the dot
        // is out. See `karakuri_engine::master`.
        runs: amount > 0.0,
    })
}

/// The level, as the mock's `.val` writes it — `1.00`, two places, and the same
/// string the transport row's exposure is written with. The two are the same
/// kind of reading and deliberately read the same way; where they stop being
/// the same *number* is ADR-0224.
fn master_text(out: f32) -> String {
    format!("{out:.2}")
}

/// The out row, painted.
///
/// Where everything goes is [`master`]'s, so this paints and derives nothing.
/// Term for term from `style.css`:
///
/// - the `out` before the track — `style="color:var(--c-faint)"` in the
///   markup, which is `pal.faint`, and it is `.trim .lbl`'s job one bay up.
/// - `.fader`, `.fader b` and `.fader s` — [`fader_into`], which is the one
///   place a knob, a well and a fill are drawn and is what the mixer's own
///   trim is painted with. Not live and never reaching: the pink glow is a
///   slot on air and this level belongs to no slot, and a scheduled move is
///   per slot too — `Deck::set_out` takes no `cancel` because nothing can be
///   moving it (ADR-0224).
/// - the figure — `.val`, `pal.text`, *a value*.
pub(super) fn master_into(ui: &Ui, pal: &Palette, row: &MasterRow) {
    let painter = ui.painter();
    let centred = |rect: Rect, galley: std::sync::Arc<egui::Galley>, colour: Color32| {
        painter.galley(
            Pos2::new(rect.min.x, rect.center().y - galley.size().y * 0.5),
            galley,
            colour,
        );
    };
    centred(
        row.label,
        painter.layout_no_wrap(
            MASTER_LABEL.to_owned(),
            FontId::new(size::BASE, FontFamily::Proportional),
            pal.faint,
        ),
        pal.faint,
    );
    fader_into(painter, pal, row.fader, false, None);
    // **Right-aligned in a box that does not move**, so the figure ends at the
    // bay's padding whatever it says — which is what makes the box's width the
    // widest reading rather than this one's.
    let words = master_text(row.out);
    let galley = painter.layout_no_wrap(
        words,
        FontId::new(size::BASE, FontFamily::Proportional),
        pal.text,
    );
    painter.galley(
        Pos2::new(
            row.value.max.x - galley.size().x,
            row.value.center().y - galley.size().y * 0.5,
        ),
        galley,
        pal.text,
    );
    for fx in row.fx.iter().flatten() {
        fx_into(ui, pal, fx);
    }
}

/// One effect row, painted. Term for term from `style.css`:
///
/// - `.fx` — a `--c-well` recess with an 8px radius.
/// - `.fx.sel` — `--c-text` and an inset mint ring, which on this bay means
///   this pass is in the frame: an amount above zero, so it is recorded
///   and it costs its passes. `.fx.off` is `--c-faint`, which is the same
///   sentence the other way round.
/// - `.fx .dot` — mint with a glow when the pass runs, `--c-faint` and no glow
///   when it does not.
/// - the cut chip — `.mini`, the mixer's own blend chip, on the feedback row
///   alone.
/// - `.fx .amt` — `--c-text`, right-aligned in a box that does not move.
pub(super) fn fx_into(ui: &Ui, pal: &Palette, row: &FxRow) {
    let painter = ui.painter();
    painter.rect_filled(row.well, CornerRadius::same(size::FX_RADIUS), pal.well);
    if row.runs {
        painter.rect_stroke(
            row.well,
            CornerRadius::same(size::FX_RADIUS),
            Stroke::new(size::HAIRLINE, pal.mint),
            StrokeKind::Inside,
        );
    }
    let ink = if row.runs { pal.text } else { pal.faint };
    painter.circle_filled(
        row.dot.center(),
        size::FX_DOT * 0.5,
        if row.runs { pal.mint } else { pal.faint },
    );
    let word = |rect: Rect, text: &str, colour: Color32| {
        let galley = painter.layout_no_wrap(
            text.to_owned(),
            FontId::new(size::BASE, FontFamily::Proportional),
            colour,
        );
        painter.galley(
            Pos2::new(rect.min.x, rect.center().y - galley.size().y * 0.5),
            galley,
            colour,
        );
    };
    word(row.name, row.fx.name(), ink);
    // **`.mini.sel`**, which is the mixer's blend chip exactly: a lavender
    // wash and a lavender word, because what it says is *this is the one
    // chosen* and lavender is this console's ink for a selection.
    if let Some(chip) = row.cut {
        mini_into(painter, pal, chip, true, |painter, colour| {
            let galley = painter.layout_no_wrap(
                row.reading.name().to_owned(),
                FontId::new(size::MINI_SIZE, FontFamily::Proportional),
                colour,
            );
            painter.galley(
                Pos2::new(
                    chip.center().x - galley.size().x * 0.5,
                    chip.center().y - galley.size().y * 0.5,
                ),
                galley,
                colour,
            );
        });
    }
    fader_into(painter, pal, row.fader, false, None);
    let galley = painter.layout_no_wrap(
        master_text(row.amount),
        FontId::new(size::BASE, FontFamily::Proportional),
        ink,
    );
    painter.galley(
        Pos2::new(
            row.value.max.x - galley.size().x,
            row.value.center().y - galley.size().y * 0.5,
        ),
        galley,
        ink,
    );
}
