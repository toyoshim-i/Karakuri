---
id: 0185
title: A fader translates a drag into an operation and applies nothing
status: accepted
date: 2026-08-26
supersedes: []
superseded_by: []
principles: []
tags: [ui]
---

# A fader translates a drag into an operation and applies nothing

## Context

The Mixer drew the deck's real gain, opacity, residency, meter, blend and mask
([ADR-0178](0178-the-mixer-draws-four-tracks-and-as-many-strips-as-the-deck-has.md)) and nothing in
it could be touched. The frame this is built to is the project owner's: **MCP is the most direct
form of operation**, and **a mouse cannot operate Karakuri at all — it operates a GUI component,
and the GUI component is a translator.** Everything except MCP needs one.

So a fader's whole job is to turn a drag into an `Operation` and hand it over. It cannot apply it:
`karakuri-console/src/` has no engine (ADR-0156). **This is the first customer
`karakuri-operation` has ever had.**

## Decision

### One thing in hand, not two

`Panel` keeps one `Option<Drag>` and `Drag` is `Boundary | Fader`. Two `Option`s side by side would
be four states with two impossible, and `input`'s **rule 1** — *a drag in hand keeps its claim,
wherever the pointer has wandered to* — would have had to remember to ask about both. One thing in
hand means rule 1 covers a fader **by construction**, and that is what a test asserts rather than
what a comment promises.

**`drag_axis()` became `in_hand()` with three answers, and reading `View::cursor` is what forced
it.** A fader's honest answer to *which resize axis* is `None`, and a `None` meaning *not a
boundary* cannot be told from a `None` meaning *nothing in hand* — which is exactly the case the
cursor needs, because falling through puts a resize cursor on a fader held against its top the
moment the pointer wanders across a boundary. One question, three answers. No new cursor: the
console's vocabulary is arrow-or-resize, and a fader drag stays the arrow, asserted positively.

### A press on the knob grabs; a press on the track does nothing

A fader at 0.3 whose top is clicked must not jump to 1.0 — that is a mix change nobody asked for,
on stage. The knob keeps its grab offset exactly as a boundary does, and both ends are exactly
reachable, which is `karakuri-midi`'s `GAIN_RANGE` argument — *chosen so that both ends of a fader
are exact* — applying here for the same reason.

**`claim` now takes the strips.** A knob sits on the fill's moving edge, so *where the control is*
depends on what the deck said this frame. That is a further widening of what *who gets this event*
depends on, beyond the text shaper
[ADR-0176](0176-a-control-the-console-draws-is-the-panels.md) already admitted, and it is stated
rather than discovered later.

### `Change::Emitted(Option<&Operation>)` — `Some` is `Now`, `None` is `Never`

Argued from its neighbours rather than copied. **It is not `Change::Pointer(Claim::Panel)`**, which
is unconditionally `Now`: a fader held against the top of its track is a claimed pointer event sixty
times a second with nothing on screen changing.

And the pointer arm's own doc forbids deciding from `Panel::moved`'s `Option` — because **a
boundary's `None` is a half-pixel threshold on a position**, and half a logical pixel is a whole
physical one. **A fader's `None` is an exact comparison of the value that would be sent**: there is
no distance below which the panel would look the same, because the same value is the same picture.
So this is `Change::Rearranged`'s shape — *what the model returned* — and not the pointer's. A
press and release with no motion emits nothing at all, which matters because `set_gain` cancels
whatever was moving the control.

### The harness applies it, and says the shortcut is its own

P-0028 is *every control ends in the same record*, and that is what makes a console fader the same
thing as a key press and a MIDI knob. **Where `Operation` becomes `Record` is not decided** — it
needs `karakuri-operation` and `karakuri-store`, neither of which depends on the other, and it is
the centre of the record design rather than a side effect of a fader.

So the example builds the record itself, and its doc says at length that this is a shortcut, whose
it is, that `karakuri-cli`'s `mix::gain_record` is the existing half and is unreachable because that
crate has no library target, and that **the day the conversion lands this function is deleted rather
than moved**, taking `karakuri-store` out of the dev-dependencies with it. The loop closing is
printed: operation → record → what the deck now says.

**The strip draws `Deck::gain`, not what the drag sent**, and that is asserted mid-gesture. A fader
that moved a number the console kept would be a second copy of the deck's state.

## What contact with the vocabulary found

It survived, and the two variants needed no change. Four things were awkward and all four are worth
having:

- **`Operation` is not `Copy`**, because other variants carry `String` and `Vec`. So `Dragged` lost
  `Copy` and every caller clones or matches by reference. **A leaf vocabulary in one enum makes its
  cheapest variants pay for its heaviest.**
- **`deck: u8` against the console's `usize` slot.** ADR-0180 already records that the manual's
  *deck* is the code's *slot*; this is the first place it costs a cast per gesture.
- **There is no `SetMask`.** The mask mini has no destination to name — `WipeKind` exists only
  inside a transition setting, and `Wipe { from, to }` writes a mask as a side effect. **It is a
  readout of state no operation can set**, and that row has to be decided on the manual's page
  before it can be a control.
- **`SetGain` cannot say *above unity* from this control** — not the vocabulary's fault, since
  `gain: f32` is open, but the trim is drawn over `[0, 1]` (ADR-0178) so this customer reaches only
  the bottom half of what the operation expresses.

## Consequences

- **The blend mini needs an affordance, and this bullet stated the reason wrongly.** It said the
  mock and P-0074 disagree — the mock's tooltip says *click to cycle*, and a vocabulary has no
  cycles — and concluded that the mini must **name** one of three, by a menu or three targets, which
  made it a question about who owns the pointer. **The premise was never true** (corrected
  2026-08-26): P-0074 forbids a cycle *operation*, and its second paragraph names *"a mini that
  cycles the blend"* as one control emitting three, an affordance built over operations by whoever
  draws the control. `SetBlendMode` naming a destination is right and is the half this bullet got
  right; the conflict was read into it. What is actually open is what a MIDI map learns from that
  chip, and the answer to the affordance is
  [ADR-0187](0187-the-blend-mini-cycles-and-a-map-learns-the-three-it-cycles-through.md).
- **The tally is two operations for three states**, and the console draws the *effective* residency
  while the operations set the *requested* one. Unlike a fader, whose readback is exact, a tally
  press writes a request the governor may not honour — so it needs a decision about what the control
  shows while the two disagree.
- **Two tests caught the test rather than the code**, continuing ADR-0178's tally. The knob-clearance
  test passed with `GRAB` widened to 60, because `claim` says *the panel's* for a boundary **and**
  for a control and the grab was never consulted; it asks `Layout::hit(p, GRAB)` directly now. And
  the readback test passed with *the panel keeps the value* injected, because `released()` clears
  the drag before the comparison — the defect lives *during* the gesture, so the test draws a frame
  mid-drag.
- `unit()` moved from `view` to `panel`, because a drag's value goes through the same clamp the
  drawing does — one function, four call sites, rather than a second four-line copy. `DECK_LETTERS`
  became public for the same reason: a second `["A","B","C","D"]` in the example would be a
  transcription with no citation, which ADR-0179's guard exists to end.
- `claim` costs two galley lookups per strip per pointer event now, said in its doc rather than left
  to be measured. A console with no deck pays nothing.
