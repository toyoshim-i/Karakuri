use super::view_common::*;

// ---------------------------------------------------------------------------
// What is drawn
// ---------------------------------------------------------------------------

/// Every leaf of the arrangement is asked to be drawn, and none is skipped.
///
/// The failure this exists for is silent: a region left out of the table leaves
/// a hole in the panel with nothing anywhere saying so, and the hole is the
/// ground showing through, which is what a divider looks like. So this asks the
/// arrangement rather than the table — every visible leaf has to be in the plan
/// — and then asks the other way, so a region invented in the table that the
/// arrangement does not have is caught too.
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

/// A bay gets a head and a row does not.
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
            // And the sequencer is the fifth, since it got a body: the ruler,
            // the rows and the playhead go in one bay and `View::draw` has to
            // be told which by the table. Its title is `view::SEQUENCER_TITLE`
            // and the rest of that bay is asserted in `tests/sequencer.rs`.
            Kind::Sequencer => assert_eq!(*name, "sequencer"),
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
                    Kind::Bay { .. }
                        | Kind::Mixer
                        | Kind::Library
                        | Kind::Master
                        | Kind::Staging
                        | Kind::Sequencer
                )
            })
            .count(),
        BAYS.len(),
        "the bay head has seven call sites, which is the whole of why it is a component"
    );
    // And five of the seven are the bays with something in their bodies.
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
    assert_eq!(
        REGIONS.iter().filter(|r| r.kind == Kind::Sequencer).count(),
        1,
        "a second sequencer region: `View::draw` has one armed pattern to give"
    );
}

/// The picture's rectangle comes off its own region, and at the width the mock
/// draws it is the mock's own picture.
///
/// This is the rectangle a caller sizes a texture from, so getting it from
/// anything but `program-view` is a texture the wrong size — and the wrong size
/// in a way nothing on screen shows, because the picture fills whatever
/// rectangle it is given either way. At `SMALLEST` the bay's own derivation
/// says exactly what it should be: the centre track is 484, `.program-body`'s
/// 9px padding leaves 466, and 466 at 16:9 is 262. Those are the two numbers
/// the arrangement's 378 was built from, arrived at from the other end.
///
/// The region and the canvas agree here to a quarter of a pixel and not
/// exactly, which is the whole reason `picture_rect` rounds: `.program-view` at
/// 466 wide is 262.125 tall and the arrangement transcribed 262, so the box is
/// 1.778626 where the canvas is 1.777778. A strict fit would hand back 465.7778
/// and a caller's `physical` would round it back to a 466-texel texture — the
/// same texture, drawn softened into a box a quarter of a pixel narrower than
/// itself.
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
