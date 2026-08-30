//! **The panel column of the manual, against the operations this crate's
//! controls emit.**
//!
//! [ADR-0213](../../../docs/adr/0213-the-interface-milestones-meter-is-the-panel-column-and-has-means-an-operator-reaches-it.md)
//! made the panel column of `docs/manual/operations.html` the Interface
//! milestone's meter and defined what a badge in it means: **`has` means an
//! operator running the instrument reaches the operation.** It also said what
//! the first flip owes — *"a test asserts that each `has` row's operation is
//! actually emitted by a console control, and fails in both directions"* — and
//! this file is that test, owed by the five badges `cargo run -p karakuri`
//! made true.
//!
//! It is shaped like `karakuri-environment/src/mcp.rs`'s pair,
//! `every_tool_this_server_publishes_has_a_route_on_the_page` and
//! `every_mcp_route_the_page_claims_is_a_tool_this_server_publishes`, and it
//! sits beside [`vocabulary.rs`](../tests/vocabulary.rs), which reads the same
//! page for the *Arranging the console* section. It is a separate file from
//! that one because it asks a different question of a different type:
//! `vocabulary.rs` checks `panel::Op` against one **section**'s rows, and this
//! checks `karakuri_operation::Operation` against one **column**'s badges.
//!
//! # The whole problem is deciding what "a control emits it" means
//!
//! There is no list in this crate of the operations its controls emit, and a
//! list written here would be the second copy of one
//! ([P-0045](../../../docs/principles/0045-generate-the-vocabulary-prose-drifts-from-code.md)).
//! So the set is **read out of this crate's own source**, and the criterion is
//! stated here rather than left for a reader to infer from a regex.
//!
//! **The criterion.** Over every `.rs` file in `crates/karakuri-console/src`,
//! a line is taken as code after two cuts: a line whose first non-space
//! characters are `//` is dropped whole — which is every `///`, `//!` and `//`
//! — and what is left is truncated at its first `//`. In what survives, every
//! `Operation::` followed by an identifier is an **emission**, and the
//! identifier is the variant. That is what separates
//! `Knob::Trim => Operation::SetGain { .. }` from the fifteen `[`Operation::…`]`
//! links in the doc comments around it, which `grep 'Operation::'` cannot.
//!
//! # What the criterion cannot see, and which way each one fails
//!
//! Every one of these is a way the scan is wrong; what matters is that all but
//! the last of them **fails loudly** rather than passing quietly, and that is
//! why the two cuts are made in the narrowing direction.
//!
//! - **A block comment.** `/* … Operation::SetSync … */` is not a line comment
//!   and is read as an emission. It is a *false positive*: the phantom variant
//!   has no `sample` arm, so [`emissions`] panics naming it.
//! - **`//` inside a string on an emitting line.** The truncation would cut
//!   the emission away with it. That is a *false negative*, and a false
//!   negative cannot fail the direction that says every emission is on the
//!   page — it fails
//!   [`every_panel_route_the_page_marks_built_is_emitted_by_a_console_control`]
//!   instead, which reports it as the page claiming a control that does not
//!   exist. Wrong reason, right failure.
//! - **An emission that never says `Operation::`.** A `use
//!   karakuri_operation::Operation::SetGain;` and a bare `SetGain { .. }`, a
//!   type alias, a variant handed back from a helper in another crate. Invisible
//!   here, and a *false negative* again — so it surfaces the same way, from the
//!   other direction, the moment the page claims it.
//! - **A false positive on a row already marked `has`.** The one combination
//!   that passes in silence: text that is not an emission, naming an operation
//!   the page already claims. Nothing here catches that, and what does is that
//!   the `has` rows each have a test that **presses the control** —
//!   `fader.rs` for the trim and the fader, `blend.rs`, `tally.rs` and
//!   `mask.rs` for the three chips, and `arrangement_pill.rs` for the save and
//!   the restore. This file does not press anything; it is an inventory, and
//!   those five are the proof each item in it is real.
//! - **Construction is not reachability, and reachability is the definition.**
//!   The largest one by far. A `pub fn` in `src/` that builds an `Operation`
//!   and that nothing on the drawn panel calls reads exactly like one a hand
//!   can reach, because ADR-0213's *the operator reaches it* is a property of
//!   `crates/karakuri/src/main.rs` — where a claimed press becomes
//!   `Mixer::blend`, `tally`, `mask` and a drag becomes `Dragged::Fader` — and
//!   this crate takes no device and cannot depend on that binary (ADR-0156).
//!   **So this file checks the necessary half and not the sufficient one.**
//!   A control written here and never wired there would pass, and the badge
//!   would be a lie the page tells on its own authority.
//! - **Only the panel column, and only through `Operation`.** The six rows of
//!   *Arranging the console* reach the operator through `panel::Op` and
//!   through a drag that is no operation at all, not through `Operation`, so
//!   no scan for `Operation::` can see them and this file says nothing about
//!   their badges. `vocabulary.rs` is where that type meets this page, and it
//!   asks a running `Panel` what a hand reaches rather than reading source.
//!
//!   **It is an exemption in the code and not only a sentence here**, which is
//!   [`ELSEWHERE`]: the second assertion below reads every `has` badge in the
//!   column and demands an emission for it, so the day *Move a boundary* was
//!   marked built this file failed saying the page claimed a control that does
//!   not exist — for a drag that no control will ever emit, because the
//!   vocabulary carries it as `Undecided`. The sentence was true of the first
//!   assertion and false of the second.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use karakuri_operation::{BlendMode, Operation, Residency, WipeKind};

/// The specification, relative to the workspace root.
const PAGE: &str = "docs/manual/operations.html";

/// The source this crate's controls are read out of, relative to the same
/// root. Its own crate, read as text: there is no other way to ask *which
/// operations does this code construct* from inside a test binary.
const SRC: &str = "crates/karakuri-console/src";

/// What marks a row on the page — the marker `vocabulary.rs` and
/// `karakuri-environment/src/mcp.rs` both match, for the reason the first of
/// them gives: sections are `<h2>` and a heading somebody adds for looks is
/// neither.
const ROW: &str = r#"<div class="op-head">"#;

/// **The one section of the page whose panel badges are not this file's**, and
/// the heading is what identifies it because a section is an `<h2>` here as it
/// is everywhere else on the page.
///
/// *Arranging the console* is the console's own shape, and every route into it
/// is [`karakuri_console::panel::Op`] or a gesture on
/// `karakuri_console::panel::Panel` — never a `karakuri_operation::Operation`,
/// which is what this file scans for. So a `has` badge in that section is a
/// claim this file cannot judge and **would judge wrongly**: *Move a boundary*
/// is a drag rather than an operation, the vocabulary carries it as
/// `Undecided`, and nothing in [`SRC`] will ever construct it. That was the
/// header's last bullet said as prose; this is it said as code, because the
/// bullet was true of the first assertion below and not of the second, which
/// went on demanding an `Operation` for every badge in the column.
///
/// **Those badges are checked, and `tests/vocabulary.rs` is where.** It reads
/// this section and asks a running `Panel` what a hand on it reaches, both
/// ways round — the same pair as here, against the type the console performs.
const ELSEWHERE: &str = "<h2>Arranging the console</h2>";

/// The badge text of a route that names nowhere. A `plan` badge is allowed to
/// be this — three of them are, and ADR-0213 says which — but a `has` badge
/// cannot: it would claim an operator reaches the operation and decline to say
/// from where.
const NOWHERE: &str = "&mdash;";

/// **The rows a control on this panel emits and an operator still cannot
/// reach**, which is the one gap between *emitted* and ADR-0213's *reached*
/// that this file has ever had to carry.
///
/// # Why the two came apart, having been the same thing until now
///
/// Every other emission on this list ends in a record and a movement:
/// `written` converts it, `crates/karakuri` applies it, and the deck is
/// somewhere else afterwards. **`SetSync` converts to `Owed(NotSettled)`**, and
/// that is not a gap in this crate or in that binary — it is
/// `karakuri-operation-record` saying that *what* its record carries is
/// undecided: setting a sync mode needs the session tempo and the engine's
/// anchor clamp, so whether the record carries the anchor that was asked for
/// or the one that was clamped is *"a decision about the bytes on disk"*, and
/// [ADR-0218](../../../docs/adr/0218-re-anchoring-is-set-sync-naming-the-mode-the-deck-is-in-and-a-cycle-cannot-say-it.md)
/// leaves it open by name. So the deck head's sync chip and its anchor are
/// reachable *affordances* over an unwritable record: a press is claimed, the
/// operation is emitted, the window prints the question, and the deck does not
/// move.
///
/// **`has` would be a lie in exactly the way ADR-0213 was written to
/// prevent** — *"the row is claimed the day a person who launched the
/// instrument can perform that operation from the panel in front of them"* —
/// and drawing no control would be a worse one, because the panel is the only
/// surface that can offer re-anchoring at all. So the badge stays `plan` and
/// the exemption is written here with its reason, which is what the first
/// assertion's own failure message invites: *"Flip the badge, or say here why
/// the control is not reachable"*.
///
/// # It is written to delete itself
///
/// A list here is a second copy of something ([P-0045]), so this one is held
/// against both of its halves by
/// [`the_unreachable_exemption_is_still_the_state_of_the_page`]: the operation
/// must still be emitted, and its badge must still **not** be `has`. The day
/// somebody settles the record, the badge flips, that test fails, and the line
/// below is what it tells them to remove. It cannot go stale in silence in
/// either direction.
///
/// **Not derived from `karakuri-operation-record`**, which is where the answer
/// lives, because this package deliberately holds no dependency on it —
/// `Cargo.toml` says so at length, and reaching for one to spell a one-line
/// exemption would undo the closing of ADR-0156 that manifest records.
///
/// [P-0045]: ../../../docs/principles/0045-generate-the-vocabulary-prose-drifts-from-code.md
const UNREACHABLE: [&str; 1] = ["Set a deck's sync mode"];

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

/// **A value of the operation a variant name stands for**, so that the row
/// this file looks for is [`Operation::title`]'s answer and never a heading
/// transcribed here.
///
/// The same job `mcp.rs`'s `sample` does for a tool's arguments, and it fails
/// the same way: a control that starts emitting something new arrives as a
/// panic naming the variant, rather than being passed over. The field values
/// are arbitrary — nothing reads them — and the *variant* is compiler-checked,
/// so renaming one in `karakuri-operation` breaks this file at build time.
fn sample(variant: &str) -> Operation {
    match variant {
        "SetGain" => Operation::SetGain { deck: 0, gain: 0.0 },
        "SetOpacity" => Operation::SetOpacity {
            deck: 0,
            opacity: 0.0,
        },
        "SetBlendMode" => Operation::SetBlendMode {
            deck: 0,
            blend: BlendMode::Over,
        },
        "SetResidency" => Operation::SetResidency {
            deck: 0,
            residency: Residency::Live,
        },
        "SetMaskShape" => Operation::SetMaskShape {
            deck: 0,
            kind: WipeKind::Linear,
            angle: 0.0,
        },
        // The two the transport row's arrangement pill emits. They are the
        // first emissions from a row of *Arranging the console*, which
        // [`ELSEWHERE`] exempts from the other direction and not from this
        // one — so the day the pill landed, this file demanded the two badges
        // on that page and got them.
        "SaveArrangement" => Operation::SaveArrangement {
            name: String::new(),
        },
        "RestoreArrangement" => Operation::RestoreArrangement {
            name: String::new(),
        },
        // The two the transport row's look controls emit. They are one row of
        // *Mixing and output* each, so unlike the pair above they are **not**
        // in [`ELSEWHERE`] and both directions below judge them.
        "SetTonemap" => Operation::SetTonemap {
            tonemap: karakuri_operation::Tonemap::Aces,
        },
        "SetExposure" => Operation::SetExposure { exposure: 1.0 },
        // The two the Inspector's deck head emits. `SetSync` comes from two
        // controls in that row — the chip that cycles and the anchor that
        // re-asks for the mode the deck is in (ADR-0218) — and one operation
        // is one row however many controls name it, which is what
        // `dedup_by_key` below is for. `SetSync` is also the one entry in
        // [`UNREACHABLE`]; see there for why its badge is not `has`.
        "SetSync" => Operation::SetSync {
            deck: 0,
            sync: karakuri_operation::Sync::Beat,
        },
        "ScrubDeck" => Operation::ScrubDeck {
            deck: 0,
            beats: 0.25,
        },
        other => panic!(
            "`{SRC}` constructs `Operation::{other}` and this file has no value for it — a \
             control started emitting an operation nobody accounted for. Add an arm here, and \
             then decide whether that row's panel badge in {PAGE} is now `has`"
        ),
    }
}

/// **Every operation a control in this crate constructs**, by the criterion in
/// this file's header, as an `Operation` apiece.
///
/// Sorted and deduplicated by title, because the question is which rows are
/// reached and one row may be reached from more than one file.
fn emissions() -> Vec<Operation> {
    let dir = workspace().join(SRC);
    let mut files: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap_or_else(|e| {
            panic!(
                "{} is this crate's source and is unreadable: {e}",
                dir.display()
            )
        })
        .map(|entry| entry.expect("a directory entry").path())
        .filter(|path| path.extension().is_some_and(|e| e == "rs"))
        .collect();
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

/// **The titles of the rows in [`ELSEWHERE`]**, read off the page rather than
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

/// **Every row's title and its panel badge**, in page order: the badge's class
/// — `has`, `plan` or `gap` — and the text it names the control's home with.
///
/// Read verbatim and never decoded, for `mcp.rs`'s reason: a `gap` badge says
/// `&mdash;` and a home that needed decoding to match would be a home nobody
/// could find on the console page.
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

/// **A control reaching past the page.**
///
/// An operation this crate's controls emit whose panel badge is not `has` is a
/// meter that has stopped moving with the thing it measures — ADR-0213's
/// stated failure mode, which is that a badge moved and a figure did not, from
/// the side where the code moved first.
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

/// **The one exemption, held against the page it exempts.**
///
/// [`UNREACHABLE`] is a list written by hand, so it is written to fail rather
/// than to go stale: an entry nothing emits is an exemption granted to
/// nobody, and an entry whose badge has become `has` is an exemption that has
/// stopped being true — which is what happens the day somebody settles
/// `SetSync`'s record and the row is genuinely reachable. Either way this
/// says so and names the line to delete.
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

/// **The page claiming a control that does not exist.**
///
/// It fails apart from the test above because it is a different failure: that
/// one says the console reached past the specification, this one says the
/// specification promises a player a control nothing draws. The home is
/// checked too, in the place `mcp.rs` checks a tool's name — the panel
/// column's badge text is a place on the console rather than an identifier
/// this crate holds, so what is checkable is that a built route names one at
/// all. A `has` badge saying `&mdash;` would be the page asserting an operator
/// reaches it and declining to say from where.
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
    for (title, _, home) in claimed {
        assert!(
            emitted.contains(title.as_str()),
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
