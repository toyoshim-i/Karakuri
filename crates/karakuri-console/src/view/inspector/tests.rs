//! What is private and worth asserting, and it is in `src/` rather than in
//! `tests/` for that one reason.
//!
//! Every region of this console is asserted from a file in `tests/`, one per
//! region, and everything here would have been in one of them. It is here
//! because what it names is private, and moving it out would mean widening a
//! surface to test it — a wider surface for a narrower reason.
//!
//! Moved here from `view/mod.rs`, on [`transport`]'s own precedent:
//! [`pane_box`] and [`group_h`] are the pane's arithmetic, and the public
//! [`inspector`] is that arithmetic plus a layout lookup — the reasoning that
//! was already written here travels with the code under
//! [ADR-0121](../../../../docs/adr/0121-moving-code-leaves-its-reasoning-behind.md),
//! the same rule `offset_text`'s own assertion moved under, one bay over.
//!
//! What is *not* here and belongs in `tests/inspector.rs` when somebody writes
//! it: the bay drawn against a real arrangement at the two viewports, on
//! `tests/library.rs`'s pattern.

use super::*;

/// A pane with `params` parameters under one node and no renderers.
fn one_node(params: usize) -> Pane {
    Pane {
        deck: 0,
        material: "drift_shell + soft_points".to_owned(),
        sync: Sync::Tempo,
        // Nothing refused, which is what a pane about the arithmetic of
        // its rows says about material it is not asking after.
        allows: [true; SYNCS.len()],
        anchor_bpm: 128.0,
        scrub_beats: 0.0,
        composite: false,
        // Not this module's row: the deck head's two build chips are drawn
        // from this, and everything here is about the rows under it.
        aimed: None,
        nodes: vec![Node {
            addr: "L1:0".to_owned(),
            name: "drift_shell".to_owned(),
            authority: Some(NodeAuthority {
                at: NodeAddress {
                    layer: Layer::L1,
                    index: 0,
                },
                level: Authority::Manual,
            }),
            // **A node with a source**, which every node but the built-in
            // camera has — and the capsule this draws is what the tests
            // below measure the head's right-hand end against.
            keep: Some(NodeAddress {
                layer: Layer::L1,
                index: 0,
            }),
            uses: Vec::new(),
            renderers: Vec::new(),
            params: (0..params)
                .map(|n| Param {
                    ord: Some(n + 1),
                    name: format!("p{n}"),
                    value: 0.5,
                    range: [0.0, 1.0],
                    param: karakuri_operation::ParamAt {
                        node: None,
                        key: format!("p{n}"),
                    },
                    bound: None,
                })
                .collect(),
        }],
    }
}

/// A rectangle the size of one inspector pane at the narrowest console the mock
/// will draw: `.console`'s `min-width: 1010px` holds the centre at 484 and each
/// pane at 237.
fn pane_at(height: f32) -> Rect {
    Rect::from_min_size(Pos2::new(0.0, 0.0), egui::vec2(237.0, height))
}

/// The two heads are the mock's boxes and the leftover is the groups'.
///
/// `.half-head` is `5 + 16.5 + 5` over its own `border-bottom`, and
/// `.deck-head` is `5 + 15.5 + 5` with none — the stylesheet's own arithmetic
/// for that row. Everything under them is the body.
#[test]
fn a_pane_is_two_heads_and_what_is_left() {
    let pane = one_node(2);
    let at = pane_box(pane_at(400.0), &pane.nodes, 0.0).expect("a pane with room in it");
    assert_eq!(at.head.height(), 27.5);
    assert_eq!(at.deck_head.height(), 25.5);
    assert_eq!(at.head.max.y, at.deck_head.min.y);
    assert_eq!(at.deck_head.max.y, at.body.min.y);
    assert_eq!(at.body.max.y, 400.0);
}

/// The inspector's own minimum is the least this draws, and the two are one
/// derivation: a pane at the minimum less the bay head has room for exactly the
/// one group of two parameters the minimum is written from, and a pixel less
/// has room for none of it.
///
/// This is what the 151.5 in `lib.rs` *means*: bay head 27, `.half-head` 27.5,
/// `.deck-head` 25.5, `.node-head` 26.5 and two `.param` rows at 22.5.
#[test]
fn the_bays_minimum_is_the_least_a_pane_can_draw() {
    let pane = one_node(2);
    let pane_h = 151.5 - size::HEAD_H;
    let at = pane_box(pane_at(pane_h), &pane.nodes, 0.0).expect("a pane at the minimum");
    assert_eq!(
        at.shown, 1,
        "one node and two of its parameters is what the minimum is written from"
    );
    let short = pane_box(pane_at(pane_h - 0.5), &pane.nodes, 0.0).expect("still two heads");
    assert_eq!(
        short.shown, 0,
        "half a pixel under the minimum and the group no longer fits"
    );
}

/// A group the pane cannot hold whole is not counted as shown, which is what
/// the head's `n of m` means since the pane started scrolling: it is drawn, as
/// far as the pane goes, and it is not one of the ones the readout says are on
/// screen.
#[test]
fn a_group_that_does_not_fit_whole_is_not_counted() {
    let pane = Pane {
        nodes: vec![one_node(2).nodes[0].clone(), one_node(5).nodes[0].clone()],
        ..one_node(2)
    };
    // Room for the first group, the hairline, and all but the last row of
    // the second.
    let first = size::NODE_HEAD_H + size::PARAM_H * 2.0;
    let second = size::NODE_HEAD_H + size::PARAM_H * 5.0;
    let body = first + size::HAIRLINE + second - size::PARAM_H;
    let at = pane_box(
        pane_at(size::HALF_HEAD_H + size::DECK_HEAD_H + body),
        &pane.nodes,
        0.0,
    )
    .expect("a pane with room in it");
    assert_eq!(
        at.shown, 1,
        "the second group is short by one row, so one is whole"
    );

    // One row more and both are drawn.
    let at = pane_box(
        pane_at(size::HALF_HEAD_H + size::DECK_HEAD_H + body + size::PARAM_H),
        &pane.nodes,
        0.0,
    )
    .expect("a pane with room in it");
    assert_eq!(at.shown, 2);
}

/// Where a group goes is the sum of the groups above it and the rules between
/// them, and the rule is between rather than under: *n* groups carry *n - 1* of
/// them, because `.node-group:last-child` has none.
#[test]
fn a_group_stacks_under_the_one_before_it_with_a_rule_between() {
    let pane = Pane {
        nodes: vec![
            one_node(2).nodes[0].clone(),
            one_node(1).nodes[0].clone(),
            one_node(3).nodes[0].clone(),
        ],
        ..one_node(2)
    };
    let at = pane_box(pane_at(400.0), &pane.nodes, 0.0).expect("a pane with room in it");
    assert_eq!(at.shown, 3);
    let first = at.group(&pane.nodes, 0);
    let second = at.group(&pane.nodes, 1);
    let third = at.group(&pane.nodes, 2);
    assert_eq!(first.min.y, at.body.min.y);
    assert_eq!(second.min.y, first.max.y + size::HAIRLINE);
    assert_eq!(third.min.y, second.max.y + size::HAIRLINE);
    assert_eq!(first.height(), size::NODE_HEAD_H + size::PARAM_H * 2.0);
    assert_eq!(second.height(), size::NODE_HEAD_H + size::PARAM_H);
}

/// A renderer row costs a group its own height, and a group without one costs
/// nothing: the mock draws `.rend-row` in the `L4` group and nowhere else.
#[test]
fn a_renderer_row_is_a_row_of_the_group_it_is_in() {
    let mut node = one_node(1).nodes[0].clone();
    let without = group_h(&node);
    node.renderers = vec![Renderer {
        name: "soft_points".to_owned(),
        live: true,
    }];
    assert_eq!(group_h(&node), without + size::REND_ROW_H);
}

/// The anchor is the mock's own two spellings, and free shows neither.
///
/// *"`T128` is the tempo a deck was engaged at … `B128 +0.25` is that with the
/// deck sitting a quarter beat ahead of the room. A free deck shows neither."*
#[test]
fn the_anchor_reads_what_the_mock_reads() {
    let mut pane = one_node(1);
    pane.sync = Sync::Tempo;
    pane.scrub_beats = 0.25;
    assert_eq!(
        anchor_text(&pane).as_deref(),
        Some("T128"),
        "tempo sync does not read the offset, so it is not drawn"
    );
    pane.sync = Sync::Beat;
    assert_eq!(anchor_text(&pane).as_deref(), Some("B128 +0.25"));
    pane.sync = Sync::Free;
    assert_eq!(
        anchor_text(&pane),
        None,
        "free is the absence of a transport rather than a setting"
    );
}

/// The pane head names the deck it is pointed at and what is in it, which is
/// the mock's `deck A · drift_night`.
#[test]
fn the_pane_head_says_which_deck_it_is_showing() {
    let mut pane = one_node(1);
    pane.deck = 1;
    assert_eq!(showing_text(&pane), "deck B · drift_shell + soft_points");
}

/// Every level the vocabulary names is on the node head.
///
/// `karakuri_operation::Authority` carries no `ALL` — *"it arrives with the
/// first reader"* — so [`AUTHORITIES`] is this crate's list, and the thing that
/// can go wrong is the list falling behind the vocabulary while [`auth_word`]
/// is updated. This holds the two together: every level [`auth_word`] can spell
/// is in the array exactly once, and the array is in the vocabulary's own
/// declaration order.
#[test]
fn every_authority_the_vocabulary_names_is_on_the_node_head() {
    // A `match` that a fourth level would not compile past, which is what
    // makes this a check on the *array* rather than on the enum.
    let expected = [
        Authority::Manual,
        Authority::Suggesting,
        Authority::Automatic,
    ];
    assert_eq!(
        AUTHORITIES.len(),
        expected.len(),
        "a level the vocabulary names is missing from the node head"
    );
    for (n, level) in expected.into_iter().enumerate() {
        assert_eq!(AUTHORITIES[n], level, "the three are in declaration order");
        assert!(
            !auth_word(level).is_empty(),
            "every level has the console's own abbreviation for it"
        );
    }
    let words: Vec<&str> = AUTHORITIES.into_iter().map(auth_word).collect();
    assert_eq!(
        words,
        vec!["man", "sug", "auto"],
        "the manual's node head reads `man / sug / auto`"
    );
}

/// A pane narrower than a parameter row's own padding is no pane, which is the
/// picture's rule stated across the axis.
#[test]
fn a_pane_with_no_room_across_it_draws_nothing() {
    let pane = one_node(2);
    let narrow = Rect::from_min_size(
        Pos2::new(0.0, 0.0),
        egui::vec2(size::PARAM_PAD_L + size::PARAM_PAD_R, 400.0),
    );
    assert!(pane_box(narrow, &pane.nodes, 0.0).is_none());
}

/// There are two panes and there is no third.
///
/// The mock's `2 up ▾` is an operator choosing how many while running, and a
/// [`Spec`](karakuri_layout::Spec) builds a [`Layout`](karakuri_layout::Layout)
/// once — so the count is the arrangement's, and a caller asking for a pane it
/// has not got gets `None` rather than one of the two it has.
///
/// And a console with no deck behind it hands over no pane at all, which is
/// every test in this crate.
#[test]
fn there_are_two_panes_and_no_third() {
    let mut layout = crate::layout();
    layout.set_viewport(karakuri_layout::Rect {
        x: 0.0,
        y: 0.0,
        w: 1920.0,
        h: 1080.0,
    });
    layout.solve();
    let pane = one_node(2);
    assert_eq!(PANES, 2, "the mock's `.insp-split` is `1fr 9px 1fr`");
    for index in 0..PANES {
        assert!(
            inspector(&layout, index, &pane, 0.0).is_some(),
            "pane {index} is in the arrangement and has room in it"
        );
    }
    assert!(
        inspector(&layout, PANES, &pane, 0.0).is_none(),
        "a third pane is a pane the arrangement has not got"
    );
    assert!(View::new(Room::Day).inspector.is_empty());
}

/// A pane starts under the bay head and not on top of it.
///
/// The one thing `pane_box`'s own tests cannot see. They are handed a rectangle
/// and the head has already been taken off it — the fixture says so in the
/// arithmetic, `151.5 - size::HEAD_H` — so every one of them passes whether or
/// not the caller does the subtraction. It did not: `.half-head` was drawn at
/// the top of `inspector-1`, which is the top of the bay, which is where
/// [`bay_head`] paints the word `Inspector`. Read against the arrangement
/// rather than against a fixture, because the fixture is what could not tell.
#[test]
fn a_pane_starts_under_the_bay_head() {
    let mut layout = crate::layout();
    layout.set_viewport(karakuri_layout::Rect {
        x: 0.0,
        y: 0.0,
        w: 1920.0,
        h: 1080.0,
    });
    layout.solve();
    let pane = one_node(2);
    let bay = to_egui(layout.rect(layout.find("inspector").expect("the bay is named")));
    for index in 0..PANES {
        let at = inspector(&layout, index, &pane, 0.0).expect("a pane with room in it");
        assert_eq!(
            at.head.min.y,
            bay.min.y + size::HEAD_H,
            "pane {index} starts where the bay head ends"
        );
        assert!(
            bay.contains_rect(at.head) && bay.contains_rect(at.deck_head),
            "pane {index} draws inside the bay it is in"
        );
    }
}
