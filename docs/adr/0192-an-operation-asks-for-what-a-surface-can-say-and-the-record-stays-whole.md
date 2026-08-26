---
id: 0192
title: An operation asks for what a surface can say, and the record stays whole
status: accepted
date: 2026-08-26
supersedes: []
superseded_by: []
principles: [0074]
tags: [vocabulary, midi, mixing]
---

# An operation asks for what a surface can say, and the record stays whole

## Context

The vocabulary landed with one variant for the output look
([ADR-0180](0180-the-operation-vocabulary-is-a-crate-with-no-dependencies.md)):

```rust
SetLook { tonemap: Tonemap, exposure: f32 }  // "Tone map and exposure"
```

and it said why, at the variant:

> One variant for two things because `karakuri_store::record::Record::Look` is one record for two
> things, and for its stated reason: a stream that set the exposure without saying which operator
> it applies to would be describing a look nobody can reconstruct.

**That reason is true, and it is a reason about the record.** A record is what a replay
reconstructs a session from: `Record::Look` carries the operator, the exposure and the white point
together because a decoder reading the stream has nothing else to read, and `karakuri-cli` replays
one by handing all three to `Present::set_tonemap` in a single uniform write. An operation is a
different thing at a different layer — it is what a surface *asks for* — and the layer that turns
one into the other already knows the current look. `karakuri-cli` has done exactly this since
before `karakuri-operation` existed:

```rust
fn set_exposure(&mut self, exposure: f32) {
    let look = Look { exposure: clamp_exposure(exposure), ..self.look };
    self.record(mix::look_record(&look));
```

The `..self.look` is the whole argument. The operator is not *asked for* by the `-` key; it is
*filled in* by the place that makes the record, from the look that is running. Reading the record's
requirement back onto the operation put a value into the ask that no surface asking for it has.

**What forced it now.** A control change turns exposure alone, and `karakuri-midi` is a crate with
no engine, no state and no readback by charter
([ADR-0180](0180-the-operation-vocabulary-is-a-crate-with-no-dependencies.md)) — `Map::action` is a
pure function of one message. The map has a target for this and the manual marks the route as
built: `cc 20 -> exposure`, `Target::Exposure { range }`, `Action::Exposure { .. }`. Under
`SetLook`, that route **could not become an `Operation` at all**: the map would have had to produce
a `Tonemap` it has no way to know. The migration of `karakuri-midi`'s `Action` onto the vocabulary
is blocked on exactly this row.

The console settled the same seam from the other end for a fader
([ADR-0185](0185-a-fader-translates-a-drag-into-an-operation-and-applies-nothing.md)): the surface
emits an operation and applies nothing, and the harness is what makes the record.

## Decision

Two variants, two rows:

```rust
SetTonemap  { tonemap: Tonemap }  // "Tone map"
SetExposure { exposure: f32 }     // "Exposure"
```

`t` translates to the first, `-`, `=` and `` ` `` to the second, `--tonemap` and `--exposure` to one
each, and `cc -> exposure` to the second — which it can now say, because the second asks only for
what a control change carries.

**`Record::Look` does not change and its reason still stands.** It is still one record for two
things — three, with the white point — because a stream that set an exposure without naming the
operator would describe a look nobody can reconstruct. Nothing in `karakuri-store`,
`karakuri-engine` or `karakuri-cli` moves. What changes is that the vocabulary stops copying a
record's completeness requirement into an ask, which is the layer confusion this corrects.
`white_point` stays out of the vocabulary for its own unchanged reason: it is in the record, and it
has no control on any surface and no row on the page.

## Alternatives rejected

- **Let the map line name a tone map: `cc 20 -> exposure aces`.** The map would then satisfy the
  variant with no readback, which is why it is tempting. It also means an operator re-asserting a
  transfer sixty times a second — every message the fader sends carries `aces` whether or not
  anything asked for it, so nudging exposure silently overwrites a tone map somebody chose with `t`
  a moment earlier. A route that undoes another route as a side effect of its own payload is worse
  than the route not existing.
- **Give `Map::action` a readback argument** — pass the current look in, fill the operator from it.
  This is the shape that ends `Map::action` being *a pure function of one message*, and that purity
  is the crate's whole test story: every map test is `parse` then `action`, with no world to set up
  and nothing to mock. It would also make `karakuri-midi` the second place that knows how a look is
  assembled, which is the knowledge the vocabulary exists to keep in one place.
- **Leave a translator in `karakuri-cli`**: let MIDI keep emitting `Action::Exposure` and have the
  CLI turn that into `SetLook` by filling the operator in. This works today — it is what the code
  already does — and it is precisely *not doing the migration*. `karakuri-midi` would go on not
  emitting an `Operation`, and the first rule would go on being satisfied by two lists agreeing.
- **Split only the ask and keep one title on the page.** The page is the specification and the test
  matches by title; two variants under one heading is the duplicate-title failure
  `no_two_operations_share_a_title` exists to refuse. The row splits, and the count line moves with
  it.

## Consequences

- **46 operations, not 45.** Recounted from the page rather than adjusted by arithmetic, the way
  [ADR-0186](0186-one-operation-names-one-of-three-residencies.md) did it: 46 `<h3>`s, and 50 built
  of 200 `class="rt …"` badges. The old row's five badges become ten — `t` and `--tonemap` to the
  tone map, `-`, `=`, `` ` ``, `cc -> exposure` and `--exposure` to the exposure, `panel transport` designed
  on both and MCP empty on both — so two more ways in are built and five more exist.
- **The two floors move to 46**, in `karakuri-operation/src/lib.rs` and in
  `tests/the_manual_and_the_vocabulary_agree.rs`. Each says what it read when it landed and now
  carries all three readings, because a floor that forgets it was lowered is a scan that can come
  back short. The manual's count line, `README.md`, `architecture.md` and the roadmap all state 45
  and all moved.
- **`karakuri-midi`'s migration is unblocked**, and nothing has migrated. `Action::Exposure`,
  `Target::Exposure` and the CLI's key handler are untouched; this changes the target they move to,
  which is the same order ADR-0186 took.
- **ADR-0180's bullet is now history rather than a rule.** *"Tone map and exposure is two-in-one and
  stays one variant"* was right when it landed and is not edited
  ([P-0066](../principles/0066-an-adr-is-a-description-of-history-corrected-but-never-revised.md));
  this record is where it stopped being the decision in force.
- **No new principle.**
  [P-0074](../principles/0074-an-operation-says-what-it-wants-never-which-way-to-move.md) is
  untouched — this is not about direction. The rule this states is narrower and lives in the
  vocabulary's own prose at the two variants: an operation carries what a surface can say, and the
  translator completes the record.
