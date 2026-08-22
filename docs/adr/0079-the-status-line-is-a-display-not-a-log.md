---
id: 0079
title: The status line is a display, not a log
status: accepted
date: 2026-08-11
supersedes: []
superseded_by: []
principles: [0030]
tags: [ui]
---

# The status line is a display, not a log

## Context

Asked when the tool becomes a proper GUI, I argued that the real gap was not the GUI but
**feedback** — every M2 feature had landed on *show the number, act on nothing* (semi-automatic
gain, the tempo octave, the NaN count, the panic key), and **each decision not to automate moves the
watching to a human.** The only display was a text line on stderr twice a second, which on stage
means reading a terminal while watching a projector. The manual reaching 350 lines was itself a
symptom: a tool needing that much prose to operate has an interface problem. And `overlay`,
`on-screen` and `HUD` appear nowhere in the roadmap — M5 is a node editor and a browser, not *what
is visible during a performance*.

Then came the question of whether that much text really scrolls past. **I had said it from
impression.**

## Decision

Measured: **287 characters per line with four slots**, which wraps to three or four lines in an
80-column terminal, twice a second — six to eight lines scrolling, for ever. The numbers supported
the concern **and showed my proposal was aimed at the wrong thing.**

The cause is not the medium. It is `eprintln!` — **a newline where there should be a `\r`.**

> The status line is a *display*, and the code emits it as a *log*.

The same content with a carriage return and a clear-to-end-of-line **updates in place**: nothing
scrolls, one line changes. No dependency, a few lines of code. Events keep their newline and the
next tick redraws the status beneath them.

So the order was corrected:

1. **In-place update with `\r`** — this is what the complaint was actually about.
2. **Fit 287 characters into 80 columns** — in place, it is still multiple lines otherwise.
3. **An overlay**, which remains the real answer to "read a laptop while watching a projector", and
   is much larger than the first two.

## Consequences

- **Both control surfaces are blind.** The keyboard was never meant to be the instrument — there is
  MIDI in — but there is no MIDI **out**, so a controller's LEDs and motor faders do not follow the
  deck.
- Recorded for the overlay when it comes: **the first one needs no font.** What reads fast in the
  dark is shape, not digits — level bars, fader positions with a transition's destination,
  residency as colour, a beat-phase pulse, the `x2?` warning, frame-budget headroom. All rectangles.
  Text rendering stays in the terminal for when a precise number is wanted.

## Evidence

Session 2026-08-11T14:03Z–14:07Z.
