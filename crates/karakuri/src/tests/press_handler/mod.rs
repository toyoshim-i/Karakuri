//! Every control `karakuri-console` claims, against the presses this window answers.
//!
//! The console draws controls and turns presses on them into operations or intents;
//! [`Readout::pointer`] hit-tests each control and acts on what comes back.
//!
//! # Behavioral verification without source-text scraping
//!
//! Previously, this module verified coverage by scraping Rust source text from disk,
//! counting regex matches and cuts to check whether method calls appeared in the
//! press handler function. That text-scanning approach was fragile, bound to formatting,
//! and abolished across the codebase in P8.
//!
//! Today, controls registered in `karakuri_console::input::PROBES` and table rows are
//! verified through direct behavioral execution:
//! - Table alignment: Table mirrors `PROBES` 1:1 in length and order, ensuring
//!   every control probe registered by `karakuri-console` corresponds to the press
//!   handler's documented derivations and asks.
//! - Probe evaluation: Every probe function in `PROBES` is directly callable
//!   against a solved [`Readout`] panel layout and `egui::Context`.
//! - Behavioral dispatch: Pointer events (`Pointer::Moved`, `Pointer::Down`,
//!   `Pointer::Up`, `Pointer::Secondary`, `Pointer::Wheel`) are dispatched through
//!   `Readout::pointer`. When pointer events land on active controls (such as solo pills,
//!   bay grips, transport buttons, transition controls, mixer chips, and library elements),
//!   they claim the event ([`Claim::Panel`]) and dispatch to the appropriate handler
//!   logic, yielding actionable [`Acted`] outcomes or state transitions rather than
//!   silently dropping events.
//!
//! # Why the check is here and can be nowhere else
//!
//! The press handler is in this package, and nothing in this workspace may depend
//! on this package — it is a binary with no library target on purpose, so there is
//! no other crate that can see both halves of the seam.

mod dispatch;
mod table;
