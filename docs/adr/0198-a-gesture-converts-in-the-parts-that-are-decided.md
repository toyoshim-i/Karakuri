---
id: 0198
title: A gesture converts in the parts that are decided, and a key that writes no record performs it itself
status: accepted
date: 2026-08-27
supersedes: []
superseded_by: [0205]
principles: [0028, 0074]
tags: [cli, keys, vocabulary, surfaces]
---

# A gesture converts in the parts that are decided, and a key that writes no record performs it itself

## Context

[ADR-0196](0196-a-map-line-names-a-state-and-an-old-line-is-refused.md) moved `karakuri-midi`
onto the vocabulary and left two surfaces named as not having moved: `karakuri-console`'s
`panel::Op` and **`karakuri-cli`'s key handler**. This is the second of those, and the survey it
starts from contradicts the sentence that named it.

**Nine operations already reach `Live::operate` from a key**, and have since
[ADR-0194](0194-where-an-operation-becomes-a-record-is-a-crate-that-depends-on-both.md) landed:
`SetGain`, `SetOpacity`, `SetBlendMode`, `SetResidency`, `SetPreview`, `SetTonemap`,
`SetExposure` and `ScrubDeck` are what `space`, `w`, `[`, `]`, `\`, `;`, `'`, `m`, `v`, `t`, `-`,
`=`, the backquote, `u` and `i` write. That commit's own message says *"no key arm moved"*, and
what it meant was the arms of the `match` in `Live::key`; the *handlers* those arms call had all
moved. ADR-0196 repeated the sentence and it read as *nothing has moved*.

So the question this record answers is not *when does the key handler move* but **what is left,
and why the rest cannot go the way MIDI went**. The survey, by key rather than by operation —
thirty-nine keys:

| group | keys | count |
|---|---|---|
| its record converts, and it goes through `Live::operate` | `space` `w` `[` `]` `\` `;` `'` `m` `v` `t` `-` `=` `` ` `` `u` `i` | 15 |
| its record is owed (`Owed::NotSettled`) | `f` `g` `x` `c` `r` `y` `b` `,` `.` | 9 |
| it writes no record (`Silent`) | `0`–`3` `z` `n` `j` `a` `k` `o` `p` `esc` | 12 |
| the vocabulary names nothing | `s` `h` `?` | 3 |

## Decision

### 1. A key whose operation writes no record cannot route through `operate`, and that is `Silent`'s shape rather than an omission

`Live::operate` turns an operation into the records it writes and applies those. An operation
`written` answers `Written::Silent` for writes none, so a key routed through it would **print
*no record* and do nothing**. `n` would stop moving the quantum, `k` would stop saving, `esc`
would stop quitting.

The MIDI router never meets this because every operation a map line can produce writes a record.
A keyboard does: twelve of its keys — the deck selection, the three transition settings, the
window snap, the save, the latency offset, quit — are the surface's own state, and **this
surface is the only thing that holds it**. `Operation::SelectDeck` says as much at its own
definition. The route ADR-0196 established is therefore the right one for a router and is not a
target for a key handler; what a key handler owes is that **the records it writes have one
derivation**, which is P-0028 and is a smaller claim than *every key is an operation handed to
`operate`*.

### 2. A gesture whose own record is owed converts in the parts that are decided

`crossfade` is one operation and four records; `wipe` is one operation and five. Both are
`Owed::NotSettled`, because a scheduled move needs the grid quantised onto a musical instant plus
the quantum and the length `Operation::SetTransition` holds and no record carries.

**That count is why the gestures cannot convert. It was never a reason their parts could not.**
Silencing the incoming deck is `Operation::SetOpacity`, forcing `over` is
`Operation::SetBlendMode`, putting it on air is `Operation::SetResidency` — three operations whose
conversions landed with ADR-0194 — and the CLI's own comment already called the silencing *"an
`opacity` record like any other"*. So each gesture now asks `Live::operate` for the parts that are
decided and builds only the parts that are not: the mask, which **no operation names at all**, and
the scheduled move, which is the owed one.

`mix::opacity_record`, `mix::blend_record` and `mix::residency_record` go with that, the way
`mix::gain_record` and `mix::preview_record` went when their operations landed. They were this
program's last three records built two ways.

### 3. What stays is named at the function and enforced by a table

`Live::key`'s documentation carries the four groups above. Four functions still write a record
directly — `cycle_sync`, `wipe`, `cycle_renderer`, `fade_slot` — and each says at its own
definition which operation it names and why that operation cannot convert. `OWED_RECORD_PATHS`
in the tests is the same list, read against the checked-in source: a method that grows a direct
write and is not on it fails by name, and a row whose function no longer writes one fails as a
leftover. A second test asserts that all seven operations those keys name are still
`Owed::NotSettled`, **against `karakuri-operation-record` rather than against this file** — so
the day one of those conversions is settled, the failure says which key is due to move. That is
`run_surface`'s `TapBeat` promise kept by a test instead of by memory.

### 4. `s`, `h` and `?` are reported and no row is written for them

The status line and the bindings text have no `<h3>` on `docs/manual/operations.html`, which is
the specification for which keys exist. They are not added here. A row invented from the
implementation is a specification written backwards, and the page is written ahead of the
interface on purpose.

## Alternatives rejected

- **Route every key through `Live::operate`, including the `Silent` ones, and let `operate`
  perform them.** This is the shape that would make the two surfaces literally identical. It
  costs `operate` a second job — it would stop being *the operation, as the records it writes*
  and become *the operation, done* — and it would need arms for the twelve surface-state
  operations, which is precisely the second list beside `written`'s exhaustive match that
  ADR-0196 removed from the router. The compile-time claim would be back where it is false.
- **Leave `crossfade` and `wipe` building their own `opacity`, `blend` and `residency` records,
  with a test keeping the two derivations in step.** That test existed and passed. It is the
  weaker answer: a test that two derivations agree is maintenance of a duplication, and the
  duplication's stated exit condition — *"these builders go as each operation lands"* — had
  already been met by three of them. Deleting the second derivation is what removes the
  possibility rather than watching it.
- **Route `Crossfade` and `Wipe` through `operate` and let them print `Owed`.** Honest about the
  gap and useless on stage: the key would stop crossfading. A change of route may not change what
  a key does.
- **Give `crossfade` a `Vec<Operation>` and route the list.** It reads well and hides the thing
  worth seeing: three of its steps are operations and one is not, and the owed one is owed for a
  reason a reader should meet at the line rather than inside a helper.
- **Add rows for `s` and `h`/`?` so every key has an operation.** Tempting because it makes a
  count come out even. `Operation::Quit` exists and the status line is arguably its neighbour —
  but *what a status line is* on a console with no terminal is a question for the page, and
  answering it from a CLI's key handler is the direction ADR-0180 exists to forbid.

## Consequences

- **Fifteen of the thirty-nine keys reach the deck through `Live::operate`**, and after this so do
  four steps of two gestures that do not. The records this program derives twice are down to zero.
- **Nine keys keep a path of their own**, all `Owed::NotSettled`, and a test will name them the day
  their conversions land. Nothing about what any key does changed.
- **Twelve keys are documented as unable to route**, which is a property of `Silent` and will hold
  for the console too: `panel::Op`'s folds, solos and pointer moves are the same group.
- **Two more findings for the operations page.** There is no `SetMask`, so `wipe`'s mask record is
  the one part of that gesture nothing in the vocabulary names — the same gap the console's mask
  mini already reports from the other side. And `s` and `h`/`?` have no row at all.
- **No new principle.** P-0028 and P-0074 are what this applies.
