---
id: 0060
title: The tempo octave is folded, not judged
status: accepted
date: 2026-08-02
supersedes: []
superseded_by: []
principles: [0035]
tags: [signal]
---

# The tempo octave is folded, not judged

## Context

A review found the tracker settling at **half tempo above about 160 bpm** — drum and bass, footwork,
hardcore, the top fifth of the search range.

The shape of the fault is the reason this record exists. **The octave check was disabled by exactly
the condition it exists to catch.** Its phase came from a single DFT bin at `1/period`; when the
period is twice the real pulse spacing, consecutive pulses land half a turn apart in that bin and
cancel, the magnitude goes to nearly zero, and the guard `on > 0.0` is false — so the check never
ran.

Worse, **it does not look like a bug**. A wrong octave collapses confidence, so it reads as *there is
no beat here*: below the gate the lock never acquires and the oscillator free-runs at `--bpm` for the
whole set.

A second finding from the same review: a 90 bpm kick with a hat at 0.15 between the beats read
**180 bpm at confidence 1.0** — a confident wrong answer, which the module's own documentation names
as the only failure that shows on stage.

## Decision

Take the operator's framing: **give the search a valid range, and make the range dynamic** — centred
on the current estimate, one octave wide — with manual ×2 / ÷2 keys as the backstop.

`tracking_window(centre) = [centre/√2, centre·√2]`, and
`fold(bpm, centre) = bpm · 2^-round(log2(bpm/centre))` doubles or halves the autocorrelation peak
until it lands inside.

**The judgement becomes arithmetic. Nothing decides anything.** Both heuristics — grid-versus-offbeat
energy, odd-versus-even comparison — are deleted, along with the repairs made to them hours earlier.
That those repairs were themselves fragile is the argument for replacing rather than continuing to
mend.

**One octave is a maximum, and the argument is tiling**: the octaves of a one-octave window tile the
tempo axis with no overlap and no gap, so every candidate has exactly one fold. Anything wider admits
both `T` and `2T`.

## Consequences

- **`--bpm` does two jobs, and it is not a coincidence.** It is the free-running default and the
  initial window centre, because both are "the tempo the operator believes it is".
- **Confidence changes meaning, which is the larger prize.** It had entangled *is there a beat* with
  *is the octave right*; folding removes the second question, so confidence now says only how well
  the grid fits the novelty. It is measured **at the peak, not at the folded period** — periodicity
  is a property of the material, the fold is a choice about naming.
- **The intended failure is asserted by a test.** A window that starts an octave off **locks an octave
  off and does not self-correct**; the escape is the key. The test exists so that nobody repairs it
  back into a heuristic in three months.
- **What is deliberately not fixed is documented where a reader will look**: folding by powers of two
  only addresses 2:1. A 3:2 error lands inside the window and looks correct.
- The window centre crosses to the audio callback in an `AtomicU32`, not a mutex, because "skip on
  contention" would mean **the window sometimes not moving** — and following is the mechanism. In one
  file, the values that may be skipped (measurements) and the value that may not (the centre) are
  told apart.

## Evidence

Session 2026-08-02T05:48Z–08:38Z, commits `8711b6b`, `a8a2a44`. Standing rule:
[P-0035](../principles/0035-arithmetic-and-an-operator-key-beat-a-rule-that-is-usually-right.md).
