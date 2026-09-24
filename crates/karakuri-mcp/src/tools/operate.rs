use std::sync::mpsc;

use karakuri_operation::Operation;

use super::super::*;

/// Request to perform an operation on the render loop thread.
pub struct OperateRequest {
    /// The operation to perform.
    pub operation: Operation,
    /// Channel reply handle for reporting execution outcome.
    pub reply: Reply,
}

/// Dispatches an operation to the render loop, returning a receiver for the result.
pub(crate) fn operate(
    operation: &Operation,
    state: &State,
) -> Result<mpsc::Receiver<News>, String> {
    if let Operation::WriteParam { deck, .. }
    | Operation::TakeParamBack { deck, .. }
    | Operation::LoadProcedure { deck, .. }
    | Operation::RestoreProcedure { deck, .. }
    | Operation::SelectRenderer { deck, .. }
    | Operation::LoadSet { deck, .. }
    | Operation::SetCompositing { deck, .. }
    | Operation::AttachSignal { deck, .. }
    | Operation::SetProperty { deck, .. }
    | Operation::Publish { deck, .. } = operation
    {
        state
            .slot_policies
            .check_writable_detail(usize::from(*deck))
            .map_err(|d| refusal_payload(&d))?;
    }
    let (tx, rx) = mpsc::channel();
    state
        .operating
        .try_send(OperateRequest {
            operation: operation.clone(),
            reply: Reply(tx),
        })
        .map_err(|e| match e {
            mpsc::TrySendError::Full(_) => format!(
                "the render loop has {ASKED} operations queued and no room for another: it \
                 is taking them slower than they are arriving, or it is not running frames \
                 at all. Nothing was performed, and asking again is safe"
            ),
            mpsc::TrySendError::Disconnected(_) => {
                "the render loop has ended: this run is shutting down and nothing was \
                 performed"
                    .to_string()
            }
        })?;
    Ok(rx)
}

/// Duration to wait for the render loop to perform an operation.
pub(crate) const OPERATE_REPLY: std::time::Duration = WIRE_REPLY;
