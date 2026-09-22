//! Tempo and beat-phase estimation from the analyser's novelty curve.
//!
//! **This estimates; it does not decide.** What the oscillator is told is
//! [`crate::lock`]'s business, and the split matters: an estimator that also
//! steered would have to be tuned for agility, and the thing being built here
//! wants to be tuned for *accuracy while the tempo holds*, which is where
//! essentially all of a set is spent.
//!
//! Pure, and no clock: a [`Tracker`] is a function of the novelty samples
//! pushed into it and of the window centre it was given. Its notion of time is
//! **stream seconds** — samples consumed divided by the sample rate — so a test
//! can drive it at any speed and a device can drive it at one.
//!
//! ## How the estimate is made
//!
//! One window of novelty (about [`WINDOW_SECONDS`] of it), and three questions
//! asked of it:
//!
//! - **Period**, by autocorrelation over the lags inside [`BPM_RANGE`]. A
//!   periodic novelty curve peaks at *every* multiple of its period, so this
//!   answers "the music repeats at some multiple of this" and nothing more.
//! - **Octave**, by [`fold`]: the period found is multiplied or divided by two
//!   until it lands inside the tracking window. See below — this is the part
//!   that used to be a judgement and is now arithmetic.
//! - **Phase and fit**, by folding the whole novelty window onto the settled
//!   period and looking at where the mass piles up. The strongest position is
//!   the beat; how much of the mass sits there is the confidence.
//!
//! ## The tracking window
//!
//! [`tracking_window`] is **exactly one octave wide**, centred on the tempo
//! being tracked: `[centre / √2, centre * √2]`. Every candidate period folds
//! into it, and one octave is the widest a window can be and still admit
//! *exactly one* fold of any candidate — the octaves of a one-octave window
//! tile the whole tempo axis without overlapping and without leaving a gap. A
//! wider window is the intuitive choice and it is the wrong one: at 1.5 octaves
//! a true tempo `T` and its double `2T` are both inside, and choosing between
//! them is the ambiguity this design exists to remove. A narrower one leaves
//! tempi that fold to nothing.
//!
//! **The centre moves, and that is the whole idea.** It is the grid's current
//! tempo, pushed in by whoever owns the grid ([`Tracker::set_centre_bpm`]), so
//! a tempo that drifts is followed across an octave boundary without anything
//! ever having to decide anything: by the time the music reaches the old
//! boundary the boundary has moved with it. Before there is a lock there is no
//! grid tempo to speak of, so the centre starts at the **session tempo** — the
//! `--bpm` the operator already gives. One number now does two jobs, and it is
//! the right one for both: it is where the oscillator free-runs and it is where
//! the tracker starts looking.
//!
//! ## What the operator has the last word on
//!
//! If the window starts centred an octave off — the operator typed 87 for a
//! track that is 174 — everything locks an octave low and **nothing automatic
//! will fix it, by construction**. That is the intended trade, and the escape
//! hatch is manual: the ×2 and ÷2 controls move the grid *and* the window
//! together, so tracking continues in the new octave rather than folding
//! straight back. It is the same division of labour [`crate::lock`] already
//! uses — a stiff grid that is hard to disturb, plus a way for a person to say
//! what a machine cannot infer. A track change that crosses an octave is
//! exactly that case: musically it is genuinely ambiguous, and an operator can
//! hear which one is right inside a bar.
//!
//! [`Estimate::half_tempo_hint`] is the only thing left of the old automatic
//! decision, and it is a **note to that operator, not an input to the grid**:
//! it says there is nearly as much novelty between the grid points as on them,
//! which is what a grid running at half the music's tempo looks like.
//!
//! ## Why this replaced two heuristics rather than repairing them
//!
//! The octave used to be settled by two measurements taken after the peak was
//! picked: halve if the odd grid points were weak against the even ones,
//! double if the novelty between the grid points was nearly as strong as on
//! them. Both were broken in ways that were hard to see, and the second was
//! broken *by exactly the condition it existed to catch* — its phase came from
//! a single DFT bin at `1 / period`, and at twice the real period consecutive
//! pulses land half a turn apart in that bin and cancel, so the check switched
//! itself off precisely where it was needed and everything above about 160 bpm
//! settled at half tempo. It read as "no beat here" rather than as a bug,
//! because an octave error collapsed the confidence too.
//!
//! Both were fixed; both fixes were load-bearing in ways that were themselves
//! fragile. That is the argument for this design rather than a third repair:
//! **the octave is a decision arithmetic can make, and signal processing kept
//! getting wrong.** Nothing here judges an octave, so nothing here can judge
//! one wrongly — the failure that is left is the operator's window being
//! centred wrong, which is visible, stable, and fixable from a key.
//!
//! ## Triplets are not octaves
//!
//! Folding by powers of two fixes 2:1 errors and only those. A **3:2 error** —
//! a triplet feel read as two thirds of the true tempo, or a half-time shuffle
//! read as three halves of it — is not touched by any of this: it lands inside
//! the window looking exactly like a correct answer, and no test here pretends
//! otherwise. The prior ([`PRIOR_OCTAVES`]) keeps a *third* of the true period
//! from winning the peak search, which is the only part of the problem that is
//! cheap; the rest is deliberately not attempted.

use std::ops::RangeInclusive;

/// How much novelty history an estimate is made from. Long, because tempo
/// accuracy is the goal and a longer window resolves the period more finely:
/// eight seconds is sixteen beats at 120 bpm, enough that a period error of
/// half a percent is visible in the correlation. It is also why the estimate
/// is slow to react to a change, which is the trade this design is choosing on
/// purpose — a tempo step happens a few times an hour, inside a blend where
/// there are two tempi in the room and no correct answer.
pub const WINDOW_SECONDS: f32 = 8.0;

/// The range of musical tempos searched during lag autocorrelation.
///
/// Lags outside this range are folded into the active tracking window.
pub const BPM_RANGE: RangeInclusive<f32> = 60.0..=200.0;

/// Width of the Gaussian prior weighting applied around the window centre in octaves.
///
/// Suppresses non-octave multiples (such as 3:2 triplet harmonics) during peak selection.
pub const PRIOR_OCTAVES: f32 = 0.65;

/// The normalised autocorrelation at which a period is believed completely.
/// Half: a click train reaches about 0.6 and music with a mixed kit reaches
/// less, so demanding more would mean never being confident about real
/// material.
const PERIODICITY_FULL: f32 = 0.5;

/// How near a grid point novelty has to sit to count as being on it, as a
/// fraction of a beat either side. 6% of a beat is 28 ms at 128 bpm — wide
/// enough for an attack plus the hop grid's own quantisation, narrow enough
/// that an offbeat is nowhere near it.
const ON_GRID_BEATS: f32 = 0.06;

/// The share of the novelty sitting on the grid at which the fit is believed
/// completely. Half, for [`PERIODICITY_FULL`]'s reason: a click train puts
/// nearly all of its mass on the grid and a mixed kit with sustained material
/// between the beats never will, so demanding more would mean never being
/// confident about music.
const ON_GRID_FULL: f32 = 0.5;

/// Ratio of off-grid to on-grid novelty above which the operator is told the
/// grid might be at half the music's tempo. High, because this note costs
/// nothing when it is right and costs trust when it is wrong: a kick with a
/// loud offbeat hat sits near 0.5, and a grid genuinely an octave low sits
/// near 1.0.
const HALF_TEMPO_HINT: f32 = 0.7;

/// How often an estimate is recomputed, in seconds of stream time. The window
/// is eight seconds long, so recomputing faster than this changes almost
/// nothing and costs an autocorrelation.
const ESTIMATE_INTERVAL_SECONDS: f32 = 0.25;

/// The tempi that fold to themselves: one octave wide, centred on `centre_bpm`.
///
/// The centre is clamped into [`BPM_RANGE`] first, so a window always overlaps
/// the lags that are actually searched. See the module doc for why one octave
/// is a maximum rather than a starting point.
pub fn tracking_window(centre_bpm: f32) -> RangeInclusive<f32> {
    let centre = centre(centre_bpm);
    centre / std::f32::consts::SQRT_2..=centre * std::f32::consts::SQRT_2
}

/// Multiplies or divides `bpm` by two until it falls within the octave window centred on `centre_bpm`.
///
/// Idempotent and exact using power-of-two factors.
pub fn fold(bpm: f32, centre_bpm: f32) -> f32 {
    let centre = centre(centre_bpm);
    if !bpm.is_finite() || bpm <= 0.0 {
        // A tempo that is not a tempo folds to the centre rather than to
        // itself: the one thing every caller is entitled to assume is that the
        // answer is inside the window.
        return centre;
    }
    let octaves = (bpm / centre).log2();
    bpm * (-octaves.round()).exp2()
}

/// A usable window centre: the caller's, clamped into [`BPM_RANGE`], with a
/// NaN or a nonsense tempo becoming the middle of it rather than poisoning
/// every fold from here on.
fn centre(centre_bpm: f32) -> f32 {
    if centre_bpm.is_finite() && centre_bpm > 0.0 {
        centre_bpm.clamp(*BPM_RANGE.start(), *BPM_RANGE.end())
    } else {
        (*BPM_RANGE.start() * *BPM_RANGE.end()).sqrt()
    }
}

/// What the tracker believes about the music, and how much.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Estimate {
    pub bpm: f32,
    /// Beat phase at [`Estimate::at`]: 0.0 is on the beat, rising to 1.0 at the
    /// next one.
    pub phase: f32,
    /// `[0, 1]`. **How well this grid fits the novelty, and nothing else.**
    /// Zero means "no idea", and everything downstream is required to treat
    /// that as "leave the oscillator alone" rather than "the tempo is zero".
    ///
    /// It carries no octave uncertainty, because after [`fold`] there is none:
    /// a grid at half the music's tempo fits every other pulse and says so
    /// honestly, rather than collapsing to zero and reading as an absence.
    pub confidence: f32,
    /// The instant `phase` describes, in **stream seconds** — the middle of the
    /// newest analysis window, which is the newest moment the tracker can say
    /// anything about. Everything that turns this into a correction has to
    /// account for how old it is by then; see [`crate::lock`].
    pub at: f64,
    /// How many times the window has been *re-measured*. Between measurements
    /// the estimate is extrapolated forward, which changes `at` and `phase`
    /// without being new evidence — and a controller that counted extrapolated
    /// estimates as evidence would reach any threshold in a few frames. This is
    /// what lets it count opinions rather than frames.
    pub revision: u64,
    /// Indicates significant novelty between grid points, suggesting possible 2x tempo.
    /// Advisory diagnostic for display; not used internally to alter tracking state.
    pub half_tempo_hint: bool,
}

impl Estimate {
    /// An estimate that says nothing. Not an `Option`, for the same reason the
    /// signal bus is not: a consumer branching on presence is a consumer that
    /// will forget one of the two branches.
    pub fn unknown(at: f64) -> Estimate {
        Estimate {
            bpm: 0.0,
            phase: 0.0,
            confidence: 0.0,
            at,
            revision: 0,
            half_tempo_hint: false,
        }
    }

    /// Where this grid says the phase will be `ahead` seconds after the instant
    /// it describes.
    ///
    /// This is the form a correction uses, and the reason it takes a duration
    /// rather than an instant is that the duration is the sum of things the
    /// caller knows and this does not: how old the estimate is by now, and how
    /// long it will be until what is drawn now is light. See [`crate::lock`].
    pub fn phase_ahead(&self, ahead: f32) -> f32 {
        let beats = ahead as f64 * self.bpm as f64 / 60.0;
        (self.phase as f64 + beats).rem_euclid(1.0) as f32
    }

    /// Where the beat grid this estimate describes says the phase will be at
    /// stream time `when` — the extrapolation the whole design rests on. While
    /// the tempo holds this is simply correct, and no per-beat chasing is
    /// needed to keep it so.
    pub fn phase_at(&self, when: f64) -> f32 {
        let beats = (when - self.at) * self.bpm as f64 / 60.0;
        (self.phase as f64 + beats).rem_euclid(1.0) as f32
    }
}

/// Estimates tempo and beat phase from a novelty curve.
pub struct Tracker {
    hop_seconds: f32,
    /// Novelty, oldest first, capped at [`WINDOW_SECONDS`]. A `VecDeque` would
    /// be the obvious shape; this is a ring in a `Vec` because the whole window
    /// is walked in order on every estimate and a contiguous copy is what the
    /// autocorrelation wants.
    history: Vec<f32>,
    write: usize,
    filled: usize,
    /// Scratch for the linearised window. Allocated once: an estimate can be
    /// computed on the audio thread.
    window: Vec<f32>,
    /// Scratch for the novelty folded onto one period, one bin per hop. Sized
    /// for the longest period an answer can have, and allocated with
    /// everything else.
    profile: Vec<f32>,
    /// The tracking window's centre — the grid's tempo, or the session tempo
    /// until there is a grid. See the module doc.
    centre_bpm: f32,
    /// Stream seconds at the centre of the newest window pushed.
    at: f64,
    /// Stream seconds when the cached estimate was made.
    estimated_at: f64,
    estimate: Estimate,
    revision: u64,
    lag_range: (usize, usize),
}

impl Tracker {
    /// `centre_bpm` is where the tracking window starts: the session tempo,
    /// which is `--bpm` and is also what the oscillator free-runs at. It moves
    /// from there with [`Tracker::set_centre_bpm`].
    pub fn new(hop_seconds: f32, window_lag: f32, centre_bpm: f32) -> Tracker {
        let hop_seconds = hop_seconds.max(1e-6);
        let len = (WINDOW_SECONDS / hop_seconds).ceil() as usize;
        // A lag is a whole number of hops, so the tempo resolution at the fast
        // end is coarser than at the slow end — 200 bpm is 28 hops at 48 kHz
        // and one hop either way is 3.6%, against 1.1% at 60 bpm. The octave
        // ladder and the parabola in `estimate_now` are both there for that.
        let slowest = (60.0 / BPM_RANGE.start() / hop_seconds).round() as usize;
        let fastest = (60.0 / BPM_RANGE.end() / hop_seconds).round() as usize;
        Tracker {
            hop_seconds,
            history: vec![0.0; len],
            write: 0,
            filled: 0,
            window: vec![0.0; len],
            // A folded period can be an octave below the slowest lag searched,
            // when the window centre is at the bottom of the range; the whole
            // window is the honest bound and costs a few kilobytes.
            profile: vec![0.0; len],
            centre_bpm: centre(centre_bpm),
            // `at` is the centre of the newest window pushed, in stream
            // seconds. The first block covers samples `0..BLOCK`, so its centre
            // is one `window_lag` in; starting a hop before that means the
            // first `push` lands exactly there.
            at: f64::from(window_lag - hop_seconds),
            estimated_at: f64::NEG_INFINITY,
            estimate: Estimate::unknown(0.0),
            revision: 0,
            lag_range: (fastest.max(2), slowest.min(len / 2)),
        }
    }

    /// Move the tracking window. **This is what makes the octave follow the
    /// music**: the caller passes the grid's current tempo, so the window is
    /// always centred on what is being tracked and a drifting tempo never
    /// reaches a boundary. It is also how the ×2 and ÷2 controls take effect —
    /// they move the grid, and the window comes with it.
    ///
    /// Takes effect at the next re-measurement, a quarter second at most. The
    /// cached estimate is left alone: it describes a window of novelty that was
    /// measured under the old centre, and re-folding it here would relabel a
    /// measurement rather than take a new one.
    pub fn set_centre_bpm(&mut self, bpm: f32) {
        self.centre_bpm = centre(bpm);
    }

    /// The tracking window's centre.
    pub fn centre_bpm(&self) -> f32 {
        self.centre_bpm
    }

    /// Push one analysis block's novelty. One call per [`crate::analysis::HOP`]
    /// samples, in order.
    pub fn push(&mut self, novelty: f32) {
        self.history[self.write] = if novelty.is_finite() {
            novelty.max(0.0)
        } else {
            0.0
        };
        self.write = (self.write + 1) % self.history.len();
        self.filled = (self.filled + 1).min(self.history.len());
        self.at += f64::from(self.hop_seconds);

        if self.at - self.estimated_at >= f64::from(ESTIMATE_INTERVAL_SECONDS) {
            self.estimated_at = self.at;
            self.revision += 1;
            self.estimate = self.estimate_now();
        } else {
            // Between recomputations the grid is *extrapolated*, not held: the
            // phase this estimate describes moves on at the tempo it found.
            // That is the whole "predict, do not chase" position, and it is why
            // an estimate carries the instant it refers to.
            //
            // Phase first, then the instant it refers to: `phase_at` measures
            // from `estimate.at`, so moving that first would make every
            // extrapolation a no-op and leave the estimate labelled fresh while
            // standing still — worth a quarter of a beat at 120 bpm, and
            // invisible except as a grid that is mysteriously late.
            self.estimate.phase = self.estimate.phase_at(self.at);
            self.estimate.at = self.at;
        }
    }

    /// The current estimate. Cheap — the work happens in [`Tracker::push`].
    pub fn estimate(&self) -> Estimate {
        self.estimate
    }

    /// Stream seconds at the centre of the newest analysis window.
    pub fn at(&self) -> f64 {
        self.at
    }

    /// "No opinion", but still a new opinion: it carries the revision, so a
    /// controller sees that the tracker has spoken and said it does not know,
    /// rather than seeing the previous estimate stand.
    fn nothing(&self) -> Estimate {
        Estimate {
            revision: self.revision,
            ..Estimate::unknown(self.at)
        }
    }

    fn estimate_now(&mut self) -> Estimate {
        let len = self.history.len();
        // Half a window is enough to say something; less is a tempo guessed
        // from two beats.
        if self.filled < len / 2 {
            return self.nothing();
        }

        // Linearise oldest-first, and remove the mean: autocorrelation of a
        // curve with a large DC term finds the DC term.
        let count = self.filled;
        let start = (self.write + len - count) % len;
        let mut sum = 0.0;
        for i in 0..count {
            let v = self.history[(start + i) % len];
            self.window[i] = v;
            sum += v;
        }
        let mean = sum / count as f32;
        if sum <= 0.0 {
            // Silence has no tempo, and saying so is not the same as saying
            // zero: confidence 0.0 leaves the oscillator free-running.
            return self.nothing();
        }
        for v in &mut self.window[..count] {
            *v -= mean;
        }

        let (lo, hi) = self.lag_range;
        if hi <= lo {
            return self.nothing();
        }

        // The peak, over every lag in `BPM_RANGE`. Which multiple of the real
        // period wins does not matter — the fold below settles all of them to
        // the same answer — so the prior is here only to keep a multiple that
        // is *not* an octave from winning. See `PRIOR_OCTAVES`.
        let mut best = (0usize, f32::NEG_INFINITY);
        {
            let window = &self.window[..count];
            for lag in lo..=hi {
                let bpm = 60.0 / (lag as f32 * self.hop_seconds);
                let score = autocorrelation(window, lag) * prior(bpm, self.centre_bpm);
                if score > best.1 {
                    best = (lag, score);
                }
            }
        }
        let (lag, peak) = best;
        if peak <= 0.0 {
            return self.nothing();
        }

        let window = &self.window[..count];

        // Estimates period using the longest valid harmonic lag under half the window length,
        // then applies parabolic interpolation across adjacent lags for sub-hop precision.
        let mut measured = lag;
        while 2 * measured < count / 2 {
            measured = peak_near(window, 2 * measured);
        }
        let refined = refine(
            autocorrelation(window, measured - 1),
            autocorrelation(window, measured),
            autocorrelation(window, measured + 1),
        );
        let found_hops = measured as f32 + refined;

        // **Measured at the peak, not at the folded period.** Whether the
        // novelty resembles itself a period later is a property of the
        // material; the fold is a choice about what to call the tempo, and a
        // grid twice as dense as the pulses correlates with nothing at its own
        // period while the music is as periodic as it ever was. Normalised
        // against the novelty's own variance rather than against the mean score
        // across lags: the window has had its mean removed, so the mean score
        // is approximately zero and dividing by it says nothing at all.
        let variance = autocorrelation(window, 0);
        let periodicity = if variance > 0.0 {
            (autocorrelation(window, lag) / variance / PERIODICITY_FULL).clamp(0.0, 1.0)
        } else {
            0.0
        };

        let bpm = fold(60.0 / (found_hops * self.hop_seconds), self.centre_bpm);
        let period = 60.0 / (bpm * self.hop_seconds);
        // One bin per hop, which is the finest the novelty curve is sampled at;
        // the parabola inside `fold_onto` is what gets the phase back under a
        // hop.
        let bins = (period.round() as usize).clamp(4, count);
        let grid = fold_onto(window, &mut self.profile[..bins], period);

        // Two things have to hold for an estimate to be worth acting on: the
        // novelty at one period apart has to actually resemble itself (there is
        // *a* period), and the novelty has to pile up at this grid's points (it
        // is *this* grid). Multiplying them means a failure of either sinks the
        // estimate, which is the conservative direction — a confident wrong
        // tempo is the only failure here that shows on stage. Neither factor
        // says anything about the octave, and that is deliberate: the octave is
        // not in question any more, so it must not be paid for twice.
        let confidence = (periodicity * grid.fit).clamp(0.0, 1.0);

        Estimate {
            bpm,
            phase: grid.phase,
            confidence,
            at: self.at,
            revision: self.revision,
            half_tempo_hint: grid.offbeat >= HALF_TEMPO_HINT,
        }
    }
}

/// Unnormalised autocorrelation at `lag`, divided by the overlap so that long
/// lags are not penalised for having fewer terms to add.
fn autocorrelation(window: &[f32], lag: usize) -> f32 {
    if lag >= window.len() {
        return 0.0;
    }
    let overlap = window.len() - lag;
    let sum: f32 = (0..overlap).map(|i| window[i] * window[i + lag]).sum();
    sum / overlap as f32
}

/// The strongest of `at` and its two neighbours — where the correlation peak
/// nearest `at` actually is, to the hop.
fn peak_near(window: &[f32], at: usize) -> usize {
    (at.saturating_sub(1)..=at + 1)
        .max_by(|a, b| autocorrelation(window, *a).total_cmp(&autocorrelation(window, *b)))
        .unwrap_or(at)
}

/// The offset of a parabola's vertex through three equally spaced samples, in
/// `[-0.5, 0.5]`.
fn refine(before: f32, at: f32, after: f32) -> f32 {
    let denominator = before - 2.0 * at + after;
    if denominator.abs() < f32::EPSILON {
        return 0.0;
    }
    (0.5 * (before - after) / denominator).clamp(-0.5, 0.5)
}

/// Where the beat sits on a period, and how much of the novelty agrees.
struct Grid {
    /// Beat phase at the newest sample: 0.0 on the beat, rising to 1.0 at the
    /// next one. The convention everything downstream uses, and it is measured
    /// at the *newest* sample because that is the only instant a caller knows a
    /// wall time for.
    phase: f32,
    /// `[0, 1]`. How much of the novelty sits on the grid, against how much
    /// would sit there if it were spread evenly.
    fit: f32,
    /// Novelty half a beat off the grid, against novelty on it. A grid at half
    /// the music's tempo reads near 1.0 here; nothing acts on it but the
    /// operator.
    offbeat: f32,
}

/// Folds the novelty window onto one candidate period and determines grid phase and fit.
///
/// Accumulates novelty periodically to avoid DFT cancellation across octave-subdivided pulses.
fn fold_onto(window: &[f32], profile: &mut [f32], period: f32) -> Grid {
    let nothing = Grid {
        phase: 0.0,
        fit: 0.0,
        offbeat: 0.0,
    };
    let bins = profile.len();
    if bins < 4 || period <= 0.0 || window.is_empty() {
        return nothing;
    }

    profile.fill(0.0);
    let mut mass = 0.0;
    for (n, &v) in window.iter().enumerate() {
        // Only the positive part carries pulses; the negative part is the mean
        // removal's shadow and would fill the troughs in.
        let v = v.max(0.0);
        let position = (n as f32).rem_euclid(period) / period;
        let bin = ((position * bins as f32) as usize).min(bins - 1);
        profile[bin] += v;
        mass += v;
    }
    if mass <= 0.0 {
        return nothing;
    }

    let peak = (0..bins)
        .max_by(|a, b| profile[*a].total_cmp(&profile[*b]))
        .unwrap_or(0);
    // A neighbourhood in beats rather than in hops, so "on the grid" means the
    // same musical thing at 60 bpm as at 200 and the baseline below is a
    // constant instead of a function of the tempo.
    let near = ((ON_GRID_BEATS * bins as f32).round() as usize).max(1);
    let covered = ((2 * near + 1) as f32 / bins as f32).min(1.0);
    let on = around(profile, peak, near);

    // Novelty spread evenly already puts `covered` of itself inside the
    // neighbourhood, so that is the zero rather than 0.0 — otherwise a fast
    // tempo, whose neighbourhood is a larger share of its shorter beat, would
    // read as more confident for no musical reason.
    let full = ON_GRID_FULL.max(covered + 0.05);
    let fit = ((on / mass - covered) / (full - covered)).clamp(0.0, 1.0);

    // Half a period away, when there is room for it to be somewhere else: at
    // fewer than six bins the two neighbourhoods overlap and the ratio would
    // be comparing a thing with itself.
    let offbeat = if bins >= 6 && on > 0.0 {
        around(profile, peak + bins / 2, near) / on
    } else {
        0.0
    };

    // The bin holds everything that landed inside it, so its mass sits at its
    // middle — half a bin, which is half a hop, which is 5 ms and worth more
    // than the whole trim budget at 128 bpm. The parabola through its
    // neighbours puts the beat back under a hop.
    let vertex = refine(
        profile[(peak + bins - 1) % bins],
        profile[peak],
        profile[(peak + 1) % bins],
    );
    let beat = (peak as f32 + 0.5 + vertex) / bins as f32;
    let newest = (window.len() - 1) as f32 / period;
    Grid {
        phase: (newest - beat).rem_euclid(1.0),
        fit,
        offbeat,
    }
}

/// Total in the `near` bins either side of `at`, wrapping — a profile is a
/// circle.
fn around(profile: &[f32], at: usize, near: usize) -> f32 {
    let bins = profile.len();
    (0..=2 * near)
        .map(|i| profile[(at + bins + i - near) % bins])
        .sum()
}

/// A log-normal weight around the window centre. See [`PRIOR_OCTAVES`] for what
/// it is and is not allowed to decide.
fn prior(bpm: f32, centre_bpm: f32) -> f32 {
    let octaves = (bpm / centre_bpm).log2() / PRIOR_OCTAVES;
    (-0.5 * octaves * octaves).exp()
}

#[cfg(test)]
mod tests;
