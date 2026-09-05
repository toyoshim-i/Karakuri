---
id: 0234
title: Carrying a show through is a principle, not a property of the finished instrument
status: accepted
date: 2026-08-31
supersedes: []
superseded_by: []
principles: [0001, 0005, 0027, 0033, 0043, 0067, 0077, 0078, 0079, 0087]
tags: [process, docs, live, mcp]
---

# Carrying a show through is a principle, not a property of the finished instrument

> **Annotated 2026-08-31, later the same day.** The MCP scope this record treats as settled — and
> whose contradiction with the manual's rule 01 it left in *What this leaves undone* for the
> maintainer to decide — was decided that evening, and rule 01 won:
> [ADR-0235](0235-mcp-reaches-every-operation-and-what-could-stop-the-show-is-refused-until-the-operator-opens-it.md)
> connects MCP to all 64 operations and closes the performance-stopping classes **by default**
> rather than by scope, each opened from the head of the bay it belongs to. None of the three
> readings offered below is the one taken. Two counts here are also true only of the morning they
> were taken: `wire_input` landed the same day, so the tools are seven and the column is 7 `has` /
> 57 `gap`. What this record argues — that the philosophy needed a principle, and that the scope had
> a conclusion with no reason attached — is untouched, and P-0079 is what the decision was argued
> against.

> **Annotated 2026-09-02.** The principle this record cites as the case where P-0079 loses —
> P-0070, *Auditioning is a prerequisite, not a convenience* — was retired that day and
> re-recorded as
> [P-0080](../principles/0080-an-operator-can-see-a-slots-own-material-without-putting-it-on-air.md),
> *An operator can see a slot's own material without putting it on air*
> ([ADR-0241](0241-auditioning-survives-the-control-that-was-retired-and-is-re-recorded-as-a-property.md)).
> **The argument below is untouched and read it as written**: what P-0079 loses to is the
> requirement, which did not move; only the mechanism P-0070 had named as *where it holds* did.
> The sentence *"the first time this one is cited to delete a control is the argument it has to
> already have answered"* was tested within two days —
> [ADR-0240](0240-the-output-shows-the-mix-and-residency-keys-belong-to-the-mixer.md) deleted the
> control, on the ground that four continuous preview cells serve the requirement better than one
> switched output, rather than on the ground that the audition was unsafe. That is the argument
> this record said had to be met, met.

## Context

The maintainer, on 2026-08-31, confirming why the MCP surface is deliberately narrow — not every
function is opened, so that a model cannot break a live show — said that **carrying a live show
through safely is a philosophy he wants held consistently across the whole application, on a par
with the rendering pipeline's shader guarantees and its execution-cost guarantees.**

Those two are principles. This one was not, and the difference is what this record is about.

### It is written twice, from the two ends, and neither form can be cited

It is not unwritten. It is written in the two registers that surround the one it is missing from.

**To a performer, as a promise.** [The roadmap](../roadmap.md)'s fourth end-state property:
*"Unbreakable on stage. Nothing stops working when the network drops or the DJ gear changes.
Defended continuously rather than built once — see Continuous concerns."*

**To a developer, as a mechanism.** [README.md](../../README.md)'s *Real-time* bullet: *"A changed
procedure is compiled on a worker thread, installed at a frame boundary, watched for a window, and
rolled back automatically if it costs too much. Nothing allocates or compiles a shader on the render
thread."*

Neither is usable in a decision. **A promise to a performer cannot be checked against a patch** — it
is a statement about the finished instrument, and the question a reviewer has is about the change in
front of them. **A description of one mechanism does not reach the next mechanism**: the README's
sentence is true of the swap watchdog and says nothing about a scheduler, a tool list or an input
that goes quiet. And it names the shapes rather than the property, which is
[P-0087](../principles/0087-name-the-property-never-the-shape.md) happening in the entrance document —
delete or replace the swap watchdog and the sentence goes with it, while the rule it was an instance
of is untouched and now unstated.

**A third register exists and is not it either.** Four of the manual's seven rules are this
philosophy in the second person — *you can always see who is holding a control, and always take it
back* (02), *nothing is hidden quietly* (04), *every channel stays visible so nothing scrolls out of
reach mid transition* (05), *there is no switch that hands the whole instrument to an agent* (06).
Those bind the surface an operator touches, which is what they are for.

**What is not stated in any of the three is the property itself**, in the register the code is
written to.

### And the two lists of four are not two renderings of one list

Worth saying because it is the first thing a reader assumes. The README's four properties are a
taxonomy of what the system *is*, each with a document behind it; the roadmap's four are the tests
for whether the end state was *reached*. They coincide on two subjects — *GPU-native* with
*Procedural, not generated frames*, and *Real-time* with *Unbreakable on stage* — and diverge on the
rest. The README's account of manual and autonomous control on one mechanism is in its preamble
rather than in a bullet; **Reproducible appears in the README nowhere at all**, in any wording; and
*For VJ work* is not one of the roadmap's success tests. Two lists of the same length doing
different jobs, agreeing where they overlap. Nothing here is a contradiction to fix, and this
paragraph exists so that nobody records one.

### Eight principles are instances of a rule none of them states

Each of these decides a case of the same question and none of them says what the question is:

- [P-0001](../principles/0001-nothing-allocates-or-compiles-a-shader-on-the-render-thread.md) — the
  unbounded stall is moved off the frame path rather than made rare
- [P-0005](../principles/0005-a-swap-happens-on-a-frame-boundary-and-an-over-budget-set-rolls-back-on-its-own.md)
  — recovery is automatic *"because this runs in front of an audience"*
- [P-0027](../principles/0027-a-silently-wrong-image-loses-to-a-loud-failure.md) — a fader to zero
  has to work *"on exactly the material that is broken"*
- [P-0033](../principles/0033-the-governor-never-takes-a-live-slot-off-air.md) — automatic demotion
  is *"the worst behaviour it could have — there is no recovery from it and no way to explain it to
  an audience"*
- [P-0043](../principles/0043-nothing-external-enters-the-render-process.md) — a hazard whose worst
  case cannot be bounded from inside the process stays outside it
- [P-0067](../principles/0067-the-language-is-bounded-so-a-procedure-can-be-priced-before-it-runs.md)
  — *"the promise that a model can write badly without taking the show down"*
- [P-0077](../principles/0077-continuous-motion-is-how-a-stopped-panel-announces-itself.md) — the
  budget may not be balanced by selling the signal that says the frame is in trouble
- [P-0078](../principles/0078-the-operator-wins-and-an-automatic-writer-yields-to-a-hand.md) — the
  control an operator reaches for when something is wrong is the one the automatic writer is holding

A reading of the whole registry finds more of them —
[P-0004](../principles/0004-a-live-set-is-never-mutated-in-place.md),
[P-0012](../principles/0012-a-measurement-carries-how-it-was-taken.md),
[P-0037](../principles/0037-check-a-measurement-against-another-measurement.md),
[P-0069](../principles/0069-the-three-clocks-never-collapse-into-each-other.md),
[P-0072](../principles/0072-a-still-panel-costs-nothing-and-what-moves-declares-its-price.md),
[P-0076](../principles/0076-a-surface-owns-the-affordance-never-the-authority.md),
[P-0082](../principles/0082-looking-never-writes-back.md),
[P-0084](../principles/0084-a-confident-wrong-automatic-judgement-is-worse-than-not-judging.md),
[P-0087](../principles/0087-name-the-property-never-the-shape.md) — which is the
argument rather than a longer list. **A property that eighteen files are instances of is being
rediscovered per file**, and the eighteenth had to make it again from nothing.

### A live decision states its conclusion and attaches no reason

The MCP scope is the case that prompted this, and it is the evidence.
[The operations page](../manual/operations.html) says what a model may touch:

> **A model can rewrite a whole procedure and cannot turn one knob.** MCP edits material, reports
> swaps and reads the library, and touches nothing in the mix, the clock or what is on air. It can
> make a deck unrecognisable and cannot put it on air.

That is a shape with no property behind it. **The reason is not in the paragraph, not in
[ADR-0199](0199-mcp-names-its-operations-and-performs-them-itself.md)** — which records how the six
tools route and explicitly says *"nothing a tool does changed"* — **and not in
[ADR-0083](0083-mcp-is-the-only-prompt-surface.md)**, which chose three tools for reasons about
prompt surfaces and remarked in passing that *"the safety device already exists."* So the scope
cannot be argued with in either direction. Somebody proposing a seventh tool has nothing to meet,
and somebody defending the six has nothing to cite.

The numbers say how large that silence is. The page has 64 operations
(`grep -c '<h3' docs/manual/operations.html`), and the MCP column reads **6 `has`, 58 `gap`, and 0
`plan`** — the only column of the four with no `plan` rows at all, against key's 17 and MIDI's 1.
`gap` means *nothing — this surface cannot reach it at all*. **The other 58 are not a backlog; they
are a stance**, and a stance that is nowhere stated.

## Decision

**The philosophy is named as a principle**:
[P-0079](../principles/0079-nothing-takes-the-show-down-and-nothing-takes-it-away-from-the-operator.md)
— *Nothing takes the show down, and nothing takes it away from the operator.*

It is written to do one job: **give a decision something to be checked against.** Three parts carry
that.

**1. Three admissible answers, and the question that asks for one.** Every mechanism that runs
during a performance answers *what does this do at its worst, on the frame it goes wrong, while the
operator's attention is on the room?* — with *it cannot happen*, *it undoes itself and puts the show
back where it was*, or *it is loud and the operator's hands still work*. A mechanism that can give
none of the three does not run during a show. This is the part that generalises: the eight are
instances, and a ninth situation is covered by the question rather than by the list.

**2. Where more than one answer is available, take the earliest.** P-0067 already argued this inside
its own domain — optimistic estimation plus runtime rollback *"moves a refusal that costs nothing
into one that costs a dropped frame in front of an audience"* — and the ordering is general. It is
what makes the rule decide rather than merely permit.

**3. The price is never the operator's.** Safety is bought with design, with time, with budget and
with loudness, never with the operator's authority. The line is drawn where an argument would
otherwise be had every time: **the instrument refuses what it cannot do, and never what it judges
unwise.** A refusal at the door is answer one and the best of the three — nothing that was on air
changed, the diagnostic says why
([P-0083](../principles/0083-a-refusal-carries-what-the-next-attempt-needs.md)), every route meets the same
sentence ([P-0061](../principles/0061-a-refusal-a-person-can-reach-from-two-surfaces-is-one-sentence.md)).
A refusal of something the instrument *can* do, on the ground that it might go badly, is the second
failure this rule names, and *go on air* is the case P-0076 had already settled: *"in a live
instrument, the one operation that cannot be refused."*

**And it is written with the case where it loses.**
[P-0070](../principles/0070-auditioning-is-a-prerequisite-not-a-convenience.md) knowingly adds
unbudgeted risk to the live path — an audition can roll back an unrelated slot's build
([ADR-0072](0072-auditioning-adds-a-draw-and-never-a-step.md)) — and keeps the control, because
choosing between candidates cannot be done blind. A rule protecting a performance may not be used to
remove what the performance is played with. That case is in the principle rather than left for
somebody to discover, because **a rule that never loses is not a rule**, and the first time this one
is cited to delete a control is the argument it has to already have answered.

**What it answers about the MCP scope** is the test of whether it was worth writing, so it is
recorded here. Not *a model is untrusted*: `write_procedure` has all three answers standing behind
it — priced before it is built, swapped at a frame boundary and rolled back on its own, and reported
through `swap_outcome` — which is why a model may rewrite a whole procedure while it is on screen. A
mix write has none of them: unpriced, immediate, irreversible, and it changes what the audience is
looking at. **The scope is per operation and not per actor**, and that is a sentence somebody can
now argue with. It also names its own exit: what would let a model turn a knob is not a longer tool
list but per-node authority
([ADR-0211](0211-authority-is-set-per-node-and-the-record-is-the-sessions.md), manual rule 06), an
operator granting a node — which puts a hand back in the loop rather than removing the hazard.

**The principle goes in `docs/principles/` and not into the manual's seven.** That page is capped by
its own rule — *"adding one means removing or merging one"* — because it is what an operator holds
in their head before the panel makes sense, and *"a mental model that grows is a mental model that
breaks."* The registry accumulates, because a codebase learns. Adding this to the manual would have
cost one of the seven, and the four listed above are the ones it would have been merged with —
losing four operator-facing rules to gain one machine-facing one.

## Alternatives

### a. Leave it implicit

Keep writing the instances. Each principle argues the live stake in its own domain, where it is
concrete and where the argument is strongest, and no reader has to hold an abstraction.

**It loses on the evidence in the Context.** The MCP paragraph is what leaving it implicit produces:
a decision the maintainer can state and defend in conversation, written down as a conclusion, with
58 of 64 rows resting on a reason that is nowhere on the page or in either ADR behind it. That is
not a hypothetical cost — it is the state of the page today, and it is why this record exists.

The second cost is the one P-0087 describes. Eighteen files rediscover the property one domain at a
time; the eighteenth had nothing to cite and made the argument again. The next mechanism — a
scheduler for P-0072's arbitration, a second automatic writer, a follower, a sequencer lane — will
either make it a nineteenth time or, more likely, settle it by accident. P-0078 names exactly that
risk in its own scope: *"the second one is the change that would otherwise settle it by accident."*

And implicit rules are asymmetric in the wrong direction. They are available to whoever already
holds them and unavailable to everybody else — an agent writing a patch, a reviewer, a future
maintainer, and the model reading `principles/` over MCP, which ADR-0000 lists as one of the
registry's purposes.

### b. Write it as a roadmap section rather than a principle

It is nearly there already: *Continuous concerns → Live safety* lists four defences, and the
end-state property points at it. Extend that section instead, where the context and the milestones
are.

**It loses because a roadmap sentence cannot be checked against a change.** The roadmap is about
*what exists, what is next, and when* — its own words for the neighbouring section are that
*Deferred by decision* is about *when* rather than *what*. A promise there is in the future tense and
addresses the finished instrument; a reviewer holding a diff has no question the future tense
answers.

**And it rots differently.** [P-0063](../principles/0063-source-cites-what-is-in-force-not-a-plan.md)
is the rule that source cites what is in force rather than a plan, and a roadmap is a plan by
construction: a section describing the state of the work must be rewritten as the work moves, and
the rule inside it is rewritten with it. A principle is current-only and deleted when it stops being
true, which is a different discipline
([ADR-0000](0000-record-decisions-here-and-standing-rules-in-principles.md)). ADR-0233's Consequences
record the roadmap carrying a claim in the present tense that a decision had made false the same
week; a standing rule kept there would be exposed to that every time the milestone around it moves.

**The section's own preamble is the argument against putting it there.** *"Not milestones. These
degrade silently if not defended at every step."* A thing defended at every step is a thing every
step is checked against, and the registry is where this repository keeps those. So the roadmap
section stays exactly as it is — it lists what defends the property, which is a status, and it is
not the wrong document for that.

### c. Add it to the manual's seven

The manual is the top of the hierarchy and this is the philosophy behind four of its rules, so the
strongest placement would be to say it there and let the four narrow ones follow from it.

**It loses on the page's cap and on the register.** Adding one means removing or merging one, and
the merge that would pay for it collapses four rules an operator uses — *take it back*, *nothing is
hidden quietly*, *channels stay visible*, *no global switch* — into one abstraction they cannot act
on. The seven are what a person holds before the panel makes sense; a rule about how mechanisms are
designed is not something an operator holds, and it would be the only one of the seven not stated in
the second person.

### d. Widen an existing principle instead

P-0033 or P-0005 could be broadened from their own domains to state the general rule, which costs no
new number.

**It loses because a summary that is widened to fit a second caller stops meaning anything
precise.** P-0033 is narrow on purpose, and P-0076 had to say so in as many words — *"which is
narrower than it is often remembered as — it forbids automatic demotion, and says nothing about
granting a request."* A file already being misremembered as broader than it is, is the last file to
make broader. The general rule earns its own number; the instances keep their edges.

## Consequences

- **The principle exists at
  [P-0079](../principles/0079-nothing-takes-the-show-down-and-nothing-takes-it-away-from-the-operator.md),
  and with it the registry is seventy-seven files under seventy-nine numbers.** ADR-0000 said
  `principles/` should grow no index *"until it passes roughly forty files"*; it passed forty some
  time ago and has none. Nothing about this record changes that, and this bullet is the
  reminder rather than the decision — `ls docs/principles/` is still the index and is still true.
- **Nothing in the code or the manual changes.** No principle is edited, superseded or deleted; no
  behaviour is decided; the MCP tool list is exactly the six it was this morning. What this record
  changes is that the six now have a stated reason, and a seventh has something to be argued
  against.
- **The eight and the ten keep their arguments.** They are instances, not consequences: each was
  decided in its own domain against its own alternative, and P-0079 is the sentence they turn out to
  share. Reading it does not make any of them redundant, and none of them acquires a pointer to it
  here — a principle cites the ADRs that decided it, and this ADR is the place the relationship is
  written down.
- **The next mechanism has a question to answer at design time**, which is the whole return on this
  record: a scheduler arbitrating P-0072's declared regions, a second automatic writer, a follower or
  a sequencer lane writing a mix control, an output plugin, or a seventh MCP tool. Each of them now
  states which of the three answers it gives before it is built, rather than after somebody
  rediscovers the rule in review.
- **`docs/adr/INDEX.md` gains this row and gains nothing else.** P-0079 does not belong in *Standing
  rules with no record yet* — that section is for a principle whose *"reasoning is still only in the
  code and the specification"*, and this one's reasoning is this record.

### What this leaves undone

- **The manual's first rule and the MCP scope contradict each other, and the contradiction is now
  visible rather than settled.** Rule 01 reads: *"Every operation is reachable from the panel, from
  the keyboard alone, from a mapped MIDI control, and from a model over MCP. There is no operation
  only the mouse can reach, and none the map cannot address."* [The operations
  page](../manual/operations.html) says the opposite of the MCP half — *"A model can rewrite a whole
  procedure and cannot turn one knob … touches nothing in the mix, the clock or what is on air"* —
  and its MCP column is **6 of 64 with no `plan` rows**, so the other 58 are not waiting to be built.
  The maintainer confirmed the narrow scope today, for the reason P-0079 now states. **So rule 01 is
  either wrong as written or means something narrower than it says**, and there is a third reading
  worth putting in front of whoever decides: rule 01 may be true *of the vocabulary* — every
  operation is named once and any surface may route into that name — while what a given surface
  publishes is a separate question, which is very nearly what P-0076 already says about affordance
  and authority. **This record does not make that change.** `docs/manual/index.html` is capped, so
  amending rule 01 means removing or merging one of the seven, and that is the maintainer's to do
  and not a side effect of naming a principle.
- **Whether the six tools are the right six**, now that there is something to check them against.
  P-0079 says the test is per operation — does this operation have a bounded worst case on a route
  with no hand in the room — and nobody has run the other 58 rows through it. Some of them will pass:
  reading is answer one by construction, and four of the six are questions already.
- **What per-node authority does to this**, which is the exit named in the Decision and is unbuilt.
  ADR-0211 settled the address and the operations page records that *"what is missing is a writer"*
  — nothing sets a node's authority, so nothing has to survive a rebuild yet. The day something does,
  the question *may a model turn a knob on a node an operator granted it* is asked for the first
  time, and P-0079 plus rule 06 are what it is asked against.
- **The scheduler P-0072 and P-0077 both wait on.** P-0077 records that its forced clause has nothing
  to bind because there is no scheduler, and that the economy it forbids becomes available the day
  something has to choose. That day is also the first test of P-0079's *take the earliest answer*
  ordering against a component that has to choose between real costs.
