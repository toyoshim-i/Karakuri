use super::*;

/// Reads current deck and session state required to complete an operation's persisted record (ADR-0156, ADR-0194, ADR-0201, ADR-0323).
pub(crate) fn reading(
    operation: &Operation,
    deck: &Deck,
    look: &Look,
    chain: &[karakuri_engine::SlotSpec],
    settings: TransitionSettings,
    banks: &karakuri_pattern::Banks,
) -> Current {
    // The whole chain, for whichever slot was asked for: `Record::MasterChain`
    // carries the whole list and a press names one slot of it, so the chain
    // that is running is handed in and `written` takes the slots the press did
    // not name — and answers the position that is not in it.
    let master_chain = match *operation {
        Operation::SetChainParam { .. }
        | Operation::AddChainEffect { .. }
        | Operation::RemoveChainEffect { .. } => Some(mix::current_chain(chain)),
        _ => None,
    };
    let look = match *operation {
        // Supply running look state to complete partial Look records (ADR-0192).
        Operation::SetTonemap { .. } | Operation::SetExposure { .. } => {
            Some(karakuri_operation_record::Look {
                tonemap: mix::tonemap(look.op),
                exposure: look.exposure,
                white_point: look.white_point,
            })
        }
        _ => None,
    };
    // Transport readings provide relative scrub base position and preserve sync/anchor values.
    let transport = match *operation {
        Operation::ScrubDeck { deck: slot, .. } => {
            EngineSlot::new(slot, deck.slot_count()).map(|slot| {
                let transport = deck.transport(slot);
                karakuri_operation_record::Transport {
                    sync: mix::sync(transport.sync()),
                    anchor_bpm: transport.anchor_bpm(),
                    scrub_beats: transport.scrub_beats(),
                }
            })
        }
        _ => None,
    };
    // Anchors the slot to the current session oscillator tempo on sync engagement (P-0092).
    let tempo = match *operation {
        Operation::SetSync { .. } => Some(mix::current_tempo(deck.signals().oscillator())),
        _ => None,
    };
    // Reads mask shape, angle, position, and softness from the target deck (ADR-0334).
    let mask = match *operation {
        Operation::SetMaskShape { deck: slot, .. }
        | Operation::SetMaskPosition { deck: slot, .. }
        | Operation::Wipe { to: slot, .. } => {
            EngineSlot::new(slot, deck.slot_count()).map(|slot| {
                let mask = deck.mask(slot);
                karakuri_operation_record::Mask {
                    kind: wipe_kind(mask.kind()),
                    angle: mask.angle(),
                    position: mask.position(),
                    softness: mask.softness(),
                }
            })
        }
        _ => None,
    };
    // Computes transition quantisation from current oscillator and UI view settings (P-0092).
    let transition = match *operation {
        Operation::FadeDeck { .. }
        | Operation::Crossfade { .. }
        | Operation::SelectRenderer { .. }
        | Operation::Wipe { .. } => Some(mix::current_transition(
            deck.signals().oscillator(),
            settings.quantum,
            settings.length,
            mix::FADE_CURVE,
            mask_kind(settings.kind),
            settings.angle,
        )),
        _ => None,
    };
    // Reads target deck residency and blend mode to preserve non-default settings across wipes (P-0094).
    let mix = match *operation {
        Operation::Wipe { to: slot, .. } => EngineSlot::new(slot, deck.slot_count())
            .map(|slot| mix::current_mix(deck.blend(slot), deck.residency(slot))),
        _ => None,
    };
    // Collect held lanes for armed sequencer pattern (ADR-0323).
    let lanes = Some(karakuri_operation_record::Lanes {
        held: banks
            .pattern()
            .held()
            .map(|(at, target)| (at, target.clone()))
            .collect(),
    });
    Current {
        look,
        master_chain,
        mask,
        transport,
        tempo,
        transition,
        mix,
        lanes,
    }
}
