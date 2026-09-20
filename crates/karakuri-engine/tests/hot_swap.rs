//! The third clause of the V1 assumption: **can we hot-swap it without
//! dropping a frame?**
//!
//! Decomposed into submodules under `tests/hot_swap/`.

#[path = "hot_swap/common.rs"]
pub mod common;

#[path = "hot_swap/lifecycle_and_swaps.rs"]
mod lifecycle_and_swaps;

#[path = "hot_swap/budgets_and_estimates.rs"]
mod budgets_and_estimates;

#[path = "hot_swap/workers_and_performance.rs"]
mod workers_and_performance;

#[path = "hot_swap/rewind_and_macros.rs"]
mod rewind_and_macros;
