use super::panel_column_common::*;

/// Returns a sample `Operation` instance for the given variant name, used to look up row titles.
pub(crate) fn sample(variant: &str) -> Operation {
    match variant {
        "SetGain" => Operation::SetGain { deck: 0, gain: 0.0 },
        "SetOpacity" => Operation::SetOpacity {
            deck: 0,
            opacity: 0.0,
        },
        "SetBlendMode" => Operation::SetBlendMode {
            deck: 0,
            blend: BlendMode::Over,
        },
        "SetResidency" => Operation::SetResidency {
            deck: 0,
            residency: Residency::Live,
        },
        "SetMaskShape" => Operation::SetMaskShape {
            deck: 0,
            kind: WipeKind::Linear,
            angle: 0.0,
        },
        // Emitted by the transport row's arrangement pill.
        "SaveArrangement" => Operation::SaveArrangement {
            name: String::new(),
        },
        "RestoreArrangement" => Operation::RestoreArrangement {
            name: String::new(),
        },
        // The two the transport row's look controls emit. They are one row of
        // *Mixing and output* each, so unlike the pair above they are **not**
        // in [`ELSEWHERE`] and both directions below judge them.
        "SetTonemap" => Operation::SetTonemap {
            tonemap: karakuri_operation::Tonemap::Aces,
        },
        "SetExposure" => Operation::SetExposure { exposure: 1.0 },
        // Master bay output control; operates on the entire fold without a deck slot (ADR-0224).
        "SetMasterOut" => Operation::SetMasterOut { out: 1.0 },
        // Master bay effect parameters across cut chips and tracks (ADR-0340).
        "SetChainParam" => Operation::SetChainParam {
            at: 0,
            param: karakuri_operation::ChainParam::Cut(karakuri_operation::Cut::Mix),
        },
        // The other two rows of the chain's list: `+ add` and the drop onto
        // the chain both ask for the add, and the `−` at the end of a slot's
        // row asks for the removal. One operation is one row however many
        // controls name it, which is `dedup_by_key`'s case again (ADR-0352).
        "AddChainEffect" => Operation::AddChainEffect {
            procedure: String::new(),
            cut: None,
        },
        "RemoveChainEffect" => Operation::RemoveChainEffect { at: 0 },
        // Inspector deck head sync controls (ADR-0218).
        "SetSync" => Operation::SetSync {
            deck: 0,
            sync: karakuri_operation::Sync::Beat,
        },
        "ScrubDeck" => Operation::ScrubDeck {
            deck: 0,
            beats: 0.25,
        },
        // Deck selection emitted when clicking on an unhandled area of a mixer strip.
        "SelectDeck" => Operation::SelectDeck { deck: 0 },
        // Transport row audio-in selection card for dynamically attaching beat sources.
        "AttachBeatSource" => Operation::AttachBeatSource {
            source: karakuri_operation::BeatSource::AudioInput("default".to_owned()),
        },
        // Mixer bay transition settings (shape, quantum, length) affecting subsequent transitions.
        "SetTransition" => Operation::SetTransition {
            setting: karakuri_operation::TransitionSetting::Quantum { beats: 4.0 },
        },
        // Transition row `go` capsule triggering a wipe between decks (ADR-0213).
        "Wipe" => Operation::Wipe { from: 0, to: 1 },
        // Tracker group controls for tap tempo and grid scaling.
        "TapBeat" => Operation::TapBeat,
        "ScaleGrid" => Operation::ScaleGrid {
            by: karakuri_operation::GridScale::Halve,
        },
        // Tracker group offset track for latency adjustment.
        "SetLatencyOffset" => Operation::SetLatencyOffset { ms: -15.0 },
        // Library bay scope chip selection (ADR-0308).
        "SelectScope" => Operation::SelectScope {
            scope: karakuri_operation::Undecided,
        },
        // Library filter inputs narrowing listed sets.
        "ListSets" => Operation::ListSets {
            holds: None,
            layer: None,
        },
        // Library row favorite toggle star (ADR-0299).
        "SetFavourite" => Operation::SetFavourite {
            id: "night01".to_owned(),
            favourite: true,
        },
        // Library bay `read` chip (ADR-0312).
        "ReadSet" => Operation::ReadSet {
            id: "night01".to_owned(),
        },
        // Drag-and-drop loading of a set from the library onto a mixer strip.
        "LoadSet" => Operation::LoadSet {
            deck: 0,
            set: "night01".to_owned(),
        },
        // History scope chip walking edit history for targeted deck's set (ADR-0308).
        "WalkHistory" => Operation::WalkHistory {
            set: Some("night01".to_owned()),
        },
        // Restoring a revision from history or staging (ADR-0326).
        "RestoreProcedure" => Operation::RestoreProcedure {
            deck: 0,
            revision: karakuri_operation::Revision::Picked(
                "20260908-143052-271_slot0_L4_beat_strokes".to_owned(),
            ),
        },
        // Staging lane row keeping a changed candidate node (ADR-0326).
        "KeepCandidate" => Operation::KeepCandidate {
            deck: 0,
            node: karakuri_operation::NodeAddress {
                layer: karakuri_operation::Layer::L1,
                index: 0,
            },
        },
        // Inspector pane head keep capsule saving set timestamp (ADR-0287).
        "SaveSet" => Operation::SaveSet { deck: 0, id: None },
        // Transport row tempo readout and direct adjustment.
        "SetFreeRunTempo" => Operation::SetFreeRunTempo { bpm: 128.0 },
        // Transport row `rec` pill toggle (ADR-0289).
        "RecordSession" => Operation::RecordSession {
            recording: karakuri_operation::Recording::Stop,
        },
        // Inspector pane deck head fold toggling compositing/layering mode (ADR-0314).
        "SetCompositing" => Operation::SetCompositing {
            deck: 0,
            compositing: true,
        },
        // Inspector node group renderer selection chips.
        "SelectRenderer" => Operation::SelectRenderer {
            deck: 0,
            renderer: 0,
        },
        // Inspector parameter fader setting a scalar or vector parameter (ADR-0268).
        "WriteParam" => Operation::WriteParam {
            deck: 0,
            param: karakuri_operation::ParamAt {
                node: None,
                key: "exposure".to_owned(),
            },
            value: karakuri_operation::ParamValue::Scalar(0.5),
        },
        // Export set transfer via library row menu (ADR-0260, ADR-0311).
        "TransferSet" => Operation::TransferSet {
            transfer: karakuri_operation::SetTransfer::Send {
                id: "night01".to_owned(),
            },
        },
        // Sequencer cell and lane mute toggles addressed by stored pattern slots (ADR-0320).
        "SetStep" => Operation::SetStep {
            pattern: 0,
            lane: 0,
            step: 4,
            on: true,
        },
        "SetLaneMute" => Operation::SetLaneMute {
            pattern: 0,
            lane: 0,
            muted: true,
        },
        // Sequencer pattern grid mode pill (ADR-0306).
        "SetPatternGrid" => Operation::SetPatternGrid {
            pattern: 0,
            grid: karakuri_operation::StepMode::Eighth,
        },
        // Sequencer new lane target assignment.
        "PointLane" => Operation::PointLane {
            pattern: 0,
            target: karakuri_operation::LaneTarget::Fader { deck: 0 },
        },
        // Sequencer remove lane button (ADR-0352).
        "RemoveLane" => Operation::RemoveLane {
            pattern: 0,
            lane: 0,
        },
        // Sequencer bank pill selection (ADR-0320, ADR-0327).
        "SelectPattern" => Operation::SelectPattern { pattern: 0 },
        // Inspector node authority setting (ADR-0319, P-0090).
        "SetAuthority" => Operation::SetAuthority {
            deck: 0,
            node: karakuri_operation::NodeAddress {
                layer: karakuri_operation::Layer::L1,
                index: 0,
            },
            authority: karakuri_operation::Authority::Suggesting,
        },
        // Curve chip attachment mapping signal source, curve, and output range.
        "AttachSignal" => Operation::AttachSignal {
            deck: 0,
            param: bind_at(),
            signal: "energy".to_owned(),
            curve: karakuri_operation::Curve::Sqrt,
            range: [0.1, 2.4],
        },
        // Remove signal attachment (ADR-0319).
        "TakeParamBack" => Operation::TakeParamBack {
            deck: 0,
            param: bind_at(),
        },
        // **The Outputs row's chips**, which name an output by a word from a
        // closed list and say whether it is on (ADR-0324). `Projector(0)` is
        // the one this repository owns; the program view's is the same
        // operation naming a different destination.
        "RouteFrame" => Operation::RouteFrame {
            output: karakuri_operation::Output::Projector(0),
            on: true,
        },
        // Inspector pane deck head capacity/seed chips (ADR-0328) and wiring cards (ADR-0305, ADR-0329).
        "WireInput" => Operation::WireInput {
            deck: 0,
            node: "swirl_warp".to_owned(),
            slot: "far".into(),
            to: "sphere_shell".to_owned(),
        },
        // Parameter publish marker setting the deck's published interface controls.
        "Publish" => Operation::Publish {
            deck: 0,
            controls: vec![karakuri_operation::Control {
                name: "exposure".to_owned(),
                node: None,
                key: "exposure".to_owned(),
                range: [0.5, 2.0],
            }],
        },
        "SetProperty" => Operation::SetProperty {
            deck: 0,
            property: karakuri_operation::Property::Capacity { elements: 65_536 },
        },
        // Node group keep capsule saving procedure source to library (ADR-0128, ADR-0338).
        "KeepProcedure" => Operation::KeepProcedure {
            deck: 0,
            node: karakuri_operation::NodeAddress {
                layer: karakuri_operation::Layer::L1,
                index: 0,
            },
            id: None,
        },
        // Library kind filter chips selecting visible procedure/set types (ADR-0338).
        "FilterLibrary" => Operation::FilterLibrary {
            kinds: karakuri_operation::LibraryKinds {
                l3: true,
                ..karakuri_operation::LibraryKinds::EVERYTHING
            },
        },
        // Procedure row load action (ADR-0338).
        "LoadProcedure" => Operation::LoadProcedure {
            deck: 0,
            procedure: "orbit_wide".to_owned(),
        },
        // Inspector pane deck target selection pulldown (ADR-0338).
        "PointPane" => Operation::PointPane {
            pane: karakuri_console::view::PANE_NAMES[0].to_owned(),
            deck: 0,
        },
        "SetSolo" => Operation::SetSolo {
            deck: 0,
            solo: true,
        },
        "SetMute" => Operation::SetMute {
            deck: 0,
            mute: true,
        },
        other => panic!(
            "`{SRC}` constructs `Operation::{other}` and this file has no value for it — a \
             control started emitting an operation nobody accounted for. Add an arm here, and \
             then decide whether that row's panel badge in {PAGE} is now `has`"
        ),
    }
}

/// The address an attachment carries — a layer, a node of it, and a key.
pub(crate) fn bind_at() -> karakuri_operation::BindAt {
    karakuri_operation::BindAt {
        layer: karakuri_operation::Layer::L1,
        index: Some(0),
        key: "turbulence".to_owned(),
    }
}
