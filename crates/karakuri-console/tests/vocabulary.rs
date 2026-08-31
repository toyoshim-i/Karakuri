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
//! - **Three rows have no `Op`**: *Move a boundary*, which is
//!   [`karakuri_console::panel::Panel::press`], `moved` and `released` — a
//!   gesture rather than an operation, and the vocabulary carries it as
//!   `Undecided` for the same reason — and the two that carry a name, *Save
//!   the arrangement* and *Put a saved arrangement back*, whose payload is a
//!   file under a store this crate cannot reach. [`NO_OP`] holds all three
//!   with the two reasons written out.
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
//!
//! # The panel column of this section, and what the pointer actually reaches
//!
//! Everything above is about *which operations exist*. The second half of this
//! file is about a different question on the same six rows —
//! [ADR-0213](../../../docs/adr/0213-the-interface-milestones-meter-is-the-panel-column-and-has-means-an-operator-reaches-it.md)
//! made the **panel column** of this page the Interface milestone's meter and
//! defined its badge: *`has` means an operator running the instrument reaches
//! the operation*. [`panel_column.rs`](../tests/panel_column.rs) is the pair
//! that flip owed, and the last thing its header says is what it cannot see:
//! these rows reach an operator through [`Op`] and never through
//! `karakuri_operation::Operation`, so a scan for `Operation::` finds none of
//! them. **This is where that column meets this type**, and it names the
//! variants outright where that file has to read its own crate as text.
//!
//! **What the pointer reaches is demonstrated rather than declared.** A list
//! here of which rows have a control would be a second copy of one
//! ([P-0045](../../../docs/principles/0045-generate-the-vocabulary-prose-drifts-from-code.md)),
//! and a copy of something nothing states. So [`reached_by_the_pointer`] asks
//! a running panel, in **four passes**, because the pointer has two routes
//! into this section and not one:
//!
//! 1. It presses every boundary the arrangement has, at the middle of that
//!    boundary's own gap, and drags it — that is *Move a boundary*,
//!    demonstrated, and the extent of the region beside it is read back either
//!    side.
//! 2. It presses, drags and releases at every point of a [`STEP`]-pixel grid
//!    over the whole console, and compares what is folded and what is soloed
//!    against what they were.
//! 3. It drives the **console's own painted control** the way the window loop
//!    drives one — [`claim`], then the derivation that drew it, then the
//!    caller performing the answer — over the transport row's arrangement pill
//!    and every row of its menu. See [`reached_through_a_painted_control`].
//! 4. It drives the second such control the same way: the `solo` pill in the
//!    Program bay's head, pressed twice, because it answers with two
//!    operations and the second is the one nothing else can reach. See
//!    [`reached_through_the_program_bays_head`].
//!
//! **The first two routes are `Panel::press`, `moved` and `released`, and the
//! last two are none of them.** That sentence used to read *the pointer's whole
//! route into this section is `Panel::press`, `moved` and `released`*, and it
//! stopped being true the day the arrangement pill was drawn: a press on a
//! painted control never enters `Panel` at all until the caller has already
//! decided what it asks for. The two are kept apart rather than merged,
//! because a grid sweep and a control are answered by different machinery and
//! fail in different directions.
//!
//! **Five rows of this section an operator reaches, and two of them are rows
//! an [`Op`] names.** *Move a boundary* is in [`NO_OP`] because a drag is a
//! gesture rather than an operation; *Save the arrangement* and *Put a saved
//! arrangement back* are in it because their payload is a file under a store
//! this crate cannot reach; *Reset the arrangement* is [`Op::Reset`] and is the
//! one the third pass performs rather than only names; *Solo a region* is
//! [`Op::Solo`] and [`Op::Unsolo`], and the fourth pass performs both. Which operations the
//! console can be *asked* for and what a hand on the panel can *do* are
//! different questions, and this file pins both.
//!
//! # What the sweep cannot see, and which way each one fails
//!
//! - **Reachability is `crates/karakuri/src/main.rs`'s, and this crate cannot
//!   depend on it** (ADR-0156). The window loop is what turns a `winit` press
//!   into [`Panel::press`] and draws the panel in front of somebody, and
//!   ADR-0213's definition is P-0030's sentence — *a window that opens and
//!   cannot be touched is a demo, not a tool*. **So this file checks the
//!   necessary half and not the sufficient one**, exactly as `panel_column.rs`
//!   does one column along. The sufficient half for the one `has` row is a
//!   test in that binary rather than a claim here:
//!   `a_drag_through_the_window_loops_own_routing_never_reaches_egui` grabs a
//!   boundary, drags it and lets go through `Readout::pointer` — the window
//!   loop's own routing — and reads the left pane's width back either side.
//!   A gesture this file demonstrates and that binary never wires would pass
//!   here, and the badge would be a lie the page tells on its own authority.
//! - **A control this crate draws and a caller applies is invisible to the
//!   sweep**, because the press never goes through [`Panel::press`]. **There
//!   are three of them, and the third and fourth passes are what two of them
//!   cost.** The
//!   Outputs row's sink chip (`view::Outputs::op`) answers [`Op::Fold`] or
//!   [`Op::Unfold`] for the picture; the transport row's arrangement pill
//!   (`view::ArrangementPill::ask`) answers an [`Op`], one of two
//!   `karakuri_operation::Operation`s, or a move of its own menu; and the
//!   Program bay head's `solo` (`view::ProgramHead::op`) answers [`Op::Solo`]
//!   of the picture or [`Op::Unsolo`].
//!
//!   The sink is **not this section's** — the page gives that control its own
//!   row, *Choose where the frame goes*, in *Output and recording* — and
//!   **nothing in this workspace checks that row's badge**. This file is not
//!   it, and says so rather than being read as covering the column. The pill
//!   **is** this section's, on three rows, which is why it is demonstrated
//!   here instead of being written off as something the sweep cannot see: the
//!   sweep still cannot, and [`reached_through_a_painted_control`] is the
//!   answer to that rather than a widening of the grid.
//! - **A control smaller than [`STEP`] in both directions** could sit between
//!   two presses. Nothing on the console is: a divider is [`Panel::press`]'s
//!   grab width either side of a gap, and a bay head is 27 tall. It is a
//!   *false negative* — a fold nobody demonstrated — and it fails the
//!   direction that says a `has` badge is reached, from the other side, the
//!   moment somebody flips the badge for the control they just drew.
//! - **Which row a new fold lands on.** The sweep can say that something
//!   folded and not whether what folded was a bay or a pane, so it panics
//!   naming the point and the node rather than guessing a row — the shape
//!   `panel_column.rs`'s `sample` uses for the same reason.
//! - **A badge whose home is wrong.** A `has` badge has to name a home and the
//!   home is checked for being *something*, as `mcp.rs` and `panel_column.rs`
//!   both check it; that the region it names is where the control actually is
//!   is a claim about `docs/manual/console.html` and is nobody's test here.

mod common;

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use common::{drawn_once, running, showing};
use karakuri_console::input::{claim, Claim};
use karakuri_console::panel::{Dragged, Op, Outcome, Panel, Pressed};
use karakuri_console::view::{arrangement, program_head, Ask};
use karakuri_layout::{Axis, NodeId, Point};

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
///
/// **Two more, and their reason is a payload rather than a gesture.** *Save
/// the arrangement* and *Put a saved arrangement back* each carry a name the
/// operator picked, and what stands behind that name is a file under the store
/// — `arrangements/<name>.arrangement.json`
/// (`docs/adr/0221-an-arrangement-is-named-by-the-operator-and-kept-in-a-fourth-place.md`).
/// **This crate has no store and cannot have one** (ADR-0156), so neither row
/// can be an `Op`: an `Op` is `Copy`, names a [`NodeId`] or nothing, and could
/// not carry a whole arrangement even if it wanted to. The operator's
/// operations are `karakuri_operation::Operation::SaveArrangement` and
/// `RestoreArrangement`, whoever holds the store performs them, and the half
/// that lands here is `Panel::layout` on the way out and
/// [`karakuri_console::panel::Panel::restore`] on the way back — a method
/// rather than a variant, and its own documentation says why.
///
/// **So this list is now two different reasons under one name**: a drag is not
/// an operation, and an operation whose payload only a third party can produce
/// is not this type's. A row landing here for a third reason wants that reason
/// written down beside these two.
const NO_OP: &[&str] = &[
    "Move a boundary",
    "Save the arrangement",
    "Put a saved arrangement back",
];

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
/// **And no surface binds a key to it any more**, since 2026-08-31: `p` was
/// the window's shortcut for it and `p` is half of the pair
/// `docs/manual/operations.html` specifies for *Nudge the latency offset*.
/// That changes nothing here — this file is about which **rows** the page has,
/// and a key was never one — but it is why the variant's own documentation now
/// says what still reads it. This entry is unaffected either way: the reply
/// still cannot be said in the vocabulary's terms, whoever asks.
///
/// **[`Op::Reset`] was the other entry and is not one any more.** It is a
/// change rather than a question, it names no target, and *Reset the
/// arrangement* is its row (ADR-0208) — so what this file pins about it is now
/// the mapping in [`rows_of`] rather than its absence, and putting it back
/// here fails [`the_variants_with_no_row_are_the_ones_written_down`] in one
/// direction and [`every_row_on_the_page_has_an_operation`] in the other.
const NO_ROW: &[&str] = &["Report"];

/// The badge text of a route that names nowhere. `panel_column.rs` holds the
/// same constant for the same reason: a `plan` or a `gap` badge is allowed to
/// be this — four in this section are — and a `has` badge is not, because it
/// would claim an operator reaches the operation and decline to say from
/// where.
const NOWHERE: &str = "&mdash;";

/// How far apart the sweep's presses are, in the panel's own pixels, and the
/// whole of what makes it affordable: 8 over a 1280 x 720 console is 14,400
/// presses. See the header for what a control smaller than this in both
/// directions would cost, and why nothing on the console is.
const STEP: f32 = 8.0;

/// How far a press drags before it lets go, along the axis of whatever it took
/// hold of. Wider than a divider's own gap, so a drag that grabbed one asks it
/// to go somewhere it is not already.
const DRAG: f32 = 24.0;

/// What a boundary has to move by to count as moved. A drag reports itself
/// when the *layout* does something rather than when the pointer does, so this
/// is read off the region beside the boundary instead: a whole pixel is more
/// than the half-pixel `Panel::moved` thinks is worth saying and far less than
/// [`DRAG`].
const MOVED: f32 = 1.0;

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

/// **The section this crate answers for, as text.** Sliced once so that a
/// badge found past the section's end belongs to another section's row, which
/// is the same cut [`rows`] makes for the same reason.
fn arranging() -> String {
    let text = page();
    let start = text.find(SECTION).unwrap_or_else(|| {
        panic!("`{SECTION}` is gone from {PAGE} — the section this crate answers for")
    });
    let rest = &text[start + SECTION.len()..];
    let end = rest.find("<h2").unwrap_or(rest.len());
    rest[..end].to_owned()
}

/// **Every row of the section with its panel badge**: the title, the badge's
/// class — `has`, `plan` or `gap` — and the text it names the control's home
/// with.
///
/// Read verbatim and never decoded, which is `mcp.rs`'s rule and
/// `panel_column.rs`'s after it: a badge that names nowhere says `&mdash;`,
/// and a home that needed decoding to match would be a home nobody could find
/// on the console page.
fn panel_badges() -> Vec<(String, String, String)> {
    let mut found = Vec::new();
    for part in arranging().split(ROW).skip(1) {
        let open = part.find("<h3>").expect("a row opens with its heading");
        let close = part[open..].find("</h3>").expect("a heading closes");
        let title = part[open + 4..open + close].to_owned();
        let body = &part[open + close..];
        let mut badge = None;
        for span in body.split(r#"<span class="rt "#).skip(1) {
            let Some(quote) = span.find('"') else {
                continue;
            };
            let class = span[..quote].to_owned();
            let Some(text) = span[quote..].strip_prefix(r#"">panel <b>"#) else {
                continue;
            };
            let Some(shut) = text.find("</b>") else {
                continue;
            };
            badge = Some((class, text[..shut].to_owned()));
            break;
        }
        let (class, home) = badge.unwrap_or_else(|| {
            panic!(
                "`{title}` under `{SECTION}` in {PAGE} has no panel badge — every row carries \
                 one, and a row that has stopped is a row this file stops measuring"
            )
        });
        found.push((title, class, home));
    }
    found
}

/// **What is folded and what is soloed**, which is the whole of what an
/// operation in this section can change about an arrangement that nothing has
/// resized. A boundary drag moves rectangles and leaves this alone, which is
/// why the drag is demonstrated separately and this is what the sweep compares.
fn shape(p: &mut Panel) -> (Vec<bool>, bool) {
    p.solve();
    let ids: Vec<NodeId> = p.nodes().iter().map(|n| n.id).collect();
    let layout = p.layout();
    (
        ids.iter().map(|id| layout.visible(*id)).collect(),
        layout.is_soloed(),
    )
}

/// The point in the middle of a boundary's own gap: along the split's axis,
/// half way between the two regions it is between, and across it, half way
/// down the first of them.
fn on_the_boundary(p: &mut Panel, split: NodeId, index: usize) -> Option<(Axis, Point, NodeId)> {
    p.solve();
    let axis = p.layout().axis(split)?;
    let (a, b) = p.pair(split, index)?;
    let (ra, rb) = (p.layout().rect(a), p.layout().rect(b));
    let along = (axis.far(ra) + axis.origin(rb)) * 0.5;
    Some((
        axis,
        match axis {
            Axis::Row => Point::new(along, ra.y + ra.h * 0.5),
            Axis::Column => Point::new(ra.x + ra.w * 0.5, along),
        },
        a,
    ))
}

/// **The arrangement this pass tells the pill is in use**, and the one name it
/// tells it is filed. Any name a store would accept; what matters is that
/// there is one, which is what makes *save* name a file rather than ask for
/// letters and what puts a row in the list to pick.
const IN_USE: &str = "night";

/// A `karakuri_layout` point, from `egui`'s. The console's controls are laid
/// out in `egui`'s rectangles and hit-tested in the layout's points, and this
/// is the one step between them.
fn point_of(p: egui::Pos2) -> Point {
    Point::new(p.x, p.y)
}

/// `at` moved [`DRAG`] along `axis`.
fn dragged_to(axis: Axis, at: Point) -> Point {
    match axis {
        Axis::Row => Point::new(at.x + DRAG, at.y),
        Axis::Column => Point::new(at.x, at.y + DRAG),
    }
}

/// **Every row of this section a hand on the panel reaches**, demonstrated on
/// a running [`Panel`] rather than listed here — see the header.
///
/// Three passes. The first presses every boundary the arrangement has and
/// drags it, and reads the region beside it back either side: that is *Move a
/// boundary*, and it is in the answer only if a boundary actually moved. The
/// second presses, drags and releases at every point of a [`STEP`] grid over
/// the whole console and compares [`shape`] against what it was. The third is
/// [`reached_through_a_painted_control`], which is a different route and not
/// a finer grid.
///
/// # What the grid still asserts, now that a press *can* reset the console
///
/// **Nothing reachable through [`Panel::press`] folds, unfolds or solos**, and
/// that is the invariant the second pass compares [`shape`] for. It is
/// narrower than it was and it is still the right one: the arrangement pill
/// can now reset the whole console from a press, but not through this route —
/// `Panel::press` hit-tests boundaries and regions and knows nothing about a
/// painted control, so **the grid cannot land on the pill however fine it
/// gets**. A `shape` that changed under this sweep would therefore still be
/// what it always was: a control nobody accounted for, reached by a route
/// nobody meant to open. It panics naming where the press was rather than
/// guessing which of the section's rows it lands on.
///
/// So the two passes are not two grids of different resolution. **A control
/// this crate paints is invisible to any grid driven through `Panel::press`**,
/// which is exactly why the third pass drives the other route by hand.
fn reached_by_the_pointer() -> BTreeSet<&'static str> {
    let mut reached = BTreeSet::new();

    let mut p = panel();
    p.solve();
    let boundaries: Vec<(NodeId, usize)> = p.layout().boundaries().collect();
    assert!(
        boundaries.len() >= 5,
        "only {} boundaries in the arrangement — a sweep with nothing to drag would demonstrate \
         nothing and every assertion below would pass on an empty answer",
        boundaries.len()
    );
    for (split, index) in boundaries {
        let mut p = panel();
        let Some((axis, at, beside)) = on_the_boundary(&mut p, split, index) else {
            continue;
        };
        let was = axis.extent(p.layout().rect(beside));
        if !matches!(p.press(at), Pressed::Grabbed { .. }) {
            continue;
        }
        let dragged = matches!(
            p.moved(dragged_to(axis, at)),
            Some(Dragged::Boundary { .. })
        );
        p.released();
        p.solve();
        let now = axis.extent(p.layout().rect(beside));
        if dragged && (now - was).abs() >= MOVED {
            reached.insert("Move a boundary");
        }
    }

    let mut p = panel();
    let before = shape(&mut p);
    let viewport = p.layout().viewport();
    let mut grabs = 0usize;
    let mut y = viewport.y;
    while y < viewport.y + viewport.h {
        let mut x = viewport.x;
        while x < viewport.x + viewport.w {
            let at = Point::new(x, y);
            let grabbed = matches!(p.press(at), Pressed::Grabbed { .. });
            // Both ways, because a control that acts on the move rather than
            // on the press does not say which axis it is watching.
            p.moved(dragged_to(Axis::Row, at));
            p.moved(dragged_to(Axis::Column, at));
            p.released();
            let now = shape(&mut p);
            assert_eq!(
                now, before,
                "a press at ({x}, {y}) folded, unfolded or soloed something, and nothing \
                 reachable through `Panel::press` is supposed to: this route is `press`, \
                 `moved` and `released` over `Layout::hit`, and none of the three changes what \
                 is folded. A control this crate *paints* cannot be reached from here at all — \
                 see `reached_through_a_painted_control` — so this is a new route rather than \
                 a new control. Say which row of `{SECTION}` it lands on, add it to \
                 `reached_by_the_pointer`, and flip that row's panel badge"
            );
            // A drag moved a boundary, so the arrangement the next press lands
            // on is not the one this pass started from.
            if grabbed {
                grabs += 1;
                p = panel();
            }
            x += STEP;
        }
        y += STEP;
    }
    // **The floor that says the grid found the console rather than missed
    // it.** A sweep whose points all landed outside the viewport, or whose
    // step had grown past a divider's grab width, would compare `shape`
    // against itself and report every row unreached — which passes one
    // direction and fails nothing. Every boundary above is a run of points a
    // press takes hold at, so a grid that covers the panel grabs many more
    // than there are boundaries.
    assert!(
        grabs >= 5,
        "the sweep pressed the whole viewport and took hold of a boundary {grabs} times — a \
         grid this coarse is not pressing the console"
    );

    reached.extend(reached_through_a_painted_control());
    reached.extend(reached_through_the_program_bays_head());
    reached
}

/// **The third pass: the rows a *painted control* reaches, driven the way the
/// window loop drives one.**
///
/// The two passes above go through [`Panel::press`], and that is the whole of
/// what they can see. A control this crate paints is reached by a different
/// route with three steps in it — [`claim`] says the press is the panel's, the
/// derivation that drew the control says what the press asks for, and the
/// **caller** performs whatever that turns out to be — and not one of the
/// three is `Panel::press`. So the sweep is blind to a painted control whether
/// or not it exists, which is not a gap in the sweep: it is what
/// `crates/karakuri/src/main.rs` does with an event, written down in the one
/// place this crate can run it.
///
/// **This demonstrates rather than declares, exactly as the sweep does.** It
/// lays the arrangement pill out at the size the panel is really solved at,
/// asks [`claim`] who a press on it belongs to, asks the pill what that press
/// asks for, opens the menu with the answer, and then asks every row of the
/// open menu the same question. A list here saying *the pill reaches the
/// reset* would be the second copy of something nothing states (P-0045), and
/// it would go on being true after the arm was deleted.
///
/// # Three rows come out of one menu, and they are not all reached the same way
///
/// - ***Start a new one* is an [`Op`]**, so it is performed here and read back:
///   the console is folded about first, the `Op` the row asks for is put
///   through [`Panel::op`], and [`shape`] has to come back to what a fresh
///   panel's was. A reset that reset nothing would be a row demonstrated and
///   not reached.
/// - ***Save* and a filed name are `karakuri_operation::Operation`s**, and
///   [`NO_OP`] is where the reason is: their payload is a file under a store
///   **this crate cannot reach** (ADR-0156). So what is demonstrated for those
///   two is the whole of what this crate has — that a press on a row of a
///   drawn menu comes back as that operation, named — and who performs it is
///   `crates/karakuri`'s, which is the same split the header states for *Move
///   a boundary*: this file checks the necessary half and never the
///   sufficient one.
///
/// **The row's title comes from the operation rather than from this file** —
/// [`rows_of`] for an `Op` and `Operation::title` for an `Operation`, which is
/// the same call `panel_column.rs` makes. A menu that reordered itself is
/// still measured, and a row whose ask changed is not silently read as the one
/// it used to be.
///
/// # The store's answer is handed in, and that is the seam rather than a prop
///
/// The pill is asked with **an arrangement in use and one name filed**,
/// because that is what decides two of the three asks: with nothing in use
/// *save* asks for letters instead of naming a file, and with nothing filed
/// there is no name to pick. Neither of those is the console being coy — a
/// name and a listing are a file and a directory, and `src/` has no disk, so
/// they arrive per frame from whoever does, exactly as
/// [`karakuri_console::view::View::transport`] does. Handing them in here is
/// this test standing where `crates/karakuri` stands.
///
/// **What that leaves uncovered is named rather than implied**: on a console
/// with *no* arrangement in use, *save* answers `Ask::Name` and the operation
/// arrives only after a name is typed and committed — which is a keyboard, and
/// this crate has none. That half is `crates/karakuri`'s
/// `a_finished_name_is_the_save_the_menu_would_have_asked_for`.
///
/// # Which failure is loud and which one is quiet, and why they differ
///
/// **Everything that would mean the demonstration is not running panics**, and
/// the message names what stopped being checked: no pill laid out, a press
/// [`claim`] does not give the panel, a pill that will not open, an open menu
/// with no rows in it, folds that changed nothing to forget, or a picked `Op`
/// that said it reset and did not.
///
/// **The one thing that does not panic is the answer itself.** If a row stops
/// asking for what it asked for, nothing is inserted for it and
/// [`every_arrangement_row_marked_built_is_reached_by_the_pointer`] is what
/// fails — naming the row, and offering the two readings it always offers:
/// *either the control went and the badge is `plan` again, or it was never on
/// the panel*. That is the direction this pass exists to serve, so it is left
/// to the assertion written for it rather than pre-empted here.
fn reached_through_a_painted_control() -> BTreeSet<&'static str> {
    let mut reached = BTreeSet::new();

    // **Something to forget.** A reset on a console that is already the
    // default changes nothing, so a pass that started there could not tell a
    // reset from a press that did nothing at all.
    //
    // **Two folds and no solo, and that is a fact about the control rather
    // than a convenience.** `Layout::solo` collapses everything off the solo's
    // path, so any solo but one on the transport row itself takes that row off
    // the screen — and a row that is not drawn has no pill in it. The reset
    // forgets a solo as well as a fold, but a hand cannot ask *this* control
    // for it while one is in force; that is the Outputs row's situation one
    // row up (`tests/outputs.rs`), and it belongs to the arrangement rather
    // than to this pass.
    let mut p = panel();
    let default = shape(&mut p);
    for bay in ["staging", "sequencer"] {
        let id = p
            .layout()
            .find(bay)
            .unwrap_or_else(|| panic!("a {bay} bay"));
        p.op(Op::Fold(id));
    }
    p.solve();
    assert_ne!(
        shape(&mut p),
        default,
        "the folds this pass makes to have something to forget changed nothing, so a reset \
         performed below would be indistinguishable from a press that did nothing"
    );

    // The pill is as wide as the name in it and sits one gap after the bar, so
    // it needs `egui`'s fonts and it needs a console with an engine behind it
    // — see `common::running`, and `view::arrangement` for why a row that is
    // not drawn has no control in it. The name and the listing are the store's
    // answer, handed in.
    let ctx = drawn_once();
    let mut view = running();
    view.arrangement.name = Some(IN_USE.to_owned());
    view.arrangement.filed = vec![IN_USE.to_owned()];
    let pill = arrangement(&ctx, p.layout(), view.transport, None, &view.arrangement)
        .unwrap_or_else(|| {
            panic!(
                "the transport row draws no arrangement pill on a solved console with an \
                 engine behind it, so nothing here can demonstrate a row of `{SECTION}` from a \
                 pointer — this pass has stopped measuring rather than found the control gone"
            )
        });

    // **Step one of the real route.** `claim` is what the window loop asks
    // before anything acts, and a press it hands to `egui` never reaches the
    // control at all.
    let on_the_pill = point_of(pill.pill.center());
    assert_eq!(
        claim(&mut p, &ctx, &view, on_the_pill),
        Claim::Panel,
        "`claim` gives a press on the arrangement pill to `egui`, so no route into `{SECTION}` \
         from this control exists however it is drawn"
    );

    // **Step two**: what the press asks for, off the same derivation `claim`
    // hit-tested. Opening a menu is not an operation and never will be — no
    // MIDI map and no MCP call could want to say it — so this arm is the
    // affordance, and the rows below are where the vocabulary starts.
    match pill.ask(&view.arrangement, on_the_pill) {
        Some(Ask::Open) => view.arrangement.opened(),
        other => panic!(
            "a press on the arrangement pill asks for {other:?} rather than opening its menu, \
             so the rows this pass reads `{SECTION}` off are unreachable"
        ),
    }

    let open = arrangement(&ctx, p.layout(), view.transport, None, &view.arrangement)
        .expect("the pill was drawn a moment ago and the panel has not moved");
    assert!(
        open.rows > 0,
        "the arrangement pill's menu opened with no rows in it, so every ask below is asked of \
         nothing and this pass would report every row unreached whatever the control does"
    );

    // **Step three**: the caller performs it, which is what an `Op` arriving
    // from a control means — the pill holds no authority and applies nothing
    // (P-0076).
    for row in 0..open.rows {
        match open.ask(&view.arrangement, point_of(open.row(row).center())) {
            Some(Ask::Panel(op)) => {
                let outcome = p.op(op);
                p.solve();
                // **The outcome first, because it is the discriminating
                // claim.** `Op::UnfoldAll` on a console this pass folded and
                // did not solo puts `shape` back to exactly the default too,
                // so a row that started asking for the unfold would satisfy
                // the comparison below and be recorded as the reset. What
                // separates them is what the operation *said* it did, which is
                // also what the program prints and what `repaint::Change`
                // reads.
                assert_eq!(
                    outcome,
                    Outcome::Reset,
                    "menu row {row} asked for `{op:?}`, and this pass takes an `Op` from a menu \
                     row as `Reset the arrangement` being reached — so an `Op` that is not the \
                     reset is a row asking for something nobody accounted for"
                );
                assert_eq!(
                    shape(&mut p),
                    default,
                    "menu row {row} asked for `{op:?}` and performing it left the console \
                     folded — the row was demonstrated and it did not do what its row on \
                     {PAGE} says, which is a worse answer than not reaching it at all"
                );
                reached.extend(rows_of(op));
            }
            // Named, not performed: the payload is a file and this crate has
            // no store. See this function's header, and `NO_OP`.
            Some(Ask::Operation(operation)) => {
                reached.insert(operation.title());
            }
            // *Save* with nothing in use, and a press on the card that is on
            // no row. Neither names an operation, and neither is a failure —
            // the first is the one flow that asks for letters, and this pass
            // hands in a name in use so that it does not arrive.
            Some(Ask::Name) | Some(Ask::Open) | Some(Ask::Shut) | None => {}
        }
    }
    reached
}

/// **The fourth pass: the `solo` pill in the Program bay's head**, driven the
/// way the window loop drives one.
///
/// It is a painted control like the arrangement pill, so the grid is blind to
/// it for the reason [`reached_through_a_painted_control`] gives at length,
/// and it is a pass of its own rather than an arm of that one because the two
/// controls are answered by different machinery and fail in different
/// directions: the pill hangs a menu over the console and this is one capsule
/// with two answers.
///
/// **Both answers are performed and read back**, which is what makes this a
/// demonstration rather than a naming. `ProgramHead::op` is
/// [`Outputs::op`](karakuri_console::view::Outputs::op)'s shape — two
/// operations chosen from the layout, never a toggle — so the pass presses it
/// twice: once on a console with nothing soloed, which has to come back
/// [`Outcome::Soloed`] and leave [`shape`] changed, and once on the console
/// that left, which has to come back [`Outcome::Unsoloed`] and put `shape`
/// back to exactly what a fresh panel's was. A pill that soloed and could not
/// undo it would be half a row demonstrated, and the undo is the half a
/// pointer has no other way to reach: a solo takes every other region off the
/// screen, so nothing else is left to press.
///
/// **The second press is asked of a freshly derived head**, not of the one the
/// first press came from. The solo moves every rectangle on the console, the
/// bay is the window afterwards, and a capsule remembered across it would be a
/// press somewhere the pill no longer is — which is [`claim`]'s own rule about
/// asking the derivation again rather than storing its answer.
///
/// **What panics and what does not** is [`reached_through_a_painted_control`]'s
/// division exactly: everything that would mean the demonstration is not
/// running says so out loud — no head laid out, a press [`claim`] does not
/// give the panel, an operation that said it soloed and did not — and the
/// answer itself is left to
/// [`every_arrangement_row_marked_built_is_reached_by_the_pointer`].
fn reached_through_the_program_bays_head() -> BTreeSet<&'static str> {
    let mut reached = BTreeSet::new();

    let ctx = drawn_once();
    let view = showing(&[]);
    let mut p = panel();
    let default = shape(&mut p);

    // **Step one of the real route.** `claim` is what the window loop asks
    // before anything acts, and a press it hands to `egui` never reaches the
    // control at all. The pill's centre, because the capsule's top 0.75 is
    // inside the grab of the boundary above the bay — `view::program_head` is
    // where that is measured, and rule 3 is what decides it.
    let head = program_head(&ctx, p.layout()).unwrap_or_else(|| {
        panic!(
            "the Program bay draws no `solo` pill on a solved console, so nothing here can \
             demonstrate a row of `{SECTION}` from a pointer — this pass has stopped measuring \
             rather than found the control gone"
        )
    });
    let on_the_pill = point_of(head.solo.center());
    assert_eq!(
        claim(&mut p, &ctx, &view, on_the_pill),
        Claim::Panel,
        "`claim` gives a press on the Program bay's `solo` pill to `egui`, so no route into \
         `{SECTION}` from this control exists however it is drawn"
    );

    // **Step two**: what the press asks for, off the same derivation `claim`
    // hit-tested. **Step three**: the caller performs it — the pill holds no
    // authority and applies nothing (P-0076).
    let op = head.op();
    let outcome = p.op(op);
    p.solve();
    assert_eq!(
        outcome,
        Outcome::Soloed(head.id),
        "a press on the `solo` pill asked for `{op:?}`, which did not solo the picture"
    );
    assert_ne!(
        shape(&mut p),
        default,
        "the `solo` pill soloed the picture and left the console looking exactly as it did, \
         so the row was demonstrated and did not do what its row on {PAGE} says"
    );
    reached.extend(rows_of(op));

    // And the other answer, off a head derived again on the console the first
    // press left.
    let head = program_head(&ctx, p.layout()).unwrap_or_else(|| {
        panic!(
            "the Program bay draws no `solo` pill with the picture soloed, so the undo this \
             row promises is unreachable — a solo takes every other control off the screen, \
             and this is the one left"
        )
    });
    let on_the_pill = point_of(head.solo.center());
    assert_eq!(
        claim(&mut p, &ctx, &view, on_the_pill),
        Claim::Panel,
        "`claim` gives a press on the `solo` pill to `egui` once the picture is soloed"
    );
    let op = head.op();
    let outcome = p.op(op);
    p.solve();
    assert_eq!(
        outcome,
        Outcome::Unsoloed { was: true },
        "a second press on the `solo` pill asked for `{op:?}`, which did not undo the solo"
    );
    assert_eq!(
        shape(&mut p),
        default,
        "undoing the solo from the pill left the console somewhere else — the row says it \
         restores what was folded before, including whatever was already folded"
    );
    reached.extend(rows_of(op));

    reached
}

/// The floor under both directions below, and the same one `panel_column.rs`
/// carries: a scan that matched nothing satisfies every loop by iterating over
/// nothing at all.
#[test]
fn the_sweep_finds_the_section_and_the_panel() {
    let badges = panel_badges();
    assert_eq!(
        badges.len(),
        rows().len(),
        "{PAGE} has {} rows under `{SECTION}` and {} panel badges — is a badge still an `rt` \
         span reading `panel <b>…</b>`?",
        rows().len(),
        badges.len()
    );
    assert!(
        badges.len() >= 8,
        "only {} rows with a panel badge under `{SECTION}` in {PAGE}",
        badges.len()
    );
    assert!(
        !reached_by_the_pointer().is_empty(),
        "the sweep reached nothing at all — every divider on this panel drags, so a sweep that \
         demonstrates none of them has stopped pressing the panel rather than found it inert"
    );
}

/// **The page claiming a control that does not exist.**
///
/// A row this section marks built in the panel column that no gesture on a
/// running panel performs — ADR-0213's failure mode from the side where the
/// page moved first, which for this section is still the likelier of the two:
/// most of its rows name a home on a console that draws the furniture and
/// hit-tests none of it.
///
/// **Four of them no longer do, and that is what the third and fourth passes
/// are.** *Solo a region* is the newest: `docs/manual/console.html` draws a
/// `solo` pill in the Program bay's head and says it solos the picture, and
/// the panel reaches both halves of the row through it — the solo and its
/// undo, performed rather than named. The three before it are the arrangement
/// family. The
/// arrangement family names the transport row, `docs/manual/console.html`
/// draws `arr · night ▾` there, and the panel reaches all three rows through
/// it — the reset performed, the save and the restore named
/// ([ADR-0225](../../../docs/adr/0225-a-menu-is-a-gesture-in-hand-rather-than-a-rectangle-on-the-panel.md)).
/// This sentence used to read *"three of the seven name the transport row,
/// where `docs/manual/console.html` draws no arrangement control at all yet"*,
/// and it is corrected rather than deleted because it is the reason this
/// assertion exists: it was the failure being guarded against, and it is now
/// the one that was fixed.
#[test]
fn every_arrangement_row_marked_built_is_reached_by_the_pointer() {
    let reached = reached_by_the_pointer();
    for (title, class, home) in panel_badges() {
        if class != "has" {
            continue;
        }
        assert!(
            reached.contains(title.as_str()),
            "{PAGE} marks `{title}` built in the panel column and no gesture on a running \
             `Panel` performs it — the page claims a control an operator cannot find. Either \
             the control went and the badge is `plan` again, or it was never on the panel"
        );
        assert_ne!(
            home, NOWHERE,
            "{PAGE} marks `{title}` built in the panel column and names no home for it — a \
             `has` badge says an operator reaches the operation, so it has to say where the \
             control is"
        );
    }
}

/// **A control reaching past the page.**
///
/// The other direction, and it fails apart from the test above because it is
/// the other failure: that one says the specification promises a player a
/// control nothing draws, and this one says a hand on the panel already
/// performs something the page still calls designed.
#[test]
fn every_arrangement_operation_the_pointer_reaches_is_marked_built() {
    let badges = panel_badges();
    for row in reached_by_the_pointer() {
        let (_, class, _) = badges
            .iter()
            .find(|(title, _, _)| title == row)
            .unwrap_or_else(|| {
                panic!(
                    "a gesture on a running `Panel` performs `{row}` and {PAGE} has no such row \
                     under `{SECTION}` — the page is the specification, so add the row there \
                     first"
                )
            });
        assert_eq!(
            class, "has",
            "a hand on the panel performs `{row}`, which {PAGE} marks `{class}` in the panel \
             column — an operation an operator reaches and a page that says no program a player \
             runs does (ADR-0213). Flip the badge, or say here why the gesture is not reachable"
        );
    }
}
