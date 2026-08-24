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

**Nowhere yet.** The panel exists and is drawn every frame at 0.268 ms median with every bay empty;
this states what it is being built to, before the bays that would make it expensive are written.
The reasoning, and the alternatives that lost, are
[ADR-0164](../adr/0164-the-panel-is-budgeted-rather-than-forbidden-to-allocate.md).
