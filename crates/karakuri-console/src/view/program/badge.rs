use super::*;

// ---------------------------------------------------------------------------
// The risk badge: one number, five bands.
// ---------------------------------------------------------------------------

/// What the governor budgeted one slot at, and which of its two numbers that is
/// — the seam the risk badge is drawn from.
///
/// `karakuri_engine::governor::Decision` carries `budgeted_ms` and a `Basis`
/// saying whether it is the two-draw estimate at the output's size or the
/// single-draw measurement at the reference resolution
/// ([ADR-0296](../../../../docs/adr/0296-the-governor-budgets-on-the-estimate-where-it-answers-and-on-the-measurement-where-it-does-not.md)).
/// This is that pair, arriving the way every other value does: `src/` takes no
/// engine (ADR-0156), so whoever holds the deck reads the report and writes
/// [`View::costs`].
///
/// The engine's third basis is this type's [`None`].
/// `governor::Basis::Unbudgetable` is a slot nothing measured and nothing
/// estimated, and `Decision::budgeted_ms` is `None` there. It is not a number
/// and it is emphatically not a zero, so it does not cross this seam as one: no
/// entry, no dot. See [`View::costs`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Budgeted {
    /// The number the governor spent on this slot, in milliseconds —
    /// `Decision::budgeted_ms`. What [`band_of`] reads.
    pub ms: f32,
    /// How it was taken. See [`Basis`], and the argument on [`band_of`] for why the
    /// dot does not draw it.
    pub basis: Basis,
}

/// Which of the governor's two numbers [`Budgeted::ms`] is —
/// `karakuri_engine::governor::Basis`, less the variant this crate spells
/// [`None`].
///
/// Carried across the seam and not drawn, which is a decision rather than an
/// omission: see [`band_of`].
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

/// The five bands the risk badge is drawn in, and the words the mock's
/// `.risk.green`, `.risk.blue`, `.risk.yellow`, `.risk.red` and `.risk.purple`
/// name them by.
///
/// The table is the console's and not the engine's.
/// `karakuri_engine::estimate`'s own documentation says so, and
/// `crates/karakuri-engine/tests/governor.rs` quotes the boundaries rather than
/// sharing them for the same reason: the engine produces a number of
/// milliseconds and has no opinion about how many of a thing an operator can
/// mix. The scale is a reading of one frame's worth of budget shared four ways,
/// which is a fact about this panel's four cells.
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

/// Where green ends and blue begins: `docs/manual/console.html`, *What a deck
/// preview cell shows, and when* — *"Green, up to 4 ms: four of these at 60
/// Hz."*
///
/// Four slots share one frame and 16.7 ms at 60 Hz is about 4 ms each, which is
/// where the whole scale comes from: it answers *how many of these, and at what
/// rate* rather than handing an operator a number to divide in a dark room.
///
/// Each of these four is named for the band it lets you into rather than the
/// one it leaves, because that is the rounding rule written into the name: a
/// value *on* a boundary rounds to the worse band, so 4.0 is blue and not
/// green. See [`band_of`].
pub const BAND_BLUE_MS: f32 = 4.0;

/// *"Yellow, about 8 ms: the boundary for two at 60 Hz, and four at 30 Hz."*
pub const BAND_YELLOW_MS: f32 = 8.0;

/// *"Red, about 12 ms: one at 60 Hz, two at 30 Hz."*
pub const BAND_RED_MS: f32 = 12.0;

/// *"Purple, over 16 ms: one slot eats a whole frame and the cell stops
/// drawing."*
///
/// The last band is the one that is also a behaviour, and the behaviour is not
/// this crate's: a slot over budget is stopped by the governor rather than
/// shown harder, and a stopped slot reaches the console as a cell with no
/// picture in it. So the console never has to decide to stop drawing — what it
/// draws is the band, and the stopping has already happened upstream.
pub const BAND_PURPLE_MS: f32 = 16.0;

/// The band one number falls in, and the whole of the console's half of
/// the risk badge.
///
/// The four boundaries are [`BAND_BLUE_MS`], [`BAND_YELLOW_MS`],
/// [`BAND_RED_MS`] and [`BAND_PURPLE_MS`], and a value on a boundary rounds
/// to the worse band — the mock says so in those words, and it is the reason
/// every comparison here is `>=` and the fall-through is green. The direction
/// is the same one the estimate rounds in: `karakuri_engine::estimate` rounds
/// toward refusing at every step, so a badge that rounded a boundary the
/// generous way would be the one place in the chain that reads a number
/// kindly.
///
/// # It does not read [`Basis`], and that is the decision rather than the
/// default
///
/// A dot drawn from an estimate at this deck's own size and a dot drawn from a
/// measurement at 1280x720 are not the same statement — P-0095 is exactly
/// that, and ADR-0296 §3 carries `FloorRead` and `Floored` on the decision so
/// the difference cannot be lost. The console keeps it ([`Budgeted::basis`])
/// and does not draw it, for two reasons.
///
/// - The number is the one the governor spent. A badge that drew a dot
///   only where the basis is [`Basis::Estimated`] would be drawing a different
///   quantity from the one the deck is being governed on: a slot falls back to
///   its measurement wherever its estimate refuses (ADR-0356), so the dot
///   would vanish on material the estimator declines to answer for — and a
///   slot the governor parks on a measured number would be parked with no
///   visible cause, which is
///   [ADR-0191](../../../../docs/adr/0191-the-panels-parked-deck-is-parked-by-the-governor-or-it-is-a-drawing-of-one.md)'s
///   complaint. A refusal is not a green dot; a number that was spent is not
///   a refusal.
/// - The mock does not distinguish them, and the page moves first. `.risk`
///   has five classes and there is no sixth mark, no hollow ring and no second
///   word in the caption for *how this was taken*. Inventing one here would be
///   the console specifying itself. What the page would have to say is
///   reported rather than drawn — see the module documentation on
///   [`caption_into`].
///
/// What P-0095 does get is the one thing the page already specifies: no
/// number, no dot. `governor::Basis::Unbudgetable` does not cross the seam
/// (see [`Budgeted`]), and a cell handed nothing draws nothing — not hollow,
/// not grey, not green by default.
pub fn band_of(ms: f32) -> Band {
    match ms {
        _ if ms >= BAND_PURPLE_MS => Band::Purple,
        _ if ms >= BAND_RED_MS => Band::Red,
        _ if ms >= BAND_YELLOW_MS => Band::Yellow,
        _ if ms >= BAND_BLUE_MS => Band::Blue,
        _ => Band::Green,
    }
}
