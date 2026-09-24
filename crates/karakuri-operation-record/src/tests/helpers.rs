use super::*;

/// Test fixture for master chain with distinct procedure, cut, and parameter values.
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

/// Test fixture for mask state with non-default shape, angle, position, and softness.
pub(crate) fn mask() -> Mask {
    Mask {
        kind: karakuri_operation::WipeKind::Radial,
        angle: 1.25,
        position: 0.4,
        softness: 0.02,
    }
}

/// Test fixture for deck mix state with non-live residency and non-default blend mode.
pub(crate) fn mix() -> Mix {
    Mix {
        blend: karakuri_operation::BlendMode::Add,
        residency: karakuri_operation::Residency::Allocated,
    }
}

/// Test fixture for transition settings with non-default timing, curve, and wipe parameters.
pub(crate) fn transition() -> Transition {
    Transition {
        start: 37.0,
        beats: 6.0,
        curve: karakuri_operation::Curve::Smooth,
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
