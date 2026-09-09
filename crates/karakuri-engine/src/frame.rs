//! One frame, and the one place it is composed.
//!
//! There were two frame loops, both in `karakuri-cli`: its `Live::frame` drove
//! the window and its `render::sequence_driven` drove a PNG, and the only
//! difference that was ever *meant* to exist between them is where the step
//! count comes from — a live run measures it from a clock, a replay reads it
//! from a `tick`. Everything else about drawing a frame is the same, and
//! `render`'s own documentation said so.
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
//! **A frame commits before it draws, and there is no path between the two.**
//! [`compose`] takes the committing work as a closure and calls it in exactly
//! one place — after every sink has been asked for a target, immediately before
//! the deck is rendered — so the order is the shape of the call rather than two
//! statements a reader has to keep in the right sequence. That was the point of
//! this module and it still is: the same move `Deck::begin_frame` makes with
//! its guard, one level up.
//!
//! **What changed is what the closure is conditional on: nothing.** This module
//! was written around the opposite rule, and the rule is worth keeping on the
//! page rather than quietly deleting — *"a frame must acquire somewhere to draw
//! before it records anything about itself, because a `tick` is a promise that
//! the deck advanced and a frame that is abandoned did not."* The defect behind
//! it was real, and is
//! `docs/adr/0078-a-frame-that-is-discarded-must-not-already-have-been-recorded.md`:
//! the loop read the clock, wrote a `tick` claiming those steps, and only then
//! found the swapchain had nothing — and `Outdated` arrives on every resize,
//! so resizing during a recording made the replay diverge from the
//! performance by however many frames the window had abandoned. So `compose`
//! withheld the closure until [`Sink::acquire`] had returned a target, and an
//! abandoned frame committed nothing at all.
//!
//! **That rule could not survive a second sink.** With one sink, "the frame was
//! abandoned" and "the sink had no target" are the same sentence. With a
//! projector beside the window they are not: a frame that reached the projector
//! and missed the window did not *not happen*, and there is no answer to "did
//! this frame advance the deck?" that is right for both of them. Nor was it
//! right with one sink as a steady state — an operator who turns every output
//! off stops publishing the instrument, and stopping the instrument is a
//! different thing that nobody asked for.
//!
//! **The guarantee it was protecting is stronger now, not abandoned.** The deck
//! advances on every frame `compose` composes, so a `tick` is an honest promise
//! *unconditionally* rather than on the frames that happened to find a target:
//! the question the old rule answered by refusing to commit — can a `tick`
//! claim steps the deck never took? — is now answered by the commit and the
//! render being adjacent, with no acquire, no sink and no early return between
//! them. Acquiring moved out from under it: every sink is asked first, each
//! answers for itself, and what an answer decides is whether that sink is drawn
//! into and presented. Publishing is what a sink gates; the instrument runs.
//!
//! ## Why it is in the engine
//!
//! It was written in `karakuri-cli`, which is scaffolding rather than the
//! destination, and that crate has no library target — so the application
//! could not reach it, and `karakuri-console`'s example hand-rolled a second
//! frame loop instead. That is the two-loops-that-drift failure this module was
//! written to end, one crate over. Nothing here needed the CLI: a frame is a
//! deck, a `Present` and somewhere to put the result, and all three are the
//! engine's. What stayed behind is what genuinely belongs to a program rather
//! than to a frame — the clock, the recorder, the PNG writer.

use crate::deck::Deck;
use crate::gpu::Gpu;
use crate::present::{Present, TonemapOp};

/// The output look: everything the tone mapper is told, in one value, so the
/// window and an offscreen render can be given the same thing and agree.
///
/// Exactly [`Present::set_tonemap`]'s arguments, which is why it is here rather
/// than with whoever puts an operator on a key: a look is what a frame is drawn
/// under, and every sink is drawn under the same one.
///
/// **The master out is deliberately not here.** It is a level too, and at 1.0
/// it does what `exposure` at 1.0 does — but it is applied at the entry to the
/// master chain, where the mix writes the composited frame, and this struct is
/// what the *tone mapper* is told at the other end of that chain. It lives on
/// the deck, as [`Deck::set_out`], because that is what owns the fold. See
/// `docs/adr/0224-out-and-exposure-are-two-levels-that-multiply-in-different-places.md`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Look {
    pub op: TonemapOp,
    /// The level going into the transfer, applied by the present pass. Not the
    /// master out; see this struct's own documentation.
    pub exposure: f32,
    /// Reinhard's only, ignored by the other three. Not on a key: it is one
    /// operator's parameter rather than a control the mix needs.
    pub white_point: f32,
}

/// Why a frame is not reaching **one sink**.
///
/// Not an error: a sink that has no target right now is an ordinary event —
/// a window between swapchain configurations, a display coming back, an output
/// the operator has switched off. It says nothing about the frame, which is
/// composed and committed either way: what a refusal decides is that this sink
/// is not drawn into and not presented, and [`compose`] hands it to the
/// caller's reporting closure so that something can be said about a window that
/// has stopped taking frames.
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
    /// stayed wedged, which is the one thing this crate's frame path may
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
/// Two implementations exist in this workspace — [`WindowSink`] below and
/// `karakuri-cli`'s PNG writer — and that is the point: an abstraction with one
/// implementation is a guess, and with two it is an extraction. A third, in the
/// tests, is what finally lets the frame loop be driven without a display.
///
/// **Everything here is called exactly once per frame per sink, in this
/// order:** `acquire`, then `view` and `size`, then `after_draw`, then
/// `present`. A sink that returns `Err` from `acquire` has **none of the rest
/// called on it** — and the frame still happens, into whichever other sinks
/// took it, and into none of them if that is all of them. Each sink answers for
/// itself and hears nothing about what the others answered.
///
/// **[`compose`]'s `finally` sits between the last `after_draw` and the first
/// `present`**, and it is not part of this contract: it is the caller's own
/// work in the frame's encoder, downstream of every sink. See [`compose`].
pub trait Sink {
    /// Take hold of this frame's attachment.
    ///
    /// Called on every sink **before anything about the frame is recorded or
    /// measured**, so a sink is never asked to answer for work already done and
    /// an `Err` costs nothing but this sink's copy of the frame. It does not
    /// cost the frame: the deck advances whatever every sink answers.
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
    /// An `Err` here is a real failure rather than a skip — the frame happened,
    /// and it happened for the other sinks too, so [`compose`] presents all of
    /// them before it returns the first error.
    fn present(&mut self, gpu: &Gpu) -> Result<(), String>;
}

/// Where a composed frame went, counted.
///
/// **Not whether it happened** — it happened; [`compose`] commits and renders
/// unconditionally. A summary rather than a per-sink report because a `Vec` of
/// outcomes would allocate on the frame path once per frame forever, for
/// something every caller so far reduces to a number: *which* sink refused and
/// why goes to the reporting closure instead, at the moment it refuses, and
/// `Skip::Fault` already latches so that costs nothing per frame.
///
/// `reached + missed` is how many sinks were asked, and **both being zero is an
/// ordinary state**: every output off is a frame that ran and published
/// nowhere, which is what setting up before doors looks like.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Outcome {
    /// How many sinks were drawn into and presented.
    pub reached: usize,
    /// How many were asked and had no target for this frame.
    pub missed: usize,
}

/// Compose one frame for `sinks`: ask every sink, commit, render, draw into the
/// ones that answered, present them.
///
/// `commit` is where everything a frame decides about itself happens —
/// measuring the clock, applying whatever the stream says belongs before this
/// frame, writing the records. **It is called once, whatever the sinks
/// answered**, and being a closure is what keeps it adjacent to the render with
/// nothing between them.
///
/// `sinks` is a borrowed slice rather than anything owned because this is the
/// frame path and the frame path allocates nothing; `refused` is called once
/// per sink that had no target, with that sink's index in the slice.
///
/// ## `finally`, and why it is not a hook looking for a user
///
/// `finally` is recorded into the frame's own encoder **after every sink that
/// took the frame has been drawn into and had [`Sink::after_draw`] called, and
/// before the encoder is submitted**. It runs on every frame, including one
/// that no sink took at all.
///
/// It is there because **a frame's submission is not only its sinks.**
/// `karakuri-console` draws its whole panel — the `egui` pass — into the same
/// command buffer the deck's frame is recorded in, because the picture is a
/// colour attachment in the deck's pass and a sampled texture in the panel's,
/// and two submissions over one texture is a race whose order is the queue's
/// business rather than the caller's. That is
/// `docs/adr/0166-the-engines-frame-and-the-panels-are-one-submission.md`. And
/// it is **not a sink**: the panel does not receive the composited frame, it
/// receives the panel, and it happens to sample what a sink produced.
///
/// **Rejected, and it is the one that will be re-proposed:** make the panel a
/// [`Sink`] whose recording work is the `egui` pass rather than the canvas —
/// no new parameter, and the panel goes in the slice beside the picture. It
/// loses on two things.
///
/// - **It changes what [`Sink`] means**, from *where a composited frame goes*
///   to *anything that wants a slot in this encoder*, and it puts a consumer
///   in a list of producers: the panel is downstream of the picture rather
///   than its peer, and it is the only thing in the slice that would be
///   handed a target it does not want the present pass drawn into.
/// - **It makes [`Outcome::reached`] count two different things at once.**
///   With the picture turned off from Outputs the panel is still in the slice,
///   so a frame that reached *no output at all* would report `reached: 1` —
///   and *every output may be off*, which the manual promises and
///   `docs/adr/0171-the-deck-advances-and-each-sink-either-gets-the-frame-or-misses-it.md`
///   made representable, stops being something the number can say.
///
/// A caller with nothing to add passes `|_| {}`, which is both of
/// `karakuri-cli`'s call sites.
///
/// **[`compose`] may reorder `sinks`.** The sinks that acquired are moved to
/// the front, keeping their order among themselves, which is how the draw loop
/// knows who answered without a `Vec<bool>` to hold it. Indices handed to
/// `refused` are the caller's, taken before anything moves.
///
/// An `Err` is one sink's [`Sink::present`] failing, reported after **every**
/// acquired sink has been presented: the frame reached them and a projector
/// failing is no reason for the window to miss a frame it had in hand. The
/// first error is the one returned.
///
/// Note what is *not* here: no clock, no recorder, no window. The frame loop
/// does not know whether it is live. That is what makes it one loop.
pub fn compose(
    gpu: &Gpu,
    deck: &mut Deck,
    present: &Present,
    sinks: &mut [&mut dyn Sink],
    refused: &mut dyn FnMut(usize, Skip),
    commit: impl FnOnce(&mut Deck) -> Committed,
    finally: impl FnOnce(&mut wgpu::CommandEncoder),
) -> Result<Outcome, String> {
    // **Every sink is asked before anything is committed**, which is the half of
    // the old ordering that is still load-bearing: a sink is never asked to
    // answer for a frame that has already been recorded.
    let mut reached = 0;
    for at in 0..sinks.len() {
        match sinks[at].acquire(gpu) {
            // Moved down beside the ones that already answered, which costs a
            // rotate of at most the length of the slice and saves the
            // allocation a per-sink answer would otherwise live in. `at` is
            // still this sink's own index: rotations only ever touch positions
            // at or below the one being visited.
            Ok(()) => {
                sinks[reached..=at].rotate_right(1);
                reached += 1;
            }
            Err(skip) => refused(at, skip),
        }
    }
    let missed = sinks.len() - reached;
    let (drawn, _) = sinks.split_at_mut(reached);

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
        // **`mix_target` and not `hdr_view`**, which are the same view until
        // the master chain has an effect turned up in it: with one, the mix
        // writes into the chain's entry and the chain's last pass writes into
        // the target the present pass reads. See `crate::master`.
        frame.render(present.mix_target(), present.size(), steps);
        // **Between the fold and every sink**, in linear HDR and upstream of
        // the one tone map. Nothing is recorded for a chain at zero.
        present.draw_chain(frame.encoder());
        // **Drawn every frame, even by a sink that will not keep it.** A
        // sequence writing one frame in a hundred used to skip the present pass
        // on the other ninety-nine, which meant one more thing that happened on
        // one path and not the other. The pass is a fullscreen triangle; the
        // expensive half is the readback, and that is still conditional — a
        // sink keeps that decision to itself, in `after_draw`.
        //
        // **Once per sink that took the frame, and no times at all when none
        // did** — a frame with every output off is drawn nowhere and still
        // rendered above. Every sink is handed the same canvas and fits it into
        // whatever size it has: `Present::draw` letterboxes per target, so a
        // window and a projector of different shapes need one `Present` between
        // them rather than one each.
        for sink in drawn.iter_mut() {
            present.draw(frame.encoder(), sink.view(), sink.size());
            sink.after_draw(frame.encoder());
        }
        // **Last, and into the same encoder.** Everything a sink is owed has
        // been recorded, so a caller adding work here is adding it downstream
        // of every sink that took the frame — which is what the console's
        // panel pass is: it samples the picture a sink was drawn into, in the
        // one command buffer, so the barrier between the two is `wgpu`'s
        // rather than the queue's guess. See this function's own
        // documentation for why it is not a sink.
        finally(frame.encoder());
        frame.finish();
    }

    // **Every sink is presented before any error is reported.** Returning at
    // the first one would let a wedged output cost every sink below it a frame
    // it had already been drawn into, which is the window waiting on a plugin —
    // the one thing `docs/plugins.md` says never happens.
    let mut failed = None;
    for sink in drawn.iter_mut() {
        if let Err(e) = sink.present(gpu) {
            failed.get_or_insert(e);
        }
    }
    match failed {
        Some(e) => Err(e),
        None => Ok(Outcome { reached, missed }),
    }
}

/// The default sink: the window on the operator's desk.
///
/// It owns the surface and its configuration because it is the only thing that
/// should touch them — a swapchain follows the window, and nothing about a
/// window reaches what is drawn. `karakuri-cli`'s `Live` used to hold both and
/// reconfigure them from three places.
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
            // `Suboptimal` is a texture like any other — it draws correctly and
            // asks to be reconfigured for performance, which the next resize
            // does anyway. Skipping the frame to reconfigure would drop a frame
            // that was in hand.
            wgpu::CurrentSurfaceTexture::Success(texture)
            | wgpu::CurrentSurfaceTexture::Suboptimal(texture) => texture,
            // The swapchain needs remaking, which is what these two mean.
            // Reconfigured here and retried next frame rather than in a loop:
            // a frame is cheap and a spin is not.
            wgpu::CurrentSurfaceTexture::Lost | wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&gpu.device, &self.config);
                return Err(Skip::Transient);
            }
            // Genuinely transient and self-describing. Naming them would be
            // naming ordinary jitter — or, for `Occluded`, naming a minimised
            // window. Neither is a fault: the frame is dropped and the next one
            // is asked for.
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Err(Skip::Transient)
            }
            // Not self-correcting the way `Timeout` is — one is fatal and the
            // other is a generic failure the caller cannot act on. Returning
            // silently left a frozen window with no reason for it anywhere;
            // saying it every frame would bury it under sixty copies a second
            // of itself, and formatting it every frame would allocate on the
            // frame path. So it is built exactly once.
            wgpu::CurrentSurfaceTexture::Validation => {
                if self.faulted {
                    return Err(Skip::Transient);
                }
                self.faulted = true;
                return Err(Skip::Fault(
                    "acquiring a texture raised a validation error \
                     — the window has stopped drawing"
                        .into(),
                ));
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

    fn present(&mut self, gpu: &Gpu) -> Result<(), String> {
        if let Some((texture, _view)) = self.current.take() {
            gpu.queue.present(texture);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::set::Set;
    use crate::swap::HotSwap;
    use karakuri_ir::typed::Checked;
    use std::cell::Cell;
    use std::rc::Rc;

    /// A sink that draws nowhere and can be told to refuse.
    ///
    /// **This is the third implementation, and the reason the other two were
    /// worth putting behind a trait.** `karakuri-cli`'s `Live::frame` had never
    /// been reached by a test: it needs a window, and `Outdated` — the case
    /// that produced a real defect — cannot be synthesised at all. (It was
    /// `SurfaceError::Outdated` until wgpu 30 replaced the `Result` with
    /// `CurrentSurfaceTexture`; the point survives the rename.)
    /// Two of the three record-ordering bugs found in this codebase lived in
    /// that function. A sink that refuses on demand is what makes the case
    /// reachable.
    struct TestSink {
        target: wgpu::Texture,
        view: wgpu::TextureView,
        /// So a test can look at what was drawn. Every width used here times
        /// four is a multiple of 256, so the rows need no padding — see
        /// `karakuri-cli`'s `render::unpad_rows` for when they do.
        readback: wgpu::Buffer,
        width: u32,
        height: u32,
        /// What `acquire` answers, popped front to back. Empty means accept.
        answers: Vec<Result<(), Skip>>,
        /// What `present` answers, every time. `None` accepts.
        present_answer: Option<String>,
        acquired: usize,
        /// **Every call the contract names, counted separately**, because the
        /// claim a refusing sink makes is about what was *not* called on it and
        /// a single "was it drawn" flag cannot tell "never asked" from "asked
        /// and did nothing". `view` takes `&self`, hence the `Cell`.
        viewed: Cell<usize>,
        after_drawn: usize,
        presented: usize,
        /// **A tick shared with the other sinks and with the caller's
        /// `finally`**, so the order of the calls across all of them is read
        /// off afterwards rather than inferred from a per-sink counter. A
        /// count says how many times; only a shared tick says *when*.
        order: Rc<Cell<usize>>,
        /// The tick this sink's `after_draw` took, and the address of the
        /// encoder it was handed. The second is what says `finally` was given
        /// **this** encoder rather than one `compose` made for it — an
        /// encoder of its own would be a second command buffer, which is
        /// exactly the race ADR-0166 is about.
        drawn_at: usize,
        encoder_at: usize,
        /// Whether the last presented frame had any light in it at all.
        lit: Option<bool>,
        /// And the frame itself, so a test can ask *where* the light is. A
        /// `Vec` on a test double, which is the one place in this file where
        /// that is nobody's business.
        pixels: Vec<u8>,
    }

    impl TestSink {
        fn new(gpu: &Gpu, answers: Vec<Result<(), Skip>>) -> TestSink {
            TestSink::sized(gpu, SIZE, SIZE, answers)
        }

        fn sized(gpu: &Gpu, width: u32, height: u32, answers: Vec<Result<(), Skip>>) -> TestSink {
            assert_eq!(width * 4 % 256, 0, "a padded readback would need unpadding");
            let target = gpu.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("test sink"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            let view = target.create_view(&Default::default());
            let readback = gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("test sink readback"),
                size: u64::from(width * height * 4),
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            TestSink {
                target,
                view,
                readback,
                width,
                height,
                answers,
                present_answer: None,
                acquired: 0,
                viewed: Cell::new(0),
                after_drawn: 0,
                presented: 0,
                order: Rc::new(Cell::new(0)),
                drawn_at: 0,
                encoder_at: 0,
                lit: None,
                pixels: Vec::new(),
            }
        }

        /// Take the ticks from the caller's clock rather than from one of this
        /// sink's own, which is what makes two sinks and a `finally` comparable.
        fn ordered_by(&mut self, order: &Rc<Cell<usize>>) {
            self.order = Rc::clone(order);
        }

        /// Whether any texel in a column band of the last presented frame has
        /// light in it. The band is what says `Present::draw` fitted the canvas
        /// into *this* sink's size rather than into somebody else's.
        fn lit_between(&self, from: u32, to: u32) -> bool {
            (0..self.height).any(|y| {
                (from..to).any(|x| {
                    let at = ((y * self.width + x) * 4) as usize;
                    self.pixels[at..at + 3].iter().any(|&b| b > 0)
                })
            })
        }

        /// Nothing at all was called on this sink but `acquire`, which is the
        /// whole of what a refusal promises.
        fn untouched_after_refusing(&self) -> bool {
            self.viewed.get() == 0 && self.after_drawn == 0 && self.presented == 0
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
            self.viewed.set(self.viewed.get() + 1);
            &self.view
        }
        fn size(&self) -> (u32, u32) {
            (self.width, self.height)
        }
        fn after_draw(&mut self, encoder: &mut wgpu::CommandEncoder) {
            self.after_drawn += 1;
            self.order.set(self.order.get() + 1);
            self.drawn_at = self.order.get();
            self.encoder_at = std::ptr::from_ref(&*encoder) as usize;
            encoder.copy_texture_to_buffer(
                self.target.as_image_copy(),
                wgpu::TexelCopyBufferInfo {
                    buffer: &self.readback,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(self.width * 4),
                        rows_per_image: Some(self.height),
                    },
                },
                wgpu::Extent3d {
                    width: self.width,
                    height: self.height,
                    depth_or_array_layers: 1,
                },
            );
        }

        fn present(&mut self, gpu: &Gpu) -> Result<(), String> {
            self.presented += 1;
            let slice = self.readback.slice(..);
            slice.map_async(wgpu::MapMode::Read, |r| r.expect("map"));
            gpu.device
                .poll(wgpu::PollType::wait_indefinitely())
                .expect("poll");
            self.pixels = slice.get_mapped_range().expect("map").to_vec();
            self.readback.unmap();
            // **Colour only.** The present pass returns `vec4(rgb, 1.0)`, so
            // every texel's alpha is 255 and a scan over all four channels
            // answers "lit" for a frame that is entirely black.
            self.lit = Some(
                self.pixels
                    .chunks(4)
                    .any(|texel| texel[..3].iter().any(|&b| b > 0)),
            );
            match &self.present_answer {
                Some(e) => Err(e.clone()),
                None => Ok(()),
            }
        }
    }

    const SIZE: u32 = 64;
    /// A second sink twice as wide as the canvas and the same height, so the
    /// canvas letterboxes into a band with black either side of it. `128 * 4`
    /// is 512, which is aligned, and the band is exactly `[32, 96)`.
    const WIDE: u32 = 128;
    const BAND: (u32, u32) = ((WIDE - SIZE) / 2, (WIDE + SIZE) / 2);
    const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

    /// One sink, no refusals worth reporting: what most of these tests want.
    fn one(sink: &mut TestSink) -> [&mut dyn Sink; 1] {
        [sink]
    }

    fn look() -> Look {
        Look {
            op: TonemapOp::Clamp,
            exposure: 1.0,
            white_point: 1.0,
        }
    }

    /// The three stages a `.kir` goes through before [`Set::build`] will take
    /// it, the way every other test in this crate spells them — except that
    /// these tests read the shipped `examples/` pair rather than an inline
    /// fixture, because what they want is material that draws something at
    /// [`SIZE`] and the workspace already has some.
    fn compile(path: &std::path::Path) -> Checked {
        let src =
            std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let render = |errs: &[karakuri_ir::IrError]| {
            errs.iter()
                .map(|e| e.render(&src))
                .collect::<Vec<_>>()
                .join("\n")
        };
        let proc = karakuri_ir::parse(&src).unwrap_or_else(|e| panic!("{}", render(&e)));
        let checked = karakuri_ir::check::check(&proc).unwrap_or_else(|e| panic!("{}", render(&e)));
        karakuri_ir::cost::estimate(&checked).unwrap_or_else(|e| panic!("{}", render(&e)));
        checked
    }

    /// A one-slot deck built from the example pair — the same pair
    /// `karakuri-engine`'s own examples build from, and the one the CLI's
    /// window opens on by default.
    fn one_slot_deck(gpu: &Gpu) -> Deck {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let l1 = compile(&root.join("examples/drift_shell.kir"));
        let l4 = compile(&root.join("examples/soft_points.kir"));
        let set = Set::build(&gpu.device, &gpu.queue, &l1, &l4, 4096, 7).expect("set");
        Deck::new(&gpu.device, vec![HotSwap::fixed(set)], SIZE, SIZE)
    }

    // All seven drive a real `Present`, so all seven are here.
    // See `karakuri-engine/tests/gpu_tests_are_under_mod_gpu.rs`.
    mod gpu {
        use super::*;

        /// **A frame with nowhere to draw commits anyway.**
        ///
        /// This test used to be `a_refused_frame_never_reaches_the_committing_work`
        /// and asserted the exact opposite: refuse, and the closure never runs.
        /// The defect behind that was real and is still worth knowing — the loop
        /// read the clock, wrote a `tick` claiming those steps, measured the
        /// audio, and only then found the swapchain had no texture, so an
        /// abandoned frame told the session it had simulated steps the deck
        /// never took and a replay obeyed the record. `Outdated` arrives on
        /// every resize, so resizing during a recording was enough to diverge
        /// the replay.
        ///
        /// **It stopped being the right question the moment a frame could reach
        /// more than one sink.** "Was this frame abandoned?" has no answer that
        /// is right for a frame the projector took and the window missed, and it
        /// was already the wrong answer for one sink held off for a whole set:
        /// every output off stopped the instrument rather than stopped
        /// publishing it. What keeps a `tick` honest now is that there is
        /// nothing between the commit and the render — which is
        /// `a_frame_commits_before_it_draws` below, the claim that survived —
        /// and the deck advancing on every frame makes the promise
        /// unconditional rather than conditional on a swapchain.
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

        /// **And it advances the deck** — the other half of the same reversal,
        /// seen in the material rather than in the stream.
        ///
        /// This was `a_refused_frame_does_not_advance_the_simulation` and
        /// asserted that the deck stood still. It stopped being the right
        /// question for the reason above, and the assertion that replaces it is
        /// the stronger one: a refused frame advances the deck by **the same**
        /// amount an accepted one does. "It moved" would pass on a deck that
        /// moved by anything at all; the promise is that a sink has no say in
        /// how far the simulation goes.
        #[test]
        fn a_refused_frame_still_advances_the_simulation() {
            let gpu = Gpu::headless().expect("no GPU");
            let mut deck = one_slot_deck(&gpu);
            let present = Present::new(&gpu.device, FORMAT, SIZE, SIZE);
            let mut sink = TestSink::new(&gpu, vec![Err(Skip::Transient)]);

            let before = deck.slot(0).set().time();
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
            let refused = deck.slot(0).set().time() - before;
            assert!(
                refused > 0.0,
                "a frame with nowhere to draw did not step the simulation"
            );

            // The accepted frame is the control: without it, "it moved" could
            // be a deck that moves for some other reason, and the equality
            // below is what says a sink has no say in how far it moves.
            let mid = deck.slot(0).set().time();
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
            let accepted = deck.slot(0).set().time() - mid;
            assert!(
                (accepted - refused).abs() < 1e-6,
                "a refused frame advanced by {refused} and an accepted one by {accepted}"
            );
            assert_eq!(sink.presented, 1, "and only the accepted one was presented");
        }

        /// **Every output off is a frame, and the deck still runs.**
        ///
        /// `docs/manual/console.html`'s promise about the outputs row — *"all
        /// of them may be off, and that is a state worth having"*, because the
        /// deck previews are auditions rather than outputs — is this: with no
        /// sink at all the frame is composed, the closure commits, the deck
        /// steps, and nothing is published. `reached` being zero *is* "nothing was
        /// presented": it is the count of the sinks that were drawn into and
        /// presented.
        ///
        /// Zero sinks is also not a miss. Nothing was asked, so nothing
        /// refused, and the reporting closure has nothing to say — a run with
        /// the outputs off must not print a line a second about it.
        #[test]
        fn a_frame_with_no_sinks_at_all_still_advances_the_deck() {
            let gpu = Gpu::headless().expect("no GPU");
            let mut deck = one_slot_deck(&gpu);
            let present = Present::new(&gpu.device, FORMAT, SIZE, SIZE);
            let mut none: [&mut dyn Sink; 0] = [];
            let mut commits = 0;
            let mut refusals = 0;

            let before = deck.slot(0).set().time();
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
                deck.slot(0).set().time() > before,
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

        /// **A sink that refuses costs the others nothing, and is told nothing
        /// else.**
        ///
        /// Three sinks rather than the two this is really about, and the
        /// arrangement is the point. The **first** refuses, so a `compose` that
        /// drew into the slice in the order it was given rather than into the
        /// ones that answered would draw into it. The **last** refuses too, so
        /// its reported index is 2 while the number of sinks that had answered
        /// by then is 1 — with two sinks those two numbers coincide and the
        /// index asserts nothing.
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

        /// **One canvas, two sinks of different shapes, fitted per sink.**
        ///
        /// What the console example already does with one `Present` and two
        /// targets — the picture and a deck preview a fifth the size — and what
        /// a projector beside a window will be. `Present::draw` letterboxes into
        /// whatever `Sink::size` says, so the wide sink gets the canvas in a
        /// centred band with black either side of it.
        ///
        /// The black is the half that catches the defect worth catching: "both
        /// are lit" passes on a `compose` that hands every sink the *first*
        /// sink's size, or its own canvas size, and draws the canvas into a
        /// corner. Where the light stops is the only thing that says which size
        /// was used.
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

        /// **A sink whose `present` fails does not cost the sinks after it their
        /// frame.**
        ///
        /// `docs/plugins.md` is explicit that the window never waits on
        /// anything else, and returning at the first error is exactly how a
        /// wedged output would make it: every sink below it in the slice had
        /// already been drawn into and would go unpresented. So all of them are
        /// presented and the **first** error is the one returned — two failing
        /// sinks here, so "first" is falsifiable rather than a word.
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

        /// The committing closure runs **before** anything is drawn, seen in the
        /// pixels rather than in the deck.
        ///
        /// **This is the claim that survived the reversal, and it is now the
        /// only thing keeping a `tick` honest** — the closure is no longer
        /// withheld from a frame with nowhere to draw, so what a `tick` promises
        /// rests entirely on the commit and the render being adjacent.
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
                        deck.set_opacity(0, 0.0);
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

        /// **`finally` is recorded into the frame's own encoder, after every
        /// sink that took the frame has been drawn into.**
        ///
        /// This is the parameter `karakuri-console`'s panel pass goes in, and
        /// the argument for it is on [`compose`]: a frame's submission is not
        /// only its sinks. Three things have to hold for that to be worth
        /// anything, and each fails a different way.
        ///
        /// - **It runs**, or the panel is never drawn.
        /// - **It runs after every sink was drawn into.** A panel recorded
        ///   before the present pass samples a picture nothing has written
        ///   yet, and that is not an error anywhere — an unwritten texture is
        ///   transparent and `egui` blends premultiplied, so what an operator
        ///   gets is the bay's card showing through. So the order is read off
        ///   the texels the closure copies out rather than off a call count:
        ///   the canvas is already there when it records.
        /// - **It is handed the frame's own encoder.** An encoder of
        ///   `compose`'s own would be a second command buffer over a texture
        ///   the first one is still writing, whose order is the queue's
        ///   business rather than the caller's — which is ADR-0166's whole
        ///   subject and is the failure that would look correct here. The
        ///   address is what says so; pixels cannot, because two submissions
        ///   in the right order produce the right pixels.
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

            // **The sinks' own textures, held here as well.** A `wgpu::Texture`
            // is a handle, so these are the same two textures rather than
            // copies of them, and holding them is what lets the closure read
            // what the sinks were drawn into while `compose` has the sinks
            // themselves borrowed.
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
                !lit_between(&wide_seen, WIDE, 0, BAND.0)
                    && !lit_between(&wide_seen, WIDE, BAND.1, WIDE),
                "the wide sink has light outside the canvas's rectangle"
            );
        }

        /// **`finally` runs on a frame no sink took, and what it records is
        /// submitted with that frame.**
        ///
        /// The case that decides whether the parameter is the frame's or the
        /// sinks': `karakuri-console`'s panel is drawn whether or not the
        /// picture is on screen — folding the picture away hides the picture,
        /// not the console — so a `finally` conditional on a sink having
        /// answered would black the whole window the moment an operator folded
        /// one region. It is asked twice, because there are two ways to have
        /// no sink and they are different code paths: a slice with nothing in
        /// it, and a slice whose every sink refused.
        ///
        /// **What is asserted is the buffer and not the call count.** A
        /// closure that ran and was handed an encoder `compose` then dropped
        /// would satisfy "it ran" and record nothing at all, so the sentinel
        /// is written first and the clear is what has to reach it.
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
    }
}
