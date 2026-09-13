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
    /// The one region a texture is drawn into: the picture in the Program bay,
    /// which is a sink and whose texels somebody else rendered.
    ///
    /// A pane in every other respect — it is inside the Program bay's card and has
    /// no head of its own — and it is a kind of its own for one reason:
    /// [`View::draw`] has to know *which* pane the picture goes in, and the
    /// alternative is comparing a name in the frame path, which puts a string where
    /// the table already says what a region is.
    ///
    /// It carries no label, and the manual says why: *"The picture carries no label
    /// of its own. The bay head already says Program, and this is a region somebody
    /// may be capturing: a capture that is neither the canvas nor a clean crop of
    /// it is worse than useless, and a word burnt into the corner is exactly
    /// that."*
    ///
    /// A clean crop is what the picture now is. [`picture_rect`] gives it the
    /// canvas's own aspect rather than the whole region, so the rectangle somebody
    /// captures is the canvas's shape and the sentence above is satisfied rather
    /// than merely quoted — before that rule the region was neither the canvas nor
    /// a crop of it, and the manual asked for one of the two.
    ///
    /// # What is beside it is the bay's card, and the cells may be in it
    ///
    /// The region is wider than the picture at every window above the mock's
    /// narrowest, and the leftover is the console's ground rather than the engine's
    /// black inside the texture. The bay's card shows through exactly as it does in
    /// every other empty body, which is the rule this module opens with and is also
    /// the mock's own answer — `.program-view` *is* the picture, and what surrounds
    /// it is `.program-body`, which sets no background of its own.
    ///
    /// Far enough above it, the four deck previews are what is in that ground —
    /// [`program_bay`], and ADR-0182 for why it is the ground down each side that
    /// they take. What is drawn beside the picture is therefore either nothing or
    /// the cells, and never a placeholder; the paragraph below is about the case
    /// where it is nothing, which is every window a capture is likely to be taken
    /// at.
    ///
    /// What that costs a capture, said here rather than found later. The `solo`
    /// pill's tooltip is *"Solo the program view: the panel folds away and only the
    /// picture is left, which is also how you capture this window"* — so after a
    /// solo the window is this region, and an operator whose window is not the
    /// canvas's shape captures `--c-panel` bars where black would read as an
    /// ordinary letterbox. That is a real cost and it is new: before this rule the
    /// bars were the engine's clear inside the texture, and they were black.
    ///
    /// `karakuri-cli`'s `a` — `Live::snap_to_canvas` — is the answer to exactly
    /// this one window along, and its own documentation says why: *"an OBS window
    /// capture of it would otherwise pick up the bars and a scale."* The console's
    /// window has no `a` yet, and this rule is what creates that gap.
    Picture,
    /// The region the deck previews are in when they are in a region: the row under
    /// the picture, [`DECKS`] cells side by side.
    ///
    /// A pane in every other respect, and it draws nothing, which is the one thing
    /// about it that is worth reading twice. The four cells are the *bay's* body
    /// rather than this region's — beside the picture they are down the sides and
    /// this region is set aside, with no extent and no entry in the plan — so they
    /// are drawn once from [`program_bay`] and never from here. The kind stays
    /// because the arrangement still has the region and the table still has to say
    /// what it is: a row that folds apart from the picture, which is what the
    /// manual promises and what the operator's `f` still acts on.
    ///
    /// Each cell carries a label and the picture does not, and the manual states
    /// both in one breath: *"The picture carries no label of its own … The A–D
    /// under it keep their letters, which are outside anything you would capture
    /// and are the only thing naming a deck."* So the two are consistent rather
    /// than at odds. See [`preview_rects`] and [`View::previews`].
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
