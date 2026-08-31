# Nothing takes the show down, and nothing takes it away from the operator

This instrument is played in front of a room, and a performance cannot be paused while something is
fixed. So every mechanism that runs during one answers a question before it is built: **what does
this do at its worst, on the frame it goes wrong, while the operator's attention is on the room?**

Three answers carry a show through. A mechanism that can give none of them does not run during one.

1. **It cannot happen.** The hazard is refused or bounded at a moment where refusing is free — before
   the show, before the frame, outside the process. A procedure is priced by the check pass so an
   expensive mistake is *refused* rather than survived
   ([P-0067](0067-the-language-is-bounded-so-a-procedure-can-be-priced-before-it-runs.md)); the
   unbounded stall is moved off the frame path rather than made rare
   ([P-0001](0001-nothing-allocates-or-compiles-a-shader-on-the-render-thread.md)); the network, the
   secrets and the foreign binary stay outside, because their worst case cannot be bounded from
   inside ([P-0043](0043-nothing-external-enters-the-render-process.md)).
2. **It undoes itself, and it puts the show back where it was.** Where the hazard cannot be refused
   ahead of time, recovery is automatic and does not wait for anyone to notice: an over-budget Set
   rolls back **without anyone asking**, and the Set it replaced is parked unstepped so that it
   returns where it left off
   ([P-0005](0005-a-swap-happens-on-a-frame-boundary-and-an-over-budget-set-rolls-back-on-its-own.md)).
   A recovery that leaves a hole in the picture is not one.
3. **It is loud, and the operator's hands still work.** Where a hazard is neither refusable nor
   recoverable, it is *visible* and the controls for escaping it are intact. A plausible wrong image
   is converted back into a failure, and pulling a fader to zero has to work on exactly the material
   that is broken ([P-0027](0027-a-silently-wrong-image-loses-to-a-loud-failure.md)); a stopped panel
   is legible as stopped because something on it was moving
   ([P-0077](0077-continuous-motion-is-how-a-stopped-panel-announces-itself.md)); a deferred request
   says it has not arrived rather than reading as a discarded one
   ([P-0075](0075-a-pending-transition-shows-where-it-is-where-it-is-going-and-that-it-has-not-arrived.md)).

**Where more than one answer is available, take the earliest.** A refusal before the show costs
nothing; a rollback costs a swap the operator asked for; loudness costs their attention at the
moment they have least of it. This is the ordering P-0067 already argued in its own domain —
*estimate optimistically and rely on the runtime rollback* "moves a refusal that costs nothing into
one that costs a dropped frame in front of an audience" — and it is general.

**And the price is never the operator's.** Safety here is bought with design, with time, with budget
and with loudness. It is never bought with the operator's authority. The governor moves slots below
Live and warns, and *deciding to load four heavy Sets is the operator's*
([P-0033](0033-the-governor-never-takes-a-live-slot-off-air.md)); an automatic writer yields to a
hand on all four routes and never the other way round
([P-0078](0078-the-operator-wins-and-an-automatic-writer-yields-to-a-hand.md)); there is no switch
that hands the whole instrument to an agent (manual rule 06, and
[ADR-0211](../adr/0211-authority-is-set-per-node-and-the-record-is-the-sessions.md)).

## What it rules out

**A mechanism whose recovery plan is the operator noticing.** This is the shape, and it is
reasonable every time it is proposed: *it almost never happens, and if it does the operator can
restart it / pull the fader / press it again.* During a set the operator is looking at the room and
their hands are on four channel faders; their attention is the scarcest thing in the building and it
is not a recovery mechanism. A design that spends it is a design with no second answer, and the
first answer — make it impossible — was available and was not taken.

Concretely, it is what forbids each of these, and each has been reached for: compiling a pipeline
lazily on first use, because the unbounded stall then lands on whichever frame happens to be first;
leaving a bad Set on air until somebody sees it; a bound that reads a `param`, which turns a price
into a guess; `dlopen`ing third-party code into the render process during a show, where a panic
crossing FFI cannot be caught; treating an absent input as zero, which freezes the picture mid-set
instead of letting it free-run
([P-0034](0034-a-quiet-room-is-not-a-missing-microphone.md)); and a measurement that silently falls
back, because `0.0 ms` reads as a very fast shader and promotes what nobody has priced
([P-0012](0012-a-measurement-carries-how-it-was-taken.md),
[P-0037](0037-check-a-measurement-against-another-measurement.md)).

**And it rules out the mirror of that: buying the safety back from the operator.** A confirmation
before a live action, a lock while something is pending, a mode that will not let the show be put in
a state the machine dislikes, a "safe" setting that removes the control instead of costing it. Each
of these makes a show safer by making it less playable, which is the trade this rule exists to
refuse — and each of them fails on its own terms as well, because a rule held in one surface binds
none of the other three ([P-0076](0076-a-surface-owns-the-affordance-never-the-authority.md)).

**Not a licence for an automatic actor to be careful on the operator's behalf.** An automatic actor
that decides a request is unwise has taken the show away, which is the second half of the rule and
not a lesser half. The two failures are symmetric: a machine that stops is a lost show, and a
machine that will not do what it is told is the same lost show with a better excuse.

## Where the machine may refuse, and where it may not

**The instrument refuses what it cannot do. It never refuses what it judges unwise.**

A refusal at the door is one of the three answers and is the best of them: an over-budget procedure,
an ABI version mismatch, a wipe with no shape chosen. All three share the properties that make a
refusal safe — nothing that was on air changed, the diagnostic says *why* rather than only *what*
([P-0046](0046-a-diagnostic-says-why-not-only-what.md)), and every route meets the same wall in the
same sentence ([P-0061](0061-a-refusal-a-person-can-reach-from-two-surfaces-is-one-sentence.md)).
A refusal that leaves the show exactly where it was has cost the operator nothing but a message.

What is forbidden is refusing an operation the instrument *can* perform, on the ground that
performing it might go badly. *Go on air* is the canonical case: it is the request that always
arrives, and P-0076 already records what a console blocking it would be — "in a live instrument, the
one operation that cannot be refused". A dangerous thing an operator asked for is a dangerous thing
the operator gets, made as legible as this system can make it.

## What it settles

**MCP's scope.** Six tools — `read_procedure`, `write_procedure`, `swap_outcome`, `read_set`,
`list_sets`, `save_set`. `docs/manual/operations.html` states the shape of that and no reason for
it: *"A model can rewrite a whole procedure and cannot turn one knob … touches nothing in the mix,
the clock or what is on air."* The reason is this rule, applied per operation rather than per actor.
`write_procedure` has all three answers standing behind it: the source is priced before it is built
(P-0067), the swap happens at a frame boundary and rolls back on its own if it is over budget
(P-0005), and the outcome is reported (`swap_outcome`, which is why that tool exists). A model can
therefore write badly without taking the show down — ADR-0083 already noted that *"the safety device
already exists … built for hand editing, it works unchanged against a model."* A write to a mix
control has none of the three: it is unpriced, it lands immediately, it does not roll back, and it
changes what the audience is looking at. **So the line is not that a model is untrusted; it is that
those operations have no bounded worst case on any route, and the model is the route that reaches
them without a hand in the room.** That distinction is what makes the scope arguable, and it names
its own exit: the answer that would let a model turn a knob is not a bigger tool list, it is
per-node authority (rule 06, ADR-0211) — an operator granting a node, which puts a hand back in the
loop rather than removing the hazard.

**Annotated 2026-08-31, the evening of the day this was written.** The scope above is now a
**default** and not a scope, and the tools are seven rather than six — `wire_input` landed the same
morning. The maintainer's decision is that MCP is connected to every operation and that the classes
which could stop a performance are closed until an operator opens the one they belong to from its
bay's head, a call into a closed class being *refused* rather than hidden
([ADR-0235](../adr/0235-mcp-reaches-every-operation-and-what-could-stop-the-show-is-refused-until-the-operator-opens-it.md)).
**The per-operation argument above is what that record draws the classes with**, and it changes job
rather than losing: the three answers decide which class an operation falls in and therefore why it
is closed by default, and the operator's opening decides whether a closed class is reachable now.
The exit this paragraph names — an operator granting, which puts a hand back in the loop — is the
one that was taken, one level up from a node of a Set to a class of operations, because the mix
faders, the master chain and the outputs are no node's.

**The governor's rollback.** Why it is automatic: waiting for a human is answer three, and answer
two was available. Why it never demotes a Live slot: taking a slot off air is not a recovery, it is
the failure — there is no recovery from it and no way to explain it to an audience (P-0033). This
rule is what separates those two automatic actions, which are otherwise the same kind of act by the
same component. **An automatic recovery may act on what is off air and never on what is on air**,
and where it must touch the live picture it may only put it back where it was.

**The budget.** Two arguments, both already had. *Estimate optimistically and roll back* loses to
"take the earliest answer" (P-0067). And a scheduler that buys frame budget by freezing the panel's
continuous motion loses because it pays with answer three: it sells the one signal that says the
frame is in trouble, at the moment the trouble starts (P-0077). A budget that cannot afford the
liveness signal is a budget that is over, and *over budget* is the report, not a look.
[P-0072](0072-a-still-panel-costs-nothing-and-what-moves-declares-its-price.md) is the same rule for
the console: what must be live during a performance is named, and each named thing declares what it
costs and how stale it may get, ahead of the frame that pays.

**Where it loses.** [P-0070](0070-auditioning-is-a-prerequisite-not-a-convenience.md) is the case,
and it is a real loss rather than a technicality: an audition adds a draw that is not in the budget
and falls inside the watchdog's judging window, so **auditioning a heavy slot can roll back an
unrelated slot's build**. That is unbudgeted risk added to the live path, knowingly, and this rule
does not get to remove the control or hide it behind a flag. It loses because choosing between
candidates cannot be done blind, and a rule that protects a performance cannot be used to remove
what the performance is played with. The bill is paid and written down
([ADR-0072](../adr/0072-auditioning-adds-a-draw-and-never-a-step.md);
[P-0058](0058-before-v1-compatibility-is-a-bill-not-an-argument.md) is the form of the answer).

**And a fourth answer that looks like a violation.** `Frame::render` checks its sizes and **panics**
(P-0027) — stopping, which is the thing this rule is named against. The distinction is what the
condition can arise from. A programming mistake is not a hazard of performing; it is a class that
must never reach a stage at all, and the way to make sure of that is to make it impossible to miss
in development. A hazard the show meets legitimately — a costly Set, a NaN in the mix, an unplugged
interface, a dropped network — never panics: it rolls back, skips the slot, free-runs, falls back.
**A panic is admissible only where the alternative is a plausible wrong picture and the condition
cannot arise except from a bug.** Anything that a night can produce on its own has to have one of
the three answers instead.

## Three registers already say it, and none of them can be cited

**To a performer**, as a promise: [roadmap.md](../roadmap.md)'s fourth end-state property —
*"Unbreakable on stage. Nothing stops working when the network drops or the DJ gear changes."*

**To a developer**, as a mechanism: [README.md](../../README.md)'s *Real-time* bullet — *"A changed
procedure is compiled on a worker thread, installed at a frame boundary, watched for a window, and
rolled back automatically if it costs too much. Nothing allocates or compiles a shader on the render
thread."*

**To an operator**, as grammar they can rely on: four of the manual's seven rules —
*you can always see who is holding a control, and always take it back* (02), *nothing is hidden
quietly* (04), *every channel stays visible so nothing scrolls out of reach mid transition* (05),
and *there is no switch that hands the whole instrument to an agent* (06).

A promise cannot be checked against a patch. A mechanism reaches the mechanisms it names and no
further, and it names two shapes rather than the property behind them
([P-0060](0060-name-the-property-not-the-shape.md)). A rule of the surface binds the surface. **The
fourth register is this one** — the same sentence in the form a decision is held against — and this
file does not move into any of the other three. `docs/manual/index.html` in particular is capped by
its own rule, "adding one means removing or merging one", because it is what an operator holds in
their head before the panel makes sense and "a mental model that grows is a mental model that
breaks". `docs/principles/` is the registry the code is written to, and it accumulates. Putting this
here costs nothing; putting it in the manual would have cost one of the seven.

## Where it holds

**Everywhere, which is the claim, so here is what it is a property of rather than a list of what it
covers.** These are instances and were written before it: the three answers in
[P-0001](0001-nothing-allocates-or-compiles-a-shader-on-the-render-thread.md),
[P-0004](0004-a-live-set-is-never-mutated-in-place.md),
[P-0005](0005-a-swap-happens-on-a-frame-boundary-and-an-over-budget-set-rolls-back-on-its-own.md),
[P-0012](0012-a-measurement-carries-how-it-was-taken.md),
[P-0027](0027-a-silently-wrong-image-loses-to-a-loud-failure.md),
[P-0034](0034-a-quiet-room-is-not-a-missing-microphone.md),
[P-0037](0037-check-a-measurement-against-another-measurement.md),
[P-0039](0039-a-warning-that-fires-on-healthy-material-teaches-the-operator-to-ignore-warnings.md),
[P-0041](0041-observing-must-not-advance-what-is-observed.md),
[P-0043](0043-nothing-external-enters-the-render-process.md),
[P-0067](0067-the-language-is-bounded-so-a-procedure-can-be-priced-before-it-runs.md),
[P-0069](0069-the-three-clocks-never-collapse-into-each-other.md),
[P-0072](0072-a-still-panel-costs-nothing-and-what-moves-declares-its-price.md),
[P-0075](0075-a-pending-transition-shows-where-it-is-where-it-is-going-and-that-it-has-not-arrived.md)
and [P-0077](0077-continuous-motion-is-how-a-stopped-panel-announces-itself.md); the never-from-the-operator
clause in [P-0033](0033-the-governor-never-takes-a-live-slot-off-air.md),
[P-0052](0052-publishing-is-a-choice-of-attention-not-of-authority.md),
[P-0076](0076-a-surface-owns-the-affordance-never-the-authority.md) and
[P-0078](0078-the-operator-wins-and-an-automatic-writer-yields-to-a-hand.md); and the boundary in
[P-0070](0070-auditioning-is-a-prerequisite-not-a-convenience.md).

In code the three answers are `karakuri-ir`'s check pass and `cost::estimate` (refuse),
[swap.rs](../../crates/karakuri-engine/src/swap.rs) and
[governor.rs](../../crates/karakuri-engine/src/governor.rs) (undo),
[deck.rs](../../crates/karakuri-engine/src/deck.rs) and
[probe.rs](../../crates/karakuri-engine/src/probe.rs) (be loud about what it measured and what it
dropped), and `crates/karakuri-environment/src/mcp.rs`, whose six tools are the scope this settles.
What defends it in practice is listed in [roadmap.md](../roadmap.md) under *Continuous concerns →
Live safety*, whose own preamble is the reason this is a principle and not a milestone: **"Not
milestones. These degrade silently if not defended at every step."**

Named in
[ADR-0234](../adr/0234-carrying-a-show-through-is-a-principle-not-a-property-of-the-finished-instrument.md).
