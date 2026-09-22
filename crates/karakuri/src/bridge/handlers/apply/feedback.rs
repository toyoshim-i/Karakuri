use super::*;

/// What a model is told about an operation this window drained off `--mcp`,
/// and `None` where the answer is *it was performed*.
///
/// [`unwritten`]'s neighbour and its opposite audience: that one writes the
/// terminal's line, which reaches an operator who can see the deck and read
/// every other line this frame printed, and this one writes the reply that
/// leaves the process. A model has neither the terminal nor the window
/// ([ADR-0315](../../../docs/adr/0315-a-model-has-no-window-so-the-twelve-surface-rows-mcp-badges-are-gap.md)),
/// so an answer of *performed* over a refusal is the whole of what it is
/// given: it reads the fade as done, sees no frame, and asks for the next
/// thing.
///
/// The two that are answered, and they are the two [`written`] does not perform:
///
/// - [`Written::Refused`] is a decision taken. The move was not scheduled, the
///   lane still holds the fader, and what the next attempt needs is to mute
///   that lane
///   ([P-0083](../../../docs/principles/0083-a-refusal-carries-what-the-next-attempt-needs.md),
///   ADR-0323).
/// - [`Written::Owed`] is a gap nobody has closed. Nothing moved and nothing
///   here decides it, which is the terminal's own words for it one function up.
///
/// `None` for [`Written::Records`], which is a performance, **and `None` for
/// [`Written::Silent`], which is one too**. That is where this differs from
/// `karakuri-cli`'s `answered`, and the difference is the two programs rather
/// than the two sentences: that program performs an operation *by* writing the
/// records it converts to, so a `Silent` there is an operation it has no
/// control for. This window has the control. A `LoadSet`, a `SelectDeck`, a
/// `RouteFrame`, a `SetTransition` and a save all answer `Silent`, and every one
/// of them is performed a few lines above the conversion in `App::performed`
/// or in `App::operated` itself — telling a model that its `load_set` changed
/// nothing would be this fix committing the defect it repairs, in the other
/// direction.
///
/// The sentence is `karakuri_operation_record::not_performed`'s and not this
/// file's, for the reason `Refusal::why` is the refusal crate's: `--mcp` on
/// this program and `--mcp` on `karakuri-cli` are two front doors onto one
/// vocabulary, and one mistake gets one explanation whichever a model came
/// through (ADR-0131).
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

/// What a refused `go` says, and the whole of what this window puts on this
/// side of that seam.
///
/// `karakuri_console::view::Go` answers *which* refusal, because the console is
/// what can see a shape is unset and how many strips it drew; the sentence is
/// here because this package is the one that has anywhere to print. What each
/// of them owes is
/// [P-0083](../../../docs/principles/0083-a-refusal-carries-what-the-next-attempt-needs.md):
/// a rejection is a message to whoever makes the next attempt, and it carries
/// the constraint and where to go rather than *refused*.
///
/// `karakuri-cli`'s `c` prints the same two, and its wording is where these
/// come from — *"a wipe needs somewhere to come from — this deck holds one
/// slot"* and *"no mask shape — `z` chooses one, and a wipe is a shape
/// moving"*. What changes on this surface is where the next attempt is made: a
/// pill two capsules to the left rather than a key.
pub(crate) fn refusal(refused: &Go, decks: usize) -> String {
    match refused {
        // **Unreachable while this window builds a full deck** — `SLOTS` is
        // `MAX_SLOTS` and the mixer draws one strip per slot — so what this
        // answers for is a console drawn before the deck reached it, and it
        // is written rather than left to be an unexplained silence. The count
        // is said because it is the thing that would have to change.
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
