//! Shared interactive widgets for the console view.

pub mod fader;
pub mod fold_grip;
pub mod head;
pub mod pills;

#[allow(unused_imports)]
pub use fader::*;
pub use fold_grip::*;
pub use head::*;
pub use pills::*;

#[allow(unused_imports)]
pub(crate) use fader::*;
pub(crate) use fold_grip::{grip_dots, pane_dividers};
#[allow(unused_imports)]
pub(crate) use head::*;
pub(crate) use pills::{
    head_pills, on_pill_at, pill_at, pill_into, pill_width, ARMED_GLOW, ARMED_WASH, ON_GLOW,
    ON_WASH,
};
