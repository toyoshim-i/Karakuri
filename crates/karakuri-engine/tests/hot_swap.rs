//! The third clause of the V1 assumption: **can we hot-swap it without
//! dropping a frame?**
//!
//! The headline is a timing claim and timing assertions are flaky, so it is
//! split. What is *asserted* here is structural and deterministic: that a swap
//! lands on a frame boundary and not inside one, that frames keep being
//! produced while a build is in flight, that a build which fails changes
//! nothing at all, and that a rollback restores the previous Set rather than
//! merely stopping the new one. What is *measured* is printed rather than
//! asserted — see `frame_times_across_a_swap_are_measured_and_reported` at the
//! bottom, and the numbers it produced in `README.md`.
//!
//! ## The harness waits for the GPU each frame, and the real one does not
//!
//! `Harness::frame` submits and then calls `device.poll(PollType::Wait)`. That
//! is the submit-and-wait pattern the render thread must never use; it is here
//! for two reasons. It bounds a headless loop that would otherwise queue
//! thousands of command buffers ahead of the GPU, standing in for the vsync
//! that bounds a real window. And it makes the interval `HotSwap` measures
//! include the GPU actually finishing, rather than only the cost of recording
//! and handing over the work — which is what makes the printed numbers mean
//! something. `probe.rs` documents the same trade for the same reason: a host
//! clock around submit-and-wait is coarse and biased high, and it is a real
//! number.

use std::sync::mpsc::{self, Sender};
use std::time::{Duration, Instant};

use karakuri_engine::swap::{Event, HotSwap, Request, Source};
use karakuri_engine::{Binding, Curve, Gpu, Present, Set, Signals, VideoSource};
use karakuri_ir::typed::Checked;

/// Deliberately small for the structural tests: what they check does not
/// depend on the workload, and a dozen of them at a realistic size would make
/// `cargo test` a coffee break. The measurement at the bottom of this file
/// uses [`REAL`] instead, because its numbers do depend on it.
const WIDTH: u32 = 256;
const HEIGHT: u32 = 256;

/// The two capacities the structural tests build at. They differ so that which
/// Set is live is observable from outside: `capacity` is a Set-level dial, so
/// the same pair of procedures at two capacities is two Sets and one artifact.
const FIRST: u32 = 4096;
const SECOND: u32 = 8192;

/// The workload the reported numbers are taken at — the CLI's own defaults,
/// so that they are comparable with the other host-clock figures in
/// `README.md` rather than being a measurement of a toy.
const REAL: (u32, (u32, u32)) = (262_144, (1280, 720));

/// A budget no frame in this harness will come near, for the tests that want a
/// candidate accepted rather than rolled back.
const GENEROUS_MS: f32 = 10_000.0;

/// How long a test will spin waiting for the worker before giving up. Generous:
/// it covers WGSL generation, two `create_shader_module` calls, pipeline
/// creation, and a whole-capacity buffer upload, on whatever machine CI turns
/// out to be.
const PATIENCE: Duration = Duration::from_secs(30);

const L1: &str = r#"
proc static_shell {
  kind     L1
  topology points
  capacity [1024, 262144] = 4096

  param radius : float [0.1, 8.0] = 2.5

  emit position, age

  element {
    let u = hash1(seed);
    let v = hash1(seed + 1000u);
    position = sphere_point(u, v) * radius;
    age      = age + dt;
  }
}
"#;

const L4: &str = r#"
proc soft_points {
  kind  L4
  blend additive

  consumes position

  param exposure : float [0.0, 8.0] = 1.0

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_size = 4.0;
  }

  fragment {
    let d = length(point_coord * 2.0 - 1.0);
    color = vec4(vec3(1.0, 1.0, 1.0) * exposure, max(0.0, 1.0 - d));
  }
}
"#;

/// An L4 consuming an attribute `L1` does not emit. `Set::build` refuses this
/// pair — stage 6, the composition check — which is the cheapest way to get a
/// build that fails *on the worker thread*, as opposed to one that fails
/// earlier and never becomes a `Request` at all.
const L4_INCOMPATIBLE: &str = r#"
proc wants_velocity {
  kind  L4
  blend additive

  consumes position, velocity

  vertex {
    clip       = camera * vec4(position + velocity, 1.0);
    point_size = 4.0;
  }

  fragment {
    color = vec4(1.0, 1.0, 1.0, 1.0);
  }
}
"#;

fn compile(src: &str) -> Checked {
    let proc = karakuri_ir::parse(src).unwrap_or_else(|e| panic!("{}", render(&e, src)));
    let checked = karakuri_ir::check::check(&proc).unwrap_or_else(|e| panic!("{}", render(&e, src)));
    karakuri_ir::cost::estimate(&checked).unwrap_or_else(|e| panic!("{}", render(&e, src)));
    checked
}

fn render(errs: &[karakuri_ir::IrError], src: &str) -> String {
    errs.iter()
        .map(|e| e.render(src))
        .collect::<Vec<_>>()
        .join("\n")
}

fn request(l4_src: &str, capacity: u32, label: &str) -> Request {
    Request {
        l1: compile(L1),
        l4: compile(l4_src),
        capacity,
        seed_salt: 19274,
        params: Vec::new(),
        bindings: Vec::new(),
        label: label.to_string(),
    }
}

/// A source that never has anything to build — what a `.kir` file that failed
/// to compile leaves behind. It still has to sleep, or it spins the worker.
struct Silent;

impl Source for Silent {
    fn poll(&mut self) -> Option<Request> {
        std::thread::sleep(karakuri_engine::swap::POLL_INTERVAL);
        None
    }
}

struct Harness {
    gpu: Gpu,
    present: Present,
    size: (u32, u32),
    swap: HotSwap,
    /// Every frame interval this harness has measured for itself, in
    /// milliseconds. Independent of the one `HotSwap` keeps, so the printed
    /// numbers are not the watchdog reporting on its own homework.
    intervals: Vec<f32>,
    last: Option<Instant>,
    /// `capacity` observed at the top and at the bottom of each frame body. If
    /// these ever differ, a swap landed in the middle of a frame.
    frame_capacities: Vec<(u32, u32)>,
}

impl Harness {
    fn new(
        budget_ms: f32,
        capacity: u32,
        size: (u32, u32),
        source: Box<dyn Source>,
    ) -> Harness {
        let gpu = Gpu::headless().expect("no GPU available");
        let present =
            Present::new(&gpu.device, wgpu::TextureFormat::Rgba16Float, size.0, size.1);
        let set = Harness::build(&gpu, L4, capacity);
        let mut swap = HotSwap::new(&gpu.device, &gpu.queue, set, budget_ms, source);
        swap.resize(size.0, size.1);
        Harness {
            gpu,
            present,
            size,
            swap,
            intervals: Vec::new(),
            last: None,
            frame_capacities: Vec::new(),
        }
    }

    /// A harness plus the `Sender` that drives it, for the tests that decide
    /// when a rebuild happens rather than watching a file for it.
    fn channel_driven(budget_ms: f32) -> (Harness, Sender<Request>) {
        Harness::channel_driven_at(budget_ms, FIRST, (WIDTH, HEIGHT))
    }

    fn channel_driven_at(
        budget_ms: f32,
        capacity: u32,
        size: (u32, u32),
    ) -> (Harness, Sender<Request>) {
        let (tx, rx) = mpsc::channel();
        (Harness::new(budget_ms, capacity, size, Box::new(rx)), tx)
    }

    fn build(gpu: &Gpu, l4_src: &str, capacity: u32) -> Set {
        Set::build(
            &gpu.device,
            &gpu.queue,
            &compile(L1),
            &compile(l4_src),
            capacity,
            19274,
        )
        .expect("the pair is compatible and the capacity is in range")
    }

    /// One frame, shaped exactly like the CLI's: `begin_frame`, then the whole
    /// frame recorded through the borrow it returns.
    fn frame(&mut self) {
        let now = Instant::now();
        if let Some(last) = self.last.replace(now) {
            self.intervals
                .push(now.duration_since(last).as_secs_f32() * 1_000.0);
        }

        let hdr = self.present.hdr_view();
        let device = &self.gpu.device;
        let queue = &self.gpu.queue;

        let set = self.swap.begin_frame();
        let at_top = set.capacity();
        set.prepare(queue, 1, &Signals::default());
        let mut encoder = device.create_command_encoder(&Default::default());
        set.render(&mut encoder, hdr, 1);
        let at_bottom = set.capacity();
        queue.submit([encoder.finish()]);

        self.frame_capacities.push((at_top, at_bottom));
        // See the module doc: standing in for vsync, and what makes the
        // measured interval include the GPU rather than only the submission.
        device.poll(wgpu::PollType::Wait).expect("poll");
    }

    /// Render frames until `wanted` matches an event, and return how many
    /// frames that took. Every event seen along the way is collected, so a
    /// caller can check that nothing else happened either.
    fn frames_until(
        &mut self,
        wanted: impl Fn(&Event) -> bool,
        what: &str,
    ) -> (u64, Vec<String>) {
        let started = Instant::now();
        let from = self.swap.frames_rendered();
        let mut seen = Vec::new();
        loop {
            self.frame();
            let mut found = false;
            for event in self.swap.events() {
                found |= wanted(&event);
                seen.push(event.to_string());
            }
            if found {
                return (self.swap.frames_rendered() - from, seen);
            }
            assert!(
                started.elapsed() < PATIENCE,
                "waited {PATIENCE:?} for {what} and it never happened; saw {seen:?}"
            );
        }
    }
}

fn is_swapped(event: &Event) -> bool {
    matches!(event, Event::Swapped { .. })
}

/// Simulation steps a Set has taken.
///
/// `Set::time` is the only public witness of it, and it is `steps * dt` at
/// `dt = 1/60` — a product that does not round to the same `f32` as the same
/// count divided by sixty, which is exactly the kind of last-bit difference
/// `Set` derives `t` from an integer counter to avoid in the first place. So
/// the tests below compare step counts and let this recover the integer,
/// rather than comparing two float expressions that mean the same thing.
fn steps_taken(set: &Set) -> u64 {
    (set.time() * 60.0).round() as u64
}

// ---------------------------------------------------------------------------
// Structural, asserted.
// ---------------------------------------------------------------------------

/// The claim, operationally: **a build does not block the render loop, and its
/// result appears between two frames rather than inside one.**
///
/// "Does not block" is not directly observable — there is no such thing as
/// asking a loop whether it was blocked. What is observable is that frames
/// continued to be produced between the request going out and the swap coming
/// back, which is the same statement from the outside. A `recv` instead of a
/// `try_recv` on the frame path would make that count zero.
///
/// "Not inside a frame" is checked by observing the live Set's identity at the
/// top and at the bottom of every frame body. Note what that does *not* check:
/// the borrow `begin_frame` returns stops a second call while it is held, but
/// the encoder is the caller's and borrows nothing, so a caller that calls
/// `begin_frame` twice inside one encoder still gets two Sets in one frame.
/// This asserts the property for a frame loop shaped like the CLI's, which is
/// the convention both callers keep — see the module doc on `swap.rs`.
#[test]
fn a_build_runs_in_the_background_and_lands_between_two_frames() {
    let (mut h, tx) = Harness::channel_driven(GENEROUS_MS);

    for _ in 0..10 {
        h.frame();
    }
    assert_eq!(h.swap.set().capacity(), FIRST);
    let t_before = h.swap.set().time();
    assert!(t_before > 0.0, "the first Set never stepped");

    tx.send(request(L4, SECOND, "second")).expect("worker alive");
    let (in_flight, _) = h.frames_until(is_swapped, "the swap");

    assert!(
        in_flight > 1,
        "only {in_flight} frame(s) were produced between the request and the swap — \
         the render loop waited for the build instead of polling for it"
    );
    assert_eq!(
        h.swap.set().capacity(),
        SECOND,
        "the swap reported success but the live Set is still the old one"
    );

    // No state transfer, by design: a new procedure means new buffers, so the
    // incoming Set starts cold. Priming a Set out of sight before showing it
    // is `docs/roadmap.md`'s M2, and a partial version of it here would be
    // something M2 has to remove.
    //
    // One step, not zero: the frame the swap landed on is a rendered frame
    // like any other, and it stepped the Set that was live at its top — which
    // by then was the new one. The claim is that it started from zero, and one
    // step after ten frames of the old Set is that claim.
    assert_eq!(
        steps_taken(h.swap.set()),
        1,
        "the swapped-in Set inherited a `t`; V1 swaps cold and M2 is what warms them"
    );

    for (i, (top, bottom)) in h.frame_capacities.iter().enumerate() {
        assert_eq!(
            top, bottom,
            "frame {i} began with a Set of capacity {top} and ended with one of {bottom}: \
             a swap landed in the middle of a frame"
        );
    }
    // And exactly one changeover happened across the whole run, at a frame
    // boundary — not one Set for the compute pass and another for the draw.
    let changes = h
        .frame_capacities
        .windows(2)
        .filter(|w| w[0].0 != w[1].0)
        .count();
    assert_eq!(changes, 1, "expected exactly one swap, saw {changes}");
}

/// A swapped-in Set carries the bindings the request stated.
///
/// A binding is Set state and a swap builds a whole new Set, so this is the
/// same "carried by being restated" the params already are — and losing it is
/// silent: `--watch` would keep working, the picture would keep updating, and
/// the only symptom would be a parameter that quietly stopped moving after the
/// first save.
#[test]
fn a_swapped_in_set_carries_the_bindings_the_request_stated() {
    let (mut h, tx) = Harness::channel_driven(GENEROUS_MS);

    for _ in 0..3 {
        h.frame();
    }
    assert!(h.swap.set().bindings().is_empty());

    let mut req = request(L4, SECOND, "bound");
    // The param moved by hand *and* bound, since the two travel together and
    // the binding has to be applied after the override to blend from it.
    req.params = vec![("radius".to_string(), 4.0)];
    req.bindings = vec![Binding::new(
        karakuri_ir::Kind::L1,
        "radius",
        "beat",
        Curve::Pow2,
        [1.0, 5.0],
    )];
    tx.send(req).expect("worker alive");
    h.frames_until(is_swapped, "the swap");

    let set = h.swap.set();
    assert_eq!(set.capacity(), SECOND, "the swap did not land");
    assert_eq!(set.params["radius"], 4.0, "the override did not survive");
    assert_eq!(
        set.bindings().len(),
        1,
        "the swapped-in Set lost the binding the request stated"
    );
    assert_eq!(set.bindings()[0].signal, "beat");
}

/// A build that fails leaves the running Set **completely** untouched: not
/// merely still rendering, but at the same `t`, with the same live count, and
/// the same buffers. The composition check is the failure used here because it
/// happens inside `Set::build`, on the worker thread, which is the case a
/// render thread could plausibly mishandle — it is holding a `Result` and has
/// to put the `Err` down without disturbing anything.
#[test]
fn a_build_that_fails_changes_nothing() {
    let (mut h, tx) = Harness::channel_driven(GENEROUS_MS);

    for _ in 0..10 {
        h.frame();
    }
    let live_before = h.swap.set().live_count(&h.gpu.device, &h.gpu.queue);
    let frames_before = h.swap.frames_rendered();

    tx.send(request(L4_INCOMPATIBLE, SECOND, "wants_velocity"))
        .expect("worker alive");
    let (elapsed, seen) = h.frames_until(
        |e| matches!(e, Event::Rejected { .. }),
        "the rejection",
    );

    assert_eq!(
        h.swap.set().capacity(),
        FIRST,
        "a failed build replaced the live Set"
    );
    assert_eq!(
        h.swap.set().live_count(&h.gpu.device, &h.gpu.queue),
        live_before,
        "a failed build disturbed the live Set's element buffers"
    );
    // `t` advanced by exactly the frames that were rendered, one step each:
    // the running Set was neither reset nor paused while the build failed.
    assert_eq!(
        steps_taken(h.swap.set()),
        frames_before + elapsed,
        "the running Set's clock did not advance normally through the rejection"
    );
    assert!(
        !h.swap.on_trial(),
        "a failed build started a watchdog trial over nothing"
    );

    let rejection = seen
        .iter()
        .find(|s| s.contains("wants_velocity"))
        .unwrap_or_else(|| panic!("no diagnostic naming the candidate: {seen:?}"));
    assert!(
        rejection.contains("velocity"),
        "the rejection does not say what was wrong: {rejection}"
    );
}

/// The other half of "a failed compile changes nothing", and the half that is
/// enforced by shape rather than by handling: a `.kir` that does not compile
/// never becomes a `Request`, so there is nothing for the render thread to
/// reject. A [`Source`] that produces nothing is exactly what
/// `karakuri-cli`'s watcher becomes on a parse error, and the running Set must
/// not notice.
#[test]
fn a_source_that_produces_nothing_leaves_the_running_set_running() {
    let mut h = Harness::new(GENEROUS_MS, FIRST, (WIDTH, HEIGHT), Box::new(Silent));

    // Long enough that the worker has polled its source many times over.
    for _ in 0..60 {
        h.frame();
    }

    let events: Vec<String> = h.swap.events().map(|e| e.to_string()).collect();
    assert!(events.is_empty(), "something happened: {events:?}");
    assert_eq!(h.swap.set().capacity(), FIRST);
    assert_eq!(steps_taken(h.swap.set()), 60);
    assert!(!h.swap.on_trial());
}

/// Rollback, forced with an absurd budget rather than with a slow shader — a
/// procedure heavy enough to miss the budget on one machine is comfortable on
/// another, and a test that depends on which is which is not a test.
///
/// The assertion that matters is not that the rollback *fired*; it is that
/// what came back is the **same Set**, still holding the state it was parked
/// with. `t` is the sharpest available witness of that: simulation time only
/// advances through `prepare`, nothing called `prepare` on the outgoing Set
/// while the candidate was on trial, so a restored Set must resume at exactly
/// the `t` it stopped at. A Set that had been rebuilt, or reset, or kept
/// stepping in the background would all show up here.
#[test]
fn the_watchdog_rolls_back_and_restores_the_previous_set_where_it_was_parked() {
    // Nothing is faster than zero milliseconds, so every candidate fails.
    let (mut h, tx) = Harness::channel_driven(0.0);

    for _ in 0..10 {
        h.frame();
    }
    tx.send(request(L4, SECOND, "second")).expect("worker alive");

    // Where the outgoing Set was parked: its step count at the top of the
    // frame the swap landed on, which is the last moment anything stepped it.
    let mut parked = None;
    let started = Instant::now();
    while parked.is_none() {
        let before = steps_taken(h.swap.set());
        h.frame();
        for event in h.swap.events() {
            if is_swapped(&event) {
                parked = Some(before);
            }
        }
        assert!(started.elapsed() < PATIENCE, "the swap never happened");
    }
    let parked = parked.expect("just set");
    assert_eq!(h.swap.set().capacity(), SECOND, "the candidate is live");
    assert!(h.swap.on_trial(), "the candidate is not being watched");

    let (frames, seen) = h.frames_until(
        |e| matches!(e, Event::RolledBack { .. }),
        "the rollback",
    );

    assert_eq!(
        h.swap.set().capacity(),
        FIRST,
        "the rollback fired but did not restore the previous Set"
    );
    // `parked + 1`, not `parked`: the frame the rollback landed on stepped the
    // restored Set once on its way past, exactly as it would have stepped any
    // other live Set. The claim is that it resumed from where it stopped and
    // not from zero, and not from somewhere it drifted to while parked.
    assert_eq!(
        steps_taken(h.swap.set()),
        parked + 1,
        "the restored Set is not the one that was parked: it was left at {parked} steps \
         and came back at {}",
        steps_taken(h.swap.set())
    );
    assert!(
        !h.swap.on_trial(),
        "the trial did not end when the verdict came in"
    );
    // The verdict waited for a window rather than firing on the first frame —
    // a watchdog that judged frame one would roll back every candidate that
    // ever existed, because a cold Set's first frame pays for its own upload.
    assert!(
        frames > 8,
        "the verdict came after {frames} frames, which is inside the warmup"
    );

    let rollback = seen
        .iter()
        .find(|s| s.contains("rolled back"))
        .unwrap_or_else(|| panic!("no rollback message: {seen:?}"));
    assert!(
        rollback.contains("host clock"),
        "the rollback message does not say what kind of number it decided on: {rollback}"
    );
}

/// A candidate that fits is kept, and the old Set is released — the other
/// branch of the same verdict, and the one that has to work for a hot swap to
/// be useful rather than merely safe.
#[test]
fn a_candidate_that_holds_the_budget_is_kept() {
    let (mut h, tx) = Harness::channel_driven(GENEROUS_MS);

    for _ in 0..5 {
        h.frame();
    }
    tx.send(request(L4, SECOND, "second")).expect("worker alive");
    h.frames_until(is_swapped, "the swap");
    let (_, seen) = h.frames_until(|e| matches!(e, Event::Accepted { .. }), "the verdict");

    assert_eq!(h.swap.set().capacity(), SECOND, "the candidate was kept");
    assert!(!h.swap.on_trial());
    assert!(
        seen.iter().any(|s| s.contains("held the budget")),
        "no acceptance message: {seen:?}"
    );
}

/// Two saves during one judging window leave two finished builds behind it, and
/// the channel is FIFO. Installing the front of that queue would put a
/// superseded Set on screen for a whole window — thirty-eight frames of a `.kir`
/// the operator has already replaced — before reaching the current one. The
/// verdict frame has to drain to the newest.
///
/// No frames are rendered while `b` and `c` build, so nothing can be installed
/// and the trial over `a` cannot end: both results are guaranteed to be waiting
/// when the window finally closes.
#[test]
fn the_build_installed_after_a_verdict_is_the_newest_one() {
    const THIRD: u32 = 12_288;
    const FOURTH: u32 = 16_384;

    let (mut h, tx) = Harness::channel_driven(GENEROUS_MS);
    for _ in 0..5 {
        h.frame();
    }
    tx.send(request(L4, SECOND, "a")).expect("worker alive");
    h.frames_until(is_swapped, "the first swap");
    assert!(h.swap.on_trial(), "`a` is not being watched");

    tx.send(request(L4, THIRD, "b")).expect("worker alive");
    tx.send(request(L4, FOURTH, "c")).expect("worker alive");
    let waited = Instant::now();
    while waited.elapsed() < Duration::from_secs(5) {
        std::thread::sleep(Duration::from_millis(50));
    }

    let seen = h.frames_until(is_swapped, "the swap after the verdict").1;
    assert_eq!(
        h.swap.set().capacity(),
        FOURTH,
        "a superseded build was installed after the verdict; events: {seen:?}"
    );
    assert!(
        !seen.iter().any(|s| s.contains("`b`")),
        "`b` was superseded before it was ever live and should not have been shown: {seen:?}"
    );
}

/// The worker can only leave its loop by panicking, and when it does its end of
/// the channel closes. `Disconnected` and `Empty` are otherwise the same thing
/// to the render thread, so without a distinction a `--watch` session would go
/// on rendering and silently ignore every save for the rest of the run. It is
/// reported once and the live Set is untouched.
#[test]
fn a_worker_that_dies_is_reported_once_and_does_not_disturb_the_live_set() {
    struct Exploding(u32);
    impl Source for Exploding {
        fn poll(&mut self) -> Option<Request> {
            self.0 += 1;
            if self.0 > 2 {
                panic!("deliberate: a build worker that will not come back");
            }
            std::thread::sleep(karakuri_engine::swap::POLL_INTERVAL);
            None
        }
    }

    let mut h = Harness::new(GENEROUS_MS, FIRST, (WIDTH, HEIGHT), Box::new(Exploding(0)));
    let started = Instant::now();
    let mut lost = 0;
    while started.elapsed() < Duration::from_secs(5) {
        h.frame();
        lost += h
            .swap
            .events()
            .filter(|e| matches!(e, Event::WorkerLost))
            .count();
        if lost > 0 && started.elapsed() > Duration::from_secs(1) {
            break;
        }
    }

    assert_eq!(lost, 1, "the dead worker was reported {lost} times, not once");
    assert_eq!(h.swap.set().capacity(), FIRST, "the live Set was disturbed");
    assert!(!h.swap.on_trial());
}

// ---------------------------------------------------------------------------
// Measured, reported.
// ---------------------------------------------------------------------------

/// Wall-clock frame intervals across a swap: worst case and median, before,
/// during, and after. **Printed, not asserted** — see the module doc. Run with
/// `cargo test -p karakuri-engine --test hot_swap -- --nocapture` to see them;
/// the numbers this produced on the development machine are in `README.md`,
/// labelled as the host-clock figures they are.
#[test]
fn frame_times_across_a_swap_are_measured_and_reported() {
    let (capacity, size) = REAL;
    // Both Sets at the same capacity, so that "before" and "after" are
    // measurements of the same workload and the only difference between them
    // is that a swap happened in between. Two capacities would confound the
    // question being asked.
    let (mut h, tx) = Harness::channel_driven_at(GENEROUS_MS, capacity, size);

    // Discarded, for the same reason the watchdog discards its own first
    // frames: the process's first frames pay for pipeline first-use, first
    // touch of the element buffers, and whatever the GPU's clocks were doing
    // before there was work. Leaving them in makes the "before" window read
    // slower than the "after" one and invites the conclusion that swapping
    // made things faster.
    for _ in 0..60 {
        h.frame();
    }
    h.intervals.clear();

    for _ in 0..120 {
        h.frame();
    }
    let before = h.intervals.len();

    tx.send(request(L4, capacity, "second")).expect("worker alive");
    let in_flight = h.frames_until(is_swapped, "the swap").0;
    // One more frame before the slice indices are taken. `Harness::frame`
    // pushes an interval at the *top* of a frame, so when the `Swapped` event
    // is first seen the swap frame has begun but not ended and its interval is
    // not in the vector yet — the last entry is the frame before it. Rendering
    // one more frame is what puts the swap frame's own interval at the end.
    h.frame();
    let at_swap = h.intervals.len();

    for _ in 0..120 {
        h.frame();
    }

    let summarize = |label: &str, xs: &[f32]| {
        let mut sorted = xs.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
        let median = sorted[sorted.len() / 2];
        let worst = sorted[sorted.len() - 1];
        eprintln!(
            "  {label:<30} n={:<4} median {median:.3} ms   worst {worst:.3} ms",
            sorted.len()
        );
    };

    eprintln!(
        "\nframe intervals at capacity {capacity}, {}x{}, host clock around \
         submit-and-wait; {in_flight} frames rendered between the request and the swap:",
        h.size.0, h.size.1
    );
    summarize("steady, before the request", &h.intervals[..before]);
    summarize("while the build was in flight", &h.intervals[before..at_swap - 1]);
    // The frame the swap landed on gets its own line: it is the one frame that
    // could plausibly cost something, since it is where the live Set is
    // replaced and where the incoming pipelines are used for the first time.
    summarize("the swap frame itself", &h.intervals[at_swap - 1..at_swap]);
    summarize("steady, after the swap", &h.intervals[at_swap..]);
    eprintln!();
}
