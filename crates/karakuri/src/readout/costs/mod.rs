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
    /// Indicates that the upcoming frame was triggered by an explicit event rather than an idle repaint.
    pub(crate) owed: bool,
    /// Metric counters accumulated during window inactivity.
    pub(crate) still: Still,
    /// Most recently completed frame cost metrics.
    pub(crate) last: Option<Cost>,
    pub(crate) said: bool,
    /// Hardware/driver adapter description string returned by wgpu.
    pub(crate) taken_on: String,
    /// True if any deck in the Program bay is actively rendering frames.
    pub(crate) live: bool,
    /// Earliest upcoming repaint deadline declared across active UI regions (ADR-0283).
    pub(crate) declared: Option<Duration>,
    /// Timestamp recorded at the start of the previous frame redraw.
    pub(crate) began: Option<Instant>,
    /// When the last audited frame was taken — see [`Costs::audit`].
    pub(crate) since_audit: Instant,
    /// Validated timestamp measurement method supported by the current GPU adapter (P-0095).
    pub(crate) clock: Option<MeasurementMethod>,
    /// The periods of the frames drawn since the last rate was taken — see [`Costs::RATE`].
    pub(crate) paced: Duration,
    /// How many frames `paced` is the periods of.
    pub(crate) paced_frames: usize,
    /// Frames a second over the last completed [`Costs::RATE`], and `None` until one has completed.
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
            // Initialize audit timer to now to avoid auditing the first frame during pipeline creation.
            since_audit: Instant::now(),
            clock: None,
            paced: Duration::ZERO,
            paced_frames: 0,
            last_rate: None,
        }
    }

    /// Interval between periodic GPU queue drain audits ([`Cost::drained`]).
    pub(crate) const AUDIT: Duration = Duration::from_millis(500);

    /// Rolling window duration used for computing the average FPS readout.
    pub(crate) const RATE: Duration = Duration::from_millis(500);

    /// Records frame start timestamp and returns the duration elapsed since the previous frame.
    pub(crate) fn tick(&mut self, at: Instant) -> Option<Duration> {
        let period = self.began.map(|began| at - began);
        self.began = Some(at);
        period
    }

    /// Returns true if enough time has elapsed to perform a periodic GPU drain audit.
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

    /// Resets the idle stillness measurement timer and counters following a window interaction.
    pub(crate) fn touched(&mut self) {
        self.quiet_since = Instant::now();
        self.still = Still::default();
    }

    /// Marks the next frame as owed to an explicit event rather than an idle repaint.
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

    /// Computes frames per second drawn while the window was untouched over `stretch` seconds.
    pub(crate) fn rate_over(&self, stretch: f64) -> f64 {
        self.still.frames as f64 / stretch
    }

    /// Returns rolling frames per second over the last completed [`Costs::RATE`] window.
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

/// Prints a GPU initialization failure message and terminates the process with exit code 1.
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
