use super::*;

/// What the store holds, summarised: every Set in it, most recent first, with
/// what each one is made of.
///
/// # It asks `setfile::summarise` and not `Store::list_sets`, and that is the
/// whole of what the filter fields needed
///
/// This used to be `Store::list_sets().map(|entry| entry.id)` — a directory
/// read and a name per file — and `view::LibraryBay` said beside the two
/// undrawn filter fields that *"nothing in the store answers it: `list_sets`
/// reads names off a directory and no index anywhere says what a Set holds"*.
/// The second half of that was already wrong:
/// [`karakuri_environment::setfile::summarise`] reads what each Set holds, node
/// by node, and `karakuri-environment`'s MCP `list_sets` has been narrowing on
/// it. So the reading moves here, one Set file per row on top of the directory
/// read, and [`narrows`] is the same retain the tool does.
///
/// Most recent first, which `Store::list_sets` is not.
/// `docs/manual/operations.html`'s row says the listing is most recent first
/// and the MCP tool sorts for it; the bay was showing ascending id, so the
/// panel and the tool answered one operation two ways. The tie-break is the id
/// and it is not decoration — two Sets written inside one tick of a coarse
/// filesystem clock carry the same mtime, and a sort whose keys tie leaves
/// whatever order the entries arrived in.
///
/// # Read once, and by the side of the seam that may read a disk
///
/// Called from `resumed`, before the first frame. A listing is a directory read
/// and a frame path does not do those
/// ([P-0091](../../../docs/principles/0091-cost-is-known-before-it-is-paid.md)),
/// and nothing in `karakuri-console`'s `src/` could do it anyway: opening a
/// store is `karakuri-store`'s and the crate depends on neither it nor the
/// engine (ADR-0156). What crosses into the console is a list of names.
///
/// The cost of reading it once is that a Set saved while this window is up does
/// not appear in the bay until the next run. That is this program's limitation
/// and not the console's — the field is rewritable per frame like every other
/// one — and closing it wants a reason to re-read rather than a timer, which is
/// a decision and not this pass's.
///
/// # A missing store is listed as nothing, and is not created
///
/// [`karakuri_store::store::Store::open`] *"establishes the store layout under
/// `root`, creating any directories that do not exist yet"*, which is the right
/// thing for a program that is about to write one and the wrong thing for one
/// that only wants to read. A program that listed a library by first making one
/// would change the directory it was run in, so the root is required to be
/// there already.
///
/// Either way the answer is a list, and an empty one is a bay with nothing in
/// it — which is what `view::library` draws for it, and is honest: a store this
/// run could not read holds nothing it can name.
pub(crate) fn procedures(root: &std::path::Path) -> Vec<ListedProcedure> {
    if !root.is_dir() {
        // Said once by `library` beside it on the same press, so this one is
        // quiet: two lines about one missing store would be one fact said
        // twice.
        return Vec::new();
    }
    let store = match Store::open(root) {
        Ok(store) => store,
        Err(e) => {
            println!("library: {} could not be opened: {e}", root.display());
            return Vec::new();
        }
    };
    let listed = match store.list_procedures() {
        Ok(listed) => listed,
        Err(e) => {
            // Said rather than swallowed, for [`library`]'s reason: a tier
            // that is empty because a directory could not be read looks
            // exactly like a tier nobody has kept into.
            println!(
                "library: {}/{} could not be listed: {e}",
                root.display(),
                Store::PROCEDURES
            );
            return Vec::new();
        }
    };
    let mut out: Vec<ListedProcedure> = listed
        .into_iter()
        .filter_map(|entry| {
            // **One small read per row, and it compiles nothing** — the badge
            // is the file's `kind` line, which `history::declared_kind` scans
            // (ADR-0338, P-0091). A file that will not read is dropped rather
            // than drawn: it is a fact about this disk at this instant and
            // there is nothing to put in a row.
            let source = store.read_procedure(&entry.name).ok()?;
            Some(ListedProcedure {
                name: entry.name,
                written: entry.written,
                kind: karakuri_environment::history::declared_kind(&source).and_then(kind_of),
            })
        })
        .collect();
    out.sort_by(|a, b| b.written.cmp(&a.written).then_with(|| a.name.cmp(&b.name)));
    out
}

/// The `kind L5` procedures of the library, as the Master bay's `+ add` offers
/// them: the address a slot of each is named by, the name the library lists it
/// under, and whether the procedure declares `retains`.
///
/// Both tiers, in the order the bay would list them — what the operator has kept
/// first and then what ships, which is `library`'s own order over the two.
///
/// One read and one check per `kind L5` row, and none for any other: the `kind`
/// badge is already on the row, so this pays only for the files it is going to
/// offer (P-0091). A file that will not read or will not check is dropped
/// rather than offered.
fn chain_offers(
    store: &std::path::Path,
    kept: &[ListedProcedure],
    shipped: &[karakuri_environment::places::PresetProcedure],
) -> Vec<view::AddChoice> {
    let l5 = |kind: Option<karakuri_operation::Layer>| kind == Some(karakuri_operation::Layer::L5);
    let opened = Store::open(store).ok();
    let mut out: Vec<view::AddChoice> = Vec::new();
    for entry in kept.iter().filter(|entry| l5(entry.kind)) {
        let Some(source) = opened
            .as_ref()
            .and_then(|store| store.read_procedure(&entry.name).ok())
            .and_then(|bytes| String::from_utf8(bytes).ok())
        else {
            continue;
        };
        if let Some((procedure, retains)) = karakuri_environment::mix::l5_offer(&source) {
            out.push(view::AddChoice {
                procedure,
                words: entry.name.clone(),
                retains,
            });
        }
    }
    for entry in shipped
        .iter()
        .filter(|entry| l5(entry.kind.and_then(kind_of)))
    {
        let Ok(source) = std::fs::read_to_string(&entry.file) else {
            continue;
        };
        if let Some((procedure, retains)) = karakuri_environment::mix::l5_offer(&source) {
            out.push(view::AddChoice {
                procedure,
                words: entry.name.clone(),
                retains,
            });
        }
    }
    out
}

/// What a Set's row is: the layers its own `slot` records fill, once each and
/// in the file's own order.
///
/// Read off the summary the listing already has, which is why a badge costs
/// nothing beyond what `all` was already paying: `setfile::summarise` reads a
/// line per node to answer *what is in my library*, and the layer is one of the
/// fields it already carries (P-0091, ADR-0338).
///
/// Once each, because a badge says *this Set fills that layer* and a Set of
/// three renderers fills L4 once for the purpose of reading a row.
pub(crate) fn set_row(set: &setfile::SetSummary) -> RowKind {
    let mut badges: Vec<karakuri_operation::Layer> = Vec::new();
    for node in &set.nodes {
        let layer = asked(node.layer);
        if !badges.contains(&layer) {
            badges.push(layer);
        }
    }
    RowKind {
        badges,
        procedure: false,
    }
}

/// What a kept procedure's row is: its one declared kind, and that it is a
/// procedure. A file that declares none draws no badge.
pub(crate) fn kept_row(kept: &ListedProcedure) -> RowKind {
    RowKind {
        badges: kept.kind.into_iter().collect(),
        procedure: true,
    }
}

/// [`kept_row`] one tier along, for a procedure that ships.
pub(crate) fn shipped_row(shipped: &karakuri_environment::places::PresetProcedure) -> RowKind {
    RowKind {
        badges: shipped.kind.and_then(kind_of).into_iter().collect(),
        procedure: true,
    }
}

/// Does this procedure pass the kind row?
///
/// `LibraryKinds::shows_layer` answers it for a file that declares a kind, and
/// this adds the one case that value cannot carry: a `.kir` with no `kind` line
/// shows only while nothing is on. It is a row of the library either way —
/// dropping it would answer *what have I kept* with a file missing — and a chip
/// that named it would be a chip claiming it is of a kind nobody wrote.
pub(crate) fn shows_kept(
    kinds: karakuri_operation::LibraryKinds,
    kind: Option<karakuri_operation::Layer>,
) -> bool {
    match kind {
        Some(layer) => kinds.shows_layer(layer),
        None => !kinds.narrowing(),
    }
}

/// What the presets root ships that can go over a layer, as its rows —
/// [`presets_listing`]'s shape one extension along and with its failures.
pub(crate) fn presets_procedures(
    presets: Option<&karakuri_environment::places::Presets>,
) -> Vec<karakuri_environment::places::PresetProcedure> {
    let Some(presets) = presets else {
        return Vec::new();
    };
    match presets.list_procedures() {
        Ok(procedures) => procedures,
        // **Said out loud and then empty**, which is [`presets_listing`]'s own
        // answer beside it: a tier that is empty because a directory could not
        // be read looks exactly like one that ships nothing.
        Err(why) => {
            println!("library: {why}");
            Vec::new()
        }
    }
}

/// One procedure the operator has kept, as [`procedures`] found it: the name
/// its row is drawn under, when it was written, and the layer it declares.
///
/// `ListedProcedure` and not `Kept`, which is taken: [`Kept`] is what a press
/// on the Inspector's `keep` capsule *files*, and this is a row of the listing
/// that files show up in. One is an act and the other is a listing, and a name
/// over both would be a name meaning two things.
///
/// `kind` is an `Option` and a `None` is a row, which is
/// `places::PresetProcedure::kind`'s own note one tier along: a `.kir` with no
/// `kind` line is a file somebody kept, and dropping it from the listing would
/// answer *what have I kept* with a file missing and nothing said. It draws no
/// badge, and a load off it is refused by name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ListedProcedure {
    pub(crate) name: String,
    pub(crate) written: std::time::SystemTime,
    pub(crate) kind: Option<karakuri_operation::Layer>,
}

pub(crate) fn library(root: &std::path::Path) -> Vec<setfile::SetSummary> {
    if !root.is_dir() {
        println!(
            "library: no store at {}, so the bay lists nothing",
            root.display()
        );
        return Vec::new();
    }
    let sets = Store::open(root).and_then(|store| setfile::summarise(&store));
    match sets {
        Ok(mut sets) => {
            sets.sort_by(|a, b| b.written.cmp(&a.written).then_with(|| a.id.cmp(&b.id)));
            sets
        }
        Err(e) => {
            // Said rather than swallowed, for the reason every other failure
            // in this file is said: a bay that is empty because the store
            // could not be read looks exactly like a bay that is empty
            // because the store is.
            println!("library: {} could not be listed: {e}", root.display());
            Vec::new()
        }
    }
}

/// Which Sets this store has starred, as their ids — the other half of what the
/// Library bay lists, and the half `my sets` *is* (ADR-0299).
///
/// # It is [`library`]'s shape one file along, and its failures are the same
///
/// A store that is not there holds no stars, a store that will not open is said
/// out loud rather than answered with silence, and either way the answer is a
/// set — because a scope that is empty because a file could not be read looks
/// exactly like one that is empty. The one difference from [`library`] is that
/// a missing `favourites.json` is not a failure at all: `Store::favourites`
/// answers an empty set for it, which is a store nobody has starred in.
///
/// Nothing is pruned against the listing here. A mark whose Set a hand removed
/// from `sets/` stays in the file — that is `Store::favourites`' own rule and
/// ADR-0299's — and what makes it harmless is that [`listing`] takes the
/// *intersection* with what the store holds, so an id naming no Set draws no
/// row and can still have its star taken off.
///
/// # Off the frame, on the press that builds a listing
///
/// One file read, beside the directory read [`library`] already does
/// ([P-0091](../../../docs/principles/0091-cost-is-known-before-it-is-paid.md)):
/// at startup and on every press that re-lists, which is a scope, a filter and
/// a star. `karakuri-console` reaches no disk at all (ADR-0156), so what
/// crosses the seam is the set of ids.
pub(crate) fn favourites(root: &std::path::Path) -> std::collections::BTreeSet<String> {
    if !root.is_dir() {
        return std::collections::BTreeSet::new();
    }
    match Store::open(root).and_then(|store| store.favourites()) {
        Ok(starred) => starred,
        Err(e) => {
            println!(
                "library: {} keeps no readable favourites: {e}",
                root.display()
            );
            std::collections::BTreeSet::new()
        }
    }
}

/// Does this Set pass the filter row? — the same retain
/// `karakuri-environment`'s MCP `list_sets` applies, and deliberately so: one
/// operation narrowed two ways is two answers to *what does this store hold*.
///
/// A Set matches, not a node. With both filters given the question is *which of
/// the Sets that hold this also have something on that layer*, so each half is
/// answered against the whole Set — a Set whose `drift_shell` is a geometry and
/// whose deformation is called something else is exactly what that question is
/// looking for.
///
/// `holds` is matched case-insensitively against what each node is called,
/// because an operator who read `drift_shell` in one row and stepped to
/// `Drift_Shell` in another is not asking a different question. The fields step
/// through the names as the store spells them, so today the fold changes no
/// answer; it is here because the tool's does and the two must not come apart
/// the day either field takes letters.
pub(crate) fn narrows(
    set: &setfile::SetSummary,
    holds: Option<&str>,
    layer: Option<karakuri_operation::Layer>,
) -> bool {
    let held = holds.map(str::to_ascii_lowercase);
    held.as_ref().is_none_or(|holds| {
        set.nodes
            .iter()
            .any(|node| node.name.to_ascii_lowercase().contains(holds))
    }) && layer.is_none_or(|layer| {
        let want = recorded(layer);
        set.nodes.iter().any(|node| node.layer == want)
    })
}

/// What the `holds` field can be stepped to: every name a node in this store's
/// Sets carries, once each and in one order.
///
/// The unnarrowed listing's, which is `view::View::holds`' own instruction and
/// the reason this takes the summaries before [`narrows`] has been near them:
/// candidates read off a filtered listing shrink as the filter bites, and a
/// step would then wander somewhere it could not come back from.
///
/// Sorted, and not in the order the Sets were written. The candidates are a
/// list somebody steps through one press at a time, so the order has to hold
/// still while they do it — where the rows above are ordered by recency and
/// move whenever anything is saved. `BTreeSet` is the sort and the
/// deduplication in one pass.
pub(crate) fn holds_choices(sets: &[setfile::SetSummary]) -> Vec<String> {
    sets.iter()
        .flat_map(|set| set.nodes.iter().map(|node| node.name.clone()))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect()
}

/// What the filter row is narrowing to, in words, or `None` where it is
/// narrowing nothing.
///
/// The console draws the two values and this says what they mean, which is the
/// division every other readout on this panel makes: a field reads `L4` and a
/// line says the listing under it is the Sets that hold one.
///
/// `layer` arrives already spelled, out of `view::Filters::layer_word` — the
/// word the field itself is drawing. A `{:?}` here would print `Field` where
/// the field reads `FIELD`, which is a readout and a sentence about it
/// disagreeing in the one place a reader can see both.
pub(crate) fn narrowing(holds: Option<&str>, layer: Option<&str>) -> Option<String> {
    match (holds, layer) {
        (None, None) => None,
        (Some(holds), None) => Some(format!("holding a node called `{holds}`")),
        (None, Some(layer)) => Some(format!("holding a {layer} node")),
        (Some(holds), Some(layer)) => Some(format!(
            "holding both a node called `{holds}` and a {layer} node"
        )),
    }
}

/// One row of a scope whose rows are files: the word the bay draws and the file
/// behind it.
///
/// Two fields because the bay lists names and a take-in needs a path: what
/// crosses into the console is a `String` per row (`view::View::library`), and
/// what this program has to be able to find again on the press is the file that
/// row came off.
///
/// Two scopes have rows of this kind — `presets`, which is a told directory
/// (ADR-0230), and `folder`, which is one somebody dropped on this window
/// (ADR-0275). They are one type because a row of either is a Set file that is
/// not in this store yet and a press on it is the same two operations
/// (`docs/manual/operations.html`'s *Send a Set to somebody, and take one in*):
/// the difference between them is which directory was listed, which is
/// [`Taking`]'s.
pub(crate) struct FileRow {
    /// What the row reads, which is the file's own name without its extension. Not
    /// read out of the file: a listing that opened twenty-three files to draw
    /// twenty-three rows would be a directory read doing a file read's work, and
    /// the id a take-in files the Set under is the one *inside* the file anyway —
    /// read there, on the press, by [`taking_in`].
    pub(crate) id: String,
    pub(crate) path: std::path::PathBuf,
}

/// Every Set the preset library offers, which is the `.kset` files in the root
/// this run resolved.
///
/// # One call, and the reading is not this program's
///
/// The listing is `karakuri_environment::places`', beside the resolution that
/// answers *where* the presets are: what a `.kset` is and which directory holds
/// them is that module's business, and a second program wanting the same list
/// must not read the same directory a second way. So this is the one place in
/// this program that knows a preset library can be listed at all, and it knows
/// nothing about how — the shape of a row, the order they come in, and what a
/// name the layout does not claim does are all answered there.
///
/// # What it lists, and why not the `.kir` files beside them
///
/// `console.html`'s *A folder scope reads Sets, and a bundle is not a third
/// thing* settles it: *"A directory of `.kir` files is a directory of parts,
/// and a library lists what you can put on a deck."* A `.kir` is one node's
/// source addressed by its content and nothing in the vocabulary takes one, so
/// the parts are not rows — they are what the rows *name*.
///
/// A root with nothing in it is a library nobody has filled, and it is not a
/// failure: the scope lists nothing and the sentence about it is
/// [`why_nothing`]'s. A directory that will not open is said out loud, for
/// [`library`]'s reason one scope along — a scope empty because a directory
/// could not be read looks exactly like one that is empty.
pub(crate) fn presets_listing(
    presets: Option<&karakuri_environment::places::Presets>,
) -> Vec<FileRow> {
    let Some(presets) = presets else {
        return Vec::new();
    };
    match presets.list_sets() {
        Ok(sets) => sets
            .into_iter()
            .map(|set| FileRow {
                id: set.id,
                path: set.file,
            })
            .collect(),
        // **Said out loud and then empty**, which is the same shape the store
        // side takes one scope along: a root that will not open looks exactly
        // like a root nobody has filled, and the difference has to be spoken
        // or it is not there. The sentence is `places`' own — it names the
        // path and how that path was arrived at — so an operator who typed
        // `--presets` reads something different from one whose checkout moved.
        Err(why) => {
            println!("presets: {why}");
            Vec::new()
        }
    }
}

/// Every Set a dropped folder holds, which is the Set files directly in the
/// directory this bay was pointed at (ADR-0275).
///
/// # What it lists, and why both spellings
///
/// `console.html`'s *A folder scope reads Sets, and a bundle is not a third
/// thing*: *"A Set file and a bundle are the same file … so the scope draws one
/// kind of row rather than two"*, and the authored form that names its parts by
/// relative path *"is a Set file, is one row, and is taken in by the same
/// operation."* So both suffixes are listed and neither is a second kind of row
/// — `Store::SET_FILE_SUFFIX` for the resolved form and
/// `setfile::AUTHORING_SUFFIX` for the authored one, borrowed from the modules
/// that own them rather than spelled here. A `.kir` is not listed: it is one
/// node's source, nothing in the vocabulary takes one, and *"a directory of
/// `.kir` files is a directory of parts"*.
///
/// A directory that holds `night.kset` and `night.kbset` lists two rows reading
/// `night`, and that is deliberate rather than got to by accident: they are two
/// files, each of which is a Set, and choosing between them here would be this
/// listing inventing a precedence between the two forms. Whichever of them a
/// press means is the take-in's question and it is answered by refusing:
/// [`Taking::file`] finds the row by the word that was pressed, two files wear
/// that word, and a press that took one of them would be picking for the
/// operator between two rows they cannot tell apart on screen. See there, where
/// the refusal names both files.
///
/// # Ascending, one directory deep, and the name is all that is read
///
/// `places::Presets::list_sets`' three rules, carried over for its reasons:
/// `read_dir` hands back no order at all, a name the layout does not claim is
/// skipped rather than repaired, and nothing here opens a file — a malformed
/// Set is a refusal at the moment it is taken in, where the operator can see
/// which row they pressed. It is not [`library`]'s most-recent-first
/// (ADR-0263): that order is a store's, where a Set's time is when the operator
/// wrote it, and a folder full of files somebody copied has mtimes that are
/// facts about this machine's disk.
///
/// # Where this belongs, and it is not here
///
/// `karakuri_environment::places` is the right home, beside
/// `Presets::list_sets`, which answers the same question about a directory this
/// run was told about rather than one it was handed: this is that function with
/// two suffixes and no `Found` behind it, and a second walk of a directory of
/// Sets is a second answer to *what is a Set file called*. It is here because
/// ADR-0275's owed work was this file's, and the reason is written down rather
/// than left to be inferred — `declared`'s own shape one bay over.
pub(crate) fn folder_listing(dir: Option<&std::path::Path>) -> Vec<String> {
    folder_files(dir).into_iter().map(|row| row.id).collect()
}

/// The same walk with the file names kept, which is what a take-in needs: the
/// bay lists words and the press has to find the file the word came off again
/// ([`FileRow`]).
///
/// One walk and not two, which is why [`folder_listing`] is a `map` over this
/// rather than a second `read_dir`: a listing the bay drew and a listing the
/// press searched that disagreed would be a press acting on a row nobody saw.
/// It is [`presets_listing`]'s shape one scope along, and that function answers
/// `FileRow`s for the same reason.
pub(crate) fn folder_files(dir: Option<&std::path::Path>) -> Vec<FileRow> {
    let Some(dir) = dir else {
        return Vec::new();
    };
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        // **Said out loud and then empty**, which is `presets_listing`'s shape
        // one scope along and for its reason: a folder that will not open
        // looks exactly like a folder holding no Sets, and the difference has
        // to be spoken or it is not there. A dropped directory can go between
        // the drop and a press an hour later — it is somebody else's
        // directory, which this program neither made nor writes.
        Err(why) => {
            println!("  folder: `{}` could not be listed: {why}", dir.display());
            return Vec::new();
        }
    };
    let mut out: Vec<(String, std::ffi::OsString)> = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name();
        // Non-UTF-8 falls out of the listing with everything else the layout
        // does not claim — `Store::list_sets`' own rule, no lossy repair.
        let Some(id) = name.to_str().and_then(|name| {
            name.strip_suffix(Store::SET_FILE_SUFFIX)
                .or_else(|| name.strip_suffix(karakuri_environment::setfile::AUTHORING_SUFFIX))
        }) else {
            continue;
        };
        // A subdirectory named like a Set file belongs to whoever made it.
        if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            continue;
        }
        out.push((id.to_owned(), name));
    }
    out.sort();
    out.into_iter()
        .map(|(id, name)| FileRow {
            id,
            path: dir.join(name),
        })
        .collect()
}

/// Why the scope that is marked lists nothing, in the words that say which
/// kind of nothing it is — and among these four chips there are two kinds.
///
/// Two of them are empty as *data*: a store nobody has saved into and a preset
/// root nobody has filled are libraries with nothing in them, which
/// `console.html` says outright — *"An empty tier is a library nobody has
/// filled rather than something gone wrong."* Fill either and the rows appear
/// with nothing else changing.
///
/// `my sets` is a third kind, and it is neither of those: the store may
/// hold plenty and nothing be starred, which is a listing that is empty
/// because of an answer rather than because of an absence (ADR-0299). What to
/// do about it is press a star, and the sentence says so.
///
/// The last of them is empty as *machinery*, and it is the one that
/// changed:
///
/// - A folder nobody has pointed anywhere is empty for want of a gesture,
///   and that is the one of the four that changed on 2026-09-08. It used to be
///   empty for want of machinery — this said *"nothing here reads a folder
///   dropped on this window yet"* — and [`folder_dropped`] is that machinery.
///   `Operation::ListSets` still has nowhere to put a directory and should
///   not: both its fields narrow what a store already holds, and which store
///   is asked at all is [`listing`]'s own answer. So this scope has two
///   sentences and `pointed` is which: no folder has been dropped yet, or
///   one has and holds no Set file. The second is [`Scope::AllSets`]' kind of
///   nothing — a library nobody has filled — read in somebody else's
///   directory.
///
/// It is said out loud on the step and again on a press, because a scope that
/// went quiet and a scope that is empty are the same experience — which is the
/// rule every other refusal in this file is written to.
///
/// The fifth is a third kind again, and it has two sentences of its own.
/// `history` lists the versions of the Set the load pulldown's deck is
/// running, so it can be empty because that deck is running *no Set* — a run
/// launched on a pair somebody typed, whose versions are filed under none
/// (ADR-0276, ADR-0304) — or because the Set it is running has not been
/// edited yet. The first is the one worth spelling out: a listing narrowed to
/// a Set matches a `None` row not at all rather than matching every one of
/// them, so *nothing here* is the true answer and not a filter that misfired.
///
/// `pointed` is whether the bay has a directory at all and `running` is
/// whether the pulldown's deck names a Set; each is read by one arm only, and
/// the other four are the same sentence whatever this window has been dropped
/// on and whatever any deck is playing.
pub(crate) fn why_nothing(scope: Scope, pointed: bool, running: bool) -> &'static str {
    match scope {
        Scope::AllSets => {
            "this store holds no Sets yet, which is a library nobody has filled: `k` keeps \
             what a deck is playing, and loading a preset leaves one here too"
        }
        Scope::MySets => {
            "nothing in this store is starred — `my sets` is the starred subset of `all` \
             and never the listing of it, so press the star at the left of a row under \
             `all` and that Set appears here"
        }
        Scope::Presets => {
            "this run found no preset library, or the one it found holds no `.kset` file — \
             `--presets DIR` is what names one, and a directory of `.kir` parts is not a \
             library"
        }
        Scope::Folder if !pointed => {
            "no folder has been dropped on this window yet — drag one off the desktop and \
             let go of it anywhere over this window, and this scope lists the Sets in it"
        }
        Scope::Folder => {
            "the folder this bay is pointed at holds no Set file, which is a directory \
             nobody has put one in: a folder scope lists `.kbset` and `.kset` files, and a \
             directory of `.kir` parts is not a library"
        }
        Scope::History if !running => {
            "the deck the `load` pulldown names is playing a typed pair rather than a Set, so \
             its versions are filed under no Set at all and a listing narrowed to one matches \
             none of them — aim that pulldown at a deck you have loaded a Set onto, or load \
             one"
        }
        Scope::History => {
            "this Set has no versions yet — every write that compiles is kept, so edit one of \
             its nodes, or let a model write one, and the version it replaced is the first row \
             here"
        }
    }
}

/// What a folder over this window reads in the `.path` row, written into the
/// console for the pass that is about to draw it.
///
/// The one thing about this bay that is a frame's business, and it is `egui`'s
/// doing rather than a choice here: `RawInput::take` *clones* `hovered_files`
/// where it *moves* `dropped_files`, so a drag over the window is a fact about
/// every pass while it lasts and there is no event to hang it off. Nothing is
/// asked of the file system for it — whether the path is a folder is the drop's
/// question (P-0091, ADR-0275) — and nothing is allocated on a pass where the
/// answer has not changed.
///
/// More than one path over the window reads as none. The row says *"the path a
/// release would set"*, a release sets nothing where two arrived (ADR-0275),
/// and drawing the first of them would be this row picking one out of a list
/// the desktop happened to build — which is the choice the refusal below exists
/// to refuse.
pub(crate) fn folder_over(view: &mut View, hovering: &[karakuri_console::egui::HoveredFile]) {
    let over = match hovering {
        [one] => one.path.as_deref(),
        _ => None,
    };
    // **Compared before it is written**, so a drag held still over the window
    // costs nothing per pass. `to_string_lossy` borrows for a path that is
    // UTF-8, which every path drawn here is in practice, and it is the same
    // conversion `Path::display` makes — so what is compared is what would be
    // drawn rather than an approximation of it.
    let same = match (over, view.incoming.as_deref()) {
        (None, None) => true,
        (Some(over), Some(shown)) => *over.to_string_lossy() == *shown,
        _ => false,
    };
    if !same {
        view.incoming = over.map(|path| path.to_string_lossy().into_owned());
    }
}

/// A folder let go on this window, which is how the `folder` scope is given a
/// directory — and the three answers ADR-0275 settles, in the words that record
/// and `console.html` write.
///
/// # It is done here, on the pass the drop arrives on, and nothing is put by
///
/// `dropped_files` is visible for exactly one pass and then gone
/// (`RawInput::take` moves it), so the release does the whole thing rather than
/// asking a question: it sets the directory and marks the `folder` chip, and
/// the listing under it is the next thing drawn. A bay that had put the path
/// aside and waited for the chip to be pressed would be waiting on an operator
/// who has already made the gesture, holding a path nothing will hand it a
/// second time.
///
/// So the file system is asked here, on a frame, which is the one place this
/// program does that and it is P-0091's rule rather than an exception to it:
/// what is asked once is asked once, and a drop is one act. A drop is also the
/// only moment the question can be asked at all — the event carries a path and
/// nothing else, and *"a file and a directory are indistinguishable at the
/// event"* (ADR-0275).
///
/// # The two refusals, and each is a policy the plumbing does not answer
///
/// A path that is not a directory is refused, naming what was dropped. A file
/// is not read as the folder it sits in — that would point this bay at a
/// directory nobody pointed at, which is the mistake the carry one bay over
/// refuses when it declines to snap a drop mark to the nearest strip (ADR-0273)
/// — and a `.kbset` is not taken in where it fell, because taking a Set in is a
/// press on a row of a listing and a file landing on this window has no row
/// under it.
///
/// More than one path is refused, and all of them are. A multi-item drag
/// arrives whole, so three folders let go together are three entries in one
/// pass and not three drops: there is no first to act on and a rest to ignore,
/// nothing says which was aimed at, and the order is the desktop's rather than
/// the operator's. The bay keeps the directory it had and the refusal counts
/// what arrived.
///
/// Both name what arrived and what to do instead (P-0083), and both say where
/// this library is still pointed — because a refusal that left an operator
/// wondering whether the bay had moved anyway is a refusal that costs a second
/// gesture to read.
///
/// `None` where nothing was dropped, which is every pass but one.
pub(crate) fn folder_dropped(
    view: &mut View,
    folder: &mut Option<std::path::PathBuf>,
    store: &std::path::Path,
    presets: Option<&karakuri_environment::places::Presets>,
    dropped: &[&std::path::Path],
) -> Option<String> {
    // **Where the bay is still pointed**, read before anything moves, because
    // both refusals say it and the accept below replaces it.
    let kept = match folder.as_deref() {
        Some(at) => format!("This library is still pointed at `{}`.", at.display()),
        None => String::from("This library is still pointed nowhere."),
    };
    let one = match dropped {
        [] => return None,
        [one] => *one,
        many => {
            return Some(format!(
                "  folder: {} paths were let go together and none of them was taken — a drop \
                 tells this window a path and never a place, so nothing says which of them was \
                 aimed at and the order is the desktop's rather than yours. Let go of one \
                 folder on its own. {kept}",
                many.len()
            ))
        }
    };
    // **Asked once, here.** `is_dir` would answer `false` for a path that
    // cannot be examined at all, which is a different thing and is said as one.
    match std::fs::metadata(one) {
        Err(why) => Some(format!(
            "  folder: `{}` could not be examined ({why}), so whether it is a folder is not \
             known and nothing was taken. {kept}",
            one.display()
        )),
        Ok(what) if !what.is_dir() => Some(match set_file(one) {
            true => format!(
                "  folder: `{}` is a Set file and what this takes is a folder — a Set is taken \
                 in by pressing its row in a listing, and a file let go on this window has no \
                 row under it. Let go of the folder that holds it and press the row. {kept}",
                one.display()
            ),
            false => format!(
                "  folder: `{}` is not a folder and what this takes is one — this bay is \
                 pointed at a directory and lists the Sets in it, so let go of the folder that \
                 holds it rather than the file itself. {kept}",
                one.display()
            ),
        }),
        Ok(_) => {
            *folder = Some(one.to_path_buf());
            // **The line the bay draws is spelled here**, which is
            // `View::library`'s seam one row up: this side reads the disk and
            // the panel is handed what to draw (ADR-0156).
            view.folder = Some(one.display().to_string());
            // **And the chip is marked in the same act**, which is the whole
            // of *the drop is recorded on the frame it is seen on*: the
            // directory and the mark are one gesture's outcome, and a bay
            // pointed at a folder it is not showing would be waiting for a
            // press nobody owes it.
            view.select_scope(Scope::Folder);
            // **Asked of the mark rather than of the call**, because
            // `select_scope` answers *whether it moved* and a second drop
            // while `folder` is already marked moves nothing: the sentence is
            // about where the mark **is**. `false` here is a console handed no
            // `folder` chip, which cannot mark one — the row is still drawn,
            // since it is where a send's save dialog opens whichever scope is
            // marked (ADR-0311), and the listing under it is whatever scope this
            // console does have.
            let marked = match view.scope() == Some(Scope::Folder) {
                true => "the `folder` chip is marked",
                false => "this console draws no `folder` chip, so nothing is marked",
            };
            // **`None`, and it cannot be anything else here**: this drop has
            // just marked the `folder` chip, so the listing being re-asked is
            // a directory's and never a history's, and a Set id handed in
            // would be a value nothing reads.
            let said = listing(view, store, presets, folder.as_deref(), None);
            // **And the cursor goes back to the top of a listing it has never
            // seen**, which is `View::select_scope`'s own rule reached the
            // other way: that method resets the cursor when the *mark* moves,
            // and a second drop while `folder` is already marked moves the
            // listing without moving the mark. Left where it was it would
            // point at the fifteenth row of a directory of three — a Set
            // nobody chose, sitting under a pill that says a press will load
            // it.
            view.point_at(0);
            Some(format!(
                "  folder: this library is pointed at `{}` — {marked} and the listing under it \
                 is what that directory holds\n{said}",
                one.display(),
            ))
        }
    }
}

/// Whether a name is one a Set file wears, which is the two suffixes
/// [`folder_listing`] lists and is asked here for one reason: an operator who
/// let go of a `.kbset` on this window was trying to take a Set in, and the
/// refusal owes them the press that does it (P-0083).
///
/// It is a name and not a reading: nothing is opened, exactly as nothing is
/// opened to draw a row.
pub(crate) fn set_file(path: &std::path::Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            name.ends_with(Store::SET_FILE_SUFFIX)
                || name.ends_with(karakuri_environment::setfile::AUTHORING_SUFFIX)
        })
}

/// The rows the Library bay lists for the scope that is marked, written into
/// the view, and the sentence to print about it.
///
/// # One function, and it is what a scope *is* on this program's side
///
/// The console draws a row of chips and marks one of them; which listing
/// belongs under that mark is this side's answer, because every one of the four
/// is something outside this process — a store, a told directory, a filter over
/// the first, a directory somebody names during the run — and
/// `karakuri-console` takes none of them (ADR-0156). So the seam is a `Vec` of
/// names, and this is the one place it is filled.
///
/// On the press that changed the scope and at startup, never on a frame. A
/// listing is a directory read (P-0091), which is the same rule [`library`]
/// states one scope down and the reason this is not called from the frame
/// handler.
///
/// All five answer with rows now, and each of the five can still answer with
/// none — [`why_nothing`] is where the sentences are, and it is one function so
/// that a scope which stops being empty stops being empty in one place. Three
/// of them depend on something that happened during the run: `folder` is `None`
/// until somebody drops a directory on this window ([`folder_dropped`]), `my
/// sets` is empty until somebody presses a star ([`favourite`]), and `history`
/// is empty until the deck the load pulldown names is running a Set that has
/// been edited.
///
/// # `history` is the one scope that is not a directory of Sets
///
/// Its rows are the versions of one Set — the one the load pulldown's deck is
/// running, which `running` carries — and they come off
/// `karakuri_environment::history::list`, most recent first, in that function's
/// own order rather than in one applied here (ADR-0263's argument on a
/// different listing).
///
/// The narrowing is a Set and never a deck, which is why `running` is an id
/// rather than a slot: two decks playing one Set have one history between them,
/// and a version is filed under the Set the slot was running (ADR-0304). A
/// `None` row matches no Set rather than matching every one of them — a version
/// written where there was no Set is a version of nothing, and treating it as a
/// wildcard would put another run's edits under whatever Set happens to be
/// loaded now (ADR-0276's own consequence).
///
/// The cap is on the walk and not on the Set. [`HISTORY_MOST`] rows are asked
/// for and the narrowing happens after, so a store whose day directories hold
/// several Sets' versions lists fewer of each; `Listing::stopped_short` is what
/// says the walk stopped with days unread, and it is said out loud beside the
/// count rather than left for the foot's `n of m` to imply.
pub(crate) fn listing(
    view: &mut View,
    store: &std::path::Path,
    presets: Option<&karakuri_environment::places::Presets>,
    folder: Option<&std::path::Path>,
    running: Option<&str>,
) -> String {
    let Some(scope) = view.scope() else {
        return String::from(
            "  library: this console was handed no scopes, so there is no library to list",
        );
    };
    // **The store's own listing is what the filter row narrows**, and that is
    // `Operation::ListSets`'s own scope rather than a shortcut here: the row is
    // *List what the **store** holds*, which is `all` — and `my sets` is that
    // same listing starred (ADR-0299), so both are narrowed by the same retain
    // and neither is a second reading. The other two listings are not the
    // store: `presets` is a told directory of files and `folder` is somebody
    // else's. So the candidates are emptied for them, which is what makes
    // `View::filters` read both fields as unset there rather than the bay
    // hiding rows under a filter it is not applying. The filter comes back with
    // the scope, because the position it is kept as is still there.
    let held = match scope {
        Scope::AllSets | Scope::MySets => library(store),
        _ => Vec::new(),
    };
    // **What the walk found, and what it did not.** Read here rather than
    // inside the arm below so that the sentence about it is written where
    // every other sentence about this listing is; `None` for every scope that
    // is not a history, which is every scope that is a directory of Sets.
    let walked = match scope {
        Scope::History => Some(walked(store, running)),
        _ => None,
    };
    view.holds = holds_choices(&held);
    // **The marks, beside the listing they are a subset of.** They are read on
    // every one of these presses rather than once for the run, because a star
    // is a press that changes them and this is the one place the bay is told
    // what the store says — see [`favourites`], and `view::View::starred`.
    //
    // **Read whichever scope is marked**, because the star is drawn on every
    // row of every listing: a `presets` row is a file this store does not hold
    // and its star is hollow, which is what `Store::set_favourite` refusing an
    // id `sets/` does not hold says at the other end.
    view.starred = favourites(store);
    // **Copied out rather than borrowed across the write below**: `filters`
    // borrows `View::holds`, and the listing is written into the same `View`.
    // **`layer` is `None` and no field sets it any more**, which is ADR-0338:
    // the `layer…` field is retired for the six kind chips, and
    // `Operation::ListSets` keeps the field for `list_sets` and `--list-sets`,
    // where *which Sets hold a node on this layer* now lives. It is kept in the
    // narrowing below rather than deleted from it because that predicate is
    // what the MCP tool applies too.
    let (holds, layer) = {
        let at = view.filters();
        (at.holds.map(str::to_owned), None)
    };
    // **What the operator has kept, and what ships**, which are the two tiers a
    // procedure is a row of (ADR-0227's shape a fourth time, ADR-0338). Read
    // here beside the Sets and on the same press, because the two halves of a
    // scope's listing are one answer: `all` gains `<store>/procedures/` and
    // `presets` gains the presets root's `.kir` files, and no other scope
    // gains either — `my sets` is the starred subset of the Sets and a star is
    // refused on anything else, and a `folder` row is a *take*, which nothing
    // does with a bare `.kir`.
    let kept = match scope {
        Scope::AllSets => procedures(store),
        _ => Vec::new(),
    };
    let shipped = match scope {
        Scope::Presets => presets_procedures(presets),
        _ => Vec::new(),
    };
    let kinds = view.filters().kinds;
    // **One listing of rows and not two lists side by side**, which is
    // ADR-0338's *a procedure is a row of the same list*: the names and what
    // each row is are built together here and split into the two fields the
    // console reads, so a badge can never describe the row above the one it is
    // drawn on.
    let rows: Vec<(String, RowKind)> = match scope {
        Scope::AllSets => {
            let mut rows: Vec<(String, RowKind, std::time::SystemTime)> = held
                .iter()
                .filter(|set| narrows(set, holds.as_deref(), layer))
                .filter(|_| kinds.shows_sets())
                .map(|set| (set.id.clone(), set_row(set), set.written))
                .chain(
                    kept.iter()
                        .filter(|kept| shows_kept(kinds, kept.kind))
                        .map(|kept| (kept.name.clone(), kept_row(kept), kept.written)),
                )
                .collect();
            // **Most recent first, and the name breaks a tie**, which is
            // `library`'s own order applied to the merged list: two files
            // written inside one tick of a coarse clock tie, and a tied sort
            // is not an order.
            rows.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.0.cmp(&b.0)));
            rows.into_iter().map(|(id, kind, _)| (id, kind)).collect()
        }
        // **The starred subset, and it is an intersection rather than a
        // second listing** (ADR-0299): the rows are the store's own, in the
        // store's own order, keeping only the ids the marks name. A mark whose
        // Set is gone draws no row, which is what makes a stale mark a line in
        // a file rather than a hazard — and it can still have its star taken
        // off, because `Store::set_favourite` refuses only the starring.
        //
        // **And no procedure is here**, which is a decision rather than an
        // omission: a star is refused on an id `sets/` does not hold, so there
        // is nothing to star on a procedure row and the starred subset of the
        // Sets is exactly what this chip is (ADR-0338).
        Scope::MySets => held
            .iter()
            .filter(|set| view.starred.contains(&set.id))
            .filter(|set| narrows(set, holds.as_deref(), layer))
            .filter(|_| kinds.shows_sets())
            .map(|set| (set.id.clone(), set_row(set)))
            .collect(),
        // **What ships, in two kinds of file and one list**: the `.kset` files
        // and the `.kir` files beside them, in name order because a shipped
        // file's mtime is when this machine checked it out.
        //
        // **A preset Set row carries no badge**, and the cost is stated where
        // it is paid (P-0091): `Presets::list_sets` opens no file at all, so
        // what layers one fills is not known without reading twenty-two Set
        // files on a press. A procedure's kind *is* known, because that listing
        // reads one small file per row for it.
        Scope::Presets => {
            let mut rows: Vec<(String, RowKind)> = presets_listing(presets)
                .into_iter()
                .filter(|_| kinds.shows_sets())
                .map(|preset| (preset.id, RowKind::default()))
                .chain(
                    shipped
                        .iter()
                        .filter(|shipped| shows_kept(kinds, shipped.kind.and_then(kind_of)))
                        .map(|shipped| (shipped.name.clone(), shipped_row(shipped))),
                )
                .collect();
            rows.sort_by(|a, b| a.0.cmp(&b.0));
            rows
        }
        // **The directory a folder was dropped on this window to name**, and
        // it is the host's for [`presets_listing`]'s reason one arm up: a
        // listing is a directory read, and this side is the only side with a
        // disk (ADR-0156). Pointed nowhere it lists nothing and the sentence
        // about it is [`why_nothing`]'s.
        //
        // **A `.kir` in it is not a row**, and that note stands as written: a
        // folder row is a *take*, `--take-in` checks every inlined source
        // against the address its `slot` record names, and a loose `.kir`
        // names nothing (ADR-0338).
        Scope::Folder => folder_listing(folder)
            .into_iter()
            .filter(|_| kinds.shows_sets())
            .map(|id| (id, RowKind::default()))
            .collect(),
        // **The versions of the Set the load pulldown's deck is running**,
        // already narrowed and already in order — see [`walked`], and this
        // function's own head for why the narrowing is a Set rather than a
        // deck and why a `None` row matches nothing.
        //
        // **The kind chips do not narrow it**, because a version is not a Set
        // and not a procedure: it is one node's source at one moment, and the
        // six chips partition the rows of a *library*.
        Scope::History => walked
            .as_ref()
            .map(|found| found.rows.clone())
            .unwrap_or_default()
            .into_iter()
            .map(|row| (row, RowKind::default()))
            .collect(),
    };
    view.library = rows.iter().map(|(name, _)| name.clone()).collect();
    view.kinds = rows.into_iter().map(|(_, kind)| kind).collect();
    // And what the Master bay's `+ add` offers, read on the same press and off
    // the same two tiers: the master chain holds `kind L5` procedures, so what
    // it can be handed is the `kind L5` rows of the library
    // ([ADR-0340](../../../docs/adr/0340-kind-l5-is-written-and-the-master-chain-is-an-ordered-list-of-them.md)).
    //
    // An entry carries a content address and a row carries a name, so this is
    // not read off `view.library`: an add names the procedure's source by its
    // address, and the address is the hash of bytes this side has and the
    // console has not (ADR-0156).
    view.chain_add = chain_offers(store, &kept, &shipped);
    // **Only where the filter was applied.** `layer` survives a scope change —
    // it is the console's own value and not a position in a listing — so a bay
    // reading `presets` under a set `layer` field would otherwise report a
    // narrowing that narrowed nothing. What says so out loud is the press:
    // `Readout::narrowed`.
    let narrowed = matches!(scope, Scope::AllSets | Scope::MySets)
        .then(|| narrowing(holds.as_deref(), None))
        .flatten();
    // **What the walk has to say beside the count**, and it is empty for every
    // other scope: a truncated listing that read as a whole one is the failure
    // `Listing::stopped_short` exists to prevent, and the foot's `n of m`
    // counts the rows this side handed over rather than the ones it did not
    // reach.
    let aside = walked.map(|found| found.said).unwrap_or_default();
    let line = match (view.library.len(), narrowed) {
        // **A filter that hid everything is a different nothing from an empty
        // library**, and it is the one kind `why_nothing` cannot name: the
        // store is not empty, and what to do about it is press a field rather
        // than save a Set. It is the sentence the MCP tool answers the same
        // case with, for the same reason.
        (0, Some(narrowed)) => format!(
            "  library: `{}` lists nothing — none of the {} Set{} here {narrowed}, so the \
             filter row is what to press rather than `k`",
            scope.name(),
            held.len(),
            match held.len() {
                1 => "",
                _ => "s",
            }
        ),
        (0, None) => format!(
            "  library: `{}` lists nothing — {}",
            scope.name(),
            why_nothing(scope, folder.is_some(), running.is_some())
        ),
        (listed, Some(narrowed)) => format!(
            "  library: `{}` lists {listed} of {} Set{}, {narrowed}",
            scope.name(),
            held.len(),
            match held.len() {
                1 => "",
                _ => "s",
            }
        ),
        (listed, None) => format!(
            "  library: `{}` lists {listed} {}",
            scope.name(),
            match (scope, listed) {
                (Scope::History, 1) => "version",
                (Scope::History, _) => "versions",
                (_, 1) => "Set",
                (_, _) => "Sets",
            }
        ),
    };
    format!("{line}{aside}")
}

pub(crate) mod arrangement;
pub(crate) mod reading;
pub(crate) mod transfer;

pub(crate) use arrangement::*;
pub(crate) use reading::*;
pub(crate) use transfer::*;
