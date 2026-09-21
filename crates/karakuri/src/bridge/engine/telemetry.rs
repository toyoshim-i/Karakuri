use super::*;

/// Is anything making texels this frame?
pub(crate) fn live(view: &View) -> bool {
    view.picture.is_some() || view.previews.iter().any(Option::is_some) || view.projector
}

/// What the transport row reads this frame, from the deck's oscillator and frame costs.
pub(crate) fn transport(
    deck: &Deck,
    costs: &Costs,
    budget_ms: Option<f32>,
    live: bool,
    health: Option<view::Stage>,
    rec: view::Rec,
) -> Option<view::Transport> {
    let last = costs.last?;
    let grid = deck.signals().oscillator();
    Some(view::Transport {
        bpm: grid.bpm(),
        beats: grid.beats(),
        beats_per_bar: karakuri_signal::oscillator::BEATS_PER_BAR,
        fps: live.then(|| costs.rate()).flatten().map(|rate| rate as f32),
        frame_ms: ms(last.whole()) as f32,
        budget_ms,
        chain_ms: Some(deck.chain_ms()).filter(|ms| *ms > 0.0),
        health,
        rec: Some(rec),
    })
}

/// Elements per geometry from the L1 declaration.
pub(crate) fn capacity_of(l1: &karakuri_ir::typed::Checked) -> u32 {
    l1.capacity
        .map_or(karakuri_ir::DEFAULT_CAPACITY, |declared| declared.default)
}
