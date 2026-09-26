use karakuri_operation::gate::Allowed;
use karakuri_operation::Operation;

use super::history::*;
use super::operate::*;
use super::procedure::*;
use super::set::*;
use super::wire::*;
use super::Called;
use crate::State;

/// Classification of how `karakuri-mcp` handles an operation variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Sayable {
    /// Handled via the generic `operate` tool.
    Operable,
    /// Has a dedicated MCP tool; generic `operate` refers callers to it.
    Tool(&'static str),
    /// Surface-specific operation requiring an interactive GUI window.
    Window,
    /// Operation fundamentally unsupported over MCP protocol with explanatory reason.
    Never(&'static str),
}

/// Whether `operate` names this operation, and what it says where it does not.
/// Exhaustive, with no wildcard arm — see [`Sayable`].
pub(crate) fn sayable(operation: &Operation) -> Sayable {
    match operation {
        // Closed operation classes return standard refusals under ADR-0235.
        Operation::TapBeat
        | Operation::ScaleGrid { .. }
        | Operation::SetLatencyOffset { .. }
        | Operation::SetSync { .. }
        | Operation::ScrubDeck { .. }
        | Operation::SetFreeRunTempo { .. }
        | Operation::AttachBeatSource { .. }
        | Operation::SetResidency { .. }
        | Operation::LoadSet { .. }
        | Operation::SetCompositing { .. }
        | Operation::SetGain { .. }
        | Operation::SetOpacity { .. }
        | Operation::SetMute { .. }
        | Operation::SetSolo { .. }
        | Operation::ClearSolo
        | Operation::SetOnline { .. }
        | Operation::SetBlendMode { .. }
        | Operation::FadeDeck { .. }
        | Operation::Crossfade { .. }
        | Operation::Wipe { .. }
        | Operation::SetMaskShape { .. }
        | Operation::SelectRenderer { .. }
        | Operation::SetMasterOut { .. }
        | Operation::SetChainParam { .. }
        | Operation::AddChainEffect { .. }
        | Operation::RemoveChainEffect { .. }
        | Operation::SetTonemap { .. }
        | Operation::SetExposure { .. }
        | Operation::WriteParam { .. }
        | Operation::AttachSignal { .. }
        | Operation::TakeParamBack { .. }
        | Operation::SetProperty { .. }
        | Operation::SetAuthority { .. }
        | Operation::RestoreProcedure { .. }
        | Operation::SetMaskPosition { .. }
        | Operation::Publish { .. }
        | Operation::SetFavourite { .. }
        | Operation::RouteFrame { .. }
        | Operation::RecordSession { .. }
        | Operation::LoadProcedure { .. }
        | Operation::KeepProcedure { .. }
        | Operation::Quit => Sayable::Operable,

        // ----- the seven that have a tool of their own ---------------------
        Operation::ReadProcedure { .. } => Sayable::Tool("read_procedure"),
        Operation::WriteProcedure { .. } => Sayable::Tool("write_procedure"),
        Operation::WireInput { .. } => Sayable::Tool("wire_input"),
        Operation::SwapOutcome => Sayable::Tool("swap_outcome"),
        Operation::SaveSet { .. } => Sayable::Tool("save_set"),
        Operation::ReadSet { .. } => Sayable::Tool("read_set"),
        Operation::ListSets { .. } => Sayable::Tool("list_sets"),
        // Walk operation: reads store and answers with rows.
        Operation::WalkHistory { .. } => Sayable::Tool("walk_history"),

        // Operations requiring dedicated UI window contexts are excluded from models.
        Operation::SelectDeck { .. }
        | Operation::SelectScope { .. }
        | Operation::SetTransition { .. }
        | Operation::KeepCandidate { .. }
        | Operation::FoldBay { .. }
        | Operation::FoldPane { .. }
        | Operation::Unfold { .. }
        | Operation::Solo { .. }
        | Operation::ResetArrangement
        | Operation::SaveArrangement { .. }
        | Operation::RestoreArrangement { .. }
        | Operation::SizeWindow { .. }
        | Operation::SetStep { .. }
        | Operation::SetLaneMute { .. }
        | Operation::PointLane { .. }
        | Operation::RemoveLane { .. }
        | Operation::SetPatternGrid { .. }
        | Operation::SelectPattern { .. }
        | Operation::FilterLibrary { .. }
        | Operation::PointPane { .. }
        | Operation::MoveBoundary { .. } => Sayable::Window,

        // Unsupported over MCP protocol.
        Operation::TransferSet { .. } => Sayable::Never(
            "both halves of it are outside what this protocol carries. A send names no \
             destination and never will — it is a read, and a read's answer goes where the \
             surface that asked puts answers, which on the panel is the system's own save \
             dialog and a model cannot answer one. A take names a file, and paths never cross \
             this protocol. The route is the command line: `--package ID > FILE.kbset` sends \
             one and `--take-in FILE` takes one in",
        ),
        Operation::WatchFiles { .. } => Sayable::Never(
            "a model does not edit a file in another program: it calls `write_procedure`, \
             which **is** its edit — checked, written, built on a worker and swapped at a \
             frame boundary — and there is nothing this row would add to it. The row names \
             an event rather than an act, somebody saving a source a watcher is looking at, \
             and the watching itself is the command line's: `--watch` turns it on for a run",
        ),
    }
}

/// Performs an audited operation against the server state, dispatching to specific tool handlers.
pub(crate) fn perform(allowed: &Allowed<'_>, state: &mut State) -> Called {
    match allowed.operation() {
        Operation::ReadProcedure { deck, node } => {
            Called::Answered(read_procedure(*deck, *node, state))
        }
        Operation::WriteProcedure { deck, node, source } => {
            Called::Answered(write_procedure(*deck, *node, source, state))
        }
        Operation::SwapOutcome => Called::Answered(swap_outcome(state)),
        Operation::WireInput {
            deck,
            node,
            slot,
            to,
        } => match wire_input(*deck, node, slot.as_str(), to, state) {
            Ok((news, note)) => Called::Wiring { news, note },
            Err(refusal) => Called::Answered(Err(refusal)),
        },
        Operation::ReadSet { id } => Called::Answered(read_set(id, state)),
        Operation::ListSets { holds, layer } => {
            Called::Answered(list_sets(holds.as_deref(), *layer, state))
        }
        Operation::WalkHistory { set } => Called::Answered(walk_history(set.as_deref(), state)),
        Operation::SaveSet { deck, id } => match save_set(*deck, id.as_deref(), state) {
            Ok(news) => Called::Saving(news),
            Err(refusal) => Called::Answered(Err(refusal)),
        },
        // Generic operation dispatch verified by `sayable` and `audited`.
        other => match sayable(other) {
            Sayable::Operable => match operate(other, state) {
                Ok(news) => Called::Operating(news),
                Err(refusal) => Called::Answered(Err(refusal)),
            },
            _ => Called::Answered(Err(format!(
                "`{}` is an operation this server publishes no tool for",
                other.title()
            ))),
        },
    }
}
