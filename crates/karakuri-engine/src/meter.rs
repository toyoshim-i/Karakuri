//! Per-slot level metering: how much light a Set is actually putting out.
//!
//! `docs/roadmap.md`, on M2's L5 mixer:
//!
//! > Semi-automatic gain needs a measured level per Set, which is the same
//! > per-Set measurement hook the budget governor needs.
//!
//! The reason it is needed at all is that a fader means nothing if each Set
//! arrives at a different nominal level. There is a tone mapper now and the
//! artifact exposure convention has been reset to 1.0, but nothing measured
//! what a Set puts out, so an operator matching two Sets by eye was doing the
//! instrument's job for it.
//!
//! **This is the meter and nothing else. Gain stays manual.** No number here
//! moves a fader, and there is no hook for one to: an exposure that moves by
//! itself is the worst thing that can happen on stage, and the honest order is
//! to show the number first and decide later whether anything should act on it.
//! What a design for automatic gain would need — a time constant, a target
//! level, a hold, a way to be overridden mid-set — is not decided, so guessing
//! at its shape here would be a second, worse answer M2 then has to remove.
//!
//! ## Two numbers
//!
//! - **Mean luminance** over the whole frame, black included. This is the "how
//!   much light is this Set putting out" figure and it is the one to match
//!   faders on. Most of the frame is black by design and that is not a
//!   distortion to correct for — a Set that lights a tenth of the frame *is*
//!   putting out a tenth of the light, and a mean taken over "the lit part"
//!   would call a sparse Set and a dense one equally bright while the mix
//!   plainly disagreed.
//! - **Peak luminance.** The blowout warning. Two Sets can agree on mean and
//!   differ by an order of magnitude at the peak, and the one with the bright
//!   core dominates wherever it lands regardless of what the fader says. Mean
//!   alone cannot see that; the pair can.
//!
//! Luminance is **linear Rec.709** — `0.2126 R + 0.7152 G + 0.0722 B` — because
//! that is the primaries the rest of the pipeline is in, not because it is a
//! convenient average. See `shaders/meter.wgsl`.
//!
//! ## What is left out of both, and why that is not a fault indicator
//!
//! A texel whose luminance is not a finite number is counted and then excluded
//! from the sum and from the peak. [`Level::bad_texels`] is that count.
//!
//! **The exclusion is the point; the count is a footnote to it.** Admitting one
//! NaN to the sum makes the mean NaN, and the mean is the number an operator
//! sets faders by — so a single stray texel used to cost a slot its entire
//! reading. That matters because non-finite texels are *ordinary*: dividing by
//! a value that reaches zero is one of the most common things a shader does,
//! and what it usually produces is a blown-out white pixel nobody notices. A
//! measurement that a routine artifact destroys is a measurement that will be
//! missing at exactly the wrong moment.
//!
//! **So there is no health flag, no fault threshold, and nothing here says a
//! slot is broken**, deliberately. Being ordinary and mostly harmless is
//! precisely what makes a NaN count a bad proxy for "this material is wrong" —
//! a warning that fires on normal material teaches an operator to ignore
//! warnings, which is worse than not having one. That is the same argument the
//! rest of this module makes about gain, and the same one `docs/roadmap.md`
//! records against a tempo octave chosen by heuristic. The count is shown; what
//! to make of it is the operator's.
//!
//! Excluded texels divide into the mean as zero rather than being taken out of
//! the denominator, which is what black does and is the only reading that keeps
//! two slots comparable when one of them has a bad texel.
//!
//! What is measured is the slot's **own** target, before gain and before
//! opacity. That is deliberate and it is the only ordering that makes the
//! number useful: it is the level the material arrives at, which is the input
//! to setting the fader, not the output of having set it. A meter downstream of
//! the fader would read the fader.
//!
//! ## Never waits, therefore lags
//!
//! The reduction runs on the GPU, the result is copied into a staging buffer,
//! and the buffer is mapped with `map_async`. **Nothing anywhere in here waits
//! for the GPU.** [`Meters::collect`] polls with `wgpu::PollType::Poll`, which
//! processes whatever has already finished and returns, and reads whichever
//! results have arrived. The render-thread rule
//! (`docs/principles/0001-nothing-allocates-or-compiles-a-shader-on-the-render-thread.md`)
//! is absolute and
//! the rest of this engine is built around it — `swap.rs` moved both building
//! *and dropping* a Set onto a worker rather than pay a stall, and
//! `Set::live_count` is an explicitly-documented stall that nothing on the
//! frame path calls.
//!
//! So a reading is a few frames old, and that is correct rather than
//! regrettable. **The lag is not a property of this module: it is how far ahead
//! of the GPU the frame loop is running, plus one.** Measured, in
//! `tests/meter.rs`:
//!
//! | frame loop | frames behind | printed by |
//! |---|---|---|
//! | headless, `poll(Wait)` per frame — the harness's stand-in for vsync, which drains the GPU every frame | **1**, median and worst, over 120 frames | `the_lag_and_the_ring_are_measured_under_pacing` |
//! | headless, nothing pacing it at all | tens: 13 to 101, median 58, on one run of 240 | `the_meter_never_blocks_the_frame_path` |
//!
//! Both rows are printed by the test that produced them — run with
//! `--nocapture` — rather than only written down here, because neither is a
//! number this machine can promise for another one. The first is stable; the
//! second is a property of how fast the host encodes relative to the GPU and
//! moves run to run, so it is quoted as one observed run and asserted only as
//! "more than one frame behind, or a skip", which is the part that distinguishes
//! a loop that is not waiting from one that is.
//!
//! The second row is not a defect and is worth stating so it is not
//! rediscovered as one: with no vsync and no compositor, the CPU encodes frames
//! as fast as it can and runs dozens ahead of the GPU, so *every* readback of
//! any kind is dozens of frames old. A vsync-paced window sits between the two
//! rows, at whatever the driver allows in flight — expect 2 or 3, which at
//! 60 Hz is 33 to 50 ms. That is below the point at which an operator watching
//! a fader would call the meter wrong, and it is inferred from the queue depth
//! rather than measured, because measuring it needs a surface and this crate
//! tests headless.
//!
//! Every reading carries its own answer on [`Level::frames_behind`], which is
//! the only honest place for it: a number in a comment would be this machine's.
//! A meter that is a few frames behind is a meter; a meter that stalls the
//! frame is a bug.
//!
//! ## The ring
//!
//! One staging buffer would be found still mapped by the very next frame, so
//! there is a ring of [`RING`] per slot. **Four**: one per frame that can be in
//! flight — the frame being recorded plus the two a vsync-paced window will
//! typically have queued ahead of the GPU — and one spare, so that a callback
//! landing a frame later than usual costs nothing.
//!
//! A frame that finds every buffer busy **skips** its measurement rather than
//! waiting. That is the whole of the failure mode: the reading ages by a frame,
//! which is the same thing the lag already is and not a new kind of wrong.
//! [`Meters::skipped`] counts skips, so the size is checkable rather than
//! asserted — `tests/meter.rs` asserts it is zero over a paced run and prints
//! it for an unpaced one, where it was 216 of 240 on the run the table above
//! came from. That is the ring correctly declining to grow to cover a loop
//! running dozens of frames ahead of the GPU. No ring size covers that, and a
//! ring sized to try would just be holding dozens of stale readings.
//!
//! The ring is not sized for throughput either: only the newest result is ever
//! read and the rest are discarded on arrival, so a deeper ring would buy
//! nothing but more discarded readings. Each entry is 8 bytes.
//!
//! ## What an Allocated slot reads
//!
//! **Nothing — [`Meters::level`] returns `None`.** An Allocated slot renders
//! nothing, so its target still holds whatever it last drew, and reporting that
//! as a level would be presenting a stale number as a live one, which is the
//! failure mode this codebase keeps finding. Going off air therefore *retires*
//! the slot's meter: the retained reading is dropped and every measurement
//! still in flight is invalidated, so a result recorded while the slot was
//! still Live cannot arrive two frames later and resurrect a level for a slot
//! that is not producing one. Coming back on air reads `None` until a fresh
//! measurement lands, a few frames later, which is the same lag every other
//! reading has.
//!
//! A resize retires every meter for the same reason: a reading of the
//! pre-resize target is a reading of a different image. So does **a build
//! landing on a slot** — a swap installs a cold Set with `t` back at zero, a
//! rollback puts a differently-aged one back, and in both cases the reading in
//! flight measures material that is no longer on air. `deck.rs` calls
//! [`Meters::retire`] for all three, and the rule behind them is one rule: the
//! moment the meter can no longer vouch that what it measured is what the slot
//! is showing, it reports nothing rather than the last thing it knew.
//!
//! ## Cost, and how to avoid paying it
//!
//! Metering is **opt-in**: a `Deck` allocates none of this until
//! [`Deck::enable_meters`](crate::deck::Deck::enable_meters) is called, and an
//! offscreen `--render` that never calls it pays nothing at all — no
//! pipelines, no buffers, no pass. When it is on, each Live slot costs one
//! compute pass of two dispatches over its own target per frame, plus a 16-byte
//! copy. Everything allocates in [`Meters::new`]; [`Meters::record`] encodes
//! commands and nothing else, in the shape `compaction.rs` and `deck.rs`
//! established.
//!
//! ## A NaN stays inside the slot that produced it
//!
//! A generated L4 that divides by zero or takes a root of a negative is a
//! procedure that compiles and runs, and `deck.rs` goes to some length to keep
//! the NaN it produces out of the *mix*. The meter measures each slot's own
//! target, so what one slot's material does is visible in that slot's reading
//! and in no other slot's. That containment is the property worth having and it
//! is unchanged.
//!
//! **What the reading does with it has changed**, and this section used to say
//! the opposite: a NaN once landed in `mean` and made it NaN, described here as
//! reporting rather than sanitizing. It is separated out instead — counted in
//! [`Level::bad_texels`], kept out of both figures — because "reported" turned
//! out to mean "the mean is gone", and the mean is the number the fader is set
//! by. See "What is left out of both" above for why that is a footnote rather
//! than a warning.

use std::sync::mpsc::{self, Receiver, Sender};

/// Threads per workgroup, and — because the second pass consumes exactly one
/// partial per thread — the number of workgroups the first pass dispatches.
/// Templated into the shader so the two cannot drift apart.
const WORKGROUP: u32 = 64;

/// Bytes in the `Level` a reduction writes: three `f32`s and one of padding.
/// Also the size of every staging buffer, and the padding is what keeps
/// `wgpu`'s 8-byte map alignment satisfied — see the struct in the shader.
const LEVEL_SIZE: u64 = 16;

/// Staging buffers per slot. **Four**: the frame being recorded, two more that
/// may be queued ahead of the GPU, and one spare. See "The ring" in the module
/// doc for why this is not larger, and [`Meters::skipped`] for how to tell
/// whether it is large enough.
pub const RING: usize = 4;

/// One slot's measured level. Linear Rec.709 luminance, over the slot's own
/// target, before gain and opacity.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Level {
    /// Mean luminance over the whole frame, black included. The figure to
    /// match faders on.
    pub mean: f32,
    /// The brightest single texel's luminance. The blowout warning: a Set whose
    /// peak is far above the others dominates the mix wherever it lands.
    pub peak: f32,
    /// How many frames the slot has rendered since the one this reading
    /// measures. Never zero in normal use — a zero would mean the frame path
    /// waited for the GPU. Its size is how far ahead of the GPU the frame loop
    /// is running; see "Never waits, therefore lags" for what that has measured
    /// as, and why the number belongs here rather than in a comment.
    ///
    /// It grows while the reading is held, which is deliberate: a reading that
    /// stops being refreshed gets visibly older instead of looking current.
    ///
    /// Through a `Deck` it cannot be zero even in principle, and that falls out
    /// of the frame guard rather than out of arithmetic: a level is unreachable
    /// while a `Frame` is open, since the guard holds the only `&mut Deck`
    /// there is, so by the time anyone can read one the frame that recorded it
    /// has already ended. Driving [`Meters`] directly, as `tests/meter.rs` does
    /// to measure a fixed image, a zero is exactly what waiting for the GPU
    /// gets you.
    pub frames_behind: u32,
    /// **Texels this reading left out**, because their luminance was not a
    /// finite number.
    ///
    /// A NaN or an infinity, from a shader that divided by a variable that
    /// reached zero, took a root of a negative, or normalised a zero vector.
    /// Nothing in the pipeline rejects any of those and **most of them are
    /// harmless to look at**: a non-finite texel tone maps to a blown-out white
    /// pixel and the frame is otherwise what it was. This is not a fault
    /// indicator and there is deliberately no such thing here — see "What is left
    /// out of both" in the module doc.
    ///
    /// What it is for is the number beside it: `mean` and `peak` are computed
    /// over the texels this did *not* count, so one stray sprite no longer
    /// takes the whole slot's reading with it. Reported so that a mean over
    /// most of a frame is legible as one.
    pub bad_texels: u32,
}

/// What a completed measurement carried, kept until a newer one arrives.
#[derive(Debug, Clone, Copy)]
struct Reading {
    mean: f32,
    peak: f32,
    bad: u32,
    /// The slot's frame ordinal at the time it was recorded.
    frame: u64,
}

/// A staging buffer's outstanding measurement.
#[derive(Debug, Clone, Copy)]
struct Claim {
    /// Which era of this slot's meter recorded it. A result whose generation is
    /// no longer current — because the slot went off air, or was resized — is
    /// dropped on arrival rather than becoming a level.
    generation: u64,
    frame: u64,
    /// `map_async` cannot be called until the copy that fills the buffer has
    /// been submitted, so [`Meters::record`] claims and [`Meters::arm`] maps.
    armed: bool,
}

struct Staging {
    buffer: wgpu::Buffer,
    /// `None` when the buffer is free to claim.
    claim: Option<Claim>,
}

/// Everything one slot's meter owns.
struct SlotMeter {
    /// (sum, peak) per first-pass workgroup, `WORKGROUP` of them. Never read
    /// on the host; kept because a rebind has to put the same buffer back into
    /// the new bind group.
    partials: wgpu::Buffer,
    /// Where the second pass writes, and the only thing copied out.
    result: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    ring: [Staging; RING],
    /// Cloned into every `map_async` callback. The callback runs on whichever
    /// thread polls the device and may outlive the measurement's usefulness, so
    /// it sends an index and nothing else and never touches this struct.
    tx: Sender<(usize, bool)>,
    rx: Receiver<(usize, bool)>,
    /// Frames this slot has been metered for. Advances only while the slot is
    /// Live, since that is the only time a measurement is recorded, which is
    /// what makes [`Level::frames_behind`] a count of rendered frames rather
    /// than of wall-clock ones.
    frame: u64,
    generation: u64,
    last: Option<Reading>,
    skipped: u64,
}

/// One meter per deck slot, over each slot's own HDR target.
///
/// Owned by [`Deck`](crate::deck::Deck) — it is the thing that knows which
/// slots are Live, that reallocates the targets on a resize, and that has an
/// encoder open at the moment a measurement can be recorded. Constructing this
/// directly is what `tests/meter.rs` does to measure a texture of known
/// contents; there is nothing deck-specific in it.
pub struct Meters {
    layout: wgpu::BindGroupLayout,
    tiles: wgpu::ComputePipeline,
    total: wgpu::ComputePipeline,
    slots: Vec<SlotMeter>,
}

impl Meters {
    /// One meter per view, in that order.
    ///
    /// Allocates and compiles, so never on the render thread — same terms as
    /// `Deck::new`.
    pub fn new(device: &wgpu::Device, views: &[&wgpu::TextureView]) -> Meters {
        assert!(
            !views.is_empty(),
            "a meter over no targets measures nothing"
        );

        let source = include_str!("shaders/meter.wgsl").replace("{{WG}}", &WORKGROUP.to_string());
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("meter"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("meter"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        // Nothing samples: the reduction loads every texel
                        // exactly once, so there is no filter to ask for.
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                storage_entry(1),
                storage_entry(2),
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("meter"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        let pipeline = |entry_point: &str, label: &str| {
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(label),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: Some(entry_point),
                compilation_options: Default::default(),
                cache: None,
            })
        };
        // One module and one layout for every slot: the passes differ only in
        // which texture is bound, and a pipeline per slot would be four
        // compiles of one shader.
        let tiles = pipeline("reduce_tiles", "meter reduce_tiles");
        let total = pipeline("reduce_total", "meter reduce_total");

        let slots = views
            .iter()
            .enumerate()
            .map(|(i, view)| SlotMeter::new(device, &layout, i, view))
            .collect();

        Meters {
            layout,
            tiles,
            total,
            slots,
        }
    }

    pub fn slot_count(&self) -> usize {
        self.slots.len()
    }

    /// Encode one slot's measurement into `encoder`, over the target it was
    /// bound to. Call once per frame, for a slot that is rendering this frame.
    ///
    /// **Encodes commands and nothing else.** The staging buffer is taken from
    /// the ring allocated in [`Meters::new`]; if every one of them is still
    /// carrying an outstanding measurement, this frame is skipped rather than
    /// waited on, and the slot's reading ages by one frame. The frame ordinal
    /// advances either way, so a skip shows up as a larger
    /// [`Level::frames_behind`] rather than as a reading that claims to be
    /// fresher than it is.
    ///
    /// Recorded *after* the slot's own render pass in the same encoder, which
    /// is what makes it this frame's image rather than last frame's.
    pub fn record(&mut self, slot: usize, encoder: &mut wgpu::CommandEncoder) {
        let (tiles, total) = (&self.tiles, &self.total);
        let meter = &mut self.slots[slot];
        let frame = meter.frame;
        meter.frame += 1;

        let Some(index) = meter.ring.iter().position(|s| s.claim.is_none()) else {
            meter.skipped += 1;
            return;
        };

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("meter"),
                timestamp_writes: None,
            });
            pass.set_bind_group(0, &meter.bind_group, &[]);
            // Both dispatches in one pass, in this order: within a pass, a
            // dispatch sees the writes of the ones before it, which is the
            // same guarantee `compaction.rs` chains its scan levels on.
            pass.set_pipeline(tiles);
            pass.dispatch_workgroups(WORKGROUP, 1, 1);
            pass.set_pipeline(total);
            pass.dispatch_workgroups(1, 1, 1);
        }
        encoder.copy_buffer_to_buffer(&meter.result, 0, &meter.ring[index].buffer, 0, LEVEL_SIZE);
        meter.ring[index].claim = Some(Claim {
            generation: meter.generation,
            frame,
            armed: false,
        });
    }

    /// Map the buffers this frame claimed. **Call immediately after the
    /// frame's `queue.submit`, and only then.**
    ///
    /// This cannot live in [`Meters::record`]: `map_async` resolves against the
    /// submissions outstanding when it is called, so arming a buffer before the
    /// copy that fills it has been submitted would fire the callback on
    /// whatever the buffer happened to hold — a reading from three frames ago,
    /// presented as this one's. Arming after the submit costs nothing and is
    /// the only ordering that is correct.
    ///
    /// Does not wait: `map_async` returns immediately and the callback fires
    /// on whoever next polls the device. It does not allocate anything on the
    /// *GPU* — the buffers are all from [`Meters::new`] — but it is not free
    /// of host allocation and saying so would be wrong: `map_async` boxes the
    /// callback it is handed, so this is one small `Box` per Live slot per
    /// frame. That is the same order of cost as the `Vec` the composite's
    /// events drain into, and it is named here rather than claimed away.
    pub fn arm(&mut self) {
        for meter in &mut self.slots {
            let tx = &meter.tx;
            for (index, staging) in meter.ring.iter_mut().enumerate() {
                if let Some(claim) = &mut staging.claim {
                    if !claim.armed {
                        claim.armed = true;
                        let tx = tx.clone();
                        staging
                            .buffer
                            .slice(..)
                            .map_async(wgpu::MapMode::Read, move |r| {
                                let _ = tx.send((index, r.is_ok()));
                            });
                    }
                }
            }
        }
    }

    /// Take delivery of whatever has arrived, and free the buffers it came in.
    ///
    /// **Never waits.** `wgpu::PollType::Poll` processes the submissions that
    /// have already finished and returns; it is not the `poll(Wait)` the render
    /// thread must never do. Nothing here depends on a result being ready — if
    /// none is, this does nothing at all and the previous reading stands, one
    /// frame older.
    ///
    /// Results that arrive out of order, or from before a retirement, are
    /// discarded rather than installed: the retained reading only ever moves
    /// forward in time.
    pub fn collect(&mut self, device: &wgpu::Device) {
        // One poll for the device, not one per slot. A `Poll` that finds
        // nothing finished is a cheap no-op. Its error is dropped rather than
        // unwrapped: the only way it fails is a lost device, which the frame's
        // own submit reports far more usefully than a panic from inside a
        // meter would.
        let _ = device.poll(wgpu::PollType::Poll);
        for meter in &mut self.slots {
            while let Ok((index, mapped)) = meter.rx.try_recv() {
                let claim = meter.ring[index]
                    .claim
                    .take()
                    .expect("a map callback for a staging buffer nothing claimed");
                if !mapped {
                    // The map failed, so there is nothing mapped to unmap and
                    // nothing to read. The buffer is free again; the reading
                    // this frame would have produced is simply missing, which
                    // is the same shape as a skip.
                    continue;
                }
                let staging = &meter.ring[index].buffer;
                let data = staging.slice(..).get_mapped_range();
                let mean = f32::from_le_bytes(data[0..4].try_into().expect("4-byte chunk"));
                let peak = f32::from_le_bytes(data[4..8].try_into().expect("4-byte chunk"));
                let bad = f32::from_le_bytes(data[8..12].try_into().expect("4-byte chunk"));
                drop(data);
                staging.unmap();
                // The shader counts in `f32` because the whole reduction is
                // one. Integral and exact to 2^24 — 16.7 million texels, which
                // is every frame this renders short of a 6K display, where the
                // count starts rounding to even. That is a count nothing acts
                // on, so rounding it is a cost worth the reduction staying one
                // type.
                let bad = bad as u32;

                let current = claim.generation == meter.generation;
                let newer = meter.last.is_none_or(|last| claim.frame >= last.frame);
                if current && newer {
                    meter.last = Some(Reading {
                        mean,
                        peak,
                        bad,
                        frame: claim.frame,
                    });
                }
            }
        }
    }

    /// The most recent level that has arrived for a slot, or `None` if none
    /// has — because the slot is not being metered (see [`Meters::retire`]),
    /// because it has only just started being, or because nothing has come back
    /// yet.
    ///
    /// A pure read: it never waits and never installs anything. Whatever
    /// [`Meters::collect`] last took delivery of is what this returns, aged.
    pub fn level(&self, slot: usize) -> Option<Level> {
        let meter = &self.slots[slot];
        meter.last.map(|reading| Level {
            mean: reading.mean,
            peak: reading.peak,
            bad_texels: reading.bad,
            // `frame` is the next ordinal, so the last one recorded is
            // `frame - 1`; a reading is at least one frame behind that by
            // construction, since the frame it was recorded in had not been
            // submitted when it was.
            frames_behind: (meter.frame.saturating_sub(1) - reading.frame) as u32,
        })
    }

    /// Stop reporting a level for a slot, and disown every measurement of it
    /// still in flight.
    ///
    /// Called when a slot goes off air, when its target is reallocated, when a
    /// build lands on it, and at both ends of an audition. All of them are the
    /// same problem: whatever arrives next was measured on an image that is no
    /// longer what the slot is showing, and a stale number presented as a live
    /// one is worse than no number. The buffers those measurements are travelling in come back to
    /// the ring as usual; only their contents are dropped.
    pub fn retire(&mut self, slot: usize) {
        let meter = &mut self.slots[slot];
        meter.generation += 1;
        meter.last = None;
    }

    /// Frames whose measurement was skipped because every staging buffer was
    /// still in flight. This is how [`RING`] is checked rather than asserted;
    /// see "The ring" in the module doc.
    pub fn skipped(&self, slot: usize) -> u64 {
        self.slots[slot].skipped
    }

    /// Point the meters at new views — a resize. Retires every slot, since a
    /// measurement of the old target says nothing about the new one.
    ///
    /// Allocates bind groups, so never on the render thread.
    pub fn rebind(&mut self, device: &wgpu::Device, views: &[&wgpu::TextureView]) {
        assert_eq!(
            views.len(),
            self.slots.len(),
            "a rebind cannot change how many slots are metered"
        );
        for (slot, view) in views.iter().enumerate() {
            self.slots[slot].bind_group = SlotMeter::bind(
                device,
                &self.layout,
                slot,
                view,
                &self.slots[slot].partials,
                &self.slots[slot].result,
            );
            self.retire(slot);
        }
    }
}

impl SlotMeter {
    fn new(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        slot: usize,
        view: &wgpu::TextureView,
    ) -> SlotMeter {
        let partials = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("meter {slot} partials")),
            // `vec3<f32>` per workgroup, which WGSL lays out at a stride of
            // 16 rather than 12: an array element is aligned to the type's
            // alignment and `vec3` is aligned as a `vec4`.
            size: u64::from(WORKGROUP) * 16,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let result = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("meter {slot} level")),
            size: LEVEL_SIZE,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let bind_group = SlotMeter::bind(device, layout, slot, view, &partials, &result);
        let ring = std::array::from_fn(|i| Staging {
            buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(&format!("meter {slot} staging {i}")),
                size: LEVEL_SIZE,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            }),
            claim: None,
        });
        let (tx, rx) = mpsc::channel();

        SlotMeter {
            partials,
            result,
            bind_group,
            ring,
            tx,
            rx,
            frame: 0,
            generation: 0,
            last: None,
            skipped: 0,
        }
    }

    fn bind(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        slot: usize,
        view: &wgpu::TextureView,
        partials: &wgpu::Buffer,
        result: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&format!("meter {slot}")),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: partials.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: result.as_entire_binding(),
                },
            ],
        })
    }
}

fn storage_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: false },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}
