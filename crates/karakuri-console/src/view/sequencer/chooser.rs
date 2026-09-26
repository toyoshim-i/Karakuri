use super::*;

/// One selectable target in the `+ lane` chooser and its formatted label.
#[derive(Debug, Clone, PartialEq)]
pub struct LaneChoice {
    /// What a pick points the lane at — the payload of `Operation::PointLane`,
    /// whole.
    pub target: LaneTarget,
    /// The words on the item.
    pub words: String,
}

/// Available targets for the `+ lane` chooser (faders across decks and selected deck parameters) (ADR-0305, ADR-0321, ADR-0327).
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
    /// Returns the bounding rectangle for the item at `index`.
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

/// Result of activating the `+ lane` control (open, shut, or point operation) (P-0090).
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
    /// Evaluates clicks on the `+ lane` button or popup card at `p`.
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
                // Dismiss if choice index is invalid under current reading.
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

/// Target kind label appended for fader lane choices (e.g. `A ▮ fader`).
pub(crate) const FADER_ITEM: &str = "fader";
