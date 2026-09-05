---
id: 0251
title: The signal bus is not readable from IR, and `beats` is the one exception
status: accepted
date: 2026-09-05
supersedes: []
superseded_by: []
principles: [0086]
tags: [ir, signal, format]
---

# The signal bus is not readable from IR, and `beats` is the one exception

## Context

P-0065 — *The signal bus is not readable from IR* — is retired into
[P-0086](../principles/0086-a-procedure-knows-only-what-it-declares.md), *A procedure knows only what
it declares*, whose body is the general rule: what reaches a procedure arrives through a declaration.

**P-0065 named no ADR, and said so**: *"It predates the recovered history — it is in the first
committed specification — so there is no ADR behind it, only this."* Two things it carried are
narrower than the general rule and have nowhere else to be addressed once the file is gone: **why
`beats` is an exception**, and **why the refusal names the fix rather than the mistake**.
`docs/ir-spec.md` states both as present-tense language rules; what it does not hold is the
alternative that lost, which is the half a reader needs when the question is re-proposed — and it is
re-proposed constantly, because a model writing a `.kir` reaches for `energy` by default.

## Decision

**`energy`, `beat`, `band`, `bpm` and the rest cannot be named inside a `.kir`.** To react to audio
or tempo, a procedure declares a `param` and the Set attaches a signal to it with a `bind` record.

**The property being bought is that a procedure's inputs are enumerable.** A direct read couples a
procedure to a live value in a place nothing outside the shader can see: the UI cannot show the
coupling, an agent cannot adjust it, and the record cannot carry it, so a session that replays does
not replay it. Going through `param` plus `bind` puts every external coupling in one declarative
form that all three read. It also gives the coupling somewhere to keep what a direct read has no
room for — a `curve`, a `range`, and the confidence blend a bound value arrives with
([ADR-0047](0047-a-binding-blends-on-confidence.md)).

**`beats` is the exception, and the reason is that it is not an input.** What follows a tempo is not
a parameter value but the passage of time: a binding writes one number, where a clock has to move
everything the procedure does. It is an ambient for the reason `t` is one — a procedure is
*evaluated at* it rather than *given* it — and `beat` and `bar` stay bus names because they are a
different thing, phases in `[0, 1]` for driving a parameter through a curve and a range, where
`beats` is unbounded and monotone for driving a position. `docs/ir-spec.md` carries the three
properties that make it usable: the same instant as `t`, advanced per substep so a frame rate drop
does not move the beat, and continuous across a tempo correction so following the room does not jump
every time the tracker trims.

## Alternatives rejected

**An ambient per signal — make the bus readable the way `beats` is.** The shortest spelling, and it
is what a generator writes unprompted. It loses the enumerability outright: the coupling exists only
inside the shader, so the panel has no control to draw, an agent has no value to move, the record
has nothing to write, and a curve, a range and a confidence have nowhere to live. `beats` survives
this argument only because it is a clock rather than a value — the exception is drawn at *is it an
input*, not at *is it convenient*.

**Let the engine infer a binding from a bus name read in the source.** It would keep the short
spelling and produce a declaration, and it fails on the same test from the other side: the range and
the curve are exactly what cannot be inferred, so the engine would be inventing the two numbers that
decide what the coupling does. It is also a correction the compiler applies to source somebody
wrote, which a rejection is supposed to hand back.

**Refuse it by naming the mistake.** *`energy` is undefined* is true and useless. The refusal is
written as the fix — *the signal bus is not readable from IR — declare `param energy` and attach a
`bind` record to it in the Set file* — which is
[P-0083](../principles/0083-a-refusal-carries-what-the-next-attempt-needs.md) applied to the one
diagnostic a generated procedure hits most often.

## Consequences

- **The diagnosis is a guess, and the hint is written knowing that.**
  `check.rs`'s `unresolved` arm reaches this sentence only when a name resolves to no local, param,
  attribute or ambient. The bus accepts any name, so the checker cannot tell a signal read from a
  typo; the hint is chosen because it does not actively mislead the second case while it repairs the
  first.
- **The specification tells a generator's author to state the rule in the prompt**, because models
  violate it readily. That instruction is a workaround for a rule the language cannot make
  unwritable, and it stays until something else makes the bus unreachable by construction.
- **The live element count is refused on the same ground and is not called out as an exception.**
  `capacity` is a compile-time constant a Set declares; the live count is engine state, and a
  procedure that branched on it would draw different geometry at different fill levels for no reason
  anybody could express. It is the same rule with a different subject, which is why it needed no
  clause of its own.
- **Nothing in the code changed for this record.** It describes what `karakuri-ir` already refuses
  and what `docs/ir-spec.md` already specifies, written down because the file that held it is being
  deleted.
