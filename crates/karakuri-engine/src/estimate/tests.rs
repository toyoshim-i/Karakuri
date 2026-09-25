use super::*;

/// A target size to fit against, and a fixture rather than a reference — see
/// [`PREPARATION_RESOLUTION`]. It was `swap::AT` until ADR-0303 removed that
/// constant.
const AT: (u32, u32) = (1280, 720);

fn measured(ms: f32, resolution: (u32, u32)) -> Measurement {
    Measurement {
        ms,
        method: MeasurementMethod::HostWallClock,
        capacity: 262_144,
        resolution,
    }
}

/// Verifies that linear fitting over two rungs accurately recovers known invariant and fragment terms.
#[test]
fn two_rungs_recover_the_a_and_b_they_were_built_from() {
    let a = 9.0_f64;
    let b = 9.2 / area(AT);
    let (low, high) = ((320, 180), (640, 360));
    let e = fit(
        measured((a + b * area(low)) as f32, low),
        measured((a + b * area(high)) as f32, high),
        AT,
        180,
        vec![Topology::Points],
    );

    let f = e.fit.expect("two clean rungs fit");
    assert!(
        (f.invariant_ms - 9.0).abs() < 1e-2,
        "a came out {}",
        f.invariant_ms
    );
    assert!(
        (f.fragment_ms - 9.2).abs() < 1e-2,
        "b*A came out {}",
        f.fragment_ms
    );
    assert!((f.ms - 18.2).abs() < 2e-2, "the answer came out {}", f.ms);
    assert_eq!(e.floor, Some(180));
    assert_eq!(e.target, AT);
}

/// The rungs may arrive in either order, and the [`Estimate`] records them low
/// area first whichever way they came.
#[test]
fn the_rungs_are_sorted_by_area_rather_than_trusted() {
    let (low, high) = ((320, 180), (640, 360));
    let e = fit(
        measured(4.0, high),
        measured(2.0, low),
        AT,
        180,
        vec![Topology::Fullscreen],
    );
    let [first, second] = e.rungs.expect("both rungs recorded");
    assert_eq!(first.resolution, low);
    assert_eq!(second.resolution, high);
    assert!(e.fit.is_ok());
}

/// Verifies that flat measurements across rungs do not multiply fragment cost by area ratio.
#[test]
fn a_flat_pair_is_no_longer_multiplied_by_the_area_ratio() {
    let e = fit(
        measured(8.980, (320, 180)),
        measured(8.996, (640, 360)),
        AT,
        180,
        vec![Topology::Points],
    );
    let f = e.fit.expect("a flat pair is still a fit");
    assert!(
        f.ms < 12.0,
        "a frame that did not move with the target estimated at {:.2} ms, and the \
         area-ratio rule this replaced said 36",
        f.ms
    );
    assert!(
        f.fragment_ms < f.invariant_ms,
        "the fragment term is {:.2} ms against an invariant {:.2} ms, which is not \
         what a flat pair means",
        f.fragment_ms,
        f.invariant_ms
    );
}

/// Verifies refusal when execution cost increases as resolution decreases (negative fragment slope).
#[test]
fn a_pair_that_got_dearer_as_it_got_smaller_is_refused() {
    let e = fit(
        measured(10.17, (320, 180)),
        measured(9.08, (452, 254)),
        AT,
        180,
        vec![Topology::Points],
    );
    match e.fit {
        Err(Unfit::FragmentTermNegative { ms_per_pixel }) => {
            assert!(ms_per_pixel < 0.0, "{ms_per_pixel} is not negative")
        }
        other => panic!("expected a negative fragment term, got {other:?}"),
    }
    assert_eq!(e.ms(), None, "a refusal hands out no number");
}

/// A negative `a` is the same failure at the other end. Two rungs whose line
/// reaches zero area below zero milliseconds did not measure one curve: the
/// simulation and the vertex stage cannot cost less than nothing.
#[test]
fn a_pair_whose_invariant_term_is_below_zero_is_refused() {
    // A slope steep enough that extrapolating back to zero area goes
    // negative: 1 ms at a sixteenth of the area and 8 ms at a quarter.
    let e = fit(
        measured(1.0, (320, 180)),
        measured(8.0, (640, 360)),
        AT,
        180,
        vec![Topology::Fullscreen],
    );
    match e.fit {
        Err(Unfit::InvariantTermNegative { invariant_ms }) => {
            assert!(invariant_ms < 0.0, "{invariant_ms} is not negative")
        }
        other => panic!("expected a negative invariant term, got {other:?}"),
    }
}

/// Verifies refusal when probe rungs would hide more fragment cost than the allowed threshold.
#[test]
fn a_pair_that_would_hide_too_much_of_the_frame_is_refused() {
    let e = fit(
        measured(4.0, (320, 180)),
        measured(6.0, (640, 360)),
        AT,
        // `speed_lines` at the shipped width: one pixel at 720 rows.
        720,
        vec![Topology::Lines],
    );
    match e.fit {
        Err(Unfit::FlooringHidesTooMuch { share, allowed }) => {
            assert!(
                (share - 0.75).abs() < 1e-9,
                "an upper rung at half the target hides 3/4, not {share}"
            );
            assert_eq!(allowed, FLOORED_SHARE_ALLOWED);
        }
        other => panic!("expected a hidden-share refusal, got {other:?}"),
    }
    assert_eq!(e.ms(), None, "a refusal hands out no number");
}

/// Verifies that a floor at target resolution adjusts the upper rung instead of refusing.
#[test]
fn a_floor_at_the_target_moves_the_upper_rung_rather_than_refusing() {
    let floor = sub_pixel_floor_rows(0.001_39).expect("a positive rate has a floor");
    assert_eq!(floor, 720);
    let [low, high] = rungs(AT, floor).expect("the upper rung moves");
    assert_eq!(low, (320, 180));
    assert_eq!(high, (1109, 624));
    let share = floored_share(low, high, AT, floor);
    assert!(
        share <= FLOORED_SHARE_ALLOWED,
        "{share} is over the allowance the rung was placed to meet"
    );
}

/// Verifies that all shipped procedure floors are placed within the hidden share allowance.
#[test]
fn every_shipped_floor_is_placed_and_none_hides_more_than_the_allowance() {
    for floor in [715, 720, 1450, 4141, u32::MAX] {
        let [low, high] = rungs(AT, floor).unwrap_or_else(|e| panic!("{floor} rows: {e:?}"));
        assert_eq!((low, high), ((320, 180), (1109, 624)), "a floor of {floor}");
        let share = floored_share(low, high, AT, floor);
        assert!(
            (0.249_1..=FLOORED_SHARE_ALLOWED).contains(&share),
            "a floor of {floor} rows hides {share}"
        );
    }
    // And the cheap pair is still what a floor under the quarter-height
    // rung gets, with nothing hidden at all.
    let [low, high] = rungs(AT, 180).expect("180 is the quarter-height rung");
    assert_eq!((low, high), ((320, 180), PREPARATION_RESOLUTION));
    assert_eq!(floored_share(low, high, AT, 180), 0.0);
}

/// The upper rung is where the allowance puts it, and no higher. `ceil` rather
/// than `round`, because rounding down would put the share over the allowance
/// the rung exists to meet.
#[test]
fn the_accurate_upper_rung_is_the_lowest_one_inside_the_allowance() {
    for rows in [8u32, 63, 100, 360, 720, 1080, 2160] {
        let high = upper_rung_rows(rows);
        let share = 1.0 - (f64::from(high) / f64::from(rows)).powi(2);
        assert!(share <= FLOORED_SHARE_ALLOWED, "{rows} rows hides {share}");
        let under = 1.0 - (f64::from(high - 1) / f64::from(rows)).powi(2);
        assert!(
            under > FLOORED_SHARE_ALLOWED,
            "{rows} rows would still be inside the allowance one row lower"
        );
    }
    assert_eq!(upper_rung_rows(720), 624);
}

/// A floor between the two rungs hides less than the peak, because no primitive
/// in the frame is small enough to reach it. That is the middle branch of
/// [`floored_share`], and it is what makes the bound tight rather than merely
/// sound.
#[test]
fn a_floor_the_frame_cannot_reach_hides_less_than_the_peak() {
    let (low, high) = ((320, 180), (1109, 624));
    let peak = floored_share(low, high, AT, u32::MAX);
    let near = floored_share(low, high, AT, 400);
    assert!(
        near < peak / 2.0,
        "a 400-row floor hides {near} against a worst case of {peak}"
    );
    // And a floor the cheap pair clears hides nothing whatever the rungs.
    assert_eq!(floored_share(low, high, AT, 180), 0.0);
}

/// Verifies that sub-pixel flooring correction bounds adversarial worst-case primitive sizes.
#[test]
fn the_correction_covers_the_worst_frame_the_flooring_can_build() {
    const COUNT: f64 = 262_144.0;
    // Milliseconds per covered pixel and invariant term for predominantly fragment workload.
    const PER_PIXEL: f64 = 2.0e-5;
    const INVARIANT: f64 = 0.5;

    let target = AT;
    let floor = u32::MAX;
    let [low, high] = rungs(target, floor).expect("the accurate pair");
    let rows = f64::from(target.1);
    // The worst primitive there is: one pixel across at the upper rung.
    let x = rows / f64::from(high.1);
    // ADR-0245: a primitive under a pixel is drawn at one and dimmed, so
    // its coverage stops falling.
    let coverage = |h: u32| {
        let across = x * f64::from(h) / rows;
        COUNT * across.max(1.0).powi(2)
    };
    let cost = |h: u32| INVARIANT + PER_PIXEL * coverage(h);

    let e = fit(
        measured(cost(low.1) as f32, low),
        measured(cost(high.1) as f32, high),
        target,
        floor,
        vec![Topology::Points],
    );
    let f = e.fit.expect("the accurate pair fits");
    let truth = cost(target.1);
    assert!(
        f64::from(f.ms) >= truth,
        "the corrected estimate is {:.3} ms against a truth of {truth:.3} ms, which is the \
         undershoot the correction exists to cover",
        f.ms
    );
    // And the raw fit is what it was rescued from: under-stating by very
    // nearly the whole share.
    let raw = f64::from(f.invariant_ms) + f64::from(f.fragment_ms);
    assert!(
        raw < truth,
        "the raw fit came out {raw:.3} ms and the truth is {truth:.3}, so this frame is not \
         the adversary it is written as"
    );
    let hid = 1.0 - raw / truth;
    let floored = e.floored.expect("a floored rung is on the record");
    assert!(
        hid <= floored.share + 1e-9,
        "the flooring hid {hid} where the bound says at most {}",
        floored.share
    );
    // **And the adversary is a real one**: it hides very nearly the whole
    // of what the bound allows, so the correction is being exercised at
    // its limit rather than against a frame that never needed it.
    assert!(
        hid > floored.share * 0.9,
        "this frame hid only {hid} of an allowed {}, so it is not the worst case",
        floored.share
    );
}

/// Verifies that sub-pixel flooring bounds hold for both stroke and sprite primitives.
#[test]
fn a_stroke_hides_less_than_a_sprite_and_both_are_inside_the_bound() {
    let target = AT;
    let [low, high] = rungs(target, u32::MAX).expect("the accurate pair");
    let rows = f64::from(target.1);
    let (p, q) = (rows / f64::from(low.1), rows / f64::from(high.1));
    let lambda = (area(target) - area(low)) / (area(high) - area(low));
    let bound = floored_share(low, high, target, u32::MAX);

    // A sprite `x` pixels across at the target: coverage `max((x/s)², 1)`
    // at the target divided by `s`, so the excess over the model is what
    // ADR-0245's rounding put there.
    let sprite = |x: f64| {
        let e = |s: f64| (1.0 - (x / s).powi(2)).max(0.0);
        e(1.0) + (lambda - 1.0) * e(p) - lambda * e(q)
    };
    // A stroke `y` pixels wide at the target, of unit length there: only
    // the width floors, so the coverage is `(1/s)·max(y/s, 1)`.
    let stroke = |y: f64| {
        let e = |s: f64| {
            let w = y / s;
            if w < 1.0 {
                (1.0 - w) / s
            } else {
                0.0
            }
        };
        e(1.0) + (lambda - 1.0) * e(p) - lambda * e(q)
    };

    let mut worst_sprite: f64 = 0.0;
    let mut worst_stroke: f64 = 0.0;
    for step in 1..=20_000u32 {
        let x = f64::from(step) / 2_000.0;
        worst_sprite = worst_sprite.max(sprite(x) / x.powi(2).max(1.0));
        worst_stroke = worst_stroke.max(stroke(x) / x.max(1.0));
    }
    assert!(
        worst_sprite <= bound + 1e-6,
        "a sprite hides {worst_sprite} where the bound says {bound}"
    );
    assert!(
        worst_stroke <= bound + 1e-6,
        "a stroke hides {worst_stroke} where the bound says {bound}"
    );
    assert!(
        worst_stroke < worst_sprite / 4.0,
        "a stroke floors in one dimension and should be far milder: \
         {worst_stroke} against {worst_sprite}"
    );
}

/// Verifies exact prediction when primitives are floored across both rungs and target.
#[test]
fn a_primitive_floored_at_the_target_too_is_predicted_exactly() {
    const COUNT: f64 = 262_144.0;
    const PER_PIXEL: f64 = 2.0e-7;
    const INVARIANT: f64 = 3.0;

    let target = AT;
    let floor = 4141;
    let [low, high] = rungs(target, floor).expect("the accurate pair");
    // 0.17 pixels across at the target, which is one pixel everywhere.
    let cost = |_h: u32| INVARIANT + PER_PIXEL * COUNT;
    let e = fit(
        measured(cost(low.1) as f32, low),
        measured(cost(high.1) as f32, high),
        target,
        floor,
        vec![Topology::Points],
    );
    let f = e.fit.expect("a flat pair fits");
    let raw = f64::from(f.invariant_ms) + f64::from(f.fragment_ms);
    assert!(
        (raw - cost(target.1)).abs() < 1e-3,
        "the raw fit came out {raw:.4} ms against a truth of {:.4}",
        cost(target.1)
    );
}

/// The rungs are half and a quarter of the target's height, at the target's
/// aspect ratio, and the upper one at the reference target is
/// [`PREPARATION_RESOLUTION`] — which is what makes that constant derived
/// rather than chosen.
#[test]
fn the_rungs_are_half_and_a_quarter_of_the_targets_height() {
    let [low, high] = rungs(AT, 1).expect("a fullscreen Set has no floor");
    assert_eq!(high, PREPARATION_RESOLUTION);
    assert_eq!(low, (320, 180));
    assert!(
        (area(high) / area(AT) - 0.25).abs() < 1e-9,
        "the upper rung is a quarter of the target's area"
    );
}

/// A floor between the two default rungs raises the lower one to it rather than
/// refusing the pair. A rung *at* the floor is above it in the sense that
/// matters: every primitive is still at least one pixel there.
#[test]
fn a_floor_between_the_rungs_raises_the_lower_one_to_it() {
    let [low, high] = rungs(AT, 250).expect("250 rows fits under 360");
    assert_eq!(low.1, 250);
    assert_eq!(high, PREPARATION_RESOLUTION);
    // And the aspect ratio is the target's, because the camera derives its
    // own from the viewport.
    assert_eq!(low, (444, 250));
}

/// The floor a rate implies, which is the language's arithmetic and not a
/// measurement: a primitive is `rate * height` pixels across, so it reaches one
/// pixel at `1 / rate` rows.
#[test]
fn a_rate_becomes_the_height_at_which_it_is_one_pixel() {
    assert_eq!(sub_pixel_floor_rows(0.001_39), Some(720));
    assert_eq!(sub_pixel_floor_rows(0.005_56), Some(180));
    assert_eq!(sub_pixel_floor_rows(0.000_69), Some(1450));
    assert_eq!(sub_pixel_floor_rows(0.0), None);
    assert_eq!(sub_pixel_floor_rows(f32::NAN), None);
}

fn bound(procedure: &str, bound: Bound) -> RateBound {
    RateBound {
        procedure: procedure.to_string(),
        bound,
    }
}

fn at_least(procedure: &str, rate: f32) -> RateBound {
    bound(
        procedure,
        Bound::AtLeast {
            rate,
            over: Vec::new(),
        },
    )
}

/// Verifies that procedures without geometry primitives floor at one row.
#[test]
fn a_set_that_draws_no_primitive_floors_at_one_row() {
    assert_eq!(floor_rows(&[bound("wash", Bound::NoPrimitive)]), Ok(1));
    assert_eq!(
        floor_rows(&[
            bound("wash", Bound::NoPrimitive),
            bound("haze", Bound::NoPrimitive)
        ]),
        Ok(1)
    );
    assert_eq!(floor_rows(&[]), Ok(1));
}

/// The greatest of the renderers' floors, not the first. The floor is the
/// height at which the *smallest* primitive in the frame is still a pixel
/// across, so the renderer drawing finest sets it for the whole Set — and a
/// fullscreen one beside it does not pull it back down.
#[test]
fn a_sets_floor_is_the_finest_renderers() {
    // 0.004 is one pixel at 250 rows and 0.00139 at 720.
    let floor = floor_rows(&[
        at_least("dots", 0.004),
        bound("wash", Bound::NoPrimitive),
        at_least("streaks", 0.001_39),
    ]);
    assert_eq!(floor, Ok(720));
}

/// One renderer nobody can bound refuses the whole Set. A floor that holds for
/// three renderers of four is not a floor: the fourth is drawing something no
/// rung can be certified above.
#[test]
fn one_unbounded_renderer_refuses_the_set() {
    let floor = floor_rows(&[
        at_least("dots", 0.004),
        bound(
            "bloom",
            Bound::Unbounded {
                at: karakuri_ir::Span::EMPTY,
                lower: 0.0,
            },
        ),
    ]);
    assert_eq!(floor, Err(Unfit::FloorUnknown));
}

/// Two draws at one size are one measurement taken twice, and `b` would be
/// `0/0`.
#[test]
fn two_rungs_at_one_size_are_refused() {
    let e = fit(
        measured(4.0, PREPARATION_RESOLUTION),
        measured(4.2, PREPARATION_RESOLUTION),
        AT,
        1,
        vec![Topology::Fullscreen],
    );
    assert!(matches!(e.fit, Err(Unfit::RungsCoincide { .. })));
}

/// Two instruments are not two rungs. `Probe::run` demotes itself to a host
/// clock for life on the first implausible sample, so one probe can hand back a
/// GPU number and then a host number — and the two are not comparable, which
/// the probe's own module doc is emphatic about.
#[test]
fn a_pair_taken_by_two_instruments_is_refused() {
    let mut high = measured(6.0, PREPARATION_RESOLUTION);
    high.method = MeasurementMethod::GpuTimestamp;
    let e = fit(
        measured(4.0, (320, 180)),
        high,
        AT,
        1,
        vec![Topology::Fullscreen],
    );
    assert_eq!(e.fit, Err(Unfit::InstrumentsDiffer));
}

/// The instrument travels with the number, and says which direction it is wrong
/// in. `P-0095` is what requires it of anything downstream of a measurement.
#[test]
fn an_estimate_says_which_clock_answered_and_that_it_reads_high() {
    let e = fit(
        measured(1.0, (320, 180)),
        measured(2.0, (640, 360)),
        AT,
        1,
        vec![Topology::Fullscreen],
    );
    assert_eq!(e.method(), Some(MeasurementMethod::HostWallClock));
    assert!(e.biased_high());
    assert_eq!(e.rungs.expect("recorded")[0].capacity, 262_144);
}

/// Verifies that an existing estimate can be re-evaluated for a different target resolution (ADR-0356).
#[test]
fn an_estimate_is_re_read_at_a_new_target_rather_than_dropped() {
    let a = 9.0_f64;
    let b = 9.2 / area(AT);
    let (low, high) = ((320, 180), (640, 360));
    let e = fit(
        measured((a + b * area(low)) as f32, low),
        measured((a + b * area(high)) as f32, high),
        AT,
        180,
        vec![Topology::Points],
    );
    let moved = e.at((640, 360));

    assert_eq!(moved.target, (640, 360));
    assert_eq!(moved.rungs, e.rungs, "the re-read drew again");
    assert_eq!(moved.floor, e.floor);
    assert_eq!(moved.floor_from, e.floor_from);
    let (was, now) = (e.fit.expect("fitted"), moved.fit.expect("re-read"));
    assert_eq!(now.ms_per_pixel, was.ms_per_pixel);
    assert!(
        (now.invariant_ms - was.invariant_ms).abs() < 1e-3,
        "the invariant term moved with the target: {} then {}",
        was.invariant_ms,
        now.invariant_ms
    );
    assert!(
        (now.fragment_ms - was.fragment_ms / 4.0).abs() < 1e-2,
        "a quarter of the area is not a quarter of the fragment term: {} then {}",
        was.fragment_ms,
        now.fragment_ms
    );
    assert!(
        (now.ms - (9.0 + 2.3)).abs() < 2e-2,
        "the answer at a quarter of the area came out {}",
        now.ms
    );
}

/// **A refusal taken before any draw keeps its refusal and records the new
/// target.** There are no rungs to re-read, so there is nothing a second
/// size can say that the first did not.
#[test]
fn a_refusal_with_no_rungs_is_re_targeted_and_stays_a_refusal() {
    // `speed_lines`' floor against a target with no room under it.
    let e = estimate_above_floor_refusal();
    assert_eq!(e.rungs, None);
    let moved = e.at((1280, 720));
    assert_eq!(moved.target, (1280, 720));
    assert_eq!(moved.fit, e.fit, "a refusal changed kind without drawing");
    assert_eq!(moved.rungs, None);
}

/// The placement refusal [`rungs`] hands back for a target with no room
/// under it, wrapped as the [`Estimate`] `estimate_above_floor` would
/// return without a device.
fn estimate_above_floor_refusal() -> Estimate {
    let (target, floor) = ((2, 2), 64);
    Estimate {
        target,
        topologies: vec![Topology::Points],
        floor: Some(floor),
        floor_from: Floor::Stated,
        floored: None,
        rungs: None,
        fit: Err(rungs(target, floor).expect_err("a 2x2 target holds no rungs")),
    }
}

/// A target smaller than either rung is answered rather than clamped: with the
/// two terms separated, only the fragment part shrinks, which is the prediction
/// rather than a rounding. The console's preview cell is 112x63.
#[test]
fn a_target_smaller_than_the_rungs_shrinks_only_the_fragment_term() {
    let e = fit(
        measured(2.0, (320, 180)),
        measured(3.0, (640, 360)),
        (112, 63),
        1,
        vec![Topology::Fullscreen],
    );
    let f = e.fit.expect("a clean pair");
    assert!(
        f.ms > f.invariant_ms && f.ms < 2.0,
        "{:.3} ms should sit between the invariant term {:.3} and the smaller rung",
        f.ms,
        f.invariant_ms
    );
}
