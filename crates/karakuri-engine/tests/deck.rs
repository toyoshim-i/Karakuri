//! The deck: several Sets resident, one to four composited.
//!
//! Decomposed into submodules under `tests/deck/`.

#[path = "deck/common.rs"]
pub mod common;

#[path = "deck/masks_and_transitions.rs"]
mod masks_and_transitions;

#[path = "deck/blending_and_master.rs"]
mod blending_and_master;

#[path = "deck/lifecycle_and_swaps.rs"]
mod lifecycle_and_swaps;

#[path = "deck/measurements_and_edge_cases.rs"]
mod measurements_and_edge_cases;
