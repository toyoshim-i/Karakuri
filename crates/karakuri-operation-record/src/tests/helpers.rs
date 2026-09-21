use super::*;

/// A look nothing else in these tests happens to be: an operator that is
/// not the first in the list and a white point that is not the default, so
/// a conversion filling either in from thin air is visible rather than
/// coincidentally right. A chain of the three shipped procedures with every
/// value distinct, so a record that copied the wrong one is a failing
/// assertion rather than a coincidence. The addresses are short stand-ins
/// for the real content addresses: what this crate does with one is compare
/// it, never resolve it.
pub(crate) fn chain() -> Chain {
    Chain {
        slots: vec![
            slot("sha256:feedback", Some("exit"), 0.34),
            slot("sha256:bloom", None, 0.6),
            slot("sha256:rgb_shift", None, 0.25),
        ],
    }
}

pub(crate) fn slot(
    procedure: &str,
    cut: Option<&str>,
    amount: f32,
) -> karakuri_store::record::ChainSlot {
    karakuri_store::record::ChainSlot {
        procedure: procedure.to_string(),
        cut: cut.map(str::to_string),
        params: [("amount".to_string(), amount)].into_iter().collect(),
    }
}

pub(crate) fn chain_record(slots: Vec<karakuri_store::record::ChainSlot>) -> Record {
    Record::MasterChain(karakuri_store::record::Chain { slots })
}

pub(crate) fn look() -> Look {
    Look {
        tonemap: Tonemap::AgX,
        exposure: 0.25,
        white_point: 4.0,
    }
}

pub(crate) fn records(written: Written) -> Vec<Record> {
    match written {
        Written::Records(records) => {
            assert!(
                !records.is_empty(),
                "`Records` is documented as never empty"
            );
            records
        }
        other => panic!("expected records, got {other:?}"),
    }
}

/// A mask nothing else in these tests happens to be: a shape that is not
/// the default, an angle nobody would reach for, a front part way across
/// and a soft edge — so a conversion filling any of the three it was not
/// asked for from thin air is visible rather than coincidentally right.
pub(crate) fn mask() -> Mask {
    Mask {
        kind: karakuri_operation::WipeKind::Radial,
        angle: 1.25,
        position: 0.4,
        softness: 0.02,
    }
}

/// A deck that is nowhere the wipe is about to put it: still at the blend
/// mode a slot starts in, and not on air.
///
/// So both of the two records a wipe writes conditionally are written
/// against this fixture, and a wipe is its full six — which is what makes
/// [`a_wipe_leaves_a_mode_the_operator_chose_and_a_deck_already_on_air`]
/// the other half of one statement rather than a second subject. A fixture
/// already under `over` would have hidden the omission behind a record that
/// says the same thing.
pub(crate) fn mix() -> Mix {
    Mix {
        blend: karakuri_operation::BlendMode::Add,
        residency: karakuri_operation::Residency::Allocated,
    }
}

/// The transition settings nothing else in these tests happens to be: an
/// instant that is not zero and not a whole bar, a length that is not the
/// default and a curve that is not the first in the list — so a conversion
/// filling any of the three in from thin air is visible rather than
/// coincidentally right.
pub(crate) fn transition() -> Transition {
    Transition {
        start: 37.0,
        beats: 6.0,
        curve: karakuri_operation::Curve::Smooth,
        // And a front shape that is neither the mask fixture's nor the
        // first in the list, for the same reason: a wipe that took its
        // shape off the deck instead of off the settings is visible here
        // rather than coincidentally right.
        wipe_kind: karakuri_operation::WipeKind::Linear,
        wipe_angle: 0.75,
    }
}

pub(crate) fn holding(deck: u8) -> Lanes {
    Lanes {
        held: vec![
            (
                0,
                karakuri_operation::LaneTarget::Param {
                    deck: 3,
                    param: karakuri_operation::ParamAt {
                        node: None,
                        key: "twist".to_string(),
                    },
                },
            ),
            (1, karakuri_operation::LaneTarget::Fader { deck }),
        ],
    }
}
