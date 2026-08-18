//! Watching one slot's two `.kir` files and rebuilding when they change.
//!
//! This is the [`Source`] the engine's build worker polls. It runs on that
//! worker thread, so everything expensive it does — a file read, four
//! validation stages, and the diagnostics it prints when one of them refuses —
//! is already off the render thread before `Set::build` is even reached.
//!
//! **One of these per deck slot, over that slot's own pair.** A slot is the
//! unit that gets replaced — that is what it means for each slot to own its own
//! `HotSwap` — so the way "rebuild the slot whose files changed" is enforced is
//! that no watcher can see another slot's files at all. Two slots given the
//! same pair both rebuild, which is right: the same edit reached both of them.
//!
//! ## Polling, not `notify`
//!
//! The worker is a poll loop already: it has to wake regularly to free Sets
//! the render thread retired, so there is no blocking `recv` for a filesystem
//! event to replace. Given that, `notify` would add a dependency, a platform
//! backend per OS, and a second event vocabulary to debounce, in exchange for
//! shaving a tenth of a second off a loop whose other end is a human editing a
//! file. Reading two small files every hundred milliseconds is not a cost worth
//! avoiding here, and it is the same amount of code.
//!
//! ## Contents, not timestamps
//!
//! What is compared is a hash of each file's bytes, not its mtime. A swap is
//! not free — the incoming Set starts cold, at `t` zero, with nothing primed —
//! so it must not happen for a save that changed no text. `touch`, a formatter
//! that rewrites identical bytes, and `git checkout` of the branch you are
//! already on all move the mtime, and under an mtime comparison each of them
//! restarts the visual for no reason. Reading the file is not extra work
//! either: this `Source` reads it a moment later anyway to compile it.
//!
//! ## Debouncing
//!
//! An editor that writes in place rather than renaming leaves the file
//! truncated for a moment, and reading it then produces a parse error against
//! a file that is about to be fine. So a change is not acted on the poll it is
//! seen; it is acted on the first poll where the timestamps have stopped
//! moving. That costs one extra interval of latency and removes a whole class
//! of spurious diagnostics.
//!
//! A compile that fails prints every diagnostic it has and returns `None`.
//! Nothing is requested, so nothing is built, so nothing is swapped — the
//! running Set keeps running with its `t` and its element buffers untouched.

use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::PathBuf;
use std::time::Duration;

use karakuri_engine::{Binding, Request, Source};

use crate::compile;

/// How often the two files are stamped. Two polls of quiet are needed before a
/// change is acted on, so this is half the latency of a save reaching the
/// screen — a fifth of a second, against a compile that takes longer than that
/// anyway.
const INTERVAL: Duration = Duration::from_millis(100);

/// What a build was made from, as the session stream will name it.
///
/// **Hashes and not text, and put in the store on this side of the channel**,
/// which is the worker's thread. A session that records what it played has to
/// name the procedure, and the two ways to get the bytes there both end on the
/// render thread: reading the file when the swap lands is too late, because it
/// may have changed again, and carrying the text across means writing it to
/// disk on a frame. This is neither.
pub struct Built {
    /// The id carried on every `swap::Event` about this build. **`(slot << 32)
    /// | n`**, so it names the slot as well and no two builds in a run share
    /// one.
    pub id: u64,
    pub l1: karakuri_store::hash::Hash,
    /// The renderers, in draw order — the whole stack, because a `Request`
    /// restates the whole stack and a record of what played has to name what
    /// was built.
    pub l4s: Vec<karakuri_store::hash::Hash>,
}

pub struct Watch {
    /// How many builds this watcher has requested, which with the slot makes
    /// an id no other build in the run shares.
    builds: u64,
    /// Where to record what was built, when a session is being recorded.
    /// `None` and nothing here reads or writes a store at all.
    recording: Option<(std::sync::Arc<karakuri_store::store::Store>, std::sync::mpsc::Sender<Built>)>,
    /// Which slot this rebuilds. Carried only so that the diagnostics this
    /// prints — from a worker thread, interleaved with every other slot's — say
    /// which of the four they are about.
    slot: usize,
    l1: PathBuf,
    /// The renderers this slot draws with, in draw order. Watched together:
    /// a rebuild restates the whole stack, so an edit to any one of them
    /// recompiles all of them and the Set that lands is the one the files say.
    l4s: Vec<PathBuf>,
    /// Whether this slot's renderers composite or overdraw. Restated on every
    /// rebuild rather than read off the outgoing Set, for the reason
    /// `Request::bindings` gives.
    layering: karakuri_engine::set::Layering,
    capacity: u32,
    seed_salt: u32,
    overrides: Vec<karakuri_engine::ParamWrite>,
    /// The interface, restated on every rebuild for the same reason the
    /// bindings are — and the more urgent one: a `control:` binding whose
    /// control did not survive the swap holds its param where it found it.
    published: Vec<karakuri_engine::set::Published>,
    /// Restated on every rebuild rather than read off the outgoing Set, for
    /// the reason `Request::bindings` gives: a request that depended on what
    /// happened to be live would not be reproducible from a record stream.
    bindings: Vec<Binding>,
    /// A hash of each file's contents as of the previous poll. `None` for a
    /// file that does not exist or cannot be read, which compares equal to
    /// itself and so reads as "unchanged" rather than as a change every
    /// interval — a deleted file is not an edit to react to.
    stamps: Vec<Option<u64>>,
    /// A change has been seen but not yet acted on — see "Debouncing" above.
    settling: bool,
    /// Where every version that compiled is kept, so an edit can be undone.
    /// `None` when no store root was given, which is the offscreen paths.
    ///
    /// **Separate from `recording`, and not folded into it.** A session
    /// recording names what reached the *screen*; this keeps what reached the
    /// *compiler*, and the difference is the whole value — a build that was
    /// rolled back for costing too much never becomes a `Record::Procedure`
    /// and is exactly the version an operator wants back.
    snapshots: Option<crate::history::Shared>,
}

impl Watch {
    // Eight, where clippy's line is seven. Six of them are one slot's identity
    // — its files, its layering, its capacity, its seed and the values it was
    // started with — and a struct to carry them would be `Watch` itself,
    // constructed one field short.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        slot: usize,
        l1: PathBuf,
        l4s: Vec<PathBuf>,
        layering: karakuri_engine::set::Layering,
        capacity: u32,
        seed_salt: u32,
        overrides: Vec<karakuri_engine::ParamWrite>,
        published: Vec<karakuri_engine::set::Published>,
        bindings: Vec<Binding>,
    ) -> Watch {
        let mut watch = Watch {
            builds: 0,
            recording: None,
            snapshots: None,
            slot,
            l1,
            l4s,
            layering,
            capacity,
            seed_salt,
            overrides,
            published,
            bindings,
            stamps: Vec::new(),
            settling: false,
        };
        // Seeded from what is on disk right now, so that the pair the CLI
        // already compiled at startup is not immediately compiled again.
        watch.stamps = watch.stamp();
        watch
    }

    fn stamp(&self) -> Vec<Option<u64>> {
        // Not a cryptographic hash and not trying to be: this compares a file
        // against its own previous contents seconds earlier, where the only
        // adversary is an editor writing the same bytes back.
        let digest = |path: &PathBuf| {
            std::fs::read(path).ok().map(|bytes| {
                let mut h = DefaultHasher::new();
                bytes.hash(&mut h);
                h.finish()
            })
        };
        std::iter::once(digest(&self.l1))
            .chain(self.l4s.iter().map(digest))
            .collect()
    }
}

impl Watch {
    /// Record what this watcher builds, into `store`, reported on `tx`.
    pub fn recording_to(
        mut self,
        store: std::sync::Arc<karakuri_store::store::Store>,
        tx: std::sync::mpsc::Sender<Built>,
    ) -> Watch {
        self.recording = Some((store, tx));
        self
    }

    /// Keep every version that compiles under `store_root`, so an edit can be
    /// walked back. See [`crate::history`].
    pub fn snapshotting_to(mut self, snapshots: crate::history::Shared) -> Watch {
        self.snapshots = Some(snapshots);
        self
    }
}

impl Source for Watch {
    fn poll(&mut self) -> Option<Request> {
        std::thread::sleep(INTERVAL);

        let stamps = self.stamp();
        if stamps != self.stamps {
            self.stamps = stamps;
            self.settling = true;
            return None;
        }
        if !self.settling {
            return None;
        }
        self.settling = false;

        let slot = self.slot;
        eprintln!("slot {slot}: recompiling:");
        // Both files, not just the changed one: the composition check needs
        // the pair, and an L4 that stopped being compatible with its L1 is a
        // diagnostic rather than a half-applied edit.
        // **The source is read here and handed on**, rather than compiled and
        // thrown away. A session that records what it played has to name the
        // procedure that was playing, and the only moment both the text and the
        // build it produced are in the same hand is this one — by the time the
        // swap lands, the file may have changed again.
        let l1_src = match std::fs::read_to_string(&self.l1) {
            Ok(src) => src,
            Err(e) => {
                eprintln!("{}: {e}\nslot {slot} unchanged", self.l1.display());
                return None;
            }
        };
        let mut l4_srcs = Vec::with_capacity(self.l4s.len());
        for path in &self.l4s {
            match std::fs::read_to_string(path) {
                Ok(src) => l4_srcs.push(src),
                Err(e) => {
                    eprintln!("{}: {e}\nslot {slot} unchanged", path.display());
                    return None;
                }
            }
        }
        let l1 = match compile::check(&l1_src) {
            Ok(checked) => checked,
            Err(report) => {
                eprintln!(
                    "{}:\n{report}\nslot {slot} unchanged; its Set is still running",
                    self.l1.display()
                );
                return None;
            }
        };
        // **Sorted by the `kind` each file declares**, exactly as the startup
        // path sorts them. This used to push every file into the renderer list,
        // which was right while a slot named an L1 and renderers and became
        // wrong without a word when `--set` learned to spell a chain: a rebuild
        // then handed `Set::build_many` an L2 or an L3 as a renderer, the build
        // was rejected with "slot L4 needs a L4 procedure, got L3", and the slot
        // kept its old Set for the rest of the run. Every save, silently.
        let mut compiled = Vec::with_capacity(l4_srcs.len());
        for (path, src) in self.l4s.iter().zip(&l4_srcs) {
            match compile::check(src) {
                Ok(checked) => compiled.push(checked),
                Err(report) => {
                    eprintln!(
                        "{}:\n{report}\nslot {slot} unchanged; its Set is still running",
                        path.display()
                    );
                    return None;
                }
            }
        }
        // Kept alongside each procedure so the edit history and the session
        // record can name the layer and the position it was built at, rather
        // than filing a camera under `L4` index 2.
        let mut addressed: Vec<(&'static str, usize, String)> = Vec::with_capacity(compiled.len());
        let mut l2s = Vec::new();
        let mut l3: Option<karakuri_ir::typed::Checked> = None;
        let mut field: Option<karakuri_ir::typed::Checked> = None;
        let mut l4s = Vec::new();
        for (path, checked) in self.l4s.iter().zip(compiled) {
            // **A rebuild that cannot be built is `None`**, on the same terms a
            // compile error is: the running Set keeps running, and the operator
            // is told which file is the problem. Editing a slot into an illegal
            // shape must not take the picture down.
            let refuse = |what: &str| {
                eprintln!(
                    "{}: {what}\nslot {slot} unchanged; its Set is still running",
                    path.display()
                );
            };
            match checked.kind {
                karakuri_ir::Kind::L2 => {
                    addressed.push(("L2", l2s.len(), checked.name.clone()));
                    l2s.push(checked);
                }
                karakuri_ir::Kind::L3 if l3.is_some() => {
                    refuse("a second L3 — a slot looks from one camera");
                    return None;
                }
                karakuri_ir::Kind::L3 => {
                    addressed.push(("L3", 0, checked.name.clone()));
                    l3 = Some(checked);
                }
                karakuri_ir::Kind::L4 => {
                    addressed.push(("L4", l4s.len(), checked.name.clone()));
                    l4s.push(checked);
                }
                // **Refused rather than accepted and ignored.** A Set cannot
                // hold a field yet — nothing splices one into its callers — so
                // rebuilding with one would produce a slot that silently lost
                // the file it was told to watch.
                karakuri_ir::Kind::Field if field.is_some() => {
                    refuse("a second `kind Field` — a slot evaluates one field");
                    return None;
                }
                karakuri_ir::Kind::Field => {
                    addressed.push(("Field", 0, checked.name.clone()));
                    field = Some(checked);
                }
                karakuri_ir::Kind::L1 => {
                    refuse("a second L1 — a slot simulates with one geometry");
                    return None;
                }
            }
        }
        if l4s.is_empty() {
            eprintln!(
                "slot {slot}: nothing here draws\nslot {slot} unchanged; its Set is still running"
            );
            return None;
        }

        // **Both compiled**, which is this feature's whole gate: a version that
        // does not compile is not a version, and one that compiled is worth
        // keeping whether or not it goes on to fit the frame budget.
        //
        // Reported and never acted on. A snapshot that could not be written
        // must not stop a build that compiled — the operator asked for a
        // picture and this is bookkeeping.
        if let Some(snapshots) = &self.snapshots {
            // A poisoned lock means another thread panicked mid-snapshot. That
            // is a bug to find in the log, not a reason to stop a build that
            // compiled — this is bookkeeping either way.
            match snapshots.lock() {
                Ok(mut snapshots) => {
                    // Every renderer, each under its own index. A rebuild
                    // recompiles the whole stack whichever file was saved, so
                    // every one of them is offered — and `Snapshots::record`
                    // drops the ones that did not change, which is what keeps
                    // the untouched renderers' chains from becoming rows of
                    // identical files.
                    // **`addressed` and `l4_srcs` are both in the slot's file
                    // order**, which the sort above deliberately did not
                    // disturb: what a version is recorded under is the layer and
                    // the position it was *built* at, and which file it came
                    // from is how it is found again.
                    let rest = addressed
                        .iter()
                        .zip(&l4_srcs)
                        .map(|((layer, index, name), src)| (*layer, *index, name, src));
                    for (layer, index, name, src) in
                        [("L1", 0, &l1.name, &l1_src)].into_iter().chain(rest)
                    {
                        if let Err(e) = snapshots.record(slot, layer, index, name, src.as_bytes()) {
                            eprintln!("slot {slot}: this version is not in the edit history: {e}");
                        }
                    }
                }
                Err(_) => eprintln!("slot {slot}: the edit history is not being written"),
            }
        }

        let label = format!(
            "{} + {}",
            l1.name,
            addressed.iter().map(|(.., name)| name.as_str()).collect::<Vec<_>>().join(" + ")
        );
        // **Unique across the whole run**, because that is what it is for: the
        // caller matches an outcome back to the source that produced it, and
        // labels repeat on every rebuild of the same pair.
        self.builds += 1;
        let id = (self.slot as u64) << 32 | self.builds;
        // **On this thread, which is the worker's.** Two artifacts of a few
        // kilobytes, written where a whole Set is about to be compiled anyway
        // — rather than on the frame that installs it.
        if let Some((store, tx)) = &self.recording {
            let stored: Result<Vec<_>, _> = std::iter::once(l1_src.as_bytes())
                .chain(l4_srcs.iter().map(|s| s.as_bytes()))
                .map(|bytes| store.put_artifact(bytes))
                .collect();
            match stored {
                Ok(hashes) => {
                    let _ = tx.send(Built {
                        id,
                        l1: hashes[0],
                        l4s: hashes[1..].to_vec(),
                    });
                }
                Err(e) => eprintln!(
                    "slot {slot}: this build is not in the session's record: {e} — \
                     a replay will show the procedure it started with"
                ),
            }
        }
        Some(Request {
            id,
            l1s: vec![(l1, self.capacity)],
            l2s,
            l3,
            field,
            l4s,
            layering: self.layering,
            seed_salt: self.seed_salt,
            params: self.overrides.clone(),
            published: self.published.clone(),
            bindings: self.bindings.clone(),
            label,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn watch_on(dir: &std::path::Path) -> Watch {
        Watch::new(
            0,
            dir.join("a.kir"),
            vec![dir.join("b.kir")],
            karakuri_engine::set::Layering::Overdraw,
            4096,
            1,
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )
    }

    /// A save that changed no bytes is not an edit. Under an mtime comparison
    /// `touch`, a formatter, or a `git checkout` of the branch already checked
    /// out each restarts the visual from `t` zero for nothing.
    #[test]
    fn rewriting_identical_bytes_is_not_a_change() {
        let dir = std::env::temp_dir().join("karakuri-watch-identical");
        std::fs::create_dir_all(&dir).expect("temp dir");
        std::fs::write(dir.join("a.kir"), "proc a {}").expect("write");
        std::fs::write(dir.join("b.kir"), "proc b {}").expect("write");

        let w = watch_on(&dir);
        let before = w.stamp();
        std::thread::sleep(Duration::from_millis(10));
        std::fs::write(dir.join("a.kir"), "proc a {}").expect("rewrite");
        assert_eq!(before, w.stamp(), "identical bytes read as a change");

        std::fs::write(dir.join("a.kir"), "proc a { }").expect("edit");
        assert_ne!(before, w.stamp(), "a real edit read as unchanged");
        std::fs::remove_dir_all(&dir).ok();
    }

    /// A missing file is stable rather than a change every interval, so
    /// deleting one does not put the watcher into a recompile loop against a
    /// path that is not there.
    #[test]
    fn a_missing_file_is_stable() {
        let dir = std::env::temp_dir().join("karakuri-watch-missing");
        std::fs::create_dir_all(&dir).expect("temp dir");
        let w = watch_on(&dir);
        assert_eq!(w.stamp(), [None, None]);
        assert_eq!(w.stamp(), w.stamp());
        std::fs::remove_dir_all(&dir).ok();
    }
}
