use super::*;

// ---------------------------------------------------------------------------
// The Sequencer bay
// ---------------------------------------------------------------------------

/// The word at the head of the Sequencer bay, in the source's own
/// capitalisation for [`Kind::Bay`]'s reason: the mock upper-cases in CSS, and
/// that is done at paint time so the word a reader searches for is the word in
/// the source.
pub(super) const SEQUENCER_TITLE: &str = "Sequencer";

/// The mark a lane's label carries after the deck's letter: `▮` for a fader and
/// `∿` for a parameter, which are the two glyphs `docs/manual/console.html`
/// draws on the four lane labels.
///
/// A `match` over the target and not a field on the reading, for
/// `karakuri_operation::Sync::name`'s reason one crate down: a target added to
/// that enum does not compile until it has a mark to be drawn with.
fn lane_mark(target: &LaneTarget) -> &'static str {
    match target {
        LaneTarget::Fader { .. } => "\u{25AE}",
        LaneTarget::Param { .. } => "\u{223F}",
    }
}

/// What one lane's row reads: the deck's letter and the mark for what it drives
/// — `A ▮`, `B ∿` — which is the mock's own label word for word.
pub(crate) fn lane_label(target: &LaneTarget) -> String {
    let letter = DECK_LETTERS
        .get(usize::from(target.deck()))
        .copied()
        // A deck past the four is a caller's error and not a state, and is
        // drawn rather than panicked for [`deck_letter`]'s reason one bay
        // along: a label is a readout and a readout does not stop a frame.
        .unwrap_or("?");
    format!("{letter} {}", lane_mark(target))
}

/// What the sequencer bay reads this frame: the armed pattern, which bank it
/// is, and where the playhead was left.
///
/// A pattern and not a copy of one, taken apart. The bay draws the mode, the
/// lanes, what each drives, its steps and its mute, which is the whole of what
/// a pattern is — so a reading with a field per drawn thing would be a second
/// spelling of `karakuri_pattern::Pattern` that could disagree with it. This
/// crate holds no pattern and applies nothing to one (ADR-0156); the host
/// writes this per frame beside the frame it is about, which is
/// [`View::mixer`]'s seam.
///
/// `step` comes from the poll and not from `beats`. Where the playhead is is
/// what the *producer* last answered — `karakuri_pattern::Playhead` — and
/// deriving it here from the transport's beats would be a second derivation
/// that could name a step the sequencer never emitted (P-0087). `None` before
/// the first poll, which draws no column: a bay that has not been polled is not
/// a bay at step zero.
#[derive(Debug, Clone, PartialEq)]
pub struct Sequenced {
    /// The armed pattern, as the host read it this frame.
    pub pattern: karakuri_pattern::Pattern,
    /// Which of `karakuri_pattern::BANKS` is armed.
    pub bank: usize,
    /// Where the poll last put the playhead.
    pub step: Option<usize>,
}

/// One item of the `+ lane` chooser: a target a lane may be pointed at, and the
/// words drawn on it.
///
/// The words are the lane label the pick will make, plus what it is. A fader
/// item reads `A ▮ fader` and a parameter item `B ∿ L2:0 twist`, so the row
/// that appears after the press reads as the item that was picked — the deck's
/// letter and [`lane_mark`], which is [`lane_label`]'s own derivation asked one
/// control earlier.
#[derive(Debug, Clone, PartialEq)]
pub struct LaneChoice {
    /// What a pick points the lane at — the payload of `Operation::PointLane`,
    /// whole.
    pub target: LaneTarget,
    /// The words on the item.
    pub words: String,
}

/// What the `+ lane` chooser offers this frame, read off the [`View`] once and
/// handed in — [`Target`]'s shape one bay along, and for its reason: the item
/// that is painted and the item a press lands on are one derivation of one
/// reading.
///
/// # What is in the list, and why it is one deck's parameters and every deck's
/// fader
///
/// A lane's target is `Fader { deck }` or `Param { deck, param }`
/// ([ADR-0321](../../../../docs/adr/0321-a-lanes-target-is-an-operation-with-its-value-elided.md)),
/// so the list is the faders of every deck the mixer draws a strip for —
/// [`View::select`]'s own count read a fourth time — and the published controls
/// of one deck: the Library bay's load pulldown's ([`View::target_deck`],
/// ADR-0305).
///
/// That mark and not a second one. It is the console's one pointer meaning *a
/// deck named without moving the keys*, which is exactly what pointing a lane
/// wants — a lane on deck C while deck A is playing — and a chooser of its own
/// in this bay would be a fourth pointer on a panel that already explains three
/// (ADR-0305's counting argument). Listing every deck's keys instead would put
/// the same key in the list once per deck, so the operator would pick a deck by
/// reading a list four times as long rather than by a control.
/// [ADR-0327](../../../../docs/adr/0327-the-lane-chooser-lists-one-decks-keys-and-the-bank-pills-are-the-four-banks.md).
///
/// What the console does not hold, it does not offer. The parameters are
/// [`View::inspector`]'s, which is written when a Set lands, and the inspector
/// holds [`PANES`] panes — so a target deck no pane is pointed at contributes
/// no parameters and the list is its faders alone. That is the reading's own
/// limit rather than this control's, and it is written down in
/// `docs/manual/console.html`'s `+ lane` tip.
#[derive(Debug, Clone, PartialEq)]
pub struct Choices {
    /// Every target on offer: the faders first, then the parameters.
    pub items: Vec<LaneChoice>,
    /// How many of the leading items are faders, which is where the card's
    /// separator goes — [`RowMenu::rule`]'s band between the loads and the send,
    /// reached by the same argument: two kinds of item, and the rule says so.
    pub faders: usize,
    /// Whether the card is down — the console's own state, like [`Target::open`].
    /// See [`View::lane_open`].
    pub open: bool,
}

impl Choices {
    /// Nothing to point at, which is a console with no mixer and no pane — every
    /// test in this crate that does not hand one in.
    pub fn none() -> Choices {
        Choices {
            items: Vec::new(),
            faders: 0,
            open: false,
        }
    }
}

/// One lane's row: the label a press mutes it by, and the cells a press sets a
/// step by.
#[derive(Debug, Clone, PartialEq)]
pub struct SeqRow {
    /// The label, and it is a control: *"Click to mute the lane and keep the
    /// pattern"*, which is where rule 02's take-back sits for a lane.
    pub label: Rect,
    /// One rectangle per step of the mode, so there are sixteen of these at a
    /// sixteenth and eight at an eighth: the row keeps its width and the cells
    /// halve in the finer one (ADR-0306).
    pub cells: Vec<Rect>,
    /// What the row draws from: the lane's own label, which slots are on, and
    /// whether it is muted.
    pub words: String,
    pub muted: bool,
    /// Whether each drawn cell is on, in the cells' order — the mode's reading of
    /// the lane's sixteen slots, taken where the row is laid out so the paint and
    /// the press cannot disagree about which slot a cell is.
    pub on: Vec<bool>,
    /// Which stored slot each drawn cell is, which is what a press sends: the
    /// identity at a sixteenth and `2k` at an eighth, so `Operation::SetStep`
    /// carries a slot and never a step (`karakuri_operation::StepMode::slot_of`).
    pub slots: Vec<usize>,
}

/// The Sequencer bay, laid out: the head's mode pill and step readout, the
/// ruler, the playhead column and a row per lane.
///
/// # What is drawn and what is not
///
///
/// [ADR-0200](../../../../docs/adr/0200-a-bays-first-pass-draws-the-values-that-exist-and-omits-the-rest.md)
/// is satisfied here for the first time in this bay, and ADR-0222 said why it
/// could not be before: every part of the drawing now reads a value that
/// exists, because a pattern exists. One thing the mock draws is still not
/// drawn: the foot's sentence, which is a readout and not a control.
///
/// The bank pills and the `+ lane` pill landed on 2026-09-09. The pills are
/// [`Head::banks`] — the head machinery gained one field and every other head's
/// [`HeadWords`] is what it was — and they are laid out here a second time
/// rather than copied ([`bank_capsules`]), which is [`program_head`]'s
/// arrangement: the capsule an operator sees and the capsule a press lands on
/// are one derivation. Four pills and no `+`: with four fixed banks the mock's
/// `+` is `Operation::SelectPattern` at an empty bank, which is what a press on
/// `seq 3` already is
/// ([ADR-0327](../../../../docs/adr/0327-the-lane-chooser-lists-one-decks-keys-and-the-bank-pills-are-the-four-banks.md)).
///
/// # The cells are the row divided by the count, and the count follows the mode
///
/// `.seq-lane` is `repeat(16, 1fr)` with a [`size::SEQ_CELL_GAP`] between, so a
/// cell is as wide as what is left of the row after the gaps — and at an eighth
/// there are eight of them over the same width, which is the mock's *"the row
/// keeps its width, so the cells halve in the finer one"* read the other way
/// round.
pub fn sequencer(
    ctx: &egui::Context,
    layout: &karakuri_layout::Layout,
    reading: Option<&Sequenced>,
    choices: &Choices,
) -> Option<Sequencer> {
    let reading = reading?;
    // Fonts are not valid until `egui` has run a pass, exactly as in `mixer`,
    // `outputs` and `transport`.
    if ctx.cumulative_pass_nr() == 0 {
        return None;
    }
    let region = to_egui(layout.rect(layout.find("sequencer")?));
    let left = region.min.x + size::SEQ_PAD_X;
    let right = region.max.x - size::SEQ_PAD_X;
    if right <= left {
        return None;
    }
    let mut y = region.min.y + size::HEAD_H + size::SEQ_PAD_TOP;

    // -- the bay head's bank pills, laid out where they are painted ---------
    // **The same head [`bay_head`] paints**, asked a second time rather than
    // copied ([`head_capsule`]'s arrangement), so the capsule an operator
    // presses is the capsule that was drawn.
    //
    // **`Open::CLOSED` and no opening threaded here**: this head opens no class
    // — `class_at("sequencer")` is `None`, which
    // `docs/manual/console.html` states of this bay in as many words — so
    // nothing in it moves with an opening. `a_sequencer_head_opens_no_class`
    // is what holds that rather than this comment.
    let banks = crate::view::region("sequencer")
        .and_then(head_of)
        .map(|head| head.with_banks(reading.bank))
        .map(|head| bank_capsules(ctx, region, &head, Open::CLOSED))
        .unwrap_or_default();

    // -- the foot's `+ lane`, measured before the rows so they clear it -----
    // `.seq-foot` sits on the bottom edge of `.seq`, so the pill is against
    // the bay's own padding at both the right and the bottom — the same two
    // numbers the head and the rows are inset by.
    let add_w = pill_width(ctx, ADD_LANE);
    let add = Rect::from_min_size(
        Pos2::new(right - add_w, region.max.y - size::SEQ_PAD_X - size::PILL_H),
        egui::vec2(add_w, size::PILL_H),
    );
    if add.min.x < left {
        return None;
    }

    // -- the head: the mode pill, then the step readout ---------------------
    let mode = reading.pattern.mode();
    let pill_w = pill_width(ctx, mode.name());
    let mode_pill = Rect::from_min_size(Pos2::new(left, y), egui::vec2(pill_w, size::PILL_H));
    let words = step_words(reading.step, mode);
    let step_w = ctx.fonts_mut(|f| {
        f.layout_no_wrap(
            words.clone(),
            FontId::new(size::BASE, FontFamily::Proportional),
            Color32::PLACEHOLDER,
        )
        .size()
        .x
    });
    let step = Rect::from_min_size(
        Pos2::new(mode_pill.max.x + size::SEQ_HEAD_GAP, y),
        egui::vec2(step_w, size::PILL_H),
    );
    if step.max.x > right {
        return None;
    }
    y = mode_pill.max.y + size::SEQ_STACK_GAP;

    // -- the ruler, inset so its numbers stand over the cells ---------------
    let ruler = Rect::from_min_max(
        Pos2::new(left + size::SEQ_RULER_INSET, y),
        Pos2::new(right, y + size::SEQ_RULER_SIZE * size::LINE),
    );
    if ruler.width() <= 0.0 {
        return None;
    }
    y = ruler.max.y + size::SEQ_STACK_GAP;

    // -- the rows, and the lane track every cell is measured in -------------
    let track_x = left + size::SEQ_LABEL_W + size::SEQ_ROW_GAP;
    if track_x >= right {
        return None;
    }
    let count = mode.count();
    let gaps = size::SEQ_CELL_GAP * (count as f32 - 1.0);
    let cell_w = (right - track_x - gaps) / count as f32;
    // **A cell with no width is no control**, which is `master`'s own refusal
    // one bay up: a bay narrow enough that the label and the track meet has
    // nothing to draw sixteen cells in, and half a grid is worse than none.
    if cell_w <= 0.0 {
        return None;
    }
    let cell_at = |row_top: f32, at: usize| {
        Rect::from_min_size(
            Pos2::new(track_x + (cell_w + size::SEQ_CELL_GAP) * at as f32, row_top),
            egui::vec2(cell_w, size::SEQ_CELL_H),
        )
    };
    let body_top = y;
    let mut rows = Vec::with_capacity(reading.pattern.lanes().len());
    for lane in reading.pattern.lanes() {
        let label = Rect::from_min_size(
            Pos2::new(left, y),
            egui::vec2(size::SEQ_LABEL_W, size::SEQ_CELL_H),
        );
        let cells: Vec<Rect> = (0..count).map(|at| cell_at(y, at)).collect();
        rows.push(SeqRow {
            label,
            cells,
            words: lane_label(lane.target()),
            muted: lane.muted(),
            on: (0..count).map(|at| lane.step_on(at, mode)).collect(),
            slots: (0..count).map(|at| mode.slot_of(at)).collect(),
        });
        y += size::SEQ_CELL_H + size::SEQ_BODY_GAP;
    }
    // **The bay is clipped rather than half drawn.** A bay too short for the
    // rows it has is `master`'s refusal again: what would be drawn is a lane
    // over the card's own edge, and a cell a press could not reach.
    let bottom = match rows.is_empty() {
        true => body_top,
        false => y - size::SEQ_BODY_GAP,
    };
    // **And the rows clear the foot**, which is the same refusal read against
    // the pill instead of against the card's edge: the `+ lane` press is a
    // control and a lane drawn over it is a control a hand cannot reach.
    if bottom > add.min.y - size::SEQ_STACK_GAP {
        return None;
    }
    // **The playhead is one column over every row**, which is `.seq-play`'s
    // `position: absolute; inset: 0`: it is the body's height and the cell's
    // width, and it is drawn under nothing — `pointer-events: none`, so it
    // claims no press.
    let playhead = reading
        .step
        .filter(|_| !rows.is_empty())
        .map(|step| step.min(count.saturating_sub(1)))
        .map(|step| {
            Rect::from_min_max(
                Pos2::new(cell_at(body_top, step).min.x, body_top),
                Pos2::new(cell_at(body_top, step).max.x, bottom),
            )
        });
    Some(Sequencer {
        mode_pill,
        mode,
        step,
        step_words: words,
        ruler,
        playhead,
        rows,
        bank: reading.bank,
        banks,
        add,
        card: lane_card(ctx, to_egui(layout.viewport()), add, choices),
    })
}

/// The `+ lane` chooser's card, or `None` while it is up — and `None` for a
/// chooser with nothing in it, which is a console the mixer draws no strip for.
///
/// # It hangs up off the pill, where a row's menu hangs down off a row
///
/// [`Load::list`]'s rule, and the same one: this pill is in the foot of a bay,
/// so what is under it is the bay's own edge and the card stands on the pill's
/// top edge, one [`size::PILL_GAP`] clear of it, over this bay's rows. It is
/// held inside the viewport, so a Sequencer bay at the bottom of a short window
/// draws the card over the bays above rather than off the top.
///
/// The rows are not counted against the room, which is that method's other
/// clause: the list is at most [`DECKS`] faders and however many controls one
/// deck published, and a window too short to hold it is a window with no
/// transport row in it either.
///
/// The items are as wide as the words in them, which is `egui`'s to answer —
/// [`View::menu`]'s reason one bay along, and why this takes the context.
fn lane_card(
    ctx: &egui::Context,
    viewport: Rect,
    add: Rect,
    choices: &Choices,
) -> Option<LaneCard> {
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
    // **The rule is drawn only where it divides two things**, which is what
    // makes it a separator rather than a line: a list of faders alone and a
    // list of parameters alone each have one kind in them.
    let ruled = choices.faders > 0 && choices.faders < choices.items.len();
    let rule_h = match ruled {
        true => size::ROW_MENU_RULE_H,
        false => 0.0,
    };
    let height = size::LIB_LIST_PAD * 2.0 + size::LIB_ROW_H * choices.items.len() as f32 + rule_h;
    // **Held inside the console**, which is the two Library cards' own rule:
    // `held_inside` clamps the left edge, so a card wider than the room it
    // stands in comes back into the window rather than off it.
    let card = held_inside(
        &viewport,
        add.max.x - (widest + (size::LIB_ROW_PAD_X + size::LIB_LIST_PAD) * 2.0),
        add.min.y - size::PILL_GAP - height,
        widest + (size::LIB_ROW_PAD_X + size::LIB_LIST_PAD) * 2.0,
        height,
    );
    Some(LaneCard {
        card,
        faders: choices.faders,
        items: choices.items.len(),
        ruled,
    })
}

/// What the head's readout says — `step 6 of 16`, counting from one as the
/// ruler does, and `step — of 16` before the first poll.
///
/// The count is in it and the mock's is not. `docs/manual/console.html` draws
/// `step 6` and says *"Step 6 of sixteen"* in its tip; the count follows the
/// mode now and the pill beside it can be pressed, so a readout that said only
/// `6` would leave a hand that had just halved the grid reading the same figure
/// against a different bar.
fn step_words(step: Option<usize>, mode: StepMode) -> String {
    match step {
        Some(step) => format!("step {} of {}", step + 1, mode.count()),
        None => format!("step \u{2014} of {}", mode.count()),
    }
}

/// The Sequencer bay's controls, as rectangles to press.
///
/// Everything here is derived from the pattern the host handed in this frame,
/// so the cell that is painted is the cell that is pressed — the rule every
/// other bay in this module follows, and the one that makes
/// [`crate::input::claim`] and the press handler ask the same question.
#[derive(Debug, Clone, PartialEq)]
pub struct Sequencer {
    /// The mode pill, reading `1/16` or `1/8`. A press asks for the other of the
    /// two by naming it — a state and never a flip.
    pub mode_pill: Rect,
    /// The mode those rectangles were laid out from, carried for
    /// [`MasterRow::out`]'s reason: whoever measured the type and whoever paints it
    /// are one statement.
    pub mode: StepMode,
    /// The step readout, which is a readout: there is nothing here to press, and a
    /// hand that wants a pattern to begin somewhere else has no control in this bay
    /// for it.
    pub step: Rect,
    /// The words in it, measured once and painted from the same string.
    pub step_words: String,
    /// The ruler, which is a readout too: four numbers over the cells.
    pub ruler: Rect,
    /// The playhead's column, or `None` for a bay nothing has polled and for a
    /// pattern with no lanes to stand over.
    pub playhead: Option<Rect>,
    /// One per lane, in the order the pattern draws them.
    pub rows: Vec<SeqRow>,
    /// Which bank these rows are, carried so a caller's operation names the bank it
    /// acted on rather than implying the armed one (`Operation::SelectDeck`'s
    /// rule).
    pub bank: usize,
    /// The four bank pills in the bay head, in bank order — the same capsules
    /// [`bay_head`] paints, laid out a second time here ([`bank_capsules`]).
    ///
    /// Empty for a bay too short to hold its own head, which is a head with no
    /// capsule to press; short of four it never is, because renumbering the ones
    /// that fit would put `seq 2`'s press on `seq 1`.
    pub banks: Vec<Rect>,
    /// The foot's `+ lane` pill. A press puts [`LaneCard`] down; it emits nothing
    /// on its own, because what is being added is *what the lane drives* and a lane
    /// with nothing to point at emits nothing.
    pub add: Rect,
    /// The chooser's card, or `None` while it is up.
    pub card: Option<LaneCard>,
}

/// The `+ lane` chooser's card, laid out — [`RowMenu`]'s shape one bay along,
/// and the same mechanism (ADR-0311): a card of items with a separator in it,
/// hanging off the control that opened it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LaneCard {
    /// The card itself.
    pub card: Rect,
    /// How many of the items are faders, which is where the rule goes.
    pub faders: usize,
    /// How many items there are altogether — [`Choices::items`]' length.
    pub items: usize,
    /// Whether the separator is drawn, which is whether there is anything on both
    /// sides of it.
    pub ruled: bool,
}

impl LaneCard {
    /// Where one item is, from the top of the card — the faders stacked with no
    /// gap, then the band, then the parameters. [`RowMenu::load`]'s own reading,
    /// with the rule in the middle rather than at the end.
    ///
    /// Panics on an item this card has not got, which is that method's rule: a
    /// caller has invented a target.
    pub fn item(&self, index: usize) -> Rect {
        assert!(
            index < self.items,
            "item {index} of a card of {}",
            self.items
        );
        let band = match self.ruled && index >= self.faders {
            true => size::ROW_MENU_RULE_H,
            false => 0.0,
        };
        Rect::from_min_size(
            Pos2::new(
                self.card.min.x + size::LIB_LIST_PAD,
                self.card.min.y + size::LIB_LIST_PAD + size::LIB_ROW_H * index as f32 + band,
            ),
            egui::vec2(
                self.card.width() - size::LIB_LIST_PAD * 2.0,
                size::LIB_ROW_H,
            ),
        )
    }

    /// The separator's band, or `None` where none is drawn — the air, hairline and
    /// air [`RowMenu::rule`] draws, between the faders and the parameters. It takes
    /// no press: a press inside it is the dismissal.
    pub fn rule(&self) -> Option<Rect> {
        self.ruled.then(|| {
            Rect::from_min_size(
                Pos2::new(
                    self.card.min.x + size::LIB_LIST_PAD,
                    self.card.min.y + size::LIB_LIST_PAD + size::LIB_ROW_H * self.faders as f32,
                ),
                egui::vec2(
                    self.card.width() - size::LIB_LIST_PAD * 2.0,
                    size::ROW_MENU_RULE_H,
                ),
            )
        })
    }

    /// Which item `p` is on, or `None` for a point on the card's padding, on the
    /// separator, or off the card altogether — [`RowMenu::picked`]'s own answer.
    pub fn picked(&self, p: karakuri_layout::Point) -> Option<usize> {
        let at = Pos2::new(p.x, p.y);
        (0..self.items).find(|index| self.item(*index).contains(at))
    }
}

/// What a press on the `+ lane` control asks for.
///
/// [`Aim`]'s shape three bays along, and the same division: every arm is either
/// this console's own state moving or one named operation, and never a lane
/// appended here
/// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
#[derive(Debug, Clone, PartialEq)]
pub enum Chose {
    /// Put the card down — a press on `+ lane` with it up.
    Open,
    /// Take it away — a press on `+ lane` again, on the card's own ground, on the
    /// separator, or anywhere else while it is down. The press is spent on the
    /// dismissal, which is [`crate::input::claim`]'s rule 2 said in the control.
    Shut,
    /// The lane, named — `Operation::PointLane { pattern, target }`, with the bank
    /// off this bay's own reading and the target off the item.
    Point(Operation),
}

impl Sequencer {
    /// What a press at `p` asks for, or `None` where there is nothing under it.
    ///
    /// Four controls and one answer, in the order the mock draws them: a bank pill
    /// chooses the pattern, a cell sets a step, a label mutes a lane, and the pill
    /// chooses what a step is worth. The mode pill is asked last and none of the
    /// four can overlap another, so the order is arbitrary rather than a precedence
    /// — it is written down so that this file and the window that acts on it ask in
    /// one order.
    ///
    /// The `+ lane` control is not here, because its press is not an operation: it
    /// puts a card down, and what comes back from that card is
    /// [`Sequencer::chose`]. That is [`LibraryBay::aim`]'s division one bay along,
    /// and the same one: a control whose press moves the console's own state
    /// answers an enum rather than an `Option<Operation>`.
    ///
    /// Every arm names the bank, which is why [`Sequencer::bank`] is carried:
    /// implying the armed one is the shape `Operation::SelectDeck`'s rule refuses,
    /// and a press that arrived while a bank press was in flight would otherwise
    /// land on whichever pattern won.
    pub fn press(&self, p: karakuri_layout::Point) -> Option<Operation> {
        let at = Pos2::new(p.x, p.y);
        // **The bank pills first, and they are in the bay head** — outside
        // every rectangle below, so this is the order the mock reads in and
        // not a precedence either.
        //
        // **A press asks for that bank and never for the next one**, which is
        // the `+`'s whole argument turned round: with four fixed banks a press
        // on `seq 3` *is* the choice landing on an empty pattern, so there is
        // nothing left for a `+` to mean (ADR-0320, ADR-0327).
        if let Some(bank) = self.banks.iter().position(|pill| pill.contains(at)) {
            return Some(Operation::SelectPattern {
                pattern: bank as u8,
            });
        }
        for (index, row) in self.rows.iter().enumerate() {
            if let Some(cell) = row.cells.iter().position(|cell| cell.contains(at)) {
                return Some(Operation::SetStep {
                    pattern: self.bank as u8,
                    lane: index as u8,
                    // **The stored slot and not the drawn step**, which is
                    // what keeps the payload independent of the mode: at an
                    // eighth this sends `2k`, so a step press and a mode press
                    // cannot race into an address that means two things.
                    step: row.slots[cell] as u8,
                    // **A state and never a flip**, which is the cell's own
                    // rule: the press asks for that step to be on, or for it
                    // to be off, and a control that could only flip has no way
                    // to arrive.
                    on: !row.on[cell],
                });
            }
            if row.label.contains(at) {
                return Some(Operation::SetLaneMute {
                    pattern: self.bank as u8,
                    lane: index as u8,
                    muted: !row.muted,
                });
            }
        }
        self.mode_pill
            .contains(at)
            .then_some(Operation::SetPatternGrid {
                pattern: self.bank as u8,
                // **The other of the two, named**: the cycle is the surface's
                // affordance and the operation carries where it arrived
                // (P-0090).
                grid: match self.mode {
                    StepMode::Sixteenth => StepMode::Eighth,
                    StepMode::Eighth => StepMode::Sixteenth,
                },
            })
    }

    /// What a press at `p` asks of the `+ lane` control, or `None` where
    /// the press was on nothing it owns.
    ///
    /// # Two questions, and which one it is depends on whether the card is down
    ///
    /// [`LibraryBay::menu_ask`]'s rule, and it is that method's word for word:
    ///
    /// - With the card up this is the pill alone — a press on it opens the
    ///   card, and a press anywhere else answers `None` so the arms above can
    ///   have it.
    /// - With one down every press is the card's, which is
    ///   [`crate::input::claim`]'s rule 2: on an item it picks, on the
    ///   separator, on the card's padding or anywhere else on the console it
    ///   dismisses. So this never answers `None` while the card is down.
    ///
    /// A pick names the bank this bay is reading, exactly as
    /// [`Sequencer::press`]'s arms do: the lane lands in the pattern that was
    /// drawn rather than in whichever is armed by the time it is performed.
    pub fn chose(&self, p: karakuri_layout::Point, choices: &Choices) -> Option<Chose> {
        let Some(card) = self.card else {
            return self
                .add
                .contains(Pos2::new(p.x, p.y))
                .then_some(Chose::Open);
        };
        Some(match card.picked(p) {
            Some(item) => match choices.items.get(item) {
                Some(choice) => Chose::Point(Operation::PointLane {
                    pattern: self.bank as u8,
                    target: choice.target.clone(),
                }),
                // **A card drawn from a longer list than the one handed in
                // here** is a caller asking two questions of two readings, and
                // the dismissal is the answer that invents nothing — the
                // re-check `LibraryBay::menu_ask` does against its listing,
                // for its reason.
                None => Chose::Shut,
            },
            None => Chose::Shut,
        })
    }

    /// Whether `p` is on anything here a press means something on, which is what
    /// [`crate::input::claim`] asks. The ruler, the readout and the playhead are
    /// readouts and answer `false`.
    ///
    /// The `+ lane` pill is one of them, and the card is not: a card that is down
    /// claims every press on the console under rule 2, which is answered before
    /// rule 4 is reached and is why this is only ever asked with the card up.
    pub fn owns(&self, p: karakuri_layout::Point) -> bool {
        self.press(p).is_some() || self.add.contains(Pos2::new(p.x, p.y))
    }

    /// How many controls this bay draws, which is what [`crate::input::PROBES`]
    /// registers: a cell per drawn step of every lane, a label per lane, the mode
    /// pill, the four bank pills and `+ lane`.
    pub fn controls(&self) -> usize {
        self.rows
            .iter()
            .map(|row| row.cells.len() + 1)
            .sum::<usize>()
            + 1
            + self.banks.len()
            + 1
    }
}

/// The Sequencer bay, painted.
///
/// Where everything goes is [`sequencer`]'s, so this paints and derives
/// nothing. Term for term from `style.css`:
///
/// - the mode pill — `.pill.armed`, because *"it is armed because it is what
///   the pattern is rather than a preference the head is holding"*.
/// - the step readout — `.seq-head`'s own `color: var(--c-faint)` with the
///   figure in `.val`'s ink, which is what the mock draws.
/// - the ruler — `.seq-ruler`, four numbers centred over the cells they start,
///   at [`size::SEQ_RULER_SIZE`] in the faint ink. Four numbers whatever the
///   mode, because the ruler counts *beats* and a bar has four of them: at
///   an eighth they group two cells rather than four, which is the same bar
///   read at the other width.
/// - the playhead — `.seq-play .lane i.at`, a wash of the lavender with its
///   own hairline, painted under the rows so a lit cell stays the colour
///   its lane is.
/// - a lane's label — `.seq-label`, right-aligned, with the mark in the
///   lavender; and `.seq-row.mute`'s faint ink where the lane is muted.
/// - a cell — `.seq-lane i`, the well with its hairline; `.on` in the mint;
///   `.on.hot` in the pink where the lane drives the deck on air, which this
///   console cannot know here and so does not draw; and `.seq-row.mute`'s
///   `opacity: 0.3` over the whole row.
pub(super) fn sequencer_into(ui: &Ui, pal: &Palette, bay: &Sequencer) {
    let painter = ui.painter();
    // **The playhead first**, which is what `.seq-play` sitting before the
    // rows in the mock's markup means once the rows are opaque: a wash under
    // the cells rather than over them.
    if let Some(column) = bay.playhead {
        painter.rect_filled(
            column,
            CornerRadius::same(size::SEQ_PLAY_RADIUS),
            tint(pal.lav, PLAYHEAD_WASH),
        );
        painter.rect_stroke(
            column,
            CornerRadius::same(size::SEQ_PLAY_RADIUS),
            Stroke::new(size::HAIRLINE, pal.lav),
            StrokeKind::Inside,
        );
    }
    pill_into(ui, pal, bay.mode_pill, bay.mode.name(), true);
    let galley = painter.layout_no_wrap(
        bay.step_words.clone(),
        FontId::new(size::BASE, FontFamily::Proportional),
        pal.dim,
    );
    painter.galley(
        Pos2::new(bay.step.min.x, bay.step.center().y - galley.size().y * 0.5),
        galley,
        pal.dim,
    );
    // **The ruler's four numbers**, centred over the cell each group starts.
    // The groups are the bar's beats, so this is the count of beats and not of
    // cells — `Transport::grid`'s four, arrived at from the other side.
    let cells = bay.rows.first().map(|row| row.cells.len()).unwrap_or(0);
    if cells > 0 {
        let per_beat = (cells / RULER_GROUPS).max(1);
        for beat in 0..RULER_GROUPS {
            let at = beat * per_beat;
            if at >= cells {
                break;
            }
            let over = bay.rows[0].cells[at];
            let galley = painter.layout_no_wrap(
                format!("{}", beat + 1),
                FontId::new(size::SEQ_RULER_SIZE, FontFamily::Proportional),
                pal.faint,
            );
            painter.galley(
                Pos2::new(
                    over.center().x - galley.size().x * 0.5,
                    bay.ruler.center().y - galley.size().y * 0.5,
                ),
                galley,
                pal.faint,
            );
        }
    }
    for row in &bay.rows {
        let ink = match row.muted {
            true => pal.faint,
            false => pal.text,
        };
        let galley = painter.layout_no_wrap(
            row.words.clone(),
            FontId::new(size::SEQ_LABEL_SIZE, FontFamily::Proportional),
            ink,
        );
        // `.seq-label`'s `justify-content: flex-end`: the name is right
        // against the cells, so the letters line up down the column.
        painter.galley(
            Pos2::new(
                row.label.max.x - galley.size().x,
                row.label.center().y - galley.size().y * 0.5,
            ),
            galley,
            ink,
        );
        for (at, cell) in row.cells.iter().enumerate() {
            let radius = CornerRadius::same(size::SEQ_CELL_RADIUS);
            match row.on[at] {
                true => {
                    let lit = match row.muted {
                        true => tint(pal.mint, MUTED_LANE),
                        false => pal.mint,
                    };
                    painter.rect_filled(*cell, radius, lit);
                }
                false => {
                    painter.rect_filled(*cell, radius, pal.well);
                    painter.rect_stroke(
                        *cell,
                        radius,
                        Stroke::new(size::HAIRLINE, pal.hair),
                        StrokeKind::Inside,
                    );
                }
            }
        }
    }
    // **The foot's `+ lane`**, an ordinary `.pill`: it is an act and not a
    // state, so it is never armed — the mock draws it plain beside a `.sep`.
    pill_into(ui, pal, bay.add, ADD_LANE, false);
}

/// The `+ lane` chooser's card, painted — [`library::row_menu_into`]'s card
/// term for term, because it is that card: the same panel fill, radius,
/// hairline and item ink, and the same separator drawn as one rule inside its
/// band.
pub(super) fn lane_card_into(ui: &Ui, pal: &Palette, card: &LaneCard, choices: &Choices) {
    let painter = ui.painter();
    painter.add(pal.shadow.as_shape(card.card, CornerRadius::same(8)));
    painter.rect_filled(card.card, CornerRadius::same(8), pal.panel);
    painter.rect_stroke(
        card.card,
        CornerRadius::same(8),
        Stroke::new(size::HAIRLINE, pal.line),
        StrokeKind::Inside,
    );
    // **Zipped against the reading rather than counted to `card.items`**, so a
    // card laid out from a longer list than the one being painted draws the
    // items that exist instead of panicking on the geometry.
    for (index, choice) in choices.items.iter().enumerate().take(card.items) {
        let at = card.item(index);
        let galley = painter.layout_no_wrap(
            choice.words.clone(),
            FontId::new(size::BASE, FontFamily::Proportional),
            pal.dim,
        );
        painter.galley(
            Pos2::new(
                at.min.x + size::LIB_ROW_PAD_X,
                at.center().y - galley.size().y * 0.5,
            ),
            galley,
            pal.dim,
        );
    }
    if let Some(band) = card.rule() {
        let rule = band.center().y;
        painter.line_segment(
            [
                Pos2::new(band.min.x + size::LIB_ROW_PAD_X, rule),
                Pos2::new(band.max.x - size::LIB_ROW_PAD_X, rule),
            ],
            Stroke::new(size::HAIRLINE, pal.hair),
        );
    }
}

/// The word on the foot's pill, the mock's own — `+ lane`.
const ADD_LANE: &str = "+ lane";

/// What a fader item says it is, after the lane label the pick will make: `A ▮
/// fader`.
///
/// A parameter item says the node and the published name instead — `B ∿ L2:0
/// twist`, which is the mock's own way of naming the fourth lane's target.
pub(crate) const FADER_ITEM: &str = "fader";

/// How many numbers the ruler draws: four, which is the beats in a bar.
///
/// It is the bar's own count rather than a division of the cells, which is what
/// makes the ruler read the same in both modes — four numbers over sixteen
/// cells is a group of four, and over eight is a group of two.
/// `docs/manual/console.html` draws exactly this: *"Four numbers over sixteen
/// cells makes a group of four, which is a bar of sixteenths counted in
/// beats."*
const RULER_GROUPS: usize = 4;

/// `.seq-play .lane i.at`'s `color-mix(in srgb, var(--c-lav) 22%,
/// transparent)`, as the percentage [`tint`] takes.
const PLAYHEAD_WASH: u8 = 22;

/// `.seq-row.mute .seq-lane`'s `opacity: 0.3`, applied to a lit cell as the
/// percentage [`tint`] takes — the pattern is kept and drives nothing, so its
/// steps are still drawn and are drawn dim.
const MUTED_LANE: u8 = 30;

/// How stale the sequencer's picture may get, which is what this bay declares
/// under
/// [P-0091](../../../../docs/principles/0091-cost-is-known-before-it-is-paid.md)
/// and what the harness turns into a deadline.
///
/// One sixteenth at the mock's tempo — 117.19 ms, which is [`BEAT_MICROS`]
/// quartered. The unit this picture moves in is a whole cell: the playhead
/// stands over one step and then over the next, so there is nothing between two
/// positions to be smooth about and the step *is* the step. The finer of the
/// two modes is the one written down, because a declaration made for the eighth
/// would be half the rate the sixteenth needs and the mode is one press away.
///
/// Stated at the mock's tempo, for [`BEAT_STALENESS`]'s reason: a staleness
/// that fell with the tempo would make `Σ (cost / staleness)` a function of how
/// fast the music is, and the two schedulability conditions could then only be
/// asserted against a fastest tempo nobody has written down (ADR-0212). What
/// the music moves is [`step_moves_in`], which is the deadline and not the
/// rate.
///
/// It is the first declaration on this panel whose unit is a beat subdivision,
/// so it is what `moves_in >= staleness` is tightest against (ADR-0322,
/// ADR-0283).
pub const STEP_STALENESS: Duration = Duration::from_micros(BEAT_MICROS / 4);

/// How long until the playhead next stands over a different cell, from the beat
/// count and the tempo the transport row is drawing.
///
/// The step index is `floor(beats × steps_per_beat)`, so the next boundary is
/// the next whole multiple of the subdivision and this is the distance to it in
/// seconds — the same arithmetic the producer polls with
/// (`karakuri_pattern::Pattern::step_at`), read forwards.
///
/// It never answers finer than the rate it declared, which is
/// [`roll_moves_in`]'s rule and [`crate::budget::Declared`]'s invariant: a
/// frame taken a hair before a boundary would otherwise ask for a deadline
/// tending to zero, which is the spin [`crate::repaint`] exists to refuse.
///
/// A pure function of its two arguments, so a test chooses the beat it asserts
/// at and nothing here reads a clock.
pub fn step_moves_in(mode: StepMode, beats: f64, bpm: f32) -> Duration {
    // A grid at no tempo has no next boundary, and the rate this declared is
    // the only honest answer — the same shape as a rest longer than the period
    // one bay up.
    if bpm <= 0.0 || !bpm.is_finite() {
        return STEP_STALENESS;
    }
    let per_beat = mode.steps_per_beat();
    let at = beats * per_beat;
    let left = (at.floor() + 1.0 - at) / per_beat * 60.0 / f64::from(bpm);
    // `left` is positive and at most one step, so this is a duration and never
    // a negative one; the clamp is the invariant rather than a guard against
    // the arithmetic.
    Duration::from_secs_f64(left.max(0.0)).max(STEP_STALENESS)
}
