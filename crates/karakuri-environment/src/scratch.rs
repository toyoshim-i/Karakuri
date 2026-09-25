//! Working copy management for live editable sessions.
//! Materializes editable shaders into `<store>/scratch/` per slot, ensuring each slot
//! owns an isolated copy of its files (`<letter><node_index>-<name>.kir`) (P-0096, ADR-0228).
use std::path::{Path, PathBuf};

/// The subdirectory of the store this lives in.
pub const DIR: &str = "scratch";

/// Copies each slot's node files into `<store>/scratch/` and updates paths to reference copies.
pub fn materialise<'a, S, N>(store_root: &Path, slots: S) -> Result<PathBuf, String>
where
    S: IntoIterator<Item = N>,
    N: IntoIterator<Item = &'a mut PathBuf>,
{
    let dir = store_root.join(DIR);
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;

    for (slot, nodes) in slots.into_iter().enumerate() {
        for (at, path) in nodes.into_iter().enumerate() {
            if path.starts_with(&dir) {
                continue;
            }
            let target = dir.join(format!("{}.kir", node_name(slot, at, &stem_of(path))));
            let source =
                std::fs::read(path.as_path()).map_err(|e| format!("{}: {e}", path.display()))?;
            write_atomic(&target, &source)?;
            *path = target;
        }
    }
    Ok(dir)
}

/// Formats a scratch file stem for a node in a given slot: `<deck_letter><at>-<name>`.
///
/// Uniquely scopes files per deck slot and node index to prevent collisions (ADR-0228).
pub fn node_name(slot: usize, at: usize, name: &str) -> String {
    format!("{}{at}-{}", deck_letter(slot), sanitize(name))
}

/// The letter the deck is drawn with, `A` for slot 0.
///
/// Returns a deck letter identifier for a given slot index (A..Z or S<slot>).
fn deck_letter(slot: usize) -> String {
    match u8::try_from(slot) {
        Ok(n) if n < 26 => char::from(b'A' + n).to_string(),
        _ => format!("S{slot}"),
    }
}

/// Extracts file stem or defaults to "procedure" if unavailable.
fn stem_of(source: &Path) -> String {
    source
        .file_stem()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "procedure".to_string())
}

/// Writes procedure source into the scratch directory and returns its target path.
pub fn place(store_root: &Path, name: &str, source: &str) -> Result<PathBuf, String> {
    let dir = store_root.join(DIR);
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let path = dir.join(format!("{}.kir", sanitize(name)));
    write_atomic(&path, source.as_bytes())?;
    Ok(path)
}

/// Writes `bytes` to `path` atomically via a temporary file rename.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    let mut tmp_name = path.file_name().unwrap_or_default().to_os_string();
    tmp_name.push(".tmp");
    let tmp_path = path.with_file_name(tmp_name);
    std::fs::write(&tmp_path, bytes).map_err(|e| format!("{}: {e}", tmp_path.display()))?;
    std::fs::rename(&tmp_path, path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(())
}

/// Sanitizes `name` for safe use as a file path component.
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

    /// Every slot's paths, in node order, as [`materialise`] wants them.
    fn nodes(sets: &mut [(PathBuf, Vec<PathBuf>)]) -> impl Iterator<Item = Vec<&mut PathBuf>> {
        sets.iter_mut()
            .map(|(l1, l4s)| std::iter::once(l1).chain(l4s.iter_mut()).collect())
    }

    /// The claim the module exists for: after this, nothing the deck holds points
    /// at the file the operator named.
    #[test]
    fn the_deck_runs_from_the_copy_and_the_original_is_untouched() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let store = tmp.path().join("store");
        let l1 = write(&tmp.path().join("presets"), "field.kir", "original l1");
        let l4 = write(&tmp.path().join("presets"), "draw.kir", "original l4");
        let mut sets = [(l1.clone(), vec![l4.clone()])];

        let dir = materialise(&store, nodes(&mut sets)).expect("materialise");

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

    /// Verifies that the same preset in two slots materializes as distinct files so editing one slot does not affect the other.
    #[test]
    fn the_same_preset_in_two_slots_is_two_files_and_an_edit_moves_one_deck() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let store = tmp.path().join("store");
        let l1 = write(
            &tmp.path().join("presets"),
            "drift_shell.kir",
            "the geometry",
        );
        let l4 = write(
            &tmp.path().join("presets"),
            "soft_points.kir",
            "the renderer",
        );
        // One pair, in both slots — which is exactly what `crates/karakuri`
        // opens on, in four.
        let mut sets = [
            (l1.clone(), vec![l4.clone()]),
            (l1.clone(), vec![l4.clone()]),
        ];

        materialise(&store, nodes(&mut sets)).expect("materialise");

        assert_ne!(
            sets[0].0, sets[1].0,
            "the two slots share one L1, so an edit cannot reach one of them"
        );
        assert_ne!(
            sets[0].1[0], sets[1].1[0],
            "the two slots share one L4, so an edit cannot reach one of them"
        );

        // The edit the operator makes, through deck A's own file.
        std::fs::write(&sets[0].0, "deck A only").expect("write");

        assert_eq!(
            std::fs::read_to_string(&sets[0].0).expect("read"),
            "deck A only",
            "the edit did not land on the deck it was made on"
        );
        assert_eq!(
            std::fs::read_to_string(&sets[1].0).expect("read"),
            "the geometry",
            "the edit moved deck B as well, which is the failure this rule exists to stop"
        );
        assert_eq!(
            std::fs::read_to_string(&l1).expect("read"),
            "the geometry",
            "the edit reached the preset the operator named"
        );
    }

    /// The name carries the deck and the node's place, ADR-0228's `A0-drift.kir`,
    /// because `<store>/scratch/<name>.kir` overwrites what is there: a name that
    /// carried neither would put two decks on one file.
    #[test]
    fn a_scratch_name_carries_the_deck_and_the_nodes_place() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let store = tmp.path().join("store");
        let l1 = write(tmp.path(), "drift_shell.kir", "geometry");
        let l4 = write(tmp.path(), "soft_points.kir", "renderer");
        let mut sets = [
            (l1.clone(), vec![l4.clone()]),
            (l1.clone(), vec![l4.clone()]),
        ];

        materialise(&store, nodes(&mut sets)).expect("materialise");

        let named = |path: &PathBuf| {
            path.file_name()
                .and_then(|n| n.to_str())
                .expect("a file name")
                .to_owned()
        };
        assert_eq!(named(&sets[0].0), "A0-drift_shell.kir");
        assert_eq!(named(&sets[0].1[0]), "A1-soft_points.kir");
        assert_eq!(named(&sets[1].0), "B0-drift_shell.kir");
        assert_eq!(named(&sets[1].1[0]), "B1-soft_points.kir");
    }

    /// The letters are the ones the preview cells carry. This crate cannot name
    /// `karakuri_console::view::DECK_LETTERS`, so the agreement is asserted here
    /// over every slot a `Deck` can have rather than assumed by two lists that
    /// would drift apart in silence.
    #[test]
    fn the_deck_letters_are_the_ones_the_preview_cells_carry() {
        let drawn = ["A", "B", "C", "D"];
        assert_eq!(
            drawn.len(),
            karakuri_engine::deck::MAX_SLOTS,
            "a deck holds {} slots and the console draws {} letters",
            karakuri_engine::deck::MAX_SLOTS,
            drawn.len()
        );
        for (slot, letter) in drawn.iter().enumerate() {
            assert_eq!(&deck_letter(slot), letter, "slot {slot}");
        }
    }

    /// Two *different* sources with one basename must not collide — that would
    /// merge two slots' material into one file, which is worse than any name. The
    /// prefix is what answers now, and it answers by construction.
    #[test]
    fn different_sources_with_the_same_basename_get_different_scratch_files() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let store = tmp.path().join("store");
        let one = write(&tmp.path().join("one"), "field.kir", "first");
        let two = write(&tmp.path().join("two"), "field.kir", "second");
        let l4 = write(tmp.path(), "draw.kir", "l4");
        let mut sets = [(one, vec![l4.clone()]), (two, vec![l4])];

        materialise(&store, nodes(&mut sets)).expect("materialise");

        assert_ne!(
            sets[0].0, sets[1].0,
            "two different sources became one file"
        );
        assert_eq!(std::fs::read_to_string(&sets[0].0).expect("read"), "first");
        assert_eq!(std::fs::read_to_string(&sets[1].0).expect("read"), "second");
    }

    /// A Set loaded from the store becomes a file the deck can run from and an
    /// editor can open — the half of the loop that saving alone did not give.
    #[test]
    fn a_procedure_from_the_store_gets_a_readable_file() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let store = tmp.path().join("store");
        let path = place(
            &store,
            &node_name(0, 0, "beat_strands"),
            "proc beat_strands {}",
        )
        .expect("place");

        assert!(path.starts_with(store.join(DIR)), "{}", path.display());
        assert_eq!(path.file_name().expect("name"), "A0-beat_strands.kir");
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
        let path = place(&store, &node_name(0, 0, "../../etc/passwd"), "x").expect("place");

        assert!(
            path.starts_with(store.join(DIR)),
            "{} escaped",
            path.display()
        );
        assert!(!path.to_string_lossy().contains(".."), "{}", path.display());
    }

    /// A path already in the scratch is the working copy and is left alone —
    /// otherwise the pass that protects the originals would rewrite the one thing
    /// that has no original.
    #[test]
    fn a_path_already_in_the_scratch_is_not_copied_again() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let store = tmp.path().join("store");
        let placed = place(&store, &node_name(0, 0, "loaded"), "from the store").expect("place");
        let l4 = write(tmp.path(), "draw.kir", "l4");
        let mut sets = [(placed.clone(), vec![l4])];

        materialise(&store, nodes(&mut sets)).expect("materialise");

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

        let err = materialise(&store, nodes(&mut sets)).expect_err("the source is not there");
        assert!(err.contains("nope.kir"), "{err}");
    }

    #[test]
    fn write_atomic_replaces_target_and_leaves_no_tmp() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let target = tmp.path().join("target.kir");
        let tmp_file = tmp.path().join("target.kir.tmp");

        write_atomic(&target, b"initial").expect("first write");
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "initial");
        assert!(!tmp_file.exists());

        write_atomic(&target, b"updated").expect("second write");
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "updated");
        assert!(!tmp_file.exists());
    }
}
