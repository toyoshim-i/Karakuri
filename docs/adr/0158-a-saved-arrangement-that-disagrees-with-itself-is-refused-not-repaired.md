---
id: 0158
title: A saved arrangement that disagrees with itself is refused, not repaired
status: accepted
date: 2026-08-23
supersedes: []
superseded_by: []
principles: []
tags: [ui, format]
---

# A saved arrangement that disagrees with itself is refused, not repaired

## Context

An operator's panel arrangement is saved and reloaded, which makes it a file — and a file gets
truncated, hand-edited, and written by a version that is not this one.

`karakuri-layout` stores its arrangement as an arena: a flat list of nodes, each split holding a
list of child indices, each node holding its parent's index, with the rectangle buffer and the
solo bookkeeping parallel to it. Every one of those is an invariant that `serde` knows nothing
about, and the failures are not subtle. A `root` outside the list indexes past the rectangle
buffer. A cycle in the children lists overflows the stack and aborts the process. **A cycle in the
parent pointers hangs** — `visible()` and `is_ancestor()` walk *up* — and a hang is the worst of
the three, because it leaves nothing to read afterwards.

Deserialisation was returning `Ok` for all of them, which is
`docs/contributing.md` §3 (P-0089 until it was retired on 2026-09-05) exactly: the
check ran clean and the program came up short later.

## Decision

**A load that returns `Ok` returns an arrangement that can be solved, hit-tested and operated**,
and everything needed for that is checked in one pass at load.

Two of those checks decided something beyond bookkeeping, because in both the file could have been
*repaired* instead.

**A `parent` that disagrees with the children list is refused rather than rebuilt.** The parent
pointer and the children list are one fact written twice, and a walk from the root could simply
overwrite the pointers with what it found — making the corruption unrepresentable and the loader
strictly more forgiving. It loses because a file that disagrees with itself is a file to report.
Silently correcting it turns "this session file is damaged" into a panel that opens with regions
somewhere the operator did not put them, and nothing anywhere says why.

**A node nothing reaches from the root is refused rather than ignored.** Unreachable debris
neither crashes nor hangs, so accepting it — or compacting it away on load — is available and
cheap. It loses on what the arena means: the parallel buffers are indexed by node and are only
meaningful for a tree, and a region dropped in silence is an operator's pane that has vanished
with no diagnostic. A pane that is gone and a pane that was never saved look identical afterwards.

**What is deliberately not checked**, because refusing it would make a file and the code that
writes it disagree about what an arrangement is: a `min` above a `max`, a negative or NaN size, and
a split with no children. All three are expressible in a `Spec` and accepted by `Layout::new`, and
the solve is total over them — it clamps at zero, its passes are bounded, and leftover space is
trailing ([ADR-0157](0157-a-maximum-is-honoured-and-the-leftover-is-trailing-space.md)). A loader
stricter than the constructor would refuse files this program itself writes.

**`Layout::new` gains no checking.** A `Spec` is a nested literal and cannot express any of these:
there is no index to be out of range and no way to write a cycle. Only the file can.

## Alternatives

**Derive `parent` at load, or stop serialising it.** The most attractive of the three, since it
removes a whole class of corruption rather than detecting it. Rejected here and worth re-proposing
only with the diagnostic answered: it also removes the ability to say the file is wrong.

**Repair what can be repaired, refuse the rest.** Rejected for having no line anyone can state.
Every rule above is repairable by some rule, so "repairable" would come to mean "whatever was
implemented", and the operator could not predict which damaged file opens and which does not.

**Validate lazily, at the first solve.** Rejected on where the error can be reported: loading a
session is a moment with a person in front of it, and a frame is not.
