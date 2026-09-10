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
//! **And no run gives two slots the same files any more.** That sentence is a
//! statement about this type, which is handed paths and believes them; it was
//! read for a while as a licence, and both programs spent it — every slot of
//! `crates/karakuri` watched the two paths the operator typed, so one save
//! rebuilt four slots. [`crate::scratch`] now copies per slot, and its header
//! carries the argument. What is left here is what was always true: given one
//! file twice, this rebuilds twice, and the unit that gets replaced is still
//! the slot.
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
//! ## A compile that fails
//!
//! It prints every diagnostic it has and answers
//! [`Polled::Refused`](karakuri_engine::swap::Polled::Refused), carrying one
//! line per diagnostic. Nothing is requested, so nothing is built, so nothing
//! is swapped — the running Set keeps running with its `t` and its element
//! buffers untouched — and nothing is filed either: a version is gated on
//! compiling, so a refusal is not one.
//!
//! **It answered `None` until 2026-09-08**, which is the same answer a poll
//! that saw no edit gives, so the refusal reached the terminal and no surface
//! in the instrument at all
//! (`docs/adr/0310-a-source-can-say-it-refused-and-the-lane-draws-it.md`). The
//! two other ways a rebuild can come to nothing — a file that will not read,
//! and a stack that will not sort into a Set — still print and answer `None`;
//! that record says why the word this carries is the checker's and not theirs.

use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::time::Duration;

use karakuri_engine::swap::{Polled, Refusal};
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

/// **What one slot is pointed at**, handed to a running [`Watch`] over
/// [`Watch::aimed_by`]'s channel.
///
/// # It is every field of the slot's identity, and that is the point
///
/// It is [`Watch::new`]'s argument list less the slot, plus the Set the slot
/// is running — which arrives at construction through
/// [`Watch::snapshotting_to`] rather than through `new`, because it is the
/// history's and only the history's. Every other field carries its argument's
/// reason and is read there, because they are the same fields and a second
/// copy of them would be as many places to get one of them wrong.
/// **Anything left out comes back as the outgoing slot's**, and
/// the symptoms are the ones those fields are documented against: a fold that
/// silently un-selects, a camera that reverts to `Orbit::default()`, salts
/// that repaint every element. Worse, none of them shows on the *load* — the
/// aim states them, so the load is right and the first later save is what goes
/// wrong.
///
/// **The slot is not here.** A watcher that could be re-pointed at a different
/// slot would rebuild somebody else's deck, and the one thing enforcing *this
/// watcher rebuilds this slot* is that it cannot see another one's files.
///
/// # Where the values come from
///
/// A Set file, read by [`crate::setfile::load`], which answers with every one
/// of these except the authorities: `Record::Authority` is deliberately not
/// Set-file state — an authority is an arrangement made during a performance,
/// and a Set file carrying one would hand the node over wherever it was next
/// loaded. So a load starts a slot with the grants a `--load-set` starts one
/// with, which is none, and the field is here rather than assumed so the
/// sender is the one saying it.
pub struct Aim {
    /// The file the slot is now spelled with — see [`Watch::head`].
    pub head: crate::compile::Named,
    /// The rest of them, in chain order — see [`Watch::rest`].
    pub rest: Vec<crate::compile::Named>,
    pub layering: karakuri_engine::set::Layering,
    pub live: Option<u32>,
    pub capacity: Option<u32>,
    pub seed_salt: u32,
    pub salts: Vec<u32>,
    pub camera: karakuri_engine::camera::Orbit,
    pub overrides: Vec<karakuri_engine::ParamWrite>,
    pub published: Vec<karakuri_engine::set::Published>,
    pub bindings: Vec<Binding>,
    pub edges: Vec<karakuri_engine::set::Edge>,
    pub authorities: Vec<karakuri_engine::swap::AuthorityAt>,
    /// **The Set this slot is now running**, which is what every version it
    /// writes from here on is filed under — or `None` where it is running
    /// material no Set names, which is a pair somebody typed.
    ///
    /// **It is here because a library load is the thing that moves it.** A
    /// load re-points a watcher at the files of a different Set, and the id is
    /// as much a part of *what this slot is now* as the files are: a re-point
    /// that left it behind would file every later version under the Set before
    /// the load, silently and in a name nothing reads back
    /// ([`crate::history::Snapshots::record`] carries that argument, and
    /// `ADR-0276` is the record). It is one field of this rather than a value
    /// a surface keeps beside its deck, so that the answer to *what is this
    /// slot running* moves with the re-point that changes it and is restated
    /// by every re-aim, like every field above it.
    ///
    /// A watcher keeping no history has nothing to file, so this is dropped on
    /// arrival there — see [`Watch::snapshots`], which is its only holder.
    pub set: Option<String>,
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
    head: crate::compile::Named,
    /// The rest of the slot's files, in the order they were spelled. Watched
    /// together with the head: a rebuild restates the whole stack, so an edit
    /// to any one of them recompiles all of them and the Set that lands is the
    /// one the files say.
    rest: Vec<crate::compile::Named>,
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
    /// never sees, so a rebuild restates what the *slot was loaded with*, and
    /// the reason is a request's: it has to be reproducible from a record
    /// stream, and what a hand did later is in the stream as a `select`.
    ///
    /// **The params used to be the example of this and are no longer**: a value
    /// a hand moved is carried across the swap by the Set itself
    /// (`docs/adr/0282-a-rebuild-inherits-the-values-somebody-moved-and-reads-the-rest-from-the-code.md`),
    /// because `Set` knows which of its values somebody stated and nothing here
    /// knows which renderer `r` last folded to.
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
    /// **The values this slot was aimed with**, stated on the build the aim
    /// causes and on no other — see [`Watch::stated`] beside it.
    ///
    /// **It used to be restated on every rebuild**, on the terms the salts, the
    /// camera and the bindings still are, and that is what made a knob
    /// unturnable: a slot pointed at a Set file carries *every declaration of
    /// every node* here, because a live save writes them all, so restating them
    /// put every parameter back to the file on the next save of any `.kir` and
    /// there was nothing an operator could do to a live Set that survived it.
    /// What replaces the restatement is
    /// [`karakuri_engine::Set::carry_moved_from`]: the values somebody stated
    /// travel across the swap on the Set itself, which is where they already
    /// were.
    ///
    /// **The aim's own build still states them, and has to.** At that moment
    /// nothing has applied them to anything — the aim is a description of a Set
    /// that is not built yet — so there is no live Set to inherit them from, and
    /// the same clause is what makes a *load* beat a ride: a value this build
    /// states is this build's answer for that key. See `carry_moved_from`.
    overrides: Vec<karakuri_engine::ParamWrite>,
    /// **Whether the overrides above have been applied to the Set this slot is
    /// running.** `true` from [`Watch::new`], because the caller built that Set
    /// with them and handed it over live; `false` from [`Watch::repointed`],
    /// because an aim describes a Set nothing has built yet.
    ///
    /// A `bool` and not a count: what it decides is whether *this* build is the
    /// one that has to state the values, and there is exactly one such build per
    /// aim.
    stated: bool,
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
    /// **Where a re-point arrives**, and `None` for a watcher nobody can
    /// re-point — every offscreen path.
    ///
    /// **`karakuri-cli`'s watched slots used to be in that list and are not
    /// any more.** They were, for as long as nothing on that surface could
    /// change what a slot is wired with; `wire_input` can, and an edge is not
    /// a file, so the only way it reaches a build worker is an aim — see
    /// `main::Aiming`. A slot that surface does *not* watch still has no
    /// watcher at all, which is a different thing from a watcher that cannot
    /// be re-pointed.
    ///
    /// A channel rather than a shared cell for the reason this module polls
    /// rather than taking `notify`: the worker is a poll loop already, so
    /// there is nothing to wake and nothing to lock. It is also what makes a
    /// re-point *safe* to offer at all — the render thread hands over a
    /// description and touches nothing this watcher owns, so a load costs the
    /// frame it happens on a `send` and no more.
    aimed: Option<Receiver<Aim>>,
    /// Where every version that compiled is kept, so an edit can be undone,
    /// **and the Set this slot is running**, which is what a version is filed
    /// under. `None` when no store root was given, which is the offscreen
    /// paths.
    ///
    /// **Separate from `stored`, and not folded into it.** That one keeps what
    /// reached the *screen*; this keeps what reached the *compiler*, and the
    /// difference is the whole value — a build that compiled and never reached
    /// a slot, because a newer one superseded it before the boundary, becomes
    /// no `Record::Procedure` and reaches no save, and the version *before* the
    /// one that stopped a slot for cost is exactly what an operator goes
    /// looking for (ADR-0316).
    ///
    /// **The id is inside the pair rather than a field beside it**, because
    /// this is its only reader: a slot keeping no history has nothing to file
    /// under, and a second field that meant nothing whenever this was `None`
    /// would be a second answer to *what is this slot running* with nobody
    /// asking it. `None` inside the pair is a slot running material no Set
    /// names — a pair on the command line — which
    /// [`crate::history::Snapshots::record`] argues is a state rather than a
    /// missing answer.
    ///
    /// **It is what the slot was running when the watcher was built, until a
    /// re-point says otherwise**, and [`Watch::repointed`] is where it moves:
    /// [`Aim::set`] is a field of an aim like the files are, so a library load
    /// carries the Set it is loading and the versions written after it are
    /// filed under that Set rather than under the one before it. There is one
    /// holder of the answer here and the destructuring that moves it has no
    /// `..`, so a re-point cannot leave it behind.
    snapshots: Option<(crate::history::Shared, Option<String>)>,
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
        head: crate::compile::Named,
        rest: Vec<crate::compile::Named>,
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
            aimed: None,
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
            // **Already applied**, which is what a constructed watcher is: every
            // caller builds the slot's first Set with these values and hands it
            // to `HotSwap::new` as the live one. Stating them again on the first
            // save would be this build answering for a key the operator may
            // have ridden since, which is exactly the clause in
            // `Set::carry_moved_from` that makes a load beat a ride — and this
            // is not a load.
            stated: true,
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
    ///
    /// `set` is the Set this slot is running **when the watcher is built**, or
    /// `None` where it is running material no Set names. It is taken here
    /// rather than in [`Watch::new`] because it is the history's and only the
    /// history's — see [`Watch::snapshots`]. **A later library load moves it**,
    /// on the aim that re-points this watcher ([`Aim::set`]), so this is the
    /// first answer rather than the only one.
    pub fn snapshotting_to(
        mut self,
        snapshots: crate::history::Shared,
        set: Option<String>,
    ) -> Watch {
        self.snapshots = Some((snapshots, set));
        self
    }

    /// **Let whoever holds the other end of `rx` point this watcher at
    /// different material**, which is how a Set reaches a *running* deck.
    ///
    /// # It is a re-point and deliberately not an install
    ///
    /// `karakuri_engine::deck::Deck::install` is the one function that puts a
    /// built Set in a slot, and it says of itself that it is *"deliberately
    /// not reachable from a key or a surface: a live run changes its material
    /// by editing a file and letting the worker build it, which is what the
    /// budget watchdog is attached to."* A surface that built a Set and handed
    /// it over would be putting material on air that **nothing measured**, in
    /// a slot the watchdog never got to judge — the two things `HotSwap` exists
    /// to guarantee.
    ///
    /// So a load says *look at these files instead* and lets go. Everything
    /// after that is the path an edit already takes: compiled on this thread,
    /// offered on the same channel, swapped at a frame boundary (P-0094),
    /// judged on that Set's own measured frame against the budget (ADR-0313),
    /// and left in the slot with the slot stopped if it costs too much
    /// (ADR-0316). **The library gets the watchdog for nothing**, and no second
    /// route into a slot is opened.
    ///
    /// # What the sender owes
    ///
    /// Files. This watcher reads paths and never a store
    /// ([`Source::poll`]), so a Set whose sources are content-addressed blobs
    /// has to be written out where the watcher can read it before the aim is
    /// sent — which is [`crate::scratch::place`], and is exactly what
    /// `--load-set` does at startup for the same watcher.
    pub fn aimed_by(mut self, rx: Receiver<Aim>) -> Watch {
        self.aimed = Some(rx);
        self
    }
}

impl Source for Watch {
    fn poll(&mut self) -> Option<Polled> {
        std::thread::sleep(INTERVAL);

        // **A re-point is acted on the poll it is seen, and a save is not.**
        // The debounce below exists because an editor writing in place leaves
        // a file truncated for a moment, so a change is trusted only once the
        // bytes have stopped moving. An aim has no such moment: whoever sent
        // it wrote the whole of every file before it went on the channel, and
        // waiting a second interval would only make a load slower than a save
        // for no reason at all. So this returns a build straight away, and the
        // stamps it re-seeds are what stop the new files from being seen as an
        // edit on the next poll.
        if self.repointed() {
            return self.rebuild();
        }

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
        self.rebuild()
    }
}

impl Watch {
    /// **Take the newest aim off the channel, if one is waiting**, and answer
    /// whether this watcher is now pointed somewhere else.
    ///
    /// **The newest and not the next**: the channel is drained to its end and
    /// only the last aim is taken. Two loads pressed inside one poll interval
    /// are one load as far as the picture is concerned — building the first of
    /// them would put a Set on screen that the operator has already replaced,
    /// and it would cost a compile to do it. That is the debounce's own
    /// reasoning with the roles swapped: there, a run of writes is one edit;
    /// here, a run of presses is one destination.
    ///
    /// `None` on the channel is a watcher nobody can re-point, which is every
    /// offscreen path and `karakuri-cli`'s own slots; a *closed* channel is a
    /// host that has gone, and reads exactly like nothing waiting.
    fn repointed(&mut self) -> bool {
        let Some(rx) = &self.aimed else {
            return false;
        };
        let mut aim = None;
        while let Ok(next) = rx.try_recv() {
            aim = Some(next);
        }
        let Some(aim) = aim else {
            return false;
        };
        let Aim {
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
            set,
        } = aim;
        // **Every field `Watch::new` takes, plus the Set the slot is running,
        // and the compiler is what says so: the destructuring above has no
        // `..`.** A re-point that left one of
        // them behind is the failure each of those fields is documented
        // against — a slot that comes back at the wrong capacity, in the wrong
        // fold, under a camera the file never named — and it would show up on
        // the *first save after the load* rather than on the load, which is
        // the hardest version of it to find. The slot itself is the one thing
        // an aim cannot carry: a watcher that changed slots would rebuild
        // somebody else's deck.
        self.head = head;
        self.rest = rest;
        self.layering = layering;
        self.live = live;
        self.capacity = capacity;
        self.seed_salt = seed_salt;
        self.salts = salts;
        self.camera = camera;
        self.overrides = overrides;
        // **Not yet applied to anything.** An aim describes a Set that does not
        // exist, so the build below is the one that has to state its values —
        // and stating them is what makes the loaded Set's numbers beat whatever
        // the outgoing Set was carrying. See `Watch::stated`.
        self.stated = false;
        self.published = published;
        self.bindings = bindings;
        self.edges = edges;
        self.authorities = authorities;
        // **What every version written from here on is filed under.** A
        // re-point is how a Set reaches a running deck, so the id is as much
        // part of *what this slot is now* as the files are — left behind, the
        // loaded Set's whole chain would be filed under the Set before it, in
        // names nothing reads back and with nothing to say it happened.
        //
        // **Dropped where there is no history**, which is every offscreen path
        // and every harness with no store: there is nothing to file under, and
        // keeping the answer somewhere a rebuild never reads is the second
        // holder `Watch::snapshots` argues against.
        if let Some((_, running)) = &mut self.snapshots {
            *running = set;
        }
        // Seeded from the new files, exactly as `Watch::new` seeds from the
        // ones it was constructed with: the build below is this material's
        // first, so the next poll must not see it as a second one.
        self.stamps = self.stamp();
        self.settling = false;
        true
    }

    /// **Compile what this watcher is pointed at and ask for it**, or `None`
    /// where the files would not read, would not check, or would not assemble.
    ///
    /// Split out of [`Source::poll`] when a re-point became the second way in:
    /// a save reaches it through the debounce and an aim reaches it at once,
    /// and everything after that decision is the same work. Two copies of it
    /// would be two answers to *what does a rebuild restate*, which is the
    /// question every field on this struct is documented against.
    fn rebuild(&mut self) -> Option<Polled> {
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
        let named: Vec<&crate::compile::Named> = std::iter::once(&self.head)
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
            match compile::diagnose(src) {
                // **With the name the slot was spelled with, and with the text
                // it compiled.** A watcher used to hand the sort bare paths, so
                // every rebuild called each node whatever its procedure
                // declared — harmless while the only thing a name did was
                // print, and not harmless once an `edge` resolves against one.
                // The source rides along for the reason [`crate::compile::Placed`]
                // gives: everything said about a node afterwards is a function
                // of the bytes that were compiled, and the file may have moved
                // by the time anything asks.
                Ok(checked) => compiled.push((
                    (*named).clone(),
                    checked,
                    std::sync::Arc::from(src.as_str()),
                )),
                // **Said on the terminal and said to the deck.** The line below
                // is the one this has always printed and it stays — a terminal
                // is a surface too, and it is the only one a headless run has.
                // What is new beside it is that the refusal now *reaches* the
                // instrument: a `.kir` the checker turns down used to answer
                // `None`, which is the same answer as "nothing happened", so
                // the operator's newest edit disagreeing with what is on screen
                // was said on a terminal nobody watches mid-set
                // (`docs/adr/0310-a-source-can-say-it-refused-and-the-lane-draws-it.md`).
                //
                // **Nothing is filed, and nothing here has to undo that.**
                // The store put and the history snapshot are both below this
                // line, so a refusal reaches neither: nothing compiled, so
                // there is no version, which is the gate the edit history is
                // already on (ADR-0089). The slot's last good version is still
                // the newest thing filed and still the thing on screen.
                Err(diagnostics) => {
                    eprintln!(
                        "{}:\n{diagnostics}\nslot {slot} unchanged; its Set is still running",
                        named.path.display()
                    );
                    return Some(Polled::Refused(Refusal {
                        // **The name the slot spelled this node with**, and
                        // the file's own where it was spelled bare — not a
                        // build's label, because there is no build and the
                        // `proc` name is the very thing a file that will not
                        // parse has not got. It is the one word that says
                        // which file to open.
                        label: named.name.clone().unwrap_or_else(|| {
                            named
                                .path
                                .file_name()
                                .map(|f| f.to_string_lossy().into_owned())
                                .unwrap_or_default()
                        }),
                        said: diagnostics.said,
                    }));
                }
            }
        }
        // **Sorted by the `kind` each file declares, in the same code the
        // startup path sorts with** — see [`crate::compile::sort_compiled`], which says
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
        let (material, placed) = match crate::compile::sort_compiled(compiled) {
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
        if let Some((snapshots, set)) = &self.snapshots {
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
                    // compiled from — see [`crate::compile::Placed`] — so what a version
                    // is filed under and what is written into it are read from
                    // one place. Zipping `srcs` back on was a second way to
                    // pair a node with its source, correct only for as long as
                    // the sort kept file order.
                    for node in &placed {
                        let layer = crate::setfile::kind_name(node.layer);
                        let index = node.index as usize;
                        // **The Set the slot is running, filed with the
                        // version.** The write is addressed by slot and this
                        // watcher is one slot's, so which Set it is a version of
                        // is a thing in hand at the moment of the write rather
                        // than something a reader has to work out later — which
                        // it cannot, the layout being names and nothing else.
                        if let Err(e) = snapshots.record(
                            slot,
                            layer,
                            index,
                            set.as_deref(),
                            &node.proc,
                            node.source.as_bytes(),
                        ) {
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
            // its address is derived from — see [`crate::compile::Placed::put`]. This
            // used to put `srcs` and zip the hashes back onto `placed` by
            // position, which paired a node with its bytes a second way.
            let stored: Result<Vec<_>, _> = placed
                .iter()
                .map(|node| {
                    node.put(store)
                        .map(|hash| (crate::setfile::kind_name(node.layer), node.index, hash))
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
        let crate::compile::Material {
            l1s,
            l2s,
            l3s,
            fields,
            l4s,
            names,
        } = material;
        // **Taken rather than read**, so that the aim's values are stated once
        // and the flag cannot be left true by a path that returns early: every
        // `None` above this line is a build that did not happen, and the values
        // are still owed to the one that does. See `Watch::stated`.
        let params = if std::mem::replace(&mut self.stated, true) {
            Vec::new()
        } else {
            self.overrides.clone()
        };
        Some(Polled::Build(Request {
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
            // **The aim's values on the aim's own build, and nothing after
            // it.** Everything else on this request is restated because the
            // engine would otherwise re-derive it; parameter values are the one
            // thing the outgoing Set already holds and can hand over, which is
            // what `Set::carry_moved_from` does at the install. Restating them
            // here as well would pin every one of them to what the slot was
            // aimed with and make a ride impossible to keep — see
            // `Watch::overrides`.
            params,
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
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn watch_on(dir: &std::path::Path) -> Watch {
        Watch::new(
            0,
            crate::compile::Named::bare(dir.join("a.kir")),
            vec![crate::compile::Named::bare(dir.join("b.kir"))],
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
            crate::compile::Named::bare(paths[0].clone()),
            paths[1..]
                .iter()
                .cloned()
                .map(crate::compile::Named::bare)
                .collect(),
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

    /// Polled until it answers, or four intervals, whichever is first. One
    /// poll sees the change and the next acts on it — see "Debouncing" — so
    /// nothing by the fourth is nothing at all.
    fn polled(watch: &mut Watch) -> Option<Polled> {
        std::iter::repeat_with(|| watch.poll())
            .take(4)
            .flatten()
            .next()
    }

    /// [`polled`], for the tests that are about what a build states.
    ///
    /// **A refusal panics with its diagnostics rather than reading as
    /// nothing.** A watcher that stopped compiling would otherwise turn every
    /// assertion below into `expect("a build")` on a `None`, which says the
    /// files never settled and is the wrong end of the failure entirely.
    fn rebuild(watch: &mut Watch) -> Option<Request> {
        match polled(watch)? {
            Polled::Build(request) => Some(request),
            Polled::Refused(refusal) => panic!(
                "the checker turned `{}` down: {}",
                refusal.label,
                refusal.said.join("; ")
            ),
        }
    }

    /// **A `.kir` the checker turns down is an answer and not a silence**, and
    /// the answer carries every diagnostic the checker had.
    ///
    /// This watcher printed its diagnostics and returned `None` until
    /// 2026-09-08, and `None` is what a poll that saw nothing returns — so the
    /// operator's newest edit disagreeing with the picture was said on a
    /// terminal and reached no surface at all
    /// (`docs/adr/0310-…`). The assertion is
    /// therefore about the *shape* of the answer first and its contents
    /// second: a refusal, with the file it is about and with what the checker
    /// said in it.
    ///
    /// **The negative control is the same watcher afterwards.** A version that
    /// refused everything would pass every assertion above the repair; the
    /// repair is what says the refusal was about the bytes.
    ///
    /// **The fixture is prepended to rather than substituted in.** Both files
    /// are ones this product can rewrite — `write_procedure` reaches them over
    /// MCP — so what is written here has to break them whatever they contain,
    /// which a line that is not a declaration does and a substitution does not
    /// (`docs/contributing.md` §3).
    #[test]
    fn a_file_the_checker_turns_down_is_a_refusal_carrying_its_diagnostics() {
        let tmp = tempfile::tempdir().expect("temp dir");
        let (mut watch, paths) = watch_over(tmp.path(), &["drift_shell.kir", "soft_points.kir"]);
        rebuild(&mut watch).expect("the examples this slot was pointed at compile");

        let good = std::fs::read_to_string(&paths[0]).expect("read the head");
        std::fs::write(&paths[0], format!("not a declaration\n{good}")).expect("break the head");
        let refusal = match polled(&mut watch) {
            Some(Polled::Refused(refusal)) => refusal,
            Some(Polled::Build(request)) => {
                panic!(
                    "a file that does not check was built as `{}`",
                    request.label
                )
            }
            None => panic!(
                "a file that does not check produced nothing at all, which is what a poll \
                 that saw no edit produces"
            ),
        };
        assert!(
            refusal.label.contains("drift_shell"),
            "the refusal names `{}` and the file that does not check is drift_shell.kir",
            refusal.label
        );
        assert!(
            !refusal.said.is_empty(),
            "a refusal with nothing in it is the silence this replaced"
        );
        // Where, which stage, and what — the head of what the terminal is
        // printing, which is `compile::Diagnostics::said`'s own claim.
        assert!(
            refusal.said[0].starts_with("1:1: parse: "),
            "the first diagnostic is `{}` and the broken line is the first one",
            refusal.said[0]
        );
        assert!(
            refusal.said.iter().all(|line| !line.contains('\n')),
            "a diagnostic on this list is a line, and one of these is a block: {:?}",
            refusal.said
        );

        // **The negative control.** The same watcher, the same slot, the file
        // as it was: a build, which is what says the refusal above was the
        // bytes and not this watcher having given up.
        std::fs::write(&paths[0], &good).expect("repair the head");
        let request = rebuild(&mut watch).expect("the repaired file compiles");
        assert!(
            request.label.contains("drift_shell"),
            "the repaired slot built `{}`",
            request.label
        );
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
            crate::compile::Named::bare(paths[0].clone()),
            paths[1..]
                .iter()
                .cloned()
                .map(crate::compile::Named::bare)
                .collect(),
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

    /// **The values a slot was aimed with are stated once, and a later rebuild
    /// states none of them** — which is the whole of what
    /// `karakuri_engine::Set::carry_moved_from` needed from this side.
    ///
    /// It is the one field on a request that stopped being restated, and the
    /// reason is that it is the one the engine does not re-derive: a salt, a
    /// camera, a fold and an edge all come back at some default the moment a
    /// request stops naming them, and a parameter value does not — the outgoing
    /// Set is holding it. Restating them made a knob unturnable, and worst
    /// exactly where an operator is most likely to be turning one: a slot
    /// pointed at a Set file carries **every declaration of every node** here,
    /// because that is what a live save writes, so every parameter went back to
    /// the file on the next save of any `.kir` and nothing an operator did to a
    /// live Set survived it.
    ///
    /// **Both halves are asserted from one watcher**, because either alone is
    /// the wrong rule: a watcher that never stated them would build the aimed
    /// Set without the values it was aimed with, and one that always stated them
    /// is what this replaces.
    #[test]
    fn the_values_a_slot_was_aimed_with_are_stated_once() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let tmp = tempfile::tempdir().expect("temp dir");
        let files = ["drift_shell.kir", "soft_points.kir"];
        let paths: Vec<PathBuf> = files.iter().map(|f| tmp.path().join(f)).collect();
        for (file, path) in files.iter().zip(&paths) {
            std::fs::copy(root.join("examples").join(file), path).expect("copy an example");
        }
        let named = |at: usize| crate::compile::Named::bare(paths[at].clone());
        let aimed_with = vec![karakuri_engine::ParamWrite::everywhere("exposure", 0.125)];

        let (aim, aimed) = std::sync::mpsc::channel();
        let mut watch = Watch::new(
            0,
            named(0),
            vec![named(1)],
            karakuri_engine::set::Layering::Overdraw,
            None,
            Some(4096),
            1,
            vec![1],
            karakuri_engine::camera::Orbit::default(),
            // **What this watcher was constructed with, and it is never
            // stated**: the caller built the live Set with these and handed it
            // over, so the first save owes the operator whatever is on the Set
            // rather than whatever the flags said before the run started.
            aimed_with.clone(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )
        .aimed_by(aimed);

        std::fs::write(&paths[0], format!("// a save\n{}", read(&paths[0]))).expect("edit");
        let request = rebuild(&mut watch).expect("a build");
        assert!(
            request.params.is_empty(),
            "a save restated the values the run was started with, so a knob ridden \
             since could not survive it"
        );

        aim.send(Aim {
            head: named(0),
            rest: vec![named(1)],
            layering: karakuri_engine::set::Layering::Overdraw,
            live: None,
            capacity: Some(4096),
            seed_salt: 1,
            salts: vec![1],
            camera: karakuri_engine::camera::Orbit::default(),
            overrides: aimed_with.clone(),
            published: Vec::new(),
            bindings: Vec::new(),
            edges: Vec::new(),
            authorities: Vec::new(),
            set: None,
        })
        .expect("the watcher is alive");
        let request = rebuild(&mut watch).expect("the aim's own build");
        assert_eq!(
            request.params.len(),
            1,
            "the aim's own build has nothing to inherit from — the Set it describes \
             does not exist yet — so it is the one build that has to state them"
        );

        std::fs::write(&paths[0], format!("// another save\n{}", read(&paths[0]))).expect("edit");
        let request = rebuild(&mut watch).expect("a build");
        assert!(
            request.params.is_empty(),
            "the aim's values were stated a second time, which pins every one of \
             them to the file for the rest of the run"
        );
    }

    fn read(path: &std::path::Path) -> String {
        std::fs::read_to_string(path).expect("read back")
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
            crate::compile::Named::bare(paths[0].clone()),
            paths[1..]
                .iter()
                .cloned()
                .map(crate::compile::Named::bare)
                .collect(),
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
            crate::compile::Named::bare(paths[0].clone()),
            paths[1..]
                .iter()
                .cloned()
                .map(crate::compile::Named::bare)
                .collect(),
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
        let named: Vec<crate::compile::Named> = paths
            .iter()
            .zip(["near", "far", "morph", "draw"])
            .map(|(path, name)| crate::compile::Named {
                name: Some(name.to_string()),
                path: path.clone(),
            })
            .collect();
        let edges = vec![karakuri_engine::set::Edge {
            node: "morph".to_string(),
            slot: "far".into(),
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
            crate::compile::Named::bare(paths[0].clone()),
            paths[1..]
                .iter()
                .cloned()
                .map(crate::compile::Named::bare)
                .collect(),
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
    /// reason [`crate::compile::sort_compiled`] is one function rather than a match in
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

        let rest: Vec<crate::compile::Named> = paths[1..]
            .iter()
            .map(|p| crate::compile::Named::bare(p.clone()))
            .collect();
        let (material, placed) =
            crate::compile::sort_slot(0, &crate::compile::Named::bare(paths[0].clone()), &rest);

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
    /// [`crate::compile::sort_compiled`] hands back a sentence rather than exiting: a
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
                crate::compile::Named::bare(paths[0].clone()),
                paths[1..]
                    .iter()
                    .cloned()
                    .map(crate::compile::Named::bare)
                    .collect(),
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
    /// **A re-point builds the material it was aimed at, on the poll it
    /// arrives, and the new files are not then seen as an edit.**
    ///
    /// This is how a Set reaches a *running* deck, and the whole reason it is
    /// a re-point rather than an install: `Deck::install` is documented as
    /// deliberately unreachable from a key or a surface, because *"a live run
    /// changes its material by editing a file and letting the worker build
    /// it, which is what the budget watchdog is attached to"*. So the failure
    /// this catches is a load that goes nowhere — an aim taken and no request
    /// made — and the deck goes on playing what it was with nothing said.
    ///
    /// **The second half is the debounce not eating it.** A save is acted on
    /// the poll *after* the one that saw it, because an editor writing in
    /// place leaves a file truncated for a moment. An aim has no such moment,
    /// and a re-point that went through the debounce would then be seen a
    /// second time as a change and built twice — so the poll after the build
    /// has to be quiet.
    #[test]
    fn an_aim_points_the_slot_at_what_it_names_and_is_not_debounced() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let tmp = tempfile::tempdir().expect("temp dir");
        let copy = |file: &str| {
            let path = tmp.path().join(file);
            std::fs::copy(root.join("examples").join(file), &path).expect("copy an example");
            path
        };
        // Two stacks that share their renderer, so what changes between them
        // is the geometry and the label says which one is running.
        let was = copy("drift_shell.kir");
        let now = copy("lattice_shell.kir");
        let renderer = copy("soft_points.kir");

        let (aim, aimed) = std::sync::mpsc::channel();
        let mut watch = Watch::new(
            0,
            crate::compile::Named::bare(was),
            vec![crate::compile::Named::bare(renderer.clone())],
            karakuri_engine::set::Layering::Overdraw,
            None,
            None,
            7,
            vec![7],
            karakuri_engine::camera::Orbit::default(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )
        .aimed_by(aimed);

        // Nothing has changed and nothing was aimed, so nothing is requested —
        // the floor under everything below, because a watcher that requested a
        // build every poll would pass the assertion after it by accident.
        assert!(
            watch.poll().is_none(),
            "a watcher nobody touched asked for a build"
        );

        // **A salt and a capacity no derivation here produces**, so that a
        // re-point which kept the outgoing slot's values cannot pass: the
        // symptom of that is invisible on the load and arrives at the first
        // later save.
        aim.send(Aim {
            head: crate::compile::Named::bare(now),
            rest: vec![crate::compile::Named::bare(renderer)],
            layering: karakuri_engine::set::Layering::Composite,
            live: Some(0),
            capacity: Some(2048),
            seed_salt: 0x0bad_cafe,
            salts: vec![0x0bad_cafe],
            camera: karakuri_engine::camera::Orbit::default(),
            overrides: Vec::new(),
            published: Vec::new(),
            bindings: Vec::new(),
            edges: Vec::new(),
            authorities: Vec::new(),
            set: None,
        })
        .expect("the watcher is still here");

        let Some(Polled::Build(request)) = watch.poll() else {
            panic!("the aim is built on the poll it arrives");
        };
        assert!(
            request.label.contains("lattice"),
            "the slot was aimed at `lattice_shell.kir` and built `{}`",
            request.label
        );
        assert_eq!(
            request.salts,
            vec![Some(0x0bad_cafe)],
            "the build kept the salts the slot was running at instead of the ones it was aimed \
             with"
        );
        assert_eq!(request.layering, karakuri_engine::set::Layering::Composite);
        assert_eq!(request.live, Some(0));
        assert_eq!(
            request.l1s[0].1, 2048,
            "the build kept the capacity the slot was running at"
        );

        assert!(
            watch.poll().is_none(),
            "the files it was aimed at were then seen as an edit, so the load built twice"
        );
    }

    /// **A library load moves which Set the versions after it are filed
    /// under**, which is the half of `ADR-0276` a re-point owes.
    ///
    /// The failure this is written against is silent and is only readable
    /// afterwards: a watcher that took the aim's files and left its Set id
    /// behind goes on filing every later version under the Set the slot was
    /// running *before* the load — a name, and nothing in the layout to say it
    /// is wrong, so *what versions has this Set had* answers with somebody
    /// else's edits.
    ///
    /// **The aim points at the files already being watched**, on purpose: the
    /// bytes do not change, so the only thing that can make this version a new
    /// one is the id in the dedup key. It is the `record` clause that says a
    /// chain is a node *of a Set*, asserted from the watcher's side.
    #[test]
    fn a_re_point_files_the_versions_after_it_under_the_set_it_loaded() {
        let tmp = tempfile::tempdir().expect("temp dir");
        let dir = tmp.path().join("scratch");
        std::fs::create_dir_all(&dir).expect("a directory to watch");
        let store = tmp.path().join("store");

        let (watch, paths) = watch_over(&dir, &["drift_shell.kir", "soft_points.kir"]);
        let shared = crate::history::Snapshots::shared(&store);
        let (aim, aimed) = std::sync::mpsc::channel();
        // **Launched on material no Set names**, which is what every slot of
        // `crates/karakuri` launches on and what `record` writes `None` for.
        let mut watch = watch.snapshotting_to(shared.clone(), None).aimed_by(aimed);

        rebuild(&mut watch).expect("the files appearing is an edit like any other");
        let before = crate::history::list(&store, 32).expect("the history lists back");
        assert!(
            before.versions.iter().all(|v| v.set.is_none()),
            "a run that has loaded nothing filed a version under a Set: {:?}",
            before.versions
        );
        assert!(
            !before.versions.is_empty(),
            "nothing was snapshotted at all, so what follows would pass against a \
             watcher that never records"
        );

        aim.send(Aim {
            head: crate::compile::Named::bare(paths[0].clone()),
            rest: vec![crate::compile::Named::bare(paths[1].clone())],
            layering: karakuri_engine::set::Layering::Overdraw,
            live: None,
            capacity: Some(4096),
            seed_salt: 1,
            salts: vec![1],
            camera: karakuri_engine::camera::Orbit::default(),
            overrides: Vec::new(),
            published: Vec::new(),
            bindings: Vec::new(),
            edges: Vec::new(),
            authorities: Vec::new(),
            set: Some("star_vortex".to_string()),
        })
        .expect("the watcher is still here");
        watch
            .poll()
            .expect("the aim is built on the poll it arrives");

        let after = crate::history::list(&store, 32).expect("the history lists back");
        let of_set: Vec<&crate::history::Version> = after
            .versions
            .iter()
            .filter(|v| v.set.as_deref() == Some("star_vortex"))
            .collect();
        assert!(
            !of_set.is_empty(),
            "the slot was loaded with `star_vortex` and its first version was filed \
             under the material the run started on: {:?}",
            after.versions
        );
        assert!(
            of_set
                .iter()
                .all(|v| v.file.to_string_lossy().contains("@star_vortex")),
            "the id is read back off the name, and the name does not carry it: {:?}",
            of_set
        );
    }

    #[test]
    fn a_missing_file_is_stable() {
        let tmp = tempfile::tempdir().expect("temp dir");
        let w = watch_on(tmp.path());
        assert_eq!(w.stamp(), [None, None]);
        assert_eq!(w.stamp(), w.stamp());
    }
}
