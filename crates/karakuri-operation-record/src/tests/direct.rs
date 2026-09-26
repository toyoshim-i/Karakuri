use super::*;

/// Seven operations need no reading at all, and a caller that has none to
/// give still gets its record. `karakuri-console`'s panel is exactly that
/// caller: three faders, no engine, and `Current::default()`.
#[test]
fn the_faders_records_need_no_reading() {
    assert_eq!(
        records(written(
            &Operation::SetGain { deck: 3, gain: 2.0 },
            &Current::default()
        )),
        vec![Record::Gain {
            slot: DeckSlot(3),
            value: 2.0
        }]
    );
    assert_eq!(
        records(written(
            &Operation::SetOpacity {
                deck: 1,
                opacity: 0.5
            },
            &Current::default()
        )),
        vec![Record::Opacity {
            slot: DeckSlot(1),
            value: 0.5
        }]
    );
    assert_eq!(
        records(written(
            &Operation::SetBlendMode {
                deck: 0,
                blend: BlendMode::Over
            },
            &Current::default()
        )),
        vec![Record::Blend {
            slot: DeckSlot(0),
            mode: "over".to_string()
        }],
        "a fader whose record needed a reading would be a console control that \
             cannot be converted without an engine, which is the seam ADR-0185 opened"
    );
}

/// Verifies that SetAuthority translates directly with node address and layer preserved.
#[test]
fn an_authority_carries_the_node_it_names_and_needs_no_reading() {
    assert_eq!(
        records(written(
            &Operation::SetAuthority {
                deck: 2,
                node: karakuri_operation::NodeAddress {
                    layer: karakuri_operation::Layer::L4,
                    index: 1,
                },
                authority: karakuri_operation::Authority::Suggesting,
            },
            &Current::default()
        )),
        vec![Record::Authority {
            slot: DeckSlot(2),
            at: karakuri_store::record::NodeAddress {
                layer: karakuri_store::record::Layer::L4,
                index: 1,
            },
            authority: "suggesting".to_string(),
        }],
        "an authority landed on another node, another deck slot or another \
             level than the one it named"
    );

    // A `kind Field` node takes one too, which is the arm most easily lost
    // in a translation: it addresses no node in the rendering sense and its
    // params are still an operator's to ride.
    assert_eq!(
        records(written(
            &Operation::SetAuthority {
                deck: 0,
                node: karakuri_operation::NodeAddress {
                    layer: karakuri_operation::Layer::Field,
                    index: 0,
                },
                authority: karakuri_operation::Authority::Manual,
            },
            &Current::default()
        )),
        vec![Record::Authority {
            slot: DeckSlot(0),
            at: karakuri_store::record::NodeAddress {
                layer: karakuri_store::record::Layer::Field,
                index: 0,
            },
            authority: "manual".to_string(),
        }]
    );
}

/// Verifies that AttachSignal and DetachSignal emit Record::Source with and without target source (ADR-0319).
#[test]
fn an_attachment_and_a_take_back_are_one_record_with_and_without_a_source() {
    let attached = records(written(
        &Operation::AttachSignal {
            deck: 3,
            param: karakuri_operation::BindAt {
                layer: karakuri_operation::Layer::L1,
                index: Some(2),
                key: "turbulence".to_string(),
            },
            signal: "energy".to_string(),
            curve: karakuri_operation::Curve::Pow2,
            range: [0.1, 2.4],
        },
        &Current::default(),
    ));
    assert_eq!(
        attached,
        vec![Record::Source {
            slot: DeckSlot(3),
            layer: karakuri_store::record::Layer::L1,
            index: Some(2),
            key: "turbulence".to_string(),
            source: Some(karakuri_store::record::Source {
                signal: "energy".to_string(),
                curve: "pow2".to_string(),
                range: [0.1, 2.4],
                // None denotes the default noise generator configuration.
                noise: None,
            }),
        }],
        "an attachment landed on another node, another deck slot or another shape \
             than the one it named"
    );

    let taken = records(written(
        &Operation::TakeParamBack {
            deck: 3,
            param: karakuri_operation::BindAt {
                layer: karakuri_operation::Layer::L1,
                index: Some(2),
                key: "turbulence".to_string(),
            },
        },
        &Current::default(),
    ));
    assert_eq!(
        taken,
        vec![Record::Source {
            slot: DeckSlot(3),
            layer: karakuri_store::record::Layer::L1,
            index: Some(2),
            key: "turbulence".to_string(),
            source: None,
        }],
        "a take-back is not the attachment's record with its attachment absent"
    );

    // Address preservation: a binding without an index targets the entire
    // layer, keeping wildcard semantics intact.
    let wild = records(written(
        &Operation::TakeParamBack {
            deck: 0,
            param: karakuri_operation::BindAt {
                layer: karakuri_operation::Layer::L4,
                index: None,
                key: "exposure".to_string(),
            },
        },
        &Current::default(),
    ));
    assert_eq!(
        wild,
        vec![Record::Source {
            slot: DeckSlot(0),
            layer: karakuri_store::record::Layer::L4,
            index: None,
            key: "exposure".to_string(),
            source: None,
        }]
    );
}

/// Verifies that WriteParam emits Record::Ride specifying target deck slot (ADR-0280).
#[test]
fn a_knob_turn_carries_the_deck_the_set_is_playing_in() {
    assert_eq!(
        records(written(
            &Operation::WriteParam {
                deck: 2,
                param: karakuri_operation::ParamAt {
                    node: Some(karakuri_operation::NodeAddress {
                        layer: karakuri_operation::Layer::L4,
                        index: 1,
                    }),
                    key: "glow.x".to_string(),
                },
                value: karakuri_operation::ParamValue::Scalar(0.4),
            },
            &Current::default()
        )),
        vec![Record::Ride {
            slot: DeckSlot(2),
            at: Some(karakuri_store::record::NodeAddress {
                layer: karakuri_store::record::Layer::L4,
                index: 1,
            }),
            key: "glow.x".to_string(),
            value: karakuri_store::record::Value::Scalar(0.4),
        }],
        "a write landed on another node, another deck slot or another value \
             than the one it named"
    );
}

/// Verifies that wildcard writes omit node address and pass multi-dimensional values intact (ADR-0268).
#[test]
fn a_wildcard_write_names_no_node_and_therefore_no_layer() {
    assert_eq!(
        records(written(
            &Operation::WriteParam {
                deck: 0,
                param: karakuri_operation::ParamAt {
                    node: None,
                    key: "glow".to_string(),
                },
                value: karakuri_operation::ParamValue::Vec3([0.4, 0.7, 1.0]),
            },
            &Current::default()
        )),
        vec![Record::Ride {
            slot: DeckSlot(0),
            at: None,
            key: "glow".to_string(),
            value: karakuri_store::record::Value::Vec3([0.4, 0.7, 1.0]),
        }],
        "a bare key came back addressed, or a wide value came back expanded"
    );
}

/// Verifies that wire spelling names match store deserialization expectations.
#[test]
fn a_record_carries_the_name_the_store_is_read_back_with() {
    assert_eq!(BlendMode::Add.name(), "add");
    assert_eq!(BlendMode::Over.name(), "over");
    assert_eq!(BlendMode::Max.name(), "max");
    assert_eq!(Residency::Live.name(), "live");
    assert_eq!(Residency::Priming.name(), "priming");
    assert_eq!(Residency::Allocated.name(), "allocated");
    assert_eq!(Sync::Free.name(), "free");
    assert_eq!(Sync::Tempo.name(), "tempo");
    assert_eq!(Sync::Beat.name(), "beat");
    assert_eq!(Tonemap::Clamp.name(), "clamp");
    assert_eq!(Tonemap::Reinhard.name(), "reinhard");
    assert_eq!(Tonemap::Aces.name(), "aces");
    assert_eq!(Tonemap::AgX.name(), "agx");
    assert_eq!(karakuri_operation::Authority::Manual.name(), "manual");
    assert_eq!(
        karakuri_operation::Authority::Suggesting.name(),
        "suggesting"
    );
    assert_eq!(karakuri_operation::Authority::Automatic.name(), "automatic");
    assert_eq!(karakuri_operation::WipeKind::None.name(), "none");
    assert_eq!(karakuri_operation::WipeKind::Linear.name(), "linear");
    assert_eq!(karakuri_operation::WipeKind::Radial.name(), "radial");
}

/// A free-running tempo being stated, which closes the gap P-0090 names:
/// *"`--bpm` exists and the v0.2 vocabulary has no tempo record."* It has
/// one now, and `Record::Tempo`'s own documentation says what shape a
/// statement takes rather than a correction — no shift, no confidence.
#[test]
fn a_free_run_tempo_is_a_correction_that_corrects_nothing() {
    assert_eq!(
        records(written(
            &Operation::SetFreeRunTempo { bpm: 174.0 },
            &Current::default()
        )),
        vec![Record::Tempo {
            bpm: 174.0,
            shift: 0.0,
            confidence: 0.0,
        }],
        "a stated tempo carried a phase shift or a confidence — it is a statement \
             rather than an estimate, and a shift would move a beat nobody moved"
    );
}
