//! Loading a `.kir` pair and putting it through the validation pipeline.
//!
//! Stages 1 through 4 live in `karakuri-ir` and stage 5 in `karakuri-codegen`;
//! all this does is read files, run them, and render any diagnostic against the
//! source it came from. Failure at any stage means no artifact, so there is no
//! partial success to report — the whole point of one severity.

use std::path::Path;

use karakuri_ir::typed::Checked;

pub fn load(path: &Path) -> Result<Checked, String> {
    let src = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    compile(&src).map_err(|report| format!("{}:\n{report}", path.display()))
}

/// The same five stages, over source already in hand rather than a path.
///
/// A Set file's procedures arrive as bytes out of the store, so they have no
/// path to name in a diagnostic; everything else about validating them is
/// identical, and it is the same function.
pub fn check(src: &str) -> Result<Checked, String> {
    compile(src)
}

fn compile(src: &str) -> Result<Checked, String> {
    let proc = karakuri_ir::parse(src).map_err(|e| render(&e, src))?;
    let checked = karakuri_ir::check::check(&proc).map_err(|e| render(&e, src))?;
    let cost = karakuri_ir::cost::estimate(&checked).map_err(|e| render(&e, src))?;
    eprintln!(
        "  {} — {} ops/element, {} ops/spawn, {} bytes/element",
        proc.name, cost.ops_per_element, cost.ops_per_spawn, cost.bytes_per_element
    );
    Ok(checked)
}

fn render(errors: &[karakuri_ir::IrError], src: &str) -> String {
    errors
        .iter()
        .map(|e| e.render(src))
        .collect::<Vec<_>>()
        .join("\n\n")
}
