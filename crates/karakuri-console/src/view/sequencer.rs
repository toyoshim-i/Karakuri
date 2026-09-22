use super::*;

pub mod chooser;
pub use chooser::*;

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
    /// The minus at the far end of the row, which takes this lane out of the
    /// pattern — the Master bay's chain slot glyph on a lane's row
    /// ([ADR-0352](../../../../docs/adr/0352-the-chains-list-is-the-master-bays-items-and-a-slot-is-taken-out-by-a-glyph-on-its-row.md)).
    /// The cells stop short of it, so the label, the track and the glyph are
    /// three columns and nothing on the row moves when the mode changes.
    pub remove: Rect,
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

    // Bay header bank pills matching header render geometry.
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
    // **The minus is at the far end of every lane's row**, where the chain
    // slot's sits on its head line: one column of glyphs against the bay's own
    // padding, so the track is what is left between the label and it
    // (ADR-0352).
    let minus_w = ctx.fonts_mut(|f| {
        f.layout_no_wrap(
            super::master::REMOVE_GLYPH.to_owned(),
            FontId::new(size::BASE, FontFamily::Proportional),
            Color32::PLACEHOLDER,
        )
        .size()
        .x
    });
    let track_right = right - minus_w - size::SEQ_ROW_GAP;
    if track_right <= track_x {
        return None;
    }
    let count = mode.count();
    let gaps = size::SEQ_CELL_GAP * (count as f32 - 1.0);
    let cell_w = (track_right - track_x - gaps) / count as f32;
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
        let remove = Rect::from_min_size(
            Pos2::new(right - minus_w, y),
            egui::vec2(minus_w, size::SEQ_CELL_H),
        );
        rows.push(SeqRow {
            label,
            cells,
            remove,
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

impl Sequencer {
    /// What a press at `p` asks for, or `None` where there is nothing under it.
    ///
    /// Five controls and one answer, in the order the mock draws them: a bank pill
    /// chooses the pattern, a cell sets a step, a label mutes a lane, the minus at
    /// the end of the row takes that lane out, and the pill chooses what a step is
    /// worth. The mode pill is asked last and none of the five can overlap another,
    /// so the order is arbitrary rather than a precedence — it is written down so
    /// that this file and the window that acts on it ask in one order.
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
            // **The minus takes the lane out**, addressed by the position it is
            // drawn at: the lanes after it move up, which is what a lane index
            // means (ADR-0352's own property, one bay along).
            if row.remove.contains(at) {
                return Some(Operation::RemoveLane {
                    pattern: self.bank as u8,
                    lane: index as u8,
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
    /// registers: a cell per drawn step of every lane, a label and a minus per
    /// lane, the mode pill, the four bank pills and `+ lane`.
    pub fn controls(&self) -> usize {
        self.rows
            .iter()
            .map(|row| row.cells.len() + 2)
            .sum::<usize>()
            + 1
            + self.banks.len()
            + 1
    }
}

/// Paints the sequencer bay, including mode pill, ruler, playhead, and lane cells.
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
        // **The minus, in the faint ink the chain slot's is drawn in** — it is
        // an act rather than a state, so it is never armed and never lit, and a
        // muted row dims it with everything else on the row.
        let glyph = painter.layout_no_wrap(
            super::master::REMOVE_GLYPH.to_owned(),
            FontId::new(size::BASE, FontFamily::Proportional),
            pal.faint,
        );
        painter.galley(
            Pos2::new(
                row.remove.max.x - glyph.size().x,
                row.remove.center().y - glyph.size().y * 0.5,
            ),
            glyph,
            pal.faint,
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

/// Computes duration until the playhead advances to the next step at current tempo.
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
