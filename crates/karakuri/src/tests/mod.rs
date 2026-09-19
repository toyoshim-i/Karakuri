use super::*;

pub(crate) use self as tests;
pub(crate) use crate::session::Sessions;

use std::time::{Duration, Instant};

use karakuri_console::focus::Step;
use karakuri_console::input::Claim;
use karakuri_console::panel::Panel;
use karakuri_console::view::{
    look as look_row, mixer as mixer_bay, picture_rect, preview_rects, tracker_group, Picture,
    Reading, RowKind, Scope, Tracker, TransitionSettings, View, DECKS, SCRUB_BEATS, SYNCS,
};
use karakuri_console::{egui, egui_wgpu};
use karakuri_engine::set::Layering;
use karakuri_engine::transport::Sync as EngineSync;
use karakuri_engine::{
    compose, Blend, Committed, Cut, Deck, DeckSlot as EngineSlot, Event, Gpu, Look, Mask, MaskKind,
    Residency, Sink, Skip, TonemapOp,
};
use karakuri_environment::{audio, mix, setfile, watch, Asked, Opening};
use karakuri_layout::{Layout, Point};
use karakuri_operation::{BeatSource, BlendMode, GridScale, Operation, SetTransfer, Undecided};
use karakuri_operation_record::{not_performed, written, Current, Owed, Silent, Written};
use karakuri_store::record::{DeckSlot, Record};
use karakuri_store::store::Store;
use winit::event::WindowEvent;

mod focus_keys;
mod gpu;
mod inspector_mcp;
mod mixer_solo_mute;
mod outputs_row;
mod press_handler;

mod keeps;

mod history;
pub(crate) use self::history::scratch_dir;

mod arrangement;

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

    // And on past it, and let go there. **The pointer ends nowhere near
    // the boundary**, which is the ordinary end of a drag and the case
    // that catches a release routed after `released` rather than before
    // it.
    //
    // **A pull this far past a pane's own minimum closes the pane**
    // (`docs/adr/0300-a-pane-folds-by-dragging-its-boundary-out-and-comes-back-by-dragging-it-in.md`),
    // which is what the assertion above used to say instead: the minimum
    // holds a drag that stops at it, and a drag that goes on through it is
    // asking for the fold. The pane keeps its edge, so the boundary is
    // still there at the window's own edge and another drag brings it
    // back — none of which is this test's subject, which is that the
    // window loop's routing never lets go of the gesture.
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
pub(super) fn drawn_once() -> egui::Context {
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
/// drew — asserted against `standing` rather than assumed, so that a row moved
/// out of the class fails here rather than making this test quietly vacuous. 2.
/// On a run nobody has touched it is refused, and the sentence is
/// `gate::refusal`'s own by equality — P-0090, *a refusal a person can reach
/// from two surfaces is one sentence*, asserted against the function rather
/// than with a `contains`. It names the Mixer bay, because a model that is told
/// only *no* reports the instrument as incapable instead of as closed. 3. A
/// press on the Mixer bay's pill — through `Readout::pointer`, which is the
/// same routing a hand goes through, and not by calling `set` here — opens the
/// class. 4. The same call, the same audit, now allowed. Nothing about the
/// operation changed and nothing about the vocabulary changed; the list a model
/// reads never shortened at any point. 5. And exactly that class. The other
/// three are still shut and an operation in one of them is still refused, which
/// is the property the console's own
/// `a_press_opens_exactly_one_class_and_leaves_the_other_three_shut` makes
/// about the value and this one makes about the run. 6. A second press shuts
/// it, and the call is refused again — the other half of the page's *"click
/// again to shut it"*, seen from the gate.
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
/// it is in the mock's own reading, and it is one row. 2. Over the part of the
/// range both of them accept, which is `Set::published`'s intersection done off
/// the cards: `[0, 1]` and `[0.2, 0.8]` is one control over `[0.2, 0.8]`. 3. A
/// node with no card is counted and not skipped in silence, which is the foot's
/// `n without a card` and the one thing that keeps a knob missing for want of a
/// card from being a knob missing. 4. Nothing was compiled. The artifacts here
/// are not `.kir` at all — they are three bytes each — so a reading that
/// fetched and checked a source could not have answered at all, which is the
/// strongest form this claim can be put in.
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

/// The answer is written into the view, under the row the cursor is on, and a
/// Set that cannot be read says so and draws nothing.
///
/// [`read_reading`] is the glue the window loop runs on a press that asked for
/// a reading — [`listing`]'s shape one control along — and the two halves worth
/// a test are the ones a caller cannot see: that what is opened is the row
/// under the cursor rather than the first row, and that a failure closes the
/// block. A reading that failed to read and a Set that declares nothing must
/// not draw the same, which is `library`'s own rule one bay up.
///
/// A CPU test: a store is a directory and a `View` takes no device.
#[test]
fn a_reading_is_written_into_the_view_under_the_row_the_cursor_is_on() {
    let root = scratch_dir("read-set-view");
    let store = Store::open(&root).expect("a store to read");
    store.write_set("night01", &[]).expect("a Set to read");
    store.write_set("morph01", &[]).expect("a Set to read");

    let mut view = View::new(Room::Day);
    view.scopes = Scope::ALL.to_vec();
    view.library = vec!["night01".to_owned(), "morph01".to_owned()];
    assert!(
        view.walk(1, 0..2),
        "the cursor did not move off the first row"
    );

    let said = read_reading(&mut view, &root);
    assert!(
        said.contains("morph01"),
        "the reading names the first row rather than the one under the cursor: `{said}`"
    );
    let open = view.opened().expect("the reading is open under the cursor");
    assert_eq!(open.at, 1);
    assert_eq!(open.reading.id, "morph01");
    // A Set file with no `slot` record in it names no material, which is a
    // reading with nothing in it rather than a failure.
    assert_eq!(open.reading.nodes, 0);
    assert_eq!(open.reading.knobs_word(), "0 knobs");

    // **And a row naming a Set this store does not hold puts the block
    // away and says why**, where leaving the last reading drawn would
    // describe one Set under another's name.
    view.library = vec!["night01".to_owned(), "gone01".to_owned()];
    let said = read_reading(&mut view, &root);
    assert!(
        said.contains("gone01") && said.contains("could not be read"),
        "a Set that is not there was read as `{said}`"
    );
    assert!(
        !view.reading_open(),
        "a reading that failed to read left a block open"
    );

    std::fs::remove_dir_all(&root).expect("clean up");
}

/// [`reread_if_open`] re-reads on a move with a reading open, and does nothing
/// on any other press — ADR-0265's rule, as a check on the one function both
/// `App::window_event` call sites share, rather than on the two copies of it
/// the window loop used to carry.
///
/// Until 2026-09-11 the two call sites were two verbatim statements, held equal
/// to each other only by a text scan
/// (`reading_follows_the_cursor::both_surfaces_re_read_the_row_the_cursor_arrived_at`)
/// that read this file and matched each one whole. Now there is one statement
/// and not two to keep in step, and this presses it directly: three presses,
/// only the middle one of which is a move, and only the third of which should
/// read anything.
///
/// A CPU test: a store is a directory and a `View` takes no device.
#[test]
fn reread_if_open_re_reads_only_on_a_move_with_a_reading_open() {
    let root = scratch_dir("reread-if-open");
    let store = Store::open(&root).expect("a store to read");
    store.write_set("night01", &[]).expect("a Set to read");
    store.write_set("morph01", &[]).expect("a Set to read");

    let mut view = View::new(Room::Day);
    view.scopes = Scope::ALL.to_vec();
    view.library = vec!["night01".to_owned(), "morph01".to_owned()];

    // **No reading is open**, so a move re-reads nothing — there is
    // nothing for the rule to keep following.
    assert!(
        view.walk(1, 0..2),
        "the cursor did not move off the first row"
    );
    assert_eq!(
        reread_if_open(true, &mut view, &root),
        None,
        "a move with no reading open re-read something anyway"
    );

    // A reading opens on the row the cursor is on now (`morph01`).
    let _ = read_reading(&mut view, &root);
    assert!(view.reading_open());

    // **A press that did not move the cursor**, with a reading open: the
    // rule is the cursor's, so this is the one call `Readout::took` used
    // to get wrong by discarding the `bool` `View::point_at` handed back.
    assert_eq!(
        reread_if_open(false, &mut view, &root),
        None,
        "a press that did not move the cursor re-read anyway"
    );
    assert_eq!(
        view.opened().expect("still open").reading.id,
        "morph01",
        "a press that did not move the cursor changed which row is open"
    );

    // **A move, with the reading still open**: the row the cursor
    // arrives at is the one that comes back, on whichever surface's
    // `moved` said so.
    assert!(view.walk(-1, 0..2), "the cursor did not move back to row 0");
    let said =
        reread_if_open(true, &mut view, &root).expect("a reading was open and the cursor moved");
    assert!(
        said.contains("night01"),
        "reread_if_open read the row the cursor left rather than the one it arrived at: {said}"
    );
    assert_eq!(view.opened().expect("still open").reading.id, "night01");

    std::fs::remove_dir_all(&root).expect("clean up");
}

/// A press on the `params` chip asks for the Set under the cursor, and a second
/// press puts the reading away.
///
/// The seam, driven the way an operator drives it: the pointer arrives, the
/// panel claims it, and what comes back is
/// [`Operation::ReadSet`](karakuri_operation::Operation::ReadSet) naming the
/// row the cursor is on. `press_handler` cannot see this — it reads text and
/// asks whether the control is asked — and `karakuri-console`'s own tests
/// cannot see it either, because the press handler is here.
///
/// The close emits nothing, which is the half worth a test: a second press
/// changes which rows the bay draws and asks no question, so a `ReadSet` here
/// would be this program saying a Set was read at the moment one stopped being.
///
/// A CPU test: a `Readout` takes no device.
#[test]
fn a_press_on_the_params_chip_asks_for_the_set_under_the_cursor() {
    let ctx = drawn_once();
    let mut readout = Readout::new(1440.0, 900.0);
    readout.panel.solve();
    readout.view.scopes = Scope::ALL.to_vec();
    readout.view.library = vec!["drift_night".to_owned(), "lattice_veil".to_owned()];
    assert!(readout.view.select_scope(Scope::MySets));

    // The capsule, asked of the derivation that draws it rather than
    // remembered — the rule the whole of `input` is written to.
    let chip = |readout: &mut Readout| {
        readout.panel.solve();
        let at = library_bay(
            readout.panel.layout(),
            &readout.view.scopes,
            &readout.view.library,
            readout.view.opened(),
            readout.view.pointed(),
            readout.view.library_scroll(),
        )
        .expect("the bay lists its rows")
        .params_chip(&ctx, readout.view.target());
        Point::new(at.min.x + 2.0, at.center().y)
    };

    let at = chip(&mut readout);
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
    let (claim, did) = readout.pointer(&ctx, Pointer::Down);
    assert_eq!(claim, Claim::Panel);
    assert_eq!(
        did,
        Acted::Emitted(Some(Operation::ReadSet {
            id: "drift_night".to_owned()
        })),
        "the chip did not ask for the Set under the cursor"
    );

    // **The reading is the caller's to answer**, and this file's press
    // handler holds no store — so the block is opened here the way the
    // window loop opens it, and the second press is what is under test.
    readout.view.read(Reading {
        id: "drift_night".to_owned(),
        ..Reading::default()
    });
    let at = chip(&mut readout);
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
    let (claim, did) = readout.pointer(&ctx, Pointer::Down);
    assert_eq!(claim, Claim::Panel);
    assert_eq!(
        did,
        Acted::Nothing,
        "closing a reading emitted an operation, which says a Set was read"
    );
    assert!(
        !readout.view.reading_open(),
        "the second press did not put the reading away"
    );
}

/// A strip, as far as the Library bay cares: something for a deck to be named
/// on. Every reading in it is beside the point here.
fn bare_strip() -> view::Strip {
    view::Strip {
        name: String::new(),
        tally: view::Tally::Allocated,
        requested: view::Tally::Allocated,
        gain: 0.0,
        gain_to: None,
        opacity: 0.0,
        opacity_to: None,
        blend: BlendMode::Add,
        mask: view::Mask::None,
        mask_angle: 0.0,
        level: None,
        is_muted: false,
        is_soloed: false,
    }
}

/// A secondary press on a Library row puts that row's menu down, and a primary
/// press on the same row does not.
///
/// This is the one thing neither crate could assert on its own.
/// `karakuri-console` has never known which button a press was — rules 1 to 4
/// of `input::claim` are all about where the pointer is — so the distinction
/// lives here, in the arm that turns a `winit` button into a [`Pointer`]. Both
/// halves are asserted because a handler that opened the menu on either button
/// would pass a test made only of the first, and would take the row's ordinary
/// press away: a primary press picks a Set up to carry it, which is what the
/// drag onto a strip is.
///
/// And the item is picked with either button, which is the other half of the
/// same seam: once the card is down it is `input::claim`'s rule 2, and that
/// rule is about a card being down rather than about what put it there. So the
/// pick here is a *primary* press on a card a secondary press opened.
///
/// A CPU test: a `Readout` takes no device.
#[test]
fn a_secondary_press_opens_a_rows_menu_and_a_primary_press_does_not() {
    let ctx = drawn_once();
    let mut readout = Readout::new(1440.0, 900.0);
    readout.panel.solve();
    readout.view.scopes = Scope::ALL.to_vec();
    readout.view.library = vec!["drift_night".to_owned(), "lattice_veil".to_owned()];
    readout.view.mixer = std::iter::repeat_with(bare_strip).take(4).collect();
    assert!(readout.view.select_scope(Scope::MySets));

    // The second row, asked of the derivation that draws it.
    let row = |readout: &mut Readout| {
        readout.panel.solve();
        let at = library_bay(
            readout.panel.layout(),
            &readout.view.scopes,
            &readout.view.library,
            readout.view.opened(),
            readout.view.pointed(),
            readout.view.library_scroll(),
        )
        .expect("the bay lists its rows")
        .row(1);
        Point::new(at.center().x, at.center().y)
    };

    // **A primary press takes the Set in hand and opens nothing.**
    let row_at = row(&mut readout);
    let at = row_at;
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
    readout.pointer(&ctx, Pointer::Down);
    assert!(
        matches!(readout.panel.in_hand(), Some(InHand::Carrying)),
        "a primary press on a row did not take the Set in hand"
    );
    assert!(
        !readout.view.menu_open(),
        "a primary press on a row put that row's menu down, which takes the carry away"
    );
    // Let the carry go again, over nothing, so the gesture does not run on
    // into the presses below.
    readout.pointer(&ctx, Pointer::Up);

    // **A secondary press on the same row puts the menu down.**
    assert_eq!(readout.pointer(&ctx, Pointer::Secondary).1, Acted::Nothing);
    assert!(
        readout.view.menu_open(),
        "a secondary press on a row did not put that row's menu down"
    );
    assert_eq!(
        readout.view.menued().row,
        Some(1),
        "the menu came down on a row the press was not on"
    );

    // **A press on another control's capsule dismisses this card rather
    // than opening that one.** The `load` button is the sharpest case
    // there is: it is in this bay's own foot, it is a control the pointer
    // reaches, and a handler that asked it before the card would have
    // opened the pulldown with a menu still down — two cards down at once,
    // which is the one thing `input::claim`'s rule 2 exists to make
    // impossible. This is the assertion that says which was asked first.
    readout.panel.solve();
    let button = library_bay(
        readout.panel.layout(),
        &readout.view.scopes,
        &readout.view.library,
        readout.view.opened(),
        readout.view.pointed(),
        readout.view.library_scroll(),
    )
    .expect("the bay draws its foot")
    .load(&ctx, readout.view.target())
    .button;
    let at = Point::new(button.center().x, button.center().y);
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
    assert_eq!(readout.pointer(&ctx, Pointer::Down).1, Acted::Nothing);
    assert!(
        !readout.view.menu_open(),
        "a press on the `load` button with a menu down did not dismiss it"
    );
    assert!(
        !readout.view.target_open(),
        "a press on the `load` button with a menu down opened the pulldown as well, so two \
         cards were down at once"
    );

    // Open it again, on the same row, for the pick below.
    assert_eq!(
        readout.pointer(&ctx, Pointer::Moved(row_at)).0,
        Claim::Panel
    );
    assert_eq!(readout.pointer(&ctx, Pointer::Secondary).1, Acted::Nothing);
    assert_eq!(readout.view.menued().row, Some(1));

    // **And the send is picked with a primary press on the card**, which
    // is rule 2: the card is down, so the press is the card's whichever
    // button it was.
    readout.panel.solve();
    let bay = library_bay(
        readout.panel.layout(),
        &readout.view.scopes,
        &readout.view.library,
        readout.view.opened(),
        readout.view.pointed(),
        readout.view.library_scroll(),
    )
    .expect("the bay lists its rows");
    let menu = bay
        .menu(
            &ctx,
            view::to_egui(readout.panel.layout().viewport()),
            readout.view.menued(),
        )
        .expect("the menu is down");
    let save = menu.save.expect("a Set row's menu carries a send").center();
    let at = Point::new(save.x, save.y);
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
    let (claim, did) = readout.pointer(&ctx, Pointer::Down);
    assert_eq!(claim, Claim::Panel);
    assert_eq!(
        did,
        Acted::Emitted(Some(Operation::TransferSet {
            transfer: SetTransfer::Send {
                id: "lattice_veil".to_owned()
            }
        })),
        "`Save as a kbset` did not ask to send the row the menu was opened on"
    );
    assert!(
        !readout.view.menu_open(),
        "the card stayed down after an item was picked"
    );
}

/// A send that reached the disk says where it went, and a dismissed dialog
/// writes nothing and says so.
///
/// Three outcomes, one sentence each, and the third is the one worth the test:
/// a press that opened a window over the panel and then wrote nothing is
/// exactly the case a reader would otherwise read as a fault, and rule 04 of
/// the manual is that nothing is hidden quietly.
///
/// The dialog is not driven here and does not need to be. What a save dialog
/// answers is a path or nothing, so [`sent`] takes that answer and the platform
/// stays outside the test — the same split [`Save::run`] is on one act along,
/// where the thread is the caller's and the write is a function.
///
/// `None` is asserted to have written nothing at all, by counting the directory
/// rather than by trusting the sentence: a `sent` that bundled first and threw
/// the bytes away would print the same words.
///
/// A CPU test: a store read and a file written.
#[test]
fn a_send_says_where_it_went_and_a_dismissed_dialog_writes_nothing_and_says_so() {
    let root = scratch_dir("send-set");
    let store = Store::open(&root).expect("a store");
    // One Set with one node, so a bundle has a source to inline.
    let hash = store
        .put_artifact(b"proc p { }\n")
        .expect("the source is stored");
    store
        .write_set(
            "night01",
            &[karakuri_store::ndjson::Line::new(Record::Slot {
                at: karakuri_store::record::NodeAddress {
                    layer: karakuri_store::record::Layer::L1,
                    index: 0,
                },
                name: Some("geo".to_owned()),
                proc_hash: hash,
            })],
        )
        .expect("the set is written");

    let out = root.join("outbox");
    std::fs::create_dir_all(&out).expect("an outbox");

    // **Dismissed**: nothing is asked of the disk and the sentence says so.
    let said = sent(&root, "night01".to_owned(), None);
    assert_eq!(said.to, None);
    assert!(
        said.said().contains("dismissed") && said.said().contains("was not written"),
        "a dismissed dialog was reported as `{}`",
        said.said()
    );
    assert_eq!(
        std::fs::read_dir(&out).expect("the outbox").count(),
        0,
        "a dismissed dialog left a file behind"
    );

    // **Written**: the file is where the operator sent it and carries the
    // source inlined, which is what makes it a bundle rather than a copy.
    let to = out.join("night01.kbset");
    let said = sent(&root, "night01".to_owned(), Some(to.clone()));
    assert_eq!(
        said.outcome,
        Ok(()),
        "the send was refused: {}",
        said.said()
    );
    assert_eq!(
        said.said(),
        format!("  send: `night01` written to `{}`", to.display())
    );
    let text = std::fs::read_to_string(&to).expect("the bundle is on the disk");
    assert!(
        text.contains("proc p"),
        "the file names the source rather than carrying it: {text}"
    );

    // **Refused**: a Set this store does not hold, and the words are the
    // bundler's rather than a second copy of them.
    let said = sent(&root, "gone01".to_owned(), Some(out.join("gone01.kbset")));
    assert!(said.outcome.is_err(), "a Set nobody holds was packaged");
    assert!(
        said.said().contains("gone01") && said.said().contains("was not written to"),
        "a refused send was reported as `{}`",
        said.said()
    );

    std::fs::remove_dir_all(&root).expect("clean up");
}

/// The marks a reading is spelled with are in the face the panel draws with.
///
/// [`spelled`] writes `0 – 8 · 2` with an en dash and a middle dot, and the
/// Library bay's old `load → A` pill is what this test exists because of: its
/// arrow was typed with a U+2192 `egui`'s default face does not carry, and the
/// panel drew `load □ A` for a release — *a readout of where a press lands,
/// with a tofu where the lands was*. A range with a tofu in it would be the
/// same failure on every row of every reading.
///
/// The minus is here too, because a declared range can start below zero — the
/// mock's own `twist` is `−2 – 2 · 0` — and it is the sign `format!` writes
/// rather than the typographic one, which is asserted so that the two are not
/// quietly swapped.
///
/// A CPU test: fonts are `egui`'s and take no device.
#[test]
fn the_marks_a_reading_is_spelled_with_are_in_the_face() {
    let ctx = drawn_once();
    let spelling = spelled("-2", "2", Some("0".to_owned()));
    assert_eq!(spelling, "-2 – 2 · 0");
    let font = egui::FontId::new(
        karakuri_console::room::size::BASE,
        egui::FontFamily::Proportional,
    );
    for mark in spelling.chars() {
        assert!(
            ctx.fonts_mut(|fonts| fonts.has_glyphs(&font, &mark.to_string())),
            "`{mark}` is not in the default face, and the panel would draw a tofu where a \
             reading's range is"
        );
    }
}

/// A press on the outputs dot, through the window loop's own routing.
///
/// The other half of the test above: that one is a boundary the panel claims
/// and `egui` never sees, and this is the console's one control, which the
/// panel claims for a different reason — `egui` owns no widget anywhere here,
/// so a press routed to it would reach nothing at all.
///
/// What is asserted is the round trip an operator makes: the picture is on
/// screen, a click on the dot folds it away by name, and a click on the same
/// dot brings it back. The dot is where it is drawn and the press is the
/// panel's at every step.
#[test]
fn a_press_on_the_outputs_dot_folds_the_picture_and_unfolds_it() {
    let ctx = drawn_once();
    let mut readout = Readout::new(1440.0, 900.0);
    readout.panel.solve();
    let picture = readout
        .panel
        .layout()
        .find("program-view")
        .expect("program-view");
    let dot = |readout: &mut Readout| {
        readout.panel.solve();
        let row =
            outputs(&ctx, readout.panel.layout(), Open::CLOSED).expect("the row draws its sink");
        (Point::new(row.sink.center().x, row.sink.center().y), row.on)
    };

    let (at, on) = dot(&mut readout);
    assert!(on, "the picture is on screen, so the sink is on");

    // The pointer arrives, and the control is the panel's.
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
    let (claim, did) = readout.pointer(&ctx, Pointer::Down);
    assert_eq!(claim, Claim::Panel);
    assert_eq!(
        did,
        Acted::Operated(Outcome::Folded {
            id: picture,
            folded: true,
            root: false
        }),
        "the press did not reach the sink"
    );
    assert!(
        !readout.panel.dragging(),
        "the press took a boundary in hand"
    );
    readout.pointer(&ctx, Pointer::Up);

    // And the dot is dark, where it still is, and turns the picture back
    // on rather than unfolding whatever else is folded.
    let (at, on) = dot(&mut readout);
    assert!(!on, "the picture is folded and the sink is still lit");
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
    assert_eq!(
        readout.pointer(&ctx, Pointer::Down).1,
        Acted::Operated(Outcome::Folded {
            id: picture,
            folded: false,
            root: false
        }),
        "the dark dot did not turn the picture back on"
    );
    assert!(
        dot(&mut readout).1,
        "the picture is back and the dot is dark"
    );
}

/// A press on a scope chip, through the window loop's own routing — which is
/// what ADR-0213 makes the *panel* badge mean.
///
/// `karakuri-console`'s `tests/library.rs` asserts everything up to the
/// operation with no window anywhere: that the chips answer a press, that a
/// press names the chip it landed on, and that nothing else in the bay takes
/// one. This is the half that badge is actually about — *"the row is claimed
/// the day a person who launched the instrument can perform that operation from
/// the panel in front of them"* — and a control demonstrated in that crate and
/// never wired here would pass there and be a lie this page tells on its own
/// authority.
///
/// What is asserted is the whole press and not the routing alone: the mark
/// moves to the chip that was pressed, the operation that leaves is
/// `SelectScope` with the payload it is specified to carry, and the library
/// cursor goes back to the top — because the listing under a new scope is a
/// listing this cursor has never seen, and a cursor left where it was would sit
/// on a Set nobody chose under a pill saying a press will load it.
///
/// And the chip that is already marked is pressed too, because that is the case
/// a step cannot reach: a step would go somewhere else, and a pointer names —
/// so the press is answered rather than refused, and the mark stays where it
/// is.
#[test]
fn a_press_on_a_scope_chip_names_the_library_the_bay_reads() {
    let ctx = drawn_once();
    let mut readout = Readout::new(1440.0, 900.0);
    readout.panel.solve();
    // A console that has been told what libraries there are and handed a
    // listing for the one it opens on, which is what `resumed` does.
    readout.view.scopes = Scope::ALL.to_vec();
    readout.view.library = vec!["drift_night".to_owned(), "lattice_veil".to_owned()];
    assert!(readout.view.select_scope(Scope::MySets));

    // The capsule, asked of the derivation that draws it rather than
    // remembered — the rule the whole of `input` is written to.
    let chip = |readout: &mut Readout, want: Scope| {
        readout.panel.solve();
        let bay = library_bay(
            readout.panel.layout(),
            &readout.view.scopes,
            &readout.view.library,
            readout.view.opened(),
            readout.view.pointed(),
            readout.view.library_scroll(),
        )
        .expect("the bay lists its rows");
        let (_, at) = bay
            .chips(&ctx, &readout.view.scopes)
            .find(|(scope, _)| *scope == want)
            .expect("the scope is on the row");
        // Two pixels in from its own left edge: the last chip in the row
        // is clipped by the pane, so its centre can be off the row.
        Point::new(at.min.x + 2.0, at.center().y)
    };

    // **The cursor is somewhere other than the top**, so that the move
    // back to it is a move and not the state it was already in.
    assert!(readout.view.walk(1, 0..2), "the cursor did not move");
    assert_eq!(readout.view.cursor_row(), 1);

    let at = chip(&mut readout, Scope::Presets);
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
    let (claim, did) = readout.pointer(&ctx, Pointer::Down);
    assert_eq!(claim, Claim::Panel);
    assert_eq!(
        did,
        Acted::Emitted(Some(Operation::SelectScope { scope: Undecided })),
        "the press did not reach the chip"
    );
    assert!(
        !readout.panel.dragging(),
        "the press took a boundary in hand"
    );
    readout.pointer(&ctx, Pointer::Up);
    assert_eq!(
        readout.view.scope(),
        Some(Scope::Presets),
        "the press was routed and the mark stayed where it was"
    );
    assert_eq!(
        readout.view.cursor_row(),
        0,
        "the scope changed and the cursor is still pointing into the listing it left"
    );

    // **The marked chip, pressed** — answered rather than refused, and the
    // mark does not step off it the way the key would.
    let at = chip(&mut readout, Scope::Presets);
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
    assert_eq!(
        readout.pointer(&ctx, Pointer::Down).1,
        Acted::Emitted(Some(Operation::SelectScope { scope: Undecided })),
        "a press on the chip that is already marked was not answered"
    );
    readout.pointer(&ctx, Pointer::Up);
    assert_eq!(
        readout.view.scope(),
        Some(Scope::Presets),
        "a press on the marked chip stepped somewhere"
    );
}

/// A press on the star at the left of a library row, through the window loop's
/// own routing — the half of the badge ADR-0213 makes a badge mean, beside
/// [`a_press_on_a_filter_field_asks_the_store_for_a_narrower_listing`].
///
/// `karakuri-console`'s `tests/library.rs` says where the mark is and that it
/// answers a press; this says an operator reaches it — a control demonstrated
/// in that crate and never wired here would pass there and be a lie the page
/// tells on its own authority.
///
/// What is asserted is that the press names a state and not a step (ADR-0299):
/// the same mark pressed twice asks for two different things, because the
/// control reads the row's present mark and asks for the other one. That is the
/// failure a toggle hides completely — a press that always emitted `true` would
/// pass every assertion about the first press and never take a star off.
///
/// And that the star has not swallowed the row it sits in: a press on the row's
/// own ground still takes the Set in hand and names no operation, which is rule
/// 4's *a control claims what it acts on and no more* asked of the two boxes
/// that overlap.
///
/// A CPU test: a `Readout` takes no device.
#[test]
fn a_press_on_a_star_names_the_state_the_row_is_not_in() {
    let ctx = drawn_once();
    let mut readout = Readout::new(1440.0, 900.0);
    readout.panel.solve();
    readout.view.scopes = Scope::ALL.to_vec();
    readout.view.library = vec!["drift_night".to_owned(), "lattice_veil".to_owned()];

    // The mark's box, asked of the derivation that draws it rather than
    // remembered — the rule the whole of `input` is written to.
    let star = |readout: &mut Readout, index: usize| {
        readout.panel.solve();
        let at = library_bay(
            readout.panel.layout(),
            &readout.view.scopes,
            &readout.view.library,
            readout.view.opened(),
            readout.view.pointed(),
            readout.view.library_scroll(),
        )
        .expect("the bay lists its rows")
        .star(index);
        Point::new(at.center().x, at.center().y)
    };

    // **Nothing starred, so the press asks for the star to go on.**
    let at = star(&mut readout, 1);
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
    let (claim, did) = readout.pointer(&ctx, Pointer::Down);
    assert_eq!(claim, Claim::Panel);
    assert_eq!(
        did,
        Acted::Emitted(Some(Operation::SetFavourite {
            id: "lattice_veil".to_owned(),
            favourite: true,
        })),
        "the press did not reach the star, or it named the wrong row"
    );
    assert!(
        !readout.panel.dragging(),
        "the press took a boundary in hand"
    );
    readout.pointer(&ctx, Pointer::Up);

    // **And with the row starred it asks for the star to come off**, which
    // is the same control reading the state it is drawn from. The marks
    // are the host's answer, so this is what `listing` would have written
    // after the write.
    readout.view.starred.insert("lattice_veil".to_owned());
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
    assert_eq!(
        readout.pointer(&ctx, Pointer::Down).1,
        Acted::Emitted(Some(Operation::SetFavourite {
            id: "lattice_veil".to_owned(),
            favourite: false,
        })),
        "a starred row was asked to be starred again"
    );
    readout.pointer(&ctx, Pointer::Up);

    // **The row's own ground is still the row's.** A press at the far end
    // of the same row takes the Set in hand and names no operation, which
    // is what a carry is (ADR-0265).
    readout.panel.solve();
    let row = library_bay(
        readout.panel.layout(),
        &readout.view.scopes,
        &readout.view.library,
        readout.view.opened(),
        readout.view.pointed(),
        readout.view.library_scroll(),
    )
    .expect("the bay lists its rows")
    .row(1);
    let ground = Point::new(row.max.x - 4.0, row.center().y);
    assert_eq!(
        readout.pointer(&ctx, Pointer::Moved(ground)).0,
        Claim::Panel
    );
    let (claim, did) = readout.pointer(&ctx, Pointer::Down);
    assert_eq!(claim, Claim::Panel);
    assert!(
        !matches!(did, Acted::Emitted(Some(Operation::SetFavourite { .. }))),
        "a press on the row's own ground was answered by the star in it: {did:?}"
    );
    readout.pointer(&ctx, Pointer::Up);
}

/// A press on one of the Library bay's two filter fields, through the window
/// loop's own routing — the half of the badge ADR-0213 makes a badge mean, one
/// row under [`a_press_on_a_scope_chip_names_the_library_the_bay_reads`].
///
/// `karakuri-console`'s `tests/library.rs` asserts everything up to the
/// operation with no window anywhere: where the two fields are, that each steps
/// its own cycle, that the operation names where it arrived and carries the
/// other field untouched, and that nothing between them takes a press. This is
/// the half that says an operator reaches it — a control demonstrated in that
/// crate and never wired here would pass there and be a lie the page tells on
/// its own authority, which is exactly what `press_handler` was written after.
///
/// What is asserted is the whole press. The operation that leaves is `ListSets`
/// carrying the step; `View::narrow` has been called, so the field the panel
/// draws next frame reads the new value; and the library cursor is back at the
/// top, because the listing under a narrower filter is one this cursor has
/// never seen.
///
/// Both fields, because the operation carries both halves: the press on `layer`
/// has to come back with the `holds` the press before it set, and a route that
/// rebuilt the operation from one field would lose it.
///
/// A CPU test: a `Readout` takes no device.
#[test]
fn a_press_on_a_filter_field_asks_the_store_for_a_narrower_listing() {
    use karakuri_console::view::Field;

    let ctx = drawn_once();
    let mut readout = Readout::new(1440.0, 900.0);
    readout.panel.solve();
    // A console told what libraries there are, handed the listing for the
    // one it opens on and told what that listing's Sets are made of —
    // which is what `listing` does on this side of the seam.
    readout.view.scopes = Scope::ALL.to_vec();
    readout.view.library = vec!["drift_night".to_owned(), "lattice_veil".to_owned()];
    readout.view.holds = vec!["drift_shell".to_owned(), "soft_points".to_owned()];
    assert!(readout.view.select_scope(Scope::MySets));
    // **The cursor is somewhere other than the top**, so that the move back
    // to it is a move and not the state it was already in.
    assert!(readout.view.walk(1, 0..2), "the cursor did not move");

    // The box, asked of the derivation that draws it rather than
    // remembered — the rule the whole of `input` is written to.
    let field = |readout: &mut Readout, which: Field| {
        readout.panel.solve();
        let at = library_bay(
            readout.panel.layout(),
            &readout.view.scopes,
            &readout.view.library,
            readout.view.opened(),
            readout.view.pointed(),
            readout.view.library_scroll(),
        )
        .expect("the bay lists its rows")
        .field(which)
        .expect("the bay draws its filter field");
        Point::new(at.center().x, at.center().y)
    };
    // **And one kind chip's, asked of the walk that paints them**, which
    // is the same rule one band down: a chip is as wide as the word in it,
    // so where it is is `egui`'s answer and never a remembered number.
    let kind_chip = |readout: &mut Readout, which: usize| {
        readout.panel.solve();
        let bay = library_bay(
            readout.panel.layout(),
            &readout.view.scopes,
            &readout.view.library,
            readout.view.opened(),
            readout.view.pointed(),
            readout.view.library_scroll(),
        )
        .expect("the bay lists its rows");
        let at = bay
            .kind_chips(&ctx)
            .nth(which)
            .expect("the bay draws six kind chips")
            .1;
        Point::new(at.center().x, at.center().y)
    };

    let at = field(&mut readout, Field::Holds);
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
    let (claim, did) = readout.pointer(&ctx, Pointer::Down);
    assert_eq!(claim, Claim::Panel);
    assert_eq!(
        did,
        Acted::Emitted(Some(Operation::ListSets {
            holds: Some("drift_shell".to_owned()),
            layer: None,
        })),
        "the press did not reach the `holds` field"
    );
    assert!(
        !readout.panel.dragging(),
        "the press took a boundary in hand"
    );
    readout.pointer(&ctx, Pointer::Up);
    assert_eq!(
        readout.view.filters().holds,
        Some("drift_shell"),
        "the press was routed and the field it stepped does not read it"
    );
    assert_eq!(
        readout.view.cursor_row(),
        0,
        "the listing narrowed and the cursor is still pointing into the one it left"
    );

    // **And the kind chips under it, which have to carry the field
    // through**: a press on a chip names all six and says nothing about
    // `holds`, so what the console is narrowed to afterwards is the pair as
    // it now stands (ADR-0338).
    let at = kind_chip(&mut readout, 2);
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
    assert_eq!(
        readout.pointer(&ctx, Pointer::Down).1,
        Acted::Emitted(Some(Operation::FilterLibrary {
            kinds: karakuri_operation::LibraryKinds {
                l3: true,
                ..karakuri_operation::LibraryKinds::EVERYTHING
            },
        })),
        "the press did not reach the `L3` chip, or it named something other than all six"
    );
    readout.pointer(&ctx, Pointer::Up);
    assert!(readout.view.filters().kinds.l3);
    assert_eq!(readout.view.filters().holds, Some("drift_shell"));
}

/// A press on a strip's ground addresses the keys to that deck — the whole
/// route, through the same `Readout::pointer` a hand goes through.
///
/// # Why it is here and can be nowhere else
///
/// `karakuri-console` owns both halves and cannot put them together:
/// `input::claim` there says a press on a strip is the panel's, and
/// `Mixer::select` says which deck it names, and nothing in that crate joins
/// the two. This file's press arm is the join, and the defect this was written
/// against lived exactly in the gap: `on_strip` asked four questions where the
/// bay has five, so every press on a strip's ground was routed to `egui`, the
/// `(Pointer::Down, Claim::Panel)` arm never ran, and the `bay.select(at)` call
/// at the end of it was unreachable — while `operations.html`'s *"click a
/// strip"*, `console.html`'s strip tip, `Mixer::select` and that call all said
/// it worked.
///
/// Deleting `|| bay.select(p).is_some()` from `input::on_strip` is the
/// injection this was watched to fail against: the claim comes back `Egui` and
/// the press asks for nothing.
///
/// The point is the strip's name box, which is the affordance's own words — *"a
/// press anywhere on this strip that no knob under the pointer claimed"* — and
/// the four that could have claimed it are asked here so that the press under
/// test is the leftover rather than a chip.
///
/// A CPU test: a `Readout` takes no device.
#[test]
fn a_press_on_a_strips_ground_selects_that_deck() {
    let ctx = drawn_once();
    let mut readout = Readout::new(1440.0, 900.0);
    readout.panel.solve();
    readout.view.mixer = (0..4)
        .map(|slot| view::Strip {
            name: format!("slot{slot}"),
            tally: view::Tally::Live,
            requested: view::Tally::Live,
            gain: 0.5,
            gain_to: None,
            opacity: 0.5,
            opacity_to: None,
            blend: BlendMode::Over,
            mask: view::Mask::None,
            mask_angle: 0.0,
            level: None,
            is_muted: false,
            is_soloed: false,
        })
        .collect();
    assert_eq!(
        readout.view.selection(),
        0,
        "the selection is not deck A, so a press naming deck C could be naming the \
         selection it started on"
    );

    // The rectangle, off the derivation that draws it — the rule the whole
    // of `input` is written to, and the reason a test presses where the
    // paint painted.
    let bay = mixer_bay(&ctx, readout.panel.layout(), &readout.view.mixer)
        .expect("the mixer bay draws its strips");
    let name = bay.strip(2).name;
    let at = Point::new(name.center().x, name.center().y);
    assert!(
        bay.grab(at).is_none()
            && bay.blend(at).is_none()
            && bay.tally(at).is_none()
            && bay.mask(at).is_none(),
        "the name box is one of the four controls inside the column, so this press is \
         not the leftover the selection is made of"
    );

    assert_eq!(
        readout.pointer(&ctx, Pointer::Moved(at)).0,
        Claim::Panel,
        "a press on a strip went to egui, so the panel's own press arm never runs"
    );
    let (claim, did) = readout.pointer(&ctx, Pointer::Down);
    assert_eq!(claim, Claim::Panel, "a press on a strip went to egui");
    assert_eq!(
        did,
        Acted::Emitted(Some(Operation::SelectDeck { deck: 2 })),
        "the press did not address the keys to the deck the strip is"
    );

    // **The selection moves where the operation is performed**, which is
    // `pointed` and not the press arm: `SelectDeck` writes no record, so
    // the surface that emits it is what performs it (ADR-0198).
    assert_eq!(
        readout.view.selection(),
        0,
        "the press moved the pointer itself"
    );
    assert!(pointed(&mut readout.view, &Operation::SelectDeck { deck: 2 }).is_some());
    assert_eq!(readout.view.selection(), 2);
}

/// A Set dragged from a library row onto a mixer strip loads the strip it was
/// let go over — the whole gesture, through the same `Readout::pointer` a hand
/// goes through.
///
/// # Why it is here and can be nowhere else
///
/// `karakuri-console` has both halves of the gesture and cannot put them
/// together: `carry.rs` there presses the model and the view directly, and
/// hands the destination in itself, because that crate has no press handler to
/// ask. The property that matters is which *moment* resolves the deck, and that
/// is this file's: the press is over the Library bay, where there is no strip
/// at all, and the release is over one. So a destination taken at the press
/// names nothing and the drop is cancelled, and a destination taken at the
/// release names the strip under the hand.
///
/// Deleting the `Mixer::dropped` ask from the release arm is the injection this
/// was watched to fail against, and moving it into the press arm is the second
/// — the first answers `Nowhere` for every drop and the second answers it for
/// every drop that began in the library, which is all of them.
///
/// # What it asserts, in the order a hand does it
///
/// 1. A press on the third row is the panel's, and it emits nothing: half a
/// gesture names one operand. 2. The cursor mark follows the hand, and
/// `Acted::Pointed` is the press saying so. It is no longer the whole of what
/// this console draws for a carry — the rectangle under the pointer is ringed
/// and the pointer is a grab, which is `View::draw`'s and is held by
/// `karakuri-console/tests/carry.rs`; the mock still draws no ghost. What is
/// owed on that answer when a reading is open is
/// [`a_carry_that_moves_the_cursor_re_reads_the_row_it_arrived_at`]; here it is
/// the mark alone, and `Acted::Nothing` in its place would be a press that
/// moved the cursor and told nobody. 3. Every move on the way is
/// `Acted::Nothing`, over two strips that are not the one it lands on. 4. The
/// release over strip C asks for `LoadSet` naming deck C and the Set from row 2
/// — not the selection, which is deck A throughout, and not the row the cursor
/// started on. 5. A second carry let go over nothing asks for nothing, which is
/// the outcome no other drag on this panel has.
///
/// A CPU test: a `Readout` takes no device.
#[test]
fn a_drop_on_a_strip_loads_the_strip_it_was_let_go_over() {
    let ctx = drawn_once();
    let mut readout = Readout::new(1440.0, 900.0);
    readout.panel.solve();
    readout.view.scopes = Scope::ALL.to_vec();
    readout.view.library = vec![
        "drift_night".to_owned(),
        "lattice_veil".to_owned(),
        "glass_shell".to_owned(),
    ];
    readout.view.mixer = (0..4)
        .map(|slot| view::Strip {
            name: format!("slot{slot}"),
            tally: view::Tally::Live,
            requested: view::Tally::Live,
            gain: 0.5,
            gain_to: None,
            opacity: 0.5,
            opacity_to: None,
            blend: BlendMode::Over,
            mask: view::Mask::None,
            mask_angle: 0.0,
            level: None,
            is_muted: false,
            is_soloed: false,
        })
        .collect();
    assert_eq!(
        readout.view.selection(),
        0,
        "the selection is not deck A, so a drop naming deck C could be naming the selection"
    );

    // The rectangles, off the derivations that draw them — the rule the
    // whole of `input` is written to, and the reason a test presses where
    // the paint painted.
    let row = |readout: &mut Readout, index: usize| {
        readout.panel.solve();
        let at = library_bay(
            readout.panel.layout(),
            &readout.view.scopes,
            &readout.view.library,
            readout.view.opened(),
            readout.view.pointed(),
            readout.view.library_scroll(),
        )
        .expect("the bay lists its rows")
        .row(index);
        Point::new(at.center().x, at.center().y)
    };
    let strip = |readout: &mut Readout, deck: u8| {
        readout.panel.solve();
        let at = mixer_bay(&ctx, readout.panel.layout(), &readout.view.mixer)
            .expect("the mixer bay draws its strips")
            .selected(deck)
            .expect("a strip for the deck");
        Point::new(at.center().x, at.center().y)
    };

    // 1 and 2: the press takes row 2 in hand, asks for nothing, and moves
    // the mark to the row the hand is on.
    let at = row(&mut readout, 2);
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
    let (claim, did) = readout.pointer(&ctx, Pointer::Down);
    assert_eq!(claim, Claim::Panel, "a press on a library row went to egui");
    assert_eq!(
        did,
        Acted::Pointed,
        "the press asked for an operation, or moved the mark without answering that it did"
    );
    assert_eq!(
        readout.view.cursor_row(),
        2,
        "the mark did not follow the hand to the row it took"
    );

    // 3: nothing is emitted on the way, including over two strips it does
    // not land on.
    for over in [strip(&mut readout, 0), strip(&mut readout, 1)] {
        let (claim, did) = readout.pointer(&ctx, Pointer::Moved(over));
        assert_eq!(claim, Claim::Panel, "the carry lost its claim");
        assert_eq!(
            did,
            Acted::Nothing,
            "a move with a Set in hand asked for something"
        );
    }

    // 4: and the release names the strip it is over.
    let onto = strip(&mut readout, 2);
    readout.pointer(&ctx, Pointer::Moved(onto));
    assert_eq!(
        readout.pointer(&ctx, Pointer::Up).1,
        Acted::Emitted(Some(Operation::LoadSet {
            deck: 2,
            set: "glass_shell".to_owned(),
        })),
        "the drop did not name the strip it was let go over"
    );
    assert!(!readout.panel.dragging(), "the carry is still in hand");

    // 5: and one let go over nothing asks for nothing at all.
    let at = row(&mut readout, 0);
    readout.pointer(&ctx, Pointer::Moved(at));
    assert_eq!(readout.pointer(&ctx, Pointer::Down).1, Acted::Pointed);
    let away = Point::new(-40.0, -40.0);
    readout.pointer(&ctx, Pointer::Moved(away));
    assert_eq!(
        readout.pointer(&ctx, Pointer::Up).1,
        Acted::Nothing,
        "a drop over nothing asked for a load"
    );
}

/// A Set let go on a deck preview cell loads that cell's deck, through the same
/// `Readout::pointer` a hand goes through — and a cell whose letter names no
/// slot loads nothing.
///
/// # Why it is here and not in `karakuri-console`
///
/// `carry.rs` there asks the two bays itself and hands the destination to
/// `Panel::released`. What this file owns is that the release asks the second
/// bay at all: the press handler resolved the drop against `Mixer::dropped`
/// alone until ADR-0273, so a carry that crossed to the centre column and let
/// go on a cell was answered `Nowhere` — the panel drawing a ring round a
/// rectangle the release then declined to use. Deleting the
/// `ProgramBay::dropped` ask from the release arm is the injection this was
/// watched to fail against.
///
/// # And the slot count is asked with it
///
/// The deck here has three slots and the row is four cells, so cell D is drawn
/// with nothing behind the letter on it. A release there names no deck, which
/// is the refusal `3` already gets from the keyboard — `pointed`, off the same
/// `View::mixer` length. Passing `DECKS` instead of that length is the second
/// injection, and it asks for `LoadSet { deck: 3 }` on a deck that has no slot
/// 3.
///
/// A CPU test: a `Readout` takes no device.
#[test]
fn a_drop_on_a_preview_cell_loads_the_deck_its_letter_names() {
    let ctx = drawn_once();
    let mut readout = Readout::new(1440.0, 900.0);
    // The Program bay's cells are read from the bit `rearrange` writes, so
    // a frame's own first act is what puts them anywhere at all.
    view::rearrange(&mut readout.panel, readout.view.canvas);
    readout.panel.solve();
    readout.view.scopes = Scope::ALL.to_vec();
    readout.view.library = vec![
        "drift_night".to_owned(),
        "lattice_veil".to_owned(),
        "glass_shell".to_owned(),
    ];
    readout.view.mixer = (0..3)
        .map(|slot| view::Strip {
            name: format!("slot{slot}"),
            tally: view::Tally::Live,
            requested: view::Tally::Live,
            gain: 0.5,
            gain_to: None,
            opacity: 0.5,
            opacity_to: None,
            blend: BlendMode::Over,
            mask: view::Mask::None,
            mask_angle: 0.0,
            level: None,
            is_muted: false,
            is_soloed: false,
        })
        .collect();
    assert_eq!(
        readout.view.selection(),
        0,
        "the selection is not deck A, so a drop naming deck B could be naming the selection"
    );

    let row = |readout: &mut Readout, index: usize| {
        readout.panel.solve();
        let at = library_bay(
            readout.panel.layout(),
            &readout.view.scopes,
            &readout.view.library,
            readout.view.opened(),
            readout.view.pointed(),
            readout.view.library_scroll(),
        )
        .expect("the bay lists its rows")
        .row(index);
        Point::new(at.center().x, at.center().y)
    };
    let cell = |readout: &mut Readout, deck: usize| {
        readout.panel.solve();
        let cells = preview_rects(readout.panel.layout(), readout.view.canvas)
            .expect("the preview row is on screen");
        assert_eq!(cells.len(), DECKS, "the row is not four cells");
        let at = cells[deck];
        Point::new(at.center().x, at.center().y)
    };

    // Row 1 onto cell B: not the selection, not the row's index as a deck.
    let at = row(&mut readout, 1);
    readout.pointer(&ctx, Pointer::Moved(at));
    assert_eq!(readout.pointer(&ctx, Pointer::Down).1, Acted::Pointed);
    let onto = cell(&mut readout, 1);
    readout.pointer(&ctx, Pointer::Moved(onto));
    assert_eq!(
        readout.pointer(&ctx, Pointer::Up).1,
        Acted::Emitted(Some(Operation::LoadSet {
            deck: 1,
            set: "lattice_veil".to_owned(),
        })),
        "the drop did not name the cell it was let go over"
    );
    assert!(!readout.panel.dragging(), "the carry is still in hand");

    // And cell D, which this deck has no slot for, loads nothing.
    let at = row(&mut readout, 2);
    readout.pointer(&ctx, Pointer::Moved(at));
    assert_eq!(readout.pointer(&ctx, Pointer::Down).1, Acted::Pointed);
    let onto = cell(&mut readout, 3);
    readout.pointer(&ctx, Pointer::Moved(onto));
    assert_eq!(
        readout.pointer(&ctx, Pointer::Up).1,
        Acted::Nothing,
        "a drop on the fourth cell of a three-slot deck asked for a load"
    );
}

/// A carry that moves the library cursor re-reads the row it arrived at, which
/// is the rule the cursor states rather than the keyboard: *"the reading
/// follows the cursor: a move with one open is a read of the row it arrived
/// at"* (`karakuri-console/src/view.rs`, `View::reading_open`).
///
/// # The defect it was written for
///
/// `Readout::took` discarded `View::point_at`'s `moved`. So taking a row in
/// hand while a reading was open on a different row moved the cursor off that
/// row, `View::opened` answered `None` because the row under the cursor was no
/// longer the Set the reading was of, and the block disappeared — for the rest
/// of the run, because nothing on this route ever walks the cursor back. The
/// arrow keys never had it: they re-read on `moved && reading_open()`.
///
/// # Why it is here and can be nowhere else
///
/// It needs all three of a press handler, a store on a disk, and the glue
/// between them, and this file is the only place that has any two. `carry.rs`
/// in `karakuri-console` presses the bay and the view directly and that crate
/// reaches no disk at all (ADR-0156), so the half it can hold is
/// `the_row_a_hand_takes_is_the_row_the_cursor_marks` — that `point_at` answers
/// the move — and not that anything acts on the answer.
///
/// # What it asserts, and what each one fails against
///
/// 1. The press answers `Acted::Pointed`, which is the whole of what
/// `Readout::took` can do about it: the readout holds no store, so the press
/// says *the cursor moved* and the caller reads the file. A `took` that drops
/// the `bool` again answers `Acted::Nothing` here. 2. The block is gone until
/// it is re-read, which is the defect itself, asserted so that step 3 cannot
/// pass by the reading never having moved. 3. `read_reading` — the call the
/// window loop makes on that answer — puts the reading under the row the hand
/// took, naming that row's Set. 4. A press on the row the cursor is already on
/// answers `Acted::Nothing`, so a carry that moved nothing costs no file read.
///
/// What it cannot see is that the window loop makes the call, because `winit`
/// cannot be asked for an `ActiveEventLoop` outside its own loop and an event
/// handler is not something a test can drive — `Readout::pointer`'s own doc.
/// `App::window_event`'s carry arm calls [`reread_if_open`] with
/// `matches!(acted, Acted::Pointed)`, the same function
/// [`reread_if_open_re_reads_only_on_a_move_with_a_reading_open`] presses
/// directly, below — this test is the two halves either side of that call, and
/// neither reaches the call itself.
///
/// A CPU test: a store is a directory and a `Readout` takes no device.
#[test]
fn a_carry_that_moves_the_cursor_re_reads_the_row_it_arrived_at() {
    let ctx = drawn_once();
    let root = scratch_dir("carry-reading");
    let store = Store::open(&root).expect("a store to read");
    store.write_set("drift_night", &[]).expect("a Set to read");
    store.write_set("lattice_veil", &[]).expect("a Set to read");

    let mut readout = Readout::new(1440.0, 900.0);
    readout.panel.solve();
    readout.view.scopes = Scope::ALL.to_vec();
    readout.view.library = vec!["drift_night".to_owned(), "lattice_veil".to_owned()];
    assert!(readout.view.select_scope(Scope::MySets));

    // The rectangles, off the derivation that draws them — and the open
    // reading goes in with them, because the block is drawn among the rows
    // and the row below it is somewhere else while it is down.
    let row = |readout: &mut Readout, index: usize| {
        readout.panel.solve();
        let at = library_bay(
            readout.panel.layout(),
            &readout.view.scopes,
            &readout.view.library,
            readout.view.opened(),
            readout.view.pointed(),
            readout.view.library_scroll(),
        )
        .expect("the bay lists its rows")
        .row(index);
        Point::new(at.center().x, at.center().y)
    };

    // The reading is opened the way the window loop opens it, on the row
    // the cursor starts on.
    read_reading(&mut readout.view, &root);
    let open = readout.view.opened().expect("a reading on the first row");
    assert_eq!((open.at, open.reading.id.as_str()), (0, "drift_night"));

    // 1: the press on the other row takes it in hand and says the mark
    // moved.
    let at = row(&mut readout, 1);
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
    let (claim, did) = readout.pointer(&ctx, Pointer::Down);
    assert_eq!(claim, Claim::Panel, "a press on a library row went to egui");
    assert_eq!(
        did,
        Acted::Pointed,
        "the carry moved the library cursor and answered nothing, so the reading open on \
         the row it left has nowhere to be drawn and nothing to bring it back"
    );
    assert_eq!(readout.view.cursor_row(), 1);

    // 2: and until the answer is acted on, the block is drawn nowhere.
    assert!(
        readout.view.opened().is_none(),
        "the reading is still drawn on a row the cursor has left"
    );
    assert!(readout.view.reading_open(), "the reading was put away");

    // 3: what the window loop does with that answer.
    let said = read_reading(&mut readout.view, &root);
    assert!(
        said.contains("lattice_veil"),
        "the re-read named a row the hand is not on: `{said}`"
    );
    let open = readout
        .view
        .opened()
        .expect("the reading followed the cursor to the row the hand took");
    assert_eq!((open.at, open.reading.id.as_str()), (1, "lattice_veil"));
    readout.pointer(&ctx, Pointer::Up);

    // 4: and a press on the row the cursor is already on moves nothing,
    // so it owes no read at all.
    let at = row(&mut readout, 1);
    readout.pointer(&ctx, Pointer::Moved(at));
    assert_eq!(
        readout.pointer(&ctx, Pointer::Down).1,
        Acted::Nothing,
        "a press on the row the cursor was already on asked for a re-read of it"
    );
    readout.pointer(&ctx, Pointer::Up);

    std::fs::remove_dir_all(&root).expect("clean up");
}

/// The filter row narrows what the bay lists, through the summary.
///
/// The other half of the same press: `a_press_on_a_filter_field_…` says the
/// operation reaches `View::narrow`, and this says the listing that comes back
/// afterwards is a narrower one — which is the whole point, and was impossible
/// while this side asked `Store::list_sets` for names.
///
/// A procedure is a row of `all` and of `presets`, with its kind on it — and of
/// neither `my sets` nor `folder` (ADR-0338, decision 1).
///
/// A CPU test: two tiers on a disk, a `View`, and no window.
#[test]
fn the_two_tiers_list_procedures_beside_sets_and_two_scopes_do_not() {
    use karakuri_operation::LibraryKinds;
    use karakuri_store::hash::Hash;
    use karakuri_store::ndjson::Line;
    use karakuri_store::record::Layer as Written;

    let root = scratch_dir("procedure-listing");
    let store = Store::open(&root).expect("a store to list");
    store
        .write_set(
            "night01",
            &[Line::new(Record::Slot {
                at: karakuri_store::record::NodeAddress {
                    layer: Written::L1,
                    index: 0,
                },
                name: Some("drift_shell".to_owned()),
                proc_hash: Hash::of(b"drift_shell"),
            })],
        )
        .expect("a Set to list");
    std::fs::write(
        root.join(Store::PROCEDURES).join("orbit_wide.kir"),
        "proc orbit_wide {\n  kind L3\n}\n",
    )
    .expect("a kept procedure");
    let shipped = root.join("shipped");
    std::fs::create_dir_all(&shipped).expect("mkdir");
    std::fs::write(shipped.join("beat_glow.kset"), "{}\n").expect("a shipped Set");
    std::fs::write(shipped.join("tunnel_eye.kir"), "  kind L3\n").expect("a shipped procedure");
    let presets = karakuri_environment::places::presets(Some(&shipped))
        .expect("the root resolves")
        .expect("a root");

    let mut view = View::new(Room::Day);
    view.scopes = Scope::ALL.to_vec();

    // `all`: the Set and the kept procedure, and the procedure carries the
    // kind its `kind` line declares while the Set carries its slots'.
    let said = listing(&mut view, &root, Some(&presets), None, None);
    assert!(
        view.library.contains(&"orbit_wide".to_owned())
            && view.library.contains(&"night01".to_owned()),
        "`all` lists {:?} — {said}",
        view.library
    );
    let at = view
        .library
        .iter()
        .position(|row| row == "orbit_wide")
        .expect("the procedure is a row");
    assert_eq!(
        view.kinds[at],
        RowKind {
            badges: vec![karakuri_operation::Layer::L3],
            procedure: true
        },
        "the procedure row's badge is not its kind"
    );
    let at = view
        .library
        .iter()
        .position(|row| row == "night01")
        .expect("the Set is a row");
    assert_eq!(
        view.kinds[at],
        RowKind {
            badges: vec![karakuri_operation::Layer::L1],
            procedure: false
        },
        "the Set row's badges are not the layers its slots fill"
    );

    // `presets`: the shipped Set and the shipped procedure, in name order.
    assert!(view.select_scope(Scope::Presets));
    listing(&mut view, &root, Some(&presets), None, None);
    assert_eq!(
        view.library,
        vec!["beat_glow".to_owned(), "tunnel_eye".to_owned()]
    );
    assert!(view.kinds[1].procedure, "the shipped `.kir` is not a row");

    // `my sets` lists no procedure, because a star is refused on anything
    // `sets/` does not hold; `folder` lists none, because a folder row is a
    // take and nothing takes a bare `.kir` in.
    assert!(view.select_scope(Scope::MySets));
    listing(&mut view, &root, Some(&presets), None, None);
    assert!(
        !view.library.contains(&"orbit_wide".to_owned()),
        "`my sets` lists a procedure: {:?}",
        view.library
    );
    assert!(view.select_scope(Scope::Folder));
    listing(&mut view, &root, Some(&presets), Some(&shipped), None);
    assert!(
        !view.library.contains(&"tunnel_eye".to_owned()),
        "`folder` lists a procedure: {:?}",
        view.library
    );

    // **The kind chips narrow by OR, and none on is everything.**
    assert!(view.select_scope(Scope::AllSets));
    let cameras = LibraryKinds {
        l3: true,
        ..LibraryKinds::EVERYTHING
    };
    assert!(view.narrow(None, cameras));
    listing(&mut view, &root, Some(&presets), None, None);
    assert_eq!(
        view.library,
        vec!["orbit_wide".to_owned()],
        "`L3` on lists {:?}",
        view.library
    );
    let sets_only = LibraryKinds {
        sets: true,
        ..LibraryKinds::EVERYTHING
    };
    assert!(view.narrow(None, sets_only));
    listing(&mut view, &root, Some(&presets), None, None);
    assert_eq!(
        view.library,
        vec!["night01".to_owned()],
        "`SET` on lists {:?}",
        view.library
    );
    assert!(view.narrow(
        None,
        LibraryKinds {
            l3: true,
            sets: true,
            ..LibraryKinds::EVERYTHING
        }
    ));
    listing(&mut view, &root, Some(&presets), None, None);
    assert_eq!(
        view.library.len(),
        2,
        "an OR of two lists {:?}",
        view.library
    );
    assert!(view.narrow(None, LibraryKinds::EVERYTHING));
    listing(&mut view, &root, Some(&presets), None, None);
    assert_eq!(
        view.library.len(),
        2,
        "none on is not everything: {:?}",
        view.library
    );

    std::fs::remove_dir_all(&root).expect("clean up");
}

/// What it narrows is the store's own listing, which is `all` and is what *List
/// what the store holds* lists. `my sets` is that listing starred (ADR-0299),
/// so the same retain applies to it and the row is not a control over one chip.
///
/// It is the same retain the MCP tool applies, over the same
/// `setfile::summarise`, which is what keeps one operation from being answered
/// two ways by two surfaces.
///
/// A CPU test: a store, a `View`, and no window.
#[test]
fn the_filter_row_narrows_the_stores_listing_through_the_summary() {
    use karakuri_store::hash::Hash;
    use karakuri_store::ndjson::Line;
    use karakuri_store::record::Layer as Written;

    let root = scratch_dir("filter-listing");
    let store = Store::open(&root).expect("a store to list");
    let slot = |layer: Written, name: &str| {
        Line::new(Record::Slot {
            at: karakuri_store::record::NodeAddress { layer, index: 0 },
            name: Some(name.to_owned()),
            proc_hash: Hash::of(name.as_bytes()),
        })
    };
    store
        .write_set("night01", &[slot(Written::L1, "drift_shell")])
        .expect("a Set to list");
    store
        .write_set("veil02", &[slot(Written::L4, "soft_points")])
        .expect("a second Set to list");

    let mut view = View::new(Room::Day);
    view.scopes = Scope::ALL.to_vec();
    assert_eq!(view.scope(), Some(Scope::AllSets));

    // Unnarrowed: both Sets, and the candidates are what their nodes are
    // called — sorted, deduplicated, and read off the *unfiltered* listing.
    let said = listing(&mut view, &root, None, None, None);
    assert_eq!(view.library.len(), 2, "the bay lists {:?}", view.library);
    assert_eq!(
        view.holds,
        vec!["drift_shell".to_owned(), "soft_points".to_owned()],
        "the `holds` field can be stepped to {:?}",
        view.holds
    );
    assert!(!said.contains("holding"), "{said}");

    // Narrowed by what a node is called.
    assert!(view.narrow(
        Some("drift_shell"),
        karakuri_operation::LibraryKinds::EVERYTHING
    ));
    let said = listing(&mut view, &root, None, None, None);
    assert_eq!(view.library, vec!["night01".to_owned()], "{said}");
    assert!(
        said.contains("1 of 2") && said.contains("drift_shell"),
        "{said}"
    );

    // **A filter that matched nothing is a different nothing from an empty
    // store**, and the line says which: the store is not empty, and what to
    // do about it is press a field rather than save a Set.
    assert!(view.narrow(
        Some("drift_shell"),
        karakuri_operation::LibraryKinds {
            l3: true,
            ..karakuri_operation::LibraryKinds::EVERYTHING
        },
    ));
    let said = listing(&mut view, &root, None, None, None);
    assert!(view.library.is_empty(), "the bay lists {:?}", view.library);
    assert!(
        said.contains("none of the 2 Sets here")
            && !said.contains(why_nothing(Scope::AllSets, false, false)),
        "{said}"
    );

    // **And the same retain applies to `my sets`**, which is this listing
    // starred: star one Set, mark the subset, and the filter that named
    // the other one leaves it with nothing — the narrowing is over what
    // the store holds and not over which chip is marked.
    assert!(view.narrow(None, karakuri_operation::LibraryKinds::EVERYTHING));
    favourite(
        &root,
        Asked::Operator,
        &Operation::SetFavourite {
            id: "night01".to_owned(),
            favourite: true,
        },
    )
    .expect("`favourite` answered nothing for a `SetFavourite`");
    assert!(view.select_scope(Scope::MySets));
    let said = listing(&mut view, &root, None, None, None);
    assert_eq!(view.library, vec!["night01".to_owned()], "{said}");
    assert!(view.narrow(
        Some("soft_points"),
        karakuri_operation::LibraryKinds::EVERYTHING
    ));
    let said = listing(&mut view, &root, None, None, None);
    assert!(
        view.library.is_empty(),
        "`my sets` lists {:?} under a filter that names the Set that is not starred",
        view.library
    );
    assert!(said.contains("none of the 2 Sets here"), "{said}");

    std::fs::remove_dir_all(&root).expect("clean up");
}

/// A press on the Program bay's `solo` pill, through the window loop's own
/// routing.
///
/// `karakuri-console`'s `tests/solo_pill.rs` and `tests/vocabulary.rs` assert
/// everything up to the operation with no window anywhere; this is the half
/// ADR-0213 makes the badge mean — *"the row is claimed the day a person who
/// launched the instrument can perform that operation from the panel in front
/// of them"* — and a control demonstrated in that crate and never wired here
/// would pass there and be a lie the page tells.
///
/// Both directions, because the pill is both. A solo takes every other control
/// off the screen, so the pill is the only thing left to press and the undo has
/// to come from it. What is asserted is the round trip an operator makes: the
/// picture is one region among many, a click on the pill leaves it holding the
/// window, and a click on the same pill — found again where it is now drawn,
/// because the solo moved every rectangle on the console — puts everything
/// back.
#[test]
fn a_press_on_the_solo_pill_solos_the_picture_and_undoes_it() {
    let ctx = drawn_once();
    let mut readout = Readout::new(1440.0, 900.0);
    readout.panel.solve();
    let picture = readout
        .panel
        .layout()
        .find("program-view")
        .expect("program-view");
    let library = readout.panel.layout().find("library").expect("library");
    // The capsule, asked of the derivation that draws it rather than
    // remembered — which is the rule the whole of `input` is written to,
    // and here it is load-bearing twice over.
    let pill = |readout: &mut Readout| {
        readout.panel.solve();
        let head = program_head(&ctx, readout.panel.layout(), Open::CLOSED)
            .expect("the bay draws its pill");
        (
            Point::new(head.solo.center().x, head.solo.center().y),
            head.soloed,
        )
    };

    let (at, soloed) = pill(&mut readout);
    assert!(!soloed, "something is soloed before anything was pressed");
    assert!(
        readout.panel.layout().visible(library),
        "the library is off the screen already, so soloing would prove nothing"
    );

    assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
    let (claim, did) = readout.pointer(&ctx, Pointer::Down);
    assert_eq!(claim, Claim::Panel);
    assert_eq!(
        did,
        Acted::Operated(Outcome::Soloed(picture)),
        "the press did not reach the pill"
    );
    assert!(
        !readout.panel.dragging(),
        "the press took a boundary in hand"
    );
    readout.pointer(&ctx, Pointer::Up);
    readout.panel.solve();
    assert!(
        !readout.panel.layout().visible(library),
        "the picture is soloed and the library is still on the screen"
    );

    // And the same pill, where it is now, undoes it.
    let (at, soloed) = pill(&mut readout);
    assert!(soloed, "the picture is soloed and the pill does not say so");
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
    assert_eq!(
        readout.pointer(&ctx, Pointer::Down).1,
        Acted::Operated(Outcome::Unsoloed { was: true }),
        "the pill did not undo the solo it made"
    );
    readout.pointer(&ctx, Pointer::Up);
    readout.panel.solve();
    assert!(
        readout.panel.layout().visible(library),
        "undoing the solo left the library folded"
    );
}
/// A control's operation becomes the record every other surface's control ends
/// in, and this is the half of that which needs no device.
///
/// This test is older than the conversion it now checks, and that is the point
/// of it. It was written against the hand-written `record` this file used to
/// carry, asserting term for term what `karakuri-cli`'s `mix::gain_record` and
/// `mix::opacity_record` already wrote. That function is deleted and
/// [`written`] answers instead (ADR-0185's promise, kept where ADR-0194 put the
/// home) — every expectation below is unchanged, so if the crate's conversion
/// disagreed with the one that was deleted, this is what says so.
///
/// `mix::gain_record` is deleted too, by the same record and for the stronger
/// reason: the conversion *is* the derivation now, and two of them is the drift
/// `mix.rs` exists to end. The comments below name it where it stood, because
/// what this test compares against is the record that function wrote rather
/// than the function.
///
/// And the other direction: an operation this program has no control for writes
/// no record here either, and the answer says *which* kind of nothing rather
/// than a bare `None` — which is the whole of what the three answers buy.
#[test]
fn a_controls_operation_becomes_the_record_the_cli_would_have_written() {
    assert_eq!(
        only_record(&Operation::SetGain {
            deck: 2,
            gain: 0.75
        }),
        // What `mix::gain_record(2, 0.75)` wrote, before ADR-0194 deleted
        // it in favour of this conversion.
        Record::Gain {
            slot: DeckSlot(2),
            value: 0.75
        }
    );
    assert_eq!(
        only_record(&Operation::SetOpacity {
            deck: 0,
            opacity: 0.25
        }),
        // `mix::opacity_record(0, 0.25)`.
        Record::Opacity {
            slot: DeckSlot(0),
            value: 0.25
        }
    );
    // **Every mode of the cycle, because a chip that emits three
    // operations has three records to write** — and the mode is a wire
    // name, so a mode that reached `Record::Blend` misspelled would be
    // refused by the engine on the way back rather than here.
    for (deck, blend) in BlendMode::ALL.into_iter().enumerate() {
        let deck = deck as u8;
        assert_eq!(
            only_record(&Operation::SetBlendMode { deck, blend }),
            // `mix::blend_record(deck, blend)`.
            Record::Blend {
                slot: DeckSlot(deck),
                mode: blend.name().to_owned(),
            },
            "`{}` did not become the record `mix::blend_record` writes",
            blend.name()
        );
        // And the engine reads its own name back, which is what says the
        // two lists are the same three words rather than two spellings of
        // them.
        assert_eq!(
            Blend::from_name(blend.name()),
            Some(blend_mode_back(blend)),
            "the engine does not know the vocabulary's `{}`",
            blend.name()
        );
    }

    // **Every residency of the cycle**, for the same reason as the blend:
    // one chip emitting three operations has three records to write. The
    // spelling is the wire's — `mix::residency_wire_name`'s three words,
    // which are deliberately not the status line's `LIVE`/`prim`/`park`
    // and not the chip's `live`/`prim`/`alloc` either, so a record written
    // in the chip's vocabulary would decode as nothing at all.
    for (deck, (residency, level)) in [
        (karakuri_operation::Residency::Live, "live"),
        (karakuri_operation::Residency::Priming, "priming"),
        (karakuri_operation::Residency::Allocated, "allocated"),
    ]
    .into_iter()
    .enumerate()
    {
        let deck = deck as u8;
        assert_eq!(
            only_record(&Operation::SetResidency { deck, residency }),
            // `mix::residency_record(deck, residency)`.
            Record::Residency {
                slot: DeckSlot(deck),
                level: level.to_owned(),
            },
            "{residency:?} did not become the record `mix::residency_record` writes"
        );
        // And it reads back as the level it named, which is what says the
        // two spellings are one list rather than two.
        assert_eq!(
            mix::parse_residency(level),
            Some(residency_back(residency)),
            "the wire spelling `{level}` does not come back as {residency:?}"
        );
    }

    // The vocabulary is larger than what this program reaches: five controls
    // writing five records. A record invented for the other 45 would be
    // somebody deciding what they mean — and the answer is now *which*
    // nothing rather than `None`, because a surface's own state and a
    // record nobody can write yet are not the same silence.
    assert_eq!(
        written(&Operation::Solo { region: None }, &Current::default()),
        Written::Silent(Silent::Surface)
    );
    assert_eq!(
        written(&Operation::SelectDeck { deck: 1 }, &Current::default()),
        Written::Silent(Silent::Surface)
    );
}

/// The one record an operation writes, for the tests that know there is exactly
/// one.
///
/// For the four whose record needs no reading at all, which is where
/// `Current::default()` — *I read nothing* — is the honest answer. A conversion
/// that answered anything but a single record for one of those four is this
/// file's assumption breaking rather than a test needing a helper, which is why
/// the panic says so.
///
/// The mask's operation is not one of them and must not be passed here: its
/// record is written out of the operation *and* a reading of the running mask
/// (ADR-0201), so it would come back `Owed(NotRead)` and this would panic —
/// correctly, and saying which operation. What the mask's tests hand in is a
/// reading, through [`reading`] where there is a deck and by hand where there
/// is not.
pub(super) fn only_record(operation: &Operation) -> Record {
    match written(operation, &Current::default()) {
        Written::Records(records) if records.len() == 1 => records.into_iter().next().unwrap(),
        other => panic!(
            "a control's operation did not write exactly one record: \
             {operation:?} -> {other:?}"
        ),
    }
}

/// A refused wipe says which refusal it was and where the next attempt is made,
/// rather than *refused*.
///
/// `karakuri_console::view::Go` answers which of the two it is because the
/// console is what can see it; the sentence is this window's, and
/// [P-0083](../../../docs/principles/0083-a-refusal-carries-what-the-next-attempt-needs.md)
/// is what it owes: the constraint and the numbers, never a bare no. A press on
/// `go` that printed nothing would read exactly like a press on the card beside
/// it, which is the failure the whole `Go` type exists to prevent.
///
/// The two are asserted to be different sentences, for
/// [`an_operation_whose_record_is_owed_is_said_rather_than_swallowed`]'s reason
/// one test up: a window that printed one line for both would tell an operator
/// with four decks and no shape that they need a second deck.
///
/// Not word for word. What has to hold is that each names what would have to
/// change — the shape pill for one, a second deck for the other — and that the
/// count is in the one whose count is the constraint.
#[test]
fn a_refused_wipe_says_which_refusal_it_was_and_where_to_go_next() {
    let no_shape = refusal(&Go::NoShape, 4);
    assert!(
        no_shape.contains("shape"),
        "the refusal for an unchosen shape does not say what is missing: `{no_shape}`"
    );
    assert!(
        no_shape.contains("pill"),
        "the refusal for an unchosen shape does not say where one is picked, so an \
         operator is told no and not told where to go: `{no_shape}`"
    );

    let alone = refusal(&Go::NoOtherDeck, 1);
    assert!(
        alone.contains('1') && alone.contains("strip"),
        "the refusal for a mixer with nowhere to wipe from does not carry the count \
         that is the constraint: `{alone}`"
    );
    assert!(
        alone.contains("deck"),
        "the refusal for a mixer with nowhere to wipe from does not say what would \
         have to change: `{alone}`"
    );
    // The plural moves with the count, which is this file's rule for every
    // sentence that carries one.
    assert!(refusal(&Go::NoOtherDeck, 0).contains("0 strips"));
    assert!(refusal(&Go::NoOtherDeck, 1).contains("1 strip,"));

    assert_ne!(
        no_shape, alone,
        "a wipe with no shape chosen and a wipe with nowhere to come from came out of \
         this window as the same sentence"
    );
}

/// An operation whose record nobody can write yet does not silently do nothing,
/// and it is not the same event as one that writes no record on purpose.
///
/// This is what the third answer is *for*, and the cheap harness is the one
/// that treats *not `Records`* as a no-op. A press that emitted `TapBeat` would
/// then look exactly like a press that emitted `SelectDeck` — nothing printed
/// and nothing moved — and an operator would read the first as *the tap did not
/// take* when what happened is *nobody has decided what a tap writes* (`Owed`
/// is a question, not an error: ADR-0194).
///
/// The operation this names has had to change twice, which is the test doing
/// what it says on the line below. It was `FadeDeck`, which stopped being owed
/// the day the transition settings became a reading; it was then `Wipe`, which
/// stopped the day the front shape went over with them and the soft edge turned
/// out to be the arriving deck's. It is now `Operation::TapBeat` — and that one
/// is a different shape rather than the next in a queue: what a tap owes is the
/// beat lock's answer and not a value any surface holds, so no reading added to
/// `Current` closes it.
///
/// The second half has been re-pointed once, and the reason is worth reading.
/// It was `FadeDeck`, on the grounds that this panel held no transition
/// settings to hand over — and the day the transition row was wired into this
/// window that stopped being true, without this test going red: it builds a
/// `Current::default()` by hand, so it went on passing while its own sentence
/// had become false. That is the failure mode `docs/contributing.md` §3 is
/// about, met from the wrong side.
///
/// It is `Operation::SetMaskPosition` against a reading nobody took, and the
/// second half stopped being about this window on 2026-09-10. Its record is
/// `Record::Mask` written whole and it needs the shape, the angle and the
/// softness it does not name (ADR-0201). Until that day [`reading`]'s mask arm
/// answered for `SetMaskShape` and for a wipe's arriving deck and for nothing
/// else, so this *was* a gap in this file — which is what ADR-0334 recorded and
/// ADR-0341 closed with one arm.
///
/// What it asserts now is the third answer itself, which is why the operation
/// did not have to change a third time: handed a `Current` with no mask in it —
/// a reading that was not taken, whatever the reason — the conversion says
/// *which* reading is missing rather than sending a front back to wherever a
/// default put it, mid-wipe. That the real reading is now taken is asserted
/// where there *is* a deck,
/// `gpu::the_go_pill_runs_a_wipe_against_the_settings_the_row_is_on`, which is
/// the half a test with no device cannot make.
///
/// Neither sentence is asserted word for word. What has to hold is that the
/// window says something, that it names the operation and the reason, and that
/// the two answers are two different sentences.
#[test]
fn an_operation_whose_record_is_owed_is_said_rather_than_swallowed() {
    // Owed, and `NotSettled` is the reason: a tap's record is the beat
    // lock's answer — a tapped tempo, a phase error, an output lag — and
    // none of it is a value a `Current` carries, so nobody has said what
    // it writes here.
    let tap = Operation::TapBeat;
    let owed = written(&tap, &Current::default());
    assert_eq!(
        owed,
        Written::Owed(Owed::NotSettled),
        "a tap is not owed any more — this test names the operation it does, and \
         the one it names has to still be one nobody can write"
    );
    let said = unwritten(&tap, &owed).expect(
        "a tap owes a record and this window said nothing at all — a press whose \
         record nobody has decided how to write reads, in silence, exactly like a \
         press that did not work",
    );
    assert!(
        said.contains("TapBeat") && said.contains(Owed::NotSettled.why()),
        "the window said `{said}`, which does not name both the operation and the \
         question it is waiting on"
    );

    // **And the other answer, which is a reading nobody took rather than a
    // record nobody has decided.** A mask position converts, and what it
    // needs is the rest of the mask — the shape, the angle and the soft
    // edge `Record::Mask` is written whole out of. Handed a reading with
    // no mask in it, the conversion says *which reading* was not handed
    // over rather than sending a front back to wherever a default put it,
    // and the sentence has to be a different one from the tap's above or
    // the two answers read alike. **This window took no mask for a
    // position until 2026-09-10** and that was the gap this half named;
    // it takes one now (ADR-0341), so what is left here is the third
    // answer itself, asserted against a `Current` built by hand.
    let front = Operation::SetMaskPosition {
        deck: 1,
        position: 0.5,
    };
    let unread = written(&front, &Current::default());
    assert_eq!(
        unread,
        Written::Owed(karakuri_operation_record::Owed::NotRead(
            karakuri_operation_record::Reading::Mask
        )),
        "a mask position with no mask handed in came back with something \
         other than the reading it is missing — a default here is a shape and an \
         angle nobody chose written over the ones a deck is wearing"
    );
    let told = unwritten(&front, &unread).expect(
        "a mask position this window cannot write said nothing at all, so a control \
         that emitted one would read exactly like a control that did not work",
    );
    assert!(
        told.contains("SetMaskPosition")
            && told.contains(Owed::NotRead(karakuri_operation_record::Reading::Mask).why()),
        "the window said `{told}`, which does not name both the operation and the \
         reading it did not get"
    );
    // **And the one it replaced is not owed any more**, which is the half
    // that would have caught this test going quietly stale: a fade is
    // scheduled against settings this console holds now, so `FadeDeck` is
    // no longer a case of *a reading this window does not have*. If this
    // ever fails, the second half above has a candidate again and somebody
    // has to say which of the two this test is about.
    assert_ne!(
        written(
            &Operation::FadeDeck { deck: 1, to: 0.0 },
            &Current {
                transition: Some(karakuri_operation_record::Transition {
                    start: 0.0,
                    beats: 4.0,
                    curve: karakuri_operation::Curve::Smooth,
                    wipe_kind: karakuri_operation::WipeKind::None,
                    wipe_angle: 0.0,
                }),
                ..Current::default()
            }
        ),
        Written::Owed(Owed::NotRead(
            karakuri_operation_record::Reading::Transition
        )),
        "a fade handed the transition settings this window now holds is still owed \
         them, so the reading this panel supplies is not the one the conversion wants"
    );
    assert_ne!(
        told, said,
        "a reading this window forgot and a record nobody has decided how to write \
         read as the same sentence"
    );

    // Silent, and settled: which deck the keys are addressed to is a
    // surface's own state and there is nothing to write.
    let select = Operation::SelectDeck { deck: 1 };
    let silent = written(&select, &Current::default());
    assert_eq!(silent, Written::Silent(Silent::Surface));
    let settled = unwritten(&select, &silent).expect(
        "selecting a deck writes no record and the window said nothing about it \
         either, so a press on such a control would leave no trace at all",
    );
    assert!(
        settled.contains(Silent::Surface.why()),
        "the window said `{settled}`, which does not say why there is no record"
    );

    // **And the two are different sentences.** Collapsing them is the
    // failure this whole test is about at one remove: a harness that
    // printed one line for both would tell an operator that an undecided
    // fade is as settled as a deck selection.
    assert_ne!(
        said, settled,
        "a record nobody can write yet and a record nobody needs to write came out \
         of this window as the same sentence"
    );

    // A record's line is `apply`'s — it says the record *and* what the
    // deck holds afterwards — so this says nothing about that case.
    // Otherwise one press prints twice.
    let gain = Operation::SetGain { deck: 0, gain: 0.5 };
    assert_eq!(
        unwritten(&gain, &written(&gain, &Current::default())),
        None,
        "an operation that wrote a record was also announced as writing none"
    );
}

/// A scheduled move on a fader a lane of the armed pattern holds is refused,
/// says so in the one sentence, and that sentence is not the gap's
/// (ADR-0323).
///
/// Three halves, and the third is the one that could go quietly stale. The
/// first is the answer: a fade over a held fader comes back `Refused` and
/// carries no record, which is the clause a replay depends on — a record
/// written live would be replayed by a run with no sequencer in it (ADR-0322,
/// P-0092). The second is this window's line for it, which has to be a
/// different sentence from an `Owed`: a gap nobody has closed and a decision
/// taken read alike otherwise.
///
/// The third is the reading. [`reading`] cannot be called here — it takes a
/// `Deck` and this binary has no device — so the source is scanned for the
/// banks arriving and for the field being filled from them. Without that, this
/// whole test passes against a `Current` built by hand while the window
/// schedules moves over lanes in silence, which is the failure
/// `docs/contributing.md` §3 is about.
#[test]
fn a_move_on_a_fader_a_lane_holds_is_refused_and_said_as_a_decision() {
    let fade = Operation::FadeDeck { deck: 1, to: 0.0 };
    let current = Current {
        transition: Some(karakuri_operation_record::Transition {
            start: 8.0,
            beats: 4.0,
            curve: karakuri_operation::Curve::Smooth,
            wipe_kind: karakuri_operation::WipeKind::None,
            wipe_angle: 0.0,
        }),
        lanes: Some(karakuri_operation_record::Lanes {
            held: vec![(3, karakuri_operation::LaneTarget::Fader { deck: 1 })],
        }),
        ..Current::default()
    };
    let refused = written(&fade, &current);
    assert_eq!(
        refused,
        Written::Refused(karakuri_operation_record::Refusal { lane: 3, deck: 1 }),
        "a fade onto a deck whose fader a lane holds was converted into records — the \
         lane cancels the fade within one step and a replay, which runs no sequencer, \
         would run it"
    );
    let told = unwritten(&fade, &refused).expect(
        "a fade this window refused said nothing at all, so a key that emitted one \
         reads exactly like a key that is not bound",
    );
    assert!(
        told.contains(&karakuri_operation_record::Refusal { lane: 3, deck: 1 }.why()),
        "the window said `{told}`, which is not the sentence the refusal is worded in \
         — the next attempt is to mute the lane it names"
    );
    let gap = unwritten(
        &Operation::TapBeat,
        &written(&Operation::TapBeat, &Current::default()),
    )
    .expect("a tap owes a record and this window says so");
    assert_ne!(
        told, gap,
        "a decision taken and a gap nobody has closed came out of this window as the \
         same sentence"
    );

    // **And the reading this window hands over.** `reading` takes the banks
    // and fills the field from the armed pattern; either half missing is a
    // refusal that silently never happens.
    // Read with the whitespace taken out, so that a reformat of the file is
    // not a failing test and a line wrapped by `cargo fmt` is not a silence.
    const APPLY: &str = include_str!("../bridge/handlers/apply.rs");
    let apply: String = APPLY.split_whitespace().collect();
    for wanted in [
        "banks:&karakuri_pattern::Banks,",
        "banks.pattern().held()",
        // The field of the `Current` this window builds, and not a mention of
        // the word in a comment above it.
        "mix,lanes,}",
    ] {
        assert!(
            apply.contains(wanted),
            "`reading` no longer carries `{wanted}` — the lanes reading is how a \
             scheduled move meets the lane holding its fader, and a field left out \
             refuses nothing and says nothing"
        );
    }
}

/// What a model asking over `--mcp` is told about an operation this window
/// refused, owed or performed — and *performed* is one of the three answers
/// rather than all of them (ADR-0131, P-0083, ADR-0315).
///
/// The defect this pins was one sentence and no branch: the drain answered
/// ``was performed on the frame it arrived on`` for every operation it took,
/// so a model that asked for a fade on a fader an unmuted lane of the armed
/// pattern holds was told the move was running. Nothing was scheduled, nothing
/// moved, the lane still held the fader, and the one place that said so was
/// this run's terminal — which a model does not have (ADR-0315). It then asked
/// for the next thing.
///
/// Four answers, and the first two are the repair:
///
/// - A refusal is the refusal's own sentence, in the wording
///   `karakuri-cli` answers a refusal in. The `assert_eq!` is against
///   [`not_performed`] rather than a spelling written out here, which is
///   `no_such_slot`'s lesson (ADR-0131): four spellings of one refusal lived
///   side by side because every test asked only whether the range appeared in
///   it. Pinning both programs to the one function is what makes them one
///   sentence — there is no second string to drift from.
/// - A gap is the gap's sentence and not the refusal's, which is
///   [`unwritten`]'s distinction carried onto the socket.
/// - A record is *performed*, unchanged.
/// - **And so is a `Silent`**, which is where this program's answer differs
///   from `karakuri-cli`'s and is deliberate: that program performs an
///   operation by writing records, so a `Silent` is one it has no control for;
///   this one has the control. `SelectDeck` moves the ring, and answering
///   *nothing on this run changed* for it would be this fix writing the defect
///   it repairs the other way round.
///
/// The fifth half is the wiring, because none of the four enters the drain.
/// `App::operated` takes a `Gfx` and an `ActiveEventLoop` and this binary has
/// neither, so the source is scanned for the conversion being carried out of
/// `App::performed` and for the drain consulting it — without that, every
/// assertion above passes against a function nothing calls, which is
/// `docs/contributing.md` §3's whole subject.
#[test]
fn a_model_is_answered_the_refusal_rather_than_told_its_move_was_performed() {
    let fade = Operation::FadeDeck { deck: 1, to: 0.0 };
    let held = Current {
        transition: Some(karakuri_operation_record::Transition {
            start: 8.0,
            beats: 4.0,
            curve: karakuri_operation::Curve::Smooth,
            wipe_kind: karakuri_operation::WipeKind::None,
            wipe_angle: 0.0,
        }),
        lanes: Some(karakuri_operation_record::Lanes {
            held: vec![(3, karakuri_operation::LaneTarget::Fader { deck: 1 })],
        }),
        ..Current::default()
    };
    let refusal = karakuri_operation_record::Refusal { lane: 3, deck: 1 };
    let refused = written(&fade, &held);
    assert_eq!(
        refused,
        Written::Refused(refusal),
        "a fade onto a deck whose fader a lane holds was converted into records, so the \
         answer this test is about is not the one being asserted"
    );
    let said = unperformed(fade.title(), &refused).expect(
        "a fade this window refused answered a model nothing at all, so the drain falls \
         through to `was performed` for a move that was never scheduled",
    );
    assert_eq!(
        said,
        not_performed(fade.title(), &refusal.why()),
        "the window answered `{said}`, which is not the sentence `karakuri-cli` answers \
         the same refusal in — one mistake, one explanation, whichever program a model \
         came through"
    );
    assert!(
        !said.contains("was performed"),
        "a refused move was reported to a model as performed: `{said}`"
    );

    // **A gap, and it is not the refusal's sentence.** A scrub with no
    // transport read is the reading this window fails to take when the deck it
    // names is not one it holds.
    let scrub = Operation::ScrubDeck {
        deck: 0,
        beats: 0.25,
    };
    let gap = written(&scrub, &Current::default());
    assert_eq!(
        gap,
        Written::Owed(Owed::NotRead(karakuri_operation_record::Reading::Transport)),
        "a scrub with no transport read no longer owes a record, so the gap this test is \
         about is not the one being asserted"
    );
    let owed = unperformed(scrub.title(), &gap)
        .expect("a scrub this window could not convert answered a model nothing at all");
    assert_eq!(
        owed,
        not_performed(
            scrub.title(),
            Owed::NotRead(karakuri_operation_record::Reading::Transport).why()
        ),
        "the window answered `{owed}`, which is not the words the crate that owes the \
         record says the gap in"
    );
    assert_ne!(
        owed, said,
        "a decision taken and a gap nobody has closed went back over the socket as the \
         same sentence"
    );

    // **A record is performed, and so is a `Silent`.** `None` here is the
    // drain falling through to the sentence that says so.
    let gain = Operation::SetGain { deck: 0, gain: 0.5 };
    assert_eq!(
        unperformed(gain.title(), &written(&gain, &Current::default())),
        None,
        "an operation that wrote a record was answered as though nothing on this run \
         changed"
    );
    let select = Operation::SelectDeck { deck: 0 };
    let silent = written(&select, &Current::default());
    assert_eq!(
        silent,
        Written::Silent(Silent::Surface),
        "a deck selection no longer writes no record, so the arm this test is about is \
         not the one being asserted"
    );
    assert_eq!(
        unperformed(select.title(), &silent),
        None,
        "an operation this window performs on its own surface — the ring moves — was \
         answered `nothing on this run changed`, which is the defect this test is about \
         written the other way round"
    );

    // **And the drain, read out of its own source**, because nothing above
    // enters it: the conversion has to be carried out of `App::performed` and
    // consulted before the `Ok` is built, and either half missing is four
    // green assertions over a function with no caller.
    // Read with the whitespace taken out, so that a reformat of the file is
    // not a failing test and a line wrapped by `cargo fmt` is not a silence.
    const APP: &str = include_str!("../app/mod.rs");
    let app: String = APP.split_whitespace().collect();
    for wanted in [
        // Carried out of the performer, at the one place the readings the
        // conversion needs are true (ADR-0323).
        "converted=Some(written);",
        // And consulted by the drain, ahead of the sentence that says it was
        // performed.
        "unperformed(title,written)",
        "reply.settled(Err(refused));",
    ] {
        assert!(
            app.contains(wanted),
            "`App::operated` no longer carries `{wanted}` — the outcome not reaching the \
             reply is a model told its refused move was performed, with every assertion \
             in this test still green"
        );
    }
}

/// The deck head's two operations, as far as this program can take them without
/// a device — and they go the same distance now, which is the point.
///
/// They used to go different distances: a scrub became a record and a sync mode
/// did not, and the second half of that is what `tests/panel_column.rs`'s one
/// exemption rested on — the chip's badge stayed `plan` because an operator who
/// pressed it reached the emission and not the move. That test said the day it
/// stopped being true it would stop being true here, and this is here.
///
/// The two are still not the same conversion, and that is what the second half
/// asserts. A scrub is relative and reads the transport it moves from; a mode
/// is absolute and reads the session tempo, replacing the anchor and clearing
/// the scrub. A sync mode that came out carrying the position the slot was
/// scrubbed to would be the two conversions having been made one.
#[test]
fn the_deck_heads_two_operations_go_different_distances() {
    // **The scrub is relative, so the record is where the slot is plus
    // what was asked for.** The reading is handed in by hand here for
    // `reading`'s reason at the mask: there is no deck in this test
    // binary, and what is being checked is the arithmetic rather than the
    // read.
    let current = Current {
        transport: Some(karakuri_operation_record::Transport {
            sync: karakuri_operation::Sync::Beat,
            anchor_bpm: 128.0,
            scrub_beats: -1.5,
        }),
        ..Current::default()
    };
    // **The amount is the console's own constant**, not a figure written
    // again here: the arrow that emits it and the record that carries it
    // are one number or the panel and the deck disagree about how far a
    // press goes.
    let scrub = Operation::ScrubDeck {
        deck: 1,
        beats: SCRUB_BEATS,
    };
    assert_eq!(
        written(&scrub, &current),
        Written::Records(vec![Record::Transport {
            slot: DeckSlot(1),
            sync: "beat".to_owned(),
            anchor_bpm: 128.0,
            scrub_beats: -1.25,
        }]),
        "a press of the deck head's forward arrow, from -1.50, did not come out at -1.25 — \
         so the record is not the offset the deck holds plus the amount the arrow asks for"
    );
    // **And the reading is what makes it one**: without it the conversion
    // says so rather than starting the deck's scrub from zero, which is
    // why `reading` has an arm for this operation at all.
    assert_eq!(
        written(&scrub, &Current::default()),
        Written::Owed(Owed::NotRead(karakuri_operation_record::Reading::Transport)),
        "a scrub with no transport read came back with a record, which means it invented \
         the position it moved from"
    );
    // **The mode goes out as a wire name and the engine reads its own name
    // back**, which is what `apply` does with it and is the blend chip's
    // assertion one control along.
    for sync in SYNCS {
        assert_eq!(
            EngineSync::from_name(sync.name()).map(mix::sync),
            Some(sync),
            "the engine does not know the vocabulary's `{}`",
            sync.name()
        );
    }

    // **A sync mode anchors at the session tempo and starts on the
    // grid.** The reading handed in is the same one the scrub used —
    // anchored at 128 and scrubbed to -1.5 — and none of it may survive:
    // `Transport::engaged` clears the scrub because *"a slot brought back
    // to the grid should be on the grid, not on wherever it was scrubbed
    // to a song ago"*, and the anchor is the room's tempo rather than the
    // one the slot was last locked to.
    let set = Operation::SetSync {
        deck: 1,
        sync: karakuri_operation::Sync::Beat,
    };
    let engaged = Current {
        tempo: Some(126.0),
        ..current
    };
    assert_eq!(
        written(&set, &engaged),
        Written::Records(vec![Record::Transport {
            slot: DeckSlot(1),
            sync: "beat".to_owned(),
            anchor_bpm: 126.0,
            scrub_beats: 0.0,
        }]),
        "a press of the deck head's sync chip, in a room at 126 bpm, did not come out \
         anchored at 126 with the scrub cleared — either the slot's old anchor survived \
         being re-engaged, or the position it was scrubbed to did"
    );
    // **And the reading is what makes it one.** Without the tempo the
    // conversion says so rather than anchoring at a guess, which is the
    // scrub's own arrangement two assertions up and the reason `reading`
    // has an arm for this operation at all.
    assert_eq!(
        written(&set, &Current::default()),
        Written::Owed(Owed::NotRead(karakuri_operation_record::Reading::Tempo)),
        "a sync mode with no session tempo read came back with a record, which means the \
         tempo it anchored the deck at was invented"
    );
    let said = unwritten(&set, &written(&set, &Current::default())).expect(
        "a sync chip press with no tempo read said nothing at all — a press that reads, in \
         silence, exactly like a press that did not work",
    );
    assert!(
        said.contains("SetSync")
            && said.contains(Owed::NotRead(karakuri_operation_record::Reading::Tempo).why()),
        "the window said `{said}`, which does not name both the operation and the reading \
         it did not get"
    );
}

/// The two crates walk the sync modes in one order, which is what makes
/// `view::Pane::allows` line up with the field it fills.
///
/// [`inspector`] builds that array by mapping `EngineSync::ALL` and the console
/// reads it by indexing [`SYNCS`], so the two orders are one order or the panel
/// skips the wrong mode — silently, and only on material that refuses
/// something. Two arrays cannot be made one by a comment.
#[test]
fn the_two_crates_walk_the_sync_modes_in_one_order() {
    assert_eq!(EngineSync::ALL.len(), SYNCS.len());
    for (index, mode) in EngineSync::ALL.into_iter().enumerate() {
        assert_eq!(
            mix::sync(mode),
            SYNCS[index],
            "`EngineSync::ALL[{index}]` is `{}` and the console's `SYNCS[{index}]` is \
             `{}` — the deck head's cycle would skip the wrong mode",
            mode.name(),
            SYNCS[index].name()
        );
    }
}

/// [`tally`] the other way round, for the assertion above alone — the
/// vocabulary's residency as the engine's, so that the round trip through the
/// wire name can be compared against something.
fn residency_back(residency: karakuri_operation::Residency) -> Residency {
    match residency {
        karakuri_operation::Residency::Live => Residency::Live,
        karakuri_operation::Residency::Priming => Residency::Priming,
        karakuri_operation::Residency::Allocated => Residency::Allocated,
    }
}

/// [`blend_mode`] the other way round, for the assertion above alone — which is
/// why it is here and not beside it: nothing the program *runs* needs to go
/// this direction, and a conversion in `src` with one test as its only caller
/// would be an abstraction with no second call site.
fn blend_mode_back(blend: BlendMode) -> Blend {
    match blend {
        BlendMode::Add => Blend::Add,
        BlendMode::Over => Blend::Over,
        BlendMode::Max => Blend::Max,
    }
}

/// Anything that makes texels this frame keeps the loop awake, and the list is
/// closed.
///
/// [`live`] decides whether the loop asks for another frame, and it is the one
/// decision in this file that has already been got wrong twice in the same
/// direction. The first time it was set once and never cleared, so folding the
/// picture away left the window drawing at full rate — found by an operator on
/// another machine following this file's own instructions, which said the
/// window goes quiet, and getting 270 frames. The second time it was the
/// picture alone, which is the same failure with a preview under it: fold the
/// picture and deck A goes on auditioning while the loop stops asking for
/// frames, so the panel keeps changing and nothing draws it.
///
/// So the assertion is over every sink, not over the one this program fills: a
/// cell nobody has wired up yet is asserted live all the same, because the
/// failure is a sink left out of the list rather than a sink that is off.
///
/// It needs no device: an `egui::TextureId` is a number, and what is being
/// asserted is a rule about `Option`s.
#[test]
fn anything_that_makes_texels_keeps_the_loop_awake() {
    let some = Picture {
        id: egui::TextureId::User(0),
        rect: egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(16.0, 9.0)),
    };
    let mut view = View::new(Room::Day);

    // Nothing is making texels, so the loop has no reason of its own to
    // draw and `ControlFlow::Wait` gets to block.
    assert!(!live(&view), "an empty panel was called live");

    // The picture, which is what this rule used to be the whole of.
    view.picture = Some(some);
    assert!(live(&view), "a live picture did not keep the loop awake");

    // **The case the picture-alone rule gets wrong**: the picture folded
    // away with deck A still auditioning under it.
    view.picture = None;
    view.previews[0] = Some(some);
    assert!(
        live(&view),
        "the picture is folded away and deck A is still rendering, and the loop was \
         told to sleep — which is the window that kept drawing 270 frames after it \
         was said to have gone quiet"
    );

    // And the list is closed: every cell counts, including the three this
    // program leaves off, because the bug is a sink that is not read here.
    for deck in 0..DECKS {
        let mut view = View::new(Room::Day);
        view.previews[deck] = Some(some);
        assert!(
            live(&view),
            "deck {deck} is rendering and the loop was told to sleep"
        );
    }

    // The other direction, which costs frames rather than pixels: with
    // every sink off the loop stops asking.
    view.previews[0] = None;
    assert!(
        !live(&view),
        "nothing is rendering and the loop stayed awake"
    );
}

/// Two paths or none, and anything else is a refusal rather than a guess.
///
/// [`sources_from`] is the whole of this program's command line and this is
/// what stops it growing a second one. The mistake it will actually be given is
/// *one* path — a Set is two files and reads like one thing — and that is
/// refused by name rather than paired with a default renderer, because a
/// program that silently supplied half the material would draw something nobody
/// asked for and say nothing about it.
///
/// A CPU test: nothing here opens a file, and a path that does not exist is
/// still a path. What is behind one is [`checked`]'s to complain about.
#[test]
fn a_set_is_two_paths_or_none_and_anything_else_is_refused() {
    let of = |args: &[&str]| sources_from(args.iter().map(|a| (*a).to_string()));

    let bare = of(&[]).expect("no arguments is the pair the preset library ships");
    assert_eq!(bare.sources.l1, shipped().l1);
    assert_eq!(bare.sources.l4, shipped().l4);
    assert!(
        bare.sources.l1.is_file() && bare.sources.l4.is_file(),
        "the default pair is not on the disk at {} and {}, so a bare run cannot draw",
        bare.sources.l1.display(),
        bare.sources.l4.display()
    );

    let named = of(&["a/geo.kir", "b/ren.kir"]).expect("two paths are a Set");
    assert_eq!(named.sources.l1, std::path::PathBuf::from("a/geo.kir"));
    assert_eq!(named.sources.l4, std::path::PathBuf::from("b/ren.kir"));
    assert_eq!(
        named.sources.material(),
        "geo + ren",
        "the strip is not named after what was actually loaded"
    );

    let one = of(&["a/geo.kir"]).expect_err(
        "one path was read as a Set, so this program would have invented the other half",
    );
    assert!(
        one.contains("a/geo.kir"),
        "the refusal `{one}` does not name the path it refused"
    );
    assert!(
        of(&["a.kir", "b.kir", "c.kir"]).is_err(),
        "three paths were read as a Set"
    );

    // **An empty message is `--help`**, which is the one arm that is a
    // request rather than a mistake — [`main`] prints [`USAGE`] to stdout
    // and exits 0 on it, and prints it to stderr and exits 2 on every
    // other. A refusal that came back empty would be a silent exit.
    assert_eq!(of(&["--help"]).err(), Some(String::new()));
    assert_eq!(of(&["-h"]).err(), Some(String::new()));
    assert!(
        !one.is_empty(),
        "a refusal came back with no sentence in it, which `main` reads as `--help`"
    );
}

/// `--mcp` takes a port, and it is refused in the three ways a flag with a
/// value is refused.
///
/// The first two are [`value_for`]'s and are the two the other flags already
/// meet — a flag at the end of the line does not fall back to a default, and a
/// flag whose value is the next flag does not eat it. The third is
/// [`number_for`]'s and is new here, because this is the first flag on this
/// command line that takes a number: a port that is not a port is a mistake on
/// the command line, and a run that started serving on some other number would
/// be the wrong kind of helpful.
#[test]
fn the_mcp_flag_takes_a_port_and_is_refused_the_three_ways_a_valued_flag_is() {
    let read = |args: &[&str]| sources_from(args.iter().map(|a| a.to_string()).collect::<Vec<_>>());

    let launch = read(&["--mcp", "8000"]).expect("a port is a port");
    assert_eq!(launch.mcp, Some(8000));
    // **On either side of the pair, like the two flags beside it.** An
    // operator types the flags in whatever order they think of them.
    let pair = shipped();
    let (l1, l4) = (pair.l1.display().to_string(), pair.l4.display().to_string());
    let launch = read(&[&l1, &l4, "--mcp", "0"]).expect("after the pair");
    assert_eq!(launch.mcp, Some(0), "a port after the pair");
    let launch = read(&["--mcp", "0", &l1, &l4]).expect("before the pair");
    assert_eq!(launch.mcp, Some(0), "a port before the pair");

    // And a run that does not ask serves nothing rather than a default port.
    assert_eq!(
        read(&[&l1, &l4]).expect("no flag").mcp,
        None,
        "a run that did not ask for a server was given one"
    );

    // The end of the line: nothing after the flag.
    let why = read(&["--mcp"]).expect_err("a flag with nothing after it");
    assert!(why.contains("--mcp"), "the refusal does not name it: {why}");
    assert!(
        why.contains("needs a value"),
        "the refusal is not the one the other flags give: {why}"
    );

    // The next flag is not a value: `--mcp --store x` must blame `--mcp`
    // rather than reading `--store` as a port and then blaming `x` for
    // being an unknown option.
    let why = read(&["--mcp", "--store", "somewhere"]).expect_err("a flag as a value");
    assert!(
        why.contains("--mcp") && why.contains("--store"),
        "the refusal does not say which flag ate which: {why}"
    );

    // And a value that is not a number.
    let why = read(&["--mcp", "eight-thousand"]).expect_err("a port that is not one");
    assert!(
        why.contains("eight-thousand") && why.contains("a port number"),
        "the refusal does not say what was expected: {why}"
    );
}

/// A wire request reaches the slot's watcher, and the rest of that watcher's
/// aim is restated with it.
///
/// The three points `mcp::WireRequest` owes, checked without a window: the edge
/// is replaced rather than appended and keyed on the input, the slot is
/// re-aimed with the run's whole wiring, and a slot this deck has not got is
/// refused in the one sentence every surface refuses one in.
///
/// The other fields are the point of the second assertion. An `Aim` is every
/// field of a slot's identity, and a rewiring that restated only the edges
/// would come back with the outgoing slot's camera, fold and salts — a defect
/// that shows on the *next* build rather than on the rewiring, which is why it
/// is asserted here rather than left to be seen. The Set the slot is running is
/// among them, and it is the one whose symptom is not a picture at all:
/// versions filed under the wrong Set, or under none.
#[test]
fn a_wire_request_reaches_the_slots_watcher_with_the_rest_of_its_aim_restated() {
    let edge = |node: &str, slot: &str, to: &str| karakuri_engine::set::Edge {
        node: node.to_string(),
        slot: slot.into(),
        to: to.to_string(),
    };
    let (tx, rx) = std::sync::mpsc::channel();
    let mut aims = vec![Aiming::new(
        tx,
        watch::Aim {
            head: karakuri_environment::compile::Named {
                name: Some("grid".into()),
                path: std::path::PathBuf::from("A0-grid.kir"),
            },
            rest: vec![karakuri_environment::compile::Named::bare("A1-points.kir")],
            layering: Layering::Composite,
            live: Some(0),
            capacity: Some(2048),
            seed_salt: 9,
            salts: vec![9],
            camera: karakuri_engine::camera::Orbit::default(),
            overrides: Vec::new(),
            published: Vec::new(),
            bindings: Vec::new(),
            edges: Vec::new(),
            authorities: Vec::new(),
            // **A slot running a Set**, which is what makes the assertion
            // below about `restated` rather than about a default.
            set: Some("night01".to_owned()),
        },
        karakuri_mcp::Slots::unpointed(),
        0,
    )];
    let mut edges = Vec::new();

    let said = rewired(
        &[(0, edge("warp", "shape", "field"))],
        &mut edges,
        &mut aims,
        1,
    );
    assert_eq!(said.len(), 1);
    let line = said[0].as_ref().expect("the slot is in range");
    assert!(
        line.contains("warp.shape=field") && line.contains("recompiling"),
        "the answer does not say what was wired or that anything rebuilds: {line}"
    );
    let aim = rx.try_recv().expect("the watcher was not re-aimed at all");
    assert_eq!(aim.edges, vec![edge("warp", "shape", "field")]);
    // **The fields that are not the edges.**
    assert_eq!(aim.head.name.as_deref(), Some("grid"));
    assert_eq!(
        aim.set.as_deref(),
        Some("night01"),
        "the rewiring dropped the Set the slot is running, so every version \
         written after it would be filed under none"
    );
    assert_eq!(aim.live, Some(0), "the fold was silently un-selected");
    assert_eq!(aim.capacity, Some(2048), "the capacity came back as none");
    assert_eq!(aim.salts, vec![9], "the salts would repaint every element");
    assert_eq!(aim.layering, Layering::Composite);

    // **The same input again is a replacement and not a second edge**,
    // because `SetError::SlotBoundTwice` refuses two edges on one input
    // where the Set is built — an append would make a model unable to
    // change its mind.
    let said = rewired(
        &[(0, edge("warp", "shape", "other"))],
        &mut edges,
        &mut aims,
        1,
    );
    assert!(said[0].is_ok(), "{:?}", said[0]);
    assert_eq!(
        edges,
        vec![edge("warp", "shape", "other")],
        "the run is wired with both, and the Set will refuse to build"
    );
    let aim = rx.try_recv().expect("the second request re-aimed nothing");
    assert_eq!(aim.edges, vec![edge("warp", "shape", "other")]);
    // And the aim the watcher is pointed at moved with it, so a third
    // request restates the second rather than the first.
    assert_eq!(aims[0].at.edges, vec![edge("warp", "shape", "other")]);

    // A slot this deck has not got, in the one sentence.
    let said = rewired(
        &[(3, edge("warp", "shape", "field"))],
        &mut edges,
        &mut aims,
        1,
    );
    let why = said[0].as_ref().expect_err("slot 3 of a deck of one");
    assert_eq!(
        why,
        &format!(
            "{}, and nothing was rewired",
            karakuri_environment::no_such_slot(3, 1)
        ),
        "the refusal is not the one every other surface gives"
    );
    assert_eq!(
        edges,
        vec![edge("warp", "shape", "other")],
        "a refused request wrote an edge anyway"
    );
    assert!(
        rx.try_recv().is_err(),
        "a refused request re-aimed a watcher"
    );
}

/// A press on the Inspector deck head's fold re-aims the slot, and the rest of
/// that watcher's aim is restated with it.
///
/// The test above one operation along, and it is the same property for the same
/// reason: a `watch::Aim` is every field of a slot's identity, so an arm that
/// changed the layering and left the rest behind would come back with the
/// outgoing slot's fold, capacity, salts, camera and Set — on the *next* build
/// rather than on the press, which is the hardest version of it to see
/// (ADR-0228, ADR-0314).
///
/// `Aiming::at` is what the second half asserts against. A press that sent an
/// aim and left `at` behind would leave the next re-aim restating the layering
/// the run launched with, so the third assertion here is that a *second* press
/// comes back to where the first one put it rather than to where the run
/// started.
///
/// No window, no device and no `Deck` — `composited` is a free function over
/// the aims for exactly this. A procedure loaded over a layer re-aims the slot
/// with exactly one file replaced, and leaves `Aim::set` where it is —
/// ADR-0338's decision 3, at the seam it crosses.
///
/// Three things it would be wrong about silently: the position it lands on (the
/// first node of that kind), the file it puts there (the procedure's own bytes,
/// in the deck's scratch), and everything else about the aim, which has to come
/// back restated rather than defaulted. The fourth is the one the maintainer
/// answered: the versions this slot writes from here on go on being filed under
/// the Set it started from.
///
/// No window, no device and no `Deck` — `overlaying` takes the aim. The strip
/// reads `<base> + <kir>` once a layer has been written over what a deck is
/// playing, and the base is the Set it is filed under — or the launch pair
/// where it is filed under none (ADR-0338).
#[test]
fn the_strip_reads_the_base_and_the_procedure_written_over_it() {
    assert_eq!(
        derived_material(
            &base_material(Some("drift_night"), "coil_vortex + star_flares"),
            "orbit_wide"
        ),
        "drift_night + orbit_wide"
    );
    // **A slot nobody has loaded a Set onto**: no id names what it is
    // running, so the base is the pair the run opened with.
    assert_eq!(
        derived_material(
            &base_material(None, "coil_vortex + star_flares"),
            "orbit_wide"
        ),
        "coil_vortex + star_flares + orbit_wide"
    );
}

#[test]
fn a_procedure_load_replaces_one_file_and_keeps_the_base_set() {
    let root = scratch_dir("procedure-load");
    Store::open(&root).expect("a store to keep in");
    std::fs::write(
        root.join(Store::PROCEDURES).join("orbit_wide.kir"),
        "proc orbit_wide {\n  kind L3\n}\n",
    )
    .expect("a kept procedure");
    // The slot's own two files, written where a watcher would be looking:
    // an L1 and an L4, which is the pair every run opens on.
    let l1 = karakuri_environment::scratch::place(&root, "A0-drift_shell", "kind L1\n")
        .expect("the geometry");
    let l4 = karakuri_environment::scratch::place(&root, "A1-star_flares", "kind L4\n")
        .expect("the renderer");

    let (tx, rx) = std::sync::mpsc::channel();
    let mut aim = Aiming::new(
        tx,
        watch::Aim {
            head: karakuri_environment::compile::Named {
                name: Some("shell".into()),
                path: l1.clone(),
            },
            rest: vec![karakuri_environment::compile::Named {
                name: Some("flares".into()),
                path: l4.clone(),
            }],
            layering: Layering::Composite,
            live: Some(2),
            capacity: Some(2048),
            seed_salt: 9,
            salts: vec![9],
            camera: karakuri_engine::camera::Orbit {
                radius: 3.5,
                ..karakuri_engine::camera::Orbit::default()
            },
            overrides: Vec::new(),
            published: Vec::new(),
            bindings: Vec::new(),
            edges: vec![karakuri_engine::set::Edge {
                node: "flares".to_string(),
                slot: "shape".into(),
                to: "shell".to_string(),
            }],
            authorities: Vec::new(),
            set: Some("drift_night".to_owned()),
        },
        karakuri_mcp::Slots::unpointed(),
        0,
    );

    // **The deck holds no camera, so the procedure is added as node 0 of
    // its kind** — the case the row is for.
    let line = overlaying(&root, None, 0, &mut aim, "orbit_wide").expect("the load was refused");
    assert!(
        line.contains("orbit_wide") && line.contains("kept") && line.contains("L3"),
        "{line}"
    );
    let sent = rx.try_recv().expect("no aim was sent");
    assert_eq!(sent.head.path, l1, "the geometry moved");
    assert_eq!(sent.rest.len(), 2, "the slot does not hold three nodes now");
    assert_eq!(sent.rest[0].path, l4, "the renderer moved");
    assert_eq!(
        std::fs::read_to_string(&sent.rest[1].path).expect("the camera's file"),
        "proc orbit_wide {\n  kind L3\n}\n",
        "the file the aim names is not the procedure's own bytes"
    );
    assert_eq!(
        sent.rest[1].name.as_deref(),
        Some("orbit_wide"),
        "a node added by this row is not named after it"
    );

    // **Everything else restated**, which is `Aiming::changed`'s single
    // derivation — the layering, the capacity, the salts, the camera and
    // the wiring come back as the slot's own.
    assert_eq!(sent.layering, Layering::Composite);
    assert_eq!(sent.capacity, Some(2048));
    assert_eq!(sent.salts, vec![9]);
    assert_eq!(sent.camera.radius, 3.5);
    assert_eq!(sent.edges.len(), 1);
    // **And the Set it is filed under does not move**, which is what keeps
    // the snapshot every compile takes alive (ADR-0304, ADR-0308).
    assert_eq!(sent.set.as_deref(), Some("drift_night"));

    // **A second load of the same kind lands on the node the first one
    // added**, which is *the first node of that kind* read a second time:
    // the slot still holds three nodes.
    std::fs::write(
        root.join(Store::PROCEDURES).join("tunnel_eye.kir"),
        "  kind L3\n",
    )
    .expect("a second camera");
    overlaying(&root, None, 0, &mut aim, "tunnel_eye").expect("the second load was refused");
    let sent = rx.try_recv().expect("no second aim was sent");
    assert_eq!(
        sent.rest.len(),
        2,
        "the second camera was added beside the first"
    );
    assert_eq!(
        std::fs::read_to_string(&sent.rest[1].path).expect("the camera's file"),
        "  kind L3\n"
    );
    assert_eq!(
        sent.rest[1].name.as_deref(),
        Some("orbit_wide"),
        "the replaced node did not keep the name the edges resolve against"
    );

    // **A renderer replaces the renderer that is there** — `L4:0`, and the
    // geometry does not move.
    std::fs::write(
        root.join(Store::PROCEDURES).join("hard_dots.kir"),
        "kind L4\n",
    )
    .expect("a renderer");
    overlaying(&root, None, 0, &mut aim, "hard_dots").expect("the renderer load was refused");
    let sent = rx.try_recv().expect("no third aim was sent");
    assert_eq!(sent.head.path, l1, "the geometry moved on a renderer load");
    assert_eq!(sent.rest.len(), 2);
    assert_eq!(
        sent.rest[0].name.as_deref(),
        Some("flares"),
        "the renderer did not keep its node name"
    );
    assert_ne!(
        sent.rest[0].path, l4,
        "the renderer's file was not replaced"
    );

    // **A name neither tier holds is refused with the name back**, and
    // nothing is sent.
    let why = overlaying(&root, None, 0, &mut aim, "no_such_thing")
        .expect_err("a name nothing holds was loaded");
    assert!(
        why.contains("no_such_thing") && why.contains("procedures"),
        "{why}"
    );
    assert!(rx.try_recv().is_err(), "a refused load sent an aim");

    // **A `.kir` that declares no kind is refused too**, because there is
    // no layer to write it over.
    std::fs::write(
        root.join(Store::PROCEDURES).join("mute.kir"),
        "// nothing\n",
    )
    .expect("a procedure with no kind");
    let why = overlaying(&root, None, 0, &mut aim, "mute")
        .expect_err("a procedure with no kind was loaded");
    assert!(why.contains("declares no `kind`"), "{why}");

    std::fs::remove_dir_all(&root).expect("clean up");
}

#[test]
fn a_composite_press_re_aims_the_slot_and_restates_the_rest_of_its_aim() {
    let (tx, rx) = std::sync::mpsc::channel();
    let mut aims = vec![Aiming::new(
        tx,
        watch::Aim {
            head: karakuri_environment::compile::Named {
                name: Some("grid".into()),
                path: std::path::PathBuf::from("A0-grid.kir"),
            },
            rest: vec![karakuri_environment::compile::Named::bare("A1-points.kir")],
            // **Overdrawing**, so the press below asks for the other one
            // and the assertion is about a field that moved.
            layering: Layering::Overdraw,
            live: Some(2),
            capacity: Some(2048),
            seed_salt: 9,
            salts: vec![9],
            // **A camera nobody's default produces**, so the assertion
            // below is about a value that was carried rather than one that
            // happens to coincide with `Orbit::default()`.
            camera: karakuri_engine::camera::Orbit {
                radius: 3.5,
                ..karakuri_engine::camera::Orbit::default()
            },
            overrides: Vec::new(),
            published: Vec::new(),
            bindings: Vec::new(),
            edges: vec![karakuri_engine::set::Edge {
                node: "warp".to_string(),
                slot: "shape".into(),
                to: "field".to_string(),
            }],
            authorities: Vec::new(),
            set: Some("night01".to_owned()),
        },
        karakuri_mcp::Slots::unpointed(),
        0,
    )];

    let line = composited(
        &mut aims,
        &Operation::SetCompositing {
            deck: 0,
            compositing: true,
        },
    )
    .expect("the arm answered nothing for the operation it is for");
    assert!(
        line.contains("composite") && line.contains("recompiling"),
        "the answer does not say what was asked for or that the slot rebuilds: {line}"
    );

    let aim = rx.try_recv().expect("the watcher was not re-aimed at all");
    assert_eq!(
        aim.layering,
        Layering::Composite,
        "the press did not move the one field it is about"
    );
    // **The thirteen that did not move.** Each of these is a symptom
    // somebody would meet on the next save rather than on this press.
    assert_eq!(aim.head.name.as_deref(), Some("grid"));
    assert_eq!(aim.rest.len(), 1);
    assert_eq!(aim.live, Some(2), "the fold was silently un-selected");
    assert_eq!(aim.capacity, Some(2048), "the capacity came back as none");
    assert_eq!(aim.seed_salt, 9);
    assert_eq!(aim.salts, vec![9], "the salts would repaint every element");
    // `Orbit` is not `PartialEq`, so the field the camera's own loss shows
    // in is what this reads — `Watch::camera`'s symptom is a slot back at
    // `Orbit::default()`, and a radius nobody could have written is what
    // tells the two apart.
    assert_eq!(
        aim.camera.radius, 3.5,
        "the camera came back at its default"
    );
    assert_eq!(aim.edges.len(), 1, "the run's wiring was dropped");
    assert_eq!(
        aim.set.as_deref(),
        Some("night01"),
        "the press dropped the Set the slot is running, so every version written after it \
         would be filed under none"
    );
    assert_eq!(aims[0].at.layering, Layering::Composite);

    // **Asking for the layering the slot is now in sends nothing**, because
    // a re-aim rebuilds the whole slot and this one would land on the same
    // picture (P-0091).
    let line = composited(
        &mut aims,
        &Operation::SetCompositing {
            deck: 0,
            compositing: true,
        },
    )
    .expect("the arm answered nothing");
    assert!(
        line.contains("already"),
        "the answer does not say the slot is already set that way: {line}"
    );
    assert!(
        rx.try_recv().is_err(),
        "a press asking for the state the slot is in recompiled it"
    );

    // **And the second press restates what the first one left**, which is
    // what keeping `Aiming::at` buys: back to overdraw, with the layering
    // read off the aim this program is holding rather than off the launch
    // pair.
    composited(
        &mut aims,
        &Operation::SetCompositing {
            deck: 0,
            compositing: false,
        },
    )
    .expect("the arm answered nothing");
    let aim = rx.try_recv().expect("the second press re-aimed nothing");
    assert_eq!(aim.layering, Layering::Overdraw);
    assert_eq!(aim.set.as_deref(), Some("night01"));

    // A slot this deck has not got, in the one sentence every surface
    // refuses one in.
    let why = composited(
        &mut aims,
        &Operation::SetCompositing {
            deck: 3,
            compositing: true,
        },
    )
    .expect("a slot the deck has not got answered nothing");
    assert!(
        why.contains(&karakuri_environment::no_such_slot(3, 1)),
        "the refusal is not the one every other surface gives: {why}"
    );
    assert!(rx.try_recv().is_err(), "a refused press re-aimed a watcher");

    // And it answers `None` for everything that is not its operation, so
    // the dispatch above can call it on every press.
    assert!(composited(&mut aims, &Operation::Quit).is_none());
}

/// The two flags say where this program's data is, and either may sit on either
/// side of the pair.
///
/// The order half is the one an operator meets: they type the flags in whatever
/// order they think of them, and `karakuri-cli` accepts `--store` before or
/// after its own command for exactly this reason
/// (`list_sets_prints_and_is_never_a_run`). A parser that matched on the
/// argument slice — which is what this one was — can only ever accept one of
/// the two spellings.
///
/// And the pair still wins, which is the claim [`Sources`]'s doc makes about
/// these flags not being a second material vocabulary: `--presets` moves what a
/// run with *no* paths opens on and reaches nothing else, so a line with both a
/// library and a pair plays the pair.
///
/// Not quite a CPU test, and this is what changed: resolving a presets root is
/// existence checks on real directories. The library it names is this
/// workspace's own `examples/`, which is on the disk whenever these tests run
/// at all.
#[test]
fn the_two_flags_say_where_the_data_is_and_may_sit_on_either_side_of_the_pair() {
    let of = |args: &[&str]| sources_from(args.iter().map(|a| (*a).to_string()));
    let library = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let library = library
        .to_str()
        .expect("this workspace's path is not utf-8");

    // **The default store is the shared constant**, which is the whole of
    // what deleting `const STORE` was for: this asserts the two programs
    // read one directory rather than two that look alike.
    assert_eq!(
        of(&[]).expect("a bare run").store,
        std::path::PathBuf::from(karakuri_environment::places::STORE),
        "a run that said nothing about a store did not get the shared default"
    );

    for spelling in [
        vec!["--store", "/tmp/library", "a/geo.kir", "b/ren.kir"],
        vec!["a/geo.kir", "b/ren.kir", "--store", "/tmp/library"],
        vec!["a/geo.kir", "--store", "/tmp/library", "b/ren.kir"],
    ] {
        let launch = of(&spelling).unwrap_or_else(|why| panic!("{spelling:?}: {why}"));
        assert_eq!(
            launch.store,
            std::path::PathBuf::from("/tmp/library"),
            "{spelling:?} read a store nobody asked for"
        );
        assert_eq!(
            launch.sources.l1,
            std::path::PathBuf::from("a/geo.kir"),
            "{spelling:?} lost the pair to the flag"
        );
        assert_eq!(launch.sources.l4, std::path::PathBuf::from("b/ren.kir"));
    }

    // `--presets` with no pair: it is what the pair defaults to, and the
    // resolution reports it as typed rather than as something found.
    let told = of(&["--presets", library]).expect("a library that is there");
    assert_eq!(
        told.sources.l1,
        std::path::Path::new(library).join("coil_vortex.kir")
    );
    assert_eq!(
        told.sources.l4,
        std::path::Path::new(library).join("star_flares.kir")
    );
    assert_eq!(
        told.presets.as_ref().map(|presets| presets.found),
        Some(karakuri_environment::places::Found::Given),
        "a `--presets` an operator typed was reported as a place this program went \
         looking in"
    );

    // And with a pair, on either side: the pair wins and the library is
    // still the one that was named.
    for spelling in [
        vec!["--presets", library, "a/geo.kir", "b/ren.kir"],
        vec!["a/geo.kir", "b/ren.kir", "--presets", library],
    ] {
        let launch = of(&spelling).unwrap_or_else(|why| panic!("{spelling:?}: {why}"));
        assert_eq!(
            launch.sources.l1,
            std::path::PathBuf::from("a/geo.kir"),
            "{spelling:?}: `--presets` overrode the paths the operator named, which \
             would make it a second way of saying what plays"
        );
        assert_eq!(launch.sources.l4, std::path::PathBuf::from("b/ren.kir"));
        assert_eq!(
            launch.presets.map(|presets| presets.dir),
            Some(std::path::PathBuf::from(library)),
            "{spelling:?} lost the library it was given"
        );
    }

    // A `--presets` that is not there is refused rather than searched
    // past, and the sentence is `places`' own — one refusal, whichever
    // program the operator reached it from.
    let missing = std::path::Path::new(library).join("no-such-library");
    let why = of(&["--presets", missing.to_str().expect("utf-8")])
        .expect_err("a `--presets` that is not there was accepted");
    assert_eq!(why, karakuri_environment::places::no_presets_at(&missing));

    // A flag with nothing after it, and a flag whose value is the next
    // flag. Neither falls back and neither swallows.
    for (spelling, wanted) in [
        (vec!["--presets"], "`--presets` needs a value"),
        (vec!["--store"], "`--store` needs a value"),
        (
            vec!["--presets", "--store", "/tmp/library"],
            "`--presets` was given no value — `--store` is an option, not one",
        ),
    ] {
        assert_eq!(
            of(&spelling).as_ref().err().map(String::as_str),
            Some(wanted),
            "{spelling:?}"
        );
    }

    // **An unknown option is not a path**, which is the mistake a typo
    // actually makes: without this, `--prests DIR` becomes a two-path Set
    // and is reported as a file that will not open.
    let typo = of(&["--prests", library]).expect_err("an unknown option was read as half of a Set");
    assert_eq!(typo, "unknown option `--prests`");
    assert!(
        !typo.is_empty(),
        "a refusal came back with no sentence in it, which `main` reads as `--help`"
    );
}

/// The capacity is the L1's own declaration, read off the `Checked`.
///
/// It was `const CAPACITY: u32 = 262144` here — `drift_shell.kir`'s declared
/// default, transcribed — for as long as this file could only ever load that
/// one file. It takes a path now, so a transcription would be right about one
/// `.kir` and silently wrong about every other: a procedure written for 131072
/// elements would run at 262144 and nothing would say so.
///
/// It is not `karakuri_ir::DEFAULT_CAPACITY` either, which is the language
/// default for a file that declared nothing and is what `check_header` makes
/// unreachable for an L1 that passed checking. The number below is asserted
/// rather than derived on purpose, and it is the reference workload's rather
/// than this program's: `docs/contributing.md` §1 names
/// `examples/drift_cloud.kset` at 1280x720, and 262144 is what that Set's L1
/// declares. It used to be asserted of whatever a bare `cargo run -p karakuri`
/// opened on, which coupled the workload to the demo and is ADR-0270. What is
/// still asserted of the shipped pair is that its capacity is read from its own
/// file, which is a different property and the one this test is named for.
#[test]
fn the_capacity_is_the_l1s_own_declaration_and_the_l4_declares_none() {
    let sources = shipped();

    // **The reference workload, pinned by name.** `drift_cloud.kset` is the
    // Set `docs/contributing.md` §1 names, and this is its L1. That the
    // `.kset` names these two parts is checked where every shipped Set is
    // composed, in `karakuri-cli`'s `examples` suite, so it is not
    // transcribed twice here.
    let reference = checked(&sources.l1.with_file_name("drift_shell.kir"));
    let pinned = reference
        .capacity
        .expect("an L1 that passed contract checking always carries a capacity");
    assert_eq!(
        pinned.default, 262_144,
        "`examples/drift_cloud.kset`'s L1 no longer declares the capacity every \
         host-clock figure in this repository was taken at, and \
         `docs/contributing.md` §1 names it as the one reference workload \
         (ADR-0270)"
    );

    // **And the pair this program opens on, checked for a per-file read and
    // not for a number.** ADR-0270 split these: which pair is the default is
    // a demo decision, and what it may not do is run at something other than
    // what its own file declares.
    let l1 = checked(&sources.l1);
    let declared = l1
        .capacity
        .expect("an L1 that passed contract checking always carries a capacity");
    assert!(
        declared.contains(declared.default),
        "the file's own default is outside the range the same file declares"
    );

    assert_eq!(capacity_of(&l1), declared.default);

    assert!(
        checked(&sources.l4).capacity.is_none(),
        "the renderer declares a capacity — `Set::build` is handed the L1's, and \
         two declarations would be two answers to how many elements there are"
    );

    // **A second L1, and it is the one that tells the two mistakes apart.**
    // The pin above is `drift_shell.kir` at 262144, which is also
    // `karakuri_ir::DEFAULT_CAPACITY` — so that assertion passes just as
    // well against a [`capacity_of`] that ignored the file and returned the
    // language default. `strand_shell.kir` declares 131072 and says why in
    // the file (512 strands x 256 samples), and it is what that defect
    // fails on. It is kept although the shipped pair no longer declares the
    // language default either (ADR-0271 moved it to `coil_vortex.kir` at
    // 10240): which pair is the default is a demo decision, and a test that
    // can only tell a per-file read from a constant while the demo happens
    // to be off the constant is a test that goes quiet the next time the
    // demo moves.
    let other = checked(&sources.l1.with_file_name("strand_shell.kir"));
    assert_eq!(
        capacity_of(&other),
        131_072,
        "a second procedure did not run at what it declares — the capacity is being \
         read from somewhere other than the file"
    );
    assert_ne!(
        capacity_of(&other),
        karakuri_ir::DEFAULT_CAPACITY,
        "the second procedure declares the language default, so this test can no \
         longer tell a per-file read from a constant — pick another `.kir`"
    );
}

/// The pair a bare run plays, for the tests that need one on the disk.
///
/// [`Sources::under`] takes a preset library and does not go looking for one;
/// this is the going-looking, and in a test binary the answer is always the
/// last candidate — the workspace this file was compiled in, which is also the
/// tree the test is run from. That is the development entry doing exactly what
/// it is for, and it is why these tests can assert the pair is on the disk
/// without an install anywhere.
///
/// A function rather than an `impl Default` on [`Sources`], because a `Default`
/// is what baked the build machine's own tree into a shipped binary: a type
/// whose default value is a search of the filesystem invites exactly that call
/// from production, and a production caller now has to say which library it
/// means.
///
/// One `.kir`, parsed and checked, for the tests that need a `Checked` and no
/// window.
///
/// Reachable from test modules because it is at the file's own scope.
///
/// `karakuri-environment`'s own five stages and not a sixth spelling. This used
/// to be a hand-rolled parse-then-check, which is what the run itself used to
/// build a slot from; the run compiles through
/// [`karakuri_environment::compile::sort_slot`] now, because that is the one
/// place that keeps the bytes a node's address is derived from
/// ([`karakuri_environment::compile::Placed::source`]). What is left here is a
/// test helper, and a test helper with its own compiler would be a second
/// answer to *does this file check* the day either moved.
#[cfg(test)]
pub(crate) fn checked(path: &std::path::Path) -> karakuri_ir::typed::Checked {
    match karakuri_environment::compile::load(path) {
        Ok((checked, _)) => checked,
        Err(report) => panic!("{report}"),
    }
}

#[cfg(test)]
pub(crate) fn shipped() -> Sources {
    let presets = karakuri_environment::places::presets(None)
        .expect("nothing was typed, so there is no typed path to refuse")
        .expect(
            "no preset library was found from the test binary, so the workspace tree this \
             test compiled in has no `examples/` in it",
        );
    Sources::under(&presets.dir)
}

/// The shipped pair in every slot, for the tests that build an [`Engine`].
///
/// A *run* may not do this — [`working_copies`] is what a run calls, and its
/// whole point is that no two slots watch one file — and this helper is not a
/// way back to that. It is legal here for the reason the copies exist: nothing
/// in these tests edits a `.kir`, no watcher of theirs ever sees a change, and
/// a test that materialised into a temporary store would be asserting the
/// copies rather than the thing it is about. The one test that *is* about the
/// copies calls `working_copies` and is named after the claim.
#[cfg(test)]
pub(crate) fn shipped_slots() -> Vec<Sources> {
    std::iter::repeat_n(shipped(), SLOTS).collect()
}

/// The reference workload's pair, for the tests whose claim is about a cost
/// rather than about what this program opens on.
///
/// `docs/contributing.md` §1 names `examples/drift_cloud.kset` —
/// `drift_shell.kir` at the 262144 elements it declares, with `soft_points.kir`
/// — and this resolves those two out of the same preset library [`shipped`]
/// answers from. It is deliberately not [`shipped_slots`], and the two were one
/// value until 2026-09-07.
///
/// What separated them is a test going quiet rather than red.
/// [`ADR-0271`](../../../docs/adr/0271-the-panel-opens-on-the-demo-rather-than-on-the-reference-workloads-pair.md)
/// moved the default pair to `examples/star_vortex.kset`'s two parts, which are
/// closed-form and 10240 elements.
/// `gpu::the_budget_parks_a_deck_and_the_strip_carries_both_residencies` then
/// measured 1.8 ms a slot against a 2.7 ms headroom and the governor answered
/// `NoPrimingNeeded` — a closed-form Set with nothing to warm — so the park the
/// test is named for was still a park and no longer the budget's. Which pair a
/// bare run opens on is a demo decision (ADR-0270); whether the budget refuses
/// a second Live slot is not, and it needs material chosen for its cost.
#[cfg(test)]
pub(crate) fn reference() -> Sources {
    let shipped = shipped();
    Sources {
        l1: shipped.l1.with_file_name("drift_shell.kir"),
        l4: shipped.l4.with_file_name("soft_points.kir"),
    }
}

#[cfg(test)]
pub(crate) fn empty_keeping() -> Keeping {
    let (_, built) = std::sync::mpsc::channel();
    let (save_tx, saves) = std::sync::mpsc::channel();
    let (send_tx, sends) = std::sync::mpsc::channel();
    let (keep_tx, keeps) = std::sync::mpsc::channel();
    Keeping {
        mcp: None,
        playing: Playing {
            playing: Vec::new(),
        },
        built,
        pending: Vec::new(),
        saves,
        save_tx,
        sends,
        send_tx,
        keeps,
        keep_tx,
        in_flight: 0,
    }
}

/// **A surface is made by the instance its adapter came from, and this
/// program has one.** `routed` opened the projector's surface on a fresh
/// `Gpu::instance()` and then asked it about `gfx.gpu.adapter`, which belongs
/// to the instance `resumed` made — a resource the fresh instance does not
/// hold, so `wgpu-core` aborted inside the `winit` mouse callback with no
/// sentence anywhere the moment the projector chip was pressed. Nothing can
/// open that window in a test (ADR-0324), so the wiring is pinned by reading
/// the source: exactly one `Gpu::instance()` in this crate, in `resumed`, and
/// every `create_surface` after it on the instance the `Gpu` keeps.
#[test]
fn every_surface_is_made_by_the_instance_the_adapter_came_from() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut fresh = Vec::new();
    let mut surfaces = Vec::new();
    let mut stack = vec![root.clone()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                if path.file_name().is_some_and(|n| n == "tests") {
                    continue;
                }
                stack.push(path);
                continue;
            }
            if path.extension().is_none_or(|e| e != "rs") {
                continue;
            }
            let src = std::fs::read_to_string(&path).unwrap();
            let rel = path.strip_prefix(&root).unwrap().display().to_string();
            for (i, line) in src.lines().enumerate() {
                let code = line.split("//").next().unwrap_or("");
                if code.contains("Gpu::instance()") {
                    fresh.push(format!("{rel}:{}", i + 1));
                }
                if code.contains("create_surface(") {
                    surfaces.push((format!("{rel}:{}", i + 1), code.trim().to_string()));
                }
            }
        }
    }
    assert_eq!(
        fresh,
        vec!["app/handler/mod.rs:83".to_string()],
        "a second wgpu::Instance would hold none of the first one's adapters: {fresh:?}"
    );
    assert!(!surfaces.is_empty(), "no surface is made anywhere");
    for (at, code) in &surfaces {
        assert!(
            code.contains("instance.create_surface(")
                || code.contains("gfx.gpu.instance.create_surface("),
            "{at}: a surface made off something other than the adapter's own instance: `{code}`"
        );
        assert!(
            !code.contains("Gpu::instance().create_surface("),
            "{at}: a surface on a fresh instance, whose adapter is another instance's: `{code}`"
        );
    }
}

/// **The picture format is a value read off a surface, and never a constant.**
/// It was `const PICTURE_FORMAT: TextureFormat = Rgba8UnormSrgb`, and no Metal
/// surface offers that format — so `routed` refused to open the projector on
/// every macOS run, naming a format the machine was never going to have. The
/// format is now read off the console's own surface in `resumed` and threaded
/// from there (ADR-0361). Nothing can open that window in a test (ADR-0324),
/// so both halves are pinned by reading the source: no 8-bit sRGB format is
/// named anywhere outside `tests/`, and the projector's check is against the
/// value the surface gave.
#[test]
fn a_picture_format_is_a_value_read_off_a_surface() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut named = Vec::new();
    let mut compared = Vec::new();
    let mut stack = vec![root.clone()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                if path.file_name().is_some_and(|n| n == "tests") {
                    continue;
                }
                stack.push(path);
                continue;
            }
            if path.extension().is_none_or(|e| e != "rs") {
                continue;
            }
            let src = std::fs::read_to_string(&path).unwrap();
            let rel = path.strip_prefix(&root).unwrap().display().to_string();
            for (i, line) in src.lines().enumerate() {
                let code = line.split("//").next().unwrap_or("");
                for spelling in ["Rgba8UnormSrgb", "Bgra8UnormSrgb"] {
                    if code.contains(spelling) {
                        named.push(format!("{rel}:{}: `{}`", i + 1, code.trim()));
                    }
                }
                if code.contains("caps.formats.contains(") {
                    compared.push((format!("{rel}:{}", i + 1), code.trim().to_string()));
                }
            }
        }
    }
    assert!(
        named.is_empty(),
        "an 8-bit sRGB format named in the source is a guess about a display that Metal \
         already falsifies — read it off the surface instead: {named:?}"
    );
    assert_eq!(
        compared.len(),
        1,
        "the projector's format check is the one place a surface's formats are asked for a \
         member, and it has moved or multiplied: {compared:?}"
    );
    let (at, code) = &compared[0];
    assert!(
        at.starts_with("app/operations.rs"),
        "{at}: the projector's format check has moved out of `routed`: `{code}`"
    );
    assert!(
        code.contains("gfx.picture_format"),
        "{at}: the projector is checked against something other than the format the console's \
         surface gave: `{code}`"
    );
}
