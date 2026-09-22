//! The camera edge, from the producer to the picture.
//!
//! The camera is no longer six numbers the host packs into each renderer's
//! uniform: it is a node with a state buffer, a derivation pass, and a bind
//! group every L4 reads — see `karakuri_engine::node::Camera`. Everything
//! between the `Orbit` a caller assigns and the texels that come out is GPU
//! work, so the picture is the only place to check that it arrived.
//!
//! Three claims, and the third is the one that survived a defect injection
//! before this file existed:
//!
//! - **The camera reaches the frame**, so moving it moves the material.
//! - **It is re-derived every frame**, so a camera that turns keeps turning.
//! - **The aspect ratio reaches the projection.** It belongs to the canvas
//!   rather than to the camera, which is exactly why it is the piece that can
//!   go missing without any of the above noticing: it enters at the derivation,
//!   from a different buffer, written by a different call.
//!
//! `node::camera`'s own unit tests hold the derivation against the host's copy
//! of the same arithmetic. These hold the *plumbing* against the picture, which
//! is a different question: a perfect derivation nothing binds draws nothing.

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
