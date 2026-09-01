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
//! # A slot runs from its own copy, and the name carries the slot
//!
//! **One copy per slot, named `A0-drift_shell.kir`** — the deck letter, the
//! node's place in that slot, and the material's own name ([`node_name`]). The
//! same preset in four slots is four files, and that is the requirement rather
//! than the price of meeting it: a slot is the unit that gets replaced —
//! [`crate::watch`]'s *"that is what it means for each slot to own its own
//! `HotSwap`"* — so a slot whose material is also somebody else's is a slot
//! that is not a channel.
//!
//! **This replaces the opposite rule**, which stood here until 2026-09-01.
//! It is quoted rather than deleted because it was argued rather than assumed,
//! and somebody will re-propose it:
//!
//! > Two slots naming one file share it — `watch` documents that as supported,
//! > and a write through MCP reports the other slots it reached. Copying
//! > per-slot would quietly end that.
//!
//! Three things are wrong with it.
//!
//! 1. **It is circular.** The report it defends — *this file is also slot 1* —
//!    exists only to describe the sharing. End the sharing and that sentence is
//!    not made untrue, it is made **empty**, which is what it should say. What
//!    is lost is a warning about an accident, not something an operator asked
//!    for.
//! 2. **It contradicts the naming rule in the same directory.** [`place`] has
//!    written `A0-drift.kir` since ADR-0228, whose argument is that
//!    `<store>/scratch/<name>.kir` **overwrites** what is there, so two decks
//!    whose material shares a name *"would silently become one file — the
//!    second load moving the first deck on its watcher's next poll, with
//!    nothing to say why"*. A copy named after its source alone put that exact
//!    failure back into the one module written to stop it, and a run with
//!    `--load-set --watch` used both rules at once, in one directory.
//! 3. **It costs the thing the slots are for.** Four decks opened on one
//!    preset are four simulations to be driven apart; one shared file means
//!    the first edit moves all four and no one of them can be moved alone. In
//!    `crates/karakuri` it also meant one save rebuilt four slots, four
//!    candidates entered the Staging lane, and the three that are parked never
//!    reach a verdict — a lane that fills on the first save and stays full for
//!    the rest of the run.
//!
//! **What survives is the obligation to speak, not the shared file.** An
//! operator who gave one preset to four decks had an edit reach all four, and
//! will expect it to. So the surface that writes says what its write reached:
//! [`crate::mcp`] still scans the other slots for the path it wrote — a scan
//! that is normally empty now, and empty *because* of this rule rather than
//! because nobody shares — and both programs print the per-deck file list at
//! startup, so the operator opens the file belonging to the deck they mean.

use std::path::{Path, PathBuf};

/// The subdirectory of the store this lives in.
pub const DIR: &str = "scratch";

/// Copy every node of every slot into the scratch and rewrite the paths to
/// point at the copies. Returns the directory, for printing.
///
/// **Call this only when the run can be edited** — `--watch`, `--mcp`, or a
/// surface that is permanently both. A run with none of those writes no `.kir`
/// at all, so there is nothing to protect the originals from, and copying would
/// leave a directory behind for a render that is supposed to be a function of
/// its arguments.
///
/// Existing scratch files are overwritten and the rest of the directory is left
/// alone. Not cleared: an operator may have put something here, and deleting a
/// directory whose name we chose is a bad way to find that out.
///
/// **Takes the slots rather than a flat list of paths**, which is the change
/// this function exists to carry: the name of a copy is [`node_name`]'s, and
/// that name is the slot and the node's place in it. A flat list could not
/// spell one. What is *not* taken is what a slot is — the caller hands over
/// each slot's paths in node order and keeps the names beside them, because a
/// node name is the Set file's business and not this module's.
///
/// **No dedup, and that is the rule rather than an omission.** The same source
/// given to four slots becomes four files. See this module's header for why the
/// opposite rule was there and why it went; the short of it is that a slot is
/// the unit that gets replaced, so a slot whose file is also somebody else's
/// cannot be moved on its own.
pub fn materialise<'a, S, N>(store_root: &Path, slots: S) -> Result<PathBuf, String>
where
    S: IntoIterator<Item = N>,
    N: IntoIterator<Item = &'a mut PathBuf>,
{
    let dir = store_root.join(DIR);
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;

    for (slot, nodes) in slots.into_iter().enumerate() {
        for (at, path) in nodes.into_iter().enumerate() {
            // Already the working copy — a Set loaded from the store, placed
            // here by `place` before this ran, under a name that already
            // carries this slot. Copying it onto itself would at best be a
            // no-op and at worst rewrite it from itself mid-run.
            if path.starts_with(&dir) {
                continue;
            }
            // **Every file in this directory is a `<letter><at>-<name>.kir`**,
            // which is the rule `place` has always followed and is now the
            // whole directory's: one spelling, so an operator reading a
            // listing of it is reading one kind of thing. A source named
            // something else keeps its name and gains the extension its
            // content already had.
            let target = dir.join(format!("{}.kir", node_name(slot, at, &stem_of(path))));
            let source =
                std::fs::read(path.as_path()).map_err(|e| format!("{}: {e}", path.display()))?;
            std::fs::write(&target, &source).map_err(|e| format!("{}: {e}", target.display()))?;
            *path = target;
        }
    }
    Ok(dir)
}

/// **The name one node of one slot is filed under**, and the one rule: the deck
/// letter, the node's place in that slot, and the material's own name —
/// `A0-drift_shell`. [`place`] appends the `.kir`.
///
/// This is ADR-0228's spelling, made the whole directory's rather than the
/// library load's. That record's argument is the one that generalises: `place`
/// *"writes `<store>/scratch/<name>.kir` and overwrites what is there, so two
/// decks loading Sets whose procedures happen to share a name would silently
/// become one file — the second load moving the first deck on its watcher's
/// next poll, with nothing to say why."* Nothing in that sentence is about a
/// *load*: it is about two decks and one directory, which is every run.
///
/// **The material's own name is kept**, disambiguated by the prefix rather than
/// replaced by it, for the reason the counter it replaces gave: *"a scratch of
/// hashes is a scratch nobody opens in an editor."* An operator has to be able
/// to see which file is which deck **and** what is in it, and the two questions
/// are answered by the two halves of this name.
///
/// **No counter and no collision list.** `<letter><at>` is unique by
/// construction — a slot index and a node index — so two different sources can
/// no longer land on one file however they are named, which is what the
/// `unique_name` this replaces was scanning for.
pub fn node_name(slot: usize, at: usize, name: &str) -> String {
    format!("{}{at}-{}", deck_letter(slot), sanitize(name))
}

/// **The letter the deck is drawn with**, `A` for slot 0.
///
/// `karakuri_console::view::DECK_LETTERS` is `["A", "B", "C", "D"]` and is the
/// answer everywhere a surface *says* which deck; this crate cannot name it —
/// `karakuri-console` is not a dependency of this package and is not becoming
/// one for four strings, since that crate takes no device and this one is
/// nothing but doors. So the letters are counted from `A`, which agrees with
/// that list on every slot a `Deck` has: `karakuri_engine::deck::MAX_SLOTS` is
/// 4, and `the_deck_letters_are_the_ones_the_preview_cells_carry` asserts the
/// agreement rather than assuming it.
///
/// Past the twenty-sixth slot it is `S26`, which no deck can reach and which is
/// still a legal, unique path component — the one thing this function must not
/// do is hand back a name two slots share.
fn deck_letter(slot: usize) -> String {
    match u8::try_from(slot) {
        Ok(n) if n < 26 => char::from(b'A' + n).to_string(),
        _ => format!("S{slot}"),
    }
}

/// The material's own name, out of the path the operator gave, without the
/// extension — [`materialise`] puts `.kir` back on, so the copy is the one kind
/// of file this directory holds.
///
/// A path with no file name at all is `procedure`, which is [`sanitize`]'s
/// fallback said again at the one place that can reach it: `materialise` has
/// already read the file, so whatever it is, it is not a directory.
fn stem_of(source: &Path) -> String {
    source
        .file_stem()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "procedure".to_string())
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

    /// Every slot's paths, in node order, as [`materialise`] wants them.
    fn nodes(sets: &mut [(PathBuf, Vec<PathBuf>)]) -> impl Iterator<Item = Vec<&mut PathBuf>> {
        sets.iter_mut()
            .map(|(l1, l4s)| std::iter::once(l1).chain(l4s.iter_mut()).collect())
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

    /// **The requirement, and the one test this whole change is about.**
    ///
    /// The same preset in two slots, edited in one place, moves one deck. Under
    /// the rule this replaces the two slots shared one file, so the edit below
    /// moved both — silently, with nothing in the program able to say which
    /// deck the operator had meant. A slot is the unit that gets replaced;
    /// a slot that cannot be moved on its own is not one.
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

    /// **The name carries the deck and the node's place**, ADR-0228's
    /// `A0-drift.kir`, because `<store>/scratch/<name>.kir` overwrites what is
    /// there: a name that carried neither would put two decks on one file.
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

    /// **The letters are the ones the preview cells carry.** This crate cannot
    /// name `karakuri_console::view::DECK_LETTERS`, so the agreement is
    /// asserted here over every slot a `Deck` can have rather than assumed by
    /// two lists that would drift apart in silence.
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
    /// merge two slots' material into one file, which is worse than any name.
    /// The prefix is what answers now, and it answers by construction.
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
    /// editor can open — the half of the three places that saving alone did not
    /// give.
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
    /// otherwise the pass that protects the originals would rewrite the one
    /// thing that has no original.
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
}
