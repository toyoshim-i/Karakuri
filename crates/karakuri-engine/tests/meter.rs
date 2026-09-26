//! Integration tests for per-slot audio/visual level metering.
//!
//! Asserts Rec.709 luminance weighting, mean/peak reduction, black frame floors,
//! and non-blocking polling across synthetic textures and live Deck Sets.

// Every test here takes a device, so the whole file is one `mod gpu` — the
// prefix `cargo test -- --skip gpu::` filters on. The convention, and the test
// that enforces it, are in `tests/gpu_tests_are_under_mod_gpu.rs`.
#[path = "common/mod.rs"]
mod common;

#[path = "meter/fixtures.rs"]
mod fixtures;

mod gpu {
    use std::sync::mpsc;
    use std::time::Instant;

    use super::common::compile;
    use super::fixtures::*;

    use karakuri_engine::deck::{Deck, Residency};
    use karakuri_engine::swap::{Event, HotSwap, Request};
    use karakuri_engine::{Gpu, Present};

    /// Verifies that black frames measure exactly zero and brightness scales monotonically.

    #[test]
    fn a_black_frame_measures_zero_and_a_brighter_frame_measures_higher() {
        let gpu = Gpu::headless().expect("no GPU available");

        let (_black, black_view) = image(&gpu, 64, 64, |_, _| [0.0, 0.0, 0.0]);
        let (_dim, dim_view) = image(&gpu, 64, 64, |_, _| [0.25, 0.25, 0.25]);
        let (_bright, bright_view) = image(&gpu, 64, 64, |_, _| [2.0, 2.0, 2.0]);

        let black = measure(&gpu, &black_view);
        let dim = measure(&gpu, &dim_view);
        let bright = measure(&gpu, &bright_view);

        assert_eq!(black.mean, 0.0, "a black frame measured light");
        assert_eq!(black.peak, 0.0, "a black frame has a peak");

        // A grey of `v` has luminance `v` exactly: the weights sum to 1.0.
        let close = |x: f32, y: f32| (x - y).abs() <= 1e-4 * (1.0 + y.abs());
        assert!(close(dim.mean, 0.25), "0.25 grey measured {}", dim.mean);
        assert!(close(dim.peak, 0.25), "0.25 grey peaked at {}", dim.peak);
        assert!(close(bright.mean, 2.0), "2.0 grey measured {}", bright.mean);
        // Above 1.0, which is the half of the range the pipeline is HDR for.
        assert!(bright.peak > 1.0, "the bright frame did not exceed 1.0");
        assert!(bright.mean > dim.mean && dim.mean > black.mean);
    }

    /// Verifies that non-finite texels (NaN, Inf) are counted in bad_texels and excluded from mean/peak.
    #[test]
    fn a_texel_that_is_not_a_number_is_counted_and_left_out_rather_than_spreading() {
        let gpu = Gpu::headless().expect("no GPU available");
        const W: u32 = 64;
        const H: u32 = 64;
        const WASH: f32 = 0.5;
        // Three texels, one per spelling, all in the first row so the arithmetic
        // below does not depend on which workgroup they land in.
        const BAD: [f32; 3] = [f32::NAN, f32::INFINITY, f32::NEG_INFINITY];

        let (_tex, view) = image(&gpu, W, H, |x, y| {
            if y == 0 && (x as usize) < BAD.len() {
                let v = BAD[x as usize];
                [v, v, v]
            } else {
                [WASH, WASH, WASH]
            }
        });
        let level = measure(&gpu, &view);

        assert_eq!(
            level.bad_texels,
            BAD.len() as u32,
            "the meter counted {} texels that were not light, not {}",
            level.bad_texels,
            BAD.len()
        );
        // A grey of `v` has luminance `v` exactly: the weights sum to 1.0. The bad
        // texels are out of the sum and still in the denominator.
        let expected = WASH * (W * H - BAD.len() as u32) as f32 / (W * H) as f32;
        let close = |x: f32, y: f32| (x - y).abs() <= 1e-4 * (1.0 + y.abs());
        assert!(
            close(level.mean, expected),
            "the mean is {} rather than {expected} — either a texel that is not light \
         reached the sum, or the ones that did not were taken out of the denominator \
         as well",
            level.mean
        );
        assert!(
            close(level.peak, WASH),
            "the peak is {} rather than {WASH}",
            level.peak
        );

        // And a frame with none of them reports none, so the count is measuring
        // the texels rather than being a constant.
        let (_clean, clean_view) = image(&gpu, W, H, |_, _| [WASH, WASH, WASH]);
        let clean = measure(&gpu, &clean_view);
        assert_eq!(
            clean.bad_texels, 0,
            "an ordinary frame was reported as having texels that were not light"
        );
        assert!(
            close(clean.mean, WASH),
            "the clean frame measured {}",
            clean.mean
        );
    }

    /// Verifies that frames containing solely non-finite texels report a zero peak rather than the initialization sentinel.
    #[test]
    fn a_frame_with_no_finite_texel_reports_no_peak_rather_than_the_sentinel() {
        let gpu = Gpu::headless().expect("no GPU available");
        const W: u32 = 64;
        const H: u32 = 64;

        let (_tex, view) = image(&gpu, W, H, |_, _| [f32::NAN, f32::NAN, f32::NAN]);
        let level = measure(&gpu, &view);

        assert_eq!(
            level.bad_texels,
            W * H,
            "the frame was supposed to be entirely unlit, so this asserts nothing"
        );
        assert_eq!(level.mean, 0.0, "a frame with no light measured light");
        assert_eq!(
            level.peak, 0.0,
            "the peak came back as {} — the sentinel it starts at, which formats as \
         41 characters in a status line",
            level.peak
        );
    }

    /// Verifies that peak and mean luminance metrics vary independently across high-contrast and diffuse patterns.
    #[test]
    fn peak_and_mean_move_independently() {
        let gpu = Gpu::headless().expect("no GPU available");

        // 64x64 = 4096 texels; the core is 8x8 = 64 of them, one 64th.
        let (_wash, wash_view) = image(&gpu, 64, 64, |_, _| [0.5, 0.5, 0.5]);
        let (_core, core_view) = image(&gpu, 64, 64, |x, y| {
            if x < 8 && y < 8 {
                [32.0, 32.0, 32.0]
            } else {
                [0.0, 0.0, 0.0]
            }
        });

        let wash = measure(&gpu, &wash_view);
        let core = measure(&gpu, &core_view);

        let close = |x: f32, y: f32| (x - y).abs() <= 1e-3 * (1.0 + y.abs());
        assert!(close(wash.mean, 0.5), "the wash measured {}", wash.mean);
        assert!(
            close(core.mean, 0.5),
            "the core frame measured {} where the wash measured {} — they were built to \
         agree, so the reduction is not a mean over the whole frame",
            core.mean,
            wash.mean
        );
        assert!(close(wash.peak, 0.5), "the wash peaked at {}", wash.peak);
        assert!(
            close(core.peak, 32.0),
            "the core peaked at {} rather than 32.0",
            core.peak
        );
        assert!(
            core.peak > 8.0 * wash.peak,
            "two frames of the same mean reported peaks within 8x of each other, so the \
         pair carries no more information than the mean alone"
        );
    }

    /// Verifies that RGB channels are weighted according to linear Rec.709 primaries.
    #[test]
    fn the_luminance_weights_are_rec_709() {
        let gpu = Gpu::headless().expect("no GPU available");

        let (_r, r_view) = image(&gpu, 32, 32, |_, _| [1.0, 0.0, 0.0]);
        let (_g, g_view) = image(&gpu, 32, 32, |_, _| [0.0, 1.0, 0.0]);
        let (_b, b_view) = image(&gpu, 32, 32, |_, _| [0.0, 0.0, 1.0]);

        let red = measure(&gpu, &r_view);
        let green = measure(&gpu, &g_view);
        let blue = measure(&gpu, &b_view);

        let close = |x: f32, y: f32| (x - y).abs() <= 1e-4;
        assert!(close(red.mean, R), "pure red measured {}", red.mean);
        assert!(close(green.mean, G), "pure green measured {}", green.mean);
        assert!(close(blue.mean, B), "pure blue measured {}", blue.mean);
        assert_ne!(
            green.mean, blue.mean,
            "green and blue of the same magnitude measured the same, which is what \
         averaging the channels would do"
        );
        assert!(
            green.mean > 9.0 * blue.mean,
            "green measured {} against blue's {}, a ratio of {:.2} rather than Rec.709's \
         {:.2}",
            green.mean,
            blue.mean,
            green.mean / blue.mean,
            G / B
        );
    }

    /// Verifies that compute reduction correctly handles ragged image tails not evenly divided by workgroup size.
    #[test]
    fn the_reduction_covers_the_tail_the_workgroups_do_not_divide() {
        let gpu = Gpu::headless().expect("no GPU available");

        for &(w, h) in &[
            (1u32, 1u32), // one texel: 4095 threads with nothing to do
            (63, 65),     // 4095: one short of a full pass
            (65, 65),     // 4225: a tail of 129
            (101, 97),    // 9797: two passes and a ragged tail
            (257, 129),   // 33153
            (4097, 1),    // one very wide row
        ] {
            // 0.25 everywhere but the last texel, which is 8.0 — so a peak that
            // stops at the last whole stride reports 0.25.
            let texel = |x: u32, y: u32| {
                let v = if (x, y) == (w - 1, h - 1) { 8.0 } else { 0.25 };
                [v, v, v]
            };
            let (_image, view) = image(&gpu, w, h, texel);
            let measured = measure(&gpu, &view);

            // Closed form, from the same numbers the image was built from: a grey
            // of `v` has luminance `v`, since the weights sum to 1.0.
            let texels = f64::from(w) * f64::from(h);
            let expected_mean = ((0.25 * (texels - 1.0) + 8.0) / texels) as f32;
            assert!(
                (measured.mean - expected_mean).abs() <= 1e-4 * (1.0 + expected_mean),
                "{w}x{h} ({} texels) measured a mean of {} rather than {expected_mean}",
                w * h,
                measured.mean
            );
            assert!(
                (measured.peak - 8.0).abs() <= 1e-3,
                "{w}x{h}: the brightest texel is the last one and the peak came back as \
             {}, so the tail the workgroups do not evenly cover was not looked at",
                measured.peak
            );
        }
    }

    /// Verifies that frames with all-negative color values report a negative peak without artificial zero-clamping.
    #[test]
    fn a_negative_frame_reports_a_negative_peak() {
        let gpu = Gpu::headless().expect("no GPU available");
        let (_image, view) = image(&gpu, 64, 64, |_, _| [-0.5, -0.5, -0.5]);
        let measured = measure(&gpu, &view);

        let close = |x: f32, y: f32| (x - y).abs() <= 1e-4 * (1.0 + y.abs());
        assert!(
            close(measured.mean, -0.5),
            "a -0.5 frame measured {}",
            measured.mean
        );
        assert!(
            close(measured.peak, -0.5),
            "a frame that is -0.5 everywhere reported a peak of {}, so the peak is \
         floored at zero and a negative Set reads as one that merely touches black",
            measured.peak
        );
    }

    /// Verifies that Deck slot exposure scales measured levels monotonically and zero exposure reads exactly zero.
    #[test]
    fn a_brighter_set_measures_higher_than_a_dimmer_one() {
        let gpu = Gpu::headless().expect("no GPU available");
        let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
        let mut deck = metered_deck(&gpu, &[1.0, 0.25, 0.0]);

        let (bright, _) = wait_for_level(&gpu, &mut deck, &present, 0);
        let dim = deck
            .level(karakuri_engine::DeckSlot(1))
            .expect("every slot is metered from the same frame");
        let dark = deck
            .level(karakuri_engine::DeckSlot(2))
            .expect("every slot is metered from the same frame");

        assert!(
            bright.mean > 0.0,
            "the bright slot measured {}, so this test is comparing nothing",
            bright.mean
        );
        assert!(
            dim.mean > 0.0 && bright.mean > 2.0 * dim.mean,
            "a Set at exposure 1.0 measured {} against 0.25's {}, which is not the four \
         times more light it is putting out",
            bright.mean,
            dim.mean
        );
        assert!(
            bright.peak > dim.peak,
            "the brighter Set's peak ({}) did not exceed the dimmer one's ({})",
            bright.peak,
            dim.peak
        );
        assert_eq!(dark.mean, 0.0, "a Set drawing black measured light");
        assert_eq!(dark.peak, 0.0, "a Set drawing black has a peak");
    }

    /// Verifies that non-finite fragment outputs are contained to their slot's meter without poisoning adjacent slots.
    #[test]
    fn a_nan_from_a_real_l4_is_counted_and_leaves_the_mean_a_number() {
        let gpu = Gpu::headless().expect("no GPU available");
        let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
        let mut deck = Deck::new(
            &gpu.device,
            vec![
                HotSwap::fixed(build_l4(&gpu, L4_NAN)),
                HotSwap::fixed(build(&gpu, 1.0)),
            ],
            WIDTH,
            HEIGHT,
        );
        deck.enable_meters(&gpu.device);

        let (bad, _) = wait_for_level(&gpu, &mut deck, &present, 0);
        let good = deck
            .level(karakuri_engine::DeckSlot(1))
            .expect("every slot is metered from the same frame");

        assert!(
            bad.bad_texels > 100,
            "the NaN Set produced only {} texels that were not light, so this test is \
         asserting nothing",
            bad.bad_texels
        );
        assert!(
            bad.mean.is_finite() && bad.peak.is_finite(),
            "the NaN Set measured mean {} peak {}, so one texel of it still takes the \
         whole reading",
            bad.mean,
            bad.peak
        );
        assert_eq!(
            good.bad_texels, 0,
            "the ordinary Set was reported as having {} texels that were not light, so a \
         NaN crossed between slots",
            good.bad_texels
        );
        assert!(
            good.mean > 0.0,
            "the ordinary Set measured {}, so it drew nothing and says nothing",
            good.mean
        );
    }

    /// Verifies that slots transitioned to Allocated residency clear their meter readings without stale resurrection.
    #[test]
    fn an_allocated_slot_reports_no_level() {
        let gpu = Gpu::headless().expect("no GPU available");
        let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
        let mut deck = metered_deck(&gpu, &[1.0, 1.0]);

        let (live, _) = wait_for_level(&gpu, &mut deck, &present, 1);
        assert!(live.mean > 0.0, "the slot measured nothing while Live");

        deck.set_residency(karakuri_engine::DeckSlot(1), Residency::Allocated);
        assert_eq!(
            deck.level(karakuri_engine::DeckSlot(1)),
            None,
            "a slot taken off air kept the level it had while it was on"
        );
        // Long enough that every measurement outstanding at the moment it went off
        // air has had time to come back.
        for _ in 0..30 {
            frame(&gpu, &mut deck, &present);
            assert_eq!(
                deck.level(karakuri_engine::DeckSlot(1)),
                None,
                "a measurement recorded while the slot was Live arrived after it went off \
             air and became a level for a slot that is rendering nothing"
            );
        }
        // The other slot is unaffected: retiring is per slot.
        assert!(
            deck.level(karakuri_engine::DeckSlot(0)).is_some(),
            "retiring one slot retired another"
        );

        deck.set_residency(karakuri_engine::DeckSlot(1), Residency::Live);
        let (again, frames) = wait_for_level(&gpu, &mut deck, &present, 1);
        assert!(
            again.mean > 0.0,
            "the returning slot measured nothing, so the first half of this test would \
         pass on a meter that had stopped working"
        );
        assert!(
            frames > 1,
            "a level was available on the very first frame back on air, so it was not a \
         fresh measurement"
        );
    }

    /// Verifies that hot-swapping a Set on a slot retires in-flight measurements and reports fresh state.
    #[test]
    fn a_build_landing_on_a_slot_retires_its_meter() {
        let gpu = Gpu::headless().expect("no GPU available");
        let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);

        let (requests, source) = mpsc::channel::<Request>();
        // A budget no frame in this harness will come near: what is under test is
        // the swap landing, not the watchdog's opinion of it.
        let swap = HotSwap::new(
            &gpu.device,
            &gpu.queue,
            build(&gpu, 1.0),
            10_000.0,
            Box::new(source),
        );
        let mut deck = Deck::new(&gpu.device, vec![swap], WIDTH, HEIGHT);
        deck.enable_meters(&gpu.device);

        let (bright, _) = wait_for_level(&gpu, &mut deck, &present, 0);
        assert!(
            bright.mean > 0.0,
            "the Set measured nothing before the swap"
        );

        requests
            .send(Request {
                names: karakuri_engine::swap::RequestNames::default(),
                edges: Vec::new(),
                id: 1,
                l1s: vec![(compile(L1), CAPACITY)],
                l2s: Vec::new(),
                l3s: Vec::new(),
                fields: Vec::new(),
                layering: karakuri_engine::set::Layering::Overdraw,
                live: None,
                published: Vec::new(),
                l4s: vec![compile(&L4.replace("{{EXPOSURE}}", "0.000"))],
                seed_salt: SEED,
                camera: karakuri_engine::camera::Orbit::default(),
                salts: Vec::new(),
                params: Vec::new(),
                bindings: Vec::new(),
                authorities: Vec::new(),
                label: "black".to_string(),
            })
            .expect("the worker is alive");

        let mut landed = false;
        for _ in 0..PATIENCE {
            frame(&gpu, &mut deck, &present);
            if deck
                .events(karakuri_engine::DeckSlot(0))
                .any(|e| matches!(e, Event::Swapped { .. }))
            {
                landed = true;
                break;
            }
        }
        assert!(landed, "the build never landed in {PATIENCE} frames");

        // The frame that just ran was drawn entirely by the black Set. Anything
        // here is the previous Set's light, reported as this one's.
        assert_eq!(
            deck.level(karakuri_engine::DeckSlot(0)),
            None,
            "a build landed and the slot kept the outgoing Set's reading — the Set on air \
         draws black and the meter is reporting the one before it, which is a stale \
         number presented as a live one"
        );

        // The frame the swap landed on was itself drawn by the black Set, so the
        // next reading is that frame's and is due immediately. What makes it
        // demonstrably fresh is not when it arrives but what it says.
        let (fresh, _) = wait_for_level(&gpu, &mut deck, &present, 0);
        assert_eq!(
            fresh.mean, 0.0,
            "the reading that arrived after the swap measured {} for a Set that multiplies \
         its colour by zero, so it is still the outgoing Set's",
            fresh.mean
        );
    }

    /// Verifies that metering readbacks execute asynchronously without blocking the main render loop.
    #[test]
    fn the_meter_never_blocks_the_frame_path() {
        let gpu = Gpu::headless().expect("no GPU available");
        let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
        let mut deck = metered_deck(&gpu, &[1.0]);

        let mut first: Option<usize> = None;
        let mut lags: Vec<u32> = Vec::new();
        for n in 1..=PATIENCE {
            frame_without_waiting(&gpu, &mut deck, &present);
            if let Some(level) = deck.level(karakuri_engine::DeckSlot(0)) {
                first.get_or_insert(n);
                lags.push(level.frames_behind);
                assert!(
                    level.frames_behind >= 1,
                    "a reading claimed to be of the frame being recorded, which cannot \
                 happen without waiting for the GPU"
                );
                assert!(
                    level.mean > 0.0,
                    "the reading arrived but measured nothing, so an empty result would \
                 satisfy this test"
                );
            }
        }

        assert!(
            first.is_some(),
            "no level ever arrived in {PATIENCE} frames without a `poll(Wait)` to force \
         one, so nothing came back on its own"
        );

        let mut sorted = lags.clone();
        sorted.sort_unstable();
        let skipped = skipped(&deck);
        eprintln!(
            "\nmeter over {PATIENCE} unpaced frames at {WIDTH}x{HEIGHT}:\n  \
         frames behind: min {} median {} max {}\n  \
         a level on {} of {PATIENCE} frames, {skipped} measurements skipped\n",
            sorted[0],
            sorted[sorted.len() / 2],
            sorted[sorted.len() - 1],
            lags.len(),
        );

        assert!(
            sorted[sorted.len() - 1] > 1 || skipped > 0,
            "over {PATIENCE} frames with nothing pacing them, every reading came back \
         exactly one frame old and not one measurement was skipped — which is what a \
         loop paced by a wait looks like, and there is no wait in this test, so the \
         wait is inside the frame path"
        );
    }

    /// Verifies meter lag and ring buffer depth under simulated display pacing.
    #[test]
    fn the_lag_and_the_ring_are_measured_under_pacing() {
        const FRAMES: usize = 120;

        let gpu = Gpu::headless().expect("no GPU available");
        let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
        let mut deck = metered_deck(&gpu, &[1.0]);

        let mut first: Option<usize> = None;
        let mut lags: Vec<u32> = Vec::new();
        let started = Instant::now();
        for n in 1..=FRAMES {
            frame(&gpu, &mut deck, &present);
            if let Some(level) = deck.level(karakuri_engine::DeckSlot(0)) {
                first.get_or_insert(n);
                lags.push(level.frames_behind);
            }
        }
        let elapsed = started.elapsed();

        let first = first.expect("no level arrived at all");
        let mut sorted = lags.clone();
        sorted.sort_unstable();
        eprintln!(
            "\nmeter lag over {FRAMES} display-paced frames at {WIDTH}x{HEIGHT}:\n  \
         first level after {first} frames\n  \
         frames behind: min {} median {} max {}\n  \
         a level on {} of {FRAMES} frames, {} measurements skipped (ring of {})\n  \
         {FRAMES} frames in {:.1} ms\n",
            sorted[0],
            sorted[sorted.len() / 2],
            sorted[sorted.len() - 1],
            lags.len(),
            skipped(&deck),
            karakuri_engine::meter::RING,
            elapsed.as_secs_f32() * 1000.0,
        );

        // The ring is sized so that a paced frame never has to give up its
        // measurement. If this starts failing, the ring is too small for whatever
        // queue depth the driver is running at — a number to raise deliberately,
        // with this test's printed output as the evidence.
        assert_eq!(
            skipped(&deck),
            0,
            "the ring of {} staging buffers ran dry under pacing",
            karakuri_engine::meter::RING
        );
        // Every frame after the first reading has one, so a meter reads as a meter
        // rather than as something that updates now and then.
        assert_eq!(
            lags.len(),
            FRAMES - first + 1,
            "{} of the {} frames after the first reading had no level",
            FRAMES - first + 1 - lags.len(),
            FRAMES - first + 1
        );
        // Loose, and one-sided on purpose: the claim is "a few frames", and a
        // reading that was somehow instant would mean something waited.
        assert!(
            (1..=16).contains(&sorted[sorted.len() / 2]),
            "the median reading was {} frames behind, which is neither a lag of a few \
         frames nor a stall",
            sorted[sorted.len() / 2]
        );
    }
}
