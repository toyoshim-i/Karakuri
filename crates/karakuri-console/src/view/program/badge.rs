use super::*;

// ---------------------------------------------------------------------------
// The risk badge: one number, five bands.
// ---------------------------------------------------------------------------

/// Governor budget for one slot, including duration in milliseconds and calculation basis.
///
/// Passed across the seam from engine reports per [ADR-0296](../../../../docs/adr/0296-the-governor-budgets-on-the-estimate-where-it-does-not.md) and ADR-0156.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Budgeted {
    /// The number the governor spent on this slot, in milliseconds —
    /// `Decision::budgeted_ms`. What [`band_of`] reads.
    pub ms: f32,
    /// How it was taken. See [`Basis`], and the argument on [`band_of`] for why the
    /// dot does not draw it.
    pub basis: Basis,
}

/// The basis used for budgeting: either estimated or measured.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Basis {
    /// The single draw at the engine's reference resolution, because nothing
    /// estimated this slot or the estimate refused. It is a real reading of this
    /// Set and it is a reading at a size that is the operator's only by
    /// coincidence.
    Measured,
    /// The two-draw fit, evaluated at the size this deck is actually drawing into.
    Estimated,
}

/// The five performance risk bands (`green`, `blue`, `yellow`, `red`, `purple`) for deck execution time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Band {
    /// Up to [`BAND_BLUE_MS`]: four of these at 60 Hz.
    Green,
    /// Over [`BAND_BLUE_MS`]: not four at 60, and two at 60 is fine.
    Blue,
    /// [`BAND_YELLOW_MS`]: two at 60 Hz, four at 30 Hz.
    Yellow,
    /// [`BAND_RED_MS`]: one at 60 Hz, two at 30 Hz.
    Red,
    /// [`BAND_PURPLE_MS`] and over: one slot eats a whole frame.
    Purple,
}

impl Band {
    /// The word the mock's class names this band by — `.risk.green` and its four
    /// siblings. What `tests/transcribed_constants_cite_the_mock.rs` looks the
    /// boundary up in `docs/manual/console.html` by, and what `karakuri-engine`'s
    /// own band prediction spells.
    pub fn word(self) -> &'static str {
        match self {
            Band::Green => "green",
            Band::Blue => "blue",
            Band::Yellow => "yellow",
            Band::Red => "red",
            Band::Purple => "purple",
        }
    }

    /// The colour it is drawn in, from the room's own five — `--c-band-*`. See
    /// [`Palette`], where the argument for five properties of their own rather than
    /// `--c-mint` and `--c-pink` is written.
    pub fn colour(self, pal: &Palette) -> Color32 {
        match self {
            Band::Green => pal.band_green,
            Band::Blue => pal.band_blue,
            Band::Yellow => pal.band_yellow,
            Band::Red => pal.band_red,
            Band::Purple => pal.band_purple,
        }
    }

    /// The five, worst last. The order the scale is written in.
    pub const ALL: [Band; 5] = [
        Band::Green,
        Band::Blue,
        Band::Yellow,
        Band::Red,
        Band::Purple,
    ];
}

/// Threshold where green ends and blue begins (4.0 ms). Values on boundary round up to worse band.
pub const BAND_BLUE_MS: f32 = 4.0;

/// *"Yellow, about 8 ms: the boundary for two at 60 Hz, and four at 30 Hz."*
pub const BAND_YELLOW_MS: f32 = 8.0;

/// *"Red, about 12 ms: one at 60 Hz, two at 30 Hz."*
pub const BAND_RED_MS: f32 = 12.0;

/// Threshold for purple risk band (> 16.0 ms, exceeding a full 60 Hz frame budget).
pub const BAND_PURPLE_MS: f32 = 16.0;

/// Maps a budgeted duration in milliseconds to its risk [`Band`].
///
/// Boundaries round toward the worse band; basis details are preserved per ADR-0296, ADR-0356, and [ADR-0191](../../../../docs/adr/0191-the-panels-parked-deck-is-parked-by-the-governor-or-it-is-a-drawing-of-one.md).
pub fn band_of(ms: f32) -> Band {
    match ms {
        _ if ms >= BAND_PURPLE_MS => Band::Purple,
        _ if ms >= BAND_RED_MS => Band::Red,
        _ if ms >= BAND_YELLOW_MS => Band::Yellow,
        _ if ms >= BAND_BLUE_MS => Band::Blue,
        _ => Band::Green,
    }
}
