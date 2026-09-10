//! **The hover layer: that the words are the page's, that the dwell is a
//! dwell, and that a tip stays inside the window.**
//!
//! Six things, and the first three are what stop the layer becoming a second
//! copy of the manual:
//!
//! 1. **The table is the console's own rows, at the granularity they claim.**
//!    `hover::TIPS` is one entry per row of `input::PROBES`, in that order, so
//!    a control registered there and never given a tip is a compile error and
//!    an entry cannot answer for its neighbour — and a row that claims five
//!    controls carries five entries unless `UNEVEN` says why it does not,
//!    which is the half of *every compact control explains itself* that the
//!    array's length cannot hold.
//! 2. **Every citation resolves to the one element it names**, in
//!    `docs/manual/console.html`. This is the half that goes stale in silence:
//!    nothing about editing the page tells you a `Cite` was reading it, so the
//!    check is *does the element this comment names still exist* rather than
//!    *is this text plausible* — `tests/transcribed_constants_cite_the_mock.rs`'s
//!    argument, over the page rather than over the stylesheet.
//! 3. **No tip arrives with an entity still in it.** The mock writes
//!    `&#8853;` and `&mdash;`, and a panel drawing those five characters is
//!    drawing markup at an operator. The scan refuses what it cannot read
//!    rather than reading it wrongly.
//! 4. **The box is the mock's own rule**, term for term out of
//!    `[data-tip]::after`.
//! 5. **The dwell is a dwell**: nothing before it, the page's words after it,
//!    and nothing at all once the pointer leaves.
//! 6. **A tip is inside the window** at every corner, and it asks for a frame
//!    when it appears or goes and never while it is up.
//!
//! None of it needs a window, a device or a disk — the page is compiled in.

mod common;

use std::collections::BTreeSet;
use std::time::Duration;

use common::{drawn_once, PLAUSIBLE};
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

/// **The rows the mock is silent about**, and the roadmap's reading rule said
/// as a value: *the mock is not exhaustive … there will be gaps in the
/// functions too*. A row that quietly lost its tips fails against this list,
/// and a row that gains one in the page is a line to delete from it.
const SILENT: [&str; 4] = [
    // The page tips a pane head's count and its `keep` capsule, and says
    // nothing about the name between them.
    "the Inspector pane heads' name",
    // `.grip` carries no tip; folding is explained in the page's prose.
    "the grip in a bay head",
    // The page tips the rows of a *reading* and the items of a row's menu,
    // neither of which is the listing row this probe claims.
    "the Library bay's list",
    // The `back` capsule has no tip of its own: the page tips the candidate
    // row it sits in.
    "the Staging lane's back capsules",
];

/// **How many tipped elements the scan must find at the least.** A floor and
/// not a count: the page only grows, and what this catches is a scan that has
/// stopped reading — `karakuri-engine/tests/gpu_tests_are_under_mod_gpu.rs`'s
/// shape. It found 140 on 2026-09-09.
const ELEMENTS_FLOOR: usize = 120;

/// A panel at a plausible viewport, solved, with a context that has drawn
/// once — every other test in this crate's opening.
fn console() -> (Panel, egui::Context) {
    let mut panel = Panel::new(PLAUSIBLE.w, PLAUSIBLE.h);
    panel.solve();
    (panel, drawn_once())
}

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
    // **And there are exactly four of them**, which is the reading rule as a
    // number: the four above and no other row. The loop is an `iff` per row
    // and would still pass with a name in `SILENT` that no row has, which is
    // how a row renamed out of this list stops being checked at all.
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

/// **The rows whose slice is not one entry per control the row claims, and
/// why each one is not.**
///
/// `input::PROBES` counts what the pointer reaches through a row and `TIPS`
/// says what explains itself, so the two numbers agree wherever the mock tips
/// a row's controls one at a time. Where they do not, the reason is one of
/// four and it is written down here rather than being a number that drifts:
/// the page tips one element over several controls, it tips several elements
/// for one, the second control is a card the page says nothing about, or the
/// tips are owed.
///
/// **The third column is how many entries the row does carry**, and it is here
/// for the reason the check exists at all: an excused row is out from under
/// `PROBES`' count, so without a number of its own an entry could be deleted
/// from one and nothing would say so. It is a transcribed count and it moves
/// with the bay, which is what makes touching one of these rows a line to
/// re-read rather than a number to bump.
///
/// **A row that gains its missing entries is a line to delete from here**, and
/// deleting it is what puts the row back under the count.
const UNEVEN: [(&str, &str, usize); 8] = [
    // One kind of control and four chips drawn: the page tips all four sinks
    // and the two plugin chips switch nothing, so `Outputs::chip_at` answers
    // `None` on them and there is nothing for a cite to resolve against.
    ("the Outputs row's sinks", "two of the four chips answer", 2),
    // **The effect rows are owed**, and to `docs/roadmap.md`'s M5.16 pass 2b
    // rather than to this milestone: ADR-0340 makes the master chain an
    // ordered list of kind L5 procedures, and a tip per row of the chain the
    // mock draws would be a citation of a bay about to be rebuilt.
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
    // `.auth` is one element the page tips once, and the three chips inside it
    // are one reading — which is why `AUTHORITIES.len()` is the claim and one
    // cite is the tip.
    (
        "a node head's three authority chips",
        "one element over the three",
        1,
    ),
    // A cell per drawn step and a label per lane are as many as the pattern
    // has: the page draws four lanes of sixteen and this console draws
    // whatever the host handed it, so the cells and the labels are one entry
    // each and the fixed controls — four banks, the mode pill, `+ lane` — are
    // one apiece.
    (
        "the Sequencer bay's cells, labels, mode pill, bank pills and + lane",
        "the cells and the labels are per lane",
        8,
    ),
];

/// **A row claiming several controls carries several tips**, which is the
/// exit `docs/roadmap.md`'s M5.11 names: *every compact control explains
/// itself on hover*.
///
/// The array being `PROBES.len()` long is what stops a *row* going untipped,
/// and it says nothing about a row that claims five controls and explains one
/// of them — the pointer then resolves to the entry that is there, and four
/// controls quietly say a fifth's words. This is that half: the count is
/// `PROBES`' own, so a control added to a row arrives here as a failure with
/// the row named.
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
    // **And every exception is a row that needs one, and carries what it
    // says it carries.** A name that no row has, or a row that has since been
    // tipped whole, is a line to delete: left there it takes that row out of
    // the check above for good. The count is what keeps an excused row from
    // quietly losing an entry — the one thing the check above would have
    // caught for it.
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

    // **And the frame the dwell runs out on asks for nothing either**, which
    // is the one frame a `Dwelling(0)` would buy: the caller asks this inside
    // the pass that paints the tip, so the deadline has already been met by
    // the frame it is being asked on.
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

    // **And back on again is a new dwell and not the old tip.** A layer that
    // took the tip down by forgetting where the pointer was would pass every
    // line above and show the words again the instant the pointer returned.
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

    // **And it flips rather than being pushed.** A box merely clamped to the
    // window would sit *over* the control it is explaining, which is what the
    // mock's `.tip-right` and its `.outputs` rule exist to stop: near an edge
    // the box opens back towards the pointer and leaves it on the edge of the
    // box. Measured away from the corner, where a clamp and a flip land in
    // the same place and neither tells you which happened.
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

/// **The tip's MIDI line is the live map's, and the page's sentence stays
/// where a control is unmapped.**
///
/// The page's `⊕ MIDI:` clause is the *mock's* assignment and no operator's:
/// a run whose map puts `cc 5` on gain A read `cc → gain A` in the tip because
/// the page said so, which is the tip being confidently wrong about the one
/// thing somebody hovers to check (P-0087, ADR-0336).
///
/// Three cases and only one of them rewrites anything, which is what this
/// checks — including that a tip with no clause is left exactly as written,
/// because the mock is not exhaustive and inventing one would be this console
/// writing the manual.
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

    // **Unmapped keeps the page's words**, because they carry the reason and
    // no reverse lookup can produce one.
    let why = "Which scope the library shows. \u{2295} MIDI: unassigned — a map line names a \
               word from a closed list, and this list grows when a directory is added.";
    assert_eq!(assigned(why, None), why);
    // Even with an assignment in hand, a tip with no clause is untouched.
    let silent = "A control the mock says nothing about.";
    assert_eq!(assigned(silent, Some("cc 5")), silent);

    // **And the constant is the page's own punctuation**, which is the one
    // thing here that is transcribed: this fails if the mock stops ending its
    // tips that way, which is what makes the transcription safe.
    let page = karakuri_console::hover::PAGE;
    assert!(
        page.contains("&#8853; MIDI:"),
        "the mock no longer spells the tip's last clause `&#8853; MIDI:`"
    );
    assert_eq!(MIDI_LINE, "\u{2295} MIDI:");
}
