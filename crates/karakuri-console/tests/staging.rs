//! Staging lane test suite.

mod common;

#[path = "staging/common.rs"]
pub mod staging_common;

#[path = "staging/geometry_and_rows.rs"]
mod geometry_and_rows;

#[path = "staging/controls_and_presses.rs"]
mod controls_and_presses;
