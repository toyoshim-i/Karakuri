---
id: 0211
title: Authority is set per node, and the record that carries it is the session's
status: accepted
date: 2026-08-28
supersedes: []
superseded_by: []
principles: []
tags: [vocabulary, records, agents, docs]
---

# Authority is set per node, and the record that carries it is the session's

## Context

`man / sug / auto` — whether a node is the operator's alone, an agent's to propose into, or an
agent's to act on — is rule 06 of [the seven rules](../manual/index.html) and one of the four
properties this project says it is defined by. It existed in three documents and in no code, and
the three documents named **three different addresses**:

- `docs/roadmap.md`'s four properties: *"Authority is a per-layer setting, not a global mode."*
- rule 06: *"Each node of a Set is manual, suggesting, or automatic, and you set that node by
  node."*
- [the console page](../manual/console.html): *"`man / sug / auto` on each node head, never a global
  mode"*, drawn on a node head addressed `L1:0`.

and M6 asked for *"Slot agents, one per slot rather than per graph node"*, which is a fourth thing
again. `docs/roadmap.md` carried this as the open decision **What authority is set on**, and named
what it was really blocking: the Inspector's authority row, which reads as blocked on `man / sug /
auto` existing nowhere and is actually blocked on nobody having picked an address, *"because an
implementation would have to pick one, and picking one from the code is the specification written
backwards."*

**The word-guarding had to come first and did**, in the commit before this one:
`docs/roadmap.md`'s *What the three words mean, and where* separates the three senses of `layer`,
the two of `authority` and the three of `slot`, and says outright that it takes none of the
decisions. This record is written in those terms, which is what it is for —
[P-0031](../principles/0031-a-name-means-one-thing-across-the-system.md) makes a decision written in
a word that means two things a defect rather than a wording preference.

## Decision

### 1. Authority is set per node of a Set, and *Set a node's authority* is the row

`docs/manual/operations.html` gains a row in *Inside a Set*, and
`karakuri_operation::Operation::SetAuthority { deck, node, authority }` is the variant.

**The evidence is what the vocabulary can address.** An operation here addresses a **node** —
`karakuri_operation::NodeAt`, a `(layer, index)` pair whose first caller in this workspace is the
MCP server — or a **deck slot**, `deck: u8`. There is no operation that addresses a *layer* of a
live Set, and the one variant carrying a `Layer` at all, `Operation::ListSets { holds, layer }`,
narrows a search of the store rather than reaching into anything running. So *per layer* is not an
address this vocabulary has, and giving it one would mean inventing the address as part of a
decision about agents.

That leaves the node and the deck slot, and **rule 06 rules the slot out outright**:

> There is no switch that hands the whole instrument to an agent, because the useful arrangement is
> almost always partial: this geometry is yours and that renderer is not.

A flag on a `deck: u8` and nothing else *is* that switch, at deck granularity. A deck slot holds one
Set, and the whole of what an operator would want to say — *this geometry is yours and that renderer
is not* — is unsayable at that address.

**The variant has two shapes and both were already here.** It names one of three, which is
`Operation::SetResidency`'s and `Operation::SetBlendMode`'s shape and
[P-0074](../principles/0074-an-operation-says-what-it-wants-never-which-way-to-move.md)'s
requirement — the vocabulary owns the value list, a surface asks for a destination and never for a
step, and the console's three-word chip is an affordance built over the three. And it carries a
`deck: u8` beside a `NodeAt`, which is `Operation::WriteProcedure`'s shape and `NodeAt`'s own
account of itself: *"The deck is not here. It is a field on the operation … which keeps one address
shape for within a Set and one for which Set."*

**`Authority::name` writes `manual`, `suggesting`, `automatic`, and there is no `ALL` beside it.**
The words are rule 06's rather than the console's `man / sug / auto`, on exactly the terms
`Residency::name` is not the status line's `LIVE`/`prim`/`park`: a record carries a name, and a node
head's abbreviation is a surface's vocabulary for a reader. `ALL` exists so a map file can be
offered the values a target may end in, and no map target names an authority — a map line cannot say
a node address at all — so it is absent on `WipeKind`'s terms and arrives with the first reader.

### 2. The record is `Record::Authority`, and it is the session's

```ndjson
{"t":"authority","slot":0,"layer":"L1","authority":"manual"}
{"t":"authority","slot":2,"layer":"L4","index":1,"authority":"suggesting"}
```

**This is the hard half, and the precedent decided it.** Every existing record that addresses a node
of a Set — `slot`, `capacity`, `param`, `bind`, `seed` — is a **Set file's** and carries no `slot`,
because what a Set *is* does not depend on which deck slot it happens to be playing in. The single
exception is `Record::Procedure { slot, layer, index, proc }`, which is a **session** fact about a
node: *"A Set file's `slot` record says what a Set is; this says what a deck slot became, at a point
in time, which is a fact about a performance."*

Authority is that second kind. **A Set does not know which agent is watching it.** An authority is an
arrangement between an operator and whatever else is in the room, made during a set; a Set file
carrying one would hand that node to an agent wherever it was next loaded, which is the same failure
`is_set_state` refuses a `gain` for — a Set file reaching outside the Set. So the record takes
`Procedure`'s shape exactly: a deck slot, a layer, an index absent at zero, and the value.

It is classified `Vocabulary::Session`, so `is_set_state` answers no, `Store::write_set` refuses it
and `project::key_for` drops it from the fold — which is worth naming because this is the first
record whose `(layer, index)` address makes it *look* foldable into a Set file and is not.

The level is a `String` and not an enum, on `residency`'s and `curve`'s terms: what a level is
allowed to be is the engine's to say, so a stream from a newer build reaches a diagnostic rather
than a parser that refuses the line.

### 3. The conversion needs no reading

`karakuri_operation_record::written` answers `Written::Records` with one record and reads nothing,
which puts it in the group with the faders rather than with the look and the mask pairs. Those two
need a `Current` because their record is written whole and each operation asks for a part of it
([ADR-0192](0192-an-operation-asks-for-what-a-surface-can-say-and-the-record-stays-whole.md));
`Record::Authority` has no other half. It is the seventh operation in that group, and the first
whose record was never written anywhere before.

One thing is translated: `karakuri_operation::Layer` into `karakuri_store::record::Layer`, in a
private `store_layer` function. **A function and not a `From` impl**, and the orphan rule settles it
rather than taste — both types are foreign to this crate, exactly as
[ADR-0194](0194-where-an-operation-becomes-a-record-is-a-crate-that-depends-on-both.md) records for
`karakuri-cli`'s `mix::blend_mode`. This crate is where it belongs because it is the only place in
the workspace that sees both spellings of that list at once.

### 4. What the row says about `Field` and about L5

**`Layer::Field` takes an authority like any other node.** It addresses no node in the rendering
sense — no pass, no buffers, it lowers into whoever evaluates it — and its params are declared,
addressable and an operator's to ride, which is the whole reason the arm is in `Layer` at all. A node
whose params an agent can move is a node an agent can be let at, so nothing about `Field` is special
here and the row says so rather than leaving it to be discovered.

**The L5 is a node and this operation cannot address it, and that is stated rather than settled.**
`docs/ir-spec.md` is explicit that *"`crate::node::Merge` is the node"* and equally explicit that
there is no `kind L5`, because a `kind` says what a procedure lowers to and compositing has none — so
L5 is in neither spelling of `Layer` and `NodeAt` has no value that names the merge. Reaching it
would mean `Layer` growing an arm, which is a change to what a `.kir` may declare and to a wire
value on five existing records; that is a decision about the IR, not about authority. It is also not
urgent: nothing on a merge can be moved by anybody today, since `Record::Merge` deliberately carries
no `gain`, `opacity`, `blend` or `mask` — *"a record whose producer does not exist waits for it"* —
so there is nothing for an agent to be granted.

## Alternatives

### Per deck slot — a flag beside `slot: u8`, and no node address at all

The cheapest by a wide margin: `Record::Gain`'s shape, no new address, no `Layer` translation, and
M6's *"Slot agents, one per slot rather than per graph node"* reads as asking for it.

**It loses to rule 06 on the manual's own words**, and the sentence is not a preference but the
reason the feature exists: *"There is no switch that hands the whole instrument to an agent, because
the useful arrangement is almost always partial: this geometry is yours and that renderer is not."*
A deck slot holds one Set. A flag on the slot is exactly that switch with the blast radius of one
deck, and the arrangement the rule calls *useful* — one node yours, another not — cannot be
expressed at that address at all. Rule 06 is the specification and the page is what the
implementation is checked against, so this alternative is not a cheaper way to the same thing; it is
a different thing.

**What it would have bought is real and is not lost.** A surface that wants *hand this whole deck
over* can send one operation per node, which is what a surface does with every other list here.

### Per layer — the wording the roadmap's four properties carried

*"Authority is a per-layer setting"* is the oldest of the three statements and the one this record
contradicts, so it is the one that owed an argument.

**It loses because the address does not exist.** No operation in this vocabulary names a layer of a
live Set — the only `Layer` in a payload is `ListSets`'s search filter — and there is nothing in the
engine, the record format or any surface that would receive *the L4s of deck 2*. Worse, `layer`
carries three senses (a `kind`, an architecture row `L0`–`L5`, and what one deck slot contributes to
the mix), and the four-properties sentence never said which; `docs/roadmap.md`'s *What the three
words mean, and where* is what made that visible. A decision taken in that word would have been
unrecordable in P-0031's sense — disjoint in practice at best, and not by name.

**What it was reaching for was probably the third sense** — hand a whole deck's contribution to an
agent — which is the per-slot alternative above, and it loses there.

## Consequences

**The engine work is owed and is deliberately not in this change.** A per-node flag has to survive a
rebuild, which means a further field on `karakuri_engine::swap::Request`, restated on every rebuild
the way that request's `params`, `published`, `bindings`, `names` and `edges` are. `swap.rs` says
what happens when one of those is left out, and it is the warning this inherits verbatim:

> a request that depends on what happens to be live is not reproducible from a record stream

> Left out, this was worse than a lost surface. The bindings *are* restated, so a `control:` binding
> survived a swap and the control it named did not — and a binding whose source is gone leaves its
> param where it was, silently, for the rest of the run.

> A rebuild that lets a value be re-derived is a rebuild that quietly discards what was loaded

Applied here that reads: a rebuild that let a node's authority be re-derived would hand a node back
to an agent an operator had taken it from, on the next save of any `.kir`, saying nothing. Until
that field exists **nothing writes an `authority` record**, and the manual's row says so.

**The Inspector's authority row is unblocked and is not drawn here.** `docs/roadmap.md` names that
row as waiting on the address; the address is taken, and drawing the chip is the console's change
and the console's crate.

**MIDI cannot reach this row and that is settled rather than outstanding**, for the reason already
written on the page: a map line carries a slot number, a word out of a value list, or a range, and
it cannot express a node address. The row's MIDI badge is empty and stays that way until the grammar
gains an address, which is a different decision.

**Three floors moved and there is no fourth.** `karakuri-operation`'s
`the_vocabulary_is_not_empty`, its `the_manual_and_the_vocabulary_agree` row scan, and
`karakuri-cli`'s `every_tool_this_server_publishes_has_a_route_on_the_page` all went 49 to 50.
`karakuri-console/tests/vocabulary.rs`'s floor of 8 counts the *Arranging the console* section only
and does not move.

**What this does not settle**: what an agent is one *of*. `docs/roadmap.md`'s control-plane roster
and M6 disagree about that, and it is a separate question — an agent scoped to a deck slot can
perfectly well respect a per-node authority. Only the authority half is taken here.
