use super::*;

/// Every arrangement the store holds, by name, for the pill's menu to list —
/// [`library`] over the fourth directory rather than the first.
///
/// Its two rules are this one's, said again because they are the same two: a
/// store that is not there is listed as nothing and is not created, since a
/// program that listed a menu by first making a store would change the
/// directory it was run in; and a store that could not be read says so, since a
/// menu that is empty because the directory would not open looks exactly like
/// one that is empty because nobody has saved.
///
/// Read when it changes rather than per frame. Once at startup, and again after
/// a save lands — which is the only thing in this program that adds a name.
/// `Store::list_arrangements` sorts by name, so the menu draws what it is
/// handed and sorts nothing.
pub(crate) fn arrangements(root: &std::path::Path) -> Vec<String> {
    if !root.is_dir() {
        return Vec::new();
    }
    match Store::open(root).and_then(|store| store.list_arrangements()) {
        Ok(filed) => filed.into_iter().map(|entry| entry.name).collect(),
        Err(e) => {
            println!("arrangements: {} could not be listed: {e}", root.display());
            Vec::new()
        }
    }
}

/// Stars or unstars a Set in the store's favourites list (ADR-0261, ADR-0299, ADR-0301).
///
/// Star operations requested by an automated model are rejected to preserve operator library intent (P-0096).
pub(crate) fn favourite(
    root: &std::path::Path,
    asked: Asked,
    operation: &Operation,
) -> Option<String> {
    let Operation::SetFavourite { id, favourite } = operation else {
        return None;
    };
    if asked == Asked::Model {
        return Some(format!(
            "star: `{id}` was not {} — `my sets` is the list of Sets the operator chose, and a \
             star is theirs to put on: it is one press on the mark at the left of that row in \
             the Library bay, under `all`. The Set itself is untouched and is listed there",
            match favourite {
                true => "starred",
                false => "unstarred",
            }
        ));
    }
    let wrote = Store::open(root).and_then(|store| store.set_favourite(id, *favourite));
    Some(match wrote {
        Ok(true) => format!(
            "star: `{id}` {} — `my sets` is the starred subset of `all`, and this row is {} \
             it",
            match favourite {
                true => "is starred",
                false => "has its star off",
            },
            match favourite {
                true => "in",
                false => "out of",
            }
        ),
        Ok(false) => format!(
            "star: `{id}` was already {}, so nothing was written",
            match favourite {
                true => "starred",
                false => "unstarred",
            }
        ),
        Err(e) => format!(
            "star: `{id}` was not {}: {e}",
            match favourite {
                true => "starred",
                false => "unstarred",
            }
        ),
    })
}

/// Saves or restores named layout arrangements to/from disk (ADR-0158, ADR-0221).
///
/// Returns `None` if the operation is unrelated to arrangement persistence.
pub(crate) fn arrangement(
    root: &std::path::Path,
    panel: &mut Panel,
    arr: &mut view::Arrangement,
    operation: &Operation,
) -> Option<String> {
    match operation {
        Operation::SaveArrangement { name } => {
            let (line, kept) = keep_arrangement(root, panel, name);
            // **The name in use moves only when the file did.** A save that
            // was refused leaves the pill saying what it said, because
            // nothing under that name is on the disk — and the listing is
            // re-read only then, since a refusal added no name to it.
            if kept {
                arr.name = Some(name.clone());
                arr.filed = arrangements(root);
            }
            Some(line)
        }
        Operation::RestoreArrangement { name } => {
            let (line, back) = put_arrangement_back(root, panel, name);
            if back {
                arr.name = Some(name.clone());
            }
            Some(line)
        }
        _ => None,
    }
}

/// The running arrangement, filed under `name` — and whether it landed. See
/// [`arrangement`].
pub(crate) fn keep_arrangement(
    root: &std::path::Path,
    panel: &Panel,
    name: &str,
) -> (String, bool) {
    if let Err(refusal) = checked_name(name) {
        return (refusal, false);
    }
    let bytes = match serde_json::to_vec(panel.layout()) {
        Ok(bytes) => bytes,
        Err(e) => {
            return (
                format!("arrangement: `{name}` was not kept — it did not serialise: {e}"),
                false,
            )
        }
    };
    let wrote = Store::open(root).and_then(|store| store.write_arrangement(name, &bytes));
    match wrote {
        Ok(()) => (
            format!(
                "arrangement: kept as `{name}` — {} bytes at {}",
                bytes.len(),
                root.join("arrangements")
                    .join(format!("{name}.arrangement.json"))
                    .display()
            ),
            true,
        ),
        Err(e) => (format!("arrangement: `{name}` was not kept: {e}"), false),
    }
}

/// Validates that an arrangement name consists of safe alphanumeric, `-`, or `_` path characters (ADR-0221, P-0090).
pub(crate) fn checked_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err(
            "arrangement: nothing was typed, and an arrangement is filed under a name — \
             the default arrangement is the one that has none"
                .to_owned(),
        );
    }
    match name
        .chars()
        .find(|c| !c.is_ascii_alphanumeric() && *c != '-' && *c != '_')
    {
        Some(bad) => Err(format!(
            "arrangement: `{name}` holds `{bad}`, and an arrangement name is letters, digits, \
             `-` and `_`: it is one path component and it names a file under \
             `<store>/arrangements/`"
        )),
        None => Ok(()),
    }
}

/// The arrangement filed under `name`, into the window the panel already has.
/// See [`arrangement`].
pub(crate) fn put_arrangement_back(
    root: &std::path::Path,
    panel: &mut Panel,
    name: &str,
) -> (String, bool) {
    if !root.is_dir() {
        return (
            format!(
                "arrangement: no store at {}, so nothing is filed under `{name}` — and the \
                 console has not moved",
                root.display()
            ),
            false,
        );
    }
    let bytes = match Store::open(root).and_then(|store| store.read_arrangement(name)) {
        Ok(bytes) => bytes,
        Err(e) => return (format!("arrangement: `{name}` is not back — {e}"), false),
    };
    let layout: Layout = match serde_json::from_slice(&bytes) {
        Ok(layout) => layout,
        // **Refused whole rather than repaired**, which is the loader's own
        // sentence and not this file's judgement (ADR-0158).
        Err(e) => {
            return (
                format!(
                    "arrangement: `{name}` is not back — the file disagrees with itself and is \
                     refused rather than repaired: {e}"
                ),
                false,
            )
        }
    };
    let viewport = panel.layout().viewport();
    panel.restore(layout);
    (
        format!(
            "arrangement: `{name}` is back, at the {} x {} this window already had",
            viewport.w, viewport.h
        ),
        true,
    )
}
