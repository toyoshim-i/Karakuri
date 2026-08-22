# A name means one thing across the system

One `t` value is one record shape in every file, so a decoder dispatches on `t` alone and needs to
know nothing about which stream it is reading. One signal name is one signal, at one confidence,
whichever code path reaches it.

**It is not enough to be disjoint in practice; they have to be disjoint by name.** `"noise"` answered
one thing on the signal bus — signed, `[-1,1)`, confidence 0.1 — and another through a `bind` record —
mapped to `[0,1]`, confidence 1.0 — and which you got depended only on which door you came in by. The
bus entry was deleted rather than reconciled, because bus completeness already comes from the
unknown-name arm, so it bought nothing and cost a second meaning.

**What it rules out.** Reusing a tag for a differently shaped record in a different file, which is
what `param` was doing across the metadata and the Set file. It follows that an unknown record is
preserved **verbatim** on a round trip: re-serialising a payload-free `Unknown` would fabricate
`{"t":"unknown"}` and destroy the line, which is worse than dropping it because it looks faithful.

**And delete what nothing reads.** Removing the dead bus entry made a `seed` field dead too, and it
went with it — a seed carried and read by nothing reads as randomness that has been accounted for.

**Where it holds.** [ir-spec.md](../ir-spec.md); `ndjson.rs` in
[karakuri-store](../../crates/karakuri-store); `bus.rs` in
[karakuri-signal](../../crates/karakuri-signal). Decided in
[ADR-0010](../adr/0010-one-t-value-is-one-record-shape.md) and
[ADR-0051](../adr/0051-a-name-means-one-thing-so-the-buss-noise-entry-is-deleted.md). Replaces the
retired P-0015, which stated this for record tags only.
