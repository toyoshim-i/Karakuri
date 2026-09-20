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
pub use karakuri_operation::SlotPolicy;
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
    Reach, Strip, StripBox, TransitionRow, TransitionSettings, ROLL_PERIOD, ROLL_REACH,
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
    pane_target, rend_chips, sens_chips, slot_mcp_pill, AimChips, Aimed, DeckHead, DeckName,
    InspectorPane, KeepPill, Naming, Node, NodeAuthority, Pane, PaneTarget, Param, ParamGrip,
    Renderer, SensChip, SlotMcpPill, Source, Uses, UsesLine, AUTHORITIES, PANES, PANE_DECKS,
    PANE_NAMES, RE_SALT_LABEL, SCRUB_BEATS, SENS_LABEL, SYNCS, TAKE_BACK,
};
// **The symbol Inspector keeps crate-internal rather than exporting**:
// `next_sync` backs [`crate::focus`]'s own cycling of the deck head's sync
// chip, which needs the derivation and not a second copy of it.
pub(crate) use inspector::next_sync;

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
pub use widgets::card::{bay_card, popup_card};
pub use widgets::chip::Tally;
pub use widgets::field::CARET;
pub use widgets::glyph::{arrow_mark, chevron_down, CHEVRON_H, CHEVRON_W};
pub use widgets::*;
#[allow(unused_imports)]
pub(crate) use widgets::*;

pub mod draw;
pub mod modal;
pub mod regions;

pub use modal::*;
pub use regions::*;

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

/// The console's view: which room it is in, and the frame's plan, kept so a
/// frame does not allocate one.
pub mod budget;
pub mod choices;
pub mod nav;
pub mod types;

pub use types::*;

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
            // Nothing to add, which is a console whose library has listed no
            // `kind L5` procedure — every test in this crate that does not
            // hand one in.
            chain_add: Vec::new(),
            // As many strips as a deck can ever have, so the frame path never
            // grows it — the same reason `placed` is built with a capacity.
            mixer: Vec::with_capacity(DECKS),
            mixer_dirty: false,
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
            slot_policies: [SlotPolicy::Auto; DECKS],
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
            chain_add_open: false,
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
}
