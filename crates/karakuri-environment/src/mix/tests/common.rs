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

/// test that wants a `Record::Opacity` asks the conversion for one rather than
/// spelling it, so what it round-trips below is what a key press and a mapped
/// pad actually write. Panics rather than returning nothing on an operation
/// that is not one record, because a test silently given no record is a test
/// that checks nothing.
pub(super) fn from_operation(operation: karakuri_operation::Operation) -> Record {
    from_operation_reading(operation, karakuri_operation_record::Current::default())
}

/// The same, for an operation whose record is not a function of the operation
/// alone. The mask pair is the case: each half writes `Record::Mask` whole, so
/// each needs the other half read back.
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

/// A reading of the transition settings a surface is holding, for the four
/// operations that schedule a move.
///
/// Through [`current_transition`] rather than by spelling a
/// `karakuri_operation_record::Transition`, which is `from_operation`'s rule
/// one function down: what these tests convert is what a surface hands over,
/// including the curve going through the two copies of that list.
///
/// The front shape is fixed here and is a wipe's alone. A fade, a crossfade and
/// a selection have no front, so the fourth setting is a value none of the
/// three reads; [`wiping`] is what hands one over on purpose.
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

/// The same, plus the three readings a wipe takes that a fade does not: the
/// front shape the transition row is holding, the mask the deck being wiped in
/// is already wearing, and where that deck already sits in the mix.
///
/// All three go through the functions a surface calls — [`current_transition`],
/// [`current_mask`] and [`current_mix`] — for [`scheduled`]'s reason: what
/// these tests convert is what a program hands over, including the shape
/// crossing `MaskKind`'s two spellings on the way.
///
/// The slot is at `add` and off air, so both of the records a wipe writes
/// conditionally are written: the caller that wants the other case is
/// [`a_wipe_leaves_the_mode_the_operator_chose_on_the_deck`], which hands in
/// its own.
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

/// A session oscillator standing on beat `at`.
///
/// 120 BPM makes a beat half a second, so a step of that length is a beat
/// apiece and the position is exact rather than a float that nearly is — which
/// matters here, because what these tests are about is the instant a move lands
/// on.
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
