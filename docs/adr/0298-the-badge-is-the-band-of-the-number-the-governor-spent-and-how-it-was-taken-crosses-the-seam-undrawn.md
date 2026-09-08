---
id: 0298
title: The badge is the band of the number the governor spent, and how it was taken crosses the seam undrawn
status: accepted
date: 2026-09-08
supersedes: []
superseded_by: []
principles: [0095]
tags: [console, view, badge, governor, estimate, manual]
---

# The badge is the band of the number the governor spent, and how it was taken crosses the seam undrawn

## Context

**M5.1 closed on 2026-09-03 with the risk badge undrawn**, on the reading that it was *"not a
drawing that is owed"* because nothing in this workspace could say what a slot costs — and nothing
was scheduled to notice when that changed. `docs/roadmap.md` names that as this repository's own
failure realised: *an unfinished task under a closed milestone is never picked up*. M5.14 exists to
pick it up and this is its fifth item, the one the other four are for.

Three records made the number. [ADR-0285](0285-a-renderers-floor-is-bounded-from-its-declared-ranges-or-refused.md)
bounded a renderer's sub-pixel floor from its declared ranges;
[ADR-0293](0293-a-rung-may-sit-under-the-floor-because-what-it-hides-is-bounded-and-paid.md) loosened
how strictly that floor is read, and twelve refusals became none;
[ADR-0296](0296-the-governor-budgets-on-the-estimate-where-it-answers-and-on-the-measurement-where-it-does-not.md)
wired it to `Deck::govern`. So `Decision::budgeted_ms` is a number for every piece of material this
instrument ships, `Decision::basis` says whether it is the two-draw estimate at the output's size or
the single-draw measurement at the reference resolution, and `Report::decisions` is the list.

`docs/manual/console.html` specifies the drawing twice — deck A's caption tooltip, and *What a deck
preview cell shows, and when* — and both passages were corrected the same morning to say what is
left: **the drawing, the table that turns a number into one of the five bands, and a path from the
deck to this caption.** This record is those three.

**What stood in the console until now was an assertion that the badge was absent.**
`the_badge_is_absent_rather_than_drawn_empty` fails when somebody draws a badge and never when a
number arrives, which is why ADR-0296 grew
`a_governed_slot_now_has_a_number_a_band_can_be_predicted_from` on the engine side rather than
trusting it. Retiring the first is part of this record and §6 is what it cost.

## Decision

### 1. The band table is the console's, and it is a transcription

`karakuri_engine::estimate`'s own documentation says the table is not the engine's, and
`tests/governor.rs` quotes the boundaries rather than sharing them. The reason is not layering
etiquette: **a millisecond is the engine's and *how many of these, at what rate* is this panel's** —
four slots share one frame, 16.7 ms at 60 Hz is about 4 ms each, and *four* is a fact about the four
cells in the Program bay.

So it lives in `crates/karakuri-console/src/view.rs`, as `Band`, `band_of` and four boundaries, and
it is a **transcription out of `docs/manual/console.html`** exactly as `room::size` is a
transcription out of `style.css`.

| constant | ms | where it comes from |
|---|---|---|
| `BAND_BLUE_MS` | 4 | *"Green, up to 4 ms: four of these at 60 Hz."* and *"Blue, over 4 ms"* — one boundary written from both sides |
| `BAND_YELLOW_MS` | 8 | *"Yellow, about 8 ms: the boundary for two at 60 Hz, and four at 30 Hz."* |
| `BAND_RED_MS` | 12 | *"Red, about 12 ms: one at 60 Hz, two at 30 Hz."* |
| `BAND_PURPLE_MS` | 16 | *"Purple, over 16 ms: one slot eats a whole frame and the cell stops drawing."* |

**Each is named for the band it lets you into rather than the one it leaves**, because that is the
page's rounding rule written into the name: *a value on a boundary rounds to the worse band*, so
`band_of` is four `>=` comparisons falling through to green and 4.0 ms is blue. The direction is the
one the whole chain rounds in — `estimate` rounds toward refusing at every step, and a badge that
read a boundary kindly would be the one place in the chain that did not.

**Checkable, and checked against the page rather than against itself.**
`tests/transcribed_constants_cite_the_mock.rs` is where numbers this crate copies out of the mock are
held to their source, and the band boundaries are that kind of number — but they cannot go through
the citation machinery already there, which resolves `` `selector` `` and `` `property: value` `` in
a stylesheet, because these are written as English. They get a reader of their own in the same file,
which finds each band's name as a whole word with a figure in milliseconds after it. **It asserts
every reading rather than the first**: the scale is stated twice on that page, and a boundary moved
in one passage and not the other now fails here instead of shipping two scales.

### 2. The dot is the band of the number that was spent — `Basis` and P-0095

**The dot is drawn from `budgeted_ms` whatever `Decision::basis` says, and the basis crosses the
seam anyway and is drawn nowhere.**

A band read from an estimate at this deck's own size and a band read from a measurement at 1280x720
are **not the same statement**. That is P-0095, and ADR-0296 §3 keeps `FloorRead` and `Floored` on
the decision precisely so the difference cannot be lost. Two things decide what the console does
with it.

- **The number is the one the governor spent.** A badge drawn only where the basis is `Estimated`
  would show a different quantity from the one the deck is being governed on. Every Set that swaps
  in on a live run arrives unestimated (ADR-0296, *Consequences*), so the dot would vanish at the
  moment an operator loaded something; and a slot the governor parked on a measured number would be
  parked with no visible cause, which is
  [ADR-0191](0191-the-panels-parked-deck-is-parked-by-the-governor-or-it-is-a-drawing-of-one.md)'s
  complaint. **A refusal is not a green dot. A number that was spent is not a refusal.**
- **The mock does not distinguish them, and the page moves first.** `.risk` carries five classes and
  there is no sixth mark, no hollow ring, and no second word in the caption for *how this was
  taken*. Inventing one here would be the console specifying itself, which is the one thing this
  crate is not allowed to do.

**What P-0095 does get is the one thing the page already specifies**: no number, no dot. It also
gets `Budgeted::basis`, which is carried across the seam, is asserted by
`an_estimate_and_a_measurement_draw_the_same_dot` to make no difference to the drawing, and is
therefore a decision with a test under it rather than a field nobody filled in. §7 is what the page
would have to say for that test to change.

### 3. Two nothings, and neither of them is a green dot

**A slot with no number draws no dot** — not hollow, not grey, not green by default, each of which
would assert a reading nobody took ([ADR-0200](0200-a-bays-first-pass-draws-the-values-that-exist-and-omits-the-rest.md)).
Three cases take that path and none is a small number: nobody has governed this deck; the governor
found neither number for the slot (`governor::Basis::Unbudgetable`, which crosses the seam as an
absent entry and **not** as `0.0`, because `0.0` draws green); and a number that is not finite,
which is a failed reading rather than a cheap Set.

**A cell with no slot behind it draws no dot either, whatever it was handed.** The mock's D cell is
the case and its tooltip is the rule — *"No slot is no cost, so there is no dot."* The picture gates
the badge, so the two halves of a caption can never disagree about whether there is a slot behind
it.

**Purple is the one band that is also a behaviour, and the behaviour is not this crate's.** *One
slot eats a whole frame and the cell stops drawing*: stopping it is the governor's, and a stopped
slot reaches the console as a cell with no picture in it. The console never decides to stop drawing.

### 4. The cost is a field of its own, because it keeps a different clock from the picture

`View::costs: [Option<Budgeted>; DECKS]`, beside `View::previews` and not inside `Picture`.

A picture is a texture registration and is rewritten **every frame** by whoever owns the device. A
cost is `Report`, and a report is taken **on a governor pass** — before the first frame, and again
whenever a residency is written. Folding the number into `Picture` would make every frame's texture
aim carry a value it did not take, dropped and re-fetched sixty times a second to no purpose, and
would put the governor inside `Gfx::aim`, which is a function about texture sizes.

`src/` takes no engine ([ADR-0156](0156-the-consoles-arrangement-is-a-tree-this-repository-owns.md)),
so `Budgeted` and `Basis` are console-side types and whoever holds the deck reads the report and
writes them — the same seam `View::previews` and `View::transport` already are.

### 5. Five colours of the room's own, and not `--c-mint` and `--c-pink`

The stylesheet gives the bands five custom properties of their own — `--c-band-green` through
`--c-band-purple`, stated for both rooms — rather than reaching for the palette's moods, and
`Palette` transcribes all five. **A green dot is not *live in the good sense* and a red one is not
*on air*.** The moods say what a thing *is* and are used wherever that thing is drawn; the bands say
where one number fell on one scale and are used in one place.

They are held against `.console.day` and `.console.night` by a test, which is a check the other
twelve colours do not have — see *Consequences*.

### 6. What retiring the absence assertion cost

`the_badge_is_absent_rather_than_drawn_empty` caught exactly one thing: a caption painting a mark it
has no value for. That is now `a_slot_with_no_number_draws_no_dot` and
`a_cell_with_no_slot_draws_no_dot_even_with_a_number`, asserted the same way — no shape but text
inside the caption, which fails for a circle, a filled rectangle, a ring or a glyph standing in for
one.

**The pair is stronger than what it replaces**, and the difference is the whole of this item: the old
assertion could not tell a dot drawn from a number apart from a dot drawn by default, because it
refused both. The new pair holds the refusal exactly where the manual puts it and the drawing
everywhere else. The comment block that replaced the test says so in the file, because a reader
finding the old name in the history is owed the reason it went.

### 7. What the page must say before the badge can say more, and it is not edited here

**Two passages went stale the moment this landed**, and they are the same sentence in the two places
the page states this scale.

- Deck A's caption tooltip: *"The estimate is taken now and the dot is not drawn yet, so the console
  draws the letter and the state and no dot at all — what is missing is this cell's own drawing
  rather than a number behind it."* The dot is drawn.
- The body, under *What a deck preview cell shows, and when*: *"The estimate exists now, and the dot
  is what is left … the badge is the last part of this cell the panel cannot fill."* It is filled.
  The paragraph is a record of two corrections already and wants a third rather than a deletion —
  what it is tracking is the reason the badge was absent, and the reason is now spent.

And two things the page states that this drawing cannot honour.

- **Deck A's tooltip calls the dot *"what this slot is estimated to cost"*.** Under
  `Basis::Measured` it is not an estimate; it is a single draw at the reference resolution. Nothing
  on screen makes that claim today, because this console draws no tooltips at all — but the page's
  own words are the specification, and the day a tooltip is drawn the word has to be settled first.
- **If the distinction is to be drawn, the page has to say what mark carries it.** The
  operator-facing difference is that a measured band is about a frame at 1280x720 and can therefore
  be a band *too good* on a larger output — the one direction the whole estimate is built to round
  away from. A ring round the dot against a filled dot is the obvious shape and it is not this
  record's to choose.

**All four are reported and none is fixed** — the page is the specification and another agent holds
it today.

## Alternatives rejected

**Draw a dot only where the basis is `Estimated`.** The honest-looking answer, and the one this
record spent the longest on. Rejected on two counts argued in §2: the badge would stop showing the
quantity the governor is deciding on, and it would go dark on every swap — the moment an operator is
most interested in what they just loaded. The instrument declining to answer is a different thing
from the instrument answering at a size that is the operator's only by coincidence, and P-0095's
demand for the second is *say how it was taken*, not *withhold*.

**Put the band table in `karakuri-engine`.** One table, shared with
`a_governed_slot_now_has_a_number_a_band_can_be_predicted_from`, and no transcription to guard.
Rejected because `estimate.rs` already says the table is not the engine's and the reason survives
scrutiny: the boundaries are a reading of one frame's budget shared four ways, and *four* is the
number of cells this panel draws. An engine that owned it would have an opinion about a console's
layout, and `karakuri-cli` — which governs and has no cells at all — would inherit it.

**Fold the cost into `Picture` as a third member.** One field on the view instead of two, and one
place a cell's state is read from. Rejected in §4: the two arrive from two places on two clocks, and
the join would have to happen inside the per-frame texture aim.

**Write `Basis::Unbudgetable` across the seam as `0.0`.** It makes `Budgeted` a plain `f32` and
removes an `Option`. It also draws a **green** dot for a slot nothing measured and nothing
estimated, which is *why an unmeasured Set is not a free one* falsified at the last step of the
chain that spent three records establishing it.

**Keep the absence assertion beside the new tests, narrowed to a cell with no number.** That is
`a_slot_with_no_number_draws_no_dot`, which is what happened — what was rejected is keeping the old
*name*, which says the badge is absent and would now be a test asserting the opposite of its title.

**Round a boundary to the better band.** Refused by the page in one sentence. It is also the only
place the chain would round generously, and the correction ADR-0293 §4 applies is sized against
exactly these boundaries: a truth of 12 ms corrected by 4/3 reports 16, one band worse, and that
arithmetic assumes 16 is purple.

**Wait for a second measurement instrument before drawing anything** — M5.14 item 2, an instrument
that measures a frame rather than a pass. It is real and it is not this. The badge is a per-slot
cost and the item is a per-frame total; making item 5 wait on it is how item 5 came to be undrawn
for the length of a closed milestone in the first place.

## Consequences

- **`crates/karakuri-console/src/view.rs` gains six public items** — `Band` (with `word`, `colour`
  and `ALL`), `Basis`, `Budgeted`, `band_of`, the four `BAND_*_MS` boundaries — and `View` gains
  `costs`. `caption_into` takes an `Option<Budgeted>` and paints a circle.
- **`crates/karakuri-console/src/room.rs`**: `Palette` gains five colours and `size` gains
  `PREVIEW_RISK`, which is `.risk`'s `width: 6px` and passes the existing citation guard unchanged.
- **`crates/karakuri/src/main.rs` owes one function and one line**, and it is the only thing outside
  this crate that this record needs. It is written out in the report accompanying this change and is
  not applied here: `main.rs` is another agent's this morning.
- **The badge is exactly as fresh as the last governor pass**, and no fresher. `ask_to_prime` takes
  a report before the first frame; `Deck::govern` runs again when a residency is written, in
  `main.rs`'s `Record::Residency` arm, **and that report is dropped today** — keeping it is the
  second place `View::costs` wants writing. A Set that swaps in arrives unestimated (ADR-0296) and a
  `resize` drops the estimate, so a badge can be a band better or worse than the slot until
  something governs again. That is the staleness ADR-0296 already names, reaching one more consumer,
  and it is not fixed here.
- **The five band colours are checked against the stylesheet and the other twelve still are not.**
  `Palette` sits outside `transcribed_constants_cite_the_mock.rs`'s scan — `SOURCES` starts at
  `pub mod size {` — so no colour in `room.rs` was held against `style.css` by anything until now.
  `the_band_colours_are_the_rooms_own_five` closes it for five of seventeen. The other twelve are a
  gap this record names and does not fill.
- **`room.rs`'s module documentation was arithmetically wrong before this and is corrected.** It
  said *"twelve of the fourteen properties"* where the struct carried sixteen, and its two bullets
  covered three of the four that are not plain hex — `--c-tint` was in neither count. It now reads
  seventeen of twenty-one, with a third bullet so the three cover all four.
- **The guard's three floors are not raised.** They were set on 2026-09-05 with a margin of two or
  three and this adds one constant, so the margin grows rather than the floor. Raising them is a
  maintenance act for whoever next reads that comment.
