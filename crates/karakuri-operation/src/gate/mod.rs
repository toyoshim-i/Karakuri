//! Operation gating, auditing, and authorization rules.
//! Enforces policy invariants and type-level authorization before operations reach the engine.

pub mod rules;
pub mod types;

#[cfg(test)]
mod tests;

pub use rules::*;
pub use types::*;
