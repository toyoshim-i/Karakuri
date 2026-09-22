//! The Sequencer bay test suite.

mod common;

#[path = "sequencer/common.rs"]
pub mod sequencer_common;

#[path = "sequencer/lanes_and_cells.rs"]
mod lanes_and_cells;

#[path = "sequencer/banks_and_chooser.rs"]
mod banks_and_chooser;
