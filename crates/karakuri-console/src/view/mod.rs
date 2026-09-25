//! Renders the console view hierarchy using `egui`.
//!
//! Paints resolved layout geometry produced by [`karakuri_layout::Layout`],
//! dispatching bay headers, deck previews, meters, faders, and control widgets.

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
    InspectorPane, KeepPill, Naming, Node, NodeAuthority, Pane, PaneTarget, PaneTargetCtx, Param,
    ParamGrip, Renderer, SensChip, SlotMcpPill, Source, Uses, UsesLine, AUTHORITIES, PANES,
    PANE_DECKS, PANE_NAMES, RE_SALT_LABEL, SCRUB_BEATS, SENS_LABEL, SYNCS, TAKE_BACK,
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
            plugin: false,
            plugin_available: false,
            plugin_name: None,
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
            master_chain_building: false,
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
