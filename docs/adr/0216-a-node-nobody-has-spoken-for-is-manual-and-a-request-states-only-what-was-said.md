---
id: 0216
title: A node nobody has spoken for is manual, and a request states only what was said
status: accepted
date: 2026-08-29
supersedes: []
superseded_by: []
principles: [0031, 0076, 0078]
tags: [engine, ui]
---

# A node nobody has spoken for is manual, and a request states only what was said

## Context

[ADR-0211](0211-authority-is-set-per-node-and-the-record-is-the-sessions.md) settled that authority
is set **per node of a Set** and that the record carrying it is the session's, and it named what was
still owed: the engine held neither the value list nor a place to keep one, so *"the chip has a
vocabulary to speak and no value to draw"*. Building that raised two questions ADR-0211 does not
answer, and neither of them could be deferred past the first line of code.

**The first is that ADR-0211 states no default.** The word does not appear in it. But a `Set` is
built from a procedure and a request, and every node it contains has an authority the moment it
exists, whether or not anybody has said anything about that node. Something had to be chosen, and
choosing it by picking the first variant would have been choosing it by accident.

**The second is what a request carries.** `swap::Request` restates **nine** things across a rebuild —
`layering`, `live`, `salts`, `camera`, `params`, `published`, `bindings`, `names` and `edges` — each
with the same reason written at it: *a request that depends on what happens to be live is not
reproducible from a record stream*. Authority is the tenth. Its shape is not obvious, because the
existing nine split into two: dense per-layer arrays (`names`, `salts`) and sparse lists applied
after the build (`params`, `published`, `bindings`, `edges`).

## Decision

**A node nobody has spoken for is `Manual`, and `Request::authorities` is sparse: an entry is
something an operator said.**

### Why manual, and not either of the other two

**Rule 06 of [the seven rules](../manual/index.html) decides it, and it decides it in one sentence
that was written about something else.** The rule is *"Each node of a Set is manual, suggesting, or
automatic, and you set that node by node"*, and its argument is *"There is no switch that hands the
whole instrument to an agent, because the useful arrangement is almost always partial."*

**Any default other than `Manual` is that switch**, thrown for every node of every Set, by nobody,
at build time. It is worse than the switch the rule refuses, because the switch at least has
somebody's hand on it. That is the whole of the argument and the other three reasons below only
agree with it.

- It is the only default that leaves
  [P-0078](../principles/0078-the-operator-wins-and-an-automatic-writer-yields-to-a-hand.md) — *the
  operator wins* — true of a freshly built Set without anybody having to act to make it true.
- `karakuri_operation::Authority` declares `Manual` first, and a vocabulary's order is not nothing:
  the crate was written to the manual, and this is the manual's order.
- The mock draws `man` selected on three of its four node heads and `sug` on the fourth. **This is
  weak evidence and is recorded as weak** — it shows one arrangement an operator might have made,
  not a default. It is listed because it agrees, not because it decides.

**Rejected: `Automatic`.** Hands every node of every Set to an agent at the moment it is built. It
would make the console's whole authority row a thing an operator has to go and undo before playing.

**Rejected: `Suggesting`.** Softer and the same shape. It makes a proposal stream the resting state
of material nobody asked to be proposed at, and P-0076 is why that is not a small thing: a surface
owns the affordance and never the authority, so there is no place for the console to quietly not
show them.

**Rejected: leaving it unrepresented** — `Option<Authority>`, with `None` meaning nobody has spoken.
Then *nobody has spoken* and *manual* are two spellings of one arrangement, which is exactly what
[P-0031](../principles/0031-a-name-means-one-thing-across-the-system.md) is about, and every reader
downstream would have to decide again what `None` means.

### Why the request is sparse

**An entry is a thing an operator said**, one per `Record::Authority`, and an empty `Vec` is a Set
nobody has spoken for whose every node lands on the default. A node the rebuild no longer has is
reported and passed over, on `Request::live`'s terms.

**Rejected: a dense per-node array**, the shape `names` and `salts` use. It needs a node count the
caller cannot know until the build has derived it, and it spells *nobody has said anything* and
*manual* as two different entries when they are one arrangement — the same fault as `Option`, one
layer along. The dense shape is right for `names` because every node has a name whether or not
anyone wrote one; it is wrong here because not every node has been spoken for.

### Where the value list lives

**`karakuri-engine` declares its own `Authority`**, and this is not a second spelling in P-0031's
sense — it is the fourth instance of a pairing this workspace already argued and documented.
`karakuri-engine` does not depend on `karakuri-operation` and `karakuri-operation` depends on
nothing at all by charter ([ADR-0180](0180-the-operation-vocabulary-is-a-crate-with-no-dependencies.md)),
because *a dependency there is a dependency each surface pays*. `Blend`/`BlendMode`, `Sync` and
`Residency` already exist in both crates, converted by one function per list in `karakuri-cli`'s
`mix.rs` — the only crate that sees both spellings, and the place the pairs are checked against each
other. Adding the dependency to save the fourth copy would invert an architecture already settled in
prose.

## Consequences

- **It inverts the symptom `docs/roadmap.md` describes, and the roadmap is corrected with this.**
  That file says a lost restatement *"hands a node back to an agent an operator had taken it from"*.
  With `Manual` as the default the dropped restatement destroys a **grant**: the node an operator
  gave to an agent is silently theirs again. The dangerous direction the roadmap named is only
  reachable if a rebuild *inherits* the previous build's value, which is a second property. **Both
  are the same defect** — the rebuilt Set stops reflecting what the operator said — and both are
  asserted.
- **The roadmap's "five fields" is corrected to nine**, counted rather than transcribed.
- **The console's `man / sug / auto` chip has a value to draw**, which is what ADR-0211 named as
  blocked. What is still missing is a writer, and **it is a narrower gap than *not yet wired***:
  `Record::Authority` is deliberately excluded from Set-file state in two places —
  `karakuri-cli`'s `setfile.rs` and `karakuri-store`'s `project.rs`, both saying that *"a Set file
  obeying one would hand the node over on every load"*. So the startup path will **never** seed this
  the way `--load-set` seeds `layering`, `live`, `camera` and `salts`. The writer, when it comes, is
  a live-session one, and `Request::authorities` is empty in every run until it exists. The field
  says that at itself rather than reading as unfinished plumbing.
- **`mix::authority` has no non-test caller yet** and carries
  `#[cfg_attr(not(test), allow(dead_code))]`, the narrowest form and the one
  `karakuri-engine`'s `node/camera.rs` already uses. Its documentation names the two readers it
  waits for — the chip, and a live save writing `Record::Authority` — and says the first of them
  deletes the attribute. The alternative was inventing a caller, which would have been this
  decision's scope growing to fill a warning.
- **`karakuri-operation`'s own documentation was made false by this and is corrected**: its list of
  the lists it copies now includes `Authority`, and `Authority`'s comment said *"`karakuri-engine`
  holds no copy of it at all yet, so for once this is not a duplicate"*, which stopped being true
  the moment this landed.
