---
id: 0124
title: A save from MCP does not wait under the lock, and answers truthfully
status: accepted
date: 2026-08-22
supersedes: []
superseded_by: []
principles: [0094]
tags: [mcp, engine]
---

# A save from MCP does not wait under the lock, and answers truthfully

## Context

Live save needed a second front door — the first control in this system to want **key, MCP and
eventually GUI**, which required a request channel from the server back into the render loop, where
only loop-to-server existed.

## Decision

**Named before implementation, and required to be proved rather than commented.** `State` sits behind
a `Mutex` and `handle` spawns a thread per connection, so **waiting for a save to finish while
holding the lock stops every other connection** — including a client that only wants to read a
procedure. The brief said to drop the guard before waiting, **and to prove it with a test rather than
a careful comment**: one save in flight while another connection still answers.

The injection confirms it: moving `settled()` before `drop(state)` starves the second connection until
its read times out.

**The tool waits for the save, with a deadline.** Telling a model *saved* before the disk has answered
is exactly the failure three rounds had just been spent removing. And **the timeout message has to be
true**: *accepted, and the result has not come back* is neither a success nor a failure, and says so.

**One save path, not two.** The brief said so explicitly, because the duplication closed that morning
had **survived being deleted once** by moving; the MCP door hangs a reply sink on the existing
`Live::save_set` → `took_save` flow. Refusal messages reuse the existing sentences for the same reason.

## Consequences

- **The review's sharpest finding: deleting `reply.accepted()` leaves every test green.** Every MCP
  test's stand-in sends `accepted` **itself**, so **nothing checks that the real loop sends it**.
  Without it, a model whose call times out is told *the loop did not take this save, nothing was
  saved, it is safe to ask again* — while the save is running and will land. **A false statement to a
  model, and the two-stage reply exists precisely to prevent it.**
  The escape existed: this same change had already cut four free functions for testability, and only
  `playing_values` truly needs a GPU. **"It needs a GPU to test" was covering things that could be
  tested.**
- **A claim of uniqueness that was false, on the one control that now has two front doors.** *No such
  slot, in the words every surface says it in* — there are **four spellings** across the CLI crate, so
  a model and an operator meet different sentences for the same mistake. The roadmap had already
  promised M5 would inherit *the same sentences whoever meets them*.
- **The frame-ordering argument is half implemented.** Requests are taken **before** `frame::compose`,
  because a client asking to keep what is playing should not wait on a swapchain — but completions are
  drained **after** three early returns, so a window latched in `Skip::Fault` lands the save while the
  client waits out the deadline and is told it is neither a success nor a failure.
- A brief error of mine, returned: I said to poll where the loop polls `rebuilds`. `rebuilds` is
  drained inside `took_up`, not the frame body, so the request drain would have sat in a function that
  rarely runs. Placed beside the MIDI surface instead — **a control surface polled once a frame whose
  actions end in the method a key press ends in** — and before compositing, so a client still gets an
  answer on a frame whose swapchain is broken.

## Evidence

Session 2026-08-22T04:59Z–05:52Z.
