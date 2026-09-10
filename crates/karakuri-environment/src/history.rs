//! Every version that compiled, kept where a person can find it.
//!
//! # What this is for
//!
//! An edit replaces a procedure with no backup, whoever made it. A hand at an
//! editor and a model over MCP reach the same file through the same path, and
//! the version that was there is gone. This keeps it.
//!
//! The gate is **compiling**, not landing. A build that compiled and cost too
//! much to run is in here like any other, and the version *before* it is
//! exactly the kind of version worth going back to — it was a real attempt and
//! something about it was right, and landing it is one of the three ways out of
//! a slot the watchdog stopped (ADR-0316). What the session stream does *not*
//! have is a version that compiled and never reached a slot, because
//! `Record::Procedure` only names what reached the screen.
//!
//! # A chain is a node, and walking it is a listing
//!
//! A chain of snapshots per slot, layer and renderer is what a surface offers
//! a walk over, and what an operator saves from once they find the one they
//! liked. [`list`] is the reading half and it is a **listing**, on the same
//! terms `karakuri_store::Store::list_sets` is one — *what versions has this
//! had* is a list, landing on one is a load, and neither word is *undo*. What
//! does not exist is the surface: no control names a version, and saving to a
//! user preset is `--save-set`'s neighbourhood. The snapshots have to be taken
//! while the editing is happening or there is nothing to list later.
//!
//! # A version is filed under the Set the slot was running
//!
//! ```text
//! <store>/history/2026/08/16/143052-271_slot0_L4_beat_strokes@star_vortex.kir
//! ```
//!
//! A chain is still `(slot, layer, index)` and that is no longer the whole
//! address. The same slot holds a different Set after a library load, so
//! without the id the two sides of that load are one chain, and *what versions
//! has this Set had* is a question these files could not be asked at all.
//!
//! **It cost an argument rather than a design, because every route that edits
//! is addressed by slot.** [`crate::mcp`]'s `write_procedure` resolves
//! `Slots::path(slot, layer, index)` and refuses a node the slot does not
//! hold, and an operator's own editor is pointed at that slot's scratch copy.
//! So at the moment of a write the program knows which Set the slot is
//! running, and [`Snapshots::record`] is handed it rather than deducing it
//! from anything.
//!
//! **The id is the answer at the moment of the write, and a version that is
//! filed is never re-filed.** Both of the cases where there is no Set fall out
//! of that one sentence, and neither of them is a word — see
//! [`Snapshots::record`].
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
//! nothing to choose between. Each chain remembers what it last wrote, and a
//! chain is a node **of a Set**: a slot re-pointed at other material is a
//! different chain with its own memory, which is what keeps a load's first
//! version from being skipped as an edit that did not happen.

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
///
/// **And per Set**, which is the fourth field of the key and the one that is
/// not an address within the arrangement — see [`Snapshots::record`], which
/// argues why the id is part of a chain rather than a label on a row.
pub struct Snapshots {
    root: PathBuf,
    /// The chain — a node **of a Set** — against what it last wrote. The
    /// `Option` is a version written where there was no Set, which is a state
    /// and not a missing key; [`Snapshots::record`] says which runs are in it.
    last: HashMap<(usize, &'static str, usize, Option<String>), Vec<u8>>,
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
    /// what this chain wrote last.
    ///
    /// Returns the path written, or `None` when the source was unchanged.
    /// **Failures are the caller's to report and never to act on**: a snapshot
    /// that could not be written must not stop a build that compiled, because
    /// the operator asked for a picture and this is bookkeeping.
    ///
    /// # `set` is the Set the slot is running at the moment of the write
    ///
    /// Not the Set the source came from, and not one worked out afterwards.
    /// **Every route that edits is addressed by slot** — [`crate::mcp`]'s
    /// `write_procedure` resolves `Slots::path(slot, layer, index)` and refuses
    /// a node the slot does not hold, and an operator's editor is pointed at
    /// that slot's scratch copy — so the one thing every writer has in hand is
    /// which Set the slot it is writing into is playing. That is what this
    /// takes, and it takes it rather than keeping a table of its own, because a
    /// second copy of *what is this slot running* is a second thing to be
    /// wrong.
    ///
    /// `None` where there is no Set. That is a state rather than an argument
    /// nobody filled in, and the last section says which runs are in it.
    ///
    /// # It goes in the name, because a row is a name and nothing else
    ///
    /// `143052-271_slot0_L4_beat_strokes@star_vortex.kir`. [`list`] walks
    /// directory entries and opens no file — which is the shape of the thing
    /// rather than a shortcut it took — so an id kept anywhere a reader would
    /// have to open something for is an id [`Version`] cannot carry.
    ///
    /// Two placements lost to that, and they lost by different amounts.
    ///
    /// - **Beside the file**, a sidecar per snapshot, costs the lister one
    ///   open per row and doubles the entries in a directory an operator is
    ///   invited to work in by hand.
    /// - **A directory per Set**, `history/<set>/YYYY/MM/DD/`, costs more than
    ///   the open. `rm -rf history/2026/07` is this module's whole retention
    ///   policy and stops being one when a month is spread under every Set
    ///   separately; and the walk is lazy *because* the day directories are the
    ///   top of the tree — newest first, stopping once it has enough — which a
    ///   reader that had to enter every Set before it could order two days
    ///   cannot be.
    ///
    /// **Behind `@`, and after the procedure name, because both of the fields
    /// either side of that separator may hold `_`.** A Set id may:
    /// [`crate::accepted_save`] spells a model's save `<stamp>_<name>`, so a
    /// fixed `_`-delimited field is not merely awkward here, it is wrong for
    /// ids this program itself writes. `@` is outside [`sanitize`]'s alphabet
    /// and both fields go through it, so the last `@` in a name this wrote is
    /// the separator and there is no other.
    ///
    /// **A name with no `@` is a version with no Set**, which is also every
    /// name written before this argument existed — so the lister goes on
    /// claiming those, and reads them back as what they are rather than as
    /// versions of a Set called something.
    ///
    /// # The dedup key is the chain, and the chain is now per Set
    ///
    /// `(slot, layer, index, set)`. Keyed without the id, a load onto a node
    /// whose source happened to match what the outgoing Set last had would be
    /// skipped, and the incoming Set's chain would begin at its first *edit* —
    /// which is the hole [`seed`] exists to close, one level up. Keyed with it,
    /// a slot loaded back to a Set it has already played dedups against what
    /// **that** Set last had instead of writing a duplicate, because the memory
    /// is per chain and the chain came back.
    ///
    /// # Where there is no Set, and what a save does about it
    ///
    /// **A run launched with a pair on the command line is `None`.** There is
    /// no id to write: nothing was loaded and nothing was saved, and the
    /// material is two paths somebody typed. A name derived from those — which
    /// is what `crates/karakuri`'s `Sources::material` builds for the mixer
    /// strip — is a readout and not an id, and putting it here would file
    /// versions under a Set no listing can ever match. `Option` is the type
    /// that can say *there is none*; a sentinel that reads as a name cannot,
    /// because an operator is free to name a Set that word.
    ///
    /// **A save mid-chain changes nothing here, because a save does not move
    /// the slot.** It copies what is playing into the library under a new id.
    /// A **load** is what changes which Set a slot is running, and the panel
    /// already spells that difference: `played` rewrites the slot's name on a
    /// `LoadSet` and nothing rewrites it on a `SaveSet`. So the versions either
    /// side of a save carry whatever the slot was already carrying, and a Set a
    /// save created has no versions filed under its own id until somebody loads
    /// it. **That is the true answer and not a gap**: nothing has ever been a
    /// version *of* it — it **is** a version, and it is in the library.
    ///
    /// The alternative was to re-file the chain from the save forward, and it
    /// loses on its own terms. The version that was saved is the one written
    /// *before* the press, so a Set re-filed from the press onwards is a Set
    /// whose history does not contain itself; and making it contain itself
    /// means renaming files that are already filed, which is the one thing this
    /// module never does to a version it has written.
    pub fn record(
        &mut self,
        slot: usize,
        layer: &'static str,
        index: usize,
        set: Option<&str>,
        proc_name: &str,
        source: &[u8],
    ) -> Result<Option<PathBuf>, String> {
        // Owned to look up, which is one allocation on a path that is about to
        // create a directory and write a file. A borrowed key would be a
        // lifetime on this whole map to save it.
        let chain = (slot, layer, index, set.map(str::to_string));
        if self.last.get(&chain).is_some_and(|s| s == source) {
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
        // The Set is in the name only when there is one, so every name a run
        // with no Set has ever written is the name it still writes — the same
        // argument the index above is left off a `_0` by, and the same one that
        // lets `list` go on claiming the names written before this field was.
        let of = match set {
            Some(set) => format!("@{}", sanitize(set)),
            None => String::new(),
        };
        let name = format!(
            "{stamp}_slot{slot}_{layer}{at}_{}{of}.kir",
            sanitize(proc_name)
        );
        let path = dir.join(name);
        std::fs::write(&path, source).map_err(|e| format!("{}: {e}", path.display()))?;
        self.last.insert(chain, source.to_vec());
        Ok(Some(path))
    }
}

/// **A version the history holds**, as [`list`] found it.
///
/// Every field is read off the **name** [`Snapshots::record`] wrote, and the
/// name is the whole of it: nothing here opens a file. A row says when, which
/// node, and what the procedure called itself, which is what a person scanning
/// a night's work is scanning for — and the one thing a row is pressed for is
/// [`Version::file`], which is handed over rather than left to be rebuilt, on
/// [`crate::places::PresetSet::file`]'s reasoning exactly: rebuilding it at a
/// call site is this module's naming rule written a second time, in a crate
/// where no test here can fail when the two spellings part company.
///
/// **The address is a node of a running arrangement *and* the Set that
/// arrangement was.** `record` takes a slot, a layer, a renderer index and the
/// Set the slot was running, and writes all four into the name — so *which Set
/// was this a version of* is read off a name like everything else here.
/// [`Version::set`] is that answer, including where the answer is *none* — see
/// [`list`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
    /// **When**, spelled the way [`stamped_id`] spells it —
    /// `20260816-143052-271` — which is the day directory and the file's own
    /// time stamp put back together.
    ///
    /// That spelling is not a third convention: `stamped_id`'s own head says
    /// its time half *is* [`TIME`], "the same string a snapshot is named with,
    /// so the two cannot drift", and its date half is the day directory with
    /// the slashes taken out. So a version's `at` and the id a Set saved in the
    /// same millisecond would be filed under are the same string, and an
    /// operator reading one against the other is reading one clock.
    ///
    /// **A string, and it sorts.** Fixed-width and zero-padded at every field,
    /// so lexicographic order is chronological order and the listing needs no
    /// date type to be ordered by. Parsing it into one is the caller's, and
    /// nothing in this crate has needed to.
    pub at: String,
    /// The slot this was a version of, as `record` was given it.
    pub slot: usize,
    /// `L1`, `L2`, `L3`, `L4` or `Field` — one of [`LAYERS`], which is the same
    /// list [`declared_kind`] answers with and the same list
    /// [`crate::mcp`]'s `Slots` addresses a node by.
    pub layer: &'static str,
    /// Which renderer of that layer, counting from zero. A name carries this
    /// only when it is not the first ([`Snapshots::record`] says why), so a
    /// name without one reads back as `0` — the row and the writer's argument
    /// are the same number either way.
    pub index: usize,
    /// What the procedure called itself, through [`sanitize`]. It is what the
    /// file was named after and not what the file says now; nothing is opened.
    pub proc_name: String,
    /// **Which Set the slot was running** when this version was written, or
    /// `None` for one written where there was no Set at all — which is a state
    /// rather than an unknown, and [`Snapshots::record`] names the runs it is
    /// the truth about.
    ///
    /// Read off the name behind `@`, and absent from a name that carries no
    /// `@`. `record` argues both the placement and the `Option`.
    ///
    /// **It is here so the narrowing is possible, and the narrowing is not
    /// here** — see [`list`].
    pub set: Option<String>,
    /// The snapshot itself — what to read, and what a load would be pointed at.
    pub file: PathBuf,
}

/// **What [`list`] found**, and what it did not.
///
/// Three fields because a listing off a directory that grows on its own has
/// three things to say, and the two beside the rows are the ones a caller
/// cannot recover for itself.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Listing {
    /// **Most recent first**, at most as many as were asked for.
    pub versions: Vec<Version>,
    /// **The walk stopped with directories left unread.** Not *there are older
    /// versions* — it says what happened, and a day left unopened could turn
    /// out to hold nothing.
    ///
    /// It exists so a truncated listing cannot read as a whole one, which is
    /// the rule [`crate::mcp`]'s `list_sets` states at its own truncation: a
    /// caller told *here is the history* over ten of ten thousand will act on a
    /// history it has not seen.
    pub stopped_short: bool,
    /// **How many entries the layout does not claim** were passed over, in the
    /// directories actually read.
    ///
    /// Counted rather than dropped, and counted rather than named. A count is
    /// bounded and a list of names is not — a day directory is a place an
    /// operator is *invited* to work in by hand, because `rm -rf
    /// history/2026/07` is this module's whole retention policy, so whatever
    /// else is in there belongs to them and is already one `ls` away. What the
    /// count buys is the thing an `ls` will not tell them: that this listing
    /// went past something.
    pub unclaimed: usize,
}

/// **The six words a layer is spelled with**, in a `kind` line and in a
/// snapshot's name.
///
/// One list, because [`declared_kind`] and [`list`] have to agree about it: the
/// layer a snapshot is filed under is the layer an agent addresses it by, and
/// two lists would be a listing that reported a node under a name nothing can
/// ask for.
///
/// **No entry is a prefix of another**, which is what lets a name's
/// `L41`/`Field` field be split into a layer and an index by trying each of
/// these in turn.
pub const LAYERS: [&str; 6] = ["L1", "L2", "L3", "L4", "Field", "L5"];

/// One of [`LAYERS`], or `None` for a word this module does not spell a layer
/// with.
fn known_layer(word: &str) -> Option<&'static str> {
    LAYERS.into_iter().find(|layer| *layer == word)
}

/// **What versions the history holds**, most recent first, at most `most` of
/// them.
///
/// `store_root` is the store, not the history root — the same argument
/// [`Snapshots::new`] takes, and [`DIR`] joined on here for the same reason, so
/// a reader and the writer cannot end up looking at two directories.
///
/// # What a version is *of*, and why this is still not `versions_of(set_id)`
///
/// [`Snapshots::record`] addresses a snapshot by **slot, layer and renderer
/// index** — a node of the arrangement that was running when it was written —
/// **and by the Set the slot was running**, which it writes into the name. So
/// both questions are answerable off these rows and neither needs a second
/// reader: *the versions this node has had* is `(slot, layer, index)`, the key
/// `record`'s dedup is on, and *the versions this Set has had* is
/// [`Version::set`].
///
/// **Neither narrowing is here, and that is one rule applied twice.** One
/// listing, ordered, with the whole address on every row; which rows an
/// operator is looking at is a question the surface asks, the way
/// `Operation::ListSets`' two filters are applied where they are answered
/// ([ADR-0262](../../../docs/adr/0262-a-library-filter-field-steps-through-what-the-store-already-holds-rather-than-taking-letters.md))
/// and not inside `Store::list_sets`. What this owes the surface is that the
/// filter is *possible*, and the address on the row is that.
///
/// **A row whose `set` is `None` is a row no Set matches**, and a narrowing has
/// to spell that rather than let it fall through as a wildcard: those versions
/// were written where there was no Set — `record` says which runs those are —
/// and a filter that folded them into whichever Set was asked for would be
/// inventing a history for it.
///
/// # Most recent first, and here that is the layout's order rather than a sort
///
/// [ADR-0263](../../../docs/adr/0263-the-library-bay-lists-most-recent-first-because-the-listing-is-the-operations-and-not-the-surfaces.md)
/// decided the Library bay lists most recent first, and this is that order.
/// **It also rejected sorting inside `Store::list_sets`**, and that half does
/// not carry over, for the two reasons it was rejected on:
///
/// - *The store answers what files are there and recency is a presentation
///   choice.* Here recency **is** what files are there. The directory is
///   `YYYY/MM/DD` and the name opens with `HHMMSS-mmm`; there is no other key,
///   and offering the alphabetical order of a name whose first ten characters
///   are a clock would be offering the same order under a worse description.
/// - *Two files inside one tick of a coarse clock tie, and a tied sort is not
///   an order.* No mtime is read here at all — the time is in the name, to the
///   millisecond, and the rest of the name breaks even that tie: two nodes
///   recorded in one millisecond differ by slot, layer, index or procedure. The
///   tie-break is the file name ascending, which is ADR-0263's shape (recency
///   descending, then the id ascending) with the only id these rows have.
///
/// The order is total and repeatable without touching the filesystem clock,
/// which is what lets the walk below be lazy.
///
/// # A directory of directories, and what is skipped
///
/// Three levels — a four-digit year, a two-digit month, a two-digit day — and
/// exactly three: nothing recurses, because nothing writes deeper than that.
///
/// **A name the layout does not claim is skipped and counted**, never repaired
/// and never opened. That is `Store::list_sets`' rule, plus the count, and the
/// count is the difference: a `sets/` directory holds what an operator saved,
/// while a history directory is one they are told to go into and delete from by
/// hand, so *something is in there that I did not write* is the ordinary case
/// rather than the alarming one, and a listing that dropped it silently would
/// be the only party who knew. [`Listing::unclaimed`] is the report;
/// `Operation::ListSets`' own row — *"and it says how many it did not show"* —
/// is the sentence it is written after.
///
/// Skipped, concretely: anything at the date levels that is not a directory of
/// the right width in digits; anything in a day directory that is a directory,
/// or whose name is not one [`Snapshots::record`] would have written — the
/// `.kir` suffix, a `HHMMSS-mmm` stamp, `slot<digits>`, a [`LAYERS`] word with
/// an optional index after it, a procedure name in [`sanitize`]'s alphabet, and
/// — where there is one — an `@` and a Set id in that same alphabet.
/// A non-UTF-8 name fails `to_str` and falls out with the rest, no lossy
/// repair and no unwrap for a hostile name to trip.
///
/// **The digits are checked for width and not for a calendar.** `2026/13/40`
/// lists under `20261340-…` and sorts where its name says. What this reads is
/// a layout, and a month number is not something it is in a position to
/// dispute — a directory an operator made by hand is theirs.
///
/// # Cost
///
/// **Days newest first, and it stops opening them once it has enough.** One
/// `read_dir` per date directory entered, one per day opened, and no file is
/// opened at all — a row is a name, which is why *no reader for the file
/// contents* is not a shortcut here but the shape of the thing.
///
/// So the bound is on **days opened**, not on entries seen: the day directory
/// that meets the cap is read whole, because `read_dir` has no order and the
/// newest name in a day cannot be known without seeing all of them. A day with
/// fifty thousand files in it is fifty thousand entries however small `most`
/// is. That is the floor, and it is the operator's own directory.
///
/// **`most` is the caller's and there is no default.** What a bay can afford to
/// draw and what a model can afford to be handed are different numbers, and
/// this is not the place either is decided
/// ([P-0090](../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
///
/// **Nothing counts what lies past the cap**, and that is the deliberate hole:
/// counting the rest means reading every remaining day directory, which is the
/// cost the cap exists not to pay. [`Listing::stopped_short`] says the walk
/// stopped, which is the property that matters — it is what keeps a truncated
/// listing from reading as a whole one — and a caller that wants the number
/// asks for a larger `most` and pays for it knowingly
/// ([P-0091](../../../docs/principles/0091-cost-is-known-before-it-is-paid.md)).
///
/// # A history that is not there is empty, and that is not an error
///
/// **The opposite of `Store::list_sets`, and the difference is who creates the
/// directory.** `Store::open` creates `sets/`, so a `sets/` that has gone is a
/// store that has been damaged since it was opened and an empty `Vec` would
/// answer a question that could not be read. Nothing creates `history/`:
/// [`Snapshots::record`] makes it on the first snapshot, so a store that has
/// never been edited has none — and neither has one whose only runs were
/// offscreen, since a `--render` is handed no store to write into. *No
/// versions* is the true answer to both, and the same holds for a day
/// directory that vanishes mid-walk, since `rm -rf history/2026/07` is this
/// module's retention policy and an operator running it is not an error.
///
/// Anything else the filesystem refuses is an `Err`, naming the directory it
/// refused, in the `String` this module's other failure is spelled in.
pub fn list(store_root: &Path, most: usize) -> Result<Listing, String> {
    let mut listing = Listing::default();
    let root = store_root.join(DIR);
    'walk: for (year, year_dir) in dated(&root, 4, &mut listing.unclaimed)? {
        for (month, month_dir) in dated(&year_dir, 2, &mut listing.unclaimed)? {
            for (day, day_dir) in dated(&month_dir, 2, &mut listing.unclaimed)? {
                // Checked before the directory is opened, not after: this is
                // the whole of the bound, and a check on the way out would have
                // already paid for the day it was refusing.
                if listing.versions.len() >= most {
                    listing.stopped_short = true;
                    break 'walk;
                }
                let date = format!("{year}{month}{day}");
                let mut of_the_day = Vec::new();
                for entry in read(&day_dir)? {
                    let entry = entry.map_err(|e| cannot_be_listed(&day_dir, &e))?;
                    let claimed = !entry
                        .file_type()
                        .map_err(|e| cannot_be_listed(&day_dir, &e))?
                        .is_dir();
                    match claimed.then(|| version(&date, &entry)).flatten() {
                        Some(found) => of_the_day.push(found),
                        None => listing.unclaimed += 1,
                    }
                }
                // One day, so `at` differs only in its time half — but sorted
                // on the whole of it anyway, because the field that orders the
                // listing across days is the field that has to order it within
                // one, or the two would be free to disagree.
                of_the_day.sort_by(|a, b| b.at.cmp(&a.at).then_with(|| a.file.cmp(&b.file)));
                listing.versions.append(&mut of_the_day);
            }
        }
    }
    if listing.versions.len() > most {
        listing.versions.truncate(most);
        listing.stopped_short = true;
    }
    Ok(listing)
}

/// The subdirectories of `dir` named in exactly `digits` ASCII digits, **newest
/// name first**, with everything else under `dir` counted into `unclaimed`.
///
/// The name comes back beside the path because the three of them spell
/// [`Version::at`], and re-reading it off the path at the bottom of the walk
/// would be reading it twice.
///
/// Fixed width and zero-padded is what makes a string sort a date sort, which
/// is the property the whole walk is lazy on.
fn dated(
    dir: &Path,
    digits: usize,
    unclaimed: &mut usize,
) -> Result<Vec<(String, PathBuf)>, String> {
    let mut out = Vec::new();
    for entry in read(dir)? {
        let entry = entry.map_err(|e| cannot_be_listed(dir, &e))?;
        let name = entry.file_name();
        let claimed = name
            .to_str()
            .filter(|n| n.len() == digits && n.bytes().all(|b| b.is_ascii_digit()))
            .map(str::to_string);
        match claimed {
            Some(name)
                if entry
                    .file_type()
                    .map_err(|e| cannot_be_listed(dir, &e))?
                    .is_dir() =>
            {
                out.push((name, entry.path()))
            }
            _ => *unclaimed += 1,
        }
    }
    out.sort();
    out.reverse();
    Ok(out)
}

/// `read_dir`, with **a directory that is not there reading as an empty one**.
///
/// [`list`] says why at length: nothing creates `history/`, a day directory is
/// something an operator is told to delete, and *no versions* is the true
/// answer to both. Every other refusal is carried out as an error naming the
/// directory.
fn read(dir: &Path) -> Result<Vec<std::io::Result<std::fs::DirEntry>>, String> {
    match std::fs::read_dir(dir) {
        Ok(entries) => Ok(entries.collect()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(cannot_be_listed(dir, &e)),
    }
}

/// The directory could not be read, named, with the reason the filesystem gave
/// rather than a guess at it — [`Snapshots::record`]'s spelling for the same
/// kind of failure, which is the one an operator sees beside it.
fn cannot_be_listed(dir: &Path, why: &std::io::Error) -> String {
    format!("cannot list the edit history at `{}`: {why}", dir.display())
}

/// One entry of a day directory read back as the version
/// [`Snapshots::record`] wrote, or `None` for a name it would not have written.
///
/// **The inverse of `record`'s naming and nothing else.** It re-derives every
/// field from the name rather than accepting anything that parses, so the day a
/// name changes shape this stops claiming the old one — which is the only thing
/// standing between a listing and a row pointing at a file that means something
/// else.
fn version(date: &str, entry: &std::fs::DirEntry) -> Option<Version> {
    let name = entry.file_name();
    let stem = name.to_str()?.strip_suffix(".kir")?;
    // **The Set comes off the tail first, on `@`.** Both fields either side of
    // it may hold `_`, so `_` cannot separate them — see [`Snapshots::record`],
    // which also says why `@` can only be the separator: it is outside
    // [`sanitize`]'s alphabet and both fields have been through it.
    //
    // A name with no `@` is a version with no Set, which is what `record`
    // writes for a run that has none and is also every name written before
    // there was a field. A name whose tail is not something `record` could have
    // written is refused outright rather than read as a procedure name with an
    // `@` in it, on this function's own rule: the inverse of the naming and
    // nothing else.
    let (stem, set) = match stem.rsplit_once('@') {
        Some((head, set)) if !set.is_empty() && sanitize(set) == set => {
            (head, Some(set.to_string()))
        }
        Some(_) => return None,
        None => (stem, None),
    };
    // `splitn(4)`, because a procedure name may hold `_` — `sanitize` keeps it
    // — and the three fields before it may not.
    let mut fields = stem.splitn(4, '_');
    let (time, slot, addressed, proc_name) = (
        fields.next()?,
        fields.next()?.strip_prefix("slot")?,
        fields.next()?,
        fields.next()?,
    );
    // `TIME` is `%H%M%S-%3f`, which is six digits, a hyphen and three.
    //
    // **Over the bytes and never a `&time[..6]`.** A file name is not this
    // program's to assume anything about, and slicing a `str` at a fixed byte
    // index panics when that index lands inside a multi-byte character — so a
    // listing would abort on a name somebody put in the directory rather than
    // pass over it, which is the opposite of what every other refusal here
    // does.
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    let stamp = time.as_bytes();
    if stamp.len() != 10
        || stamp[6] != b'-'
        || !stamp[..6].iter().all(u8::is_ascii_digit)
        || !stamp[7..].iter().all(u8::is_ascii_digit)
    {
        return None;
    }
    // `L4` and `L41` are one field, and no member of `LAYERS` is a prefix of
    // another, so the first that strips is the only one that can.
    let (layer, index) = LAYERS
        .into_iter()
        .find_map(|layer| addressed.strip_prefix(layer).map(|rest| (layer, rest)))?;
    // Absent is the first renderer: `record` leaves the index off a `_0`, so
    // the row and the argument it was written from are the same number.
    let index = if index.is_empty() {
        0
    } else if digits(index) {
        index.parse().ok()?
    } else {
        return None;
    };
    if !digits(slot) || proc_name.is_empty() || sanitize(proc_name) != proc_name {
        return None;
    }
    Some(Version {
        at: format!("{date}-{time}"),
        slot: slot.parse().ok()?,
        layer,
        index,
        proc_name: proc_name.to_string(),
        set,
        file: entry.path(),
    })
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
///
/// **A slot arrives with the Set it is about to run**, or `None` where it is
/// about to run material no Set names — a pair on the command line. It is the
/// same argument [`Snapshots::record`] takes and for its reason: the caller is
/// the one holding it, and the seed is the first version of the chain it
/// belongs to.
pub fn seed<'a>(
    shared: &Shared,
    sets: impl Iterator<Item = (usize, Option<&'a str>, Vec<&'a Path>)>,
) {
    let Ok(mut snapshots) = shared.lock() else {
        return;
    };
    for (slot, set, paths) in sets {
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
            if let Err(e) = snapshots.record(slot, layer, index, set, &name, &source) {
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
/// **Shared with [`crate::mcp`]'s `Slots`**, which resolves a
/// `(slot, layer, index)` address by the same scan. Two readers of a `kind`
/// line would be two rules for what layer a file is on, and the layer a
/// snapshot is filed under has to be the layer an agent addresses it by or the
/// surface would be editing one node and undoing another. **`pub` rather than
/// `pub(crate)` for exactly that reader**, which was one crate over when this
/// was widened and is a module beside this one now; the widening is kept
/// because the argument was never about the keyword.
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
        // [`LAYERS`] rather than a match written out here, because [`list`]
        // reads the same six words back off a file name — and the layer a
        // snapshot is filed under has to be the layer a listing reports it
        // under, for the reason this doc gives about addressing.
        return known_layer(rest.trim_start().split(char::is_whitespace).next()?);
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

    /// **Every word in [`LAYERS`] is a word [`declared_kind`] reads back**, and
    /// the sixth is the one that would have gone quietly wrong.
    ///
    /// The layer a snapshot is filed under has to be the layer an agent
    /// addresses it by; `declared_kind` answers off `LAYERS` rather than off a
    /// match written out here for exactly that reason, and this is the
    /// assertion that the indirection holds — including for a `kind` line with
    /// a trailing comment, which is the shape the scan's *first token after the
    /// space* rule exists for.
    #[test]
    fn declared_kind_reads_back_every_layer_including_l5() {
        for layer in LAYERS {
            let source = format!("proc p {{\n  kind {layer}\n}}\n");
            assert_eq!(
                declared_kind(source.as_bytes()),
                Some(layer),
                "`kind {layer}` has to read back as `{layer}`"
            );
        }
        assert_eq!(
            declared_kind(b"proc feedback {\n  kind L5   // a frame effect\n  retains\n}\n"),
            Some("L5"),
            "a trailing comment is not part of the answer"
        );
        // **`L5` is not a prefix of another word and no other word is a prefix
        // of it**, which is what lets a snapshot's `L41`/`Field` name be split
        // into a layer and an index by trying each of these in turn.
        assert_eq!(declared_kind(b"proc p {\n  kind L55\n}\n"), None);
        assert_eq!(declared_kind(b"proc p {\n  kind L6\n}\n"), None);
    }

    /// The load-bearing claim: what was there before an edit is still readable
    /// after it.
    #[test]
    fn a_snapshot_holds_the_source_it_was_given() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut snaps = Snapshots::new(tmp.path());

        let written = snaps
            .record(0, "L4", 0, None, "soft_points", b"the first version")
            .expect("record")
            .expect("a first version is always new");
        assert_eq!(std::fs::read(&written).expect("read"), b"the first version");

        let second = snaps
            .record(0, "L4", 0, None, "soft_points", b"the second version")
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
            .record(0, "L1", 0, None, "field", b"same")
            .expect("record")
            .is_some());
        assert!(
            snaps
                .record(0, "L1", 0, None, "field", b"same")
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
            .record(0, "L1", 0, None, "field", b"same")
            .expect("record")
            .is_some());
        assert!(
            snaps
                .record(1, "L1", 0, None, "field", b"same")
                .expect("record")
                .is_some(),
            "slot 1's first snapshot was skipped because slot 0 had the same source"
        );
        assert!(
            snaps
                .record(0, "L4", 0, None, "field", b"same")
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
            .record(0, "L4", 0, None, "sprites", b"first")
            .expect("record")
            .is_some());
        assert!(
            snaps
                .record(0, "L4", 1, None, "strokes", b"second")
                .expect("record")
                .is_some(),
            "the second renderer's first snapshot was skipped as the first renderer's"
        );
        assert!(
            snaps
                .record(0, "L4", 1, None, "strokes", b"second")
                .expect("record")
                .is_none(),
            "the second renderer's unchanged source was written again"
        );
        assert!(
            snaps
                .record(0, "L4", 0, None, "sprites", b"first")
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
            .record(0, "L4", 0, None, "sprites", b"a")
            .expect("record")
            .expect("written");
        let second = snaps
            .record(0, "L4", 1, None, "strokes", b"b")
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
            .record(2, "L4", 0, None, "beat_strokes", b"x")
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
            std::iter::once((0, None, vec![l1.as_path(), l4.as_path()])),
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
                None,
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
            std::iter::once((0, None, vec![l1.as_path(), l4.as_path()])),
        );

        let mut snapshots = shared.lock().expect("lock");
        assert!(
            snapshots
                .record(0, "L1", 0, None, "field_one", b"proc field_one {}")
                .expect("record")
                .is_none(),
            "the untouched L1 was written a second time"
        );
        assert!(
            snapshots
                .record(0, "L4", 0, None, "draw_one", b"proc draw_one { edited }")
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
            .record(0, "L1", 0, None, "../../etc/passwd", b"x")
            .expect("record")
            .expect("new");

        assert!(
            path.starts_with(tmp.path().join(DIR)),
            "{} escaped the history root",
            path.display()
        );
        assert!(!path.to_string_lossy().contains(".."), "{}", path.display());
    }

    // ----- The lister ---------------------------------------------------
    //
    // **Every fixture below is written by `Snapshots::record`.** A history
    // built by hand here would be this module's naming rule typed a second
    // time, and the listing and the writer could then drift apart in exactly
    // the way neither would notice — the tests would go on passing against a
    // layout nothing writes.

    /// Put a snapshot the writer produced under another day, keeping the name
    /// the writer gave it.
    ///
    /// The clock is the one argument `record` does not take — the day comes
    /// from `Local::now()` — so a history spanning two days is built by moving
    /// what the writer wrote rather than by typing a file name this module
    /// might no longer spell that way.
    fn on(day: &str, store: &Path, path: &Path) -> PathBuf {
        let dir = store.join(DIR).join(day);
        std::fs::create_dir_all(&dir).expect("the day directory");
        let moved = dir.join(path.file_name().expect("a name"));
        std::fs::rename(path, &moved).expect("move");
        moved
    }

    /// **A store nothing has edited lists nothing, and does not fail.**
    ///
    /// `Store::open` does not create `history/` — `record` does, on the first
    /// snapshot — so *no directory* is the ordinary state of a store that has
    /// never been edited, and of every store a panel run filled, because only
    /// `karakuri-cli` builds a `Snapshots` today. An error here would report a
    /// damaged store for the commonest case there is.
    #[test]
    fn a_store_that_has_never_been_edited_lists_no_versions() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let listing = list(tmp.path(), 100).expect("a store with no history is not a failure");
        assert_eq!(listing, Listing::default());
    }

    /// **A row carries the address the snapshot was recorded under**, all four
    /// fields of it, and the file it names is the one that was written.
    ///
    /// This is the anti-drift test: `record`'s arguments go in and the same
    /// numbers come back out of the name, including the two the name spells
    /// oddly — a renderer index that is left off when it is zero, and a
    /// multi-digit index that runs into the layer word (`L410`).
    #[test]
    fn a_row_reads_back_the_address_the_snapshot_was_recorded_under() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut snaps = Snapshots::new(tmp.path());
        for (slot, layer, index, name) in [
            (2, "L4", 0, "beat_strokes"),
            (0, "L4", 10, "far"),
            (1, "Field", 0, "blob"),
        ] {
            snaps
                .record(slot, layer, index, None, name, name.as_bytes())
                .expect("record")
                .expect("new");
        }

        let listing = list(tmp.path(), 100).expect("listed");
        assert_eq!(listing.versions.len(), 3, "{listing:?}");
        assert_eq!(listing.unclaimed, 0, "{listing:?}");
        assert!(!listing.stopped_short, "{listing:?}");

        let found = |name: &str| {
            listing
                .versions
                .iter()
                .find(|v| v.proc_name == name)
                .unwrap_or_else(|| panic!("no row for {name}: {listing:?}"))
        };
        let address = |v: &Version| (v.slot, v.layer, v.index);
        assert_eq!(address(found("beat_strokes")), (2, "L4", 0));
        assert_eq!(
            address(found("far")),
            (0, "L4", 10),
            "`L410` did not read back as the eleventh renderer of L4"
        );
        assert_eq!(address(found("blob")), (1, "Field", 0));
        // Recorded with no Set, so read back with none — and not with a word
        // standing in for one, which is what the `Option` is for.
        assert!(
            listing.versions.iter().all(|v| v.set.is_none()),
            "a version written with no Set came back filed under one: {listing:?}"
        );

        // And the row points at the snapshot rather than describing it.
        assert_eq!(
            std::fs::read(&found("blob").file).expect("read"),
            b"blob",
            "the row's file is not the one the snapshot went into"
        );
        // `at` is `stamped_id`'s spelling: eight digits of date, then `TIME`.
        let at = &found("blob").at;
        assert_eq!(at.len(), 19, "{at}");
        assert!(
            at.starts_with(&chrono::Local::now().format("%Y%m%d").to_string()),
            "{at} is not today's local date"
        );
    }

    /// **Most recent first, and across the day directories rather than within
    /// one.** A walk starts at the newest version, and the newest version is in
    /// yesterday's directory as often as in today's.
    #[test]
    fn versions_are_listed_most_recent_first_across_two_days() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut snaps = Snapshots::new(tmp.path());
        let write = |snaps: &mut Snapshots, name: &str| {
            snaps
                .record(0, "L4", 0, None, name, name.as_bytes())
                .expect("record")
                .expect("new")
        };
        // One chain, four versions of it — which is what a walk walks.
        let first = write(&mut snaps, "v1");
        let second = write(&mut snaps, "v2");
        write(&mut snaps, "v3");
        write(&mut snaps, "v4");
        on("2000/01/02", tmp.path(), &first);
        on("2000/01/02", tmp.path(), &second);

        let listing = list(tmp.path(), 100).expect("listed");
        let ats: Vec<&str> = listing.versions.iter().map(|v| v.at.as_str()).collect();
        assert_eq!(ats.len(), 4, "{listing:?}");
        // **Non-ascending rather than strictly descending, and that is the
        // contract.** Four `record` calls in a row land inside one millisecond
        // on an unloaded machine — the first draft of this asserted `>` and
        // failed on exactly that — so `at` is not unique and the order it
        // gives is the order it can give. What breaks the tie is the file
        // name, asserted below, which is what makes the listing repeatable.
        assert!(
            ats.windows(2).all(|pair| pair[0] >= pair[1]),
            "not most recent first: {ats:?}"
        );
        assert!(
            listing
                .versions
                .windows(2)
                .all(|pair| pair[0].at > pair[1].at || pair[0].file < pair[1].file),
            "a tie was left to `read_dir`, which is not an order: {listing:?}"
        );
        let today = chrono::Local::now().format("%Y%m%d").to_string();
        assert!(
            ats[0].starts_with(&today) && ats[1].starts_with(&today),
            "today's versions are not at the top: {ats:?}"
        );
        assert!(
            ats[2].starts_with("20000102") && ats[3].starts_with("20000102"),
            "the older day is not at the bottom: {ats:?}"
        );
    }

    /// **What the layout does not claim is skipped and counted**, at every
    /// level, and never repaired into a row that points at somebody else's
    /// file.
    ///
    /// A day directory is a place an operator is invited into — `rm -rf
    /// history/2026/07` is the whole retention policy — so things that are not
    /// snapshots turn up in it as a matter of course, and the count is what
    /// stops the listing being the only party who knew.
    #[test]
    fn a_name_the_layout_does_not_claim_is_counted_and_not_listed() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut snaps = Snapshots::new(tmp.path());
        let kept = snaps
            .record(0, "L4", 0, None, "sprites", b"kept")
            .expect("record")
            .expect("new");
        let root = tmp.path().join(DIR);
        let day = kept.parent().expect("a day directory").to_path_buf();

        // Two at the top: a note somebody left, and a file with a year's name
        // on it — which is claimed by name and refused by type.
        std::fs::write(root.join("README"), "mine").expect("write");
        std::fs::write(root.join("1999"), "not a year").expect("write");
        // And six in the day directory: a note, an editor's leftover, a
        // directory, a directory named exactly like a snapshot, a slot that is
        // not a number, and a layer this module does not spell.
        std::fs::write(day.join("notes.txt"), "mine").expect("write");
        std::fs::write(day.join("120000-000_slot0_L4_x.kir.tmp"), "half").expect("write");
        std::fs::create_dir(day.join("scratch")).expect("mkdir");
        std::fs::create_dir(day.join("120000-000_slot0_L4_x.kir")).expect("mkdir");
        std::fs::write(day.join("120000-000_slotX_L4_x.kir"), "x").expect("write");
        std::fs::write(day.join("120000-000_slot0_L9_x.kir"), "x").expect("write");
        // **And a name that is the right length in bytes and not in
        // characters.** `123456é90` is ten bytes with the `é` across the
        // seventh and eighth, so a stamp check that sliced `&time[7..]` would
        // panic on a boundary rather than pass the name over — a file somebody
        // else put in the directory taking the listing down with it.
        std::fs::write(day.join("123456é90_slot0_L4_x.kir"), "x").expect("write");
        // **And a name with an `@` whose tail is not something `record` could
        // have written**, `a.b` being outside `sanitize`'s alphabet. Refused
        // outright rather than read back as a procedure with an `@` in its
        // name: this reader is the inverse of the naming and nothing else, so
        // a name it could not have produced is somebody else's file.
        std::fs::write(day.join("120000-000_slot0_L4_x@a.b.kir"), "x").expect("write");

        let listing = list(tmp.path(), 100).expect("listed");
        assert_eq!(
            listing.versions.len(),
            1,
            "something that is not a snapshot was listed as one: {listing:?}"
        );
        assert_eq!(listing.versions[0].file, kept);
        assert_eq!(
            listing.unclaimed, 10,
            "what was passed over was not reported: {listing:?}"
        );
    }

    /// **A version carries the Set the slot was running, and the reader gets it
    /// back off the name.**
    ///
    /// The load-bearing half of *a version is filed under the Set*: `record`'s
    /// argument goes in and the same string comes out of `list`, which opens
    /// no file — so the id is in the layout rather than in something a reader
    /// would have to fetch.
    #[test]
    fn a_version_written_under_a_set_reads_back_under_it() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut snaps = Snapshots::new(tmp.path());
        let written = snaps
            .record(0, "L4", 0, Some("star_vortex"), "beat_strokes", b"x")
            .expect("record")
            .expect("new");

        let name = written
            .file_name()
            .expect("a name")
            .to_string_lossy()
            .to_string();
        assert!(
            name.ends_with("_beat_strokes@star_vortex.kir"),
            "the Set is not in the name: {name}"
        );

        let listing = list(tmp.path(), 100).expect("listed");
        assert_eq!(listing.versions.len(), 1, "{listing:?}");
        let row = &listing.versions[0];
        assert_eq!(
            row.set.as_deref(),
            Some("star_vortex"),
            "the row does not say which Set this was a version of: {listing:?}"
        );
        // And the rest of the address survived the new field rather than being
        // eaten by it — `@` is the separator and `_` is still the other one.
        assert_eq!(
            (row.slot, row.layer, row.index, row.proc_name.as_str()),
            (0, "L4", 0, "beat_strokes"),
            "{listing:?}"
        );
        assert_eq!(row.file, written);
    }

    /// **Two Sets on one slot are two chains, and the second is not swallowed
    /// as the first's unchanged source.**
    ///
    /// This is the whole gap. A library load moves a slot to different
    /// material, and the two sides of it were indistinguishable: one chain,
    /// one `last`, and the incoming Set's first version skipped whenever its
    /// source happened to match what the outgoing one had — so that Set's
    /// history began at its first *edit*, which is the hole `seed` exists to
    /// close one level up.
    ///
    /// **Byte-identical on purpose.** Different sources would pass under the
    /// old key too and would prove nothing about it.
    #[test]
    fn two_sets_on_one_slot_are_two_chains() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut snaps = Snapshots::new(tmp.path());
        assert!(snaps
            .record(0, "L4", 0, Some("drift_cloud"), "shell", b"same")
            .expect("record")
            .is_some());
        assert!(
            snaps
                .record(0, "L4", 0, Some("star_vortex"), "shell", b"same")
                .expect("record")
                .is_some(),
            "the Set loaded onto this slot had its first version skipped as the \
             Set before it having already written those bytes"
        );
        // And the dedup is not lost, only re-keyed: the same Set writing the
        // same bytes again is still one row.
        assert!(
            snaps
                .record(0, "L4", 0, Some("star_vortex"), "shell", b"same")
                .expect("record")
                .is_none(),
            "an unchanged source was written again once the key grew a field"
        );
        // **And a slot loaded back to a Set it has already played dedups
        // against what that Set last had.** The memory is per chain, and the
        // chain came back — so switching between two Sets is not a file per
        // switch.
        assert!(
            snaps
                .record(0, "L4", 0, Some("drift_cloud"), "shell", b"same")
                .expect("record")
                .is_none(),
            "coming back to a Set wrote its unchanged source a second time"
        );

        let listing = list(tmp.path(), 100).expect("listed");
        assert_eq!(listing.versions.len(), 2, "{listing:?}");
        let mut sets: Vec<&str> = listing
            .versions
            .iter()
            .map(|v| v.set.as_deref().unwrap_or("<none>"))
            .collect();
        sets.sort();
        assert_eq!(
            sets,
            vec!["drift_cloud", "star_vortex"],
            "the two sides of a load are not distinguishable: {listing:?}"
        );
        // Same node, same source, same millisecond — the Set is the only thing
        // telling these two rows apart, which is the point.
        assert_ne!(listing.versions[0].file, listing.versions[1].file);
    }

    /// **A run with no Set files versions under none, and a save does not go
    /// back and re-file them.**
    ///
    /// Both of the cases where there is no id, in the order they happen. A run
    /// launched with a pair on the command line has none at all — there is
    /// nothing to write, and `Option` is what says so rather than a word an
    /// operator is free to name a Set. Then something saves what is playing,
    /// and from that point the answer to *what is this slot running* may be a
    /// Set — but the versions already filed were versions of what the slot was
    /// running when they were written, and nothing renames a file this module
    /// has already written.
    #[test]
    fn a_run_with_no_set_files_under_none_and_a_later_id_does_not_reach_back() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut snaps = Snapshots::new(tmp.path());
        let before = snaps
            .record(0, "L4", 0, None, "flares", b"before")
            .expect("record")
            .expect("new");
        let after = snaps
            .record(0, "L4", 0, Some("kept_take"), "flares", b"after")
            .expect("record")
            .expect("new");

        // The name a run with no Set writes is the name it has always written,
        // which is what keeps every version recorded before there was a field
        // readable.
        assert!(
            !before
                .file_name()
                .expect("named")
                .to_string_lossy()
                .contains('@'),
            "a version with no Set was filed under one: {}",
            before.display()
        );

        let listing = list(tmp.path(), 100).expect("listed");
        let row = |file: &Path| {
            listing
                .versions
                .iter()
                .find(|v| v.file == file)
                .unwrap_or_else(|| panic!("no row for {}: {listing:?}", file.display()))
        };
        assert_eq!(
            row(&before).set,
            None,
            "the earlier version was re-filed under the Set a later save made: {listing:?}"
        );
        assert_eq!(row(&after).set.as_deref(), Some("kept_take"), "{listing:?}");
        // The bytes each row points at are still each row's, which is what
        // says the two are versions and not one row rewritten.
        assert_eq!(std::fs::read(&before).expect("read"), b"before");
        assert_eq!(std::fs::read(&after).expect("read"), b"after");
    }

    /// A Set id reaches this from a name an operator typed, and `mcp`'s
    /// `checked_id` is not on every route in. A path component is not where to
    /// find out that the two disagree about what an id is.
    #[test]
    fn a_set_id_cannot_escape_the_history_directory() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut snaps = Snapshots::new(tmp.path());
        let path = snaps
            .record(0, "L1", 0, Some("../../etc"), "field", b"x")
            .expect("record")
            .expect("new");

        assert!(
            path.starts_with(tmp.path().join(DIR)),
            "{} escaped the history root",
            path.display()
        );
        assert!(!path.to_string_lossy().contains(".."), "{}", path.display());
    }

    /// **The cap bounds the walk, and the listing says it stopped.**
    ///
    /// The history grows with every save an operator makes and nothing prunes
    /// it, so a listing on a path an operator can press has to have a price
    /// (P-0091). The bound is on **days opened**: with nothing asked for,
    /// nothing is opened, which is what the `unclaimed` count proves here —
    /// the stray file in the day directory is only seen by a walk that went in.
    #[test]
    fn the_cap_bounds_the_walk_and_a_short_listing_says_so() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut snaps = Snapshots::new(tmp.path());
        for name in ["v1", "v2", "v3"] {
            snaps
                .record(0, "L4", 0, None, name, name.as_bytes())
                .expect("record")
                .expect("new");
        }
        let day = tmp
            .path()
            .join(DIR)
            .join(chrono::Local::now().format("%Y/%m/%d").to_string());
        std::fs::write(day.join("notes.txt"), "mine").expect("write");

        let whole = list(tmp.path(), 3).expect("listed");
        assert_eq!(whole.versions.len(), 3, "{whole:?}");
        assert!(
            !whole.stopped_short,
            "a listing that reached the end said it had not: {whole:?}"
        );
        assert_eq!(whole.unclaimed, 1, "{whole:?}");

        let cut = list(tmp.path(), 2).expect("listed");
        assert_eq!(cut.versions.len(), 2, "{cut:?}");
        assert!(
            cut.stopped_short,
            "a truncated listing reads as the whole history: {cut:?}"
        );
        assert_eq!(
            cut.versions,
            whole.versions[..2],
            "the cap kept the wrong end — a walk starts at the newest"
        );

        let none = list(tmp.path(), 0).expect("listed");
        assert!(none.versions.is_empty(), "{none:?}");
        assert!(none.stopped_short, "{none:?}");
        assert_eq!(
            none.unclaimed, 0,
            "the day directory was read for a listing that asked for nothing: {none:?}"
        );
    }
}
