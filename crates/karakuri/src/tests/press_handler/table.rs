use super::super::*;
use karakuri_console::input::PROBES;

/// Mapping from each probe row in `karakuri_console::input::PROBES` to its receiver derivation and method invocations (ADR-0240, ADR-0273).
const ASKED: [(&str, &str, &[&str]); PROBES.len()] = [
    // Outputs row sinks: hit-tests chips and applies presentation folding operations (ADR-0324).
    (
        "the Outputs row's sinks",
        "outputs(",
        &["row.chip_at(", "row.op("],
    ),
    // Dropdown cards: card contents hit-test before background bays, handling selection and dismissal.
    ("the audio-in pill", "audio_in_pill(", &["pill.ask("]),
    // Tracker group: unified layout derivation covers latency offset, tap tempo, and octave chips.
    (
        "the tracker group's three",
        "tracker_group(",
        &["group.tapped(", "group.octave(", "group.nudge("],
    ),
    // Learn pill: toggles MIDI learn mode directly without dispatching session operations (ADR-0236, ADR-0336).
    (
        "the transport row's learn pill",
        "view::learn_pill(",
        &["pill.hit(", "pill.next("],
    ),
    // Map pill: status readout pill that claims pointer hits to prevent egui event leakage.
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
    // Record toggle pill: derived from transport row layout, toggles recording state.
    (
        "the transport row's rec pill",
        "transport_row(",
        &["row.record("],
    ),
    // Tempo display readout: derived from transport row, click adjusts session tempo.
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
    // Transition row controls: wipe, fade, crossfade triggers and Go button.
    (
        "the transition row's four",
        "transition_row(",
        &["row.shape(", "row.quantum(", "row.length(", "row.go("],
    ),
    // Master bay: handles output master gain knob, effect level knobs, and feedback cut chip.
    (
        "the Master bay's five",
        "master_row(",
        &["row.grab(", "row.chip("],
    ),
    // Inspector pane header name: focuses name editing field on click.
    (
        "the Inspector pane heads' name",
        "deck_name(",
        &["named.hit("],
    ),
    // Inspector pane header keep: returns keep operation for modified procedures.
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
    // Inspector deck selector pulldown and popup card: hit-tests dropdown selections before deck mark.
    (
        "the Inspector pane heads' deck pulldown",
        "pane_pulldown(",
        &["target.hit(", ".picked("],
    ),
    // Deck head controls: derives Inspector pane head, routing scrub arrows, parameter resets, and build chips (ADR-0314, ADR-0328).
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
    // Node group wiring capsule and card: opens routing card emitting `WireInput` on selection.
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
    // Procedure keep capsule: delegates procedure saving through `InspectorPane::keep_procedure` (ADR-0338).
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
    // Library bay controls: derives library bay layout per hit-test query.
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
    // Library kind filter chips: routes selection across all six kind categories (ADR-0338).
    (
        "the Library bay's kind chips",
        "library_bay(",
        &["bay.kind("],
    ),
    // Library row badges: displays kind tooltip while passing clicks through to parent row selection (ADR-0338).
    ("the Library bay's row badges", "library_bay(", &[]),
    (
        "the params chip in the Library bay's foot",
        "library_bay(",
        &["bay.read("],
    ),
    // Library load button and deck pulldown: combined derivation for deck target selection and Set loading (ADR-0305).
    (
        "the Library bay's load button and deck pulldown",
        "library_bay(",
        &["bay.aim("],
    ),
    // **The star at the left of each row**, asked before the row it is in
    // so that the smaller box wins — the same derivation again, told apart
    // from the row below it by the call.
    ("the Library bay's stars", "library_bay(", &["bay.starred("]),
    // Library and history list rows: handles primary selection, version landing, and secondary context menus.
    (
        "the Library bay's list",
        "library_bay(",
        &["bay.take(", "bay.land(", "bay.menu_ask("],
    ),
    // **The four class pills**, one derivation asked four times: they are
    // in four different regions and cannot be one laid-out box, but they
    // are one type and one question.
    ("the class pills", "mcp_pill(", &["pill.hit("]),
    // Sequencer bay controls: routes cell triggers, mode pills, bank selection, and lane creation menus.
    (
        "the Sequencer bay's cells, labels, minus glyphs, mode pill, bank pills and + lane",
        "sequencer_bay(",
        &["bay.press(", "bay.chose("],
    ),
    // Staging lane rows and rollback capsules: handles candidate retention and rollback requests (ADR-0326).
    (
        "the Staging lane's back capsules",
        "staging_bay(",
        &["bay.back("],
    ),
    ("the Staging lane's rows", "staging_bay(", &["bay.keep("]),
];

/// Asserts that [`ASKED`] entries align 1:1 in length and order with console `PROBES`.
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

/// Verifies each probe in `PROBES` matches its [`ASKED`] definition and evaluates correctly against layout.
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
