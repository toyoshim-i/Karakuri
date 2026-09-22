//! Cost tracking, still-panel budget measurement, allocator counting, and drift checking.

use std::time::{Duration, Instant};

use karakuri_engine::probe::MeasurementMethod;

pub mod entry;
pub mod report;

pub(crate) use entry::*;

pub(crate) struct Costs {
    /// What the frames that *were* asked for cost.
    pub(crate) frames: Vec<Cost>,
    /// Every frame drawn, including any past the end of `frames`.
    pub(crate) drawn: usize,
    /// When the window was last touched by anything at all.
    pub(crate) quiet_since: Instant,
    /// A frame is owed to something that happened, and the next one drawn is that
    /// frame rather than a frame drawn on a still panel.
    ///
    /// Without it the reading blames the panel for the frame it was asked for: an
    /// event resets the stretch and the frame it asked for lands a millisecond into
    /// the new one, so a window that did exactly the right thing reports having
    /// drawn on an untouched panel.
    pub(crate) owed: bool,
    /// What has been drawn since `quiet_since` that nothing asked for.
    pub(crate) still: Still,
    /// The frame just drawn, kept so the transport row can print what it cost.
    ///
    /// Not a second measurement and not a second sample: it is the last [`Cost`]
    /// [`Costs::push`] was handed, which `frames` stops keeping after [`SAMPLE`] of
    /// them because that vector is a sample rather than a history. A readout wants
    /// the last frame and the reading wants the sample; both are the same numbers.
    pub(crate) last: Option<Cost>,
    pub(crate) said: bool,
    /// What ran it, as the adapter reports it: the backend, the adapter's own name,
    /// and whether it calls itself discrete.
    ///
    /// Printed because a readout that does not name its backend is how a
    /// measurement gets written into a table as though it were another one's.
    /// `WGPU_BACKEND` was honoured by nothing for months and the failure was
    /// invisible precisely here — twenty-five runs were recorded as DX12 and were
    /// Vulkan, and nothing on screen could have said otherwise
    /// (`docs/adr/0168-a-backend-override-is-honoured-because-a-no-op-cannot-be-caught.md`).
    /// The adapter is asked rather than the environment, so an override that does
    /// not take reads as what it is.
    pub(crate) taken_on: String,
    /// Whether anything in the Program bay is making texels — the picture, a deck
    /// auditioning in a preview cell, or both. [`live`] is the rule and this is the
    /// frame's answer to it.
    ///
    /// It decides which sentence the reading prints about the frames it counted,
    /// and nothing else: the frames are counted the same way either way, which is
    /// the point — the number is not adjusted for knowing the answer.
    pub(crate) live: bool,
    /// What the panel declared it needed, as `View::animating` answered it on the
    /// last frame: the soonest *move* out of the regions that are declaring, and
    /// `None` only when none of them is.
    ///
    /// A deadline and not a rate, which is ADR-0283: a region declares its
    /// staleness for the motion it has, so a pending region that is at rest answers
    /// the rest it has left rather than a rate it is not using. The beat is the
    /// exception and it is the interesting one — the light travels on every frame
    /// the session advances, so its deadline is its declared staleness on every
    /// frame there is, and `24.671ms` here is the beat and nothing else.
    ///
    /// It is here for one sentence, and the sentence was wrong without it. With
    /// nothing in the Program bay making texels the reading used to blame the only
    /// other thing it knew about — an `egui` repaint delay answered immediately —
    /// and on this program that is never the answer: folding the picture and the
    /// preview row away leaves the window drawing 28.0 to 28.3 frames a second over
    /// two runs on 2026-08-26, which is the roll's declared 30 Hz and not a
    /// mishandled delay. Folding the mixer bay away as well takes it to 0 frames,
    /// because the chip that declares the 30 Hz is then not laid out (ADR-0193). A
    /// reading that names the wrong cause is worse than one that names none.
    ///
    /// Those two readings were taken before the beat declared, and the beat
    /// declares whenever the transport row is drawn and there is an engine behind
    /// it — 24.7 ms, about forty a second, whether or not anything is pending
    /// ([ADR-0212](../../../docs/adr/0212-the-beat-is-a-light-that-travels-and-it-declares-for-itself.md),
    /// [P-0094](../../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)).
    /// So this window's `None` now needs the transport row folded away as well,
    /// which is a fourth fold and is the arm below saying so.
    ///
    /// And the roll's 30 Hz is no longer 30 Hz, which is what would make the
    /// 28.0-to-28.3 reading above unreproducible if it were taken again: with the
    /// transport row folded and a slot parked, the mixer asks for fourteen frames a
    /// second rather than thirty-one, because the seventeen that fell inside the
    /// roll's rest drew the chip in the position it was already in
    /// ([ADR-0283](../../../docs/adr/0283-a-region-declares-when-its-picture-next-changes-not-that-something-is-pending.md),
    /// `karakuri-console`'s `tests/moving.rs`, which counts both).
    pub(crate) declared: Option<Duration>,
    /// The top of the last frame, which is the one anchor a period is measured from
    /// — see [`Cost::period`] and [`Costs::tick`].
    ///
    /// It is the *same point* of every redraw, so two consecutive values bracket a
    /// whole frame with no gap and no overlap. `None` before the first one.
    pub(crate) began: Option<Instant>,
    /// When the last audited frame was taken — see [`Costs::audit`].
    pub(crate) since_audit: Instant,
    /// What this program was able to establish about this adapter's GPU timestamps,
    /// as the deck's own probe earned it: `GpuTimestamp` where calibration survived
    /// and nothing later caught it lying, `HostWallClock` where it did not, and
    /// `None` where nothing has been probed at all.
    ///
    /// It is not `Features::TIMESTAMP_QUERY`, and the difference is the whole of
    /// P-0095: the machine this was written on advertises the feature and does not
    /// deliver it. The reading prints this beside the frame figures so that *why is
    /// this a host clock* has an answer taken against a load rather than read off a
    /// flag — and prints `None` as *nothing asked* rather than as a verdict.
    pub(crate) clock: Option<MeasurementMethod>,
    /// The periods of the frames drawn since the last rate was taken — see
    /// [`Costs::RATE`].
    ///
    /// Every drawn frame's, owed or not; [`Costs::touched`] does not clear it.
    pub(crate) paced: Duration,
    /// How many frames `paced` is the periods of.
    pub(crate) paced_frames: usize,
    /// Frames a second over the last completed [`Costs::RATE`], and `None` until
    /// one has completed.
    pub(crate) last_rate: Option<f64>,
}

impl Costs {
    pub(crate) fn new() -> Costs {
        Costs {
            frames: Vec::with_capacity(SAMPLE),
            drawn: 0,
            quiet_since: Instant::now(),
            owed: false,
            still: Still::default(),
            last: None,
            said: false,
            taken_on: String::from("an adapter nobody asked"),
            live: false,
            declared: None,
            began: None,
            // **Now rather than the beginning of time**, so the first frame of
            // a run is not the audited one: it builds the font atlas and pays
            // for every pipeline the panel touches, and blocking on the GPU
            // there would measure a frame nothing else in this reading is
            // about.
            since_audit: Instant::now(),
            clock: None,
            paced: Duration::ZERO,
            paced_frames: 0,
            last_rate: None,
        }
    }

    /// How rarely a frame is audited — the frame that blocks on `Device::poll` to
    /// find out what the GPU still owed ([`Cost::drained`]).
    ///
    /// A stretch of wall clock and not a frame count, and the reason is the case
    /// this instrument exists for. Every so many frames sounds steadier and is
    /// exactly backwards: a loop at four frames a second is the loop that most
    /// needs the GPU's own number, and one audit in sixty frames would take fifteen
    /// seconds to produce one — five times the [`STILL`] the reading is taken over,
    /// so the reading would carry no GPU figure precisely where it is the whole
    /// answer. Half a second gives six samples over a three-second reading whether
    /// the window is drawing at four frames a second or at sixty.
    ///
    /// It costs least where it fires most often, which is the same argument from
    /// the other side: a frame that is already waiting on the GPU pays almost
    /// nothing to be told how long, because the wait is one it was about to take in
    /// `get_current_texture` anyway.
    ///
    /// The cost of it is printed rather than assumed. The reading prints what the
    /// audited frames' periods were against the rest, so a machine where this is
    /// not cheap says so on that machine instead of being reassured by a sentence
    /// written on another one.
    pub(crate) const AUDIT: Duration = Duration::from_millis(500);

    /// How much drawn time the transport row's rate is taken over — see
    /// [`Costs::rate`].
    ///
    /// Summed from [`Cost::period`], so the stretch covers drawn frames only.
    pub(crate) const RATE: Duration = Duration::from_millis(500);

    /// The top of a frame: store the anchor and hand back the interval since the
    /// previous one, which is [`Cost::period`].
    ///
    /// Called from one place, at the same statement of every redraw, because that
    /// is what makes two consecutive anchors a whole frame. A redraw that returns
    /// early — a surface to reconfigure, a frame to skip — still ticks here, so the
    /// frame after one of those carries a period spanning both; they are rare, they
    /// show up in the tail rather than the median, and the alternative is an anchor
    /// that moves depending on which branch a frame took.
    pub(crate) fn tick(&mut self, at: Instant) -> Option<Duration> {
        let period = self.began.map(|began| at - began);
        self.began = Some(at);
        period
    }

    /// Whether this frame pays for the GPU's answer — see [`Cost::drained`] and
    /// [`Costs::AUDIT`].
    ///
    /// Reads the clock rather than counting frames, so that the rate the answer
    /// arrives at does not fall away exactly as the frames get slower. It is asked
    /// once per frame and it is the ask that resets the stretch, so a caller that
    /// asks and then declines to measure has thrown a sample away rather than
    /// deferred one.
    pub(crate) fn audit(&mut self) -> bool {
        let now = Instant::now();
        match now.duration_since(self.since_audit) >= Costs::AUDIT {
            true => {
                self.since_audit = now;
                true
            }
            false => false,
        }
    }

    /// Something touched the window, so the stillness starts again from here and
    /// what was drawn during the last stretch is no longer about a still panel.
    ///
    /// Every window event that is not a frame this loop asked for itself counts,
    /// including the ones the operator did not cause — a move, a focus, an
    /// occlusion. Resetting too eagerly only ever makes the reading harder to
    /// reach, never easier to pass.
    pub(crate) fn touched(&mut self) {
        self.quiet_since = Instant::now();
        self.still = Still::default();
    }

    /// A frame was asked for by something that happened, so the next one drawn is
    /// not on the panel's account.
    ///
    /// Every `request_redraw` in this file goes through here except one, and the
    /// exception is the point of the reading: the frame `egui` asked for after a
    /// delay it named. The distinction is between `egui` saying *I have not
    /// finished drawing what you just asked me to* — a `repaint_delay` of zero, a
    /// second pass of a frame already owed, which is what it does for a pass or two
    /// while the font atlas settles — and `egui` saying *wake me in 250 ms*, which
    /// is an animation and is per-frame work on a panel nobody is touching. The
    /// first goes through here and the second does not, so a delay mishandled into
    /// a spin shows in the reading as the frames it actually drew.
    ///
    /// The other way it can fail is loud rather than quiet: something asking for
    /// frames without pause never lets the window be still for [`STILL`], and then
    /// no reading is printed at all. A run of this program that prints no reading
    /// is that failure.
    pub(crate) fn owes(&mut self) {
        self.owed = true;
        self.touched();
    }

    pub(crate) fn push(&mut self, cost: Cost) {
        self.drawn += 1;
        self.last = Some(cost);
        if !self.owed {
            self.still.frames += 1;
            self.still.allocs += cost.allocs;
            self.still.bytes += cost.bytes;
        }
        self.owed = false;
        if let Some(period) = cost.period {
            self.paced += period;
            self.paced_frames += 1;
            if self.paced >= Costs::RATE {
                self.last_rate = Some(self.paced_frames as f64 / self.paced.as_secs_f64());
                self.paced = Duration::ZERO;
                self.paced_frames = 0;
            }
        }
        if self.frames.len() < SAMPLE {
            self.frames.push(cost);
        }
    }

    /// Frames a second: what was drawn on the untouched window, over the stretch it
    /// was drawn in.
    ///
    /// The stretch is the caller's: [`Costs::say`] is taken exactly [`STILL`] after
    /// `quiet_since` and says so in the sentence above the number.
    ///
    /// Only frames drawn on a window nobody touched are counted, so this is zero
    /// while a pointer moves over the window. The transport row reads
    /// [`Costs::rate`] instead.
    pub(crate) fn rate_over(&self, stretch: f64) -> f64 {
        self.still.frames as f64 / stretch
    }

    /// Frames a second for the transport row: the frames drawn over the last
    /// [`Costs::RATE`] of their own periods, or `None` before the first such
    /// stretch has been drawn.
    ///
    /// Every drawn frame counts, owed or not, and touching the window does not
    /// reset it, so the figure holds while the pointer moves. Nothing new is
    /// timed: the periods are the ones [`Costs::tick`] already hands each frame.
    ///
    /// It holds its last value while no frame is pushed, so the transport row
    /// asks it only while something is live (ADR-0177).
    pub(crate) fn rate(&self) -> Option<f64> {
        self.last_rate
    }

    /// When the reading is due, and `None` once it has been taken. It is also what
    /// keeps the loop on a deadline until then — see [`App::about_to_wait`].
    pub(crate) fn due(&self) -> Option<Instant> {
        match self.said {
            true => None,
            false => Some(self.quiet_since + STILL),
        }
    }
}

/// Say what failed, say whether an override is the likely reason, and stop.
///
/// Never returns, so it can stand where a `?` cannot: this is inside a `winit`
/// callback, where a panic aborts rather than unwinds and takes the message
/// with it.
pub(crate) fn no_gpu(what: &str) -> ! {
    eprintln!("{what}");
    match std::env::var("WGPU_BACKEND") {
        Ok(want) => eprintln!(
            "WGPU_BACKEND is set to `{want}`, and this codebase honours it — so the most likely \
             reason is that this machine has no {want}. Unset it to take whatever the machine \
             offers."
        ),
        Err(_) => eprintln!(
            "WGPU_BACKEND is not set, so this is the machine's own default backend failing \
             rather than an override."
        ),
    }
    std::process::exit(1)
}

pub(crate) fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}
