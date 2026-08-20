//! Watching one slot's `.kir` files and rebuilding when they change.
//!
//! This is the [`Source`] the engine's build worker polls. It runs on that
//! worker thread, so everything expensive it does — a file read, four
//! validation stages, and the diagnostics it prints when one of them refuses —
//! is already off the render thread before `Set::build` is even reached.
//!
//! **One of these per deck slot, over that slot's own files.** A slot is the
//! unit that gets replaced — that is what it means for each slot to own its own
//! `HotSwap` — so the way "rebuild the slot whose files changed" is enforced is
//! that no watcher can see another slot's files at all. Two slots given the
//! same files both rebuild, which is right: the same edit reached both of them.
//!
//! ## Polling, not `notify`
//!
//! The worker is a poll loop already: it has to wake regularly to free Sets
//! the render thread retired, so there is no blocking `recv` for a filesystem
//! event to replace. Given that, `notify` would add a dependency, a platform
//! backend per OS, and a second event vocabulary to debounce, in exchange for
//! shaving a tenth of a second off a loop whose other end is a human editing a
//! file. Reading a handful of small files every hundred milliseconds is not a
//! cost worth avoiding here, and it is the same amount of code.
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

/// How often the files are stamped. Two polls of quiet are needed before a
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
    /// Every node of the rebuilt slot, each with the address a `procedure`
    /// record names it by — `(layer, index)`, the layer spelled the way the
    /// record spells it.
    ///
    /// **The whole stack, not the one file that changed.** A rebuild restates
    /// the slot, so a record of what played has to name every node in it; and
    /// it is a list rather than an L1 and some renderers because a slot can
    /// hold two geometries, a chain and a camera, all of which are procedures
    /// somebody may have just edited.
    pub nodes: Vec<(&'static str, u32, karakuri_store::hash::Hash)>,
}

pub struct Watch {
    /// How many builds this watcher has requested, which with the slot makes
    /// an id no other build in the run shares.
    builds: u64,
    /// Where to record what was built, when a session is being recorded.
    /// `None` and nothing here reads or writes a store at all.
    recording: Option<(
        std::sync::Arc<karakuri_store::store::Store>,
        std::sync::mpsc::Sender<Built>,
    )>,
    /// Which slot this rebuilds. Carried only so that the diagnostics this
    /// prints — from a worker thread, interleaved with every other slot's — say
    /// which of the four they are about.
    slot: usize,
    /// The file the slot was spelled with. **Not necessarily its L1** — what
    /// layer it is on is what its own `kind` declares, which the sort reads
    /// like it reads every other file's. It is first here only because it is
    /// first on the command line, and list order is chain order.
    head: PathBuf,
    /// The rest of the slot's files, in the order they were spelled. Watched
    /// together with the head: a rebuild restates the whole stack, so an edit
    /// to any one of them recompiles all of them and the Set that lands is the
    /// one the files say.
    rest: Vec<PathBuf>,
    /// Whether this slot's renderers composite or overdraw. Restated on every
    /// rebuild rather than read off the outgoing Set, for the reason
    /// `Request::bindings` gives.
    layering: karakuri_engine::set::Layering,
    /// `--capacity` when it was given, and otherwise `None` — each geometry
    /// then runs at the default its own `capacity` declaration names.
    ///
    /// **Not one resolved number.** A rebuild recompiles the files, so a
    /// procedure's declared default is the *new* file's, and a slot with two
    /// geometries has two of them. Resolving at startup and carrying the answer
    /// would have pinned every later build to whatever the first one declared.
    capacity: Option<u32>,
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
        head: PathBuf,
        rest: Vec<PathBuf>,
        layering: karakuri_engine::set::Layering,
        capacity: Option<u32>,
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
            head,
            rest,
            layering,
            capacity,
            seed_salt,
            overrides,
            published,
            bindings,
            stamps: Vec::new(),
            settling: false,
        };
        // Seeded from what is on disk right now, so that the files the CLI
        // already compiled at startup are not immediately compiled again.
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
        std::iter::once(digest(&self.head))
            .chain(self.rest.iter().map(digest))
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
        // Every file, not only the one that changed: the composition check
        // needs the whole stack, and an L4 that stopped being compatible with
        // its L1 is a diagnostic rather than a half-applied edit.
        //
        // **The source is read here and handed on**, rather than compiled and
        // thrown away. A session that records what it played has to name the
        // procedure that was playing, and the only moment both the text and the
        // build it produced are in the same hand is this one — by the time the
        // swap lands, the file may have changed again.
        let paths: Vec<&std::path::Path> = std::iter::once(self.head.as_path())
            .chain(self.rest.iter().map(PathBuf::as_path))
            .collect();
        let mut srcs = Vec::with_capacity(paths.len());
        for path in &paths {
            match std::fs::read_to_string(path) {
                Ok(src) => srcs.push(src),
                Err(e) => {
                    eprintln!("{}: {e}\nslot {slot} unchanged", path.display());
                    return None;
                }
            }
        }
        let mut compiled = Vec::with_capacity(paths.len());
        for (path, src) in paths.iter().zip(&srcs) {
            match compile::check(src) {
                // Bare, because a watcher has no names to give: `Watch::new`
                // is handed paths. What addresses these nodes is `(slot, layer,
                // index)`, which is what the sort answers.
                Ok(checked) => compiled.push((crate::Named::bare(*path), checked)),
                Err(report) => {
                    eprintln!(
                        "{}:\n{report}\nslot {slot} unchanged; its Set is still running",
                        path.display()
                    );
                    return None;
                }
            }
        }
        // **Sorted by the `kind` each file declares, in the same code the
        // startup path sorts with** — see [`crate::sort_compiled`], which says
        // why that is one function. This used to be a copy of that match, and a
        // copy is how the two came to disagree about the head: it was taken for
        // the L1 whatever it declared, so a slot spelled with an L2 first
        // started fine and then rebuilt into a Set the engine refused.
        //
        // **A rebuild that cannot be assembled is `None`**, on the same terms a
        // compile error is: the running Set keeps running, and the operator is
        // told which file is the problem. Editing a slot into an illegal shape
        // must not take the picture down — which is the whole of what this side
        // does differently, and the reason the sort returns a sentence rather
        // than exiting.
        let (material, placed) = match crate::sort_compiled(compiled) {
            Ok(sorted) => sorted,
            Err(e) => {
                eprintln!("slot {slot}: {e}\nslot {slot} unchanged; its Set is still running");
                return None;
            }
        };

        // **Everything compiled**, which is this feature's whole gate: a version
        // that does not compile is not a version, and one that compiled is worth
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
                    // Every file, each under its own layer and index. A rebuild
                    // recompiles the whole stack whichever file was saved, so
                    // every one of them is offered — and `Snapshots::record`
                    // drops the ones that did not change, which is what keeps
                    // the untouched renderers' chains from becoming rows of
                    // identical files.
                    // **`placed` and `srcs` are both in the slot's file
                    // order**, which the sort deliberately did not disturb: what
                    // a version is recorded under is the layer and the position
                    // it was *built* at, and which file it came from is how it
                    // is found again.
                    for (node, src) in placed.iter().zip(&srcs) {
                        let layer = crate::setfile::kind_name(node.layer);
                        let index = node.index as usize;
                        if let Err(e) =
                            snapshots.record(slot, layer, index, &node.proc, src.as_bytes())
                        {
                            eprintln!("slot {slot}: this version is not in the edit history: {e}");
                        }
                    }
                }
                Err(_) => eprintln!("slot {slot}: the edit history is not being written"),
            }
        }

        let label = placed
            .iter()
            .map(|node| node.proc.as_str())
            .collect::<Vec<_>>()
            .join(" + ");
        // **Unique across the whole run**, because that is what it is for: the
        // caller matches an outcome back to the source that produced it, and
        // labels repeat on every rebuild of the same files.
        self.builds += 1;
        let id = (self.slot as u64) << 32 | self.builds;
        // **On this thread, which is the worker's.** A few artifacts of a few
        // kilobytes, written where a whole Set is about to be compiled anyway
        // — rather than on the frame that installs it.
        if let Some((store, tx)) = &self.recording {
            let stored: Result<Vec<_>, _> = srcs
                .iter()
                .map(|src| store.put_artifact(src.as_bytes()))
                .collect();
            match stored {
                Ok(hashes) => {
                    // **Every hash takes the address the sort gave its file**,
                    // the head included. `placed` and `srcs` are both in file
                    // order, which the sort deliberately did not disturb, so
                    // zipping the hashes onto it names each node the way a
                    // `procedure` record does.
                    let nodes = placed
                        .iter()
                        .zip(&hashes)
                        .map(|(node, hash)| {
                            (crate::setfile::kind_name(node.layer), node.index, *hash)
                        })
                        .collect();
                    let _ = tx.send(Built { id, nodes });
                }
                Err(e) => eprintln!(
                    "slot {slot}: this build is not in the session's record: {e} — \
                     a replay will show the procedure it started with"
                ),
            }
        }
        // The names the sort collected are dropped, because a watcher never had
        // any to collect: its files arrive as bare paths. See `names` below.
        let crate::Material {
            l1s,
            l2s,
            l3,
            field,
            l4s,
            ..
        } = material;
        Some(Request {
            id,
            // **Each geometry at the capacity it declares**, and `--capacity`
            // over all of them — the rule the startup path follows, asked again
            // here because a rebuild recompiles the files and the declaration
            // may have just changed.
            l1s: l1s
                .into_iter()
                .map(|l1| {
                    let capacity = self.capacity.unwrap_or_else(|| {
                        l1.capacity.map_or(crate::DEFAULT_CAPACITY, |c| c.default)
                    });
                    (l1, capacity)
                })
                .collect(),
            l2s,
            l3,
            field,
            l4s,
            // **Restated, like every other part of a request.** A rebuild that
            // read the names off the outgoing Set would depend on what happened
            // to be live, which is the property `bindings` gives its reason for.
            names: karakuri_engine::swap::RequestNames::default(),
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
            Some(4096),
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
        // **A directory of its own, not a fixed name under `/tmp`.** Two runs
        // of this suite at once — a `pre-push` hook beside a terminal, say —
        // shared the fixed one, and each deleted the other's files mid-test.
        // It failed once in a whole-workspace run and passed every time it was
        // run alone, which is what that shape looks like from the outside.
        let tmp = tempfile::tempdir().expect("temp dir");
        let dir = tmp.path();
        std::fs::write(dir.join("a.kir"), "proc a {}").expect("write");
        std::fs::write(dir.join("b.kir"), "proc b {}").expect("write");

        let w = watch_on(dir);
        let before = w.stamp();
        std::thread::sleep(Duration::from_millis(10));
        std::fs::write(dir.join("a.kir"), "proc a {}").expect("rewrite");
        assert_eq!(before, w.stamp(), "identical bytes read as a change");

        std::fs::write(dir.join("a.kir"), "proc a { }").expect("edit");
        assert_ne!(before, w.stamp(), "a real edit read as unchanged");
    }

    /// The examples this suite sorts, copied into a directory of their own so
    /// that a watcher can be built over paths that do not exist yet.
    ///
    /// **Built before the files are written**, which is what makes writing them
    /// the edit it wakes on: a missing file stamps as `None`, and appearing is
    /// a change like any other. The alternative is editing a file's text, which
    /// would make the two paths sort different bytes.
    fn watch_over(dir: &std::path::Path, files: &[&str]) -> (Watch, Vec<PathBuf>) {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let paths: Vec<PathBuf> = files.iter().map(|f| dir.join(f)).collect();
        let watch = Watch::new(
            0,
            paths[0].clone(),
            paths[1..].to_vec(),
            karakuri_engine::set::Layering::Overdraw,
            Some(4096),
            1,
            Vec::new(),
            Vec::new(),
            Vec::new(),
        );
        for (file, path) in files.iter().zip(&paths) {
            std::fs::copy(root.join("examples").join(file), path).expect("copy an example");
        }
        (watch, paths)
    }

    /// Polled until it builds, or four intervals, whichever is first. One poll
    /// sees the change and the next acts on it — see "Debouncing" — so a build
    /// that has not arrived by the fourth is a refusal.
    fn rebuild(watch: &mut Watch) -> Option<Request> {
        std::iter::repeat_with(|| watch.poll())
            .take(4)
            .flatten()
            .next()
    }

    /// **The startup path and the rebuild path answer "which layer is this file
    /// on, and which node of that layer" identically**, which is the whole
    /// reason [`crate::sort_compiled`] is one function rather than a match in
    /// each of them.
    ///
    /// The two used to hold a copy each and had already drifted: the rebuild
    /// took its head for the L1 whatever the file declared, so a slot spelled
    /// with a deformer first sorted one way at startup and another way on the
    /// first save. Asserted as agreement rather than as two expected answers,
    /// because what has to hold is that they are the same answer — an expected
    /// answer written twice is the drift again, in the tests.
    #[test]
    fn a_rebuild_addresses_a_slot_exactly_as_startup_did() {
        // A whole stack, head first and deliberately not an L1: every layer, and
        // two of the three that can hold several, so a wrong *index* fails here
        // as loudly as a wrong layer.
        let files = [
            "swirl_warp.kir",
            "drift_shell.kir",
            "beat_jump.kir",
            "melt_blob.kir",
            "soft_points.kir",
            "lattice_shell.kir",
            "kaleidoscope.kir",
            "glass_shell.kir",
        ];
        let tmp = tempfile::tempdir().expect("temp dir");
        let (watch, paths) = watch_over(tmp.path(), &files);

        let store_dir = tempfile::tempdir().expect("temp dir");
        let store = std::sync::Arc::new(
            karakuri_store::store::Store::open(store_dir.path()).expect("store"),
        );
        let (tx, rx) = std::sync::mpsc::channel();
        let mut watch = watch.recording_to(store, tx);
        let request = rebuild(&mut watch).expect("the stack compiles, so it rebuilds");

        let rest: Vec<crate::Named> = paths[1..]
            .iter()
            .map(|p| crate::Named::bare(p.clone()))
            .collect();
        let (material, placed) = crate::sort_slot(0, &crate::Named::bare(paths[0].clone()), &rest);

        let procs = |checked: &[karakuri_ir::typed::Checked]| {
            checked
                .iter()
                .map(|c| c.name.clone())
                .collect::<Vec<String>>()
        };
        assert_eq!(
            request
                .l1s
                .iter()
                .map(|(c, _)| c.name.clone())
                .collect::<Vec<String>>(),
            procs(&material.l1s),
            "the two paths disagree about the geometries"
        );
        assert_eq!(procs(&request.l2s), procs(&material.l2s), "the deformers");
        assert_eq!(procs(&request.l4s), procs(&material.l4s), "the renderers");
        let camera = |c: &Option<karakuri_ir::typed::Checked>| c.as_ref().map(|c| c.name.clone());
        assert_eq!(camera(&request.l3), camera(&material.l3), "the camera");
        assert_eq!(camera(&request.field), camera(&material.field), "the field");

        // **The addresses, file by file**, which is the half a request cannot
        // show: a `procedure` record names a node by `(layer, index)`, and the
        // history files a version under the same pair.
        let built = rx.try_recv().expect("a recorded build is reported");
        let rebuilt: Vec<(&str, u32)> = built
            .nodes
            .iter()
            .map(|(layer, index, _)| (*layer, *index))
            .collect();
        let started: Vec<(&str, u32)> = placed
            .iter()
            .map(|node| (crate::setfile::kind_name(node.layer), node.index))
            .collect();
        assert_eq!(
            rebuilt, started,
            "the two paths address the same files as different nodes"
        );
        // The premise, stated so that a rewrite of the file list cannot quietly
        // turn this back into a test about a slot whose head is its L1.
        assert_eq!(
            started[0],
            ("L2", 0),
            "this asserts nothing unless the head is a file that is not the L1"
        );
    }

    /// **A rebuild that cannot be assembled leaves the running Set alone.** This
    /// is the one thing the two sorting paths do differently, and the reason
    /// [`crate::sort_compiled`] hands back a sentence rather than exiting: a
    /// startup with no picture has nothing to keep showing, and an operator
    /// editing a slot into an illegal shape has a picture on stage.
    #[test]
    fn a_slot_that_cannot_be_assembled_leaves_the_running_set_alone() {
        // Two cameras, and then the same stack with one — so this fails if the
        // refusal stopped happening *and* if it started happening to everything.
        for (files, buildable) in [
            (
                &["drift_shell.kir", "beat_jump.kir", "soft_points.kir"][..],
                true,
            ),
            (
                &[
                    "drift_shell.kir",
                    "beat_jump.kir",
                    "beat_jump.kir",
                    "soft_points.kir",
                ][..],
                false,
            ),
            // Nothing that draws: a Set with no renderer has no frame to give.
            (&["drift_shell.kir", "swirl_warp.kir"][..], false),
        ] {
            let tmp = tempfile::tempdir().expect("temp dir");
            // Two of the same file need two names, or the copy is one file.
            let unique: Vec<String> = files
                .iter()
                .enumerate()
                .map(|(n, f)| format!("{n}_{f}"))
                .collect();
            let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
            let paths: Vec<PathBuf> = unique.iter().map(|f| tmp.path().join(f)).collect();
            let mut watch = Watch::new(
                0,
                paths[0].clone(),
                paths[1..].to_vec(),
                karakuri_engine::set::Layering::Overdraw,
                Some(4096),
                1,
                Vec::new(),
                Vec::new(),
                Vec::new(),
            );
            for (file, path) in files.iter().zip(&paths) {
                std::fs::copy(root.join("examples").join(file), path).expect("copy an example");
            }
            assert_eq!(
                rebuild(&mut watch).is_some(),
                buildable,
                "{files:?} rebuilt the wrong way"
            );
        }
    }

    /// A missing file is stable rather than a change every interval, so
    /// deleting one does not put the watcher into a recompile loop against a
    /// path that is not there.
    #[test]
    fn a_missing_file_is_stable() {
        let tmp = tempfile::tempdir().expect("temp dir");
        let w = watch_on(tmp.path());
        assert_eq!(w.stamp(), [None, None]);
        assert_eq!(w.stamp(), w.stamp());
    }
}
