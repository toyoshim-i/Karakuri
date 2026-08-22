# A live Set is never mutated in place

Changing a Set's node topology or any procedure inside it produces a **new Set**. The
running one is not edited. Parameter values are the single exception, and they are uniform
writes — nothing about the graph moves, which is why they need no fork.

**What it rules out.** Editing a Set that is on air, which would make a frame observable
mid-edit and make "what was running at time *t*" unanswerable. It also rules out treating a
fork as a fallback for expensive edits only: the fork *is* the edit mechanism, which is what
lets a candidate be built and primed off air and then swapped in whole.

**Value and instance are different things.** A **Set value** is immutable, content-addressed and
appears in records; a **compiled instance** is the thing on the GPU, and a change needing neither
reallocation nor recompilation may be applied to it in place. That is what makes editing in the
background cheap without making a Set mutable — an unsaved intermediate simply never exists.
Genuinely mutating a Set would open a hole in the record stream, and replay, undo, A/B comparison
and session recording are lost together.

**Where it holds.** [set.rs](../../crates/karakuri-engine/src/set.rs);
[karakuri-engine/src/lib.rs](../../crates/karakuri-engine/src/lib.rs). Sharpened in
[ADR-0027](../adr/0027-a-set-value-is-immutable-and-its-compiled-instance-is-not.md).
