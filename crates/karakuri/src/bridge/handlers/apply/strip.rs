use super::super::*;

/// Applies strip and slot-level records to the deck.
pub(crate) fn apply_strip_record(record: &Record, deck: &mut Deck) -> Option<String> {
    match *record {
        Record::Gain { slot, value } => {
            let slot = held(deck, slot.0)?;
            deck.set_gain(slot, value);
            Some(format!(
                "  fader: deck {} trim -> SetGain {{ deck: {slot}, gain: {value:.3} }} \
                 -> Record::Gain -> deck.gain({slot}) = {:.3}",
                deck_letter(slot.0),
                deck.gain(slot)
            ))
        }
        Record::Opacity { slot, value } => {
            let slot = held(deck, slot.0)?;
            deck.set_opacity(slot, value);
            Some(format!(
                "  fader: deck {} fader -> SetOpacity {{ deck: {slot}, opacity: {value:.3} }} \
                 -> Record::Opacity -> deck.opacity({slot}) = {:.3}",
                deck_letter(slot.0),
                deck.opacity(slot)
            ))
        }
        Record::Mute { slot, muted } => {
            let slot = held(deck, slot.0)?;
            deck.set_mute(slot, muted);
            Some(format!(
                "  mute: deck {} -> SetMute {{ deck: {slot}, mute: {muted} }} \
                 -> Record::Mute -> deck.is_muted({slot}) = {}",
                deck_letter(slot.0),
                deck.is_muted(slot)
            ))
        }
        Record::Solo { slot, soloed } => {
            let slot = held(deck, slot.0)?;
            deck.set_solo(slot, soloed);
            Some(format!(
                "  solo: deck {} -> SetSolo {{ deck: {slot}, solo: {soloed} }} \
                 -> Record::Solo -> deck.is_soloed({slot}) = {}",
                deck_letter(slot.0),
                deck.is_soloed(slot)
            ))
        }
        Record::Online { slot, online } => {
            let slot = held(deck, slot.0)?;
            deck.set_online(slot, online);
            Some(format!(
                "  online: deck {} -> SetOnline {{ deck: {slot}, online: {online} }} \
                 -> Record::Online -> deck.is_online({slot}) = {}",
                deck_letter(slot.0),
                deck.is_online(slot)
            ))
        }
        Record::Blend { slot, ref mode } => {
            let slot = held(deck, slot.0)?;
            let blend = Blend::from_name(mode)?;
            deck.set_blend(slot, blend);
            Some(format!(
                "  blend: deck {} -> SetBlendMode {{ deck: {slot}, blend: {mode} }} \
                 -> Record::Blend -> deck.blend({slot}) = {}",
                deck_letter(slot.0),
                deck.blend(slot).name()
            ))
        }
        Record::Residency { slot, ref level } => {
            let slot = held(deck, slot.0)?;
            let residency = mix::parse_residency(level)?;
            deck.set_residency(slot, residency);
            deck.govern();
            Some(format!(
                "  tally: deck {} -> SetResidency {{ deck: {slot}, residency: {level} }} \
                 -> Record::Residency -> deck.requested_residency({slot}) = {:?}, \
                 deck.residency({slot}) = {:?}",
                deck_letter(slot.0),
                deck.requested_residency(slot),
                deck.residency(slot)
            ))
        }
        Record::Policy { slot, ref policy } => {
            let slot = held(deck, slot.0)?;
            Some(format!(
                "  policy: deck {} -> Record::Policy {{ policy: {policy} }}",
                deck_letter(slot.0)
            ))
        }
        Record::Mask {
            slot,
            ref kind,
            angle,
            position,
            softness,
        } => {
            let slot = held(deck, slot.0)?;
            let shape = MaskKind::from_name(kind)?;
            deck.set_mask(slot, Mask::new(shape, angle, position, softness));
            Some(format!(
                "  mask: deck {} -> SetMaskShape {{ deck: {slot}, kind: {kind}, \
                 angle: {angle:.3} }} -> Record::Mask -> deck.mask({slot}) = {} \
                 at {:.3} rad, front at {:.3}",
                deck_letter(slot.0),
                deck.mask(slot).kind().name(),
                deck.mask(slot).angle(),
                deck.mask(slot).position()
            ))
        }
        Record::Transition {
            slot,
            ref control,
            to,
            start,
            beats,
            ref curve,
        } => {
            let slot = held(deck, slot.0)?;
            let control = Control::from_name(control)?;
            let curve = karakuri_engine::binding::Curve::parse(curve)?;
            let from = match control {
                Control::Gain => deck.gain(slot),
                Control::Opacity => deck.opacity(slot),
                Control::MaskPosition => deck.mask(slot).position(),
            };
            deck.schedule(karakuri_engine::Transition::new(
                slot.index(),
                control,
                from,
                to,
                start,
                beats,
                curve,
            ));
            Some(format!(
                "  transition: deck {} {} {from:.3} -> {to:.3} -> Record::Transition {{ \
                 start: {start:.3}, beats: {beats:.3}, curve: {} }} -> \
                 deck.transitions_on({slot}) = {}",
                deck_letter(slot.0),
                control.name(),
                curve.name(),
                deck.transitions_on(slot).count()
            ))
        }
        _ => None,
    }
}
