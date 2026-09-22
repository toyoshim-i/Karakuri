//! The deck: several Sets resident, one to four composited.
//!
//! Decomposed into submodules under `tests/deck/`.

#[path = "deck/common.rs"]
pub mod common;

#[path = "deck/masks_and_transitions.rs"]
mod masks_and_transitions;

#[path = "deck/blending.rs"]
mod blending;

#[path = "deck/master.rs"]
mod master;

#[path = "deck/lifecycle_and_swaps.rs"]
mod lifecycle_and_swaps;

#[path = "deck/budget_and_stopped.rs"]
mod budget_and_stopped;

#[path = "deck/measurements_and_edge_cases.rs"]
mod measurements_and_edge_cases;
