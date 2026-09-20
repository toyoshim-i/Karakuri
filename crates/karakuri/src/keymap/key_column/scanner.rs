use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use super::data::*;
use karakuri_console::focus::ANY;

/// Byte for byte what `karakuri-console/tests/panel_column.rs` does, and
/// resolves [`PAGE`] from it.
pub fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

pub fn page() -> String {
    let path = workspace().join(PAGE);
    fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "{} is the specification and could not be read: {e}",
            path.display()
        )
    })
}

/// The grammar guard's own keys, beside the digit.
///
/// `crate::grammar` matches all six as literals — `Key::Named(NamedKey::…)`
/// arms `bound` used to scan for — but they stay declared here rather than
/// scanned for the same reason [`DIGIT`] always was one line down:
/// [`crate::keymap::KEY_BINDINGS`] is checked against `docs/manual/operations.html`
/// directly by
/// [`every_binding_the_table_names_a_title_for_reaches_a_route_marked_built`]
/// below, and a scan of this file's text is not what answers *what does the
/// window loop bind* for any key any more, table-driven or guard.
const GRAMMAR_KEYS: &[&str] = &["up", "down", "left", "right", "space", "enter"];

/// `backspace`, which is neither in [`crate::keymap::KEY_BINDINGS`] nor in
/// `crate::grammar`. It is a literal in the two letter-taking flows — the
/// arrangement pill's name and the inspector's — that return before
/// `window_event`'s own `match` on `key.logical_key` is ever reached, so it
/// belongs to neither table. Both flows bind it unconditionally, so unlike
/// [`DIGIT`] it is declared outright in [`bound`] rather than asked of anything
/// at runtime.
const NAME_ENTRY_KEY: &str = "backspace";

/// Every key the window loop binds, both the table and the grammar guard beside
/// it — declared rather than scanned.
///
/// [`crate::keymap::KEY_BINDINGS`] answers the ten literal keys directly: this is now a
/// lookup over data `window_event` itself dispatches through, not a second copy
/// of it. The other eight — `crate::grammar`'s four named keys, the four
/// arrows, and the digit — are not in that table (`crate::grammar`'s own doc
/// comment says why: it is a guard rather than arms, for the same reason the
/// digits were always a special case here), so they are declared in
/// [`GRAMMAR_KEYS`] and [`DIGIT`] rather than read out of this file's source.
/// And `backspace` — see [`NAME_ENTRY_KEY`] — is neither the table's nor the
/// grammar's, and is declared for its own reason beside them. None of the four
/// is data this function could observe wrongly — every one names permanent
/// code, not a configuration — so declaring them is not a weaker check than
/// scanning for them was; it is the same facts, asserted instead of parsed.
pub fn bound() -> BTreeSet<String> {
    let mut found: BTreeSet<String> = crate::keymap::KEY_BINDINGS
        .iter()
        .map(|binding| binding.legend.to_owned())
        .collect();
    found.extend(GRAMMAR_KEYS.iter().map(|key| (*key).to_owned()));
    found.insert(NAME_ENTRY_KEY.to_owned());
    // **The one key of the grammar that names a different row in every
    // bay**, contributed by the console's dispatch table rather than
    // declared unconditionally like the rest of [`GRAMMAR_KEYS`] — see
    // [`DIGIT`]. `crate::grammar` binds it with a guard rather than a
    // literal, and *which* rows it reaches depends on
    // `karakuri_console::focus::BUILT` rather than on anything this
    // file says, so this is the one entry that still asks the console
    // rather than stating a fact `main.rs` alone could get wrong.
    if !karakuri_console::focus::BUILT.is_empty() {
        found.insert(DIGIT.to_owned());
    }
    found
}

/// Every row's title and its key badge, in page order: the badge's class —
/// `has`, `plan` or `gap` — and the keys it names.
///
/// Read verbatim and never decoded, which is `mcp.rs`'s rule and
/// `panel_column.rs`'s after it: a badge that names nothing says `&mdash;`, and
/// a key that needed decoding to match would be a key nobody could find on
/// their keyboard.
pub fn key_badges() -> Vec<(String, String, String)> {
    let html = page();
    let mut found = Vec::new();
    for row in html.split(ROW).skip(1) {
        let Some(open) = row.find("<h3>") else {
            continue;
        };
        let rest = &row[open + "<h3>".len()..];
        let Some(close) = rest.find("</h3>") else {
            continue;
        };
        let title = rest[..close].to_string();
        // The row ends where the next section does; a badge found past
        // that would belong to another row.
        let body = &rest[close..];
        let body = &body[..body.find("</section>").unwrap_or(body.len())];
        let mut badge = None;
        for span in body.split(r#"<span class="rt "#).skip(1) {
            let Some(quote) = span.find('"') else {
                continue;
            };
            let class = span[..quote].to_string();
            let Some(text) = span[quote..].strip_prefix(r#"">key <b>"#) else {
                continue;
            };
            let Some(shut) = text.find("</b>") else {
                continue;
            };
            badge = Some((class, text[..shut].to_string()));
            break;
        }
        let Some((class, keys)) = badge else {
            continue;
        };
        found.push((title, class, keys));
    }
    found
}

/// The rows [`ROWS`] says a route reaches, or `None` if this program does not
/// bind it at all.
///
/// A route is a key and the bay it is addressed in, `None` for a global — which
/// is the whole of what changed here: `space` alone names no route, and
/// `("mixer", "space")` names five rows.
pub fn rows_of(bay: Option<&str>, key: &str) -> Option<&'static [&'static str]> {
    ROWS.iter()
        .find(|(b, k, _)| *b == bay && *k == key)
        .map(|(_, _, rows)| *rows)
}

/// What a key badge says, parsed — the keys it names and the bay it names them
/// in, or `None` for a badge that is not one of the two spellings this column
/// carries.
///
/// The two spellings are ADR-0331's: a built badge names bare letters, and a
/// designed one names a key of the grammar and the bay it is addressed in.
/// Since 2026-09-10 a built badge may be either, because the grammar is bound
/// in two bays — which is the clause that record left for *"the code that binds
/// `Tab`"* and this is it.
///
/// A badge naming a bay resolves every key in it to that bay; a badge naming
/// none resolves every key to a global. A badge cannot mix them, and that is
/// not a limitation to work around: a press goes to the bay that has focus or
/// it does not, and a row reached both ways would need two badges rather than
/// one with two halves.
pub fn parsed(badge: &str) -> Option<(Vec<String>, Option<&'static str>)> {
    let (keys, bay) = match badge.split_once(IN_THE) {
        Some((keys, bay)) => {
            let bay = bay.trim();
            // **The any-bay spelling first**, because it is not one of the
            // nine and resolving it against [`BAYS`] would answer `None`
            // and read as a badge nobody can parse.
            let bay = match bay == ANY_BAY.0 {
                true => ANY_BAY.1,
                false => BAYS
                    .iter()
                    .find(|(page, _)| *page == bay)
                    .map(|(_, name)| *name)?,
            };
            (keys, Some(bay))
        }
        None => (badge, None),
    };
    let keys = keys
        .split_whitespace()
        .map(|key| match key.chars().all(|c| c.is_ascii_digit()) {
            // Every digit is the one key of the grammar, which is what
            // makes `1 2 3 4` a badge naming one route and not four.
            true => DIGIT.to_owned(),
            false => SPELLED
                .iter()
                .find(|(page, _)| *page == key)
                .map_or_else(|| key.to_owned(), |(_, name)| (*name).to_owned()),
        })
        .collect();
    Some((keys, bay))
}

/// How a message names the bay a route is addressed in, or says it is global.
pub fn said(bay: Option<&str>) -> String {
    match bay {
        Some(bay) if bay == ANY => String::from(" in any bay"),
        Some(bay) => format!(" in the {bay}"),
        None => String::from(" globally"),
    }
}
