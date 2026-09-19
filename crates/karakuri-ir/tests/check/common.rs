pub use karakuri_ir::ast::{Attr, BinOp, BlockKind, Kind, Output, SlotTy, Topology, Ty};
pub use karakuri_ir::check::check;
pub use karakuri_ir::parse::parse;
pub use karakuri_ir::typed::{Checked, Slot, TExprKind, TStmt, Target};

/// Parse then check, panicking with rendered diagnostics if either stage
/// unexpectedly fails. Used for fixtures this test expects to be valid.
pub fn check_ok(src: &str) -> Checked {
    let proc = parse(src).unwrap_or_else(|errs| {
        panic!(
            "expected source to parse:\n{}",
            errs.iter()
                .map(|e| e.render(src))
                .collect::<Vec<_>>()
                .join("\n")
        )
    });
    check(&proc).unwrap_or_else(|errs| {
        panic!(
            "expected source to check clean:\n{}",
            errs.iter()
                .map(|e| e.render(src))
                .collect::<Vec<_>>()
                .join("\n")
        )
    })
}

/// Parse then check, panicking if parsing fails (these fixtures are meant to
/// be syntactically valid and semantically wrong) and returning the check
/// errors otherwise.
pub fn check_err(src: &str) -> Vec<karakuri_ir::error::IrError> {
    let proc = parse(src).unwrap_or_else(|errs| {
        panic!(
            "expected source to parse (it should fail *checking*, not parsing):\n{}",
            errs.iter()
                .map(|e| e.render(src))
                .collect::<Vec<_>>()
                .join("\n")
        )
    });
    check(&proc).expect_err("expected the check pass to reject this source")
}

/// Every refusal a source draws, whichever stage made it.
pub fn refusals(src: &str) -> Vec<karakuri_ir::error::IrError> {
    match parse(src) {
        Err(errs) => errs,
        Ok(proc) => check(&proc).expect_err("expected this source to be rejected"),
    }
}
