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

mod transport;

// **The one symbol Transport keeps crate-internal rather than exporting**:
// `next_tonemap` backs [`crate::focus`]'s own cycling of the same control,
// which needs the derivation and not the row.
pub(crate) use transport::next_tonemap;

/// **The Transport bay's own module**, split out under
/// [ADR-0121](../../../../docs/adr/0121-moving-code-leaves-its-reasoning-behind.md)'s
/// rule that a move carries its reasoning with it: every symbol below is
/// [`transport`]'s, re-exported here so that nothing outside this crate that
/// already writes `view::transport` or `view::TransportRow` has to learn a
/// second path for it.
pub use transport::{
    arrangement, audio_in, beat_at, exposure_at, learn_pill, look, map_pill, offset_at,
    tracker_group, transport, unit_of, unit_of_offset, Arrangement, ArrangementPill, Ask, AudioAsk,
    AudioIn, AudioInPill, Item, LearnPill, Look, LookRow, MapPill, MapRow, Menu, Rec, Tracker,
    TrackerGroup, Transport, TransportRow, BEAT_PITCH, BEAT_STALENESS, EXPOSURE_MAX, EXPOSURE_MIN,
    EXPOSURE_TRACK_W, LATENCY_OFFSET_MAX_MS, LATENCY_OFFSET_MIN_MS, LATENCY_OFFSET_STEP_MS,
    OFFSET_TRACK_W, TEMPO_BAND, TEMPO_SPAN,
};

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

mod inspector;

/// **The Inspector bay's own module**, split out under
/// [ADR-0121](../../../../docs/adr/0121-moving-code-leaves-its-reasoning-behind.md)'s
/// rule that a move carries its reasoning with it: every symbol below is
/// [`inspector`]'s, re-exported here so that nothing outside this crate that
/// already writes `view::inspector` or `view::InspectorPane` has to learn a
/// second path for it.
pub use inspector::{
    auth_chips, count_text, deck_head, deck_name, inspector, keep_pill, node_keep, pane_count,
    pane_target, rend_chips, sens_chips, AimChips, Aimed, DeckHead, DeckName, InspectorPane,
    KeepPill, Naming, Node, NodeAuthority, Pane, PaneTarget, Param, ParamGrip, Renderer, SensChip,
    Source, Uses, UsesLine, AUTHORITIES, PANES, PANE_DECKS, PANE_NAMES, RE_SALT_LABEL, SCRUB_BEATS,
    SENS_LABEL, SYNCS, TAKE_BACK,
};
// **Two symbols Inspector keeps crate-internal rather than exporting**:
// `arrow_mark` backs the Library bay's own `load → A` arrow between its two
// capsules, and `next_sync` backs [`crate::focus`]'s own cycling of the deck
// head's sync chip — both need the derivation and not a second copy of it.
pub(crate) use inspector::{arrow_mark, next_sync};

mod program;

/// **The Program bay's own module**, split out under
/// [ADR-0121](../../../../docs/adr/0121-moving-code-leaves-its-reasoning-behind.md)'s
/// rule that a move carries its reasoning with it: every symbol below is
/// [`program`]'s, re-exported here so that nothing outside this crate that
/// already writes `view::program` or `view::ProgramBay` has to learn a second
/// path for it.
pub use program::{
    band_of, caption_of, picture_rect, preview_rects, program_bay, program_body,
    program_body_with_row_h, program_head, rearrange, Band, Basis, Body, Budgeted, Picture,
    Placement, ProgramBay, ProgramHead, BAND_BLUE_MS, BAND_PURPLE_MS, BAND_RED_MS, BAND_YELLOW_MS,
    MOCK_CANVAS, PREVIEW_MATERIAL, PREVIEW_NO_SLOT, PREVIEW_OVERLOADED,
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

/// A card of that size at that corner, pushed back inside the viewport's right
/// edge if it would hang over it — and never past its left edge, which is what
/// the `max` is for on a window narrower than the card.
fn held_inside(viewport: &Rect, x: f32, y: f32, w: f32, h: f32) -> Rect {
    let x = x.min(viewport.max.x - w).max(viewport.min.x);
    Rect::from_min_size(Pos2::new(x, y), egui::vec2(w, h))
}

// ---------------------------------------------------------------------------
// Shared across more than one bay
//
// Each of these physically sat inside the Transport bay's own range of the
// file this crate used to be one module, and stayed here rather than moving
// to `transport` under the Mixer and Library extractions' rule (ADR-0121):
// a symbol moves with the bay that owns it, and none of these is Transport's
// alone. `cargo build` is what found each one — a private item is visible to
// a child module but not to a sibling, so a first attempt at moving every one
// of them into `transport.rs` failed to compile at exactly these names.
// ---------------------------------------------------------------------------

/// **The tempo this row is drawn at in the mock** — `.bpm`'s `128.0` — in
/// thousandths of a beat a minute, so [`BEAT_STALENESS`]'s arithmetic stays in
/// integers.
const MOCK_BPM_MILLI: u64 = 128_000;

/// **One beat at that tempo**, in microseconds: 468 750, which is 468.75 ms.
const BEAT_MICROS: u64 = 60 * 1_000_000 * 1_000 / MOCK_BPM_MILLI;

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

/// **The `▾` at the end of the pill, drawn rather than typed** — [`grip_dots`]'
/// reason one control along. Whether a black down-pointing small triangle is
/// in `egui`'s default face is a question with no good answer, and a triangle
/// is the same mark either way.
///
/// Half the type it sits beside wide and half of that tall, which is about
/// what the glyph's ink measures at [`size::BASE`].
const CHEVRON_W: f32 = size::BASE * 0.5;
const CHEVRON_H: f32 = CHEVRON_W * 0.5;

/// **The caret**, a light vertical bar. Typed rather than drawn, unlike the
/// chevron: `U+258F LEFT ONE EIGHTH BLOCK` is a box-drawing character, and
/// `egui`'s default face carries the block elements. Where it did not, the
/// fallback is a visible box in the one place an operator is looking, which is
/// louder than a caret that has quietly gone.
const CARET: char = '▏';

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
                            transport::transport_into(ui, &pal, &row);
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
                            transport::tracker_into(ui, &pal, &group);
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
                            transport::look_into(ui, &pal, &row);
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
                    program::preview(ui, &pal, cell, previews[deck]);
                    program::caption_into(
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
                    inspector::inspector_into(
                        ui,
                        &pal,
                        &at,
                        pane,
                        inspector::on_air(strips, pane.deck),
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
                    transport::audio_in_into(ui, &pal, &pill, audio);
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
                transport::learn_into(ui, &pal, &pill);
            }
            if let Some((pill, map)) =
                map_pill(ui.ctx(), panel.layout(), values, audio, tracking, map).zip(map)
            {
                transport::map_into(ui, &pal, &pill, map);
            }
            if let Some(pill) =
                arrangement(ui.ctx(), panel.layout(), values, audio, tracking, map, arr)
            {
                transport::arrangement_into(ui, &pal, &pill, arr);
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
                                inspector::uses_card_into(ui, &pal, &line, uses, card, room);
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
                                inspector::pane_list_into(ui, &pal, &target, pane.deck, card);
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
