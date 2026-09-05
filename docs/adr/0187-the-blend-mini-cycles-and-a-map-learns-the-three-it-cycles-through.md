---
id: 0187
title: The blend mini cycles, and a map learns the three it cycles through
status: accepted
date: 2026-08-26
supersedes: []
superseded_by: []
principles: [0090]
tags: [ui, vocabulary, decks]
---

# The blend mini cycles, and a map learns the three it cycles through

## Context

The mixer strip drew six readings and two of them became controls
([ADR-0185](0185-a-fader-translates-a-drag-into-an-operation-and-applies-nothing.md)): the trim's
knob and the fader's. The blend `.mini` was painted and dead — a chip saying `add`, `over` or `max`,
with a tooltip in the mock that says *click to cycle* and nothing behind it.

**The reason it stayed dead was a misreading of a principle, and that is the first thing this record
corrects.** ADR-0185's consequences said the mock's tooltip and
[P-0090](../principles/0090-a-surface-offers-it-never-decides.md) disagree —
the tooltip cycles, and a vocabulary has no cycles — and concluded that the chip must **name** one
of three, by a menu or by three targets. `docs/roadmap.md` said it twice, once as work and once as a
decision nobody had taken.

The premise was never true. P-0074 forbids a cycle **operation**, and its second paragraph names
this exact control as the thing it permits:

> **A toggle is an affordance, built over operations by whoever draws the control**, and it belongs
> there. A pad that flips on-air is one control emitting two operations; **a mini that cycles the
> blend is one control emitting three.** The operator sees a toggle; the vocabulary never does.

So there was no conflict to resolve, and the work the conflict was blocking was four lines. What is
genuinely open — and what this record is actually for — is the half nobody was looking at: **what a
MIDI map learns from a control whose affordance is a cycle.**

## Decision

### The chip cycles, and the operation names where it arrived

A press on the blend `.mini` moves the deck to the next of `BlendMode::ALL` — `add`, `over`, `max`,
wrapping — and emits `Operation::SetBlendMode { deck, blend }` naming the **destination**. Never a
step, because there is no step in the vocabulary to name.

The cycle is `view::after`, four lines in `karakuri-console` and nothing at all in
`karakuri-operation`. That division is P-0090's: the vocabulary owns the three values, and whoever
draws the control owns the order a pointer walks them in.

**The console translates and applies nothing**, which is ADR-0185's shape unchanged: the panel emits
an `Operation`, `examples/panel.rs` turns it into `Record::Blend` and the record moves the deck, and
the strip then draws what the deck says. It is the third control on `input.rs`'s rule 3 — *"a
control the console draws is the panel's"* — asked the way the other two are: the derivation that
draws it, asked whether the point is on it, with nothing stored.

### `view::Strip::blend` is a `BlendMode` and no longer the engine's word

It was a `&'static str` handed straight through from `Blend::name`, with a doc comment arguing that
the console has nothing to do with the blend but draw the engine's word. **Making the chip a control
ended that argument**: to say what the *next* mode is, the console has to know which of the three
this one is, and a string it cannot exhaustively match is the wrong carrier for that.

So the field is `karakuri_operation::BlendMode`, `BlendMode` gains `ALL` and a lower-case `name()`
built as a `match` — the way `karakuri_engine::deck::Blend::name` is one, so a variant added to the
enum does not compile until it has a name — and the harness converts the engine's `Blend` into it.
**A fourth engine blend mode with no operation variant then fails to compile at the harness**,
rather than drawing a word on a chip no control can reach and no map can ask for. That is the
vocabulary doing its job rather than a cost it imposes.

### Why the alternatives are not merely unchosen, but unavailable

Each was measured rather than argued about. The strip is **53 wide inside `.strip`'s
`padding: 7px 4px`**, in a **61** track; the mode row already holds a **23**-wide mask chip beside
the blend one; `.mini` is 9px type in `padding: 0 6px` inside a 1px border, and
`.strip-mode` puts a 3px gap between the two. Widths below are `egui`'s galleys at
`MINI_SIZE`, taken this session at the default proportional face.

**Three segments side by side does not fit, before anything is added to it.** The three words as
**bare glyphs** are `add` 15.06, `over` 17.84 and `max` 16.84 — **49.75 of the 53 available**, with
no padding, no separator, no border and no room for the mask chip that shares the row. The most
degenerate segmented control anyone would draw — the three words touching, one hairline between each
pair and one round the outside — is **53.75, already over** with the mask chip still to place. This
is not a tight fit that could be tuned; there is nothing to tune it with.

**Stacking the three fits the width and costs the window.** A column of three minis is
3 × 15.5 + 2 × 3 = **52.5** against the row's 15.5, so **+37** on `STRIP_H` — and the mask chip
shares the row and needs the same treatment, so **+74**. That number is not local: `STRIP_H` is a
term in the mixer bay's 316, which is a term in the right pane's 530, which is a term in the
console's minimum window height of **632**. Stacking makes it **706**. The manual's claim about this
bay is *"four channel strips, all visible, nothing that scrolls out of reach mid transition"*, and
paying 74 pixels of everybody's screen for a control that is pressed a few times a set is the wrong
trade.

**A popup fits the geometry and costs the input rule.** Three things, all of them structural:

- **A rule ordered *before* the boundary's first refusal.** `input.rs`'s rule 2 is that a pointer
  within `GRAB` of a boundary is the panel's, and rule 3 — the controls — comes after it, which is
  what *first refusal* means. An open picker floats over the panel and can open next to a bay edge,
  so a picker under a boundary's 6px band would have dead rows. It would have to be asked **first**,
  which inverts the one ordering that module is written around.
- **Somewhere to store which picker is open.** Nothing in this console stores control state today:
  the Outputs sink reads `Layout::visible` rather than keeping an `on`, and the mixer's values are
  the harness's every frame. An open picker is the first piece of state the console would own about
  its own surface.
- **A dismissal path rule 3 forbids in so many words.** A press outside an open picker has to close
  it, which means claiming a press in order to throw it away — *"claiming a press in order to throw
  it away would put the rule and the act out of step."*

The cycle costs none of the three, and P-0090 names it.

### What a MIDI map learns from this chip

**This is the decision the affordance forces, and it is the reason this record exists.**

The obvious mapping for a cycling chip is a pad that means *next*, and that is exactly the shape
P-0074's third paragraph warns about:

> a surface that can only step has no way to arrive, and two surfaces stepping the same control
> disagree about where they are.

Two pads meaning *next* on two controllers, or a pad and this chip, cannot agree on where the blend
is; and a model over MCP that wants `over` has no way to ask for it through a *next*.

**The decision: the component owns three operations. The pointer is shown a cycle, and a map is
offered the three values.** This is the maintainer's own model of what a GUI component is — a
translator mapping one Karakuri operation to N input methods — with N equal to 3 here, where a fader
is one operation to one continuous input. The chip is a single affordance over three named
destinations, and every other surface is offered the destinations rather than the affordance:
`SetBlendMode { deck, blend: Over }` is a thing a pad can carry, a map can parse, and MCP can say.

**Nothing MIDI is implemented by this change.** `karakuri_midi::map::Action` still has
`CycleBlend` and has not moved onto the vocabulary. This is what that migration aims at, recorded
now because the affordance is what makes the question concrete and because a decision reconstructed
later is one reconstructed from transcripts.

## Consequences

- **ADR-0185's blend bullet is corrected rather than revised** — the record says what was concluded,
  that the premise was false, and points here
  ([P-0066](../principles/0066-an-adr-is-a-description-of-history-corrected-but-never-revised.md)).
  `docs/roadmap.md` said the same thing in two places and both are now what remains open, which is
  the MIDI half.
- **`docs/manual/console.html` listed four blend modes** — *"add, over, screen, multiply"* — where
  the engine has three and `karakuri-operation` names three. Corrected to the three that exist, and
  the tooltip now says what the note learns: one of the three, never the step.
- **Four console controls answer a pointer now**, not three: the Outputs dot, the two faders and
  this chip. `input.rs`'s rule 3 did not change to hold the third, which is what it was written for.
- **The mode row already overflows the strip and nothing clamps it**, which building this found and
  which is now a test rather than a sentence. Blend chip plus 3px gap plus the 23-wide mask chip is
  55.06 / 57.84 / 56.84 against a 53 content box, centred, so it spills 1.03–2.42 each side into
  `.strip`'s own 4px of padding and is still 1.58–2.97 clear of the 61 track. Nothing collides
  today. `tests/blend.rs` asserts the row stays inside the **track**, because the tracks tile with a
  gap between them, so a row inside its own track cannot reach its neighbour — a fourth mode or a
  longer word breaks the build rather than painting one strip's chip over the next.
- **The clearance from the boundary's grab band was measured for this control too, not inherited.**
  The chip is at the bottom of a strip where the knobs are in the middle of one, which is a
  different clearance against a different boundary. `tests/blend.rs` asks `Layout::hit` directly as
  well as `claim`, which is the test ADR-0185 caught: `claim` says *the panel's* for a boundary
  **and** for a control, so a version that only asked `claim` passes with `GRAB` widened to 60.
- **The mask mini is still a readout and for a harder reason.** ADR-0185 recorded it: there is no
  `SetMask`, `WipeKind` exists only inside a transition setting, and *"it is a readout of state no
  operation can set"*. The blend chip's answer does not transfer, because what the blend chip needed
  was an affordance and what the mask needs is a row on the operations page.
- **The tally is next and is not this.** It is two operations for three states and the console draws
  the *effective* residency while `SetResidency` sets the *requested* one, so its open question is
  what the chip shows while the two disagree — a question about readback, where this was a question
  about the pointer.
- `karakuri-operation` gained `BlendMode::ALL` and `BlendMode::name`, which is the first time
  anything in that crate has been more than a name and a payload. Both are stated as *what values
  exist* and *what they are called*; the cycle is deliberately not there.
