---
id: 0284
title: A readout is drawn and the panel badge does not move, because the meter counts a gesture
status: accepted
date: 2026-09-08
supersedes: []
superseded_by: [0288]
principles: []
tags: [console, manual, operations, m5]
---

# A readout is drawn and the panel badge does not move, because the meter counts a gesture

## Context

*Find out what a write did* is `Operation::SwapOutcome`, and
[`docs/manual/console.html`](../manual/console.html) draws it in the transport row as a capsule
reading `landed`, with a note — *Health, in the transport* — that says what it is for:

> Frame time against the budget, and what the last write did — **landed, rolled back for cost, or
> failed to build**. The rollback already happens on its own; what was missing was any way to see
> that it had. Unbreakable on stage is only reassuring if it is visible.

That capsule is now drawn. `crates/karakuri` watches every slot's sources, so a save from any
editor builds, swaps and is judged on every run; `Deck::events` is drained once a frame in
`staging`; and the same drain that writes the Staging lane's rows now writes
`view::Transport::health`, which `view::transport_row` places against the row's right padding and
`view::transport_into` paints in the mock's own `.pill.armed`.

**So the panel reaches the row, and the page's panel column still says `plan`.** That is the
question this record settles, and it is a question about the meter rather than about the drawing.

### What the meter actually asks

[ADR-0213](0213-the-interface-milestones-meter-is-the-panel-column-and-has-means-an-operator-reaches-it.md)
made the panel column the milestone's meter and defined `has` as *an operator running the
instrument reaches the operation*. It also named what the first flip owed, and
`karakuri-console/tests/panel_column.rs` is that test. Its second assertion —
`every_panel_route_the_page_marks_built_is_emitted_by_a_console_control` — reads every `has` badge
in the column and demands that some line of `crates/karakuri-console/src` construct
`Operation::<that variant>`. Flipping this row's badge fails it, verbatim:

> `docs/manual/operations.html` marks `Find out what a write did` built in the panel column, and no
> control in `crates/karakuri-console/src` emits it — the page claims a control an operator cannot
> find.

**And it is right to, for every row it was written about.** For a write, emitting is the only way a
surface reaches an operation: a fader that moves nothing is not a fader. **The proxy is
write-shaped**, and
[ADR-0281](0281-every-route-reaches-every-write-and-a-read-is-the-routes-own-interface-design.md)
has just decided that a read is not.

### What the two records before this one settled, and what neither reaches

[ADR-0264](0264-a-reading-is-one-question-and-only-the-opening-asks-it.md) settled which **gesture**
of a reading emits: opening the Library bay's `read` block emits `ReadSet`, closing it and moving
the cursor emit nothing. That is why *Read what one Set holds and declares* carries a `has` panel
badge — a press asks the question, so the scan sees it. **A readout has no gesture at all.** There
is no press to attach the emission to, and inventing one would be ADR-0264's own rejected
alternative in a worse form: an operation emitted at a moment nobody asked anything.

ADR-0281 loosened rule 01 — a write is reachable from every route, a read is each route's own
interface design — and marked four rows `read`, this one among them. It says exactly what it left
open:

> **A readout's `has` badge is not settled by this record and the question survives it.** … This
> rule frees an **empty** cell; it says nothing about a full one.

### What is true of the tree as this is written

- The capsule is drawn on every run, not only under `--mcp`, and stands after the Staging lane has
  emptied — the lane is *what is outstanding* and the capsule is *what the last write did*.
- `karakuri-console` holds the drawing: `tests/transport.rs` asserts the three words off the paint
  pass, that nothing is drawn where nothing has been written, and that the `armed` wash is spent
  only on the verdict that is on screen.
- `crates/karakuri` holds the seam: the value reaching `Readout::health` through a real save, a real
  build and a real verdict is asserted in `the_swap_report_says_what_the_lane_says`, which takes a
  device because a `swap::Event` cannot be made without one.
- Neither of those is visible to `panel_column.rs`, which reads source text for `Operation::`.

## Decision

**The readout is drawn and the row's panel badge stays `plan`. A `has` badge in that column
continues to mean an operation some control emits, and correcting the meter for a read that is
answered without being asked is a decision taken on its own rather than as a side effect of drawing
one capsule.**

Three things go with that, and the last is what makes it honest rather than convenient:

- **Nothing is widened to make the badge fit.** `panel_column.rs` is untouched. No exemption list is
  added, no badge class is invented, and no line is written in `karakuri-console` for the scan to
  find — a `pub fn` there returning `Operation::SwapOutcome` that no control calls is precisely the
  *false positive on a row already marked `has`* that file names as the one combination that passes
  in silence.
- **The state is written where the state is kept.** `docs/roadmap.md`'s M5.4 says the capsule is
  built and the badge did not move, so the bay's exit condition — *no `plan` badge in the panel
  column of this bay's rows* — is not met by this row and says why.
- **`plan` is now imprecise on this row, and that is the price.** ADR-0213 glossed it as *this
  surface is meant to reach it, and no program a player runs does yet*, and the second half has
  stopped being true. What the badge is carrying here is *the meter cannot see this yet*, which is a
  different sentence. This record is where that is written down rather than left for a reader to
  notice.

## Alternatives rejected

### a. `has`, with `panel_column.rs` relaxed for rows the page marks `read`

**The strongest of the three, and the one to re-propose.** It costs no hand-written list: ADR-0281
put `<span class="op-kind">read</span>` in the `op-head` of the four `Silent::Question` rows, so the
relaxation would be read off the page the file already parses — *a `has` panel badge on a row the
page marks `read` need not name an emission*. It is bounded to four rows by construction, it is the
page saying it rather than a person, and it says on the meter what is true for a player: **on the
panel, you find out what a write did by looking at the transport.**

**It loses on what it would stop checking, not on what it would allow.** The assertion it relaxes is
the only mechanical thing standing behind a `has` badge in that column, and relaxing it for a class
of rows removes it for *every* member of that class, including the two that are `has` today and
genuinely emit — *List what the store holds* and *Read what one Set holds and declares*. The day one
of those controls is deleted, its badge would go on reading `has` and nothing would say so. Trading
a check that works on three rows for a badge on the fourth is the wrong direction, and the
replacement — a check that the readout is *drawn* — cannot live in that file: `karakuri-console`
cannot see `crates/karakuri`, and the drawing is only half of the seam anyway.

It also decides a rule that governs **six rows and not one** — every read on every surface, and
every readout any bay draws hereafter — from inside one bay's work. `docs/roadmap.md` said that
about this row before it was built and it is still true.

### b. `gap`, on the reading that a readout is not an act

Coherent, and it very nearly wins. ADR-0264's whole argument is that an operation is *asked*, and a
capsule nobody pressed asks nothing; ADR-0281 says an empty cell on a read row is that route's own
answer rather than a debt; and it satisfies M5.4's exit sentence, which is phrased *no `plan`
badge*. Nothing would have to change in any test.

**It loses to the reader the page is for.** `gap` means *this surface does not reach it*, and the
README says the manual is *"for somebody who wants to play the instrument rather than build it"*. A
player who reads `gap` here goes looking somewhere else for a thing that is on their screen at all
times. A badge that is mechanically defensible and sends the reader away is worse than one that is
imprecise about how far along the work is — and it would also be irreversible in the wrong
direction, because the day the meter is corrected the cell would have to move from `gap` to `has`,
which reads as *somebody built it* about a capsule that had been there for months.

### c. A fourth badge class for *answered without being asked*

ADR-0281 rejected a fourth class for the read/write question and most of its argument carries: the
four files that read this page compare only against `"has"`, so a new class is invisible to every
one of them and costs no test — which is exactly what is wrong with it. It would put a word on the
page that nothing holds true, on a page whose columns are the milestone's only meter. And it draws a
distinction one row wide today.

### d. Draw nothing, and leave the row to `swap_outcome`

The panel already holds every verdict and a model can already ask for one, so the cheapest answer to
*is this row done* is to decide the panel declines the read — which ADR-0281 explicitly makes
available (*"Each stays `plan` until somebody decides the bay does not draw it"*).

It loses to the mock, which is the specification for the drawing (`docs/contributing.md` §5 step 3):
`console.html` draws the capsule, tips it, and argues for it in a note of its own. And it loses to
what the note says — the rollback happens on its own and nothing on the panel said so, while
`crates/karakuri` prints nothing at all about a swap. Declining the read here would leave an
operator watching a picture that disagrees with the file on disk with no way to find out, which is
the failure the note was written about.

## Consequences

- **The M5.4 exit is not met, and one row is why.** *Find out what a write did* keeps its `plan`
  panel badge with its control drawn. `docs/roadmap.md` carries that sentence beside the item, so
  the milestone's own meter is not quietly satisfied by a change to the page.
- **`panel_column.rs` is unchanged and still means what it meant.** Every `has` badge in the column
  names an operation a control constructs, both directions still fail, and the file's `UNREACHABLE`
  list is still empty.
- **What holds the capsule honest is two tests in two crates and neither is the badge.**
  `karakuri-console/tests/transport.rs` for the drawing and `crates/karakuri`'s
  `the_swap_report_says_what_the_lane_says` for the seam. That pairing is `mod press_handler`'s
  arrangement read on the readout's side, where the scan it uses can see nothing at all: a readout
  hit-tests nothing, so `input::PROBES` gains no row and `ASKED` gains no entry.
- **The three words are one type, and that is now load-bearing in two places.**
  `view::Stage` is the Staging lane's `Candidate::stage` and the transport's `Transport::health`;
  `console.html` spells the same three answers in both notes, so a second enum would have been
  `docs/contributing.md` §4's *a name meaning two things*.
- **The capsule declares nothing under ADR-0283, and that is the right answer.** It changes when a
  person saves a file, which is not a rate. The unit of declaration is the region, the transport row
  already declares `BEAT_STALENESS` for as long as it is drawn, and a staleness invented for the
  capsule could only ever be slower than the beat's — so it would change nothing a window does and
  would be a rate for something that does not move.
- **The other three read rows are untouched.** Two of them are `has` on the strength of a gesture
  that really does emit (ADR-0264), and *Read one node's source* is `plan` for reasons that have
  nothing to do with this — the Inspector draws no source area and the console has one
  letter-taking flow.

## What this does not decide

- **Whether the meter is corrected, and how.** Alternative (a) is the shape to start from, and what
  it needs beside it is a check that a drawn readout is drawn — which is not `panel_column.rs`'s to
  make.
- **Whether a `rec` capsule follows this one into the row.** *Record the session* is machinery
  rather than a rectangle and is blocked on its own things; the gap the mock leaves after `landed`
  is not reserved for it here.
- **What treatment a verdict that is not on screen should have.** `armed` is spent on `landed` and
  the other two take the plain `.pill`, because those are the mock's two treatments for this
  capsule. A colour of alarm is a decision `console.html` has not taken.
- **Whether the capsule should name the deck a verdict came from.** It carries no address, so what
  it can honestly say is *the last write*, and the lane beside it is where a slot is named. Giving
  it a letter would be the finer address `swap::Event` does not carry — the Staging lane's own
  standing gap.
