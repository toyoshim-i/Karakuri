use crate::types::{RefusalCode, RefusalDetail};
use crate::Operation;

use super::types::{Allowed, Class, Open, Reading, Running, Standing, Unclassed};

/// The gate. One operation, the opening the operator has set, and what has been
/// read of what is running — a yes, or the one sentence it is refused in.
///
/// Called once, over the operation, after the call is named and before it acts.
/// Strongly-typed structured audit verifying whether an operation is allowed by active gate policy.
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

/// In `karakuri_environment::mcp` that is between `asked` and `perform`, which
/// is the only seam every tool crosses.
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

/// The one sentence a refused call is answered in, and `None` where there is
/// nothing to refuse.
///
/// A free function, because
/// [P-0090](../../../docs/principles/0090-a-surface-offers-it-never-decides.md)
/// says a refusal a person can reach from two surfaces is one sentence — and
/// this one will be reachable from a second automatic route the day a lane
/// exists (ADR-0222). Asserted by equality against this function rather than by
/// a `contains`.
///
/// It says three things because
/// [P-0083](../../../docs/principles/0083-a-refusal-carries-what-the-next-attempt-needs.md)
/// asks for the constraint and not only the fact: which operation, which class
/// it is in, and that the operator can open that class, and where. A model told
/// only *no* reports the instrument as incapable; one told this can hand the
/// person sitting there something to do.
///
/// [`Operation::LoadSet`] names its deck, because its class turns on the deck
/// being live: two identical calls are answered differently a minute apart, and
/// a refusal that only said *closed* would be unfixable by the model that got
/// it. [`Operation::LoadProcedure`] is the same class and names its deck for
/// the same reason.
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

/// Where every operation in the vocabulary stands, per operation and
/// exhaustively.
///
/// No wildcard arm. See the module documentation: a sixty-fifth operation stops
/// the build here until somebody says which class it is in, which is
/// `karakuri_operation_record::written`'s discipline and its reason.
///
/// The split ADR-0235 states is held by a test rather than restated here:
/// `the_classification_is_the_split_adr_0235_states` walks every operation
/// through this match and asserts the closed, open and total counts, so the
/// figures move when this match does and a sentence cannot go stale beside it.
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
        // **A level and not an effect**, which is why it is filed here rather
        // than with the master chain (ADR-0224): one number at the entry to the
        // master chain that blacks out the whole fold.
        Operation::SetMasterOut { .. } => Standing::Closed(Class::MixFaders),

        // ----- The master effects -------------------------------------------
        //
        // Every one acts on the composited frame after the mix has run and
        // immediately. The first three are the chain itself — a slot's values,
        // a slot added and a slot taken out — and an added slot is the one of
        // them that changes what a frame costs. All three are shut against a
        // model until an operator opens them.
        Operation::SetChainParam { .. } => Standing::Closed(Class::MasterEffects),
        Operation::AddChainEffect { .. } => Standing::Closed(Class::MasterEffects),
        Operation::RemoveChainEffect { .. } => Standing::Closed(Class::MasterEffects),
        Operation::SetTonemap { .. } => Standing::Closed(Class::MasterEffects),
        Operation::SetExposure { .. } => Standing::Closed(Class::MasterEffects),

        // ----- The sequencer's lanes ----------------------------------------
        //
        // *"The route that would defeat it is a lane, not a tool."* All six
        // carry `Undecided` today; they are classed now so that the pattern
        // arriving is not also the day the audit acquires a hole.
        //
        // `RemoveLane` is here with the other five and not with the master
        // chain's remove: what it takes out is a row of a pattern, so the route
        // it would open is a lane.
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
        //
        // **Twenty-three rows, and `WriteProcedure` is the case that decides
        // how the classes are drawn.** It rewrites the contents of a deck that
        // is in live mode and it stays open, because the class is not the noun:
        // it is P-0094's question with the noun as its subject, and a procedure
        // rewrite is priced by the check pass, compiled off the frame path,
        // installed at a frame boundary, measured for thirty frames and rolled
        // back to the parked previous version if it costs too much. *"Its worst
        // case is the picture it replaced, coming back."* `WireInput`,
        // `RestoreProcedure`, `KeepCandidate` and `WatchFiles` land through the
        // same rebuild.
        Operation::SetTransition { .. } => Standing::Open,
        Operation::WireInput { .. } => Standing::Open,
        Operation::SaveSet { .. } => Standing::Open,
        // **A library write beside `SaveSet` and for `SaveSet`'s reason.** It
        // puts one node's source in a file beside the library and moves no
        // deck, no fader and no pixel; at its worst, on the frame it goes
        // wrong, what it has done is written a `.kir` under the store.
        Operation::KeepProcedure { .. } => Standing::Open,
        Operation::ListSets { .. } => Standing::Open,
        // **A narrowing of what one bay is drawing.** P-0094's question comes
        // back empty on every count: it changes which rows an operator is
        // looking at and nothing about what any deck is doing, which is
        // `SelectScope`'s answer arrived at from the other side.
        Operation::FilterLibrary { .. } => Standing::Open,
        // **Open where `SelectDeck` is closed, and the difference is what a
        // wrong one costs.** The deck selection is
        // `ClosedUnclassed(Unclassed::Selection)` because it decides where
        // every later keyed operation lands, so a wrong one puts the next
        // press on the wrong deck. A pane's target addresses nothing — every
        // operation the Inspector emits names its deck outright — so a wrong
        // one redraws a pane.
        Operation::PointPane { .. } => Standing::Open,
        Operation::SelectScope { .. } => Standing::Open,
        // **A star changes nothing that is on air.** P-0094's question asked
        // of it comes back empty on every count: at its worst, on the frame it
        // goes wrong, with the operator's attention on the room, it has written
        // one line into a small file beside the library and moved no deck, no
        // fader and no pixel. It is `SelectScope`'s neighbour for the same
        // reason it is on the page — both are about which Sets an operator is
        // looking at, and neither is about what any of them is doing.
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
        // **The nine that arrange the console are the closest call on this
        // side.** Folding away the bay holding the fader an operator is
        // reaching for is a real hazard, and it is P-0094's third answer with
        // the controls intact: it is the largest visible change the panel can
        // make, and it is undone by one key the operator's hand is already
        // near.
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
