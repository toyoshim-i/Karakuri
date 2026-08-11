//! One frame, and the one place it is composed.
//!
//! There were two frame loops: `Live::frame` drove the window and
//! `render::sequence_driven` drove a PNG, and the only difference that was ever
//! *meant* to exist between them is where the step count comes from — a live
//! run measures it from a clock, a replay reads it from a `tick`. Everything
//! else about drawing a frame is the same, and `render`'s own documentation
//! said so.
//!
//! It was not the same. The seam that was meant to be one line had become five,
//! and **the extra four are where this project's replay defects came from**:
//!
//! - The **look** was applied per frame live and once before the loop
//!   offscreen, so a session in which the operator changed the tone mapper or
//!   the exposure replayed entirely under whatever it started with.
//! - The **governor** ran live and had never run on the offscreen path, so a
//!   replay granted every residency request where `Record::Residency`'s own
//!   documentation says the effective level must be re-derived per machine.
//! - The **present pass** ran every frame live and only on kept frames
//!   offscreen.
//! - The **events drain and the status line** are the live path's alone, which
//!   is correct and is the one difference that stayed.
//!
//! **Two of those were already fixed, one at a time, in the commit before this
//! one** — and that is the argument for this module rather than against it.
//! Each was found by a review reading two functions side by side and noticing
//! they disagreed; neither was found by a test, because no test could see the
//! difference between two loops. Fixing them left the two loops in place to
//! drift again. This makes the seam the one line it was supposed to be, so
//! there is nothing left to drift.
//!
//! ## The ordering is structural, not stated
//!
//! A frame must acquire somewhere to draw **before** it records anything about
//! itself, because a `tick` is a promise that the deck advanced and a frame
//! that is abandoned did not. That used to be two statements in the right order
//! inside one long function, and getting it wrong was invisible — a replay
//! diverged from the performance by however many frames the window had
//! abandoned, which on a resize is several. That one *was* this module's to
//! fix, and it was fixed by making it unrepresentable rather than by putting
//! the statements back in order.
//!
//! [`compose`] takes the committing work as a closure and calls it only once
//! [`Sink::acquire`] has returned a target. There is no order left to get
//! wrong. This is the same move `Deck::begin_frame` makes with its guard — a
//! second frame while one is open does not compile — one level up.

use karakuri_engine::{Deck, Gpu, Present};
#[cfg(test)]
use karakuri_store::record::MAX_STEPS;

use crate::Look;

/// Why a frame is not being drawn.
///
/// Not an error: a sink that has no target right now is an ordinary event —
/// a window between swapchain configurations, a display coming back. What
/// matters is that **nothing was committed**, so the caller's clock and record
/// stream should behave as though the frame never happened, which is what
/// [`compose`] guarantees by not calling the committing closure at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Skip {
    /// Ask again next frame; there is nothing worth saying about it. Ordinary
    /// jitter, and a message here would be sixty messages a second.
    Transient,
    /// Worth naming, and **produced at most once for as long as the condition
    /// lasts** — the sink latches it, because the sink is the only thing that
    /// knows whether this is the same fault as last frame.
    ///
    /// The latch has to be here rather than in the caller. A caller that
    /// formatted the message and then decided not to print it would allocate
    /// on the frame path sixty times a second for as long as a wedged window
    /// stayed wedged, which is the one thing this program's frame path may
    /// never do.
    Fault(String),
}

/// What a frame decided about itself, produced by the closure [`compose`] calls
/// once it has somewhere to draw.
pub struct Committed {
    /// How many simulation steps this frame advances by. The **one** thing that
    /// legitimately differs between a live run and a replay: measured from a
    /// clock there, read from a `tick` here.
    pub steps: u8,
    /// The output look this frame is under. Carried per frame rather than
    /// fixed before the loop because a `look` record moves it mid-session, and
    /// a loop that took it as a parameter had no way to hear about that.
    pub look: Look,
}

/// Where a composited frame goes.
///
/// Two implementations exist in this program — a window and a PNG writer — and
/// that is the point: an abstraction with one implementation is a guess, and
/// with two it is an extraction. A third, in the tests, is what finally lets
/// the frame loop be driven without a display.
///
/// **Everything here is called exactly once per drawn frame, in this order:**
/// `acquire`, then `view` and `size`, then `after_draw`, then `present`. A sink
/// that returns `Err` from `acquire` has none of the rest called.
pub trait Sink {
    /// Take hold of this frame's attachment.
    ///
    /// Called **before anything about the frame is recorded or measured**, so
    /// returning `Err` costs nothing but the frame.
    fn acquire(&mut self, gpu: &Gpu) -> Result<(), Skip>;

    /// The attachment acquired above.
    fn view(&self) -> &wgpu::TextureView;

    /// Its size in texels, which **need not be the canvas's** — a window is a
    /// preview and the canvas is fitted into it. See `Present::draw`.
    fn size(&self) -> (u32, u32);

    /// Recorded into the frame's own encoder, immediately after the present
    /// pass has drawn into [`Sink::view`].
    ///
    /// The default is nothing, which is what a window wants: it has only to be
    /// presented. A PNG writer puts its texture-to-buffer copy here so that the
    /// copy belongs to the frame that produced it rather than to an encoder of
    /// its own.
    fn after_draw(&mut self, _encoder: &mut wgpu::CommandEncoder) {}

    /// After the frame's encoder has been submitted.
    ///
    /// A window presents; a PNG writer maps its readback and writes the file.
    /// An `Err` here is a real failure rather than a skip — the frame happened.
    fn present(&mut self, gpu: &Gpu) -> Result<(), String>;
}

/// What [`compose`] did.
#[derive(Debug)]
pub enum Outcome {
    /// The frame was committed and drawn.
    Drawn,
    /// The sink had no target. **The committing closure was never called**, so
    /// no clock was read, no record was written, and the deck did not advance.
    Skipped(Skip),
}

/// Compose one frame: acquire a target, commit, render, draw, present.
///
/// `commit` is where everything a frame decides about itself happens —
/// measuring the clock, applying whatever the stream says belongs before this
/// frame, writing the records. **It is called only after a target exists**, and
/// that is the whole reason it is a closure rather than a parameter.
///
/// Note what is *not* here: no clock, no recorder, no window. The frame loop
/// does not know whether it is live. That is what makes it one loop.
pub fn compose(
    gpu: &Gpu,
    deck: &mut Deck,
    present: &Present,
    sink: &mut impl Sink,
    commit: impl FnOnce(&mut Deck) -> Committed,
) -> Result<Outcome, String> {
    if let Err(skip) = sink.acquire(gpu) {
        return Ok(Outcome::Skipped(skip));
    }

    let Committed { steps, look } = commit(deck);

    // Per frame and unconditionally: one `queue.write_buffer` into storage
    // sized at construction, which is the claim `Present`'s module doc makes
    // about switching operators mid-set being free. Tracking whether it changed
    // would buy nothing and cost a way to go stale.
    present.set_tonemap(&gpu.queue, look.op, look.exposure, look.white_point);

    // The guard owns the encoder, so everything recorded here is one generation
    // of Sets: builds are installed inside `begin_frame`, before the encoder
    // exists, and there is no way to reach a second generation while this one
    // is open.
    {
        let mut frame = deck.begin_frame(&gpu.device, &gpu.queue);
        frame.render(present.hdr_view(), present.size(), steps);
        // **Drawn every frame, even by a sink that will not keep it.** A
        // sequence writing one frame in a hundred used to skip the present pass
        // on the other ninety-nine, which meant one more thing that happened on
        // one path and not the other. The pass is a fullscreen triangle; the
        // expensive half is the readback, and that is still conditional — a
        // sink keeps that decision to itself, in `after_draw`.
        present.draw(frame.encoder(), sink.view(), sink.size());
        sink.after_draw(frame.encoder());
        frame.finish();
    }

    sink.present(gpu)?;
    Ok(Outcome::Drawn)
}

/// The default sink: the window on the operator's desk.
///
/// It owns the surface and its configuration because it is the only thing that
/// should touch them — a swapchain follows the window, and nothing about a
/// window reaches what is drawn. `Live` used to hold both and reconfigure them
/// from three places.
pub struct WindowSink {
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    /// The acquired texture and a view of it, held from `acquire` to `present`.
    /// `None` between frames, and `view` is only reachable in between.
    current: Option<(wgpu::SurfaceTexture, wgpu::TextureView)>,
    /// Whether a fault has already been reported. See [`Skip::Fault`]: the
    /// message is built once and never again, so a window that stays broken
    /// costs nothing per frame.
    faulted: bool,
}

impl WindowSink {
    pub fn new(surface: wgpu::Surface<'static>, config: wgpu::SurfaceConfiguration) -> WindowSink {
        WindowSink {
            surface,
            config,
            current: None,
            faulted: false,
        }
    }

    /// The window changed size. **Nothing that is rendered changes** — see
    /// `Record::Canvas`. All that follows a window is the swapchain, because
    /// the swapchain *is* the window.
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(device, &self.config);
    }
}

impl Sink for WindowSink {
    fn acquire(&mut self, gpu: &Gpu) -> Result<(), Skip> {
        let texture = match self.surface.get_current_texture() {
            Ok(texture) => texture,
            // The swapchain needs remaking, which is what these two mean.
            // Reconfigured here and retried next frame rather than in a loop:
            // a frame is cheap and a spin is not.
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                self.surface.configure(&gpu.device, &self.config);
                return Err(Skip::Transient);
            }
            // Genuinely transient and self-describing. Naming it would be
            // naming ordinary jitter.
            Err(wgpu::SurfaceError::Timeout) => return Err(Skip::Transient),
            // Not self-correcting the way `Timeout` is — one is fatal and the
            // other is a generic failure the caller cannot act on. Returning
            // silently left a frozen window with no reason for it anywhere;
            // saying it every frame would bury it under sixty copies a second
            // of itself, and formatting it every frame would allocate on the
            // frame path. So it is built exactly once.
            Err(e) => {
                if self.faulted {
                    return Err(Skip::Transient);
                }
                self.faulted = true;
                return Err(Skip::Fault(format!("{e} — the window has stopped drawing")));
            }
        };
        // A frame arrived, so whatever went wrong is over and the next fault
        // is worth naming again.
        self.faulted = false;
        let view = texture.texture.create_view(&Default::default());
        self.current = Some((texture, view));
        Ok(())
    }

    fn view(&self) -> &wgpu::TextureView {
        &self
            .current
            .as_ref()
            .expect("`view` before `acquire` — see the Sink contract")
            .1
    }

    fn size(&self) -> (u32, u32) {
        (self.config.width, self.config.height)
    }

    fn present(&mut self, _gpu: &Gpu) -> Result<(), String> {
        if let Some((texture, _view)) = self.current.take() {
            texture.present();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use karakuri_engine::{HotSwap, Set, TonemapOp};
    use std::cell::RefCell;

    use crate::{Clock, DT};
    use std::time::Instant;

    /// A sink that draws nowhere and can be told to refuse.
    ///
    /// **This is the third implementation, and the reason the other two were
    /// worth putting behind a trait.** `Live::frame` had never been reached by
    /// a test in this program: it needs a window, and `SurfaceError::Outdated`
    /// — the case that produced a real defect — cannot be synthesised at all.
    /// Two of the three record-ordering bugs found in this codebase lived in
    /// that function. A sink that refuses on demand is what makes the case
    /// reachable.
    struct TestSink {
        target: wgpu::Texture,
        view: wgpu::TextureView,
        /// So a test can look at what was drawn. `SIZE * 4` is 256, so the rows
        /// need no padding — see `render::unpad_rows` for when they do.
        readback: wgpu::Buffer,
        /// What `acquire` answers, popped front to back. Empty means accept.
        answers: Vec<Result<(), Skip>>,
        acquired: usize,
        presented: usize,
        /// Whether the last presented frame had any light in it at all.
        lit: Option<bool>,
    }

    impl TestSink {
        fn new(gpu: &Gpu, answers: Vec<Result<(), Skip>>) -> TestSink {
            let target = gpu.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("test sink"),
                size: wgpu::Extent3d {
                    width: SIZE,
                    height: SIZE,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            let view = target.create_view(&Default::default());
            let readback = gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("test sink readback"),
                size: u64::from(SIZE * SIZE * 4),
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            TestSink {
                target,
                view,
                readback,
                answers,
                acquired: 0,
                presented: 0,
                lit: None,
            }
        }
    }

    impl Sink for TestSink {
        fn acquire(&mut self, _gpu: &Gpu) -> Result<(), Skip> {
            let answer = if self.acquired < self.answers.len() {
                self.answers[self.acquired].clone()
            } else {
                Ok(())
            };
            self.acquired += 1;
            answer
        }
        fn view(&self) -> &wgpu::TextureView {
            &self.view
        }
        fn size(&self) -> (u32, u32) {
            (SIZE, SIZE)
        }
        fn after_draw(&mut self, encoder: &mut wgpu::CommandEncoder) {
            encoder.copy_texture_to_buffer(
                self.target.as_image_copy(),
                wgpu::TexelCopyBufferInfo {
                    buffer: &self.readback,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(SIZE * 4),
                        rows_per_image: Some(SIZE),
                    },
                },
                wgpu::Extent3d {
                    width: SIZE,
                    height: SIZE,
                    depth_or_array_layers: 1,
                },
            );
        }

        fn present(&mut self, gpu: &Gpu) -> Result<(), String> {
            self.presented += 1;
            let slice = self.readback.slice(..);
            slice.map_async(wgpu::MapMode::Read, |r| r.expect("map"));
            gpu.device.poll(wgpu::PollType::Wait).expect("poll");
            // **Colour only.** The present pass returns `vec4(rgb, 1.0)`, so
            // every texel's alpha is 255 and a scan over all four channels
            // answers "lit" for a frame that is entirely black.
            let lit = slice
                .get_mapped_range()
                .chunks(4)
                .any(|texel| texel[..3].iter().any(|&b| b > 0));
            self.readback.unmap();
            self.lit = Some(lit);
            Ok(())
        }
    }

    const SIZE: u32 = 64;
    const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

    fn look() -> Look {
        Look {
            op: TonemapOp::Clamp,
            exposure: 1.0,
            white_point: 1.0,
        }
    }

    /// A one-slot deck built from the example pair, which is what every other
    /// test in this crate reaches for when it needs real material.
    fn one_slot_deck(gpu: &Gpu) -> Deck {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let l1 = crate::compile::load(&root.join("examples/drift_shell.kir")).expect("L1");
        let l4 = crate::compile::load(&root.join("examples/soft_points.kir")).expect("L4");
        let set = Set::build(&gpu.device, &gpu.queue, &l1, &l4, 4096, 7).expect("set");
        Deck::new(&gpu.device, vec![HotSwap::fixed(set)], SIZE, SIZE)
    }

    /// **A frame with nowhere to draw commits nothing.**
    ///
    /// The defect this replaces was invisible: the loop read the clock, wrote a
    /// `tick` claiming those steps, measured the audio, and only then found the
    /// swapchain had no texture — so an abandoned frame told the session it had
    /// simulated steps the deck never took, and a replay obeyed the record.
    /// `Outdated` arrives on every resize, so resizing during a recording was
    /// enough to make the replay diverge.
    ///
    /// It cannot happen now because the committing work is a closure `compose`
    /// only calls once a target exists, and this is that claim: refuse, and the
    /// closure never runs.
    #[test]
    fn a_refused_frame_never_reaches_the_committing_work() {
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
        let commits = RefCell::new(0);

        for expected in [false, false, true] {
            let before = *commits.borrow();
            let outcome = compose(&gpu, &mut deck, &present, &mut sink, |_| {
                *commits.borrow_mut() += 1;
                Committed {
                    steps: 1,
                    look: look(),
                }
            })
            .expect("compose");
            let committed = *commits.borrow() > before;
            assert_eq!(
                committed, expected,
                "committing on a refused frame is the whole defect: {outcome:?}"
            );
        }
        assert_eq!(*commits.borrow(), 1, "only the accepted frame committed");
        assert_eq!(sink.presented, 1, "and only it was presented");
    }

    /// A refused frame does not advance the deck either — the other half of the
    /// same promise, seen from the material rather than from the stream.
    #[test]
    fn a_refused_frame_does_not_advance_the_simulation() {
        let gpu = Gpu::headless().expect("no GPU");
        let mut deck = one_slot_deck(&gpu);
        let present = Present::new(&gpu.device, FORMAT, SIZE, SIZE);
        let mut sink = TestSink::new(&gpu, vec![Err(Skip::Transient)]);

        let before = deck.slot(0).set().time();
        compose(&gpu, &mut deck, &present, &mut sink, |_| Committed {
            steps: 4,
            look: look(),
        })
        .expect("compose");
        assert_eq!(
            deck.slot(0).set().time(),
            before,
            "a frame with nowhere to draw stepped the simulation"
        );

        // And an accepted one does, so the assertion above is about the refusal
        // rather than about a deck that never moves.
        compose(&gpu, &mut deck, &present, &mut sink, |_| Committed {
            steps: 4,
            look: look(),
        })
        .expect("compose");
        assert!(
            deck.slot(0).set().time() > before,
            "an accepted frame did not step either — the test proves nothing"
        );
    }

    /// The committing closure runs **before** anything is drawn, seen in the
    /// pixels rather than in the deck.
    ///
    /// This test was vacuous first, and the mutation is what said so: it set a
    /// gain in the closure and asserted the deck held it afterwards, which is
    /// true whichever side of the draw the closure ran on. It was testing that
    /// `set_gain` works. **A claim about order has to be read off the thing the
    /// order affects**, so it reads the frame: a slot faded to nothing in the
    /// closure must produce a dark frame, and it only does if the closure ran
    /// first.
    #[test]
    fn a_frame_commits_before_it_draws() {
        let gpu = Gpu::headless().expect("no GPU");
        let present = Present::new(&gpu.device, FORMAT, SIZE, SIZE);

        // The control, and it is not optional: if this material drew nothing at
        // this size the assertion below would hold for the wrong reason.
        let mut deck = one_slot_deck(&gpu);
        let mut sink = TestSink::new(&gpu, vec![]);
        compose(&gpu, &mut deck, &present, &mut sink, |_| Committed {
            steps: 1,
            look: look(),
        })
        .expect("compose");
        assert_eq!(sink.lit, Some(true), "the fixture drew nothing to darken");

        // The same frame, with the fader taken to zero inside the closure.
        // Opacity rather than gain because opacity silences a layer under every
        // blend mode and gain does not silence `over`.
        let mut deck = one_slot_deck(&gpu);
        let mut sink = TestSink::new(&gpu, vec![]);
        compose(&gpu, &mut deck, &present, &mut sink, |deck| {
            deck.set_opacity(0, 0.0);
            Committed {
                steps: 1,
                look: look(),
            }
        })
        .expect("compose");
        assert_eq!(
            sink.lit,
            Some(false),
            "the frame was drawn before the closure that faded it"
        );
    }

    /// **An abandoned frame's interval is not lost — the next frame counts it.**
    ///
    /// The claim the frame loop's ordering rests on, and until `steps` could be
    /// told what time it is there was no way to state it: the first version of
    /// this test asserted that the step count did not exceed `MAX_STEPS` (it
    /// cannot: `steps` clamps to it) and that the carry was under one (it is:
    /// `steps` subtracts its own floor). Both survived deleting the body of
    /// `Clock::steps`.
    ///
    /// Two clocks over the same span, one reading it in two frames and one in
    /// a single frame because the other was abandoned, must hand out the same
    /// total. That is what "the time survives" means, and it is false for any
    /// clock that resets `last` somewhere other than a frame that goes ahead.
    #[test]
    fn a_frame_that_never_drew_leaves_its_time_for_the_next_one() {
        let start = Instant::now();
        let ms = |n: u64| start + std::time::Duration::from_millis(n);

        let mut drew_every_frame = Clock::new(start);
        let both = u32::from(drew_every_frame.steps(ms(16))) + u32::from(drew_every_frame.steps(ms(32)));

        // The same thirty-two milliseconds, with the frame at 16 ms abandoned:
        // `steps` is not called, so `last` does not move.
        let mut skipped_one = Clock::new(start);
        let one = u32::from(skipped_one.steps(ms(32)));

        assert_eq!(
            one, both,
            "the abandoned frame's interval was dropped rather than carried"
        );
        assert!(both > 0, "thirty-two milliseconds is at least one step");
    }

    /// The carry is what makes that true across a frame rate that does not
    /// divide the step rate: whole steps out, the fraction kept.
    #[test]
    fn the_clock_hands_out_whole_steps_and_keeps_the_fraction() {
        let start = Instant::now();
        let mut clock = Clock::new(start);
        let mut total = 0u32;
        // Sixty frames of 16 ms is 960 ms, and at `DT` per step that is a known
        // number of steps — known well enough that dropping the carry loses
        // several of them.
        for i in 1..=60u64 {
            total += u32::from(clock.steps(start + std::time::Duration::from_millis(i * 16)));
        }
        let expected = (0.960 / f64::from(DT)).floor() as u32;
        assert_eq!(
            total, expected,
            "the fraction between frames was dropped: {total} steps for 960 ms"
        );
    }

    /// And the anti-spiral clamp holds: a stall does not become a catch-up.
    #[test]
    fn a_long_gap_falls_behind_rather_than_catching_up() {
        let start = Instant::now();
        let mut clock = Clock::new(start);
        let steps = clock.steps(start + std::time::Duration::from_secs(5));
        assert_eq!(steps, MAX_STEPS, "five seconds is not four steps' worth");
    }
}
