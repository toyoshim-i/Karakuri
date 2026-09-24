use super::*;

/// Verifies that part records round-trip verbatim and are classified as authoring format.
#[test]
fn a_part_round_trips_through_the_line_the_spec_prints() {
    let line = r#"{"t":"part","layer":"L1","path":"drift_shell.kir"}"#;
    let rec = round_trip_verbatim(line);
    assert_eq!(
        rec,
        Record::Part {
            layer: Layer::L1,
            index: 0,
            name: None,
            path: "drift_shell.kir".to_string(),
        }
    );
    assert!(!rec.is_set_state());
    assert!(!rec.is_metadata());
    assert!(rec.is_authoring());

    // Addressed form with explicit layer, index, and name.
    let named =
        r#"{"t":"part","layer":"L4","index":1,"name":"veil","path":"parts/soft_points.kir"}"#;
    assert_eq!(
        round_trip_verbatim(named),
        Record::Part {
            layer: Layer::L4,
            index: 1,
            name: Some("veil".to_string()),
            path: "parts/soft_points.kir".to_string(),
        }
    );
}

/// Verifies that edge records serialize slot as a bare string on the wire.
#[test]
fn an_edge_round_trips_with_slot_as_a_bare_string() {
    let line = r#"{"t":"edge","node":"morph","slot":"far","to":"sphere_shell"}"#;
    let rec = round_trip_verbatim(line);
    assert_eq!(
        rec,
        Record::Edge {
            node: "morph".to_string(),
            slot: InputPort("far".to_string()),
            to: "sphere_shell".to_string(),
        }
    );
}

/// Verifies that gain records serialize deck slot as a bare integer.
#[test]
fn a_gain_round_trips_with_slot_as_a_bare_number() {
    let line = r#"{"t":"gain","slot":2,"value":0.8}"#;
    let rec = round_trip_verbatim(line);
    assert_eq!(
        rec,
        Record::Gain {
            slot: DeckSlot(2),
            value: 0.8,
        }
    );
}

/// Verifies that procedure records serialize slot beside a node address.
#[test]
fn a_procedure_round_trips_with_slot_as_a_bare_number_beside_a_node_address() {
    let hash = "sha256:486779000000000000000000000000000000000000000000000000000000abcd";
    let line = format!(r#"{{"t":"procedure","slot":3,"layer":"L1","proc":"{hash}"}}"#);
    let rec = round_trip_verbatim(&line);
    assert_eq!(
        rec,
        Record::Procedure {
            slot: DeckSlot(3),
            at: NodeAddress {
                layer: Layer::L1,
                index: 0,
            },
            proc_hash: hash.parse().expect("a hash"),
        }
    );
}

/// Verifies that part records and slot records have disjoint syntax.
#[test]
fn a_part_is_not_a_slot_with_a_path_where_the_address_goes() {
    let part = r#"{"t":"part","layer":"L1","path":"drift_shell.kir"}"#;
    let slot = r#"{"t":"slot","layer":"L1","proc":"sha256:00"}"#;
    assert!(matches!(
        serde_json::from_str::<Record>(part).expect("a part parses"),
        Record::Part { .. }
    ));
    assert!(serde_json::from_str::<Record>(
        r#"{"t":"slot","layer":"L1","path":"drift_shell.kir"}"#
    )
    .is_err());
    assert!(serde_json::from_str::<Record>(slot).is_err());
}

/// Verifies that procedure records round-trip and an absent node index defaults to zero.
#[test]
fn a_procedure_round_trips_and_an_absent_index_stays_absent() {
    let hash = "sha256:486779000000000000000000000000000000000000000000000000000000abcd";
    let old = format!(r#"{{"t":"procedure","slot":0,"layer":"L4","proc":"{hash}"}}"#);
    let Record::Procedure { at, slot, .. } = round_trip_verbatim(&old) else {
        panic!("not a procedure");
    };
    assert_eq!((slot, at.index), (DeckSlot(0), 0));

    let stacked = format!(r#"{{"t":"procedure","slot":2,"layer":"L4","index":1,"proc":"{hash}"}}"#);
    let Record::Procedure { at, slot, .. } = round_trip_verbatim(&stacked) else {
        panic!("not a procedure");
    };
    assert_eq!((slot, at.index), (DeckSlot(2), 1));
}

/// Verifies that source records round-trip with and without attachments.
#[test]
fn a_source_round_trips_with_an_attachment_and_without_one() {
    let attached = r#"{"t":"source","slot":0,"layer":"L1","key":"turbulence","source":{"signal":"energy","curve":"pow2","range":[0.1,2.4]}}"#;
    let rec = round_trip_verbatim(attached);
    assert_eq!(
        rec,
        Record::Source {
            slot: DeckSlot(0),
            layer: Layer::L1,
            index: None,
            key: "turbulence".to_string(),
            source: Some(Source {
                signal: "energy".to_string(),
                curve: "pow2".to_string(),
                range: [0.1, 2.4],
                noise: None,
            }),
        }
    );
    assert!(!rec.is_set_state());
    assert!(!rec.is_metadata());

    // Take-back: source field is absent.
    let taken = r#"{"t":"source","slot":2,"layer":"L4","index":1,"key":"exposure"}"#;
    assert_eq!(
        round_trip_verbatim(taken),
        Record::Source {
            slot: DeckSlot(2),
            layer: Layer::L4,
            index: Some(1),
            key: "exposure".to_string(),
            source: None,
        }
    );

    // Generator attachment with noise.
    let noisy = r#"{"t":"source","slot":1,"layer":"L1","key":"spawn_rate","source":{"signal":"noise","curve":"lin","range":[0.0,600.0],"noise":{"kind":"fbm","rate":0.5,"stream":0,"octaves":3}}}"#;
    let Record::Source {
        source: Some(source),
        ..
    } = round_trip_verbatim(noisy)
    else {
        panic!("not an attachment");
    };
    assert_eq!(
        source.noise,
        Some(BindNoise {
            kind: "fbm".to_string(),
            rate: 0.5,
            stream: 0,
            octaves: 3,
        })
    );
}

/// Verifies that authority records round-trip and omit zero index.
#[test]
fn an_authority_round_trips_and_an_absent_index_stays_absent() {
    let first = r#"{"t":"authority","slot":0,"layer":"L1","authority":"manual"}"#;
    let rec = round_trip_verbatim(first);
    assert_eq!(
        rec,
        Record::Authority {
            slot: DeckSlot(0),
            at: NodeAddress {
                layer: Layer::L1,
                index: 0,
            },
            authority: "manual".to_string(),
        },
        "an absent index is the first node of that layer"
    );
    assert!(
        !rec.is_set_state(),
        "authority is the session's: a Set does not know which agent is watching it, \
         and a Set file carrying one would hand that node over wherever it was loaded"
    );
    assert!(!rec.is_metadata());

    let second = r#"{"t":"authority","slot":2,"layer":"L4","index":1,"authority":"suggesting"}"#;
    assert_eq!(
        round_trip_verbatim(second),
        Record::Authority {
            slot: DeckSlot(2),
            at: NodeAddress {
                layer: Layer::L4,
                index: 1,
            },
            authority: "suggesting".to_string(),
        },
        "the second renderer of deck slot 2"
    );

    // A `kind Field` node takes one: it draws nothing and its params are
    // still an operator's to ride, which is why `Layer` has the arm.
    assert_eq!(
        round_trip_verbatim(
            r#"{"t":"authority","slot":1,"layer":"Field","authority":"automatic"}"#
        ),
        Record::Authority {
            slot: DeckSlot(1),
            at: NodeAddress {
                layer: Layer::Field,
                index: 0,
            },
            authority: "automatic".to_string(),
        }
    );
}

/// Verifies that multiple `slot` records with distinct node indices serialize and fold independently.
#[test]
fn a_slot_round_trips_and_an_absent_index_or_name_stays_absent() {
    let hash = "sha256:9c1b04000000000000000000000000000000000000000000000000000000abcd";
    let old = format!(r#"{{"t":"slot","layer":"L4","proc":"{hash}"}}"#);
    let Record::Slot { at, name, .. } = round_trip_verbatim(&old) else {
        panic!("not a slot");
    };
    assert_eq!(at.index, 0);
    assert_eq!(
        name, None,
        "a slot from before names existed is unnamed, not named nothing"
    );

    let stacked = format!(r#"{{"t":"slot","layer":"L4","index":2,"proc":"{hash}"}}"#);
    let Record::Slot { at, .. } = round_trip_verbatim(&stacked) else {
        panic!("not a slot");
    };
    assert_eq!(at.index, 2);

    // The spec's own example of a named source, verbatim — so the field
    // order is asserted here too, and a name written after `proc` would be
    // a file this reader wrote and the specification did not print.
    let named = format!(r#"{{"t":"slot","layer":"L1","index":1,"name":"veil","proc":"{hash}"}}"#);
    let Record::Slot { at, name, .. } = round_trip_verbatim(&named) else {
        panic!("not a slot");
    };
    assert_eq!((at.index, name.as_deref()), (1, Some("veil")));
}

#[test]
fn tick_round_trips() {
    assert_eq!(
        round_trip(r#"{"t":"tick","steps":1}"#),
        Record::Tick { steps: 1 }
    );
}

/// Verifies that capacity records round-trip and an absent node index defaults to zero.
#[test]
fn a_capacity_addresses_a_geometry_and_an_absent_index_stays_absent() {
    assert_eq!(
        round_trip_verbatim(r#"{"t":"capacity","layer":"L1","value":524288}"#),
        Record::Capacity {
            at: NodeAddress {
                layer: Layer::L1,
                index: 0,
            },
            value: 524288
        }
    );
    // Explicit secondary index.
    assert_eq!(
        round_trip_verbatim(r#"{"t":"capacity","layer":"L1","index":1,"value":65536}"#),
        Record::Capacity {
            at: NodeAddress {
                layer: Layer::L1,
                index: 1,
            },
            value: 65536
        }
    );
}

/// Verifies that seed derivation salts are assigned per node rather than by order in `--set` flags.
#[test]
fn a_seed_addresses_a_source_and_an_absent_index_stays_absent() {
    assert_eq!(
        round_trip_verbatim(r#"{"t":"seed","stream":"L1","value":19274}"#),
        Record::Seed {
            stream: Layer::L1,
            index: 0,
            value: 19274
        }
    );
    assert_eq!(
        round_trip_verbatim(r#"{"t":"seed","stream":"L1","index":1,"value":4}"#),
        Record::Seed {
            stream: Layer::L1,
            index: 1,
            value: 4
        }
    );
}

/// Verifies that param records round-trip addressed and as wildcards.
#[test]
fn a_param_round_trips_addressed_and_as_a_wildcard() {
    // Addressed: `at` is `Some`, and `index` is written because it says
    // something.
    assert_eq!(
        round_trip_verbatim(r#"{"t":"param","layer":"L4","index":1,"key":"glow.x","value":0.4}"#),
        Record::Param {
            at: Some(NodeAddress {
                layer: Layer::L4,
                index: 1,
            }),
            key: "glow.x".to_string(),
            value: Value::Scalar(0.4),
        }
    );

    // Node 0, addressed — `index` is written explicitly even though it is
    // zero, because that is what tells it apart from the wildcard below;
    // `at` is `Some` because a node was named.
    assert_eq!(
        round_trip_verbatim(r#"{"t":"param","layer":"L1","index":0,"key":"radius","value":2.6}"#),
        Record::Param {
            at: Some(NodeAddress {
                layer: Layer::L1,
                index: 0,
            }),
            key: "radius".to_string(),
            value: Value::Scalar(2.6),
        }
    );

    // The wildcard: `layer` is still on the wire — the placeholder every
    // writer before this address existed wrote, and every writer since —
    // but `index` is absent, and that alone is what makes `at` `None`.
    assert_eq!(
        round_trip_verbatim(r#"{"t":"param","layer":"L1","key":"exposure","value":2.0}"#),
        Record::Param {
            at: None,
            key: "exposure".to_string(),
            value: Value::Scalar(2.0),
        }
    );
}

#[test]
fn unknown_records_are_ignored_not_rejected() {
    // Forward compatibility: a newer engine's record must not break an
    // older reader.
    assert_eq!(
        serde_json::from_str::<Record>(r#"{"t":"phrase","at":4.0}"#).unwrap(),
        Record::Unknown
    );
}

#[test]
fn a_bind_without_a_noise_object_round_trips_and_stays_without_one() {
    // The field is absent rather than `null` on the way out: a Set file is
    // read by humans and generated by LLMs, and a `"noise":null` on every
    // ordinary binding teaches both that it is a thing to fill in.
    let rec = round_trip(
        r#"{"t":"bind","layer":"L1","key":"turbulence","signal":"energy","curve":"pow2","range":[0.1,2.4]}"#,
    );
    assert_eq!(
        rec,
        Record::Bind {
            layer: Layer::L1,
            index: None,
            key: "turbulence".into(),
            signal: "energy".into(),
            curve: "pow2".into(),
            range: [0.1, 2.4],
            noise: None,
        }
    );
    assert!(!serde_json::to_string(&rec).unwrap().contains("noise"));
}

#[test]
fn a_noise_bind_round_trips_with_its_generator() {
    // The example out of `docs/ir-spec.md`'s "Binding noise", verbatim.
    let rec = round_trip(
        r#"{"t":"bind","layer":"L1","key":"spawn_rate","signal":"noise",
            "noise":{"kind":"perlin","rate":0.5,"stream":3},"curve":"lin","range":[4000,16000]}"#,
    );
    let Record::Bind { noise: Some(n), .. } = rec else {
        panic!("expected a bind carrying a noise generator");
    };
    assert_eq!(n.kind, "perlin");
    assert_eq!(n.rate, 0.5);
    assert_eq!(n.stream, 3);
    // Absent in the spec's own example, so it has to have a default or the
    // example does not decode.
    assert_eq!(n.octaves, 4);
}

#[test]
fn an_empty_noise_object_is_the_default_generator() {
    // Field by field: a generator an LLM under-specified is one that runs,
    // not one that is refused. `{}` is the extreme case of that.
    let rec: Record = serde_json::from_str(
        r#"{"t":"bind","layer":"L1","key":"k","signal":"noise","curve":"lin","range":[0,1],"noise":{}}"#,
    )
    .expect("parse");
    let Record::Bind { noise: Some(n), .. } = rec else {
        panic!("expected a bind carrying a noise generator");
    };
    assert_eq!(n, BindNoise::default());
}

/// A decoded frame reproduces the values it was emitted with. That is the whole
/// promise of putting the measurement in the stream: replay writes the same
/// uniforms live did, so it has to be the same numbers.
#[test]
fn an_audio_frame_round_trips_with_every_value_it_carried() {
    let rec = round_trip(
        r#"{"t":"audio","energy":0.42,"onset":0.75,
            "bands":[0.9,0.4,0.2,0.11,0.05,0.02,0.01,0.0],"confidence":1.0}"#,
    );
    assert_eq!(
        rec,
        Record::Audio {
            energy: 0.42,
            onset: 0.75,
            bands: vec![0.9, 0.4, 0.2, 0.11, 0.05, 0.02, 0.01, 0.0],
            confidence: 1.0,
        }
    );
}

/// A silent room is a measurement and reads as one: zeroes at full confidence.
/// A dead input is the same zeroes at no confidence. Two different lines, and a
/// decoder that lost the difference would make an unplugged interface look like
/// a quiet one.
#[test]
fn silence_and_absence_are_different_lines() {
    let silent =
        round_trip(r#"{"t":"audio","energy":0.0,"onset":0.0,"bands":[0.0,0.0],"confidence":1.0}"#);
    let absent =
        round_trip(r#"{"t":"audio","energy":0.0,"onset":0.0,"bands":[0.0,0.0],"confidence":0.0}"#);
    assert_ne!(silent, absent);
    let Record::Audio { confidence, .. } = silent else {
        panic!("expected an audio record");
    };
    assert_eq!(confidence, 1.0);
}

/// The band count is the array's length, so a stream from something that
/// measures more bands than this reader knows about still decodes rather than
/// failing — the same forward compatibility the unknown-`t` rule is for, one
/// level down.
#[test]
fn a_band_count_this_reader_does_not_expect_still_decodes() {
    let rec: Record = serde_json::from_str(
        r#"{"t":"audio","energy":0.5,"onset":0.0,"bands":[0.1,0.2,0.3,0.4,0.5,0.6,0.7,0.8,0.9,1.0,0.1,0.2],"confidence":1.0}"#,
    )
    .expect("parse");
    let Record::Audio { bands, .. } = rec else {
        panic!("expected an audio record");
    };
    assert_eq!(bands.len(), 12);
}

/// A correction round-trips exactly, because replay applies it verbatim: a
/// shift that decoded to a different number would put the beat somewhere else
/// than the live run did.
#[test]
fn a_tempo_correction_round_trips() {
    assert_eq!(
        round_trip(r#"{"t":"tempo","bpm":128.25,"shift":-0.0125,"confidence":0.82}"#),
        Record::Tempo {
            bpm: 128.25,
            shift: -0.0125,
            confidence: 0.82,
        }
    );
    // A free-running tempo being stated: no shift, no claim.
    assert_eq!(
        round_trip(r#"{"t":"tempo","bpm":120.0,"shift":0.0,"confidence":0.0}"#),
        Record::Tempo {
            bpm: 120.0,
            shift: 0.0,
            confidence: 0.0,
        }
    );
}

/// Verifies that metadata records round-trip through JSON serialization verbatim.
#[test]
fn a_meta_round_trips_through_the_line_the_spec_prints() {
    let hash = Hash::of(b"proc drift_shell { kind L1 }");
    let line = format!(r#"{{"t":"meta","hash":"{hash}","name":"drift_shell","kind":"L1","v":1}}"#);
    let rec = round_trip_verbatim(&line);
    assert_eq!(
        rec,
        Record::Meta {
            hash,
            name: "drift_shell".to_string(),
            kind: Layer::L1,
            v: 1,
        }
    );
    assert!(rec.is_metadata());
    assert!(!rec.is_set_state());
}

/// Verifies that param_decl records round-trip through JSON serialization verbatim.
#[test]
fn a_param_decl_round_trips_through_the_line_the_spec_prints() {
    let line =
        r#"{"t":"param_decl","key":"radius","type":"float","min":0.1,"max":8.0,"default":2.0}"#;
    let rec = round_trip_verbatim(line);
    assert_eq!(
        rec,
        Record::ParamDecl {
            key: "radius".to_string(),
            ty: "float".to_string(),
            min: 0.1,
            max: 8.0,
            default: Some(2.0),
        }
    );
    assert!(rec.is_metadata());
    assert!(!rec.is_set_state());
}

/// Verifies that capacity_decl records round-trip through JSON serialization verbatim.
#[test]
fn a_capacity_decl_round_trips_through_the_line_the_spec_prints() {
    let line = r#"{"t":"capacity_decl","min":65536,"max":1048576,"default":262144}"#;
    let rec = round_trip_verbatim(line);
    assert_eq!(
        rec,
        Record::CapacityDecl {
            min: 65536,
            max: 1048576,
            default: 262144,
        }
    );
    assert!(rec.is_metadata());
    assert!(!rec.is_set_state());
}

/// Verifies that emit records round-trip through JSON serialization verbatim.
#[test]
fn an_emit_round_trips_through_the_line_the_spec_prints() {
    let line = r#"{"t":"emit","attrs":["position","velocity","age"]}"#;
    let rec = round_trip_verbatim(line);
    assert_eq!(
        rec,
        Record::Emit {
            attrs: vec![
                "position".to_string(),
                "velocity".to_string(),
                "age".to_string(),
            ],
        }
    );
    assert!(rec.is_metadata());
    assert!(!rec.is_set_state());
}

#[test]
fn ticks_are_not_state() {
    assert!(!Record::Tick { steps: 1 }.is_set_state());
    // Nor is anything else a frame measured or decided.
    assert!(!Record::Audio {
        energy: 0.5,
        onset: 0.0,
        bands: vec![0.1],
        confidence: 1.0
    }
    .is_set_state());
    assert!(!Record::Tempo {
        bpm: 128.0,
        shift: 0.0,
        confidence: 0.9
    }
    .is_set_state());
    assert!(Record::Set {
        id: "drift_01".into(),
        v: 1
    }
    .is_set_state());
}

#[test]
fn value_helper_methods_and_variants() {
    let s = Value::Scalar(1.5);
    assert_eq!(s.len(), 1);
    assert_eq!(s.components(), &[1.5]);
    assert_eq!(s.as_slice(), &[1.5]);
    assert_eq!(s.get(0), Some(1.5));
    assert_eq!(s.get(1), None);
    assert!(!s.is_empty());

    let v2 = Value::Vec2([1.0, 2.0]);
    assert_eq!(v2.len(), 2);
    assert_eq!(v2.components(), &[1.0, 2.0]);
    assert_eq!(v2.get(0), Some(1.0));
    assert_eq!(v2.get(1), Some(2.0));
    assert_eq!(v2.get(2), None);

    let v3 = Value::Vec3([1.0, 2.0, 3.0]);
    assert_eq!(v3.len(), 3);
    assert_eq!(v3.components(), &[1.0, 2.0, 3.0]);
    assert_eq!(v3.get(2), Some(3.0));
    assert_eq!(v3.get(3), None);

    let v4 = Value::Vec4([1.0, 2.0, 3.0, 4.0]);
    assert_eq!(v4.len(), 4);
    assert_eq!(v4.components(), &[1.0, 2.0, 3.0, 4.0]);
    assert_eq!(v4.get(3), Some(4.0));
    assert_eq!(v4.get(4), None);

    let col = Value::Color([0.1, 0.2, 0.3, 1.0]);
    assert_eq!(col.len(), 4);
    assert_eq!(col.components(), &[0.1, 0.2, 0.3, 1.0]);
    assert_eq!(col.get(0), Some(0.1));

    // Serde round-trip
    let serialized = serde_json::to_string(&v4).unwrap();
    assert_eq!(serialized, "[1.0,2.0,3.0,4.0]");
    let deserialized: Value = serde_json::from_str(&serialized).unwrap();
    assert_eq!(deserialized, v4);

    let col_serialized = serde_json::to_string(&col).unwrap();
    assert_eq!(col_serialized, "[0.1,0.2,0.3,1.0]");
}

#[test]
fn a_header_round_trips_through_the_line_the_spec_prints() {
    let line = r#"{"t":"header","version":2}"#;
    let rec = round_trip_verbatim(line);
    assert_eq!(rec, Record::Header { version: 2 });
    assert!(rec.is_set_state());
    assert!(!rec.is_metadata());
    assert!(!rec.is_authoring());
}
