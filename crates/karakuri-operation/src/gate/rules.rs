use crate::types::{RefusalCode, RefusalDetail};
use crate::Operation;

use super::types::{Allowed, Class, Open, Reading, Running, Standing, Unclassed};

/// Performs structured policy audit verifying whether an operation is allowed (ADR-0235).
pub fn audit_detail<'a>(
    operation: &'a Operation,
    open: Open,
    running: Running<'_>,
) -> Result<Allowed<'a>, RefusalDetail> {
    let standing = standing(operation, running);
    match standing {
        Standing::Open => Ok(Allowed(operation)),
        Standing::Closed(class) if open.holds(class) => Ok(Allowed(operation)),
        _ => Err(refusal_detail(operation, standing)
            .expect("a standing that is not `Open` and not an opened class has a refusal")),
    }
}

/// Checks whether an operation is permitted under active gate rules, returning error text on refusal.
pub fn audit<'a>(
    operation: &'a Operation,
    open: Open,
    running: Running<'_>,
) -> Result<Allowed<'a>, String> {
    audit_detail(operation, open, running).map_err(|detail| detail.message)
}

/// Detailed structured refusal for an operation rejected by the audit gate.
pub fn refusal_detail(operation: &Operation, standing: Standing) -> Option<RefusalDetail> {
    let title = operation.title();
    match standing {
        Standing::Open => None,
        Standing::Closed(class) => {
            let because_clause = because(operation, class);
            let deck = match operation {
                Operation::LoadSet { deck, .. } | Operation::LoadProcedure { deck, .. } => {
                    Some(*deck)
                }
                _ => None,
            };
            let message = format!(
                "`{title}`{because_clause} is in the class {}, which is closed by default — the operator opens \
                 it at {}.",
                class.title(),
                class.opened_at()
            );
            Some(RefusalDetail {
                code: RefusalCode::BayClosed,
                message,
                slot: deck.map(|d| d as usize),
                deck,
                lane: None,
                class: Some(class),
                policy: None,
                in_mix: None,
            })
        }
        Standing::ClosedUnclassed(group) => {
            let message = format!(
                "`{title}` is closed by default with {}, and no bay opens that yet — which class \
                 it belongs to is not settled.",
                group.title()
            );
            Some(RefusalDetail {
                code: RefusalCode::BayClosed,
                message,
                slot: None,
                deck: None,
                lane: None,
                class: None,
                policy: None,
                in_mix: None,
            })
        }
        Standing::Unread(Reading::Live) => {
            let message = format!(
                "`{title}` is closed by default where the deck it names is live, and which decks \
                 are live was not read."
            );
            Some(RefusalDetail {
                code: RefusalCode::BayClosed,
                message,
                slot: None,
                deck: None,
                lane: None,
                class: None,
                policy: None,
                in_mix: None,
            })
        }
    }
}

/// Formats the single-sentence human-readable refusal message for an operation (P-0083, P-0090).
pub fn refusal(operation: &Operation, standing: Standing) -> Option<String> {
    refusal_detail(operation, standing).map(|d| d.message)
}

/// The clause a refusal owes where the class turned on more than the
/// operation's name. Empty for every row but the two loads — see [`refusal`].
fn because(operation: &Operation, class: Class) -> String {
    match (operation, class) {
        (Operation::LoadSet { deck, .. }, Class::LiveDeck)
        | (Operation::LoadProcedure { deck, .. }, Class::LiveDeck) => {
            format!(" (deck {deck} is live)")
        }
        _ => String::new(),
    }
}

/// Exhaustively evaluates the policy standing of an operation against runtime state (ADR-0235).
pub fn standing(operation: &Operation, running: Running<'_>) -> Standing {
    match operation {
        // ----- The clock ---------------------------------------------------
        // Immediate, unpriced clock adjustments (ADR-0235).
        Operation::TapBeat => Standing::ClosedUnclassed(Unclassed::Clock),
        Operation::ScaleGrid { .. } => Standing::ClosedUnclassed(Unclassed::Clock),
        Operation::SetLatencyOffset { .. } => Standing::ClosedUnclassed(Unclassed::Clock),
        Operation::SetSync { .. } => Standing::ClosedUnclassed(Unclassed::Clock),
        Operation::ScrubDeck { .. } => Standing::ClosedUnclassed(Unclassed::Clock),
        Operation::SetFreeRunTempo { .. } => Standing::ClosedUnclassed(Unclassed::Clock),

        // ----- Inputs and outputs, routed, enabled and disabled -------------
        // I/O routing and session recording operations (ADR-0235, ADR-0240).
        Operation::AttachBeatSource { .. } => Standing::Closed(Class::InputsAndOutputs),
        Operation::RouteFrame { .. } => Standing::Closed(Class::InputsAndOutputs),
        Operation::RecordSession { .. } => Standing::Closed(Class::InputsAndOutputs),

        // ----- Which deck the next key press lands on -----------------------
        Operation::SelectDeck { .. } => Standing::ClosedUnclassed(Unclassed::Selection),

        // ----- What a deck that is live is drawing --------------------------
        // Operations affecting visible live deck rendering output (ADR-0235).
        Operation::SetResidency { .. } => Standing::Closed(Class::LiveDeck),
        // Loading into a live deck replaces visual output; non-live decks remain open.
        Operation::LoadSet { deck, .. } => match running.live {
            None => Standing::Unread(Reading::Live),
            Some(live) if live.contains(deck) => Standing::Closed(Class::LiveDeck),
            Some(_) => Standing::Open,
        },
        Operation::LoadProcedure { deck, .. } => match running.live {
            None => Standing::Unread(Reading::Live),
            Some(live) if live.contains(deck) => Standing::Closed(Class::LiveDeck),
            Some(_) => Standing::Open,
        },
        Operation::SetCompositing { .. } => Standing::Closed(Class::LiveDeck),
        Operation::SelectRenderer { .. } => Standing::Closed(Class::LiveDeck),
        Operation::WriteParam { .. } => Standing::Closed(Class::LiveDeck),
        Operation::AttachSignal { .. } => Standing::Closed(Class::LiveDeck),
        Operation::TakeParamBack { .. } => Standing::Closed(Class::LiveDeck),
        Operation::SetProperty { .. } => Standing::Closed(Class::LiveDeck),
        // Modifying published interfaces renumbers bindings on live decks.
        Operation::Publish { .. } => Standing::Closed(Class::LiveDeck),

        // ----- The mix faders ----------------------------------------------
        // Direct audience-facing mixer controls (P-0094).
        Operation::SetGain { .. } => Standing::Closed(Class::MixFaders),
        Operation::SetOpacity { .. } => Standing::Closed(Class::MixFaders),
        Operation::SetMute { .. } => Standing::Closed(Class::MixFaders),
        Operation::SetSolo { .. } => Standing::Closed(Class::MixFaders),
        Operation::ClearSolo => Standing::Closed(Class::MixFaders),
        Operation::SetOnline { .. } => Standing::Closed(Class::MixFaders),
        Operation::SetBlendMode { .. } => Standing::Closed(Class::MixFaders),
        Operation::FadeDeck { .. } => Standing::Closed(Class::MixFaders),
        Operation::Crossfade { .. } => Standing::Closed(Class::MixFaders),
        Operation::Wipe { .. } => Standing::Closed(Class::MixFaders),
        Operation::SetMaskShape { .. } => Standing::Closed(Class::MixFaders),
        Operation::SetMaskPosition { .. } => Standing::Closed(Class::MixFaders),
        Operation::SetMasterOut { .. } => Standing::Closed(Class::MixFaders),

        // ----- The master effects -------------------------------------------
        Operation::SetChainParam { .. } => Standing::Closed(Class::MasterEffects),
        Operation::AddChainEffect { .. } => Standing::Closed(Class::MasterEffects),
        Operation::RemoveChainEffect { .. } => Standing::Closed(Class::MasterEffects),
        Operation::SetTonemap { .. } => Standing::Closed(Class::MasterEffects),
        Operation::SetExposure { .. } => Standing::Closed(Class::MasterEffects),

        // ----- The sequencer's lanes ----------------------------------------
        Operation::SetStep { .. } => Standing::ClosedUnclassed(Unclassed::Lanes),
        Operation::SetLaneMute { .. } => Standing::ClosedUnclassed(Unclassed::Lanes),
        Operation::PointLane { .. } => Standing::ClosedUnclassed(Unclassed::Lanes),
        Operation::RemoveLane { .. } => Standing::ClosedUnclassed(Unclassed::Lanes),
        Operation::SetPatternGrid { .. } => Standing::ClosedUnclassed(Unclassed::Lanes),
        Operation::SelectPattern { .. } => Standing::ClosedUnclassed(Unclassed::Lanes),

        // ----- Who may move a node ------------------------------------------
        Operation::SetAuthority { .. } => Standing::ClosedUnclassed(Unclassed::Authority),

        // ----- Quitting -----------------------------------------------------
        Operation::Quit => Standing::ClosedUnclassed(Unclassed::Quitting),

        // ----- Open by default ----------------------------------------------
        Operation::SetTransition { .. } => Standing::Open,
        Operation::WireInput { .. } => Standing::Open,
        Operation::SaveSet { .. } => Standing::Open,
        Operation::KeepProcedure { .. } => Standing::Open,
        Operation::ListSets { .. } => Standing::Open,
        Operation::FilterLibrary { .. } => Standing::Open,
        Operation::PointPane { .. } => Standing::Open,
        Operation::SelectScope { .. } => Standing::Open,
        Operation::SetFavourite { .. } => Standing::Open,
        Operation::ReadSet { .. } => Standing::Open,
        Operation::TransferSet { .. } => Standing::Open,
        Operation::WalkHistory { .. } => Standing::Open,
        Operation::ReadProcedure { .. } => Standing::Open,
        Operation::WriteProcedure { .. } => Standing::Open,
        Operation::WatchFiles { .. } => Standing::Open,
        Operation::SwapOutcome => Standing::Open,
        Operation::KeepCandidate { .. } => Standing::Open,
        Operation::RestoreProcedure { .. } => Standing::Open,
        Operation::MoveBoundary { .. } => Standing::Open,
        Operation::FoldBay { .. } => Standing::Open,
        Operation::FoldPane { .. } => Standing::Open,
        Operation::Unfold { .. } => Standing::Open,
        Operation::Solo { .. } => Standing::Open,
        Operation::ResetArrangement => Standing::Open,
        Operation::SaveArrangement { .. } => Standing::Open,
        Operation::RestoreArrangement { .. } => Standing::Open,
        Operation::SizeWindow { .. } => Standing::Open,
    }
}
