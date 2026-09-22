use super::super::*;
use karakuri_console::input::PROBES;

/// Every row of `karakuri_console::input::PROBES`, and how this file reaches
/// what it claims.
///
/// The row's name, the derivation this file calls to get the receiver, and
/// every call the press handler makes on that receiver for the controls the row
/// covers. One entry per row and in the console's own order, which is what
/// [`the_table_is_the_consoles_own_rows_in_the_consoles_own_order`] holds it to
/// — so an entry cannot quietly answer for its neighbour.
///
/// The array is `PROBES.len()` long, and that is the whole of the backward
/// direction. A control the console starts claiming and this window never asks
/// used to be found by reading that crate's source for `pub fn`s taking a
/// `Point`; it is now a row in a value, and a row with no entry here does not
/// compile.
///
/// The asks are written down rather than derived, and they have to be: nothing
/// in a row's name says `mixer_bay`, and the derivations are a deliberate
/// arrangement rather than a convention — a bay is derived *once* and asked six
/// times, because six derivations of one laid-out strip would be six answers.
///
/// Three entries name calls made on a button *up* rather than a press. A carry
/// names its deck by where it is let go, so the mixer bay and the Program bay
/// are laid out at the release and asked which rectangle the pointer is over —
/// `bay.dropped(` and `cells.dropped(`.
///
/// The preview cells' entry is that release and nothing else, because a press
/// on a cell asks for nothing: ADR-0240 retired `SetPreview`, and
/// `ProgramBay::cell`'s own documentation still says *"A press on a cell asks
/// for nothing, and that is the decision rather than a gap."* The cells are
/// claimed all the same, because they are drawn over a boundary and
/// `input::claim` has to keep a press on them off `egui`
/// ([ADR-0273](../../../docs/adr/0273-the-carry-lands-on-two-sets-of-rectangles-and-wears-a-face.md)).
const ASKED: [(&str, &str, &[&str]); PROBES.len()] = [
    // **The Outputs row's sinks**, and the two calls are one control: the
    // row answers which chip the point is on, and then how this surface
    // carries out what that chip asked for.
    //
    // **`row.hit(` was the first of the two until 2026-09-09**, when the
    // row stopped drawing one sink and started drawing the list it has
    // (ADR-0324). `chip_at` is the same question over every chip, and it
    // is what `input::claim` asks as well — one derivation, so a chip that
    // lights under the pointer is a chip a press reaches. `row.op(` stays
    // because the program view's on and off is still the fold: the
    // operation names the output and the fold is how the console performs
    // it, which is one fact rather than two.
    (
        "the Outputs row's sinks",
        "outputs(",
        &["row.chip_at(", "row.op("],
    ),
    // **The two cards, and they are the only two whose order matters**:
    // each draws over the bays, so while one is down a press inside it
    // belongs to the card. `ask` is the whole of each pill's offer — the
    // press on the capsule, the press on a row, and the dismissal — which
    // is why neither names `item`, the sub-question each `ask` composes.
    // They are asked through one local, so it is the derivation above each
    // that tells the two entries apart.
    ("the audio-in pill", "audio_in_pill(", &["pill.ask("]),
    // **The tracker group's three, derived once for all of them**, and
    // the first row added to the console's table since it landed. The
    // offset's figure is as wide as the number in it and the octave is
    // laid out from where the tap ends, so a second derivation would put
    // the chip a press lands on somewhere the mark is not.
    (
        "the tracker group's three",
        "tracker_group(",
        &["group.tapped(", "group.octave(", "group.nudge("],
    ),
    // **The `learn` pill**, and its one call is `hit` rather than an
    // `ask`: what a press names is a `bool` and not an operation, because
    // a learn edits the *map* rather than being a member of the vocabulary
    // the map addresses (ADR-0236, ADR-0336). `pill.next(` is the state it
    // names and is read at the press.
    (
        "the transport row's learn pill",
        "view::learn_pill(",
        &["pill.hit(", "pill.next("],
    ),
    // **The `map` pill is a readout, and it is derived all the same.**
    // `input::claim` already keeps a press on it off `egui`, so the press
    // handler could have let it fall through — and then this table would
    // have carried an entry with no derivation and no ask, which passes
    // vacuously. So the press is recognised at the control and swallowed
    // there, which is a fact a reader can check. There is no ask because
    // there is nothing to ask: reaching a different map while running is
    // not built, and the pill says which file is loaded.
    (
        "the transport row's map pill",
        "view::map_pill(",
        &["row.pill.contains("],
    ),
    ("the arrangement pill", "arrangement_pill(", &["pill.ask("]),
    // **The two look controls, derived once for both** — the exposure
    // track's place is measured from the tone map capsule's, so they are
    // two questions about one laid-out group.
    (
        "the look group's two",
        "look_row(",
        &["row.tonemap(", "row.exposure("],
    ),
    // **The `rec` pill at the end of that row**, and the derivation is the
    // *row* rather than a pill of its own: the capsule takes the row's
    // right padding and the health capsule and the frame readout are laid
    // out backwards from it, so there is one derivation of that end and
    // this entry names it. One call, because what a press means is the
    // pill's own state and `record` reads it — a toggle is one control and
    // not two.
    (
        "the transport row's rec pill",
        "transport_row(",
        &["row.record("],
    ),
    // **The tempo figure at the head of that row**, and it is the `rec`
    // pill's entry one item along: the same derivation, because the figure
    // is the row's first readout and the control is that reading. One call
    // — where along the number the press landed *is* the tempo, and
    // `tempo` is where the band that refuses a mis-click sits.
    (
        "the transport row's tempo figure",
        "transport_row(",
        &["row.tempo("],
    ),
    // **The Mixer bay's five, derived once and asked five times on a
    // press**, plus the drop at the release. `select` is asked last and is
    // the strip itself — what none of the other four claimed.
    (
        "a mixer strip's five",
        "mixer_bay(",
        &[
            "bay.grab(",
            "bay.blend(",
            "bay.tally(",
            "bay.mask(",
            "bay.select(",
            "bay.dropped(",
        ],
    ),
    // **The transition row's four, and the reason this module exists.**
    // These are the controls that were drawn, claimed and never asked
    // between `74719c3` and `094804c`. Deleting this row's branch from the
    // press handler is the injection this module was watched to fail
    // against; deleting `go`'s `match` alone, and leaving the other three
    // wired, is the second.
    (
        "the transition row's four",
        "transition_row(",
        &["row.shape(", "row.quantum(", "row.length(", "row.go("],
    ),
    // **The Master bay's five**, and they are the Mixer bay's `grab` and
    // `blend` on a different type bound to a different local — which is why
    // the local is part of what is written down and the method alone is
    // not. Two calls for five controls: `grab` answers for the out knob and
    // the three effect rows' knobs, and `chip` for the feedback row's cut.
    (
        "the Master bay's five",
        "master_row(",
        &["row.grab(", "row.chip("],
    ),
    // **The Inspector pane heads' name**, and it is the capsule's
    // arrangement at the other end of the same row: the pane is derived
    // per index and the run from the pane, so the derivation named here is
    // the inner one. The press emits nothing — it opens the field, and the
    // operation is the commit's, at Return.
    (
        "the Inspector pane heads' name",
        "deck_name(",
        &["named.hit("],
    ),
    // **The Inspector pane heads' `keep`**, and it is the deck head's
    // arrangement one row up: the pane is derived per index and the
    // capsule from the pane, so the derivation named here is the inner
    // one. What the press hands back is an operation; the write itself is
    // at the call site, because a disk write is not a thing to do on a
    // frame.
    (
        "the Inspector pane heads' keep",
        "keep_pill(",
        &["pill.keep("],
    ),
    (
        "the Inspector pane heads' slot mcp policy",
        "slot_mcp_pill(",
        &["pill.hit("],
    ),
    // **The `▾` beside it and the card it puts down**, which is the `uses`
    // line's arrangement one row up: the pick is asked before the mark,
    // because a press inside the card belongs to the card and the mark the
    // card came out of is above it. Both go through the same derivation on
    // the view, so the mark that is painted is the mark a press lands on.
    (
        "the Inspector pane heads' deck pulldown",
        "pane_pulldown(",
        &["target.hit(", ".picked("],
    ),
    // **The deck head's seven, one Inspector pane at a time.** The pane is
    // derived per index and the head from the pane, so the derivation
    // named here is the inner one: `inspector_pane` answers a rectangle
    // and `deck_head_row` answers the control. **Seven controls and six
    // calls**: the scrub's two arrows are two of the seven and `scrub`
    // answers for both of them. The last three are the row's build chips
    // and each re-aims the slot rather than moving the mix, which is why
    // the arms that act on them are beside the library load's and not
    // beside the sync chip's (ADR-0314, ADR-0328).
    (
        "a deck head's seven",
        "deck_head_row(",
        &[
            "head.sync(",
            "head.reanchor(",
            "head.scrub(",
            "head.resized(",
            "head.re_salted(",
            "head.compositing(",
        ],
    ),
    // **The renderer chips**, and the pane is the whole derivation: which
    // group and which chip are inside `select_renderer`, which is the
    // `params` chip's arrangement in the Library bay. So the derivation
    // named here is the *outer* one, unlike the two rows above it.
    (
        "the renderer chips",
        "inspector_pane(",
        &["at_pane.select_renderer("],
    ),
    // **A parameter row's fader, one Inspector pane at a time**, and it is
    // the Master bay's `grab` on a third type bound to a third local: the
    // pane is derived per index and the fader from the pane, so the
    // derivation named here is the outer one, as the chips' above is.
    (
        "a parameter row's fader",
        "inspector_pane(",
        &["at_pane.grab("],
    ),
    // **A parameter row's publish mark**, which is the leftmost cell of the
    // row the fader is on and is derived from the same laid-out pane: one
    // press arm, one call, and what it asks for is the whole interface
    // rather than this entry (ADR-0329).
    (
        "a parameter row's publish mark",
        "inspector_pane(",
        &["at_pane.publishing("],
    ),
    // **A node group's `uses` capsule and its card**, and the two are one
    // row for the Library bay's load button and deck pulldown's reason:
    // they are one derivation and one ask. The capsule emits nothing — it
    // puts the card down — and the card's rows emit `WireInput`, which is
    // why `Readout::wiring_at` is the name here rather than either half.
    (
        "a node group's `uses` capsule and its card",
        "inspector_pane(",
        &["laid.uses_chip(", "laid.wired("],
    ),
    // **A node head's three `man / sug / auto` chips**, and the pane is
    // the whole derivation for the renderer chips' reason two rows up:
    // which group, which head and which chip are inside
    // `InspectorPane::set_authority`.
    (
        "a node head's three authority chips",
        "inspector_pane(",
        &["at_pane.set_authority("],
    ),
    // **The `keep` capsule at the right of the same head**, and the pane is
    // the whole derivation for the chips' reason: which group and where
    // the capsule is are inside `InspectorPane::keep_procedure`. The two
    // heads that carry none answer `None` there rather than being checked
    // here (ADR-0338, decision 4).
    (
        "a node head's keep capsule",
        "inspector_pane(",
        &["at_pane.keep_procedure("],
    ),
    // **The sensitivity row's curve chip and `take back`**, the two of its
    // four chips that are controls — the other two are readouts and answer
    // `None`, which is `view::SensChip`'s decision and not this file's.
    // One derivation for both, as the authority chips' row above is.
    (
        "a sensitivity row's curve and take back",
        "inspector_pane(",
        &["at_pane.sensitivity("],
    ),
    // **The Program bay head's `solo`**, and the two calls are the Outputs
    // sink's arrangement one bay over: the head answers whether the point
    // is on the capsule, and then which of the two operations it is.
    (
        "the Program bay head's solo",
        "program_head(",
        &["head.hit(", "head.op("],
    ),
    // **The grip in each bay head**, the capsule above's neighbour in the
    // same head: one derivation asked once per bay, because four heads
    // cannot be one laid-out box. A pane has no row here and needs none —
    // it folds by its own boundary, which rule 3 claims (ADR-0300).
    (
        "the grip in a bay head",
        "bay_grip(",
        &["grip.hit(", "grip.op("],
    ),
    // **The four deck preview cells**, asked at the release alone — see
    // the head of this table.
    (
        "the deck preview cells",
        "program_bay(",
        &["cells.dropped("],
    ),
    // **The Library bay's five**, and the bay is derived once per question
    // rather than held across them, exactly as `input::claim` derives it
    // five times: a value held across all five would outlive the question
    // it answers. So one derivation is named by five entries, and each is
    // told apart by the call rather than by it.
    (
        "the Library bay's scope chips",
        "library_bay(",
        &["bay.chip("],
    ),
    (
        "the Library bay's filter fields",
        "library_bay(",
        &["bay.filter("],
    ),
    // **The six kind chips one row under the field**, the same derivation
    // again and told apart by the call — the scope chips' own arrangement
    // two rows up. One entry for six capsules, because `LibraryBay::kind`
    // is the whole of that row's offer: which chip a press landed on is
    // inside it, and what leaves names all six (ADR-0338).
    (
        "the Library bay's kind chips",
        "library_bay(",
        &["bay.kind("],
    ),
    // **A row's badges are a readout and still a row here**, which is the
    // `map` pill's own sentence in this table read one bay along — with one
    // difference that is the whole of the entry: this press is **not**
    // swallowed. A badge sits inside the row it is drawn on, so a press on
    // it takes that row in hand exactly as a press on the name does, and
    // the list's own entry below is what asks for it. What the row here
    // buys is the tooltip: the words that say what a badge means and that
    // the control which narrows by kind is the chip row above (ADR-0338).
    // So it names no ask, and the derivation it names is the one the rows
    // beneath it are hit-tested against.
    ("the Library bay's row badges", "library_bay(", &[]),
    (
        "the params chip in the Library bay's foot",
        "library_bay(",
        &["bay.read("],
    ),
    // **The `load` button and the deck pulldown beside it**, and they are
    // one call rather than two: `LibraryBay::aim` is the whole of that
    // control's offer — the press on the button, the press on the
    // capsule, the press on a row of the list and the dismissal — which is
    // the shape the two cards in the transport row are already in, and why
    // neither entry names `Load::picked`, the sub-question `aim` composes.
    // They are told apart by nothing but this table, exactly as the
    // audio-in and arrangement pills are.
    // **The `load` button and the deck pulldown beside it**, one entry
    // for two controls because they are one derivation and one ask:
    // `LibraryBay::aim` is the whole of that control's offer — the press
    // on the button, the press on the capsule, the press on a row of the
    // list it puts down, and the dismissal — which is the shape the two
    // cards in the transport row are each in, and why this names no
    // `Load::picked`, the sub-question `aim` composes. The console's row
    // claims two; this table's rows are that table's rows, so there is one
    // here (ADR-0305).
    (
        "the Library bay's load button and deck pulldown",
        "library_bay(",
        &["bay.aim("],
    ),
    // **The star at the left of each row**, asked before the row it is in
    // so that the smaller box wins — the same derivation again, told apart
    // from the row below it by the call.
    ("the Library bay's stars", "library_bay(", &["bay.starred("]),
    // **The list's row names three calls**, because one rectangle means
    // three things: a press on a library row takes a Set in hand, a press
    // on a `history` row lands that version on its node, and a *secondary*
    // press on either puts that row's menu down. The first two are one
    // control in `input::PROBES` — exactly one of them can answer, told
    // apart by which listing goes in — and the third is a second control
    // on the same rectangle, which is why that row claims two. All three
    // are owed here.
    //
    // **`bay.menu_ask(` is one entry for two arms of the handler**, the
    // secondary press that opens the card and the press of either button
    // that picks from it, exactly as `bay.aim(` is one entry for the
    // pulldown's four.
    (
        "the Library bay's list",
        "library_bay(",
        &["bay.take(", "bay.land(", "bay.menu_ask("],
    ),
    // **The four class pills**, one derivation asked four times: they are
    // in four different regions and cannot be one laid-out box, but they
    // are one type and one question.
    ("the class pills", "mcp_pill(", &["pill.hit("]),
    // **The Sequencer bay's, one derivation and two calls**: a cell, a
    // label, the mode pill and a bank pill are four questions about one
    // laid-out bay, and `Sequencer::press` answers all four — which of them
    // it was is inside the operation it hands back, and the line
    // `sequenced` prints says so.
    //
    // **`bay.chose(` is the second call for the same reason `bay.aim(` is
    // one entry for the pulldown's four**: the `+ lane` pill's press is not
    // an operation but a card going down, so it answers an enum, and the
    // card's own picks and dismissals come back through the same method.
    (
        "the Sequencer bay's cells, labels, minus glyphs, mode pill, bank pills and + lane",
        "sequencer_bay(",
        &["bay.press(", "bay.chose("],
    ),
    // **The Staging lane's two, and they are the Library bay's star and
    // its row one bay up**: a row that is itself a control, with a smaller
    // box inside it asked first. The lane is derived once per question
    // rather than held across both, which is that bay's rule for its
    // reason, so the two entries name one derivation and are told apart by
    // the call — `bay.back(` is the capsule and `bay.keep(` is the row it
    // sits in (ADR-0326).
    (
        "the Staging lane's back capsules",
        "staging_bay(",
        &["bay.back("],
    ),
    ("the Staging lane's rows", "staging_bay(", &["bay.keep("]),
];

/// [`ASKED`] is the console's own rows, in the console's own order.
///
/// The array's length is `PROBES.len()`, so a row added or removed there is a
/// compile error here; this is the other half of that, and it is what makes an
/// entry identifiable at all. Written in the same order, an entry and a row are
/// the same control by position — written in any other, the two would agree on
/// a count and disagree about which control each line is about, and every
/// message this module prints would name the wrong one.
#[test]
fn the_table_is_the_consoles_own_rows_in_the_consoles_own_order() {
    for (at, (name, derivation, _)) in ASKED.iter().enumerate() {
        assert_eq!(
            *name, PROBES[at].name,
            "entry {at} of `ASKED` says the press handler reaches `{name}` through \
             `{derivation}`, and row {at} of `karakuri_console::input::PROBES` is \
             `{}` — the two lists have gone out of order, and every entry from here \
             down is being checked against another control's row",
            PROBES[at].name
        );
    }
}

/// Every probe registered in `karakuri_console::input::PROBES` evaluates against
/// a solved `Readout` layout and `egui::Context`.
///
/// Asserts that each of the 38 control probes:
/// - Matches its corresponding entry in [`ASKED`] in name, order, and non-empty derivation.
/// - Directly evaluates via `probe.ask` against solved panel layout, context, and view without error.
/// - Produces `false` on non-control space (origin) and `true` when tested on active controls.
#[test]
fn every_control_probe_in_probes_evaluates_against_readout_layout_and_context() {
    let ctx = crate::tests::drawn_once();
    let mut readout = Readout::new(1440.0, 900.0);
    readout.panel.solve();

    assert_eq!(
        PROBES.len(),
        ASKED.len(),
        "`PROBES` and `ASKED` must have identical lengths"
    );

    for (at, probe) in PROBES.iter().enumerate() {
        let (asked_name, derivation, asks) = ASKED[at];
        assert_eq!(
            probe.name, asked_name,
            "entry {at} of `ASKED` says `{asked_name}` but `PROBES` has `{}`",
            probe.name
        );
        assert!(
            !derivation.is_empty(),
            "probe `{}` has an empty derivation",
            probe.name
        );

        // Directly evaluate probe function pointer against the solved Readout layout,
        // context, and view on arbitrary unhandled space.
        let hit_empty = (probe.ask)(&readout.panel, &ctx, &readout.view, Point::new(0.0, 0.0));
        assert!(
            !hit_empty,
            "probe `{}` should evaluate to false on unhandled point (0, 0)",
            probe.name
        );

        // Claims should be positive unless it's a readout row without asks.
        assert!(
            probe.claims > 0 || asks.is_empty(),
            "probe `{}` has 0 claims but specifies asks",
            probe.name
        );
    }
}
