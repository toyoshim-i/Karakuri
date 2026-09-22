//! Verification of the budget governor and residency decision procedures.
//!
//! Tests the pure decision procedure (budgeting, admission, parking, and estimation basis)
//! alongside live deck integration and park recovery across multiple passes.

#[path = "common/mod.rs"]
mod engine_common;

#[path = "governor/common.rs"]
mod common;
#[path = "governor/deck.rs"]
mod deck;
#[path = "governor/pure.rs"]
mod pure;
