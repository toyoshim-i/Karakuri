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
        /// The controls the mock draws in this head, and only the ones that are
        /// controls. Every other pill in the mock's bay heads states a value the
        /// console does not have yet — `1920x1080`, `previews 3 of 4`, `3 of 3 - page
        /// 1`, `2 waiting` — and a pill reading `previews 3 of 4` over an empty bay is
        /// exactly the scaffolding that looks finished. They arrive with the bay that
        /// knows the number.
        pills: &'static [&'static str],
        /// Whether the mock draws a `.grip` in this head. Four of the eight bays carry
        /// one, and it marks the bay that absorbs its column's height. Stated rather
        /// than derived: `program` carries a grip and is
        /// [`karakuri_layout::Sizing::Fixed`], so the two do not agree and the mock is
        /// the reference.
        grip: bool,
    },
    /// The transport row: a strip with no heading (ADR-0159), holding the tempo,
    /// the beat grid, the bar and the frame readout.
    ///
    /// A kind of its own for the reason [`Kind::Outputs`] is one, and it was
    /// [`Row`](Kind::Outputs) — the abstract *headless strip* — while its body was
    /// empty. Now that both rows have something in them, a `Row` would mean *the
    /// transport* to [`View::draw`] and nothing in the table would say so; the
    /// alternative is comparing a name on the frame path, which puts a string where
    /// the table already says what a region is.
    ///
    /// See [`transport`] for what is drawn here, for where the values come from,
    /// and for the six things in the mock's row that are not drawn.
    Transport,
    /// The Outputs row, which is a headless strip like [`Kind::Transport`] with the
    /// console's one control in it.
    ///
    /// A row in every other respect — it carries `class="bay"` for the card and no
    /// `.bay-head`, which is ADR-0159 — and it is a kind of its own for the reason
    /// [`Kind::Picture`] is one: [`View::draw`] has to know *which* row the sinks
    /// go in, and the alternative is comparing a name on the frame path, which puts
    /// a string where the table already says what a region is.
    ///
    /// See [`outputs`] for what is drawn here, for the state the dot is read from,
    /// and for the four things in the mock's row that are not drawn.
    Outputs,
    /// The Mixer bay, which is a bay in every other respect: the same card and the
    /// same [`bay_head`], carrying [`mixer::MIXER_TITLE`], no pill and no grip —
    /// the mock gives the mixer none of the three.
    ///
    /// A kind of its own for the reason [`Kind::Picture`] is one, and it is the
    /// first *bay* to need it: [`View::draw`] has to know which bay the strips go
    /// in, and the alternative is comparing a name on the frame path, which puts a
    /// string where the table already says what a region is.
    ///
    /// See [`mixer`] for what is drawn here, for where the values come from, and
    /// for the four things in the mock's bay that are not drawn.
    Mixer,
    /// The Library bay, which is a bay in every other respect: the same card and
    /// the same [`bay_head`], carrying [`library::LIBRARY_TITLE`], no pill and the
    /// grip the mock draws in this head.
    ///
    /// A kind of its own for [`Kind::Mixer`]'s reason: [`View::draw`] has to know
    /// which bay the listing goes in, and the alternative is comparing a name on
    /// the frame path, which puts a string where the table already says what a
    /// region is.
    ///
    /// See [`library`] for what is drawn here, for where the values come from, and
    /// for the six things in the mock's bay that are not drawn.
    Library,
    /// The Master bay, which is a bay in every other respect: the same card and the
    /// same [`bay_head`], carrying [`master::MASTER_TITLE`], no pill and the grip
    /// the mock draws in this head.
    ///
    /// A kind of its own for [`Kind::Mixer`]'s reason: [`View::draw`] has to know
    /// which bay the out row goes in, and the alternative is comparing a name on
    /// the frame path, which puts a string where the table already says what a
    /// region is.
    ///
    /// See [`master`] for what is drawn here, for where the level comes from, and
    /// for why the three effects the mock draws under the row are not drawn: they
    /// exist nowhere in this workspace, and a chain over machinery that is not
    /// there is the scaffolding this module refuses.
    Master,
    /// The Staging lane, which is a bay in every other respect: the same card and
    /// the same [`bay_head`], carrying [`staging::STAGING_TITLE`], no pill and no
    /// grip — the mock gives this head a count and the console draws no readout in
    /// a bay head.
    ///
    /// A kind of its own for [`Kind::Mixer`]'s reason: [`View::draw`] has to know
    /// which bay the candidate rows go in, and the alternative is comparing a name
    /// on the frame path, which puts a string where the table already says what a
    /// region is.
    ///
    /// See [`staging`] for what is drawn here, for where the rows come from, and
    /// for the six things in the mock's lane and the page's row that are not drawn.
    Staging,
    /// The Sequencer bay, which is a bay in every other respect: the same card and
    /// the same [`bay_head`], carrying [`sequencer::SEQUENCER_TITLE`], no pill and
    /// no grip — the mock gives this head three bank pills and the console draws
    /// none of them yet.
    ///
    /// A kind of its own for [`Kind::Mixer`]'s reason: [`View::draw`] has to know
    /// which bay the ruler, the rows and the playhead go in, and the alternative is
    /// comparing a name on the frame path, which puts a string where the table
    /// already says what a region is.
    ///
    /// See [`sequencer`] for what is drawn here, for where the pattern comes from,
    /// and for the three things in the mock's bay that are not drawn.
    Sequencer,
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

/// The word on the Program bay's one pill, in the mock's own spelling.
///
/// A constant rather than a literal in two places: [`REGIONS`] puts it in that
/// bay's head and [`program_head`] finds it there again, and a pill nobody can
/// find is a control that silently stops existing.
pub const SOLO_PILL: &str = "solo";

/// Every region the console draws, and what it draws there.
///
/// The table is the whole of what is region-specific. A node the arrangement
/// names and this does not list is structure — `left-pane`, `centre`,
/// `right-pane` are splits an operator folds, not things with a face — and is
/// not drawn. `tests/view.rs` asserts in both directions: every visible leaf of
/// the arrangement is in the plan, and nothing is in the plan that is not a
/// region.
///
/// The order is the arrangement's, top to bottom and left to right, so this
/// reads like the panel.
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
        // A kind of its own since the lane got rows, exactly as the Library
        // is: `View::draw` has to know which bay a candidate goes in. **The
        // mock's `2 waiting` is still not drawn** — a bay head's pills are its
        // controls, and the module documentation is where that is argued,
        // omission by omission.
        kind: Kind::Staging,
    },
    Region {
        name: "program",
        kind: Kind::Bay {
            title: "Program",
            // `solo` is an operation the panel already has — `Op::Solo` over
            // the picture — so it is a control and not a readout. It is the
            // only pill in the mock's heads that is, it is drawn, and a press
            // on it now performs the operation: [`program_head`] is where the
            // capsule and what it asks for both come from.
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
