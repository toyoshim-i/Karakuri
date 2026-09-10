---
id: 0328
title: The Inspector's deck head steps a slot's capacity and re-salts it, and the payload loses its node
status: accepted
date: 2026-09-09
supersedes: []
superseded_by: []
principles: [0085, 0087, 0090, 0091, 0092]
tags: [console, engine, operations, watch, m5.5]
---

# The Inspector's deck head steps a slot's capacity and re-salts it, and the payload loses its node

## Context

*Element capacity, seeds, the camera* was the last `plan` badge in M5.5 and the last row this bay
was blocked on. Its camera third was answered on 2026-09-09
([ADR-0318](0318-the-built-in-cameras-three-placement-numbers-are-parameter-rows.md)) and turned
out to be no operation at all. What was left was two thirds of one row, and
[ADR-0314](0314-a-control-that-moves-a-field-of-the-aim-re-aims-the-slot-and-the-rebuild-is-the-write.md)
had already established that the **mechanism** was not what stood in the way:

> `Aiming::changed(|at| …)` … applies the change to the aim this program is holding … and sends
> `restated(&self.at)`.

A capacity and the salts are fields of that aim, exactly as the layering is. So ADR-0314 wrote
*nothing is blocked and nothing is drawn*, and named the two things that decided what would be
drawn:

> **The obvious affordance is refused by cost, and the second one lies.** A capacity is a number in
> a declared range … which on this panel is a parameter row's track … But a parameter track dragged
> is a uniform write a frame and a capacity track dragged is a **rebuild** a frame … And
> `Aim::capacity` is one `Option<u32>` for the **whole slot** where `Property::Capacity` addresses
> a **node**, which is ADR-0228's own recorded limit.

Both of those are answered here, and neither is answered with a new idea.

**The affordance already exists one bay over.**
[ADR-0262](0262-a-library-filter-field-steps-through-what-the-store-already-holds-rather-than-taking-letters.md)
met the same question for the Library's two filter fields — a value a hand should not type, over a
list somebody else owns — with a field that **steps** a closed list and an operation that names
where the step arrived. `TransitionRow::shape` and `Mixer::blend` are the same affordance two bays
further along. Nothing about it had to be invented for a capacity; what had to be decided is what
the rungs are.

**The address was a mismatch in the vocabulary rather than a limit of the mechanism.**
`Property::Capacity { node: NodeAt, elements: u32 }` and `Property::Seed { node: NodeAt, salt: u64 }`
had **no constructor anywhere in the workspace** — the row's only route was `--capacity`, which is a
flag and builds no `Operation`. So the node was a payload nothing filled and nothing read, and the
`u64` was twice the width of every field it could land in.

## Decision

### 1. The capacity is a chip in the deck head that steps the powers of two inside the declared range

`DeckHead::resized` answers `Operation::SetProperty` carrying `Property::Capacity { elements }` —
the number the step arrived at, never a step, because there is no step in the vocabulary to name
([P-0090](../principles/0090-a-surface-offers-it-never-decides.md)). `resized` in
`crates/karakuri/src/main.rs` sits beside `composited` and does the same thing with one field
instead of another: `Aiming::changed(|at| at.capacity = Some(elements))`, the worker recompiles the
slot off the render thread, and the Staging lane carries the verdict.

- **A drag is what is refused, and a step is what replaces it.** `ParamGrip` turns a pointer into a
  value every frame it moves; for a parameter that is a uniform write and for a capacity it is a
  full rebuild with every element buffer in the slot reallocated, dozens of them across one gesture
  ([P-0091](../principles/0091-cost-is-known-before-it-is-paid.md)).
- **The rungs are the powers of two, and the range is the material's.** `capacity_ladder` in
  `crates/karakuri/src/main.rs` folds every geometry's declared range to the part all of them accept
  — `lo.max(min)`, `hi.min(max)`, which is `declared`'s own arithmetic one bay over where two nodes
  publish one key — and keeps the powers of two inside it. **The wrap goes through the bottom and
  not through unset**, which is where this differs from ADR-0262's fields: a filter has a state that
  is *not narrowed* and a capacity has no such state.
- **The step is the next rung *above* what the slot is running**, not the next along a list. A
  procedure may declare `capacity [4096, 1048576] = 81920` and a Set file may record anything in
  range, so a `position` lookup would have fallen off the ladder and taken such a slot from 81920 to
  4096 on a press that reads as one step.
- **The chip is lit when the slot is not on what its material declares.** That is a reading of what
  **landed**, like every other readout on this row; *the aim carries a number* was the alternative
  and is a reading of what was **asked** — a hand stepping round to the declared default would leave
  the chip lit over a slot running exactly what its files say.
- **A deck whose geometries share no declared range has an empty ladder**, and the chip is drawn and
  claims nothing — the inert scrub's arrangement one control along.

**`Set::declared_capacities` is new, and it is the one thing this decision added to the engine.**
The declaration was consulted at `Set::build` by `capacity_in_range` and dropped, so nothing holding
a built `Set` could say what the material would accept. A control offering capacities the build then
refuses is a control that mostly prints refusals; what is offered has to be a reading somebody else
took, which is `Deck::sync_allowed`'s arrangement and the reason the sync chip's cycle can skip
(P-0090). It answers `[min, max, default]` per geometry, in `Set::source_capacities`' order, and it
is the same declaration `capacity_in_range` refuses against — so a rung this chip offers is a rung
the engine builds.

### 2. The seeds are a `re-salt` capsule, and the console is handed the salt

`DeckHead::re_salted` answers `SetProperty` carrying `Property::Seed { salt }`, and the salt is
`Aimed::salt` — a number the host put in the pane. `re_salted` in `crates/karakuri/src/main.rs`
performs it as `at.seed_salt = salt` with `at.salts` **cleared**: the aim carries both a slot seed
and a per-geometry list, `Set::build` prefers the list, and a slot filled from a Set file has one
`seed` line per geometry — so writing only the seed would move nothing there. Clearing is the same
numbers with the arithmetic left in the engine, which already derives each geometry's salt from the
slot's ([P-0087](../principles/0087-name-the-property-never-the-shape.md)).

**The number is derived and not invented**, and that is the whole of this half. `inspector` reads
`Set::source_salts` — what the slot is actually running — and asks
`karakuri_engine::set::derived_salt(salt, 1)` for the next value of the sequence the engine already
spaces geometries apart with. So the same press on the same slot lands on the same picture twice, a
replay of a kept Set reproduces it because `capacity` and `seed` are Set-file records, and nothing
on this panel produces a frame a later run cannot produce again
([P-0092](../principles/0092-the-same-inputs-produce-the-same-frame.md)).

**Nothing is refused.** The sequence goes forward, so the capsule cannot ask for the salt the slot
is already on — which is why it has no *already in that state* guard where the fold beside it does.
What is given up is a way back by pressing: the recovery is the Set you kept, and both pages say so.

### 3. `Property` loses its node and its `u64`, and the aim is not widened

`Property::Capacity { elements: u32 }` and `Property::Seed { salt: u32 }`. The operation already
names a deck, and a deck is the address: `Aim::capacity` is `--capacity`'s own field — *"`--capacity`
overrides every source"* — so one number sizes every geometry in the slot, and `Aim::seed_salt` is
the slot's.

The mismatch ADR-0314 recorded is closed **by narrowing the payload rather than by widening the
mechanism**, and the reason is that nothing was on the other side of it: no route constructed either
arm, no applier could have honoured an address, and a payload nobody reads is a free variable
(P-0087). **What would revive the address** is a pairing Set whose two geometries want two different
capacities; the day that is a want, `Aim::capacity` grows an entry per geometry and this arm grows
its address back.

### 4. It writes no record, on `SetCompositing`'s terms and in its company

`written` already answered `Silent(NoRecord)` for `SetProperty` and still does. `Record::Capacity`
and `Record::Seed` are **Set-file** records — what a Set *is* — and carry no slot, on
`karakuri-store`'s own rule that what a Set is does not depend on which deck slot it plays in. So a
session replayed does not come back at a capacity a hand stepped to or the salt a hand pressed for.
**A keep does**, and that is the half worth knowing: a keep writes one `capacity` and one `seed`
record per geometry off what the Set is running at, so what a hand asked for is written down the
moment the deck is kept and the gap is the session stream's alone. It is `Operation::LoadSet`'s hole
and not a new one.

### 5. The two chips are dropped before the row is

This is the one place on the console where a row draws part of itself rather than all of it, and the
reason is a measurement rather than a preference. An Inspector pane at the console's declared
minimum window is **237.5** pixels wide; the row needs about **296** for all seven. A row that took
all of them or none would therefore draw **nothing** at the width this arrangement claims to work
at — trading two controls that were never there for four that were, which is the opposite of what
`deck_head`'s *a control that does not fit is no control at all* is for
([P-0094](../principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)).

The panel opens at 1440 and the two are drawn from a window of about **1360** with two panes open,
so nothing an operator launches into is short of them. It is still the first control on this console
that comes and goes with the window, and both manual pages say so rather than leaving it to be found
by dragging.

## Alternatives

### a. A capacity track — a parameter row's fader, on the mock's own reading

ADR-0314's alternative **c**, and what ADR-0286's shape suggests on sight; the mock's Library reading
argues for it in as many words — *"capacity included, because a procedure declares one the same way
it declares a knob"*. Refused by what a drag **is** here, above. A track that acted only on release
would be a control behaving differently from every other track on this console, which is a decision
about the console rather than about this row.

### b. Widen `Aim::capacity` to one entry per geometry and address the node

ADR-0314's alternative **b**, carried forward and rejected again — now for a reason that is about
its merits rather than about its cost. It is still a sweep through `karakuri-environment` and every
construction of an `Aim`; what settles it is that the address it would buy has **no want behind it**.
Nothing constructs a per-node capacity, no surface draws one, and a Set holding two geometries at two
capacities is a state a Set *file* can express and no operator has asked for. Building the wide
mechanism first and looking for the control afterwards is the order this bay has already been caught
by once. **What would revive it is named above**, which is what makes this a decision rather than an
omission.

### c. A typed number — a field the operator writes a capacity into

The control that has no cycle length and no ladder to argue about, and it is refused where ADR-0262
refused it: this console's letter-taking flows are an arrangement's name and a Set's id in a pane
head, each bounded to one path component (ADR-0292, ADR-0221), and a number is a third flow rather
than the second one again. It also loses something the step keeps — a typed capacity can be any
integer in the range, so the first press an operator makes is a rebuild of the slot at a number
chosen by a keystroke, where a step is always one rung and always buildable. The open question
ADR-0262 left is still open and this record does not close it; the day the console takes text, this
chip is a candidate for it.

### d. A random salt from the console

The obvious implementation of *re-seed*, and the one thing this panel must not do. A salt chosen from
an entropy source is a picture no later run can produce: the record stream would carry the number,
so a *replay* would follow, but the operator's own repeat of the gesture would not, and a console
that reached for randomness once would have a reason to reach for it again. P-0092 is the rule and
the sequence is the mechanism that already exists (P-0085) — `derived_salt` is what spaces a Set's
geometries apart, and using it to space a slot's re-seeds apart costs nothing and adds no concept.

### e. Leave the row `plan` and wait for a text-entry decision

What the tree already was, and it is honest: the badge said the route was not built. It loses to the
same reading ADR-0314 applied to the mechanism — the sentence stopped one clause short of its own
answer. *The obvious affordance is refused* is true, and the clause after it is *and the console
already has the one that replaces it, three bays away*.

### f. Rename the operations row to *Element capacity and seeds*

ADR-0318's alternative, unchanged and still declined for its own reasons: *the camera* is the word an
operator comes to that row looking for, the heading is compared for equality by
`the_manual_and_the_vocabulary_agree`, and the tip is where the page says which row took it.

## Consequences

Checked against the tree, file by file.

- **`docs/manual/operations.html`** — *Element capacity, seeds, the camera* reads `panel deck head`
  with a `has` badge and its `op-when` is *on a worker* where it was *launch*, which is the word
  *Composite a deck's renderers* and *Load material into a deck* already carry. Its tip says what
  each chip is, why the capacity steps, that the operation names a deck and no node and what would
  revive the address, that a pane too narrow keeps the five, and why a MIDI line cannot learn the
  ladder. The heading is unchanged.
- **`docs/manual/console.html`** — both mock deck heads draw the two chips before the fold: deck A
  lit at `524288` against `drift_shell`'s declared `[4096, 1048576] = 262144`, deck B unlit at
  `32768`, which is `lattice_shell`'s own default. The deck head note gains three paragraphs — the
  step and why it is not a track, what lit means and that the chip is the slot's, and the re-salt —
  and a fourth saying what a narrow pane does. Its opening sentence is *its clock, its size, its
  seed and its fold*, and *five chips and a deck's name* is now seven.
- **`docs/manual/style.css`** — the `.deck-head` comment says what the row heads. The row's height
  is unchanged, because both new chips are `.mini`s and the block already sizes itself to one.
- **`crates/karakuri-operation/src/lib.rs`** — `Property::Capacity { elements: u32 }` and
  `Property::Seed { salt: u32 }`, with the argument for the missing address at the enum and the
  width at the arm. `gate.rs`'s sample constructs the narrower `Seed`; the class is unchanged and
  `Operation::SetProperty` is still `Closed(Class::LiveDeck)`.
- **`crates/karakuri-operation-record/src/lib.rs`** — the arm is unchanged and the comment above it
  now says that `SetProperty` has two panel controls and still writes nothing, and that a keep is
  what closes the gap for these two where nothing closes it for a load.
- **`crates/karakuri-engine/src/set.rs`** — `Set::declared_capacities` and the field behind it, seeded
  in `build_inner` off the `Checked`s the build was handed. It is the only change to this crate and
  it is a reader; the fallback for a `Checked` with no declaration is unreachable, because
  `capacity_in_range` refuses that Set first.
- **`crates/karakuri-console/src/view.rs`** — `Aimed` is the pane's reading, `AimChips` is the pair
  laid out with what each press asks for, `stepped_capacity` is the ladder walk, `RE_SALT_LABEL` is
  the capsule's word, and `DeckHead::resized`, `re_salted`, `hit_size` and `hit_salt` are the four
  new methods. `deck_head` lays the three after the `.sep` leftwards from the fold and drops the two
  when the row cannot hold them; `deck_head_into` paints them.
- **`crates/karakuri-console/src/input.rs`** — the probe row is *a deck head's seven* claiming 7 —
  seven controls, six methods, because `DeckHead::scrub` still answers for both arrows.
- **`crates/karakuri/src/main.rs`** — `inspector` fills `Pane::aimed` from three readings of what
  landed; `capacity_ladder` is the fold; `resized` and `re_salted` are the two free functions over
  the aims, beside `composited`; the press handler asks the head for six; and
  `every_control_in_the_table_is_asked_by_the_press_handler` names `head.resized(` and
  `head.re_salted(`.
- **The MCP route landed on these two performers the same day**, in the pass that wrote
  `set_property`: an operation a model asks for is handed to `App::performed` as an
  `Acted::Emitted`, which is the arm `resized` and `re_salted` are called in, so a model's
  `set_property` and a press on the chip are the same act from there on. The row's MCP badge reads
  `has operate` and `mcp.rs`'s *nothing on this frame performs it* group says `SetProperty` left it
  **by gaining a performer rather than by that rule changing**. It also makes `resized`'s *already
  aimed at this capacity* guard reachable: the chip's step is strictly above what the slot is
  running and cannot produce one, and a call naming a number outright can — which is why the guard
  is in the window and not in the console (P-0090).
- **`gpu::a_pane_reads_a_running_set` is where the host's reading is held**, and every assertion in
  it is against what the engine answers rather than against a number: the chip reads
  `Set::source_capacities`' first entry, lit and unlit agree with `Set::declared_capacities`'
  default, every rung is a power of two inside **every** geometry's declared range, and the salt is
  `derived_salt` of the salt the slot is running and differs from it. The numbers belong to
  `examples/coil_vortex.kir`, which the MCP surface exists to rewrite, and a fixture the product can
  rewrite is not a fixture (`docs/contributing.md` §3).
- **The pair this panel opens on runs at a capacity that is not a power of two**, and that is worth
  writing down rather than leaving as a curiosity: `coil_vortex` declares `capacity [1024, 1048576]
  = 10240`. So the *shipped* state of this program is the off-the-ladder case — the chip reads 10240
  unlit and one press asks for 16384 — which means the step-up rule is exercised by launching the
  program rather than only by a test that contrives it.
- **`docs/roadmap.md`** — M5.5's *Blocked on* is **nothing**, where it was five on 2026-09-08. The
  bay's exit — no `plan` badge in the panel column of its rows — is met on the reading that a `gap`
  meets it as a `has` does; it is not closed here, because closing it is the maintainer's.
- **What is not closed.** A per-geometry capacity is unaskable from any surface, which is now a
  statement rather than a gap. The console's text entry is still undecided and this record is a
  second reason to expect the question. And the deck head is the row with the least room left on
  this console: the next control it grows has nowhere to go, which is *Mx — TODO*'s warning about a
  region's declared minimum arriving one bay early.

## What was watched fail

Each was run alone against its own injected defect, `docs/contributing.md` §3's terms, and every
substitution was checked to have landed before its result was read.

- **Three tests went red on their own before a line of the console had been pressed**, which is the
  evidence worth the most here because nothing was injected to get it. The moment the mock pane
  carried an `Aimed`, `deck_head::the_deck_head_is_the_rows_own_geometry` and
  `every_control_clears_every_boundarys_grab` both failed at `SMALLEST` with *"a deck head with room
  for its chips"* — which is how the 237.5-against-296 measurement was **found** rather than
  assumed, and it is the whole of §5 above; the first draft of this decision put the two chips in
  the row and would have taken the other five off the console at its own declared minimum.
  `a_row_too_narrow_for_its_chips_draws_none_of_them` failed with them and passes again unchanged,
  which is what says the drop is a drop and not a widening of that rule.
- `deck_head::a_pane_too_narrow_for_the_build_chips_keeps_the_rest_of_the_row`, written first with
  the narrowing measured from the wrong end: everything after the `.sep` moves left with the row's
  right edge, so a formula that placed the row's edge at the chip's left drew the chips anyway —
  `left: Some(AimChips { size: [[637.6 522.5] - [674.9 538.0]], resize: Some(65536), … })
  right: None`. It is the slack between the arrows and the chip now, and it asserts the fit at that
  width as well as the drop one pixel under it, so it cannot pass by refusing everything.
- `deck_head::the_capacity_chip_steps_every_rung_once_and_wraps`, twice. With `stepped_capacity`'s
  `find(|c| **c > at)` weakened to `>=`, the walk stops dead:
  *"walking the ladder from its bottom visited [4096, 4096, 4096, 4096, 4096, 4096, 4096], which is
  not every rung once"*. And with the `or_else(|| candidates.first())` wrap deleted, the press at
  the top rung asks for nothing at all and the walk panics naming the rung it stopped at — a chip
  that reaches 262144 and then stops being a control.
- `deck_head::a_capacity_off_the_ladder_steps_up`, with the step written as a `position` lookup and
  a fallback to index 0 — the shape this was very nearly implemented as:
  `left: Some(SetProperty { deck: 1, property: Capacity { elements: 4096 } })
  right: Some(SetProperty { deck: 1, property: Capacity { elements: 131072 } })`. A slot at 81920
  dropped to the bottom of its own declared range by one press.
- `deck_head::the_re_salt_capsule_asks_for_the_salt_it_was_handed`, with `re_salted` sending
  `aim.re_salt.wrapping_add(1)` instead of the number it was given:
  `left: … Seed { salt: 2654435770 } right: … Seed { salt: 2654435769 }`. A console computing a salt
  rather than naming one, which is the defect P-0092 is about and which no test of the *shape* of
  the operation would have caught.
- `deck_head::a_capacity_with_no_shared_range_is_drawn_and_claims_nothing`, with `hit_size`'s
  `resize.is_some()` guard dropped: `assertion failed: !head.owns(at(aim.size.center()))` — a chip
  claiming a press it has nothing to answer with.
- `main::press_handler::every_control_in_the_table_is_asked_by_the_press_handler`, with the
  `.or_else(|| head.resized(at))` line taken out of the window's press chain: *"the press handler
  derives `deck_head_row(` and never asks it `head.resized(at)` — a deck head's seven is claimed by
  `input::claim`, drawn by this window, and then declined. That is the seam this module exists
  for"*. The defect it names is the one a console-only test cannot see: a chip that lights under a
  pointer and does nothing.
- `gpu::a_pane_reads_a_running_set` **failed on its own the first time the new assertions ran**,
  and the assertion was the thing that was wrong: it claimed the running capacity is on the ladder,
  and the launch pair answered *"the pair runs at 10240 and the ladder is [1024, …, 1048576]"*. That
  is `coil_vortex`'s declared default and it is not a power of two — so the test was asserting a
  property the shipped material does not have, and what replaced it is the property that does: the
  running value is inside the declared range, and there is always a rung to step to. The
  off-the-ladder case stopped being hypothetical in the same failure.
- `panel_column::every_operation_a_console_control_emits_has_a_panel_route_marked_built`, run with
  the console emitting and the page badge put back to `plan`: *"a control in
  crates/karakuri-console/src emits `Element capacity, seeds, the camera`, which
  docs/manual/operations.html marks `plan` in the panel column — a control an operator reaches and a
  page that says no program a player runs does (ADR-0213). Flip the badge, or say here why the
  control is not reachable"*. The seam names the missing half of the change without being asked
  which half.
