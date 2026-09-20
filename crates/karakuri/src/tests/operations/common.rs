use super::*;

/// The one record an operation writes, for the tests that know there is exactly
/// one.
///
/// For the four whose record needs no reading at all, which is where
/// `Current::default()` — *I read nothing* — is the honest answer. A conversion
/// that answered anything but a single record for one of those four is this
/// file's assumption breaking rather than a test needing a helper, which is why
/// the panic says so.
///
/// The mask's operation is not one of them and must not be passed here: its
/// record is written out of the operation *and* a reading of the running mask
/// (ADR-0201), so it would come back `Owed(NotRead)` and this would panic —
/// correctly, and saying which operation. What the mask's tests hand in is a
/// reading, through [`reading`] where there is a deck and by hand where there
/// is not.
pub(crate) fn only_record(operation: &Operation) -> Record {
    match written(operation, &Current::default()) {
        Written::Records(records) if records.len() == 1 => records.into_iter().next().unwrap(),
        other => panic!(
            "a control's operation did not write exactly one record: \
             {operation:?} -> {other:?}"
        ),
    }
}

/// [`tally`] the other way round, for the assertion above alone — the
/// vocabulary's residency as the engine's, so that the round trip through the
/// wire name can be compared against something.
pub(crate) fn residency_back(residency: karakuri_operation::Residency) -> Residency {
    match residency {
        karakuri_operation::Residency::Live => Residency::Live,
        karakuri_operation::Residency::Priming => Residency::Priming,
        karakuri_operation::Residency::Allocated => Residency::Allocated,
    }
}

/// [`blend_mode`] the other way round, for the assertion above alone — which is
/// why it is here and not beside it: nothing the program *runs* needs to go
/// this direction, and a conversion in `src` with one test as its only caller
/// would be an abstraction with no second call site.
pub(crate) fn blend_mode_back(blend: BlendMode) -> Blend {
    match blend {
        BlendMode::Add => Blend::Add,
        BlendMode::Over => Blend::Over,
        BlendMode::Max => Blend::Max,
    }
}

/// One `.kir`, parsed and checked, for the tests that need a `Checked` and no
/// window.
///
/// Reachable from test modules because it is at the file's own scope.
///
/// `karakuri-environment`'s own five stages and not a sixth spelling. This used
/// to be a hand-rolled parse-then-check, which is what the run itself used to
/// build a slot from; the run compiles through
/// [`karakuri_environment::compile::sort_slot`] now, because that is the one
/// place that keeps the bytes a node's address is derived from
/// ([`karakuri_environment::compile::Placed::source`]). What is left here is a
/// test helper, and a test helper with its own compiler would be a second
/// answer to *does this file check* the day either moved.
pub(crate) fn checked(path: &std::path::Path) -> karakuri_ir::typed::Checked {
    match karakuri_environment::compile::load(path) {
        Ok((checked, _)) => checked,
        Err(report) => panic!("{report}"),
    }
}

/// The pair a bare run plays, for the tests that need one on the disk.
///
/// [`Sources::under`] takes a preset library and does not go looking for one;
/// this is the going-looking, and in a test binary the answer is always the
/// last candidate — the workspace this file was compiled in, which is also the
/// tree the test is run from. That is the development entry doing exactly what
/// it is for, and it is why these tests can assert the pair is on the disk
/// without an install anywhere.
///
/// A function rather than an `impl Default` on [`Sources`], because a `Default`
/// is what baked the build machine's own tree into a shipped binary: a type
/// whose default value is a search of the filesystem invites exactly that call
/// from production, and a production caller now has to say which library it
/// means.
#[cfg(test)]
pub(crate) fn shipped() -> Sources {
    let presets = karakuri_environment::places::presets(None)
        .expect("nothing was typed, so there is no typed path to refuse")
        .expect(
            "no preset library was found from the test binary, so the workspace tree this \
             test compiled in has no `examples/` in it",
        );
    Sources::under(&presets.dir)
}

/// The shipped pair in every slot, for the tests that build an [`Engine`].
///
/// A *run* may not do this — [`working_copies`] is what a run calls, and its
/// whole point is that no two slots watch one file — and this helper is not a
/// way back to that. It is legal here for the reason the copies exist: nothing
/// in these tests edits a `.kir`, no watcher of theirs ever sees a change, and
/// a test that materialised into a temporary store would be asserting the
/// copies rather than the thing it is about. The one test that *is* about the
/// copies calls `working_copies` and is named after the claim.
#[cfg(test)]
pub(crate) fn shipped_slots() -> Vec<Sources> {
    std::iter::repeat_n(shipped(), SLOTS).collect()
}

/// The reference workload's pair, for the tests whose claim is about a cost
/// rather than about what this program opens on.
///
/// `docs/contributing.md` §1 names `examples/drift_cloud.kset` —
/// `drift_shell.kir` at the 262144 elements it declares, with `soft_points.kir`
/// — and this resolves those two out of the same preset library [`shipped`]
/// answers from. It is deliberately not [`shipped_slots`], and the two were one
/// value until 2026-09-07.
///
/// What separated them is a test going quiet rather than red.
/// [`ADR-0271`](../../../docs/adr/0271-the-panel-opens-on-the-demo-rather-than-on-the-reference-workloads-pair.md)
/// moved the default pair to `examples/star_vortex.kset`'s two parts, which are
/// closed-form and 10240 elements.
/// `gpu::the_budget_parks_a_deck_and_the_strip_carries_both_residencies` then
/// measured 1.8 ms a slot against a 2.7 ms headroom and the governor answered
/// `NoPrimingNeeded` — a closed-form Set with nothing to warm — so the park the
/// test is named for was still a park and no longer the budget's. Which pair a
/// bare run opens on is a demo decision (ADR-0270); whether the budget refuses
/// a second Live slot is not, and it needs material chosen for its cost.
#[cfg(test)]
pub(crate) fn reference() -> Sources {
    let shipped = shipped();
    Sources {
        l1: shipped.l1.with_file_name("drift_shell.kir"),
        l4: shipped.l4.with_file_name("soft_points.kir"),
    }
}

#[cfg(test)]
pub(crate) fn empty_keeping() -> Keeping {
    let (_, built) = std::sync::mpsc::channel();
    let (save_tx, saves) = std::sync::mpsc::channel();
    let (send_tx, sends) = std::sync::mpsc::channel();
    let (keep_tx, keeps) = std::sync::mpsc::channel();
    Keeping {
        mcp: None,
        slot_policies: karakuri_environment::SlotPolicies::new(),
        playing: Playing {
            playing: Vec::new(),
        },
        built,
        pending: Vec::new(),
        saves,
        save_tx,
        sends,
        send_tx,
        keeps,
        keep_tx,
        in_flight: 0,
    }
}
