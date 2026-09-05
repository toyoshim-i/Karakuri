---
id: 0242
title: The command line is test tooling, and the instrument's principles do not bind it
status: accepted
date: 2026-09-02
supersedes: []
superseded_by: []
principles: [0080, 0093]
tags: [process, docs, ui]
---

# The command line is test tooling, and the instrument's principles do not bind it

## Context

Three documents used *the instrument* for two different programs, and
[ADR-0220](0220-the-key-column-is-the-instruments-keyboard-and-the-clis-keys-are-its-own.md) said so
in as many words:

> ADR-0045 is accepted and unsuperseded and calls `karakuri-cli` one; `README.md` and ADR-0214 call
> it scaffolding rather than the destination; ADR-0213 and `crates/karakuri`'s own manifest use the
> word for the new binary. This record takes the last, and **names the collision rather than
> resolving it** — retiring ADR-0045's word is a separate decision about a record that was true when
> written.

This is that separate decision, and it is taken because the unresolved word started deciding things.
[P-0080](../principles/0080-an-operator-can-see-a-slots-own-material-without-putting-it-on-air.md)
was written with a *Where it is not met* section naming `karakuri-cli` as a surface that fails the
rule — one window, no cells, and no way to see a single slot on its own since
[ADR-0240](0240-the-output-shows-the-mix-and-residency-keys-belong-to-the-mixer.md) retired the `v`
key. It said the gap "is a gap rather than a decision: nothing about a one-window program makes the
requirement not apply to it." That sentence is what a registry entry does with an unanswered scope
question, and the answer was available: nobody plays this program.

The same collision runs the other way in the design. The retired operation *Choose what the output
shows* existed because a design made for the console — a picture and four cells — was bent to fit a
program with one window: with one window the only place to put a look is the output. A rule written
against the command line's shape is how that arrived, and a rule that binds the command line is how
it would arrive again.

## Decision

**`karakuri-cli` is test tooling.** It is a way to drive the engine, watch a file, record a session
and check that a change did what it said. It is not what a player plays, and no version of it is
planned to be.

**The instrument is `crates/karakuri`** — the console binary — which is the reading ADR-0220 already
took for the key column and the panel column of [every operation](../manual/operations.html).

**A principle stating what the instrument owes an operator does not bind the command line, and a
surface of `karakuri-cli` that fails such a rule is not a gap.** It is not a decision to be defended
either: the rule does not reach the program, so there is nothing there to meet or to owe.

**What does still bind it is everything that is not about the operator's surface.** Determinism, the
record stream, the render-thread rules, the test and process rules — those are about the system, and
`karakuri-cli` is in the system. The line is *who the rule is written for*: a rule written for the
person playing the instrument stops at the instrument, and a rule written about how the machine
behaves does not.

**ADR-0045's word is retired, and the record is not.** *The CLI is an instrument, not a demo* was
true of what it described in July: the alternative it beat was a window that opens and cannot be
touched, and the observations it lists are still the reason the keys exist. It is annotated with this
record rather than superseded, because what it decided — that the program is driven and says what it
did — is untouched.

## Alternatives rejected

**Leave the scope unstated and answer it per rule.** The status quo, and it is what produced a gap in
P-0080 against a program nobody plays. A registry read by whoever writes the next brief cannot carry
a rule whose reach is decided again each time it is cited; the reader who acts on the gap is the cost.

**Hold `karakuri-cli` to the instrument's rules anyway, as a second surface.** It is the generous
reading and it is what ADR-0220 rejected one column along for the same reason: it prints an
instruction that is wrong in front of the reader. Here it is worse than wrong, because the
requirement in P-0080 cannot be met by a one-window program without putting a candidate on the
output, which is the operation this project has just decided should never have existed. The
generous reading asks for the defect back.

**Delete `karakuri-cli`.** Out of scope of this record and not proposed by it. It is what the tests
and the watch loop run on today.

## Consequences

- **P-0080 loses its `karakuri-cli` gap** and states its scope in the rule itself. What remains under
  its *Where it is not met* is one entry, the console's residency gate
  (`docs/contributing.md` §4).
- **Every other `docs/principles/` file inherits this line**, which is the point and also the risk:
  no file names its scope today, so a reader deciding whether a rule reaches `karakuri-cli` is
  applying *who the rule is written for* by hand. A rule whose reach is not obvious should say, the
  way P-0080 now does.
- **The word *instrument* means one program in this repository from here.** ADR-0045 keeps its title
  and carries an annotation; ADR-0220's collision is closed rather than named.
- **`docs/roadmap.md` still lists the `karakuri-cli` audition gap** as one of two P-0080 leaves owed.
  It is one gap now, and that document is corrected where it is maintained rather than here.
