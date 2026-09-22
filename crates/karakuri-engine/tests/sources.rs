//! Integration tests for multiple geometry sources in one Set.
//!
//! Verifies that multiple L1 geometry sources can share a Set without collision:
//! structured layouts run identically per source, while hash salts differ by default.
//! Tests both GPU-backed simulation/rendering and pure CPU-side validation refusals.

#[path = "common/mod.rs"]
mod engine_common;

#[path = "sources/common.rs"]
mod common;

#[path = "sources/refused.rs"]
mod refused;

#[path = "sources/gpu_tests.rs"]
mod gpu_tests;
