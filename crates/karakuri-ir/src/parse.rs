//! `.kir` source to [`Proc`].
//!
//! Syntax only. Undefined names, type errors, and contract violations are not
//! this pass's business — it should accept anything shaped like the grammar and
//! leave meaning to the check pass, so that a single malformed expression does
//! not mask every later diagnostic.

use crate::ast::Proc;
use crate::error::IrResult;

/// Parse one `.kir` file.
///
/// Returns every syntax error found, not just the first: a generated procedure
/// with four mistakes should produce four diagnostics so a repair prompt can fix
/// them in one pass.
pub fn parse(_src: &str) -> IrResult<Proc> {
    todo!("R1: parser")
}
