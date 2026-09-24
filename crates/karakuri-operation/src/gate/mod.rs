//! Operation gating, auditing, and authorization rules.
//!
//! Enforces policy invariants before operations reach the engine (ADR-0235, ADR-0236):
//! - Operations in protected classes are refused unless explicitly enabled via [`Open`].
//! - Authorization is type-level: [`audit`] is the sole constructor of [`Allowed`].
//! - Closed by default: [`Open::default`] starts with all protected classes closed.
//! - Exhaustive classification: [`standing`] maps every [`Operation`] variant to a [`Class`].

pub mod rules;
pub mod types;

#[cfg(test)]
mod tests;

pub use rules::*;
pub use types::*;
