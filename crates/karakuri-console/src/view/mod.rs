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

// Internal helper used by `focus` to cycle tone mapping modes.
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
// Internal helper used by `focus` to cycle deck head sync state.
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

pub mod prompt;

/// Prompt bay module re-exports (ADR-0121).
pub use prompt::*;

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
            // Projector window initially inactive.
            projector: false,
            plugin: false,
            plugin_available: false,
            plugin_name: None,
            picture: None,
            previews: [None; DECKS],
            // Overloaded state per deck, initialized to false.
            overloaded: [false; DECKS],
            costs: [None; DECKS],
            transport: None,
            // Default empty arrangement.
            arrangement: Arrangement::NONE,
            // Audio input configuration unassigned.
            audio: None,
            // Learn mode disarmed by default (P-0094).
            learn: false,
            // MIDI mapping unset.
            map: None,
            // Beat tracker unconfigured.
            tracker: None,
            // Look exposure and tone mapping unconfigured.
            look: None,
            // Master out level unconfigured.
            master_out: None,
            master_chain: None,
            master_chain_building: false,
            // Available chain effect procedures.
            chain_add: Vec::new(),
            // Pre-allocate mixer strips up to DECKS capacity.
            mixer: Vec::with_capacity(DECKS),
            mixer_dirty: false,
            // Library listing rows.
            library: Vec::new(),
            kinds: Vec::new(),
            // Starred item set initially empty.
            starred: std::collections::BTreeSet::new(),
            // Target Set identifier unset (ADR-0304).
            aimed: None,
            // Pre-allocate scope chips.
            scopes: Vec::with_capacity(Scope::ALL.len()),
            // Holds filter criteria; unallocated until configured by store metadata.
            holds: Vec::new(),
            // Active folder path or incoming drag payload.
            folder: None,
            incoming: None,
            // Filter criteria unselected.
            holds_at: None,
            showing: LibraryKinds::EVERYTHING,
            // Active deck name prompt unset.
            naming: None,
            // Pane scroll offsets initialized to top.
            scroll: [0.0; PANES],
            // Default pane deck bindings (pane 0 -> deck A, pane 1 -> deck B).
            pane_deck: PANE_DECKS,
            // Pulldown menu closed.
            pane_open: None,
            // Pre-allocated for max deck slots to avoid frame reallocations.
            staging: Vec::with_capacity(DECKS),
            // Pre-allocate inspector panes up to PANES capacity.
            inspector: Vec::with_capacity(PANES),
            canvas: MOCK_CANVAS,
            // Bay opening state defaulted to closed (ADR-0235).
            opening: Open::CLOSED,
            slot_policies: [SlotPolicy::Auto; DECKS],
            phase: Phase::ZERO,
            // Initial focus defaults to the first reachable bay in traversal order.
            focus: Focus::default(),
            // Target deck index initially 0 (deck A).
            target: 0,
            // Target pulldown closed.
            target_open: false,
            wiring_open: None,
            // Lane chooser card closed.
            lane_open: false,
            chain_add_open: false,
            // Row context menu closed.
            menu_row: None,
            // Library scroll offset initialized to top.
            library_scroll: 0.0,
            // Inspection reading details closed.
            reading: None,
            // Transition defaults to start settings.
            transition: TransitionSettings::START,
            // Sequencer unconfigured.
            sequencer: None,
            // Prompt bay unselected and menu closed.
            prompt: PromptState::new(),
            // Pre-allocate region layout cache up to REGIONS capacity.
            placed: Vec::with_capacity(REGIONS.len()),
        }
    }
}
