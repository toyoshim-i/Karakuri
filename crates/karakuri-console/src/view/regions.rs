#[allow(unused_imports)]
use super::*;

/// What the console draws in a region — the third thing about a region, after
/// its name and its rectangle, and the only one this module owns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A titled box. The manual's word, sixteen times over, and the mock's
    /// `.bay-head` is the title bar this draws.
    Bay {
        /// The mock's own capitalisation. `.bay-head` upper-cases in CSS, and that is
        /// done at paint time here rather than in the string, so the word a reader
        /// searches for is the word in the source.
        title: &'static str,
        /// Interactive control labels rendered in this bay's header.
        pills: &'static [&'static str],
        /// Indicates if the bay head renders a column height-absorbing grip.
        grip: bool,
    },
    /// Transport row: headless strip (ADR-0159) for tempo, beat grid, bar, and frame readout.
    Transport,
    /// Outputs row: headless strip (ADR-0159) rendering sink status and output controls.
    Outputs,
    /// Mixer bay: channel fader strips, pan, and mute/solo controls.
    Mixer,
    /// Library bay: asset browser and source listing.
    Library,
    /// Master bay: master chain processing and output level controls.
    Master,
    /// Staging bay: candidate operations and transition staging.
    Staging,
    /// Sequencer bay: timeline ruler, lane rows, and playhead.
    Sequencer,
    /// Prompt bay: embedded agent terminal and CLI selection.
    Prompt,
    /// One subdivision of a bay, which has no head of its own because the bay
    /// around it has one. The inspector's two panes.
    Pane,
    /// Texture display target region for rendered output in the Program bay.
    Picture,
    /// Layout region for deck preview cells under or beside the Program picture.
    Previews,
}

/// One region of the console: the name the arrangement knows it by, and what
/// the panel draws there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Region {
    /// The arrangement's name, which is the manual's word for the region and the
    /// name all four surfaces address it by (ADR-0156, ADR-0159).
    pub name: &'static str,
    pub kind: Kind,
}

/// Label for the Program bay's solo pill button.
pub const SOLO_PILL: &str = "solo";

/// Layout regions rendered by the console in panel display order (top-to-bottom, left-to-right).
pub const REGIONS: &[Region] = &[
    Region {
        name: "transport",
        kind: Kind::Transport,
    },
    Region {
        name: "library",
        kind: Kind::Library,
    },
    Region {
        name: "staging",
        // Renders candidate rows; header pills remain strictly interactive controls.
        kind: Kind::Staging,
    },
    Region {
        name: "prompt",
        kind: Kind::Prompt,
    },
    Region {
        name: "program",
        kind: Kind::Bay {
            title: "Program",
            // Interactive solo pill mapped to `program_head` control operations.
            pills: &[SOLO_PILL],
            grip: true,
        },
    },
    Region {
        name: "program-view",
        kind: Kind::Picture,
    },
    Region {
        name: "deck-previews",
        kind: Kind::Previews,
    },
    Region {
        name: "inspector",
        kind: Kind::Bay {
            title: "Inspector",
            pills: &[],
            grip: true,
        },
    },
    Region {
        name: "inspector-1",
        kind: Kind::Pane,
    },
    Region {
        name: "inspector-2",
        kind: Kind::Pane,
    },
    Region {
        name: "mixer",
        kind: Kind::Mixer,
    },
    Region {
        name: "master",
        kind: Kind::Master,
    },
    Region {
        name: "sequencer",
        kind: Kind::Sequencer,
    },
    Region {
        name: "outputs",
        kind: Kind::Outputs,
    },
];

/// The region a name is, or `None` where the arrangement names something this
/// panel does not draw a face for.
pub fn region(name: &str) -> Option<&'static Region> {
    REGIONS.iter().find(|r| r.name == name)
}
