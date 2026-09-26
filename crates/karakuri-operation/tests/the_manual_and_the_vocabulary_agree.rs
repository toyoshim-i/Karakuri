//! Integration tests verifying bidirectional correspondence between
//! operations in `docs/manual/operations.html` and the [`Operation`] enum.

use std::fs;
use std::path::{Path, PathBuf};

use karakuri_operation::Operation;

/// The specification, relative to the workspace root.
const PAGE: &str = "docs/manual/operations.html";

/// HTML opening tag marking the start of an operation definition block in the manual.
const ROW: &str = r#"<div class="op-head">"#;

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

fn page() -> String {
    let path = workspace().join(PAGE);
    fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "{} is the specification and could not be read: {e}",
            path.display()
        )
    })
}

/// Extracts verbatim text of every `<h3>` opening an operation row, preserving document order.
fn headings(html: &str) -> Vec<String> {
    let mut found = Vec::new();
    for row in html.split(ROW).skip(1) {
        let Some(open) = row.find("<h3>") else {
            continue;
        };
        let rest = &row[open + "<h3>".len()..];
        let Some(close) = rest.find("</h3>") else {
            continue;
        };
        found.push(rest[..close].to_string());
    }
    found
}

#[test]
fn the_manual_and_the_vocabulary_name_the_same_operations() {
    let html = page();
    let rows = headings(&html);

    for title in &rows {
        assert!(
            Operation::TITLES.contains(&title.as_str()),
            "`{title}` is an operation in {PAGE} and no variant of `Operation` carries that \
             title — the manual specifies an operation the vocabulary cannot name, so no \
             surface can route into it. Add a variant to `operations!` in \
             karakuri-operation/src/lib.rs carrying what it acts on, or `Undecided` where \
             what it acts on is not settled"
        );
    }

    for title in Operation::TITLES {
        assert!(
            rows.iter().any(|row| row == title),
            "`Operation` carries the title `{title}` and no row of {PAGE} says it — an \
             operation nobody specified, with no prose and no four routes. Either a row was \
             removed from the page or a title here drifted from the one it is copied from; \
             the page is the specification, so change it there first and then here"
        );
    }

    // Minimum floor assertion to prevent false passes if document structure or path changes.
    assert!(
        rows.len() >= 62,
        "only {} operations found in {PAGE} — is a row still `{ROW}` followed by an `<h3>`?",
        rows.len()
    );
}

/// Verifies that every `<h3>` heading in the manual opens an operation row.
#[test]
fn every_heading_on_the_page_is_an_operation() {
    let html = page();
    let all = html.matches("<h3>").count();
    let rows = headings(&html).len();
    assert_eq!(
        all, rows,
        "{PAGE} has {all} `<h3>` headings and {rows} of them open an operation row. An \
         `<h3>` on that page is an operation; one outside a `{ROW}` is either a row with \
         broken markup — which this test reads as no operation at all — or a heading that \
         wants to be an `<h4>`"
    );
}

/// Titles are compared byte for byte, so an HTML entity in a heading is a
/// title the type cannot spell the same way. It would read as one operation
/// missing and one unspecified, which is two confusing failures for one
/// typographic decision — so it fails here instead, saying what it is.
#[test]
fn every_title_is_plain_text() {
    for title in headings(&page()) {
        assert!(
            !title.contains('&'),
            "the row `{title}` in {PAGE} spells its heading with an HTML entity. A title is \
             matched byte for byte against `Operation::TITLES`, and a Rust string literal \
             cannot carry `&mdash;` — write the character itself, as every other heading on \
             the page does"
        );
    }
}

/// Verifies that `Operation::TITLES` strictly follows the document order in the manual.
#[test]
fn the_vocabulary_is_in_the_manuals_order() {
    let rows = headings(&page());
    let named: Vec<&str> = Operation::TITLES.to_vec();
    if rows.len() != named.len() {
        // Said by the membership test, in its own words. Saying it twice here
        // would report a reordering that has not happened.
        return;
    }
    for (at, (row, title)) in rows.iter().zip(named).enumerate() {
        assert_eq!(
            row, title,
            "operation {at} is `{row}` in {PAGE} and `{title}` in `Operation` — so either a \
             row moved, or one side was reworded and the other was not. The enum follows the \
             page, section by section, so that a reviewer can read them side by side"
        );
    }
}
