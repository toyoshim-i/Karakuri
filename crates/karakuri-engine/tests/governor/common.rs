#![allow(unused_imports)]

pub(crate) use karakuri_engine::deck::{Deck, Residency};
pub(crate) use karakuri_engine::estimate::{fit, rungs, Estimate, Floor, Unfit};
pub(crate) use karakuri_engine::governor::{
    Basis, Estimated, FloorRead, Governor, Reason, SlotState,
};
pub(crate) use karakuri_engine::probe::{Measurement, MeasurementMethod};
pub(crate) use karakuri_engine::swap::HotSwap;
pub(crate) use karakuri_engine::{Gpu, Present, Set};
pub(crate) use karakuri_ir::typed::Checked;
pub(crate) use karakuri_ir::Topology;

pub(crate) const WIDTH: u32 = 128;
pub(crate) const HEIGHT: u32 = 128;
pub(crate) const CAPACITY: u32 = 1024;

pub(crate) fn cost(ms: f32) -> Measurement {
    Measurement {
        ms,
        method: MeasurementMethod::GpuTimestamp,
        capacity: CAPACITY,
        resolution: (1280, 720),
    }
}

pub(crate) fn live(ms: f32) -> SlotState {
    SlotState {
        requested: Residency::Live,
        cost: Some(cost(ms)),
        estimate: None,
        closed_form: false,
    }
}

pub(crate) fn priming(ms: f32) -> SlotState {
    SlotState {
        requested: Residency::Priming,
        cost: Some(cost(ms)),
        estimate: None,
        closed_form: false,
    }
}

/// A Live slot nothing has measured — every `HotSwap::fixed` Set until
/// `Deck::measure_slots` runs. Its cost is not zero, it is unknown.
pub(crate) fn unmeasured_live() -> SlotState {
    SlotState {
        requested: Residency::Live,
        cost: None,
        estimate: None,
        closed_form: false,
    }
}
