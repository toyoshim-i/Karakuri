---
id: 0173
title: A frame's submission is not only its sinks
status: accepted
date: 2026-08-25
supersedes: []
superseded_by: []
principles: []
tags: [engine, ui, architecture]
---

# A frame's submission is not only its sinks

## Context

With the frame loop in `karakuri-engine`
([ADR-0172](0172-the-frame-loop-is-the-engines-because-the-cli-is-scaffolding.md)),
`karakuri-console` could finally call it instead of hand-rolling a second one. One thing was in the
way.

`compose` creates the deck's encoder, draws into every sink that took the frame, and **submits**.
The console needs one more thing in that same command buffer: its whole `egui` pass. Not for
tidiness — the picture is a colour attachment in the deck's pass and a sampled texture in the
panel's, and two submissions over one texture is a race whose order is the queue's business rather
than the caller's. That is
[ADR-0166](0166-the-engines-frame-and-the-panels-are-one-submission.md), and it is not negotiable.

So the panel's pass has to be recorded between the last sink's `after_draw` and the submit, and
there was no way to put it there.

## Decision

**`compose` takes a `finally: impl FnOnce(&mut wgpu::CommandEncoder)`**, recorded into the frame's
own encoder after every sink that took the frame has been drawn into and had `after_draw` called,
and before the encoder is submitted. It runs on every frame, including one that no sink took at
all. `karakuri-cli`'s two call sites pass `|_| {}`.

The reason is the title: **a frame's submission is not only its sinks.** A sink is where a
composited frame *goes*; the panel is the application drawing its own window, which happens to
sample what a sink produced and therefore has to share the barrier.

## The alternative that lost, and it will be re-proposed

**Make the panel a `Sink` whose recording work is the `egui` pass rather than the canvas** — no new
parameter, and the panel goes in the slice beside the picture. Given a `Sink::draw` with a default
body that draws the canvas, it is a small and rather elegant change. It loses on two things.

**It changes what `Sink` means**, from *where a composited frame goes* to *anything that wants a
slot in this encoder*. And it puts a **consumer in a list of producers**: the picture is a texture a
sink writes and the panel is the thing that reads it, so they are not peers, and the panel would be
the only member of the slice that does not want the present pass drawn into the target it was
handed.

**It makes `Outcome::reached` count two different things at once**, and this is the decisive one.
With the picture turned off from Outputs the panel is still in the slice, so a frame that reached
*no output at all* would report `reached: 1`. *Every output may be off* is a state the manual
promises outright — *"with every sink off you can still see the decks and work on them, which is
what setting up before doors is"* — and
[ADR-0171](0171-the-deck-advances-and-each-sink-either-gets-the-frame-or-misses-it.md) made it
representable on purpose. Admitting the panel to the slice takes that away without anything failing
to compile: the number goes on being returned and quietly stops meaning one thing.

**The general form is worth keeping**, because the mistake was made here before it was caught:
before widening an abstraction to admit a new implementor, ask what every summary value derived
from the collection now *means*, and whether the newcomer is a peer of the others or downstream of
one.

## Consequences

- **The console's sinks are the picture and deck A's preview cell, and nothing else.** Both are
  textures the panel samples, so `Sink::present` is `Ok(())` for both. The window is not a sink
  here; the CLI's window shows the canvas and the console's shows the panel.
- **The sizing hole is closed**, and it was two derivations agreeing by luck: `resumed` computed
  the first texture sizes one way and `window_event` computed them again another way, with nothing
  tying the two together and neither reachable from a test. There is one `aims(layout)` now, and
  `Engine::new` calls it. `docs/roadmap.md` recorded the hole with an injection that nothing caught;
  the same injection now fails three tests.
- **The picture and its rectangle are one statement.** `Presented::aim` returns the `Picture` after
  the fit, so a resize cannot leave the view holding a freed registration — which is its own test,
  and its own injected defect.
- **`compose` writes the tone-map uniform unconditionally**, so the console had to state a look for
  the first time. It states ACES at 1.0 — what `Present::new` already uploads — because this
  example has no session, no `look` record and no key that changes one, and any other value would
  have silently changed the picture with no test in the workspace able to see it.
- One assertion could not be written and is named where it is missing: that `finally` receives the
  frame's encoder *semantically*. Two submissions in the right order produce the right pixels, so
  no readback distinguishes them — which is precisely why ADR-0166 calls it a race nobody wrote
  down. The test compares the encoder's address and says so.
