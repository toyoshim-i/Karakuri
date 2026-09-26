use super::*;

/// Returns the diagnostic message sent to an MCP caller if an operation was not performed.
///
/// Returns `None` if the operation was performed (records written or handled silently; ADR-0131, ADR-0315, ADR-0323).
pub(crate) fn unperformed(title: &str, written: &Written) -> Option<String> {
    match written {
        Written::Records(_) | Written::Silent(_) => None,
        Written::Owed(owed) => Some(not_performed(title, owed.why())),
        Written::Refused(refusal) => Some(not_performed(title, &refusal.why())),
    }
}

/// Generates a diagnostic message when an operation produces no persisted record,
/// distinguishing between settled silence, missing readings (owed), or refusal (ADR-0323).
pub(crate) fn unwritten(operation: &Operation, written: &Written) -> Option<String> {
    match written {
        Written::Records(_) => None,
        Written::Silent(silent) => Some(format!(
            "  emitted: {operation:?} -> no record, and that is settled: {}",
            silent.why()
        )),
        Written::Owed(owed) => Some(format!(
            "  emitted: {operation:?} -> no record, and that is a gap rather than a \
             decision: {}. nothing moved, and nothing here decides it",
            owed.why()
        )),
        // **A decision rather than a gap, so it is not the line above**
        // (ADR-0323). Nothing was scheduled and no record was written, which is
        // what a replay of this session will also see.
        Written::Refused(refusal) => Some(format!(
            "  emitted: {operation:?} -> refused, and nothing was scheduled: {}",
            refusal.why()
        )),
    }
}

/// Formats a refusal message when a `go` (wipe) command cannot be executed (P-0083).
pub(crate) fn refusal(refused: &Go, decks: usize) -> String {
    match refused {
        // Fallback if the mixer was drawn with fewer decks than required for a wipe.
        Go::NoOtherDeck => format!(
            "  wipe: refused — this mixer draws {decks} strip{}, and a wipe needs a deck to \
             come from as well as one to arrive. Two decks side by side in the bay is what \
             makes the press mean something",
            match decks {
                1 => "",
                _ => "s",
            }
        ),
        Go::NoShape => String::from(
            "  wipe: refused — no shape chosen, and a wipe is a shape moving. The first pill \
             on this row is where one is picked: it reads `no shape` now, and a press on it \
             takes it to `left`. The vocabulary refuses this one too — \
             `Operation::Wipe` is *refused with no shape chosen* at its own definition — and \
             the refusal is here because the shape is this console's own setting",
        ),
        // A wipe is not a refusal, and this arm exists so that the day a
        // fourth answer lands somebody has to say what it reads rather than
        // a wildcard printing one of these two over it.
        Go::Wipe(operation) => format!(
            "  wipe: {} is not a refusal and this line should not have been reached",
            operation.title()
        ),
    }
}
