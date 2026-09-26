//! Verification for the hover and tooltip layer.
//!
//! Tests tooltip table coverage against registered input probes, citation validity
//! against `docs/manual/console.html`, HTML entity scrubbing, dwell timing, and boundary placement.

mod common;

use std::collections::BTreeSet;
use std::time::Duration;

use karakuri_console::hover::{
    elements, flat, Hover, Tip, Tips, TIPS, TIP_GAP, TIP_LINE, TIP_MAX_W, TIP_MIN_W, TIP_PAD_X,
    TIP_PAD_Y, TIP_RADIUS, TIP_SIZE,
};
use karakuri_console::input::{Claim, PROBES};
use karakuri_console::panel::Panel;
use karakuri_console::repaint::{Change, Repaint};
use karakuri_console::room::Room;
use karakuri_console::view::{transport, View};
use karakuri_layout::Point;

/// The rows the mock is silent about, and the roadmap's reading rule said as a
/// value: *the mock is not exhaustive … there will be gaps in the functions
/// too*. A row that quietly lost its tips fails against this list, and a row
/// that gains one in the page is a line to delete from it.
const SILENT: [&str; 5] = [
    // The page tips a pane head's count and its `keep` capsule, and says
    // nothing about the name between them.
    "the Inspector pane heads' name",
    // The slot's MCP policy pill is an agent control with no page tip.
    "the Inspector pane heads' slot mcp policy",
    // `.grip` carries no tip; folding is explained in the page's prose.
    "the grip in a bay head",
    // The page tips the rows of a *reading* and the items of a row's menu,
    // neither of which is the listing row this probe claims.
    "the Library bay's list",
    // The `back` capsule has no tip of its own: the page tips the candidate
    // row it sits in.
    "the Staging lane's back capsules",
];

/// How many tipped elements the scan must find at the least. A floor and not a
/// count: the page only grows, and what this catches is a scan that has stopped
/// reading — `karakuri-engine/tests/gpu_tests_are_under_mod_gpu.rs`'s shape. It
/// found 140 on 2026-09-09.
const ELEMENTS_FLOOR: usize = 120;

use common::default_console as console;

/// A console with an engine behind it, which is what draws a transport row.
fn view() -> View {
    let mut view = View::new(Room::Day);
    view.transport = Some(common::mock_transport());
    view
}

/// The middle of the tempo figure — a control with a tip, reached the way the
/// window reaches it.
fn on_tempo(panel: &Panel, ctx: &egui::Context, view: &View) -> Point {
    let row =
        transport(ctx, panel.layout(), view.transport).expect("a row with an engine behind it");
    Point::new(row.bpm.center().x, row.bpm.center().y)
}

/// Run one pass and let the layer paint into it.
fn paint(ctx: &egui::Context, hover: &mut Hover, panel: &Panel, view: &View, now: Duration) {
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
        hover.paint(ui, panel, view, now)
    });
    // A pass that laid a galley out asks for a texture upload, and `epaint`
    // panics on a delta nobody applied — `common::drawn_once`'s own line.
    out.textures_delta.clear();
}

#[test]
fn the_table_is_the_consoles_own_rows_in_the_consoles_own_order() {
    // The length is the compiler's — `TIPS` is `[_; PROBES.len()]`.
    for (at, (name, _)) in TIPS.iter().enumerate() {
        assert_eq!(
            *name, PROBES[at].name,
            "row {at} of `hover::TIPS` says it answers for `{name}`, and row {at} of \
             `karakuri_console::input::PROBES` is `{}`",
            PROBES[at].name
        );
    }
}

#[test]
fn every_drawn_control_with_a_page_tip_resolves() {
    let tips = Tips::read();
    let missing: Vec<&str> = tips
        .missing()
        .map(|index| flat().nth(index).expect("an index out of `flat`").control)
        .collect();
    assert!(
        missing.is_empty(),
        "these controls cite an element `docs/manual/console.html` does not have: {missing:?}"
    );
    assert_eq!(tips.len(), flat().count());
}

#[test]
fn a_row_with_no_tips_is_one_the_mock_is_silent_about() {
    for (name, tips) in TIPS {
        assert_eq!(
            tips.is_empty(),
            SILENT.contains(&name),
            "`{name}` has {} tips and `SILENT` says {}",
            tips.len(),
            match SILENT.contains(&name) {
                true => "it has none",
                false => "it has some",
            }
        );
    }
    // Verify exactly the four silent rows have no tooltip entries.
    let empty: Vec<&str> = TIPS
        .iter()
        .filter(|(_, tips)| tips.is_empty())
        .map(|(name, _)| *name)
        .collect();
    assert_eq!(
        empty, SILENT,
        "the rows the mock is silent about are {empty:?} and `SILENT` names {SILENT:?}"
    );
}

/// Counts expected hover tooltip entries for rows where control count and tooltip count differ.
const UNEVEN: [(&str, &str, usize); 8] = [
    // One kind of control and four chips drawn: the page tips all four sinks
    // and the two plugin chips switch nothing, so `Outputs::chip_at` answers
    // `None` on them and there is nothing for a cite to resolve against.
    ("the Outputs row's sinks", "two of the four chips answer", 2),
    // Effect rows are deferred to milestone M5.16 pass 2b (ADR-0340).
    (
        "the Master bay's five",
        "the chain's rows are M5.16 pass 2b's",
        2,
    ),
    // The second control is the card the mark puts down, and the mock draws
    // the mark alone.
    (
        "the Inspector pane heads' deck pulldown",
        "the card carries no tip",
        1,
    ),
    // Seven controls and six tips: `.scrub` is one element over both arrows,
    // which is one control to a hand and `DeckHead::scrub`'s own answer.
    (
        "a deck head's seven",
        "the scrub's two arrows are one tip",
        6,
    ),
    // One control and two entries: the mark is drawn as a number where the
    // interface carries the control and as a dot where it does not, and the
    // page tips both faces.
    (
        "a parameter row's publish mark",
        "the mark has two faces",
        2,
    ),
    // The capsule is tipped and the card it opens is not.
    (
        "a node group's `uses` capsule and its card",
        "the card carries no tip",
        1,
    ),
    // The auth group is tipped as a single unit across its three authority chips.
    (
        "a node head's three authority chips",
        "one element over the three",
        1,
    ),
    // Sequencer steps and lane labels each provide one tooltip entry alongside fixed controls.
    (
        "the Sequencer bay's cells, labels, minus glyphs, mode pill, bank pills and + lane",
        "the cells, the labels and the minus glyphs are per lane",
        9,
    ),
];

/// Rows with multiple compact controls provide individual tooltips for each control.
#[test]
fn a_row_is_tipped_at_the_granularity_it_claims() {
    for (at, (name, tips)) in TIPS.iter().enumerate() {
        if SILENT.contains(name) || UNEVEN.iter().any(|(row, _, _)| row == name) {
            continue;
        }
        assert_eq!(
            tips.len(),
            PROBES[at].claims,
            "`{name}` claims {} controls in `input::PROBES` and explains {} of them — either \
             it is a control short, or the row belongs in `UNEVEN` with the reason",
            PROBES[at].claims,
            tips.len()
        );
    }
    // Verify that every row listed in UNEVEN is valid and matches its expected tip count.
    for (name, why, entries) in UNEVEN {
        let (at, (_, tips)) = TIPS
            .iter()
            .enumerate()
            .find(|(_, (row, _))| *row == name)
            .unwrap_or_else(|| panic!("`UNEVEN` names `{name}`, which is no row of `TIPS`"));
        assert_eq!(
            tips.len(),
            entries,
            "`{name}` is excused as `{why}` and carries {} tips where this list says {entries}",
            tips.len()
        );
        assert_ne!(
            tips.len(),
            PROBES[at].claims,
            "`{name}` is excused as `{why}` and now carries one tip per control it claims — \
             the line is to delete"
        );
    }
}

#[test]
fn no_two_controls_cite_one_element() {
    let mut seen = BTreeSet::new();
    for tip in flat() {
        let cite = (tip.cites.class, tip.cites.text, tip.cites.nth);
        assert!(
            seen.insert(cite),
            "`{}` cites the same element as a control before it: {cite:?}",
            tip.control
        );
    }
}

#[test]
fn the_scan_reads_the_whole_page() {
    let found = elements(karakuri_console::hover::PAGE);
    assert!(
        found.len() >= ELEMENTS_FLOOR,
        "the scan found {} tipped elements in `docs/manual/console.html` and the floor is \
         {ELEMENTS_FLOOR} — a scan that has stopped reading passes every other check here",
        found.len()
    );
    assert!(found.iter().all(|el| !el.tip.is_empty()));
}

#[test]
fn no_tip_carries_an_unresolved_entity() {
    let tips = Tips::read();
    for (index, tip) in flat().enumerate() {
        let words = tips.get(index).expect("every cite resolves");
        // An entity is an `&`, a short name and a `;`. Anything longer than
        // that is prose about an ampersand rather than markup.
        let left = words.split('&').skip(1).find_map(|tail| {
            let name = tail.split(';').next()?;
            (tail.len() > name.len() && name.len() < 10 && !name.contains(' ')).then_some(name)
        });
        assert!(
            left.is_none(),
            "`{}`'s tip still carries an entity the scan cannot read: `&{};`",
            tip.control,
            left.unwrap_or_default()
        );
    }
}

#[test]
fn the_box_is_the_mocks_own_rule() {
    let css = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/manual/style.css"
    ))
    .expect("the stylesheet the console transcribes");
    let at = css.find("[data-tip]::after").expect("the tip's own rule");
    let rule = &css[at..at + css[at..].find('}').expect("a closed rule")];
    for declaration in [
        "min-width: 150px",
        "max-width: 236px",
        "padding: 7px 9px",
        "border-radius: 9px",
        "font-size: 10.5px",
        "line-height: 1.55",
        "top: calc(100% + 6px)",
        "box-shadow: 0 8px 26px rgba(0,0,0,0.22)",
    ] {
        assert!(
            rule.contains(declaration),
            "`[data-tip]::after` no longer says `{declaration}`, and `karakuri_console::hover` \
             is transcribing it"
        );
    }
    // And the constants are the numbers in those declarations.
    assert_eq!(TIP_MIN_W, 150.0);
    assert_eq!(TIP_MAX_W, 236.0);
    assert_eq!((TIP_PAD_Y, TIP_PAD_X), (7.0, 9.0));
    assert_eq!(TIP_RADIUS, 9.0);
    assert_eq!(TIP_SIZE, 10.5);
    assert_eq!(TIP_LINE, 1.55);
    assert_eq!(TIP_GAP, 6.0);
}

#[test]
fn a_tip_appears_after_the_dwell_and_not_before() {
    let (panel, ctx) = console();
    let view = view();
    let at = on_tempo(&panel, &ctx, &view);
    let dwell = Hover::dwell(&ctx);
    let mut hover = Hover::new();

    assert_eq!(
        hover.moved(Claim::Panel, &panel, &ctx, &view, at, Duration::ZERO),
        Tip::Still,
        "a move that starts a dwell owes no frame of its own"
    );
    assert_eq!(
        hover.resting_on().map(|tip| tip.control),
        Some("the tempo figure")
    );
    assert_eq!(hover.owed(&ctx, Duration::ZERO), Tip::Dwelling(dwell));

    // One tick short of the dwell: still nothing, and the deadline is what is
    // left of it.
    let nearly = dwell - Duration::from_millis(1);
    paint(&ctx, &mut hover, &panel, &view, nearly);
    assert_eq!(
        hover.showing(),
        None,
        "a tip drawn before its dwell has run"
    );
    assert_eq!(
        hover.owed(&ctx, nearly),
        Tip::Dwelling(Duration::from_millis(1))
    );

    paint(&ctx, &mut hover, &panel, &view, dwell);
    let words = hover
        .showing()
        .expect("the tip is up once the dwell has run");
    assert!(
        words.starts_with("What the grid is running at"),
        "the words are the page's own: {words:.60}"
    );
    assert!(!words.contains("&#"), "and they are not the page's markup");

    // Up, and asking for nothing: the box does not move while it is shown.
    assert_eq!(hover.owed(&ctx, dwell * 2), Tip::Still);

    // The frame where dwell expires requests no further frames since it is handled during painting.
    let mut fresh = Hover::new();
    fresh.moved(Claim::Panel, &panel, &ctx, &view, at, Duration::ZERO);
    assert_eq!(fresh.owed(&ctx, dwell), Tip::Still);
}

#[test]
fn it_goes_when_the_pointer_leaves() {
    let (panel, ctx) = console();
    let view = view();
    let at = on_tempo(&panel, &ctx, &view);
    let dwell = Hover::dwell(&ctx);
    let mut hover = Hover::new();
    hover.moved(Claim::Panel, &panel, &ctx, &view, at, Duration::ZERO);
    paint(&ctx, &mut hover, &panel, &view, dwell);
    assert!(hover.showing().is_some());

    // Off the control and onto the console's ground, which is `egui`'s.
    let ground = Point::new(at.x, at.y + 400.0);
    assert_eq!(
        hover.moved(Claim::Egui, &panel, &ctx, &view, ground, dwell),
        Tip::Gone,
        "a tip on screen that must not be is owed a frame now"
    );
    assert_eq!(hover.showing(), None);
    assert_eq!(hover.resting_on().map(|tip| tip.control), None);
    assert_eq!(hover.owed(&ctx, dwell), Tip::Still);

    // Re-entering the control initiates a fresh dwell period rather than reviving the old tip.
    hover.moved(Claim::Panel, &panel, &ctx, &view, at, dwell);
    assert_eq!(
        hover.showing(),
        None,
        "a tip is up again with no dwell behind it"
    );
    assert_eq!(hover.owed(&ctx, dwell), Tip::Dwelling(dwell));

    // And the same from the window's edge rather than from another control.
    paint(&ctx, &mut hover, &panel, &view, dwell * 2);
    assert!(hover.showing().is_some());
    assert_eq!(hover.left(), Tip::Gone);
    assert_eq!(hover.showing(), None);
}

#[test]
fn a_drag_is_not_a_hover() {
    let (panel, ctx) = console();
    let view = view();
    let at = on_tempo(&panel, &ctx, &view);
    let mut hover = Hover::new();
    // `claim` gives a drag to the panel wherever the pointer is (rule 1), and
    // a gesture in hand is the pointer being used rather than pointed.
    hover.moved(Claim::Egui, &panel, &ctx, &view, at, Duration::ZERO);
    assert_eq!(hover.resting_on().map(|tip| tip.control), None);
}

#[test]
fn the_layer_asks_for_a_frame_only_on_appear_or_move() {
    assert_eq!(Change::Tip(Tip::Still).repaint(), Repaint::Never);
    assert_eq!(Change::Tip(Tip::Gone).repaint(), Repaint::Now);
    assert_eq!(
        Change::Tip(Tip::Dwelling(Duration::from_millis(500))).repaint(),
        Repaint::After(Duration::from_millis(500))
    );
}

#[test]
fn a_tip_at_an_edge_is_inside_the_window() {
    let viewport = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1440.0, 900.0));
    let size = egui::vec2(236.0, 120.0);
    for at in [
        Point::new(0.0, 0.0),
        Point::new(1439.0, 0.0),
        Point::new(0.0, 899.0),
        Point::new(1439.0, 899.0),
        Point::new(720.0, 450.0),
    ] {
        let box_ = karakuri_console::hover::placed(viewport, at, size);
        assert!(
            viewport.contains_rect(box_),
            "a tip at {at:?} is drawn at {box_:?}, which leaves the window {viewport:?}"
        );
    }
    // Down and to the right of the pointer where there is room for it, which
    // is the mock's `top: calc(100% + 6px)`.
    let under = karakuri_console::hover::placed(viewport, Point::new(100.0, 100.0), size);
    assert_eq!(under.min, egui::pos2(100.0, 100.0 + TIP_GAP));

    // Verify that tooltips flip rather than clamp near viewport edges (.tip-right).
    let near_right = karakuri_console::hover::placed(viewport, Point::new(1300.0, 100.0), size);
    assert!(
        near_right.max.x <= 1300.0,
        "a tip near the right edge is drawn at {near_right:?} and opens over the pointer"
    );
    let near_bottom = karakuri_console::hover::placed(viewport, Point::new(100.0, 800.0), size);
    assert!(
        near_bottom.max.y <= 800.0,
        "a tip near the bottom edge is drawn at {near_bottom:?} and opens over the pointer"
    );
}

/// Tooltip MIDI assignments reflect the live mapping, leaving unmapped controls with base descriptions (P-0087, ADR-0336).
#[test]
fn the_tips_midi_line_is_read_off_the_map_and_the_pages_reason_survives() {
    use karakuri_console::hover::{assigned, MIDI_LINE};

    let mapped = "The trim. Click to drag. \u{2295} MIDI: cc → gain A, which sets it outright \
                  where a key steps it.";
    let live = assigned(mapped, Some("cc 5"));
    assert!(
        live.ends_with("\u{2295} MIDI: cc 5, which is what the map in use says today."),
        "the tip did not take the live assignment: {live}"
    );
    assert!(
        live.starts_with("The trim. Click to drag."),
        "the rest of the tip did not survive: {live}"
    );
    assert!(
        !live.contains("gain A"),
        "the mock's own assignment is still in the tip: {live}"
    );

    // Unmapped controls preserve the documentation's original explanatory text.
    let why = "Which scope the library shows. \u{2295} MIDI: unassigned — a map line names a \
               word from a closed list, and this list grows when a directory is added.";
    assert_eq!(assigned(why, None), why);
    // Even with an assignment in hand, a tip with no clause is untouched.
    let silent = "A control the mock says nothing about.";
    assert_eq!(assigned(silent, Some("cc 5")), silent);

    // Validates transcribed MIDI punctuation prefix against documentation source.
    let page = karakuri_console::hover::PAGE;
    assert!(
        page.contains("&#8853; MIDI:"),
        "the mock no longer spells the tip's last clause `&#8853; MIDI:`"
    );
    assert_eq!(MIDI_LINE, "\u{2295} MIDI:");
}
