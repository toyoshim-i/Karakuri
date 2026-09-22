use super::panel_column_common::*;
use super::sample::sample;

/// Every `.rs` file under `dir`, including the ones in directories under it.
///
/// The read was one level deep until 2026-09-07, with the count floor above as
/// its only guard — and the floor could not have caught the case it was written
/// for. There are seven `.rs` files directly under `SRC`; splitting `view.rs`
/// into `view/` would leave seven of them there and take every emission in it
/// out of this scan, silently, with `files.len() >= 7` still true.
/// `crates/karakuri/src/main.rs`'s press handler ran the same listing with the
/// same floor and is gone; this is the other half.
fn walk(dir: &Path, into: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir).unwrap_or_else(|e| {
        panic!(
            "{} is under this crate's source and is unreadable: {e}",
            dir.display()
        )
    });
    for entry in entries {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            walk(&path, into);
        } else if path.extension().is_some_and(|e| e == "rs") {
            into.push(path);
        }
    }
}

/// Every operation a control in this crate constructs, by the criterion in this
/// file's header, as an `Operation` apiece.
///
/// Sorted and deduplicated by title, because the question is which rows are
/// reached and one row may be reached from more than one file. Every `.rs` file
/// under [`SRC`], as one string, for [`DRAWN`]'s markers to be looked for in.
///
/// Comments are not cut, unlike [`emissions`]: a marker naming a field or a
/// function is looked for as text, and a mention of one in a doc comment is a
/// mention of a thing that exists. The failure this guards against is a readout
/// that stopped being drawn, and deleting a field deletes the lines that talk
/// about it.
fn code_of_src() -> String {
    let dir = workspace().join(SRC);
    let mut files = Vec::new();
    walk(&dir, &mut files);
    files.sort();
    files
        .iter()
        .map(|path| {
            fs::read_to_string(path)
                .unwrap_or_else(|e| panic!("{} could not be read: {e}", path.display()))
        })
        .collect()
}

fn emissions() -> Vec<Operation> {
    let dir = workspace().join(SRC);
    let mut files = Vec::new();
    walk(&dir, &mut files);
    files.sort();
    assert!(
        files.len() >= 7,
        "only {} `.rs` files found under {SRC} — a scan that reads nothing would find no \
         emissions and pass every assertion below",
        files.len()
    );

    let mut variants = BTreeSet::new();
    for path in &files {
        let text = fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("{} could not be read: {e}", path.display()));
        for line in text.lines() {
            // The first cut: a line that is nothing but a comment. `///`,
            // `//!` and `//` all begin this way.
            let line = line.trim_start();
            if line.starts_with("//") {
                continue;
            }
            // The second cut: whatever trails a `//` on a line of code.
            let code = match line.find("//") {
                Some(at) => &line[..at],
                None => line,
            };
            for after in code.split("Operation::").skip(1) {
                let end = after
                    .find(|c: char| !c.is_alphanumeric() && c != '_')
                    .unwrap_or(after.len());
                if end > 0 {
                    variants.insert(after[..end].to_owned());
                }
            }
        }
    }

    let mut found: Vec<Operation> = variants.iter().map(|v| sample(v)).collect();
    found.sort_by_key(|op| op.title());
    found.dedup_by_key(|op| op.title());
    found
}

/// The titles of the rows in [`ELSEWHERE`], read off the page rather than
/// listed here, so that a row added to that section is exempt the day it lands
/// and a section renamed out from under this file panics instead of quietly
/// exempting nothing.
fn elsewhere() -> BTreeSet<String> {
    let html = page();
    let start = html.find(ELSEWHERE).unwrap_or_else(|| {
        panic!(
            "`{ELSEWHERE}` is gone from {PAGE} — the section whose panel badges are checked \
                by `tests/vocabulary.rs` rather than here"
        )
    });
    let rest = &html[start + ELSEWHERE.len()..];
    let end = rest.find("<h2").unwrap_or(rest.len());
    let mut found = BTreeSet::new();
    for part in rest[..end].split(ROW).skip(1) {
        let Some(open) = part.find("<h3>") else {
            continue;
        };
        let rest = &part[open + "<h3>".len()..];
        let Some(close) = rest.find("</h3>") else {
            continue;
        };
        found.insert(rest[..close].to_owned());
    }
    assert!(
        found.len() >= 6,
        "only {} rows found under `{ELSEWHERE}` in {PAGE} — an exemption that matches nothing \
         is one this file would not notice it had stopped granting",
        found.len()
    );
    found
}

/// Every row's title and its panel badge, in page order: the badge's class —
/// `has`, `plan` or `gap` — and the text it names the control's home with.
///
/// Read verbatim and never decoded, for `mcp.rs`'s reason: a `gap` badge says
/// `&mdash;` and a home that needed decoding to match would be a home nobody
/// could find on the console page. What a readout the panel draws is drawn by,
/// one row per readout: the title on [`PAGE`] and a marker in [`SRC`] that
/// draws it.
///
/// # Why a `has` badge can be satisfied by this and not only by an emission
///
/// [`every_panel_route_the_page_marks_built_is_emitted_by_a_console_control`]
/// demands that some line of this crate construct `Operation::<the variant>`,
/// and for a write that is the only way a surface reaches an operation: a fader
/// that moves nothing is not a fader. A readout has no gesture at all. It is
/// answered without being asked, so there is no press to attach an emission to,
/// and inventing one would be ADR-0264's own rejected alternative in a worse
/// form.
///
///
/// [ADR-0284](../../../docs/adr/0284-a-readout-is-drawn-and-the-panel-badge-does-not-move-because-the-meter-counts-a-gesture.md)
/// left the badge at `plan` and named the correction to start from: relax the
/// check for rows the page marks `read`, and hold the drawing somewhere that
/// can see it. What it turned that shape down for was what it would stop
/// checking — relaxing for a class of rows removes the emission check for
/// *every* member of it, including the two that are `has` today because a
/// gesture really does emit. This table is the answer to that objection: the
/// check is not relaxed, it is given a second way to be met, and the second way
/// is as mechanical as the first. A row that is neither emitted nor drawn still
/// fails.
///
/// A hand-written table, and the precedent is `press_handler::ASKED` in
/// `crates/karakuri/src/main.rs` — the same shape, the same failure mode, and
/// the same answer to it: an entry that names nothing fails, and a badge with
/// no entry fails. What this one cannot see is the *seam* — that the value
/// reaching the readout is the one the deck said — because `karakuri-console`
/// cannot see `crates/karakuri`. That half is the window's own test, named
/// beside each entry.
const DRAWN: [(&str, &str); 1] = [
    // **The transport row's health capsule** — `landed`, `overloaded` or
    // `failed`, taken off the one drain that writes the Staging lane. The
    // drawing is `tests/transport.rs`'s; the seam is `crates/karakuri`'s
    // `the_swap_report_says_what_the_lane_says`, which takes a device because
    // a `swap::Event` cannot be made without one.
    ("Find out what a write did", "pub health: Option<Stage>"),
];

/// The titles the page marks `read`, off the mark
/// [ADR-0281](../../../docs/adr/0281-every-route-reaches-every-write-and-a-read-is-the-routes-own-interface-design.md)
/// put in each one's `op-head`: *it only asks, so a column left empty here is
/// that way in's own design and not something owed*.
fn reads() -> BTreeSet<String> {
    let html = page();
    let mut found = BTreeSet::new();
    for row in html.split(ROW).skip(1) {
        let Some(close) = row.find("</h3>") else {
            continue;
        };
        let Some(open) = row[..close].rfind("<h3>") else {
            continue;
        };
        let head = &row[close..];
        let head = &head[..head.find("</div>").unwrap_or(head.len())];
        if head.contains(r#"<span class="op-kind">read</span>"#) {
            found.insert(row[open + "<h3>".len()..close].to_owned());
        }
    }
    assert!(
        found.len() >= 4,
        "only {} rows of {PAGE} carry the `read` mark — a mark that matches almost nothing is          one this file would not notice had been renamed",
        found.len()
    );
    found
}

fn panel_routes() -> Vec<(String, String, String)> {
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
        // The row ends where the next section does; a badge found past that
        // would belong to another row.
        let body = &rest[close..];
        let body = &body[..body.find("</section>").unwrap_or(body.len())];
        let mut badge = None;
        for span in body.split(r#"<span class="rt "#).skip(1) {
            let Some(quote) = span.find('"') else {
                continue;
            };
            let class = span[..quote].to_string();
            let Some(text) = span[quote..].strip_prefix(r#"">panel <b>"#) else {
                continue;
            };
            let Some(shut) = text.find("</b>") else {
                continue;
            };
            badge = Some((class, text[..shut].to_string()));
            break;
        }
        let Some((class, home)) = badge else {
            continue;
        };
        found.push((title, class, home));
    }
    found
}

/// The floor under both directions: a scan that matched nothing would satisfy
/// every `for` loop below by iterating over nothing at all.
#[test]
fn the_scan_finds_the_page_and_the_source() {
    let routes = panel_routes();
    assert!(
        routes.len() >= 50,
        "only {} rows with a panel badge found in {PAGE} — is a row still `{ROW}` followed by \
         an `<h3>` and its `rt` badges?",
        routes.len()
    );
    let emitted = emissions();
    assert!(
        emitted.len() >= 5,
        "only {} operations found emitted in {SRC} — the five the Mixer bay's controls emit are \
         the floor, and a scan below it is a scan that has stopped matching code",
        emitted.len()
    );
}

/// A control reaching past the page.
///
/// An operation this crate's controls emit whose panel badge is not `has` is a
/// meter that has stopped moving with the thing it measures — ADR-0213's stated
/// failure mode, which is that a badge moved and a figure did not, from the
/// side where the code moved first.
#[test]
fn every_operation_a_console_control_emits_has_a_panel_route_marked_built() {
    let routes = panel_routes();
    for operation in emissions() {
        let title = operation.title();
        let row = routes
            .iter()
            .find(|(row, _, _)| row == title)
            .unwrap_or_else(|| {
                panic!(
                    "a control in {SRC} emits `{title}` and {PAGE} has no row with that heading \
                     — a control reaching an operation nobody specified. The page is the \
                     specification, so add the row there first"
                )
            });
        if UNREACHABLE.contains(&title) {
            continue;
        }
        assert_eq!(
            row.1, "has",
            "a control in {SRC} emits `{title}`, which {PAGE} marks `{}` in the panel column — \
             a control an operator reaches and a page that says no program a player runs does \
             (ADR-0213). Flip the badge, or say here why the control is not reachable",
            row.1
        );
    }
}

/// [`DRAWN`], held against the page and the source it stands between.
///
/// Written to fail rather than to pass, on
/// [`the_unreachable_exemption_is_still_the_state_of_the_page`]'s terms: a
/// table that names a row the page no longer marks `read`, or a marker no
/// longer in the source, is a second way to meet a `has` badge that has stopped
/// being met. It is also what stops the table being used on a write row, where
/// an emission is the only honest evidence.
#[test]
fn every_drawn_entry_names_a_read_row_the_page_marks_built_and_a_drawing_that_is_there() {
    let reads = reads();
    let routes = panel_routes();
    let src = code_of_src();
    for (title, mark) in DRAWN {
        assert!(
            reads.contains(title),
            "`{title}` is in `DRAWN` and {PAGE} does not mark it `read` — this table is a \
             second way to meet a `has` badge and it is only ever available to a readout. A \
             write reaches an operation by emitting it"
        );
        let row = routes
            .iter()
            .find(|(row, _, _)| row == title)
            .unwrap_or_else(|| panic!("`{title}` is in `DRAWN` and {PAGE} has no row for it"));
        assert_eq!(
            row.1, "has",
            "`{title}` is in `DRAWN` and {PAGE} marks it `{}` in the panel column — an entry \
             here is the evidence behind a built badge, so a row that is not built does not \
             need one and should not carry one",
            row.1
        );
        assert!(
            src.contains(mark),
            "`{title}` is in `DRAWN` naming `{mark}`, and no file under {SRC} contains it — \
             the drawing this badge stands on has gone, or been renamed. The badge is `plan` \
             again, or this marker is"
        );
    }
}

/// The one exemption, held against the page it exempts.
///
/// [`UNREACHABLE`] is a list written by hand, so it is written to fail rather
/// than to go stale: an entry nothing emits is an exemption granted to nobody,
/// and an entry whose badge has become `has` is an exemption that has stopped
/// being true — which is what happens the day somebody settles `SetSync`'s
/// record and the row is genuinely reachable. Either way this says so and names
/// the line to delete.
#[test]
fn the_unreachable_exemption_is_still_the_state_of_the_page() {
    let emitted: BTreeSet<&str> = emissions().iter().map(|op| op.title()).collect();
    let routes = panel_routes();
    for title in UNREACHABLE {
        assert!(
            emitted.contains(title),
            "`{title}` is exempted in `UNREACHABLE` and no control in {SRC} emits it — an \
             exemption granted to nobody. Delete the line"
        );
        let (_, class, _) = routes
            .iter()
            .find(|(row, _, _)| row == title)
            .unwrap_or_else(|| panic!("`{title}` is exempted here and {PAGE} has no such row"));
        assert_ne!(
            class, "has",
            "{PAGE} marks `{title}` built in the panel column, and `UNREACHABLE` still says an \
             operator cannot reach it. If the record it owes has been settled, delete the line \
             in `UNREACHABLE` — the exemption has done its job"
        );
    }
}

/// The page claiming a control that does not exist.
///
/// It fails apart from the test above because it is a different failure: that
/// one says the console reached past the specification, this one says the
/// specification promises a player a control nothing draws. The home is checked
/// too, in the place `mcp.rs` checks a tool's name — the panel column's badge
/// text is a place on the console rather than an identifier this crate holds,
/// so what is checkable is that a built route names one at all. A `has` badge
/// saying `&mdash;` would be the page asserting an operator reaches it and
/// declining to say from where.
#[test]
fn every_panel_route_the_page_marks_built_is_emitted_by_a_console_control() {
    let emitted: BTreeSet<&str> = emissions().iter().map(|op| op.title()).collect();
    let routes = panel_routes();
    let elsewhere = elsewhere();
    let claimed: Vec<&(String, String, String)> = routes
        .iter()
        .filter(|(_, class, _)| class == "has")
        // The console's own shape is reached through `panel::Op` and through a
        // drag that is no operation at all, so a scan for `Operation::` can
        // only report a built badge there as a control that does not exist.
        // See [`ELSEWHERE`], and `tests/vocabulary.rs` for what does check it.
        .filter(|(title, _, _)| !elsewhere.contains(title))
        .collect();
    assert!(
        claimed.len() >= 5,
        "only {} rows of {PAGE} mark a panel route built — the scan found less than the column \
         holds, which would pass this test by finding nothing",
        claimed.len()
    );
    let reads = reads();
    let src = code_of_src();
    for (title, _, home) in claimed {
        // **A row the page marks `read` may be met by a drawing instead**, and
        // only by one this file can name and find. See [`DRAWN`]: a readout is
        // answered without being asked, so there is no gesture to emit at, and
        // the check is given a second way to be met rather than relaxed.
        let drawn = reads.contains(title.as_str())
            && DRAWN
                .iter()
                .any(|(row, mark)| row == title && src.contains(mark));
        assert!(
            emitted.contains(title.as_str()) || drawn,
            "{PAGE} marks `{title}` built in the panel column, and no control in {SRC} emits \
             it — the page claims a control an operator cannot find. Either the control went \
             and the badge is `plan` again, or it never emitted this operation"
        );
        assert_ne!(
            home, NOWHERE,
            "{PAGE} marks `{title}` built in the panel column and names no home for it — a \
             `has` badge says an operator reaches the operation, so it has to say where the \
             control is"
        );
    }
}
