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
    pane_target, rend_chips, sens_chips, AimChips, Aimed, DeckHead, DeckName, InspectorPane,
    KeepPill, Naming, Node, NodeAuthority, Pane, PaneTarget, Param, ParamGrip, Renderer, SensChip,
    Source, Uses, UsesLine, AUTHORITIES, PANES, PANE_DECKS, PANE_NAMES, RE_SALT_LABEL, SCRUB_BEATS,
    SENS_LABEL, SYNCS, TAKE_BACK,
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
pub use widgets::card::{bay_card, popup_card};
pub use widgets::chip::Tally;
pub use widgets::field::CARET;
pub use widgets::glyph::{arrow_mark, chevron_down, CHEVRON_H, CHEVRON_W};
pub use widgets::*;
#[allow(unused_imports)]
pub(crate) use widgets::*;

pub mod draw;
pub mod regions;

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
    /// What the master chain is running this frame, or `None` for a console with
    /// no engine behind it — in which case the bay draws the out row and nothing
    /// under it.
    ///
    /// [`View::master_out`]'s seam exactly, one row down:
    /// `karakuri_engine::present::Present::chain_reading` is what a harness reads
    /// it from, and [`Chain`] is that value mirrored into a crate with no engine
    /// in it. A chain with no slot in it is `Some` with an empty list, which is
    /// the default chain: the bay then draws the out row and `+ add`.
    pub master_chain: Option<Chain>,
    /// What the Master bay's `+ add` offers this frame: one entry per `kind L5`
    /// procedure the library holds, in the order the library lists them.
    ///
    /// Written by the host per press beside [`View::library`]: an entry carries
    /// the content address of a procedure's source and this crate reads no store
    /// (ADR-0156). Empty is a library listing no `kind L5` procedure, and the
    /// card does not go down on one.
    ///
    /// It is also what a carried Library row is resolved through at a release over
    /// the chain — a row this list does not name is not a `kind L5`, and the drop
    /// is refused with that reason.
    pub chain_add: Vec<AddChoice>,
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
    /// Whether the Master bay's `+ add` chooser is down — [`View::lane_open`]'s
    /// field one bay along. [`View::open_chain_add`] and
    /// [`View::shut_chain_add`] are the only ways in.
    chain_add_open: bool,
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
            // Nothing to add, which is a console whose library has listed no
            // `kind L5` procedure — every test in this crate that does not
            // hand one in.
            chain_add: Vec::new(),
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
    ///
    /// It takes every card the address descends into away, wherever the address
    /// is: the Transport's two
    /// ([ADR-0350](../../../../docs/adr/0350-the-transports-two-cards-are-walked-and-the-tempo-figure-steps-by-a-beat-a-minute.md)),
    /// the Sequencer's `+ lane`
    /// ([ADR-0351](../../../../docs/adr/0351-the-lane-chooser-is-a-rung-of-the-address.md))
    /// and the Master's `+ add`
    /// ([ADR-0352](../../../../docs/adr/0352-the-chains-list-is-the-master-bays-items-and-a-slot-is-taken-out-by-a-glyph-on-its-row.md)).
    /// The cards the address does not walk are left alone.
    pub fn tab(&mut self, panel: &Panel, step: i32) -> bool {
        let moved = self.focus.tab(panel.layout(), step);
        if moved {
            focus::shut_cards(self);
        }
        moved
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
    ///
    /// A card on the address's path is a level of the address, so `esc` takes it
    /// away and the address ends on the control it hangs from — one level up where
    /// the address had descended into the card, and where it already was
    /// otherwise. A card that is down anywhere else is left alone and `esc` is the
    /// ordinary climb. [`focus::card_on_path`] is that reading, and it is one
    /// reading for all four cards
    /// ([ADR-0332](../../../../docs/adr/0332-focus-is-a-pointer-the-console-owns-and-the-three-pointers-are-instances-of-it.md)).
    pub fn focus_up(&mut self, panel: &Panel) -> bool {
        if let Some((control, inside)) = focus::card_on_path(self, panel) {
            focus::shut_card(self, control);
            if inside {
                self.focus.up(panel.layout());
            }
            return true;
        }
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

    /// What the Master bay's `+ add` offers this frame — [`View::lane_choices`]'
    /// shape one bay along. The item that is drawn and the item a press lands on
    /// are one derivation.
    pub fn chain_choices(&self) -> AddChoices {
        AddChoices {
            items: self.chain_add.clone(),
            open: self.chain_add_open,
        }
    }

    /// Whether the `+ add` chooser's card is down.
    pub fn chain_add_open(&self) -> bool {
        self.chain_add_open
    }

    /// Put the card down, and answer whether it went down — [`View::open_lane`]'s
    /// rule: a card with nothing on it offers nothing to pick.
    pub fn open_chain_add(&mut self) -> bool {
        if self.chain_add_open || self.chain_add.is_empty() {
            return false;
        }
        self.chain_add_open = true;
        true
    }

    /// Take the card away, and answer whether there was one down.
    pub fn shut_chain_add(&mut self) -> bool {
        let was = self.chain_add_open;
        self.chain_add_open = false;
        was
    }

    /// What adding the carried Library row to the chain asks for, or the reason
    /// it asks for nothing.
    ///
    /// The chain holds `kind L5` procedures, so a row that is not one is refused
    /// and the refusal says what the chain holds
    /// ([P-0083](../../../../docs/principles/0083-a-refusal-carries-what-the-next-attempt-needs.md)).
    /// [`View::chain_add`] is the list a row is resolved against: it is the
    /// library's `kind L5` rows, by the name each is listed under.
    pub fn chain_landing(&self, row: &str) -> Result<Operation, &'static str> {
        self.chain_add
            .iter()
            .find(|choice| choice.words == row)
            .map(AddChoice::operation)
            .ok_or("the master chain holds kind L5 procedures, and this row is not one")
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
}
