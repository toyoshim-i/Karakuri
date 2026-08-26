---
id: 0183
title: A node is out of the layout for two reasons, and they are two bits
status: accepted
date: 2026-08-26
supersedes: []
superseded_by: []
principles: []
tags: [ui]
---

# A node is out of the layout for two reasons, and they are two bits

## Context

The Program bay is about to arrange itself: with a wide, short bay the four deck previews move from
a row under the picture into two columns beside it
([ADR-0182](0182-the-program-bays-body-arranges-itself-for-the-larger-picture.md)). When they are
beside, the node that holds the row must claim **no height**.

**The only mechanism in `karakuri-layout` that takes a node's height away was `collapsed`, and that
is the operator's fold.** Sharing it makes the operator's fold and an automatic rearrangement
indistinguishable: an unfold in the rearranged state would put an empty strip back, and a fold the
operator never made would be saved into their arrangement.

## Decision

**A node is out of the layout for two reasons and they are two bits.**

- **The operator's fold** — `collapse`, `expand`, `toggle`, what `Op::Fold` writes, saved with the
  arrangement and restored by a load.
- **`set_aside`** — written by whoever is drawing, because it has put that region somewhere else or
  has no room for it. **Never saved.**

**One private predicate reads both**, and nothing else reads either flag directly. `is_collapsed`
keeps meaning *the operator folded it*; every *should this be laid out* reader — `visible`,
`visible_children`, `hit`, `divider_at`, `reweight`, `visible_count`, `visible_child` and everything
that follows it, `measure`, and the solve's freeze and divider placement — asks the disjunction.

### The name

`view` was the trap: **in this crate `view` means a leaf node**, so *"set aside by the view"* — the
phrase the decision arrived as — could not become the name. `set_aside` keeps the half of that
phrase which was right, and reads correctly both ways: *set `aside` on this node*, and *set this
node aside*. `set_x(id, bool)` is the crate's existing shape for a write.

**A single boolean setter rather than a verb pair is deliberate.** The caller writes this every
frame from the geometry, so an idempotent write is the natural shape; a `stow`/`restore` pair would
mirror `collapse`/`expand` and make the operator's fold and this look like the same kind of thing,
which is the confusion the decision exists to prevent.

Rejected, and the reasons are about what each word claims: **`omitted`** reads as an oversight
rather than a decision, and *omitted from what* has no answer in a crate that does not know what it
is drawing; **`elsewhere`** and **`displaced`** cover *put somewhere else* and not *no room for it*,
which is half the reason; **`hidden`** and **`suppressed`** read as permanent, and as a *stronger*
fold rather than a different one; **`withheld`** implies from someone.

### It is not saved, and `check_structure` has nothing to say

`#[serde(skip)]`. A load always yields false regardless of the file *or* of the arena it was
assembled into, because the bit is a pure function of the geometry and is re-derived on the next
solve. `Spec` gained nothing: it is not the arrangement's to declare.

[ADR-0158](0158-a-saved-arrangement-that-disagrees-with-itself-is-refused.md) refuses a file that
disagrees with itself, and **a field that is never written cannot**. A check would need a second
copy of the bit in the file to compare against, and putting one there in order to check it is
exactly what not saving it avoids. That argument is written where the other unchecked things are.

### Writing the same value marks nothing dirty, and that is P-0072's first clause

The caller writes this bit every frame from the geometry. **A layout that went dirty on every write
would cost the panel *a still panel does no per-frame work*.** `expand` on a node that is not
collapsed already behaved this way; `set_aside` does too, and it is asserted rather than assumed —
the injected non-idempotent version fails with `rect() read a stale solve`.

### Solo snapshots the operator's fold and only that

Unchanged code, and that is the conclusion rather than an omission. Saving `aside` would mean
restoring, at the unsolo, a bit the caller has since re-derived from a geometry the solo itself
changed — and it would need a second saved vector, which is a wire field.

**So a node that is set aside stays set aside through a solo and after the unsolo, including the
soloed node itself** — left holding the whole viewport and still not laid out, with a zero
rectangle, until whoever set it aside says otherwise. **A solo says which region is alone on screen;
it does not say whether the caller is drawing that region here, and the crate must not guess.**

## Consequences

- **P-0071 and P-0073 are untouched.** `set_aside` is an operation reached through `node_mut`, so
  the solve still holds the arrangement by `&` and cannot write to it — and `lib.rs`'s P-0071
  paragraph now names it with the sentence that matters: **taking a region out is an operation
  however derived the value being written is.** The caller works it out and then tells the layout.
  A split's `usable` is zero when none of its children is laid out, unchanged, and a set-aside child
  is skipped in `measure` exactly as a folded one is — which is what makes the motivating case work.
- **No guard was built here** against a bay vanishing when the picture is folded *and* the row is
  set aside. That rule — *rearrange only while the picture is visible* — is the caller's, and a
  crate that does not know what a picture is may not hold it.
- **Nothing in `karakuri-console` had to move.** Every caller was checked: the ones that draw
  already ask `visible` and `visible_children`, and the ones that ask `is_collapsed` genuinely mean
  the operator's fold.
- **One test helper was reading the wrong question**, and it was load-bearing rather than cosmetic:
  `assert_tiles` asked `is_collapsed` for *does this child take extent and a divider*, and with the
  old predicate the new geometry test fails inside the helper counting a phantom divider.
- **Two documents said one thing while meaning another, before this.** `is_collapsed`'s doc answered
  a question about *reasons* with a sentence about *ancestors*; `visible_children`'s said *"only the
  children's own flags are read"* — plural, when there was one. Both now say which question they
  answer.
- *One bit for both* is caught by four of the six tests, and **the two that miss it are named**: the
  geometry is identical under one bit, which is exactly why the defect is attractive. What catches
  it is the flags, the file, and the solo.
