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
pub(crate) mod folder;
pub(crate) mod reading;
pub(crate) mod transfer;

pub(crate) use arrangement::*;
pub(crate) use folder::*;
pub(crate) use reading::*;
pub(crate) use transfer::*;
