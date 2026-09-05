---
id: 0213
title: The Interface milestone's meter is the panel column, and `has` means an operator reaches it
status: accepted
date: 2026-08-28
supersedes: []
superseded_by: []
principles: [0087, 0093, 0094]
tags: [docs, process, console]
---

# The Interface milestone's meter is the panel column, and `has` means an operator reaches it

## Context

M5's goal is *the application*: a GUI a person plays a VJ set on. Its opening move was naming
rather than drawing ([ADR-0180](0180-the-operation-vocabulary-is-a-crate-with-no-dependencies.md)),
and that half is finished — fifty operations, one row each on
[`docs/manual/operations.html`](../manual/operations.html), checked against
`karakuri_operation::Operation` both ways round.

What the milestone has never had is a way to answer *how far along is it*. The roadmap's own
sentence about the page has gone stale twice, both times the same way — a badge moved and a
transcribed figure did not — and it says so at the point where it says it. So the question is not
only what to count but **what nothing has to transcribe**.

Counted on this page today, a column at a time:

```sh
for col in panel key MIDI MCP; do
  grep -o "class=\"rt [a-z]*\">$col " docs/manual/operations.html \
    | sed 's/.*rt //;s/">.*//' | sort | uniq -c
done
```

| Column | `has` | `plan` | `gap` | rows |
| --- | ---: | ---: | ---: | ---: |
| panel | **0** | 46 | 4 | 50 |
| key | 21 | 0 | 29 | 50 |
| MIDI | 8 | 1 | 41 | 50 |
| MCP | 6 | 0 | 44 | 50 |
| CLI | 16 | 0 | 0 | 16 |

216 badges, 51 of them `has`. **The panel column is the only one of the four that is entirely
empty**, and it is the column the milestone is named after.

## Decision

**The Interface milestone's progress meter is the panel column of
`docs/manual/operations.html`, and a `has` badge in that column means an operator running the
instrument reaches the operation.**

Two halves, and the second is the load-bearing one.

**The meter is the column**, not a number written beside it. Nothing transcribes it: the count is
the `panel` line of the loop above, and the remaining work is the `plan`
badges — which are not a to-do list somebody keeps, because each one already names where its
control lives. Grouped, the 46 read:

```
grep -o 'rt plan">panel <b>[^<]*' docs/manual/operations.html | sed 's/.*<b>//' | sort | uniq -c | sort -rn
```

> 4 transport · 4 inspector · 3 library · 3 deck head · 3 &mdash; · 2 transition row · 2 drag ·
> then 25 homes with one row each — tap, tally chip, take back, strip trim, strip mask mini,
> strip fader, strip button, strip, solo, sensitivity row, renderer chips, rec, preview, pane
> edge, outputs, offset, node head, library → deck, keep, health readout, crossfader, close,
> click a strip, bay head, ½ ×2.

**`has` means the operator reaches it.** Not *a control exists somewhere in this workspace* and not
*a cargo target can be made to emit it*: the row is claimed the day a person who launched the
instrument can perform that operation from the panel in front of them. That is
[P-0094](../principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)'s sentence used as a definition —
*a window that opens and cannot be touched is a demo, not a tool* — and it is what the README
already says the manual is for: *"for somebody who wants to play the instrument rather than build
it"*.

**What this commits to.** When the panel column's first badge flips to `has`, a test asserts that
each `has` row's operation is actually emitted by a console control, and fails in both directions —
a control reaching past the page, and the page claiming a control that does not exist. That is not
a new idea to invent: `crates/karakuri-environment/src/mcp.rs` already does it for the MCP column
(`karakuri-cli/src/mcp.rs` when this was written; it moved in ADR-0214), in
`every_tool_this_server_publishes_has_a_route_on_the_page` and
`every_mcp_route_the_page_claims_is_a_tool_this_server_publishes`, matching on the badge's **text**
as well as on the title so that *"the page names a call nobody can make"* fails too. The
vocabulary's own manual test says the four columns *"become checkable one surface at a time as they
migrate, in the crate that owns the surface"*; this is the second of the four, owed on the first
flip rather than afterwards, because a meter nothing checks is the roadmap sentence that has
already gone stale twice.

**What it costs, stated plainly.** The meter reads **0 of 46 until a program exists.** Five
operations are emitted by console controls today — `SetGain` and `SetOpacity` from
`panel.rs`, and `SetBlendMode`, `SetResidency` and `SetMaskShape` from the three chips in
`view.rs` — and none of them moves the meter, because the only thing that runs them is
`examples/panel.rs`. A milestone that has been running for weeks reads zero on its own meter, and
the page has to say so in the one place it currently says the opposite: the legend and the count
paragraph both assert the panel is not built, which is
[P-0093](../principles/0093-a-statement-is-held-true-by-the-thing-it-describes-or-it-is-deleted.md) owed from the other end —
not a goal stated in the present tense, but a **failure** stated in the present tense after it
stopped being the reason. Correcting them is part of this change and is
[P-0093](../principles/0093-a-statement-is-held-true-by-the-thing-it-describes-or-it-is-deleted.md):
the stale text is a defect of the changes that made it stale.

## Alternatives rejected

**A hand-maintained checklist section in the roadmap.** The obvious answer, and the one that reads
best on the day it is written: a list of what the panel still owes, ticked off as it lands. It is
[P-0093](../principles/0093-a-statement-is-held-true-by-the-thing-it-describes-or-it-is-deleted.md)'s *maintaining a
second copy of a list by hand* exactly, and
[P-0093](../principles/0093-a-statement-is-held-true-by-the-thing-it-describes-or-it-is-deleted.md)'s *invariants that live
only in a README and are enforced by review*. The decisive part is that **the second copy would
carry no information the first does not**: the grouped `plan` badges above already are the board,
one line per home, derived by a `grep` from the specification rather than written down beside it.
A checklist would be that list retyped, and this project has watched exactly that go stale twice in
the roadmap's own sentence about this page.

**Counting drawn regions — five of the console's nine.** Already written down, already argued
region by region against the manual, and it moves. It loses on being the wrong resolution twice
over. It is too coarse: the Mixer bay is one region and holds **all five** of the controls that
emit anything at all, so the number cannot distinguish a bay with a live fader in it from a bay
with a heading. And it hides the shape of what is left — of the four undrawn bays, **three are blocked on
machinery rather than on drawing**: Staging has no queue anywhere in the workspace to list, Master
has no master out level in `karakuri-engine` and no writable L5, and the Sequencer has no pattern
behind it. Drawing is not what those three are waiting for, so a meter denominated in drawing
would report them as work anyone could pick up.

**`has` meaning "a control in the console's example reaches it".** The tempting one, because it
would move the meter today. `grep -rn 'Operation::' crates/karakuri-console/src/*.rs` returns
twenty matches; fifteen are prose in doc comments and **five are emissions** — `SetGain` and
`SetOpacity` at `panel.rs:197` and `:198`, and `SetBlendMode`, `SetResidency` and `SetMaskShape` at
`view.rs:3399`, `:3467` and `:3535`. So five rows would flip: *Gain*, *Opacity*, *Blend mode*,
*Put a deck on air, prime it, or take it off*, and *Set a deck's mask shape*.

It lost twice. It lost to P-0094 and to the README's statement of who the manual is for: nobody can
play a set from `cargo run --example panel`, so a page written for a player would be asserting a
route none of its readers has. And it lost to
[P-0087](../principles/0087-name-the-property-never-the-shape.md), which is the sharper of the two.
*Which cargo target the control happens to live in* is the shape; **the operator reaches it** is
the property. Naming the shape would have the meter move the day the code is copied from an example
into a binary and not the day anything became reachable — and worse, it would go on reading the same
value if a control were later moved back out, because the property it was standing in for was never
the one being asked about.

**Say nothing and let the milestone be measured by its list of remaining work.** What the roadmap
says today is that *"the panel routes in `manual/operations.html` are still `plan`, because the
console is an example rather than the program"* — which is true, is the right reason, and is not a
meter: it does not say 0 of 46, it is not derived, and there is nothing to reread when a badge
moves. The reason this record exists is that the page had already been made to say something false
in order to avoid saying zero.

## Consequences

- **The milestone has a number, and it is zero.** 0 of 46 panel routes, against 21 of 50 for the
  keyboard, 8 for MIDI and 6 for MCP. The first flip is the one that costs the most, because it is
  the flip that also owes the test.
- **The first `has` in the panel column owes a test in `karakuri-console`**, shaped like
  `mcp.rs`'s pair and like `tests/vocabulary.rs`, which already reads this page for the *Arranging
  the console* section
  ([ADR-0197](0197-the-consoles-op-stays-and-what-blocks-it-is-the-page-rather-than-the-code.md)).
  Until then the column is unchecked, and that is stated rather than assumed.
- **`plan` stops meaning what it said and means what it carried.** The legend glossed it as *"it
  has a home on the console, and the console is not built"*; both halves were false — three badges
  name no home at all (*Wire a procedure's input to a node*, *Narrow the published interface*,
  *Walk the edit history*), and the panel draws part of itself. The class means **this surface is
  meant to reach it, and no program a player runs does yet**. Naming a home is what 43 of the 46
  additionally do.
- **The five emitting controls are not a claim on the page.** They stay `plan`, and this record is
  why. When they flip they flip together with whatever puts the panel in front of a player, and
  they are the cheapest five to prove the new test against.
- **This does not decide anything about the page's specification.** No badge moves, no row gains a
  home, and the three homeless rows stay homeless — naming a home is an edit to the specification,
  which ADR-0197 established is a decision of its own rather than a note taken while counting.
