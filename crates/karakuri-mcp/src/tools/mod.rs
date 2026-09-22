pub mod dispatch;
pub mod history;
pub mod operate;
pub mod procedure;
pub mod schema;
pub mod set;
pub mod wire;

use std::sync::mpsc;

use karakuri_ir::Kind;
use karakuri_operation::gate::{self, Allowed};
use karakuri_operation::{NodeAddress, Operation, RefusalDetail};
use serde_json::{json, Value};

pub use operate::OperateRequest;
pub use procedure::check_procedure;
pub use set::{check_set_configuration, LISTED};

pub(crate) use dispatch::{perform, sayable, Sayable};
pub(crate) use history::*;
pub(crate) use operate::*;
pub(crate) use procedure::*;
pub(crate) use schema::tools;
pub(crate) use set::*;
pub(crate) use wire::*;

use crate::{
    layer_list, layer_name, layer_named, layer_of, operated, DiagnosticReport, News, Slots, State,
    MAX_ID,
};

/// What one tool call came to: an answer, or a wait that belongs outside the
/// state lock. See [`Pending`].
pub(crate) enum Called {
    Answered(Result<String, String>),
    Saving(mpsc::Receiver<News>),
    /// An edge the loop has been asked for, and what this server has to add to
    /// whatever it answers — see [`Pending::Wiring`].
    Wiring {
        news: mpsc::Receiver<News>,
        note: String,
    },
    /// An operation the loop has been asked to perform — see [`OperateRequest`]. No
    /// note beside it: what this server knows about the run that the loop will not
    /// say is the class the audit refused on, and a refusal never reaches here.
    Operating(mpsc::Receiver<News>),
}

/// What one tool call names, in the vocabulary — or the refusal its arguments
/// earned.
///
/// A tool call is a request from outside the process naming a thing to do,
/// which is a MIDI message's shape rather than a key press's, and
/// [ADR-0196](../../../docs/adr/0196-a-map-line-names-a-state-and-an-old-line-is-refused.md)
/// is what that surface did with it: message becomes `Operation`, and something
/// else performs it. The second half of that does not exist here and cannot.
/// `karakuri_operation_record::written` answers `Silent` for all six of these —
/// `Question` for the four that ask and `OnLanding` for the two whose record is
/// written where the work lands — so `Live::operate` would print *no record*
/// and do nothing, which is
/// [ADR-0198](../../../docs/adr/0198-a-gesture-converts-in-the-parts-that-are-decided.md)'s
/// finding about twelve keys, holding here for all six tools. There is also no
/// `Live` on this thread to route into: this server reaches the render loop for
/// exactly one thing, over the channel [`SaveRequest`] travels on.
///
/// So what routes is the naming. The wire's own words — a tool name and a JSON
/// object — become the operation the manual specifies, once, here; and
/// [`perform`] dispatches on that operation rather than on the string. A tool
/// whose payload the vocabulary cannot say does not compile, and the row on
/// `docs/manual/operations.html` that a tool claims is the row its operation's
/// title names rather than one a second list asserts.
///
/// The order arguments are refused in is the order they were refused in before
/// this routed, deliberately: a change of route may not change what a tool
/// answers, and the refusals here are the surface's product — a model that is
/// told which mistake it made fixes its own call. So the slot is checked where
/// each tool checked it, `checked_id` runs where each tool ran it, and nothing
/// new is decided in front of anything old.
pub(crate) fn asked(name: &str, args: &Value, slots: &Slots) -> Result<Asked, String> {
    Ok(match name {
        "read_procedure" => match address(args, slots) {
            Ok((deck, node)) => Asked::Named(Operation::ReadProcedure { deck, node }),
            Err(refusal) => Asked::Refused(refusal),
        },
        "write_procedure" => match written_procedure(args, slots) {
            Ok(operation) => Asked::Named(operation),
            Err(refusal) => Asked::Refused(refusal),
        },
        "wire_input" => match wired_input(args, slots) {
            Ok(operation) => Asked::Named(operation),
            Err(refusal) => Asked::Refused(refusal),
        },
        "swap_outcome" => Asked::Named(Operation::SwapOutcome),
        "read_set" => match named_set(args) {
            Ok(operation) => Asked::Named(operation),
            Err(refusal) => Asked::Refused(refusal),
        },
        "list_sets" => match listing(args) {
            Ok(operation) => Asked::Named(operation),
            Err(refusal) => Asked::Refused(refusal),
        },
        // **The store's other listing**, beside `list_sets` for its reason: the
        // walk is a directory read this thread can do and the render loop
        // cannot afford, and its answer is rows rather than a report that
        // something was performed (`docs/adr/0342-…`).
        "walk_history" => match walked_history(args) {
            Ok(operation) => Asked::Named(operation),
            Err(refusal) => Asked::Refused(refusal),
        },
        "save_set" => match kept(args, slots) {
            Ok(operation) => Asked::Named(operation),
            Err(refusal) => Asked::Refused(refusal),
        },
        // The `operate` tool dispatches arbitrary operations by name via [`operated`] and [`SPELLED`].
        "operate" => match operated(args, slots) {
            Ok(operation) => Asked::Named(operation),
            Err(refusal) => Asked::Refused(refusal),
        },
        other => return Err(format!("no tool `{other}`")),
    })
}

/// What [`asked`] made of one call: the operation, or what the caller is told
/// instead.
///
/// The refusal is a tool result and not a protocol error, which is why it is
/// carried in the `Ok` half rather than returned — see [`tool_result`]. The
/// `Err` of [`asked`] is the one thing that really is a protocol mistake: a
/// tool this server does not publish.
pub(crate) enum Asked {
    Named(Operation),
    Refused(String),
}

/// The deck one slot number names.
///
/// `Operation` carries `deck: u8`, and every `slot` in
/// `karakuri_store::record::Record` is a `u8` too, so a slot past 255 is not a
/// deck anything in this program can address. Refused in the words
/// [`Slots::nodes`] and [`Slots::holds`] refuse an absent slot in, because it
/// is the same mistake and an operator is told one story about it
/// ([`crate::no_such_slot`]).
///
/// Called after every argument the tool used to parse before it reached the
/// slot, so that a call with two mistakes in it is still told about the same
/// one it was told about before.
pub(crate) fn deck_named(slot: usize, slots: &Slots) -> Result<u8, String> {
    slots
        .holds(slot)
        .map_err(|e| refusal_payload(&RefusalDetail::slot_unallocated(slot, e)))?;
    u8::try_from(slot).map_err(|_| {
        let msg = karakuri_environment::no_such_slot(slot, slots.count());
        refusal_payload(&RefusalDetail::slot_unallocated(slot, msg))
    })
}

/// `read_procedure`'s arguments as the deck and node they name.
pub(crate) fn address(args: &Value, slots: &Slots) -> Result<(u8, NodeAddress), String> {
    let (slot, layer, index) = slot_layer_index(args)?;
    let deck = deck_named(slot, slots)?;
    Ok((
        deck,
        NodeAddress {
            layer: layer_of(layer),
            index: index as u32,
        },
    ))
}

pub(crate) fn call_tool(request: &Value, state: &mut State) -> Result<Called, String> {
    let params = request.get("params").ok_or("no params")?;
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or("no tool name")?;
    let args = params.get("arguments").cloned().unwrap_or(json!({}));

    if name == "check_procedure" {
        let source = args
            .get("source")
            .and_then(Value::as_str)
            .ok_or("`source` is required")?;
        let report = check_procedure(source);
        let report_json = serde_json::to_string_pretty(&report).unwrap_or_default();
        return Ok(Called::Answered(if report.success {
            Ok(report_json)
        } else {
            Err(report_json)
        }));
    }
    if name == "check_set" {
        let id = args
            .get("id")
            .and_then(Value::as_str)
            .ok_or("`id` is required")?;
        let report = check_set_configuration(&state.store, id);
        let report_json = serde_json::to_string_pretty(&report).unwrap_or_default();
        return Ok(Called::Answered(if report.success {
            Ok(report_json)
        } else {
            Err(report_json)
        }));
    }
    if name == "get_permissions" {
        let opening = state.opening.read();
        let slot_accesses = state.slot_policies.all();
        let slot_count = state.slots.count();
        let slots: Vec<Value> = (0..slot_count)
            .map(|slot| {
                let access = slot_accesses.get(slot).copied().unwrap_or_default();
                let letter = (b'A' + slot as u8) as char;
                json!({
                    "slot": slot,
                    "name": format!("Deck {letter}"),
                    "policy": access.policy.name(),
                    "in_mix": access.in_mix,
                    "writable": access.is_writable(),
                    "reason": access.refusal_reason(slot),
                })
            })
            .collect();
        let resp = json!({
            "bays": {
                "program": if opening.holds(karakuri_operation::gate::Class::LiveDeck) { "on" } else { "off" },
                "mixer": if opening.holds(karakuri_operation::gate::Class::MixFaders) { "on" } else { "off" },
                "master": if opening.holds(karakuri_operation::gate::Class::MasterEffects) { "on" } else { "off" },
                "outputs": if opening.holds(karakuri_operation::gate::Class::InputsAndOutputs) { "on" } else { "off" },
            },
            "slots": slots,
        });
        return Ok(Called::Answered(Ok(
            serde_json::to_string_pretty(&resp).unwrap_or_default()
        )));
    }
    if name == "read_slot" {
        let slot = match args
            .get("slot")
            .and_then(Value::as_u64)
            .ok_or("`slot` is required and is a number")
        {
            Ok(s) => s as usize,
            Err(e) => return Ok(Called::Answered(Err(e.into()))),
        };
        if let Err(e) = state.slots.holds(slot) {
            let detail = RefusalDetail::slot_unallocated(slot, e);
            return Ok(Called::Answered(Err(refusal_payload(&detail))));
        }
        let nodes = match state.slots.nodes(slot) {
            Ok(n) => n,
            Err(e) => return Ok(Called::Answered(Err(e))),
        };
        let mut node_entries = Vec::new();
        for (kind, index, path) in nodes {
            let source = match std::fs::read_to_string(&path) {
                Ok(s) => s,
                Err(e) => return Ok(Called::Answered(Err(format!("{}: {e}", path.display())))),
            };
            node_entries.push(json!({
                "layer": layer_name(kind),
                "index": index,
                "source": source,
            }));
        }
        let resp = json!({
            "slot": slot,
            "nodes": node_entries,
        });
        return Ok(Called::Answered(Ok(
            serde_json::to_string_pretty(&resp).unwrap_or_default()
        )));
    }
    if name == "copy_slot" {
        let from_slot = match args
            .get("from_slot")
            .and_then(Value::as_u64)
            .ok_or("`from_slot` is required and is a number")
        {
            Ok(s) => s as usize,
            Err(e) => return Ok(Called::Answered(Err(e.into()))),
        };
        let to_slot = match args
            .get("to_slot")
            .and_then(Value::as_u64)
            .ok_or("`to_slot` is required and is a number")
        {
            Ok(s) => s as usize,
            Err(e) => return Ok(Called::Answered(Err(e.into()))),
        };
        if let Err(e) = state.slots.holds(from_slot) {
            let detail = RefusalDetail::slot_unallocated(from_slot, e);
            return Ok(Called::Answered(Err(refusal_payload(&detail))));
        }
        if let Err(e) = state.slots.holds(to_slot) {
            let detail = RefusalDetail::slot_unallocated(to_slot, e);
            return Ok(Called::Answered(Err(refusal_payload(&detail))));
        }
        if let Err(d) = state.slot_policies.check_writable_detail(to_slot) {
            return Ok(Called::Answered(Err(refusal_payload(&d))));
        }
        let layer_filter = args.get("layer").and_then(Value::as_str);
        if let Some(filter_name) = layer_filter {
            if layer_named(filter_name).is_none() {
                return Ok(Called::Answered(Err(format!(
                    "`{filter_name}` is not a known layer"
                ))));
            }
        }

        let from_nodes = match state.slots.nodes(from_slot) {
            Ok(n) => n,
            Err(e) => return Ok(Called::Answered(Err(e))),
        };
        let to_nodes = match state.slots.nodes(to_slot) {
            Ok(n) => n,
            Err(e) => return Ok(Called::Answered(Err(e))),
        };

        // Stage files in temporary files before atomic move, preventing watchers
        // from catching intermediate partial writes or mismatched multi-node compiles.
        let mut staging = Vec::new();
        for (kind, index, from_path) in &from_nodes {
            if let Some(filter_name) = layer_filter {
                if layer_name(*kind) != filter_name {
                    continue;
                }
            }
            if let Some((_, _, to_path)) = to_nodes.iter().find(|(k, i, _)| k == kind && i == index)
            {
                let source = match std::fs::read(from_path) {
                    Ok(s) => s,
                    Err(e) => {
                        for (tmp, _) in &staging {
                            let _ = std::fs::remove_file(tmp);
                        }
                        return Ok(Called::Answered(Err(format!(
                            "{}: {e}",
                            from_path.display()
                        ))));
                    }
                };
                let mut tmp_name = to_path.file_name().unwrap_or_default().to_os_string();
                tmp_name.push(".tmp");
                let tmp_path = to_path.with_file_name(tmp_name);
                if let Err(e) = std::fs::write(&tmp_path, &source) {
                    for (tmp, _) in &staging {
                        let _ = std::fs::remove_file(tmp);
                    }
                    let _ = std::fs::remove_file(&tmp_path);
                    return Ok(Called::Answered(Err(format!(
                        "{}: {e}",
                        tmp_path.display()
                    ))));
                }
                staging.push((tmp_path, to_path.clone()));
            }
        }

        let mut copied = 0;
        for (tmp_path, to_path) in staging {
            if let Err(e) = std::fs::rename(&tmp_path, &to_path) {
                return Ok(Called::Answered(Err(format!("{}: {e}", to_path.display()))));
            }
            copied += 1;
        }

        return Ok(Called::Answered(Ok(format!(
            "copied {copied} procedure{} from slot {from_slot} to slot {to_slot}",
            if copied == 1 { "" } else { "s" }
        ))));
    }

    Ok(match asked(name, &args, &state.slots)? {
        // **The gate, and there is one of it.** Named, then audited, then done
        // — every tool crosses this seam because [`perform`] takes what
        // [`audited`] returns and nothing else can make one.
        Asked::Named(operation) => match audited(&operation, state) {
            Ok(allowed) => perform(&allowed, state),
            Err(detail) => Called::Answered(Err(refusal_payload(&detail))),
        },
        Asked::Refused(refusal) => Called::Answered(Err(refusal)),
    })
}

/// This surface's one call into the audit.
///
/// The classification, the four classes and the refusal sentence are
/// `karakuri_operation::gate`'s and not this module's, which is
/// [ADR-0236](../../../docs/adr/0236-a-map-is-the-layer-between-a-surface-and-the-vocabulary-and-the-audit-is-one-of-the-things-it-does.md)
/// refusing to let one surface hold the rule: *"a rule held by one surface
/// binds one surface"*, and a sequencer lane is already decided as a fifth
/// route that would otherwise arrive with a second copy of the table. What is
/// this module's is the two things only it can supply — the opening the run was
/// handed and what it has read of what is running.
///
/// And it has read nothing, which is said rather than defaulted.
/// `Running::unread()` is honest: this server holds `Slots`, a store root and a
/// watch flag, and no residency at all. Exactly one row turns on that reading —
/// `Operation::LoadSet`, whose class is *a deck in live mode* — and this
/// surface publishes no tool that names it, so nothing is refused today that
/// was not refused yesterday. The day a `load_set` tool lands it is refused
/// with *which decks are live was not read* until somebody wires the reading,
/// which is
/// `docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md`'s
/// answer rather than a guess that the deck is idle.
pub(crate) fn audited<'a>(
    operation: &'a Operation,
    state: &State,
) -> Result<Allowed<'a>, RefusalDetail> {
    gate::audit_detail(operation, state.opening.read(), gate::Running::unread())
}

/// Serialise a [`RefusalDetail`] into a JSON string for structured envelope reporting.
pub(crate) fn refusal_payload(detail: &RefusalDetail) -> String {
    json!({
        "code": detail.code.as_str(),
        "message": detail.message,
        "slot": detail.slot,
        "policy": detail.policy.map(|p| p.name()),
        "in_mix": detail.in_mix,
    })
    .to_string()
}

/// One tool call's answer, in the shape the protocol gives a tool.
///
/// A tool failure is a result, not a protocol error. A model that is told "the
/// call was malformed" learns nothing; one handed the checker's diagnostics can
/// fix its own source, which is the whole loop.
///
/// A function rather than a `json!` at each call site, because `save_set`'s
/// answer is built after the lock is gone — see [`Pending`] — and two spellings
/// of this shape would be two chances to disagree about `isError`.
pub(crate) fn tool_result(outcome: Result<String, String>) -> Value {
    match outcome {
        Ok(text) => {
            if let Ok(report) = serde_json::from_str::<DiagnosticReport>(&text) {
                json!({ "content": [{ "type": "text", "text": text }], "isError": false, "report": report })
            } else {
                json!({ "content": [{ "type": "text", "text": text }], "isError": false })
            }
        }
        Err(text) => {
            if let Ok(report) = serde_json::from_str::<DiagnosticReport>(&text) {
                json!({ "content": [{ "type": "text", "text": text }], "isError": true, "report": report })
            } else if let Ok(val) = serde_json::from_str::<Value>(&text) {
                if let Some(refusal) = val.get("refusal") {
                    let msg = refusal
                        .get("message")
                        .and_then(Value::as_str)
                        .unwrap_or(&text);
                    json!({
                        "content": [{ "type": "text", "text": msg }],
                        "isError": true,
                        "refusal": refusal
                    })
                } else if val.get("code").is_some() && val.get("message").is_some() {
                    let msg = val.get("message").and_then(Value::as_str).unwrap_or(&text);
                    json!({
                        "content": [{ "type": "text", "text": msg }],
                        "isError": true,
                        "refusal": val
                    })
                } else {
                    json!({ "content": [{ "type": "text", "text": text }], "isError": true })
                }
            } else {
                json!({ "content": [{ "type": "text", "text": text }], "isError": true })
            }
        }
    }
}

/// `index` is optional and defaults to 0, unlike the wildcard an absent `index`
/// means on a `param` record. The difference is the same one that runs through
/// the whole address: this names *a procedure to read or rewrite*, and there is
/// no such thing as rewriting every renderer at once with one source — where a
/// `param` addresses a *value*, and one value reaching every declaration is
/// both meaningful and the useful default.
///
/// It defaults because the first node of a layer is what a client that says
/// nothing means, on every layer: a slot holding one camera and one shape has
/// nothing else `index` could name, and one holding two has an order its files
/// were given in.
///
/// The layer is parsed here into the compiler's own `Kind` and travels as one
/// from here on, so the layer this resolves a file for and the layer a written
/// source is checked against are the same value rather than two readings of one
/// string.
pub(crate) fn slot_layer_index(args: &Value) -> Result<(usize, Kind, usize), String> {
    let slot = args
        .get("slot")
        .and_then(Value::as_u64)
        .ok_or("`slot` is required and is a number")? as usize;
    let named = args
        .get("layer")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("`layer` is required and is one of {}", layer_list()))?;
    let layer = layer_named(named)
        .ok_or_else(|| format!("no layer `{named}`: a slot's nodes are {}", layer_list()))?;
    let index = match args.get("index") {
        None => 0,
        Some(v) => v
            .as_u64()
            .ok_or("`index` is a number: which node of that layer, from 0")?
            as usize,
    };
    Ok((slot, layer, index))
}

/// A Set id a client may name, or why not.
///
/// A Set id is one path component. [`crate::history::stamped_id`] says so where
/// it explains why the date is spelled `20260816` rather than `2026/08/16`, and
/// the store spells the file `<dir>/<id>.kbset` without checking that what it
/// was handed is one. That is the operator's own business on `--save-set`,
/// where the id came out of their own shell. It is not a model's: this is the
/// same rule [`Slots`] exists for — paths never cross the protocol — and
/// `../../../somewhere/else` is a path.
///
/// Letters, digits, `-` and `_`, which is what a stamp is made of and what a
/// name anybody would type is made of. Refused rather than sanitised: a set
/// filed under a name its caller did not ask for is a worse answer than one
/// that is told to pick another.
///
/// A name a client picks twice no longer overwrites, and the reason it once did
/// no longer holds. This paragraph said the opposite until 2026-09-05, and the
/// argument it made was sound on its own premise: a name a caller typed is an
/// instruction, `--save-set ID` has always obeyed it by overwriting, and
/// `save_set` did what `--save-set` did. What changed is where a model's save
/// lands. It writes `<store>/sandbox/` rather than the operator's library
/// ([P-0096](../../../docs/principles/0096-the-operators-library-is-written-by-an-operators-own-act.md)),
/// and nothing in that directory is an id an operator typed — it is a session's
/// edit history, and a snapshot a later snapshot can replace is not one. So
/// `crate::filed_as` puts the stamp in front of whatever name a client chose
/// and the accept names what was written; the renaming this paragraph refused
/// is still refused here, because refusing a bad id and naming a good file are
/// two different jobs and this one is still the first. What remains true
/// unchanged is that the behaviour is documented — in the tool description a
/// model reads and in `docs/manual.md`. Undocumented was the thing that was not
/// allowed.
///
/// No `con`, `nul`, `aux`, `com1` check. They are reserved device names on
/// Windows and would be a file that is not a file. There is no Windows target
/// today and no `cfg` for one here; this sentence is the record that the case
/// is known, so that whoever ports this finds it written down rather than finds
/// it on a projector.
///
/// Public since 2026-09-08, because it stopped being the model's wall alone:
/// the Inspector pane head's name is an operator typing an id, which
/// `crate::filed_as` passes straight through for `Asked::Operator` (ADR-0292).
/// One wall, so a `/` cannot become a path on either route.
pub fn checked_id(id: &str) -> Result<String, String> {
    if id.is_empty() {
        return Err(
            "`id` is empty: a set is filed under a name, or under none at all if \
                    `id` is left out"
                .into(),
        );
    }
    // **Bytes, and the message says bytes.** `str::len` is bytes and this said
    // "characters", which is the same number for everything that gets past the
    // charset check below and a different one for what does not — so the one
    // caller the message existed for, the one sending something this refuses,
    // was told a number it could not count to.
    if id.len() > MAX_ID {
        return Err(format!(
            "`id` is {} bytes and the most is {MAX_ID}: it becomes a file name",
            id.len()
        ));
    }
    if let Some(bad) = id
        .chars()
        .find(|c| !c.is_ascii_alphanumeric() && *c != '-' && *c != '_')
    {
        return Err(format!(
            "`id` holds `{bad}`, and a set id is letters, digits, `-` and `_`: it is one \
             path component and it names a file in the store"
        ));
    }
    Ok(id.to_string())
}

/// Wait for one save's outcome, and say something true when it does not come.
///
/// A free function over the channel rather than a loop inside [`Pending`], for
/// the reason `main.rs`'s `drained_saves` is one: the bound is the whole of
/// what makes waiting here safe, and it has to be checkable without a render
/// loop, a window or a disk.
///
/// It waits, rather than returning on acceptance, and that was the decision
/// worth arguing. Answering the moment the loop has the request would make this
/// tool cheap and its answer worthless: a model told "saved" before the disk
/// has spoken will tell its user the set is kept, and the cases where that is a
/// lie — a full store, a network mount that stopped answering, a slot whose
/// sources are not savable — are precisely the ones anybody would want to hear
/// about. The other three tools already work this way: `write_procedure` hands
/// back the checker's verdict and not "it is being checked".
///
/// A timeout is neither success nor failure, and the text says so. The protocol
/// has one boolean and it cannot carry a third state, so `isError` is set — a
/// model reading `isError: false` reports the set as kept, which is the one
/// thing that must not happen here, while a model reading `true` looks again.
/// What the flag cannot carry, the sentence does.
pub(crate) fn awaited(
    news: &mpsc::Receiver<News>,
    wait: std::time::Duration,
) -> Result<String, String> {
    let deadline = std::time::Instant::now() + wait;
    let mut accepted: Option<String> = None;
    while let Some(left) = deadline.checked_duration_since(std::time::Instant::now()) {
        match news.recv_timeout(left) {
            Ok(News::Accepted(said)) => accepted = Some(said),
            Ok(News::Settled(outcome)) => return outcome,
            Err(mpsc::RecvTimeoutError::Timeout) => break,
            // **The loop dropped the request without answering it**, which is
            // what the end of a run looks like from here. Which of the two
            // sentences depends on whether it was ever taken: one that was
            // never taken saved nothing, and one that was may well have reached
            // the disk on the way out.
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err(match accepted {
                    Some(said) => format!(
                        "{said}\n\nThe render loop then ended without saying what became of \
                         it. Whether that file was written is not something this server can \
                         still find out."
                    ),
                    None => "the render loop ended before it took this save: nothing was saved"
                        .to_string(),
                })
            }
        }
    }
    Err(match accepted {
        Some(said) => format!(
            "{said}\n\n**This is neither a success nor a failure.** The save was accepted \
             and had not reported back after {wait:?}. It is being written or it is not; \
             nothing here knows which, and no `save` record claims either way until it \
             lands. Do not report the set as kept — look for it under that id."
        ),
        None => format!(
            "the render loop had not taken this save after {wait:?} — it is running slowly \
             or not at all. Nothing was saved, and asking again is safe."
        ),
    })
}

/// Awaits acknowledgment for an applied edge from the render loop up to `wait` duration.
///
/// Returns `Ok(result)` on loop acceptance, or `Err(reason)` if the edge was refused or timed out.
pub(crate) fn applied(
    news: &mpsc::Receiver<News>,
    wait: std::time::Duration,
) -> Result<String, String> {
    let deadline = std::time::Instant::now() + wait;
    let mut accepted: Option<String> = None;
    while let Some(left) = deadline.checked_duration_since(std::time::Instant::now()) {
        match news.recv_timeout(left) {
            Ok(News::Accepted(said)) => accepted = Some(said),
            Ok(News::Settled(outcome)) => return outcome,
            Err(mpsc::RecvTimeoutError::Timeout) => break,
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err(match accepted {
                    Some(said) => format!(
                        "{said}\n\nThe render loop then ended without saying what it did \
                         with the edge. Whether the slot was rewired is not something this \
                         server can still find out — `read_set` on a set saved since would \
                         say, and nothing else here will."
                    ),
                    None => "the render loop ended before it took this edge: nothing was \
                             rewired"
                        .to_string(),
                })
            }
        }
    }
    Err(match accepted {
        Some(said) => format!(
            "{said}\n\n**This is neither a success nor a failure.** The edge was taken and \
             the loop had not said what it did with it after {wait:?}, which is a frame it \
             should have answered on. Do not write the same edge again on the assumption \
             that it was lost — ask `swap_outcome` what the slot has been doing."
        ),
        None => format!(
            "the render loop had not taken this edge after {wait:?} — it is running slowly, \
             it is not running frames at all, or this run's loop does not take edges. \
             Nothing was rewired, and asking again is safe."
        ),
    })
}
