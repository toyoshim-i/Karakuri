use std::time::{Duration, Instant};

use super::common::*;

mod gpu {
    use super::*;

    #[allow(dead_code)]
    fn channel_driven() {
        let _ = Gpu::headless();
    }

    #[allow(dead_code)]
    fn channel_driven_at() {
        let _ = Gpu::headless();
    }

    #[allow(dead_code)]
    fn new() {
        let _ = Gpu::headless();
    }

    #[test]
    fn a_rebuild_can_add_a_renderer_over_the_same_geometry() {
        let (mut h, tx) = Harness::channel_driven(GENEROUS_MS);

        for _ in 0..5 {
            h.frame();
        }
        tx.send(request_many(&[L4, L4_WIDE], SECOND, "two renderers"))
            .expect("worker alive");
        // One wait, not two: the swap and its verdict are one drain (ADR-0313),
        // so asking for the verdict after the swap has been consumed waits for
        // an event that has already gone past.
        let (_, seen) = h.frames_until(|e| matches!(e, Event::Accepted { .. }), "the verdict");
        assert!(
            seen.iter().any(|s| s.contains("swapped in")),
            "the verdict arrived without the swap that produced it: {seen:?}"
        );

        let set = h.swap.set();
        assert_eq!(set.capacity(), SECOND, "the candidate was not kept");
        let mut exposures: Vec<f32> = set
            .params()
            .filter(|(_, _, name, _)| *name == "exposure")
            .map(|(_, _, _, value)| value)
            .collect();
        exposures.sort_by(|a, b| a.partial_cmp(b).expect("no NaN"));
        assert_eq!(
            exposures,
            vec![0.5, 1.0],
            "the swapped-in Set does not hold both renderers' `exposure`"
        );
    }

    /// Verifies that when multiple builds accumulate in the queue, only the newest build is installed.
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
        assert_eq!(h.swap.set().capacity(), SECOND, "`a` did not land");

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

    /// Verifies that worker crashes emit a single `Event::WorkerLost` event without disturbing the running Set.
    #[test]
    fn a_worker_that_dies_is_reported_once_and_does_not_disturb_the_live_set() {
        struct Exploding(u32);
        impl Source for Exploding {
            fn poll(&mut self) -> Option<Polled> {
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

        assert_eq!(
            lost, 1,
            "the dead worker was reported {lost} times, not once"
        );
        assert_eq!(h.swap.set().capacity(), FIRST, "the live Set was disturbed");
    }

    /// Verifies that source refusal diagnostics propagate to the render thread without altering running Set state (ADR-0310).
    #[test]
    fn a_source_that_refused_says_so_and_the_live_set_is_untouched() {
        /// One refusal and then nothing, which is a watcher over a file that
        /// was saved once and does not check.
        struct Refusing(bool);
        impl Source for Refusing {
            fn poll(&mut self) -> Option<Polled> {
                std::thread::sleep(karakuri_engine::swap::POLL_INTERVAL);
                if std::mem::replace(&mut self.0, false) {
                    return Some(Polled::Refused(Refusal {
                        label: "drift_shell.kir".to_owned(),
                        said: vec![
                            "3:5: parse: expected `}`".to_owned(),
                            "7:1: type: unknown builtin `curl2`".to_owned(),
                        ],
                    }));
                }
                None
            }
        }

        let mut h = Harness::new(
            GENEROUS_MS,
            FIRST,
            (WIDTH, HEIGHT),
            Box::new(Refusing(true)),
        );
        let started = Instant::now();
        let mut said: Vec<(String, Vec<String>)> = Vec::new();
        let mut others = 0;
        while started.elapsed() < PATIENCE {
            h.frame();
            for event in h.swap.events() {
                match event {
                    Event::SourceRefused { label, said: lines } => {
                        said.push((label.to_string(), lines))
                    }
                    _ => others += 1,
                }
            }
            if !said.is_empty() && started.elapsed() > Duration::from_millis(500) {
                break;
            }
        }

        assert_eq!(
            said.len(),
            1,
            "one refusal was polled and {} reached the render thread",
            said.len()
        );
        assert_eq!(
            others, 0,
            "a refusal produced {others} other events, and nothing was built"
        );
        assert_eq!(said[0].0, "drift_shell.kir", "the refusal lost its file");
        assert_eq!(
            said[0].1,
            vec![
                "3:5: parse: expected `}`",
                "7:1: type: unknown builtin `curl2`"
            ],
            "the diagnostics did not survive the channel"
        );

        // Rejected builds leave incumbent slot state unchanged.
        // that had reached `install_if_ready` as a candidate would be a swap.
        assert_eq!(
            h.swap.set().capacity(),
            FIRST,
            "the live Set was replaced by a build that never happened"
        );

        // And the whole of it goes to a terminal and to a model, which is the
        // one-round-trip half of `docs/principles/0083-…`: the row draws the
        // first line and this carries them all.
        let printed = Event::SourceRefused {
            label: said[0].0.as_str().into(),
            said: said[0].1.clone(),
        }
        .to_string();
        for line in &said[0].1 {
            assert!(
                printed.contains(line),
                "the printed refusal drops `{line}`: {printed}"
            );
        }
    }

    /// Measures and logs wall-clock frame intervals before, during, and after a hot-swap.
    #[test]
    fn frame_times_across_a_swap_are_measured_and_reported() {
        let (capacity, size) = REAL;
        // Both Sets at the same capacity, so that "before" and "after" are
        // measurements of the same workload and the only difference between them
        // is that a swap happened in between. Two capacities would confound the
        // question being asked.
        let (mut h, tx) = Harness::channel_driven_at(GENEROUS_MS, capacity, size);

        // Warm up pipeline to eliminate first-use overhead before recording baseline intervals.
        for _ in 0..60 {
            h.frame();
        }
        h.intervals.clear();

        for _ in 0..120 {
            h.frame();
        }
        let before = h.intervals.len();

        tx.send(request(L4, capacity, "second"))
            .expect("worker alive");
        let in_flight = h.frames_until(is_swapped, "the swap").0;
        // Render an additional frame so the swap frame's interval is captured in history.
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
        summarize(
            "while the build was in flight",
            &h.intervals[before..at_swap - 1],
        );
        // The frame the swap landed on gets its own line: it is the one frame that
        // could plausibly cost something, since it is where the live Set is
        // replaced and where the incoming pipelines are used for the first time.
        summarize("the swap frame itself", &h.intervals[at_swap - 1..at_swap]);
        summarize("steady, after the swap", &h.intervals[at_swap..]);
        eprintln!();
    }
}
