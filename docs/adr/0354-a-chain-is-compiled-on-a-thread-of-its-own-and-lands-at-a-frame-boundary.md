---
id: 0354
title: A chain is compiled on a thread of its own and lands at a frame boundary
status: accepted
date: 2026-09-14
supersedes: []
superseded_by: []
principles: [0091, 0092, 0094]
tags: [engine, master-chain, threading, m5]
---

# A chain is compiled on a thread of its own and lands at a frame boundary

## Context

[ADR-0340](0340-kind-l5-is-written-and-the-master-chain-is-an-ordered-list-of-them.md) named this
as owed under *Where a slot is compiled is the second pass's to move*: `mix::build_chain` compiled
procedures and made pipelines **on the thread that applied the record**, which on every live path
is the thread that encodes the frame. What that cost was a `naga` pass and a render pipeline per
slot, plus an entry target, up to two ping-pong targets and up to two retention textures — an
allocation and a compile on the render thread, on the frame an operator pressed a Master row.

`crates/karakuri-engine/src/lib.rs` states the invariant this broke in its first line —
*"Zero allocation on the render thread; shaders are never compiled on the render thread"* — and
`README.md` states it as a pillar. A Set has satisfied it since
[ADR-0033](0033-freeing-on-the-render-thread-is-the-same-invariant-as-allocating.md): `HotSwap`
builds on a worker, installs at a frame boundary by `try_recv`, and hands the outgoing Set to a
graveyard drained by that worker, because **freeing is the same invariant as allocating**. A chain
had none of that.

The gap was tolerable while a chain was three fixed passes set once at startup. It stopped being
tolerable when the Library learned to drop a procedure onto the chain: the list now changes under a
performer's hand, at a rate limited only by how fast a row can be pressed.

## Decision

**The master chain is built on a worker thread named `karakuri-chain`, one per `Present` whose
chain a live operator can change, and the finished chain is installed at a frame boundary.**

### 1. A second named thread, not a second `Source` on `karakuri-build`

The worker pattern is `swap.rs`'s and is reproduced in `chain_swap.rs`: a thread holding clones of
the `Device` and `Queue` the render thread renders with, a request channel in, a `Done` channel
back, `try_recv` at the frame boundary, and a graveyard by `try_lock` — **never `lock`**.

Sharing `karakuri-build` was the alternative, and
[ADR-0255](0255-three-clocks-run-at-once-and-a-slower-ones-work-never-lands-on-a-faster-one.md)
is what settles it. A Set build is generation-clock work by that table's own reckoning and it is
long: `Set::build_many` compiles every layer, allocates element buffers at the declared capacity —
12 MB at 262144 elements — and is then followed on the same thread by a probe measurement, which is
a submission and a `device.poll(wait)`. A chain build is three shader modules and four textures. On
one thread a press on a Master row lands behind whatever Set is being built, and how long the
operator waits for a bloom to appear becomes a function of how large the Set someone else is
loading is. That is a slower clock's work delaying a faster one's, which is the thing ADR-0255
rules out.

`contributing.md`'s *no shared abstraction without two call sites* is satisfied without a shared
abstraction being built: this is the second call site, so a generic worker **could** be extracted —
and is not, because the two workers differ in what the thread is for rather than only in what it
builds. `karakuri-build`'s loop asks a `Source` and measures what it built; `karakuri-chain`'s
takes a job off a channel and builds. What they share is thirty lines of channel and graveyard
plumbing, and a generic over `Request`, `Done`, the build function and the measurement step would
be longer than both. The two are kept apart and the duplication is named here rather than hidden.

### 2. The build allocates its own targets, and the render thread adopts them

`ChainTargets::build` makes the entry target, the ping-pong targets, the retention history and one
bind group per slot, taking a device and a `ChainWorkshop` — the two bind group layouts, the
sampler, the view the `exit` cut is held from, and the size — and **nothing of the caller's**. It
is the one derivation of those textures: `MasterChain::allocate` is that function called against
the chain's own workshop, so the synchronous path and the worker path cannot drift.

The workshop is taken from the `Present` when the build is asked for, which fixes the size the
targets are made at. A resize in between is therefore detectable rather than silent: the install
compares the two sizes, retires the stale targets unused, and allocates on the render thread that
once. A resize is already a reallocation of every frame-sized texture in the program, so paying for
one more there is not a new cost; a wrong-sized chain drawing would have been a wrong picture.

### 3. Retirement is the worker's, in both directions

`Present::set_chain` returns a `RetiredChain` — the outgoing slots with their pipelines and uniform
buffers, and the targets they ran through — and is `#[must_use]`, so dropping it on the render
thread is a thing a reader can see rather than the default. `ChainSwap` puts it in a graveyard by
`try_lock` and `karakuri-chain` drains and drops it. A build that a newer one superseded is retired
the same way and announces nothing.

The worker submits and polls before handing a chain over, for ADR-0033's second reason: **work moved
off the render thread is not off it until its side effects are too.**

### 4. Between the press and the install, the old chain draws

Nothing waits. `ChainSwap::begin_frame` is a `try_recv` loop that installs the newest finished
build and retires the rest, and a frame on which nothing is finished draws the chain that is
running ([P-0094](../principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)).

**The console is told nothing new until the install lands, and that needed no new code.** The
Master bay's rows are `chain_view(&engine.present, …)` written every frame from
`Present::chain_reading` — the reading rather than the state, which is
[ADR-0156](0156-the-consoles-arrangement-is-a-tree-this-repository-owns.md) — so a bay drawn between the
press and the install draws the list that is on air, which is the list the picture is of. What an
operator does not get is a *pending* indication, and that is named in *Consequences*.

### 5. The two real-time hosts ask; the offline renderer builds where it stands

`mix::apply_chain` is the asynchronous entry and is what `karakuri/src/app/handler.rs` and
`karakuri-cli/src/live/mod.rs` call. It keeps the cheap path unchanged — a list whose *shape* is the
shape already running is `Present::set_chain_params`, a `queue.write_buffer` per slot, applied on
the calling thread and allowed ([P-0091](../principles/0091-cost-is-known-before-it-is-paid.md)) —
resolves and checks the sources, which touch no device, and hands `ChainSlot`s to the worker.
Asking again for a list already being built is a no-op, so a host may ask on every frame.

`mix::install_chain` is the synchronous path and `karakuri-environment/src/render.rs` is its one
caller. It is `HotSwap::install`'s counterpart exactly: an offscreen run advances by whole steps
with no clock anywhere in it, so there is no frame for a build to be late for, and carrying a worker
would buy a replay nothing. A source scan asserts which file calls which, because nothing in a type
can say it.

### 6. The clock and the charge are unchanged

[ADR-0349](0349-the-chain-is-the-frames-so-it-reads-the-session-clock-and-is-charged-against-the-frames-budget.md)
made `frame::compose` the one writer of both `Present::set_chain_clock` and
`Deck::set_chain_ops_per_fragment`, every frame, from the `Present`. An installed chain is therefore
fed both on the first frame it draws with no host mentioning either, and installing before `compose`
is what makes that true. `MasterChain::install_targets` writes each slot's whole uniform block with
the clock that is running, so a chain installed mid-session starts at the session's `t` rather than
at zero.

## Alternatives rejected

**One worker shared with `karakuri-build`, as a second `Source` on the same loop.** One thread
fewer, one graveyard fewer, and the plumbing already exists. Rejected on ADR-0255 as above: a press
on a Master row would queue behind a Set build and its probe, and the operator's wait would be a
function of someone else's capacity declaration. The `Source` trait also polls for *one* kind of
work and returns `Polled`, so a second kind means a second variant, a second `Done` variant and a
branch in `run_worker` for the measurement step — changes to the file a Set's build lives in, for a
chain's sake.

**Compile on the worker and allocate the targets at install.** Much the smaller change: `Slot::build`
is the expensive half and the textures are four at most. Rejected because ADR-0033 is about
allocation as much as compilation, and four frame-sized `Rgba16Float` textures at 3840x2160 is 127
MB of allocation on the render thread. Having gone to a worker at all, leaving the allocation behind
would be a rule kept in the letter.

**Block the render thread on the build for one frame, as a load does.** A chain is small and a
single dropped frame on a press is arguably acceptable. Rejected because it is the argument that
loses every time it is made: P-0091 does not have a size threshold, and the frame it drops is the
frame the operator was watching to see what the press did.

**Keep `apply_chain` synchronous and give it a worker only in the GUI.** The CLI's live loop is the
same thread with the same frame, so the invariant would be true of one host and not the other, and
the source scan that enforces it could not be written.

## Consequences

- **A press takes effect a frame or more later**, and how many is how long three shader
  compiles take. The chain that is on air keeps drawing until then.
- **Nothing on the console says a build is in flight.** The Master bay draws the running list, which
  is honest about the picture and silent about the press. A pending row — the Library's `plan` badge
  or a spinner on the row that was pressed — is owed and is not bought here; `ChainSwap::building`
  is the reading it would be drawn from and exists.
- **A slot that refuses is reported a frame later and through an event**, where it used to be the
  return value of the call. `mix::apply_chain` still refuses synchronously for an address nothing
  holds or a source that does not check — neither needs a device — but `SlotError::NotL5` and its
  three siblings arrive as `ChainEvent::Refused` on the host's next frame boundary.
- **A resize during a build costs one render-thread allocation.** Named in §2, bounded, and it
  happens on a frame that is reallocating everything anyway.
- **`Present::set_chain` is `#[must_use]`**, so every caller now says where the outgoing chain is
  freed. In a test that is `drop(...)` on the spot and correct.
- **Two workers now exist with the same shape and no shared code.** §1 says why; if a third appears,
  the extraction is worth making and this record is where the count started.

## Evidence

Session 2026-09-14. `cargo test -p karakuri-engine --test master --test chain_swap` on a Metal
host, including the two defect injections that made
`a_chain_built_on_the_worker_draws_what_building_it_here_draws` and
`the_newest_of_two_pending_builds_wins` fail before they passed.
