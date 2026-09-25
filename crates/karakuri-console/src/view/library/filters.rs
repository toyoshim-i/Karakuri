use super::*;

// ---------------------------------------------------------------------------
// The library's filter field and its kind chips
// ---------------------------------------------------------------------------

/// What the `holds` field reads with no filter set, which is the mock's own
/// `holds&hellip;` — the ellipsis is the placeholder saying the field is empty,
/// and a field that *is* set reads the value instead. That is the whole of the
/// difference between the two states, because `.field` has one rule in
/// `style.css` and no set variant: inventing a second colour for it would be a
/// reading the mock never took.
pub const HOLDS_UNSET: &str = "holds…";

/// The layers, in the order the kind chips draw them and with the word each
/// chip carries.
///
/// Curated here because the vocabulary has no list to cycle, which is
/// [`WIPE_SHAPES`]' whole argument three bays along:
/// `karakuri_operation::Layer` is an enum with no `ALL` and no word for a
/// member, so a control that draws it has to say which order and which
/// spelling. The spellings are the five the MCP `list_sets` tool takes and the
/// five the Inspector's `.addr` writes — `L1:0`, `L4` — so a kind a hand
/// presses and a layer a model names are the same five words.
///
/// It was the `layer` field's cycle until 2026-09-10, and what reads it now is
/// [`KindChip`]: ADR-0338 replaced that field with six toggles, and the list of
/// five outlived the control because the list was never what was wrong with it
/// — *"ADR-0262 was right about the list and wrong about the control, and what
/// changed is that the list grew a sixth member which is not a layer at all"*.
///
/// Five and not every [`Layer`] there is, since `kind L5` joined the vocabulary
/// (ADR-0340): the word for it, the seventh chip and the badge are that
/// record's own pass, which draws them here and in the mock together. An array
/// is not a match, so a variant added to the vocabulary does not fail to
/// compile here — this is the line to read the day one lands, and
/// `tests/library.rs` counts them.
pub const LAYERS: [(Layer, &str); 5] = [
    (Layer::L1, "L1"),
    (Layer::L2, "L2"),
    (Layer::L3, "L3"),
    (Layer::L4, "L4"),
    (Layer::Field, "FIELD"),
];

/// The word the `SET` chip carries, and the one place it is spelled — the sixth
/// of the six, and the only one that is not a [`Layer`].
pub const SETS_CHIP: &str = "SET";

/// One chip of the kind filter row: a layer, or the Sets.
///
/// # Six and not five, because a Set is not a kind
///
/// A procedure declares one of [`Layer`]'s five kinds; a Set fills several and
/// declares none. So *is this its kind* is a question a procedure answers and a
/// Set does not, and the sixth chip asks the other question — *show me the
/// Sets* — which is why the row is a partition of the rows rather than a filter
/// over one kind of them
/// (`docs/adr/0338-a-procedure-is-a-row-of-the-library-and-one-loaded-over-a-layer-makes-a-set-with-no-name.md`).
///
/// # A toggle, and the arithmetic is this console's
///
/// [`karakuri_operation::LibraryKinds`] is six named booleans and says nothing
/// about a press ([P-0090]). A press here flips this chip's field and sends all
/// six — [`LibraryBay::kind`] — because what leaves has to be a destination
/// rather than a step: six statements each saying *this one changed* are six
/// things a second surface can arrive in the middle of.
///
/// [P-0090]: ../../../docs/principles/0090-a-surface-offers-it-never-decides.md
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KindChip {
    /// One of the five kinds a procedure declares.
    Layer(Layer),
    /// The Sets, which is the row kind that declares no `kind` at all.
    Sets,
}

impl KindChip {
    /// The six, left to right in the order `.lib-kinds` draws them — the five of
    /// [`LAYERS`] and the Sets last, which is the mock's own `L1 L2 L3 L4 FIELD
    /// SET`.
    ///
    /// A seventh for `kind L5` is ADR-0340's, whose own pass draws it here and in
    /// the mock together: a control the panel draws that the mock does not is a
    /// defect (`docs/contributing.md` §5), and the payload already carries the
    /// field it would set — [`KindChip::on`] reads it.
    pub const ALL: [KindChip; 6] = [
        KindChip::Layer(Layer::L1),
        KindChip::Layer(Layer::L2),
        KindChip::Layer(Layer::L3),
        KindChip::Layer(Layer::L4),
        KindChip::Layer(Layer::Field),
        KindChip::Sets,
    ];

    /// The word in the chip — [`LAYERS`]' spelling, or [`SETS_CHIP`].
    ///
    /// A layer with no entry in [`LAYERS`] reads [`SETS_CHIP`], which is
    /// unreachable while that list is exhaustive and is the fallback rather than a
    /// panic for [`Filters::holds_word`]'s reason: a bay draws what it was handed,
    /// and a missing word is a chip an operator cannot read rather than a run that
    /// stops.
    pub fn word(self) -> &'static str {
        match self {
            KindChip::Sets => SETS_CHIP,
            KindChip::Layer(layer) => LAYERS
                .iter()
                .find(|(kind, _)| *kind == layer)
                .map_or(SETS_CHIP, |(_, word)| *word),
        }
    }

    /// Whether this chip is on, read off the payload the host was last handed — the
    /// mint `.kind.on` against the plain `.kind`.
    ///
    /// It is the chip's own field and never [`LibraryKinds::narrowing`]: a row with
    /// nothing on shows everything, and drawing all six lit for it would say six
    /// presses had been made.
    pub fn on(self, kinds: LibraryKinds) -> bool {
        match self {
            KindChip::Sets => kinds.sets,
            KindChip::Layer(Layer::L1) => kinds.l1,
            KindChip::Layer(Layer::L2) => kinds.l2,
            KindChip::Layer(Layer::L3) => kinds.l3,
            KindChip::Layer(Layer::L4) => kinds.l4,
            KindChip::Layer(Layer::Field) => kinds.field,
            // **Answered, and not yet drawn.** `KindChip` is parameterised by
            // [`Layer`], so the sixth kind gives it a variant by existing;
            // whether the filter row shows a seventh chip is
            // [`KindChip::ALL`]'s question and is the pass that gives the
            // chain its slots.
            KindChip::Layer(Layer::L5) => kinds.l5,
        }
    }

    /// All six with this one turned the other way, which is what a press on it asks
    /// for — the surface's arithmetic, and the whole of it.
    pub fn flipped(self, kinds: LibraryKinds) -> LibraryKinds {
        let mut kinds = kinds;
        let want = !self.on(kinds);
        match self {
            KindChip::Sets => kinds.sets = want,
            KindChip::Layer(Layer::L1) => kinds.l1 = want,
            KindChip::Layer(Layer::L2) => kinds.l2 = want,
            KindChip::Layer(Layer::L3) => kinds.l3 = want,
            KindChip::Layer(Layer::L4) => kinds.l4 = want,
            KindChip::Layer(Layer::Field) => kinds.field = want,
            KindChip::Layer(Layer::L5) => kinds.l5 = want,
        }
        kinds
    }
}

/// Which filter field a press landed on, and there is one.
///
/// # It has one variant, and it is an enum rather than nothing
///
/// `Field::Layer` was the second until 2026-09-10, when ADR-0338 replaced the
/// `layer…` field with [`KindChip`]'s six toggles — *"a filter over a closed
/// list of six is a set of toggles, because every subset of six is askable and
/// a position in a cycle can only ever name one"*. What is left is `holds…`,
/// which is untouched and is still a first cut (ADR-0292 took its premise away
/// and ADR-0338 does not answer it).
///
/// Kept as a name rather than collapsed into the method, for [`Knob`]'s reason:
/// [`LibraryBay::field`] and [`LibraryBay::filter`] are asked *which field*,
/// the row is drawn from a list, and a caller passing nothing would have to be
/// edited again the day the row grows the filter this one is a first cut of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    /// `holds…` — what a node in the Set is called.
    Holds,
}

impl Field {
    /// The row's fields, left to right, in the order `.lib-filters` draws them.
    pub const ALL: [Field; 1] = [Field::Holds];
}

/// What the two filter fields are narrowing the listing to, read off the
/// console for the frame that draws them and the press that changes them.
///
/// It is [`Operation::ListSets`]'s two payload fields, borrowed: `holds` points
/// into [`View::holds`], which is the row of candidates the host handed in, so
/// nothing is cloned to draw a frame and the `String` is built once, at the
/// press, where the operation is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Filters<'a> {
    /// Part of a node's name, or `None` for a field nobody has set.
    pub holds: Option<&'a str>,
    /// Which kinds of row the listing shows, which is the six chips under the field
    /// — [`karakuri_operation::LibraryKinds`], and [`LibraryKinds::EVERYTHING`]
    /// where none of them is on.
    ///
    /// A payload and not a position, where `layer` beside it was an `Option<Layer>`
    /// until 2026-09-10: a cycle can name six of the sixty-four states this row
    /// has, and a filter over a closed list of six is every subset of it
    /// (ADR-0338).
    pub kinds: LibraryKinds,
}

impl Filters<'_> {
    /// Nothing narrowed at all, which is where a run begins and is every test in
    /// this crate that does not say otherwise.
    pub const NONE: Filters<'static> = Filters {
        holds: None,
        kinds: LibraryKinds::EVERYTHING,
    };

    /// What the `holds` field reads: the filter, or [`HOLDS_UNSET`].
    pub fn holds_word(&self) -> &str {
        self.holds.unwrap_or(HOLDS_UNSET)
    }

    /// The word this field reads.
    pub fn word(&self, field: Field) -> &str {
        match field {
            Field::Holds => self.holds_word(),
        }
    }
}

/// Where the `holds` filter goes when the field is pressed: the next candidate
/// along, and off the end back to nothing.
///
/// `choices` is [`View::holds`] — the node names the host read off the store —
/// and a filter that is not among them steps to the first, which is the state a
/// host that rewrote the candidates leaves behind.
///
/// With no candidates at all it stays `None`, and the press is still answered:
/// what it asks for is the listing again, unnarrowed, which is the same
/// question `LibraryBay::chip` answers when the chip already marked is pressed.
fn stepped_holds(choices: &[String], at: Option<&str>) -> Option<String> {
    let next = match at {
        None => 0,
        Some(at) => choices
            .iter()
            .position(|choice| choice == at)
            .map_or(0, |at| at + 1),
    };
    choices.get(next).cloned()
}

impl LibraryBay {
    /// One filter field's box, or `None` for a bay with no filter row.
    ///
    /// `.field` carries `flex: 1; min-width: 0` and nothing else is in the row, so
    /// it takes the whole of `.lib-filters`'s content box — which is a division
    /// rather than a measurement, and is why this bay still asks `egui` for
    /// nothing. The chips above and below it are the exception and stay the
    /// exception: a chip is as wide as the word in it and a field is not.
    ///
    /// It shared the row with `layer…` until 2026-09-10, each taking half of what
    /// was left after one [`size::LIB_FILTERS_GAP`]. ADR-0338 retired that field
    /// for the six chips in [`LibraryBay::kinds`], which are their own row under
    /// this one — six toggles and a `flex: 1` field on one row in a bay this narrow
    /// would leave the chips a few pixels each, and a chip nobody can hit is not a
    /// control (`style.css`, `.lib-kinds`).
    pub fn field(&self, which: Field) -> Option<Rect> {
        let row = self.filters?;
        let width = row.width() - size::LIB_FILTERS_PAD_X * 2.0;
        let at = match which {
            Field::Holds => row.min.x + size::LIB_FILTERS_PAD_X,
        };
        Some(Rect::from_min_size(
            Pos2::new(at, row.min.y + size::LIB_FILTERS_PAD_Y),
            egui::vec2(width, size::FIELD_H),
        ))
    }

    /// One kind chip and its box, left to right in [`KindChip::ALL`]'s order — the
    /// same walk `library_into` paints and [`LibraryBay::kind`] hit-tests, so the
    /// capsule a press lands on is the capsule the mint is drawn in.
    ///
    /// A chip is as wide as the word in it, which is [`LibraryBay::chips`]'
    /// sentence one row up and the second place this bay has to ask `egui`
    /// anything. `.kind` is `font-size: 9px; padding: 0 6px` with a hairline
    /// border, so the box is the word at [`size::KIND_SIZE`] inside
    /// [`size::KIND_PAD_X`] either side.
    ///
    /// Empty for a bay with no kind row, which is a bay with no filter row — see
    /// [`LibraryBay::kinds`], where the one condition is written.
    pub fn kind_chips<'a>(
        &self,
        ctx: &'a egui::Context,
    ) -> impl Iterator<Item = (KindChip, Rect)> + 'a {
        let row = self.kinds;
        let mut x = row.map_or(0.0, |row| row.min.x + size::LIB_KINDS_PAD_X);
        let top = row.map_or(0.0, |row| row.min.y + size::LIB_KINDS_PAD_Y);
        let drawn = row.map_or(0, |_| KindChip::ALL.len());
        KindChip::ALL.into_iter().take(drawn).map(move |chip| {
            let width = if ctx.cumulative_pass_nr() == 0 {
                size::KIND_PAD_X * 2.0
            } else {
                ctx.fonts_mut(|f| {
                    f.layout_no_wrap(
                        chip.word().to_owned(),
                        FontId::new(size::KIND_SIZE, FontFamily::Proportional),
                        Color32::PLACEHOLDER,
                    )
                    .size()
                    .x
                }) + size::KIND_PAD_X * 2.0
            };
            let box_ = Rect::from_min_size(Pos2::new(x, top), egui::vec2(width, size::KIND_H));
            x += width + size::LIB_KINDS_GAP;
            (chip, box_)
        })
    }

    /// What a press at `p` on the kind row asks for, or `None` where there is no
    /// chip under it.
    ///
    /// # The chip flips and the operation names all six
    ///
    /// `Operation::FilterLibrary { kinds }` carries the whole row, never one chip:
    /// *"six presses that each say this one changed are six statements two hands
    /// can disagree about, and one that says these are the kinds showing is a
    /// destination"* (ADR-0338, which is `Operation::Publish`'s rule on a different
    /// list). So the flip is this console's arithmetic ([P-0090]) and what leaves
    /// is where it arrived — [`KindChip::flipped`].
    ///
    /// A press on a chip that is on turns it off, and with the last one off the row
    /// is [`LibraryKinds::EVERYTHING`] again: every state this row can be in is one
    /// a press can leave, which is what a control with more than two positions
    /// owes.
    ///
    /// [P-0090]: ../../../docs/principles/0090-a-surface-offers-it-never-decides.md
    ///
    /// # A chip is pressed only where it is drawn
    ///
    /// [`LibraryBay::chip`]'s rule one row up and for its reason: the row clips, so
    /// a chip that runs past the bay's own edge is pressable only where it is
    /// painted, and the point is held to the row before any chip is asked about.
    /// `None` before the first pass for that method's reason too — there are no
    /// fonts to measure a word with.
    pub fn kind(
        &self,
        ctx: &egui::Context,
        at: Filters<'_>,
        p: karakuri_layout::Point,
    ) -> Option<Operation> {
        let row = self.kinds?;
        if ctx.cumulative_pass_nr() == 0 {
            return None;
        }
        let p = Pos2::new(p.x, p.y);
        if !row.contains(p) {
            return None;
        }
        self.kind_chips(ctx)
            .find(|(_, box_)| box_.contains(p))
            .map(|(chip, _)| Operation::FilterLibrary {
                kinds: chip.flipped(at.kinds),
            })
    }

    /// What a press at `p` on the filter row asks the store for, or `None` where
    /// there is no field under it.
    ///
    /// # The field steps, and the operation names where it arrived
    ///
    /// [`TransitionRow::shape`]'s affordance, in a bay where it costs an
    /// explanation rather than a sentence. What [`Operation::ListSets`] carries is
    /// free text and a layer — *"narrowed by what a node is called or by which
    /// layer a Set uses"* — and this console has one letter-taking flow,
    /// [`Menu::Naming`], which ADR-0221 bounds to one path component of a name. A
    /// filter is not a name, so typing into these two would be a second
    /// letter-taking flow, and what that flow *is* is a decision about the console
    /// rather than about this bay ([`Chosen`]'s rule, one control up: the first
    /// control that wants a thing is not where it is decided).
    ///
    /// So the fields step, over two closed lists, and the operation names the
    /// destination — never a step, because there is no step in the vocabulary to
    /// name (P-0090). `layer` steps [`LAYERS`], which is the console's own curation
    /// of an enum with no list in it. `holds` steps [`View::holds`], which is the
    /// host's answer to *what are this store's Sets made of* and arrives across the
    /// same seam as the listing itself — this crate reads no store (ADR-0156). Both
    /// wrap through unset, so every state either field can be in is one the press
    /// can leave.
    ///
    /// A press with nothing to step to is answered all the same, and that is the
    /// state a `holds` field has on a store whose Sets name no node: it asks for
    /// the listing again, which is a real question and the one [`LibraryBay::chip`]
    /// already answers for the chip that is marked.
    ///
    /// # One derivation, asked twice
    ///
    /// [`crate::input::claim`]'s rule 4 asks this and so does the caller that acts
    /// on the press, exactly as they both ask [`LibraryBay::chip`]. The whole field
    /// is the target, its border included, which is [`Outputs::sink`]'s rule about
    /// a padding being what makes a word a hand can find.
    ///
    /// The `9` pixels of padding either side of the row and the
    /// [`size::LIB_FILTERS_GAP`] between the two fields are not targets, and this
    /// answers `None` for them: they are bare card, the way the gaps between the
    /// scope chips are.
    pub fn filter(
        &self,
        holds: &[String],
        at: Filters<'_>,
        p: karakuri_layout::Point,
    ) -> Option<Operation> {
        let p = Pos2::new(p.x, p.y);
        let which = Field::ALL
            .into_iter()
            .find(|field| self.field(*field).is_some_and(|box_| box_.contains(p)))?;
        Some(match which {
            // **`layer` goes out unset and the field it came from is gone.**
            // `Operation::ListSets` keeps that field — `list_sets` over MCP and
            // `--list-sets` are where *which Sets hold a node on this layer*
            // lives — and this console stopped asking it on 2026-09-10, because
            // a kind chip asks a different question of a different thing
            // (ADR-0338). The panel's way to *which Sets use this* is `holds`,
            // which asks by node name.
            Field::Holds => Operation::ListSets {
                holds: stepped_holds(holds, at.holds),
                layer: None,
            },
        })
    }
}

/// The filter row, painted: two fields and the rule under them.
///
/// Where the row goes and where each field in it goes are [`library`]'s and
/// [`LibraryBay::field`]'s, so this paints and derives nothing —
/// [`scopes_into`]'s rule one row up, and it is stricter here because the
/// fields are hit-tested and a second division would put a capsule a press
/// lands on somewhere the border is not.
///
/// Term for term from `style.css`:
///
/// - `.lib-filters { display: flex; gap: 5px; padding: 6px 9px; border-bottom:
///   1px solid var(--c-hair) }` — the field from the left of the row, over a
///   rule the row's bottom pixel.
/// - `.field { border: 1px solid var(--c-line); border-radius: 999px; padding:
///   0 9px; color: var(--c-faint); flex: 1 }` — a word at [`size::BASE`] in a
///   bordered capsule, taking the whole of what is left.
///
/// A set field and an unset one differ in the word alone, which is
/// [`HOLDS_UNSET`]'s sentence: `style.css` gives `.field` one rule and no set
/// variant, so `L4` where `layer…` was is the whole of the mark. `.scope.sel`'s
/// wash is not borrowed for it — that mark says *this is where a press lands*
/// about a chip a press moves between, and every press here lands on the field
/// it is already on.
///
/// A word too long for its field is clipped rather than elided, which is
/// `.lib-row`'s answer one box down and for the same reason: `.field` sets
/// `min-width: 0` and no `text-overflow`, so there is no ellipsis to draw. A
/// node name is what can be long enough for it.
pub(crate) fn filters_into(ui: &Ui, pal: &Palette, bay: &LibraryBay, at: Filters<'_>) {
    let Some(row) = bay.filters else {
        return;
    };
    let painter = ui.painter().with_clip_rect(row);
    for field in Field::ALL {
        let Some(box_) = bay.field(field) else {
            continue;
        };
        filter_field(&painter, pal, box_, at.word(field));
    }

    // `border-bottom: 1px solid var(--c-hair)` — the row's own bottom pixel,
    // and the same hairline the scope row above it draws. It is inside the row
    // rather than under it, which is what keeps the list's top where
    // [`library_box`] put it.
    let rule = row.max.y - size::HAIRLINE * 0.5;
    painter.line_segment(
        [Pos2::new(row.min.x, rule), Pos2::new(row.max.x, rule)],
        Stroke::new(size::HAIRLINE, pal.hair),
    );
}

/// The kind row, painted: the six toggles and the rule under them.
///
/// Where each chip goes is [`LibraryBay::kind_chips`], so this paints and
/// derives nothing — [`scopes_into`]'s rule two rows up, and it is that method's
/// shape term for term because it is the same object: a row of capsules, as
/// wide as the words in them, over a hairline that is the row's own bottom
/// pixel.
///
/// Term for term from `style.css`:
///
/// - `.lib-kinds { display: flex; gap: 4px; padding: 5px 9px; border-bottom:
///   1px solid var(--c-hair) }` — six chips from the left of the row, one
///   [`size::LIB_KINDS_GAP`] apart.
/// - `.kind { font-size: 9px; padding: 0 6px; border-radius: 999px; border: 1px
///   solid var(--c-line); color: var(--c-faint) }` — a word at
///   [`size::KIND_SIZE`] in a bordered capsule.
/// - `.kind.on { border-color: transparent; color: var(--c-mint); background:
///   color-mix(in srgb, var(--c-mint) 15%, transparent) }` — mint and not
///   lavender, which is what this console draws a control that is *on*: the
///   `params` pill in this bay's own foot is lit the same way, and lavender
///   here is the deck the keys are addressed to.
///
/// All six plain is the row a run opens on, and it says *everything shows*
/// rather than *nothing does* — [`karakuri_operation::LibraryKinds::narrowing`]
/// settles that reading once, and nothing here draws a seventh state for it.
pub(crate) fn kinds_into(ui: &Ui, pal: &Palette, bay: &LibraryBay, at: Filters<'_>) {
    let Some(row) = bay.kinds else {
        return;
    };
    let painter = ui.painter().with_clip_rect(row);
    for (chip, box_) in bay.kind_chips(ui.ctx()) {
        let on = chip.on(at.kinds);
        toggle_chip(
            &painter,
            box_,
            on,
            chip.word(),
            size::KIND_SIZE,
            pal,
            pal.mint,
        );
    }

    let rule = row.max.y - size::HAIRLINE * 0.5;
    painter.line_segment(
        [Pos2::new(row.min.x, rule), Pos2::new(row.max.x, rule)],
        Stroke::new(size::HAIRLINE, pal.hair),
    );
}
