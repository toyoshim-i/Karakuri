//! **`panel::Op` against the manual's *Arranging the console* rows.**
//!
//! `karakuri-operation`'s own
//! `tests/the_manual_and_the_vocabulary_agree.rs` reads
//! `docs/manual/operations.html` and checks it against `Operation` in both
//! directions. This is that check one level out: the page is still the
//! specification, and what is checked against it is the type the console
//! actually performs — [`karakuri_console::panel::Op`].
//!
//! # Why it is a separate check rather than the same one
//!
//! The vocabulary is a leaf crate with no dependencies at all, so it names a
//! region by `String` — `FoldBay { bay }`, `FoldPane { pane }`,
//! `Unfold { region }`, `Solo { region }` — and `karakuri_layout::Layout::find`
//! resolves one, because `Layout::new` refuses an arrangement that uses a name
//! twice. `Op` names a [`karakuri_layout::NodeId`], which is a handle with a
//! private field that only a `Layout` can hand out. So the two types cannot be
//! the same type, and the question *do they name the same operations* is not
//! answered by either crate's own suite.
//!
//! # The three things it pins, and every one of them is a gap
//!
//! - **One `Op` variant has no row**: [`Op::Report`], and that one is
//!   permanent (ADR-0205) — its reply is a list of pixel rectangles keyed by a
//!   handle, and three of the four surfaces could not carry it. The page is
//!   the specification for *which operations exist*, so an operation the
//!   console performs and the page does not name is an operation nobody
//!   specified — and giving it a row is an edit to the specification rather
//!   than a rename. [`Op::Reset`] was the second of the two until that edit
//!   was made: *Reset the arrangement* is its row now (ADR-0208), and the
//!   mapping to it is what [`rows_of`] pins.
//! - **One row has no `Op`**: *Move a boundary*, which is
//!   [`karakuri_console::panel::Panel::press`], `moved` and `released` — a
//!   gesture rather than an operation. The vocabulary carries it as
//!   `Undecided` for the same reason, and says so at the variant.
//! - **Two splits in the console's arrangement have no name**, so nothing but
//!   the pointer can reach them: the root column and the body row. `Layout`
//!   hands both to a caller as `Hit::Divider { split, .. }`, the program's `g`
//!   turns that into [`Op::Fold`] of the split, and
//!   `Outcome::Folded { root: true }` exists to report one of the two. A
//!   `String` cannot say either — `Layout::name` answers `None` — so an `Op`
//!   replaced by an `Operation` would lose them.
//!
//! **The third of those is decided now, and the decision is that they stay
//! unnamed** (ADR-0204): folding the root produces a blank window, which is
//! not the outcome the manual reaches for — *Solo a region* is, and its row
//! says so — and the body row has no word on the page at all. So the count
//! below is no longer a decision waiting to be taken; it is **what the
//! migration costs, held at two**. The first is decided too, and in both
//! directions: [`Op::Report`] keeps no row for good and [`Op::Reset`] gained
//! one. *Move a boundary* is the one still open, and this file is what stops
//! any of the three being closed by accident.
//!
//! # [`Op::FoldEnclosing`] is not a fourth, and the rule is what settles it
//!
//! It reads like one. It names its target *by relation* — the split around
//! whatever the pointer is over — where every row on the page names one
//! outright, so an inventory taken by eye counts it as an operation the page
//! has no word for. But **naming a target by relation is the caller's and
//! never the operation's**, and that is settled three times over.
//! `karakuri_operation::Operation::Crossfade` states it as a rule at the
//! variant — *"**Both decks named.** *The next deck* is the keyboard's
//! translation of this, not the operation."* ADR-0175 applied it to this
//! variant already: *under the pointer* stopped being part of what an
//! operation **means** and became one way of naming which region, so the two
//! arms live in `crates/karakuri/src/main.rs` and a divider arrives as
//! [`Op::Fold`] of the split it already names. And [`Op`]'s own doc says it
//! outright — *"**Resolving the pointer is the caller's**"*.
//!
//! What is left on the model's side of that seam is one step up the tree:
//! `FoldEnclosing(id)` is `Fold(layout.parent(id))`, and `Panel::op` is where
//! the parent is read. So it performs the fold the page already specifies, on
//! a node the model works out, and **the page owes it no row** — which is why
//! [`rows_of`] gives it the same two rows as [`Op::Fold`] rather than a row of
//! its own. A row appearing on the page *for* it would be the specification
//! gaining an operation this rule says it does not need, and
//! [`every_row_on_the_page_has_an_operation`] is the assertion that fails
//! then: a row no `Op` reaches is a row somebody has to account for.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use karakuri_console::panel::{Op, Panel};
use karakuri_layout::NodeId;

/// The specification, relative to the workspace root.
const PAGE: &str = "docs/manual/operations.html";

/// The section of it this crate answers for.
const SECTION: &str = "<h2>Arranging the console</h2>";

/// What marks a row on that page — the same marker
/// `karakuri-operation`'s test matches, and for the same reason: sections are
/// `<h2>` and a heading somebody adds for looks is neither.
const ROW: &str = r#"<div class="op-head">"#;

/// Rows in the section that no [`Op`] reaches, with why.
///
/// **A drag is not an operation.** Moving a boundary is
/// `Panel::press`/`moved`/`released` over `Layout::hit`, and it is a gesture
/// with a position in it rather than something a caller can ask for by name.
/// `Operation::MoveBoundary` carries `Undecided` and gives the same two
/// reasons: more than half the boundaries here belong to a split the
/// arrangement left unnamed, and `Layout::set_divider` takes a pixel.
const NO_OP: &[&str] = &["Move a boundary"];

/// [`Op`] variants that no row names, with why.
///
/// **One entry, and it is permanent.** [`Op::Report`] is a question whose
/// reply the vocabulary cannot say: `Outcome::Report` is a `Vec<Placement>`,
/// and a `Placement` carries a [`NodeId`] and a pixel `Rect` — a handle no
/// surface but this crate can hold, in the coordinates of a window a model is
/// not looking at. A row for it would promise three surfaces a reply that does
/// not cross, which is not the same as three routes nobody has built yet
/// (ADR-0205).
///
/// **[`Op::Reset`] was the other entry and is not one any more.** It is a
/// change rather than a question, it names no target, and *Reset the
/// arrangement* is its row (ADR-0208) — so what this file pins about it is now
/// the mapping in [`rows_of`] rather than its absence, and putting it back
/// here fails [`the_variants_with_no_row_are_the_ones_written_down`] in one
/// direction and [`every_row_on_the_page_has_an_operation`] in the other.
const NO_ROW: &[&str] = &["Report"];

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

/// Every `<h3>` of a row in *Arranging the console*, in page order.
fn rows() -> Vec<String> {
    let text = page();
    let start = text.find(SECTION).unwrap_or_else(|| {
        panic!("`{SECTION}` is gone from {PAGE} — the section this crate answers for")
    });
    let rest = &text[start + SECTION.len()..];
    let end = rest.find("<h2").unwrap_or(rest.len());
    let mut out = Vec::new();
    for part in rest[..end].split(ROW).skip(1) {
        let open = part.find("<h3>").expect("a row opens with its heading");
        let close = part[open..].find("</h3>").expect("a heading closes");
        out.push(part[open + 4..open + close].to_owned());
    }
    out
}

/// **Which rows an operation lands on.** One match over every variant, so a
/// variant added to [`Op`] does not compile until somebody has said which row
/// it is — or that it is none, which is [`NO_ROW`] and wants a reason there.
///
/// More than one variant may name one row, and one variant may name two:
/// [`Op::Fold`] folds whatever node it is handed, and the page draws the bay
/// and the pane as separate rows because it describes two different
/// consequences. Whether they are one operation over two kinds of region is
/// an open question at `Operation::FoldPane` itself.
fn rows_of(op: Op) -> &'static [&'static str] {
    match op {
        Op::Fold(_) => &["Fold a bay away", "Fold a pane away"],
        // `Fold` of a node the model works out — `layout.parent(id)` — so it
        // performs whichever of the two folds that node turns out to be, and
        // needs no row of its own. Both rows for the same reason `Fold` has
        // both: the split enclosing a region of a bay is that bay, and the
        // split enclosing a bay is the pane it is stacked in. Where the parent
        // is a split the page specifies no fold for, that is a question about
        // the page's rows rather than about this variant.
        Op::FoldEnclosing(_) => &["Fold a bay away", "Fold a pane away"],
        // One row over both, which is the vocabulary's own reading:
        // `Unfold { region }` is `Some` for the named region and `None` for
        // everything folded — "two of its variants under one heading".
        Op::Unfold(_) => &["Bring back what is folded"],
        Op::UnfoldAll => &["Bring back what is folded"],
        // The same shape again: `Solo { region: None }` undoes the solo.
        Op::Solo(_) => &["Solo a region"],
        Op::Unsolo => &["Solo a region"],
        // The page's title, byte for byte. Renaming the row without renaming
        // it here is `Op::Reset` reaching for an operation nobody named, and
        // renaming it here without renaming the row is the same failure from
        // the other side; the two assertions below catch one direction each.
        Op::Reset => &["Reset the arrangement"],
        Op::Report => &[],
    }
}

/// The name [`NO_ROW`] knows a variant by.
fn name_of(op: Op) -> &'static str {
    match op {
        Op::Fold(_) => "Fold",
        Op::FoldEnclosing(_) => "FoldEnclosing",
        Op::Unfold(_) => "Unfold",
        Op::UnfoldAll => "UnfoldAll",
        Op::Solo(_) => "Solo",
        Op::Unsolo => "Unsolo",
        Op::Reset => "Reset",
        Op::Report => "Report",
    }
}

/// Every variant, once. The two matches above will not compile without a new
/// one; this list is what a new one also has to be added to, and
/// [`every_variant_is_in_the_list`] carries the floor that says so.
fn ops(id: NodeId) -> Vec<Op> {
    vec![
        Op::Fold(id),
        Op::FoldEnclosing(id),
        Op::Unfold(id),
        Op::UnfoldAll,
        Op::Solo(id),
        Op::Unsolo,
        Op::Reset,
        Op::Report,
    ]
}

fn panel() -> Panel {
    Panel::new(1280.0, 720.0)
}

/// A floor rather than a count, and it is here because the list above is the
/// one thing the compiler cannot check: `rows_of` and `name_of` refuse a new
/// variant, `ops` merely omits it. Raise this when one is added.
#[test]
fn every_variant_is_in_the_list() {
    let p = panel();
    let root = p.layout().root();
    assert!(
        ops(root).len() >= 8,
        "only {} operations listed — `Op` has more than this file knows about",
        ops(root).len()
    );
}

/// A row an operation names and the page does not have is a row that was
/// renamed, moved or deleted under the console — and the console would go on
/// claiming to reach it.
#[test]
fn every_row_an_operation_names_is_on_the_page() {
    let p = panel();
    let root = p.layout().root();
    let page: BTreeSet<String> = rows().into_iter().collect();
    assert!(
        !page.is_empty(),
        "no rows found under `{SECTION}` in {PAGE} — the scan matched nothing, which is not the \
         same as the section being empty"
    );
    for op in ops(root) {
        for row in rows_of(op) {
            assert!(
                page.contains(*row),
                "`Op::{}` names the row `{row}`, and {PAGE} has no such row under `{SECTION}` — \
                 the page is the specification, so this is the console reaching for an operation \
                 nobody named",
                name_of(op)
            );
        }
    }
}

/// The other direction: a row the console cannot perform. Every one of them is
/// in [`NO_OP`] with a reason, so a new row arrives as a failure rather than
/// as an operation the panel silently does not have.
#[test]
fn every_row_on_the_page_has_an_operation() {
    let p = panel();
    let root = p.layout().root();
    let reached: BTreeSet<&str> = ops(root)
        .into_iter()
        .flat_map(|op| rows_of(op).iter().copied())
        .collect();
    for row in rows() {
        assert!(
            reached.contains(row.as_str()) || NO_OP.contains(&row.as_str()),
            "{PAGE} specifies `{row}` under `{SECTION}` and no `Op` reaches it — give it one, or \
             add it to `NO_OP` here with the reason it is not an operation"
        );
    }
}

/// And the variants with no row are exactly [`NO_ROW`], both ways round: one
/// that gains a row should stop being an exception, and a new exception has to
/// be written down rather than discovered.
#[test]
fn the_variants_with_no_row_are_the_ones_written_down() {
    let p = panel();
    let root = p.layout().root();
    let unspecified: Vec<&str> = ops(root)
        .into_iter()
        .filter(|op| rows_of(*op).is_empty())
        .map(name_of)
        .collect();
    assert_eq!(
        unspecified, NO_ROW,
        "the operations {PAGE} does not name are not the ones this file says they are — an \
         operation the page never specified is one no surface but this crate can reach"
    );
}

/// **Two regions the vocabulary cannot say, and the pointer can.**
///
/// `Operation` names a region by `String`, and `Layout::name` answers `None`
/// for a split the arrangement left unnamed. There are exactly two of those —
/// the root column and the body row — and both are handed to a caller as
/// `Hit::Divider { split, .. }`, which `crates/karakuri/src/main.rs` turns into
/// `Op::Fold` of the split. So this is the cost of `Op` becoming `Operation`,
/// counted: it is two, and they are these.
///
/// **The decision was taken rather than left pending** (ADR-0204): the two
/// stay unnamed, so `Op` stays `Op`, and this assertion is the standing price
/// rather than a note that somebody still has to choose. Naming either of them
/// is what fails here, and it should: it would be asserting that folding the
/// whole panel away, or folding the row of three panes, is an operation an
/// operator asks for — and the page says the opposite twice, once by having no
/// row for either and once by reaching the outcome an operator does want
/// through *Solo a region*.
///
/// The count is asserted rather than the list alone, because a third unnamed
/// split would be a third region only a mouse could fold, arriving without
/// anybody deciding it should.
#[test]
fn exactly_two_splits_have_no_name_for_an_operation_to_use() {
    let p = panel();
    let l = p.layout();
    let unnamed: Vec<NodeId> = p
        .nodes()
        .iter()
        .map(|n| n.id)
        .filter(|id| l.name(*id).is_none())
        .collect();
    let root = l.root();
    // The body row is the root's second child: the transport, the row of three
    // columns, and the outputs row.
    let body = l.children(root)[1];
    assert_eq!(
        unnamed,
        vec![root, body],
        "the arrangement's unnamed splits are not the two this file knows about — every other \
         node is reachable by name from a MIDI map or an MCP call, and one that is not is a \
         region only the pointer can fold"
    );
    assert!(l.name(root).is_none() && l.name(body).is_none());
}
