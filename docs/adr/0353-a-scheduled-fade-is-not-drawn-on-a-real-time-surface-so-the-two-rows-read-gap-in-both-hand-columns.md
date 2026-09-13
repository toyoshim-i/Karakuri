---
id: 0353
title: A scheduled fade is not drawn on a real-time surface, so the two rows read gap in both hand columns
status: accepted
date: 2026-09-14
supersedes: []
superseded_by: []
principles: [0085, 0087, 0090, 0091]
tags: [operations, console, keyboard, transitions, manual, m5]
---

# A scheduled fade is not drawn on a real-time surface, so the two rows read `gap` in both hand columns

## Context

**Two rows of [the operations page](../manual/operations.html) were the whole of M5.13's remainder.**
*Fade a deck out or in* and *Crossfade to the next deck* each read `gap` in the panel column and
`plan` in the key column, the key badge naming `enter &middot; in the Mixer` —
`grep -c 'rt plan">key' docs/manual/operations.html` returned 2, against 7 on 2026-09-10 and 27 on
2026-09-09. Every other row of that column had been answered by the grammar reaching the bay that
draws the control the key hangs on
([ADR-0331](0331-the-key-column-is-spelled-two-ways-and-a-built-badge-keeps-its-bare-letter.md),
[ADR-0343](0343-the-grammar-reaches-all-nine-bays-and-space-on-a-bay-is-the-fold.md)).

**These two had no control for a key to hang on, and the roadmap said so in the plainest terms it
had.** `docs/roadmap.md`'s M5.13 entry read *"Blocked on a decision the maintainer owns, and it is
the whole of what is left. What control the Mixer draws for a fade and for a crossfade is undrawn"*,
and it recommended a capsule each on the transition row. The `enter` badge was written against that
recommendation: a route designed for a control nobody had drawn.

**The Mixer's transition row draws one capsule and it is the wipe's.** `crates/karakuri/src/keymap.rs`
records the reason at the `(mixer, enter)` entry — *"the row draws one capsule and `Operation::Wipe`
is what it asks for"* — and `karakuri-console`'s `focus::BUILT` declares the same four acting keys
for the Mixer that it declares for the other eight bays. So the keyboard reached the wipe and nothing
else on that row, and the two badges were a promise about a drawing rather than a description of one.

### The two records that had already decided this from the other end

[ADR-0232](0232-a-control-is-thrown-because-the-request-is-asynchronous-and-a-curve-is-a-helper-on-the-control-map.md)
is the frame, and its first decision is one sentence:

> **The panel is real-time control and authors no envelopes, and *thrown* means asynchronous.**

Its third says what follows for a surface that draws no control for a scheduled move, and says it
about this exact pair of operations:

> **And the panel hands in nothing at all today, which is this decision working rather than an
> exception to it.** `crates/karakuri` draws no control that asks for a fade, a crossfade or a
> selection, so its reading is `None` … Handing in a start it had no control for would be the window
> choosing a setting nobody chose.

And its fifth says where a scheduled move is authored instead:

> **Curve control, if it is ever wanted, is automation: a helper where the control map already sits,
> never a feature of the engine.**

[ADR-0236](0236-a-map-is-the-layer-between-a-surface-and-the-vocabulary-and-the-audit-is-one-of-the-things-it-does.md)
gave that position a name three hours later: a map is the layer between a surface and the vocabulary,
holding *"which surface gesture names which operation, whether a surface may reach a class of
operations at all … and helpers that carry time."* The third job is where a scheduled fade is issued
from, and the layer's only built instance today is `karakuri-midi`.

[ADR-0233](0233-the-consoles-mixer-has-no-crossfader.md) had already reached the same badge for the
crossfade from the panel's side, refusing a `go`-shaped button for it in its alternative c:

> **`gap` is the true statement today**: the panel cannot reach a crossfade at all, which is exactly
> what that badge means … Nothing here forbids such a button; it forbids claiming one before it
> exists.

**What none of the three had said is that the keyboard is the same surface.** ADR-0232's argument is
about what a *real-time route* authors, and the keyboard is one: `crates/karakuri`'s key match
dispatches through `karakuri_console::focus` into the same `Readout` the pointer reaches, on the same
frame, on the same thread. A key badge naming a press that would author a scheduled move is the same
claim as a panel badge naming a capsule that would.

## Decision

The maintainer's, on 2026-09-14, restating ADR-0232 rather than adding to it.

**1. The panel and the keyboard are real-time routes, and a fade from either is the opacity fader
moved.** A hand asking for a fade on this instrument drags the strip fader, or steps it with
`&uarr;&darr;` on the addressed fader with the keys in the Mixer — which is the row *Opacity*, and it
is `has` in the panel column, in the key column, in the MIDI column and in the MCP column. That is
[P-0085](../principles/0085-take-the-mechanism-that-exists-and-pay-the-bill-now.md) answered rather
than dodged: the mechanism a fade is made of exists, is drawn, and is under the operator's hand.

**2. No control that asks for a *scheduled* fade or crossfade is drawn on either surface.** A
scheduled move names an instant on the grid, a length in beats and a curve, and none of the three is
a thing a hand names at the moment of the press. Scheduled fades and crossfades are automation: they
are issued by the routes with a long turnaround — MCP today, and a helper at the map layer
(ADR-0236's third job) or a sequencer lane when one exists.

**3. Therefore both rows read `gap` in the panel column and `gap` in the key column.** The panel
column already did. The key column moves from `plan` to `gap` on 2026-09-14, and the page's own head
note gains the fourth reason an empty key column carries — *a scheduled move, which a real-time
surface does not author* — beside the three ADR-0331 wrote.

**This is a reading of ADR-0232 and ADR-0233, not a demotion to meet an exit.** The badge legend
fixes what the two words mean: `plan` is *designed: this surface is meant to reach it and does not
yet*, and `gap` is *nothing: this surface cannot reach it at all*. Under ADR-0232 §1 the panel is not
*meant to* reach a scheduled fade and never was; the `plan` badge was a description of a
recommendation that had not been taken, which is
[ADR-0213](0213-the-interface-milestones-meter-is-the-panel-column-and-has-means-an-operator-reaches-it.md)'s
failure mode read on the other column. **A badge that is corrected is not a badge that is lowered.**

**And the one sentence is worded once.** Both rows carry it identically in their tips, which is
[ADR-0315](0315-a-model-has-no-window-so-the-twelve-surface-rows-mcp-badges-are-gap.md)'s form for
exactly this shape of answer — twelve rows, one sentence, so twelve rows cannot drift into twelve
explanations
([P-0083](../principles/0083-a-refusal-carries-what-the-next-attempt-needs.md),
[ADR-0131](0131-one-refusal-sentence-per-mistake-across-the-surfaces-that-face-a-person.md)).

## Alternatives

### a. A fourth transition setting naming the move, and `go` performing it

The transition row already holds three settings that decide what the next move means — the wipe
shape, the quantum and the length — and `go` already fires one scheduled move. A fourth capsule
cycling *wipe · fade · crossfade* would make `go` fire whichever is named, which costs the row one
capsule, costs the vocabulary nothing, and gives the fade and the crossfade a key route for free:
`enter` on the `go` capsule with the keys in the Mixer, exactly the badge that was there.

**It loses on ADR-0232 §1.** What the fourth capsule authors is a scheduled move on a real-time
surface: the instant, the length and the curve are chosen ahead of the press and the press only
releases them. That is an envelope with three settings instead of a curve, and *the panel is
real-time control and authors no envelopes* does not become false because the envelope is spelled in
capsules.

**And it makes the settings row mean two things at once.** The operations page says of those settings
that *"they decide what the next fade, crossfade or wipe means"* — they are read by the strip's fade
and by the renderer chips as well, wherever the move is asked from. A capsule that selects *which*
move `go` performs is not a setting of that kind; it is an operand, and putting it in the row would
make one row both the modifier and the verb.

### b. Capsules of their own on the transition row for the fade and the crossfade

Two more capsules beside `go`, each firing its own operation with the row's settings. It is the
option the roadmap carried under recommendation, it needs no new state, and it answers the key column
by the same `enter` the wipe already uses.

**It loses on ADR-0232 §1 for alternative a's reason**, and for the crossfade it loses twice, because
ADR-0233 already declined it as a badge: *"A badge reading `panel transition-row button` would be a
plan written on the day the previous plan was deleted, for a control nobody has designed."* That
record refused the drawing's *claim* and left the drawing open; this one answers why the drawing is
not coming — the row would be authoring a scheduled move, which is the thing the surface does not do.

**And the fade's own sub-question has no answer on that row.** A fade is out *or* in on one deck
where a wipe and a crossfade name no direction, so a capsule would have to spell a direction, a
destination or both — three more decisions taken to keep a badge that was describing none of them.

### c. `enter` on the addressed fader scheduling a fade

No new drawing at all: the fader is already addressed and already stepped by `&uarr;&darr;`, so
`enter` on it could ask for a fade to where the fader is pointed, at the row's quantum and length. It
is the cheapest of the three and it is the one that keeps the key column's `has` side growing.

**It loses on ADR-0232 §1 with the surface's own affordance as the evidence.** `enter` is *the act
the addressed thing is for*, and what a fader is for is the level under your hand — the promise
ADR-0233 names, *that the position under your fingers is the value*. A press that scheduled a move to
somewhere the fader is not would make one control both the real-time value and the authored
destination, on the one surface where those are drawn as two different marks:
[P-0087](../principles/0087-name-the-property-never-the-shape.md)'s *asked for, and waiting* is
already drawn on that fader as a hairline the fill reaches for.

**And it would put the destination and the schedule in different hands.** The operator would name the
destination by stepping the fader — which *is already the fade*, landing immediately — and then ask
for it again as a scheduled move to where it has already gone. The gesture describes a move that has
happened.

## Consequences

- **`grep -c 'rt plan">key' docs/manual/operations.html` returns 0**, which is M5.13's exit condition
  met. The key column reads 46 `has` / 0 `plan` / 22 `gap`, against 46 / 2 / 20 on 2026-09-13; the two
  rows crossed from `plan` to `gap` and nothing moved into or out of `has`, so **nothing about what is
  built changed**.
- **No badge on any other column moves**, and neither row's MCP badge is touched: both read `has`
  against `operate`, which is the route this record names as the one that asks for a scheduled move.
  The MIDI badges stay `gap` for their own reason — a map line cannot say an instant, a length and a
  curve — and they move the day a map line names a helper.
- **`crates/karakuri/src/keymap.rs` states it at the `(mixer, enter)` entry.** That entry already held
  *Wipe the next deck in* alone; its comment now says why the other two rows are reached by no key
  rather than only why they are drawn on no control. `ROWS` and `NO_ROW` are unchanged, no test in
  that module reads a `plan` badge, and `key_column`'s six checks are green untouched — both badge
  directions read `has` rows only.
- **The manual's two pages carry it as specification.** Each row's tip on
  [the operations page](../manual/operations.html) names the real-time form (the row *Opacity*, the
  fader dragged or stepped), names the scheduled form's route (MCP today), and carries the one
  sentence about both columns verbatim. [The console page](../manual/console.html)'s transition-row
  tip, its strip-fader tip and its two notes — *The mixer has no crossfader* and *Nothing on a strip
  starts a fade* — stop saying a fade or a crossfade is asked for at the keyboard.
- **Rule 01 is not amended and the page's head note is.** *One vocabulary, four ways in* stays capped
  at seven rules ([the manual's own rule about itself](../manual/index.html)), and the exception is
  stated where the page already enumerates the reasons a key column is empty. That is where ADR-0331
  put the other three.
- **Nothing in the vocabulary, the gate, the conversion or any record changes.**
  `Operation::FadeDeck` and `Operation::Crossfade` keep everything they have, which is
  [P-0090](../principles/0090-a-surface-offers-it-never-decides.md) read in the direction ADR-0233
  read it: removing an affordance is the surface's to do, and the operation is not the surface's to
  touch.

### What this leaves undone

- **The helper still does not exist**, and this record builds nothing. ADR-0232 left it conditional —
  *if that sort of thing were added* — and ADR-0236 named the layer it would live in without building
  that layer either. Until one exists, MCP is the only route that asks for a scheduled fade or
  crossfade, and this record says so rather than implying a second.
- **Whether an automation helper and a sequencer lane are one mechanism is still open**, which is
  ADR-0232's question and ADR-0236's partial answer: the same job of the same layer, with steps where
  the helper has a curve, and one mechanism or two not settled.
- **Nothing forbids a control for a scheduled move being designed later.** ADR-0233's clause holds
  unchanged — it forbids claiming one before it exists — and this record adds the ground such a design
  would have to win on: it would be a surface authoring a move it does not perform, which is what
  ADR-0232 §1 says a real-time surface does not do, so the proposal is a revision of that record
  rather than a drawing this one declined.
- **The wipe is untouched and stays the row's one gesture.** `Operation::Wipe` is reached by `go` and
  by `enter` on the fourth control of the Mixer's head, and it is a scheduled move drawn on a
  real-time surface. That is not an exception to this record and is not resolved by it: a wipe's
  front position has no control anywhere, so the capsule is the only way a hand reaches the move at
  all, where a fade's fader is drawn four inches up the same strip.
