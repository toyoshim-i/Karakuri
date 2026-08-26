# A surface owns the affordance, never the authority

A control decides **what a press asks for**. It never decides **what may be asked**. A constraint on
what the instrument will accept lives where the record is applied, so every way in meets the same
wall.

The proposal this rules out is a reasonable one and it will be made again: *while a transition is
pending, the panel should forbid every operation except the one that withdraws it.* The constraint
is often real. **Where it lives is what this settles**, and putting it in the surface that drew the
control is what loses.

## A rule that binds one surface binds none

MIDI, the keyboard and MCP do not pass through the console. A lock held in the panel makes the
instrument behave differently depending on which hand touched it — a pad does what a chip refuses —
which is precisely the divergence *one vocabulary, four ways in* exists to end. The operator's model
of the machine then has a hole in it that only shows up mid-set, and the manual already names that
failure in another context — `docs/manual/console.html`, on a control that has to explain itself:
**“a control that quietly declines the last of something is a rule an operator can only find by
experiment.”** A lock held by one surface is that sentence with a second clause: *and only from one
of the four ways in.*

It is also the cheapest rule in the system to get wrong, because a second surface that never
implements the lock does not *fail*. It just works, differently, for a year.

## And the one thing that must never be refused would be

*Go on air* is the request that always arrives. The governor never takes a Live slot off air
([P-0033](0033-the-governor-never-takes-a-live-slot-off-air.md), which is narrower than it is often
remembered as — it forbids **automatic demotion**, and says nothing about granting a request); the
half that completes it is `crates/karakuri-engine/src/deck.rs`'s module doc, where *effective Live
and requested Live are the same set of slots, since nothing demotes a Live slot and nothing promotes
into Live.* A Live request lands, always.

A console that blocked a deck from going live because a *prime* request on some slot was still
parked would refuse, in a live instrument, the one operation that cannot be refused — and it would
do it from a rule the engine has never heard of. That is not a bug in the lock's condition; it is
what a lock in the wrong place is for.

## Where a constraint is real, it goes where the record is applied

Then all four surfaces meet it, the refusal is one sentence from one function
([P-0061](0061-a-refusal-a-person-can-reach-from-two-surfaces-is-one-sentence.md)), and the surface
**draws** the refusal rather than being the authority for it — greying a control, or explaining the
wall it just met, is a surface reflecting a rule it does not own.

The vocabulary already has refusals of this kind. `docs/manual/operations.html` records one against
*Wipe the next deck in*: **“Mask, opacity, blend and residency land immediately; the mask's travel
is scheduled. Refused with no shape chosen.”** That refusal belongs to the operation on every route
— panel, key, MIDI, MCP — and no surface gets to have its own version of it.

## What a surface does own is what a press asks for

That much is settled and this rule is the boundary on it rather than a repeat of it:
[P-0074](0074-an-operation-says-what-it-wants-never-which-way-to-move.md) gives the **affordance** to
whoever draws the control, and the vocabulary only ever sees a destination. The worked example is
the blend chip — its cycle is four lines in `karakuri-console` and nothing at all in
`karakuri-operation`
([ADR-0187](../adr/0187-the-blend-mini-cycles-and-a-map-learns-the-three-it-cycles-through.md)).

So the maintainer's instinct is available in full, on the right side of the line: **a press on a
control whose transition is pending may ask for the withdrawal.** That is a control choosing which
destination a press names, which is exactly what P-0074 hands it — an affordance, not a lock. It
does what the lock was wanted for, and it moves no authority: MIDI, the keyboard and MCP still name
whatever destination they name, and the engine still decides.

## What this adds that P-0061 and P-0028 do not

**P-0061 is about the *wording* of a refusal that already exists; P-0028 is about *values* ending in
one record; this is about where the *rule that produces a refusal* lives** — and neither of the
other two forbids a surface from inventing one of its own.

## Where it holds

**Nowhere yet, because nothing has tried it.** No console control refuses anything today: the four
that answer a pointer each name a destination and hand it over. This is written down now because
the first control with a pending state is the first control anyone would put a lock on.

Decided in
[ADR-0188](../adr/0188-a-pending-transition-says-it-is-pending-and-no-surface-holds-the-rule.md), which records
the console-held constraint as a rejected alternative with both arguments, beside
[P-0075](0075-a-pending-transition-shows-where-it-is-where-it-is-going-and-that-it-has-not-arrived.md), which is
what a pending state has to say.
