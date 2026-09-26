use super::*;

/// Helper to extract single records from operations that require no contextual reading (`Current::default()`).
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

/// Parses and checks a single `.kir` source file using `karakuri_environment::compile::load`.
pub(crate) fn checked(path: &std::path::Path) -> karakuri_ir::typed::Checked {
    match karakuri_environment::compile::load(path) {
        Ok((checked, _)) => checked,
        Err(report) => panic!("{report}"),
    }
}

/// Resolves the fallback preset library path from the current workspace root for disk-backed tests.
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

/// Supplies default test slot configurations without creating isolated disk working copies.
#[cfg(test)]
pub(crate) fn shipped_slots() -> Vec<Sources> {
    std::iter::repeat_n(shipped(), SLOTS).collect()
}

/// Resolves reference workload pair paths (`examples/drift_cloud.kset`) for compute budget evaluation (ADR-0270, ADR-0271).
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
