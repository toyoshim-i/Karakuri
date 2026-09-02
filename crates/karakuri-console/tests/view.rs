//! What the console draws, and who gets a pointer event.
//!
//! None of this needs a window or a device: what to draw is a walk of the
//! arrangement, and who gets an event is a hit test. The one test here that
//! does need a device is not here at all — it is `crates/karakuri/src/main.rs`'s
//! `gpu::egui_paints_the_console_onto_a_device`, under `mod gpu` like every
//! other one in the workspace.

mod common;

use common::{arranged, drawn_once, id_of, near, rect_of, showing, solved, PLAUSIBLE, SMALLEST};
use karakuri_console::input::{claim, Claim};
use karakuri_console::panel::Panel;
use karakuri_console::room::{Palette, Room};
use karakuri_console::view::{
    caption_of, picture_rect, plan_into, preview_rects, region, Kind, Placed, DECKS, REGIONS,
};
use karakuri_layout::{NodeId, Point};

/// The seven bays, read off the mock's `.bay-head`s — `console.html` heads
/// exactly these and nothing else.
const BAYS: &[&str] = &[
    "library",
    "staging",
    "program",
    "inspector",
    "mixer",
    "master",
    "sequencer",
];

/// **The canvas the picture is fitted to**, and it is the workspace's
/// reference workload — 1280x720, which `crates/karakuri/src/main.rs` names `CANVAS` and
/// builds its `Present` at. It is a value the caller hands in rather than
/// anything `src/` knows (ADR-0156), so a test hands one in too.
const CANVAS: (u32, u32) = (1280, 720);

/// **A canvas that is not the mock's shape**, so that a picture fitted to a
/// hard-coded 16:9 and one fitted to *the canvas* can be told apart. 4:3 is
/// the obvious other shape a performance runs at, and `--canvas` takes any
/// pair of numbers.
const SQUARISH: (u32, u32) = (1024, 768);

/// The transport and the outputs, which carry `class="bay"` for the card
/// styling and no `.bay-head` at all (ADR-0159). Two kinds and one rule: the
/// outputs row is a [`Kind::Outputs`] because it has a control in it, and it
/// is as headless as the transport.
const ROWS: &[&str] = &["transport", "outputs"];

fn planned(panel: &mut Panel) -> Vec<Placed> {
    let mut out = Vec::new();
    plan_into(panel, CANVAS, &mut out);
    out
}

// ---------------------------------------------------------------------------
// What is drawn
// ---------------------------------------------------------------------------

/// **Every leaf of the arrangement is asked to be drawn, and none is skipped.**
///
/// The failure this exists for is silent: a region left out of the table
/// leaves a hole in the panel with nothing anywhere saying so, and the hole is
/// the ground showing through, which is what a divider looks like. So this
/// asks the arrangement rather than the table — every visible leaf has to be
/// in the plan — and then asks the other way, so a region invented in the
/// table that the arrangement does not have is caught too.
#[test]
fn every_leaf_of_the_arrangement_is_drawn_and_none_is_skipped() {
    // **The narrowest console the mock draws**, where the Program bay's body
    // is the mock's own arrangement and every one of the thirteen regions is
    // in the plan. `PLAUSIBLE` is the other case and it is below, because
    // there the deck previews are beside the picture and the row they used to
    // be in is set aside — a region out of the plan for a reason that is not
    // the operator's.
    let mut panel = Panel::new(SMALLEST.w, SMALLEST.h);
    let placed = planned(&mut panel);
    let layout = panel.layout();

    for node in panel.nodes() {
        if layout.is_view(node.id) && layout.visible(node.id) {
            assert!(
                placed.iter().any(|p| p.id == node.id),
                "{:?} is a leaf of the arrangement and nothing draws it",
                layout.name(node.id)
            );
        }
    }

    // Two bays are splits: their heads sit above the regions that tile them,
    // so each is drawn and its parts are drawn inside it. The inspector is one
    // and the Program bay is the other — the picture and the deck previews
    // fold apart, so the bay around them is a split.
    let split_bays: Vec<NodeId> = ["inspector", "program"]
        .iter()
        .map(|name| id_of(layout, name))
        .collect();
    for (name, id) in ["inspector", "program"].iter().zip(&split_bays) {
        assert!(
            placed.iter().any(|p| p.id == *id),
            "the {name} bay's own head is not drawn"
        );
    }

    // And nothing else. `left-pane`, `centre` and `right-pane` are splits an
    // operator folds, not things with a face.
    for p in &placed {
        assert!(
            layout.is_view(p.id) || split_bays.contains(&p.id),
            "{:?} is a split and is being drawn as a region",
            layout.name(p.id)
        );
        assert_eq!(
            layout.name(p.id),
            Some(p.region.name),
            "a placed region carries a name the arrangement does not agree with"
        );
    }

    // Twelve leaves and the inspector, which is every row of the table.
    assert_eq!(placed.len(), REGIONS.len());

    // Tree order, which is what puts a split bay's card under the regions that
    // tile it — the inspector's panes, and the Program bay's picture and
    // preview row.
    let at = |name: &str| placed.iter().position(|p| p.region.name == name).unwrap();
    assert!(at("inspector") < at("inspector-1"));
    assert!(at("inspector-1") < at("inspector-2"));
    assert!(at("program") < at("program-view"));
    assert!(at("program-view") < at("deck-previews"));

    // **And at a window past the crossover, one region is missing on
    // purpose.** The four cells went beside the picture, so `deck-previews` is
    // set aside and is not a rectangle to draw — the cells are drawn from
    // `View::draw`'s one site off `program_bay`, and the leaf loop above is
    // satisfied because a region that is not `visible` is not asked for.
    // `tests/rearrange.rs` is where that arrangement is held; what this says
    // is that the *table* is still complete, one row shorter.
    let mut wide = Panel::new(PLAUSIBLE.w, PLAUSIBLE.h);
    let placed = planned(&mut wide);
    let layout = wide.layout();
    for node in wide.nodes() {
        if layout.is_view(node.id) && layout.visible(node.id) {
            assert!(
                placed.iter().any(|p| p.id == node.id),
                "{:?} is a leaf of the arrangement and nothing draws it",
                layout.name(node.id)
            );
        }
    }
    let row = id_of(layout, "deck-previews");
    assert!(
        layout.is_set_aside(row) && !layout.is_collapsed(row),
        "the row is not set aside at a 1920 window, so the cells are still under the picture"
    );
    assert_eq!(placed.len(), REGIONS.len() - 1);
    assert!(
        !placed.iter().any(|p| p.region.name == "deck-previews"),
        "the row is set aside and is still being asked for as a rectangle to draw"
    );
}

/// **A bay gets a head and a row does not.**
///
/// Seven bays, two rows and two panes, and the title of each bay is the mock's
/// own word for it — so a bay renamed in the manual and not here shows the old
/// word on the face of the panel, which is exactly what ADR-0159 is about.
#[test]
fn a_bay_gets_a_head_and_a_row_does_not() {
    for name in BAYS {
        match region(name)
            .unwrap_or_else(|| panic!("no region named {name}"))
            .kind
        {
            Kind::Bay { title, .. } => assert_eq!(
                title.to_lowercase(),
                *name,
                "the bay head says {title} and the arrangement calls it {name}"
            ),
            // The mixer is a bay and takes the same head, and it is a kind of
            // its own because `View::draw` has to know which bay the strips
            // go in — the title is `view::MIXER_TITLE` and is asserted
            // against the arrangement's name in `tests/mixer.rs`, where the
            // rest of that bay is.
            Kind::Mixer => assert_eq!(*name, "mixer"),
            // And the library is the second bay that is a kind of its own,
            // for the same reason: `View::draw` has to know which bay the
            // listing goes in. Its title is `view::LIBRARY_TITLE` and the rest
            // of that bay is asserted in `tests/library.rs`.
            Kind::Library => assert_eq!(*name, "library"),
            // And the master is the third, for the same reason again:
            // `View::draw` has to know which bay the out row goes in. Its
            // title is `view::MASTER_TITLE` and the rest of that bay is
            // asserted in `tests/master.rs`.
            Kind::Master => assert_eq!(*name, "master"),
            // And the staging lane is the fourth, since it got rows: the
            // candidate rows go in one bay and `View::draw` has to be told
            // which by the table. Its title is `view::STAGING_TITLE` and the
            // rest of that bay is asserted in `tests/staging.rs`.
            Kind::Staging => assert_eq!(*name, "staging"),
            other => panic!("{name} is a bay in the mock and a {other:?} here"),
        }
    }
    for name in ROWS {
        assert!(
            matches!(region(name).unwrap().kind, Kind::Transport | Kind::Outputs),
            "{name} has no heading in the mock and is being given one"
        );
    }
    // And the difference between the two rows is what is *in* each of them,
    // not a head on either: one draws four readouts and the other draws the
    // console's one control, and `View::draw` has to be told which is which by
    // the table rather than by comparing a name on the frame path.
    assert_eq!(
        region("transport").unwrap().kind,
        Kind::Transport,
        "the transport row is where the tempo, the beat and the frame readout are drawn"
    );
    assert_eq!(
        region("outputs").unwrap().kind,
        Kind::Outputs,
        "the Outputs row is where the one sink is drawn"
    );
    for name in ["inspector-1", "inspector-2"] {
        assert_eq!(
            region(name).unwrap().kind,
            Kind::Pane,
            "{name} sits inside a bay and has no head of its own"
        );
    }
    // The preview row sits inside a bay and has no head either, and it is a
    // kind of its own for `Kind::Picture`'s reason: `View::draw` has to know
    // which pane the cells go in.
    assert_eq!(
        region("deck-previews").unwrap().kind,
        Kind::Previews,
        "the row under the picture is where the four deck cells are drawn"
    );
    // *"The picture carries no label of its own"* — so it is not a bay, and it
    // is the one region that draws a texture.
    assert_eq!(
        region("program-view").unwrap().kind,
        Kind::Picture,
        "the picture in the Program bay is what a texture is drawn into"
    );
    assert_eq!(
        REGIONS.iter().filter(|r| r.kind == Kind::Picture).count(),
        1,
        "a second picture region: `View::draw` has one texture to give"
    );
    assert_eq!(
        REGIONS
            .iter()
            .filter(|r| {
                matches!(
                    r.kind,
                    Kind::Bay { .. } | Kind::Mixer | Kind::Library | Kind::Master | Kind::Staging
                )
            })
            .count(),
        BAYS.len(),
        "the bay head has seven call sites, which is the whole of why it is a component"
    );
    // And four of the seven are the bays with something in their bodies.
    assert_eq!(
        REGIONS.iter().filter(|r| r.kind == Kind::Mixer).count(),
        1,
        "a second mixer region: `View::draw` has one list of strips to give"
    );
    assert_eq!(
        REGIONS.iter().filter(|r| r.kind == Kind::Library).count(),
        1,
        "a second library region: `View::draw` has one listing to give"
    );
    assert_eq!(
        REGIONS.iter().filter(|r| r.kind == Kind::Master).count(),
        1,
        "a second master region: `View::draw` has one out level to give"
    );
    assert_eq!(
        REGIONS.iter().filter(|r| r.kind == Kind::Staging).count(),
        1,
        "a second staging region: `View::draw` has one lane of candidates to give"
    );
}

/// **The picture's rectangle comes off its own region, and at the width the
/// mock draws it is the mock's own picture.**
///
/// This is the rectangle a caller sizes a texture from, so getting it from
/// anything but `program-view` is a texture the wrong size — and the wrong
/// size in a way nothing on screen shows, because the picture fills whatever
/// rectangle it is given either way. At `SMALLEST` the bay's own derivation
/// says exactly what it should be: the centre track is 484, `.program-body`'s
/// 9px padding leaves 466, and 466 at 16:9 is 262. Those are the two numbers
/// the arrangement's 378 was built from, arrived at from the other end.
///
/// **The region and the canvas agree here to a quarter of a pixel and not
/// exactly**, which is the whole reason `picture_rect` rounds: `.program-view`
/// at 466 wide is 262.125 tall and the arrangement transcribed 262, so the box
/// is 1.778626 where the canvas is 1.777778. A strict fit would hand back
/// 465.7778 and a caller's `physical` would round it back to a 466-texel
/// texture — the same texture, drawn softened into a box a quarter of a pixel
/// narrower than itself.
#[test]
fn the_pictures_rectangle_is_its_region_less_the_head_and_the_padding() {
    let layout = solved(SMALLEST);
    let rect = picture_rect(&layout, CANVAS).expect("the picture is on screen");

    assert!(near(rect.width(), 466.0), "{} wide", rect.width());
    assert!(near(rect.height(), 262.0), "{} tall", rect.height());
    // **The tolerance is 0.01 and every other assertion in this file uses
    // `near` at 1e-3, and that is not a slack anybody forgot to tighten**: 466
    // x 262 is 1.778626 and 16:9 is 1.777778, so this is the one comparison in
    // the file that cannot be exact. It is the mock's rounding, and the
    // paragraph above is where it comes from.
    assert!(
        (rect.width() / rect.height() - 16.0 / 9.0).abs() < 0.01,
        "the mock's own picture is 16:9 and this is {}:{}",
        rect.width(),
        rect.height()
    );

    // And it is that region's, term for term: the bay head is painted over the
    // top of it and `.program-body`'s padding is inside it. At this width the
    // fit takes nothing off — the box is the picture — so every edge is the
    // region's own inset and the leftover is zero.
    let region = rect_of(&layout, "program-view");
    assert!(near(rect.min.x, region.x + 9.0));
    assert!(near(rect.min.y, region.y + 27.0 + 9.0 + 2.0));
    assert!(near(rect.max.x, region.x + region.w - 9.0));
    assert!(near(rect.max.y, region.y + region.h - 2.0));
}

/// **The picture is the canvas's shape at every window, centred in whatever
/// the region has, and a whole number of pixels.**
///
/// The rule ADR-0170 took for a deck preview cell, applied where it was first
/// refused. Before it, the picture was the whole region and `Present::draw`
/// letterboxed into it — so at any window above the mock's narrowest a texture
/// was allocated at the region's full size and the bars inside it were
/// rendered and uploaded every frame. At 1920 wide that is 1396 x 262 where
/// 466 x 262 is the picture: **two texels in three are black nobody looks
/// at.**
///
/// The wide end and the tall end both matter and they fail differently. Wide
/// is the ordinary case and the region is wider than the canvas, so the height
/// is what limits and the leftover is ground either side. Tall only happens
/// when an operator drags the program's height past what the width can carry,
/// and then the width limits and the leftover is above and below — a case a
/// rule written for wide windows alone gets wrong in silence.
///
/// # The window is arranged first, and the region is a different region past
/// the crossover
///
/// Every assertion below is against the `program-view` region, and past a
/// 1588-wide window that region is the **whole bay**: the cells have gone down
/// the sides and the row is set aside, so the picture's region is the bay
/// itself less nothing. The claims still hold term for term — the picture is
/// still the canvas's shape, still whole pixels, still centred in what the
/// region leaves, still inside it — which is the point worth having: *the
/// picture never leaves the rectangle it is clipped to* is the one property
/// that does not care which arrangement won, and it is the property a
/// rearrangement half-applied would break.
#[test]
fn the_picture_is_the_canvass_shape_at_every_window() {
    for width in [990.0, 1010.0, 1280.0, 1920.0, 3440.0] {
        let panel = arranged(
            karakuri_layout::Rect {
                w: width,
                ..SMALLEST
            },
            CANVAS,
        );
        let layout = panel.layout();
        let rect = picture_rect(layout, CANVAS).expect("the picture is on screen");
        let region = rect_of(layout, "program-view");

        assert!(
            (rect.width() / rect.height() - 16.0 / 9.0).abs() < 0.01,
            "at {width} wide the picture is {}x{} and the canvas is 16:9",
            rect.width(),
            rect.height()
        );
        // **Whole pixels**, which is what makes the texture a blit rather than
        // a resample: `physical` rounds, so a fractional rectangle is a
        // texture of one size drawn into a box of another.
        assert!(
            near(rect.width(), rect.width().round()) && near(rect.height(), rect.height().round()),
            "at {width} wide the picture is {}x{}, which is not a whole number of pixels",
            rect.width(),
            rect.height()
        );
        // **Centred in the box the region leaves**, so the ground either side
        // is equal — the same claim ADR-0170 makes about a cell in its track.
        let before = rect.min.x - (region.x + 9.0);
        let after = (region.x + region.w - 9.0) - rect.max.x;
        assert!(
            near(before, after),
            "at {width} wide the picture has {before} before it and {after} after it"
        );
        assert!(
            before >= -1e-3,
            "at {width} wide the picture starts {before} inside the region's padding"
        );
        // Inside the region, always: the leftover is the console's ground and
        // never a picture hanging over the bay.
        assert!(
            rect.min.y >= region.y + 27.0 + 9.0 - 1e-3 && rect.max.y <= region.y + region.h + 1e-3,
            "at {width} wide the picture runs from {} to {} outside its region",
            rect.min.y,
            rect.max.y
        );
    }

    // **The width stops following the window**, which is the pass the change
    // removes: the region grows and the picture does not. Stated **within an
    // arrangement**, because ADR-0182 put a step between the two — below, the
    // picture stops at the mock's 466 and the leftover is ground; beside, it
    // stops at what the bay's *height* carries and the leftover is ground
    // again. Neither follows the window; there is one step between them and
    // `tests/rearrange.rs` is where it is held.
    let below = |w: f32| {
        let panel = arranged(karakuri_layout::Rect { w, ..SMALLEST }, CANVAS);
        picture_rect(panel.layout(), CANVAS).expect("on screen")
    };
    let narrow = below(1300.0);
    let mid = below(1450.0);
    assert!(
        near(mid.width(), narrow.width()) && near(mid.height(), narrow.height()),
        "a window 150 wider changed the picture below the crossover: {:?} against {:?}",
        mid.size(),
        narrow.size()
    );
    let wide = below(PLAUSIBLE.w);
    let widest = below(3440.0);
    assert!(
        near(widest.width(), wide.width()) && near(widest.height(), wide.height()),
        "a window 1520 wider changed the picture beside the cells: {:?} against {:?}",
        widest.size(),
        wide.size()
    );
    // And the region did grow, in both, so the assertions above are about the
    // rule and not about a window that never widened.
    let panel = arranged(PLAUSIBLE, CANVAS);
    let region = rect_of(panel.layout(), "program-view");
    assert!(region.w - wide.width() > 500.0);

    // **A height drag is what does still grow it**, which is the half of the
    // manual's "sized by height" that survives: the region is much wider than
    // the canvas at this window, so the height is what limits and the picture
    // follows it.
    // **A bay dragged this tall is below's country and the layout needs no
    // rearranging**: two columns of a body 835 tall are 1488 wide and the body
    // is 1396, so beside cannot be drawn at all and the bit stays false. That
    // is ADR-0182's *"a tall bay widens both columns until below wins again"*,
    // reached from the other end.
    let mut layout = solved(PLAUSIBLE);
    let centre = id_of(&layout, "centre");
    layout.set_divider(centre, 0, 4000.0);
    layout.solve();
    let dragged = picture_rect(&layout, CANVAS).expect("on screen");
    // **Against the same window before the drag**, and it is a ratio rather
    // than the 400 pixels this used to add: `wide` is the picture *beside* the
    // cells now, 592 x 333 rather than the mock's 466 x 262, so the margin it
    // was compared against was measured on a rectangle that is no longer the
    // one at this window. Doubling was the claim while the bay was 378; with
    // the preview captions in it the picture beside is 350 rather than 333, so
    // what a full-height drag buys is 642 against 350 — under twice, and the
    // ratio is written as what it measures rather than rounded up to a claim
    // the rectangle no longer supports.
    assert!(
        dragged.height() > wide.height() * 1.8,
        "dragging the program taller left the picture at {} tall against the {} it had \
         before the drag",
        dragged.height(),
        wide.height()
    );
    assert!(
        (dragged.width() / dragged.height() - 16.0 / 9.0).abs() < 0.01,
        "a picture dragged taller is {}x{}",
        dragged.width(),
        dragged.height()
    );

    // **And past a point the width is what answers instead** — a narrow window
    // dragged tall, where the leftover is above and below rather than either
    // side. A rule written for wide windows alone gets this one wrong in
    // silence, because on screen it is still a picture in a bay.
    let mut layout = solved(karakuri_layout::Rect {
        w: SMALLEST.w,
        h: 1400.0,
        ..SMALLEST
    });
    let centre = id_of(&layout, "centre");
    layout.set_divider(centre, 0, 4000.0);
    layout.solve();
    let tall = picture_rect(&layout, CANVAS).expect("on screen");
    let region = rect_of(&layout, "program-view");
    let box_h = region.h - 27.0 - 9.0;
    assert!(
        box_h > tall.height() + 100.0,
        "the region is {box_h} tall inside its head and the picture is {}, so nothing was \
         left over and this is not the width-limited case",
        tall.height()
    );
    assert!(
        (tall.width() / tall.height() - 16.0 / 9.0).abs() < 0.01,
        "a picture taller than its width can carry is {}x{}",
        tall.width(),
        tall.height()
    );
    // The width is the box's, so the picture is exactly the mock's again — and
    // it is centred in what is left, top and bottom.
    assert!(near(tall.width(), 466.0), "{} wide", tall.width());
    let above = tall.min.y - (region.y + 27.0 + 9.0);
    let below = (region.y + region.h) - tall.max.y;
    assert!(
        near(above, below),
        "the picture has {above} above it and {below} below it"
    );
    assert!(
        above > 100.0,
        "there is only {above} of leftover above the picture"
    );
}

/// **The shape is the canvas's and not a 16:9 written into this crate.**
///
/// The failure is silent and it lasts until somebody runs a performance at a
/// canvas the mock's designer never drew: a hard-coded 16:9 gives a picture of
/// the wrong shape, `Present::draw` letterboxes the real canvas inside it, and
/// what comes back is bars in a rectangle that was supposed to have none —
/// which is exactly the state this whole rule exists to leave behind, with
/// nothing on screen saying it came back.
///
/// `--canvas` takes any pair of numbers and reaches a replay through
/// `Record::Canvas`, so the shape is a value and not a constant.
#[test]
fn the_shape_is_the_canvass_and_not_a_sixteen_by_nine_in_this_crate() {
    let layout = solved(PLAUSIBLE);
    let wide = picture_rect(&layout, CANVAS).expect("on screen");
    let squarish = picture_rect(&layout, SQUARISH).expect("on screen");

    assert!(
        (squarish.width() / squarish.height() - 4.0 / 3.0).abs() < 0.01,
        "a 4:3 canvas got a {}x{} picture, so the shape came from somewhere other than \
         the canvas",
        squarish.width(),
        squarish.height()
    );
    // Same box, same height, and a narrower picture — the region is wider than
    // either, so the height is what limits both.
    assert!(near(squarish.height(), wide.height()));
    assert!(squarish.width() < wide.width());

    // And a canvas taller than it is wide is not a special case either.
    let portrait = picture_rect(&layout, (720, 1280)).expect("on screen");
    assert!(
        (portrait.width() / portrait.height() - 9.0 / 16.0).abs() < 0.01,
        "a portrait canvas got a {}x{} picture",
        portrait.width(),
        portrait.height()
    );
}

/// **No rectangle where the picture is folded away**, which is the manual's
/// *"there is no state where it is hidden and still costing a pass"*: a caller
/// that renders into this rectangle records no pass at all when there is none.
///
/// **What carries it is the size test and not a visibility test**, and that
/// was learnt from this test rather than assumed: written with both, deleting
/// the visibility check left it passing, because a folded region keeps its
/// rectangle and loses its extent. So the check went and this is what holds
/// the remaining line — including for a picture folded by its bay rather than
/// by itself, which a visibility test and a size test answer alike and which
/// is asserted here so that the equivalence is not left as a belief.
#[test]
fn a_folded_picture_has_no_rectangle() {
    let mut layout = solved(PLAUSIBLE);
    assert!(picture_rect(&layout, CANVAS).is_some());

    layout.collapse(id_of(&layout, "program-view"));
    layout.solve();
    assert_eq!(picture_rect(&layout, CANVAS), None);

    // The previews are still there, so this is the picture being folded and
    // not the bay.
    assert!(layout.visible(id_of(&layout, "deck-previews")));

    layout.expand(id_of(&layout, "program-view"));
    layout.solve();
    assert!(picture_rect(&layout, CANVAS).is_some());

    // The bay folded around it, which is `g` over the picture rather than `f`.
    let mut layout = solved(PLAUSIBLE);
    layout.collapse(id_of(&layout, "program"));
    layout.solve();
    assert_eq!(picture_rect(&layout, CANVAS), None);

    // And a solo on the previews, which leaves the picture out rather than
    // folding anything above it.
    let mut layout = solved(PLAUSIBLE);
    layout.solo(id_of(&layout, "deck-previews"));
    layout.solve();
    assert_eq!(picture_rect(&layout, CANVAS), None);
}

/// **The four preview cells are the mock's own, at the width the mock draws
/// them.**
///
/// Every figure here is `lib.rs`'s Program bay derivation read from the other
/// end. At `SMALLEST` the centre track is 484, `.program-body`'s 9px padding
/// either side leaves 466, and 466 less three 6px gaps over four tracks is
/// **112** — which at 16:9 is **63**, the image's height. A cell is that image
/// and the caption band under it, `.cell`'s 4 and `.caption`'s 13, so the row
/// is 80 and the arrangement gave `deck-previews` that plus its 9px of padding
/// underneath. The sum the bay was built from and the rectangles it solves to
/// are the same numbers or the bay is wrong.
///
/// The insets are three of the four on purpose: nothing at the top, because
/// the 9 above the cells in the CSS is the split's 8px divider plus
/// `program-view`'s own bottom and belongs to neither this region nor the
/// picture.
#[test]
fn the_preview_cells_are_the_mocks_at_the_width_the_mock_draws() {
    let layout = solved(SMALLEST);
    let cells = preview_rects(&layout, CANVAS).expect("the previews are on screen");
    let region = rect_of(&layout, "deck-previews");

    for (deck, cell) in cells.iter().enumerate() {
        assert!(
            near(cell.width(), 112.0),
            "cell {deck} is {} wide",
            cell.width()
        );
        assert!(
            near(cell.height(), 63.0),
            "cell {deck} is {} tall",
            cell.height()
        );
        // Nothing above the row, and `.program-body`'s padding under it.
        assert!(
            near(cell.min.y, region.y),
            "cell {deck} starts at {} and the region at {}",
            cell.min.y,
            region.y
        );
        // The **image** ends where the caption band starts, and the caption
        // ends where `.program-body`'s padding does — so the row fills the
        // region less that pad, and the image is one term of the row.
        assert!(
            near(cell.max.y, region.y + region.h - 9.0 - 17.0),
            "cell {deck} ends at {} and the caption band starts at {}",
            cell.max.y,
            region.y + region.h - 9.0 - 17.0
        );
        assert!(
            near(caption_of(*cell).max.y, region.y + region.h - 9.0),
            "cell {deck}'s caption ends at {} and the region's padding leaves {}",
            caption_of(*cell).max.y,
            region.y + region.h - 9.0
        );
    }

    // `.previews`'s `gap: 6px`, between the tracks and nowhere else.
    for deck in 1..DECKS {
        let gap = cells[deck].min.x - cells[deck - 1].max.x;
        assert!(
            near(gap, 6.0),
            "the gap before cell {deck} is {gap} and not 6"
        );
    }

    // The row is the region less a pad either side, so the first cell's left
    // edge and the last cell's right edge are that pad in from the region.
    assert!(
        near(cells[0].min.x, region.x + 9.0),
        "the row starts at {} and the region's padding leaves {}",
        cells[0].min.x,
        region.x + 9.0
    );
    assert!(
        near(cells[DECKS - 1].max.x, region.x + region.w - 9.0),
        "the row ends at {} and the region's padding leaves {}",
        cells[DECKS - 1].max.x,
        region.x + region.w - 9.0
    );
}

/// **The cells never overlap, never leave the region, and never stop being
/// 16:9** — at any width, and especially at one much wider than the mock's.
///
/// This is the test the gap arithmetic is checked by, and the wrong version it
/// exists for is a plausible one: divide the row by four and take a gap off
/// each cell, which loses a gap's worth of width and leaves the last cell
/// short of the region's padding — four tracks have **three** gaps between
/// them and not four.
///
/// The wide end is the other half. `deck-previews` is pinned at 72 tall, so a
/// wider window widens the track and not the row, and a cell that filled its
/// track would stop being 16:9 the moment the window left `SMALLEST`. It is
/// 16:9 and centred instead, which is what these assertions say.
///
/// # Why the widths stop at 1500, and it is not a threshold nudged to pass
///
/// **This is the row, and past a 1588-wide window there is no row**: the cells
/// go down the sides of the picture and `deck-previews` is set aside, so every
/// assertion here about the region they tile is an assertion about a rectangle
/// with nothing in it. The four widths are the four of the old five that are
/// below the crossover, and the fifth is asserted at the bottom to be past it
/// — so a crossover that moved fails here rather than quietly leaving this
/// test asserting the row's arithmetic at one width. The cells' arrangement
/// beside the picture is `tests/rearrange.rs`, and the arithmetic of both is
/// `tests/program_body.rs`.
///
/// A body 706 wide is where the flip is, and the body is the window less 778
/// — the two side tracks, the four dividers and `.program-body`'s padding — so
/// the last window with a row in it is **1483** (ADR-0239).
#[test]
fn the_preview_cells_tile_their_region_and_stay_sixteen_by_nine() {
    for width in [1280.0, 1350.0, 1400.0, 1450.0] {
        let panel = arranged(
            karakuri_layout::Rect {
                w: width,
                ..SMALLEST
            },
            CANVAS,
        );
        let layout = panel.layout();
        let cells = preview_rects(layout, CANVAS).expect("the previews are on screen");
        let region = rect_of(layout, "deck-previews");
        assert!(
            !layout.is_set_aside(id_of(layout, "deck-previews")),
            "at {width} wide the row is set aside, so this is not the arrangement being \
             asserted below"
        );
        let row_left = region.x + 9.0;
        let row_right = region.x + region.w - 9.0;

        for (deck, cell) in cells.iter().enumerate() {
            assert!(
                (cell.width() / cell.height() - 16.0 / 9.0).abs() < 0.01,
                "at {width} wide, cell {deck} is {}x{} and a preview is 16:9",
                cell.width(),
                cell.height()
            );
            assert!(
                cell.min.x >= row_left - 1e-3 && cell.max.x <= row_right + 1e-3,
                "at {width} wide, cell {deck} runs from {} to {} outside the row's {row_left}..{row_right}",
                cell.min.x,
                cell.max.x
            );
            assert!(
                cell.min.y >= region.y - 1e-3 && cell.max.y <= region.y + region.h - 9.0 + 1e-3,
                "at {width} wide, cell {deck} runs from {} to {} outside the region",
                cell.min.y,
                cell.max.y
            );
        }
        for deck in 1..DECKS {
            assert!(
                cells[deck].min.x >= cells[deck - 1].max.x - 1e-3,
                "at {width} wide, cell {deck} starts at {} and cell {} ends at {}",
                cells[deck].min.x,
                deck - 1,
                cells[deck - 1].max.x
            );
        }

        // Centred in its track: the ground either side of a cell is equal, and
        // it is the same for every cell.
        let track = (row_right - row_left - 6.0 * (DECKS - 1) as f32) / DECKS as f32;
        for (deck, cell) in cells.iter().enumerate() {
            let track_x = row_left + (track + 6.0) * deck as f32;
            let before = cell.min.x - track_x;
            let after = track_x + track - cell.max.x;
            assert!(
                near(before, after),
                "at {width} wide, cell {deck} has {before} before it and {after} after it in its track"
            );
        }

        // The row is pinned at 63 tall by the arrangement, so a wider window
        // buys width and no height at all — which is the whole reason a cell
        // cannot both fill its track and stay 16:9.
        assert!(
            near(cells[0].height(), 63.0),
            "at {width} wide the row is {} tall",
            cells[0].height()
        );
    }

    // **1483 is the last window with a row in it and 1484 is the first
    // without**, which is what makes the four widths above the four that are
    // in this test's country rather than four that happen to pass (ADR-0239).
    let row_at = |w: f32| {
        let panel = arranged(karakuri_layout::Rect { w, ..SMALLEST }, CANVAS);
        panel
            .layout()
            .is_set_aside(id_of(panel.layout(), "deck-previews"))
    };
    assert!(
        !row_at(1483.0),
        "the row went beside the picture before 1484"
    );
    assert!(
        row_at(1484.0),
        "the cells are still in the row at 1484 wide"
    );
}
/// rule stated once more on the other half of the bay: a caller that renders
/// four auditions into these rectangles records no pass at all when there are
/// none, and *"a priming deck draws only while something auditions it"*.
///
/// folds the row, and `g` over the bay folds the picture with it.
#[test]
fn a_folded_preview_row_has_no_rectangles() {
    let mut layout = solved(PLAUSIBLE);
    assert!(preview_rects(&layout, CANVAS).is_some());

    layout.collapse(id_of(&layout, "deck-previews"));
    layout.solve();
    assert_eq!(preview_rects(&layout, CANVAS), None);

    // The picture is still there, so this is the row being folded and not the
    // bay — *"The deck previews under it are auditions of their own, so they
    // stay when it goes"*, read the other way round.
    assert!(picture_rect(&layout, CANVAS).is_some());

    layout.expand(id_of(&layout, "deck-previews"));
    layout.solve();
    assert!(preview_rects(&layout, CANVAS).is_some());

    // **And the fold the other way round, which is the sentence the program's
    // readout now prints**: *"fold the picture away (f over it) and deck A
    // keeps the loop awake on its own"*. The manual's own words are the same
    // claim — *"The deck previews under it are auditions of their own, so they
    // stay when it goes"* — and a readout that says a thing the arrangement
    // does not do is how this project has been wrong twice about what folding
    // the picture costs.
    let mut layout = solved(PLAUSIBLE);
    layout.collapse(id_of(&layout, "program-view"));
    layout.solve();
    assert_eq!(picture_rect(&layout, CANVAS), None);
    assert!(
        preview_rects(&layout, CANVAS).is_some(),
        "the picture is folded and the previews went with it, so an audition \
         that should still be running has nowhere to go"
    );

    // The bay folded around it.
    let mut layout = solved(PLAUSIBLE);
    layout.collapse(id_of(&layout, "program"));
    layout.solve();
    assert_eq!(preview_rects(&layout, CANVAS), None);

    // And a solo on the picture, which leaves the row out rather than folding
    // it.
    let mut layout = solved(PLAUSIBLE);
    layout.solo(id_of(&layout, "program-view"));
    layout.solve();
    assert_eq!(preview_rects(&layout, CANVAS), None);
}

// ---------------------------------------------------------------------------
// The palette
// ---------------------------------------------------------------------------

/// Every colour in a palette, so a transcription that left one behind is one
/// comparison rather than fourteen.
fn colours(p: &Palette) -> Vec<(&'static str, egui::Color32)> {
    vec![
        ("ground", p.ground),
        ("panel", p.panel),
        ("well", p.well),
        ("line", p.line),
        ("hair", p.hair),
        ("text", p.text),
        ("dim", p.dim),
        ("faint", p.faint),
        ("mint", p.mint),
        ("pink", p.pink),
        ("lav", p.lav),
        ("sun", p.sun),
        ("tint", p.tint),
        ("glow", p.glow),
        ("glow_pink", p.glow_pink),
    ]
}

/// Perceived lightness, near enough to tell a white room from a black one.
fn luma(c: egui::Color32) -> f32 {
    (0.2126 * c.r() as f32 + 0.7152 * c.g() as f32 + 0.0722 * c.b() as f32) / 255.0
}

/// **The palette answers for both rooms**, and answers differently in each.
///
/// The failure worth catching is a colour transcribed once and left: the two
/// blocks in `style.css` are fifteen near-identical lines each, and one line
/// copied from the wrong block is invisible until somebody switches rooms in a
/// dark hall.
#[test]
fn the_palette_answers_for_both_rooms() {
    let day = Room::Day.palette();
    let night = Room::Night.palette();

    for ((name, a), (_, b)) in colours(&day).into_iter().zip(colours(&night)) {
        assert_ne!(
            a, b,
            "--c-{name} is the same colour in both rooms, so one of them was not transcribed"
        );
    }

    // "Pastel on white by day and lit on black in a dark hall."
    assert!(luma(day.ground) > 0.8, "the day ground is not white");
    assert!(luma(day.text) < 0.3, "the day ink is not dark");
    assert!(luma(night.ground) < 0.1, "the night ground is not black");
    assert!(luma(night.text) > 0.8, "the night ink is not light");

    // A bay is a card standing off the ground, in both rooms. This is what
    // makes an empty bay visible at all, which is the whole of this pass.
    assert!(luma(day.panel) > luma(day.ground));
    assert!(luma(night.panel) > luma(night.ground));

    // The room key toggles, and toggling twice is where it started.
    assert_eq!(Room::Day.other(), Room::Night);
    assert_eq!(Room::Night.other().other(), Room::Night);
    assert_eq!(Room::Day.palette(), Palette::DAY);
    assert_eq!(Room::Night.palette(), Palette::NIGHT);
}

// ---------------------------------------------------------------------------
// The input rule
// ---------------------------------------------------------------------------

/// The middle of the first boundary the arrangement has.
fn on_a_boundary(panel: &mut Panel) -> Point {
    panel.solve();
    let layout = panel.layout();
    let (split, index) = layout.boundaries().next().expect("no boundary at all");
    let gap = layout.boundary(split, index).expect("no pair");
    Point::new(gap.x + gap.w * 0.5, gap.y + gap.h * 0.5)
}

/// The `solo` pill in the Program bay's head, near the right end of it.
/// **The place a boundary and a control are closest**, and so the place the
/// rule is worth stating.
///
/// **It was *the one control the panel draws* and it is a control that acts
/// now**, which is what these two tests had to be re-read against: a press
/// here used to be `egui`'s because nothing on the panel answered it, and it
/// is the panel's under rule 4 because `view::program_head` answers it. The
/// point is kept exactly where it was — 24 in from the right of the bay and 14
/// down, which is inside the capsule — because what it is here for is the
/// nearness to the boundary rather than the pill.
fn on_the_solo_pill(panel: &mut Panel) -> Point {
    panel.solve();
    let program = rect_of(panel.layout(), "program");
    Point::new(program.x + program.w - 24.0, program.y + 14.0)
}

/// **A pointer on a boundary reaches the panel, and `egui` does not see it.**
#[test]
fn a_pointer_on_a_boundary_is_the_panels() {
    let mut panel = Panel::new(PLAUSIBLE.w, PLAUSIBLE.h);
    let p = on_a_boundary(&mut panel);
    assert_eq!(
        claim(&mut panel, &drawn_once(), &showing(&[]), p),
        Claim::Panel
    );
}

/// **A pointer off a boundary and off every control is `egui`'s** — and the
/// `solo` pill is the case that says which of the two rules answered.
///
/// The pill used to be in the first half of that sentence: it was the one
/// thing the panel drew that a hand would reach for, and a press on it was
/// `egui`'s because nothing here answered it. It is a control now, so it is
/// the panel's under rule 4 — **not** under rule 3, which is the distinction
/// this test is for, and the point is far enough from the boundary above the
/// bay that only rule 4 can be giving it away.
#[test]
fn a_pointer_off_a_boundary_is_eguis() {
    let ctx = drawn_once();
    let mut panel = Panel::new(PLAUSIBLE.w, PLAUSIBLE.h);
    panel.solve();

    let library = rect_of(panel.layout(), "library");
    let middle = Point::new(library.x + library.w * 0.5, library.y + library.h * 0.5);
    assert_eq!(claim(&mut panel, &ctx, &showing(&[]), middle), Claim::Egui);

    let pill = on_the_solo_pill(&mut panel);
    assert!(
        !matches!(
            panel.layout().hit(pill, karakuri_console::panel::GRAB),
            karakuri_layout::Hit::Divider { .. }
        ),
        "the point taken for the solo pill is on a boundary, so what claims it below says \
         nothing about the control"
    );
    assert_eq!(
        claim(&mut panel, &ctx, &showing(&[]), pill),
        Claim::Panel,
        "the solo pill is a control and no boundary grabs this point, so rule 4 is what \
         gives the press to the panel"
    );

    // Outside the window entirely: nothing to grab, so egui's.
    assert_eq!(
        claim(&mut panel, &ctx, &showing(&[]), Point::new(-40.0, -40.0)),
        Claim::Egui
    );
}

/// **A drag in hand keeps its claim, wherever the pointer wanders.**
///
/// This is the one that is a bug waiting to happen. A claim re-decided from
/// the pointer on every event hands the middle of a drag to `egui` the moment
/// the pointer leaves the six pixels either side of the boundary — which it
/// does immediately, because a drag is how a boundary gets anywhere — and then
/// two things think they are dragging. The release matters just as much: asked
/// after `released`, the claim sees no drag and hands `egui` a button-up it
/// never saw the button-down for.
///
/// **The `solo` pill is in the wander now rather than in the before and
/// after**, because it stopped being an elsewhere-point the day it became a
/// control: rule 1 has to beat rule 4 as well as rule 3, and a point that is
/// the panel's either way cannot say whether the drag kept its claim. What
/// carries that half is the library's middle, which is on no control and on no
/// boundary, and it goes `egui` -> panel -> `egui` across the gesture.
#[test]
fn a_drag_in_hand_keeps_its_claim_wherever_the_pointer_goes() {
    let ctx = drawn_once();
    let mut panel = Panel::new(PLAUSIBLE.w, PLAUSIBLE.h);
    let start = on_a_boundary(&mut panel);
    let pill = on_the_solo_pill(&mut panel);
    panel.solve();
    let library = rect_of(panel.layout(), "library");
    let middle = Point::new(library.x + library.w * 0.5, library.y + library.h * 0.5);

    // Before the press: the elsewhere-point is egui's, and the pill is the
    // panel's for rule 4 rather than for anything this test is about.
    assert_eq!(claim(&mut panel, &ctx, &showing(&[]), middle), Claim::Egui);
    assert_eq!(claim(&mut panel, &ctx, &showing(&[]), pill), Claim::Panel);

    panel.press(start);
    assert!(panel.dragging());

    for wandered in [pill, middle, Point::new(-500.0, 4000.0), start] {
        assert_eq!(
            claim(&mut panel, &ctx, &showing(&[]), wandered),
            Claim::Panel,
            "a boundary is in hand and the claim was given away at {wandered:?}"
        );
        panel.moved(wandered);
    }

    // The release is still the panel's, and it has to be asked before
    // `released` takes the drag out of hand.
    assert_eq!(claim(&mut panel, &ctx, &showing(&[]), middle), Claim::Panel);
    panel.released();
    assert!(!panel.dragging());

    // And afterwards the claim is back where it was.
    assert_eq!(claim(&mut panel, &ctx, &showing(&[]), middle), Claim::Egui);
    assert_eq!(claim(&mut panel, &ctx, &showing(&[]), pill), Claim::Panel);
    let boundary = on_a_boundary(&mut panel);
    assert_eq!(
        claim(&mut panel, &ctx, &showing(&[]), boundary),
        Claim::Panel
    );
}
