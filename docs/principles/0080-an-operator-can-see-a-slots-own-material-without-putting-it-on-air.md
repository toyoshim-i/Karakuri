# An operator can see a slot's own material without putting it on air

Choosing between candidates is the basic workflow of this instrument, and it cannot be done blind.
So the instrument owes the operator a look at what a slot is making **before** that slot reaches the
audience. This is a prerequisite rather than a convenience: without it the only way to find out what
a slot holds is to show it to the room.

Three clauses, and each is what the rule is *for* rather than a limit on how it is met.

**Its own material.** What the operator sees is that slot's texels, moving, through the transfer
curve the audience will see them through. Not the mix it would join, and not a still, a thumbnail or
a scaled-down copy — what is being judged is the level and the colour the material arrives at, and
none of those three carries it.

**Whatever its residency.** An off-air slot is exactly the one worth looking at: an Allocated one
shows the still it stopped at, a Priming one shows what it is warming into. Offering only the slots
that are easy to offer — the ones already drawing, because they are already drawing — removes the
rule from the one moment it is needed.

**Without changing it.** Looking adds a draw and never a step. A slot that is watched and then put
on air resumes where it stopped, or the audition would have moved the thing it was opened to judge —
[P-0041](0041-observing-must-not-advance-what-is-observed.md).

## This is the property; the mechanism is separate and has already changed once

The rule this replaces named a mechanism: *an operator can put any one slot on the output in place of
the mix*, with `Deck::set_preview` and the CLI's `v` key as where it held. That mechanism was retired
in [ADR-0240](../adr/0240-the-output-shows-the-mix-and-residency-keys-belong-to-the-mixer.md) — the
Program picture is the master mix and nothing swaps it — and the rule, stated as a shape, went false
with it while the requirement behind it had not moved at all.
[P-0060](0060-name-the-property-not-the-shape.md) is the rule that says why, and this file is an
instance of it: the property is that the operator can see a slot on its own, and *how many outputs
that takes* was never the thing being required. Re-recorded in
[ADR-0241](../adr/0241-auditioning-survives-the-control-that-was-retired-and-is-re-recorded-as-a-property.md).

## What it costs, and that the bill is paid rather than argued

An audition adds a draw that is not in the budget and falls inside the window `swap.rs`'s watchdog
judges candidates on, so watching a heavy slot can roll back an unrelated slot's build
([ADR-0072](../adr/0072-auditioning-adds-a-draw-and-never-a-step.md)). That is unbudgeted risk on the
live path, added knowingly. It is the case
[P-0079](0079-nothing-takes-the-show-down-and-nothing-takes-it-away-from-the-operator.md) loses, and
it loses for the reason written there: a rule protecting a performance may not be used to remove what
the performance is played with. The answer to the cost is to pay it and write it down
([P-0058](0058-before-v1-compatibility-is-a-bill-not-an-argument.md)), not to remove the look or hide
it behind a flag.

## Where it holds

The console's Program bay draws four deck preview cells beside or below the picture, one per deck
slot, each presented from `Deck::slot_view` for the slot it is lettered for — so no cell can show
another deck's material under the wrong letter, and no operator has to give up the mix to use one
([ADR-0239](../adr/0239-the-program-bay-preserves-preview-size-when-arranging-beside.md),
`crates/karakuri/src/main.rs`). The engine draws a slot into its own target and never resamples the
composite for it, which is what makes the first clause true rather than approximately true.

## Where it is not met

**Two surfaces fail this rule today, and both are stated here because a rule in force says where it
is not yet true** ([P-0036](0036-an-invariant-that-is-not-yet-true-says-so.md)).

**`karakuri-cli` cannot audition a slot at all.** It has one window, no cells, and — since ADR-0240
retired the operation and the `v` key with it — no way to see any single slot on its own. What is
left is `f` and `g`, which take a slot *out of* the mix and put it back; that answers "what is this
one contributing" and does not answer "what is this one making". An operator choosing a candidate on
this program is choosing blind. It is a gap rather than a decision: nothing about a one-window
program makes the requirement not apply to it.

**The console's cells show only Live decks.** The draw is gated on `Residency::Live` in
`Engine::aim`, and the engine draws an Allocated or Priming slot into its target only under an
audition that nothing can now switch on. So the second clause above — *whatever its residency* — is
met by no surface in this workspace: the slot an operator most needs to look at, the one warming or
parked and not yet on air, is the one nothing draws. The machinery is still there and unreachable
rather than absent, which is what makes this a gap with a known shape rather than a design question.
