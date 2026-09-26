use super::*;
use karakuri_console::view;
use karakuri_store::record::Record;
use karakuri_store::store::Store;

/// Nothing that comes back from `get_current_texture` is dropped without a
/// decision. The loop waits for events, so an outcome that neither reconfigures
/// nor asks for another frame is a window that never draws again and says
/// nothing about it.
#[test]
fn every_frame_that_could_not_be_acquired_is_acted_on() {
    use wgpu::CurrentSurfaceTexture as Acquired;
    // The two that mean the swapchain is stale: reconfigure, and ask again.
    assert_eq!(missed(&Acquired::Outdated), Some(Missed::Remake));
    assert_eq!(missed(&Acquired::Lost), Some(Missed::Remake));
    // Jitter: ask again, without reconfiguring.
    assert_eq!(missed(&Acquired::Timeout), Some(Missed::Again));
    // A window nobody can see: asking again is a spin, and the OS says
    // when it is back.
    assert_eq!(missed(&Acquired::Occluded), Some(Missed::Idle));
    // Not self-correcting, so it is said rather than retried.
    assert_eq!(missed(&Acquired::Validation), Some(Missed::Fault));
}

/// Compares documented allocation figures against runtime measurements to detect regressions (ADR-0164).
#[test]
fn a_reading_that_has_moved_says_the_sentence_quoting_it_is_stale() {
    // Allowed tolerance band for allocation drift between runs.
    assert_eq!(drifted(WRITTEN_ALLOCS, WRITTEN_ALLOCS), None);
    assert_eq!(
        drifted(WRITTEN_ALLOCS + 14, WRITTEN_ALLOCS),
        None,
        "the run-to-run spread of the reading this quotes must not read as staleness"
    );

    // Verifies significant allocation shifts trigger drift warnings (ADR-0164).
    assert!(
        drifted(456, 184).is_some(),
        "the mixer bay's landing is the drift this exists to have caught"
    );
    assert!(
        drifted(184, 456).is_some(),
        "drift is not caught one way round only"
    );

    // A band of two is still a band: an order of magnitude is well out of
    // it, from either end.
    assert!(drifted(WRITTEN_ALLOCS * 10, WRITTEN_ALLOCS).is_some());
    assert!(drifted(WRITTEN_ALLOCS / 10, WRITTEN_ALLOCS).is_some());
}

/// Verifies frame counting accuracy: unrequested frames trigger counts, while event-driven frames do not corrupt intervals.
#[test]
fn a_frame_nobody_asked_for_is_the_one_counted_against_a_still_panel() {
    let frame = Cost {
        allocs: 7,
        bytes: 70,
        ..Default::default()
    };
    let mut costs = Costs::new();

    // A frame something asked for is that something's.
    costs.owes();
    costs.push(frame);
    assert_eq!(costs.still, Still::default(), "an owed frame was counted");

    // A frame nobody asked for — `egui`'s own deadline, on an untouched
    // window — is per-frame work on a still panel, which is the number.
    costs.push(frame);
    assert_eq!(
        costs.still,
        Still {
            frames: 1,
            allocs: 7,
            bytes: 70
        },
        "a frame nobody asked for was not counted"
    );

    // Both frames happened, whoever they belonged to.
    assert_eq!(costs.drawn, 2);

    // And touching the window starts the stretch again, so what was drawn
    // during the last one stops being about a still panel.
    costs.touched();
    assert_eq!(costs.still, Still::default());
}

/// The transport row's rate counts owed frames and is not reset by a touch.
///
/// Every frame here is owed, as under a moving pointer, and the rate arrives
/// once [`Costs::RATE`] of periods has been pushed.
#[test]
fn a_moving_pointer_does_not_take_the_rate_off_the_transport_row() {
    let mut costs = Costs::new();

    // The first frame of a run has no period and is not part of any stretch.
    costs.push(Cost::default());

    let frame = Cost {
        period: Some(Duration::from_micros(16_667)),
        ..Cost::default()
    };
    // 29 frames at 60 Hz is 483 ms, short of the stretch.
    for i in 0..29 {
        costs.owes();
        costs.push(frame);
        assert_eq!(
            costs.rate(),
            None,
            "a rate before a whole stretch, frame {i}"
        );
    }
    assert_eq!(
        costs.still,
        Still::default(),
        "no frame was on a still panel"
    );

    // The 30th reaches 500 ms.
    costs.owes();
    costs.push(frame);
    let rate = costs
        .rate()
        .expect("no rate after a whole stretch of periods");
    assert!((rate - 60.0).abs() < 0.1, "rate {rate}, not 60");

    // Touching the window does not take it away.
    costs.touched();
    costs.owes();
    assert_eq!(costs.rate(), Some(rate));
}

/// Asserts distinction between CPU time ([`Cost::whole`]) and total frame interval including waits ([`Cost::period`]).
#[test]
fn a_frames_cost_has_the_wait_in_it_and_the_three_cpu_stretches_do_not() {
    let frame = Cost {
        wait: Duration::from_millis(200),
        engine: Duration::from_millis(1),
        ui: Duration::from_millis(1),
        paint: Duration::from_millis(1),
        period: Some(Duration::from_millis(205)),
        ..Cost::default()
    };

    assert_eq!(frame.whole(), Duration::from_millis(3), "the CPU's share");
    assert_eq!(
        frame.period,
        Some(Duration::from_millis(205)),
        "what the frame cost"
    );
    // The residue: the period, less the wait, less the three stretches.
    // Two milliseconds of this frame are in no field of it, which is the
    // claim `Cost::whole` used to make in prose — that its three *tile the
    // frame exactly* — measured instead of asserted.
    assert_eq!(frame.elsewhere(), Some(Duration::from_millis(2)));

    // **A frame with no predecessor has no period, and no residue
    // either.** A zero here would read as a frame that spent nothing
    // anywhere, which is the shape of answer P-0095 refuses.
    assert_eq!(Cost::default().elsewhere(), None);
}

/// Asserts period intervals require two frames and audit intervals are clock-driven rather than frame-count-driven ([`Costs::AUDIT`]).
#[test]
fn the_first_frame_has_no_period_and_an_audit_does_not_repeat() {
    let mut costs = Costs::new();
    let began = Instant::now();

    assert_eq!(
        costs.tick(began),
        None,
        "there was no frame before the first"
    );
    assert_eq!(
        costs.tick(began + Duration::from_millis(17)),
        Some(Duration::from_millis(17)),
        "the interval between two anchors is the frame"
    );

    // Fresh, the stretch has not elapsed: a run does not audit its first
    // frame, which is the one that builds the font atlas.
    assert!(!costs.audit(), "the first frame of a run was audited");
    // Wound back past the stretch, exactly one frame takes the audit and
    // the frame after it does not.
    costs.since_audit = Instant::now() - Costs::AUDIT;
    assert!(costs.audit(), "a stretch elapsed and nothing was audited");
    assert!(!costs.audit(), "two frames in a row were audited");
}

/// Verifies splitter dragging via `Readout::pointer` updates pane boundaries while isolating drag state from `egui`.
#[test]
fn a_drag_through_the_window_loops_own_routing_never_reaches_egui() {
    let ctx = drawn_once();
    let mut readout = Readout::new(1440.0, 900.0);
    readout.panel.solve();
    let layout = readout.panel.layout();
    let centre = layout.rect(layout.find("centre").expect("centre"));
    let left = layout.rect(layout.find("left-pane").expect("left-pane"));
    let was = left.w;
    // The gap between the left pane and the centre, at half height.
    let start = Point::new((left.x + left.w + centre.x) * 0.5, left.y + left.h * 0.5);

    // Approaching it is egui's until the pointer is on it.
    assert_eq!(
        readout
            .pointer(&ctx, Pointer::Moved(Point::new(start.x - 60.0, start.y)))
            .0,
        Claim::Egui
    );
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(start)).0, Claim::Panel);
    assert_eq!(readout.pointer(&ctx, Pointer::Down).0, Claim::Panel);

    // A hand does not stay on the boundary: it runs on across the panel,
    // and every one of these is inside a bay.
    for x in [start.x + 40.0, start.x + 120.0, start.x + 200.0] {
        assert_eq!(
            readout
                .pointer(&ctx, Pointer::Moved(Point::new(x, start.y)))
                .0,
            Claim::Panel,
            "the drag lost its claim at x = {x}"
        );
        // A wheel in the middle of a drag is the panel's too, and it
        // scrolls nothing: rule 1 withholds it from `egui` and
        // `input::wheeled` refuses it, so no pane moves under a hand that
        // is holding a boundary.
        let wheeled = readout.pointer(
            &ctx,
            Pointer::Wheel(karakuri_console::room::size::WHEEL_STEP),
        );
        assert_eq!(wheeled.0, Claim::Panel);
        assert_eq!(
            wheeled.1,
            Acted::Nothing,
            "a wheel in the middle of a boundary drag moved something"
        );
    }
    readout.panel.solve();
    let wide = pane_width(&readout);
    assert!(
        wide > was + 100.0,
        "the boundary did not move: the left pane went from {was} to {wide}"
    );

    // Now back the other way, to exactly what the left pane will not go
    // below. The boundary started at 340 and the pointer took hold 5 past
    // it, so 180 back from where it grabbed asks for the pane's own
    // minimum and no further.
    let stop = Point::new(start.x - 180.0, start.y);
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(stop)).0, Claim::Panel);
    readout.panel.solve();
    assert!(
        (pane_width(&readout) - 160.0).abs() < 0.01,
        "the left pane's stated minimum did not hold the drag: {}",
        pane_width(&readout)
    );

    // Dragging beyond minimum boundary threshold triggers pane fold (ADR-0300).
    let far = Point::new(start.x - 200.0, start.y);
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(far)).0, Claim::Panel);
    assert_eq!(readout.pointer(&ctx, Pointer::Up).0, Claim::Panel);

    readout.panel.solve();
    let left = readout
        .panel
        .layout()
        .find("left-pane")
        .expect("a left pane");
    assert!(
        readout.panel.layout().is_closed(left),
        "a drag 40 past the left pane's own minimum left it {} wide rather than closing it",
        pane_width(&readout)
    );

    // Verifies dragging a folded pane's boundary unfolds the pane back into view.
    let edge = readout
        .panel
        .layout()
        .boundary(readout.panel.layout().parent(left).expect("a body row"), 0)
        .expect("a closed pane keeps its boundary");
    let held = Point::new(edge.x + edge.w * 0.5, start.y);
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(held)).0, Claim::Panel);
    assert_eq!(readout.pointer(&ctx, Pointer::Down).0, Claim::Panel);
    assert_eq!(
        readout
            .pointer(&ctx, Pointer::Moved(Point::new(held.x + 40.0, held.y)))
            .0,
        Claim::Panel
    );
    assert_eq!(readout.pointer(&ctx, Pointer::Up).0, Claim::Panel);
    readout.panel.solve();
    assert!(
        !readout.panel.layout().is_closed(left),
        "the boundary the closed pane kept was dragged inward and the pane did not come back"
    );
    assert!(
        (pane_width(&readout) - 160.0).abs() < 0.01,
        "the pane came back {} wide rather than at the minimum it declares",
        pane_width(&readout)
    );

    // And afterwards the pointer, where it is standing, is egui's again.
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(far)).0, Claim::Egui);
}

/// Verifies mouse wheel events over an Inspector pane scroll only that targeted pane (ADR-0307).
#[test]
fn a_wheel_over_an_inspector_pane_scrolls_that_pane() {
    let ctx = drawn_once();
    let mut readout = Readout::new(1440.0, 900.0);
    readout.view.inspector = (0..view::PANES).map(deep_pane).collect();
    readout.panel.solve();

    let step = karakuri_console::room::size::WHEEL_STEP;
    let middle = |readout: &Readout, name: &str| {
        let layout = readout.panel.layout();
        let rect = layout.rect(layout.find(name).expect("a pane"));
        Point::new(rect.x + rect.w * 0.5, rect.y + rect.h * 0.75)
    };
    let first = middle(&readout, view::PANE_NAMES[0]);
    let second = middle(&readout, view::PANE_NAMES[1]);

    // 1. The wheel over the first pane is the panel's, and it moves that
    //    pane's position by exactly one notch.
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(first)).0, Claim::Egui);
    let turned = readout.pointer(&ctx, Pointer::Wheel(step));
    assert_eq!(
        turned,
        (Claim::Panel, Acted::Pointed),
        "the wheel over a pane did not reach the panel, or reached it and moved nothing"
    );
    assert_eq!(readout.view.scroll_in(0), step);
    assert_eq!(
        readout.view.scroll_in(1),
        0.0,
        "the wheel over one pane moved the other"
    );

    // 2. And the other pane is its own.
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(second)).0, Claim::Egui);
    assert_eq!(
        readout.pointer(&ctx, Pointer::Wheel(step * 2.0)),
        (Claim::Panel, Acted::Pointed)
    );
    assert_eq!(readout.view.scroll_in(1), step * 2.0);
    assert_eq!(
        readout.view.scroll_in(0),
        step,
        "the second pane's wheel moved the first"
    );

    // 3. A wheel anywhere else on the console is `egui`'s and moves
    //    nothing — the Library bay's list is a whole column away and is
    //    the bay whose own note says it does not scroll.
    let elsewhere = middle(&readout, "library");
    assert_eq!(
        readout.pointer(&ctx, Pointer::Moved(elsewhere)).0,
        Claim::Egui
    );
    assert_eq!(
        readout.pointer(&ctx, Pointer::Wheel(step)),
        (Claim::Egui, Acted::Nothing),
        "a wheel over a bay that does not scroll was taken by the panel"
    );
    assert_eq!(readout.view.scroll_in(0), step);
    assert_eq!(readout.view.scroll_in(1), step * 2.0);

    // 4. A wheel against the top of a pane's list is the panel's and is
    //    owed no frame — which is what `Acted::Nothing` says here and what
    //    `Change::Wheeled`'s second field carries.
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(first)).0, Claim::Egui);
    assert_eq!(
        readout.pointer(&ctx, Pointer::Wheel(-step * 10.0)),
        (Claim::Panel, Acted::Pointed),
        "the first spin back should have moved it to the top"
    );
    assert_eq!(readout.view.scroll_in(0), 0.0);
    assert_eq!(
        readout.pointer(&ctx, Pointer::Wheel(-step)),
        (Claim::Panel, Acted::Nothing),
        "a wheel spun against the top of the list asked for a frame"
    );
}

/// A pane with more in it than any pane on this panel can hold — four node
/// groups of six rows each, which is 4 x (26.5 + 6 x 22.5) = 646 and is taller
/// than the Inspector bay at the window this test opens.
fn deep_pane(deck: usize) -> view::Pane {
    view::Pane {
        deck,
        material: format!("deep_{deck}"),
        sync: karakuri_operation::Sync::Free,
        allows: [true; view::SYNCS.len()],
        anchor_bpm: 128.0,
        scrub_beats: 0.0,
        composite: false,
        // Not this test's row: the deck head's two build chips are
        // drawn from this and nothing here is about them.
        aimed: None,
        nodes: (0..4)
            .map(|node| view::Node {
                addr: format!("L1:{node}"),
                name: format!("node_{node}"),
                authority: Some(view::NodeAuthority {
                    at: karakuri_operation::NodeAddress {
                        layer: karakuri_operation::Layer::L1,
                        index: node as u32,
                    },
                    level: karakuri_operation::Authority::Manual,
                }),
                keep: Some(karakuri_operation::NodeAddress {
                    layer: karakuri_operation::Layer::L1,
                    index: node as u32,
                }),
                uses: Vec::new(),
                renderers: Vec::new(),
                params: (0..6)
                    .map(|at| view::Param {
                        ord: Some(at + 1),
                        name: format!("n{node}p{at}"),
                        value: 0.5,
                        range: [0.0, 1.0],
                        param: karakuri_operation::ParamAt {
                            node: Some(karakuri_operation::NodeAddress {
                                layer: karakuri_operation::Layer::L1,
                                index: node as u32,
                            }),
                            key: format!("n{node}p{at}"),
                        },
                        bound: None,
                    })
                    .collect(),
            })
            .collect(),
    }
}

/// The left pane's width, solved. A helper because the test asks three times
/// and the chain is four calls long.
fn pane_width(readout: &Readout) -> f32 {
    let layout = readout.panel.layout();
    layout.rect(layout.find("left-pane").expect("left-pane")).w
}

/// Ensures an initial draw pass runs to initialize `egui` font metrics and layouts before routing pointer queries.
pub(crate) fn drawn_once() -> egui::Context {
    let ctx = egui::Context::default();
    let mut out = ctx.run_ui(egui::RawInput::default(), |_| {});
    out.textures_delta.clear();
    ctx
}

/// Verifies permission class toggles: clicking a bay capsule pill opens its gate class and unblocks operations, while a second press shuts it (ADR-0156, ADR-0235, P-0090).
#[test]
fn the_gate_lets_a_refused_operation_through_once_the_class_is_open() {
    use karakuri_operation::gate::{audit, refusal, standing, Running, Standing};

    let ctx = drawn_once();
    let mut readout = Readout::new(1440.0, 900.0);
    readout.panel.solve();

    // A write to a mix fader: unpriced, immediate, irreversible, and what
    // the audience is looking at — P-0094's three answers, all missing.
    let write = Operation::SetGain { deck: 0, gain: 0.5 };
    assert_eq!(
        standing(&write, Running::unread()),
        Standing::Closed(Class::MixFaders),
        "`SetGain` is no longer in the class this test is about"
    );
    // And one from another class, to hold the press to one class below.
    let elsewhere = Operation::RecordSession {
        recording: karakuri_operation::Recording::Stop,
    };
    assert_eq!(
        standing(&elsewhere, Running::unread()),
        Standing::Closed(Class::InputsAndOutputs)
    );

    // 2. Refused, in one sentence, and it says where a hand opens it.
    let refused = audit(&write, readout.opening.read(), Running::unread())
        .expect_err("a mix write is allowed on a run nobody has opened anything on");
    assert_eq!(
        refused,
        refusal(&write, Standing::Closed(Class::MixFaders)).expect("a refusal has a sentence")
    );
    assert!(
        refused.contains("the head of the Mixer bay"),
        "the refusal does not say where the pill is: {refused}"
    );

    // 3. The press. Where the capsule is comes from the same derivation
    // that painted it, and the event goes through the window loop's own
    // routing — `Opening::set` is never called from this test.
    let capsule = |readout: &mut Readout| {
        readout.panel.solve();
        let pill = mcp_pill(
            &ctx,
            readout.panel.layout(),
            Class::MixFaders,
            readout.view.opening,
        )
        .expect("the Mixer bay draws its class pill");
        (
            Point::new(pill.pill.center().x, pill.pill.center().y),
            pill.open,
        )
    };
    let (at, open) = capsule(&mut readout);
    assert!(!open, "the pill reads open on a run that has just started");
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
    let (claim, did) = readout.pointer(&ctx, Pointer::Down);
    assert_eq!(claim, Claim::Panel);
    assert_eq!(
        did,
        Acted::Opened,
        "the class pill went down one of the other two paths — an `Operation` \
         or an operation on the arrangement — and ADR-0236 says it is neither"
    );
    readout.pointer(&ctx, Pointer::Up);

    // 4. The same call, the same audit, allowed.
    let allowed = audit(&write, readout.opening.read(), Running::unread())
        .expect("the operator opened the class and the call is still refused");
    assert_eq!(allowed.operation(), &write);

    // 5. And exactly that class.
    for class in Class::ALL {
        assert_eq!(
            readout.opening.read().holds(*class),
            *class == Class::MixFaders,
            "one press opened or shut {class:?} as well"
        );
    }
    assert_eq!(
        audit(&elsewhere, readout.opening.read(), Running::unread())
            .expect_err("opening the mix faders opened the outputs too"),
        refusal(&elsewhere, Standing::Closed(Class::InputsAndOutputs)).expect("a sentence")
    );

    // 6. And a second press shuts it again.
    let (at, open) = capsule(&mut readout);
    assert!(
        open,
        "the pill did not read open after the press that opened it"
    );
    readout.pointer(&ctx, Pointer::Moved(at));
    assert_eq!(readout.pointer(&ctx, Pointer::Down).1, Acted::Opened);
    assert_eq!(readout.opening.read(), Open::CLOSED);
    assert_eq!(
        audit(&write, readout.opening.read(), Running::unread())
            .expect_err("the class was shut again and the call still goes through"),
        refused
    );
}

/// All four pills are reachable through the window loop's routing, not the
/// Mixer's alone — three of them are in a bay head and the fourth is in a row
/// that has none, and the one this file could most easily have got wrong is the
/// one with no head to hang it in.
#[test]
fn each_of_the_four_pills_opens_its_own_class_through_a_press() {
    let ctx = drawn_once();
    for class in Class::ALL {
        let mut readout = Readout::new(1440.0, 900.0);
        readout.panel.solve();
        let pill = mcp_pill(&ctx, readout.panel.layout(), *class, readout.view.opening)
            .unwrap_or_else(|| panic!("{class:?} draws no pill"));
        let at = Point::new(pill.pill.center().x, pill.pill.center().y);

        assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
        assert_eq!(readout.pointer(&ctx, Pointer::Down).1, Acted::Opened);
        assert_eq!(
            readout.opening.read(),
            Open::CLOSED.with(*class, true),
            "a press on {class:?}'s pill did not open exactly it"
        );
        // **The view is written in the same breath as the handle**, or the
        // very next press is aimed at the capsule that used to be there:
        // the two words are not the same width.
        assert_eq!(readout.view.opening, readout.opening.read());
    }
}

/// Verifies control declarations are extracted directly from node cards rather than compiling shaders, combining shared keys and reporting uncarded nodes.
#[test]
fn a_reading_is_read_off_the_cards_and_never_off_a_compile() {
    use karakuri_store::ndjson::Line;
    let root = scratch_dir("read-set-declares");
    let store = Store::open(&root).expect("a store to read");

    // Three nodes: a geometry, a renderer that declares `exposure` over
    // the whole range, and a second renderer that declares it narrower.
    let card = |records: Vec<Record>| -> Vec<Line> { records.into_iter().map(Line::new).collect() };
    let param = |key: &str, min: f32, max: f32, default: Option<f32>| Record::ParamDecl {
        key: key.to_owned(),
        ty: "float".to_owned(),
        min,
        max,
        default,
    };
    let geometry = store.put_artifact(b"g\n").expect("an artifact");
    store
        .write_meta(
            &geometry,
            &card(vec![
                param("radius", 0.0, 8.0, Some(2.0)),
                Record::CapacityDecl {
                    min: 16384,
                    max: 1048576,
                    default: 262144,
                },
                Record::Emit {
                    attrs: vec!["position".to_owned(), "size".to_owned()],
                },
            ]),
        )
        .expect("a card");
    let wide = store.put_artifact(b"r1\n").expect("an artifact");
    store
        .write_meta(&wide, &card(vec![param("exposure", 0.0, 1.0, Some(0.4))]))
        .expect("a card");
    let narrow = store.put_artifact(b"r2\n").expect("an artifact");
    store
        .write_meta(&narrow, &card(vec![param("exposure", 0.2, 0.8, Some(0.9))]))
        .expect("a card");
    // And a fourth node whose artifact this store has no card for, which
    // is an ordinary state and not a damaged store.
    let bare = store.put_artifact(b"r3\n").expect("an artifact");

    let slot = |layer, index, hash| {
        Line::new(Record::Slot {
            at: karakuri_store::record::NodeAddress { layer, index },
            name: None,
            proc_hash: hash,
        })
    };
    use karakuri_store::record::Layer as Written;
    store
        .write_set(
            "drift_night",
            &[
                slot(Written::L1, 0, geometry),
                slot(Written::L4, 0, wide),
                slot(Written::L4, 1, narrow),
                slot(Written::L4, 2, bare),
            ],
        )
        .expect("a Set to read");

    let reading = declared(&root, "drift_night").expect("the Set reads");
    assert_eq!(reading.id, "drift_night");
    assert_eq!(
        reading
            .knobs
            .iter()
            .map(|knob| (knob.key.as_str(), knob.range.as_str()))
            .collect::<Vec<_>>(),
        vec![
            ("radius", "0 – 8 · 2"),
            // **One row and two nodes declare it**, over the part of the
            // range both of them accept, and the default is the first
            // declarer's — a control is one number and two nodes may
            // declare two.
            ("exposure", "0.2 – 0.8 · 0.4"),
        ],
        "the reading publishes {:?}",
        reading.knobs
    );
    assert_eq!(
        reading.capacity.as_deref(),
        Some("16384 – 1048576 · 262144"),
        "the capacity is drawn in a knob's shape"
    );
    assert_eq!(reading.emits.as_deref(), Some("position, size"));
    assert_eq!((reading.nodes, reading.described), (4, 3));
    assert_eq!(reading.cards_word(), "1 without a card");
    assert_eq!(reading.knobs_word(), "2 knobs");
    assert_eq!(reading.nodes_word(), "4 nodes");
    // The head, two knobs, the capacity, what it emits, and the foot.
    assert_eq!(reading.rows(), 6);

    // **A default the card cannot state as a number is a word and not a
    // blank**, because the `.kir` grammar makes the expression mandatory:
    // what a blank would say here is that there is no default, which is
    // false.
    let expr = store.put_artifact(b"g2\n").expect("an artifact");
    store
        .write_meta(&expr, &card(vec![param("hue", 0.0, 1.0, None)]))
        .expect("a card");
    store
        .write_set("expr01", &[slot(Written::L1, 0, expr)])
        .expect("a Set to read");
    assert_eq!(
        declared(&root, "expr01")
            .expect("the Set reads")
            .knobs
            .first()
            .map(|knob| knob.range.clone()),
        Some("0 – 1 · expr".to_owned())
    );

    // **A Set that declares nothing is an answer**: no capacity row, no
    // emits row, and a head that says `0 knobs`.
    let empty = store.put_artifact(b"e\n").expect("an artifact");
    store.write_meta(&empty, &card(vec![])).expect("a card");
    store
        .write_set("empty01", &[slot(Written::L4, 0, empty)])
        .expect("a Set to read");
    let reading = declared(&root, "empty01").expect("the Set reads");
    assert_eq!(
        (reading.capacity.clone(), reading.emits.clone()),
        (None, None)
    );
    assert_eq!(reading.rows(), 2, "a head and a foot are the whole of it");

    std::fs::remove_dir_all(&root).expect("clean up");
}
