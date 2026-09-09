---
id: 0318
title: The built-in camera's three placement numbers are parameter rows
status: accepted
date: 2026-09-09
supersedes: []
superseded_by: []
principles: [0084, 0086, 0087, 0090, 0092]
tags: [console, engine, camera, operations, setfile, m5.5]
---

# The built-in camera's three placement numbers are parameter rows

## Context

The camera was the last thing on this panel an operator could not touch. The Inspector drew its
group, its address and its authority chip, and under the head a line saying there was nothing
there:

> **no published parameters** — the orbit's six numbers are the Set's, not this node's

The vocabulary said the same thing one crate along, and said it as an open question:

> **The camera, and it is the one arm with no payload that can be written down.** There are two
> live answers and they are not the same operation. `Record::Camera` describes a built-in orbit —
> `{ kind, index, radius, speed }` — while L3 is a procedure layer whose params are written by
> `Operation::WriteParam` like any other node's. Which of those *is* "the camera" decides whether
> this arm carries three numbers or should not exist at all.

That is `Property::Camera(Undecided)`, and M5.5's *Element capacity, seeds, the camera* carried it
as the row's undecided third ([ADR-0314](0314-a-control-that-moves-a-field-of-the-aim-re-aims-the-slot-and-the-rebuild-is-the-write.md)
settled the other two-thirds and left this one where it was).

The maintainer answered it on 2026-09-09:

> 「Orbit の 3 値をパラメータ行として」

**The `Orbit`'s three values, as parameter rows.** Which three, and what that costs, is what this
record works out.

### What an `Orbit` is

`karakuri-engine/src/camera.rs` — six fields, and they divide three and three:

| | | |
|---|---|---|
| `radius` | `speed` | `height` |
| `fov_y` | `near` | `far` |

The first row is **where the camera is standing**: how far from the target, how fast it goes round,
how high above it. The second row is **the lens**: what the projection is. The maintainer's three
are the first row, and nothing else in the struct is a candidate — a `.kir`'s own `sweep` example
in [ir-spec.md](../ir-spec.md), *The `camera` block (L3)*, is this orbit written as a procedure and
declares exactly two of them, with the third hard-coded:

```
param radius : float [1.0, 40.0] = 8.0
param speed  : float [0.0, 2.0]  = 0.15

camera {
  let a  = t * speed * 6.2831853;
  eye    = vec3(cos(a) * radius, 2.0, sin(a) * radius);
  target = vec3(0.0, 0.0, 0.0);
}
```

### The state before this

- `Set::camera: Orbit` was a public field. A `camera` record, a `--from-set`, a rebuild's
  `swap::Request::camera` and eighteen of this crate's own tests all assigned it directly.
- The camera node's parameter map was **empty** — `n.map(declared_defaults).unwrap_or_default()`
  in `Set::build_many` — with the reason written beside it: *it is not a procedure and declares no
  params*.
- `Record::Camera` carried `radius` and `speed`. `setfile.rs`'s own module doc called this out:
  *"`camera` carrying two of the six fields the engine's orbit has is the one left that is a gap
  rather than a mismatch."*
- No flag, no key, no MIDI target and no MCP tool reached any of the six.

## Decision

**The built-in camera's `radius`, `speed` and `height` are parameters of the camera node.** The
engine declares them — the names, the ranges and the defaults — where a `.kir` would, because the
built-in is a node with no procedure behind it. Everything else follows from the parameter map
being populated, and **nothing was written to make any of it work**:

- The Inspector draws three rows under the `orbit` group, each a fader over its declared range
  (ADR-0286's shape), because `Set::published`'s default interface is what the pane is built from.
- A drag emits `Operation::WriteParam`, lands `Record::Ride`, and reaches `Deck::write_param` →
  `Set::write_param` — the route [ADR-0280](0280-a-parameter-written-to-a-live-set-is-a-session-record.md)
  laid.
- A replay reproduces it, because a `Ride` is what a replay reads.
- A rebuild keeps it, because [ADR-0282](0282-a-rebuild-inherits-the-values-somebody-moved-and-reads-the-rest-from-the-code.md)
  carries the values somebody moved and reads the rest from the code.
- A MIDI knob reaches it, because the three take positions in the published interface.
- A `bind` drives it, blended on confidence, because the frame path resolves a camera's parameters
  through the same `effective` every other layer goes through
  ([P-0084](../principles/0084-a-confident-wrong-automatic-judgement-is-worse-than-not-judging.md)).

**`Property::Camera(Undecided)` is deleted.** The two live answers were never two: the built-in
orbit's numbers *are* a node's params, so `Operation::WriteParam` is the operation and an arm
carrying three numbers would have been a second way to write a parameter.

**The lens three are not rows.** `fov_y`, `near` and `far` are what the projection is rather than
where the camera is, and `blend weighted` normalises every fragment's depth against the near and
far planes — so a fader on either would move how the picture composites while appearing to move the
camera. A control whose effect is not the one it draws is the thing this panel does not do. A hand
that wants them writes an L3, which declares whatever it likes and gets rows for all of it.

**Three ranges, and two of them are the specification's own.** `radius : [1.0, 40.0]` and
`speed : [0.0, 2.0]` are lifted verbatim from the `sweep` example above, so the built-in and the
simplest procedure that could replace it are the same camera to a hand. `height : [-40.0, 40.0]`
has no precedent — the example hard-codes 2.0 — and is derived rather than chosen: symmetric,
because looking up from underneath is as much a shot as looking down; and reaching as far either
way as `radius` does, past which the eye is further from the target than any radius could put it,
which is a distance and not a height.

**A bare name does not reach them.** *A bare name reaches every node that declares the key* is the
rule, and its reason is that two renderers' `exposure` is one knob — two authors wrote the same
word about the same idea. Nobody wrote the camera's three; the engine did, and `radius` is a word
**seven of this repository's own example procedures already use**, `drift_shell` among them. So a
bare `--param radius=3.0` would swing the camera as a side effect of moving a geometry, and the
only thing the two ever shared was a spelling. The three are reached by address —
`--param L3:0:radius`, a published control that carries its address, a `bind` that names `L3`, a
`Ride` with an `at` — and `Set::addressed_only` is the one place that is decided, with four
readers. It is deliberately not a general rule about engine-declared parameters: there is one such
node, and inventing the general case would be designing for a second one that does not exist.

**`Record::Camera` grows `height`.** A number a hand can move and a file cannot record is a knob
that walks back on its own, so the record carries the three it is about. `height` absent reads as
2.0, which is what every file written before this meant.

**The built-in camera writes no `param` records.** Its values are on the `camera` line, and a
node's number written twice in one file is two spellings of one fact. This is the `slot` rule one
level down — *the built-in camera has no `slot` record because it has no procedure to reference,
and the `camera` record describes it instead* — and the skip lives in `setfile::save`, which is
what knows the format, rather than in the two callers that hand it `Set::params` whole.

**`Set::camera` is private, and `Set::orbit` is the read.** The field is what was *stated*; the
parameter map is where a hand, a binding's blend and a carried ride leave their answer. An
assignment to the field now states half a camera and nothing would say so — the picture would go
on using the map and the save would write the field. `docs/contributing.md` §4's third tier is to
make the mistake unspellable, and eighteen call sites in this crate's own tests were the
demonstration that a comment would not have held it.

## Alternatives

**A dedicated camera operation carrying three numbers.** `SetProperty { property: Camera { radius,
speed, height } }`, or a `SetCamera` row of its own. It is what the `Undecided` arm was shaped for
and it loses on every count that matters: it needs a record, a `Deck` setter, an `apply` arm, a
MIDI target, a console control and a place in the published interface, all of which the parameter
route already has — and having built them, a camera's `radius` and a geometry's `radius` would be
two mechanisms that must agree about ranges, authority, rebuild carry and replay, and would stop
agreeing the first time one of them grew a refusal. The camera would also be the one node on the
panel whose knobs a MIDI controller could not reach, because a map line is learned against a
position in the published interface and this operation would have none.

**Leaving it undecided.** The record it would preserve is honest — the page said *this page
specifies what the bay shows and stops there* — but the question was answerable and the answer
was cheap. What made it look like a fork is that `Record::Camera` and a procedure's `param` are
two shapes for one fact, which is the thing to resolve rather than to choose between.

**The lens three as rows too, making six.** Rejected above: `near` and `far` are read by the
weighted blend, so a fader on either changes compositing while drawing as a camera move. It is
also not what the maintainer asked for.

**Distinct key names — `orbit_radius` and the like — instead of the addressed-only rule.** It
removes the collision by construction and needs no rule at all. It loses because the names would
no longer be the `Orbit`'s: the ir-spec example's `radius` and `speed` are what a procedure
replacing the built-in declares, and a hand that learned `orbit_radius` on the built-in would have
to learn `radius` again the day a `.kir` took its place.

**Renaming the operations row to *Element capacity and seeds*.** The heading names three things and
the operation now sets two, which is the drift `docs/contributing.md` §4 is written against. It
loses on balance: *the camera* is the word an operator comes to that row looking for, so the page
answers where it went at the place they look; and the rename would orphan the heading as
[ADR-0180](0180-the-operation-vocabulary-is-a-crate-with-no-dependencies.md) and ADR-0314 quote it,
for a sentence the tip already carries. It stays a one-line change if a reader disagrees, and the
tip says so.

## Consequences

Checked against the tree, file by file.

- **`docs/manual/console.html`** — the Inspector's note is *The camera is a node, and three of its
  six numbers are rows*, and it says which three, why the other three are not, and that a camera
  saved is the camera that comes back. Both mock panes draw three rows under `L3:0 orbit` in place
  of the `node-note`, and the ords after them are renumbered — deck A's `hue` is 8 and its
  wildcard `exposure` 9, deck B's `size` is 6. The Library bay's *5 nodes* tip no longer says the
  inspector draws no rows under `orbit`.
- **`docs/manual/operations.html`** — *Write a parameter*'s tip says the camera's three are written
  by it. *Element capacity, seeds, the camera* is *one row and two operations*, its tip says where
  the camera third went, and its badges are unchanged. The heading is unchanged, so
  `the_manual_and_the_vocabulary_agree` still compares equal strings.
- **`docs/ir-spec.md`** — *Several cameras* states the three keys, the three ranges and why the
  lens three are not among them; the `camera` bullet gains `height` and the rule that the built-in
  writes no `param` records; both sample files carry `"height":2.0`.
- **`docs/manual.md`** — *Without an L3 the camera is a slow orbit* gains the three flags, their
  ranges, and the sentence that a bare `--param radius` moves the geometry that declared it and
  not the camera.
- **`crates/karakuri-engine/src/camera.rs`** — `Orbit::PLACEMENT` is the declaration table;
  `placement`, `set_placement`, `placement_values`, `placement_ranges` and `with_placement` are
  the accessors. The ranges' provenance is written at the constant.
- **`crates/karakuri-engine/src/node/camera.rs`** — `param_keys` returns the three for the
  built-in, out of one `OnceLock`; `is_builtin` is the question the Set asks.
- **`crates/karakuri-engine/src/set.rs`** — the build seeds the camera node's values and ranges;
  `addressed_only` is the bare-name rule with four readers (`set_param`, `landing`,
  `declared_range`, `published`); `published` emits the three addressed, in node order;
  `aim_camera` and `orbit` are the writer and the reader; `camera` is private; `value_at` is
  public because two controls may now share a name.
- **`crates/karakuri-engine/src/swap.rs`** — the install calls `set.aim_camera(request.camera)`,
  and the comment saying the order was not a contest says that it is one now.
- **`crates/karakuri-store/src/record.rs`** — `Record::Camera` carries `height`, defaulting to
  2.0 through `default_camera_height`, which names the copy it is of.
- **`crates/karakuri-environment/src/setfile.rs`** — writes and reads `height`; skips the built-in
  camera node in the `param` run; `Saving::camera` says it is `Set::orbit`.
- **`crates/karakuri-operation/src/lib.rs`** — `Property` has two arms and its doc carries the
  argument for the heading it no longer fully describes.
- **`crates/karakuri/src/main.rs`, `crates/karakuri-cli/src/main.rs`** — the save gathers
  `set.orbit()`; the `camera` record is applied through `set.aim_camera`; the Inspector reads a
  control's value by its address rather than by its name.
- **`Set::landing_of` is new, and it closed the one thing this decision broke.**
  `karakuri/src/main.rs`'s `node_of` puts a published control into a node group, and a wildcard
  over two or more nodes belongs to several at once and is drawn in none. It worked that out by
  walking `Set::params` and counting the nodes holding the key — the same walk `Set::write_param`
  refuses on, agreeing with it by coincidence rather than by construction. The day the camera
  declared a `radius` the two stopped agreeing: the engine saw one landing and the panel saw two,
  so the **geometry's** `radius` row was dropped as belonging to several groups and
  `gpu::a_pane_reads_a_running_set` failed with *a published control lost its group* — ord 2
  missing from a run of 29. `node_of` asks `Set::landing_of` now, so the rule lives where the
  write is decided and a surface asks for it
  ([P-0090](../principles/0090-a-surface-offers-it-never-decides.md)). **The row the pane lost
  was not the camera's**, which is the part worth keeping: a surface re-deriving an engine rule
  fails somewhere else.
- **Nothing was added to `karakuri-console`.** The pane is built from `Set::published`, so the
  three rows arrived in the Inspector with no console change at all, which is the clearest
  statement of what this decision is.
- **Tests.** `karakuri-engine/tests/camera.rs` gains four, each watched to fail against an
  injected defect run on its own: the three are declared at the aimed values; they publish
  addressed, in order, over the stated ranges; a write to `L3:0:radius` moves the picture; a bare
  `radius` reaches nothing; and a rebuild keeps a ridden radius while the lens comes from the
  restatement. **Seven existing tests moved**, and each moved in the same direction — a list that
  said the camera's map was empty now says what is in it, and a count of what a bare name reached
  now says which nodes it did *not*: `tests/camera.rs`'s two addressing tests,
  `tests/binding.rs`'s two-nodes-one-name, `tests/publish.rs`'s three (the default interface, its
  order, and the count a refusal leaves behind) and `set.rs`'s own
  `a_wildcard_write_is_refused_where_the_nodes_it_lands_on_disagree`. They are sharper for it: an
  off-by-one in `slot_of` now lands the orbit's `radius` on a renderer rather than landing
  nothing. `karakuri-environment` gains the two file claims — the `param` run leaves the built-in
  alone, and a `camera` line with no `height` reads as the engine's own default, which is what
  holds the restated 2.0 in `karakuri-store` to `Orbit::default()` — and its round trip carries a
  height. **`karakuri/src/main.rs`'s `gpu::a_pane_reads_a_running_set` is the eighth**, and it is
  the one that caught the `node_of` defect above rather than being adjusted for it: its doc now
  says that three of this deck's controls are addressed, and that both halves of `node_of` are
  exercised on one key — the wildcard `radius` landing on the L1 alone and the addressed one on
  the camera. Every one of the six new tests, and that one against the walk it replaced, was
  watched to fail against an injected defect run on its own.
- **What is not closed.** The lens three are still unreachable from any surface and unrecorded in
  any file, which is now a statement rather than a gap: nothing moves them, so a save has nothing
  about them to lose. A camera *procedure*'s parameters were already ordinary and are untouched.
