use super::super::*;

/// Applies engine, master out/chain, transport, look, and tempo records.
pub(crate) fn apply_engine_record(
    record: &Record,
    deck: &mut Deck,
    look: &mut Look,
    chain: &mut Vec<karakuri_engine::SlotSpec>,
) -> Option<String> {
    match *record {
        Record::Look {
            ref op,
            exposure,
            white_point,
        } => {
            let op = mix::parse_op(op)?;
            *look = Look {
                op,
                exposure,
                white_point,
            };
            Some(format!(
                "  look: -> Record::Look {{ op: {op_name}, exposure: {exposure:.3}, \
                 white_point: {white_point:.3} }} -> every sink is drawn under {op_name} at \
                 exposure {exposure:.3}",
                op_name = mix::op_wire_name(op)
            ))
        }
        Record::MasterOut { value } => {
            deck.set_out(value);
            Some(format!(
                "  master: out -> SetMasterOut {{ out: {value:.3} }} -> Record::MasterOut -> \
                 deck.out() = {:.3}, at the entry to the master chain",
                deck.out()
            ))
        }
        Record::MasterChain(ref want) => {
            let mut slots = Vec::with_capacity(want.slots.len());
            for slot in &want.slots {
                slots.push(karakuri_engine::SlotSpec {
                    procedure: slot.procedure.clone(),
                    cut: match &slot.cut {
                        None => None,
                        Some(word) => Some(Cut::parse(word)?),
                    },
                    params: slot.params.clone(),
                });
            }
            let said = format!(
                "  master: chain -> Record::MasterChain -> {} slot{} — {}",
                slots.len(),
                if slots.len() == 1 { "" } else { "s" },
                if slots.is_empty() {
                    "the frame is the mix".to_string()
                } else {
                    slots
                        .iter()
                        .map(|s| {
                            let cut = s
                                .cut
                                .map(|c| format!(" ({})", c.name()))
                                .unwrap_or_default();
                            format!("{}{cut}", &s.procedure[..s.procedure.len().min(14)])
                        })
                        .collect::<Vec<_>>()
                        .join(", ")
                }
            );
            *chain = slots;
            Some(said)
        }
        Record::Transport {
            slot,
            ref sync,
            anchor_bpm,
            scrub_beats,
        } => {
            let slot = held(deck, slot.0)?;
            let mode = EngineSync::from_name(sync)?;
            deck.set_transport(slot, mode, anchor_bpm, scrub_beats)
                .ok()?;
            Some(format!(
                "  scrub: deck {} -> ScrubDeck {{ deck: {slot} }} -> Record::Transport {{ \
                 sync: {sync}, anchor_bpm: {anchor_bpm:.1}, scrub_beats: {scrub_beats:+.2} }} \
                 -> deck.transport({slot}).scrub_beats() = {:+.2} beats",
                deck_letter(slot.0),
                deck.transport(slot).scrub_beats()
            ))
        }
        Record::Select {
            slot,
            renderer,
            start,
        } => {
            let slot = held(deck, slot.0)?;
            deck.schedule_selection(Selection::new(slot.index(), renderer as usize, start));
            Some(format!(
                "  renderer: deck {} -> SelectRenderer {{ renderer: {renderer} }} -> \
                 Record::Select {{ start: {start:.3} }} -> deck.selections_on({slot}) = {} \
                 armed, landing on the beat grid",
                deck_letter(slot.0),
                deck.selections_on(slot).count()
            ))
        }
        Record::Tempo { bpm, shift, .. } => {
            let mut signals = *deck.signals();
            karakuri_environment::audio::apply_tempo(&mut signals, record);
            deck.set_signals(signals);
            Some(format!(
                "  tempo: -> Record::Tempo {{ bpm: {bpm:.1}, shift: {shift:+.3} }} -> \
                 deck.signals().oscillator().bpm() = {:.1}, from now on and without moving a \
                 beat that has already happened",
                deck.signals().oscillator().bpm()
            ))
        }
        _ => None,
    }
}
