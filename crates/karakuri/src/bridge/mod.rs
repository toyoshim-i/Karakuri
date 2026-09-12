//! Engine coordination, presentation sinks, and view bridging.

use std::sync::Arc;

use karakuri_console::egui_wgpu;
use karakuri_console::focus::{self, Step};
use karakuri_console::panel::Panel;
use karakuri_console::view::{
    self, Basis, Budgeted, Go, Picture, Reading, RowKind, Scope, TransitionSettings, View, DECKS,
    DECK_LETTERS,
};
use karakuri_engine::estimate::Estimate;
use karakuri_engine::governor::{Basis as Spent, Report};
use karakuri_engine::set::{Layering, Published};
use karakuri_engine::transition::Selection;
use karakuri_engine::transport::Sync as EngineSync;
use karakuri_engine::{
    Blend, Control, Cut, Deck, DeckSlot as EngineSlot, Event, Gpu, HotSwap, Look, Mask, MaskKind,
    Present, Residency, Set, TonemapOp,
};
use karakuri_environment::{audio, history, mix, setfile, watch, Asked};
use karakuri_ir::Kind as Layer;
use karakuri_layout::Layout;
use karakuri_mcp as mcp;
use karakuri_operation::{BlendMode, Operation, SetTransfer};
use karakuri_operation_record::{Current, Written};
use karakuri_store::record::{DeckSlot, Record};
use karakuri_store::store::Store;
use winit::keyboard::{Key, NamedKey};
use winit::window::Window;

use crate::{
    deck_letter, ms, rewired, watched, Costs, Keeping, Sources, ASKED_TO_PRIME, CANVAS, ON_AIR,
    SEED_SALT, SLOTS,
};

mod engine;
mod filesystem;
mod handlers;
mod sinks;

pub(crate) use engine::*;
pub(crate) use filesystem::*;
pub(crate) use handlers::*;
pub(crate) use sinks::*;
