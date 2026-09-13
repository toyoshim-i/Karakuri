---
id: 0348
title: A chain slot's cut is set through the parameter row, and a position the chain has not got writes nothing
status: accepted
date: 2026-09-13
supersedes: []
superseded_by: []
principles: [0090, 0092]
tags: [vocabulary, console, mcp, m5]
---

# A chain slot's cut is set through the parameter row, and a position the chain has not got writes nothing

## Context

[ADR-0340](0340-kind-l5-is-written-and-the-master-chain-is-an-ordered-list-of-them.md) §7 retires
*Feedback*, *Bloom* and *RGB shift* and names **three** rows in their place: *Set a chain effect's
parameter*, addressed by position in the chain and parameter key, *Add an effect to the master
chain*, and *Remove an effect from the master chain*. `docs/roadmap.md`'s M5.16 says the same
three.

**A slot holds two kinds of thing.** ADR-0340 §2 and §4: a slot is one `kind L5` procedure, the
values of the parameters that procedure declares, and — where it declares `retains` — the cut it
reads, `mix` or `exit`. `Record::MasterChain` carries them as two different fields for that
reason, `params` keyed by declaration name and `cut` a word.

**The cut is an operator's choice and was decided to be one.**
[ADR-0317](0317-the-master-chain-is-three-fixed-passes-and-feedback-reads-either-cut.md) put the
maintainer's answer — *both, selectable* — into the vocabulary, and ADR-0340 §2 keeps it:
*"Which cut is the slot's answer"*, the file declaring `retains` and saying nothing about which.
The Master bay draws the chip that picks it, and `docs/manual/console.html` specifies it.

So the three rows ADR-0340 names have to carry the cut somewhere, or an operator loses a choice two
records went out of their way to give them. ADR-0340 does not say where, and the row it does name
for the cut is the add: *"an add names a procedure by content address and, where the procedure
declares `retains`, a cut"*.

**And the address is now a position rather than a pass.** Under `SetFeedback` the conversion
appended a slot where the chain had not got one — ADR-0340's *for now*, and the only thing a row
naming a pass could do. A row naming a *position* has no such move: position 3 of a chain of two
is not a slot, and the vocabulary has never had an answer for an address the state does not hold
on this side of the seam, because every other address is a deck slot that the record applier
refuses. Here the record is built **from** the reading, so a position nobody holds cannot reach
the record at all.

## Decision

**1. `SetChainParam` carries a position and one of two settings.**

```rust
SetChainParam { at: u32, param: ChainParam } => "Set a chain effect's parameter",

pub enum ChainParam {
    Declared { key: String, value: f32 },
    Cut(Cut),
}
```

A declared parameter at a value, or the cut the slot reads — one of the two, never both. That is
what ADR-0192 asks of an ask: a surface says the thing it moved, and the conversion completes the
whole-chain record from the chain that is running.

**2. `AddChainEffect` carries the cut as well, because an add has to.** A slot whose procedure
declares `retains` and whose record carries no cut is refused where the chain is built
(`ir-spec.md`, *L5's chain*), so the cut is part of what makes a slot and not only of what changes
one.

**3. A position the chain has not got writes no record and says so.**
`karakuri_operation_record::Owed::NotInChain` — *"the master chain that is running has no slot at
that position"* — for `SetChainParam` and for `RemoveChainEffect`.

## Alternatives rejected

**A fourth row: *Set a chain effect's cut*.** The cleanest read of *one row, one thing*. It loses
on the count two records already fixed: ADR-0340 §7 and the roadmap both say three rows arrive, and
M5.16's exit is measured over the Master bay's rows. It also splits what a reader of the page sees
into two rows that differ only in the type of one field, where the page's own rule is that a row is
an act an operator performs — and picking a cut and moving an amount are the same act on the same
slot from the same row of the bay.

**The cut riding along with every amount**, as `Feedback { amount, cut }` did. It is refused by the
argument that record carried in the first place, read one layer up
([ADR-0192](0192-an-operation-asks-for-what-a-surface-can-say-and-the-record-stays-whole.md)): a
payload that names a value nobody asked for re-asserts it, so a fader ride would overwrite a cut
somebody chose a moment earlier, sixty times a second. `Feedback` held the two together because the
*record* needed both; the record still needs both and now gets them from the chain that is running.

**The cut only at the add.** It fits ADR-0340's three rows exactly and costs one clause. It also
takes the choice away: an operator who wanted the other cut would have to remove the slot and add
it again, losing every parameter on it, and the console's chip would have no operation to emit. The
two cuts are *different pictures* (ADR-0317), which is the whole reason both exist.

**A reserved key — `cut` as a `ChainParam::Declared`, with 0 and 1 for the two words.** One
variant, no enum. It puts a word from a closed list into a float, makes `cut` a declaration name no
procedure may use, and would reach `Record::MasterChain`'s `params` map, where the record says the
keys are the procedure's own.

**Appending on a position the chain has not got**, which is what `SetFeedback` did with a
procedure. Under a pass it was the honest reading of *turn feedback up*; under a position it is
not — position 3 says *the slot at 3* and there is no slot at 3, so appending puts a pass in the
frame nobody asked for and charges the frame for it (ADR-0340 §5, and
[P-0091](../principles/0091-cost-is-known-before-it-is-paid.md)). Writing the chain back unchanged
was refused too: a record describing no change is a line a replay obeys and a reader cannot
explain.

## Consequences

- **`karakuri-operation` loses `Feedback`'s two neighbours.** `Bloom` and `RgbShift` had no reader
  once the variants went; `Feedback` stays as the Master bay's reading of its first row and says at
  its definition that no operation carries it.
- **`karakuri_operation_record::Chain` is the slot list and nothing else.** `feedback`, `bloom`,
  `rgb_shift` and `shipped` were there to resolve a pass to a slot, which is now the surface's
  work: `karakuri_environment::mix::shipped_rows` answers where each shipped pass sits, and the
  Master bay's three rows are laid out from it.
- **`Owed` has a fifth answer**, and it is the first that is about an *address* rather than about a
  reading not taken. Every surface prints it through `Owed::why` already.
- **The console's three effect rows address the slots the shipped procedures hold**, and a row
  whose pass the chain has not got is drawn dim and takes no press: a chain operation names a
  position and that row has none. **On a default run the chain is empty, so all three rows are
  inert** — the panel has no control that adds a slot until the bay draws the chain's own rows and
  `+ add`. That is why the three rows' panel badge is `plan` and why *Set a chain effect's
  parameter* is the one entry in `panel_column.rs`'s `UNREACHABLE`.
- **No principle moves.** This applies P-0090 — a surface offers, it never decides — and P-0092,
  the retained frame being part of what a replay reproduces.
