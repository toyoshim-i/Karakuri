//! The console, drawn: every region in its place, with its heading, and nothing
//! inside.
//!
//! This is the half of the crate that knows a toolkit. It reads the solved
//! rectangles out of [`Panel`] and paints them with `egui`; it holds no fact
//! about the arrangement and answers no question [`karakuri_layout::Layout`]
//! can answer.
//!
//! # What is here, and what is deliberately not
//!
//! Every leaf of the arrangement gets its region, and seven of them get a bay
//! head. Nothing else. A bay's body is empty and looks it — no placeholder
//! rows, no sample values, no greyed-out control that hints at what will be
//! there. Scaffolding that looks finished does not get replaced, and the next
//! person to open the panel should be in no doubt about what exists.
//!
//! One body is not empty, and it is not an exception to that. The Program bay's
//! picture is a [`Kind::Picture`], and what it draws is a texture handed in
//! from outside — real texels off a real device, not a mock-up of some. It
//! draws nothing at all when there is no texture, exactly as every other empty
//! body does. See [`Picture`] and [`picture_rect`].
//!
//! # The deck previews are the one thing drawn with nothing behind it
//!
//! [`View::draw`] paints four cells whether or not a deck is running in any of
//! them, and that is not the scaffolding the paragraph above forbids. A
//! greyed-out control is a placeholder for a control that does not exist yet; a
//! preview cell is not a placeholder for anything, it is the region's own face.
//! The mock's `.preview` is a bare well before it is a picture — one of its
//! four cells holds no texels at all — and the Program bay's head reads
//! *previews 3 of 4*, which is an operator's ordinary choice and not a state
//! waiting to be finished. A cell with nothing behind it is what that state
//! looks like, not a stand-in for a full one, and the caption under it says
//! which nothing: `material` where there is a picture and `no slot` where there
//! is not.
//!
//! So the two rules are one rule. Nothing is drawn that claims something exists
//! which does not; a cell with no slot behind it exists and says so. See
//! [`DECKS`], [`preview_rects`] and [`View::previews`], and
//! [ADR-0170](../../../../docs/adr/0170-a-deck-preview-cell-is-drawn-whether-or-not-a-deck-is-behind-it.md)
//! for the alternative that lost and for what would reopen it.
//!
//! # The Staging lane draws a row per node a build changed
//!
//!
//! [ADR-0200](../../../../docs/adr/0200-a-bays-first-pass-draws-the-values-that-exist-and-omits-the-rest.md)
//! draws every part of the mock that has a value behind it and omits the rest.
//! Applied to this bay it draws four of the six things a candidate row could
//! carry — the deck, the node's address, what that node's procedure calls
//! itself, and whether it is on screen — and omits the other two outright.
//!
//! This section used to say the lane draws nothing, and the reason it gave was
//! true of the program rather than of the workspace. Both halves of the
//! producer were already public and already wired elsewhere:
//! `karakuri_engine::deck::Deck::slot` hands out the `HotSwap` a verdict comes
//! out of, and `karakuri-cli` builds its slots with `HotSwap::new` over a
//! `karakuri_environment::watch::Watch`. What was true is narrower: the program
//! this panel is drawn by built both its deck slots with `HotSwap::fixed`,
//! whose `Receiver`'s `Sender` is dropped at construction — so no `swap::Event`
//! of any variant was emitted in any run of it, and a row would have been
//! [ADR-0191](../../../../docs/adr/0191-the-panels-parked-deck-is-parked-by-the-governor-or-it-is-a-drawing-of-one.md)'s
//! parked chip. That was host wiring and it is now wired: `crates/karakuri`
//! watches its own `.kir` pair per slot, and a save is a build, a swap and a
//! verdict.
//!
//! What a row is, and it is the node rather than the slot. A `swap::Event`
//! carries an `id` and a `label` and no node, because a
//! `karakuri_engine::Request` restates *every* node of the slot — so a verdict
//! is over a build. Which node of that build changed is a diff:
//! `karakuri_environment::watch::Built` carries `(layer, index, hash)` for the
//! whole stack on every build, and consecutive builds differ where the hashes
//! do. The caller takes it where both lists are in one hand and hands in one
//! [`Candidate`] per changed node, with the build's one verdict on each of them
//! ([ADR-0326](../../../../docs/adr/0326-a-staging-row-is-a-changed-node-and-the-row-is-the-keep.md)).
//! A verdict with no changed node behind it draws one row on the slot with no
//! address — a build that did not happen has no node list to hold against the
//! one before it, and a rebuild that restated the stack unchanged has an empty
//! diff.
//!
//! The two omissions, each with what it waits on, are written out at
//! [`staging`]: the coloured dot and the `you` in `you, 14:41` (`origin` has no
//! producer anywhere) and the `14:41` itself (three spellings exist and none is
//! chosen — the same decision the Library's time column waits on). The head's
//! `2 waiting` is not drawn either, because a bay head's pills are its controls
//! and the rows are the count. The mock's third `.cand` — *a rejected candidate
//! costs nothing* — is a note to whoever is reading the mock and not a thing
//! the lane draws, and empty this lane draws *"no row, no placeholder, and no
//! standing sentence"*, which is still every frame of every run until somebody
//! saves a file.
//!
//! Both of the lane's operations are built, and they are the row and a capsule
//! in it. `karakuri_operation::Operation` spells *Keep a candidate* and *Put a
//! node's previous version back* `{ deck, node }` apiece, and the node is what
//! the row now names. A press on the row keeps that candidate —
//! `Written::Silent(Silent::Surface)`, so nothing moves and the row leaves —
//! and the `back` capsule at its end lands that node's previous version, which
//! is a build like any other and writes its record at the swap
//! (`Written::Silent(Silent::OnLanding)`). The free act is on the large target
//! and the act that writes a file is on the small one. What the row says
//! without either is still the thing nothing else in this instrument says:
//! after `swap::Event::Overloaded` the version is still in the slot and the
//! slot has stopped updating, so what is on screen is a held frame and only a
//! word says so — the row's, and the deck cell's caption beside it — and the
//! capsule on that row is the way out.
//!
//! The height is what this leaves wrong, and it is not this module's to fix.
//! `lib.rs` pins the lane at `fixed(125.0)` with a minimum of `66.0` — the
//! mock's three `.cand` rows, and one — and empty is still this lane's ordinary
//! state, so those pixels are held open over nothing at the expense of the
//! Library, which is the bay in that column that absorbs. A content-height lane
//! needs `arrangement()` to take an argument, or `karakuri-layout` to grow a
//! setter for a view's size, and it has neither. The numbers stay where they
//! are, and what they buy is that a lane which fills up has the room the mock
//! gave it.
//!
//! # The Program bay's body arranges itself, and that is one derivation
//!
//! The picture and the four cells are not where the two regions that hold them
//! are: on a wide, short bay the cells go down the sides of the picture and
//! `deck-previews` is taken out of the layout altogether, which is what leaves
//! the bay's width to the picture. [`program_bay`] is the one place that
//! decides it and [`rearrange`] is what writes the one bit that follows;
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
//! The transport and the outputs are rows, not bays: they carry no heading,
//! because they have none in the mock — both carry `class="bay"`, which is the
//! card styling, and neither carries a `.bay-head`
//! ([ADR-0159](../../../../docs/adr/0159-the-consoles-words-are-the-manuals-and-the-middle-one-is-not-a-pane.md)).
//!
//! # The transport row is four readouts and nine controls
//!
//! The four readouts are the beat grid, the bar, the frame readout and the
//! health capsule — the things in the mock's row that are a value somebody
//! measured rather than a control over something that does not exist. All four
//! are [`Kind::Transport`]'s, which is [`transport`]. The capsule joined them
//! on 2026-09-08: it is drawn as a `.pill` and is no more a control than the
//! frame readout beside it, which is the reading a press proves rather than the
//! shape of the box.
//!
//! The nine controls are six rows of [`crate::input::PROBES`], and the row is
//! where this module's rule has cost the most and bought the most back. Two of
//! them are [`transport`]'s own — the tempo figure ([`TransportRow::tempo`])
//! and the `rec` pill ([`TransportRow::record`]), which landed together and are
//! why the tempo is not counted above: the number is the track a press names a
//! value on
//! ([ADR-0291](../../../../docs/adr/0291-the-tempo-figure-is-the-track-and-the-band-is-a-guard-on-the-hand.md)).
//! The other seven are laid out from that row and derived beside it: the
//! audio-in pill ([`audio_in`]), the tracker group's tap, offset and octave
//! ([`tracker_group`]), the arrangement pill ([`arrangement`]), and the tone
//! map's capsule and exposure track ([`look`]). The ones the console still
//! cannot know are named in [`transport`], one by one, with what is missing
//! behind each.
//!
//! The count is of what this crate draws and not of the mock, which is why it
//! is written here at all: the nine are what those six probe rows claim between
//! them, so the figure is checkable against one table rather than remembered.
//! The mock's own total is not written anywhere in this module and must not be
//! — see the paragraph below.
//!
//! The arrangement pill is the one that was here first, and it was here for the
//! rule rather than despite it. Every control in this row that is still undrawn
//! is a control over machinery that is in neither this crate nor the program —
//! a learn mode with no map, a map file this program has not got. The
//! arrangement is not one of those: it is the tree this crate owns, it is
//! already saved and put back by name over a real file, and `r` already resets
//! it. So `arr · night ▾` is a control over something that exists, which is the
//! only test this module's rule has ever applied — see [`arrangement`],
//! [`Arrangement`] and
//! [ADR-0221](../../../../docs/adr/0221-an-arrangement-is-named-by-the-operator-and-kept-in-a-fourth-place.md).
//! It is the mock's own `map · nanoKONTROL2 ▾` in shape and in position,
//! because a named file with *save*, *load* and *start a new one* under one
//! control is exactly the family, and the map pill is the same three over a
//! file this program has not got.
//!
//! No count is written here. This comment said *six of the mock's ten* and the
//! mock has gained an offset pill and an octave pair since, which is the third
//! time a transcribed figure about that page has gone stale in this repository.
//! What the mock holds is `docs/manual/console.html`'s `.transport` block, and
//! counting it is a command rather than a memory.
//!
//! What it draws with no engine behind it is nothing at all — not a row of
//! zeroes and not a row of dashes, either of which is a reading invented for a
//! console that has none. That is [`View::picture`]'s rule, and the values
//! arrive the same way it does: a plain [`Transport`] written per frame by
//! whoever owns the engine, because `src/` takes no device, no window and no
//! clock (ADR-0156) and this row is made of a clock and an engine.
//!
//! # The Outputs row has one control in it, and it is the console's first
//!
//! [`Kind::Outputs`] draws the word OUTPUTS and one `.sink` — the picture,
//! which the manual lists as *program view*. Everything else in the mock's row
//! is a control over something that does not exist, and [`outputs`] names each
//! of them and says why it is not drawn. The dot is lit from
//! [`Layout::visible`] on the picture's node and a press on it asks for
//! [`Op::Fold`] or [`Op::Unfold`] by name, so the control and the keyboard
//! reach one operation between them rather than two states that agree until
//! they do not.
//!
//! It is also the first thing on the panel a press has to *reach*, which is the
//! rule [`crate::input`] was written for and the clearance `tests/outputs.rs`
//! holds.
//!
//! # The Mixer bay is the first bay with something in its body
//!
//! [`Kind::Mixer`] draws the card and the head every other bay gets, and then
//! as many strips as the deck has — not four. `Deck::slot_count` is what there
//! is, and a page of this bay has [`DECKS`] tracks whatever that number is, so
//! a track with no strip in it draws nothing at all rather than an empty strip.
//! That is not the deck previews' rule read backwards. A preview cell is the
//! region's own face and says `no slot`; a strip is six readings, and an empty
//! one is six readings nobody took — the row of zeroes ADR-0177 refuses, with a
//! different glyph. See [`mixer`] and [`Strip`], and [`Meter`] for why a meter
//! is not a [`Fader`].
//!
//! Five things in that bay answer a pointer and the rest are readouts, and the
//! source says which rather than leaving the next reader to discover it. The
//! two fader knobs are played, and the blend mini, the tally chip and the mask
//! mini each cycle
//! ([ADR-0185](../../../../docs/adr/0185-a-fader-translates-a-drag-into-an-operation-and-applies-nothing.md),
//! [ADR-0187](../../../../docs/adr/0187-the-blend-mini-cycles-and-a-map-learns-the-three-it-cycles-through.md),
//! [ADR-0195](../../../../docs/adr/0195-the-tally-chip-cycles-from-the-request-so-the-parked-case-needs-no-case.md),
//! [ADR-0203](../../../../docs/adr/0203-the-mask-chip-carries-the-angle-it-does-not-control.md)):
//! each emits an [`Operation`] and applies nothing, because the value belongs
//! to the engine rather than to the arrangement. The meter and the number are
//! drawn from what the deck says and a press on either reaches nothing — and so
//! does a press on a fader's *track*, off the knob.
//!
//! The mask mini is the one control that carries a value it never draws. The
//! chip says *which shape*, and [`Operation::SetMaskShape`] carries a shape and
//! an angle; so [`Strip::mask_angle`] is read to build the operation and
//! painted nowhere, which is what stops choosing a shape from straightening a
//! diagonal front ([`Mixer::mask`]).
//!
//! The tally is the one control that reads two values. `Deck::residency` is
//! what a slot is doing and `Deck::requested_residency` is what it was asked to
//! do, and while they disagree the chip's word rolls part of the way toward the
//! request and falls back, once a second, and never lands — see
//! [`Strip::pending`], [`roll_at`] and [`mixer::tally_into`], and
//! [P-0087](../../../../docs/principles/0087-name-the-property-never-the-shape.md)
//! for what that has to say. A press on it cycles from the residency that was
//! *requested* ([`Mixer::tally`]), which is what makes a parked chip's press
//! the withdrawal of its own prime request with no case in the code for it. It
//! is the panel's first live region and still its only one ([`View::declares`],
//! which answers with the bay's name, what one update of it costs and how stale
//! it may get) — and it declares only while the bay it rolls in is laid out,
//! because a fold takes the chip off the screen and a price paid for what
//! nobody can see is ADR-0193 broken rather than served.
//!
//! # The bay head is one component with seven call sites
//!
//! [`bay_head`] is written once and seven bays call it: six through
//! [`Kind::Bay`] in [`REGIONS`] and the mixer through [`Kind::Mixer`], which is
//! a bay whose body is not empty. That is this repository's rule about an
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
//! have to be an offset applied to every rectangle on the way out and undone on
//! every pointer coordinate on the way in — a second coordinate space, for ten
//! pixels of margin. The window's edge is the panel's edge here, and the ground
//! shows in the dividers alone.

use std::time::Duration;

use egui::epaint::text::{LayoutJob, TextFormat};
use egui::{Color32, CornerRadius, FontFamily, FontId, Pos2, Rect, Stroke, StrokeKind, Ui};
use karakuri_layout::{Axis, Hit, NodeId};
use karakuri_operation::gate::{Class, Open};
use karakuri_operation::{
    Authority, BeatSource, BlendMode, GridScale, LaneTarget, Layer, LibraryKinds, NodeAddress,
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

/// The Transport bay's own module, split out under
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

/// The Mixer bay's own module, split out under
/// [ADR-0121](../../../../docs/adr/0121-moving-code-leaves-its-reasoning-behind.md)'s
/// rule that a move carries its reasoning with it: every symbol below is
/// [`mixer`]'s, re-exported here so that nothing outside this crate that
/// already writes `view::mixer` or `view::TransitionRow` has to learn a second
/// path for it.
pub use mixer::{
    after, mixer, roll_at, roll_moves_in, transition, Fader, Go, Level, Mask, Meter, Mixer, Phase,
    Reach, Strip, StripBox, Tally, TransitionRow, TransitionSettings, ROLL_PERIOD, ROLL_REACH,
    ROLL_STALENESS, ROLL_TRAVEL,
};
pub(crate) use mixer::{next, next_shape, residency, wipe_kind};

mod library;

/// The Library bay's own module, split out under
/// [ADR-0121](../../../../docs/adr/0121-moving-code-leaves-its-reasoning-behind.md)'s
/// rule that a move carries its reasoning with it: every symbol below is
/// [`library`]'s, re-exported here so that nothing outside this crate that
/// already writes `view::library` or `view::LibraryBay` has to learn a second
/// path for it.
pub use library::{
    library, load_item, Aim, Block, Chosen, Field, Filters, KindChip, LibraryBay, Load, Menued,
    Opened, Picked, Pointed, Pointing, Published, Read, Reading, RowItem, RowKind, RowMenu, Rows,
    Scope, Taken, Target, Wiring, HOLDS_UNSET, LAYERS, SETS_CHIP,
};

mod inspector;

/// The Inspector bay's own module, split out under
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

/// The Program bay's own module, split out under
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

mod master;

/// The Master bay's own module, split out under
/// [ADR-0121](../../../../docs/adr/0121-moving-code-leaves-its-reasoning-behind.md)'s
/// rule that a move carries its reasoning with it: every symbol below is
/// [`master`]'s, re-exported here so that nothing outside this crate that
/// already writes `view::master` or `view::MasterRow` has to learn a second
/// path for it.
pub use master::*;

mod sequencer;

/// The Sequencer bay's own module, split out under
/// [ADR-0121](../../../../docs/adr/0121-moving-code-leaves-its-reasoning-behind.md)'s
/// rule that a move carries its reasoning with it: every symbol below is
/// [`sequencer`]'s, re-exported here so that nothing outside this crate that
/// already writes `view::sequencer` or `view::Sequencer` has to learn a second
/// path for it.
pub use sequencer::*;
// **Two symbols Sequencer keeps crate-internal rather than exporting**:
// `lane_label` and `FADER_ITEM` back [`View::lane_choices`], which needs the
// derivation and the word without re-exporting them as the public surface.
pub(crate) use sequencer::{lane_label, FADER_ITEM};

mod staging;

/// The Staging bay's own module, split out under
/// [ADR-0121](../../../../docs/adr/0121-moving-code-leaves-its-reasoning-behind.md)'s
/// rule that a move carries its reasoning with it: every symbol below is
/// [`staging`]'s, re-exported here so that nothing outside this crate that
/// already writes `view::staging` or `view::StagingBay` has to learn a second
/// path for it.
pub use staging::*;

mod layout;

/// The layout and geometry placement module, split out under
/// [ADR-0121](../../../../docs/adr/0121-moving-code-leaves-its-reasoning-behind.md)'s
/// rule that a move carries its reasoning with it: every symbol below is
/// [`layout`]'s, re-exported here so that nothing outside this crate that
/// already writes `view::to_egui` or `view::Placed` has to learn a second path
/// for it.
pub use layout::*;
pub(crate) use layout::{held_inside, positive, track};

mod outputs;

/// The Outputs row's own module, split out under
/// [ADR-0121](../../../../docs/adr/0121-moving-code-leaves-its-reasoning-behind.md)'s
/// rule that a move carries its reasoning with it: every symbol below is
/// [`outputs`]'s, re-exported here so that nothing outside this crate that
/// already writes `view::outputs` or `view::Outputs` has to learn a second path
/// for it.
pub use outputs::*;

pub mod widgets;

/// The shared interactive widgets module, split out under
/// [ADR-0121](../../../../docs/adr/0121-moving-code-leaves-its-reasoning-behind.md)'s
/// rule that a move carries its reasoning with it: every symbol below is
/// [`widgets`]'s, re-exported here so that nothing outside this crate that
/// already writes `view::FoldGrip` or `view::bay_grip` has to learn a second
/// path for it.
pub use widgets::*;
#[allow(unused_imports)]
pub(crate) use widgets::*;

/// The whole of a texture, in `egui`'s texture coordinates. The picture fills
/// its rectangle, and that is now one answer rather than two.
///
/// # What changed, and why the argument is not deleted
///
/// This used to read *"the fitting happened when the engine drew into it, and
/// doing it again here would be two answers to how does a 16:9 canvas sit in
/// this box"*. That was true while [`picture_rect`] handed the engine the whole
/// `program-view` region: the region was not the canvas's shape, so
/// `Present::draw` had to choose where the canvas sat inside it, and a second
/// fit here would have chosen differently.
///
/// The console decides the box now. [`picture_rect`] gives the picture the
/// canvas's own aspect, so the engine is handed a target it already agrees with
/// and its fit has nothing left to do. The two answers did not become one by
/// one of them being dropped — one of them became the identity.
///
/// # The engine's letterbox stays, and it is not redundant
///
/// [`fitted`] rounds to whole pixels and the caller's own `physical` rounds
/// again to whole texels, so the target is never exactly the canvas's ratio:
/// the mock's 466 x 262 is 16:9 to a quarter of a pixel and no closer.
/// `letterbox` is what absorbs that, and after this rule the bars it draws are
/// sub-texel where they used to be the black half of a region.
const WHOLE_TEXTURE: Rect = Rect {
    min: Pos2::new(0.0, 0.0),
    max: Pos2::new(1.0, 1.0),
};

/// How many deck preview cells there are, and it is derived rather than picked
/// off the mock.
///
/// `karakuri_engine`'s `deck::MAX_SLOTS` is 4 — *"a deck holds 1 to 4 slots"*,
/// asserted in `Deck::new` — so four is the most auditions there can ever be at
/// once, and a fifth cell would be a cell no deck can ever fill. The mock
/// agrees from the other end: `.previews` is `grid-template-columns: repeat(4,
/// 1fr)` with four `.preview` cells in it, and the Program bay's head reads
/// *previews 3 of 4*. Two readings, one number.
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

/// How many cells go down one side when they are beside the picture: half of
/// [`DECKS`], which is two — A and B down the left, C and D down the right.
///
/// Two columns of two rather than one column of four, and the arithmetic is
/// worth writing down because it does not point where a reader expects. A
/// column of *n* cells stacked down a body *H* tall is about `H / n` per cell,
/// so it is `(H / n) * 16 / 9` wide, and all four of them cost `(4 / n) * (H /
/// n) * 16 / 9` — one column of four is the cheapest arrangement and four
/// columns of one is the dearest. So this is not the choice that leaves the
/// picture the most room; one side of four would leave more.
///
/// It is chosen for the two things that are not width. The picture stays
/// centred in the bay with equal ground either side, which is what [`fitted`]
/// gives every other box on this panel; and a cell stays `H / 2` rather than `H
/// / 4` tall, which at the mock's own body is 163 against 79 — a preview an
/// operator can read a cut in against one that is a smaller thumbnail than the
/// row it replaced.
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
const SOLO_PILL: &str = "solo";

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

/// The tempo this row is drawn at in the mock — `.bpm`'s `128.0` — in
/// thousandths of a beat a minute, so [`BEAT_STALENESS`]'s arithmetic stays in
/// integers.
const MOCK_BPM_MILLI: u64 = 128_000;

/// One beat at that tempo, in microseconds: 468 750, which is 468.75 ms.
const BEAT_MICROS: u64 = 60 * 1_000_000 * 1_000 / MOCK_BPM_MILLI;

/// The `▾` at the end of the pill, drawn rather than typed — [`grip_dots`]'
/// reason one control along. Whether a black down-pointing small triangle is in
/// `egui`'s default face is a question with no good answer, and a triangle is
/// the same mark either way.
///
/// Half the type it sits beside wide and half of that tall, which is about what
/// the glyph's ink measures at [`size::BASE`].
const CHEVRON_W: f32 = size::BASE * 0.5;
const CHEVRON_H: f32 = CHEVRON_W * 0.5;

/// The caret, a light vertical bar. Typed rather than drawn, unlike the
/// chevron: `U+258F LEFT ONE EIGHTH BLOCK` is a box-drawing character, and
/// `egui`'s default face carries the block elements. Where it did not, the
/// fallback is a visible box in the one place an operator is looking, which is
/// louder than a caret that has quietly gone.
const CARET: char = '▏';

/// The console's view: which room it is in, and the frame's plan, kept so a
/// frame does not allocate one.
pub struct View {
    pub room: Room,
    /// Whether the projector window is open, which is the one output this crate
    /// cannot read for itself.
    ///
    /// The picture's on and off is [`Layout::visible`] on its own node and the
    /// Outputs row reads it there; a projector is a second window, a second surface
    /// and a second [`karakuri_engine::frame::Sink`], and this crate takes no
    /// device
    /// ([ADR-0156](../../../../docs/adr/0156-the-consoles-arrangement-is-a-tree-this-repository-owns.md)).
    /// So it arrives the way [`View::picture`] does: written per frame by whoever
    /// owns the window. `false` for every test in this crate, which is a console
    /// with no engine behind it.
    ///
    /// The two plugin chips have no field, and that is not an omission: there is no
    /// manifest to read them out of, so *not installed* is what they are rather
    /// than something somebody could tell this crate.
    pub projector: bool,
    /// What to draw in the Program bay's picture this frame, or `None` for a
    /// console with no engine behind it — which is every test in this crate and the
    /// whole of what `cargo test -p karakuri-console` sees.
    ///
    /// Set per frame by whoever owns the device, because that is who knows whether
    /// the texture it names is still the right size. A stale id here is a freed
    /// registration, so it is written beside the frame that made it rather than
    /// kept.
    pub picture: Option<Picture>,
    /// What to draw in each of the four deck preview cells this frame, in slot
    /// order, or `None` for a cell with no deck slot behind it.
    ///
    /// `None` is not "that deck is off air". A cell shows its slot's own material
    /// whatever the slot's residency — a parked deck's still and a warming deck's
    /// picture are the two an operator most needs to see, which is
    /// [ADR-0258](../../../../docs/adr/0258-the-look-comes-before-the-fader-so-a-cell-draws-every-slot-and-says-which-nothing-it-is.md).
    /// `None` is a cell with nothing to sample at all: a deck of fewer slots than
    /// there are cells, or a console with no engine behind it.
    ///
    /// The same seam as [`View::picture`], four times over: registering a texture
    /// takes a device, this crate has none, so whoever owns the device registers
    /// them and writes this per frame beside the frame that made them. A stale id
    /// here is a freed registration. Every test in this crate leaves every entry
    /// `None`, which is what `cargo test -p karakuri-console` sees and is a console
    /// with no engine behind it.
    ///
    /// All `None` is a state, not an absence, and the caption is where it is said:
    /// a cell with nothing behind it reads [`PREVIEW_NO_SLOT`] under its letter
    /// rather than going blank. It is deliberately not the manual's *empty* — a
    /// slot that exists with nothing loaded into it is a state the engine cannot be
    /// in — and deliberately not *off*, which was residency and has not gated a
    /// cell since ADR-0240. See [`state_word`], the module documentation, and
    /// [`preview_rects`] for where the rectangles come from.
    pub previews: [Option<Picture>; DECKS],
    /// Which of the four slots have stopped updating, in slot order, and `false`
    /// for every cell with nothing behind it — which is every test in this crate
    /// and is a console with no engine behind it.
    ///
    /// A slot is stopped when the version in it costs more than one frame may
    /// (`karakuri_engine::deck::Deck::overloaded`, ADR-0316). The engine then skips
    /// that slot's step and its draw, so its target holds the last image it made
    /// and this cell goes on showing it. The caption is where that is said —
    /// [`PREVIEW_OVERLOADED`] in place of [`PREVIEW_MATERIAL`] — because the
    /// picture cannot say it: a held frame of good material looks like material,
    /// and an unmarked still is a preview that lies
    /// ([ADR-0269](../../../../docs/adr/0269-a-slot-that-is-drawn-is-stepped-and-a-preview-runs-at-the-rooms-tempo.md)).
    ///
    /// A `bool` beside the picture rather than a third state of it. The two keep
    /// different clocks, exactly as [`View::costs`] does: a picture is a texture
    /// registration rewritten every frame by whoever owns the device, and this
    /// changes when a build lands. It is also not a residency and must not be read
    /// as one — a stopped Live slot is still in the mix.
    ///
    /// The same seam every value here crosses: `src/` takes no engine (ADR-0156),
    /// so whoever holds the deck reads it and writes this.
    pub overloaded: [bool; DECKS],
    /// What the governor budgeted each of the four slots at, in slot order, or
    /// `None` for a slot it has no number for — which is every test in this crate
    /// and is a console nobody has governed.
    ///
    /// The risk badge is read from this and from nothing else. [`band_of`] turns
    /// the number into one of five bands and [`caption_into`] draws the dot; `None`
    /// draws no dot at all, which is the manual's own state for a slot with no cost
    /// and is not a hollow one.
    ///
    /// A second field rather than a third member of [`Picture`], and the two halves
    /// of a cell keep different clocks on purpose. A picture is a texture
    /// registration and is rewritten every frame by whoever owns the device; a cost
    /// is `karakuri_engine::governor::Report`, which is taken on a governor pass
    /// and not on a frame — before the first frame, and again whenever a residency
    /// is written. Folding the cost into `Picture` would make every frame's texture
    /// aim carry a number it did not take, and the number would be dropped and
    /// re-fetched sixty times a second to no purpose.
    ///
    /// Where it comes from. One entry per `Decision` in `Report::decisions`, at
    /// `Decision::slot`: `budgeted_ms` and `Decision::basis`, with
    /// `governor::Basis::Unbudgetable` written as `None` — that is a slot nothing
    /// measured and nothing estimated, and it is *not* a zero. `src/` takes no
    /// engine (ADR-0156), so whoever holds the deck reads the report and writes
    /// this, exactly as [`View::previews`] is written by whoever holds the device.
    ///
    /// It is not gated on residency and it is not gated on the picture here. A
    /// parked deck is the one whose cost an operator most wants, because the cost
    /// is why it is parked; the gate that does exist is [`caption_into`]'s, which
    /// is the mock's *no slot is no cost*.
    pub costs: [Option<Budgeted>; DECKS],
    /// What the transport row reads this frame, or `None` for a console with no
    /// engine behind it — which is every test in this crate, and what the row draws
    /// then is nothing at all.
    ///
    /// The same seam as [`View::picture`], one row up and without a device: the
    /// tempo, the beat and the frame's cost are a clock and an engine, and `src/`
    /// has neither (ADR-0156). So whoever owns them reads them and writes this per
    /// frame, beside the frame that measured it. See [`Transport`] and
    /// [`transport`].
    pub transport: Option<Transport>,
    /// What the arrangement pill in that row reads, and what its menu is doing.
    ///
    /// Half of it is the same seam as [`View::transport`] and half of it is not,
    /// which is the one field here that is both. The name in use and the names
    /// filed are a file and a directory, so whoever owns the store reads them and
    /// writes them here — [`View::library`]'s seam, one row up. The menu is this
    /// crate's own and moves only through [`Arrangement`]'s methods: what the
    /// *control* is doing is not something the program can be the model of record
    /// for.
    ///
    /// [`Arrangement::NONE`] until somebody says otherwise, which is every test in
    /// this crate and is a console with no store behind it: the default
    /// arrangement, nothing filed, and the menu shut. See [`Arrangement`] and
    /// [`arrangement`].
    pub arrangement: Arrangement,
    /// What the audio-in pill in that row reads, and whether its card is down — or
    /// `None` for a console nobody has told anything about audio, which is every
    /// test in this crate and draws no pill at all.
    ///
    /// The same two halves [`View::arrangement`] has, over a device instead of a
    /// file: which input is open and what inputs there are are a microphone and an
    /// enumeration of a host, and `src/` has neither (ADR-0156) — so whoever opened
    /// one writes them here. Whether the card is down is this crate's and moves
    /// only through [`AudioIn`]'s methods.
    ///
    /// `Option`, where the arrangement is not, and the difference is real: every
    /// console has an arrangement — the default one, which is what is on screen —
    /// and a console has an *input* only if somebody opened one and said so.
    /// `Some(AudioIn::NONE)` is a program that looked and found nothing, and it
    /// draws `audio-in · none`; `None` is a program that never said, and a pill
    /// drawn for it would be this crate answering a question about a device on its
    /// own authority. See [`audio_in`].
    pub audio: Option<AudioIn>,
    /// Whether MIDI learn is armed — the transport row's `learn` pill, lit while
    /// this is true.
    ///
    /// This crate's own state, and the only console state on this row that is
    /// ([`AudioIn`]'s card is the other). Arming is a mode and it is the one thing
    /// rule 04 lets be a mode, because the pill *is* the readout: it is lit for
    /// exactly as long as the mode is on, and nothing else on the panel changes
    /// meaning while it is
    /// ([ADR-0336](../../../../docs/adr/0336-a-learn-is-a-map-edit-and-the-tips-midi-line-is-the-live-map.md)).
    ///
    /// A `bool` and not an `Option`, where [`View::map`] beside it is an `Option`,
    /// and the difference is the same one `audio` draws: whether a mode is armed is
    /// a fact this crate owns outright, and *which map is loaded* is a fact about a
    /// file this crate cannot see.
    ///
    /// What it does not do is bind anything. The gesture is the host's — what the
    /// pointer is on, what arrived on the wire, and what goes in the file are all
    /// outside this crate (ADR-0156). This is the arming and the lamp.
    pub learn: bool,
    /// Which map is loaded, for the transport row's `map` pill, or `None` for a
    /// console nobody has told — which is every test in this crate and every run
    /// with no surface.
    ///
    /// It is [`View::audio`]'s shape one pill along and for its reason: a map is a
    /// *file*, `src/` reads none (ADR-0156), so whoever loaded one writes its name
    /// here. `Some(MapPill::NONE)` is a program with a surface and no map, which
    /// draws `map · none`; `None` draws no pill at all.
    pub map: Option<MapPill>,
    /// What the other three controls in the tracker group read this frame — the
    /// latency offset the open session is holding, and which way the grid can still
    /// be moved an octave — or `None` for a console nobody has told anything about
    /// the beat tracker, which is every test in this crate that does not say
    /// otherwise and draws none of the three.
    ///
    /// The same seam as [`View::audio`], over the beat lock instead of over the
    /// device it listens through: the range the tracker searches is
    /// `karakuri-audio`'s and the offset is a value
    /// `karakuri_environment::audio::Audio` holds, and `src/` has neither
    /// (ADR-0156) — so whoever opened one writes them here per frame.
    ///
    /// A field of its own rather than three more on [`View::audio`], which is
    /// [`View::look`]'s argument beside [`View::transport`]: the pill is *which
    /// room is being heard*, and this is *what the tracker is doing with it*. The
    /// two are `Some` together in every program that draws this row, and a type
    /// that could only say them together would be answering one question with two.
    /// See [`Tracker`] and [`tracker_group`].
    pub tracker: Option<Tracker>,
    /// What the two look controls in that row read this frame, or `None` for a
    /// console with no engine behind it — which is every test in this crate that
    /// does not hand one in, and what the row draws there is nothing at all.
    ///
    /// The same seam as [`View::transport`], two items along the same row: the
    /// operator and the level are `karakuri_engine::frame::Look`, `src/` has no
    /// engine (ADR-0156), so whoever owns one reads it and writes this per frame
    /// beside the frame it was drawn under.
    ///
    /// It is a second field rather than two more fields on [`Transport`] because it
    /// is a different reading of a different thing: a tempo and a frame cost are
    /// what the *session* is doing, and a look is what the picture is being put
    /// through. `transport` answering `None` and this answering `None` are two
    /// facts, and a console driving one and not the other is a state the type
    /// should be able to say. See [`Look`] and [`look`].
    pub look: Option<Look>,
    /// What the Master bay's out row reads this frame, or `None` for a console with
    /// no engine behind it — which is every test in this crate that does not hand
    /// one in, and what the bay draws then is its card and its head.
    ///
    /// The same seam as [`View::look`], one bay away and one level along the same
    /// chain: this is `karakuri_engine::deck::Deck::out`, which is applied where
    /// the mix *writes* the composited frame, and the look is applied where the
    /// present pass *reads* it. `src/` has no engine (ADR-0156), so whoever owns
    /// one reads it and writes this per frame beside the frame it was drawn under.
    ///
    /// A bare level rather than a struct, and it stays one now that the chain
    /// exists: this is the level at the chain's *entry*, and what the chain is set
    /// to is [`View::master_chain`] beside it. The two are two fields for the
    /// reason they are two records — one is a level a fader rides and one is a set
    /// of settings a press moves
    /// ([ADR-0224](../../../../docs/adr/0224-out-and-exposure-are-two-levels-that-multiply-in-different-places.md),
    /// [ADR-0317](../../../../docs/adr/0317-the-master-chain-is-three-fixed-passes-and-feedback-reads-either-cut.md)).
    pub master_out: Option<f32>,
    /// What the Master bay's three effect rows read this frame, or `None` for a
    /// console with no engine behind it — in which case the bay draws the out row
    /// and nothing under it.
    ///
    /// [`View::master_out`]'s seam exactly, one row down:
    /// `karakuri_engine::present::Present::chain` is what a harness reads it from,
    /// and [`Chain`] is that value mirrored into a crate with no engine in it.
    pub master_chain: Option<Chain>,
    /// What each mixer strip reads this frame, one per slot the deck has, in slot
    /// order — and empty for a console with no deck behind it, which is every test
    /// in this crate and what the bay draws then is nothing at all.
    ///
    /// The same seam as [`View::transport`], and empty rather than `Option<Vec<_>>`
    /// because an empty list of strips is already the whole of *no deck*: a deck
    /// has one to four slots (`MAX_SLOTS`, asserted in `Deck::new`), so there is no
    /// deck that has none and no second way to say it.
    ///
    /// A `Vec` a caller keeps and rewrites, rather than a fixed array of `Option`s
    /// like [`View::previews`]: a preview cell is off or on and the row is always
    /// four, where the strips are *as many as the deck has* and a `None` in the
    /// middle of them would be a slot no `Deck` can have. [`View::new`] gives it
    /// room for [`DECKS`] so the frame path never grows it. See [`Strip`] and
    /// [`mixer`].
    pub mixer: Vec<Strip>,
    /// What the Library bay lists this frame: the name of every Set the store
    /// holds, in the order the store listed them — and empty for a console with no
    /// store behind it, which is every test in this crate and what the bay draws
    /// then is nothing at all.
    ///
    /// The same seam as [`View::mixer`], and empty rather than `Option<Vec<_>>` for
    /// the same reason: an empty listing is already the whole of *no library*, and
    /// a store that holds nothing and a store that is not there are the same bay —
    /// one with no row to draw.
    ///
    /// Read once rather than per frame, by whoever owns the store. A listing is a
    /// directory read, which is not a thing to do on a frame path (P-0091), and
    /// nothing in this crate can do it anyway: opening a store is
    /// `karakuri-store`'s and `src/` depends on neither it nor the engine
    /// (ADR-0156). What crosses the seam is a list of names.
    ///
    /// A name and nothing else, because a name is what exists: see [`library`] for
    /// the star and the time the mock draws beside it, and for why neither is here.
    ///
    /// It is the listing of whichever scope is marked, and not of the store: the
    /// bay lists `my sets` where that chip is marked and the presets root's `.kset`
    /// files where that one is, and both are a directory read the host does on the
    /// press that changed the scope. See [`View::scopes`] and [`Scope`].
    pub library: Vec<String>,
    /// What each row of [`View::library`] is, in the same order — a Set or a
    /// procedure, and the layers its badge names.
    ///
    /// Empty is every row a Set with no badge, which is what this bay drew before
    /// ADR-0338 and is every test in this crate that does not say otherwise. So a
    /// host that has not been changed to answer it draws exactly what it drew, and
    /// a host that has hands the two halves over on one press — see [`RowKind`],
    /// where that default is argued, and [`View::rows`], which is what every
    /// control here reads.
    ///
    /// The same seam as [`View::library`]: a Set's layers are its file's own `slot`
    /// records and a procedure's kind is the `kind` line of a source, both of which
    /// are a directory read on the press that builds a listing — and this crate
    /// reads no store (ADR-0156, P-0091).
    ///
    /// Shorter than the listing is not an error, for [`View::starred`]'s reason
    /// read the other way: a row past the end of it is a Set with no badges, which
    /// is a row this console can draw and act on.
    pub kinds: Vec<RowKind>,
    /// Which Sets this store has starred, as their ids — and empty for a console
    /// with no store behind it, which is every test in this crate that does not say
    /// otherwise and is a listing whose every row draws a hollow star.
    ///
    /// The same seam as [`View::library`], in the same rows: the marks are
    /// `<store>/favourites.json` beside the Sets (ADR-0299), and this crate reads
    /// no store (ADR-0156). So the host reads them on the same press the listing is
    /// built on and hands over the set of ids.
    ///
    /// A `BTreeSet` and not a `Vec<bool>` beside the listing. The question a row
    /// asks is *is this one starred*, once per row, and two vectors of the same
    /// length are two vectors that can disagree about it — where a set of ids
    /// answers the same question about a listing that was rewritten under it
    /// without being wrong, and is what `karakuri_store::Store::favourites` already
    /// hands back.
    ///
    /// Ids the listing does not hold are not an error here. `my sets` is the
    /// intersection of these with what the store holds and the host is what takes
    /// it; a mark left behind by a file somebody deleted draws no row and is not
    /// this field's to prune.
    pub starred: std::collections::BTreeSet<String>,
    /// Which Set the load pulldown's deck is running, as the host reads it off that
    /// deck's aim — and `None` for a deck playing the pair the run was launched
    /// with, which is where every run starts.
    ///
    /// It is here because a walk names a Set and this console cannot spell one.
    /// [`Operation::WalkHistory`] carries the id of the history it is a walk of;
    /// the id rides the aim a library load sends and this crate reads no engine
    /// (ADR-0156), so the host answers it here, exactly as it answers the rows of
    /// that walk into [`View::library`] — see [`Chosen::asked`], which is the one
    /// place this is read.
    ///
    /// A readout rebuilt per frame, like [`View::mixer`] beside it, rather than
    /// written on the press that changes the aim: the pulldown's deck can be
    /// re-pointed by a load, by a key, by a mapped control and by a model, and a
    /// value written at one of those four is a value stale after the other three.
    ///
    /// The deck it is read off is the load pulldown's and not the selection's
    /// ([`View::target_deck`], ADR-0305): the walk is drawn under the pulldown that
    /// says which deck the bay is preparing, and the landing it feeds lands there.
    pub aimed: Option<String>,
    /// Which libraries this console has to offer, in the order the chips are drawn
    /// — and empty for a console nobody has told, which is every test in this crate
    /// that does not say otherwise and what the bay then draws is no scope row at
    /// all.
    ///
    /// The same seam as [`View::library`], one row up in the same bay: what a scope
    /// can be asked is a store, a told directory and a directory somebody names
    /// during the run, and this crate has none of the three (ADR-0156). So the host
    /// says which there are and answers the marked one into [`View::library`].
    ///
    /// Empty rather than [`Scope::ALL`] by default, which is [`View::mixer`]'s rule
    /// and not a shortage: a console that has been told nothing has no libraries
    /// rather than four it cannot answer, and a default here would be this crate
    /// asserting that a presets root and a folder exist on a machine it cannot look
    /// at.
    pub scopes: Vec<Scope>,
    /// What the `holds` field can be stepped to, in the order it steps them — and
    /// empty for a console nobody has told, which is every test in this crate that
    /// does not say otherwise and is a field a press asks the listing again
    /// through.
    ///
    /// The same seam as [`View::scopes`], two rows up in the same bay: `holds` is
    /// matched against what a Set's nodes are called, which is
    /// `karakuri_environment::setfile::summarise`'s reading of the store, and this
    /// crate reads no store (ADR-0156). So the host says what there is to narrow by
    /// and the console steps through it.
    ///
    /// Written on the same read as [`View::library`] and off the same summary,
    /// because the two are one directory read: the rows are the Sets that matched
    /// and these are the names any of them could be matched by. They are the
    /// candidates of the *unnarrowed* listing, so the row a press steps to does not
    /// depend on what the field is already set to — candidates read off a filtered
    /// listing would shrink as the filter bit, and a step would then wander
    /// somewhere it could not come back from.
    pub holds: Vec<String>,
    /// Which directory this library is pointed at, as the host spells it — and
    /// `None` until a folder has been dropped on this window, which is where every
    /// run starts and is a bay with no `.path` row at all.
    ///
    /// The same seam as [`View::library`], one row up in the same bay: a directory
    /// is a thing on a disk, this crate reaches no disk (ADR-0156), and what
    /// crosses is the line to draw. The host writes it on the drop that chose it
    /// and never on a frame — a drop is one act, and asking the file system what a
    /// path is is not a thing to do per frame (P-0091).
    ///
    /// It is not a scope and it is not `Scope::Folder`. The row is drawn whichever
    /// chip is marked, because it is also where a send's save dialog opens
    /// (ADR-0311, superseding ADR-0267's *where a send lands*); what the folder
    /// scope's *listing* is arrives in [`View::library`] like every other scope's.
    ///
    /// It is a `String` rather than a `PathBuf` for the same reason the listing is:
    /// this crate never opens it, so what it needs is the spelling. See
    /// [`Pointed`].
    pub folder: Option<String>,
    /// The path a release would set, while a folder is over the window — and `None`
    /// whenever no drag is over it, which is nearly always.
    ///
    /// Beside [`View::folder`] rather than inside it, because the two are different
    /// facts: one is where this bay *is* pointed and the other is where it *would
    /// be*. A drag that leaves the window without being let go clears this and
    /// leaves the other standing, which is the row going back to what it said — see
    /// [`View::pointed`], where the two become the one row the mock draws.
    ///
    /// Written per frame by whoever reads the platform's hover, which is the one
    /// thing about this bay that is a frame's business: `egui` clones the hovered
    /// files onto every pass while a drag is over the window, so there is no event
    /// to hang it off. Nothing is asked of the file system for it (P-0091) —
    /// whether the path is a folder is the drop's question and not the hover's, and
    /// the row says nothing about whether the release will be allowed (ADR-0275).
    pub incoming: Option<String>,
    /// What the Staging lane lists this frame: one candidate per deck slot whose
    /// newest build has a verdict outstanding or whose file no longer agrees with
    /// its picture — and empty for a console with no engine behind it, which is
    /// every test in this crate that does not hand one in and what the bay draws
    /// then is its card and its head.
    ///
    /// The same seam as [`View::mixer`], and empty rather than `Option<Vec<_>>` for
    /// the same reason: an empty lane is already the whole of *nothing waiting*,
    /// and a run in which nobody has rewritten a procedure and a run with no
    /// producer at all are one bay — one with no row to draw. Empty is this lane's
    /// ordinary state, which is what makes it different from every other bay here:
    /// a library with nothing in it is a library nobody has filled.
    ///
    /// Written when an event arrives rather than per frame, by whoever drains
    /// `karakuri_engine::deck::Deck::events` — which is a `Vec` the engine only
    /// appends to when a build lands, is refused or is judged, and which a caller
    /// that never drains grows for the rest of the run. So a frame on which nothing
    /// was swapped touches nothing here, and the name a row carries is rewritten
    /// only when the row's own build changes. See [`Candidate`] and [`staging`],
    /// which is also where the four things the mock's row has and this does not are
    /// named.
    pub staging: Vec<Candidate>,
    /// What each Inspector pane is showing this frame, one per pane the console has
    /// room to point at something — and empty for a console with no deck behind it,
    /// which is every test in this crate and what the bay draws then is its card,
    /// its head and the bar between its panes.
    ///
    /// The same seam as [`View::mixer`], and empty rather than `[Option<Pane>;
    /// PANES]` for the same reason: an empty list is already the whole of *no
    /// deck*, and a pane pointed at nothing and a pane that is not there are one
    /// bay — one with no head to draw.
    ///
    /// A pane is *pointed*, not choosing. The mock's `showing … ▾` is a control and
    /// this pass adds none (ADR-0200), so which deck each pane shows is whoever
    /// fills this saying so — and pane *i* draws `inspector[i]`, in [`PANE_NAMES`]'
    /// order.
    ///
    /// Written when a Set lands rather than per frame, which is
    /// `karakuri_engine::set::Set::published`'s own instruction — *"Allocates, so
    /// not the frame path. A console reads this when a Set lands, not per frame."*
    /// — and is [`View::staging`]'s rule read one bay along: the two are written on
    /// the same frames and off the same drain, because a candidate row and a pane's
    /// rows are two readings of one event. Between landings what is in here does
    /// not move: nothing in this workspace writes a published value, binds a signal
    /// in the panel binary, or grants an authority (ADR-0216), so a Set that has
    /// landed reads the same on every frame after it until the next one does. The
    /// field is rewritable per frame like every other one; what the `man / sug /
    /// auto` chip still waits on is a writer for the authority, which is a
    /// different gap. See [`Pane`] and [`inspector`].
    pub inspector: Vec<Pane>,
    /// The shape of what is being rendered, which is what the Program bay arranges
    /// its body for — [`program_bay`], and [`picture_rect`] for why it is two
    /// numbers rather than a ratio.
    ///
    /// The same seam as [`View::picture`], and it belongs beside it: the picture's
    /// rectangle is handed in by whoever built the `Present`, and this is the
    /// number that rectangle was derived from. Written per frame by that same
    /// caller and in the same breath, so that the canvas the texture was sized for
    /// and the canvas the bay arranged itself for cannot be two different numbers —
    /// which would put the cells in one arrangement and the picture in the other.
    ///
    /// [`MOCK_CANVAS`] until somebody says otherwise, which is every test in this
    /// crate and is a console with no engine behind it: there is no picture to
    /// draw, and the four cells still have to go somewhere.
    pub canvas: (u32, u32),
    /// Which classes the operator has opened to a model, written per frame by
    /// whoever holds the run's opening.
    ///
    /// Handed in like every other value here, and for the sharper version of the
    /// usual reason: the model of record is a `karakuri_environment::Opening`,
    /// which is a handle a *second* surface reads on every MCP call — so a copy
    /// kept in this crate would be the console answering on the server's behalf,
    /// and would go on saying *open* after something else shut it. This crate names
    /// [`karakuri_operation::gate::Open`] and nothing in `karakuri-environment` at
    /// all (ADR-0156); [`McpPill::next`] hands a value back and whoever owns the
    /// handle writes it.
    ///
    /// [`Open::CLOSED`] until somebody says otherwise, which is every test in this
    /// crate and is the state ADR-0235 says a run starts in: four classes shut, and
    /// no way to write down an `Open` that starts open.
    pub opening: Open,
    /// How long the panel has been animating, written per frame by whoever has the
    /// clock — see [`Phase`], which carries the whole argument for why this is a
    /// value and not an `Instant`.
    ///
    /// The same seam as [`View::transport`], and the one this crate is least able
    /// to cross: a clock in `src/` is P-0092 broken in the file whose own doc says
    /// so. [`Phase::ZERO`] until somebody says otherwise, which is every test in
    /// this crate and is a console with no clock behind it — a panel drawn at the
    /// origin of every animation on it.
    pub phase: Phase,
    /// Which bay the keyboard is talking to, and what each bay remembers — the
    /// pointer this console owns, and the one three of its fields used to be
    /// ([`crate::focus`]).
    ///
    /// The deck selection, the library cursor and the marked scope are three
    /// readings of this, on two bays: the Mixer's remembered item is the deck the
    /// keys are addressed to, the Library's is the row under the cursor, and the
    /// control last named under the Library's head is the scope. Each of the three
    /// was a private field with the same paragraph written at it — *"nothing
    /// downstream can be the model of record for it, and a host that kept a copy
    /// would be keeping the console's state on its behalf"* — and that paragraph is
    /// written once now
    /// ([ADR-0259](../../../../docs/adr/0259-the-keyboard-is-addressed-to-the-bay-that-has-focus-and-a-global-letter-is-a-convenience-or-the-operators-own.md),
    /// [ADR-0332](../../../../docs/adr/0332-focus-is-a-pointer-the-console-owns-and-the-three-pointers-are-instances-of-it.md)).
    /// Nothing about what any of the three means changes, which is the record's own
    /// clause: what each of them refuses is still refused where it was refused, by
    /// [`View::select`], [`View::walk`] and [`View::select_scope`].
    ///
    /// Private, with [`View::select`], [`View::walk`], [`View::point_at`],
    /// [`View::select_scope`], [`View::step_scope`], [`View::tab`] and
    /// [`View::focus_up`] the only ways in, which is [`View::arrangement`]'s rule:
    /// what a *pointer* is at is not something the program can be told, because the
    /// console is what refuses a deck there is no strip for.
    ///
    /// Read by [`mixer::mixer_into`] for the solid ring, by [`View::focus_mark`]
    /// for the dashed one, and by nothing else on the panel. The Library bay's foot
    /// read the selection for its letter until 2026-09-08 and reads
    /// [`View::target`] now (ADR-0305), which is what lets a load be aimed at a
    /// deck the keys are not addressed to; `console.html`'s crossfader read it as
    /// well, and there is no crossfader. What still fills its `deck` in from there
    /// is every deck-addressed *key*, `l` included.
    focus: Focus,
    /// Which deck the Library bay's load is aimed at, and the third pointer this
    /// console owns.
    ///
    /// It is not [`View::selection`] and that is the point. The selection is what a
    /// *key* press is addressed to; this is what the *button* in the Library bay's
    /// foot lands on, and the two are free to name two different decks — which is
    /// the one thing the selection cannot do, and the whole of what the pulldown
    /// bought
    /// ([ADR-0305](../../../../docs/adr/0305-the-library-bays-load-is-a-button-and-a-pulldown-and-the-deck-it-names-is-not-the-selection.md)).
    /// `l` goes on reading the selection.
    ///
    /// It writes no record and no operation names it, which is
    /// [`View::cursor_row`]'s argument one mark along: picking a deck in the
    /// pulldown changes what the *next* press will ask for and nothing about what
    /// any deck is playing, so nothing downstream can be the model of record for it
    /// and a host that kept a copy would be keeping the console's state on its
    /// behalf. `Operation::SelectDeck` is emphatically not what a pick emits: that
    /// operation moves the selection, and this one must not.
    ///
    /// Private, with [`View::aim_at`] the only way in, which is
    /// [`View::selection`]'s rule and for its reason: the console is what refuses a
    /// deck there is no strip for.
    ///
    /// Zero until somebody says otherwise — deck A, which is the letter the mock's
    /// pulldown reads. Kept across everything that is not a deck: changing the
    /// scope, narrowing the listing and walking the cursor all leave it alone,
    /// because none of them is about a deck.
    target: u8,
    /// Whether the pulldown's list is down, and it is the console's own state
    /// rather than a reading — [`Arrangement::menu`]'s argument on a third control:
    /// what a *control* is doing is not something the program can be the model of
    /// record for.
    ///
    /// Private, with [`View::open_target`] and [`View::shut_target`] the only ways
    /// in. `open_target` refuses a console the mixer is drawing no strip for: a
    /// card with no rows in it is a gesture with nothing to pick and nothing to
    /// leave by, and `input::claim`'s rule 2 would give it every press on the
    /// console until a second one shut it.
    target_open: bool,
    /// Which `uses` line's card is down, as `(pane, node, input)` — or `None` with
    /// none of them open, which is where every run begins.
    ///
    /// The console's own state, exactly as [`target_open`] beside it is. A card
    /// being down changes what the *next press* reaches and nothing about any deck,
    /// so no operation names it: opening a list is not something a map or a model
    /// could ever want to say, which is ADR-0305's argument for the pulldown one
    /// bay along and is this one's unchanged.
    ///
    /// One at a time, and one for the whole console. Two cards down at once would
    /// put two rectangles over the same pane with `input::claim`'s rule 2 giving
    /// each of them every press; and a card belongs to a pane, a node and an input
    /// together, which is why the three travel as one value rather than as three
    /// fields that can disagree.
    ///
    /// [`target_open`]: Self::target_open
    wiring_open: Option<(usize, usize, usize)>,
    /// Whether the `+ lane` chooser's card is down, and it is the field above one
    /// bay along — the console's own state rather than a reading, on
    /// [`Arrangement::menu`]'s argument: what a *control* is doing is not something
    /// the program can be the model of record for.
    ///
    /// Private, with [`View::open_lane`] and [`View::shut_lane`] the only ways in,
    /// and `open_lane` refuses a chooser with nothing in it for `open_target`'s
    /// reason: `input::claim`'s rule 2 would give an empty card every press on the
    /// console until a second one shut it.
    lane_open: bool,
    /// Which row of the Library bay's list has its menu down, or `None` for none —
    /// the console's own state, exactly as [`target_open`] beside it is, and not a
    /// fourth mark: a menu is a card that is there or is not, and the row under it
    /// draws the same either way.
    ///
    /// One field where the pulldown takes two, and that is the whole of the
    /// difference between the two cards: a pulldown is a capsule that is drawn
    /// whether or not its list is down, so *which deck* outlives *is it open*; a
    /// row menu is the card and nothing else, so an open menu with no row under it
    /// cannot be spelled here.
    ///
    /// Private, with [`View::open_menu`] and [`View::shut_menu`] the only ways in.
    /// `open_menu` does not refuse a console the mixer is drawing no strip for,
    /// where `open_target` does: this card carries `Save as a kbset` under the
    /// separator whatever the mixer is doing, so there is always something to pick
    /// and something to leave by.
    ///
    /// [`target_open`]: Self::target_open
    menu_row: Option<usize>,
    /// How far down its listing the Library bay is scrolled, in logical pixels, as
    /// it is stored — the number [`library`] clamps and never the one it clamped.
    ///
    /// [`View::scroll`]'s shape one bay over and for its reasons, which are worth
    /// reading as a pair rather than restated here: the position is the console's
    /// own, no operation names it, nothing outside this crate could be the model of
    /// record for it, and it is not part of the arrangement — `karakuri-layout`'s
    /// tree holds sizes and folds and holds no scroll position
    /// ([ADR-0307](../../../../docs/adr/0307-the-inspectors-pane-scrolls-and-the-position-is-the-panes-own.md),
    /// [ADR-0312](../../../../docs/adr/0312-the-params-pill-is-a-toggle-and-the-library-bay-scrolls.md)).
    ///
    /// One and not one per scope. A scope is a different listing in the same bay,
    /// the way a build landing is a different pane's contents one bay over — and
    /// unlike the Inspector's two panes, which are two places in one list at once,
    /// only one scope is ever being read. A position per scope would be five
    /// numbers of which four are always stale, and the chip press that changes the
    /// listing is exactly the moment an operator wants the top.
    ///
    /// Private, with [`View::library_scroll`] and [`View::scroll_library_by`] the
    /// only ways in.
    library_scroll: f32,
    /// What the Set under the cursor declares, opened — or `None` for a bay with
    /// nothing open, which is where every run starts.
    ///
    /// The reading is the host's and the opening is this console's, which is the
    /// seam every other value in this bay crosses: what a Set declares is read off
    /// the cards the store keeps (ADR-0156, and P-0091 — it is a file read, done on
    /// the press and never on a frame), and *whether the block is down* is the
    /// console's own state, exactly as which row the cursor is on is. So a host
    /// answers [`Operation::ReadSet`] by writing this, and puts the reading away by
    /// clearing it.
    ///
    /// One row is open at a time, which is the mock's own rule and is what keeps
    /// this a mode of the list rather than a second list — so this is one reading
    /// and not a set of them. It carries the id it is of, and [`View::opened`]
    /// draws it only under the row of that name: a listing rewritten under an open
    /// reading closes it rather than filing it under whatever has taken that
    /// position.
    ///
    /// Private, with [`View::read`] and [`View::shut_reading`] the only ways in,
    /// which is [`View::selection`]'s rule one pointer up.
    reading: Option<Reading>,
    /// Which of [`View::holds`] the `holds` field is set to, or `None` for a field
    /// nobody has set — the fifth of this console's pointers.
    ///
    /// A position and not a `String`, for [`View::scope`]'s reason one field along:
    /// what the field can be set to is the row of candidates the host handed in, so
    /// what a pointer into it can be is a place in that row.
    ///
    /// A stale position reads as unset rather than as the last candidate, where
    /// [`View::scope`] and [`View::cursor_row`] both clamp. The two of them point
    /// at something drawn — a chip, a row — and the nearest one is the right
    /// answer; this one *narrows a listing*, and clamping it would leave the bay
    /// hiding Sets under a filter nobody chose.
    holds_at: Option<usize>,
    /// Which kinds of row the listing is showing, which is the six chips under the
    /// `holds` field — [`LibraryKinds::EVERYTHING`] where none of them is on, which
    /// is where every run starts.
    ///
    /// A value and not a position, where [`View::holds_at`] above it is a position,
    /// and it is [`View::transition`]'s distinction below: the kinds are the
    /// console's own closed list ([`KindChip::ALL`]) and cannot change under this
    /// pointer, where the `holds` candidates are a reading of a store that can.
    ///
    /// It was `layer: Option<Layer>` until 2026-09-10, the `layer…` field's value,
    /// and ADR-0338 replaced that field with these six toggles: a cycle names six
    /// of the sixty-four states this row has, and the two controls ask different
    /// questions — *which Sets hold a node on this layer*, which
    /// `Operation::ListSets` still carries, against *which kinds of row is this
    /// listing showing*.
    ///
    /// `showing` and not `kinds`, because [`View::kinds`] is already the row of
    /// what each *listed row* is: one field says what the bay is being asked for
    /// and the other says what came back, and two fields called `kinds` would be a
    /// name meaning two things.
    showing: LibraryKinds,
    /// Which pane head is taking letters, and what has been typed into it — or
    /// `None` for a console where nothing is being named, which is where every run
    /// starts.
    ///
    /// The console's own state and not a reading, which is [`Arrangement::menu`]'s
    /// argument on a second control: what a *control* is doing is this crate's,
    /// nothing about a half-typed name is saved, restored or reset, and a host that
    /// kept a copy would be keeping the console's gesture on its behalf.
    ///
    /// It cannot live in [`View::inspector`], which is the reason it is a field
    /// here at all: a pane is rewritten whenever a Set lands, so a buffer kept in
    /// one would be a name that vanished mid-word.
    ///
    /// Private, with [`View::name_set`] and the four methods beside it the only
    /// ways in — [`View::selection`]'s rule, and see [`Naming`] for why there is
    /// one of these and not one per pane.
    naming: Option<Naming>,
    /// How far each Inspector pane is scrolled, one position per pane, and the
    /// console's eighth pointer.
    ///
    /// A pointer and not a reading, which is [`View::cursor_row`]'s argument
    /// arriving at a second bay: no operation names it, nothing downstream could be
    /// the model of record for it, and a host that kept a copy would be keeping the
    /// console's state on its behalf. It is not part of the arrangement either —
    /// `karakuri-layout`'s tree holds sizes and folds, which is what a saved
    /// arrangement carries — so a save does not take it and a restore does not move
    /// it
    /// ([ADR-0307](../../../../docs/adr/0307-the-inspectors-pane-scrolls-and-the-position-is-the-panes-own.md)).
    ///
    /// It cannot live in [`View::inspector`], which is [`View::naming`]'s reason
    /// one field up: a pane is rewritten whenever a Set lands, so a position kept
    /// in one would go back to the top every time a build arrived — and a knob
    /// under an operator's hand would leave the screen on a frame nobody touched.
    ///
    /// One per pane and not one per deck. Two panes pointed at one deck are two
    /// places in one list, because what is scrolled is the pane; a position filed
    /// under the deck would move the pane an operator is not looking at.
    ///
    /// Stored unclamped against the *pane*, clamped at the draw. See
    /// [`InspectorPane::scroll`] and [`View::scroll_by`], which are the two halves
    /// of that: the pane's height is a viewport and P-0082 is why a solve may not
    /// write through it.
    ///
    /// Private, with [`View::scroll_by`] and [`View::scroll_in`] the only ways in,
    /// which is [`View::selection`]'s rule. Zero until somebody turns a wheel,
    /// which is every test in this crate and is a pane at the top of what its deck
    /// holds.
    scroll: [f32; PANES],
    /// Which deck each Inspector pane is pointed at, one per pane, and the
    /// console's ninth pointer — the pulldown on the pane head (`docs/adr/0338-…`,
    /// decision 5).
    ///
    /// Beside [`View::scroll`] because it is the same kind of thing: a pane's own
    /// answer, two panes are two of them, and neither is the console's. It is not
    /// [`View::selection`] — that is *the* deck, one value, what a key press is
    /// addressed to — and it is not [`View::target`] either, which is the Library
    /// bay's load mark. Three pointers naming a deck, and the whole reason there
    /// are three is that each answers a different question.
    ///
    /// It cannot live in [`View::inspector`], which is [`View::naming`]'s and
    /// [`View::scroll`]'s reason: a pane is rewritten whenever a Set lands, so a
    /// target kept in one would be the host's answer read back as the console's
    /// question. [`Pane::deck`] is what the host *filled from this*, and the two
    /// are a pointer and a reading rather than two copies.
    ///
    /// Private, with [`View::point_pane`] the only way in — [`View::selection`]'s
    /// rule, and that method is what refuses a deck the mixer draws no strip for.
    ///
    /// Deck A and deck B, which is where a run opens and is the mock's own two
    /// heads.
    pane_deck: [u8; PANES],
    /// Which pane head's pulldown is down, or `None` — the console's own state,
    /// like [`View::target_open`] and [`Arrangement::menu`].
    ///
    /// One `Option` and not one flag per pane, for [`Naming`]'s reason read on a
    /// card: `input::claim`'s rule 2 hands every press to a card that is down, so
    /// two down at once would be two hands mid-choice with nothing on the panel
    /// saying which one the next press belongs to.
    pane_open: Option<usize>,
    /// What the next fade, crossfade or wipe means — the wipe's front shape and
    /// angle, the grid it starts on and how long it lasts — and the fourth of this
    /// console's pointers.
    ///
    /// A pointer and not a reading, which is [`View::selection`]'s argument
    /// arriving at a fourth control: [`Operation::SetTransition`] is
    /// `Silent(Surface)` — *"these change nothing you can see and write nothing to
    /// the stream"* — so nothing downstream can be the model of record for it, and
    /// a host that kept a copy would be keeping the console's state on its behalf.
    /// The host reads it to convert the next [`Operation::Wipe`], which is what
    /// `karakuri_operation_record::Current::transition` is asking for.
    ///
    /// Four values and not a position in the three cycles, where
    /// [`View::cursor_row`] and [`View::scope`] are both positions. Those point
    /// into a *listing this console was handed*, which can change under them; these
    /// three cycles are the console's own and cannot, and what the host has to be
    /// told is the shape and the two beat counts rather than where they sit in a
    /// table it cannot see.
    ///
    /// Private, with [`View::set_transition`] the only way in, which is
    /// [`View::selection`]'s rule: the console is what refuses a setting no pill
    /// can draw.
    ///
    /// [`TransitionSettings::START`] until somebody says otherwise, which is every
    /// test in this crate and is where a run begins.
    transition: TransitionSettings,
    /// What the Sequencer bay reads this frame, or `None` for a console with no
    /// sequencer behind it — which is every test in this crate that does not hand
    /// one in, and what the bay draws then is nothing at all.
    ///
    /// The same seam as [`View::mixer`]: a pattern is authored state a *session*
    /// holds and `src/` holds no session (ADR-0156), so whoever owns one writes
    /// this per frame beside the frame it is about. It is not this console's own
    /// pointer — a press here hands back an [`Operation`] and applies nothing,
    /// exactly as a fader's drag does.
    ///
    /// Why the reading is a whole pattern: see [`Sequenced`], which is where the
    /// alternative — a field per drawn thing — is refused.
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

    /// Which deck the keys are addressed to — the Mixer bay's remembered address,
    /// read as a deck.
    ///
    /// [`Operation::SelectDeck`] *"writes no record, and is the reason every other
    /// variant names its deck instead of meaning the selected one"* — so nothing
    /// downstream can be the model of record for it, and a host that kept a copy
    /// would be keeping the console's state on its behalf.
    /// [ADR-0219](../../../../docs/adr/0219-the-crossfader-spans-the-selection-and-the-one-after-it.md)
    /// recorded it as living *"in the specification and not in `karakuri-console`'s
    /// code"*; [`View::focus`] the field is where that stopped being true, and
    /// ADR-0259 is why it is the same field the library cursor and the marked scope
    /// are in.
    ///
    /// Deck A for a bay nobody has addressed, which is the strip the mock rings.
    /// The digit that names the first strip is `1` — digits count what the bay drew
    /// — and this is the one place the step down to a deck index is written.
    ///
    /// [`View::focus`]: Self::focus
    pub fn selection(&self) -> u8 {
        self.focus
            .address(focus::MIXER)
            .and_then(|address| address.remembered(&[]))
            .map_or(0, |nth| nth.saturating_sub(1) as u8)
    }

    /// Address the keys to `deck`, and answer whether that moved anything.
    ///
    /// A deck the mixer has no strip for is refused, and that is the whole of the
    /// rule: the selection is drawn as a ring round a strip and is what every
    /// deck-addressed key names its deck by, so a selection past the deck's slots
    /// would be a ring nowhere and a key aimed at a deck the press would be turned
    /// down on. [`View::aim_at`] refuses on the same count one mark along.
    /// [`View::mixer`] is *"one per slot the deck has"*, so its length is the
    /// deck's own count arriving the way every other reading does — and a console
    /// with no deck behind it has no strip to select, which is every test in this
    /// crate.
    ///
    /// It refuses rather than clamping. A press on `3` at a two-slot deck means
    /// *deck D* and there is no deck D; clamping would answer *deck B*, which is a
    /// different deck than the one asked for and would move the mix under a hand
    /// that asked for nothing of the sort.
    ///
    /// The `bool` is [`Arrangement::typed`]'s: a caller repaints on a move and not
    /// on a press, so a press that changed nothing costs no frame (P-0091).
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

    /// Where focus is and what every bay remembers, to read — [`View::focus`] the
    /// field, which is where the argument is.
    ///
    /// A reading and not a door. It is `&`, so the three pointers still have
    /// exactly the writers they had: `select`, `walk`, `point_at`, `select_scope`
    /// and `step_scope` are what refuse a deck the mixer draws no strip for, a row
    /// past the listing and a scope with no chip, and a `&mut` here would be a way
    /// past all five. What it is for is asking one question — *are these the same
    /// field* — which `tests/focus.rs` asks and nothing else can.
    ///
    /// [`View::focus`]: Self::focus
    pub fn focus(&self) -> &Focus {
        &self.focus
    }

    /// Where focus is and what every bay remembers, to write — and it is
    /// `pub(crate)` where [`View::focus`] above is `pub`.
    ///
    /// What it is for is the address path and not the memory.
    /// [`crate::focus::press`] moves `Address::at` as a digit descends and `esc`
    /// climbs, and every write to the *remembered* item still goes through
    /// [`View::select`], [`View::walk`], [`View::point_at`], [`View::select_scope`]
    /// and [`View::step_scope`] — the five methods that refuse a deck the mixer
    /// draws no strip for, a row past the listing and a scope with no chip. The
    /// grammar adds a route and no exception, which is the whole of why this door
    /// is the crate's and not the world's.
    pub(crate) fn focus_mut(&mut self) -> &mut Focus {
        &mut self.focus
    }

    /// Which bay a key press is addressed to, resolved against the arrangement —
    /// [`Focus::bay`], which is where the argument is.
    ///
    /// `None` only for an arrangement with no bay in it, which no arrangement this
    /// crate builds is.
    pub fn focused(&self, panel: &Panel) -> Option<&'static Region> {
        self.focus.bay(panel.layout())
    }

    /// `Tab`, and `shift-Tab` at `step` of `-1` — [`Focus::tab`], and the `bool` is
    /// [`View::select`]'s: a caller repaints on a move.
    ///
    /// The bay it leaves keeps where it was. `Tab` never descends and never pops,
    /// so the address the Mixer had reached is the address it has when focus comes
    /// back to it — which is the deck selection persisting *"while your hands are
    /// in the library"*, seen as the general rule rather than as a habit of one
    /// bay.
    pub fn tab(&mut self, panel: &Panel, step: i32) -> bool {
        self.focus.tab(panel.layout(), step)
    }

    /// `esc`: up one level of the focused bay's address, and `false` where there
    /// was no level to leave — [`Focus::up`].
    ///
    /// At bay level it acts on nothing and the caller says so, which is ADR-0259's
    /// own clause and
    /// [P-0083](../../../../docs/principles/0083-a-refusal-carries-what-the-next-attempt-needs.md):
    /// there is no unfocused state to fall out into, and a key that declines
    /// silently is indistinguishable from one that is not bound. It does not quit —
    /// the window's own close is what does
    /// ([ADR-0315](../../../../docs/adr/0315-a-model-has-no-window-so-the-twelve-surface-rows-mcp-badges-are-gap.md)).
    pub fn focus_up(&mut self, panel: &Panel) -> bool {
        self.focus.up(panel.layout())
    }

    /// Where the dashed focus ring goes, or `None` where the arrangement gives the
    /// focused bay no rectangle to put one on — [`focus::mark`], with the bay this
    /// console has focus on.
    ///
    /// A derivation rather than a paint, which is this crate's arrangement
    /// everywhere a mark is drawn: [`View::draw`] paints from this and
    /// `tests/focus.rs` asks it the same question, so the ring an operator sees and
    /// the ring a test reads cannot come apart.
    ///
    /// `panel` must be solved: [`Layout::rect`] refuses to answer from a dirty one.
    pub fn focus_mark(&self, panel: &Panel) -> Option<Rect> {
        focus::mark(panel.layout(), self.focus.bay(panel.layout())?)
    }

    /// The mark a *folded* bay holding focus wears, or `None` where the focused bay
    /// is not folded — [`focus::folded_head`], with the bay this console has focus
    /// on, and the title to paint in it.
    ///
    /// A folded region has no rectangle, so the ring alone would land on nothing an
    /// operator could read; `docs/manual/console.html` specifies the head alone,
    /// and this is where the panel draws it. It is [`View::draw`]'s one mark
    /// painted over the bays rather than inside one, for the reason the five cards
    /// are: the edge a fold leaves belongs to whatever is drawn next to it, and
    /// this is only ever drawn while an operator has deliberately tabbed onto the
    /// bay that is not there.
    ///
    /// `panel` must be solved: [`Layout::rect`] refuses to answer from a dirty one.
    pub fn folded_mark(&self, panel: &Panel) -> Option<(Rect, &'static str)> {
        let bay = self.focus.bay(panel.layout())?;
        let mark = focus::folded_head(panel.layout(), bay)?;
        // A headless row has no title to draw, and the mark is what says a bay
        // is there — so the word alone stands for it, exactly as the row
        // stands in for the head that is not drawn (ADR-0159, ADR-0259).
        Some((mark, head_of(bay).map_or("", |head| head.title)))
    }

    /// What the `+ lane` chooser offers this frame — the one value [`sequencer`]
    /// lays its card out from, and [`View::target`]'s shape one bay along: read
    /// once for the frame and handed to the paint and to the press, so the item
    /// that is drawn and the item a press lands on are one derivation of one
    /// reading.
    ///
    /// The faders are every strip the mixer draws, which is [`View::select`]'s
    /// count read again; the parameters are one deck's, the load pulldown's
    /// ([`View::target_deck`]) — see [`Choices`], which carries the argument.
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

    /// Whether the `+ lane` chooser's card is down — see [`View::lane_open`] the
    /// field.
    pub fn lane_open(&self) -> bool {
        self.lane_open
    }

    /// Put the card down, and answer whether it went down.
    ///
    /// Refused where there is nothing to point at, which is [`View::open_target`]'s
    /// rule: a card with no items offers nothing to pick, and
    /// [`crate::input::claim`]'s rule 2 would give it every press on the console
    /// until a second press shut it again.
    pub fn open_lane(&mut self) -> bool {
        if self.lane_open || self.lane_choices().items.is_empty() {
            return false;
        }
        self.lane_open = true;
        true
    }

    /// Take the card away, and answer whether there was one down —
    /// [`View::shut_target`]'s shape and its reason.
    pub fn shut_lane(&mut self) -> bool {
        let was = self.lane_open;
        self.lane_open = false;
        was
    }

    /// Every live region that is declaring this frame, each with what one update of
    /// it costs, how stale it may get, and when its picture is next different from
    /// the one on screen.
    ///
    /// The first two are P-0091's and are constants of the presentation; the third
    /// is [`crate::budget::Declared::moves_in`], it is a function of the frame, and
    /// it exists because *how often must this be drawn* and *is this moving now*
    /// are two questions and only one of them was being asked
    /// ([ADR-0283](../../../../docs/adr/0283-a-region-declares-when-its-picture-next-changes-not-that-something-is-pending.md)).
    ///
    /// # This is P-0091's naming, and the unit is a region
    ///
    ///
    /// [P-0091](../../../../docs/principles/0091-cost-is-known-before-it-is-paid.md):
    /// *"Anything that must be live names two numbers — what its update costs, and
    /// how stale it may get in milliseconds."* This is that naming, and
    /// [`crate::budget`] holds the numbers with the arguments for where each came
    /// from. A region is redrawn whole or not at all, so what appears here is a
    /// node of the arrangement — by the name every surface addresses it by — and
    /// never an animation, a control or a slice of a frame.
    ///
    /// # Two regions, at two rates, for two different reasons
    ///
    /// The transport row, whenever the beat grid is drawn: the light travels the
    /// grid once a bar and it is the panel's continuous motion, which
    /// [P-0094](../../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)
    /// says is how a stopped panel announces itself. It declares [`BEAT_STALENESS`]
    /// and it does not ask whether anything is pending, which is the whole point of
    /// it: a signal that only ran while something was happening would be quiet
    /// exactly when the panel had gone quiet.
    ///
    /// The mixer bay, while something in it is outstanding. Inside it the tally's
    /// word rolls toward a residency that has not been granted and each of a
    /// strip's two faders reaches toward a value a transition has not reached yet —
    /// three presentations at one rate, off one [`Phase`], so they are one term and
    /// not three
    /// ([ADR-0190](../../../../docs/adr/0190-the-parked-tally-rolls-because-two-lamps-do-not-fit-in-fifty-three-pixels.md),
    /// [ADR-0206](../../../../docs/adr/0206-a-fader-marks-where-it-is-going-and-keeps-reaching-for-it.md)).
    /// A second *rate* would be a second declaration; a second *user* of one rate
    /// is not — which is why the beat is a second entry here and the two faders are
    /// not.
    ///
    /// This is the first frame on which either sum has two terms, and
    /// `tests/schedulable.rs` reads them: `Σ (cost / staleness)` is 0.0889 against
    /// 1.0, and `max(cost)` is still one number because both regions declare the
    /// same whole panel pass (ADR-0210).
    ///
    /// # Pending is not enough: the region that shows it has to be laid out
    ///
    /// A region that is not on screen declares nothing, however much is pending
    /// behind it. The strips are rewritten every frame from the deck, so *is
    /// anything pending* is a fact about the deck; P-0091 is about what must be
    /// live, and a bay the operator has folded away is not live. A declaration made
    /// for it buys a repaint of something nobody can see — measured, before this
    /// asked: with the picture and the preview row folded the window sat at 28.7 to
    /// 29.0 frames a second, and folding the mixer bay on top of that moved the
    /// price of a frame and not the rate. It draws nothing at all now. The
    /// alternative — declare it anyway and let a scheduler drop it — is
    /// [ADR-0193](../../../../docs/adr/0193-a-region-that-is-not-laid-out-declares-nothing-rather-than-being-dropped-later.md),
    /// which is where it lost, and it lost on the sentence being false rather than
    /// unaffordable: a region nobody can see is not showing anything, so it cannot
    /// be showing anything out of date.
    ///
    /// This is
    /// [P-0073](../../../../docs/principles/0073-a-node-claims-only-what-its-visible-content-can-use.md)
    /// in time rather than in space — *a node claims only what its visible content
    /// can use*, where what is claimed is a share of the frame budget rather than a
    /// share of the viewport.
    ///
    /// The question is asked of the layout, which already answers it.
    /// [`Layout::visible`](karakuri_layout::Layout::visible) is ADR-0183's
    /// disjunction — the operator's fold and the drawing's `set_aside`, read as one
    /// — walked up the ancestors, so a folded right pane takes the mixer with it
    /// and no second derivation of *is this laid out* is written here. It is not
    /// [`mixer`]'s `None`, which is a different question and a stricter one: that
    /// answers *can the strips be laid out in this rectangle this pass*, and says
    /// no for a window merely too small and for the frame before `egui` has fonts.
    /// Neither is a reason to stop the roll — only being out of the layout is — and
    /// the layout answers this one without a solve, which matters because the fold
    /// is applied and this is asked before the next one.
    ///
    /// # Nothing arbitrates between two of these
    ///
    /// ADR-0164's second half is a scheduler and there is not one. What this feeds
    /// is [`View::animating`], which takes the soonest staleness and nothing else,
    /// and `tests/schedulable.rs`, which sums over whatever this answers and
    /// asserts the two conditions the principle states.
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

    /// What the Sequencer bay declares: the step's staleness while it has a
    /// playhead to move *and* the bay is laid out, and nothing otherwise — with the
    /// deadline taken from where the beat has got to inside the current step.
    ///
    /// # Three conditions, and the third is what makes it honest
    ///
    /// The bay is laid out, which is ADR-0193 and is [`View::mixer_declares`]'s
    /// first condition word for word.
    ///
    /// There is a pattern behind it. [`View::sequencer`] is `None` for a console
    /// with no session, and [`sequencer`] draws nothing then.
    ///
    /// And it has a lane. The picture that moves here is the playhead column, which
    /// stands over the rows — so a pattern with no lanes has nothing for it to
    /// stand on and this bay is a head and a ruler that do not move. A declaration
    /// made for it would buy frames that redraw a still picture, which is the whole
    /// of what
    /// [ADR-0283](../../../../docs/adr/0283-a-region-declares-when-its-picture-next-changes-not-that-something-is-pending.md)
    /// is about. A muted lane still counts: the mute stops the lane *writing*, and
    /// the column goes on crossing its cells.
    ///
    /// # The deadline is the music's and the rate is not
    ///
    /// [`step_moves_in`] is a function of `beats` and the tempo, so the deadline
    /// moves with the grid the way the steps do; [`STEP_STALENESS`] is a constant
    /// at the mock's tempo, so the sums `tests/schedulable.rs` asserts do not
    /// become a function of how fast the music is (ADR-0212).
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

    /// The soonest any live region on this panel will next look different from what
    /// is on screen, and `None` when nothing on it is moving.
    ///
    /// # It is `moves_in` and not `staleness`, and that is the whole of ADR-0283
    ///
    /// A staleness says how finely a region has to be drawn *while it moves*; it
    /// does not say whether the region is moving now. The mixer bay's roll rests
    /// for 600 ms of every second and its curve is exactly zero throughout, so a
    /// deadline taken from the staleness alone woke this window seventeen times a
    /// second to draw a chip in the position it was already in.
    /// [`crate::budget::Declared::moves_in`] is what a region answers instead,
    /// `staleness` stays the constant `tests/schedulable.rs` sums, and the
    /// invariant between them — `moves_in >= staleness` — is why this can only take
    /// a frame away and never bring one forward
    /// ([ADR-0283](../../../../docs/adr/0283-a-region-declares-when-its-picture-next-changes-not-that-something-is-pending.md)).
    ///
    /// Nothing about the beat changes, and P-0094 is why it must not: the light
    /// travels the grid on every frame the session advances, so its two numbers are
    /// one number and this goes on answering [`BEAT_STALENESS`] for as long as the
    /// row is drawn.
    ///
    /// # The view is what knows the rate, so the harness is told rather than
    /// guessing
    ///
    /// [`View::declares`] is where the regions and their two numbers are, and this
    /// is the one of the two numbers a window can act on today:
    /// `crate::repaint::Change::Animating` turns it into a deadline. A rate written
    /// into the harness instead would be a presentation's number kept where the
    /// presentation is not — change the roll and the window goes on servicing the
    /// old one, with nothing failing to compile and nothing to assert against.
    ///
    /// The soonest, not the sum, and that is the whole of what is decided here: a
    /// deadline is met by drawing, and one frame drawn in time for the soonest is
    /// in time for every other. Choosing which region a frame is *for* is a
    /// scheduler's, and there is not one.
    ///
    /// # `None` is the panel saying it is still, and that is now a narrower state
    /// than *nothing pending*
    ///
    /// `None` is not an absence of information: it is the panel saying it is still,
    /// and the window then sleeps. A console with no parked slot and no scheduled
    /// move on a fader costs exactly what it cost before either existed, which is a
    /// claim `tests/parked.rs` and `tests/armed.rs` make rather than a hope — and
    /// both of them ask it of a console with no engine behind it, which is what
    /// every test in this crate is.
    ///
    /// With an engine behind it and the transport row on screen this never answers
    /// `None`, because the beat is moving and says so
    /// ([`View::transport_declares`],
    /// [P-0094](../../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)).
    /// That is ADR-0164's still-panel clause narrowing rather than failing: a panel
    /// with something moving on it is a panel with something changing on it, and
    /// the reason it is moving is a declaration rather than an accident.
    pub fn animating(&self, layout: &karakuri_layout::Layout) -> Option<Duration> {
        self.declares(layout).map(|live| live.moves_in).min()
    }

    /// Draw the whole console. The `ui` is the root one [`egui::Context::run_ui`]
    /// hands the frame's closure.
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
                            outputs::outputs_into(ui, &pal, &row);
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
                            sequencer::sequencer_into(ui, &pal, &bay);
                        }
                    }
                    Kind::Master => {
                        card(ui, &pal, rect);
                        head_into(ui, &pal, rect, placed.region, opening);
                        if let Some(row) = master(ui.ctx(), panel.layout(), out, chain) {
                            master::master_into(ui, &pal, &row);
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
                            staging::staging_into(ui, &pal, &bay, waiting);
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
                        sequencer::lane_card_into(ui, &pal, &card, &choices);
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
    /// `egui` sets it, not `winit`. `egui_winit`'s `handle_platform_output` already
    /// writes the window's cursor from `PlatformOutput` every frame, so a
    /// `Window::set_cursor` call beside it is a second writer and the last one each
    /// frame wins — which is a flicker that depends on event order. One writer, and
    /// it is the one that is already there.
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

/// A bay's card: `.bay`'s panel fill, 11px radius and drop shadow.
pub(crate) fn card(ui: &Ui, pal: &Palette, rect: Rect) {
    let radius = CornerRadius::same(size::BAY_RADIUS as u8);
    ui.painter().add(pal.shadow.as_shape(rect, radius));
    ui.painter().rect_filled(rect, radius, pal.panel);
}
