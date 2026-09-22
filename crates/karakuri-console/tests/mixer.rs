//! The Mixer bay: as many strips as the deck has, every one of them a readout,
//! and nothing at all where there is no deck.

mod common;

#[path = "mixer/common.rs"]
pub mod mixer_common;

#[path = "mixer/layout.rs"]
mod layout;

#[path = "mixer/display.rs"]
mod display;

#[path = "mixer/interaction.rs"]
mod interaction;
