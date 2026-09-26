use super::*;

// ---------------------------------------------------------------------------
// The Library listing
// ---------------------------------------------------------------------------

// -- the star at the left of every row --------------------------------------

/// Bounding size of the favorite star mark on each library row.
pub(crate) const STAR_SIZE: f32 = size::BASE;

/// Gap between the favorite star and the item name in a library row.
pub(crate) const STAR_GAP: f32 = 7.0;

/// Inner radius ratio for rendering the 5-pointed star mark cleanly at small sizes.
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
