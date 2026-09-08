---
id: 0282
title: A rebuild inherits the values somebody moved and reads the rest from the code
status: accepted
date: 2026-09-08
supersedes: []
superseded_by: []
principles: [0092]
tags: [engine, params, swap, watch, m5.4]
---

# A rebuild inherits the values somebody moved and reads the rest from the code

## Context

[ADR-0280](0280-a-parameter-written-to-a-live-set-is-a-session-record.md) gave a knob a route into
a live Set — `Deck::write_param`, recorded as `Record::Ride` — and left the next question open in
its §6: **what a rebuild does with the value.** The answer it left open is the maintainer's, and it
is one sentence:

> コードが持つデフォルト値はコードの値を見ても、リアルタイムのパラメータはライブから引き継ぐのが自然では？

A **declared default** comes from the code, because that is what the author just wrote. A **value a
knob moved** carries over from the live Set, because that is what the operator's hands are on.

§6 concluded that this was *"a change to what a rebuild **is**"*, and the reasoning went through the
plumbing that happened to be there: `Set` cannot tell a declared value from a moved one, telling
them apart is what `Watch::overrides` already does, `Watch` lives on the build worker, so the frame
loop can only reach it down a channel.

The maintainer's reply is the whole of this record's context:

> 単にデフォルトと現在値を覚えとくだけとは思うんだが。そんなに大袈裟かね。

**The information was already being computed and thrown away.** `declared_defaults` reads a node's
declarations into the map `Set::params` starts life as; from the next line on, nothing remembers
that those numbers were declarations. §6's own fact — *"`Set` holds `params` and `ranges` and no
declared defaults"* — is true, and it is a statement about what is *kept* rather than about what is
*known*.

## Decision

### 1. `Set` remembers which of its values somebody stated

One field, `Set::moved`: a `Vec<HashSet<String>>`, one entry per node, in the same node order as
`names`, `params`, `ranges` and `authorities` — built from the same walk, beside `authorities` and
in the same shape, so a node's marks cannot end up at a different index from its values.

A Set fresh out of `build_many` has nothing marked, because every number in `params` came from
`declared_defaults`. `Set::set_param` and `Set::set_param_at` mark as they write, and they are the
only two places a value in `params` ever changes: every route in — a `--param`, a `param` record out
of a Set file, a `ride` into a live Set, a rebuild's own restatement — goes through one of them. So
the mark means **something other than the declaration put this number here**, and never *which
surface did*. A binding is not a writer and does not appear: it blends *from* a param's value.

### 2. A written flag, and not a second copy of the declared values

The maintainer's phrasing is *remember the default and the current value*, and the comparison it
implies — a value is moved where it differs from the remembered default — was weighed and lost.

**It costs the same and answers wrong in one place.** Both shapes are exactly one new collection per
node; neither is cheaper. The comparison cannot hear a value stated as the number the declaration
already held:

- `--param exposure=0.5` against a `.kir` declaring `0.5` would survive a rebuild or not according
  to whether two floats coincide. That is a numeric accident deciding an operator-visible
  behaviour, silently, which is the class of failure this repository writes against.
- An operator who rides a knob back to where it started has said something, and the comparison
  reads it as never having touched it — so the next edit of that declaration moves it.

A flag is exact, and exactness is what the rebuild is deciding with. **Nothing else wants the
declared values**: `karakuri_environment::meta` writes a knob's default out of
`karakuri_ir::Param::default_scalar` — the declaration itself — so a defaults map on `Set` would
have had one reader, been equal to a rule with a known hole, and been a second home for a number the
IR already holds.

### 3. The rule, at the install

`Set::carry_moved_from(&Set)`, called in `HotSwap::install_if_ready` on the frame the candidate goes
in — the one moment both Sets are in one hand. For every value the outgoing Set has marked, at the
`(layer, index)` address `ParamWrite::at` spells: write it into the incoming Set unless the incoming
Set has already marked that key.

**A value this build states beats an inherited one**, and that clause is what separates a *rebuild*
from a *load*. A slot pointed at a Set file states every declaration of every node, because that is
what a live save writes; an operator who loads a preset over a slot they have been riding asked for
the preset. Without the clause, the load comes up wearing the ride.

**Not in `HotSwap::install`.** That is the replay path, and a replay has no operator to inherit
from: it builds from the record stream and the `ride` records land at the frames they were made at
(ADR-0280 §7). Inheriting there would be a second, unrecorded source of the same value.

### 4. `Watch::overrides` shrinks: stated on the build an aim causes, and on no other

`Request::params` was restated on every rebuild, on the terms the salts, the camera, the fold, the
edges and the authorities still are. **It is the one of them the engine does not re-derive.** A salt,
a camera, a fold and an edge all come back at some default the moment a request stops naming them;
a parameter value does not, because the outgoing Set is holding it and now knows which of its values
somebody stated.

Restating it was not merely redundant, it was the defect at its widest. A slot filled by
`--load-set`, or by the panel's library, carries **every declaration of every node** in `overrides`,
because `playing_values` writes them all. Restating that list put every parameter back to the file
on the next save of any `.kir` in the slot — so on the panel's normal mode there was nothing an
operator could do to a live Set that survived one.

The aim's own build still states them, and has to: an aim describes a Set that does not exist yet,
so there is nothing to inherit from, and it is the same statement that makes a load beat a ride.
`Watch::new` therefore starts `stated: true` — its caller built the slot's first Set with those
values and handed it over live — and `Watch::repointed` sets it back to `false`.

### 5. What the rule answers, case by case

- **A name the new Set does not declare**: lands nowhere. Not said out loud — the author deleted the
  `param` line in the file that caused this rebuild — and `carry_moved_from` returns the count a
  caller with something to say would say it with.
- **A name that is new**: never marked in the outgoing Set, so it comes up at its declaration.
- **A declared range that moved under a carried value**: nothing is clamped and nothing is refused.
  `Set::ranges` is the console's and the agent's; no uniform write is checked against it, and a
  carried value out of range is exactly as legal as `--param radius=9.0` already is.
- **A type that changed, including ADR-0268's component keys**: a change of *keys*, not of values. A
  `float glow` that became a `vec3` stops declaring `glow` and starts declaring `glow.x`, `glow.y`,
  `glow.z`, so the moved `glow` lands nowhere and the three components arrive as declared.
- **A node added or removed**: the carry is addressed by `(layer, index)`, so a bare key can never
  re-land on a different node, and a node this build no longer has is passed over — `Request::live`
  and `Request::authorities`' terms exactly. A `--watch` rebuild recompiles a fixed list of files and
  the node order is that list, so the addresses hold across one; a build that changes the material is
  a load, and a load states its own values.

## Alternatives

### a. Remember the declared defaults and compare

The maintainer's own phrasing, and the obvious reading of it. Rejected in §2 above: same cost, and a
value stated as the number the declaration already held is invisible to it.

### b. Leave `Watch::overrides` restated on every rebuild

Purely additive: the carry then fills in only the keys nothing stated, and no existing behaviour
changes at all. Rejected because the keys nothing stated are the ones nobody is riding. A slot with
a `--param`, and every slot the panel loads from its library, states its whole parameter list — so
this variant fixes the case that was never really broken and leaves the operator's hands exactly
where they were.

### c. Feed the watcher from the live write, down a channel

ADR-0280 §6's first home. A new message type and a send per ridden control per frame on the render
thread, to put a fact on the build worker that the render thread already has in front of it. It also
keeps `Watch`'s list as the answer to *which values are not the code's*, which is a second home for
something `Set` can say about itself.

### d. Carry every value by name

What §6 correctly calls not implementable: `params` holds one number per key and, before `moved`,
no memory of where it came from, so this carries the **outgoing declaration** forward and an author
editing `param radius = 2.0` to `5.0` sees nothing change. It is the failure the first of the tests
below reproduces on demand.

## Consequences

- **A ridden knob survives a rebuild.** ADR-0280's last consequence — *"A ridden knob still does not
  survive a rebuild, by §6"* — is closed, and `docs/roadmap.md`'s *One thing to know before drawing
  the first control* no longer holds: a parameter control owes a writer into the Set and no longer
  owes one into the watcher beside it.
- **One case is given up, and it is real.** An override for a name a rebuild dropped survived in
  `Watch::overrides` and re-landed when the name came back; a Set that never held it has nothing to
  carry. It is reachable — delete a `param` from a `.kir`, save, put it back, save — and it is the
  price of the rule that a value belongs to the Set that is holding it. Deleting a declaration and
  restoring it now restores the declaration.
- **`--param` and a Set file's `param` records are *moved*, not *declared*.** They are neither the
  author's declaration nor a knob, and the line drawn is that the code's declaration is the only
  thing that is the code's: **everything else that put a number into the Set came from outside it and
  carries.** Concretely, this is also what today's behaviour already was, so nothing an operator
  relied on moves. The visible consequence is the other way round: for a slot loaded from a Set
  file, an edit to a `.kir`'s declared default still shows nothing, because the file stated that key
  — which is what loading a file means.
- **A live save is unchanged and still writes every value**, moved or declared. `moved` is a fact
  about a running Set and not Set-file state, on `Record::Authority`'s terms: a file carrying it
  would decide, wherever it was next loaded, which of somebody else's knobs were being ridden.
- **A frame that swaps allocates a `String` per carried key**, once per swap, on the frame that
  already resizes render targets.

## What was watched fail

Each of the five below was run against its own injected defect, alone.

- `hot_swap::gpu::an_edited_declaration_lands_on_a_value_nobody_moved`, with the carry reading
  `outgoing.params[slot].keys()` instead of `outgoing.moved[slot]` — alternative **d**, and it
  prints `left: Some(2.5) right: Some(5.0)`: the author's edit to 5.0 arriving as the outgoing Set's
  2.5, which is §6's argument reproduced as a failure.
- `hot_swap::gpu::a_ridden_value_crosses_a_rebuild_and_a_declared_one_does_not`, with the
  `carry_moved_from` call removed — `left: Some(1.0) right: Some(3.25)`, the ride walked back to the
  declaration. That is the defect this record closes.
- `hot_swap::gpu::a_value_the_request_states_beats_the_one_the_operator_moved`, with the
  already-marked clause deleted — `left: Some(3.25) right: Some(0.125)`, a load coming up wearing
  the ride.
- `hot_swap::gpu::a_dropped_name_is_gone_and_a_new_node_starts_at_its_declaration`, with the carry
  going through `set_param` (bare) rather than `set_param_at` — `left: Some(3.25) right: Some(0.5)`,
  a ride on `L4:0` landing on a renderer that did not exist when it was made.
- `watch::tests::the_values_a_slot_was_aimed_with_are_stated_once`, twice: always restating
  (*"a save restated the values the run was started with"*) and never restating (`left: 0 right: 1`,
  the aim's own build with nothing to say).
