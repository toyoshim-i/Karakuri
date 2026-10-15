pub mod capture;
mod dispatch;
pub mod history;
pub mod library;
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

use crate::state::LAYERS;
pub(crate) use capture::capture_frame;
pub(crate) use library::list_library;
pub use operate::OperateRequest;
pub use procedure::check_procedure;
pub(crate) use schema::build_tools;
pub use schema::tools;
pub use set::{check_set_configuration, LISTED};

use agent_man::mcp::{HandlerResult, PendingResult, ToolResult};

pub(crate) use dispatch::{perform, sayable, Sayable};
pub(crate) use history::*;
pub(crate) use procedure::*;
pub(crate) use set::*;
pub(crate) use wire::*;

use crate::{
    layer_list, layer_name, layer_named, layer_of, operated, CaptureNews, CapturedImage,
    DiagnosticReport, News, Slots, State, MAX_ID,
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
    /// A frame capture the loop has been asked to perform — see [`CaptureRequest`].
    Capturing(mpsc::Receiver<CaptureNews>),
}

/// Resolves an incoming MCP tool invocation into an [`Operation`] or an argument refusal.
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
        // Filesystem history walk performed off the render thread (ADR-0342).
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

/// Result of parsing a tool call: an executable operation or a formatted refusal.
pub(crate) enum Asked {
    Named(Operation),
    Refused(String),
}

/// Resolves a slot index to a deck number (`u8`), verifying allocation and bounds.
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

/// Parses single or batch write_procedure arguments and delegates to write_procedures_batch.
pub(crate) fn handle_write_procedure(args: &Value, state: &State) -> Result<Called, String> {
    let slot = match args.get("slot").and_then(Value::as_u64) {
        Some(s) => s as usize,
        None => {
            return Ok(Called::Answered(Err(
                "`slot` is required and is a number".into()
            )))
        }
    };
    if let Err(e) = state.slots.holds(slot) {
        let detail = RefusalDetail::slot_unallocated(slot, e);
        return Ok(Called::Answered(Err(refusal_payload(&detail))));
    }

    let deck = match deck_named(slot, &state.slots) {
        Ok(d) => d,
        Err(e) => return Ok(Called::Answered(Err(e))),
    };

    let mut updates = Vec::new();
    if let Some(nodes_val) = args.get("nodes").and_then(Value::as_array) {
        if nodes_val.is_empty() {
            return Ok(Called::Answered(Err(
                "`nodes` array must not be empty".into()
            )));
        }
        for (i, node_val) in nodes_val.iter().enumerate() {
            let layer_str = match node_val.get("layer").and_then(Value::as_str) {
                Some(l) => l,
                None => {
                    return Ok(Called::Answered(Err(format!(
                        "nodes[{i}].layer is required"
                    ))))
                }
            };
            let layer = match layer_named(layer_str) {
                Some(l) => l,
                None => {
                    return Ok(Called::Answered(Err(format!(
                        "`{layer_str}` is not a valid layer"
                    ))))
                }
            };
            let index = node_val.get("index").and_then(Value::as_u64).unwrap_or(0) as usize;
            let source = match node_val.get("source").and_then(Value::as_str) {
                Some(s) => s.to_string(),
                None => {
                    return Ok(Called::Answered(Err(format!(
                        "nodes[{i}].source is required"
                    ))))
                }
            };
            updates.push((layer, index, source));
        }
    } else {
        let layer_str = match args.get("layer").and_then(Value::as_str) {
            Some(l) => l,
            None => {
                let msg = format!(
                    "`layer` is required and names a layer: {}",
                    LAYERS
                        .iter()
                        .map(|layer| layer_name(*layer))
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                return Ok(Called::Answered(Err(msg)));
            }
        };
        let layer = match layer_named(layer_str) {
            Some(l) => l,
            None => {
                let msg = format!(
                    "`{layer_str}` is not a layer: {}",
                    LAYERS
                        .iter()
                        .map(|l| layer_name(*l))
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                return Ok(Called::Answered(Err(msg)));
            }
        };
        let index = args.get("index").and_then(Value::as_u64).unwrap_or(0) as usize;
        let source = match args.get("source").and_then(Value::as_str) {
            Some(s) => s.to_string(),
            None => return Ok(Called::Answered(Err("`source` is required".into()))),
        };
        updates.push((layer, index, source));
    }

    Ok(Called::Answered(super::procedure::write_procedures_batch(
        deck, &updates, state,
    )))
}

pub(crate) fn dispatch_tool(name: &str, args: &Value, state: &mut State) -> Result<Called, String> {
    if name == "write_procedure" {
        return handle_write_procedure(args, state);
    }
    if name == "capture_frame" {
        return capture_frame(args, state);
    }
    if name == "list_library" {
        return Ok(Called::Answered(list_library(args, state)));
    }
    if name == "read_procedure" {
        if let Some(proc_name) = args.get("name").and_then(Value::as_str) {
            return Ok(Called::Answered(read_procedure_by_name(proc_name, state)));
        }
    }
    if name == "check_procedure" {
        let source = match args.get("source").and_then(Value::as_str) {
            Some(s) => s,
            None => return Ok(Called::Answered(Err("`source` is required".into()))),
        };
        let report = check_procedure(source);
        let report_json = serde_json::to_string_pretty(&report).unwrap_or_default();
        return Ok(Called::Answered(if report.success {
            Ok(report_json)
        } else {
            Err(report_json)
        }));
    }
    if name == "check_set" {
        let id = match args.get("id").and_then(Value::as_str) {
            Some(i) => i,
            None => return Ok(Called::Answered(Err("`id` is required".into()))),
        };
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
                "library": opening.library().name(),
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
            let source = match karakuri_store::fs::read_to_string(&path) {
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
                let source = match karakuri_store::fs::read(from_path) {
                    Ok(s) => s,
                    Err(e) => {
                        for (tmp, _) in &staging {
                            let _ = karakuri_store::fs::remove_file(tmp);
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
                if let Err(e) = karakuri_store::fs::write(&tmp_path, &source) {
                    for (tmp, _) in &staging {
                        let _ = karakuri_store::fs::remove_file(tmp);
                    }
                    let _ = karakuri_store::fs::remove_file(&tmp_path);
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
            if let Err(e) = karakuri_store::fs::rename(&tmp_path, &to_path) {
                return Ok(Called::Answered(Err(format!("{}: {e}", to_path.display()))));
            }
            copied += 1;
        }

        return Ok(Called::Answered(Ok(format!(
            "copied {copied} procedure{} from slot {from_slot} to slot {to_slot}",
            if copied == 1 { "" } else { "s" }
        ))));
    }

    Ok(match asked(name, args, &state.slots)? {
        // Unified operation gate: named, audited, and performed.
        Asked::Named(operation) => match audited(&operation, state) {
            Ok(allowed) => perform(&allowed, state),
            Err(detail) => Called::Answered(Err(refusal_payload(&detail))),
        },
        Asked::Refused(refusal) => Called::Answered(Err(refusal)),
    })
}

/// Audits an operation against the server's opening state and residency.
pub(crate) fn audited<'a>(
    operation: &'a Operation,
    state: &State,
) -> Result<Allowed<'a>, RefusalDetail> {
    let open = state.opening.read();
    let all = state.slot_policies.all();
    let live_decks: Vec<u8> = all
        .iter()
        .enumerate()
        .filter(|(_, a)| a.in_mix)
        .map(|(i, _)| i as u8)
        .collect();
    let running = gate::Running::live(&live_decks);

    let deck_target = match operation {
        Operation::WriteParam { deck, .. }
        | Operation::TakeParamBack { deck, .. }
        | Operation::AttachSignal { deck, .. }
        | Operation::LoadProcedure { deck, .. }
        | Operation::LoadSet { deck, .. } => Some(usize::from(*deck)),
        _ => None,
    };
    if let Some(slot) = deck_target {
        match state.slot_policies.check_writable_detail(slot) {
            Ok(()) => {
                let open_with_slot = open.with(gate::Class::LiveDeck, true);
                return gate::audit_detail(operation, open_with_slot, running);
            }
            Err(detail) => {
                // If the target slot policy refuses writes, report that directly.
                return Err(detail);
            }
        }
    }
    gate::audit_detail(operation, open, running)
}

/// Serialise a [`RefusalDetail`] into a JSON string for structured envelope reporting.
pub(crate) fn refusal_payload(detail: &RefusalDetail) -> String {
    let hint =
        crate::resources::doc_engine().hint(detail.code.as_str(), agent_man::RenderMode::Mcp);
    json!({
        "code": detail.code.as_str(),
        "message": detail.message,
        "slot": detail.slot,
        "policy": detail.policy.map(|p| p.name()),
        "in_mix": detail.in_mix,
        "hint": hint,
    })
    .to_string()
}

/// Formats a tool execution outcome into agent-man's ToolResult with structuredContent.
pub(crate) fn tool_result(outcome: Result<String, String>) -> ToolResult {
    let engine = crate::resources::doc_engine();
    match outcome {
        Ok(text) => {
            if let Ok(mut report) = serde_json::from_str::<DiagnosticReport>(&text) {
                for d in &mut report.diagnostics {
                    if let Some(h) = engine.hint(&d.code, agent_man::RenderMode::Mcp) {
                        d.hint = Some(h);
                    }
                }
                if let Some(first_d) = report.diagnostics.first() {
                    report.hint = first_d.hint.clone();
                }
                let mut content_text = text.clone();
                if let Some(h) = &report.hint {
                    content_text = format!("{content_text}\n\n{h}");
                }
                ToolResult::structured(content_text, json!({ "report": report }))
            } else {
                ToolResult::text(text)
            }
        }
        Err(text) => {
            if let Ok(mut report) = serde_json::from_str::<DiagnosticReport>(&text) {
                for d in &mut report.diagnostics {
                    if let Some(h) = engine.hint(&d.code, agent_man::RenderMode::Mcp) {
                        d.hint = Some(h);
                    }
                }
                if let Some(first_d) = report.diagnostics.first() {
                    report.hint = first_d.hint.clone();
                }
                let mut content_text = text.clone();
                if let Some(h) = &report.hint {
                    content_text = format!("{content_text}\n\n{h}");
                }
                ToolResult::error_structured(content_text, json!({ "report": report }))
            } else if let Ok(mut val) = serde_json::from_str::<Value>(&text) {
                if let Some(refusal) = val.get_mut("refusal") {
                    let code = refusal.get("code").and_then(Value::as_str);
                    let hint = code.and_then(|c| engine.hint(c, agent_man::RenderMode::Mcp));
                    if let Some(h) = &hint {
                        refusal["hint"] = json!(h);
                    }
                    let msg = refusal
                        .get("message")
                        .and_then(Value::as_str)
                        .unwrap_or(&text);
                    let full_msg = match &hint {
                        Some(h) => format!("{msg}\n\n{h}"),
                        None => msg.to_string(),
                    };
                    ToolResult::error_structured(full_msg, json!({ "refusal": refusal }))
                } else if val.get("code").is_some() && val.get("message").is_some() {
                    let code = val.get("code").and_then(Value::as_str);
                    let hint = code.and_then(|c| engine.hint(c, agent_man::RenderMode::Mcp));
                    if let Some(h) = &hint {
                        val["hint"] = json!(h);
                    }
                    let msg = val.get("message").and_then(Value::as_str).unwrap_or(&text);
                    let full_msg = match &hint {
                        Some(h) => format!("{msg}\n\n{h}"),
                        None => msg.to_string(),
                    };
                    ToolResult::error_structured(full_msg, json!({ "refusal": val }))
                } else {
                    ToolResult::error(text)
                }
            } else {
                ToolResult::error(text)
            }
        }
    }
}

/// A pending asynchronous operation to be awaited with [`State`] unlocked.
pub enum Pending {
    /// A save the render loop has been asked for.
    Saving { news: mpsc::Receiver<News> },
    /// An operation the render loop has been asked for.
    Operating { news: mpsc::Receiver<News> },
    /// A rewiring the render loop has been asked for.
    Wiring {
        news: mpsc::Receiver<News>,
        note: String,
    },
    /// A frame capture the render loop has been asked for.
    Capturing { news: mpsc::Receiver<CaptureNews> },
}

const SAVE_GONE: &str = "the render loop ended before it took this save: nothing was saved";
const OPERATE_GONE: &str = "the render loop disconnected before this operation completed";
const WIRE_GONE: &str = "the render loop disconnected before this wire completed";
const CAPTURE_GONE: &str = "the render loop disconnected before this capture completed";

fn noted(outcome: Result<String, String>, note: &str) -> Result<String, String> {
    outcome.map(|said| {
        if note.is_empty() {
            said
        } else {
            format!("{said}\n\n{note}")
        }
    })
}

fn capture_result(outcome: Result<CapturedImage, String>) -> ToolResult {
    match outcome {
        Ok(image) => ToolResult::text_and_image(image.description, image.data, image.mime_type)
            .with_structured(json!({
                "width": image.width,
                "height": image.height,
                "format": "image/png",
            })),
        Err(err) => ToolResult::error(err),
    }
}

impl PendingResult<ToolResult> for Pending {
    fn poll_settled(&mut self) -> Option<ToolResult> {
        if let Pending::Capturing { news } = self {
            return match news.try_recv() {
                Ok(CaptureNews::Settled(outcome)) => Some(capture_result(outcome)),
                Err(mpsc::TryRecvError::Empty) => None,
                Err(mpsc::TryRecvError::Disconnected) => Some(ToolResult::error(CAPTURE_GONE)),
            };
        }
        let (news, gone, note) = match self {
            Pending::Saving { news } => (news, SAVE_GONE, ""),
            Pending::Operating { news } => (news, OPERATE_GONE, ""),
            Pending::Wiring { news, note } => (news, WIRE_GONE, note.as_str()),
            Pending::Capturing { .. } => unreachable!(),
        };
        match news.try_recv() {
            Ok(News::Settled(outcome)) => Some(tool_result(noted(outcome, note))),
            Ok(News::Accepted(_)) | Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => Some(ToolResult::error(gone)),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn settled(self: Box<Self>) -> ToolResult {
        match *self {
            Pending::Saving { news } => tool_result(awaited(&news, crate::SAVE_REPLY)),
            Pending::Operating { news } => tool_result(applied(&news, operate::OPERATE_REPLY)),
            Pending::Wiring { news, note } => {
                tool_result(noted(applied(&news, crate::WIRE_REPLY), &note))
            }
            Pending::Capturing { news } => {
                capture_result(awaited_capture(&news, crate::CAPTURE_REPLY))
            }
        }
    }
}

pub(crate) fn called_to_handler_result(called: Called) -> HandlerResult<ToolResult> {
    match called {
        Called::Answered(outcome) => HandlerResult::Ready(tool_result(outcome)),
        Called::Saving(news) => HandlerResult::Pending(Box::new(Pending::Saving { news })),
        Called::Wiring { news, note } => {
            HandlerResult::Pending(Box::new(Pending::Wiring { news, note }))
        }
        Called::Operating(news) => HandlerResult::Pending(Box::new(Pending::Operating { news })),
        Called::Capturing(news) => HandlerResult::Pending(Box::new(Pending::Capturing { news })),
    }
}

/// Parses `(slot, layer, index)` arguments from tool inputs, defaulting `index` to 0.
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

/// Validates that a Set ID is a single non-empty path component of ASCII alphanumeric,
/// hyphen, or underscore characters, within `MAX_ID` bytes.
pub fn checked_id(id: &str) -> Result<String, String> {
    if id.is_empty() {
        return Err(
            "`id` is empty: a set is filed under a name, or under none at all if \
                    `id` is left out"
                .into(),
        );
    }
    // Length check in bytes matching `str::len`.
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

/// Awaits the outcome of a save operation over `news` until `wait` expires.
///
/// Returns `Ok(outcome)` if settled before timeout, or `Err` with diagnostic text on timeout.
#[cfg(not(target_arch = "wasm32"))]
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
            // Request was dropped without a response during shutdown.
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
#[cfg(not(target_arch = "wasm32"))]
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

/// Awaits the outcome of a frame capture operation over `news` until `wait` expires.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn awaited_capture(
    news: &mpsc::Receiver<CaptureNews>,
    wait: std::time::Duration,
) -> Result<CapturedImage, String> {
    match news.recv_timeout(wait) {
        Ok(CaptureNews::Settled(outcome)) => outcome,
        Err(mpsc::RecvTimeoutError::Timeout) => Err(format!(
            "the render loop had not completed this capture after {wait:?} — it is running slowly \
             or not at all."
        )),
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            Err("the render loop ended before it completed this capture".to_string())
        }
    }
}
