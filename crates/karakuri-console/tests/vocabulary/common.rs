pub(crate) use std::collections::BTreeSet;
pub(crate) use std::fs;
pub(crate) use std::path::{Path, PathBuf};

pub(crate) use super::common::{drawn_once, running, showing};
pub(crate) use karakuri_console::egui;
pub(crate) use karakuri_console::input::{claim, Claim};
pub(crate) use karakuri_console::panel::{Dragged, Op, Outcome, Panel, Pressed};
pub(crate) use karakuri_console::view::{
    arrangement, bay_grip, program_head, Ask, BAY_GRIPS, REGIONS,
};
pub(crate) use karakuri_layout::{Axis, NodeId, Point};
pub(crate) use karakuri_operation::gate::Open;

/// The specification, relative to the workspace root.
pub(crate) const PAGE: &str = "docs/manual/operations.html";

/// The section of it this crate answers for.
pub(crate) const SECTION: &str = "<h2>Arranging the console</h2>";

/// What marks a row on that page — the same marker `karakuri-operation`'s test
/// matches, and for the same reason: sections are `<h2>` and a heading somebody
/// adds for looks is neither.
pub(crate) const ROW: &str = r#"<div class="op-head">"#;

/// Rows in the section that no [`Op`] reaches, with why.
///
/// A drag is not an operation. Moving a boundary is
/// `Panel::press`/`moved`/`released` over `Layout::hit`, and it is a gesture
/// with a position in it rather than something a caller can ask for by name.
/// `Operation::MoveBoundary` carries `Undecided` and gives the same two
/// reasons: more than half the boundaries here belong to a split the
/// arrangement left unnamed, and `Layout::set_divider` takes a pixel.
///
/// Two more, and their reason is a payload rather than a gesture. *Save the
/// arrangement* and *Put a saved arrangement back* each carry a name the
/// operator picked, and what stands behind that name is a file under the store
/// — `arrangements/<name>.arrangement.json`
/// (`docs/adr/0221-an-arrangement-is-named-by-the-operator-and-kept-in-a-fourth-place.md`).
/// This crate has no store and cannot have one (ADR-0156), so neither row can
/// be an `Op`: an `Op` is `Copy`, names a [`NodeId`] or nothing, and could not
/// carry a whole arrangement even if it wanted to. The operator's operations
/// are `karakuri_operation::Operation::SaveArrangement` and
/// `RestoreArrangement`, whoever holds the store performs them, and the half
/// that lands here is `Panel::layout` on the way out and
/// [`karakuri_console::panel::Panel::restore`] on the way back — a method
/// rather than a variant, and its own documentation says why.
///
/// So this list is now two different reasons under one name: a drag is not an
/// operation, and an operation whose payload only a third party can produce is
/// not this type's. A row landing here for a third reason wants that reason
/// written down beside these two.
pub(crate) const NO_OP: &[&str] = &[
    "Move a boundary",
    "Save the arrangement",
    "Put a saved arrangement back",
];

/// [`Op`] variants that no row names, with why.
///
/// One entry, and it is permanent. [`Op::Report`] is a question whose reply the
/// vocabulary cannot say: `Outcome::Report` is a `Vec<Placement>`, and a
/// `Placement` carries a [`NodeId`] and a pixel `Rect` — a handle no surface
/// but this crate can hold, in the coordinates of a window a model is not
/// looking at. A row for it would promise three surfaces a reply that does not
/// cross, which is not the same as three routes nobody has built yet
/// (ADR-0205).
///
/// And no surface binds a key to it any more, since 2026-08-31: `p` was the
/// window's shortcut for it and `p` is half of the pair
/// `docs/manual/operations.html` specifies for *Nudge the latency offset*. That
/// changes nothing here — this file is about which rows the page has, and a key
/// was never one — but it is why the variant's own documentation now says what
/// still reads it. This entry is unaffected either way: the reply still cannot
/// be said in the vocabulary's terms, whoever asks.
///
/// [`Op::Reset`] was the other entry and is not one any more. It is a change
/// rather than a question, it names no target, and *Reset the arrangement* is
/// its row (ADR-0208) — so what this file pins about it is now the mapping in
/// [`rows_of`] rather than its absence, and putting it back here fails
/// [`the_variants_with_no_row_are_the_ones_written_down`] in one direction and
/// [`every_row_on_the_page_has_an_operation`] in the other.
pub(crate) const NO_ROW: &[&str] = &["Report"];

/// The badge text of a route that names nowhere. `panel_column.rs` holds the
/// same constant for the same reason: a `plan` or a `gap` badge is allowed to
/// be this — four in this section are — and a `has` badge is not, because it
/// would claim an operator reaches the operation and decline to say from where.
pub(crate) const NOWHERE: &str = "&mdash;";

/// How far apart the sweep's presses are, in the panel's own pixels, and the
/// whole of what makes it affordable: 8 over a 1280 x 720 console is 14,400
/// presses. See the header for what a control smaller than this in both
/// directions would cost, and why nothing on the console is.
pub(crate) const STEP: f32 = 8.0;

/// How far a press drags before it lets go, along the axis of whatever it took
/// hold of. Wider than a divider's own gap, so a drag that grabbed one asks it
/// to go somewhere it is not already.
pub(crate) const DRAG: f32 = 24.0;

/// What a boundary has to move by to count as moved. A drag reports itself when
/// the *layout* does something rather than when the pointer does, so this is
/// read off the region beside the boundary instead: a whole pixel is more than
/// the half-pixel `Panel::moved` thinks is worth saying and far less than
/// [`DRAG`].
pub(crate) const MOVED: f32 = 1.0;

pub(crate) fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

pub(crate) fn page() -> String {
    let path = workspace().join(PAGE);
    fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "{} is the specification and could not be read: {e}",
            path.display()
        )
    })
}

/// Every `<h3>` of a row in *Arranging the console*, in page order.
pub(crate) fn rows() -> Vec<String> {
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

/// Which rows an operation lands on. One match over every variant, so a variant
/// added to [`Op`] does not compile until somebody has said which row it is —
/// or that it is none, which is [`NO_ROW`] and wants a reason there.
///
/// More than one variant may name one row, and one variant may name two:
/// [`Op::Fold`] folds whatever node it is handed, and the page draws the bay
/// and the pane as separate rows because it describes two different
/// consequences. Whether they are one operation over two kinds of region is an
/// open question at `Operation::FoldPane` itself.
pub(crate) fn rows_of(op: Op) -> &'static [&'static str] {
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
pub(crate) fn name_of(op: Op) -> &'static str {
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
pub(crate) fn ops(id: NodeId) -> Vec<Op> {
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

pub(crate) fn panel() -> Panel {
    Panel::new(1280.0, 720.0)
}
