# Continuous motion is how a stopped panel announces itself, and it is not spare budget

**A person does not read a stopped display as current.** A panel that has been moving regularly and
then stops says *something has gone wrong* to whoever is looking at it, and they know from that
moment not to trust what is on it. Nobody has to be taught this and no legend explains it.

So the panel needs **no convention for being frozen**. What it needs is **something that is always
moving**, because a stop is only visible against motion. On a panel where nothing moves, a panel
that has stopped and a panel that is idle are the same picture, and the operator finds out by
acting on a stale reading.

**That motion is a signal and not slack.** It is the cheapest thing the console has to say *this is
live*, it is read correctly by someone who has never opened this repository, and it is the only
statement that still works when whatever would otherwise report the fault is itself the thing that
has stopped.

## What it rules out

**A scheduler that buys frame budget by freezing the panel's continuous motion when things get
tight.** This is the economy anybody would reach for first — it is a real saving, it degrades
nothing an operator is reading *at that instant*, and it is exactly wrong: it sells the one signal
that says the frame is in trouble, at the moment the trouble starts. The panel then goes quiet
while it is failing, which is the shape
[P-0027](0027-a-silently-wrong-image-loses-to-a-loud-failure.md) and
[P-0030](0030-an-instrument-says-what-it-did.md) both refuse in their own domains: a plausible
still surface, with the failure suppressed.

Under [P-0072](0072-a-still-panel-costs-nothing-and-what-moves-declares-its-price.md) the
continuous region declares a cost and a staleness like anything else, and it is scheduled like
anything else. What it is not is **the first thing dropped**: a budget that cannot afford it is a
budget that is over, and the answer to over budget is that the panel is over budget rather than
that it looks calm.

It also rules out the converse economy — a panel designed so that *nothing* moves when nothing is
pending, on the argument that a still panel costs nothing. A still panel does cost nothing, and
that clause of P-0072 stands; what this rule adds is that at least one region is live for as long
as the console is claiming to show a live instrument, and that region is why the rest can be
trusted.

## What it asks of the presentation, and how much of that is preference

Marked as [P-0056](0056-mark-what-is-preference-so-it-can-be-revisited.md) asks: the forced part and
the chosen part, separately.

**Forced.** Something is moving continuously while the console is live, and a scheduler may not stop
it to make room.

**Preference rather than force: continuous movement is a better carrier of this signal than a
discrete flip.** A flip is a 0/1 switch — the beat grid used to light **exactly one dot**, changing
once a beat, two to four times a second at ordinary tempos. That proves liveness **across an
interval**: an observer knows the panel was alive between two flips, and a stop is only apparent
once a flip that was due does not arrive. A continuous movement proves it **at every instant**,
which is what this signal wants.

**It is a preference and it has been taken.** The beat grid is now a light that travels the grid
rather than a dot that switches — `TransportRow::at` is a position, `view::beat_at` says how much of
the light is on each dot, and the mock's one lit dot is a *frame* of that travel rather than a
different picture
([ADR-0212](../adr/0212-the-beat-is-a-light-that-travels-and-it-declares-for-itself.md)). It stays
marked as preference because it is what a future proposal would have to argue against: what the
indicator is, at what rate and at what cost is a decision that can be re-taken, where the forced
clause above cannot.

## Where it holds

**The forced clause holds, and the scheduler that would have to respect it does not exist.**

**What holds.** The beat grid is the panel's continuous motion and it moves on its own account: the
transport row declares a cost and a staleness — `budget::PANEL_PASS` and `view::BEAT_STALENESS`,
1.26 ms every 24.67, about forty a second — for as long as the grid is drawn, and
`repaint::Change::Animating` turns that into the deadline the window waits on. **It does not ask
whether anything is pending**, which is the sentence this rule is: a console with nothing parked and
no fade armed goes on moving, where before it went still and looked calm
([ADR-0212](../adr/0212-the-beat-is-a-light-that-travels-and-it-declares-for-itself.md)). And the
preference above is met with it — the light is somewhere on the grid at every instant, at most two
dots carry it, and the four of them always sum to exactly one dot's worth, so a still grid at half
brightness is not a picture the console can draw.

**What does not.** *A scheduler may not stop it to make room* is a rule with nothing to bind:
**there is no scheduler**. P-0072's arbitration is still unbuilt, and what stops this being urgent
is that both declaring regions fit — `Σ (cost / staleness)` is 0.0889 against 1.0, so nothing has to
be refused and nothing is choosing. The economy this rule forbids becomes available on the day
something *does* choose, and the day it does, this file is what it is checked against.

**And the motion is the operator's to fold away.** A folded transport row declares nothing
([ADR-0193](../adr/0193-a-region-that-is-not-laid-out-declares-nothing-rather-than-being-dropped-later.md)),
and so does a console with no engine behind it — which is every test in this crate. Neither is the
economy this rule forbids: what it rules out is the *panel* buying budget by stopping the motion,
not an operator hiding the row it is in. It is still a hole to know about — the panel has one
continuous region, and folding one row leaves it with none.

**One region, three presentations, and it is still not this rule's motion.** The mixer bay declares
too, and it runs **only while something is pending** — the tally's roll toward a residency the
governor has not granted, and a reach on each of a strip's two faders
([ADR-0190](../adr/0190-the-parked-tally-rolls-because-two-lamps-do-not-fit-in-fifty-three-pixels.md),
[ADR-0206](../adr/0206-a-fader-marks-where-it-is-going-and-keeps-reaching-for-it.md)). That is what
P-0072's first clause asks for and what this rule says is not enough on its own.

Decided in
[ADR-0189](../adr/0189-motion-may-carry-the-meaning-and-a-stopped-animation-is-a-fault-to-report.md),
which is one decision with two consequences: a presentation may carry its meaning in motion,
because a stopped panel is read as stopped — which is only true while something on it moves.
