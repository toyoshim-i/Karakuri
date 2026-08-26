# A still panel costs nothing, and what moves declares its price

**A panel with nothing changing on it is paid for once and not again.** No per-frame work is done
to redraw what nobody has touched and nothing has moved.

**What must be live during a performance is named, and each named thing declares two numbers**: what
its update costs, and **how stale it may get, in milliseconds**. Not in frames — a frame stops being
a unit of time the moment the rate drops, and a tolerance stated in frames silently loosens by the
same factor, which is to say it becomes most permissive exactly when the machine is most loaded.

**Everything else is scheduled into what is left**, by how stale it is against what it can afford.
That is aging without a second mechanism: the longer something waits, the more urgent it becomes.
A region is redrawn whole or not at all, so **the unit of deferral is a region and never a slice of
time**.

**What the operator does costs what it costs.** Dragging a divider, folding a pane, opening a
library — these are the operator's own load, they are transient, and they are not budgeted. The
budget exists for the frames nobody is touching.

## Two conditions, and both are arithmetic

A panel is schedulable only if both hold. They are sums over the named regions, so a test asserts
them rather than a stage discovering them.

    Σ (cost / staleness)  ≤  budget / frame interval
    max(cost)             ≤  a small part of the budget

The first says there is enough capacity on average. **The second is the one that gets forgotten**:
an update is not divisible, so a region costing most of the budget is a traffic jam of one — every
frame it runs, nothing else can, and the readouts that had to be live miss their deadlines. A
region that cannot meet it is split, or its unchanging part is drawn once into a texture and
composited after.

## Phase is spread by the budget, not by a mechanism

Regions sharing a tolerance come due together, and worse, they *start* together — draw everything
on the first frame and they are in phase for the rest of the run. The budget prevents that by
itself, as long as it is applied to the first frame too: what does not fit is carried, and the phase
it lands on is the phase it keeps. Where two regions still align, a deterministic offset separates
them — deterministic, so that the same state gives the same schedule and the panel has no jitter
anybody has to explain.

## The program view is not GUI cost

What is inside it is the engine's output, already accounted for by the governor, and counting it
here would count it twice. **What does land on this budget is the composite** — putting that
texture into the panel — which is small and constant and is not nothing. It is drawn every frame at
the highest priority regardless: an operator reads the picture for timing, and a preview that
stutters is not a slow preview, it is a broken clock.

## What this rules out

A toolkit pass run on a frame where nothing changed. A cost that is paid every frame and named
nowhere. A tolerance stated in frames. A region whose price is a constant while what it draws grows
with its contents. And rate-limiting the panel as a way of affording it — halving the rate does not
remove the cost, it clumps it, and a periodic hitch is more visible than a constant one.

## Where it holds

**The first clause is not reachable on the example as it opens** — though it is three folds away,
which it was not before 2026-08-26 — and that is a fact about what the panel holds rather than about
this rule.
`crates/karakuri-console/examples/panel.rs` opens a window with a live picture in the Program bay,
deck A auditioning in the preview row under it, a mixer bay, a transport, an outputs row, and deck
B parked by the governor with its tally rolling once a second. Nothing on it is still. Over three
seconds with nothing touching that window it draws **58.3 to 59.3 frames a second and spends 18.1
to 22.2% of a second drawing** — nine runs on 2026-08-26, an Apple M4 Pro at 1440x900 logical,
host clock, debug profile with dependencies at opt-level 3. Read that as an order of magnitude: a
reading taken an hour earlier, at the commit before the parked deck landed, put the same window at
37.1% on this machine in another power state, which is the swing the example's own last paragraph
warns about.

**Folding gets back to zero, in three folds, and that was measured rather than reasoned** — the
folds applied at startup through the same `Op::Fold` the `f` key sends, on a temporary build,
because a synthesised keystroke needs an Accessibility grant this process has not got. Fold the
picture away and deck A goes on auditioning underneath at 47.0 frames a second (one run). Fold the
preview row as well and nothing in the Program bay is making texels, and the window still draws
**28.0 to 28.3 a second at 427 and 425 allocations a frame** (two runs on 2026-08-26) — the roll's
declared 30 Hz, arriving as the window's deadline, and the mixer bay is on screen, so the chip that
declares it is being drawn. **Fold the mixer bay on top of that and the window draws nothing at
all**: 0 frames, 0 allocations and 0 bytes over three seconds, in both of two runs, and the reading
prints *"so P-0072's first clause holds here"*.

**It did not, and that was this file's own measurement of a defect.** The same three folds used to
leave the rate exactly where two folds had it — 28.7 and 29.0 a second, re-taken on 2026-08-26 with
the fix backed out, at 259 and 261 allocations a frame against 427 with the bay drawn. Only the
price per frame moved, because the fold hid the chip while the governor's parked slot stayed parked
and [`View::animating`](../../crates/karakuri-console/src/view.rs) answered off the deck rather than
off what was on screen. It now answers off both, and a region that is not laid out declares nothing
— [ADR-0193](../adr/0193-a-region-that-is-not-laid-out-declares-nothing-rather-than-being-dropped-later.md),
which is [P-0073](0073-a-node-claims-only-what-its-visible-content-can-use.md) stated in time rather
than in space. **Zero still needs a console with nothing pending *that is drawn*,** and
[`tests/parked.rs`](../../crates/karakuri-console/tests/parked.rs) asserts both halves headlessly:
nothing pending, `animating` answers `None` and the window is told `Never`; something pending in a
bay that is folded away, the same, and unfolding it declares again. The decision that stops
a frame being drawn at all is [`repaint.rs`](../../crates/karakuri-console/src/repaint.rs), one
closed list of everything that can change what the console shows —
[ADR-0165](../adr/0165-the-repaint-decision-is-one-closed-list.md).

**One region declares, and there is still no scheduler.** The mixer strip's residency chip is the
only client: `View::animating` returns `view::ROLL_STALENESS` — `ROLL_TRAVEL` in `ROLL_STEPS`
steps, 33.33 ms, about thirty a second — while any strip carries a request the engine has not
granted **and the bay that draws the chip is laid out**, and `repaint::Change::Animating` turns that
into the deadline the window waits on
([ADR-0190](../adr/0190-the-parked-tally-rolls-because-two-lamps-do-not-fit-in-fifty-three-pixels.md),
[P-0075](0075-a-pending-transition-shows-where-it-is-where-it-is-going-and-that-it-has-not-arrived.md)).
That is the *declaring* half of this file, with one caller. **Everything downstream of the
declaration is still absent**: no region declares a **cost** — ADR-0190 measured what the roll costs
rather than the region announcing it — nothing arbitrates between two regions, and neither
schedulability condition above is checked anywhere. They arrive with the second declaring region,
which [P-0077](0077-continuous-motion-is-how-a-stopped-panel-announces-itself.md) wants to be the
beat.

**What a drawn frame costs.** A median of **525 allocations and 694.3 kB** in the middle of those
nine runs, the nine spread 524 to 538 and 671.4 to 695.3 kB — with everything above on the panel.
[ADR-0164](../adr/0164-the-panel-is-budgeted-rather-than-forbidden-to-allocate.md)'s **184 and
226.2 kB** is not that number's predecessor in any comparable sense: it was a mean over 180 frames
of an empty panel driven by a loop that always drew, it is still true of the panel it measured, and
the example that quotes it now holds the quoted figure against the run it has just taken rather
than repeating it. The per-frame price has never been what this rule is about — how many frames pay
it is.
