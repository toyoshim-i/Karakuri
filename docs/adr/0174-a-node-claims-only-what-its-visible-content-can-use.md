---
id: 0174
title: A node claims only what its visible content can use
status: accepted
date: 2026-08-25
supersedes: []
superseded_by: []
principles: [0073]
tags: [ui]
---

# A node claims only what its visible content can use

## Context

`docs/manual/console.html` says of the program view sink: *"Turn it off and that picture goes,
**giving its height to the inspector**."* It did not. Probed at the mock's narrowest console,
collapsing `program-view`:

```
before: program h 378   inspector h 142   deck-previews h  72
after:  program h 378   inspector h 142   deck-previews h 378
```

`program` is `Fixed(378)` and stayed 378 because **the solve is purely top-down**: a parent's
rectangle decides what is available and no size ever flows upward. The preview row then swelled to
fill the bay, which the solver's "nothing flexible is left unfrozen" branch does deliberately and
correctly for the case its own comment names.

So the manual's sentence was a sentence about nothing, and an existing test asserted the swelling
as correct — with a comment arguing that the sentence was *"the sink's doing on the bay's height
and not the fold's"*. That was a rationalisation written after observing the behaviour. The manual
says the picture *"is on screen exactly when that sink is on"*, so the fold and the sink are one
state and cannot have two geometries.

## Decision

**A measuring pass, and one rule stated twice.**

`measure` walks the arena post-order before anything is placed and writes, per node, **how much it
can use** along its parent's axis — `Solved::usable`:

- a `Flex` leaf can use any amount (`f32::INFINITY`); a `Fixed` leaf can use exactly its stored size
- a split can use the sum of its **visible** children's `usable` plus the dividers between them,
  infinite if any of them is infinite, and **zero if none is visible**
- a node's own `max` caps it

Two claims are then capped by it, and they are the same sentence twice: a `Fixed` child claims
`min(stored, usable)`, and a declared minimum becomes `min(declared, usable)`. A minimum is what a
node needs *while its content is whole*; holding it with the content folded away is holding space
the node will leave empty.

**With the picture visible nothing changes at all** — `program-view` is `Flex`, so the bay can use
any amount, `min(378, ∞)` is 378 and `min(200, ∞)` is 200. Every existing assertion about 378 and
200 passes unedited. The rule is a no-op except where it bites.

**P-0082 is untouched and this is why it had to be checked**: a bottom-up pass is the shape a reader
expects to see mutating nodes. `measure` takes `&Arrangement` like the rest of the solve and writes
into the `Solved` buffers, so writing a size back into the arrangement still does not compile.

### A split laid out across its parent's axis can use any amount

Not stated when this was set out, and the code needs it: summing a column's children gives a
**height**, and its parent may be handing out **widths**. Adding them is arithmetic on two
different questions. So a split whose axis differs from its parent's is `f32::INFINITY`, and only
the *no visible children* case is checked before that. The console's `left-pane` stores a width of
218 and holds a 125-tall bay; a sum-always rule would have had it claim a width read off heights.

### The leftover branch is **not** capped, and this was tried and reverted

The obvious next move is to cap the "nothing flexible is left unfrozen" branch as well. **It is
wrong, and the session fuzzer caught it in one run:** a split whose children are all folded can use
zero, and `0 * scale` is zero for every scale, so a parent with nothing flexible could never fill
itself and left a permanent hole — a solo would leave the window mostly empty.

The distinction is worth keeping in the words the code now uses: **a cap says what a node would ask
for; in that branch nobody is asking, and the split is disposing of space no child claimed.** So
the solve carries two sums — the capped one for the flexible pool, the stored one for disposal —
and that branch's behaviour is bit-identical to before.

## Where this meets ADR-0157, which still describes what happens

[ADR-0157](0157-a-maximum-is-honoured-and-the-leftover-is-trailing-space.md) says a maximum is honoured
and the leftover becomes trailing space. It still is, in fewer places:

- A `max` on the node itself now caps `usable`, so a maximum is honoured earlier rather than by
  clamping after the claim. Same rectangle.
- Trailing space still appears exactly when every visible child is at its maximum; neither test
  suite's tiling check needed an escape clause widened.
- **The narrowing:** a `max` stated *inside* a fixed split now propagates outward. A split storing
  378 over a `max(100)` child and a 72 row used to claim 378 and hold 198 of trailing space inside
  itself; it claims 180 now, and a flexible sibling takes the 198. The space nothing may take is
  still left empty — it just arises less often, because a parent no longer asks for space its
  content cannot use.

## Consequences

- **The fold and the sink are one state with one geometry.** Folding `program-view` leaves the bay
  at 72, the preview row at its own 72 rather than swollen, and every pixel of the 306 with the
  inspector — and unfolding restores 378 and 142 exactly, because nothing was written back.
- **Two silent semantic changes, both on arrangements that contradict themselves.** A
  `Fixed(100).min(150)` view used to solve to 150 and now solves to 100; a `min(200).max(100)` used
  to give the minimum and now gives the maximum. No arrangement in the workspace has either, and
  both follow from the rule as stated — but *a fixed leaf is unaffected* holds only where
  `min ≤ size ≤ max`.
- **`reweight` and the solve now derive the same sum two ways.** `reweight` counts a fixed sibling
  at its stored size, and capping it there would read a measurement taken before `resize_pair` wrote
  the sizes it is inverting. So a drag can land short in a split holding a capped fixed child *and*
  two or more flexible ones. It is documented on `reweight` rather than fixed, and nothing is told a
  lie: `set_divider` re-solves and returns where the boundary actually landed. The console cannot
  reach it — `centre` has exactly one flexible child.
- **The roadmap's *a split's minimum is not derived from its children's* is half answered.** A
  ceiling is derived now; the declared minimum is still declared, and the test that recomputes it by
  hand still exists. Deriving it outright is a separate decision, because a declared minimum can be
  *larger* than the sum of its children's and often should be.
