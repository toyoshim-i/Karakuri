//! Integration tests for multiple geometry sources in one Set.
//!
//! Verifies layout isolation, hash salt differentiation, and GPU simulation across multiple sources.

#[path = "common/mod.rs"]
mod engine_common;

#[path = "sources/common.rs"]
mod common;

#[path = "sources/refused.rs"]
mod refused;

#[path = "sources/gpu_tests.rs"]
mod gpu_tests;
