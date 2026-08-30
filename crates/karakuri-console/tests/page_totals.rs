//! **The operations page's own summary, against the operations page's own
//! badges.**
//!
//! [`docs/manual/operations.html`](../../../docs/manual/operations.html) opens
//! by counting itself — *"54 operations, and 45 of the 232 ways in exist"* —
//! and its *Arranging the console* essay counts that section the same way. Two
//! other files already check the page against **code**
//! ([`panel_column.rs`](../tests/panel_column.rs) against the operations a
//! control emits, [`vocabulary.rs`](../tests/vocabulary.rs) against
//! `karakuri_operation::Operation` and a running `Panel`). Nothing checked the
//! page against **itself**, and that is where it kept going wrong.
//!
//! # Why this file exists, which is a count of how often it has happened
//!
//! Three times in three days a badge moved and a sentence about how many
//! badges there are did not. Twice the sentence was corrected by hand and went
//! stale again within the day; once it went stale by the very commit that
//! corrected it. `docs/roadmap.md` answered the same problem by **deleting**
//! eight transcribed figures and writing the command that derives each — it
//! can, because it is read by people at a terminal. This page cannot: it is
//! published at <https://toyoshim-i.github.io/Karakuri/manual/> for somebody
//! who wants to play the instrument, and a lede that says *run this grep* is
//! not a lede. So the figures stay written and this is what holds them true.
//!
//! **The figures checked here are exactly the ones the page derives from
//! itself**: how many rows it has, how many badges, how many are built, and
//! the same three over the one section that states its own arithmetic. A
//! figure about anything else — the CLI's thirty-nine keys, the eight letters
//! that collide — is a claim about code and belongs in the two files above.
//!
//! # The numbers are words as often as digits
//!
//! The page writes `54` and `232` as digits and *Eight*, *nine*, *thirty-two*,
//! *Five* and *four* as words, because that is how the sentences read.
//! [`count`] takes either, and **a word this page starts using that is not in
//! its table is a loud failure rather than a silent skip** — the alternative
//! is a check that quietly stops checking the day somebody writes *ten*.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// The page this file answers for. The same constant `panel_column.rs` and
/// `vocabulary.rs` carry, and for the same reason: a path written once.
const PAGE: &str = "docs/manual/operations.html";

/// The section whose own arithmetic is checked alongside the whole page's, and
/// the one this crate answers for.
const SECTION: &str = "<h2>Arranging the console</h2>";

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

/// **The page as prose**: every tag dropped and every run of whitespace folded
/// to one space.
///
/// Every anchor below is a phrase somebody wrote to be read, and the source is
/// hard-wrapped and marked up. An anchor that had to spell `<strong>` or land
/// on a line break would be a check that failed the day somebody re-flowed a
/// paragraph or emphasised a different word — a false alarm about the one
/// thing this file is not about. The counts are taken from the raw text, where
/// the markup is; only the sentences are read from this.
///
/// Entities are left alone: `&mdash;` is a word of the prose as far as an
/// anchor is concerned, and decoding it would only give an anchor a character
/// nobody can type.
fn flat(page: &str) -> String {
    let mut prose = String::with_capacity(page.len());
    let mut depth = 0usize;
    for c in page.chars() {
        match c {
            '<' => depth += 1,
            '>' => depth = depth.saturating_sub(1),
            _ if depth == 0 => prose.push(c),
            _ => {}
        }
    }
    prose.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// **Everything between `SECTION` and the next `<h2>`.**
fn section(page: &str) -> &str {
    let from = page
        .find(SECTION)
        .unwrap_or_else(|| panic!("`{SECTION}` is gone from {PAGE}"));
    let rest = &page[from + SECTION.len()..];
    match rest.find("<h2>") {
        Some(to) => &rest[..to],
        None => rest,
    }
}

/// A row is an `<h3>`, which is what `grep -c '<h3'` counts and what the
/// README names as the way to ask how many operations there are.
fn rows(html: &str) -> usize {
    html.matches("<h3>").count()
}

/// **Every badge in `html`, as its class.** A badge is an `rt` span, the same
/// shape `panel_column.rs` reads and the same one the three commands under
/// *The meter is one column* in `docs/roadmap.md` read.
fn badges(html: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut rest = html;
    while let Some(at) = rest.find("class=\"rt ") {
        rest = &rest[at + "class=\"rt ".len()..];
        let Some(end) = rest.find('"') else { break };
        out.push(&rest[..end]);
        rest = &rest[end..];
    }
    out
}

fn built(html: &str) -> usize {
    badges(html).iter().filter(|c| **c == "has").count()
}

/// How many badges of one column in `html` read `has`.
fn built_in(html: &str, column: &str) -> usize {
    html.matches(&format!("class=\"rt has\">{column} ")).count()
}

/// **A number the page wrote, as digits or as a word.**
///
/// Panics on a word this table does not hold, rather than answering `None` and
/// letting the assertion above it pass on a figure nobody read.
fn count(word: &str) -> usize {
    let words: HashMap<&str, usize> = [
        ("one", 1),
        ("two", 2),
        ("three", 3),
        ("four", 4),
        ("five", 5),
        ("six", 6),
        ("seven", 7),
        ("eight", 8),
        ("nine", 9),
        ("ten", 10),
        ("eleven", 11),
        ("twelve", 12),
        ("twenty", 20),
        ("thirty-two", 32),
    ]
    .into_iter()
    .collect();
    let key = word.trim().to_ascii_lowercase();
    if let Ok(n) = key.parse::<usize>() {
        return n;
    }
    *words.get(key.as_str()).unwrap_or_else(|| {
        panic!(
            "{PAGE} counts itself with the word `{word}`, which this file has no number for — \
             add it to `count` rather than letting the figure go unchecked"
        )
    })
}

/// **The one number between `before` and `after`**, so a sentence is found by
/// what it says rather than by where it sits.
///
/// Panics naming the anchor when the sentence has been rewritten. That is the
/// behaviour this file wants: a summary that moved without being re-derived is
/// what it exists to catch, and a check that quietly found no sentence would
/// report the opposite of the truth.
fn figure(flat: &str, before: &str, after: &str) -> usize {
    let from = flat.find(before).unwrap_or_else(|| {
        panic!(
            "{PAGE} no longer says `{before}` — the sentence this file checks was rewritten. \
             Re-point the anchor at the new wording rather than deleting the check"
        )
    });
    let rest = &flat[from + before.len()..];
    let to = rest
        .find(after)
        .unwrap_or_else(|| panic!("{PAGE} says `{before}` and no longer says `{after}` after it"));
    count(&rest[..to])
}

/// **The prose from `phrase` onward**, so a figure is looked for inside the
/// sentence that states it rather than in the first place on the page that
/// happens to use the same three words.
fn from<'a>(flat: &'a str, phrase: &str) -> &'a str {
    let at = flat.find(phrase).unwrap_or_else(|| {
        panic!("{PAGE} no longer says `{phrase}` — the passage this file checks was rewritten")
    });
    &flat[at..]
}

/// **The number immediately before `after`**, for a figure whose sentence
/// gives it no left-hand anchor of its own.
fn figure_before(flat: &str, after: &str) -> usize {
    let at = flat.find(after).unwrap_or_else(|| {
        panic!("{PAGE} no longer says `{after}` — the sentence this file checks was rewritten")
    });
    let word = flat[..at]
        .rsplit(' ')
        .next()
        .expect("a word before the phrase");
    count(word)
}

/// **The lede counts the whole page, and the page is what it counts.**
///
/// Three figures in one sentence: how many operations there are, how many of
/// the ways in exist, and how many ways in there are. Each is a `grep` over
/// this same file, so none can be right in the prose and wrong in the badges
/// without this failing.
#[test]
fn the_lede_counts_the_page_it_is_the_lede_of() {
    let page = page();
    let flat = flat(&page);

    let ops = figure_before(&flat, " operations, and ");
    assert_eq!(
        ops,
        rows(&page),
        "{PAGE}'s lede says {ops} operations and the page has {} `<h3>` rows",
        rows(&page)
    );

    let exist = figure(&flat, " operations, and ", " of the ");
    assert_eq!(
        exist,
        built(&page),
        "{PAGE}'s lede says {exist} of the ways in exist and {} badges read `has`",
        built(&page)
    );

    let ways = figure_before(&flat, " ways in exist");
    assert_eq!(
        ways,
        badges(&page).len(),
        "{PAGE}'s lede says {ways} ways in and the page draws {} badges",
        badges(&page).len()
    );
}

/// **The console's own section counts itself too**, and it is the section this
/// crate answers for — so a control landed here moves a badge, and the two
/// sentences about how many badges there are have to move with it.
#[test]
fn the_arranging_section_counts_its_own_rows_and_routes() {
    let page = page();
    let flat = flat(&page);
    let lede = from(&flat, "ways in exist. ");
    let section = section(&page);

    let shape = figure_before(lede, " of them are the console's own shape");
    assert_eq!(
        shape,
        rows(section),
        "{PAGE}'s lede says {shape} operations are the console's own shape and `{SECTION}` has {}",
        rows(section)
    );

    let exist = figure_before(lede, " of their ");
    assert_eq!(
        exist,
        built(section),
        "{PAGE}'s lede says {exist} of that section's routes exist and {} of its badges read `has`",
        built(section)
    );

    let ways = figure(lede, " of their ", " routes exist");
    assert_eq!(
        ways,
        badges(section).len(),
        "{PAGE}'s lede says that section has {ways} routes and it draws {} badges",
        badges(section).len()
    );
}

/// **And the essay under the table counts the same section a second way**, by
/// column — which is the sentence that went stale twice.
#[test]
fn the_arranging_essay_counts_the_columns_it_names() {
    let page = page();
    let flat = flat(&page);
    let essay = from(&flat, "Arranging the console is reached from the keyboard");
    let section = section(&page);

    let by_key = figure_before(essay, " of its ");
    assert_eq!(
        by_key,
        built_in(section, "key"),
        "the essay says {by_key} of `{SECTION}`'s rows are built in the key column and {} of its \
         key badges read `has`",
        built_in(section, "key")
    );

    let of_eight = figure(essay, " of its ", " rows are built in the key column");
    assert_eq!(
        of_eight,
        rows(section),
        "the essay says `{SECTION}` has {of_eight} rows and it has {}",
        rows(section)
    );

    let by_panel = figure(
        essay,
        " rows are built in the key column and ",
        " in the panel column",
    );
    assert_eq!(
        by_panel,
        built_in(section, "panel"),
        "the essay says {by_panel} of `{SECTION}`'s rows are built in the panel column and {} of \
         its panel badges read `has`",
        built_in(section, "panel")
    );
}

/// **The floor.** Every scan above answers zero on a page whose markup has
/// changed shape, and zero compares equal to zero — so a suite that had
/// stopped reading the page would pass every assertion in it.
///
/// The same floor `panel_column.rs` and `vocabulary.rs` each carry, stated
/// here because it is this file's scans that would be the ones returning
/// nothing.
#[test]
fn the_scans_find_the_page() {
    let page = page();
    let section = section(&page);
    assert!(
        rows(&page) >= 40,
        "only {} `<h3>` rows found in {PAGE} — is a row still an `<h3>`?",
        rows(&page)
    );
    assert!(
        badges(&page).len() >= 200,
        "only {} badges found in {PAGE} — is a badge still `class=\"rt …\"`?",
        badges(&page).len()
    );
    assert!(
        rows(section) >= 8,
        "only {} rows found under `{SECTION}`, which states its own arithmetic",
        rows(section)
    );
    assert!(
        built(&page) > 0 && built(section) > 0 && built_in(section, "key") > 0,
        "no badge read `has`, so every comparison above is zero against zero"
    );
}
