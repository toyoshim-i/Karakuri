---
id: 0179
title: A transcribed number cites the rule it was copied from, and the citation is checked
status: accepted
date: 2026-08-25
supersedes: []
superseded_by: []
principles: []
tags: [ui, testing]
---

# A transcribed number cites the rule it was copied from, and the citation is checked

## Context

`karakuri-console` is built from `docs/manual/style.css` by hand. Seventy-six numbers — a bay's
radius, a pill's padding, every divider, every gap — are literals in Rust that mean *what the mock
says*.

**A wrong transcription was invisible.** The crate's tests state a region's arithmetic *in terms of*
those constants, so a test of that arithmetic cannot catch a constant that is wrong: it compares the
layout against the same wrong number. That is not a hypothetical —
[ADR-0177](0177-the-transport-row-shows-what-the-console-can-know.md) records `.transport`'s
`gap: 14px` transcribed as 10, with its test green. **Twenty-three of the constants were named by no
test at all**, and `lib.rs`'s five were private, so no test *could* name them.

## Decision

**One test reads the checked-in source and the checked-in stylesheet and holds every transcribed
number to the rule its own doc comment cites** — the shape
`crates/karakuri-engine/tests/gpu_tests_are_under_mod_gpu.rs` already uses for a convention, with
floors so the scan cannot pass by finding nothing.

**The doc comment is the citation, and it stays prose.** Sixty-nine of the seventy-two needed no
change: the rule read out of what was already written is *a source in backticks — a selector, or a
file under `docs/manual/` — and a declaration in backticks containing a colon, binding to the
nearest source named before it*. That let `` `.sink`'s own `gap: 6px` `` and *"`.vfader s`'s
`height: 9px`, and its `left: -2px; right: -2px`"* stand as they were. **A machine format was
rejected**: these comments are read by people first, and 72 of them rewritten into an attribute
would have bought the same check at the cost of the thing that makes them worth reading.

**Three kinds, told apart, and a constant declaring none fails.** *Derived* is read from the
initializer rather than the prose — it names another constant, so the compiler holds the arithmetic
and no citation is demanded. *Transcribed* is a literal with a resolvable citation. *The console's
own* is a literal phrase in the doc. A literal with none of the three panics telling the writer
which to declare, and **that half is what makes this a convention rather than a lint**: the next
number written has to say where it came from.

**The stylesheet is the specification, so the check fails in that direction too** — change
`.fader-col`'s height in the CSS and the constant that no longer matches is named. That is the
direction that will actually happen.

## What it found

**No wrong number.** All seventy-six match, including the twenty-three nothing had ever named. That
is the result, and it is clean.

**One wrong citation, of exactly the invisible kind.** `PILL_GAP` cited *"`.bay-head`'s inner
`gap: 5px`"*; `.bay-head`'s `gap` is `0.8rem`. The 5px is real and lives **inline in
`console.html`**, on the spans that hold the pills. Right number, wrong source — a guard that
trusted the citation would have resolved it against a rule that says something else.

**A duplicated number, collapsed.** `PANE_DIVIDER` (`room::size`) and `INSPECTOR_DIVIDER` (`lib.rs`)
were the same 9 from the same declaration, written in two files, and **nothing named either** — they
could have drifted apart with both files green. `lib.rs` uses `size::PANE_DIVIDER` now, and if
`.insp-split`'s `1fr 9px 1fr` changes, one test fails once naming one constant.

**A comment that misattributed a declaration.** The deleted `INSPECTOR_DIVIDER` ended *"the mock
draws it as a grabbable bar (`cursor: col-resize`)"*. Read by a person, *it* is the divider and the
attribution is right; read by the nearest-source rule, `cursor` binds to `.insp-split`, which does
not set it — `.divider-v` does. **This is the one shape where the citation rule is weaker than
prose**, and it was not worked around: the fact was carried into the surviving comment with its real
owner named, so it is now checked.

## Consequences

- `HAIRLINE` was the last number nothing checked, and it now cites `.bay-head`'s
  `border-bottom: 1px solid var(--c-hair)`. The other rules in the mock are all one pixel too, so
  the citation is one of many rather than an arbitrary pick: it is the one this constant is drawn
  for. **Zero of the seventy-six are unchecked.**
- The guard reads two files, and each source must contribute at least one constant — a marker that
  survives while the constants move out from under it is loud rather than a quietly smaller scan.
- `PROGRAM_DIVIDER`'s comment says *"the second divider in the console that is not 10"*, and its
  pair — *"the one divider that is not 10"* — now lives in the other file. The two sentences were
  written to be read together and now span two files. Left as it is and recorded.
- **It is one crate's rule.** No principle is written, because `karakuri-console` is the only crate
  transcribing a specification held somewhere else; a second one is when this becomes a rule of the
  system rather than of a file.
