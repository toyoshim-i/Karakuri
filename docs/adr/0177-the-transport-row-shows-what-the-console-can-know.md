---
id: 0177
title: The transport row shows what the console can know, and the budget is the display's
status: accepted
date: 2026-08-25
supersedes: []
superseded_by: []
principles: []
tags: [ui]
---

# The transport row shows what the console can know, and the budget is the display's

## Context

The mock's transport row carries ten things: a BPM, an `audio-in` pill, a beat grid, a bar number,
`tap`, `learn`, `map · nanoKONTROL2`, a frame readout, `landed`, and `● rec`. The console has an
engine behind it and nothing else — no audio input, no MIDI map, no tap, no build watcher, no
session recorder.

## Decision

**Four things are drawn**: the BPM, the beat grid, the bar number, and the frame readout. The other
six are named in the source with what is missing behind each, and drawn nowhere. A pill that looks
like a control and does nothing is the scaffolding-that-looks-finished this crate refuses, and this
is the third row to make that argument after the deck previews and Outputs.

**The values arrive as plain numbers through the seam `View::picture` already uses** — a
`View::transport: Option<Transport>` written per frame by whoever owns the engine. `None` is a
console with no engine, which is every test in this crate, and it draws **nothing at all** rather
than a row of zeroes: a zero is a reading, and inventing one is worse than an empty row.

### `beats_per_bar` is a field, not a four

The mock draws four dots and `karakuri_signal::oscillator::BEATS_PER_BAR` is 4 — and **that
constant says of itself that it is provisional**: *"v0.2 of the IR spec has no time-signature
concept anywhere … a fixed assumption of common time … Treat it as provisional until a real time
signature shows up in the format."*

Transcribing a 4 into the console would copy a number whose owner has already marked it temporary,
and the day a time signature lands the panel would draw four dots against a grid of three with
nothing saying so. So the grid is as many dots as the field says, and the example asks
`karakuri-signal` for the constant by name — a **dev**-dependency, example-only, so `src/` gains
nothing and ADR-0156's seam is intact.

### The budget is the display's refresh interval

`16.67 ms` on a 60 Hz panel, read once from `winit`'s `refresh_rate_millihertz` when the window
opens. The surface is `PresentMode::Fifo`, so a frame longer than one interval misses a vsync;
that is what the number means. `None` where `winit` will not say, and then the row draws the frame
time with no budget beside it. A window dragged to a display of a different rate keeps what it
opened on, which is written down where it is decided.

**It is not `swap::DEFAULT_BUDGET_MS`.** That 20 ms is what a *candidate Set* must hold to survive
a hot swap, taken offscreen at a fixed 1280x720 as a median over a window of frames. The mock's own
tooltip runs the two together — *"12.4 of 16.6 — there is headroom. A candidate that cannot hold
this is rolled back on its own"* — and they are two different numbers about two different things.

### A rate is a claim about now; a frame time is a claim about a frame

`fps` is `Option`, and it is `None` while nothing is live: the loop has stopped drawing, so there is
no rate. The frame time beside it stays, because it is a fact about a frame that happened and does
not go stale by standing still. The row drops the words it has no number for rather than printing a
zero.

## Consequences

- **A geometry test asserted nothing, and P-0025 is what caught it.** The house style states a
  region's arithmetic in terms of the `size::` constants — which means a test of that arithmetic
  cannot catch a *wrong constant*: transcribing `.transport`'s `gap: 14px` as 10 left the test
  green, because it compared the layout against the same 10. The constants are now asserted against
  the stylesheet's literals, and the relations against the constants, which are two different
  claims. Every `size::` constant in this crate has the same shape, and this is the first test to
  separate them.
- **"No engine means nothing at all" was being decided in two places**, and they agreed until they
  did not: `View::draw` zipped the row's geometry with the values, so a geometry function defected
  to return a row of zeroes still painted nothing and the test passed. The row now carries the
  `Transport` it was measured from — whoever measured and whoever paints are one statement, which
  is the argument `view::Picture` is built on — and the rule lives in one place.
- **`Kind::Row` became `Kind::Transport`.** It was the shared kind for both rows, its own
  documentation described the transport specifically, and Outputs took a kind of its own when it
  was built. One region, one kind, following that precedent.
- **Three things in this workspace are called *transport***: this row, `karakuri-console`'s
  `view::Transport`, and `Deck::transport(slot)` — which is a *slot's sync mode* (`Free`, `Tempo`,
  `Beat`) and a different thing with the same word on it. ADR-0159 settled the console's words
  against the manual's; this is the first place one of them collides with the engine's, and it is
  recorded rather than resolved because both names are right in their own document.
- **The mock's row wraps and this one cannot.** `.transport` is `flex-wrap: wrap` and the
  arrangement pins the row at exactly 48 with its minimum meeting its maximum, so there is no second
  line for a wrap to go on. A row too narrow draws nothing.
- The position is read as of the end of the previous frame — the deck advances its oscillator inside
  `render` — which is the same one-frame lag the frame cost beside it has, stated once.
