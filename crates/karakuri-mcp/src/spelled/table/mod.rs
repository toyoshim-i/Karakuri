use std::sync::LazyLock;

use super::*;

pub(crate) mod arrangement;
pub(crate) mod decks;
pub(crate) mod library;
pub(crate) mod master;
pub(crate) mod mixing;
pub(crate) mod procedures;
pub(crate) mod run;
pub(crate) mod sequencer;
pub(crate) mod set;
pub(crate) mod transport;

/// Every operation of the vocabulary, and how this surface spells the ones it takes.
///
/// Sixty-four rows, one per `<h3>` of `docs/manual/operations.html`, in that
/// page's order.
pub(crate) static SPELLED: LazyLock<Vec<Spelled>> = LazyLock::new(|| {
    let mut all = Vec::with_capacity(64);
    all.extend_from_slice(transport::TRANSPORT);
    all.extend_from_slice(decks::DECKS);
    all.extend_from_slice(mixing::MIXING);
    all.extend_from_slice(master::MASTER);
    all.extend_from_slice(sequencer::SEQUENCER);
    all.extend_from_slice(set::SET);
    all.extend_from_slice(library::LIBRARY);
    all.extend_from_slice(procedures::PROCEDURES);
    all.extend_from_slice(arrangement::ARRANGEMENT);
    all.extend_from_slice(run::RUN);
    all
});
