use egui::{Color32, FontFamily, FontId, Pos2, Rect};
use karakuri_operation::Operation;

use super::super::super::*;
use super::*;

// ---------------------------------------------------------------------------
// LibraryBay row layout and hit-testing
// ---------------------------------------------------------------------------

impl LibraryBay {
    /// The `index`th row's rectangle, with [`scroll`](Self::scroll) already taken
    /// off — so a row above the list has a negative-going top and one below it a
    /// top past `list.max.y`.
    ///
    /// Derived rather than stored for [`TransportRow::dot`]'s reason: the rows are
    /// a stride and a count, and a `Vec` of them would be an allocation a frame
    /// does not need.
    ///
    /// Every index in the listing is an answer, where this used to be a caller's
    /// error past [`rows`](Self::rows): a scrolled bay has rows off both edges and
    /// [`drawn`](Self::drawn) is what says which of them reach the picture, so the
    /// rectangle has to exist before that question can be asked.
    /// [`InspectorPane::group`] is the same change one bay over (ADR-0312,
    /// ADR-0307).
    pub fn row(&self, index: usize) -> Rect {
        Rect::from_min_size(
            Pos2::new(
                self.list.min.x,
                self.list.min.y - self.scroll + size::LIB_ROW_H * index as f32 + self.pushed(index),
            ),
            egui::vec2(self.list.width(), size::LIB_ROW_H),
        )
    }

    /// Which rows reach the picture, as a range into the listing — the ones a
    /// scrolled list has any of on screen, cut edges included.
    ///
    /// It is what [`library_into`] paints and what [`LibraryBay::take`],
    /// [`LibraryBay::land`], [`LibraryBay::starred`] and [`LibraryBay::menu_ask`]
    /// walk, so a control is hit-tested over exactly the rows that were drawn. It
    /// is not [`rows`](Self::rows), which counts the whole ones and is the foot's
    /// number: a star in a row cut by the bottom edge is drawn and is pressable,
    /// and the row it is in is not counted.
    ///
    /// Arithmetic rather than a walk, which is where this parts company with
    /// [`InspectorPane::drawn`]: the rows are a stride, and the one thing that is
    /// not is the reading block, which [`pushed`](Self::pushed) already holds. So
    /// the range is found by asking the two ends rather than by walking every row
    /// of a listing a store answered.
    pub fn drawn(&self) -> std::ops::Range<usize> {
        let touching = |index: usize| {
            let row = self.row(index);
            row.max.y > self.list.min.y && row.min.y < self.list.max.y
        };
        let first = (0..self.total).find(|index| touching(*index));
        match first {
            None => 0..0,
            // **From the first one that touches, and it is a run**: every row
            // after it either touches or is below the list, so the end is the
            // first that does not.
            Some(first) => {
                let end = (first..self.total)
                    .find(|index| !touching(*index))
                    .unwrap_or(self.total);
                first..end
            }
        }
    }

    /// How far a reading pushes the `index`th row down, which is nothing at all for
    /// every row above it and the whole block for every row below.
    ///
    /// The block is between two rows and not over them, which is what makes this a
    /// mode of the list rather than a card drawn on top of one: the rows under the
    /// cursor keep their order and their stride and start lower down. So the block
    /// scrolls with them — it is part of [`library_content_h`]'s content, and the
    /// rows it pushes past the bottom are one notch of the wheel away rather than
    /// out of reach (ADR-0312). The foot's `n of m` says how many are whole, in the
    /// words it says it in for a library taller than its list.
    fn pushed(&self, index: usize) -> f32 {
        match self.reading {
            Some(block) if index >= block.under => {
                size::READING_MARGIN_TOP + block.well.height() + size::READING_MARGIN_BOTTOM
            }
            _ => 0.0,
        }
    }

    /// The `index`th row's star, at the left of the row inside `.lib-row`'s own
    /// padding — [`STAR_SIZE`] square, centred across the row's height.
    ///
    /// Derived rather than stored for [`LibraryBay::row`]'s reason, and off that
    /// method rather than off the list, so a star follows the row a reading pushed
    /// down exactly as the name beside it does.
    ///
    /// One derivation for the paint and the press, which is [`LibraryBay::load`]'s
    /// rule two capsules along: `library_into` paints from this and
    /// [`LibraryBay::starred`] hit-tests it, so the mark a press lands on is the
    /// mark that is drawn.
    pub fn star(&self, index: usize) -> Rect {
        let row = self.row(index);
        Rect::from_min_size(
            Pos2::new(
                row.min.x + size::LIB_ROW_PAD_X,
                row.center().y - STAR_SIZE * 0.5,
            ),
            egui::vec2(STAR_SIZE, STAR_SIZE),
        )
    }

    /// Where the name in the `index`th row starts, which is one star and one
    /// [`STAR_GAP`] in from where it used to.
    ///
    /// It is derived here rather than at the paint so that the mark and the word
    /// are placed by one arithmetic — `.lib-row` is a flex row, and a name laid out
    /// from the row's padding while the star was laid out from the same padding
    /// would draw the two on top of each other.
    pub(crate) fn named(&self, index: usize) -> f32 {
        self.star(index).max.x + STAR_GAP
    }

    /// What a press at `p` on a row's star asks for, or `None` where there is no
    /// star under it.
    ///
    /// # It names the state, and the state is the one the row is not in
    ///
    /// `Operation::SetFavourite { id, favourite }` is not a toggle (ADR-0299) — *"a
    /// map with a button per direction, a model that says which one it wants and a
    /// key all have to be able to say star this and mean it"* — so the control is
    /// what reads the row's present state and asks for the other one. That is
    /// [`TransitionRow::shape`]'s division three bays along: the press names where
    /// it arrived, and the arithmetic that got it there is the surface's
    /// ([P-0090]).
    ///
    /// [P-0090]: ../../../docs/principles/0090-a-surface-offers-it-never-decides.md
    ///
    /// # The listing and the marks both go in with the point
    ///
    /// [`LibraryBay::take`]'s arrangement one control along and for its reason: the
    /// operand is a name this crate reads no store for (ADR-0156), so the rows the
    /// host handed in are what a row index means, and which of them are starred is
    /// the host's answer too — see [`View::starred`].
    ///
    /// A listing shorter than the rows drawn asks nothing, which is `take`'s
    /// refusal rather than a clamp: a star answered bare would name a Set nobody
    /// can see. A procedure row has no star and answers nothing, which is
    /// [`Rows::set`]'s whole job: a star is refused on an id `<store>/sets/` does
    /// not hold — `StoreError::NoSet`, ADR-0299 — so a procedure row draws the
    /// column with nothing in it and a press there is a press on the row.
    pub fn starred(
        &self,
        rows: Rows<'_>,
        marks: &std::collections::BTreeSet<String>,
        p: karakuri_layout::Point,
    ) -> Option<Operation> {
        let p = Pos2::new(p.x, p.y);
        // **The star's own box, inside the list** — `at_row`'s bound stated on
        // the smaller rectangle: a star belongs to a row, and a row off the
        // top of the list has one.
        let row = self
            .drawn()
            .find(|index| self.list.contains(p) && self.star(*index).contains(p))?;
        let id = rows.set(row)?;
        Some(Operation::SetFavourite {
            id: id.to_owned(),
            favourite: !marks.contains(id),
        })
    }

    /// Which row `p` is on, or `None` for a point on the list's own ground, on the
    /// reading between two rows, or outside the list altogether.
    ///
    /// # It is bounded by the list and not only by the row
    ///
    /// A scrolled bay has rows off both edges and [`LibraryBay::row`] answers for
    /// every one of them, so a rectangle under the filter fields or over the foot
    /// is a rectangle a press could land in while the row it belongs to is not on
    /// screen. That is [`InspectorPane::grip`]'s *refuses a press outside the body*
    /// one bay over, and it is the one thing here that would fail silently: the bay
    /// would claim a press on a row nobody can see, under controls that are drawn
    /// there (ADR-0312, ADR-0307).
    ///
    /// [`drawn`](Self::drawn) and not [`rows`](Self::rows), so a press on the
    /// visible half of a cut row reaches it — the row is drawn, and a control
    /// claims what it acts on.
    fn at_row(&self, p: Pos2) -> Option<usize> {
        if !self.list.contains(p) {
            return None;
        }
        self.drawn().find(|index| self.row(*index).contains(p))
    }

    /// What the foot reads: `n of m`, the mock's own `5 of 27`.
    pub fn count(&self) -> String {
        format!("{} of {}", self.rows, self.total)
    }

    /// A capsule `width` wide at the far end of the foot, which is where the
    /// pulldown goes and what everything else in the row is measured back from.
    ///
    /// `.lib-foot` is a flex row of the count, a `.sep { flex: 1 }` and the three
    /// capsules, so the count is one [`size::LIB_FOOT_PAD_X`] in from the left and
    /// the last item is one in from the right with the whole of the leftover
    /// between them. Nothing else in the row has a width, so the spacer's share is
    /// the only arithmetic and it is a subtraction.
    ///
    /// Taken as an argument rather than derived, because a capsule is as wide as
    /// the words in it and this derivation asks `egui` for nothing — [`library`]'s
    /// own rule. The caller measures the galley it is about to paint and hands the
    /// number in, so the box the capsule is drawn in and the box a test asks about
    /// are one statement.
    pub fn pill(&self, width: f32) -> Rect {
        Rect::from_min_size(
            Pos2::new(
                self.foot.max.x - size::LIB_FOOT_PAD_X - width,
                self.foot.center().y - size::PILL_H * 0.5,
            ),
            egui::vec2(width, size::PILL_H),
        )
    }

    /// The foot's load control, measured and laid out: the `load` button, the `→`
    /// label between them, the pulldown and how many decks its list holds.
    ///
    /// One derivation for the paint and for the press, which is
    /// [`ArrangementPill`]'s arrangement one bay along: each capsule is as wide as
    /// what is in it, and two measurements would be a control drawn in one box and
    /// pressed in another. [`library_into`] paints from this,
    /// [`crate::input::claim`] hit-tests it and `tests/library.rs` asks it where
    /// the capsules are.
    ///
    /// Laid out from the right, because the pulldown is the far end of the row.
    /// `.lib-foot`'s `gap: 8px` ([`size::LIB_FOOT_GAP`]) is between every pair of
    /// children, so the label and the button are stepped back from the pulldown by
    /// it and [`LibraryBay::params_chip`] is stepped back from the button by it
    /// again. The pulldown is as wide as a one-letter deck name and the button is
    /// as wide as `load`, so a row measured forwards from the count would move both
    /// capsules whenever the letter did.
    ///
    /// Why it takes the context: a word's width is `egui`'s to answer and nobody
    /// else's, which is [`pill_width`]'s reason and [`mixer`]'s. Before the first
    /// pass there are no fonts, and a zero-width word makes a capsule of the
    /// padding and the mark — which is what a console that has drawn nothing has.
    pub fn load(&self, ctx: &egui::Context, at: Target) -> Load {
        let run = |text: &str| {
            if ctx.cumulative_pass_nr() == 0 {
                return egui::Vec2::ZERO;
            }
            ctx.fonts_mut(|f| {
                f.layout_no_wrap(
                    text.to_owned(),
                    FontId::new(size::BASE, FontFamily::Proportional),
                    Color32::PLACEHOLDER,
                )
                .size()
            })
        };
        let (word, mark) = (run(LOAD_PILL), run(at.letter()));
        let mid = self.foot.center().y;
        // **The pulldown**, and the gap inside it between the letter and the
        // chevron is the one gap the mock states inside a capsule — the same
        // `.sink` gap `arr · night ▾` puts between its words and its mark.
        let deck = self.pill(size::PILL_PAD_X * 2.0 + mark.x + size::SINK_GAP + CHEVRON_W);
        let letter = Rect::from_min_size(
            Pos2::new(deck.min.x + size::PILL_PAD_X, mid - mark.y * 0.5),
            mark,
        );
        let chevron = Rect::from_center_size(
            Pos2::new(deck.max.x - size::PILL_PAD_X - CHEVRON_W * 0.5, mid),
            egui::vec2(CHEVRON_W, CHEVRON_H),
        );
        // **The label, on the foot's own ground and on neither capsule.**
        let arrow = Rect::from_center_size(
            Pos2::new(deck.min.x - size::LIB_FOOT_GAP - LOAD_ARROW * 0.5, mid),
            egui::vec2(LOAD_ARROW, LOAD_ARROW),
        );
        let button = Rect::from_min_size(
            Pos2::new(
                arrow.min.x - size::LIB_FOOT_GAP - (word.x + size::PILL_PAD_X * 2.0),
                mid - size::PILL_H * 0.5,
            ),
            egui::vec2(word.x + size::PILL_PAD_X * 2.0, size::PILL_H),
        );
        Load {
            button,
            text: Rect::from_min_size(
                Pos2::new(button.min.x + size::PILL_PAD_X, mid - word.y * 0.5),
                word,
            ),
            arrow,
            deck,
            letter,
            chevron,
            // **Zero while it is shut**, which is what stops [`Load::row`]
            // handing out a rectangle for a list nobody opened.
            rows: match at.open {
                true => at.decks,
                false => 0,
            },
        }
    }

    /// The foot's `params` chip, laid out: the capsule between the count and the
    /// `load` button.
    ///
    /// `.lib-foot` is a flex row of the count, a `.sep { flex: 1 }` and the
    /// capsules one [`size::LIB_FOOT_GAP`] apart, so this is measured back from
    /// where [`LibraryBay::load`] put the button rather than forward from the
    /// count: the capsules to its right are as wide as the words and the letter in
    /// them, and a chip placed from the left would move whenever any of them did.
    ///
    /// One derivation for the paint and the press, which is [`LibraryBay::load`]'s
    /// own rule one capsule along — [`library_into`] paints this and
    /// [`LibraryBay::read`] hit-tests it, so the capsule a press lands on is the
    /// capsule the word is in.
    pub fn params_chip(&self, ctx: &egui::Context, at: Target) -> Rect {
        let load = self.load(ctx, at).button;
        let width = pill_width(ctx, PARAMS_PILL);
        Rect::from_min_size(
            Pos2::new(load.min.x - size::LIB_FOOT_GAP - width, load.min.y),
            egui::vec2(width, size::PILL_H),
        )
    }

    /// What a press at `p` on the `params` chip asks for, or `None` where there is
    /// no chip under it.
    ///
    /// # The operand is the cursor, which is the load button's operand
    ///
    /// `console.html`'s note: *"Its operand is the cursor, which is the same
    /// operand the pill beside it already uses — so the route costs one chip in the
    /// foot and nothing else"*. So `set` is the Set under the cursor, handed in the
    /// way every other reading of the listing is (ADR-0156), and what comes back
    /// names it.
    ///
    /// # The two answers, and why closing is not an operation
    ///
    /// See [`Read`]. Whether the press opens or closes is read off
    /// [`LibraryBay::reading`] — the block this bay is *drawing* — and not off
    /// anything this method is told, so the chip cannot answer *shut* for a reading
    /// nobody can see.
    ///
    /// A press with no row under the cursor asks nothing, which is a library that
    /// lists nothing: there is no Set to read and the chip is still drawn, because
    /// the foot is what the count is in. It is [`Mixer::grab`]'s answer for a press
    /// on a fader's track — a control claims what it acts on, and claiming a press
    /// to throw it away would put the rule and the act out of step.
    pub fn read(
        &self,
        ctx: &egui::Context,
        at: Target,
        set: Option<&str>,
        p: karakuri_layout::Point,
    ) -> Option<Read> {
        if ctx.cumulative_pass_nr() == 0 {
            return None;
        }
        if !self.params_chip(ctx, at).contains(Pos2::new(p.x, p.y)) {
            return None;
        }
        match self.reading {
            Some(_) => Some(Read::Shut),
            None => Some(Read::Open(Operation::ReadSet {
                id: set?.to_owned(),
            })),
        }
    }

    /// What a press at `p` on the foot's load control asks for, or `None`
    /// where the press was on nothing this control owns.
    ///
    /// The same derivation [`crate::input::claim`] hit-tests, asked a second
    /// time rather than copied — [`ArrangementPill::ask`]'s arrangement, and
    /// the reason is the same: the control that claims a press and the control
    /// that acts on it cannot come apart.
    ///
    /// # The three answers a press can give, and which mark each one moves
    ///
    /// - The pulldown puts its list down, or takes it away again. Neither
    ///   is an operation and neither moves a deck.
    /// - A row of that list is [`Aim::Deck`]: the target moves to the deck
    ///   that was picked and nothing is asked for, exactly as a press on a
    ///   row of the listing above asks for nothing (`LibraryBay::take`). The
    ///   deck selection does not move — that is the whole of what the second
    ///   mark is for, and `View::select` is not called from here.
    /// - The button is `Operation::LoadSet { deck, set }`, with the deck
    ///   off [`Target::deck`] and the Set off the cursor — or
    ///   `Operation::LoadProcedure` where the row under the cursor is a
    ///   procedure, which is one file written over what the deck is playing
    ///   rather than every layer replaced (ADR-0338). With no row under the
    ///   cursor it is [`Aim::NoSet`] and nothing is emitted: a load with one
    ///   operand missing is not a load, and answering `None` would leave the
    ///   press claimed and unaccounted for.
    ///
    /// While the list is down, every press is the dismissal, which is
    /// [`ArrangementPill::ask`]'s rule and `input::claim`'s rule 2: the card
    /// is drawn over this bay's own list, so a press on the rows underneath it
    /// belongs to the card and not to what it is covering. So the button
    /// answers [`Aim::Shut`] while the list is down rather than loading
    /// through it.
    ///
    /// `None` before the first pass, which is [`LibraryBay::read`]'s guard
    /// and [`mixer`]'s: there are no fonts until `egui` has run one, so there
    /// is no capsule width to measure and nothing has been drawn to press.
    pub fn aim(
        &self,
        ctx: &egui::Context,
        viewport: Rect,
        at: Target,
        rows: Rows<'_>,
        cursor: usize,
        p: karakuri_layout::Point,
    ) -> Option<Aim> {
        if ctx.cumulative_pass_nr() == 0 {
            return None;
        }
        let load = self.load(ctx, at);
        if load.hit_deck(p) {
            return Some(match at.open {
                true => Aim::Shut,
                false => Aim::Open,
            });
        }
        if at.open {
            return Some(match load.picked(viewport, p) {
                Some(deck) => Aim::Deck(deck),
                None => Aim::Shut,
            });
        }
        if !load.hit_button(p) {
            return None;
        }
        // **Which of the two loads it is is the row's, and the operand is the
        // same word either way.** A Set row names every layer of what the deck
        // will play; a procedure row names one file written over what it is
        // playing already (ADR-0338). The button, the row menu and the drag all
        // arrive at this pair, which is why the division is here rather than
        // three times over.
        Some(match (rows.name(cursor), rows.procedure(cursor)) {
            (Some(name), true) => Aim::Load(Operation::LoadProcedure {
                deck: at.deck,
                procedure: name.to_owned(),
            }),
            (Some(id), false) => Aim::Load(Operation::LoadSet {
                deck: at.deck,
                set: id.to_owned(),
            }),
            (None, _) => Aim::NoSet,
        })
    }

    /// A row's menu, laid out, or `None` where none is down.
    ///
    /// # It hangs down off the row, where the deck pulldown's card hangs up
    ///
    /// Both hang into the room there is, which is [`Load::list`]'s own rule read
    /// from the other end: that card stands on a capsule in the bay's foot and so
    /// has only this bay's list above it, and this one stands on a row of that list
    /// and has the rest of the list below it. It is inset from the row's left edge
    /// by [`size::ROW_MENU_INSET`] so that it hangs under the name that was pressed
    /// rather than under the star beside it, and it is held inside the viewport, so
    /// a press on the last row of a bay at the bottom of the window draws the card
    /// over the bay rather than off the screen.
    ///
    /// The rows are not counted against the room, which is [`Load::list`]'s clause
    /// and the same argument: this card lists at most [`DECKS`] loads and one send,
    /// and a window too short for five rows of type has no transport row in it
    /// either.
    ///
    /// Why it takes the context: the items are as wide as the words in them, which
    /// is `egui`'s to answer and nobody else's — [`LibraryBay::load`]'s reason one
    /// control along. `None` before the first pass for that reason too.
    pub fn menu(&self, ctx: &egui::Context, viewport: Rect, at: Menued) -> Option<RowMenu> {
        if ctx.cumulative_pass_nr() == 0 {
            return None;
        }
        let row = self.row(at.row?);
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
        let loads = at.decks.min(DECKS);
        // **The send is measured only where it is drawn**, which is
        // [`Menued::sends`]: a card as wide as `Save as a kbset` with no such
        // item in it would be a menu whose width said what it holds and was
        // wrong.
        let widest = (0..loads)
            .map(|deck| width(&load_item(deck as u8)))
            .chain(at.sends.then(|| width(MENU_SAVE)))
            .fold(size::ROW_MENU_MIN_W, f32::max);
        let sent = match at.sends {
            true => size::LIB_ROW_H + size::ROW_MENU_RULE_H,
            false => 0.0,
        };
        let height = size::LIB_LIST_PAD * 2.0 + size::LIB_ROW_H * loads as f32 + sent;
        let card = held_inside(
            &viewport,
            row.min.x + size::ROW_MENU_INSET,
            row.max.y,
            widest + (size::LIB_ROW_PAD_X + size::LIB_LIST_PAD) * 2.0,
            height,
        );
        let band = card.min.y + size::LIB_LIST_PAD + size::LIB_ROW_H * loads as f32;
        Some(RowMenu {
            card,
            loads,
            rule: at.sends.then(|| {
                Rect::from_min_size(
                    Pos2::new(card.min.x + size::LIB_LIST_PAD, band),
                    egui::vec2(
                        card.width() - size::LIB_LIST_PAD * 2.0,
                        size::ROW_MENU_RULE_H,
                    ),
                )
            }),
            save: at.sends.then(|| {
                Rect::from_min_size(
                    Pos2::new(
                        card.min.x + size::LIB_LIST_PAD,
                        band + size::ROW_MENU_RULE_H,
                    ),
                    egui::vec2(card.width() - size::LIB_LIST_PAD * 2.0, size::LIB_ROW_H),
                )
            }),
        })
    }

    /// What a press at `p` asks of a row's menu, or `None` where the press
    /// was on nothing this control owns.
    ///
    /// # Two questions, and which one it is depends on whether a menu is down
    ///
    /// - With none down this is *the secondary press*: which row it named,
    ///   and nothing else. A press on the list's own ground below the last row
    ///   answers `None`, exactly as [`LibraryBay::take`] does, and so does a
    ///   press on a row with no Set behind it — a `history` row is a version,
    ///   and every item this menu carries names a Set.
    /// - With one down every press is the card's, which is
    ///   [`crate::input::claim`]'s rule 2 and [`ArrangementPill::ask`]'s rule:
    ///   on an item it picks, on the separator or anywhere else it dismisses.
    ///   So this never answers `None` while a menu is down.
    ///
    /// Both operands are in hand here, which is why the arms carry whole
    /// operations: the deck is the item and the Set is `sets[at.row]`, the row
    /// the menu was opened on rather than the cursor's — a menu opened on the
    /// fourth row and picked at `Load to Slot C` loads the fourth Set onto
    /// deck C whatever the cursor and the pulldown are doing.
    ///
    /// The row is re-checked against the listing rather than trusted, for
    /// [`LibraryBay::take`]'s reason: a listing that shrank between the press
    /// that opened the menu and the press that picked from it would otherwise
    /// name a Set nobody can see. A menu over a row that has gone dismisses.
    pub fn menu_ask(
        &self,
        ctx: &egui::Context,
        viewport: Rect,
        at: Menued,
        rows: Rows<'_>,
        p: karakuri_layout::Point,
    ) -> Option<Picked> {
        if ctx.cumulative_pass_nr() == 0 {
            return None;
        }
        let Some(row) = at.row else {
            let row = self.at_row(Pos2::new(p.x, p.y))?;
            rows.name(row)?;
            return Some(Picked::Open(row));
        };
        let Some(menu) = self.menu(ctx, viewport, at) else {
            return Some(Picked::Shut);
        };
        let Some(name) = rows.name(row) else {
            return Some(Picked::Shut);
        };
        Some(match menu.picked(p) {
            // **The item names the deck and the row names which load it
            // is**, which is [`LibraryBay::aim`]'s division arriving by the
            // third route: four items on a procedure row load that one file
            // over that deck's layer, and four on a Set row load every layer
            // (ADR-0338).
            Some(RowItem::Load(deck)) => Picked::Load(match rows.procedure(row) {
                true => Operation::LoadProcedure {
                    deck,
                    procedure: name.to_owned(),
                },
                false => Operation::LoadSet {
                    deck,
                    set: name.to_owned(),
                },
            }),
            // **The send is a Set's and a procedure row has none.** What is
            // written out is that Set with every source it names inlined after
            // it, checked against the address each `slot` record carries — and
            // a loose `.kir` names nothing, which is ADR-0338's own reason a
            // `folder` lists no procedure. So the item is not drawn on such a
            // row (see [`Menued::sends`]) and a press where it would have been
            // dismisses.
            Some(RowItem::Save) => match rows.set(row) {
                Some(id) => Picked::Send(Operation::TransferSet {
                    transfer: karakuri_operation::SetTransfer::Send { id: id.to_owned() },
                }),
                None => Picked::Shut,
            },
            None => Picked::Shut,
        })
    }

    /// What a press at `p` on the list takes in hand, or `None` where there is no
    /// drawn row under it.
    ///
    /// # It answers a payload where every other control here answers an operation
    ///
    /// A press on a row asks for nothing yet. `console.html`'s *How a Set reaches a
    /// deck* has the gesture as a drag — *"Dragging a row onto a strip is a second
    /// route to the same command, and never the first … it names both operands in
    /// the one gesture"* — and half a gesture names one operand. So what comes back
    /// is the Set, on its way to [`crate::panel::Panel::carry`], and the operation
    /// is built at the drop where the second operand is
    /// ([`crate::panel::Released::Dropped`]).
    ///
    /// A press that is never dragged anywhere asks for nothing either, and that is
    /// the same sentence rather than a second rule: the row is picked up, carried
    /// nowhere, and let go over nothing.
    ///
    /// # The listing goes in with the point
    ///
    /// [`LibraryBay::read`]'s arrangement one row up, and for the reason that
    /// method states: the operand is a name this crate reads no store for
    /// (ADR-0156), so the rows the host handed in are what a row index means.
    /// Handing them in is also what makes this refuse rather than clamp — a listing
    /// shorter than the rows drawn takes nothing in hand, where an index answered
    /// bare would name a Set nobody can see.
    ///
    /// The rows are walked rather than divided. A row's stride is
    /// [`size::LIB_ROW_H`] and a reading pushes the rows under it down by a whole
    /// block, so *which row is at `y`* is not one division — [`LibraryBay::row`]
    /// already holds that arithmetic, and asking it per row is what keeps the row a
    /// press lands on the row the paint drew. Never more than [`LibraryBay::rows`]
    /// of them, so a press below the last row is on the list's own ground and
    /// belongs to nobody.
    pub fn take(&self, rows: Rows<'_>, p: karakuri_layout::Point) -> Option<Taken> {
        self.at_row(Pos2::new(p.x, p.y)).and_then(|row| {
            Some(Taken {
                row,
                set: rows.name(row)?.to_owned(),
                // **What is in hand says which load a drop names**, which is
                // the drag's half of [`LibraryBay::aim`]'s division: the
                // gesture names both operands and the second arrives at the
                // release, so the first has to carry what kind of row it was
                // (ADR-0338).
                procedure: rows.procedure(row),
            })
        })
    }

    /// What a press at `p` on a row of the `history` listing asks for, or `None`
    /// where there is no drawn row under it.
    ///
    /// # It is a load, and it is not [`Operation::LoadSet`]
    ///
    /// A row here is a version of one node rather than a Set, so what a press on it
    /// asks for is `Operation::RestoreProcedure` carrying that version — *put a
    /// node's previous version back*, which is the row
    /// `docs/manual/operations.html` names for landing and says so at the walk
    /// beside it: *"landing is not this row"*. Nothing else in this bay changes:
    /// the version is written over that node's working copy and the watcher builds
    /// it, so the load is the path an edit already takes (ADR-0228).
    ///
    /// # Both operands are marks this console keeps
    ///
    /// The deck is [`Target::deck`] — the pulldown in the foot, which is what
    /// narrowed the listing to a Set in the first place, so the rows a hand is
    /// looking at and the deck a press lands on cannot come apart. The version is
    /// the row, by the word the host handed in: this crate reads no store
    /// (ADR-0156), and the name is matched back against the listing that produced
    /// it, which is `SetTransfer::Take`'s arrangement and the rule that keeps a
    /// path out of a payload.
    ///
    /// # It takes the versions and [`LibraryBay::take`] takes the Sets
    ///
    /// The two are one press on one rectangle and they are told apart by which
    /// listing is handed in: [`View::sets`] is empty under `history` and
    /// [`View::versions`] is empty everywhere else, so exactly one of them can
    /// answer and neither has to be told what the scope is. A carry of a version
    /// would be a Set named by a word no store holds, and a landing on a Set would
    /// be a node named by a word that addresses none.
    ///
    /// The rows are walked rather than divided, which is `take`'s rule and for its
    /// reason: a reading pushes the rows under it down by a whole block, so
    /// [`LibraryBay::row`] is the arithmetic and asking it per row is what keeps
    /// the row a press lands on the row the paint drew.
    pub fn land(
        &self,
        versions: &[String],
        at: Target,
        p: karakuri_layout::Point,
    ) -> Option<Operation> {
        let row = self.at_row(Pos2::new(p.x, p.y))?;
        Some(Operation::RestoreProcedure {
            deck: at.deck,
            revision: karakuri_operation::Revision::Picked(versions.get(row)?.clone()),
        })
    }

    /// The `index`th row's badges, right to left from the row's own padding — one
    /// box per word, in the order [`Rows::badges`] hands them and laid out so the
    /// last word ends where the row's padding starts.
    ///
    /// A badge is as wide as the word in it, which is [`kind_chips`]' sentence one
    /// row up: `.badge` is `font-size: 8px; padding: 0 4px`, so the box is the word
    /// at [`size::BADGE_SIZE`] inside [`size::BADGE_PAD_X`] either side, and
    /// `.badges`' `gap: 3px` is [`size::BADGE_GAP`].
    ///
    /// One derivation for the paint and the hover, which is [`LibraryBay::load`]'s
    /// rule: `library_into` paints from this and `hover`'s probe asks it whether
    /// the pointer is on one, so the readout a tip explains is the readout that is
    /// drawn. Nothing presses it — what narrows the list by kind is the row of
    /// chips above.
    ///
    /// Laid out from the right, because the mock puts the badges at the end of the
    /// row after the name: a row measured forwards from the name would move every
    /// badge whenever a name got longer.
    pub fn badges<'a>(
        &self,
        ctx: &'a egui::Context,
        index: usize,
        words: &'a [&'static str],
    ) -> impl Iterator<Item = (&'static str, Rect)> + 'a {
        let row = self.row(index);
        let widths: Vec<f32> = if ctx.cumulative_pass_nr() == 0 {
            words.iter().map(|_| size::BADGE_PAD_X * 2.0).collect()
        } else {
            words
                .iter()
                .map(|word| {
                    ctx.fonts_mut(|f| {
                        f.layout_no_wrap(
                            (*word).to_owned(),
                            FontId::new(size::BADGE_SIZE, FontFamily::Proportional),
                            Color32::PLACEHOLDER,
                        )
                        .size()
                        .x
                    }) + size::BADGE_PAD_X * 2.0
                })
                .collect()
        };
        let whole: f32 =
            widths.iter().sum::<f32>() + size::BADGE_GAP * widths.len().saturating_sub(1) as f32;
        let mut x = row.max.x - size::LIB_ROW_PAD_X - whole;
        let top = row.center().y - size::BADGE_H * 0.5;
        words
            .iter()
            .zip(widths)
            .map(move |(word, width)| {
                let box_ = Rect::from_min_size(Pos2::new(x, top), egui::vec2(width, size::BADGE_H));
                x += width + size::BADGE_GAP;
                (*word, box_)
            })
            .collect::<Vec<_>>()
            .into_iter()
    }
}
