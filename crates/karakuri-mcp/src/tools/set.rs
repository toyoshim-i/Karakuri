use std::sync::mpsc;

use karakuri_ir::{Diagnostic, DiagnosticReport};
use karakuri_operation::Operation;
use karakuri_store::hash::Hash;
use karakuri_store::record::{Layer, Record};
use karakuri_store::store::{Store, StoreError};
use serde_json::Value;

use super::super::*;
use super::*;

/// Verify a set configuration in the store and return a `DiagnosticReport`.
pub fn check_set_configuration(store_path: &std::path::Path, id: &str) -> DiagnosticReport {
    let store = match Store::open(store_path) {
        Ok(s) => s,
        Err(e) => {
            return DiagnosticReport {
                diagnostics: vec![Diagnostic {
                    code: "KIR-E501-STORE-OPEN-FAILED".to_string(),
                    message: format!("cannot open store at `{}`: {e}", store_path.display()),
                    line: None,
                    column: None,
                    remedy: Some(
                        "Verify the store directory path exists and has correct permissions."
                            .to_string(),
                    ),
                }],
                success: false,
            };
        }
    };
    match store.read_set(id) {
        Ok(_) => DiagnosticReport::ok(),
        Err(e) => DiagnosticReport {
            diagnostics: vec![Diagnostic {
                code: "KIR-E502-SET-READ-FAILED".to_string(),
                message: format!("failed to read set `{id}`: {e}"),
                line: None,
                column: None,
                remedy: Some(
                    "Check that the set ID is correctly spelled and saved in the library."
                        .to_string(),
                ),
            }],
            success: false,
        },
    }
}

/// `save_set`'s arguments as the operation they name.
///
/// **`slot` is required and `id` is not**, which is [`slot_layer_index`]'s
/// convention and its reason: an argument a client says nothing about should
/// mean the obvious thing, and the obvious thing here is the name a key press
/// gets. Unlike `index` there is no default written down — the loop stamps it,
/// and stamping it here would be a second answer to what a nameless save is
/// called, which is why [`Operation::SaveSet`]'s `id` is an `Option` as well.
///
/// The slot is checked before the id is read, because that is the order this
/// tool refused in before it routed. See [`Slots::holds`] on why this is not
/// [`Slots::nodes`].
pub(crate) fn kept(args: &Value, slots: &Slots) -> Result<Operation, String> {
    let slot = args
        .get("slot")
        .and_then(Value::as_u64)
        .ok_or("`slot` is required and is a number")? as usize;
    let deck = deck_named(slot, slots)?;
    let id = match args.get("id") {
        // **`null` is absent, not a bad string.** A client that builds its
        // arguments from a record with an empty field sends `"id": null`, and
        // that is a caller saying nothing about the id rather than one getting
        // its type wrong — "`id` is a string" is a refusal about a mistake it
        // did not make. Every other optional argument here reads an absent one
        // as its default; `null` is the second spelling of absent and gets the
        // same answer. It is *not* the same as `""`, which is a caller naming a
        // file with no name and is still refused — see [`checked_id`].
        None | Some(Value::Null) => None,
        Some(id) => Some(checked_id(
            id.as_str()
                .ok_or("`id` is a string: what to file the set under")?,
        )?),
    };
    Ok(Operation::SaveSet { deck, id })
}

/// `read_set`'s argument as the operation it names.
///
/// **The same check `save_set` puts a name through, and the reason is the same
/// one.** This id becomes `<store>/sets/<id>.kbset`, so
/// `../../../somewhere/else` is a path, and paths never cross this protocol —
/// see [`checked_id`] and [`Slots`]. A read is not the harmless half of that
/// rule: it is the half that hands a file's contents back to the caller.
///
/// It runs here rather than in the tool because an id that is a path is not an
/// id, and an operation carries what it acts on.
pub(crate) fn named_set(args: &Value) -> Result<Operation, String> {
    let id = args
        .get("id")
        .and_then(Value::as_str)
        .ok_or("`id` is required and is a string: which set to read")?;
    Ok(Operation::ReadSet {
        id: checked_id(id)?,
    })
}

/// `list_sets`'s arguments as the operation they name.
///
/// **The caller's spelling of `holds` is carried, not a folded one.** The match
/// is case-insensitive and that is [`list_sets`]'s decision about matching; an
/// operation carries what it was asked for.
pub(crate) fn listing(args: &Value) -> Result<Operation, String> {
    let holds = match args.get("holds") {
        // `null` is absent, for the reason [`kept`]'s `id` says: a client
        // building arguments from a record with an empty field sends one, and
        // that is a caller saying nothing rather than a caller getting a type
        // wrong.
        None | Some(Value::Null) => None,
        Some(value) => Some(
            value
                .as_str()
                .ok_or("`holds` is a string: part of a node's name")?
                .to_string(),
        ),
    };
    let layer = match args.get("layer") {
        None | Some(Value::Null) => None,
        Some(value) => {
            let spelled = value
                .as_str()
                .ok_or("`layer` is a string: which layer a set must hold a node on")?;
            // **The same spellings the other tools take**, from the same table:
            // a model that addressed `Field` in `read_procedure` must not be
            // told there is no such layer here.
            let kind = layer_named(spelled)
                .ok_or_else(|| format!("no layer `{spelled}` — {}", layer_list()))?;
            Some(layer_of(kind))
        }
    };
    Ok(Operation::ListSets { holds, layer })
}

/// **[`Operation::SaveSet`], done**: ask the render loop to keep what a slot is
/// playing.
///
/// Nothing about the Set is read here and nothing could be: this thread does not
/// hold it. What this does is hand the request over and give the caller back the
/// half it waits on — everything decidable from the arguments alone was decided
/// in [`kept`].
///
/// **This is the one tool that reaches the render loop, and it is not a second
/// save path.** The request is taken where the MIDI surface is taken and ends in
/// `Live::save_set`, the method the `k` key ends in. The record is
/// `Record::Save` and it is written at the frame the save landed, which is why
/// `karakuri_operation_record::written` answers `Silent(OnLanding)` for this
/// operation rather than handing anybody a record to write here.
pub(crate) fn save_set(
    deck: u8,
    id: Option<&str>,
    state: &State,
) -> Result<mpsc::Receiver<News>, String> {
    let slot = usize::from(deck);
    let id = id.map(str::to_string);
    let (tx, rx) = mpsc::channel();
    state
        .asked
        .try_send(SaveRequest {
            slot,
            id,
            reply: Reply(tx),
        })
        // **Answered rather than waited for**, both ways. A model must never be
        // left holding a call on a loop that will not answer it, and these are
        // the two shapes of "it will not": one that has stopped taking requests,
        // and one that is gone.
        .map_err(|e| match e {
            // **What `Full` proves and no more.** It used to say the loop "has
            // taken none of them", which the error does not support: the queue
            // holds [`ASKED`] requests nobody has taken *yet*, and a loop
            // running slowly reaches that as surely as one that has stopped.
            // Naming the second as though it were the fact would send a model
            // looking for a dead render thread when the answer is to ask again.
            mpsc::TrySendError::Full(_) => format!(
                "the render loop has {ASKED} save requests queued and no room for another: \
                 it is taking them slower than they are arriving, or it is not running \
                 frames at all. Nothing was saved, and asking again is safe"
            ),
            mpsc::TrySendError::Disconnected(_) => {
                "the render loop has ended: this run is shutting down and nothing was saved"
                    .to_string()
            }
        })?;
    Ok(rx)
}

/// **What one saved Set holds, and what each of its procedures declares** —
/// off the store, with nothing loaded, nothing compiled and no GPU.
///
/// **This is the reader `<hash>.meta.ndjson` did not have.** Every path that
/// stores an artifact from a compile writes a card beside it — see
/// [`crate::meta::card`] — and until this, `Store::read_meta` had no caller
/// outside its own tests. A figure with a producer and no consumer is how the
/// last wrong number in this program got published, so the card gets its reader
/// in the same milestone that gave it a writer.
///
/// **A tool and not a resource.** The resource list is a curriculum, not an
/// index — see `docs/adr/0092-a-resource-listing-is-a-curriculum-not-an-index.md`.
/// A resource is a curated few a client reads in
/// full, and a user's Sets are neither curated nor few nor knowable at startup.
/// The two resources here are the spec and the vocabulary, which every client
/// should read once; a library is searched, and searching is a call.
///
/// **The caller names a Set, because a Set id is the only handle a model can
/// hold.** The three candidates were the address the other tools take
/// (`slot`, `layer`, `index`), a Set id, and a content hash:
///
/// - **A hash is what the card is filed under and it is the one to reject**,
///   easiest though it is. Nothing in this protocol has ever handed a model a
///   hash, so the first call could not be made — a tool whose argument only
///   this tool's own output can supply is a tool nobody can start using. It is
///   also the *only* one of the three that needs no validation, being hex and
///   64 characters, and choosing an argument for the convenience of its
///   validation is choosing the wrong argument.
/// - **`(slot, layer, index)` names what is on screen**, whose source a model
///   can already fetch with `read_procedure` and read the declarations off
///   directly. It would answer a question that is already answerable.
/// - **A Set id names material this surface is otherwise blind to.** A saved
///   Set that has not been loaded has no file behind it that `read_procedure`
///   can reach — its sources are bytes in the store under hashes nothing shows
///   — so *"which of these saved things should I use"* is unanswerable without
///   this. `save_set` comes back naming the id it wrote, and `--load-set ID`
///   is spelled with one, so a model that has kept anything has one in hand.
///
/// **What this does not say is what the Set has those knobs turned to.** The
/// file read here carries `param` and `capacity` records beside the `slot`s and
/// they are deliberately passed over: a `param_decl` says a knob exists and
/// what it may be turned between, a `param` says where this Set left it, and
/// the record vocabulary keeps them apart under two names for exactly that
/// reason. Rendering both in one block would be the place they get confused,
/// and *"what is it set to"* is a second question that deserves being asked as
/// one. The lines are in hand the moment anybody wants it.
///
/// **One number here is not a declaration**, and it is the last block: what the
/// Set will allocate to hold elements. See [`element_storage_block`] for why
/// that is computed rather than measured, and why it is the one figure in this
/// answer that is about the Set as a whole rather than about a procedure.
pub(crate) fn read_set(id: &str, state: &State) -> Result<String, String> {
    // Already one path component, because that is part of naming a set rather
    // than part of reading one — see [`asked`]'s `read_set` arm and
    // [`checked_id`].
    let store = Store::open(&state.store)
        .map_err(|e| format!("the store at `{}`: {e}", state.store.display()))?;
    let lines = store.read_set(id).map_err(|e| {
        format!(
            "reading set `{id}`: {e} — this reads the operator's library, which is \
             filed under the id a set was saved under by the `k` key or by \
             `--save-set ID`. A set kept with `save_set` is in `<store>/{}/` and not \
             here, because the library is written by the operator's own act",
            Store::SANDBOX
        )
    })?;
    // **The file's own order**, which is the order [`crate::setfile::save`]
    // wrote the nodes in, and the order a hand-written file chose. Sorting by
    // layer would impose a reading nobody wrote, for the reason
    // [`crate::meta::card`] keeps a procedure's parameters in declaration order.
    let nodes: Vec<(Layer, u32, Option<String>, Hash)> = lines
        .iter()
        .filter_map(|line| match line.record() {
            Record::Slot {
                at,
                name,
                proc_hash,
            } => Some((at.layer, at.index, name.clone(), *proc_hash)),
            _ => None,
        })
        .collect();
    if nodes.is_empty() {
        return Ok(format!(
            "set `{id}` is in this store and names no material: it holds {} record{} \
             and none of them is a `slot`, so there is nothing in it to describe.",
            lines.len(),
            if lines.len() == 1 { "" } else { "s" },
        ));
    }
    let mut out = format!(
        "set `{id}` holds {} node{}, and `--load-set {id}` plays it. Everything below \
         is what a procedure *declares* — the range a value is refused outside of — \
         and not what this set has anything turned to.\n",
        nodes.len(),
        if nodes.len() == 1 { "" } else { "s" },
    );
    for (layer, index, name, hash) in nodes {
        out.push('\n');
        out.push_str(&node_block(&store, layer, index, name.as_deref(), &hash));
    }
    out.push('\n');
    out.push_str(&element_storage_block(&store, id));
    Ok(out)
}

/// **How many Sets one answer renders, however many matched.**
///
/// A library is not bounded by anything: a run that presses `k` between takes
/// keeps a Set a minute, and a store two thousand deep is an ordinary end state
/// rather than a broken one. A protocol answer is read into a context window,
/// so the choice is between a fixed ceiling and an answer whose size is the
/// user's own filing habits — and twenty is about what a reader can weigh in
/// one go. What must never happen is the ceiling being reached silently, which
/// is why [`list_sets`] says the total and the shown count in the same
/// sentence.
pub const LISTED: usize = 20;

/// **What this store holds** — every Set saved into it, most recent first, with
/// what each one is made of.
///
/// **The listing `read_set` needed and did not have.** `read_set` takes an id
/// and its own description ends by telling a model to use it to choose between
/// things it has kept — which was unreachable, because nothing said what was
/// kept. A model could read a Set it had just saved, in the same conversation,
/// and nothing else; an operator had `ls` on a directory of `.kbset`. This
/// is the other half, and it is the half the milestone is named for.
///
/// **Most recent first, and the tie-break is why this sorts at all.**
/// `Store::list_sets` orders by id, which is total and repeatable and is the
/// right order for the store to promise; *what did I just save* is the question
/// this surface is mostly asked, so it sorts on the write time and breaks ties
/// by id. The tie-break is not decoration: two Sets written within one tick of
/// a coarse filesystem clock carry the same mtime, and a sort whose keys tie
/// falls back to whatever order the entries arrived in — which is not an order,
/// and would differ between two calls on an unchanged store. A model asking
/// twice must not be told two different things about a library nobody touched.
///
/// **What it does not say is what any of it declares.** That needs a card per
/// artifact and, for the element storage, a compile pass over the whole Set —
/// which is what `read_set` is for, on one Set a caller has chosen. A listing
/// that did it for a library would compile a thousand procedures to print a
/// thousand lines. The per-node cards this *does* read are only the ones a
/// name needs: a node the file named costs nothing to name here.
///
/// **The summary comes from [`crate::setfile::summarise`]**, which `--list-sets`
/// renders too. One derivation, two renderings — an operator's line and this —
/// so the two surfaces cannot come to disagree about what a store holds or
/// about what a node in it is called.
pub(crate) fn list_sets(
    holds: Option<&str>,
    layer: Option<karakuri_operation::Layer>,
    state: &State,
) -> Result<String, String> {
    // **Lowercased once here rather than per node.** Case-insensitive because a
    // model that read `drift_shell` in one answer and types `Drift_Shell` into
    // the next is not asking a different question. Folded here rather than in
    // [`listing`], because it is a decision about *matching* and an operation
    // carries what it was asked for.
    let holds = holds.map(str::to_ascii_lowercase);
    let layer = layer.map(karakuri_environment::meta::op_to_record);
    let opened = |e: StoreError| format!("the store at `{}`: {e}", state.store.display());
    let store = Store::open(&state.store).map_err(opened)?;
    let mut sets = karakuri_environment::setfile::summarise(&store).map_err(opened)?;
    let held = sets.len();
    // **An empty store is an answer and not a failure**, and it is a different
    // answer from a filter that matched nothing: one sends a reader to
    // `save_set`, the other to a different filter. Answered before the filters
    // are applied, because a filter over nothing has nothing to say.
    if held == 0 {
        return Ok(format!(
            "this store holds no sets at all — nothing has been kept here yet. This lists \
             the operator's library, which is written by their own act: the `k` key, or \
             `--save-set ID` on the command line. What `save_set` keeps goes to the \
             sandbox and is not listed here. This store is `{}`.",
            state.store.display()
        ));
    }
    sets.sort_by(|a, b| b.written.cmp(&a.written).then_with(|| a.id.cmp(&b.id)));
    // **A set matches, not a node.** With both filters given the question is
    // "which of the sets that use this also deform something", so each is
    // answered against the whole set rather than against one node — a set whose
    // `drift_shell` is a geometry and whose deformation is called something
    // else is exactly what that question is looking for.
    sets.retain(|set| {
        holds.as_ref().is_none_or(|holds| {
            set.nodes
                .iter()
                .any(|node| node.name.to_ascii_lowercase().contains(holds))
        }) && layer.is_none_or(|layer| set.nodes.iter().any(|node| node.layer == layer))
    });
    let narrowed = describe_filters(holds.as_deref(), layer);
    let matched = sets.len();
    if matched == 0 {
        return Ok(format!(
            "none of the {held} set{} {narrowed}. The store is not empty — \
             call this with no arguments to see everything in it. `holds` is matched \
             against what each node is called, which is the name the set gave it or the \
             name its procedure gives itself.",
            plural(held),
        ));
    }
    let shown = matched.min(LISTED);
    let mut out = if matched > shown {
        // **Never a truncated list that reads as a whole one.** A model told
        // "here are your sets" over twenty of two hundred will tell its user
        // they have twenty, and act on a library it has not seen.
        format!(
            "{matched} set{} {narrowed}, and the {shown} most recently written are below — \
             **this is not all of them**: {} more matched and are not listed. Narrow it \
             with `holds`, or with `layer`, or ask for a set by id with `read_set`.\n",
            plural(matched),
            matched - shown,
        )
    } else {
        format!(
            "{matched} set{} {narrowed}, most recently written first — all of them are \
             below.\n",
            plural(matched),
        )
    };
    for set in sets.iter().take(shown) {
        out.push_str(&set_line(set));
    }
    out.push_str(
        "\nEach line is a set's id, when it was written, and what it holds: an address \
         per node and what that node is called in this set. `read_set` with one of these \
         ids says what each of its procedures declares — the parameters, the element \
         counts and what it emits — and `--load-set ID` is what plays one.\n",
    );
    Ok(out)
}

/// One Set as a line of a listing.
fn set_line(set: &karakuri_environment::setfile::SetSummary) -> String {
    let written = karakuri_environment::setfile::written_at(set.written);
    // **A file in `sets/` that will not read is listed and named.** Dropping it
    // would answer "what have I kept" with something missing, and rendering it
    // as a set of no nodes would say it holds nothing.
    if let Some(why) = &set.unreadable {
        return format!(
            "`{}` — written {written}, and could not be read: {why}\n",
            set.id
        );
    }
    if set.nodes.is_empty() {
        return format!(
            "`{}` — written {written}, and names no material: it holds no `slot` record\n",
            set.id
        );
    }
    let nodes: Vec<String> = set
        .nodes
        .iter()
        .map(|node| {
            format!(
                "{}:{} `{}`",
                layer_spelled(node.layer),
                node.index,
                node.name
            )
        })
        .collect();
    format!(
        "`{}` — written {written}, {} node{}: {}\n",
        set.id,
        set.nodes.len(),
        plural(set.nodes.len()),
        nodes.join(", "),
    )
}

/// What the filters did to a listing, as the middle of a sentence — so that
/// every count this tool prints is said to be a count *of* something, and a
/// filtered answer can never be read as the whole store.
fn describe_filters(holds: Option<&str>, layer: Option<Layer>) -> String {
    match (holds, layer) {
        (None, None) => "in this store".to_string(),
        (Some(holds), None) => format!("in this store hold a node whose name contains `{holds}`"),
        (None, Some(layer)) => format!("in this store hold a {} node", layer_spelled(layer)),
        (Some(holds), Some(layer)) => format!(
            "in this store hold both a node whose name contains `{holds}` and a {} node",
            layer_spelled(layer)
        ),
    }
}
