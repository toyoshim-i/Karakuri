//! **The name in an Inspector pane's head: this console's second
//! letter-taking flow.**
//!
//! The head reads `showing  deck A · drift_night` with the `keep` capsule at
//! the other end of the same row. A press on the name puts the head into a
//! naming state, letters go into it, return files the deck under what was
//! typed, and escape leaves it alone —
//! [ADR-0292](../../../docs/adr/0292-the-pane-heads-name-takes-letters-and-the-keep-capsule-stays-a-stamp.md).
//!
//! Nine things:
//!
//! 1. Where the run sits, as `.half-head`'s flex row lays it out — after the
//!    label, one gap along, aligned with the capsule at the other end.
//! 2. **That it clears every boundary's grab**, which is `keep_pill.rs`'
//!    arithmetic at the other end of the same row: the capsule's nearest
//!    boundary is the pane divider on its right, and this one's is the
//!    divider on its left.
//! 3. **Where the `▾` boundary is.** The mock's chevron means *point this
//!    pane at another deck* and is not a control this console has. The name
//!    target stops at the run's own ink and the chevron's rectangle is
//!    reserved beside it, so the day the chooser lands it takes that place
//!    rather than taking it back.
//! 4. That the run never reaches the `keep` capsule.
//! 5. That a press opens the field in that head and in no other, and that one
//!    head asks at a time.
//! 6. That the commit is `SaveSet { deck, id: Some(typed) }` for the deck that
//!    head is showing — and that **the capsule beside it still files under a
//!    stamp**, which is ADR-0287 surviving as the unnamed route.
//! 7. That escape leaves the deck alone, and that a half-typed name is not
//!    kept for next time.
//! 8. That a head with no room for the run draws none, and that a console
//!    that has not drawn has none — `keep_pill.rs`' two guards, on a readout
//!    instead of on a capsule.
//! 9. That the field is *painted*: the label reads `keep as` and the run reads
//!    what was typed with the caret after it.
//!
//! **What is not here and cannot be**: that `input::claim` gives the panel a
//! press on the run, and that the keyboard reaches `View::type_into_name`
//! while a head is asking. Those are `input::PROBES`' row and the window's key
//! arm, in files this test's author does not own.

mod common;

use common::{drawn_once, near, rect_of, PLAUSIBLE, SMALLEST};
use karakuri_console::panel::{Panel, GRAB};
use karakuri_console::room::{size, Room};
use karakuri_console::view::{
    deck_name, inspector, keep_pill, InspectorPane, Pane, View, PANES, PANE_NAMES, SYNCS,
};
use karakuri_layout::{Point, Rect};
use karakuri_operation::{Operation, Sync};

/// The mock's own deck A, which is the head the mock draws
/// `deck A · drift_night` in.
fn mock() -> Pane {
    Pane {
        deck: 0,
        material: "drift_night".to_owned(),
        sync: Sync::Tempo,
        allows: [true; SYNCS.len()],
        anchor_bpm: 128.0,
        scrub_beats: 0.0,
        composite: true,
        nodes: Vec::new(),
    }
}

/// That pane pointed at another deck.
fn showing(deck: usize) -> Pane {
    Pane { deck, ..mock() }
}

/// A panel at a viewport, solved, with a context that has drawn once.
fn console(viewport: Rect) -> (Panel, egui::Context) {
    let mut panel = Panel::new(viewport.w, viewport.h);
    panel.solve();
    (panel, drawn_once())
}

/// A `karakuri_layout` point, from `egui`'s.
fn at(p: egui::Pos2) -> Point {
    Point::new(p.x, p.y)
}

/// **How wide a run of the head's own type is**, asked of the same fonts the
/// paint asks. Written here rather than imported so that a change to the
/// derivation has to be made twice before it can pass unnoticed.
fn run(ctx: &egui::Context, text: &str) -> f32 {
    ctx.fonts_mut(|f| {
        f.layout_no_wrap(
            text.to_owned(),
            egui::FontId::new(size::BASE, egui::FontFamily::Proportional),
            egui::Color32::PLACEHOLDER,
        )
        .size()
        .x
    })
}

/// A view with two panes and nothing else — a console with a deck behind it
/// and no store, which is what every test in this file is.
fn view_of(panes: [Pane; PANES]) -> View {
    let mut view = View::new(Room::Day);
    view.inspector = panes.into_iter().collect();
    view
}

// ---------------------------------------------------------------------------
// Where the control is
// ---------------------------------------------------------------------------

/// **The run sits one `.half-head` gap after the label**, one padding down
/// from the top, and is as tall as the capsule at the other end of the row —
/// `align-items: center` over a content box whose `border-bottom` is inside
/// the row.
#[test]
fn the_run_sits_one_gap_after_the_label() {
    let pane = mock();
    for viewport in [SMALLEST, PLAUSIBLE] {
        let (panel, ctx) = console(viewport);
        for index in 0..PANES {
            let at_pane = inspector(panel.layout(), index, &pane).expect("a pane with room in it");
            let named = deck_name(&ctx, &at_pane, &pane, None).expect("a head with room for it");
            let head = at_pane.head;

            let left =
                head.min.x + size::HALF_HEAD_PAD_X + run(&ctx, "showing") + size::HALF_HEAD_GAP;
            assert!(
                near(named.name.min.x, left),
                "the run starts at {} and `showing` inside the head's padding ends one gap before \
                 {left}",
                named.name.min.x
            );
            assert!(
                near(named.name.min.y, head.min.y + size::HALF_HEAD_PAD_Y),
                "the run is {} down from the top of the head and `.half-head`'s padding is {}",
                named.name.min.y - head.min.y,
                size::HALF_HEAD_PAD_Y
            );
            assert!(
                near(named.name.height(), size::PILL_H),
                "the run's box is {} tall and the row's content is `BASE` at `LINE`, which is {}",
                named.name.height(),
                size::PILL_H
            );
            assert!(
                near(named.name.width(), run(&ctx, "deck A · drift_night")),
                "the run's box is {} wide and `deck A · drift_night` measures {}",
                named.name.width(),
                run(&ctx, "deck A · drift_night")
            );
            assert!(
                head.contains_rect(named.name),
                "the run is not inside the head it is drawn in"
            );
            assert_eq!(named.deck, pane.deck);
        }
    }
}

/// **The run never reaches the `keep` capsule**, which is the clip
/// `inspector_into` paints the words inside: everything up to the capsule, one
/// `.half-head` gap short of it.
#[test]
fn the_run_stops_one_gap_short_of_the_capsule() {
    let pane = mock();
    for viewport in [SMALLEST, PLAUSIBLE] {
        let (panel, ctx) = console(viewport);
        for index in 0..PANES {
            let at_pane = inspector(panel.layout(), index, &pane).expect("a pane with room in it");
            let named = deck_name(&ctx, &at_pane, &pane, None).expect("a head with room for it");
            let pill = keep_pill(&ctx, &at_pane, &pane).expect("a head with room for its capsule");
            assert!(
                named.name.max.x <= pill.pill.min.x - size::HALF_HEAD_GAP + common::EPS,
                "the run ends at {} and the capsule starts at {}, which is less than the row's \
                 own gap of {} away",
                named.name.max.x,
                pill.pill.min.x,
                size::HALF_HEAD_GAP
            );
            assert!(
                !pill.hit(at(named.name.right_center())),
                "the capsule claims the right-hand end of the run"
            );
        }
    }
}

/// **A long name is clipped rather than growing over the capsule**, which is
/// the row's own answer to a long name either way — there is no ellipsis in
/// this console to draw.
#[test]
fn a_name_too_long_for_the_head_is_clipped_at_the_capsule() {
    let pane = Pane {
        material: "a".repeat(200),
        ..mock()
    };
    let (panel, ctx) = console(PLAUSIBLE);
    let at_pane = inspector(panel.layout(), 0, &pane).expect("a pane with room in it");
    let named = deck_name(&ctx, &at_pane, &pane, None).expect("a head with room for some of it");
    let pill = keep_pill(&ctx, &at_pane, &pane).expect("a head with room for its capsule");
    assert!(
        named.name.max.x <= pill.pill.min.x - size::HALF_HEAD_GAP + common::EPS,
        "a 200-character name ran the target under the capsule"
    );
    assert!(
        named.name.width() < run(&ctx, &format!("deck A · {}", "a".repeat(200))),
        "the target is as wide as the whole name, so it covers ink the head does not paint"
    );
}

/// **A head with no room for any of the run draws none**, which is
/// `keep_pill`'s rule read on a readout: a target over ink nobody can see is a
/// press that lands on nothing an operator could have aimed at.
///
/// The narrow head is built here rather than solved for, because the panel's
/// own minimum is far wider than this — `keep_pill.rs`' reason, one control
/// along.
#[test]
fn a_head_with_no_room_for_the_run_draws_none() {
    let pane = mock();
    let (panel, ctx) = console(PLAUSIBLE);
    let at_pane = inspector(panel.layout(), 0, &pane).expect("a pane with room in it");
    let full = at_pane.head;
    let narrowed = |w: f32| InspectorPane {
        head: egui::Rect::from_min_size(full.min, egui::vec2(w, full.height())),
        ..at_pane
    };
    // The label inside the head's left padding, one gap, and then the capsule
    // and the gap the words stop short of it by: a head exactly this wide has
    // the run starting where the run must already have stopped.
    let capsule = run(&ctx, "keep") + size::PILL_PAD_X * 2.0;
    let bare = size::HALF_HEAD_PAD_X
        + run(&ctx, "showing")
        + size::HALF_HEAD_GAP
        + size::HALF_HEAD_GAP
        + capsule
        + size::HALF_HEAD_PAD_X;
    assert!(
        deck_name(&ctx, &narrowed(bare + 1.0), &pane, None).is_some(),
        "a head with one pixel of room for the run drew none, so this test cannot tell a fit \
         from a refusal"
    );
    assert_eq!(
        deck_name(&ctx, &narrowed(bare), &pane, None),
        None,
        "a head with no room left between the label and the capsule drew a target anyway, and \
         `inspector_into`'s clip is what paints no ink under it"
    );
}

/// **Before the first frame there is nothing here to press.**
/// `Context::fonts` is not valid until a pass has run, and this run is as wide
/// as the words in it — the same guard every measured control carries.
#[test]
fn a_console_that_has_not_drawn_has_no_run() {
    let pane = mock();
    let (panel, _) = console(PLAUSIBLE);
    let at_pane = inspector(panel.layout(), 0, &pane).expect("a pane with room in it");
    assert_eq!(
        deck_name(&egui::Context::default(), &at_pane, &pane, None),
        None
    );
}

// ---------------------------------------------------------------------------
// The claim rule, and where the `▾` boundary sits
// ---------------------------------------------------------------------------

/// **The run clears every boundary's grab.** The capsule at the other end of
/// this row is measured against the pane divider on its right; this one's
/// nearest boundary is the divider on its *left*, and what it clears is
/// `.half-head`'s left-hand padding plus the label plus a gap, against a
/// `GRAB` of 6.
#[test]
fn the_run_clears_every_boundarys_grab() {
    let pane = mock();
    for viewport in [SMALLEST, PLAUSIBLE] {
        let (panel, ctx) = console(viewport);
        for (index, name) in PANE_NAMES.iter().enumerate() {
            let at_pane = inspector(panel.layout(), index, &pane).expect("a pane with room in it");
            let named = deck_name(&ctx, &at_pane, &pane, None).expect("a head with room for it");

            let clearance = named.name.min.x - at_pane.head.min.x;
            assert!(
                clearance > GRAB,
                "the run starts {clearance} in from the pane's left edge and a boundary grabs \
                 {GRAB} — the control is inside a boundary's grab, and `input`'s rule is what \
                 has to change"
            );
            let bay = rect_of(panel.layout(), name);
            let above = named.name.min.y - bay.y;
            assert!(
                near(above, size::HEAD_H + size::HALF_HEAD_PAD_Y),
                "the run is {above} down from the pane's top edge, and the bay head plus this \
                 row's padding come to {}",
                size::HEAD_H + size::HALF_HEAD_PAD_Y
            );
            assert!(above > GRAB, "the run has {above} of pane above it");

            for probe in [
                named.name.left_top(),
                named.name.right_top(),
                named.name.left_bottom(),
                named.name.right_bottom(),
                named.name.center(),
            ] {
                assert!(
                    !matches!(
                        panel.layout().hit(at(probe), GRAB),
                        karakuri_layout::Hit::Divider { .. }
                    ),
                    "a boundary grabs {probe:?}, which is on the name of pane {index}"
                );
                assert!(
                    named.hit(at(probe)),
                    "{probe:?} is a corner of the run and the run says it is not on it"
                );
            }
        }
    }
}

/// **The `▾`'s place is reserved and is claimed by nothing.** The chevron
/// means *point this pane at another deck*, which is a control this console
/// does not have: the name target stops at the run's own ink, and the
/// rectangle one gap after it is held so that the chooser takes it rather than
/// taking it back.
#[test]
fn the_chevrons_place_is_beside_the_run_and_is_not_claimed() {
    let pane = mock();
    for viewport in [SMALLEST, PLAUSIBLE] {
        let (panel, ctx) = console(viewport);
        for index in 0..PANES {
            let at_pane = inspector(panel.layout(), index, &pane).expect("a pane with room in it");
            let named = deck_name(&ctx, &at_pane, &pane, None).expect("a head with room for it");

            assert!(
                near(named.chevron.min.x - named.name.max.x, size::HALF_HEAD_GAP),
                "the chevron is {} after the run and `.half-head`'s gap is {}",
                named.chevron.min.x - named.name.max.x,
                size::HALF_HEAD_GAP
            );
            assert!(
                !named.name.intersects(named.chevron),
                "the name target overlaps the place the chooser goes"
            );
            for probe in [
                named.chevron.center(),
                named.chevron.left_center(),
                named.chevron.right_center(),
            ] {
                assert!(
                    !named.hit(at(probe)),
                    "the name claims {probe:?}, which is where the chooser goes"
                );
            }
            // And the gap between them belongs to neither.
            assert!(!named.hit(at(egui::Pos2::new(
                named.name.max.x + size::HALF_HEAD_GAP * 0.5,
                named.name.center().y
            ))));
        }
    }
}

/// **A press off the run asks for nothing**, which is *a control claims what
/// it acts on and no more*: the label to its left is a readout, the deck head
/// under it is four other controls, and the capsule at the far end is the
/// unnamed route.
#[test]
fn a_press_off_the_run_is_not_on_it() {
    let pane = mock();
    let (panel, ctx) = console(PLAUSIBLE);
    let at_pane = inspector(panel.layout(), 0, &pane).expect("a pane with room in it");
    let named = deck_name(&ctx, &at_pane, &pane, None).expect("a head with room for it");
    let pill = keep_pill(&ctx, &at_pane, &pane).expect("a head with room for its capsule");
    let head = at_pane.head;
    for probe in [
        // The label, at the left of the head.
        egui::Pos2::new(head.min.x + size::HALF_HEAD_PAD_X, head.center().y),
        // The gap between the label and the run.
        egui::Pos2::new(
            named.name.min.x - size::HALF_HEAD_GAP * 0.5,
            head.center().y,
        ),
        // The capsule at the other end.
        pill.pill.center(),
        // The padding to the right of it.
        egui::Pos2::new(head.max.x - size::HALF_HEAD_PAD_X * 0.5, head.center().y),
        // The deck head under it.
        at_pane.deck_head.center(),
    ] {
        assert!(
            !named.hit(at(probe)),
            "the name claims {probe:?}, which is not on it"
        );
    }
}

// ---------------------------------------------------------------------------
// What the flow does
// ---------------------------------------------------------------------------

/// **One head asks at a time, and it is the one that was pressed.** The
/// keyboard is taken whole while a name is being asked for, so two open fields
/// would be two places one keystroke could go with nothing on the panel saying
/// which.
#[test]
fn one_head_asks_at_a_time() {
    let mut view = view_of([showing(0), showing(1)]);
    assert!(view.naming_set().is_none(), "a console starts reading");
    assert_eq!(view.naming_set_in(0), None);

    view.name_set(1);
    assert_eq!(view.naming_set_in(1), Some(""));
    assert_eq!(
        view.naming_set_in(0),
        None,
        "the other head is asking for a name nobody pressed it for"
    );
    assert_eq!(view.naming_set().map(|n| n.pane), Some(1));

    view.name_set(0);
    assert_eq!(view.naming_set_in(0), Some(""));
    assert_eq!(view.naming_set_in(1), None, "two heads are asking at once");
}

/// **The letters, the rub-out and the refusal**, which is the arrangement
/// pill's own contract on a second flow: nothing typed is checked, and a
/// control character is not a letter because a newline is the commit.
#[test]
fn the_field_takes_letters_and_gives_them_back() {
    let mut view = view_of([showing(0), showing(1)]);
    assert!(
        !view.type_into_name('n'),
        "a console with no head asking took a character"
    );
    assert!(!view.rub_out_of_name());

    view.name_set(0);
    for c in "glass".chars() {
        assert!(view.type_into_name(c));
    }
    assert_eq!(view.naming_set_in(0), Some("glass"));

    // **Nothing is checked here**, which is the surface owning the affordance
    // and never the authority: a `/` goes in and the wall is where the file is
    // written.
    assert!(view.type_into_name('/'));
    assert_eq!(view.naming_set_in(0), Some("glass/"));
    assert!(view.rub_out_of_name());
    assert_eq!(view.naming_set_in(0), Some("glass"));

    // A newline is Return arriving as text, and that is the commit.
    assert!(!view.type_into_name('\n'));
    assert_eq!(view.naming_set_in(0), Some("glass"));

    for _ in 0..5 {
        view.rub_out_of_name();
    }
    assert_eq!(view.naming_set_in(0), Some(""));
    assert!(
        !view.rub_out_of_name(),
        "an empty name gave a character back"
    );
}

/// **The commit is the deck that head is showing, filed under what was
/// typed** — `SaveSet { deck, id: Some(typed) }`, which is ADR-0128's *an
/// operator's own act gets the name it asked for*.
#[test]
fn the_commit_files_this_heads_deck_under_the_typed_name() {
    for (index, deck) in [(0usize, 0usize), (1, 2)] {
        let mut view = view_of([showing(0), showing(deck)]);
        view.name_set(index);
        for c in "glass_shell".chars() {
            view.type_into_name(c);
        }
        let want = match index {
            0 => 0,
            _ => deck,
        };
        assert_eq!(
            view.named_set(),
            Some(Operation::SaveSet {
                deck: want as u8,
                id: Some("glass_shell".to_owned())
            })
        );
        assert!(
            view.naming_set().is_none(),
            "the field is still asking after the commit"
        );
    }
}

/// **An empty name leaves as an empty name.** The wall is where the bytes are
/// written, in one sentence, by whoever writes them — a head that refused it
/// here would be a rule an operator could only find by experiment.
#[test]
fn an_empty_name_is_emitted_and_refused_elsewhere() {
    let mut view = view_of([showing(0), showing(1)]);
    view.name_set(0);
    assert_eq!(
        view.named_set(),
        Some(Operation::SaveSet {
            deck: 0,
            id: Some(String::new())
        })
    );
}

/// **A commit into a head that has gone emits nothing and takes the field
/// away.** The deck is read at the commit rather than at the press, because
/// this gesture spans frames and what is filed has to be the deck the head
/// says it is filing now.
#[test]
fn a_commit_into_a_pane_that_has_gone_emits_nothing() {
    let mut view = view_of([showing(0), showing(1)]);
    view.name_set(1);
    view.type_into_name('x');
    view.inspector.truncate(1);
    assert_eq!(view.named_set(), None);
    assert!(
        view.naming_set().is_none(),
        "the field outlived the head it was drawn in"
    );
}

/// **Escape leaves the deck alone, and a half-typed name is not kept for the
/// next time**: the buffer is the gesture, and the gesture ended.
#[test]
fn escape_leaves_it_alone_and_keeps_nothing() {
    let mut view = view_of([showing(0), showing(1)]);
    view.name_set(0);
    for c in "half".chars() {
        view.type_into_name(c);
    }
    view.stop_naming_set();
    assert!(view.naming_set().is_none());
    view.name_set(0);
    assert_eq!(
        view.naming_set_in(0),
        Some(""),
        "the field came back with the abandoned name in it"
    );
}

/// **The capsule beside it still files under a stamp**, which is ADR-0287
/// surviving as the *unnamed* route: two routes to one operation, differing in
/// exactly the `id` — ADR-0128's own sentence, drawn on one row.
#[test]
fn the_capsule_is_the_unnamed_route_and_is_unchanged() {
    let pane = mock();
    let (panel, ctx) = console(PLAUSIBLE);
    let mut view = view_of([showing(0), showing(1)]);
    let at_pane = inspector(panel.layout(), 0, &pane).expect("a pane with room in it");
    let pill = keep_pill(&ctx, &at_pane, &pane).expect("a head with room for its capsule");

    view.name_set(0);
    for c in "glass_shell".chars() {
        view.type_into_name(c);
    }
    let named = view.named_set().expect("a name was typed");
    let stamped = pill.keep(at(pill.pill.center())).expect("a press on it");

    assert_eq!(stamped, Operation::SaveSet { deck: 0, id: None });
    assert_eq!(
        named,
        Operation::SaveSet {
            deck: 0,
            id: Some("glass_shell".to_owned())
        }
    );
}

// ---------------------------------------------------------------------------
// What the head draws while it is asking
// ---------------------------------------------------------------------------

/// Every galley the console paints **wholly inside** `rect`, on one frame.
/// `keep_pill.rs`' helper, narrowed to text: `egui` tessellates on the CPU and
/// the device only ever sees the result, so a whole frame through
/// `Context::run_ui` is all a run of words takes to read.
fn words_inside(view: &mut View, panel: &mut Panel, rect: egui::Rect) -> Vec<String> {
    let ctx = drawn_once();
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| view.draw(ui, panel));
    out.textures_delta.clear();
    out.shapes
        .into_iter()
        .filter(|clipped| {
            let bounds = clipped.shape.visual_bounding_rect();
            bounds.is_finite() && rect.contains_rect(bounds)
        })
        .filter_map(|clipped| match clipped.shape {
            egui::Shape::Text(text) => Some(text.galley.job.text.clone()),
            _ => None,
        })
        .collect()
}

/// **The head says what the letters are for, and shows them with the caret
/// after them.** `showing deck A · drift_night` becomes
/// `keep as deck A · glass▏`: the label is the only thing in the row that can
/// say what a run of letters is *for*, and the deck stays because the half of
/// the run being replaced is exactly the half a name is.
#[test]
fn the_head_reads_keep_as_while_it_is_asking() {
    let mut panel = Panel::new(PLAUSIBLE.w, PLAUSIBLE.h);
    panel.solve();
    let mut view = view_of([showing(0), showing(1)]);
    let head = inspector(panel.layout(), 0, &view.inspector[0])
        .expect("a pane with room in it")
        .head;

    let reading = words_inside(&mut view, &mut panel, head);
    assert!(
        reading.iter().any(|w| w == "showing"),
        "a head that is reading does not say `showing`: {reading:?}"
    );
    assert!(
        reading.iter().any(|w| w == "deck A · drift_night"),
        "a head that is reading does not say what it is showing: {reading:?}"
    );

    view.name_set(0);
    for c in "glass".chars() {
        view.type_into_name(c);
    }
    let asking = words_inside(&mut view, &mut panel, head);
    assert!(
        asking.iter().any(|w| w == "keep as"),
        "a head that is asking still says `showing`: {asking:?}"
    );
    assert!(
        asking.iter().any(|w| w == "deck A · glass\u{258f}"),
        "a head that is asking does not draw what was typed with the caret after it: {asking:?}"
    );
    assert!(
        !asking.iter().any(|w| w == "deck A · drift_night"),
        "the head is drawing the material and the name being typed at once: {asking:?}"
    );

    // **And the other head is untouched**, which is what one field at a time
    // looks like on screen.
    let other = inspector(panel.layout(), 1, &view.inspector[1])
        .expect("a pane with room in it")
        .head;
    let untouched = words_inside(&mut view, &mut panel, other);
    assert!(
        untouched.iter().any(|w| w == "showing"),
        "the second head went into a naming state nobody pressed it for: {untouched:?}"
    );
}
