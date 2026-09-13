//! Shared interactive widgets for the console view.

pub mod card;
pub mod chip;
pub mod fader;
pub mod field;
pub mod fold_grip;
pub mod glyph;
pub mod head;
pub mod pills;
pub mod track;

#[allow(unused_imports)]
pub use card::*;
#[allow(unused_imports)]
pub use chip::*;
#[allow(unused_imports)]
pub use fader::*;
#[allow(unused_imports)]
pub use field::*;
#[allow(unused_imports)]
pub use fold_grip::*;
#[allow(unused_imports)]
pub use glyph::*;
#[allow(unused_imports)]
pub use head::*;
#[allow(unused_imports)]
pub use pills::*;
#[allow(unused_imports)]
pub use track::*;

#[allow(unused_imports)]
pub(crate) use card::*;
#[allow(unused_imports)]
pub(crate) use chip::*;
#[allow(unused_imports)]
pub(crate) use fader::*;
#[allow(unused_imports)]
pub(crate) use field::*;
#[allow(unused_imports)]
pub(crate) use fold_grip::{grip_dots, pane_dividers};
#[allow(unused_imports)]
pub(crate) use glyph::*;
#[allow(unused_imports)]
pub(crate) use head::*;
#[allow(unused_imports)]
pub(crate) use pills::{
    head_pills, on_pill_at, pill_at, pill_into, pill_width, ARMED_GLOW, ARMED_WASH, ON_GLOW,
    ON_WASH,
};
#[allow(unused_imports)]
pub(crate) use track::*;
