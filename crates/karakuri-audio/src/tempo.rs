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

/// The lags the autocorrelation searches, as tempi. Wider than a DJ set needs
/// at both ends, because the peak is allowed to be at any multiple of the real
/// period — the fold is what brings it back, and it can only bring back what
/// was searched for.
///
/// **This is not the range of answers.** An answer is whatever lands in the
/// tracking window, which reaches half an octave past either end of this when
/// the centre is at an extreme. A grid at 240 bpm is a perfectly good grid;
/// clamping the fold back into this range would be a lie about the tempo.
pub const BPM_RANGE: RangeInclusive<f32> = 60.0..=200.0;

/// Width of the peak-search prior, in octaves, around the window centre.
///
/// **The prior can no longer decide an octave**: every octave of a candidate
/// folds to the same answer, so preferring one of them over another changes
/// nothing at all. What is left for it is keeping a *non*-octave multiple from
/// winning — a peak at three times the real period folds to something 3:2 out
/// and wrong. At 0.65 octaves, a candidate half an octave from the centre is
/// weighted 0.74 and the third of it is weighted 0.06, which is a decision
/// rather than a tie-break.
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

/// Multiply or divide `bpm` by two until it lands in the window centred on
/// `centre_bpm`.
///
/// Exact, because the only factor ever applied is a power of two: a folded
/// tempo and an unfolded one are the same number of ULPs from the truth.
///
/// **A tempo exactly half an octave from the centre has two equally right
/// answers**, and which one it gets is decided by the last bit of a logarithm.
/// That is not worth legislating the way [`crate::lock::wrap_beats`]'s half-beat
/// is, because it cannot alternate the way that one could: folding is
/// idempotent, so whichever side a given tempo lands on it stays there, and
/// two successive windows can only disagree if the music is sitting on the
/// boundary — which means the grid is a full half-octave away from it, and the
/// ×2 key is the answer to that rather than a rounding rule.
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
    /// There is nearly as much novelty *between* this grid's points as on them,
    /// so the music may be running at twice the grid's tempo.
    ///
    /// **A note for the operator, never an input to the grid.** Nothing in this
    /// crate reads it; the CLI prints it next to the tempo, and the answer to
    /// it is the ×2 key. A grid moved by this measurement automatically is the
    /// confidently-wrong-tempo failure this design was built to remove.
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

        // The period, and it is worth being fussy about: a grid running 1% fast
        // drifts a whole beat inside ten seconds, which is the steady-state
        // error this whole design exists to avoid. Two things buy that back
        // from a lag that is a whole number of hops.
        //
        // **It is measured at the longest lag in this peak's octave ladder that
        // still overlaps half the window.** One hop is 3% of the period at 174
        // bpm and 48 kHz, and 0.4% of the same tempo measured eight lags
        // further out; the fold below brings whichever was measured back to the
        // octave the grid is in, so there is no reason at all to measure at the
        // coarse end. It is worth a factor of five at the fast end — 196 bpm
        // reads 0.02% out this way and 0.13% out at its own lag. Half the
        // window is where it stops, because past that the correlation is
        // averaging over too few beats to be a peak rather than an accident.
        // The doubled peak sits within a hop of twice this one — a period is
        // not a whole number of hops, so the two do not land on the same
        // fraction of one — and finding it there is what keeps the parabola
        // below centred on a maximum instead of clamped against one.
        //
        // **Then a parabola through the peak and its neighbours**, which is
        // what makes the answer sub-hop at all.
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

/// Fold the whole novelty window onto one period and read the grid off it.
///
/// **This is what replaced a single DFT bin at the beat frequency**, and the
/// reason is the octave: `arg(Σ novelty[n]·e^{-2πin/p})` is degenerate at
/// exactly the periods this estimator now reaches on purpose. At twice the
/// real pulse spacing consecutive pulses land half a turn apart in that bin and
/// cancel — the magnitude goes to nothing and the angle is arbitrary — so a
/// grid deliberately folded an octave low would come back with a random phase
/// and no confidence, which is the old failure wearing a new hat. Piling the
/// novelty into bins has no such degeneracy in either direction: two pulses per
/// period make two humps and the stronger one is picked, one pulse per two
/// periods makes one.
///
/// Which of two equal humps is picked is a *parity* the novelty cannot settle —
/// it says where a pulse is, not whether that pulse is a downbeat — and that is
/// left unsettled rather than guessed at.
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
mod tests {
    use super::*;
    use crate::analysis::{Analyzer, BLOCK, HOP};
    use std::f32::consts::TAU;

    const RATE: u32 = 48_000;

    /// A click train at `bpm`, `seconds` long, with a click at sample 0.
    fn clicks(bpm: f32, seconds: f32, amplitude: f32) -> Vec<f32> {
        let period = (60.0 / bpm * RATE as f32) as usize;
        let len = (seconds * RATE as f32) as usize;
        let mut samples = vec![0.0; len];
        let mut at = 0;
        while at < len {
            for n in 0..64.min(len - at) {
                samples[at + n] = amplitude * (1.0 - n as f32 / 64.0);
            }
            at += period;
        }
        samples
    }

    /// Analyse a signal and feed the tracker, exactly as the device does, with
    /// the tracking window centred at `centre` — which is the operator's
    /// `--bpm` before there is a grid, so most tests pass the tempo they built
    /// the signal at and the octave ones deliberately pass something else.
    fn track(samples: &[f32], centre: f32) -> Tracker {
        let mut analyzer = Analyzer::new(RATE);
        let mut tracker = Tracker::new(analyzer.hop_seconds(), analyzer.window_lag(), centre);
        let mut block = vec![0.0f32; BLOCK];
        let mut at = 0;
        while at + BLOCK <= samples.len() {
            block.copy_from_slice(&samples[at..at + BLOCK]);
            tracker.push(analyzer.analyze(&block).novelty);
            at += HOP;
        }
        tracker
    }

    /// What an estimate costs, on a host clock, in the audio callback where it
    /// runs. Ignored for the same reason as `analysis`'s: it is a measurement.
    /// `cargo test -p karakuri-audio --release -- --ignored --nocapture`.
    #[test]
    #[ignore = "a measurement, not an assertion"]
    fn what_one_estimate_costs() {
        let hop = HOP as f32 / RATE as f32;
        let mut tracker = Tracker::new(hop, BLOCK as f32 / 2.0 / RATE as f32, 120.0);
        for i in 0..2000 {
            tracker.push((i % 47) as f32 * 0.01);
        }
        // One push in every ESTIMATE_INTERVAL_SECONDS re-measures; the rest
        // extrapolate. Timed over a whole interval so the answer is what a
        // second of audio costs rather than what the lucky push costs.
        let pushes = (ESTIMATE_INTERVAL_SECONDS / hop).ceil() as u32;
        let start = std::time::Instant::now();
        for i in 0..pushes {
            tracker.push((i % 47) as f32 * 0.01);
        }
        eprintln!(
            "{pushes} pushes covering one estimate: {:.1} µs total, over {:.0} ms of audio",
            start.elapsed().as_secs_f64() * 1e6,
            pushes as f32 * hop * 1000.0
        );
    }

    // -- the window ---------------------------------------------------------

    /// The window is exactly one octave wide, and that is the property the
    /// whole design rests on: any tempo folds into it, and only one fold of it
    /// does.
    #[test]
    fn the_window_is_one_octave_and_admits_exactly_one_fold() {
        for centre in [60.0_f32, 87.0, 120.0, 174.0, 200.0] {
            let window = tracking_window(centre);
            let ratio = window.end() / window.start();
            assert!(
                (ratio - 2.0).abs() < 1e-4,
                "the window around {centre} spans {ratio} octaves"
            );
            // Every tempo, not merely a few: each folds in, and its double and
            // its half do not.
            for step in 0..400 {
                let bpm = 20.0 * 1.01_f32.powi(step);
                let folded = fold(bpm, centre);
                assert!(
                    *window.start() <= folded && folded <= *window.end(),
                    "{bpm} folded to {folded}, outside {window:?}"
                );
                assert!(
                    !window.contains(&(folded * 2.0)) && !window.contains(&(folded * 0.5)),
                    "{window:?} holds both {folded} and one of its octaves"
                );
                // Folding is by powers of two and nothing else: the answer is
                // always the same tempo, an exact number of octaves away.
                let octaves = (folded / bpm).log2();
                assert!(
                    (octaves - octaves.round()).abs() < 1e-5,
                    "{bpm} folded to {folded}, which is {octaves} octaves away"
                );
            }
        }
    }

    /// A tempo already in the window is left exactly alone, folding is
    /// idempotent — including on the boundary, where the answer itself is a
    /// coin toss and only the stability matters — and the factor is always a
    /// power of two.
    #[test]
    fn a_tempo_in_the_window_folds_to_itself_and_folding_again_changes_nothing() {
        assert_eq!(fold(128.0, 128.0), 128.0);
        assert_eq!(fold(174.0, 128.0), 174.0);
        assert_eq!(fold(96.0, 128.0), 96.0);
        // Two octaves either way, exactly.
        assert_eq!(fold(512.0, 128.0), 128.0);
        assert_eq!(fold(32.0, 128.0), 128.0);
        // On the boundary and either side of it, and everywhere else.
        let edge = 128.0 * std::f32::consts::SQRT_2;
        for bpm in [
            edge,
            edge - 0.01,
            edge + 0.01,
            60.0,
            200.0,
            87.0,
            349.0,
            // ...and a tempo that is not one lands in the window too, which is
            // the one thing a caller is allowed to assume.
            0.0,
            -12.0,
            f32::NAN,
            f32::INFINITY,
        ] {
            let once = fold(bpm, 128.0);
            assert_eq!(once, fold(once, 128.0), "{bpm} did not settle");
            assert!(tracking_window(128.0).contains(&once), "{bpm} → {once}");
        }
    }

    /// A centre outside the searchable range is clamped rather than believed,
    /// and a nonsense one does not poison every fold after it. A tap at 400 bpm
    /// is a grid nothing here can measure; a window centred there would answer
    /// with tempi no lag in the search could have produced.
    #[test]
    fn a_centre_outside_the_range_is_clamped_and_a_nonsense_one_is_ignored() {
        assert_eq!(tracking_window(400.0), tracking_window(*BPM_RANGE.end()));
        assert_eq!(tracking_window(10.0), tracking_window(*BPM_RANGE.start()));
        for centre in [400.0_f32, 10.0, 0.0, -5.0, f32::NAN, f32::INFINITY] {
            let window = tracking_window(centre);
            assert!(
                *window.start() >= *BPM_RANGE.start() / 2.0
                    && *window.end() <= *BPM_RANGE.end() * 2.0,
                "a centre of {centre} gave the window {window:?}"
            );
            // Whatever the centre was taken to mean, the fold means the same.
            let folded = fold(133.0, centre);
            assert!(
                window.contains(&folded),
                "133 → {folded}, outside {window:?}"
            );
        }
        assert_eq!(
            Tracker::new(0.01, 0.02, 400.0).centre_bpm(),
            *BPM_RANGE.end()
        );
    }

    // -- the estimate -------------------------------------------------------

    #[test]
    fn a_click_train_gives_its_own_tempo() {
        for bpm in [90.0_f32, 128.0, 174.0] {
            let estimate = track(&clicks(bpm, 12.0, 0.8), bpm).estimate();
            assert!(
                (estimate.bpm - bpm).abs() < bpm * 0.01,
                "a {bpm} bpm train read {} bpm",
                estimate.bpm
            );
            assert!(
                estimate.confidence > 0.5,
                "a clean click train read confidence {}",
                estimate.confidence
            );
        }
    }

    /// The estimate is accurate enough to *extrapolate* from, which is the
    /// property the whole design leans on: a grid that is 1% off drifts a
    /// beat in ten seconds.
    ///
    /// **The fast end is where this is hard and where it is checked.** A lag is
    /// a whole number of hops and 196 bpm is 28 of them, so a period measured
    /// at its own lag is quantised to about 3% and lands within 0.13% of the
    /// truth after the parabola — good enough for the grid, and four times
    /// worse than measuring the same period eight lags further out. A tenth of
    /// a percent is a beat in eight minutes, and it is what the octave ladder
    /// buys; without it these tempi miss by two to three times this bound.
    #[test]
    fn the_tempo_is_accurate_enough_to_run_a_grid_off() {
        for bpm in [128.0_f32, 174.0, 196.0] {
            for seconds in [12.0_f32, 16.0] {
                let estimate = track(&clicks(bpm, seconds, 0.8), bpm).estimate();
                let error = (estimate.bpm - bpm).abs() / bpm;
                assert!(
                    error < 0.001,
                    "a {bpm} bpm train over {seconds}s read {} — {}% out, which drifts a \
                     beat in {} seconds",
                    estimate.bpm,
                    error * 100.0,
                    (1.0 / (error * bpm / 60.0)) as u32
                );
            }
        }
    }

    /// The phase points at the beat, not merely at a constant offset from it.
    #[test]
    fn the_phase_lands_on_the_beat() {
        // Clicks at sample 0 and every period after, so a beat lands exactly on
        // stream time k * 60/bpm.
        let bpm = 120.0_f32;
        let estimate = track(&clicks(bpm, 12.0, 0.8), bpm).estimate();

        // Where the estimate thinks the most recent beat was, in stream time.
        let beats_per_second = estimate.bpm as f64 / 60.0;
        let last_beat = estimate.at - estimate.phase as f64 / beats_per_second;
        // ...against where the beats actually are.
        let period = 60.0 / bpm as f64;
        let off = (last_beat / period).fract();
        let off = if off > 0.5 { off - 1.0 } else { off };
        assert!(
            off.abs() < 0.05,
            "the beat was placed {off} of a beat away from where it is"
        );
    }

    /// Silence has no tempo and says so, rather than reporting a confident
    /// zero or the last thing it saw.
    #[test]
    fn silence_has_no_tempo_and_no_confidence() {
        let tracker = track(&vec![0.0; RATE as usize * 10], 120.0);
        assert_eq!(tracker.estimate().confidence, 0.0);
    }

    /// Something with no beat in it must not read as a confident tempo. A
    /// sustained tone has plenty of energy and no pulses at all.
    #[test]
    fn a_sustained_tone_is_not_a_tempo() {
        let samples: Vec<f32> = (0..RATE as usize * 10)
            .map(|n| 0.5 * (TAU * 440.0 * n as f32 / RATE as f32).sin())
            .collect();
        let estimate = track(&samples, 120.0).estimate();
        assert!(
            estimate.confidence < 0.3,
            "a sustained tone read confidence {}",
            estimate.confidence
        );
    }

    /// The same novelty gives the same estimate. Replay depends on the `tempo`
    /// record rather than on this, but an estimator whose answer depended on
    /// anything else would be untestable as well as unrecordable.
    #[test]
    fn the_same_input_produces_the_same_estimate() {
        let samples = clicks(128.0, 14.0, 0.8);
        let once = track(&samples, 120.0).estimate();
        let twice = track(&samples, 120.0).estimate();
        assert_eq!(once, twice);
        // And the centre is part of the input, not of the history: a tracker
        // told the same things in the same order lands in the same place.
        let moved = {
            let mut tracker = track(&samples, 120.0);
            tracker.set_centre_bpm(120.0);
            tracker.estimate()
        };
        assert_eq!(once, moved);
    }

    // -- the octave ---------------------------------------------------------

    /// **The whole tempo range, not the three tempi that were convenient**, and
    /// at several window centres each: an operator's `--bpm` is a guess, and
    /// every guess inside the window has to give the same answer or the window
    /// is not doing its job.
    ///
    /// This is where an estimator quietly stops working over part of its range,
    /// and the failure is silent — a half-tempo answer reads as "the grid
    /// settled somewhere odd" rather than as a bug, so nothing downstream
    /// complains.
    ///
    /// Two window lengths per tempo, because the failure this replaced was
    /// intermittent: the phase the old check read was numerically arbitrary, so
    /// which window it was measured over decided the answer.
    #[test]
    fn every_tempo_in_the_range_reads_itself_and_not_its_half() {
        let mut wrong = Vec::new();
        for bpm in [
            62.0_f32, 70.0, 85.0, 100.0, 110.0, 120.0, 128.0, 140.0, 150.0, 160.0, 165.0, 170.0,
            174.0, 180.0, 185.0, 190.0, 196.0,
        ] {
            for seconds in [12.0_f32, 16.0] {
                let samples = clicks(bpm, seconds, 0.8);
                // The true tempo, and an operator 30% out either way — still
                // inside the window, which reaches ±41%.
                for centre in [bpm, bpm * 1.3, bpm / 1.3] {
                    let estimate = track(&samples, centre).estimate();
                    if (estimate.bpm - bpm).abs() > bpm * 0.02 || estimate.confidence < 0.5 {
                        wrong.push((bpm, seconds, centre, estimate.bpm, estimate.confidence));
                    }
                }
            }
        }
        assert!(
            wrong.is_empty(),
            "click trains misread (bpm, window, centre, read, confidence): {wrong:?}"
        );
    }

    /// Eighth notes with the beat on every other one: the correlation peak is
    /// at the beat, the subdivision is inside the window too, and the estimate
    /// has to come back with the beat.
    #[test]
    fn a_subdivided_pattern_reads_the_beat_and_not_the_subdivision() {
        // 128 bpm beats, with a weak click between each pair.
        let bpm = 128.0_f32;
        let period = (60.0 / bpm * RATE as f32) as usize;
        let len = RATE as usize * 14;
        let mut samples = vec![0.0f32; len];
        let mut at = 0;
        let mut strong = true;
        while at < len {
            let amplitude = if strong { 0.9 } else { 0.15 };
            for n in 0..64.min(len - at) {
                samples[at + n] = amplitude * (1.0 - n as f32 / 64.0);
            }
            at += period / 2;
            strong = !strong;
        }
        let estimate = track(&samples, bpm).estimate();
        assert!(
            (estimate.bpm - bpm).abs() < 4.0,
            "a subdivided {bpm} bpm pattern read {} bpm",
            estimate.bpm
        );
    }

    /// The other direction: every pulse is a beat, and a grid at half tempo
    /// would fit them all. It must not settle there.
    #[test]
    fn an_even_click_train_is_not_read_at_half_tempo() {
        for bpm in [120.0_f32, 140.0] {
            let estimate = track(&clicks(bpm, 14.0, 0.8), bpm).estimate();
            assert!(
                (estimate.bpm - bpm / 2.0).abs() > 5.0,
                "a {bpm} bpm train settled at half tempo: {}",
                estimate.bpm
            );
            assert!(
                (estimate.bpm - bpm).abs() < bpm * 0.02,
                "a {bpm} bpm train read {}",
                estimate.bpm
            );
        }
    }

    /// A kick with a quiet hat between every pair of beats. The hat is a beat
    /// *subdivision*, not a beat, so this must read the kick's tempo — and the
    /// wrong answer here is the expensive one: it is a confident double, not a
    /// shrug. It read 180 bpm at confidence 1.0 before the fold replaced the
    /// check that was supposed to prevent exactly that.
    ///
    /// Several window lengths and several hat levels, because the two
    /// interleaved sets of grid points used to be distinguishable only by which
    /// one the novelty was stronger at, and which of them a walk reached first
    /// depended on where the window happened to end.
    #[test]
    fn a_quiet_offbeat_is_a_subdivision_and_not_a_doubled_tempo() {
        let bpm = 90.0_f32;
        let mut wrong = Vec::new();
        for seconds in [11.0_f32, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0] {
            for hat in [0.15_f32, 0.3, 0.45] {
                let estimate = track(&kick_and_hat(bpm, seconds, hat), bpm).estimate();
                if (estimate.bpm - bpm).abs() > bpm * 0.05 {
                    wrong.push((seconds, hat, estimate.bpm, estimate.confidence));
                }
            }
        }
        assert!(
            wrong.is_empty(),
            "a quiet offbeat moved the octave (window, hat, read, confidence): {wrong:?}"
        );
    }

    /// A kick every beat with a quieter hat halfway between.
    fn kick_and_hat(bpm: f32, seconds: f32, hat: f32) -> Vec<f32> {
        let period = (60.0 / bpm * RATE as f32) as usize;
        let len = (seconds * RATE as f32) as usize;
        let mut samples = vec![0.0f32; len];
        let burst = |s: &mut Vec<f32>, at: usize, amplitude: f32| {
            for n in 0..64.min(s.len().saturating_sub(at)) {
                s[at + n] = amplitude * (1.0 - n as f32 / 64.0);
            }
        };
        let mut at = 0;
        while at < len {
            burst(&mut samples, at, 0.9);
            if at + period / 2 < len {
                burst(&mut samples, at + period / 2, hat);
            }
            at += period;
        }
        samples
    }

    /// **The intended failure, asserted so nobody repairs it into a
    /// heuristic.** A window centred an octave below the music locks an octave
    /// below the music — confidently, because the grid does fit: every one of
    /// its points is a beat. Nothing automatic will move it, and the operator's
    /// ×2 key is the whole answer.
    #[test]
    fn a_window_centred_an_octave_off_locks_an_octave_off() {
        let estimate = track(&clicks(174.0, 14.0, 0.8), 87.0).estimate();
        assert!(
            (estimate.bpm - 87.0).abs() < 87.0 * 0.02,
            "a 174 bpm train under an 87 bpm window read {} bpm",
            estimate.bpm
        );
        // And it is believed, which is the part that used to be wrong: an
        // octave error collapsed the confidence and read as an absent beat.
        assert!(
            estimate.confidence > crate::lock::GATE_CONFIDENCE,
            "the octave-low grid read confidence {}, so it would never lock",
            estimate.confidence
        );
        // The other direction: a window an octave above the music.
        let estimate = track(&clicks(90.0, 14.0, 0.8), 180.0).estimate();
        assert!(
            (estimate.bpm - 180.0).abs() < 180.0 * 0.02,
            "a 90 bpm train under a 180 bpm window read {} bpm",
            estimate.bpm
        );
        assert!(
            estimate.confidence > crate::lock::GATE_CONFIDENCE,
            "the octave-high grid read confidence {}, so it would never lock",
            estimate.confidence
        );
    }

    /// The hint, which is the only thing left of the old automatic decision.
    /// It fires when the grid really is at half the music's tempo, and stays
    /// quiet on a kick with a quiet hat — which is the pattern the old
    /// automatic double turned into a confident wrong tempo.
    #[test]
    fn the_half_tempo_hint_fires_on_a_grid_an_octave_low_and_not_on_an_offbeat() {
        let low = track(&clicks(174.0, 14.0, 0.8), 87.0).estimate();
        assert!(
            low.half_tempo_hint,
            "a grid at half the music's tempo did not hint (read {} bpm, c{})",
            low.bpm, low.confidence
        );
        let right = track(&clicks(174.0, 14.0, 0.8), 174.0).estimate();
        assert!(
            !right.half_tempo_hint,
            "a correct grid hinted at half tempo"
        );
        for hat in [0.15_f32, 0.3, 0.45] {
            let estimate = track(&kick_and_hat(90.0, 14.0, hat), 90.0).estimate();
            assert!(
                !estimate.half_tempo_hint,
                "a quiet {hat} offbeat hinted at half tempo"
            );
        }
    }

    /// **The centre moves with the grid, and that is what follows a drift
    /// across an octave boundary.** The music runs away from a static window
    /// until it is outside it and folds to half; the same music under a window
    /// that is told where the grid is stays whole all the way.
    ///
    /// The tracker is fed its own answer here, which is what a locked grid
    /// hands back to it — `crate::lock` is what makes that a slow trim rather
    /// than a copy, and `tests/beat_tracking.rs` runs the real thing.
    #[test]
    fn a_tempo_that_drifts_across_the_boundary_is_followed_when_the_centre_moves() {
        let start = 130.0_f32;
        let end = 195.0_f32;
        let samples = accelerating(start, end, 40.0);

        let mut analyzer = Analyzer::new(RATE);
        let mut moving = Tracker::new(analyzer.hop_seconds(), analyzer.window_lag(), start);
        let mut fixed = Tracker::new(analyzer.hop_seconds(), analyzer.window_lag(), start);
        let mut block = vec![0.0f32; BLOCK];
        let mut at = 0;
        while at + BLOCK <= samples.len() {
            block.copy_from_slice(&samples[at..at + BLOCK]);
            let novelty = analyzer.analyze(&block).novelty;
            moving.push(novelty);
            fixed.push(novelty);
            // What a grid does: it is at the tempo it was last told about.
            let estimate = moving.estimate();
            if estimate.confidence > crate::lock::GATE_CONFIDENCE {
                moving.set_centre_bpm(estimate.bpm);
            }
            at += HOP;
        }

        // The boundary the music crossed: the static window's top edge.
        assert!(
            end > *tracking_window(start).end(),
            "the drift never left the static window, so this proves nothing"
        );
        let followed = moving.estimate();
        assert!(
            (followed.bpm - end).abs() < end * 0.04,
            "the moving window ended at {} bpm and the music at {end}",
            followed.bpm
        );
        // ...and the same novelty under a window that never moved is exactly
        // the octave error this design accepts, which is what keeps the
        // assertion above from passing against a tracker that ignores the
        // centre entirely.
        let stuck = fixed.estimate();
        assert!(
            (stuck.bpm - end / 2.0).abs() < end * 0.04,
            "the static window ended at {} bpm, neither the music nor its half",
            stuck.bpm
        );
    }

    /// A click train whose tempo rises linearly from `from` to `to`.
    fn accelerating(from: f32, to: f32, seconds: f32) -> Vec<f32> {
        let len = (seconds * RATE as f32) as usize;
        let mut samples = vec![0.0f32; len];
        let mut at = 0usize;
        while at < len {
            for n in 0..64.min(len - at) {
                samples[at + n] = 0.8 * (1.0 - n as f32 / 64.0);
            }
            let bpm = from + (to - from) * (at as f32 / len as f32);
            at += (60.0 / bpm * RATE as f32) as usize;
        }
        samples
    }

    // -- extrapolation ------------------------------------------------------

    /// The grid runs on its own between estimates. This is the mechanism the
    /// brief calls "predict, do not chase", so it is asserted directly.
    #[test]
    fn a_grid_extrapolates_forward_at_its_own_tempo() {
        let estimate = Estimate {
            bpm: 120.0,
            phase: 0.25,
            confidence: 1.0,
            at: 10.0,
            revision: 1,
            half_tempo_hint: false,
        };
        // Half a second at 120 bpm is exactly one beat.
        assert!((estimate.phase_at(10.5) - 0.25).abs() < 1e-6);
        // A quarter second is half a beat.
        assert!((estimate.phase_at(10.25) - 0.75).abs() < 1e-6);
        // And it wraps rather than running past 1.0.
        assert!((0.0..1.0).contains(&estimate.phase_at(17.3)));
    }
}
