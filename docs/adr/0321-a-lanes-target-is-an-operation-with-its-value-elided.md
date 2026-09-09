---
id: 0321
title: A lane's target is an operation with its value elided
status: accepted
date: 2026-09-09
supersedes: []
superseded_by: []
principles: [0085, 0090]
tags: [console, sequencer, vocabulary, midi]
---

# A lane's target is an operation with its value elided

## Context

**This is the sharpest question in the sequencer bay, and both the mock and the vocabulary say so in
the same sentence.** `+ lane`'s tip and `Operation::PointLane`'s documentation carry it word for
word:

> *"a deck fader is a slot number, a Set parameter is a node address and a parameter within it, and
> nothing in this program spells both, so picking either would buy a control that reaches one lane
> in four."*

The mock draws four lanes: deck A, B and C's channel faders, and `L2:0 twist` on deck B. The first
three are `Operation::SetOpacity { deck, opacity }`, classified `Vocabulary::Session`; the fourth is
`Operation::WriteParam { deck, param, value }`, which reaches inside a deck's Set. **The test any
spelling has to pass is that it reaches all four**, and *reaches one lane in four* is the failure
this bay has already used twice to kill a proposal:
[ADR-0222](0222-a-sequencer-lane-is-a-fifth-route-and-not-a-binding.md) killed the binding reading
with *"there is no binding on a deck fader anywhere in the engine"*, and `Operation::SetLaneMute`'s
doc killed `TakeParamBack` for the same reason a fortnight later.

It is separated from
[ADR-0320](0320-a-pattern-is-one-bar-of-sixteen-slots-a-lane-is-a-target-and-two-levels-and-a-cell-is-a-bit.md)
on purpose. The data shape is a thing to agree to; this is the thing to say yes to, and folding it
into a struct definition would bury it. It is also the record a later `Trim`, `MaskPosition` or
`Out` arm is checked against, and the one M5.12's learn spelling will be read beside.

## Decision

**A lane's target is an arm of this vocabulary with its value field left out, and a lane emits
`target.operation(value)`.**

```rust
/// What a lane drives: an operation of this vocabulary with its value left out.
pub enum LaneTarget {
    /// Deck A/B/C's channel fader — three of the four lanes the console draws.
    Fader { deck: u8 },
    /// A parameter inside the Set on a deck — the fourth.
    Param { deck: u8, param: ParamAt },
}
```

`Fader { deck }` → `Operation::SetOpacity { deck, opacity: value }`.
`Param { deck, param }` → `Operation::WriteParam { deck, param, value: ParamValue::Scalar(value) }`.

It lives in `karakuri-operation` beside `NodeAt` and `ParamAt`, because two of the five payloads name
it and **a payload may name nothing a surface cannot construct**
([ADR-0192](0192-an-operation-asks-for-what-a-surface-can-say-and-the-record-stays-whole.md)). It
costs that crate no dependency: both arms are made of types it already owns.

**Why this is the one that works.**

- **It covers all four lanes the mock draws**, which is the test the alternatives fail.
- **It is what the mock already says a target may be**, written as a type instead of as prose:
  *"anything in the vocabulary a lane can emit, which is why three deck faders and a Set parameter
  sit side by side below and all four are lanes."*
- **[ADR-0268](0268-a-vector-parameter-is-driven-one-component-at-a-time.md) costs nothing.** A
  vector parameter is driven one component at a time and `ParamAt` already carries that — `key` is
  `glow.x` and never `glow` — so a lane on a vector component needs no `component` field. The
  addressing problem was solved where the write lands, and a lane reaches it by the road `--param`
  reaches it by.
- **Two working precedents in the tree**, which is
  [P-0085](../principles/0085-take-the-mechanism-that-exists-and-pay-the-bill-now.md) taken at the
  width of a type: `karakuri_console::panel::Knob::operation(value)` is *"the operation this knob
  asks for at `value`, and the whole of the translation a fader performs"*, and
  `karakuri_midi::map::Target::operation(message)` is a target plus a MIDI message becoming an
  `Operation`. A third is not a new mechanism.
- **A closed enum with an exhaustive `match`** means a target added does not compile until it says
  what it emits, which is `Record::vocabulary`'s discipline and the reason `written` is exhaustive.

**What the first pass leaves out, and what that costs.** `Trim { deck }`, `MaskPosition { deck }`,
`Out` and `Exposure` are each one arm and one line of `operation()`. They are additions to a closed
list, so none of them is a decision and leaving them out is scope rather than a gap.

**`Residency` and `Blend` are not lane targets, and that is a decision rather than an omission.**
Each takes a word from a closed list rather than a level, and a step is a level, so a lane pointed at
one would have to invent the word an on-step means. It is said here so a reader does not have to
notice it from the enum's silence.

## Alternatives rejected

**A published-interface position — M5.12's learn spelling, a deck and a position.** It covers the
fourth lane and **not the first three**. `Published` is a control a *Set* declares
(`docs/ir-spec.md`, *What a Set publishes*), and `Record::Opacity` is `Vocabulary::Session` and no
Set's. So it reaches one lane in four, which is the failure ADR-0222 used to kill the binding reading
and `SetLaneMute`'s doc used to kill `TakeParamBack`. **Refused, and for the third time in the same
bay.**

**A bind-like target — `deck`, `node`, `key`, `component`.** It covers the fourth lane natively and
the first three only by inventing a pseudo-key: `deck 0, node None, key "opacity"`. That is a second
addressing scheme for a control the vocabulary already addresses as `SetOpacity { deck }`, and it is
the four-fields-in-one-`&str` shape `karakuri-signal` already refused for `noise` and that ADR-0222
quotes when it refuses a bus name. It also puts `Record::Opacity` behind a name no other route uses,
so a MIDI map, a key and a lane would spell one control three ways.

**Reuse `karakuri_midi::map::Target`.** The nearest existing thing, and it is *already* this
decision's shape: its arms are `Gain { slot, range }`, `Opacity { slot, range }`,
`Exposure { range }`, `MaskPosition { slot, range }`, `Residency { slot, residency }`,
`Blend { slot, blend }` and `Tap`, and `Target::operation` turns one plus a message into an
`Operation`. **It is the right shape in the wrong home.** It is a private enum in a device crate; it
is closed to what a *map line* can spell, which is why it has no `param` arm at all and why *Write a
parameter* is M5.12's one `plan` MIDI badge; and its `range` is a map's own affair rather than a
lane's, because a lane's two levels are ADR-0320's `on` and `off`. **Lift the shape, not the type.**

**A `String` naming the control.** Not proposed and refused pre-emptively, because it is what a
reader reaches for when the enum grows: a name is the four-fields-in-one-`&str` failure again, and
`karakuri-operation` owns enumerations *"rather than passing strings"*
([P-0090](../principles/0090-a-surface-offers-it-never-decides.md)).

## Consequences

**The payload it makes constructible is the one thing here nothing presses yet**, and that is the
scope rather than a gap: `PointLane`'s row stays `plan` because the bay draws no target chooser.

**What was built the same day** (2026-09-09):

- **`karakuri-operation` carries `LaneTarget`** with its two arms, `LaneTarget::operation(value)` as
  an exhaustive `match`, and `LaneTarget::deck()` for the lookup ADR-0323 needs. `StepMode` sits
  beside it (ADR-0320). **No dependency is added**, which is what the arms being made of `u8` and
  `ParamAt` buys.
- **`PointLane { pattern: u8, target: LaneTarget }`** is the payload. Nothing constructs it on a
  surface yet; `karakuri_pattern::Pattern::push` is the act it names, and `crates/karakuri`'s
  `demonstration_banks` is what calls that until a chooser is drawn.
- **The fader arm is what the first slice runs on**:
  `Fader { deck: 0 }` → `Operation::SetOpacity`, polled per frame, and
  `a_parameter_lane_emits_a_parameter_write` in `karakuri-pattern` is what holds the other arm
  honest before a surface can point at it.
- **`docs/manual/console.html`'s `+ lane` tip lost *"What a target is spelled as is not
  answered"*** and gained the two arms and why they reach all four lanes. It **keeps its
  arena-insert gap**, which this record does not touch: the panel still cannot grow a bay's body
  while it is running, and that is one gap drawn in five places.
- **`docs/manual/operations.html`'s *Point a lane at what it drives* stops saying nothing spells
  both**, and its panel badge stays `plan`.
- **The fourth lane's tip stops calling this the bay's hardest question.**

**What it does not decide.** M5.12's learn spelling, which is a different question with a different
constraint — a map line names a slot, a range or a word from a closed list — and is read beside this
record rather than settled by it. Removing a lane, which has no control, no row and no operation, and
is owed a purposeful note rather than an invention.
