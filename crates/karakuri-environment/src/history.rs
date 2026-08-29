//! Every version that compiled, kept where a person can find it.
//!
//! # What this is for
//!
//! An edit replaces a procedure with no backup, whoever made it. A hand at an
//! editor and a model over MCP reach the same file through the same path, and
//! the version that was there is gone. This keeps it.
//!
//! The gate is **compiling**, not landing. A build that compiled and was then
//! rolled back for costing too much is exactly the kind of version worth going
//! back to — it was a real attempt and something about it was right — and it is
//! the one the session stream does *not* have, because `Record::Procedure` only
//! names what reached the screen.
//!
//! # Undo, without an undo tool
//!
//! A chain of snapshots per slot and layer is what a surface walks to offer
//! undo, and what an operator saves from once they find the one they liked.
//! Neither of those exists yet — nothing walks the chain, and saving to a user
//! preset is `--save-set`'s neighbourhood — but the snapshots have to be taken
//! while the editing is happening or there is nothing to walk later.
//!
//! # Why a date directory, and why local time
//!
//! ```text
//! <store>/history/2026/08/16/143052-271_slot0_L4_beat_strokes.kir
//! ```
//!
//! **There is deliberately no retention policy and no cleanup command.** A day
//! per directory means `rm -rf history/2026/07` is the cleanup, which is a
//! feature nobody has to write, learn, or trust.
//!
//! The date is **local**, and the reason is narrower than it first looks. It is
//! *not* that local time avoids splitting a night's work across two
//! directories: an event that runs past midnight splits either way, and if
//! anything it splits more often in local time, because that is when people
//! actually work. The reason is that the directory has to be named the day the
//! operator would call it — a person looking for last night's edits opens the
//! directory with last night's date on it, and a UTC name would be the wrong
//! one for half the world and half the day.
//!
//! # The version a run starts with is snapshotted before anything is edited
//!
//! Otherwise the first edit records only its *result*, and the version being
//! replaced — the one an undo goes back to — was never written down. So the
//! chain is seeded from the scratch at launch, which is what makes the first
//! edit undoable rather than the second.
//!
//! That seeding and the watcher's snapshots share **one** [`Snapshots`] for the
//! run, because they share the dedup: seeded separately, the first rebuild
//! would write the untouched procedure a second time.
//!
//! # Unchanged sources are not snapshotted
//!
//! A rebuild recompiles both procedures whichever one was saved, so writing
//! both every time would fill the directory with duplicates of the file nobody
//! touched — and make the chain for that layer a row of identical entries with
//! nothing to choose between. Each layer remembers what it last wrote.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// The subdirectory of the store this lives in.
pub const DIR: &str = "history";

/// **The clock a name is taken off**: local time to the millisecond.
///
/// Milliseconds, because two writes inside one second is an ordinary thing for
/// an editor that formats on write — and a name that collided would overwrite
/// the thing it was supposed to be preserving.
const TIME: &str = "%H%M%S-%3f";

/// A name for something an operator will look for by **when they made it**.
///
/// This module's convention, borrowed rather than reinvented, and borrowed for
/// its stated reason: a live save has no way to be given a name — a key press
/// cannot type one — so the only thing it can be filed under is the moment it
/// happened, and an operator goes looking for the time they pressed the key.
/// Local for the reason the date directory is local: the answer has to be the
/// one the person would say out loud, and a UTC name is the wrong one for half
/// the world and half the day.
///
/// The date is spelled `20260816` rather than `2026/08/16` because this is a
/// Set id and a Set id is one path component; the *time* half is [`TIME`], the
/// same string a snapshot is named with, so the two cannot drift.
pub fn stamped_id() -> String {
    let now = chrono::Local::now();
    let stamp = format!("{}-{}", now.format("%Y%m%d"), now.format(TIME));
    static ISSUED: std::sync::OnceLock<std::sync::Mutex<std::collections::HashSet<String>>> =
        std::sync::OnceLock::new();
    let issued = ISSUED.get_or_init(Default::default);
    match issued.lock() {
        Ok(mut issued) => unused(&mut issued, stamp),
        // A poisoned lock means a caller panicked holding it. The stamp is
        // still a name, and refusing to save over it would be this bookkeeping
        // deciding whether an operator keeps their work.
        Err(_) => stamp,
    }
}

/// `stamp`, or `stamp-1`, `stamp-2` — **the first spelling nothing in this run
/// has been given yet.**
///
/// [`TIME`] resolves to a millisecond because two writes inside one second is
/// ordinary, and [`Snapshots::record`] gets away with that alone: it dedups on
/// content, and its names carry the slot, layer and node index, so the writers
/// that could collide are already spelled apart. A Set id carries none of that.
/// Two saves of the same run in the same millisecond produced one id, and the
/// second Set file overwrote the first — the operator was told both had been
/// kept. Out of reach for a key press, and not out of reach for the MCP control
/// this is about to grow.
///
/// **A set of what this run has issued, rather than a look in the store.**
/// Checking the store is the stronger answer and it is a different function:
/// this one has no store to check, `Live::save_set` names the id on the render
/// thread and the store is only opened on the save thread, and a check there
/// would still race the write. What is left over is narrow and worth stating:
/// an id already on disk from an *earlier* run is not detected here, and two
/// runs saving in the same millisecond can still collide. Both need the store,
/// and neither is the case that arrived with a control that can be called twice
/// in a frame.
fn unused(issued: &mut std::collections::HashSet<String>, stamp: String) -> String {
    if issued.insert(stamp.clone()) {
        return stamp;
    }
    let mut nth = 1u32;
    loop {
        let candidate = format!("{stamp}-{nth}");
        if issued.insert(candidate.clone()) {
            return candidate;
        }
        nth += 1;
    }
}

/// Takes a snapshot per **node** — slot, layer, and which node of that layer —
/// and remembers what it last took, so an unchanged procedure is not written
/// again.
///
/// **Per node rather than per layer**, because a slot draws with a list of
/// renderers. Keyed by layer alone, a stack wrote every renderer under `L4` and
/// each one's `last` overwrote the previous — so a two-renderer slot recorded
/// one snapshot per save, alternating between two procedures that had not
/// changed, and the chain a surface walks became unusable exactly where there
/// was most to walk back through.
pub struct Snapshots {
    root: PathBuf,
    last: HashMap<(usize, &'static str, usize), Vec<u8>>,
}

/// One history for the run, shared between the launch-time seeding and every
/// slot's watcher — see the module doc on why they cannot each have their own.
pub type Shared = std::sync::Arc<std::sync::Mutex<Snapshots>>;

impl Snapshots {
    pub fn new(store_root: &Path) -> Snapshots {
        Snapshots {
            root: store_root.join(DIR),
            last: HashMap::new(),
        }
    }

    pub fn shared(store_root: &Path) -> Shared {
        std::sync::Arc::new(std::sync::Mutex::new(Snapshots::new(store_root)))
    }

    /// Write `source` as this slot and layer's newest snapshot, unless it is
    /// what was written last.
    ///
    /// Returns the path written, or `None` when the source was unchanged.
    /// **Failures are the caller's to report and never to act on**: a snapshot
    /// that could not be written must not stop a build that compiled, because
    /// the operator asked for a picture and this is bookkeeping.
    pub fn record(
        &mut self,
        slot: usize,
        layer: &'static str,
        index: usize,
        proc_name: &str,
        source: &[u8],
    ) -> Result<Option<PathBuf>, String> {
        if self
            .last
            .get(&(slot, layer, index))
            .is_some_and(|s| s == source)
        {
            return Ok(None);
        }
        let now = chrono::Local::now();
        let dir = self.root.join(now.format("%Y/%m/%d").to_string());
        std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;

        let stamp = now.format(TIME).to_string();
        // The index is in the name only when it is not the first, so every
        // name a one-renderer run has ever written is the name it still writes.
        // A file is read by a person looking for what they changed, and a `_0`
        // on every L4 of every ordinary run is noise in the way of that.
        let at = if index == 0 {
            String::new()
        } else {
            format!("{index}")
        };
        let name = format!("{stamp}_slot{slot}_{layer}{at}_{}.kir", sanitize(proc_name));
        let path = dir.join(name);
        std::fs::write(&path, source).map_err(|e| format!("{}: {e}", path.display()))?;
        self.last.insert((slot, layer, index), source.to_vec());
        Ok(Some(path))
    }
}

/// A procedure name as a path component.
///
/// The name comes from a checked `.kir`, so it is already an identifier — but
/// it reaches this function from a file the operator or a model wrote, and a
/// name is not the place to find out that the parser and this agree about what
/// an identifier is.
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
        "unnamed".to_string()
    } else {
        cleaned
    }
}

/// Seed the chains from what a run is about to play, so the first edit has
/// something to be undone back to.
///
/// Reported and never fatal, on the same terms as every other snapshot: a
/// history that could not be written must not stop a run from starting.
pub fn seed<'a>(shared: &Shared, sets: impl Iterator<Item = (usize, Vec<&'a Path>)>) {
    let Ok(mut snapshots) = shared.lock() else {
        return;
    };
    for (slot, paths) in sets {
        // Every file, each under its own layer and its own index within that
        // layer — the whole stack, because the history is a place an operator
        // walks back through and a node missing from it cannot be walked back
        // to.
        //
        // **The layer is read off the file, not off the position.** Everything
        // after the first path used to be filed as `L4`, which was right while
        // a slot was a pair and became wrong without a word when `--set`
        // learned to spell a chain: an L2's starting version landed as
        // `..._slot0_L4_swirl_warp.kir`, under a name the watcher does not use
        // for its later versions — so the chain an operator walks back through
        // began at the second edit.
        let mut counts: std::collections::HashMap<&'static str, usize> =
            std::collections::HashMap::new();
        for (positional, path) in paths.into_iter().enumerate() {
            let Ok(source) = std::fs::read(path) else {
                // Unreadable here means the compile is about to fail and say so
                // against the path the operator gave. Not this module's to
                // report twice.
                continue;
            };
            // The same text scan `declared_name` is and for the same reason:
            // this runs before anything is compiled. A file that declares no
            // `kind` will not compile either, so the position it was given in
            // is as good an answer as any.
            let layer = declared_kind(&source).unwrap_or(if positional == 0 { "L1" } else { "L4" });
            let index = counts.entry(layer).or_insert(0);
            let (layer, index) = (layer, *index);
            *counts.get_mut(layer).expect("just inserted") += 1;
            // The name a `.kir` declares, before it has been parsed — the
            // history is written for a person reading file names, and waiting
            // for the check pass would mean seeding after the first compile,
            // which is after the first edit could already have happened.
            let name = declared_name(&source).unwrap_or_else(|| "start".to_string());
            if let Err(e) = snapshots.record(slot, layer, index, &name, &source) {
                eprintln!("slot {slot}: the starting {layer} is not in the edit history: {e}");
            }
        }
    }
}

/// The word after `kind`, scanned out of the source text, as the history spells
/// a layer.
///
/// Deliberately not a parse, for the reason [`declared_name`] gives: this runs
/// before anything is compiled, and a file that does not compile still needs a
/// name to be found again under.
///
/// **Shared with `karakuri-cli`'s `mcp::Slots`**, which resolves a
/// `(slot, layer, index)` address by the same scan. Two readers of a `kind`
/// line would be two rules for what layer a file is on, and the layer a
/// snapshot is filed under has to be the layer an agent addresses it by or the
/// surface would be editing one node and undoing another. **`pub` rather than
/// `pub(crate)` for exactly that reader**, which is now one crate over.
pub fn declared_kind(source: &[u8]) -> Option<&'static str> {
    let text = std::str::from_utf8(source).ok()?;
    for line in text.lines() {
        let Some(rest) = line.trim_start().strip_prefix("kind") else {
            continue;
        };
        // `kind L1` and not `kindly`, which is the whole of what the space
        // buys — and the first token after it, so a trailing comment is not
        // part of the answer.
        if !rest.starts_with(char::is_whitespace) {
            continue;
        }
        return match rest.trim_start().split(char::is_whitespace).next()? {
            "L1" => Some("L1"),
            "L2" => Some("L2"),
            "L3" => Some("L3"),
            "L4" => Some("L4"),
            "Field" => Some("Field"),
            _ => None,
        };
    }
    None
}

/// The identifier after `proc`, scanned out of the source text.
///
/// Deliberately not a parse. This runs before anything is compiled, it feeds a
/// file name and nothing else, and a source that turns out not to compile
/// should still leave a snapshot named after what it called itself.
fn declared_name(source: &[u8]) -> Option<String> {
    let text = std::str::from_utf8(source).ok()?;
    for line in text.lines() {
        let line = line.trim_start();
        if let Some(rest) = line.strip_prefix("proc ") {
            let name = rest
                .trim_start()
                .split(|c: char| c.is_whitespace() || c == '{')
                .next()?;
            if !name.is_empty() {
                return Some(name.to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files(root: &Path) -> Vec<PathBuf> {
        let mut out = Vec::new();
        let mut stack = vec![root.to_path_buf()];
        while let Some(dir) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(&dir) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else {
                    out.push(path);
                }
            }
        }
        out.sort();
        out
    }

    /// The load-bearing claim: what was there before an edit is still readable
    /// after it.
    #[test]
    fn a_snapshot_holds_the_source_it_was_given() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut snaps = Snapshots::new(tmp.path());

        let written = snaps
            .record(0, "L4", 0, "soft_points", b"the first version")
            .expect("record")
            .expect("a first version is always new");
        assert_eq!(std::fs::read(&written).expect("read"), b"the first version");

        let second = snaps
            .record(0, "L4", 0, "soft_points", b"the second version")
            .expect("record")
            .expect("changed");
        assert_ne!(written, second, "the second snapshot overwrote the first");
        assert_eq!(std::fs::read(&written).expect("read"), b"the first version");
    }

    /// A rebuild recompiles both layers whichever one was saved. Without this
    /// the untouched layer's chain is a row of identical files, and the one
    /// question a history has to answer — what changed — is the one it stops
    /// being able to answer.
    #[test]
    fn an_unchanged_source_is_not_written_again() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut snaps = Snapshots::new(tmp.path());

        assert!(snaps
            .record(0, "L1", 0, "field", b"same")
            .expect("record")
            .is_some());
        assert!(
            snaps
                .record(0, "L1", 0, "field", b"same")
                .expect("record")
                .is_none(),
            "an identical source was written a second time"
        );
        assert_eq!(files(tmp.path()).len(), 1);
    }

    /// And "unchanged" is per slot and per layer, not global — two slots
    /// holding the same source are two chains, and an edit to one must not be
    /// mistaken for the other having already been recorded.
    #[test]
    fn each_slot_and_layer_has_its_own_chain() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut snaps = Snapshots::new(tmp.path());

        assert!(snaps
            .record(0, "L1", 0, "field", b"same")
            .expect("record")
            .is_some());
        assert!(
            snaps
                .record(1, "L1", 0, "field", b"same")
                .expect("record")
                .is_some(),
            "slot 1's first snapshot was skipped because slot 0 had the same source"
        );
        assert!(
            snaps
                .record(0, "L4", 0, "field", b"same")
                .expect("record")
                .is_some(),
            "the L4 chain was skipped because the L1 chain had the same source"
        );
        assert_eq!(files(tmp.path()).len(), 3);
    }

    /// **And per renderer, which is the one this was actually wrong about.**
    ///
    /// A slot draws with a list of L4s. Keyed by layer alone, every renderer of
    /// a stack shared one chain and one `last`, so two renderers holding
    /// different sources recorded one snapshot per save — each overwriting the
    /// other's memory of what it had last written, and each then looking
    /// changed on the next save. The chain a surface walks back through was
    /// alternating between two procedures neither of which had been edited.
    ///
    /// Both halves are asserted: two renderers are two chains, and the second
    /// renderer's own repeat is still skipped, so fixing the collision did not
    /// cost the "unchanged is not written again" property it was hiding.
    #[test]
    fn each_renderer_of_a_stack_has_its_own_chain() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut snaps = Snapshots::new(tmp.path());

        assert!(snaps
            .record(0, "L4", 0, "sprites", b"first")
            .expect("record")
            .is_some());
        assert!(
            snaps
                .record(0, "L4", 1, "strokes", b"second")
                .expect("record")
                .is_some(),
            "the second renderer's first snapshot was skipped as the first renderer's"
        );
        assert!(
            snaps
                .record(0, "L4", 1, "strokes", b"second")
                .expect("record")
                .is_none(),
            "the second renderer's unchanged source was written again"
        );
        assert!(
            snaps
                .record(0, "L4", 0, "sprites", b"first")
                .expect("record")
                .is_none(),
            "the first renderer looked changed because the second had written since"
        );
        assert_eq!(files(tmp.path()).len(), 2);
    }

    /// **A renderer's index is in the name only when it is not the first**, so
    /// every file a one-renderer run has ever written keeps the name it had. A
    /// history is read by a person looking for what they changed, and a `_0` on
    /// every L4 of every ordinary run is noise in the way of that.
    #[test]
    fn only_a_renderer_past_the_first_carries_its_index_in_the_name() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut snaps = Snapshots::new(tmp.path());

        let first = snaps
            .record(0, "L4", 0, "sprites", b"a")
            .expect("record")
            .expect("written");
        let second = snaps
            .record(0, "L4", 1, "strokes", b"b")
            .expect("record")
            .expect("written");
        let name =
            |p: &std::path::Path| p.file_name().expect("named").to_string_lossy().to_string();

        assert!(
            name(&first).contains("_L4_"),
            "the first renderer grew an index: {}",
            name(&first)
        );
        assert!(
            name(&second).contains("_L41_"),
            "the second renderer is not distinguishable from the first: {}",
            name(&second)
        );
    }

    /// The name has to say which slot and which layer it came from, or a
    /// directory of a night's work is unreadable.
    #[test]
    fn the_name_carries_the_day_the_slot_the_layer_and_the_procedure() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut snaps = Snapshots::new(tmp.path());
        let path = snaps
            .record(2, "L4", 0, "beat_strokes", b"x")
            .expect("record")
            .expect("new");

        let name = path
            .file_name()
            .expect("a name")
            .to_string_lossy()
            .to_string();
        assert!(name.contains("slot2"), "{name}");
        assert!(name.contains("L4"), "{name}");
        assert!(name.contains("beat_strokes"), "{name}");
        assert!(name.ends_with(".kir"), "{name}");

        // Nested a directory per day, which is what makes `rm -rf` the cleanup.
        let rel = path
            .strip_prefix(tmp.path().join(DIR))
            .expect("under the history root");
        assert_eq!(
            rel.components().count(),
            4,
            "expected YYYY/MM/DD/name, got {rel:?}"
        );

        // And the day is the operator's day, not UTC's — the whole reason the
        // dependency is here.
        let today = chrono::Local::now().format("%Y/%m/%d").to_string();
        assert!(
            path.to_string_lossy().contains(&today),
            "{} is not under today's local date {today}",
            path.display()
        );
    }

    /// **What makes the first edit undoable.** Without the seed, the first
    /// rebuild records the version that replaced the original and the original
    /// is nowhere.
    #[test]
    fn seeding_writes_the_version_a_run_starts_with() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let l1 = tmp.path().join("a.kir");
        let l4 = tmp.path().join("b.kir");
        std::fs::write(&l1, "proc field_one {\n  kind L1\n}").expect("write");
        std::fs::write(&l4, "proc draw_one {\n  kind L4\n}").expect("write");

        let shared = Snapshots::shared(tmp.path());
        seed(
            &shared,
            std::iter::once((0, vec![l1.as_path(), l4.as_path()])),
        );

        let names: Vec<String> = files(&tmp.path().join(DIR))
            .iter()
            .map(|p| p.file_name().expect("name").to_string_lossy().to_string())
            .collect();
        assert_eq!(names.len(), 2, "{names:?}");
        assert!(names.iter().any(|n| n.contains("field_one")), "{names:?}");
        assert!(names.iter().any(|n| n.contains("draw_one")), "{names:?}");
    }

    /// **A chain's starting version is filed under the layer its file
    /// declares.** Every path after the first was filed as `L4`, so an L2's
    /// first snapshot landed under a name the watcher does not use for the
    /// later ones — and the chain an operator walks back through began at the
    /// second edit, with the version the run started from unreachable under
    /// any name they would think to look for.
    #[test]
    fn a_seeded_chain_is_filed_under_the_layer_its_file_declares() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let write = |name: &str, body: &str| {
            let path = tmp.path().join(name);
            std::fs::write(&path, body).expect("write");
            path
        };
        let l1 = write("a.kir", "proc gen {\n  kind L1\n}");
        let l2 = write("b.kir", "proc warp {\n  kind L2\n}");
        let fld = write("c.kir", "proc blob {\n  kind Field\n}");
        let near = write("d.kir", "proc near {\n  kind L4\n}");
        let far = write("e.kir", "proc far {\n  kind L4\n}");

        let shared = Snapshots::shared(tmp.path());
        seed(
            &shared,
            std::iter::once((
                0,
                vec![
                    l1.as_path(),
                    l2.as_path(),
                    fld.as_path(),
                    near.as_path(),
                    far.as_path(),
                ],
            )),
        );

        let names: Vec<String> = files(&tmp.path().join(DIR))
            .iter()
            .map(|p| p.file_name().expect("name").to_string_lossy().to_string())
            .collect();
        let has = |part: &str| names.iter().any(|n| n.contains(part));
        assert_eq!(names.len(), 5, "{names:?}");
        assert!(has("L1_gen"), "{names:?}");
        assert!(has("L2_warp"), "{names:?}");
        assert!(has("Field_blob"), "{names:?}");
        // And the index counts within a layer, so the second renderer is the
        // one that carries a number — not the second file.
        assert!(has("L4_near"), "{names:?}");
        assert!(has("L41_far"), "{names:?}");
    }

    /// And the seed shares the dedup with the watcher, or the first rebuild
    /// writes the untouched procedure all over again — which is the duplicate
    /// this arrangement exists to avoid.
    #[test]
    fn a_rebuild_after_seeding_does_not_rewrite_an_untouched_procedure() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let l1 = tmp.path().join("a.kir");
        let l4 = tmp.path().join("b.kir");
        std::fs::write(&l1, "proc field_one {}").expect("write");
        std::fs::write(&l4, "proc draw_one {}").expect("write");

        let shared = Snapshots::shared(tmp.path());
        seed(
            &shared,
            std::iter::once((0, vec![l1.as_path(), l4.as_path()])),
        );

        let mut snapshots = shared.lock().expect("lock");
        assert!(
            snapshots
                .record(0, "L1", 0, "field_one", b"proc field_one {}")
                .expect("record")
                .is_none(),
            "the untouched L1 was written a second time"
        );
        assert!(
            snapshots
                .record(0, "L4", 0, "draw_one", b"proc draw_one { edited }")
                .expect("record")
                .is_some(),
            "the edited L4 was skipped"
        );
    }

    /// **Two saves inside one millisecond get two ids**, so the second does not
    /// write over the first.
    ///
    /// The clock is the only thing a live save can be named by — a key press
    /// cannot type a name — and it resolves to a millisecond, which is finer
    /// than a hand and not finer than a program. `Live::save_set` prints the id
    /// and reports that the file was kept, so a collision is not a lost save
    /// but a save reported as kept and then overwritten by the next one.
    ///
    /// Driven through [`unused`] with a fixed stamp rather than by calling
    /// `stamped_id` in a tight loop: this is about the rule, and a test that
    /// depended on two calls landing in the same millisecond would pass by
    /// accident on a slow machine.
    #[test]
    fn two_ids_taken_off_one_millisecond_are_two_ids() {
        let mut issued = std::collections::HashSet::new();
        let stamp = "20260816-143052-271".to_string();
        let ids: Vec<String> = (0..3).map(|_| unused(&mut issued, stamp.clone())).collect();

        assert_eq!(
            ids,
            vec![
                "20260816-143052-271",
                "20260816-143052-271-1",
                "20260816-143052-271-2"
            ],
            "a second save in the same millisecond was handed the first one's \
             id, so its Set file wrote over a file the operator was told had \
             been kept"
        );

        // **The control.** A different millisecond is left exactly as it is —
        // an operator looks for the time they pressed the key, and an index on
        // every id would be noise in the way of that.
        assert_eq!(
            unused(&mut issued, "20260816-143052-272".to_string()),
            "20260816-143052-272"
        );
    }

    /// A procedure name reaches this from a file somebody else wrote. It is an
    /// identifier by the time it gets here, and a path component is not where
    /// to discover that the parser and this module disagree about that.
    #[test]
    fn a_procedure_name_cannot_escape_the_history_directory() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut snaps = Snapshots::new(tmp.path());
        let path = snaps
            .record(0, "L1", 0, "../../etc/passwd", b"x")
            .expect("record")
            .expect("new");

        assert!(
            path.starts_with(tmp.path().join(DIR)),
            "{} escaped the history root",
            path.display()
        );
        assert!(!path.to_string_lossy().contains(".."), "{}", path.display());
    }
}
