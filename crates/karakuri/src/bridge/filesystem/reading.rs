use super::*;

/// What one Set declares, read off the cards the store keeps beside its
/// artifacts — the answer to
/// [`Operation::ReadSet`](karakuri_operation::Operation::ReadSet), in the shape
/// the Library bay draws it in.
///
/// # It is the same reading the MCP tool gives a model, through the same path
///
/// `karakuri_mcp::read_set` renders this for a model, and what it reads is the
/// Set file's `slot` records and each artifact's metadata card —
/// `Store::read_set`, then `Store::read_meta` per node, then the `param_decl`,
/// `capacity_decl` and `emit` records on it. This walks the same records rather
/// than a second source: a panel and a model that disagreed about what a Set
/// declares would be two answers to one question. What differs is the
/// rendering, and it differs because the destinations do: a model is handed
/// prose it reads in a context window and a bay is handed one line per control
/// in a column eight characters wide.
///
/// The prose renderer is not called, and could not be: it returns one `String`
/// per Set with its blocks already laid out in sentences, and a listing row
/// wants the key and the range apart. Lifting a structured reading into
/// `karakuri-environment` so that both surfaces render one value is the right
/// shape and is a change to a crate this pass may not touch; what is here is
/// written against the same records in the same order so that the day somebody
/// does, this is what moves.
///
/// # The three blocks the row promises, and the fourth that is not here
///
/// *"Every knob with its range and default, the element count, the attributes
/// emitted — each read off the artifact's own card, so those three fetch no
/// source and compile nothing."* The three are each a record on a card. The
/// element storage is deliberately absent: the MCP tool's
/// `element_storage_block` calls `setfile::load`, which fetches every source in
/// the Set and runs `compile::check` over it, so a panel that drew it would be
/// paying exactly the cost that sentence says these three do not.
/// `docs/manual/console.html`'s note says the same and says where the figure
/// goes instead — *"A model asking over MCP gets it and pays for it; a press in
/// a library list is not the place to spend that"* — which the row settles the
/// same way.
///
/// # One control per key, and the range is the one every node agrees to
///
/// A Set publishes one control per *key* and not one per declaration
/// (`docs/ir-spec.md`, and `karakuri_engine::set::Set::published`), so a name
/// two nodes declare is one row over the part of the range both of them accept.
/// That intersection is `Set::declared_range`'s own arithmetic — `lo.max(min)`,
/// `hi.min(max)` — done here because a built Set is what this chip exists to be
/// pressed before: `published()` needs an engine and a device, and a reading
/// that took one would be the load it is meant to save.
///
/// The default is the first declarer's, and the order is the file's own — the
/// same order `read_set` prints its nodes in and the order `Set::published`
/// walks. A default is one number and two nodes may declare two; the
/// alternative is drawing neither, which is a blank row and is the one thing
/// the mock's own tip refuses.
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

/// A declared range and the default that applies until something turns it, in
/// the shape the mock draws every line of a reading in: `0 – 8 · 2`.
///
/// The three marks are the mock's own — an en dash between the ends of the
/// range and a middle dot before the default — and both are in the face the
/// panel draws with, which is asserted rather than assumed
/// ([`the_marks_a_reading_is_spelled_with_are_in_the_face`]): the Library bay's
/// foot read `load → A` with the arrow typed as a U+2192 the default face does
/// not carry, and drew `load □ A` for a release. The arrow is drawn rather than
/// typed now, and is a label between two capsules (ADR-0305).
///
/// A default that is not a literal is a word and not a blank. The card's
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

/// The reading under the cursor, read and written into the view, and the
/// sentence to print about it.
///
/// [`listing`]'s shape one control along, and it is here for that function's
/// reason: reading a Set file and the cards behind it is a disk read, this is
/// the side of the seam that owns the store, and `karakuri-console` takes none
/// of the three (ADR-0156). On the press and never on a frame (P-0091) — which
/// is the press on the `params` chip, and the key that moves the cursor while a
/// reading is open, because the reading follows the cursor.
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

/// The reading follows the cursor, on whichever surface moved it.
///
/// `karakuri-console/src/view.rs` states the rule on `View::reading_open`: *"a
/// move with one open is a read of the row it arrived at, and a move with
/// nothing open is a pointer moving"*. Two surfaces move that cursor, the arrow
/// keys through `View::walk` and a carry's press through `View::point_at`, and
/// both owe it the same read — this is the one place that read is written, so
/// there is one implementation of the rule for both to call rather than two
/// copies that could answer it differently.
///
/// `moved` is each caller's own answer to *did this press move the cursor*: the
/// arrow-key arm compares `View::cursor_row()` before and after the press, and
/// the carry's arm is `matches!(acted, Acted::Pointed)`. Neither shape is
/// repeated here, because *what counts as a move* is each surface's own
/// question and this function's only question is what to do once one has
/// happened.
///
/// # The defect this rule exists to prevent
///
/// Until ADR-0265, `Readout::took` discarded `View::point_at`'s `moved`. A
/// carry taken in hand while a reading was open on a different row moved the
/// cursor off it, `View::opened` answered `None` because the row under the
/// cursor was no longer the Set the reading was of, and the block vanished with
/// nothing on that route ever walking the cursor back — no panic, no
/// diagnostic, a reading that stopped being drawn. The arrow keys never had the
/// bug — they always re-read on `moved && reading_open()` — so the two call
/// sites already agreed before this function existed; what it buys is that they
/// cannot silently stop agreeing.
///
/// Returns the line to print rather than printing it, so a caller with nothing
/// to print — the ordinary case, a press with no reading open — pays for no
/// `println!` and a test can call this with no stdout to capture.
pub(crate) fn reread_if_open(
    moved: bool,
    view: &mut View,
    store: &std::path::Path,
) -> Option<String> {
    (moved && view.reading_open()).then(|| read_reading(view, store))
}

/// The record's layer a vocabulary layer names.
///
/// A third spelling of a list that already has two conversions in
/// `karakuri-environment` — `setfile::kind_of` and `mcp.rs`'s pair — and it is
/// here because both of those are private to that crate and neither is on its
/// way out. What crosses the seam is the summary, whose nodes carry a
/// `karakuri_store::record::Layer`, and what a filter carries is a
/// `karakuri_operation::Layer`; the comparison has to happen on one side.
///
/// Exhaustive with no wildcard, which is what makes it a table rather than a
/// guess: a sixth layer does not compile until somebody says which it is.
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

/// The word a `kind` line spells a layer with, as the vocabulary's own value —
/// `karakuri_environment::history::LAYERS`' six words read back.
///
/// `None` for a word that is not one of the six, which is a `.kir` declaring no
/// kind at all: `declared_kind` already answers `None` there, and this is the
/// same absence carried one step further rather than a second reading of it.
///
/// The wildcard is the risk here and it is why `LAYERS` is the list. This match
/// answers a word rather than a value, so a sixth kind does *not* stop the
/// build — it falls to `None` and a `kind L5` file reads as one declaring
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

/// Which kinds the filter row is showing, as a sentence — the words of the
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
