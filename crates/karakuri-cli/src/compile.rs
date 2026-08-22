//! Loading a `.kir` pair and putting it through the validation pipeline.
//!
//! Stages 1 through 4 live in `karakuri-ir` and stage 5 in `karakuri-codegen`;
//! all this does is read files, run them, and render any diagnostic against the
//! source it came from. Failure at any stage means no artifact, so there is no
//! partial success to report — the whole point of one severity.

use std::path::Path;

use karakuri_ir::typed::Checked;

/// **The compiled procedure and the text it was compiled from**, together.
///
/// The source is handed back rather than dropped, and that is the whole reason
/// this returns a pair. A `.kir` is read here exactly once per run, and what
/// the run goes on to say about that node — the address a live save writes, the
/// hash a `procedure` record names, the artifact a replay resolves — is a
/// function of *these* bytes. A second reader asking the path again is a second
/// answer to "what is this node running", and it is a different answer the
/// moment anything has rewritten the file in between: an editor, a model over
/// MCP, a formatter. See [`crate::Placed`], which is where these bytes are kept.
pub fn load(path: &Path) -> Result<(Checked, String), String> {
    let src = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let checked = compile(&src).map_err(|report| format!("{}:\n{report}", path.display()))?;
    Ok((checked, src))
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
    // **A field is reported on its own terms**, because neither figure beside
    // it means anything for one: it has no elements to have a per-element cost
    // and nothing to spawn. Printing those zeroes beside it was a line that read
    // as a measurement and was not one.
    //
    // **And no bytes/element from anything, because this is one procedure and
    // a procedure does not decide the layout it is allocated under.** The
    // struct the engine sizes a buffer by is built from everything that reached
    // the node, plus whatever a downstream consumer asked to be carried — both
    // properties of a Set, and there is no Set anywhere in this file. A built
    // Set is what reports the figure now; see the module doc on
    // `karakuri_ir::cost` for why nowhere earlier could.
    if proc.kind == karakuri_ir::Kind::Field {
        eprintln!(
            "  {} — {} ops/evaluation",
            proc.name, cost.ops_per_evaluation
        );
    } else {
        eprintln!(
            "  {} — {} ops/element, {} ops/spawn",
            proc.name, cost.ops_per_element, cost.ops_per_spawn
        );
    }
    Ok(checked)
}

fn render(errors: &[karakuri_ir::IrError], src: &str) -> String {
    errors
        .iter()
        .map(|e| e.render(src))
        .collect::<Vec<_>>()
        .join("\n\n")
}
