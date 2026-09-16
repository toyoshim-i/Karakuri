use super::descriptor::ControlDescriptor;
use super::id::ControlId;

/// Single declarative registry of interactive control descriptors.
pub const DESCRIPTORS: [ControlDescriptor; 39] = [
    ControlDescriptor {
        id: ControlId::OutputsSink,
        probe_name: ControlId::OutputsSink.probe_name(),
        label: "Outputs row sinks",
        hotkey: None,
        operation_title: Some("Route outputs"),
    },
    ControlDescriptor {
        id: ControlId::AudioIn,
        probe_name: ControlId::AudioIn.probe_name(),
        label: "Audio-in pill",
        hotkey: None,
        operation_title: Some("Attach a beat source"),
    },
    ControlDescriptor {
        id: ControlId::Tracker,
        probe_name: ControlId::Tracker.probe_name(),
        label: "Tracker group",
        hotkey: Some("b"),
        operation_title: Some("Tap the beat"),
    },
    ControlDescriptor {
        id: ControlId::TransportLearn,
        probe_name: ControlId::TransportLearn.probe_name(),
        label: "Transport learn pill",
        hotkey: None,
        operation_title: None,
    },
    ControlDescriptor {
        id: ControlId::TransportMap,
        probe_name: ControlId::TransportMap.probe_name(),
        label: "Transport map pill",
        hotkey: None,
        operation_title: None,
    },
    ControlDescriptor {
        id: ControlId::Arrangement,
        probe_name: ControlId::Arrangement.probe_name(),
        label: "Arrangement pill",
        hotkey: Some("r"),
        operation_title: Some("Reset the arrangement"),
    },
    ControlDescriptor {
        id: ControlId::Look,
        probe_name: ControlId::Look.probe_name(),
        label: "Look group",
        hotkey: None,
        operation_title: Some("Tone map"),
    },
    ControlDescriptor {
        id: ControlId::TransportRec,
        probe_name: ControlId::TransportRec.probe_name(),
        label: "Transport rec pill",
        hotkey: None,
        operation_title: Some("Record session"),
    },
    ControlDescriptor {
        id: ControlId::TransportTempo,
        probe_name: ControlId::TransportTempo.probe_name(),
        label: "Transport tempo figure",
        hotkey: None,
        operation_title: Some("Set the free-run tempo"),
    },
    ControlDescriptor {
        id: ControlId::MixerStrip,
        probe_name: ControlId::MixerStrip.probe_name(),
        label: "Mixer strip",
        hotkey: None,
        operation_title: Some("Select a deck"),
    },
    ControlDescriptor {
        id: ControlId::Transition,
        probe_name: ControlId::Transition.probe_name(),
        label: "Transition row",
        hotkey: Some("space"),
        operation_title: Some("Wipe the next deck in"),
    },
    ControlDescriptor {
        id: ControlId::Master,
        probe_name: ControlId::Master.probe_name(),
        label: "Master bay",
        hotkey: None,
        operation_title: Some("Master out"),
    },
    ControlDescriptor {
        id: ControlId::InspectorPaneName,
        probe_name: ControlId::InspectorPaneName.probe_name(),
        label: "Inspector pane name",
        hotkey: None,
        operation_title: None,
    },
    ControlDescriptor {
        id: ControlId::InspectorPaneKeep,
        probe_name: ControlId::InspectorPaneKeep.probe_name(),
        label: "Inspector pane keep capsule",
        hotkey: Some("k"),
        operation_title: Some("Keep what a deck is playing"),
    },
    ControlDescriptor {
        id: ControlId::InspectorSlotMcp,
        probe_name: ControlId::InspectorSlotMcp.probe_name(),
        label: "Inspector pane slot MCP policy",
        hotkey: None,
        operation_title: None,
    },
    ControlDescriptor {
        id: ControlId::InspectorPaneTarget,
        probe_name: ControlId::InspectorPaneTarget.probe_name(),
        label: "Inspector pane deck pulldown",
        hotkey: None,
        operation_title: Some("Point an Inspector pane at a deck"),
    },
    ControlDescriptor {
        id: ControlId::InspectorDeckHead,
        probe_name: ControlId::InspectorDeckHead.probe_name(),
        label: "Deck head controls",
        hotkey: None,
        operation_title: Some("Set a deck's sync mode"),
    },
    ControlDescriptor {
        id: ControlId::InspectorRenderers,
        probe_name: ControlId::InspectorRenderers.probe_name(),
        label: "Renderer chips",
        hotkey: None,
        operation_title: Some("Choose which renderer of a deck is live"),
    },
    ControlDescriptor {
        id: ControlId::InspectorParam,
        probe_name: ControlId::InspectorParam.probe_name(),
        label: "Parameter row fader",
        hotkey: None,
        operation_title: Some("Write a parameter"),
    },
    ControlDescriptor {
        id: ControlId::InspectorPublish,
        probe_name: ControlId::InspectorPublish.probe_name(),
        label: "Parameter row publish mark",
        hotkey: None,
        operation_title: Some("Narrow the published interface"),
    },
    ControlDescriptor {
        id: ControlId::InspectorUses,
        probe_name: ControlId::InspectorUses.probe_name(),
        label: "Node uses capsule and card",
        hotkey: None,
        operation_title: Some("Wire a procedure's input to a node"),
    },
    ControlDescriptor {
        id: ControlId::InspectorAuthority,
        probe_name: ControlId::InspectorAuthority.probe_name(),
        label: "Node authority chips",
        hotkey: None,
        operation_title: Some("Set a node's authority"),
    },
    ControlDescriptor {
        id: ControlId::InspectorNodeKeep,
        probe_name: ControlId::InspectorNodeKeep.probe_name(),
        label: "Node keep capsule",
        hotkey: None,
        operation_title: Some("Keep a node's procedure"),
    },
    ControlDescriptor {
        id: ControlId::InspectorSensitivity,
        probe_name: ControlId::InspectorSensitivity.probe_name(),
        label: "Sensitivity curve and take back",
        hotkey: None,
        operation_title: Some("Take a parameter back"),
    },
    ControlDescriptor {
        id: ControlId::ProgramSolo,
        probe_name: ControlId::ProgramSolo.probe_name(),
        label: "Program bay solo capsule",
        hotkey: None,
        operation_title: Some("Solo the program view"),
    },
    ControlDescriptor {
        id: ControlId::BayGrip,
        probe_name: ControlId::BayGrip.probe_name(),
        label: "Bay head fold grip",
        hotkey: Some("g"),
        operation_title: Some("Fold a pane away"),
    },
    ControlDescriptor {
        id: ControlId::DeckPreview,
        probe_name: ControlId::DeckPreview.probe_name(),
        label: "Deck preview cells",
        hotkey: None,
        operation_title: Some("Select a deck"),
    },
    ControlDescriptor {
        id: ControlId::LibraryScope,
        probe_name: ControlId::LibraryScope.probe_name(),
        label: "Library scope chips",
        hotkey: None,
        operation_title: Some("Choose which scope the library shows"),
    },
    ControlDescriptor {
        id: ControlId::LibraryFilter,
        probe_name: ControlId::LibraryFilter.probe_name(),
        label: "Library filter fields",
        hotkey: None,
        operation_title: Some("Filter the library"),
    },
    ControlDescriptor {
        id: ControlId::LibraryKinds,
        probe_name: ControlId::LibraryKinds.probe_name(),
        label: "Library kind chips",
        hotkey: None,
        operation_title: Some("Filter the library by kind"),
    },
    ControlDescriptor {
        id: ControlId::LibraryBadges,
        probe_name: ControlId::LibraryBadges.probe_name(),
        label: "Library row badges",
        hotkey: None,
        operation_title: None,
    },
    ControlDescriptor {
        id: ControlId::LibraryParams,
        probe_name: ControlId::LibraryParams.probe_name(),
        label: "Library params chip",
        hotkey: None,
        operation_title: Some("Read what one Set holds and declares"),
    },
    ControlDescriptor {
        id: ControlId::LibraryLoad,
        probe_name: ControlId::LibraryLoad.probe_name(),
        label: "Library load button and deck pulldown",
        hotkey: Some("enter"),
        operation_title: Some("Load material into a deck"),
    },
    ControlDescriptor {
        id: ControlId::LibraryStars,
        probe_name: ControlId::LibraryStars.probe_name(),
        label: "Library row stars",
        hotkey: None,
        operation_title: Some("Star a Set, or take the star off"),
    },
    ControlDescriptor {
        id: ControlId::LibraryList,
        probe_name: ControlId::LibraryList.probe_name(),
        label: "Library list rows",
        hotkey: None,
        operation_title: Some("List what the store holds"),
    },
    ControlDescriptor {
        id: ControlId::ClassPills,
        probe_name: ControlId::ClassPills.probe_name(),
        label: "MCP safety class pills",
        hotkey: None,
        operation_title: None,
    },
    ControlDescriptor {
        id: ControlId::Sequencer,
        probe_name: ControlId::Sequencer.probe_name(),
        label: "Sequencer bay controls, including a lane's minus",
        hotkey: None,
        operation_title: Some("Toggle a step"),
    },
    ControlDescriptor {
        id: ControlId::StagingBack,
        probe_name: ControlId::StagingBack.probe_name(),
        label: "Staging lane back capsules",
        hotkey: None,
        operation_title: Some("Take a candidate back"),
    },
    ControlDescriptor {
        id: ControlId::StagingCandidate,
        probe_name: ControlId::StagingCandidate.probe_name(),
        label: "Staging lane candidate rows",
        hotkey: None,
        operation_title: Some("Promote a candidate"),
    },
];

/// Query helper returning the descriptor for a strongly-typed [`ControlId`].
pub fn descriptor_for(id: ControlId) -> &'static ControlDescriptor {
    &DESCRIPTORS[id.probe_index()]
}

/// Query helper returning the descriptor matching a probe name string, if any.
pub fn descriptor_for_probe(probe_name: &str) -> Option<&'static ControlDescriptor> {
    DESCRIPTORS.iter().find(|d| d.probe_name == probe_name)
}

/// Query helper returning the descriptor matching a keyboard shortcut, if any.
pub fn descriptor_for_hotkey(hotkey: &str) -> Option<&'static ControlDescriptor> {
    DESCRIPTORS.iter().find(|d| d.hotkey == Some(hotkey))
}
