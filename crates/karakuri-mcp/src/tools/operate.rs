use std::sync::mpsc;

use karakuri_operation::Operation;

use super::super::*;

/// **[`Operation`], asked of the render loop.**
///
/// The third thing this server reaches the loop for, and it is the loop for the
/// reason a save and an edge are: this is where every other surface's presses
/// are performed. A model's `SetGain` and an operator's hand on the fader end
/// in one function, on one thread, at one frame — which is what
/// [P-0090](../../../docs/principles/0090-a-surface-offers-it-never-decides.md)
/// asks of a fourth route and what performing it here instead would give up.
///
/// **What the loop owes a request it takes**, written where the sender is:
///
/// 1. **Perform it where a press of the same operation is performed**, and
///    nowhere else. Not a second route into the deck.
/// 2. **Answer once, at the frame it was performed on** — [`Reply::settled`].
///    Not at the swap: what a *rebuild* made of an operation that starts one is
///    `swap_outcome`'s answer, as it is for a written procedure.
pub struct OperateRequest {
    /// The operation, already through the audit — see [`audited`]. The loop
    /// performs it and does not judge it again.
    pub operation: Operation,
    /// Where the answer goes. One message: see the contract above.
    pub reply: Reply,
}

/// **[`Operation`], done**: hand it to the render loop and give the caller back
/// the half it waits on.
///
/// Nothing is performed here and nothing could be: this thread holds no deck,
/// no look and no chain, and a surface that performed the mix on a connection
/// thread would be the second route
/// [P-0090](../../../docs/principles/0090-a-surface-offers-it-never-decides.md)
/// exists to prevent. It is [`save_set`]'s shape and [`wire_input`]'s promise.
pub(crate) fn operate(
    operation: &Operation,
    state: &State,
) -> Result<mpsc::Receiver<News>, String> {
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

/// **How long an `operate` call waits for the render loop to perform it.**
///
/// [`WIRE_REPLY`] and not [`SAVE_REPLY`], for that constant's reason and the
/// same one: this waits for the loop to reach the top of a frame and act, which
/// is one frame at any frame rate anybody plays at, and no disk is involved.
/// What a *rebuild* made of an operation that starts one lands thirty judged
/// frames later and is `swap_outcome`'s answer.
pub(crate) const OPERATE_REPLY: std::time::Duration = WIRE_REPLY;
