---
id: 0330
title: The console paints its own hover layer, and the tips are the manual's own words
status: accepted
date: 2026-09-09
supersedes: []
superseded_by: []
principles: [0085, 0090, 0091]
tags: [ui, console, manual]
---

# The console paints its own hover layer, and the tips are the manual's own words

## Context

M5.11 has been blocked on a decision rather than on work since it was written: **who owns the
pointer** — whether the panel gains `egui` widgets so that the toolkit can carry a tooltip, or
whether the console paints its own hover layer.
[`karakuri-console`'s `input`](../../crates/karakuri-console/src/input.rs) named it as its own
decision and deliberately did not take it, because nothing it was doing forced an answer: rule 4
claims a press on a painted control, and a press is not a rest.

Two things had already been decided around it, and together they decide this.

**The console already answers *what is under the pointer*.** `input::PROBES` is one row per
derivation for every control the panel draws, and `claim` walks it on every press and every move.
A tooltip is that question asked on a rest instead of on a press —
[P-0085](../principles/0085-take-the-mechanism-that-exists-and-pay-the-bill-now.md), *check what
it is made of; the parts are usually already there under other names*.

**`egui` owns no widget anywhere on this console, and that is
[ADR-0156](0156-the-consoles-arrangement-is-a-tree-this-repository-owns.md)'s seam.** The
arrangement is this repository's tree and what the toolkit receives is a rectangle; the console
paints, and `claim`'s rule 4 exists because *a press routed to `egui` there reaches nothing at
all*. A widget under every control, placed only so that a tooltip could hang off it, would put
hit testing and layout for one feature into a second system that has no other business here.

**And the words already exist.** The mock carries about a hundred and forty `data-tip` attributes,
and M5.1 to M5.9 each ended by rewriting their bay's prose into more of them, which is why the
roadmap says this sub-milestone *starts with its specification written and owes only the drawing*.

## Decision

**The console paints its own hover layer**, in a module of its own:
`crates/karakuri-console/src/hover.rs`.

**The tip text is not a second copy.** `docs/manual/console.html` is embedded with `include_str!`
and parsed **once at start-up**, off the frame path, into one string per control. Nothing in
`crates/` restates a word of a tip, so this is `docs/contributing.md` §4's first tier —
*generated* — rather than its second.

**What is transcribed is the key, and only the key.** Which drawn control a tip belongs to cannot
be derived from the page: the mock carries no `id` and no `data-control`, and its classes repeat —
twenty of its tipped elements are a bare `.pill`. So `hover::TIPS` writes down a **citation** per control: the
element's class and the words inside it, in the page's own spelling. That is the smallest thing
that names one element of the mock, it is a string a reader can `grep` for, and
`tests/hover.rs` resolves every one of them against the page in
`tests/transcribed_constants_cite_the_mock.rs`'s pattern.

**The table is the console's own rows.** `TIPS` is `[(&str, &[Tipped]); PROBES.len()]`, one entry
per row of `input::PROBES` and in that table's order — `karakuri/src/main.rs`'s `ASKED` shape one
crate over, and for its reason: **a control added to that table arrives here as a compile error**.
A row whose controls the mock draws with no tip carries an **empty slice**, which is the
maintainer's reading rule said as a value rather than as a silence: *the mock is not exhaustive …
there will be gaps in the functions too*. Four rows are empty today and `tests/hover.rs` names
them, so a row that loses its tips fails rather than going quiet.

**The dwell is `egui`'s own `Interaction::tooltip_delay`, read and not transcribed** — 0.5 s in
`egui` 0.36's default style. The console paints its own layer and still waits as long as
everything else on the operator's machine, and a caller that restyles the context moves both.

**The layer declares a deadline and not a region.** It is not a node of the arrangement — a tip is
drawn over the whole console and belongs to no bay — so it is not something `View::declares` can
name or `tests/schedulable.rs` can sum. What it carries is
[ADR-0283](0283-a-region-declares-when-its-picture-next-changes-not-that-something-is-pending.md)'s
third number: `Change::Tip` is *when is this layer's picture next different from the one on
screen*, which is the remainder of a dwell while one is running, **nothing at all once the tip is
up** — the box does not move while it is shown — and a frame **now** only when a tip is on screen
that must not be. Its cost is on the operator's own side of ADR-0210's line: a hand held still is
a gesture, and the frames it buys are frames somebody asked for.

**The box is the mock's `[data-tip]::after`, term for term**, and the constants cite it. It is
anchored to the **pointer** rather than to the control, which is the one place this departs from
the page, and it flips at an edge rather than being pushed: a probe answers *is the pointer on
this control* and not *where is it*, so hanging the box off the control's own rectangle would mean
every derivation handing one back.

## Alternatives

**`egui` widgets under the console's controls, with `egui`'s tooltips.** The real alternative, and
it is what the roadmap named. `Ui::interact` over each control's rectangle plus `on_hover_text`
would draw a tooltip with the dwell, the fade, the placement and the edge flipping already
written — a genuine saving, and it is the reason this was open for as long as it was.

It loses on the two records above. **ADR-0156**: the arrangement is application state four
surfaces write to, and hit testing that arrives with a widget is a second answer to *what is under
the pointer* — one that would live in `egui::Memory` under an `Id`, keyed by rectangles this crate
would have to feed it every frame, and one that `claim` would then have to be reconciled with.
Rule 4 is currently one sentence: *if the pointer is on a control the console draws, the event is
the panel's*. With widgets it becomes two rules and an ordering between them, and the failure when
they disagree is a control that stops taking presses with nothing saying so — which is exactly the
defect `input::PROBES`' own documentation records for the mixer strip.

**P-0085** is the second half: what would be bought is a dwell timer, a box and a flip, which are
together about ninety lines here, and what would be paid is a second hit-testing system for the
lifetime of the panel. **What would revive it** is `egui` owning a control on this console for
some other reason — a text field is the likely one, and there is already one place letters are
typed — because then the second system is present whether or not tooltips use it, and this
argument's premise is gone.

**A transcription of the tips in Rust, held equal to the page by a test.** The other half of the
choice this record had to make, and the one `tests/transcribed_constants_cite_the_mock.rs` already
does for `room::size`. Rejected on what is being transcribed: those are 156 *numbers*, and these
are about sixty passages of prose several hundred words each — the manual's own voice, copied into
a source file, where every edit to the page is a diff somebody has to mirror. The test would hold
them equal, so this is not a correctness argument; it is that the copy is the size of the manual
and the page is the half that moves. What it would have bought is the 340 KB the embedded page
costs in the binary and the one parse at start-up, and neither is a price worth a second copy of
the manual.

**Parsing the page at run time from `docs/manual/`.** Rejected because an installed copy has no
`docs/` beside it, and a tip that is missing is indistinguishable from a control that has none.

**A key derived structurally from the page — the nth `data-tip` in document order, or a path
through the markup.** It is generated rather than transcribed, which is the better tier, and it is
wrong here: the ordinal of an element is not a fact about the control, so a tip inserted in the
transport row would silently re-key every control after it. The citation is checked; an ordinal
would be checkable only against itself. The day the page carries a `data-control` attribute this
is a five-line deletion, and that is the thing to do rather than this.

## Consequences

- **M5.11's blocker is gone and its remaining work is drawing.** The roadmap's M5.11 says so, and
  what it now owes is per-control coverage rather than a decision.
- **M5.12's learn has somewhere to live.** *the assignment lives in the control's own tooltip* was
  waiting on this and on nothing else; the tooltip exists, and the words are already in the page —
  96 of the mock's 144 tips end with a `MIDI:` clause, which is where a learned assignment goes.
- **`input::PROBES` gains a second reader**, and it is the one that makes the table's shape pay
  twice: `main.rs`'s `ASKED` says every control is *asked*, and `hover::TIPS` says every control
  *explains itself*. Both are `PROBES.len()` arrays and neither can be added to quietly.
- **A row of `PROBES` that claims several controls explains them at the granularity `TIPS`
  writes.** Forty-seven controls are tipped, across twenty-six of the thirty rows; where a row's
  sub-question is not asked here the pointer resolves to the entry that is — the Sequencer bay is
  one tip over five kinds of control, and the deck head is four over seven. Those are entries to
  add, not decisions to take.
- **Rule 03's second clause is paid in the mock's tense, not the instrument's.** A tip is the
  page's prose, so *what it is* and *what a click will do* are exactly right and *what state it is
  in* is the state the mock is drawn in — the tempo figure's tip says the band is ±15% of *that
  number*, and the octave's says `×2` is refused *at this tempo*. A tip that read the live value
  would have to be composed rather than quoted, which is a second copy of the sentence by another
  route: what it would take is the page marking the substitutable spans, and that is a change to
  the manual rather than to this module. Written down here because the rule is the reason this
  exists at all, and paying two of its three clauses is worth saying out loud.
- **Editing the console page now rebuilds `karakuri-console`.** `include_str!` makes the page a
  compile-time input, so a tip rewritten in `docs/manual/console.html` reaches the panel by
  `cargo build` and cannot go stale — and the price is that a prose edit recompiles a crate. It is
  the right direction of the two: a page edit that did *not* rebuild would be a panel drawing last
  week's words with nothing saying so.
- **The layer does not know that a card is down.** `claim`'s rule 2 gives every press to whichever
  card is open, and this layer is handed `claim`'s answer rather than re-deriving its condition,
  so a pointer resting over a control an open card covers still resolves to that control. Closing
  it is a seam in `input` that answers *is a card down* for both; it is written down here rather
  than in a comment nobody meets.

**2026-09-10, the coverage.** The fourth consequence above is paid. `hover::TIPS` carries **76
controls across 34 of its 38 rows** — the table was thirty rows when this was accepted — and the
four empty slices are still the four: a pane head's name, a bay head's grip, the Library bay's list
and the Staging lane's `back` capsules. Five rows gained entries. The Sequencer bay's one tip is
eight: the four bank pills, a lane's cell, a lane's label, the mode pill and `+ lane`. The class
pills and the deck preview cells are four entries each, which is the one place `Cite::nth` carries
the whole of a distinction — the four class pills are one class and one text in the page and are
told apart by their ordinal. A sensitivity row has its `take back` capsule beside its curve chip,
and the deck head has the capacity chip and `re-salt`.

**Every sub-question is the press handler's own derivation asked one question finer**, which is
what this record said those entries were: `Sequencer::press` answers *four controls and one answer*
and the operation says which, `ProgramBay::cell` says which deck, `SensChip::operation` says which
chip, and `DeckHead::resized` and `re_salted` are the two build chips. Nothing was derived here that
could disagree with what the frame drew. Three entries are not one per control and say so:
`.scrub` is one element over both arrows, `.auth` is one over the three authority chips, and a
lane's cells and labels are as many as the pattern has rather than as many as the mock draws.

**The Master bay's five is what is left, and it is left deliberately.** Its three effect rows are
[ADR-0340](0340-kind-l5-is-written-and-the-master-chain-is-an-ordered-list-of-them.md)'s: the chain
becomes an ordered list of kind L5 procedures, so a citation per row would be written against a bay
about to be rebuilt. They are owed to the roadmap's M5.16 pass 2b rather than to M5.11.

**And `tests/hover.rs` gained the count.** The array being `PROBES.len()` long stops a *row* going
untipped and says nothing about a row that claims five controls and explains one — the failure this
record described as *"the pointer resolves to the entry that is"*, which is a control confidently
saying its neighbour's words. A row now carries one entry per control `input::PROBES` says it
claims, unless `UNEVEN` names it with the reason and with the count it does carry — an excused row
is out from under `PROBES`' number, so without one of its own it could lose an entry in silence. What is still not caught is a control the page
tips that no row of `PROBES` claims: the register is the console's own and this layer reads it,
which is that table's documented blind spot rather than a second one.
