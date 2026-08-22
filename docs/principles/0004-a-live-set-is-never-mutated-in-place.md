# A live Set is never mutated in place

Changing a Set's node topology or any procedure inside it produces a **new Set**. The
running one is not edited. Parameter values are the single exception, and they are uniform
writes — nothing about the graph moves, which is why they need no fork.

**What it rules out.** Editing a Set that is on air, which would make a frame observable
mid-edit and make "what was running at time *t*" unanswerable. It also rules out treating a
fork as a fallback for expensive edits only: the fork *is* the edit mechanism, which is what
lets a candidate be built and primed off air and then swapped in whole.

**Where it holds.** [set.rs](../../crates/karakuri-engine/src/set.rs);
[karakuri-engine/src/lib.rs](../../crates/karakuri-engine/src/lib.rs).
