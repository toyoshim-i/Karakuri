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

**The first clause is not reachable on the example as it opens** — and it is **four** folds away
now, which was three between 2026-08-26 and the beat declaring on 2026-08-28 — and that is a fact
about what the panel holds rather than about this rule.
`crates/karakuri/src/main.rs` opens a window with a live picture in the Program bay,
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

**That zero now takes a fourth fold, and this sentence is derived rather than re-measured.** The
beat grid declares for as long as it is drawn
([ADR-0212](../adr/0212-the-beat-is-a-light-that-travels-and-it-declares-for-itself.md)), so with
the three folds above the window goes on drawing at `view::BEAT_STALENESS` — 24.67 ms, about forty a
second — and the transport row has to be folded away as well before nothing on this panel is
declaring. The three-fold reading above is what this example measured on 2026-08-26 and stands as
that; the fourth fold has not been measured, and `examples/panel.rs` now says which of the two
declarations its arm is looking at.

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

**Two regions declare both numbers, and there is still no scheduler.**
[`View::declares`](../../crates/karakuri-console/src/view.rs) answers with a region's name, a
**cost** and a **staleness**, and `repaint::Change::Animating` turns the soonest staleness into the
deadline the window waits on.

**The transport row** declares whenever the beat grid is drawn — a light travelling the grid once a
bar, which is
[P-0077](0077-continuous-motion-is-how-a-stopped-panel-announces-itself.md)'s continuous motion and
is the one declaration on this panel that **does not ask whether anything is pending**
([ADR-0212](../adr/0212-the-beat-is-a-light-that-travels-and-it-declares-for-itself.md)). Its
staleness is `view::BEAT_STALENESS`, one beat at the mock's 128.0 BPM in as many steps as the travel
has pixels: **24.67 ms, about forty a second**.

**The mixer bay** declares while anything in it is pending *and* the bay is laid out. Three
presentations share that one declaration, because the unit is a region — the tally's word
rolling toward a residency the governor has not granted, and a reach on each of a strip's two faders
while a transition has not run
([ADR-0190](../adr/0190-the-parked-tally-rolls-because-two-lamps-do-not-fit-in-fifty-three-pixels.md),
[ADR-0206](../adr/0206-a-fader-marks-where-it-is-going-and-keeps-reaching-for-it.md),
[P-0075](0075-a-pending-transition-shows-where-it-is-where-it-is-going-and-that-it-has-not-arrived.md)).
The staleness is `view::ROLL_STALENESS` — `ROLL_TRAVEL` in `ROLL_STEPS` steps, 33.33 ms, about
thirty a second. The cost is
[`budget::PANEL_PASS`](../../crates/karakuri-console/src/budget.rs), **1.26 ms**, and it is one
constant every region declares rather than a figure of its own: `egui` is immediate mode, so there
is no way to redraw one region of this panel and a per-region number would describe work this
console cannot do — a deliberate over-declaration a scheduler can refine downwards, which is what
[ADR-0210](../adr/0210-a-declared-cost-is-one-panel-pass-written-down-and-held-against-the-run.md)
decided along with how a written-down cost is kept honest.

**Both conditions are checked, and this is what they read.**
[`tests/schedulable.rs`](../../crates/karakuri-console/tests/schedulable.rs) sums over whatever the
view declares, at the most this console can ever declare — an engine behind it, every strip pending
in all three ways, every region laid out. `Σ (cost / staleness)` is **0.0889** against a
`budget / frame interval` of **1.0** — 0.0511 for the beat and 0.0378 for the roll, and the first
frame on which that sum has had two terms — and `max(cost)` is **1.26 ms** against **4.17 ms**, a
quarter of the 16.6 ms a frame has to fit in. The second is still one number, because both regions
declare the same whole panel pass.

**The budget is the whole frame's**, because the console has never written down a panel's
share of one — so the conditions hold in their most permissive form, and what they cannot catch is a
panel that fits the frame while leaving the engine nothing. A folded region is in neither sum
([ADR-0193](../adr/0193-a-region-that-is-not-laid-out-declares-nothing-rather-than-being-dropped-later.md)),
which is asserted rather than assumed, both ways in.

**What is still absent is the policy rather than the inputs, and the reason has changed.** It used
to be that choosing between one region and nothing is an abstraction with one call site. The second
declaring region has arrived and it is the beat, as P-0077 wanted — and still nothing arbitrates,
because **both fit**: at 0.0889 of a frame the frame that meets the sooner deadline meets the other
one as well, and `View::animating`'s *soonest staleness* is the whole policy. A scheduler is what a
budget that cannot afford everything needs, and the first thing one under pressure will offer is the
beat, which is exactly what P-0077 forbids.

**What a drawn frame costs.** A median of **525 allocations and 694.3 kB** in the middle of those
nine runs, the nine spread 524 to 538 and 671.4 to 695.3 kB — with everything above on the panel.
**Re-taken on 2026-08-28 over ten runs**, with the faders' reaches since added: **524 allocations in
every one of them**, 55.3 to 58.7 frames a second in nine and 37.0 in the tenth, and 19.6 to 26.3%
of a second drawing in those nine against 14.3% in the tenth. The allocation count did not move and the milliseconds did, which is the
machine rather than the panel and is exactly the swing the paragraphs above warn about.

**Taken once more on 2026-08-28, with the beat declaring**: **535 allocations** and 673.9 kB a
frame, 58.3 frames a second, 20.1% of a second drawing, and the panel-draw median at **0.997 ms**
against the 1.260 `PANEL_PASS` declares — both inside the 2x band `examples/panel.rs` holds its own
figures to, and 535 inside the 524 to 538 the nine runs of 2026-08-26 spread. **One run rather than
a batch**, so read the ten allocations as a direction and not a finding: the likeliest place for
them is the second halo the travelling light draws on the frames it is between two dots
([ADR-0212](../adr/0212-the-beat-is-a-light-that-travels-and-it-declares-for-itself.md)), and one
reading cannot tell that from the swing this paragraph has already been caught by twice.
[ADR-0164](../adr/0164-the-panel-is-budgeted-rather-than-forbidden-to-allocate.md)'s **184 and
226.2 kB** is not that number's predecessor in any comparable sense: it was a mean over 180 frames
of an empty panel driven by a loop that always drew, it is still true of the panel it measured, and
the example that quotes it now holds the quoted figure against the run it has just taken rather
than repeating it. The per-frame price has never been what this rule is about — how many frames pay
it is.
