use std::sync::mpsc;

use karakuri_operation::Operation;
use serde_json::Value;

use super::super::*;
use super::*;

/// `wire_input`'s arguments as the operation they name.
///
/// **Both ends are names and neither is a [`NodeAddress`]**, which is the one place
/// this surface departs from the address the rest of it uses — and it is a
/// decision made twice before this tool existed. `Record::Edge` states it: *a
/// position moves when the list is reordered, and reordering silently changing
/// which geometry a morph blends towards is the exact failure this record exists
/// to end.* [`NodeAddress`]'s own documentation states the other half: *so
/// `Operation::WireInput` takes names and everything else takes this, and the
/// two are not interchangeable.* A tool here that took `{layer, index}` because
/// its five neighbours do would be spelling an edge in the one address an edge
/// may not be spelled in, and
/// `docs/principles/0086-a-procedure-knows-only-what-it-declares.md` is what it would
/// be breaking: the names are the *Set's* answer, and a procedure never knows
/// them.
///
/// **Nothing here checks that the names resolve, and that is not laziness.**
/// The nodes of a Set are named by the Set — `--set morph=warp.kir` names one,
/// and a bare file is named after the procedure inside it — and [`Slots`] holds
/// paths, not names, because a path never crosses this protocol. So a check
/// built from the files alone would accept an edge that is wrong wherever a
/// name was given on the command line and refuse one that is right, which is
/// `docs/principles/0084-…`'s failure exactly: a warning that fires on healthy
/// material. The names are refused where the Set is built, in the sentence
/// `--edge` meets there too
/// (`docs/principles/0090-a-surface-offers-it-never-decides.md`).
///
/// **The deck is checked last**, after all four arguments have been read, on
/// [`written_procedure`]'s terms: an argument this tool cannot do without is
/// named before a slot number that may also be wrong.
pub(crate) fn wired_input(args: &Value, slots: &Slots) -> Result<Operation, String> {
    let slot = args
        .get("slot")
        .and_then(Value::as_u64)
        .ok_or("`slot` is required and is a number")? as usize;
    let node = edge_name(args, "node", "the node that declares the input")?;
    let input = edge_name(args, "input", "what that node's procedure calls the input")?;
    let to = edge_name(args, "to", "the node bound to it")?;
    let deck = deck_named(slot, slots)?;
    Ok(Operation::WireInput {
        deck,
        node,
        // **`Operation::WireInput`'s `slot` is the *input*, and this surface's
        // `slot` is the deck's.** One word for two things is what the schema's
        // `input` exists to avoid
        // (`docs/contributing.md` §4),
        // and this line is where the two spellings meet.
        slot: input.into(),
        to,
    })
}

/// One end of an edge, or the refusal an empty one earns.
///
/// **Empty is refused rather than resolved to nothing**, which is `parse_edge`'s
/// rule on the command line and its sentence: *every part names something.* An
/// edge with no slot in it is a statement about a node, and there is no such
/// statement — and a name is not otherwise constrained here, because a node is
/// called whatever `--set` or a `proc` line called it and this server is not the
/// thing that decides what that may be.
pub(crate) fn edge_name(args: &Value, key: &str, what: &str) -> Result<String, String> {
    let named = args
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("`{key}` is required and is a string: {what}"))?;
    if named.is_empty() {
        return Err(format!(
            "`{key}` is empty, and every part of an edge names something: a node, the \
             input it declares, and the node bound to it"
        ));
    }
    Ok(named.to_string())
}

/// **[`Operation::WireInput`], done**: ask the render loop to bind one node's
/// declared input to another node.
///
/// Nothing about the Set is read here and nothing could be — see
/// [`wired_input`] on why the names are not checked on this side — so what this
/// does is hand the request over and give the caller back the half it waits on.
///
/// **It is `save_set`'s shape and `write_procedure`'s promise.** The request
/// goes to the loop like a save, because only the loop holds what a slot is
/// wired with; but what comes back is a write's answer rather than a save's — an
/// edge lands the way an edited procedure lands, at a frame boundary and under
/// the same budget, so the tool answers when the edge is *written* and points at
/// `swap_outcome` for what the build made of it. See [`WIRE_REPLY`].
///
/// **No record is written anywhere, and that is a hole rather than a design.**
/// `karakuri_operation_record::written` answers `Silent(Silent::NoRecord)` for
/// `WireInput`: `Record::Edge` exists and is a *Set file's*, with no `slot` to
/// carry the deck this operation names. So a rewiring during a set is the one
/// thing a model can do here that a replay does not reconstruct — see the module
/// documentation, which says what would close it.
pub(crate) fn wire_input(
    deck: u8,
    node: &str,
    input: &str,
    to: &str,
    state: &State,
) -> Result<(mpsc::Receiver<News>, String), String> {
    let slot = usize::from(deck);
    let (tx, rx) = mpsc::channel();
    state
        .wiring
        .try_send(WireRequest {
            slot,
            edge: karakuri_engine::set::Edge {
                node: node.to_string(),
                slot: input.into(),
                to: to.to_string(),
            },
            reply: Reply(tx),
        })
        // **Answered rather than waited for**, in [`save_set`]'s two shapes and
        // its words: a queue nobody is emptying and a loop that has ended are
        // different facts, and neither of them may leave a model holding a call.
        .map_err(|e| match e {
            mpsc::TrySendError::Full(_) => format!(
                "the render loop has {ASKED} edges queued and no room for another: it is \
                 taking them slower than they are arriving, or it is not running frames at \
                 all. Nothing was rewired, and asking again is safe"
            ),
            mpsc::TrySendError::Disconnected(_) => {
                "the render loop has ended: this run is shutting down and nothing was \
                 rewired"
                    .to_string()
            }
        })?;
    // **What the loop's own sentence will not say.** The loop knows what it did
    // with the edge; only this side knows how the run was started, and a run
    // without `--watch` has no watcher to rebuild the slot with the new wiring —
    // which is the same thing `write_procedure` says about a file nothing will
    // pick up, about the other half of one edit.
    let note = if state.watching {
        String::new()
    } else {
        "**This run was started without `--watch`, so no watcher will rebuild the slot** \
         — what is on screen was built with the wiring this run started with and will go \
         on being it. The edge is the run's from here on, so a `save_set` of this slot \
         records it; the picture does not change."
            .to_string()
    };
    Ok((rx, note))
}
