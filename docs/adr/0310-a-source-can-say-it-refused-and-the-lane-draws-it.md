---
id: 0310
title: A source can say it refused, and the lane draws it
status: accepted
date: 2026-09-08
supersedes: []
superseded_by: []
principles: [0083, 0087, 0091, 0094]
tags: [engine, environment, console, manual, m5]
---

# A source can say it refused, and the lane draws it

> **Annotated 2026-09-09: one of the five words changed and the shape of this record did not.**
> [ADR-0316](0316-an-over-budget-candidate-stays-in-the-slot-and-the-slot-stops-updating.md) renames
> `swap::Event::RolledBack` to `Event::Overloaded` and `view::Stage::RolledBack` to
> `Stage::Overloaded`, and the word the lane and the health capsule draw is `overloaded`. Wherever
> this record says *`RolledBack` is the budget's verdict*, that is still exactly what it is — a
> different fact about a different moment from a checker's refusal, which is the distinction argued
> below and is untouched. What did change is what the verdict *does*: the version stays in the slot
> and the slot stops updating, so the alternative rejected here — *"a word meaning your Set was too
> expensive used for your file has a typo"* — is if anything sharper, because the first of those is
> now also a slot that is not running.
>
> **The consequence about a refusal waiting behind a trial was already false** and this record's own
> last-but-one bullet says so, per ADR-0313.

## Context

`karakuri_engine::swap::Source::poll` answered `Option<Request>`, and the whole of what a source
could say was *here is something to build* or nothing. So
`karakuri_environment::watch::Watch`, which is the only real implementation of that trait, did
what the trait left it: a `.kir` the checker turned down was printed to stderr and the poll
returned `None`.

**`None` is also what a poll that saw no edit returns.** The two are the same answer, and one of
them is the operator's newest save disagreeing with what is on screen. Nothing downstream could
tell them apart, so no `Request` was made, no `swap::Event` was emitted, and the Staging lane —
the bay that exists to say *the file and the picture do not agree* — drew nothing at all. The
disagreement an operator most wants mid-set was said on a terminal nobody is watching.

Three documents had already named it and none could close it. `roadmap.md`'s M5.7 called it one
of *"two findings this lane exists for, and neither has a row anywhere"* and priced it: *"an
engine change, `Source::poll` having no way to say I refused."*
[`karakuri-console`'s `view::Stage`](../../crates/karakuri-console/src/view.rs) carried the gap
in its own documentation — *"this lane cannot draw it — which is a real gap"* — and
[console.html](../manual/console.html) carried it in prose, as *"one disagreement this lane
still cannot show, which is the one it would most like to."*

This is [P-0094](../principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)'s
*be loud* clause failing at the last step rather than at the first. The refusal is the earliest
and cheapest of the three admissible answers — it happens before anything is built, costs a
frame nothing, and leaves the picture where it was — and being loud about it is the part that
was not built. A refusal nobody is told about is not a refusal; it is a save that did nothing.

## Decision

**`Source::poll` answers `Option<Polled>`, and `Polled::Refused` carries the diagnostics.**

```rust
pub enum Polled {
    Build(Request),
    Refused(Refusal),
}

pub struct Refusal {
    pub label: String,     // what the material calls itself
    pub said: Vec<String>, // every diagnostic, one line each
}
```

- **The seam is the engine's**, because *what a source has to say* is the engine's vocabulary and
  a channel beside `poll` would be a second one. `None` still means *nothing happened*, and the
  case that is not nothing now has a word.
- **`swap::Event::SourceRefused { label, said }` is the outcome**, beside `Swapped`, `Rejected`,
  `Accepted` and `RolledBack` — a fifth word and not one of theirs. `Rejected` is *this Set would
  not build* and `RolledBack` is the budget's verdict; this is *there was never a Set to build*,
  which is a different fact about a different moment
  ([ADR-0042](0042-a-silently-wrong-image-loses-to-a-loud-failure.md)'s *convert the silence back
  into a failure at the place that caused it*, one level away from the picture).
- **It carries no `id`.** Every other variant does, because a caller matches an outcome back to
  the `Request` that produced it. A refusal produced no request, put nothing in the store and
  filed no version, so an id would be a thread to a build that does not exist — which is
  [ADR-0089](0089-history-is-gated-on-compiling-not-on-landing.md)'s gate said in a type. The
  watcher's `builds` counter does not advance for one either.
- **The diagnostics are formatted on the worker.** `compile::Diagnostics` comes back from the
  checker with the terminal's rendering and the row's one-line form already built, beside the
  four validation stages that produced them; the render thread moves two allocations into an
  `Event` and does no formatting at all
  ([P-0091](../principles/0091-cost-is-known-before-it-is-paid.md)).
- **The row's line is a slice of the terminal's, not a second rendering.**
  `Diagnostics::said` is the first line of each rendered diagnostic — `IrError::render` writes
  *line:col: stage: message* and then the source line, the caret and the hint — so the sentence a
  lane draws cannot come to disagree with the one a terminal prints
  ([P-0087](../principles/0087-name-the-property-never-the-shape.md)).
- **The lane draws a row on the word `did not compile`**, carrying the first diagnostic and a
  count of the rest — `console.html`'s lane and its first `data-tip` moved first, and
  `view::Stage::NotCompiled` and `view::Candidate::said` follow it. Every diagnostic goes to the
  places that have room for them: stderr, which keeps the line it always printed, and the MCP
  surface, whose reader is the program writing the next attempt
  ([P-0083](../principles/0083-a-refusal-carries-what-the-next-attempt-needs.md) — the whole file
  at once, so the repair is one round trip).
- **The transport's health capsule says it too**, off the same drain and in the plain treatment:
  `armed` is *this is working*, and nothing was built.
- **The stderr line stays.** A terminal is a surface, and it is the only one a headless run has.

## Alternatives, and why they lost

### A refusal as a `Rejected` event, reusing the word the engine already has

The smallest diff by a wide margin: `Watch` synthesises a `SetError`, the existing arm reports
it, and the lane's `refused` row appears with no new variant, no new `Stage` and no page change.

**It loses on what the word would then mean.** `Rejected` is *the files checked and the Set they
were assembled into would not build*, and `console.html` states that as the row's definition —
*"Refused on a row is a build that failed."* A source the checker turned down did not reach a
build, and the two failures want different repairs: one is a composition the operator has to
reconcile across files, the other is a syntax error at a line and a column. Spelling them the
same is `docs/contributing.md` §4's *a name meaning two things*, and the cost of it lands on the
one reader who cannot ask — a model at the other end of `--mcp`, which would be told its
procedure failed to build when it failed to parse.

The same argument rules out `RolledBack` harder still, and that one is the budget's verdict: a
word meaning *your Set was too expensive* used for *your file has a typo* would make the one
number the watchdog exists to report unreadable.

### A refusal as a `Request` that is built and fails

Keep `poll` as it is, and have the watcher hand over a request it knows will fail — an empty
`l4s`, a deliberately broken `Checked` — so the existing rejection path reports it.

Rejected as a lie with a cost. It spends a build worker's cycle, a `catch_unwind` and possibly a
device on producing a failure that was already known before the request was made, and the error
the operator reads would be whatever `Set::build` happened to say rather than what the checker
said — the diagnostics, which are the entire value of the refusal, would have to be smuggled
through a `SetError` variant invented for the purpose. It also puts a request into a queue that
`install_if_ready` collapses to the newest, so a refusal could be silently superseded by a later
build; a refusal is not a candidate and must not be treated as one.

### A second channel beside `poll`

A `Sender<Refusal>` handed to the source at construction, or a `fn refused(&mut self) ->
Option<Refusal>` with a default of `None` that the worker asks after every poll.

**It loses on ordering, which is the one thing a stream of verdicts has to keep.** A save that
does not compile followed by a save that does is a refusal and then a swap, in that order, and an
operator reading them the other way round would see a working build reported as broken. One
`mpsc` is FIFO; two are not ordered against each other at all. The two-method form has the same
defect one step earlier — the worker would have to decide which to ask first, and a source would
have to remember a refusal across a call it did not make.

It is also a second answer to *what did this poll produce*, where there is exactly one poll and
one answer. `Polled` is that answer.

### Say it only on the MCP surface, and leave the panel alone

The refusal already reaches a model through `Reporter::swap` the moment it becomes an `Event`, so
the cheap half of this record could stop there.

It answers the wrong reader. The person whose attention this is competing for is at the panel,
mid-set, and the surface they are looking at is the lane — which is the bay whose stated purpose
is a file that disagrees with the picture. A refusal visible only to a program is the same
silence with a better audience.

### A fourth `Stage`, against reusing `Refused` on the console side

Considered and taken, and it is worth saying why the console did not simply map the new event
onto the word it had. The lane and the health capsule read one enum, so a shared word would make
the capsule unable to say which happened either, and `Stage`'s own documentation would have to
carry the ambiguity the page had just resolved. The cost is one word — `did not compile` — and
one row of prose per surface.

## Consequences

- **`swap::Event` has six variants and `view::Stage` has four**, and the mapping is
  `crates/karakuri/src/main.rs`'s `verdict`, which is a `match` with no wildcard: a seventh event
  stops the build there rather than being passed over.
- **`Source::poll` is a breaking change to the trait**, and there are four implementations in
  this workspace: `Watch`, `Receiver<Request>` (which cannot refuse, and says so at the impl),
  and `Silent` and `Exploding` in `tests/hot_swap.rs`. `Silent`'s documentation said it was *what
  a `.kir` file that failed to compile leaves behind*; that is no longer what a refusal looks
  like, and it now says so.
- **`compile::check` is unchanged for every caller but one.** `compile::diagnose` is the same
  five stages with the diagnostics kept apart, `check` is one line over it, and `Watch::rebuild`
  is the only site that took the new one — `grep -rn 'compile::check' --include='*.rs' crates/`
  is the rest of them, and none of them has a row to draw.
- **Only the checker's refusal is reported.** The two other paths in `Watch::rebuild` that answer
  nothing — a file that will not read, and a stack that will not sort into a Set — still print
  and return `None`. They are refusals of a different kind and `did not compile` is the wrong
  word for either; giving them one is a further decision and not this one.
- **A refusal that arrives while a candidate is on trial waits for the verdict**, because
  `install_if_ready` returns before the drain while a trial is open and an `mpsc` cannot be read
  past. On a Live slot that is a judging window; on a parked slot, whose trial is frozen, it is
  until the slot goes on air — which is the same wait that slot's own verdicts are already under.
  The comment at the guard says so.
- **`Polled` and `Done` are unboxed, with `clippy::large_enum_variant` allowed at each and the
  reason written there.** One `Polled` exists at a time on the worker's stack, and `Done`'s
  wasted room is one refusal's worth of an `mpsc` node that is allocated either way.
- **`docs/manual/console.html` moved first**, and the lane has a `data-tip` for the first time —
  the roadmap's own note that *"the mock draws the lane with no `data-tip` anywhere in it"* is one
  tip less true. The page's *one disagreement this lane still cannot show* paragraph is replaced
  by what it now shows, and the health capsule's note and pill tip name the fourth word.
- **Four tests hold it, each watched to fail against the shape it replaced.**
  `karakuri-environment`'s `a_file_the_checker_turns_down_is_a_refusal_carrying_its_diagnostics`
  runs a real watcher over a real example and carries the repair as its negative control;
  `karakuri-engine`'s `a_source_that_refused_says_so_and_the_live_set_is_untouched` is under
  `mod gpu` because a `HotSwap` takes a device, and asserts the live Set's capacity and that no
  trial started; `karakuri-console`'s
  `a_row_the_checker_turned_down_draws_the_first_diagnostic_and_counts_the_rest` reads the
  sentence off the painted frame rather than counting shapes; and `crates/karakuri`'s
  `every_verdict_says_what_it_does_to_the_lane` carries the new arm and the half where a build
  landing on a refused row clears the diagnostics with it.
- **What is still not said is the node.** A verdict is over a build and the lane is addressed by
  the deck letter, which M5.7's item 4 is about and this record does not touch — except that a
  refusal knows exactly which file it was about, and hands its name over as the row's label. That
  is finer than any other row's and it is not the node address either.
