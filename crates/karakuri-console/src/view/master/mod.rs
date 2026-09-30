//! Master bay body: output level fader and downstream effects chain (ADR-0156, ADR-0224, ADR-0340).

pub(crate) mod layout;
pub(crate) mod paint;
pub(crate) mod types;
mod view;

pub use layout::*;
pub(crate) use paint::*;
pub use types::*;
