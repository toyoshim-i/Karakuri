---
id: 0248
title: A wipe's front shape is the transition row's, and travels with the quantum and the length
status: accepted
date: 2026-09-04
supersedes: []
superseded_by: []
principles: []
tags: [operations, format, ui]
---

# A wipe's front shape is the transition row's, and travels with the quantum and the length

## Context

`Operation::Wipe { from, to }` is one gesture and six records: a mask at position 0 on the incoming
deck, that deck silenced and put on air under `over`, and one scheduled move carrying the front to 1.
Two of the fields those records need are not the wipe's to choose, and **that, rather than any
difficulty, is why `c` was the last key in `karakuri-cli` still building its own records by hand.**

- **The front's shape.** Two operations carry a shape and they mean different things.
  `TransitionSetting::WipeShape` — the third setting of `Operation::SetTransition`, what `z` presses
  — is *the shape the next wipe takes*, a console setting that names no deck.
  `Operation::SetMaskShape` is *the shape a deck's mask has now*, and it names a deck. Nothing said
  which one a wipe reads.
- **The soft edge.** `Record::Mask`'s `softness` is named by no operation anywhere. It has one
  constant behind it in `karakuri-cli`, which says of itself that it is *"not a key"*, and no control
  on any surface.

`karakuri-cli`'s own `OWED_RECORD_PATHS` carried the sentence: *"`Wipe` — the shape its front takes
is `SetTransition`'s and the soft edge is no operation's, so nothing has said whose they are to
write."* It is the last of six operations that were held out of `Live::operate`; the other five have
each been paid. **Three of them were paid by exactly the move this record makes**: a fade, a
crossfade and a renderer selection each needed the instant, the length and the curve, and the answer
was that those are the *surface's own state*, handed to the conversion inside `Current::transition`
rather than invented by the gesture.

**The two readings had collided in the tree.** `karakuri-operation-record`'s `Transition` type
carried a paragraph refusing a shape on the other side's terms — *"a shape is a `Record::Mask`'s and
reaches a record through `Operation::SetMaskShape`, which converts already. A field for it would be a
value no arm reads"* — while the vocabulary went on defining `TransitionSetting::WipeShape` as the
shape the next wipe takes. Both could not be right, and neither had been chosen.

## Decision

**A wipe reads its front shape from the transition row**, handed over inside `Current::transition`,
in the same field and by the same road as the instant and the length. `TransitionSetting::WipeShape`
is what sets it and `z` is what presses it.

**The soft edge is read off the mask already running on the deck being wiped in**, which is what the
conversion does for `Operation::SetMaskShape` today. No operation is added and no softness is
invented: `Record::Mask` is written whole, and the field it cannot be told comes from the mask it is
replacing.

**The reason is the symmetry, and it is not an aesthetic one.** The quantum and the length are the
first two settings of `Operation::SetTransition`; the shape is the third. The first two are already
the surface's own state, arriving as a reading, for a reason written at `Current::transition` —
there is nothing to read them back from, so they are handed in. Nothing about the third setting is
different, and giving one setting of one operation a road of its own is a seam the next reader has to
learn.

## Alternatives rejected

**The shape is the deck's mask, and a wipe reads whatever `SetMaskShape` last put there.** This is
the position the tree already held, in `Transition`'s own doc, and it is the serious alternative
rather than a straw one: a shape *is* a `Record::Mask`'s field, `SetMaskShape` already converts, and
the mask is per-deck where `SetTransition` names no deck. It loses on two counts. It makes a wipe a
two-step gesture — set the incoming deck's mask, then press `c` — where the manual has it as one
press. And it leaves `TransitionSetting::WipeShape` with nothing to do, so taking it would be
retiring half of the row *Choose the wipe shape, the quantum, the length*, which is a change to
`docs/manual/operations.html` and the nine places §5 lists. Neither cost buys anything the decision
above does not already give.

**Give the soft edge an operation, and let the transition row say the whole look of the next wipe.**
Attractive, and the one thing that would make a softness reachable from any surface at all — today it
cannot be put on a MIDI knob, which for a parameter this expressive is a real gap. It is rejected
*here* rather than refused: it changes the row's heading on the manual's page, which is a change to
the specification and belongs to whoever is drawing that row. Nothing about this record forecloses
it, and the wipe reads a softness the same way either way.

**Leave it owed until the transition row is drawn.** The row is M5.2's and is drawn nowhere at all,
so the question would have been answered by whoever drew it, under deadline, with the three other
rows of that bay waiting on the same settings. The blocker was a decision and not a drawing, and
answering it now is what lets the drawing be judged on the drawing.

## Consequences

- **`Current` gains a sixth reading, `Mix { blend, residency }`**, with `Reading::Mix`. One reading
  and not two: both come off the same deck in the same lookup, so a caller cannot hold one without
  the other, and two would force the arm to pick an order between two `NotRead` answers for one
  question. It follows `Look`, `Transport` and `Mask` — a reading is one subject rather than one
  field. It is also the first reading taken so that a record can be left **out** rather than
  completed.
- **`Operation::Wipe` converts, and `Owed::NotSettled` is down to two** — the tap and the octave
  shift, both of which need the beat tracker. `karakuri-cli`'s `OWED_RECORD_PATHS` is empty of
  wipes; `Live::wipe` is one `operate` call, its two refusals and its print, and
  `mix::transition_record` is deleted, which its own module doc had promised would happen the day
  this was settled.
- **The wipe writes four, five or six records rather than always six**, so `Written::Records` says
  it is never empty and not always the same length. `Record::Blend` is written only when the
  arriving deck is at `Blend::Add`, and `Record::Residency` only when it is not already Live.
- **The old surface's doc contradicted its own code, and the code was right.** It said *"the mode is
  left wherever the operator had it, and `over` is only forced when the slot was still at the
  default"* — and `Blend::Add` **is** `#[default]`. So `m` in front of `c` has never meant anything
  at `add`; it means something at `over` and at `max`, which the restored conditions preserve
  exactly. The first implementation of this record dropped both conditions on the grounds that the
  conversion has no deck to ask, which is how the discrepancy was found.
- **The shape is `Current::transition`'s and the soft edge is `Current::mask`'s.** The second
  `Record::Mask` restates the shape rather than reading it back, because the first has not been
  applied when the second is built.
- **`Transition`'s paragraph refusing a shape is rewritten rather than deleted.** It quotes its own
  last sentence, says the argument was an argument and not an oversight, and keeps the half that
  survives: a deck's *current* mask shape is still `SetMaskShape`'s, and what `Transition` carries
  is the shape the next wipe will make it.

## What this does not settle

**Whether a deliberately chosen `add` should survive a wipe.** The restored condition cannot tell
*the operator chose `add`* from *nobody has touched this slot*, because both read `Blend::Add`. No
condition on the blend's value alone gives all three of preserving a chosen `add`, preserving `max`,
and still making a fresh deck wipe *over*; separating them needs surface state that does not exist —
whether this slot's blend has ever been set. Named here rather than decided, and the restored
behaviour is the one that keeps the most of what an operator can reach today.
