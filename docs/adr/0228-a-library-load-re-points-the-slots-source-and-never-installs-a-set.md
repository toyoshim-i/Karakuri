---
id: 0228
title: A library load re-points the slot's source and never installs a Set
status: accepted
date: 2026-08-30
supersedes: []
superseded_by: []
principles: [0085, 0091, 0094]
tags: [engine, console, store, hot-swap]
---

# A library load re-points the slot's source and never installs a Set

## Context

The Library bay has listed the store's Sets in every run since it landed and could not put one on a
deck. `13e27ec` closed that, and this record is the route it took, because the route somebody will
re-propose is the other one.

**The gap was named and closed the same day.**
[ADR-0226](0226-m5-closes-when-the-manual-is-implemented-and-the-meter-is-progress-rather-than-completion.md)
wrote it that morning — *"Not one of the seven names that gap. Finish all seven and the library still
cannot load"* — and carries a parenthesis saying it closed, *"and the closing is the argument rather
than a correction to it"*. The same survey pass put it in [roadmap.md](../roadmap.md) twice, as
*"nothing in this workspace loads a Set into a running deck"* and as a `plan` badge that could not
say why. What is older than the survey is the sentence the survey was quoting, and it is the whole
decision.

**`Deck::install`'s own documentation is the argument** (`crates/karakuri-engine/src/deck.rs`). It is
the one function that puts a built Set in a slot, and it says of itself:

> **A replay's only way to follow a `procedure` record**, and deliberately not reachable from a key
> or a surface: a live run changes its material by editing a file and letting the worker build it,
> which is what the budget watchdog is attached to. This is the other end of that — reading back what
> a run already did.

So the missing piece was never a way to *install* a Set. It was a way to **re-point a slot's source**:
a loaded Set compiled on the build worker, swapped at a frame boundary, judged against the budget and
rolled back on its own if it costs too much, exactly as an edited file is
([P-0094](../principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)).
Anything that handed a built Set straight into a slot would put back the hole that comment describes —
a live change with no watchdog attached to it.

## Decision

**A load says *look at these files instead* and lets go.** `crates/karakuri`'s `loading` writes the
library Set's procedures into the scratch and sends a `watch::Aim`; it touches no deck, and its own
comment says what that buys: *"Everything after this line is the path a save already takes"*. The
library gets the trial, the verdict and the rollback for nothing, and no second route into a slot is
opened —
[P-0085](../principles/0085-take-the-mechanism-that-exists-and-pay-the-bill-now.md), where the mechanism that
already exists is the file watcher.

Both halves of the mechanism below were **forced by the code rather than chosen**, which is why they
are recorded here: a reader who thinks either one was taste will simplify it away.

### 1. It arrives over a channel, because the `Source` is on the worker

`HotSwap::new` (`crates/karakuri-engine/src/swap.rs`) takes `source: Box<dyn Source>` and **moves**
it: the box goes into the closure the worker thread is spawned with, and `run_worker(device, queue,
source, …)` owns it from there. Its own line is *"`source` is polled on the worker thread"*. In
`crates/karakuri`'s `watched` the `watch::Watch` is built inline as an argument to that call, so the
render thread never holds one at all — there is nothing on this side to call a setter on.

So a re-point is a `std::sync::mpsc::Sender<watch::Aim>` — `watch::Aim` and `Watch::aimed_by` in
`crates/karakuri-environment/src/watch.rs`, with the receiver installed at construction beside the
watcher it belongs to. The `aimed` field says why a channel is right rather than merely necessary:

> A channel rather than a shared cell for the reason this module polls rather than taking `notify`:
> the worker is a poll loop already, so there is nothing to wake and nothing to lock. It is also what
> makes a re-point *safe* to offer at all — the render thread hands over a description and touches
> nothing this watcher owns, so a load costs the frame it happens on a `send` and no more.

**The aim is every field `Watch::new` takes, less the slot.** `Watch::repointed` destructures it with
**no `..`**, and the comment there is that the compiler is the check: a field left behind comes back
as the outgoing slot's layering, fold, capacity, camera or salts, and *"it would show up on the first
save after the load rather than on the load, which is the hardest version of it to find"*. The slot is
the one thing an aim cannot carry, because a watcher that changed slots would rebuild somebody else's
deck. `crates/karakuri`'s `loading` therefore reads all of it off the Set file; the authorities are
the deliberate exception and are empty, because `Record::Authority` is not Set-file state and a load
starts a slot with the grants `--load-set` gives one, which is none.

**Drained to the end, and the newest is taken.** `Watch::repointed` loops `try_recv` until the channel
is empty and keeps only the last aim: *"Two loads pressed inside one poll interval are one load as far
as the picture is concerned — building the first of them would put a Set on screen that the operator
has already replaced, and it would cost a compile to do it."* A closed channel reads as nothing
waiting, and `None` is a watcher nobody can re-point — every offscreen path and every slot
`karakuri-cli` builds.

**And it is deliberately not debounced.** `Source::poll` checks `repointed()` *before* the file
stamps, and returns a build on the poll the aim is seen. The debounce exists because an editor writing
in place leaves a file truncated for a moment; an aim has no such moment, since whoever sent it wrote
every file before the `send`. Waiting a second interval would only make a load slower than a save for
no reason. The re-point re-seeds `self.stamps` from the new files so the next poll does not read its
own writes as an edit, and `watch.rs`'s
`an_aim_points_the_slot_at_what_it_names_and_is_not_debounced` asserts both halves.

### 2. It writes files first, because a library Set has none

`Watch` reads **paths** and never a store, so a Set whose sources are content-addressed blobs cannot
be aimed at directly. `loading` writes each procedure out with
`karakuri_environment::scratch::place` and aims at the results — which is exactly what `--load-set`
does at startup, on the same watcher (`crates/karakuri-cli/src/main.rs` calls
`scratch::place(&root, &checked.name, src)` for the same reason).

**The scratch name carries the deck and the node's place**, `A0-drift.kir`, and that is not decoration:
`scratch::place` writes `<store>/scratch/<name>.kir` and overwrites what is there, so two decks loading
Sets whose procedures happen to share a name would silently become one file — the second load moving
the first deck on its watcher's next poll, with nothing to say why. `--load-set` does not need this
because it fills one slot before the run starts.

**The node name is the Set file's and not the procedure's**, which is `--load-set`'s own pairing: an
`edge` in the file resolves against the name the file wrote, so a rebuild that called a node whatever
its procedure declares would break the slot on its first save.

### 3. The Staging lane shows the result, and nothing was added to it

A load reaches the slot's `HotSwap` through the same worker a save does and therefore emits the same
`swap::Event`s. `crates/karakuri`'s `staging` drains `Deck::events` per slot and maps
`Swapped`/`Rejected`/`RolledBack` to a row and `Accepted` to a row leaving the lane; it neither knows
nor can know which of the two put the candidate there. The producer the lane was gated on is the
watcher, wired earlier the same day in `c6db997` — before that the program built both slots with `HotSwap::fixed`,
whose sender is dropped at construction, so not one `swap::Event` of any variant was ever emitted and
the lane could reach no state but empty.

The budget is real for the same reason: `watched` passes `DEFAULT_BUDGET_MS`, and its comment records
that `HotSwap::fixed` *"judged against infinity, so no candidate could ever be thrown out for cost"*.

## Alternatives

### a. Reach `Deck::install` from the surface

**The obvious shape, and the one the survey's wording pointed at**: `Deck::install` is unreachable
from a key or a surface, so make it reachable and the library can load. Both roadmap entries name that
unreachability as the blocker, and a reader who stops at the first half of the sentence will re-propose
this on sight.

It loses to the second half of that same sentence, and to the machinery it names. `HotSwap` exists to
guarantee two things about anything that reaches the screen — that it was **measured**, and that there
is a **previous Set parked** to put back. A Set built on the render thread and installed straight into
a slot has neither: no trial, no budget verdict, no rollback. `Watch::aimed_by`'s documentation states
it as the refusal it is:

> A surface that built a Set and handed it over would be putting material on air that **nothing
> measured**, in a slot with no previous Set parked to roll back to — the two things `HotSwap` exists
> to guarantee.

The failure is also the worst-timed one available: an expensive Set loaded mid-set takes the show down
and leaves nothing to put back, which is precisely the case the watchdog and P-0094 were written for.
`install` keeps its one caller — a replay following a `procedure` record, *"reading back what a run
already did"* — and this decision does not widen it.

### b. A setter on the `Watch`

`watch.set_head(…)` from the render thread, and no channel at all. **This one is refused by the
compiler rather than by an argument**, and the distinction is worth keeping: there is no design here
to weigh, and no amount of care makes it available. `HotSwap::new` moved the boxed `Source` onto the
worker thread; `crates/karakuri`'s `watched` constructs the `Watch` as a temporary inside that call, so
nothing on the render thread has a handle to it and nothing can be given one without a lock around the
whole watcher. That lock would be the shared cell the `aimed` field's comment declines — contention
with a thread that is compiling, in exchange for a shape whose only advantage was looking shorter.

Recorded because a reader who knows only that *the re-point is a channel* may read the channel as
ceremony and try to delete it.

### c. Aim the watcher at the store's artifact files

Halfway plausible, because the premise is nearly true: an artifact **is** a file —
`Store::artifact_path` is `<root>/<hash>.kir` — so a watcher that reads paths could be pointed at one
and the scratch write skipped.

It loses to what a content address means. `Store::put_artifact` is documented as never rewriting what
is on disk: *"the artifact already on disk is never rewritten, so it can never be corrupted by a
concurrent or repeated `put`"*. A slot pointed at one is a slot whose material cannot be edited — and
being editable is the entire point of a re-point, since the mechanism being reused is *the file
watcher*. It also walks straight into what `scratch.rs` was created to stop: before the scratch
existed, an editing surface was given write access to whatever path the material came from, and
*"three shipped presets were replaced in one session"*. The scratch is the one writable place of the
three that module names, and a load that skipped it would make the store's own blobs the fourth.
`scratch::unique_name` adds the smaller half of the same answer: *"a scratch of hashes is a scratch
nobody opens in an editor."*

## Consequences

- **The load is reachable from one key and one cursor.** `l` over the Library bay, with the two
  operands already on screen — the cursor says which Set and the deck selection says which deck — and
  the press emits `Operation::LoadSet`, which `crates/karakuri`'s `played` performs. `LoadSet` writes
  no record, so the surface that names it is the surface that performs it; deck selection is the seam
  that fills `deck` in, and `0` `1` `2` `3` mean the same thing here as at the command line because a
  deck is a slot number.
- **The strip's name is written on the aim, not on the verdict.** A build may still be refused or
  rolled back, and a name that waited for the verdict would leave the strip naming material that is
  no longer in the file. Which of the three happened is the Staging lane's answer, on the deck it
  happened to, and a hedging strip name would be a second quieter answer to the question that lane is
  about.
- **Every failure leaves the deck as it was, in a sentence.** A store that will not open, a Set that is
  not there, a procedure that no longer checks, a scratch that will not be written, or a worker that
  has gone: nothing is sent and what is playing goes on playing.
- **The drain runs on a frame, so a fully folded window defers it.** `staging` is called from the frame
  handler, and `live` — *"is anything making texels this frame"* — is what decides whether the loop
  asks for another frame. With the picture and every preview cell folded away no frame is requested,
  so the verdicts sit on the channel until something else asks for one. **This is not new and not the
  load's**: a save reaches the lane the same way and waits the same way, and the still-panel reading
  ([ADR-0164](0164-the-panel-is-budgeted-rather-than-forbidden-to-allocate.md))
  measures 0 frames a second in exactly that state on purpose.

### What this leaves undone

- **The listing is read once, at startup.** `library` is called from `resumed` before the first frame,
  so a Set saved by another program while this window is up does not appear in the bay until the next
  run — and since the key reads `view.library.get(at)`, a Set that is not listed cannot be loaded
  either. The limit is the listing and not the load: `loading` opens the store itself and would find
  it. Closing this wants **a reason to re-read rather than a timer**, which is a decision of its own —
  `arrangements` has that reason, since the only thing that can add a name to it is a save this
  program performs.
- **Neither `Watch::storing_to` nor `Watch::snapshotting_to` is wired here.** A loaded Set's build
  reaches the screen and nowhere else: nothing is put under a content address, so no node address can
  be derived, and no version is kept, so nothing can be walked back. Those two are what the Staging
  lane's own operations — *keep the candidate* and *put a node's previous version back* — are waiting
  on, and they are unchanged by this record.
- **A Set that recorded two different capacities loses the second.** The aim carries one
  `Option<u32>` for the slot, taken from the first geometry's recorded capacity, because that is
  `Watch`'s shape; each geometry is otherwise rebuilt at the capacity its own file declares. A limit
  this program **shares with `--load-set`** rather than one it invented.
- **A load hands over no published controls and no grants.** `setfile` writes and reads no published
  control, so the aim's list is empty for the same reason this program's startup watchers pass an
  empty one; the authorities are empty by the decision above, not by omission.
- **Nothing removes scratch files a previous load left.** `place` writes the nodes of the load it is
  performing, so a shorter Set loaded onto a deck that held a longer one leaves the surplus
  `<letter><n>-*.kir` on disk. They are unpointed-at rather than read, and no watcher will build them.
