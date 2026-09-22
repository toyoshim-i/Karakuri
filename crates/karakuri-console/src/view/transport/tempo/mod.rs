use std::time::Duration;

use super::super::*;
use super::audio_in::Rec;
use super::*;

mod layout;
mod paint;

use layout::*;
pub(crate) use paint::*;

// ---------------------------------------------------------------------------
// The Transport bay
// ---------------------------------------------------------------------------

/// What the transport row reads this frame: the session's tempo and position,
/// and what the frame before this one cost.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transport {
    /// The tempo, drawn in `.bpm`'s treatment.
    pub bpm: f32,
    /// Musical position, unbounded and monotone.
    pub beats: f64,
    /// How many beats there are in a bar, which is how many dots the grid has.
    pub beats_per_bar: u32,
    /// Frames a second, or `None` where nobody can say yet.
    pub fps: Option<f32>,
    /// What one frame cost on the CPU, in milliseconds.
    pub frame_ms: f32,
    /// What a frame has to fit in, in milliseconds — and `None` where nothing can say.
    pub budget_ms: Option<f32>,
    /// What the master chain costs this frame, in milliseconds.
    pub chain_ms: Option<f32>,
    /// What the last write did — the mock's `landed` capsule.
    pub health: Option<Stage>,
    /// Whether a session is being recorded.
    pub rec: Option<Rec>,
}

impl Transport {
    /// How many dots the beat grid has: [`Transport::beats_per_bar`], and at least
    /// one.
    pub fn dots(&self) -> u32 {
        self.beats_per_bar.max(1)
    }

    /// Where the beat is inside the current bar, from zero, and continuous.
    pub fn position(&self) -> f32 {
        self.beats.rem_euclid(self.dots() as f64) as f32
    }

    /// One-based bar index computed from beat count and dots per bar.
    pub fn bar(&self) -> i64 {
        self.beats.div_euclid(self.dots() as f64) as i64 + 1
    }
}

/// Computes layout rectangles and values for the transport row readouts and controls.
///
/// Returns `None` if transport values are missing or layout is uninitialized.
pub fn transport(
    ctx: &egui::Context,
    layout: &karakuri_layout::Layout,
    values: Option<Transport>,
) -> Option<TransportRow> {
    let values = values?;
    if ctx.cumulative_pass_nr() == 0 {
        return None;
    }
    let row = to_egui(layout.rect(layout.find("transport")?));
    let width = |job: LayoutJob| ctx.fonts_mut(|f| f.layout_job(job).size().x);
    let bpm = width(bpm_job(&values, Color32::PLACEHOLDER, Color32::PLACEHOLDER));
    let label = width(span(BPM_LABEL, Color32::PLACEHOLDER));
    let bar = width(span(&bar_text(&values), Color32::PLACEHOLDER));
    let frame = width(frame_job(
        &values,
        Color32::PLACEHOLDER,
        Color32::PLACEHOLDER,
    ));
    let health = values
        .health
        .map(|stage| size::PILL_PAD_X * 2.0 + width(span(stage.word(), Color32::PLACEHOLDER)));
    let rec = values.rec.map(|_| {
        size::PILL_PAD_X * 2.0
            + size::SINK_DOT
            + size::SINK_GAP
            + width(span(REC_LABEL, Color32::PLACEHOLDER))
    });
    transport_row(row, &values, bpm, label, bar, frame, health, rec)
}

/// The transport row's furniture: a rectangle for each readout, and which beat
/// is lit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransportRow {
    /// The tempo, in `.bpm`'s 20px box.
    pub bpm: Rect,
    /// The faint `BPM` beside it.
    pub label: Rect,
    /// The whole `.beat-grid`: [`TransportRow::dots`] dots and the gaps between
    /// them. One dot is [`TransportRow::dot`].
    pub grid: Rect,
    /// How many dots are in that grid — [`Transport::dots`], carried so the painter
    /// and a test read the same number the width was built from.
    pub dots: u32,
    /// Where the light is, in beats from the first dot's centre.
    pub at: f32,
    /// `bar 37`.
    pub bar: Rect,
    /// The frame readout.
    pub frame: Rect,
    /// The health capsule.
    pub health: Option<Rect>,
    /// The `● rec` pill.
    pub rec: Option<Rect>,
    /// The values these rectangles were measured from.
    pub values: Transport,
}

/// How far a press may move the tempo: ±15% of what the grid is running at when
/// it lands.
pub const TEMPO_BAND: f32 = 0.15;

/// What the whole figure spans, as the same fraction of the same tempo: two
/// bands either way.
pub const TEMPO_SPAN: f32 = TEMPO_BAND * 2.0;

impl TransportRow {
    /// One dot of the beat grid, from the left.
    pub fn dot(&self, index: u32) -> Rect {
        assert!(index < self.dots, "dot {index} of a grid of {}", self.dots);
        Rect::from_min_size(
            Pos2::new(self.grid.min.x + BEAT_PITCH * index as f32, self.grid.min.y),
            egui::vec2(size::BEAT_W, size::BEAT_H),
        )
    }

    /// How much of the light is on the dot at `index` this frame, from `0.0` to
    /// `1.0`.
    pub fn lit(&self, index: u32) -> f32 {
        assert!(index < self.dots, "dot {index} of a grid of {}", self.dots);
        beat_at(self.at, index, self.dots)
    }

    /// How far along the figure `p` is, on `[0, 1]` across [`TransportRow::bpm`].
    fn along(&self, p: karakuri_layout::Point) -> f32 {
        (p.x - self.bpm.min.x) / self.bpm.width()
    }

    /// What a press `unit` of the way along the figure names, in beats a minute.
    pub fn tempo_at(&self, unit: f32) -> f32 {
        self.values.bpm * (1.0 + (unit - 0.5) * 2.0 * TEMPO_SPAN)
    }

    /// Whether a tempo is one a press may ask for: within [`TEMPO_BAND`].
    pub fn in_band(&self, bpm: f32) -> bool {
        bpm.is_finite()
            && bpm > 0.0
            && (bpm - self.values.bpm).abs() <= self.values.bpm.abs() * TEMPO_BAND
    }

    /// Whether `p` is on the tempo figure asking for something it may have.
    pub fn on_tempo(&self, p: karakuri_layout::Point) -> bool {
        self.bpm.contains(Pos2::new(p.x, p.y)) && self.in_band(self.tempo_at(self.along(p)))
    }

    /// What a press at `p` asks the grid to run at.
    pub fn tempo(&self, p: karakuri_layout::Point) -> Option<Operation> {
        self.on_tempo(p).then(|| Operation::SetFreeRunTempo {
            bpm: self.tempo_at(self.along(p)),
        })
    }

    /// Whether `p` is on the `rec` pill.
    pub fn on_rec(&self, p: karakuri_layout::Point) -> bool {
        self.rec
            .is_some_and(|pill| pill.contains(Pos2::new(p.x, p.y)))
    }

    /// What a press at `p` on the `rec` pill asks for, or `None` off it.
    pub fn record(&self, p: karakuri_layout::Point) -> Option<Operation> {
        let rec = self.values.rec?;
        self.on_rec(p).then_some(Operation::RecordSession {
            recording: match rec {
                Rec::Idle => Recording::Start { id: None },
                Rec::Running => Recording::Stop,
            },
        })
    }
}

/// From one dot to the next: [`size::BEAT_W`] and the [`size::BEAT_GAP`] after
/// it, which is what one beat of travel measures on this grid.
pub const BEAT_PITCH: f32 = size::BEAT_W + size::BEAT_GAP;

/// How many steps one beat of travel is drawn in: the pixels in one
/// [`BEAT_PITCH`], so the light moves by at most one of them between updates.
const BEAT_STEPS: u64 = BEAT_PITCH as u64;

/// How stale the beat grid may get, declared under P-0091 and turned into a deadline.
pub const BEAT_STALENESS: Duration = Duration::from_micros(BEAT_MICROS / BEAT_STEPS);

/// How much of the light is on the dot at `index`, with the light `at` beats
/// into a bar of `dots`.
pub fn beat_at(at: f32, index: u32, dots: u32) -> f32 {
    let dots = dots.max(1) as f32;
    if dots < 2.0 {
        return 1.0;
    }
    let round = (index as f32 - at).rem_euclid(dots);
    let away = round.min(dots - round);
    match away < 1.0 {
        true => 0.5 * (1.0 + (away * std::f32::consts::PI).cos()),
        false => 0.0,
    }
}
