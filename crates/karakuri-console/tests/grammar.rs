//! The six keys inside a bay: what a digit names, what the arrows walk, what
//! `space` cycles and what `enter` performs.

mod common;

#[path = "grammar/common.rs"]
pub mod grammar_common;

#[path = "grammar/navigation_and_dispatch.rs"]
mod navigation_and_dispatch;

#[path = "grammar/space_and_enter.rs"]
mod space_and_enter;

#[path = "grammar/cards_and_choosers.rs"]
mod cards_and_choosers;

#[path = "grammar/chain_and_dismissal.rs"]
mod chain_and_dismissal;
