//! Verification of the master chain: an ordered sequence of L5 post-processing slots.
//!
//! Asserts parity between shipped L5 procedures (`feedback.kir`, `bloom.kir`, `rgb_shift.kir`)
//! and legacy hand-written shader passes on real GPU readbacks (ADR-0340).

#[path = "master/common.rs"]
mod common;

#[path = "master/pipeline.rs"]
mod pipeline;

#[path = "master/cuts.rs"]
mod cuts;

#[path = "master/execution.rs"]
mod execution;
