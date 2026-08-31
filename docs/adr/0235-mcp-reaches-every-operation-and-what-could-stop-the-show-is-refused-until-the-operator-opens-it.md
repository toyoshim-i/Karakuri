---
id: 0235
title: MCP reaches every operation, and what could stop the show is refused until the operator opens it
status: accepted
date: 2026-08-31
supersedes: []
superseded_by: []
principles: [0005, 0027, 0046, 0061, 0067, 0074, 0076, 0078, 0079]
tags: [mcp, live, vocabulary, surfaces, docs]
---

# MCP reaches every operation, and what could stop the show is refused until the operator opens it

## Context

The maintainer, on 2026-08-31, hours after
[ADR-0234](0234-carrying-a-show-through-is-a-principle-not-a-property-of-the-finished-instrument.md)
recorded that the MCP scope was a stance nobody had stated:

> MCPについては原則通り全部を繋げるのを最終目標にしよう。そこは原点に帰る。ただし、ライブの原則も大切
> なので、リアルタイムパフォーマンスを止めうる操作はデフォルトでは禁止。例えばライブモードのデックの中
> 身やミックスフェーダー、マスターエフェクトなど。入出力系のルーティングや有効化無効化とかも。これらは
> Bayに開放設定を設けて、開放時のみMCPに見せよう。

**The final goal for MCP is that everything is connected, as the rule always said — that is a return
to the origin.** But the live principle matters too, so **any operation that could stop a real-time
performance is forbidden by default**: the contents of a deck that is in live mode, the mix faders,
the master effects, and the routing and the enabling and disabling of inputs and outputs. **Those get
an opening setting in a Bay, and are shown to MCP only while it is open.**

Asked what *shown only while open* meant mechanically, he settled it in the same conversation:

> これは繋がってるけど監査で禁止、失敗する仕組みを作るってことね

**It is connected, and forbidden by an audit; you build a mechanism that fails.** The two sentences
are recorded together because the second replaces the mechanism the first implied — 見せよう reads as
*publish a shorter list*, and what is decided is that the list never shortens and the call is refused.
That correction is the whole of *Alternative b* below, and it is a rejected alternative rather than a
misreading precisely because it was the first spelling of the decision.

### This is a return, and the numbers say what it returns from

[The manual's rule 01](../manual/index.html) has said it from the start:

> Every operation is reachable from the panel, from the keyboard alone, from a mapped MIDI control,
> and from a model over MCP. There is no operation only the mouse can reach, and none the map cannot
> address.

[The operations page](../manual/operations.html) says the opposite of the MCP half:

> **A model can rewrite a whole procedure and cannot turn one knob.** MCP edits material, wires what
> that material declares, reports swaps and reads the library, and touches nothing in the mix, the
> clock or what is on air. It can make a deck unrecognisable and cannot put it on air.

Counted today, after `wire_input` landed this morning: the page has **64 operations**
(`grep -c '<h3' docs/manual/operations.html`) and the MCP column reads **7 `has`, 57 `gap`, 0
`plan`** — still the only column of the four with no `plan` rows at all, against key's 17 and
panel's 42. `gap` means *nothing — this surface cannot reach it at all*.

**Two passes this week read the 57 as a deliberate scope, and both were reading the page correctly.**
[ADR-0234](0234-carrying-a-show-through-is-a-principle-not-a-property-of-the-finished-instrument.md)
called them *"not a backlog; they are a stance, and a stance that is nowhere stated"*, and
[P-0079](../principles/0079-nothing-takes-the-show-down-and-nothing-takes-it-away-from-the-operator.md)
then stated it under *What it settles → MCP's scope*. Neither invented the narrowness; both found it
written and gave it the reason it was missing.

**This decision says rule 01 was right and the scope paragraph was the thing standing in the wrong
place.** ADR-0234 left that open deliberately — *"rule 01 is either wrong as written or means
something narrower than it says"*, with a third reading offered in which rule 01 is true of the
vocabulary while what a surface publishes is a separate question, and *"this record does not make
that change"*. This record makes it, and it takes none of those three: rule 01 is true as written, of
the surface as well as of the vocabulary, because every operation is connected and a closed one is
answered rather than absent.

### What changes is the kind of thing the narrowness is

It was a **scope**: a fixed statement about what a model may ever touch, drawn once and for the life
of the program. It becomes a **default and a permission**: the operator opens a class, and MCP can
then reach it.

That is
[P-0078](../principles/0078-the-operator-wins-and-an-automatic-writer-yields-to-a-hand.md) in a form
nobody had applied here. P-0078 is about a hand landing on a control an automatic writer is moving,
and it already refuses to let the surface decide — *"a hand is a hand on all four routes"*. The same
rule read the other way is this: **what a model may reach is the operator's to say, and it is said by
an operator's hand rather than by a constant compiled into the server.** And it is what P-0079 asks
for in the clause it states most flatly: *"the price is never the operator's. Safety here is bought
with design, with time, with budget and with loudness. It is never bought with the operator's
authority."* A permanent scope is bought with the operator's authority — it decides, ahead of every
show and on behalf of every operator, that this instrument's model may not be trusted with a fader
even by someone who wants it to be.

### How this stands with P-0079's own worked case

A reader meets P-0079 first, so this has to be exact. **P-0079's argument is not wrong and is no
longer the whole rule.** What it argued is that the line falls *per operation* and not per actor:
`write_procedure` is priced before it is built (P-0067), lands at a frame boundary and rolls back on
its own (P-0005), and reports what it did (`swap_outcome`), while *"a write to a mix control has none
of the three: it is unpriced, it lands immediately, it does not roll back, and it changes what the
audience is looking at."*

That test survives intact and does a different job now. **It decides which class an operation is in;
it no longer decides whether a class is reachable.** The three answers are why a class is closed *by
default* — they are the argument for the default rather than a ceiling on what may ever be permitted.
The missing half was always the same one: the three answers ask what the mechanism does at its worst
*with nobody watching*, and an operator who has just opened a class is watching. **A hand in the room
is the fourth answer, and it is the one P-0079 named its own exit through** — *"the answer that would
let a model turn a knob is not a bigger tool list, it is per-node authority … an operator granting a
node, which puts a hand back in the loop rather than removing the hazard."* This is that exit, taken
one level up: **a class of operations rather than a node of a Set**, because the mix faders, the
master chain and the outputs belong to no node and rule 06's granularity cannot address them at all.

### The enforcement is an audit, and where it lives is the design

**A closed class is connected and refused.** The tool list is complete and constant; what varies is
whether the call is allowed through. Three reasons, in the order they decide it:

**Rule 01 says *reachable*.** A tool a model cannot see is not reachable, so a filtered list would
have kept the contradiction alive under a new name while claiming to have closed it. Connected and
audited is the reading under which rule 01 is simply true.

**A refusal can say why; an absence cannot.**
[P-0046](../principles/0046-a-diagnostic-says-why-not-only-what.md) is the rule that a diagnostic
states the constraint behind it, and it is the principle written *for this reader in particular* — *"a
diagnostic written for a human turned out to be a specification for a model."* A model that cannot see
a tool learns nothing and plans around a hole it cannot name. A model that is refused is told the
class is closed, that the operator can open it, and which class it is — which it can act on, and can
report to the person sitting there. That is also
[P-0027](../principles/0027-a-silently-wrong-image-loses-to-a-loud-failure.md) in the register it
applies to a tool surface: a missing tool is the plausible-looking output — everything reads as
working, nothing is logged, and what is wrong is invisible in exactly the way a black frame from an
out-of-range `textureLoad` is.

**One refusal, one sentence.**
[P-0061](../principles/0061-a-refusal-a-person-can-reach-from-two-surfaces-is-one-sentence.md): this
refusal will be reachable from more than one surface the moment a second automatic route exists, so
it is written once, in one function, and asserted by equality against that function rather than by a
`contains`.

**And it costs a check on every call rather than a filter on one listing.** That check is the thing
that has to be right, because **an audit skipped on one path is the whole mechanism gone.** The
vocabulary is where it can be applied uniformly, and the code is already shaped for it:
`crates/karakuri-environment/src/mcp.rs`'s `asked()` turns a tool name and a JSON object into an
`Operation` from `karakuri-operation`, and `perform()` dispatches on *that operation* rather than on
the string — *"what routes is the naming"*. So the audit reads an `Operation`, in one place, after
every tool has been named and before any of them acts.

**`tools()` stays what it is**, which is worth checking rather than assuming: it is a `fn` returning a
`json!` literal, with only the layer list computed (from `LAYERS`, so a client is offered exactly what
`layer_named` accepts). Nothing about it becomes stateful, it needs no access to whether a class is
open, and the list a model reads at the start of a session is still true at the end of it.

**The route that would defeat it is a lane, not a tool.** ADR-0222 settled that a sequencer lane is a
fifth route into this vocabulary — *"a lane emits operations on the beat the way the pointer, the
keys, a map and a model emit them"*. A model that cannot write `SetOpacity` but can point a lane at a
deck fader and unmute it has written the mix one beat later, through a path with no audit on it. So
the audit is on **the operation as it lands, together with who asked for it**, and the lane rows are
classed accordingly below.

## Decision

### The final goal is every operation, and rule 01 is what it returns to

MCP is connected to all 64 operations. The MCP column of the operations page becomes a column of
`plan` rather than a column of `gap`: nothing on it is *unreachable by design* any more, and what is
not built is a backlog like every other column's.

### Four classes are closed by default

Named by the maintainer, and each argued from P-0079's question — *what does this do at its worst, on
the frame it goes wrong, while the operator's attention is on the room?* — rather than asserted. The
operations are `karakuri-operation`'s, which is the vocabulary all four surfaces route into.

**1. What a live deck is drawing.** `LoadSet` (on a slot that is Live), `WriteParam`, `AttachSignal`,
`TakeParamBack`, `SetProperty`, `SelectRenderer`, `SetCompositing`, `Publish`, and `SetResidency`.
At its worst each of these replaces what the audience is looking at, in the frame it arrives, with
nothing that prices it and nothing that puts it back. `WriteParam` is the plainest: the manual calls
it *"the sharpest gap"* because it is a **uniform write** rather than a rebuild — it lands at once,
where a procedure rewrite goes through the swap machinery. `Publish` is here for a different reason
and belongs to the same rule: narrowing a live deck's published interface takes controls out from
under the operator's hand and renumbers every MIDI binding after the one it removed, which is
P-0079's *"buying the safety back from the operator"* performed by a model.

**Two in that list have an unobvious class and are named for it.**

- **`LoadSet` on a slot that is not live is not in this class.** The class the maintainer drew is *a
  deck in live mode*, so it is a predicate over an operation **and its target**, not over an operation
  alone: loading into an `Allocated` or `Priming` slot touches nothing on air, and its worst case is a
  build the governor already handles without the operator noticing. That is a property of the class as
  stated rather than an invention here, and it has a cost the audit has to carry: it reads engine state,
  so two identical calls can be answered differently a minute apart, and the refusal sentence must
  therefore name the deck and say that it is live — a refusal that only says *closed* would be unfixable
  by the model that got it (P-0046).
- **`SetResidency` is the hinge and is closed for that reason.** It does not act on a live deck's
  contents; it is how a slot *becomes* live, and how one stops being live. Leaving it open while
  closing the contents would be a hole big enough to walk the whole class through — a model cannot
  write a live deck's material, so it takes the deck off air, writes it, and puts it back — and its own
  worst case is the one P-0033 calls not a recovery but the failure: a slot taken off air in front of a
  room.

**2. The mix faders.** `SetGain`, `SetOpacity`, `SetBlendMode`, `FadeDeck`, `Crossfade`, `Wipe`,
`SetMaskShape`, `SetMaskPosition`, and `SetMasterOut`. This is the class P-0079 already worked: none
of the three answers is available, and it is what the audience is looking at. `SetMasterOut` is here
rather than with the master effects because it is a level and not an effect — it is also the largest
single reach in the vocabulary, one number at the entry to the master chain that blacks out the whole
fold. Which of the two classes it is filed under changes nothing while both are closed, and it is
said out loud so that nothing falls between them.

**3. The master effects.** `SetFeedback`, `SetBloom`, `SetRgbShift`, `SetTonemap`, `SetExposure`.
Every one of them acts on the composited frame after the mix has run, unpriced and immediately, and
the first three carry `Undecided` payloads because the chain does not exist yet — so they are closed
before they are buildable, which is the correct order. `SetTonemap` and `SetExposure` are the ones a
reader might argue are merely a look: at their worst they are the transfer and the level going into
it, and a wrong one is a whole show either black or blown out with the operator's hand nowhere near
it.

**4. Inputs and outputs, routed, enabled and disabled.** `AttachBeatSource`, `SetPreview`,
`RouteFrame`, `RecordSession`. `AttachBeatSource` switches what drives the clock the entire
performance is running on; `SetPreview` decides whether the output shows the mix or one deck
auditioned; `RouteFrame` decides where the frame goes at all; `RecordSession` starts and stops the
thing a night is being kept in. None of them has a bounded worst case and all four are the show's
plumbing rather than its picture, which is exactly why they are easy to forget.

### Four groups fall on the closed side under the rule and are not in the maintainer's examples

His list is exemplary — 例えば — and the rule is the question, not the list. These are what the
question answers where the examples do not reach, and they are marked as this record applying the
rule rather than as words that were spoken.

- **The clock.** `TapBeat`, `ScaleGrid`, `SetLatencyOffset`, `SetSync`, `ScrubDeck`,
  `SetFreeRunTempo`. Unpriced, immediate, irreversible — a tap sets the phase and there is no
  un-tapping it — and everything moving on the grid moves with them. The manual's own scope
  paragraph named the clock beside the mix, and it was right to.
- **`Quit`.** The sharpest case in the vocabulary and in none of the four classes: it does not risk
  stopping the performance, it stops it. It has no first answer, no second and no third.
- **`SelectDeck`.** A console pointer that writes no record, and the reason it is closed is P-0079's
  second half rather than its first: it decides which deck the operator's *next key press* lands on.
  Its worst case is the operator putting the wrong thing on air with their own hand.
- **The sequencer's lanes.** `PointLane`, `SetLaneMute`, `SetStep`, `SetPatternGrid`,
  `SelectPattern`. A lane emits operations, and three of the four the console draws are deck faders,
  so a lane pointed at a fader and unmuted is a mix write on a delay. All five carry `Undecided`
  today because nothing in this program holds a pattern; they are classed now so that the pattern
  arriving is not also the day the audit acquires a hole.

**And one row is closed in a way the others are not.** `SetAuthority` is rule 06's operation — who
may move one node — and it is the mechanism this whole decision is an instance of. **A permission an
actor can grant itself is not a permission.** It is closed by default like the rest, and this record
recommends that it and the Bay's opening setting are the two things that are never openable to MCP;
that recommendation is flagged in *What this leaves undone* rather than decided here, because it is
the one point where *connect everything* and *the operator opens it* genuinely collide.

### The opening is an indicator and a toggle in the head of the bay the class belongs to

Decided by the maintainer the same day. Four bays, one per class:

| class | bay |
| --- | --- |
| the mix faders | **Mixer** |
| the master effects | **Master** |
| input/output routing, enabling and disabling | **Outputs** |
| what a deck that is live is drawing | **Program** |

**Three of the four are one-to-one and the fourth is not.** Asked where the live-deck class goes, he
said: *"Program, I suppose — though it is hard to find."* It is there because **the Program bay is
where what is on air lives** — the picture and the preview cells — so the toggle for the material
that is on air belongs beside them. **The Inspector was the other candidate and loses for a concrete
reason**: the Inspector is where a node's source is read and written, so an opening drawn there would
close `write_procedure` by default — the one tool of the seven that is actually used, and the one
this record has just argued must stay open. A placement that closes the open case is the wrong
placement.

**And it is hard to find, which is written down rather than smoothed over.** That is the maintainer's
own word for it and it is a cost this decision carries: an operator looking for *may a model touch
the mix* will look at the Mixer bay and find it, and one looking for *may a model rewrite what is on
air* has to already know that on-air lives in Program. [The roadmap](../roadmap.md)'s fifth
instrumentation command turns that into an instruction for whoever writes the page — *"the more
detail something is written in, the more important it is and the sooner it is wanted"* — so **a
control an operator will not stumble on owes more prose than one they will**, and the Program bay's
indicator owes the most of the four.

**The mechanism exists and needs no new idiom.** `crates/karakuri-console/src/view.rs` gained
`head_pills` this morning: it lays a bay head's pills out once, right to left, and `bay_head` paints
from it while `program_head` hit-tests from it, so *"the capsule an operator sees and the capsule a
press lands on cannot come apart."* The Program bay head's `solo` pill is its first user and the
opening sits beside it.

### What is already open stays open, and `write_procedure` is why the classes are drawn by the question

The seven tools that exist — `read_procedure`, `write_procedure`, `wire_input`, `swap_outcome`,
`read_set`, `list_sets`, `save_set` — are unaffected. Nothing closes today and no model loses a call
it could make yesterday.

**`write_procedure` rewrites the contents of a deck that is in live mode. It is inside the
maintainer's words and it stays open, and that is the case that decides how the classes are drawn.**
Saying it plainly: a model can take a slot that is on air in front of a room and replace the source
of the node it is drawing with, while the audience watches. If the class were read off the noun —
*the contents of a live deck* — this would be the first thing closed, and the only thing MCP does
today would be the first casualty of a decision whose stated goal is to connect more.

So the class is not the noun. **It is P-0079's question with the noun as its subject: unpriced,
immediate, irreversible writes to what a live deck draws.** `write_procedure` fails to be one of
those on every count — priced by the check pass before it is built, compiled off the frame path,
installed at a frame boundary, measured for thirty frames and rolled back to the parked previous
version if it costs too much, with the outcome reported through `swap_outcome`. Its worst case is the
picture it replaced, coming back. That is all three answers, and the same three keep `wire_input`,
`restore_procedure`-style rows and the edit history on the open side, because they land through the
same rebuild.

**The 23 operations open by default**, then, are the seven above plus `RestoreProcedure`,
`WalkHistory`, `KeepCandidate`, `WatchFiles` (whatever a watched file changes still arrives through
the priced swap), `SelectScope`, `TransferSet`, `SetTransition` (it writes nothing to the stream and
changes nothing you can see — it decides what the operator's *next* gesture means, which is visible
on the panel and reversible in one write), and the nine that arrange the console: `MoveBoundary`,
`FoldBay`, `FoldPane`, `Unfold`, `Solo`, `ResetArrangement`, `SaveArrangement`, `RestoreArrangement`
and `SizeWindow`. **The console rows are the closest call on this side.** Folding away the bay holding
the fader an operator is reaching for is a real hazard and it is answer three with the controls
intact: it is loud — it is the largest visible change the panel can make — and it is undone by one key
the operator's hand is already near, `z` for what is folded and `r` for the arrangement.

**41 closed, 23 open, 64 total.** The split is a default and each closed row is one opening away from
being reachable, which is the difference this record exists to make.

## Alternatives

### a. Keep it as a scope, and argue only about which operations are in it

The position both of this week's passes landed on, and it is coherent: P-0079 gives it a reason, the
reason is per operation, and a seventh tool now has something to be argued against. It loses on the
maintainer's own ground — the rule was always *every operation, four ways in*, and a scope makes rule
01 permanently false in the one column that never had a `plan` row. It also fails on P-0079's own
terms, because the safety is bought with the operator's authority: it settles, for every operator and
every show, that no hand in the room is enough.

### b. Close by not advertising — publish a shorter tool list while a class is closed

The first spelling of the decision, in 見せよう, and it has a real argument: a model does not plan
around a tool it cannot see, so a closed class costs nothing in wasted calls, and the refusal path
never has to be written. It loses on three counts and the maintainer replaced it the same day.
**Rule 01 says reachable, and an unadvertised tool is not reachable** — the contradiction this
decision exists to end would have survived under a new name. **A refusal says why and an absence
cannot** (P-0046): the model learns nothing, and neither does the person sitting beside it, who is the
one who can open the class. **And a shortened list is P-0027's silently wrong picture in the register
of a tool surface** — nothing is broken, nothing is logged, and the model reports back that the
instrument cannot do a thing it can do. It also makes `tools()` stateful and makes the list a model
read at the start of a session untrue by the middle of it, for nothing.

### c. Per-node authority alone, which is the exit P-0079 already named

Grant a model a node with `SetAuthority` and let the existing rule 06 machinery carry the whole
question. It is the right mechanism for what it addresses and it cannot address this: **authority is
per node of a Set**, and the mix faders, the master chain, the outputs, the clock and the transport
are no node's — `SetMasterOut` deliberately names no deck at all, and the merge node that folds a
slot's renderers is in neither spelling of `Layer` and cannot be addressed by `SetAuthority` today. A
mechanism that reaches inside Sets and nothing between them leaves every class named here untouched.
The two compose instead: authority says who may move a node, and the opening says which classes are
live for a model at all.

### d. Open everything and rely on the session record to undo it

The record is the timeline and a replay reconstructs it, so in principle a bad write is walkable
back. It loses to P-0079's ordering — *where more than one answer is available, take the earliest* —
and it does not actually have the second answer: walking history is not automatic, it does not happen
on the frame it goes wrong, and *"a mechanism whose recovery plan is the operator noticing"* is the
exact shape the principle is named against.

### e. Ask the operator per call — a confirmation, a pending queue, a hold

Explicitly forbidden, and by the same principle that motivates the decision: *"a confirmation before
a live action, a lock while something is pending, a mode that will not let the show be put in a state
the machine dislikes"* — each makes a show safer by making it less playable. An opening set once,
ahead of the show or between numbers, costs the operator's attention at a moment they have some. A
prompt costs it at the moment they have least.

## Consequences

- **Rule 01 stands as written and needs no amendment.** The manual's seven are capped and this
  decision spends none of that budget — which is the second reason to prefer this shape over
  ADR-0234's third reading, where rule 01 would have had to be narrowed to a statement about the
  vocabulary.
- **The MCP column of the operations page becomes 7 `has`, 57 `plan`, 0 `gap`.** Every row is meant
  to be reached and is not yet, including the closed ones: a closed class is *reached* and answered
  with a refusal, which is a built route. The page's *"60 of the 272 ways in exist"* count is
  unaffected, because only `has` counts as existing.
- **The audit is one function over an `Operation`, called from one place.** It reads the operation and
  who asked for it, not a tool name — which is what keeps a second route (a lane, a follower, a
  second server) from arriving with its own copy of the table. Its refusal is one sentence from one
  function (P-0061), asserted by equality.
- **Some rows are audited against engine state, not against a table.** `LoadSet` is the known one and
  `SetResidency` is why it cannot be avoided by closing the class harder. This is the sharpest new
  cost in the design: the audit is not a pure function of the operation.
- **The seven tools' behaviour does not change, and neither does `tools()`.** No code in this
  workspace changes on the day this is recorded.
- **A model's failure mode is now a refusal it can report.** The tool description and the refusal
  sentence both have to say that the class is closed *and that an operator can open it*, or the model
  will report the instrument as incapable rather than as closed.

### The manual work this creates

Handed over precisely, because the pages are another writer's:

- **`docs/manual/operations.html`, the scope paragraph** — *"A model can rewrite a whole procedure and
  cannot turn one knob. MCP edits material, wires what that material declares, reports swaps and
  reads the library, and touches nothing in the mix, the clock or what is on air. It can make a deck
  unrecognisable and cannot put it on air."* It is now wrong as a statement of what MCP may ever do,
  and right as a statement of what is open by default.
- **The two paragraphs written today that give the scope its reason.** The first — *"The reason is
  that a model must not be able to break a live show"* — ends *"That is the line, and it is drawn per
  operation rather than per surface: the day a mix write is priced and reversible is the day this
  paragraph is wrong."* **It is now wrong for a different reason than it predicted**: no mix write
  became priced or reversible; the line stopped being a line and became a default. The second — *"And
  the narrow half is not the weak half"* — survives as written, and its closing sentence about *"a
  column with six badges"* is stale twice over (seven badges, and the column is no longer a stance).
- **57 MCP badges move from `gap` to `plan`**, leaving the column 7 / 57 / 0. Whoever makes the change
  should expect the number to be exactly 57 unless the maintainer rules that a row is never connected
  at all — `Quit` is the only plausible candidate, and this record classes it as closed rather than
  disconnected.
- **`docs/manual/console.html` owes four bay heads an indicator**, and the Program bay's owes the
  most prose of the four for the reason above. What the pill says in each of its two states, and what
  a bay carrying no class draws, is the console page's.

### What this leaves undone

- **Whether the setting is per class or per operation within a class.** Per class is what was said;
  per operation is finer and multiplies the surface by 41.
- **Whether a bay that carries no class draws the indicator at all.** The Library, the Inspector, the
  Staging bay and the Sequencer have no class of their own, and an indicator that is present
  everywhere reads as a state where it is absent.
- **Whether an opening survives a restart.** A setting that persists is one an operator can forget
  they left on; one that does not is one they have to set again before every show.
- **Whether an open class closes itself after a while**, and if so whether the timer is wall clock,
  idle time, or the show's own transport.
- **What a model is told when a class closes mid-session while it is mid-plan.** The tool list does
  not change, by construction — so the only signal is that a call which succeeded a minute ago is
  refused now, and a model three calls into a plan learns it on the fourth. Whether that is enough,
  and whether anything should be pushed rather than discovered on the next call, is open.
- **Whether `SetAuthority` and the opening setting itself are ever openable.** This record recommends
  no, on the ground that a permission an actor can grant itself is not one, and does not decide it —
  it is the one place *connect everything* and *the operator opens it* pull against each other, and it
  is the maintainer's.
- **The opening setting has no operation.** Nothing in `karakuri-operation` names it, so there is
  either a 65th row on the operations page or a reason it is not one — and if it is a row, rule 01
  says it is reachable from four surfaces, which walks straight into the question above.
  *(**Annotated 2026-08-31, later the same day: this one is settled and there is a reason it is not
  a row.**
  [ADR-0236](0236-a-map-is-the-layer-between-a-surface-and-the-vocabulary-and-the-audit-is-one-of-the-things-it-does.md)
  decides that the opening is **configuration of the map** — the layer every surface reaches the
  vocabulary through — and not a member of the vocabulary the map addresses, so no 65th row exists
  and the circularity does not arise. The rule it draws is narrower than *map configuration is never
  an operation*, because `Operation::PointLane` already is one: **a setting that decides whether a
  surface may reach a class of operations cannot itself be one of those operations**, since rule 01
  would make it reachable from the surface it governs. What survives of the item above it is
  `SetAuthority` alone, which is a genuine row and still open.
  [P-0066](../principles/0066-an-adr-is-a-description-of-history-corrected-but-never-revised.md).)*
- **Whether the clock, `Quit`, `SelectDeck` and the lane rows are closed as this record classes
  them.** They are the rule applied past the examples it was given, and each is one line to move.
- **What "inputs and outputs" covers.** Read here as the instrument's signal I/O — the beat source,
  the preview, the frame's destination, the recorder — and not as `TransferSet`'s sending a Set to
  somebody, which touches nothing on air and is classed open. If the maintainer meant file transfer
  too, that is one row.
