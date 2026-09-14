//! Strongly-typed identifiers for interactive controls in the console.

/// Strongly-typed identifier corresponding to each interactive control probe.
///
/// There are exactly 38 variants, mapped 1-to-1 in probe order to
/// [`crate::input::PROBES`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ControlId {
    OutputsSink,
    AudioIn,
    Tracker,
    TransportLearn,
    TransportMap,
    Arrangement,
    Look,
    TransportRec,
    TransportTempo,
    MixerStrip,
    Transition,
    Master,
    InspectorPaneName,
    InspectorPaneKeep,
    InspectorPaneTarget,
    InspectorDeckHead,
    InspectorRenderers,
    InspectorParam,
    InspectorPublish,
    InspectorUses,
    InspectorAuthority,
    InspectorNodeKeep,
    InspectorSensitivity,
    ProgramSolo,
    BayGrip,
    DeckPreview,
    LibraryScope,
    LibraryFilter,
    LibraryKinds,
    LibraryBadges,
    LibraryParams,
    LibraryLoad,
    LibraryStars,
    LibraryList,
    ClassPills,
    Sequencer,
    StagingBack,
    StagingCandidate,
}

impl ControlId {
    /// All 38 control identifiers in exact probe order.
    pub const ALL: [ControlId; 38] = [
        ControlId::OutputsSink,
        ControlId::AudioIn,
        ControlId::Tracker,
        ControlId::TransportLearn,
        ControlId::TransportMap,
        ControlId::Arrangement,
        ControlId::Look,
        ControlId::TransportRec,
        ControlId::TransportTempo,
        ControlId::MixerStrip,
        ControlId::Transition,
        ControlId::Master,
        ControlId::InspectorPaneName,
        ControlId::InspectorPaneKeep,
        ControlId::InspectorPaneTarget,
        ControlId::InspectorDeckHead,
        ControlId::InspectorRenderers,
        ControlId::InspectorParam,
        ControlId::InspectorPublish,
        ControlId::InspectorUses,
        ControlId::InspectorAuthority,
        ControlId::InspectorNodeKeep,
        ControlId::InspectorSensitivity,
        ControlId::ProgramSolo,
        ControlId::BayGrip,
        ControlId::DeckPreview,
        ControlId::LibraryScope,
        ControlId::LibraryFilter,
        ControlId::LibraryKinds,
        ControlId::LibraryBadges,
        ControlId::LibraryParams,
        ControlId::LibraryLoad,
        ControlId::LibraryStars,
        ControlId::LibraryList,
        ControlId::ClassPills,
        ControlId::Sequencer,
        ControlId::StagingBack,
        ControlId::StagingCandidate,
    ];

    /// The exact probe string corresponding to this control in [`crate::input::PROBES`].
    pub const fn probe_name(self) -> &'static str {
        match self {
            ControlId::OutputsSink => "the Outputs row's sinks",
            ControlId::AudioIn => "the audio-in pill",
            ControlId::Tracker => "the tracker group's three",
            ControlId::TransportLearn => "the transport row's learn pill",
            ControlId::TransportMap => "the transport row's map pill",
            ControlId::Arrangement => "the arrangement pill",
            ControlId::Look => "the look group's two",
            ControlId::TransportRec => "the transport row's rec pill",
            ControlId::TransportTempo => "the transport row's tempo figure",
            ControlId::MixerStrip => "a mixer strip's five",
            ControlId::Transition => "the transition row's four",
            ControlId::Master => "the Master bay's five",
            ControlId::InspectorPaneName => "the Inspector pane heads' name",
            ControlId::InspectorPaneKeep => "the Inspector pane heads' keep",
            ControlId::InspectorPaneTarget => "the Inspector pane heads' deck pulldown",
            ControlId::InspectorDeckHead => "a deck head's seven",
            ControlId::InspectorRenderers => "the renderer chips",
            ControlId::InspectorParam => "a parameter row's fader",
            ControlId::InspectorPublish => "a parameter row's publish mark",
            ControlId::InspectorUses => "a node group's `uses` capsule and its card",
            ControlId::InspectorAuthority => "a node head's three authority chips",
            ControlId::InspectorNodeKeep => "a node head's keep capsule",
            ControlId::InspectorSensitivity => "a sensitivity row's curve and take back",
            ControlId::ProgramSolo => "the Program bay head's solo",
            ControlId::BayGrip => "the grip in a bay head",
            ControlId::DeckPreview => "the deck preview cells",
            ControlId::LibraryScope => "the Library bay's scope chips",
            ControlId::LibraryFilter => "the Library bay's filter fields",
            ControlId::LibraryKinds => "the Library bay's kind chips",
            ControlId::LibraryBadges => "the Library bay's row badges",
            ControlId::LibraryParams => "the params chip in the Library bay's foot",
            ControlId::LibraryLoad => "the Library bay's load button and deck pulldown",
            ControlId::LibraryStars => "the Library bay's stars",
            ControlId::LibraryList => "the Library bay's list",
            ControlId::ClassPills => "the class pills",
            ControlId::Sequencer => {
                "the Sequencer bay's cells, labels, minus glyphs, mode pill, bank pills and + lane"
            }
            ControlId::StagingBack => "the Staging lane's back capsules",
            ControlId::StagingCandidate => "the Staging lane's rows",
        }
    }

    /// The 0-based probe index in [`crate::input::PROBES`].
    pub const fn probe_index(self) -> usize {
        match self {
            ControlId::OutputsSink => 0,
            ControlId::AudioIn => 1,
            ControlId::Tracker => 2,
            ControlId::TransportLearn => 3,
            ControlId::TransportMap => 4,
            ControlId::Arrangement => 5,
            ControlId::Look => 6,
            ControlId::TransportRec => 7,
            ControlId::TransportTempo => 8,
            ControlId::MixerStrip => 9,
            ControlId::Transition => 10,
            ControlId::Master => 11,
            ControlId::InspectorPaneName => 12,
            ControlId::InspectorPaneKeep => 13,
            ControlId::InspectorPaneTarget => 14,
            ControlId::InspectorDeckHead => 15,
            ControlId::InspectorRenderers => 16,
            ControlId::InspectorParam => 17,
            ControlId::InspectorPublish => 18,
            ControlId::InspectorUses => 19,
            ControlId::InspectorAuthority => 20,
            ControlId::InspectorNodeKeep => 21,
            ControlId::InspectorSensitivity => 22,
            ControlId::ProgramSolo => 23,
            ControlId::BayGrip => 24,
            ControlId::DeckPreview => 25,
            ControlId::LibraryScope => 26,
            ControlId::LibraryFilter => 27,
            ControlId::LibraryKinds => 28,
            ControlId::LibraryBadges => 29,
            ControlId::LibraryParams => 30,
            ControlId::LibraryLoad => 31,
            ControlId::LibraryStars => 32,
            ControlId::LibraryList => 33,
            ControlId::ClassPills => 34,
            ControlId::Sequencer => 35,
            ControlId::StagingBack => 36,
            ControlId::StagingCandidate => 37,
        }
    }

    /// Resolve a `ControlId` from its probe index, returning `None` if out of bounds.
    pub fn from_probe_index(idx: usize) -> Option<ControlId> {
        if idx < Self::ALL.len() {
            Some(Self::ALL[idx])
        } else {
            None
        }
    }

    /// Resolve a `ControlId` from its probe name string in [`crate::input::PROBES`].
    pub fn from_probe_name(name: &str) -> Option<ControlId> {
        Self::ALL.iter().copied().find(|id| id.probe_name() == name)
    }
}
