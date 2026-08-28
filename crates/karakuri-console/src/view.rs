//! The console, drawn: every region in its place, with its heading, and
//! nothing inside.
//!
//! This is the half of the crate that knows a toolkit. It reads the solved
//! rectangles out of [`Panel`] and paints them with `egui`; it holds no fact
//! about the arrangement and answers no question [`karakuri_layout::Layout`]
//! can answer.
//!
//! # What is here, and what is deliberately not
//!
//! **Every leaf of the arrangement gets its region, and seven of them get a
//! bay head.** Nothing else. A bay's body is empty and looks it — no
//! placeholder rows, no sample values, no greyed-out control that hints at
//! what will be there. Scaffolding that looks finished does not get replaced,
//! and the next person to open the panel should be in no doubt about what
//! exists.
//!
//! **One body is not empty, and it is not an exception to that.** The Program
//! bay's picture is a [`Kind::Picture`], and what it draws is a texture handed
//! in from outside — real texels off a real device, not a mock-up of some. It
//! draws nothing at all when there is no texture, exactly as every other empty
//! body does. See [`Picture`] and [`picture_rect`].
//!
//! # The deck previews are the one thing drawn with nothing behind it
//!
//! [`View::draw`] paints four cells whether or not a deck is running in any of
//! them, and that is **not** the scaffolding the paragraph above forbids. A greyed-out control is a placeholder for a control that does not
//! exist yet; a preview cell is not a placeholder for anything, it is the
//! region's own face. The mock's `.preview` is a well with a letter in it
//! before it is a picture — three of its four cells hold no texels at all —
//! and the Program bay's head reads *previews 2 of 4*, which is an operator's
//! ordinary choice and not a state waiting to be finished. **An empty cell is
//! what off looks like, not a stand-in for a full one**, and the label says
//! so: `A` where there is a picture and `C · off` where there is not.
//!
//! So the two rules are one rule. Nothing is drawn that claims something
//! exists which does not; a cell that is off exists and says it is off.
//! See [`DECKS`], [`preview_rects`] and [`View::previews`], and
//! [ADR-0170](../../../docs/adr/0170-a-deck-preview-cell-is-drawn-whether-or-not-a-deck-is-behind-it.md)
//! for the alternative that lost and for what would reopen it.
//!
//! # The Program bay's body arranges itself, and that is one derivation
//!
//! The picture and the four cells are **not** where the two regions that hold
//! them are: on a wide, short bay the cells go down the sides of the picture
//! and `deck-previews` is taken out of the layout altogether, which is what
//! leaves the bay's width to the picture. [`program_bay`] is the one place
//! that decides it and [`rearrange`] is what writes the one bit that follows;
//! [`picture_rect`], [`preview_rects`] and [`View::draw`] are readers of that
//! one answer and derive no rectangle of their own.
//!
//! It is why [`Kind::Previews`] draws nothing: the row's region is not where
//! the cells are, and past the crossover it is not even in the plan.
//! [ADR-0182](../../../docs/adr/0182-the-program-bays-body-arranges-itself-for-the-larger-picture.md)
//! is the arrangement and
//! [ADR-0183](../../../docs/adr/0183-a-node-is-out-of-the-layout-for-two-reasons-and-they-are-two-bits.md)
//! is the bit.
//!
//! The transport and the outputs are **rows, not bays**: they carry no
//! heading, because they have none in the mock — both carry `class="bay"`,
//! which is the card styling, and neither carries a `.bay-head`
//! ([ADR-0159](../../../docs/adr/0159-the-consoles-words-are-the-manuals-and-the-middle-one-is-not-a-pane.md)).
//!
//! # The transport row is four readouts and no controls at all
//!
//! [`Kind::Transport`] draws the tempo, the beat grid, the bar and the frame
//! readout — **the four things in the mock's row that are a value somebody
//! measured rather than a control over something that does not exist.** The
//! other six are named in [`transport`], one by one, with what is missing
//! behind each; the paragraph above is the whole of the argument and this row
//! is where it costs the most, because six of the mock's ten items go.
//!
//! What it draws with no engine behind it is **nothing at all** — not a row of
//! zeroes and not a row of dashes, either of which is a reading invented for a
//! console that has none. That is [`View::picture`]'s rule, and the values
//! arrive the same way it does: a plain [`Transport`] written per frame by
//! whoever owns the engine, because `src/` takes no device, no window and no
//! clock (ADR-0156) and this row is made of a clock and an engine.
//!
//! # The Outputs row has one control in it, and it is the console's first
//!
//! [`Kind::Outputs`] draws the word OUTPUTS and **one** `.sink` — the picture,
//! which the manual lists as *program view*. Everything else in the mock's row
//! is a control over something that does not exist, and [`outputs`] names each
//! of them and says why it is not drawn. The dot is lit from
//! [`Layout::visible`] on the picture's node and a press on it asks for
//! [`Op::Fold`] or [`Op::Unfold`] by name, so the control and the keyboard
//! reach one operation between them rather than two states that agree until
//! they do not.
//!
//! It is also the first thing on the panel a press has to *reach*, which is
//! the rule [`crate::input`] was written for and the clearance
//! `tests/outputs.rs` holds.
//!
//! # The Mixer bay is the first bay with something in its body
//!
//! [`Kind::Mixer`] draws the card and the head every other bay gets, and then
//! **as many strips as the deck has** — not four. `Deck::slot_count` is what
//! there is, and a page of this bay has [`DECKS`] tracks whatever that number
//! is, so a track with no strip in it draws nothing at all rather than an
//! empty strip. **That is not the deck previews' rule read backwards.** A
//! preview cell is the region's own face and says `off`; a strip is six
//! readings, and an empty one is six readings nobody took — the row of zeroes
//! ADR-0177 refuses, with a different glyph. See [`mixer`] and [`Strip`], and
//! [`Meter`] for why a meter is not a [`Fader`].
//!
//! **Five things in that bay answer a pointer and the rest are readouts**,
//! and the source says which rather than leaving the next reader to discover
//! it. The two fader knobs are played, and the blend mini, the tally chip and
//! the mask mini each cycle
//! ([ADR-0185](../../../docs/adr/0185-a-fader-translates-a-drag-into-an-operation-and-applies-nothing.md),
//! [ADR-0187](../../../docs/adr/0187-the-blend-mini-cycles-and-a-map-learns-the-three-it-cycles-through.md),
//! [ADR-0195](../../../docs/adr/0195-the-tally-chip-cycles-from-the-request-so-the-parked-case-needs-no-case.md),
//! [ADR-0203](../../../docs/adr/0203-the-mask-chip-carries-the-angle-it-does-not-control.md)):
//! each emits an [`Operation`] and applies nothing, because the value belongs
//! to the engine rather than to the arrangement. The meter and the number are
//! drawn from what the deck says and a press on either reaches nothing — and
//! so does a press on a fader's *track*, off the knob.
//!
//! **The mask mini is the one control that carries a value it never draws.**
//! The chip says *which shape*, and [`Operation::SetMaskShape`] carries a
//! shape **and an angle**; so [`Strip::mask_angle`] is read to build the
//! operation and painted nowhere, which is what stops choosing a shape from
//! straightening a diagonal front ([`Mixer::mask`]).
//!
//! **The tally is the one control that reads two values.** `Deck::residency`
//! is what a slot is doing and `Deck::requested_residency` is what it was asked
//! to do, and while they disagree the chip's word rolls part of the way toward
//! the request and falls back, once a second, and never lands — see
//! [`Strip::pending`], [`roll_at`] and [`tally_into`], and
//! [P-0075](../../../docs/principles/0075-a-pending-transition-shows-where-it-is-where-it-is-going-and-that-it-has-not-arrived.md)
//! for what that has to say. **A press on it cycles from the residency that
//! was *requested*** ([`Mixer::tally`]), which is what makes a parked chip's
//! press the withdrawal of its own prime request with no case in the code for
//! it. It is the panel's first live region and still its only
//! one ([`View::declares`], which answers with the bay's name, what one update
//! of it costs and how stale it may get) — and it declares only while the bay
//! it rolls in is laid out, because a fold takes the chip off the screen and a
//! price paid for what nobody can see is P-0072 broken rather than served.
//!
//! # The bay head is one component with seven call sites
//!
//! [`bay_head`] is written once and seven bays call it: six through
//! [`Kind::Bay`] in [`REGIONS`] and the mixer through [`Kind::Mixer`], which
//! is a bay whose body is not empty. That is this repository's rule about an
//! abstraction needing two call sites, satisfied on the day it is written
//! rather than promised for later — and so is [`fader`], which is written once
//! and drawn twice in every strip.
//!
//! # `.console`'s own 10px of padding is not drawn
//!
//! The mock's console is a card with `padding: 10px`, so the ground shows in a
//! ring around the outermost bays as well as in every divider. The
//! arrangement's root fills the viewport it is given and
//! [`Panel::set_viewport`] puts that viewport at the origin, so the ring would
//! have to be an offset applied to every rectangle on the way out and undone
//! on every pointer coordinate on the way in — a second coordinate space, for
//! ten pixels of margin. The window's edge is the panel's edge here, and the
//! ground shows in the dividers alone.

use std::time::Duration;

use egui::epaint::text::{LayoutJob, TextFormat};
use egui::{Color32, CornerRadius, FontFamily, FontId, Pos2, Rect, Stroke, StrokeKind, Ui};
use karakuri_layout::{Axis, Hit, NodeId};
use karakuri_operation::{BlendMode, Operation, Residency, WipeKind};

use crate::budget::{Declared, PANEL_PASS};
use crate::panel::{unit, Grab, InHand, Knob, Op, Panel, GRAB};
use crate::room::{size, Palette, Room};

/// The whole of a texture, in `egui`'s texture coordinates. The picture fills
/// its rectangle, and that is now one answer rather than two.
///
/// # What changed, and why the argument is not deleted
///
/// This used to read *"the fitting happened when the engine drew into it, and
/// doing it again here would be two answers to how does a 16:9 canvas sit in
/// this box"*. That was true while [`picture_rect`] handed the engine the
/// whole `program-view` region: the region was not the canvas's shape, so
/// `Present::draw` had to choose where the canvas sat inside it, and a second
/// fit here would have chosen differently.
///
/// **The console decides the box now.** [`picture_rect`] gives the picture the
/// canvas's own aspect, so the engine is handed a target it already agrees
/// with and its fit has nothing left to do. The two answers did not become
/// one by one of them being dropped — one of them became the identity.
///
/// # The engine's letterbox stays, and it is not redundant
///
/// [`fitted`] rounds to whole pixels and the caller's own `physical` rounds
/// again to whole texels, so the target is **never exactly the canvas's
/// ratio**: the mock's 466 x 262 is 16:9 to a quarter of a pixel and no
/// closer. `letterbox` is what absorbs that, and after this rule the bars it
/// draws are sub-texel where they used to be the black half of a region.
const WHOLE_TEXTURE: Rect = Rect {
    min: Pos2::new(0.0, 0.0),
    max: Pos2::new(1.0, 1.0),
};

/// **How many deck preview cells there are**, and it is derived rather than
/// picked off the mock.
///
/// `karakuri_engine`'s `deck::MAX_SLOTS` is 4 — *"a deck holds 1 to 4
/// slots"*, asserted in `Deck::new` — so four is the most auditions there can
/// ever be at once, and a fifth cell would be a cell no deck can ever fill.
/// The mock agrees from the other end: `.previews` is
/// `grid-template-columns: repeat(4, 1fr)` with four `.preview` cells in it,
/// and the Program bay's head reads *previews 2 of 4*. Two readings, one
/// number.
///
/// `karakuri-engine` is not a dependency of this crate and is not becoming one
/// for a `usize` — `src/` takes no device, which is the seam ADR-0156 left
/// standing. The number is transcribed with its derivation, the way every
/// number read off the mock is.
pub const DECKS: usize = 4;

/// The letters the mock puts in the four cells, which is how an operator says
/// *which* deck. `A` through `D`, in slot order.
///
/// Public because a harness saying *which deck a fader moved* has to say it in
/// the letters the cells are drawn with, and a second list written out there
/// would be a copy that goes on saying `A B C D` the day this one does not —
/// [ADR-0179](../../../docs/adr/0179-a-transcribed-number-cites-the-rule-it-was-copied-from.md)
/// on a word instead of on a number.
pub const DECK_LETTERS: [&str; DECKS] = ["A", "B", "C", "D"];

/// **How many cells go down one side when they are beside the picture**: half
/// of [`DECKS`], which is two — A and B down the left, C and D down the right.
///
/// **Two columns of two rather than one column of four**, and the arithmetic
/// is worth writing down because it does not point where a reader expects. A
/// column of *n* cells stacked down a body *H* tall is about `H / n` per cell,
/// so it is `(H / n) * 16 / 9` wide, and all four of them cost
/// `(4 / n) * (H / n) * 16 / 9` — **one column of four is the cheapest
/// arrangement and four columns of one is the dearest.** So this is not the
/// choice that leaves the picture the most room; one side of four would leave
/// more.
///
/// It is chosen for the two things that are not width. The picture stays
/// **centred in the bay** with equal ground either side, which is what
/// [`fitted`] gives every other box on this panel; and a cell stays
/// `H / 2` rather than `H / 4` tall, which at the mock's own body is 163
/// against 79 — a preview an operator can read a cut in against one that is a
/// smaller thumbnail than the row it replaced.
const PER_COLUMN: usize = DECKS / 2;

/// Two columns divide [`DECKS`] exactly, and a `DECKS` that stopped being even
/// would put a cell in neither column rather than fail. It is 4 and the
/// engine's `deck::MAX_SLOTS` is why, so this is a compile-time reading of that
/// number and not a runtime check.
const _: () = assert!(DECKS % 2 == 0 && DECKS == PER_COLUMN * 2);

/// What the console draws in a region — the third thing about a region, after
/// its name and its rectangle, and the only one this module owns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A titled box. The manual's word, sixteen times over, and the mock's
    /// `.bay-head` is the title bar this draws.
    Bay {
        /// The mock's own capitalisation. `.bay-head` upper-cases in CSS, and
        /// that is done at paint time here rather than in the string, so the
        /// word a reader searches for is the word in the source.
        title: &'static str,
        /// The controls the mock draws in this head, and **only the ones that
        /// are controls**. Every other pill in the mock's bay heads states a
        /// value the console does not have yet — `1920x1080`, `previews 2 of
        /// 4`, `4 of 4 - page 1`, `2 waiting` — and a pill reading `previews 2
        /// of 4` over an empty bay is exactly the scaffolding that looks
        /// finished. They arrive with the bay that knows the number.
        pills: &'static [&'static str],
        /// Whether the mock draws a `.grip` in this head. Four of the eight
        /// bays carry one, and it marks the bay that absorbs its column's
        /// height. **Stated rather than derived**: `program` carries a grip
        /// and is [`karakuri_layout::Sizing::Fixed`], so the two do not agree
        /// and the mock is the reference.
        grip: bool,
    },
    /// **The transport row**: a strip with no heading (ADR-0159), holding the
    /// tempo, the beat grid, the bar and the frame readout.
    ///
    /// A kind of its own for the reason [`Kind::Outputs`] is one, and it was
    /// [`Row`](Kind::Outputs) — the abstract *headless strip* — while its body
    /// was empty. Now that both rows have something in them, a `Row` would
    /// mean *the transport* to [`View::draw`] and nothing in the table would
    /// say so; the alternative is comparing a name on the frame path, which
    /// puts a string where the table already says what a region is.
    ///
    /// See [`transport`] for what is drawn here, for where the values come
    /// from, and for the six things in the mock's row that are **not** drawn.
    Transport,
    /// **The Outputs row**, which is a headless strip like
    /// [`Kind::Transport`] with the console's one control in it.
    ///
    /// A row in every other respect — it carries `class="bay"` for the card
    /// and no `.bay-head`, which is ADR-0159 — and it is a kind of its own for
    /// the reason [`Kind::Picture`] is one: [`View::draw`] has to know *which*
    /// row the sinks go in, and the alternative is comparing a name on the
    /// frame path, which puts a string where the table already says what a
    /// region is.
    ///
    /// See [`outputs`] for what is drawn here, for the state the dot is read
    /// from, and for the four things in the mock's row that are **not** drawn.
    Outputs,
    /// **The Mixer bay**, which is a bay in every other respect: the same card
    /// and the same [`bay_head`], carrying [`MIXER_TITLE`], no pill and no
    /// grip — the mock gives the mixer none of the three.
    ///
    /// A kind of its own for the reason [`Kind::Picture`] is one, and it is
    /// the first *bay* to need it: [`View::draw`] has to know **which** bay
    /// the strips go in, and the alternative is comparing a name on the frame
    /// path, which puts a string where the table already says what a region
    /// is.
    ///
    /// See [`mixer`] for what is drawn here, for where the values come from,
    /// and for the four things in the mock's bay that are **not** drawn.
    Mixer,
    /// **The Library bay**, which is a bay in every other respect: the same
    /// card and the same [`bay_head`], carrying [`LIBRARY_TITLE`], no pill and
    /// the grip the mock draws in this head.
    ///
    /// A kind of its own for [`Kind::Mixer`]'s reason: [`View::draw`] has to
    /// know **which** bay the listing goes in, and the alternative is
    /// comparing a name on the frame path, which puts a string where the table
    /// already says what a region is.
    ///
    /// See [`library`] for what is drawn here, for where the values come from,
    /// and for the six things in the mock's bay that are **not** drawn.
    Library,
    /// One subdivision of a bay, which has no head of its own because the bay
    /// around it has one. The inspector's two panes.
    Pane,
    /// **The one region a texture is drawn into**: the picture in the Program
    /// bay, which is a sink and whose texels somebody else rendered.
    ///
    /// A pane in every other respect — it is inside the Program bay's card and
    /// has no head of its own — and it is a kind of its own for one reason:
    /// [`View::draw`] has to know *which* pane the picture goes in, and the
    /// alternative is comparing a name in the frame path, which puts a string
    /// where the table already says what a region is.
    ///
    /// **It carries no label**, and the manual says why: *"The picture carries
    /// no label of its own. The bay head already says Program, and this is a
    /// region somebody may be capturing: a capture that is neither the canvas
    /// nor a clean crop of it is worse than useless, and a word burnt into the
    /// corner is exactly that."*
    ///
    /// **A clean crop is what the picture now is.** [`picture_rect`] gives it
    /// the canvas's own aspect rather than the whole region, so the rectangle
    /// somebody captures is the canvas's shape and the sentence above is
    /// satisfied rather than merely quoted — before that rule the region was
    /// neither the canvas nor a crop of it, and the manual asked for one of
    /// the two.
    ///
    /// # What is beside it is the bay's card, and the cells may be in it
    ///
    /// The region is wider than the picture at every window above the mock's
    /// narrowest, and the leftover is the console's ground rather than the
    /// engine's black inside the texture. The bay's card shows through exactly
    /// as it does in every other empty body, which is the rule this module
    /// opens with and is also the mock's own answer — `.program-view` *is* the
    /// picture, and what surrounds it is `.program-body`, which sets no
    /// background of its own.
    ///
    /// **Far enough above it, the four deck previews are what is in that
    /// ground** — [`program_bay`], and ADR-0182 for why it is the ground down
    /// each side that they take. What is drawn beside the picture is therefore
    /// either nothing or the cells, and never a placeholder; the paragraph
    /// below is about the case where it is nothing, which is every window a
    /// capture is likely to be taken at.
    ///
    /// **What that costs a capture, said here rather than found later.** The
    /// `solo` pill's tooltip is *"Solo the program view: the panel folds away
    /// and only the picture is left, which is also how you capture this
    /// window"* — so after a solo the window **is** this region, and an
    /// operator whose window is not the canvas's shape captures `--c-panel`
    /// bars where black would read as an ordinary letterbox. That is a real
    /// cost and it is new: before this rule the bars were the engine's clear
    /// inside the texture, and they were black.
    ///
    /// `karakuri-cli`'s `a` — `Live::snap_to_canvas` — is the answer to
    /// exactly this one window along, and its own documentation says why:
    /// *"an OBS window capture of it would otherwise pick up the bars and a
    /// scale."* **The console's window has no `a` yet, and this rule is what
    /// creates that gap.**
    Picture,
    /// **The region the deck previews are in when they are in a region**: the
    /// row under the picture, [`DECKS`] cells side by side.
    ///
    /// A pane in every other respect, and **it draws nothing**, which is the
    /// one thing about it that is worth reading twice. The four cells are the
    /// *bay's* body rather than this region's — beside the picture they are
    /// down the sides and this region is set aside, with no extent and no
    /// entry in the plan — so they are drawn once from [`program_bay`] and
    /// never from here. The kind stays because the arrangement still has the
    /// region and the table still has to say what it is: a row that folds
    /// apart from the picture, which is what the manual promises and what the
    /// operator's `f` still acts on.
    ///
    /// **Each cell carries a label and the picture does not**, and the manual
    /// states both in one breath: *"The picture carries no label of its own …
    /// The A–D under it keep their letters, which are outside anything you
    /// would capture and are the only thing naming a deck."* So the two are
    /// consistent rather than at odds. See [`preview_rects`] and
    /// [`View::previews`].
    Previews,
}

/// One region of the console: the name the arrangement knows it by, and what
/// the panel draws there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Region {
    /// The arrangement's name, which is the manual's word for the region and
    /// the name all four surfaces address it by (ADR-0156, ADR-0159).
    pub name: &'static str,
    pub kind: Kind,
}

/// Every region the console draws, and what it draws there.
///
/// **The table is the whole of what is region-specific.** A node the
/// arrangement names and this does not list is structure — `left-pane`,
/// `centre`, `right-pane` are splits an operator folds, not things with a face
/// — and is not drawn. `tests/view.rs` asserts in both directions: every
/// visible leaf of the arrangement is in the plan, and nothing is in the plan
/// that is not a region.
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
        kind: Kind::Bay {
            title: "Staging",
            pills: &[],
            grip: false,
        },
    },
    Region {
        name: "program",
        kind: Kind::Bay {
            title: "Program",
            // `solo` is an operation the panel already has — `Op::Solo` over
            // whatever the pointer is on — so it is a control and not a
            // readout. It is drawn, and it is the only pill in the mock's
            // heads that is.
            pills: &["solo"],
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
        kind: Kind::Bay {
            title: "Master",
            pills: &[],
            grip: true,
        },
    },
    Region {
        name: "sequencer",
        kind: Kind::Bay {
            title: "Sequencer",
            pills: &[],
            grip: false,
        },
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

/// One region, where it solved to this frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placed {
    pub id: NodeId,
    pub region: &'static Region,
    pub rect: karakuri_layout::Rect,
}

/// Every region to draw this frame, in tree order, appended to `out` after
/// clearing it.
///
/// **Tree order is load bearing and not a convenience.** The inspector is a
/// split: its bay card and head cover the same rectangle its two panes tile,
/// so the card has to be painted before them. Tree order gives that for
/// nothing, and any other order would need the rule written out.
///
/// **Arranges the Program bay first**, which is [`rearrange`] and is where the
/// solve happens: `Layout::rect` refuses to answer from a dirty layout, and a
/// plan taken before the bay had arranged itself would list `deck-previews` as
/// a region to draw on the very frame its cells went somewhere else. On a
/// frame where nothing moved both solves are flag tests and the bit is written
/// with the value it already had, which marks nothing dirty (ADR-0183).
///
/// `canvas` is the picture's shape, which is what decides that arrangement —
/// [`program_bay`]. It is [`View::canvas`] at the one call site that draws.
pub fn plan_into(panel: &mut Panel, canvas: (u32, u32), out: &mut Vec<Placed>) {
    rearrange(panel, canvas);
    out.clear();
    let layout = panel.layout();
    for node in panel.nodes() {
        if !layout.visible(node.id) {
            continue;
        }
        let Some(region) = layout.name(node.id).and_then(region) else {
            continue;
        };
        out.push(Placed {
            id: node.id,
            region,
            rect: layout.rect(node.id),
        });
    }
}

/// **A picture to draw in the Program bay: a texture somebody else rendered,
/// and where it goes.**
///
/// The id is `egui`'s, which means it has already been registered with an
/// [`egui_wgpu::Renderer`](crate::egui_wgpu::Renderer) — and that registration
/// needs a device, which is exactly what this crate does not have. So the
/// caller does it and hands the result over; this module draws an id and a
/// rectangle and knows nothing about either. It is the same seam the whole
/// crate is built on, one level in: `src/` reads and paints, and everything
/// that takes a device is the example's.
///
/// **The rectangle is passed rather than looked up**, and that is what makes
/// the pair checkable: whoever sized the texture and whoever placed it are the
/// same statement, so a texture sized from the window and drawn into the
/// picture's region cannot be written by accident. [`picture_rect`] is what a
/// caller derives both from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Picture {
    /// The registered texture. Whatever it holds is drawn as-is: the console
    /// tints it with nothing.
    pub id: egui::TextureId,
    /// Where to draw it, in the same logical pixels the arrangement is stated
    /// in — [`picture_rect`]'s answer for the frame this is being drawn on.
    pub rect: Rect,
}

/// **Where the picture goes**: the canvas's own shape, as large as the Program
/// bay's body leaves room for once the four deck previews have their places,
/// centred in what is left — below the bay head painted over the top of the
/// bay and inside `.program-body`'s padding.
///
/// `canvas` is what the Set renders at and what the deck is sized to: `--canvas`
/// in the product, which reaches a replay through `Record::Canvas` so a session
/// renders at the size the performance ran at. **Two dimensions rather than a
/// ratio**, and that is a conclusion rather than a habit. It is the shape
/// `karakuri_engine::present::letterbox(canvas, target)` and `Present::size`
/// already speak, so a caller hands over the number it built its `Present`
/// from rather than deriving a float on the way in — and a ratio derived at a
/// call site is exactly where `9.0 / 16.0` gets written for `16.0 / 9.0`,
/// which is a bug nothing on screen shows. It also carries its own
/// provenance: `(1280, 720)` reads as a canvas and `1.7777778` reads as a
/// number somebody typed. `src/` still takes no engine (ADR-0156) — this is a
/// pair of `u32`s, arriving the way every other value does.
///
/// `None` where there is no picture to draw, which is the manual's *"it is on
/// screen exactly when that sink is on — so there is no state where it is
/// hidden and still costing a pass"*: a caller that renders into this
/// rectangle records no pass at all when there is no rectangle. A folded
/// picture is the case that matters — see *The rectangle is the bay's now*
/// below, and [`program_bay`], which is where it is decided.
///
/// # The insets are the bay's own derivation, read backwards
///
/// The box is the **bay** less [`size::HEAD_H`] and one
/// [`size::PROGRAM_BODY_PAD`] on each of the four sides ([`bay_body`]), and
/// then less whatever arrangement the cells took out of it — a row and a
/// divider along the bottom, or a column and a divider down each side.
///
/// It reaches the mock's own numbers by the same arithmetic it always did.
/// The arrangement gives `program-view` 27 + 9 + 262 at the mock's width — bay
/// head, `.program-body`'s padding above the picture, and the picture itself —
/// and `deck-previews` 63 + 9 under it with the split's 8px divider between,
/// so the body less the row less the divider is that region less the head and
/// the top pad, to the pixel. The 9 under the picture in the CSS is the
/// divider and belongs to neither child; the 9 under the *row* is the body's
/// bottom padding, and it is the body's now rather than the row region's,
/// which is the one term that moved. `tests/program_body.rs`'s
/// `below_is_what_the_console_draws_today` is that agreement as an assertion.
///
/// At the narrowest console the mock will draw, that box is exactly
/// **466 x 262** — and 466 x 262 is 16:9 *to a quarter of a pixel* rather than
/// exactly. `.program-view` carries `aspect-ratio: 16/9`, so 466 wide is
/// **262.125** tall, and the arrangement transcribed the whole pixel the mock
/// rasterises it at. The box is therefore 1.778626 where the canvas is
/// 1.7777778, which is the whole of why the paragraph below exists.
///
/// # The rectangle is a whole number of pixels, and that is about resampling
///
/// A strict fit into that box gives 465.7778 x 262. **A caller's `physical`
/// rounds to whole texels, so the texture it then allocates is 466 wide** —
/// and the picture would be a 466-texel texture drawn into a 465.7778-wide
/// box, where every texel on screen is a fractional sample of its neighbours
/// instead of a blit. Of every region on this panel that is worst here: the
/// picture is a *preview of what is being captured*, and softening it is the
/// one thing it may not do. It also buys nothing — the texture's own ratio is
/// 466:262 either way, because `physical` rounded. **The fractional quarter
/// pixel is not more faithful to 16:9; it is the same texture, softened.**
///
/// Two smaller reasons, and they are second. Every number in this console is a
/// whole logical pixel because the mock is authored in whole ones —
/// [`preview_cells`] comes out 112 x 63 with no rounding at all because 466
/// happens to divide, not because a cell is exempt from this. And a box whose
/// extent is whole is a box the picture's edge lands on the pixel grid in,
/// which is what `.program-view`'s own hard-edged well is drawn as.
///
/// **What it costs, plainly:** the picture's ratio is then the mock's 1.778626
/// rather than exactly the canvas's. `Present::draw` is what absorbs the
/// difference and that is why it stays — see [`WHOLE_TEXTURE`]. Snapping does
/// not make the engine's letterbox redundant; it makes it sub-texel.
///
/// # The leftover is the console's ground, and a capture pays for it
///
/// Above the mock's narrowest the region is wider than the picture, and what
/// is beside the picture is the Program bay's card with nothing drawn on it.
/// [`Kind::Picture`] is where that is argued and where the cost to an operator
/// capturing a soloed window is written out, along with the `a` this console's
/// window has not got yet.
///
/// # The rectangle is the **bay's** now, and not this region's
///
/// It was this region's until the body started arranging itself. Beside the
/// picture the four cells stand in ground the `program-view` region owns — the
/// row is [`Layout::set_aside`](karakuri_layout::Layout::set_aside) there and
/// has no extent at all — so a picture inset out of this region and cells
/// taken off the bay would be two answers to *where does the picture go*, and
/// they would differ by two columns and a divider. **One derivation, and this
/// is one of its readers**: [`program_bay`] arranges the whole body once and
/// this is its picture. See [ADR-0182](../../../docs/adr/0182-the-program-bays-body-arranges-itself-for-the-larger-picture.md).
///
/// **The `None` rule moved with it**, and that is the one line of this that is
/// not the same sentence it was. *A folded region keeps its rectangle and
/// loses its extent* was what said the picture is not on screen, and it does
/// not any more: the box is taken off the **bay**, which keeps its 378
/// whatever the picture does, so a folded picture would be handed a body and
/// fitted into it. [`program_bay`] asks the operator's fold by name and this
/// answers `None` from it — and the size test is still in there underneath,
/// on the fitted rectangle, for a bay with no room in it.
///
/// `layout` must be solved: `Layout::rect` refuses to answer from a dirty one.
pub fn picture_rect(layout: &karakuri_layout::Layout, canvas: (u32, u32)) -> Option<Rect> {
    program_bay(layout, canvas)?.picture
}

/// **Whether there is anything of this rectangle to draw**, which is the one
/// rule [`picture_rect`], [`preview_cells`] and [`program_body`] each answer
/// `None` from.
///
/// It is asked of the *fitted* rectangle rather than of the box it was fitted
/// into, always: a box under half a pixel rounds to nothing, which is nothing
/// to draw and nothing to render into, and a box the inset turned inside out
/// gives a negative extent `egui` draws back-to-front rather than refuses. The
/// three call sites had a copy of this comparison each before they had a
/// function; the sentence is the same one in all three, and now so is the
/// answer. [`kept`] is the same rule as an `Option`, for the callers that hand
/// the rectangle straight back.
fn positive(rect: Rect) -> bool {
    rect.width() > 0.0 && rect.height() > 0.0
}

/// **The largest rectangle of `aspect` that fits inside `inside`, centred in
/// it, and a whole number of pixels in each direction.**
///
/// # One derivation with two call sites, and they were one rule before they
/// were one function
///
/// [`picture_rect`] fits the canvas into what the bay's body leaves it and
/// [`preview_cells`] fits a cell into its track, and
/// [ADR-0170](../../../docs/adr/0170-a-deck-preview-cell-is-drawn-whether-or-not-a-deck-is-behind-it.md)
/// states the second in words the first now takes unchanged: *"as large as the
/// track's width and the row's height both allow, centred"*. Two copies of it
/// is two chances for a picture and the thumbnails under it to disagree about
/// what *centred* means.
///
/// **`aspect` is two numbers and not a ratio**, for the reason
/// [`picture_rect`] gives at length, and it is deliberately **not** called
/// `canvas`: the picture's two numbers are the canvas's and a cell's are
/// [`PREVIEW_ASPECT`]'s, which is the mock's. One argument, two provenances,
/// each stated where it is passed.
///
/// # What is rounded and what is not
///
/// **The extent is rounded and the position is not.** The extent is what a
/// caller's `physical` turns into texels, so it is the half that decides
/// whether the picture is blitted or resampled — [`picture_rect`] is where
/// that argument is written out. The offset is left as the true centre,
/// because rounding it is a cell no longer centred in its track, and *centred*
/// is the other half of the rule ADR-0170 took. A region whose own origin is
/// fractional still samples fractionally, and that is the arrangement's
/// coordinate rather than this rule's to fix.
///
/// The clamp is `floor` rather than the box's own extent so that the answer
/// stays whole: rounding up can exceed the box by up to half a pixel, and
/// clamping to a fractional edge would hand back the fractional extent this
/// exists to avoid.
fn fitted(inside: Rect, aspect: (u32, u32)) -> Rect {
    let (aw, ah) = (aspect.0.max(1) as f32, aspect.1.max(1) as f32);
    let scale = (inside.width() / aw).min(inside.height() / ah);
    let w = (aw * scale).round().min(inside.width().floor());
    let h = (ah * scale).round().min(inside.height().floor());
    Rect::from_min_size(
        Pos2::new(
            inside.min.x + (inside.width() - w) * 0.5,
            inside.min.y + (inside.height() - h) * 0.5,
        ),
        egui::vec2(w, h),
    )
}

/// **The shape of one deck preview cell**, and it is the mock's number rather
/// than the canvas's: `.preview` carries `aspect-ratio: 16/9` in `style.css`,
/// in a `.previews` grid of `repeat(4, 1fr)` with a 6px gap.
///
/// Two numbers rather than a ratio for [`picture_rect`]'s reason, and passed
/// to the same [`fitted`] the picture goes through.
///
/// # It agrees with the canvas today by coincidence, and that is a pass rather
/// than a rename
///
/// An audition is the **same canvas** the picture shows, so a canvas that is
/// not 16:9 would letterbox inside a cell — a second fit inside a rectangle
/// `Present::draw` has already fitted, which is precisely what ADR-0170
/// rejected one level up. The honest number here is therefore the canvas's.
///
/// **The reason it is not the canvas's has changed, and the number has not.**
/// It used to be structural — *"making a cell canvas-aware means putting a
/// canvas on [`View`] and writing it per frame"*, and there was no canvas on
/// [`View`] to read. There is one now ([`View::canvas`]), so that reason is
/// spent; what holds the number is
/// [ADR-0182](../../../docs/adr/0182-the-program-bays-body-arranges-itself-for-the-larger-picture.md)'s
/// own decision instead — a cell is the mock's shape and never the canvas's,
/// asserted by `a_cell_is_the_mocks_shape_and_never_the_canvass` — and making
/// it the canvas's is still a pass of its own, with the second fit above to
/// answer for.
const PREVIEW_ASPECT: (u32, u32) = (16, 9);

/// **What the Program bay arranges its body for when nobody has said what is
/// being rendered**: the mock's own picture, which is `.program-view`'s
/// `aspect-ratio: 16/9` in `style.css`.
///
/// A console with no engine behind it has no canvas — it is `src/` taking no
/// device, one number further on (ADR-0156) — and it still has to put four
/// cells somewhere, so [`View::canvas`] starts here and whoever owns the
/// `Present` writes the real thing over it every frame. It is two numbers
/// rather than a ratio for [`picture_rect`]'s reason, and it is the mock's
/// **picture** rather than [`PREVIEW_ASPECT`]'s cell: the two are the same 16
/// and 9 read off two different rules in the same stylesheet, and a `--canvas`
/// that is not 16:9 moves one of them and not the other.
pub const MOCK_CANVAS: (u32, u32) = (16, 9);

/// **Where the four deck previews go**: a row of [`DECKS`] cells inside the
/// `deck-previews` region, under the picture.
///
/// The mirror of [`picture_rect`] and it carries the same `None` rule for the
/// same reason — a folded row has a rectangle with no extent in it, so the
/// cells come out degenerate and there is nothing to draw or to render into.
/// **A fixed-size array rather than a `Vec`**: there are four decks and there
/// is no fifth, so a caller cannot ask for one and cannot forget one either.
///
/// # The insets are `.program-body`'s, and only three of the four
///
/// The row is the region inset by [`size::PROGRAM_BODY_PAD`] left, right and
/// **bottom**, and by nothing at the top. Read the Program bay's own
/// derivation in `lib.rs`: `deck-previews` is 63 + 9, the row of cells and the
/// padding under them. The 9 *above* the cells in the CSS is not in this
/// region at all — it is the split's 8px divider plus `program-view`'s own
/// bottom, which is why [`picture_rect`] takes nothing off the bottom.
///
/// At the narrowest console the mock will draw, that leaves 484 - 9 - 9 = 466
/// for four tracks and three [`size::PREVIEW_GAP`]s: (466 - 18) / 4 = **112**
/// wide, and 112 at 16:9 is **63** tall, which is exactly the height the row
/// has. The mock's cell, arrived at from the other end.
///
/// # A cell is 16:9 and centred in its track, and the alternative is written
/// down
///
/// The arrangement pins this region at 72 tall (`lib.rs`: fixed 72, minimum
/// 72, because a row of four cells at a fixed type size has nothing in it that
/// gets smaller). So a wider window widens the track and does **not** heighten
/// the row, and past the reference width a cell cannot both fill its track and
/// stay 16:9. One of the two has to give, and it is the track:
///
/// - **Taken:** the cell is 16:9, as large as the track's width and the row's
///   height both allow, and centred in its track. At the reference width that
///   is exactly 112 x 63 and fills the track; wider, it stays 63 tall with
///   ground either side. The texture then fills the cell exactly, so nothing
///   letterboxes twice and the cell is always the shape of what it shows.
///   **[`picture_rect`] now takes that same rule for the picture**, and the
///   two share [`fitted`] rather than stating it twice.
/// - **Rejected:** fill the track and letterbox the texels inside it. That
///   keeps the row looking like a grid at every width, and pays for it by
///   stretching the cell away from the shape of its picture — a 16:9 audition
///   in a 200x63 well, with bars the console has drawn itself inside a
///   rectangle the engine already fitted. Two fits for one question, which is
///   the thing `WHOLE_TEXTURE` refuses one level up.
///
/// # The row is one of two places the cells go, and this asks which
///
/// **It insets the `deck-previews` region no longer**, and it cannot: beside
/// the picture that region has no extent at all — it is
/// [`Layout::set_aside`](karakuri_layout::Layout::set_aside), which is exactly
/// what leaves the bay's width to the picture — so a cell derived from it
/// would be `None` at every window past the crossover, and deck A's audition
/// would stop being sized, stop being drawn and stop keeping the window's loop
/// awake. The cells come through [`program_bay`] like the picture does and
/// like the drawing does: **one derivation, and every reader of it reads the
/// same frame's answer.**
///
/// `canvas` is therefore an argument that was not here before. A cell is the
/// mock's shape and never the canvas's ([`PREVIEW_ASPECT`]), and this still
/// holds — what the canvas decides is *which arrangement*, because the thing
/// being compared is the picture, and the picture is the canvas's shape.
///
/// `layout` must be solved: `Layout::rect` refuses to answer from a dirty one,
/// and it wants [`rearrange`] to have been run on it — see [`program_bay`].
pub fn preview_rects(
    layout: &karakuri_layout::Layout,
    canvas: (u32, u32),
) -> Option<[Rect; DECKS]> {
    program_bay(layout, canvas)?.cells
}

/// The four cells inside a `deck-previews` region, or `None` where there is no
/// room for them.
///
/// **One call site now, and it is the one case where the row is the answer**:
/// [`program_bay`]'s guard branch, where the picture is folded away and the
/// row is the whole of the bay. Everywhere else the cells come off the bay's
/// body through [`program_body`], because everywhere else there is a picture
/// for them to be arranged around — and the two agree to the pixel where they
/// overlap, which is `tests/program_body.rs`'s
/// `below_is_what_the_console_draws_today`.
fn preview_cells(region: Rect) -> Option<[Rect; DECKS]> {
    let pad = size::PROGRAM_BODY_PAD;
    let row = Rect::from_min_max(
        Pos2::new(region.min.x + pad, region.min.y),
        Pos2::new(region.max.x - pad, region.max.y - pad),
    );
    let cells = preview_row(row);
    // The same rule `picture_rect` states, and stated on the cell rather
    // than on the region because the cell is what gets drawn. Every cell is
    // the same size, so the first one answers for all four.
    match positive(cells[0]) {
        true => Some(cells),
        false => None,
    }
}

/// **[`DECKS`] cells side by side across `row`**, each [`PREVIEW_ASPECT`] and
/// centred in its track.
///
/// The row of the mock, with nothing said about where the row is: that is
/// [`preview_cells`]'s inset off the `deck-previews` region, and
/// [`program_body`]'s strip along the bottom of the bay's body. **Two call
/// sites for one row**, and they have to agree exactly — the second one is the
/// same row in the same place, arrived at from the bay rather than from the
/// region, and a second copy of this arithmetic is where the two would drift.
fn preview_row(row: Rect) -> [Rect; DECKS] {
    // [`PREVIEW_ASPECT`], as large as the track and the row both allow, and
    // centred — which is `fitted`, the same call `picture_rect` makes. The
    // aspect passed is the mock's rather than the canvas's, and the constant
    // is where that difference is argued.
    std::array::from_fn(|deck| {
        fitted(
            track(row, DECKS, deck, size::PREVIEW_GAP, Axis::Row),
            PREVIEW_ASPECT,
        )
    })
}

/// **One of `count` equal tracks laid along `axis` inside `strip`**, with `gap`
/// between them and nowhere else.
///
/// That last clause is the whole of it, and it is the reading the mock's grids
/// and flex rows all take: `repeat(4, 1fr)` with a `gap` is four tracks and
/// **three** gaps, not four tracks each carrying one. The same sentence is
/// written on [`size::PREVIEW_GAP`], on [`size::BEAT_GAP`] and on
/// [`size::STRIP_GAP`], and this is the arithmetic all three describe —
/// [`TransportRow::dot`] is the fourth, and it steps a fixed dot width rather
/// than dividing a strip, so it states the rule and does not call this.
///
/// **Three call sites, and the third is what made it worth a function.** The
/// row of previews and the mixer's page of strips were the same six lines
/// written twice with a different gap in them; a column beside the picture is
/// the third, and it is those six lines read one axis along. A track spans
/// `strip` across the axis, exactly as a node of the arrangement spans its
/// parent across its own — which is why the axis is
/// [`karakuri_layout::Axis`] rather than a `bool`.
fn track(strip: Rect, count: usize, index: usize, gap: f32, axis: Axis) -> Rect {
    let gaps = gap * (count.max(1) - 1) as f32;
    let (along, across) = match axis {
        Axis::Row => (strip.width(), strip.height()),
        Axis::Column => (strip.height(), strip.width()),
    };
    let size = (along - gaps) / count.max(1) as f32;
    let at = (size + gap) * index as f32;
    match axis {
        Axis::Row => Rect::from_min_size(
            Pos2::new(strip.min.x + at, strip.min.y),
            egui::vec2(size, across),
        ),
        Axis::Column => Rect::from_min_size(
            Pos2::new(strip.min.x, strip.min.y + at),
            egui::vec2(across, size),
        ),
    }
}

/// **Which way round the Program bay's body is arranged.**
///
/// Not a state and not a setting: [`program_body`] answers it from the
/// rectangle it is given, every time it is asked, and nothing stores it. See
/// that function for the decider and for why it is a decision rather than a
/// preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    /// **The mock's own**: the picture across the top, the four cells in a row
    /// under it. The default, and what a tie gives.
    Below,
    /// The picture in the middle, two cells down the left and two down the
    /// right. What a bay wider than it is tall gets.
    Beside,
}

/// **Where everything in the Program bay's body goes**: the picture, and the
/// four deck preview cells.
///
/// One value rather than two calls, for [`Picture`]'s own reason: whoever
/// placed the picture and whoever placed the cells are then one statement, so
/// a picture drawn for one arrangement and cells drawn for the other cannot be
/// written by accident. That is not a hypothetical here — the two arrangements
/// put the cells in different halves of the bay, so the failure would be four
/// thumbnails over the top of the picture rather than a rectangle a few pixels
/// out.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Body {
    /// Which arrangement won, carried because the caller cannot derive it from
    /// the rectangles without re-running the decider — and re-running it is
    /// the second answer this value exists to prevent.
    pub placement: Placement,
    /// The picture: the canvas's shape, as large as the arrangement leaves
    /// room for, centred in what is left ([`fitted`]).
    pub picture: Rect,
    /// The four cells, in [`DECK_LETTERS`] order — **always four, and always
    /// in that order**. See [`program_body`] for why a cell does not move when
    /// the deck behind it stops.
    pub cells: [Rect; DECKS],
}

/// **How the Program bay's body arranges itself in the rectangle it has**: the
/// picture and the four deck preview cells, either the mock's way or down the
/// sides, whichever leaves the picture larger.
///
/// `body` is the bay **less its head and less `.program-body`'s padding** —
/// [`bay_body`], taken off the bay as a whole because the arrangement below
/// spans both of the bay's regions and the divider between them. `canvas` is the picture's
/// aspect and arrives as two numbers for the reason [`picture_rect`] gives at
/// length.
///
/// `None` where neither arrangement can be drawn — [`positive`]'s rule, asked
/// of the picture and of every cell.
///
/// # Why there is a second arrangement at all
///
/// [ADR-0181](../../../docs/adr/0181-the-picture-is-the-canvass-shape-and-the-leftover-is-the-consoles.md)
/// gave the picture the canvas's shape, and the leftover became the console's
/// ground. On a wide bay that leftover is **ground down each side**: at a
/// 1920-wide window the body is 1396 x 333 and the picture is 466 x 262, so
/// 930 pixels of the bay's width are empty and the four cells are 63 tall in a
/// row under it. Putting the cells in that ground is what lets the picture take
/// the height instead.
///
/// # The decider is the picture's size, and nothing else
///
/// **Whichever arrangement gives the larger picture wins, and a tie goes to
/// [`Placement::Below`]**, which is the mock's. Both are computed and their
/// pictures compared; nothing is stored, nothing is remembered between frames,
/// and the same rectangle always gives the same answer — so
/// [P-0071](../../../docs/principles/0071-solving-a-layout-never-mutates-it.md)
/// is untouched, and so is the property it buys: a window dragged wide and
/// back again comes back to exactly the arrangement it left.
///
/// The alternative is a stored mode — a preference, or a hysteresis band
/// around the crossover — and each of those is a second piece of state about
/// the same question, which is what this repository stops on. What a stateless
/// decider costs is a flip at one width, and the numbers below are what say
/// whether that width is anywhere an operator lives.
///
/// **Three worked cases, and the arithmetic is in `tests/program_body.rs`:**
///
/// - **The mock's narrowest**, 466 x 333. Below gives 466 x 262. Beside cannot
///   be drawn at all: two columns and their gaps come to 597, which is more
///   than the body is wide, so the picture's box is negative. Below wins
///   because it is the only one there is.
/// - **A 1920 window**, 1396 x 333. Below gives 466 x 262 — 122,092 texels.
///   Beside gives **592 x 333**, which is 197,136, and the picture is **61%
///   larger**. Beside wins.
/// - **800 wide**, where below still wins: beside's picture would be 202 x 114.
///   The flip is at **1064**, one pixel of body width, and there is exactly one
///   of them — below's picture stops growing with the width at 466 and beside's
///   never shrinks, so the two curves cross once and never again.
///
/// # A side column is as wide as two stacked cells, which makes it the
/// height's answer and not the width's
///
/// A column holds [`PER_COLUMN`] cells stacked with one [`size::PREVIEW_GAP`]
/// between them, so a cell is `(H - gap) / 2` tall and the column is that at
/// [`PREVIEW_ASPECT`] — **a function of the body's height alone**. Reading it
/// off the width instead is the mistake worth naming: the column would grow
/// with the very width it is competing for, and beside would never win at any
/// width — an arrangement that exists in the source and never on the screen.
///
/// Two things had to be checked about this rule and both hold:
///
/// - **It is not so greedy that beside never wins.** The column does not follow
///   the width, so widening the bay adds the whole increment to the picture's
///   box; below's picture is capped by the height it has *after* the row and
///   the gap come off, and beside's by the whole height. Beside therefore wins
///   at every width past the crossover and the crossover exists at every
///   height — at the mock's 333 it is a body 1064 wide, which is a 1588-wide
///   window, well inside an ordinary desktop.
/// - **It is greedy in the other direction, and that is a real cost rather
///   than a caveat.** The column follows the height, so a bay dragged taller
///   widens both columns while the picture's box is what pays for them: at 1396
///   wide the picture beside peaks at a body 391 tall and shrinks after it,
///   below's grows with every pixel, and the two cross at **427** — a Program
///   bay 472 tall, which an operator can drag to. So this arrangement is the
///   answer for a bay that is **wide and short**, and the cells go back under
///   the picture when it stops being short. It is one flip in each direction
///   and not a flicker — beside's picture is single-peaked in the height and
///   below's is monotone — and `tests/program_body.rs` sweeps both axes rather
///   than taking that on trust. The corner that follows from the same rule:
///   **a bay dragged to its own minimum of 200 goes beside at every width**,
///   the mock's narrowest included, because a body 155 tall has only 84 left
///   for the picture once the row and the divider come off.
/// - **It is not so mean that the cells are unreadable.** A cell beside the
///   picture is `(333 - 6) / 2 = 163` tall against the row's **63**, so the
///   arrangement that takes the cells out of the row makes each of them larger
///   rather than smaller. At the Program bay's own minimum height the body is
///   155 and a cell is still 73, which is more than the mock's row gives at any
///   width at all.
///
/// # Which cell goes where, and why none of them moves
///
/// **A and B down the left, C and D down the right**, each column read top to
/// bottom — the row's own left-to-right order, folded in half, so an operator
/// who knows where `C` was in the row finds it at the top of the other side
/// rather than somewhere new.
///
/// **With fewer than four decks running nothing fills and nothing shifts.**
/// The mock's head reads *previews 2 of 4* and
/// [ADR-0170](../../../docs/adr/0170-a-deck-preview-cell-is-drawn-whether-or-not-a-deck-is-behind-it.md)
/// is the answer: a cell is drawn whether or not a deck is behind it, because
/// *an empty cell is what off looks like, not a stand-in for a full one*. So
/// the question *does the left column fill first, or do they alternate* has a
/// third answer, and it is the one that keeps the letters meaning something:
/// **a cell's place is its deck's, not its turn's.** `C · off` sits at the top
/// of the right column whether or not C is running, and turning B off does not
/// slide C into B's place — the letter is the only thing naming a deck, and a
/// label that moves when a neighbour stops is a label an operator cannot point
/// at. This function is handed no liveness at all, which is that rule as a
/// signature.
///
/// # The gaps are the mock's, and there are two of them rather than three
///
/// - **Between the picture and a column**: `.program-body`'s `gap: 8px`, which
///   is `PROGRAM_DIVIDER` in `lib.rs` and the divider the arrangement already
///   leaves between the picture and the row. CSS's `gap` shorthand sets the row
///   gap and the column gap alike, so the body's own declaration states this
///   number for the across-the-bay direction too; nothing is invented for it.
/// - **Between two stacked cells**: [`size::PREVIEW_GAP`], `.previews`'s
///   `gap: 6px`, for exactly the same reading of the same shorthand — it is the
///   gap between two `.preview` cells, and the mock states one number for both
///   directions.
///
/// There is no third: the columns sit against the body's own edges, which are
/// already `.program-body`'s padding in from the card, and the cells are
/// centred in their tracks by [`fitted`] rather than pushed to an edge.
pub fn program_body(body: Rect, canvas: (u32, u32)) -> Option<Body> {
    // Both, every time, and then one comparison. Two fits and six divisions is
    // not a cost worth a cached answer — and a cached answer is the state the
    // paragraph above refuses.
    match (below(body, canvas), beside(body, canvas)) {
        (Some(below), Some(beside)) if area(beside.picture) > area(below.picture) => Some(beside),
        // The tie, and every case where beside cannot be drawn.
        (Some(below), _) => Some(below),
        (None, beside) => beside,
    }
}

/// How many texels a rectangle is, which is the whole of the decider.
///
/// **Area rather than width or height**, and that is the one of the three that
/// answers the question being asked: the picture is a preview of what is being
/// captured, so what an operator gets more of is pixels. Comparing widths would
/// hand the bay to whichever arrangement is wider at a height where it is also
/// shorter.
fn area(rect: Rect) -> f32 {
    rect.width() * rect.height()
}

/// **The mock's arrangement**: the picture across the top, the four cells in a
/// row along the bottom.
///
/// The row is [`size::PREVIEW_ROW_H`] tall — the 63 the arrangement pins
/// `deck-previews` at, less the padding under it — and it keeps that height at
/// every width, exactly as the arrangement does. The picture takes what is left
/// above it, less one `PROGRAM_DIVIDER`.
fn below(body: Rect, canvas: (u32, u32)) -> Option<Body> {
    let row = Rect::from_min_max(
        Pos2::new(body.min.x, body.max.y - size::PREVIEW_ROW_H),
        body.max,
    );
    let picture = fitted(
        Rect::from_min_max(
            body.min,
            Pos2::new(body.max.x, row.min.y - crate::PROGRAM_DIVIDER),
        ),
        canvas,
    );
    drawable(Placement::Below, picture, preview_row(row))
}

/// **The other arrangement**: two cells down the left, two down the right, and
/// the picture in the middle.
///
/// The column's width is the whole of the rule — see [`program_body`], where it
/// is argued and checked — and everything else here is [`track`] and
/// [`fitted`], the same two calls the row goes through.
fn beside(body: Rect, canvas: (u32, u32)) -> Option<Body> {
    let (aw, ah) = (
        PREVIEW_ASPECT.0.max(1) as f32,
        PREVIEW_ASPECT.1.max(1) as f32,
    );
    // **The body's height, and never its width.** Two cells stacked with one
    // gap between them, and the column is one of them lying at its aspect.
    let cell = (body.height() - size::PREVIEW_GAP * (PER_COLUMN - 1) as f32) / PER_COLUMN as f32;
    let column = cell * aw / ah;
    let picture = fitted(
        Rect::from_min_max(
            Pos2::new(body.min.x + column + crate::PROGRAM_DIVIDER, body.min.y),
            Pos2::new(body.max.x - column - crate::PROGRAM_DIVIDER, body.max.y),
        ),
        canvas,
    );
    let cells = std::array::from_fn(|deck| {
        // A and B down the left, C and D down the right: the deck's index
        // decides which side and which of the two tracks, and nothing else
        // does.
        let side = match deck < PER_COLUMN {
            true => Rect::from_min_max(body.min, Pos2::new(body.min.x + column, body.max.y)),
            false => Rect::from_min_max(Pos2::new(body.max.x - column, body.min.y), body.max),
        };
        fitted(
            track(
                side,
                PER_COLUMN,
                deck % PER_COLUMN,
                size::PREVIEW_GAP,
                Axis::Column,
            ),
            PREVIEW_ASPECT,
        )
    });
    drawable(Placement::Beside, picture, cells)
}

/// One arrangement, or `None` where it cannot be drawn.
///
/// [`positive`]'s rule over the whole arrangement rather than over one
/// rectangle, because the two halves are one answer: a body that holds the
/// picture and has no room for a cell is not this arrangement with a cell
/// missing, it is the other arrangement's turn.
fn drawable(placement: Placement, picture: Rect, cells: [Rect; DECKS]) -> Option<Body> {
    match positive(picture) && cells.iter().copied().all(positive) {
        true => Some(Body {
            placement,
            picture,
            cells,
        }),
        false => None,
    }
}

/// **What the Program bay holds this frame, and where it holds it**: the
/// picture, the four deck preview cells, and which way round the two were
/// arranged.
///
/// Both halves are optional because **the bay's two regions fold apart** —
/// `console.html`: *"The picture is a sink ... The deck previews under it are
/// auditions of their own, so they stay when it goes."* So a bay with a
/// picture and no cells is the operator having folded the row, a bay with
/// cells and no picture is the operator having folded the picture, and
/// [`program_bay`] answers `None` where the bay itself is not on screen. The
/// two are one value for [`Body`]'s own reason: whoever placed the picture and
/// whoever placed the cells have to be one statement, and here they are one
/// statement about one solve as well.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProgramBay {
    /// Which way round the body was arranged. **[`Placement::Below`] whenever
    /// there is no picture**, which is the guard rule below written into the
    /// value rather than left to the caller.
    pub placement: Placement,
    /// Where the picture goes, or `None` where the operator folded it away or
    /// the bay has no room for it — [`picture_rect`]'s answer.
    pub picture: Option<Rect>,
    /// Where the four cells go, in [`DECK_LETTERS`] order, or `None` where the
    /// operator folded the row away — [`preview_rects`]'s answer.
    pub cells: Option<[Rect; DECKS]>,
}

/// **The Program bay, arranged for the rectangle it solved to** — the one
/// derivation of where the picture and the four cells go, and the only thing
/// in this crate that reads the bay's geometry to decide it.
///
/// [`picture_rect`], [`preview_rects`], [`rearrange`] and [`View::draw`] are
/// its four readers and none of them derives a rectangle of its own. That is
/// the rule [`fitted`] and [`preview_row`] already live by, one level up:
/// **the picture and the cells move together or they overlap.**
///
/// # The body is the bay's, and the bay is what does not move
///
/// The rectangle handed to [`program_body`] is the **bay** less the head
/// painted over it and less `.program-body`'s padding — not the `program-view`
/// region, which is what [`picture_rect`] used to inset. It has to be the bay,
/// for the reason ADR-0182 gives (the arrangement below spans both regions and
/// the divider between them) and for one more that only matters here: **the
/// bay's rectangle does not depend on the bit this decision writes.** The bay
/// is `Fixed(378)` over a flexible `program-view`, so what it can use is
/// unbounded whether or not the row is set aside
/// ([ADR-0174](../../../docs/adr/0174-a-node-claims-only-what-its-visible-content-can-use.md)),
/// and the placement is therefore a fixed point after one write rather than
/// something that could chase itself around the solve. [`rearrange`] is where
/// that argument is finished.
///
/// # The guard rule, and it is the console's because the crate may not hold it
///
/// **The body only arranges itself while the picture is there.** A split can
/// use nothing when none of its children is laid out, so setting the row aside
/// while `program-view` is folded would leave the whole bay claiming zero and
/// the Program bay would vanish from the panel — and the manual promises the
/// opposite: the deck previews *"are auditions of their own, so they stay when
/// it goes"*.
/// [ADR-0183](../../../docs/adr/0183-a-node-is-out-of-the-layout-for-two-reasons-and-they-are-two-bits.md)
/// says outright that this rule is the caller's, *"and a crate that does not
/// know what a picture is may not hold it"*. So it is here, and it is
/// structural rather than a check: with the picture folded there is nothing to
/// arrange around, the row is [`Placement::Below`] at its own height in the
/// region the arrangement gave it, and ADR-0174's round trip is exactly what
/// it was before this pass.
///
/// The other fold is the mirror of it and was already true: with the row
/// folded there are no cells to place, so the picture takes the body whole.
///
/// # Which bit is asked, and it is the operator's
///
/// [`is_collapsed`](karakuri_layout::Layout::is_collapsed) for each half and
/// [`visible`](karakuri_layout::Layout::visible) for the bay, which is
/// ADR-0183's two bits read apart on purpose. **Asking `visible` of the row
/// here would be this function reading its own answer back**: the row is set
/// aside exactly when this said *beside*, so the cells would vanish on the
/// frame after they moved. The ancestors are asked once, of the bay, where the
/// second bit cannot be.
///
/// `layout` must be solved, and it wants [`rearrange`] to have been run on it
/// this frame — with a stale bit and a folded picture the row's region is the
/// one place here that reads a rectangle the bit decides.
pub fn program_bay(layout: &karakuri_layout::Layout, canvas: (u32, u32)) -> Option<ProgramBay> {
    let bay = layout.find("program")?;
    // The bay itself, its bay folded around it, or a solo somewhere else: one
    // question for every ancestor, asked where `set_aside` is not in the way.
    if !layout.visible(bay) {
        return None;
    }
    let picture = layout.find("program-view")?;
    let row = layout.find("deck-previews")?;
    let body = bay_body(to_egui(layout.rect(bay)));
    match (!layout.is_collapsed(picture), !layout.is_collapsed(row)) {
        // Both halves on screen, and this is the arrangement ADR-0182 decides.
        (true, true) => program_body(body, canvas).map(|arranged| ProgramBay {
            placement: arranged.placement,
            picture: Some(arranged.picture),
            cells: Some(arranged.cells),
        }),
        // The row is folded: nothing to arrange around, so the picture has the
        // body whole — the same `fitted` the two arrangements end in.
        (true, false) => Some(ProgramBay {
            placement: Placement::Below,
            picture: kept(fitted(body, canvas)),
            cells: None,
        }),
        // **The guard.** The picture is folded, so nothing moves: the row is
        // below at its own height, in the region the arrangement solved for
        // it, which is the rectangle it has had since ADR-0174.
        (false, true) => Some(ProgramBay {
            placement: Placement::Below,
            picture: None,
            cells: preview_cells(to_egui(layout.rect(row))),
        }),
        // Both folded. The bay has a head and no body at all, which is what it
        // had before any of this.
        (false, false) => None,
    }
}

/// **The Program bay's body**: its rectangle less the head painted over the
/// top of it and less `.program-body`'s padding on all four sides.
///
/// The inset [`picture_rect`] used to take off the `program-view` region, off
/// the bay instead — and with the bottom padding taken off, which that one
/// could not: the 9 under the picture belonged to `deck-previews` when the
/// region was the box, and belongs to the body now that the bay is.
fn bay_body(bay: Rect) -> Rect {
    let pad = size::PROGRAM_BODY_PAD;
    Rect::from_min_max(
        Pos2::new(bay.min.x + pad, bay.min.y + size::HEAD_H + pad),
        Pos2::new(bay.max.x - pad, bay.max.y - pad),
    )
}

/// A rectangle, where there is anything of it to draw — [`positive`] as an
/// `Option`, which is the shape all four of its call sites wanted.
fn kept(rect: Rect) -> Option<Rect> {
    match positive(rect) {
        true => Some(rect),
        false => None,
    }
}

/// **Arrange the Program bay for the rectangle it has, and tell the layout
/// what that means for the row**: `deck-previews` is set aside where the cells
/// went beside the picture, and put back where they are under it.
///
/// Returns **whether anything moved**, which is a
/// [`Change::Rearranged`](crate::repaint::Change::Rearranged) and is the whole
/// of what a caller does with it.
///
/// # Where in the frame this goes, and why it is here rather than in the solve
///
/// The bit *has to be computed from a solved layout* — it is a function of the
/// bay's rectangle — and *writing it dirties the layout when it changes*. So
/// the order is forced: **solve, decide, write, solve.**
///
/// - **A frame on which nothing moved does no work.** Both solves are the flag
///   test `Layout::solve` opens with, and the write is
///   [`set_aside`](karakuri_layout::Layout::set_aside) with the value the node
///   already carries, which marks nothing dirty by construction (ADR-0183).
///   What is left is one [`program_bay`] — a dozen divisions and two fits, no
///   allocation — and no repaint is asked for, which is P-0072's first clause.
/// - **The frame it does change solves to the new arrangement and not to the
///   previous one**, because the second solve is after the write. That frame
///   costs **two solves**, and it is worth saying plainly rather than hiding:
///   a placement only changes when the bay's rectangle does, which is a window
///   resize or a drag on a boundary, and P-0072 does not budget what the
///   operator does.
/// - **Nothing re-enters the solve.** The value written is derived from the
///   bay's rectangle, and the bay is `Fixed(378)` over a flexible
///   `program-view`: what it can use is unbounded whether or not the row is
///   set aside, so the second solve gives the bay the rectangle the first one
///   did and asking again would write the same bit. One step, and it is a
///   fixed point rather than a loop — `tests/rearrange.rs` asserts that by
///   running this twice and watching the second one say nothing moved.
///
/// The one case where the bay's rectangle *does* depend on the bit is the one
/// the guard rule covers: with the picture folded the bay can use only the
/// row, and a row set aside would leave it able to use nothing. [`program_bay`]
/// never answers *beside* there, so that fixed point is the row's 72 and not
/// zero.
pub fn rearrange(panel: &mut Panel, canvas: (u32, u32)) -> bool {
    panel.solve();
    let beside = matches!(
        program_bay(panel.layout(), canvas).map(|bay| bay.placement),
        Some(Placement::Beside)
    );
    let Some(row) = panel.layout().find("deck-previews") else {
        return false;
    };
    let moved = panel.set_aside(row, beside);
    // The second solve, and on all but the frame the placement changed it is
    // the flag test the first one was.
    panel.solve();
    moved
}

/// **What the transport row reads this frame**: the session's tempo and
/// position, and what the frame before this one cost.
///
/// # The same seam as [`View::picture`], and it is not crossed
///
/// Every field is a number and none of them is a `Deck`, an `Instant` or a
/// `Duration` that means *now*. This crate takes no device, no window and no
/// clock (ADR-0156), and this row is made of a clock and an engine — so
/// whoever owns those reads them and writes this per frame, exactly as
/// whoever owns the device registers a texture and writes [`View::picture`].
/// A `Deck` here would put the engine in this crate's dependencies; an
/// `Instant` here would put a clock in it, and then the row would be reading
/// wall time in a repository whose first principle is that nothing does
/// ([P-0002](../../../docs/principles/0002-simulation-time-comes-from-a-record-never-from-a-clock.md)).
///
/// **It carries what cannot be derived and nothing that can.** The beat within
/// the bar and the bar number are arithmetic on [`Transport::beats`] and are
/// [`Transport::beat`] and [`Transport::bar`] rather than two more fields:
/// three numbers for one position is three chances for the lit dot and the bar
/// beside it to come from different arithmetic and disagree.
///
/// **No repaint arm.** [`crate::repaint::Change`] is one list of everything
/// that can change what the console shows, and this is not on it: the values
/// move when the engine draws a frame, and a caller that is drawing engine
/// frames is already asking for frames for the picture beside this row. A
/// panel with nothing live on it is a panel where the oscillator is not
/// advancing either, so the row is right to be still — see
/// [`Transport::fps`], which is the one field that would go stale there and
/// is the one the example leaves `None`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transport {
    /// The tempo, drawn in `.bpm`'s treatment. `karakuri_signal`'s
    /// `Oscillator::bpm` is an `f32` and so is this.
    ///
    /// **Not derivable from [`Transport::beats`]**, which is why it is its
    /// own field: the oscillator's position is an accumulator precisely so
    /// that a tempo correction changes the rate from now on without moving a
    /// beat that has already happened, so the tempo is not the slope of the
    /// position and cannot be read off it.
    pub bpm: f32,
    /// **Musical position, unbounded and monotone** — `Oscillator::beats`,
    /// which counts from the start of the session and never wraps.
    ///
    /// `f64` because that is the type the oscillator accumulates it in, and
    /// narrowing a number this crate does not own is a rounding taken for
    /// nothing. What it would eventually cost is real but distant — an `f32`
    /// stops separating consecutive beats somewhere past eight million of
    /// them — so the reason to keep it is the first one and not the second.
    ///
    /// Which dot is lit and which bar it is are both read off this one number
    /// — see [`Transport::beat`] and [`Transport::bar`].
    pub beats: f64,
    /// **How many beats there are in a bar**, which is how many dots the grid
    /// has.
    ///
    /// **A field rather than a constant, and that is a conclusion rather than
    /// caution.** The mock draws four and `karakuri_signal`'s
    /// `oscillator::BEATS_PER_BAR` is 4, so the number is not in doubt
    /// today — but that constant says of
    /// itself that *"v0.2 of the IR spec has no time-signature concept
    /// anywhere … a fixed assumption of common time rather than something
    /// derived from the record stream. Treat it as provisional until a real
    /// time signature shows up in the format."* A constant here would be this
    /// crate transcribing a number the crate that owns it has written down as
    /// provisional, and the day a time signature lands the console would draw
    /// four dots against a grid of three with nothing saying so. So it is
    /// asked of whoever owns the grid, once a frame, and the grid is that many
    /// dots wide.
    ///
    /// Read through [`Transport::dots`], which is where a zero is dealt with.
    pub beats_per_bar: u32,
    /// **Frames a second**, or `None` where nobody can say yet.
    ///
    /// A rate is measured over a stretch, so a window that has not been left
    /// alone for one has no rate — and `None` draws no `fps` at all rather
    /// than a `0` or a stale number. **Not derived from
    /// [`Transport::frame_ms`]**: `1000 / frame_ms` is the rate the loop
    /// *could* manage, and what it *does* draw is decided by the display and
    /// by whether anything asked for a frame. On a `Fifo` surface those two
    /// differ by the whole vsync wait, which is most of the frame.
    pub fps: Option<f32>,
    /// **What one frame cost on the CPU**, in milliseconds — the `12.4` in the
    /// mock's `12.4/16.6 ms`.
    ///
    /// It is a fact about a frame that has already been drawn, so it does not
    /// go stale on a still window the way a rate does: the last frame did cost
    /// this.
    pub frame_ms: f32,
    /// **What a frame has to fit in**, in milliseconds — the `16.6`, and
    /// `None` where nothing can say what it is.
    ///
    /// Then the row draws the frame time with no budget beside it, which is
    /// the rule the whole of this value follows: a reading nobody has is not
    /// drawn as a plausible one.
    pub budget_ms: Option<f32>,
}

impl Transport {
    /// **How many dots the beat grid has**: [`Transport::beats_per_bar`], and
    /// at least one.
    ///
    /// The clamp is here and in one place because a bar of no beats is two
    /// failures at once — a grid with nothing in it, and a division by zero in
    /// [`Transport::beat`] that comes out `NaN` and lights no dot in a row
    /// that has none. One beat to the bar is the nearest thing to a grid that
    /// can be drawn, and every reader of the field goes through here.
    pub fn dots(&self) -> u32 {
        self.beats_per_bar.max(1)
    }

    /// **Which dot is lit**: the beat within the current bar, from zero.
    ///
    /// `rem_euclid` rather than `%` because [`Transport::beats`] can be
    /// negative — `Oscillator::behind` reads the same grid at an earlier time,
    /// and a slot warming behind the session is exactly what that is for — and
    /// `%` on a negative is negative, which is a dot index no grid has.
    ///
    /// **The clamp on the way out is not belt and braces.** `(-1e-18_f64)
    /// .rem_euclid(4.0)` is `4.0` exactly: the true remainder is a hair under
    /// the divisor and rounds up to it. That is one dot past the end of the
    /// grid, which is a rectangle drawn beside it or a panic on an index, for
    /// a value that is *just before* the downbeat.
    pub fn beat(&self) -> u32 {
        let dots = self.dots();
        (self.beats.rem_euclid(dots as f64) as u32).min(dots - 1)
    }

    /// **Which bar it is, counting from one** — the `37` in the mock's
    /// `bar 37`.
    ///
    /// One-based because that is how a bar is counted out loud, and there is
    /// no bar 0 in anything an operator says. Signed for the reason
    /// [`Transport::beat`] takes `rem_euclid`: a position before the session's
    /// zero is a bar before the first one, and `as u32` on it would saturate
    /// to zero and draw `bar 1` for every one of them.
    pub fn bar(&self) -> i64 {
        self.beats.div_euclid(self.dots() as f64) as i64 + 1
    }
}

/// **The transport row, laid out**: where each of the four readouts goes, and
/// which beat is lit.
///
/// # One derivation, for the reason [`Outputs`] is one
///
/// [`View::draw`] paints exactly these rectangles. Nothing hit-tests them
/// **because nothing in this row is a control** — see below — so this has one
/// call site today where [`outputs`] had two on the day it was written. It is
/// a value rather than a paint-as-you-go pass all the same, because that is
/// what makes the arithmetic something `tests/transport.rs` can ask about
/// without a device, which is the whole of how this crate is checked.
///
/// # Nothing here is a control, and that is the answer rather than an omission
///
/// [`crate::input::claim`] asks [`outputs`] and nothing else, and it stays
/// that way: a point in this row that is not inside a boundary's [`GRAB`] is
/// `egui`'s. Every one of the mock's controls in this row is in the list below
/// of things that are not drawn, so what is left — a tempo, a beat, a bar and
/// a frame time — is four readouts, and a readout is not something a press
/// acts on. `tests/transport.rs` asserts it over the row rather than leaving
/// it to be inferred from the absence of a hit test.
///
/// # What is in the mock's row and is deliberately not here
///
/// The mock draws ten things and **four of them exist**. Each of the other six
/// is a control over machinery that is in neither this crate nor the example,
/// and drawing one is the scaffolding this module's documentation refuses — a
/// pill that looks like a control and does nothing does not get replaced:
///
/// - `audio-in`, drawn `.armed`, is the session listening to the room. What
///   would be behind it is a measured `AudioFrame` on the session's signals,
///   and nothing here opens an input or installs one — a `Deck` with nobody
///   calling `set_audio` runs on a synthesized bus, and an armed pill would
///   say the room is being listened to while it is not.
/// - `tap` is tap tempo: a performer's taps timed and turned into a tempo
///   correction. Nothing times a tap here, and the pill would correct nothing.
/// - `learn` is MIDI learn, and its own tooltip says what it would do —
///   *"Click, then point at any control and move a knob on your surface, and
///   the two are mapped."* There is no map, no learn mode, and exactly one
///   control on the panel to point at.
/// - `map · nanoKONTROL2 ▾` is the map in use, *and it is a file*. None is
///   loaded, saved or recalled anywhere here, and a menu naming a device
///   nobody has plugged in is worse than no menu.
/// - `landed` is what the last write did — *landed, rolled back for cost, or
///   failed to build*. That is a build watcher's verdict over a hot swap, and
///   nothing writes a procedure while this panel runs, so the pill would be
///   reporting on a write that never happens.
/// - `● rec` is recording the session to the store as it happens. There is no
///   session recorder behind this panel and no record stream is written from
///   it.
///
/// The `.sep` between the bar and the frame readout **is** drawn, in the only
/// way a `flex: 1` spacer can be: it is space, so what it does is push the
/// frame readout to the right edge, and that is where [`transport_row`] puts
/// it.
///
/// # No tooltips, for the reason the Outputs row has none
///
/// Four of the mock's ten items carry a `data-tip` and two of the four drawn
/// ones do. A tooltip needs `egui` to own a widget, this console paints, and
/// giving one readout a widget is a decision about who owns the pointer — see
/// [`outputs`], where the same sentence is written about a control.
///
/// `layout` must be solved: [`Layout::rect`] refuses to answer from a dirty
/// one. `ctx` is asked for the type, because where each readout ends is where
/// the next one starts.
pub fn transport(
    ctx: &egui::Context,
    layout: &karakuri_layout::Layout,
    values: Option<Transport>,
) -> Option<TransportRow> {
    // **No engine behind the console, so there is no row.** Every test in this
    // crate is here, and so is the whole of `cargo test -p karakuri-console`.
    // Drawing a row of zeroes would be inventing a tempo nothing is running
    // at; this is `View::picture`'s rule, one row along.
    let values = values?;
    // Fonts are not valid until `egui` has run a pass, exactly as in
    // `outputs`. Nothing routes a pointer here, so this is only the frame
    // before the first one — and there is nothing to draw on it either.
    if ctx.cumulative_pass_nr() == 0 {
        return None;
    }
    let row = to_egui(layout.rect(layout.find("transport")?));
    let width = |job: LayoutJob| ctx.fonts_mut(|f| f.layout_job(job).size().x);
    let bpm = width(bpm_job(&values, Color32::PLACEHOLDER, Color32::PLACEHOLDER));
    let label = width(span(BPM_LABEL, Color32::PLACEHOLDER));
    let bar = width(span(&bar_text(&values), Color32::PLACEHOLDER));
    let frame = width(frame_job(
        &values,
        Color32::PLACEHOLDER,
        Color32::PLACEHOLDER,
    ));
    transport_row(row, &values, bpm, label, bar, frame)
}

/// **The faint word beside the number**, and the mock's own capitalisation
/// this time: `.transport`'s `BPM` is upper-case in the markup rather than in
/// CSS, so it is upper-case here.
const BPM_LABEL: &str = "BPM";

/// The transport row's furniture: a rectangle for each readout, and which beat
/// is lit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransportRow {
    /// The tempo, in `.bpm`'s 20px box.
    pub bpm: Rect,
    /// The faint `BPM` beside it.
    pub label: Rect,
    /// The whole `.beat-grid`: [`TransportRow::dots`] dots and the gaps
    /// between them. One dot is [`TransportRow::dot`].
    pub grid: Rect,
    /// How many dots are in that grid — [`Transport::dots`], carried so the
    /// painter and a test read the same number the width was built from.
    pub dots: u32,
    /// **Which of them is lit**: [`Transport::beat`], which is always a dot
    /// this grid has.
    pub on: u32,
    /// `bar 37`.
    pub bar: Rect,
    /// The frame readout, pushed to the right edge by the `.sep`.
    pub frame: Rect,
    /// **The values these rectangles were measured from.**
    ///
    /// Carried rather than passed to the painter beside this, for
    /// [`Picture`]'s own reason: whoever measured the type and whoever paints
    /// it are then one statement, so a row laid out for one tempo and painted
    /// with another cannot be written by accident. It is also what leaves
    /// **one** place where *no engine means nothing at all* is decided — this
    /// answering `None` — rather than that rule being asked once here and
    /// again in [`View::draw`], which is two answers that agree until they do
    /// not.
    pub values: Transport,
}

impl TransportRow {
    /// One dot of the beat grid, from the left. **The gaps are between the
    /// dots and nowhere else**, which is what a flex row with a `gap` is —
    /// the same reading [`preview_cells`] takes of a grid.
    ///
    /// Panics on a dot this grid has not got, which is a caller having
    /// invented a beat: [`TransportRow::on`] is [`Transport::beat`] and that
    /// is inside the grid by construction.
    pub fn dot(&self, index: u32) -> Rect {
        assert!(index < self.dots, "dot {index} of a grid of {}", self.dots);
        Rect::from_min_size(
            Pos2::new(
                self.grid.min.x + (size::BEAT_W + size::BEAT_GAP) * index as f32,
                self.grid.min.y,
            ),
            egui::vec2(size::BEAT_W, size::BEAT_H),
        )
    }
}

/// The arithmetic of the row, away from the type it measures and the layout it
/// reads.
///
/// Term for term from `.transport` and what is in it, in `style.css`:
///
/// - `.transport { display: flex; align-items: center; gap: 14px;
///   padding: 9px 12px }` — the readouts laid left to right from
///   [`size::TRANSPORT_PAD_X`], one [`size::TRANSPORT_GAP`] between each pair,
///   and every one of them **centred in the row** rather than sat on its
///   padding. The four boxes are four different heights — 30 for the number,
///   16.5 for a line of type, 6 for the beat grid — and `align-items: center`
///   is what puts them on one line through the middle.
/// - `.sep { flex: 1 }` — a spacer that takes everything left over, so what
///   comes after it is against the row's right padding. The frame readout is
///   laid out from the right edge backwards for that reason, and it is the one
///   thing in the row whose position is not the position of the thing before
///   it.
///
/// # The 48 comes back here
///
/// The row is 48 and the number in it is 30 (`.bpm`'s 20px at
/// `line-height: 1.5`), so there is (48 - 30) / 2 = **9** of row above and
/// below it — which is `.transport`'s own `padding: 9px`, arrived at from the
/// other end. The arrangement's `9 + 30 + 9` and the centring agree exactly
/// here, where the Outputs row's 8 + 18.5 + 8 had to give up a quarter pixel.
///
/// `None` where the row cannot hold what goes in it: folded away, soloed away,
/// or too narrow to keep the frame readout clear of the bar. **The mock wraps
/// and this does not**, which is the argument [`outputs_row`] makes about the
/// second sink, in the one case where the mock's `flex-wrap: wrap` has
/// something to wrap: a row too narrow draws nothing rather than a second line
/// of transport in a bay 48 tall that has no room for one.
fn transport_row(
    row: Rect,
    t: &Transport,
    bpm_w: f32,
    label_w: f32,
    bar_w: f32,
    frame_w: f32,
) -> Option<TransportRow> {
    let mid = row.center().y;
    // An inline span at the console's own type: `font-size: 11px` at
    // `line-height: 1.5`, which is the box every line of ordinary text in the
    // mock sits in.
    let span_h = size::BASE * size::LINE;
    let bpm = Rect::from_min_size(
        Pos2::new(row.min.x + size::TRANSPORT_PAD_X, mid - size::BPM_H * 0.5),
        egui::vec2(bpm_w, size::BPM_H),
    );
    let label = Rect::from_min_size(
        Pos2::new(bpm.max.x + size::TRANSPORT_GAP, mid - span_h * 0.5),
        egui::vec2(label_w, span_h),
    );
    let dots = t.dots();
    let grid = Rect::from_min_size(
        Pos2::new(label.max.x + size::TRANSPORT_GAP, mid - size::BEAT_H * 0.5),
        egui::vec2(
            size::BEAT_W * dots as f32 + size::BEAT_GAP * (dots - 1) as f32,
            size::BEAT_H,
        ),
    );
    let bar = Rect::from_min_size(
        Pos2::new(grid.max.x + size::TRANSPORT_GAP, mid - span_h * 0.5),
        egui::vec2(bar_w, span_h),
    );
    // The `.sep`: everything after it is against the right padding.
    let frame = Rect::from_min_size(
        Pos2::new(
            row.max.x - size::TRANSPORT_PAD_X - frame_w,
            mid - span_h * 0.5,
        ),
        egui::vec2(frame_w, span_h),
    );
    // The same rule `picture_rect` states, on the two ends of the row: the
    // number is the tallest thing in it and the frame readout is the furthest
    // right, so a row that holds both holds everything between them — and the
    // last clause is the wrap the mock does and this does not.
    match row.contains_rect(bpm)
        && row.contains_rect(frame)
        && frame.min.x >= bar.max.x + size::TRANSPORT_GAP
    {
        true => Some(TransportRow {
            bpm,
            label,
            grid,
            dots,
            on: t.beat(),
            bar,
            frame,
            values: *t,
        }),
        false => None,
    }
}

/// **The tempo as one laid-out run**, so that measuring it and painting it
/// cannot be two different runs of type.
///
/// `.bpm`'s `128.0` is one decimal place, which is also as fine as a tempo is
/// ever named — and the mock's own number, so a transcription that started
/// printing `128` would be visible against it.
///
/// # The gradient is drawn per glyph, which is what the CSS comes to here
///
/// `.bpm` is `background: linear-gradient(94deg, var(--c-mint), var(--c-lav))`
/// with `background-clip: text`: the two colours run left to right across the
/// number, near enough — 94deg is four degrees off horizontal. `epaint` fills
/// a galley with one colour, so the run is split into one format run per glyph
/// and each takes its own point along the ramp. Five glyphs is a coarse ramp
/// and it is the mock's two colours rather than one of them; the alternative
/// is picking an end and losing the other, which is a transcription that drops
/// half of what it read.
fn bpm_job(t: &Transport, from: Color32, to: Color32) -> LayoutJob {
    let text = bpm_text(t);
    let mut job = LayoutJob::default();
    let last = text.chars().count().saturating_sub(1).max(1) as f32;
    for (n, (at, ch)) in text.char_indices().enumerate() {
        job.append(
            &text[at..at + ch.len_utf8()],
            0.0,
            TextFormat {
                font_id: FontId::new(size::BPM_SIZE, FontFamily::Proportional),
                extra_letter_spacing: size::BPM_TRACKING,
                color: mix(from, to, n as f32 / last),
                ..Default::default()
            },
        );
    }
    job
}

/// The tempo, as the mock writes it.
fn bpm_text(t: &Transport) -> String {
    format!("{:.1}", t.bpm)
}

/// The bar, as the mock writes it: `bar 37`.
fn bar_text(t: &Transport) -> String {
    format!("bar {}", t.bar())
}

/// **The frame readout as one laid-out run**: `58 fps · 12.4/16.6 ms`, with
/// the numbers in `.val` and everything else faint.
///
/// One [`LayoutJob`] rather than four galleys laid end to end, because the
/// mock is one run of text with two colours in it — `.val { color:
/// var(--c-text); font-weight: 500 }` inside a span that is `--c-faint` — and
/// laying it out as one is what keeps the spaces between the parts the type's
/// own rather than a gap this file invented. The weight is not honoured:
/// `egui`'s default proportional face has no bold, which `room` says once for
/// the whole crate.
///
/// **Both absences drop their own words and nothing else.** No rate drops
/// `58 fps · ` and leaves `12.4 ms`; no budget drops `/16.6` and leaves
/// `12.4 ms`. Neither draws a `0`, a `—` or a plausible 16.6 that nothing
/// measured.
fn frame_job(t: &Transport, val: Color32, faint: Color32) -> LayoutJob {
    let mut job = LayoutJob::default();
    let mut push = |text: String, colour: Color32| {
        job.append(
            &text,
            0.0,
            TextFormat {
                font_id: FontId::new(size::BASE, FontFamily::Proportional),
                color: colour,
                ..Default::default()
            },
        );
    };
    if let Some(fps) = t.fps {
        push(format!("{fps:.0}"), val);
        push(" fps · ".to_owned(), faint);
    }
    push(format!("{:.1}", t.frame_ms), val);
    match t.budget_ms {
        Some(budget) => push(format!("/{budget:.1} ms"), faint),
        None => push(" ms".to_owned(), faint),
    }
    job
}

/// A plain span of the console's own type: `font-size: 11px`, one colour.
fn span(text: &str, colour: Color32) -> LayoutJob {
    LayoutJob::simple_singleline(
        text.to_owned(),
        FontId::new(size::BASE, FontFamily::Proportional),
        colour,
    )
}

/// Two colours mixed, `t` of the way from the first to the second.
///
/// In gamma space, component by component, because that is where a CSS
/// `linear-gradient` in `srgb` interpolates and this is transcribing one.
fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let at = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t).round() as u8;
    Color32::from_rgb(at(a.r(), b.r()), at(a.g(), b.g()), at(a.b(), b.b()))
}

/// **The transport row's contents**: the tempo, the beat grid, the bar and the
/// frame readout.
///
/// Where everything goes is [`transport_row`]'s, so this paints and derives
/// nothing. Term for term from `style.css`:
///
/// - `.bpm` — the gradient, which is [`bpm_job`]'s.
/// - the label beside it — `style="color:var(--c-faint)"` in the markup, which
///   is `pal.faint`.
/// - `.beat-grid i` — `background: var(--c-line)`, and `.on` is
///   `var(--c-pink)` with `box-shadow: 0 0 9px var(--c-glowp)`. The halo is an
///   [`egui::epaint::Shadow`] at [`size::BEAT_GLOW`] with a corner radius of
///   half the dot's height, which is the same mechanism the Outputs row's dot
///   uses for its own glow.
/// - `bar 37` — `style="color:var(--c-dim)"`, `pal.dim`, *a label beside a
///   value*.
/// - the frame readout — `.val` over `--c-faint`, which is [`frame_job`]'s.
///
/// Every galley is centred in the box [`transport_row`] gave it, which is
/// `align-items: center` done where the type's real height is known: a
/// 20px face is not 30 tall and an 11px one is not 16.5, so the CSS box and
/// the galley are different heights and the box is what is centred on.
fn transport_into(ui: &Ui, pal: &Palette, row: &TransportRow) {
    let t = &row.values;
    let painter = ui.painter();
    let centred = |rect: Rect, galley: std::sync::Arc<egui::Galley>| {
        painter.galley(
            Pos2::new(rect.min.x, rect.center().y - galley.size().y * 0.5),
            galley,
            // Every run in these jobs carries its own colour, so the fallback
            // `epaint` substitutes for a `Color32::PLACEHOLDER` is never
            // reached — and where it were, ink is a better answer than none.
            pal.text,
        );
    };

    centred(row.bpm, painter.layout_job(bpm_job(t, pal.mint, pal.lav)));
    centred(row.label, painter.layout_job(span(BPM_LABEL, pal.faint)));

    let radius = CornerRadius::same((size::BEAT_H * 0.5) as u8);
    for index in 0..row.dots {
        let dot = row.dot(index);
        let lit = index == row.on;
        if lit {
            painter.add(
                egui::epaint::Shadow {
                    offset: [0, 0],
                    blur: size::BEAT_GLOW,
                    spread: 0,
                    color: pal.glow_pink,
                }
                .as_shape(dot, radius),
            );
        }
        painter.rect_filled(
            dot,
            radius,
            match lit {
                true => pal.pink,
                false => pal.line,
            },
        );
    }

    centred(row.bar, painter.layout_job(span(&bar_text(t), pal.dim)));
    centred(
        row.frame,
        painter.layout_job(frame_job(t, pal.text, pal.faint)),
    );
}

/// **The word at the head of the Outputs row**, in the source's own
/// capitalisation for the reason [`Kind::Bay`]'s title is: the mock
/// upper-cases in CSS, and that is done at paint time here so the word a
/// reader searches for is the word in the source.
///
/// It is **not** a [`bay_head`]. The mock's markup is an inline span with
/// `.bay-head`'s type on it — `font-size: 10px`, `letter-spacing: 0.16em`,
/// `text-transform: uppercase`, `--c-faint` — inside a row that has no head at
/// all (ADR-0159), so it is that typography and none of that structure: no
/// hairline under it, no pills or grip beside it, and the row's own padding
/// rather than a head's.
const OUTPUTS_LABEL: &str = "Outputs";

/// **The one sink the console has**, and the manual's name for it: *"The
/// picture in the Program bay is a sink like any other and is the first
/// row."*
const PROGRAM_VIEW: &str = "program view";

/// **The Outputs row, laid out**: where the word goes, where the console's one
/// control is, and whether that control is lit.
///
/// # One derivation, because a control drawn where it cannot be clicked is
/// silent
///
/// [`View::draw`] paints exactly these rectangles and [`crate::input::claim`]
/// hit-tests exactly these rectangles, the way [`preview_cells`] serves both
/// [`preview_rects`] and the frame. Two copies of this arithmetic is a dot
/// that lights up under a pointer that cannot switch it, and nothing on screen
/// says so.
///
/// # `on` is read, never stored
///
/// The manual: *"The picture is a sink, listed in Outputs as program view, and
/// **it is on screen exactly when that sink is on**."* So the sink's state is
/// [`Layout::visible`] on the picture's node and there is no second copy of it
/// to drift — a fold from the keyboard lights the dot down, and the dot folds
/// the same node the keyboard does.
/// [ADR-0161](../../../docs/adr/0161-a-solo-is-stored-because-it-cannot-be-derived.md)
/// stored `soloed` because it could not be derived; this can.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Outputs {
    /// Where the word OUTPUTS is painted: its top-left, and the box the
    /// galley fills.
    pub label: Rect,
    /// **The control**: `.sink`'s capsule, dot and name together, which is
    /// what a press has to land in. The mock gives the whole chip the click,
    /// not the dot alone — a 7px dot is not a target a hand finds, which is
    /// [`GRAB`]'s argument one control along.
    pub sink: Rect,
    /// The `.dot` inside it.
    pub dot: Rect,
    /// The node this sink switches: the picture, `program-view`.
    pub id: NodeId,
    /// Whether the picture is on screen — `layout.visible(id)`, read here.
    pub on: bool,
}

impl Outputs {
    /// **What a press on the control asks for.** Two operations and no toggle:
    /// [`Op::Fold`] while the picture is on, [`Op::Unfold`] while it is off.
    /// The toggle is this method — an affordance over two operations — and the
    /// vocabulary underneath it stays two things a MIDI map or an MCP call can
    /// ask for by name. See [`Op`].
    ///
    /// **A press on a dark dot always lights it**, whatever darkened it — the
    /// picture folded on its own, the Program bay folded around it, a solo
    /// that left it out. That is [`Op::Unfold`]'s rule and not a special case
    /// here: an unfold makes its node *visible*, so it undoes the way to it as
    /// well as the node. This control is why the rule is written that way, and
    /// the manual's own note on this row is the argument — *"Nothing is
    /// refused here, so nothing has to be explained: a control that quietly
    /// declines the last of something is a rule an operator can only find by
    /// experiment."* A press that lit nothing would be that rule with no words
    /// at all.
    pub fn op(&self) -> Op {
        match self.on {
            true => Op::Fold(self.id),
            false => Op::Unfold(self.id),
        }
    }

    /// Whether `p` is on the control.
    pub fn hit(&self, p: karakuri_layout::Point) -> bool {
        self.sink.contains(Pos2::new(p.x, p.y))
    }
}

/// **The Outputs row's furniture, derived**: the word, and the one sink.
///
/// `None` where there is no row to draw in — the row folded away, a solo
/// somewhere else, or a window too small to hold the chip — which is
/// [`picture_rect`]'s rule stated on a control instead of a picture: a
/// rectangle with nothing in it is not something to paint or to click.
///
/// # What is in the mock's row and is deliberately not here
///
/// The mock draws four `.sink`s and a `+ add output` pill. **One of them
/// exists.** Drawing the others is the scaffolding this module's
/// documentation refuses — a control that looks finished does not get
/// replaced, and each of these is a control over something that has not been
/// built:
///
/// - `projector · DELL U2720Q` is a second window on another display. That is
///   a second `Sink` and a second surface, and neither exists.
/// - `Syphon` and `NDI · no plugin` are plugin sinks. There is no plugin
///   system, and `NDI`'s own row says so — it is drawn `.absent`, which is a
///   control explaining that the thing it would switch is not installed.
/// - `+ add output` adds a region while the panel is running, which the arena
///   cannot do: `docs/roadmap.md` records it as **one gap drawn five times**
///   (`+ lane`, `+ add`, `+ add output`, `+` on the scope list, and the
///   inspector's `2 up`), and it arrives with the arena operations, not with
///   this row.
///
/// # The tooltip is not drawn, and not half-drawn either
///
/// Every `.sink` in the mock carries a `data-tip`, and the manual makes a
/// point of it: *"hover says three things: what the control is, what state it
/// is in, and what a click will do."* A tooltip needs `egui` to **own a
/// widget** — a `Response` with a hover state and a layer above the panel —
/// and this console paints, with no widget anywhere in it
/// ([`crate::input`]). Giving one control a widget is a decision about who
/// owns the pointer, and it is its own; so no tooltip is drawn here and no
/// half of one is left behind.
///
/// # The row does not wrap, and the mock's does
///
/// `.outputs` is `flex-wrap: wrap`, which is a rule about what happens to the
/// *fifth* thing in a row that is too narrow to hold it. There is one sink and
/// it fits at every width the console draws — 172 of a narrowest 990 — so
/// wrapping is a rule with nothing to apply to, and it is a decision to take
/// with the second sink rather than a behaviour to write for one.
///
/// `layout` must be solved: [`Layout::rect`] refuses to answer from a dirty
/// one. `ctx` is asked for the type: the chip's width is the width of the name
/// in it, and where the name goes is where the word before it ended.
pub fn outputs(ctx: &egui::Context, layout: &karakuri_layout::Layout) -> Option<Outputs> {
    // **Fonts are not valid until `egui` has run a pass**, and it says so
    // outright. A pointer event can reach this before the first frame — the
    // window is up and the loop has not drawn yet — so the answer there is
    // that there is no control, which is also the true one: a control that has
    // never been drawn is not one a press can be on.
    if ctx.cumulative_pass_nr() == 0 {
        return None;
    }
    let row = to_egui(layout.rect(layout.find("outputs")?));
    let id = layout.find("program-view")?;
    let label = ctx.fonts_mut(|f| f.layout_job(label_job(Color32::PLACEHOLDER)).size().x);
    let name = ctx.fonts_mut(|f| {
        f.layout_no_wrap(
            PROGRAM_VIEW.to_owned(),
            FontId::new(size::BASE, FontFamily::Proportional),
            Color32::PLACEHOLDER,
        )
        .size()
        .x
    });
    outputs_row(row, label, name).map(|(label, sink, dot)| Outputs {
        label,
        sink,
        dot,
        id,
        on: layout.visible(id),
    })
}

/// The arithmetic of the row, away from the type it measures and the layout it
/// reads — the two rectangles [`outputs`] hands out and the dot inside the
/// second.
///
/// Term for term from `.outputs` and `.sink` in `style.css`:
///
/// - `.outputs { display: flex; align-items: center; gap: 8px;
///   padding: 8px 11px }` — the word and the chip laid left to right from
///   [`size::OUTPUTS_PAD_X`], one [`size::OUTPUTS_GAP`] between them, and both
///   **centred in the row** rather than sat on its padding.
/// - `.sink { padding: 1px 10px; gap: 6px; border-radius: 999px }` — a
///   [`size::SINK_H`] capsule holding a [`size::SINK_DOT`] dot, a
///   [`size::SINK_GAP`], and the name.
///
/// **The centring is where the 34 comes back.** The row is 34 and a sink is
/// 18.5, so there is (34 - 18.5) / 2 = 7.75 of row above the chip and 7.75
/// below — more than the six pixels [`GRAB`] widens the boundary above it by,
/// which is the whole reason this control can be clicked at all. `.outputs`'s
/// own padding is 8 and the arrangement rounded 8 + 18.5 + 8 down to 34, so
/// the quarter pixel the CSS and the row disagree by is spent here rather than
/// argued about: `align-items: center` is what the CSS says, and it is what
/// leaves the two clearances equal.
fn outputs_row(row: Rect, label_w: f32, name_w: f32) -> Option<(Rect, Rect, Rect)> {
    let mid = row.center().y;
    let label = Rect::from_min_size(
        Pos2::new(row.min.x + size::OUTPUTS_PAD_X, mid - size::HEAD_SIZE * 0.5),
        egui::vec2(label_w, size::HEAD_SIZE),
    );
    let sink = Rect::from_min_size(
        Pos2::new(label.max.x + size::OUTPUTS_GAP, mid - size::SINK_H * 0.5),
        egui::vec2(
            size::SINK_PAD_X * 2.0 + size::SINK_DOT + size::SINK_GAP + name_w,
            size::SINK_H,
        ),
    );
    let dot = Rect::from_center_size(
        Pos2::new(sink.min.x + size::SINK_PAD_X + size::SINK_DOT * 0.5, mid),
        egui::vec2(size::SINK_DOT, size::SINK_DOT),
    );
    // The same rule [`picture_rect`] states, on the chip rather than on the
    // row because the chip is what is drawn and clicked: a row folded away has
    // a rectangle with no extent in it, and one too narrow for the chip has
    // nowhere to put it. Either way there is no control.
    match row.contains_rect(sink) {
        true => Some((label, sink, dot)),
        false => None,
    }
}

/// The word OUTPUTS as one laid-out run, so that measuring it and painting it
/// cannot be two different runs of type.
fn label_job(colour: Color32) -> LayoutJob {
    spaced(
        &OUTPUTS_LABEL.to_uppercase(),
        size::HEAD_SIZE,
        colour,
        size::HEAD_TRACKING,
    )
}

// ---------------------------------------------------------------------------
// The Mixer bay
// ---------------------------------------------------------------------------

/// **The word at the head of the Mixer bay**, in the source's own
/// capitalisation for [`Kind::Bay`]'s reason: the mock upper-cases in CSS, and
/// that is done at paint time so the word a reader searches for is the word in
/// the source.
const MIXER_TITLE: &str = "Mixer";

/// `.trim .lbl`: the `g`, which is the only word in this bay that is neither
/// the deck's nor the engine's — it is the mock's.
const TRIM_LABEL: &str = "g";

/// **How long the panel has been animating**, and the one value everything
/// that moves on it derives its own rate from.
///
/// # One phase, panel-wide, because two clocks drift and one does not
///
/// [P-0075](../../../docs/principles/0075-a-pending-transition-shows-where-it-is-where-it-is-going-and-that-it-has-not-arrived.md)
/// asks for exactly this: *"Everything pending moves together. Two controls
/// moving out of step looks broken rather than informative, and it is not a
/// smaller cost either: N independent animations are N deadlines for a
/// scheduler to service where one phase is one."* Two strips parked at once
/// are two rolls, and they are the same roll because they are read off the
/// same number.
///
/// # It is elapsed time and not a wrapped fraction
///
/// A phase already reduced to *where we are in the cycle* fixes the cycle, and
/// then a second presentation at another rate cannot be derived from it at all
/// — it would need a phase of its own, which is the drift this exists to
/// prevent. So the value is the whole elapsed interval and each presentation
/// takes its own [`Phase::cycle`] out of it. The console has one moving thing
/// today; it is about to have the beat and the rest of the mixer, and *this is
/// where the second one is either free or a second clock*.
///
/// # It is a `Duration` and not an `Instant`, which is the whole seam
///
/// `src/` reads no clock. [`Transport`]'s own doc says why in the general
/// case — *"an `Instant` here would put a clock in it, and then the row would
/// be reading wall time in a repository whose first principle is that nothing
/// does"*
/// ([P-0002](../../../docs/principles/0002-simulation-time-comes-from-a-record-never-from-a-clock.md))
/// — and every `Instant::now` in this crate is in `examples/panel.rs`, which
/// owns the window. An `Instant` is a *reading*; a `Duration` is a number, and
/// a number is what a caller writes and a test chooses. So this arrives per
/// frame the way [`Transport`] and [`View::mixer`] arrive
/// ([ADR-0156](../../../docs/adr/0156-the-consoles-arrangement-is-a-tree-this-repository-owns.md)),
/// and a test asserting an animation writes the phase it wants rather than
/// catching one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Phase(Duration);

impl Phase {
    /// The origin, and what a console with no clock behind it is at — every
    /// test in this crate, and the first frame of a window.
    pub const ZERO: Phase = Phase(Duration::ZERO);

    /// A phase from the interval that has elapsed since whoever owns the clock
    /// started counting. Where the origin is does not matter and is not
    /// asked: every presentation is periodic in it.
    pub const fn since(elapsed: Duration) -> Phase {
        Phase(elapsed)
    }

    /// **Where this phase sits inside one `period`**, as a fraction in
    /// `[0, 1)`.
    ///
    /// The one operation a presentation performs on a phase, and the reason
    /// the carrier is an interval: a rate is chosen here, by the thing that
    /// moves, rather than baked into the value the harness writes.
    ///
    /// In `f64` and returned as `f32`. A session runs for hours and a phase is
    /// seconds from an origin — at 3600 s an `f32` step is already coarser
    /// than a millisecond, and the fractional part is what survives the
    /// division, so the division is the one place the extra bits are worth
    /// having.
    pub fn cycle(self, period: Duration) -> f32 {
        let period = period.as_secs_f64();
        match period > 0.0 {
            true => (self.0.as_secs_f64() / period).fract() as f32,
            false => 0.0,
        }
    }
}

/// **How often anything pending on this panel sets off toward where it is
/// going**, and the period every other number in the presentation is a
/// fraction of.
///
/// One second, which is P-0075's own *"once a second"* and is the rate that
/// reads as *waiting* rather than as a fault. See
/// [ADR-0190](../../../docs/adr/0190-the-parked-tally-rolls-because-two-lamps-do-not-fit-in-fifty-three-pixels.md).
///
/// **Two users, one rate, and that is the rule rather than a coincidence**:
/// the tally's word rolling toward a residency, and a fader's fill reaching
/// toward a scheduled value ([`Reach`],
/// [ADR-0206](../../../docs/adr/0206-a-fader-marks-where-it-is-going-and-keeps-reaching-for-it.md)).
/// P-0075 asks for one phase panel-wide because two controls moving out of
/// step looks broken rather than informative, and a second period here is how
/// that would happen with nothing failing to compile.
pub const ROLL_PERIOD: Duration = Duration::from_millis(1000);

/// **How much of the period the word is moving for**: 400 ms out and back,
/// and 600 ms at rest.
///
/// The rest is not slack. It is what makes the roll read as *an attempt that
/// keeps being made* rather than as a chip that wobbles — and it is what makes
/// most of a parked strip's frames the strip's settled appearance, so the
/// operator reads the effective residency off a still chip nearly two thirds
/// of the time (P-0075's first clause).
pub const ROLL_TRAVEL: Duration = Duration::from_millis(400);

/// **How far toward the destination the moving thing gets**: a fraction of the
/// pitch between the two words on the tally, and a fraction of the gap between
/// the two values on a fader.
///
/// Less than one, and that is the presentation: *"a slot machine that never
/// quite lands"*. Landing is what arrival looks like, so a roll that reached
/// 1.0 would state the opposite of the truth once a second — and a band that
/// covered the gap to a fader's mark would say a scheduled fade had already
/// run. At 0.4 the current word keeps most of its rows and the destination
/// shows enough of its own to be read.
pub const ROLL_REACH: f32 = 0.4;

/// **How many steps the travel is drawn in**, which is the only reason
/// [`ROLL_STALENESS`] is a number at all.
const ROLL_STEPS: u32 = 12;

/// **How stale the roll may get**, which is what a presentation declares under
/// [P-0072](../../../docs/principles/0072-a-still-panel-costs-nothing-and-what-moves-declares-its-price.md)
/// and what the harness turns into a deadline.
///
/// [`ROLL_TRAVEL`] in [`ROLL_STEPS`] steps — 33.33 ms, about thirty a second.
/// It is **one number for the whole period** rather than a fine one while the
/// word moves and a coarse one while it rests: two numbers is two live regions
/// wearing one name, and choosing between them frame by frame is a scheduler's
/// job (P-0072's second half) rather than a presentation's. A presentation
/// declares what it needs; what the panel can afford is decided somewhere
/// else.
pub const ROLL_STALENESS: Duration =
    Duration::from_micros(ROLL_TRAVEL.as_millis() as u64 * 1000 / ROLL_STEPS as u64);

/// **How far a pending presentation has got toward its destination this
/// frame**, as a fraction of the distance to it: `0.0` at rest, [`ROLL_REACH`]
/// at the top of the travel, and never `1.0`.
///
/// One curve, two readers — the pitch between the tally's two words
/// ([`tally_into`]) and the gap between a fader's value and its mark
/// ([`reach`]).
///
/// A raised cosine over [`ROLL_TRAVEL`], flat for the rest of
/// [`ROLL_PERIOD`]. It is that curve rather than a triangle for one reason
/// worth having: it leaves and arrives at zero *with zero velocity*, so the
/// word does not snap into the rest it holds for the next 600 ms, and the
/// frame the travel begins on is not a jump.
///
/// **A pure function of the phase, which is the point of the phase being a
/// value.** A test asserts it at a phase it chose; nothing samples a clock to
/// find out what the panel is doing.
pub fn roll_at(phase: Phase) -> f32 {
    let travel = ROLL_TRAVEL.as_secs_f32() / ROLL_PERIOD.as_secs_f32();
    let t = phase.cycle(ROLL_PERIOD);
    match t < travel {
        true => ROLL_REACH * 0.5 * (1.0 - (t / travel * std::f32::consts::TAU).cos()),
        false => 0.0,
    }
}

/// **Where a slot sits between compiled and composited**, which is the mock's
/// `.tally` and `karakuri_engine`'s `Residency` — three states and no fourth.
///
/// The words are the mock's abbreviations and the variants are the engine's
/// names, because each document owns one end: `live`, `prim` and `alloc` are
/// what fits in a strip 53 wide, and `Live`, `Priming` and `Allocated` are
/// what `Deck::residency` answers.
///
/// **The mock draws a fourth and it is not a residency.** `.strip.empty` reads
/// `empty` in `.tally.off`, and a `Deck` has no such slot: every one of its
/// `slot_count` slots holds a `HotSwap`. An empty strip is a *track on the
/// page nothing fills*, and this bay draws nothing at all in one rather than a
/// strip full of dashes — see [`mixer`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tally {
    /// `.tally.live` — stepped and composited. On air.
    Live,
    /// `.tally.priming` — stepped and warming its buffers, drawn only while
    /// something auditions it.
    Priming,
    /// `.tally.alloc` — compiled, buffers held, not stepping, keeping its `t`.
    Allocated,
}

impl Tally {
    /// **Every residency there is**, so that anything which has to hold all
    /// three cannot be given two.
    ///
    /// One customer today and it is [`mixer`], which measures the chip against
    /// the widest word rather than the current one. A `match` cannot express
    /// *the widest of them*, and a list written at the call site would be a
    /// second list of the residencies — the exact drift a fourth variant would
    /// walk straight past. Here it is one line under the enum, and a fourth
    /// variant that is not added to it is a `[Tally; 3]` that no longer
    /// compiles.
    pub const ALL: [Tally; 3] = [Tally::Live, Tally::Priming, Tally::Allocated];

    /// The mock's own word, lower-case here and upper-cased at paint time
    /// because `.tally` is `text-transform: uppercase` — the rule
    /// [`Kind::Bay`]'s title states.
    pub fn word(self) -> &'static str {
        match self {
            Tally::Live => "live",
            Tally::Priming => "prim",
            Tally::Allocated => "alloc",
        }
    }
}

/// **What shape of the frame a layer reaches**: the second `.mini` in a strip,
/// and `karakuri_engine`'s `MaskKind` — three kinds and no fourth.
///
/// # It is a mark rather than a word, and the mark is drawn rather than typed
///
/// The mock writes `&#9711;` and `&#9681;` — a circle, and a circle with one
/// half filled — and it has to: a strip is 53 wide inside its padding, and
/// `over` beside `linear` is 75 before either mini's border. So the mask says
/// its state in a shape.
///
/// **Drawn rather than set as a glyph**, which is [`grip_dots`]'s argument one
/// control along: whether `◯` and `◑` are in `egui`'s default face is a
/// question with no good answer, and a circle is the same mark either way. It
/// also settles the third one. The mock names `◯` in its own tooltip —
/// *"Mask: none"* — and draws `◑` on the strip beside it without saying which
/// kind that is, and it draws no third mark anywhere; a glyph for the third
/// would be a character picked out of a font, where a **mark** is the shape
/// the mask makes and can be argued from the mask.
///
/// # There is no `ALL` beside it, where [`Tally`] and `BlendMode` have one
///
/// Both of those constants exist for a **reader**: `Tally::ALL` is what
/// [`mixer`] measures the tally capsule against, because a `match` cannot
/// express *the widest of them*, and `BlendMode::ALL` is what a map file is
/// offered. Nothing measures this chip against its three shapes — it holds a
/// mark rather than a word and is [`size::MINI_SIZE`] wide whichever shape it
/// is showing — and no map target names a shape, which is why
/// `karakuri_operation::WipeKind` has no `ALL` either and says so at
/// [`WipeKind::name`]. A list written here would be a second statement of the
/// order with no reader, and the order is [`next_shape`]'s: a `match`, so a
/// fourth shape does not compile until somebody says what follows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mask {
    /// No mask: the layer reaches the whole frame. The mock's `◯`.
    None,
    /// A straight edge across the frame — the mock's `◑`, which is what a hard
    /// edge down the middle of a circle looks like.
    Linear,
    /// A circle. The same outline with a filled centre, which is the same
    /// family as the mock's two and is the shape a radial mask makes.
    Radial,
}

/// **What a slot's meter last read**, which is two of the four numbers
/// `karakuri_engine::meter::Level` carries.
///
/// Plain numbers, for [`Transport`]'s reason: this crate takes no engine
/// (ADR-0156), so whoever owns the deck reads a level and writes this.
///
/// # What is left out, and why each
///
/// - **`frames_behind`.** `Deck::level` reads back without ever waiting, so a
///   reading is a few frames old and the engine says by how many. It is not
///   carried, and not because it does not matter: the engine's own
///   measurements are **1** frame behind under pacing, *"2 or 3"* on a vsync
///   window, and *"tens: 13 to 101"* with nothing pacing the loop at all — and
///   all three are normal. A staleness threshold picked in this crate would
///   blank a healthy meter on one loop and pass a dead one on another, because
///   the console does not know which loop it is on. What the meter draws is a
///   fact about a frame that has already been drawn, exactly as
///   [`Transport::frame_ms`] is, and it does not go stale by standing still
///   the way a *rate* does — which is the whole of why `frame_ms` is a number
///   and `fps` is an `Option`. So **handing over no reading at all is the
///   caller's decision**, on the same terms `fps: None` is, and the caller is
///   the one thing that knows its own loop.
/// - **`bad_texels`.** It is the denominator the mean was taken over rather
///   than a reading — *"`mean` and `peak` are computed over the texels this
///   did not count"* — and the mock's 6px meter has nowhere to say it. A meter
///   that drew it would be reporting on the measurement instead of on the
///   image.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Level {
    /// Mean luminance over the whole frame — *"the figure to match faders
    /// on"*, and what the meter's column is as tall as.
    pub mean: f32,
    /// The brightest single texel. What the peak mark sits on.
    pub peak: f32,
}

/// **What one mixer strip reads this frame**: what the deck is playing, where
/// the slot sits, and the four numbers in front of it.
///
/// # The same seam as [`Transport`], and it is not crossed
///
/// Every field is a string, a number or a word, and none of them is a `Deck`.
/// `src/` takes no device and no engine (ADR-0156), so whoever owns the deck
/// reads `Deck::residency`, `gain`, `opacity`, `blend`, `mask` and `level` and
/// writes one of these per slot per frame — exactly as whoever owns the device
/// registers a texture and writes [`View::picture`].
///
/// **No repaint arm for the values arriving**, which is [`Transport`]'s
/// reason: these move when the engine draws a frame, and a caller drawing
/// engine frames is already asking for frames for the picture two bays along.
///
/// **A drag on one of the two faders is a different thing and does have an
/// arm** — [`crate::repaint::Change::Emitted`]. That is not the value
/// arriving; it is an operation leaving, on a gesture an operator is making,
/// and it is on the list because [`crate::repaint::Change`] is one list of
/// everything that can change what the console shows and a fader that reached
/// no repaint would leave the strip drawn at the value before the drag.
///
/// # Five of these are controls and the rest are readouts, and the source
/// says which
///
/// **The two faders are played.** A press on the trim's knob or the fader's
/// knob takes it in hand ([`Mixer::grab`]), and dragging it emits
/// [`Operation::SetGain`] or `SetOpacity` for the caller to turn into a
/// record. Nothing here applies it, and nothing here keeps the value: the
/// fields below are still written by whoever owns the deck, every frame, so a
/// fader that has just been dragged shows the new number **because the deck
/// changed**.
///
/// **The blend chip is the third, and it cycles.** A press on it emits
/// [`Operation::SetBlendMode`] naming the mode after this one ([`Mixer::blend`]),
/// and nothing here applies that either. It is one control emitting three
/// operations, which is P-0074's own worked example of an affordance — see
/// [ADR-0187](../../../docs/adr/0187-the-blend-mini-cycles-and-a-map-learns-the-three-it-cycles-through.md).
///
/// **The tally chip is the fourth, and it cycles too.** A press on it emits
/// [`Operation::SetResidency`] naming the next of the three ([`Mixer::tally`])
/// — counted from [`Strip::requested`] rather than from [`Strip::tally`],
/// which is what makes a press on a parked chip the withdrawal of its own
/// prime request without a case in the code for it
/// ([ADR-0195](../../../docs/adr/0195-the-tally-chip-cycles-from-the-request-so-the-parked-case-needs-no-case.md)).
///
/// **The mask mini is the fifth, and it cycles too.** A press on it emits
/// [`Operation::SetMaskShape`] naming the shape after this one ([`Mixer::mask`])
/// — **and the angle this strip is already wearing**, which is
/// [`Strip::mask_angle`]: the chip chooses a shape and the angle is not its
/// business, so it hands back the one it was given rather than a default that
/// would straighten a diagonal front
/// ([ADR-0203](../../../docs/adr/0203-the-mask-chip-carries-the-angle-it-does-not-control.md)).
///
/// **Everything else in the bay is a readout.** The meter and the number are
/// drawn from what the deck says and a press on either reaches nothing — and
/// so does a press on a fader's *track*, off the knob, which would otherwise
/// be a jump nobody asked for. `tests/mixer.rs` asserts both directions rather
/// than leaving either to be inferred from the absence of a hit test.
///
/// **[`Strip::gain_to`] and [`Strip::opacity_to`] are readouts too**, and the
/// order is the tally's: the roll was drawn a commit before the chip became a
/// control, because a control that could not yet say it was pending would look
/// dead for exactly as long as that commit. Nothing here schedules a move,
/// nothing here cancels one, and no press on a mark reaches anything — a
/// scheduled fade is armed from a key, a map or an MCP call, and what this
/// strip owes it is that it is not invisible until it happens
/// ([ADR-0206](../../../docs/adr/0206-a-fader-marks-where-it-is-going-and-keeps-reaching-for-it.md)).
#[derive(Debug, Clone, PartialEq)]
pub struct Strip {
    /// **What the deck is playing**, in `.strip-name`.
    ///
    /// A `String`, and **the harness's word rather than the engine's**:
    /// nothing reachable from a `Deck` carries a name for the material in a
    /// slot. `Deck::slot` hands out a `HotSwap`, `HotSwap::set` hands out a
    /// `Set`, and a `Set` names its *nodes* (`Set::node_names`) and its
    /// *published controls* (`Published::name`) and has no name of its own —
    /// which is right, because a Set is built from a list of `.kir` files and
    /// only whoever passed that list knows what to call the result. So the
    /// caller names its own material, and a name invented in `src/` would be a
    /// label the console made up about somebody else's Set.
    ///
    /// Elided rather than wrapped where it does not fit, which is
    /// `.strip-name`'s own `overflow: hidden; text-overflow: ellipsis;
    /// white-space: nowrap`: a strip is 53 wide inside its padding and most
    /// real names are wider. An empty string draws no name at all, which is a
    /// harness with nothing to say rather than a strip with nothing in it.
    pub name: String,
    /// Where the slot sits — `Deck::residency`, which is the **effective**
    /// residency and not `requested_residency`. The governor moves a slot down
    /// without anybody asking it to, and a tally that did not follow it would
    /// be showing what was asked for over a slot doing something else.
    ///
    /// **What the chip draws, and not what a press on it counts from.**
    /// [`Mixer::tally`] steps from [`Strip::requested`]: the word says where
    /// the deck *is*, and the cycle is about what was last asked for. The two
    /// are the same value on every settled slot and they part exactly where it
    /// matters — see [`Mixer::tally`] for the parked case, which is the whole
    /// argument.
    pub tally: Tally,
    /// **What was asked for** — `Deck::requested_residency`, the other half of
    /// the pair [`Strip::tally`] is one of, and **what a press on the chip
    /// counts from** ([`Mixer::tally`]).
    ///
    /// # Why the strip carries both and derives nothing else
    ///
    /// [P-0075](../../../docs/principles/0075-a-pending-transition-shows-where-it-is-where-it-is-going-and-that-it-has-not-arrived.md)
    /// asks a pending control to say three things — where it is, where it is
    /// going, and that it has not arrived — and the first two *are* these two
    /// values. The third is [`Strip::pending`], which is a comparison of them.
    ///
    /// **There is deliberately no `parked: bool` beside them.** A third field
    /// would be the same fact stored twice, and the copy is the one that goes
    /// stale: the harness could write a `tally` and a `requested` that
    /// disagree and a `parked` that says they do not, and nothing in this
    /// crate could tell. One derivation with several readers is this crate's
    /// habit — [`StripBox::fader_at`] is the same shape — and it is P-0075's
    /// *derived every frame, never stored* read one level down, in the surface
    /// rather than in the engine.
    ///
    /// **Two fields rather than one `Tally` grown into a pair.** `Tally` is
    /// the mock's `.tally` and the engine's `Residency`, and both of those
    /// name *one* place a slot can sit; a variant meaning "allocated, and
    /// asked to prime" would be a fourth residency in a type whose own
    /// documentation says there are three and no fourth. The relation between
    /// two residencies is not a residency.
    pub requested: Tally,
    /// **The trim** — `Deck::gain`: linear, floored at zero, and deliberately
    /// open above 1.0 because the mix is HDR.
    ///
    /// Its own field and not the fader's twin. *"`gain` is a trim and
    /// `opacity` is the fader"*: opacity at zero silences under every blend
    /// mode and gain at zero does not silence `over`, which
    /// `karakuri-engine/src/mix.rs` states outright — so these are two
    /// controls, and the bay draws them as two rather than as two identical
    /// sliders. A `g` and a mini fader lying down against the tall one in the
    /// middle of the strip.
    pub gain: f32,
    /// **Where a scheduled move is taking the trim**, or `None` for a trim
    /// nothing is moving — `Deck::transitions_on(slot)`'s
    /// `Control::Gain` entry, and its `Transition::to`.
    ///
    /// # It is the destination and nothing else about the move
    ///
    /// A `Transition` also carries the instant it starts, how long it lasts
    /// and the curve it takes. None of the three is here, and the reason is
    /// the same seam every other field on this type is on: **the console has
    /// no beat count**, so a start in beats and a length in beats are two
    /// numbers it could not turn into anything a strip draws. What it can draw
    /// is where the control is going, which is P-0075's second clause and the
    /// whole of what the fader is being asked to say.
    ///
    /// **Armed and running are the same state here, deliberately.** A
    /// transition is in the deck's list from the moment it is scheduled until
    /// the beat it finishes on, and `Deck::transitions_on` answers with it
    /// throughout — so a fade quantised to the next bar and a fade halfway
    /// through are both *a request that has not arrived*, which is exactly the
    /// relation P-0075 is about. What separates them on the surface is that
    /// the value under the knob is moving in the second case, and the mark
    /// stands still in both.
    ///
    /// **One per control, because the engine allows one**: `Deck::schedule`
    /// cancels whatever was moving that pair before pushing, so a harness that
    /// finds two has read the wrong slot.
    ///
    /// # What it cannot say, and it is the trim's existing gap
    ///
    /// The track shows `[0, 1]` and the mix is HDR, so a move from 1.5 to 2.0
    /// is two values the track draws in one place — see [`StripBox::trim_at`],
    /// where that gap is already written down. [`Strip::gain_pending`] answers
    /// `None` there rather than declaring a staleness for an animation that
    /// would not move a pixel.
    pub gain_to: Option<f32>,
    /// **The fader** — `Deck::opacity`: a proportion in `[0, 1]`, and the one
    /// control that silences a slot under every blend mode, which is what
    /// makes it the way out of material that has gone NaN.
    pub opacity: f32,
    /// **Where a scheduled move is taking the fader**, or `None` for a fader
    /// nothing is moving — `Deck::transitions_on(slot)`'s `Control::Opacity`
    /// entry, on [`Strip::gain_to`]'s terms and for its reasons.
    ///
    /// # And the third control has nowhere to say it
    ///
    /// `Control` has three members and this strip carries two of them.
    /// `Control::MaskPosition` — *how far a mask's front has travelled* — is
    /// the one a wipe is made of, and **the strip has no control and no
    /// readout for it at all**: the mask `.mini` names a *shape*, and the
    /// position, the angle and the softness are an inspector row that does not
    /// exist yet ([`Strip::mask`] says so, and [`Strip::mask_angle`] is the
    /// same fact from the other side). A destination carried here for it would
    /// be a number with nowhere to be drawn, and drawing it on the shape chip
    /// would say a shape was changing when none is.
    ///
    /// So an armed wipe is invisible on this panel, it is an under-draw rather
    /// than a decision that it does not matter, and it is
    /// [ADR-0206](../../../docs/adr/0206-a-fader-marks-where-it-is-going-and-keeps-reaching-for-it.md)'s
    /// named consequence: what closes it is a mask-position control, and it
    /// closes the same day that control lands.
    pub opacity_to: Option<f32>,
    /// **The blend in force**, as one of the vocabulary's three — and the word
    /// drawn on the chip is [`BlendMode::name`].
    ///
    /// # Why this is not the engine's word
    ///
    /// It was a `&'static str` — `Blend::name`, handed straight through, on
    /// the argument that the console has nothing to do with the blend but draw
    /// the engine's word. **That stopped being true when the chip became a
    /// control** ([ADR-0187](../../../docs/adr/0187-the-blend-mini-cycles-and-a-map-learns-the-three-it-cycles-through.md)):
    /// to say what the *next* mode is, [`Mixer::blend`] has to know which of
    /// the three this one is, and a string it cannot exhaustively match is the
    /// wrong carrier for that. A `&str` would make the chip's arithmetic a
    /// comparison against three literals with a fourth case that has no
    /// answer.
    ///
    /// **So the type is the vocabulary's, and the harness converts.**
    /// `karakuri-console` already depends on `karakuri-operation`, and
    /// `examples/panel.rs` turns `Deck::blend`'s `karakuri_engine::deck::Blend`
    /// into one of these with a `match`. The failure that buys is worth
    /// stating: a fourth engine blend mode with no operation variant stops
    /// compiling **at the harness**, rather than drawing a word on a chip no
    /// control can reach and no map can ask for. That is the vocabulary doing
    /// its job — P-0074's *"the two rules … are not jointly satisfiable unless
    /// the vocabulary owns the lists."*
    ///
    /// The mock's own tooltip lists four — *"add, over, screen, multiply"* —
    /// and there are three. The disagreement was the mock's to settle and
    /// [ADR-0187](../../../docs/adr/0187-the-blend-mini-cycles-and-a-map-learns-the-three-it-cycles-through.md)
    /// settled it: `docs/manual/console.html` now lists the three that exist.
    pub blend: BlendMode,
    /// The mask in force — `Deck::mask(slot).kind()`. Its angle, position and
    /// softness are not drawn: `.mini` is a chip that says *which shape*, and
    /// three numbers about that shape are the inspector's row, not this one.
    ///
    /// **What a press on the chip counts from** ([`Mixer::mask`]), the way
    /// [`Strip::requested`] is what the tally's press counts from.
    pub mask: Mask,
    /// **The angle the mask is already wearing** — `Deck::mask(slot).angle()`,
    /// in radians. **Read to build an operation, and drawn nowhere.**
    ///
    /// # Why the strip carries a number no part of it paints
    ///
    /// [`Operation::SetMaskShape`] carries a shape **and an angle**, because
    /// the vocabulary's row is one an operator can say the whole of and a
    /// record is written whole
    /// ([ADR-0201](../../../docs/adr/0201-the-mask-is-two-rows-because-a-control-change-can-only-set.md)).
    /// The chip names the shape and nothing on this strip names the angle — so
    /// a press has to carry a value the control does not control, and the only
    /// honest one is **the value it already has**. Sending `0.0` would make
    /// choosing a shape silently straighten a diagonal wipe: a press that
    /// changed something nobody asked it to, which is the class of failure
    /// [ADR-0192](../../../docs/adr/0192-an-operation-asks-for-what-a-surface-can-say-and-the-record-stays-whole.md)
    /// and ADR-0201 are both about, arriving one layer further out.
    ///
    /// **It is [`Strip::requested`]'s arrangement exactly**: a value the strip
    /// carries because a press has to be computed from it, written by whoever
    /// owns the deck like every other field here, and never a value this crate
    /// keeps. See
    /// [ADR-0203](../../../docs/adr/0203-the-mask-chip-carries-the-angle-it-does-not-control.md).
    ///
    /// **Not the position and not the softness**, which the record also
    /// carries: neither is in the operation at all, and the half a shape
    /// operation does not ask for is filled in from a reading of the mask that
    /// is running, where the record is written — `karakuri_operation_record`'s
    /// `Current`. The angle is here because the *operation* names it; those
    /// two are not, because it does not.
    pub mask_angle: f32,
    /// **What the slot's meter last read**, or `None` for no reading at all.
    ///
    /// `Deck::level` is already `None` for *no meter, no measurement yet, a
    /// slot that is neither Live nor being auditioned, and a slot whose
    /// material was just replaced by a resize or a swap* — every case where a
    /// held reading would be about a different image. So `None` here draws the
    /// meter's well and nothing in it, which is the mock's own `alloc` strip:
    /// a `.vmeter` with no `b` and no `u` inside it.
    pub level: Option<Level>,
}

impl Strip {
    /// **Where this slot has been asked to go and has not got to**, or `None`
    /// for a slot that is where it was asked to be.
    ///
    /// The one derivation the pending presentation reads, and it answers a
    /// *word* rather than a `bool` because that is what the surface has to
    /// draw: P-0075's second clause is that the destination is identifiable
    /// from the surface itself, so the thing worth deriving is the destination
    /// and not the fact that there is one.
    ///
    /// # Why it is an inequality and not the engine's pair
    ///
    /// `Deck::is_parked` is exactly `requested == Residency::Priming &&
    /// effective == Residency::Allocated`, and today this answers `Some` on
    /// precisely those slots — `deck.rs`'s module doc closes the other cases
    /// itself: *"the governor may hold a slot below what was asked for, and
    /// may never put one above it"*, and *"effective Live and requested Live
    /// are the same set of slots"*. So the two forms agree slot for slot, and
    /// there is no frame on which they differ.
    ///
    /// They differ in what a **second** kind of disagreement would do to them.
    /// Written as the engine's pair, a surface matching `Priming` over
    /// `Allocated` draws a settled chip over any other outstanding request —
    /// an under-draw, silent, and exactly the failure P-0075 exists to name.
    /// Written as an inequality it draws the new one without being taught,
    /// because the presentation was never about *parked*: it is about a
    /// request that has not landed, which is
    /// [P-0060](../../../docs/principles/0060-name-the-property-not-the-shape.md)
    /// — the property rather than the one shape it currently takes.
    ///
    /// **`park` is still the word**, and it is the status line's: `karakuri-cli`
    /// spells it out for an operator in prose, which is the same clause met by
    /// a different means (ADR-0188).
    pub fn pending(&self) -> Option<Tally> {
        match self.requested == self.tally {
            true => None,
            false => Some(self.requested),
        }
    }

    /// **Where the trim is going and has not got to**, or `None` for a trim
    /// that is where it has been asked to be.
    ///
    /// [`Strip::pending`] one control along, and the same shape: the field is
    /// the request, the derivation is the third clause, and nothing is stored
    /// that says *a move is pending* — that fact is `self.gain_to` against
    /// `self.gain`, read every frame off values the harness rewrote this
    /// frame.
    ///
    /// # It compares what the track can show, and that is not the same as the
    /// raw pair
    ///
    /// The trim draws `[0, 1]` and gain is open above unity, so a scheduled
    /// move from 1.5 to 2.0 is two values with **one position** on this track
    /// ([`StripBox::trim_at`]). Compared raw it is pending; compared through
    /// [`unit`] it is not, and this compares through [`unit`].
    ///
    /// The reason is [`View::animating`] rather than the mark: a `Some` here
    /// declares a staleness, and a staleness bought for an animation that
    /// cannot move a pixel is the cost ADR-0193 refused to pay for a bay
    /// nobody can see, arriving through the other door. **It is still an
    /// under-draw and it is the trim's existing gap, not a new one** — the
    /// same track that cannot show 1.0 against 3.0 cannot show a move between
    /// them, and both close together, on the pass that lets a hand push the
    /// control past the top.
    ///
    /// A destination that is not a number is zero, which is [`unit`]'s rule
    /// and is where a NaN out of an engine stops being a `NaN`-wide band.
    pub fn gain_pending(&self) -> Option<f32> {
        self.gain_to
            .filter(|to| unit(*to) != unit(self.gain))
            .map(unit)
    }

    /// **Where the fader is going and has not got to**, on
    /// [`Strip::gain_pending`]'s terms.
    ///
    /// Opacity is a proportion, so the clamp is the identity on every value a
    /// deck can hold and the comparison is the plain one. It is written
    /// through [`unit`] anyway, because a harness writing a destination the
    /// engine never held is the case both of these are the last line of
    /// defence for — and two derivations that agree except in the case nobody
    /// tests are worse than one shape written twice.
    pub fn opacity_pending(&self) -> Option<f32> {
        self.opacity_to
            .filter(|to| unit(*to) != unit(self.opacity))
            .map(unit)
    }
}

/// **A fader, laid out**: the track, the length of it the value fills, and the
/// knob sitting on the value.
///
/// One type and one derivation for both of a strip's faders — see [`fader`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fader {
    /// The well: `.fader`'s 5px capsule lying down, or `.vfader`'s 17px one
    /// standing up.
    pub track: Rect,
    /// Which way it runs. [`Axis::Row`] fills from the left and
    /// [`Axis::Column`] fills from the **bottom**, because a fader stands up.
    pub axis: Axis,
    /// What the value fills, from the track's own zero.
    pub fill: Rect,
    /// The knob, centred on the fill's moving edge.
    pub knob: Rect,
    /// **How far the knob's centre moves between the two ends**: the track
    /// less the inset the fill sits inside it by, along the axis.
    ///
    /// A field, and not a fill stored beside its track — the thing
    /// [`StripBox`] refuses. The fill says where the value *is* and says
    /// nothing about how far it could go; the zero end is recoverable from the
    /// fill (a row's `fill.min.x`, a column's `fill.max.y`, neither of which
    /// moves) and the length is not recoverable from anything here. It is what
    /// turns a pointer back into a value — see [`Mixer::grab`] — so the
    /// derivation that draws the fader and the one that grabs it are one.
    pub travel: f32,
}

/// **A meter, laid out**: the well, the column the mean fills, and the peak's
/// mark.
///
/// # It is not a [`Fader`], and the difference is not the handle alone
///
/// They share exactly one thing and it is [`filled`] — a value turned into a
/// length up a track — which is why that is a function of its own with three
/// call sites rather than a shared type. What they do not share is what each
/// **is**: a fader's knob is a grab target and [`Mixer::grab`] is what makes
/// it one, while a peak mark is a reading and never will be. A shared type
/// would be a type half of whose fields are about a gesture the other half can
/// never have — and [`Fader::travel`], which is the length a *hand* moves the
/// value along, is the field that says so.
///
/// They disagree about the track as well. `.vfader b` sits
/// [`size::VFADER_INSET`] inside its well and `.vmeter b` fills its own edge
/// to edge; `.vfader`'s fill is a capsule and `.vmeter` is `overflow: hidden`
/// around a square column; and a knob is meant to stand proud of its track
/// where nothing in a meter may leave the well.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Meter {
    /// `.vmeter`'s 6px capsule.
    pub well: Rect,
    /// [`Level::mean`], up from the bottom.
    pub fill: Rect,
    /// [`Level::peak`], a [`size::VMETER_PEAK_H`] bar across the well.
    pub peak: Rect,
}

/// **A scheduled move on a fader, laid out**: the mark on the destination, and
/// how far this frame's attempt to get there has reached.
///
/// The presentation
/// [P-0075](../../../docs/principles/0075-a-pending-transition-shows-where-it-is-where-it-is-going-and-that-it-has-not-arrived.md)
/// is met with on a track, decided in
/// [ADR-0206](../../../docs/adr/0206-a-fader-marks-where-it-is-going-and-keeps-reaching-for-it.md):
/// **the knob and the fill go on saying where the control is, a mark says
/// where it is going, and the fill keeps setting off toward the mark and
/// falling back**. Nothing about the value moves, which is a stricter first
/// clause than the tally's roll manages — the word there is lifted off its
/// centre line and this is not.
///
/// # Two rectangles and no phase
///
/// It is a [`Fader`]'s neighbour and it is measured the same way: a pure
/// function of the two values and the displacement, with no clock and no
/// `Phase` in it — [`roll_at`] is applied by whoever is painting, exactly as
/// it is for the tally's word. A test asks for a reach at a displacement it
/// chose.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Reach {
    /// **The destination**, as a hairline across the track at the position the
    /// knob would sit at if the move had landed.
    ///
    /// **As long as the knob is wide**, so it is the knob's own footprint
    /// reduced to a line — and so it always has the same two pixels of strip
    /// to be read against that the knob stands proud on
    /// ([`size::VFADER_KNOB_OUT`]), whatever the fill is doing underneath. A
    /// mark the width of the *track* would cross the fill's own lavender end
    /// in its own colour on a move down from the top.
    ///
    /// It is painted **over** the knob rather than under it. Where the two
    /// nearly coincide the knob would otherwise swallow it — 9 pixels of knob
    /// on a 98-pixel travel is every move under about a tenth of the fader,
    /// and a quarter of the trim — and a mark that disappears exactly when the
    /// control is nearly there says *arrived* at the one moment it must not.
    pub mark: Rect,
    /// **What the reach has covered at this displacement**: from the value's
    /// own edge toward [`Reach::mark`], and never as far as it — [`ROLL_REACH`]
    /// of the way at the top of the travel, nothing at rest.
    ///
    /// Across the fill's own width rather than the track's, because it is the
    /// fill reaching. Empty at rest, which is a rectangle with no area and
    /// nothing painted.
    pub band: Rect,
}

/// **One strip's furniture**: a rectangle for each of the six things stacked
/// in it, and the two tracks a value rides.
///
/// The fills and the knobs are **not** fields, because each is a function of a
/// value this already knows where to put — see [`StripBox::trim_at`],
/// [`StripBox::fader_at`] and [`StripBox::meter_at`]. A fill stored beside its
/// track is two statements about one number.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StripBox {
    /// `.strip` itself: the 9px well everything else is inside.
    pub rect: Rect,
    /// `.strip-name`, the full width of the strip's content box because the
    /// CSS says `width: 100%`.
    pub name: Rect,
    /// `.tally`'s capsule, as wide as **the widest** of the three residency
    /// words inside its padding — the same box whichever one it is showing,
    /// so the chip does not resize when the deck moves and does not resize
    /// under a word rolling through it. See [`mixer`], where it is measured.
    ///
    /// The word is centred in it ([`tally_into`]), and the capsule is centred
    /// in the strip, so widening the box does not move the word: it grows
    /// symmetrically around type that was already on the strip's centre line.
    ///
    /// **It is the chip a press acts on** and not only the box a word is
    /// painted into — [`Mixer::tally`] hit-tests exactly this rectangle, the
    /// way [`StripBox::blend`] is hit-tested. Being the widest word's width
    /// rather than the shown word's is what the blend chip cannot say: this
    /// target stands still while the deck moves under it and while a word
    /// rolls through it.
    pub tally: Rect,
    /// The `g` in `.trim`.
    pub trim_label: Rect,
    /// `.trim`'s `.fader`: the horizontal track, [`size::FADER_H`] tall.
    pub trim: Rect,
    /// `.vfader`: the tall track, [`size::FADER_COL_H`] high.
    pub fader: Rect,
    /// `.vmeter`, beside it.
    pub meter: Rect,
    /// `.strip-num`: the opacity as a number.
    pub num: Rect,
    /// The blend `.mini`, which is **the chip a press acts on** and not only
    /// the box a word is painted into — [`Mixer::blend`] hit-tests exactly
    /// this rectangle. As wide as the word in it, inside `.mini`'s padding and
    /// border.
    pub blend: Rect,
    /// The mask `.mini`, which is **the chip a press acts on** and not only
    /// the box a mark is drawn into — [`Mixer::mask`] hit-tests exactly this
    /// rectangle, the way [`StripBox::blend`] and [`StripBox::tally`] are.
    /// It holds a mark rather than a word, so it is the same width whichever
    /// shape it is showing: a target that stands still while the deck moves
    /// under it, which the tally's capsule buys by being the widest word's and
    /// the blend chip cannot say at all.
    pub mask: Rect,
}

impl StripBox {
    /// **The trim at a gain**, which is [`fader`] on the horizontal track.
    ///
    /// The gain is shown over `[0, 1]`, and that range is read off
    /// `karakuri-midi`'s `GAIN_RANGE` rather than chosen here: *"`[0, 1]` for
    /// gain even though the mix is HDR and values above 1.0 are ordinary …
    /// a fader whose top is unity is what a fader means."* So a gain pushed
    /// past unity fills the track and stops, and this bay cannot yet show the
    /// difference between 1.0 and 3.0 — a real gap, and it belongs with the
    /// pass that lets a hand push the control past the top.
    pub fn trim_at(&self, gain: f32) -> Fader {
        fader(
            self.trim,
            Axis::Row,
            gain,
            0.0,
            egui::vec2(size::FADER_KNOB_W, size::FADER_KNOB_H),
        )
    }

    /// **The fader at an opacity**, which is [`fader`] on the vertical track.
    pub fn fader_at(&self, opacity: f32) -> Fader {
        fader(
            self.fader,
            Axis::Column,
            opacity,
            size::VFADER_INSET,
            egui::vec2(size::VFADER_KNOB_W, size::VFADER_KNOB_H),
        )
    }

    /// **The trim's scheduled move**, from the gain it is at to the one it was
    /// asked for, `rolled` of the way — which is [`reach`] on the horizontal
    /// track.
    ///
    /// Off [`StripBox::trim_at`] twice rather than off any arithmetic of its
    /// own: the destination's mark is *where the knob would be*, so it is the
    /// same derivation asked a second question, and a mark that drifted from
    /// the knob it stands for would be a fader with two ideas of what a value
    /// looks like.
    pub fn trim_reach(&self, gain: f32, to: f32, rolled: f32) -> Reach {
        reach(self.trim_at(gain), self.trim_at(to), rolled)
    }

    /// **The fader's scheduled move**, on [`StripBox::trim_reach`]'s terms and
    /// off [`StripBox::fader_at`].
    pub fn fader_reach(&self, opacity: f32, to: f32, rolled: f32) -> Reach {
        reach(self.fader_at(opacity), self.fader_at(to), rolled)
    }

    /// **The meter at a reading.**
    pub fn meter_at(&self, level: Level) -> Meter {
        // **The peak mark stays inside the well.** `.vmeter u` is placed by
        // its own `bottom`, so a 2px bar at a peak of 1.0 would sit from the
        // top of the well to two pixels above it — and `.vmeter` is
        // `overflow: hidden`, which clips it away exactly at the reading that
        // matters most. Its travel is the well's height less its own, which is
        // the clamp `Transport::beat` makes on the far end of a grid.
        let travel = (self.meter.height() - size::VMETER_PEAK_H).max(0.0);
        let top = self.meter.max.y - size::VMETER_PEAK_H - travel * unit(level.peak);
        Meter {
            well: self.meter,
            fill: filled(self.meter, Axis::Column, level.mean),
            peak: Rect::from_min_size(
                Pos2::new(self.meter.min.x, top),
                egui::vec2(self.meter.width(), size::VMETER_PEAK_H),
            ),
        }
    }
}

/// **The mixer's strips, laid out**: the values, and where each one goes.
///
/// # It borrows the values rather than carrying a copy
///
/// [`TransportRow`] carries the [`Transport`] it was measured from, for
/// [`Picture`]'s reason: whoever measured the type and whoever paints it are
/// then one statement, so a row laid out for one value and painted with
/// another cannot be written by accident. A [`Strip`] carries a name, so it is
/// not `Copy` and a copy per frame would be a `String` allocated per strip per
/// frame. The borrow says the same thing for nothing — these boxes were
/// measured from *these* strips — and the compiler holds it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mixer<'a> {
    /// **The values these rectangles were measured from**, in slot order.
    pub strips: &'a [Strip],
    /// Where each of them is. `None` past the last strip: a track on the page
    /// no deck fills, which is drawn as nothing at all.
    boxes: [Option<StripBox>; DECKS],
}

impl<'a> Mixer<'a> {
    /// How many strips there are — the deck's slot count, as far as one page
    /// of this bay reaches.
    pub fn count(&self) -> usize {
        self.boxes.iter().filter(|at| at.is_some()).count()
    }

    /// One strip's furniture. Panics on a strip this bay has not got, which is
    /// a caller having invented a slot — the rule [`TransportRow::dot`] states
    /// about a dot of a grid.
    pub fn strip(&self, index: usize) -> StripBox {
        let count = self.count();
        self.boxes
            .get(index)
            .copied()
            .flatten()
            .unwrap_or_else(|| panic!("strip {index} of a mixer of {count}"))
    }

    /// **What a press at `p` takes hold of**, or `None` where there is nothing
    /// under it that a hand can move.
    ///
    /// # It is the knob, and the track is deliberately not a target
    ///
    /// **A press on the track, off the knob, does nothing.** A fader at 0.3
    /// whose top is clicked would jump to 1.0 — a change to the mix nobody
    /// asked for, made on stage — and this bay's controls are played during a
    /// performance. So the answer is `None` there, and it is a decision rather
    /// than a hit test that stops at the knob by accident.
    ///
    /// It also decides who the *event* belongs to, since
    /// [`crate::input::claim`]'s rule 3 asks this: **the panel claims what it
    /// acts on**, so a press on a track goes to `egui`, which owns no widget
    /// there and does nothing with it — which is the same nothing, arrived at
    /// without the panel claiming a press it would throw away.
    ///
    /// # One derivation, asked twice
    ///
    /// [`crate::input::claim`] asks this and so does the caller that acts on
    /// the press, exactly as [`Outputs::op`] is asked after [`Outputs::hit`]
    /// (ADR-0176). Two copies of *where the knob is* is a control drawn where
    /// it cannot be grabbed, with nothing on screen saying so.
    ///
    /// **The value is part of the geometry here**, which the Outputs chip does
    /// not have to deal with: the knob sits on the fill's moving edge, so
    /// where it is depends on what the deck said this frame. That is the same
    /// [`Strip`] the bay was laid out from, so the knob a hand sees and the
    /// knob it grabs are the same one.
    pub fn grab(&self, p: karakuri_layout::Point) -> Option<Grab> {
        let p = Pos2::new(p.x, p.y);
        self.strips
            .iter()
            .zip(self.boxes)
            .enumerate()
            .find_map(|(index, (strip, at))| {
                let at = at?;
                // The manual's *deck* is the code's *slot*, and a deck holds
                // `MAX_SLOTS` of them — `DECKS`, which is 4 — so the index is
                // a `u8` with room to spare. See `Knob::operation`.
                let deck = index as u8;
                grabbed(at.trim_at(strip.gain), deck, Knob::Trim, p)
                    .or_else(|| grabbed(at.fader_at(strip.opacity), deck, Knob::Fader, p))
            })
    }

    /// **What a press at `p` asks the blend to become**, or `None` where
    /// there is no blend chip under it.
    ///
    /// # The chip cycles, and the operation names where it arrived
    ///
    /// Click it and the deck's blend moves to the next of
    /// [`BlendMode::ALL`], wrapping from the last back to the first. What
    /// comes out is [`Operation::SetBlendMode`] naming the **destination** —
    /// never a step, because there is no step in the vocabulary to name.
    ///
    /// That is P-0074's own worked example rather than an exception to it:
    /// *"A toggle is an affordance, built over operations by whoever draws the
    /// control, and it belongs there … a mini that cycles the blend is one
    /// control emitting three. The operator sees a toggle; the vocabulary
    /// never does."* The cycle is [`after`], which is four lines in this file
    /// and nothing at all in `karakuri-operation`.
    ///
    /// **What a MIDI map is offered is the three values, not the cycle** —
    /// [ADR-0187](../../../docs/adr/0187-the-blend-mini-cycles-and-a-map-learns-the-three-it-cycles-through.md),
    /// which is the decision this affordance forces and the reason it is
    /// recorded at all.
    ///
    /// # One derivation, asked twice, and the whole chip is the target
    ///
    /// [`crate::input::claim`]'s rule 3 asks this and so does the caller that
    /// acts on the press — the arrangement [`Outputs::op`] and [`Mixer::grab`]
    /// both have, where the derivation that claims a press is asked again
    /// rather than copied. [`StripBox::blend`] is the chip's own rectangle,
    /// the one the word is painted into, so a chip a hand sees and a chip it
    /// clicks are the same one.
    ///
    /// **The whole chip is the target and not the glyphs in it**, which is
    /// [`Outputs::sink`]'s rule one bay along: a 15px word is not something a
    /// hand finds, and `.mini`'s padding is what makes it one.
    ///
    /// # It names the strip's own deck
    ///
    /// The slot index, cast the way [`Mixer::grab`] casts it — the manual's
    /// *deck* is the code's *slot* (ADR-0180), and a deck holds `MAX_SLOTS` of
    /// them, so the index is a `u8` with room to spare.
    pub fn blend(&self, p: karakuri_layout::Point) -> Option<Operation> {
        let p = Pos2::new(p.x, p.y);
        self.strips
            .iter()
            .zip(self.boxes)
            .enumerate()
            .find_map(|(index, (strip, at))| {
                at.filter(|at| at.blend.contains(p))
                    .map(|_| Operation::SetBlendMode {
                        deck: index as u8,
                        blend: after(strip.blend),
                    })
            })
    }

    /// **What a press at `p` asks the residency to become**, or `None` where
    /// there is no tally chip under it.
    ///
    /// # The chip cycles, and it cycles from what was *requested*
    ///
    /// Click it and the deck is asked for the next of [`Tally::ALL`] —
    /// `live`, `prim`, `alloc`, wrapping — and what comes out is
    /// [`Operation::SetResidency`] naming that **destination**. The
    /// affordance is the blend chip's ([`Mixer::blend`], ADR-0187) and so is
    /// the division it rests on: the cycle is [`next`] here and nothing at all
    /// in `karakuri-operation`, which is P-0074's *"a toggle is an affordance,
    /// built over operations by whoever draws the control"*.
    ///
    /// **The step is taken from [`Strip::requested`] and not from
    /// [`Strip::tally`]**, and that is the decision rather than a detail. The
    /// two disagree exactly while a request has not landed, and cycling from
    /// the request is what makes that case come out right **with no case in
    /// the code for it**: a parked slot's request is `Priming`, so the next is
    /// `Allocated` — which *is* the withdrawal of the prime request, said by
    /// the ordinary arithmetic. Cycling from the effective residency would
    /// answer `Live` there, and a press meant to take a request back would put
    /// the deck on air.
    ///
    /// That is what
    /// [P-0076](../../../docs/principles/0076-a-surface-owns-the-affordance-never-the-authority.md)
    /// permits a surface: *"a press on a control whose transition is pending
    /// may ask for the withdrawal"* — a control choosing which destination a
    /// press names, arrived at out of one rule rather than a branch, and
    /// **not** a lock. The chip refuses nothing; every request is handed over
    /// and the engine decides.
    ///
    /// It is also what `karakuri-cli`'s `w` already does: `toggle_priming`
    /// reads `Deck::requested_residency` to choose its direction, so a parked
    /// slot's `w` withdraws rather than re-asking. Two surfaces reading
    /// different halves of the pair would disagree about what a press means
    /// on exactly the slots where it matters.
    ///
    /// # One derivation, asked twice, and the whole chip is the target
    ///
    /// [`crate::input::claim`]'s rule 3 asks this and so does the caller that
    /// acts on the press — [`Outputs::op`], [`Mixer::grab`] and
    /// [`Mixer::blend`] are the same arrangement. [`StripBox::tally`] is the
    /// capsule the word is painted into, and it is **the widest of the three
    /// words whatever it is showing** ([`mixer`]), so this target does not
    /// move when the deck moves under it and does not move while a word is
    /// rolling through it — which the blend chip, sized to the word it shows,
    /// cannot say.
    ///
    /// # It names the strip's own deck
    ///
    /// The slot index, cast the way [`Mixer::grab`] and [`Mixer::blend`] cast
    /// it — the manual's *deck* is the code's *slot* (ADR-0180), and a deck
    /// holds `MAX_SLOTS` of them, so the index is a `u8` with room to spare.
    pub fn tally(&self, p: karakuri_layout::Point) -> Option<Operation> {
        let p = Pos2::new(p.x, p.y);
        self.strips
            .iter()
            .zip(self.boxes)
            .enumerate()
            .find_map(|(index, (strip, at))| {
                at.filter(|at| at.tally.contains(p))
                    .map(|_| Operation::SetResidency {
                        deck: index as u8,
                        residency: residency(next(strip.requested)),
                    })
            })
    }

    /// **What a press at `p` asks the mask's shape to become**, or `None`
    /// where there is no mask mini under it.
    ///
    /// # The chip cycles, and the operation names where it arrived
    ///
    /// Click it and the deck's mask moves to the next shape — none, linear,
    /// radial, wrapping — and what comes out is
    /// [`Operation::SetMaskShape`] naming that **destination**. The affordance
    /// is the blend chip's ([`Mixer::blend`], ADR-0187) and the tally's
    /// (ADR-0195), and so is the division under it: the cycle is
    /// [`next_shape`] here and nothing at all in `karakuri-operation`, which
    /// is P-0074's *"a toggle is an affordance, built over operations by
    /// whoever draws the control"*.
    ///
    /// **The order starts at `None`**, which is
    /// `karakuri_engine::deck::MaskKind::ALL`'s and is written down there:
    /// *"`None` first, because it is the default and a cycle should start
    /// where a slot starts."* This crate has no engine (ADR-0156), so the
    /// order is restated in [`next_shape`] rather than read from it.
    ///
    /// # It carries the angle it does not control, and that is the decision
    ///
    /// [`Operation::SetMaskShape`] is a shape **and an angle**, and this chip
    /// names only the shape — *"`.mini` is a chip that says which shape, and
    /// three numbers about that shape are the inspector's row, not this one."*
    /// So the angle handed back is [`Strip::mask_angle`], the one the slot is
    /// already wearing: a press chooses a shape and changes nothing else.
    /// Sending `0.0` would make choosing a shape straighten a diagonal front,
    /// which is a surface asking for a change nobody made — see
    /// [ADR-0203](../../../docs/adr/0203-the-mask-chip-carries-the-angle-it-does-not-control.md).
    ///
    /// **The position and the softness are not here at all.** The operation
    /// does not name them, and the record that does is filled in from a
    /// reading of the running mask where the record is written (ADR-0201).
    /// A surface carrying a value no operation asks for would be this crate
    /// keeping half a deck.
    ///
    /// # One derivation, asked twice, and the whole chip is the target
    ///
    /// [`crate::input::claim`]'s rule 3 asks this and so does the caller that
    /// acts on the press — [`Outputs::op`], [`Mixer::grab`], [`Mixer::blend`]
    /// and [`Mixer::tally`] are the same arrangement. [`StripBox::mask`] is
    /// the chip's own rectangle, the one the mark is drawn into, and it is
    /// [`size::MINI_SIZE`] wide inside `.mini`'s padding whichever shape it is
    /// showing — so this target, like the tally's capsule and unlike the blend
    /// chip's, does not move under the value it draws.
    ///
    /// # It names the strip's own deck
    ///
    /// The slot index, cast the way [`Mixer::grab`], [`Mixer::blend`] and
    /// [`Mixer::tally`] cast it — the manual's *deck* is the code's *slot*
    /// (ADR-0180), and a deck holds `MAX_SLOTS` of them, so the index is a
    /// `u8` with room to spare.
    pub fn mask(&self, p: karakuri_layout::Point) -> Option<Operation> {
        let p = Pos2::new(p.x, p.y);
        self.strips
            .iter()
            .zip(self.boxes)
            .enumerate()
            .find_map(|(index, (strip, at))| {
                at.filter(|at| at.mask.contains(p))
                    .map(|_| Operation::SetMaskShape {
                        deck: index as u8,
                        kind: wipe_kind(next_shape(strip.mask)),
                        angle: strip.mask_angle,
                    })
            })
    }

    /// Every strip and its box, in slot order.
    pub fn placed(&self) -> impl Iterator<Item = (&'a Strip, StripBox)> {
        let boxes = self.boxes;
        self.strips
            .iter()
            .zip(boxes)
            .filter_map(|(strip, at)| at.map(|at| (strip, at)))
    }
}

/// **The Mixer bay's strips, derived**: one per slot the deck has, and none at
/// all where there is no deck.
///
/// # The page has [`DECKS`] tracks whatever the deck holds
///
/// `.mixer-strips` is `grid-template-columns: repeat(4, 1fr)`, and that four
/// is the same four [`DECKS`] is — `MAX_SLOTS` is 4, so no deck can fill a
/// fifth. **A strip is one of four tracks wide even where there is one
/// strip**, and that is read off the arrangement rather than chosen here: the
/// right pane's minimum width is written as *"four mixer strips still side by
/// side … four of them with three 4px gaps inside `.mixer-strips`' 6 + 6 is
/// 172"*. Tracks that followed the strip *count* would make a one-slot deck's
/// strip 244 wide in a pane sized for four 61-wide ones, and would re-derive
/// that minimum every time a slot was installed.
///
/// So a track with no strip in it **draws nothing at all** — not an empty
/// strip. That is the opposite of what [`preview`] does with a cell that is
/// off, and the two are not in tension: a preview cell is the region's
/// own face and says `off`, where a strip is six readings and an empty one is
/// six readings nobody took. The mock draws such a strip — `.strip.empty`,
/// with `—` for a name, `empty` for a tally and both tracks bare — and
/// inventing one is ADR-0177's row of zeroes with a different glyph.
///
/// **The example's deck has one slot, so it draws one strip**, and that is the
/// example rather than a gap in it — exactly as three of its preview cells
/// read `off`.
///
/// # What is in the mock's bay and is deliberately not here
///
/// - **The head's `4 of 4 · page 1`.** *"The strip is a paged list whose
///   length is a number, and the header says which page you are on"* — so both
///   halves of that pill are about paging, and there is none. Every strip the
///   deck has is drawn, on the one page, so the pill could only ever read
///   `n of n · page 1`: two numbers that are always equal and a third that is
///   always 1. That is [`Kind::Bay`]'s own rule about a pill stating a value
///   the console does not have, and the number that *is* known — how many
///   strips there are — is on the face of the bay already. It arrives with
///   paging.
/// - **`.xfade`, the whole crossfade row under the strips**: an A/B track with
///   a handle on it, and `wipe`, `iris`, `next bar`, `8 beats` and `go`. That
///   is a transition being armed and then fired, and every one of them is a
///   control over machinery the console cannot reach — nothing here holds what
///   is armed, and `go` would fire nothing. It is 61 of the bay's 316 and it
///   stays empty, exactly as six of the transport row's ten items do.
/// - **`.strip.focus` and `.wfocus`, which are the two focuses**: the deck
///   selection, which persists and is what a key press is addressed to, and
///   keyboard focus, which is transient. The mock draws them differently on
///   purpose — a solid lavender ring and a dashed sun outline — because
///   *"drawing them the same erases which of the two a reader is looking at"*.
///   **Neither exists in this console**: nothing here selects a deck and
///   nothing takes keyboard focus, so a ring would be drawn around a state
///   that is not kept.
/// - **Every tooltip.** Four of this bay's controls carry one, and a tooltip
///   needs `egui` to own a widget — the sentence [`outputs`] writes about a
///   control, one bay along.
///
/// `layout` must be solved: [`Layout::rect`] refuses to answer from a dirty
/// one. `ctx` is asked for the type, because the tally's capsule and the
/// blend's mini are as wide as the words in them.
pub fn mixer<'a>(
    ctx: &egui::Context,
    layout: &karakuri_layout::Layout,
    strips: &'a [Strip],
) -> Option<Mixer<'a>> {
    // **No deck behind the console, so there are no strips.** Every test in
    // this crate is here, and so is the whole of `cargo test -p
    // karakuri-console`. Drawing four empty strips would be inventing six
    // readings a slot; this is `View::picture`'s rule, one bay along.
    if strips.is_empty() {
        return None;
    }
    // Fonts are not valid until `egui` has run a pass, exactly as in
    // `outputs` and `transport`.
    if ctx.cumulative_pass_nr() == 0 {
        return None;
    }
    let region = to_egui(layout.rect(layout.find("mixer")?));
    let row = strips_row(region)?;
    let width = |job: LayoutJob| ctx.fonts_mut(|f| f.layout_job(job).size().x);
    let label = width(span_at(
        TRIM_LABEL,
        size::TRIM_LABEL_SIZE,
        Color32::PLACEHOLDER,
    ));
    // **The chip is as wide as the widest residency, not as the one it is
    // showing.** Measured once for the bay rather than per strip: it is the
    // same three words in every strip, so a per-strip measurement would be
    // three galley layouts a strip for one answer.
    //
    // It read `strip.tally` alone until the tally learned to say that a
    // request had not landed, and that was a defect rather than a
    // simplification: a chip sized to the current word is a capsule that
    // changes width when the deck moves under it, and one that changes width
    // *while a word is rolling through it* would be a control resizing on its
    // own animation. What the mock does not have is the reason it survived —
    // a still page draws each chip once, so shrink-to-fit and this are the
    // same picture there and only one of them is the same picture over time.
    let tally = Tally::ALL
        .into_iter()
        .map(|tally| width(tally_job(tally, Color32::PLACEHOLDER)))
        .fold(0.0f32, f32::max);
    let mut boxes = [None; DECKS];
    for (index, strip) in strips.iter().take(DECKS).enumerate() {
        let blend = width(span_at(
            strip.blend.name(),
            size::MINI_SIZE,
            Color32::PLACEHOLDER,
        ));
        boxes[index] = strip_box(
            track(row, DECKS, index, size::STRIP_GAP, Axis::Row),
            label,
            tally,
            blend,
        );
    }
    Some(Mixer { strips, boxes })
}

/// **Where the strips go**: the `.mixer-strips` grid inside a mixer region.
///
/// The region less [`size::HEAD_H`] for the bay head painted over the top of
/// it, inset by [`size::STRIPS_PAD`] left, right and top, and **exactly
/// [`size::STRIP_H`] tall** rather than whatever is left over. The bay is
/// taller than its strips by design — `.xfade` is the other 61 of it — and a
/// mixer with room to grow (it is the only visible child of a soloed right
/// pane, and a fixed child with room takes it, ADR-0157) grows the bay and not
/// the strips: a `.strip` is a column of fixed type around a `.fader-col`
/// whose 104 is stated in the CSS, so there is nothing in it that gets bigger
/// any more than there is anything that gets smaller.
///
/// `None` where the region cannot hold it — folded away, soloed away, or a
/// window too small — which is [`picture_rect`]'s rule stated on a row of
/// strips.
fn strips_row(region: Rect) -> Option<Rect> {
    let row = Rect::from_min_size(
        Pos2::new(
            region.min.x + size::STRIPS_PAD,
            region.min.y + size::HEAD_H + size::STRIPS_PAD,
        ),
        egui::vec2(region.width() - size::STRIPS_PAD * 2.0, size::STRIP_H),
    );
    match row.width() > 0.0 && region.contains_rect(row) {
        true => Some(row),
        false => None,
    }
}

/// The arithmetic of one strip, away from the type it measures and the layout
/// it reads.
///
/// Term for term from `.strip` and what is in it, in `style.css`:
///
/// - `.strip { display: flex; flex-direction: column; align-items: center;
///   gap: 5px; padding: 7px 4px }` — six things stacked from the top of the
///   strip's content box, one [`size::STRIP_GAP_Y`] between each pair, each
///   **centred across the strip** rather than filling it. Two are the
///   exception, and the CSS states both: `.strip-name` and `.trim` are
///   `width: 100%`.
/// - `.trim { gap: 5px; padding: 0 3px }` with `.trim .fader { flex: 1 }` —
///   the `g`, then the track, which takes what is left and is centred in the
///   row because the label is the taller of the two.
/// - `.fader-col { display: flex; gap: 6px; height: 104px }` around a
///   `.vfader` of 17 and a `.vmeter` of 6, both `height: 100%` — so the column
///   is 29 wide, centred, and its `align-items: flex-end` has nothing left to
///   align.
/// - `.strip-mode { gap: 3px }` — two minis, centred.
///
/// `None` where the track is too narrow to hold what is in it, which is
/// [`picture_rect`]'s rule stated on a strip: the fader column is the widest
/// fixed thing in it, and a strip that cannot hold that has no readings to
/// show. At the right pane's own minimum of 172 a track is exactly 37 and the
/// column is exactly 29 inside 4 + 4 of padding, so the arrangement's minimum
/// and this are one number or neither.
fn strip_box(track: Rect, label_w: f32, tally_w: f32, blend_w: f32) -> Option<StripBox> {
    let inner = Rect::from_min_max(
        Pos2::new(
            track.min.x + size::STRIP_PAD_X,
            track.min.y + size::STRIP_PAD_Y,
        ),
        Pos2::new(
            track.max.x - size::STRIP_PAD_X,
            track.max.y - size::STRIP_PAD_Y,
        ),
    );
    let column_w = size::VFADER_W + size::FADER_COL_GAP + size::VMETER_W;
    if inner.width() < column_w {
        return None;
    }
    // Each row starts one gap after the one before it ended, and is as tall as
    // its own type — which is what a flex column is.
    let mut y = inner.min.y;
    let mut row = |h: f32| {
        let at = Rect::from_min_size(Pos2::new(inner.min.x, y), egui::vec2(inner.width(), h));
        y = at.max.y + size::STRIP_GAP_Y;
        at
    };
    let name = row(size::STRIP_NAME_SIZE * size::LINE);
    let tally = centred_in(row(size::TALLY_H), tally_w + size::TALLY_PAD_X * 2.0);
    let trim = row(size::TRIM_H);
    let column = centred_in(row(size::FADER_COL_H), column_w);
    let num = row(size::STRIP_NUM_SIZE * size::LINE);
    let blend_w = blend_w + size::MINI_PAD_X * 2.0 + size::HAIRLINE * 2.0;
    // The mask's mini holds a mark rather than a word, and the mark is drawn
    // at the size the glyph it stands in for would have been — see `Mask`.
    let mask_w = size::MINI_SIZE + size::MINI_PAD_X * 2.0 + size::HAIRLINE * 2.0;
    let mode = centred_in(row(size::MINI_H), blend_w + size::MODE_GAP + mask_w);

    let trim_label = Rect::from_min_size(
        Pos2::new(trim.min.x + size::TRIM_PAD_X, trim.min.y),
        egui::vec2(label_w, trim.height()),
    );
    let trim_track = Rect::from_min_size(
        Pos2::new(
            trim_label.max.x + size::TRIM_GAP,
            trim.center().y - size::FADER_H * 0.5,
        ),
        egui::vec2(
            trim.max.x - size::TRIM_PAD_X - trim_label.max.x - size::TRIM_GAP,
            size::FADER_H,
        ),
    );
    match trim_track.width() > 0.0 {
        true => Some(StripBox {
            rect: track,
            name,
            tally,
            trim_label,
            trim: trim_track,
            fader: Rect::from_min_size(column.min, egui::vec2(size::VFADER_W, column.height())),
            meter: Rect::from_min_size(
                Pos2::new(column.max.x - size::VMETER_W, column.min.y),
                egui::vec2(size::VMETER_W, column.height()),
            ),
            num,
            blend: Rect::from_min_size(mode.min, egui::vec2(blend_w, mode.height())),
            mask: Rect::from_min_size(
                Pos2::new(mode.max.x - mask_w, mode.min.y),
                egui::vec2(mask_w, mode.height()),
            ),
        }),
        false => None,
    }
}

/// A box `w` wide centred across `row`, which is `align-items: center` on one
/// child of a flex column.
fn centred_in(row: Rect, w: f32) -> Rect {
    Rect::from_min_size(
        Pos2::new(row.center().x - w * 0.5, row.min.y),
        egui::vec2(w, row.height()),
    )
}

/// **How much of a track a value fills**, from the track's own zero — the left
/// end of a row, and the **bottom** of a column, because a fader stands up and
/// a meter fills from the floor.
///
/// **The whole of what a [`Fader`] and a [`Meter`] share**, and it is a
/// function rather than a shared type for the reason [`Meter`] gives. Three
/// call sites the day it is written: the trim's fill, the opacity fader's
/// fill, and the meter's column.
fn filled(track: Rect, axis: Axis, at: f32) -> Rect {
    let at = unit(at);
    match axis {
        Axis::Row => Rect::from_min_max(
            track.min,
            Pos2::new(track.min.x + track.width() * at, track.max.y),
        ),
        Axis::Column => Rect::from_min_max(
            Pos2::new(track.min.x, track.max.y - track.height() * at),
            track.max,
        ),
    }
}

/// **The fader, and it is one component with two call sites on the day it is
/// written**: the horizontal trim and the vertical opacity. That is this
/// repository's rule about an abstraction satisfied when it lands rather than
/// promised for later, which [`bay_head`] is the other instance of.
///
/// What the two disagree about is an argument each — the axis, the inset the
/// fill sits inside its track by (`.fader b` fills its 5px track edge to edge
/// where `.vfader b` is `left: 3px; right: 3px; bottom: 3px` inside its 17px
/// one), and the knob's size. Everything else is this.
///
/// **One number drives the fill and the knob**, which is what stops the two
/// disagreeing: the knob is centred on the fill's moving edge. The mock sets
/// them by hand and a few percent apart — `width: 72%` with `left: 66%`,
/// `height: 97%` with `bottom: 94%` — which is an author centring a 9px knob
/// on a fill's end in percentages, and this is the same mark with the
/// arithmetic done once.
///
/// So the knob overhangs its track by half its length at either end, and the
/// mock's own boxes have the room: [`size::STRIP_GAP_Y`] above and below the
/// fader column, and [`size::TRIM_PAD_X`] plus [`size::STRIP_PAD_X`] either
/// side of the trim.
fn fader(track: Rect, axis: Axis, at: f32, inset: f32, knob: egui::Vec2) -> Fader {
    let inside = track.shrink(inset);
    let fill = filled(inside, axis, at);
    let edge = match axis {
        Axis::Row => Pos2::new(fill.max.x, track.center().y),
        Axis::Column => Pos2::new(track.center().x, fill.min.y),
    };
    Fader {
        track,
        axis,
        fill,
        knob: Rect::from_center_size(edge, knob),
        travel: match axis {
            Axis::Row => inside.width(),
            Axis::Column => inside.height(),
        },
    }
}

/// **A scheduled move on a laid-out fader**: the mark on where it is going,
/// and the band reaching `rolled` of the way from where it is.
///
/// **Two faders in and no value arithmetic**, which is what keeps this honest:
/// `now` and `to` are the same track measured at the two values, so the mark
/// lands exactly where the knob would and the band starts exactly where the
/// fill ends. The knob's centre is the fill's moving edge — [`fader`] says so
/// — and that one number is the whole of what this reads out of each.
///
/// **The displacement is a fraction of the gap rather than a distance**, so
/// the reach is in proportion to the move: a fade across the fader sets off a
/// long way and a fade of a hundredth sets off a pixel. That is the honest
/// picture and it is also the limit — under about a hundredth of the travel
/// the whole disagreement is a pixel, and what carries the message there is
/// the mark rather than the motion. See
/// [ADR-0206](../../../docs/adr/0206-a-fader-marks-where-it-is-going-and-keeps-reaching-for-it.md).
///
/// Two call sites the day it is written, which is this repository's rule about
/// an abstraction: the trim and the opacity fader, exactly as [`fader`] itself
/// has.
fn reach(now: Fader, to: Fader, rolled: f32) -> Reach {
    let (from, dest) = (now.knob.center(), to.knob.center());
    match now.axis {
        Axis::Row => {
            let head = from.x + (dest.x - from.x) * rolled;
            Reach {
                mark: Rect::from_center_size(
                    Pos2::new(dest.x, now.track.center().y),
                    egui::vec2(size::HAIRLINE, now.knob.height()),
                ),
                band: Rect::from_min_max(
                    Pos2::new(from.x.min(head), now.fill.min.y),
                    Pos2::new(from.x.max(head), now.fill.max.y),
                ),
            }
        }
        Axis::Column => {
            let head = from.y + (dest.y - from.y) * rolled;
            Reach {
                mark: Rect::from_center_size(
                    Pos2::new(now.track.center().x, dest.y),
                    egui::vec2(now.knob.width(), size::HAIRLINE),
                ),
                band: Rect::from_min_max(
                    Pos2::new(now.fill.min.x, from.y.min(head)),
                    Pos2::new(now.fill.max.x, from.y.max(head)),
                ),
            }
        }
    }
}

/// **One laid-out fader, taken hold of at `p`** — or `None` where `p` is not
/// on its knob.
///
/// The inverse of [`fader`], off the same three numbers it laid out: the
/// track's zero end, which is the fill's fixed edge; the travel, which is the
/// length the value rides; and where the knob's centre is now, which is the
/// fill's moving edge. **The offset is the pointer less that centre**, so a
/// press keeps whatever it grabbed at and the value does not jump.
fn grabbed(fader: Fader, deck: u8, knob: Knob, p: Pos2) -> Option<Grab> {
    if !fader.knob.contains(p) {
        return None;
    }
    // Which end is zero is [`filled`]'s rule, read backwards: a row fills from
    // the left and a column from the **bottom**, because a fader stands up.
    let (zero, edge, coord) = match fader.axis {
        Axis::Row => (fader.fill.min.x, fader.fill.max.x, p.x),
        Axis::Column => (fader.fill.max.y, fader.fill.min.y, p.y),
    };
    Grab::new(deck, knob, fader.axis, zero, fader.travel, coord - edge)
}

/// **The next blend mode round the cycle**, wrapping from the last back to the
/// first — and the whole of the affordance the blend chip is.
///
/// It is four lines here and nothing at all in `karakuri-operation`, which is
/// P-0074's division: *"A toggle is an affordance, built over operations by
/// whoever draws the control, and it belongs there."* The vocabulary owns the
/// three values; this owns the order a pointer walks them in.
///
/// **A match rather than an index into [`BlendMode::ALL`]**, for the reason
/// [`BlendMode::name`] is one: a fourth mode does not compile until somebody
/// says what follows it. The cost is that the order is written twice — here
/// and in `ALL` — so `tests/blend.rs` walks `ALL` through this and asserts
/// they are the same cycle, which is the measurement that keeps the two from
/// drifting rather than a comment promising they will not.
fn after(blend: BlendMode) -> BlendMode {
    match blend {
        BlendMode::Add => BlendMode::Over,
        BlendMode::Over => BlendMode::Max,
        BlendMode::Max => BlendMode::Add,
    }
}

/// **The next residency round the cycle**, wrapping from the last back to the
/// first — the whole of the affordance the tally chip is, and the order the
/// mock's own tooltip lists: *"one of three residencies — live, priming,
/// allocated"*.
///
/// [`after`]'s division, one control along: the cycle is three lines here and
/// nothing in `karakuri-operation`, which owns the three values and not the
/// order a pointer walks them in (P-0074).
///
/// **A match rather than an index into [`Tally::ALL`]**, for [`after`]'s
/// reason: a fourth residency does not compile until somebody says what
/// follows it. The price is that the order is written twice — here and in
/// `ALL` — so `tests/tally.rs` walks `ALL` through this and asserts they are
/// the same cycle.
///
/// **What it is asked about is [`Strip::requested`]**, and why is
/// [`Mixer::tally`].
fn next(tally: Tally) -> Tally {
    match tally {
        Tally::Live => Tally::Priming,
        Tally::Priming => Tally::Allocated,
        Tally::Allocated => Tally::Live,
    }
}

/// **The console's word for a residency, as the vocabulary's** — and it is the
/// whole of what the console has to know about the difference.
///
/// [`Tally`] is the mock's `.tally` and the engine's `Residency` seen from the
/// surface; [`karakuri_operation::Residency`] is what an operation may name.
/// They are the same three states and two crates' words for them, so the
/// translation is a match — and a match rather than a cast so that a fourth
/// state on either side stops the build here, where the two lists meet, rather
/// than at a chip drawing a word no operation can carry.
///
/// The mirror image of it is `examples/panel.rs`'s `tally`, which turns the
/// *engine's* `Residency` into a [`Tally`] on the way in. Three names for
/// three states is the cost `karakuri-operation` pays for depending on nothing
/// (P-0074), and this is one of the two places it is paid.
fn residency(tally: Tally) -> Residency {
    match tally {
        Tally::Live => Residency::Live,
        Tally::Priming => Residency::Priming,
        Tally::Allocated => Residency::Allocated,
    }
}

/// **The next mask shape round the cycle**, wrapping from the last back to the
/// first — the whole of the affordance the mask mini is.
///
/// [`after`]'s division, one control along: the cycle is three lines here and
/// nothing in `karakuri-operation`, which owns the three shapes and not the
/// order a pointer walks them in (P-0074).
///
/// **A match, for [`after`]'s reason**: a fourth shape does not compile until
/// somebody says what follows it. Unlike [`after`] and [`next`] the price is
/// not a second copy of the order — [`Mask`] has no `ALL` and neither does
/// `karakuri_operation::WipeKind`, because nothing reads one (see [`Mask`]) —
/// so this is the only statement of the order in this crate, and
/// `tests/mask.rs` is what checks it against the order it is a copy of.
///
/// **It starts at [`Mask::None`]**, which is `karakuri_engine::deck::MaskKind::ALL`'s
/// own order and its own reason: *"`None` first, because it is the default and
/// a cycle should start where a slot starts."* A cycle that began at `Linear`
/// would be a control whose first press on a fresh slot moves it somewhere it
/// has never been.
fn next_shape(mask: Mask) -> Mask {
    match mask {
        Mask::None => Mask::Linear,
        Mask::Linear => Mask::Radial,
        Mask::Radial => Mask::None,
    }
}

/// **The console's word for a mask shape, as the vocabulary's** — [`residency`]
/// one control along, and for its reason exactly.
///
/// [`Mask`] is the mock's second `.mini` and the engine's `MaskKind` seen from
/// the surface; [`karakuri_operation::WipeKind`] is what an operation may name.
/// They are the same three shapes and two crates' words for them, so the
/// translation is a match — and a match rather than a cast so that a fourth
/// shape on either side stops the build here, where the two lists meet, rather
/// than at a chip drawing a mark no operation can carry.
///
/// The mirror image of it is `examples/panel.rs`'s reading of
/// `Deck::mask(slot).kind()`, which turns the *engine's* `MaskKind` into a
/// [`Mask`] on the way in.
fn wipe_kind(mask: Mask) -> WipeKind {
    match mask {
        Mask::None => WipeKind::None,
        Mask::Linear => WipeKind::Linear,
        Mask::Radial => WipeKind::Radial,
    }
}

/// The tally's word as one laid-out run, so that measuring it and painting it
/// cannot be two different runs of type.
fn tally_job(tally: Tally, colour: Color32) -> LayoutJob {
    spaced(
        &tally.word().to_uppercase(),
        size::TALLY_SIZE,
        colour,
        size::TALLY_TRACKING,
    )
}

/// A plain span at a size that is not the console's [`size::BASE`] — this bay
/// has four of them, which is why it takes one.
fn span_at(text: &str, size: f32, colour: Color32) -> LayoutJob {
    LayoutJob::simple_singleline(
        text.to_owned(),
        FontId::new(size, FontFamily::Proportional),
        colour,
    )
}

/// `.strip-name` as one laid-out run: `overflow: hidden; text-overflow:
/// ellipsis; white-space: nowrap` is exactly one row, broken anywhere, with an
/// ellipsis standing for what did not fit.
fn name_job(name: &str, width: f32, colour: Color32) -> LayoutJob {
    let mut job = span_at(name, size::STRIP_NAME_SIZE, colour);
    job.wrap = egui::epaint::text::TextWrapping {
        max_width: width,
        max_rows: 1,
        break_anywhere: true,
        overflow_character: Some('…'),
    };
    job
}

/// **The Mixer bay's strips, painted.**
///
/// Where everything goes is [`mixer`]'s, so this paints and derives nothing.
fn mixer_into(ui: &Ui, pal: &Palette, mixer: &Mixer, phase: Phase) {
    for (strip, at) in mixer.placed() {
        strip_into(ui, pal, strip, at, phase);
    }
}

/// **One strip**, term for term from `style.css`:
///
/// - `.strip` — `background: var(--c-well)`, `border-radius: 9px`.
/// - `.strip-name` — `font-size: 10px`, `var(--c-dim)`, centred, elided.
/// - `.tally` — three washes and three inks, which is [`tally_into`]'s.
/// - `.trim .lbl` — `font-size: 9px`, `var(--c-faint)`.
/// - `.fader` and `.vfader` — [`fader_into`]'s.
/// - `.vmeter` — [`meter_into`]'s.
/// - `.strip-num` — `var(--c-text)`, *a value*, and `font-weight: 500` is not
///   honoured because `egui`'s default proportional face has no bold.
/// - `.strip-mode` — two [`mini_into`]s.
fn strip_into(ui: &Ui, pal: &Palette, strip: &Strip, at: StripBox, phase: Phase) {
    // Clipped to the strip and half the gap around it: a fader's knob is
    // meant to stand proud of its track, and `.mixer-strips`' 4px gap is where
    // that goes — but nothing in one strip may reach the strip beside it.
    let painter = ui
        .painter()
        .with_clip_rect(at.rect.expand(size::STRIP_GAP * 0.5));
    painter.rect_filled(
        at.rect,
        CornerRadius::same(size::STRIP_RADIUS as u8),
        pal.well,
    );

    if !strip.name.is_empty() {
        let galley = painter.layout_job(name_job(&strip.name, at.name.width(), pal.dim));
        centre_galley(&painter, at.name, galley, pal.dim);
    }

    tally_into(&painter, pal, at.tally, strip.tally, strip.pending(), phase);

    let galley = painter.layout_job(span_at(TRIM_LABEL, size::TRIM_LABEL_SIZE, pal.faint));
    painter.galley(
        Pos2::new(
            at.trim_label.min.x,
            at.trim_label.center().y - galley.size().y * 0.5,
        ),
        galley,
        pal.faint,
    );
    // **The same displacement for both faders and for the chip above them**,
    // because it is the same phase: P-0075's *everything pending moves
    // together*, which on this strip is three presentations reading one
    // number. A strip with a fade on each fader reaches twice, in step.
    let rolled = roll_at(phase);
    fader_into(
        &painter,
        pal,
        at.trim_at(strip.gain),
        false,
        strip
            .gain_pending()
            .map(|to| at.trim_reach(strip.gain, to, rolled)),
    );
    fader_into(
        &painter,
        pal,
        at.fader_at(strip.opacity),
        strip.tally == Tally::Live,
        strip
            .opacity_pending()
            .map(|to| at.fader_reach(strip.opacity, to, rolled)),
    );
    meter_into(&painter, pal, at.meter, strip.level.map(|l| at.meter_at(l)));

    // `.strip-num`: the opacity to two places, which is what the mock writes.
    // The value itself and not the fader's clamped one — the fader clamps
    // because a track has ends and a number does not.
    let galley = painter.layout_job(span_at(
        &format!("{:.2}", strip.opacity),
        size::STRIP_NUM_SIZE,
        pal.text,
    ));
    centre_galley(&painter, at.num, galley, pal.text);

    mini_into(&painter, pal, at.blend, true, |painter, colour| {
        let galley = painter.layout_job(span_at(strip.blend.name(), size::MINI_SIZE, colour));
        centre_galley(painter, at.blend, galley, colour);
    });
    mini_into(&painter, pal, at.mask, false, |painter, colour| {
        mask_mark(painter, at.mask.center(), colour, strip.mask);
    });
}

/// A galley centred in a box, both ways — which is `align-items: center` on a
/// flex column done where the type's real height is known.
fn centre_galley(
    painter: &egui::Painter,
    rect: Rect,
    galley: std::sync::Arc<egui::Galley>,
    colour: Color32,
) {
    painter.galley(
        Pos2::new(
            rect.center().x - galley.size().x * 0.5,
            rect.center().y - galley.size().y * 0.5,
        ),
        galley,
        colour,
    );
}

/// `.tally`: a capsule with the residency's word in it, in the residency's own
/// colours.
///
/// - `.tally.live` — `color-mix(in srgb, var(--c-pink) 18%, transparent)`
///   behind `var(--c-pink)`, with `box-shadow: 0 0 10px var(--c-glowp)`. The
///   halo is an [`egui::epaint::Shadow`], the mechanism the transport's lit
///   beat and the Outputs row's dot both use.
/// - `.tally.priming` — a 20% wash of `var(--c-sun)` behind `var(--c-sun)`,
///   and **no halo**: priming is warming out of sight, not on air.
/// - `.tally.alloc` — `var(--c-tint)` behind `var(--c-dim)`, which is the
///   palette's own *"the wash behind a node head, and the allocated tally"* —
///   the first use of `--c-tint` in this crate, and it was transcribed against
///   this day.
///
/// # And the roll, where the request has not landed
///
/// `pending` is [`Strip::pending`] — the residency this slot was asked for and
/// has not reached. While it is `Some`, the word rolls part of the way toward
/// it and falls back, once a second, and never arrives: [`roll_at`] is the
/// displacement and [ADR-0190](../../../docs/adr/0190-the-parked-tally-rolls-because-two-lamps-do-not-fit-in-fifty-three-pixels.md)
/// is why it is a roll and not two lamps side by side — 53 pixels, and the
/// pair the rule exists for is 85.125 of them.
///
/// # The clip is new, not narrowed
///
/// Nothing called `with_clip_rect` on a tally before this: the chip painted a
/// filled rect and a galley that fitted inside it, so there was nothing to
/// clip and no clip to get wrong. A second word travelling through the box is
/// the first thing here that is drawn to be cut off, and the cut is what makes
/// the roll a roll rather than two words overlapping the trim row underneath.
/// It is introduced deliberately and it is on the type alone — see the note at
/// the clip itself, because the halo has to go on spilling.
fn tally_into(
    painter: &egui::Painter,
    pal: &Palette,
    rect: Rect,
    tally: Tally,
    pending: Option<Tally>,
    phase: Phase,
) {
    let radius = CornerRadius::same((size::TALLY_H * 0.5) as u8);
    let ink = |tally| match tally {
        Tally::Live => (tint(pal.pink, 18), pal.pink),
        Tally::Priming => (tint(pal.sun, 20), pal.sun),
        Tally::Allocated => (pal.tint, pal.dim),
    };
    let (fill, ink_now) = ink(tally);
    if tally == Tally::Live {
        painter.add(
            egui::epaint::Shadow {
                offset: [0, 0],
                blur: size::TALLY_GLOW,
                spread: 0,
                color: pal.glow_pink,
            }
            .as_shape(rect, radius),
        );
    }
    painter.rect_filled(rect, radius, fill);

    // **The clip is the capsule, and it is on the words alone.** The halo is
    // drawn to spill — `.tally.live`'s `box-shadow: 0 0 10px` is 10px of it
    // outside the box — so a clip taken before the shadow would trim the one
    // shape in this chip that is meant to leave it. Everything after this
    // line is type that may be halfway out of the box on purpose.
    let painter = painter.with_clip_rect(rect.intersect(painter.clip_rect()));

    let galley = painter.layout_job(tally_job(tally, ink_now));
    // **The pitch is the travel's, not the geometry's**, and it is at least
    // the box: at the natural row pitch the two words would both be partly
    // visible with nothing between them, which at 9px is mud. One box height
    // apart leaves a blank band of exactly the slack the chip already has —
    // 13.5 less a 10.0 ink row is 3.5 — for every displacement, because the
    // band is the difference of two constants and not a function of how far
    // the roll has got. It costs the chip nothing: the second word is
    // outside the capsule at rest and clipped away.
    let pitch = size::TALLY_H.max(galley.size().y);
    let rolled = match pending {
        Some(_) => roll_at(phase) * pitch,
        None => 0.0,
    };
    let word_into = |painter: &egui::Painter, galley: std::sync::Arc<egui::Galley>, colour, dy| {
        painter.galley(
            Pos2::new(
                rect.center().x - galley.size().x * 0.5,
                rect.center().y - galley.size().y * 0.5 + dy,
            ),
            galley,
            colour,
        );
    };
    word_into(&painter, galley, ink_now, -rolled);
    // **The destination in its own ink**, which is the second half of what is
    // being said: the word names where the slot is going and the colour is the
    // one that slot will be drawn in when it gets there. It comes up from
    // below — one pitch under the settled word — so a still frame of a chip
    // that is not rolling is the chip as it was.
    if let Some(to) = pending {
        let (_, ink_to) = ink(to);
        let galley = painter.layout_job(tally_job(to, ink_to));
        word_into(&painter, galley, ink_to, pitch - rolled);
    }
}

/// A fader: the well, the fill and the knob.
///
/// - `.fader` / `.vfader` — `background: var(--c-well)` with
///   `box-shadow: inset 0 0 0 1px var(--c-hair)`, which is a 1px stroke on the
///   inside, exactly as a preview cell's is.
/// - `.fader b` / `.vfader b` — `linear-gradient(90deg, var(--c-mint),
///   var(--c-lav))` lying down and `linear-gradient(0deg, …)` standing up,
///   which is the same colour ramp the tempo carries and is drawn the same way
///   — see [`gradient`].
/// - `.fader s` / `.vfader s` — `background: var(--c-panel)` with
///   `box-shadow: 0 0 0 1px var(--c-line)`, which is a 1px stroke on the
///   **outside** because a `0 0 0 1px` shadow sits around the box rather than
///   inside it. Its `0 1px 4px rgba(0,0,0,0.18)` drop shadow is dropped for
///   the reason `room` collapses the day palette's pair: `epaint` draws one
///   shadow and the rim is the one that says where the knob is.
///
/// `live` is the one thing about a fader that is not the fader's:
/// `.strip.live .vfader s` is `box-shadow: 0 0 0 1px var(--c-pink),
/// 0 0 9px var(--c-glowp)`, so the knob of a slot that is on air takes a pink
/// rim and a pink halo and no other knob does. It is the mock saying *the
/// fader you are about to move is the one the audience is watching*.
fn fader_into(
    painter: &egui::Painter,
    pal: &Palette,
    fader: Fader,
    live: bool,
    reach: Option<Reach>,
) {
    let track_r = CornerRadius::same((fader.track.width().min(fader.track.height()) * 0.5) as u8);
    painter.rect_filled(fader.track, track_r, pal.well);
    painter.rect_stroke(
        fader.track,
        track_r,
        Stroke::new(size::HAIRLINE, pal.hair),
        StrokeKind::Inside,
    );
    gradient(painter, fader.fill, fader.axis, pal.mint, pal.lav, true);

    // **The band, over the fill and under the knob**: it is the fill setting
    // off, so it is drawn where the fill is drawn and the truth stays on top
    // of it. `.vfader em`'s 20% wash of `var(--c-lav)` — the mock's own ink
    // for *an address*, which is what a destination is — square rather than a
    // capsule, because it is a stretch of the track rather than a value's own
    // shape.
    //
    // **The mock draws it standing up only**, on deck B's fader: the trim's is
    // the same band lying down, and no strip in the mock has a gain fade armed
    // on it to draw one in.
    //
    // **Nothing at all while it rests**, which is where two thirds of an armed
    // strip's frames are: at a displacement of zero the band has no area, and
    // an empty rectangle painted every frame is a shape on the frame's budget
    // (ADR-0164) that draws nothing.
    if let Some(reach) = reach.filter(|reach| positive(reach.band)) {
        painter.rect_filled(reach.band, CornerRadius::ZERO, tint(pal.lav, 20));
    }

    let knob_r = CornerRadius::same((fader.knob.width().min(fader.knob.height()) * 0.5) as u8);
    if live {
        painter.add(
            egui::epaint::Shadow {
                offset: [0, 0],
                blur: size::VFADER_KNOB_GLOW,
                spread: 0,
                color: pal.glow_pink,
            }
            .as_shape(fader.knob, knob_r),
        );
    }
    painter.rect_filled(fader.knob, knob_r, pal.panel);
    painter.rect_stroke(
        fader.knob,
        knob_r,
        Stroke::new(
            size::HAIRLINE,
            match live {
                true => pal.pink,
                false => pal.line,
            },
        ),
        StrokeKind::Outside,
    );

    // **The mark last, over everything, and it is one pixel.** `.vfader i`'s
    // `background: var(--c-lav)`. It is the only thing here that may not be
    // lost: painted under the knob it would vanish inside it for every move
    // shorter than the knob is long, which is where a mark saying *not yet* is
    // needed most. A hairline over a 9px knob hides nothing of where the
    // control is.
    if let Some(reach) = reach {
        painter.rect_filled(reach.mark, CornerRadius::ZERO, pal.lav);
    }
}

/// The meter: the well, the mean's column and the peak's mark — and **nothing
/// in the well where there is no reading**, which is the mock's own `alloc`
/// strip.
///
/// - `.vmeter` — the fader's well, at 6 wide.
/// - `.vmeter b` — `linear-gradient(0deg, var(--c-mint), var(--c-sun))`, and
///   square rather than a capsule: `.vmeter` is `overflow: hidden` and the
///   column is clipped by the well rather than rounded itself.
/// - `.vmeter u` — `background: var(--c-pink)`, *on air*, which is the colour
///   the peak shares with the live tally and the lit beat.
fn meter_into(painter: &egui::Painter, pal: &Palette, well: Rect, meter: Option<Meter>) {
    let radius = CornerRadius::same((well.width() * 0.5) as u8);
    painter.rect_filled(well, radius, pal.well);
    painter.rect_stroke(
        well,
        radius,
        Stroke::new(size::HAIRLINE, pal.hair),
        StrokeKind::Inside,
    );
    let Some(meter) = meter else {
        return;
    };
    let painter = painter.with_clip_rect(meter.well);
    gradient(&painter, meter.fill, Axis::Column, pal.mint, pal.sun, false);
    painter.rect_filled(meter.peak, CornerRadius::ZERO, pal.pink);
}

/// One `.mini`: a capsule with a 1px border and whatever goes in it.
///
/// `sel` is `.mini.sel` — `color: var(--c-lav)`, `border-color: transparent`,
/// and a 15% wash of the same behind it. **The blend's mini is always `.sel`
/// and the mask's never is**, which is the mock's and reads: the blend names
/// which of `Blend::ALL` is in force, where the mask is a picker showing the
/// shape it is set to.
fn mini_into(
    painter: &egui::Painter,
    pal: &Palette,
    rect: Rect,
    sel: bool,
    contents: impl FnOnce(&egui::Painter, Color32),
) {
    let radius = CornerRadius::same((size::MINI_H * 0.5) as u8);
    match sel {
        true => {
            painter.rect_filled(rect, radius, tint(pal.lav, 15));
        }
        false => {
            painter.rect_stroke(
                rect,
                radius,
                Stroke::new(size::HAIRLINE, pal.line),
                StrokeKind::Inside,
            );
        }
    }
    contents(
        painter,
        match sel {
            true => pal.lav,
            false => pal.faint,
        },
    );
}

/// The mask's mark, drawn rather than typed — see [`Mask`].
///
/// A circle [`size::MINI_SIZE`] across, which is the size the glyph it stands
/// in for would have been, and then what the mask does to it: nothing, one
/// half filled, or a filled centre.
fn mask_mark(painter: &egui::Painter, centre: Pos2, colour: Color32, mask: Mask) {
    let r = size::MINI_SIZE * 0.5;
    match mask {
        Mask::None => {}
        // A half-disc is a circle with half of it not painted, so it is a clip
        // rather than a path: `epaint` has no arc and a polygon of one would
        // be a curve this file approximated by hand.
        Mask::Linear => {
            painter
                .with_clip_rect(Rect::from_min_max(
                    Pos2::new(centre.x, centre.y - r),
                    Pos2::new(centre.x + r, centre.y + r),
                ))
                .circle_filled(centre, r, colour);
        }
        Mask::Radial => {
            painter.circle_filled(centre, r * 0.5, colour);
        }
    }
    painter.circle_stroke(centre, r, Stroke::new(size::HAIRLINE, colour));
}

/// **A two-colour ramp along `axis`**, which is `linear-gradient` and a
/// painter that has none.
///
/// The same problem [`bpm_job`] answers for a line of type, and a different
/// answer because this is a shape rather than a run of glyphs: a `Mesh` of two
/// triangles with a colour at each corner, which `epaint` interpolates across
/// exactly. One shape, and no stepping.
///
/// `capsule` is `border-radius: 999px` on the fill: a fader's fill is a
/// capsule and its two ends are circles at the ramp's own ends, where a
/// meter's column is square and is clipped by the well around it instead.
/// Everything is clipped to `rect`, so an end cap on a fill shorter than it is
/// wide is a sliver rather than a bulge.
fn gradient(
    painter: &egui::Painter,
    rect: Rect,
    axis: Axis,
    from: Color32,
    to: Color32,
    capsule: bool,
) {
    if !(rect.width() > 0.0 && rect.height() > 0.0) {
        return;
    }
    let painter = painter.with_clip_rect(rect);
    // `90deg` runs left to right and `0deg` runs **up**, so a column's `from`
    // is at the bottom — which is also where its zero is.
    let colour_at = |p: Pos2| match axis {
        Axis::Row => match p.x <= rect.center().x {
            true => from,
            false => to,
        },
        Axis::Column => match p.y >= rect.center().y {
            true => from,
            false => to,
        },
    };
    let mut mesh = egui::epaint::Mesh::default();
    for corner in [
        rect.left_top(),
        rect.right_top(),
        rect.right_bottom(),
        rect.left_bottom(),
    ] {
        mesh.colored_vertex(corner, colour_at(corner));
    }
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    painter.add(egui::Shape::mesh(mesh));

    if capsule {
        let r = rect.width().min(rect.height()) * 0.5;
        let (start, end) = match axis {
            Axis::Row => (
                Pos2::new(rect.min.x + r, rect.center().y),
                Pos2::new(rect.max.x - r, rect.center().y),
            ),
            Axis::Column => (
                Pos2::new(rect.center().x, rect.max.y - r),
                Pos2::new(rect.center().x, rect.min.y + r),
            ),
        };
        painter.circle_filled(start, r, from);
        painter.circle_filled(end, r, to);
    }
}

/// `color-mix(in srgb, X n%, transparent)`, as the alpha it is: `n`% of 255,
/// rounded. The mock's own wash behind an armed control, a live tally and a
/// selected mini, and `room`'s documentation is where the equivalence is
/// argued.
fn tint(colour: Color32, percent: u8) -> Color32 {
    let alpha = ((percent as u32 * 255 + 50) / 100) as u8;
    Color32::from_rgba_unmultiplied(colour.r(), colour.g(), colour.b(), alpha)
}

// ---------------------------------------------------------------------------
// The Library bay
// ---------------------------------------------------------------------------

/// **The word at the head of the Library bay**, in the source's own
/// capitalisation for [`Kind::Bay`]'s reason: the mock upper-cases in CSS, and
/// that is done at paint time so the word a reader searches for is the word in
/// the source.
const LIBRARY_TITLE: &str = "Library";

/// **The Library bay, laid out**: where the rows go, how many of them there is
/// room for, and where the count under them goes.
///
/// # What the bay is standing on, and it is a listing rather than a walk
///
/// The manual: *"A scope and a walk, not one flat list: favourites, my sets,
/// app presets, a folder."* **One of those four exists** — `my sets`, which is
/// [`karakuri_store::Store::list_sets`](../../../crates/karakuri-store/src/store.rs)
/// and is a directory of Set files. So this draws that list and no scope
/// chooser above it, for the reason [`outputs`] draws one sink of four: a
/// control over machinery that does not exist is the scaffolding this module's
/// documentation refuses.
///
/// # What is in the mock's bay and is deliberately not here
///
/// - **The `.scopes` row** — `favourites`, `my sets`, `presets`, `folder` and
///   a `+`. Every chip is a control, three of the four name a collection
///   nothing in this workspace can produce, and what a folder scope even reads
///   is one of the two questions `console.html` itself lists as **still
///   open**: *"A directory of `.kir` files is a different thing from a
///   directory of Set files, and a bundle is a third."* The `+` is the arena's
///   own gap drawn a fifth time, which [`outputs`] already names.
/// - **The `.path` row**, `~/sets/tour-2026/night-b › opening`. It is the
///   walk *inside* a folder scope, so it says nothing until that scope is
///   decided.
/// - **The `.lib-filters` fields**, `holds…` and `layer…`. Two text controls,
///   and the vocabulary has the operation they would emit —
///   `Operation::ListSets { holds, layer }` — but nothing in the store answers
///   it: `list_sets` reads names off a directory and no index anywhere says
///   what a Set *holds*.
/// - **`.lib-row`'s `.star`.** A favourite is a fact about a Set that nothing
///   in this workspace keeps — there is no such field on a `SetEntry`, no
///   record that carries one, and no metadata card that mentions one. Drawing
///   a hollow star on every row would assert that nothing is a favourite,
///   which is a reading nobody took.
/// - **`.lib-row .dim`, the time beside each name.** This one is different
///   from the others and is worth the sentence: the *value* exists —
///   `SetEntry::written` is the Set file's own mtime — and what does not exist
///   is a spelling for it. The one answer in this workspace is
///   `karakuri-cli`'s `setfile::written_at`, in a package with no library
///   target, and it is local time to the second where the mock's column is
///   `16:09`. Writing a second spelling here is the kind of second answer this
///   repository deletes rather than adds, so the column waits for the one
///   spelling to be somewhere both callers can reach.
/// - **`.lib-row.cursor`, and the `load → C` pill in the foot.** A cursor is a
///   selection this console does not keep — the same sentence [`mixer`] writes
///   about the deck selection — and the pill is *"How a Set gets from the
///   library to a deck"*, which is the second of `console.html`'s own still
///   open questions. **Neither blocks the listing**: what is missing is a way
///   to play from this bay, not a way to draw it.
///
/// # The foot's number is the mock's own, read the mock's way
///
/// `5 of 27` is how many rows are listed against how many the scope holds, and
/// both halves are here: the total is what the harness handed over, and the
/// count is how many rows the bay had room for. So a library taller than its
/// list says so in the one place the mock puts it, and nothing scrolls —
/// which is honest, because there is no scroll position anywhere in this
/// crate and inventing one would be a control.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LibraryBay {
    /// `.lib-list`'s content box: the region under the bay head and above the
    /// foot, inside [`size::LIB_LIST_PAD`], where the rows are laid from the
    /// top with no gap between them.
    pub list: Rect,
    /// **How many rows are drawn**, which is how many fit in [`list`](Self::list) —
    /// never more than [`total`](Self::total), and never zero, because a bay
    /// with no room for one row is no bay at all.
    pub rows: usize,
    /// **How many Sets the store holds**, which is what the harness handed
    /// over. The second half of the foot's `n of m`.
    pub total: usize,
    /// `.lib-foot`, along the bottom edge of the bay, with its rule on top.
    pub foot: Rect,
}

impl LibraryBay {
    /// The `index`th row's rectangle, counting from the top of the list.
    ///
    /// Derived rather than stored for [`TransportRow::dot`]'s reason: the rows
    /// are a stride and a count, and a `Vec` of them would be an allocation a
    /// frame does not need. `index` past [`rows`](Self::rows) is a rectangle
    /// past the end of the list, which is a caller's error and not a state —
    /// the one caller iterates `0..rows`.
    pub fn row(&self, index: usize) -> Rect {
        Rect::from_min_size(
            Pos2::new(
                self.list.min.x,
                self.list.min.y + size::LIB_ROW_H * index as f32,
            ),
            egui::vec2(self.list.width(), size::LIB_ROW_H),
        )
    }

    /// What the foot reads: `n of m`, the mock's own `5 of 27`.
    pub fn count(&self) -> String {
        format!("{} of {}", self.rows, self.total)
    }
}

/// **The Library bay's rows, derived** — see [`LibraryBay`] for what is drawn
/// here and for the six things in the mock's bay that are not.
///
/// `layout` must be solved: [`Layout::rect`] refuses to answer from a dirty
/// one. Unlike [`outputs`], [`transport`] and [`mixer`] this asks `egui` for
/// nothing: every box in the bay is the full width of the list, so no
/// rectangle here is the width of the type in it.
///
/// `None` where there is nothing to list, and `None` where there is no room to
/// list it: a console with no store behind it is every test in this crate and
/// the whole of `cargo test -p karakuri-console`, and what the bay draws then
/// is the card and its head and nothing else — this is [`mixer`]'s rule, one
/// column along, and [`View::picture`]'s before that.
pub fn library(layout: &karakuri_layout::Layout, sets: &[String]) -> Option<LibraryBay> {
    // **No store behind the console, so there is nothing to list.** Drawing an
    // empty list with `0 of 0` under it would be a reading of a library nobody
    // opened, which is the row of zeroes ADR-0177 is about.
    if sets.is_empty() {
        return None;
    }
    library_box(to_egui(layout.rect(layout.find("library")?)), sets.len())
}

/// The arithmetic of the bay, away from the layout it reads.
///
/// Term for term from `style.css`:
///
/// - `.lib-foot { padding: 5px 10px; font-size: 10px; border-top: 1px solid
///   var(--c-hair) }` — a [`size::LIB_FOOT_H`] row along the bottom of the
///   bay, its rule the top pixel of it.
/// - `.lib-list { display: flex; flex-direction: column; padding: 3px }` —
///   what is left between the bay head and that row, inset by
///   [`size::LIB_LIST_PAD`] on all four sides.
/// - `.lib-row { padding: 3px 7px }` — [`size::LIB_ROW_H`] each, stacked from
///   the top of the list with no gap, because `.lib-list` states none.
///
/// # The foot is at the bottom and the leftover is the list's
///
/// The mock's bay is a flow: its children stack from the top and whatever is
/// left over is under the last of them. **Here the leftover goes to the list
/// instead**, and the mock says which: the Library is the bay that carries
/// `style="flex:1"` in its column and a `.grip` in its head, which is *"the
/// bay that absorbs its column's height"* — and what absorbs it is the list,
/// since a foot of one line at a fixed type size has nothing in it that gets
/// bigger. A foot left floating under the last row would also put its
/// `border-top` between the list and bare card, which is a rule separating
/// something from nothing.
///
/// `None` where the region cannot hold the foot and one row, which is
/// [`picture_rect`]'s rule stated on a listing.
fn library_box(region: Rect, total: usize) -> Option<LibraryBay> {
    let foot = Rect::from_min_max(
        Pos2::new(region.min.x, region.max.y - size::LIB_FOOT_H),
        region.max,
    );
    let list = Rect::from_min_max(
        Pos2::new(
            region.min.x + size::LIB_LIST_PAD,
            region.min.y + size::HEAD_H + size::LIB_LIST_PAD,
        ),
        Pos2::new(
            region.max.x - size::LIB_LIST_PAD,
            foot.min.y - size::LIB_LIST_PAD,
        ),
    );
    // **Narrower than its own padding is no list**, which is
    // [`picture_rect`]'s rule stated across the axis. There is no matching
    // check down it: a bay too short for the foot is already a bay too short
    // for a row, and `rows` below is what answers that.
    if list.width() <= 0.0 {
        return None;
    }
    let fits = (list.height() / size::LIB_ROW_H).floor().max(0.0) as usize;
    let rows = fits.min(total);
    (rows > 0).then_some(LibraryBay {
        list,
        rows,
        total,
        foot,
    })
}

/// **The Library bay's rows, painted.**
///
/// Where everything goes is [`library`]'s, so this paints and derives nothing.
///
/// Term for term from `style.css`:
///
/// - `.lib-row` — `color: var(--c-dim)`, a name at [`size::BASE`], one
///   [`size::LIB_ROW_PAD_X`] in from the left of the list and centred across
///   the row's own height.
/// - `.lib-foot` — `color: var(--c-faint)` at [`size::LIB_FOOT_SIZE`], one
///   [`size::LIB_FOOT_PAD_X`] in and centred, over a
///   `border-top: 1px solid var(--c-hair)`.
///
/// **A name too long for the track is clipped rather than elided**, which is
/// the mock's own answer: `.lib-row` sets no `text-overflow` where `.path` and
/// `.strip-name` both do, so there is no ellipsis to draw. The clip is
/// `.lib-list`'s box, which is the same `with_clip_rect` the picture, a
/// preview cell and a tally are each drawn inside.
fn library_into(ui: &Ui, pal: &Palette, bay: &LibraryBay, sets: &[String]) {
    let painter = ui.painter().with_clip_rect(bay.list);
    for (index, name) in sets.iter().take(bay.rows).enumerate() {
        let row = bay.row(index);
        let galley = painter.layout_job(span_at(name, size::BASE, pal.dim));
        painter.galley(
            Pos2::new(
                row.min.x + size::LIB_ROW_PAD_X,
                row.center().y - galley.size().y * 0.5,
            ),
            galley,
            pal.dim,
        );
    }

    let painter = ui.painter().with_clip_rect(bay.foot);
    let rule = bay.foot.min.y + size::HAIRLINE * 0.5;
    painter.line_segment(
        [
            Pos2::new(bay.foot.min.x, rule),
            Pos2::new(bay.foot.max.x, rule),
        ],
        Stroke::new(size::HAIRLINE, pal.hair),
    );
    let galley = painter.layout_job(span_at(&bay.count(), size::LIB_FOOT_SIZE, pal.faint));
    painter.galley(
        Pos2::new(
            bay.foot.min.x + size::LIB_FOOT_PAD_X,
            bay.foot.center().y - galley.size().y * 0.5,
        ),
        galley,
        pal.faint,
    );
}

/// The console's view: which room it is in, and the frame's plan, kept so a
/// frame does not allocate one.
pub struct View {
    pub room: Room,
    /// **What to draw in the Program bay's picture this frame**, or `None` for
    /// a console with no engine behind it — which is every test in this crate
    /// and the whole of what `cargo test -p karakuri-console` sees.
    ///
    /// Set per frame by whoever owns the device, because that is who knows
    /// whether the texture it names is still the right size. A stale id here
    /// is a freed registration, so it is written beside the frame that made it
    /// rather than kept.
    pub picture: Option<Picture>,
    /// **What to draw in each of the four deck preview cells this frame**, in
    /// slot order, or `None` for a cell whose deck is off.
    ///
    /// **The same seam as [`View::picture`], four times over**: registering a
    /// texture takes a device, this crate has none, so whoever owns the device
    /// registers them and writes this per frame beside the frame that made
    /// them. A stale id here is a freed registration. Every test in this crate
    /// leaves every entry `None`, which is what `cargo test -p
    /// karakuri-console` sees and is a console with no engine behind it.
    ///
    /// **All `None` is a state, not an absence.** The manual's *previews 2 of
    /// 4* is an operator choosing how many decks audition, so a cell with
    /// nothing in it is a cell that is off and it is drawn saying so — see the
    /// module documentation, and [`preview_rects`] for where the rectangles
    /// come from.
    pub previews: [Option<Picture>; DECKS],
    /// **What the transport row reads this frame**, or `None` for a console
    /// with no engine behind it — which is every test in this crate, and what
    /// the row draws then is nothing at all.
    ///
    /// **The same seam as [`View::picture`]**, one row up and without a
    /// device: the tempo, the beat and the frame's cost are a clock and an
    /// engine, and `src/` has neither (ADR-0156). So whoever owns them reads
    /// them and writes this per frame, beside the frame that measured it.
    /// See [`Transport`] and [`transport`].
    pub transport: Option<Transport>,
    /// **What each mixer strip reads this frame**, one per slot the deck has,
    /// in slot order — and **empty** for a console with no deck behind it,
    /// which is every test in this crate and what the bay draws then is
    /// nothing at all.
    ///
    /// **The same seam as [`View::transport`]**, and empty rather than
    /// `Option<Vec<_>>` because an empty list of strips is already the whole
    /// of *no deck*: a deck has one to four slots (`MAX_SLOTS`, asserted in
    /// `Deck::new`), so there is no deck that has none and no second way to
    /// say it.
    ///
    /// A `Vec` a caller keeps and rewrites, rather than a fixed array of
    /// `Option`s like [`View::previews`]: a preview cell is off or on and the
    /// row is always four, where the strips are *as many as the deck has* and
    /// a `None` in the middle of them would be a slot no `Deck` can have.
    /// [`View::new`] gives it room for [`DECKS`] so the frame path never grows
    /// it. See [`Strip`] and [`mixer`].
    pub mixer: Vec<Strip>,
    /// **What the Library bay lists this frame**: the name of every Set the
    /// store holds, in the order the store listed them — and **empty** for a
    /// console with no store behind it, which is every test in this crate and
    /// what the bay draws then is nothing at all.
    ///
    /// **The same seam as [`View::mixer`]**, and empty rather than
    /// `Option<Vec<_>>` for the same reason: an empty listing is already the
    /// whole of *no library*, and a store that holds nothing and a store that
    /// is not there are the same bay — one with no row to draw.
    ///
    /// **Read once rather than per frame**, by whoever owns the store. A
    /// listing is a directory read, which is not a thing to do on a frame
    /// path (P-0072), and nothing in this crate can do it anyway: opening a
    /// store is `karakuri-store`'s and `src/` depends on neither it nor the
    /// engine (ADR-0156). What crosses the seam is a list of names.
    ///
    /// A name and nothing else, because a name is what exists: see [`library`]
    /// for the star and the time the mock draws beside it, and for why neither
    /// is here.
    pub library: Vec<String>,
    /// **The shape of what is being rendered**, which is what the Program bay
    /// arranges its body for — [`program_bay`], and [`picture_rect`] for why
    /// it is two numbers rather than a ratio.
    ///
    /// **The same seam as [`View::picture`], and it belongs beside it**: the
    /// picture's rectangle is handed in by whoever built the `Present`, and
    /// this is the number that rectangle was derived from. Written per frame
    /// by that same caller and in the same breath, so that the canvas the
    /// texture was sized for and the canvas the bay arranged itself for cannot
    /// be two different numbers — which would put the cells in one arrangement
    /// and the picture in the other.
    ///
    /// [`MOCK_CANVAS`] until somebody says otherwise, which is every test in
    /// this crate and is a console with no engine behind it: there is no
    /// picture to draw, and the four cells still have to go somewhere.
    pub canvas: (u32, u32),
    /// **How long the panel has been animating**, written per frame by
    /// whoever has the clock — see [`Phase`], which carries the whole
    /// argument for why this is a value and not an `Instant`.
    ///
    /// **The same seam as [`View::transport`]**, and the one this crate is
    /// least able to cross: a clock in `src/` is P-0002 broken in the file
    /// whose own doc says so. [`Phase::ZERO`] until somebody says otherwise,
    /// which is every test in this crate and is a console with no clock behind
    /// it — a panel drawn at the origin of every animation on it.
    pub phase: Phase,
    placed: Vec<Placed>,
}

impl View {
    pub fn new(room: Room) -> View {
        View {
            room,
            picture: None,
            previews: [None; DECKS],
            transport: None,
            // As many strips as a deck can ever have, so the frame path never
            // grows it — the same reason `placed` is built with a capacity.
            mixer: Vec::with_capacity(DECKS),
            // Nothing until somebody lists a store, which is every test in
            // this crate. No capacity is reserved: how many Sets a store holds
            // is not a number this crate has, and the list is written once
            // rather than per frame.
            library: Vec::new(),
            canvas: MOCK_CANVAS,
            phase: Phase::ZERO,
            // Every region the console has, so the frame path never grows it.
            placed: Vec::with_capacity(REGIONS.len()),
        }
    }

    /// **Every live region that is declaring this frame**, each with what one
    /// update of it costs and how stale it may get.
    ///
    /// # This is P-0072's naming, and the unit is a region
    ///
    /// [P-0072](../../../docs/principles/0072-a-still-panel-costs-nothing-and-what-moves-declares-its-price.md):
    /// *"What must be live during a performance is named, and each named thing
    /// declares two numbers: what its update costs, and how stale it may get,
    /// in milliseconds."* This is that naming, and [`crate::budget`] holds the
    /// numbers with the arguments for where each came from. **A region is
    /// redrawn whole or not at all**, so what appears here is a node of the
    /// arrangement — by the name every surface addresses it by — and never an
    /// animation, a control or a slice of a frame.
    ///
    /// # One region, three presentations, one declaration
    ///
    /// The mixer bay is the only entry today. Inside it the tally's word rolls
    /// toward a residency that has not been granted and each of a strip's two
    /// faders reaches toward a value a transition has not reached yet — three
    /// presentations at one rate, off one [`Phase`], so they are one term and
    /// not three
    /// ([ADR-0190](../../../docs/adr/0190-the-parked-tally-rolls-because-two-lamps-do-not-fit-in-fifty-three-pixels.md),
    /// [ADR-0206](../../../docs/adr/0206-a-fader-marks-where-it-is-going-and-keeps-reaching-for-it.md)).
    /// A second *rate* would be a second declaration; a second *user* of this
    /// rate is not. The beat is the panel's other candidate and P-0077 wants
    /// it moving continuously; it is in the transport row, which folds, and it
    /// would carry its own node here exactly as this one does.
    ///
    /// # Pending is not enough: the region that shows it has to be laid out
    ///
    /// **A region that is not on screen declares nothing**, however much is
    /// pending behind it. The strips are rewritten every frame from the deck,
    /// so *is anything pending* is a fact about the deck; P-0072 is about what
    /// **must be live**, and a bay the operator has folded away is not live.
    /// A declaration made for it buys a repaint of something nobody can see —
    /// measured, before this asked: with the picture and the preview row
    /// folded the window sat at 28.7 to 29.0 frames a second, and folding the
    /// mixer bay on top of that moved the price of a frame and not the rate.
    /// It draws nothing at all now. The alternative — declare it anyway and
    /// let a scheduler drop it — is
    /// [ADR-0193](../../../docs/adr/0193-a-region-that-is-not-laid-out-declares-nothing-rather-than-being-dropped-later.md),
    /// which is where it lost, and it lost on the sentence being **false**
    /// rather than unaffordable: a region nobody can see is not showing
    /// anything, so it cannot be showing anything out of date.
    ///
    /// This is
    /// [P-0073](../../../docs/principles/0073-a-node-claims-only-what-its-visible-content-can-use.md)
    /// in time rather than in space — *a node claims only what its visible
    /// content can use*, where what is claimed is a share of the frame budget
    /// rather than a share of the viewport.
    ///
    /// **The question is asked of the layout, which already answers it.**
    /// [`Layout::visible`](karakuri_layout::Layout::visible) is ADR-0183's
    /// disjunction — the operator's fold and the drawing's `set_aside`, read
    /// as one — walked up the ancestors, so a folded right pane takes the
    /// mixer with it and no second derivation of *is this laid out* is written
    /// here. It is **not** [`mixer`]'s `None`, which is a different question
    /// and a stricter one: that answers *can the strips be laid out in this
    /// rectangle this pass*, and says no for a window merely too small and for
    /// the frame before `egui` has fonts. Neither is a reason to stop the
    /// roll — only being out of the layout is — and the layout answers this
    /// one without a solve, which matters because the fold is applied and this
    /// is asked before the next one.
    ///
    /// # Nothing arbitrates between two of these
    ///
    /// P-0072's second half is a scheduler and there is not one. What this
    /// feeds is [`View::animating`], which takes the soonest staleness and
    /// nothing else, and `tests/schedulable.rs`, which sums over whatever this
    /// answers and asserts the two conditions the principle states.
    pub fn declares(&self, layout: &karakuri_layout::Layout) -> impl Iterator<Item = Declared> {
        // An array rather than a `Vec`, so asking what the panel declares
        // allocates nothing on a path that is walked every frame — and so that
        // the second live region is one more element rather than a change of
        // shape.
        [self.mixer_declares(layout)].into_iter().flatten()
    }

    /// **What the mixer bay declares**: the roll's staleness while anything in
    /// it is pending *and* the bay is laid out, and nothing otherwise.
    ///
    /// Three pending things, one rate — see [`View::declares`], which carries
    /// the whole argument.
    fn mixer_declares(&self, layout: &karakuri_layout::Layout) -> Option<Declared> {
        let bay = layout.find("mixer").is_some_and(|id| layout.visible(id));
        let moving = bay
            && self.mixer.iter().any(|strip| {
                strip.pending().is_some()
                    || strip.gain_pending().is_some()
                    || strip.opacity_pending().is_some()
            });
        moving.then_some(Declared {
            region: "mixer",
            cost: PANEL_PASS,
            staleness: ROLL_STALENESS,
        })
    }

    /// **The soonest staleness any live region on this panel will tolerate**,
    /// and `None` when nothing on it is moving.
    ///
    /// # The view is what knows the rate, so the harness is told rather than
    /// guessing
    ///
    /// [`View::declares`] is where the regions and their two numbers are, and
    /// this is the one of the two numbers a window can act on today:
    /// `crate::repaint::Change::Animating` turns it into a deadline. A rate
    /// written into the harness instead would be a presentation's number kept
    /// where the presentation is not — change the roll and the window goes on
    /// servicing the old one, with nothing failing to compile and nothing to
    /// assert against.
    ///
    /// **The soonest, not the sum**, and that is the whole of what is decided
    /// here: a deadline is met by drawing, and one frame drawn in time for the
    /// soonest is in time for every other. Choosing which region a frame is
    /// *for* is a scheduler's, and there is not one.
    ///
    /// # Nothing pending means nothing moving, and that is the whole of
    /// P-0072's first clause
    ///
    /// `None` is not an absence of information: it is the panel saying it is
    /// still, and the window then sleeps. **A console with no parked slot and
    /// no scheduled move on a fader costs exactly what it cost before either
    /// existed**, which is a claim `tests/parked.rs` and `tests/armed.rs` make
    /// rather than a hope.
    pub fn animating(&self, layout: &karakuri_layout::Layout) -> Option<Duration> {
        self.declares(layout).map(|live| live.staleness).min()
    }

    /// Draw the whole console. The `ui` is the root one
    /// [`egui::Context::run_ui`] hands the frame's closure.
    pub fn draw(&mut self, ui: &mut Ui, panel: &mut Panel) {
        let pal = self.room.palette();
        // **The bay arranges itself before anything else this frame reads the
        // layout**, the cursor below included: the boundary between the
        // picture and the row is not there while the cells are beside the
        // picture, and a resize cursor drawn from the solve before the
        // rearrangement is a cursor for a divider that has gone.
        plan_into(panel, self.canvas, &mut self.placed);
        self.cursor(ui.ctx(), panel);

        // **Where the four cells go, asked once and off the same derivation
        // the picture came through.** Not from `placed`: beside the picture
        // the row is set aside, so `deck-previews` is not in the plan at all
        // and a cell drawn from its entry would be a cell drawn in one of the
        // two arrangements only.
        let cells = program_bay(panel.layout(), self.canvas).and_then(|bay| bay.cells);
        let picture = self.picture;
        let previews = self.previews;
        let values = self.transport;
        let strips = self.mixer.as_slice();
        let sets = self.library.as_slice();
        let phase = self.phase;
        let frame = egui::Frame::NONE.fill(pal.ground);
        egui::CentralPanel::default().frame(frame).show(ui, |ui| {
            for placed in &self.placed {
                let rect = to_egui(placed.rect);
                match placed.region.kind {
                    Kind::Bay { title, pills, grip } => {
                        card(ui, &pal, rect);
                        bay_head(ui, &pal, rect, title, pills, grip);
                        // The inspector is the one bay that is a split, and
                        // its panes' boundary is drawn as the mock's
                        // `.divider-v` rather than left as bare ground: it is
                        // inside a card, where the ground does not reach.
                        pane_dividers(ui, &pal, panel, placed.id, rect);
                    }
                    // The other row with something in it, and everything in it
                    // is a readout: where each one goes is `transport`'s
                    // answer, and with no engine behind the console there is
                    // no answer and the card is as empty as every other body
                    // in this pass.
                    Kind::Transport => {
                        card(ui, &pal, rect);
                        if let Some(row) = transport(ui.ctx(), panel.layout(), values) {
                            transport_into(ui, &pal, &row);
                        }
                    }
                    // The one row with something in it, and the something is
                    // one control. Where it goes is `outputs`'s answer and
                    // not this pass's: the same call `input::claim` makes, so
                    // the chip that is painted is the chip that is clicked.
                    Kind::Outputs => {
                        card(ui, &pal, rect);
                        if let Some(row) = outputs(ui.ctx(), panel.layout()) {
                            outputs_into(ui, &pal, &row);
                        }
                    }
                    // The one bay with something in its body, and it is a bay
                    // in every other respect: the same card and the same head
                    // the other six get, and then as many strips as the deck
                    // has. With no deck behind the console there are none and
                    // the body is as empty as every other one in this pass —
                    // `mixer`'s answer, not this pass's, so that *no deck
                    // means nothing at all* is decided in one place.
                    Kind::Mixer => {
                        card(ui, &pal, rect);
                        bay_head(ui, &pal, rect, MIXER_TITLE, &[], false);
                        if let Some(bay) = mixer(ui.ctx(), panel.layout(), strips) {
                            mixer_into(ui, &pal, &bay, phase);
                        }
                    }
                    // The other bay with something in its body, and it is a
                    // bay in every other respect: the same card and the same
                    // head, and then as many rows as the bay has room for.
                    // With no store behind the console there are none and the
                    // body is as empty as every other one in this pass —
                    // `library`'s answer, not this pass's, so that *no store
                    // means nothing at all* is decided in one place.
                    Kind::Library => {
                        card(ui, &pal, rect);
                        bay_head(ui, &pal, rect, LIBRARY_TITLE, &[], true);
                        if let Some(bay) = library(panel.layout(), sets) {
                            library_into(ui, &pal, &bay, sets);
                        }
                    }
                    // A pane draws nothing of its own. It has no card — it is
                    // inside the bay's — and no head, and its body is as empty
                    // as every other body in this pass.
                    Kind::Pane => {}
                    // The one body that is not empty, because its texels are
                    // not this crate's to invent: a texture is there or it is
                    // not, and where it is not the bay's card shows through
                    // exactly as every other empty body does. No placeholder,
                    // for the reason this module's documentation gives.
                    Kind::Picture => {
                        if let Some(picture) = picture {
                            // Clipped to the region: the rectangle came from
                            // outside, and a stale one is a picture painted
                            // over the inspector rather than a wrong picture.
                            ui.painter().with_clip_rect(rect).image(
                                picture.id,
                                picture.rect,
                                WHOLE_TEXTURE,
                                Color32::WHITE,
                            );
                        }
                    }
                    // **The one arm that draws nothing, and it is not an
                    // empty body.** The four cells are the bay's and not this
                    // region's: they are drawn below, once, from wherever
                    // `program_bay` put them — which is under the picture on a
                    // narrow bay and down the sides of a wide one, where this
                    // region has no extent and no entry in the plan at all. A
                    // cell drawn from here would be a cell drawn in one of the
                    // two arrangements only.
                    Kind::Previews => {}
                }
            }

            // **The deck previews, wherever they went.** Painted after the
            // loop rather than in it for the reason the arm above gives, and
            // painting last costs nothing: they sit inside the Program bay's
            // card, which the loop has already painted, and nothing else on
            // the panel overlaps them. A cell is the region's own face rather
            // than a placeholder — the module documentation is where that
            // argument is.
            if let Some(cells) = cells {
                for (deck, cell) in cells.into_iter().enumerate() {
                    preview(ui, &pal, cell, deck, previews[deck]);
                }
            }
        });
    }

    /// Say what is under the pointer.
    ///
    /// **`egui` sets it, not `winit`.** `egui_winit`'s
    /// `handle_platform_output` already writes the window's cursor from
    /// `PlatformOutput` every frame, so a `Window::set_cursor` call beside it
    /// is a second writer and the last one each frame wins — which is a
    /// flicker that depends on event order. One writer, and it is the one that
    /// is already there.
    fn cursor(&self, ctx: &egui::Context, panel: &mut Panel) {
        panel.solve();
        // A boundary in hand keeps the resize cursor even where the pointer
        // has run off it, for the reason `input`'s rule 1 keeps the events:
        // the gesture is what is happening, not the position.
        //
        // **And a gesture that is not a boundary drag suppresses it**, for the
        // same reason and in the other direction. Falling through to the hit
        // test with a fader in hand would flick a resize cursor on the moment
        // a fader held against its top let the pointer wander across a
        // boundary, which is a cursor for a gesture that is not happening.
        //
        // There is no cursor of its own for a fader: the console's whole
        // cursor vocabulary is *arrow, or resize over a boundary*, and
        // inventing a third mark here would be one control saying something no
        // other one on the panel says.
        let axis = match panel.in_hand() {
            Some(InHand::Boundary(axis)) => Some(axis),
            Some(InHand::Fader) => None,
            None => match panel.layout().hit(panel.cursor(), GRAB) {
                Hit::Divider { split, .. } => panel.layout().axis(split),
                _ => None,
            },
        };
        match axis {
            Some(Axis::Row) => ctx.set_cursor_icon(egui::CursorIcon::ResizeHorizontal),
            Some(Axis::Column) => ctx.set_cursor_icon(egui::CursorIcon::ResizeVertical),
            None => {}
        }
    }
}

/// **One deck preview cell**: the mock's `.preview`, with whatever is behind
/// it and the letter that says which deck it is.
///
/// Painted the same whether a deck is running in it or not, because *off* is a
/// state an operator chooses rather than a thing not built yet — the module
/// documentation is where that argument is written out.
///
/// Term for term from `.preview` in `style.css`:
///
/// - `background: var(--c-well)` — `pal.well`, and it is the ground the
///   texture is drawn over rather than a fallback for one, so a texture with
///   any transparency in it reads as a recess and not as a hole.
/// - `border-radius: 7px` — [`size::PREVIEW_RADIUS`].
/// - `box-shadow: inset 0 0 0 1px var(--c-hair)` — a 1px stroke on the
///   **inside** of the box in `pal.hair`, drawn last so it sits over the
///   texture exactly as an inset shadow sits over a background image.
/// - `display: flex; align-items: flex-end; padding: 3px 5px;
///   font-size: 9px` — the letter in the bottom-left corner, inside that
///   padding, at [`size::PREVIEW_SIZE`].
///
/// # The two colours the label takes, and what each is standing for
///
/// - **A picture behind it:** `.preview.a` and `.preview.b` set
///   `color: rgba(255,255,255,0.7)`, which is white because the mock's cell is
///   always a dark gradient. The console's cell holds real texels of unknown
///   brightness in either room, and there is no *white at 70%* in the palette
///   — so this is `pal.text`, `--c-text`, the mock's own colour for a value.
///   It follows the room, which is the one thing the mock's literal white
///   cannot do.
/// - **Nothing behind it:** `.preview`'s own `color: var(--c-faint)`, which is
///   `pal.faint` — *"a heading, and anything switched off"*, and this cell is
///   switched off.
///
/// And the word follows the colour: the mock writes `C &middot; off` in a cell
/// with nothing in it, so a cell that is off says `off` rather than leaving
/// the reader to tell a dark thumbnail from an empty well.
fn preview(ui: &Ui, pal: &Palette, cell: Rect, deck: usize, picture: Option<Picture>) {
    let radius = CornerRadius::same(size::PREVIEW_RADIUS as u8);
    // Clipped to the cell for the reason the picture is clipped to its region:
    // the rectangle in `picture` came from outside, and a stale one is a
    // thumbnail painted across the bay rather than a wrong thumbnail.
    let painter = ui.painter().with_clip_rect(cell);
    painter.rect_filled(cell, radius, pal.well);

    if let Some(picture) = picture {
        painter.image(picture.id, picture.rect, WHOLE_TEXTURE, Color32::WHITE);
    }
    painter.rect_stroke(
        cell,
        radius,
        Stroke::new(size::HAIRLINE, pal.hair),
        StrokeKind::Inside,
    );

    let letter = DECK_LETTERS[deck];
    let (label, colour) = match picture {
        Some(_) => (letter.to_owned(), pal.text),
        None => (format!("{letter} · off"), pal.faint),
    };
    let galley = painter.layout_no_wrap(
        label,
        FontId::new(size::PREVIEW_SIZE, FontFamily::Proportional),
        colour,
    );
    painter.galley(
        Pos2::new(
            cell.min.x + size::PREVIEW_PAD_X,
            cell.max.y - size::PREVIEW_PAD_Y - galley.size().y,
        ),
        galley,
        colour,
    );
}

/// **The Outputs row's contents**: the word, and the one `.sink`.
///
/// Where everything goes is [`outputs`]'s, so this paints and derives nothing.
/// Term for term from `style.css`:
///
/// - `.sink` — `background: var(--c-well)`, `color: var(--c-dim)`, and
///   `border-radius: 999px`, which on a box this short is a capsule.
/// - `.sink.on` — `color: var(--c-text)` over
///   `color-mix(in srgb, var(--c-mint) 14%, transparent)`, which is
///   `pal.mint` at 14% alpha exactly (see `room`'s documentation).
/// - `.dot` — `var(--c-faint)` off, and `var(--c-mint)` with
///   `box-shadow: 0 0 8px var(--c-glow)` on. The halo is an
///   [`egui::epaint::Shadow`] with the blur the CSS names and a corner radius
///   of half the dot, which is a blurred circle — the same mechanism the bay's
///   card draws its drop shadow with, and the first thing in this crate to use
///   `--c-glow`, which `room` transcribed against the day a control was drawn.
fn outputs_into(ui: &Ui, pal: &Palette, row: &Outputs) {
    let painter = ui.painter();

    // The word: `.bay-head`'s type on a row that has no head — see
    // `OUTPUTS_LABEL`.
    let galley = painter.layout_job(label_job(pal.faint));
    painter.galley(row.label.min, galley, pal.faint);

    let (ink, fill, dot) = match row.on {
        true => (pal.text, tint(pal.mint, 14), pal.mint),
        false => (pal.dim, pal.well, pal.faint),
    };
    painter.rect_filled(
        row.sink,
        CornerRadius::same((size::SINK_H * 0.5) as u8),
        fill,
    );
    if row.on {
        painter.add(
            egui::epaint::Shadow {
                offset: [0, 0],
                blur: 8,
                spread: 0,
                color: pal.glow,
            }
            .as_shape(row.dot, CornerRadius::same((size::SINK_DOT * 0.5) as u8)),
        );
    }
    painter.circle_filled(row.dot.center(), size::SINK_DOT * 0.5, dot);

    let galley = painter.layout_no_wrap(
        PROGRAM_VIEW.to_owned(),
        FontId::new(size::BASE, FontFamily::Proportional),
        ink,
    );
    painter.galley(
        Pos2::new(
            row.dot.max.x + size::SINK_GAP,
            row.sink.center().y - galley.size().y * 0.5,
        ),
        galley,
        ink,
    );
}

/// A bay's card: `.bay`'s panel fill, 11px radius and drop shadow.
fn card(ui: &Ui, pal: &Palette, rect: Rect) {
    let radius = CornerRadius::same(size::BAY_RADIUS as u8);
    ui.painter().add(pal.shadow.as_shape(rect, radius));
    ui.painter().rect_filled(rect, radius, pal.panel);
}

/// **The bay head, and the whole of what one is.**
///
/// A title on the left, the bay's own controls on the right, and a grip at the
/// far right where the mock draws one — laid out into the top
/// [`size::HEAD_H`] of `rect`, with `.bay-head`'s hairline under it.
///
/// Seven call sites on the day it is written, which is [`REGIONS`]'s seven
/// bays.
///
/// The pills and the grip are laid out **right to left** from the right edge,
/// which is what `justify-content: space-between` on a two-child flex row
/// comes to: the title takes the left and the group takes the right, and the
/// group's own order is its writing order once it is placed.
pub fn bay_head(
    ui: &Ui,
    pal: &Palette,
    rect: Rect,
    title: &str,
    pills: &[&str],
    grip: bool,
) -> Rect {
    let head = Rect::from_min_max(
        rect.min,
        Pos2::new(rect.max.x, (rect.min.y + size::HEAD_H).min(rect.max.y)),
    );
    let painter = ui.painter().with_clip_rect(head);

    // `border-bottom: 1px solid var(--c-hair)`.
    let rule = head.max.y - size::HAIRLINE * 0.5;
    painter.line_segment(
        [Pos2::new(head.min.x, rule), Pos2::new(head.max.x, rule)],
        Stroke::new(size::HAIRLINE, pal.hair),
    );

    let mid = head.center().y;
    let mut right = head.max.x - size::HEAD_PAD_X;

    if grip {
        right -= grip_dots(ui, pal, Pos2::new(right, mid));
        right -= size::PILL_GAP;
    }
    for pill in pills.iter().rev() {
        right -= pill_at(ui, pal, Pos2::new(right, mid), pill);
        right -= size::PILL_GAP;
    }

    // `text-transform: uppercase` plus `letter-spacing: 0.16em`, at
    // `--c-faint`. `egui`'s default proportional face has no bold, so
    // `font-weight: 700` is not honoured — see `room`'s documentation.
    let job = spaced(
        &title.to_uppercase(),
        size::HEAD_SIZE,
        pal.faint,
        size::HEAD_TRACKING,
    );
    let galley = painter.layout_job(job);
    painter.galley(
        Pos2::new(head.min.x + size::HEAD_PAD_X, mid - galley.size().y * 0.5),
        galley,
        pal.faint,
    );

    head
}

/// One `.pill`, right-aligned to `right`. Returns its width, so a caller
/// laying a row of them out right to left can step back by it.
fn pill_at(ui: &Ui, pal: &Palette, right: Pos2, text: &str) -> f32 {
    let painter = ui.painter();
    let galley = painter.layout_no_wrap(
        text.to_owned(),
        FontId::new(size::BASE, FontFamily::Proportional),
        pal.dim,
    );
    let w = galley.size().x + size::PILL_PAD_X * 2.0;
    let rect = Rect::from_min_size(
        Pos2::new(right.x - w, right.y - size::PILL_H * 0.5),
        egui::vec2(w, size::PILL_H),
    );
    painter.rect_stroke(
        rect,
        // `border-radius: 999px` on a box this short is a capsule.
        CornerRadius::same((size::PILL_H * 0.5) as u8),
        Stroke::new(1.0, pal.line),
        StrokeKind::Inside,
    );
    painter.galley(
        Pos2::new(
            rect.min.x + size::PILL_PAD_X,
            right.y - galley.size().y * 0.5,
        ),
        galley,
        pal.dim,
    );
    w
}

/// The mock's `.grip`, `⋮⋮` — drawn rather than typed, because whether a
/// vertical ellipsis is in `egui`'s default face is a question with no good
/// answer and six dots is the same mark either way. Returns its width.
fn grip_dots(ui: &Ui, pal: &Palette, right: Pos2) -> f32 {
    const COLS: usize = 2;
    const ROWS: usize = 3;
    const R: f32 = 1.0;
    const STEP: f32 = 3.5;
    let w = STEP * (COLS - 1) as f32 + R * 2.0;
    let h = STEP * (ROWS - 1) as f32;
    let painter = ui.painter();
    for c in 0..COLS {
        for r in 0..ROWS {
            painter.circle_filled(
                Pos2::new(
                    right.x - w + R + c as f32 * STEP,
                    right.y - h * 0.5 + r as f32 * STEP,
                ),
                R,
                pal.faint,
            );
        }
    }
    w
}

/// The mock's `.divider-v` between a bay's subdivisions: a `--c-hair` capsule
/// in the gap, below the head.
///
/// A no-op for every bay that is a leaf, which is six of the seven.
fn pane_dividers(ui: &Ui, pal: &Palette, panel: &Panel, id: NodeId, rect: Rect) {
    let layout = panel.layout();
    let Some(axis) = layout.axis(id) else {
        return;
    };
    let top = (rect.min.y + size::HEAD_H).min(rect.max.y);
    for (index, _) in layout.visible_children(id).enumerate().skip(1) {
        let Some(gap) = layout.boundary(id, index - 1) else {
            continue;
        };
        let gap = to_egui(gap);
        let bar = match axis {
            Axis::Row => {
                Rect::from_min_max(Pos2::new(gap.min.x, top), Pos2::new(gap.max.x, rect.max.y))
            }
            Axis::Column => Rect::from_min_max(
                Pos2::new(rect.min.x, gap.min.y.max(top)),
                Pos2::new(rect.max.x, gap.max.y.max(top)),
            ),
        };
        if bar.width() > 0.0 && bar.height() > 0.0 {
            ui.painter().rect_filled(
                bar,
                CornerRadius::same((size::PANE_DIVIDER * 0.5) as u8),
                pal.hair,
            );
        }
    }
}

/// A run of text with `letter-spacing`, which `egui` states per format run
/// rather than per style.
fn spaced(text: &str, size: f32, colour: Color32, tracking: f32) -> LayoutJob {
    let mut job = LayoutJob::default();
    job.append(
        text,
        0.0,
        TextFormat {
            font_id: FontId::new(size, FontFamily::Proportional),
            extra_letter_spacing: tracking,
            color: colour,
            ..Default::default()
        },
    );
    job
}

/// The arrangement's rectangle, in `egui`'s. Both are top-left origin in
/// logical pixels, so this is only two types meeting.
pub fn to_egui(r: karakuri_layout::Rect) -> Rect {
    Rect::from_min_size(Pos2::new(r.x, r.y), egui::vec2(r.w, r.h))
}
