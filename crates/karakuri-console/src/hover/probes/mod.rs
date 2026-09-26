//! Control probes, hit-test derivations, and registration table for the console hover layer.

use karakuri_layout::Point;

use super::citation::Cite;
use crate::control::{descriptor_for, ControlDescriptor, ControlId};
use crate::panel::Panel;
use crate::view::View;

pub mod hit_test;
pub mod table;

pub use table::*;

/// One control that explains itself on hover: its identity, where its words are
/// in the mock, and the derivation that says the pointer is on it.
pub struct Tipped {
    /// The control's name, in the words [`crate::input::PROBES`] uses for the row
    /// it belongs to. It is what a test and a reader address this row by, and it is
    /// never drawn.
    pub control: &'static str,
    /// Where this control's words are in the mock.
    pub cites: Cite,
    /// The derivation that draws the control, asked whether the point is on it, and
    /// nothing is stored — [`crate::input::Probe::ask`]'s signature and its rule,
    /// one question finer.
    pub at: fn(&Panel, &egui::Context, &View, Point) -> bool,
}

/// Every tipped control, flattened out of [`TIPS`] in the order [`resolve`]
/// asks them.
pub fn flat() -> impl Iterator<Item = &'static Tipped> {
    TIPS.iter().flat_map(|(_, tips)| tips.iter())
}

/// Returns the index in [`flat`] for the control under `p`, using first-match order.
pub fn resolve(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> Option<usize> {
    if ctx.cumulative_pass_nr() == 0 || view.has_modal_overlay() {
        return None;
    }
    flat().position(|tip| (tip.at)(panel, ctx, view, p))
}

/// Returns the [`ControlId`] corresponding to a flat tip index.
pub fn control_id_at(index: usize) -> Option<ControlId> {
    let mut count = 0;
    for (probe_idx, (_, tips)) in TIPS.iter().enumerate() {
        if index < count + tips.len() {
            return ControlId::from_probe_index(probe_idx);
        }
        count += tips.len();
    }
    None
}

/// Returns the [`ControlDescriptor`] corresponding to a flat tip index.
pub fn descriptor_at(index: usize) -> Option<&'static ControlDescriptor> {
    control_id_at(index).map(descriptor_for)
}

/// Returns the keyboard shortcut assigned to a specific tipped control, if any.
pub fn hotkey_for_tip(index: usize) -> Option<&'static str> {
    let id = control_id_at(index)?;
    let tip = flat().nth(index)?;
    match id {
        ControlId::Transition => {
            if tip.control == "the go capsule" {
                Some("space")
            } else {
                None
            }
        }
        ControlId::Tracker => {
            if tip.control == "the tap" {
                Some("b")
            } else {
                None
            }
        }
        _ => descriptor_at(index).and_then(|d| d.hotkey),
    }
}
