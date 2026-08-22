# A rejection carries what the next attempt needs

There is one severity. An error means no artifact, there are no warnings, and the answer to a
rejection is to generate again rather than to proceed with annotations. It follows that every
rejection must carry enough to aim the next attempt: a cost rejection states the estimate, the
ceiling and the call that dominates; a contract rejection names the fix.

**What it rules out.** "Over budget" as a message — a model cannot tell how much to remove. It
also rules out adding a warning severity: a diagnostic that neither stops a build nor gets read
is a diagnostic nobody acts on, and the author here is a program in a loop.

**Where it holds.** [ir-spec.md](../ir-spec.md), diagnostics; `cost.rs` in
[karakuri-ir](../../crates/karakuri-ir). Decided in
[ADR-0012](../adr/0012-one-severity-and-a-rejection-carries-numbers.md).
