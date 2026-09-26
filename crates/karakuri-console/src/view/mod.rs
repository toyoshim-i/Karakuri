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

/// Transport bay module re-exports (ADR-0121).
pub use transport::{
    arrangement, audio_in, beat_at, exposure_at, learn_pill, look, map_pill, offset_at,
    tracker_group, transport, unit_of, unit_of_offset, Arrangement, ArrangementPill, Ask, AudioAsk,
    AudioIn, AudioInPill, Item, LearnPill, Look, LookRow, MapPill, MapRow, Menu, Rec, Tracker,
    TrackerGroup, Transport, TransportRow, BEAT_PITCH, BEAT_STALENESS, EXPOSURE_MAX, EXPOSURE_MIN,
    EXPOSURE_TRACK_W, LATENCY_OFFSET_MAX_MS, LATENCY_OFFSET_MIN_MS, LATENCY_OFFSET_STEP_MS,
    OFFSET_TRACK_W, TEMPO_BAND, TEMPO_SPAN,
};

mod mixer;

/// Mixer bay module re-exports (ADR-0121).
pub use mixer::{
    after, mixer, roll_at, roll_moves_in, transition, Fader, Go, Level, Mask, Meter, Mixer, Phase,
    Reach, Strip, StripBox, TransitionRow, TransitionSettings, ROLL_PERIOD, ROLL_REACH,
    ROLL_STALENESS, ROLL_TRAVEL,
};
pub(crate) use mixer::{next, next_shape, residency, wipe_kind};

mod library;

/// Library bay module re-exports (ADR-0121).
pub use library::{
    library, load_item, Aim, Block, Chosen, Field, Filters, KindChip, LibraryBay, Load, Menued,
    Opened, Picked, Pointed, Pointing, Published, Read, Reading, RowItem, RowKind, RowMenu, Rows,
    Scope, Taken, Target, Wiring, HOLDS_UNSET, LAYERS, SETS_CHIP,
};

mod inspector;

/// Inspector bay module re-exports (ADR-0121).
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

/// Program bay module re-exports (ADR-0121).
pub use program::{
    band_of, caption_of, picture_rect, preview_rects, program_bay, program_body,
    program_body_with_row_h, program_head, rearrange, Band, Basis, Body, Budgeted, Picture,
    Placement, ProgramBay, ProgramHead, BAND_BLUE_MS, BAND_PURPLE_MS, BAND_RED_MS, BAND_YELLOW_MS,
    MOCK_CANVAS, PREVIEW_MATERIAL, PREVIEW_NO_SLOT, PREVIEW_OVERLOADED,
};

mod master;

/// Master bay module re-exports (ADR-0121).
pub use master::*;

mod sequencer;

/// Sequencer bay module re-exports (ADR-0121).
pub use sequencer::*;
mod staging;

/// Staging bay module re-exports (ADR-0121).
pub use staging::*;

mod layout;

/// Layout and geometry placement module re-exports (ADR-0121).
pub use layout::*;
pub(crate) use layout::{held_inside, positive, track};

mod outputs;

/// Outputs row module re-exports (ADR-0121).
pub use outputs::*;

pub mod widgets;

/// Shared interactive widgets module re-exports (ADR-0121).
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

/// Normalized uv coordinates mapping the full texture rectangle in egui.
const WHOLE_TEXTURE: Rect = Rect {
    min: Pos2::new(0.0, 0.0),
    max: Pos2::new(1.0, 1.0),
};

/// Maximum deck slots and preview cells supported simultaneously (ADR-0156).
pub const DECKS: usize = 4;

/// Deck identifier letters in slot order (ADR-0179).
pub const DECK_LETTERS: [&str; DECKS] = ["A", "B", "C", "D"];

/// Preview cell count per side when displayed beside the main picture (half of [`DECKS`]).
const PER_COLUMN: usize = DECKS / 2;

/// Two columns divide [`DECKS`] exactly, and a `DECKS` that stopped being even
/// would put a cell in neither column rather than fail. It is 4 and the
/// engine's `deck::MAX_SLOTS` is why, so this is a compile-time reading of that
/// number and not a runtime check.
const _: () = assert!(DECKS % 2 == 0 && DECKS == PER_COLUMN * 2);

// ---------------------------------------------------------------------------
// Shared cross-bay constants (ADR-0121)
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
            // Holds filter criteria; unallocated until configured by store metadata.
            holds: Vec::new(),
            // Active folder path or incoming drag payload.
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
            // Pre-allocated for max deck slots to avoid frame reallocations.
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
            // Initial focus defaults to the first reachable bay in traversal order.
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
