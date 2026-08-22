---
id: 0135
title: One record enum, and one classifier behind both predicates
status: accepted
date: 2026-08-22
supersedes: []
superseded_by: []
principles: []
tags: [store, records, metadata]
---

# One record enum, and one classifier behind both predicates

## Context

The metadata file is a third vocabulary beside the Set file's and the session stream's, and
`docs/ir-spec.md` says it is read by a *separate decoder*. That reads like an argument for a
separate Rust enum.

## Decision

**One `Record` enum for all three vocabularies**, and **one private
`Record::vocabulary() -> Vocabulary` that both `is_set_state` and `is_metadata` read off.**

## Alternatives

**A separate `MetaRecord` enum.** Rejected, and the argument is not taste. With a separate
enum, a `param_decl` fed to the Set decoder deserialises to `Record::Unknown` — and the
format *promises* to pass an unknown record over, preserving it verbatim. So `write_set`
could not refuse a metadata record without breaking that promise, and refusing it is the
whole point. The spec's "one `t` means one shape" is a rule about **names**, and the names
stay disjoint (see [ADR-0134](0134-the-metadata-preview-becomes-thumbnail.md)).

**Widen `is_set_state`'s refusal list to cover the four.** Rejected for the reason its own
doc gives about `Record::Save`: the function asks *whose state is this*, its documented
count would gain a third meaning, and one function would have two jobs.

**Two exhaustive matches, one per predicate.** This was the first shape and it is the
interesting rejection. Two exhaustive matches stop a record being classified *nowhere* —
the compiler catches a missing arm — but they do not stop it being classified
*differently in each*, which is exactly how the two predicates drifted apart the first time.
One classification, two readers of it.

## Consequences

Adding a record forces a decision in one place. **The claim has one stated exception**:
`mix::change` ends with a `_ => Ok(None)` wildcard, so a new deck record is silently "not a
mix change" there. That is the safe default and it is named in both files rather than left
to be discovered, because the alternative — listing every ignored record — would have to
grow for every record in the format, including ones a given build does not know.
