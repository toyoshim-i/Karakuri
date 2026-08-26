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
discrete flip.** The beat indicator today lights **exactly one dot** — `TransportRow::on` is
`Transport::beat`, and `.beat-grid i` is `var(--c-line)` where `.on` is `var(--c-pink)` with a glow
— so it is a 0/1 switch per dot, changing once a beat, two to four times a second at ordinary
tempos. That proves liveness **across an interval**: an observer knows the panel was alive between
two flips, and a stop is only apparent once a flip that was due does not arrive. A continuous
movement proves it **at every instant**, which is what this signal wants.

**This is a reason to prefer continuous motion. It is not a decision to change the beat.** The
maintainer wants the beat analogue eventually and that is where the two lines meet, but nothing here
settles what the indicator becomes, at what rate, or what it costs — and the discrete grid satisfies
the forced part today.

## Where it holds

**Nowhere yet, and one of the three reasons has gone.** There is still no scheduler to refuse the
economy, and the beat grid still moves because the panel redraws rather than because anything
decided it must. What has changed is the third: **one region now declares a cost and a staleness**
([P-0072](0072-a-still-panel-costs-nothing-and-what-moves-declares-its-price.md)) — the mixer
strip's residency chip, which rolls at a rate it names while a request of the operator's has not
landed
([ADR-0190](../adr/0190-the-parked-tally-rolls-because-two-lamps-do-not-fit-in-fifty-three-pixels.md)).

**That region is not this rule's motion**, and the distinction is the whole of why this file is
still waiting. The roll runs **only while something is pending**: a console with nothing parked is a
console with nothing moving on it, which is exactly what P-0072's first clause asks for and exactly
what this rule says is not enough. What proves the panel is alive has to move whether or not
anything is happening, and the only thing on this panel that does that is the beat — a discrete
flip, which satisfies the forced part and is not the continuous carrier this file prefers.

So the first candidate for the forced clause is still the beat indicator, and the roll is the first
evidence that the *declaration* half of P-0072 works. This is written **before** the scheduler
rather than after it: the rest of P-0072 is the next item in [roadmap.md](../roadmap.md)'s order of
work, and the first thing a budget under pressure offers is this saving.

Decided in
[ADR-0189](../adr/0189-motion-may-carry-the-meaning-and-a-stopped-animation-is-a-fault-to-report.md),
which is one decision with two consequences: a presentation may carry its meaning in motion
([P-0075](0075-a-pending-transition-shows-where-it-is-where-it-is-going-and-that-it-has-not-arrived.md)),
because a stopped panel is read as stopped — which is only true while something on it moves.
