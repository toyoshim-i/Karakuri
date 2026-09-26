use super::*;

/// Lists procedures in the store, newest first, reading their declared kinds (ADR-0156, P-0091).
///
/// Returns an empty list if the store root does not exist or fails to open.
pub(crate) fn procedures(root: &std::path::Path) -> Vec<ListedProcedure> {
    if !root.is_dir() {
        // Said once by `library` beside it on the same press, so this one is
        // quiet: two lines about one missing store would be one fact said
        // twice.
        return Vec::new();
    }
    let store = match Store::open(root) {
        Ok(store) => store,
        Err(e) => {
            println!("library: {} could not be opened: {e}", root.display());
            return Vec::new();
        }
    };
    let listed = match store.list_procedures() {
        Ok(listed) => listed,
        Err(e) => {
            // Said rather than swallowed, for [`library`]'s reason: a tier
            // that is empty because a directory could not be read looks
            // exactly like a tier nobody has kept into.
            println!(
                "library: {}/{} could not be listed: {e}",
                root.display(),
                Store::PROCEDURES
            );
            return Vec::new();
        }
    };
    let mut out: Vec<ListedProcedure> = listed
        .into_iter()
        .filter_map(|entry| {
            // Reads declared kind badge from source without compiling (ADR-0338, P-0091).
            let source = store.read_procedure(&entry.name).ok()?;
            Some(ListedProcedure {
                name: entry.name,
                written: entry.written,
                kind: karakuri_environment::history::declared_kind(&source).and_then(kind_of),
            })
        })
        .collect();
    out.sort_by(|a, b| b.written.cmp(&a.written).then_with(|| a.name.cmp(&b.name)));
    out
}

/// Returns `kind L5` procedure choices for the Master bay's `+ add` list (ADR-0340, P-0091).
///
/// Combines user-kept procedures followed by shipped presets, skipping unreadable files.
fn chain_offers(
    store: &std::path::Path,
    kept: &[ListedProcedure],
    shipped: &[karakuri_environment::places::PresetProcedure],
) -> Vec<view::AddChoice> {
    let l5 = |kind: Option<karakuri_operation::Layer>| kind == Some(karakuri_operation::Layer::L5);
    let opened = Store::open(store).ok();
    let mut out: Vec<view::AddChoice> = Vec::new();
    for entry in kept.iter().filter(|entry| l5(entry.kind)) {
        let Some(source) = opened
            .as_ref()
            .and_then(|store| store.read_procedure(&entry.name).ok())
            .and_then(|bytes| String::from_utf8(bytes).ok())
        else {
            continue;
        };
        if let Some((procedure, retains)) = karakuri_environment::mix::l5_offer(&source) {
            out.push(view::AddChoice {
                procedure,
                words: entry.name.clone(),
                retains,
            });
        }
    }
    for entry in shipped
        .iter()
        .filter(|entry| l5(entry.kind.and_then(kind_of)))
    {
        let Ok(source) = std::fs::read_to_string(&entry.file) else {
            continue;
        };
        if let Some((procedure, retains)) = karakuri_environment::mix::l5_offer(&source) {
            out.push(view::AddChoice {
                procedure,
                words: entry.name.clone(),
                retains,
            });
        }
    }
    out
}

/// Returns the distinct layer badges filled by a Set's nodes in declaration order (ADR-0338, P-0091).
pub(crate) fn set_row(set: &setfile::SetSummary) -> RowKind {
    let mut badges: Vec<karakuri_operation::Layer> = Vec::new();
    for node in &set.nodes {
        let layer = asked(node.layer);
        if !badges.contains(&layer) {
            badges.push(layer);
        }
    }
    RowKind {
        badges,
        procedure: false,
    }
}

/// What a kept procedure's row is: its one declared kind, and that it is a
/// procedure. A file that declares none draws no badge.
pub(crate) fn kept_row(kept: &ListedProcedure) -> RowKind {
    RowKind {
        badges: kept.kind.into_iter().collect(),
        procedure: true,
    }
}

/// [`kept_row`] one tier along, for a procedure that ships.
pub(crate) fn shipped_row(shipped: &karakuri_environment::places::PresetProcedure) -> RowKind {
    RowKind {
        badges: shipped.kind.and_then(kind_of).into_iter().collect(),
        procedure: true,
    }
}

/// Determines if a procedure matches the kind filter. Unkinded procedures match only when no filter is active.
pub(crate) fn shows_kept(
    kinds: karakuri_operation::LibraryKinds,
    kind: Option<karakuri_operation::Layer>,
) -> bool {
    match kind {
        Some(layer) => kinds.shows_layer(layer),
        None => !kinds.narrowing(),
    }
}

/// What the presets root ships that can go over a layer, as its rows —
/// [`presets_listing`]'s shape one extension along and with its failures.
pub(crate) fn presets_procedures(
    presets: Option<&karakuri_environment::places::Presets>,
) -> Vec<karakuri_environment::places::PresetProcedure> {
    let Some(presets) = presets else {
        return Vec::new();
    };
    match presets.list_procedures() {
        Ok(procedures) => procedures,
        // **Said out loud and then empty**, which is [`presets_listing`]'s own
        // answer beside it: a tier that is empty because a directory could not
        // be read looks exactly like one that ships nothing.
        Err(why) => {
            println!("library: {why}");
            Vec::new()
        }
    }
}

/// A user-kept procedure row with display name, timestamp, and optional declared layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ListedProcedure {
    pub(crate) name: String,
    pub(crate) written: std::time::SystemTime,
    pub(crate) kind: Option<karakuri_operation::Layer>,
}

pub(crate) fn library(root: &std::path::Path) -> Vec<setfile::SetSummary> {
    if !root.is_dir() {
        println!(
            "library: no store at {}, so the bay lists nothing",
            root.display()
        );
        return Vec::new();
    }
    let sets = Store::open(root).and_then(|store| setfile::summarise(&store));
    match sets {
        Ok(mut sets) => {
            sets.sort_by(|a, b| b.written.cmp(&a.written).then_with(|| a.id.cmp(&b.id)));
            sets
        }
        Err(e) => {
            // Said rather than swallowed, for the reason every other failure
            // in this file is said: a bay that is empty because the store
            // could not be read looks exactly like a bay that is empty
            // because the store is.
            println!("library: {} could not be listed: {e}", root.display());
            Vec::new()
        }
    }
}

/// Returns the set of starred Set IDs recorded in `<store>/favourites.json` (ADR-0299).
pub(crate) fn favourites(root: &std::path::Path) -> std::collections::BTreeSet<String> {
    if !root.is_dir() {
        return std::collections::BTreeSet::new();
    }
    match Store::open(root).and_then(|store| store.favourites()) {
        Ok(starred) => starred,
        Err(e) => {
            println!(
                "library: {} keeps no readable favourites: {e}",
                root.display()
            );
            std::collections::BTreeSet::new()
        }
    }
}

/// Checks if a Set matches both the case-insensitive node name substring and layer filter.
pub(crate) fn narrows(
    set: &setfile::SetSummary,
    holds: Option<&str>,
    layer: Option<karakuri_operation::Layer>,
) -> bool {
    let held = holds.map(str::to_ascii_lowercase);
    held.as_ref().is_none_or(|holds| {
        set.nodes
            .iter()
            .any(|node| node.name.to_ascii_lowercase().contains(holds))
    }) && layer.is_none_or(|layer| {
        let want = recorded(layer);
        set.nodes.iter().any(|node| node.layer == want)
    })
}

/// Returns sorted, deduplicated node names across all sets for cycling the `holds` filter.
pub(crate) fn holds_choices(sets: &[setfile::SetSummary]) -> Vec<String> {
    sets.iter()
        .flat_map(|set| set.nodes.iter().map(|node| node.name.clone()))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect()
}

/// Human-readable explanation of active filter criteria, or `None` if unconstrained.
pub(crate) fn narrowing(holds: Option<&str>, layer: Option<&str>) -> Option<String> {
    match (holds, layer) {
        (None, None) => None,
        (Some(holds), None) => Some(format!("holding a node called `{holds}`")),
        (None, Some(layer)) => Some(format!("holding a {layer} node")),
        (Some(holds), Some(layer)) => Some(format!(
            "holding both a node called `{holds}` and a {layer} node"
        )),
    }
}

/// Populates the Library bay view rows for the currently active scope and returns a summary log line (ADR-0156, ADR-0263, ADR-0304).
pub(crate) fn listing(
    view: &mut View,
    store: &std::path::Path,
    presets: Option<&karakuri_environment::places::Presets>,
    folder: Option<&std::path::Path>,
    running: Option<&str>,
) -> String {
    let Some(scope) = view.scope() else {
        return String::from(
            "  library: this console was handed no scopes, so there is no library to list",
        );
    };
    // The filter row narrows store-backed set listings (`AllSets` and `MySets`) (ADR-0299).
    let held = match scope {
        Scope::AllSets | Scope::MySets => library(store),
        _ => Vec::new(),
    };
    // **What the walk found, and what it did not.** Read here rather than
    // inside the arm below so that the sentence about it is written where
    // every other sentence about this listing is; `None` for every scope that
    // is not a history, which is every scope that is a directory of Sets.
    let walked = match scope {
        Scope::History => Some(walked(store, running)),
        _ => None,
    };
    view.holds = holds_choices(&held);
    // Read starred set IDs for every listing update; presets/unheld sets display hollow stars.
    view.starred = favourites(store);
    // Layer filter retired in UI in favor of kind chips; kept None here (ADR-0338).
    let (holds, layer) = {
        let at = view.filters();
        (at.holds.map(str::to_owned), None)
    };
    // Procedures loaded per scope: AllSets includes user-kept, Presets includes shipped presets (ADR-0227, ADR-0338).
    let kept = match scope {
        Scope::AllSets => procedures(store),
        _ => Vec::new(),
    };
    let shipped = match scope {
        Scope::Presets => presets_procedures(presets),
        _ => Vec::new(),
    };
    let kinds = view.filters().kinds;
    // Names and row badges built together to maintain 1:1 alignment (ADR-0338).
    let rows: Vec<(String, RowKind)> = match scope {
        Scope::AllSets => {
            let mut rows: Vec<(String, RowKind, std::time::SystemTime)> = held
                .iter()
                .filter(|set| narrows(set, holds.as_deref(), layer))
                .filter(|_| kinds.shows_sets())
                .map(|set| (set.id.clone(), set_row(set), set.written))
                .chain(
                    kept.iter()
                        .filter(|kept| shows_kept(kinds, kept.kind))
                        .map(|kept| (kept.name.clone(), kept_row(kept), kept.written)),
                )
                .collect();
            // **Most recent first, and the name breaks a tie**, which is
            // `library`'s own order applied to the merged list: two files
            // written inside one tick of a coarse clock tie, and a tied sort
            // is not an order.
            rows.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.0.cmp(&b.0)));
            rows.into_iter().map(|(id, kind, _)| (id, kind)).collect()
        }
        // Starred sets intersection; procedures are excluded as they cannot be starred (ADR-0299, ADR-0338).
        Scope::MySets => held
            .iter()
            .filter(|set| view.starred.contains(&set.id))
            .filter(|set| narrows(set, holds.as_deref(), layer))
            .filter(|_| kinds.shows_sets())
            .map(|set| (set.id.clone(), set_row(set)))
            .collect(),
        // Shipped presets and procedures sorted by name; preset sets omit badges to avoid disk reads (P-0091).
        Scope::Presets => {
            let mut rows: Vec<(String, RowKind)> = presets_listing(presets)
                .into_iter()
                .filter(|_| kinds.shows_sets())
                .map(|preset| (preset.id, RowKind::default()))
                .chain(
                    shipped
                        .iter()
                        .filter(|shipped| shows_kept(kinds, shipped.kind.and_then(kind_of)))
                        .map(|shipped| (shipped.name.clone(), shipped_row(shipped))),
                )
                .collect();
            rows.sort_by(|a, b| a.0.cmp(&b.0));
            rows
        }
        // Folder drag-and-drop listing; lists sets only, excluding loose `.kir` files (ADR-0156, ADR-0338).
        Scope::Folder => folder_listing(folder)
            .into_iter()
            .filter(|_| kinds.shows_sets())
            .map(|id| (id, RowKind::default()))
            .collect(),
        // History versions for the active set, unconstrained by kind chips.
        Scope::History => walked
            .as_ref()
            .map(|found| found.rows.clone())
            .unwrap_or_default()
            .into_iter()
            .map(|row| (row, RowKind::default()))
            .collect(),
    };
    view.library = rows.iter().map(|(name, _)| name.clone()).collect();
    view.kinds = rows.into_iter().map(|(_, kind)| kind).collect();
    // Offers `kind L5` procedures for master chain add, addressing sources by content hash (ADR-0156, ADR-0340).
    view.chain_add = chain_offers(store, &kept, &shipped);
    // Apply filter narrowing only for store-backed scopes.
    let narrowed = matches!(scope, Scope::AllSets | Scope::MySets)
        .then(|| narrowing(holds.as_deref(), None))
        .flatten();
    // Aside message from history walk if truncated or partial.
    let aside = walked.map(|found| found.said).unwrap_or_default();
    let line = match (view.library.len(), narrowed) {
        // Guide operator to reset filter when all sets are hidden by filter criteria.
        (0, Some(narrowed)) => format!(
            "  library: `{}` lists nothing — none of the {} Set{} here {narrowed}, so the \
             filter row is what to press rather than `k`",
            scope.name(),
            held.len(),
            match held.len() {
                1 => "",
                _ => "s",
            }
        ),
        (0, None) => format!(
            "  library: `{}` lists nothing — {}",
            scope.name(),
            why_nothing(scope, folder.is_some(), running.is_some())
        ),
        (listed, Some(narrowed)) => format!(
            "  library: `{}` lists {listed} of {} Set{}, {narrowed}",
            scope.name(),
            held.len(),
            match held.len() {
                1 => "",
                _ => "s",
            }
        ),
        (listed, None) => format!(
            "  library: `{}` lists {listed} {}",
            scope.name(),
            match (scope, listed) {
                (Scope::History, 1) => "version",
                (Scope::History, _) => "versions",
                (_, 1) => "Set",
                (_, _) => "Sets",
            }
        ),
    };
    format!("{line}{aside}")
}
