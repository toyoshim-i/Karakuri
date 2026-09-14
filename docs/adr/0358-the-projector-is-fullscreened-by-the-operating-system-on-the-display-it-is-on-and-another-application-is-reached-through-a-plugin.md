---
id: 0358
title: The projector is fullscreened by the operating system on the display it is on, and another application is reached through a plugin
status: accepted
date: 2026-09-14
supersedes: []
superseded_by: []
principles: [0085, 0090, 0094]
tags: [outputs, ui, plugins, m6]
---

# The projector is fullscreened by the operating system on the display it is on, and another application is reached through a plugin

## Context

[ADR-0324](0324-an-output-is-a-named-destination-with-a-size-and-an-on-off.md) built the
projector as a second `winit` window with its own `WindowSink`, and left one line untaken: *"the
window that opens is not made fullscreen and not put on a chosen display … because choosing
*which* display is the part that is missing and a fullscreen with no way to say where is worse
than none."* The roadmap rewrite of 2026-09-14 read that as owed work and asked for two things
under M6: *borderless fullscreen on an operator-selected display using `winit` monitor
enumeration*, and *projector display refresh decoupled from console UI rendering passes*.

Neither is what the record says. The second is refused by
[ADR-0166](0166-the-engines-frame-and-the-panels-are-one-submission.md) — one encoder, one
submission, and a second submission over the deck's targets is the race that record is about.
The first turned on a question ADR-0324 left open: how a display is named. This record closes it.

## Decision

The maintainer's, on 2026-09-14, in three parts.

**1. The projector is a window, and that is the whole of it.** It is built (ADR-0324):
`Operation::RouteFrame { Output::Projector(0), on }` opens and closes it, its size is the size
the window manager gave it (ADR-0246), and it is the second `Sink` in `compose`'s slice
(ADR-0171).

**2. Fullscreen is the operating system's own gesture, on the display the window is on.** An
operator drags the window to the projector's display and makes it fullscreen the way any window on
that platform is made fullscreen; it fills the display it was on when the gesture was made. No
operation names a display, no chip lists monitors, and no key of this program's spells a fullscreen
of its own. `winit`'s default window carries the platform's fullscreen affordance, so nothing is
added for this.

**3. A picture for another application goes out through a plugin, not through a window.** Where
the destination is not a display but another program — a projection-mapping tool, a capture, a
second machine — the route is the output plugin sink [docs/plugins.md](../plugins.md) specifies:
Syphon on macOS, Spout on Windows, NDI across a network, each the platform's de facto
inter-application texture sharing, out of process behind the sink boundary. That side is
specified and not built, and it is where a display chosen by something other than a hand is
answered.

## Alternatives rejected

**`Operation::PlaceOutput { output, display }` with an index into `available_monitors()`,
drawn as a pulldown on the projector chip.** It is what the roadmap asked for. It loses on
ADR-0324 §1: a monitor index is an identity that changes when a cable does, and a map line or a
model holding one names nothing after the change. It also makes the chip carry a label the page
would then have to keep true across hot-plugs.

**A key of this program's that fullscreens the projector on its own display.** One line of
`winit` — `Fullscreen::Borderless(None)` — and it was the recommendation carried to the maintainer.
It loses because it is a second spelling of a gesture the platform already owns for every window,
and a second spelling is a second thing to teach and to keep consistent with the first
([P-0085](../principles/0085-take-the-mechanism-that-exists-and-pay-the-bill-now.md): the
mechanism exists). Where a platform has no such gesture, the answer is not a key but part 3.

**A frame loop of the projector's own.** Refused by ADR-0166, as above.

## Consequences

- The roadmap's M6 item is struck and reads as settled by this record.
- `crates/karakuri/src/gfx.rs`'s `Projector` doc and the projector chip's tip on the console page
  stop saying *not built* and say what the window is.
- What is owed and unscheduled is the output plugin side of `docs/plugins.md`, which is the
  route for every destination that is not a display in the room.
