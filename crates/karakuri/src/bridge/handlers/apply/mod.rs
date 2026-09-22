use super::*;

mod engine;
mod feedback;
mod param;
mod procedure;
mod reading;
mod strip;

pub(crate) use engine::apply_engine_record;
pub(crate) use feedback::{refusal, unperformed, unwritten};
pub(crate) use param::apply_param_record;
pub(crate) use procedure::{
    asked_for, base_material, derived_material, overlaying, put_back, slot_salt,
};
pub(crate) use reading::reading;
pub(crate) use strip::apply_strip_record;

/// A press on a strip or a deck key, applied to the console's own pointer, and
/// what to say about it. `None` for every operation that is not it.
///
/// `Operation::SelectDeck` *"writes no record, and is the reason every other
/// variant names its deck instead of meaning the selected one"*, so there is
/// nothing on the deck for [`apply`] to move and the surface that emits it is
/// what performs it (ADR-0198). This is that performance, and it is one line
/// beside [`arrangement`]'s for the same reason: the alternative is a second
/// route into the view.
///
/// A deck the mixer has no strip for is refused, and `View::select` is where
/// that rule lives — the ring would be drawn nowhere and the library's pill
/// would name a deck a load could not reach. It is said here rather than
/// swallowed, because a key that does nothing and a key that is not bound are
/// the same experience.
pub(crate) fn pointed(view: &mut View, operation: &Operation) -> Option<String> {
    let Operation::SelectDeck { deck } = *operation else {
        return None;
    };
    let letter = deck_letter(deck);
    if usize::from(deck) >= view.mixer.len() {
        return Some(format!(
            "  select: deck {letter} refused — this deck has {} slot{}, and a selection with no \
             strip under it is a ring drawn nowhere and a `load` pill naming a deck the press \
             could not reach",
            view.mixer.len(),
            match view.mixer.len() {
                1 => "",
                _ => "s",
            }
        ));
    }
    view.select(deck);
    Some(format!(
        "  select: deck {letter} -> SelectDeck {{ deck: {deck} }} -> no record, and that is \
         settled: it is a surface's own pointer. The keys are addressed here, and the library's \
         foot reads `load -> {letter}`"
    ))
}

/// Applies inspector pane target selection to UI view state (ADR-0338, Principle 0083).
pub(crate) fn pointed_pane(view: &mut View, operation: &Operation) -> Option<String> {
    let Operation::PointPane { pane, deck } = operation else {
        return None;
    };
    let Some(at) = view::PANE_NAMES.iter().position(|name| name == pane) else {
        return Some(format!(
            "  pane: `{pane}` refused — this console's panes are {}",
            view::PANE_NAMES.join(" and ")
        ));
    };
    let letter = deck_letter(*deck);
    if !view.point_pane(at, *deck) && usize::from(*deck) >= view.mixer.len() {
        return Some(format!(
            "  pane: `{pane}` -> deck {letter} refused — this deck has {} slot{}, and a pane \
             pointed at one it has not got is a head naming a deck with nothing under it",
            view.mixer.len(),
            match view.mixer.len() {
                1 => "",
                _ => "s",
            }
        ));
    }
    Some(format!(
        "  pane: `{pane}` -> deck {letter} -> no record, and that is settled: a pane's target is \
         a surface's own pointer. The keys stay where they are and the pane next door does not \
         move"
    ))
}

/// Applies transition configuration updates to the UI view state.
pub(crate) fn scheduled(view: &mut View, operation: &Operation) -> Option<String> {
    let Operation::SetTransition { setting } = operation else {
        return None;
    };
    if !view.set_transition(*setting) {
        let at = view.transition();
        return Some(format!(
            "  transition: {setting:?} refused or already there — the row is on `{}`, `{}`, \
             `{}`, and a pill draws only what its own cycle names",
            at.shape_word(),
            at.quantum_word(),
            at.length_word()
        ));
    }
    let at = view.transition();
    Some(format!(
        "  transition: {setting:?} -> no record, and that is settled: it is a surface's own \
         setting. The next fade, crossfade or wipe is `{}` on the `{}`, over {} beat{}",
        at.shape_word(),
        at.quantum_word(),
        at.length,
        match at.length == 1.0 {
            true => "",
            false => "s",
        }
    ))
}

/// A candidate kept, applied to the lane, and what to say about it. `None` for
/// every operation that is not it.
///
/// [`scheduled`]'s shape one bay over, and for its reason: `written` answers
/// `Silent(Silent::Surface)` for `Operation::KeepCandidate`, so there is no
/// record for [`apply`] to move a deck with and the surface that draws the row
/// is what performs the press. What it changes is one line in one list.
///
/// Nothing else moves, and that is the operation rather than a shortfall. The
/// version is where it was, the store holds every version it held, and the
/// picture is the picture. What a keep says is that a person has looked at this
/// node and is done with it — `console.html`'s *Accepting settles the node and
/// changes nothing on screen*.
///
/// A row that is not there is said rather than swallowed, which is
/// [`pointed`]'s rule: nothing this window emits can reach it — the control is
/// the row and a row that is not drawn takes no press — so a line here is a
/// mapped controller or an MCP call arriving at a node with no candidate on it,
/// the day either reaches this row.
pub(crate) fn kept(view: &mut View, operation: &Operation) -> Option<String> {
    let Operation::KeepCandidate { deck, node } = operation else {
        return None;
    };
    let slot = usize::from(*deck);
    let addr = node_addr(ir_layer(node.layer), node.index);
    let before = view.staging.len();
    view.staging
        .retain(|row| row.deck != slot || row.at != Some(*node));
    let letter = deck_letter(*deck);
    if view.staging.len() == before {
        return Some(format!(
            "  keep: deck {letter} {addr} has no candidate row — nothing was outstanding on \
             that node, and the lane is as it was"
        ));
    }
    Some(format!(
        "  keep: deck {letter} {addr} -> KeepCandidate -> no record, and that is settled: the \
         material already changed and its `procedure` record was written at the swap. The row \
         leaves the lane and nothing else moves; {} still waiting",
        match view.staging.len() {
            0 => "nothing".to_owned(),
            n => format!(
                "{n} row{}",
                match n {
                    1 => "",
                    _ => "s",
                }
            ),
        }
    ))
}

/// The record, applied to the deck, and what to say about it.
///
/// This is not the half ADR-0185 promised to delete, and it did not go with it.
/// Turning an `Operation` into a `Record` was the shortcut — that function is
/// gone and [`written`] answers instead
/// ([ADR-0194](../../../docs/adr/0194-where-an-operation-becomes-a-record-is-a-crate-that-depends-on-both.md)).
/// Turning a record into a *deck movement* is a different job and is the
/// harness's by design: `karakuri-operation-record` has no engine and never
/// will, so somebody who owns a deck has to decode.
///
/// `karakuri-cli`'s `mix::change` is the real decoder and it does two things
/// this does not: it refuses a slot the deck has not got, with the same
/// sentence every other surface refuses one with, and it turns a record into a
/// `Change` that a caller applies. This is the shortest path from the records
/// [`written`] answers with to the setters they name.
///
/// The line it returns is the loop closing, printed so that it can be read
/// rather than inferred: the operation, the record, and what the deck says
/// afterwards — which is where the next frame's strip comes from.
///
/// # It takes the look as well as the deck, and that is not a second target
///
/// `Record::Look` is the one record here that does not name a slot: the look is
/// what *every* sink is drawn under, so it is `Engine::look` rather than
/// anything on the deck ([`Engine::look`], and `karakuri_engine::frame::Look`
/// for why the master out is deliberately not in it). Handing both in is what
/// keeps this one function the only place a record becomes a movement — a
/// second `apply_look` beside it would be the second route into the engine that
/// P-0090 exists to refuse.
pub(crate) fn apply(
    record: &Record,
    deck: &mut Deck,
    look: &mut Look,
    chain: &mut Vec<karakuri_engine::SlotSpec>,
) -> Option<String> {
    apply_strip_record(record, deck)
        .or_else(|| apply_param_record(record, deck))
        .or_else(|| apply_engine_record(record, deck, look, chain))
}
