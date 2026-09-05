---
id: 0223
title: A wildcard write is refused where the nodes it lands on disagree
status: accepted
date: 2026-08-30
supersedes: []
superseded_by: []
principles: [0090, 0094]
tags: [engine, agents, vocabulary, docs]
---

# A wildcard write is refused where the nodes it lands on disagree

## Context

Two things that were each decided well are, together, a hole.

**Authority is per node.**
[ADR-0211](0211-authority-is-set-per-node-and-the-record-is-the-sessions.md) settled that against
per-slot and per-layer, on rule 06's own sentence — *"There is no switch that hands the whole
instrument to an agent, because the useful arrangement is almost always partial: this geometry is
yours and that renderer is not."*
[ADR-0216](0216-a-node-nobody-has-spoken-for-is-manual-and-a-request-states-only-what-was-said.md)
built the engine half: `karakuri_engine::set::Authority` per node, `Manual` for a node nobody has
spoken for, restated across every rebuild on `swap::Request::authorities`.

**A parameter write with no address is a wildcard.** `karakuri_operation::ParamAt`'s `node` is an
`Option`, `karakuri_engine::binding::ParamWrite`'s `at` is an `Option`, and absent means *every node
of the Set that declares the key* in both. That is [ir-spec.md](../ir-spec.md)'s rule — **one
control per key, not one per declaration** — and it is what makes a published interface small enough
to learn. It is also the default: a Set with no `--publish` publishes one control per key, and
`crates/karakuri` has no `--publish` flag at all, so **every control the panel will ever draw is a
wildcard**.

Put together: grant an agent one renderer, and a bare-name control declared by that renderer and by
the geometry beside it reaches both. **The grant of one node is, through any such control, the grant
of every node declaring that key** — rule 06's switch at the width of a key rather than the width of
a deck.

### What was read before deciding, because the shape of the answer depends on it

- **`Set::write_param` is the one entry point, and it is one in fact and not only in its own doc.**
  Outside the engine it has exactly one non-test caller — `karakuri-cli`'s `build`, which applies
  `--param` and the params a `--load-set` restates. Inside it there are two: `swap.rs`'s rebuild,
  applying `Request::params`, and `Set::set_published`, which is the published-control route. There
  is no fourth. `Set::set_param` and `Set::set_param_at` are public below it and are reached only by
  this crate's own tests.
- **Nothing in this workspace distinguishes an agent's write from the operator's**, and the reason
  is sharper than *not yet*: **no agent write of a parameter exists at all**. The MCP server
  publishes six tools — `read_procedure`, `write_procedure`, `swap_outcome`, `save_set`, `read_set`
  and `list_sets` — and not one of them moves a value.
  [The operations page](../manual/operations.html) already says it in its own words: *"A model can
  rewrite a whole procedure and cannot turn one knob."* `Operation::WriteParam` is constructed
  nowhere and executed nowhere; its only mention outside `karakuri-operation` is
  `karakuri-operation-record`'s classification of what it would write.
- **So every parameter write in the program today is the operator's**: `--param`, the params a Set
  file restates through `--load-set`, and that same list restated on every rebuild. A `Record::Param`
  on replay reaches the same list. `Set::set_published` has no caller yet.

That is the fact this decision had to be designed around. A rule keyed on *who is asking* would have
had one asker, no second call site, and no test that could fail.

## Decision

**A bare-name parameter write is refused unless every node it lands on is under one authority, and
the refusal names each of those nodes and what it is under.**

`karakuri_engine::set::CrossesAuthority` is the refusal; `Set::write_param` returns
`Result<usize, CrossesAuthority>`; the sentence is

> `` `radius` `` is declared by nodes that are not under one authority — `L1:0 manual, L4:0
> automatic` — and a bare name writes every node that declares it

with a hint naming the two ways out.

**It is in the one entry point**, which is
[P-0090](../principles/0090-a-surface-offers-it-never-decides.md): a
`--param`, a `param` record on replay, a rebuild's restatement and a published control are the same
wildcard and meet the same wall in the same words. It is not in a surface, which is
[P-0090](../principles/0090-a-surface-offers-it-never-decides.md) — the rule lives
where the write lands, so the panel, a key, a map and a model all meet it.

**Refused whole, before anything is written.** The check runs before the walk that would write, so a
refused write moves nothing.

**The addressed write is never refused.** What this rule takes away is one *spelling* and never the
reach: `--param L4:1:exposure=2.0` says which node it means, so it crosses nothing, and a node an
operator granted is still a node the operator can write.

**A key this Set declares nowhere is not a refusal.** An empty landing is uniform, and the caller's
answer stays the `0` it already reports *no parameter named* for — a name a regenerated artifact no
longer has should not take the show down, and it does not start doing so by way of an authority it
never had.

### Why this needs no asker, and what does

**A control that spans two arrangements is incoherent whoever is holding it.** That is why the
recommendation could land now: the refusal is a property of the *landing*, not of the hand. It
closes the hole in the Context outright — after the grant, the bare name is refused, so the agent
cannot reach the kept renderer through it, and neither can anything else.

**What needs an asker is the narrower question**, and it is not taken here: may an agent write an
*addressed* param on a node the operator kept? Answering yes or no needs the write to carry whether
the operator or an agent is asking, and nothing in this workspace can say. That is named in
[Consequences](#consequences) as what this leaves undone.

### And this is not P-0094 stood on its head

[P-0094](../principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md) says the
operator wins, and a refusal an operator can meet reads at first like its opposite. It is not, on
two counts. P-0094 is about a *writer that fights back* — an automatic thing reasserting itself over
a hand — and nothing here writes anything: the wildcard is refused for everybody at once, and the
last write stands. And the hand is not stopped: the addressed form is always available, and
`--publish NAME=L4:0:key[…]` narrows the interface to one node. The operator loses one *spelling* of
a write over a Set they have deliberately split in two, and gains a sentence saying which nodes they
split it into.

## Alternatives

### b. Land it only on the nodes whose authority allows it, silently partial

The cheapest, and it keeps every published control working after a partial grant. **It loses to
[P-0094](../principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)**, which is about
exactly this: a control that moves three renderers of four and looks like it moved all of them is
the plausible wrong picture, and the operator finds out mid-set. A fader is what a VJ reaches for
when material has gone wrong, and one that moves *most* of what it is drawn over is worse than one
that refuses.

**The mechanism makes it worse than the principle already does.** `write_param` answers with a count
and its only caller tests `== 0`, so a partial landing returns a plausible number and is
indistinguishable at that call site from a write that reached everything. The failure would not even
be visible to the code that reports it.

**And it cannot be spelled today.** *The nodes whose authority allows it* presumes an asker: with the
operator as the only writer that exists, every node allows every write, so (b) implemented honestly
is a no-op that would be mistaken for a rule. Its refusal would arrive the day an agent did — as
silence.

**Its argument is real and is the roadmap's**: a partial grant makes a published control unusable
until the operator grants the rest or narrows the interface. Answered rather than dismissed — the
addressed write is always there, `--publish` addresses a node, and *unusable and saying so* beats
*usable and lying*.

### c. Require the published interface to name nodes — add `--publish` to the panel and stop defaulting to wildcards

This removes the hole by removing the wildcard.

**It loses to [ir-spec.md](../ir-spec.md)'s own design.** *One control per key, not one per
declaration* is what keeps a published interface learnable, and the default interface is the whole
of what an author gets for free; making every panel control an explicit declaration is a chore
imposed on every Set to answer a question about agents.

**And it does not actually close the hole.** A wildcard stays spellable — `--param exposure=2.0` is
one, and an author may publish one deliberately — so a rule at the place the write lands would still
be owed, which is P-0090. (c) is an *affordance* change offered in place of an authority rule, and
the affordance is not where the authority lives.

### Introducing the asker now, with the operator as the only value

An `Asker { Operator, Agent }` on `write_param`, `Agent` constructed nowhere, the refusal keyed on
it.

**Rejected on [ADR-0216](0216-a-node-nobody-has-spoken-for-is-manual-and-a-request-states-only-what-was-said.md)'s
own terms**: it landed `mix::authority` with a `dead_code` attribute rather than inventing a caller,
because *"the alternative was inventing a caller, which would have been this decision's scope growing
to fill a warning"*. Here the invention would be larger — a parameter threaded through four call
sites, none of which can pass anything but `Operator` — and
[contributing.md](../contributing.md) §1 asks for two call sites before an abstraction. What it would
have bought is the narrower refusal, which is a decision about what an agent may do and is owed the
day something can ask it.

## Consequences

- **`Set::write_param` returns `Result<usize, CrossesAuthority>`.** Three call sites moved with it:
  `karakuri-cli`'s `build`, which prints the engine's sentence rather than a reworded one;
  `swap.rs`'s rebuild, which prints the same; and `Set::set_published`.
- **A rebuild's restatement can never meet the refusal, and that is deliberate.** `swap.rs` applies
  `Request::params` before `Request::authorities`, so every node is at `Authority::default` when the
  params land and the landing is uniform whatever the request goes on to grant. What a request
  restates is *where the operator left the controls* — a fact, not a new write — and a rebuild that
  re-asked permission for it would put every control back to the `.kir` default on the next save of
  any file.
- **Nothing in any run today can meet it either.** `Request::authorities` is empty in every run —
  `Watch` never fills it, and no surface writes a `Record::Authority` — and nothing calls
  `set_authority` outside tests, so every landing is uniformly `Manual`. **The check is live the day
  the first grant lands**, and it is a test rather than a comment that says so:
  `a_wildcard_write_is_refused_where_the_nodes_it_lands_on_disagree` grants one node and watches the
  bare name be refused, watches the values not move, and watches the addressed write still land.
- **`Set::set_published` answers `false` for a refusal, which is also its answer for a name nothing
  publishes, and that is the one place the sentence is lost.** It has no caller; the caller that
  arrives takes the `Result` `write_param` already returns, which is a change to that signature and
  not to the rule. Named here rather than left to be discovered.

  *(Closed in the commit that landed this record. It returns `Result<bool, CrossesAuthority>`: a
  published control is a wildcard unless the author named a node, so this was the route the decision
  was about, and leaving the sentence lost on it would have been leaving it lost where it mattered
  most. Having no caller is what made it ten `.expect` lines in two test files.)*
- **`Set::set_param` and `Set::set_param_at` stay public and unchecked.** They are the mechanism
  below the entry point and are reached only by this crate's tests; narrowing them would break
  `crates/karakuri-engine/tests/` without changing what any surface can do.
- **What this leaves undone is the asker.** Whether an *addressed* write by an agent onto a node the
  operator kept is refused — and what `Suggesting` means for a write that has nowhere to be proposed
  to — is not decided here, and cannot be tested until something in this workspace writes a parameter
  on an agent's behalf. When MCP grows that tool, the asker is what it has to carry, and this
  refusal is not it: a uniform landing of `Manual` nodes accepts an agent's wildcard today.
- **The manual changed with the code**: the *Write a parameter* row on
  [operations.html](../manual/operations.html) states the refusal and that an address is never
  refused, and [manual.md](../manual.md)'s `--param name=value` row says the same for the flag. Both
  say outright that nothing grants an authority yet, so nothing can meet this today.
