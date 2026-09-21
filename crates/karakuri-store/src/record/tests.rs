use super::*;
use crate::hash::Hash;

/// The wire line the spec documents, parsed and written back. Every other
/// record has one of these; without it a rename or a reordered field breaks
/// every recorded session and nothing says so.
#[test]
fn a_transition_round_trips_through_the_line_the_spec_prints() {
    let line = r#"{"t":"transition","slot":0,"control":"opacity","to":0.0,"start":64.0,"beats":8.0,"curve":"smooth"}"#;
    let rec = round_trip(line);
    assert_eq!(
        rec,
        Record::Transition {
            slot: DeckSlot(0),
            control: "opacity".to_string(),
            to: 0.0,
            start: 64.0,
            beats: 8.0,
            curve: "smooth".to_string(),
        }
    );
    // And it is the session's rather than a Set's, for both reasons at
    // once: it is the deck's, and it is an event rather than state.
    assert!(!rec.is_set_state());
}

/// A selection round-trips through the line the spec prints, bytes and all.
///
/// `round_trip_verbatim` rather than `round_trip`, because the whole shape of
/// this record is what it does *not* carry: a `beats` or a `curve` added later
/// with a serde default would still compare equal to itself and would change
/// every session ever recorded. A selection is a cut and has neither — see
/// [`Record::Select`].
#[test]
fn a_selection_round_trips_through_the_line_the_spec_prints() {
    let line = r#"{"t":"select","slot":1,"renderer":2,"start":64.0}"#;
    let rec = round_trip_verbatim(line);
    assert_eq!(
        rec,
        Record::Select {
            slot: DeckSlot(1),
            renderer: 2,
            start: 64.0,
        }
    );
    // The session's, on both of the counts `is_set_state` separates: it is
    // the deck's, and it is an event rather than state. A Set file that
    // carried one would also be claiming a layering it cannot record.
    assert!(!rec.is_set_state());
    assert!(!rec.is_metadata());
}

/// A `ride` round-trips through the line a session writes, bytes and all, in
/// both of its two shapes: addressed, and the wildcard.
///
/// `round_trip_verbatim` for [`Record::Select`]'s reason with one more of its
/// own — this record's whole shape is what it leaves out. `at` absent is *every
/// node declaring the key*, and an `at` acquiring a serde default would turn
/// every wildcard ever recorded into a write addressed at `L1:0` while still
/// comparing equal to itself. The bare wildcard line is therefore the one that
/// has to survive untouched.
#[test]
fn a_ride_round_trips_addressed_and_as_a_wildcard() {
    // Addressed, and at a node that is not 0 — the index is written only
    // where it says something, so an addressed write at node 0 and a
    // wildcard are told apart by `at` and never by `index`.
    let rec = round_trip_verbatim(
        r#"{"t":"ride","slot":2,"at":{"layer":"L4","index":1},"key":"glow.x","value":0.4}"#,
    );
    assert_eq!(
        rec,
        Record::Ride {
            slot: DeckSlot(2),
            at: Some(NodeAddress {
                layer: Layer::L4,
                index: 1
            }),
            key: "glow.x".to_string(),
            value: Value::Scalar(0.4),
        }
    );
    // The session's, and for `procedure`'s and `authority`'s reason rather
    // than `select`'s: it is state, and it is the deck's. A Set file that
    // carried one would restore a knob position wherever it was loaded.
    assert!(!rec.is_set_state());
    assert!(!rec.is_metadata());

    // Node 0 of a layer, addressed. The `index` is absent because it is
    // zero, and the `at` is present because a node was named — which is the
    // distinction `Record::Param` cannot draw at all.
    let rec = round_trip_verbatim(
        r#"{"t":"ride","slot":0,"at":{"layer":"L1"},"key":"radius","value":2.6}"#,
    );
    assert_eq!(
        rec,
        Record::Ride {
            slot: DeckSlot(0),
            at: Some(NodeAddress {
                layer: Layer::L1,
                index: 0
            }),
            key: "radius".to_string(),
            value: Value::Scalar(2.6),
        }
    );

    // The wildcard: no node, and so no layer either.
    let rec = round_trip_verbatim(r#"{"t":"ride","slot":1,"key":"exposure","value":2.0}"#);
    assert_eq!(
        rec,
        Record::Ride {
            slot: DeckSlot(1),
            at: None,
            key: "exposure".to_string(),
            value: Value::Scalar(2.0),
        }
    );

    // A wide value on one line, which is what a model writes and what a
    // reader with the Set in hand expands — `param`'s rule exactly
    // (ADR-0268).
    let rec = round_trip_verbatim(r#"{"t":"ride","slot":0,"key":"glow","value":[0.4,0.7,1.0]}"#);
    assert_eq!(
        rec,
        Record::Ride {
            slot: DeckSlot(0),
            at: None,
            key: "glow".to_string(),
            value: Value::Vec3([0.4, 0.7, 1.0]),
        }
    );
}

/// A merge round-trips through the line the spec prints, bytes and all, with
/// and without the one field it has.
///
/// `round_trip_verbatim` rather than `round_trip`, because the shape of this
/// record is mostly what it does *not* carry. A `gain`, an `opacity`, a `blend`
/// or a `mask` added later with a serde default would compare equal to itself
/// and would rewrite every composited Set ever saved — and the bare
/// `{"t":"merge"}` is the line a Set nobody has selected in writes, so it is
/// the one that has to survive untouched.
#[test]
fn a_merge_round_trips_and_an_absent_live_stays_absent() {
    // A composited Set nobody has selected in: every input live, which is
    // `mix::Input::default()`, so there is nothing for `live` to say.
    let rec = round_trip_verbatim(r#"{"t":"merge"}"#);
    assert_eq!(
        rec,
        Record::Merge { live: None },
        "the presence of the record is the whole statement"
    );

    // And a variant pool with a renderer chosen in it, which is the fact
    // `Record::Select` could not leave behind before this record existed.
    assert_eq!(
        round_trip_verbatim(r#"{"t":"merge","live":1}"#),
        Record::Merge { live: Some(1) }
    );

    // Renderer 0 is a choice somebody made and is written, where an absent
    // `live` is nobody having chosen. The two are different pictures — one
    // renderer against all of them — so the field must not be skipped at
    // zero the way an `index` is.
    assert_eq!(
        round_trip_verbatim(r#"{"t":"merge","live":0}"#),
        Record::Merge { live: Some(0) }
    );
}

/// A merge is what a Set *is*, and belongs in a Set file.
///
/// The assertion this record exists to make. A composited Set saved and loaded
/// back overdrew, because the layering was the one thing a Set knew about
/// itself that the file could not say — so `Set::select_renderer` came back
/// with nothing to select between and a saved variant pool was an unselectable
/// one.
#[test]
fn a_merge_is_set_state_and_not_an_artifacts_declaration() {
    let rec = Record::Merge { live: Some(2) };
    assert!(
        rec.is_set_state(),
        "the layering is what a Set is, so `Store::write_set` has to take it"
    );
    // And it is not a card's: nothing about it is something a compile pass
    // reads off a `.kir`. An L5 has no procedure to compile.
    assert!(!rec.is_metadata());
}

/// The wire line the spec prints, parsed and written back.
#[test]
fn a_mask_round_trips_through_the_line_the_spec_prints() {
    let line = r#"{"t":"mask","slot":1,"kind":"linear","angle":0.0,"position":0.5,"softness":0.1}"#;
    let rec = round_trip(line);
    assert_eq!(
        rec,
        Record::Mask {
            slot: DeckSlot(1),
            kind: "linear".to_string(),
            angle: 0.0,
            position: 0.5,
            softness: 0.1,
        }
    );
    assert!(!rec.is_set_state());
}

/// The wire line the spec prints, parsed and written back.
#[test]
fn a_canvas_round_trips_through_the_line_the_spec_prints() {
    let line = r#"{"t":"canvas","width":1920,"height":1080}"#;
    let rec = round_trip(line);
    assert_eq!(
        rec,
        Record::Canvas {
            width: 1920,
            height: 1080,
        }
    );
    // **The one that would be most tempting to put in a Set file**, and the
    // one it would do the most damage in: a Set renders at whatever size it
    // is handed, so a Set file carrying a canvas would resize every *other*
    // Set in the deck by being loaded.
    assert!(!rec.is_set_state());
}

/// The wire line the spec prints, parsed and written back.
///
/// And the one assertion this record exists to make: it is not Set state, so
/// `Store::write_set` refuses it. A `save` folded into a Set file would be a
/// file claiming, every time it was opened, that a save had just happened —
/// which is the outside-the-stream effect a replay is defined not to perform,
/// arriving by the one door nobody watches.
#[test]
fn a_save_round_trips_through_the_line_the_spec_prints() {
    let line = r#"{"t":"save","slot":1,"id":"20260816-143052-271"}"#;
    let rec = round_trip(line);
    assert_eq!(
        rec,
        Record::Save {
            slot: DeckSlot(1),
            id: "20260816-143052-271".to_string(),
        }
    );
    assert!(!rec.is_set_state());
}

/// The scrub round-trips through the line the spec prints, bytes and all — and
/// it is `scrub_beats` on the wire, not just in Rust.
///
/// This record had no round-trip test at all until the rename, which is exactly
/// the gap that lets a serialised field's writer and reader drift apart in
/// silence: a half-applied rename compiles in neither direction here, but a
/// `serde(rename)` on one side and not the other compiles in both and only
/// fails on disk.
///
/// `round_trip_verbatim` rather than `round_trip`, on [`Record::Procedure`]'s
/// terms: every field is required and written in declaration order, so a field
/// appearing, vanishing or moving is a changed stream and only the byte
/// comparison sees it.
#[test]
fn a_transport_round_trips_through_the_line_the_spec_prints() {
    let line = r#"{"t":"transport","slot":0,"sync":"beat","anchor_bpm":126.0,"scrub_beats":-0.25}"#;
    let rec = round_trip_verbatim(line);
    assert_eq!(
        rec,
        Record::Transport {
            slot: DeckSlot(0),
            sync: "beat".to_string(),
            anchor_bpm: 126.0,
            scrub_beats: -0.25,
        }
    );
    assert!(
        !rec.is_set_state(),
        "a transport is session state, not something a Set file may carry"
    );
}

/// The old spelling fails loudly, and that is the whole bill.
///
/// `offset_beats` was renamed to `scrub_beats` on disk rather than kept behind
/// a `serde(rename)`, so a stream written before the rename does not replay.
/// What this pins is *how* it does not replay: serde has no default for the
/// field, so the line is a parse error naming the missing field —
/// [`crate::store::StoreError::Record`] with its line number — and not a scrub
/// silently read as zero, which is the failure mode
/// `docs/adr/0134-the-metadata-preview-becomes-thumbnail.md` was written about.
/// An unknown key is dropped without comment; a *missing required* one is not,
/// and that asymmetry is what makes this rename affordable.
#[test]
fn a_stream_with_the_old_offset_beats_fails_its_line_rather_than_reading_a_zero_scrub() {
    let old = r#"{"t":"transport","slot":0,"sync":"beat","anchor_bpm":126.0,"offset_beats":-0.25}"#;
    let err = serde_json::from_str::<Record>(old)
        .expect_err("the pre-rename spelling parsed — the rename left an alias behind");
    assert!(
        err.to_string().contains("scrub_beats"),
        "the error does not name the field that is missing: {err}"
    );
}

/// The four-field `master_chain` is refused, and the refusal names the shape it
/// found.
///
/// The record changed shape under the same `t` between 2026-09-09 and M5.16 —
/// four scalars became a list — and `docs/adr/0340-…` refused reading both, on
/// P-0085: a compatibility cost before v1 is a bill and not an argument, and
/// this bill is a handful of sessions against a decoder carrying two shapes for
/// the length of the project. What it owed instead is this: not a silent
/// default, which here would play an empty chain where the session had three
/// passes, but a refusal that says which shape it met.
#[test]
fn the_four_field_master_chain_is_refused_naming_the_shape() {
    let old = r#"{"t":"master_chain","feedback":0.5,"cut":"exit","bloom":0.6,"rgb_shift":0.4}"#;
    let err = serde_json::from_str::<Record>(old)
        .expect_err("the four-field form parsed — the list did not replace it");
    let said = err.to_string();
    assert!(
        said.contains("feedback"),
        "the refusal does not name the shape it found: {said}"
    );
    assert!(
        said.contains("slots"),
        "the refusal does not name the shape it wants: {said}"
    );
}

/// A slot list round-trips, and an empty one is a real value rather than an
/// absence: the default chain is empty and that is what keeps the default look
/// free.
#[test]
fn a_master_chain_slot_list_round_trips() {
    let line = r#"{"t":"master_chain","slots":[{"proc":"sha256:a3","cut":"exit","params":{"amount":0.5}},{"proc":"sha256:77","params":{"amount":0.8}}]}"#;
    let Record::MasterChain(chain) = round_trip(line) else {
        panic!("not a master_chain");
    };
    assert_eq!(chain.slots.len(), 2);
    assert_eq!(chain.slots[0].procedure, "sha256:a3");
    assert_eq!(chain.slots[0].cut.as_deref(), Some("exit"));
    assert_eq!(chain.slots[0].params.get("amount"), Some(&0.5));
    // **No cut where the procedure declares no `retains`**, and the field
    // is absent rather than null: a slot that answered a cut it was not
    // asked for is refused by the engine, so writing one would be writing
    // a record this build refuses to obey.
    assert_eq!(chain.slots[1].cut, None);
    assert_eq!(
        round_trip(r#"{"t":"master_chain","slots":[]}"#),
        Record::MasterChain(Chain::default())
    );
}

/// A slot carrying a key this build has never heard of is refused too, for
/// [`Record::MasterChain`]'s own reason one level down: the four-field form's
/// keys would otherwise land here as a slot nobody wrote.
#[test]
fn a_chain_slot_with_an_unknown_key_is_refused() {
    let line = r#"{"t":"master_chain","slots":[{"proc":"sha256:a3","amount":0.5}]}"#;
    let err = serde_json::from_str::<Record>(line)
        .expect_err("an unknown key on a chain slot was dropped");
    assert!(err.to_string().contains("amount"), "{err}");
}

fn round_trip(line: &str) -> Record {
    let rec: Record = serde_json::from_str(line).expect("parse");
    let back = serde_json::to_string(&rec).expect("serialise");
    let again: Record = serde_json::from_str(&back).expect("reparse");
    assert_eq!(rec, again);
    rec
}

/// The same, and the bytes have to match too.
///
/// `round_trip` compares the parsed values, which cannot see a field that was
/// added with a default: a record gaining `index: 0` re-serialises with
/// `"index":0` in it and still equals itself. That is a changed stream for
/// every session ever recorded, and the only assertion that catches it is this
/// one. Only for records whose fields are written in declaration order with
/// nothing optional set.
fn round_trip_verbatim(line: &str) -> Record {
    let rec = round_trip(line);
    assert_eq!(
        serde_json::to_string(&rec).expect("serialise"),
        line,
        "the record did not come back as the bytes it went in as"
    );
    rec
}

/// A `part` round-trips through the line the spec prints, bytes and all, and
/// answers all three questions the way the authoring form needs.
///
/// `round_trip_verbatim` for [`Record::Select`]'s reason and one more: an
/// `index` or a `name` acquiring a serde default here would put a field into
/// every authoring file anybody hand-writes, and the whole of what a `.kset` is
/// for is being a file a person writes by hand.
///
/// The three assertions are the classification, and they are what keeps this
/// record out of a store: it is not Set state (so `Store::write_set` refuses
/// it), it is not an artifact's metadata (so it is refused by the question that
/// says *which* form the file is, not by the one that says it declares
/// something), and it *is* the authoring form's.
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

    // And the addressed spelling, which is the same record about the
    // second renderer of a stack with a name this Set gave it.
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

/// `Record::Edge`'s `slot` is `InputPort` in Rust and a bare string on the
/// wire, which is the whole claim
/// `docs/adr/0344-slot-is-disambiguated-into-three-types-and-adr-0049s-wait-is-over.md`
/// makes about it: the Rust identifier gets a type that cannot be confused with
/// a deck member or a Set-node address, and a `.kbset` file or a session stream
/// written before this change parses exactly as it did — `round_trip_verbatim`
/// is what catches `InputPort` serialising as `{"0":"far"}` or any other shape
/// a naive newtype wrapper could produce instead of the plain `"far"` this
/// asserts.
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

/// `Record::Gain`'s `slot` is `DeckSlot` in Rust and a bare number on the wire,
/// which is the whole claim
/// `docs/adr/0344-slot-is-disambiguated-into-three-types-and-adr-0049s-wait-is-over.md`
/// makes about the deck sense: the Rust identifier gets a type that cannot be
/// confused with a Set-node address or a declared input, and a `.kbset` file or
/// a session stream written before this change parses exactly as it did —
/// `round_trip_verbatim` is what catches `DeckSlot` serialising as `{"0":2}` or
/// any other shape a naive newtype wrapper could produce instead of the plain
/// `2` this asserts.
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

/// `Record::Procedure` carries two of `slot`'s three senses at once — its own
/// `slot: DeckSlot` beside `at: NodeAddress` — which is the case the module
/// documentation's whole point rests on: two fields named and typed for two
/// unrelated addresses, on one record, and neither reads as the other.
/// `round_trip_verbatim` on [`Record::Procedure`]'s own compatibility terms: an
/// absent `index` stays absent.
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

/// `slot` and `part` are two tags and not one tag with two shapes, which is
/// what `docs/contributing.md` §4 asks of a name and what a decoder dispatching
/// on `t` alone depends on.
///
/// The failure this defends against is silent: give a `slot` an optional `path`
/// instead, and the authoring line above parses as a `slot` with no `proc` — a
/// node with no procedure, in a file that says it is resolved. Here it cannot
/// parse at all, which is the whole difference.
#[test]
fn a_part_is_not_a_slot_with_a_path_where_the_address_goes() {
    let part = r#"{"t":"part","layer":"L1","path":"drift_shell.kir"}"#;
    let slot = r#"{"t":"slot","layer":"L1","proc":"sha256:00"}"#;
    assert!(matches!(
        serde_json::from_str::<Record>(part).expect("a part parses"),
        Record::Part { .. }
    ));
    assert!(
        serde_json::from_str::<Record>(r#"{"t":"slot","layer":"L1","path":"drift_shell.kir"}"#)
            .is_err(),
        "a `slot` carrying a path instead of an address is not a record this vocabulary has"
    );
    assert!(
        serde_json::from_str::<Record>(slot).is_err(),
        "and the address a `slot` does carry is a hash, so this fixture is a bad one \
         rather than a second shape"
    );
}

/// `procedure` names a node, and `index` is what says which.
///
/// A deck slot draws with one L1 and however many L4s, so a layer alone stopped
/// being enough. The compatibility claim is the whole point of the field being
/// optional: a stream recorded before stacks existed carries no `index`, must
/// parse as index 0, and must come back byte for byte — `round_trip` alone
/// would not notice `"index":0` appearing in every `procedure` line of every
/// session ever recorded.
#[test]
fn a_procedure_round_trips_and_an_absent_index_stays_absent() {
    let hash = "sha256:486779000000000000000000000000000000000000000000000000000000abcd";
    let old = format!(r#"{{"t":"procedure","slot":0,"layer":"L4","proc":"{hash}"}}"#);
    let Record::Procedure { at, slot, .. } = round_trip_verbatim(&old) else {
        panic!("not a procedure");
    };
    assert_eq!(
        (slot, at.index),
        (DeckSlot(0), 0),
        "an absent index is the first node"
    );

    let stacked = format!(r#"{{"t":"procedure","slot":2,"layer":"L4","index":1,"proc":"{hash}"}}"#);
    let Record::Procedure { at, slot, .. } = round_trip_verbatim(&stacked) else {
        panic!("not a procedure");
    };
    assert_eq!(
        (slot, at.index),
        (DeckSlot(2), 1),
        "the second renderer of slot 2"
    );
}

/// A `source` round-trips both ways round, bytes and all — with an attachment
/// and without one — and it is the session's rather than any Set's.
///
/// `round_trip_verbatim` rather than `round_trip`, on [`Record::Authority`]'s
/// terms: `index` is absent for a wildcard and `source` is absent for a
/// take-back, and a field appearing where nothing wrote one is exactly the
/// failure this catches — a `"source":null` on the take-back line would
/// round-trip by value and be a different line on the wire.
///
/// The `is_set_state` assertion is what this record exists to make: a Set file
/// carrying one would attach a signal wherever that file was next loaded, and
/// into whatever deck slot it landed in.
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
        },
        "an absent index is every node of that layer declaring the key, which is \
         `Record::Bind`'s rule and not `Record::Slot`'s"
    );
    assert!(
        !rec.is_set_state(),
        "a `source` is the session's: it names a deck slot, and a Set file carrying \
         one would attach a signal wherever it was next loaded"
    );
    assert!(!rec.is_metadata());

    // **The take-back, and the absence is on the wire.** A `"source"`
    // written as `null` would be a second spelling of nothing.
    let taken = r#"{"t":"source","slot":2,"layer":"L4","index":1,"key":"exposure"}"#;
    assert_eq!(
        round_trip_verbatim(taken),
        Record::Source {
            slot: DeckSlot(2),
            layer: Layer::L4,
            index: Some(1),
            key: "exposure".to_string(),
            source: None,
        },
        "a take-back is this record with its attachment absent"
    );

    // **A generator rides on the attachment**, field for field with
    // `Record::Bind`'s — `stream` is written even at 0, which is that
    // record's own shape and is why it is on the line here.
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

/// An authority round-trips through its wire name, bytes and all, and is the
/// session's rather than any Set's.
///
/// `round_trip_verbatim` rather than `round_trip`, on [`Record::Procedure`]'s
/// terms: the address is `(layer, index)` with the index absent at zero, so a
/// field appearing where nothing wrote one is the failure this catches and a
/// value comparison cannot see it.
///
/// The word is carried, not interpreted. `manual`, `suggesting` and `automatic`
/// are `karakuri_operation::Authority::name`'s, and a level this build does not
/// know reaches the engine's diagnostic rather than this decoder's refusal —
/// which is why the third line here parses at all.
///
/// The `is_set_state` assertion is the one this record exists to make: a Set
/// does not know which agent is watching it, so a Set file carrying an
/// authority would hand a node over every time it was loaded.
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

/// A capacity names a geometry, and `index` is what says which.
///
/// A Set holds more than one source, each at the default its own procedure
/// declares, so a layer alone stopped being able to say which of them is being
/// resized — two geometries at two capacities were inexpressible however they
/// were spelled on the way in. The line the specification prints carries no
/// index and is every capacity record ever written, so it has to come back byte
/// for byte; `round_trip` alone would not notice `"index":0` appearing in all
/// of them.
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
    // The thing that could not be said at all before: a second geometry,
    // at its own capacity, in the same file as the first.
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

/// A `param` round-trips through the flat wire shape `layer`/`index` always
/// had, bytes and all, in both of its shapes: addressed, and the wildcard — and
/// this is the one that exercises [`mod@node_or_every_node`], since a bare
/// `#[serde(flatten)]` on `Option<NodeAddress>` would read the wildcard line
/// below as addressed to node 0 instead (`layer` is present on it, same as
/// every wildcard `param` ever written).
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

/// The wire line the spec prints, parsed and written back.
///
/// Byte for byte, with the `hash` filled in: the specification prints this line
/// with the address elided — `"sha256:a3f2c1…"` — so the one value that cannot
/// be copied off the page is the artifact's identity, and it is built here the
/// way `karakuri-store`'s metadata tests build it. The order of the keys around
/// it is the specification's.
///
/// The head of a card is the record with the most to lose by drifting: it says
/// *which* artifact everything below it describes, so a reordered field is a
/// file every reader still parses and no reader can match against the `.kir` it
/// was read off.
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

/// The wire line the spec prints, parsed and written back.
///
/// Byte for byte, because a metadata file is written by one build and read by
/// another: `round_trip` compares the parsed values and would not notice the
/// keys coming back in a different order, which is a different file for every
/// card ever regenerated. Nothing optional here, so the whole line is the
/// record.
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
    // A declaration and not a value: it is the artifact's vocabulary, so a
    // Set file must not carry it and `Store::write_set` asks this.
    assert!(rec.is_metadata());
    assert!(!rec.is_set_state());
}

/// The wire line the spec prints, parsed and written back.
///
/// Byte for byte, for the reason above — and this is the record with the most
/// to lose by it: three bare numbers under three interchangeable keys, where a
/// reordered field produces a file that still parses everywhere and says a
/// capacity nobody declared.
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

/// The wire line the spec prints, parsed and written back.
///
/// Byte for byte, which for one field is the key and the order of the list
/// inside it: the attributes are written in declaration order, so a card that
/// reordered them would describe a struct the procedure does not write.
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
