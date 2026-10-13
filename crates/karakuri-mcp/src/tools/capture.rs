//! MCP tool for capturing rendered frames of deck slots or the composed Program.

use serde_json::Value;

use super::Called;
use crate::state::{CaptureReply, CaptureRequest, CaptureTarget};
use crate::State;
use karakuri_operation::RefusalDetail;

/// Dispatches a `capture_frame` tool invocation, validating parameters and queuing a render capture.
pub(crate) fn capture_frame(args: &Value, state: &mut State) -> Result<Called, String> {
    let target_str = args
        .get("target")
        .and_then(Value::as_str)
        .unwrap_or("program");

    let target = match target_str.to_ascii_lowercase().as_str() {
        "program" => CaptureTarget::Program,
        "slot" => {
            let slot = match args.get("slot").and_then(Value::as_u64) {
                Some(s) => s as usize,
                None => {
                    return Ok(Called::Answered(Err(
                        "`slot` is required when target is 'slot'".into(),
                    )))
                }
            };
            if let Err(e) = state.slots.holds(slot) {
                return Ok(Called::Answered(Err(super::refusal_payload(
                    &RefusalDetail::slot_unallocated(slot, e),
                ))));
            }
            CaptureTarget::Slot(slot)
        }
        other => {
            return Ok(Called::Answered(Err(format!(
                "unknown target `{other}`: must be 'program' or 'slot'"
            ))))
        }
    };

    let max_dim = match args.get("max_dimension").and_then(Value::as_u64) {
        Some(dim) => {
            if !(128..=1024).contains(&dim) {
                return Ok(Called::Answered(Err(format!(
                    "`max_dimension` must be between 128 and 1024 (got {dim})"
                ))));
            }
            dim as u32
        }
        None => 512,
    };

    let (tx, rx) = std::sync::mpsc::channel();
    let reply = CaptureReply(tx);
    let request = CaptureRequest {
        target,
        max_dimension: max_dim,
        reply,
    };

    if state.capturing.try_send(request).is_err() {
        return Ok(Called::Answered(Err(
            "the render loop is disconnected or too busy to accept capture requests".into(),
        )));
    }

    Ok(Called::Capturing(rx))
}
