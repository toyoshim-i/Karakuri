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

/// Literal navigation and activation keys handled in grammar guards.
const GRAMMAR_KEYS: &[&str] = &["up", "down", "left", "right", "space", "enter"];

/// Backspace key handled in text-entry input flows.
const NAME_ENTRY_KEY: &str = "backspace";

/// Returns the complete set of keys bound by the window loop across table and guards.
pub fn bound() -> BTreeSet<String> {
    let mut found: BTreeSet<String> = crate::keymap::KEY_BINDINGS
        .iter()
        .map(|binding| binding.legend.to_owned())
        .collect();
    found.extend(GRAMMAR_KEYS.iter().map(|key| (*key).to_owned()));
    found.insert(NAME_ENTRY_KEY.to_owned());
    // Include digit key if any bay in console focus defines active bindings.
    if !karakuri_console::focus::BUILT.is_empty() {
        found.insert(DIGIT.to_owned());
    }
    found
}

/// Extracts row title, badge class (`has`, `plan`, `gap`), and keys from the manual HTML.
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

/// Returns matching manual rows reached by a given bay and key combination.
pub fn rows_of(bay: Option<&str>, key: &str) -> Option<&'static [&'static str]> {
    ROWS.iter()
        .find(|(b, k, _)| *b == bay && *k == key)
        .map(|(_, _, rows)| *rows)
}

/// Parses key badge content into list of keys and optional target bay (ADR-0331).
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
