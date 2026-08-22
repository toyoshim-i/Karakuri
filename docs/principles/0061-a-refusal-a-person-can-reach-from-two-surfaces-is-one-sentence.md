A mistake that can be made from more than one surface is refused in **one sentence, from
one function**. Four spellings of *no such slot* lived in the key handler, the MCP tool,
the MIDI router and the mix path, and a model and an operator making the same mistake on
the same control were answered differently.

Per-surface wording is what loses. It reads as harmless — each sentence is fine on its own —
and it is discovered by reading four files rather than by a failing test, because an
assertion that a message *contains* the offending value passes under every variant. Assert
equality against the shared function instead.

The rule is about **surfaces that face a person or a model**. Panic and `assert!` text
addressed to whoever is holding a debugger is a different register and may differ; say so
where it differs, so the exclusion reads as a decision rather than as the drift this rules
out. See [ADR-0131](../adr/0131-one-refusal-sentence-per-mistake-across-the-surfaces-that-face-a-person.md).
