use std::cell::Cell;
use std::rc::Rc;

use super::*;

/// Verifies that frame commit callbacks execute even when all sinks refuse presentation.
#[test]
fn a_refused_frame_still_reaches_the_committing_work() {
    let gpu = Gpu::headless().expect("no GPU");
    let mut deck = one_slot_deck(&gpu);
    let present = Present::new(&gpu.device, FORMAT, SIZE, SIZE);
    let mut sink = TestSink::new(
        &gpu,
        vec![
            Err(Skip::Transient),
            Err(Skip::Fault("gone".into())),
            Ok(()),
        ],
    );
    let mut commits = 0;
    let mut refusals: Vec<(usize, Skip)> = Vec::new();

    for at in 0..3 {
        let outcome = {
            let mut sinks = one(&mut sink);
            compose(
                &gpu,
                &mut deck,
                &present,
                &mut sinks,
                &mut |sink_at, skip| refusals.push((sink_at, skip)),
                |_| {
                    commits += 1;
                    Committed {
                        steps: 1,
                        look: look(),
                    }
                },
                |_| {},
            )
            .expect("compose")
        };
        assert_eq!(
            commits,
            at + 1,
            "frame {at} did not commit: the committing closure is conditional again"
        );
        let reached = usize::from(at == 2);
        assert_eq!(
            outcome,
            Outcome {
                reached,
                missed: 1 - reached
            },
            "frame {at} counted its sinks wrong"
        );
    }
    assert_eq!(sink.presented, 1, "only the accepted frame was presented");
    assert_eq!(
        refusals,
        vec![(0, Skip::Transient), (0, Skip::Fault("gone".into()))],
        "a refusal is reported once, with its sink's index and its reason"
    );
}

/// Verifies that simulation time advances identically regardless of sink refusal.
#[test]
fn a_refused_frame_still_advances_the_simulation() {
    let gpu = Gpu::headless().expect("no GPU");
    let mut deck = one_slot_deck(&gpu);
    let present = Present::new(&gpu.device, FORMAT, SIZE, SIZE);
    let mut sink = TestSink::new(&gpu, vec![Err(Skip::Transient)]);

    let before = deck.slot(DeckSlot(0)).set().time();
    {
        let mut sinks = one(&mut sink);
        compose(
            &gpu,
            &mut deck,
            &present,
            &mut sinks,
            &mut |_, _| {},
            |_| Committed {
                steps: 4,
                look: look(),
            },
            |_| {},
        )
        .expect("compose");
    }
    let refused = deck.slot(DeckSlot(0)).set().time() - before;
    assert!(
        refused > 0.0,
        "a frame with nowhere to draw did not step the simulation"
    );

    // The accepted frame is the control: without it, "it moved" could
    // be a deck that moves for some other reason, and the equality
    // below is what says a sink has no say in how far it moves.
    let mid = deck.slot(DeckSlot(0)).set().time();
    {
        let mut sinks = one(&mut sink);
        compose(
            &gpu,
            &mut deck,
            &present,
            &mut sinks,
            &mut |_, _| {},
            |_| Committed {
                steps: 4,
                look: look(),
            },
            |_| {},
        )
        .expect("compose");
    }
    let accepted = deck.slot(DeckSlot(0)).set().time() - mid;
    assert!(
        (accepted - refused).abs() < 1e-6,
        "a refused frame advanced by {refused} and an accepted one by {accepted}"
    );
    assert_eq!(sink.presented, 1, "and only the accepted one was presented");
}

/// Verifies that composing with zero sinks still commits and advances the simulation.
#[test]
fn a_frame_with_no_sinks_at_all_still_advances_the_deck() {
    let gpu = Gpu::headless().expect("no GPU");
    let mut deck = one_slot_deck(&gpu);
    let present = Present::new(&gpu.device, FORMAT, SIZE, SIZE);
    let mut none: [&mut dyn Sink; 0] = [];
    let mut commits = 0;
    let mut refusals = 0;

    let before = deck.slot(DeckSlot(0)).set().time();
    let outcome = compose(
        &gpu,
        &mut deck,
        &present,
        &mut none,
        &mut |_, _| refusals += 1,
        |_| {
            commits += 1;
            Committed {
                steps: 4,
                look: look(),
            }
        },
        |_| {},
    )
    .expect("a frame that publishes nowhere is not an error");

    assert_eq!(commits, 1, "the frame did not commit");
    assert!(
        deck.slot(DeckSlot(0)).set().time() > before,
        "the deck stopped when the outputs went off"
    );
    assert_eq!(
        outcome,
        Outcome {
            reached: 0,
            missed: 0
        },
        "nothing was asked, so nothing was reached and nothing missed"
    );
    assert_eq!(refusals, 0, "nothing refused, because nothing was asked");
}

/// Verifies that sink refusals are reported by index without affecting accepted sinks.
#[test]
fn a_sink_that_refuses_costs_the_others_nothing() {
    let gpu = Gpu::headless().expect("no GPU");
    let mut deck = one_slot_deck(&gpu);
    let present = Present::new(&gpu.device, FORMAT, SIZE, SIZE);
    let mut first = TestSink::new(&gpu, vec![Err(Skip::Transient)]);
    let mut taking = TestSink::new(&gpu, vec![]);
    let mut last = TestSink::new(&gpu, vec![Err(Skip::Fault("wedged".into()))]);
    let mut refusals: Vec<(usize, Skip)> = Vec::new();

    let outcome = {
        let mut sinks: [&mut dyn Sink; 3] = [&mut first, &mut taking, &mut last];
        compose(
            &gpu,
            &mut deck,
            &present,
            &mut sinks,
            &mut |at, skip| refusals.push((at, skip)),
            |_| Committed {
                steps: 1,
                look: look(),
            },
            |_| {},
        )
        .expect("compose")
    };

    assert_eq!(
        outcome,
        Outcome {
            reached: 1,
            missed: 2
        }
    );
    assert_eq!(
        refusals,
        vec![(0, Skip::Transient), (2, Skip::Fault("wedged".into()))],
        "each refusal is reported once, at the sink's own place in the slice"
    );
    for (which, sink) in [("the first", &first), ("the last", &last)] {
        assert!(
            sink.untouched_after_refusing(),
            "{which} sink refused and was then drawn into: {} view, {} after_draw, \
                     {} present",
            sink.viewed.get(),
            sink.after_drawn,
            sink.presented
        );
    }
    assert_eq!(
        taking.viewed.get(),
        1,
        "the sink that answered was not drawn"
    );
    assert_eq!(taking.after_drawn, 1, "its `after_draw` did not run");
    assert_eq!(taking.presented, 1, "it was not presented");
    assert_eq!(
        taking.lit,
        Some(true),
        "it was presented and there was nothing in it"
    );
}

/// Verifies that multiple sinks with different resolutions render letterboxed correctly.
#[test]
fn two_sinks_of_different_sizes_both_get_the_canvas() {
    let gpu = Gpu::headless().expect("no GPU");
    let mut deck = one_slot_deck(&gpu);
    let present = Present::new(&gpu.device, FORMAT, SIZE, SIZE);
    let mut square = TestSink::new(&gpu, vec![]);
    let mut wide = TestSink::sized(&gpu, WIDE, SIZE, vec![]);

    let outcome = {
        let mut sinks: [&mut dyn Sink; 2] = [&mut square, &mut wide];
        compose(
            &gpu,
            &mut deck,
            &present,
            &mut sinks,
            &mut |_, _| {},
            |_| Committed {
                steps: 1,
                look: look(),
            },
            |_| {},
        )
        .expect("compose")
    };

    assert_eq!(
        outcome,
        Outcome {
            reached: 2,
            missed: 0
        }
    );
    assert_eq!(square.lit, Some(true), "the fixture drew nothing to fit");
    assert!(
        wide.lit_between(BAND.0, BAND.1),
        "the wide sink got no canvas: the second sink was not drawn into"
    );
    assert!(
        !wide.lit_between(0, BAND.0) && !wide.lit_between(BAND.1, WIDE),
        "the wide sink has light outside the canvas's rectangle, so the canvas \
                 was fitted to a size that is not this sink's"
    );
}

/// Verifies that presentation failure on one sink does not prevent remaining sinks from presenting.
#[test]
fn a_present_that_fails_does_not_cost_the_next_sink_its_frame() {
    let gpu = Gpu::headless().expect("no GPU");
    let mut deck = one_slot_deck(&gpu);
    let present = Present::new(&gpu.device, FORMAT, SIZE, SIZE);
    let mut first = TestSink::new(&gpu, vec![]);
    first.present_answer = Some("the projector went".into());
    let mut taking = TestSink::new(&gpu, vec![]);
    let mut last = TestSink::new(&gpu, vec![]);
    last.present_answer = Some("and so did the recorder".into());

    let failure = {
        let mut sinks: [&mut dyn Sink; 3] = [&mut first, &mut taking, &mut last];
        compose(
            &gpu,
            &mut deck,
            &present,
            &mut sinks,
            &mut |_, _| {},
            |_| Committed {
                steps: 1,
                look: look(),
            },
            |_| {},
        )
        .expect_err("the failing sink's error")
    };

    assert_eq!(
        failure, "the projector went",
        "the error reported is not the first one"
    );
    assert_eq!(
        taking.presented, 1,
        "a sink after a failing one never had its frame presented"
    );
    assert_eq!(taking.lit, Some(true), "and it was presented empty");
    assert_eq!(
        last.presented, 1,
        "the last sink was never asked to present at all"
    );
}

/// Verifies that commit closure modifications take effect before frame rendering occurs.
#[test]
fn a_frame_commits_before_it_draws() {
    let gpu = Gpu::headless().expect("no GPU");
    let present = Present::new(&gpu.device, FORMAT, SIZE, SIZE);

    // The control, and it is not optional: if this material drew nothing at
    // this size the assertion below would hold for the wrong reason.
    let mut deck = one_slot_deck(&gpu);
    let mut sink = TestSink::new(&gpu, vec![]);
    {
        let mut sinks = one(&mut sink);
        compose(
            &gpu,
            &mut deck,
            &present,
            &mut sinks,
            &mut |_, _| {},
            |_| Committed {
                steps: 1,
                look: look(),
            },
            |_| {},
        )
        .expect("compose");
    }
    assert_eq!(sink.lit, Some(true), "the fixture drew nothing to darken");

    // The same frame, with the fader taken to zero inside the closure.
    // Opacity rather than gain because opacity silences a slot under every
    // blend mode and gain does not silence `over`.
    let mut deck = one_slot_deck(&gpu);
    let mut sink = TestSink::new(&gpu, vec![]);
    {
        let mut sinks = one(&mut sink);
        compose(
            &gpu,
            &mut deck,
            &present,
            &mut sinks,
            &mut |_, _| {},
            |deck| {
                deck.set_opacity(DeckSlot(0), 0.0);
                Committed {
                    steps: 1,
                    look: look(),
                }
            },
            |_| {},
        )
        .expect("compose");
    }
    assert_eq!(
        sink.lit,
        Some(false),
        "the frame was drawn before the closure that faded it"
    );
}

/// Verifies that `finally` callback executes after sink draws within the same command encoder.
#[test]
fn finally_is_recorded_after_every_sink_into_the_frames_own_encoder() {
    let gpu = Gpu::headless().expect("no GPU");
    let mut deck = one_slot_deck(&gpu);
    let present = Present::new(&gpu.device, FORMAT, SIZE, SIZE);
    let mut square = TestSink::new(&gpu, vec![]);
    let mut wide = TestSink::sized(&gpu, WIDE, SIZE, vec![]);
    let order = Rc::new(Cell::new(0));
    square.ordered_by(&order);
    wide.ordered_by(&order);

    // Textures retained directly to verify content post-draw while sinks remain borrowed.
    let sizes = [(SIZE, SIZE), (WIDE, SIZE)];
    let targets = [square.target.clone(), wide.target.clone()];
    let seen: Vec<wgpu::Buffer> = sizes
        .iter()
        .map(|(w, h)| {
            gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("what finally saw"),
                size: u64::from(w * h * 4),
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            })
        })
        .collect();

    let mut ran = 0usize;
    let mut finally_at = 0usize;
    let mut finally_encoder = 0usize;
    {
        let mut sinks: [&mut dyn Sink; 2] = [&mut square, &mut wide];
        compose(
            &gpu,
            &mut deck,
            &present,
            &mut sinks,
            &mut |_, _| {},
            |_| Committed {
                steps: 1,
                look: look(),
            },
            |encoder| {
                ran += 1;
                order.set(order.get() + 1);
                finally_at = order.get();
                finally_encoder = std::ptr::from_ref(&*encoder) as usize;
                for (at, target) in targets.iter().enumerate() {
                    let (width, height) = sizes[at];
                    encoder.copy_texture_to_buffer(
                        target.as_image_copy(),
                        wgpu::TexelCopyBufferInfo {
                            buffer: &seen[at],
                            layout: wgpu::TexelCopyBufferLayout {
                                offset: 0,
                                bytes_per_row: Some(width * 4),
                                rows_per_image: Some(height),
                            },
                        },
                        wgpu::Extent3d {
                            width,
                            height,
                            depth_or_array_layers: 1,
                        },
                    );
                }
            },
        )
        .expect("compose");
    }

    assert_eq!(ran, 1, "`finally` did not run at all");
    assert_eq!(square.after_drawn, 1, "the square sink was not drawn into");
    assert_eq!(wide.after_drawn, 1, "the wide sink was not drawn into");
    assert!(
        finally_at > square.drawn_at && finally_at > wide.drawn_at,
        "`finally` was called at tick {finally_at}, and the sinks were drawn into at \
                 {} and {} — so it ran before a sink that took the frame",
        square.drawn_at,
        wide.drawn_at
    );
    assert_ne!(finally_encoder, 0, "`finally` never saw an encoder");
    assert_eq!(
        (finally_encoder, finally_encoder),
        (square.encoder_at, wide.encoder_at),
        "`finally` was handed an encoder that is not the one the sinks were drawn \
                 into, so the panel's pass would be a second command buffer over a texture \
                 the first is still writing"
    );

    // And what it recorded is in the frame's submission, downstream of
    // the present passes: the canvas is already in both textures, and
    // in the wide one it is in that sink's own band rather than
    // anybody else's.
    let read = |at: usize| {
        let slice = seen[at].slice(..);
        slice.map_async(wgpu::MapMode::Read, |r| r.expect("map"));
        gpu.device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("poll");
        let texels = slice.get_mapped_range().expect("map").to_vec();
        seen[at].unmap();
        texels
    };
    let lit_between = |texels: &[u8], width: u32, from: u32, to: u32| {
        (0..SIZE).any(|y| {
            (from..to).any(|x| {
                let i = ((y * width + x) * 4) as usize;
                texels[i..i + 3].iter().any(|&b| b > 0)
            })
        })
    };
    let square_seen = read(0);
    assert!(
        lit_between(&square_seen, SIZE, 0, SIZE),
        "`finally` copied the square sink out and there was nothing in it, so it \
                 was recorded ahead of the present pass that draws it"
    );
    let wide_seen = read(1);
    assert!(
        lit_between(&wide_seen, WIDE, BAND.0, BAND.1),
        "`finally` copied the wide sink out and its band was empty"
    );
    assert!(
        !lit_between(&wide_seen, WIDE, 0, BAND.0) && !lit_between(&wide_seen, WIDE, BAND.1, WIDE),
        "the wide sink has light outside the canvas's rectangle"
    );
}

/// Verifies that `finally` callback executes and submits commands even when no sink accepts the frame.
#[test]
fn finally_runs_when_no_sink_took_the_frame() {
    const SENTINEL: [u8; 256] = [0xAA; 256];
    let gpu = Gpu::headless().expect("no GPU");
    let mut deck = one_slot_deck(&gpu);
    let present = Present::new(&gpu.device, FORMAT, SIZE, SIZE);
    let mark = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("what finally recorded"),
        size: SENTINEL.len() as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let cleared = |gpu: &Gpu, mark: &wgpu::Buffer| {
        let slice = mark.slice(..);
        slice.map_async(wgpu::MapMode::Read, |r| r.expect("map"));
        gpu.device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("poll");
        let read = slice.get_mapped_range().expect("map").to_vec();
        mark.unmap();
        read.iter().all(|&b| b == 0)
    };

    // No sink at all: the outputs are off, and the console's panel is
    // still on the operator's screen.
    gpu.queue.write_buffer(&mark, 0, &SENTINEL);
    let mut none: [&mut dyn Sink; 0] = [];
    let mut ran = 0usize;
    let outcome = compose(
        &gpu,
        &mut deck,
        &present,
        &mut none,
        &mut |_, _| {},
        |_| Committed {
            steps: 1,
            look: look(),
        },
        |encoder| {
            ran += 1;
            encoder.clear_buffer(&mark, 0, None);
        },
    )
    .expect("compose");
    assert_eq!(
        outcome,
        Outcome {
            reached: 0,
            missed: 0
        }
    );
    assert_eq!(ran, 1, "`finally` did not run on a frame with no sinks");
    assert!(
        cleared(&gpu, &mark),
        "`finally` ran on a frame with no sinks and what it recorded never \
                 reached the device — the encoder it was handed was not submitted"
    );

    // And a sink that refused, which is the same frame arrived at the
    // other way: something was asked and had no target.
    gpu.queue.write_buffer(&mark, 0, &SENTINEL);
    let mut sink = TestSink::new(&gpu, vec![Err(Skip::Transient)]);
    let mut ran = 0usize;
    let outcome = {
        let mut sinks = one(&mut sink);
        compose(
            &gpu,
            &mut deck,
            &present,
            &mut sinks,
            &mut |_, _| {},
            |_| Committed {
                steps: 1,
                look: look(),
            },
            |encoder| {
                ran += 1;
                encoder.clear_buffer(&mark, 0, None);
            },
        )
        .expect("compose")
    };
    assert_eq!(
        outcome,
        Outcome {
            reached: 0,
            missed: 1
        }
    );
    assert_eq!(
        ran, 1,
        "`finally` did not run on a frame every sink refused"
    );
    assert!(
        cleared(&gpu, &mark),
        "`finally` ran on a frame every sink refused and what it recorded never \
                 reached the device"
    );
    assert!(
        sink.untouched_after_refusing(),
        "the refusing sink was drawn into anyway"
    );
}

/// Verifies that master chain receives updated session clock uniforms and deck receives chain cost.
#[test]
fn a_composed_frame_hands_the_chain_its_clock_and_the_deck_its_price() {
    let gpu = Gpu::headless().expect("no GPU available");
    let mut deck = one_slot_deck(&gpu);
    let mut present = Present::new(&gpu.device, FORMAT, SIZE, SIZE);
    let chain = clock_chain(&gpu, &present);
    let ops = chain.ops_per_fragment();
    assert!(ops > 0, "the probe costs nothing at all");
    drop(present.set_chain(&gpu.device, &gpu.queue, chain));

    assert_eq!(
        present.chain_clock(),
        crate::Clock::default(),
        "a chain that has never been composed is not at zero"
    );
    assert_eq!(
        deck.chain_ops_per_fragment(),
        0,
        "a deck that has never been composed was already charged"
    );

    const STEPS: u8 = 2;
    let mut sink = TestSink::new(&gpu, vec![Ok(()), Ok(())]);
    let compose_one = |deck: &mut Deck, sink: &mut TestSink| {
        let mut sinks = one(sink);
        compose(
            &gpu,
            deck,
            &present,
            &mut sinks,
            &mut |_, _| {},
            |_| Committed {
                steps: STEPS,
                look: look(),
            },
            |_| {},
        )
        .expect("compose");
    };

    let first = deck.chain_clock(STEPS);
    compose_one(&mut deck, &mut sink);
    assert_eq!(
        present.chain_clock(),
        first,
        "the chain was not handed the clock this frame advanced to"
    );
    assert!(first.t > 0.0, "the first frame handed the chain t = 0");
    assert_eq!(
        first.dt,
        crate::set::DT,
        "`dt` is the fixed simulation step"
    );
    assert_eq!(
        deck.chain_ops_per_fragment(),
        ops,
        "the deck was not charged what the running chain costs"
    );
    assert!(
        deck.chain_ms() > 0.0,
        "a chain that costs ops priced at nothing"
    );
    assert!(
        deck.govern().chain_ms > 0.0,
        "the governor spent nothing on a running chain"
    );

    let second = deck.chain_clock(STEPS);
    assert!(
        second.t > first.t,
        "the session clock did not move across a frame: {} then {}",
        first.t,
        second.t
    );
    compose_one(&mut deck, &mut sink);
    assert_eq!(
        present.chain_clock(),
        second,
        "the second frame did not move the chain's clock"
    );
}
