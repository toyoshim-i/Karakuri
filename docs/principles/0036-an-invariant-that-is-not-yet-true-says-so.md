# An invariant that is not yet true says so

An invariant is a claim, and a claim has a truth value on a date. Where one is an aspiration, the text
says which parts hold today — it costs a clause and buys the reader the ability to trust the rest.

**What it rules out.** Stating a goal in the present tense. *The record stream is the only path that
mutates engine state* was written unconditionally while `tick` had no writer at all: `Live::steps()`
passed a `u8` straight through and `Record::Tick` was never constructed anywhere. The session seed,
the initial tempo and the bindings were not in the stream either — **audio, the newest subsystem, was
obeying the invariant that the oldest one did not.**

**Then close it.** The clause is a marker, not a resting place; every record type gained a writer
within a day, and the sentence went back to being unconditional because it had become true.

**Where it holds.** [README.md](../../README.md). Decided in
[ADR-0063](../adr/0063-an-invariant-that-is-not-yet-true-says-so.md).
