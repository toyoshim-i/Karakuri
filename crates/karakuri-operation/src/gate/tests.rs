use super::*;
use crate::{
    Authority, BeatSource, BlendMode, ChainParam, Control, Curve, Cut, GridScale, LaneTarget,
    Layer, NodeAddress, Operation, Output, ParamAt, ParamValue, Property, Recording, Residency,
    SetTransfer, StepMode, Sync, Tonemap, TransitionSetting, Undecided, WipeKind,
};

fn node() -> NodeAddress {
    NodeAddress {
        layer: Layer::L4,
        index: 0,
    }
}

fn param() -> ParamAt {
    ParamAt {
        node: Some(node()),
        key: "radius".into(),
    }
}

/// The same parameter, as an attachment addresses one — see [`crate::BindAt`].
fn bind_at() -> crate::BindAt {
    crate::BindAt {
        layer: Layer::L1,
        index: Some(0),
        key: "radius".into(),
    }
}

/// One of every operation in the vocabulary, in the manual's order.
///
/// It is checked against [`Operation::TITLES`] rather than counted, so an
/// operation added to the vocabulary and forgotten here is a failing test
/// rather than a fixture that is quietly one short — which is what would let
/// the split assertions below go on passing over a row nobody classed.
fn every_operation() -> Vec<Operation> {
    vec![
        Operation::TapBeat,
        Operation::ScaleGrid {
            by: GridScale::Halve,
        },
        Operation::SetLatencyOffset { ms: 5.0 },
        Operation::SetSync {
            deck: 0,
            sync: Sync::Beat,
        },
        Operation::ScrubDeck {
            deck: 0,
            beats: 0.25,
        },
        Operation::SetFreeRunTempo { bpm: 120.0 },
        Operation::AttachBeatSource {
            source: BeatSource::AudioInput("default".into()),
        },
        Operation::SelectDeck { deck: 0 },
        Operation::SetResidency {
            deck: 0,
            residency: Residency::Live,
        },
        Operation::LoadSet {
            deck: 0,
            set: "a".into(),
        },
        Operation::SetCompositing {
            deck: 0,
            compositing: true,
        },
        Operation::SetGain { deck: 0, gain: 1.0 },
        Operation::SetOpacity {
            deck: 0,
            opacity: 1.0,
        },
        Operation::SetMute {
            deck: 0,
            mute: false,
        },
        Operation::SetSolo {
            deck: 0,
            solo: false,
        },
        Operation::ClearSolo,
        Operation::SetOnline {
            deck: 0,
            online: true,
        },
        Operation::SetBlendMode {
            deck: 0,
            blend: BlendMode::Add,
        },
        Operation::FadeDeck { deck: 0, to: 0.0 },
        Operation::Crossfade { from: 0, to: 1 },
        Operation::Wipe { from: 0, to: 1 },
        Operation::SetMaskShape {
            deck: 0,
            kind: WipeKind::Linear,
            angle: 0.0,
        },
        Operation::SetMaskPosition {
            deck: 0,
            position: 0.5,
        },
        Operation::SetTransition {
            setting: TransitionSetting::Quantum { beats: 4.0 },
        },
        Operation::SelectRenderer {
            deck: 0,
            renderer: 0,
        },
        Operation::SetMasterOut { out: 1.0 },
        Operation::SetChainParam {
            at: 0,
            param: ChainParam::Cut(Cut::Exit),
        },
        Operation::AddChainEffect {
            procedure: String::new(),
            cut: None,
        },
        Operation::RemoveChainEffect { at: 0 },
        Operation::SetTonemap {
            tonemap: Tonemap::Aces,
        },
        Operation::SetExposure { exposure: 1.0 },
        Operation::SetStep {
            pattern: 0,
            lane: 0,
            step: 0,
            on: true,
        },
        Operation::SetLaneMute {
            pattern: 0,
            lane: 0,
            muted: true,
        },
        Operation::PointLane {
            pattern: 0,
            target: LaneTarget::Fader { deck: 0 },
        },
        Operation::RemoveLane {
            pattern: 0,
            lane: 0,
        },
        Operation::SetPatternGrid {
            pattern: 0,
            grid: StepMode::Sixteenth,
        },
        Operation::SelectPattern { pattern: 0 },
        Operation::WriteParam {
            deck: 0,
            param: param(),
            value: ParamValue::Scalar(1.0),
        },
        Operation::AttachSignal {
            deck: 0,
            param: bind_at(),
            signal: "rms".into(),
            curve: Curve::Lin,
            range: [0.0, 1.0],
        },
        Operation::TakeParamBack {
            deck: 0,
            param: bind_at(),
        },
        Operation::WireInput {
            deck: 0,
            node: "warp".into(),
            slot: "shape".into(),
            to: "blob".into(),
        },
        Operation::Publish {
            deck: 0,
            controls: vec![Control {
                name: "level".into(),
                node: Some(node()),
                key: "exposure".into(),
                range: [0.0, 1.0],
            }],
        },
        Operation::SetProperty {
            deck: 0,
            property: Property::Seed { salt: 1 },
        },
        Operation::SetAuthority {
            deck: 0,
            node: node(),
            authority: Authority::Manual,
        },
        Operation::KeepProcedure {
            deck: 0,
            node: node(),
            id: None,
        },
        Operation::PointPane {
            pane: "inspector-1".into(),
            deck: 0,
        },
        Operation::SaveSet { deck: 0, id: None },
        Operation::ListSets {
            holds: None,
            layer: None,
        },
        Operation::FilterLibrary {
            kinds: crate::LibraryKinds::EVERYTHING,
        },
        Operation::SelectScope { scope: Undecided },
        Operation::SetFavourite {
            id: "a".into(),
            favourite: true,
        },
        Operation::ReadSet { id: "a".into() },
        Operation::TransferSet {
            transfer: SetTransfer::Send { id: "a".into() },
        },
        Operation::WalkHistory {
            set: Some("a".into()),
        },
        Operation::LoadProcedure {
            deck: 0,
            procedure: "orbit_wide".into(),
        },
        Operation::ReadProcedure {
            deck: 0,
            node: node(),
        },
        Operation::WriteProcedure {
            deck: 0,
            node: node(),
            source: "proc p { kind L4 }".into(),
        },
        Operation::WatchFiles {
            watching: Undecided,
        },
        Operation::SwapOutcome,
        Operation::KeepCandidate {
            deck: 0,
            node: node(),
        },
        Operation::RestoreProcedure {
            deck: 0,
            revision: crate::Revision::Previous(node()),
        },
        Operation::MoveBoundary {
            boundary: Undecided,
        },
        Operation::FoldBay {
            bay: "mixer".into(),
        },
        Operation::FoldPane {
            pane: "inspector-1".into(),
        },
        Operation::Unfold { region: None },
        Operation::Solo { region: None },
        Operation::ResetArrangement,
        Operation::SaveArrangement { name: "a".into() },
        Operation::RestoreArrangement { name: "a".into() },
        Operation::SizeWindow {
            width: 1280,
            height: 720,
        },
        Operation::RouteFrame {
            output: Output::Program,
            on: true,
        },
        Operation::RecordSession {
            recording: Recording::Stop,
        },
        Operation::Quit,
    ]
}

/// No deck is live, which is the reading that puts `LoadSet` on the open side.
/// The fixture's decks are all 0, so this is a list that does not hold it
/// rather than an empty one — an empty list would pass against a `contains`
/// that had been inverted.
const NOTHING_LIVE: &[u8] = &[7];
const DECK_0_IS_LIVE: &[u8] = &[0];

/// The fixture is the vocabulary, and nothing here is asserted about a shorter
/// list than the manual specifies.
#[test]
fn the_fixture_holds_one_of_every_operation() {
    let titles: Vec<&str> = every_operation().iter().map(Operation::title).collect();
    assert_eq!(
        titles,
        Operation::TITLES.to_vec(),
        "the fixture and the vocabulary have come apart — every assertion below is over \
             whichever rows the fixture happens to hold"
    );
}

/// A sixty-fifth operation cannot be added without somebody saying which class
/// it is in.
///
/// The property itself is the compiler's: [`standing`] is a `match` over
/// `Operation` with no wildcard arm, so a new variant is a `non-exhaustive
/// patterns` error and this crate does not build. A test cannot assert that — a
/// test only runs on a build that succeeded, so a green suite is evidence of
/// nothing here. What a test can do is hold the *shape* the compiler needs: the
/// day somebody silences that error with `_ => Standing::Open` the build goes
/// green again and the mechanism is gone silently, and this is what makes that
/// loud.
///
/// It reads this file's own text, which is
/// `karakuri::key_column::the_keys_this_file_lists_are_the_keys_the_window_loop_binds`'s
/// method and its reason: the thing being checked is the source, so the source
/// is what is read.
#[test]
fn a_wildcard_arm_would_end_the_exhaustiveness() {
    let source = include_str!("rules.rs");
    let from = source
        .find("pub fn standing(")
        .expect("`standing` is in this file");
    let to = source[from..]
        .find("\n#[cfg(test)]")
        .map(|at| from + at)
        .unwrap_or(source.len());
    for line in source[from..to].lines() {
        let line = line.trim_start();
        assert!(
            !(line.starts_with("_ =>") || line.starts_with("_ if")),
            "`standing` has a wildcard arm: `{line}`. The classification is exhaustive \
                 over the vocabulary on purpose — a wildcard is how a sixty-fourth operation \
                 gets a class nobody chose"
        );
    }
}

/// 42 closed, 27 open, 69 total — ADR-0235's count less the one row ADR-0240
/// retired, plus the one ADR-0299 added, plus ADR-0338's four, plus *Remove a
/// lane* on the closed side with the sequencer's other five. It is still the
/// one number that says the classification was applied to the whole vocabulary
/// rather than to the rows somebody remembered: the record read 41, 23, 64,
/// *Choose what the output shows* leaving the vocabulary took one off the
/// closed side and off the total, *Star a Set, or take the star off* put one
/// back on the open side and on the total, and ADR-0338 adds *Load a procedure
/// over a layer* to the closed side and *Filter the library by kind*, *Keep a
/// node's procedure* and *Point an Inspector pane at a deck* to the open one.
///
/// Counted with the deck the two loads name live, because that is how the
/// record counts it: `LoadSet`'s row is listed under *what a live deck is
/// drawing* and the closed count includes it, and `LoadProcedure` is the same
/// predicate over the same slot. Those two are the only rows whose standing is
/// not a function of the operation alone, so the split is a split *given a
/// reading* — and the reading that makes it 41 is the one the class was drawn
/// for. With nothing live it is 40 and 29, which is the same classification and
/// not a second one.
#[test]
fn the_classification_is_the_split_adr_0235_states() {
    let running = Running::live(DECK_0_IS_LIVE);
    let mut open = 0;
    let mut closed = 0;
    for operation in every_operation() {
        match standing(&operation, running) {
            Standing::Open => open += 1,
            Standing::Closed(_) | Standing::ClosedUnclassed(_) | Standing::Unread(_) => closed += 1,
        }
    }
    assert_eq!((closed, open, closed + open), (46, 27, 73));
}

fn members(class: Class) -> Vec<&'static str> {
    every_operation()
        .iter()
        .filter(|operation| {
            standing(operation, Running::live(DECK_0_IS_LIVE)) == Standing::Closed(class)
        })
        .map(|operation| operation.title())
        .collect()
}

/// What a deck that is live is drawing is closed until the Program bay opens
/// it, and the two loads are in it only while the deck they name is live. *Load
/// a procedure over a layer* is last because it is in *The library* section of
/// the page and this list is in the page's order.
#[test]
fn what_a_live_deck_is_drawing_is_closed_until_the_program_bay_opens_it() {
    assert_eq!(
        members(Class::LiveDeck),
        vec![
            "Put a deck on air, prime it, or take it off",
            "Load material into a deck",
            "Composite a deck's renderers",
            "Choose which renderer of a deck is live",
            "Write a parameter",
            "Attach a signal to a parameter",
            "Take a parameter back",
            "Narrow the published interface",
            "Element capacity, seeds, the camera",
            "Load a procedure over a layer",
        ]
    );
    assert_eq!(Class::LiveDeck.bay(), "Program");
}

/// The mix faders are closed until the Mixer bay opens them, and the master out
/// is filed with them rather than with the master effects (ADR-0224: it is a
/// level and not an effect).
#[test]
fn the_mix_faders_are_closed_until_the_mixer_bay_opens_them() {
    assert_eq!(
        members(Class::MixFaders),
        vec![
            "Gain",
            "Opacity",
            "Mute a deck",
            "Solo a deck",
            "Clear solo",
            "Set a slot's online state",
            "Blend mode",
            "Fade a deck out or in",
            "Crossfade to the next deck",
            "Wipe the next deck in",
            "Set a deck's mask shape",
            "Set a deck's mask position",
            "Master out",
        ]
    );
    assert_eq!(Class::MixFaders.bay(), "Mixer");
}

/// The master effects are closed until the Master bay opens them — the three
/// that are the chain itself, and the two at the far end of it.
#[test]
fn the_master_effects_are_closed_until_the_master_bay_opens_them() {
    assert_eq!(
        members(Class::MasterEffects),
        vec![
            "Set a chain effect's parameter",
            "Add an effect to the master chain",
            "Remove an effect from the master chain",
            "Tone map",
            "Exposure"
        ]
    );
    assert_eq!(Class::MasterEffects.bay(), "Master");
}

/// Inputs and outputs are closed until the Outputs bay opens them — the show's
/// plumbing rather than its picture, which is why they are easy to forget.
#[test]
fn inputs_and_outputs_are_closed_until_the_outputs_bay_opens_them() {
    assert_eq!(
        members(Class::InputsAndOutputs),
        vec![
            "Attach a beat source",
            "Choose where the frame goes",
            "Record the session",
        ]
    );
    assert_eq!(Class::InputsAndOutputs.bay(), "Outputs");
}

/// Closed by default, all four classes, and it is the type's own `Default`
/// rather than a value a caller chose: there is no way to write down an `Open`
/// that starts open.
#[test]
fn closed_by_default_is_the_types_own_default() {
    assert_eq!(Open::default(), Open::CLOSED);
    for class in Class::ALL {
        assert!(!Open::CLOSED.holds(*class), "{} starts open", class.title());
    }
}

/// Opening one class opens no other, which is the whole of what an opening per
/// class means.
#[test]
fn opening_one_class_opens_no_other() {
    for class in Class::ALL {
        let open = Open::CLOSED.with(*class, true);
        for other in Class::ALL {
            assert_eq!(
                open.holds(*other),
                other == class,
                "opening {} changed {}",
                class.title(),
                other.title()
            );
        }
        assert_eq!(open.with(*class, false), Open::CLOSED);
    }
}

/// A closed operation is refused in the one sentence, asserted by The one class
/// whose pill is not in a bay head says so, because the Outputs row has none —
/// a refusal that sent an operator looking for a head would be worse than one
/// that named no place at all, since a model repeats it to the person sitting
/// there.
#[test]
fn the_class_with_no_bay_head_does_not_send_anyone_looking_for_one() {
    let operation = Operation::RecordSession {
        recording: crate::Recording::Stop,
    };
    let refused = audit(&operation, Open::CLOSED, Running::live(NOTHING_LIVE))
        .expect_err("the show's plumbing is closed by default");
    assert!(
        refused.ends_with("the Outputs row, which has no head."),
        "the refusal for a class drawn in a headless row still points at a head: {refused}"
    );
    for class in [Class::LiveDeck, Class::MixFaders, Class::MasterEffects] {
        assert!(
            class.opened_at().starts_with("the head of the"),
            "{class:?} is drawn in a bay with a head and its refusal no longer says so"
        );
    }
}

/// equality against [`refusal`] rather than by a `contains` — P-0090.
#[test]
fn a_closed_operation_is_refused_in_the_one_sentence() {
    let operation = Operation::SetOpacity {
        deck: 2,
        opacity: 0.0,
    };
    let refused = audit(&operation, Open::CLOSED, Running::live(NOTHING_LIVE))
        .expect_err("the mix faders are closed by default");
    assert_eq!(
        refused,
        "`Opacity` is in the class the mix faders, which is closed by default — the \
             operator opens it at the head of the Mixer bay."
    );
    assert_eq!(
        Some(refused),
        refusal(&operation, Standing::Closed(Class::MixFaders))
    );
}

/// A refusal names the operation, its class and where the class is opened —
/// P-0083, in the register it applies to a tool surface: a model told only *no*
/// reports the instrument as incapable rather than as closed.
#[test]
fn a_refusal_names_the_operation_its_class_and_the_bay_that_opens_it() {
    for operation in every_operation() {
        let Standing::Closed(class) = standing(&operation, Running::live(NOTHING_LIVE)) else {
            continue;
        };
        let said = refusal(&operation, Standing::Closed(class)).expect("a closed row refuses");
        assert!(said.contains(operation.title()), "{said}");
        assert!(said.contains(class.title()), "{said}");
        assert!(said.contains(class.bay()), "{said}");
        assert!(said.contains("the operator opens it"), "{said}");
    }
}

/// An open operation is untouched by the gate, with every class closed — which
/// is the state a run starts in. `write_procedure` is the case the classes are
/// drawn to keep open: it rewrites what a live deck is drawing and its worst
/// case is the picture it replaced, coming back.
#[test]
fn an_open_operation_is_untouched_by_the_gate() {
    for operation in every_operation() {
        if standing(&operation, Running::live(NOTHING_LIVE)) != Standing::Open {
            continue;
        }
        let allowed = audit(&operation, Open::CLOSED, Running::live(NOTHING_LIVE))
            .unwrap_or_else(|refused| panic!("{refused}"));
        assert_eq!(allowed.operation(), &operation);
        assert_eq!(refusal(&operation, Standing::Open), None);
    }
}

/// The seven tools that exist stay open, which is ADR-0235's promise that
/// nothing closes on the day it is recorded. Held here over the operations;
/// `karakuri_environment::mcp` holds it over the wire.
#[test]
fn the_seven_tools_that_exist_are_all_on_the_open_side() {
    for operation in [
        Operation::ReadProcedure {
            deck: 0,
            node: node(),
        },
        Operation::WriteProcedure {
            deck: 0,
            node: node(),
            source: String::new(),
        },
        Operation::WireInput {
            deck: 0,
            node: "a".into(),
            slot: "b".into(),
            to: "c".into(),
        },
        Operation::SwapOutcome,
        Operation::ReadSet { id: "a".into() },
        Operation::ListSets {
            holds: None,
            layer: None,
        },
        Operation::SaveSet { deck: 0, id: None },
    ] {
        assert_eq!(
            standing(&operation, Running::unread()),
            Standing::Open,
            "`{}` is one of the seven tools that exist and ADR-0235 closes none of them",
            operation.title()
        );
    }
}

/// Opening a class lets its operations through and no others.
#[test]
fn an_opened_class_lets_its_own_operations_through_and_no_others() {
    let open = Open::CLOSED.with(Class::MixFaders, true);
    assert!(audit(
        &Operation::SetGain { deck: 0, gain: 0.0 },
        open,
        Running::live(NOTHING_LIVE)
    )
    .is_ok());
    assert!(
        audit(
            &Operation::SetExposure { exposure: 0.0 },
            open,
            Running::live(NOTHING_LIVE)
        )
        .is_err(),
        "opening the mix faders opened the master effects"
    );
}

/// Loading a set into a deck that is not live touches nothing on air, so it is
/// open; into one that is, it replaces the picture, so it is closed — and the
/// refusal names the deck and says it is live, or the model that got it cannot
/// act on it.
#[test]
fn loading_a_set_is_closed_only_where_the_deck_it_names_is_live() {
    let operation = Operation::LoadSet {
        deck: 0,
        set: "a".into(),
    };
    assert_eq!(
        standing(&operation, Running::live(NOTHING_LIVE)),
        Standing::Open
    );
    assert_eq!(
        standing(&operation, Running::live(DECK_0_IS_LIVE)),
        Standing::Closed(Class::LiveDeck)
    );
    assert_eq!(
        audit(&operation, Open::CLOSED, Running::live(DECK_0_IS_LIVE)).expect_err("live"),
        "`Load material into a deck` (deck 0 is live) is in the class what a deck that is \
             live is drawing, which is closed by default — the operator opens it at the head \
             of the Program bay."
    );
}

/// A reading nobody took closes the row it decides, and the refusal says which
/// reading was missing rather than saying *closed* — a diagnostic says why, not
/// only what.
#[test]
fn a_reading_nobody_took_closes_the_row_it_decides() {
    let operation = Operation::LoadSet {
        deck: 0,
        set: "a".into(),
    };
    assert_eq!(
        standing(&operation, Running::unread()),
        Standing::Unread(Reading::Live)
    );
    assert_eq!(
        audit(
            &operation,
            Open::CLOSED.with(Class::LiveDeck, true),
            Running::unread()
        )
        .expect_err("an unread reading is not opened by opening the class"),
        "`Load material into a deck` is closed by default where the deck it names is \
             live, and which decks are live was not read."
    );
}

/// A group ADR-0235 closes past its four classes is refused and nothing opens
/// it, and the refusal says that rather than naming a bay that does not exist.
#[test]
fn a_group_with_no_bay_is_refused_and_no_opening_reaches_it() {
    let every = Open::CLOSED
        .with(Class::LiveDeck, true)
        .with(Class::MixFaders, true)
        .with(Class::MasterEffects, true)
        .with(Class::InputsAndOutputs, true);
    for (operation, group) in [
        (Operation::TapBeat, Unclassed::Clock),
        (Operation::Quit, Unclassed::Quitting),
        (Operation::SelectDeck { deck: 0 }, Unclassed::Selection),
        (
            Operation::PointLane {
                pattern: 0,
                target: LaneTarget::Fader { deck: 0 },
            },
            Unclassed::Lanes,
        ),
        (
            Operation::SetAuthority {
                deck: 0,
                node: node(),
                authority: Authority::Automatic,
            },
            Unclassed::Authority,
        ),
    ] {
        assert_eq!(
            standing(&operation, Running::live(NOTHING_LIVE)),
            Standing::ClosedUnclassed(group)
        );
        let refused = audit(&operation, every, Running::live(NOTHING_LIVE))
            .expect_err("no opening reaches a group with no bay");
        assert_eq!(
            refused,
            format!(
                "`{}` is closed by default with {}, and no bay opens that yet — which \
                     class it belongs to is not settled.",
                operation.title(),
                group.title()
            )
        );
    }
}
