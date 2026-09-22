use super::*;

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
