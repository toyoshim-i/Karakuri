#![allow(unused_imports, dead_code)]

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

pub use super::common::*;
pub use karakuri_console::focus::{self, Arrow, Asked, Control, Grammar, Level, Press, Step, HEAD};
pub use karakuri_console::panel::{Op, Panel};
pub use karakuri_console::room::Room;
pub use karakuri_console::view::{
    AddChoice, Ask, AudioAsk, AudioIn, Candidate, Chain, ChainSlot, Look, Mask, Node,
    NodeAuthority, Pane, Param, Renderer, Scope, Sequenced, SlotParam, Stage, Strip, Tally,
    Tracker, View, AUTHORITIES, SCRUB_BEATS, SYNCS,
};
pub use karakuri_operation::LaneTarget;
pub use karakuri_operation::{
    Authority, BeatSource, BlendMode, ChainParam, Curve, Cut, Layer, NodeAddress, Operation,
    ParamAt, ParamValue, Residency, Revision, StepMode, Sync, Tonemap, TransitionSetting,
    Undecided, WipeKind,
};
pub use karakuri_pattern::{Lane, Pattern};

/// The mixer's four strips, at the values this file steps from. Every one of
/// them differs from its neighbours in the three states, so a cycle that
/// answered from the wrong strip comes out wrong rather than right by luck.
pub fn strip(at: usize) -> Strip {
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
        is_muted: false,
        is_soloed: false,
    }
}

/// One parameter row, published at `ord` over `range` and holding `value`.
pub fn param(ord: usize, name: &str, range: [f32; 2], value: f32) -> Param {
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
pub fn node() -> Node {
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
pub fn pane(deck: usize) -> Pane {
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
pub fn pattern() -> Pattern {
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
pub fn console() -> (Panel, View) {
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
    // Two slots of the chain, one of which declares `retains` and so reads a
    // cut, and one of which does not. Both of the cut chip's answers are
    // reachable.
    view.master_chain = Some(Chain {
        slots: vec![
            ChainSlot {
                name: "feedback".to_owned(),
                cut: Some(karakuri_operation::Cut::Mix),
                params: vec![SlotParam {
                    key: "amount".to_owned(),
                    range: [0.0, 0.95],
                    value: 0.2,
                    default: 0.0,
                }],
            },
            ChainSlot {
                name: "bloom".to_owned(),
                cut: None,
                params: vec![SlotParam {
                    key: "amount".to_owned(),
                    range: [0.0, 1.0],
                    value: 0.1,
                    default: 0.0,
                }],
            },
        ],
    });
    // The `+ add` chooser's offer: one procedure that declares `retains` and
    // one that does not, so the cut a slot arrives at is both answers.
    view.chain_add = vec![
        AddChoice {
            procedure: "sha256:feed".to_owned(),
            words: "feedback".to_owned(),
            retains: true,
        },
        AddChoice {
            procedure: "sha256:b100".to_owned(),
            words: "bloom".to_owned(),
            retains: false,
        },
    ];
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
pub fn holding(view: &View, deck: u8) -> Option<focus::Held> {
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
pub fn focus_on(view: &mut View, panel: &Panel, bay: &str) {
    for _ in 0..10 {
        if view.focused(panel).map(|found| found.name) == Some(bay) {
            return;
        }
        view.tab(panel, 1);
    }
    panic!("`Tab` did not reach `{bay}` in one turn of the ring");
}

/// One press, with the deck's own readings behind it.
pub fn press(view: &mut View, panel: &Panel, key: Press) -> Asked {
    let held: Vec<Option<focus::Held>> = (0..4).map(|deck| holding(view, deck)).collect();
    focus::press(view, panel, key, |deck| {
        held.get(usize::from(deck)).copied().flatten()
    })
}

/// A path pressed digit by digit, and the answer to the last press.
pub fn walk_to(view: &mut View, panel: &Panel, bay: &str, path: &[usize]) {
    focus_on(view, panel, bay);
    for digit in path {
        let said = press(view, panel, Press::Digit(*digit));
        assert!(
            !matches!(said, Asked::Nothing(_)),
            "the walk to {path:?} in `{bay}` was refused at `{digit}`: {said:?}"
        );
    }
}

pub fn at(view: &View, bay: &str) -> Vec<usize> {
    view.focus()
        .address(bay)
        .map_or_else(Vec::new, |address| address.at().to_vec())
}

/// A console with a grid running and both of the Transport's cards' contents
/// behind it — two inputs on the machine and two arrangements filed.
pub fn transport() -> (Panel, View) {
    let (panel, mut view) = console();
    view.transport = Some(super::common::mock_transport());
    let mut audio = AudioIn::NONE;
    audio.inputs = vec![
        "Scarlett 2i2".to_owned(),
        "MacBook Pro Microphone".to_owned(),
    ];
    view.audio = Some(audio);
    view.arrangement.filed = vec!["night".to_owned(), "wide".to_owned()];
    (panel, view)
}
