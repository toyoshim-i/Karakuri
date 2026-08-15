//! The working copy an editing session runs from.
//!
//! # Three places, and only one of them is writable
//!
//! | | where | who writes it |
//! |---|---|---|
//! | app presets | `examples/` | nobody — they ship with the program |
//! | user presets | `<store>/sets/<id>.set.ndjson` | `--save-set`, and nothing else |
//! | **scratch** | `<store>/scratch/` | `--watch`, `--mcp`, and the operator's editor |
//!
//! Before this existed there was one place and it was whatever path the
//! operator named, so `--mcp` handed a model write access to the repository's
//! own examples. It used them: three shipped presets were replaced in one
//! session, and what saved them was that they happened to be under version
//! control. **That is not a property of this program**, and a `.kir` an
//! operator wrote for a show would simply have been gone.
//!
//! So a run that can be edited copies its material here first and runs from the
//! copy. Nothing downstream needs to know: [`materialise`] rewrites the deck's
//! paths, and the compile, the watcher and the MCP surface all read the same
//! field.
//!
//! # Only when something can write
//!
//! An offscreen render never edits anything, and creating a directory as a side
//! effect of `--render` would make a pure function of its arguments into one
//! that leaves a mark. The caller gates on that; see [`materialise`]'s
//! documentation for the condition.
//!
//! # Sharing is preserved, deliberately
//!
//! Two slots naming one file share it — `watch.rs` documents that as supported,
//! and a write through MCP reports the other slots it reached. Copying
//! per-slot would quietly end that: the same file in two slots would become two
//! files, an edit would land in one, and the surface's report would be wrong
//! rather than merely different. So the copy is keyed by **source path**, and
//! two slots that shared a source still share a scratch file.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// The subdirectory of the store this lives in.
pub const DIR: &str = "scratch";

/// Copy every distinct source in `sets` into the scratch and rewrite `sets` to
/// point at the copies. Returns the directory, for printing.
///
/// **Call this only when the run can be edited** — `--watch` or `--mcp`. A run
/// with neither writes no `.kir` at all, so there is nothing to protect the
/// originals from, and copying would leave a directory behind for a render that
/// is supposed to be a function of its arguments.
///
/// Existing scratch files are overwritten and the rest of the directory is left
/// alone. Not cleared: an operator may have put something here, and deleting a
/// directory whose name we chose is a bad way to find that out.
pub fn materialise(
    store_root: &Path,
    sets: &mut [(PathBuf, PathBuf)],
) -> Result<PathBuf, String> {
    let dir = store_root.join(DIR);
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;

    // Keyed by the source path, so two slots that shared a file still do.
    let mut copied: HashMap<PathBuf, PathBuf> = HashMap::new();
    // Basenames already handed out, so two different sources called `field.kir`
    // do not become one file. A collision here would silently merge two slots'
    // material, which is the one failure this whole module exists to prevent.
    let mut taken: Vec<String> = Vec::new();

    for (l1, l4) in sets.iter_mut() {
        for path in [l1, l4] {
            if let Some(existing) = copied.get(path.as_path()) {
                *path = existing.clone();
                continue;
            }
            let name = unique_name(path, &mut taken);
            let target = dir.join(&name);
            let source = std::fs::read(path.as_path())
                .map_err(|e| format!("{}: {e}", path.display()))?;
            std::fs::write(&target, &source).map_err(|e| format!("{}: {e}", target.display()))?;
            copied.insert(path.clone(), target.clone());
            *path = target;
        }
    }
    Ok(dir)
}

/// A file name for `source` that no other source in this run has taken.
///
/// The basename, because the operator has to be able to find it — a scratch of
/// hashes is a scratch nobody opens in an editor. Disambiguated by a counter
/// only when two different sources really do share one.
fn unique_name(source: &Path, taken: &mut Vec<String>) -> String {
    let base = source
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "procedure.kir".to_string());
    if !taken.contains(&base) {
        taken.push(base.clone());
        return base;
    }
    let (stem, ext) = match base.rsplit_once('.') {
        Some((stem, ext)) => (stem.to_string(), format!(".{ext}")),
        None => (base.clone(), String::new()),
    };
    for n in 2.. {
        let candidate = format!("{stem}-{n}{ext}");
        if !taken.contains(&candidate) {
            taken.push(candidate.clone());
            return candidate;
        }
    }
    unreachable!("the loop returns")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &Path, name: &str, body: &str) -> PathBuf {
        let path = dir.join(name);
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("mkdir");
        std::fs::write(&path, body).expect("write");
        path
    }

    /// The claim the module exists for: after this, nothing the deck holds
    /// points at the file the operator named.
    #[test]
    fn the_deck_runs_from_the_copy_and_the_original_is_untouched() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let store = tmp.path().join("store");
        let l1 = write(&tmp.path().join("presets"), "field.kir", "original l1");
        let l4 = write(&tmp.path().join("presets"), "draw.kir", "original l4");
        let mut sets = vec![(l1.clone(), l4.clone())];

        let dir = materialise(&store, &mut sets).expect("materialise");

        assert!(sets[0].0.starts_with(&dir), "L1 still points at {:?}", sets[0].0);
        assert!(sets[0].1.starts_with(&dir), "L4 still points at {:?}", sets[0].1);
        assert_eq!(std::fs::read_to_string(&sets[0].0).expect("read"), "original l1");

        // And writing through the deck's path leaves the preset alone, which is
        // the whole point rather than a consequence worth assuming.
        std::fs::write(&sets[0].0, "rewritten").expect("write");
        assert_eq!(std::fs::read_to_string(&l1).expect("read"), "original l1");
    }

    /// Two slots naming one file still name one file. `watch.rs` supports that
    /// and MCP reports it; per-slot copies would have ended both quietly.
    #[test]
    fn two_slots_sharing_a_source_still_share_one_scratch_file() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let store = tmp.path().join("store");
        let l1 = write(tmp.path(), "field.kir", "l1");
        let a = write(tmp.path(), "a.kir", "a");
        let b = write(tmp.path(), "b.kir", "b");
        let mut sets = vec![(l1.clone(), a), (l1.clone(), b)];

        materialise(&store, &mut sets).expect("materialise");

        assert_eq!(sets[0].0, sets[1].0, "the shared L1 became two files");
        assert_ne!(sets[0].1, sets[1].1, "two different L4s became one file");
    }

    /// Two *different* sources with one basename must not collide — that would
    /// merge two slots' material into one file, which is worse than any name.
    #[test]
    fn different_sources_with_the_same_basename_get_different_scratch_files() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let store = tmp.path().join("store");
        let one = write(&tmp.path().join("one"), "field.kir", "first");
        let two = write(&tmp.path().join("two"), "field.kir", "second");
        let l4 = write(tmp.path(), "draw.kir", "l4");
        let mut sets = vec![(one, l4.clone()), (two, l4)];

        materialise(&store, &mut sets).expect("materialise");

        assert_ne!(sets[0].0, sets[1].0, "two different sources became one file");
        assert_eq!(std::fs::read_to_string(&sets[0].0).expect("read"), "first");
        assert_eq!(std::fs::read_to_string(&sets[1].0).expect("read"), "second");
    }

    /// A source that is not there is a diagnostic, not a scratch file holding
    /// nothing — the compile would otherwise fail on a path the operator never
    /// typed.
    #[test]
    fn a_missing_source_names_the_path_the_operator_gave() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let store = tmp.path().join("store");
        let missing = tmp.path().join("nope.kir");
        let l4 = write(tmp.path(), "draw.kir", "l4");
        let mut sets = vec![(missing, l4)];

        let err = materialise(&store, &mut sets).expect_err("the source is not there");
        assert!(err.contains("nope.kir"), "{err}");
    }
}
