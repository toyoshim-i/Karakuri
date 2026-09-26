//! Camera integration tests verifying GPU uniform binding and frame derivation.

// Every test that takes a device lives under `mod gpu` — the prefix
// `cargo test -- --skip gpu::` filters on. The convention, and the test that
// enforces it, are in `tests/gpu_tests_are_under_mod_gpu.rs`.
#[path = "common/mod.rs"]
mod common;

#[path = "camera/fixtures.rs"]
mod fixtures;

#[path = "camera/gpu.rs"]
mod gpu;

#[path = "camera/refused.rs"]
mod refused;
