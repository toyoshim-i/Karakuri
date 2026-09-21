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

/// A figure quoted in prose is held against the run that was just taken, so a
/// panel that grows says so instead of leaving a sentence that was true of a
/// smaller one.
///
/// This is the failure the guard exists for, and it is not hypothetical: the
/// line above the reading cited ADR-0164's 184 allocations and said the `egui`
/// pass "is still that" through the mixer bay landing at 456 and the parked
/// deck at 525 — two commits of a present-tense claim nobody re-checked,
/// because nothing re-checked it.
///
/// What can be asserted here is the verdict, not the reading. A reading needs a
/// device, a window and three seconds of nobody touching it, so it cannot be
/// taken from `cargo test`; what this file can do is make the figure in the
/// sentence and the figure under the verdict one constant, and hold [`drifted`]
/// to catching what actually went wrong.
#[test]
fn a_reading_that_has_moved_says_the_sentence_quoting_it_is_stale() {
    // The band a run has to stay inside to say nothing. The nine runs
    // behind the figure of 2026-08-31 agreed to the allocation; the nine
    // behind 2026-08-26's read between 524 and 538, which is the widest
    // run-to-run spread this file has ever taken, and a guard that fired
    // on 14 allocations is one nobody could keep passing.
    assert_eq!(drifted(WRITTEN_ALLOCS, WRITTEN_ALLOCS), None);
    assert_eq!(
        drifted(WRITTEN_ALLOCS + 14, WRITTEN_ALLOCS),
        None,
        "the run-to-run spread of the reading this quotes must not read as staleness"
    );

    // And what it is for: ADR-0164's 184 against the mixer bay's 456 is
    // 2.5x, so the first run after that bay landed would have said the
    // sentence had stopped being true. It is the same answer whichever of
    // the two is the one written down, because a pass that got cheaper
    // makes the sentence just as untrue.
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

/// A frame nobody asked for is the one the reading is about.
///
/// The measurement carries the claim now, so what it counts has to be asserted
/// rather than eyeballed on stdout. The failure it exists for is the one that
/// made the first run of this read `3 frames` on a window that had behaved
/// perfectly: an event resets the stretch, and the frame that event asked for
/// lands a millisecond into the new one and gets blamed on the panel. The other
/// direction is worse and is asserted too — a `push` that never counts anything
/// reads `0 frames` whatever the window is doing, which is a measurement that
/// cannot fail.
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

/// The defect this instrument was built for, stated as an assertion.
///
/// A frame that spent 200 ms blocked and 3 ms on the CPU is a 203 ms frame.
/// [`Cost::whole`] answers 3 ms, and that is not an error in it — it is CPU
/// time and says so — but it is what a reader who wants *what did this frame
/// cost* used to be handed, and what a loop at four frames a second was read
/// off as *idle 97.6% of the time*. The number with the wait in it is
/// [`Cost::period`], and the frame's own arithmetic is here so that a later
/// widening of `whole` fails rather than passes.
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

/// A period is an interval and needs two frames, and an audit happens at most
/// once per [`Costs::AUDIT`] however many frames go by.
///
/// Both are the same rule from two sides: the instrument reads the clock rather
/// than counting frames, so nothing about how fast this window draws changes
/// what either answers.
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

/// A whole drag, through the window loop's own routing.
///
/// `karakuri_console::input`'s tests are about the rule; this is about this
/// file obeying it, which is a different claim and the one that actually
/// reaches an operator. It drives the gesture a hand makes — press on the
/// boundary between the left pane and the centre, run the pointer well past it
/// and across two bays, let go — through `Readout::pointer`, which is the
/// method `window_event` calls, and asserts two things: the boundary moved, so
/// the drag works with a toolkit in the loop, and `egui` was never told about
/// any of it, so the two never both think they are dragging.
///
/// It cannot be a real pointer: synthesising one takes an Accessibility grant
/// this process does not have, and a test that needs a human to click is not a
/// test.
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

    // **And the way back, through the same routing.** A closed pane keeps
    // its boundary at the window's own edge, so the gesture that brings it
    // back is a second drag on that boundary — pressed, pulled inward, let
    // go. This is the sufficient half of *Fold a pane away* for the row
    // that needs it: `karakuri-console` cannot depend on this file, so
    // whether a hand on a real window reaches the fold is a test here.
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

/// A wheel over an Inspector pane scrolls that pane, and no other.
///
/// # Why it is here and can be nowhere else
///
/// `karakuri-console` holds the position and the derivation, and
/// `input::wheeled` answers *which pane* — but nothing in that crate joins the
/// two, because joining them is routing a window event and there is no window
/// there. `Readout::pointer` is the join, and it is a method rather than four
/// arms of `window_event` for exactly this reason: `winit` hands out no
/// `ActiveEventLoop` outside its own loop, so the handler is not something a
/// test can call and the part worth testing is this
/// ([ADR-0307](../../docs/adr/0307-the-inspectors-pane-scrolls-and-the-position-is-the-panes-own.md)).
///
/// Four things, and the third is the one a single-pane inspector would have
/// hidden: the wheel is aimed with the pointer, so two panes are two positions
/// and turning one must leave the other where it was.
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

/// A context that has drawn once, which is what routing a pointer takes: the
/// claim rule asks where the Outputs row's control is, that is the width of the
/// type in it, and `egui`'s fonts are not valid until a pass has run. The
/// window loop has drawn long before a hand arrives; a test has to say so.
///
/// The texture delta is cleared because `epaint` panics if one is dropped
/// unapplied — there is no renderer here to apply it to, which is the whole of
/// what makes this a test and not a window.
///
/// `mod gpu` uses it too: a strip is laid out with the type in it, and a device
/// does not make fonts valid.
pub(crate) fn drawn_once() -> egui::Context {
    let ctx = egui::Context::default();
    let mut out = ctx.run_ui(egui::RawInput::default(), |_| {});
    out.textures_delta.clear();
    ctx
}

/// The whole of what the four class pills are for: an operation the gate
/// refuses becomes one it allows, because a hand pressed a capsule.
///
/// # Why it is here and can be nowhere else
///
/// It crosses three crates and no two of them can see the third.
/// `karakuri-console` draws the pill and hands back a value; it must not name
/// `karakuri-environment` at all (ADR-0156), so it cannot reach the handle.
/// `karakuri-environment` holds the `Opening` and cannot see a console.
/// `karakuri-operation`'s gate holds the audit and the refusal and depends on
/// neither. This file is the only place all three are in scope, which is the
/// same reason `key_column` is a unit test in this binary: a surface is where
/// the buck stops, nothing may depend on this package, and the checks that need
/// everything at once live in it.
///
/// # What it asserts, in the order an operator's afternoon goes
///
/// 1. `SetGain` is in the mix-fader class, which is the classification ADR-0235
///    drew — asserted against `standing` rather than assumed, so that a row moved
///    out of the class fails here rather than making this test quietly vacuous.
/// 2. On a run nobody has touched it is refused, and the sentence is
///    `gate::refusal`'s own by equality — P-0090, *a refusal a person can reach
///    from two surfaces is one sentence*, asserted against the function rather
///    than with a `contains`. It names the Mixer bay, because a model that is told
///    only *no* reports the instrument as incapable instead of as closed.
/// 3. A press on the Mixer bay's pill — through `Readout::pointer`, which is the
///    same routing a hand goes through, and not by calling `set` here — opens the
///    class.
/// 4. The same call, the same audit, now allowed. Nothing about the
///    operation changed and nothing about the vocabulary changed; the list a model
///    reads never shortened at any point.
/// 5. And exactly that class. The other three are still shut and an operation in
///    one of them is still refused, which is the property the console's own
///    `a_press_opens_exactly_one_class_and_leaves_the_other_three_shut` makes
///    about the value and this one makes about the run.
/// 6. A second press shuts it, and the call is refused again — the other half of
///    the page's *"click again to shut it"*, seen from the gate.
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

/// A reading is read off the cards, one row per key, and never off a compile.
///
/// The claim `docs/manual/operations.html` makes for this row — *"each read off
/// the artifact's own card, so those three fetch no source and compile
/// nothing"* — and the four things [`declared`] has to get right, each of which
/// a plainer reading would get wrong:
///
/// 1. One control per key. `exposure` is declared by two nodes here, exactly as
///    it is in the mock's own reading, and it is one row.
/// 2. Over the part of the range both of them accept, which is
///    `Set::published`'s intersection done off the cards: `[0, 1]` and `[0.2, 0.8]`
///    is one control over `[0.2, 0.8]`.
/// 3. A node with no card is counted and not skipped in silence, which is the
///    foot's `n without a card` and the one thing that keeps a knob missing for
///    want of a card from being a knob missing.
/// 4. Nothing was compiled. The artifacts here are not `.kir` at all — they are
///    three bytes each — so a reading that fetched and checked a source could
///    not have answered at all, which is the strongest form this claim can be put in.
///
/// A CPU test: a store is a directory and no adapter is opened.
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
