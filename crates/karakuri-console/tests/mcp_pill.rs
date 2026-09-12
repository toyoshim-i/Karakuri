//! The four class pills: what a model may reach, and the hand that opens it.
//!
//! `docs/manual/console.html` draws four of them and specifies each in its own
//! tooltip — one in the Program bay's head beside `solo`, one in the Mixer
//! bay's, one in the Master bay's, and one beside the `Outputs` label, because
//! that row has no head to put an indicator in. Each says what it opens, that a
//! click opens the class and a second click shuts it, and what about the press
//! is deliberately unsettled.
//!
//! Six things, and the third and the fourth are why this file exists rather
//! than a few more assertions in `solo_pill.rs` and `outputs.rs`:
//!
//! 1. That each of the four is drawn where the page puts it, and that the
//! region it is in is the bay a refusal names. 2. That each is hit-tested where
//! it is painted, and that nothing beside one is. 3. That a press opens exactly
//! one class and leaves the other three shut — the property, over all four,
//! because an opening that took its neighbours with it is a permission nobody
//! granted. 4. That a second press shuts it, which is the half of the page's
//! sentence a control could silently not have. 5. That the word says which
//! state the class is in, and that the capsule is measured for the word it
//! holds. 6. That a bay carrying no class draws nothing at all, and that a pill
//! nobody has drawn is not one a press can be on.
//!
//! What is not here is the press reaching a run's `Opening`, and the audit
//! answering differently afterwards. That crosses two crates this one cannot
//! see — `karakuri-environment` holds the handle and `karakuri-operation`'s
//! gate holds the audit — so it is `crates/karakuri/src/main.rs`'s
//! `the_gate_lets_a_refused_operation_through_once_the_class_is_open`, which is
//! the one place that can see both. This crate takes no handle at all
//! (ADR-0156): `McpPill::next` hands a value back and somebody else writes it.
//!
//! None of it needs a window or a device. It does need `egui`'s fonts, because
//! a `.pill` is as wide as the word in it — see `common::drawn_once`.

mod common;

use common::{drawn_once, id_of, near, rect_of, showing, solved, PLAUSIBLE, SMALLEST};
use karakuri_console::input::{claim, Claim};
use karakuri_console::panel::{Op, Panel, GRAB};
use karakuri_console::room::{size, Room};
use karakuri_console::view::{
    class_at, mcp_pill, mcp_word, outputs, program_head, region, View, REGIONS,
};
use karakuri_layout::{Layout, Point, Rect};
use karakuri_operation::gate::{Class, Open};

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

/// The four pills on one solved console, in `Class::ALL`'s order.
fn pills(ctx: &egui::Context, layout: &Layout, open: Open) -> Vec<karakuri_console::view::McpPill> {
    Class::ALL
        .iter()
        .map(|class| {
            mcp_pill(ctx, layout, *class, open).unwrap_or_else(|| panic!("{class:?} draws no pill"))
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Where each of the four is
// ---------------------------------------------------------------------------

/// The region a class opens is the bay its refusal names, in both directions.
///
/// `karakuri_operation::gate::Class::bay` is what a refused model is told —
/// *"the operator opens it at the head of the Mixer bay"* — and this console is
/// what has to draw a pill there. A refusal naming a place with no pill in it
/// is worse than one naming none, because the model repeats it to the person
/// sitting there and sends them looking.
#[test]
fn the_region_a_class_opens_is_the_bay_its_refusal_names() {
    for class in Class::ALL {
        let name = karakuri_console::view::opens(*class);
        assert_eq!(
            name,
            class.bay().to_lowercase(),
            "{class:?} opens at `{name}` and its refusal says {}",
            class.bay()
        );
        assert!(
            region(name).is_some(),
            "{class:?} opens at `{name}`, which is not a region this console draws"
        );
        assert_eq!(
            class_at(name),
            Some(*class),
            "`{name}` answers for no class"
        );
    }

    // And every other region carries none, which is the answer that commits to
    // neither half of ADR-0235's open question: the Library, the Inspector,
    // Staging and the Sequencer have no class, and *"an indicator that is
    // present everywhere reads as a state wherever it is absent"*.
    let carrying: Vec<&str> = REGIONS
        .iter()
        .map(|r| r.name)
        .filter(|name| class_at(name).is_some())
        .collect();
    assert_eq!(
        carrying,
        vec!["program", "mixer", "master", "outputs"],
        "some region other than the four ADR-0235 names is drawing a class pill"
    );
    assert_eq!(Class::ALL.len(), 4);
}

/// Each of the four is a `.pill` in the region the page puts it in, and every
/// number here is the mock's.
#[test]
fn each_class_draws_a_capsule_in_its_own_region() {
    for viewport in [SMALLEST, PLAUSIBLE] {
        let (panel, ctx) = console(viewport);
        for class in Class::ALL {
            let pill = mcp_pill(&ctx, panel.layout(), *class, Open::CLOSED)
                .unwrap_or_else(|| panic!("{class:?} draws no pill at {viewport:?}"));
            assert_eq!(pill.class, *class);
            assert!(!pill.open, "a console nobody has opened reads open");

            // `.pill` is 16.5 whatever it is in, which is the one number the
            // three bay heads and the headless row have in common.
            assert!(
                near(pill.pill.height(), size::PILL_H),
                "{class:?}'s capsule is {} tall and a `.pill` is {}",
                pill.pill.height(),
                size::PILL_H
            );

            // And it is inside the region whose name the refusal says.
            let region = rect_of(panel.layout(), karakuri_console::view::opens(*class));
            let region = egui::Rect::from_min_size(
                egui::pos2(region.x, region.y),
                egui::vec2(region.w, region.h),
            );
            assert!(
                region.contains_rect(pill.pill),
                "{class:?}'s capsule {:?} is not inside {region:?}",
                pill.pill
            );
        }

        // No two of them are the same rectangle, which four pills read out of
        // one derivation could quietly have been.
        let four = pills(&ctx, panel.layout(), Open::CLOSED);
        for (i, a) in four.iter().enumerate() {
            for b in four.iter().skip(i + 1) {
                assert!(
                    !a.pill.intersects(b.pill),
                    "{:?} and {:?} are drawn over each other",
                    a.class,
                    b.class
                );
            }
        }
    }
}

/// The Program bay's pill sits beside `solo` and to the right of it, which is
/// where the page draws it: `1920×1080`, `solo`, `mcp · shut`, `previews 2 of
/// 4`.
///
/// And `solo` moves when this one does. The two words are not the same width,
/// so a head laid out for one state and hit-tested against the other would put
/// a press on `solo` a few pixels off the capsule an operator sees. One
/// derivation answers for both — `view::head_capsule` — and this is the
/// assertion that says so.
#[test]
fn the_program_bays_pill_sits_beside_solo_and_moves_it() {
    let (panel, ctx) = console(PLAUSIBLE);
    let shut = program_head(&ctx, panel.layout(), Open::CLOSED).expect("the bay draws its pill");
    let pill = mcp_pill(&ctx, panel.layout(), Class::LiveDeck, Open::CLOSED).expect("a class pill");

    assert!(
        shut.solo.max.x <= pill.pill.min.x,
        "the class pill is left of `solo`: solo ends at {} and it starts at {}",
        shut.solo.max.x,
        pill.pill.min.x
    );
    assert!(near(shut.solo.center().y, pill.pill.center().y));
    assert!(
        near(pill.pill.min.x - shut.solo.max.x, size::PILL_GAP),
        "the two capsules are {} apart and `.bay-head`'s gap is {}",
        pill.pill.min.x - shut.solo.max.x,
        size::PILL_GAP
    );

    // Open, the word is longer and `solo` moves left with it.
    let open = Open::CLOSED.with(Class::LiveDeck, true);
    let opened = program_head(&ctx, panel.layout(), open).expect("the bay draws its pill");
    let wide = mcp_pill(&ctx, panel.layout(), Class::LiveDeck, open).expect("a class pill");
    assert_ne!(
        wide.pill.width(),
        pill.pill.width(),
        "`{}` and `{}` laid out to the same width, so this test cannot see the \
         difference it was written for",
        mcp_word(false),
        mcp_word(true)
    );
    assert!(
        near(wide.pill.max.x, pill.pill.max.x),
        "the right edge moved"
    );
    assert!(
        near(opened.solo.max.x, wide.pill.min.x - size::PILL_GAP),
        "`solo` did not move with the class pill beside it"
    );
}

/// The Outputs row's pill sits beside the word that stands in for a head, which
/// is the placement this console had no precedent for.
///
/// The page says it outright: *"This row has no bay head to put an indicator in
/// — it is headless, like the transport — so the pill sits beside the word that
/// stands in for one."* So it is laid out in `.outputs`'s own flex row, one gap
/// after the label and one before the sink, and it is a `.pill` rather than a
/// `.sink` — a control that looked like a sink here would read as a fifth
/// output.
#[test]
fn the_outputs_rows_pill_sits_beside_the_word_that_stands_in_for_a_head() {
    let (panel, ctx) = console(PLAUSIBLE);
    let row = outputs(&ctx, panel.layout(), Open::CLOSED).expect("the row draws its sink");
    let pill = mcp_pill(&ctx, panel.layout(), Class::InputsAndOutputs, Open::CLOSED)
        .expect("a class pill");

    // One derivation, two readers: the row lays it out and the class pill is
    // read back out of it, so the four openings are one type however
    // differently the two placements are arrived at.
    assert_eq!(pill.pill, row.mcp);

    assert!(near(row.mcp.min.x, row.label.max.x + size::OUTPUTS_GAP));
    assert!(near(row.sink.min.x, row.mcp.max.x + size::OUTPUTS_GAP));
    assert!(near(row.mcp.height(), size::PILL_H));
    assert!(
        !near(row.mcp.height(), size::SINK_H),
        "the class pill is a sink's height, and would read as a fifth output"
    );

    // Centred in the row like everything else in it, which is where its
    // clearance comes from.
    let region = rect_of(panel.layout(), "outputs");
    assert!(near(row.mcp.center().y, region.y + region.h * 0.5));
    let clearance = (region.h - size::PILL_H) * 0.5;
    assert!(
        clearance > GRAB,
        "the class pill has {clearance} of row above it and a boundary grabs {GRAB}"
    );
}

// ---------------------------------------------------------------------------
// It is painted, and it is pressed in the same rectangle
// ---------------------------------------------------------------------------

/// Every shape the console paints wholly inside `rect`, on one frame —
/// `library.rs`'s helper, and the reasoning is written out there.
fn shapes_inside(view: &mut View, panel: &mut Panel, rect: egui::Rect) -> Vec<egui::Shape> {
    let ctx = drawn_once();
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| view.draw(ui, panel));
    out.textures_delta.clear();
    out.shapes
        .into_iter()
        .filter(|clipped| {
            let bounds = clipped.shape.visual_bounding_rect();
            bounds.is_finite() && rect.contains_rect(bounds)
        })
        .map(|clipped| clipped.shape)
        .collect()
}

/// Each of the four is actually painted, in the capsule the hit test uses.
///
/// *Nothing is drawn there* is a claim about the paint pass and not about a
/// rectangle, so it is asserted by drawing a frame and counting what landed
/// inside each capsule — `library.rs`'s method, one control along. A pill
/// derived and never painted would pass every other test in this file and be a
/// control an operator cannot see.
#[test]
fn each_pill_is_painted_in_the_capsule_a_press_lands_on() {
    let mut panel = Panel::new(PLAUSIBLE.w, PLAUSIBLE.h);
    panel.solve();
    let ctx = drawn_once();
    let mut view = View::new(Room::Day);

    for class in Class::ALL {
        let shut = mcp_pill(&ctx, panel.layout(), *class, Open::CLOSED).expect("a class pill");
        assert!(
            !shapes_inside(&mut view, &mut panel, shut.pill).is_empty(),
            "{class:?}'s capsule is derived and nothing is painted in it"
        );
    }

    // And an opened class is painted differently from a shut one — the word
    // and the treatment both, which is `.pill.armed`'s own argument on the
    // audio-in pill: a capsule that was only lit would leave *which* class
    // unanswered, and one that only carried a name would make an open class
    // and a shut one look alike at the distance a panel is read from.
    //
    // **The two are counted over the union of the two capsules**, because they
    // are not the same width: a box measured for one of them would answer for
    // half the other, and the `.armed` treatment's own glow is wider than the
    // capsule and is excluded from both by containment.
    let opened = Open::CLOSED.with(Class::MixFaders, true);
    let shut = mcp_pill(&ctx, panel.layout(), Class::MixFaders, Open::CLOSED).expect("a pill");
    let open = mcp_pill(&ctx, panel.layout(), Class::MixFaders, opened).expect("a pill");
    assert!(open.open && !shut.open);
    let both = shut.pill.union(open.pill);
    view.opening = opened;
    let lit = shapes_inside(&mut view, &mut panel, both);
    view.opening = Open::CLOSED;
    let dark = shapes_inside(&mut view, &mut panel, both);
    assert_ne!(
        lit, dark,
        "an open class and a shut one paint the same shapes in the same capsule"
    );
}

/// A press on a pill is the panel's and a press beside one is `egui`'s — the
/// rule every control here lives by: a control claims what it acts on and no
/// more.
#[test]
fn each_pill_is_claimed_and_nothing_beside_it_is() {
    let (mut panel, ctx) = console(PLAUSIBLE);
    let four = pills(&ctx, panel.layout(), Open::CLOSED);

    for pill in &four {
        for probe in [
            pill.pill.center(),
            pill.pill.center_bottom(),
            pill.pill.left_bottom(),
            pill.pill.right_bottom(),
        ] {
            assert!(
                pill.hit(at(probe)),
                "{:?} does not claim {probe:?}",
                pill.class
            );
            assert_eq!(
                claim(&mut panel, &ctx, &showing(&[]), at(probe)),
                Claim::Panel,
                "{probe:?} is on {:?}'s capsule and went to egui",
                pill.class
            );
        }

        // Just left of the capsule, in the ground it is laid out from, and
        // just below it.
        let beside = egui::pos2(pill.pill.min.x - 6.0, pill.pill.center().y);
        let under = egui::pos2(pill.pill.center().x, pill.pill.max.y + 8.0);
        for off in [beside, under] {
            assert!(
                !pill.hit(at(off)),
                "{:?} claims {off:?}, which is off the capsule",
                pill.class
            );
        }
    }
}

// ---------------------------------------------------------------------------
// What a press asks for
// ---------------------------------------------------------------------------

/// A press opens exactly one class and leaves the other three shut.
///
/// The property over all four, because this is the assertion nothing else can
/// make for it: `Open`'s fields are private and `Open::with` names one class,
/// so the only way three classes could be opened by a press on the fourth is
/// here — in the value this control composes. `McpPill::next` takes the whole
/// opening and writes it back for exactly that reason.
#[test]
fn a_press_opens_exactly_one_class_and_leaves_the_other_three_shut() {
    let (panel, ctx) = console(PLAUSIBLE);

    for class in Class::ALL {
        let pill = mcp_pill(&ctx, panel.layout(), *class, Open::CLOSED).expect("a class pill");
        assert!(!pill.open);
        let after = pill.next(Open::CLOSED);
        assert!(after.holds(*class), "a press on {class:?} did not open it");
        for other in Class::ALL.iter().filter(|other| *other != class) {
            assert!(
                !after.holds(*other),
                "a press on {class:?} also opened {other:?}"
            );
        }
        // And the pill drawn under that opening says so, which is the state an
        // operator reads rather than the value the press composed.
        let lit = mcp_pill(&ctx, panel.layout(), *class, after).expect("a class pill");
        assert!(lit.open);
        assert_eq!(mcp_word(lit.open), "mcp · open");
    }
}

/// A second press shuts it, and puts the opening back exactly as it was — the
/// other half of the page's own sentence, *"Click to open the class; click
/// again to shut it."*
#[test]
fn a_second_press_shuts_it_and_leaves_the_rest_alone() {
    let (panel, ctx) = console(PLAUSIBLE);

    // From a console with two classes already open, so that the round trip is
    // asserted against something other than `CLOSED` — a `next` that answered
    // with a fresh `Open` would pass every assertion above and lose the two.
    let before = Open::CLOSED
        .with(Class::MasterEffects, true)
        .with(Class::InputsAndOutputs, true);

    for class in Class::ALL {
        let pill = mcp_pill(&ctx, panel.layout(), *class, before).expect("a class pill");
        let once = pill.next(before);
        assert_ne!(once.holds(*class), before.holds(*class));

        let again = mcp_pill(&ctx, panel.layout(), *class, once).expect("a class pill");
        assert_eq!(
            again.next(once),
            before,
            "two presses on {class:?} did not leave the opening as it was"
        );
    }
}

/// The word says which state the class is in, and the capsule is measured for
/// the word it holds.
///
/// The two are one assertion on purpose: a control whose word changed and whose
/// box did not would be one an operator could press on the half of it that is
/// not there.
#[test]
fn the_word_says_the_state_and_the_capsule_is_measured_for_it() {
    assert_eq!(mcp_word(false), "mcp · shut");
    assert_eq!(mcp_word(true), "mcp · open");

    let (panel, ctx) = console(PLAUSIBLE);
    for class in Class::ALL {
        let shut = mcp_pill(&ctx, panel.layout(), *class, Open::CLOSED).expect("a pill");
        let open = mcp_pill(
            &ctx,
            panel.layout(),
            *class,
            Open::CLOSED.with(*class, true),
        )
        .expect("a pill");
        assert!(!shut.open && open.open);
        assert_ne!(
            shut.pill.width(),
            open.pill.width(),
            "{class:?}'s capsule is the same width in both states, so one of the two \
             words is being drawn in a box measured for the other"
        );
    }
}

// ---------------------------------------------------------------------------
// Where there is no pill at all
// ---------------------------------------------------------------------------

/// A bay carrying no class draws nothing, which is the answer ADR-0235's open
/// question is left at rather than settled by this console.
#[test]
fn a_bay_with_no_class_draws_no_pill() {
    let (mut panel, ctx) = console(PLAUSIBLE);
    for name in ["library", "staging", "inspector", "sequencer"] {
        assert_eq!(class_at(name), None);
        // Nothing in those bays' heads is a control at all, which is the
        // stronger statement and the one a press can check: the whole head is
        // `egui`'s.
        let bay = rect_of(panel.layout(), name);
        let head = Point::new(bay.x + bay.w - 30.0, bay.y + size::HEAD_H * 0.5);
        assert_eq!(
            claim(&mut panel, &ctx, &showing(&[]), head),
            Claim::Egui,
            "something in the {name} bay's head is being claimed as a control"
        );
    }
}

/// A region that is folded away, or soloed away, has no pill to press.
///
/// `program_head`'s rule and `outputs`'s, stated once over all four: a
/// rectangle with nothing in it is not something to paint or to click.
#[test]
fn a_folded_or_soloed_region_draws_no_pill() {
    let ctx = drawn_once();
    for class in Class::ALL {
        let name = karakuri_console::view::opens(*class);
        let mut layout = solved(PLAUSIBLE);
        assert!(mcp_pill(&ctx, &layout, *class, Open::CLOSED).is_some());

        layout.collapse(id_of(&layout, name));
        layout.solve();
        assert_eq!(
            mcp_pill(&ctx, &layout, *class, Open::CLOSED),
            None,
            "the {name} region is folded and its class pill is still drawn"
        );

        layout.expand(id_of(&layout, name));
        layout.solve();
        assert!(mcp_pill(&ctx, &layout, *class, Open::CLOSED).is_some());

        // A solo somewhere else takes the whole region off the screen.
        layout.solo(id_of(&layout, "transport"));
        layout.solve();
        assert_eq!(
            mcp_pill(&ctx, &layout, *class, Open::CLOSED),
            None,
            "a solo left the {name} region invisible and its class pill drawn"
        );
    }
}

/// Before anything has been drawn there is no pill, which is not a special case
/// to be worked around: a capsule is as wide as the word in it, the word has
/// not been laid out, and a press cannot be on something that has never been on
/// screen.
#[test]
fn a_console_that_has_never_drawn_has_no_pill() {
    let mut panel = Panel::new(PLAUSIBLE.w, PLAUSIBLE.h);
    panel.solve();
    let fresh = egui::Context::default();
    assert_eq!(fresh.cumulative_pass_nr(), 0);
    for class in Class::ALL {
        assert_eq!(mcp_pill(&fresh, panel.layout(), *class, Open::CLOSED), None);
    }
}

/// Opening a class moves nothing on the console, which is the whole of *refused
/// rather than hidden* seen from the operator's side: the manual's note on the
/// Mixer's pill says *"Nothing here is ever refused to a hand"*, and a pill
/// that folded, greyed or disabled anything would be this surface taking
/// something away from the person who just granted it.
#[test]
fn opening_a_class_moves_nothing_in_the_arrangement() {
    let (mut panel, ctx) = console(PLAUSIBLE);
    let before = common::rects(panel.layout());
    for class in Class::ALL {
        let pill = mcp_pill(&ctx, panel.layout(), *class, Open::CLOSED).expect("a pill");
        let _ = pill.next(Open::CLOSED);
    }
    panel.solve();
    assert_eq!(
        before,
        common::rects(panel.layout()),
        "a class pill moved the arrangement"
    );
    // And it is not an `Op` at all, which is the type saying so: this control
    // has no `op()` to call, and `Op` is the console's own vocabulary.
    let _: fn(&karakuri_console::view::Outputs) -> Op = karakuri_console::view::Outputs::op;
}
