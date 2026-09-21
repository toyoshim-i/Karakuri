//! The `.kir` abstract syntax tree.
//!
//! The parser produces this verbatim from source. Names are not resolved, types
//! are not inferred, and nothing beyond syntax is checked: an `Assign` target
//! is a bare string here, and a `Call` covers builtins and type constructors
//! alike. Resolution and typing happen in the check pass, which consumes this.
//!
//! Specification: `docs/ir-spec.md`.

pub mod attr;
pub mod decl;
pub mod expr;
pub mod stmt;
pub mod types;

#[cfg(test)]
mod tests;

pub use attr::*;
pub use decl::*;
pub use expr::*;
pub use stmt::*;
pub use types::*;
