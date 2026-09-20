use super::super::*;
use super::*;

// ---------------------------------------------------------------------------
// The Library listing
// ---------------------------------------------------------------------------

// -- the star at the left of every row --------------------------------------

/// The box the star stands in, which is the box the glyph it stands in for
/// would have had: `.lib-row`'s type is the console's [`size::BASE`] and the
/// mark is square at that size, which is [`arrow_mark`]'s rule for a mark drawn
/// instead of typed.
///
/// Derived rather than transcribed for that reason — the mock sets no width on
/// `.star` at all, because a glyph has one.
pub(crate) const STAR_SIZE: f32 = size::BASE;

/// The gap between the star and the name, which is `.lib-row`'s `gap: 7px` —
/// the row is a flex of the star, the name and the time, and this is the one
/// spacing in it that is not padding.
///
/// It happens to be [`size::LIB_ROW_PAD_X`]'s number and is not read off it:
/// one is the row's padding and the other is the gap between two of its
/// children, and they move for different reasons.
pub(crate) const STAR_GAP: f32 = 7.0;

/// How far in from a star's own box its points reach. A five-pointed star is
/// ten rim points at two radii, and this is the inner one as a fraction of the
/// outer.
///
/// The console's own number: a pentagram's exact inner radius is about `0.382`
/// of its outer and reads as a spike at eleven pixels, so this is the fatter
/// star a small mark wants. Nothing in the mock says it, because the mock has a
/// glyph.
pub(crate) const STAR_WAIST: f32 = 0.46;

pub mod layout;
pub mod menu;
pub mod reading;
pub mod render;
pub mod rows;

pub use menu::*;
pub use reading::*;
pub(crate) use render::*;
pub use rows::*;
