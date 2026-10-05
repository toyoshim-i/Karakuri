//! Platform-abstracted filesystem interface.
//!
//! On native platforms, re-exports standard `std::fs`.
//! On WebAssembly (`wasm32`), delegates to an in-memory virtual filesystem with bundled presets.

#[cfg(not(target_arch = "wasm32"))]
pub use std::fs::*;

#[cfg(target_arch = "wasm32")]
pub use crate::vfs::*;
