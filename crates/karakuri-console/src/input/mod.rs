//! Input routing and pointer event arbitration between layout boundaries and `egui`.
//!
//! Evaluates pointer interaction priorities: active boundary drags, open modal dropdowns,
//! boundary grab margins, and registered control probes.

mod claim;
mod handlers;
mod probes;
mod wheel;

pub use claim::{claim, Claim};
pub use probes::{Probe, CONTROLS, PROBES};
pub use wheel::{wheeled, Turned};
