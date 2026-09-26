//! Verifies press handler control dispatch: ensures `PROBES` match `ASKED` table entries 1:1,
//! probes evaluate against solved layouts, and pointer events route to active handlers.

mod dispatch;
mod table;
