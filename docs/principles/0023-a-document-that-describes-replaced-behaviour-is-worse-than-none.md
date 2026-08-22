# A document that describes replaced behaviour is worse than none

A change that inverts a design goes and finds the text describing the old one — its own comments
included — and rewrites it as history or deletes it. Stale documentation is a defect of the change
that made it stale, not housekeeping for later.

**What it rules out.** Leaving the old description in place because the code is right anyway. The
costs are not symmetric: **missing** documentation sends a reader to the code, **confident and
wrong** documentation makes them act. `layout.rs` and `set.rs` both said the tail of the draw range
holds the dead elements. It is the reverse, the implementation is correct because the vertex stage
reads a per-element flag, and anyone believing the comment optimises by truncating the range and
drops living elements.

**The hardest case to see is your own.** When `t` moved from per frame to per substep, the sentence
saying `t` is constant across a frame's substeps was left standing **inside the function that
implements the change**.

**Where it holds.** Decided in
[ADR-0031](../adr/0031-a-document-describing-replaced-behaviour-is-worse-than-none.md).
