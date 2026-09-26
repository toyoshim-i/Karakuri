//! Hot-swap verification: ensures pipeline replacement occurs without frame drops.
//!
//! Decomposed into submodules under `tests/hot_swap/`.

#[path = "common/mod.rs"]
mod engine_common;

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
