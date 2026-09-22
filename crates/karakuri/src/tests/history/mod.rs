use super::*;

/// A temporary directory of this test's own, named after the test that wants it
/// — the shape every other CPU test in this file uses.
pub(crate) fn scratch_dir(what: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!(
        "karakuri-{what}-{}-{:?}",
        std::process::id(),
        std::thread::current().id(),
    ));
    // Whatever a previous run left behind, so the claims are about what
    // this run put there.
    let _ = std::fs::remove_dir_all(&root);
    root
}

mod presets;
mod versions;
