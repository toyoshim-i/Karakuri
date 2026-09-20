use karakuri_operation::{LaneTarget, Operation};

use super::*;
use crate::view::master::{AddChoice, AddChoices};
use crate::view::sequencer::{lane_label, Choices, LaneChoice, FADER_ITEM};

// ---------------------------------------------------------------------------
// View popup choices and lane/chain controls
// ---------------------------------------------------------------------------

impl View {
    /// What the `+ lane` chooser offers this frame — the one value [`sequencer`]
    /// lays its card out from, and [`View::target`]'s shape one bay along: read
    /// once for the frame and handed to the paint and to the press, so the item
    /// that is drawn and the item a press lands on are one derivation of one
    /// reading.
    ///
    /// The faders are every strip the mixer draws, which is [`View::select`]'s
    /// count read again; the parameters are one deck's, the load pulldown's
    /// ([`View::target_deck`]) — see [`Choices`], which carries the argument.
    pub fn lane_choices(&self) -> Choices {
        let mut items: Vec<LaneChoice> = (0..self.mixer.len().min(DECKS))
            .map(|deck| {
                let target = LaneTarget::Fader { deck: deck as u8 };
                LaneChoice {
                    words: format!("{} {FADER_ITEM}", lane_label(&target)),
                    target,
                }
            })
            .collect();
        let faders = items.len();
        let deck = self.target_deck();
        for pane in self
            .inspector
            .iter()
            .filter(|pane| pane.deck == usize::from(deck))
        {
            for node in &pane.nodes {
                for param in &node.params {
                    let target = LaneTarget::Param {
                        deck,
                        param: param.param.clone(),
                    };
                    items.push(LaneChoice {
                        // **The address and the published name**, which is the
                        // mock's own way of naming this lane — *"L2:0 twist on
                        // deck B"* — with the deck's letter and mark in front
                        // of it so the item reads as the row it will make.
                        words: format!("{} {} {}", lane_label(&target), node.addr, param.name),
                        target,
                    });
                }
            }
        }
        Choices {
            items,
            faders,
            open: self.lane_open,
        }
    }

    /// Whether the `+ lane` chooser's card is down — see [`View::lane_open`] the
    /// field.
    pub fn lane_open(&self) -> bool {
        self.lane_open
    }

    /// Put the card down, and answer whether it went down.
    ///
    /// Refused where there is nothing to point at, which is [`View::open_target`]'s
    /// rule: a card with no items offers nothing to pick, and
    /// [`crate::input::claim`]'s rule 2 would give it every press on the console
    /// until a second press shut it again.
    pub fn open_lane(&mut self) -> bool {
        if self.lane_open || self.lane_choices().items.is_empty() {
            return false;
        }
        self.lane_open = true;
        true
    }

    /// Take the card away, and answer whether there was one down —
    /// [`View::shut_target`]'s shape and its reason.
    pub fn shut_lane(&mut self) -> bool {
        let was = self.lane_open;
        self.lane_open = false;
        was
    }

    /// What the Master bay's `+ add` offers this frame — [`View::lane_choices`]'
    /// shape one bay along. The item that is drawn and the item a press lands on
    /// are one derivation.
    pub fn chain_choices(&self) -> AddChoices {
        AddChoices {
            items: self.chain_add.clone(),
            open: self.chain_add_open,
        }
    }

    /// Whether the `+ add` chooser's card is down.
    pub fn chain_add_open(&self) -> bool {
        self.chain_add_open
    }

    /// Put the card down, and answer whether it went down — [`View::open_lane`]'s
    /// rule: a card with nothing on it offers nothing to pick.
    pub fn open_chain_add(&mut self) -> bool {
        if self.chain_add_open || self.chain_add.is_empty() {
            return false;
        }
        self.chain_add_open = true;
        true
    }

    /// Take the card away, and answer whether there was one down.
    pub fn shut_chain_add(&mut self) -> bool {
        let was = self.chain_add_open;
        self.chain_add_open = false;
        was
    }

    /// What adding the carried Library row to the chain asks for, or the reason
    /// it asks for nothing.
    ///
    /// The chain holds `kind L5` procedures, so a row that is not one is refused
    /// and the refusal says what the chain holds
    /// ([P-0083](../../../../docs/principles/0083-a-refusal-carries-what-the-next-attempt-needs.md)).
    /// [`View::chain_add`] is the list a row is resolved against: it is the
    /// library's `kind L5` rows, by the name each is listed under.
    pub fn chain_landing(&self, row: &str) -> Result<Operation, &'static str> {
        self.chain_add
            .iter()
            .find(|choice| choice.words == row)
            .map(AddChoice::operation)
            .ok_or("the master chain holds kind L5 procedures, and this row is not one")
    }
}
