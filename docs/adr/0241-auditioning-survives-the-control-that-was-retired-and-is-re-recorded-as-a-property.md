---
id: 0241
title: Auditioning survives the control that was retired, and is re-recorded as a property
status: accepted
date: 2026-09-02
supersedes: []
superseded_by: []
principles: [0080, 0087, 0093]
tags: [docs, process, ui, engine]
---

# Auditioning survives the control that was retired, and is re-recorded as a property

> **Annotated 2026-09-02, later the same day, after the maintainer read it.** **Two corrections, and
> the record stands as history in both.**
>
> **The gaps below are one gap, not two.** *Consequences* names `karakuri-cli` first: one window, no
> cells, no key since `v` went, so an operator choosing a candidate there is choosing blind. That is
> not a gap. The command line is test tooling and the instrument's principles do not bind it —
> [ADR-0242](0242-the-command-line-is-test-tooling-and-the-instruments-principles-do-not-bind-it.md)
> — and it has been removed from
> [P-0080](../principles/0080-an-operator-can-see-a-slots-own-material-without-putting-it-on-air.md).
> The second gap is real and is now the only one: the console's four cells are gated on
> `Residency::Live` in `Engine::aim`, so the off-air candidate is the one nothing draws. The
> machinery kept unreachable below is kept for exactly that.
>
> **And *four cells meet it better than one switched output did* concedes too much.** One switched
> output did not meet the requirement worse; it did not meet it at all, because material on the main
> output is the broadcast of the final result rather than a look at a candidate. The reasoning below
> — that P-0070 named a shape and the property survived it — is right, and this is the sentence
> inside it that reads as though the retired control had been a poorer instance of the same rule.
> [ADR-0240](0240-the-output-shows-the-mix-and-residency-keys-belong-to-the-mixer.md) carries the
> matching annotation.

## Context

[ADR-0240](0240-the-output-shows-the-mix-and-residency-keys-belong-to-the-mixer.md) retired the
operation *Choose what the output shows*. The `v` key, `Operation::SetPreview`, `Record::Preview` and
the console's press are gone, and the Program bay's four cells each present their own deck slot
continuously instead.

That record lists P-0070, *Auditioning is a prerequisite, not a convenience*, among the principles it
rests on. It was resting on a file the same decision had just made false. P-0070 said:

> An operator can put any one slot on the output in place of the mix, **whatever that slot's
> residency**.

and named its *Where it holds* as `Deck::set_preview` and the `v` key in `karakuri-cli`. After
ADR-0240 neither is reachable, so the rule as written was a claim about a control that no longer
exists — and it was cited, the same day, as support for removing that control.

The requirement had not moved. Only the shape it was written in had.

## Decision

**Retire P-0070 and re-record the rule as
[P-0080](../principles/0080-an-operator-can-see-a-slots-own-material-without-putting-it-on-air.md),
*An operator can see a slot's own material without putting it on air*.** Delete the file, do not
reuse the number, record the retirement in [INDEX.md](INDEX.md), and re-point what cited it —
`docs/contributing.md` §4 and
[ADR-0059](0059-a-records-pointer-into-the-principles-registry-is-metadata.md).

**What the old rule got right, and what P-0080 keeps.** Three clauses, all of them the requirement
rather than the mechanism: the operator judges the slot's *own moving material* through the transfer
curve the audience gets, at *any residency*, and *without stepping it*
([P-0082](../principles/0082-looking-never-writes-back.md)). The cost is also kept
where P-0070 put it: an audition adds an unbudgeted draw inside the watchdog's judging window
([ADR-0072](0072-auditioning-adds-a-draw-and-never-a-step.md)), that bill is paid rather than argued,
and [P-0079](../principles/0079-nothing-takes-the-show-down-and-nothing-takes-it-away-from-the-operator.md)
loses to it on purpose.

**What it named that was a mechanism rather than a property.** *One output, switched away from the
mix.* How many outputs the instrument has, and whether the look costs the operator the mix while it
is up, are answers to the requirement — not the requirement. Four cells beside the picture meet it
better than one switched output did, and the rule written as a shape could not say so; it could only
go false.
[P-0087](../principles/0087-name-the-property-never-the-shape.md) is that rule, and this is an instance
of it with the failure mode the other way round from the one P-0087 was written against: there,
deleting a named shape let the duplication relocate; here, deleting a named shape left a live
requirement with nothing standing for it.

**Deleting nothing and letting P-0070 stand was rejected.** It is what ADR-0240 left behind, and it
is the exact case
[P-0093](../principles/0093-a-statement-is-held-true-by-the-thing-it-describes-or-it-is-deleted.md) calls
out: *leaving a wrong description is ruled out too*. `docs/principles/` is the present tense and is
kept current by deletion and renumbering — a registry that keeps a rule naming a deleted method is a
registry nobody can cite.

**Editing P-0070 in place was rejected**, for the reason `docs/contributing.md` §4 gives: a rule that
stops being true is deleted and re-recorded, never edited into something else, so that a reader who
followed a pointer to P-0070 finds out it was retired rather than finding a different rule under the
name they were sent to.

## Consequences

- `docs/principles/0070-auditioning-is-a-prerequisite-not-a-convenience.md` is deleted. P-0070 is
  never reused; INDEX.md's *Retired numbers* table carries the row.
- P-0080 exists and has this record, so the P-0070 bullet under INDEX.md's *Standing rules with no
  record yet* is gone rather than moved: the reasoning is no longer only in the code.
- **The gap is named in the principle rather than left for somebody to meet.** Two surfaces fail
  P-0080 today.

  `karakuri-cli` cannot audition any slot at all: one window, no cells, and no key since `v` went.
  `f` and `g` take a slot out of the mix and put it back, which answers what a slot *contributes* and
  not what it *is*. An operator choosing a candidate there is choosing blind.

  The console's four cells are gated on `Residency::Live` in `Engine::aim`
  (`crates/karakuri/src/main.rs`), and the engine's draw of an Allocated or Priming slot into its own
  target sits behind an `auditioned` branch nothing can now switch on. So *whatever its residency* —
  the clause P-0070 argued hardest for — is met by no surface in this workspace. The machinery is
  present and unreachable rather than missing.

  Both are written into P-0080's *Where it is not met* under
  [P-0093](../principles/0093-a-statement-is-held-true-by-the-thing-it-describes-or-it-is-deleted.md).
- `Deck::set_preview` and `Deck::preview` are deleted, together with the five tests in
  `crates/karakuri-engine/tests/deck.rs` whose only subject they were. The `preview` field, the
  `auditioned` branches in `Frame::render` and the `Input::unity` arm that folds an audition are left
  standing and are now unreachable — **deliberately, and named here so it is a decision rather than
  an oversight**: that is the machinery the second gap above would be closed with, and throwing it
  away is a separate decision from retiring the operator-facing control. Nothing in the workspace can
  set `preview` to `Some`, so those branches are dead until something does.
- `crates/karakuri-engine/tests/preview.rs` is untouched. It is about fitting the canvas into a
  window that is not its shape and never called `Deck::set_preview`; the name is the other sense of
  the word.
