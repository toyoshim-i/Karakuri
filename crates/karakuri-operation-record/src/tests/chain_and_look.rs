use super::*;

/// Verifies that SetExposure preserves active tonemap and white point from Current::look (ADR-0192).
#[test]
fn an_exposure_keeps_the_operator_that_is_running() {
    let current = Current {
        look: Some(look()),
        ..Current::default()
    };
    let written = written(&Operation::SetExposure { exposure: 2.5 }, &current);
    assert_eq!(
        records(written),
        vec![Record::Look {
            op: "agx".to_string(),
            exposure: 2.5,
            white_point: 4.0,
        }],
        "an exposure change rewrote the tone map operator or the white point — \
             the record carries all three and only the exposure was asked for"
    );
}

/// The other half, and it fails apart from the first: `t` names an operator
/// and says nothing about the level going into it, so the exposure and the
/// white point come from the reading.
#[test]
fn a_tone_map_keeps_the_exposure_that_is_running() {
    let current = Current {
        look: Some(look()),
        ..Current::default()
    };
    let written = written(
        &Operation::SetTonemap {
            tonemap: Tonemap::Reinhard,
        },
        &current,
    );
    assert_eq!(
        records(written),
        vec![Record::Look {
            op: "reinhard".to_string(),
            exposure: 0.25,
            white_point: 4.0,
        }],
        "a tone map change rewrote the exposure or the white point — the record \
             carries all three and only the operator was asked for"
    );
}

/// The chain that is running fills in the slots nobody moved. A press on
/// one slot leaves the others where they stand.
#[test]
fn a_slot_moved_keeps_every_other_slot_that_is_running() {
    let current = Current {
        master_chain: Some(chain()),
        ..Current::default()
    };
    let written = written(
        &Operation::SetChainParam {
            at: 1,
            param: karakuri_operation::ChainParam::Declared {
                key: "amount".into(),
                value: 0.9,
            },
        },
        &current,
    );
    assert_eq!(
        records(written),
        vec![chain_record(vec![
            slot("sha256:feedback", Some("exit"), 0.34),
            slot("sha256:bloom", None, 0.9),
            slot("sha256:rgb_shift", None, 0.25),
        ])],
        "a press on one slot rewrote a slot it did not name — the record carries the \
             whole list and only the slot at that position was asked for"
    );
}

/// A cut is set on the slot and not on the chain: it is one of the things a
/// slot is set to, and it reaches the record through the same row.
#[test]
fn a_cut_is_set_on_the_slot_it_names_and_keeps_that_slots_values() {
    let current = Current {
        master_chain: Some(chain()),
        ..Current::default()
    };
    let written = written(
        &Operation::SetChainParam {
            at: 0,
            param: karakuri_operation::ChainParam::Cut(karakuri_operation::Cut::Mix),
        },
        &current,
    );
    assert_eq!(
        records(written),
        vec![chain_record(vec![
            slot("sha256:feedback", Some("mix"), 0.34),
            slot("sha256:bloom", None, 0.6),
            slot("sha256:rgb_shift", None, 0.25),
        ])]
    );
}

/// An add lands at the end and carries the values its procedure declares,
/// which is no `params` at all: the record carries what was asked for, and
/// a parameter nobody moved runs at the declaration's own default.
#[test]
fn an_add_appends_a_slot_with_its_cut_and_no_values() {
    let current = Current {
        master_chain: Some(chain()),
        ..Current::default()
    };
    let written = written(
        &Operation::AddChainEffect {
            procedure: "sha256:other".into(),
            cut: Some(karakuri_operation::Cut::Mix),
        },
        &current,
    );
    assert_eq!(
        records(written),
        vec![chain_record(vec![
            slot("sha256:feedback", Some("exit"), 0.34),
            slot("sha256:bloom", None, 0.6),
            slot("sha256:rgb_shift", None, 0.25),
            ChainSlot {
                procedure: "sha256:other".into(),
                cut: Some("mix".into()),
                params: Default::default(),
            },
        ])]
    );
}

/// A remove takes the slot out and the slots after it move up.
#[test]
fn a_remove_takes_one_slot_out_and_closes_the_gap() {
    let current = Current {
        master_chain: Some(chain()),
        ..Current::default()
    };
    let written = written(&Operation::RemoveChainEffect { at: 0 }, &current);
    assert_eq!(
        records(written),
        vec![chain_record(vec![
            slot("sha256:bloom", None, 0.6),
            slot("sha256:rgb_shift", None, 0.25),
        ])]
    );
}

/// A position the chain has not got writes nothing and says so.
#[test]
fn a_position_the_chain_has_not_got_is_owed_rather_than_appended() {
    let current = Current {
        master_chain: Some(chain()),
        ..Current::default()
    };
    for operation in [
        Operation::SetChainParam {
            at: 3,
            param: karakuri_operation::ChainParam::Declared {
                key: "amount".into(),
                value: 0.5,
            },
        },
        Operation::RemoveChainEffect { at: 3 },
    ] {
        assert_eq!(
            written(&operation, &current),
            Written::Owed(Owed::NotInChain),
            "{operation:?} wrote a chain for a slot that is not in it"
        );
    }
}

/// A chain that was not read is said, never defaulted — the look pair's
/// rule at the row below it.
#[test]
fn a_chain_operation_with_no_chain_read_is_owed_it_rather_than_given_a_default() {
    for operation in [
        Operation::SetChainParam {
            at: 0,
            param: karakuri_operation::ChainParam::Cut(karakuri_operation::Cut::Mix),
        },
        Operation::AddChainEffect {
            procedure: "sha256:other".into(),
            cut: None,
        },
        Operation::RemoveChainEffect { at: 0 },
    ] {
        assert_eq!(
            written(&operation, &Current::default()),
            Written::Owed(Owed::NotRead(Reading::MasterChain)),
            "{operation:?} invented a chain nobody read"
        );
    }
}

/// Verifies that missing context in Current returns Written::Owed rather than inventing defaults.
#[test]
fn a_reading_that_was_not_taken_is_owed_rather_than_guessed() {
    assert_eq!(
        written(
            &Operation::SetExposure { exposure: 2.5 },
            &Current::default()
        ),
        Written::Owed(Owed::NotRead(Reading::Look)),
        "an exposure with no look read came back with a record — which means the \
             operator in it was invented"
    );
    assert_eq!(
        written(
            &Operation::ScrubDeck {
                deck: 1,
                beats: 0.25
            },
            &Current::default()
        ),
        Written::Owed(Owed::NotRead(Reading::Transport)),
        "a scrub with no transport read came back with a record — which means the \
             scrub it moved from was invented"
    );
}

/// Verifies that SetMasterOut translates directly from operation arguments without external state.
#[test]
fn a_master_out_is_written_from_the_operation_alone() {
    assert_eq!(
        records(written(
            &Operation::SetMasterOut { out: 0.25 },
            &Current::default()
        )),
        vec![Record::MasterOut { value: 0.25 }],
        "the master out asked for a reading, or wrote something other than the level it \
             was handed — it names no deck and completes no record, so `Current::default()` \
             is everything it needs"
    );
    // The look that is running is beside the point rather than absent, so
    // a conversion that had started reading one would be caught writing a
    // different record here as well as the same one above.
    let current = Current {
        look: Some(look()),
        ..Current::default()
    };
    assert_eq!(
        records(written(&Operation::SetMasterOut { out: 3.0 }, &current)),
        vec![Record::MasterOut { value: 3.0 }],
        "a master out of 3.0 was clamped, or the look that is running reached the record \
             — the level is open above 1.0 and the engine is where that is decided"
    );
}
