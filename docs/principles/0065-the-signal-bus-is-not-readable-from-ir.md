# The signal bus is not readable from IR

`energy`, `beat`, `band`, `bpm` and the rest cannot be named inside a `.kir`. To react to audio or
tempo, a procedure declares a `param` and a `bind` record in the Set file attaches a signal to it.

**What it rules out.** Reaching a live value from inside a procedure. The coupling would be invisible
to everything outside the shader: the UI could not show it, an agent could not adjust it, and the
persistence layer could not record it. Going through a `param` plus a `bind` keeps **every external
coupling declarative**, so all three see the same thing — and a bound value carries a curve, a range
and a confidence blend that a direct read has nowhere to put.

**One exception, and it is deliberate.** `beats` reaches the IR as an **ambient**, with no `param` and
no `bind`, because what follows a tempo is not a parameter value but **the passage of time** — a
binding writes one number where a clock has to move everything the procedure does.

**Generators violate this readily**, which is why the specification says to state it in the generation
prompt, and why the checker's refusal names the fix rather than the mistake: *the signal bus is not
readable from IR — declare `param energy` and attach a `bind` record to it in the Set file.*

**Where it holds.** [ir-spec.md](../ir-spec.md), "Ambient values and the signal rule"; `check.rs` in
[karakuri-ir](../../crates/karakuri-ir). It predates the recovered history — it is in the first
committed specification — so there is no ADR behind it, only this.
