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

/// The word on the control at the end of the chain's list.
const ADD_LABEL: &str = "+ add";

/// The glyph at the end of a slot's row, which takes that slot out of the
/// chain — `docs/manual/console.html`'s own mark for it.
pub(super) const REMOVE_GLYPH: &str = "\u{2212}";

/// Layout of the Master bay's body: out row, chain slot wells, and `+ add`.
/// Shared by rendering and hit-testing to prevent drift.
#[derive(Debug, Clone, PartialEq)]
pub struct MasterRow {
    /// The faint `out` at the head of the row.
    pub label: Rect,
    /// The fader: `.fader`'s 5px well lying down, what the level fills of it, and
    /// the knob centred on the fill's moving edge.
    pub fader: Fader,
    /// Fixed-width rectangle for the level figure, sized to the widest possible reading.
    pub value: Rect,
    /// The value these rectangles were measured from, carried for
    /// [`LookRow::values`]' reason: whoever measured the type and whoever paints it
    /// are one statement.
    pub out: f32,
    /// Visible chain slot wells in order (ADR-0340). Rows drop from bottom up if space is short.
    pub slots: Vec<SlotRow>,
    /// `+ add` at the end of the list, or `None` where the bay has no room for
    /// it. A press puts [`AddCard`] down and asks for nothing on its own; the
    /// procedure is named by picking an item of that card.
    pub add: Option<Rect>,
    /// The chooser's card, or `None` while it is up.
    pub card: Option<AddCard>,
    /// Landing rectangle for drag-and-drop carry release over the slot list (ADR-0273).
    pub list: Option<Rect>,
}

/// Layout of a chain slot: well, status dot, procedure name, cut chip, remove button, and param rows.
#[derive(Debug, Clone, PartialEq)]
pub struct SlotRow {
    /// Where this slot sits in the chain, counted from the mix's output, which is
    /// the address every chain operation takes.
    pub at: u32,
    /// The well the slot is drawn in.
    pub well: Rect,
    /// `.fx .dot`.
    pub dot: Rect,
    /// The procedure's name.
    pub name: Rect,
    /// The word in it, measured once and painted from the same string.
    pub words: String,
    /// Cut chip rectangle for procedures declaring `retains`, or `None`.
    pub cut: Option<Rect>,
    /// Which cut the slot reads, `None` where it reads none. The chip is laid
    /// out from it and the operation a press asks for carries it.
    pub reading: Option<karakuri_operation::Cut>,
    /// The `−` at the end of the row, which takes this slot out of the chain.
    pub remove: Rect,
    /// One row per parameter the procedure declares, in declaration order.
    pub params: Vec<ParamRow>,
}

/// Layout of a slot parameter row: label key, slider track, and value figure.
#[derive(Debug, Clone, PartialEq)]
pub struct ParamRow {
    /// Which slot this row moves.
    pub at: u32,
    /// The key the procedure declares, which is what the operation carries.
    pub key: String,
    /// The range the procedure declares it over. The track's position is a
    /// fraction of it and the operation carries the value.
    pub range: [f32; 2],
    /// What the slot holds it at, in the procedure's own units.
    pub value: f32,
    /// The key, drawn before the track.
    pub label: Rect,
    /// The track.
    pub fader: Fader,
    /// The figure, in a box as wide as the widest reading — [`MasterRow::value`]'s
    /// rule and its reason.
    pub amount: Rect,
}

impl ParamRow {
    /// Where along its declared range this row's value sits, on `[0, 1]`, which
    /// is what a fader draws. A range of no width is at zero.
    pub fn along(&self) -> f32 {
        let [low, high] = self.range;
        match high > low {
            true => ((self.value - low) / (high - low)).clamp(0.0, 1.0),
            false => 0.0,
        }
    }

    /// What a drag on this row asks for.
    ///
    /// The track's position is a fraction of the declared range, and the amount
    /// the operation carries is what the slot is set to.
    pub fn knob(&self) -> Knob {
        Knob::Chain {
            at: self.at,
            key: self.key.clone(),
            range: self.range,
        }
    }
}

impl SlotRow {
    /// Operation requested by pressing the cut chip (P-0090), or `None` if outside.
    pub fn chip(&self, p: karakuri_layout::Point) -> Option<Operation> {
        let chip = self.cut?;
        if !chip.contains(Pos2::new(p.x, p.y)) {
            return None;
        }
        Some(Operation::SetChainParam {
            at: self.at,
            param: karakuri_operation::ChainParam::Cut(self.next_cut()),
        })
    }

    /// The cut after the one this slot reads, wrapping. It is the chip's whole
    /// arithmetic, and a key and a press cycle the same list.
    pub fn next_cut(&self) -> karakuri_operation::Cut {
        let all = karakuri_operation::Cut::ALL;
        let showing = self
            .reading
            .and_then(|cut| all.iter().position(|c| *c == cut))
            .unwrap_or(0);
        all[(showing + 1) % all.len()]
    }

    /// What a press on this slot's `−` asks for, or `None` where `p` is not on
    /// it.
    pub fn minus(&self, p: karakuri_layout::Point) -> Option<Operation> {
        self.remove
            .contains(Pos2::new(p.x, p.y))
            .then_some(Operation::RemoveChainEffect { at: self.at })
    }
}

/// Master chain state snapshot mirrored for console display (ADR-0156).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Chain {
    /// The slots, in the chain's own order. Empty is the default chain.
    pub slots: Vec<ChainSlot>,
}

/// One slot of the chain, as the bay reads it.
#[derive(Debug, Clone, PartialEq)]
pub struct ChainSlot {
    /// What the row names the procedure: the name the library gives that address,
    /// or the short address where nothing names it.
    pub name: String,
    /// Which cut the slot reads, and `None` where its procedure declares no
    /// `retains` — which is whether the row draws a chip at all.
    pub cut: Option<karakuri_operation::Cut>,
    /// The parameters the procedure declares, in declaration order.
    pub params: Vec<SlotParam>,
}

/// One declared parameter of a slot, as the bay reads it.
#[derive(Debug, Clone, PartialEq)]
pub struct SlotParam {
    /// The name the procedure declares for it, which is what the operation
    /// carries.
    pub key: String,
    /// The range the procedure declares it over, low then high. The fader is laid
    /// out from it.
    pub range: [f32; 2],
    /// What the slot holds it at, in the procedure's own units.
    pub value: f32,
    /// What the procedure declares it at, which is where `space` on this row
    /// returns it to.
    pub default: f32,
}

/// One item of the `+ add` chooser: a `kind L5` procedure the library holds.
#[derive(Debug, Clone, PartialEq)]
pub struct AddChoice {
    /// The content address of the procedure's source, which is what
    /// [`Operation::AddChainEffect`] carries.
    pub procedure: String,
    /// The words on the item, which is the name the library lists it under.
    pub words: String,
    /// Whether the procedure declares `retains`, which is whether a slot of it
    /// takes a cut.
    pub retains: bool,
}

impl AddChoice {
    /// Builds [`Operation::AddChainEffect`] with default cut if `retains` is declared (ADR-0348).
    pub fn operation(&self) -> Operation {
        Operation::AddChainEffect {
            procedure: self.procedure.clone(),
            cut: self.retains.then(karakuri_operation::Cut::default),
        }
    }
}

/// What the `+ add` chooser offers this frame, read off the [`View`] once and
/// handed in — [`Choices`]' shape one bay along. The item that is painted and
/// the item a press lands on are one derivation.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AddChoices {
    /// Every procedure on offer, in the order the library lists them.
    pub items: Vec<AddChoice>,
    /// Whether the card is down — the console's own state, like [`Target::open`].
    pub open: bool,
}

impl AddChoices {
    /// Nothing to add, which is a console whose library is listing no `kind L5`
    /// procedure — every test in this crate that does not hand one in.
    pub fn none() -> AddChoices {
        AddChoices::default()
    }
}

/// The `+ add` chooser's card, laid out — [`LaneCard`]'s shape one bay along and
/// the same mechanism: a card of items hanging off the control that opened it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AddCard {
    /// The card itself.
    pub card: Rect,
    /// How many items it draws — [`AddChoices::items`]' length.
    pub items: usize,
}

impl AddCard {
    /// Where one item is, from the top of the card — [`LaneCard::item`]'s own
    /// reading, with no rule in it: one kind of thing is on this card.
    ///
    /// Panics on an item this card has not got.
    pub fn item(&self, index: usize) -> Rect {
        assert!(
            index < self.items,
            "item {index} of a card of {}",
            self.items
        );
        Rect::from_min_size(
            Pos2::new(
                self.card.min.x + size::LIB_LIST_PAD,
                self.card.min.y + size::LIB_LIST_PAD + size::LIB_ROW_H * index as f32,
            ),
            egui::vec2(
                self.card.width() - size::LIB_LIST_PAD * 2.0,
                size::LIB_ROW_H,
            ),
        )
    }

    /// Which item `p` is on, or `None` for a point on the card's padding or off
    /// the card altogether — [`LaneCard::picked`]'s own answer.
    pub fn picked(&self, p: karakuri_layout::Point) -> Option<usize> {
        let at = Pos2::new(p.x, p.y);
        (0..self.items).find(|index| self.item(*index).contains(at))
    }
}

/// What a press on the `+ add` control or on its card asks for.
///
/// [`Chose`]'s shape one bay along, and the same division: every arm is either
/// the console's own state or an operation, and nothing here is both.
#[derive(Debug, Clone, PartialEq)]
pub enum Added {
    /// The pill was pressed with the card up: put it down.
    Open,
    /// A press that dismisses the card and asks for nothing.
    Shut,
    /// An item was picked, and this is what it asks for.
    Add(Operation),
}

impl MasterRow {
    /// Resolves which knob is grabbed at point `p`. The track itself is not a target.
    pub fn grab(&self, p: karakuri_layout::Point) -> Option<Grab> {
        let at = Pos2::new(p.x, p.y);
        grabbed(self.fader, Knob::Out, at).or_else(|| {
            self.slots
                .iter()
                .flat_map(|slot| slot.params.iter())
                .find_map(|row| grabbed(row.fader, row.knob(), at))
        })
    }

    /// What a press on a slot's cut chip or on its `−` asks for, or `None` —
    /// see [`SlotRow::chip`] and [`SlotRow::minus`].
    pub fn chip(&self, p: karakuri_layout::Point) -> Option<Operation> {
        self.slots
            .iter()
            .find_map(|slot| slot.chip(p).or_else(|| slot.minus(p)))
    }

    /// Resolves `+ add` click or card pick/dismissal at point `p` (Rule 2).
    pub fn chose(&self, p: karakuri_layout::Point, choices: &AddChoices) -> Option<Added> {
        if let Some(card) = self.card {
            return Some(match card.picked(p) {
                Some(item) => match choices.items.get(item) {
                    Some(choice) => Added::Add(choice.operation()),
                    None => Added::Shut,
                },
                None => Added::Shut,
            });
        }
        let add = self.add?;
        add.contains(Pos2::new(p.x, p.y)).then_some(Added::Open)
    }

    /// Returns the landing rectangle if `p` is over the chain's slot list (ADR-0273).
    pub fn dropped(&self, p: karakuri_layout::Point) -> Option<Rect> {
        self.list.filter(|list| list.contains(Pos2::new(p.x, p.y)))
    }

    /// Whether `p` is on one of the things here a hand can move, which is what
    /// [`crate::input::claim`] asks — the knobs, the cut chips, the `−` glyphs,
    /// `+ add` and whatever the card is drawing.
    pub fn owns(&self, p: karakuri_layout::Point) -> bool {
        self.grab(p).is_some()
            || self.chip(p).is_some()
            || self
                .add
                .is_some_and(|add| add.contains(Pos2::new(p.x, p.y)))
            // The card's own rectangle, and not every press while it is down:
            // `crate::input::claim`'s rule 2 already claims the console for a
            // card that is down.
            || self
                .card
                .is_some_and(|card| card.card.contains(Pos2::new(p.x, p.y)))
    }
}

/// Derives layout for the Master bay body: out row, chain slots, and `+ add`.
/// Returns `None` if engine state is absent or space is insufficient.
pub fn master(
    ctx: &egui::Context,
    layout: &karakuri_layout::Layout,
    out: Option<f32>,
    chain: Option<&Chain>,
    choices: &AddChoices,
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

    // The widest `d.dd` there is, which is every reading a level on this bay
    // can ask for. Measured rather than assumed.
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
    // refusal one crate layer down: a bay narrow enough that the word and the
    // figure meet has nothing left to draw a fader in.
    if track.width() <= 0.0 {
        return None;
    }

    // The chain's own rows, under the out row and off the same width. A
    // console with no chain behind it draws the out row it was handed a level
    // for and nothing under it.
    let mut slots = Vec::new();
    let mut add = None;
    let mut list: Option<Rect> = None;
    if let Some(chain) = chain {
        let mut top = row.max.y + size::MASTER_STACK_GAP;
        for (at, slot) in chain.slots.iter().enumerate() {
            let height = well_height(slot.params.len());
            let well =
                Rect::from_min_size(Pos2::new(row.min.x, top), egui::vec2(row.width(), height));
            if !region.contains_rect(well) {
                break;
            }
            let Some(drawn) = slot_row(ctx, at as u32, slot, well, widest, &width) else {
                break;
            };
            slots.push(drawn);
            list = Some(match list {
                Some(held) => held.union(well),
                None => well,
            });
            top = well.max.y + size::MASTER_STACK_GAP;
        }
        // `+ add` is the end of the list, and it is drawn only where the bay
        // has room for it.
        let pill = Rect::from_min_size(
            Pos2::new(row.min.x, top),
            egui::vec2(row.width(), size::FX_H),
        );
        if region.contains_rect(pill) {
            add = Some(pill);
            list = Some(match list {
                Some(held) => held.union(pill),
                None => pill,
            });
        }
    }

    let card = add.and_then(|add| add_card(ctx, to_egui(layout.viewport()), add, choices));

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
        slots,
        add,
        card,
        list,
    })
}

/// How tall a slot's well is: the head line, then one line per declared
/// parameter, inside the well's own padding and [`size::FX_PAD_Y`] between the
/// lines.
fn well_height(params: usize) -> f32 {
    let lines = 1 + params;
    size::FX_PAD_Y * 2.0 + size::MASTER_ROW_H * lines as f32 + size::FX_PAD_Y * params as f32
}

/// Lays out a single effects chain slot within `well`.
fn slot_row(
    ctx: &egui::Context,
    at: u32,
    slot: &ChainSlot,
    well: Rect,
    widest: f32,
    width: &dyn Fn(&str) -> f32,
) -> Option<SlotRow> {
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
    let inner = Rect::from_min_max(
        Pos2::new(well.min.x + size::FX_PAD_X, well.min.y + size::FX_PAD_Y),
        Pos2::new(well.max.x - size::FX_PAD_X, well.max.y - size::FX_PAD_Y),
    );
    if inner.width() <= 0.0 {
        return None;
    }
    let head = Rect::from_min_size(inner.min, egui::vec2(inner.width(), size::MASTER_ROW_H));
    let mid = head.center().y;
    let dot = Rect::from_center_size(
        Pos2::new(head.min.x + size::FX_DOT * 0.5, mid),
        egui::vec2(size::FX_DOT, size::FX_DOT),
    );
    let name = Rect::from_min_size(
        Pos2::new(dot.max.x + size::FX_GAP, head.min.y),
        egui::vec2(width(&slot.name), head.height()),
    );
    // The `−` is at the far end of the head line, where the figure is on the
    // rows under it: one column of things that end at the well's padding, so
    // nothing on this row moves when a name gets longer.
    let minus = width(REMOVE_GLYPH);
    let remove = Rect::from_min_size(
        Pos2::new(head.max.x - minus, head.min.y),
        egui::vec2(minus, head.height()),
    );
    // The chip is on a slot whose procedure declares `retains` and on no
    // other. It is as wide as the wider of the two words rather than as wide
    // as the one it is showing.
    let cut = slot.cut.map(|_| {
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
    let mut params = Vec::with_capacity(slot.params.len());
    let mut top = head.max.y + size::FX_PAD_Y;
    for param in &slot.params {
        let line = Rect::from_min_size(
            Pos2::new(inner.min.x, top),
            egui::vec2(inner.width(), size::MASTER_ROW_H),
        );
        let label = Rect::from_min_size(line.min, egui::vec2(width(&param.key), line.height()));
        let amount = Rect::from_min_size(
            Pos2::new(line.max.x - widest, line.min.y),
            egui::vec2(widest, line.height()),
        );
        let centre = line.center().y;
        let track = Rect::from_min_max(
            Pos2::new(label.max.x + size::FX_GAP, centre - size::FADER_H * 0.5),
            Pos2::new(amount.min.x - size::FX_GAP, centre + size::FADER_H * 0.5),
        );
        // A track with no length is no control — [`master`]'s own refusal.
        // A row without one is not drawn at all.
        if track.width() <= 0.0 {
            return None;
        }
        let row = ParamRow {
            at,
            key: param.key.clone(),
            range: param.range,
            value: param.value,
            label,
            fader: fader(
                track,
                Axis::Row,
                0.0,
                0.0,
                egui::vec2(size::FADER_KNOB_W, size::FADER_KNOB_H),
            ),
            amount,
        };
        // The fill and the knob ride the declared range, which is what
        // [`ParamRow::along`] answers — laid out once the row can be asked.
        let placed = ParamRow {
            fader: fader(
                track,
                Axis::Row,
                row.along(),
                0.0,
                egui::vec2(size::FADER_KNOB_W, size::FADER_KNOB_H),
            ),
            ..row
        };
        params.push(placed);
        top = line.max.y + size::FX_PAD_Y;
    }
    Some(SlotRow {
        at,
        well,
        dot,
        name,
        words: slot.name.clone(),
        cut,
        reading: slot.cut,
        remove,
        params,
    })
}

/// Derives the `+ add` chooser card, clamped to the viewport, or `None` if shut/empty.
fn add_card(
    ctx: &egui::Context,
    viewport: Rect,
    add: Rect,
    choices: &AddChoices,
) -> Option<AddCard> {
    if !choices.open || choices.items.is_empty() {
        return None;
    }
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
    let widest = choices
        .items
        .iter()
        .map(|item| width(&item.words))
        .fold(size::ROW_MENU_MIN_W, f32::max);
    let height = size::LIB_LIST_PAD * 2.0 + size::LIB_ROW_H * choices.items.len() as f32;
    let card = held_inside(
        &viewport,
        add.max.x - (widest + (size::LIB_ROW_PAD_X + size::LIB_LIST_PAD) * 2.0),
        add.max.y + size::PILL_GAP,
        widest + (size::LIB_ROW_PAD_X + size::LIB_LIST_PAD) * 2.0,
        height,
    );
    Some(AddCard {
        card,
        items: choices.items.len(),
    })
}

/// The level, as the mock's `.val` writes it — `1.00`, two places, and the same
/// string the transport row's exposure is written with. The two are the same
/// kind of reading and deliberately read the same way; where they stop being
/// the same *number* is ADR-0224.
fn master_text(out: f32) -> String {
    format!("{out:.2}")
}

/// Paints the Master bay body (label, fader, level value, slots, and add button).
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
    // bay's padding whatever it says.
    let galley = painter.layout_no_wrap(
        master_text(row.out),
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
    for slot in &row.slots {
        slot_into(ui, pal, slot);
    }
    if let Some(add) = row.add {
        add_into(ui, pal, add);
    }
}

/// Paints a single chain slot (well, mint indicator dot, name, cut chip, and remove button).
pub(super) fn slot_into(ui: &Ui, pal: &Palette, slot: &SlotRow) {
    let painter = ui.painter();
    painter.rect_filled(slot.well, CornerRadius::same(size::FX_RADIUS), pal.well);
    painter.rect_stroke(
        slot.well,
        CornerRadius::same(size::FX_RADIUS),
        Stroke::new(size::HAIRLINE, pal.mint),
        StrokeKind::Inside,
    );
    painter.circle_filled(slot.dot.center(), size::FX_DOT * 0.5, pal.mint);
    let word = |rect: Rect, text: &str, colour: Color32, align_right: bool| {
        let galley = painter.layout_no_wrap(
            text.to_owned(),
            FontId::new(size::BASE, FontFamily::Proportional),
            colour,
        );
        let x = match align_right {
            true => rect.max.x - galley.size().x,
            false => rect.min.x,
        };
        painter.galley(
            Pos2::new(x, rect.center().y - galley.size().y * 0.5),
            galley,
            colour,
        );
    };
    word(slot.name, &slot.words, pal.text, false);
    word(slot.remove, REMOVE_GLYPH, pal.faint, true);
    // **`.mini.sel`**, which is the mixer's blend chip exactly: a lavender
    // wash and a lavender word, because what it says is *this is the one
    // chosen*.
    if let (Some(chip), Some(cut)) = (slot.cut, slot.reading) {
        mini_into(painter, pal, chip, true, |painter, colour| {
            let galley = painter.layout_no_wrap(
                cut.name().to_owned(),
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
    for param in &slot.params {
        word(param.label, &param.key, pal.faint, false);
        fader_into(painter, pal, param.fader, false, None);
        word(param.amount, &master_text(param.value), pal.text, true);
    }
}

/// `+ add`, painted: the word centred in a `.fx` well, faint, which is the
/// mock's own `.fx.off` for this control.
pub(super) fn add_into(ui: &Ui, pal: &Palette, add: Rect) {
    let painter = ui.painter();
    painter.rect_filled(add, CornerRadius::same(size::FX_RADIUS), pal.well);
    let galley = painter.layout_no_wrap(
        ADD_LABEL.to_owned(),
        FontId::new(size::BASE, FontFamily::Proportional),
        pal.faint,
    );
    painter.galley(
        Pos2::new(
            add.center().x - galley.size().x * 0.5,
            add.center().y - galley.size().y * 0.5,
        ),
        galley,
        pal.faint,
    );
}
