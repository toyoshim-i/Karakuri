use karakuri_operation::{LaneTarget, Operation};

use super::*;
use crate::view::master::{AddChoice, AddChoices};
use crate::view::sequencer::{lane_label, Choices, LaneChoice, FADER_ITEM};

// ---------------------------------------------------------------------------
// View popup choices and lane/chain controls
// ---------------------------------------------------------------------------

impl View {
    /// Returns lane chooser options this frame (faders and target deck parameters),
    /// shared between layout and hit-testing (see [`Choices`]).
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
                        // Target lane label formatted with deck and parameter name.
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

    /// Returns whether the `+ lane` chooser card is currently open.
    pub fn lane_open(&self) -> bool {
        self.lane_open
    }

    /// Opens the lane chooser card if closed and items are available.
    /// Returns `true` if opened, adhering to Rule 2 modal claim semantics.
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

    /// Resolves a library row to a chain operation, or returns a refusal reason
    /// if not a `kind L5` procedure ([P-0083](../../../../docs/principles/0083-a-refusal-carries-what-the-next-attempt-needs.md)).
    pub fn chain_landing(&self, row: &str) -> Result<Operation, &'static str> {
        self.chain_add
            .iter()
            .find(|choice| choice.words == row)
            .map(AddChoice::operation)
            .ok_or("the master chain holds kind L5 procedures, and this row is not one")
    }
}
