---
id: 0175
title: An operation carries what it acts on, and unfolding means making a region visible
status: accepted
date: 2026-08-25
supersedes: []
superseded_by: []
principles: []
tags: [ui]
---

# An operation carries what it acts on, and unfolding means making a region visible

## Context

`Op::Fold` meant *"fold the region under the pointer"* — the pointer was part of what the operation
**was**, in the operation's own doc comment. That works while a pointer is the only surface, and M5
opens on the opposite premise: *each operation is named once, and every surface routes into that
name.* A MIDI control has no pointer, an MCP call has no pointer, and the console's own Outputs dot
folds the picture while the pointer is at the other end of the panel.

## Decision

**The operations that act on a node take the node**: `Op::Fold(NodeId)`, `Op::Unfold(NodeId)`,
`Op::FoldEnclosing(NodeId)`, `Op::Solo(NodeId)`. The ones that act on the arrangement take nothing:
`UnfoldAll`, `Unsolo`, `Reset`, `Report`.

*Under the pointer* stops being part of an operation's meaning and becomes **one way of naming
which region** — `Panel::under()`, which a key press resolves before emitting and a drawn control
does not need at all. It resolves at **zero grab**: a grab exists so a hand can find a nine-pixel
gap, and an operation that took one would fold the wrong region within six pixels of every edge.

### Unfolding is not folding backwards

**`Fold(id)` folds one node. `Unfold(id)` makes one node *visible*** — it expands `id`, every
collapsed ancestor of it, and a solo that is what is keeping it off screen.

**Hiding a thing hides one thing; showing a thing means showing the way to it.** That is the
ordinary shape of revealing a node in a tree, and it is why the two are not each other's inverse.

**The alternative is `expand(id)` and nothing else, and the console's one control settled it.**
With the Program bay folded around the picture, the Outputs dot is dark; a press that expanded the
picture *inside a still-folded bay* would light nothing, move nothing and say nothing. The first
version of this code did exactly that and argued in its own doc comment that it was *"the
arrangement having two levels of fold and not this control lying"*. That is an explanation of a
mechanism standing in for a design, and `docs/manual/console.html` had already ruled the other way
in the note on that very row: *"Nothing is refused here, so nothing has to be explained: a control
that quietly declines the last of something is a rule an operator can only find by experiment."* A
control that appears not to respond is that rule with no words at all.

The property is checkable rather than argued, and is a test: **a press on a lit dot darkens it and
a press on a dark dot lights it, always** — asserted at three distances, with the picture folded,
with the bay folded around it, and with the whole centre column folded two levels up.

`UnfoldAll` keeps its own meaning and is not folded into this: it is what reaches a fold **nobody
has a name for**.

### The solo is dropped only when it is the thing hiding the region, and it is dropped first

`Layout::solo` snapshots every node's collapsed flag, then collapses everything off the solo path;
`unsolo` writes that snapshot back. So **expanding under a live solo is quietly undone**: the next
`unsolo` restores the snapshot and throws the expand away with nothing said. Dropping the solo
before expanding is what avoids it, and the ordering is reachable by an operator — fold the
picture, *then* solo the Outputs row, then press the dot. Expanding first puts the fold straight
back and the press does nothing.

## Consequences

- **`Fold` was a toggle that could never toggle.** `Layout::hit` skips collapsed children, so the
  pointer can never be over a folded region: `f` twice never unfolded anything, and
  `Outcome::Folded.folded` was always `true` when reached from a key. Making the operations
  directional changed no behaviour; it made the vocabulary say what it always did. `folded` is now
  read back out of the layout rather than assumed, so an operation that found the node already
  there reports the truth rather than the intent.
- **`Outcome::OnDivider` is gone** — nothing can produce it. *The pointer is on a divider* is what a
  resolution finds, not what a fold reports.
- **Two repaint tests only compiled because `Op` was pointer-relative** — a fold with nothing under
  the pointer, and a fold with the pointer on a divider. Those keys now emit no operation at all,
  so the panel is stiller than it was. Said where the tests are rather than quietly rewritten.
- **`FoldEnclosing` meant two things** — for a divider, that split; for a region, its parent. With a
  named target it can mean only one, so the harness emits `Op::Fold(split)` over a gap and
  `Op::FoldEnclosing(id)` over a region: the same two arms, out where the pointer is.
- **`Layout::soloed()` can lie, and only convention stops it.** The solve never reads `soloed`, and
  `check_structure` checks only that it addresses a node — the solo's exclusivity lives entirely in
  collapsed flags that any later `expand` or `collapse` may contradict. The rule above never leaves
  that state, so this is latent rather than live; it is recorded because it is an invariant held by
  nobody. Deciding what `expand` under a solo *should* do is a separate decision, and it is owed.
