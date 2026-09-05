---
id: 0246
title: The render size belongs to the output, and the session's canvas is only its default
status: accepted
date: 2026-09-02
supersedes: [0077]
superseded_by: []
principles: [0086]
tags: [engine, ui, format, operations]
---

# The render size belongs to the output, and the session's canvas is only its default

## Context

[ADR-0077](0077-the-canvas-belongs-to-the-session-and-the-window-gets-no-vote.md), 2026-08-11, made
the canvas a session control: `--canvas WxH`, defaulting to 1920×1080, flowing as a record so a
replay draws at the size it was performed at, and **the window gets no vote**. `karakuri-cli`
implements all of it.

**Its premise was that the render size changed the picture, and that premise was true when it was
written.** `point_size` was a count of pixels then, so the same Set at 112x63 drew sprites eleven
times fatter than at 1280x720 — a preview that was not a preview of anything, and a replay at
another size that was a different performance. Against that, taking the vote away from the window
is the only safe answer.

**Two changes on 2026-09-02 removed the premise.**

- `point_size` became `point_rate`, a fraction of the render target's height (96c9cc7, specified in
  `docs/ir-spec.md` under *`point_rate` is a fraction of the target's height, and not a count of
  pixels*). A sprite is the same share of the frame at every size. **That change has no ADR of its
  own**; its argument is in its commit message and in the specification.
- A primitive below a pixel is drawn at one pixel and dimmed rather than dropped
  ([ADR-0245](0245-the-sub-pixel-compensation-is-paid-in-the-colour-because-alpha-is-coverage.md)),
  which closed the last way a small target still produced a different picture — and the worst one,
  because it was arbitrary: which sprites survived was decided by where they landed on the sample
  grid.

**So the rule is now protecting against something that no longer happens, and it costs.**
`Operation::RouteFrame` carries `output: Undecided`, no output has an identity anywhere in the
workspace, and a projector on a 4K display, a Syphon publish and the picture in the Program bay are
all forced through one session-wide number. The console page's own mock has been drawing a
`1920×1080` pill over a program that renders at a 1280×720 constant for as long as the panel has
existed.

## Decision

**The render size is a property of an output.** It is held in the Outputs row beside the destination
it belongs to, and it is what that output is actually rendered at. Two outputs may hold two sizes.

**The operator, or the destination window, decides it.** A projector window fullscreen on a 4K
display renders at what that window is, unless the operator says otherwise — and the operator may
always say otherwise, because a thermal budget is a reason to send 1080p into a 4K window and
[P-0079](../principles/0079-nothing-takes-the-show-down-and-nothing-takes-it-away-from-the-operator.md)
is that rule.

**The session's `canvas` record degrades to a default and a reference.** It is what an output starts
at when nothing else says, and what a replay draws at when the stream carries no per-output size. It
is no longer the authority. `--canvas` keeps its spelling and loses its finality.

**This is safe because the picture does not depend on the size**, which is now a standing rule:
[P-0086](../principles/0086-a-procedure-knows-only-what-it-declares.md), *The render size is not
part of the picture*. Every mechanism that makes it true — `point_rate` as a fraction, the sub-pixel
floor, letterboxing rather than stretching, and no ambient a procedure could ask the size with — is
the price of this record rather than an independent nicety, and P-0086 is where that is written so a
later change cannot quietly take one back.

## What this does not settle

Named rather than left for the next reader to meet.

- **Whether two outputs at two sizes cost two renders or one render and a scale.** Nothing here
  answers it. Today the deck's slot targets, the mix and the present pass are one canvas, and
  `Present::draw` already fits that canvas into targets of different sizes a frame — so *one render,
  scaled per output* is what the code would do if nothing changed, and *render per output* is what
  the words most plainly say. This is the first question M5.6 meets.
- **What else an output's identity holds**, beyond a destination and a size. `RouteFrame`'s payload
  is still `Undecided`; this record adds one field it must carry and does not close the question.
- **What a replay reproduces** when the outputs it was performed to are gone: the sizes it rendered
  at, or the canvas. ADR-0077's reason for recording the size at all survives this record and needs
  an answer shaped for per-output sizes.

## Alternatives rejected

**Keep ADR-0077 as it stands.** Its premise is gone, and the cost of keeping it is concrete: a
projector fullscreen on a 4K display renders 1080p and upscales because a session-wide number said
so, and no output can be given a size of its own without contradicting the record.

**Let the window decide and give the operator no say.** It is the simplest reading of *the
destination decides*, and it hands the show to whatever display got plugged in. An operator whose
machine cannot afford 4K must be able to say so, and P-0079 forbids a rule that takes that away.

**Drop the session `canvas` record entirely, since it is no longer the authority.** A replay of a run
whose outputs no longer exist still has to draw at something, and a reproduction that picks its own
size is not a reproduction. It stays, with a smaller claim.

**Wait for M5.6 and decide it there.** The canvas half was sitting in the roadmap's *decisions nobody
has taken* as though it were open, when what was open was one clause of a record four weeks old.
Leaving it there is what had the risk badge's bands and the whole-frame budget reading as blocked on
a decision nobody was going to take.

## Consequences

- **[P-0086](../principles/0086-a-procedure-knows-only-what-it-declares.md) exists**, and it is
  what the mechanisms above answer to rather than each answering to itself.
- **ADR-0077 is superseded** and annotated with the clause that reversed. Its `--canvas` flag, its
  refusal of `--size` as a way to say the canvas, and its `canvas` record all keep working.
- **`crates/karakuri/src/main.rs`'s `const CANVAS: (u32, u32) = (1280, 720)` is wrong twice over
  now**: it is a session-wide constant where the size belongs to an output, and it is a measurement
  default — the workspace's reference workload — standing in for an instrument's. What replaces it
  is an output's size, and whoever wants the reference workload types it.
- **`docs/manual.md`'s *The canvas is fixed for a run* becomes false when this is built**, and is
  true today. It is not edited here; nothing is built by this record.
- **The roadmap's *decisions nobody has taken* loses the canvas half**, which is now decided and
  whose implementation is M5.6's along with the rest of what an output is.
