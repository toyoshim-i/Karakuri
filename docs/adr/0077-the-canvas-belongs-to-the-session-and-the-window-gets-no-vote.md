---
id: 0077
title: The canvas belongs to the session; the window gets no vote
status: accepted
date: 2026-08-11
supersedes: []
superseded_by: []
principles: [0028]
tags: [engine, format, ui]
---

# The canvas belongs to the session; the window gets no vote

## Context

With the window settled as **a preview** — full-screen projection is possible, OBS capture in
between is common, and beyond that it is Syphon — the question became who decides the drawing size.

`--size` was doing two jobs and losing one of them live: `--replay` drew at `args.size`, while
`Live::resize` pushed the *window's* size into both `present` and the deck. So **a session performed
in a 1280×720 window and replayed at `--size 1920x1080` produces different pixels, and the stream
says nothing about which is right.**

## Decision

**`canvas` becomes the tenth session control** — the operator's, not a Set's, since every Set on the
deck draws to the same canvas — and it flows as a record, so a replay draws at the size it was
performed at.

| Flag | Meaning |
| --- | --- |
| `--canvas WxH` | The drawing size. The same meaning live, under `--render`, and under `--replay` |
| `--size WxH` | The preview window only. **Refused together with `--render`**, since there is no window |

The default rises to 1920×1080, because *not* defaulting to whatever size a window happened to open
at is the entire point of the change.

## Consequences

Three side effects, all in the right direction:

- **Replay draws at the size performed.**
- **Dragging the window stops reallocating every slot's targets every frame.** `Live::resize` had
  been calling `deck.resize` on the render thread while a window was being dragged — an allocation
  on the frame path, the very thing this design forbids, and **the last one left**.
- **`present_mode` became a choice.** `caps.present_modes[0]` is backend ordering that nobody
  selected and nothing displayed, so the same session paced differently per machine with no way to
  know. `Fifo` is supported everywhere and is the type's own default; it is now set explicitly and
  announced at startup — consistent with a tool whose position is to show the number.

The preview is **letterboxed**, because stretching lies about framing and framing is what a preview
is for. That makes OBS's window capture grab the bars, so there is a key that **snaps the window to
1:1 with the canvas**: press it and OBS gets pixel-for-pixel with no bars and no scaling.

## Evidence

Session 2026-08-11T09:40Z–10:03Z.
