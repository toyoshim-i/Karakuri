//! The six keys inside a bay: what a digit names, what the arrows walk, what
//! `space` cycles and what `enter` performs.
//!
//!
//! [ADR-0259](../../../docs/adr/0259-the-keyboard-is-addressed-to-the-bay-that-has-focus-and-a-global-letter-is-a-convenience-or-the-operators-own.md)
//! designed the grammar,
//! [ADR-0333](../../../docs/adr/0333-the-console-resolves-the-address-and-the-window-loop-names-the-operation.md)
//! built it in two bays and
//! [ADR-0343](../../../docs/adr/0343-the-grammar-reaches-all-nine-bays-and-space-on-a-bay-is-the-fold.md)
//! in the other seven. This holds the console's half: the address a press moves
//! and what it says the press landed on. That the window loop then names the
//! right operation is `crates/karakuri/src/main.rs`'s `key_column`, and that a
//! press on a running panel moves a deck is `mod gpu`'s.
//!
//! Eight claims:
//!
//! 1. A digit names the nth thing one level below the address and `0` the head,
//! counting what the bay drew from one. 2. Naming a strip is the deck
//! selection, which is why that row keeps a key badge rather than losing one —
//! and it goes through `View::select`, so a deck the mixer draws no strip for
//! is refused and the address does not descend. 3. The arrows take the
//! neighbour along the axis the bay draws its items on, and the next value of a
//! level along the other. The Sequencer is where both axes are used at once —
//! its lanes are a column and a lane's cells are a row — and that is the axis
//! check ADR-0333 left with nothing holding it. 4. `space` is the addressed
//! thing's next state, and it is the same cycle the chip walks — asked of the
//! same functions rather than restated. 5. `space` on a bay is the fold, in
//! every one of the nine, and a folded bay answers it and nothing else. 6.
//! `enter` is the act the addressed thing is for, and it declines in a bay
//! whose items perform nothing. 7. The address is a path, which the Inspector
//! is what proves: three rungs, and the third is reached by a digit through a
//! rung that is not a control. 8. A refusal says why. A key that declines and a
//! key that is not bound are the same experience, so every `Nothing` carries a
//! sentence.
//!
//! No device and no `egui` pass. A press is a walk of a path.

mod common;

use common::PLAUSIBLE;
use karakuri_console::focus::{self, Arrow, Asked, Control, Grammar, Level, Press, Step, HEAD};
use karakuri_console::panel::{Op, Panel};
use karakuri_console::room::Room;
use karakuri_console::view::{
    Candidate, Chain, Look, Mask, Node, NodeAuthority, Pane, Param, Renderer, Scope, Sequenced,
    Stage, Strip, Tally, Tracker, View, AUTHORITIES, SCRUB_BEATS, SYNCS,
};
use karakuri_operation::LaneTarget;
use karakuri_operation::{
    Authority, BlendMode, Curve, Layer, NodeAddress, Operation, ParamAt, ParamValue, Residency,
    Revision, StepMode, Sync, Tonemap, TransitionSetting, Undecided, WipeKind,
};
use karakuri_pattern::{Lane, Pattern};

/// The mixer's four strips, at the values this file steps from. Every one of
/// them differs from its neighbours in the three states, so a cycle that
/// answered from the wrong strip comes out wrong rather than right by luck.
fn strip(at: usize) -> Strip {
    Strip {
        name: format!("set {at}"),
        tally: Tally::Live,
        requested: [Tally::Live, Tally::Priming, Tally::Allocated, Tally::Live][at],
        gain: 1.0,
        gain_to: None,
        opacity: 1.0,
        opacity_to: None,
        blend: [
            BlendMode::Add,
            BlendMode::Over,
            BlendMode::Max,
            BlendMode::Add,
        ][at],
        mask: [Mask::None, Mask::Linear, Mask::Radial, Mask::None][at],
        mask_angle: 0.25,
        level: None,
    }
}

/// One parameter row, published at `ord` over `range` and holding `value`.
fn param(ord: usize, name: &str, range: [f32; 2], value: f32) -> Param {
    Param {
        ord: Some(ord),
        name: name.to_owned(),
        value,
        range,
        param: ParamAt {
            node: Some(NodeAddress {
                layer: Layer::L1,
                index: 0,
            }),
            key: name.to_owned(),
        },
        bound: None,
    }
}

/// The pane's one node: an authority chip, two renderers with the first live,
/// and three parameter rows. The third is bound, so `enter` on it has an
/// attachment to take back and the two above it have none.
fn node() -> Node {
    Node {
        keep: None,
        addr: "L1:0".to_owned(),
        name: "drift_shell".to_owned(),
        authority: Some(NodeAuthority {
            at: NodeAddress {
                layer: Layer::L1,
                index: 0,
            },
            level: Authority::Manual,
        }),
        uses: Vec::new(),
        renderers: ["soft_points", "strand_strokes"]
            .into_iter()
            .enumerate()
            .map(|(index, name)| Renderer {
                name: name.to_owned(),
                live: index == 0,
            })
            .collect(),
        params: vec![
            param(1, "radius", [0.0, 4.0], 2.0),
            param(2, "turbulence", [0.0, 3.0], 1.5),
            Param {
                bound: Some(karakuri_console::view::Source {
                    signal: "energy".to_owned(),
                    curve: Curve::Pow2,
                    range: [0.1, 2.4],
                    at: karakuri_operation::BindAt {
                        layer: Layer::L1,
                        index: Some(0),
                        key: "spin".to_owned(),
                    },
                }),
                ..param(3, "spin", [0.0, 1.0], 0.5)
            },
        ],
    }
}

/// One Inspector pane, on the deck the arrangement points it at.
fn pane(deck: usize) -> Pane {
    Pane {
        deck,
        material: "drift_night".to_owned(),
        sync: Sync::Beat,
        allows: [true; SYNCS.len()],
        anchor_bpm: 128.0,
        scrub_beats: 0.0,
        composite: true,
        aimed: None,
        nodes: vec![node()],
    }
}

/// A pattern with two lanes, the first unmuted with step 0 off and the second
/// muted — so a press on either says the state it arrives at rather than being
/// right by symmetry.
fn pattern() -> Pattern {
    let mut pattern = Pattern::empty();
    pattern.push(Lane::new(LaneTarget::Fader { deck: 0 }, 1.0, 0.0));
    let mut muted = Lane::new(LaneTarget::Fader { deck: 1 }, 1.0, 0.0);
    muted.set_muted(true);
    pattern.push(muted);
    pattern
}

/// A console with every bay drawing something, which is what the seven new rows
/// of the dispatch table need to be asked at all: a bay drawing nothing answers
/// a digit with its own refusal, which is a different claim.
fn console() -> (Panel, View) {
    let mut panel = Panel::new(PLAUSIBLE.w, PLAUSIBLE.h);
    panel.solve();
    let mut view = View::new(Room::Day);
    view.mixer = (0..4).map(strip).collect();
    view.library = ["one", "two", "three"]
        .iter()
        .map(|s| (*s).to_owned())
        .collect();
    view.scopes = Scope::ALL.to_vec();
    view.staging = vec![
        Candidate {
            deck: 0,
            at: Some(NodeAddress {
                layer: Layer::L1,
                index: 0,
            }),
            addr: "L1:0".to_owned(),
            name: "drift_shell".to_owned(),
            stage: Stage::Landed,
            said: Vec::new(),
        },
        Candidate {
            deck: 1,
            at: Some(NodeAddress {
                layer: Layer::L2,
                index: 0,
            }),
            addr: "L2:0".to_owned(),
            name: "veil".to_owned(),
            stage: Stage::Landed,
            said: Vec::new(),
        },
    ];
    view.inspector = vec![pane(0), pane(1)];
    view.master_out = Some(0.8);
    view.master_chain = Some(Chain {
        feedback: 0.2,
        cut: karakuri_operation::Cut::Mix,
        bloom: 0.1,
        rgb_shift: 0.0,
    });
    view.look = Some(Look {
        tonemap: Tonemap::Aces,
        exposure: 1.0,
    });
    view.tracker = Some(Tracker {
        offset_ms: Some(0.0),
        halve: true,
        double: true,
    });
    view.sequencer = Some(Sequenced {
        pattern: pattern(),
        bank: 2,
        step: None,
    });
    (panel, view)
}

/// What the deck is holding, answered off the strips this file built — which is
/// what `crates/karakuri` reads off the *deck* at the press. Here they are the
/// same values, and which of the two a console reads is that file's question
/// rather than this one's.
fn holding(view: &View, deck: u8) -> Option<focus::Held> {
    let strip = view.mixer.get(usize::from(deck))?;
    Some(focus::Held {
        requested: strip.requested,
        blend: strip.blend,
        mask: strip.mask,
        mask_angle: strip.mask_angle,
    })
}

/// Put focus on `bay`, in at most one turn of the ring — bounded for
/// `tests/focus.rs`'s reason: a bay a walk cannot reach is a hang and not a
/// failure.
fn focus_on(view: &mut View, panel: &Panel, bay: &str) {
    for _ in 0..10 {
        if view.focused(panel).map(|found| found.name) == Some(bay) {
            return;
        }
        view.tab(panel, 1);
    }
    panic!("`Tab` did not reach `{bay}` in one turn of the ring");
}

/// One press, with the deck's own readings behind it.
fn press(view: &mut View, panel: &Panel, key: Press) -> Asked {
    let held: Vec<Option<focus::Held>> = (0..4).map(|deck| holding(view, deck)).collect();
    focus::press(view, panel, key, |deck| {
        held.get(usize::from(deck)).copied().flatten()
    })
}

/// A path pressed digit by digit, and the answer to the last press.
fn walk_to(view: &mut View, panel: &Panel, bay: &str, path: &[usize]) {
    focus_on(view, panel, bay);
    for digit in path {
        let said = press(view, panel, Press::Digit(*digit));
        assert!(
            !matches!(said, Asked::Nothing(_)),
            "the walk to {path:?} in `{bay}` was refused at `{digit}`: {said:?}"
        );
    }
}

fn at(view: &View, bay: &str) -> Vec<usize> {
    view.focus()
        .address(bay)
        .map_or_else(Vec::new, |address| address.at().to_vec())
}

// ---------------------------------------------------------------------------
// The table
// ---------------------------------------------------------------------------

/// Which keys act in which bay is derived from what the bay is made of, not
/// listed beside it — so this is the derivation held against the nine bays the
/// record walks.
///
/// The fold is one route and not nine. `space` at bay level works wherever
/// focus is, so it is declared under `focus::ANY` rather than nine times over —
/// which is what lets *Fold a bay away* carry one badge saying `in any bay`.
#[test]
fn the_dispatch_table_is_the_nine_bays_the_record_walks() {
    let reaches = focus::reaches();
    assert_eq!(
        reaches,
        vec![
            (focus::ANY, Grammar::Space),
            ("transport", Grammar::Digit),
            ("transport", Grammar::Arrows),
            ("transport", Grammar::Space),
            ("library", Grammar::Digit),
            ("library", Grammar::Arrows),
            ("library", Grammar::Space),
            ("library", Grammar::Enter),
            ("staging", Grammar::Digit),
            ("staging", Grammar::Arrows),
            ("staging", Grammar::Enter),
            ("program", Grammar::Digit),
            ("program", Grammar::Arrows),
            ("program", Grammar::Space),
            ("inspector", Grammar::Digit),
            ("inspector", Grammar::Arrows),
            ("inspector", Grammar::Space),
            ("inspector", Grammar::Enter),
            ("mixer", Grammar::Digit),
            ("mixer", Grammar::Arrows),
            ("mixer", Grammar::Space),
            ("mixer", Grammar::Enter),
            ("master", Grammar::Digit),
            ("master", Grammar::Arrows),
            ("master", Grammar::Space),
            ("sequencer", Grammar::Digit),
            ("sequencer", Grammar::Arrows),
            ("sequencer", Grammar::Space),
            ("outputs", Grammar::Digit),
            ("outputs", Grammar::Arrows),
            ("outputs", Grammar::Space),
        ],
        "the grammar the console declares is not the one the records walk. `enter` reaches \
         nothing in five of the nine — no item there performs — and `space` reaches nothing in \
         Staging below the fold, which is ADR-0259's own finding about a lane of things that \
         happened"
    );
    assert_eq!(
        focus::BUILT.len(),
        9,
        "the manual's *What each region is standing on* lists nine bays and the dispatch table \
         has a different number of rows — a bay absent from it is a bay the four keys decline in"
    );
}

/// ADR-0259's *no bay uses all six keys* is no longer true, and this is
/// where that is written down.
///
/// The record found it of the panel as it stood: *"`space` reaches nothing in
/// Staging and nothing in Library below its head, and `enter` reaches nothing
/// in Mixer, Outputs, Master or Program — so no bay uses all six keys, and a
/// legend implying otherwise would be lying."* Two of those four clauses have
/// since stopped holding, and neither by anything this grammar decided:
///
/// - The Library gained a star, drawn after ADR-0259 was written, and a
///   star is a state — so `space` reaches a row after all.
/// - The Mixer's `go` capsule is its head's, which is where ADR-0343 puts
///   the transition row — so `enter` reaches a row of that bay.
///
/// So two bays use all four of the keys that act, and seven still do not.
/// The list is asserted rather than the claim, so a third bay joining them is
/// a failure that says which rather than a sentence quietly going false.
#[test]
fn two_bays_use_all_four_of_the_keys_that_act_and_seven_do_not() {
    let all: Vec<&str> = focus::BUILT
        .iter()
        .filter(|bay| Grammar::ALL.iter().all(|key| bay.reaches(*key)))
        .map(|bay| bay.bay)
        .collect();
    assert_eq!(
        all,
        vec!["library", "inspector", "mixer"],
        "which bays answer all four of the acting keys has changed. ADR-0259 found that none \
         did; the Library's star and the Mixer's `go` are what made two of them, and the \
         Inspector's parameter rows are a level that also performs. A fourth is a finding to \
         write down rather than one to pass quietly"
    );
}

// ---------------------------------------------------------------------------
// A digit
// ---------------------------------------------------------------------------

#[test]
fn a_digit_names_the_nth_thing_below_the_address_and_zero_the_head() {
    let (panel, mut view) = console();
    focus_on(&mut view, &panel, "mixer");
    assert_eq!(at(&view, "mixer"), Vec::<usize>::new());

    // `2` is the second strip — deck B — and naming it is the deck selection.
    assert_eq!(
        press(&mut view, &panel, Press::Digit(2)),
        Asked::Emitted(Operation::SelectDeck { deck: 1 }),
        "a digit that names a strip did not name the deck selection with it. ADR-0259: *naming a \
         strip is the deck selection*, which is why that row keeps a key badge"
    );
    assert_eq!(at(&view, "mixer"), vec![2], "the address did not descend");
    assert_eq!(view.selection(), 1, "the selection did not move with it");

    // `3` under it is that strip's third control — the fader. `2 3` is deck B's
    // fader, which is the record's own example.
    assert_eq!(press(&mut view, &panel, Press::Digit(3)), Asked::Moved);
    assert_eq!(at(&view, "mixer"), vec![2, 3]);
    assert_eq!(
        press(&mut view, &panel, Press::Arrow(Arrow::Up)),
        Asked::Stepped {
            level: Level::Fader(1),
            step: Step::Up
        },
        "`2 3` is not deck B's fader — the digits count the controls a strip draws, in the order \
         it draws them: the tally, the trim, the fader, the blend chip and the mask mini"
    );
}

#[test]
fn a_digit_past_what_the_bay_drew_is_refused_and_the_address_stays_put() {
    let (panel, mut view) = console();
    focus_on(&mut view, &panel, "mixer");
    view.mixer.truncate(2);
    let refused = press(&mut view, &panel, Press::Digit(4));
    assert!(
        matches!(refused, Asked::Nothing(_)),
        "deck D was named at a two-strip mixer and was not refused: {refused:?}"
    );
    assert_eq!(
        at(&view, "mixer"),
        Vec::<usize>::new(),
        "the address descended onto a strip the mixer is not drawing. `View::select` is what \
         refuses a deck there is no strip for, and a press it turns down is a press that named \
         nothing"
    );
    assert_eq!(
        view.selection(),
        0,
        "the selection moved on a refused press"
    );
}

/// `0` is the head where a bay draws one and the row itself where it does not,
/// which is ADR-0259's reading of the two headless rows.
#[test]
fn zero_names_the_head_and_a_headless_row_says_it_has_none() {
    let (panel, mut view) = console();
    focus_on(&mut view, &panel, "library");
    assert_eq!(press(&mut view, &panel, Press::Digit(HEAD)), Asked::Moved);
    assert_eq!(at(&view, "library"), vec![HEAD]);

    // The Mixer's head is the transition row since ADR-0343, so `0` descends
    // there too — and its first control is the shape pill.
    let (panel, mut view) = console();
    focus_on(&mut view, &panel, "mixer");
    assert_eq!(press(&mut view, &panel, Press::Digit(HEAD)), Asked::Moved);
    assert_eq!(at(&view, "mixer"), vec![HEAD]);

    // The Transport draws no head, so `0` reaches nothing and says so — and
    // the address stays at the bay rather than descending onto a rung that is
    // not there.
    let (panel, mut view) = console();
    focus_on(&mut view, &panel, "transport");
    let said = press(&mut view, &panel, Press::Digit(HEAD));
    match said {
        Asked::Nothing(why) => assert!(
            why.contains("headless") || why.contains("no head"),
            "a headless row declined `0` without saying it draws no head: {why}"
        ),
        other => panic!("the Transport's `0` descended into a head it does not draw: {other:?}"),
    }
    assert_eq!(at(&view, "transport"), Vec::<usize>::new());
}

/// The Inspector is what proves the address is a path, and it is three rungs: a
/// pane, the thing under it, and the control under that.
#[test]
fn the_inspectors_address_is_three_rungs_deep() {
    let (panel, mut view) = console();
    // `1 2 3` — the first pane's second thing (its one node group, the deck
    // head being the first) and that group's third control.
    walk_to(&mut view, &panel, "inspector", &[1, 2, 3]);
    assert_eq!(at(&view, "inspector"), vec![1, 2, 3]);
    // A node group's controls are its authority chip, its renderer chips and
    // then its parameter rows, so `3` is the first parameter.
    assert_eq!(
        press(&mut view, &panel, Press::Arrow(Arrow::Up)),
        Asked::Emitted(Operation::WriteParam {
            deck: 0,
            param: ParamAt {
                node: Some(NodeAddress {
                    layer: Layer::L1,
                    index: 0
                }),
                key: "radius".to_owned(),
            },
            // 2.0 of `[0, 4]` is halfway, and a tenth of the published range
            // up is 2.4.
            value: ParamValue::Scalar(2.4),
        }),
        "`1 2 3 ↑` did not write the first parameter of the first pane's node a tenth up"
    );
    // And a fourth rung reaches nothing: a parameter's rows are its own.
    let said = press(&mut view, &panel, Press::Digit(1));
    assert!(matches!(said, Asked::Nothing(_)), "{said:?}");
}

// ---------------------------------------------------------------------------
// The arrows, and the axis
// ---------------------------------------------------------------------------

/// The mixer's strips are a row and the library's rows are a column, so each
/// bay answers one pair and declines the other — which is ADR-0259's *the
/// neighbour of the addressed thing, along the axis it is drawn on*.
#[test]
fn the_arrows_walk_a_bays_items_along_the_axis_it_draws_them_on() {
    let (panel, mut view) = console();
    focus_on(&mut view, &panel, "mixer");
    assert_eq!(
        press(&mut view, &panel, Press::Arrow(Arrow::Right)),
        Asked::Emitted(Operation::SelectDeck { deck: 1 }),
        "`→` did not walk the strips"
    );
    let up = press(&mut view, &panel, Press::Arrow(Arrow::Up));
    assert!(
        matches!(up, Asked::Nothing(_)),
        "`↑` walked a row of strips: {up:?}"
    );

    let (panel, mut view) = console();
    focus_on(&mut view, &panel, "library");
    assert_eq!(
        press(&mut view, &panel, Press::Arrow(Arrow::Down)),
        Asked::Moved
    );
    assert_eq!(view.cursor_row(), 1, "`↓` did not walk the library's rows");
    let right = press(&mut view, &panel, Press::Arrow(Arrow::Right));
    assert!(
        matches!(right, Asked::Nothing(_)),
        "`→` walked a column of rows: {right:?}"
    );
}

/// The one bay that uses both axes, which is the check ADR-0333 left with
/// nothing holding it: *"which arrow pair a badge names is not checked … the
/// axis is `Built::across` and nothing holds the page against it"*.
///
/// The Sequencer's lanes are a column and a lane's cells are a row, so `↑↓`
/// walk the lanes and `←→` are refused there, while a cell says the opposite —
/// and the sentence each refusal carries names the axis that does work.
#[test]
fn the_sequencers_lanes_are_a_column_and_its_cells_are_a_row() {
    let sequencer = focus::built("sequencer").expect("the sequencer's grammar");
    assert!(
        !sequencer.across,
        "the Sequencer's lanes are stacked, so `↑↓` walk them"
    );
    assert_eq!(
        Control::Step.answers(),
        focus::Answers::Cells { across: true },
        "a lane's cells are a row, so `←→` walk them — the one control on the panel whose axis \
         is not its bay's"
    );

    let (panel, mut view) = console();
    focus_on(&mut view, &panel, "sequencer");
    let across = press(&mut view, &panel, Press::Arrow(Arrow::Right));
    match across {
        Asked::Nothing(why) => assert!(
            why.contains("column") && why.contains("up and down"),
            "`→` on the Sequencer's lanes declined without naming the axis that works: {why}"
        ),
        other => panic!("`→` walked a column of lanes: {other:?}"),
    }
    assert_eq!(
        press(&mut view, &panel, Press::Arrow(Arrow::Down)),
        Asked::Moved
    );

    // And on a cell the two swap over: `↑↓` are the ones with nothing to do.
    walk_to(&mut view, &panel, "sequencer", &[1, 2]);
    let down = press(&mut view, &panel, Press::Arrow(Arrow::Down));
    match down {
        Asked::Nothing(why) => assert!(
            why.contains("row") && why.contains("left and right"),
            "`↓` on a lane's cell declined without naming the axis that works: {why}"
        ),
        other => panic!("`↓` walked a row of cells: {other:?}"),
    }
}

/// The walk starts from the item the bay remembers, at bay level, which is what
/// keeps the two keys the Library already had (ADR-0333).
#[test]
fn the_arrows_walk_from_the_remembered_item_without_a_digit_first() {
    let (panel, mut view) = console();
    focus_on(&mut view, &panel, "library");
    assert_eq!(at(&view, "library"), Vec::<usize>::new());
    press(&mut view, &panel, Press::Arrow(Arrow::Down));
    press(&mut view, &panel, Press::Arrow(Arrow::Down));
    assert_eq!(
        view.cursor_row(),
        2,
        "two presses of `↓` with the address at the bay did not walk two rows. Before the \
         grammar these were `up` and `down` and needed nothing pressed first; a walk that \
         declined until a digit had been pressed would take that away"
    );
    assert_eq!(
        at(&view, "library"),
        Vec::<usize>::new(),
        "walking at bay level descended. The arrows move along a level and never into one"
    );
}

/// A walk is not a cycle, which is `View::walk`'s rule arriving at the row of
/// strips: a key held down must not jump the length of it.
#[test]
fn the_strips_are_walked_and_clamped_rather_than_wrapped() {
    let (panel, mut view) = console();
    focus_on(&mut view, &panel, "mixer");
    for _ in 0..8 {
        press(&mut view, &panel, Press::Arrow(Arrow::Right));
    }
    assert_eq!(
        view.selection(),
        3,
        "the walk did not stop at the last strip"
    );
    for _ in 0..8 {
        press(&mut view, &panel, Press::Arrow(Arrow::Left));
    }
    assert_eq!(view.selection(), 0, "the walk did not stop at the first");
}

/// A level takes the arrows and a state does not. A closed list has no axis, so
/// the arrows decline on the three chips and say why.
#[test]
fn the_arrows_step_a_level_and_decline_on_a_state() {
    let (panel, mut view) = console();
    focus_on(&mut view, &panel, "mixer");
    press(&mut view, &panel, Press::Digit(1));
    // The trim is a strip's second control, the tally its first.
    press(&mut view, &panel, Press::Digit(2));
    assert_eq!(
        press(&mut view, &panel, Press::Arrow(Arrow::Down)),
        Asked::Stepped {
            level: Level::Trim(0),
            step: Step::Down
        }
    );
    assert!(view.focus().address("mixer").map(|a| a.at()) == Some(&[1, 2][..]));

    // Back up to the strip and onto the tally, which is a state.
    view.focus_up(&panel);
    press(&mut view, &panel, Press::Digit(1));
    let refused = press(&mut view, &panel, Press::Arrow(Arrow::Up));
    assert!(
        matches!(refused, Asked::Nothing(_)),
        "an arrow stepped a residency, whose three values are a closed list rather than a \
         continuum: {refused:?}"
    );
}

/// The four levels the world holds are the host's, and the console says which
/// one and which way rather than reading it — ADR-0333's seam, met in the three
/// bays ADR-0343 adds.
#[test]
fn the_levels_the_world_holds_are_named_and_not_read() {
    let cases: [(&str, &[usize], Level); 3] = [
        ("master", &[1], Level::Out),
        ("transport", &[10], Level::Exposure),
        ("transport", &[3], Level::Offset),
    ];
    for (bay, path, level) in cases {
        let (panel, mut view) = console();
        walk_to(&mut view, &panel, bay, path);
        assert_eq!(
            press(&mut view, &panel, Press::Arrow(Arrow::Up)),
            Asked::Stepped {
                level,
                step: Step::Up
            },
            "`{path:?} ↑` in `{bay}` did not ask the host to step {level:?}"
        );
        assert_eq!(
            press(&mut view, &panel, Press::Space),
            Asked::Stepped {
                level,
                step: Step::Default
            },
            "`space` on {level:?} is not the value it was declared at"
        );
    }
}

/// A level with nothing behind it says so rather than asking the host to step a
/// value no session is holding — P-0083, on the one control of the three that
/// can be absent while its bay is drawn.
#[test]
fn the_offset_says_so_with_no_session_open() {
    let (panel, mut view) = console();
    view.tracker = None;
    walk_to(&mut view, &panel, "transport", &[3]);
    match press(&mut view, &panel, Press::Arrow(Arrow::Up)) {
        Asked::Nothing(why) => assert!(
            why.contains("audio-in"),
            "an offset with no session did not name the control that opens one: {why}"
        ),
        other => panic!("an offset with no session behind it was stepped: {other:?}"),
    }
}

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

/// On a level the one state worth naming is the value it was declared at, which
/// is the clause ADR-0259 buys with an argument rather than finds.
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

    // **And a bay nobody folded wears none**, whatever else is folded: the bit
    // this reads is the operator's own fold and not *is this drawn*.
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

/// Every refusal carries the sentence that says why, which is P-0083 and the
/// page's own rule that a key that declines and a key that is not bound are the
/// same experience.
///
/// The four preview cells are ADR-0259's clearest case of an item with neither
/// a state nor an act: a digit lands, the ring is drawn, and both keys decline.
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
    // And the three effects of the master chain, which ADR-0259 calls the one
    // place a bay is drawn and its operations are not sayable.
    let (panel, mut view) = console();
    walk_to(&mut view, &panel, "master", &[2]);
    match press(&mut view, &panel, Press::Space) {
        Asked::Nothing(why) => assert!(
            why.contains("spelling"),
            "an effect declined without saying its parameters have no spelling: {why}"
        ),
        other => panic!("an effect answered space: {other:?}"),
    }
}

/// A control whose press puts a card down declines and says so, rather than
/// opening something the address cannot then walk.
#[test]
fn a_control_that_opens_a_card_declines_and_names_what_is_in_it() {
    let cases: [(&str, &[usize], &str); 3] = [
        ("transport", &[7], "input"),
        ("transport", &[8], "save"),
        ("sequencer", &[HEAD, 6], "drive"),
    ];
    for (bay, path, word) in cases {
        let (panel, mut view) = console();
        walk_to(&mut view, &panel, bay, path);
        match press(&mut view, &panel, Press::Enter) {
            Asked::Nothing(why) => assert!(
                why.contains("card") && why.contains(word),
                "`{path:?} enter` in `{bay}` declined without saying what the card holds: {why}"
            ),
            other => panic!("`{path:?} enter` in `{bay}` opened a card: {other:?}"),
        }
    }
}

/// An address on something the bay has stopped drawing goes back to the bay,
/// rather than acting on whatever has taken that position — which is
/// `View::point_at`'s rule about a row past the listing, one level up.
#[test]
fn an_address_on_something_that_is_gone_goes_back_to_the_bay() {
    let (panel, mut view) = console();
    focus_on(&mut view, &panel, "mixer");
    press(&mut view, &panel, Press::Digit(4));
    press(&mut view, &panel, Press::Digit(3));
    assert_eq!(at(&view, "mixer"), vec![4, 3]);

    view.mixer.truncate(2);
    let said = press(&mut view, &panel, Press::Space);
    assert!(
        matches!(said, Asked::Nothing(_)),
        "a press on an address the bay has stopped drawing acted on something: {said:?}"
    );
    assert_eq!(
        at(&view, "mixer"),
        Vec::<usize>::new(),
        "the address stayed on a strip that is gone"
    );

    // **And the same one rung up**, where the path names an item and no control
    // under it: a strip that has gone is a strip that has gone at either depth,
    // and clamping onto the nearest one would put a press on a deck nobody
    // addressed.
    let (panel, mut view) = console();
    focus_on(&mut view, &panel, "mixer");
    press(&mut view, &panel, Press::Digit(4));
    assert_eq!(at(&view, "mixer"), vec![4]);
    view.mixer.truncate(2);
    let said = press(&mut view, &panel, Press::Digit(1));
    assert!(
        matches!(said, Asked::Nothing(_)),
        "a press on an item the bay has stopped drawing acted on something: {said:?}"
    );
    assert_eq!(at(&view, "mixer"), Vec::<usize>::new());
}

/// The floor under the whole file: a `Control` list that had gone empty would
/// satisfy most of it by refusing everything.
#[test]
fn the_bays_are_made_of_what_the_record_walks() {
    let mixer = focus::built("mixer").expect("the mixer's grammar");
    assert_eq!(
        mixer.item().first,
        &[
            Control::Tally,
            Control::Trim,
            Control::Fader,
            Control::Blend,
            Control::Mask
        ],
        "a strip's controls are not the five the bay draws, in the order it draws them"
    );
    assert_eq!(
        mixer.head,
        &[
            Control::Shape,
            Control::Quantum,
            Control::Length,
            Control::Go
        ],
        "the Mixer's head is the transition row: the settings are about the bay rather than \
         about any one strip, which is what a head is (ADR-0343)"
    );
    assert!(mixer.selects && mixer.across);

    let library = focus::built("library").expect("the library's grammar");
    assert_eq!(library.head, &[Control::Scope]);
    assert_eq!(library.item().first, &[Control::Star, Control::Params]);
    assert!(!library.selects && !library.across);
    assert!(library.act.is_some(), "a library row is an act");

    let inspector = focus::built("inspector").expect("the inspector's grammar");
    assert_eq!(inspector.item().first, &[Control::DeckHead]);
    assert_eq!(inspector.item().then, Some(Control::Node));
    assert_eq!(
        inspector.beneath(Control::DeckHead).first,
        &[Control::Sync, Control::Anchor, Control::Composite]
    );
    assert_eq!(inspector.beneath(Control::Node).then, Some(Control::Param));

    // `Undecided` is what a scope step carries, and it is named here so that a
    // payload that grew a value fails against the page rather than silently.
    assert_eq!(
        Operation::SelectScope { scope: Undecided },
        Operation::SelectScope { scope: Undecided }
    );
    // And the scrub is a quarter beat, which is the deck head's own arrows and
    // the amount `↑↓` on an anchor ask for.
    assert_eq!(SCRUB_BEATS, 0.25);
}
