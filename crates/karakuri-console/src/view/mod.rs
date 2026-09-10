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
//! region's own face. The mock's `.preview` is a bare well before it is a
//! picture — one of its four cells holds no texels at all — and the Program
//! bay's head reads *previews 3 of 4*, which is an operator's ordinary choice
//! and not a state waiting to be finished. **A cell with nothing behind it is
//! what that state looks like, not a stand-in for a full one**, and the
//! caption under it says which nothing: `material` where there is a picture
//! and `no slot` where there is not.
//!
//! So the two rules are one rule. Nothing is drawn that claims something
//! exists which does not; a cell with no slot behind it exists and says so.
//! See [`DECKS`], [`preview_rects`] and [`View::previews`], and
//! [ADR-0170](../../../../docs/adr/0170-a-deck-preview-cell-is-drawn-whether-or-not-a-deck-is-behind-it.md)
//! for the alternative that lost and for what would reopen it.
//!
//! # The Staging lane draws a row per node a build changed
//!
//! [ADR-0200](../../../../docs/adr/0200-a-bays-first-pass-draws-the-values-that-exist-and-omits-the-rest.md)
//! draws every part of the mock that has a value behind it and omits the
//! rest. Applied to this bay it draws **four** of the six things a candidate
//! row could carry — the deck, the node's address, what that node's procedure
//! calls itself, and whether it is on screen — and omits the other two
//! outright.
//!
//! **This section used to say the lane draws nothing, and the reason it gave
//! was true of the program rather than of the workspace.** Both halves of the
//! producer were already public and already wired elsewhere:
//! `karakuri_engine::deck::Deck::slot` hands out the `HotSwap` a verdict comes
//! out of, and `karakuri-cli` builds its slots with `HotSwap::new` over a
//! `karakuri_environment::watch::Watch`. What was true is narrower: the
//! program this panel is drawn by built both its deck slots with
//! `HotSwap::fixed`, whose `Receiver`'s `Sender` is dropped at construction —
//! so no `swap::Event` of any variant was emitted in any run of it, and a row
//! would have been
//! [ADR-0191](../../../../docs/adr/0191-the-panels-parked-deck-is-parked-by-the-governor-or-it-is-a-drawing-of-one.md)'s
//! parked chip. That was host wiring and it is now wired: `crates/karakuri`
//! watches its own `.kir` pair per slot, and a save is a build, a swap and a
//! verdict.
//!
//! **What a row is, and it is the node rather than the slot.** A `swap::Event`
//! carries an `id` and a `label` and no node, because a
//! `karakuri_engine::Request` restates *every* node of the slot — so a
//! **verdict** is over a build. Which node of that build **changed** is a
//! diff: `karakuri_environment::watch::Built` carries `(layer, index, hash)`
//! for the whole stack on every build, and consecutive builds differ where the
//! hashes do. The caller takes it where both lists are in one hand and hands
//! in one [`Candidate`] per changed node, with the build's one verdict on each
//! of them
//! ([ADR-0326](../../../../docs/adr/0326-a-staging-row-is-a-changed-node-and-the-row-is-the-keep.md)).
//! **A verdict with no changed node behind it draws one row on the slot with
//! no address** — a build that did not happen has no node list to hold against
//! the one before it, and a rebuild that restated the stack unchanged has an
//! empty diff.
//!
//! **The two omissions, each with what it waits on**, are written out at
//! [`staging`]: the coloured dot and the `you` in
//! `you, 14:41` (`origin` has no producer anywhere) and the `14:41` itself
//! (three spellings exist and none is chosen — the same decision the Library's
//! time column waits on). The head's `2 waiting` is not drawn either, because
//! a bay head's pills are its controls and the rows are the count. The mock's third `.cand` —
//! *a rejected candidate costs nothing* — is a note to whoever is reading the
//! mock and not a thing the lane draws, and empty this lane draws *"no row, no
//! placeholder, and no standing sentence"*, which is still every frame of
//! every run until somebody saves a file.
//!
//! **Both of the lane's operations are built, and they are the row and a
//! capsule in it.** `karakuri_operation::Operation` spells *Keep a candidate*
//! and *Put a node's previous version back* `{ deck, node }` apiece, and the
//! node is what the row now names. A press on the row keeps that candidate —
//! `Written::Silent(Silent::Surface)`, so nothing moves and the row leaves —
//! and the `back` capsule at its end lands that node's previous version, which
//! is a build like any other and writes its record at the swap
//! (`Written::Silent(Silent::OnLanding)`). The free act is on the large target
//! and the act that writes a file is on the small one. **What the row says
//! without either is still the thing nothing else in this instrument says**:
//! after `swap::Event::Overloaded` the version is still in the slot and the
//! slot has stopped updating, so what is on screen is a held frame and only a
//! word says so — the row's, and the deck cell's caption beside it — and the
//! capsule on that row is the way out.
//!
//! **The height is what this leaves wrong, and it is not this module's to
//! fix.** `lib.rs` pins the lane at `fixed(125.0)` with a minimum of `66.0` —
//! the mock's three `.cand` rows, and one — and empty is still this lane's
//! ordinary state, so those pixels are held open over nothing at the expense
//! of the Library, which is the bay in that column that absorbs. A
//! content-height lane needs `arrangement()` to take an argument, or
//! `karakuri-layout` to grow a setter for a view's size, and it has neither.
//! The numbers stay where they are, and what they buy is that a lane which
//! fills up has the room the mock gave it.
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
//! [ADR-0182](../../../../docs/adr/0182-the-program-bays-body-arranges-itself-for-the-larger-picture.md)
//! is the arrangement and
//! [ADR-0183](../../../../docs/adr/0183-a-node-is-out-of-the-layout-for-two-reasons-and-they-are-two-bits.md)
//! is the bit.
//!
//! The transport and the outputs are **rows, not bays**: they carry no
//! heading, because they have none in the mock — both carry `class="bay"`,
//! which is the card styling, and neither carries a `.bay-head`
//! ([ADR-0159](../../../../docs/adr/0159-the-consoles-words-are-the-manuals-and-the-middle-one-is-not-a-pane.md)).
//!
//! # The transport row is four readouts and nine controls
//!
//! **The four readouts are the beat grid, the bar, the frame readout and the
//! health capsule** — the things in the mock's row that are a value somebody
//! measured rather than a control over something that does not exist. All
//! four are [`Kind::Transport`]'s, which is [`transport`]. The capsule joined
//! them on 2026-09-08: it is drawn as a `.pill` and is no more a control than
//! the frame readout beside it, which is the reading a press proves rather
//! than the shape of the box.
//!
//! **The nine controls are six rows of [`crate::input::PROBES`]**, and the row
//! is where this module's rule has cost the most and bought the most back. Two
//! of them are [`transport`]'s own — the tempo figure
//! ([`TransportRow::tempo`]) and the `rec` pill ([`TransportRow::record`]),
//! which landed together and are why the tempo is not counted above: the
//! number **is** the track a press names a value on ([ADR-0291](../../../../docs/adr/0291-the-tempo-figure-is-the-track-and-the-band-is-a-guard-on-the-hand.md)).
//! The other seven are laid out from that row and derived beside it: the
//! audio-in pill ([`audio_in`]), the tracker group's tap, offset and octave
//! ([`tracker_group`]), the arrangement pill ([`arrangement`]), and the tone
//! map's capsule and exposure track ([`look`]). The ones the console still
//! cannot know are named in [`transport`], one by one, with what is missing
//! behind each.
//!
//! **The count is of what this crate draws and not of the mock**, which is
//! why it is written here at all: the nine are what those six probe rows claim
//! between them, so the figure is checkable against one table rather than
//! remembered. The mock's own total is not written anywhere in this module and
//! must not be — see the paragraph below.
//!
//! **The arrangement pill is the one that was here first, and it was here for
//! the rule rather than despite it.** Every control in this row that is still
//! undrawn is a control over machinery that is in neither this crate nor the
//! program — a learn mode with no map, a map file this program has not got.
//! The arrangement is not one of those: it is the tree this crate owns, it is
//! already saved and put back by name over a real file, and `r` already
//! resets it. So `arr · night ▾` is a control over something that exists,
//! which is the only test this module's rule has ever applied — see
//! [`arrangement`], [`Arrangement`] and
//! [ADR-0221](../../../../docs/adr/0221-an-arrangement-is-named-by-the-operator-and-kept-in-a-fourth-place.md).
//! It is the mock's own `map · nanoKONTROL2 ▾` in shape and in position,
//! because a named file with *save*, *load* and *start a new one* under one
//! control is exactly the family, and the map pill is the same three over a
//! file this program has not got.
//!
//! **No count is written here.** This comment said *six of the mock's ten* and
//! the mock has gained an offset pill and an octave pair since, which is the
//! third time a transcribed figure about that page has gone stale in this
//! repository. What the mock holds is
//! `docs/manual/console.html`'s `.transport` block, and counting it is a
//! command rather than a memory.
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
//! preview cell is the region's own face and says `no slot`; a strip is six
//! readings, and an empty one is six readings nobody took — the row of zeroes
//! ADR-0177 refuses, with a different glyph. See [`mixer`] and [`Strip`], and
//! [`Meter`] for why a meter is not a [`Fader`].
//!
//! **Five things in that bay answer a pointer and the rest are readouts**,
//! and the source says which rather than leaving the next reader to discover
//! it. The two fader knobs are played, and the blend mini, the tally chip and
//! the mask mini each cycle
//! ([ADR-0185](../../../../docs/adr/0185-a-fader-translates-a-drag-into-an-operation-and-applies-nothing.md),
//! [ADR-0187](../../../../docs/adr/0187-the-blend-mini-cycles-and-a-map-learns-the-three-it-cycles-through.md),
//! [ADR-0195](../../../../docs/adr/0195-the-tally-chip-cycles-from-the-request-so-the-parked-case-needs-no-case.md),
//! [ADR-0203](../../../../docs/adr/0203-the-mask-chip-carries-the-angle-it-does-not-control.md)):
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
//! [`Strip::pending`], [`roll_at`] and [`mixer::tally_into`], and
//! [P-0087](../../../../docs/principles/0087-name-the-property-never-the-shape.md)
//! for what that has to say. **A press on it cycles from the residency that
//! was *requested*** ([`Mixer::tally`]), which is what makes a parked chip's
//! press the withdrawal of its own prime request with no case in the code for
//! it. It is the panel's first live region and still its only
//! one ([`View::declares`], which answers with the bay's name, what one update
//! of it costs and how stale it may get) — and it declares only while the bay
//! it rolls in is laid out, because a fold takes the chip off the screen and a
//! price paid for what nobody can see is ADR-0193 broken rather than served.
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
use karakuri_operation::gate::{Class, Open};
use karakuri_operation::{
    Authority, BeatSource, BlendMode, GridScale, LaneTarget, Layer, LibraryKinds, NodeAt,
    Operation, Output, Recording, Residency, Revision, StepMode, Sync, Tonemap, TransitionSetting,
    Undecided, WipeKind,
};

use crate::budget::{Declared, PANEL_PASS};
use crate::focus::{self, Focus};
use crate::panel::{unit, Grab, InHand, Knob, Op, Panel, GRAB};
use crate::room::{size, Palette, Room};

mod mixer;

/// **The Mixer bay's own module**, split out under
/// [ADR-0121](../../../../docs/adr/0121-moving-code-leaves-its-reasoning-behind.md)'s
/// rule that a move carries its reasoning with it: every symbol below is
/// [`mixer`]'s, re-exported here so that nothing outside this crate that
/// already writes `view::mixer` or `view::TransitionRow` has to learn a
/// second path for it.
pub use mixer::{
    after, mixer, roll_at, roll_moves_in, transition, Fader, Go, Level, Mask, Meter, Mixer, Phase,
    Reach, Strip, StripBox, Tally, TransitionRow, TransitionSettings, ROLL_PERIOD, ROLL_REACH,
    ROLL_STALENESS, ROLL_TRAVEL,
};
pub(crate) use mixer::{next, next_shape, residency, wipe_kind};

mod library;

/// **The Library bay's own module**, split out under
/// [ADR-0121](../../../../docs/adr/0121-moving-code-leaves-its-reasoning-behind.md)'s
/// rule that a move carries its reasoning with it: every symbol below is
/// [`library`]'s, re-exported here so that nothing outside this crate that
/// already writes `view::library` or `view::LibraryBay` has to learn a
/// second path for it.
pub use library::{
    library, load_item, Aim, Block, Chosen, Field, Filters, KindChip, LibraryBay, Load, Menued,
    Opened, Picked, Pointed, Pointing, Published, Read, Reading, RowItem, RowKind, RowMenu, Rows,
    Scope, Taken, Target, Wiring, HOLDS_UNSET, LAYERS, SETS_CHIP,
};

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
/// and the Program bay's head reads *previews 3 of 4*. Two readings, one
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
/// [ADR-0179](../../../../docs/adr/0179-a-transcribed-number-cites-the-rule-it-was-copied-from.md)
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
        /// value the console does not have yet — `1920x1080`, `previews 3 of
        /// 4`, `3 of 3 - page 1`, `2 waiting` — and a pill reading `previews 3
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
    /// and the same [`bay_head`], carrying [`mixer::MIXER_TITLE`], no pill and no
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
    /// card and the same [`bay_head`], carrying [`library::LIBRARY_TITLE`], no pill and
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
    /// **The Master bay**, which is a bay in every other respect: the same
    /// card and the same [`bay_head`], carrying [`MASTER_TITLE`], no pill and
    /// the grip the mock draws in this head.
    ///
    /// A kind of its own for [`Kind::Mixer`]'s reason: [`View::draw`] has to
    /// know **which** bay the out row goes in, and the alternative is
    /// comparing a name on the frame path, which puts a string where the table
    /// already says what a region is.
    ///
    /// See [`master`] for what is drawn here, for where the level comes from,
    /// and for why the three effects the mock draws under the row are **not**
    /// drawn: they exist nowhere in this workspace, and a chain over machinery
    /// that is not there is the scaffolding this module refuses.
    Master,
    /// **The Staging lane**, which is a bay in every other respect: the same
    /// card and the same [`bay_head`], carrying [`STAGING_TITLE`], no pill and
    /// no grip — the mock gives this head a count and the console draws no
    /// readout in a bay head.
    ///
    /// A kind of its own for [`Kind::Mixer`]'s reason: [`View::draw`] has to
    /// know **which** bay the candidate rows go in, and the alternative is
    /// comparing a name on the frame path, which puts a string where the table
    /// already says what a region is.
    ///
    /// See [`staging`] for what is drawn here, for where the rows come from,
    /// and for the six things in the mock's lane and the page's row that are
    /// **not** drawn.
    Staging,
    /// **The Sequencer bay**, which is a bay in every other respect: the same
    /// card and the same [`bay_head`], carrying [`SEQUENCER_TITLE`], no pill
    /// and no grip — the mock gives this head three bank pills and the console
    /// draws none of them yet.
    ///
    /// A kind of its own for [`Kind::Mixer`]'s reason: [`View::draw`] has to
    /// know **which** bay the ruler, the rows and the playhead go in, and the
    /// alternative is comparing a name on the frame path, which puts a string
    /// where the table already says what a region is.
    ///
    /// See [`sequencer`] for what is drawn here, for where the pattern comes
    /// from, and for the three things in the mock's bay that are **not**
    /// drawn.
    Sequencer,
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

/// **The word on the Program bay's one pill**, in the mock's own spelling.
///
/// A constant rather than a literal in two places: [`REGIONS`] puts it in that
/// bay's head and [`program_head`] finds it there again, and a pill nobody can
/// find is a control that silently stops existing.
const SOLO_PILL: &str = "solo";

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

// ---------------------------------------------------------------------------
// The four class pills: what a model may reach, and the hand that opens it
// ---------------------------------------------------------------------------

/// **The word on a class's pill while the class is shut**, in the mock's own
/// spelling. `docs/manual/console.html` draws all four of them this way and
/// draws none of them any other way, because every class starts shut and shut
/// is the console the page draws.
const MCP_SHUT: &str = "mcp · shut";

/// **And the word while it is open**, which `docs/manual/console.html`
/// specifies under *what a model is refused, and where a class opens*: *"open,
/// it reads `mcp · open` and is drawn armed"*. It is the other half of a
/// sentence its four tooltips write — *"Click to open the class; click again to
/// shut it"* — rather than a second control. A pill reading `shut` in both
/// states would be a control that never answers a press, which is the one thing
/// those tooltips rule out.
const MCP_OPEN: &str = "mcp · open";

/// **Which of the two a class reads as.** Two constants and this, so that the
/// word a hand presses, the word [`crate::input`] measures the capsule by, and
/// the word a program prints in a legend are one string.
///
/// Exported for the third of those: the answer to *what does this pill say* is
/// this crate's and nobody else's, and a caller that wrote the words out again
/// would be the copy that goes stale — which is the defect `karakuri`'s startup
/// legend has already been repaired of once.
pub fn mcp_word(open: bool) -> &'static str {
    match open {
        true => MCP_OPEN,
        false => MCP_SHUT,
    }
}

/// **Where a class's pill is drawn**, as the arrangement's own name for the
/// region.
///
/// [`Class::bay`] is the same answer in the words a *refusal* says it in —
/// `Program`, `Mixer`, `Master`, `Outputs` — and these are the regions those
/// name. A `match` rather than a lowercase of that function, so a fifth class
/// stops the build here instead of looking for a region nobody has drawn;
/// `tests/mcp_pill.rs` asserts the two answers agree, in both directions.
pub fn opens(class: Class) -> &'static str {
    match class {
        Class::LiveDeck => "program",
        Class::MixFaders => "mixer",
        Class::MasterEffects => "master",
        Class::InputsAndOutputs => "outputs",
    }
}

/// **The class a region opens**, or `None` for the nine regions that open none.
///
/// **A bay carrying no class draws no pill**, which is the answer that commits
/// to neither of the two ADR-0235 leaves open: it lists *"whether a bay that
/// carries no class draws the indicator at all"* as undecided, and the manual
/// says why it matters — *"an indicator that is present everywhere reads as a
/// state wherever it is absent"*. Drawing nothing at the Library, the
/// Inspector, Staging and the Sequencer is what the page draws.
pub fn class_at(region: &str) -> Option<Class> {
    Class::ALL
        .iter()
        .copied()
        .find(|class| opens(*class) == region)
}

/// **A bay head's furniture**: the word in it, the controls the mock draws in
/// it, whether it carries a grip, and the class it opens.
///
/// [`Kind::Bay`] carries the first three in [`REGIONS`] and the four bays that
/// are kinds of their own — the Mixer, the Master, the Library and the Staging
/// lane — carried them nowhere, because [`View::draw`] wrote them out at its
/// own call to [`bay_head`]. **That was one copy while nothing but the paint
/// needed them and it is four copies now**: [`mcp_pill`] has to lay a head's
/// pills out again to find the class capsule among them, and a head whose words
/// the paint and the press disagreed about is precisely the defect
/// [`head_pills`] was written against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Head {
    /// The mock's own capitalisation; `.bay-head` upper-cases at paint time.
    pub title: &'static str,
    /// The controls the table lists in this head, and only the controls.
    pub pills: &'static [&'static str],
    /// Whether the mock draws a `.grip` at the right of it.
    pub grip: bool,
    /// The class this head opens, where it opens one — [`class_at`].
    pub class: Option<Class>,
    /// **The bank pills, and which of the four is armed** — `None` for every
    /// head but the Sequencer's, which is the one head in [`REGIONS`] whose
    /// capsules are a *value* rather than a table entry.
    ///
    /// # Why the head machinery grew a field rather than the table growing a row
    ///
    /// [`Head::pills`] is `&'static [&'static str]`, which is every capsule
    /// this console had until 2026-09-09: a word chosen when the region was
    /// written. The Sequencer's `seq 1 … seq 4` are four words that are always
    /// the same and **one mark that is not** — which bank the lanes are reading
    /// — and [`view::sequencer`](sequencer)'s own note named that as the
    /// missing piece: *"what is missing is a head that can say which pill is
    /// armed"*.
    ///
    /// **It is [`mcp_pill`]'s arrangement and not a second one.** A class pill
    /// carries a state into a head already; it does it by [`mcp_word`], because
    /// its two states have two words. A bank's do not — `seq 2` reads `seq 2`
    /// whether it is armed or not — so what a bank needs is the other half of
    /// what the class pill already gets: [`HeadWords::armed`], which is where
    /// *is this capsule lit* is answered for both of them now.
    ///
    /// **Every other head is `None` and its [`HeadWords`] is what it was**, to
    /// the word and to the bit; `tests/head_words.rs` renders every region's
    /// head before and after and compares.
    pub banks: Option<usize>,
}

/// **The head a region draws**, or `None` where it has none.
///
/// The transport and the Outputs row are headless (ADR-0159), and a pane, the
/// picture and the preview row are inside a bay that has one. **The Outputs row
/// answering `None` here is what makes the fourth class pill a placement of its
/// own** — see [`outputs`], which is where it goes instead.
pub fn head_of(region: &Region) -> Option<Head> {
    let (title, pills): (_, &'static [&'static str]) = match region.kind {
        Kind::Bay { title, pills, .. } => (title, pills),
        Kind::Mixer => (mixer::MIXER_TITLE, &[]),
        Kind::Master => (MASTER_TITLE, &[]),
        Kind::Library => (library::LIBRARY_TITLE, &[]),
        Kind::Staging => (STAGING_TITLE, &[]),
        Kind::Sequencer => (SEQUENCER_TITLE, &[]),
        Kind::Transport | Kind::Outputs | Kind::Pane | Kind::Picture | Kind::Previews => {
            return None
        }
    };
    Some(Head {
        title,
        pills,
        grip: head_grip(region.kind),
        class: class_at(region.name),
        // **No head is born with banks**, which is what keeps this table a
        // table: the armed bank is a value the host writes per frame, so the
        // one head that draws them asks for them — [`Head::with_banks`].
        banks: None,
    })
}

/// **Whether a region's head draws a grip**, said once and in a form a `const`
/// can ask.
///
/// It was the third term of [`head_of`]'s own match, which was one statement
/// while the paint was the only reader of it. [`BAY_GRIPS`] is the second
/// reader and it is a `const`: [`crate::input`]'s table says how many controls
/// a probe reaches, that number is *how many heads draw one of these*, and a
/// hand-written four is the shape of thing this crate counts rather than
/// remembers ([`REGIONS`], `Scope::ALL`, `Class::ALL`). A `bool` per kind in
/// two places would be two answers to *does this head carry a grip*, and the
/// day they disagreed the mark and the control would be in different heads.
///
/// **`Kind::Bay` carries its own**, because [`REGIONS`] states it there; the
/// four bays that are kinds of their own state it here, which is where they
/// stated it before. Everything headless answers `false` rather than being
/// unreachable, so the count above is a walk over every region and not over a
/// subset somebody has to keep.
const fn head_grip(kind: Kind) -> bool {
    match kind {
        Kind::Bay { grip, .. } => grip,
        // The mock draws one in these two heads and not in the other two —
        // see each kind's own documentation, which is where the reading is.
        Kind::Library | Kind::Master => true,
        Kind::Mixer | Kind::Staging | Kind::Sequencer => false,
        Kind::Transport | Kind::Outputs | Kind::Pane | Kind::Picture | Kind::Previews => false,
    }
}

/// **How many bay heads draw a grip**, counted off [`REGIONS`] — which is how
/// many bays a pointer can fold, because the grip is the control ([`bay_grip`]).
///
/// Four, on the day it is written: the Library, the Program bay, the Inspector
/// and the Master. It is a count and not a four for [`crate::input::PROBES`]'
/// reason — the row that registers this control says how many controls it
/// reaches, and a grip drawn in a fifth head has to raise that number on its
/// own rather than wait for somebody to notice.
pub const BAY_GRIPS: usize = grips();

/// [`BAY_GRIPS`], in a `const` — which `Iterator::filter` is not.
const fn grips() -> usize {
    let mut total = 0;
    let mut at = 0;
    while at < REGIONS.len() {
        if head_grip(REGIONS[at].kind) {
            total += 1;
        }
        at += 1;
    }
    total
}

/// **How many capsules one bay head can hold**: the most any entry in
/// [`REGIONS`] lists — the Program bay's `solo` — plus the class pill, plus
/// the bank pills the Sequencer's head draws off a value ([`Head::banks`]).
///
/// **The four are added rather than maxed**, which is deliberately the
/// pessimistic reading: nothing says a head with banks may not also list a
/// control and open a class, and a bound that assumed otherwise would drop a
/// capsule off the end of [`HeadWords`] in silence the day one did — which is
/// exactly what the `const` assertion below exists to stop.
const HEAD_PILLS: usize = 2 + karakuri_pattern::BANKS;

/// And every entry fits **with everything a head can be handed beside it** —
/// the class pill and the four bank pills. A head listing one more control than
/// that would lose a capsule off the end of [`HeadWords`] silently, which is a
/// control that stops existing rather than a build that stops.
const _: () = {
    let mut at = 0;
    while at < REGIONS.len() {
        if let Kind::Bay { pills, .. } = REGIONS[at].kind {
            assert!(pills.len() + 1 + karakuri_pattern::BANKS <= HEAD_PILLS)
        }
        at += 1;
    }
};

/// **Every capsule in a head this frame**: the table's own controls, and then
/// the class pill where the bay opens a class.
///
/// **The class pill is last, which is rightmost.** [`head_pills`] lays a head
/// out right to left, and `docs/manual/console.html` draws the Program bay's
/// head as `1920×1080`, `solo`, `mcp · shut`, `previews 3 of 4` — so the
/// opening sits to the right of `solo`. That is read off the page rather than
/// chosen here, and it is why `solo`'s own capsule moves when a class is
/// opened: the two words are not the same width, and one derivation answering
/// for both is what keeps the pill an operator sees and the pill a press lands
/// on the same rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeadWords {
    words: [&'static str; HEAD_PILLS],
    /// **Which of them are lit**, in the same order — see [`HeadWords::armed`].
    armed: [bool; HEAD_PILLS],
    len: usize,
}

impl HeadWords {
    /// The words, in the order [`head_pills`] takes them.
    pub fn as_slice(&self) -> &[&'static str] {
        &self.words[..self.len]
    }

    /// **Whether the capsule at `index` is drawn `.pill.armed`**, which is one
    /// question with two answers behind it and used to be asked at the paint.
    ///
    /// [`bay_head`] read `words[index] == MCP_OPEN`, which works only while
    /// *lit* and *this word* are the same fact. They are for a class pill,
    /// whose two states are two words ([`mcp_word`]); they are not for a bank
    /// pill, which reads `seq 2` armed and `seq 2` plain. So the bit is derived
    /// where the word is, beside it, and the paint reads one answer instead of
    /// re-deriving one of them.
    ///
    /// `false` past the end, which is a capsule that is not there.
    pub fn armed(&self, index: usize) -> bool {
        self.armed.get(index).copied().unwrap_or(false)
    }
}

impl Head {
    /// **This head with `armed` of its four bank pills lit** — the one door
    /// into [`Head::banks`], so a head that draws banks is a head somebody
    /// handed a value to.
    pub fn with_banks(self, armed: usize) -> Head {
        Head {
            banks: Some(armed),
            ..self
        }
    }

    /// This head's capsules under `open` — see [`HeadWords`].
    pub fn words(&self, open: Open) -> HeadWords {
        let mut words = [""; HEAD_PILLS];
        let mut armed = [false; HEAD_PILLS];
        let mut len = 0;
        for pill in self.pills.iter().take(HEAD_PILLS) {
            words[len] = pill;
            len += 1;
        }
        // **The banks sit between the table's controls and the class pill**,
        // which keeps the class pill rightmost — the rule this type's own
        // documentation states and reads off the mock.
        if let Some(at) = self.banks {
            for (bank, word) in BANK_PILLS.iter().enumerate() {
                words[len] = word;
                armed[len] = bank == at;
                len += 1;
            }
        }
        if let Some(class) = self.class {
            words[len] = mcp_word(open.holds(class));
            armed[len] = open.holds(class);
            len += 1;
        }
        HeadWords { words, armed, len }
    }
}

/// **The bank pills' words**, one per `karakuri_pattern::BANKS` — `seq 1` …
/// `seq 4`, counting from one as everything an operator reads on this console
/// does.
///
/// A `const` array sized off that crate's own count, so a fifth bank is a
/// missing word here rather than a pill nobody draws.
const BANK_PILLS: [&str; karakuri_pattern::BANKS] = ["seq 1", "seq 2", "seq 3", "seq 4"];

/// **One capsule in a bay head, by the word in it** — [`head_pills`]'s answer,
/// asked for one pill rather than for all of them.
///
/// The one derivation three readers share: [`bay_head`] paints from it,
/// [`program_head`] finds `solo` in it and [`mcp_pill`] finds the class pill in
/// it. `None` where the head is too short to hold the capsule, which is a bay
/// clipped shorter than its own head — a capsule half out of a head is not one
/// to press.
fn head_capsule(
    ctx: &egui::Context,
    rect: Rect,
    head: &Head,
    open: Open,
    want: &str,
) -> Option<Rect> {
    let words = head.words(open);
    let words = words.as_slice();
    let mut found = None;
    head_pills(ctx, rect, words, head.grip, |index, capsule| {
        if words[index] == want {
            found = Some(capsule);
        }
    });
    found.filter(|capsule| head_box(rect).contains_rect(*capsule))
}

/// **Where each of a head's bank pills is**, in bank order — [`head_capsule`]
/// asked by position instead of by word, because four capsules are wanted and
/// not one.
///
/// **By index and not by word**, which is the difference that matters: the four
/// bank words are distinct today, and a control found by the word in it is a
/// control that moves the day two capsules read the same thing. The class pill
/// can be found by its word because [`mcp_word`]'s two are the state; a bank's
/// is not ([`HeadWords::armed`]).
///
/// **Empty for a head that draws no banks, and empty for one that cannot hold
/// all four** — a capsule half out of a head is not one to press
/// ([`head_capsule`]'s own filter), and dropping the ones that fit would
/// renumber the rest: the pills are laid out right to left, so the capsule that
/// falls off is `seq 1` and every bank after it would answer for its neighbour.
fn bank_capsules(ctx: &egui::Context, rect: Rect, head: &Head, open: Open) -> Vec<Rect> {
    if head.banks.is_none() {
        return Vec::new();
    }
    let words = head.words(open);
    let slice = words.as_slice();
    // Where the banks start: the table's own controls come first, and this
    // head has none of them today — read off the same walk rather than
    // assumed, so a Sequencer head that ever lists a control still finds them.
    let first = head.pills.len().min(slice.len());
    let box_of = head_box(rect);
    let mut out = vec![None; karakuri_pattern::BANKS];
    head_pills(ctx, rect, slice, head.grip, |index, capsule| {
        if let Some(bank) = index.checked_sub(first) {
            if let Some(slot) = out.get_mut(bank) {
                *slot = Some(capsule).filter(|it| box_of.contains_rect(*it));
            }
        }
    });
    match out.iter().all(Option::is_some) {
        true => out.into_iter().flatten().collect(),
        false => Vec::new(),
    }
}

// ---------------------------------------------------------------------------
// The fold a pointer reaches by a rectangle: the grip in a bay's head
// ---------------------------------------------------------------------------

/// **A fold, as a rectangle to press and the node it folds** — the console's
/// own shape, reached from the panel instead of from `f`.
///
/// # It was two controls and it is one, because a pane stopped needing a shape
///
/// [ADR-0295](../../../../docs/adr/0295-the-grip-is-the-fold-and-a-panes-outer-edge-is-the-other-one.md)
/// gave *Fold a pane away* a second derivation here — `pane_edge`, a band
/// `GRAB` deep on the pane's outer edge with nothing drawn in it — and it was
/// never registered, because that band lay over the outer three pixels of
/// every row of the Library bay's list.
/// [ADR-0300](../../../../docs/adr/0300-a-pane-folds-by-dragging-its-boundary-out-and-comes-back-by-dragging-it-in.md)
/// replaced it with a **drag**: a pane's boundary pulled out past the pane's
/// own minimum closes it, and the divider it leaves behind at the window's
/// edge is what pulls it back. There is no band while the pane is open,
/// nothing overlaps a library row, and the control needs no derivation at all
/// — a boundary is `karakuri_console::input`'s rule 3, claimed before any
/// control is asked. So this type is one control now: the grip, which never
/// had the conflict.
///
/// **The two are still one row's worth of operation apiece and one variant
/// here** — [`Op::Fold`] of a node — and `tests/vocabulary.rs`'s `rows_of`
/// gives it both of *Fold a bay away* and *Fold a pane away*.
///
/// # There is no unfold on it, and that is the arrangement rather than a gap
///
/// [`Outputs::op`] and [`ProgramHead::op`] each choose between two operations,
/// because the thing they act on is still on screen when it is off. **A folded
/// node has no rectangle**, so this control is not drawn once its press has
/// landed: the grip goes with the head it is in. That is `Op`'s own sentence
/// about the pointer —
/// *"a folded region has no rectangle, so the pointer could never be over one,
/// so `f` on the keyboard only ever folded"* — met by a control instead of by
/// a key, and the way back is `z` for the same reason it is for `f`.
///
/// # A fold writes no session record
///
/// `karakuri-operation-record` answers `Silent::Surface` for all four fold
/// operations — *a surface's own state* — so nothing here routes through the
/// window's `written`, and there is no `Operation` for it to route as: this is
/// [`Op`] and the two unnamed splits are why (ADR-0204).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FoldGrip {
    /// **The control**: what a press has to land in.
    pub grip: Rect,
    /// The node a press folds.
    pub id: NodeId,
}

impl FoldGrip {
    /// **What a press asks for**, and there is one answer — see the type's own
    /// documentation for why this is not the two [`ProgramHead::op`] chooses
    /// between.
    pub fn op(&self) -> Op {
        Op::Fold(self.id)
    }

    /// Whether `p` is on the control.
    pub fn hit(&self, p: karakuri_layout::Point) -> bool {
        self.grip.contains(Pos2::new(p.x, p.y))
    }
}

/// **The grip in a bay head, derived** — the mark `docs/manual/console.html`
/// draws in four of the seven heads this console has, and the panel's route
/// into *Fold a bay away*.
///
/// `None` where there is nothing to press: a region this console draws no head
/// for, a head with no grip in it, a bay that is folded or off a solo
/// somewhere else, or a bay clipped shorter or narrower than the target.
///
/// # The mark is the control, and it is not given a second shape
///
/// The page's lede says what the mark means — *"The grip in a bay's head says
/// whose size you are setting: a bay with one is yours to size, and a bay
/// without one is the height of what is in it"* — and the operations page puts
/// *Fold a bay away* at the **bay head**. A fold is the far end of setting that
/// height, so the reading a press takes is the reading the mark already
/// carries. **A *reading* which is a control does not get a second shape
/// invented for it**
/// ([ADR-0291](../../../../docs/adr/0291-the-tempo-figure-is-the-track-and-the-band-is-a-guard-on-the-hand.md)),
/// so nothing new is drawn here: [`bay_head`] paints the same six dots it
/// painted before this function existed.
///
/// **What that costs is the three heads the mock draws no grip in** — the
/// Mixer, the Staging lane and the Sequencer, and the two headless rows beside
/// them — which have no pointer route into this row and keep `f`. Drawing a
/// grip in every head would buy them one and would be an edit to the page
/// rather than to this function: the mark would stop saying which side of a
/// divider the number belongs to, which is the whole of what the lede says it
/// is for.
/// [ADR-0295](../../../../docs/adr/0295-the-grip-is-the-fold-and-a-panes-outer-edge-is-the-other-one.md)
/// is where that is taken.
///
/// # Where the rectangle is, and the 0.75 of a pixel it shares with `solo`
///
/// The mark is [`GRIP_W`] = **5.5** wide and 9 tall, which is not a target a
/// hand finds — the same sentence [`GRAB`] is written under, and the same one
/// [`LookRow::grip`] answers with a band. So the target is **the head's own
/// reservation for the mark, grown to a line's height**: [`head_pills`] steps
/// `GRIP_W + PILL_GAP` back from [`size::HEAD_PAD_X`] before it places a
/// capsule, so that **10.5** is the strip of head no other control can ever be
/// drawn in, and this is it — 10.5 x [`size::PILL_H`], hard against the head's
/// right-hand padding and centred on the head's mid-line, which is the box
/// every other capsule in this head is placed in.
///
/// **The reservation rather than a pill's padding**, and the capsule beside it
/// is what settles it: a `.pill` is its content inside `padding: 0 8px`, and
/// 5.5 + 16 = 21.5 would reach 6 pixels into the capsule [`head_pills`] places
/// one [`size::PILL_GAP`] to the left of the mark. The reservation ends exactly
/// where that capsule ends, so the two **abut and never overlap** — two
/// rectangles sharing an edge, which is what two rows of the Library bay's list
/// already are, and the capsule is asked first in both the claim and the
/// caller.
///
/// **What it takes that is not the mark is the gap**, and the gap is what the
/// head keeps between the mark and the capsule so that a hand aiming at one
/// does not land on the other. Giving it to the fold is what makes the mark
/// findable at all; giving it to nobody would be a control 5.5 wide, which is
/// the thing this paragraph starts by refusing.
///
/// **Sideways it clears every boundary**: [`size::HEAD_PAD_X`] is 10 against a
/// [`GRAB`] of 6, and that padding is the page's. **Down the head it is
/// `program_head`'s own 0.75 of a pixel**, arrived at the same way and for the
/// same reason: a head is [`size::HEAD_H`] = 27 and the target is 16.5,
/// centred, so 5.25 of head sits above it against a `GRAB` of 6, and a bay
/// whose top edge is a boundary lends that boundary the top 0.75 of the
/// target. Rule 3 gives the boundary first refusal and there is no case where
/// both think they are dragging; `tests/fold_grip.rs` measures it by asking
/// [`karakuri_layout::Layout::hit`] at the target's own corners rather than by
/// doing the arithmetic again.
///
/// **No `egui` and no first frame.** The grip is six dots at a fixed width, so
/// nothing here is as wide as a word — this is the one control in a bay head
/// that costs no galley and exists on the frame before anything has been
/// laid out. `layout` must be solved.
pub fn bay_grip(layout: &karakuri_layout::Layout, name: &str) -> Option<FoldGrip> {
    if !head_grip(region(name)?.kind) {
        return None;
    }
    let id = layout.find(name)?;
    // The bay itself, a column folded around it, or a solo somewhere else: one
    // question for every ancestor, which is [`program_head`]'s own guard.
    if !layout.visible(id) {
        return None;
    }
    let head = head_box(to_egui(layout.rect(id)));
    let mid = head.center().y;
    let right = head.max.x - size::HEAD_PAD_X;
    let grip = Rect::from_min_max(
        Pos2::new(right - GRIP_W - size::PILL_GAP, mid - size::PILL_H * 0.5),
        Pos2::new(right, mid + size::PILL_H * 0.5),
    );
    // A head clipped to a bay too short or too narrow to hold the target draws
    // no control rather than half of one — [`head_capsule`]'s own guard, one
    // capsule along.
    head.contains_rect(grip).then_some(FoldGrip { grip, id })
}

/// **A class's pill, derived**: the capsule, the class it opens, and whether
/// that class is open now.
///
/// # It has no `op`, and that is the decision rather than an omission
///
/// [`Outputs::op`] and [`ProgramHead::op`] both answer *what does a press ask
/// for* with a named [`Op`], and every other control on this panel answers with
/// an [`Operation`]. **This one answers with neither**, and
/// [ADR-0236](../../../../docs/adr/0236-a-map-is-the-layer-between-a-surface-and-the-vocabulary-and-the-audit-is-one-of-the-things-it-does.md)
/// is why: the opening is **configuration of the map** — the layer every
/// surface reaches the vocabulary through — and not a member of the vocabulary
/// the map addresses. The rule it draws is narrower than *map configuration is
/// never an operation*, because [`Operation::PointLane`] already is one: **a
/// setting that decides whether a surface may reach a class of operations
/// cannot itself be one of those operations**, since rule 01 would then make it
/// reachable from the surface it governs, and a permission an actor can grant
/// itself is not a permission.
///
/// So a press hands back a **value** — [`McpPill::next`] — and whoever holds
/// the run's `karakuri_environment::Opening` writes it there. This crate takes
/// no such handle and names nothing in that package (ADR-0156); what it does
/// name is [`Open`], which is `karakuri-operation`'s and is the leaf every
/// surface already depends on.
///
/// # Refused rather than hidden, which is why the pill is only ever a pill
///
/// Nothing on this console is turned off while a class is shut. ADR-0235:
/// *"the list never shortens and the call is refused"*, and the manual says it
/// of the operator too — *"Nothing here is ever refused to a hand."* So this
/// control draws a word and changes no other control's state, and a reader
/// looking for the half of it that greys something out will not find one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct McpPill {
    /// **The control**: the capsule a press has to land in, which is the
    /// rectangle [`bay_head`] or [`outputs_into`] painted.
    pub pill: Rect,
    /// The class it opens.
    pub class: Class,
    /// Whether that class is open — read out of the opening handed in, and kept
    /// nowhere here.
    pub open: bool,
}

impl McpPill {
    /// **What a press asks for**, and it is a state rather than a direction:
    /// the opening handed in, with this one class set the other way.
    ///
    /// [`Open::with`] *"names a state and never a direction"* (P-0090), and the
    /// toggle is this method choosing which state it means — the same
    /// affordance-over-a-named-thing [`Outputs::op`] and [`ProgramHead::op`]
    /// have, one layer out of the vocabulary.
    ///
    /// **It takes the opening rather than holding it**, so a press writes the
    /// other three classes back exactly as they were: a control that returned a
    /// bare `bool` would leave the caller to compose the value, which is the
    /// one place three classes could quietly be shut by a press on the fourth.
    pub fn next(&self, open: Open) -> Open {
        open.with(self.class, !self.open)
    }

    /// Whether `p` is on the control.
    pub fn hit(&self, p: karakuri_layout::Point) -> bool {
        self.pill.contains(Pos2::new(p.x, p.y))
    }
}

/// **One of the four class pills, derived**, or `None` where there is none to
/// press — before the first frame, with the bay folded, off a solo somewhere
/// else, or in a bay too short to hold its own head.
///
/// **Three of the four sit in a bay head** and come out of [`head_capsule`],
/// which is [`head_pills`] asked a second time rather than copied. **The fourth
/// sits in a row that has no head at all** — [`Kind::Outputs`] is a headless
/// strip (ADR-0159) — so it comes out of [`outputs`], beside the word that
/// stands in for a head. `Class::opened_at` is the gate saying the same thing
/// in the words a refusal uses: three are *the head of the … bay* and one is
/// *the Outputs row, which has no head*.
///
/// `layout` must be solved. `ctx` is asked for the type, because a `.pill` is
/// as wide as the word in it — and the two words are not the same width, which
/// is why this takes the opening rather than reading it back off anything.
pub fn mcp_pill(
    ctx: &egui::Context,
    layout: &karakuri_layout::Layout,
    class: Class,
    open: Open,
) -> Option<McpPill> {
    // Fonts are not valid until `egui` has run a pass, exactly as in
    // [`outputs`] and [`program_head`]: a press before the first frame is a
    // press on a control that has never been drawn.
    if ctx.cumulative_pass_nr() == 0 {
        return None;
    }
    let name = opens(class);
    let node = layout.find(name)?;
    // The region itself, a column folded around it, or a solo somewhere else:
    // one question for every ancestor, which is [`program_head`]'s own guard.
    if !layout.visible(node) {
        return None;
    }
    let pill = match head_of(region(name)?) {
        Some(head) => head_capsule(
            ctx,
            to_egui(layout.rect(node)),
            &head,
            open,
            mcp_word(open.holds(class)),
        )?,
        None => outputs(ctx, layout, open)?.mcp,
    };
    Some(McpPill {
        pill,
        class,
        open: open.holds(class),
    })
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
/// that takes a device is the program's.
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
/// this is its picture. See [ADR-0182](../../../../docs/adr/0182-the-program-bays-body-arranges-itself-for-the-larger-picture.md).
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
/// [ADR-0170](../../../../docs/adr/0170-a-deck-preview-cell-is-drawn-whether-or-not-a-deck-is-behind-it.md)
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
/// [ADR-0182](../../../../docs/adr/0182-the-program-bays-body-arranges-itself-for-the-larger-picture.md)'s
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
    //
    // **The caption band comes off the track before the image is fitted**, so
    // what this answers is the image and never the image plus its label — see
    // [`caption_band`] and [`caption_of`].
    std::array::from_fn(|deck| {
        fitted(
            above_caption(track(row, DECKS, deck, size::PREVIEW_GAP, Axis::Row)),
            PREVIEW_ASPECT,
        )
    })
}

/// **How much of a cell the caption takes**: `.cell`'s gap and `.caption`'s
/// own height, which is the band under every image and is never inside one.
///
/// One function rather than the sum written three times — [`preview_row`],
/// [`beside`] and [`caption_of`] all need it and a second copy of it is where
/// a caption would land over the picture it labels.
fn caption_band() -> f32 {
    size::PREVIEW_CAPTION_GAP + size::PREVIEW_CAPTION_H
}

/// `slot` with the caption band taken off the bottom, which is the box an
/// image is fitted into.
fn above_caption(slot: Rect) -> Rect {
    Rect::from_min_max(slot.min, Pos2::new(slot.max.x, slot.max.y - caption_band()))
}

/// **Where a cell's caption goes**: directly under the image, the image's own
/// width, one [`size::PREVIEW_CAPTION_GAP`] below it and
/// [`size::PREVIEW_CAPTION_H`] tall.
///
/// Derived from the image rather than carried beside it in [`ProgramBay`], for
/// the reason [`Body`] gives about the picture and the cells: the two are one
/// statement. A caption that could be handed in separately is a caption that
/// could be handed in stale, and this way there is one rectangle in the world
/// and the label is a function of it. It is also what keeps
/// [`preview_rects`]'s answer the **image** — the engine sizes a texture from
/// that rectangle, and a cell rectangle that quietly included the caption
/// would put texels over the letter.
///
/// The image's width and not the track's: the image is centred in its track
/// ([ADR-0170](../../../../docs/adr/0170-a-deck-preview-cell-is-drawn-whether-or-not-a-deck-is-behind-it.md)),
/// and a caption starting at the track's left edge would sit off under the
/// ground beside the cell it names.
///
/// # *Cell* means two things here, and it is named rather than renamed
///
/// The mock's `.cell` is the image **and** the caption — that is what
/// [`size::PREVIEW_ROW_H`] measures — while [`ProgramBay::cells`] and
/// [`preview_rects`] answer the **images**, because a texture is sized from
/// one and a rectangle that quietly included the caption would put texels over
/// the letter. Renaming the field is a ripple through six test files and the
/// program's frame path, so the clash is written down here instead
/// ([`docs/contributing.md`](../../../../docs/contributing.md) §4
/// is the rule it is in tension with, and this is the report rather than the
/// fix).
pub fn caption_of(image: Rect) -> Rect {
    Rect::from_min_max(
        Pos2::new(image.min.x, image.max.y + size::PREVIEW_CAPTION_GAP),
        Pos2::new(image.max.x, image.max.y + caption_band()),
    )
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
/// [ADR-0181](../../../../docs/adr/0181-the-picture-is-the-canvass-shape-and-the-leftover-is-the-consoles.md)
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
/// [P-0082](../../../../docs/principles/0082-looking-never-writes-back.md)
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
/// The mock's head reads *previews 3 of 4* and
/// [ADR-0170](../../../../docs/adr/0170-a-deck-preview-cell-is-drawn-whether-or-not-a-deck-is-behind-it.md)
/// is the answer: a cell is drawn whether or not a deck is behind it, because
/// *an empty cell is what off looks like, not a stand-in for a full one*. So
/// the question *does the left column fill first, or do they alternate* has a
/// third answer, and it is the one that keeps the letters meaning something:
/// **a cell's place is its deck's, not its turn's.** `C · no slot` sits at the top
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
pub fn program_body(body: Rect, canvas: (u32, u32)) -> Option<Body> {
    program_body_with_row_h(body, canvas, size::PREVIEW_ROW_H)
}

/// Dynamic row height version of program_body
pub fn program_body_with_row_h(body: Rect, canvas: (u32, u32), row_h: f32) -> Option<Body> {
    match (below(body, canvas, row_h), beside(body, canvas, row_h)) {
        (Some(below), Some(beside)) if area(beside.picture) > area(below.picture) => Some(beside),
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
/// The row is `row_h` tall, along `.program-body`'s bottom edge;
/// the picture takes what is left above it, less one `PROGRAM_DIVIDER`.
fn below(body: Rect, canvas: (u32, u32), row_h: f32) -> Option<Body> {
    if body.height() <= row_h + crate::PROGRAM_DIVIDER {
        return None;
    }
    let row = Rect::from_min_max(Pos2::new(body.min.x, body.max.y - row_h), body.max);
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
/// Preserves the preview cell size from the row arrangement (ADR-0239).
fn beside(body: Rect, canvas: (u32, u32), row_h: f32) -> Option<Body> {
    let (aw, ah) = (
        PREVIEW_ASPECT.0.max(1) as f32,
        PREVIEW_ASPECT.1.max(1) as f32,
    );
    // `row_h` is a whole cell — the image and the caption band under it — so
    // the image is what is left when the band comes off, exactly as it is in
    // the row. A cell beside the picture carries its caption too: the letter
    // is the only thing naming a deck and it does not stop naming one because
    // the bay went wide.
    let cell_h = (row_h - caption_band()).min(body.height());
    let cell_w = (cell_h * aw / ah).round();
    let column = cell_w;

    let min_w = column * 2.0 + crate::PROGRAM_DIVIDER * 2.0;
    let total_cells_h =
        (cell_h + caption_band()) * PER_COLUMN as f32 + size::PREVIEW_GAP * (PER_COLUMN - 1) as f32;
    if body.width() <= min_w || body.height() < total_cells_h {
        return None;
    }

    let picture = fitted(
        Rect::from_min_max(
            Pos2::new(body.min.x + column + crate::PROGRAM_DIVIDER, body.min.y),
            Pos2::new(body.max.x - column - crate::PROGRAM_DIVIDER, body.max.y),
        ),
        canvas,
    );

    let top_offset = ((body.height() - total_cells_h) / 2.0).max(0.0).round();
    let cells = std::array::from_fn(|deck| {
        let col_idx = deck / PER_COLUMN; // 0 for left (A, B), 1 for right (C, D)
        let row_idx = deck % PER_COLUMN; // 0 for top (A, C), 1 for bottom (B, D)
        let x = match col_idx {
            0 => body.min.x,
            _ => body.max.x - column,
        };
        let y = body.min.y
            + top_offset
            + row_idx as f32 * (cell_h + caption_band() + size::PREVIEW_GAP);
        Rect::from_min_size(Pos2::new(x, y), egui::vec2(cell_w, cell_h))
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

impl ProgramBay {
    /// Which cell `p` is on, as a deck in [`DECK_LETTERS`] order.
    ///
    /// `None` for the ground between two cells, for anywhere else in the bay,
    /// and for a console whose preview row is folded away -- a cell that is
    /// not drawn is not one a press can be on.
    pub fn cell(&self, p: karakuri_layout::Point) -> Option<u8> {
        let at = Pos2::new(p.x, p.y);
        self.cells?
            .iter()
            .position(|cell| cell.contains(at))
            .map(|deck| deck as u8)
    }

    /// **Which deck a carry let go at `p` lands on**, or `None` where no cell
    /// of a deck that has a slot is under it.
    ///
    /// [`Mixer::dropped`]'s answer one bay over, and the same gesture:
    /// `console.html`'s *How a Set reaches a deck* names **two** sets of
    /// rectangles a release can land on, *"the four deck preview cells take a
    /// drop as well, and each names the deck its letter names"*. The Library
    /// bay is in the left pane and the mixer in the right, so a carry between
    /// them crosses the whole window; the cells are in the centre column,
    /// beside the list the Set came out of.
    ///
    /// **A cell is an operand and never a choice.** Nothing routes a cell —
    /// `A` is deck A whatever is loaded, which is ADR-0240 — so there is no
    /// second thing a release here could mean and no reading to take before
    /// it means the first.
    ///
    /// # `slots` is how many the deck has, and it is why this takes an
    /// argument where [`Mixer::dropped`] takes none
    ///
    /// The mixer walks the strips it drew and there is one per slot, so *a
    /// deck with no slot* is already a strip that is not there. **The row is
    /// [`DECKS`] cells whatever the deck holds** — a cell that vanished would
    /// move the other three, and the letter is the only thing naming a deck
    /// ([ADR-0170](../../../../docs/adr/0170-a-deck-preview-cell-is-drawn-whether-or-not-a-deck-is-behind-it.md))
    /// — so the fourth cell of a three-slot deck is a rectangle whose letter
    /// names no deck, and a release on it has nothing to load into. It
    /// answers `None`, which is the same refusal `3` gets from the keyboard
    /// and the same reading behind it: [`View::select`] is *"a deck the mixer
    /// has no strip for is refused"*, off [`View::mixer`]'s length, and
    /// `karakuri/src/main.rs`'s `pointed` prints it.
    ///
    /// **That is not the refusal ADR-0265 forbids**, and the two are worth
    /// keeping apart. What is not read is the deck's *residency* and the
    /// cell's *material*: a drop on a live deck asks for the load, and a cell
    /// drawing nothing because no engine has handed it a texture is a target
    /// like any other ([`View::previews`], where `None` is two states). What
    /// is read is whether the letter names a deck at all — the operand, not
    /// the answer.
    ///
    /// The count is the caller's for [`Mixer::dropped`]'s reason, one step
    /// further out: this type is the bay's *geometry*, derived from a solved
    /// layout and nothing else, and a slot count is a reading the console is
    /// handed per frame.
    pub fn dropped(&self, p: karakuri_layout::Point, slots: usize) -> Option<u8> {
        self.cell(p).filter(|deck| usize::from(*deck) < slots)
    }

    /// **A press on a cell asks for nothing, and that is the decision rather
    /// than a gap.**
    ///
    /// This used to answer `Operation::SetPreview` — the cell's deck, or the
    /// mix where the press was on the cell the output was already showing.
    /// [ADR-0240](../../../../docs/adr/0240-the-output-shows-the-mix-and-residency-keys-belong-to-the-mixer.md)
    /// retired that operation: the Program Picture always presents the master
    /// mix and the four cells always audition their own decks, drawn from
    /// `karakuri_engine::deck::Deck::slot_view` every frame, so there is
    /// nothing left for a press to swap. **The cells are still the panel's**
    /// — [`ProgramBay::owns`] and [`ProgramBay::cell`] are unchanged and
    /// `tests/preview_cells.rs` still holds the boundary arithmetic — because
    /// what a control claims is what it is drawn over, and a cell an operator
    /// can drag the row's boundary off has to be claimed whether or not a
    /// press on the middle of it asks for anything.
    ///
    /// **A release on one asks for something, and that is not this sentence
    /// weakening.** [`ProgramBay::dropped`] is where it is, and a press and a
    /// release are two moments: nothing is in hand at the press, so there is
    /// still nothing for it to ask for — the carry is what puts the second
    /// operand there ([ADR-0273](../../../../docs/adr/0273-the-carry-lands-on-two-sets-of-rectangles-and-wears-a-face.md)).
    ///
    /// Whether `p` is on any of the cells -- the union of the four, for
    /// [`crate::input`]'s rule 4.
    pub fn owns(&self, p: karakuri_layout::Point) -> bool {
        self.cell(p).is_some()
    }
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
/// ([ADR-0174](../../../../docs/adr/0174-a-node-claims-only-what-its-visible-content-can-use.md)),
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
/// [ADR-0183](../../../../docs/adr/0183-a-node-is-out-of-the-layout-for-two-reasons-and-they-are-two-bits.md)
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
    let row_h = match layout.sizing(row) {
        karakuri_layout::Sizing::Fixed(h) => (h - size::PROGRAM_BODY_PAD).max(size::PREVIEW_ROW_H),
        _ => size::PREVIEW_ROW_H,
    };
    match (!layout.is_collapsed(picture), !layout.is_collapsed(row)) {
        // Both halves on screen, and this is the arrangement ADR-0182 decides.
        (true, true) => program_body_with_row_h(body, canvas, row_h).and_then(|arranged| {
            match arranged.placement {
                Placement::Beside => Some(ProgramBay {
                    placement: Placement::Beside,
                    picture: Some(arranged.picture),
                    cells: Some(arranged.cells),
                }),
                Placement::Below => {
                    let pic_rect = to_egui(layout.rect(picture));
                    let pic_area = Rect::from_min_max(
                        Pos2::new(
                            pic_rect.min.x + size::PROGRAM_BODY_PAD,
                            pic_rect.min.y + size::HEAD_H + size::PROGRAM_BODY_PAD,
                        ),
                        Pos2::new(pic_rect.max.x - size::PROGRAM_BODY_PAD, pic_rect.max.y),
                    );
                    let row_rect = to_egui(layout.rect(row));
                    let row_area = Rect::from_min_max(
                        Pos2::new(row_rect.min.x + size::PROGRAM_BODY_PAD, row_rect.min.y),
                        Pos2::new(
                            row_rect.max.x - size::PROGRAM_BODY_PAD,
                            row_rect.max.y - size::PROGRAM_BODY_PAD,
                        ),
                    );
                    drawable(
                        Placement::Below,
                        fitted(pic_area, canvas),
                        preview_row(row_area),
                    )
                    .map(|b| ProgramBay {
                        placement: Placement::Below,
                        picture: Some(b.picture),
                        cells: Some(b.cells),
                    })
                }
            }
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

/// **The `solo` pill in the Program bay's head, derived** -- the one control
/// this console has in a bay head, and the panel's route into *Solo a region*.
///
/// `docs/manual/console.html` draws it and says what it does in as many
/// words: *"Solo the program view: the panel folds away and only the picture
/// is left, which is also how you capture this window."* So the region it
/// names is `program-view` and not the bay around it, and that is read off the
/// page rather than chosen here.
///
/// # It is two operations and no toggle, which is [`Outputs::op`]'s rule
///
/// A solo has an undo and the vocabulary spells the two apart --
/// `karakuri_operation::Operation::Solo`'s `region` is `None` for *undo the
/// solo*, *"explicit rather than a toggle: the caller says which way"* -- so
/// [`Op::Solo`] and [`Op::Unsolo`] are what a press asks for and the choosing
/// between them is the affordance. [`ProgramHead::soloed`] is what it is
/// chosen from, and it is read back out of the layout rather than remembered.
///
/// **It undoes a solo it did not make.** `Layout::solo` collapses everything
/// off the soloed node's path, so the only solo this pill is still drawn under
/// is one on `program-view` itself or on something enclosing it -- every other
/// solo takes the Program bay off the screen and there is no pill to press.
/// Which is the same sentence `Op::Unsolo` already carries: what an unsolo
/// undoes is *the* solo, because there is only ever one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProgramHead {
    /// **The control**: the `solo` capsule, which is what a press has to land
    /// in. [`head_pills`]'s answer for it, so it is the rectangle
    /// [`bay_head`] painted.
    pub solo: Rect,
    /// The node it solos: the picture, `program-view`.
    pub id: NodeId,
    /// Whether anything is soloed -- `layout.is_soloed()`, read here.
    pub soloed: bool,
}

impl ProgramHead {
    /// **What a press on the pill asks for.** See the type's own
    /// documentation for why it is two operations rather than one that
    /// toggles.
    pub fn op(&self) -> Op {
        match self.soloed {
            true => Op::Unsolo,
            false => Op::Solo(self.id),
        }
    }

    /// Whether `p` is on the control.
    pub fn hit(&self, p: karakuri_layout::Point) -> bool {
        self.solo.contains(Pos2::new(p.x, p.y))
    }
}

/// **The Program bay's head, derived**: the one pill in it, and the node it
/// acts on.
///
/// `None` where there is no pill to press -- before the first frame, with the
/// bay folded or off a solo somewhere else, or in a bay too short to hold its
/// own head. That is [`outputs`]'s rule stated on a capsule instead of on a
/// chip: a rectangle with nothing in it is not something to paint or to click.
///
/// **The pill is found by name rather than by position.** [`REGIONS`] is where
/// a bay's controls are listed, and this reads [`SOLO_PILL`] out of the
/// Program bay's entry -- so a second pill added to that head moves this one
/// along and nothing here has to be told.
///
/// **A second pill was duly added and this capsule duly moved.** The class pill
/// ([`mcp_pill`]) sits to `solo`'s right and is not the same width in its two
/// states, which is why this now takes an opening: [`head_capsule`] lays the
/// whole head out under that opening and hands back the one capsule asked for,
/// so the two pills cannot be laid out against two different states.
///
/// `layout` must be solved: [`karakuri_layout::Layout::rect`] refuses to
/// answer from a dirty one. `ctx` is asked for the type, because a `.pill` is
/// as wide as the word in it.
///
/// # What it costs the operator, and it is 0.75 of a pixel
///
/// **This is one of the first two controls on the console that do not clear
/// every boundary's grab** — the four deck preview cells under it are the
/// other, and [`ProgramBay::preview`] carries theirs. The number is worth
/// having in front of you rather than in a test alone. A bay head is [`size::HEAD_H`] = 27 and a `.pill` is
/// [`size::PILL_H`] = 16.5, centred, so there is (27 - 16.5) / 2 = **5.25** of
/// head above the capsule -- against a [`crate::panel::GRAB`] of **6**. The
/// Program bay is the first child of the centre column, so its top edge is the
/// body row's, and the boundary between the transport row and the body grabs
/// six pixels past it.
///
/// So the top **0.75** of the capsule is the boundary's and the other 15.75 is
/// the panel's. [`crate::input`]'s rule 3 is what decides that and it decides
/// it the same way every time: the boundary gets first refusal, there is no
/// case where both think they are dragging, and the hazard that rule was
/// written for does not arise. What is lost is the sliver, and
/// `tests/solo_pill.rs` is what states the number and fails if it grows.
///
/// **The three things that could change it are all somebody else's**: the
/// head's height and the pill's box are `docs/manual/console.html`'s, and
/// `GRAB` is the rule's. This function draws the control where the page puts
/// it.
pub fn program_head(
    ctx: &egui::Context,
    layout: &karakuri_layout::Layout,
    open: Open,
) -> Option<ProgramHead> {
    // Fonts are not valid until `egui` has run a pass, exactly as in
    // [`outputs`]: a press before the first frame is a press on a control that
    // has never been drawn.
    if ctx.cumulative_pass_nr() == 0 {
        return None;
    }
    let bay = layout.find("program")?;
    // The bay itself, its column folded around it, or a solo somewhere else:
    // one question for every ancestor, which is [`program_bay`]'s own guard.
    if !layout.visible(bay) {
        return None;
    }
    let id = layout.find("program-view")?;
    let head = head_of(region("program")?)?;
    // A head clipped to a bay shorter than 27 has nowhere to put a 16.5
    // capsule, and a capsule half out of the head is not one to press --
    // [`head_capsule`]'s own guard, which is why it is not repeated here.
    let solo = head_capsule(ctx, to_egui(layout.rect(bay)), &head, open, SOLO_PILL)?;
    Some(ProgramHead {
        solo,
        id,
        soloed: layout.is_soloed(),
    })
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
///   allocation — and no repaint is asked for, which is ADR-0164's
///   still-panel clause.
/// - **The frame it does change solves to the new arrangement and not to the
///   previous one**, because the second solve is after the write. That frame
///   costs **two solves**, and it is worth saying plainly rather than hiding:
///   a placement only changes when the bay's rectangle does, which is a window
///   resize or a drag on a boundary, and ADR-0210 does not budget what the
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
/// ([P-0092](../../../../docs/principles/0092-the-same-inputs-produce-the-same-frame.md)).
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
/// is the one the program leaves `None`.
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
    /// **What the last write did** — the mock's `landed` capsule at the end of
    /// this row, and `None` until a write has done anything.
    ///
    /// # It is [`Stage`] and not a fourth spelling of the same three words
    ///
    /// `docs/manual/console.html` gives the pill and the Staging lane's rows
    /// one sentence apiece and they are the same answers — *"landed,
    /// overloaded, failed to build, or did not compile"* under *Health, in the
    /// transport*, and *"whether it is on screen: landed, overloaded for
    /// costing more than one frame may, refused, or did not compile"* on a
    /// candidate row. A second enum here
    /// would be `docs/contributing.md` §4's *a name meaning two things* built
    /// on purpose, so [`Stage`] has two readers and one set of words.
    ///
    /// **The two readings are not the same reading, which is why both are
    /// drawn.** A candidate row is one deck slot with a verdict *outstanding*
    /// and leaves the lane the moment the watchdog says the version held the
    /// budget — *"empty is this lane's ordinary state"*. This is the last
    /// verdict there was, on whichever slot, and it stands after the lane has
    /// emptied. So the lane answers *what is unsettled* and this answers *what
    /// did the last write do*, which is the row the operations page gives this
    /// pill and gives the lane's controls a different one.
    ///
    /// # `None` is *nothing has been written*, and it draws no capsule
    ///
    /// Not a word for it, not a dash, and not an `armed` pill reading
    /// `landed` about a build nobody made — the rule the two fields above
    /// follow, and the Staging lane's own *"no row, no placeholder, and no
    /// standing sentence"*. A run in which nobody rewrites a procedure never
    /// draws this, which is most runs.
    ///
    /// **Two `swap::Event`s leave it alone rather than clearing it.** The
    /// watchdog's verdict in favour settles a candidate and does not take the
    /// last write off the screen, and a lost build worker says nothing about a
    /// write that already happened — `crates/karakuri/src/main.rs`'s
    /// `staging`, which is where the one drain feeds both readings.
    pub health: Option<Stage>,
    /// **Whether a session is being recorded** — the mock's `● rec` pill at
    /// the very end of this row, after [`Transport::health`] — and `None` for
    /// a console nobody has told anything about recording, which draws no
    /// pill at all.
    ///
    /// # `None` is *nobody said*, and it is not *not recording*
    ///
    /// [`View::audio`]'s distinction one row along, and for its reason: a
    /// recording is a file being written by a program that has a store, and
    /// `src/` has neither (ADR-0156). `Some(Rec::Idle)` is a program that
    /// holds a store and is not recording into it, and it draws the pill in
    /// the plain treatment because a press on it would start one; `None` is a
    /// console that was never told, and a pill drawn for it would be this
    /// crate answering a question about a disk on its own authority. Every
    /// test in this crate that does not say otherwise leaves it `None`.
    ///
    /// # It is on [`Transport`] rather than beside it, which [`View::look`] is
    /// not
    ///
    /// [`Transport::health`]'s reason, and this is the second field here that
    /// is not a clock: what this type is is *what the transport row reads this
    /// frame*, and the two capsules at the end of the row are read by it. The
    /// difference from the look — which is two fields away on [`View`] because
    /// the row draws it and the master chain owns it — is that neither of
    /// these two capsules is drawn anywhere else and neither belongs to
    /// another bay.
    pub rec: Option<Rec>,
}

/// **What the `● rec` pill says this frame**, and the whole of its state.
///
/// Two values because the control is a toggle and a toggle has two ends: a
/// press on it starts a recording or stops the one running, and which of those
/// a press means is exactly this. There is no third value for *starting* —
/// opening a recorder writes a Set file and creates another, so it happens off
/// the frame path (`crates/karakuri/src/main.rs`), and until the recorder is
/// open nothing is being recorded and the pill says so.
///
/// **A two-valued enum rather than a `bool`**, which is
/// [P-0087](../../../../docs/principles/0087-name-the-property-never-the-shape.md):
/// `Some(true)` at a call site says nothing, and this is read at four of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rec {
    /// Nothing is being recorded. The mock's plain `.pill`, and a press starts
    /// one.
    Idle,
    /// A session is being written to the store as it happens. The mock's
    /// `.pill.on`, and a press stops it.
    Running,
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

    /// **Where the beat is inside the current bar**, from zero, and
    /// continuous: `0.0` is on the first dot, `1.5` is halfway between the
    /// second and the third, and `3.75` is three quarters of the way from the
    /// last dot back round to the first.
    ///
    /// **A position and not an index**, which is the whole of what the grid
    /// draws now
    /// ([ADR-0212](../../../../docs/adr/0212-the-beat-is-a-light-that-travels-and-it-declares-for-itself.md)):
    /// the light is somewhere on the grid at every instant rather than on one
    /// dot at a time, so what the row reads is the fractional beat the
    /// oscillator already accumulates rather than a truncation of it. The
    /// fraction was always in [`Transport::beats`] and was thrown away here.
    ///
    /// **It is the whole of the beat's motion, and it is not a clock.** No
    /// second value is written for the grid and none is derived from
    /// [`Phase`]: the beat moves because the session moves, which is what a
    /// beat grid is for, and a wall-clock sweep would go on sweeping at its
    /// own rate over a session running at another one — see [`beat_at`].
    ///
    /// `rem_euclid` rather than `%` because [`Transport::beats`] can be
    /// negative — `Oscillator::behind` reads the same grid at an earlier time,
    /// and a slot warming behind the session is exactly what that is for — and
    /// `%` on a negative is negative, which is a position no grid has.
    ///
    /// **The clamp this used to need went with the index.** `(-1e-18_f64)
    /// .rem_euclid(4.0)` is `4.0` exactly: the true remainder is a hair under
    /// the divisor and rounds up to it. As a dot *index* that was one past the
    /// end of the grid — a rectangle drawn beside it, or a panic — and it was
    /// clamped. As a *position* it is the downbeat: `4.0` and `0.0` are the
    /// same point on a cycle of four, [`beat_at`] measures round the cycle,
    /// and the value arrives where it belongs with nothing written for it.
    pub fn position(&self) -> f32 {
        self.beats.rem_euclid(self.dots() as f64) as f32
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

/// **The transport row, laid out**: where each of the five readouts goes, and
/// which beat is lit.
///
/// # One derivation, for the reason [`Outputs`] is one
///
/// [`View::draw`] paints exactly these rectangles. Nothing hit-tests *these*
/// four, because none of them is a control — see below — but this now has
/// three call sites rather than the one it was written with: the frame paints
/// it, and [`arrangement`] asks it where the bar ended and where the frame
/// readout begins, from both the frame and [`crate::input::claim`]. A value
/// rather than a paint-as-you-go pass is what made that possible, and it was
/// already the right shape for the reason it was written: it makes the
/// arithmetic something `tests/transport.rs` can ask about without a device,
/// which is the whole of how this crate is checked.
///
/// # Four of the six are readouts, and two are controls
///
/// A point in this row that is not inside a boundary's [`GRAB`] and not on one
/// of this row's controls is `egui`'s. A beat, a bar, a frame time and what
/// the last write did are four readouts, and a readout is not something a
/// press acts on. **The last of them is drawn as a capsule and is still one**:
/// the shape is the mock's, and what makes a thing a control here is that a
/// press on it asks for something. `tests/transport.rs` asserts it over the
/// row rather than leaving it to be inferred from the absence of a hit test,
/// and it asks with the pill drawn so that the assertion cannot pass on the
/// control having gone.
///
/// **The tempo is not one of the four**, and it left them when the figure
/// became the track: the number is still a reading, and a press on it names a
/// value outright — [`TransportRow::tempo`], and ADR-0291. A readout that a
/// press acts on is a control, which is the same test the capsule fails.
///
/// **The other control is [`TransportRow::rec`], and it is drawn *inside* this
/// derivation where [`arrangement`] is drawn beside it.** The difference is
/// where each one sits: the arrangement pill is laid out from
/// [`TransportRow::bar`] and held clear of [`TransportRow::frame`], so a row
/// that never heard of it is laid out exactly as it is now; the `rec` pill
/// takes the row's right padding, and the health capsule and the frame readout
/// are laid out backwards from it. A pill drawn beside the row could not move
/// them, and two derivations of one right-hand end are two answers.
///
/// **This paragraph said *nothing here is a control*, then *none of these five
/// is*, then *the sixth is this row's one control*, and every one of those
/// changes is deliberate.** What a press on the capsule asks for is
/// [`TransportRow::record`], and what a press on the number asks for is
/// [`TransportRow::tempo`].
///
/// # What is in the mock's row and is deliberately not here
///
/// **The list is empty as of 2026-09-10, and that is a state rather than a
/// deletion.** It held `learn` and `map · nanoKONTROL2 ▾` — *"each a control
/// over machinery that is in neither this crate nor the program"* — and both
/// are drawn now:
///
/// - `learn` is [`learn_pill`], and the machinery arrived with
///   `docs/adr/0336-a-learn-is-a-map-edit-and-the-tips-midi-line-is-the-live-map.md`:
///   the panel opens a port at start-up, and a press on this arms the gesture
///   the mock's tooltip describes.
/// - `map · <name>` is [`map_pill`], **and it is a readout**. What was written
///   against it was that *"a menu naming a device nobody has plugged in is
///   worse than no menu"*, and that half is unchanged — reaching a map while
///   running is not built, so the pill has no chevron and no menu and says
///   which file is loaded. A capsule that looked like a menu and opened none
///   would still be the scaffolding this module refuses; one that states a
///   true thing is not.
///
/// **The rule that emptied it is the one that kept them out**: a control is
/// drawn when there is something behind it. [`arrangement`] was the first item
/// in this row to pass that test and [`audio_in`] the second; these are the
/// third and fourth.
///
/// **`● rec` left this list on 2026-09-08**, and what was written against it
/// was that *"there is no session recorder behind this panel and no record
/// stream is written from it"*. There is one now: `crates/karakuri` opens a
/// `karakuri_environment::session::Recorder` on a press and closes it on the
/// next, and this pill is the toggle — [`TransportRow::record`], drawn in the
/// mock's `.pill.on` while a recording runs. The mock's tip named one gesture
/// on it, *click to stop*, and the control is both: what a press means is what
/// [`Transport::rec`] says it will be, before it is made.
///
/// **This paragraph carried a total of the row and it no longer does**, which
/// is a deletion rather than an oversight. It said *"the mock draws ten things
/// and five of them exist"*, and the ten was a number nobody could check
/// against the markup: `.transport` has fourteen children and one of them —
/// `.tracker` — holds four more. A list of what is missing is checkable one
/// item at a time; a total of what a row holds is a second count of the mock,
/// and it had already gone stale once, when `audio-in` left this list.
///
/// **`landed` left it on 2026-09-08 and it is the one item here that never
/// was a control.** What was written against it was that *"nothing writes a
/// procedure while this panel runs, so the pill would be reporting on a write
/// that never happens"*, and that was already false when it was read again:
/// `crates/karakuri` watches every slot's sources, so a save from any editor
/// builds, swaps and is judged, and every verdict of that is drawn in the
/// Staging lane. This row is where the same stream says *what the last write
/// did* — [`Transport::health`], which is a value the harness hands in and not
/// a control this crate offers.
///
/// **Three items left it on 2026-09-08** — `tap`, `offset` and the octave's
/// `½ ×2` — which is M5.4's own work: [`tracker_group`] draws them, and what
/// was written here about the first of them was that *"nothing times a tap
/// here, and the pill would correct nothing"*. Something does now, and it
/// always did on the keyboard: `karakuri_environment::audio::Audio::tap` is
/// what `b` has been reaching, and the pill reaches it by emitting the same
/// operation. `audio-in` was the first to leave, on the same terms.
///
/// The `.sep` between the bar and the frame readout **is** drawn, in the only
/// way a `flex: 1` spacer can be: it is space, so what it does is push the
/// frame readout to the right edge, and that is where [`transport_row`] puts
/// it.
///
/// # No tooltips, for the reason the Outputs row has none
///
/// Most of the mock's items carry a `data-tip` and so do most of the drawn
/// ones — the beat grid's is what the travelling light means, and it landed
/// with the light
/// ([ADR-0212](../../../../docs/adr/0212-the-beat-is-a-light-that-travels-and-it-declares-for-itself.md)). A tooltip needs `egui` to own a widget, this console paints, and
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
    // **The health capsule, measured only where there is a verdict to draw.**
    // `.pill`'s `padding: 0 8px` around one word — there is no chevron here
    // and no second span, because nothing about this capsule opens and it
    // says one thing.
    let health = values
        .health
        .map(|stage| size::PILL_PAD_X * 2.0 + width(span(stage.word(), Color32::PLACEHOLDER)));
    // **The `rec` pill, measured only where somebody has said whether a
    // recording is running**, and the same width either way it is: the mark
    // and the word do not change between the two states — only the treatment
    // does — so this is one number rather than the wider of two, and the pill
    // does not move under a press that lands on it.
    let rec = values.rec.map(|_| {
        size::PILL_PAD_X * 2.0
            + size::SINK_DOT
            + size::SINK_GAP
            + width(span(REC_LABEL, Color32::PLACEHOLDER))
    });
    transport_row(row, &values, bpm, label, bar, frame, health, rec)
}

/// **The word in the `rec` pill**, and the mock's `&#9679; rec` without its
/// mark: the mark is drawn rather than typed, which is [`Mask`]'s rule for the
/// same reason — *"whether `◯` and `◑` are in `egui`'s default face is a
/// question with no good answer, and a circle is the same mark either way"*.
const REC_LABEL: &str = "rec";

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
    /// **Where the light is**, in beats from the first dot's centre —
    /// [`Transport::position`], carried for the reason [`TransportRow::dots`]
    /// is carried: the painter and a test read the one number the row was
    /// measured from. How much of it lands on any given dot is
    /// [`TransportRow::lit`].
    pub at: f32,
    /// `bar 37`.
    pub bar: Rect,
    /// The frame readout, pushed to the right by the `.sep` — against the
    /// row's right padding where there is no verdict to draw, and one
    /// [`size::TRANSPORT_GAP`] before [`TransportRow::health`] where there is.
    pub frame: Rect,
    /// **The health capsule**, and `None` where [`Transport::health`] is —
    /// which is every run until somebody rewrites a procedure.
    ///
    /// **The `rec` pill is what follows it**, so the right padding is
    /// [`TransportRow::rec`]'s where there is one and this capsule's where
    /// there is not — and everything before it is laid out backwards from
    /// whichever ends the row. No gap is left for a pill that is not drawn.
    pub health: Option<Rect>,
    /// **The `● rec` pill**, and `None` where [`Transport::rec`] is — which is
    /// a console nobody has told anything about recording.
    ///
    /// **It is the last thing in the row and takes the right padding**, which
    /// is where the mock puts it: after `landed`, hard against the end of
    /// `.transport`. It is the one thing in this row that is a control — a
    /// press on it starts a recording or stops the one running — and it is
    /// carried here rather than derived beside the row for
    /// [`TransportRow::health`]'s reason one item along: where each readout
    /// ends is where the next one starts, so the pill's place and the
    /// readouts' places are one derivation and not two that agree until they
    /// do not.
    ///
    /// **It said it was the one control in this row and it is one of two**:
    /// the tempo figure is the other ([`TransportRow::tempo`]), landed in the
    /// same commit and drawn at the row's other end.
    pub rec: Option<Rect>,
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

/// **How far a press may move the tempo**: **±15%** of what the grid is
/// running at when it lands.
///
/// # It is a guard against a mis-click, and not a bound on what a tempo may be
///
/// Nothing in this workspace says a tempo may not be 240 —
/// `karakuri_audio::tempo::BPM_RANGE` says outright that it is *not* the range
/// of answers, and *"a grid at 240 bpm is a perfectly good grid"* — and this
/// band does not say it either: it bounds **one press**, against the tempo of
/// the moment, so a hand walks the grid anywhere it likes — **240 is five
/// presses up from the tempo the mock draws**, and the figure names it
/// outright on the last of them.
///
/// **The `½ ×2` beside the figure is not the shortcut it looks like**, and at
/// this row's own tempo it is not available at all: [`Tracker::double`] is
/// `karakuri_audio`'s `BPM_RANGE` against twice the grid, 256 is outside it,
/// and the mock draws that half inert. So the figure is the whole of the way
/// up from 128, and `docs/manual/console.html`'s *"get there in two presses"*
/// is not arithmetic that works from there — reported rather than quietly
/// satisfied by a wider band than the one that was decided.
///
/// # A press outside it is ignored, and *ignored* is the decision
///
/// Not clamped, which is the one thing a track this crate has drawn before
/// does do — [`unit_of_offset`] draws a value past either end *at* that end,
/// because the offset's ends are the range of the value. These are not ends of
/// anything: they are how far a hand is trusted to have meant it. Clamping
/// would turn a press the operator did not mean into a 15% move of the grid,
/// which is the loudest thing this row can do; ignoring it leaves the tempo
/// where it was, and a tempo that did not move is a press the operator can see
/// did not land ([ADR-0291](../../../../docs/adr/0291-the-tempo-figure-is-the-track-and-the-band-is-a-guard-on-the-hand.md)).
///
/// # It is here and nowhere else
///
/// **It is not in `karakuri-audio`, and must not be.** The tracker folds every
/// candidate into a window centred on the grid and the beat lock re-acquires
/// on evidence; a ±15% bound anywhere in that path is a grid that cannot
/// follow a song, which is a far worse failure than a hand that can ask for
/// anything. So the band lives at the one point a *press* becomes an
/// operation, which is this type — the same seam
/// [`TrackerGroup::half`] draws an inert octave chip at, one group along.
pub const TEMPO_BAND: f32 = 0.15;

/// **What the whole figure spans**, as the same fraction of the same tempo:
/// **two bands either way**, so [`TEMPO_BAND`] is the middle half of the
/// number and the outer half is the guard.
///
/// **The figure has to reach past the band or the band could not be missed**,
/// and a guard nothing can land in is not a guard — it is an assertion that
/// the surface already satisfies, and `tests/tempo_figure.rs` could only pass
/// it by construction. So the question is not whether the number reaches further
/// than a press may go, but by how much, and the answer is *one more band*:
/// a press that misses by up to as much again as it is allowed to move is
/// refused, and a press that misses by more than that is off the figure and
/// was never this control's.
///
/// **Linear, and that is the offset track's arithmetic rather than the
/// exposure's.** [`offset_at`] is linear because a latency offset is a
/// *difference*, and [`exposure_at`] is logarithmic because an exposure is a
/// *ratio* — a tempo is the second kind of quantity, which is why the control
/// beside this one is an octave and why this band is a percentage. But the
/// band is stated as ±15% **of the tempo at the press**, and at the instant of
/// a press that is a fixed number of beats a minute: the whole geometry is
/// settled by the one number the row was measured with, and equal distances
/// along the figure are equal numbers of beats a minute. Over a span this
/// narrow the two curves are within a percent of each other in any case, and
/// the linear one is the one whose middle is exactly the tempo drawn there.
pub const TEMPO_SPAN: f32 = TEMPO_BAND * 2.0;

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
            Pos2::new(self.grid.min.x + BEAT_PITCH * index as f32, self.grid.min.y),
            egui::vec2(size::BEAT_W, size::BEAT_H),
        )
    }

    /// **How much of the light is on the dot at `index` this frame**, from
    /// `0.0` to `1.0` — [`beat_at`] at this row's position, so the painter and
    /// a test ask one question and get one answer.
    ///
    /// Panics on a dot this grid has not got, for [`TransportRow::dot`]'s
    /// reason.
    pub fn lit(&self, index: u32) -> f32 {
        assert!(index < self.dots, "dot {index} of a grid of {}", self.dots);
        beat_at(self.at, index, self.dots)
    }

    /// **How far along the figure `p` is**, on `[0, 1]` across
    /// [`TransportRow::bpm`] and outside it either side.
    ///
    /// Not clamped, unlike [`unit_of_offset`]'s: what is off the number is not
    /// at its end, it is not this control's, and every caller here has asked
    /// [`Rect::contains`] first.
    fn along(&self, p: karakuri_layout::Point) -> f32 {
        (p.x - self.bpm.min.x) / self.bpm.width()
    }

    /// **What a press `unit` of the way along the figure names**, in beats a
    /// minute — [`offset_at`]'s job on the one control whose track is the
    /// reading itself.
    ///
    /// **The middle of the number is the number**: at `0.5` this is
    /// [`Transport::bpm`] exactly, which is what `docs/manual/console.html`
    /// means by *"the number under your finger is the one you get"* — the
    /// figure is drawn at the tempo it names, so pressing where it is drawn
    /// asks for what is already there. The ends are [`TEMPO_SPAN`] either way,
    /// and the half of the figure between the quarters is [`TEMPO_BAND`].
    ///
    /// **It is a function of the tempo the row was measured at**, so the whole
    /// control moves with the grid: at 128 a press at the right-hand end asks
    /// for 166.4, and at 256 the same pixel asks for 332.8. There is no track
    /// with ends in it anywhere here, which is why there is no constant naming
    /// them — the offset's [`LATENCY_OFFSET_MIN_MS`] is a restatement of a
    /// range something else holds, and nothing holds one for this.
    pub fn tempo_at(&self, unit: f32) -> f32 {
        self.values.bpm * (1.0 + (unit - 0.5) * 2.0 * TEMPO_SPAN)
    }

    /// **Whether a tempo is one a press may ask for**: within [`TEMPO_BAND`]
    /// of what the grid is running at, measured against the tempo this row was
    /// derived with.
    ///
    /// **Which is the tempo at the press**, and not one from an earlier frame:
    /// `karakuri/src/main.rs` derives this row again on the pointer event, off
    /// the same [`View::transport`] the frame before it was painted from — the
    /// honest cost [`tracker_group`] states for the group beside this one, paid
    /// here for the same reason.
    pub fn in_band(&self, bpm: f32) -> bool {
        // **A number that is not a tempo is not one a press may ask for**, and
        // the case it guards is a band of nothing: a row drawn at `0.0` has a
        // band `0.0` wide, and every point on the figure would name exactly
        // zero and pass a comparison written with `<=`. Nothing in this
        // workspace runs a grid at zero — `karakuri_signal`'s own clamp floors
        // it at 1.0 — but this row is drawn from whatever the harness hands
        // in, and a control that emits an operation naming a tempo no
        // oscillator will take is a press that does nothing (P-0094).
        bpm.is_finite()
            && bpm > 0.0
            && (bpm - self.values.bpm).abs() <= self.values.bpm.abs() * TEMPO_BAND
    }

    /// **Whether `p` is on the tempo figure asking for something it may
    /// have** — on the number, and inside [`TransportRow::in_band`].
    ///
    /// **The band is part of the hit test and not only of the answer**, which
    /// is [`TrackerGroup::half`]'s rule word for word: *inert is not claimed*,
    /// and this crate's own *a control claims what it acts on and no more*. So
    /// a press on the guard is `egui`'s — it falls through exactly as a press
    /// on the refused half of the octave does, rather than being swallowed by
    /// a control that then does nothing, which is the one outcome an operator
    /// cannot tell from a panel that has stopped
    /// ([P-0094](../../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)).
    ///
    /// **The number's own box is the target and nothing is grown**, unlike
    /// [`OffsetTrack::grip`]: `.bpm` is 20px type at `line-height: 1.5`, so
    /// this is 30 tall against that track's 5, and it leaves 9 of the row's 48
    /// above and below — clear of the [`GRAB`] of the boundary under the row,
    /// which is 6.
    pub fn on_tempo(&self, p: karakuri_layout::Point) -> bool {
        self.bpm.contains(Pos2::new(p.x, p.y)) && self.in_band(self.tempo_at(self.along(p)))
    }

    /// **What a press at `p` asks the grid to run at**, or `None` off the
    /// figure and on the guard around it.
    ///
    /// # The press names the tempo outright, and that is the decision
    ///
    /// [`Operation::SetFreeRunTempo`] carries the number, and the number is
    /// where along the figure the press landed. `docs/manual/console.html`:
    /// *"A press names a value outright rather than stepping, so the figure is
    /// the track and the number under your finger is the one you get."* It is
    /// [`OffsetTrack`]'s sentence one control to the left
    /// ([ADR-0277](../../../../docs/adr/0277-the-latency-offset-is-a-track-because-a-capsule-cannot-name-a-value.md)),
    /// and it arrives here without that record's derivation: **this row has no
    /// key**, so there is no press to make a pixel out of, and what decides the
    /// figure's scale is the figure's own width and [`TEMPO_BAND`].
    ///
    /// # It is emitted with a room being tracked, and that is not this crate's
    /// call
    ///
    /// The operation is *what the grid runs at with nothing driving it*, and
    /// the state it is for is a program with no audio device — which is the
    /// state `crates/karakuri` runs in, where `tap` and the octave both refuse
    /// out loud because there is no room. With a room open it is **accepted**
    /// rather than refused: the tracker searches a window centred on the grid,
    /// so moving the grid moves the window and the estimate is made again
    /// around the new target — `karakuri_audio`'s `BeatLock::retarget`, which
    /// is where that is written down and is the reason this is not a control
    /// that undoes itself two seconds later. Nothing here can see whether a
    /// room is being tracked in any case: this crate takes no device
    /// (ADR-0156), and [`Tracker`] carries what the panel *draws* rather than
    /// what a press is allowed to be.
    pub fn tempo(&self, p: karakuri_layout::Point) -> Option<Operation> {
        self.on_tempo(p).then(|| Operation::SetFreeRunTempo {
            bpm: self.tempo_at(self.along(p)),
        })
    }

    /// **Whether `p` is on the `rec` pill**, which is one of the two controls
    /// this derivation owns: the tempo figure is the other
    /// ([`TransportRow::on_tempo`]), and the four things beside them — the
    /// beat grid, the bar, the frame readout and the health capsule — are
    /// readouts, which are not something a press acts on.
    ///
    /// `false` where the pill is not drawn — a console nobody has told
    /// anything about recording claims nothing.
    pub fn on_rec(&self, p: karakuri_layout::Point) -> bool {
        self.rec
            .is_some_and(|pill| pill.contains(Pos2::new(p.x, p.y)))
    }

    /// **What a press at `p` on the `rec` pill asks for**, or `None` off it.
    ///
    /// **It is a toggle and the two ends are two payloads**, which is what
    /// [`karakuri_operation::Recording`] is: a press with nothing running asks
    /// to begin one, a press with something running asks to end it, and the
    /// pill's own treatment is what says which the next press will be before
    /// it is made. So one capsule is one control and not two, and the state it
    /// reads is the state the press acts on — one reading, so the pill that is
    /// drawn and the press that is performed cannot come apart.
    ///
    /// **`id` is `None`, and the payload saying so is what makes each start a
    /// fresh recording.** A capsule types no name — this console's one
    /// letter-taking flow is bounded to naming an arrangement — so whoever
    /// performs it files under a stamp, exactly as the `keep` capsule's save
    /// does. That is not only a convention here: a second head under one id
    /// would land in the middle of an existing stream and be read back as
    /// edits, so a start that could name the id a stop just closed would be a
    /// press that corrupts a session (ADR-0289).
    ///
    /// **It refuses nothing.** Whether there is a deck to write a head from
    /// and whether the store will take the file are the instrument's answers
    /// rather than this surface's — this crate reaches no disk at all
    /// (ADR-0156) — so the press names the operation and whoever performs it
    /// says what happened.
    pub fn record(&self, p: karakuri_layout::Point) -> Option<Operation> {
        let rec = self.values.rec?;
        self.on_rec(p).then_some(Operation::RecordSession {
            recording: match rec {
                Rec::Idle => Recording::Start { id: None },
                Rec::Running => Recording::Stop,
            },
        })
    }
}

/// **From one dot to the next**: [`size::BEAT_W`] and the [`size::BEAT_GAP`]
/// after it, which is what one beat of travel measures on this grid.
///
/// Derived rather than transcribed: the mock declares an item width and a flex
/// `gap`, and the pitch of a flex row is the one plus the other. It is the
/// unit [`beat_at`] measures in and the unit [`TransportRow::dot`] steps by,
/// which is why it is a constant rather than the same sum written twice.
pub const BEAT_PITCH: f32 = size::BEAT_W + size::BEAT_GAP;

/// **The tempo this row is drawn at in the mock** — `.bpm`'s `128.0` — in
/// thousandths of a beat a minute, so [`BEAT_STALENESS`]'s arithmetic stays in
/// integers.
const MOCK_BPM_MILLI: u64 = 128_000;

/// **One beat at that tempo**, in microseconds: 468 750, which is 468.75 ms.
const BEAT_MICROS: u64 = 60 * 1_000_000 * 1_000 / MOCK_BPM_MILLI;

/// **How many steps one beat of travel is drawn in**: the pixels in one
/// [`BEAT_PITCH`], so the light moves by at most one of them between updates.
///
/// [`mixer::ROLL_STEPS`]'s shape and a different answer, because the two motions are
/// different sizes: the roll travels a few pixels of a word's pitch and twelve
/// steps is smooth over it, where the light crosses a whole dot and a gap
/// every beat.
const BEAT_STEPS: u64 = BEAT_PITCH as u64;

/// **How stale the beat grid may get**, which is what the transport row
/// declares under
/// [P-0091](../../../../docs/principles/0091-cost-is-known-before-it-is-paid.md)
/// and what the harness turns into a deadline.
///
/// [`BEAT_MICROS`] in [`BEAT_STEPS`] steps — **24.67 ms, about forty a
/// second**. One beat of travel is one [`BEAT_PITCH`], so this is the light
/// moving by one pixel and no more, which is the coarsest step that reads as a
/// movement rather than as a sequence of positions
/// ([P-0094](../../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)).
///
/// **Stated at the mock's tempo, and it is the one number here that the music
/// moves.** A beat is 468.75 ms at 128.0 BPM and 375 ms at 160, so the same
/// declaration is a pixel and a quarter a step up there. The alternative —
/// derive it per frame from [`Transport::bpm`], which the row is handed — is
/// [ADR-0212](../../../../docs/adr/0212-the-beat-is-a-light-that-travels-and-it-declares-for-itself.md),
/// and it lost on what it does to the arithmetic rather than on the drawing: a
/// staleness that falls with the tempo makes `Σ (cost / staleness)` a function
/// of how fast the music is, so the two schedulability conditions could only
/// be asserted against a fastest tempo nobody has written down — and inventing
/// one inside the test that noticed it was missing is exactly what
/// [ADR-0210](../../../../docs/adr/0210-a-declared-cost-is-one-panel-pass-written-down-and-held-against-the-run.md)
/// refused for the panel's share of the budget.
///
/// **Finer than [`ROLL_STALENESS`]'s 33.33 ms and coarser than a frame**, and
/// both are the presentation rather than a preference for frames: the roll
/// travels a few pixels and this crosses the grid, so it wants more steps; and
/// a 60 Hz frame is 16.6 ms, so a beat grid that asked for every frame would
/// be asking for more than its own drawing can use.
pub const BEAT_STALENESS: Duration = Duration::from_micros(BEAT_MICROS / BEAT_STEPS);

/// **How much of the light is on the dot at `index`**, with the light `at`
/// beats into a bar of `dots`: `1.0` under its centre, `0.0` a whole
/// [`BEAT_PITCH`] away, and a raised cosine between the two.
///
/// # The same curve as [`roll_at`], and two of the reasons are the same
///
/// It leaves and arrives at zero **with zero velocity**, so a dot does not
/// snap into being dark as the light leaves it. And it is a pure function of a
/// value the harness handed in, so a test asserts it at a position it chose
/// and nothing samples a clock to find out what the panel is doing.
///
/// # Two dots at once, and the row's total light is constant
///
/// The falloff is exactly one pitch wide, so at most two dots are lit and
/// `f(d) + f(1 - d) = 1` for every `d` — the raised cosine's own identity, and
/// therefore the grid's dots always sum to exactly one dot's worth of light
/// (every grid but the degenerate one below, which is one dot and holds all of
/// it). The
/// light **moves along the grid** rather than the grid brightening and dimming
/// as it goes, which is what makes a stop visible: a still grid at half
/// brightness would be indistinguishable from a light sat between two dots.
///
/// # Measured round the cycle, not along the row
///
/// The bar wraps, so the distance from the last dot to the first is one pitch
/// and not three: the light leaves the right-hand end of the grid and arrives
/// at the left-hand end in the same instant, each dot half lit, and there is
/// no frame on which it jumps. That is also why [`Transport::position`] needs
/// no clamp — a position of exactly `dots` is a distance of zero from the
/// first dot.
///
/// # What it draws at the instant of a beat is the mock
///
/// At a whole `at` the dot under the light is `1.0` and every other is exactly
/// `0.0`: one dot in `--c-pink` with its halo and the rest in `--c-line`,
/// which is `.beat-grid i.on` and the four dots the mock's markup draws. **The
/// mock is a frame of this** rather than a picture this contradicts, and
/// `docs/manual/style.css` carries the travel between those frames.
pub fn beat_at(at: f32, index: u32, dots: u32) -> f32 {
    let dots = dots.max(1) as f32;
    // **A grid of one dot has nowhere for the light to go**, so it is on that
    // dot at every position. Measuring round a cycle one pitch long would say
    // *half lit* halfway through the beat and leave the row dimming with
    // nothing to dim toward — the identity above wants two dots to share the
    // light between. This is [`Transport::dots`]' own clamp seen from here: a
    // bar of no beats is drawn as the nearest thing to a grid there is.
    if dots < 2.0 {
        return 1.0;
    }
    let round = (index as f32 - at).rem_euclid(dots);
    let away = round.min(dots - round);
    match away < 1.0 {
        true => 0.5 * (1.0 + (away * std::f32::consts::PI).cos()),
        false => 0.0,
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
// Eight, where clippy's line is seven. Six of them are one measured width
// apiece — the tempo figure, its label, the bar, the frame readout, the health
// capsule and the `rec` pill — and measuring is the one thing this function
// exists not to do: [`transport`] holds the `egui::Context`, asks the fonts for
// each width, and hands them here so that the arithmetic can be read, and
// tested, without one. The other two are the rectangle it lays out inside and
// the values it lays out for. A struct to carry the six would be `TransportRow`
// again with a width where each `Rect` is, built one field at a time by the
// caller and consumed once here — which is these arguments with a name on them
// and a second type to keep in step with the first.
#[allow(clippy::too_many_arguments)]
fn transport_row(
    row: Rect,
    t: &Transport,
    bpm_w: f32,
    label_w: f32,
    bar_w: f32,
    frame_w: f32,
    health_w: Option<f32>,
    rec_w: Option<f32>,
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
    // The `.sep`: everything after it is against the right padding. **The
    // last of those is the `rec` pill where there is one and the health
    // capsule where there is not**, so the right padding is claimed by
    // whichever of the three ends the row, and everything before it is laid
    // out backwards from there — which is the same "from the right edge
    // backwards" the frame readout has always been laid out by, asked of the
    // thing beside it rather than of the row.
    //
    // **The pill is last because the mock puts it last**, after `landed`; and
    // nothing is reserved for one that is not drawn, which is the rule this
    // end of the row has always followed.
    let right = row.max.x - size::TRANSPORT_PAD_X;
    let rec = rec_w.map(|w| {
        Rect::from_min_size(
            Pos2::new(right - w, mid - size::PILL_H * 0.5),
            egui::vec2(w, size::PILL_H),
        )
    });
    let health_end = match rec {
        Some(pill) => pill.min.x - size::TRANSPORT_GAP,
        None => right,
    };
    let health = health_w.map(|w| {
        Rect::from_min_size(
            Pos2::new(health_end - w, mid - size::PILL_H * 0.5),
            egui::vec2(w, size::PILL_H),
        )
    });
    let frame_end = match health {
        Some(pill) => pill.min.x - size::TRANSPORT_GAP,
        None => health_end,
    };
    let frame = Rect::from_min_size(
        Pos2::new(frame_end - frame_w, mid - span_h * 0.5),
        egui::vec2(frame_w, span_h),
    );
    // The same rule `picture_rect` states, on the two ends of the row: the
    // number is the tallest thing in it and the frame readout is the furthest
    // right, so a row that holds both holds everything between them — and the
    // last clause is the wrap the mock does and this does not.
    //
    // **The capsule is inside the same `and`, and the whole row goes when it
    // does not fit.** It is one of the row's readouts rather than a control
    // drawn over it, so a window too narrow to hold it draws nothing — which
    // is the answer this row already gives, one item along.
    match row.contains_rect(bpm)
        && row.contains_rect(frame)
        && health.is_none_or(|pill| row.contains_rect(pill))
        && rec.is_none_or(|pill| row.contains_rect(pill))
        && frame.min.x >= bar.max.x + size::TRANSPORT_GAP
    {
        true => Some(TransportRow {
            bpm,
            label,
            grid,
            dots,
            at: t.position(),
            bar,
            frame,
            health,
            rec,
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
///   uses for its own glow. **Those two are the ends of a ramp rather than two
///   states**: [`beat_at`] says how much of the light is on each dot, the fill
///   is mixed between the two colours by it and the halo is scaled by it, and
///   the mock's `.on` is what a dot with all of the light on it looks like.
///   The mock's own travel between those frames is `@keyframes beat-sweep`.
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
        let lit = row.lit(index);
        // **The halo belongs to the light rather than to the dot**, so it
        // fades in and out with it instead of switching. Nothing at all is
        // added for a dot the light has left: `beat_at` is exactly zero a
        // pitch away, so a grid of four is one halo at the instant of a beat
        // and two between two of them, and never four.
        if lit > 0.0 {
            painter.add(
                egui::epaint::Shadow {
                    offset: [0, 0],
                    blur: size::BEAT_GLOW,
                    spread: 0,
                    color: pal.glow_pink.gamma_multiply(lit),
                }
                .as_shape(dot, radius),
            );
        }
        painter.rect_filled(dot, radius, mix(pal.line, pal.pink, lit));
    }

    centred(row.bar, painter.layout_job(span(&bar_text(t), pal.dim)));
    centred(
        row.frame,
        painter.layout_job(frame_job(t, pal.text, pal.faint)),
    );

    // **The health capsule**, in the mock's own `.pill.armed` and only for the
    // verdict the mock draws it on. `armed` is this console's *live in the
    // good sense* — the same treatment the audio-in pill wears with an input
    // open and the wipe shape wears with a shape chosen — so it is the word
    // for a build that is on screen and running, and the wrong one for a
    // build that is not: a stopped slot drawn in the mint that says *this is
    // working* would read as the opposite of what it means. The other two take the plain
    // `.pill`, which is the mock's other treatment and not a third one
    // invented here; a colour of alarm is a decision `console.html` has not
    // taken for this capsule and is not taken for it here.
    if let (Some(rect), Some(stage)) = (row.health, t.health) {
        pill_into(ui, pal, rect, stage.word(), stage == Stage::Landed);
    }

    // **The `rec` pill**, in the mock's own two treatments and in no third
    // one: `.pill` while nothing is being recorded, and `.pill.on` while
    // something is. The pink is the mock's choice and it is the right one —
    // `.pill.on` is *this is the press that does it* and the pink is the pink
    // a tally on air is, which is what a recording running is. `armed` would
    // have said *this setting is chosen*, which is not what a running writer
    // is.
    if let (Some(rect), Some(rec)) = (row.rec, t.rec) {
        rec_into(ui, pal, rect, rec);
    }
}

/// **The `● rec` pill**: a capsule, a round mark, and the word after it.
///
/// It is not [`pill_into`] because it is not a capsule with a word in it: the
/// mock writes `&#9679; rec`, and the mark is **drawn** rather than set as a
/// glyph for [`Mask`]'s reason — a font this crate does not choose is not
/// something a control's width should depend on.
///
/// **The mark's diameter and the gap after it are the Outputs row's sink's**
/// ([`size::SINK_DOT`] and [`size::SINK_GAP`]), because that is this console's
/// other round mark before a word inside a capsule. Two numbers invented here
/// would be a second answer to a question `style.css` has already been read
/// for once.
///
/// **The mark takes the pill's own colour**, both ways: `.pill.on` is a pink
/// word over a pink wash and `.pill` is a dim word inside a hairline, and a
/// mark in some third ink would be a state this capsule does not have.
fn rec_into(ui: &Ui, pal: &Palette, rect: Rect, rec: Rec) {
    let painter = ui.painter();
    let radius = CornerRadius::same((rect.height() * 0.5) as u8);
    let ink = match rec {
        // `.pill.on`: no border, a `--c-pink` word over a wash of the same,
        // and the `box-shadow: 0 0 10px var(--c-glowp)` that goes with it —
        // `on_pill_at`'s three lines, drawn here because the word inside is
        // not the whole of what this capsule holds.
        Rec::Running => {
            painter.add(
                egui::epaint::Shadow {
                    offset: [0, 0],
                    blur: ON_GLOW,
                    spread: 0,
                    color: pal.glow_pink,
                }
                .as_shape(rect, radius),
            );
            painter.rect_filled(rect, radius, tint(pal.pink, ON_WASH));
            pal.pink
        }
        // The plain `.pill`: a hairline round nothing, and the dim.
        Rec::Idle => {
            painter.rect_stroke(rect, radius, Stroke::new(1.0, pal.line), StrokeKind::Inside);
            pal.dim
        }
    };
    let dot = Rect::from_min_size(
        Pos2::new(
            rect.min.x + size::PILL_PAD_X,
            rect.center().y - size::SINK_DOT * 0.5,
        ),
        egui::vec2(size::SINK_DOT, size::SINK_DOT),
    );
    painter.circle_filled(dot.center(), size::SINK_DOT * 0.5, ink);
    let galley = painter.layout_no_wrap(
        REC_LABEL.to_owned(),
        FontId::new(size::BASE, FontFamily::Proportional),
        ink,
    );
    painter.galley(
        Pos2::new(
            dot.max.x + size::SINK_GAP,
            rect.center().y - galley.size().y * 0.5,
        ),
        galley,
        ink,
    );
}

// ---------------------------------------------------------------------------
// The audio-in pill
// ---------------------------------------------------------------------------

/// **The pill's first word**, `docs/manual/console.html`'s own, hyphen and
/// all: it is the head of the group of four that need an input, and the page
/// names that group by this pill.
const AUDIO_LABEL: &str = "audio-in";

/// **What the pill says with no input open**, and it is a word for a state
/// rather than a name — [`NO_ARRANGEMENT`] one pill to the left, and the
/// preview cell's `C · no slot` one bay down.
///
/// **It is not the same statement as silence**, which is the whole of
/// [P-0084](../../../../docs/principles/0084-a-confident-wrong-automatic-judgement-is-worse-than-not-judging.md):
/// a quiet room measures `0.0` at full confidence and this pill would name the
/// input it measured it through. `none` is *no provider* — every name answers
/// what it answered before audio existed — and the two must not read alike on
/// a panel, because one of them is a room and the other is a cable.
const NO_INPUT: &str = "none";

/// **What the card says where the machine has no inputs at all.**
///
/// A menu with nothing in it would be a card an operator presses and cannot
/// tell from one that failed to open, so the empty case says which it is. It
/// is drawn the way [`Menu::Naming`]'s field is — a card with one line in it
/// and no rows, so [`AudioInPill::row`] hands out no rectangle for something
/// that is not a list — and it is a sentence rather than a row because there
/// is nothing to pick: a press on it shuts the menu like a press on any other
/// part of the card.
const NO_INPUTS: &str = "no inputs on this machine";

/// **What the audio-in pill reads this frame, and whether its menu is down.**
///
/// # The input and the list are handed in, for [`Arrangement`]'s reason
///
/// Which input is open is a device and what inputs there are is an
/// enumeration of the host. `src/` takes no device (ADR-0156) and this crate's
/// manifest holds nothing that could reach one, so whoever opened the input
/// reads both and writes them here — the arrangement pill's seam with a
/// microphone in place of a directory.
///
/// # The menu is not handed in, and that is the other half of the same seam
///
/// Whether the card is down is what the *control* is doing rather than what
/// the instrument is doing, so it lives here and moves only through
/// [`AudioIn::opened`] and [`AudioIn::shut`] — ADR-0225 §d, one pill along. A
/// menu open in the program's memory would be this console drawing a state it
/// could not answer questions about.
///
/// **Two states and not [`Menu`]'s three.** That enum's third state is a name
/// being typed, and nothing here asks for letters: an input is picked from the
/// list of the ones that exist, and a device is not named into being. The
/// *mechanism* is ADR-0225's and is shared — the card hangs from the pill, it
/// is `.lib-row` inside `.lib-list` padding, it counts itself in the Library
/// bay's foot, and `input.rs`'s rule 2 is modal over it — and it is the
/// mechanism that record is about, not the shape of the flag.
#[derive(Debug, Clone, PartialEq)]
pub struct AudioIn {
    /// **The input in use**, by the description the device answers to, or
    /// `None` for an instrument with nothing open. See [`NO_INPUT`].
    pub device: Option<String>,
    /// **Every input the machine has**, in the order the host listed them.
    ///
    /// **Read when the menu opens rather than per frame**, which is
    /// [`Arrangement::filed`]'s rule for its reason: enumerating a host is a
    /// device read and a frame path does not do one (P-0091). It changes when
    /// somebody plugs something in, and the press that opens the menu is the
    /// moment that matters — so the list a hand is about to read is the list
    /// as of the press.
    ///
    /// Empty is a machine with no inputs, and the card says so in as many
    /// words ([`NO_INPUTS`]).
    pub inputs: Vec<String>,
    /// Whether the card is down. Private for [`Arrangement::menu`]'s reason.
    down: bool,
}

impl AudioIn {
    /// **An instrument with nothing open and nothing listed**, and the menu
    /// shut. A `const` for [`Arrangement::NONE`]'s reason: a test can name the
    /// state without building one.
    ///
    /// It is not what [`View::audio`] holds by default — that is `None`, which
    /// is a console nobody has told anything about audio and draws no pill at
    /// all. This is the console that has been told, and told there is nothing.
    pub const NONE: AudioIn = AudioIn {
        device: None,
        inputs: Vec::new(),
        down: false,
    };

    /// **What the pill says after `audio-in ·`**: the input in use, or
    /// [`NO_INPUT`].
    pub fn word(&self) -> &str {
        self.device.as_deref().unwrap_or(NO_INPUT)
    }

    /// The card is down.
    pub fn open(&self) -> bool {
        self.down
    }

    /// Put it down.
    pub fn opened(&mut self) {
        self.down = true;
    }

    /// Take it away.
    pub fn shut(&mut self) {
        self.down = false;
    }

    /// **How many rows the open card has**: one per input. Zero while it is
    /// shut, and zero on a machine with no inputs — that card is a sentence,
    /// not a list.
    fn rows(&self) -> usize {
        match self.down {
            true => self.inputs.len(),
            false => 0,
        }
    }
}

/// **What a press on the audio-in pill or on one of its rows asks for.**
///
/// Three arms rather than [`Ask`]'s five, and it is a second enum rather than
/// three of that one: two of those arms are the arrangement family's — a name
/// being asked for and a `panel::Op` — and neither is a thing this control can
/// ever want. A shared enum with two arms that cannot happen is a `match` every
/// caller has to answer for twice.
#[derive(Debug, Clone, PartialEq)]
pub enum AudioAsk {
    /// Put the card down — a press on the pill with it shut. **The caller
    /// reads the host on this**, and writes what it found into
    /// [`AudioIn::inputs`] before the next frame draws the card.
    Open,
    /// Take it away — a press on the pill again, or anywhere on the card that
    /// is not a row.
    Shut,
    /// **Listen to this input**, named. [`Operation::AttachBeatSource`] with a
    /// [`BeatSource::AudioInput`] carrying the name the row was drawn with —
    /// which is the name the device answered to when the list was read, and
    /// the name `AudioInput::open` matches a selector against.
    ///
    /// **Nothing is refused here.** A device that has gone away since the list
    /// was read is refused where it is opened, out loud, with the list as it
    /// is then (P-0094, P-0090): this control cannot see a device and must not
    /// pretend to.
    Operation(Operation),
}

/// **The audio-in pill, laid out**: the capsule, what is written in it, and
/// the card under it while it is down.
///
/// One derivation, for [`ArrangementPill`]'s reason — [`View::draw`] paints
/// exactly these rectangles and [`crate::input::claim`] hit-tests exactly
/// these rectangles.
#[derive(Debug, Clone, PartialEq)]
pub struct AudioInPill {
    /// **The capsule**, which is what a press has to land in to open the card.
    pub pill: Rect,
    /// Where `audio-in · Scarlett 2i2` is painted, inside the capsule's
    /// padding.
    pub text: Rect,
    /// The `▾` after it. Drawn rather than typed — see [`CHEVRON_W`].
    pub chevron: Rect,
    /// **The card, or `None` while it is shut.**
    pub menu: Option<Rect>,
    /// **How many of [`AudioIn::rows`] the card has room for**, between the
    /// pill and the bottom of the console. Fewer than there are is a machine
    /// with more inputs than the window is tall, and the foot says `n of m`
    /// in the Library bay's own words rather than the list quietly ending.
    /// Zero while it is shut, and zero on a machine with no inputs.
    pub rows: usize,
    /// **What the card would list if the window were tall enough**, carried so
    /// that the foot and the rows are one number rather than two.
    pub of: usize,
}

impl AudioInPill {
    /// Whether `p` is on the pill itself.
    pub fn hit(&self, p: karakuri_layout::Point) -> bool {
        self.pill.contains(Pos2::new(p.x, p.y))
    }

    /// **Whether `p` is anywhere this control owns** — the pill, or the card
    /// while it is down.
    pub fn owns(&self, p: karakuri_layout::Point) -> bool {
        let at = Pos2::new(p.x, p.y);
        self.pill.contains(at) || self.menu.is_some_and(|menu| menu.contains(at))
    }

    /// **Where one row of the card is**, from the top — [`ArrangementPill::row`]
    /// without the rule, because this card has no verbs above its list.
    ///
    /// Panics on a row this card has not got, which is that function's rule: a
    /// caller has invented an item.
    pub fn row(&self, index: usize) -> Rect {
        assert!(index < self.rows, "row {index} of a card of {}", self.rows);
        let menu = self.menu.expect("a card with rows in it");
        Rect::from_min_size(
            Pos2::new(
                menu.min.x + size::LIB_LIST_PAD,
                menu.min.y + size::LIB_LIST_PAD + size::LIB_ROW_H * index as f32,
            ),
            egui::vec2(menu.width() - size::LIB_LIST_PAD * 2.0, size::LIB_ROW_H),
        )
    }

    /// **Which input `p` is on**, as an index into [`AudioIn::inputs`], or
    /// `None` for a point on no row — the card's padding, its foot, or
    /// anywhere off it.
    pub fn item(&self, p: karakuri_layout::Point) -> Option<usize> {
        let at = Pos2::new(p.x, p.y);
        (0..self.rows).find(|index| self.row(*index).contains(at))
    }

    /// **What a press at `p` asks for**, or `None` where the press was on
    /// nothing this control owns. [`ArrangementPill::ask`]'s shape over a list
    /// with no verbs in it.
    ///
    /// A press on the pill toggles the card; a press anywhere else on the card
    /// shuts it, because a press that did nothing at all is the one thing
    /// worse than a press that declines.
    pub fn ask(&self, audio: &AudioIn, p: karakuri_layout::Point) -> Option<AudioAsk> {
        if self.hit(p) {
            return Some(match audio.open() {
                true => AudioAsk::Shut,
                false => AudioAsk::Open,
            });
        }
        let menu = self.menu?;
        if !menu.contains(Pos2::new(p.x, p.y)) {
            return None;
        }
        Some(
            match self.item(p).and_then(|index| audio.inputs.get(index)) {
                Some(name) => AudioAsk::Operation(Operation::AttachBeatSource {
                    source: BeatSource::AudioInput(name.clone()),
                }),
                None => AudioAsk::Shut,
            },
        )
    }
}

/// **The audio-in pill's furniture, derived**: the capsule, the words in it,
/// and the card under it.
///
/// # Where it sits, and why it is first of the drawn controls in this row
///
/// `docs/manual/console.html`'s `.transport` puts `.tracker` — the four things
/// that need an input — immediately after `bar 37`, and this pill is the head
/// of that group. Everything the mock draws between the bar and here is
/// nothing at all, so this lands one [`size::TRANSPORT_GAP`] after the bar.
///
/// **The other three of its group are drawn now**, and they are
/// [`tracker_group`]'s: the offset, the tap and the octave, laid out from this
/// pill's right edge at [`size::PILL_GAP`] — the group's own tighter gap,
/// which is what `.tracker` sets and what this paragraph said would happen the
/// day they landed. [`arrangement`] is laid out from the *group's* right edge
/// now rather than from this pill's, at the row's gap, because what ends there
/// is a whole group.
///
/// # No pill at all where the console has not been told
///
/// `None` wherever [`transport`] answers `None` — the row folded away, soloed
/// away, too narrow, or a console with no engine behind it — and `None` again
/// where `audio` is `None`, which is a console nobody has said anything to
/// about audio and is every test in this crate that does not say otherwise. A
/// pill reading `audio-in · none` on a console that was never told is a
/// reading invented here, which is [`View::transport`]'s own rule: **empty is
/// a state and unasked is not.**
///
/// # What it costs to ask
///
/// One galley lookup for the pill's own words, always, and **while the card is
/// down one more per input**, since the card is as wide as the widest name in
/// it. Paid on a pointer event and on a frame, and only while an operator is
/// looking at the card.
///
/// `layout` must be solved: [`Layout::rect`] refuses to answer from a dirty
/// one.
pub fn audio_in(
    ctx: &egui::Context,
    layout: &karakuri_layout::Layout,
    values: Option<Transport>,
    audio: Option<&AudioIn>,
) -> Option<AudioInPill> {
    let audio = audio?;
    // The row, asked once and for everything, exactly as `arrangement` asks
    // it: whether there is one at all, and where the bar ended.
    let row = transport(ctx, layout, values)?;
    let strip = to_egui(layout.rect(layout.find("transport")?));
    let width = |text: &str| {
        ctx.fonts_mut(|f| {
            f.layout_no_wrap(
                text.to_owned(),
                FontId::new(size::BASE, FontFamily::Proportional),
                Color32::PLACEHOLDER,
            )
            .size()
            .x
        })
    };

    let text_w = width(&audio_text(audio));
    let pill_w = size::PILL_PAD_X * 2.0 + text_w + size::SINK_GAP + CHEVRON_W;
    let mid = strip.center().y;
    let pill = Rect::from_min_size(
        Pos2::new(
            row.bar.max.x + size::TRANSPORT_GAP,
            mid - size::PILL_H * 0.5,
        ),
        egui::vec2(pill_w, size::PILL_H),
    );
    // The same rule `arrangement` and `outputs_row` state: a capsule that does
    // not fit in the row it is drawn in is no control at all, rather than half
    // of one over the frame readout.
    if !strip.contains_rect(pill) || pill.max.x + size::TRANSPORT_GAP > row.frame.min.x {
        return None;
    }
    let text = Rect::from_min_size(
        Pos2::new(pill.min.x + size::PILL_PAD_X, mid - size::PILL_H * 0.5),
        egui::vec2(text_w, size::PILL_H),
    );
    let chevron = Rect::from_center_size(
        Pos2::new(pill.max.x - size::PILL_PAD_X - CHEVRON_W * 0.5, mid),
        egui::vec2(CHEVRON_W, CHEVRON_H),
    );

    let (menu, rows, of) = input_card(&pill, layout, audio, &width);
    Some(AudioInPill {
        pill,
        text,
        chevron,
        menu,
        rows,
        of,
    })
}

/// **The pill's words**: `audio-in · Scarlett 2i2`, or `audio-in · none`. One
/// run of text, for [`pill_text`]'s reason.
fn audio_text(audio: &AudioIn) -> String {
    format!("{AUDIO_LABEL} · {}", audio.word())
}

/// **The card under the audio-in pill**: where it is, how many rows fit in it,
/// and how many there are.
///
/// [`menu_card`]'s arithmetic without the hairline, because this list has no
/// verbs over it — so the furniture is the padding alone. A machine with no
/// inputs gets a card one line tall with [`NO_INPUTS`] in it and no rows at
/// all, which is the shape `menu_card` gives a name being typed and for the
/// same reason: that card is not a list, and nothing may hand out a row
/// rectangle for it.
fn input_card(
    pill: &Rect,
    layout: &karakuri_layout::Layout,
    audio: &AudioIn,
    width: &dyn Fn(&str) -> f32,
) -> (Option<Rect>, usize, usize) {
    if !audio.open() {
        return (None, 0, 0);
    }
    let viewport = to_egui(layout.viewport());
    let top = pill.max.y + size::PILL_GAP;
    let furniture = size::LIB_LIST_PAD * 2.0;

    if audio.inputs.is_empty() {
        let card = held_inside(
            &viewport,
            pill.min.x,
            top,
            width(NO_INPUTS).max(pill.width()) + (size::LIB_ROW_PAD_X + size::LIB_LIST_PAD) * 2.0,
            furniture + size::LIB_ROW_H,
        );
        return (Some(card), 0, 0);
    }

    let of = audio.rows();
    let widest = audio
        .inputs
        .iter()
        .map(|name| width(name))
        .fold(pill.width(), f32::max);
    // How many rows there is room for between the card's top and the bottom of
    // the console. Asked twice for `menu_card`'s reason: the foot is only owed
    // where something is left out.
    let room = |foot: f32| {
        (((viewport.max.y - top - furniture - foot) / size::LIB_ROW_H).floor()).max(0.0) as usize
    };
    let rows = match room(0.0) >= of {
        true => of,
        false => room(size::LIB_FOOT_H).min(of),
    };
    let foot = match rows < of {
        true => size::LIB_FOOT_H,
        false => 0.0,
    };
    let card = held_inside(
        &viewport,
        pill.min.x,
        top,
        widest + (size::LIB_ROW_PAD_X + size::LIB_LIST_PAD) * 2.0,
        furniture + size::LIB_ROW_H * rows as f32 + foot,
    );
    (Some(card), rows, of)
}

/// **The audio-in pill, painted**, and the card under it.
///
/// Where everything goes is [`audio_in`]'s, so this paints and derives
/// nothing. Term for term from `.pill` and `.pill.armed` in `style.css`:
///
/// - shut, with nothing open: the ordinary capsule — `border: 1px solid
///   var(--c-line); color: var(--c-dim)` — which is [`arrangement_into`]'s
///   treatment and this is the same pill two places along the same row.
/// - **with an input open, `.armed`**: `border-color: transparent; color:
///   var(--c-mint); background: color-mix(in srgb, var(--c-mint) 14%,
///   transparent)`, which is the mock's own class on this pill and the one
///   place in this row a colour means *live*. The `box-shadow: 0 0 9px
///   var(--c-glow)` goes with it, exactly as the beat grid's lit dot carries
///   its halo.
///
/// **The colour and the word say the same thing on purpose.** A pill that was
/// only lit would leave *which* room is being heard unanswered, and a pill
/// that only carried a name would make a dead input and a live one look alike
/// at the distance a panel is read from.
fn audio_in_into(ui: &Ui, pal: &Palette, pill: &AudioInPill, audio: &AudioIn) {
    let painter = ui.painter();
    let radius = CornerRadius::same((size::PILL_H * 0.5) as u8);
    let armed = audio.device.is_some();
    let ink = match armed {
        true => pal.mint,
        false => pal.dim,
    };
    if armed {
        painter.add(
            egui::epaint::Shadow {
                offset: [0, 0],
                blur: ARMED_GLOW,
                spread: 0,
                color: pal.glow,
            }
            .as_shape(pill.pill, radius),
        );
        painter.rect_filled(pill.pill, radius, tint(pal.mint, ARMED_WASH));
    } else {
        painter.rect_stroke(
            pill.pill,
            radius,
            Stroke::new(size::HAIRLINE, pal.line),
            StrokeKind::Inside,
        );
    }
    let galley = painter.layout_no_wrap(
        audio_text(audio),
        FontId::new(size::BASE, FontFamily::Proportional),
        ink,
    );
    painter.galley(
        Pos2::new(
            pill.text.min.x,
            pill.text.center().y - galley.size().y * 0.5,
        ),
        galley,
        ink,
    );
    painter.add(egui::Shape::convex_polygon(
        vec![
            pill.chevron.left_top(),
            pill.chevron.right_top(),
            Pos2::new(pill.chevron.center().x, pill.chevron.max.y),
        ],
        ink,
        Stroke::NONE,
    ));

    let Some(card) = pill.menu else {
        return;
    };
    painter.add(pal.shadow.as_shape(card, CornerRadius::same(8)));
    painter.rect_filled(card, CornerRadius::same(8), pal.panel);
    painter.rect_stroke(
        card,
        CornerRadius::same(8),
        Stroke::new(size::HAIRLINE, pal.line),
        StrokeKind::Inside,
    );

    let line = |rect: Rect, text: String, colour: Color32| {
        let galley = painter.layout_no_wrap(
            text,
            FontId::new(size::BASE, FontFamily::Proportional),
            colour,
        );
        painter.galley(
            Pos2::new(
                rect.min.x + size::LIB_ROW_PAD_X,
                rect.center().y - galley.size().y * 0.5,
            ),
            galley,
            colour,
        );
    };

    // **A machine with no inputs**, which is a sentence and not a list —
    // `pill.rows` is zero, so `row` hands out nothing.
    if audio.inputs.is_empty() {
        let only = Rect::from_min_size(
            Pos2::new(
                card.min.x + size::LIB_LIST_PAD,
                card.min.y + size::LIB_LIST_PAD,
            ),
            egui::vec2(card.width() - size::LIB_LIST_PAD * 2.0, size::LIB_ROW_H),
        );
        line(only, NO_INPUTS.to_owned(), pal.faint);
        return;
    }

    for index in 0..pill.rows {
        let name = &audio.inputs[index];
        // **The one that is open is the one colour the list has**, for the
        // reason the pill has one: a card of names with nothing marked leaves
        // an operator to remember which they picked.
        let colour = match audio.device.as_deref() == Some(name.as_str()) {
            true => pal.mint,
            false => pal.text,
        };
        line(pill.row(index), name.clone(), colour);
    }
    // **`n of m`, in the Library bay's own words**, and only where the list
    // could not be shown whole.
    if pill.rows < pill.of {
        let foot = Rect::from_min_max(
            Pos2::new(card.min.x, card.max.y - size::LIB_FOOT_H),
            card.max,
        );
        let galley = painter.layout_no_wrap(
            format!("{} of {}", pill.rows, pill.of),
            FontId::new(size::BASE, FontFamily::Proportional),
            pal.faint,
        );
        painter.galley(
            Pos2::new(
                foot.min.x + size::LIB_ROW_PAD_X,
                foot.center().y - galley.size().y * 0.5,
            ),
            galley,
            pal.faint,
        );
    }
}

/// **The mock's `box-shadow: 0 0 9px var(--c-glow)` on `.pill.armed`**, as the
/// blur it is.
const ARMED_GLOW: u8 = 9;

/// **`color-mix(in srgb, var(--c-mint) 14%, transparent)`**, as the percentage
/// [`tint`] takes.
const ARMED_WASH: u8 = 14;

/// **The mock's `box-shadow: 0 0 10px var(--c-glowp)` on `.pill.on`**, as the
/// blur it is — one pixel wider than [`ARMED_GLOW`] and in the pink halo
/// rather than the mint one, which is the same pair
/// [`crate::room::size::TALLY_GLOW`] is on the other side of this bay.
const ON_GLOW: u8 = 10;

/// **`color-mix(in srgb, var(--c-pink) 16%, transparent)`**, as the percentage
/// [`tint`] takes. Two points heavier than [`ARMED_WASH`], which is the mock's
/// own difference between *armed* and *this is the press that does it*.
const ON_WASH: u8 = 16;

// ---------------------------------------------------------------------------
// The tracker group: the latency offset, the tap and the octave
// ---------------------------------------------------------------------------

/// **The word before the offset track** — [`EXPOSURE_LABEL`]'s arrangement at
/// the other end of this row, and the mock's own word: a faint label, one
/// [`size::TRIM_GAP`], a horizontal [`fader`], another gap and the figure.
const OFFSET_LABEL: &str = "offset";

/// **The unit, written into the figure and never left off.**
///
/// `docs/manual/console.html` is plain about why: *"Two things on this panel
/// are called an offset, and they are not the same thing … The unit is what
/// tells them apart, so neither is ever drawn without it."* The other is the
/// deck head's anchor, which is in **beats** and is one deck's.
const OFFSET_UNIT: &str = " ms";

/// **What is written in the tap capsule**, and the whole of it — the mock
/// writes one word with no value beside it, because a tap has no state to
/// read back. It is [`TONEMAP_LABEL`]'s kind of capsule and not
/// [`AUDIO_LABEL`]'s: no `▾`, because nothing opens.
const TAP_LABEL: &str = "tap";

/// **The two halves of the octave**, in the mock's own marks — `&frac12;` and
/// `&times;2`, which are one character and two.
const HALVE_MARK: &str = "½";
const DOUBLE_MARK: &str = "×2";

/// **The two ends of the latency offset, in milliseconds.**
///
/// `karakuri_environment::audio::LATENCY_OFFSET_RANGE` is `-200.0..=200.0` and
/// this is that range restated, for [`EXPOSURE_STOPS`]' reason one control
/// along: this crate depends on nothing that could reach it (ADR-0156), and a
/// control has to know its own ends to lay a track out.
///
/// **The copy is held to the original where both are visible**, which is
/// `crates/karakuri` — the binary that has this crate and that one as
/// dependencies — rather than asserted here. That is the difference between
/// this restatement and [`EXPOSURE_STOPS`]': the exposure's ends are private
/// to `karakuri-cli` and nothing can compare them, and these are public.
pub const LATENCY_OFFSET_MIN_MS: f32 = -200.0;
pub const LATENCY_OFFSET_MAX_MS: f32 = 200.0;

/// **What one press of an offset key moves it by**, in milliseconds —
/// `karakuri_environment::audio::LATENCY_OFFSET_STEP_MS`, restated here for
/// the reason the two ends above are, and held to the original in the same
/// place.
pub const LATENCY_OFFSET_STEP_MS: f32 = 5.0;

/// **How many presses of an offset key cross the whole span**: four hundred
/// milliseconds at five a press is **eighty**.
const OFFSET_PRESSES: f32 =
    (LATENCY_OFFSET_MAX_MS - LATENCY_OFFSET_MIN_MS) / LATENCY_OFFSET_STEP_MS;

/// **The track, in pixels: one pixel a press** — [`EXPOSURE_TRACK_W`]'s whole
/// argument, one control along and on a value that is a difference rather than
/// a ratio
/// ([ADR-0277](../../../../docs/adr/0277-the-latency-offset-is-a-track-because-a-capsule-cannot-name-a-value.md)).
///
/// A pointer on this track can ask for any of the eighty positions along it
/// and `o` and `p` can ask for any of the eighty values between the ends, so
/// **neither surface can reach a value the other cannot** — which is what
/// stops a track and a pair of keys nearly agreeing. `docs/manual/console.html`
/// carries the same sentence for the exposure and now for this.
pub const OFFSET_TRACK_W: f32 = OFFSET_PRESSES;

/// **What a point `unit` of the way along the track asks for**, in
/// milliseconds.
///
/// **Linear, and that is not the exposure's arithmetic.** An exposure is a
/// *ratio* and its track is logarithmic for a stated reason — *"an additive
/// step would be enormous at 0.1 and invisible at 8.0"*. A latency offset is a
/// **difference** between two arrival times: five milliseconds is five
/// milliseconds at either end of the range, which is why the key steps by an
/// addition and why equal distances along this track are equal numbers of
/// milliseconds. The middle of the track is exactly zero, because the range is
/// symmetric and the subtraction is exact at a half.
pub fn offset_at(at: f32) -> f32 {
    LATENCY_OFFSET_MIN_MS + unit(at) * (LATENCY_OFFSET_MAX_MS - LATENCY_OFFSET_MIN_MS)
}

/// **Where an offset sits on the track**, on `[0, 1]` — [`offset_at`]
/// inverted, and clamped to the ends for [`unit_of`]'s reason: a value past
/// either end is drawn at that end and the figure beside the track is what
/// says the number. Nothing on the way in is held to these ends — the clamp is
/// `karakuri_environment::audio`'s, at the one place an offset is applied — so
/// this is drawing a reading rather than enforcing a bound.
pub fn unit_of_offset(ms: f32) -> f32 {
    unit((ms - LATENCY_OFFSET_MIN_MS) / (LATENCY_OFFSET_MAX_MS - LATENCY_OFFSET_MIN_MS))
}

/// **What the three controls beside the audio-in pill read this frame**: the
/// offset the session is holding, and which way the grid can still be moved an
/// octave.
///
/// # The same seam as [`View::transport`], and it is the beat lock's rather
/// than the engine's
///
/// Every field is a number or a `bool` and none of them is a device. Whether
/// an octave is available is `karakuri_audio`'s `BPM_RANGE` against the tempo
/// the session is running at, and the offset is a value
/// `karakuri_environment::audio::Audio` holds — `src/` has neither (ADR-0156),
/// so whoever owns them reads them and writes this per frame, exactly as
/// whoever owns the engine writes [`View::look`].
///
/// **It carries what cannot be derived and nothing that can.** The console
/// holds the tempo already — [`Transport::bpm`] — but not the range the
/// tracker searches, so *which half is live* is one of the two things it
/// cannot work out for itself. `docs/manual/console.html` says the panel can
/// work it out *before the press*, and this is what makes that true.
///
/// **A second value rather than three more fields on [`Transport`]**, for
/// [`Look`]'s reason: a tempo and a frame cost are what the session is doing,
/// and these are what the *tracker* is doing. A console driving one and not the
/// other is a state the type should be able to say.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tracker {
    /// **The latency offset the open session is holding**, in milliseconds, or
    /// `None` where nothing is open.
    ///
    /// `None` draws no offset control at all rather than a track at a
    /// plausible figure, which is [`View::audio`]'s own rule one control to the
    /// left: an offset belongs to a session, and until one is open there is no
    /// value being held anywhere for a track to point at. The page says so.
    pub offset_ms: Option<f32>,
    /// **Whether halving the grid is available at this tempo** — the tracker's
    /// range against half of what the session is running at.
    pub halve: bool,
    /// **And whether doubling is.** Never both: the range is under two octaves
    /// wide, which is the mock's own note and is why this is two fields rather
    /// than a direction.
    pub double: bool,
}

/// **The offset control, laid out**: the word, the track, what the value fills
/// of it, the band a press has to land in, and the figure.
///
/// [`LookRow`]'s exposure half, on a linear value — and a type of its own
/// because it is the one part of [`TrackerGroup`] that is not always there.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OffsetTrack {
    /// The faint `offset` before the track.
    pub label: Rect,
    /// **The track**, `.fader`'s 5px well lying down at [`OFFSET_TRACK_W`]
    /// long.
    pub track: Rect,
    /// What the offset fills of it, from the left — [`unit_of_offset`].
    pub fill: Rect,
    /// **What a press has to land in to set the offset**: the track grown to a
    /// line's height and no wider, which is [`LookRow::grip`]'s rule and its
    /// argument — a 5px-tall target is not something a hand finds, and a
    /// target reaching past either end would have two pixels of itself asking
    /// for the same value.
    pub grip: Rect,
    /// `−15 ms`, after the track. See [`tracker_group`] for why it is after it.
    pub value: Rect,
}

/// **The three controls that finish the tracker group, laid out**: the offset,
/// the tap and the two halves of the octave.
///
/// # One derivation, for [`LookRow`]'s reason
///
/// [`View::draw`] paints exactly these rectangles and [`crate::input::claim`]
/// hit-tests exactly these rectangles. They are laid end to end from the
/// audio-in pill's right edge, so where the octave is depends on how wide the
/// offset's figure is — three questions about one laid-out group, and a second
/// walk would put the chip a press lands on somewhere the mark is not.
///
/// # Why they are one group and not three controls in a row
///
/// `docs/manual/style.css` says it at `.tracker`, which is the class the mock
/// puts round exactly these four: *"the pill that says whether there is an
/// audio input, the latency offset … the tap, and the octave. None of them
/// means anything without an input, so they are one item at the pills' own 5px
/// rather than four at the row's 14."* So the gap **inside** this group is
/// [`size::PILL_GAP`] and the gap between the group and what follows it is
/// [`size::TRANSPORT_GAP`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrackerGroup {
    /// **The offset**, or `None` where no session is open to be holding one.
    pub offset: Option<OffsetTrack>,
    /// **The tap capsule**, which is what a press has to land in to tap the
    /// beat. Drawn whatever the audio-in pill says, because what it asks for
    /// does not depend on a reading — a tap with no room to tap against is
    /// refused **out loud**, which is `Operation::TapBeat`'s own row on
    /// `docs/manual/operations.html`.
    pub tap: Rect,
    /// Where `tap` is painted, inside the capsule's padding.
    pub tap_text: Rect,
    /// **The `½` half**, drawn whether or not it is live.
    pub halve: Rect,
    /// **The `×2` half.** At most one of the two is ever live, which is the
    /// range being under two octaves wide, and the inert one keeps its shape
    /// rather than going away — `.octave i.idle`, and the deck head's scrub
    /// arrows one bay over.
    pub double: Rect,
    /// **The values these rectangles were measured from**, carried for
    /// [`TransportRow::values`]' reason: whoever measured the type and whoever
    /// paints it are one statement.
    pub values: Tracker,
}

impl TrackerGroup {
    /// Whether `p` is on the tap capsule.
    pub fn hit_tap(&self, p: karakuri_layout::Point) -> bool {
        self.tap.contains(Pos2::new(p.x, p.y))
    }

    /// Whether `p` is on the offset track's band. See [`OffsetTrack::grip`],
    /// and `false` where there is no offset drawn at all.
    pub fn hit_offset(&self, p: karakuri_layout::Point) -> bool {
        self.offset
            .is_some_and(|offset| offset.grip.contains(Pos2::new(p.x, p.y)))
    }

    /// **Which half of the octave `p` is on, as the factor it asks for** — or
    /// `None` off both, and `None` on either while that direction is refused.
    ///
    /// **Inert is not claimed**, which is [`DeckHead::arrow`]'s rule word for
    /// word and [`crate::input`]'s *a control claims what it acts on and no
    /// more*. Both halves are drawn on every tempo, because the range is under
    /// two octaves wide and a pair that vanished would move the rest of the
    /// row under the hand every time the grid crossed 100 or 120 BPM.
    fn half(&self, p: karakuri_layout::Point) -> Option<GridScale> {
        let p = Pos2::new(p.x, p.y);
        match (
            self.values.halve && self.halve.contains(p),
            self.values.double && self.double.contains(p),
        ) {
            (true, _) => Some(GridScale::Halve),
            (_, true) => Some(GridScale::Double),
            _ => None,
        }
    }

    /// **Whether `p` is on any of the three**, which is what
    /// [`crate::input::claim`] asks.
    pub fn owns(&self, p: karakuri_layout::Point) -> bool {
        self.hit_tap(p) || self.hit_offset(p) || self.half(p).is_some()
    }

    /// **What a press at `p` asks for when it lands on the tap capsule**:
    /// [`Operation::TapBeat`], which carries nothing because a tap is an
    /// instant and the instant is when the operation arrives.
    ///
    /// **It is emitted with no input open too**, and that is the decision
    /// rather than a gap: the operations page says the refusal *"says where to
    /// open one rather than doing nothing, because a key that declines and a
    /// key that is not bound are the same experience"*, and the surface that
    /// could see there was no room is not this one — this crate has no device
    /// (ADR-0156). So the press names the operation and whoever performs it
    /// says what happened, exactly as `b` already does.
    pub fn tapped(&self, p: karakuri_layout::Point) -> Option<Operation> {
        self.hit_tap(p).then_some(Operation::TapBeat)
    }

    /// **What a press at `p` asks the grid to do**:
    /// [`Operation::ScaleGrid`] naming the direction, or `None` off both
    /// halves and on a refused one.
    pub fn octave(&self, p: karakuri_layout::Point) -> Option<Operation> {
        self.half(p).map(|by| Operation::ScaleGrid { by })
    }

    /// **What a press at `p` asks the offset to become**, or `None` where
    /// there is no track under it.
    ///
    /// # The press sets it outright, and that is the decision
    ///
    /// Where along the track the press landed *is* the value, through
    /// [`offset_at`], and what comes out is [`Operation::SetLatencyOffset`]
    /// naming it. The manual is where the words come from — *"The value is
    /// absolute and the keys are the nudge … because a control that could only
    /// be nudged is a control no fader can reach"* — and it is
    /// [`LookRow::exposure`]'s argument arriving one control earlier, because
    /// that control was built from this sentence.
    ///
    /// **It is not [`Mixer::grab`]**, for [`LookRow::exposure`]'s reason:
    /// there is no knob here and no gesture to be mid-way through, so a press
    /// that names a value cannot move the mix under a hand.
    pub fn nudge(&self, p: karakuri_layout::Point) -> Option<Operation> {
        let offset = self.offset?;
        self.hit_offset(p).then(|| Operation::SetLatencyOffset {
            ms: offset_at((p.x - offset.track.min.x) / offset.track.width()),
        })
    }
}

/// **The tracker's other three controls, derived**: the offset, the tap and
/// the octave.
///
/// # Where they sit, and why it is after the audio-in pill
///
/// `docs/manual/console.html`'s `.tracker` is the four things that need an
/// input, in this order: the pill that says whether there is one, the offset,
/// the tap, the octave. [`audio_in`] is the head of it and this is the rest,
/// laid out from that pill's right edge at [`size::PILL_GAP`] — the group's
/// own tighter gap and not the row's — which is exactly what [`audio_in`]'s
/// own documentation said would happen the day these were drawn.
///
/// **With no pill it lands after the bar**, at [`size::TRANSPORT_GAP`], which
/// is where [`arrangement`] used to land and is the same fallback: a console
/// told about the tracker and not about audio is a state nothing in this
/// workspace produces, and it is laid out rather than refused because a
/// derivation that panicked on it would be answering a question about a device
/// on its own authority.
///
/// # The order inside the group, and where the figure goes
///
/// The mock's, unchanged. **The figure is after the track** for
/// [`look`]'s reason, which is the one piece of this layout decided by what a
/// hand does rather than by what the mock draws: it is the only part whose
/// width moves with its value, so putting it last leaves the track and
/// everything after it where they were, and a target that walked away from the
/// pointer as it was set would be worst at the moment it was being used most
/// precisely.
///
/// # No row and no values means none of this
///
/// `None` wherever [`transport`] answers `None` — the row folded away, soloed
/// away, too narrow, or a console with no engine behind it — and `None` again
/// where `tracker` is `None`, which is a console nobody has told anything
/// about the beat tracker and is every test in this crate that does not say
/// otherwise. Drawing a tap on a console with no tracker behind it would be
/// the scaffolding this module refuses.
///
/// # What it costs to ask
///
/// **Five galley lookups of its own**, and two of them go with the offset: the
/// `tap` word, the `offset` word, the figure — which is the one whose width
/// moves with its value — and one for each of `½` and `×2`, since a capsule
/// here is as wide as the mark in it. A console with nothing open pays three
/// of the five.
///
/// And **it re-derives the row and the pill**, which is [`look`]'s honest cost
/// one group to the left and for the same reason: the derivation that draws a
/// control is the one that hit-tests it.
///
/// Paid on a pointer event and on a frame, and a console with no tracker
/// behind it pays none of it: the `tracker?` is the first line.
///
/// `layout` must be solved: [`Layout::rect`] refuses to answer from a dirty
/// one.
pub fn tracker_group(
    ctx: &egui::Context,
    layout: &karakuri_layout::Layout,
    values: Option<Transport>,
    audio: Option<&AudioIn>,
    tracker: Option<Tracker>,
) -> Option<TrackerGroup> {
    let tracker = tracker?;
    let row = transport(ctx, layout, values)?;
    let strip = to_egui(layout.rect(layout.find("transport")?));
    let width = |text: &str| {
        ctx.fonts_mut(|f| {
            f.layout_no_wrap(
                text.to_owned(),
                FontId::new(size::BASE, FontFamily::Proportional),
                Color32::PLACEHOLDER,
            )
            .size()
            .x
        })
    };
    let mark = |text: &str| {
        ctx.fonts_mut(|f| {
            f.layout_no_wrap(
                text.to_owned(),
                FontId::new(size::OCTAVE_SIZE, FontFamily::Proportional),
                Color32::PLACEHOLDER,
            )
            .size()
            .x
        })
    };

    let mid = strip.center().y;
    let span_h = size::BASE * size::LINE;
    // **Where the group's head ended.** The pill where the console has been
    // told about audio, and the bar where it has not — [`arrangement`]'s own
    // one-answer arrangement, read one item earlier in the same row.
    let (after, gap) = match audio_in(ctx, layout, values, audio) {
        Some(pill) => (pill.pill.max.x, size::PILL_GAP),
        None => (row.bar.max.x, size::TRANSPORT_GAP),
    };

    // The offset, where a session is open to be holding one.
    let mut at = after + gap;
    let offset = tracker.offset_ms.map(|ms| {
        let label = Rect::from_min_size(
            Pos2::new(at, mid - span_h * 0.5),
            egui::vec2(width(OFFSET_LABEL), span_h),
        );
        let track = Rect::from_min_size(
            Pos2::new(label.max.x + size::TRIM_GAP, mid - size::FADER_H * 0.5),
            egui::vec2(OFFSET_TRACK_W, size::FADER_H),
        );
        let grip = Rect::from_min_max(
            Pos2::new(track.min.x, mid - size::PILL_H * 0.5),
            Pos2::new(track.max.x, mid + size::PILL_H * 0.5),
        );
        let value = Rect::from_min_size(
            Pos2::new(track.max.x + size::TRIM_GAP, mid - span_h * 0.5),
            egui::vec2(width(&offset_text(ms)), span_h),
        );
        at = value.max.x + size::PILL_GAP;
        OffsetTrack {
            label,
            track,
            fill: filled(track, Axis::Row, unit_of_offset(ms)),
            grip,
            value,
        }
    });

    let tap_w = size::PILL_PAD_X * 2.0 + width(TAP_LABEL);
    let tap = Rect::from_min_size(
        Pos2::new(at, mid - size::PILL_H * 0.5),
        egui::vec2(tap_w, size::PILL_H),
    );
    let tap_text = Rect::from_min_size(
        Pos2::new(tap.min.x + size::PILL_PAD_X, mid - size::PILL_H * 0.5),
        egui::vec2(width(TAP_LABEL), size::PILL_H),
    );

    let chip = |x: f32, text: &str| {
        Rect::from_min_size(
            Pos2::new(x, mid - size::OCTAVE_H * 0.5),
            egui::vec2(
                mark(text) + size::OCTAVE_PAD_X * 2.0 + size::HAIRLINE * 2.0,
                size::OCTAVE_H,
            ),
        )
    };
    let halve = chip(tap.max.x + size::PILL_GAP, HALVE_MARK);
    let double = chip(halve.max.x + size::OCTAVE_GAP, DOUBLE_MARK);

    // The rule [`arrangement`], [`look`] and `outputs_row` all state: a control
    // that does not fit in the row it is drawn in is no control at all, rather
    // than half of one over the frame readout.
    if !strip.contains_rect(tap)
        || !strip.contains_rect(double)
        || double.max.x + size::TRANSPORT_GAP > row.frame.min.x
    {
        return None;
    }

    Some(TrackerGroup {
        offset,
        tap,
        tap_text,
        halve,
        double,
        values: tracker,
    })
}

/// **The figure after the track**: the offset, signed both ways and never
/// without its unit — `−15 ms`, and `+0 ms` at the middle.
///
/// **The sign is always drawn**, which is the page's own instruction: *"the
/// sign is the half that gets read wrong at two in the morning, so the panel
/// says it in words rather than leaving −15 ms to be interpreted."* The words
/// are `karakuri`'s, in the line it prints; the **mark** is this control's, and
/// a `+` that only appeared above zero would be a control that says less
/// exactly where it matters.
///
/// Whole milliseconds, because the step is five of them and the ends are two
/// hundred: a decimal place here would be a precision no surface can ask for.
///
/// **The sign is written in ASCII**, as the deck head's anchor writes its own
/// signed offset — the `−` in the mock and in this comment is the page's
/// typography, and the panel paints what a `{:+}` writes.
///
/// **And the rounding is taken before the sign, so there is no `-0 ms`.** A
/// press a fraction of a pixel left of the middle asks for a value that rounds
/// to zero, and `{:+.0}` writes `-0` for it: two spellings of the one value an
/// operator is most likely to be aiming at.
fn offset_text(ms: f32) -> String {
    // `-0.0 == 0.0` in IEEE, so this is the whole of the normalisation.
    let whole = match ms.round() == 0.0 {
        true => 0.0,
        false => ms.round(),
    };
    format!("{whole:+.0}{OFFSET_UNIT}")
}

/// **The tracker's three controls, painted.**
///
/// Where everything goes is [`tracker_group`]'s, so this paints and derives nothing.
/// Term for term from `style.css`:
///
/// - the `offset` before the track — `style="color:var(--c-faint)"` in the
///   markup, which is `pal.faint`, and is [`look_into`]'s treatment of `exp`.
/// - `.fader` and `.fader b` — the well and the ramp [`look_into`] paints, and
///   `.fader s` is deliberately not drawn for its reason: the mock's knob is a
///   handle and this control has none.
/// - `.pill` — the tap capsule, which is [`arrangement_into`]'s treatment with
///   no `▾`.
/// - `.octave i` — `border: 1px solid var(--c-line); color: var(--c-dim)` at
///   `border-radius: 999px`, and `.octave i.idle` is `color: var(--c-faint);
///   border-color: var(--c-hair)`. **Never grey without a reason** is the
///   stylesheet's own comment, and the reason is on the mock's tooltip.
fn tracker_into(ui: &Ui, pal: &Palette, group: &TrackerGroup) {
    let painter = ui.painter();
    let centred = |rect: Rect, galley: std::sync::Arc<egui::Galley>, colour: Color32| {
        painter.galley(
            Pos2::new(rect.min.x, rect.center().y - galley.size().y * 0.5),
            galley,
            colour,
        );
    };

    if let Some(offset) = group.offset {
        centred(
            offset.label,
            painter.layout_no_wrap(
                OFFSET_LABEL.to_owned(),
                FontId::new(size::BASE, FontFamily::Proportional),
                pal.faint,
            ),
            pal.faint,
        );
        let radius = CornerRadius::same((size::FADER_H * 0.5) as u8);
        painter.rect_filled(offset.track, radius, pal.well);
        painter.rect_stroke(
            offset.track,
            radius,
            Stroke::new(size::HAIRLINE, pal.hair),
            StrokeKind::Inside,
        );
        gradient(painter, offset.fill, Axis::Row, pal.mint, pal.lav, true);
        centred(
            offset.value,
            painter.layout_no_wrap(
                offset_text(group.values.offset_ms.unwrap_or_default()),
                FontId::new(size::BASE, FontFamily::Proportional),
                pal.text,
            ),
            pal.text,
        );
    }

    painter.rect_stroke(
        group.tap,
        CornerRadius::same((size::PILL_H * 0.5) as u8),
        Stroke::new(size::HAIRLINE, pal.line),
        StrokeKind::Inside,
    );
    centred(
        group.tap_text,
        painter.layout_no_wrap(
            TAP_LABEL.to_owned(),
            FontId::new(size::BASE, FontFamily::Proportional),
            pal.dim,
        ),
        pal.dim,
    );

    let half = |rect: Rect, text: &str, live: bool| {
        let (ink, edge) = match live {
            true => (pal.dim, pal.line),
            false => (pal.faint, pal.hair),
        };
        painter.rect_stroke(
            rect,
            CornerRadius::same((size::OCTAVE_H * 0.5) as u8),
            Stroke::new(size::HAIRLINE, edge),
            StrokeKind::Inside,
        );
        let galley = painter.layout_no_wrap(
            text.to_owned(),
            FontId::new(size::OCTAVE_SIZE, FontFamily::Proportional),
            ink,
        );
        painter.galley(
            Pos2::new(
                rect.center().x - galley.size().x * 0.5,
                rect.center().y - galley.size().y * 0.5,
            ),
            galley,
            ink,
        );
    };
    half(group.halve, HALVE_MARK, group.values.halve);
    half(group.double, DOUBLE_MARK, group.values.double);
}

// ---------------------------------------------------------------------------
// learn, and the map it writes into
// ---------------------------------------------------------------------------

/// **What the `map` pill reads** — the name of the map file in use, or `None`
/// for a surface running without one.
///
/// [`AudioIn`]'s shape one pill along, and it is the same seam: a map is a
/// file, `src/` reads none (ADR-0156), so whoever loaded one writes its name
/// here and this crate draws it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MapPill {
    /// What the map is called — the file's stem, which is the name an operator
    /// gave it and not its path (P-0087).
    pub name: Option<String>,
}

impl MapPill {
    /// **A surface with no map**, which draws `map · none`. It is a state and
    /// not an absence: a run that opened a port and found no file to load is
    /// one an operator can still learn into.
    pub const NONE: MapPill = MapPill { name: None };

    /// What the pill says after `map ·`.
    pub fn word(&self) -> &str {
        self.name.as_deref().unwrap_or(NO_MAP)
    }
}

/// **What the pill says where no map is loaded**, and it is deliberately not a
/// name: `none` is a word for a state, where a name would be a reading this
/// console invented. [`NO_ARRANGEMENT`]'s argument one pill along, and the
/// same shape [`audio_text`] uses for an input nobody opened.
const NO_MAP: &str = "none";

/// The `learn` pill's word, which is the whole of its label.
const LEARN_LABEL: &str = "learn";

/// The `map` pill's first word, the mock's own abbreviation — `map · <name>`.
const MAP_LABEL: &str = "map";

/// **The `learn` pill**: the capsule, and whether it is lit.
///
/// **A pill and not a capsule with two ends**, which the `rec` control is: a
/// press means *the other state* and the word never changes, because *learn*
/// is what the control is rather than what a press will do. It is `.pill.lav`
/// in the mock, lit while armed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LearnPill {
    /// **The control**: the capsule a press has to land in.
    pub pill: Rect,
    /// Whether learn is armed, read off [`View::learn`] and kept nowhere here.
    pub armed: bool,
}

impl LearnPill {
    /// Whether `p` is on the control.
    pub fn hit(&self, p: karakuri_layout::Point) -> bool {
        self.pill.contains(Pos2::new(p.x, p.y))
    }

    /// **What a press asks for**, which is a state and never a direction
    /// (P-0090): the arming, the other way round.
    ///
    /// It is a `bool` and not an [`Operation`] because **a learn is not an
    /// operation** — it is a setting of the map layer every surface reaches
    /// the vocabulary through, which is `Vocabulary::Setting`'s own shape
    /// ([ADR-0236](../../../../docs/adr/0236-a-map-is-the-layer-between-a-surface-and-the-vocabulary-and-the-audit-is-one-of-the-things-it-does.md),
    /// and [ADR-0336](../../../../docs/adr/0336-a-learn-is-a-map-edit-and-the-tips-midi-line-is-the-live-map.md)
    /// for the argument against the other reading). The four `mcp` pills are
    /// the same kind of control and carry no operation either.
    pub fn next(&self) -> bool {
        !self.armed
    }
}

/// **The `map` pill**: where the readout goes.
///
/// **No press and no menu**, which is what makes this a readout rather than
/// the control the mock draws. The mock's tip says *"Click to save, load, or
/// start a new one"*, and reaching a map while running is not built — the pill
/// says which file is loaded and nothing else. A capsule that looked like a
/// menu and opened none is the scaffolding [`transport`] refuses; a capsule
/// that says a true thing is not.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MapRow {
    /// The capsule, painted.
    pub pill: Rect,
}

/// **Where the `learn` pill goes**, or `None` where there is no room or
/// nothing to say.
///
/// `None` where the console has not been told about a map — `View::map` — for
/// [`audio_in`]'s reason exactly: a program with no surface has nothing to
/// learn onto, and a lit-able pill drawn for it would be this crate answering
/// a question about a device on its own authority.
///
/// `layout` must be solved.
pub fn learn_pill(
    ctx: &egui::Context,
    layout: &karakuri_layout::Layout,
    values: Option<Transport>,
    audio: Option<&AudioIn>,
    tracker: Option<Tracker>,
    map: Option<&MapPill>,
    armed: bool,
) -> Option<LearnPill> {
    map?;
    let row = transport(ctx, layout, values)?;
    let strip = to_egui(layout.rect(layout.find("transport")?));
    let text_w = width_of(ctx, LEARN_LABEL);
    let pill_w = size::PILL_PAD_X * 2.0 + text_w;
    let mid = strip.center().y;
    // **Where the group before this one ended** — the tracker group's last
    // chip, the audio-in pill, or the bar. The same one answer [`arrangement`]
    // takes, asked two items further back; the gap is the row's own.
    let after = match (
        tracker_group(ctx, layout, values, audio, tracker),
        audio_in(ctx, layout, values, audio),
    ) {
        (Some(group), _) => group.double.max.x,
        (None, Some(before)) => before.pill.max.x,
        (None, None) => row.bar.max.x,
    };
    let pill = Rect::from_min_size(
        Pos2::new(after + size::TRANSPORT_GAP, mid - size::PILL_H * 0.5),
        egui::vec2(pill_w, size::PILL_H),
    );
    // [`outputs_row`]'s rule: a capsule that does not fit in its row is no
    // control at all rather than half of one over the frame readout.
    if !strip.contains_rect(pill) || pill.max.x + size::TRANSPORT_GAP > row.frame.min.x {
        return None;
    }
    Some(LearnPill { pill, armed })
}

/// **Where the `map` pill goes**, or `None` where there is no room or nothing
/// to say.
///
/// It follows [`learn_pill`], which is the mock's order — `learn`, then
/// `map · nanoKONTROL2` — and falls back through the same chain where the
/// `learn` pill did not fit, so one control dropping for want of room does not
/// take the next with it.
pub fn map_pill(
    ctx: &egui::Context,
    layout: &karakuri_layout::Layout,
    values: Option<Transport>,
    audio: Option<&AudioIn>,
    tracker: Option<Tracker>,
    map: Option<&MapPill>,
) -> Option<MapRow> {
    let map = map?;
    let row = transport(ctx, layout, values)?;
    let strip = to_egui(layout.rect(layout.find("transport")?));
    let words = format!("{MAP_LABEL} · {}", map.word());
    let pill_w = size::PILL_PAD_X * 2.0 + width_of(ctx, &words);
    let mid = strip.center().y;
    let after = match (
        learn_pill(ctx, layout, values, audio, tracker, Some(map), false),
        tracker_group(ctx, layout, values, audio, tracker),
        audio_in(ctx, layout, values, audio),
    ) {
        (Some(learn), _, _) => learn.pill.max.x,
        (None, Some(group), _) => group.double.max.x,
        (None, None, Some(before)) => before.pill.max.x,
        (None, None, None) => row.bar.max.x,
    };
    let pill = Rect::from_min_size(
        Pos2::new(after + size::TRANSPORT_GAP, mid - size::PILL_H * 0.5),
        egui::vec2(pill_w, size::PILL_H),
    );
    if !strip.contains_rect(pill) || pill.max.x + size::TRANSPORT_GAP > row.frame.min.x {
        return None;
    }
    Some(MapRow { pill })
}

/// One text measurement, the way every pill in this row takes one.
fn width_of(ctx: &egui::Context, text: &str) -> f32 {
    ctx.fonts_mut(|f| {
        f.layout_no_wrap(
            text.to_owned(),
            FontId::new(size::BASE, FontFamily::Proportional),
            Color32::PLACEHOLDER,
        )
        .size()
        .x
    })
}

// ---------------------------------------------------------------------------
// The arrangement pill
// ---------------------------------------------------------------------------

/// **The pill's first word**, the mock's own abbreviation one pill along from
/// `map · nanoKONTROL2 ▾`. The map pill names a file with *save*, *load* and
/// *start a new one* under it, and this is the same three over a different
/// file, so it is the same two-part label.
const ARRANGEMENT_LABEL: &str = "arr";

/// **What the pill says where no arrangement has been named, and it is
/// deliberately not a name.**
///
/// The default arrangement *has* no name: it reaches
/// [`crate::layout`] rather than a file, so it is the one arrangement nobody
/// could have saved and nothing filed can shadow it
/// ([ADR-0221](../../../../docs/adr/0221-an-arrangement-is-named-by-the-operator-and-kept-in-a-fourth-place.md)
/// §2). Writing `default` here would be this console inventing one — and a
/// worse invention than most, because an operator *may* save an arrangement
/// called `default` and it shadows nothing, so the pill would read the same
/// for two different states.
///
/// A space is what makes it safe as well as honest: a name is one path
/// component of letters, digits, `-` and `_`, so `the default` is not a name
/// anything can be filed under and no save can make this line ambiguous. It is
/// the preview cell's `C · no slot` one row up — a word for a state, where a name
/// would be a reading invented for a console that has none.
const NO_ARRANGEMENT: &str = "the default";

/// **What *save* is called in the menu.** The ellipsis is the one thing on
/// this panel that says *this item asks for something before it does
/// anything*, and it is only there while it is true: with a name in use,
/// saving again means that name and asks for nothing, so the word loses it.
const SAVE_ITEM: &str = "save";
const SAVE_ITEM_ASKING: &str = "save as…";

/// **What the reset is called in the menu**, in the manual's own words rather
/// than in the vocabulary's. *Reset the arrangement* is the row and
/// [`Op::Reset`] is the operation; *start a new one* is what the family reads
/// as from inside this control, where the default is *"one arrangement among
/// the ones you could name"* and not a fourth thing beside the three.
const NEW_ITEM: &str = "start a new one";

/// **The `▾` at the end of the pill, drawn rather than typed** — [`grip_dots`]'
/// reason one control along. Whether a black down-pointing small triangle is
/// in `egui`'s default face is a question with no good answer, and a triangle
/// is the same mark either way.
///
/// Half the type it sits beside wide and half of that tall, which is about
/// what the glyph's ink measures at [`size::BASE`].
const CHEVRON_W: f32 = size::BASE * 0.5;
const CHEVRON_H: f32 = CHEVRON_W * 0.5;

/// **What the arrangement pill reads this frame, and what its menu is doing.**
///
/// # The name and the list are handed in, for [`View::picture`]'s reason
///
/// Which arrangement is in use is *which file was last written or read*, and
/// which names exist is a directory. `src/` takes no device, no window and no
/// clock (ADR-0156) and it takes no disk either — `karakuri-console`'s
/// manifest has no entry that could reach one — so whoever owns the store
/// reads both and writes them here, exactly as whoever owns the engine writes
/// [`View::transport`]. What crosses the seam is a name and a list of names.
///
/// # The menu is not handed in, and that is the other half of the same seam
///
/// [`Menu`] is this crate's: it is what the *control* is doing, not what the
/// instrument is doing, and it moves only through the methods below. A menu
/// open in the program's memory would be the arrangement living in the
/// toolkit's memory one level along (ADR-0156's own argument), and the console
/// would then be drawing a state it could not answer questions about.
///
/// It is held here rather than in [`Panel`] because it is not part of the
/// arrangement: nothing about an open menu is saved, restored or reset, and a
/// [`Panel::restore`] that put somebody else's open menu back would be
/// restoring a gesture.
#[derive(Debug, Clone, PartialEq)]
pub struct Arrangement {
    /// **The arrangement in use**, or `None` for the default — which is not a
    /// name and is drawn as [`NO_ARRANGEMENT`].
    ///
    /// A save and a restore both put a name here, because both leave that
    /// arrangement the one in use; a reset takes it away, because the default
    /// is what is now on screen and it has no name.
    pub name: Option<String>,
    /// **Every name already filed**, in the order whoever read the store
    /// listed them — `Store::list_arrangements` sorts by name, so this is
    /// alphabetical and the menu does not sort it again.
    ///
    /// **Read when it changes rather than per frame**, which is
    /// [`View::library`]'s rule for its reason: a listing is a directory read
    /// and that is not a thing to do on a frame path (P-0091). It changes
    /// exactly when a save lands, and whoever performed the save is who
    /// re-reads it.
    ///
    /// Empty is a console with no store behind it — every test in this crate —
    /// and the menu then offers *save* and *start a new one* and lists
    /// nothing, which is honest: there is nothing to put back.
    pub filed: Vec<String>,
    /// What the control is doing. See [`Menu`].
    pub menu: Menu,
}

/// **What the pill's menu is doing**, and the console's only state that is
/// neither the arrangement nor a value handed in.
///
/// Three states rather than a `bool` and a buffer beside it: *shut*, *open*,
/// and *open with a name being typed into it*. The third is a state of the
/// menu and not a fourth thing, which is what stops a buffer being read while
/// nothing is asking for one.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Menu {
    /// The pill alone.
    #[default]
    Shut,
    /// The menu is down: *save*, *start a new one*, and the names filed.
    Open,
    /// **The one flow on this panel that asks for letters**, with what has
    /// been typed so far.
    ///
    /// The buffer is a `String` this crate owns and whoever holds the keyboard
    /// fills, one character at a time, through [`Arrangement::typed`] and
    /// [`Arrangement::rubbed_out`] — the same split as everything else here,
    /// since `src/` has no key events to read (ADR-0156).
    ///
    /// **Nothing in it is checked.** A name that is not one path component is
    /// refused where the record is applied, in one sentence, by whoever writes
    /// the file — the surface owns the affordance and never the authority
    /// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md),
    /// [P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
    /// A pill that quietly dropped the characters it did not like would be a
    /// rule an operator could only find by experiment.
    Naming(String),
}

impl Arrangement {
    /// **A console with no store behind it**: the default arrangement, nothing
    /// filed, and the menu shut.
    ///
    /// A `const` rather than a `Default` impl alone so that a test — and
    /// [`crate::input::claim`]'s own documentation — can name the state
    /// without building one. Every test in this crate is this.
    pub const NONE: Arrangement = Arrangement {
        name: None,
        filed: Vec::new(),
        menu: Menu::Shut,
    };

    /// **What the pill says after `arr ·`**: the name in use, or the word for
    /// the arrangement that has none. See [`NO_ARRANGEMENT`].
    pub fn word(&self) -> &str {
        self.name.as_deref().unwrap_or(NO_ARRANGEMENT)
    }

    /// The menu is down, whether or not a name is being typed into it.
    pub fn open(&self) -> bool {
        !matches!(self.menu, Menu::Shut)
    }

    /// **What has been typed so far**, or `None` when nothing is asking for a
    /// name. The caret is drawn after it and there is no selection: this is a
    /// name, not a document.
    pub fn naming(&self) -> Option<&str> {
        match &self.menu {
            Menu::Naming(typed) => Some(typed),
            _ => None,
        }
    }

    /// Put the menu down.
    pub fn opened(&mut self) {
        self.menu = Menu::Open;
    }

    /// Take it away, typed name and all. A name abandoned half-typed is not
    /// kept for the next time the menu opens: the buffer is the gesture, and
    /// the gesture ended.
    pub fn shut(&mut self) {
        self.menu = Menu::Shut;
    }

    /// **Ask for a name**, starting from empty. Reached only where there is no
    /// name in use — with one in use, *save* means that name and asks nothing
    /// ([`ArrangementPill::ask`]).
    pub fn asks_a_name(&mut self) {
        self.menu = Menu::Naming(String::new());
    }

    /// **One character into the name being typed**, and `false` where nothing
    /// was asking for one.
    ///
    /// Control characters are not a name and never reach the buffer — a
    /// newline is Return arriving as text, which is the commit and not a
    /// letter. Everything else does, unchecked, for the reason
    /// [`Menu::Naming`] gives.
    pub fn typed(&mut self, c: char) -> bool {
        match (&mut self.menu, c.is_control()) {
            (Menu::Naming(name), false) => {
                name.push(c);
                true
            }
            _ => false,
        }
    }

    /// **The last character back out again**, and `false` where there was
    /// nothing to take — no name being typed, or an empty one.
    pub fn rubbed_out(&mut self) -> bool {
        match &mut self.menu {
            Menu::Naming(name) => name.pop().is_some(),
            _ => false,
        }
    }

    /// **How many rows the open menu has**: *save*, *start a new one*, and one
    /// per name filed. Zero while the menu is shut or asking for a name, which
    /// is a menu with a field in it rather than a list.
    fn rows(&self) -> usize {
        match self.menu {
            Menu::Open => VERBS + self.filed.len(),
            _ => 0,
        }
    }
}

impl Default for Arrangement {
    fn default() -> Arrangement {
        Arrangement::NONE
    }
}

/// **The two items above the list**: *save* and *start a new one*. The names
/// filed follow them, and *load* is that list rather than an item of its own —
/// which is the manual's own sentence: *"load is a list — the names already
/// filed, which a hand can pick without typing anything."*
const VERBS: usize = 2;

/// **What a press on one of the menu's rows lands on.**
///
/// [`ArrangementPill::ask`] turns one of these into what the press *asks for*;
/// this is only which row it was. Split in two so that the hit test and the
/// operation are one derivation asked twice rather than one function that does
/// both — [`Outputs::op`]'s arrangement, over a list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Item {
    /// *save*. With no name in use it asks for one; with a name in use it
    /// means that name.
    Save,
    /// *start a new one*, which is the reset.
    New,
    /// The name at this index of [`Arrangement::filed`] — put that
    /// arrangement back.
    Filed(usize),
}

/// **What a press on the pill or on one of its rows asks for.**
///
/// Every arm is either a move of this control's own state or one named
/// operation, and never a change to the arrangement made here: the pill asks,
/// and whoever applies the record decides
/// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
/// **Nothing is refused in this list.** A name nothing is filed under and a
/// file that disagrees with itself are both refused where the bytes are, in
/// one sentence each, and this control cannot see either.
#[derive(Debug, Clone, PartialEq)]
pub enum Ask {
    /// Put the menu down — a press on the pill with the menu shut.
    Open,
    /// Take it away — a press on the pill again, or anywhere off the menu
    /// while it is down.
    Shut,
    /// **Ask for a name**: *save* with no arrangement in use. The one item on
    /// this panel that asks for letters.
    Name,
    /// **The reset**, which reaches code and not a file, and is the same
    /// [`Op`] the `r` key performs — one operation, two surfaces
    /// ([ADR-0208](../../../../docs/adr/0208-resetting-is-the-default-case-of-restoring-an-arrangement.md)).
    Panel(Op),
    /// **One operation of the vocabulary, named**: a save under a name, or a
    /// restore of one. Both reach a file and only whoever holds the store can
    /// perform either.
    Operation(Operation),
}

/// **The arrangement pill, laid out**: the capsule, what is written in it, and
/// the menu under it while it is down.
///
/// # One derivation, for [`Outputs`]' reason
///
/// [`View::draw`] paints exactly these rectangles and [`crate::input::claim`]
/// hit-tests exactly these rectangles. Two copies of the arithmetic is a menu
/// row that lights under a pointer that cannot pick it, with nothing on screen
/// saying so.
///
/// # It carries no name and no list
///
/// The rectangles are here and the words are [`Arrangement`]'s, which is why
/// [`ArrangementPill::ask`] takes one: a laid-out pill that had *copied* the
/// name it was measured from is a second copy to drift, and the caller has the
/// first one in its hand already. It is [`Mixer`]'s split with the borrow
/// turned round — the mixer keeps the strips because a knob's position *is* a
/// value, and nothing here moves with the name except the width.
#[derive(Debug, Clone, PartialEq)]
pub struct ArrangementPill {
    /// **The capsule**, which is what a press has to land in to open the menu.
    /// The mock gives the whole pill the click and so does this.
    pub pill: Rect,
    /// Where `arr · night` is painted, inside the capsule's padding.
    pub text: Rect,
    /// The `▾` after it. Drawn rather than typed — see [`CHEVRON_W`].
    pub chevron: Rect,
    /// **The menu, or `None` while it is shut** — the whole card, which is
    /// what a press has to land in to be a pick rather than a dismissal.
    pub menu: Option<Rect>,
    /// **How many of [`Arrangement::rows`] the menu has room for**, between
    /// the pill and the bottom of the console.
    ///
    /// Fewer than there are is a store with more arrangements than the window
    /// is tall, and the foot says so in the Library bay's own words — `n of m`
    /// — rather than the list quietly ending. Zero while the menu is shut, and
    /// while it is asking for a name: that menu is a field, not a list.
    pub rows: usize,
    /// **What the menu would list if the window were tall enough**, carried so
    /// that the foot and the rows are one number rather than two.
    pub of: usize,
}

impl ArrangementPill {
    /// Whether `p` is on the pill itself.
    pub fn hit(&self, p: karakuri_layout::Point) -> bool {
        self.pill.contains(Pos2::new(p.x, p.y))
    }

    /// **Whether `p` is anywhere this control owns** — the pill, or the menu
    /// while it is down. This is what [`crate::input::claim`] asks, and it is
    /// wider than [`ArrangementPill::hit`] by exactly the open menu.
    pub fn owns(&self, p: karakuri_layout::Point) -> bool {
        let at = Pos2::new(p.x, p.y);
        self.pill.contains(at) || self.menu.is_some_and(|menu| menu.contains(at))
    }

    /// **Where one row of the menu is**, from the top. The rows stack with no
    /// gap between them, which is `.lib-list`'s own reading — the one list in
    /// the mock that has none.
    ///
    /// Panics on a row this menu has not got, which is [`TransportRow::dot`]'s
    /// rule: a caller has invented an item.
    pub fn row(&self, index: usize) -> Rect {
        assert!(index < self.rows, "row {index} of a menu of {}", self.rows);
        let menu = self.menu.expect("a menu with rows in it");
        let top = menu.min.y + size::LIB_LIST_PAD + size::LIB_ROW_H * index as f32
            // The rule under the two verbs, which the names sit below.
            + match index >= VERBS {
                true => size::HAIRLINE,
                false => 0.0,
            };
        Rect::from_min_size(
            Pos2::new(menu.min.x + size::LIB_LIST_PAD, top),
            egui::vec2(menu.width() - size::LIB_LIST_PAD * 2.0, size::LIB_ROW_H),
        )
    }

    /// **Which item `p` is on**, or `None` for a point on no row — the menu's
    /// padding, its foot, or anywhere off it.
    pub fn item(&self, p: karakuri_layout::Point) -> Option<Item> {
        let at = Pos2::new(p.x, p.y);
        (0..self.rows)
            .find(|index| self.row(*index).contains(at))
            .map(|index| match index {
                0 => Item::Save,
                1 => Item::New,
                other => Item::Filed(other - VERBS),
            })
    }

    /// **What a press at `p` asks for**, or `None` where the press was on
    /// nothing this control owns.
    ///
    /// The same derivation [`crate::input::claim`] hit-tests, asked a second
    /// time rather than copied — [`Outputs::op`]'s arrangement, and the reason
    /// is the same: the pill that claims a press and the pill that acts on it
    /// cannot come apart.
    ///
    /// # The three answers a press on a row can give
    ///
    /// - **Save** with a name in use is [`Operation::SaveArrangement`] naming
    ///   it. *"Once a name is in use, saving again means that name: saving
    ///   over it is what saving it again is."* With no name in use there is
    ///   nothing to save over, so it asks for one instead.
    /// - **Start a new one** is [`Op::Reset`] — the same operation `r`
    ///   performs, reached from the other end of the panel exactly as the
    ///   Outputs row's dot reaches `f`'s fold.
    /// - **A name** is [`Operation::RestoreArrangement`] naming it, which is
    ///   the reset's own sentence with a name in it.
    ///
    /// A press on the pill toggles the menu; a press anywhere else on the menu
    /// — its padding, its foot — shuts it, because a press that did nothing at
    /// all is the one thing worse than a press that declines.
    pub fn ask(&self, arr: &Arrangement, p: karakuri_layout::Point) -> Option<Ask> {
        if self.hit(p) {
            return Some(match arr.open() {
                true => Ask::Shut,
                false => Ask::Open,
            });
        }
        let menu = self.menu?;
        if !menu.contains(Pos2::new(p.x, p.y)) {
            return None;
        }
        Some(match self.item(p) {
            Some(Item::Save) => match &arr.name {
                Some(name) => Ask::Operation(Operation::SaveArrangement { name: name.clone() }),
                None => Ask::Name,
            },
            Some(Item::New) => Ask::Panel(Op::Reset),
            Some(Item::Filed(index)) => match arr.filed.get(index) {
                Some(name) => Ask::Operation(Operation::RestoreArrangement { name: name.clone() }),
                // A row past the end of the list, which `row` has already
                // refused to produce a rectangle for. Unreachable rather than
                // guessed at.
                None => Ask::Shut,
            },
            None => Ask::Shut,
        })
    }
}

/// **The arrangement pill's furniture, derived**: the capsule, the words in
/// it, and the menu under it.
///
/// # Where it sits, and why it is after the bar
///
/// `docs/manual/console.html`'s `.transport` is a flex row and this pill is
/// the last item in it before the `.sep`, immediately after
/// `map · nanoKONTROL2 ▾`. Everything the mock draws between the tracker group
/// and this pill — `learn`, and then `map` — is one of the controls
/// [`transport`] names and does not draw, so a flex row closes up and this
/// lands one [`size::TRANSPORT_GAP`] after the octave's second half — or after
/// the audio-in pill on a console with no tracker behind it, or after
/// `bar 37` on one with neither. That is the mock's own layout with the
/// undrawn items taken out, and not a position chosen here.
///
/// **It moves with the tempo, by a glyph or two.** `92.5` is narrower than
/// `128.0` and everything after it slides, which is what a flex row is and
/// what the beat grid and the bar already do. The alternative — pinning it to
/// the right edge, where the mock's `landed` and `rec` sit — buys a control
/// that never moves and puts it in the group the manual does not put it in.
///
/// # No row means no pill, and that is the row's answer rather than a second
/// one
///
/// `None` wherever [`transport`] answers `None`: the row folded away, soloed
/// away, too narrow, or a console with no engine behind it. The last is the
/// one worth stating, because the arrangement exists whether or not a tempo
/// does — but the *row* does not, and a pill floating in a bay that is drawing
/// nothing at all would be a control in a row that is not there. Where the row
/// is, this asks it for the bar's right edge and lays out from there, so the
/// pill's place and the readouts' places are one derivation.
///
/// # What it costs to ask
///
/// One galley lookup for the pill's own words, always. **While the menu is
/// down it is one more per row** — the two verbs and every name filed — since
/// the card is as wide as the widest thing in it, and a name this crate never
/// measured would be a name drawn outside its own card. Paid on a pointer
/// event and on a frame, and only while the menu is open, which is a menu an
/// operator is looking at.
///
/// `layout` must be solved: [`Layout::rect`] refuses to answer from a dirty
/// one.
pub fn arrangement(
    ctx: &egui::Context,
    layout: &karakuri_layout::Layout,
    values: Option<Transport>,
    audio: Option<&AudioIn>,
    tracker: Option<Tracker>,
    // **The map, only so this pill knows where the group before it ended.**
    // `learn` and `map` sit between the tracker group and this one in the
    // mock, and a console that has not been told about a surface draws
    // neither — so this is `None` on every run without one and the pill lands
    // exactly where it did before they existed.
    map: Option<&MapPill>,
    arr: &Arrangement,
) -> Option<ArrangementPill> {
    // The row, asked once and for everything: whether there is one at all,
    // and where the bar ended. `transport` answers the first for the whole
    // module, which is what keeps *no engine means nothing at all* in one
    // place.
    let row = transport(ctx, layout, values)?;
    let strip = to_egui(layout.rect(layout.find("transport")?));
    let width = |text: &str| {
        ctx.fonts_mut(|f| {
            f.layout_no_wrap(
                text.to_owned(),
                FontId::new(size::BASE, FontFamily::Proportional),
                Color32::PLACEHOLDER,
            )
            .size()
            .x
        })
    };

    let words = pill_text(arr);
    let text_w = width(&words);
    // `.pill`'s `padding: 0 8px` around the words, one `.sink` gap, and the
    // chevron — the one gap the mock states inside a capsule.
    let pill_w = size::PILL_PAD_X * 2.0 + text_w + size::SINK_GAP + CHEVRON_W;
    let mid = strip.center().y;
    // **Where the group before this one ended**, which is the tracker group's
    // last chip where the console has been told about the tracker, the
    // audio-in pill where it has been told only about audio, and the bar where
    // it has been told neither — the same one-answer arrangement [`look`]
    // takes of *this* pill, asked one item further back. The gap is the row's
    // own either way: what ends before this pill is a whole group, and
    // `.tracker`'s tighter 5 is the gap *inside* that group.
    let after = match (
        map_pill(ctx, layout, values, audio, tracker, map),
        learn_pill(ctx, layout, values, audio, tracker, map, false),
        tracker_group(ctx, layout, values, audio, tracker),
        audio_in(ctx, layout, values, audio),
    ) {
        (Some(map), _, _, _) => map.pill.max.x,
        (None, Some(learn), _, _) => learn.pill.max.x,
        (None, None, Some(group), _) => group.double.max.x,
        (None, None, None, Some(before)) => before.pill.max.x,
        (None, None, None, None) => row.bar.max.x,
    };
    let pill = Rect::from_min_size(
        Pos2::new(after + size::TRANSPORT_GAP, mid - size::PILL_H * 0.5),
        egui::vec2(pill_w, size::PILL_H),
    );
    // The same rule [`outputs_row`] states: a capsule that does not fit in the
    // row it is drawn in is no control at all, rather than half of one over
    // the frame readout.
    if !strip.contains_rect(pill) || pill.max.x + size::TRANSPORT_GAP > row.frame.min.x {
        return None;
    }
    let text = Rect::from_min_size(
        Pos2::new(pill.min.x + size::PILL_PAD_X, mid - size::PILL_H * 0.5),
        egui::vec2(text_w, size::PILL_H),
    );
    let chevron = Rect::from_center_size(
        Pos2::new(pill.max.x - size::PILL_PAD_X - CHEVRON_W * 0.5, mid),
        egui::vec2(CHEVRON_W, CHEVRON_H),
    );

    let (menu, rows, of) = menu_card(&pill, layout, arr, &width);
    Some(ArrangementPill {
        pill,
        text,
        chevron,
        menu,
        rows,
        of,
    })
}

/// **The pill's words**: `arr · night`, or `arr · the default`.
///
/// One string rather than three galleys laid end to end, for [`frame_job`]'s
/// reason: the mock writes one run of text and laying it out as one keeps the
/// spaces round the `·` the type's own rather than a gap this file invented.
fn pill_text(arr: &Arrangement) -> String {
    format!("{ARRANGEMENT_LABEL} · {}", arr.word())
}

/// **The menu card under the pill**: where it is, how many rows fit in it, and
/// how many there are.
///
/// `(None, 0, 0)` while the menu is shut, which is the ordinary state and
/// costs one branch.
///
/// # It hangs from the pill and is held inside the console
///
/// Down from the pill's bottom edge by one [`size::PILL_GAP`], left-aligned
/// with it, and pushed back inside the viewport's right edge where a long name
/// would take it past — a card half outside the window is a list with items
/// nobody can read.
///
/// **The bottom is a count and not a clip.** The transport row is at the top
/// of the console, so a menu hanging down has the whole window; where a store
/// holds more arrangements than that window is tall, the card lists as many as
/// fit and says `n of m` in the Library bay's own foot. Truncating in silence
/// is the failure P-0094 is about, and this is that bay's answer to the same
/// question rather than a second one.
fn menu_card(
    pill: &Rect,
    layout: &karakuri_layout::Layout,
    arr: &Arrangement,
    width: &dyn Fn(&str) -> f32,
) -> (Option<Rect>, usize, usize) {
    if !arr.open() {
        return (None, 0, 0);
    }
    let viewport = to_egui(layout.viewport());
    let top = pill.max.y + size::PILL_GAP;

    // **Asking for a name is a field and not a list**, so the card is one row
    // wide enough to type into and there is nothing to pick.
    if let Some(typed) = arr.naming() {
        let field = width(&naming_text(typed)).max(pill.width());
        let card = held_inside(
            &viewport,
            pill.min.x,
            top,
            field + (size::LIB_ROW_PAD_X + size::LIB_LIST_PAD) * 2.0,
            size::LIB_LIST_PAD * 2.0 + size::LIB_ROW_H,
        );
        return (Some(card), 0, 0);
    }

    let of = arr.rows();
    let widest = std::iter::once(save_word(arr))
        .chain(std::iter::once(NEW_ITEM))
        .chain(arr.filed.iter().map(String::as_str))
        .map(width)
        .fold(pill.width(), f32::max);
    // How many rows there is room for between the card's top and the bottom of
    // the console, once the padding and the rule under the verbs are paid for.
    // The foot is only owed where something is left out, so it is asked for
    // twice: once assuming it is not there and once assuming it is.
    let furniture = size::LIB_LIST_PAD * 2.0 + size::HAIRLINE;
    let room = |foot: f32| {
        (((viewport.max.y - top - furniture - foot) / size::LIB_ROW_H).floor()).max(0.0) as usize
    };
    let rows = match room(0.0) >= of {
        true => of,
        false => room(size::LIB_FOOT_H).min(of),
    };
    let foot = match rows < of {
        true => size::LIB_FOOT_H,
        false => 0.0,
    };
    let card = held_inside(
        &viewport,
        pill.min.x,
        top,
        widest + (size::LIB_ROW_PAD_X + size::LIB_LIST_PAD) * 2.0,
        furniture + size::LIB_ROW_H * rows as f32 + foot,
    );
    (Some(card), rows, of)
}

/// A card of that size at that corner, pushed back inside the viewport's right
/// edge if it would hang over it — and never past its left edge, which is what
/// the `max` is for on a window narrower than the card.
fn held_inside(viewport: &Rect, x: f32, y: f32, w: f32, h: f32) -> Rect {
    let x = x.min(viewport.max.x - w).max(viewport.min.x);
    Rect::from_min_size(Pos2::new(x, y), egui::vec2(w, h))
}

/// **What *save* is called with this arrangement in use.** The ellipsis is
/// there exactly while the item asks for something — see [`SAVE_ITEM`].
fn save_word(arr: &Arrangement) -> &'static str {
    match arr.name {
        Some(_) => SAVE_ITEM,
        None => SAVE_ITEM_ASKING,
    }
}

/// **The name being typed, with the caret after it** — `night▏`.
///
/// One run of text rather than a galley and a drawn bar, so that measuring the
/// field and painting it cannot be two different runs: the caret is the width
/// of the caret whatever the face is, and a field measured without it would
/// put the caret outside its own card at the moment the name filled it.
fn naming_text(typed: &str) -> String {
    format!("{typed}{CARET}")
}

/// **The caret**, a light vertical bar. Typed rather than drawn, unlike the
/// chevron: `U+258F LEFT ONE EIGHTH BLOCK` is a box-drawing character, and
/// `egui`'s default face carries the block elements. Where it did not, the
/// fallback is a visible box in the one place an operator is looking, which is
/// louder than a caret that has quietly gone.
const CARET: char = '▏';

/// **The arrangement pill, painted**, and the menu under it.
///
/// Where everything goes is [`arrangement`]'s, so this paints and derives
/// nothing. Term for term from `.pill` in `style.css`:
///
/// - `border: 1px solid var(--c-line); border-radius: 999px; padding: 0 8px;
///   color: var(--c-dim)` — a capsule with a hairline round it, which is
///   [`pill_at`]'s treatment and this is the same pill in another row.
/// - the `▾` — `--c-dim` with the words, since it is part of the same run in
///   the mock's markup.
///
/// The menu has no term in the stylesheet, because the mock draws no menu: it
/// is the Library bay's list, which is the one list this console already
/// draws, at the same `.lib-row` box and inside the same `.lib-list` padding
/// (P-0085). It sits on a card with the panel's own shadow under it, which is
/// what says it is above the bays rather than inside one.
/// **The `learn` pill, painted** — lavender while armed, the row's own outline
/// while it is not.
///
/// `.pill.lav` is the mock's class on this control and the palette's `lav` is
/// that colour, so *armed* is drawn in the ink the page already gave it rather
/// than in one this file chose. The word never changes: *learn* is what the
/// control **is**, and what a press will do is said by the lamp — which is the
/// distinction the `rec` capsule draws the other way, being one control with
/// two ends.
fn learn_into(ui: &Ui, pal: &Palette, pill: &LearnPill) {
    let painter = ui.painter();
    let radius = CornerRadius::same((size::PILL_H * 0.5) as u8);
    let ink = match pill.armed {
        true => pal.lav,
        false => pal.dim,
    };
    if pill.armed {
        painter.rect_filled(pill.pill, radius, pal.tint);
    }
    painter.rect_stroke(
        pill.pill,
        radius,
        Stroke::new(size::HAIRLINE, ink),
        StrokeKind::Inside,
    );
    let galley = painter.layout_no_wrap(
        LEARN_LABEL.to_owned(),
        FontId::new(size::BASE, FontFamily::Proportional),
        ink,
    );
    painter.galley(
        Pos2::new(
            pill.pill.min.x + size::PILL_PAD_X,
            pill.pill.center().y - galley.size().y * 0.5,
        ),
        galley,
        ink,
    );
}

/// **The `map` pill, painted** — `map · default`, and no chevron.
///
/// **The chevron is the one thing left off the mock's own drawing**, and it is
/// left off on purpose: `▾` on this console means *there is a menu under
/// this*, which the arrangement pill and the `audio-in` pill both keep, and
/// there is no menu here. Drawing one over a readout would be the scaffolding
/// [`transport`] refuses — a control that looks like a control and does
/// nothing.
fn map_into(ui: &Ui, pal: &Palette, pill: &MapRow, map: &MapPill) {
    let painter = ui.painter();
    painter.rect_stroke(
        pill.pill,
        CornerRadius::same((size::PILL_H * 0.5) as u8),
        Stroke::new(size::HAIRLINE, pal.line),
        StrokeKind::Inside,
    );
    let galley = painter.layout_no_wrap(
        format!("{MAP_LABEL} · {}", map.word()),
        FontId::new(size::BASE, FontFamily::Proportional),
        pal.dim,
    );
    painter.galley(
        Pos2::new(
            pill.pill.min.x + size::PILL_PAD_X,
            pill.pill.center().y - galley.size().y * 0.5,
        ),
        galley,
        pal.dim,
    );
}

fn arrangement_into(ui: &Ui, pal: &Palette, pill: &ArrangementPill, arr: &Arrangement) {
    let painter = ui.painter();
    painter.rect_stroke(
        pill.pill,
        // `border-radius: 999px` on a box this short is a capsule.
        CornerRadius::same((size::PILL_H * 0.5) as u8),
        Stroke::new(size::HAIRLINE, pal.line),
        StrokeKind::Inside,
    );
    let galley = painter.layout_no_wrap(
        pill_text(arr),
        FontId::new(size::BASE, FontFamily::Proportional),
        pal.dim,
    );
    painter.galley(
        Pos2::new(
            pill.text.min.x,
            pill.text.center().y - galley.size().y * 0.5,
        ),
        galley,
        pal.dim,
    );
    // The `▾`: a triangle with its point down, in the box `arrangement`
    // measured for it.
    painter.add(egui::Shape::convex_polygon(
        vec![
            pill.chevron.left_top(),
            pill.chevron.right_top(),
            Pos2::new(pill.chevron.center().x, pill.chevron.max.y),
        ],
        pal.dim,
        Stroke::NONE,
    ));

    let Some(card) = pill.menu else {
        return;
    };
    painter.add(pal.shadow.as_shape(card, CornerRadius::same(8)));
    painter.rect_filled(card, CornerRadius::same(8), pal.panel);
    painter.rect_stroke(
        card,
        CornerRadius::same(8),
        Stroke::new(size::HAIRLINE, pal.line),
        StrokeKind::Inside,
    );

    let row_text = |painter: &egui::Painter, rect: Rect, text: String, colour: Color32| {
        let galley = painter.layout_no_wrap(
            text,
            FontId::new(size::BASE, FontFamily::Proportional),
            colour,
        );
        painter.galley(
            Pos2::new(
                rect.min.x + size::LIB_ROW_PAD_X,
                rect.center().y - galley.size().y * 0.5,
            ),
            galley,
            colour,
        );
    };

    // **The field, where a name is being asked for.** The card has one row in
    // it and `pill.rows` is zero, which is what stops `row` handing out a
    // rectangle for something that is not a list.
    if let Some(typed) = arr.naming() {
        let field = Rect::from_min_size(
            Pos2::new(
                card.min.x + size::LIB_LIST_PAD,
                card.min.y + size::LIB_LIST_PAD,
            ),
            egui::vec2(card.width() - size::LIB_LIST_PAD * 2.0, size::LIB_ROW_H),
        );
        row_text(painter, field, naming_text(typed), pal.text);
        return;
    }

    for index in 0..pill.rows {
        let rect = pill.row(index);
        let (text, colour) = match index {
            0 => (save_word(arr).to_owned(), pal.text),
            1 => (NEW_ITEM.to_owned(), pal.text),
            other => (arr.filed[other - VERBS].clone(), pal.dim),
        };
        row_text(painter, rect, text, colour);
    }
    // The rule under the two verbs, which is what makes the names below it a
    // list rather than two more items.
    if pill.rows > VERBS {
        let y = pill.row(VERBS).min.y - size::HAIRLINE * 0.5;
        painter.line_segment(
            [
                Pos2::new(card.min.x + size::LIB_LIST_PAD, y),
                Pos2::new(card.max.x - size::LIB_LIST_PAD, y),
            ],
            Stroke::new(size::HAIRLINE, pal.hair),
        );
    }
    // **`n of m`, in the Library bay's own words**, and only where the list
    // could not be shown whole.
    if pill.rows < pill.of {
        let foot = Rect::from_min_max(
            Pos2::new(card.min.x, card.max.y - size::LIB_FOOT_H),
            card.max,
        );
        let galley = painter.layout_no_wrap(
            format!("{} of {}", pill.rows.saturating_sub(VERBS), pill.of - VERBS),
            FontId::new(size::LIB_FOOT_SIZE, FontFamily::Proportional),
            pal.faint,
        );
        painter.line_segment(
            [
                Pos2::new(foot.min.x, foot.min.y),
                Pos2::new(foot.max.x, foot.min.y),
            ],
            Stroke::new(size::HAIRLINE, pal.hair),
        );
        painter.galley(
            Pos2::new(
                foot.min.x + size::LIB_FOOT_PAD_X,
                foot.center().y - galley.size().y * 0.5,
            ),
            galley,
            pal.faint,
        );
    }
}

// ---------------------------------------------------------------------------
// The look: the tone map, and the level going into it
// ---------------------------------------------------------------------------

/// **The tone map pill's first word**, the mock's own two-part label one pill
/// along from `arr · night ▾` — and **no chevron**, which is what says this one
/// acts on the press instead of putting a card down. The two pills that open a
/// menu carry the mark and the three that do not (`learn`, `tap` — drawn since
/// 2026-09-08, and [`TAP_LABEL`] — and this) do not.
const TONEMAP_LABEL: &str = "tone";

/// **The word before the exposure track** — `.trim`'s `g` one row up, and the
/// same arrangement: a faint label, [`size::TRIM_GAP`], a horizontal
/// [`fader`].
const EXPOSURE_LABEL: &str = "exp";

/// **Every tone map operator, in the order this control walks them.**
///
/// `karakuri_operation::Tonemap` has no `ALL` — nothing had read one until now
/// — so unlike the blend chip's cycle this is **not** a second copy of a list
/// the vocabulary keeps ([`after`] says what that costs). It is the only
/// statement of the order in this workspace apart from `karakuri-cli`'s own
/// `next_tonemap`, which walks the engine's `TonemapOp` in exactly this order,
/// and it is what [`next_tonemap`] is checked against.
///
/// The order is the engine's own declaration order, which is the order
/// [ADR-0037](../../../../docs/adr/0037-tone-mapping-is-a-uniform-and-the-default-is-chosen-by-looking.md)
/// compared them in: `Clamp` first, because it is *"not a tone mapper — the
/// control that shows what the other three are fixing"* and is where a
/// `Present` that nobody has written to starts. A cycle should start where the
/// thing it cycles starts, which is [`next_shape`]'s own reason.
const TONEMAPS: [Tonemap; 4] = [
    Tonemap::Clamp,
    Tonemap::Reinhard,
    Tonemap::Aces,
    Tonemap::AgX,
];

/// **How many stops of exposure the track spans**, and the whole of what
/// decides its two ends: 1.0 sits at the middle, so the track runs from
/// `2^-6` to `2^6` — a sixty-fourth to sixty-four.
///
/// It is `karakuri-cli`'s own interactive range, restated rather than shared
/// because `EXPOSURE_MIN` and `EXPOSURE_MAX` are private to that binary and
/// this crate depends on nothing that could reach them. What is stated there
/// is *"the interactive bounds, and only the interactive ones"* — the bounds a
/// key nudges inside, where `--exposure` is deliberately not held to them
/// because *"a batch render asks for something extreme on purpose"*. A control
/// under a hand is exactly the interactive case, so this is the same span for
/// the same reason.
const EXPOSURE_STOPS: f32 = 12.0;

/// The two ends of that span, which is [`exposure_at`] at each end of the
/// track. Written out so a reader meets the numbers rather than an exponent,
/// and asserted against the derivation in `tests/look.rs`.
pub const EXPOSURE_MIN: f32 = 1.0 / 64.0;
pub const EXPOSURE_MAX: f32 = 64.0;

/// **How many presses of an exposure key cross the whole span.**
///
/// `karakuri-cli`'s `EXPOSURE_STEP` is `1.189_207`, which is `2^(1/4)`, and it
/// says why it is a ratio rather than an addition: *"a stop is a ratio, and an
/// additive step would be enormous at 0.1 and invisible at 8.0. This is a
/// quarter of a stop, near enough."* Four presses to a stop over
/// [`EXPOSURE_STOPS`] stops is forty-eight.
const EXPOSURE_PRESSES: f32 = EXPOSURE_STOPS * 4.0;

/// **The track, in pixels: one pixel a press.**
///
/// This is the number that makes the two surfaces agree instead of nearly
/// agreeing. A pointer on this track can ask for any of the 48 positions along
/// it and a keyboard stepping a quarter stop at a time can ask for any of the
/// 48 values between the ends, so **neither surface can reach a value the
/// other cannot** — and one pixel is the coarsest step that still reads as a
/// movement rather than as a sequence of positions, which is
/// [`BEAT_STALENESS`]' own rule about the beat's travel
/// ([P-0094](../../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)).
///
/// A width picked for looks would have been a number with nothing behind it,
/// which is what the mock's `width: 48px` says here as well.
pub const EXPOSURE_TRACK_W: f32 = EXPOSURE_PRESSES;

/// **What the look controls read this frame**: the operator that is running,
/// and the level going into it.
///
/// # The same seam as [`View::transport`], and it is not the Master bay's `out`
///
/// The look is the engine's — `karakuri_engine::frame::Look`, which is exactly
/// what `Present::set_tonemap` is told — and `src/` has no engine (ADR-0156),
/// so whoever owns one reads it and writes this per frame.
///
/// **`white_point` is not here**, and it is not an oversight: it is in the
/// record, it is Reinhard's parameter alone, and no surface has a control for
/// it, so a field here would be a value this crate carries and no control
/// names. That is the vocabulary's own line — `Operation::SetExposure` asks
/// for the level and nothing else, and whoever writes the record fills the
/// other two thirds in from the look that is running
/// ([ADR-0192](../../../../docs/adr/0192-an-operation-asks-for-what-a-surface-can-say-and-the-record-stays-whole.md)).
///
/// **And `exposure` here is the tone mapper's, never `Deck::set_out`.** The
/// two are levels that multiply in different places — the master out where the
/// mix writes the composited frame, this where the present pass reads it — and
/// with nothing in the master chain they are indistinguishable in every frame
/// this program can draw
/// ([ADR-0224](../../../../docs/adr/0224-out-and-exposure-are-two-levels-that-multiply-in-different-places.md)).
/// The Master bay's row is the other one and nothing can move it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Look {
    /// The transfer that is running. One of [`TONEMAPS`].
    pub tonemap: Tonemap,
    /// The level going into it. Unbounded on the way **in** — a `--exposure`
    /// flag is not held to the track's ends — and drawn wherever
    /// [`unit_of`] puts it, which is at an end for anything past one.
    pub exposure: f32,
}

/// **The two look controls, laid out**: the capsule that names the operator,
/// and the track that sets the level.
///
/// # One derivation, for [`Outputs`]' reason
///
/// [`View::draw`] paints exactly these rectangles and [`crate::input::claim`]
/// hit-tests exactly these rectangles. Two copies of the arithmetic is a track
/// that lights under a pointer that cannot set it.
///
/// # Two controls and one type, because they are one group in the row
///
/// They are two operations and two presses
/// ([ADR-0192](../../../../docs/adr/0192-an-operation-asks-for-what-a-surface-can-say-and-the-record-stays-whole.md)),
/// and they are laid out together because the second's place is measured from
/// the first's — which is what a flex row is. Splitting them into two
/// functions would mean measuring the capsule twice, once to draw it and once
/// to find out where the track starts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LookRow {
    /// **The capsule**, which is what a press has to land in to move the tone
    /// map on.
    ///
    /// **As wide as the widest of the four names whatever word is in it**,
    /// which is [`StripBox::tally`]'s rule and not the blend chip's: a target
    /// sized to the word it is showing moves under the hand that is pressing
    /// it, and here it would take the exposure track with it.
    pub tone: Rect,
    /// Where `tone · aces` is painted, inside the capsule's padding. Narrower
    /// than the capsule for every operator but the widest, so the words are
    /// centred in it.
    pub tone_text: Rect,
    /// The faint `exp` before the track.
    pub label: Rect,
    /// **The track**, `.fader`'s 5px well lying down at
    /// [`EXPOSURE_TRACK_W`] long.
    pub track: Rect,
    /// What the level fills of it, from the left — [`unit_of`] of the
    /// exposure.
    pub fill: Rect,
    /// **What a press has to land in to set the exposure**: the track grown to
    /// a line's height and no wider.
    ///
    /// Grown, because a 5px-tall target is not something a hand finds — the
    /// `.mini`'s own argument (*"a 15px word is not something a hand finds,
    /// and `.mini`'s padding is what makes it one"*), met here by a band
    /// rather than by drawing a fatter track. **No wider**, because the value
    /// a press asks for is where along the track it landed, and a target that
    /// reached past either end would have two pixels of itself asking for the
    /// same value.
    pub grip: Rect,
    /// `1.00`, after the track. See [`look`] for why it is after it.
    pub value: Rect,
    /// **The values these rectangles were measured from**, carried for
    /// [`TransportRow::values`]' reason: whoever measured the type and whoever
    /// paints it are one statement.
    pub values: Look,
}

impl LookRow {
    /// Whether `p` is on the tone map's capsule.
    pub fn hit_tone(&self, p: karakuri_layout::Point) -> bool {
        self.tone.contains(Pos2::new(p.x, p.y))
    }

    /// Whether `p` is on the exposure track's band. See [`LookRow::grip`].
    pub fn hit_exposure(&self, p: karakuri_layout::Point) -> bool {
        self.grip.contains(Pos2::new(p.x, p.y))
    }

    /// **Whether `p` is on either of the two**, which is what
    /// [`crate::input::claim`] asks.
    pub fn owns(&self, p: karakuri_layout::Point) -> bool {
        self.hit_tone(p) || self.hit_exposure(p)
    }

    /// **What a press at `p` asks the tone map to become**, or `None` where
    /// there is no capsule under it.
    ///
    /// # The capsule cycles, and the operation names where it arrived
    ///
    /// Click it and the look moves to the next of [`TONEMAPS`] — `clamp`,
    /// `reinhard`, `aces`, `agx`, wrapping — and what comes out is
    /// [`Operation::SetTonemap`] naming the **destination**, never a step,
    /// because there is no step in the vocabulary to name. The affordance is
    /// [`Mixer::blend`]'s exactly
    /// ([ADR-0187](../../../../docs/adr/0187-the-blend-mini-cycles-and-a-map-learns-the-three-it-cycles-through.md))
    /// and so is the division it rests on: the cycle is [`next_tonemap`] here
    /// and nothing at all in `karakuri-operation`, which is P-0090's
    /// division: a toggle is an affordance, built over operations by whoever
    /// draws the control.
    ///
    /// **What a map is offered is the four values, not the cycle** — the same
    /// sentence ADR-0187 wrote about three, and the reason the manual's row
    /// says a map or a model *"would name the one it wants"*.
    ///
    /// # Why not a menu, now that there is one
    ///
    /// ADR-0187 could not have a popup: the blend chip is in a 53-wide strip,
    /// and a card had nowhere to be and no rule to be modal under. Both of
    /// those changed
    /// ([ADR-0225](../../../../docs/adr/0225-a-menu-is-a-gesture-in-hand-rather-than-a-rectangle-on-the-panel.md)),
    /// so the alternative is genuinely available here and it is still refused.
    /// A menu is **a gesture in hand**: it takes every pointer event on the
    /// console until it is shut, and a boundary cannot be dragged while it is
    /// down. That is a price worth paying for a list of files an operator
    /// named and did not pay for four words that never change — and it puts a
    /// second press in front of a control that is pressed on the beat.
    pub fn tonemap(&self, p: karakuri_layout::Point) -> Option<Operation> {
        self.hit_tone(p).then(|| Operation::SetTonemap {
            tonemap: next_tonemap(self.values.tonemap),
        })
    }

    /// **What a press at `p` asks the exposure to become**, or `None` where
    /// there is no track under it.
    ///
    /// # The press sets it outright, and that is the decision
    ///
    /// Where along the track the press landed *is* the value, through
    /// [`exposure_at`], and what comes out is [`Operation::SetExposure`]
    /// naming it. The manual is where the words for that come from — *"the
    /// value is absolute and the keys are the nudge … because a control that
    /// could only be nudged is a control no fader can reach"* — and the whole
    /// span is reachable in one press.
    ///
    /// **It is not [`Mixer::grab`], and the difference is not an
    /// inconsistency.** A fader answers `None` for a press on its *track*, on
    /// purpose: it has a knob, the knob keeps the offset it was taken hold of
    /// at, and a press that jumped the value would move the mix under a hand
    /// mid-gesture. There is no knob here and no gesture to be mid-way
    /// through — the press is the whole of it — so the two rules are answers
    /// to two different questions rather than one rule applied twice. Nothing
    /// is drawn that looks like a handle, for the same reason.
    ///
    /// **[`crate::input`]'s *a control claims what it acts on and no more* is
    /// kept exactly.** Every point of [`LookRow::grip`] asks for a value, so
    /// nothing is claimed in order to be thrown away.
    ///
    /// # It says one thing per press, so it needs no coalescing
    ///
    /// [ADR-0207](../../../../docs/adr/0207-a-continuous-control-says-one-thing-per-frame.md)
    /// coalesces a swept MIDI fader to one operation a frame, and names the
    /// console's own fader as the other precedent — which emits only when the
    /// value changed, because it holds the value it last drew. Neither applies
    /// to a press: one press is one operation, and a second press in the same
    /// frame is a second thing an operator asked for.
    ///
    /// # And it can never be pending, which is why nothing marks a destination
    ///
    /// [ADR-0206](../../../../docs/adr/0206-a-fader-marks-where-it-is-going-and-keeps-reaching-for-it.md)
    /// gives a fader a mark for where a scheduled move is going.
    /// `karakuri_engine::deck::Control` is **per slot** — a gain, an opacity
    /// and a mask front — so there is no transition that can be scheduled on
    /// the look at all, and a mark here would be a presentation of a state
    /// that cannot occur.
    pub fn exposure(&self, p: karakuri_layout::Point) -> Option<Operation> {
        self.hit_exposure(p).then(|| Operation::SetExposure {
            exposure: exposure_at((p.x - self.track.min.x) / self.track.width()),
        })
    }
}

/// **The next tone map round the cycle**, wrapping from the last back to the
/// first — the whole of the affordance the capsule is.
///
/// [`after`]'s division, one row up: the cycle is four lines here and nothing
/// in `karakuri-operation`, which owns the four operators and not the order a
/// pointer walks them in (P-0090).
///
/// **A match rather than an index into [`TONEMAPS`]**, for [`after`]'s reason:
/// a fifth operator does not compile until somebody says what follows it. The
/// price is that the order is written twice — here and in [`TONEMAPS`] — so
/// `tests/look.rs` walks the array through this and asserts they are the same
/// cycle, which is `tests/blend.rs`'s own measurement.
///
/// **`karakuri-cli` has this function over the engine's `TonemapOp`** and in
/// this order. Two crates naming the same order is what P-0090 costs a
/// vocabulary that depends on nothing, and the test above is what stops the
/// two drifting on this side.
pub(crate) fn next_tonemap(tonemap: Tonemap) -> Tonemap {
    match tonemap {
        Tonemap::Clamp => Tonemap::Reinhard,
        Tonemap::Reinhard => Tonemap::Aces,
        Tonemap::Aces => Tonemap::AgX,
        Tonemap::AgX => Tonemap::Clamp,
    }
}

/// **What a point `unit` of the way along the track asks for**, in exposure.
///
/// `2^((unit − ½) · stops)`: the middle of the track is `2^0`, which is
/// **exactly 1.0**, and the two ends are exactly [`EXPOSURE_MIN`] and
/// [`EXPOSURE_MAX`] — the subtraction is exact at a half and `exp2` of a whole
/// number is a power of two with no rounding in it.
///
/// # Logarithmic, and that is not a preference
///
/// Exposure is a ratio: `karakuri-cli`'s step is a *factor* and says why —
/// *"an additive step would be enormous at 0.1 and invisible at 8.0"*. A
/// linear track is that same mistake drawn in space rather than in time: over
/// [`EXPOSURE_MIN`] to [`EXPOSURE_MAX`] unity would sit **0.75 of a pixel**
/// from the left-hand end of [`EXPOSURE_TRACK_W`], so the half of the range
/// that darkens the picture would be one pixel wide and the other half would
/// be the whole control. Equal distances along this track are equal ratios,
/// which is the only reading under which both halves are usable.
pub fn exposure_at(unit: f32) -> f32 {
    ((crate::panel::unit(unit) - 0.5) * EXPOSURE_STOPS).exp2()
}

/// **Where an exposure sits on the track**, on `[0, 1]` — [`exposure_at`]
/// inverted, and clamped to the ends.
///
/// **A value past either end is drawn at that end**, which is deliberate and
/// is stated rather than hidden: `--exposure 200` is accepted unclamped by the
/// flag on purpose, and a fill that ran off the track would be a picture of a
/// value nobody could point at. The figure beside the track is what says the
/// number in that case, which is [`LookRow::value`]'s whole job. Zero and
/// negative — which the flag refuses everywhere — take `log2` to negative
/// infinity and land at the floor rather than anywhere surprising, and a NaN
/// is zero for [`crate::panel::unit`]'s reason.
pub fn unit_of(exposure: f32) -> f32 {
    crate::panel::unit(exposure.log2() / EXPOSURE_STOPS + 0.5)
}

/// **The look controls, derived**: the capsule that names the operator, the
/// track that sets the level, and the figure beside it.
///
/// # Where they sit, and why it is after the arrangement pill
///
/// `docs/manual/console.html`'s `.transport` is a flex row and these are the
/// last two items in it before the `.sep`, immediately after `arr · night ▾`.
/// The page's own argument for the place is that this row is already four
/// groups with a reason each — the clock, the four things that need an audio
/// input, the three that say what this console is *set up* as, and health past
/// the spacer — and a look is none of them: it needs no input, it is no file,
/// and it is played rather than arranged. So it is a fourth group at the end
/// of the left half, and the row then reads in the order a frame does, since
/// the tone map is the one transfer the pipeline ends in, applied immediately
/// before the single encode at the output
/// ([P-0064](../../../../docs/principles/0064-the-pipeline-is-linear-hdr-and-srgb-is-encoded-once-at-final-output.md),
/// [ADR-0037](../../../../docs/adr/0037-tone-mapping-is-a-uniform-and-the-default-is-chosen-by-looking.md)).
///
/// **Nothing already drawn moves**, which was a consequence rather than the
/// reason and is worth reading with its date on it: this said [`arrangement`]
/// was *"still one [`size::TRANSPORT_GAP`] after the bar"*, and it is not, as
/// of 2026-09-08 — [`tracker_group`] draws three of the items the mock puts
/// between the two, and the pill is laid out from the last of them. What has
/// not changed is that this group is measured from the pill's right edge, so
/// where it sits is still one answer read one item further along.
///
/// # The order inside the group, and where the figure goes
///
/// The operator first and its level second, which is `Record::Look`'s own
/// order and the manual's row order — the exposure is *the level going into
/// that transfer*, so it reads as belonging to the thing on its left.
///
/// **The figure is after the track**, and that is the one piece of this layout
/// decided by what a hand does rather than by what the mock draws. It is the
/// only part of the group whose width moves with its value, so putting it last
/// leaves the capsule and the track exactly where they were: a target that
/// walked away from the pointer as it was set would be worst at the moment it
/// was being used most precisely. The capsule ahead of it is held to the
/// widest of the four names for the same reason
/// ([`LookRow::tone`]).
///
/// # No row and no pill means none of this
///
/// `None` wherever [`arrangement`] answers `None` — which is wherever
/// [`transport`] does — and `None` for a console with no engine behind it,
/// which is every test in this crate that does not hand a look in. The place
/// is measured from the pill's right edge, so a console that cannot draw the
/// pill cannot draw what comes after it either; that is one answer to *does
/// this row fit* rather than a second one.
///
/// # What it costs to ask
///
/// **Six galley lookups of its own**: the four operator names, because the
/// capsule is as wide as the widest of them; the `exp` label; and the figure.
///
/// **And it re-derives the row and the pill**, which is the honest cost of
/// being laid out from something else's right edge rather than from the row's:
/// [`transport`] is asked twice more per frame and [`arrangement`] once more,
/// where a paint-as-you-go pass that carried a cursor along the row would ask
/// each once. That is deliberate and it is [`Outputs`]' rule — the derivation
/// that draws a control is the one that hit-tests it, so a control cannot be
/// painted anywhere a press cannot reach — and it is what makes every
/// rectangle here something `tests/look.rs` can ask about without a device.
/// The measured price of the whole panel pass is a median 1518 allocations a
/// frame (`crates/karakuri`'s `WRITTEN_ALLOCS`, taken 2026-08-31); this adds
/// on the order of forty, which is inside the factor of two that file will
/// quote a figure across and is why the figure has not been re-taken here —
/// re-taking one needs a window and three seconds of nobody touching it.
///
/// Paid on a pointer event and on a frame, and a console with no look behind
/// it pays none of it: the `look?` is the first line.
///
/// `layout` must be solved: [`Layout::rect`] refuses to answer from a dirty
/// one.
// **Eight, where clippy's line is seven**, and it went past it when the row
// gained `learn` and `map`. Every one is a thing the *row before this one* is
// laid out from — the tempo, the input, the tracker, the map, the arrangement
// — and this group is the last item in a flex row, so it is laid out from all
// of them. A struct carrying them would be `View` itself with the fields no
// pill in this row reads left out, which is `App::new`'s sentence one crate
// along.
#[allow(clippy::too_many_arguments)]
pub fn look(
    ctx: &egui::Context,
    layout: &karakuri_layout::Layout,
    values: Option<Transport>,
    audio: Option<&AudioIn>,
    tracker: Option<Tracker>,
    // Handed straight to [`arrangement`], which is the item before this one
    // and the only reason this needs it — see that function's own note.
    map: Option<&MapPill>,
    arr: &Arrangement,
    look: Option<Look>,
) -> Option<LookRow> {
    let look = look?;
    // The pill, asked once and for both things it answers: whether the row
    // exists at all, and where the group before this one ended. `transport`
    // answers the first for the whole module and `arrangement` is laid out
    // from it, so this is that one answer read one item further along.
    let pill = arrangement(ctx, layout, values, audio, tracker, map, arr)?;
    let row = transport(ctx, layout, values)?;
    let strip = to_egui(layout.rect(layout.find("transport")?));
    let width = |text: &str| {
        ctx.fonts_mut(|f| {
            f.layout_no_wrap(
                text.to_owned(),
                FontId::new(size::BASE, FontFamily::Proportional),
                Color32::PLACEHOLDER,
            )
            .size()
            .x
        })
    };

    let mid = strip.center().y;
    let span_h = size::BASE * size::LINE;
    // The capsule is the widest of the four whatever it says — see
    // `LookRow::tone`.
    let widest = TONEMAPS
        .iter()
        .map(|op| width(&tone_text(*op)))
        .fold(0.0, f32::max);
    let tone = Rect::from_min_size(
        Pos2::new(
            pill.pill.max.x + size::TRANSPORT_GAP,
            mid - size::PILL_H * 0.5,
        ),
        egui::vec2(size::PILL_PAD_X * 2.0 + widest, size::PILL_H),
    );
    // Centred in a capsule it does not fill, which is what *as wide as the
    // widest* comes to for the other three.
    let words = tone_text(look.tonemap);
    let tone_text = Rect::from_center_size(
        Pos2::new(tone.center().x, mid),
        egui::vec2(width(&words), size::PILL_H),
    );

    let label = Rect::from_min_size(
        Pos2::new(tone.max.x + size::TRANSPORT_GAP, mid - span_h * 0.5),
        egui::vec2(width(EXPOSURE_LABEL), span_h),
    );
    let track = Rect::from_min_size(
        Pos2::new(label.max.x + size::TRIM_GAP, mid - size::FADER_H * 0.5),
        egui::vec2(EXPOSURE_TRACK_W, size::FADER_H),
    );
    let grip = Rect::from_min_max(
        Pos2::new(track.min.x, mid - size::PILL_H * 0.5),
        Pos2::new(track.max.x, mid + size::PILL_H * 0.5),
    );
    let value = Rect::from_min_size(
        Pos2::new(track.max.x + size::TRIM_GAP, mid - span_h * 0.5),
        egui::vec2(width(&exposure_text(look.exposure)), span_h),
    );

    // The rule `arrangement` and `outputs_row` both state: a control that does
    // not fit in the row it is drawn in is no control at all, rather than half
    // of one over the frame readout.
    if !strip.contains_rect(tone)
        || !strip.contains_rect(grip)
        || value.max.x + size::TRANSPORT_GAP > row.frame.min.x
    {
        return None;
    }

    Some(LookRow {
        tone,
        tone_text,
        label,
        track,
        fill: filled(track, Axis::Row, unit_of(look.exposure)),
        grip,
        value,
        values: look,
    })
}

/// **The capsule's words**: `tone · aces`.
///
/// One string rather than two galleys laid end to end, for [`pill_text`]'s
/// reason: the mock writes one run of text and laying it out as one keeps the
/// spaces round the `·` the type's own.
fn tone_text(tonemap: Tonemap) -> String {
    format!("{TONEMAP_LABEL} · {}", tonemap.name())
}

/// **The level, as the Master bay's own row writes it** — `out 1.00`, two
/// places. The two are the same kind of reading and are deliberately written
/// the same way; where they stop being the same *number* is
/// [ADR-0224](../../../../docs/adr/0224-out-and-exposure-are-two-levels-that-multiply-in-different-places.md).
fn exposure_text(exposure: f32) -> String {
    format!("{exposure:.2}")
}

/// **The look controls, painted.**
///
/// Where everything goes is [`look`]'s, so this paints and derives nothing.
/// Term for term from `style.css`:
///
/// - `.pill` — the capsule, which is [`arrangement_into`]'s treatment and this
///   is the same pill three items along. No `▾`, because nothing opens.
/// - the `exp` before the track — `style="color:var(--c-faint)"` in the
///   markup, which is `pal.faint`, and it is `.trim .lbl`'s job one bay up.
/// - `.fader` — `background: var(--c-well)` with
///   `box-shadow: inset 0 0 0 1px var(--c-hair)` and `border-radius: 999px`,
///   which on a 5px box is a capsule.
/// - `.fader b` — `linear-gradient(90deg, var(--c-mint), var(--c-lav))`,
///   through [`gradient`], which is the one place that ramp is drawn and is
///   what the mixer's own faders are painted with.
/// - the figure — `.val`, `pal.text`, *a value*.
///
/// **`.fader s` is deliberately not drawn.** The mock's knob is a handle and
/// this control has none — see [`LookRow::exposure`].
fn look_into(ui: &Ui, pal: &Palette, row: &LookRow) {
    let painter = ui.painter();
    painter.rect_stroke(
        row.tone,
        CornerRadius::same((size::PILL_H * 0.5) as u8),
        Stroke::new(size::HAIRLINE, pal.line),
        StrokeKind::Inside,
    );
    let centred = |rect: Rect, galley: std::sync::Arc<egui::Galley>, colour: Color32| {
        painter.galley(
            Pos2::new(rect.min.x, rect.center().y - galley.size().y * 0.5),
            galley,
            colour,
        );
    };
    let words = tone_text(row.values.tonemap);
    centred(
        row.tone_text,
        painter.layout_no_wrap(
            words,
            FontId::new(size::BASE, FontFamily::Proportional),
            pal.dim,
        ),
        pal.dim,
    );
    centred(
        row.label,
        painter.layout_no_wrap(
            EXPOSURE_LABEL.to_owned(),
            FontId::new(size::BASE, FontFamily::Proportional),
            pal.faint,
        ),
        pal.faint,
    );

    let radius = CornerRadius::same((size::FADER_H * 0.5) as u8);
    painter.rect_filled(row.track, radius, pal.well);
    painter.rect_stroke(
        row.track,
        radius,
        Stroke::new(size::HAIRLINE, pal.hair),
        StrokeKind::Inside,
    );
    gradient(painter, row.fill, Axis::Row, pal.mint, pal.lav, true);

    centred(
        row.value,
        painter.layout_no_wrap(
            exposure_text(row.values.exposure),
            FontId::new(size::BASE, FontFamily::Proportional),
            pal.text,
        ),
        pal.text,
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

/// **The projector window's chip**, and the whole of its name.
///
/// The mock draws `projector · DELL U2720Q`, and the display half is not
/// built: naming which screen a window is on wants a list of displays this
/// program does not read, and a label that carries a monitor's model is a
/// label that changes when the cable does — which is exactly why
/// [`karakuri_operation::Output`] is a closed list and not a string. The chip
/// says what it is; the manual's tooltip says which display it would name.
const PROJECTOR: &str = "projector";

/// **The two plugin sinks the mock draws, and what they say with no plugin
/// loaded.**
///
/// `docs/plugins.md` specifies the process and the handshake and nothing
/// implements them, so there is no manifest to read a sink out of and neither
/// of these is switchable. They are drawn rather than omitted for the mock's
/// own reason, which its `NDI · no plugin` chip already carried: *a control
/// explaining that the thing it would switch is not installed* is a state, and
/// a row that simply did not mention Syphon would leave an operator wondering
/// whether this program has heard of it.
const PLUGIN_SINKS: [&str; 2] = ["Syphon · no plugin", "NDI · no plugin"];

/// **One chip in the Outputs row that is not the program view.**
///
/// The program view is not one of these and the asymmetry is the model rather
/// than a shortcut: its on/off is [`Layout::visible`] on the picture's node —
/// read, never stored ([`Outputs::on`]) — and no other output has a layout
/// node at all. A uniform array would have needed an `Option<NodeId>` on every
/// chip, which would say a projector window might have one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SinkChip {
    /// **What this chip names**, and what a press asks for by name — see
    /// [`karakuri_operation::Output`], which is where the closed list is
    /// argued.
    pub output: Output,
    /// The capsule, dot and name together: what a press has to land in, the
    /// same whole-chip target the program view's is.
    pub chip: Rect,
    /// The `.dot` inside it.
    pub dot: Rect,
    /// Whether this output is on. **`false` until somebody says otherwise** —
    /// this crate has no window and no plugin host, so [`Outputs::told`] is
    /// how the answer arrives.
    pub on: bool,
    /// **Whether there is anything behind it at all.** `false` draws `.absent`
    /// — dim, and not a control — which is the mock's own state for a sink
    /// whose plugin is not loaded.
    pub present: bool,
    /// What is written in it.
    pub name: &'static str,
}

impl SinkChip {
    /// **What a press on this chip asks for**, or `None` where there is
    /// nothing behind it to switch.
    ///
    /// The `on` it names is the state being *asked for* and not the state it
    /// is in: an operation names a destination and never a toggle
    /// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)),
    /// and the chip's toggle is this method.
    pub fn route(&self) -> Option<Operation> {
        self.present.then_some(Operation::RouteFrame {
            output: self.output,
            on: !self.on,
        })
    }

    /// Whether `p` is on this chip. Absent chips answer `false`: a control
    /// that switches nothing does not take a press away from the row it sits
    /// in.
    pub fn hit(&self, p: karakuri_layout::Point) -> bool {
        self.present && self.chip.contains(Pos2::new(p.x, p.y))
    }
}

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
/// [ADR-0161](../../../../docs/adr/0161-solo-remembers-which-region-because-it-cannot-be-derived.md)
/// stored `soloed` because it could not be derived; this can.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Outputs {
    /// Where the word OUTPUTS is painted: its top-left, and the box the
    /// galley fills.
    pub label: Rect,
    /// **The class pill**, beside the word that stands in for a head.
    ///
    /// **This row is the one placement the console had no precedent for.** The
    /// other three openings sit in a bay head, where [`head_pills`] has laid
    /// capsules out since the `solo` pill landed; this row is headless
    /// (ADR-0159, [`Kind::Outputs`]) — no hairline, no pills and no grip — so
    /// there was nothing to add a capsule to. `docs/manual/console.html` says
    /// where it goes and why in as many words: *"This row has no bay head to
    /// put an indicator in — it is headless, like the transport — so the pill
    /// sits beside the word that stands in for one."*
    ///
    /// So it is laid out here, in `.outputs`'s own flex row, between the word
    /// and the first sink: one [`size::OUTPUTS_GAP`] after the label, one
    /// before the chip, [`size::PILL_H`] tall and centred in the row like
    /// everything else in it. **Which is why [`outputs`] takes an opening**:
    /// the two words are not the same width, so where the sink starts depends
    /// on what the pill says.
    ///
    /// [`mcp_pill`] is what reads it back out, so that the four openings are
    /// one type and one probe however differently the two placements are
    /// arrived at.
    pub mcp: Rect,
    /// Whether the class this row's pill opens is open — read out of the
    /// opening handed in, and kept nowhere here.
    pub open: bool,
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
    /// **The rest of the list**, left to right after the program view: the
    /// projector window, then the two plugin sinks.
    ///
    /// See [`SinkChip`] for why they are not one array with the program view
    /// in it. `on` is `false` on all three until [`Outputs::told`] says
    /// otherwise, and the two plugin chips are `present: false` for as long as
    /// there is no manifest to read them out of.
    pub more: [SinkChip; 3],
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

    /// **What a press on the program view's chip asks for**, in the
    /// vocabulary every surface shares.
    ///
    /// [`Outputs::op`] above is how the console *performs* it and this is what
    /// is *asked*, which is one fact and not two: the picture's on and off is
    /// the fold, so the operation names the output and the fold is what
    /// carries it out. Storing a second `on` beside the layout node is what
    /// this refuses —
    /// [ADR-0161](../../../../docs/adr/0161-solo-remembers-which-region-because-it-cannot-be-derived.md)
    /// stored `soloed` because it could not be derived; this can.
    pub fn route(&self) -> Operation {
        Operation::RouteFrame {
            output: Output::Program,
            on: !self.on,
        }
    }

    /// **Say which of the outputs this crate cannot see are on.**
    ///
    /// The picture's state is a layout node and this row reads it; a projector
    /// window is `crates/karakuri`'s and there is nothing in this crate that
    /// could know ([ADR-0156](../../../../docs/adr/0156-the-consoles-arrangement-is-a-tree-this-repository-owns.md)
    /// — the console takes no device). So it arrives the way
    /// [`View::picture`] does: written by whoever owns the window, beside the
    /// frame that draws it.
    ///
    /// A builder rather than a fourth argument to [`outputs`], because the
    /// answer is only needed to *paint* a chip and every caller that
    /// hit-tests one has no opinion about it.
    pub fn told(mut self, projector: bool) -> Outputs {
        self.more[0].on = projector;
        self
    }

    /// Whether `p` is on the program view's control.
    pub fn hit(&self, p: karakuri_layout::Point) -> bool {
        self.sink.contains(Pos2::new(p.x, p.y))
    }

    /// **Which chip `p` is on**, over the whole row — the program view and the
    /// three beside it.
    ///
    /// `None` for a press on the row's ground, on the word, on the class pill
    /// or on a chip with nothing behind it. [`Outputs::hit`] is the same
    /// question asked of the first chip alone, and it stays because
    /// [`crate::input`]'s claim rule is written against one control.
    pub fn chip_at(&self, p: karakuri_layout::Point) -> Option<Output> {
        if self.hit(p) {
            return Some(Output::Program);
        }
        self.more.iter().find(|c| c.hit(p)).map(|c| c.output)
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
pub fn outputs(
    ctx: &egui::Context,
    layout: &karakuri_layout::Layout,
    open: Open,
) -> Option<Outputs> {
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
    // **The class pill's own word**, measured the way every capsule on this
    // console is: `.pill` is as wide as what is in it, and the two states are
    // not the same width.
    let opened = open.holds(Class::InputsAndOutputs);
    let pill = pill_width(ctx, mcp_word(opened));
    // **Every chip's name is measured, not just the first.** A capsule is as
    // wide as what is in it, so where the third starts depends on what the
    // second says — the same rule the class pill above made this row take for
    // its first chip, four times over.
    let widths: [f32; 4] = std::array::from_fn(|i| {
        let word = match i {
            0 => PROGRAM_VIEW,
            1 => PROJECTOR,
            n => PLUGIN_SINKS[n - 2],
        };
        ctx.fonts_mut(|f| {
            f.layout_no_wrap(
                word.to_owned(),
                FontId::new(size::BASE, FontFamily::Proportional),
                Color32::PLACEHOLDER,
            )
            .size()
            .x
        })
    });
    outputs_row(row, label, pill, widths).map(|OutputsRow { label, mcp, chips }| Outputs {
        label,
        mcp,
        open: opened,
        sink: chips[0].0,
        dot: chips[0].1,
        id,
        on: layout.visible(id),
        more: std::array::from_fn(|i| SinkChip {
            output: match i {
                0 => Output::Projector(0),
                n => Output::Plugin(n as u8 - 1),
            },
            chip: chips[i + 1].0,
            dot: chips[i + 1].1,
            on: false,
            present: i == 0,
            name: match i {
                0 => PROJECTOR,
                n => PLUGIN_SINKS[n - 1],
            },
        }),
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
/// **What [`outputs_row`] hands back**: the word, the class pill and the four
/// chips, each with the dot inside it.
///
/// A named type rather than a tuple because the tuple grew a term per chip and
/// stopped being readable at the call site — which is the same reason
/// [`Outputs`] itself is a struct.
struct OutputsRow {
    label: Rect,
    mcp: Rect,
    /// The capsule and its dot, per chip, left to right.
    chips: [(Rect, Rect); 4],
}

fn outputs_row(row: Rect, label_w: f32, pill_w: f32, name_w: [f32; 4]) -> Option<OutputsRow> {
    let mid = row.center().y;
    let label = Rect::from_min_size(
        Pos2::new(row.min.x + size::OUTPUTS_PAD_X, mid - size::HEAD_SIZE * 0.5),
        egui::vec2(label_w, size::HEAD_SIZE),
    );
    // **The class pill, between the word and the first sink**, which is where
    // `docs/manual/console.html` draws it and why: this row has no head to put
    // an indicator in, so it sits beside the word that stands in for one. It is
    // a `.pill` and not a `.sink` — [`size::PILL_H`] and not
    // [`size::SINK_H`] — because it is the same capsule the three bay heads
    // draw, and a control that looked like a sink here would read as a fifth
    // output.
    let mcp = Rect::from_min_size(
        Pos2::new(label.max.x + size::OUTPUTS_GAP, mid - size::PILL_H * 0.5),
        egui::vec2(pill_w, size::PILL_H),
    );
    // **The chips, left to right from the pill, one `.outputs` gap apart.**
    let mut x = mcp.max.x;
    let chips: [(Rect, Rect); 4] = std::array::from_fn(|i| {
        let chip = Rect::from_min_size(
            Pos2::new(x + size::OUTPUTS_GAP, mid - size::SINK_H * 0.5),
            egui::vec2(
                size::SINK_PAD_X * 2.0 + size::SINK_DOT + size::SINK_GAP + name_w[i],
                size::SINK_H,
            ),
        );
        x = chip.max.x;
        let dot = Rect::from_center_size(
            Pos2::new(chip.min.x + size::SINK_PAD_X + size::SINK_DOT * 0.5, mid),
            egui::vec2(size::SINK_DOT, size::SINK_DOT),
        );
        (chip, dot)
    });
    // The same rule [`picture_rect`] states, on the chips rather than on the
    // row because a chip is what is drawn and clicked: a row folded away has
    // a rectangle with no extent in it, and one too narrow for the chips has
    // nowhere to put them. Either way there is no control.
    //
    // **It is the whole list or none of it, and the mock's `flex-wrap: wrap`
    // is still a rule with nothing to apply to.** The row is one
    // [`size::OUTPUTS_H`] tall in an arrangement that cannot grow it, so
    // wrapping is not available; and the four chips, the word and the class
    // pill come to well under the narrowest console this draws, so the case
    // where they do not fit is the same case the row is folded away in.
    // Dropping the tail silently would be the worse answer of the two — a
    // control that is not drawn cannot say it is not drawn — and it is not
    // reachable either.
    match row.contains_rect(chips[3].0) {
        true => Some(OutputsRow { label, mcp, chips }),
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

/// **One laid-out fader, taken hold of at `p`** — or `None` where `p` is not
/// on its knob.
///
/// The inverse of [`fader`], off the same three numbers it laid out: the
/// track's zero end, which is the fill's fixed edge; the travel, which is the
/// length the value rides; and where the knob's centre is now, which is the
/// fill's moving edge. **The offset is the pointer less that centre**, so a
/// press keeps whatever it grabbed at and the value does not jump.
fn grabbed(fader: Fader, knob: Knob, p: Pos2) -> Option<Grab> {
    if !fader.knob.contains(p) {
        return None;
    }
    // Which end is zero is [`filled`]'s rule, read backwards: a row fills from
    // the left and a column from the **bottom**, because a fader stands up.
    let (zero, edge, coord) = match fader.axis {
        Axis::Row => (fader.fill.min.x, fader.fill.max.x, p.x),
        Axis::Column => (fader.fill.max.y, fader.fill.min.y, p.y),
    };
    Grab::new(knob, fader.axis, zero, fader.travel, coord - edge)
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

/// **The drop mark**: `.strip.drop` and `.cell.drop`'s `outline: 2px solid
/// var(--c-text)`, round the one rectangle a carried Set would land on.
///
/// # One function, because it is one mark in two places
///
/// A strip and a deck preview cell wear the same ring — `style.css` gives the
/// two selectors one declaration and says why: the release names a deck, and
/// which of the two rectangles it was let go over is not something the mark
/// has to distinguish. What each caller brings is the target's own corner,
/// [`size::STRIP_RADIUS`] or [`size::PREVIEW_RADIUS`], because the ring is on
/// the rectangle's edge and an edge has the corner it has.
///
/// # `--c-text`, and it is free ink
///
/// Every state this console has is spelled in one of four colours — lavender
/// is the deck the keys are addressed to, pink is live, sun is priming and the
/// star, mint is armed — and the text ink is what a word is drawn in when
/// nothing is being said about it. That is exactly what this mark has to say:
/// it says **where** the release lands and never **whether** it is allowed
/// ([ADR-0265](../../../../docs/adr/0265-a-release-names-the-deck-and-nothing-is-refused.md)).
/// Lavender is ruled out twice over — the selection is already a lavender ring
/// round a strip, and the deck being carried to is usually the deck already
/// selected, so the two would be one mark on one strip in the moment it is
/// read fastest.
///
/// # [`StrokeKind::Outside`], and it is what makes the pair legible
///
/// `.strip.focus` is an inset `box-shadow` and this is an `outline` at
/// `outline-offset: 0`, so a strip that is both selected and under the pointer
/// wears the inner ring and the outer one at once instead of one clobbering
/// the other. **Unclipped**, unlike the selection above: the ink is outside
/// the target's rectangle, and a painter clipped to it would draw nothing at
/// all. It fits — `.mixer-strips` has a `gap: 4px` between strips and the
/// `.previews` grid a `gap: 6px`, against two of ink each side.
fn drop_ring(ui: &Ui, pal: &Palette, at: Rect, radius: f32) {
    ui.painter().rect_stroke(
        at,
        CornerRadius::same(radius as u8),
        Stroke::new(size::DROP_RING, pal.text),
        StrokeKind::Outside,
    );
}

/// **The dashed ring on the bay a key press is addressed to**, the mock's
/// `.wfocus` — `outline: 2px dashed var(--c-sun)` at `outline-offset: 2px`.
///
/// **Dashed rather than a second solid ring**, and the reason is
/// `console.html`'s in its own words: *"the selection is a solid ring and focus
/// is a dashed one, because drawing them the same way would erase which of the
/// two you are looking at."* [`drop_ring`] above is the third mark on this
/// panel and is solid in the text ink; the three are told apart by line and
/// colour, which is why none of them is drawn at another's weight.
///
/// **`egui` has no dashed stroke on a rectangle**, so the four edges are laid
/// out as one closed path and dashed along it. The corner radius the mock sets
/// is not honoured for that reason and is not a loss: a 4px corner on a dash
/// pattern of [`size::WFOCUS_DASH`] is a rounding of one dash.
fn wfocus_into(ui: &Ui, pal: &Palette, at: Rect) {
    let ring = at.expand(size::WFOCUS_OFFSET);
    let path = [
        ring.left_top(),
        ring.right_top(),
        ring.right_bottom(),
        ring.left_bottom(),
        ring.left_top(),
    ];
    let mut dashes = Vec::new();
    egui::Shape::dashed_line_many(
        &path,
        Stroke::new(size::WFOCUS_RING, pal.sun),
        size::WFOCUS_DASH,
        size::WFOCUS_GAP,
        &mut dashes,
    );
    ui.painter().extend(dashes);
}

/// **A folded bay's mark, painted** — the head alone, and the whole of what a
/// folded bay draws.
///
/// `docs/manual/console.html` is the specification, term for term: a
/// `.bay-head` with `border-bottom: 0`, the bay's title in the head's own
/// spaced-out capitals, and [`focus::OPENS`] beside it in the code face at
/// `--c-faint`. The dashed ring is the caller's, so this paints a head and
/// nothing else.
///
/// **It says there is a bay here and that `space` opens it, and nothing
/// else** — not a bay's contents in miniature, and not a count of what is
/// inside. While focus is on a folded bay a digit, `enter` and the arrows
/// decline and say why, so a mark standing in for those would be offering a
/// press that is refused.
fn folded_head_into(ui: &Ui, pal: &Palette, at: Rect, title: &str) {
    card(ui, pal, at);
    let painter = ui.painter().with_clip_rect(at);
    let mid = at.center().y;
    let job = spaced(
        &title.to_uppercase(),
        size::HEAD_SIZE,
        pal.faint,
        size::HEAD_TRACKING,
    );
    let galley = painter.layout_job(job);
    let left = at.min.x + size::HEAD_PAD_X;
    painter.galley(
        Pos2::new(left, mid - galley.size().y * 0.5),
        galley.clone(),
        pal.faint,
    );
    // **The word goes after the title and not at the far end**, because a
    // folded row is as wide as the bay was and the two would part company on
    // a wide panel — the mark is one statement, read left to right.
    let says = painter.layout_no_wrap(
        focus::OPENS.to_owned(),
        FontId::new(size::HEAD_SIZE, FontFamily::Monospace),
        pal.faint,
    );
    let after = match title.is_empty() {
        true => left,
        false => left + galley.size().x + size::HEAD_PAD_X,
    };
    painter.galley(Pos2::new(after, mid - says.size().y * 0.5), says, pal.faint);
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
// The Master bay
// ---------------------------------------------------------------------------

/// **The word at the head of the Master bay**, in the source's own
/// capitalisation for [`Kind::Bay`]'s reason: the mock upper-cases in CSS, and
/// that is done at paint time so the word a reader searches for is the word in
/// the source.
const MASTER_TITLE: &str = "Master";

/// `.master-row`'s first item: the `out` before the track, which is the bay's
/// own word for the level and not the engine's — `Deck::out` is what it moves.
const MASTER_LABEL: &str = "out";

/// **The out row, laid out**: the word, the fader and the figure.
///
/// # One derivation, for [`Outputs`]' reason
///
/// [`View::draw`] paints exactly these rectangles and [`crate::input::claim`]
/// hit-tests exactly this knob. Two copies of the arithmetic is a knob painted
/// where a hand cannot take hold of it.
///
/// # It is the bay's whole body, and the rest of the bay is not built
///
/// `docs/manual/console.html` draws three effects under this row — feedback,
/// bloom and rgb shift — and the word `master` appears nowhere in
/// `karakuri-engine` except at the level this row moves
/// ([ADR-0224](../../../../docs/adr/0224-out-and-exposure-are-two-levels-that-multiply-in-different-places.md)).
/// There is no chain, so there is nothing to draw a chain from: a row of
/// effects over machinery that does not exist is the scaffolding this module's
/// documentation refuses, and the bay's card shows through under this row
/// exactly as it does in every other empty body.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MasterRow {
    /// The faint `out` at the head of the row.
    pub label: Rect,
    /// **The fader**: `.fader`'s 5px well lying down, what the level fills of
    /// it, and the knob centred on the fill's moving edge.
    pub fader: Fader,
    /// **`1.00`, at the far end of the row and in a box that does not move.**
    ///
    /// As wide as the widest reading this control can ask for rather than as
    /// wide as the one it is showing, which is [`LookRow::tone`]'s rule met by
    /// a figure instead of by a word: the track between the label and this box
    /// is what is left over, so a figure that changed width as it was dragged
    /// would take the track — and the knob on it — with it.
    pub value: Rect,
    /// **The value these rectangles were measured from**, carried for
    /// [`LookRow::values`]' reason: whoever measured the type and whoever
    /// paints it are one statement.
    pub out: f32,
    /// **The master chain's three rows**, in the chain's own order —
    /// feedback, bloom, rgb shift — and `None` for one there is no room for.
    ///
    /// **A fixed three and not a list**, because the chain is fixed: three
    /// built-in passes, all of them loaded, in that order
    /// ([ADR-0317](../../../../docs/adr/0317-the-master-chain-is-three-fixed-passes-and-feedback-reads-either-cut.md)).
    /// A `Vec` here would be a claim that the count can change, which is the
    /// `+ add` row's question and is not this one's.
    ///
    /// A row drops out from the bottom up when the bay is short, on
    /// [`mixer::strips_row`]'s rule: the arrangement's own minimum for this bay keeps
    /// room for the out row and one effect, so a bay at its minimum draws one.
    pub fx: [Option<FxRow>; 3],
}

/// **Which pass of the master chain a row is**, in the chain's order.
///
/// The order is the engine's and is not a preference: feedback reads the
/// previous frame, bloom spreads what is over the knee, and rgb shift is last
/// so the other two are seen through it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fx {
    Feedback,
    Bloom,
    RgbShift,
}

impl Fx {
    /// The three, in the order the bay draws them.
    pub const ALL: [Fx; 3] = [Fx::Feedback, Fx::Bloom, Fx::RgbShift];

    /// **The word the row draws**, which is the mock's own and the operations
    /// page's heading in lower case.
    pub fn name(self) -> &'static str {
        match self {
            Fx::Feedback => "feedback",
            Fx::Bloom => "bloom",
            Fx::RgbShift => "rgb shift",
        }
    }
}

/// **One effect row of the master chain, laid out**: the dot, the word, the
/// feedback row's cut chip, the track and the figure.
///
/// `.fx` in `docs/manual/console.html` — a well with `FX_PAD_X` either side
/// and `FX_PAD_Y` above and below, its items [`size::FX_GAP`] apart, the track
/// taking what is left between the word and the figure exactly as the out
/// row's does.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FxRow {
    /// Which pass, which is what decides the word and the operation.
    pub fx: Fx,
    /// The well the row is drawn in.
    pub well: Rect,
    /// `.fx .dot`: lit when this pass is in the frame.
    pub dot: Rect,
    /// The word.
    pub name: Rect,
    /// **The cut chip, on the feedback row and on neither of the others** —
    /// a `.mini`, the mixer's own blend chip: the same 9px word inside the
    /// same padding, because it is the same thing, one value of a closed list
    /// shown and cycled.
    ///
    /// `None` is *this row has no second parameter*, and it is the shape that
    /// says so: an always-present rectangle nobody draws would be a control
    /// two rows can be pressed on.
    pub cut: Option<Rect>,
    /// The track.
    pub fader: Fader,
    /// The figure, in a box as wide as the widest reading — [`MasterRow::value`]'s
    /// rule and its reason.
    pub value: Rect,
    /// **What this pass is set to**, `[0, 1]` of its own reach. Carried for
    /// [`MasterRow::out`]'s reason.
    pub amount: f32,
    /// Which cut the feedback pass is reading. Carried on every row because
    /// the chip is laid out from it and the operation a drag asks for needs
    /// it; meaningless on the other two, which is why only the feedback row
    /// has a chip to draw it in.
    pub reading: karakuri_operation::Cut,
    /// **Whether this pass is in the frame at all.** An amount of zero is not
    /// a pass multiplying by nothing: no pass is recorded, so the row is dim
    /// and the dot is out.
    pub runs: bool,
}

impl FxRow {
    /// **What a drag on this row asks for.** The amount is the track's
    /// position, and the feedback row's knob carries the cut beside it because
    /// the operation is the whole of what the pass is set to.
    pub fn knob(&self) -> Knob {
        match self.fx {
            Fx::Feedback => Knob::Feedback { cut: self.reading },
            Fx::Bloom => Knob::Bloom,
            Fx::RgbShift => Knob::RgbShift,
        }
    }

    /// **What a press on the cut chip asks for**, or `None` where `p` is not
    /// on one — which is every point of the two rows that have no chip.
    ///
    /// **The next cut and not a step**, which is the difference between the
    /// affordance and the operation: the chip cycles because a surface may,
    /// and what it emits names where the pass is going
    /// (`docs/principles/0090-a-surface-offers-it-never-decides.md`). The
    /// list is two long and the cycle is this crate's arithmetic over it,
    /// exactly as the blend chip's is.
    pub fn chip(&self, p: karakuri_layout::Point) -> Option<Operation> {
        let chip = self.cut?;
        if !chip.contains(Pos2::new(p.x, p.y)) {
            return None;
        }
        let at = karakuri_operation::Cut::ALL
            .iter()
            .position(|c| *c == self.reading)
            .unwrap_or(0);
        let next = karakuri_operation::Cut::ALL[(at + 1) % karakuri_operation::Cut::ALL.len()];
        Some(Operation::SetFeedback {
            params: karakuri_operation::Feedback {
                amount: self.amount * karakuri_operation::Feedback::MAX,
                cut: next,
            },
        })
    }
}

/// **What the master chain is running at**, as the Master bay reads it.
///
/// `karakuri_engine::master::Chain` mirrored into this crate for the reason
/// every mirrored list here is mirrored: the panel depends on the vocabulary
/// and on no engine. Four values, because a row that drew one without the
/// others could not build the operation the whole chain's record is made from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Chain {
    /// `[0, 1]` of `karakuri_operation::Feedback::MAX`, which is the track's
    /// own position — a fader draws a position and the operation carries the
    /// amount.
    pub feedback: f32,
    pub cut: karakuri_operation::Cut,
    pub bloom: f32,
    pub rgb_shift: f32,
}

impl MasterRow {
    /// **What a press at `p` takes hold of**, or `None` where there is nothing
    /// under it that a hand can move.
    ///
    /// # It is the knob, and the track is deliberately not a target
    ///
    /// [`Mixer::grab`]'s rule, and this control is the one on the panel it is
    /// most obviously right for: a master out at 0.3 whose track was clicked
    /// would put the whole programme at 1.0, on stage, because a hand landed
    /// three pixels off a knob.
    ///
    /// **It is not [`LookRow::exposure`]'s rule, and the two do not disagree.**
    /// That control has no handle drawn and no gesture to be mid-way through,
    /// so a press is the whole of it. This one has a handle — the mock draws
    /// the `.fader s` here and deliberately draws none on the exposure track —
    /// and a handle that jumped to the pointer would be a lie about what a
    /// handle is.
    ///
    /// # One derivation, asked twice
    ///
    /// [`crate::input::claim`] asks this and so does the caller that acts on
    /// the press, exactly as [`Mixer::grab`] is. **The value is part of the
    /// geometry**: the knob sits on the fill's moving edge, so where it is
    /// depends on what the deck said this frame, and this is the same reading
    /// the row was laid out from.
    pub fn grab(&self, p: karakuri_layout::Point) -> Option<Grab> {
        let at = Pos2::new(p.x, p.y);
        grabbed(self.fader, Knob::Out, at).or_else(|| {
            self.fx
                .iter()
                .flatten()
                .find_map(|row| grabbed(row.fader, row.knob(), at))
        })
    }

    /// **What a press on the feedback row's cut chip asks for**, or `None`.
    /// The bay's one control that is not a fader — see [`FxRow::chip`].
    pub fn chip(&self, p: karakuri_layout::Point) -> Option<Operation> {
        self.fx.iter().flatten().find_map(|row| row.chip(p))
    }

    /// **Whether `p` is on one of the things here a hand can move**, which is
    /// what [`crate::input::claim`] asks — the four knobs and the cut chip,
    /// and not the tracks under them.
    pub fn owns(&self, p: karakuri_layout::Point) -> bool {
        self.grab(p).is_some() || self.chip(p).is_some()
    }
}

/// **The Master bay's out row, derived**: the word, the fader and the figure.
///
/// # Where it sits
///
/// `docs/manual/console.html`'s `.master-body` is a column inside the bay,
/// under the head, inset by [`size::MASTER_PAD_X`] either side and
/// [`size::MASTER_PAD_TOP`] from the head; `.master-row` is a flex row of
/// three items, [`size::MASTER_GAP`] apart, with the fader taking what is left
/// between the label and the figure. That is the mock term for term, and the
/// arrangement's own minimum for this bay in `lib.rs` is written from the same
/// numbers — *"bay head 27, `.master-body` padding 8 + 10, the out row
/// 16.5"*.
///
/// # The figure's box is fixed and the track is what flexes
///
/// The mock gives the fader `flex: 1` and puts the figure after it, so the
/// track's far end is wherever the figure begins. A figure sized to what it
/// says would therefore move the track *while the track is being dragged*,
/// which is [`look`]'s own argument about the exposure's number met here by a
/// control that has a handle: there the figure could simply go last, here it
/// is between the track and the bay's edge. So the box is as wide as the
/// widest reading this control can ask for.
///
/// **The widest is measured and not assumed**: all ten `d.dd` strings are laid
/// out and the widest of them wins, because whether `0.00` is wider than
/// `1.11` is a fact about whatever font the room is drawn in and not one to
/// take on trust ([`docs/contributing.md`](../../../../docs/contributing.md) §1).
/// Ten cached layouts of four characters, on a pointer event and on a frame.
///
/// **A reading outside `[0, 1]` is the one case it does not cover**, and it is
/// stated rather than guarded: `Deck::set_out` is open above 1.0 and this drag
/// tops out at exactly 1.00, so nothing can put a fifth character in the box
/// today. If something does, the figure is right-aligned and grows back over
/// the track's end rather than out past the bay's padding — which keeps the
/// row inside the card, and is the reason it is right-aligned rather than the
/// reason the box is this wide.
///
/// # None where there is nothing to draw
///
/// `None` for a console with no engine behind it — which is every test in this
/// crate that does not hand a level in — and `None` for a bay with no room for
/// the row, which is [`mixer::strips_row`]'s rule one bay up: folded away, soloed
/// away, or a window too small.
///
/// # What it costs to ask
///
/// **Eleven galley lookups**: the `out` label, and the ten `d.dd` strings the
/// widest is taken over. The ten are what buys a track that does not move
/// under a hand, and they are ten *cached* layouts of four characters. **The
/// reading itself is not among them**, which is the whole of why the box does
/// not move: nothing in this derivation lays out the level. Paid on a pointer
/// event and on a frame, and a console with no level behind it pays none of
/// it: the `out?` is the first line.
///
/// `layout` must be solved: [`Layout::rect`] refuses to answer from a dirty
/// one.
pub fn master(
    ctx: &egui::Context,
    layout: &karakuri_layout::Layout,
    out: Option<f32>,
    chain: Option<Chain>,
) -> Option<MasterRow> {
    let out = out?;
    // Fonts are not valid until `egui` has run a pass, exactly as in `mixer`,
    // `outputs` and `transport`.
    if ctx.cumulative_pass_nr() == 0 {
        return None;
    }
    let region = to_egui(layout.rect(layout.find("master")?));
    let width = |text: &str| {
        ctx.fonts_mut(|f| {
            f.layout_no_wrap(
                text.to_owned(),
                FontId::new(size::BASE, FontFamily::Proportional),
                Color32::PLACEHOLDER,
            )
            .size()
            .x
        })
    };
    let row = Rect::from_min_size(
        Pos2::new(
            region.min.x + size::MASTER_PAD_X,
            region.min.y + size::HEAD_H + size::MASTER_PAD_TOP,
        ),
        egui::vec2(
            region.width() - size::MASTER_PAD_X * 2.0,
            size::MASTER_ROW_H,
        ),
    );
    if row.width() <= 0.0 || !region.contains_rect(row) {
        return None;
    }

    // **The widest `d.dd` there is**, which is every reading this control can
    // ask for and is measured rather than assumed: whether `0.00` is wider
    // than `1.11` is a fact about a font, so all ten are laid out and the
    // widest wins. Nothing here reads the level, which is the point — see the
    // paragraph above.
    let widest = (0..10)
        .map(|d| width(&format!("{d}.{d}{d}")))
        .fold(0.0, f32::max);

    let label = Rect::from_min_size(row.min, egui::vec2(width(MASTER_LABEL), row.height()));
    let value = Rect::from_min_size(
        Pos2::new(row.max.x - widest, row.min.y),
        egui::vec2(widest, row.height()),
    );
    let mid = row.center().y;
    let track = Rect::from_min_max(
        Pos2::new(label.max.x + size::MASTER_GAP, mid - size::FADER_H * 0.5),
        Pos2::new(value.min.x - size::MASTER_GAP, mid + size::FADER_H * 0.5),
    );
    // **A track with no length is no control**, which is `Grab::new`'s own
    // refusal one crate layer down and `strip_box`'s rule one bay up: a bay
    // narrow enough that the word and the figure meet has nothing left to
    // draw a fader in, and half a fader is worse than none.
    if track.width() <= 0.0 {
        return None;
    }

    // **The three effect rows, under the out row and off the same width.**
    // `None` for a chain nothing is behind — a console with no engine draws
    // the out row it was handed a level for and nothing under it — and `None`
    // per row for one the bay is too short to hold.
    let mut fx = [None, None, None];
    if let Some(chain) = chain {
        let mut top = row.max.y + size::MASTER_STACK_GAP;
        for (at, kind) in Fx::ALL.into_iter().enumerate() {
            let well = Rect::from_min_size(
                Pos2::new(row.min.x, top),
                egui::vec2(row.width(), size::FX_H),
            );
            if !region.contains_rect(well) {
                break;
            }
            fx[at] = fx_row(ctx, kind, well, chain, widest, &width);
            top = well.max.y + size::MASTER_STACK_GAP;
        }
    }

    Some(MasterRow {
        label,
        // `.fader b` fills its 5px track edge to edge, so there is no inset —
        // the trim's arrangement, and not `.vfader`'s.
        fader: fader(
            track,
            Axis::Row,
            out,
            0.0,
            egui::vec2(size::FADER_KNOB_W, size::FADER_KNOB_H),
        ),
        value,
        out,
        fx,
    })
}

/// **One effect row, laid out inside `well`.**
///
/// The out row's arrangement one line down and inside a padded well: the dot,
/// the word, the feedback row's chip, the track taking what is left, and the
/// figure in a box that does not move. `widest` is the out row's own
/// measurement of the widest `d.dd`, passed in rather than taken again —
/// the figures are the same shape and one measurement is what keeps the two
/// rows' boxes the same width.
fn fx_row(
    ctx: &egui::Context,
    fx: Fx,
    well: Rect,
    chain: Chain,
    widest: f32,
    width: &dyn Fn(&str) -> f32,
) -> Option<FxRow> {
    // The chip's word is 9px where everything else on the row is `BASE`, so
    // this row needs the one measurement `master`'s own closure cannot give it.
    let width_at = |text: &str, size: f32| {
        ctx.fonts_mut(|f| {
            f.layout_no_wrap(
                text.to_owned(),
                FontId::new(size, FontFamily::Proportional),
                Color32::PLACEHOLDER,
            )
            .size()
            .x
        })
    };
    let amount = match fx {
        Fx::Feedback => chain.feedback,
        Fx::Bloom => chain.bloom,
        Fx::RgbShift => chain.rgb_shift,
    };
    let inner = Rect::from_min_max(
        Pos2::new(well.min.x + size::FX_PAD_X, well.min.y + size::FX_PAD_Y),
        Pos2::new(well.max.x - size::FX_PAD_X, well.max.y - size::FX_PAD_Y),
    );
    if inner.width() <= 0.0 {
        return None;
    }
    let mid = inner.center().y;
    let dot = Rect::from_center_size(
        Pos2::new(inner.min.x + size::FX_DOT * 0.5, mid),
        egui::vec2(size::FX_DOT, size::FX_DOT),
    );
    let name = Rect::from_min_size(
        Pos2::new(dot.max.x + size::FX_GAP, inner.min.y),
        egui::vec2(width(fx.name()), inner.height()),
    );
    // **The chip is on the feedback row and on neither of the others**, and it
    // is as wide as the wider of the two words rather than as wide as the one
    // it is showing — the figure's rule at the other end of the row, for the
    // same reason: a chip that changed width when it was pressed would move
    // the track it sits beside.
    let cut = (fx == Fx::Feedback).then(|| {
        // **A `.mini`, the mixer's own blend chip** — the same 9px word inside
        // the same padding and the same border, because it is the same thing:
        // one value of a closed list, shown and cycled.
        let word = karakuri_operation::Cut::ALL
            .iter()
            .map(|c| width_at(c.name(), size::MINI_SIZE))
            .fold(0.0, f32::max);
        let chip = word + size::MINI_PAD_X * 2.0 + size::HAIRLINE * 2.0;
        Rect::from_center_size(
            Pos2::new(name.max.x + size::FX_GAP + chip * 0.5, mid),
            egui::vec2(chip, size::MINI_H),
        )
    });
    let value = Rect::from_min_size(
        Pos2::new(inner.max.x - widest, inner.min.y),
        egui::vec2(widest, inner.height()),
    );
    let after = cut.map_or(name.max.x, |c| c.max.x);
    let track = Rect::from_min_max(
        Pos2::new(after + size::FX_GAP, mid - size::FADER_H * 0.5),
        Pos2::new(value.min.x - size::FX_GAP, mid + size::FADER_H * 0.5),
    );
    // **A track with no length is no control** — [`master`]'s own refusal, and
    // a row without one is not drawn at all rather than drawn half.
    if track.width() <= 0.0 {
        return None;
    }
    Some(FxRow {
        fx,
        well,
        dot,
        name,
        cut,
        fader: fader(
            track,
            Axis::Row,
            amount,
            0.0,
            egui::vec2(size::FADER_KNOB_W, size::FADER_KNOB_H),
        ),
        value,
        amount,
        reading: chain.cut,
        // **The one thing a row says that is not a number**: an amount of zero
        // is the pass not being recorded at all, so the row is dim and the dot
        // is out. See `karakuri_engine::master`.
        runs: amount > 0.0,
    })
}

/// **The level, as the mock's `.val` writes it** — `1.00`, two places, and the
/// same string the transport row's exposure is written with. The two are the
/// same kind of reading and deliberately read the same way; where they stop
/// being the same *number* is ADR-0224.
fn master_text(out: f32) -> String {
    format!("{out:.2}")
}

/// **The out row, painted.**
///
/// Where everything goes is [`master`]'s, so this paints and derives nothing.
/// Term for term from `style.css`:
///
/// - the `out` before the track — `style="color:var(--c-faint)"` in the
///   markup, which is `pal.faint`, and it is `.trim .lbl`'s job one bay up.
/// - `.fader`, `.fader b` and `.fader s` — [`fader_into`], which is the one
///   place a knob, a well and a fill are drawn and is what the mixer's own
///   trim is painted with. **Not live and never reaching**: the pink glow is a
///   slot on air and this level belongs to no slot, and a scheduled move is
///   per slot too — `Deck::set_out` takes no `cancel` because nothing can be
///   moving it (ADR-0224).
/// - the figure — `.val`, `pal.text`, *a value*.
fn master_into(ui: &Ui, pal: &Palette, row: &MasterRow) {
    let painter = ui.painter();
    let centred = |rect: Rect, galley: std::sync::Arc<egui::Galley>, colour: Color32| {
        painter.galley(
            Pos2::new(rect.min.x, rect.center().y - galley.size().y * 0.5),
            galley,
            colour,
        );
    };
    centred(
        row.label,
        painter.layout_no_wrap(
            MASTER_LABEL.to_owned(),
            FontId::new(size::BASE, FontFamily::Proportional),
            pal.faint,
        ),
        pal.faint,
    );
    fader_into(painter, pal, row.fader, false, None);
    // **Right-aligned in a box that does not move**, so the figure ends at the
    // bay's padding whatever it says — which is what makes the box's width the
    // widest reading rather than this one's.
    let words = master_text(row.out);
    let galley = painter.layout_no_wrap(
        words,
        FontId::new(size::BASE, FontFamily::Proportional),
        pal.text,
    );
    painter.galley(
        Pos2::new(
            row.value.max.x - galley.size().x,
            row.value.center().y - galley.size().y * 0.5,
        ),
        galley,
        pal.text,
    );
    for fx in row.fx.iter().flatten() {
        fx_into(ui, pal, fx);
    }
}

/// **One effect row, painted.** Term for term from `style.css`:
///
/// - `.fx` — a `--c-well` recess with an 8px radius.
/// - `.fx.sel` — `--c-text` and an inset mint ring, which on this bay means
///   **this pass is in the frame**: an amount above zero, so it is recorded
///   and it costs its passes. `.fx.off` is `--c-faint`, which is the same
///   sentence the other way round.
/// - `.fx .dot` — mint with a glow when the pass runs, `--c-faint` and no glow
///   when it does not.
/// - the cut chip — `.mini`, the mixer's own blend chip, on the feedback row
///   alone.
/// - `.fx .amt` — `--c-text`, right-aligned in a box that does not move.
fn fx_into(ui: &Ui, pal: &Palette, row: &FxRow) {
    let painter = ui.painter();
    painter.rect_filled(row.well, CornerRadius::same(size::FX_RADIUS), pal.well);
    if row.runs {
        painter.rect_stroke(
            row.well,
            CornerRadius::same(size::FX_RADIUS),
            Stroke::new(size::HAIRLINE, pal.mint),
            StrokeKind::Inside,
        );
    }
    let ink = if row.runs { pal.text } else { pal.faint };
    painter.circle_filled(
        row.dot.center(),
        size::FX_DOT * 0.5,
        if row.runs { pal.mint } else { pal.faint },
    );
    let word = |rect: Rect, text: &str, colour: Color32| {
        let galley = painter.layout_no_wrap(
            text.to_owned(),
            FontId::new(size::BASE, FontFamily::Proportional),
            colour,
        );
        painter.galley(
            Pos2::new(rect.min.x, rect.center().y - galley.size().y * 0.5),
            galley,
            colour,
        );
    };
    word(row.name, row.fx.name(), ink);
    // **`.mini.sel`**, which is the mixer's blend chip exactly: a lavender
    // wash and a lavender word, because what it says is *this is the one
    // chosen* and lavender is this console's ink for a selection.
    if let Some(chip) = row.cut {
        mini_into(painter, pal, chip, true, |painter, colour| {
            let galley = painter.layout_no_wrap(
                row.reading.name().to_owned(),
                FontId::new(size::MINI_SIZE, FontFamily::Proportional),
                colour,
            );
            painter.galley(
                Pos2::new(
                    chip.center().x - galley.size().x * 0.5,
                    chip.center().y - galley.size().y * 0.5,
                ),
                galley,
                colour,
            );
        });
    }
    fader_into(painter, pal, row.fader, false, None);
    let galley = painter.layout_no_wrap(
        master_text(row.amount),
        FontId::new(size::BASE, FontFamily::Proportional),
        ink,
    );
    painter.galley(
        Pos2::new(
            row.value.max.x - galley.size().x,
            row.value.center().y - galley.size().y * 0.5,
        ),
        galley,
        ink,
    );
}

// ---------------------------------------------------------------------------
// The Sequencer bay
// ---------------------------------------------------------------------------

/// **The word at the head of the Sequencer bay**, in the source's own
/// capitalisation for [`Kind::Bay`]'s reason: the mock upper-cases in CSS, and
/// that is done at paint time so the word a reader searches for is the word in
/// the source.
const SEQUENCER_TITLE: &str = "Sequencer";

/// **The mark a lane's label carries after the deck's letter**: `▮` for a
/// fader and `∿` for a parameter, which are the two glyphs
/// `docs/manual/console.html` draws on the four lane labels.
///
/// A `match` over the target and not a field on the reading, for
/// `karakuri_operation::Sync::name`'s reason one crate down: a target added to
/// that enum does not compile until it has a mark to be drawn with.
fn lane_mark(target: &LaneTarget) -> &'static str {
    match target {
        LaneTarget::Fader { .. } => "\u{25AE}",
        LaneTarget::Param { .. } => "\u{223F}",
    }
}

/// **What one lane's row reads**: the deck's letter and the mark for what it
/// drives — `A ▮`, `B ∿` — which is the mock's own label word for word.
fn lane_label(target: &LaneTarget) -> String {
    let letter = DECK_LETTERS
        .get(usize::from(target.deck()))
        .copied()
        // A deck past the four is a caller's error and not a state, and is
        // drawn rather than panicked for [`deck_letter`]'s reason one bay
        // along: a label is a readout and a readout does not stop a frame.
        .unwrap_or("?");
    format!("{letter} {}", lane_mark(target))
}

/// **What the sequencer bay reads this frame**: the armed pattern, which bank
/// it is, and where the playhead was left.
///
/// **A pattern and not a copy of one, taken apart.** The bay draws the mode,
/// the lanes, what each drives, its steps and its mute, which is the whole of
/// what a pattern is — so a reading with a field per drawn thing would be a
/// second spelling of `karakuri_pattern::Pattern` that could disagree with it.
/// This crate holds no pattern and applies nothing to one (ADR-0156); the host
/// writes this per frame beside the frame it is about, which is
/// [`View::mixer`]'s seam.
///
/// **`step` comes from the poll and not from `beats`.** Where the playhead is
/// is what the *producer* last answered — `karakuri_pattern::Playhead` — and
/// deriving it here from the transport's beats would be a second derivation
/// that could name a step the sequencer never emitted (P-0087). `None` before
/// the first poll, which draws no column: a bay that has not been polled is
/// not a bay at step zero.
#[derive(Debug, Clone, PartialEq)]
pub struct Sequenced {
    /// The armed pattern, as the host read it this frame.
    pub pattern: karakuri_pattern::Pattern,
    /// Which of `karakuri_pattern::BANKS` is armed.
    pub bank: usize,
    /// Where the poll last put the playhead.
    pub step: Option<usize>,
}

/// **One item of the `+ lane` chooser**: a target a lane may be pointed at, and
/// the words drawn on it.
///
/// **The words are the lane label the pick will make, plus what it is.** A
/// fader item reads `A ▮ fader` and a parameter item `B ∿ L2:0 twist`, so the
/// row that appears after the press reads as the item that was picked — the
/// deck's letter and [`lane_mark`], which is [`lane_label`]'s own derivation
/// asked one control earlier.
#[derive(Debug, Clone, PartialEq)]
pub struct LaneChoice {
    /// What a pick points the lane at — the payload of
    /// `Operation::PointLane`, whole.
    pub target: LaneTarget,
    /// The words on the item.
    pub words: String,
}

/// **What the `+ lane` chooser offers this frame**, read off the [`View`] once
/// and handed in — [`Target`]'s shape one bay along, and for its reason: the
/// item that is painted and the item a press lands on are one derivation of one
/// reading.
///
/// # What is in the list, and why it is one deck's parameters and every deck's fader
///
/// A lane's target is `Fader { deck }` or `Param { deck, param }`
/// ([ADR-0321](../../../../docs/adr/0321-a-lanes-target-is-an-operation-with-its-value-elided.md)),
/// so the list is the faders of every deck the mixer draws a strip for —
/// [`View::select`]'s own count read a fourth time — and the published controls
/// of **one** deck: the Library bay's load pulldown's
/// ([`View::target_deck`], ADR-0305).
///
/// **That mark and not a second one.** It is the console's one pointer meaning
/// *a deck named without moving the keys*, which is exactly what pointing a
/// lane wants — a lane on deck C while deck A is playing — and a chooser of its
/// own in this bay would be a fourth pointer on a panel that already explains
/// three (ADR-0305's counting argument). Listing every deck's keys instead
/// would put the same key in the list once per deck, so the operator would pick
/// a deck by reading a list four times as long rather than by a control.
/// [ADR-0327](../../../../docs/adr/0327-the-lane-chooser-lists-one-decks-keys-and-the-bank-pills-are-the-four-banks.md).
///
/// **What the console does not hold, it does not offer.** The parameters are
/// [`View::inspector`]'s, which is written when a Set lands, and the inspector
/// holds [`PANES`] panes — so a target deck no pane is pointed at contributes
/// no parameters and the list is its faders alone. That is the reading's own
/// limit rather than this control's, and it is written down in
/// `docs/manual/console.html`'s `+ lane` tip.
#[derive(Debug, Clone, PartialEq)]
pub struct Choices {
    /// Every target on offer: the faders first, then the parameters.
    pub items: Vec<LaneChoice>,
    /// **How many of the leading items are faders**, which is where the card's
    /// separator goes — [`RowMenu::rule`]'s band between the loads and the
    /// send, reached by the same argument: two kinds of item, and the rule says
    /// so.
    pub faders: usize,
    /// **Whether the card is down** — the console's own state, like
    /// [`Target::open`]. See [`View::lane_open`].
    pub open: bool,
}

impl Choices {
    /// **Nothing to point at**, which is a console with no mixer and no pane —
    /// every test in this crate that does not hand one in.
    pub fn none() -> Choices {
        Choices {
            items: Vec::new(),
            faders: 0,
            open: false,
        }
    }
}

/// **One lane's row**: the label a press mutes it by, and the cells a press
/// sets a step by.
#[derive(Debug, Clone, PartialEq)]
pub struct SeqRow {
    /// **The label, and it is a control**: *"Click to mute the lane and keep
    /// the pattern"*, which is where rule 02's take-back sits for a lane.
    pub label: Rect,
    /// **One rectangle per step of the mode**, so there are sixteen of these
    /// at a sixteenth and eight at an eighth: the row keeps its width and the
    /// cells halve in the finer one (ADR-0306).
    pub cells: Vec<Rect>,
    /// What the row draws from: the lane's own label, which slots are on, and
    /// whether it is muted.
    pub words: String,
    pub muted: bool,
    /// **Whether each drawn cell is on**, in the cells' order — the mode's
    /// reading of the lane's sixteen slots, taken where the row is laid out so
    /// the paint and the press cannot disagree about which slot a cell is.
    pub on: Vec<bool>,
    /// **Which stored slot each drawn cell is**, which is what a press sends:
    /// the identity at a sixteenth and `2k` at an eighth, so
    /// `Operation::SetStep` carries a slot and never a step
    /// (`karakuri_operation::StepMode::slot_of`).
    pub slots: Vec<usize>,
}

/// **The Sequencer bay, laid out**: the head's mode pill and step readout, the
/// ruler, the playhead column and a row per lane.
///
/// # What is drawn and what is not
///
/// [ADR-0200](../../../../docs/adr/0200-a-bays-first-pass-draws-the-values-that-exist-and-omits-the-rest.md)
/// is satisfied here for the first time in this bay, and ADR-0222 said why it
/// could not be before: every part of the drawing now reads a value that
/// exists, because a pattern exists. **One thing the mock draws is still not
/// drawn**: the foot's sentence, which is a readout and not a control.
///
/// The bank pills and the `+ lane` pill landed on 2026-09-09. The pills are
/// [`Head::banks`] — the head machinery gained one field and every other head's
/// [`HeadWords`] is what it was — and they are laid out here a second time
/// rather than copied ([`bank_capsules`]), which is [`program_head`]'s
/// arrangement: the capsule an operator sees and the capsule a press lands on
/// are one derivation. **Four pills and no `+`**: with four fixed banks the
/// mock's `+` is `Operation::SelectPattern` at an empty bank, which is what a
/// press on `seq 3` already is
/// ([ADR-0327](../../../../docs/adr/0327-the-lane-chooser-lists-one-decks-keys-and-the-bank-pills-are-the-four-banks.md)).
///
/// # The cells are the row divided by the count, and the count follows the mode
///
/// `.seq-lane` is `repeat(16, 1fr)` with a [`size::SEQ_CELL_GAP`] between, so
/// a cell is as wide as what is left of the row after the gaps — and at an
/// eighth there are eight of them over the same width, which is the mock's
/// *"the row keeps its width, so the cells halve in the finer one"* read the
/// other way round.
pub fn sequencer(
    ctx: &egui::Context,
    layout: &karakuri_layout::Layout,
    reading: Option<&Sequenced>,
    choices: &Choices,
) -> Option<Sequencer> {
    let reading = reading?;
    // Fonts are not valid until `egui` has run a pass, exactly as in `mixer`,
    // `outputs` and `transport`.
    if ctx.cumulative_pass_nr() == 0 {
        return None;
    }
    let region = to_egui(layout.rect(layout.find("sequencer")?));
    let left = region.min.x + size::SEQ_PAD_X;
    let right = region.max.x - size::SEQ_PAD_X;
    if right <= left {
        return None;
    }
    let mut y = region.min.y + size::HEAD_H + size::SEQ_PAD_TOP;

    // -- the bay head's bank pills, laid out where they are painted ---------
    // **The same head [`bay_head`] paints**, asked a second time rather than
    // copied ([`head_capsule`]'s arrangement), so the capsule an operator
    // presses is the capsule that was drawn.
    //
    // **`Open::CLOSED` and no opening threaded here**: this head opens no class
    // — `class_at("sequencer")` is `None`, which
    // `docs/manual/console.html` states of this bay in as many words — so
    // nothing in it moves with an opening. `a_sequencer_head_opens_no_class`
    // is what holds that rather than this comment.
    let banks = crate::view::region("sequencer")
        .and_then(head_of)
        .map(|head| head.with_banks(reading.bank))
        .map(|head| bank_capsules(ctx, region, &head, Open::CLOSED))
        .unwrap_or_default();

    // -- the foot's `+ lane`, measured before the rows so they clear it -----
    // `.seq-foot` sits on the bottom edge of `.seq`, so the pill is against
    // the bay's own padding at both the right and the bottom — the same two
    // numbers the head and the rows are inset by.
    let add_w = pill_width(ctx, ADD_LANE);
    let add = Rect::from_min_size(
        Pos2::new(right - add_w, region.max.y - size::SEQ_PAD_X - size::PILL_H),
        egui::vec2(add_w, size::PILL_H),
    );
    if add.min.x < left {
        return None;
    }

    // -- the head: the mode pill, then the step readout ---------------------
    let mode = reading.pattern.mode();
    let pill_w = pill_width(ctx, mode.name());
    let mode_pill = Rect::from_min_size(Pos2::new(left, y), egui::vec2(pill_w, size::PILL_H));
    let words = step_words(reading.step, mode);
    let step_w = ctx.fonts_mut(|f| {
        f.layout_no_wrap(
            words.clone(),
            FontId::new(size::BASE, FontFamily::Proportional),
            Color32::PLACEHOLDER,
        )
        .size()
        .x
    });
    let step = Rect::from_min_size(
        Pos2::new(mode_pill.max.x + size::SEQ_HEAD_GAP, y),
        egui::vec2(step_w, size::PILL_H),
    );
    if step.max.x > right {
        return None;
    }
    y = mode_pill.max.y + size::SEQ_STACK_GAP;

    // -- the ruler, inset so its numbers stand over the cells ---------------
    let ruler = Rect::from_min_max(
        Pos2::new(left + size::SEQ_RULER_INSET, y),
        Pos2::new(right, y + size::SEQ_RULER_SIZE * size::LINE),
    );
    if ruler.width() <= 0.0 {
        return None;
    }
    y = ruler.max.y + size::SEQ_STACK_GAP;

    // -- the rows, and the lane track every cell is measured in -------------
    let track_x = left + size::SEQ_LABEL_W + size::SEQ_ROW_GAP;
    if track_x >= right {
        return None;
    }
    let count = mode.count();
    let gaps = size::SEQ_CELL_GAP * (count as f32 - 1.0);
    let cell_w = (right - track_x - gaps) / count as f32;
    // **A cell with no width is no control**, which is `master`'s own refusal
    // one bay up: a bay narrow enough that the label and the track meet has
    // nothing to draw sixteen cells in, and half a grid is worse than none.
    if cell_w <= 0.0 {
        return None;
    }
    let cell_at = |row_top: f32, at: usize| {
        Rect::from_min_size(
            Pos2::new(track_x + (cell_w + size::SEQ_CELL_GAP) * at as f32, row_top),
            egui::vec2(cell_w, size::SEQ_CELL_H),
        )
    };
    let body_top = y;
    let mut rows = Vec::with_capacity(reading.pattern.lanes().len());
    for lane in reading.pattern.lanes() {
        let label = Rect::from_min_size(
            Pos2::new(left, y),
            egui::vec2(size::SEQ_LABEL_W, size::SEQ_CELL_H),
        );
        let cells: Vec<Rect> = (0..count).map(|at| cell_at(y, at)).collect();
        rows.push(SeqRow {
            label,
            cells,
            words: lane_label(lane.target()),
            muted: lane.muted(),
            on: (0..count).map(|at| lane.step_on(at, mode)).collect(),
            slots: (0..count).map(|at| mode.slot_of(at)).collect(),
        });
        y += size::SEQ_CELL_H + size::SEQ_BODY_GAP;
    }
    // **The bay is clipped rather than half drawn.** A bay too short for the
    // rows it has is `master`'s refusal again: what would be drawn is a lane
    // over the card's own edge, and a cell a press could not reach.
    let bottom = match rows.is_empty() {
        true => body_top,
        false => y - size::SEQ_BODY_GAP,
    };
    // **And the rows clear the foot**, which is the same refusal read against
    // the pill instead of against the card's edge: the `+ lane` press is a
    // control and a lane drawn over it is a control a hand cannot reach.
    if bottom > add.min.y - size::SEQ_STACK_GAP {
        return None;
    }
    // **The playhead is one column over every row**, which is `.seq-play`'s
    // `position: absolute; inset: 0`: it is the body's height and the cell's
    // width, and it is drawn under nothing — `pointer-events: none`, so it
    // claims no press.
    let playhead = reading
        .step
        .filter(|_| !rows.is_empty())
        .map(|step| step.min(count.saturating_sub(1)))
        .map(|step| {
            Rect::from_min_max(
                Pos2::new(cell_at(body_top, step).min.x, body_top),
                Pos2::new(cell_at(body_top, step).max.x, bottom),
            )
        });
    Some(Sequencer {
        mode_pill,
        mode,
        step,
        step_words: words,
        ruler,
        playhead,
        rows,
        bank: reading.bank,
        banks,
        add,
        card: lane_card(ctx, to_egui(layout.viewport()), add, choices),
    })
}

/// **The `+ lane` chooser's card, or `None` while it is up** — and `None` for a
/// chooser with nothing in it, which is a console the mixer draws no strip for.
///
/// # It hangs up off the pill, where a row's menu hangs down off a row
///
/// [`Load::list`]'s rule, and the same one: this pill is in the **foot** of a
/// bay, so what is under it is the bay's own edge and the card stands on the
/// pill's top edge, one [`size::PILL_GAP`] clear of it, over this bay's rows.
/// It is held inside the viewport, so a Sequencer bay at the bottom of a short
/// window draws the card over the bays above rather than off the top.
///
/// **The rows are not counted against the room**, which is that method's other
/// clause: the list is at most [`DECKS`] faders and however many controls one
/// deck published, and a window too short to hold it is a window with no
/// transport row in it either.
///
/// **The items are as wide as the words in them**, which is `egui`'s to answer
/// — [`View::menu`]'s reason one bay along, and why this takes the context.
fn lane_card(
    ctx: &egui::Context,
    viewport: Rect,
    add: Rect,
    choices: &Choices,
) -> Option<LaneCard> {
    if !choices.open || choices.items.is_empty() {
        return None;
    }
    let width = |text: &str| {
        ctx.fonts_mut(|f| {
            f.layout_no_wrap(
                text.to_owned(),
                FontId::new(size::BASE, FontFamily::Proportional),
                Color32::PLACEHOLDER,
            )
            .size()
            .x
        })
    };
    let widest = choices
        .items
        .iter()
        .map(|item| width(&item.words))
        .fold(size::ROW_MENU_MIN_W, f32::max);
    // **The rule is drawn only where it divides two things**, which is what
    // makes it a separator rather than a line: a list of faders alone and a
    // list of parameters alone each have one kind in them.
    let ruled = choices.faders > 0 && choices.faders < choices.items.len();
    let rule_h = match ruled {
        true => size::ROW_MENU_RULE_H,
        false => 0.0,
    };
    let height = size::LIB_LIST_PAD * 2.0 + size::LIB_ROW_H * choices.items.len() as f32 + rule_h;
    // **Held inside the console**, which is the two Library cards' own rule:
    // `held_inside` clamps the left edge, so a card wider than the room it
    // stands in comes back into the window rather than off it.
    let card = held_inside(
        &viewport,
        add.max.x - (widest + (size::LIB_ROW_PAD_X + size::LIB_LIST_PAD) * 2.0),
        add.min.y - size::PILL_GAP - height,
        widest + (size::LIB_ROW_PAD_X + size::LIB_LIST_PAD) * 2.0,
        height,
    );
    Some(LaneCard {
        card,
        faders: choices.faders,
        items: choices.items.len(),
        ruled,
    })
}

/// **What the head's readout says** — `step 6 of 16`, counting from one as the
/// ruler does, and `step — of 16` before the first poll.
///
/// **The count is in it and the mock's is not.** `docs/manual/console.html`
/// draws `step 6` and says *"Step 6 of sixteen"* in its tip; the count follows
/// the mode now and the pill beside it can be pressed, so a readout that said
/// only `6` would leave a hand that had just halved the grid reading the same
/// figure against a different bar.
fn step_words(step: Option<usize>, mode: StepMode) -> String {
    match step {
        Some(step) => format!("step {} of {}", step + 1, mode.count()),
        None => format!("step \u{2014} of {}", mode.count()),
    }
}

/// **The Sequencer bay's controls, as rectangles to press.**
///
/// Everything here is derived from the pattern the host handed in this frame,
/// so the cell that is painted is the cell that is pressed — the rule every
/// other bay in this module follows, and the one that makes
/// [`crate::input::claim`] and the press handler ask the same question.
#[derive(Debug, Clone, PartialEq)]
pub struct Sequencer {
    /// **The mode pill**, reading `1/16` or `1/8`. A press asks for the other
    /// of the two by naming it — a state and never a flip.
    pub mode_pill: Rect,
    /// The mode those rectangles were laid out from, carried for
    /// [`MasterRow::out`]'s reason: whoever measured the type and whoever
    /// paints it are one statement.
    pub mode: StepMode,
    /// **The step readout**, which is a readout: there is nothing here to
    /// press, and a hand that wants a pattern to begin somewhere else has no
    /// control in this bay for it.
    pub step: Rect,
    /// The words in it, measured once and painted from the same string.
    pub step_words: String,
    /// **The ruler**, which is a readout too: four numbers over the cells.
    pub ruler: Rect,
    /// **The playhead's column**, or `None` for a bay nothing has polled and
    /// for a pattern with no lanes to stand over.
    pub playhead: Option<Rect>,
    /// One per lane, in the order the pattern draws them.
    pub rows: Vec<SeqRow>,
    /// Which bank these rows are, carried so a caller's operation names the
    /// bank it acted on rather than implying the armed one
    /// (`Operation::SelectDeck`'s rule).
    pub bank: usize,
    /// **The four bank pills in the bay head**, in bank order — the same
    /// capsules [`bay_head`] paints, laid out a second time here
    /// ([`bank_capsules`]).
    ///
    /// **Empty for a bay too short to hold its own head**, which is a head with
    /// no capsule to press; short of four it never is, because renumbering the
    /// ones that fit would put `seq 2`'s press on `seq 1`.
    pub banks: Vec<Rect>,
    /// **The foot's `+ lane` pill.** A press puts [`LaneCard`] down; it emits
    /// nothing on its own, because what is being added is *what the lane
    /// drives* and a lane with nothing to point at emits nothing.
    pub add: Rect,
    /// **The chooser's card, or `None` while it is up.**
    pub card: Option<LaneCard>,
}

/// **The `+ lane` chooser's card, laid out** — [`RowMenu`]'s shape one bay
/// along, and the same mechanism (ADR-0311): a card of items with a separator
/// in it, hanging off the control that opened it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LaneCard {
    /// The card itself.
    pub card: Rect,
    /// **How many of the items are faders**, which is where the rule goes.
    pub faders: usize,
    /// How many items there are altogether — [`Choices::items`]' length.
    pub items: usize,
    /// **Whether the separator is drawn**, which is whether there is anything
    /// on both sides of it.
    pub ruled: bool,
}

impl LaneCard {
    /// **Where one item is**, from the top of the card — the faders stacked
    /// with no gap, then the band, then the parameters. [`RowMenu::load`]'s own
    /// reading, with the rule in the middle rather than at the end.
    ///
    /// Panics on an item this card has not got, which is that method's rule: a
    /// caller has invented a target.
    pub fn item(&self, index: usize) -> Rect {
        assert!(
            index < self.items,
            "item {index} of a card of {}",
            self.items
        );
        let band = match self.ruled && index >= self.faders {
            true => size::ROW_MENU_RULE_H,
            false => 0.0,
        };
        Rect::from_min_size(
            Pos2::new(
                self.card.min.x + size::LIB_LIST_PAD,
                self.card.min.y + size::LIB_LIST_PAD + size::LIB_ROW_H * index as f32 + band,
            ),
            egui::vec2(
                self.card.width() - size::LIB_LIST_PAD * 2.0,
                size::LIB_ROW_H,
            ),
        )
    }

    /// **The separator's band**, or `None` where none is drawn — the air,
    /// hairline and air [`RowMenu::rule`] draws, between the faders and the
    /// parameters. It takes no press: a press inside it is the dismissal.
    pub fn rule(&self) -> Option<Rect> {
        self.ruled.then(|| {
            Rect::from_min_size(
                Pos2::new(
                    self.card.min.x + size::LIB_LIST_PAD,
                    self.card.min.y + size::LIB_LIST_PAD + size::LIB_ROW_H * self.faders as f32,
                ),
                egui::vec2(
                    self.card.width() - size::LIB_LIST_PAD * 2.0,
                    size::ROW_MENU_RULE_H,
                ),
            )
        })
    }

    /// **Which item `p` is on**, or `None` for a point on the card's padding,
    /// on the separator, or off the card altogether — [`RowMenu::picked`]'s
    /// own answer.
    pub fn picked(&self, p: karakuri_layout::Point) -> Option<usize> {
        let at = Pos2::new(p.x, p.y);
        (0..self.items).find(|index| self.item(*index).contains(at))
    }
}

/// **What a press on the `+ lane` control asks for.**
///
/// [`Aim`]'s shape three bays along, and the same division: every arm is either
/// this console's own state moving or one named operation, and never a lane
/// appended here
/// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
#[derive(Debug, Clone, PartialEq)]
pub enum Chose {
    /// Put the card down — a press on `+ lane` with it up.
    Open,
    /// Take it away — a press on `+ lane` again, on the card's own ground, on
    /// the separator, or anywhere else while it is down. **The press is spent
    /// on the dismissal**, which is [`crate::input::claim`]'s rule 2 said in
    /// the control.
    Shut,
    /// **The lane, named** — `Operation::PointLane { pattern, target }`, with
    /// the bank off this bay's own reading and the target off the item.
    Point(Operation),
}

impl Sequencer {
    /// **What a press at `p` asks for**, or `None` where there is nothing
    /// under it.
    ///
    /// Four controls and one answer, in the order the mock draws them: a bank
    /// pill chooses the pattern, a cell sets a step, a label mutes a lane, and
    /// the pill chooses what a step is worth. **The mode pill is asked last**
    /// and none of the four can overlap another, so the order is arbitrary
    /// rather than a precedence — it is written down so that this file and the
    /// window that acts on it ask in one order.
    ///
    /// **The `+ lane` control is not here**, because its press is not an
    /// operation: it puts a card down, and what comes back from that card is
    /// [`Sequencer::chose`]. That is [`LibraryBay::aim`]'s division one bay
    /// along, and the same one: a control whose press moves the console's own
    /// state answers an enum rather than an `Option<Operation>`.
    ///
    /// **Every arm names the bank**, which is why [`Sequencer::bank`] is
    /// carried: implying the armed one is the shape `Operation::SelectDeck`'s
    /// rule refuses, and a press that arrived while a bank press was in flight
    /// would otherwise land on whichever pattern won.
    pub fn press(&self, p: karakuri_layout::Point) -> Option<Operation> {
        let at = Pos2::new(p.x, p.y);
        // **The bank pills first, and they are in the bay head** — outside
        // every rectangle below, so this is the order the mock reads in and
        // not a precedence either.
        //
        // **A press asks for that bank and never for the next one**, which is
        // the `+`'s whole argument turned round: with four fixed banks a press
        // on `seq 3` *is* the choice landing on an empty pattern, so there is
        // nothing left for a `+` to mean (ADR-0320, ADR-0327).
        if let Some(bank) = self.banks.iter().position(|pill| pill.contains(at)) {
            return Some(Operation::SelectPattern {
                pattern: bank as u8,
            });
        }
        for (index, row) in self.rows.iter().enumerate() {
            if let Some(cell) = row.cells.iter().position(|cell| cell.contains(at)) {
                return Some(Operation::SetStep {
                    pattern: self.bank as u8,
                    lane: index as u8,
                    // **The stored slot and not the drawn step**, which is
                    // what keeps the payload independent of the mode: at an
                    // eighth this sends `2k`, so a step press and a mode press
                    // cannot race into an address that means two things.
                    step: row.slots[cell] as u8,
                    // **A state and never a flip**, which is the cell's own
                    // rule: the press asks for that step to be on, or for it
                    // to be off, and a control that could only flip has no way
                    // to arrive.
                    on: !row.on[cell],
                });
            }
            if row.label.contains(at) {
                return Some(Operation::SetLaneMute {
                    pattern: self.bank as u8,
                    lane: index as u8,
                    muted: !row.muted,
                });
            }
        }
        self.mode_pill
            .contains(at)
            .then_some(Operation::SetPatternGrid {
                pattern: self.bank as u8,
                // **The other of the two, named**: the cycle is the surface's
                // affordance and the operation carries where it arrived
                // (P-0090).
                grid: match self.mode {
                    StepMode::Sixteenth => StepMode::Eighth,
                    StepMode::Eighth => StepMode::Sixteenth,
                },
            })
    }

    /// **What a press at `p` asks of the `+ lane` control**, or `None` where
    /// the press was on nothing it owns.
    ///
    /// # Two questions, and which one it is depends on whether the card is down
    ///
    /// [`LibraryBay::menu_ask`]'s rule, and it is that method's word for word:
    ///
    /// - **With the card up** this is the pill alone — a press on it opens the
    ///   card, and a press anywhere else answers `None` so the arms above can
    ///   have it.
    /// - **With one down** every press is the card's, which is
    ///   [`crate::input::claim`]'s rule 2: on an item it picks, on the
    ///   separator, on the card's padding or anywhere else on the console it
    ///   dismisses. So this never answers `None` while the card is down.
    ///
    /// **A pick names the bank this bay is reading**, exactly as
    /// [`Sequencer::press`]'s arms do: the lane lands in the pattern that was
    /// drawn rather than in whichever is armed by the time it is performed.
    pub fn chose(&self, p: karakuri_layout::Point, choices: &Choices) -> Option<Chose> {
        let Some(card) = self.card else {
            return self
                .add
                .contains(Pos2::new(p.x, p.y))
                .then_some(Chose::Open);
        };
        Some(match card.picked(p) {
            Some(item) => match choices.items.get(item) {
                Some(choice) => Chose::Point(Operation::PointLane {
                    pattern: self.bank as u8,
                    target: choice.target.clone(),
                }),
                // **A card drawn from a longer list than the one handed in
                // here** is a caller asking two questions of two readings, and
                // the dismissal is the answer that invents nothing — the
                // re-check `LibraryBay::menu_ask` does against its listing,
                // for its reason.
                None => Chose::Shut,
            },
            None => Chose::Shut,
        })
    }

    /// **Whether `p` is on anything here a press means something on**, which
    /// is what [`crate::input::claim`] asks. The ruler, the readout and the
    /// playhead are readouts and answer `false`.
    ///
    /// **The `+ lane` pill is one of them**, and the card is not: a card that
    /// is down claims every press on the console under rule 2, which is
    /// answered before rule 4 is reached and is why this is only ever asked
    /// with the card up.
    pub fn owns(&self, p: karakuri_layout::Point) -> bool {
        self.press(p).is_some() || self.add.contains(Pos2::new(p.x, p.y))
    }

    /// **How many controls this bay draws**, which is what
    /// [`crate::input::PROBES`] registers: a cell per drawn step of every
    /// lane, a label per lane, the mode pill, the four bank pills and
    /// `+ lane`.
    pub fn controls(&self) -> usize {
        self.rows
            .iter()
            .map(|row| row.cells.len() + 1)
            .sum::<usize>()
            + 1
            + self.banks.len()
            + 1
    }
}

/// **The Sequencer bay, painted.**
///
/// Where everything goes is [`sequencer`]'s, so this paints and derives
/// nothing. Term for term from `style.css`:
///
/// - the mode pill — `.pill.armed`, because *"it is armed because it is what
///   the pattern is rather than a preference the head is holding"*.
/// - the step readout — `.seq-head`'s own `color: var(--c-faint)` with the
///   figure in `.val`'s ink, which is what the mock draws.
/// - the ruler — `.seq-ruler`, four numbers centred over the cells they start,
///   at [`size::SEQ_RULER_SIZE`] in the faint ink. **Four numbers whatever the
///   mode**, because the ruler counts *beats* and a bar has four of them: at
///   an eighth they group two cells rather than four, which is the same bar
///   read at the other width.
/// - the playhead — `.seq-play .lane i.at`, a wash of the lavender with its
///   own hairline, painted **under** the rows so a lit cell stays the colour
///   its lane is.
/// - a lane's label — `.seq-label`, right-aligned, with the mark in the
///   lavender; and `.seq-row.mute`'s faint ink where the lane is muted.
/// - a cell — `.seq-lane i`, the well with its hairline; `.on` in the mint;
///   `.on.hot` in the pink where the lane drives the deck on air, which this
///   console cannot know here and so does not draw; and `.seq-row.mute`'s
///   `opacity: 0.3` over the whole row.
fn sequencer_into(ui: &Ui, pal: &Palette, bay: &Sequencer) {
    let painter = ui.painter();
    // **The playhead first**, which is what `.seq-play` sitting before the
    // rows in the mock's markup means once the rows are opaque: a wash under
    // the cells rather than over them.
    if let Some(column) = bay.playhead {
        painter.rect_filled(
            column,
            CornerRadius::same(size::SEQ_PLAY_RADIUS),
            tint(pal.lav, PLAYHEAD_WASH),
        );
        painter.rect_stroke(
            column,
            CornerRadius::same(size::SEQ_PLAY_RADIUS),
            Stroke::new(size::HAIRLINE, pal.lav),
            StrokeKind::Inside,
        );
    }
    pill_into(ui, pal, bay.mode_pill, bay.mode.name(), true);
    let galley = painter.layout_no_wrap(
        bay.step_words.clone(),
        FontId::new(size::BASE, FontFamily::Proportional),
        pal.dim,
    );
    painter.galley(
        Pos2::new(bay.step.min.x, bay.step.center().y - galley.size().y * 0.5),
        galley,
        pal.dim,
    );
    // **The ruler's four numbers**, centred over the cell each group starts.
    // The groups are the bar's beats, so this is the count of beats and not of
    // cells — `Transport::grid`'s four, arrived at from the other side.
    let cells = bay.rows.first().map(|row| row.cells.len()).unwrap_or(0);
    if cells > 0 {
        let per_beat = (cells / RULER_GROUPS).max(1);
        for beat in 0..RULER_GROUPS {
            let at = beat * per_beat;
            if at >= cells {
                break;
            }
            let over = bay.rows[0].cells[at];
            let galley = painter.layout_no_wrap(
                format!("{}", beat + 1),
                FontId::new(size::SEQ_RULER_SIZE, FontFamily::Proportional),
                pal.faint,
            );
            painter.galley(
                Pos2::new(
                    over.center().x - galley.size().x * 0.5,
                    bay.ruler.center().y - galley.size().y * 0.5,
                ),
                galley,
                pal.faint,
            );
        }
    }
    for row in &bay.rows {
        let ink = match row.muted {
            true => pal.faint,
            false => pal.text,
        };
        let galley = painter.layout_no_wrap(
            row.words.clone(),
            FontId::new(size::SEQ_LABEL_SIZE, FontFamily::Proportional),
            ink,
        );
        // `.seq-label`'s `justify-content: flex-end`: the name is right
        // against the cells, so the letters line up down the column.
        painter.galley(
            Pos2::new(
                row.label.max.x - galley.size().x,
                row.label.center().y - galley.size().y * 0.5,
            ),
            galley,
            ink,
        );
        for (at, cell) in row.cells.iter().enumerate() {
            let radius = CornerRadius::same(size::SEQ_CELL_RADIUS);
            match row.on[at] {
                true => {
                    let lit = match row.muted {
                        true => tint(pal.mint, MUTED_LANE),
                        false => pal.mint,
                    };
                    painter.rect_filled(*cell, radius, lit);
                }
                false => {
                    painter.rect_filled(*cell, radius, pal.well);
                    painter.rect_stroke(
                        *cell,
                        radius,
                        Stroke::new(size::HAIRLINE, pal.hair),
                        StrokeKind::Inside,
                    );
                }
            }
        }
    }
    // **The foot's `+ lane`**, an ordinary `.pill`: it is an act and not a
    // state, so it is never armed — the mock draws it plain beside a `.sep`.
    pill_into(ui, pal, bay.add, ADD_LANE, false);
}

/// **The `+ lane` chooser's card, painted** — [`library::row_menu_into`]'s card term
/// for term, because it is that card: the same panel fill, radius, hairline
/// and item ink, and the same separator drawn as one rule inside its band.
fn lane_card_into(ui: &Ui, pal: &Palette, card: &LaneCard, choices: &Choices) {
    let painter = ui.painter();
    painter.add(pal.shadow.as_shape(card.card, CornerRadius::same(8)));
    painter.rect_filled(card.card, CornerRadius::same(8), pal.panel);
    painter.rect_stroke(
        card.card,
        CornerRadius::same(8),
        Stroke::new(size::HAIRLINE, pal.line),
        StrokeKind::Inside,
    );
    // **Zipped against the reading rather than counted to `card.items`**, so a
    // card laid out from a longer list than the one being painted draws the
    // items that exist instead of panicking on the geometry.
    for (index, choice) in choices.items.iter().enumerate().take(card.items) {
        let at = card.item(index);
        let galley = painter.layout_no_wrap(
            choice.words.clone(),
            FontId::new(size::BASE, FontFamily::Proportional),
            pal.dim,
        );
        painter.galley(
            Pos2::new(
                at.min.x + size::LIB_ROW_PAD_X,
                at.center().y - galley.size().y * 0.5,
            ),
            galley,
            pal.dim,
        );
    }
    if let Some(band) = card.rule() {
        let rule = band.center().y;
        painter.line_segment(
            [
                Pos2::new(band.min.x + size::LIB_ROW_PAD_X, rule),
                Pos2::new(band.max.x - size::LIB_ROW_PAD_X, rule),
            ],
            Stroke::new(size::HAIRLINE, pal.hair),
        );
    }
}

/// **The word on the foot's pill**, the mock's own — `+ lane`.
const ADD_LANE: &str = "+ lane";

/// **What a fader item says it is**, after the lane label the pick will make:
/// `A ▮ fader`.
///
/// A parameter item says the node and the published name instead — `B ∿ L2:0
/// twist`, which is the mock's own way of naming the fourth lane's target.
const FADER_ITEM: &str = "fader";

/// **How many numbers the ruler draws**: four, which is the beats in a bar.
///
/// It is the bar's own count rather than a division of the cells, which is
/// what makes the ruler read the same in both modes — four numbers over
/// sixteen cells is a group of four, and over eight is a group of two.
/// `docs/manual/console.html` draws exactly this: *"Four numbers over sixteen
/// cells makes a group of four, which is a bar of sixteenths counted in
/// beats."*
const RULER_GROUPS: usize = 4;

/// **`.seq-play .lane i.at`'s `color-mix(in srgb, var(--c-lav) 22%,
/// transparent)`**, as the percentage [`tint`] takes.
const PLAYHEAD_WASH: u8 = 22;

/// **`.seq-row.mute .seq-lane`'s `opacity: 0.3`**, applied to a lit cell as
/// the percentage [`tint`] takes — the pattern is kept and drives nothing, so
/// its steps are still drawn and are drawn dim.
const MUTED_LANE: u8 = 30;

/// **How stale the sequencer's picture may get**, which is what this bay
/// declares under
/// [P-0091](../../../../docs/principles/0091-cost-is-known-before-it-is-paid.md)
/// and what the harness turns into a deadline.
///
/// **One sixteenth at the mock's tempo — 117.19 ms**, which is
/// [`BEAT_MICROS`] quartered. The unit this picture moves in is a whole cell:
/// the playhead stands over one step and then over the next, so there is
/// nothing between two positions to be smooth about and the step *is* the
/// step. The finer of the two modes is the one written down, because a
/// declaration made for the eighth would be half the rate the sixteenth needs
/// and the mode is one press away.
///
/// **Stated at the mock's tempo, for [`BEAT_STALENESS`]'s reason**: a
/// staleness that fell with the tempo would make `Σ (cost / staleness)` a
/// function of how fast the music is, and the two schedulability conditions
/// could then only be asserted against a fastest tempo nobody has written down
/// (ADR-0212). What the music moves is [`step_moves_in`], which is the
/// deadline and not the rate.
///
/// **It is the first declaration on this panel whose unit is a beat
/// subdivision**, so it is what `moves_in >= staleness` is tightest against
/// (ADR-0322, ADR-0283).
pub const STEP_STALENESS: Duration = Duration::from_micros(BEAT_MICROS / 4);

/// **How long until the playhead next stands over a different cell**, from the
/// beat count and the tempo the transport row is drawing.
///
/// The step index is `floor(beats × steps_per_beat)`, so the next boundary is
/// the next whole multiple of the subdivision and this is the distance to it
/// in seconds — the same arithmetic the producer polls with
/// (`karakuri_pattern::Pattern::step_at`), read forwards.
///
/// **It never answers finer than the rate it declared**, which is
/// [`roll_moves_in`]'s rule and [`crate::budget::Declared`]'s invariant: a
/// frame taken a hair before a boundary would otherwise ask for a deadline
/// tending to zero, which is the spin [`crate::repaint`] exists to refuse.
///
/// **A pure function of its two arguments**, so a test chooses the beat it
/// asserts at and nothing here reads a clock.
pub fn step_moves_in(mode: StepMode, beats: f64, bpm: f32) -> Duration {
    // A grid at no tempo has no next boundary, and the rate this declared is
    // the only honest answer — the same shape as a rest longer than the period
    // one bay up.
    if bpm <= 0.0 || !bpm.is_finite() {
        return STEP_STALENESS;
    }
    let per_beat = mode.steps_per_beat();
    let at = beats * per_beat;
    let left = (at.floor() + 1.0 - at) / per_beat * 60.0 / f64::from(bpm);
    // `left` is positive and at most one step, so this is a duration and never
    // a negative one; the clamp is the invariant rather than a guard against
    // the arithmetic.
    Duration::from_secs_f64(left.max(0.0)).max(STEP_STALENESS)
}

/// **A pane head's deck list, painted** — [`deck_list_into`]'s card one bay
/// along, with the deck the pane is *showing* in the panel's own text colour
/// and the rest dim.
///
/// Where everything goes is [`PaneTarget`]'s, so this paints and derives
/// nothing, which is [`deck_list_into`]'s own sentence.
///
/// **The marked row is what this pane is pointed at and never the deck
/// selection**, which is the whole of what this mark is: a pane showing deck C
/// while the keys are on deck A draws `C` in the text colour here and the ring
/// stays on A's strip, one bay over.
fn pane_list_into(ui: &Ui, pal: &Palette, target: &PaneTarget, showing: usize, card: Rect) {
    let painter = ui.painter();
    painter.add(pal.shadow.as_shape(card, CornerRadius::same(8)));
    painter.rect_filled(card, CornerRadius::same(8), pal.panel);
    painter.rect_stroke(
        card,
        CornerRadius::same(8),
        Stroke::new(size::HAIRLINE, pal.line),
        StrokeKind::Inside,
    );
    // `take` rather than a range, because the rows are the letters — see
    // [`deck_list_into`], and `View::point_pane` is what stops a deck this
    // crate has no letter for being asked for.
    for (index, letter) in DECK_LETTERS.iter().enumerate().take(target.rows) {
        let row = target.row(card, index);
        let ink = match index == showing {
            true => pal.text,
            false => pal.dim,
        };
        let galley = painter.layout_no_wrap(
            (*letter).to_owned(),
            FontId::new(size::BASE, FontFamily::Proportional),
            ink,
        );
        painter.galley(
            Pos2::new(
                row.min.x + size::LIB_ROW_PAD_X,
                row.center().y - galley.size().y * 0.5,
            ),
            galley,
            ink,
        );
    }
}

/// **A `uses` line's card, painted** — one row per node the input may be wired
/// to, with the one it is wired to now drawn in the panel's own text colour.
///
/// Where everything goes is [`UsesLine`]'s, so this paints and derives nothing,
/// which is [`deck_list_into`]'s own sentence one bay along. Drawn from
/// [`View::draw`] **after the bays** for that card's reason: it hangs out of a
/// line inside a pane and over the groups under it.
///
/// **The node already wired is not in the list**, so the *marked* row here is
/// never one of them — the ink says nothing about the current wiring and every
/// row is a change. That is why this is one colour where the deck pulldown's
/// list is two: a pulldown names where the *next* press lands and this one
/// names where the input goes.
fn uses_card_into(ui: &Ui, pal: &Palette, line: &UsesLine, uses: &Uses, card: Rect, room: Rect) {
    let painter = ui.painter();
    painter.add(pal.shadow.as_shape(card, CornerRadius::same(8)));
    painter.rect_filled(card, CornerRadius::same(8), pal.panel);
    painter.rect_stroke(
        card,
        CornerRadius::same(8),
        Stroke::new(size::HAIRLINE, pal.line),
        StrokeKind::Inside,
    );
    let painter = painter.with_clip_rect(card);
    for (index, name) in uses.candidates.iter().enumerate().take(line.rows) {
        let Some(row) = line.row_at(room, index) else {
            continue;
        };
        let galley = painter.layout_no_wrap(
            name.clone(),
            FontId::new(size::BASE, FontFamily::Proportional),
            pal.dim,
        );
        painter.galley(
            Pos2::new(
                row.min.x + size::LIB_ROW_PAD_X,
                row.center().y - galley.size().y * 0.5,
            ),
            galley,
            pal.dim,
        );
    }
}

// ---------------------------------------------------------------------------
// The Staging lane
// ---------------------------------------------------------------------------

/// **The word at the head of the Staging lane**, in the source's own
/// capitalisation for [`Kind::Bay`]'s reason: the mock upper-cases in CSS, and
/// that is done at paint time so the word a reader searches for is the word in
/// the source.
const STAGING_TITLE: &str = "Staging";

/// **Where a candidate stands, in the four words `console.html` uses for
/// it** — *"Whether it is on screen: landed, overloaded for costing more than
/// one frame may, refused, or did not compile"*.
///
/// # Two readers, and one set of words
///
/// This is the Staging lane's [`Candidate::stage`] and the transport row's
/// [`Transport::health`]. The page gives them one sentence each and they are
/// the same four answers, so a second enum for the capsule would be
/// `docs/contributing.md` §4's *a name meaning two things*. What differs is
/// **which** verdict each is showing, not what a verdict is: a lane row is a
/// node a build changed whose verdict is still outstanding, and it leaves on
/// `Accepted` with the rest of its slot's; the capsule is the last verdict
/// there was and stands after the lane empties. **One build's verdict can be
/// on several rows at once**, which is a fact about how many nodes it changed
/// and not about the verdict (ADR-0326).
///
/// # One variant per `swap::Event` a verdict is outstanding on, and no fifth
///
/// `karakuri_engine::swap::Event` has six variants and this has four. The
/// two that are not here are the two that leave nothing outstanding:
/// `Accepted` is the watchdog saying the version held the budget, at which
/// point the file and the picture agree and the row leaves the lane; and
/// `WorkerLost` is about the *worker* rather than about a version — nothing
/// will be built again, and no candidate changed state when it happened.
///
/// **`Refused` and `NotCompiled` are two states and not two spellings**, which
/// is the page's own distinction: *"Refused on a row is a build that failed"*
/// — the files checked and the Set they were assembled into would not build —
/// against a `.kir` the *checker* turned down, where there was never a Set to
/// build. The second of them reached no row at all until 2026-09-08, because
/// `karakuri_environment::watch::Watch::poll` printed its diagnostics and
/// answered *nothing to build*; it now answers
/// `karakuri_engine::swap::Polled::Refused` and the deck reports it
/// (`docs/adr/0310-a-source-can-say-it-refused-and-the-lane-draws-it.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// `Event::Swapped` — the build is the live Set and is on screen.
    ///
    /// **There is no fourth word for "being judged", and there is no longer
    /// anything to name one after.** The watchdog used to hold a candidate on
    /// trial for eight warmup and thirty judged frames, which
    /// `karakuri_engine::swap::HotSwap::on_trial` reported; the page's row said
    /// nothing about it, on the reading that a version being judged is on
    /// screen and this row says what is on screen. ADR-0313 removed the trial —
    /// the verdict is reached on the candidate's own measured cost, in the same
    /// call the swap lands in — so a row on this word is a build that landed
    /// **and is running**, and a build that landed and was stopped goes
    /// straight to [`Stage::Overloaded`] in the same drain.
    Landed,
    /// `Event::Overloaded` — **the row this lane most needs to draw.** The
    /// version costs more than one frame may, so it stayed in the slot and the
    /// slot stopped updating: no step, no draw, and the target holding the
    /// last image it made, which the mix and that deck's cell go on reading.
    ///
    /// **Nothing was put back, and that is why the row matters more than it
    /// used to.** The watchdog used to restore the previous *Set* and could
    /// not restore the previous *file*, so the picture and the disk disagreed
    /// silently; now the picture is a still of the version the operator asked
    /// for, which looks like working material until something says otherwise.
    /// Two things say it — this row, and that deck's caption
    /// ([`PREVIEW_OVERLOADED`]) — and it stands until the operator fades the
    /// slot out, lands an earlier version, or saves something that fits
    /// (ADR-0316).
    Overloaded,
    /// `Event::Rejected` — the build failed and **nothing changed**: the
    /// running Set is still running, with its `t` and its live count
    /// untouched, and the disk holds material that does not assemble.
    Refused,
    /// `Event::SourceRefused` — the checker turned the source down, so nothing
    /// was built at all: no Set, no candidate, and **no version**, because the
    /// edit history is gated on compiling
    /// (`docs/adr/0089-history-is-gated-on-compiling-not-on-landing.md`). The
    /// picture is whatever was already playing and the disk holds material
    /// that does not check.
    ///
    /// **It is the one row that carries a sentence** — see
    /// [`Candidate::said`], which is why.
    NotCompiled,
}

impl Stage {
    /// **The word drawn at the far end of a candidate row, and the word in
    /// the transport's health capsule**, which is `console.html`'s and the
    /// mock's own: the capsule names the same four answers in the same
    /// words — *"the other answers are overloaded, failed to build, and did
    /// not compile"*.
    ///
    /// **That capsule is in the transport row and this said *the deck head*
    /// until 2026-09-08**, which was wrong rather than stale: there is no
    /// `landed` pill on a deck head anywhere in the mock, and the one the
    /// sentence is quoting is `.transport`'s.
    pub fn word(self) -> &'static str {
        match self {
            Stage::Landed => "landed",
            Stage::Overloaded => "overloaded",
            Stage::Refused => "refused",
            Stage::NotCompiled => "did not compile",
        }
    }
}

/// **One candidate**, which is one node a build changed and nobody has ruled
/// on — or, where no node can be named, one deck slot whose file no longer
/// agrees with its picture.
///
/// # A row is a changed node, and the caller is what knows which
///
/// A `swap::Event` carries an `id` and a `label` and nothing else about *what*
/// was built, because a `karakuri_engine::Request` restates every node of a
/// slot — so a **verdict** is over a build rather than over a node. Which
/// nodes that build actually *changed* is a diff of the hashes a build reports
/// against the ones the slot was already playing, and the caller is where both
/// lists are in one hand. So a build that changed two nodes is handed in as
/// two of these, carrying one verdict each
/// (`docs/adr/0326-a-staging-row-is-a-changed-node-and-the-row-is-the-keep.md`).
///
/// **[`at`](Self::at) is `None` where no node can be named**, and that is a
/// state rather than a gap: `Event::Rejected` and `Event::SourceRefused` are
/// builds that did not happen, so there is no new node list to hold against
/// the old one, and a rebuild that restated the stack without changing any of
/// it has an empty diff. Such a row is the slot's, draws no address, and
/// offers neither press — both name a node.
///
/// **[`said`](Self::said) is here on the same rule rather than against it**:
/// `Event::SourceRefused` carries the diagnostics, so they are a thing the
/// wire has and not a thing this crate derives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    /// **Which deck slot the verdict is about**, drawn as
    /// [`DECK_LETTERS`]' letter.
    ///
    /// **It is not the node**, and it is not a stand-in for one: the node is
    /// [`at`](Self::at), and this is the address the *verdict* has. Both are
    /// drawn, because a node address is per slot — two decks running one Set
    /// have the same `L4:0` — and both are needed to spell the operation a
    /// press on this row asks for.
    ///
    /// **It is drawn because a label does not say where.** The program this
    /// panel is drawn by runs every one of its [`DECKS`] slots from its
    /// **own** copy of the material — `working_copies` in
    /// `karakuri/src/main.rs`, and `docs/manual/console.html`'s *Every deck
    /// runs from its own copy* — so a save reaches the one deck whose file it
    /// is. What it does not do is make the labels distinct: four slots opened
    /// on one preset build four labels that are the same string, because a
    /// label is every node's `proc` name joined and those are the same
    /// procedures. So the label says what was built and only this says where.
    ///
    /// **And where is the reading this lane exists for**, though no longer for
    /// the reason it was written with. A trial used to be *frozen* on a slot
    /// that was not being drawn, so a load into a parked deck left a verdict
    /// outstanding and a row in the lane until that deck went on air; ADR-0313
    /// removed the trial, and an off-air slot's candidate is now judged at the
    /// install like any other (see [`Stage::Landed`]). What the letter is for
    /// is unchanged and is the sentence above it: four slots opened on one
    /// preset build four rows whose labels are the same string, so this is the
    /// only thing in the instrument that says which deck to look at — and *put
    /// a node's previous version back* is likewise an act on one slot.
    ///
    /// **This said the program *"plays one pair of files in both its slots, so
    /// one save produces two builds whose labels are the same string"* until
    /// 2026-09-08.** That was two slots sharing one pair, which was a defect in
    /// the program rather than a property of the lane; the copies ended it, and
    /// the argument for the letter is the parked deck above.
    pub deck: usize,
    /// **Which node of that slot the build changed**, and `None` where no node
    /// can be named — see the head of this type.
    ///
    /// **It is the payload half and [`addr`](Self::addr) is the drawn half**,
    /// which is [`Param`]'s arrangement one bay over: `Param::name` is what
    /// the row reads and `Param::param` is what a press asks with. Both are
    /// written by whoever read the Set, in one place, so they are filled
    /// together or not at all.
    ///
    /// **It is what both of this row's presses are addressed by.** `Keep a
    /// candidate` settles this node and `Put a node's previous version back`
    /// steps it back one version; neither can be spelled without it, so a row
    /// where this is `None` offers neither.
    pub at: Option<NodeAt>,
    /// **The mock's `.addr` — `L4:0`** — and empty on a row that names no
    /// node.
    ///
    /// **Written by whoever read the Set**, exactly as [`Node::addr`] is and
    /// for its reason: the layer names are `karakuri-ir`'s `Kind` and this
    /// crate depends on neither it nor the engine.
    pub addr: String,
    /// **What the node's procedure calls itself** — `Set::node_names`' entry
    /// for [`at`](Self::at), which is the same name the Inspector writes on a
    /// node head, and the harness's word rather than this crate's for
    /// [`Strip::name`]'s reason.
    ///
    /// **On a row that names no node it is the build's own label** —
    /// `Request::label`, every node's `proc` name joined with ` + ` — because
    /// that is the finest thing such a verdict has. A checker's refusal is the
    /// one that is finer than the build: it is about one file, and the label
    /// it hands over is that file's name.
    ///
    /// Clipped rather than elided where it does not fit, which is the mock's
    /// own answer: `.cand` sets no `text-overflow` where `.strip-name` and
    /// `.path` both do. An empty string draws no name at all.
    pub name: String,
    /// Whether it is on screen — [`Stage`].
    pub stage: Stage,
    /// **What the checker said**, one line per diagnostic, and empty on every
    /// stage but [`Stage::NotCompiled`].
    ///
    /// # Why this row carries a sentence when no other one does
    ///
    /// The three verdicts above it are about a build the operator can see the
    /// result of: a landed one is on screen, a rolled-back one has the
    /// previous Set on screen, and a refused one names a Set that would not
    /// assemble. A source the checker turned down has produced nothing to
    /// look at, so the word alone tells an operator that a save did not take
    /// and nothing whatever about why. *A refusal carries what the next
    /// attempt needs* (`docs/principles/0083-…`), and on a lane the row is
    /// where it can carry it.
    ///
    /// # The whole list, and the row draws the first of it
    ///
    /// Every diagnostic in a file is reported at once so a repair is one round
    /// trip, and the terminal and the MCP surface both get the lot. A row is
    /// one line, so [`staging_into`] draws the first and says how many others
    /// there are. The list is here rather than the first-and-a-count because
    /// the count is the list's own length, and two fields that must agree are
    /// two fields that can stop agreeing.
    ///
    /// **Formatted where the file was read**, which is the build worker: these
    /// are the strings `karakuri_engine::swap::Refusal` carried, moved through
    /// the event rather than built on the frame they arrive on.
    pub said: Vec<String>,
}

/// **The Staging lane, laid out**: where the candidate rows go and how many of
/// them there is room for.
///
/// # What is drawn, and it is four of the six things a row could be
///
/// `console.html` specifies a row as four things — the node, what the
/// procedure calls itself, whether it is on screen, and when it arrived — and
/// the mock draws two more with no value behind them. This draws **the deck,
/// the node's address, the name and the verdict**, and, on the one verdict
/// that has produced nothing to look at, **what the checker said**
/// ([`Candidate::said`]);
/// [ADR-0200](../../../../docs/adr/0200-a-bays-first-pass-draws-the-values-that-exist-and-omits-the-rest.md)
/// is why the rest is omitted outright rather than drawn hollow.
///
/// **The node arrived on 2026-09-09 and it is what a row now *is*.**
/// `swap::Event` carries an `id` and a `label` and no node at all, because a
/// `Request` restates every node of the slot and a verdict is therefore over a
/// *build*; which node of that build **changed** is a diff, and the caller is
/// where both lists are in one hand — `karakuri_environment::watch::Built`
/// carries `(layer, index, hash)` for the whole stack on every build, and
/// consecutive builds differ where the hashes do. So one build that changed
/// two nodes is two rows here, each with the build's one verdict on it
/// (`docs/adr/0326-a-staging-row-is-a-changed-node-and-the-row-is-the-keep.md`).
/// A verdict with no diff behind it — a build that did not happen, or a
/// rebuild that restated the stack unchanged — is one row with no address, and
/// [`Candidate::at`] is where that is argued.
///
/// What is still omitted:
///
/// - **The coloured dot, and the `you` in `you, 14:41`** — who wrote it.
///   `origin` — the prompt, the model, the seed — is specified in
///   `docs/ir-spec.md` and produced by nothing (*"`origin` and `tag` have no
///   producer"*), so a hand in an editor and a model over MCP are the same
///   save down the same path. The dot goes with it, because the colour *is*
///   the producer.
/// - **`14:41`** — when it arrived. It waits on what the Library bay's `.dim`
///   column waits on and is refused for its reason: the value would exist and
///   a *spelling* does not. `karakuri_environment::history`'s is `%H%M%S-%3f`,
///   which is half a filename; `karakuri_environment::setfile::written_at` is
///   local to the second; the mock's `14:41` is a third. A fourth written here
///   would be the second answer this repository deletes rather than adds, and
///   it would be the *same* decision the Library is waiting on, taken twice.
/// - **The head's `2 waiting`.** [`Kind::Bay`]'s pills are static words and
///   are the mock's *controls* only — every readout in a bay head is undrawn
///   for that reason, `previews 3 of 4` included. And the number would say
///   what the rows already say: this lane has no truncation to report, where
///   the Library's foot has (`n of m`), so a count over the rows would be one
///   readout of two values against another of the same one.
/// - **The mock's third `.cand`,** *a rejected candidate costs nothing*. The
///   page says what that is: a note to whoever is reading the mock, and not a
///   thing the lane draws.
///
/// # The two controls, and which part of the row each is
///
/// **The row is `Keep a candidate` and the capsule at its end is `Put a
/// node's previous version back`** — [`StagingBay::keep`] and
/// [`StagingBay::back`]. It is the Library's list one bay up, arranged the
/// same way: a row that is itself a control with a smaller box inside it,
/// asked first, which there is [`LibraryBay::starred`] and here is the
/// capsule. What decided which act goes on which target is what each costs —
/// keeping moves nothing, writes nothing and takes a line off a list, and a
/// step back writes over the operator's working copy and rebuilds the slot, so
/// the free one takes the large target and the one that writes takes the small
/// one.
///
/// **Neither is offered on a row that names no node** ([`Candidate::at`]),
/// because `karakuri_operation::Operation` spells both of them with one, and
/// **keeping is not offered on [`Stage::Overloaded`]** either: a slot that has
/// stopped is not a candidate an operator is choosing between, and settling it
/// would take away the one row saying the slot is not running (ADR-0316). The
/// capsule *is* offered there — landing an earlier version is one of the three
/// ways out of a stopped slot. Their record questions were settled long before
/// the controls were: `KeepCandidate` is `Written::Silent(Silent::Surface)`
/// and `RestoreProcedure` is `Written::Silent(Silent::OnLanding)`.
///
/// # A row is a changed node, and it leaves when nothing is outstanding
///
/// The rows are the caller's ([`View::staging`]), read off
/// `karakuri_engine::deck::Deck`'s per-slot events and the per-node hashes the
/// builds reported: a slot gets its rows when its newest event is `Swapped`,
/// `Rejected`, `Overloaded` or `SourceRefused`, and loses all of them on
/// `Accepted` — the watchdog saying the version held the budget, which is the
/// one outcome that leaves the file and the picture agreeing.
///
/// **What that stands in for is the operator's own verdict, and it is not the
/// same judgement.** *Keeping* is taste and the watchdog's verdict is cost;
/// the page is explicit that the two are different questions. With no control
/// to keep with, a row that waited for one would never leave the lane, and a
/// lane that never empties is not the lane the page describes — *"empty is
/// this lane's ordinary state"*. So the cost verdict clears the row, and the
/// day a `Keep` control lands it is what clears it instead.
///
/// **A parked slot's row is settled like any other's**, and that was not
/// always so. `HotSwap::begin_frame_parked` used to freeze a trial — a slot
/// that was not being drawn was not paying for the frames it would be judged
/// on — so a build landing in a parked slot had a verdict outstanding for as
/// long as the slot stayed off air. A candidate is judged on its own measured
/// cost since ADR-0313, which does not move with residency, so the verdict
/// arrives at the install wherever the slot is.
///
/// # Where it goes
///
/// `layout` must be solved: [`Layout::rect`] refuses to answer from a dirty
/// one. Like [`library`] and unlike [`master`] this asks `egui` for nothing:
/// every box in the row is the row's own width or a text box measured at paint
/// time, so no rectangle here is the width of the type in it.
///
/// `None` where there is no candidate and `None` where there is no room for
/// one: a console with no engine behind it is every test in this crate, and
/// what the bay draws then is its card and its head and nothing at all —
/// *"no row, no placeholder, and no standing sentence"*.
pub fn staging(layout: &karakuri_layout::Layout, candidates: &[Candidate]) -> Option<StagingBay> {
    // **Nothing outstanding on any slot, which is this lane's ordinary
    // state** — and the state every run starts in. Drawing an empty list
    // would be the standing sentence the page refuses.
    if candidates.is_empty() {
        return None;
    }
    staging_box(
        to_egui(layout.rect(layout.find("staging")?)),
        candidates.len(),
    )
}

/// **The Staging lane, laid out** — see [`staging`] for what is drawn in it
/// and for the six things in the mock's lane and the page's row that are not.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StagingBay {
    /// `.stage-list`'s content box: the region under the bay head, inside
    /// [`size::STAGE_LIST_PAD_TOP`], [`size::STAGE_LIST_PAD_X`] and
    /// [`size::STAGE_LIST_PAD_BOTTOM`], where the rows are laid from the top
    /// with [`size::STAGE_GAP`] between them.
    pub list: Rect,
    /// **How many rows are drawn**, which is how many fit in [`list`](Self::list) —
    /// never more than [`total`](Self::total), and never zero, because a lane
    /// with no room for one row draws no list at all.
    pub rows: usize,
    /// **How many candidates the caller handed over.** Carried and not drawn:
    /// there is no foot in this bay to say `n of m` in, and the head's count
    /// is not drawn either ([`staging`]). It is here because a lane that had
    /// room for fewer rows than there are candidates is a fact a test should
    /// be able to ask about without counting shapes.
    pub total: usize,
}

impl StagingBay {
    /// The `index`th row's rectangle, counting from the top of the list.
    ///
    /// Derived rather than stored for [`LibraryBay::row`]'s reason — the rows
    /// are a stride and a count, and a `Vec` of them would be an allocation a
    /// frame does not need — with the one difference that this stride carries
    /// a gap: `.stage-list` is a column flex with `gap: 5px` where `.lib-list`
    /// states none.
    pub fn row(&self, index: usize) -> Rect {
        Rect::from_min_size(
            Pos2::new(
                self.list.min.x,
                self.list.min.y + (size::CAND_H + size::STAGE_GAP) * index as f32,
            ),
            egui::vec2(self.list.width(), size::CAND_H),
        )
    }

    /// **The `back` capsule of the `index`th row**, or `None` where that row
    /// does not offer one and `None` where the row has no space for it.
    ///
    /// Laid out from the row's right-hand padding, back past the verdict and
    /// one [`size::CAND_GAP`], which is where [`staging_into`] paints it —
    /// **one derivation for the painted capsule and the pressed one**, which
    /// is every other control on this console
    /// ([`crate::input`]): the derivation that draws a control is asked a
    /// second time rather than copied, so what an operator sees and what a
    /// press lands on cannot come apart.
    ///
    /// **`None` where the row would be all capsule**, which is
    /// [`keep_pill`]'s rule and [`look`]'s: *a control that does not fit in
    /// the row it is drawn in is no control at all, rather than half of one*.
    /// The words to its left are a readout and are clipped; this is a target
    /// and is not drawn where it would be cut. What it has to leave room for
    /// is the deck letter and the address, because a capsule sitting on top of
    /// the address would be a press whose operand is underneath it.
    ///
    /// **Offered wherever the row names a node** ([`Candidate::at`]), the
    /// overloaded row included — see [`staging`] for which press is offered
    /// where, and why this one is offered on more rows than the keep is.
    pub fn back_capsule(
        &self,
        ctx: &egui::Context,
        candidates: &[Candidate],
        index: usize,
    ) -> Option<Rect> {
        // Fonts are not valid until `egui` has run a pass, exactly as in
        // [`keep_pill`] — and on the frame before the first one there is
        // nothing drawn here to press.
        if ctx.cumulative_pass_nr() == 0 || index >= self.rows {
            return None;
        }
        let candidate = candidates.get(index)?;
        candidate.at?;
        let row = self.row(index);
        let width = |text: &str, at: f32| {
            ctx.fonts_mut(|f| {
                f.layout_job(span_at(text, at, Color32::PLACEHOLDER))
                    .size()
                    .x
            })
        };
        let verdict = width(candidate.stage.word(), size::CAND_WHO_SIZE);
        let w = pill_width(ctx, BACK_LABEL);
        let capsule = Rect::from_min_size(
            Pos2::new(
                row.max.x - size::CAND_PAD_X - verdict - size::CAND_GAP - w,
                row.center().y - size::PILL_H * 0.5,
            ),
            egui::vec2(w, size::PILL_H),
        );
        // **Against what the row's left-hand end is already using**, which is
        // the deck letter and the address: a capsule that reached back over
        // the address would be drawn on top of the thing it is a control for,
        // and a press would then be aimed by something it is covering.
        let letter = DECK_LETTERS.get(candidate.deck).copied().unwrap_or("?");
        let least = size::CAND_PAD_X
            + width(letter, size::BASE)
            + size::CAND_GAP
            + width(&candidate.addr, size::BASE)
            + size::CAND_GAP;
        (capsule.min.x >= row.min.x + least).then_some(capsule)
    }

    /// **A press on the `back` capsule of a row**, as the operation it asks
    /// for: this node's previous version, one step and never a cursor.
    ///
    /// **The listing goes in with the point**, exactly as it does for the
    /// Library bay's rows and for its reason: what a row *is* is a value the
    /// caller derived off a deck and handed over, and this crate holds none of
    /// it between frames.
    pub fn back(
        &self,
        ctx: &egui::Context,
        candidates: &[Candidate],
        p: karakuri_layout::Point,
    ) -> Option<Operation> {
        let at = egui::pos2(p.x, p.y);
        (0..self.rows).find_map(|index| {
            let capsule = self.back_capsule(ctx, candidates, index)?;
            let node = candidates.get(index)?.at?;
            capsule.contains(at).then(|| Operation::RestoreProcedure {
                deck: candidates[index].deck as u8,
                revision: Revision::Previous(node),
            })
        })
    }

    /// **A press on a candidate row**, as the operation it asks for: keep this
    /// candidate, which settles the node and takes the row off the lane.
    ///
    /// **The capsule inside the row is not part of it**, and that is a refusal
    /// here rather than an order at the call site: the row is asked after
    /// [`StagingBay::back`] on every route, but a control that depended on
    /// being asked second would be one press away from doing two things the
    /// day somebody reordered a `match`. [`crate::input::claim`]'s rule 4 is
    /// *a control claims what it acts on and no more*, and what this acts on
    /// is the row less its capsule.
    ///
    /// **`None` on a row that offers no keep**, which is two cases and one
    /// sentence each: a row with no node has nothing to settle, and a row on
    /// [`Stage::Overloaded`] is a slot that has stopped rather than a
    /// candidate anyone is choosing between (ADR-0316). A press on either is
    /// nobody's, and [`crate::input::claim`] hands it to `egui` exactly as it
    /// hands over a press on the list's own ground.
    pub fn keep(
        &self,
        ctx: &egui::Context,
        candidates: &[Candidate],
        p: karakuri_layout::Point,
    ) -> Option<Operation> {
        let at = egui::pos2(p.x, p.y);
        (0..self.rows).find_map(|index| {
            let candidate = candidates.get(index)?;
            let node = candidate.at?;
            if candidate.stage == Stage::Overloaded || !self.row(index).contains(at) {
                return None;
            }
            if self
                .back_capsule(ctx, candidates, index)
                .is_some_and(|capsule| capsule.contains(at))
            {
                return None;
            }
            Some(Operation::KeepCandidate {
                deck: candidate.deck as u8,
                node,
            })
        })
    }
}

/// **The word in the capsule at the end of a candidate row**, and it is not
/// `keep`: this console already spends that word on the Inspector pane head's
/// capsule, where it names *Keep what a deck is playing* and writes a Set into
/// the library. Two capsules reading `keep` and meaning two acts would be
/// `docs/contributing.md` §4's *a name meaning two things*, and the more
/// expensive misreading of the two is the one where a person presses this
/// expecting a save. The row's own act needs no word at all, because the
/// control is the row.
const BACK_LABEL: &str = "back";

/// The arithmetic of the lane, away from the layout it reads.
///
/// Term for term from `style.css`:
///
/// - `.stage-list { padding: 6px 9px 8px; display: flex; flex-direction:
///   column; gap: 5px }` — what is left under the bay head, inset by those
///   three numbers, with the rows stacked from the top of it.
/// - `.cand { padding: 4px 7px }` — [`size::CAND_H`] each, with
///   [`size::STAGE_GAP`] between one and the next and none above the first or
///   under the last.
///
/// # How many rows fit, and the gap is one fewer than the rows
///
/// `n` rows occupy `n * CAND_H + (n - 1) * STAGE_GAP`, so the count is
/// `floor((h + gap) / (row + gap))` — the standard trick of lending the last
/// row a gap it does not have. At the height the arrangement pins this bay to
/// it is three: 125 less the head's 27 and the list's 6 and 8 is **84**, and
/// `(84 + 5) / 29.5` is 3.01. Three is the mock's own lane, which is why the
/// arrangement's 125 was written from three rows and two gaps — so the number
/// of rows this bay has room for and the number its height was derived from
/// are one derivation or neither. At the bay's declared minimum of 66 it is
/// one: 66 less 27, 6 and 8 is 25, and `(25 + 5) / 29.5` is 1.01.
///
/// **The half-pixel of slack in both is the arrangement's rounding and not a
/// coincidence**: 27 + 14 + 3 × 24.5 + 2 × 5 is 124.5 and the bay is pinned at
/// 125, and 27 + 14 + 24.5 is 65.5 against a minimum of 66. Half a pixel is
/// less than the gap, so neither number buys a row it was not written for.
///
/// `None` where the region cannot hold one row, which is [`picture_rect`]'s
/// rule stated on a list.
fn staging_box(region: Rect, total: usize) -> Option<StagingBay> {
    let list = Rect::from_min_max(
        Pos2::new(
            region.min.x + size::STAGE_LIST_PAD_X,
            region.min.y + size::HEAD_H + size::STAGE_LIST_PAD_TOP,
        ),
        Pos2::new(
            region.max.x - size::STAGE_LIST_PAD_X,
            region.max.y - size::STAGE_LIST_PAD_BOTTOM,
        ),
    );
    // **Narrower than its own padding is no list**, which is
    // [`library::library_box`]'s refusal across the same axis.
    if list.width() <= 0.0 {
        return None;
    }
    let fits = ((list.height() + size::STAGE_GAP) / (size::CAND_H + size::STAGE_GAP))
        .floor()
        .max(0.0) as usize;
    let rows = fits.min(total);
    (rows > 0).then_some(StagingBay { list, rows, total })
}

/// **The Staging lane's candidate rows, painted.**
///
/// Where everything goes is [`staging`]'s, so this paints and derives nothing.
///
/// Term for term from `style.css`:
///
/// - `.cand` — `background: var(--c-well)` at `border-radius: 8px`, which is
///   the one row in this console that has a well behind it, and
///   `color: var(--c-dim)` for the type in it.
/// - the deck's letter — `pal.faint`, in the place the mock puts its
///   `.dot`: the row's first item, [`size::CAND_GAP`] before the address. It
///   is the letter [`DECK_LETTERS`] gives and the same word the preview cells
///   carry, which the manual calls *"the only thing naming a deck"*.
/// - `.addr` — `pal.lav` at [`size::BASE`], between the letter and the name,
///   which is the mock's own colour for a node address and the one the
///   Inspector's node heads are already drawn in. Empty on a row that names no
///   node, which then draws nothing there and no gap either.
/// - `.pill` — the `back` capsule, laid out by [`StagingBay::back_capsule`]
///   and painted from that same derivation, so what is drawn and what a press
///   lands on are one rectangle. Drawn only where that answers, which is a row
///   with a node and room for it.
/// - `.cand .who` — `pal.faint` at [`size::CAND_WHO_SIZE`], hard against the
///   far end of the row's padding box, which is what `.sep`'s `flex: 1` does
///   to it in the mock. The mock puts the producer there and this puts the
///   verdict, for the reason [`staging`] gives: the producer has no value
///   behind it and the verdict is the whole of what the row is for.
/// - **what the checker said**, in the same faint and at the same size,
///   between the name and the verdict — drawn only where there is one, which
///   is [`Stage::NotCompiled`] and nothing else. It is not a term from the
///   mock, which draws no such row; it is [`Candidate::said`], and the
///   argument for it is there.
///
/// **A name too long for the track is clipped rather than elided**, which is
/// the mock's own answer — `.cand` sets no `text-overflow` — and the clip is
/// the list's box, the same `with_clip_rect` the Library's rows are drawn
/// inside. The verdict is painted after the name and inside the same clip, so
/// a name that runs the width of the row is drawn under it rather than over
/// it: the verdict is the one thing in the row that must stay readable.
fn staging_into(ui: &Ui, pal: &Palette, bay: &StagingBay, candidates: &[Candidate]) {
    let painter = ui.painter().with_clip_rect(bay.list);
    for (index, candidate) in candidates.iter().take(bay.rows).enumerate() {
        let row = bay.row(index);
        painter.rect_filled(row, size::CAND_RADIUS, pal.well);

        let letter = DECK_LETTERS.get(candidate.deck).copied().unwrap_or("?");
        let deck = painter.layout_job(span_at(letter, size::BASE, pal.faint));
        let after = deck.size().x;
        painter.galley(
            Pos2::new(
                row.min.x + size::CAND_PAD_X,
                row.center().y - deck.size().y * 0.5,
            ),
            deck,
            pal.faint,
        );

        // **The node's address, in the lavender the mock gives `.addr`** — the
        // same colour and the same spelling the Inspector writes on a node
        // head, because it is the same address. A row that names no node draws
        // nothing here and takes no gap for it: an empty galley is zero wide
        // and the gap is added to what the address measured, so the name sits
        // where it sat before this column existed.
        let addr = painter.layout_job(span_at(&candidate.addr, size::BASE, pal.lav));
        let addressed = addr.size().x;
        painter.galley(
            Pos2::new(
                row.min.x + size::CAND_PAD_X + after + size::CAND_GAP,
                row.center().y - addr.size().y * 0.5,
            ),
            addr,
            pal.lav,
        );
        let after = match candidate.addr.is_empty() {
            true => after,
            false => after + size::CAND_GAP + addressed,
        };

        let name = painter.layout_job(span_at(&candidate.name, size::BASE, pal.dim));
        let named = name.size().x;
        painter.galley(
            Pos2::new(
                row.min.x + size::CAND_PAD_X + after + size::CAND_GAP,
                row.center().y - name.size().y * 0.5,
            ),
            name,
            pal.dim,
        );

        // **What the checker said, after the name and in the faint** — see
        // [`Candidate::said`]. The first diagnostic, with a count of the
        // others after it, at the same size the verdict is drawn at: this is
        // the row's second reading and not its first, and a line of `2:8:
        // parse: unknown kind` set in the name's size would read as the
        // material's name.
        //
        // Drawn before the verdict and inside the same clip, so a long
        // diagnostic goes under the word rather than over it — the rule the
        // name is already drawn under, and for its reason: the verdict is the
        // one thing in the row that must stay readable.
        if let Some(first) = candidate.said.first() {
            let rest = candidate.said.len() - 1;
            let text = match rest {
                0 => first.clone(),
                1 => format!("{first} · 1 more"),
                more => format!("{first} · {more} more"),
            };
            let said = painter.layout_job(span_at(&text, size::CAND_WHO_SIZE, pal.faint));
            painter.galley(
                Pos2::new(
                    row.min.x + size::CAND_PAD_X + after + size::CAND_GAP + named + size::CAND_GAP,
                    row.center().y - said.size().y * 0.5,
                ),
                said,
                pal.faint,
            );
        }

        let word = painter.layout_job(span_at(
            candidate.stage.word(),
            size::CAND_WHO_SIZE,
            pal.faint,
        ));
        painter.galley(
            Pos2::new(
                row.max.x - size::CAND_PAD_X - word.size().x,
                row.center().y - word.size().y * 0.5,
            ),
            word,
            pal.faint,
        );

        // **The `back` capsule, from the derivation a press is answered
        // from** — [`StagingBay::back_capsule`], asked here rather than laid
        // out a second time, so the capsule an operator sees and the capsule a
        // press lands on are one rectangle. It is drawn last because it is the
        // one thing in the row that is a target: a name long enough to reach
        // it goes under it rather than over it, which is the rule the verdict
        // above is already drawn under.
        if let Some(capsule) = bay.back_capsule(ui.ctx(), candidates, index) {
            pill_at(ui, pal, capsule, BACK_LABEL);
        }
    }
}

// ---------------------------------------------------------------------------
// The Inspector
// ---------------------------------------------------------------------------

/// **How many panes the inspector has**, which is `lib.rs`'s [`arrangement`]
/// and the mock's `.insp-split` read as one number: the CSS is
/// `grid-template-columns: 1fr 9px 1fr`, two tracks and the bar between them,
/// and the arrangement builds `inspector-1` and `inspector-2` to match.
///
/// A [`Spec`](karakuri_layout::Spec) builds a
/// [`Layout`](karakuri_layout::Layout) once and the arena has no insert, so
/// the count is settled at build time — the mock's `2 up ▾` is an operator
/// choosing it while running, and that is a control this pass does not add.
pub const PANES: usize = 2;

/// **The arrangement's name for each pane**, in the order the mock draws them.
///
/// Public for [`DECK_LETTERS`]'s reason: a harness that says *which pane* has
/// to say it in the names the arrangement addresses them by, and a second list
/// written out there would go on saying `inspector-1` the day this one does
/// not.
pub const PANE_NAMES: [&str; PANES] = ["inspector-1", "inspector-2"];

/// **Which deck each pane opens pointed at** — the first pane at deck A and
/// the second at deck B, which is the mock's own two heads.
///
/// It is what this console did before the pulldown existed, written down as a
/// *default* rather than left as the host's habit: the panes used to be filled
/// slot by slot, so a fourth slot could not be looked at at all. See
/// [`View::pane_deck`], which is the pointer this seeds.
///
/// **Not a reading of anything**, so a console with fewer strips than this
/// names opens with a pane pointed at a deck the mixer draws none for — which
/// is a pane with no nodes in it and is the honest state, exactly as
/// [`View::selection`]'s deck A is on a console with no deck behind it.
///
/// [`View::pane_deck`]: View::pane_deck
pub const PANE_DECKS: [u8; PANES] = [0, 1];

/// **The three levels a node head shows, in the order it shows them.**
///
/// `karakuri_operation::Authority` deliberately carries no `ALL` — *"no map
/// target names an authority … it arrives with the first reader"* — and this
/// is that first reader, so the list is written here rather than there. The
/// order is the vocabulary's own declaration order, most restrictive first,
/// which is also the mock's `man / sug / auto`.
///
/// **A fourth level cannot slip past it**: [`auth_word`] is a `match`, so a
/// level added to the vocabulary does not compile until it has a word here,
/// and `every_authority_the_vocabulary_names_is_on_the_node_head` holds this
/// array against that match.
///
/// **Public since the chips became controls**: `crate::input::PROBES` says how
/// many controls a node head's chips are, and a `3` written there would be a
/// second answer to a question this array already gives. The list is the
/// console's and the count is one reading of it — [`PANE_NAMES`]' reason for
/// being public, one list along.
pub const AUTHORITIES: [Authority; 3] = [
    Authority::Manual,
    Authority::Suggesting,
    Authority::Automatic,
];

/// **The node head's abbreviation for one level**, which is the console's own
/// word and deliberately not `Authority::name`: the vocabulary spells these
/// *manual*, *suggesting* and *automatic* because that is what a record
/// carries, and the manual's node head reads `man / sug / auto`.
///
/// A `match` for [`blend_mode`](crate::view)'s reason one crate along: a
/// fourth level in the vocabulary stops the build here rather than drawing a
/// blank chip.
fn auth_word(authority: Authority) -> &'static str {
    match authority {
        Authority::Manual => "man",
        Authority::Suggesting => "sug",
        Authority::Automatic => "auto",
    }
}

/// **The word on the sync chip**, which is `karakuri_operation::Sync::name`
/// and not a second spelling: *free*, *tempo*, *beat* are the manual's three
/// and the mock's three.
fn sync_word(sync: Sync) -> &'static str {
    sync.name()
}

/// **Every sync mode, in the order the deck head's chip walks them** — the
/// vocabulary's own declaration order, which is `Sync::ALL`'s in
/// `karakuri_engine::transport` and the mock's *free, tempo, beat*.
///
/// [`AUTHORITIES`]' array one head along, and it exists for the same two
/// reasons: [`DeckHead::mode`] steps *positions* in this list, which a `match`
/// cannot express once the step may be refused, and `tests/deck_head.rs` walks
/// it to assert the cycle reaches every mode. A fourth mode is still caught at
/// build time, by [`sync_word`]'s `match` on the way to a word and by
/// [`anchor_letter`]'s on the way to a letter.
///
/// **Public because [`Pane::allows`] is stated in this order**, and whoever
/// fills that field is in another crate: an order a caller has to match and
/// cannot name is an order two crates would each write down.
pub const SYNCS: [Sync; 3] = [Sync::Free, Sync::Tempo, Sync::Beat];

/// **The next mode the chip can actually get to**, walking [`SYNCS`] from the
/// one after `at` and taking the first this deck's material can honour.
///
/// # It skips rather than offering, and that is the mock's own sentence
///
/// *"Click to cycle, and the cycle skips a mode this material cannot honour
/// instead of offering it"*. Whether a mode is honourable is
/// `karakuri_engine::deck::Deck::sync_allowed`'s and arrives here as
/// [`Pane::allows`] — the surface owns the affordance and never the authority
/// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)),
/// so this chooses which destination to name and refuses nothing.
///
/// # Landing back on `at` is a state and not a failure
///
/// The walk starts at `at + 1`, so it comes back to `at` only when every other
/// mode is refused — and the operation that comes out then names the mode the
/// deck is in, which **re-anchors**. That is the one re-anchor a cycle can
/// reach and it is the one where re-anchoring does nothing: the only material
/// that refuses two modes is material that accumulates *and* reads the beat,
/// where the mode left is `Free` and `Free` reads no anchor. The control that
/// can ask for it deliberately is [`DeckHead::anchor`]
/// ([ADR-0218](../../../../docs/adr/0218-re-anchoring-is-set-sync-naming-the-mode-the-deck-is-in-and-a-cycle-cannot-say-it.md)).
///
/// `Free` is refused by nothing, so the loop always finds something and there
/// is no `None` to answer.
pub(crate) fn next_sync(at: Sync, allows: [bool; SYNCS.len()]) -> Sync {
    let from = SYNCS.iter().position(|s| *s == at).unwrap_or(0);
    (1..=SYNCS.len())
        .map(|step| (from + step) % SYNCS.len())
        .find(|index| allows[*index])
        .map(|index| SYNCS[index])
        .unwrap_or(at)
}

/// **The letter the anchor readout leads with**: the mock's `T128` under tempo
/// sync and `B128 +0.25` under beat, and nothing at all under free — *"a free
/// deck shows neither, because free is the absence of a transport rather than
/// a setting, and a column reading free on every deck would be four words of
/// nothing."*
fn anchor_letter(sync: Sync) -> Option<&'static str> {
    match sync {
        Sync::Free => None,
        Sync::Tempo => Some("T"),
        Sync::Beat => Some("B"),
    }
}

/// **What one pane of the inspector is showing**, handed in by whoever has a
/// deck — the same seam [`Strip`] crosses, one bay along.
///
/// `src/` takes no device and no engine (ADR-0156), so nothing here asks a
/// `Set` anything: every field is a value somebody who *can* ask read off one
/// and wrote down. `crates/karakuri` is where that reading is, and it is where
/// the two omissions below are decided as well.
#[derive(Debug, Clone, PartialEq)]
pub struct Pane {
    /// **Which deck this pane is pointed at**, as an index into
    /// [`DECK_LETTERS`] — the mock's `deck A`.
    ///
    /// **The pane is pointed rather than choosing**, and that is the mock's
    /// `showing … ▾` not being drawn: the chooser is a control and this pass
    /// adds none, so whoever fills this says which deck each pane shows.
    ///
    /// **And it is not [`View::selection`]**, which this console does now
    /// keep. That is *the* deck — one value, what a key press is addressed to,
    /// drawn as one ring — and there are two panes: a chooser here picks a
    /// deck to *look at* while the keys stay where they were, which is the
    /// whole of why the mock draws a caret in each pane head and a ring on one
    /// strip. So this waits on a per-pane pointer nothing keeps, and reading
    /// the deck selection into it would fold two facts into one and make the
    /// second pane a copy of the first.
    pub deck: usize,
    /// **What that deck is playing**, which is the same name the deck's mixer
    /// strip carries and comes from the same place — see [`Strip::name`], and
    /// the short of it is that a `Set` has no name of its own and only
    /// whoever built it knows what to call it.
    pub material: String,
    /// What this deck's clock is locked to: `karakuri_engine::transport::Sync`
    /// as the vocabulary's copy of the same three.
    pub sync: Sync,
    /// **Which of [`SYNCS`] this deck's material can honour**, in that order —
    /// what the sync chip's cycle skips over.
    ///
    /// **The answer and not the two facts it is computed from**, which is the
    /// seam every other field here crosses read one step further along.
    /// `karakuri_engine::deck::Deck::sync_allowed` is what decides it, off
    /// whether the Set in the slot is closed form and whether it reads
    /// `beats`, and `src/` has no engine (ADR-0156) — so whoever owns one asks
    /// it three times and writes the three answers here. Handing in the two
    /// properties instead would put a third copy of `Transport::allows`' rule
    /// in a crate that owns no material, and a control is not the authority on
    /// what it may ask for
    /// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
    ///
    /// **A property of the Set, so it moves when a build lands in the slot** —
    /// the engine says so at `sync_allowed`, and it is why this is read beside
    /// [`Pane::sync`] rather than once.
    ///
    /// All three `true` is the whole of *nothing is refused*, which is what a
    /// console with no engine behind it and every test in this crate that does
    /// not say otherwise hands in; `Free` is refused by no material at all, so
    /// the first entry is never `false` in a reading anything took.
    pub allows: [bool; SYNCS.len()],
    /// **The tempo the deck was engaged at**, which is what its rate is
    /// measured against. Drawn only under [`Sync::Tempo`] and [`Sync::Beat`] —
    /// see [`anchor_letter`].
    pub anchor_bpm: f32,
    /// **The scrub's own value, in beats**, signed. Drawn only under
    /// [`Sync::Beat`], *"since that is the only mode that reads the offset"*.
    ///
    /// In **beats** and one deck's, where the transport row's offset is in
    /// milliseconds and is the whole instrument's — `style.css` says the unit
    /// is what tells them apart, *"so neither is ever drawn without one"*, and
    /// the mock's own `+0.25` is what this is drawn as.
    pub scrub_beats: f64,
    /// Whether this deck's Set folds its renderers into one result or
    /// overdraws them: `karakuri_engine::set::Layering`, as a bit.
    ///
    /// **What the deck is doing, and what a press names the other of.** The
    /// chip's tooltip is *"Click to overdraw them instead"*, and this is both
    /// the word [`deck_head_into`] draws and the state
    /// [`DeckHead::composite`] reads to say which layering a press is asking
    /// for — a destination and never a flip
    /// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
    ///
    /// **It said the press was not a control until 2026-09-09**, on the
    /// grounds that layering is a *build* decision in the engine —
    /// `Set::layering` answers off whether the Set was built with a merge, and
    /// nothing writes it afterwards. Both halves of that are still true and
    /// the conclusion was wrong: a rebuild is what this instrument already
    /// does to change what a slot is running, and the layering is one field of
    /// the aim a watcher is pointed at, so a press re-aims the slot and the
    /// worker rebuilds it — the route a library load takes, judged against the
    /// budget like any other build
    /// ([ADR-0314](../../../../docs/adr/0314-a-control-that-moves-a-field-of-the-aim-re-aims-the-slot-and-the-rebuild-is-the-write.md)).
    /// It is a **readout of what landed** rather than of what was asked for,
    /// which is `Mixer::residency`'s division: the build may still be rolled
    /// back, and the Staging lane is what says so.
    pub composite: bool,
    /// **The two fields of this slot's aim the deck head can move**, or `None`
    /// on a deck with no geometry to size and no randomness to seed — see
    /// [`Aimed`], which is where the argument is.
    pub aimed: Option<Aimed>,
    /// **The node groups**, in node order, which is the order a Set addresses
    /// its own nodes in.
    pub nodes: Vec<Node>,
}

/// **What a slot is built at, in the two fields the deck head offers**: how
/// many elements each of its geometries runs at, and the salt its randomness
/// comes from.
///
/// # It is a reading somebody else took, which is why the candidates are here
///
/// Neither number can be worked out on this side. The ladder is the powers of
/// two inside the range the *material* declares — `capacity [min, max] =
/// default`, which is a `.kir`'s statement and reaches this crate through
/// whoever read the Set — and the salt is the next in the slot's own
/// deterministic sequence, which is the engine's arithmetic over the salt the
/// slot is actually running. So both cross the seam as answers rather than as
/// the facts they are computed from, exactly as [`Pane::allows`] does and for
/// the same reason: a control is not the authority on what it may ask for
/// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)),
/// and this crate holds no engine and no store (ADR-0156).
///
/// **The salt is handed over rather than invented**, which is the half worth
/// stating twice. A console that reached for a random number would produce a
/// picture no later run could produce again
/// ([P-0092](../../../../docs/principles/0092-the-same-inputs-produce-the-same-frame.md));
/// a console handed the next value of a sequence names a destination the way
/// every other control here does.
/// `docs/adr/0328-the-inspectors-deck-head-steps-a-slots-capacity-and-re-salts-it.md`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Aimed {
    /// **What the slot is running at**, which is the number the chip reads.
    ///
    /// One number for the slot, because that is what a re-aim carries: a Set
    /// holding two geometries runs both at a stated capacity, and where none
    /// is stated this is the first geometry's own declared default — the same
    /// reading ADR-0228 records the aim taking.
    pub capacity: u32,
    /// **Whether [`Aimed::capacity`] was asked for**, rather than being what
    /// the material declares for itself. It is the chip's lit state and
    /// nothing else: the number is drawn either way, and this says who chose
    /// it.
    pub stated: bool,
    /// **The numbers a press steps through**, ascending — the powers of two
    /// inside the range every one of this deck's geometries accepts.
    ///
    /// **Empty is a chip that is drawn and claims nothing**, which is the
    /// arrangement an inert scrub is already in: a deck whose geometries
    /// declare no range in common has no capacity a re-aim could send that all
    /// of them would build at, and there is nothing here to offer.
    pub capacities: Vec<u32>,
    /// **The salt `re-salt` asks for**: the next in this slot's own sequence,
    /// derived from the salt it is running.
    pub salt: u32,
}

/// **One node group**: the head that names a node and says who may move it,
/// and whatever is under it.
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    /// The mock's `.addr` — `L1:0`, `L2:0`, `L4`. **Written by whoever read
    /// the Set**, because the layer names are `karakuri-ir`'s `Kind` and this
    /// crate depends on neither it nor the engine.
    pub addr: String,
    /// What the node is called: `Set::node_names`, or the mock's `renderers`
    /// for the group that folds several.
    pub name: String,
    /// **Who may move this node**, or `None` where the group is more than one
    /// node and so has no one value.
    ///
    /// The second case is the mock's own `L4 renderers` head: authority is set
    /// **per node** (ADR-0211, ADR-0216) and that head stands over every
    /// renderer the Set has, so a chip on it would be one of *n* answers drawn
    /// as *the* answer. A Set with one renderer has one node under that head
    /// and the chip is drawn.
    ///
    /// **It carries the node's address beside the level, and it is one field
    /// rather than two.** A press on a chip has to say which node it is
    /// about, and the two are absent together — a head with no one answer has
    /// no one node either — so a pair of `Option`s would be *present or absent
    /// as a unit* held true by prose, which is `karakuri_store::record::NodeAt`'s
    /// argument one crate along and `docs/contributing.md` §4's structural
    /// tier.
    pub authority: Option<NodeAuthority>,
    /// **The node whose source a `keep` on this head would write**, or `None`
    /// on a head with nothing to keep.
    ///
    /// **Two heads carry no capsule and both are the rule rather than an
    /// omission** (`docs/adr/0338-…`, decision 4):
    ///
    /// - **A head standing over more than one node**, which is the mock's
    ///   folded `L4 renderers`. It is [`Node::authority`]'s own absence one
    ///   control along and for its sentence: one capsule over three renderers
    ///   would be one of three answers drawn as *the* answer. Open the fold
    ///   and each renderer has its own.
    /// - **The built-in camera**, which is a node with no procedure behind it:
    ///   a Set declaring no `kind L3` holds the built-in orbit at `L3:0`
    ///   (`docs/ir-spec.md`, *Several cameras*), and there is **no source to
    ///   write**. The mock draws that absence too, and this pass reproduces it
    ///   rather than drawing a capsule that refuses.
    ///
    /// **It is a field beside [`Node::authority`] rather than that field read
    /// again**, because the two absences are not the same set: the built-in
    /// camera *has* an authority and has nothing to keep. A `bool` beside the
    /// address would be *present or absent as a unit* held true by prose,
    /// which is [`NodeAuthority`]'s own argument, so the address and the
    /// having-one are one `Option`.
    pub keep: Option<NodeAt>,
    /// The mock's `.rend-row`: every renderer this Set has, and which of them
    /// is live. Empty on every group that is not the renderers'.
    pub renderers: Vec<Renderer>,
    /// **The inputs this node's procedure declares**, and what fills each —
    /// empty on every node that declares none, which is most of them.
    pub uses: Vec<Uses>,
    /// The published controls that belong to this node.
    pub params: Vec<Param>,
}

/// **One input a node's procedure declares, and the node filling it** — the
/// mock's `.uses` line, under the node head and above that node's rows.
///
/// # The slot is the procedure's word and the node is the Set's
///
/// `uses far : Geometry` names `far` and stops, because a part that names the
/// parts around it is bound to one Set and stops being a library part
/// ([P-0086](../../../../docs/principles/0086-a-procedure-knows-only-what-it-declares.md),
/// [ADR-0152](../../../../docs/adr/0152-a-kir-names-a-slot-and-the-set-names-the-nodes.md)).
/// So `slot` is the declaration's own word and `to` is a node's name, which is
/// exactly `karakuri_engine::set::Edge`'s two halves and exactly what
/// [`Operation::WireInput`](karakuri_operation::Operation::WireInput) carries.
///
/// # The candidates are a reading somebody else took
///
/// [`Uses::candidates`] is the list the card offers, handed across the seam
/// like [`Pane::allows`] and [`View::holds`] before it: which nodes are of the
/// kind this input takes is a question about the Set, and this crate holds no
/// engine (ADR-0156). **What is offered is not what may be reached** — a name
/// the Set cannot use is refused where the Set is built, in the sentence a
/// model's `wire_input` meets
/// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Uses {
    /// **What the procedure calls this input** — `far` in `--edge morph.far=…`.
    pub slot: String,
    /// **The node filling it**, by name. There is no unfilled state: a Set with
    /// an empty input does not build, so this control replaces and never
    /// clears.
    pub to: String,
    /// **The nodes a pick may name**, in node order — this deck's nodes of the
    /// kind the input takes, with the declaring node left out of its own list.
    pub candidates: Vec<String>,
}

/// **Which node a group's head is, and who may move it.**
///
/// The address is `karakuri_operation::NodeAt`, **carried over from whoever
/// read the Set** and deliberately not parsed back out of [`Node::addr`]:
/// that field is the mock's `L1:0`, a display string in the layer word
/// `docs/ir-spec.md` owns, and reading an address back out of what is drawn is
/// the shape *a statement is held true by the thing it describes* forbids
/// ([ADR-0286](../../../../docs/adr/0286-a-parameter-row-writes-the-control-it-draws-and-carries-the-range-rather-than-the-position.md)).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeAuthority {
    pub at: NodeAt,
    /// Who may move it — `manual` where nobody has spoken for it, which is
    /// what a node nobody has spoken for **is** rather than a placeholder.
    pub level: Authority,
}

/// **One renderer chip.**
#[derive(Debug, Clone, PartialEq)]
pub struct Renderer {
    pub name: String,
    /// **Whether this is the one that reaches the screen**, and it is only
    /// ever true under [`Pane::composite`]: *"On, a deck's renderers fold into
    /// one result and one of them is live; off, they are all overdrawn."*
    /// Under overdraw every renderer draws, so marking one would assert a
    /// choice the layering does not make.
    pub live: bool,
}

/// **One parameter row**: what the mock's `.param` reads.
#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    /// **The position in the deck's published interface**, counting from one
    /// and spanning nodes — the mock's `.ord`, and the number a MIDI control
    /// is learned against: *"knob 3 is knob 3 whatever Set is loaded"*.
    ///
    /// **`None` is a control the interface does not carry**, and it is the one
    /// state of this field rather than a missing value: a control off the
    /// interface has no position, and a position is exactly what a knob counts.
    /// Such a row keeps its place in its group and loses its number, its fader
    /// and its figure — drawn so the mark can be pressed again, because this
    /// bay is where publishing is chosen and a choice nobody can see is one
    /// nobody can unmake
    /// ([ADR-0100](../../../../docs/adr/0100-a-published-interface-is-a-choice-of-attention.md),
    /// `docs/adr/0329-…`).
    ///
    /// **It is not *hidden* and it is not *locked***: `--param`, a `param`
    /// record and a model naming the address all still reach the value, which
    /// is ADR-0100's whole sentence — publishing is a choice of attention and
    /// never one of authority.
    pub ord: Option<usize>,
    /// What the Set published it as, which may be an alias for the key inside
    /// the node.
    pub name: String,
    /// What it holds.
    pub value: f32,
    /// **What the Set published it over**, low then high — `Published::range`,
    /// which *"narrows, never redefines"* the range the procedure declared.
    ///
    /// # It used to be the position and is now the range, and that is a
    /// decision rather than a widening
    ///
    /// This field read `at: f32`, *"where it sits in its published range,
    /// `[0, 1]`"*, and its own argument was that **the fader is the only
    /// reader**, so a range plus a value would be a second derivation of
    /// *where along the track*. A fader a hand can move has a second reader —
    /// the grab, which turns a pointer back into a value — and that one needs
    /// the range whichever way this field is spelled. So the position is
    /// [`Param::at`], derived here, and the two directions are one statement
    /// in one place: [`ParamGrip`] and
    /// [ADR-0286](../../../../docs/adr/0286-a-parameter-row-writes-the-control-it-draws-and-carries-the-range-rather-than-the-position.md).
    pub range: [f32; 2],
    /// **Which control this row is**, as the vocabulary addresses one —
    /// `Published::at` and `Published::key`, carried over unchanged.
    ///
    /// **Not the group the row was drawn in**, which is the other reading and
    /// is the one ADR-0286 refuses: a wildcard covering exactly one node is
    /// *drawn* in that node's group, and it goes on meaning every node that
    /// declares the key. See [`ParamGrip`].
    ///
    /// The vocabulary's own type rather than a pair of this crate's, because
    /// the operation carries exactly this and a second spelling of an address
    /// is what `karakuri-operation` exists to stop
    /// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
    pub param: karakuri_operation::ParamAt,
    /// **What is holding this control**, or `None` for a row nothing is
    /// driving — the mock's `.param.bound` and the `.sens` row under it.
    ///
    /// **Written by whoever read the Set**, off its bindings, and it is the
    /// seventh reading the harness takes: `Set::bindings` was the one of them
    /// this pane did not ask for, on ADR-0191's terms, because nothing bound
    /// anything and a bound row was a state the program could not enter. What
    /// changed is that a press can now attach one.
    pub bound: Option<Source>,
}

/// **What is driving one parameter row** — the mock's `.pval.src` and the
/// three readouts on the `.sens` row under it.
///
/// # It is the attachment's own answer and not a second derivation
///
/// The signal, the shape and the range are read off the binding the Set is
/// holding and carried over unchanged, which is [`Param::param`]'s rule one
/// field along: what a control *is* comes from whoever published it, and a
/// surface that rebuilt any of it would be a second statement about one
/// attachment.
///
/// **[`Source::range`] is not [`Param::range`], and the two are two facts.**
/// The row's range is what the control was *published* over — the span the
/// fader rides, which is the procedure's declaration narrowed — and this one
/// is what the signal is *mapped onto*, which a `bind` may narrow again within
/// it. Drawing one and writing the other is what a chip on this row would do
/// if there were one range here, and the operator would see a mapping change
/// under a press that only asked for a different curve.
#[derive(Debug, Clone, PartialEq)]
pub struct Source {
    /// The signal's own name, as the row's `.pval.src` reads it: `energy`,
    /// `beat`, `band3`, `noise`, or `control:<name>` for a macro.
    pub signal: String,
    /// The shape the signal is put through, and the one chip on this row that
    /// is a control.
    pub curve: karakuri_operation::Curve,
    /// What the signal is mapped onto, low then high — the attachment's, not
    /// the row's. See the type's own note.
    pub range: [f32; 2],
    /// **Which attachment this is**, as the vocabulary addresses one.
    ///
    /// `karakuri_operation::BindAt` and not [`Param::param`]'s `ParamAt`,
    /// carried over from the binding rather than derived from the row: an
    /// attachment is one layer's, where a value's wildcard names no layer at
    /// all, and the two are two facts rather than two spellings (see
    /// `BindAt`). It is also what makes a row's *take back* remove the
    /// attachment it is drawn from rather than one that happens to match by
    /// name.
    pub at: karakuri_operation::BindAt,
}

impl Param {
    /// **Where the value sits in the published range**, `[0, 1]` — the
    /// fader's fill, and what a knob's centre is put on.
    ///
    /// **A range of no width is a control with one position**, and the fader
    /// sits at its start rather than at a division by zero. That is the guard
    /// the harness used to carry when this was a field; it is here now, so
    /// there is one place a degenerate range is answered for.
    pub fn at(&self) -> f32 {
        let [low, high] = self.range;
        match high > low {
            false => 0.0,
            true => unit((self.value - low) / (high - low)),
        }
    }

    /// **What this control holds with its fader at `at`** — [`Param::at`]
    /// inverted, and the whole of what a hand on this row asks for.
    ///
    /// `at` is a position on `[0, 1]`, which is what [`Grab::value`] answers,
    /// and it is clamped here for [`unit`]'s reason rather than trusted: a
    /// published range narrows and never redefines, so a value outside it is
    /// one the procedure did not say it still looks like itself over.
    pub fn valued(&self, at: f32) -> f32 {
        let [low, high] = self.range;
        low + (high - low) * unit(at)
    }

    /// **Whether a hand can move this row at all.**
    ///
    /// `Grab::new`'s refusal read on the value axis instead of on the track: a
    /// published range of no width is a control with one position, so a knob
    /// on it is a handle with nowhere to go and every drag of it would ask for
    /// the value it already holds. The row is still **drawn** — the fill and
    /// the figure say what it is — and it is not taken hold of.
    ///
    /// # A bound row is drawn and is not taken hold of either
    ///
    /// **The number a hand would write is not the number the row is
    /// showing.** A knob a hand moves writes the value a binding blends
    /// *from*, and at a measurement's full confidence that value carries no
    /// weight at all — so the handle would move under the hand and the picture
    /// would not, which is the one thing
    /// [ADR-0286](../../../../docs/adr/0286-a-parameter-row-writes-the-control-it-draws-and-carries-the-range-rather-than-the-position.md)
    /// refuses a track press for: *"a handle that jumped to the pointer would
    /// be a lie about what a handle is"*, read on the value axis.
    ///
    /// **It is not a refusal of the write**, and that distinction is
    /// [P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)'s:
    /// `Operation::WriteParam` on a bound parameter is legal, lands, and
    /// leaves the attachment where it is — a `--param` does exactly that
    /// today. What this says is that *this row's fader* is not the affordance
    /// for it while something else is holding the control, and the affordance
    /// that is there is `take back`, one row down. After it the row is a
    /// handle again.
    ///
    /// That is the question ADR-0286 left open — *"whether a hand may move a
    /// knob a signal is holding is Take a parameter back's question"* — and
    /// `docs/manual/console.html`'s *Who is holding a control* is where the
    /// page says it.
    /// **This row as an entry of a published interface** — the shape
    /// [`Operation::Publish`](karakuri_operation::Operation::Publish) carries,
    /// which is `karakuri_engine::set::Published`'s.
    ///
    /// **The address and the range are the ones the row was drawn from**, not
    /// ones re-derived here: a wildcard stays a wildcard and a narrowed range
    /// stays narrowed, which is [`ParamGrip`]'s own rule about writing the
    /// control the row draws rather than the group it was placed in
    /// (ADR-0286).
    fn control(&self) -> karakuri_operation::Control {
        karakuri_operation::Control {
            name: self.name.clone(),
            node: self.param.node,
            key: self.param.key.clone(),
            range: self.range,
        }
    }

    fn movable(&self) -> bool {
        // **A control off the interface draws no fader**, which is what
        // publishing decides: the row is a name and a mark, and there is
        // nothing on it to take hold of.
        self.ord.is_some() && self.bound.is_none() && self.range[1] > self.range[0]
    }
}

/// **The Inspector's pane, laid out**: the head that says which deck, the deck
/// head under it, and what is left for the node groups.
///
/// # What is in the mock's pane and is deliberately not here
///
/// This is [ADR-0200](../../../../docs/adr/0200-a-bays-first-pass-draws-the-values-that-exist-and-omits-the-rest.md)
/// applied to the bay it named as the next one and the hardest: *draw every
/// part of the mock that has a value behind it and omit the rest outright — no
/// placeholder, and no empty case the mock did not itself draw.* Nine things
/// are omitted and each one is named with what it waits on. **Three of the
/// nine have since been drawn**, and they are the deck head's — see
/// [`DeckHead`], which is where their argument now lives.
///
/// **Two are controls and are still not here.**
///
/// - **`showing … ▾`**, the chooser in the pane head. Which deck a pane shows
///   is a **per-pane** pointer this console does not keep, and it is not the
///   deck selection: that one is what a key press is addressed to and there is
///   one of it, where there is a caret in every pane head — see
///   [`Pane::deck`]. So the pane is *pointed* by whoever fills that field and
///   the caret is not drawn. The word `showing` and the deck it names are a
///   readout and are.
/// - **`keep`**, the pill beside it: *"Keep deck A as a Set, exactly as it is
///   on screen … It goes into the library under a name."* That is a write into
///   the store, which is the Library bay's `load` from the other end —
///   and it is the end that is still open. A load re-points a slot's source
///   and lets the worker build it; a keep has to read a *running* Set back out
///   and name it, which is `Set::published`'s side of the seam and a different
///   question entirely.
///
/// **A third was `composite`, called a readout here until 2026-09-09, and it
/// is a control** — [`Pane::composite`] and [`DeckHead::composite`]. The
/// sentence that stood here said layering is a build decision in the engine so
/// a press on it is a rebuild rather than a write, and *"there is no operation
/// in the vocabulary for it to name"*, which was wrong twice:
/// `Operation::SetCompositing` has been in the vocabulary the whole time, and a
/// rebuild is exactly how this instrument changes what a slot is running. A
/// press re-aims the slot — the layering is one field of `watch::Aim` — and the
/// worker builds it off the render thread, which is the route a library load
/// takes
/// ([ADR-0314](../../../../docs/adr/0314-a-control-that-moves-a-field-of-the-aim-re-aims-the-slot-and-the-rebuild-is-the-write.md)).
///
/// **And the fourth was the anchor, which the mock draws as a readout and
/// [ADR-0218](../../../../docs/adr/0218-re-anchoring-is-set-sync-naming-the-mode-the-deck-is-in-and-a-cycle-cannot-say-it.md)
/// made a control.** A press on it emits `SetSync` naming the mode the deck is
/// already in, which re-anchors, and its face goes on being a reading of two
/// numbers the deck has. The sync chip beside it and the scrub's two arrows
/// landed with it — the whole of the deck head is [`deck_head`] now, and this
/// type is the pane's three rows.
///
/// **Two had no value in this workspace at all until 2026-09-09**, which was
/// ADR-0191's rule — a panel drawing a state the engine never entered is a
/// drawing of one — **and both are drawn now**, because a press can attach a
/// signal (ADR-0319).
///
/// - **`.param.bound`'s `.pval.src`**, a bound parameter showing its source
///   instead of a number. [`Param::bound`] is the reading, off
///   `Set::bindings`, and what made it enterable is `Deck::bind` rather than
///   anything in this crate. **The mock's other two sources are still states
///   this program cannot enter**: `midi 21` and `seq 1` are not bindings at
///   all — no MIDI map reaches a Set's parameter, and a sequencer lane is a
///   fifth route into the vocabulary rather than a signal on the bus
///   (ADR-0222) — so a row drawn from either would be ADR-0191's drawing.
/// - **The `.sens` row under a bound parameter** — the signal, the curve, the
///   range and `take back`, which is [`SensChip`]. Two of its four are
///   controls and two are readouts, and the mock's `step` curve beside `seq 1`
///   is not one of the four this vocabulary has.
///
/// **Two are the shape of the mock disagreeing with the shape of a Set**, and
/// they are the two things this pass found:
///
/// - **A published control that names no node has no row.** The manual groups
///   parameters *"by node, the way a Set is addressed everywhere else"*, and
///   the mock draws every `.param` inside a `.node-group`. But
///   `Published::at` is an `Option` and the **default** interface — the one
///   every Set in `crates/karakuri` has, since that binary has no `--publish`
///   — is made entirely of wildcards: *"one control per key, not one per
///   declaration"*, addressed at every node that declares the key. A wildcard
///   covering exactly one node is that node's and is drawn there; one covering
///   several belongs to several groups and is **omitted**, because the mock
///   draws no row outside a group and inventing a place for one is a
///   specification written backwards. It waits on the page saying where such a
///   row goes.
/// - **The `L4` group's authority chip.** See [`Node::authority`].
///
/// **And one is the pane running out of room**, which is what
/// [`InspectorPane::scroll`] answers: the pane scrolls, and
/// [`InspectorPane::shown`] is what it says about that.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InspectorPane {
    /// `.half-head`, along the top of the pane, with its rule on the bottom.
    pub head: Rect,
    /// `.deck-head`, under it — the deck's own clock and its fold.
    pub deck_head: Rect,
    /// What is left under the two heads, where the node groups stack from the
    /// top with a hairline between them.
    pub body: Rect,
    /// **How far this pane's body is scrolled, in force this frame** — the
    /// stored position clamped against what there is to scroll through, and
    /// never the stored position itself.
    ///
    /// # Clamped here and stored nowhere
    ///
    /// The clamp is `0 ..= (content - body height)`, and both ends of that
    /// range move when the pane is dragged — so a clamp written back into
    /// [`View`] would be a resize rewriting what an operator scrolled to. That
    /// is
    /// [ADR-0250](../../../../docs/adr/0250-below-the-minima-the-arrangement-scales-rather-than-being-rewritten.md)'s
    /// rejected *clamp the stored size during the solve*, one region in, and
    /// [P-0082](../../../../docs/principles/0082-looking-never-writes-back.md) is
    /// the rule: **a shorter pane draws less of the same position and stores
    /// nothing**, so dragging it back reproduces what was on screen exactly
    /// rather than nearly.
    ///
    /// What [`View::scroll_by`] does clamp is the *content*, which is a
    /// reading of the deck rather than a viewport — see it for why the two are
    /// not the same clamp.
    pub scroll: f32,
    /// **How tall everything in this pane is**: [`group_h`] over every node
    /// with a [`size::HAIRLINE`] between two of them, whether or not any of it
    /// is on screen.
    ///
    /// It is the number [`scroll`](Self::scroll) is clamped against and the
    /// number [`View::scroll_by`] is clamped against, derived in one place so
    /// the two cannot disagree.
    pub content: f32,
    /// **How many node groups this pane is showing whole**, which is the `n`
    /// of the `n of m` its head reads — [`pane_count`], and rule 04 of
    /// [the manual](../../../../docs/manual/index.html): *"A list that showed you
    /// part of itself says so and says how much."*
    ///
    /// # It is the readout's number and not the walk's
    ///
    /// [`InspectorPane::drawn`] is what is painted and what a press is
    /// hit-tested against, and it is the wider of the two: a group cut by the
    /// top edge or the bottom one is drawn as far as the pane goes and can be
    /// pressed where it is drawn. This counts the ones that are **whole**, so
    /// that `m of m` means *nothing is out of sight* and can never be read off
    /// a pane with a group hanging over an edge.
    ///
    /// **It used to be how many were drawn, and the two were one number.**
    /// A pane drew a group whole or not at all, because there was no position
    /// to scroll to — so below roughly 534px of Inspector bay not one
    /// parameter row was drawn, in the bay whose whole content is parameter
    /// rows. The maintainer's answer to that was *the pane scrolls*, and this
    /// is the half of the old rule that survives it: a part-drawn group is no
    /// longer a lie about what a node has, because the count says how many are
    /// whole and the rest is one notch of the wheel away
    /// ([ADR-0307](../../../../docs/adr/0307-the-inspectors-pane-scrolls-and-the-position-is-the-panes-own.md)).
    ///
    /// **Zero is a state and not a `None`.** A pane too short to hold one
    /// group whole still says which deck it is showing and what that deck's
    /// clock is doing, and still draws as much of the group as it has room
    /// for; it is [`inspector`]'s `None` that means *there is no pane here to
    /// draw*.
    pub shown: usize,
}

impl InspectorPane {
    /// Where the `index`th group goes, and how tall it is — **in the pane's
    /// own coordinates, with [`scroll`](Self::scroll) already taken off**, so
    /// a group above the body has a negative-going top and one below it a top
    /// past `body.max.y`.
    ///
    /// Derived rather than stored for [`LibraryBay::row`]'s reason — the
    /// groups are a walk and a `Vec` of rectangles would be an allocation a
    /// frame does not need — but a walk rather than a stride, because a group
    /// is as tall as what is in it.
    ///
    /// **Every index in `nodes` is an answer**, where this used to refuse
    /// anything past `shown`: a scrolled pane has groups off both edges and
    /// [`drawn`](Self::drawn) is what says which of them reach the picture, so
    /// the rectangle has to exist before that question can be asked. Past the
    /// end of `nodes` is still a caller's error.
    pub fn group(&self, nodes: &[Node], index: usize) -> Rect {
        let top = self.body.min.y - self.scroll
            + nodes
                .iter()
                .take(index)
                .map(|node| group_h(node) + size::HAIRLINE)
                .sum::<f32>();
        Rect::from_min_size(
            Pos2::new(self.body.min.x, top),
            egui::vec2(self.body.width(), group_h(&nodes[index])),
        )
    }

    /// **Which groups reach the picture**, as a range into `nodes` — the ones
    /// a scrolled body has any of on screen, cut edges included.
    ///
    /// It is what [`inspector_into`] paints and what
    /// [`InspectorPane::grip`] and [`InspectorPane::select_renderer`] walk, so
    /// a control is hit-tested over exactly the groups that were drawn. **It
    /// is not [`shown`](Self::shown)**, which counts the whole ones and is the
    /// readout's number: a fader in a group cut by the bottom edge is drawn
    /// and is pressable, and the group it is in is not counted as shown.
    ///
    /// A walk rather than arithmetic, for [`group`](Self::group)'s reason: the
    /// groups are of unequal height. Empty where the pane's body has no
    /// height at all, which is a folded pane.
    pub fn drawn(&self, nodes: &[Node]) -> std::ops::Range<usize> {
        let mut first = nodes.len();
        let mut last = 0;
        let mut top = self.body.min.y - self.scroll;
        for (index, node) in nodes.iter().enumerate() {
            let bottom = top + group_h(node);
            if bottom > self.body.min.y && top < self.body.max.y {
                first = first.min(index);
                last = index + 1;
            }
            top = bottom + size::HAIRLINE;
        }
        match first < last {
            true => first..last,
            false => 0..0,
        }
    }

    /// **What a press at `p` on a renderer chip asks for**, or `None` off
    /// every chip this pane drew.
    ///
    /// # The row is a choice only where the deck composites and holds two
    ///
    /// [Every operation](../../../../docs/manual/operations.html) states the
    /// condition on the row itself — *"Only where the deck composites and
    /// holds two or more. One-way: no position in the cycle folds them all
    /// back in"* — and `docs/manual/console.html` states it from the chips'
    /// side: *"This Set composites, so one renderer is live and the rest are
    /// not."* Under overdraw every renderer draws, so a selection would name a
    /// state the picture is not in; with one renderer there is nothing to
    /// choose between. **Both are drawn and neither is claimed**, which is
    /// [`DeckHead::arrow`]'s arrangement on an inert scrub and this crate's
    /// own rule stated at [`crate::input`]: *a control claims what it acts on
    /// and no more*. The chips keep their shape either way, because a row that
    /// vanished when a deck stopped compositing would move every parameter row
    /// under it out from under the hand.
    ///
    /// **Every chip of a live row is claimed, the lit one included.** It names
    /// a destination, which is what an operation on this panel is
    /// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)),
    /// and pressing the lit one is the selection the deck already has asked
    /// for again — the anchor's shape two rows up. A chip that stopped being
    /// pressable the moment it lit would take the claim out from under a hand
    /// on the beat the swap landed.
    ///
    /// # What is asked before what
    ///
    /// The body, then the group's row, then the chips — [`LibraryBay::chip`]'s
    /// order one bay along and for its reason: the chips are laid end to end
    /// from the row's left padding and [`inspector_into`] clips the paint to
    /// the pane, so a chip that finishes outside the pane is a target only for
    /// the part of it that is drawn. The row is the pane's own width, so that
    /// clip is this row's `contains`.
    ///
    /// **It costs a galley lookup per chip and only inside a renderer row**,
    /// which is [`LibraryBay::chips`]' price: a chip is as wide as the word in
    /// it, and the walk stops at the one under the pointer.
    pub fn select_renderer(
        &self,
        ctx: &egui::Context,
        pane: &Pane,
        p: karakuri_layout::Point,
    ) -> Option<Operation> {
        // Fonts are not valid until `egui` has run a pass — [`deck_head`]'s
        // guard, and before the first one there is no chip drawn to press.
        if ctx.cumulative_pass_nr() == 0 {
            return None;
        }
        let at = Pos2::new(p.x, p.y);
        if !self.body.contains(at) {
            return None;
        }
        self.drawn(&pane.nodes).find_map(|index| {
            let node = pane.nodes.get(index)?;
            if !a_choice(pane, node) {
                return None;
            }
            let row = rend_row_in(self.group(&pane.nodes, index), node);
            if !row.contains(at) {
                return None;
            }
            rend_chips(ctx, row, &node.renderers)
                .find(|(_, chip)| chip.contains(at))
                .map(|(renderer, _)| Operation::SelectRenderer {
                    deck: pane.deck as u8,
                    renderer: renderer as u32,
                })
        })
    }

    /// **What a press at `p` on a node head's `man / sug / auto` asks for**,
    /// or `None` off every chip this pane drew.
    ///
    /// [`InspectorPane::select_renderer`]'s shape one row up, and the same
    /// order of questions: the body, then the group's head, then the chips.
    /// The head is the pane's own width, so the clip [`inspector_into`] paints
    /// under is this row's `contains`.
    ///
    /// **A head with no chip is not a target**, which is `Node::authority`
    /// being `None` — a head that folds more than one node has no one answer
    /// to draw and so no destination to press. Everything else about the walk
    /// is `select_renderer`'s.
    pub fn set_authority(
        &self,
        ctx: &egui::Context,
        pane: &Pane,
        p: karakuri_layout::Point,
    ) -> Option<Operation> {
        // Fonts are not valid until `egui` has run a pass, and before the
        // first one there is no chip drawn to press — `deck_head`'s guard.
        if ctx.cumulative_pass_nr() == 0 {
            return None;
        }
        let at = Pos2::new(p.x, p.y);
        if !self.body.contains(at) {
            return None;
        }
        self.drawn(&pane.nodes).find_map(|index| {
            let node = pane.nodes.get(index)?;
            let at_node = node.authority?.at;
            let group = self.group(&pane.nodes, index);
            let head = Rect::from_min_max(
                group.min,
                Pos2::new(group.max.x, group.min.y + size::NODE_HEAD_H),
            );
            if !head.contains(at) {
                return None;
            }
            // **Inside what the `keep` capsule leaves**, and the trim is
            // [`auth_chips`]' own rather than applied here — a press on the
            // capsule is [`InspectorPane::keep_procedure`]'s and reaches no
            // chip, because the chips are not drawn there.
            auth_chips(ctx, head, node)
                .find(|(_, chip)| chip.contains(at))
                .map(|(authority, _)| Operation::SetAuthority {
                    deck: pane.deck as u8,
                    node: at_node,
                    authority,
                })
        })
    }

    /// **What a press at `p` on a node head's `keep` capsule asks for**, or
    /// `None` off every capsule this pane drew.
    ///
    /// [`InspectorPane::set_authority`]'s walk at the other end of the same
    /// row, and the same order of questions: the body, the group's head, then
    /// the capsule.
    ///
    /// **A head with no capsule is not a target**, which is [`Node::keep`]
    /// being `None` — a head over several nodes, and the built-in camera. Both
    /// fall out here by the derivation answering `None` rather than by a check
    /// of their own, which is the same shape `set_authority` refuses a folded
    /// head in.
    ///
    /// **`id: None`, and the store names the file.** This is the press that
    /// types nothing, so it takes the stamp —
    /// [ADR-0128](../../../../docs/adr/0128-a-set-saved-under-a-name-the-caller-chose-overwrites.md)'s
    /// two routes drawn on one capsule, exactly as [`KeepPill`] draws them for
    /// the deck. The name a head *has* typed is
    /// [`View::named_set`]'s, and the host is what pairs the two: a keep sent
    /// while this pane's head is asking for a name files under what was typed.
    pub fn keep_procedure(
        &self,
        ctx: &egui::Context,
        pane: &Pane,
        p: karakuri_layout::Point,
    ) -> Option<Operation> {
        if ctx.cumulative_pass_nr() == 0 {
            return None;
        }
        let at = Pos2::new(p.x, p.y);
        if !self.body.contains(at) {
            return None;
        }
        self.drawn(&pane.nodes).find_map(|index| {
            let node = pane.nodes.get(index)?;
            let at_node = node.keep?;
            let group = self.group(&pane.nodes, index);
            let head = Rect::from_min_max(
                group.min,
                Pos2::new(group.max.x, group.min.y + size::NODE_HEAD_H),
            );
            if !head.contains(at) {
                return None;
            }
            node_keep(ctx, head, node)?
                .contains(at)
                .then_some(Operation::KeepProcedure {
                    deck: pane.deck as u8,
                    node: at_node,
                    id: None,
                })
        })
    }

    /// **What a press at `p` on a sensitivity row's chips asks for**, or
    /// `None` off every chip this pane drew and off the two that are readouts.
    ///
    /// [`InspectorPane::set_authority`]'s walk one level in: the body, the
    /// row, then the chips. **A row nothing is holding has no sensitivity row
    /// at all** — [`sens_rect`] answers `None` — so there is no case for a
    /// press on one, which is the mock's own arrangement rather than a check.
    ///
    /// The two readouts answer `None` here rather than being left out of
    /// [`sens_chips`], because they are **drawn** and a press has to be able to
    /// land on them and do nothing: leaving them out would put the chips after
    /// them in the wrong place.
    pub fn sensitivity(
        &self,
        ctx: &egui::Context,
        pane: &Pane,
        p: karakuri_layout::Point,
    ) -> Option<Operation> {
        if ctx.cumulative_pass_nr() == 0 {
            return None;
        }
        let at = Pos2::new(p.x, p.y);
        if !self.body.contains(at) {
            return None;
        }
        let deck = pane.deck as u8;
        self.drawn(&pane.nodes).find_map(|index| {
            let node = pane.nodes.get(index)?;
            let group = self.group(&pane.nodes, index);
            node.params.iter().enumerate().find_map(|(at_row, param)| {
                let source = param.bound.as_ref()?;
                let row = sens_rect(group, node, at_row)?;
                if !row.contains(at) {
                    return None;
                }
                sens_chips(ctx, row, source)
                    .find(|(_, chip)| chip.contains(at))
                    .and_then(|(chip, _)| chip.operation(deck, source))
            })
        })
    }

    /// **What a press at `p` takes hold of in this pane**, or `None` where
    /// there is nothing under it a hand can move.
    ///
    /// # It is the knob, and the track is deliberately not a target
    ///
    /// [`Mixer::grab`]'s rule and [`MasterRow::grab`]'s, and it is this bay's
    /// for the same reason read one bay along: a parameter at 0.2 whose track
    /// was clicked would put the value at the far end of its published range,
    /// on stage, because a hand landed three pixels off a knob. The mock draws
    /// a `.fader s` on every `.param` and deliberately draws none on the
    /// transport's exposure track, which is what tells a control with a handle
    /// from one that is set outright — *"a handle that jumped to the pointer
    /// would be a lie about what a handle is"*.
    ///
    /// **The `.param` rows carry no tooltip in the mock**, which is where the
    /// mixer's version of this rule is written down (*"a press on the track
    /// off the knob does nothing, which is every fader in this bay's rule"*),
    /// so the page does not yet say it for this bay. The console's answer is
    /// the mixer's; `docs/manual/console.html` is where it has to be said.
    ///
    /// # A row with nowhere to go is not taken hold of
    ///
    /// [`Param::movable`]: a published range of no width is a control with one
    /// position. The row is drawn — a fill at the start and a figure — and it
    /// is not a handle, which is `Grab::new`'s own refusal read on the value
    /// axis instead of on the track.
    ///
    /// # Only what is drawn, and only where it is drawn
    ///
    /// Two conditions, and they are two because the pane scrolls.
    /// [`drawn`](Self::drawn) is the groups that reach the picture, which is
    /// what a press may land in; **`body.contains` is what keeps a row that
    /// has gone under a head from taking the press anyway**. A group scrolled
    /// off the top still has a rectangle — [`group`](Self::group) answers one
    /// for every index — and that rectangle overlaps the deck head and the
    /// pane head above it, where the paint is clipped away and a knob is
    /// therefore not on screen. Without this check the pane would claim a
    /// press on a knob nobody can see, under a control that is drawn there;
    /// with it, a press outside the body reaches this bay's other derivations
    /// and no other, exactly as `select_renderer` beside it already asked.
    ///
    /// **The clip is the authority and this is the same rectangle**, which is
    /// [`inspector_into`]'s `with_clip_rect(at.body)` asked as a question
    /// rather than applied as a paint.
    pub fn grip<'a>(&self, pane: &'a Pane, p: karakuri_layout::Point) -> Option<ParamGrip<'a>> {
        let p = Pos2::new(p.x, p.y);
        if !self.body.contains(p) {
            return None;
        }
        // The manual's *deck* is the code's *slot*, and a deck holds
        // `MAX_SLOTS` of them — `DECKS`, which is 4 — so the index is a `u8`
        // with room to spare. [`Mixer::grab`]'s note, one bay along.
        let deck = pane.deck as u8;
        self.drawn(&pane.nodes).find_map(|index| {
            let node = pane.nodes.get(index)?;
            let group = self.group(&pane.nodes, index);
            node.params
                .iter()
                .enumerate()
                .find_map(|(at, param)| match param.movable() {
                    false => None,
                    true => {
                        let fader = param_fader(param_rect(group, node, at), param)?;
                        fader
                            .knob
                            .contains(p)
                            .then_some(ParamGrip { deck, param, fader })
                    }
                })
        })
    }

    /// **Whether `p` is on a parameter fader's knob**, which is what
    /// [`crate::input::claim`] asks — the knob, and not the track under it.
    pub fn owns(&self, pane: &Pane, p: karakuri_layout::Point) -> bool {
        self.grip(pane, p).is_some()
    }

    /// **What a press at `p` on a parameter row's leftmost cell asks for**:
    /// [`Operation::Publish`](karakuri_operation::Operation::Publish) carrying
    /// the interface this deck would have with that one control's membership
    /// changed — or `None` off every mark.
    ///
    /// # The whole list, because that is what the operation is about
    ///
    /// The vocabulary says it at the variant: *"the whole ordered list, not one
    /// entry. A MIDI control is bound to a position in the published interface,
    /// so adding one entry at a time would renumber every binding after it; and
    /// an interface that publishes nothing publishes everything, which is a
    /// statement about the list and not about an entry."* So this builds the
    /// list the press is asking for — every published row **in interface
    /// order**, less the one pressed, or with it appended where it was not on
    /// the list — and names it. Nothing here says *drop this one*, which is
    /// what keeps two hands on one deck from disagreeing about what is
    /// published
    /// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
    ///
    /// **Sorted by [`Param::ord`] and not by where the rows are drawn.** A
    /// wildcard control is *placed* in whichever group it resolves to and is
    /// *numbered* by its position in the interface, and the two orders are not
    /// the same walk — so building the list off the pane's own order would
    /// renumber every knob on a deck with a wildcard in it, on a press that
    /// was about a different row entirely.
    ///
    /// **A row that goes back on lands at the end**, which is a decision and
    /// not an accident: nothing in the pane says where it *was*, the position
    /// it left is now somebody else's, and inventing a place for it would move
    /// knobs nobody pressed anything about. The page says so.
    pub fn publishing(&self, pane: &Pane, p: karakuri_layout::Point) -> Option<Operation> {
        let at = Pos2::new(p.x, p.y);
        if !self.body.contains(at) {
            return None;
        }
        let pressed = self.drawn(&pane.nodes).find_map(|index| {
            let node = pane.nodes.get(index)?;
            let group = self.group(&pane.nodes, index);
            node.params.iter().enumerate().find_map(|(row, param)| {
                ord_cell(param_rect(group, node, row), param)
                    .contains(at)
                    .then_some(param)
            })
        })?;
        let mut kept: Vec<(usize, &Param)> = pane
            .nodes
            .iter()
            .flat_map(|node| node.params.iter())
            .filter(|param| !std::ptr::eq(*param, pressed))
            .filter_map(|param| Some((param.ord?, param)))
            .collect();
        kept.sort_by_key(|(ord, _)| *ord);
        let mut controls: Vec<karakuri_operation::Control> =
            kept.into_iter().map(|(_, param)| param.control()).collect();
        if pressed.ord.is_none() {
            controls.push(pressed.control());
        }
        Some(Operation::Publish {
            deck: pane.deck as u8,
            controls,
        })
    }

    /// **Where one row's publish mark is** — the leftmost cell of the
    /// `row`th parameter row of the `node`th group, or `None` where the pane is
    /// not drawing that group or that row.
    ///
    /// The same rectangle [`InspectorPane::publishing`] resolves a press
    /// against and [`param_into`] paints into, which is this bay's rule
    /// everywhere: the derivation that draws a control is the one that
    /// hit-tests it.
    pub fn publish_mark(&self, pane: &Pane, node: usize, row: usize) -> Option<Rect> {
        if !self.drawn(&pane.nodes).any(|drawn| drawn == node) {
            return None;
        }
        let at = pane.nodes.get(node)?;
        let param = at.params.get(row)?;
        Some(ord_cell(
            param_rect(self.group(&pane.nodes, node), at, row),
            param,
        ))
    }

    /// **Where one node's `index`th `uses` line's control is**, or `None`
    /// where the pane is not drawing that group, that group has no such input,
    /// or the line falls outside the body.
    ///
    /// `open` is whether *this* line's card is down, which is the console's own
    /// state and not the pane's — [`View::wiring_open`], the arrangement
    /// [`Load`] is already in with [`View::target_open`].
    ///
    /// **The capsule is as wide as the name in it**, which is why this asks
    /// `egui` for a galley: a node's name is data and a capsule sized to a
    /// constant would clip one Set's names and not another's.
    pub fn uses_line(
        &self,
        ctx: &egui::Context,
        pane: &Pane,
        node: usize,
        index: usize,
        open: bool,
    ) -> Option<UsesLine> {
        if ctx.cumulative_pass_nr() == 0 {
            return None;
        }
        let at = pane.nodes.get(node)?;
        let uses = at.uses.get(index)?;
        if !self.drawn(&pane.nodes).any(|drawn| drawn == node) {
            return None;
        }
        let row = uses_rect(self.group(&pane.nodes, node), index);
        if !self.body.contains_rect(row) {
            return None;
        }
        Some(UsesLine {
            chip: uses_chip_in(ctx, row, uses),
            row,
            rows: match open {
                true => uses.candidates.len(),
                false => 0,
            },
        })
    }

    /// **Which `uses` capsule `p` is on**, as `(node, input)` — or `None` off
    /// every one of them.
    ///
    /// A press here opens a card and emits nothing, which is [`Load`]'s
    /// pulldown exactly: what a pick asks for is the operation, and *open the
    /// list* is not something a map or a model could ever want to say
    /// (ADR-0305).
    pub fn uses_chip(
        &self,
        ctx: &egui::Context,
        pane: &Pane,
        p: karakuri_layout::Point,
    ) -> Option<(usize, usize)> {
        self.drawn(&pane.nodes).find_map(|node| {
            let at = pane.nodes.get(node)?;
            (0..at.uses.len())
                .find(|index| {
                    self.uses_line(ctx, pane, node, *index, false)
                        .is_some_and(|line| line.hit_chip(p))
                })
                .map(|index| (node, index))
        })
    }

    /// **What a press at `p` on an open card asks for**:
    /// [`Operation::WireInput`](karakuri_operation::Operation::WireInput)
    /// naming the node the pick landed on — or `None` off every row.
    ///
    /// # It names the node, and the refusal is not here
    ///
    /// The card lists [`Uses::candidates`], which is a reading of the Set
    /// somebody else took, and what leaves this crate is the name that was
    /// picked. **Nothing is validated on this side**: a name the Set cannot use
    /// is refused where the Set is *built*, by name and with what the Set does
    /// hold — the same wall a model's `wire_input` meets, which sends its edge
    /// to the same place with no check of its own
    /// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
    ///
    /// **A pick replaces**, and that is the language's shape rather than this
    /// control's: an input takes one node, `SetError::SlotBoundTwice` refuses
    /// two edges on one input, and a Set with an unbound input does not build
    /// at all (ADR-0152). So there is no *unwire*, and nothing here offers one.
    pub fn wired(
        &self,
        ctx: &egui::Context,
        pane: &Pane,
        viewport: Rect,
        open: (usize, usize),
        p: karakuri_layout::Point,
    ) -> Option<Operation> {
        let (node, index) = open;
        let at = pane.nodes.get(node)?;
        let uses = at.uses.get(index)?;
        let line = self.uses_line(ctx, pane, node, index, true)?;
        let to = uses.candidates.get(line.picked(viewport, p)?)?;
        Some(Operation::WireInput {
            deck: pane.deck as u8,
            node: at.name.clone(),
            slot: uses.slot.clone(),
            to: to.clone(),
        })
    }

    /// **What a press at `p` takes hold of**, as the model holds every other
    /// fader — [`grabbed`], so a parameter fader keeps whatever it grabbed at
    /// and the value does not jump under the hand.
    pub fn grab(&self, pane: &Pane, p: karakuri_layout::Point) -> Option<Grab> {
        let grip = self.grip(pane, p)?;
        grabbed(
            grip.fader,
            Knob::Param {
                deck: grip.deck,
                param: grip.param.param.clone(),
                range: grip.param.range,
            },
            Pos2::new(p.x, p.y),
        )
    }
}

/// **A parameter fader taken hold of**: which deck, which control, and the
/// track the value rides.
///
/// # What it answers, and what it deliberately does not
///
/// The brief on this control is *a pointer landing on a parameter row's fader
/// answers which deck, which parameter and what value*, and those are the
/// three things here: [`deck`](Self::deck), [`param`](Self::param)'s
/// [`Param::param`], and [`Param::valued`] at wherever the drag gets to.
///
/// **It is not a `Grab`, and that is the seam rather than a gap.** A
/// [`crate::panel::Knob`] is what turns a track position into an
/// [`Operation`], and the arm for this control is
/// [`crate::panel::Knob`]'s to grow — see the module the operation is
/// constructed in. What is here is everything the view can answer without it:
/// where the knob is, which is geometry and the value it was drawn from, and
/// which control it belongs to, which is what the harness read off
/// `Set::published`. The one line that closes it reads
///
/// ```ignore
/// grabbed(
///     grip.fader,
///     Knob::Param {
///         deck: grip.deck,
///         param: grip.param.param.clone(),
///         range: grip.param.range,
///     },
///     Pos2::new(p.x, p.y),
/// )
/// ```
///
/// and it is [`grabbed`] — the same inverse of [`fader`] the mixer's two
/// knobs and the master out are taken hold of through, so a parameter fader
/// keeps whatever it grabbed at and the value does not jump.
///
/// # The control it names is the published one, not the group it was drawn in
///
/// [`Param::param`] is `Published::at` and `Published::key` carried over, so a
/// **wildcard** row stays a wildcard: `None` means every node that declares
/// the key, and the engine refuses one that spans nodes under disagreeing
/// authorities (ADR-0223). The row was *placed* in a group by resolving that
/// wildcard where it covered exactly one node, and writing what the placement
/// resolved to would narrow the control to the node it happens to reach today
/// — [ADR-0286](../../../../docs/adr/0286-a-parameter-row-writes-the-control-it-draws-and-carries-the-range-rather-than-the-position.md).
#[derive(Debug, Clone, PartialEq)]
pub struct ParamGrip<'a> {
    /// **Which deck's**, which is the manual's word for what the code calls a
    /// slot — [`Pane::deck`], the deck this pane is pointed at, and not
    /// [`View::selection`].
    pub deck: u8,
    /// **The row a hand landed on**, borrowed from the pane it was drawn from
    /// rather than copied: the address, the range and the value it holds are
    /// all on it already, and a copy of any of them here would be a second
    /// statement about one control.
    pub param: &'a Param,
    /// **The track it took hold of**, at the value the row was drawn at — what
    /// [`grabbed`] measures the grip's offset and travel from.
    pub fader: Fader,
}

/// **How tall everything in a pane comes to**: [`group_h`] over every node,
/// with a [`size::HAIRLINE`] between two of them.
///
/// **One function because two callers must agree.** [`pane_box`] clamps the
/// position in force against it and [`View::scroll_by`] clamps the stored one,
/// and the same sum written twice is two answers to how far a pane scrolls.
fn content_h(nodes: &[Node]) -> f32 {
    let mut total = 0.0;
    for (index, node) in nodes.iter().enumerate() {
        if index > 0 {
            total += size::HAIRLINE;
        }
        total += group_h(node);
    }
    total
}

/// **How tall one node group is**: its head, the renderer row if it has one,
/// and a [`size::PARAM_H`] row per parameter.
///
/// The hairline between two groups is **not** in here and is added by whoever
/// stacks them — `.node-group`'s `border-bottom` is `0` on the last of them,
/// so *n* groups carry *n - 1* rules, which is [`size::PREVIEW_GAP`]'s reading
/// of a gap one axis along, and [`content_h`] is where that sum is taken.
fn group_h(node: &Node) -> f32 {
    size::NODE_HEAD_H
        + uses_h(node)
        + match node.renderers.is_empty() {
            true => 0.0,
            false => size::REND_ROW_H,
        }
        + node.params.iter().map(rows_h).sum::<f32>()
}

/// **How tall a node's declared inputs come to**: one [`size::USES_H`] line
/// each, and nothing at all on the nodes that declare none — which is most of
/// them, and is why this is an addend rather than a row every group carries.
///
/// One function because [`group_h`] sums it and [`param_rect`] and
/// [`uses_rect`] walk past it, which is [`rows_h`]'s own reason one row down.
fn uses_h(node: &Node) -> f32 {
    node.uses.len() as f32 * size::USES_H
}

/// **Where a node group's `index`th `uses` line goes**: under `.node-head`,
/// the full width of the group and [`size::USES_H`] tall.
fn uses_rect(group: Rect, index: usize) -> Rect {
    let top = group.min.y + size::NODE_HEAD_H + index as f32 * size::USES_H;
    Rect::from_min_max(
        Pos2::new(group.min.x, top),
        Pos2::new(group.max.x, top + size::USES_H),
    )
}

/// **Where a `uses` line's capsule is**, inside the line — the one derivation
/// [`InspectorPane::uses_line`] hit-tests and [`uses_into`] paints, which is
/// [`rend_chips`]' arrangement one row down and its reason: the same arithmetic
/// written twice is a capsule drawn where a hand cannot reach it.
///
/// **The name, the gap and the `▾`** — which is drawn rather than typed, so it
/// is a width here and a mark at the paint, exactly as the Outputs row's pill
/// and the Library bay's pulldown already spell it. `.uses`'s `.sep` puts it
/// against the right of the line, which is the node head's arrangement one row
/// up and the deck head's fold two bays over.
fn uses_chip_in(ctx: &egui::Context, row: Rect, uses: &Uses) -> Rect {
    let text_w = ctx.fonts_mut(|f| {
        f.layout_no_wrap(
            uses.to.clone(),
            FontId::new(size::USES_SIZE, FontFamily::Proportional),
            Color32::PLACEHOLDER,
        )
        .size()
        .x
    });
    let width = size::PILL_PAD_X * 2.0 + text_w + size::SINK_GAP + CHEVRON_W;
    Rect::from_min_size(
        Pos2::new(
            row.max.x - size::PARAM_PAD_R - width,
            row.center().y - size::USES_CHIP_H * 0.5,
        ),
        egui::vec2(width, size::USES_CHIP_H),
    )
}

/// **One `uses` line's control, laid out**: the capsule naming the node that
/// fills the input, and the card of candidates under it.
///
/// # One derivation, and the card hangs *down*
///
/// [`View::draw`] paints these rectangles and [`crate::input::claim`]
/// hit-tests them, which is [`DeckHead`]'s rule and [`Load`]'s. The card hangs
/// **down** from the capsule where the Library bay's hangs up, and the
/// difference is where each control sits: that one is in the *foot* of a bay
/// and this one is inside a pane's body, with the rest of the pane under it.
/// It is held inside the viewport for [`Load::list`]'s reason, so a line near
/// the bottom of a short console draws its card over what is above it rather
/// than off the edge.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UsesLine {
    /// **The whole line**, the full width of the group — what
    /// [`uses_into`] paints into and what a press has to be inside before any
    /// of it is asked.
    pub row: Rect,
    /// **The capsule naming the node filling this input**, at the right of the
    /// line. A press on it opens the card; a press on it while the card is
    /// down is the host's to read as *shut it*, which is [`Load`]'s
    /// arrangement.
    pub chip: Rect,
    /// **How many candidates the card offers**, which is [`Uses::candidates`]'
    /// length while the card is down and **zero** while it is shut —
    /// [`Load::rows`]' shape and its reason: [`UsesLine::row_at`] cannot hand
    /// out a rectangle for a card nobody opened.
    pub rows: usize,
}

impl UsesLine {
    /// Whether `p` is on the capsule.
    pub fn hit_chip(&self, p: karakuri_layout::Point) -> bool {
        self.chip.contains(Pos2::new(p.x, p.y))
    }

    /// **The card under the capsule, or `None` while it is shut** — and `None`
    /// for a line with no candidate to offer, which is a deck holding one node
    /// of the kind this input takes: the node already wired is left out of its
    /// own list, so there is nothing to pick and no card to open.
    pub fn list(&self, viewport: Rect) -> Option<Rect> {
        if self.rows == 0 {
            return None;
        }
        let height = size::LIB_LIST_PAD * 2.0 + size::LIB_ROW_H * self.rows as f32;
        let width = self
            .chip
            .width()
            .max(size::LIB_ROW_H + (size::LIB_ROW_PAD_X + size::LIB_LIST_PAD) * 2.0);
        Some(held_inside(
            &viewport,
            self.chip.min.x,
            self.chip.max.y + size::PILL_GAP,
            width,
            height,
        ))
    }

    /// **Where the `index`th candidate's row is**, or `None` off the end and
    /// `None` while the card is shut.
    pub fn row_at(&self, viewport: Rect, index: usize) -> Option<Rect> {
        if index >= self.rows {
            return None;
        }
        let list = self.list(viewport)?;
        let top = list.min.y + size::LIB_LIST_PAD + size::LIB_ROW_H * index as f32;
        Some(Rect::from_min_max(
            Pos2::new(list.min.x + size::LIB_LIST_PAD, top),
            Pos2::new(list.max.x - size::LIB_LIST_PAD, top + size::LIB_ROW_H),
        ))
    }

    /// **Which candidate `p` is on**, or `None` off every row.
    pub fn picked(&self, viewport: Rect, p: karakuri_layout::Point) -> Option<usize> {
        let at = Pos2::new(p.x, p.y);
        (0..self.rows).find(|index| {
            self.row_at(viewport, *index)
                .is_some_and(|row| row.contains(at))
        })
    }
}

/// **How tall one parameter row and whatever is under it comes to**: a
/// [`size::PARAM_H`] row, and a [`size::SENS_H`] sensitivity row where
/// something is holding the control.
///
/// **One function because three callers must agree.** [`group_h`] sums it,
/// [`param_rect`] walks it as an offset, and [`sens_rect`] steps off the end of
/// one row — and a group as tall as *n* rows with a press resolved against a
/// stride of *n* is a chip drawn where a hand cannot reach it. That is
/// [`param_rect`]'s own argument about a running sum, one level down.
fn rows_h(param: &Param) -> f32 {
    size::PARAM_H
        + match param.bound {
            None => 0.0,
            Some(_) => size::SENS_H,
        }
}

/// **Where a node group's renderer row is**: `.rend-row` under `.node-head`,
/// the full width of the group and [`size::REND_ROW_H`] tall.
///
/// **One formula, because the row is painted *and* pressed.** [`node_into`]
/// walks a group from the top and [`InspectorPane::select_renderer`] asks
/// where the chips in it are; the same arithmetic written twice is two answers
/// that can disagree, which is [`deck_head`]'s rule one row up — *the
/// derivation that draws a control is the one that hit-tests it*.
///
/// Asked only where [`Node::renderers`] is not empty. On a group that has none
/// the rectangle it answers is where the first parameter row goes, which is
/// [`group_h`]'s own arithmetic read the other way.
fn rend_row_in(group: Rect, node: &Node) -> Rect {
    let top = group.min.y + size::NODE_HEAD_H + uses_h(node);
    Rect::from_min_max(
        Pos2::new(group.min.x, top),
        Pos2::new(group.max.x, top + size::REND_ROW_H),
    )
}

/// **Whether this group's renderer chips are a choice a press can make**, and
/// it is [`Renderer::live`]'s own condition asked of the row rather than of
/// one chip: the deck composites, and it holds more than one renderer.
///
/// The manual's row carries the whole of it — *"Only where the deck composites
/// and holds two or more"* — and [`InspectorPane::select_renderer`] is where
/// the argument for drawing the other two cases and claiming neither is
/// written.
fn a_choice(pane: &Pane, node: &Node) -> bool {
    pane.composite && node.renderers.len() > 1
}

/// **One renderer chip's width**: the name at [`size::BASE`] inside
/// [`size::REND_PAD_X`] either side, which is what `.rend` is as wide as. Its
/// `border: 1px solid var(--c-line)` is counted in [`size::REND_H`] down the
/// chip and not across it, exactly as `.mini`'s is in [`deck_head`].
///
/// Asked of `egui` rather than derived, for [`library::chip_width`]'s reason one bay
/// along: a capsule is as wide as the word in it, and the only thing that
/// knows how wide a word is is the thing that will paint it.
fn rend_width(ctx: &egui::Context, name: &str) -> f32 {
    ctx.fonts_mut(|f| {
        f.layout_no_wrap(
            name.to_owned(),
            FontId::new(size::BASE, FontFamily::Proportional),
            Color32::PLACEHOLDER,
        )
        .size()
        .x
    }) + size::REND_PAD_X * 2.0
}

/// **Every renderer chip and its box**, left to right in draw order — the same
/// walk [`rend_row_into`] paints and [`InspectorPane::select_renderer`]
/// hit-tests, so the capsule a press lands on is the capsule the wash is drawn
/// in.
///
/// [`LibraryBay::chips`]' arrangement one bay along, down to the reason it is
/// an iterator: a `Vec` of rectangles would be an allocation on a path asked
/// once per pointer event and once per frame.
///
/// **The index is the renderer's own, in draw order** — the numbering
/// `Operation::SelectRenderer`, `--param L4:1:…` and a `select` record all
/// use, so what comes out of a press is a position in the Set rather than a
/// position in whatever this row managed to draw.
///
/// **The boxes are not clipped and the paint is.** `.rend-row` wraps in the
/// mock and this console draws one row of it (see [`rend_row_into`]), so a
/// chip past the pane's right edge is yielded whole here and held to the part
/// of it that is drawn where the press is answered.
pub fn rend_chips<'a>(
    ctx: &'a egui::Context,
    row: Rect,
    renderers: &'a [Renderer],
) -> impl Iterator<Item = (usize, Rect)> + 'a {
    let mut x = row.min.x + size::REND_ROW_PAD_L;
    // One padding down from the top of the row, which is where `.rend-row`
    // puts it: its padding is `3px 10px 6px 12px`, so a chip is not centred in
    // the row and the space under it is twice the space over it.
    let top = row.min.y + size::REND_ROW_PAD_T;
    renderers.iter().enumerate().map(move |(index, rend)| {
        let chip = Rect::from_min_size(
            Pos2::new(x, top),
            egui::vec2(rend_width(ctx, &rend.name), size::REND_H),
        );
        x += chip.width() + size::REND_GAP;
        (index, chip)
    })
}

/// **The word in a sensitivity row's left-hand track**, `.sens`'s first
/// column.
pub const SENS_LABEL: &str = "sensitivity";

/// **The word on a sensitivity row's last chip**, which is the second rule's
/// other half.
pub const TAKE_BACK: &str = "take back";

/// **One chip on a sensitivity row**, and which of the four a press landed on.
///
/// # Two are controls and two are readouts, and that is the decision
///
/// The mock draws four pills and this crate claims two of them, which is
/// [`InspectorPane::select_renderer`]'s arrangement on an inert renderer row:
/// *a control claims what it acts on and no more*.
///
/// - **[`SensChip::Signal`] is a readout.** The signal bus is **open by
///   design** — `SignalBus::sample` cannot fail and a name nobody provides
///   comes back at confidence 0.0 — so there is no list of sources anywhere
///   for a chooser to be built over, and a console inventing one would be the
///   surface deciding what may be asked for, which is
///   [P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)
///   exactly inverted. A source is named where a source can be named: a Set
///   file's `bind` line, or `--bind`.
/// - **[`SensChip::Curve`] is the control**, and it is the blend chip's shape
///   one bay along: four destinations, a cycle drawn over them here, and
///   `Operation::AttachSignal` naming the one it arrives at. It re-attaches
///   the same signal over the same range through the next shape.
/// - **[`SensChip::Range`] is a readout**, because a range is the procedure's
///   declaration and not an operator's to write —
///   `karakuri_operation::ParamValue`'s own sentence — and there is no second
///   number on this row for a confidence either: a value arrives with how well
///   it is known.
/// - **[`SensChip::TakeBack`] is the other control**, and it is the row this
///   whole arrangement exists for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SensChip {
    Signal,
    Curve,
    Range,
    TakeBack,
}

impl SensChip {
    /// The four, left to right, in the order the mock draws them.
    pub const ALL: [SensChip; 4] = [
        SensChip::Signal,
        SensChip::Curve,
        SensChip::Range,
        SensChip::TakeBack,
    ];

    /// **What this chip reads**, off the attachment the row was drawn from.
    ///
    /// The range is written to two places, which is `.pval`'s figure and the
    /// mock's own `0.10 – 2.40`; the en dash is the mock's `&ndash;`.
    pub fn text(self, source: &Source) -> String {
        match self {
            SensChip::Signal => source.signal.clone(),
            SensChip::Curve => source.curve.name().to_string(),
            SensChip::Range => {
                format!("{:.2} \u{2013} {:.2}", source.range[0], source.range[1])
            }
            SensChip::TakeBack => TAKE_BACK.to_string(),
        }
    }

    /// **What a press on this chip asks for**, or `None` for the two that are
    /// readouts — see the type's own note for why those two are not controls.
    ///
    /// **The curve chip restates the attachment.** An `AttachSignal` carries
    /// the whole of what an attachment is, so changing one field means sending
    /// the other three back unchanged — the source and the range come off
    /// [`Source`] rather than being rebuilt, which is what stops a press for a
    /// different shape from silently re-mapping the signal.
    pub fn operation(self, deck: u8, source: &Source) -> Option<Operation> {
        match self {
            SensChip::Signal | SensChip::Range => None,
            SensChip::Curve => Some(Operation::AttachSignal {
                deck,
                param: source.at.clone(),
                signal: source.signal.clone(),
                curve: next_curve(source.curve),
                range: source.range,
            }),
            SensChip::TakeBack => Some(Operation::TakeParamBack {
                deck,
                param: source.at.clone(),
            }),
        }
    }
}

/// **The next of the four shapes**, wrapping — the cycle the curve chip is,
/// and it lives here rather than in the vocabulary for
/// [`karakuri_operation::Curve::ALL`]'s stated reason: the list is the
/// vocabulary's and the cycle is the surface's.
fn next_curve(curve: karakuri_operation::Curve) -> karakuri_operation::Curve {
    let all = karakuri_operation::Curve::ALL;
    let at = all.iter().position(|c| *c == curve).unwrap_or(0);
    all[(at + 1) % all.len()]
}

/// **Where each chip of a sensitivity row goes**, laid end to end from the
/// row's left-hand track.
///
/// [`rend_chips`]' shape one row down and for its reason: the row is painted
/// *and* pressed, and two copies of where a chip is would be a chip painted
/// where a hand cannot reach it. A chip is as wide as the word in it, so this
/// costs a galley lookup per chip.
pub fn sens_chips<'a>(
    ctx: &'a egui::Context,
    row: Rect,
    source: &'a Source,
) -> impl Iterator<Item = (SensChip, Rect)> + 'a {
    // `.sens`'s two tracks: the word, then the chips, with the grid's own gap
    // between them.
    let mut x = row.min.x + size::SENS_PAD_L + size::SENS_LABEL_W + size::SENS_GAP;
    // One padding down from the top of the row, which is where `.sens` puts
    // it: `padding: 2px 10px 6px 12px`, so a chip is not centred and the space
    // under it is three times the space over it.
    let top = row.min.y + size::SENS_PAD_T;
    SensChip::ALL.into_iter().map(move |chip| {
        let w = sens_width(ctx, &chip.text(source));
        let rect = Rect::from_min_size(Pos2::new(x, top), egui::vec2(w, size::SENS_CHIP_H));
        x += w + size::SENS_CHIP_GAP;
        (chip, rect)
    })
}

/// One sensitivity chip's width: the word at [`size::SENS_SIZE`] inside
/// `.pill`'s padding.
fn sens_width(ctx: &egui::Context, text: &str) -> f32 {
    ctx.fonts_mut(|f| {
        f.layout_no_wrap(
            text.to_owned(),
            FontId::new(size::SENS_SIZE, FontFamily::Proportional),
            Color32::PLACEHOLDER,
        )
        .size()
        .x
    }) + size::SENS_CHIP_PAD_X * 2.0
}

/// **One pane of the Inspector, derived** — see [`InspectorPane`] for what is
/// drawn here and for the nine things in the mock's pane that are not.
///
/// `index` is which pane, into [`PANE_NAMES`]. `scroll` is the position that
/// pane is scrolled to — [`View::scroll_in`], the console's own state and not
/// a reading — and it is **taken here rather than applied by the painter**,
/// because a control is hit-tested off the derivation that draws it and an
/// offset added on one side of that seam and not the other is two answers
/// about where a knob is. It arrives unclamped and leaves clamped:
/// [`InspectorPane::scroll`] is what is in force, and nothing is written back
/// (P-0082). `layout` must be solved:
/// [`Layout::rect`](karakuri_layout::Layout::rect) refuses to answer from a
/// dirty one. Like [`library`] this asks `egui` for nothing: every box in the
/// pane is either the full width of the pane or a track of the mock's own
/// grid, so no rectangle here is the width of the type in it.
///
/// `None` where there is no pane by that name, and `None` where there is no
/// room for the two heads — which is [`picture_rect`]'s rule stated on a pane.
/// A console with no deck behind it hands over no panes at all, and what the
/// bay draws then is its card, its head and the bar between the two panes,
/// exactly as it did before this pass — [`mixer`]'s rule, one bay along.
///
/// **The bay head is taken off the top here and not in [`pane_box`]**, which
/// is what [`mixer::strips_row`] and [`library::library_box`] do one bay along: the head is
/// painted *over* the region rather than laid out beside it, so every body in
/// this file starts at `region.min.y + size::HEAD_H` and the arithmetic under
/// it is written as if the head were not there. A pane is the one body in the
/// arrangement whose region is not the bay's own — `inspector-1` is a child of
/// the split — and that is what hid this: the pane is the full height of the
/// bay, head included, so a `.half-head` drawn at `region.min` lands on top of
/// the word `Inspector`.
pub fn inspector(
    layout: &karakuri_layout::Layout,
    index: usize,
    pane: &Pane,
    scroll: f32,
) -> Option<InspectorPane> {
    let region = to_egui(layout.rect(layout.find(PANE_NAMES.get(index)?)?));
    let under_head = Rect::from_min_max(
        Pos2::new(region.min.x, region.min.y + size::HEAD_H),
        region.max,
    );
    pane_box(under_head, &pane.nodes, scroll)
}

/// The arithmetic of a pane, away from the layout it reads.
///
/// Term for term from `style.css`:
///
/// - `.half-head { padding: 5px 10px; border-bottom: 1px solid var(--c-hair) }`
///   — a [`size::HALF_HEAD_H`] row along the top of the pane, its rule the
///   bottom pixel of it.
/// - `.deck-head { padding: 5px 10px }` — a [`size::DECK_HEAD_H`] row under
///   it, with no rule of its own: *"Its box is `.node-head`'s without the
///   tint"*, and the tint is what separates it from the group below.
/// - what is left is the node groups', stacked from the top.
///
/// **The leftover is the last group's and not the pane's**, which is the
/// opposite of what [`library::library_box`] does with its foot, and the reason is the
/// same read the other way: the mock's pane is a flow with nothing under the
/// groups at all, so there is no row for a leftover to sit under. It shows as
/// the bay's own card below the last group, which is every other empty body in
/// this pass.
fn pane_box(region: Rect, nodes: &[Node], scroll: f32) -> Option<InspectorPane> {
    let head = Rect::from_min_max(
        region.min,
        Pos2::new(region.max.x, region.min.y + size::HALF_HEAD_H),
    );
    let deck_head = Rect::from_min_max(
        Pos2::new(region.min.x, head.max.y),
        Pos2::new(region.max.x, head.max.y + size::DECK_HEAD_H),
    );
    let body = Rect::from_min_max(Pos2::new(region.min.x, deck_head.max.y), region.max);
    // **Narrower than a parameter row's own padding is no pane**, which is
    // [`library::library_box`]'s width check with the mock's own indent in it. There is
    // no matching check down the pane: a pane too short for a group draws its
    // two heads and no group, which is what `shown` answers.
    if body.width() <= size::PARAM_PAD_L + size::PARAM_PAD_R {
        return None;
    }
    // **And a pane too short for its two heads is no pane**, which is what
    // this function's caller promises. `positive` is not enough on its own:
    // the two heads are stated heights, so they stay positive while running
    // off the bottom of a region shorter than their sum.
    if !positive(head) || !positive(deck_head) || deck_head.max.y > region.max.y {
        return None;
    }
    // How tall the whole stack is, from the one function `View::scroll_by`
    // clamps the stored position against as well.
    let content = content_h(nodes);
    // **The clamp is here and the store is not touched.** `max(0.0)` is what
    // a pane taller than its content answers — there is nothing to scroll
    // through, so the position in force is the top whatever an operator once
    // spun the wheel to, and the position they spun to is still where they
    // left it when the pane comes back (P-0082, ADR-0250).
    let scroll = scroll.clamp(0.0, (content - body.height()).max(0.0));
    // **How many are whole**, which is the readout's number and not the walk's
    // — `InspectorPane::drawn` is the walk. A group is whole when both its
    // edges are inside the body: the top one after the scroll has been taken
    // off, and the bottom one before the body's own.
    let mut shown = 0;
    let mut top = -scroll;
    for (index, node) in nodes.iter().enumerate() {
        let rule = match index {
            0 => 0.0,
            _ => size::HAIRLINE,
        };
        top += rule;
        let bottom = top + group_h(node);
        if top >= -EPSILON && bottom <= body.height() + EPSILON {
            shown += 1;
        }
        top = bottom;
    }
    Some(InspectorPane {
        head,
        deck_head,
        body,
        scroll,
        content,
        shown,
    })
}

/// **What counts as touching an edge**, for [`pane_box`]'s *is this group
/// whole* — a hair either way, because both sides of that comparison are sums
/// of `f32` constants and a group that exactly fills the body would otherwise
/// be counted or not by the last bit of a float.
const EPSILON: f32 = 0.001;

/// **What the anchor reads**, and `None` under free sync.
///
/// The mock's own two spellings: `T128` where a deck is tempo-synced, and
/// `B128 +0.25` where it is beat-synced and sitting a quarter beat ahead of
/// the room. The tempo is whole because the mock writes it whole —
/// `T<b>128</b>` — and the offset carries two places and a sign because the
/// mock's `<em>+0.25</em>` does. **The offset is in beats and the transport
/// row's is in milliseconds**, so neither is ever drawn without knowing which
/// it is; here that is the `B` in front of it.
fn anchor_text(pane: &Pane) -> Option<String> {
    let letter = anchor_letter(pane.sync)?;
    Some(match pane.sync {
        Sync::Beat => format!("{letter}{:.0} {:+.2}", pane.anchor_bpm, pane.scrub_beats),
        _ => format!("{letter}{:.0}", pane.anchor_bpm),
    })
}

/// **How far one press of the scrub moves a deck**, in beats.
///
/// A quarter beat, which is the vocabulary's own figure —
/// [`Operation::ScrubDeck`] writes it at the field (*"How far, in beats. A
/// quarter beat is what a key press asks for"*) and the row it fills is
/// titled *Scrub a deck a quarter beat*. The console page says the same of
/// this control: *"a quarter beat a press, into that same offset"*.
///
/// **Signed at the call site and not here.** The left arrow asks for minus
/// this and the right for plus it, so the amount is one number and the
/// direction is which chip was pressed — see [`DeckHead::scrub`], and the
/// manual's *"the one control here meant to go backwards"*.
pub const SCRUB_BEATS: f64 = 0.25;

/// **What the `re-salt` capsule reads.**
pub const RE_SALT_LABEL: &str = "re-salt";

/// **Where a press on the capacity chip arrives**: the next number up the
/// ladder, and off the top back to the bottom — or `None` where the ladder is
/// empty and there is nothing to ask for.
///
/// # The next one *above* what is running, rather than the next one along
///
/// `candidates` is [`Aimed::capacities`], ascending, and the running value is
/// not necessarily one of them: a procedure may declare `capacity [4096,
/// 1048576] = 81920`, and a Set loaded from a file may be running at whatever
/// that file recorded. Asking for the first candidate *greater than* where the
/// slot is answers both cases in one line — the next power of two from a value
/// that is on the ladder, and the next power of two up from one that is not —
/// where a `position` lookup would have fallen back to the bottom of the range
/// and taken a slot from 81920 to 4096 on a press that reads as *one step*.
///
/// **The wrap goes through the bottom and not through unset**, which is where
/// this differs from the Library's two filter fields (ADR-0262): a filter has a
/// state that is *not narrowed* and a capacity has no such state — every
/// geometry is running at some number — so the end of the ladder is the
/// beginning of it.
fn stepped_capacity(candidates: &[u32], at: u32) -> Option<u32> {
    candidates
        .iter()
        .find(|candidate| **candidate > at)
        .or_else(|| candidates.first())
        .copied()
}

/// **The deck head's controls, laid out**: the chip that names the mode, the
/// anchor that re-asks for it, the two arrows that scrub, the two chips that
/// ask for a different build, and the fold at the right.
///
/// # One derivation, for [`Outputs`]' reason
///
/// [`View::draw`] paints exactly these rectangles and [`crate::input::claim`]
/// hit-tests exactly these rectangles. Two copies of a flex row's running sum
/// is an arrow that lights under a pointer that cannot move the deck.
///
/// # Six rectangles and one type, because they are one row
///
/// [`LookRow`]'s reason one bay over: each one's place is measured from the
/// last, which is what a flex row is, and splitting them into six functions
/// would mean measuring the chip before each of them again to find out where
/// it starts. The fold is in here for the same reason, and it is a control
/// too — see [`DeckHead::composite`], which was the one rectangle here that
/// claimed nothing until 2026-09-09.
///
/// **The two before the fold are measured from the right**, because that is
/// what `.sep`'s `flex: 1` does to everything after it: the fold sits against
/// the row's right-hand padding, the `re-salt` capsule one gap before it and
/// the capacity chip one gap before that, and what says the row fits is that
/// the arrows end before the leftmost of the three begins.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DeckHead {
    /// **The sync chip**, which is what a press has to land in to move the
    /// mode on. `.mini`'s box round [`sync_word`], at the left of the row.
    pub mode: Rect,
    /// **The anchor**, or `None` under [`Sync::Free`], where there is nothing
    /// to read and nothing to re-anchor — see [`anchor_text`].
    ///
    /// **The text run grown to a chip's height**, which is [`LookRow::grip`]'s
    /// treatment of a 5px track and its argument: `.anchor` is a bare span at
    /// [`size::ANCHOR_SIZE`] with no padding of its own, and 13.5 pixels of
    /// type is not a target a hand finds. It is grown to [`size::MINI_H`], so
    /// it is the same 15.5 as the chips either side of it and sits in the same
    /// [`size::DECK_HEAD_PAD_Y`] the row gives them. **No wider than the
    /// words**, because the row is a flex row and a target that reached past
    /// its own text would take the arrows' places with it.
    pub anchor: Option<Rect>,
    /// **A quarter beat back.** One `.scrub i`.
    pub back: Rect,
    /// **A quarter beat forward.** The other.
    pub forward: Rect,
    /// **The fold at the right of the row**, and the fourth control on it —
    /// [`DeckHead::compositing`] is what a press on it asks for.
    ///
    /// **It was not a control until 2026-09-09**, on the argument
    /// [`Pane::composite`] carries and corrects: layering is a *build*
    /// decision, so a press is a rebuild rather than a write. That is true and
    /// is the reason this works rather than the reason it could not — a
    /// rebuild off the render thread, judged against the budget and rolled
    /// back on its own, is what this instrument does to change what a slot is
    /// running, and the layering is one field of the aim a watcher is pointed
    /// at (ADR-0314).
    ///
    /// It is here because the row is laid out **to** it: it is `.sep`'s
    /// `flex: 1` pushing it against the right-hand padding, and what says the
    /// controls on the left fit is that they end before it.
    pub composite: Rect,
    /// **The capacity chip and the `re-salt` capsule**, or `None` on a deck
    /// with nothing to size and nothing to seed — see [`AimChips`].
    ///
    /// One field for two chips because they are one reading: both are drawn
    /// exactly when [`Pane::aimed`] is, and a state where one of them was there
    /// and the other was not is not a state this row has.
    pub aim: Option<AimChips>,
    /// Which deck this head belongs to, as [`Operation::SetSync`],
    /// [`Operation::ScrubDeck`] and [`Operation::SetCompositing`] each name
    /// one — [`Pane::deck`], carried so that a press answers with the deck it
    /// was measured for.
    pub deck: usize,
    /// **What the fold chip is showing**, and what a press names the other of
    /// — [`Pane::composite`] as it was read, carried for [`DeckHead::locked`]'s
    /// reason one field down: whoever measured this row and whoever acts on a
    /// press in it are one statement, so a chip cannot name a destination
    /// computed from a state some later frame read.
    pub composited: bool,
    /// **What the mode chip is showing**, and what re-anchoring re-asks for.
    /// Carried for [`LookRow::values`]' reason: whoever measured this row and
    /// whoever acts on a press in it are one statement.
    pub locked: Sync,
    /// **What this deck's material can honour**, [`Pane::allows`] as it was
    /// read — the whole of what the cycle skips on.
    pub allows: [bool; SYNCS.len()],
}

/// **The deck head's two build chips, laid out and with what a press on each
/// one asks for** — the capacity the slot's geometries run at, and the salt
/// its randomness comes from.
///
/// # The destinations are carried, for [`DeckHead::composited`]'s reason
///
/// Whoever measured this row and whoever acts on a press in it are one
/// statement. The step is arithmetic over [`Aimed::capacities`] and the salt is
/// a number the host handed in, and both are worked out **once**, on the frame
/// that laid the chips out — so a chip a hand pressed and the operation that
/// leaves this crate cannot be about two different readings of the slot.
///
/// # Neither says *step* and neither says *again*
///
/// What crosses into the vocabulary is
/// [`Operation::SetProperty`](karakuri_operation::Operation::SetProperty)
/// naming a number: `Property::Capacity` carries the element count the step
/// arrived at and `Property::Seed` carries the salt. The affordance —
/// *press it and it moves on* — is the surface's, which is
/// [`DeckHead::sync`]'s division and [`Mixer::blend`]'s
/// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AimChips {
    /// **The capacity chip**, reading [`Aimed::capacity`]. `.mini`'s box round
    /// the number, one gap left of [`AimChips::salt`].
    pub size: Rect,
    /// **Where a press on [`AimChips::size`] arrives**, or `None` where there
    /// is nothing to step to — see [`stepped_capacity`] and
    /// [`Aimed::capacities`].
    ///
    /// **`None` is drawn and not claimed**, which is `input`'s *a control
    /// claims what it acts on and no more* and the arrangement an inert scrub
    /// is already in: the number is still worth reading on a deck whose
    /// geometries share no range, and a press on it has nothing to ask for.
    pub resize: Option<u32>,
    /// **The `re-salt` capsule**, one gap left of [`DeckHead::composite`].
    pub salt: Rect,
    /// **The salt a press on it asks for** — [`Aimed::salt`], carried.
    ///
    /// There is no state in which this chip is drawn and inert: a slot with a
    /// geometry has randomness to re-seed, and a slot without one draws neither
    /// of these two.
    pub re_salt: u32,
}

impl DeckHead {
    /// Whether `p` is on the sync chip.
    pub fn hit_mode(&self, p: karakuri_layout::Point) -> bool {
        self.mode.contains(Pos2::new(p.x, p.y))
    }

    /// Whether `p` is on the anchor, which a free deck does not draw.
    pub fn hit_anchor(&self, p: karakuri_layout::Point) -> bool {
        self.anchor
            .is_some_and(|at| at.contains(Pos2::new(p.x, p.y)))
    }

    /// Whether `p` is on the fold at the right of the row.
    pub fn hit_composite(&self, p: karakuri_layout::Point) -> bool {
        self.composite.contains(Pos2::new(p.x, p.y))
    }

    /// **Whether `p` is on the capacity chip *and* the chip has somewhere to
    /// step**, which is [`DeckHead::arrow`]'s arrangement written for a chip:
    /// a deck whose geometries share no declared range draws the number and
    /// claims nothing.
    pub fn hit_size(&self, p: karakuri_layout::Point) -> bool {
        self.aim
            .is_some_and(|aim| aim.resize.is_some() && aim.size.contains(Pos2::new(p.x, p.y)))
    }

    /// **Whether `p` is on the `re-salt` capsule**, which is claimed wherever
    /// it is drawn.
    pub fn hit_salt(&self, p: karakuri_layout::Point) -> bool {
        self.aim
            .is_some_and(|aim| aim.salt.contains(Pos2::new(p.x, p.y)))
    }

    /// **Which arrow `p` is on, as the amount it asks for** — or `None` off
    /// both, and `None` on either while the scrub is inert.
    ///
    /// **Inert is not claimed**, which is [`crate::input`]'s *a control claims
    /// what it acts on and no more*, and it is why this answers for liveness
    /// as well as for position. The arrows are drawn on every deck, because a
    /// scrub is *"an offset added to the room's position under beat sync"* and
    /// a pair of chips that vanished on two modes out of three would move the
    /// rest of the row under the hand every time the chip beside them was
    /// pressed. Drawn and not claimed is the arrangement a fader's track is
    /// already in.
    fn arrow(&self, p: karakuri_layout::Point) -> Option<f64> {
        if self.locked != Sync::Beat {
            return None;
        }
        let p = Pos2::new(p.x, p.y);
        match (self.back.contains(p), self.forward.contains(p)) {
            (true, _) => Some(-SCRUB_BEATS),
            (_, true) => Some(SCRUB_BEATS),
            _ => None,
        }
    }

    /// **Whether `p` is on any of the six**, which is what
    /// [`crate::input::claim`] asks. The fold is one of them since 2026-09-09,
    /// and it is the only one of the six that is claimed on every deck: a
    /// sync chip is always live, an anchor is not drawn on a free deck, an
    /// arrow is not claimed off beat sync, and the two build chips are drawn
    /// only where the deck has a geometry — where a layering is a state every
    /// slot is in.
    pub fn owns(&self, p: karakuri_layout::Point) -> bool {
        self.hit_mode(p)
            || self.hit_anchor(p)
            || self.arrow(p).is_some()
            || self.hit_size(p)
            || self.hit_salt(p)
            || self.hit_composite(p)
    }

    /// **What a press at `p` asks this deck's clock to become**, or `None`
    /// where there is no sync chip under it.
    ///
    /// # The chip cycles, the operation names where it arrived, and the cycle
    /// skips
    ///
    /// Click it and the deck is asked for the next of [`SYNCS`] its material
    /// can honour — *free*, *tempo*, *beat*, wrapping — and what comes out is
    /// [`Operation::SetSync`] naming the **destination**, never a step,
    /// because there is no step in the vocabulary to name. The affordance is
    /// [`Mixer::blend`]'s and [`LookRow::tonemap`]'s exactly
    /// ([ADR-0187](../../../../docs/adr/0187-the-blend-mini-cycles-and-a-map-learns-the-three-it-cycles-through.md)),
    /// and so is the division it rests on: the cycle is [`next_sync`] here and
    /// nothing at all in `karakuri-operation`, which is P-0090's division: a
    /// toggle is an affordance, built over operations by whoever draws the
    /// control.
    ///
    /// **The skip is the one thing this cycle has that the other two do not**,
    /// and it is not a refusal: what may be asked for is the engine's
    /// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)),
    /// and this chip is choosing which of the destinations *it* offers to
    /// name, out of a reading somebody else took. A mode this material cannot
    /// honour is passed over rather than handed on to be refused, which is the
    /// mock's own *"skips a mode this material cannot honour instead of
    /// offering it"*.
    ///
    /// **What a map is offered is the three modes, not the cycle** — the
    /// sentence ADR-0187 wrote about three blend modes, and the reason the
    /// anchor beside this chip can be a fourth way to say one of them.
    pub fn sync(&self, p: karakuri_layout::Point) -> Option<Operation> {
        self.hit_mode(p).then(|| Operation::SetSync {
            deck: self.deck as u8,
            sync: next_sync(self.locked, self.allows),
        })
    }

    /// **What a press on the anchor asks for**: [`Operation::SetSync`] naming
    /// the mode this deck is **already in**, which re-anchors it — or `None`
    /// off the anchor, and `None` on a free deck, which draws none.
    ///
    /// # It is one operation asked for from two ends, and that is the record
    ///
    /// `karakuri_engine::transport::Transport::engage`'s own documentation is
    /// where this comes from: *"Re-engaging the mode a slot is already in
    /// re-anchors it, which is how an operator says 'call **this** the
    /// reference tempo' without a second control."* `Transport::engaged`
    /// recomputes the anchor from the session tempo every time and clears the
    /// scrub with it, so naming the mode that is running is a real move rather
    /// than a press that does nothing.
    ///
    /// **A cycle structurally cannot ask for it**, which is why this is a
    /// second target on the row rather than a second press on the first:
    /// [`next_sync`] starts at the mode *after* the one the deck is in, so the
    /// one state it can never arrive at is the state it is in. That is a fault
    /// of the affordance and not a gap in the vocabulary, and it is why there
    /// is no `ReAnchor` variant here to name — an operation whose meaning is
    /// *again* is the shape P-0090 rules out
    /// ([ADR-0218](../../../../docs/adr/0218-re-anchoring-is-set-sync-naming-the-mode-the-deck-is-in-and-a-cycle-cannot-say-it.md)).
    ///
    /// **A free deck has no anchor and loses nothing.** `Free` is the absence
    /// of a transport rather than a setting and reads no anchor at all, so
    /// there is nothing for a press to re-ask for — [`anchor_text`] draws
    /// nothing there and this answers `None` off the same `Option`.
    pub fn reanchor(&self, p: karakuri_layout::Point) -> Option<Operation> {
        self.hit_anchor(p).then_some(Operation::SetSync {
            deck: self.deck as u8,
            sync: self.locked,
        })
    }

    /// **What a press on an arrow asks for**: [`Operation::ScrubDeck`] moving
    /// this deck a [`SCRUB_BEATS`] back or forward — or `None` off both, and
    /// `None` wherever the scrub is inert.
    ///
    /// # The one control on this panel that moves by an amount
    ///
    /// Every other control here names a destination, which is P-0090's rule,
    /// and the vocabulary says at the variant why this one does not: *"it is
    /// relative because nothing in this instrument can set a position"*.
    /// Scrubbing moves closed-form material by an amount; accumulating
    /// material cannot be moved to a position at all, so an absolute
    /// `at_beat` would be an operation that does not exist for two thirds of
    /// the material. So this is not the exception to P-0090 it looks like —
    /// there is no destination in the language for it to name.
    ///
    /// **Signed and unbounded**, which is the deck head's own spelling: the
    /// offset this writes is drawn in the anchor beside it, in **beats** and
    /// for one deck, where the transport row's offset is in milliseconds and
    /// is the whole instrument's. Nothing clamps it here and there is nothing
    /// to clamp it to.
    ///
    /// **Inert under anything but beat sync**, because that is the only mode
    /// that reads the offset. The arrows keep their shape and the console page
    /// says why rather than greying them out: *"Nothing here re-runs a deck's
    /// history to place it."*
    ///
    /// # One press is one operation
    ///
    /// [ADR-0207](../../../../docs/adr/0207-a-continuous-control-says-one-thing-per-frame.md)
    /// coalesces a swept MIDI fader to one operation a frame; neither half of
    /// it applies to a press, and a second press in the same frame is a second
    /// quarter beat an operator asked for. That is the whole reason the amount
    /// is a constant rather than a distance along anything.
    pub fn scrub(&self, p: karakuri_layout::Point) -> Option<Operation> {
        self.arrow(p).map(|beats| Operation::ScrubDeck {
            deck: self.deck as u8,
            beats,
        })
    }

    /// **What a press on the fold asks for**: [`Operation::SetCompositing`]
    /// naming the layering this deck is **not** in — or `None` off the chip.
    ///
    /// # A destination and not a flip, on a chip that reads as a toggle
    ///
    /// [`Mixer::blend`]'s division and [`DeckHead::sync`]'s: the affordance is
    /// *press it and it changes*, and what leaves this crate is the state
    /// being asked for. Nothing in `karakuri-operation` says *toggle*, because
    /// two surfaces stepping one control disagree about where they are
    /// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)),
    /// and the destination is computed from [`DeckHead::composited`] — the
    /// state the frame that laid this row out drew — so the chip a hand
    /// pressed and the operation that leaves are one statement.
    ///
    /// # The press is a rebuild, and that is the mechanism rather than a cost
    ///
    /// `Set::merge` is written at `Set::build` and nothing moves it
    /// afterwards, which read for a year as *the engine has no setter for
    /// this, so the control is blocked*. What it actually means is that the
    /// control is not a write at all: the layering is one field of the
    /// description a slot's watcher is pointed at, so the window that acts on
    /// this operation restates the rest of that description with this field
    /// changed and sends it, and the worker rebuilds the slot off the render
    /// thread. That build lands at a frame boundary and is judged there and
    /// then, on what one frame of that Set was measured to cost, exactly as an
    /// edited file and a library load are, and rolls itself back if it cannot
    /// hold the frame — which is
    /// the point rather than the price, because compositing costs a
    /// frame-sized target per renderer
    /// ([P-0091](../../../../docs/principles/0091-cost-is-known-before-it-is-paid.md),
    /// [P-0085](../../../../docs/principles/0085-take-the-mechanism-that-exists-and-pay-the-bill-now.md),
    /// [ADR-0314](../../../../docs/adr/0314-a-control-that-moves-a-field-of-the-aim-re-aims-the-slot-and-the-rebuild-is-the-write.md)).
    ///
    /// **Nothing here knows any of that**, and this crate could not: it names
    /// a destination and a deck, and where the rebuild happens is the window's
    /// (ADR-0156). What it does owe is that the chip goes on reading what
    /// *landed* rather than what was asked for, which is
    /// [`Pane::composite`]'s own note.
    pub fn compositing(&self, p: karakuri_layout::Point) -> Option<Operation> {
        self.hit_composite(p).then_some(Operation::SetCompositing {
            deck: self.deck as u8,
            compositing: !self.composited,
        })
    }

    /// **What a press on the capacity chip asks for**:
    /// [`Operation::SetProperty`] naming the element count the step arrived at
    /// — or `None` off the chip, and `None` on a chip with nowhere to step.
    ///
    /// # A number a hand should not drag, so the control steps
    ///
    /// A capacity is a number in a declared range, which everywhere else on
    /// this console is [`Param`]'s track — and the mock's own reading of a
    /// Set says so, *"capacity included, because a procedure declares one the
    /// same way it declares a knob"*. It is refused here by what a drag **is**:
    /// [`ParamGrip`] turns a pointer into a value every frame it moves, which
    /// for a parameter is a uniform write and for a capacity is a full rebuild
    /// of the slot with every element buffer in it reallocated — dozens of them
    /// across one gesture, which is
    /// [P-0091](../../../../docs/principles/0091-cost-is-known-before-it-is-paid.md)
    /// at its widest. So the affordance is the Library filters' one bay over
    /// ([ADR-0262](../../../../docs/adr/0262-a-library-filter-field-steps-through-what-the-store-already-holds-rather-than-taking-letters.md)):
    /// the field steps a closed list and the operation names where it arrived.
    ///
    /// **What it steps is not this console's list.** The powers of two inside a
    /// declared range belong to the material in the slot, so they arrive as
    /// [`Aimed::capacities`] the way `holds` arrives as [`View::holds`], and
    /// what is offered is a reading somebody else took (P-0090).
    ///
    /// # The press is a rebuild, and it is the fold's mechanism exactly
    ///
    /// The capacity is one field of the description this slot's watcher is
    /// pointed at, so the window restates the rest and sends it and the worker
    /// recompiles the slot off the render thread, judged at a frame boundary
    /// against what one frame of that Set was measured to cost — see
    /// [`DeckHead::compositing`], where the argument is written out, and
    /// `docs/adr/0328-…`. **Nothing here knows any of that** and this crate
    /// could not: it names a number and a deck.
    pub fn resized(&self, p: karakuri_layout::Point) -> Option<Operation> {
        let aim = self.aim?;
        let elements = aim.resize?;
        aim.size
            .contains(Pos2::new(p.x, p.y))
            .then_some(Operation::SetProperty {
                deck: self.deck as u8,
                property: karakuri_operation::Property::Capacity { elements },
            })
    }

    /// **What a press on the `re-salt` capsule asks for**:
    /// [`Operation::SetProperty`] naming the salt this slot's randomness is to
    /// come from — or `None` off the capsule.
    ///
    /// # The number is handed in, and that is the whole of the decision
    ///
    /// A salt is the one payload on this row that could plausibly be *made up*,
    /// and a console that made one up would be a surface producing a picture no
    /// later run could produce again
    /// ([P-0092](../../../../docs/principles/0092-the-same-inputs-produce-the-same-frame.md)).
    /// [`Aimed::salt`] is the next value of the slot's own deterministic
    /// sequence, derived by whoever read the Set from the salt the slot is
    /// actually running — so this names a destination like every other control
    /// here, the same press twice from the same place lands on the same two
    /// pictures, and a Set kept afterwards records the salts it was running at.
    ///
    /// **Nothing is refused here.** Asking for the salt a slot is already on is
    /// not a state this capsule can produce — the sequence goes forward — and a
    /// re-seed changes the picture, so unlike the fold beside it there is no
    /// press that buys a recompile and moves nothing.
    pub fn re_salted(&self, p: karakuri_layout::Point) -> Option<Operation> {
        let aim = self.aim?;
        aim.salt
            .contains(Pos2::new(p.x, p.y))
            .then_some(Operation::SetProperty {
                deck: self.deck as u8,
                property: karakuri_operation::Property::Seed { salt: aim.re_salt },
            })
    }
}

/// **The deck head's controls, derived** — the chips in
/// [`InspectorPane::deck_head`], laid along it the way `.deck-head`'s flex row
/// lays them.
///
/// # Measured off a laid-out pane rather than off the layout
///
/// [`inspector`] answers where the pane is and this answers where the chips in
/// its second row are: two questions about one laid-out pane, which is exactly
/// what the mixer bay's four controls are about one laid-out strip. A second
/// derivation from the layout would be a second answer that could disagree
/// with the one the frame drew.
///
/// # What it costs to ask
///
/// **Five galley lookups per pane**: the mode's word, the anchor's two
/// numbers, the capacity's digits, the `re-salt` capsule's word and the fold's.
/// The two arrows cost none — they are marks rather than words, which is
/// [`Mixer::mask`]'s own saving one bay over — and nothing here asks after the
/// node groups below.
///
/// **It is asked twice on a frame**, once here and once for the galleys
/// [`deck_head_into`] paints, which is [`look`]'s honest cost written down one
/// bay along: the derivation that draws a control is the one that hit-tests
/// it, so a control cannot be painted anywhere a press cannot reach. Two panes
/// at three lookups is on the order of ten allocations a frame against the
/// panel pass's measured median of 1518 (`crates/karakuri`'s `WRITTEN_ALLOCS`,
/// taken 2026-08-31), which is inside the factor of two that file quotes a
/// figure across.
///
/// # `None` is a row that cannot hold its own controls
///
/// [`look`]'s rule and [`arrangement`]'s: *a control that does not fit in the
/// row it is drawn in is no control at all, rather than half of one*. The fold
/// is pushed against the right-hand padding and the three controls run from
/// the left, so what says the row fits is that the arrows end before the fold
/// begins. A pane narrow enough to fail that draws its two heads' words and no
/// chips at all, where it used to draw chips cut in half by
/// [`inspector_into`]'s clip rectangle — which is a picture of a control that
/// cannot be pressed
/// ([P-0094](../../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)).
pub fn deck_head(ctx: &egui::Context, at: &InspectorPane, pane: &Pane) -> Option<DeckHead> {
    // Fonts are not valid until `egui` has run a pass, exactly as in
    // [`transport`], [`outputs`] and [`mixer`] — and on the frame before the
    // first one there is nothing drawn here to press.
    if ctx.cumulative_pass_nr() == 0 {
        return None;
    }
    let row = at.deck_head;
    let mid = row.center().y;
    let width = |text: &str, size: f32| {
        ctx.fonts_mut(|f| {
            f.layout_no_wrap(
                text.to_owned(),
                FontId::new(size, FontFamily::Proportional),
                Color32::PLACEHOLDER,
            )
            .size()
            .x
        })
    };
    // `.mini`'s box, which is `mini_word`'s arithmetic: the padding either
    // side of the word, with no border counted, because that is what this row
    // has always been drawn with and this is the same chip.
    let mini = |text: &str, x: f32| {
        Rect::from_min_size(
            Pos2::new(x, mid - size::MINI_H * 0.5),
            egui::vec2(
                width(text, size::MINI_SIZE) + size::MINI_PAD_X * 2.0,
                size::MINI_H,
            ),
        )
    };

    let mode = mini(sync_word(pane.sync), row.min.x + size::DECK_HEAD_PAD_X);
    let anchor = anchor_text(pane).map(|text| {
        Rect::from_min_size(
            Pos2::new(mode.max.x + size::DECK_HEAD_GAP, mid - size::MINI_H * 0.5),
            egui::vec2(width(&text, size::ANCHOR_SIZE), size::MINI_H),
        )
    });
    // One `.deck-head` gap after whichever of the two came last — a free deck
    // draws no anchor, and a flex row closes up rather than leaving a hole
    // where one would have been.
    let arrows = anchor.map_or(mode.max.x, |at| at.max.x) + size::DECK_HEAD_GAP;
    // `.scrub i`'s box: the mark is as wide as the glyph it stands in for,
    // which is [`Mixer::mask`]'s rule, inside its own padding and its border.
    let arrow_w = size::SCRUB_SIZE + size::SCRUB_PAD_X * 2.0 + size::HAIRLINE * 2.0;
    let arrow = |x: f32| {
        Rect::from_min_size(
            Pos2::new(x, mid - size::SCRUB_H * 0.5),
            egui::vec2(arrow_w, size::SCRUB_H),
        )
    };
    let back = arrow(arrows);
    let forward = arrow(back.max.x + size::SCRUB_GAP);

    // `.sep`'s `flex: 1` puts the fold hard against the right of the row, and
    // the two build chips are measured leftwards from it — a flex row's running
    // sum taken from the other end, which is what everything after the `.sep`
    // is.
    let fold = mini(COMPOSITE_LABEL, row.min.x);
    let composite = mini(
        COMPOSITE_LABEL,
        row.max.x - size::DECK_HEAD_PAD_X - fold.width(),
    );
    // **Both or neither**, which is [`DeckHead::aim`]'s own sentence: they come
    // from one reading, so a pane with no [`Pane::aimed`] draws the row it drew
    // before this control existed.
    //
    // **And neither where the row cannot hold them**, which is the one place
    // this row's *a control that does not fit is no control at all* is answered
    // by dropping part of the row rather than all of it. The reason is a
    // measurement: an Inspector pane at the console's declared minimum window
    // is 237 pixels wide and the five chips that were here already come to
    // within a couple of dozen of that, so a row that took all seven or none
    // would answer *none* at the width this arrangement claims to work at —
    // trading two controls that were never there for four that were. So the two
    // build chips are dropped first and the row goes on drawing what it drew
    // before them, and the page says so rather than leaving an operator to
    // discover it by dragging
    // ([P-0094](../../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)).
    let aim = pane.aimed.as_ref().and_then(|aimed| {
        let word = aimed.capacity.to_string();
        let width_of = |text: &str| mini(text, row.min.x).width();
        let salt = mini(
            RE_SALT_LABEL,
            composite.min.x - size::DECK_HEAD_GAP - width_of(RE_SALT_LABEL),
        );
        let size_at = mini(&word, salt.min.x - size::DECK_HEAD_GAP - width_of(&word));
        let fits =
            row.contains_rect(size_at) && forward.max.x + size::DECK_HEAD_GAP <= size_at.min.x;
        fits.then_some(AimChips {
            size: size_at,
            resize: stepped_capacity(&aimed.capacities, aimed.capacity),
            salt,
            re_salt: aimed.salt,
        })
    });

    if !row.contains_rect(mode)
        || !row.contains_rect(composite)
        || forward.max.x + size::DECK_HEAD_GAP > composite.min.x
    {
        return None;
    }

    Some(DeckHead {
        mode,
        anchor,
        back,
        forward,
        composite,
        aim,
        deck: pane.deck,
        composited: pane.composite,
        locked: pane.sync,
        allows: pane.allows,
    })
}

/// **The `keep` pill in a pane's head, laid out** — the capsule at the right
/// of `.half-head`, and the one control in this bay that performs rather than
/// sets.
///
/// # It keeps the pane's deck, and `k` keeps the selection
///
/// The mock draws one of these per pane and the tooltip names the pane's own
/// deck: *"Keep deck A as a Set, exactly as it is on screen."* So this carries
/// [`Pane::deck`] the way [`DeckHead::deck`] does, and a press answers with
/// the deck the pill was measured for — a pill in the second pane keeps that
/// pane's deck while the selection stays where the operator put it. The key
/// `k` keeps *the selected deck*, because a bare key press cannot say which,
/// and the two are one operation asked for from two ends
/// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
///
/// # What it files it under
///
/// [`Operation::SaveSet`] with **no id**, which is the same call the key makes
/// and is a decision rather than an omission
/// ([ADR-0287](../../../../docs/adr/0287-the-keep-pill-files-under-a-stamp-because-the-consoles-one-letter-taking-flow-is-an-arrangements-name.md)).
/// What the store does with a `None` is `karakuri_environment::accepted_save`'s
/// convention — a stamp, because *"an operator looks for the time they saved
/// it"*.
///
/// **The reason has changed and the decision has not.** ADR-0287 argued the
/// `None` from there being one letter-taking flow on this console and it being
/// an arrangement's; there are two now, and the second is the name in the head
/// beside this capsule
/// ([ADR-0292](../../../../docs/adr/0292-the-pane-heads-name-takes-letters-and-the-keep-capsule-stays-a-stamp.md)).
/// What holds the capsule at `None` from here on is
/// [ADR-0128](../../../../docs/adr/0128-a-set-saved-under-a-name-the-caller-chose-overwrites.md)
/// rather than the absence of a field: *"an operator's own act gets the name it
/// asked for; a key press cannot type one and takes a stamp"*. This is the
/// press that types nothing, so this is the one that takes the stamp — see
/// [`DeckName`] for the one that does not.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KeepPill {
    /// **The capsule**, which is what a press has to land in. The mock gives
    /// the whole pill the click and so does this — [`ArrangementPill::pill`]'s
    /// own reading.
    pub pill: Rect,
    /// Which deck this pill keeps, as [`Pane::deck`] — carried so that a press
    /// answers with the deck it was measured for, which is
    /// [`DeckHead::deck`]'s reason one row down.
    pub deck: usize,
}

impl KeepPill {
    /// Whether `p` is on the capsule, which is the whole of what this control
    /// owns: there is no menu under it and no second target beside it.
    pub fn hit(&self, p: karakuri_layout::Point) -> bool {
        self.pill.contains(Pos2::new(p.x, p.y))
    }

    /// **What a press at `p` asks for**, or `None` off the capsule.
    ///
    /// The same derivation [`crate::input::claim`] hit-tests, asked a second
    /// time rather than copied — [`DeckHead::sync`]'s arrangement, and the
    /// reason is the same one row up: the pill that claims a press and the
    /// pill that acts on it cannot come apart.
    ///
    /// **It refuses nothing.** What a keep costs and whether the store will
    /// take it are the instrument's answers rather than this surface's, and
    /// the operation is *"on a worker"* on the page it is specified on — the
    /// press leaves and the answer arrives later.
    pub fn keep(&self, p: karakuri_layout::Point) -> Option<Operation> {
        self.hit(p).then_some(Operation::SaveSet {
            deck: self.deck as u8,
            // **`None`, and it is the payload saying so rather than this
            // control inventing a stamp** — the sentence
            // `crates/karakuri/src/main.rs` already writes over the `k` arm,
            // and the same one: *"a caller that can type a name is not made to
            // take a timestamp"*, and this control is not one of them.
            id: None,
        })
    }
}

/// **The pane head's pill, derived** — [`inspector`] answers where the head is
/// and this answers where the capsule in it is, which is [`deck_head`]'s
/// division one row down.
///
/// `.sep`'s `flex: 1` puts it hard against the head's right-hand padding, and
/// **one padding down from the top rather than centred in the row**: the rule
/// at the bottom is inside `.half-head`, so the row's middle is half a pixel
/// below the middle of its content box — which is the scope row's own note one
/// bay along, on a row built the same way.
///
/// # What it costs to ask
///
/// **One galley lookup per pane**, for the word in the capsule, on a pointer
/// event and on a frame — [`deck_head`]'s three beside it, and paid the same
/// way.
///
/// # `None` is a head that cannot hold it
///
/// [`deck_head`]'s rule and [`look`]'s: *a control that does not fit in the
/// row it is drawn in is no control at all, rather than half of one*. The
/// words to its left are a readout and are clipped; the pill is a target and
/// is not drawn where it would be cut.
pub fn keep_pill(ctx: &egui::Context, at: &InspectorPane, pane: &Pane) -> Option<KeepPill> {
    // Fonts are not valid until `egui` has run a pass, exactly as in
    // [`deck_head`] — and on the frame before the first one there is nothing
    // drawn here to press.
    if ctx.cumulative_pass_nr() == 0 {
        return None;
    }
    let head = at.head;
    let w = pill_width(ctx, KEEP_LABEL);
    let pill = Rect::from_min_size(
        Pos2::new(
            head.max.x - size::HALF_HEAD_PAD_X - w,
            head.min.y + size::HALF_HEAD_PAD_Y,
        ),
        egui::vec2(w, size::PILL_H),
    );
    // **Measured against `.half-head`'s content box and not against the row**,
    // because a flex item cannot be laid out inside its parent's padding: the
    // capsule is placed from the right-hand padding, so what it runs off is
    // the left one, and a head with less room between its two paddings than
    // the word needs draws none. [`positive`] is what says the head is a row
    // at all — a pane with no height has one that is not.
    let room = head.width() - size::HALF_HEAD_PAD_X * 2.0;
    (positive(head) && pill.width() <= room).then_some(KeepPill {
        pill,
        deck: pane.deck,
    })
}

/// **The word in the capsule**, which is the mock's own and is the row's name
/// in the panel column of [every operation](../../../../docs/manual/operations.html).
const KEEP_LABEL: &str = "keep";

/// **The letter of the deck a pane is pointed at**, or `?` for a pane pointed
/// past the end of [`DECK_LETTERS`] — which is a caller's error and not a
/// state, and is drawn rather than panicked for [`showing_text`]'s reason: a
/// head is a readout and a readout does not stop a frame.
fn deck_letter(pane: &Pane) -> &'static str {
    DECK_LETTERS.get(pane.deck).copied().unwrap_or("?")
}

/// **What the pane head reads**: the mock's `deck A · drift_night`.
fn showing_text(pane: &Pane) -> String {
    format!("deck {} · {}", deck_letter(pane), pane.material)
}

/// **What that same run reads while the head is taking letters** — `deck A ·
/// glass_sh▏`, with [`CARET`] after it as the arrangement's field has.
///
/// **The deck stays and the material goes.** What is being typed is the name
/// this deck's material will be filed under, so the run says which deck is
/// being filed for the whole of the gesture — and the half of it that is
/// replaced is exactly the half a name is. A field that had cleared the run
/// would take the one word that says *whose* name this is off the screen at
/// the moment an operator is looking hardest at it.
fn naming_text_in_head(pane: &Pane, typed: &str) -> String {
    format!("deck {} · {typed}{CARET}", deck_letter(pane))
}

/// The word the mock puts in front of it.
const SHOWING_LABEL: &str = "showing";

/// **The word in front of the run while the head is taking letters**, where
/// [`SHOWING_LABEL`] is the word in front of it the rest of the time.
///
/// The row stops being a readout the moment letters are going into it, and the
/// label is the only thing that can say what they are *for*: they name the Set
/// the capsule at the other end of the same row files. It is [`KEEP_LABEL`]'s
/// own word rather than a new one, which is [`SAVE_ITEM_ASKING`]'s arrangement
/// three bays along — the thing that asks for something says so in the verb it
/// is about to perform.
const NAMING_LABEL: &str = "keep as";

/// **The word this head has in front of its run**, which is the one thing
/// about the row that says whether it is reading or asking.
fn head_label(naming: Option<&str>) -> &'static str {
    match naming {
        Some(_) => NAMING_LABEL,
        None => SHOWING_LABEL,
    }
}

/// **The name in a pane head, laid out** — the mock's `.what`, and this
/// console's **second** letter-taking flow
/// ([ADR-0292](../../../../docs/adr/0292-the-pane-heads-name-takes-letters-and-the-keep-capsule-stays-a-stamp.md)).
///
/// # A press on it names the Set, and the capsule beside it goes on stamping
///
/// [ADR-0128](../../../../docs/adr/0128-a-set-saved-under-a-name-the-caller-chose-overwrites.md)
/// is what puts two routes on one row: *"an operator's own act gets the name it
/// asked for; a key press cannot type one and takes a stamp"*. The `keep`
/// capsule is the second of those and is unchanged — [`KeepPill::keep`] emits
/// `id: None` exactly as ADR-0287 decided — and this is the first: a press
/// here puts the head into [`Naming`], and the commit is
/// [`View::named_set`]'s `id: Some(typed)`.
///
/// # What it does **not** claim, and that is the whole of its right-hand edge
///
/// The mock's head is `showing`, the name, `▾`, `.sep`, `keep`. **The `▾` is
/// the chooser** — *point this pane at another deck* — which is
/// [`Pane::deck`]'s per-pane pointer and is still not a control this console
/// has (ADR-0200). It is not drawn, and this derivation reserves
/// [`DeckName::chevron`] for it anyway: the target is the run's own ink and
/// stops there, so the day the chooser lands it takes the rectangle beside the
/// name rather than taking it *back*. A name target that had run to the
/// capsule would have swallowed the chooser's place before anybody drew it,
/// and a press meant for the caret would be a press that re-points the pane.
///
/// # The run is one target and is deliberately not two
///
/// `deck A · drift_night` is one `.what` in the mock and one galley here.
/// Claiming the material and leaving `deck A ·` a readout would be a boundary
/// inside a run of text with nothing on screen drawing it, which is the
/// opposite of *a control claims what it acts on and no more*: what this acts
/// on is the name display, and the name display is the whole run.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DeckName {
    /// **The run as it is painted**, clipped to what the head has room for —
    /// see [`deck_name`]. A press has to land in this and nowhere else.
    pub name: Rect,
    /// **Where the mock's `▾` goes**, one `.half-head` gap after the run.
    /// Drawn by nobody and claimed by nobody: it is the chooser's place, held
    /// so that this control's edge is a measured thing rather than a comment.
    pub chevron: Rect,
    /// Which deck this head names, as [`Pane::deck`] — carried for
    /// [`KeepPill::deck`]'s reason one capsule along.
    pub deck: usize,
}

impl DeckName {
    /// Whether `p` is on the run, which is the whole of what this control
    /// owns: the label to its left is a readout, and the rectangle to its
    /// right is the chooser's.
    pub fn hit(&self, p: karakuri_layout::Point) -> bool {
        self.name.contains(Pos2::new(p.x, p.y))
    }
}

/// **What the count at the right of a pane head reads** — `n of m`, the node
/// groups this pane is showing whole out of the ones the deck has.
///
/// The Library foot's `5 of 27` counted on this bay's items rather than on
/// that one's rows, which is what
/// [ADR-0259](../../../../docs/adr/0259-the-keyboard-is-addressed-to-the-bay-that-has-focus-and-a-global-letter-is-a-convenience-or-the-operators-own.md)
/// says a pane's items are: *"Items are its panes; a pane's controls are its
/// deck head and its node groups"*. A percentage would be a number about a
/// rectangle, and what an operator counts is groups.
pub fn count_text(at: &InspectorPane, pane: &Pane) -> String {
    format!("{} of {}", at.shown, pane.nodes.len())
}

/// **The count in a pane head, derived** — where the run goes, or `None` for a
/// head with no room for it between the label and the capsule.
///
/// It is a **readout**: nothing hit-tests it, it names no operation, and it
/// carries no row on [every operation](../../../../docs/manual/operations.html) —
/// the mixer head's `3 of 3 · page 1` one bay along, and the reason is the
/// same one that keeps the Library's cursor off that page. What it is *for* is
/// rule 04 — *"A list that showed you part of itself says so and says how
/// much"* — which is the whole of what a scrolled pane owes a reader, and is
/// why this is derived beside the two controls in the row rather than painted
/// wherever there happened to be space.
///
/// # Where it sits, and what gives way to what
///
/// `.half-head` is a flex row: the label and the run are at the left, `.sep`
/// takes what is over, and the capsule is hard against the right-hand padding.
/// This goes one [`size::HALF_HEAD_GAP`] to the left of the capsule — the
/// mixer head's order, where the readout is left of the pill — and the run to
/// its left is what gives way when the pane is narrowed, because the run is
/// the one thing in the row that is clipped rather than dropped.
///
/// **`None` is a head that cannot hold it**, which is [`keep_pill`]'s rule
/// read on a readout: measured against the room between the label and the
/// capsule, so a head that would have to draw this over the words draws none
/// of it. A pane at the declared minimum of 208 has room for all three
/// ([ADR-0279](../../../../docs/adr/0279-the-centre-is-two-parameter-rows-wide-because-a-pane-that-cannot-draw-a-fader-is-not-a-minimum.md)),
/// so this `None` is a pane below what the arrangement admits rather than a
/// state rule 04 is broken in.
pub fn pane_count(
    ctx: &egui::Context,
    at: &InspectorPane,
    pane: &Pane,
    naming: Option<&str>,
) -> Option<Rect> {
    // Fonts are not valid until `egui` has run a pass — [`keep_pill`]'s guard,
    // and before the first one there is no head painted to read.
    if ctx.cumulative_pass_nr() == 0 {
        return None;
    }
    let head = at.head;
    if !positive(head) {
        return None;
    }
    let run = |text: &str| {
        ctx.fonts_mut(|f| {
            f.layout_no_wrap(
                text.to_owned(),
                FontId::new(size::BASE, FontFamily::Proportional),
                Color32::PLACEHOLDER,
            )
            .size()
            .x
        })
    };
    let right = match keep_pill(ctx, at, pane) {
        Some(pill) => pill.pill.min.x - size::HALF_HEAD_GAP,
        None => head.max.x - size::HALF_HEAD_PAD_X,
    };
    // Where the run beside it would start: the label inside the left padding,
    // and one gap. This is measured against that rather than against the
    // head's edge so that a head narrow enough to want the room for its words
    // keeps it — the words are what says *which deck*, and a count of groups
    // on a pane whose deck has gone unnamed is a number about nothing.
    let left = head.min.x + size::HALF_HEAD_PAD_X + run(head_label(naming)) + size::HALF_HEAD_GAP;
    let w = run(&count_text(at, pane));
    let top = head.min.y + size::HALF_HEAD_PAD_Y;
    (right - w >= left).then(|| {
        Rect::from_min_max(
            Pos2::new(right - w, top),
            Pos2::new(right, top + size::PILL_H),
        )
    })
}

/// **The pane head's name, derived** — [`inspector`] answers where the head is
/// and this answers where the run in it is, which is [`keep_pill`]'s division
/// along the same row.
///
/// `naming` is what the head is taking letters into, or `None` for a head that
/// is reading — and it is a parameter rather than a field of [`Pane`] because
/// a pane is rewritten whenever a Set lands ([`View::inspector`]) and a buffer
/// kept there would be a name that vanished mid-word. It lives in
/// [`View::naming_set`], which is [`Arrangement::menu`]'s argument on a second
/// control: what a *control* is doing is this crate's, and it is not part of
/// anything a host hands in.
///
/// # What it costs to ask
///
/// **Three galley lookups per pane** — the label, the run, and [`keep_pill`]'s
/// word, because where the run may be painted to is where the capsule starts.
/// The capsule is derived here rather than passed in for [`crate::input`]'s
/// own reason one bay along, where [`on_pill`](crate::input) derives the
/// tracker group as well: one derivation asked twice cannot come apart, and
/// two arguments that a caller could fill from two frames can.
///
/// # `None` is a head with no ink to press
///
/// [`keep_pill`]'s rule read on a readout instead of on a capsule. The run is
/// clipped where the words are clipped — one `.half-head` gap short of the
/// capsule — so a head narrow enough that the label alone fills it leaves no
/// name on screen, and a target over ink nobody can see is a press that lands
/// on nothing an operator could have aimed at.
pub fn deck_name(
    ctx: &egui::Context,
    at: &InspectorPane,
    pane: &Pane,
    naming: Option<&str>,
) -> Option<DeckName> {
    // Fonts are not valid until `egui` has run a pass, exactly as in
    // [`keep_pill`] — and on the frame before the first one there is nothing
    // drawn here to press.
    if ctx.cumulative_pass_nr() == 0 {
        return None;
    }
    let head = at.head;
    if !positive(head) {
        return None;
    }
    let run = |text: &str| {
        ctx.fonts_mut(|f| {
            f.layout_no_wrap(
                text.to_owned(),
                FontId::new(size::BASE, FontFamily::Proportional),
                Color32::PLACEHOLDER,
            )
            .size()
            .x
        })
    };
    // **The same clip [`inspector_into`] paints the words inside**, written
    // once here and read there: everything up to whatever is next along the
    // row, one `.half-head` gap short of it. That is the count where the head
    // has room for one ([`pane_count`]), the capsule where it has not, and the
    // head's own edge where it has neither.
    let limit = match pane_count(ctx, at, pane, naming) {
        Some(count) => count.min.x - size::HALF_HEAD_GAP,
        None => match keep_pill(ctx, at, pane) {
            Some(pill) => pill.pill.min.x - size::HALF_HEAD_GAP,
            None => head.max.x,
        },
    };
    // **And the chooser's own room comes off it**, which is the half of
    // ADR-0292's *the chooser is boxed in* that the chooser landing makes
    // real: `.half-head` is `showing`, the run, `▾`, `.sep`, the count and the
    // capsule, so the `▾` sits **between** the run and everything else in the
    // row. The run is the one thing here that is clipped rather than dropped
    // (`pane_count`'s own note), so it is the run that gives way and never the
    // control. Before this the chevron was reserved and unpainted, and its
    // rectangle could sit on top of the count in a narrow head — which cost
    // nothing while nobody drew it and would be a target over another
    // control's ink now that somebody does.
    let limit = limit - (CHEVRON_W + size::HALF_HEAD_GAP);
    let left = head.min.x + size::HALF_HEAD_PAD_X + run(head_label(naming)) + size::HALF_HEAD_GAP;
    let text = match naming {
        Some(typed) => naming_text_in_head(pane, typed),
        None => showing_text(pane),
    };
    let right = (left + run(&text)).min(limit);
    if right <= left {
        return None;
    }
    let top = head.min.y + size::HALF_HEAD_PAD_Y;
    let name = Rect::from_min_max(Pos2::new(left, top), Pos2::new(right, top + size::PILL_H));
    Some(DeckName {
        // **The chooser**, one gap after the run and at the glyph's own
        // measure — [`CHEVRON_W`], which is the arrangement pill's `▾` three
        // bays along. It was reserved and drawn by nobody until 2026-09-10
        // (ADR-0292's *the chooser is boxed in*), and it is a control now:
        // [`pane_target`] is what paints and hit-tests it, off this
        // rectangle. What has not changed is that [`DeckName::name`] stops
        // before it — the run is one target and the mark beside it is
        // another.
        chevron: Rect::from_min_size(
            Pos2::new(
                name.max.x + size::HALF_HEAD_GAP,
                name.center().y - CHEVRON_H * 0.5,
            ),
            egui::vec2(CHEVRON_W, CHEVRON_H),
        ),
        name,
        deck: pane.deck,
    })
}

/// **The pulldown on a pane head, and the card it brings down** — *point this
/// pane at another deck*.
///
/// # It is the pane's own pointer and it is not the deck selection
///
/// A pick moves this pane and nothing else: not the deck the keys are
/// addressed to ([`View::selection`]), not the pane next door, and not the
/// Library bay's load target ([`View::target_deck`]). That is the whole of why
/// the mark exists — a pane can show a deck the keys are **not** on — and it
/// is [`Load`]'s argument one bay along
/// (`docs/adr/0305-…`, `docs/adr/0338-…`, decision 5).
///
/// # A pulldown and not a flip
///
/// The maintainer's choice, and
/// [P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)
/// underneath it: a flip is a *step*, so two panes stepping cannot both be
/// aimed without knowing where they started, and a key, a map line or a model
/// would have to count presses to say *deck C*. Every row of this card names a
/// destination.
///
/// # What it offers is what the mixer is drawing
///
/// [`Target::decks`]' count read a second time and not a second rule: a deck
/// the mixer draws no strip for is not in the list, which is
/// [`View::select`]'s own refusal met from one more direction.
///
/// # The card hangs down, as the `uses` line's does
///
/// It is inside a pane's body's own bay rather than in a foot, so what is
/// under the head is the pane — [`UsesLine::list`]'s division, and it is held
/// inside the viewport for that method's reason.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PaneTarget {
    /// **The `▾` after the run** — [`DeckName::chevron`], made live. A press on
    /// it puts the card down; a press on it while the card is down is the
    /// host's to read as *shut it*, which is [`Load`]'s arrangement.
    pub chevron: Rect,
    /// **Which pane this head belongs to**, as an index into [`PANE_NAMES`] —
    /// what [`Operation::PointPane`]'s `pane` is spelled from, and what says
    /// which of [`View::pane_deck`]'s entries a pick moves.
    pub pane: usize,
    /// **How many decks the card offers**, which is how many strips the mixer
    /// is drawing while it is down and **zero** while it is shut —
    /// [`Load::rows`]' shape and its reason: [`PaneTarget::row`] cannot hand
    /// out a rectangle for a card nobody opened.
    pub rows: usize,
}

impl PaneTarget {
    /// Whether `p` is on the mark, which is the whole of what the shut control
    /// owns: the run to its left is [`DeckName`]'s and the count to its right
    /// is a readout.
    pub fn hit(&self, p: karakuri_layout::Point) -> bool {
        self.chevron.contains(Pos2::new(p.x, p.y))
    }

    /// **The card under the mark, or `None` while it is shut** — and `None`
    /// for a console with no strip to offer, which is every test in this crate
    /// that hands no mixer in.
    pub fn list(&self, viewport: Rect) -> Option<Rect> {
        if self.rows == 0 {
            return None;
        }
        let height = size::LIB_LIST_PAD * 2.0 + size::LIB_ROW_H * self.rows as f32;
        let width = size::LIB_ROW_H + (size::LIB_ROW_PAD_X + size::LIB_LIST_PAD) * 2.0;
        Some(held_inside(
            &viewport,
            self.chevron.min.x,
            self.chevron.max.y + size::PILL_GAP,
            width,
            height,
        ))
    }

    /// **Where the `index`th deck's row is**, from the top of `card` — the
    /// decks in [`DECK_LETTERS`] order, which is [`Load::row`]'s own reading.
    ///
    /// Panics on a row this card has not got, which is that method's rule: a
    /// caller has invented a deck.
    pub fn row(&self, card: Rect, index: usize) -> Rect {
        assert!(index < self.rows, "deck {index} of a list of {}", self.rows);
        Rect::from_min_size(
            Pos2::new(
                card.min.x + size::LIB_LIST_PAD,
                card.min.y + size::LIB_LIST_PAD + size::LIB_ROW_H * index as f32,
            ),
            egui::vec2(card.width() - size::LIB_LIST_PAD * 2.0, size::LIB_ROW_H),
        )
    }

    /// **What a press at `p` on the card asks for**, or `None` off every row.
    ///
    /// The pane is named by [`PANE_NAMES`], which is the arrangement's own
    /// handle for it — `karakuri-operation` has no dependencies and cannot
    /// hold one, which is [`Operation::FoldPane`]'s spelling and its reason.
    pub fn picked(&self, viewport: Rect, p: karakuri_layout::Point) -> Option<Operation> {
        let card = self.list(viewport)?;
        let at = Pos2::new(p.x, p.y);
        let deck = (0..self.rows).find(|index| self.row(card, *index).contains(at))?;
        Some(Operation::PointPane {
            pane: PANE_NAMES.get(self.pane)?.to_string(),
            deck: deck as u8,
        })
    }
}

/// **The pulldown on one pane head, derived** — [`deck_name`] answers where
/// the run is and this answers where the mark after it is, which is
/// [`keep_pill`]'s division along the same row.
///
/// `None` is a head with no run drawn in it, which is [`deck_name`]'s own
/// refusal: the mark sits one gap after the run, so a head too narrow to paint
/// any of the name has nowhere to put it. The run is clipped short of this
/// mark rather than over it — see [`deck_name`], where that is one line.
pub fn pane_target(
    ctx: &egui::Context,
    at: &InspectorPane,
    pane: &Pane,
    index: usize,
    naming: Option<&str>,
    decks: usize,
    open: bool,
) -> Option<PaneTarget> {
    let named = deck_name(ctx, at, pane, naming)?;
    Some(PaneTarget {
        chevron: named.chevron,
        pane: index,
        // **Zero while it is shut**, which is what stops [`PaneTarget::row`]
        // handing out a rectangle for a card nobody opened — [`Load`]'s own
        // field.
        rows: match open {
            true => decks.min(DECK_LETTERS.len()),
            false => 0,
        },
    })
}

/// **A pane head taking letters**, and the whole of the console's second
/// letter-taking flow's state.
///
/// # One at a time, and it carries which head it is in
///
/// [`Menu::Naming`] is the first flow and it is one because a menu is one; this
/// is one because **the keyboard is one**. Whoever holds the keys takes them
/// whole while a name is being asked for — `s` is an `s` in a name and not a
/// solo — so two open fields would be two places one keystroke could go, with
/// nothing on the panel saying which. So this is an `Option` on the console and
/// not a field per pane, and it names the pane the field is drawn in.
///
/// # The buffer is a `String` this crate owns and does not check
///
/// [`Menu::Naming`]'s rule, unchanged and for its reason: a name that is not
/// one path component is refused where the file is written, in one sentence, by
/// whoever writes it — the surface owns the affordance and never the authority
/// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
/// A head that quietly dropped the characters it did not like would be a rule
/// an operator could only find by experiment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Naming {
    /// **Which pane head the field is in**, as an index into
    /// [`View::inspector`] and so into [`PANE_NAMES`].
    pub pane: usize,
    typed: String,
}

impl Naming {
    /// **What has been typed so far.** The caret is drawn after it and there
    /// is no selection: this is a name, not a document —
    /// [`Arrangement::naming`]'s own sentence.
    pub fn typed(&self) -> &str {
        &self.typed
    }
}

impl View {
    /// **Which pane head is taking letters, and what is in it** — or `None`
    /// for a console where nothing is being named, which is every test in this
    /// crate that does not say otherwise.
    pub fn naming_set(&self) -> Option<&Naming> {
        self.naming.as_ref()
    }

    /// **What pane `index`'s head is taking letters into**, or `None` where it
    /// is reading. This is what [`deck_name`] and [`inspector_into`] each ask,
    /// so that one head is asking and the other is not.
    pub fn naming_set_in(&self, index: usize) -> Option<&str> {
        self.naming
            .as_ref()
            .filter(|naming| naming.pane == index)
            .map(Naming::typed)
    }

    /// **What is being typed into the head of whichever pane is showing
    /// `deck`**, or `None` where no head is asking for a name over that deck.
    ///
    /// **What it is for is a node's keep** (ADR-0338, decision 4): the capsule
    /// on a node group's head types nothing and takes a stamp, and a head that
    /// *is* taking letters is what a keep from that pane files under — which
    /// is ADR-0128's two routes drawn on one capsule, exactly as the deck's
    /// own `keep` draws them.
    ///
    /// **The pane is found by the deck rather than carried**, which is
    /// [`View::named_set`]'s own rule: the gesture spans frames, so what is
    /// filed is what the head says it is filing *now*. An empty buffer answers
    /// `Some("")`, and that is the field's rule and not this method's — the
    /// console emits what was typed, including nothing, and the wall is where
    /// the file is written (P-0090).
    pub fn naming_over(&self, deck: u8) -> Option<String> {
        let naming = self.naming.as_ref()?;
        let pane = self.inspector.get(naming.pane)?;
        (pane.deck == usize::from(deck)).then(|| naming.typed().to_owned())
    }

    /// **Ask for a name in pane `index`'s head**, starting from empty.
    ///
    /// **Starting from empty rather than from the material's name.** The run
    /// under the caret read `drift_night` a moment ago and the field does not
    /// keep it: a buffer seeded with what was there is a name an operator
    /// commits by pressing return once, which is the shape of an overwrite
    /// nobody typed. What ADR-0128 makes an instruction is a name that was
    /// *typed*.
    pub fn name_set(&mut self, index: usize) {
        self.naming = Some(Naming {
            pane: index,
            typed: String::new(),
        });
    }

    /// **Take the field away, typed name and all**, which is what escape
    /// asks and what a press somewhere else asks. [`Arrangement::shut`]'s
    /// sentence: a name abandoned half-typed is not kept for the next time,
    /// because the buffer is the gesture and the gesture ended.
    pub fn stop_naming_set(&mut self) {
        self.naming = None;
    }

    /// **One character into the name being typed**, and `false` where no head
    /// was asking for one. [`Arrangement::typed`]'s rule and its refusal:
    /// control characters are not a name and never reach the buffer, because a
    /// newline is Return arriving as text and that is the commit.
    pub fn type_into_name(&mut self, c: char) -> bool {
        match (&mut self.naming, c.is_control()) {
            (Some(naming), false) => {
                naming.typed.push(c);
                true
            }
            _ => false,
        }
    }

    /// **The last character back out again**, and `false` where there was
    /// nothing to take — no head asking, or an empty name.
    pub fn rub_out_of_name(&mut self) -> bool {
        match &mut self.naming {
            Some(naming) => naming.typed.pop().is_some(),
            None => false,
        }
    }

    /// **The name is finished, and this is what it asks for**: the deck that
    /// head is showing, filed under what was typed.
    ///
    /// # The deck is read at the commit and not at the press
    ///
    /// [`KeepPill`] carries the deck it was measured for because its press is
    /// one instant; this gesture spans frames, and what is filed has to be the
    /// deck the head says it is filing *now*. So the pane is looked up again
    /// and `None` is a pane that has gone — a console handed a shorter
    /// [`View::inspector`] while somebody was typing — where the field is
    /// taken away and nothing is emitted, rather than a keep landing on a deck
    /// whose head is no longer on screen.
    ///
    /// # It refuses nothing else
    ///
    /// An empty name arrives here as an empty name and leaves as one, which is
    /// [`Arrangement`]'s rule at the same seam: `id` is one path component and
    /// the wall is where the bytes are written. **A name typed twice
    /// overwrites**, which is ADR-0128 and is not this control's to soften.
    ///
    /// **The field is taken away whether or not the name is any good**, for
    /// `Readout::named`'s reason one bay along: a refusal is said out loud by
    /// whoever refuses it, and a field left standing over the refusal would be
    /// the panel asking the question again without saying the answer.
    pub fn named_set(&mut self) -> Option<Operation> {
        let naming = self.naming.take()?;
        let deck = self.inspector.get(naming.pane)?.deck as u8;
        Some(Operation::SaveSet {
            deck,
            id: Some(naming.typed),
        })
    }
}

/// **The word on the fold chip**, which is the mock's own and is drawn whether
/// or not it changes anything: *"A deck publishing a single renderer draws the
/// chip anyway and says that it changes nothing either way, because a deck
/// that grows a second one needs the control already where it was."*
const COMPOSITE_LABEL: &str = "composite";

/// **What a parameter row's leftmost cell reads where the control is not on
/// the interface** — the mock's `&middot;`, in `.param.unpub .ord`'s hairline
/// colour.
///
/// A dot where a number would be, because a control off the interface has no
/// **position** and a position is exactly what a MIDI knob counts. It is the
/// same cell either way: the number and the mark are one control's two states
/// rather than a mark drawn beside a number
/// (`docs/adr/0329-…`).
const UNPUBLISHED: &str = "·";

/// **One pane of the Inspector, painted.**
///
/// Where everything goes is [`inspector`]'s, so this paints and derives
/// nothing but the position of one chip after another along a row, which is
/// what a flex row is.
///
/// Term for term from `style.css`:
///
/// - `.half-head` — `color: var(--c-faint)` for the label, `.what`'s
///   `color: var(--c-text)` for the deck and its material, over a
///   `border-bottom: 1px solid var(--c-hair)`.
/// - `.deck-head` — a `.mini` for the sync mode, `.anchor` at
///   [`size::ANCHOR_SIZE`] beside it, and the fold's `.mini` pushed to the
///   right by `.sep`'s `flex: 1`.
/// - `.node-head` — `background: var(--c-tint)`, `.addr`'s
///   `color: var(--c-lav)`, the name in `var(--c-dim)`, and `.auth`'s three
///   words at the right.
/// - `.rend-row` — `.rend` chips, the live one in `var(--c-pink)` over a 15%
///   wash of it.
/// - `.param` — the mock's four tracks, with the fader taking what the other
///   three leave.
///
/// **Everything is clipped to the pane**, which is what makes the overflow
/// safe: a group that fits and a name that does not are the same clip, and it
/// is the same `with_clip_rect` the picture, a preview cell and the library's
/// list are each drawn inside.
/// **Whether the deck a pane is showing is on air**, off the Mixer bay's own
/// reading of it.
///
/// [`Strip::tally`] through [`residency`], which is the one place a tally
/// becomes a residency on this console — a second reading of it here would be
/// two statements about one fact, and the mock draws the pane's `keep` and the
/// strip's tally in one pink for exactly the reason that they are one fact.
///
/// **A deck with no strip is not on air**, which is a state rather than a
/// fallback: [`View::mixer`] is as long as the deck has slots, so a pane
/// pointed past the end is pointed at nothing, and nothing is not live.
fn on_air(strips: &[Strip], deck: usize) -> bool {
    strips
        .get(deck)
        .is_some_and(|strip| residency(strip.tally) == Residency::Live)
}

fn inspector_into(
    ui: &Ui,
    pal: &Palette,
    at: &InspectorPane,
    pane: &Pane,
    on_air: bool,
    naming: Option<&str>,
    // **The pulldown's mark**, derived by the caller off the same reading the
    // press is hit-tested against — `View::pane_pulldown`. It is handed in
    // rather than asked here because the card's rows are read off the mixer,
    // which `draw` has already borrowed. The card itself is painted after
    // every bay, for the `uses` line's card's reason.
    target: Option<PaneTarget>,
) {
    // **Derived here and hit-tested by `claim` off the same call**, and asked
    // before the words are painted rather than after: `.half-head` is a flex
    // row with `.sep` between them, so the readout is what gives way when the
    // pane is narrow and the pill keeps its place. `None` is a head with no
    // room for the capsule, which draws none — see [`keep_pill`].
    let keep = keep_pill(ui.ctx(), at, pane);
    // **And the count beside it**, which is what rule 04 asks of a pane that
    // is showing part of itself — derived here off the same head and painted
    // below, exactly as the capsule is. See [`pane_count`].
    let count = pane_count(ui.ctx(), at, pane, naming);
    // What is left of the head for the two words: everything up to whatever is
    // next along the row, one `.half-head` gap short of it. A name too long for
    // that is clipped, which is the row's own answer to a long name either way
    // — the head is a clip rectangle and there is no ellipsis in this console
    // to draw.
    let words = match count.map(|c| c.min.x).or(keep.map(|pill| pill.pill.min.x)) {
        Some(x) => Rect::from_min_max(
            at.head.min,
            Pos2::new(x - size::HALF_HEAD_GAP, at.head.max.y),
        ),
        None => at.head,
    };
    let painter = ui.painter().with_clip_rect(words);
    let label = painter.layout_job(span_at(head_label(naming), size::BASE, pal.faint));
    let y = at.head.center().y - label.size().y * 0.5;
    painter.galley(
        Pos2::new(at.head.min.x + size::HALF_HEAD_PAD_X, y),
        label,
        pal.faint,
    );
    // **The run, from the same derivation `claim` hit-tests** — the mock's
    // `.what`, and the console's second letter-taking flow while a name is
    // going into it. `None` is a head with no room to paint any of it, which
    // is [`deck_name`]'s own refusal and leaves the label alone in the row.
    if let Some(named) = deck_name(ui.ctx(), at, pane, naming) {
        // **A ground under the field while it is asking, and none while it is
        // reading.** A caret says letters are going *somewhere*; the tint says
        // where, which is the one thing a run of text in a row of readouts
        // cannot say for itself. It is `.node-head`'s own `--c-tint`, so the
        // console spends no new colour on it — and the pink a capsule is lit
        // in is deliberately not reached for here, because that pink means
        // *on air* two controls away.
        if naming.is_some() {
            painter.rect_filled(named.name, CornerRadius::same(3), pal.tint);
        }
        let what = painter.layout_job(span_at(
            &match naming {
                Some(typed) => naming_text_in_head(pane, typed),
                None => showing_text(pane),
            },
            size::BASE,
            pal.text,
        ));
        painter.galley(
            Pos2::new(
                named.name.min.x,
                named.name.center().y - what.size().y * 0.5,
            ),
            what,
            pal.text,
        );
    }
    // `.half-head`'s own `border-bottom`, the bottom pixel of the row — drawn
    // through the whole head rather than through the words' clip, which stops
    // one gap short of the pill.
    let painter = ui.painter().with_clip_rect(at.head);
    let rule = at.head.max.y - size::HAIRLINE * 0.5;
    painter.line_segment(
        [
            Pos2::new(at.head.min.x, rule),
            Pos2::new(at.head.max.x, rule),
        ],
        Stroke::new(size::HAIRLINE, pal.hair),
    );
    // **The pulldown's mark, after the run and inside the head's own clip** —
    // the mock's `▾` beside `deck A · drift_night`, drawn rather than typed
    // for [`CHEVRON_W`]'s reason. It is painted in the label's ink rather than
    // the run's: the mark is a control and the name beside it is a readout,
    // and the console draws every `▾` it has in `--c-faint`.
    if let Some(target) = target {
        let mark = target.chevron;
        painter.add(egui::Shape::convex_polygon(
            vec![
                mark.left_top(),
                mark.right_top(),
                Pos2::new(mark.center().x, mark.max.y),
            ],
            pal.faint,
            Stroke::NONE,
        ));
    }
    // **The mock draws the first pane's `keep` as `.pill.on` and the second
    // pane's as a plain `.pill`**, and what the lit one reads is now on the
    // page: the deck this pane is *showing* is on air. Deck A in the mock is
    // on air *and* holds the selection *and* is the first pane, and the wash
    // is the first of the three for two reasons the console already holds —
    // `.pill.on`'s pink *is* the pink a tally on air is drawn in
    // ([`on_pill_at`]), and the selection is drawn in lavender everywhere
    // else on this panel, so a pink wash meaning *selected* would be the one
    // colour on the console saying two things.
    //
    // **It is handed in rather than asked here**, which is `mixer_into`'s
    // `marked` and `selection` one bay over: residency is the *mixer's*
    // reading of a deck — [`Strip::tally`] — and a second derivation of it in
    // this bay would be two statements about one fact.
    if let Some(pill) = keep {
        match on_air {
            true => on_pill_at(ui, pal, pill.pill, KEEP_LABEL),
            false => pill_at(ui, pal, pill.pill, KEEP_LABEL),
        }
    }
    // **The count, in the label's own ink**: `.half-head`'s `color:
    // var(--c-faint)`, which is what the mock gives every readout in this row
    // and what the Library foot gives its own `5 of 27`. It is painted inside
    // the head's clip and not the words' — the words stop short of it.
    if let Some(rect) = count {
        let galley = painter.layout_job(span_at(&count_text(at, pane), size::BASE, pal.faint));
        painter.galley(
            Pos2::new(rect.min.x, rect.center().y - galley.size().y * 0.5),
            galley,
            pal.faint,
        );
    }

    // **Derived here and hit-tested by `claim` off the same call**, which is
    // the rule every other control on this panel is drawn under. `None` is a
    // row too narrow to hold its chips, and it draws none rather than half of
    // each — see [`deck_head`].
    if let Some(head) = deck_head(ui.ctx(), at, pane) {
        deck_head_into(ui, pal, &head, pane);
    }

    // **The clip is what makes a scrolled pane safe**, and it is the same
    // rectangle [`InspectorPane::grip`] refuses a press outside: a group cut
    // by the top edge is painted with its head under the deck head and clipped
    // away there, and a press on the part that is not on screen reaches
    // nothing.
    let painter = ui.painter().with_clip_rect(at.body);
    for index in at.drawn(&pane.nodes) {
        let node = &pane.nodes[index];
        let rect = at.group(&pane.nodes, index);
        node_into(&painter, pal, rect, node);
        // `.node-group`'s `border-bottom: 1px solid var(--c-hair)`, which
        // `:last-child` does not carry — so it goes *between* two groups, and
        // the last node's is not drawn whether or not the pane is scrolled far
        // enough to have it on screen. It is `nodes.len()` and no longer the
        // count of what is drawn, because a group cut by the bottom edge has
        // a rule under it and the next group is what it separates from.
        if index + 1 < pane.nodes.len() {
            let rule = rect.max.y + size::HAIRLINE * 0.5;
            painter.line_segment(
                [Pos2::new(rect.min.x, rule), Pos2::new(rect.max.x, rule)],
                Stroke::new(size::HAIRLINE, pal.hair),
            );
        }
    }
}

/// **The deck head, painted**: the sync chip, the anchor beside it, the two
/// scrub arrows and the fold at the right.
///
/// Where everything goes is [`deck_head`]'s, so this paints and derives
/// nothing — which is the change this pass made to it: the row used to be a
/// running sum here and a press had nowhere to ask what it had landed on.
///
/// Term for term from `style.css`:
///
/// - `.mini` — the mode chip and the fold, through [`mini_into`], selected
///   because a mode is always one of three and the fold is on or off.
/// - `.anchor` — `font-size: 9px` in `--c-faint`, the run [`anchor_text`]
///   writes.
/// - `.scrub i` — `padding: 0 4px; border-radius: 999px; border: 1px solid
///   var(--c-line)` with `--c-dim` inside it, and `.scrub.idle i`'s
///   `--c-faint` over `--c-hair` where the deck is not beat-synced. **Never
///   grey without a reason**, which is the stylesheet's own note on this pair:
///   an inert scrub keeps its shape, and what says why is the page.
///
/// **The arrows are drawn rather than typed**, which is [`CHEVRON_W`]'s reason
/// three bays along: whether a black left-pointing small triangle is in
/// `egui`'s default face is a question with no good answer, and a triangle is
/// the same mark either way.
fn deck_head_into(ui: &Ui, pal: &Palette, at: &DeckHead, pane: &Pane) {
    let painter = ui.painter().with_clip_rect(at.mode.union(at.composite));
    let word = |rect: Rect, text: &str, sel: bool| {
        let galley = painter.layout_no_wrap(
            text.to_owned(),
            FontId::new(size::MINI_SIZE, FontFamily::Proportional),
            pal.faint,
        );
        mini_into(&painter, pal, rect, sel, |painter, colour| {
            painter.galley(
                Pos2::new(
                    rect.min.x + size::MINI_PAD_X,
                    rect.center().y - galley.size().y * 0.5,
                ),
                galley,
                colour,
            );
        });
    };
    word(at.mode, sync_word(pane.sync), true);

    if let (Some(rect), Some(text)) = (at.anchor, anchor_text(pane)) {
        let galley = painter.layout_job(span_at(&text, size::ANCHOR_SIZE, pal.faint));
        painter.galley(
            Pos2::new(rect.min.x, rect.center().y - galley.size().y * 0.5),
            galley,
            pal.faint,
        );
    }

    // Live is the one mode that reads the offset; the other two keep the
    // chips and lose the ink, which is `.scrub.idle`.
    let live = pane.sync == Sync::Beat;
    let (ink, edge) = match live {
        true => (pal.dim, pal.line),
        false => (pal.faint, pal.hair),
    };
    for (rect, back) in [(at.back, true), (at.forward, false)] {
        painter.rect_stroke(
            rect,
            CornerRadius::same((size::SCRUB_H * 0.5) as u8),
            Stroke::new(size::HAIRLINE, edge),
            StrokeKind::Inside,
        );
        arrow_mark(&painter, rect.center(), size::SCRUB_SIZE, ink, back);
    }

    if let (Some(chips), Some(aimed)) = (at.aim, pane.aimed.as_ref()) {
        // **Lit says somebody asked for this number**, and unlit says it is
        // what the material declares for itself — which is what `.mini.sel`
        // already means on this row for the fold beside it: the chip's two
        // states answer *who chose this* rather than restating the number.
        word(chips.size, &aimed.capacity.to_string(), aimed.stated);
        // **Never lit**, because a capsule that performs has no state to be in
        // — the `keep` pill's arrangement two rows up.
        word(chips.salt, RE_SALT_LABEL, false);
    }

    word(at.composite, COMPOSITE_LABEL, pane.composite);
}

/// **An arrow's mark**, drawn rather than typed — a triangle with its point to
/// the left when `back`, to the right when not.
///
/// `across` wide and the same tall, which is [`Mask`]'s rule for a mark that
/// stands in for a glyph: the box is the size the glyph would have been. It is
/// not [`CHEVRON_W`]'s 2:1, because the ink of a left-pointing small triangle
/// is about as wide as it is tall where a down-pointing one is wider than it
/// is deep.
///
/// **The size is an argument and not [`size::SCRUB_SIZE`]**, because the same
/// mark is drawn at two sizes now: the Inspector's two scrub arrows, at the
/// size of the chip they sit in, and the Library foot's `→`, at [`library::LOAD_ARROW`]
/// beside the type it stands between. One triangle, so an arrow this console
/// draws is the same arrow wherever it is drawn.
fn arrow_mark(painter: &egui::Painter, centre: Pos2, across: f32, colour: Color32, back: bool) {
    let r = across * 0.5;
    let point = match back {
        true => centre.x - r,
        false => centre.x + r,
    };
    let base = match back {
        true => centre.x + r,
        false => centre.x - r,
    };
    painter.add(egui::Shape::convex_polygon(
        vec![
            Pos2::new(point, centre.y),
            Pos2::new(base, centre.y - r),
            Pos2::new(base, centre.y + r),
        ],
        colour,
        Stroke::NONE,
    ));
}

/// **One node group**: the head, the renderer row where there is one, and a
/// row per parameter.
fn node_into(painter: &egui::Painter, pal: &Palette, rect: Rect, node: &Node) {
    let head = Rect::from_min_max(
        rect.min,
        Pos2::new(rect.max.x, rect.min.y + size::NODE_HEAD_H),
    );
    // `.node-head`'s `background: var(--c-tint)`, which is the wash that tells
    // a head from the rows under it — the same tint the *allocated* tally
    // carries.
    painter.rect_filled(head, CornerRadius::ZERO, pal.tint);
    let mut x = head.min.x + size::NODE_HEAD_PAD_X;
    let addr = painter.layout_job(span_at(&node.addr, size::BASE, pal.lav));
    let w = addr.size().x;
    painter.galley(
        Pos2::new(x, head.center().y - addr.size().y * 0.5),
        addr,
        pal.lav,
    );
    x += w + size::NODE_HEAD_GAP;
    let name = painter.layout_job(span_at(&node.name, size::BASE, pal.dim));
    painter.galley(
        Pos2::new(x, head.center().y - name.size().y * 0.5),
        name,
        pal.dim,
    );
    if let Some(authority) = node.authority {
        auth_into(painter, pal, head, node, authority.level);
    }
    // **The `keep` capsule at the right of the head**, and the chips above are
    // laid out inside what it leaves — [`node_keep`], which is where both
    // halves of that arithmetic are. A head that carries none draws none,
    // which is [`Node::keep`]'s two cases rather than a capsule that refuses.
    if let Some(pill) = node_keep(painter.ctx(), head, node) {
        // **A plain `.mini` and never `.sel`**, which is the mock's own and is
        // [`KeepPill`]'s note one row up read on a node: a keep is a press and
        // not a setting, so there is nothing here for a wash to be *on*. The
        // pane head's capsule carries one because it reads the deck's
        // residency, and a node has none.
        let galley = painter.layout_no_wrap(
            KEEP_LABEL.to_owned(),
            FontId::new(size::MINI_SIZE, FontFamily::Proportional),
            pal.faint,
        );
        mini_into(painter, pal, pill, false, |painter, colour| {
            painter.galley(
                Pos2::new(
                    pill.min.x + size::MINI_PAD_X,
                    pill.center().y - galley.size().y * 0.5,
                ),
                galley,
                colour,
            );
        });
    }

    // **The inputs this node declares, under its head and above its rows**,
    // for the same reason and asked the same way: [`uses_rect`] and
    // [`uses_chip_in`] are where that arithmetic is written, and this paints
    // what they answer.
    for (index, uses) in node.uses.iter().enumerate() {
        uses_into(painter, pal, uses_rect(rect, index), uses);
    }

    // **Where the renderer row is, asked rather than measured here**: a press
    // has to resolve to the same rectangle the chips were drawn in, and
    // [`rend_row_in`] is the one place that arithmetic is written.
    if !node.renderers.is_empty() {
        rend_row_into(painter, pal, rend_row_in(rect, node), &node.renderers);
    }
    // **Where a row is, asked rather than accumulated.** The running sum this
    // loop used to keep was a second answer to the same question the moment a
    // press had to be resolved to a row — see [`param_rect`].
    for (index, param) in node.params.iter().enumerate() {
        param_into(painter, pal, param_rect(rect, node, index), param);
        // **Where the sensitivity row is, asked rather than measured here**,
        // which is the renderer row's rule one level up: a press has to
        // resolve to the same rectangle the chips were drawn in.
        if let (Some(row), Some(source)) = (sens_rect(rect, node, index), param.bound.as_ref()) {
            sens_into(painter, pal, row, source);
        }
    }
}

/// **`man / sug / auto`, right-aligned on the node head**, with the one the
/// node is on filled: `.auth span.sel`'s `color: var(--c-mint)` over a 15%
/// wash of it, and the other two in `var(--c-faint)` with no box at all.
///
/// **All three and not only the one**, which is the manual's own row: *"`man /
/// sug / auto` on each node head, never a global mode."* What is drawn is
/// which of the three this node is on, and **all three are claimed**: each
/// names a destination, which is what an operation on this panel is
/// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)),
/// and pressing the one a node is already on asks for what it already has —
/// the renderer row's rule one row down, and the anchor's two bays over. A
/// chip that stopped being pressable the moment it lit would take the claim
/// out from under a hand.
///
/// **A head that folds more than one node draws none**, which is `Node::authority`
/// being `None`: authority is per node, so one chip over three renderers would
/// be one of three answers drawn as *the* answer and a press on it would set
/// three nodes at once.
fn auth_into(
    painter: &egui::Painter,
    pal: &Palette,
    head: Rect,
    node: &Node,
    authority: Authority,
) {
    for (level, rect) in auth_chips(painter.ctx(), head, node) {
        let galley = painter.layout_no_wrap(
            auth_word(level).to_owned(),
            FontId::new(size::AUTH_SIZE, FontFamily::Proportional),
            pal.faint,
        );
        let sel = level == authority;
        let colour = match sel {
            true => pal.mint,
            false => pal.faint,
        };
        if sel {
            painter.rect_filled(
                rect,
                // `border-radius: 999px` on a box this short is a capsule.
                CornerRadius::same((size::AUTH_H * 0.5) as u8),
                tint(pal.mint, 15),
            );
        }
        painter.galley(
            Pos2::new(
                rect.min.x + size::AUTH_PAD_X,
                rect.center().y - galley.size().y * 0.5,
            ),
            galley,
            colour,
        );
    }
}

/// **Where each of the three authority chips goes on a node head**,
/// right-aligned inside the head's own padding.
///
/// **One derivation, asked twice** — [`auth_into`] paints these and
/// [`InspectorPane::set_authority`] hit-tests them, which is [`rend_chips`]'
/// rule one row up: two copies of where a chip is would be a chip painted
/// where a hand cannot press it. The three were drawn and unclaimed from
/// 2026-08-29 until the writer existed, and the running sum inside the painter
/// was exactly the shape a press had nowhere to ask about.
///
/// Right-aligned, so the whole row has to be measured before the first chip
/// can be placed: `.node-head`'s `.sep` pushes `.auth` to the end of the flex
/// row.
///
/// # The node is taken because the `keep` capsule is at the same end
///
/// `.node-head` ends `.sep, .auth, .mini` — the capsule is hard against the
/// head's padding and the chips are stepped back from it — so where a chip
/// goes depends on whether this head carries one ([`node_keep`]). **The trim
/// is inside this function rather than at its callers**, because a caller that
/// forgot it would place three chips over the capsule, and the paint and the
/// hit-test would agree with each other and disagree with the mock. Two
/// callers each applying it correctly is a rule held by prose, which is
/// exactly what `docs/contributing.md` §4's structural tier is against.
pub fn auth_chips(
    ctx: &egui::Context,
    head: Rect,
    node: &Node,
) -> impl Iterator<Item = (Authority, Rect)> {
    let head = auth_head(ctx, head, node);
    let widths: Vec<f32> = AUTHORITIES
        .into_iter()
        .map(|level| auth_width(ctx, level))
        .collect();
    let total: f32 = widths.iter().sum::<f32>() + size::AUTH_GAP * (AUTHORITIES.len() - 1) as f32;
    let mut x = head.max.x - size::NODE_HEAD_PAD_X - total;
    let top = head.center().y - size::AUTH_H * 0.5;
    AUTHORITIES
        .into_iter()
        .zip(widths)
        .map(move |(level, w)| {
            let rect = Rect::from_min_size(Pos2::new(x, top), egui::vec2(w, size::AUTH_H));
            x += w + size::AUTH_GAP;
            (level, rect)
        })
        .collect::<Vec<_>>()
        .into_iter()
}

/// **Where a node head's `keep` capsule goes**, or `None` on a head that
/// carries none.
///
/// The mock's `.node-head` is a flex row of the address, the name, a `.sep`,
/// the `.auth` chips and then `<span class="mini">keep</span>` — so the
/// capsule is hard against the head's right-hand padding and the chips are
/// stepped back from it by [`size::NODE_HEAD_GAP`], which is `.node-head`'s
/// own `gap: 7px`. That is why this is derived before [`auth_chips`] rather
/// than beside it: the chips are laid out inside what this leaves.
///
/// **`None` on the two heads that carry no capsule** — [`Node::keep`], where
/// the rule is written — and `None` on a head with no room for it, which is
/// [`keep_pill`]'s rule one row down: *a control that does not fit in the row
/// it is drawn in is no control at all, rather than half of one*.
///
/// **One derivation, asked twice** — [`node_into`] paints it and
/// [`InspectorPane::keep_procedure`] hit-tests it, which is [`auth_chips`]'
/// own rule: two copies of where a capsule is would be a capsule painted where
/// a hand cannot press it.
pub fn node_keep(ctx: &egui::Context, head: Rect, node: &Node) -> Option<Rect> {
    node.keep?;
    // Fonts are not valid until `egui` has run a pass — [`keep_pill`]'s guard,
    // and before the first one there is nothing drawn here to press.
    if ctx.cumulative_pass_nr() == 0 {
        return None;
    }
    let w = ctx.fonts_mut(|f| {
        f.layout_no_wrap(
            KEEP_LABEL.to_owned(),
            FontId::new(size::MINI_SIZE, FontFamily::Proportional),
            Color32::PLACEHOLDER,
        )
        .size()
        .x
    }) + size::MINI_PAD_X * 2.0;
    let pill = Rect::from_min_size(
        Pos2::new(
            head.max.x - size::NODE_HEAD_PAD_X - w,
            head.center().y - size::MINI_H * 0.5,
        ),
        egui::vec2(w, size::MINI_H),
    );
    // Measured against the head's content box, exactly as [`keep_pill`] is:
    // the capsule is placed from the right-hand padding, so what it runs off
    // is the left one.
    let room = head.width() - size::NODE_HEAD_PAD_X * 2.0;
    (positive(head) && w <= room).then_some(pill)
}

/// **What is left of a node head for the authority chips** — the head, less
/// the `keep` capsule and the gap before it where there is one.
///
/// One function because [`auth_into`] paints the chips and
/// [`InspectorPane::set_authority`] hit-tests them, and a head trimmed in one
/// of the two would be three chips drawn where a hand cannot press them. It is
/// [`node_keep`]'s other half: the two controls at the right of this row are
/// laid out from the right, the capsule first.
fn auth_head(ctx: &egui::Context, head: Rect, node: &Node) -> Rect {
    match node_keep(ctx, head, node) {
        Some(keep) => Rect::from_min_max(
            head.min,
            Pos2::new(
                keep.min.x - size::NODE_HEAD_GAP + size::NODE_HEAD_PAD_X,
                head.max.y,
            ),
        ),
        None => head,
    }
}

/// One authority chip's width: its word at [`size::AUTH_SIZE`] inside
/// `.auth span`'s `padding: 0 5px`.
fn auth_width(ctx: &egui::Context, level: Authority) -> f32 {
    ctx.fonts_mut(|f| {
        f.layout_no_wrap(
            auth_word(level).to_owned(),
            FontId::new(size::AUTH_SIZE, FontFamily::Proportional),
            Color32::PLACEHOLDER,
        )
        .size()
        .x
    }) + size::AUTH_PAD_X * 2.0
}

/// **The renderer chips**, one per renderer the Set has, from the left.
///
/// The live one carries `.rend.sel`: `color: var(--c-pink)` over a 15% wash of
/// it, no border, and the pink halo `box-shadow: 0 0 9px var(--c-glowp)` —
/// which is the same 9 the lit beat and a live fader knob carry. The rest are
/// `.rend`'s `border: 1px solid var(--c-line)` around `var(--c-dim)`.
///
/// **One row and what fits of it.** `.rend-row` wraps in the mock and the
/// console does not: a wrapped row is a group taller than [`group_h`] said it
/// was, and the pane's own arithmetic is what says whether a group is drawn at
/// all. A chip past the right-hand edge is clipped, which is the same answer
/// the pane gives a group past the bottom.
///
/// **Where each chip goes is [`rend_chips`]', so this paints and derives
/// nothing** — the change this pass made to it, and [`deck_head_into`]'s rule
/// one row up: the row used to be a running sum here and a press had nowhere
/// to ask what it had landed on.
fn rend_row_into(painter: &egui::Painter, pal: &Palette, row: Rect, renderers: &[Renderer]) {
    for (index, rect) in rend_chips(painter.ctx(), row, renderers) {
        let rend = &renderers[index];
        let galley = painter.layout_no_wrap(
            rend.name.clone(),
            FontId::new(size::BASE, FontFamily::Proportional),
            pal.dim,
        );
        let radius = CornerRadius::same((size::REND_H * 0.5) as u8);
        match rend.live {
            true => {
                painter.add(
                    egui::epaint::Shadow {
                        offset: [0, 0],
                        blur: size::TALLY_GLOW - 1,
                        spread: 0,
                        color: pal.glow_pink,
                    }
                    .as_shape(rect, radius),
                );
                painter.rect_filled(rect, radius, tint(pal.pink, 15));
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
        painter.galley(
            Pos2::new(
                rect.min.x + size::REND_PAD_X,
                rect.center().y - galley.size().y * 0.5,
            ),
            galley,
            match rend.live {
                true => pal.pink,
                false => pal.dim,
            },
        );
    }
}

/// **One `uses` line**: what the procedure calls the input at the left, and the
/// capsule naming the node filling it at the right.
///
/// **The chevron is drawn rather than typed**, which is [`CHEVRON_W`]'s reason
/// wherever this console draws a pulldown — the Outputs row's pill, the
/// arrangement pill and the Library bay's deck capsule all carry the same mark.
fn uses_into(painter: &egui::Painter, pal: &Palette, row: Rect, uses: &Uses) {
    let painter = painter.with_clip_rect(row);
    let word = painter.layout_job(span_at(
        &format!("uses {}", uses.slot),
        size::USES_SIZE,
        pal.dim,
    ));
    painter.galley(
        Pos2::new(
            row.min.x + size::PARAM_PAD_L,
            row.center().y - word.size().y * 0.5,
        ),
        word,
        pal.dim,
    );
    let chip = uses_chip_in(painter.ctx(), row, uses);
    painter.rect_stroke(
        chip,
        CornerRadius::same((chip.height() * 0.5) as u8),
        Stroke::new(size::HAIRLINE, pal.line),
        StrokeKind::Inside,
    );
    let name = painter.layout_job(span_at(&uses.to, size::USES_SIZE, pal.dim));
    painter.galley(
        Pos2::new(
            chip.min.x + size::PILL_PAD_X,
            chip.center().y - name.size().y * 0.5,
        ),
        name,
        pal.dim,
    );
    // The `▾`, drawn as the same triangle every pulldown on this console draws:
    // `CHEVRON_W` across and `CHEVRON_H` deep, centred in the padding at the
    // capsule's right.
    let chevron = Rect::from_center_size(
        Pos2::new(
            chip.max.x - size::PILL_PAD_X - CHEVRON_W * 0.5,
            chip.center().y,
        ),
        egui::vec2(CHEVRON_W, CHEVRON_H),
    );
    painter.add(egui::Shape::convex_polygon(
        vec![
            chevron.left_top(),
            chevron.right_top(),
            Pos2::new(chevron.center().x, chevron.max.y),
        ],
        pal.dim,
        Stroke::NONE,
    ));
}

/// **One parameter row**, in the mock's own four tracks: the ordinal at
/// [`size::PARAM_ORD_W`] right-aligned, the name at [`size::PARAM_NAME_W`],
/// the fader taking what is left, and the value at [`size::PARAM_VAL_W`]
/// right-aligned.
///
/// **The value is two places**, which is the mock's `2.40`, `0.71`, `1.20` —
/// every number in its `.pval` column. It is not a second spelling of the
/// transport's tempo: that one is a BPM and this is a parameter, and the mock
/// writes the two differently for that reason.
fn param_into(painter: &egui::Painter, pal: &Palette, row: Rect, param: &Param) {
    let left = row.min.x + size::PARAM_PAD_L;
    let right = row.max.x - size::PARAM_PAD_R;
    // **The mark is the number**, and a control the interface does not carry
    // has none: `.param.unpub .ord` is the mock's dot in the hairline colour,
    // where a published row's is its position in `--c-faint`. One cell, two
    // states, and the state *is* whether it is published — a second mark beside
    // the number would be two spellings of one fact
    // (`docs/adr/0329-…`).
    let (word, ink) = match param.ord {
        Some(ord) => (ord.to_string(), pal.faint),
        None => (UNPUBLISHED.to_owned(), pal.hair),
    };
    let ord = painter.layout_job(span_at(&word, size::PARAM_ORD_SIZE, ink));
    painter.galley(
        Pos2::new(
            left + size::PARAM_ORD_W - ord.size().x,
            row.center().y - ord.size().y * 0.5,
        ),
        ord,
        ink,
    );
    let name_x = left + size::PARAM_ORD_W + size::PARAM_GAP;
    // `.param .pname`'s `overflow: hidden; text-overflow: ellipsis` — one row,
    // broken anywhere, with an ellipsis for what did not fit. The mock says so
    // for this column and not for the library's, which is why one elides and
    // the other clips.
    // `.param.unpub .pname` is a shade further back than `.param`'s, which is
    // the whole of what an unpublished row looks like beside a published one:
    // the name is still legible — the row is drawn so it can be pressed again —
    // and nothing about it invites a hand.
    let name_ink = match param.ord {
        Some(_) => pal.dim,
        None => pal.faint,
    };
    let mut job = span_at(&param.name, size::BASE, name_ink);
    job.wrap = egui::epaint::text::TextWrapping {
        max_width: size::PARAM_NAME_W,
        max_rows: 1,
        break_anywhere: true,
        overflow_character: Some('…'),
    };
    let name = painter.layout_job(job);
    painter.galley(
        Pos2::new(name_x, row.center().y - name.size().y * 0.5),
        name,
        name_ink,
    );
    // **A control the interface does not carry stops here.** The fader and the
    // figure are what publishing decides the panel shows, so a row that is off
    // the list is a mark and a name and nothing else — and a value drawn beside
    // a control this pane says it is not showing would be the page's own
    // sentence contradicted in the same row.
    if param.ord.is_none() {
        return;
    }
    // **A bound row shows its source instead of a number** — `.param.bound`'s
    // `.pval.src`, and the readout the manual calls *where disagreeing with
    // the system begins*. The mock draws it in the same right-aligned track
    // the figure is in, so this is one galley either way, and it is the mint
    // `.src` carries rather than `.pval`'s text colour.
    let (text, colour) = match &param.bound {
        None => (format!("{:.2}", param.value), pal.text),
        Some(source) => (source.signal.clone(), pal.mint),
    };
    let value = painter.layout_job(span_at(&text, size::BASE, colour));
    painter.galley(
        Pos2::new(
            right - value.size().x,
            row.center().y - value.size().y * 0.5,
        ),
        value,
        colour,
    );
    if let Some(fader) = param_fader(row, param) {
        fader_into(painter, pal, fader, false, None);
    }
}

/// **One sensitivity row**: the word in `.sens`'s first track, then the chips
/// that say what is holding the control and offer the two things a hand can do
/// about it.
///
/// **Where each chip goes is [`sens_chips`]', so this paints and derives
/// nothing** — [`rend_row_into`]'s rule one row down, and the reason a press
/// has somewhere to ask what it landed on.
///
/// **Two of the four are drawn as readouts and two as controls**, and nothing
/// in the paint says which: the mock gives the source pill `.pill.armed` and
/// the other three a plain `.pill`, and *armed* here is the mint of something
/// that is holding a control rather than of something that can be pressed.
/// Which chips are claimed is [`SensChip::operation`]'s, and a panel that drew
/// the difference would be drawing a rule the mock does not.
fn sens_into(painter: &egui::Painter, pal: &Palette, row: Rect, source: &Source) {
    let label = painter.layout_job(span_at(SENS_LABEL, size::SENS_SIZE, pal.faint));
    painter.galley(
        Pos2::new(
            row.min.x + size::SENS_PAD_L,
            row.min.y + size::SENS_PAD_T + (size::SENS_CHIP_H - label.size().y) * 0.5,
        ),
        label,
        pal.faint,
    );
    let radius = CornerRadius::same((size::SENS_CHIP_H * 0.5) as u8);
    for (chip, rect) in sens_chips(painter.ctx(), row, source) {
        let armed = chip == SensChip::Signal;
        match armed {
            // `.pill.armed`: no border, a 14% wash of the mint and the same
            // nine-pixel halo an armed pill carries everywhere else on this
            // panel.
            true => {
                painter.add(
                    egui::epaint::Shadow {
                        offset: [0, 0],
                        blur: size::TALLY_GLOW - 1,
                        spread: 0,
                        color: pal.glow,
                    }
                    .as_shape(rect, radius),
                );
                painter.rect_filled(rect, radius, tint(pal.mint, 14));
            }
            // `.pill`'s `border: 1px solid var(--c-line)`.
            false => {
                painter.rect_stroke(
                    rect,
                    radius,
                    Stroke::new(size::HAIRLINE, pal.line),
                    StrokeKind::Inside,
                );
            }
        }
        let colour = match armed {
            true => pal.mint,
            false => pal.dim,
        };
        let galley = painter.layout_no_wrap(
            chip.text(source),
            FontId::new(size::SENS_SIZE, FontFamily::Proportional),
            colour,
        );
        painter.galley(
            Pos2::new(
                rect.min.x + size::SENS_CHIP_PAD_X,
                rect.center().y - galley.size().y * 0.5,
            ),
            galley,
            colour,
        );
    }
}

/// **One parameter row's fader, laid out** — the track between the name and
/// the figure, what the value fills of it, and the knob on the fill's moving
/// edge. `None` where the row is too narrow to have a track at all.
///
/// # One derivation, asked twice
///
/// [`param_into`] paints this and [`InspectorPane::grip`] hit-tests it, which
/// is [`Mixer::grab`]'s rule and [`MasterRow::grab`]'s: two copies of where a
/// knob is would be a knob painted where a hand cannot take hold of it. **The
/// value is part of the geometry** — the knob sits on the fill's moving edge,
/// so where it is depends on what the deck said this frame, and the row a hand
/// grabs is the row it saw.
///
/// **`.param`'s middle track**, which is the `1fr` of `grid-template-columns:
/// 15px 88px 1fr 58px`: the ordinal, the name and the figure are stated widths
/// and this is what is left between them. `lib.rs` measures the pane's own
/// minimum off exactly that — *"the fader is the `1fr` track and is drawn only
/// where what is left over is positive"* (ADR-0279) — and this is where that
/// `positive` is asked.
fn param_fader(row: Rect, param: &Param) -> Option<Fader> {
    let left = row.min.x + size::PARAM_PAD_L;
    let right = row.max.x - size::PARAM_PAD_R;
    let track = Rect::from_min_max(
        Pos2::new(
            left + size::PARAM_ORD_W + size::PARAM_GAP + size::PARAM_NAME_W + size::PARAM_GAP,
            row.center().y - size::FADER_H * 0.5,
        ),
        Pos2::new(
            right - size::PARAM_VAL_W - size::PARAM_GAP,
            row.center().y + size::FADER_H * 0.5,
        ),
    );
    match positive(track) {
        false => None,
        // `.fader b` fills its 5px track edge to edge, so the inset is zero —
        // the one argument that tells this fader from the mixer's vertical
        // one, which `fader` takes for exactly this reason.
        true => Some(fader(
            track,
            Axis::Row,
            param.at(),
            0.0,
            egui::vec2(size::FADER_KNOB_W, size::FADER_KNOB_H),
        )),
    }
}

/// **Where the `index`th parameter row of `node` goes** inside the group
/// rectangle [`InspectorPane::group`] answered.
///
/// [`InspectorPane::group`]'s walk one level in, and a function rather than a
/// running sum inside [`node_into`] for the reason the deck head was lifted out
/// of `inspector_into`: a press had nowhere to ask what it had landed on. The
/// head is [`size::NODE_HEAD_H`], the renderer row is [`size::REND_ROW_H`]
/// where the group has one, and the rows are [`size::PARAM_H`] each from
/// there — which is [`group_h`] read as an offset instead of as a total, and
/// the two are checked against each other in `tests/param_fader.rs`.
fn param_rect(group: Rect, node: &Node, index: usize) -> Rect {
    let top = group.min.y
        + size::NODE_HEAD_H
        + uses_h(node)
        + match node.renderers.is_empty() {
            true => 0.0,
            false => size::REND_ROW_H,
        }
        // **A walk and not a stride, because a row is as tall as what is under
        // it.** A bound row carries a sensitivity row, so the rows above this
        // one are not all [`size::PARAM_H`] — which is [`InspectorPane::group`]'s
        // own reason for walking the groups instead of multiplying, one level in.
        + node.params.iter().take(index).map(rows_h).sum::<f32>();
    Rect::from_min_max(
        Pos2::new(group.min.x, top),
        Pos2::new(group.max.x, top + size::PARAM_H),
    )
}

/// **The leftmost cell of a parameter row**, which is the mark that publishes
/// it: [`size::PARAM_ORD_W`] wide at the row's left padding, the full height of
/// the row.
///
/// **The whole cell and not the ink in it.** A published row's number is one or
/// two glyphs and an unpublished row's is a dot, so a target the size of what is
/// drawn would be a control that shrank as the interface grew past nine — which
/// is the *drawn and not claimed* mistake made in the other direction. The cell
/// is a fixed track of the mock's own grid, so the target is the same size on
/// every row.
///
/// `param` is taken so that this cannot be asked of a row that has none to give;
/// there is no such row today, and the argument for the cell being one control's
/// two states is at [`UNPUBLISHED`].
fn ord_cell(row: Rect, _param: &Param) -> Rect {
    let left = row.min.x + size::PARAM_PAD_L;
    Rect::from_min_max(
        Pos2::new(left, row.min.y),
        Pos2::new(left + size::PARAM_ORD_W, row.max.y),
    )
}

/// **Where the `index`th row's sensitivity row goes** — directly under the row
/// itself, the full width of the group and [`size::SENS_H`] tall — or `None`
/// where nothing is holding that control.
///
/// [`param_rect`] stepped off the end of the row it belongs to, which is the
/// one place that relationship is written: the `.sens` row is not a row of its
/// own in the mock's list, it is what a `.param.bound` grows.
fn sens_rect(group: Rect, node: &Node, index: usize) -> Option<Rect> {
    let param = node.params.get(index)?;
    param.bound.as_ref()?;
    let row = param_rect(group, node, index);
    Some(Rect::from_min_max(
        Pos2::new(row.min.x, row.max.y),
        Pos2::new(row.max.x, row.max.y + size::SENS_H),
    ))
}

/// The console's view: which room it is in, and the frame's plan, kept so a
/// frame does not allocate one.
pub struct View {
    pub room: Room,
    /// **Whether the projector window is open**, which is the one output this
    /// crate cannot read for itself.
    ///
    /// The picture's on and off is [`Layout::visible`] on its own node and the
    /// Outputs row reads it there; a projector is a second window, a second
    /// surface and a second [`karakuri_engine::frame::Sink`], and this crate
    /// takes no device
    /// ([ADR-0156](../../../../docs/adr/0156-the-consoles-arrangement-is-a-tree-this-repository-owns.md)).
    /// So it arrives the way [`View::picture`] does: written per frame by
    /// whoever owns the window. `false` for every test in this crate, which is
    /// a console with no engine behind it.
    ///
    /// **The two plugin chips have no field**, and that is not an omission:
    /// there is no manifest to read them out of, so *not installed* is what
    /// they are rather than something somebody could tell this crate.
    pub projector: bool,
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
    /// slot order, or `None` for a cell with no deck slot behind it.
    ///
    /// **`None` is not "that deck is off air".** A cell shows its slot's own
    /// material whatever the slot's residency — a parked deck's still and a
    /// warming deck's picture are the two an operator most needs to see, which
    /// is
    /// [ADR-0258](../../../../docs/adr/0258-the-look-comes-before-the-fader-so-a-cell-draws-every-slot-and-says-which-nothing-it-is.md).
    /// `None` is a cell with nothing to sample at all: a deck of fewer slots
    /// than there are cells, or a console with no engine behind it.
    ///
    /// **The same seam as [`View::picture`], four times over**: registering a
    /// texture takes a device, this crate has none, so whoever owns the device
    /// registers them and writes this per frame beside the frame that made
    /// them. A stale id here is a freed registration. Every test in this crate
    /// leaves every entry `None`, which is what `cargo test -p
    /// karakuri-console` sees and is a console with no engine behind it.
    ///
    /// **All `None` is a state, not an absence**, and the caption is where it
    /// is said: a cell with nothing behind it reads [`PREVIEW_NO_SLOT`] under
    /// its letter rather than going blank. It is deliberately not the manual's
    /// *empty* — a slot that exists with nothing loaded into it is a state the
    /// engine cannot be in — and deliberately not *off*, which was residency
    /// and has not gated a cell since ADR-0240. See [`state_word`], the module
    /// documentation, and [`preview_rects`] for where the rectangles come
    /// from.
    pub previews: [Option<Picture>; DECKS],
    /// **Which of the four slots have stopped updating**, in slot order, and
    /// `false` for every cell with nothing behind it — which is every test in
    /// this crate and is a console with no engine behind it.
    ///
    /// **A slot is stopped when the version in it costs more than one frame
    /// may** (`karakuri_engine::deck::Deck::overloaded`, ADR-0316). The engine
    /// then skips that slot's step and its draw, so its target holds the last
    /// image it made and this cell goes on showing it. **The caption is where
    /// that is said** — [`PREVIEW_OVERLOADED`] in place of
    /// [`PREVIEW_MATERIAL`] — because the picture cannot say it: a held frame
    /// of good material looks like material, and an unmarked still is a
    /// preview that lies
    /// ([ADR-0269](../../../../docs/adr/0269-a-slot-that-is-drawn-is-stepped-and-a-preview-runs-at-the-rooms-tempo.md)).
    ///
    /// **A `bool` beside the picture rather than a third state of it.** The two
    /// keep different clocks, exactly as [`View::costs`] does: a picture is a
    /// texture registration rewritten every frame by whoever owns the device,
    /// and this changes when a build lands. It is also not a residency and
    /// must not be read as one — a stopped Live slot is still in the mix.
    ///
    /// **The same seam every value here crosses**: `src/` takes no engine
    /// (ADR-0156), so whoever holds the deck reads it and writes this.
    pub overloaded: [bool; DECKS],
    /// **What the governor budgeted each of the four slots at**, in slot
    /// order, or `None` for a slot it has no number for — which is every test
    /// in this crate and is a console nobody has governed.
    ///
    /// **The risk badge is read from this and from nothing else.**
    /// [`band_of`] turns the number into one of five bands and
    /// [`caption_into`] draws the dot; `None` draws no dot at all, which is
    /// the manual's own state for a slot with no cost and is not a hollow one.
    ///
    /// **A second field rather than a third member of [`Picture`]**, and the
    /// two halves of a cell keep different clocks on purpose. A picture is a
    /// texture registration and is rewritten every frame by whoever owns the
    /// device; a cost is `karakuri_engine::governor::Report`, which is taken
    /// on a governor pass and not on a frame — before the first frame, and
    /// again whenever a residency is written. Folding the cost into `Picture`
    /// would make every frame's texture aim carry a number it did not take,
    /// and the number would be dropped and re-fetched sixty times a second to
    /// no purpose.
    ///
    /// **Where it comes from.** One entry per `Decision` in
    /// `Report::decisions`, at `Decision::slot`: `budgeted_ms` and
    /// `Decision::basis`, with `governor::Basis::Unbudgetable` written as
    /// `None` — that is a slot nothing measured and nothing estimated, and it
    /// is *not* a zero. `src/` takes no engine (ADR-0156), so whoever holds
    /// the deck reads the report and writes this, exactly as
    /// [`View::previews`] is written by whoever holds the device.
    ///
    /// **It is not gated on residency and it is not gated on the picture
    /// here.** A parked deck is the one whose cost an operator most wants,
    /// because the cost is why it is parked; the gate that does exist is
    /// [`caption_into`]'s, which is the mock's *no slot is no cost*.
    pub costs: [Option<Budgeted>; DECKS],
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
    /// **What the arrangement pill in that row reads, and what its menu is
    /// doing.**
    ///
    /// **Half of it is the same seam as [`View::transport`] and half of it is
    /// not**, which is the one field here that is both. The name in use and
    /// the names filed are a file and a directory, so whoever owns the store
    /// reads them and writes them here — [`View::library`]'s seam, one row up.
    /// The menu is this crate's own and moves only through
    /// [`Arrangement`]'s methods: what the *control* is doing is not something
    /// the program can be the model of record for.
    ///
    /// [`Arrangement::NONE`] until somebody says otherwise, which is every
    /// test in this crate and is a console with no store behind it: the
    /// default arrangement, nothing filed, and the menu shut. See
    /// [`Arrangement`] and [`arrangement`].
    pub arrangement: Arrangement,
    /// **What the audio-in pill in that row reads, and whether its card is
    /// down** — or `None` for a console nobody has told anything about audio,
    /// which is every test in this crate and draws no pill at all.
    ///
    /// **The same two halves [`View::arrangement`] has**, over a device
    /// instead of a file: which input is open and what inputs there are are a
    /// microphone and an enumeration of a host, and `src/` has neither
    /// (ADR-0156) — so whoever opened one writes them here. Whether the card
    /// is down is this crate's and moves only through [`AudioIn`]'s methods.
    ///
    /// **`Option`, where the arrangement is not**, and the difference is real:
    /// every console has an arrangement — the default one, which is what is on
    /// screen — and a console has an *input* only if somebody opened one and
    /// said so. `Some(AudioIn::NONE)` is a program that looked and found
    /// nothing, and it draws `audio-in · none`; `None` is a program that never
    /// said, and a pill drawn for it would be this crate answering a question
    /// about a device on its own authority. See [`audio_in`].
    pub audio: Option<AudioIn>,
    /// **Whether MIDI learn is armed** — the transport row's `learn` pill, lit
    /// while this is true.
    ///
    /// **This crate's own state, and the only console state on this row that
    /// is** ([`AudioIn`]'s card is the other). Arming is a mode and it is the
    /// one thing rule 04 lets be a mode, because the pill *is* the readout: it
    /// is lit for exactly as long as the mode is on, and nothing else on the
    /// panel changes meaning while it is
    /// ([ADR-0336](../../../../docs/adr/0336-a-learn-is-a-map-edit-and-the-tips-midi-line-is-the-live-map.md)).
    ///
    /// **A `bool` and not an `Option`, where [`View::map`] beside it is an
    /// `Option`**, and the difference is the same one `audio` draws: whether a
    /// mode is armed is a fact this crate owns outright, and *which map is
    /// loaded* is a fact about a file this crate cannot see.
    ///
    /// **What it does not do is bind anything.** The gesture is the host's —
    /// what the pointer is on, what arrived on the wire, and what goes in the
    /// file are all outside this crate (ADR-0156). This is the arming and the
    /// lamp.
    pub learn: bool,
    /// **Which map is loaded**, for the transport row's `map` pill, or `None`
    /// for a console nobody has told — which is every test in this crate and
    /// every run with no surface.
    ///
    /// It is [`View::audio`]'s shape one pill along and for its reason: a map
    /// is a *file*, `src/` reads none (ADR-0156), so whoever loaded one writes
    /// its name here. `Some(MapPill::NONE)` is a program with a surface and no
    /// map, which draws `map · none`; `None` draws no pill at all.
    pub map: Option<MapPill>,
    /// **What the other three controls in the tracker group read this
    /// frame** — the latency offset the open session is holding, and which way
    /// the grid can still be moved an octave — or `None` for a console nobody
    /// has told anything about the beat tracker, which is every test in this
    /// crate that does not say otherwise and draws none of the three.
    ///
    /// **The same seam as [`View::audio`]**, over the beat lock instead of over
    /// the device it listens through: the range the tracker searches is
    /// `karakuri-audio`'s and the offset is a value
    /// `karakuri_environment::audio::Audio` holds, and `src/` has neither
    /// (ADR-0156) — so whoever opened one writes them here per frame.
    ///
    /// **A field of its own rather than three more on [`View::audio`]**, which
    /// is [`View::look`]'s argument beside [`View::transport`]: the pill is
    /// *which room is being heard*, and this is *what the tracker is doing
    /// with it*. The two are `Some` together in every program that draws this
    /// row, and a type that could only say them together would be answering
    /// one question with two. See [`Tracker`] and [`tracker_group`].
    pub tracker: Option<Tracker>,
    /// **What the two look controls in that row read this frame**, or `None`
    /// for a console with no engine behind it — which is every test in this
    /// crate that does not hand one in, and what the row draws there is
    /// nothing at all.
    ///
    /// **The same seam as [`View::transport`]**, two items along the same row:
    /// the operator and the level are `karakuri_engine::frame::Look`, `src/`
    /// has no engine (ADR-0156), so whoever owns one reads it and writes this
    /// per frame beside the frame it was drawn under.
    ///
    /// It is a second field rather than two more fields on [`Transport`]
    /// because it is a different reading of a different thing: a tempo and a
    /// frame cost are what the *session* is doing, and a look is what the
    /// picture is being put through. `transport` answering `None` and this
    /// answering `None` are two facts, and a console driving one and not the
    /// other is a state the type should be able to say. See [`Look`] and
    /// [`look`].
    pub look: Option<Look>,
    /// **What the Master bay's out row reads this frame**, or `None` for a
    /// console with no engine behind it — which is every test in this crate
    /// that does not hand one in, and what the bay draws then is its card and
    /// its head.
    ///
    /// **The same seam as [`View::look`]**, one bay away and one level along
    /// the same chain: this is `karakuri_engine::deck::Deck::out`, which is
    /// applied where the mix *writes* the composited frame, and the look is
    /// applied where the present pass *reads* it. `src/` has no engine
    /// (ADR-0156), so whoever owns one reads it and writes this per frame
    /// beside the frame it was drawn under.
    ///
    /// **A bare level rather than a struct**, and it stays one now that the
    /// chain exists: this is the level at the chain's *entry*, and what the
    /// chain is set to is [`View::master_chain`] beside it. The two are two
    /// fields for the reason they are two records — one is a level a fader
    /// rides and one is a set of settings a press moves
    /// ([ADR-0224](../../../../docs/adr/0224-out-and-exposure-are-two-levels-that-multiply-in-different-places.md),
    /// [ADR-0317](../../../../docs/adr/0317-the-master-chain-is-three-fixed-passes-and-feedback-reads-either-cut.md)).
    pub master_out: Option<f32>,
    /// **What the Master bay's three effect rows read this frame**, or `None`
    /// for a console with no engine behind it — in which case the bay draws
    /// the out row and nothing under it.
    ///
    /// [`View::master_out`]'s seam exactly, one row down:
    /// `karakuri_engine::present::Present::chain` is what a harness reads it
    /// from, and [`Chain`] is that value mirrored into a crate with no engine
    /// in it.
    pub master_chain: Option<Chain>,
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
    /// path (P-0091), and nothing in this crate can do it anyway: opening a
    /// store is `karakuri-store`'s and `src/` depends on neither it nor the
    /// engine (ADR-0156). What crosses the seam is a list of names.
    ///
    /// A name and nothing else, because a name is what exists: see [`library`]
    /// for the star and the time the mock draws beside it, and for why neither
    /// is here.
    ///
    /// **It is the listing of whichever scope is marked**, and not of the
    /// store: the bay lists `my sets` where that chip is marked and the
    /// presets root's `.kset` files where that one is, and both are a
    /// directory read the host does on the press that changed the scope. See
    /// [`View::scopes`] and [`Scope`].
    pub library: Vec<String>,
    /// **What each row of [`View::library`] is**, in the same order — a Set or
    /// a procedure, and the layers its badge names.
    ///
    /// **Empty is every row a Set with no badge**, which is what this bay drew
    /// before ADR-0338 and is every test in this crate that does not say
    /// otherwise. So a host that has not been changed to answer it draws
    /// exactly what it drew, and a host that has hands the two halves over on
    /// one press — see [`RowKind`], where that default is argued, and
    /// [`View::rows`], which is what every control here reads.
    ///
    /// **The same seam as [`View::library`]**: a Set's layers are its file's
    /// own `slot` records and a procedure's kind is the `kind` line of a
    /// source, both of which are a directory read on the press that builds a
    /// listing — and this crate reads no store (ADR-0156, P-0091).
    ///
    /// **Shorter than the listing is not an error**, for [`View::starred`]'s
    /// reason read the other way: a row past the end of it is a Set with no
    /// badges, which is a row this console can draw and act on.
    pub kinds: Vec<RowKind>,
    /// **Which Sets this store has starred**, as their ids — and **empty** for
    /// a console with no store behind it, which is every test in this crate
    /// that does not say otherwise and is a listing whose every row draws a
    /// hollow star.
    ///
    /// **The same seam as [`View::library`]**, in the same rows: the marks are
    /// `<store>/favourites.json` beside the Sets (ADR-0299), and this crate
    /// reads no store (ADR-0156). So the host reads them on the same press the
    /// listing is built on and hands over the set of ids.
    ///
    /// **A `BTreeSet` and not a `Vec<bool>` beside the listing.** The question
    /// a row asks is *is this one starred*, once per row, and two vectors of
    /// the same length are two vectors that can disagree about it — where a
    /// set of ids answers the same question about a listing that was rewritten
    /// under it without being wrong, and is what
    /// `karakuri_store::Store::favourites` already hands back.
    ///
    /// **Ids the listing does not hold are not an error here.** `my sets` is
    /// the intersection of these with what the store holds and the host is
    /// what takes it; a mark left behind by a file somebody deleted draws no
    /// row and is not this field's to prune.
    pub starred: std::collections::BTreeSet<String>,
    /// **Which Set the load pulldown's deck is running**, as the host reads it
    /// off that deck's aim — and `None` for a deck playing the pair the run was
    /// launched with, which is where every run starts.
    ///
    /// **It is here because a walk names a Set and this console cannot spell
    /// one.** [`Operation::WalkHistory`] carries the id of the history it is a
    /// walk of; the id rides the aim a library load sends and this crate reads
    /// no engine (ADR-0156), so the host answers it here, exactly as it answers
    /// the rows of that walk into [`View::library`] — see [`Chosen::asked`],
    /// which is the one place this is read.
    ///
    /// **A readout rebuilt per frame, like [`View::mixer`] beside it**, rather
    /// than written on the press that changes the aim: the pulldown's deck can
    /// be re-pointed by a load, by a key, by a mapped control and by a model,
    /// and a value written at one of those four is a value stale after the
    /// other three.
    ///
    /// **The deck it is read off is the load pulldown's and not the
    /// selection's** ([`View::target_deck`], ADR-0305): the walk is drawn under
    /// the pulldown that says which deck the bay is preparing, and the landing
    /// it feeds lands there.
    pub aimed: Option<String>,
    /// **Which libraries this console has to offer**, in the order the chips
    /// are drawn — and **empty** for a console nobody has told, which is every
    /// test in this crate that does not say otherwise and what the bay then
    /// draws is no scope row at all.
    ///
    /// **The same seam as [`View::library`]**, one row up in the same bay:
    /// what a scope can be asked is a store, a told directory and a directory
    /// somebody names during the run, and this crate has none of the three
    /// (ADR-0156). So the host says which there are and answers the marked one
    /// into [`View::library`].
    ///
    /// **Empty rather than [`Scope::ALL`] by default**, which is [`View::mixer`]'s
    /// rule and not a shortage: a console that has been told nothing has no
    /// libraries rather than four it cannot answer, and a default here would
    /// be this crate asserting that a presets root and a folder exist on a
    /// machine it cannot look at.
    pub scopes: Vec<Scope>,
    /// **What the `holds` field can be stepped to**, in the order it steps
    /// them — and **empty** for a console nobody has told, which is every test
    /// in this crate that does not say otherwise and is a field a press asks
    /// the listing again through.
    ///
    /// **The same seam as [`View::scopes`]**, two rows up in the same bay:
    /// `holds` is matched against what a Set's nodes are called, which is
    /// `karakuri_environment::setfile::summarise`'s reading of the store, and
    /// this crate reads no store (ADR-0156). So the host says what there is to
    /// narrow by and the console steps through it.
    ///
    /// **Written on the same read as [`View::library`]** and off the same
    /// summary, because the two are one directory read: the rows are the Sets
    /// that matched and these are the names any of them could be matched by.
    /// **They are the candidates of the *unnarrowed* listing**, so the row a
    /// press steps to does not depend on what the field is already set to —
    /// candidates read off a filtered listing would shrink as the filter bit,
    /// and a step would then wander somewhere it could not come back from.
    pub holds: Vec<String>,
    /// **Which directory this library is pointed at**, as the host spells
    /// it — and `None` until a folder has been dropped on this window, which
    /// is where every run starts and is a bay with no `.path` row at all.
    ///
    /// **The same seam as [`View::library`]**, one row up in the same bay: a
    /// directory is a thing on a disk, this crate reaches no disk (ADR-0156),
    /// and what crosses is the line to draw. The host writes it on the drop
    /// that chose it and never on a frame — a drop is one act, and asking the
    /// file system what a path is is not a thing to do per frame (P-0091).
    ///
    /// **It is not a scope and it is not `Scope::Folder`.** The row is drawn
    /// whichever chip is marked, because it is also where a send's save dialog
    /// opens (ADR-0311, superseding ADR-0267's *where a send lands*); what the
    /// folder scope's *listing* is arrives in [`View::library`] like every
    /// other scope's.
    ///
    /// It is a `String` rather than a `PathBuf` for the same reason the
    /// listing is: this crate never opens it, so what it needs is the
    /// spelling. See [`Pointed`].
    pub folder: Option<String>,
    /// **The path a release would set**, while a folder is over the window —
    /// and `None` whenever no drag is over it, which is nearly always.
    ///
    /// **Beside [`View::folder`] rather than inside it**, because the two are
    /// different facts: one is where this bay *is* pointed and the other is
    /// where it *would be*. A drag that leaves the window without being let go
    /// clears this and leaves the other standing, which is the row going back
    /// to what it said — see [`View::pointed`], where the two become the one
    /// row the mock draws.
    ///
    /// **Written per frame by whoever reads the platform's hover**, which is
    /// the one thing about this bay that is a frame's business: `egui` clones
    /// the hovered files onto every pass while a drag is over the window, so
    /// there is no event to hang it off. Nothing is asked of the file system
    /// for it (P-0091) — whether the path is a folder is the drop's question
    /// and not the hover's, and the row says nothing about whether the release
    /// will be allowed (ADR-0275).
    pub incoming: Option<String>,
    /// **What the Staging lane lists this frame**: one candidate per deck slot
    /// whose newest build has a verdict outstanding or whose file no longer
    /// agrees with its picture — and **empty** for a console with no engine
    /// behind it, which is every test in this crate that does not hand one in
    /// and what the bay draws then is its card and its head.
    ///
    /// **The same seam as [`View::mixer`]**, and empty rather than
    /// `Option<Vec<_>>` for the same reason: an empty lane is already the
    /// whole of *nothing waiting*, and a run in which nobody has rewritten a
    /// procedure and a run with no producer at all are one bay — one with no
    /// row to draw. **Empty is this lane's ordinary state**, which is what
    /// makes it different from every other bay here: a library with nothing in
    /// it is a library nobody has filled.
    ///
    /// **Written when an event arrives rather than per frame**, by whoever
    /// drains `karakuri_engine::deck::Deck::events` — which is a `Vec` the
    /// engine only appends to when a build lands, is refused or is judged, and
    /// which a caller that never drains grows for the rest of the run. So a
    /// frame on which nothing was swapped touches nothing here, and the name a
    /// row carries is rewritten only when the row's own build changes. See
    /// [`Candidate`] and [`staging`], which is also where the four things the
    /// mock's row has and this does not are named.
    pub staging: Vec<Candidate>,
    /// **What each Inspector pane is showing this frame**, one per pane the
    /// console has room to point at something — and **empty** for a console
    /// with no deck behind it, which is every test in this crate and what the
    /// bay draws then is its card, its head and the bar between its panes.
    ///
    /// **The same seam as [`View::mixer`]**, and empty rather than
    /// `[Option<Pane>; PANES]` for the same reason: an empty list is already
    /// the whole of *no deck*, and a pane pointed at nothing and a pane that
    /// is not there are one bay — one with no head to draw.
    ///
    /// **A pane is *pointed*, not choosing.** The mock's `showing … ▾` is a
    /// control and this pass adds none (ADR-0200), so which deck each pane
    /// shows is whoever fills this saying so — and pane *i* draws
    /// `inspector[i]`, in [`PANE_NAMES`]' order.
    ///
    /// **Written when a Set lands rather than per frame**, which is
    /// `karakuri_engine::set::Set::published`'s own instruction — *"Allocates,
    /// so not the frame path. A console reads this when a Set lands, not per
    /// frame."* — and is [`View::staging`]'s rule read one bay along: the two
    /// are written on the same frames and off the same drain, because a
    /// candidate row and a pane's rows are two readings of one event. Between
    /// landings what is in here does not move: nothing in this workspace
    /// writes a published value, binds a signal in the panel binary, or grants
    /// an authority (ADR-0216), so a Set that has landed reads the same on
    /// every frame after it until the next one does. **The field is rewritable
    /// per frame like every other one**; what the `man / sug / auto` chip
    /// still waits on is a writer for the authority, which is a different gap.
    /// See [`Pane`] and [`inspector`].
    pub inspector: Vec<Pane>,
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
    /// **Which classes the operator has opened to a model**, written per frame
    /// by whoever holds the run's opening.
    ///
    /// **Handed in like every other value here**, and for the sharper version
    /// of the usual reason: the model of record is a
    /// `karakuri_environment::Opening`, which is a handle a *second* surface
    /// reads on every MCP call — so a copy kept in this crate would be the
    /// console answering on the server's behalf, and would go on saying *open*
    /// after something else shut it. This crate names
    /// [`karakuri_operation::gate::Open`] and nothing in `karakuri-environment`
    /// at all (ADR-0156); [`McpPill::next`] hands a value back and whoever owns
    /// the handle writes it.
    ///
    /// **[`Open::CLOSED`] until somebody says otherwise**, which is every test
    /// in this crate and is the state ADR-0235 says a run starts in: four
    /// classes shut, and no way to write down an `Open` that starts open.
    pub opening: Open,
    /// **How long the panel has been animating**, written per frame by
    /// whoever has the clock — see [`Phase`], which carries the whole
    /// argument for why this is a value and not an `Instant`.
    ///
    /// **The same seam as [`View::transport`]**, and the one this crate is
    /// least able to cross: a clock in `src/` is P-0092 broken in the file
    /// whose own doc says so. [`Phase::ZERO`] until somebody says otherwise,
    /// which is every test in this crate and is a console with no clock behind
    /// it — a panel drawn at the origin of every animation on it.
    pub phase: Phase,
    /// **Which bay the keyboard is talking to, and what each bay remembers** —
    /// the pointer this console owns, and the one three of its fields used to
    /// be ([`crate::focus`]).
    ///
    /// **The deck selection, the library cursor and the marked scope are three
    /// readings of this**, on two bays: the Mixer's remembered item is the
    /// deck the keys are addressed to, the Library's is the row under the
    /// cursor, and the control last named under the Library's head is the
    /// scope. Each of the three was a private field with the same paragraph
    /// written at it — *"nothing downstream can be the model of record for it,
    /// and a host that kept a copy would be keeping the console's state on its
    /// behalf"* — and that paragraph is written once now
    /// ([ADR-0259](../../../../docs/adr/0259-the-keyboard-is-addressed-to-the-bay-that-has-focus-and-a-global-letter-is-a-convenience-or-the-operators-own.md),
    /// [ADR-0332](../../../../docs/adr/0332-focus-is-a-pointer-the-console-owns-and-the-three-pointers-are-instances-of-it.md)).
    /// **Nothing about what any of the three means changes**, which is the
    /// record's own clause: what each of them refuses is still refused where it
    /// was refused, by [`View::select`], [`View::walk`] and
    /// [`View::select_scope`].
    ///
    /// **Private, with [`View::select`], [`View::walk`], [`View::point_at`],
    /// [`View::select_scope`], [`View::step_scope`], [`View::tab`] and
    /// [`View::focus_up`] the only ways in**, which is [`View::arrangement`]'s
    /// rule: what a *pointer* is at is not something the program can be told,
    /// because the console is what refuses a deck there is no strip for.
    ///
    /// **Read by [`mixer::mixer_into`] for the solid ring, by [`View::focus_mark`]
    /// for the dashed one, and by nothing else on the panel.** The Library
    /// bay's foot read the selection for its letter until 2026-09-08 and reads
    /// [`View::target`] now (ADR-0305), which is what lets a load be aimed at a
    /// deck the keys are not addressed to; `console.html`'s crossfader read it
    /// as well, and there is no crossfader. What still fills its `deck` in from
    /// there is every deck-addressed *key*, `l` included.
    focus: Focus,
    /// **Which deck the Library bay's load is aimed at**, and the third
    /// pointer this console owns.
    ///
    /// **It is not [`View::selection`] and that is the point.** The selection
    /// is what a *key* press is addressed to; this is what the *button* in the
    /// Library bay's foot lands on, and the two are free to name two different
    /// decks — which is the one thing the selection cannot do, and the whole of
    /// what the pulldown bought
    /// ([ADR-0305](../../../../docs/adr/0305-the-library-bays-load-is-a-button-and-a-pulldown-and-the-deck-it-names-is-not-the-selection.md)).
    /// `l` goes on reading the selection.
    ///
    /// **It writes no record and no operation names it**, which is
    /// [`View::cursor_row`]'s argument one mark along: picking a deck in the
    /// pulldown changes what the *next* press will ask for and nothing about
    /// what any deck is playing, so nothing downstream can be the model of
    /// record for it and a host that kept a copy would be keeping the
    /// console's state on its behalf. `Operation::SelectDeck` is emphatically
    /// not what a pick emits: that operation moves the selection, and this one
    /// must not.
    ///
    /// **Private, with [`View::aim_at`] the only way in**, which is
    /// [`View::selection`]'s rule and for its reason: the console is what
    /// refuses a deck there is no strip for.
    ///
    /// Zero until somebody says otherwise — deck A, which is the letter the
    /// mock's pulldown reads. **Kept across everything that is not a deck**:
    /// changing the scope, narrowing the listing and walking the cursor all
    /// leave it alone, because none of them is about a deck.
    target: u8,
    /// **Whether the pulldown's list is down**, and it is the console's own
    /// state rather than a reading — [`Arrangement::menu`]'s argument on a
    /// third control: what a *control* is doing is not something the program
    /// can be the model of record for.
    ///
    /// **Private, with [`View::open_target`] and [`View::shut_target`] the
    /// only ways in.** `open_target` refuses a console the mixer is drawing no
    /// strip for: a card with no rows in it is a gesture with nothing to pick
    /// and nothing to leave by, and `input::claim`'s rule 2 would give it
    /// every press on the console until a second one shut it.
    target_open: bool,
    /// **Which `uses` line's card is down**, as `(pane, node, input)` — or
    /// `None` with none of them open, which is where every run begins.
    ///
    /// **The console's own state, exactly as [`target_open`] beside it is.** A
    /// card being down changes what the *next press* reaches and nothing about
    /// any deck, so no operation names it: opening a list is not something a
    /// map or a model could ever want to say, which is ADR-0305's argument for
    /// the pulldown one bay along and is this one's unchanged.
    ///
    /// **One at a time, and one for the whole console.** Two cards down at once
    /// would put two rectangles over the same pane with `input::claim`'s rule 2
    /// giving each of them every press; and a card belongs to a pane, a node and
    /// an input together, which is why the three travel as one value rather than
    /// as three fields that can disagree.
    ///
    /// [`target_open`]: Self::target_open
    wiring_open: Option<(usize, usize, usize)>,
    /// **Whether the `+ lane` chooser's card is down**, and it is the field
    /// above one bay along — the console's own state rather than a reading, on
    /// [`Arrangement::menu`]'s argument: what a *control* is doing is not
    /// something the program can be the model of record for.
    ///
    /// **Private, with [`View::open_lane`] and [`View::shut_lane`] the only
    /// ways in**, and `open_lane` refuses a chooser with nothing in it for
    /// `open_target`'s reason: `input::claim`'s rule 2 would give an empty card
    /// every press on the console until a second one shut it.
    lane_open: bool,
    /// **Which row of the Library bay's list has its menu down**, or `None`
    /// for none — the console's own state, exactly as [`target_open`] beside
    /// it is, and not a fourth mark: a menu is a card that is there or is not,
    /// and the row under it draws the same either way.
    ///
    /// **One field where the pulldown takes two**, and that is the whole of
    /// the difference between the two cards: a pulldown is a capsule that is
    /// drawn whether or not its list is down, so *which deck* outlives *is it
    /// open*; a row menu is the card and nothing else, so an open menu with no
    /// row under it cannot be spelled here.
    ///
    /// **Private, with [`View::open_menu`] and [`View::shut_menu`] the only
    /// ways in.** `open_menu` does **not** refuse a console the mixer is
    /// drawing no strip for, where `open_target` does: this card carries
    /// `Save as a kbset` under the separator whatever the mixer is doing, so
    /// there is always something to pick and something to leave by.
    ///
    /// [`target_open`]: Self::target_open
    menu_row: Option<usize>,
    /// **How far down its listing the Library bay is scrolled**, in logical
    /// pixels, as it is **stored** — the number [`library`] clamps and never
    /// the one it clamped.
    ///
    /// [`View::scroll`]'s shape one bay over and for its reasons, which are
    /// worth reading as a pair rather than restated here: the position is the
    /// console's own, no operation names it, nothing outside this crate could
    /// be the model of record for it, and it is not part of the arrangement —
    /// `karakuri-layout`'s tree holds sizes and folds and holds no scroll
    /// position
    /// ([ADR-0307](../../../../docs/adr/0307-the-inspectors-pane-scrolls-and-the-position-is-the-panes-own.md),
    /// [ADR-0312](../../../../docs/adr/0312-the-params-pill-is-a-toggle-and-the-library-bay-scrolls.md)).
    ///
    /// **One and not one per scope.** A scope is a different listing in the
    /// same bay, the way a build landing is a different pane's contents one
    /// bay over — and unlike the Inspector's two panes, which are two places
    /// in one list at once, only one scope is ever being read. A position per
    /// scope would be five numbers of which four are always stale, and the
    /// chip press that changes the listing is exactly the moment an operator
    /// wants the top.
    ///
    /// **Private, with [`View::library_scroll`] and
    /// [`View::scroll_library_by`] the only ways in.**
    library_scroll: f32,
    /// **What the Set under the cursor declares, opened** — or `None` for a
    /// bay with nothing open, which is where every run starts.
    ///
    /// **The reading is the host's and the opening is this console's**, which
    /// is the seam every other value in this bay crosses: what a Set declares
    /// is read off the cards the store keeps (ADR-0156, and P-0091 — it is a
    /// file read, done on the press and never on a frame), and *whether the
    /// block is down* is the console's own state, exactly as which row the
    /// cursor is on is. So a host answers [`Operation::ReadSet`] by writing
    /// this, and puts the reading away by clearing it.
    ///
    /// **One row is open at a time**, which is the mock's own rule and is what
    /// keeps this a mode of the list rather than a second list — so this is
    /// one reading and not a set of them. It carries the id it is of, and
    /// [`View::opened`] draws it only under the row of that name: a listing
    /// rewritten under an open reading closes it rather than filing it under
    /// whatever has taken that position.
    ///
    /// **Private, with [`View::read`] and [`View::shut_reading`] the only ways
    /// in**, which is [`View::selection`]'s rule one pointer up.
    reading: Option<Reading>,
    /// **Which of [`View::holds`] the `holds` field is set to**, or `None` for
    /// a field nobody has set — the fifth of this console's pointers.
    ///
    /// **A position and not a `String`**, for [`View::scope`]'s reason one
    /// field along: what the field can be set to is the row of candidates the
    /// host handed in, so what a pointer into it can be is a place in that row.
    ///
    /// **A stale position reads as unset rather than as the last candidate**,
    /// where [`View::scope`] and [`View::cursor_row`] both clamp. The two of
    /// them point at something drawn — a chip, a row — and the nearest one is
    /// the right answer; this one *narrows a listing*, and clamping it would
    /// leave the bay hiding Sets under a filter nobody chose.
    holds_at: Option<usize>,
    /// **Which kinds of row the listing is showing**, which is the six chips
    /// under the `holds` field — [`LibraryKinds::EVERYTHING`] where none of them
    /// is on, which is where every run starts.
    ///
    /// **A value and not a position**, where [`View::holds_at`] above it is a
    /// position, and it is [`View::transition`]'s distinction below: the kinds
    /// are the console's own closed list ([`KindChip::ALL`]) and cannot change
    /// under this pointer, where the `holds` candidates are a reading of a
    /// store that can.
    ///
    /// **It was `layer: Option<Layer>` until 2026-09-10**, the `layer…` field's
    /// value, and ADR-0338 replaced that field with these six toggles: a cycle
    /// names six of the sixty-four states this row has, and the two controls
    /// ask different questions — *which Sets hold a node on this layer*, which
    /// `Operation::ListSets` still carries, against *which kinds of row is this
    /// listing showing*.
    ///
    /// **`showing` and not `kinds`**, because [`View::kinds`] is already the
    /// row of what each *listed row* is: one field says what the bay is being
    /// asked for and the other says what came back, and two fields called
    /// `kinds` would be a name meaning two things.
    showing: LibraryKinds,
    /// **Which pane head is taking letters, and what has been typed into it**
    /// — or `None` for a console where nothing is being named, which is where
    /// every run starts.
    ///
    /// **The console's own state and not a reading**, which is
    /// [`Arrangement::menu`]'s argument on a second control: what a *control*
    /// is doing is this crate's, nothing about a half-typed name is saved,
    /// restored or reset, and a host that kept a copy would be keeping the
    /// console's gesture on its behalf.
    ///
    /// **It cannot live in [`View::inspector`]**, which is the reason it is a
    /// field here at all: a pane is rewritten whenever a Set lands, so a
    /// buffer kept in one would be a name that vanished mid-word.
    ///
    /// **Private, with [`View::name_set`] and the four methods beside it the
    /// only ways in** — [`View::selection`]'s rule, and see [`Naming`] for why
    /// there is one of these and not one per pane.
    naming: Option<Naming>,
    /// **How far each Inspector pane is scrolled**, one position per pane, and
    /// the console's eighth pointer.
    ///
    /// **A pointer and not a reading**, which is [`View::cursor_row`]'s
    /// argument arriving at a second bay: no operation names it, nothing
    /// downstream could be the model of record for it, and a host that kept a
    /// copy would be keeping the console's state on its behalf. It is not part
    /// of the arrangement either — `karakuri-layout`'s tree holds sizes and
    /// folds, which is what a saved arrangement carries — so a save does not
    /// take it and a restore does not move it
    /// ([ADR-0307](../../../../docs/adr/0307-the-inspectors-pane-scrolls-and-the-position-is-the-panes-own.md)).
    ///
    /// **It cannot live in [`View::inspector`]**, which is [`View::naming`]'s
    /// reason one field up: a pane is rewritten whenever a Set lands, so a
    /// position kept in one would go back to the top every time a build
    /// arrived — and a knob under an operator's hand would leave the screen on
    /// a frame nobody touched.
    ///
    /// **One per pane and not one per deck.** Two panes pointed at one deck
    /// are two places in one list, because what is scrolled is the pane; a
    /// position filed under the deck would move the pane an operator is not
    /// looking at.
    ///
    /// **Stored unclamped against the *pane*, clamped at the draw.** See
    /// [`InspectorPane::scroll`] and [`View::scroll_by`], which are the two
    /// halves of that: the pane's height is a viewport and P-0082 is why a
    /// solve may not write through it.
    ///
    /// **Private, with [`View::scroll_by`] and [`View::scroll_in`] the only
    /// ways in**, which is [`View::selection`]'s rule. Zero until somebody
    /// turns a wheel, which is every test in this crate and is a pane at the
    /// top of what its deck holds.
    scroll: [f32; PANES],
    /// **Which deck each Inspector pane is pointed at**, one per pane, and the
    /// console's ninth pointer — the pulldown on the pane head
    /// (`docs/adr/0338-…`, decision 5).
    ///
    /// **Beside [`View::scroll`] because it is the same kind of thing**: a
    /// pane's own answer, two panes are two of them, and neither is the
    /// console's. It is not [`View::selection`] — that is *the* deck, one
    /// value, what a key press is addressed to — and it is not
    /// [`View::target`] either, which is the Library bay's load mark. Three
    /// pointers naming a deck, and the whole reason there are three is that
    /// each answers a different question.
    ///
    /// **It cannot live in [`View::inspector`]**, which is [`View::naming`]'s
    /// and [`View::scroll`]'s reason: a pane is rewritten whenever a Set lands,
    /// so a target kept in one would be the host's answer read back as the
    /// console's question. [`Pane::deck`] is what the host *filled from this*,
    /// and the two are a pointer and a reading rather than two copies.
    ///
    /// **Private, with [`View::point_pane`] the only way in** —
    /// [`View::selection`]'s rule, and that method is what refuses a deck the
    /// mixer draws no strip for.
    ///
    /// **Deck A and deck B**, which is where a run opens and is the mock's own
    /// two heads.
    pane_deck: [u8; PANES],
    /// **Which pane head's pulldown is down**, or `None` — the console's own
    /// state, like [`View::target_open`] and [`Arrangement::menu`].
    ///
    /// One `Option` and not one flag per pane, for [`Naming`]'s reason read on
    /// a card: `input::claim`'s rule 2 hands every press to a card that is
    /// down, so two down at once would be two hands mid-choice with nothing on
    /// the panel saying which one the next press belongs to.
    pane_open: Option<usize>,
    /// **What the next fade, crossfade or wipe means** — the wipe's front
    /// shape and angle, the grid it starts on and how long it lasts — and the
    /// fourth of this console's pointers.
    ///
    /// **A pointer and not a reading**, which is [`View::selection`]'s
    /// argument arriving at a fourth control:
    /// [`Operation::SetTransition`] is `Silent(Surface)` — *"these change
    /// nothing you can see and write nothing to the stream"* — so nothing
    /// downstream can be the model of record for it, and a host that kept a
    /// copy would be keeping the console's state on its behalf. The host
    /// reads it to convert the next [`Operation::Wipe`], which is what
    /// `karakuri_operation_record::Current::transition` is asking for.
    ///
    /// **Four values and not a position in the three cycles**, where
    /// [`View::cursor_row`] and [`View::scope`] are both positions. Those
    /// point into a *listing this console was handed*, which can change under
    /// them; these three cycles are the console's own and cannot, and what
    /// the host has to be told is the shape and the two beat counts rather
    /// than where they sit in a table it cannot see.
    ///
    /// **Private, with [`View::set_transition`] the only way in**, which is
    /// [`View::selection`]'s rule: the console is what refuses a setting no
    /// pill can draw.
    ///
    /// [`TransitionSettings::START`] until somebody says otherwise, which is
    /// every test in this crate and is where a run begins.
    transition: TransitionSettings,
    /// **What the Sequencer bay reads this frame**, or `None` for a console
    /// with no sequencer behind it — which is every test in this crate that
    /// does not hand one in, and what the bay draws then is nothing at all.
    ///
    /// **The same seam as [`View::mixer`]**: a pattern is authored state a
    /// *session* holds and `src/` holds no session (ADR-0156), so whoever owns
    /// one writes this per frame beside the frame it is about. It is not this
    /// console's own pointer — a press here hands back an
    /// [`Operation`] and applies nothing, exactly as a fader's drag does.
    ///
    /// **Why the reading is a whole pattern**: see [`Sequenced`], which is
    /// where the alternative — a field per drawn thing — is refused.
    pub sequencer: Option<Sequenced>,
    placed: Vec<Placed>,
}

impl View {
    pub fn new(room: Room) -> View {
        View {
            room,
            // **A console with no engine behind it has no projector window**,
            // which is every test in this crate and every frame before
            // whoever owns the window has said otherwise.
            projector: false,
            picture: None,
            previews: [None; DECKS],
            // Nothing is stopped on a console with no engine behind it, which
            // is every test in this crate.
            overloaded: [false; DECKS],
            costs: [None; DECKS],
            transport: None,
            // The default arrangement, nothing filed and the menu shut, which
            // is every test in this crate and is a console with no store
            // behind it.
            arrangement: Arrangement::NONE,
            // Nobody has said anything about audio, which is every test in
            // this crate: no pill at all, rather than one reading `none` on
            // this crate's own authority. See the field.
            audio: None,
            // **Never armed at start-up**, which is the state every mode on
            // this panel opens in for P-0094's reason: a console that came up
            // in a mode nobody chose would rebind the first knob touched.
            learn: false,
            // Nobody has said, which draws no pill — [`View::map`].
            map: None,
            // And nothing said about the beat tracker, which is the same
            // console from the other side: no offset, no tap and no octave.
            tracker: None,
            // No engine behind the console, so there is no look to draw — the
            // transport's own answer, one group along.
            look: None,
            // And no level at the other end of the same chain, for the same
            // reason: the Master bay draws its head and nothing under it.
            master_out: None,
            master_chain: None,
            // As many strips as a deck can ever have, so the frame path never
            // grows it — the same reason `placed` is built with a capacity.
            mixer: Vec::with_capacity(DECKS),
            // Nothing until somebody lists a store, which is every test in
            // this crate. No capacity is reserved: how many Sets a store holds
            // is not a number this crate has, and the list is written once
            // rather than per frame.
            library: Vec::new(),
            kinds: Vec::new(),
            // **Nothing starred**, which is a console with no store behind it
            // and is every test in this crate that does not say otherwise —
            // `View::library`'s rule one field down.
            starred: std::collections::BTreeSet::new(),
            // **And no Set aimed at**, which is a console with no engine behind
            // it and is also where every real run starts: a slot plays the pair
            // the command line settled until somebody loads a Set (ADR-0304),
            // so a walk asked for before that names no Set and lists nothing.
            aimed: None,
            // And nothing said about what libraries there are, which is the
            // same console from the other side: no chips, and so no scope
            // row. Room for the four the mock draws, so a host that says so
            // at startup does not grow it — `mixer`'s reason, one row up.
            scopes: Vec::with_capacity(Scope::ALL.len()),
            // And nothing to narrow by, which is the same console again: the
            // `holds` field is drawn wherever the scope row is and a press on
            // it asks for the listing over again until a host says what this
            // store's Sets are made of. No capacity is reserved, for
            // `library`'s reason one field up.
            holds: Vec::new(),
            // **And pointed nowhere**, which is where every run begins: no
            // folder has been dropped on the window, so there is no `.path`
            // row and the scopes sit straight on the filters. Nothing over the
            // window either — the second is a drag that is happening now, and
            // one is not.
            folder: None,
            incoming: None,
            // Neither field set, which is the whole library rather than a
            // narrowed one — where a run begins, and every test in this crate
            // that does not say otherwise.
            holds_at: None,
            showing: LibraryKinds::EVERYTHING,
            // **No head asking for a name**, which is where a run starts and
            // is every test in this crate that does not say otherwise: the two
            // pane heads are readouts until a hand lands on one of them.
            naming: None,
            // Every pane at the top of what its deck holds, which is where a
            // run starts and is every test in this crate that does not turn a
            // wheel.
            scroll: [0.0; PANES],
            // **Deck A in the first pane and deck B in the second**, which is
            // the mock's two heads and is what this console showed before the
            // pulldown existed: the host filled pane `n` from slot `n`, and
            // that arrangement is now a *default* rather than a rule.
            pane_deck: PANE_DECKS,
            // **No pulldown down**, which is where a run starts and is every
            // test in this crate that does not open one.
            pane_open: None,
            // Nothing outstanding on any slot, which is a console with no
            // engine behind it and is also every ordinary frame of one that
            // has. Room for as many rows as a deck can ever have slots, so
            // the caller's write never grows it — `mixer`'s reason, one bay
            // up.
            staging: Vec::with_capacity(DECKS),
            // As many panes as the inspector has, so the frame path never
            // grows it — the same reason `mixer` is built with a capacity.
            inspector: Vec::with_capacity(PANES),
            canvas: MOCK_CANVAS,
            // Four classes shut, which is the run ADR-0235 describes and is
            // also a console nobody has handed an opening to.
            opening: Open::CLOSED,
            phase: Phase::ZERO,
            // **Nothing addressed and no bay named**, which is the honest
            // start rather than a table of zeroes: `Focus::bay` answers *the
            // first bay the traversal reaches* off the arrangement, and every
            // bay's remembered address is the first thing it drew until
            // somebody moves it. So deck A and the first library row are still
            // where the marks are — the mock's own two — and neither is a
            // reading of anything, so neither has a *nothing* to be: a console
            // with no deck draws no strips and so no ring, and one with no
            // store draws no rows and so no cursor.
            focus: Focus::default(),
            // **Deck A, and it is a third mark rather than a copy of the
            // first.** Both start on A because that is where the mock draws
            // both, and nothing keeps them together after that: `select` moves
            // one and `aim_at` moves the other.
            target: 0,
            // **Shut**, which is not a fourth mark: a list is down or it is
            // not there, and the pulldown draws the same either way.
            target_open: false,
            wiring_open: None,
            // **Shut**, for the reason the pulldown's list is: the `+ lane`
            // pill draws the same whether its card is down or not.
            lane_open: false,
            // **No menu**, which is not a mark either, for the reason above
            // it: a card is down on a row or there is no card.
            menu_row: None,
            // **The top of the listing**, which is where every bay starts and
            // the one position a console with no store behind it can be at.
            library_scroll: 0.0,
            // **Nothing open**, which is not a fourth mark: a reading is a
            // block of rows or it is not there, and the chip that opens one
            // draws the same either way — `PARAMS_PILL`.
            reading: None,
            // No shape, the next bar and four beats — where a run begins, and
            // `karakuri-cli`'s own opening state. Not a reading of anything
            // either, for the three above's reason.
            transition: TransitionSettings::START,
            // **Nothing said about a sequencer**, which is a console with no
            // session behind it and draws no ruler, no rows and no head —
            // `View::mixer`'s empty list one bay along, in the `Option` shape
            // `View::transport` uses for the same seam.
            sequencer: None,
            // Every region the console has, so the frame path never grows it.
            placed: Vec::with_capacity(REGIONS.len()),
        }
    }

    /// **Which deck the keys are addressed to** — the Mixer bay's remembered
    /// address, read as a deck.
    ///
    /// [`Operation::SelectDeck`] *"writes no record, and is the reason every
    /// other variant names its deck instead of meaning the selected one"* — so
    /// nothing downstream can be the model of record for it, and a host that
    /// kept a copy would be keeping the console's state on its behalf.
    /// [ADR-0219](../../../../docs/adr/0219-the-crossfader-spans-the-selection-and-the-one-after-it.md)
    /// recorded it as living *"in the specification and not in
    /// `karakuri-console`'s code"*; [`View::focus`] the field is where that
    /// stopped being true, and ADR-0259 is why it is the same field the library
    /// cursor and the marked scope are in.
    ///
    /// **Deck A for a bay nobody has addressed**, which is the strip the mock
    /// rings. The digit that names the first strip is `1` — digits count what
    /// the bay drew — and this is the one place the step down to a deck index
    /// is written.
    ///
    /// [`View::focus`]: Self::focus
    pub fn selection(&self) -> u8 {
        self.focus
            .address(focus::MIXER)
            .and_then(|address| address.remembered(&[]))
            .map_or(0, |nth| nth.saturating_sub(1) as u8)
    }

    /// **Address the keys to `deck`**, and answer whether that moved anything.
    ///
    /// **A deck the mixer has no strip for is refused**, and that is the whole
    /// of the rule: the selection is drawn as a ring round a strip and is what
    /// every deck-addressed key names its deck by, so a selection past the
    /// deck's slots would be a ring nowhere and a key aimed at a deck the
    /// press would be turned down on. [`View::aim_at`] refuses on the same
    /// count one mark along. [`View::mixer`] is *"one per slot the deck
    /// has"*, so its length is the deck's own count arriving the way every
    /// other reading does — and a console with no deck behind it has no strip
    /// to select, which is every test in this crate.
    ///
    /// **It refuses rather than clamping.** A press on `3` at a two-slot deck
    /// means *deck D* and there is no deck D; clamping would answer *deck B*,
    /// which is a different deck than the one asked for and would move the mix
    /// under a hand that asked for nothing of the sort.
    ///
    /// The `bool` is [`Arrangement::typed`]'s: a caller repaints on a move and
    /// not on a press, so a press that changed nothing costs no frame
    /// (P-0091).
    pub fn select(&mut self, deck: u8) -> bool {
        if usize::from(deck) >= self.mixer.len() {
            return false;
        }
        let moved = self.selection() != deck;
        // **The Mixer bay's remembered item, one-based**, which is the digit
        // that names the strip: `1` is deck A. See [`crate::focus::Address`].
        self.focus
            .address_mut(focus::MIXER)
            .remember(&[], usize::from(deck) + 1);
        moved
    }

    /// **Where focus is and what every bay remembers, to read** —
    /// [`View::focus`] the field, which is where the argument is.
    ///
    /// **A reading and not a door.** It is `&`, so the three pointers still
    /// have exactly the writers they had: `select`, `walk`, `point_at`,
    /// `select_scope` and `step_scope` are what refuse a deck the mixer draws
    /// no strip for, a row past the listing and a scope with no chip, and a
    /// `&mut` here would be a way past all five. What it is for is asking one
    /// question — *are these the same field* — which `tests/focus.rs` asks and
    /// nothing else can.
    ///
    /// [`View::focus`]: Self::focus
    pub fn focus(&self) -> &Focus {
        &self.focus
    }

    /// **Where focus is and what every bay remembers, to write** — and it is
    /// `pub(crate)` where [`View::focus`] above is `pub`.
    ///
    /// **What it is for is the address path and not the memory.**
    /// [`crate::focus::press`] moves `Address::at` as a digit descends and
    /// `esc` climbs, and every write to the *remembered* item still goes
    /// through [`View::select`], [`View::walk`], [`View::point_at`],
    /// [`View::select_scope`] and [`View::step_scope`] — the five methods that
    /// refuse a deck the mixer draws no strip for, a row past the listing and a
    /// scope with no chip. **The grammar adds a route and no exception**, which
    /// is the whole of why this door is the crate's and not the world's.
    pub(crate) fn focus_mut(&mut self) -> &mut Focus {
        &mut self.focus
    }

    /// **Which bay a key press is addressed to**, resolved against the
    /// arrangement — [`Focus::bay`], which is where the argument is.
    ///
    /// `None` only for an arrangement with no bay in it, which no arrangement
    /// this crate builds is.
    pub fn focused(&self, panel: &Panel) -> Option<&'static Region> {
        self.focus.bay(panel.layout())
    }

    /// **`Tab`, and `shift-Tab` at `step` of `-1`** — [`Focus::tab`], and the
    /// `bool` is [`View::select`]'s: a caller repaints on a move.
    ///
    /// **The bay it leaves keeps where it was.** `Tab` never descends and never
    /// pops, so the address the Mixer had reached is the address it has when
    /// focus comes back to it — which is the deck selection persisting *"while
    /// your hands are in the library"*, seen as the general rule rather than as
    /// a habit of one bay.
    pub fn tab(&mut self, panel: &Panel, step: i32) -> bool {
        self.focus.tab(panel.layout(), step)
    }

    /// **`esc`: up one level of the focused bay's address**, and `false` where
    /// there was no level to leave — [`Focus::up`].
    ///
    /// **At bay level it acts on nothing and the caller says so**, which is
    /// ADR-0259's own clause and
    /// [P-0083](../../../../docs/principles/0083-a-refusal-carries-what-the-next-attempt-needs.md):
    /// there is no unfocused state to fall out into, and a key that declines
    /// silently is indistinguishable from one that is not bound. **It does not
    /// quit** — the window's own close is what does
    /// ([ADR-0315](../../../../docs/adr/0315-a-model-has-no-window-so-the-twelve-surface-rows-mcp-badges-are-gap.md)).
    pub fn focus_up(&mut self, panel: &Panel) -> bool {
        self.focus.up(panel.layout())
    }

    /// **Where the dashed focus ring goes**, or `None` where the arrangement
    /// gives the focused bay no rectangle to put one on — [`focus::mark`], with
    /// the bay this console has focus on.
    ///
    /// **A derivation rather than a paint**, which is this crate's arrangement
    /// everywhere a mark is drawn: [`View::draw`] paints from this and
    /// `tests/focus.rs` asks it the same question, so the ring an operator sees
    /// and the ring a test reads cannot come apart.
    ///
    /// `panel` must be solved: [`Layout::rect`] refuses to answer from a dirty
    /// one.
    pub fn focus_mark(&self, panel: &Panel) -> Option<Rect> {
        focus::mark(panel.layout(), self.focus.bay(panel.layout())?)
    }

    /// **The mark a *folded* bay holding focus wears**, or `None` where the
    /// focused bay is not folded — [`focus::folded_head`], with the bay this
    /// console has focus on, and the title to paint in it.
    ///
    /// **A folded region has no rectangle**, so the ring alone would land on
    /// nothing an operator could read; `docs/manual/console.html` specifies the
    /// head alone, and this is where the panel draws it. It is [`View::draw`]'s
    /// one mark painted over the bays rather than inside one, for the reason
    /// the five cards are: the edge a fold leaves belongs to whatever is drawn
    /// next to it, and this is only ever drawn while an operator has
    /// deliberately tabbed onto the bay that is not there.
    ///
    /// `panel` must be solved: [`Layout::rect`] refuses to answer from a dirty
    /// one.
    pub fn folded_mark(&self, panel: &Panel) -> Option<(Rect, &'static str)> {
        let bay = self.focus.bay(panel.layout())?;
        let mark = focus::folded_head(panel.layout(), bay)?;
        // A headless row has no title to draw, and the mark is what says a bay
        // is there — so the word alone stands for it, exactly as the row
        // stands in for the head that is not drawn (ADR-0159, ADR-0259).
        Some((mark, head_of(bay).map_or("", |head| head.title)))
    }

    /// **Which `uses` line's card is down**, or `None` — see
    /// [`View::wiring_open`] the field.
    pub fn wiring_open(&self) -> Option<(usize, usize, usize)> {
        self.wiring_open
    }

    /// **Put one `uses` line's card down**, and answer whether anything moved.
    ///
    /// **Refused for a console with no pane**, which is [`View::open_target`]'s
    /// own guard and its reason: a card with no line under it offers nothing to
    /// pick and nothing to leave by, and `input::claim`'s rule 2 would hand it
    /// every press until a second one shut it.
    pub fn open_wiring(&mut self, pane: usize, node: usize, input: usize) -> bool {
        if self.inspector.get(pane).is_none() {
            return false;
        }
        let at = Some((pane, node, input));
        let moved = self.wiring_open != at;
        self.wiring_open = at;
        moved
    }

    /// **Put it away**, and answer whether one was down.
    pub fn shut_wiring(&mut self) -> bool {
        let was = self.wiring_open.is_some();
        self.wiring_open = None;
        was
    }

    /// **What the `+ lane` chooser offers this frame** — the one value
    /// [`sequencer`] lays its card out from, and [`View::target`]'s shape one
    /// bay along: read once for the frame and handed to the paint and to the
    /// press, so the item that is drawn and the item a press lands on are one
    /// derivation of one reading.
    ///
    /// **The faders are every strip the mixer draws**, which is
    /// [`View::select`]'s count read again; **the parameters are one deck's**,
    /// the load pulldown's ([`View::target_deck`]) — see [`Choices`], which
    /// carries the argument.
    pub fn lane_choices(&self) -> Choices {
        let mut items: Vec<LaneChoice> = (0..self.mixer.len().min(DECKS))
            .map(|deck| {
                let target = LaneTarget::Fader { deck: deck as u8 };
                LaneChoice {
                    words: format!("{} {FADER_ITEM}", lane_label(&target)),
                    target,
                }
            })
            .collect();
        let faders = items.len();
        let deck = self.target_deck();
        for pane in self
            .inspector
            .iter()
            .filter(|pane| pane.deck == usize::from(deck))
        {
            for node in &pane.nodes {
                for param in &node.params {
                    let target = LaneTarget::Param {
                        deck,
                        param: param.param.clone(),
                    };
                    items.push(LaneChoice {
                        // **The address and the published name**, which is the
                        // mock's own way of naming this lane — *"L2:0 twist on
                        // deck B"* — with the deck's letter and mark in front
                        // of it so the item reads as the row it will make.
                        words: format!("{} {} {}", lane_label(&target), node.addr, param.name),
                        target,
                    });
                }
            }
        }
        Choices {
            items,
            faders,
            open: self.lane_open,
        }
    }

    /// **Whether the `+ lane` chooser's card is down** — see
    /// [`View::lane_open`] the field.
    pub fn lane_open(&self) -> bool {
        self.lane_open
    }

    /// **Put the card down**, and answer whether it went down.
    ///
    /// **Refused where there is nothing to point at**, which is
    /// [`View::open_target`]'s rule: a card with no items offers nothing to
    /// pick, and [`crate::input::claim`]'s rule 2 would give it every press on
    /// the console until a second press shut it again.
    pub fn open_lane(&mut self) -> bool {
        if self.lane_open || self.lane_choices().items.is_empty() {
            return false;
        }
        self.lane_open = true;
        true
    }

    /// **Take the card away**, and answer whether there was one down —
    /// [`View::shut_target`]'s shape and its reason.
    pub fn shut_lane(&mut self) -> bool {
        let was = self.lane_open;
        self.lane_open = false;
        was
    }

    /// **How far one Inspector pane is scrolled**, as it is stored — the
    /// number [`inspector`] clamps and never the one it clamped.
    ///
    /// Zero for a pane index past [`PANES`], which is a caller's error and not
    /// a state: the bay has two panes and `PANE_NAMES` is what says so.
    pub fn scroll_in(&self, pane: usize) -> f32 {
        self.scroll.get(pane).copied().unwrap_or(0.0)
    }

    /// **Turn one pane's wheel by `by` pixels**, positive down the list, and
    /// answer whether the stored position moved.
    ///
    /// # Two clamps, and only one of them is here
    ///
    /// This one is against the **content** — how tall everything the deck
    /// publishes comes to — and it is a reading of the deck rather than of a
    /// viewport, so a stored position bounded by it is not a position any
    /// resize can rewrite. Without it a wheel spun over a short Set would put
    /// the number in the thousands and an operator would have to spin it all
    /// the way back before anything moved, which is *"a control you cannot see
    /// being moved"* by another name.
    ///
    /// The other clamp is against the pane's own height and belongs where the
    /// pane is laid out — [`InspectorPane::scroll`], which is
    /// [P-0082](../../../../docs/principles/0082-looking-never-writes-back.md):
    /// a shorter pane draws less of the same position and stores nothing, so
    /// dragging it back reproduces the picture exactly rather than nearly
    /// (ADR-0250's argument one region in).
    ///
    /// **A pane this console is not showing anything in refuses the wheel**
    /// rather than storing a position for it, which is [`View::point_at`]'s
    /// rule: what a pointer can be at is something drawn.
    pub fn scroll_by(&mut self, pane: usize, by: f32) -> bool {
        let Some(showing) = self.inspector.get(pane) else {
            return false;
        };
        let Some(at) = self.scroll.get_mut(pane) else {
            return false;
        };
        let content = content_h(&showing.nodes);
        let next = (*at + by).clamp(0.0, content);
        let moved = next != *at;
        *at = next;
        moved
    }

    /// **Which deck one Inspector pane is pointed at** — see
    /// [`View::pane_deck`] the field, which is where the argument is.
    ///
    /// **This is what the host reads to fill the pane**, which is
    /// [`View::target_deck`]'s arrangement one bay along: the pointer is the
    /// console's and what is under it is the host's answer to *what is that
    /// deck playing*.
    ///
    /// Deck A for a pane index past [`PANES`], which is a caller's error and
    /// not a state — [`View::scroll_in`]'s own rule.
    ///
    /// [`View::pane_deck`]: Self::pane_deck
    pub fn pane_deck(&self, pane: usize) -> u8 {
        self.pane_deck.get(pane).copied().unwrap_or(0)
    }

    /// **Every pane's target at once**, which is what a host reads before it
    /// fills [`View::inspector`]: the panes are written through a `&mut` of
    /// that field, so asking pane by pane while it is borrowed is a second
    /// borrow of this struct. It is two bytes and `Copy`.
    pub fn pane_decks(&self) -> [u8; PANES] {
        self.pane_deck
    }

    /// **Point one pane at `deck`**, put the card away, and answer whether
    /// anything moved.
    ///
    /// **A deck the mixer has no strip for is refused**, which is
    /// [`View::select`]'s rule and [`View::aim_at`]'s read a third time rather
    /// than a third rule: the head says which deck it is showing, so a target
    /// past the deck's slots would be a letter naming a deck with nothing
    /// under it.
    ///
    /// **It refuses rather than clamping**, for `select`'s reason: a pick of
    /// deck D at a two-slot deck means *deck D*, and clamping would point the
    /// pane at deck B — a different deck than the one asked for.
    ///
    /// **Nothing else moves**, and that is the whole of what this mark is for:
    /// not the deck selection, not the pane next door, not the Library bay's
    /// load target. Each of those is a pointer of its own with a writer of its
    /// own, and this one touches none of them.
    ///
    /// **The card goes away here**, which is [`View::aim_at`]'s clause: a pick
    /// is one gesture and this is the whole of it, so a card left down would
    /// go on claiming every press on the console. It is put away even where
    /// the pane did not move — picking the deck a pane already shows is still
    /// a hand finishing what it started.
    ///
    /// The `bool` is [`View::select`]'s: a caller repaints on a move and not
    /// on a press.
    pub fn point_pane(&mut self, pane: usize, deck: u8) -> bool {
        if usize::from(deck) >= self.mixer.len() {
            return false;
        }
        let Some(at) = self.pane_deck.get_mut(pane) else {
            return false;
        };
        let moved = *at != deck || self.pane_open.is_some();
        *at = deck;
        self.pane_open = None;
        moved
    }

    /// **Which pane head's pulldown is down**, or `None` — see
    /// [`View::pane_open`] the field.
    ///
    /// [`View::pane_open`]: Self::pane_open
    pub fn pane_target_open(&self) -> Option<usize> {
        self.pane_open
    }

    /// **Put one pane head's card down**, and answer whether anything moved.
    ///
    /// **Refused for a pane this console is not showing**, which is
    /// [`View::open_wiring`]'s own guard and its reason: a card with no head
    /// under it offers nothing to pick and nothing to leave by, and
    /// `input::claim`'s rule 2 would hand it every press until a second one
    /// shut it.
    pub fn open_pane_target(&mut self, pane: usize) -> bool {
        if pane >= PANES || self.inspector.get(pane).is_none() {
            return false;
        }
        let moved = self.pane_open != Some(pane);
        self.pane_open = Some(pane);
        moved
    }

    /// **Put it away**, and answer whether one was down.
    pub fn shut_pane_target(&mut self) -> bool {
        let was = self.pane_open.is_some();
        self.pane_open = None;
        was
    }

    /// **What one pane head's pulldown is**, laid out — the mark and, while it
    /// is down, the card under it.
    ///
    /// Read once for the frame and handed to the paint and to the press,
    /// exactly as [`View::target`] is: the mark that is drawn and the mark a
    /// press lands on are one derivation of one reading.
    ///
    /// `None` is a head with no run in it, which is [`deck_name`]'s refusal —
    /// see [`pane_target`].
    pub fn pane_pulldown(
        &self,
        ctx: &egui::Context,
        at: &InspectorPane,
        pane: &Pane,
        index: usize,
    ) -> Option<PaneTarget> {
        pane_target(
            ctx,
            at,
            pane,
            index,
            self.naming_set_in(index),
            self.mixer.len(),
            self.pane_open == Some(index),
        )
    }

    /// **Every live region that is declaring this frame**, each with what one
    /// update of it costs, how stale it may get, and when its picture is next
    /// different from the one on screen.
    ///
    /// The first two are P-0091's and are constants of the presentation; the
    /// third is [`crate::budget::Declared::moves_in`], it is a function of the
    /// frame, and it exists because *how often must this be drawn* and *is
    /// this moving now* are two questions and only one of them was being asked
    /// ([ADR-0283](../../../../docs/adr/0283-a-region-declares-when-its-picture-next-changes-not-that-something-is-pending.md)).
    ///
    /// # This is P-0091's naming, and the unit is a region
    ///
    /// [P-0091](../../../../docs/principles/0091-cost-is-known-before-it-is-paid.md):
    /// *"Anything that must be live names two numbers — what its update costs,
    /// and how stale it may get in milliseconds."* This is that naming, and [`crate::budget`] holds the
    /// numbers with the arguments for where each came from. **A region is
    /// redrawn whole or not at all**, so what appears here is a node of the
    /// arrangement — by the name every surface addresses it by — and never an
    /// animation, a control or a slice of a frame.
    ///
    /// # Two regions, at two rates, for two different reasons
    ///
    /// **The transport row**, whenever the beat grid is drawn: the light
    /// travels the grid once a bar and it is the panel's continuous motion,
    /// which [P-0094](../../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)
    /// says is how a stopped panel announces itself. It declares
    /// [`BEAT_STALENESS`] and **it does not ask whether anything is pending**,
    /// which is the whole point of it: a signal that only ran while something
    /// was happening would be quiet exactly when the panel had gone quiet.
    ///
    /// **The mixer bay**, while something in it is outstanding. Inside it the
    /// tally's word rolls toward a residency that has not been granted and
    /// each of a strip's two faders reaches toward a value a transition has
    /// not reached yet — three presentations at one rate, off one [`Phase`],
    /// so they are one term and not three
    /// ([ADR-0190](../../../../docs/adr/0190-the-parked-tally-rolls-because-two-lamps-do-not-fit-in-fifty-three-pixels.md),
    /// [ADR-0206](../../../../docs/adr/0206-a-fader-marks-where-it-is-going-and-keeps-reaching-for-it.md)).
    /// A second *rate* would be a second declaration; a second *user* of one
    /// rate is not — which is why the beat is a second entry here and the two
    /// faders are not.
    ///
    /// **This is the first frame on which either sum has two terms**, and
    /// `tests/schedulable.rs` reads them: `Σ (cost / staleness)` is 0.0889
    /// against 1.0, and `max(cost)` is still one number because both regions
    /// declare the same whole panel pass (ADR-0210).
    ///
    /// # Pending is not enough: the region that shows it has to be laid out
    ///
    /// **A region that is not on screen declares nothing**, however much is
    /// pending behind it. The strips are rewritten every frame from the deck,
    /// so *is anything pending* is a fact about the deck; P-0091 is about what
    /// **must be live**, and a bay the operator has folded away is not live.
    /// A declaration made for it buys a repaint of something nobody can see —
    /// measured, before this asked: with the picture and the preview row
    /// folded the window sat at 28.7 to 29.0 frames a second, and folding the
    /// mixer bay on top of that moved the price of a frame and not the rate.
    /// It draws nothing at all now. The alternative — declare it anyway and
    /// let a scheduler drop it — is
    /// [ADR-0193](../../../../docs/adr/0193-a-region-that-is-not-laid-out-declares-nothing-rather-than-being-dropped-later.md),
    /// which is where it lost, and it lost on the sentence being **false**
    /// rather than unaffordable: a region nobody can see is not showing
    /// anything, so it cannot be showing anything out of date.
    ///
    /// This is
    /// [P-0073](../../../../docs/principles/0073-a-node-claims-only-what-its-visible-content-can-use.md)
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
    /// ADR-0164's second half is a scheduler and there is not one. What this
    /// feeds is [`View::animating`], which takes the soonest staleness and
    /// nothing else, and `tests/schedulable.rs`, which sums over whatever this
    /// answers and asserts the two conditions the principle states.
    pub fn declares(&self, layout: &karakuri_layout::Layout) -> impl Iterator<Item = Declared> {
        // An array rather than a `Vec`, so asking what the panel declares
        // allocates nothing on a path that is walked every frame — and so that
        // the second live region was one more element rather than a change of
        // shape, which is what it turned out to be. In `REGIONS`' order, so
        // the declarations read down the panel.
        [
            self.transport_declares(layout),
            self.mixer_declares(layout),
            self.sequencer_declares(layout),
        ]
        .into_iter()
        .flatten()
    }

    /// **What the transport row declares**: the beat's staleness for as long
    /// as the grid is being drawn, and nothing when it is not.
    ///
    /// # Two conditions, and neither of them is *pending*
    ///
    /// **The row is laid out**, which is ADR-0193 asked of a row rather than
    /// of a bay — [`Layout::visible`](karakuri_layout::Layout::visible)
    /// answers for `transport` exactly as it answers for `mixer`, and a folded
    /// row is not showing a beat, so it cannot be showing one out of date.
    /// The transport is a direct child of the unnamed root, so the only thing
    /// that encloses it is the root itself (ADR-0204).
    ///
    /// **There are values behind it.** [`View::transport`] is `None` for a
    /// console with no engine, and [`transport`] draws no row at all then —
    /// there is no light on a grid that is not there. That is the same seam as
    /// [`View::picture`] and not a second rule.
    ///
    /// **Nothing else is asked, and that is the declaration's whole content.**
    /// P-0094's forced clause is that *something is moving continuously while
    /// the console is live*, so a beat that declared only while something was
    /// pending would be the signal going quiet at the moment it is worth
    /// having. It is also why this is not folded into
    /// [`View::mixer_declares`]: two rates, two deadlines, two regions.
    ///
    /// # The health capsule declares nothing, and that is the right answer
    /// rather than an omission
    ///
    /// [`Transport::health`] changes when a build lands, is thrown out or
    /// fails to assemble, which is a person saving a file — so its own
    /// `moves_in` is *not until something happens*, and a region cannot
    /// declare that. It does not have to: **the unit of declaration is the
    /// region and not the presentation** ([`Declared`] — *"a second rate would
    /// be a second declaration; a second user of one rate is not"*), and the
    /// region it is in is already drawn at [`BEAT_STALENESS`] for as long as
    /// there is a row at all. A staleness written for the capsule would be a
    /// rate for something that does not move (ADR-0283), and it could only
    /// ever be slower than the beat's, so it would change nothing a window
    /// does — [`View::animating`] takes the soonest.
    fn transport_declares(&self, layout: &karakuri_layout::Layout) -> Option<Declared> {
        let row = layout
            .find("transport")
            .is_some_and(|id| layout.visible(id));
        (row && self.transport.is_some()).then_some(Declared {
            region: "transport",
            cost: PANEL_PASS,
            staleness: BEAT_STALENESS,
            // **The one region whose two numbers are the same number**, and
            // that is what an honest *I move at this rate* looks like: the
            // light is on the grid at [`Transport::beats`], the harness
            // advances the session by one step on every frame it composes, so
            // every frame this declaration buys draws the light somewhere it
            // was not (ADR-0283). There is no rest to find and nothing to
            // gate — P-0094 is the rule that says there had better not be.
            moves_in: BEAT_STALENESS,
        })
    }

    /// **What the Sequencer bay declares**: the step's staleness while it has a
    /// playhead to move *and* the bay is laid out, and nothing otherwise —
    /// with the deadline taken from where the beat has got to inside the
    /// current step.
    ///
    /// # Three conditions, and the third is what makes it honest
    ///
    /// **The bay is laid out**, which is ADR-0193 and is
    /// [`View::mixer_declares`]'s first condition word for word.
    ///
    /// **There is a pattern behind it.** [`View::sequencer`] is `None` for a
    /// console with no session, and [`sequencer`] draws nothing then.
    ///
    /// **And it has a lane.** The picture that moves here is the playhead
    /// column, which stands over the rows — so a pattern with no lanes has
    /// nothing for it to stand on and this bay is a head and a ruler that do
    /// not move. A declaration made for it would buy frames that redraw a
    /// still picture, which is the whole of what
    /// [ADR-0283](../../../../docs/adr/0283-a-region-declares-when-its-picture-next-changes-not-that-something-is-pending.md)
    /// is about. **A muted lane still counts**: the mute stops the lane
    /// *writing*, and the column goes on crossing its cells.
    ///
    /// # The deadline is the music's and the rate is not
    ///
    /// [`step_moves_in`] is a function of `beats` and the tempo, so the
    /// deadline moves with the grid the way the steps do; [`STEP_STALENESS`]
    /// is a constant at the mock's tempo, so the sums `tests/schedulable.rs`
    /// asserts do not become a function of how fast the music is (ADR-0212).
    fn sequencer_declares(&self, layout: &karakuri_layout::Layout) -> Option<Declared> {
        let bay = layout
            .find("sequencer")
            .is_some_and(|id| layout.visible(id));
        let reading = self.sequencer.as_ref()?;
        let transport = self.transport.as_ref()?;
        (bay && !reading.pattern.lanes().is_empty()).then(|| Declared {
            region: "sequencer",
            cost: PANEL_PASS,
            staleness: STEP_STALENESS,
            moves_in: step_moves_in(reading.pattern.mode(), transport.beats, transport.bpm),
        })
    }

    /// **The soonest any live region on this panel will next look different
    /// from what is on screen**, and `None` when nothing on it is moving.
    ///
    /// # It is `moves_in` and not `staleness`, and that is the whole of
    /// ADR-0283
    ///
    /// A staleness says how finely a region has to be drawn *while it moves*;
    /// it does not say whether the region is moving now. The mixer bay's roll
    /// rests for 600 ms of every second and its curve is exactly zero
    /// throughout, so a deadline taken from the staleness alone woke this
    /// window seventeen times a second to draw a chip in the position it was
    /// already in. [`crate::budget::Declared::moves_in`] is what a region
    /// answers instead, `staleness` stays the constant `tests/schedulable.rs`
    /// sums, and the invariant between them — `moves_in >= staleness` — is why
    /// this can only take a frame away and never bring one forward
    /// ([ADR-0283](../../../../docs/adr/0283-a-region-declares-when-its-picture-next-changes-not-that-something-is-pending.md)).
    ///
    /// **Nothing about the beat changes**, and P-0094 is why it must not: the
    /// light travels the grid on every frame the session advances, so its two
    /// numbers are one number and this goes on answering [`BEAT_STALENESS`]
    /// for as long as the row is drawn.
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
    /// # `None` is the panel saying it is still, and that is now a narrower
    /// state than *nothing pending*
    ///
    /// `None` is not an absence of information: it is the panel saying it is
    /// still, and the window then sleeps. **A console with no parked slot and
    /// no scheduled move on a fader costs exactly what it cost before either
    /// existed**, which is a claim `tests/parked.rs` and `tests/armed.rs` make
    /// rather than a hope — and both of them ask it of a console with no
    /// engine behind it, which is what every test in this crate is.
    ///
    /// **With an engine behind it and the transport row on screen this never
    /// answers `None`**, because the beat is moving and says so
    /// ([`View::transport_declares`],
    /// [P-0094](../../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)).
    /// That is ADR-0164's still-panel clause narrowing rather than failing: a panel
    /// with something moving on it is a panel with something changing on it,
    /// and the reason it is moving is a declaration rather than an accident.
    pub fn animating(&self, layout: &karakuri_layout::Layout) -> Option<Duration> {
        self.declares(layout).map(|live| live.moves_in).min()
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
        let program = program_bay(panel.layout(), self.canvas);
        let cells = program.and_then(|bay| bay.cells);
        // **Where a Set in hand would land, resolved once for the frame off
        // the derivations the release asks.** `None` for every other drag and
        // for no drag at all, which is what makes this the *carry's* mark
        // rather than a hover: `Panel::in_hand` is the question *is a gesture
        // in progress*, and `Panel::cursor` is where the pointer is whoever
        // claimed the event.
        //
        // **At most one of the two answers**, and that is where the pointer is
        // rather than a rule either bay keeps: the mixer's strips and the
        // Program bay's cells are rectangles in two regions of the panel, so a
        // point inside one set is outside the other. Over an alley between two
        // strips, over a bay head, over the transition row or over another bay
        // both answer `None` and nothing at all is marked — a mark that
        // snapped to the nearest strip would name a deck nobody pointed at,
        // and the release that followed it would load one.
        //
        // **The strip half is resolved inside the bay's own arm below**, off
        // the one `mixer` call this frame makes: a second derivation would be
        // a second answer, and the mark has to be round the strip the release
        // will name. Only the cell half is answered here, where the bay it
        // reads is already derived.
        //
        // **The slot count goes in with the point**, and it is the same
        // reading [`View::select`] refuses a key on: the row is [`DECKS`]
        // cells whatever the deck holds, so a cell whose letter names no slot
        // is a rectangle with nothing to load into
        // ([`ProgramBay::dropped`]).
        let carried = matches!(panel.in_hand(), Some(InHand::Carrying)).then(|| panel.cursor());
        let marked_cell =
            carried.and_then(|at| program.and_then(|bay| bay.dropped(at, self.mixer.len())));
        let picture = self.picture;
        let previews = self.previews;
        // **Beside the pictures, and read once for the frame with them**: the
        // word a caption draws is a function of the pair, so a frame that read
        // one of them twice could draw a mark against the other's answer.
        let overloaded = self.overloaded;
        // **Beside the pictures, and read once for the frame for their
        // reason.** What each cell costs and what each cell is showing are two
        // fields because they arrive from two places — see [`View::costs`].
        let costs = self.costs;
        let values = self.transport;
        let arr = &self.arrangement;
        // **Read once for the frame beside the arrangement**, and for the same
        // reason: the arrangement pill is laid out from where this one ends,
        // so a frame that asked twice could lay the two out from two answers.
        let audio = self.audio.as_ref();
        // **And the tracker's own three, read once beside it.** The arrangement
        // pill is laid out from where the octave's second half ends, so a frame
        // that asked twice could lay the row out from two answers.
        let tracking = self.tracker;
        // **And the map, read once beside the two above and for their reason**:
        // `learn`, `map` and the arrangement pill are laid out one from the
        // next, so a frame that asked twice could lay three controls out from
        // three answers.
        let map = self.map.as_ref();
        let armed = self.learn;
        let look_at = self.look;
        let out = self.master_out;
        let chain = self.master_chain;
        let strips = self.mixer.as_slice();
        let sets = self.library.as_slice();
        // **And what each of those rows is**, read beside the names for their
        // reason: the two halves are one listing (`Rows`), and a badge drawn
        // from a second read could describe a row that had been rewritten
        // under it (ADR-0338).
        let kinds = self.kinds.as_slice();
        // **Which of them are starred, read once for the frame beside the
        // listing it points into** — `draw` takes `&mut self`, and the arm
        // below borrows both.
        let starred = &self.starred;
        let scopes = self.scopes.as_slice();
        // **The fifth pointer, read once for the frame** beside the two slices
        // it borrows from — `draw` takes `&mut self`, and a filter read inside
        // the arm below would be a second borrow of `holds`.
        let narrowed = self.filters();
        // **And where this library is pointed, read once beside it** — the
        // `.path` row is a rectangle in the bay as well as a line of type, so
        // the derivation and the paint are asked one value, and `draw` takes
        // `&mut self` where this borrows two fields.
        let pointed = self.pointed();
        // **The sixth, read here for the two above's reason**: it borrows the
        // listing this frame is drawing, and `draw` takes `&mut self`. It is
        // the cursor and the reading put together — see [`View::opened`].
        let opened = self.opened();
        // **The two pointers, read once for the frame** beside the readings
        // they are drawn against — `draw` takes `&mut self` and the arms below
        // borrow these slices, so a pointer read inside an arm would be a
        // second borrow of the thing it points into.
        let selection = self.selection();
        let cursor_row = self.cursor_row();
        // **Where the dashed ring goes, asked once for the frame** beside the
        // three pointers it is now the same field as — [`View::focus_mark`],
        // which is the derivation this paints from rather than a second
        // reading of where focus is.
        let focused = self.focus_mark(panel);
        // **And the mark a folded bay wears**, read here for the reason above
        // it: it is the same pointer asked a second question, and a folded bay
        // has no rectangle for the ring alone to sit on.
        let folded = self.folded_mark(panel);
        // **The third of them**, and it is read the same way and for the same
        // reason: which chip is marked is a position in the row this frame is
        // drawing, and a scope past its end is the last chip there is.
        let scope = self.marked();
        // **The fourth, and it is read here for the same reason** — the
        // transition row is laid out from it and painted from it, and `draw`
        // takes `&mut self` while the arms below borrow the slices beside it.
        let transition_at = self.transition;
        // **What the foot's load control is aimed at, read once for the
        // frame** beside the pointers above it and for their reason: `draw`
        // takes `&mut self` and the arm below borrows the slices this reads
        // its deck count off. **It is not `selection`** — the pulldown is this
        // bay's own mark and the two are free to name two different decks,
        // which is `View::target` the field.
        let load = self.target();
        // **And the row menu's reading, off the same borrow and for the same
        // reason** — it counts the strips too, and the arm below borrows the
        // slices it would be read off.
        // **Which `uses` line has its card down**, read once for the frame like
        // every other pointer this console keeps — see [`View::wiring_open`].
        let wiring = self.wiring_open;
        // **And which pane head's pulldown is down, read the same way** — see
        // [`View::pane_open`]. The count of decks it offers is the strips',
        // which `load` above has already read off the same slice.
        let pane_open = self.pane_open;
        let menued = self.menued();
        // **And how far the Library bay is scrolled**, read once for the frame
        // beside the two pointers above it: `library` clamps it and hands the
        // clamped value back, and this is the stored one going in
        // (`LibraryBay::scroll`, P-0082).
        let scrolled_to = self.library_scroll();
        let waiting = self.staging.as_slice();
        // **What the Sequencer bay reads, taken once for the pass** beside the
        // strips it sits under: `draw` takes `&mut self` and the loop below
        // borrows the fields a reading would be read off.
        let sequenced = self.sequencer.as_ref();
        // **And what its `+ lane` chooser offers**, taken here for the same
        // reason and read off three of this console's own values — the strips,
        // the load pulldown's deck and that deck's published rows.
        let choices = self.lane_choices();
        let panes = self.inspector.as_slice();
        // **The eighth pointer, read once for the frame beside the panes it is
        // about** — how far each of them is scrolled. It is `Copy` and two
        // `f32`s wide, and it is read here for the reason the seventh below is:
        // `draw` takes `&mut self` and the loop already borrows `inspector`.
        let scrolled = self.scroll;
        // **The seventh pointer, read once for the frame** beside the panes it
        // points into — `draw` takes `&mut self`, and a head asked inside the
        // loop below would be a second borrow of the same struct.
        let naming = self.naming.as_ref();
        let phase = self.phase;
        // **The opening, read once for the pass.** Every head that opens a
        // class lays its pills out against this one value, so no two capsules
        // on a frame can be placed against two different states.
        let opening = self.opening;
        // **What this crate cannot see** — see `Outputs::told`. Copied out
        // beside `opening` for the same reason: the loop below borrows the
        // arrangement and the palette, and one `bool` read here is one place
        // the answer comes from.
        let projector = self.projector;
        let frame = egui::Frame::NONE.fill(pal.ground);
        egui::CentralPanel::default().frame(frame).show(ui, |ui| {
            for placed in &self.placed {
                let rect = to_egui(placed.rect);
                match placed.region.kind {
                    Kind::Bay { .. } => {
                        card(ui, &pal, rect);
                        head_into(ui, &pal, rect, placed.region, opening);
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
                        // **The look's two controls, inside the row they
                        // belong to.** Unlike the arrangement pill this has
                        // nothing that hangs out of the row, so it is painted
                        // here rather than after the loop — and the pill's
                        // menu, which does hang out, is painted after and
                        // therefore over it, which is the order a card that is
                        // above the bays wants. Where it goes is `look`'s
                        // answer and not this pass's: the same call
                        // `input::claim` makes.
                        // **The tracker's other three, inside the row they
                        // belong to**, and painted before the look for the one
                        // reason that matters: the look is laid out from the
                        // arrangement pill, which is laid out from these, so
                        // this is the first of the three answers rather than a
                        // second one. Where it goes is `tracker_group`'s — the
                        // same call `input::claim` makes.
                        if let Some(group) =
                            tracker_group(ui.ctx(), panel.layout(), values, audio, tracking)
                        {
                            tracker_into(ui, &pal, &group);
                        }
                        if let Some(row) = look(
                            ui.ctx(),
                            panel.layout(),
                            values,
                            audio,
                            tracking,
                            None,
                            arr,
                            look_at,
                        ) {
                            look_into(ui, &pal, &row);
                        }
                    }
                    // The one row with something in it, and the something is
                    // one control. Where it goes is `outputs`'s answer and
                    // not this pass's: the same call `input::claim` makes, so
                    // the chip that is painted is the chip that is clicked.
                    Kind::Outputs => {
                        card(ui, &pal, rect);
                        if let Some(row) =
                            outputs(ui.ctx(), panel.layout(), opening).map(|r| r.told(projector))
                        {
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
                        head_into(ui, &pal, rect, placed.region, opening);
                        if let Some(bay) = mixer(ui.ctx(), panel.layout(), strips) {
                            // **Which strip a Set in hand would land on, off
                            // this bay and not a second one.** `Mixer::dropped`
                            // is the derivation the release asks, so the ring
                            // is painted round the strip that release names.
                            let marked = carried.and_then(|at| bay.dropped(at));
                            mixer::mixer_into(ui, &pal, &bay, phase, selection, marked);
                        }
                        // **The transition row, under the strips**, and it is
                        // painted from here rather than from inside
                        // `mixer_into` because it is not a strip's: the three
                        // settings are the console's own and are drawn with or
                        // without a deck, where `mixer` answers `None` for a
                        // console that has none. Where the row goes is
                        // `transition`'s answer and not this pass's: the same
                        // call `input::claim` makes, so the pill that is
                        // painted is the pill that is clicked. It is the
                        // arrangement `look_into` already has beside
                        // `transport_into`, one row down.
                        //
                        // **The strip count goes with it**, and it is the one
                        // thing on this row that is a reading: whether the
                        // `go` capsule is lit is *whether a wipe has anywhere
                        // to come from*, which is the mixer's own length and
                        // the same count `TransitionRow::go` is asked with.
                        if let Some(row) = transition(ui.ctx(), panel.layout(), transition_at) {
                            mixer::transition_into(ui, &pal, &row, strips.len());
                        }
                    }
                    // **The third bay with something in its body, and it is
                    // one row of the three the mock draws there.** The card
                    // and the head are every other bay's; under the row the
                    // card shows through, because the chain the mock draws is
                    // three effects that exist nowhere in this workspace and a
                    // control over machinery that is not there is the
                    // scaffolding this module refuses. Where the row goes is
                    // `master`'s answer and not this pass's: the same call
                    // `input::claim` makes, so the knob that is painted is the
                    // knob a hand takes hold of.
                    // **The fifth bay with something in its body**, and the
                    // first one whose body reads a value no engine holds: a
                    // pattern is authored state a session keeps, handed in
                    // per frame like every other reading here. The card and
                    // the head are every other bay's; with no pattern behind
                    // the console there are no rows and the body is as empty
                    // as every other one in this pass — `sequencer`'s answer,
                    // not this pass's. Where the cells go is that function's
                    // too: the same call `input::claim` makes, so the cell
                    // that is painted is the cell a press lands on.
                    Kind::Sequencer => {
                        card(ui, &pal, rect);
                        // **The one head on this console that is handed a
                        // value**: the bank pills say which of the four the
                        // rows are reading, and that is the armed bank rather
                        // than anything the region table could carry
                        // ([`Head::banks`]). With no pattern behind the console
                        // there is no armed bank either, and the head is every
                        // other bay's.
                        match sequenced.map(|reading| reading.bank) {
                            Some(armed) => {
                                if let Some(head) = head_of(placed.region) {
                                    bay_head(ui, &pal, rect, &head.with_banks(armed), opening);
                                }
                            }
                            None => head_into(ui, &pal, rect, placed.region, opening),
                        }
                        if let Some(bay) = sequencer(ui.ctx(), panel.layout(), sequenced, &choices)
                        {
                            sequencer_into(ui, &pal, &bay);
                        }
                    }
                    Kind::Master => {
                        card(ui, &pal, rect);
                        head_into(ui, &pal, rect, placed.region, opening);
                        if let Some(row) = master(ui.ctx(), panel.layout(), out, chain) {
                            master_into(ui, &pal, &row);
                        }
                    }
                    // The other bay with something in its body, and it is a
                    // bay in every other respect: the same card and the same
                    // head, then the row of chips saying which library is
                    // being read, and then as many rows of that library as the
                    // bay has room for. Told nothing about any library there
                    // is neither, and the body is as empty as every other one
                    // in this pass — `library`'s answer, not this pass's, so
                    // that *nothing said means nothing at all* is decided in
                    // one place.
                    Kind::Library => {
                        card(ui, &pal, rect);
                        head_into(ui, &pal, rect, placed.region, opening);
                        if let Some(bay) =
                            library(panel.layout(), scopes, sets, opened, pointed, scrolled_to)
                        {
                            // **The chips before the rows**, and painted from
                            // the draw rather than from inside the listing's
                            // own paint: the scope row belongs to the bay's
                            // head — it says which library is being read,
                            // where everything under it is what that library
                            // holds.
                            library::scopes_into(ui, &pal, &bay, scopes, scope);
                            // **And the path row between them and the fields**,
                            // for the scope row's reason and one more of its
                            // own: it says which directory this library is
                            // pointed at, where the chips say which library —
                            // and it is drawn whichever chip is marked,
                            // because it is also where a send's save dialog
                            // opens (ADR-0311).
                            if let Some(at) = pointed {
                                library::path_into(ui, &pal, &bay, at);
                            }
                            // **And the filter row between them and the
                            // rows**, for the scope row's reason: it belongs
                            // to the bay's head — it narrows what the library
                            // being read answers, where everything under it is
                            // what came back.
                            library::filters_into(ui, &pal, &bay, narrowed);
                            // **And the kind row under the field**, which is
                            // the filter row's own sentence one band down: it
                            // narrows what the library answers by what a row
                            // *is*, where the field above narrows by what a
                            // Set is made of (ADR-0338).
                            library::kinds_into(ui, &pal, &bay, narrowed);
                            library::library_into(
                                ui,
                                &pal,
                                &bay,
                                library::Listed {
                                    rows: Rows { names: sets, kinds },
                                    starred,
                                },
                                cursor_row,
                                load,
                                opened,
                            );
                        }
                    }
                    // **The fourth bay with something in its body**, and it is
                    // a bay in every other respect: the same card and the same
                    // head, and then a row per node a build changed and whose verdict is
                    // outstanding. With no engine behind the console there are
                    // none and the body is as empty as every other one in this
                    // pass — `staging`'s answer, not this pass's, so that
                    // *nothing waiting means nothing at all* is decided in one
                    // place. The head takes no pill: the mock's `2 waiting` is
                    // a readout, and a bay head's pills are its controls.
                    Kind::Staging => {
                        card(ui, &pal, rect);
                        head_into(ui, &pal, rect, placed.region, opening);
                        if let Some(bay) = staging(panel.layout(), waiting) {
                            staging_into(ui, &pal, &bay, waiting);
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
                    // **The drop mark, and it is not a reading of the cell.**
                    // What is behind the rectangle is not read — a cell
                    // drawing material, a still, or nothing at all is a target
                    // exactly the same — so this is `marked_cell`'s answer and
                    // not `previews[deck]`'s. Painted before the image so the
                    // hairline round the well goes on over it, exactly as the
                    // ring round a strip goes under the selection's.
                    let marked = marked_cell == Some(deck as u8);
                    if marked {
                        drop_ring(ui, &pal, cell, size::PREVIEW_RADIUS);
                    }
                    preview(ui, &pal, cell, previews[deck]);
                    caption_into(
                        ui,
                        &pal,
                        cell,
                        deck,
                        previews[deck],
                        overloaded[deck],
                        costs[deck],
                        marked,
                    );
                }
            }

            // **The Inspector's panes, after the loop for the same reason.**
            // `Kind::Pane` is a unit variant and stays one — `tests/view.rs`
            // asserts that both panes are one — so it cannot carry *which*
            // pane, and comparing `inspector-1` against a name on the frame
            // path is the string in the loop this table exists to avoid. They
            // sit inside the Inspector bay's card, which the loop has already
            // painted, and nothing else on the panel overlaps them.
            for (index, pane) in panes.iter().enumerate().take(PANES) {
                if let Some(at) = inspector(panel.layout(), index, pane, scrolled[index]) {
                    let typed = naming
                        .filter(|naming| naming.pane == index)
                        .map(Naming::typed);
                    inspector_into(
                        ui,
                        &pal,
                        &at,
                        pane,
                        on_air(strips, pane.deck),
                        typed,
                        // **The same derivation `claim` hit-tests**, asked
                        // here rather than inside the paint because the rows
                        // it offers are the mixer's, and this loop already
                        // borrows what a reading of them would come off.
                        pane_target(
                            ui.ctx(),
                            &at,
                            pane,
                            index,
                            typed,
                            load.decks,
                            pane_open == Some(index),
                        ),
                    );
                }
            }

            // **The arrangement pill, and its menu over everything.** Painted
            // after the loop for a reason the two above do not have: the menu
            // hangs *out* of the row it belongs to and down over the bays, so
            // a card painted from the `Kind::Transport` arm would go on before
            // the Library bay and end up under it. The pill itself is inside
            // its row and would not care; the two halves of one control are
            // one call, and where it goes is `arrangement`'s answer — the same
            // call `input::claim` makes, so the pill that is painted is the
            // pill that is clicked.
            // **The audio-in pill, and its card over everything**, for the
            // arrangement pill's reason one item to the left: the card hangs
            // out of the row and down over the bays. It is painted before the
            // arrangement pill because it is drawn before it in the row; the
            // two cards can never both be down, since `input`'s rule 2 sends
            // the press that would open the second one to whichever is already
            // open, and that press shuts it.
            if let Some(audio) = audio {
                if let Some(pill) = audio_in(ui.ctx(), panel.layout(), values, Some(audio)) {
                    audio_in_into(ui, &pal, &pill, audio);
                }
            }
            // **`learn` and `map`, in the mock's order and before the
            // arrangement pill**, which is where they sit in `.transport`:
            // `learn`, `map · nanoKONTROL2 ▾`, `arr · night ▾`. Neither has a
            // card to hang out of the row, so either could have been painted
            // from the `Kind::Transport` arm; they are here so that the three
            // controls that are laid out one from the next are painted in one
            // place, in the order they are laid out.
            if let Some(pill) = learn_pill(
                ui.ctx(),
                panel.layout(),
                values,
                audio,
                tracking,
                map,
                armed,
            ) {
                learn_into(ui, &pal, &pill);
            }
            if let Some((pill, map)) =
                map_pill(ui.ctx(), panel.layout(), values, audio, tracking, map).zip(map)
            {
                map_into(ui, &pal, &pill, map);
            }
            if let Some(pill) =
                arrangement(ui.ctx(), panel.layout(), values, audio, tracking, map, arr)
            {
                arrangement_into(ui, &pal, &pill, arr);
            }
            // **A `uses` line's card, over everything for the same reason** —
            // it hangs out of a line inside a pane and down over the groups
            // under it, so a card painted from inside the pane loop would go on
            // before them. It can never be down while any of the others is,
            // because `input`'s rule 2 sends the press that would open a second
            // one to whichever is already open.
            if let Some((pane_at, node, input)) = wiring {
                if let Some(pane) = panes.get(pane_at) {
                    if let Some(at) = inspector(panel.layout(), pane_at, pane, scrolled[pane_at]) {
                        if let Some(line) = at.uses_line(ui.ctx(), pane, node, input, true) {
                            let room = to_egui(panel.layout().viewport());
                            if let (Some(card), Some(uses)) = (
                                line.list(room),
                                pane.nodes.get(node).and_then(|at| at.uses.get(input)),
                            ) {
                                uses_card_into(ui, &pal, &line, uses, card, room);
                            }
                        }
                    }
                }
            }
            // **A pane head's deck list, over everything for the same reason**
            // — it hangs out of a head at the top of a pane and down over that
            // pane's own groups, so a card painted from inside the pane loop
            // would go on before them. It can never be down while any of the
            // others is, because `input`'s rule 2 sends the press that would
            // open a second one to whichever is already open.
            if let Some(pane_at) = pane_open {
                if let Some(pane) = panes.get(pane_at) {
                    if let Some(at) = inspector(panel.layout(), pane_at, pane, scrolled[pane_at]) {
                        let typed = naming
                            .filter(|naming| naming.pane == pane_at)
                            .map(Naming::typed);
                        let room = to_egui(panel.layout().viewport());
                        if let Some(target) =
                            pane_target(ui.ctx(), &at, pane, pane_at, typed, load.decks, true)
                        {
                            if let Some(card) = target.list(room) {
                                pane_list_into(ui, &pal, &target, pane.deck, card);
                            }
                        }
                    }
                }
            }
            // **The Library bay's deck list, over everything for the two
            // cards' reason** — it hangs out of that bay's foot and up over
            // its own rows, so a card painted from inside the arm would go on
            // before them. It can never be down while either of those is,
            // because `input`'s rule 2 sends the press that would open a
            // second one to whichever is already open.
            if load.open {
                if let Some(bay) =
                    library(panel.layout(), scopes, sets, opened, pointed, scrolled_to)
                {
                    let at = bay.load(ui.ctx(), load);
                    if let Some(card) = at.list(to_egui(panel.layout().viewport())) {
                        library::deck_list_into(ui, &pal, &at, load, card);
                    }
                }
            }
            // **And a row's menu, last of the four cards** — it hangs out of a
            // row of that bay's list and down over the rows under it, so a
            // card painted from inside the arm would go on before them. It can
            // never be down while any of the other three is, for their reason.
            if menued.row.is_some() {
                if let Some(bay) =
                    library(panel.layout(), scopes, sets, opened, pointed, scrolled_to)
                {
                    if let Some(menu) =
                        bay.menu(ui.ctx(), to_egui(panel.layout().viewport()), menued)
                    {
                        library::row_menu_into(ui, &pal, &menu);
                    }
                }
            }
            // **And the Sequencer's `+ lane` chooser, a fifth card here for
            // the four above it's reason** — it hangs out of that bay's foot
            // and up over its own rows, so a card painted from inside the arm
            // would go on before them and under whatever is drawn after that
            // bay. It can never be down while any of the other four is, for
            // their reason.
            if choices.open {
                if let Some(bay) = sequencer(ui.ctx(), panel.layout(), sequenced, &choices) {
                    if let Some(card) = bay.card {
                        lane_card_into(ui, &pal, &card, &choices);
                    }
                }
            }
            // **The dashed focus ring, painted last** — after every card and
            // every menu, for the five cards above it's reason read the other
            // way: this mark is *proud* of the head it rings
            // ([`size::WFOCUS_OFFSET`]), so it lands on the ground between two
            // bays, and anything drawn after it would go over it. **One ring
            // on one bay**, because focus is in one place — see
            // [`View::focus_mark`], which is the derivation and where the
            // folded case is argued.
            if let Some((mark, title)) = folded {
                folded_head_into(ui, &pal, mark, title);
                wfocus_into(ui, &pal, mark);
            }
            if let Some(mark) = focused {
                wfocus_into(ui, &pal, mark);
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
        // There is no cursor of its own for a fader: a knob under the hand is
        // already drawn where the hand is, and a value moving is the whole of
        // what a fader drag says. So it suppresses the resize and asks for
        // nothing in its place.
        //
        // **A carry suppresses it for the same reason and does not stop
        // there.** A Set on its way to a deck crosses every boundary between
        // the Library bay and the mixer, so falling through to the hit test
        // would flick a resize cursor on over each of them — a cursor for a
        // gesture that is not happening. What goes on instead is
        // `CursorIcon::Grabbing`, and `console.html` is what asks for it:
        // *"the pointer itself is a grab for as long as the Set is in hand …
        // it is the one thing that says a gesture is still running while the
        // hand is over nothing at all"*.
        //
        // **This comment argued against that shape and the argument was
        // wrong**, so it is rewritten rather than deleted: it read that a
        // grab *"would be a third mark in a vocabulary of two, said by one
        // control on the panel"*. Two things are wrong with it. A carry is
        // not said by a control — the Set leaves the Library bay's list and is
        // over no control at all for most of the gesture, which is the one
        // state on this panel that nothing drawn can report. And a carry is
        // the only drag here that can be **cancelled**: a boundary and a fader
        // come to rest wherever the pointer left them, so a cursor for either
        // would be decoration, where this one is the difference between a
        // gesture still running and a press that was missed. The page is
        // where that was settled
        // ([ADR-0273](../../../../docs/adr/0273-the-carry-lands-on-two-sets-of-rectangles-and-wears-a-face.md));
        // ADR-0265's consequence naming the old shape is annotated there.
        //
        // **Still one writer and still one icon per frame.** The grab is set
        // here rather than beside the drop mark for the reason the whole
        // function exists: `egui_winit` writes the window's cursor out of
        // `PlatformOutput` once, and a second setter is a flicker that depends
        // on event order.
        let axis = match panel.in_hand() {
            Some(InHand::Boundary(axis)) => Some(axis),
            Some(InHand::Carrying) => {
                ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
                return;
            }
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

/// **One deck preview cell's image**: the mock's `.preview`, and whatever is
/// behind it.
///
/// **Nothing is written on it.** The letter and the state word are the
/// caption's, under the image — [`caption_into`] — and the reason is the one
/// the mock states beside the rule: a letter laid over the material is
/// unreadable exactly when the deck is live and the material is bright, which
/// is the one moment the row is read fastest. The image carries material or it
/// carries the bare well, and either way it carries nothing this crate wrote.
///
/// The cell is painted whether or not a texture was handed in, because *no
/// slot behind it* is a state rather than a thing not built yet — the module
/// documentation is where that argument is written out.
///
/// **A cell is not dark because its deck is off air.** Every slot is drawn
/// into its own target on every frame, so a parked or warming deck has a
/// picture here exactly as a Live one does; what the residency decides is
/// whether that slot reaches the *mix*, which is the picture above and the
/// mixer strip beside.
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
fn preview(ui: &Ui, pal: &Palette, cell: Rect, picture: Option<Picture>) {
    let radius = CornerRadius::same(size::PREVIEW_RADIUS as u8);
    // Clipped to the cell for the reason the picture is clipped to its region:
    // the rectangle in `picture` came from outside, and a stale one is a
    // thumbnail painted across the bay rather than a wrong thumbnail.
    let painter = ui.painter().with_clip_rect(cell);
    painter.rect_filled(cell, radius, pal.well);

    if let Some(picture) = picture {
        painter.image(picture.id, picture.rect, WHOLE_TEXTURE, Color32::WHITE);
    }
    // **One stroke, `pal.hair`, on every cell.** A cell used to wear a
    // `--c-mint` ring when the output was auditioning the deck it is drawn
    // for, and the ring went with the operation: ADR-0240 makes the picture
    // the master mix always and every cell its own deck's monitor always, so
    // there is no longer a fifth fact for a mark to carry and no cell that is
    // more *on* than the other three. That is the mock's own state restored —
    // `style.css` gives `.preview` one `inset 0 0 0 1px var(--c-hair)` and no
    // second colour.
    painter.rect_stroke(
        cell,
        radius,
        Stroke::new(size::HAIRLINE, pal.hair),
        StrokeKind::Inside,
    );
}

/// **The word a cell's caption gives for what the cell is showing.**
///
/// Three, because three is what the pair of values this reads distinguishes
/// and drawing a fourth would be drawing a state the program cannot be in.
///
/// - [`PREVIEW_MATERIAL`] — there is a deck slot behind this cell and the
///   image is that slot's own target. It says nothing about residency: a
///   parked deck's still and a live deck's frame are the same word, which is
///   [ADR-0258](../../../../docs/adr/0258-the-look-comes-before-the-fader-so-a-cell-draws-every-slot-and-says-which-nothing-it-is.md).
/// - [`PREVIEW_OVERLOADED`] — there is a slot behind this cell and it has
///   **stopped updating**: the version in it costs more than one frame may, so
///   the engine skips its step and its draw and the image is the last frame it
///   made (ADR-0316). **It outranks `material` and does not replace what the
///   cell shows**, which is the whole shape of the decision — the image stays,
///   because blanking it would be indistinguishable from an empty slot, and
///   the word is what separates a still from a preview
///   ([ADR-0269](../../../../docs/adr/0269-a-slot-that-is-drawn-is-stepped-and-a-preview-runs-at-the-rooms-tempo.md)).
///   It is not residency either: this cell reads the same word on air and off.
/// - [`PREVIEW_NO_SLOT`] — there is no slot behind this cell at all: a deck of
///   fewer slots than there are cells, or a console with no engine behind it.
///   **It wins over the mark**, because a mark about a slot that is not there
///   is about nothing: the flag crosses the seam per cell and a caller writing
///   one beside no picture is saying two things at once, of which this draws
///   the one that is about the cell.
///
/// **Not `empty` and not `off`, and both of those are worth naming.** The
/// mock's D cell said `D · off` when
/// [ADR-0170](../../../../docs/adr/0170-a-deck-preview-cell-is-drawn-whether-or-not-a-deck-is-behind-it.md)
/// landed and this function's ancestor copied the word; the page has since
/// moved and neither word is what the cell distinguishes. *Off* was residency,
/// and residency has not gated a cell since ADR-0240. *Empty* is a slot that
/// exists with nothing loaded into it — the manual names it and the engine
/// cannot be in it, because `Deck::new` builds a slot per `HotSwap` and
/// `slot_view` is `None` only past `slot_count`. Writing either here would
/// assert a reading nobody took, which is ADR-0200's rule and ADR-0191's
/// before it.
fn state_word(picture: Option<Picture>, overloaded: bool) -> &'static str {
    match (picture, overloaded) {
        (None, _) => PREVIEW_NO_SLOT,
        (Some(_), true) => PREVIEW_OVERLOADED,
        (Some(_), false) => PREVIEW_MATERIAL,
    }
}

/// A cell showing its slot's own material. See [`state_word`].
pub const PREVIEW_MATERIAL: &str = "material";

/// A cell showing the last frame a stopped slot drew — the word
/// `swap::Event::Overloaded` and [`Stage::Overloaded`] carry, said here because
/// this is where the still is. See [`state_word`] and [`View::overloaded`].
pub const PREVIEW_OVERLOADED: &str = "overloaded";

/// A cell with no deck slot behind it. See [`state_word`].
pub const PREVIEW_NO_SLOT: &str = "no slot";

// ---------------------------------------------------------------------------
// The risk badge: one number, five bands.
// ---------------------------------------------------------------------------

/// **What the governor budgeted one slot at, and which of its two numbers that
/// is** — the seam the risk badge is drawn from.
///
/// `karakuri_engine::governor::Decision` carries `budgeted_ms` and a `Basis`
/// saying whether it is the two-draw estimate at the output's size or the
/// single-draw measurement at the reference resolution
/// ([ADR-0296](../../../../docs/adr/0296-the-governor-budgets-on-the-estimate-where-it-answers-and-on-the-measurement-where-it-does-not.md)).
/// This is that pair, arriving the way every other value does: `src/` takes no
/// engine (ADR-0156), so whoever holds the deck reads the report and writes
/// [`View::costs`].
///
/// **The engine's third basis is this type's [`None`].**
/// `governor::Basis::Unbudgetable` is a slot nothing measured and nothing
/// estimated, and `Decision::budgeted_ms` is `None` there. It is not a number
/// and it is emphatically not a zero, so it does not cross this seam as one:
/// no entry, no dot. See [`View::costs`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Budgeted {
    /// The number the governor spent on this slot, in milliseconds —
    /// `Decision::budgeted_ms`. What [`band_of`] reads.
    pub ms: f32,
    /// **How it was taken.** See [`Basis`], and the argument on [`band_of`]
    /// for why the dot does not draw it.
    pub basis: Basis,
}

/// **Which of the governor's two numbers [`Budgeted::ms`] is** —
/// `karakuri_engine::governor::Basis`, less the variant this crate spells
/// [`None`].
///
/// Carried across the seam and **not drawn**, which is a decision rather than
/// an omission: see [`band_of`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Basis {
    /// The single draw at the engine's reference resolution, because nothing
    /// estimated this slot or the estimate refused. It is a real reading of
    /// this Set and it is a reading at a size that is the operator's only by
    /// coincidence.
    Measured,
    /// The two-draw fit, evaluated at the size this deck is actually drawing
    /// into.
    Estimated,
}

/// **The five bands the risk badge is drawn in**, and the words the mock's
/// `.risk.green`, `.risk.blue`, `.risk.yellow`, `.risk.red` and `.risk.purple`
/// name them by.
///
/// **The table is the console's and not the engine's.**
/// `karakuri_engine::estimate`'s own documentation says so, and
/// `crates/karakuri-engine/tests/governor.rs` quotes the boundaries rather
/// than sharing them for the same reason: the engine produces a number of
/// milliseconds and has no opinion about how many of a thing an operator can
/// mix. The scale is a reading of one frame's worth of budget shared four
/// ways, which is a fact about this panel's four cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Band {
    /// Up to [`BAND_BLUE_MS`]: four of these at 60 Hz.
    Green,
    /// Over [`BAND_BLUE_MS`]: not four at 60, and two at 60 is fine.
    Blue,
    /// [`BAND_YELLOW_MS`]: two at 60 Hz, four at 30 Hz.
    Yellow,
    /// [`BAND_RED_MS`]: one at 60 Hz, two at 30 Hz.
    Red,
    /// [`BAND_PURPLE_MS`] and over: one slot eats a whole frame.
    Purple,
}

impl Band {
    /// The word the mock's class names this band by — `.risk.green` and its
    /// four siblings. What `tests/transcribed_constants_cite_the_mock.rs`
    /// looks the boundary up in `docs/manual/console.html` by, and what
    /// `karakuri-engine`'s own band prediction spells.
    pub fn word(self) -> &'static str {
        match self {
            Band::Green => "green",
            Band::Blue => "blue",
            Band::Yellow => "yellow",
            Band::Red => "red",
            Band::Purple => "purple",
        }
    }

    /// The colour it is drawn in, from the room's own five — `--c-band-*`.
    /// See [`Palette`], where the argument for five properties of their own
    /// rather than `--c-mint` and `--c-pink` is written.
    pub fn colour(self, pal: &Palette) -> Color32 {
        match self {
            Band::Green => pal.band_green,
            Band::Blue => pal.band_blue,
            Band::Yellow => pal.band_yellow,
            Band::Red => pal.band_red,
            Band::Purple => pal.band_purple,
        }
    }

    /// The five, worst last. The order the scale is written in.
    pub const ALL: [Band; 5] = [
        Band::Green,
        Band::Blue,
        Band::Yellow,
        Band::Red,
        Band::Purple,
    ];
}

/// **Where green ends and blue begins**: `docs/manual/console.html`, *What a
/// deck preview cell shows, and when* — *"Green, up to 4 ms: four of these at
/// 60 Hz."*
///
/// Four slots share one frame and 16.7 ms at 60 Hz is about 4 ms each, which
/// is where the whole scale comes from: it answers *how many of these, and at
/// what rate* rather than handing an operator a number to divide in a dark
/// room.
///
/// **Each of these four is named for the band it lets you into rather than the
/// one it leaves**, because that is the rounding rule written into the name: a
/// value *on* a boundary rounds to the worse band, so 4.0 is blue and not
/// green. See [`band_of`].
pub const BAND_BLUE_MS: f32 = 4.0;

/// *"Yellow, about 8 ms: the boundary for two at 60 Hz, and four at 30 Hz."*
pub const BAND_YELLOW_MS: f32 = 8.0;

/// *"Red, about 12 ms: one at 60 Hz, two at 30 Hz."*
pub const BAND_RED_MS: f32 = 12.0;

/// *"Purple, over 16 ms: one slot eats a whole frame and the cell stops
/// drawing."*
///
/// **The last band is the one that is also a behaviour**, and the behaviour is
/// not this crate's: a slot over budget is stopped by the governor rather than
/// shown harder, and a stopped slot reaches the console as a cell with no
/// picture in it. So the console never has to decide to stop drawing — what it
/// draws is the band, and the stopping has already happened upstream.
pub const BAND_PURPLE_MS: f32 = 16.0;

/// **The band one number falls in**, and the whole of the console's half of
/// the risk badge.
///
/// The four boundaries are [`BAND_BLUE_MS`], [`BAND_YELLOW_MS`],
/// [`BAND_RED_MS`] and [`BAND_PURPLE_MS`], and **a value on a boundary rounds
/// to the worse band** — the mock says so in those words, and it is the reason
/// every comparison here is `>=` and the fall-through is green. The direction
/// is the same one the estimate rounds in: `karakuri_engine::estimate` rounds
/// toward refusing at every step, so a badge that rounded a boundary the
/// generous way would be the one place in the chain that reads a number
/// kindly.
///
/// # It does not read [`Basis`], and that is the decision rather than the
/// default
///
/// A dot drawn from an estimate at this deck's own size and a dot drawn from a
/// measurement at 1280x720 are **not the same statement** — P-0095 is exactly
/// that, and ADR-0296 §3 carries `FloorRead` and `Floored` on the decision so
/// the difference cannot be lost. The console keeps it ([`Budgeted::basis`])
/// and does not draw it, for two reasons.
///
/// - **The number is the one the governor spent.** A badge that drew a dot
///   only where the basis is [`Basis::Estimated`] would be drawing a different
///   quantity from the one the deck is being governed on: every Set swapped in
///   on a live run arrives unestimated (ADR-0296, *Consequences*), so the dot
///   would vanish at the moment an operator loaded something — and a slot the
///   governor parks on a measured number would be parked with no visible
///   cause, which is
///   [ADR-0191](../../../../docs/adr/0191-the-panels-parked-deck-is-parked-by-the-governor-or-it-is-a-drawing-of-one.md)'s
///   complaint. **A refusal is not a green dot; a number that was spent is not
///   a refusal.**
/// - **The mock does not distinguish them**, and the page moves first. `.risk`
///   has five classes and there is no sixth mark, no hollow ring and no second
///   word in the caption for *how this was taken*. Inventing one here would be
///   the console specifying itself. What the page would have to say is
///   reported rather than drawn — see the module documentation on
///   [`caption_into`].
///
/// **What P-0095 does get** is the one thing the page already specifies: no
/// number, no dot. `governor::Basis::Unbudgetable` does not cross the seam
/// (see [`Budgeted`]), and a cell handed nothing draws nothing — not hollow,
/// not grey, not green by default.
pub fn band_of(ms: f32) -> Band {
    match ms {
        _ if ms >= BAND_PURPLE_MS => Band::Purple,
        _ if ms >= BAND_RED_MS => Band::Red,
        _ if ms >= BAND_YELLOW_MS => Band::Yellow,
        _ if ms >= BAND_BLUE_MS => Band::Blue,
        _ => Band::Green,
    }
}

/// **One deck preview cell's caption**: the mock's `.caption`, under the
/// image and never on it.
///
/// Term for term from `style.css`:
///
/// - `.cell`'s `gap: 4px` and `.caption`'s `height: 13px` — [`caption_of`],
///   which is where the rectangle comes from.
/// - `.caption`'s `font-size: 9px` — [`size::PREVIEW_SIZE`], for both the
///   letter and the word.
/// - `.caption`'s `gap: 5px` — [`size::PREVIEW_CAPTION_GAP_X`], between the
///   two.
/// - `align-items: center` — both galleys centred on the caption's own middle,
///   which is what puts a 9px letter and a 9px word on one baseline in a 13px
///   strip.
///
/// # The two colours, and what each is standing for
///
/// - **The letter** is `.caption b`'s `var(--c-dim)`, `pal.dim` — *"a label
///   beside"* a value, which is what a letter welded to a cell is. It is one
///   colour in both states, because the letter is not a state: `A` is `A`
///   whether or not anything is behind it, and a letter that dimmed when the
///   slot went away would be the cell's state said twice.
/// - **The word** is `.caption`'s own `var(--c-faint)`, `pal.faint`.
///
/// # The third colour, and it is the drop mark rather than a state
///
/// `marked` is `.cell.drop .caption b`'s `color: var(--c-text)`: while a
/// carried Set would land on this cell, the letter comes up out of its dim
/// with the ring round the image. **It is not a fourth state of the cell** —
/// the two above are still the only two [`View::previews`] distinguishes, and
/// nothing here reads the picture for it. What the ring says is *where the
/// release lands* and what the letter says is *which deck that is*, which is
/// the one thing on a cell that already names the operand
/// ([`drop_ring`], and `console.html`'s *How a Set reaches a deck*).
///
/// # The badge, and the two ways it is not drawn
///
/// `.risk` is `width: 6px; height: 6px; border-radius: 999px; margin-left:
/// auto` — a 6px dot in the band's colour, pushed to the far end of the
/// caption so the four line up down the row and can be read as a column
/// without reading a word ([`size::PREVIEW_RISK`], and the stylesheet's own
/// comment says the reason). [`band_of`] is the table; [`Band::colour`] is the
/// five properties the room states for it.
///
/// **A slot with no number draws no dot**, which is the state the manual
/// already describes and is not a fifth thing this function invents: not
/// hollow, not grey, not green by default, each of which would assert a
/// reading nobody took —
/// [ADR-0200](../../../../docs/adr/0200-a-bays-first-pass-draws-the-values-that-exist-and-omits-the-rest.md),
/// draw the values that exist and omit the rest. It covers three cases and
/// they are three different nothings: the governor found neither number for
/// this slot (`governor::Basis::Unbudgetable`, which does not cross the seam —
/// [`Budgeted`]), nobody has governed this deck yet, and **a number that is
/// not finite**, which is a failed reading rather than a small one.
///
/// **And a cell with no slot behind it draws no dot either, whatever it was
/// handed.** The mock's D cell is the case and its tooltip is the rule: *"No
/// slot is no cost, so there is no dot."* A dot beside [`PREVIEW_NO_SLOT`]
/// would be a cost for a thing that is not there, so the picture gates the
/// badge — which also means the two halves of a cell can never disagree about
/// whether there is a slot.
///
/// **Purple is the one band that is also a behaviour, and the behaviour is not
/// this crate's.** *One slot eats a whole frame and the cell stops drawing* —
/// stopping it is the governor's, and a stopped slot arrives here as a cell
/// with no picture. So this function never decides to stop drawing; it draws
/// the band it was handed and the stopping has already happened upstream.
///
/// # What the page would have to say before the badge could say more
///
/// The dot is one mark for two kinds of number. A band read from an estimate
/// at this deck's own size and a band read from a measurement at the reference
/// resolution are not the same statement (P-0095, and ADR-0296 §3), and the
/// mock has five classes on `.risk` and nothing for *how this was taken*.
/// [`Budgeted::basis`] carries the difference across the seam and this draws
/// it nowhere; [`band_of`] argues why that is the honest reading today, and
/// **the page moves first** if it is to stop being. What it would have to
/// state is a second mark on the caption and what it means — a ring round the
/// dot for a measured number, say, against a filled dot for an estimated one —
/// because the operator-facing difference is that a measured band is about a
/// frame at 1280x720 and can therefore be a band too good on a larger output,
/// which is the one direction the whole estimate is built to round away from.
#[allow(clippy::too_many_arguments)]
fn caption_into(
    ui: &Ui,
    pal: &Palette,
    image: Rect,
    deck: usize,
    picture: Option<Picture>,
    // **Whether this slot has stopped updating** — [`View::overloaded`]. It
    // changes the word and nothing else: not the letter, not the badge, and
    // not the image above, which is the still it is about.
    overloaded: bool,
    cost: Option<Budgeted>,
    marked: bool,
) {
    let at = caption_of(image);
    let painter = ui.painter().with_clip_rect(at);
    let font = FontId::new(size::PREVIEW_SIZE, FontFamily::Proportional);

    let ink = match marked {
        true => pal.text,
        false => pal.dim,
    };
    let letter = painter.layout_no_wrap(DECK_LETTERS[deck].to_owned(), font.clone(), ink);
    let width = letter.size().x;
    painter.galley(
        Pos2::new(at.min.x, at.center().y - letter.size().y * 0.5),
        letter,
        ink,
    );

    let word = painter.layout_no_wrap(state_word(picture, overloaded).to_owned(), font, pal.faint);
    painter.galley(
        Pos2::new(
            at.min.x + width + size::PREVIEW_CAPTION_GAP_X,
            at.center().y - word.size().y * 0.5,
        ),
        word,
        pal.faint,
    );

    // **The badge, where there is a slot and a number for it.** Both halves
    // are gates rather than one: `picture` is whether there is a slot at all
    // and `cost` is whether anything budgeted it, and the mock's D cell is the
    // first of the two — *"No slot is no cost, so there is no dot."* A number
    // that is not finite is a failed reading and takes the same path as no
    // number; see this function's documentation for all three nothings.
    let dot = picture
        .and(cost)
        .filter(|budgeted| budgeted.ms.is_finite())
        .map(|budgeted| band_of(budgeted.ms));
    if let Some(band) = dot {
        // `margin-left: auto` — the far end of the caption, centred in its
        // height, and `border-radius: 999px` on a 6px box is a circle of half
        // that across.
        let radius = size::PREVIEW_RISK * 0.5;
        painter.circle_filled(
            Pos2::new(at.max.x - radius, at.center().y),
            radius,
            band.colour(pal),
        );
    }
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

    // **The class pill, painted exactly as a bay head's is** — the same
    // [`pill_at`] the other three go through, in a row that has no head to put
    // it in. See [`Outputs::mcp`].
    pill_into(ui, pal, row.mcp, mcp_word(row.open), row.open);

    // **The program view first, then the three beside it**, all four through
    // one closure: three states and one set of colours, so a chip cannot be
    // painted one way here and another way one line down.
    //
    // `.sink` is `--c-well` with `--c-dim` type; `.sink.on` is the mint tint,
    // `--c-text` and the dot's glow; and `.sink.absent` is the well with the
    // faintest type there is and no glow, which is the mock's own state for a
    // sink whose plugin is not loaded — dim enough to read as *not installed*
    // rather than as *switched off*.
    let chip_into = |chip: Rect, dot_at: Rect, name: &str, on: bool, present: bool| {
        let (ink, fill, dot) = match (present, on) {
            (false, _) => (pal.faint, pal.well, tint(pal.faint, 40)),
            (true, true) => (pal.text, tint(pal.mint, 14), pal.mint),
            (true, false) => (pal.dim, pal.well, pal.faint),
        };
        painter.rect_filled(chip, CornerRadius::same((size::SINK_H * 0.5) as u8), fill);
        if on && present {
            painter.add(
                egui::epaint::Shadow {
                    offset: [0, 0],
                    blur: 8,
                    spread: 0,
                    color: pal.glow,
                }
                .as_shape(dot_at, CornerRadius::same((size::SINK_DOT * 0.5) as u8)),
            );
        }
        painter.circle_filled(dot_at.center(), size::SINK_DOT * 0.5, dot);
        let galley = painter.layout_no_wrap(
            name.to_owned(),
            FontId::new(size::BASE, FontFamily::Proportional),
            ink,
        );
        painter.galley(
            Pos2::new(
                dot_at.max.x + size::SINK_GAP,
                chip.center().y - galley.size().y * 0.5,
            ),
            galley,
            ink,
        );
    };
    chip_into(row.sink, row.dot, PROGRAM_VIEW, row.on, true);
    for c in &row.more {
        chip_into(c.chip, c.dot, c.name, c.on, c.present);
    }
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
///
/// **The head arrives as a [`Head`] rather than as three arguments**, because
/// the four bays that are kinds of their own used to spell theirs out at this
/// call and there is now a second reader of every one of them: [`mcp_pill`]
/// lays the same head out again to find the class capsule in it. One table
/// (`head_of`), one derivation (`head_pills`), and the capsule an operator sees
/// is the capsule a press lands on.
pub fn bay_head(ui: &Ui, pal: &Palette, rect: Rect, head: &Head, open: Open) -> Rect {
    let box_of = head_box(rect);
    let painter = ui.painter().with_clip_rect(box_of);

    // `border-bottom: 1px solid var(--c-hair)`.
    let rule = box_of.max.y - size::HAIRLINE * 0.5;
    painter.line_segment(
        [Pos2::new(box_of.min.x, rule), Pos2::new(box_of.max.x, rule)],
        Stroke::new(size::HAIRLINE, pal.hair),
    );

    let mid = box_of.center().y;
    if head.grip {
        grip_dots(ui, pal, Pos2::new(box_of.max.x - size::HEAD_PAD_X, mid));
    }
    // **Painted where [`head_pills`] puts them rather than laid out again
    // here.** That is [`outputs`]'s arrangement one row down: the capsule that
    // is drawn and the capsule a press lands on are one rectangle, so the
    // Program bay's `solo` cannot come apart from the control
    // [`program_head`] hands to a caller -- nor its class pill from the one
    // [`mcp_pill`] hands over.
    let armed = head.words(open);
    let words = armed.as_slice();
    head_pills(ui.ctx(), rect, words, head.grip, |index, capsule| {
        pill_into(ui, pal, capsule, words[index], armed.armed(index));
    });

    // `text-transform: uppercase` plus `letter-spacing: 0.16em`, at
    // `--c-faint`. `egui`'s default proportional face has no bold, so
    // `font-weight: 700` is not honoured — see `room`'s documentation.
    let job = spaced(
        &head.title.to_uppercase(),
        size::HEAD_SIZE,
        pal.faint,
        size::HEAD_TRACKING,
    );
    let galley = painter.layout_job(job);
    painter.galley(
        Pos2::new(box_of.min.x + size::HEAD_PAD_X, mid - galley.size().y * 0.5),
        galley,
        pal.faint,
    );

    box_of
}

/// **A region's head, painted** — the one call all five of the console's
/// headed bays make.
///
/// It was five calls to [`bay_head`] with the title, the pills and the grip
/// written out at each: one copy while the paint was the only reader of them,
/// and four copies the moment [`mcp_pill`] had to lay the same head out again
/// to find a class capsule in it. [`head_of`] is the table now and this is the
/// one caller that turns it into paint.
///
/// A no-op for a region with no head, which is the transport, the Outputs row,
/// a pane, the picture and the preview row.
fn head_into(ui: &Ui, pal: &Palette, rect: Rect, region: &Region, open: Open) {
    if let Some(head) = head_of(region) {
        bay_head(ui, pal, rect, &head, open);
    }
}

/// **The box a bay head is painted into**: the top [`size::HEAD_H`] of the
/// bay, clipped to the bay itself so that a bay shorter than its own head has
/// a shorter head rather than one drawn over whatever is below it.
///
/// Two readers, which is why it is a function: [`bay_head`] paints into it and
/// [`head_pills`] lays the head's controls out in it. The mid-line a pill is
/// centred on is this box's, so a head clipped short takes its pills up with
/// it and the two cannot disagree about where the row is.
pub(crate) fn head_box(rect: Rect) -> Rect {
    Rect::from_min_max(
        rect.min,
        Pos2::new(rect.max.x, (rect.min.y + size::HEAD_H).min(rect.max.y)),
    )
}

/// The width of a `.pill` holding `text`: the run at [`size::BASE`], plus
/// `.pill`'s `padding: 0 8px` either side.
///
/// `ctx` rather than a `Ui`, because [`program_head`] is a derivation and has
/// no painter -- the same reason [`mixer`] and [`outputs`] measure their words
/// off the context.
fn pill_width(ctx: &egui::Context, text: &str) -> f32 {
    let run = ctx.fonts_mut(|f| {
        f.layout_no_wrap(
            text.to_owned(),
            FontId::new(size::BASE, FontFamily::Proportional),
            Color32::PLACEHOLDER,
        )
        .size()
        .x
    });
    run + size::PILL_PAD_X * 2.0
}

/// **Where a bay head's pills go**, right to left from the right edge of the
/// head: the grip first where the mock draws one, then the pills in reverse,
/// one [`size::PILL_GAP`] apart. `place` is called once per pill, with its
/// index in `pills` and the capsule it occupies.
///
/// **One derivation for a painted pill and a pressed one.** [`bay_head`]
/// paints from it and [`program_head`] hit-tests from it, which is the
/// arrangement every other control on this console already has
/// ([`crate::input`]): the derivation that draws a control is asked a second
/// time rather than copied, so the capsule an operator sees and the capsule a
/// press lands on cannot come apart.
fn head_pills(
    ctx: &egui::Context,
    rect: Rect,
    pills: &[&str],
    grip: bool,
    mut place: impl FnMut(usize, Rect),
) {
    let head = head_box(rect);
    let mid = head.center().y;
    let mut right = head.max.x - size::HEAD_PAD_X;
    if grip {
        right -= GRIP_W + size::PILL_GAP;
    }
    for (index, pill) in pills.iter().enumerate().rev() {
        let w = pill_width(ctx, pill);
        place(
            index,
            Rect::from_min_size(
                Pos2::new(right - w, mid - size::PILL_H * 0.5),
                egui::vec2(w, size::PILL_H),
            ),
        );
        right -= w + size::PILL_GAP;
    }
}

/// **One capsule in either of the mock's two treatments**: the ordinary
/// `.pill`, or `.pill.armed` where what it names is live.
///
/// The only caller that ever asks for the second is a class pill that is open,
/// and it is the mock's own class rather than an invention here -- the audio-in
/// pill already carries it, and its documentation is where the argument was
/// first written: *"a pill that was only lit would leave which room is being
/// heard unanswered, and a pill that only carried a name would make a dead
/// input and a live one look alike at the distance a panel is read from."* An
/// opening is the same pair of questions -- *which class* and *is it open* --
/// so it gets the same pair of answers.
///
/// **`docs/manual/console.html` draws the shut state only, because every class
/// starts shut, and specifies the open one in words**: *what a model is
/// refused, and where a class opens* says the pill reads `mcp · open` and is
/// drawn armed, and makes that argument in its own terms. Both the word and the
/// treatment are the page's.
fn pill_into(ui: &Ui, pal: &Palette, rect: Rect, text: &str, armed: bool) {
    match armed {
        true => armed_pill_at(ui, pal, rect, text),
        false => pill_at(ui, pal, rect, text),
    }
}

/// `.pill.armed`: no border, a `--c-mint` word over a wash of the same, and the
/// `box-shadow: 0 0 9px var(--c-glow)` that goes with it.
fn armed_pill_at(ui: &Ui, pal: &Palette, rect: Rect, text: &str) {
    let painter = ui.painter();
    // `border-radius: 999px` on a box this short is a capsule, drawn as half
    // the box's own height rather than half [`size::PILL_H`] — the transition
    // row's pills count `.pill`'s border and are two pixels taller
    // ([`size::XPILL_H`]), and a radius read off the constant would leave
    // those three with a corner rather than a capsule.
    let radius = CornerRadius::same((rect.height() * 0.5) as u8);
    painter.add(
        egui::epaint::Shadow {
            offset: [0, 0],
            blur: ARMED_GLOW,
            spread: 0,
            color: pal.glow,
        }
        .as_shape(rect, radius),
    );
    painter.rect_filled(rect, radius, tint(pal.mint, ARMED_WASH));
    let galley = painter.layout_no_wrap(
        text.to_owned(),
        FontId::new(size::BASE, FontFamily::Proportional),
        pal.mint,
    );
    painter.galley(
        Pos2::new(
            rect.min.x + size::PILL_PAD_X,
            rect.center().y - galley.size().y * 0.5,
        ),
        galley,
        pal.mint,
    );
}

/// `.pill.on`: [`armed_pill_at`] in the pink rather than the mint, with the
/// heavier wash and the one-pixel-wider halo the mock gives it — no border, a
/// `--c-pink` word over `color-mix(in srgb, var(--c-pink) 16%, transparent)`,
/// and `box-shadow: 0 0 10px var(--c-glowp)`.
///
/// **The console's second treatment for a lit capsule, and the two say
/// different things.** `.pill.armed` is *this setting is chosen*; `.pill.on`
/// is drawn in the same pink a tally on air is, and what it says is *this is
/// live*: the press that runs the transition on the `go` it was written for,
/// the recording that is running on the `rec` capsule, and the deck a pane is
/// showing being on air on the Inspector's `keep`. **The mock does not reserve
/// it for `go`**, which this said until 2026-09-08 and which the mock has
/// contradicted in two rows since before it was written. Callers:
/// [`transition_into`] and [`inspector_into`].
fn on_pill_at(ui: &Ui, pal: &Palette, rect: Rect, text: &str) {
    let painter = ui.painter();
    // Half the box's own height, for [`armed_pill_at`]'s reason: this capsule
    // is an `XPILL_H` and not a `PILL_H`.
    let radius = CornerRadius::same((rect.height() * 0.5) as u8);
    painter.add(
        egui::epaint::Shadow {
            offset: [0, 0],
            blur: ON_GLOW,
            spread: 0,
            color: pal.glow_pink,
        }
        .as_shape(rect, radius),
    );
    painter.rect_filled(rect, radius, tint(pal.pink, ON_WASH));
    let galley = painter.layout_no_wrap(
        text.to_owned(),
        FontId::new(size::BASE, FontFamily::Proportional),
        pal.pink,
    );
    painter.galley(
        Pos2::new(
            rect.min.x + size::PILL_PAD_X,
            rect.center().y - galley.size().y * 0.5,
        ),
        galley,
        pal.pink,
    );
}

/// One `.pill`, painted into the capsule [`head_pills`] laid out for it.
fn pill_at(ui: &Ui, pal: &Palette, rect: Rect, text: &str) {
    let painter = ui.painter();
    painter.rect_stroke(
        rect,
        // `border-radius: 999px` on a box this short is a capsule, drawn as
        // half the box's own height — [`armed_pill_at`]'s reason.
        CornerRadius::same((rect.height() * 0.5) as u8),
        Stroke::new(1.0, pal.line),
        StrokeKind::Inside,
    );
    let galley = painter.layout_no_wrap(
        text.to_owned(),
        FontId::new(size::BASE, FontFamily::Proportional),
        pal.dim,
    );
    painter.galley(
        Pos2::new(
            rect.min.x + size::PILL_PAD_X,
            rect.center().y - galley.size().y * 0.5,
        ),
        galley,
        pal.dim,
    );
}

/// The mock's `.grip`: two columns of three dots.
const GRIP_COLS: usize = 2;
const GRIP_ROWS: usize = 3;
/// One dot's radius.
const GRIP_R: f32 = 1.0;
/// Centre to centre, both ways.
const GRIP_STEP: f32 = 3.5;

/// **How wide a grip is**, which [`head_pills`] steps back by before it places
/// the first pill. A constant rather than a return value, because the pills
/// are now laid out where nothing is painting.
///
/// **Public since the mark became a control** ([`bay_grip`]): 5.5 is what says
/// a target grown the way a `.pill` grows would reach into the capsule beside
/// it, and that measurement is a test's rather than this module's.
pub const GRIP_W: f32 = GRIP_STEP * (GRIP_COLS - 1) as f32 + GRIP_R * 2.0;

/// The mock's `.grip`, `⋮⋮` — drawn rather than typed, because whether a
/// vertical ellipsis is in `egui`'s default face is a question with no good
/// answer and six dots is the same mark either way. Right-aligned to `right`;
/// its width is [`GRIP_W`], which is what the pills beside it step back by.
fn grip_dots(ui: &Ui, pal: &Palette, right: Pos2) {
    let h = GRIP_STEP * (GRIP_ROWS - 1) as f32;
    let painter = ui.painter();
    for c in 0..GRIP_COLS {
        for r in 0..GRIP_ROWS {
            painter.circle_filled(
                Pos2::new(
                    right.x - GRIP_W + GRIP_R + c as f32 * GRIP_STEP,
                    right.y - h * 0.5 + r as f32 * GRIP_STEP,
                ),
                GRIP_R,
                pal.faint,
            );
        }
    }
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
    for (index, _) in layout.placed_children(id).enumerate().skip(1) {
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

#[cfg(test)]
mod tests {
    //! **What is private and worth asserting, and it is in `src/` rather than
    //! in `tests/` for that one reason.**
    //!
    //! Every region of this console is asserted from a file in `tests/`, one
    //! per region, and everything here would have been in one of them. It is
    //! here because what it names is private, and moving it out would mean
    //! widening a surface to test it — a wider surface for a narrower reason.
    //!
    //! **Most of it is the Inspector's**, which would have been
    //! `tests/inspector.rs`: [`pane_box`] and [`group_h`] are the pane's
    //! arithmetic, and the public [`inspector`] is that arithmetic plus a
    //! layout lookup. The rest is one figure the transport row writes —
    //! [`offset_text`] — for the same reason and no other.
    //!
    //! What is *not* here and belongs in `tests/inspector.rs` when somebody
    //! writes it: the bay drawn against a real arrangement at the two
    //! viewports, on `tests/library.rs`'s pattern.

    use super::*;

    /// A pane with `params` parameters under one node and no renderers.
    fn one_node(params: usize) -> Pane {
        Pane {
            deck: 0,
            material: "drift_shell + soft_points".to_owned(),
            sync: Sync::Tempo,
            // Nothing refused, which is what a pane about the arithmetic of
            // its rows says about material it is not asking after.
            allows: [true; SYNCS.len()],
            anchor_bpm: 128.0,
            scrub_beats: 0.0,
            composite: false,
            // Not this module's row: the deck head's two build chips are drawn
            // from this, and everything here is about the rows under it.
            aimed: None,
            nodes: vec![Node {
                addr: "L1:0".to_owned(),
                name: "drift_shell".to_owned(),
                authority: Some(NodeAuthority {
                    at: NodeAt {
                        layer: Layer::L1,
                        index: 0,
                    },
                    level: Authority::Manual,
                }),
                // **A node with a source**, which every node but the built-in
                // camera has — and the capsule this draws is what the tests
                // below measure the head's right-hand end against.
                keep: Some(NodeAt {
                    layer: Layer::L1,
                    index: 0,
                }),
                uses: Vec::new(),
                renderers: Vec::new(),
                params: (0..params)
                    .map(|n| Param {
                        ord: Some(n + 1),
                        name: format!("p{n}"),
                        value: 0.5,
                        range: [0.0, 1.0],
                        param: karakuri_operation::ParamAt {
                            node: None,
                            key: format!("p{n}"),
                        },
                        bound: None,
                    })
                    .collect(),
            }],
        }
    }

    /// A rectangle the size of one inspector pane at the narrowest console the
    /// mock will draw: `.console`'s `min-width: 1010px` holds the centre at
    /// 484 and each pane at 237.
    fn pane_at(height: f32) -> Rect {
        Rect::from_min_size(Pos2::new(0.0, 0.0), egui::vec2(237.0, height))
    }

    /// **The two heads are the mock's boxes and the leftover is the groups'.**
    ///
    /// `.half-head` is `5 + 16.5 + 5` over its own `border-bottom`, and
    /// `.deck-head` is `5 + 15.5 + 5` with none — the stylesheet's own
    /// arithmetic for that row. Everything under them is the body.
    #[test]
    fn a_pane_is_two_heads_and_what_is_left() {
        let pane = one_node(2);
        let at = pane_box(pane_at(400.0), &pane.nodes, 0.0).expect("a pane with room in it");
        assert_eq!(at.head.height(), 27.5);
        assert_eq!(at.deck_head.height(), 25.5);
        assert_eq!(at.head.max.y, at.deck_head.min.y);
        assert_eq!(at.deck_head.max.y, at.body.min.y);
        assert_eq!(at.body.max.y, 400.0);
    }

    /// **The inspector's own minimum is the least this draws**, and the two
    /// are one derivation: a pane at the minimum less the bay head has room
    /// for exactly the one group of two parameters the minimum is written
    /// from, and a pixel less has room for none of it.
    ///
    /// This is what the 151.5 in `lib.rs` *means*: bay head 27, `.half-head`
    /// 27.5, `.deck-head` 25.5, `.node-head` 26.5 and two `.param` rows at
    /// 22.5.
    #[test]
    fn the_bays_minimum_is_the_least_a_pane_can_draw() {
        let pane = one_node(2);
        let pane_h = 151.5 - size::HEAD_H;
        let at = pane_box(pane_at(pane_h), &pane.nodes, 0.0).expect("a pane at the minimum");
        assert_eq!(
            at.shown, 1,
            "one node and two of its parameters is what the minimum is written from"
        );
        let short = pane_box(pane_at(pane_h - 0.5), &pane.nodes, 0.0).expect("still two heads");
        assert_eq!(
            short.shown, 0,
            "half a pixel under the minimum and the group no longer fits"
        );
    }

    /// **A group the pane cannot hold whole is not counted as shown**, which
    /// is what the head's `n of m` means since the pane started scrolling: it
    /// is drawn, as far as the pane goes, and it is not one of the ones the
    /// readout says are on screen.
    #[test]
    fn a_group_that_does_not_fit_whole_is_not_counted() {
        let pane = Pane {
            nodes: vec![one_node(2).nodes[0].clone(), one_node(5).nodes[0].clone()],
            ..one_node(2)
        };
        // Room for the first group, the hairline, and all but the last row of
        // the second.
        let first = size::NODE_HEAD_H + size::PARAM_H * 2.0;
        let second = size::NODE_HEAD_H + size::PARAM_H * 5.0;
        let body = first + size::HAIRLINE + second - size::PARAM_H;
        let at = pane_box(
            pane_at(size::HALF_HEAD_H + size::DECK_HEAD_H + body),
            &pane.nodes,
            0.0,
        )
        .expect("a pane with room in it");
        assert_eq!(
            at.shown, 1,
            "the second group is short by one row, so one is whole"
        );

        // One row more and both are drawn.
        let at = pane_box(
            pane_at(size::HALF_HEAD_H + size::DECK_HEAD_H + body + size::PARAM_H),
            &pane.nodes,
            0.0,
        )
        .expect("a pane with room in it");
        assert_eq!(at.shown, 2);
    }

    /// **Where a group goes is the sum of the groups above it and the rules
    /// between them**, and the rule is between rather than under: *n* groups
    /// carry *n - 1* of them, because `.node-group:last-child` has none.
    #[test]
    fn a_group_stacks_under_the_one_before_it_with_a_rule_between() {
        let pane = Pane {
            nodes: vec![
                one_node(2).nodes[0].clone(),
                one_node(1).nodes[0].clone(),
                one_node(3).nodes[0].clone(),
            ],
            ..one_node(2)
        };
        let at = pane_box(pane_at(400.0), &pane.nodes, 0.0).expect("a pane with room in it");
        assert_eq!(at.shown, 3);
        let first = at.group(&pane.nodes, 0);
        let second = at.group(&pane.nodes, 1);
        let third = at.group(&pane.nodes, 2);
        assert_eq!(first.min.y, at.body.min.y);
        assert_eq!(second.min.y, first.max.y + size::HAIRLINE);
        assert_eq!(third.min.y, second.max.y + size::HAIRLINE);
        assert_eq!(first.height(), size::NODE_HEAD_H + size::PARAM_H * 2.0);
        assert_eq!(second.height(), size::NODE_HEAD_H + size::PARAM_H);
    }

    /// **A renderer row costs a group its own height**, and a group without
    /// one costs nothing: the mock draws `.rend-row` in the `L4` group and
    /// nowhere else.
    #[test]
    fn a_renderer_row_is_a_row_of_the_group_it_is_in() {
        let mut node = one_node(1).nodes[0].clone();
        let without = group_h(&node);
        node.renderers = vec![Renderer {
            name: "soft_points".to_owned(),
            live: true,
        }];
        assert_eq!(group_h(&node), without + size::REND_ROW_H);
    }

    /// **The anchor is the mock's own two spellings, and free shows neither.**
    ///
    /// *"`T128` is the tempo a deck was engaged at … `B128 +0.25` is that with
    /// the deck sitting a quarter beat ahead of the room. A free deck shows
    /// neither."*
    #[test]
    fn the_anchor_reads_what_the_mock_reads() {
        let mut pane = one_node(1);
        pane.sync = Sync::Tempo;
        pane.scrub_beats = 0.25;
        assert_eq!(
            anchor_text(&pane).as_deref(),
            Some("T128"),
            "tempo sync does not read the offset, so it is not drawn"
        );
        pane.sync = Sync::Beat;
        assert_eq!(anchor_text(&pane).as_deref(), Some("B128 +0.25"));
        pane.sync = Sync::Free;
        assert_eq!(
            anchor_text(&pane),
            None,
            "free is the absence of a transport rather than a setting"
        );
    }

    /// **The pane head names the deck it is pointed at and what is in it**,
    /// which is the mock's `deck A · drift_night`.
    #[test]
    fn the_pane_head_says_which_deck_it_is_showing() {
        let mut pane = one_node(1);
        pane.deck = 1;
        assert_eq!(showing_text(&pane), "deck B · drift_shell + soft_points");
    }

    /// **Every level the vocabulary names is on the node head.**
    ///
    /// `karakuri_operation::Authority` carries no `ALL` — *"it arrives with
    /// the first reader"* — so [`AUTHORITIES`] is this crate's list, and the
    /// thing that can go wrong is the list falling behind the vocabulary while
    /// [`auth_word`] is updated. This holds the two together: every level
    /// [`auth_word`] can spell is in the array exactly once, and the array is
    /// in the vocabulary's own declaration order.
    #[test]
    fn every_authority_the_vocabulary_names_is_on_the_node_head() {
        // A `match` that a fourth level would not compile past, which is what
        // makes this a check on the *array* rather than on the enum.
        let expected = [
            Authority::Manual,
            Authority::Suggesting,
            Authority::Automatic,
        ];
        assert_eq!(
            AUTHORITIES.len(),
            expected.len(),
            "a level the vocabulary names is missing from the node head"
        );
        for (n, level) in expected.into_iter().enumerate() {
            assert_eq!(AUTHORITIES[n], level, "the three are in declaration order");
            assert!(
                !auth_word(level).is_empty(),
                "every level has the console's own abbreviation for it"
            );
        }
        let words: Vec<&str> = AUTHORITIES.into_iter().map(auth_word).collect();
        assert_eq!(
            words,
            vec!["man", "sug", "auto"],
            "the manual's node head reads `man / sug / auto`"
        );
    }

    /// **A pane narrower than a parameter row's own padding is no pane**,
    /// which is the picture's rule stated across the axis.
    #[test]
    fn a_pane_with_no_room_across_it_draws_nothing() {
        let pane = one_node(2);
        let narrow = Rect::from_min_size(
            Pos2::new(0.0, 0.0),
            egui::vec2(size::PARAM_PAD_L + size::PARAM_PAD_R, 400.0),
        );
        assert!(pane_box(narrow, &pane.nodes, 0.0).is_none());
    }

    /// **There are two panes and there is no third.**
    ///
    /// The mock's `2 up ▾` is an operator choosing how many while running, and
    /// a [`Spec`](karakuri_layout::Spec) builds a
    /// [`Layout`](karakuri_layout::Layout) once — so the count is the
    /// arrangement's, and a caller asking for a pane it has not got gets
    /// `None` rather than one of the two it has.
    ///
    /// And a console with no deck behind it hands over no pane at all, which
    /// is every test in this crate.
    #[test]
    fn there_are_two_panes_and_no_third() {
        let mut layout = crate::layout();
        layout.set_viewport(karakuri_layout::Rect {
            x: 0.0,
            y: 0.0,
            w: 1920.0,
            h: 1080.0,
        });
        layout.solve();
        let pane = one_node(2);
        assert_eq!(PANES, 2, "the mock's `.insp-split` is `1fr 9px 1fr`");
        for index in 0..PANES {
            assert!(
                inspector(&layout, index, &pane, 0.0).is_some(),
                "pane {index} is in the arrangement and has room in it"
            );
        }
        assert!(
            inspector(&layout, PANES, &pane, 0.0).is_none(),
            "a third pane is a pane the arrangement has not got"
        );
        assert!(View::new(Room::Day).inspector.is_empty());
    }

    /// **A pane starts under the bay head and not on top of it.**
    ///
    /// The one thing `pane_box`'s own tests cannot see. They are handed a
    /// rectangle and the head has already been taken off it — the fixture
    /// says so in the arithmetic, `151.5 - size::HEAD_H` — so every one of
    /// them passes whether or not the caller does the subtraction. It did
    /// not: `.half-head` was drawn at the top of `inspector-1`, which is the
    /// top of the bay, which is where [`bay_head`] paints the word
    /// `Inspector`. Read against the arrangement rather than against a
    /// fixture, because the fixture is what could not tell.
    #[test]
    fn a_pane_starts_under_the_bay_head() {
        let mut layout = crate::layout();
        layout.set_viewport(karakuri_layout::Rect {
            x: 0.0,
            y: 0.0,
            w: 1920.0,
            h: 1080.0,
        });
        layout.solve();
        let pane = one_node(2);
        let bay = to_egui(layout.rect(layout.find("inspector").expect("the bay is named")));
        for index in 0..PANES {
            let at = inspector(&layout, index, &pane, 0.0).expect("a pane with room in it");
            assert_eq!(
                at.head.min.y,
                bay.min.y + size::HEAD_H,
                "pane {index} starts where the bay head ends"
            );
            assert!(
                bay.contains_rect(at.head) && bay.contains_rect(at.deck_head),
                "pane {index} draws inside the bay it is in"
            );
        }
    }

    /// **The offset figure carries its sign and its unit, and it has no
    /// `-0 ms`.**
    ///
    /// `docs/manual/console.html` is why the sign is always drawn — *"the sign
    /// is the half that gets read wrong at two in the morning"* — and why the
    /// unit is never left off: *"Two things on this panel are called an
    /// offset … The unit is what tells them apart."*
    ///
    /// **The zero is the case worth an assertion.** The track is 80 pixels for
    /// a range of 400 ms, so a press a fraction of a pixel either side of the
    /// middle asks for a value that rounds to zero — and a `{:+.0}` written
    /// straight off it answers `-0 ms` on one side and `+0 ms` on the other,
    /// which is two spellings of the one value an operator is most likely to
    /// be aiming at.
    #[test]
    fn the_offset_figure_is_signed_carries_its_unit_and_has_one_zero() {
        assert_eq!(offset_text(-15.0), "-15 ms");
        assert_eq!(offset_text(15.0), "+15 ms");
        assert_eq!(offset_text(LATENCY_OFFSET_MIN_MS), "-200 ms");
        assert_eq!(offset_text(LATENCY_OFFSET_MAX_MS), "+200 ms");
        // Either side of the middle, and exactly on it.
        assert_eq!(offset_text(0.0), "+0 ms");
        assert_eq!(offset_text(-0.4), "+0 ms");
        assert_eq!(offset_text(0.4), "+0 ms");
        // And the first value on each side that is not zero still says which
        // side it is on, so the normalisation above has not swallowed a sign.
        assert_eq!(offset_text(-0.6), "-1 ms");
        assert_eq!(offset_text(0.6), "+1 ms");
    }
}
