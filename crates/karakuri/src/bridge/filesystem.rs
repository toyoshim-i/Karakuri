use super::*;

/// **What the store holds, summarised**: every Set in it, most recent first,
/// with what each one is made of.
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
/// **Most recent first, which `Store::list_sets` is not.**
/// `docs/manual/operations.html`'s row says the listing is most recent first
/// and the MCP tool sorts for it; the bay was showing ascending id, so the
/// panel and the tool answered one operation two ways. The tie-break is the id
/// and it is not decoration — two Sets written inside one tick of a coarse
/// filesystem clock carry the same mtime, and a sort whose keys tie leaves
/// whatever order the entries arrived in.
///
/// # Read once, and by the side of the seam that may read a disk
///
/// Called from `resumed`, before the first frame. A listing is a directory
/// read and a frame path does not do those
/// ([P-0091](../../../docs/principles/0091-cost-is-known-before-it-is-paid.md)),
/// and nothing in `karakuri-console`'s `src/` could do it anyway: opening a
/// store is `karakuri-store`'s and the crate depends on neither it nor the
/// engine (ADR-0156). What crosses into the console is a list of names.
///
/// The cost of reading it once is that a Set saved while this window is up
/// does not appear in the bay until the next run. That is this program's
/// limitation and not the console's — the field is rewritable per frame like
/// every other one — and closing it wants a reason to re-read rather than a
/// timer, which is a decision and not this pass's.
///
/// # A missing store is listed as nothing, and is not created
///
/// [`karakuri_store::store::Store::open`] *"establishes the store layout under
/// `root`, creating any directories that do not exist yet"*, which is the
/// right thing for a program that is about to write one and the wrong thing
/// for one that only wants to read. A program that listed a library by first
/// making one would change the directory it was run in, so the root is
/// required to be there already.
///
/// Either way the answer is a list, and an empty one is a bay with nothing in
/// it — which is what `view::library` draws for it, and is honest: a store
/// this run could not read holds nothing it can name.
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

/// **What a Set's row is**: the layers its own `slot` records fill, once each
/// and in the file's own order.
///
/// **Read off the summary the listing already has**, which is why a badge costs
/// nothing beyond what `all` was already paying: `setfile::summarise` reads a
/// line per node to answer *what is in my library*, and the layer is one of the
/// fields it already carries (P-0091, ADR-0338).
///
/// **Once each**, because a badge says *this Set fills that layer* and a Set of
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

/// **What a kept procedure's row is**: its one declared kind, and that it is a
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

/// **Does this procedure pass the kind row?**
///
/// `LibraryKinds::shows_layer` answers it for a file that declares a kind, and
/// this adds the one case that value cannot carry: **a `.kir` with no `kind`
/// line shows only while nothing is on**. It is a row of the library either way
/// — dropping it would answer *what have I kept* with a file missing — and a
/// chip that named it would be a chip claiming it is of a kind nobody wrote.
pub(crate) fn shows_kept(
    kinds: karakuri_operation::LibraryKinds,
    kind: Option<karakuri_operation::Layer>,
) -> bool {
    match kind {
        Some(layer) => kinds.shows_layer(layer),
        None => !kinds.narrowing(),
    }
}

/// **What the presets root ships that can go over a layer**, as its rows —
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

/// **One procedure the operator has kept**, as [`procedures`] found it: the
/// name its row is drawn under, when it was written, and the layer it declares.
///
/// **`ListedProcedure` and not `Kept`**, which is taken: [`Kept`] is what a
/// press on the Inspector's `keep` capsule *files*, and this is a row of the
/// listing that files show up in. One is an act and the other is a listing, and
/// a name over both would be a name meaning two things.
///
/// **`kind` is an `Option` and a `None` is a row**, which is
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

/// **Which Sets this store has starred**, as their ids — the other half of
/// what the Library bay lists, and the half `my sets` *is* (ADR-0299).
///
/// # It is [`library`]'s shape one file along, and its failures are the same
///
/// A store that is not there holds no stars, a store that will not open is
/// **said out loud** rather than answered with silence, and either way the
/// answer is a set — because a scope that is empty because a file could not be
/// read looks exactly like one that is empty. The one difference from
/// [`library`] is that a missing `favourites.json` is not a failure at all:
/// `Store::favourites` answers an empty set for it, which is a store nobody
/// has starred in.
///
/// **Nothing is pruned against the listing here.** A mark whose Set a hand
/// removed from `sets/` stays in the file — that is `Store::favourites`' own
/// rule and ADR-0299's — and what makes it harmless is that [`listing`] takes
/// the *intersection* with what the store holds, so an id naming no Set draws
/// no row and can still have its star taken off.
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

/// **What one Set declares, read off the cards the store keeps beside its
/// artifacts** — the answer to
/// [`Operation::ReadSet`](karakuri_operation::Operation::ReadSet), in the shape
/// the Library bay draws it in.
///
/// # It is the same reading the MCP tool gives a model, through the same path
///
/// `karakuri_mcp::read_set` renders this for a model, and what it
/// reads is the Set file's `slot` records and each artifact's metadata card —
/// `Store::read_set`, then `Store::read_meta` per node, then the `param_decl`,
/// `capacity_decl` and `emit` records on it. **This walks the same records**
/// rather than a second source: a panel and a model that disagreed about what
/// a Set declares would be two answers to one question. What differs is the
/// rendering, and it differs because the destinations do: a model is handed
/// prose it reads in a context window and a bay is handed one line per control
/// in a column eight characters wide.
///
/// **The prose renderer is not called, and could not be**: it returns one
/// `String` per Set with its blocks already laid out in sentences, and a
/// listing row wants the key and the range apart. Lifting a structured reading
/// into `karakuri-environment` so that both surfaces render one value is the
/// right shape and is a change to a crate this pass may not touch; what is
/// here is written against the same records in the same order so that the day
/// somebody does, this is what moves.
///
/// # The three blocks the row promises, and the fourth that is not here
///
/// *"Every knob with its range and default, the element count, the attributes
/// emitted — each read off the artifact's own card, so those three fetch no
/// source and compile nothing."* The three are each a record on a card.
/// **The element storage is deliberately absent**: the MCP
/// tool's `element_storage_block` calls `setfile::load`, which fetches every
/// source in the Set and runs `compile::check` over it, so a panel that drew
/// it would be paying exactly the cost that sentence says these three do not.
/// `docs/manual/console.html`'s note says the same and says where the figure
/// goes instead — *"A model asking over MCP gets it and pays for it; a press
/// in a library list is not the place to spend that"* — which the row settles
/// the same way.
///
/// # One control per key, and the range is the one every node agrees to
///
/// A Set publishes one control per *key* and not one per declaration
/// (`docs/ir-spec.md`, and `karakuri_engine::set::Set::published`), so a name
/// two nodes declare is one row over the part of the range both of them
/// accept. That intersection is `Set::declared_range`'s own arithmetic —
/// `lo.max(min)`, `hi.min(max)` — done here because **a built Set is what this
/// chip exists to be pressed before**: `published()` needs an engine and a
/// device, and a reading that took one would be the load it is meant to save.
///
/// The **default** is the first declarer's, and the order is the file's own —
/// the same order `read_set` prints its nodes in and the order
/// `Set::published` walks. A default is one number and two nodes may declare
/// two; the alternative is drawing neither, which is a blank row and is the
/// one thing the mock's own tip refuses.
pub(crate) fn declared(root: &std::path::Path, id: &str) -> Result<Reading, String> {
    let store = Store::open(root).map_err(|e| format!("the store at {}: {e}", root.display()))?;
    let lines = store
        .read_set(id)
        .map_err(|e| format!("reading set `{id}`: {e}"))?;
    let mut reading = Reading {
        id: id.to_owned(),
        ..Reading::default()
    };
    // The key's range so far and the first declarer's default, in the order
    // the keys first appeared — a `Vec` and not a map for `Set::published`'s
    // own reason: the order is what the author wrote, and a hasher's order
    // would move between runs.
    let mut knobs: Vec<(String, [f32; 2], Option<f32>)> = Vec::new();
    let mut capacity: Option<[u32; 3]> = None;
    let mut emits: Vec<String> = Vec::new();
    for line in &lines {
        let Record::Slot { proc_hash, .. } = line.record() else {
            continue;
        };
        reading.nodes += 1;
        // **A node with no card is counted rather than skipped in silence.**
        // A card is derived rather than kept, so an artifact stored as bytes
        // has none — an ordinary state of a working store, which
        // `mcp::node_block` says at length — and what it costs this reading is
        // whatever that node declared. The foot is where that is said.
        let Ok(card) = store.read_meta(proc_hash) else {
            continue;
        };
        reading.described += 1;
        for entry in &card {
            match entry.record() {
                Record::ParamDecl {
                    key,
                    min,
                    max,
                    default,
                    ..
                } => match knobs.iter_mut().find(|(seen, ..)| seen == key) {
                    Some((_, range, _)) => {
                        range[0] = range[0].max(*min);
                        range[1] = range[1].min(*max);
                    }
                    None => knobs.push((key.clone(), [*min, *max], *default)),
                },
                Record::CapacityDecl { min, max, default } => {
                    capacity = Some(match capacity {
                        None => [*min, *max, *default],
                        Some([lo, hi, was]) => [lo.max(*min), hi.min(*max), was],
                    })
                }
                // **The union, in the order the nodes declare them.** A Set
                // over two geometries emits what both of them do, and the row
                // is *what a renderer drawn over it can consume* rather than
                // any one node's list.
                Record::Emit { attrs } => {
                    for attr in attrs {
                        if !emits.contains(attr) {
                            emits.push(attr.clone());
                        }
                    }
                }
                _ => {}
            }
        }
    }
    reading.knobs = knobs
        .into_iter()
        // **Spelled `view::Published` in full**, because
        // `karakuri_engine::set::Published` is in scope here under the same
        // name and they are the same idea two levels apart: one is what a
        // built Set publishes and this is what a file's cards say it will.
        .map(|(key, [min, max], default)| view::Published {
            key,
            range: spelled(
                &format!("{min}"),
                &format!("{max}"),
                default.map(|d| format!("{d}")),
            ),
        })
        .collect();
    reading.capacity = capacity.map(|[min, max, default]| {
        spelled(
            &format!("{min}"),
            &format!("{max}"),
            Some(format!("{default}")),
        )
    });
    reading.emits = (!emits.is_empty()).then(|| emits.join(", "));
    Ok(reading)
}

/// **A declared range and the default that applies until something turns it**,
/// in the shape the mock draws every line of a reading in: `0 – 8 · 2`.
///
/// **The three marks are the mock's own** — an en dash between the ends of the
/// range and a middle dot before the default — and both are in the face the
/// panel draws with, which is asserted rather than assumed
/// ([`the_marks_a_reading_is_spelled_with_are_in_the_face`]): the Library
/// bay's foot read `load → A` with the arrow typed as a U+2192 the default
/// face does not carry, and drew `load □ A` for a release. The arrow is drawn
/// rather than typed now, and is a label between two capsules (ADR-0305).
///
/// **A default that is not a literal is a word and not a blank.** The card's
/// `default` is absent where the declaration's expression is not a number this
/// build can state — never where there is none, since the `.kir` grammar makes
/// the expression mandatory — so what a blank would say here is false. `expr`
/// is what is drawn instead, in a column that has room for four characters.
pub(crate) fn spelled(min: &str, max: &str, default: Option<String>) -> String {
    format!(
        "{min} – {max} · {}",
        default.unwrap_or_else(|| "expr".to_owned())
    )
}

/// **The reading under the cursor, read and written into the view**, and the
/// sentence to print about it.
///
/// [`listing`]'s shape one control along, and it is here for that function's
/// reason: reading a Set file and the cards behind it is a disk read, this is
/// the side of the seam that owns the store, and `karakuri-console` takes none
/// of the three (ADR-0156). **On the press and never on a frame** (P-0091) —
/// which is the press on the `params` chip, and the key that moves the cursor
/// while a reading is open, because the reading follows the cursor.
pub(crate) fn read_reading(view: &mut View, store: &std::path::Path) -> String {
    // **The Sets, which is empty under `history`**: a reading is what a Set
    // declares and a row of that scope is a version, so the cursor has no
    // operand there — `view::View::sets`.
    let Some(id) = view.sets().get(view.cursor_row()).cloned() else {
        // A press with no row under the cursor asks nothing —
        // `LibraryBay::read` answers `None` for it — so this is reachable only
        // from a cursor that moved in a listing that went empty in between, or
        // from a reading left open while the bay was marked `history`: the
        // block is not drawn there (`View::opened` matches the row's name), and
        // it is put away here rather than left holding an answer about a Set
        // nobody can see.
        view.shut_reading();
        return String::from(match view.scope() {
            Some(scope) if !scope.lists_sets() => {
                "  read: `history` lists the versions of a Set rather than Sets, so there is \
                 nothing under the cursor for a reading to be about"
            }
            _ => "  read: this bay lists nothing, so there is no Set to read",
        });
    };
    match declared(store, &id) {
        Ok(reading) => {
            let line = format!(
                "  read: `{id}` declares {} over {}{}",
                reading.knobs_word(),
                reading.nodes_word(),
                match reading.nodes == reading.described {
                    true => String::new(),
                    false => format!(", {}", reading.cards_word()),
                }
            );
            view.read(reading);
            line
        }
        // **Said out loud and the block put away**, which is [`library`]'s
        // rule one bay up: a reading that failed to read and a Set that
        // declares nothing must not draw the same.
        Err(e) => {
            view.shut_reading();
            format!("  read: `{id}` could not be read — {e}")
        }
    }
}

/// **The reading follows the cursor, on whichever surface moved it.**
///
/// `karakuri-console/src/view.rs` states the rule on `View::reading_open`:
/// *"a move with one open is a read of the row it arrived at, and a move with
/// nothing open is a pointer moving"*. **Two surfaces move that cursor**, the
/// arrow keys through `View::walk` and a carry's press through
/// `View::point_at`, and both owe it the same read — this is the one place
/// that read is written, so there is one implementation of the rule for both
/// to call rather than two copies that could answer it differently.
///
/// `moved` is each caller's own answer to *did this press move the cursor*:
/// the arrow-key arm compares `View::cursor_row()` before and after the
/// press, and the carry's arm is `matches!(acted, Acted::Pointed)`. Neither
/// shape is repeated here, because *what counts as a move* is each surface's
/// own question and this function's only question is what to do once one
/// has happened.
///
/// # The defect this rule exists to prevent
///
/// Until ADR-0265, `Readout::took` discarded `View::point_at`'s `moved`. A
/// carry taken in hand while a reading was open on a **different** row moved
/// the cursor off it, `View::opened` answered `None` because the row under
/// the cursor was no longer the Set the reading was of, and the block
/// vanished with nothing on that route ever walking the cursor back — no
/// panic, no diagnostic, a reading that stopped being drawn. The arrow keys
/// never had the bug — they always re-read on `moved && reading_open()` — so
/// the two call sites already agreed before this function existed; what it
/// buys is that they cannot silently stop agreeing.
///
/// Returns the line to print rather than printing it, so a caller with
/// nothing to print — the ordinary case, a press with no reading open — pays
/// for no `println!` and a test can call this with no stdout to capture.
pub(crate) fn reread_if_open(
    moved: bool,
    view: &mut View,
    store: &std::path::Path,
) -> Option<String> {
    (moved && view.reading_open()).then(|| read_reading(view, store))
}

/// **The record's layer a vocabulary layer names.**
///
/// A third spelling of a list that already has two conversions in
/// `karakuri-environment` — `setfile::kind_of` and `mcp.rs`'s pair — and it is
/// here because both of those are private to that crate and neither is on its
/// way out. What crosses the seam is the summary, whose nodes carry a
/// `karakuri_store::record::Layer`, and what a filter carries is a
/// `karakuri_operation::Layer`; the comparison has to happen on one side.
///
/// **Exhaustive with no wildcard**, which is what makes it a table rather than
/// a guess: a sixth layer does not compile until somebody says which it is.
pub(crate) fn asked(layer: karakuri_store::record::Layer) -> karakuri_operation::Layer {
    use karakuri_operation::Layer as Asked;
    use karakuri_store::record::Layer as Written;
    match layer {
        Written::L1 => Asked::L1,
        Written::L2 => Asked::L2,
        Written::L3 => Asked::L3,
        Written::L4 => Asked::L4,
        Written::Field => Asked::Field,
        Written::L5 => Asked::L5,
    }
}

/// **The word a `kind` line spells a layer with, as the vocabulary's own
/// value** — `karakuri_environment::history::LAYERS`' six words read back.
///
/// `None` for a word that is not one of the six, which is a `.kir` declaring
/// no kind at all: `declared_kind` already answers `None` there, and this is
/// the same absence carried one step further rather than a second reading of
/// it.
///
/// **The wildcard is the risk here and it is why `LAYERS` is the list.** This
/// match answers a word rather than a value, so a sixth kind does *not* stop
/// the build — it falls to `None` and a `kind L5` file reads as one declaring
/// nothing. That is the failure `setfile::layer_from_ordinal`'s comment names
/// one layer earlier, arriving through the other door.
pub(crate) fn kind_of(word: &str) -> Option<karakuri_operation::Layer> {
    use karakuri_operation::Layer;
    match word {
        "L1" => Some(Layer::L1),
        "L2" => Some(Layer::L2),
        "L3" => Some(Layer::L3),
        "L4" => Some(Layer::L4),
        "Field" => Some(Layer::Field),
        "L5" => Some(Layer::L5),
        _ => None,
    }
}

/// **Which kinds the filter row is showing, as a sentence** — the words of the
/// chips that are on, or *every kind* where none of them is.
///
/// `karakuri_operation::LibraryKinds::narrowing` settles the reading of *none
/// on* once and this says it in the panel's own words: a row that hid the whole
/// listing would be a state an operator cannot see their way out of.
pub(crate) fn showing(kinds: karakuri_operation::LibraryKinds) -> String {
    if !kinds.narrowing() {
        return String::from("every kind");
    }
    let on: Vec<&str> = karakuri_console::view::KindChip::ALL
        .into_iter()
        .filter(|chip| chip.on(kinds))
        .map(|chip| chip.word())
        .collect();
    on.join(", ")
}

pub(crate) fn recorded(layer: karakuri_operation::Layer) -> karakuri_store::record::Layer {
    use karakuri_operation::Layer as Asked;
    use karakuri_store::record::Layer as Written;
    match layer {
        Asked::L1 => Written::L1,
        Asked::L2 => Written::L2,
        Asked::L3 => Written::L3,
        Asked::L4 => Written::L4,
        Asked::Field => Written::Field,
        Asked::L5 => Written::L5,
    }
}

/// **Does this Set pass the filter row?** — the same retain
/// `karakuri-environment`'s MCP `list_sets` applies, and deliberately so: one
/// operation narrowed two ways is two answers to *what does this store hold*.
///
/// **A Set matches, not a node.** With both filters given the question is
/// *which of the Sets that hold this also have something on that layer*, so
/// each half is answered against the whole Set — a Set whose `drift_shell` is a
/// geometry and whose deformation is called something else is exactly what that
/// question is looking for.
///
/// `holds` is matched case-insensitively against what each node is called,
/// because an operator who read `drift_shell` in one row and stepped to
/// `Drift_Shell` in another is not asking a different question. **The fields
/// step through the names as the store spells them**, so today the fold changes
/// no answer; it is here because the tool's does and the two must not come
/// apart the day either field takes letters.
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

/// **What the `holds` field can be stepped to**: every name a node in this
/// store's Sets carries, once each and in one order.
///
/// **The unnarrowed listing's**, which is `view::View::holds`' own instruction
/// and the reason this takes the summaries before [`narrows`] has been near
/// them: candidates read off a filtered listing shrink as the filter bites, and
/// a step would then wander somewhere it could not come back from.
///
/// **Sorted, and not in the order the Sets were written.** The candidates are a
/// list somebody steps through one press at a time, so the order has to hold
/// still while they do it — where the rows above are ordered by recency and
/// move whenever anything is saved. `BTreeSet` is the sort and the deduplication
/// in one pass.
pub(crate) fn holds_choices(sets: &[setfile::SetSummary]) -> Vec<String> {
    sets.iter()
        .flat_map(|set| set.nodes.iter().map(|node| node.name.clone()))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect()
}

/// **What the filter row is narrowing to, in words**, or `None` where it is
/// narrowing nothing.
///
/// The console draws the two values and this says what they mean, which is the
/// division every other readout on this panel makes: a field reads `L4` and a
/// line says the listing under it is the Sets that hold one.
///
/// **`layer` arrives already spelled**, out of `view::Filters::layer_word` —
/// the word the field itself is drawing. A `{:?}` here would print `Field`
/// where the field reads `FIELD`, which is a readout and a sentence about it
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

/// **One row of a scope whose rows are files**: the word the bay draws and the
/// file behind it.
///
/// Two fields because the bay lists **names** and a take-in needs a **path**:
/// what crosses into the console is a `String` per row
/// (`view::View::library`), and what this program has to be able to find again
/// on the press is the file that row came off.
///
/// **Two scopes have rows of this kind** — `presets`, which is a told
/// directory (ADR-0230), and `folder`, which is one somebody dropped on this
/// window (ADR-0275). They are one type because a row of either is a Set file
/// that is not in this store yet and a press on it is the same two operations
/// (`docs/manual/operations.html`'s *Send a Set to somebody, and take one
/// in*): the difference between them is which directory was listed, which is
/// [`Taking`]'s.
pub(crate) struct FileRow {
    /// What the row reads, which is the file's own name without its
    /// extension. **Not read out of the file**: a listing that opened
    /// twenty-three files to draw twenty-three rows would be a directory read
    /// doing a file read's work, and the id a take-in files the Set under is
    /// the one *inside* the file anyway — read there, on the press, by
    /// [`taking_in`].
    pub(crate) id: String,
    pub(crate) path: std::path::PathBuf,
}

/// **Every Set the preset library offers**, which is the `.kset` files in the
/// root this run resolved.
///
/// # One call, and the reading is not this program's
///
/// The listing is `karakuri_environment::places`', beside the resolution that
/// answers *where* the presets are: what a `.kset` is and which directory
/// holds them is that module's business, and a second program wanting the same
/// list must not read the same directory a second way. So this is the one
/// place in this program that knows a preset library can be listed at all, and
/// it knows nothing about how — the shape of a row, the order they come in,
/// and what a name the layout does not claim does are all answered there.
///
/// # What it lists, and why not the `.kir` files beside them
///
/// `console.html`'s *A folder scope reads Sets, and a bundle is not a third
/// thing* settles it: *"A directory of `.kir` files is a directory of parts,
/// and a library lists what you can put on a deck."* A `.kir` is one node's
/// source addressed by its content and nothing in the vocabulary takes one, so
/// the parts are not rows — they are what the rows *name*.
///
/// **A root with nothing in it is a library nobody has filled**, and it is not
/// a failure: the scope lists nothing and the sentence about it is
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

/// **Every Set a dropped folder holds**, which is the Set files directly in
/// the directory this bay was pointed at (ADR-0275).
///
/// # What it lists, and why both spellings
///
/// `console.html`'s *A folder scope reads Sets, and a bundle is not a third
/// thing*: *"A Set file and a bundle are the same file … so the scope draws
/// one kind of row rather than two"*, and the authored form that names its
/// parts by relative path *"is a Set file, is one row, and is taken in by the
/// same operation."* So both suffixes are listed and neither is a second kind
/// of row — `Store::SET_FILE_SUFFIX` for the resolved form and
/// `setfile::AUTHORING_SUFFIX` for the authored one, borrowed from the modules
/// that own them rather than spelled here. A `.kir` is **not** listed: it is
/// one node's source, nothing in the vocabulary takes one, and *"a directory of
/// `.kir` files is a directory of parts"*.
///
/// **A directory that holds `night.kset` and `night.kbset` lists two rows
/// reading `night`**, and that is deliberate rather than got to by accident:
/// they are two files, each of which is a Set, and choosing between them here
/// would be this listing inventing a precedence between the two forms.
/// **Whichever of them a press means is the take-in's question and it is
/// answered by refusing**: [`Taking::file`] finds the row by the word that was
/// pressed, two files wear that word, and a press that took one of them would
/// be picking for the operator between two rows they cannot tell apart on
/// screen. See there, where the refusal names both files.
///
/// # Ascending, one directory deep, and the name is all that is read
///
/// `places::Presets::list_sets`' three rules, carried over for its reasons:
/// `read_dir` hands back no order at all, a name the layout does not claim is
/// skipped rather than repaired, and nothing here opens a file — a malformed
/// Set is a refusal at the moment it is taken in, where the operator can see
/// which row they pressed. It is **not** [`library`]'s most-recent-first
/// (ADR-0263): that order is a store's, where a Set's time is when the
/// operator wrote it, and a folder full of files somebody copied has mtimes
/// that are facts about this machine's disk.
///
/// # Where this belongs, and it is not here
///
/// **`karakuri_environment::places` is the right home**, beside
/// `Presets::list_sets`, which answers the same question about a directory
/// this run was told about rather than one it was handed: this is that
/// function with two suffixes and no `Found` behind it, and a second walk of a
/// directory of Sets is a second answer to *what is a Set file called*. It is
/// here because ADR-0275's owed work was this file's, and the reason is
/// written down rather than left to be inferred — `declared`'s own shape one
/// bay over.
pub(crate) fn folder_listing(dir: Option<&std::path::Path>) -> Vec<String> {
    folder_files(dir).into_iter().map(|row| row.id).collect()
}

/// **The same walk with the file names kept**, which is what a take-in needs:
/// the bay lists words and the press has to find the file the word came off
/// again ([`FileRow`]).
///
/// **One walk and not two**, which is why [`folder_listing`] is a `map` over
/// this rather than a second `read_dir`: a listing the bay drew and a listing
/// the press searched that disagreed would be a press acting on a row nobody
/// saw. It is [`presets_listing`]'s shape one scope along, and that function
/// answers `FileRow`s for the same reason.
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

/// **Why the scope that is marked lists nothing**, in the words that say which
/// kind of nothing it is — and among these four chips there are two kinds.
///
/// Two of them are empty as *data*: a store nobody has saved into and a preset
/// root nobody has filled are libraries with nothing in them, which
/// `console.html` says outright — *"An empty tier is a library nobody has
/// filled rather than something gone wrong."* Fill either and the rows appear
/// with nothing else changing.
///
/// **`my sets` is a third kind, and it is neither of those**: the store may
/// hold plenty and nothing be starred, which is a listing that is empty
/// because of an answer rather than because of an absence (ADR-0299). What to
/// do about it is press a star, and the sentence says so.
///
/// **The last of them is empty as *machinery*, and it is the one that
/// changed:**
///
/// - **A folder nobody has pointed anywhere is empty for want of a gesture**,
///   and that is the one of the four that changed on 2026-09-08. It used to be
///   empty for want of machinery — this said *"nothing here reads a folder
///   dropped on this window yet"* — and [`folder_dropped`] is that machinery.
///   `Operation::ListSets` still has nowhere to put a directory and should
///   not: both its fields narrow what a store already holds, and which store
///   is asked at all is [`listing`]'s own answer. **So this scope has two
///   sentences and `pointed` is which**: no folder has been dropped yet, or
///   one has and holds no Set file. The second is [`Scope::AllSets`]' kind of
///   nothing — a library nobody has filled — read in somebody else's
///   directory.
///
/// It is said out loud on the step and again on a press, because a scope that
/// went quiet and a scope that is empty are the same experience — which is the
/// rule every other refusal in this file is written to.
///
/// **The fifth is a third kind again, and it has two sentences of its own.**
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

/// **What a folder over this window reads in the `.path` row**, written into
/// the console for the pass that is about to draw it.
///
/// **The one thing about this bay that is a frame's business**, and it is
/// `egui`'s doing rather than a choice here: `RawInput::take` *clones*
/// `hovered_files` where it *moves* `dropped_files`, so a drag over the window
/// is a fact about every pass while it lasts and there is no event to hang it
/// off. Nothing is asked of the file system for it — whether the path is a
/// folder is the drop's question (P-0091, ADR-0275) — and nothing is
/// allocated on a pass where the answer has not changed.
///
/// **More than one path over the window reads as none.** The row says *"the
/// path a release would set"*, a release sets nothing where two arrived
/// (ADR-0275), and drawing the first of them would be this row picking one out
/// of a list the desktop happened to build — which is the choice the refusal
/// below exists to refuse.
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

/// **A folder let go on this window**, which is how the `folder` scope is
/// given a directory — and the three answers ADR-0275 settles, in the words
/// that record and `console.html` write.
///
/// # It is done here, on the pass the drop arrives on, and nothing is put by
///
/// `dropped_files` is visible for exactly one pass and then gone
/// (`RawInput::take` moves it), so the release does the whole thing rather
/// than asking a question: it sets the directory **and** marks the `folder`
/// chip, and the listing under it is the next thing drawn. A bay that had put
/// the path aside and waited for the chip to be pressed would be waiting on an
/// operator who has already made the gesture, holding a path nothing will hand
/// it a second time.
///
/// **So the file system is asked here, on a frame**, which is the one place
/// this program does that and it is P-0091's rule rather than an exception to
/// it: what is asked once is asked once, and a drop is one act. A drop is also
/// the only moment the question can be asked at all — the event carries a path
/// and nothing else, and *"a file and a directory are indistinguishable at the
/// event"* (ADR-0275).
///
/// # The two refusals, and each is a policy the plumbing does not answer
///
/// **A path that is not a directory is refused, naming what was dropped.** A
/// file is not read as the folder it sits in — that would point this bay at a
/// directory nobody pointed at, which is the mistake the carry one bay over
/// refuses when it declines to snap a drop mark to the nearest strip
/// (ADR-0273) — and a `.kbset` is not taken in where it fell, because taking a
/// Set in is a press on a row of a listing and a file landing on this window
/// has no row under it.
///
/// **More than one path is refused, and all of them are.** A multi-item drag
/// arrives whole, so three folders let go together are three entries in one
/// pass and not three drops: there is no first to act on and a rest to ignore,
/// nothing says which was aimed at, and the order is the desktop's rather than
/// the operator's. The bay keeps the directory it had and the refusal counts
/// what arrived.
///
/// **Both name what arrived and what to do instead** (P-0083), and both say
/// where this library is still pointed — because a refusal that left an
/// operator wondering whether the bay had moved anyway is a refusal that costs
/// a second gesture to read.
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

/// **Whether a name is one a Set file wears**, which is the two suffixes
/// [`folder_listing`] lists and is asked here for one reason: an operator who
/// let go of a `.kbset` on this window was trying to take a Set in, and the
/// refusal owes them the press that does it (P-0083).
///
/// It is a **name** and not a reading: nothing is opened, exactly as nothing
/// is opened to draw a row.
pub(crate) fn set_file(path: &std::path::Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            name.ends_with(Store::SET_FILE_SUFFIX)
                || name.ends_with(karakuri_environment::setfile::AUTHORING_SUFFIX)
        })
}

/// **The rows the Library bay lists for the scope that is marked**, written
/// into the view, and the sentence to print about it.
///
/// # One function, and it is what a scope *is* on this program's side
///
/// The console draws a row of chips and marks one of them; **which listing
/// belongs under that mark is this side's answer**, because every one of the
/// four is something outside this process — a store, a told directory, a
/// filter over the first, a directory somebody names during the run — and
/// `karakuri-console` takes none of them (ADR-0156). So the seam is a `Vec` of
/// names, and this is the one place it is filled.
///
/// **On the press that changed the scope and at startup, never on a frame.** A
/// listing is a directory read (P-0091), which is the same rule [`library`]
/// states one scope down and the reason this is not called from the frame
/// handler.
///
/// All five answer with rows now, and each of the five can still answer with
/// none — [`why_nothing`] is where the sentences are, and it is one
/// function so that a scope which stops being empty stops being empty in one
/// place. **Three of them depend on something that happened during the run**:
/// `folder` is `None` until somebody drops a directory on this window
/// ([`folder_dropped`]), `my sets` is empty until somebody presses a star
/// ([`favourite`]), and `history` is empty until the deck the load pulldown
/// names is running a Set that has been edited.
///
/// # `history` is the one scope that is not a directory of Sets
///
/// Its rows are the versions of **one Set** — the one the load pulldown's
/// deck is running, which `running` carries — and they come off
/// `karakuri_environment::history::list`, most recent first, in that
/// function's own order rather than in one applied here (ADR-0263's argument
/// on a different listing).
///
/// **The narrowing is a Set and never a deck**, which is why `running` is an
/// id rather than a slot: two decks playing one Set have one history between
/// them, and a version is filed under the Set the slot was running
/// (ADR-0304). **A `None` row matches no Set** rather than matching every one
/// of them — a version written where there was no Set is a version of
/// nothing, and treating it as a wildcard would put another run's edits under
/// whatever Set happens to be loaded now (ADR-0276's own consequence).
///
/// **The cap is on the walk and not on the Set.** [`HISTORY_MOST`] rows are
/// asked for and the narrowing happens after, so a store whose day directories
/// hold several Sets' versions lists fewer of each; `Listing::stopped_short`
/// is what says the walk stopped with days unread, and it is said out loud
/// beside the count rather than left for the foot's `n of m` to imply.
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

/// **How many rows of the edit history one walk asks for.**
///
/// `karakuri_environment::history::list` takes the number from its caller and
/// has no default, because *"what a bay can afford to draw and what a model can
/// afford to be handed are different numbers"* (P-0090) — so this is the
/// panel's answer and nowhere else's.
///
/// **Larger than any bay can draw, and small enough that the walk stops after a
/// handful of days.** The Library bay's list is one row per
/// `view::size::LIB_ROW_H`, so a full-height bay on a tall display draws a few
/// tens of them; the cost of the walk is one `read_dir` per day directory
/// entered and no file opened at all, and it stops entering them once it has
/// this many. What lies past it is not counted — counting it is the cost the
/// cap exists not to pay — and `Listing::stopped_short` says the walk stopped,
/// which is the property that matters.
pub(crate) const HISTORY_MOST: usize = 200;

/// **The rows the `history` scope lists, and the sentence about how they were
/// found.**
///
/// Split out of [`listing`] because it is the one arm of that function that
/// has something to say beside the count: the walk is capped and it is capped
/// on *days opened* rather than on this Set's rows, so a listing that stopped
/// short has to say so or it reads as the whole history.
///
/// **A row is the name the store filed the version under**, less the Set id
/// every row here shares — see [`version_row`], which is also how a landing
/// finds the file again.
pub(crate) struct Walked {
    pub(crate) rows: Vec<String>,
    pub(crate) said: String,
}

pub(crate) fn walked(store: &std::path::Path, running: Option<&str>) -> Walked {
    // **No Set, no rows, and not an error.** The deck is playing a pair
    // somebody typed; its versions are filed under no Set, and `why_nothing`
    // is where that is said in words.
    let Some(id) = running else {
        return Walked {
            rows: Vec::new(),
            said: String::new(),
        };
    };
    let found = match karakuri_environment::history::list(store, HISTORY_MOST) {
        Ok(found) => found,
        // Said rather than swallowed, and the scope lists nothing: a history
        // that would not open is a different fact from a Set with no versions,
        // and the two must not draw the same empty list in silence.
        Err(why) => {
            return Walked {
                rows: Vec::new(),
                said: format!("\n  history: {why} — so this scope lists nothing"),
            };
        }
    };
    let rows: Vec<String> = found
        .versions
        .iter()
        // **`Some(id)` and never `None`.** A version written where there was
        // no Set is a version of nothing, so it matches no Set rather than
        // every one of them (ADR-0276).
        .filter(|version| version.set.as_deref() == Some(id))
        .map(version_row)
        .collect();
    let mut said = String::new();
    if found.stopped_short {
        said.push_str(
            "\n  history: the walk stopped with days unread, so this is part of what is \
             there rather than all of it",
        );
    }
    if found.unclaimed > 0 {
        said.push_str(&format!(
            "\n  history: {} entr{} under `history/` that this layout does not claim {} \
             passed over",
            found.unclaimed,
            match found.unclaimed {
                1 => "y",
                _ => "ies",
            },
            match found.unclaimed {
                1 => "was",
                _ => "were",
            }
        ));
    }
    Walked { rows, said }
}

/// **One version, as a row of the Library bay's list and as the word a landing
/// names it by.**
///
/// It is the name [`karakuri_environment::history::Snapshots::record`] wrote,
/// less the `@<set>` every row of one walk shares and less the `.kir` — when,
/// which slot, which layer and index, and what the procedure called itself,
/// which is what `Version`'s own head says a row is for.
///
/// **One spelling, used twice.** The bay is handed this and hands it back at
/// the press, and [`restored`] rebuilds it per candidate to find the file
/// again — so the row an operator pressed and the version that is landed
/// cannot come apart, and no path crosses the seam. That is
/// `SetTransfer::Take`'s arrangement: the panel re-asks the listing and finds
/// the row by the word that was pressed.
///
/// **The index is spelled only when it is not the first**, which is `record`'s
/// own rule read back rather than a second one: a `_0` on every L4 of every
/// ordinary run is noise in the way of what a person is scanning for.
pub(crate) fn version_row(version: &karakuri_environment::history::Version) -> String {
    // **The spelling is the history module's**, since 2026-09-10: a model
    // walking the same history over MCP reads rows out of `walk_history` and
    // hands one back in `Revision::Picked`, so a second `format!` here would be
    // a second answer to *what is this row called* — and the two surfaces hand
    // the name to each other (`docs/adr/0342-…`).
    version.filed_as()
}

/// **What a take-in did**: the file it read, the id that file filed itself
/// under, and the sentence `setfile::unbundle` reported.
///
/// **Three fields because the press has three callers for them and each is a
/// different question.** The `said` is what the operator reads. The `id` is
/// what the load that follows names, and it is the file's own rather than the
/// row's word. The `file` is what the *operation* names —
/// `Operation::TransferSet`'s `SetTransfer::Take { file }` carries a path,
/// *"because a file is what the only existing route takes"* — so it is
/// returned rather than re-derived: the listing is asked once, on the press,
/// and asking it a second time to name what was already taken in would be two
/// answers to *which file was this* with a directory read between them.
#[derive(Debug)]
pub(crate) struct TakenIn {
    pub(crate) file: std::path::PathBuf,
    pub(crate) id: String,
    pub(crate) said: String,
}

/// **Which listing a take-in's row came off**, and it is the whole of the
/// difference between the two scopes that have rows of files.
///
/// # Two scopes, one row, one press
///
/// `docs/manual/operations.html`'s *Send a Set to somebody, and take one in*:
/// *"Taking one in is not a second row — opening a preset is this row"*, and a
/// folder row is the same row again. `console.html` says the folder side in as
/// many words — *"a folder row is a **take**"*, *"A row here is taken into the
/// store and then loaded, which is one press because taking it in is what
/// gives it a name"* — so the two differ in **which directory was listed** and
/// in nothing else. That is what this type is, and it is why [`taking_in`]
/// takes one rather than a presets root.
///
/// **The `folder` half is what landed on 2026-09-08.** It was refused out
/// loud until then — *"a folder row is a **take**, taking a Set in from a
/// folder is not built"* — because the scope had no directory to list, which
/// ADR-0275 gave it.
///
/// # A path is derived here and never spelled by a surface
///
/// `Operation::TransferSet`'s `SetTransfer::Take { file }` carries a path, and
/// the rule that admits it is that **every route that fills it derives it from
/// something the program itself produced**. Both arms obey it the same way:
/// the listing is asked *again* on the press and the row is found by the word
/// that was pressed ([`Taking::file`]), so what a surface handed over is a
/// word off a listing this program read and never a path.
pub(crate) enum Taking<'a> {
    /// The preset library this run resolved (ADR-0230) — a told directory,
    /// and the same one [`presets_listing`] draws the rows of.
    Presets(Option<&'a karakuri_environment::places::Presets>),
    /// The directory somebody dropped on this window (ADR-0275), and `None`
    /// for a bay that has been pointed nowhere.
    Folder(Option<&'a std::path::Path>),
}

impl Taking<'_> {
    /// The rows this listing holds, **asked again** rather than kept — see
    /// [`taking_in`], where that rule is argued.
    pub(crate) fn rows(&self) -> Vec<FileRow> {
        match self {
            Taking::Presets(presets) => presets_listing(*presets),
            Taking::Folder(dir) => folder_files(*dir),
        }
    }

    /// What the scope is called, for a refusal to name.
    pub(crate) fn scope(&self) -> &'static str {
        match self {
            Taking::Presets(_) => "the preset library",
            Taking::Folder(_) => "the folder this bay is pointed at",
        }
    }

    /// **The file behind the word that was pressed**, or a sentence saying why
    /// there is not one.
    ///
    /// # Two refusals, and the second is the one a folder brought
    ///
    /// **A word this listing no longer holds** is the row having gone between
    /// the listing and the press — a directory this program neither made nor
    /// writes, which is a folder's ordinary condition and a preset root's
    /// unusual one.
    ///
    /// **A word two files wear** is `folder_files`' own note arriving: a
    /// directory holding `night.kbset` and `night.kset` draws two rows reading
    /// `night`, and neither the listing nor the bay puts a precedence between
    /// the two forms. **So the press is refused and both file names go back**
    /// ([P-0083](../../../docs/principles/0083-a-refusal-carries-what-the-next-attempt-needs.md)),
    /// because taking one of them would be this program choosing between two
    /// rows an operator cannot tell apart on screen. A `presets` root can
    /// hold only `.kset` files, so this arm is a folder's in practice and is
    /// asked of both because the rule is the row's rather than the scope's.
    pub(crate) fn file(&self, row: &str) -> Result<std::path::PathBuf, String> {
        let mut found: Vec<std::path::PathBuf> = self
            .rows()
            .into_iter()
            .filter(|held| held.id == row)
            .map(|held| held.path)
            .collect();
        match found.len() {
            0 => Err(format!("{} has no `{row}` in it any more", self.scope())),
            1 => Ok(found.remove(0)),
            _ => Err(format!(
                "{} holds {} files called `{row}` — {} — and this row names a word rather \
                 than a file, so which of them you meant is not something the listing can \
                 say. Rename or move one of them and press again",
                self.scope(),
                found.len(),
                found
                    .iter()
                    .filter_map(|path| path.file_name().and_then(|name| name.to_str()))
                    .collect::<Vec<_>>()
                    .join(" and ")
            )),
        }
    }
}

/// **A row of `presets` or of a `folder`, taken into this store**, and the id
/// it landed under.
///
/// # Taking it in is not a second operation, and it is what gives it a name
///
/// `docs/manual/operations.html`'s *Send a Set to somebody, and take one in*:
/// *"Taking one in is not a second row — opening a preset is this row"*, so
/// opening a preset **is** that row performed. `console.html` reaches it from
/// the other side: *"loading a preset is a packaging step, and a packaging step
/// writes into the store: `my sets` gains a row you did not make."* That is
/// what this does, and it is why a preset row is one press rather than two —
/// the take-in is what gives the Set the id the load needs.
///
/// # It is `karakuri-cli`'s own route and not a second one
///
/// `taken_in_file` resolves a `.kset` with `setfile::bundle_authored` — which
/// is `setfile::resolve` behind its wall, and then the inlining — and hands the
/// result to `setfile::unbundle`. **Resolved *and* inlined rather than resolved
/// alone**, for that function's stated reason: `unbundle` writes a metadata
/// card for each source the lines carry, and handing it resolved lines with
/// nothing inlined would file the Set and leave every artifact cardless. Two
/// routes into one store that reach two different stores is the disagreement a
/// second spelling always is.
///
/// **Both spellings, and the branch is that function's too.** A `.kbset` has
/// its sources inside it and is read straight off the disk as lines; a `.kset`
/// names its parts by relative path and is resolved first. The `presets` scope
/// lists only the second form, so this branch was not reachable until a folder
/// row could be pressed — `console.html`: *"A Set file and a bundle are the
/// same file … so the scope draws one kind of row rather than two"*, and the
/// difference between them is a property of a file rather than a kind of row.
///
/// **The two binaries have no library target between them**, which is why this
/// is a second spelling of `karakuri-cli`'s eight lines rather than a call to
/// them, and it is written down here rather than left to be discovered: the
/// branch is the same branch and the two must not come apart the day a third
/// form arrives.
///
/// **The wall is `resolve`'s and not this file's**: a part named from outside
/// the file's own directory is refused, by path, because *"a Set somebody
/// handed you is not a way of asking this machine for its files"* (ADR-0229).
/// Nothing here loosens it and nothing here repeats it.
///
/// # An id this store already holds is refused, and the refusal is not written
/// # here
///
/// `setfile::unbundle` asks what the store holds before it writes a byte and
/// refuses an id that is taken — *"the id came from the file rather than from
/// you"* — and that sentence is the one the operator gets. A second check here
/// would be a second answer to *may this be overwritten*, and the two would
/// disagree the day one of them moved. What this adds is which of the two acts
/// failed: nothing was taken in, so nothing was loaded, and the deck is exactly
/// as it was.
///
/// # The id is the file's own
///
/// Read off the `set` record in the resolved lines rather than taken from the
/// row's word, because that is the id `unbundle` files it under and the id
/// `my sets` will list. They are the same word in `examples/`, and a preset
/// whose file says otherwise would otherwise be loaded by a name the store does
/// not hold.
pub(crate) fn taking_in(
    root: &std::path::Path,
    from: Taking<'_>,
    row: &str,
) -> Result<TakenIn, String> {
    // **Asked again rather than kept**, which is [`listing`]'s shape: the rows
    // crossed into the console as words, and the file behind a word is found
    // by asking the library again on the press. A second copy of the listing
    // held on this side is a copy that goes on naming a file that has moved.
    let file = from.file(row)?;
    let store = Store::open(root).map_err(|e| format!("store `{}`: {e}", root.display()))?;
    // **The form is the file's own and the branch is `karakuri-cli`'s** — see
    // this function's head. A name is what says which, and nothing is opened
    // to ask: the two suffixes are the two `folder_files` lists.
    let authored = file
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with(karakuri_environment::setfile::AUTHORING_SUFFIX));
    let lines = match authored {
        true => karakuri_environment::setfile::bundle_authored(&store, &file)?,
        false => karakuri_store::ndjson::read(&file)
            .map_err(|e| format!("reading `{}`: {e}", file.display()))?,
    };
    let id = lines
        .iter()
        .find_map(|line| match line.record() {
            Record::Set { id, .. } => Some(id.clone()),
            _ => None,
        })
        .ok_or_else(|| {
            format!(
                "`{}` carries no `set` record, so it names no id to file itself under",
                file.display()
            )
        })?;
    let said = karakuri_environment::setfile::unbundle(&store, &lines)?;
    Ok(TakenIn { file, id, said })
}

/// **The two rows of the vocabulary one press on a `presets` or a `folder` row
/// performs**, in the order they happen.
///
/// # Two operations because they are two rows of the page, and one press
///
/// `docs/manual/operations.html`'s *Send a Set to somebody, and take one in*:
/// *"Taking one in is not a second row — opening a preset is this row"*, so
/// loading a Set out of presets or out of a folder **is** that row performed.
/// The load after it is *Load material into a deck*, which is a different row
/// with a different operation. **One press, two rows** — `console.html` says
/// why it is one press: *"That is one press rather than two because taking it
/// in is what gives it the name the load needs."*
///
/// So the press emits both. Emitting only the load would be a press that
/// performs two of the page's rows and names one, and the row it dropped would
/// be the one **nothing in this workspace constructs**.
///
/// # Naming what a surface performed is the scope's rule, not a new one
///
/// `space` on the Library's head steps the mark itself and emits
/// `Operation::SelectScope` anyway, *"so that the press is recorded as `Silent(Surface)` rather than as
/// nothing at all"*. This is that, one key along: `written` answers
/// `Silent(NoRecord)` for a transfer, nothing in [`App::performed`] performs
/// one, and the emission is the naming.
///
/// # And it is not the key badge
///
/// `key_column::ROWS` maps `enter` in the Library to *Load material into a
/// deck* alone, and that
/// stays true: what an operator reaches from the keyboard is a load, and the
/// taking-in is what a load off `presets` does on the way. ADR-0213's
/// distinction is between an operator **reaching** an operation and something
/// **happening**, and constructing an operation is neither — which is
/// `panel_column.rs`'s own sentence, *"construction is not reachability, and
/// reachability is the definition."*
///
/// The transfer names the **file**, because that is what
/// `SetTransfer::Take` carries — *"a path because a file is what the only
/// existing route takes"* — and the load names the **id**, which is the file's
/// own `set` record rather than the row's word. They are the two halves of
/// [`TakenIn`] and neither is derived from the other here.
pub(crate) fn taken_in_press(deck: u8, taken: TakenIn) -> [Operation; 2] {
    [
        Operation::TransferSet {
            transfer: SetTransfer::Take { file: taken.file },
        },
        Operation::LoadSet {
            deck,
            set: taken.id,
        },
    ]
}

/// **The other direction of that row: a Set out of this store and into a file
/// the operator names**, asked for and answered without a frame waiting on
/// either half.
///
/// # The dialog is asked for here and awaited nowhere
///
/// `rfd::AsyncFileDialog::save_file` is called on this thread — the main one,
/// which is where a press handler is — and returns a future at once. On macOS
/// what that call has already done is `beginSheetModalForWindow:`, an
/// **asynchronous** sheet hung on this window: the run loop is untouched, so
/// the frame loop goes on drawing behind it and the panel's continuous motion
/// goes on saying *this is live*. That is the whole of what
/// [P-0094](../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)
/// asks of a mechanism that could run during a performance, and it was
/// **measured** rather than assumed before this was written —
/// [ADR-0311](../../../docs/adr/0311-a-row-menu-loads-a-set-onto-a-named-deck-and-saves-it-through-the-systems-own-dialog.md)
/// carries the reading and the probe. The synchronous `FileDialog::save_file`
/// is the thing this must not be: it is `runModal`, a nested run loop, and a
/// panel that stops drawing.
///
/// **The future is awaited on a worker and so is everything after it**, which
/// is [`Keeping::save_set`]'s thread one act along and for its reason: a
/// bundle is a store read and every source inlined, then a file written, and
/// none of that is a thing to do on a frame (P-0091). The thread is detached
/// and no frame waits for it; the outcome comes back down a channel and is
/// said where a keep's is.
///
/// **The store is opened on the worker rather than handed in**, exactly as
/// [`Save::run`] does it: a `Store` is not what crosses the thread, a root is.
///
/// # Where the dialog opens, and what it is called
///
/// The name offered is `<id>.kbset` — the store's own naming rule, so nothing
/// is invented ([P-0096](../../../docs/principles/0096-the-operators-library-is-written-by-an-operators-own-act.md))
/// — and the directory is the one the Library bay is pointed at where a folder
/// has been dropped on this window (ADR-0275), and the platform's own default
/// where none has. **A file already there is the dialog's question and never
/// this program's**: asking again on this side would be two programs asking
/// one question, and the operator would have answered the wrong one first.
pub(crate) fn sending(
    window: &Arc<Window>,
    root: &std::path::Path,
    folder: Option<&std::path::Path>,
    id: &str,
    tx: std::sync::mpsc::Sender<Sent>,
) {
    let mut dialog = rfd::AsyncFileDialog::new().set_file_name(format!(
        "{id}{}",
        karakuri_store::store::Store::SET_FILE_SUFFIX
    ));
    if let Some(folder) = folder {
        dialog = dialog.set_directory(folder);
    }
    let asked = dialog.set_parent(&**window).save_file();
    let (id, root) = (id.to_owned(), root.to_path_buf());
    std::thread::spawn(move || {
        let answer = pollster::block_on(asked).map(|handle| handle.path().to_path_buf());
        let _ = tx.send(sent(&root, id, answer));
    });
}

/// **What the dialog's answer comes to**: a file written, or nothing at all.
///
/// Split out of [`sending`]'s thread so that the half with no window in it can
/// be run without one — the dialog is the platform's and the answer is a
/// `PathBuf` or it is `None`, which is the whole of what this needs to know.
///
/// **`None` writes nothing and nothing is opened**: the store is not read, no
/// bundle is built and no path is touched. That is the property `a_dismissed_dialog_writes_nothing_and_says_so`
/// is watched to fail against, and it is why the early return is here rather
/// than inside a `map` over the write.
pub(crate) fn sent(root: &std::path::Path, id: String, to: Option<std::path::PathBuf>) -> Sent {
    let Some(to) = to else {
        return Sent {
            id,
            to: None,
            outcome: Ok(()),
        };
    };
    let outcome = bundled(root, &id).and_then(|text| {
        std::fs::write(&to, text).map_err(|e| format!("writing `{}`: {e}", to.display()))
    });
    Sent {
        id,
        to: Some(to),
        outcome,
    }
}

/// **One Set as the bytes of a `.kbset`**, which is `karakuri-cli`'s
/// `packaged_set` with the authoring half taken out.
///
/// This side never packages a `.kset`: the id half is the whole of what a row
/// of a library listing can name, and the flag's other spelling is *take in
/// then send in one flag* ([ADR-0260](../../../docs/adr/0260-sending-a-set-is-a-read-and-a-reads-answer-goes-where-the-surface-that-asked-puts-answers.md)),
/// which is two presses here and already reached.
///
/// **`setfile::bundle` is where the inlining and its one refusal live**
/// (ADR-0231): one missing artifact refuses the whole thing and names the
/// node, because a bundle short of a procedure looks self-contained and is
/// not. Nothing here repeats that and nothing here loosens it.
pub(crate) fn bundled(root: &std::path::Path, id: &str) -> Result<String, String> {
    let store = Store::open(root).map_err(|e| format!("store `{}`: {e}", root.display()))?;
    Ok(setfile::bundle(&store, id)?
        .iter()
        .map(|line| format!("{}\n", line.as_str()))
        .collect())
}

/// **Put a library Set on a running deck**, which is the whole of what
/// `Operation::LoadSet` needed and is a re-point rather than an install.
///
/// # It writes files and sends a description, and it builds nothing
///
/// `Deck::install` is the one function that puts a built Set in a slot, and it
/// is *"deliberately not reachable from a key or a surface: a live run changes
/// its material by editing a file and letting the worker build it, which is
/// what the budget watchdog is attached to."* So this does what an operator
/// with an editor does, in one press: it reads the Set out of the store,
/// writes every procedure in it into the scratch, and tells that slot's
/// watcher to look there instead. **Everything after this line is the path a
/// save already takes** — the worker compiles off the render thread, the swap
/// lands at a frame boundary, and the watchdog judges it there on what one
/// frame of that Set costs and rolls it back on its own if that is over the
/// budget. The
/// library gets all of that for nothing, and no second route into a slot is
/// opened.
///
/// **Nothing here is on the frame path.** A store read, a `setfile::load` that
/// checks every procedure, and up to a handful of small writes — on the press,
/// which is where this file already reads a directory (`arrangement`), and
/// never on a frame (P-0091). The compile is the worker's.
///
/// # The scratch name carries the slot, and it is the directory's rule now
///
/// `scratch::place` writes `<store>/scratch/<name>.kir` and overwrites what is
/// there, so two decks loading two Sets whose procedures happen to share a
/// name would be one file: the second load would move the first deck as well,
/// on its watcher's next poll, and nothing would say why. The name is
/// therefore `A0-drift.kir` — the deck letter, the node's place in the Set,
/// and the procedure's own name — which is unique per slot **and** per node,
/// stays readable in an editor, and says which deck an open file belongs to.
///
/// **It is `scratch::node_name` rather than a `format!` here**, because that
/// argument was never about loading. It is about two decks and one directory,
/// which is every run: every slot is materialised under the same spelling at
/// startup ([`working_copies`]), so a load writes into a directory already
/// laid out this way and a second spelling would be a second answer.
///
/// # What the aim states, and why all of it
///
/// [`watch::Aim`] is `Watch::new`'s argument list less the slot, and every
/// field here is read off the Set file rather than left at this program's
/// startup value — which is the failure each of `Watch`'s own fields is
/// documented against, and which would not show on the load at all. A
/// layering, a fold or a camera left behind is a slot that loads correctly and
/// then comes back as a different picture on the first later save.
///
/// The authorities are the one exception and are empty: `Record::Authority` is
/// deliberately not Set-file state, so a Set carries no grants and a load
/// starts a slot with none — which is what `--load-set` gives one.
///
/// The `Err` is a sentence for the operator. Every way this fails leaves the
/// deck exactly as it was: a store that will not open, a Set that is not
/// there, a procedure in it that no longer checks, a scratch that will not be
/// written, or a worker that has gone.
pub(crate) fn loading(
    root: &std::path::Path,
    slot: usize,
    salt: u32,
    // **The slot's [`Aiming`] and not its sender**, so that where the watcher
    // is pointed is kept with what was sent. A load that sent an aim and left
    // `Aiming::at` behind would leave the next rewiring restating the pair the
    // run launched with — see [`Aiming`].
    aim: &mut Aiming,
    id: &str,
) -> Result<String, String> {
    let store = Store::open(root).map_err(|e| format!("{}: {e}", root.display()))?;
    let loaded = karakuri_environment::setfile::load(&store, id)?;
    // Said rather than swallowed: a note is the reader telling the operator
    // what it did with a file it could only partly honour, and a load that
    // quietly ignored one is a picture nobody can account for.
    for note in &loaded.notes {
        println!("  load: {note}");
    }

    let letter = deck_letter(slot as u8);
    let mut named = Vec::with_capacity(loaded.srcs.len());
    for (at, ((checked, src), name)) in loaded.nodes().zip(loaded.node_names()).enumerate() {
        let path = karakuri_environment::scratch::place(
            root,
            // **The directory's one naming rule, asked rather than spelled
            // again.** It was written out here when this was the only thing in
            // this program that wrote into the scratch; every slot is
            // materialised at startup now, so a second spelling of
            // `A0-drift.kir` would be a second answer to what a scratch file
            // is called — and the two would disagree on the day one of them
            // moved.
            &karakuri_environment::scratch::node_name(slot, at, &checked.name),
            src,
        )?;
        // **The Set file's node name and not the procedure's**, which is
        // `--load-set`'s own pairing: an `edge` in the file resolves against
        // the name the file wrote, and a rebuild that called the node whatever
        // its procedure declares would break the slot on its first save.
        named.push(karakuri_environment::compile::Named { name, path });
    }
    let mut named = named.into_iter();
    let head = named
        .next()
        .ok_or_else(|| format!("`{id}` names no procedure at all"))?;

    // **The file's salts, and a derived one where it recorded none** — the
    // rule `karakuri-cli`'s `salts_for` follows, restated here because that
    // program has no library target. The seed is the file's first salt where
    // it has one and this slot's own where it has not, so a Set that recorded
    // its colours comes back with them and one that did not is salted like the
    // slot it landed in.
    let seed_salt = loaded.salts.first().copied().flatten().unwrap_or(salt);
    let salts: Vec<u32> = (0..loaded.l1s.len())
        .map(|at| {
            loaded
                .salts
                .get(at)
                .copied()
                .flatten()
                .unwrap_or_else(|| karakuri_engine::set::derived_salt(seed_salt, at))
        })
        .collect();

    let nodes = loaded.srcs.len();
    aim.re_point(watch::Aim {
        head,
        rest: named.collect(),
        layering: loaded.layering,
        live: loaded.live,
        // **The first geometry's recorded capacity over all of them**, which
        // is what `--load-set` folds into `--capacity` and is `Watch`'s own
        // shape: one `Option<u32>` for the slot, because a rebuild recompiles
        // the files and each geometry's own declaration is the default. A Set
        // that recorded two different capacities loses the second, which is a
        // limit this program shares with the command line rather than one it
        // invented.
        capacity: loaded.capacities.first().copied().flatten(),
        seed_salt,
        salts,
        // A Set holds a built-in camera whatever its files declare, so
        // `Orbit::default()` where the file recorded none is the camera it
        // would have been built with rather than a value invented here.
        camera: loaded.camera.unwrap_or_default(),
        overrides: loaded.params,
        // Nothing in a Set file publishes a control — `setfile` writes none
        // and reads none — so this is empty for the same reason this program's
        // startup watchers pass an empty list: there is no `--publish` here to
        // fold in either (ADR-0216).
        published: Vec::new(),
        bindings: loaded.bindings,
        edges: loaded.edges,
        authorities: Vec::new(),
        // **What this slot is now running, and what every version it writes
        // from here on is filed under.** It is the id the operator picked out
        // of the library, carried on the aim because that is what a re-point
        // moves: left off, the loaded Set's whole chain would go on being filed
        // under the material the slot was running before the press, in names
        // nothing reads back (ADR-0276).
        set: Some(id.to_string()),
    })
    .map_err(|()| {
        format!(
            "deck {letter}'s build worker is gone, so `{id}` cannot be built; \
             what is on that deck keeps running"
        )
    })?;
    Ok(format!(
        "  load: deck {letter} <- `{id}` ({nodes} node{}) -> written into {}/{} and its watcher \
         re-pointed; the worker builds it and the budget judges it",
        match nodes {
            1 => "",
            _ => "s",
        },
        root.display(),
        karakuri_environment::scratch::DIR,
    ))
}

/// **Every arrangement the store holds, by name**, for the pill's menu to
/// list — [`library`] over the fourth directory rather than the first.
///
/// Its two rules are this one's, said again because they are the same two: a
/// store that is not there is listed as nothing and **is not created**, since
/// a program that listed a menu by first making a store would change the
/// directory it was run in; and a store that could not be read says so, since
/// a menu that is empty because the directory would not open looks exactly
/// like one that is empty because nobody has saved.
///
/// **Read when it changes rather than per frame.** Once at startup, and again
/// after a save lands — which is the only thing in this program that adds a
/// name. `Store::list_arrangements` sorts by name, so the menu draws what it
/// is handed and sorts nothing.
pub(crate) fn arrangements(root: &std::path::Path) -> Vec<String> {
    if !root.is_dir() {
        return Vec::new();
    }
    match Store::open(root).and_then(|store| store.list_arrangements()) {
        Ok(filed) => filed.into_iter().map(|entry| entry.name).collect(),
        Err(e) => {
            println!("arrangements: {} could not be listed: {e}", root.display());
            Vec::new()
        }
    }
}

/// **A star put on a Set or taken off it**, and the second route in this
/// program that both reads an operation and reaches a disk.
///
/// # It is [`arrangement`]'s shape and sits beside it for its reason
///
/// The panel cannot reach the store (ADR-0156) and the store cannot reach the
/// panel, so the two halves meet in a third party and this file is it. It
/// answers `None` for every other operation, which is what lets it sit on the
/// one path an emitted operation already takes rather than being a second
/// route into the store.
///
/// # What it writes, and what it deliberately does not
///
/// `Store::set_favourite` — one stat, one atomic write of
/// `<store>/favourites.json`, and the whole of the layout question is
/// ADR-0299's rather than this file's. **Nothing here re-lists**: the marks
/// the bay draws and the rows `my sets` holds are both [`listing`]'s answer,
/// and a second derivation here would be a second answer to *what is starred*
/// with a file write between them. The caller re-lists on the same branch it
/// re-lists a scope press on.
///
/// # Who asked decides where it lands, and for a star there is nowhere else
///
/// [P-0096](../../../../docs/principles/0096-the-operators-library-is-written-by-an-operators-own-act.md)
/// is the actor and not the flag, and `my sets` is by construction the list of
/// Sets **the operator chose** — so a model's star must not reach
/// `<store>/favourites.json`. That much is
/// [ADR-0261](../../../docs/adr/0261-a-model-asked-save-lands-in-a-sandbox-because-the-operators-library-is-the-operators-own-act.md)'s
/// rule applied one control along, and it is why this branches on [`Asked`]
/// exactly as `karakuri_environment::filed_as` does for a save.
///
/// **Where the two part company is the second directory.** A model's save
/// lands in `<store>/sandbox/` because what lands there is *material* — an
/// edit-history snapshot an operator goes looking for after a show — so
/// refusing it would lose an evening of work. A star is one bit whose whole
/// meaning is *this row appears under `my sets`*, so a sandbox favourites file
/// would be a list no scope lists, no tool reads and the operator never sees,
/// while the model was told it had succeeded. **So a model's star is refused
/// out loud** (ADR-0301), and the refusal names the id and says where the Set
/// is — which is the same shape the class pills' refusals take, so that a
/// model can tell the person beside it which mark to press.
///
/// **`Standing::Open` stays and `gate.rs` is untouched.** The refusal is the
/// performer's and not the gate's, exactly as a model's save is not refused at
/// the gate but filed somewhere else by whoever performs it.
///
/// **The model arm is written before the route is**, which is [`arrangement`]'s
/// own position: no tool publishes `SetFavourite` today, the page's MCP badge
/// is `plan`, and a control that arrives at this function finds the rule
/// already here rather than adding it.
///
/// # The three things it can say, and each is said out loud
///
/// **The state was already the one asked for**, which is `Ok(false)` and is an
/// ordinary answer rather than a refusal: the operation names a state and not
/// a toggle, so a second press of *star this* says the same thing again and
/// the file's own time is not touched. **The Set is not one this store
/// holds**, which is `StoreError::NoSet` carrying the id back
/// ([P-0083](../../../docs/principles/0083-a-refusal-carries-what-the-next-attempt-needs.md))
/// — a `presets` or a `folder` row is a file rather than a Set of this
/// library's, and starring one is refused with the sentence saying so.
/// **Taking a star off is never refused**, which is the asymmetry that makes a
/// mark left behind by a file somebody deleted clearable from the row it no
/// longer draws.
///
/// **Never panics**, for [`arrangement`]'s reason: a panic reachable from an
/// event handler aborts this process rather than unwinding.
pub(crate) fn favourite(
    root: &std::path::Path,
    asked: Asked,
    operation: &Operation,
) -> Option<String> {
    let Operation::SetFavourite { id, favourite } = operation else {
        return None;
    };
    if asked == Asked::Model {
        return Some(format!(
            "star: `{id}` was not {} — `my sets` is the list of Sets the operator chose, and a \
             star is theirs to put on: it is one press on the mark at the left of that row in \
             the Library bay, under `all`. The Set itself is untouched and is listed there",
            match favourite {
                true => "starred",
                false => "unstarred",
            }
        ));
    }
    let wrote = Store::open(root).and_then(|store| store.set_favourite(id, *favourite));
    Some(match wrote {
        Ok(true) => format!(
            "star: `{id}` {} — `my sets` is the starred subset of `all`, and this row is {} \
             it",
            match favourite {
                true => "is starred",
                false => "has its star off",
            },
            match favourite {
                true => "in",
                false => "out of",
            }
        ),
        Ok(false) => format!(
            "star: `{id}` was already {}, so nothing was written",
            match favourite {
                true => "starred",
                false => "unstarred",
            }
        ),
        Err(e) => format!(
            "star: `{id}` was not {}: {e}",
            match favourite {
                true => "starred",
                false => "unstarred",
            }
        ),
    })
}

/// **Where a named arrangement is kept and put back**, and the one route in
/// this program that both reads an operation and reaches a disk.
///
/// # Why it is here, in a package neither side depends on
///
/// The panel cannot reach the store. `karakuri-console` dropped
/// `karakuri-store` when this program moved out of it, and the drop was the
/// point — a crate that takes no device and no disk is what ADR-0156 bought,
/// and its manifest now has no entry that could be reached for at all. The
/// store cannot reach the panel either: `karakuri-store`'s `src/` must not
/// name `karakuri-layout`, so it keeps an arrangement as bytes it does not
/// understand, exactly as it keeps `.kir` source
/// ([ADR-0221](../../../docs/adr/0221-an-arrangement-is-named-by-the-operator-and-kept-in-a-fourth-place.md)
/// §4). **So the two halves meet in a third party, and this file is the third
/// party** — the same position it holds for a record, where the vocabulary
/// says what to write and only somebody holding a `Deck` can apply it
/// ([`apply`]).
///
/// # The route, and where each half of it is decided
///
/// - **Saving** is `serde_json::to_vec` of [`Panel::layout`] into
///   `Store::write_arrangement`, which is ADR-0221's own sentence. The format
///   is `karakuri-layout`'s hand-written `Serialize`, so an unbounded maximum
///   goes out as an explicit absence rather than as an infinity JSON cannot
///   spell, and a `NodeId` goes out as the bare number it is.
/// - **Putting one back** is `Store::read_arrangement`, `serde_json` into a
///   [`Layout`], and [`Panel::restore`]. **Neither this file nor the panel
///   checks the arrangement**: `Layout`'s `TryFrom<Wire>` is the one place a
///   file that disagrees with itself is refused rather than repaired
///   (ADR-0158), and a check here would be a second answer to a question that
///   already has one.
///
/// # What it does with each of the three ways it can fail
///
/// **Says it and moves nothing**, and never panics: a panic reachable from an
/// event handler aborts this process rather than unwinding (see the module
/// documentation). The three are a store it could not open or write, a name
/// nothing is filed under, and a file that will not read back — and the third
/// is the one that has to be told apart from the second, because *there is no
/// such arrangement* and *the arrangement you saved is broken* send an
/// operator to two different places.
///
/// **A name nothing is filed under never falls back to the default.**
/// `Store::read_arrangement` answers `StoreError::NoArrangement(name)` and
/// that sentence carries the name, which is the whole reason the store has a
/// fourth error variant rather than reusing `NotFound`: an operator who
/// mistyped a name needs to be told the name, not to watch their console reset
/// (ADR-0221 §2).
///
/// # The store is created by a save and not by a restore
///
/// [`library`] refuses to create one, because *"a program that listed a
/// library by first making one would change the directory it was run in"*, and
/// a restore is a read on exactly those terms. A **save** is the case
/// `Store::open` establishing the layout is right for — it is a program that
/// is about to write — so the two halves below differ, deliberately, and the
/// restore's guard is what keeps `cargo run -p karakuri` in somebody's home
/// directory from leaving a `.karakuri` behind for having asked a question.
///
/// # It answers `None` for every other operation
///
/// Which is what lets it sit on the one path every emitted operation already
/// takes ([`App::performed`]) rather than being a second route into the
/// panel. **The transport row's arrangement pill emits both**, and the
/// manual's two rows say it is the only one of the four surfaces that can: a
/// `panel` badge each and three empty ones, because a name is what a key
/// press, a map line and an unpublished tool each have no way to say. This
/// wiring was written before that control existed — exactly as [`unwritten`]
/// is written for controls that do not exist yet — and the control is what
/// arrived at it.
pub(crate) fn arrangement(
    root: &std::path::Path,
    panel: &mut Panel,
    arr: &mut view::Arrangement,
    operation: &Operation,
) -> Option<String> {
    match operation {
        Operation::SaveArrangement { name } => {
            let (line, kept) = keep_arrangement(root, panel, name);
            // **The name in use moves only when the file did.** A save that
            // was refused leaves the pill saying what it said, because
            // nothing under that name is on the disk — and the listing is
            // re-read only then, since a refusal added no name to it.
            if kept {
                arr.name = Some(name.clone());
                arr.filed = arrangements(root);
            }
            Some(line)
        }
        Operation::RestoreArrangement { name } => {
            let (line, back) = put_arrangement_back(root, panel, name);
            if back {
                arr.name = Some(name.clone());
            }
            Some(line)
        }
        _ => None,
    }
}

/// The running arrangement, filed under `name` — and whether it landed. See
/// [`arrangement`].
pub(crate) fn keep_arrangement(
    root: &std::path::Path,
    panel: &Panel,
    name: &str,
) -> (String, bool) {
    if let Err(refusal) = checked_name(name) {
        return (refusal, false);
    }
    let bytes = match serde_json::to_vec(panel.layout()) {
        Ok(bytes) => bytes,
        Err(e) => {
            return (
                format!("arrangement: `{name}` was not kept — it did not serialise: {e}"),
                false,
            )
        }
    };
    let wrote = Store::open(root).and_then(|store| store.write_arrangement(name, &bytes));
    match wrote {
        Ok(()) => (
            format!(
                "arrangement: kept as `{name}` — {} bytes at {}",
                bytes.len(),
                root.join("arrangements")
                    .join(format!("{name}.arrangement.json"))
                    .display()
            ),
            true,
        ),
        Err(e) => (format!("arrangement: `{name}` was not kept: {e}"), false),
    }
}

/// **The one place a typed arrangement name is refused**, and the reason it is
/// here rather than in the pill that took the letters.
///
/// `<name>` becomes one path component under `<store>/arrangements/`, and
/// `karakuri-store` says outright that **nothing there checks it**: *"`<name>`
/// becomes one path component and that is the caller's rule to keep"*
/// ([ADR-0221](../../../docs/adr/0221-an-arrangement-is-named-by-the-operator-and-kept-in-a-fourth-place.md)
/// §1, which names letters, digits, `-` and `_`). So `../../elsewhere` is a
/// path, and a path never reaches that call from here.
///
/// **The surface owns the affordance and never the authority**
/// ([P-0090](../../../docs/principles/0090-a-surface-offers-it-never-decides.md)):
/// the pill takes whatever is typed and this is where it meets the wall, so a
/// name refused by a hand and a name refused by anything else that ever
/// reaches this operation meet the same one. A pill that silently dropped the
/// characters it did not like would be a rule an operator could only find by
/// experiment — which is the failure the console page names about a control
/// that quietly declines.
///
/// **It says the same three things `mcp::checked_id` says about a Set id**,
/// which is [P-0090](../../../docs/principles/0090-a-surface-offers-it-never-decides.md)
/// as far as it can be kept today and no further: that function is private to
/// `karakuri-environment`'s `mcp` module and its sentences say `id` and
/// `<store>/sets/`, so it cannot be called from here and could not be quoted
/// if it were. **When an arrangement name gets a second surface — a map line,
/// an MCP tool, a `--restore-arrangement` flag — the two collapse into one
/// shared `checked_name`, and this comment is where whoever does it should
/// start.**
pub(crate) fn checked_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err(
            "arrangement: nothing was typed, and an arrangement is filed under a name — \
             the default arrangement is the one that has none"
                .to_owned(),
        );
    }
    match name
        .chars()
        .find(|c| !c.is_ascii_alphanumeric() && *c != '-' && *c != '_')
    {
        Some(bad) => Err(format!(
            "arrangement: `{name}` holds `{bad}`, and an arrangement name is letters, digits, \
             `-` and `_`: it is one path component and it names a file under \
             `<store>/arrangements/`"
        )),
        None => Ok(()),
    }
}

/// The arrangement filed under `name`, into the window the panel already has.
/// See [`arrangement`].
pub(crate) fn put_arrangement_back(
    root: &std::path::Path,
    panel: &mut Panel,
    name: &str,
) -> (String, bool) {
    if !root.is_dir() {
        return (
            format!(
                "arrangement: no store at {}, so nothing is filed under `{name}` — and the \
                 console has not moved",
                root.display()
            ),
            false,
        );
    }
    let bytes = match Store::open(root).and_then(|store| store.read_arrangement(name)) {
        Ok(bytes) => bytes,
        Err(e) => return (format!("arrangement: `{name}` is not back — {e}"), false),
    };
    let layout: Layout = match serde_json::from_slice(&bytes) {
        Ok(layout) => layout,
        // **Refused whole rather than repaired**, which is the loader's own
        // sentence and not this file's judgement (ADR-0158).
        Err(e) => {
            return (
                format!(
                    "arrangement: `{name}` is not back — the file disagrees with itself and is \
                     refused rather than repaired: {e}"
                ),
                false,
            )
        }
    };
    let viewport = panel.layout().viewport();
    panel.restore(layout);
    (
        format!(
            "arrangement: `{name}` is back, at the {} x {} this window already had",
            viewport.w, viewport.h
        ),
        true,
    )
}
