pub(crate) use std::collections::BTreeSet;
pub(crate) use std::fs;
pub(crate) use std::path::{Path, PathBuf};

pub(crate) use karakuri_operation::{BlendMode, Operation, Residency, WipeKind};

/// The specification, relative to the workspace root.
pub(crate) const PAGE: &str = "docs/manual/operations.html";

/// The source this crate's controls are read out of, relative to the same root.
/// Its own crate, read as text: there is no other way to ask *which operations
/// does this code construct* from inside a test binary.
pub(crate) const SRC: &str = "crates/karakuri-console/src";

/// What marks a row on the page — the marker `vocabulary.rs` and
/// `karakuri-environment/src/mcp.rs` both match, for the reason the first of
/// them gives: sections are `<h2>` and a heading somebody adds for looks is
/// neither.
pub(crate) const ROW: &str = r#"<div class="op-head">"#;

/// Identifies the section whose badges map to console gestures/Ops rather than
/// `karakuri_operation::Operation`, which are verified in `tests/vocabulary.rs`.
pub(crate) const ELSEWHERE: &str = "<h2>Arranging the console</h2>";

/// The badge text of a route that names nowhere. A `plan` badge is allowed to
/// be this — three of them are, and ADR-0213 says which — but a `has` badge
/// cannot: it would claim an operator reaches the operation and decline to say
/// from where.
pub(crate) const NOWHERE: &str = "&mdash;";

/// Operations emitted by console controls but not yet operator-reachable (ADR-0213).
pub(crate) const UNREACHABLE: [&str; 0] = [];

pub(crate) fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

pub(crate) fn page() -> String {
    let path = workspace().join(PAGE);
    fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "{} is the specification and could not be read: {e}",
            path.display()
        )
    })
}
