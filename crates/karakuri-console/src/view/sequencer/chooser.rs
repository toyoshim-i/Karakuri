use super::*;

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
}

/// The `+ lane` chooser's card, painted — [`library::row_menu_into`]'s card
/// term for term, because it is that card: the same panel fill, radius,
/// hairline and item ink, and the same separator drawn as one rule inside its
/// band.
pub(crate) fn lane_card_into(ui: &Ui, pal: &Palette, card: &LaneCard, choices: &Choices) {
    let painter = ui.painter();
    popup_card(painter, pal, card.card);
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
pub(crate) const ADD_LANE: &str = "+ lane";

/// What a fader item says it is, after the lane label the pick will make: `A ▮
/// fader`.
///
/// A parameter item says the node and the published name instead — `B ∿ L2:0
/// twist`, which is the mock's own way of naming the fourth lane's target.
pub(crate) const FADER_ITEM: &str = "fader";
