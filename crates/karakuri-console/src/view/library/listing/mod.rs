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

/// Returns the persistent favourite key stored in `favourites.json` for an item.
/// Procedures are prefixed with `proc:` to prevent name collisions with Sets.
pub fn favourite_key(name: &str, is_proc: bool) -> String {
    if is_proc {
        format!("proc:{name}")
    } else {
        name.to_string()
    }
}

/// Checks whether an item is starred in the favourites set, supporting legacy entries.
pub fn is_starred(starred: &std::collections::BTreeSet<String>, name: &str, is_proc: bool) -> bool {
    if is_proc {
        starred.contains(&format!("proc:{name}"))
    } else {
        starred.contains(name) || starred.contains(&format!("set:{name}"))
    }
}
