#![allow(unused_imports, dead_code)]

pub(super) use super::super::*;
pub(super) use karakuri_engine::binding::Curve;
pub(super) use karakuri_engine::chain_swap::ChainSlot;
pub(super) use karakuri_engine::deck::{Blend, Mask, MaskKind, Residency};
pub(super) use karakuri_engine::master::{Chain, Cut, Slot, SlotSpec};
pub(super) use karakuri_engine::present::TonemapOp;
pub(super) use karakuri_engine::set::Authority;
pub(super) use karakuri_engine::transition::Control;
pub(super) use karakuri_engine::transport::{Sync, Transport};
pub(super) use karakuri_engine::Look;
pub(super) use karakuri_signal::oscillator::Oscillator;
pub(super) use karakuri_store::record::{DeckSlot, Record};

/// Converts an operation to a single record using the default current state.
pub(super) fn from_operation(operation: karakuri_operation::Operation) -> Record {
    from_operation_reading(operation, karakuri_operation_record::Current::default())
}

/// Converts an operation to a single record given a specific current state reading.
pub(super) fn from_operation_reading(
    operation: karakuri_operation::Operation,
    current: karakuri_operation_record::Current,
) -> Record {
    use karakuri_operation_record::Written;
    match karakuri_operation_record::written(&operation, &current) {
        Written::Records(records) if records.len() == 1 => {
            records.into_iter().next().expect("length just checked")
        }
        other => panic!("`{}` is not one record: {other:?}", operation.title()),
    }
}

/// Builds a Current state reading with scheduled transition settings.
pub(super) fn scheduled(
    at: u8,
    quantum: f64,
    beats: f64,
    shape: Curve,
) -> karakuri_operation_record::Current {
    karakuri_operation_record::Current {
        transition: Some(current_transition(
            &grid_at(at),
            quantum,
            beats,
            shape,
            MaskKind::Linear,
            0.0,
        )),
        ..karakuri_operation_record::Current::default()
    }
}

/// Builds a Current state reading with wipe transition, mask, and mix settings.
pub(super) fn wiping(
    at: u8,
    quantum: f64,
    beats: f64,
    shape: Curve,
    front: MaskKind,
    angle: f32,
    worn: Mask,
) -> karakuri_operation_record::Current {
    karakuri_operation_record::Current {
        transition: Some(current_transition(
            &grid_at(at),
            quantum,
            beats,
            shape,
            front,
            angle,
        )),
        mask: Some(current_mask(worn)),
        mix: Some(current_mix(Blend::Add, Residency::Allocated)),
        ..karakuri_operation_record::Current::default()
    }
}

/// Returns an oscillator standing at beat `at` with 120 BPM.
pub(super) fn grid_at(at: u8) -> Oscillator {
    let mut grid = Oscillator::new(120.0);
    grid.advance(at, 0.5);
    grid
}

/// A reading of a mask that is wearing exactly this, for the two operations
/// that need one.
pub(super) fn reading_of(mask: Mask) -> karakuri_operation_record::Current {
    karakuri_operation_record::Current {
        mask: Some(karakuri_operation_record::Mask {
            kind: wipe_kind(mask.kind()),
            angle: mask.angle(),
            position: mask.position(),
            softness: mask.softness(),
        }),
        ..karakuri_operation_record::Current::default()
    }
}
