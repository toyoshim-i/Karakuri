//! Every constant this crate transcribes out of the mock says where it came
//! from, and this file checks that it came from there. It reads the sources
//! in [`SOURCES`] and `docs/manual/style.css` and holds each constant against
//! the source its own doc comment cites.
//!
//! # Why a transcription needs a guard of its own
//!
//! `room::size` is 156 numbers copied out of a stylesheet by hand — 72 of them
//! when this file landed, and the count carries its date because it only grows
//! (2026-09-05) — and a wrong
//! copy is invisible to every other test in the crate. Those tests state a
//! region's arithmetic *in terms of* these constants — `head.height() ==
//! size::HEAD_H`, and so on — so a test of the arithmetic compares the layout
//! against the same wrong number and passes. ADR-0177 records the case that
//! found it: `.transport`'s `gap: 14px` was transcribed as 10 and its test
//! stayed green. When this file landed, 23 of the 72 were named by no test at
//! all.
//!
//! `lib.rs`'s arrangement dividers are the same transcription and are more
//! exposed, not less: they are private consts, so no test can name even one of
//! them. They are read here for that reason. Nothing else in the workspace is
//! — `karakuri-layout` and the engine have numbers of their own and none of
//! them is a copy of a stylesheet.
//!
//! The direction that matters most is the other one. The stylesheet is the
//! specification and it is the one that moves: nothing about editing
//! `style.css` tells you that a Rust constant was reading it. So the check is
//! not "is this number plausible" but "does the rule this comment names still
//! say this", and a change on either side breaks it.
//!
//! # Three kinds of constant, told apart rather than forced into one mould
//!
//! - transcribed — a literal that appears in the mock. Its doc comment
//!   cites a selector and a declaration, and the guard resolves both.
//! - derived — computed from other constants in the module: `HEAD_H` is
//!   `6 + 10 * 1.5 + 6`, `PILL_H` is `BASE * LINE`. The arithmetic is the
//!   claim and the compiler already holds it; no stylesheet carries such a
//!   sum, so none is demanded. A derived constant is recognised from its
//!   *initializer* — it names another constant — and not from its prose,
//!   because that is the half that cannot be got wrong.
//! - the console's own — a number with no single source in the mock.
//!   [`HAIRLINE`] is the one here: one pixel is what the mock draws every rule
//!   at, and no one selector is its source. `panel::GRAB` is the same shape and
//!   lives outside this module.
//!
//! A constant that declares none of the three fails, which is the half
//! that makes this a convention rather than a lint: the next literal written
//! here has to say where it came from before it can compile a green suite.
//!
//! # The citation form
//!
//! The doc comments were prose before this file existed and they are prose
//! still — 69 of the 72 needed no change at all. What is read out of them is:
//!
//! - a source, in backticks: a selector (anything starting with `.`) or a
//!   file under `docs/manual/`;
//! - a declaration, in backticks and containing a colon: `gap: 14px`, or
//!   several at once, `width: 15px; height: 6px`.
//!
//! A declaration binds to the nearest source named *before* it in the same doc
//! comment, so `` `.trim`'s `gap: 5px` `` and `` `.sink`'s own `gap: 6px` ``
//! and "`.vfader s`'s `height: 9px`, and its `left: -2px; right: -2px`" all
//! read the same way without anyone having to write to a format. Intra-doc
//! links — ``[`BASE`]`` — are stripped before the scan, so a cross-reference
//! is never mistaken for a source.
//!
//! A source that is a selector is resolved in the stylesheet: the rule must
//! exist, it must set that property, and its value must be the value the
//! comment quotes. A source that is a file is resolved by looking the
//! declaration up in that file verbatim — weaker, and used by exactly one
//! constant ([`PILL_GAP`]), because the mock sets that one inline in the
//! markup and the stylesheet genuinely does not carry it.
//!
//! A declaration with no source before it is prose, not a citation:
//! `TALLY_H`'s aside about `border-radius: 999px` names no selector because it
//! is talking about every capsule in the mock. Those are still checked to
//! exist somewhere in `docs/manual/`, so a property that has been renamed away
//! fails wherever it is mentioned — but they do not satisfy the requirement to
//! cite, and they do not feed the number check below.
//!
//! # What the numbers are checked against
//!
//! The stylesheet's value, not the comment's copy of it — the comment has
//! already been held against the stylesheet by then. A transcribed
//! constant's value must be one of the numbers in the declarations it cites,
//! which is what makes a shared comment work: `padding: 6px 10px` cited by
//! both `HEAD_PAD_X` and `HEAD_PAD_Y` offers 6 and 10 and each takes one.
//! Signs are dropped — `.vfader s`'s `left: -2px` is `VFADER_KNOB_OUT =
//! 2.0`, two pixels *proud*; this module is about sizes and a direction is not
//! one.
//!
//! A derived constant that cites something has its citation resolved like
//! any other, and then the literal factors in its own expression are checked
//! against the cited numbers: `HEAD_TRACKING` is `HEAD_SIZE * 0.16` against
//! `letter-spacing: 0.16em`, so the 0.16 is held to the stylesheet even though
//! 1.6 never appears there.
//!
//! # How the doc comments are grouped
//!
//! One doc comment can cover several constants — `padding: 6px 10px` is
//! written once above `HEAD_PAD_X` and `HEAD_PAD_Y`, and only the first of the
//! two carries the comment as far as `rustdoc` is concerned. This file reads
//! them the way a person does: a doc comment covers every constant that
//! follows it until the next doc comment or the next section rule. That is
//! load-bearing — read strictly, ten constants here would have no citation at
//! all and the guard would demand ten comments nobody wants.
//!
//! The shape is `karakuri-engine/tests/gpu_tests_are_under_mod_gpu.rs`: scan
//! the checked-in source, and carry floors so the scan cannot silently pass by
//! finding nothing.

use karakuri_console::room::Palette;
use karakuri_console::view::{Band, BAND_BLUE_MS, BAND_PURPLE_MS, BAND_RED_MS, BAND_YELLOW_MS};
use std::fs;
use std::path::{Path, PathBuf};

/// The transcriptions under test, each paired with the text its constants start
/// after. A pair rather than a bare path because `room.rs` also holds
/// `Palette::DAY` and `Palette::NIGHT`, which are constants and are not
/// transcribed sizes, and because a marker that drifts fails loudly here rather
/// than quietly shrinking the scan.
const SOURCES: &[(&str, &str)] = &[
    ("crates/karakuri-console/src/room.rs", "pub mod size {"),
    (
        "crates/karakuri-console/src/lib.rs",
        "use karakuri_layout::{Layout, Spec};",
    ),
];

/// The specification they are a transcription of.
const STYLESHEET: &str = "docs/manual/style.css";

/// The mock as a whole. A citation may name a file in here, and an uncited
/// declaration mentioned in prose has to exist somewhere in it.
const MOCK: &str = "docs/manual";

/// The phrase that declares a constant to have no single source in the mock.
/// Deliberately a sentence a person would write anyway rather than an
/// attribute: these comments are read by people first.
const OWN: &str = "the console's own";

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

/// Whitespace runs to one space, trimmed, and no space hugging a `:` or a `;`.
/// One normalisation for both sides: it turns the stylesheet's `gap: 5px` and
/// the markup's `gap:5px` into the same string, which is the only reason a
/// citation can name either.
fn tighten(text: &str) -> String {
    let squashed = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut out = String::with_capacity(squashed.len());
    for ch in squashed.chars() {
        if ch == ' ' && matches!(out.chars().last(), Some(':') | Some(';')) {
            continue;
        }
        if matches!(ch, ':' | ';') {
            while out.ends_with(' ') {
                out.pop();
            }
        }
        out.push(ch);
    }
    out
}

/// Whitespace runs to one space, trimmed — for a selector or a value, where the
/// spaces around a colon are not in play.
fn squash(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// `/* … */` out, keeping the length loosely irrelevant — nothing downstream
/// works from offsets into the original.
fn strip_css_comments(css: &str) -> String {
    let mut out = String::with_capacity(css.len());
    let mut rest = css;
    while let Some(at) = rest.find("/*") {
        out.push_str(&rest[..at]);
        match rest[at + 2..].find("*/") {
            Some(end) => rest = &rest[at + 2 + end + 2..],
            None => return out,
        }
    }
    out.push_str(rest);
    out
}

/// One `selector { property: value; … }`, flattened: a rule listing several
/// selectors becomes one of these per selector, so a lookup is an exact match
/// on one name and never a guess about which comma-separated part was meant.
struct Rule {
    selector: String,
    decls: Vec<(String, String)>,
}

/// Every rule in a stylesheet, descending into `@media` and anything else that
/// nests. A block whose body contains a `{` is a container, not a rule.
fn rules(css: &str, into: &mut Vec<Rule>) {
    let bytes = css.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let Some(open) = css[i..].find('{').map(|n| i + n) else {
            break;
        };
        let mut depth = 0usize;
        let mut k = open;
        while k < bytes.len() {
            match bytes[k] {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
            k += 1;
        }
        let close = k.min(bytes.len());
        let selectors = &css[i..open];
        let body = &css[open + 1..close];
        if body.contains('{') {
            rules(body, into);
        } else {
            let decls: Vec<(String, String)> = body
                .split(';')
                .filter_map(|d| d.split_once(':'))
                .map(|(p, v)| (squash(p), squash(v)))
                .collect();
            for selector in selectors.split(',') {
                let selector = squash(selector);
                if !selector.is_empty() {
                    into.push(Rule {
                        selector,
                        decls: decls.clone(),
                    });
                }
            }
        }
        i = close + 1;
    }
}

/// What the stylesheet says about one property of one selector. `None` for the
/// selector means no such rule; `Some(None)` means the rule is there and does
/// not set it — two different failures, and both are worth telling apart.
fn declared<'a>(rules: &'a [Rule], selector: &str, property: &str) -> Option<Option<&'a str>> {
    let mut seen = false;
    let mut value = None;
    for rule in rules.iter().filter(|r| r.selector == selector) {
        seen = true;
        // Last one wins, as the cascade would have it.
        for (p, v) in &rule.decls {
            if p == property {
                value = Some(v.as_str());
            }
        }
    }
    seen.then_some(value)
}

/// Every `-?\d+(\.\d+)?` in a string, as magnitudes. Written out rather than
/// pulled in, for the same reason the GPU scan is: this crate's test
/// dependencies are the ones its own tests need and a guard is not a reason to
/// grow them.
fn numbers(text: &str) -> Vec<f64> {
    let b = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if !b[i].is_ascii_digit() {
            i += 1;
            continue;
        }
        let start = if i > 0 && b[i - 1] == b'-' { i - 1 } else { i };
        let mut j = i;
        while j < b.len() && b[j].is_ascii_digit() {
            j += 1;
        }
        if b.get(j) == Some(&b'.') && b.get(j + 1).is_some_and(u8::is_ascii_digit) {
            j += 1;
            while j < b.len() && b[j].is_ascii_digit() {
                j += 1;
            }
        }
        if let Ok(n) = text[start..j].parse::<f64>() {
            out.push(n.abs());
        }
        i = j;
    }
    out
}

/// Where a declaration was said to come from.
enum Source {
    /// A selector, resolved in the stylesheet.
    Selector(String),
    /// A file under `docs/manual/`, resolved by looking the declaration up in it
    /// verbatim.
    File(String),
    /// Nothing named it — prose about the mock at large.
    Prose,
}

struct Citation {
    source: Source,
    property: String,
    value: String,
}

/// A doc comment and the constants it covers.
struct Group {
    doc: String,
    consts: Vec<(String, String)>,
}

/// One source file's constants, read as a person reads them: a doc comment,
/// then the run of constants it covers. A bare `//` section rule ends a run,
/// because that is what it does to the eye.
fn groups(source: &str, path: &str, from: &str) -> Vec<Group> {
    let start = source
        .find(from)
        .unwrap_or_else(|| panic!("`{from}` is not in {path} any more, so nothing was read of it"));
    let lines: Vec<&str> = source[start..].lines().map(str::trim).collect();
    let mut out: Vec<Group> = Vec::new();
    let mut doc: Vec<&str> = Vec::new();
    let mut open: Option<Group> = None;
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        if let Some(text) = line.strip_prefix("///") {
            // A new comment closes whatever run was open.
            out.extend(open.take());
            doc.push(text.trim());
        } else if line.starts_with("const ") || line.starts_with("pub const ") {
            let mut stmt = line.to_string();
            while !stmt.contains(';') && i + 1 < lines.len() {
                i += 1;
                stmt.push(' ');
                stmt.push_str(lines[i]);
            }
            let (name, init) = stmt
                .trim_start_matches("pub ")
                .trim_start_matches("const ")
                .trim()
                .split_once('=')
                .and_then(|(decl, init)| {
                    Some((
                        decl.split_once(':')?.0.trim(),
                        init.trim().trim_end_matches(';'),
                    ))
                })
                .unwrap_or_else(|| panic!("cannot read the constant on `{stmt}`"));
            let group = open.get_or_insert_with(|| Group {
                doc: doc.join(" "),
                consts: Vec::new(),
            });
            group
                .consts
                .push((name.to_string(), init.trim().to_string()));
            doc.clear();
        } else if line.starts_with("//") {
            out.extend(open.take());
            doc.clear();
        }
        i += 1;
    }
    out.extend(open);
    out
}

/// `[`NAME`]` out, so a cross-reference to another constant is never read as a
/// source. Done before the backtick scan and not during it, because the scan
/// only knows about backticks and an intra-doc link is a bracket around a pair
/// of them.
fn strip_links(doc: &str) -> String {
    let mut out = String::with_capacity(doc.len());
    let mut rest = doc;
    while let Some(at) = rest.find("[`") {
        out.push_str(&rest[..at]);
        match rest[at + 2..].find("`]") {
            Some(end) => rest = &rest[at + 2 + end + 2..],
            None => break,
        }
    }
    out.push_str(rest);
    out
}

/// The citations in one doc comment, in the order they are written. A source
/// span replaces whatever source was in force; a declaration span takes the one
/// in force at that point.
fn citations(doc: &str) -> Vec<Citation> {
    let stripped = strip_links(doc);
    let mut out = Vec::new();
    let mut source = Source::Prose;
    // Odd pieces of a split on the delimiter are what was between a pair.
    for (n, span) in stripped.split('`').enumerate() {
        if n % 2 == 0 {
            continue;
        }
        let span = squash(span);
        if span.starts_with(MOCK) {
            source = Source::File(span);
        } else if span.starts_with('.') {
            source = Source::Selector(span);
        } else if span.contains(':') {
            for one in span.split(';') {
                if let Some((property, value)) = one.split_once(':') {
                    out.push(Citation {
                        source: match &source {
                            Source::Selector(s) => Source::Selector(s.clone()),
                            Source::File(f) => Source::File(f.clone()),
                            Source::Prose => Source::Prose,
                        },
                        property: squash(property),
                        value: squash(value),
                    });
                }
            }
        }
    }
    out
}

/// Does this initializer name another constant? That is what makes it derived,
/// and it is read from the code rather than from the prose because the code is
/// the half that cannot be got wrong.
fn is_derived(init: &str) -> bool {
    init.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .any(|word| {
            word.len() >= 3
                && word.starts_with(|c: char| c.is_ascii_uppercase())
                && word
                    .chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
        })
}

fn holds(pool: &[f64], want: f64) -> bool {
    pool.iter().any(|n| (n - want).abs() < 1e-9)
}

#[test]
fn every_transcribed_constant_matches_the_source_it_cites() {
    let root = workspace();
    let css = fs::read_to_string(root.join(STYLESHEET)).expect("the stylesheet");
    let mut sheet = Vec::new();
    rules(&strip_css_comments(&css), &mut sheet);

    let mock: Vec<(String, String)> = fs::read_dir(root.join(MOCK))
        .expect("the mock")
        .map(|entry| entry.expect("dir entry").path())
        .filter(|p| p.is_file())
        .map(|p| {
            let text = fs::read_to_string(&p).expect("read the mock");
            (p.display().to_string(), tighten(&text))
        })
        .collect();

    let (mut total, mut transcribed, mut derived, mut own, mut resolved) = (0, 0, 0, 0, 0);

    for (path, from) in SOURCES {
        let source = fs::read_to_string(root.join(path)).expect("a transcription");
        let read = groups(&source, path, from);
        // A marker can survive while the constants under it move somewhere
        // else, and a source that contributes nothing is a source that is not
        // being checked at all.
        assert!(
            read.iter().any(|g| !g.consts.is_empty()),
            "no constants were read out of {path} after `{from}` — the scan has lost the file"
        );

        for group in read {
            let named = format!(
                "{} in {path}",
                group
                    .consts
                    .iter()
                    .map(|(n, _)| n.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            let cited = citations(&group.doc);
            // The numbers the stylesheet actually carries at the places this
            // comment points at — what a transcription is held against.
            let mut pool: Vec<f64> = Vec::new();
            let mut anchored = 0;

            for citation in &cited {
                let Citation {
                    source,
                    property,
                    value,
                } = citation;
                match source {
                    Source::Selector(selector) => {
                        anchored += 1;
                        resolved += 1;
                        let Some(found) = declared(&sheet, selector, property) else {
                            panic!(
                            "{named}: the doc comment cites `{selector}`, and {STYLESHEET} has no \
                             such selector — the mock has been renamed out from under the \
                             transcription, so nothing here is checking anything"
                        );
                        };
                        let Some(found) = found else {
                            panic!(
                                "{named}: the doc comment cites `{selector}`'s `{property}`, and \
                             `{selector}` in {STYLESHEET} does not set `{property}` — either the \
                             declaration moved to another rule or the citation names the wrong one"
                            );
                        };
                        assert_eq!(
                        found, value,
                        "\n{named}: {STYLESHEET} says `{selector} {{ {property}: {found} }}` and \
                         the doc comment says `{property}: {value}`.\nThe stylesheet is the \
                         specification: if it moved on purpose, the constant and its comment \
                         follow it.\n"
                    );
                        pool.extend(numbers(found));
                    }
                    Source::File(file) => {
                        anchored += 1;
                        let want = tighten(&format!("{property}: {value}"));
                        let text = mock
                            .iter()
                            .find(|(path, _)| path.ends_with(file.as_str()))
                            .unwrap_or_else(|| {
                                panic!(
                                "{named}: the doc comment cites `{file}`, which is not in {MOCK}"
                            )
                            });
                        assert!(
                        text.1.contains(&want),
                        "{named}: the doc comment says `{file}` carries `{property}: {value}`, \
                         and it does not"
                    );
                        pool.extend(numbers(value));
                    }
                    Source::Prose => {
                        // Not a citation, but still a claim about the mock.
                        let want = tighten(&format!("{property}: {value}"));
                        assert!(
                        mock.iter().any(|(_, text)| text.contains(&want)),
                        "{named}: the doc comment mentions `{property}: {value}`, and nothing in \
                         {MOCK} says that any more — name the selector it came from, or fix the \
                         prose"
                    );
                    }
                }
            }

            for (name, init) in &group.consts {
                total += 1;
                let computed = is_derived(init);
                if anchored == 0 {
                    if computed {
                        derived += 1;
                    } else if group.doc.to_lowercase().contains(OWN) {
                        own += 1;
                    } else {
                        panic!(
                            "{name} in {path} is a literal, and its doc comment neither cites a \
                         selector and a declaration in {STYLESHEET} nor says it is \
                         `{OWN}`.\nEvery number this crate copies out of the mock says where it \
                         came from — a transcription cites the rule it was copied from, a derived \
                         one is written as the arithmetic over the constants it is derived from, \
                         and one the mock has no single source for says so in those words."
                        );
                    }
                    continue;
                }
                if computed {
                    derived += 1;
                    let factors = numbers(init);
                    assert!(
                    factors.is_empty() || factors.iter().any(|f| holds(&pool, *f)),
                    "\n{name} in {path} is `{init}`, and none of its own factors {factors:?} is \
                     among the numbers {pool:?} the stylesheet carries where this comment \
                     points.\nA derived constant that cites a rule is claiming the rule is where \
                     its literal factor came from.\n"
                );
                } else {
                    transcribed += 1;
                    let value = numbers(init);
                    let [value] = value[..] else {
                        panic!("{name} in {path} = `{init}` is neither one number nor derived from any");
                    };
                    assert!(
                    holds(&pool, value),
                    "\n{name} in {path} = {init}, and {STYLESHEET} carries {pool:?} where this \
                     comment points.\nOne of the two is a wrong transcription, and the \
                     stylesheet is the specification.\n"
                );
                }
            }
        }
    }

    // Floors, not counts: three of them because they fail apart. A moved
    // marker gives no constants at all; a broken CSS parse gives constants but
    // resolves no selector; and a stylesheet that has quietly lost its rules
    // gives both but nothing to resolve against. They read 76, 63 and 67 when
    // `lib.rs` was added to the scan and 85, 70 and 77 when the Library bay's
    // eight landed, and are meant to be raised, never lowered to fit a smaller
    // one. The margin under each reading is the same two or three it was set
    // with: a floor at the reading itself fails on any deletion at all, which
    // is a different question from the one this guards.
    //
    // **They had not been raised since, and the numbers say by how much**: the
    // scan reads 160, 127 and 118 on 2026-09-05, where the floors still stood
    // at 85's margin. A floor two thirds under the reading is not a floor —
    // seventy constants could have gone before it said anything — so they are
    // set from that reading here, with the same margin, when the Library
    // filter row's six moved into `room::size` and came under this guard.
    assert_eq!(
        total,
        transcribed + derived + own,
        "every constant is exactly one of the three kinds"
    );
    assert!(
        total >= 158,
        "only {total} constants read out of {} files — has a marker drifted?",
        SOURCES.len()
    );
    assert!(
        transcribed >= 124,
        "only {transcribed} constants read as transcribed — the citation scan is not seeing them"
    );
    assert!(
        resolved >= 115,
        "only {resolved} citations resolved against {STYLESHEET} — is the stylesheet still parsing?"
    );
    assert!(
        sheet.len() >= 100,
        "only {} rules parsed out of {STYLESHEET}",
        sheet.len()
    );
}

// ---------------------------------------------------------------------------
// The risk badge's five bands
// ---------------------------------------------------------------------------
//
// **A number transcribed out of `console.html`'s prose rather than out of a
// declaration, which is why it is checked here and not above.** The scan above
// resolves a citation of the form `` `selector` `` and `` `property: value` ``,
// and the band boundaries are written as English — *"Green, up to 4 ms: four
// of these at 60 Hz."* They are exactly the kind of number this file exists
// for, and none of them can be reached by that machinery, so they get a reader
// of their own.
//
// The direction that matters is still the other one: **the page is the
// specification and it is the one that moves**, and nothing about rewriting a
// sentence in it tells you that a Rust constant was reading it.

/// The page the bands are specified on: *What a deck preview cell shows, and
/// when*, and deck A's caption tooltip.
const PAGE: &str = "docs/manual/console.html";

/// Every reading of one band's boundary in the page.
///
/// A band's name, as a whole word, with a figure in milliseconds after it and
/// nothing but the qualifier — *up to*, *over*, *about* — in between. An
/// occurrence with no `ms` after it inside the window is prose about a colour
/// rather than a statement of the scale (*"drawn green by default"*), and a
/// window is what tells the two apart without this file having to hold a copy
/// of the sentences.
///
/// Whole word, because `red` is inside `coloured`, `prepared` and `required`,
/// all three of which are on this page.
fn boundaries_in(page: &str, word: &str) -> Vec<f64> {
    /// Far enough to clear `</strong>, up to ` and no further: the next sentence's
    /// own figures must not be in reach.
    const WINDOW: usize = 40;

    let lower = page.to_lowercase();
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(at) = lower[from..].find(word).map(|n| from + n) {
        let end = at + word.len();
        from = end;
        let before = lower[..at].chars().next_back();
        let after = lower[end..].chars().next();
        if before.is_some_and(|c| c.is_ascii_alphanumeric())
            || after.is_some_and(|c| c.is_ascii_alphanumeric())
        {
            continue;
        }
        let window = &lower[end..lower.len().min(end + WINDOW)];
        let Some(ms) = window.find(" ms") else {
            continue;
        };
        // The last figure before the unit, which is the one the unit is on.
        if let Some(last) = numbers(&window[..ms]).last() {
            out.push(*last);
        }
    }
    out
}

/// The five bands' boundaries are the ones `docs/manual/console.html` states,
/// and they are stated there twice.
///
/// `view::BAND_BLUE_MS` and its three siblings are the console's own table —
/// `karakuri-engine` produces a number of milliseconds and has no opinion about
/// how many of a thing an operator can mix — and they are a transcription out
/// of the page like every constant above.
///
/// Two readings per band, and they must agree. The scale is written on deck A's
/// caption tooltip and again in the body under *What a deck preview cell shows,
/// and when*, which is the duplication the manual's own rule warns about: *"the
/// same prose sits in three places … and duplication produces gaps and
/// contradictions"*. So this asserts every reading of a band rather than the
/// first, and a page that moved a boundary in one passage and not the other
/// fails here rather than shipping two scales.
///
/// Green and blue are one boundary read from both ends — *"Green, up to 4 ms"*
/// and *"Blue, over 4 ms"* — which is the four-boundary table stated as five
/// bands, and is why [`BAND_BLUE_MS`] is named for the band it lets you into
/// rather than the one it leaves.
#[test]
fn the_band_boundaries_are_the_ones_the_console_page_states() {
    let page = fs::read_to_string(workspace().join(PAGE)).expect("the console page");
    let page = squash(&page);

    for (band, want) in [
        // The one boundary that is written from both sides.
        (Band::Green, BAND_BLUE_MS),
        (Band::Blue, BAND_BLUE_MS),
        (Band::Yellow, BAND_YELLOW_MS),
        (Band::Red, BAND_RED_MS),
        (Band::Purple, BAND_PURPLE_MS),
    ] {
        let read = boundaries_in(&page, band.word());
        assert!(
            read.len() >= 2,
            "{PAGE} states {}'s boundary {} time(s), and it is specified twice — on deck A's \
             caption tooltip and in the body. Has a passage been rewritten?",
            band.word(),
            read.len()
        );
        for reading in &read {
            assert_eq!(
                *reading,
                f64::from(want),
                "\n{PAGE} says {} is at {reading} ms and the console's table says {want}.\nThe \
                 page is the specification: if it moved on purpose, the constant follows it.\n",
                band.word()
            );
        }
    }

    // **The rule that turns four boundaries into five bands**, in the page's
    // own words. `view::band_of` is written as `>=` and falls through to
    // green because of this sentence, and a page that dropped it would leave
    // the one half of the table that cannot be read off the numbers.
    assert!(
        page.contains("A value on a boundary rounds to the worse band"),
        "{PAGE} no longer says that a value on a boundary rounds to the worse band, and \
         `view::band_of` is written from that sentence"
    );

    // And the scale's own derivation, which is where 4 ms comes from at all:
    // four slots to a frame, 16.7 ms at 60 Hz.
    assert!(page.contains("16.7 ms at 60 Hz is about 4 ms each"));
}

/// The five band colours are the room's own five, and neither room wears the
/// other's.
///
/// `--c-band-green` and its four siblings are stated twice in the stylesheet
/// like every other `--c-*`: once for the page under `@media
/// (prefers-color-scheme: …)` and once on `.console.day` and `.console.night`,
/// which is what the room switch toggles. The `.console.*` pair is what `room`
/// transcribes, so it is the pair this reads — the page's are the surrounding
/// document's and only happen to agree.
///
/// This is the check the twelve moods above it do not have. `Palette` sits
/// outside the scan at the top of this file — `SOURCES` starts at `pub mod size
/// {`, so no colour in `room.rs` is held against the stylesheet by anything.
/// These five are held because they are new; the other twelve are named in the
/// report that added them.
#[test]
fn the_band_colours_are_the_rooms_own_five() {
    let root = workspace();
    let css = fs::read_to_string(root.join(STYLESHEET)).expect("the stylesheet");
    let mut sheet = Vec::new();
    rules(&strip_css_comments(&css), &mut sheet);

    for (selector, pal) in [
        (".console.day", Palette::DAY),
        (".console.night", Palette::NIGHT),
    ] {
        for band in Band::ALL {
            let property = format!("--c-band-{}", band.word());
            let found = declared(&sheet, selector, &property)
                .unwrap_or_else(|| panic!("{STYLESHEET} has no `{selector}`"))
                .unwrap_or_else(|| panic!("`{selector}` in {STYLESHEET} does not set {property}"));
            let [r, g, b, _] = band.colour(&pal).to_srgba_unmultiplied();
            assert_eq!(
                found,
                format!("#{r:02x}{g:02x}{b:02x}"),
                "\n{selector} {{ {property}: {found} }} and the palette carries \
                 #{r:02x}{g:02x}{b:02x}.\nThe stylesheet is the specification.\n"
            );
        }

        // **The class the mock draws the dot with**, one per band, each
        // setting the property above and nothing else. It is what makes
        // `Band::word` a transcription rather than a name somebody picked.
        for band in Band::ALL {
            let selector = format!(".risk.{}", band.word());
            let found = declared(&sheet, &selector, "background")
                .unwrap_or_else(|| panic!("{STYLESHEET} has no `{selector}`"))
                .unwrap_or_else(|| panic!("`{selector}` does not set a background"));
            assert_eq!(found, format!("var(--c-band-{})", band.word()));
        }
    }
}
