//! The working copy an editing session runs from.
//!
//! # Three places, and only one of them is writable
//!
//! | | where | who writes it |
//! |---|---|---|
//! | app presets | `examples/` | nobody — they ship with the program |
//! | user presets | `<store>/sets/<id>.kbset` | `--save-set`, and nothing else |
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
//! # Material that has no file of its own
//!
//! A Set loaded with `--load-set` names its procedures by hash: the sources are
//! in the store and there is no `.kir` anywhere to copy. [`place`] writes them
//! here, which is what makes a saved Set editable at all — before there was a
//! scratch, `--mcp` with `--load-set` was refused because there was nothing for
//! a model to read or rewrite.
//!
//! That closes the loop the three places were separated for: a user preset is
//! materialised here, edited by a hand or a model, and saved back with
//! `--save-set`. Saving worked already; reading one back to edit it did not.
//!
//! # Sharing is preserved, deliberately
//!
//! Two slots naming one file share it — [`crate::watch`] documents that as
//! supported, and a write through MCP reports the other slots it reached. Copying
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
/// **Takes the paths rather than the slots**, because what a slot is has grown a
/// name beside each path and this module has no business knowing that. Every
/// path in every slot, in any order — the dedup is keyed by the source path, so
/// order decides only which of two identical sources keeps the plain basename.
pub fn materialise<'a>(
    store_root: &Path,
    paths: impl Iterator<Item = &'a mut PathBuf>,
) -> Result<PathBuf, String> {
    let dir = store_root.join(DIR);
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;

    // Keyed by the source path, so two slots that shared a file still do.
    let mut copied: HashMap<PathBuf, PathBuf> = HashMap::new();
    // Basenames already handed out, so two different sources called `field.kir`
    // do not become one file. A collision here would silently merge two slots'
    // material, which is the one failure this whole module exists to prevent.
    let mut taken: Vec<String> = Vec::new();

    {
        for path in paths {
            // Already the working copy — a Set loaded from the store, placed
            // here by `place` before this ran. Copying it onto itself would at
            // best be a no-op and at worst rename it out from under the deck
            // when its basename collided with something else's.
            if path.starts_with(&dir) {
                if let Some(name) = path.file_name().map(|n| n.to_string_lossy().to_string()) {
                    taken.push(name);
                }
                continue;
            }
            if let Some(existing) = copied.get(path.as_path()) {
                *path = existing.clone();
                continue;
            }
            let name = unique_name(path, &mut taken);
            let target = dir.join(&name);
            let source =
                std::fs::read(path.as_path()).map_err(|e| format!("{}: {e}", path.display()))?;
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

/// Write a procedure that has no file of its own into the scratch, and return
/// the path the deck should use.
///
/// `name` is a suggestion — the procedure's own name makes the scratch readable
/// — and is made safe as a path component here, because it arrives from a Set
/// file rather than from this program.
pub fn place(store_root: &Path, name: &str, source: &str) -> Result<PathBuf, String> {
    let dir = store_root.join(DIR);
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let path = dir.join(format!("{}.kir", sanitize(name)));
    std::fs::write(&path, source).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(path)
}

/// A name as a path component. The same rule the edit history uses, and here
/// for the same reason: the name comes out of a file somebody else wrote.
fn sanitize(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if cleaned.is_empty() {
        "procedure".to_string()
    } else {
        cleaned
    }
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
        let mut sets = [(l1.clone(), vec![l4.clone()])];

        let dir = materialise(
            &store,
            sets.iter_mut()
                .flat_map(|(l1, l4s)| std::iter::once(l1).chain(l4s.iter_mut())),
        )
        .expect("materialise");

        assert!(
            sets[0].0.starts_with(&dir),
            "L1 still points at {:?}",
            sets[0].0
        );
        assert!(
            sets[0].1[0].starts_with(&dir),
            "L4 still points at {:?}",
            sets[0].1[0]
        );
        assert_eq!(
            std::fs::read_to_string(&sets[0].0).expect("read"),
            "original l1"
        );

        // And writing through the deck's path leaves the preset alone, which is
        // the whole point rather than a consequence worth assuming.
        std::fs::write(&sets[0].0, "rewritten").expect("write");
        assert_eq!(std::fs::read_to_string(&l1).expect("read"), "original l1");
    }

    /// Two slots naming one file still name one file. [`crate::watch`]
    /// supports that and [`crate::mcp`] reports it; per-slot copies would have
    /// ended both quietly.
    #[test]
    fn two_slots_sharing_a_source_still_share_one_scratch_file() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let store = tmp.path().join("store");
        let l1 = write(tmp.path(), "field.kir", "l1");
        let a = write(tmp.path(), "a.kir", "a");
        let b = write(tmp.path(), "b.kir", "b");
        let mut sets = [(l1.clone(), vec![a]), (l1.clone(), vec![b])];

        materialise(
            &store,
            sets.iter_mut()
                .flat_map(|(l1, l4s)| std::iter::once(l1).chain(l4s.iter_mut())),
        )
        .expect("materialise");

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
        let mut sets = [(one, vec![l4.clone()]), (two, vec![l4])];

        materialise(
            &store,
            sets.iter_mut()
                .flat_map(|(l1, l4s)| std::iter::once(l1).chain(l4s.iter_mut())),
        )
        .expect("materialise");

        assert_ne!(
            sets[0].0, sets[1].0,
            "two different sources became one file"
        );
        assert_eq!(std::fs::read_to_string(&sets[0].0).expect("read"), "first");
        assert_eq!(std::fs::read_to_string(&sets[1].0).expect("read"), "second");
    }

    /// A Set loaded from the store becomes a file the deck can run from and an
    /// editor can open — the half of the three places that saving alone did not
    /// give.
    #[test]
    fn a_procedure_from_the_store_gets_a_readable_file() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let store = tmp.path().join("store");
        let path = place(&store, "beat_strands", "proc beat_strands {}").expect("place");

        assert!(path.starts_with(store.join(DIR)), "{}", path.display());
        assert_eq!(path.file_name().expect("name"), "beat_strands.kir");
        assert_eq!(
            std::fs::read_to_string(&path).expect("read"),
            "proc beat_strands {}"
        );
    }

    /// And a name out of a Set file cannot walk out of the scratch.
    #[test]
    fn a_placed_name_cannot_escape_the_scratch() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let store = tmp.path().join("store");
        let path = place(&store, "../../etc/passwd", "x").expect("place");

        assert!(
            path.starts_with(store.join(DIR)),
            "{} escaped",
            path.display()
        );
        assert!(!path.to_string_lossy().contains(".."), "{}", path.display());
    }

    /// A path already in the scratch is the working copy and is left alone —
    /// otherwise the pass that protects the originals would rewrite the one
    /// thing that has no original.
    #[test]
    fn a_path_already_in_the_scratch_is_not_copied_again() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let store = tmp.path().join("store");
        let placed = place(&store, "loaded", "from the store").expect("place");
        let l4 = write(tmp.path(), "draw.kir", "l4");
        let mut sets = [(placed.clone(), vec![l4])];

        materialise(
            &store,
            sets.iter_mut()
                .flat_map(|(l1, l4s)| std::iter::once(l1).chain(l4s.iter_mut())),
        )
        .expect("materialise");

        assert_eq!(sets[0].0, placed, "the placed procedure was moved");
        assert_eq!(
            std::fs::read_to_string(&placed).expect("read"),
            "from the store"
        );
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
        let mut sets = [(missing, vec![l4])];

        let err = materialise(
            &store,
            sets.iter_mut()
                .flat_map(|(l1, l4s)| std::iter::once(l1).chain(l4s.iter_mut())),
        )
        .expect_err("the source is not there");
        assert!(err.contains("nope.kir"), "{err}");
    }
}
