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

/// Rows in the section that no [`Op`] reaches (boundary dragging is a positional gesture,
/// and saving/restoring arrangements requires external storage; ADR-0156, ADR-0221).
pub(crate) const NO_OP: &[&str] = &[
    "Move a boundary",
    "Save the arrangement",
    "Put a saved arrangement back",
];

/// [`Op`] variants that no row names. [`Op::Report`] returns internal placement data
/// unrepresentable in the general vocabulary (ADR-0205).
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

/// Minimum boundary displacement in pixels required to register layout movement.
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

/// Maps each [`Op`] variant to its corresponding manual row titles, or [`NO_ROW`].
pub(crate) fn rows_of(op: Op) -> &'static [&'static str] {
    match op {
        Op::Fold(_) => &["Fold a bay away", "Fold a pane away"],
        // Folds parent node (bay or pane) determined dynamically via `layout.parent(id)`.
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
