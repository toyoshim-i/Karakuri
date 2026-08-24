---
id: 0171
title: The deck advances, and each sink either gets the frame or misses it
status: accepted
date: 2026-08-25
supersedes: []
superseded_by: []
principles: []
tags: [engine, determinism, ui]
---

# The deck advances, and each sink either gets the frame or misses it

## Context

`frame::compose` took **one** sink, and if that sink could not acquire a target it returned
`Outcome::Skipped` **without calling the committing closure at all** — so no clock was read, no
record was written, and the deck did not advance.

That rule was not arbitrary. It is
[ADR-0078](0078-a-frame-that-is-discarded-must-not-already-have-been-recorded.md): the loop used to
read the clock, write a `tick` claiming those steps, measure the audio, and *then* find the
swapchain had nothing. `Outdated` arrives on every resize, so resizing during a recorded session
made the replay diverge from the performance by however many frames the window had abandoned.
Withholding the commit until `Sink::acquire` returned made the defect unrepresentable rather than
merely fixed.

**The rule cannot survive a second sink**, and the second sink is what M5 is for — a projector
window, the plugin sinks, and the picture in the console's own Program bay. With one sink, *the
frame was abandoned* and *the sink had no target* are the same sentence. With a projector beside
the window they are not: a frame that reached the projector and missed the window did not *not
happen*, and there is no answer to *did this frame advance the deck?* that is right for both.

It was also wrong with one sink, as a steady state rather than as a lost swapchain. The manual
promises that every output may be off — *"with every sink off you can still see the decks and work
on them, which is what setting up before doors is"* — and an operator who turns every output off is
stopping the publishing, not the instrument.

## Decision

**Advancing a frame and publishing it are two things.**

`compose` takes `&mut [&mut dyn Sink]`. Every sink is asked to acquire, before anything is
committed. Then the committing closure runs and the deck is rendered **unconditionally** — whether
every sink acquired, one did, or none did. Each sink that acquired is drawn into and presented; a
sink that refused has nothing else in the contract called on it. Zero sinks is an ordinary state,
not an error: the deck advances and `Present::draw` is not called at all.

**The guarantee ADR-0078 was protecting is stronger now, not abandoned.** Its question — can a
`tick` claim steps the deck never took? — was answered there by refusing to commit, and is answered
here by the commit and the render being adjacent inside `compose`, with no acquire, no sink and no
early return between them. The deck advances on every frame `compose` composes, so a `tick` is an
honest promise *unconditionally* rather than only on the frames that happened to find a target.

### The acquired set is carried in the slice, and the price is the caller's order

Nothing allocates on the frame path, so *which sinks acquired* may not be a `Vec<bool>`, and a
bitset would carry an arbitrary cap on how many outputs a performance may have. `compose` partitions
the slice in place instead: an accepting sink is rotated down beside the ones that already answered,
and `split_at_mut` gives the draw-and-present set.

**Rejected: a `Sink::acquired(&self) -> bool` on the trait.** It needs no partition and reads more
plainly, and it loses because it is a second answer to a question `acquire` already answered — every
implementation would have to keep the two agreeing, and one that did not would be drawn into after
refusing, which is the failure this whole seam exists to make unrepresentable.

**The cost is that `compose` may reorder `sinks`**, which is documented on it. Both production call
sites build the array at the call, so nothing observes the order.

Refusals are reported through `&mut dyn FnMut(usize, Skip)` with the sink's own index. The
`Skip::Fault` latch stays in the sink, for the reason it was put there: a caller that formatted a
message and then decided not to print it would allocate sixty times a second for as long as a
wedged window stayed wedged.

**A `present` error on one sink does not stop the others being presented.** Returning at the first
would let a wedged output cost every sink below it a frame it had already been drawn into — the
window waiting on a plugin, which `docs/plugins.md` says never happens. The first error is returned
after all of them have been tried.

## Consequences

- **`Live::frame` has no early returns left**, and all three of them were bugs waiting. A transient
  refusal — which is *every* frame during a resize, and every frame of a minimised window — used to
  skip the deck's event drain, the `procedure` records, the governor and the status line. A latched
  `Fault` skipped them *permanently*, so a build landing while the window was wedged was never
  announced and never recorded. A `present` failure skipped the same three. All of that now runs; a
  run whose window has stopped taking frames is exactly when an operator needs to be told the rest.
- **Three doc comments asserted the rule that was reversed** and none of them pointed at
  `frame.rs`, so none would have been found by following a link: `Clock::steps` said it is *"only
  called by a frame that is going ahead, which is why the acquire has to come first"*, and
  `Live::run_requests` and `Live::finished_saves` both documented their position as *"above every
  early return"* in a function that no longer has one. Rewritten, with the old reason kept as
  history.
- **A test asserted the defect's shape rather than the property.** `live_save_tests` scanned
  `Live::frame`'s source for the string `return` and `expect`ed to find one, so removing the early
  returns made it panic on its own `expect` — the suite's only failure. Its real property is that a
  save's outcome must not depend on there being a surface to draw on, which is *more* true now; it
  asserts that instead, in two halves, the second dormant against a `return` ever coming back.
- `Clock::last` moves on every frame the loop runs, so no interval is carried between frames. What
  still has to survive is the gap where the loop does not run at all, and `MAX_STEPS` is the
  anti-spiral clamp on that however long it was.
- The fan-out `docs/plugins.md` designs is now built rather than seamed. What is still owed is a
  second sink to put in the slice.
