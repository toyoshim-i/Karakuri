#![allow(unused_imports)]

use super::grammar_common::*;

// ---------------------------------------------------------------------------
// `space`
// ---------------------------------------------------------------------------

/// The key and the chip are one cycle. The three states are asked of the same
/// three functions `view::Mixer`'s chips ask, so a press and a click cannot
/// disagree about which state comes next.
#[test]
fn space_on_a_state_names_the_state_the_chip_would_name() {
    let cases: [(usize, Operation); 3] = [
        (
            1,
            // Deck B's requested residency is `priming`, so the next is
            // `allocated` — which is the withdrawal of the prime request, said
            // by the ordinary arithmetic.
            Operation::SetResidency {
                deck: 1,
                residency: Residency::Allocated,
            },
        ),
        (
            4,
            Operation::SetBlendMode {
                deck: 1,
                blend: BlendMode::Max,
            },
        ),
        (
            5,
            Operation::SetMaskShape {
                deck: 1,
                kind: WipeKind::Radial,
                angle: 0.25,
            },
        ),
    ];
    for (control, want) in cases {
        let (panel, mut view) = console();
        focus_on(&mut view, &panel, "mixer");
        press(&mut view, &panel, Press::Digit(2));
        press(&mut view, &panel, Press::Digit(control));
        assert_eq!(
            press(&mut view, &panel, Press::Space),
            Asked::Emitted(want.clone()),
            "`space` on deck B's control {control} did not ask for `{want:?}`"
        );
    }
}

/// Space key on a level resets to its declared default value (ADR-0259).
#[test]
fn space_on_a_level_is_the_value_it_was_declared_at() {
    let (panel, mut view) = console();
    focus_on(&mut view, &panel, "mixer");
    press(&mut view, &panel, Press::Digit(1));
    for (control, level) in [(2, Level::Trim(0)), (3, Level::Fader(0))] {
        press(&mut view, &panel, Press::Digit(control));
        assert_eq!(
            press(&mut view, &panel, Press::Space),
            Asked::Stepped {
                level,
                step: Step::Default
            }
        );
        view.focus_up(&panel);
    }
}

#[test]
fn space_on_the_librarys_head_is_the_scope() {
    let (panel, mut view) = console();
    focus_on(&mut view, &panel, "library");
    press(&mut view, &panel, Press::Digit(HEAD));
    press(&mut view, &panel, Press::Digit(1));
    assert_eq!(
        press(&mut view, &panel, Press::Space),
        Asked::Scope,
        "`0 1 space` in the Library is not the scope. ADR-0259: *`0` is the head and its controls \
         are the scope chips, so `space` there cycles the scope*"
    );
}

/// The transition row is the Mixer's head, which is the question ADR-0333 left
/// open and ADR-0343 answers: the settings are about the bay rather than about
/// any one strip, and that is what a head is.
#[test]
fn the_transition_rows_three_settings_are_the_mixers_head() {
    let cases: [(usize, TransitionSetting); 3] = [
        (
            1,
            TransitionSetting::WipeShape {
                kind: WipeKind::Linear,
                angle: 0.0,
            },
        ),
        (2, TransitionSetting::Quantum { beats: 1.0 }),
        (3, TransitionSetting::Length { beats: 2.0 }),
    ];
    for (control, setting) in cases {
        let (panel, mut view) = console();
        walk_to(&mut view, &panel, "mixer", &[HEAD, control]);
        assert_eq!(
            press(&mut view, &panel, Press::Space),
            Asked::Emitted(Operation::SetTransition { setting }),
            "`0 {control} space` in the Mixer did not step the transition row's {control}th pill"
        );
    }
}

/// A row's own controls are its star and its `params` chip, which is what
/// ADR-0333 left owed: *"a row has no state"* was written before the star was
/// drawn, and a star is a state.
#[test]
fn a_library_rows_star_is_addressable() {
    let (panel, mut view) = console();
    walk_to(&mut view, &panel, "library", &[2, 1]);
    assert_eq!(
        press(&mut view, &panel, Press::Space),
        Asked::Emitted(Operation::SetFavourite {
            id: "two".to_owned(),
            favourite: true,
        }),
        "`2 1 space` in the Library did not star the second row"
    );
    // And a Set already starred is un-starred, which is the state the press
    // names rather than a flip anything downstream works out.
    view.starred.insert("two".to_owned());
    assert_eq!(
        press(&mut view, &panel, Press::Space),
        Asked::Emitted(Operation::SetFavourite {
            id: "two".to_owned(),
            favourite: false,
        })
    );
}

/// The Program head's `solo` is `s` and `u` collapsed into the one control they
/// always described (ADR-0259), and it is a move of the arrangement rather than
/// an operation of the vocabulary.
#[test]
fn space_on_the_programs_solo_is_the_solo_and_the_unsolo() {
    let (panel, mut view) = console();
    walk_to(&mut view, &panel, "program", &[HEAD, 1]);
    let id = panel.layout().find("program-view").expect("the picture");
    assert_eq!(
        press(&mut view, &panel, Press::Space),
        Asked::Panel(Op::Solo(id)),
        "`0 1 space` in the Program bay did not solo the picture"
    );
}

/// The Outputs row is the simplest of the nine: one item, one state, and one
/// press asking for the operation that names the output and the fold that
/// carries it out.
#[test]
fn space_on_a_sink_routes_the_frame_and_folds_the_picture() {
    let (panel, mut view) = console();
    walk_to(&mut view, &panel, "outputs", &[1]);
    let id = panel.layout().find("program-view").expect("the picture");
    assert_eq!(
        press(&mut view, &panel, Press::Space),
        Asked::Routed(
            Operation::RouteFrame {
                output: karakuri_operation::Output::Program,
                on: false,
            },
            Op::Fold(id),
        ),
        "`1 space` in the Outputs row did not turn the picture off"
    );
}

/// The Sequencer's four `space` rows, each naming the state it arrives at
/// rather than a flip — and the cell carries the stored slot rather than the
/// drawn step, which is what keeps the payload independent of the mode.
#[test]
fn space_in_the_sequencer_names_the_state_it_arrives_at() {
    let cases: [(&[usize], Operation); 4] = [
        (
            &[HEAD, 1],
            Operation::SetPatternGrid {
                pattern: 2,
                grid: StepMode::Eighth,
            },
        ),
        (&[HEAD, 4], Operation::SelectPattern { pattern: 2 }),
        (
            &[1, 1],
            Operation::SetLaneMute {
                pattern: 2,
                lane: 0,
                muted: true,
            },
        ),
        (
            &[2, 3],
            Operation::SetStep {
                pattern: 2,
                lane: 1,
                step: 1,
                on: true,
            },
        ),
    ];
    for (path, want) in cases {
        let (panel, mut view) = console();
        walk_to(&mut view, &panel, "sequencer", path);
        assert_eq!(
            press(&mut view, &panel, Press::Space),
            Asked::Emitted(want.clone()),
            "`{path:?} space` in the Sequencer did not ask for `{want:?}`"
        );
    }
    // And the second lane is muted already, so its label asks to be unmuted —
    // which is the state being named rather than a flip.
    let (panel, mut view) = console();
    walk_to(&mut view, &panel, "sequencer", &[2, 1]);
    assert_eq!(
        press(&mut view, &panel, Press::Space),
        Asked::Emitted(Operation::SetLaneMute {
            pattern: 2,
            lane: 1,
            muted: false,
        })
    );
}

/// `enter` on a lane takes that lane out of the pattern, by the position the bay
/// drew it at — the minus at the end of the row, reached by the one key that can
/// reach it (a lane draws eighteen controls and the digits stop at nine).
#[test]
fn enter_on_a_lane_takes_it_out_of_the_pattern() {
    for (lane, at) in [(1usize, 0u8), (2, 1)] {
        let (panel, mut view) = console();
        walk_to(&mut view, &panel, "sequencer", &[lane]);
        assert_eq!(
            press(&mut view, &panel, Press::Enter),
            Asked::Emitted(Operation::RemoveLane {
                pattern: 2,
                lane: at
            }),
            "`{lane} enter` in the Sequencer did not take lane {at} out of bank 2"
        );
    }
    // A lane this pattern does not draw is refused and says what was named,
    // rather than taking the nearest row out.
    let (panel, mut view) = console();
    focus_on(&mut view, &panel, "sequencer");
    assert!(matches!(
        press(&mut view, &panel, Press::Digit(3)),
        Asked::Nothing(_)
    ));
}

/// The Inspector's four chips, each on the third rung and each naming the state
/// it arrives at.
#[test]
fn space_in_the_inspector_cycles_the_four_chips() {
    let cases: [(&[usize], Operation); 4] = [
        (
            &[2, 1, 1],
            Operation::SetSync {
                deck: 1,
                // Beat is the last of the three and the cycle wraps.
                sync: Sync::Free,
            },
        ),
        (
            &[2, 1, 3],
            Operation::SetCompositing {
                deck: 1,
                compositing: false,
            },
        ),
        (
            &[2, 2, 1],
            Operation::SetAuthority {
                deck: 1,
                node: NodeAddress {
                    layer: Layer::L1,
                    index: 0,
                },
                authority: AUTHORITIES[1],
            },
        ),
        (
            &[2, 2, 2],
            Operation::SelectRenderer {
                deck: 1,
                renderer: 1,
            },
        ),
    ];
    for (path, want) in cases {
        let (panel, mut view) = console();
        walk_to(&mut view, &panel, "inspector", path);
        assert_eq!(
            press(&mut view, &panel, Press::Space),
            Asked::Emitted(want.clone()),
            "`{path:?} space` in the Inspector did not ask for `{want:?}`"
        );
    }
}

/// The tone map cycles the four operators, which is the console's own cycle and
/// the same one the pill walks.
#[test]
fn space_on_the_tone_map_cycles_the_operators() {
    let (panel, mut view) = console();
    walk_to(&mut view, &panel, "transport", &[9]);
    let said = press(&mut view, &panel, Press::Space);
    match said {
        Asked::Emitted(Operation::SetTonemap { tonemap }) => assert_ne!(
            tonemap,
            Tonemap::Aces,
            "the tone map cycled onto the operator it was already running"
        ),
        other => panic!("`9 space` in the Transport is not the tone map: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// `space` on a bay: the fold
// ---------------------------------------------------------------------------

/// `space` at bay level folds the focused bay, in every one of the nine —
/// ADR-0259's rule, which ADR-0333 declined to bind while it meant it in two.
#[test]
fn space_on_a_bay_folds_it_in_every_bay() {
    for bay in focus::BUILT {
        let (panel, mut view) = console();
        focus_on(&mut view, &panel, bay.bay);
        let id = panel.layout().find(bay.bay).expect("the bay's node");
        assert_eq!(
            press(&mut view, &panel, Press::Space),
            Asked::Panel(Op::Fold(id)),
            "`space` at bay level in `{}` is not the fold. It is the same press in all nine, \
             which is what lets *Fold a bay away* carry one badge",
            bay.bay
        );
    }
}

/// And a folded bay answers `space` and nothing else, which is the narrow
/// reason it keeps its place in the ring: it is there so that there is
/// something to press to open it, not so that it can be operated.
#[test]
fn a_folded_bay_answers_space_and_declines_the_other_three() {
    let (mut panel, mut view) = console();
    let id = panel.layout().find("master").expect("the master bay");
    panel.op(Op::Fold(id));
    panel.solve();
    focus_on(&mut view, &panel, "master");
    assert_eq!(
        press(&mut view, &panel, Press::Space),
        Asked::Panel(Op::Unfold(id)),
        "`space` on a folded bay did not open it, which is the one thing it is in the ring for"
    );
    for key in [Press::Digit(1), Press::Arrow(Arrow::Down), Press::Enter] {
        let said = press(&mut view, &panel, key);
        assert!(
            matches!(said, Asked::Nothing(_)),
            "`{key:?}` acted on a bay that is folded away: {said:?}"
        );
    }
}

/// And the mark is drawn, which is the one drawing ADR-0259 created a need for:
/// a folded region has no rectangle, so the ring alone would land on nothing an
/// operator could read.
#[test]
fn a_folded_bay_holding_focus_wears_the_mark() {
    let (mut panel, mut view) = console();
    focus_on(&mut view, &panel, "master");
    assert!(
        view.folded_mark(&panel).is_none(),
        "an open bay wore the folded mark"
    );
    let ring = view
        .focus_mark(&panel)
        .expect("an open bay's head is ringed");

    let id = panel.layout().find("master").expect("the master bay");
    panel.op(Op::Fold(id));
    panel.solve();
    assert!(
        view.focus_mark(&panel).is_none(),
        "a folded region has no rectangle, so the ordinary ring has nothing to sit on"
    );
    let (mark, title) = view
        .folded_mark(&panel)
        .expect("a folded bay holding focus wears the head alone");
    assert_eq!(
        title, "Master",
        "the mark does not name the bay that is there"
    );
    assert!(
        mark.height() > 0.0 && mark.width() > 0.0,
        "the mark is an empty rectangle, which is nothing drawn"
    );
    assert!(
        mark.width() >= ring.width() * 0.5,
        "the mark is not the width of the bay it stands for: {mark:?} against {ring:?}"
    );

    // An unfolded bay displays no folded mark regardless of other bays' fold states.
    focus_on(&mut view, &panel, "mixer");
    assert!(view.folded_mark(&panel).is_none());
}

// ---------------------------------------------------------------------------
// `enter`
// ---------------------------------------------------------------------------

#[test]
fn enter_on_a_library_row_is_the_load_and_on_its_params_chip_is_the_reading() {
    let (panel, mut view) = console();
    focus_on(&mut view, &panel, "library");
    press(&mut view, &panel, Press::Digit(2));
    assert_eq!(
        press(&mut view, &panel, Press::Enter),
        Asked::Load,
        "`enter` on a library row is not the load"
    );
    assert_eq!(
        view.cursor_row(),
        1,
        "the digit that named the row did not move the cursor the load reads"
    );
    // And its second control is the `params` chip, which opens what that Set
    // holds and declares.
    press(&mut view, &panel, Press::Digit(2));
    assert_eq!(
        press(&mut view, &panel, Press::Enter),
        Asked::Emitted(Operation::ReadSet {
            id: "two".to_owned()
        })
    );
}

/// A candidate row's two acts are two controls and a digit chooses between
/// them, which is ADR-0259's `n 1` and `n 2`.
#[test]
fn enter_on_a_staging_rows_two_controls_keeps_it_and_puts_the_previous_back() {
    let node = NodeAddress {
        layer: Layer::L2,
        index: 0,
    };
    let (panel, mut view) = console();
    walk_to(&mut view, &panel, "staging", &[2, 1]);
    assert_eq!(
        press(&mut view, &panel, Press::Enter),
        Asked::Emitted(Operation::KeepCandidate { deck: 1, node }),
        "`2 1 enter` in Staging did not keep the second candidate"
    );
    let (panel, mut view) = console();
    walk_to(&mut view, &panel, "staging", &[2, 2]);
    assert_eq!(
        press(&mut view, &panel, Press::Enter),
        Asked::Emitted(Operation::RestoreProcedure {
            deck: 1,
            revision: Revision::Previous(node),
        }),
        "`2 2` in Staging did not put the node's previous version back"
    );
}

/// `enter` on the Mixer's head runs the transition on the addressed strip,
/// which is the deck selection: this deck is covered and the next one round
/// arrives over it.
#[test]
fn enter_on_the_transition_rows_go_wipes_the_next_deck_in() {
    let (panel, mut view) = console();
    focus_on(&mut view, &panel, "mixer");
    // A shape has to be chosen, and the refusal says so first.
    walk_to(&mut view, &panel, "mixer", &[HEAD, 4]);
    match press(&mut view, &panel, Press::Enter) {
        Asked::Nothing(why) => assert!(
            why.contains("shape"),
            "`go` with no shape chosen declined without saying which pill picks one: {why}"
        ),
        other => panic!("a wipe ran with no shape chosen: {other:?}"),
    }
    view.set_transition(TransitionSetting::WipeShape {
        kind: WipeKind::Radial,
        angle: 0.0,
    });
    view.select(2);
    assert_eq!(
        press(&mut view, &panel, Press::Enter),
        Asked::Emitted(Operation::Wipe { from: 2, to: 3 }),
        "`0 4 enter` in the Mixer did not wipe the next deck in over the addressed one"
    );
}

/// A parameter row performs as well as sets, which is the one control on the
/// panel that is two of ADR-0259's kinds at once: the sensitivity row under it
/// carries `take back`, and a row with nothing holding it draws none.
#[test]
fn enter_on_a_bound_parameter_takes_the_attachment_back() {
    let (panel, mut view) = console();
    // `1 2 5` — the third parameter of the pane's node, which is the bound one.
    walk_to(&mut view, &panel, "inspector", &[1, 2, 5]);
    assert_eq!(
        press(&mut view, &panel, Press::Enter),
        Asked::Emitted(Operation::TakeParamBack {
            deck: 0,
            param: karakuri_operation::BindAt {
                layer: Layer::L1,
                index: Some(0),
                key: "spin".to_owned(),
            },
        })
    );
    // And a row nothing is holding says so rather than emitting a take-back
    // for an attachment that is not there.
    let (panel, mut view) = console();
    walk_to(&mut view, &panel, "inspector", &[1, 2, 3]);
    match press(&mut view, &panel, Press::Enter) {
        Asked::Nothing(why) => assert!(
            why.contains("holding"),
            "an unbound parameter's `enter` declined without saying why: {why}"
        ),
        other => panic!("a row nothing is holding was taken back: {other:?}"),
    }
}

#[test]
fn enter_declines_where_a_bays_items_perform_nothing() {
    let (panel, mut view) = console();
    focus_on(&mut view, &panel, "mixer");
    press(&mut view, &panel, Press::Digit(1));
    let refused = press(&mut view, &panel, Press::Enter);
    assert!(
        matches!(refused, Asked::Nothing(_)),
        "`enter` performed something on a strip. A strip's five controls all set rather than \
         perform, and this bay's one act is its head's `go`: {refused:?}"
    );
}

// ---------------------------------------------------------------------------
// The refusals
// ---------------------------------------------------------------------------

/// Asserts inert items (like preview cells) refuse space and enter with descriptive explanations (P-0083, ADR-0259).
#[test]
fn every_refusal_says_why() {
    let (panel, mut view) = console();
    walk_to(&mut view, &panel, "program", &[2]);
    for key in [Press::Space, Press::Enter] {
        match press(&mut view, &panel, key) {
            Asked::Nothing(why) => assert!(
                why.len() > 20,
                "a preview cell declined `{key:?}` without saying why: {why}"
            ),
            other => panic!(
                "a preview cell answered `{key:?}` with {other:?} — a monitor is a \
                             thing you look at"
            ),
        }
    }
    // And a slot of the master chain, which is a rung rather than a control:
    // its parameter rows, its cut chip and its `−` are what answer a key.
    let (panel, mut view) = console();
    walk_to(&mut view, &panel, "master", &[2]);
    match press(&mut view, &panel, Press::Space) {
        Asked::Nothing(why) => assert!(
            why.contains("rung"),
            "a chain slot declined without saying it is a rung: {why}"
        ),
        other => panic!("a chain slot answered space: {other:?}"),
    }
}
