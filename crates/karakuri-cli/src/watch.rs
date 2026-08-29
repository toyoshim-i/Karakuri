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
    /// Where to put what was built, and who to tell. `None` and nothing here
    /// reads or writes a store at all.
    ///
    /// **Wired whenever the run is editable, not only when a session is being
    /// recorded.** The two were the same switch because the session stream was
    /// the only reader: [`Built::nodes`] existed to become `procedure` records,
    /// so there was no reason to store a build nobody would name. A live save
    /// is the second reader and it wants exactly the same fact — *which sources
    /// landed* — and it wants it in the ordinary case, which is `--watch` with
    /// no recording at all. Coupling them meant the one control that saves what
    /// is on screen worked only for a run that was already recording itself.
    ///
    /// Nothing about the session moved with it: `Live::record_procedure` still
    /// writes a record only when there is a recorder. What changed is that the
    /// hashes exist either way.
    stored: Option<(
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
    head: crate::Named,
    /// The rest of the slot's files, in the order they were spelled. Watched
    /// together with the head: a rebuild restates the whole stack, so an edit
    /// to any one of them recompiles all of them and the Set that lands is the
    /// one the files say.
    rest: Vec<crate::Named>,
    /// Whether this slot's renderers composite or overdraw. Restated on every
    /// rebuild rather than read off the outgoing Set, for the reason
    /// `Request::bindings` gives.
    ///
    /// **And rather than re-derived from `--merge`**, which is the sharper half
    /// now that a Set file records a layering: a slot filled by `--load-set`
    /// from a composited file is compositing without the flag ever having been
    /// typed, so a rebuild that asked the flag would drop the slot to overdraw
    /// on the first save of any `.kir` in it — the L5 gone, every renderer back
    /// over one attachment, and any selection with it. That is `Watch::camera`'s
    /// failure in a louder register: the camera came back wrong, this comes back
    /// as a different picture entirely. `main::layering_for` is the one reading,
    /// taken once and handed to both the Set and this.
    layering: karakuri_engine::set::Layering,
    /// **Which renderer the slot is folded to**, restated on every rebuild
    /// beside the layering that makes it mean anything, and `None` for every
    /// input live.
    ///
    /// A Set is built with every input live — `mix::Input::default()` — so a
    /// rebuild that left this out would silently un-select a slot loaded from a
    /// file that recorded a selection, which is the same discard the layering
    /// above describes and is invisible in exactly the same way: the picture
    /// changes and nothing says why.
    ///
    /// **The run's own selection is not tracked here**, and that is a limit
    /// rather than an oversight: `r` moves the fold at a frame this watcher
    /// never sees, so a rebuild restates what the *slot was loaded with*. It is
    /// what the params do with a value a record has moved since, and for the
    /// same reason — a request has to be reproducible from a record stream, and
    /// what a hand did later is in the stream as a `select`.
    live: Option<u32>,
    /// `--capacity` when it was given, and otherwise `None` — each geometry
    /// then runs at the default its own `capacity` declaration names.
    ///
    /// **Not one resolved number.** A rebuild recompiles the files, so a
    /// procedure's declared default is the *new* file's, and a slot with two
    /// geometries has two of them. Resolving at startup and carrying the answer
    /// would have pinned every later build to whatever the first one declared.
    capacity: Option<u32>,
    seed_salt: u32,
    /// **One hash salt per geometry**, restated on every rebuild rather than
    /// left to be derived there — for the reason `Request::bindings` gives, and
    /// with a louder symptom: a slot filled from a Set file is running at the
    /// salts that file recorded, so a rebuild that derived its own would repaint
    /// every element in it on the next save of a `.kir`.
    ///
    /// **One resolved number each, unlike `capacity` above**, and the two differ
    /// because what they depend on differs. A capacity may come from the *new*
    /// file's declaration, so resolving it at startup would pin every later
    /// build to the first one's; a salt depends on nothing in the file at all,
    /// so the run's own numbers are the answer for as long as the run lasts.
    ///
    /// A rebuild whose files declare *more* geometries than the run started
    /// with gets a salt for the ones this knows and a derived one for the rest,
    /// which is the rule `Set::build_many` follows for a short list — the ones
    /// that were already there keep their colours, and the new one is salted
    /// like a source nobody recorded, because that is what it is.
    salts: Vec<u32>,
    /// **The six numbers the slot's camera is aimed with**, restated on every
    /// rebuild rather than left to be taken from the outgoing Set — for the
    /// reason `Request::bindings` gives, and with the symptom that outlives the
    /// run: `Set::build_many` starts every Set from `Orbit::default()`, so a
    /// slot filled by `--load-set` used to lose the camera its file recorded on
    /// the first save of any `.kir`, and the next live save wrote the defaults
    /// into a new preset. The picture came back; the file did not.
    ///
    /// **One resolved `Orbit`, on `salts`' side of the question above rather
    /// than `capacity`'s.** A capacity is `Option` because the *new* file may
    /// declare one, so there is an answer only the rebuild can know; nothing in
    /// a `.kir` declares the built-in orbit's six numbers, so the run's own are
    /// the answer for as long as the run lasts and resolving them once at
    /// startup pins nothing.
    ///
    /// **And not an `Option<Orbit>` either**, which is where this parts company
    /// with both of them: a Set holds a built-in camera whatever its files
    /// declare and it is built at exactly this default, so a slot that loaded no
    /// `camera` record carries `Orbit::default()` and states it. "Nothing
    /// loaded" and "the default loaded" are the same Set, and an `Option` here
    /// would be a distinction nothing downstream could act on.
    camera: karakuri_engine::camera::Orbit,
    overrides: Vec<karakuri_engine::ParamWrite>,
    /// The interface, restated on every rebuild for the same reason the
    /// bindings are — and the more urgent one: a `control:` binding whose
    /// control did not survive the swap holds its param where it found it.
    published: Vec<karakuri_engine::set::Published>,
    /// Restated on every rebuild rather than read off the outgoing Set, for
    /// the reason `Request::bindings` gives: a request that depended on what
    /// happened to be live would not be reproducible from a record stream.
    bindings: Vec<Binding>,
    /// **Which node fills each declared input slot**, restated on every rebuild
    /// for the reason the bindings are — and with the loudest symptom of any of
    /// them: a slot nothing binds is refused outright, so a rebuild that left
    /// these out would turn every save of a morph's `.kir` into a build that
    /// does not land, with the picture frozen at whatever startup produced.
    edges: Vec<karakuri_engine::set::Edge>,
    /// **Who may move each node the operator has spoken for**, restated on
    /// every rebuild for the reason the edges above it are: a request that
    /// depended on what happened to be live is not reproducible from a record
    /// stream. Concretely, and this is the whole of it — an operator grants a
    /// node to an agent, a `.kir` in the slot is saved, and the rebuild takes
    /// the grant back without a word. A picture that changed says so; an
    /// arrangement about who may write does not.
    ///
    /// **Empty in every run there is today, and said here rather than hidden
    /// at the request.** Nothing writes an authority into a watcher yet, and
    /// the startup path is not the thing that will: `Record::Authority` is
    /// deliberately *not* Set-file state — `setfile` and
    /// `karakuri_store::project` both drop it, because an authority is an
    /// arrangement made during a performance and a Set file carrying one would
    /// hand the node over wherever it was next loaded. So there is no
    /// `--load-set` reading to seed this from, and a `Vec::new()` spelled at
    /// the `Request` would have been the truth of today and unreachable
    /// tomorrow. It is a field, taken through [`Watch::new`] like the bindings
    /// and the edges beside it, so that the day a writer exists there is
    /// somewhere for it to write: a field that cannot be filled is worse than
    /// no field at all.
    ///
    /// **The run's own grants are not tracked here**, which is a limit on
    /// `Watch::live`'s exact terms rather than an oversight: an
    /// `Operation::SetAuthority` moves a node at a frame this watcher never
    /// sees, so a rebuild restates what the slot was *handed*, and what a hand
    /// did later is in the stream as an `authority` record.
    authorities: Vec<karakuri_engine::swap::AuthorityAt>,
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
    /// **Separate from `stored`, and not folded into it.** That one keeps what
    /// reached the *screen*; this keeps what reached the *compiler*, and the
    /// difference is the whole value — a build that was rolled back for costing
    /// too much never becomes a `Record::Procedure`, never reaches a save, and
    /// is exactly the version an operator wants back.
    snapshots: Option<karakuri_environment::history::Shared>,
}

impl Watch {
    // Fourteen, where clippy's line is seven. Every one of them is a piece of
    // one slot's identity — its files, its layering, its fold, its capacity,
    // its seed, its salts, its camera, and the values and grants it was
    // started with — and a struct to carry them would be `Watch` itself,
    // constructed one field short.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        slot: usize,
        head: crate::Named,
        rest: Vec<crate::Named>,
        layering: karakuri_engine::set::Layering,
        live: Option<u32>,
        capacity: Option<u32>,
        seed_salt: u32,
        salts: Vec<u32>,
        camera: karakuri_engine::camera::Orbit,
        overrides: Vec<karakuri_engine::ParamWrite>,
        published: Vec<karakuri_engine::set::Published>,
        bindings: Vec<Binding>,
        edges: Vec<karakuri_engine::set::Edge>,
        authorities: Vec<karakuri_engine::swap::AuthorityAt>,
    ) -> Watch {
        let mut watch = Watch {
            builds: 0,
            stored: None,
            snapshots: None,
            slot,
            head,
            rest,
            layering,
            live,
            capacity,
            seed_salt,
            salts,
            camera,
            overrides,
            published,
            bindings,
            edges,
            authorities,
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
        std::iter::once(digest(&self.head.path))
            .chain(self.rest.iter().map(|n| digest(&n.path)))
            .collect()
    }
}

impl Watch {
    /// Put what this watcher builds into `store`, and report it on `tx`.
    pub fn storing_to(
        mut self,
        store: std::sync::Arc<karakuri_store::store::Store>,
        tx: std::sync::mpsc::Sender<Built>,
    ) -> Watch {
        self.stored = Some((store, tx));
        self
    }

    /// Keep every version that compiles under `store_root`, so an edit can be
    /// walked back. See [`crate::history`].
    pub fn snapshotting_to(mut self, snapshots: karakuri_environment::history::Shared) -> Watch {
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
        let named: Vec<&crate::Named> = std::iter::once(&self.head)
            .chain(self.rest.iter())
            .collect();
        let paths: Vec<&std::path::Path> = named.iter().map(|n| n.path.as_path()).collect();
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
        for (named, src) in named.iter().zip(&srcs) {
            match compile::check(src) {
                // **With the name the slot was spelled with, and with the text
                // it compiled.** A watcher used to hand the sort bare paths, so
                // every rebuild called each node whatever its procedure
                // declared — harmless while the only thing a name did was
                // print, and not harmless once an `edge` resolves against one.
                // The source rides along for the reason [`crate::Placed`]
                // gives: everything said about a node afterwards is a function
                // of the bytes that were compiled, and the file may have moved
                // by the time anything asks.
                Ok(checked) => compiled.push((
                    (*named).clone(),
                    checked,
                    std::sync::Arc::from(src.as_str()),
                )),
                Err(report) => {
                    eprintln!(
                        "{}:\n{report}\nslot {slot} unchanged; its Set is still running",
                        named.path.display()
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
                    // **The bytes come off the node, not off a second list
                    // zipped onto it.** `placed` carries the text each file was
                    // compiled from — see [`crate::Placed`] — so what a version
                    // is filed under and what is written into it are read from
                    // one place. Zipping `srcs` back on was a second way to
                    // pair a node with its source, correct only for as long as
                    // the sort kept file order.
                    for node in &placed {
                        let layer = karakuri_environment::setfile::kind_name(node.layer);
                        let index = node.index as usize;
                        if let Err(e) =
                            snapshots.record(slot, layer, index, &node.proc, node.source.as_bytes())
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
        if let Some((store, tx)) = &self.stored {
            // **Each node put from its own carried source**, which is also what
            // its address is derived from — see [`crate::Placed::put`]. This
            // used to put `srcs` and zip the hashes back onto `placed` by
            // position, which paired a node with its bytes a second way.
            let stored: Result<Vec<_>, _> = placed
                .iter()
                .map(|node| {
                    node.put(store).map(|hash| {
                        (
                            karakuri_environment::setfile::kind_name(node.layer),
                            node.index,
                            hash,
                        )
                    })
                })
                .collect();
            match stored {
                Ok(nodes) => {
                    let _ = tx.send(Built { id, nodes });
                }
                // **Named against both readers.** A build whose sources are
                // not in the store is one a session cannot name and one a live
                // save cannot write down — and an operator who has just pressed
                // save needs to know which of those they are looking at.
                Err(e) => eprintln!(
                    "slot {slot}: this build's sources are not in the store: {e} — \
                     a replay will show the procedure it started with, and a save \
                     of this slot will write the files it started from"
                ),
            }
        }
        let crate::Material {
            l1s,
            l2s,
            l3s,
            fields,
            l4s,
            names,
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
                        l1.capacity
                            .map_or(karakuri_ir::DEFAULT_CAPACITY, |c| c.default)
                    });
                    (l1, capacity)
                })
                .collect(),
            l2s,
            l3s,
            fields,
            l4s,
            // **Restated, like every other part of a request.** A rebuild that
            // read the names off the outgoing Set would depend on what happened
            // to be live, which is the property `bindings` gives its reason for.
            //
            // The slot's own, from the paths it was spelled with — they used to
            // be dropped here, and an edge is written against them.
            names: karakuri_engine::swap::RequestNames {
                l1s: names.l1s,
                l2s: names.l2s,
                l3s: names.l3s,
                l4s: names.l4s,
                fields: names.fields,
            },
            edges: self.edges.clone(),
            layering: self.layering,
            // **The fold, restated with it.** A layering that survived a
            // rebuild and a selection that did not would be a slot that comes
            // back compositing every renderer at once — see `Watch::live`.
            live: self.live,
            seed_salt: self.seed_salt,
            // **The run's own, restated.** A rebuild is a new Set of the same
            // material, and the material is what changed — not which geometry
            // gets which randomness.
            salts: self.salts.iter().copied().map(Some).collect(),
            // **The run's own, restated**, like the salts above it and for a
            // louder reason — see `Watch::camera`. Nothing in the files this
            // watcher just recompiled says where the built-in camera is
            // pointing, so a request that left this out would hand the engine a
            // Set aimed at `Orbit::default()` however the operator loaded it.
            camera: self.camera,
            params: self.overrides.clone(),
            published: self.published.clone(),
            bindings: self.bindings.clone(),
            // **Who may move each node, restated with them.** Empty in every
            // run today, and cloned from the field rather than spelled
            // `Vec::new()` here for the reason `Watch::authorities` gives: what
            // is stated at the request is what the rebuild carries, so the day
            // a grant is handed to a watcher this is already the line that
            // stops the next save from taking it back.
            authorities: self.authorities.clone(),
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
            crate::Named::bare(dir.join("a.kir")),
            vec![crate::Named::bare(dir.join("b.kir"))],
            karakuri_engine::set::Layering::Overdraw,
            None,
            Some(4096),
            1,
            vec![1],
            karakuri_engine::camera::Orbit::default(),
            Vec::new(),
            Vec::new(),
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
            crate::Named::bare(paths[0].clone()),
            paths[1..].iter().cloned().map(crate::Named::bare).collect(),
            karakuri_engine::set::Layering::Overdraw,
            None,
            Some(4096),
            1,
            vec![1],
            karakuri_engine::camera::Orbit::default(),
            Vec::new(),
            Vec::new(),
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

    /// **A rebuild restates the salts the slot is running at**, rather than
    /// leaving them to be derived where the Set is built.
    ///
    /// A slot filled by `--load-set` runs at the salts its file recorded, and
    /// nothing in a `.kir` says what they are. A request that left them out
    /// hands `Set::build_many` an empty list, which derives from the ordinal —
    /// so every element in the slot would change colour on the next save of a
    /// file that had nothing to do with the geometry, and the Set an operator
    /// loaded would stop being the Set they loaded partway through an edit.
    #[test]
    fn a_rebuild_restates_the_salts_the_slot_is_running_at() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let tmp = tempfile::tempdir().expect("temp dir");
        // Two geometries, so that "each keeps its own" is a claim at all.
        let files = ["drift_shell.kir", "lattice_shell.kir", "soft_points.kir"];
        let paths: Vec<PathBuf> = files.iter().map(|f| tmp.path().join(f)).collect();
        // Numbers no derivation produces, so a request that derived its own
        // cannot pass by accident.
        let salts: Vec<u32> = vec![0x0bad_cafe, 0x1234_5678];
        let mut watch = Watch::new(
            0,
            crate::Named::bare(paths[0].clone()),
            paths[1..].iter().cloned().map(crate::Named::bare).collect(),
            karakuri_engine::set::Layering::Overdraw,
            None,
            Some(4096),
            salts[0],
            salts.clone(),
            karakuri_engine::camera::Orbit::default(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        );
        for (file, path) in files.iter().zip(&paths) {
            std::fs::copy(root.join("examples").join(file), path).expect("copy an example");
        }

        let request = rebuild(&mut watch).expect("a build");
        assert_eq!(request.l1s.len(), 2, "two geometries in the slot");
        assert_eq!(
            request.salts,
            vec![Some(salts[0]), Some(salts[1])],
            "a rebuild handed the engine salts other than the ones the slot is running at"
        );
    }

    /// **A rebuild restates the camera the slot is aimed with**, rather than
    /// leaving the Set it builds to start from `Orbit::default()`.
    ///
    /// Nothing in a `.kir` says where the built-in orbit is pointing — a
    /// `camera` record does, and `--load-set` is what brings one in. So a
    /// request that left this out handed `Set::build_many` a Set aimed at the
    /// defaults, and the first save of any file in the slot re-aimed a camera
    /// the operator had loaded, with nothing said. The live save then recorded
    /// `Set::camera` faithfully, which is how a wrong picture turned into a
    /// wrong file.
    #[test]
    fn a_rebuild_restates_the_camera_the_slot_is_aimed_with() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let tmp = tempfile::tempdir().expect("temp dir");
        let files = ["drift_shell.kir", "soft_points.kir"];
        let paths: Vec<PathBuf> = files.iter().map(|f| tmp.path().join(f)).collect();
        // Six numbers no default produces, so a request that let the camera be
        // re-derived cannot pass by accident.
        let aimed = karakuri_engine::camera::Orbit {
            radius: 3.25,
            speed: 0.75,
            height: -1.5,
            fov_y: 0.9,
            near: 0.25,
            far: 250.0,
        };
        let mut watch = Watch::new(
            0,
            crate::Named::bare(paths[0].clone()),
            paths[1..].iter().cloned().map(crate::Named::bare).collect(),
            karakuri_engine::set::Layering::Overdraw,
            None,
            Some(4096),
            1,
            vec![1],
            aimed,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        );
        for (file, path) in files.iter().zip(&paths) {
            std::fs::copy(root.join("examples").join(file), path).expect("copy an example");
        }

        let request = rebuild(&mut watch).expect("a build");
        // All six, not the two a `camera` record spells: a rebuild that carried
        // half of them would be as wrong as one that carried none, and quieter.
        let six = |o: &karakuri_engine::camera::Orbit| {
            (o.radius, o.speed, o.height, o.fov_y, o.near, o.far)
        };
        assert_eq!(
            six(&request.camera),
            six(&aimed),
            "a rebuild asked for a Set aimed somewhere other than where the slot is aimed"
        );
    }

    /// **A rebuild restates the layering and the fold the slot was loaded
    /// with**, rather than leaving either to be worked out again.
    ///
    /// A Set file records both — a `merge` record and its `live` — so a slot
    /// filled by `--load-set` can be compositing, and folded to one renderer,
    /// without `--merge` ever having been typed. A rebuild that re-derived the
    /// layering from the flag would drop the slot to overdraw on the first save
    /// of any `.kir` in it: the L5 gone, every renderer back over one
    /// attachment, and the selection with it. That is `Watch::camera`'s failure
    /// in a register where the picture does not come back — and a request that
    /// carried the layering but not the fold would be half of it, a slot
    /// compositing every alternative at once.
    ///
    /// Both are asserted from one rebuild, because both travel on one request
    /// and either alone is not the Set that was loaded.
    #[test]
    fn a_rebuild_restates_the_layering_and_the_fold_the_slot_was_loaded_with() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let tmp = tempfile::tempdir().expect("temp dir");
        // Two renderers over one geometry, which is what there has to be for a
        // fold to be a choice at all.
        let files = ["drift_shell.kir", "soft_points.kir", "soft_points.kir"];
        let paths: Vec<PathBuf> = (0..files.len())
            .map(|at| tmp.path().join(format!("{at}.kir")))
            .collect();
        let mut watch = Watch::new(
            0,
            crate::Named::bare(paths[0].clone()),
            paths[1..].iter().cloned().map(crate::Named::bare).collect(),
            // What a composited Set file loads as — no flag was typed here,
            // which is the whole point.
            karakuri_engine::set::Layering::Composite,
            Some(1),
            Some(4096),
            1,
            vec![1],
            karakuri_engine::camera::Orbit::default(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        );
        for (file, path) in files.iter().zip(&paths) {
            std::fs::copy(root.join("examples").join(file), path).expect("copy an example");
        }

        let request = rebuild(&mut watch).expect("a build");
        assert_eq!(
            request.layering,
            karakuri_engine::set::Layering::Composite,
            "a rebuild asked for a Set that overdraws, and the slot was compositing"
        );
        assert_eq!(
            request.live,
            Some(1),
            "a rebuild asked for a Set folding every renderer, and the slot was folded to one"
        );
    }

    /// **A rebuild restates the names and the edges the slot is wired with.**
    ///
    /// A watcher used to be handed bare paths, so every rebuild called each node
    /// whatever its procedure declared — harmless while a name only printed. It
    /// stops being harmless twice over now: an `edge` names the node that
    /// declares a slot and the node bound to it, so a rebuild that dropped the
    /// names resolves against spellings that are no longer there, and one that
    /// dropped the edges leaves the slot unbound — which is refused outright.
    /// Either way the save that lands is a build that will not build, with the
    /// picture frozen at whatever startup produced.
    #[test]
    fn a_rebuild_restates_the_names_and_edges_the_slot_is_wired_with() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let tmp = tempfile::tempdir().expect("temp dir");
        let files = [
            "lattice_shell.kir",
            "sphere_shell.kir",
            "morph.kir",
            "soft_points.kir",
        ];
        let paths: Vec<PathBuf> = files.iter().map(|f| tmp.path().join(f)).collect();
        // Named as the command line names them, with the far geometry carrying
        // the name the edge points with.
        let named: Vec<crate::Named> = paths
            .iter()
            .zip(["near", "far", "morph", "draw"])
            .map(|(path, name)| crate::Named {
                name: Some(name.to_string()),
                path: path.clone(),
            })
            .collect();
        let edges = vec![karakuri_engine::set::Edge {
            node: "morph".to_string(),
            slot: "far".to_string(),
            to: "far".to_string(),
        }];
        let mut watch = Watch::new(
            0,
            named[0].clone(),
            named[1..].to_vec(),
            karakuri_engine::set::Layering::Overdraw,
            None,
            Some(32768),
            1,
            vec![1, 2],
            karakuri_engine::camera::Orbit::default(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            edges.clone(),
            Vec::new(),
        );
        for (file, path) in files.iter().zip(&paths) {
            std::fs::copy(root.join("examples").join(file), path).expect("copy an example");
        }

        let request = rebuild(&mut watch).expect("a build");
        assert_eq!(
            request.names.l1s,
            vec![Some("near".to_string()), Some("far".to_string())],
            "a rebuild renamed the geometries the edge points at"
        );
        assert_eq!(request.names.l2s, vec![Some("morph".to_string())]);
        assert_eq!(
            request.edges, edges,
            "a rebuild dropped the wiring, so the slot it rebuilds is unbound"
        );
    }

    /// **A rebuild restates the authority the slot's nodes were handed.**
    ///
    /// The failure this is against is the one every field beside it is against,
    /// and it is the quietest of them: an operator grants a node to an agent,
    /// somebody saves a `.kir` in that slot, and the rebuild hands back a Set
    /// where every node is `Authority::Manual` again — the default, and the
    /// only default it could be. Nothing refuses it, because a rebuild is not a
    /// surface, and nothing looks wrong, because an arrangement about who may
    /// write a param does not draw. The grant is simply gone.
    ///
    /// **Asserted over a request rather than over a Set**, which is as far as
    /// this side goes: applying the list is `HotSwap`'s, tested where it lives,
    /// and reaching a built Set from here would want a device. What is this
    /// watcher's to get wrong is whether the list survives the rebuild at all,
    /// and that is exactly what this reads.
    ///
    /// **Two levels and two layers, neither of them the default.** A test that
    /// granted one node `Manual` would pass against a request that dropped the
    /// list entirely, since that is where the build would land anyway.
    #[test]
    fn a_rebuild_restates_the_authority_the_slots_nodes_were_handed() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let tmp = tempfile::tempdir().expect("temp dir");
        // Two geometries and a renderer, so that a wrong index and a wrong
        // layer are both failures this can see.
        let files = ["drift_shell.kir", "lattice_shell.kir", "soft_points.kir"];
        let paths: Vec<PathBuf> = files.iter().map(|f| tmp.path().join(f)).collect();
        let authorities = vec![
            karakuri_engine::swap::AuthorityAt::new(
                karakuri_ir::Kind::L1,
                1,
                karakuri_engine::set::Authority::Automatic,
            ),
            karakuri_engine::swap::AuthorityAt::new(
                karakuri_ir::Kind::L4,
                0,
                karakuri_engine::set::Authority::Suggesting,
            ),
        ];
        let mut watch = Watch::new(
            0,
            crate::Named::bare(paths[0].clone()),
            paths[1..].iter().cloned().map(crate::Named::bare).collect(),
            karakuri_engine::set::Layering::Overdraw,
            None,
            Some(4096),
            1,
            vec![1, 2],
            karakuri_engine::camera::Orbit::default(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            authorities.clone(),
        );
        for (file, path) in files.iter().zip(&paths) {
            std::fs::copy(root.join("examples").join(file), path).expect("copy an example");
        }

        let request = rebuild(&mut watch).expect("a build");
        assert_eq!(
            request.authorities, authorities,
            "a rebuild dropped the grants, so the next save takes back every node \
             an operator gave away"
        );
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
        let mut watch = watch.storing_to(store, tx);
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
        assert_eq!(procs(&request.l3s), procs(&material.l3s), "the cameras");
        assert_eq!(
            procs(&request.fields),
            procs(&material.fields),
            "the fields"
        );

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
            .map(|node| {
                (
                    karakuri_environment::setfile::kind_name(node.layer),
                    node.index,
                )
            })
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
        // A stack that assembles, and then one that cannot — so this fails if
        // the refusal stopped happening *and* if it started happening to
        // everything. **Two cameras is the first kind and used to be the
        // second**: a slot holds as many as its files declare, and which
        // renderer draws from which is an `edge`.
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
                true,
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
                crate::Named::bare(paths[0].clone()),
                paths[1..].iter().cloned().map(crate::Named::bare).collect(),
                karakuri_engine::set::Layering::Overdraw,
                None,
                Some(4096),
                1,
                vec![1],
                karakuri_engine::camera::Orbit::default(),
                Vec::new(),
                Vec::new(),
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
