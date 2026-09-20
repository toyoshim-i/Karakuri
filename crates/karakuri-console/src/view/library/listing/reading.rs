use egui::{Pos2, Rect};
use karakuri_operation::Operation;

use super::super::super::*;

// ---------------------------------------------------------------------------
// The Library reading block and published parameters
// ---------------------------------------------------------------------------

/// The word in the foot's button, which is the whole of the mock's own `load` —
/// the first of the three the readout `load &rarr; A` became (ADR-0305).
pub const LOAD_PILL: &str = "load";

/// The `→` between the word and the letter, drawn rather than typed —
/// [`CHEVRON_W`]'s reason and [`arrow_mark`]'s shape, which is what the two
/// scrub arrows in the Inspector already are.
pub const LOAD_ARROW: f32 = size::BASE * 0.5;

/// The word the foot's first capsule reads, which is the mock's own `params` —
/// the chip between the count and the `load` button.
pub const PARAMS_PILL: &str = "params";

/// The two words at the head of a reading: what the row under the cursor
/// declares, and how many controls that comes to.
pub const READING_HEAD: &str = "declares";

/// The word the capacity row is drawn under, which the mock draws in the same
/// range-and-default shape as a knob *"because that is how a procedure declares
/// it"*.
pub const READING_CAPACITY: &str = "capacity";

/// The word the emitted attributes are drawn under — *"what a renderer drawn
/// over it can consume"*.
pub const READING_EMITS: &str = "emits";

/// One published control of a Set, as a reading of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Published {
    /// The key the control is published under — `radius`, `exposure`.
    pub key: String,
    /// The declared range and the default, as the host spelled them: the mock's `0
    /// – 8 · 2`.
    pub range: String,
}

/// What the Set under the cursor holds and declares, opened under its row.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Reading {
    /// Which Set it is a reading of.
    pub id: String,
    /// One per published key, in the order the Set publishes them.
    pub knobs: Vec<Published>,
    /// How many elements the geometry declares it can carry.
    pub capacity: Option<String>,
    /// What the geometry emits, as the host joined them.
    pub emits: Option<String>,
    /// How many nodes the Set file names.
    pub nodes: usize,
    /// How many of those the store holds a card for.
    pub described: usize,
}

impl Reading {
    /// How many rows it is drawn in: the head, one per knob, the capacity and the
    /// emitted attributes where there are any, and the foot.
    pub fn rows(&self) -> usize {
        2 + self.knobs.len()
            + usize::from(self.capacity.is_some())
            + usize::from(self.emits.is_some())
    }

    /// The head's right-hand word: `6 knobs`.
    pub fn knobs_word(&self) -> String {
        format!(
            "{} knob{}",
            self.knobs.len(),
            match self.knobs.len() {
                1 => "",
                _ => "s",
            }
        )
    }

    /// The foot's left-hand word: `5 nodes`.
    pub fn nodes_word(&self) -> String {
        format!(
            "{} node{}",
            self.nodes,
            match self.nodes {
                1 => "",
                _ => "s",
            }
        )
    }

    /// The foot's right-hand word: the mock's `all described`, or how many nodes
    /// declared nothing this could read.
    pub fn cards_word(&self) -> String {
        match self.nodes.saturating_sub(self.described) {
            0 => "all described".to_owned(),
            missing => format!("{missing} without a card"),
        }
    }
}

/// A reading and the row it is open under, handed to [`library`] together.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Opened<'a> {
    /// Which row of the listing the reading belongs to — [`View::cursor_row`].
    pub at: usize,
    /// What that Set declares.
    pub reading: &'a Reading,
}

/// The box a reading is drawn in, inside [`LibraryBay::list`] and under the row
/// the cursor is on.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Block {
    /// The well itself: the rows' own box, without the margin around it.
    pub well: Rect,
    /// How many rows are in it — [`Reading::rows`], whether or not the list had
    /// room for them.
    pub rows: usize,
    /// The first row of the *listing* drawn under the block, which is the cursor's
    /// own row plus one.
    pub under: usize,
}

impl Block {
    /// The `index`th row of the reading, counting from the top of the well.
    pub fn row(&self, index: usize) -> Rect {
        Rect::from_min_size(
            Pos2::new(
                self.well.min.x,
                self.well.min.y + size::LIB_ROW_H * index as f32,
            ),
            egui::vec2(self.well.width(), size::LIB_ROW_H),
        )
    }
}

/// What a press on the `params` chip asks for.
#[derive(Debug, Clone, PartialEq)]
pub enum Read {
    /// Nothing is open under the cursor: read the Set that is there.
    Open(Operation),
    /// The reading under the cursor is open, and the press puts it away.
    Shut,
}
