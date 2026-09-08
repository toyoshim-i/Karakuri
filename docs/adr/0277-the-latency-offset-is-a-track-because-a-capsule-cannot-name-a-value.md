---
id: 0277
title: The latency offset is a track, because a capsule cannot name a value
status: accepted
date: 2026-09-08
supersedes: []
superseded_by: []
principles: []
tags: [console, transport, manual, m5.4]
---

# The latency offset is a track, because a capsule cannot name a value

## Context

M5.4's *Nudge the latency offset* row carries a `plan` panel badge, and its own tooltip on
[every operation](../manual/operations.html) said why: *"The panel badge stays designed because the
pill that would set it outright is not drawn."*

**What the mock draws is a capsule.** `docs/manual/console.html`'s `.tracker` held
`<span class="pill">offset −15 ms</span>`, and the tip on it said **"Click to set it outright; o and
p step it five milliseconds a press."** Those two sentences do not fit together. A press on a capsule
can ask for exactly one thing — *the next one* — which is what every other capsule on this console
does: the blend mini cycles three modes ([ADR-0187](0187-the-blend-mini-cycles-and-a-map-learns-the-three-it-cycles-through.md)),
the tone map cycles four, the sync chip cycles the modes a deck's material can honour, and the
Library bay's two filter fields step through a list. Every one of those is a **closed list a press
can walk**. The offset is 400 milliseconds in steps of five: eighty values, signed both ways, and
nothing in the vocabulary names a *step* of it at all —
`karakuri_operation::Operation::SetLatencyOffset` carries `ms` and says so at the variant:
*"Absolute, though the heading says nudge … an absolute value can express every nudge and a nudge
cannot express a setting, and a fader has to be able to reach it."*

**The manual had already decided it, one control along.** `console.html`'s note *The latency offset,
and which offset it is* says **"The value is absolute and the keys are the nudge — five milliseconds
a press — because a control that could only be nudged is a control no fader can reach."** That
sentence appears once in the manual, in this note — and
`karakuri_console::view::LookRow::exposure` quotes it as *"The manual is where the words for that
come from"*. The exposure track was built from the offset's argument before the offset had a control
at all.

**And the program was written for it.** `crates/karakuri/src/main.rs`'s `nudged` says
*"A setting becomes the step that reaches it, which is what lets a fader emit this operation the day
one exists without a second application path."*

## Decision

**The mock moves first (`docs/contributing.md` §5 step 3), and the offset becomes a track.**

`docs/manual/console.html`'s `.tracker` now draws the exposure group's shape one group to the left: a
faint `offset`, a `.fader` at `width:80px` with its fill, and the figure `−15 ms` after it. The note
gained two paragraphs saying why, and the operations page's row tooltip says what a press does
instead of why the badge is still `plan`.

**Eighty pixels, because there are eighty presses.** `LATENCY_OFFSET_RANGE` is `-200.0..=200.0` and
`LATENCY_OFFSET_STEP_MS` is 5, so the span is exactly eighty steps and the track is exactly eighty
pixels: **one pixel is one press of `o` or `p`**, and neither surface can reach a value the other
cannot. That is `EXPOSURE_TRACK_W`'s own construction — *"A width picked for looks would have been a
number with nothing behind it"* — arriving at the control the argument was borrowed from.

**Linear, where the exposure is logarithmic.** An exposure is a *ratio* and its track says so; a
latency offset is a **difference** between two arrival times, five milliseconds is five milliseconds
at either end, and the range is symmetric so the middle of the track is exactly zero.

**With nothing open there is no track.** `View::tracker`'s `offset_ms` is `Option<f32>`, and `None`
draws no offset control at all rather than a track at
`karakuri_environment::audio::DEFAULT_LATENCY_OFFSET_MS`. An offset belongs to a session; the default
is where the *next* session starts, not a value anything is holding. This is `View::audio`'s own rule
one control to the left — *empty is a state and unasked is not* — and the page now says it.

## Alternatives rejected

**A capsule that steps, like `o` and `p`.** The cheapest thing to build, and it is what "the same
call" would have meant. It loses to the manual's own sentence above: a control that can only be
nudged is one no fader can reach, and the row's MIDI column is empty *because* nothing names an
absolute target yet — building the panel's only route as a stepper would have made the panel the
second surface that cannot ask for a value. It also loses to
`Operation::SetSync`'s sentence about the same shape: *"A control that can only cycle cannot offer
it: a cycle asks for the next mode and never for the one it is standing on."*

**The capsule as its own track** — hit-test along the width of `offset −15 ms` and map it to the
range. It keeps the mock's picture and breaks the reason `LookRow::tone` is held to the widest of the
four operator names: the capsule is as wide as the number in it, so the target would move under the
hand *as the hand set it*, and worst at the moment it was being used most precisely.

**A card, like the audio-in and arrangement pills.** Those two open a list of **files and devices an
operator named**; a card here would be a list somebody had to invent, and
[ADR-0225](0225-a-menu-is-a-gesture-in-hand-rather-than-a-rectangle-on-the-panel.md) prices a menu at
*every pointer event on the console until it is shut* — which is not a price to pay to reach a number
that is already on screen.

**Leave it undrawn and report it.** Honest, and it was the fallback. It loses because there is
nothing undecided underneath: the operation is absolute, the ends and the step are public constants,
the application path exists and is documented as waiting for a fader, and the alternative shape had
already been argued down in the manual. Leaving it would have held M5.4's exit open on a sentence
rather than on a decision.

**Draw the capsule as a readout with no press.** Refused by `karakuri_console::view`'s own
documentation: *"a pill that looks like a control and does nothing does not get replaced."*

## Consequences

- **`docs/manual/console.html` and `docs/manual/operations.html` both moved**, and they are the only
  places the old shape was specified. The row's panel badge is `has`.
- **`karakuri-console` restates the two ends and the step**, as `view::LATENCY_OFFSET_MIN_MS`,
  `LATENCY_OFFSET_MAX_MS` and `LATENCY_OFFSET_STEP_MS`, because it depends on nothing that could
  reach `karakuri-environment` (ADR-0156). Unlike `EXPOSURE_STOPS` — whose originals are private to
  `karakuri-cli` and cannot be compared with anything — **these are held to the originals by a
  test**, `the_consoles_offset_track_spans_the_sessions_own_range` in `crates/karakuri/src/main.rs`,
  which is the one package that names both crates.
- **`view::OFFSET_TRACK_W` is derived and not chosen**: it is the span divided by the step.
- The offset is one of three controls in `view::tracker_group`, which is one row of
  `input::PROBES` ([ADR-0274](0274-a-control-is-a-row-in-the-consoles-own-table.md)) and one entry of
  `press_handler::ASKED`. `CONTROLS` is 39.
- **`arrangement` and `look` each took a parameter.** The arrangement pill is laid out from the
  tracker group's right edge now rather than from the audio-in pill's, so both take
  `tracker: Option<Tracker>`; every call site in `crates/karakuri-console/tests` moved with them.
- **Nothing about `o` and `p` changed.** They step, they read the value they are standing on, and
  they end where the press ends: `nudged`, and `Audio::nudge_latency_offset` is still the one place
  an offset is applied and the one place it is clamped.
