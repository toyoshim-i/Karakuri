use super::*;

/// Returns the PRNG seed salt for a given slot index (ADR-0269).
///
/// Ensures each slot runs a distinct simulation of its procedures.
pub(crate) fn slot_salt(slot: usize) -> u32 {
    SEED_SALT + slot as u32
}

/// Computes the base material identifier for a slot from loaded Set ID or launch pair.
pub(crate) fn base_material(set: Option<&str>, launch: &str) -> String {
    set.map(str::to_owned).unwrap_or_else(|| launch.to_owned())
}

/// Formats the composite material display string for a slot with an overlaid procedure (ADR-0338).
pub(crate) fn derived_material(base: &str, procedure: &str) -> String {
    format!("{base} + {procedure}")
}

/// Re-aims `slot` with `procedure` overlaid on its declared layer (ADR-0227, ADR-0338, Principle 0091).
pub(crate) fn overlaying(
    root: &std::path::Path,
    presets: Option<&std::path::Path>,
    slot: usize,
    aim: &mut Aiming,
    name: &str,
) -> Result<String, String> {
    let (source, tier) = kept_source(root, presets, name)?;
    let kind = karakuri_environment::history::declared_kind(&source).ok_or_else(|| {
        format!(
            "`{name}` declares no `kind`, so there is no layer to write it over — a procedure \
             says which layer it implements in a `kind` line, and this one says nothing"
        )
    })?;
    // **Head and rest are one list here**, because *the first node of that kind*
    // is a question about the slot's files in order and the split is only how a
    // `watch::Aim` carries them.
    let mut files: Vec<karakuri_environment::compile::Named> = std::iter::once(aim.at.head.clone())
        .chain(aim.at.rest.iter().cloned())
        .collect();
    let at = files.iter().enumerate().find_map(|(index, file)| {
        let source = std::fs::read(&file.path).ok()?;
        let declared = karakuri_environment::history::declared_kind(&source)
            .unwrap_or(if index == 0 { "L1" } else { "L4" });
        (declared == kind).then_some(index)
    });
    let (at, added) = match at {
        Some(at) => (at, false),
        None => (files.len(), true),
    };
    if added && files.iter().any(|file| file.name.as_deref() == Some(name)) {
        return Err(format!(
            "deck {} already holds a node called `{name}`, and this row would add a second — \
             rename one of them and load again",
            deck_letter(slot as u8)
        ));
    }
    let path = karakuri_environment::scratch::place(
        root,
        &karakuri_environment::scratch::node_name(slot, at, name),
        &String::from_utf8_lossy(&source),
    )?;
    match added {
        // **Named after the row**, because nothing in the slot named it: a node
        // added here is addressed by the name an operator can read off the row
        // they pressed.
        true => files.push(karakuri_environment::compile::Named {
            name: Some(name.to_string()),
            path,
        }),
        // **The node keeps the name the Set gave it**, which is what an `edge`
        // resolves against — see this function's own head.
        false => files[at].path = path,
    }
    let mut files = files.into_iter();
    let head = files
        .next()
        .ok_or_else(|| String::from("this deck is running no files at all"))?;
    let rest: Vec<karakuri_environment::compile::Named> = files.collect();
    // Restate all other fields so the slot preserves its configuration (ADR-0314).
    aim.changed(|at| {
        at.head = head;
        at.rest = rest;
    })
    .map_err(|()| {
        format!(
            "deck {}'s build worker is gone, so `{name}` cannot be built; what is on that deck \
              keeps running",
            deck_letter(slot as u8)
        )
    })?;
    let where_ = match added {
        true => format!("added as {kind}:0, which this deck had no node for"),
        false => format!("written over {kind}:0"),
    };
    Ok(format!(
        "  load: `{name}` ({tier}) {where_} on deck {} — the slot is recompiling on the worker \
         with every other layer where it was, and the staging lane says whether the build \
         landed, was overloaded or did not compile",
        deck_letter(slot as u8)
    ))
}

/// Loads a procedure's source bytes, checking user store then shipped presets (P-0096).
///
/// Returns the bytes and a tier label ("kept" or "shipped").
pub(crate) fn kept_source(
    root: &std::path::Path,
    presets: Option<&std::path::Path>,
    name: &str,
) -> Result<(Vec<u8>, &'static str), String> {
    if let Ok(store) = Store::open(root) {
        if let Ok(source) = store.read_procedure(name) {
            return Ok((source, "kept"));
        }
    }
    let shipped = presets.map(|dir| dir.join(format!("{name}.kir")));
    if let Some(path) = shipped {
        if let Ok(source) = std::fs::read(&path) {
            return Ok((source, "shipped"));
        }
    }
    Err(format!(
        "no procedure named `{name}` — this store's `{}/` does not hold one and the presets root \
         does not ship one, so the row it was listed under has gone since the listing was built",
        Store::PROCEDURES
    ))
}

/// Formats a revision target as a human-readable name or node address for diagnostics.
pub(crate) fn asked_for(revision: &karakuri_operation::Revision) -> String {
    match revision {
        karakuri_operation::Revision::Picked(name) => format!("`{name}`"),
        karakuri_operation::Revision::Previous(node) => format!(
            "the version before {}'s",
            node_addr(ir_layer(node.layer), node.index)
        ),
    }
}

/// Restores a previously snapshotted procedure version to disk for a slot (ADR-0308, ADR-0326, Principle 0083).
pub(crate) fn put_back(
    store: &std::path::Path,
    slot: usize,
    letter: &str,
    id: &str,
    revision: &karakuri_operation::Revision,
    slots: &karakuri_mcp::Slots,
) -> String {
    let asked = asked_for(revision);
    let found = match karakuri_environment::history::list(store, HISTORY_MOST) {
        Ok(found) => found,
        Err(why) => {
            return format!(
                "  put back: the edit history could not be read: {why} — nothing moved, and \
                 what is on deck {letter} is still running"
            );
        }
    };
    // **`Some(id)` and never `None`**, which is [`walked`]'s filter said again
    // where it decides what is written rather than what is drawn: a version
    // filed under no Set is a version of nothing, and landing one because a
    // `None` read as a wildcard would put another run's edit on a deck.
    let of_the_set = || {
        found
            .versions
            .iter()
            .filter(|version| version.set.as_deref() == Some(id))
    };
    let version = match revision {
        karakuri_operation::Revision::Picked(picked) => {
            let Some(version) = of_the_set().find(|version| version_row(version) == *picked) else {
                return format!(
                    "  put back: `{picked}` is not one of `{id}`'s versions in the last \
                     {HISTORY_MOST} this store wrote — a day directory an operator deleted by \
                     hand is the ordinary way that happens, since `rm -rf history/YYYY/MM` is \
                     the whole of the retention policy; nothing moved, and deck {letter} is \
                     still playing `{id}`"
                );
            };
            version
        }
        karakuri_operation::Revision::Previous(node) => {
            let layer = setfile::kind_name(ir_layer(node.layer));
            let addr = node_addr(ir_layer(node.layer), node.index);
            // **Most recent first is `history::list`'s own order**, kept
            // rather than re-sorted here: the newest is what the slot is
            // running and the one after it is the step back.
            let mut chain = of_the_set()
                .filter(|version| version.layer == layer && version.index == node.index as usize);
            let running = chain.next();
            let Some(version) = running.and(chain.next()) else {
                return format!(
                    "  put back: deck {letter} {addr} has {} in `{id}`'s history, so there is \
                     nothing before what it is playing to step back to — the history keeps \
                     every version that compiled, and this node has only ever had the one. \
                     Nothing moved.",
                    match running {
                        Some(_) => "one version",
                        None => "no version",
                    }
                );
            };
            version
        }
    };
    let source = match std::fs::read(&version.file) {
        Ok(source) => source,
        Err(e) => {
            return format!(
                "  put back: {}: {e} — the row is a name and the file behind it is gone, so \
                 nothing moved and deck {letter} is still playing `{id}`",
                version.file.display()
            );
        }
    };
    let target = match slots.file(slot, version.layer, version.index) {
        Ok(path) => path.clone(),
        Err(why) => {
            return format!(
                "  put back: deck {letter} does not hold the node {asked} was a version \
                 of — {why}; `{id}` has been edited since, or this version came off another \
                 slot. Nothing moved."
            );
        }
    };
    match std::fs::write(&target, &source) {
        Ok(()) => format!(
            "  put back: deck {letter} {}:{} <- `{}` -> written into {}; the worker \
             builds it and the budget judges it, and the version it replaces is kept because \
             the rebuild compiles it",
            version.layer,
            version.index,
            version_row(version),
            target.display()
        ),
        Err(e) => format!(
            "  put back: {}: {e} — nothing moved, and what is on deck {letter} is still \
             running",
            target.display()
        ),
    }
}
