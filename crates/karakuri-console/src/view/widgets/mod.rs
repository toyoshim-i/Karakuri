//! Shared interactive widgets for the console view.

pub mod fold_grip;
pub mod pills;

pub use fold_grip::*;
pub use pills::*;

pub(crate) use fold_grip::{grip_dots, pane_dividers};
pub(crate) use pills::{
    head_pills, on_pill_at, pill_at, pill_into, pill_width, ARMED_GLOW, ARMED_WASH, ON_GLOW,
    ON_WASH,
};
