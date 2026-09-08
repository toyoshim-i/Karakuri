---
id: 0301
title: A model's star is refused, because a favourite has no sandbox to land in
status: accepted
date: 2026-09-08
supersedes: []
superseded_by: []
principles: [0096, 0083, 0090]
tags: [library, store, mcp, operations, console]
---

# A model's star is refused, because a favourite has no sandbox to land in

## Context

[ADR-0299](0299-my-sets-is-the-starred-subset-and-the-star-is-kept-beside-the-sets.md) made `my sets`
the **starred subset** of the library and put the mark in `<store>/favourites.json`, beside the Sets.
It settled where the value lives and what the operation is —
`Operation::SetFavourite { id, favourite }`, `Standing::Open`, `Written::Silent(Silent::Surface)` —
and it did not say what happens when the caller is a **model**.

That question is not open, and the shape it is answered in was decided before it was asked.
[P-0096](../principles/0096-the-operators-library-is-written-by-an-operators-own-act.md) is the rule:
*the operator's library is written by an operator's own act — `--save-set`, the `k` key, a control
they pressed, and any route their hands reach next*. And
[ADR-0261](0261-a-model-asked-save-lands-in-a-sandbox-because-the-operators-library-is-the-operators-own-act.md)
is how that rule met a model asking for a save: **not a closed door**, but a second directory —
`<store>/sandbox/` — with `Operation::SaveSet` left `Standing::Open` and `gate.rs` untouched, because
*"refusing the operation would leave a session of live edits with nothing kept anywhere, which is the
loss this rule exists to prevent, taken from the other side."*

**`my sets` is by construction the list of Sets the operator chose.** That is the whole of what
ADR-0299 decided it was, and it is the answer to the roadmap's symptom — a `my sets` filling up with
things the operator did not make. A model's star reaching `<store>/favourites.json` would put things
there that the operator did not choose, which is that symptom arriving through a different door.

So the actor decides, as it does for a save. What is left is the question ADR-0261 answered with a
directory: **where does a model's star land instead?**

## Decision

**A model's star is refused, out loud, and the refusal says the list is the operator's own and where
the Set is.** The performer branches on `karakuri_environment::Asked`, exactly as
`karakuri_environment::filed_as` branches for a save; `favourite()` in `crates/karakuri/src/main.rs`
is that performer, and `Asked::Operator` is what the panel's own press passes.

**`Standing::Open` stays and `gate.rs` is untouched**, which is ADR-0261's own arrangement: an
operation that is open is open to every control channel (rule 01), the refusal is the *performer's*
and not the gate's, and a model's call is answered rather than turned away at the door.

**The MCP badge stays `plan`.** A model can name a Set id, so `gap` — *"this surface cannot reach it
at all"* — would be a lie about what is reachable rather than a statement about what is allowed. The
row is owed a tool; what that tool answers is this record.

## Why not a sandbox, when a save gets one

**ADR-0261's sandbox works because what lands in it is material.** A `<store>/sandbox/<id>.kbset` is
a file with content: an explicit edit-history snapshot, *"the thing an operator goes looking for
after a show when a model has been editing live"*. It can be listed, read, loaded and packaged. The
model's work survives, which is why refusing the save would have been the greater loss.

**A star is one bit, and its whole meaning is a view of the operator's library.** *This id appears
under `my sets`* — nothing else. So `<store>/sandbox/favourites.json` would be:

- **read by nothing.** No scope lists it, no tool answers it, and no route in this program looks at
  it. Giving it a scope would be drawing a second favourites view whose contents are a model's
  opinion of somebody else's library, which is a bigger decision than the one being avoided.
- **indistinguishable from having done nothing**, from the operator's side. There is no artifact to
  go and find afterwards, because there was never any content — where the sandbox Set is exactly
  that artifact.
- **a success reported for an act that had no effect.** That is the failure
  [P-0083](../principles/0083-a-refusal-carries-what-the-next-attempt-needs.md) is written against
  from the other end: a call answered `ok` carries nothing the next attempt needs, and a model that
  believes it starred a Set will go on to say so to the person beside it.

**So the honest answer is the refusal, and it is a useful one.** It names the id, says `my sets` is
the list of Sets the operator chose, and says where the Set actually is — under `all` in the Library
bay, with the mark at the left of its row. That is the shape the class pills' refusals already
take: *"a model that hits the wall mid-plan can say which pill a person has to press instead of
reporting an instrument that cannot do the thing"* (`console.html`). Nothing is lost by refusing,
because there was nothing to keep.

## Alternatives rejected

- **Let a model write `<store>/favourites.json`.** It is the roadmap's own symptom through another
  door: `my sets` fills with rows the operator did not choose, and ADR-0299 exists to stop exactly
  that. P-0096 decides it on the actor, and this is the operator's library by every reading of that
  table.
- **A sandbox favourites file.** Above: a list nothing lists, no artifact to find, and a success
  reported for an act with no effect.
- **Refuse it at the gate, with `Standing` narrowed or a `Class` added.** ADR-0261 declined to close
  that door for a save and the reason carries: an operation that is open is open to every channel,
  and *which* directory a write lands in is the performer's question rather than the gate's. A gate
  that refused this would also have to say why a save is not refused, and the difference between
  them is not a class of operation — it is whether there is a second place worth writing to.
- **Silently do nothing and answer `ok`.** P-0083, and it is the worst of the four: it is the
  sandbox's failure mode with the file removed.
- **Let a model star only Sets it saved itself.** A model's saves land in `<store>/sandbox/` and
  `favourites.json` is a list of `<store>/sets/` ids, so the set of Sets this would permit is empty
  today. A rule that permits nothing is a refusal with an argument to maintain.

## Consequences

- **`favourite()` takes an `Asked`**, and the `Asked::Model` arm is written before any route reaches
  it — which is `arrangement()`'s own position, and the reason is the same: a control that arrives
  at this function finds the rule already here rather than adding one.
- **Nothing in `karakuri-operation` or `karakuri-operation-record` moves.**
  `Operation::SetFavourite` keeps `Standing::Open` and `Written::Silent(Silent::Surface)`, and the
  40 closed / 24 open split ADR-0299 recorded is unchanged.
- **The store is untouched.** `Store::set_favourite` knows nothing about who asked, exactly as
  `Store::write_set` does not: the actor is the caller's, which is what keeps P-0096 a rule about
  routes rather than a check inside a directory.
- **The refusal is a sentence and not a `StoreError`.** It is decided before the store is opened,
  so a model's star costs no file read at all.
- **This is a decision and not a principle.** P-0096 is the rule; what this settles is one control's
  answer under it, and ADR-0249's gate is not met — reading this record settles no question outside
  the star.
