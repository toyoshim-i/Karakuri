use super::*;

/// Returns the target value for a control undergoing a scheduled transition on the given slot.
pub(crate) fn destination(deck: &Deck, slot: EngineSlot, control: Control) -> Option<f32> {
    deck.transitions_on(slot)
        .find(|t| t.control() == control)
        .map(|t| t.to())
}

/// Extracts budget decision timing and basis for each deck preview cell from the report (ADR-0296, ADR-0298).
pub(crate) fn costs(governed: &Report) -> [Option<Budgeted>; DECKS] {
    let mut out = [None; DECKS];
    for decision in &governed.decisions {
        let Some(cell) = out.get_mut(decision.slot) else {
            continue;
        };
        *cell = match (decision.budgeted_ms, decision.basis) {
            (Some(ms), Spent::Estimated) => Some(Budgeted {
                ms,
                basis: Basis::Estimated,
            }),
            (Some(ms), Spent::Measured) => Some(Budgeted {
                ms,
                basis: Basis::Measured,
            }),
            // `Unbudgetable`, and a number arriving without a basis — which
            // cannot happen, and is not worth inventing a band for if it does.
            _ => None,
        };
    }
    out
}

/// Updates view strip states for all slots from current engine deck readings (ADR-0156, ADR-0164, ADR-0187, ADR-0203).
pub(crate) fn mixer(deck: &Deck, names: &[String], out: &mut Vec<view::Strip>) {
    out.truncate(deck.slot_count());
    while out.len() < deck.slot_count() {
        out.push(view::Strip {
            name: String::new(),
            tally: view::Tally::Allocated,
            requested: view::Tally::Allocated,
            gain: 0.0,
            gain_to: None,
            opacity: 0.0,
            opacity_to: None,
            blend: BlendMode::Add,
            mask: view::Mask::None,
            mask_angle: 0.0,
            level: None,
            is_muted: false,
            is_soloed: false,
        });
    }
    for (slot, strip) in out.iter_mut().enumerate() {
        // Preserve per-slot material name if assigned, or leave empty (P-0094).
        let name = names.get(slot).map_or("", String::as_str);
        if strip.name != name {
            strip.name.clear();
            strip.name.push_str(name);
        }
        let slot = EngineSlot(slot as u8);
        strip.tally = tally(deck.residency(slot));
        strip.requested = tally(deck.requested_residency(slot));
        strip.gain = deck.gain(slot);
        strip.opacity = deck.opacity(slot);
        strip.is_muted = deck.is_muted(slot);
        strip.is_soloed = deck.is_soloed(slot);
        // Query target destinations for any active scheduled transitions on gain and opacity.
        strip.gain_to = destination(deck, slot, Control::Gain);
        strip.opacity_to = destination(deck, slot, Control::Opacity);
        strip.blend = blend_mode(deck.blend(slot));
        strip.mask = masked(deck.mask(slot).kind());
        // Preserve current mask angle on the strip state for subsequent mask shape changes (ADR-0203).
        strip.mask_angle = deck.mask(slot).angle();
        strip.level = deck.level(slot).map(|level| view::Level {
            mean: level.mean,
            peak: level.peak,
        });
    }
}

/// Reports which slots are stopped due to watchdog overloads (ADR-0269, ADR-0316).
pub(crate) fn stopped_slots(deck: &Deck) -> [bool; view::DECKS] {
    std::array::from_fn(|slot| slot < deck.slot_count() && deck.overloaded(EngineSlot(slot as u8)))
}

/// Converts engine residency state to view tally representation.
pub(crate) fn tally(residency: Residency) -> view::Tally {
    match residency {
        Residency::Live => view::Tally::Live,
        Residency::Priming => view::Tally::Priming,
        Residency::Allocated => view::Tally::Allocated,
    }
}

/// Converts an engine blend mode to vocabulary blend mode (P-0090, ADR-0156, ADR-0194).
pub(crate) fn blend_mode(blend: Blend) -> BlendMode {
    match blend {
        Blend::Add => BlendMode::Add,
        Blend::Over => BlendMode::Over,
        Blend::Max => BlendMode::Max,
    }
}

// Blend cycle order is defined in `karakuri_console::view::after` (ADR-0187, ADR-0333).

/// Reads live deck mixer parameters (residency, blend mode, and mask settings) for a given slot index.
pub(crate) fn holding(deck: &Deck, at: u8) -> Option<focus::Held> {
    let slot = held(deck, at)?;
    Some(focus::Held {
        // The residency the deck was last requested for.
        requested: tally(deck.requested_residency(slot)),
        blend: blend_mode(deck.blend(slot)),
        mask: masked(deck.mask(slot).kind()),
        mask_angle: deck.mask(slot).angle(),
    })
}

/// The engine's mask shape as the console's, and the mirror image of
/// `karakuri_console::view::wipe_kind` on the way back out.
pub(crate) fn masked(kind: MaskKind) -> view::Mask {
    match kind {
        MaskKind::None => view::Mask::None,
        MaskKind::Linear => view::Mask::Linear,
        MaskKind::Radial => view::Mask::Radial,
    }
}

/// Linear delta (0.1) applied per keyboard step to slot trim gain (ADR-0214).
pub(crate) const GAIN_STEP: f32 = 0.1;

/// One press of an opacity key, and [`GAIN_STEP`]'s sentence one control along:
/// `karakuri-cli`'s `OPACITY_STEP`, which is the same tenth, and the console's
/// page is silent about this one too.
pub(crate) const OPACITY_STEP: f32 = 0.1;

/// Calculates the adjusted gain value for a step input, floored at 0.0 with default at 1.0 (P-0064, ADR-0259).
pub(crate) fn gain_key(step: Step, from: f32) -> f32 {
    let asked = match step {
        Step::Down => from - GAIN_STEP,
        Step::Up => from + GAIN_STEP,
        Step::Default => 1.0,
    };
    asked.max(0.0)
}

/// Adjusts slot opacity value for step input, clamped to `[0.0, 1.0]` (ADR-0259).
pub(crate) fn opacity_key(step: Step, from: f32) -> f32 {
    let asked = match step {
        Step::Down => from - OPACITY_STEP,
        Step::Up => from + OPACITY_STEP,
        Step::Default => 1.0,
    };
    asked.clamp(0.0, 1.0)
}

/// Adjusts master output level for step input, clamped to `[0.0, 1.0]` with default 1.0.
pub(crate) fn out_key(step: Step, from: f32) -> f32 {
    let asked = match step {
        Step::Down => from - OPACITY_STEP,
        Step::Up => from + OPACITY_STEP,
        Step::Default => 1.0,
    };
    asked.clamp(0.0, 1.0)
}

/// Validates that a requested slot index exists on the deck, returning typed `EngineSlot`.
pub(crate) fn held(deck: &Deck, slot: u8) -> Option<EngineSlot> {
    EngineSlot::new(slot, deck.slot_count())
}
