use karakuri_ir::typed::Checked;
use karakuri_operation::Operation;
use karakuri_store::hash::Hash;
use karakuri_store::ndjson::Line;
use karakuri_store::record::{Layer, Record};
use karakuri_store::store::{Store, StoreError};
use serde_json::Value;

use super::super::*;
use super::*;

pub(crate) fn swap_outcome(state: &mut State) -> Result<String, String> {
    state.drain();
    let dropped = state.dropped.load(std::sync::atomic::Ordering::Relaxed);
    let missing = if dropped == 0 {
        String::new()
    } else {
        format!(
            "\n\n({dropped} earlier report{} were dropped for want of room — this is \
                 not the whole history)",
            if dropped == 1 { "" } else { "s" }
        )
    };
    Ok(if state.recent.is_empty() {
        format!(
            "nothing has swapped, been overloaded or failed to build since this run \
             started.{missing}"
        )
    } else {
        format!("{}{missing}", state.recent.join("\n"))
    })
}

/// `walk_history`'s arguments as the operation they name.
///
/// **`set` is required where the payload's own field is an `Option`**, and the
/// two do not disagree: `None` there is *a walk that names no Set*, which is
/// the panel's answer for a deck running the pair the run launched with — those
/// versions are filed under no Set and a narrowing matches none of them
/// (ADR-0276). A model asking for that would be asking for a listing that is
/// empty by construction, so this surface does not offer it and says so.
///
/// **[`checked_id`] for [`named_set`]'s reason**: a Set id is one path
/// component, and the walk matches it against what the store filed a version
/// under.
pub(crate) fn walked_history(args: &Value) -> Result<Operation, String> {
    let set = args.get("set").and_then(Value::as_str).ok_or(
        "`set` is required and is a string: which Set's versions to walk. A walk is \
         narrowed to one Set — `list_sets` names the ids — because two decks playing one \
         Set have one history between them and a version written under no Set is matched \
         by no id",
    )?;
    Ok(Operation::WalkHistory {
        set: Some(checked_id(set)?),
    })
}

/// **How many rows the walk asks for before the narrowing**, and it is a
/// number this surface chooses rather than one `history::list` has.
///
/// `list`'s cap is on **days opened** and the narrowing to one Set happens
/// after it, so a store whose day directories hold several Sets' versions
/// yields fewer of each — asking for two hundred is asking for the last two
/// hundred versions *this store* wrote, of which some are the Set that was
/// asked about. Larger than the twenty a reply renders, so an ordinary Set's
/// recent history survives the narrowing whole; small enough that the walk
/// stops after a handful of day directories, which is the whole of what it
/// costs
/// ([P-0091](../../../docs/principles/0091-cost-is-known-before-it-is-paid.md)).
/// What lies past it is not counted — counting it is the cost the cap exists
/// not to pay — and `Listing::stopped_short` is what says the walk stopped.
pub(crate) const WALKED: usize = 200;

/// **Every version of one Set that compiled**, most recent first, as rows a
/// landing can name back.
///
/// **A read, answered on this thread**, which is why the row is a tool beside
/// `read_set` and `list_sets` rather than a name `operate` takes: it walks
/// `<store>/history/` and opens no file at all, and a directory walk handed to
/// the render loop is a directory walk on the path that must not wait
/// (`docs/adr/0342-…`, ADR-0199).
///
/// **Narrowed to one Set here rather than in `history::list`**, which is that
/// module's own rule — *"which rows an operator is looking at is a question the
/// surface asks"* — and the same division `list_sets`' two filters are applied
/// under. **A row filed under no Set matches no id** and is never folded in:
/// those versions were written where the slot was running the pair a run
/// launched with, and a filter that let them through would be inventing a
/// history for whichever Set was asked about (ADR-0276, ADR-0308).
///
/// **The three things a walk has to say beside its rows** are said: the walk
/// stopping short, the entries under `history/` the layout does not claim, and
/// a rendering shorter than what matched. Each of the three is a way for a
/// listing to read as the whole of something it is not.
pub(crate) fn walk_history(set: Option<&str>, state: &State) -> Result<String, String> {
    // Cannot happen from this surface — [`walked_history`] requires `set` — and
    // written out rather than unwrapped, because the payload's `None` is a real
    // value on another surface and this is what it would mean here.
    let Some(id) = set else {
        return Err(
            "this walk names no Set, and a walk of no Set lists nothing: versions written \
             while a slot was running material nobody had saved are filed under no Set, and \
             no id matches them. Name a Set — `list_sets` says which ones this store holds"
                .to_string(),
        );
    };
    let found = karakuri_environment::history::list(&state.store, WALKED)?;
    let rows: Vec<&karakuri_environment::history::Version> = found
        .versions
        .iter()
        .filter(|version| version.set.as_deref() == Some(id))
        .collect();
    let mut out = String::new();
    if rows.is_empty() {
        out.push_str(&format!(
            "no version of set `{id}` is in this store's edit history. Every build that \
             compiles is filed there, so this is a Set nothing has been edited on in what \
             the walk covers — or one this store has never played. `read_set` says what \
             `{id}` holds and `list_sets` says what else is here.\n"
        ));
    } else {
        let shown = rows.len().min(LISTED);
        if rows.len() > shown {
            // **Never a truncated list that reads as a whole one**, which is
            // [`list_sets`]' rule on its own truncation.
            out.push_str(&format!(
                "{} version{} of set `{id}` are in what this walk covered, and the {shown} \
                 most recent are below — **this is not all of them**: {} more matched and \
                 are not listed.\n",
                rows.len(),
                plural(rows.len()),
                rows.len() - shown,
            ));
        } else {
            out.push_str(&format!(
                "{} version{} of set `{id}`, most recent first — all of the ones this walk \
                 covered are below.\n",
                rows.len(),
                plural(rows.len()),
            ));
        }
        for version in rows.iter().take(shown) {
            out.push_str(&format!("`{}`\n", version.filed_as()));
        }
    }
    if found.stopped_short {
        out.push_str(
            "\nThe walk stopped with day directories unread, so this is part of what is \
             there rather than all of it: it asks for the last few hundred versions this \
             store wrote and narrows them to the Set afterwards, so a store several Sets \
             are being edited in shows fewer of each.\n",
        );
    }
    if found.unclaimed > 0 {
        out.push_str(&format!(
            "\n{} entr{} under `history/` that this layout does not claim {} passed over. A \
             day directory is a place an operator works in by hand — `rm -rf \
             history/2026/07` is this store's whole retention policy — so whatever else is \
             in there is theirs.\n",
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
    // **The closing paragraph is the rows' own and is not printed under an
    // empty answer**, where every sentence in it would be about something that
    // is not there.
    if !rows.is_empty() {
        out.push_str(
            "\nEach row is the name the store filed a version under: when it was written, \
             which slot, which node of that slot, and what the procedure called itself. To \
             put one back, call `operate` with `Put a node's previous version back` and \
             `{\"deck\": <the row's slot>, \"revision\": {\"picked\": \"<row>\"}}` — the \
             version's bytes are written over that node's working copy and built like any \
             other edit. The gate on this history is **compiling** and not landing, so a \
             version that cost too much to run is in here too.\n",
        );
    }
    Ok(out)
}

/// The `s` on a count, in the one place, because every sentence here has one.
pub(crate) fn plural(n: usize) -> &'static str {
    if n == 1 {
        ""
    } else {
        "s"
    }
}

/// **What this Set will allocate to hold its elements**, node by node and in
/// total — computed from the Set file and a compile pass, with nothing built,
/// no adapter opened and no GPU touched.
///
/// **A figure with a producer and no reader is how the last wrong number in
/// this program got published.** `Set::element_storage` has reported this per
/// node since the buffers existed and no binary in this tree printed it; the
/// figure that *was* printed, at stage 4, was a second arithmetic over one
/// procedure's `emit` list — it claimed 96 bytes per element where 312 were
/// allocated, and it was withdrawn rather than corrected — see
/// `docs/adr/0116-stage-four-stops-claiming-the-byte-figure.md`.
/// So this one is not a second arithmetic: `Plan::element_storage` calls
/// the same sizing the allocation calls, over the same walk a build makes, and
/// a test in `karakuri-engine` asserts the two answers about one Set are equal.
///
/// **Every number needs the whole Set and not one card**, which is why this is
/// a block of its own rather than a line inside [`node_block`]. What a node
/// allocates depends on what *reaches* it: an L2 writes everything upstream
/// emitted as well as its own `emit`, an `amplify` above a node multiplies the
/// element count for everything below it, and an L1 that can `kill()` pays for
/// a buffer its text never mentions. A per-node figure read off a card would be
/// wrong in exactly the three ways the withdrawn one was.
///
/// **Not computed at all, rather than computed from a guess**, wherever the
/// Set does not check out: a Set naming an artifact this store has not got, or
/// one whose nodes do not compose, has no figure — and being told which is more
/// use than a number that assumed its way past the problem.
pub(crate) fn element_storage_block(store: &Store, id: &str) -> String {
    // **The same shape as the "no card" branch of [`node_block`]**: a Set this
    // cannot cost is an ordinary thing to meet in a working store rather than a
    // failed call, and what a model is owed is the sentence saying which half
    // is missing.
    let unavailable = |why: &str| {
        format!(
            "element storage: not computed — {why}. This is the one figure here that \
             needs the set to hold together as a whole, because what a node allocates \
             depends on what reaches it; everything above is read off each artifact's \
             own card and stands on its own.\n"
        )
    };
    let loaded = match karakuri_environment::setfile::load(store, id) {
        Ok(loaded) => loaded,
        Err(why) => return unavailable(&why),
    };
    // **What each geometry runs at: the file's number, or the procedure's own
    // default where the file names none** — `capacity [min, max] = default`,
    // and the default is what the spec says applies. The other branch of
    // `capacity_for` is `--capacity`, an operator's flag overriding both, and
    // there are no flags in an MCP call: this answer is about a file.
    let mut sources: Vec<(&Checked, u32)> = Vec::with_capacity(loaded.l1s.len());
    for (at, l1) in loaded.l1s.iter().enumerate() {
        match loaded
            .capacities
            .get(at)
            .copied()
            .flatten()
            .or_else(|| l1.capacity.map(|declared| declared.default))
        {
            Some(capacity) => sources.push((l1, capacity)),
            None => {
                return unavailable(&format!(
                    "`{}` declares no `capacity` and this set records none for it, so \
                     there is no element count to size anything against",
                    l1.name
                ))
            }
        }
    }
    let l2s: Vec<&Checked> = loaded.l2s.iter().collect();
    let l3s: Vec<&Checked> = loaded.l3s.iter().collect();
    let fields: Vec<&Checked> = loaded.fields.iter().collect();
    let l4s: Vec<&Checked> = loaded.l4s.iter().collect();
    let plan = match karakuri_engine::Set::validate(
        &sources,
        &l2s,
        &l3s,
        &fields,
        &l4s,
        // **What `--load-set` builds this file as**, which the file itself now
        // records — a `merge` record, or its absence for a Set that overdraws.
        // Read from the file rather than defaulted, so this refuses a
        // composited Set with more renderers than a fold can hold exactly where
        // loading it would. The choice changes no *number* here either way:
        // compositing costs a render target per renderer, and a render target
        // is not element storage.
        loaded.layering,
        // A salt decides what the elements *are* and never how many bytes they
        // take, so the set's own is enough here and a source deriving one from
        // it changes nothing this block prints.
        loaded.salts.first().copied().flatten().unwrap_or_default(),
        &loaded.salts,
        karakuri_engine::set::Wiring {
            l1s: &loaded.names.l1s,
            l2s: &loaded.names.l2s,
            l3s: &loaded.names.l3s,
            l4s: &loaded.names.l4s,
            fields: &loaded.names.fields,
            edges: &loaded.edges,
        },
    ) {
        Ok(plan) => plan,
        Err(e) => return unavailable(&format!("this set does not build: {e}")),
    };

    let planned = plan.element_storage();
    let names = plan.node_names();
    let total: u64 = planned.iter().map(|p| p.storage.bytes).sum();
    let mut out = format!(
        "element storage: {total} bytes in total, across the {} node{} of this set that \
         hold elements, at the capacities the file records. Nothing was built to find \
         that out: it is the arithmetic the allocation itself is sized by, run over the \
         file.\n",
        planned.len(),
        if planned.len() == 1 { "" } else { "s" },
    );
    for entry in &planned {
        out.push_str(&format!(
            "  `{}` — {} bytes for {} element{}, {} bytes each\n",
            names[entry.node],
            entry.storage.bytes,
            entry.storage.capacity,
            if entry.storage.capacity == 1 { "" } else { "s" },
            entry.storage.per_element(),
        ));
    }
    // **A repeated name is not a mistake and has to say so.** A set over two
    // geometries instantiates its whole chain of deformations once per
    // geometry, so one deform procedure is two nodes with buffers of their own
    // — and they are different sizes whenever the geometries are.
    if planned
        .iter()
        .enumerate()
        .any(|(at, entry)| planned[..at].iter().any(|seen| seen.node == entry.node))
    {
        out.push_str(
            "A name appears twice above because this set has more than one geometry: the \
             chain is instantiated once per geometry, and each instance holds buffers of \
             its own.\n",
        );
    }
    // **What the number means, for a reader who has never seen this system.**
    // The withdrawn figure was as wrong in what it was taken to mean as in its
    // arithmetic, and a number relayed as "what this costs a GPU" would be that
    // mistake in a new costume. A renderer having no row at all is part of the
    // same sentence: it draws from the buffer the node above it allocated, so a
    // row for it would be that memory counted twice.
    out.push_str(
        "What that covers: one element struct per element, the four-byte liveness flag \
         beside it, the second copy a geometry keeps so it can read what it wrote last \
         step, and the destination index a geometry that spawns or kills pays for. A \
         renderer and a camera hold no elements and so have no row. It is NOT what this \
         set costs a GPU: render targets, uniform blocks and every other buffer not \
         indexed by an element are outside it, so it is a floor on device memory and \
         never the figure to allocate against. What it is exactly is the cost of one \
         more element — the per-element numbers above are exact divisions rather than \
         averages.\n",
    );
    out
}

/// One node of a Set: its address in the Set, its artifact, and its card.
///
/// **What it is called is [`crate::setfile::node_called`]'s answer**, and this
/// is the function that used to decide it. `list_sets` names the same node in a
/// listing and a model has to find, when it reads the Set, the node the listing
/// told it about — so the three candidates are weighed in one place and read
/// here rather than weighed a second time.
pub(crate) fn node_block(
    store: &Store,
    layer: Layer,
    index: u32,
    name: Option<&str>,
    hash: &Hash,
) -> String {
    // **Twelve hex characters and not sixty-four.** A hash is not an argument
    // anything here takes — see [`read_set`] — so what this is for is telling
    // two nodes apart and recognising the same artifact in two Sets, which
    // twelve does at a length a reader can hold. `Hash::short` is the same
    // shortening every log line in this program uses.
    let short = hash.short(12);
    let address = format!("{}:{index}", layer_spelled(layer));
    // The head of one block: the address, what the node is called, and the
    // address its source is stored under — except where the name *is* that
    // address, which is what a node with no name of its own and no card to
    // declare one gets, and saying it twice adds nothing to saying it once.
    let head = |called: &str| {
        if called == short {
            format!("{address} `{short}`")
        } else {
            format!("{address} `{called}` — stored as {short}")
        }
    };
    match store.read_meta(hash) {
        Ok(card) => {
            let (declared, body) = rendered_card(&card);
            let called =
                karakuri_environment::setfile::node_called(name, declared.as_deref(), hash);
            // **What did not win, where something had to lose.** A Set's own
            // name for a node hides the name the procedure gives itself, and a
            // model choosing between saved material wants both: the one this
            // set addresses the node by, and the one that identifies the
            // artifact wherever else it appears.
            let also = match &declared {
                Some(declared) if *declared != called => {
                    format!(", and the artifact calls itself `{declared}`")
                }
                _ => String::new(),
            };
            format!("{}{also}\n{body}", head(&called))
        }
        // **Not an error, and it must not read as one.** `Store::read_meta`
        // answers `NotFound` for a card that was never written, which is an
        // ordinary state of a working store rather than damage: a card is
        // derived, `Store::put_artifact` writes none of its own — it takes bytes
        // and does not compile — and an artifact stored before cards existed has
        // none either. A model told "not found" would report a broken library;
        // what it is owed is the sentence that says the source is there and the
        // description is not.
        //
        // **Two absences, and the pair is worth the extra read.** A hash with no
        // card and a hash this store has never seen are the same `NotFound` from
        // here and completely different facts: the second means the Set was
        // written against another store and will not load here at all, which is
        // the more useful thing anyone could be told and is invisible if both
        // say "no card". The artifact is only fetched on this branch, so the
        // ordinary path pays nothing for it.
        Err(StoreError::NotFound(_)) => {
            let standing = if store.get_artifact(hash).is_err() {
                "this store does not hold that artifact at all, so nothing here can \
                 say what it declares and `--load-set` could not build this set \
                 either — the set was saved somewhere else, or beside a store that \
                 has since been moved"
            } else {
                "its source is here and it has no metadata card. That is an ordinary \
                 state and not a damaged store: a card is derived rather than kept, so \
                 an artifact stored as bytes, or stored by a build older than cards, \
                 has none until something compiles it and stores it again. What it \
                 declares is in its source, at the top of the procedure"
            };
            // No card, so there is no declared name to weigh: the set's own
            // name if it has one, and the short hash otherwise.
            let called = karakuri_environment::setfile::node_called(name, None, hash);
            format!("{}\n  {standing}.\n", head(&called))
        }
        // A card that is there and will not read is the one case that *is* a
        // damaged store, and it says so in different words for that reason.
        Err(e) => {
            let called = karakuri_environment::setfile::node_called(name, None, hash);
            format!("{}\n  its card could not be read: {e}\n", head(&called))
        }
    }
}

/// A card's four records as prose: what the procedure calls itself, and the
/// lines describing what it declares.
///
/// **Only the four a card can carry today.** `origin`, `parent`, `perf`, `tag`
/// and `thumbnail` are specified and nothing writes one — see
/// [`crate::meta::card`], which says why each is absent rather than empty — so
/// they fall through the catch-all, which is also what makes this reader survive
/// meeting a card written by a build that has more of them.
pub(crate) fn rendered_card(card: &[Line]) -> (Option<String>, String) {
    let mut declared = None;
    let mut body = String::new();
    let mut params = 0usize;
    for line in card {
        match line.record() {
            Record::Meta { name, .. } => declared = Some(name.clone()),
            Record::ParamDecl {
                key,
                ty,
                min,
                max,
                default,
            } => {
                params += 1;
                body.push_str(&format!(
                    "  param {key} : {ty}, anywhere from {min} to {max}{}\n",
                    match default {
                        Some(default) => format!(", and {default} until something turns it"),
                        // The record's own reading, in words: an absent
                        // `default` says the default is not a number this build
                        // can state, never that there is none — every declared
                        // param has one, because the `.kir` grammar makes the
                        // expression mandatory.
                        None => ". Its default is an expression rather than a literal, so the \
                             card cannot state it as a number"
                            .to_string(),
                    }
                ));
            }
            Record::CapacityDecl { min, max, default } => body.push_str(&format!(
                "  capacity: between {min} and {max} elements, and {default} of them \
                 until a set says otherwise\n"
            )),
            Record::Emit { attrs } => body.push_str(&format!(
                "  emits {} — what a renderer drawn over it can consume\n",
                attrs.join(", ")
            )),
            _ => {}
        }
    }
    // Said rather than left to silence: a block with no `param` line reads as a
    // rendering that dropped them. Every other absence here is a whole record
    // the card deliberately does not write — see [`crate::meta::card`] — and
    // reads correctly as nothing, but "there is nothing to turn on this one" is
    // an answer to the question that was asked.
    if params == 0 {
        body.push_str("  no parameters: there is nothing to turn on this one\n");
    }
    (declared, body)
}

/// A record [`Layer`] under the name this protocol already spells it with.
///
/// **Found through [`crate::setfile::layer_of`] rather than matched again.**
/// The mapping between a record's `Layer` and the compiler's `Kind` exists once,
/// is total, and is the one `--load-set` reads a Set through; a second match
/// here would be a second answer to which layer a stored node is on, and the
/// name a model is given for a node has to be the name it addresses one by. The
/// fallback cannot be reached while that mapping stays total — and it renders as
/// a word rather than panicking, because a layer added on one side only is a
/// thing to see in an answer, not a thread to take down.
pub(crate) fn layer_spelled(layer: Layer) -> &'static str {
    LAYERS
        .iter()
        .copied()
        .find(|kind| karakuri_environment::setfile::layer_of(*kind) == layer)
        .map_or("unknown", layer_name)
}
