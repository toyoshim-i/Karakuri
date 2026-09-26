use super::*;

/// Reads metadata declared by a Set's artifacts from store cards (ADR-0156).
///
/// Collects parameter ranges and defaults, element capacity, and emitted attributes.
pub(crate) fn declared(root: &std::path::Path, id: &str) -> Result<Reading, String> {
    let store = Store::open(root).map_err(|e| format!("the store at {}: {e}", root.display()))?;
    let lines = store
        .read_set(id)
        .map_err(|e| format!("reading set `{id}`: {e}"))?;
    let mut reading = Reading {
        id: id.to_owned(),
        ..Reading::default()
    };
    // The key's range so far and the first declarer's default, in the order
    // the keys first appeared — a `Vec` and not a map for `Set::published`'s
    // own reason: the order is what the author wrote, and a hasher's order
    // would move between runs.
    let mut knobs: Vec<(String, [f32; 2], Option<f32>)> = Vec::new();
    let mut capacity: Option<[u32; 3]> = None;
    let mut emits: Vec<String> = Vec::new();
    for line in &lines {
        let Record::Slot { proc_hash, .. } = line.record() else {
            continue;
        };
        reading.nodes += 1;
        // Nodes without metadata cards are still counted in total node count.
        let Ok(card) = store.read_meta(proc_hash) else {
            continue;
        };
        reading.described += 1;
        for entry in &card {
            match entry.record() {
                Record::ParamDecl {
                    key,
                    min,
                    max,
                    default,
                    ..
                } => match knobs.iter_mut().find(|(seen, ..)| seen == key) {
                    Some((_, range, _)) => {
                        range[0] = range[0].max(*min);
                        range[1] = range[1].min(*max);
                    }
                    None => knobs.push((key.clone(), [*min, *max], *default)),
                },
                Record::CapacityDecl { min, max, default } => {
                    capacity = Some(match capacity {
                        None => [*min, *max, *default],
                        Some([lo, hi, was]) => [lo.max(*min), hi.min(*max), was],
                    })
                }
                // **The union, in the order the nodes declare them.** A Set
                // over two geometries emits what both of them do, and the row
                // is *what a renderer drawn over it can consume* rather than
                // any one node's list.
                Record::Emit { attrs } => {
                    for attr in attrs {
                        if !emits.contains(attr) {
                            emits.push(attr.clone());
                        }
                    }
                }
                _ => {}
            }
        }
    }
    reading.knobs = knobs
        .into_iter()
        // **Spelled `view::Published` in full**, because
        // `karakuri_engine::set::Published` is in scope here under the same
        // name and they are the same idea two levels apart: one is what a
        // built Set publishes and this is what a file's cards say it will.
        .map(|(key, [min, max], default)| view::Published {
            key,
            range: spelled(
                &format!("{min}"),
                &format!("{max}"),
                default.map(|d| format!("{d}")),
            ),
        })
        .collect();
    reading.capacity = capacity.map(|[min, max, default]| {
        spelled(
            &format!("{min}"),
            &format!("{max}"),
            Some(format!("{default}")),
        )
    });
    reading.emits = (!emits.is_empty()).then(|| emits.join(", "));
    Ok(reading)
}

/// Formats a knob parameter range and default value for display in the reading pane (ADR-0305).
pub(crate) fn spelled(min: &str, max: &str, default: Option<String>) -> String {
    format!(
        "{min} – {max} · {}",
        default.unwrap_or_else(|| "expr".to_owned())
    )
}

/// Reads the Set metadata for the currently selected library row into the view (ADR-0156, P-0091).
pub(crate) fn read_reading(view: &mut View, store: &std::path::Path) -> String {
    // **The Sets, which is empty under `history`**: a reading is what a Set
    // declares and a row of that scope is a version, so the cursor has no
    // operand there — `view::View::sets`.
    let Some(id) = view.sets().get(view.cursor_row()).cloned() else {
        // Dismiss the reading pane if the selected row is no longer valid.
        view.shut_reading();
        return String::from(match view.scope() {
            Some(scope) if !scope.lists_sets() => {
                "  read: `history` lists the versions of a Set rather than Sets, so there is \
                 nothing under the cursor for a reading to be about"
            }
            _ => "  read: this bay lists nothing, so there is no Set to read",
        });
    };
    match declared(store, &id) {
        Ok(reading) => {
            let line = format!(
                "  read: `{id}` declares {} over {}{}",
                reading.knobs_word(),
                reading.nodes_word(),
                match reading.nodes == reading.described {
                    true => String::new(),
                    false => format!(", {}", reading.cards_word()),
                }
            );
            view.read(reading);
            line
        }
        // **Said out loud and the block put away**, which is [`library`]'s
        // rule one bay up: a reading that failed to read and a Set that
        // declares nothing must not draw the same.
        Err(e) => {
            view.shut_reading();
            format!("  read: `{id}` could not be read — {e}")
        }
    }
}

/// Updates the active Set reading when the cursor row moves in the Library bay (ADR-0265).
pub(crate) fn reread_if_open(
    moved: bool,
    view: &mut View,
    store: &std::path::Path,
) -> Option<String> {
    (moved && view.reading_open()).then(|| read_reading(view, store))
}

/// Maps an operation layer to its corresponding store record layer representation.
pub(crate) fn asked(layer: karakuri_store::record::Layer) -> karakuri_operation::Layer {
    use karakuri_operation::Layer as Asked;
    use karakuri_store::record::Layer as Written;
    match layer {
        Written::L1 => Asked::L1,
        Written::L2 => Asked::L2,
        Written::L3 => Asked::L3,
        Written::L4 => Asked::L4,
        Written::Field => Asked::Field,
        Written::L5 => Asked::L5,
    }
}

/// Parses a declared kind string into a vocabulary `Layer`.
pub(crate) fn kind_of(word: &str) -> Option<karakuri_operation::Layer> {
    use karakuri_operation::Layer;
    match word {
        "L1" => Some(Layer::L1),
        "L2" => Some(Layer::L2),
        "L3" => Some(Layer::L3),
        "L4" => Some(Layer::L4),
        "Field" => Some(Layer::Field),
        "L5" => Some(Layer::L5),
        _ => None,
    }
}

/// Formats active kind filter chips as a human-readable description for UI display.
pub(crate) fn showing(kinds: karakuri_operation::LibraryKinds) -> String {
    if !kinds.narrowing() {
        return String::from("every kind");
    }
    let on: Vec<&str> = karakuri_console::view::KindChip::ALL
        .into_iter()
        .filter(|chip| chip.on(kinds))
        .map(|chip| chip.word())
        .collect();
    on.join(", ")
}

pub(crate) fn recorded(layer: karakuri_operation::Layer) -> karakuri_store::record::Layer {
    use karakuri_operation::Layer as Asked;
    use karakuri_store::record::Layer as Written;
    match layer {
        Asked::L1 => Written::L1,
        Asked::L2 => Written::L2,
        Asked::L3 => Written::L3,
        Asked::L4 => Written::L4,
        Asked::Field => Written::Field,
        Asked::L5 => Written::L5,
    }
}
