//! Verification of manual key bindings (`docs/manual/operations.html`) against instrument bindings.
//!
//! Validates reachability between manual table badges and window event key dispatch (ADR-0213, ADR-0214, ADR-0259, P-0087).

pub(super) mod data;
pub(super) mod scanner;
#[cfg(test)]
mod tests;

pub(crate) use data::*;
pub(crate) use scanner::*;
