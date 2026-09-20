use egui::{Pos2, Rect};
use karakuri_operation::Operation;

use super::super::super::*;

// ---------------------------------------------------------------------------
// Library load target pulldown and row context menu
// ---------------------------------------------------------------------------

/// What a row menu's four load items are called, without the letter — `Load to
/// Slot A`.
pub const MENU_LOAD: &str = "Load to Slot";

/// The one item of a row menu that is not a load, under the separator.
pub const MENU_SAVE: &str = "Save as a kbset";

/// What the foot's load control is aimed at, and whether its list is down.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Target {
    /// Which deck a press on `load` lands on, as a slot number.
    pub deck: u8,
    /// How many decks the pulldown offers.
    pub decks: usize,
    /// Whether the list is down.
    pub open: bool,
}

impl Target {
    /// The letter the pulldown shows.
    pub fn letter(&self) -> &'static str {
        DECK_LETTERS[usize::from(self.deck)]
    }
}

/// What a press on the foot's load control asks for.
#[derive(Debug, Clone, PartialEq)]
pub enum Aim {
    /// Put the list down — a press on the pulldown with it shut.
    Open,
    /// Take it away.
    Shut,
    /// A deck named, and nothing asked for.
    Deck(u8),
    /// The load, named.
    Load(Operation),
    /// A press on `load` with no row under the cursor.
    NoSet,
}

/// What a press on a node group's `uses` line asks for.
#[derive(Debug, Clone, PartialEq)]
pub enum Wiring {
    /// A capsule pressed, named by the pane, the node and which of that node's
    /// inputs it is.
    Chip {
        pane: usize,
        node: usize,
        input: usize,
    },
    /// Take the card away.
    Shut,
    /// The rewiring, named.
    Pick(Operation),
}

/// What a press on a pane head's `▾` asks for.
#[derive(Debug, Clone, PartialEq)]
pub enum Pointing {
    /// The mark pressed, named by the pane whose head it is in.
    Mark(usize),
    /// Take the card away.
    Shut,
    /// The pointing, named.
    Pick(Operation),
}

/// The foot's load control, laid out: the `load` button, the `→` label, the
/// pulldown and the list under it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Load {
    /// The `load` button.
    pub button: Rect,
    /// Where [`LOAD_PILL`]'s word is painted.
    pub text: Rect,
    /// The `→` label's box.
    pub arrow: Rect,
    /// The pulldown's capsule.
    pub deck: Rect,
    /// Where the deck's letter is painted.
    pub letter: Rect,
    /// The `▾` after it.
    pub chevron: Rect,
    /// How many decks the list offers.
    pub rows: usize,
}

impl Load {
    /// Whether `p` is on the `load` button.
    pub fn hit_button(&self, p: karakuri_layout::Point) -> bool {
        self.button.contains(Pos2::new(p.x, p.y))
    }

    /// Whether `p` is on the pulldown's capsule.
    pub fn hit_deck(&self, p: karakuri_layout::Point) -> bool {
        self.deck.contains(Pos2::new(p.x, p.y))
    }

    /// The list under the pulldown, or `None` while it is shut.
    pub fn list(&self, viewport: Rect) -> Option<Rect> {
        if self.rows == 0 {
            return None;
        }
        let height = size::LIB_LIST_PAD * 2.0 + size::LIB_ROW_H * self.rows as f32;
        let width = self
            .deck
            .width()
            .max(size::LIB_ROW_H + (size::LIB_ROW_PAD_X + size::LIB_LIST_PAD) * 2.0);
        Some(held_inside(
            &viewport,
            self.deck.min.x,
            self.deck.min.y - size::PILL_GAP - height,
            width,
            height,
        ))
    }

    /// Where one row of the list is, from the top of `card`.
    pub fn row(&self, card: Rect, index: usize) -> Rect {
        assert!(index < self.rows, "deck {index} of a list of {}", self.rows);
        Rect::from_min_size(
            Pos2::new(
                card.min.x + size::LIB_LIST_PAD,
                card.min.y + size::LIB_LIST_PAD + size::LIB_ROW_H * index as f32,
            ),
            egui::vec2(card.width() - size::LIB_LIST_PAD * 2.0, size::LIB_ROW_H),
        )
    }

    /// Which deck `p` is on, or `None` for a point on no row.
    pub fn picked(&self, viewport: Rect, p: karakuri_layout::Point) -> Option<u8> {
        let card = self.list(viewport)?;
        let at = Pos2::new(p.x, p.y);
        (0..self.rows)
            .find(|index| self.row(card, *index).contains(at))
            .map(|index| index as u8)
    }
}

/// What a row's own menu is open on, and how many decks it offers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Menued {
    /// Which row of the listing the menu is down on, or `None` for no menu at all.
    pub row: Option<usize>,
    /// How many decks the loads offer.
    pub decks: usize,
    /// Whether this row can be sent.
    pub sends: bool,
}

/// One item of a row's menu, as what a press on it is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowItem {
    /// `Load to Slot A` … `Load to Slot D`, and the deck is the item.
    Load(u8),
    /// `Save as a kbset`, under the separator.
    Save,
}

/// What a press on a row's menu asks for.
#[derive(Debug, Clone, PartialEq)]
pub enum Picked {
    /// Put the menu down on this row.
    Open(usize),
    /// Take it away.
    Shut,
    /// A load, named.
    Load(Operation),
    /// The send, named.
    Send(Operation),
}

/// A row's menu, laid out.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RowMenu {
    /// The card itself, held inside the viewport.
    pub card: Rect,
    /// How many load items there are.
    pub loads: usize,
    /// The separator's band.
    pub rule: Option<Rect>,
    /// The `Save as a kbset` item, under the band.
    pub save: Option<Rect>,
}

impl RowMenu {
    /// Where one load item is, from the top of the card.
    pub fn load(&self, index: usize) -> Rect {
        assert!(
            index < self.loads,
            "load {index} of a menu of {}",
            self.loads
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

    /// Which item `p` is on.
    pub fn picked(&self, p: karakuri_layout::Point) -> Option<RowItem> {
        let at = Pos2::new(p.x, p.y);
        if self.save.is_some_and(|save| save.contains(at)) {
            return Some(RowItem::Save);
        }
        (0..self.loads)
            .find(|index| self.load(*index).contains(at))
            .map(|index| RowItem::Load(index as u8))
    }
}

/// The word one load item carries — `Load to Slot A`.
pub fn load_item(deck: u8) -> String {
    format!("{MENU_LOAD} {}", DECK_LETTERS[usize::from(deck)])
}
