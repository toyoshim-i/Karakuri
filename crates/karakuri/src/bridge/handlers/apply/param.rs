use super::super::*;

/// Applies parameter-level records that reach inside a Set (Ride, Source, Authority).
pub(crate) fn apply_param_record(record: &Record, deck: &mut Deck) -> Option<String> {
    match *record {
        Record::Ride { .. } => {
            let writes = match karakuri_environment::mix::change(record, deck.slot_count()) {
                Ok(Some(karakuri_environment::mix::Change::Ride { slot, writes })) => {
                    let mut reached = 0;
                    for write in &writes {
                        match deck.write_param(EngineSlot(slot as u8), write) {
                            Ok(n) => reached += n,
                            Err(refused) => return Some(format!("  {refused}")),
                        }
                    }
                    (slot, writes, reached)
                }
                Ok(_) => return None,
                Err(refused) => return Some(format!("  {refused}")),
            };
            let (slot, writes, reached) = writes;
            match reached {
                0 => Some(format!(
                    "  {}",
                    karakuri_environment::no_such_param(slot, &writes[0].key)
                )),
                _ => Some(format!(
                    "  knob: deck {} -> WriteParam -> Record::Ride -> {} on {reached} node(s)",
                    deck_letter(slot as u8),
                    writes
                        .iter()
                        .map(|w| format!("{} = {:.3}", w.key, w.value))
                        .collect::<Vec<_>>()
                        .join(", ")
                )),
            }
        }
        Record::Source { .. } => {
            let (slot, key, bound) =
                match karakuri_environment::mix::change(record, deck.slot_count()) {
                    Ok(Some(karakuri_environment::mix::Change::Source {
                        slot,
                        layer,
                        index,
                        key,
                        binding,
                    })) => match binding {
                        Some(binding) => {
                            let signal = binding.signal.clone();
                            let curve = binding.curve.name();
                            let range = binding.range;
                            match deck.bind(EngineSlot(slot as u8), binding) {
                                karakuri_engine::set::Bound::Yes => (
                                    slot,
                                    key,
                                    format!(
                                        "{signal} through {curve} onto [{:.2}, {:.2}]",
                                        range[0], range[1]
                                    ),
                                ),
                                karakuri_engine::set::Bound::NoSuchParam => {
                                    return Some(format!(
                                        "  {}",
                                        karakuri_environment::no_such_param(slot, &key)
                                    ))
                                }
                                karakuri_engine::set::Bound::NoSuchControl => {
                                    return Some(format!(
                                        "  slot {slot}: `{signal}` is not a control this Set \
                                     publishes"
                                    ))
                                }
                            }
                        }
                        None => match deck.unbind(EngineSlot(slot as u8), layer, index, &key) {
                            true => (slot, key, "nothing — taken back".to_string()),
                            false => {
                                return Some(format!("  slot {slot}: nothing was driving `{key}`"))
                            }
                        },
                    },
                    Ok(_) => return None,
                    Err(refused) => return Some(format!("  {refused}")),
                };
            Some(format!(
                "  source: deck {} -> Record::Source -> `{key}` is driven by {bound}",
                deck_letter(slot as u8)
            ))
        }
        Record::Authority { .. } => {
            let (slot, level) = match karakuri_environment::mix::change(record, deck.slot_count()) {
                Ok(Some(karakuri_environment::mix::Change::Authority {
                    slot,
                    layer,
                    index,
                    authority,
                })) => match deck.set_authority(EngineSlot(slot as u8), layer, index, authority) {
                    true => (slot, authority),
                    false => {
                        return Some(format!(
                            "  slot {slot}: no node {}:{index} to speak for",
                            karakuri_environment::meta::layer_name(
                                karakuri_environment::meta::layer_of(layer),
                            ),
                        ))
                    }
                },
                Ok(_) => return None,
                Err(refused) => return Some(format!("  {refused}")),
            };
            Some(format!(
                "  authority: deck {} -> SetAuthority -> Record::Authority -> {}",
                deck_letter(slot as u8),
                level.name()
            ))
        }
        _ => None,
    }
}
