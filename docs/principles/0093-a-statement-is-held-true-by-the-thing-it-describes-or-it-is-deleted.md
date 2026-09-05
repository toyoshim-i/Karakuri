# A statement is held true by the thing it describes, or it is deleted

Where one fact is stated twice and the two can disagree, make them one thing or delete one. Where a
fact exists as data in the program, publish the generated form: the builtins and signatures handed to
a model come from `Builtin::ALL` and `signature()`, the checker's own table, so the moment they went
stale compilation would fail. Where an invariant can be checked mechanically, a test checks it — a
crate that scans its own source for `Instant::now` makes *rendering reads only the local oscillator*
a property of the crate rather than a claim about it. **A guarantee is structural, or it says which
convention holds it** and points at what enforces it.

**Where neither is possible, the statement dates itself.** A claim that is not yet true says which
parts hold today, which costs a clause and buys the reader the ability to trust the rest, and is a
marker rather than a resting place. A settled section marks which of its parts are forced and which
are chosen, so a wrong one is a one-clause revision instead of a reopened decision.

**What it rules out.** A vocabulary list maintained by hand. An invariant asserted in a README and
enforced by review: everything it was meant to catch was found by a test a reviewer had already read
past. Claiming the compiler enforces something it does not — a brief asserted the borrow checker made
a mid-frame swap impossible, and it did not, because the command encoder belongs to the caller and
borrows nothing; a review demonstrated it by recording two Sets of different capacity into a single
submit, and the guard that closed it makes a second `begin_frame` an `E0499`. A comment describing
replaced behaviour, the sharpest of which sat *inside the function implementing the change* — `t is
constant across a frame's substeps` in `VideoSource::render` — and two comments that were wrong
rather than stale, both saying the tail of the draw range holds the dead elements when it holds the
newest live ones, so anyone who believes them optimises by truncating the range and drops living
elements. A source comment citing a plan: 71 references to the roadmap across 26 source files against
one reference to an ADR or a principle in the whole of `crates/`, and a milestone named without a
filename is the same citation and harder to find, because searching for the document does not catch
it. A workaround written in prose, which is a missing feature with a distribution channel — *set
exposure to 0.05* is a tone mapper written in English, and since the right value follows the element
count it ends as a table, 0.05 at 262144 and 0.5 at 16384, while one Set authored at 0.05 and another
at 1.6 do not mix. A name meaning two things: `{"t":"param"}` used for two differently shaped
records, and a `noise` that is signed at confidence 0.1 on the bus against a declared generator
mapped to `[0,1]` at confidence 1.0 — it is not enough to be disjoint in practice, and the bus entry
was deleted rather than reconciled because completeness already came from the unknown-name arm. And a
goal stated in the present tense: *the record stream is the only path that mutates engine state*,
written unconditionally while `Record::Tick` was never constructed anywhere.

**Where it does not reach.** `docs/contributing.md` is the entry document — the ADR and principle
rules are first revealed there — so it quotes in full on purpose. `docs/adr/` is history: a record is
corrected where it was wrong and annotated with what it became, and never brought into step with the
present. What this rule forbids is an **independent copy that drifts unnoticed**; a quotation that
names what it quotes is checkable and is allowed.

**Where it holds.** The three mechanisms are all in the tree.
**Generated**: `Builtin::ALL` and `signature()` in
[builtin.rs](../../crates/karakuri-ir/src/builtin.rs) are the checker's own table, and
[mcp.rs](../../crates/karakuri-environment/src/mcp.rs) serves the builtin list and the signatures a
model is handed out of them rather than out of a second copy — a resource rather than a link,
because it travels over the connection that is already open.
**Tested**: [no_clock_access.rs](../../crates/karakuri-signal/tests/no_clock_access.rs) scans
`karakuri-signal`'s own source for `Instant::now` and its kin, and
[naga_test.rs](../../crates/karakuri-codegen/tests/naga_test.rs) puts generated WGSL through a
compiler, because a generator that has never been through one emits plausible invalid code.
**Structural**: [deck.rs](../../crates/karakuri-engine/src/deck.rs)'s frame guard owns the encoder,
so a second `begin_frame` is an `E0499` and a frame cannot be built from two generations of Sets —
and `Deck::slot` returns a shared reference for the same reason, since `&mut HotSwap` would reopen
the hole. Where it is a convention instead, it says so and names its holder: `ndjson.rs` in
[karakuri-store](../../crates/karakuri-store/) keeps an unknown record verbatim rather than
re-serialising a payload-free `Unknown` into `{"t":"unknown"}`, and `bus.rs` in
[karakuri-signal](../../crates/karakuri-signal/) is where the second meaning of `noise` was deleted
rather than reconciled. [ir-spec.md](../ir-spec.md)'s *Multiple L1 sources, and `source`* marks two
clauses **preference rather than force**, which is what made one of them a one-clause revision when
it turned out to be wrong. [contributing.md](../contributing.md) §4 carries the two `grep` commands
that count roadmap citations in `crates/` against ADR and principle citations, so the ratio this
rule was written from cannot go stale in silence. Decided in
[ADR-0010](../adr/0010-one-t-value-is-one-record-shape.md),
[ADR-0017](../adr/0017-an-invariant-that-can-be-tested-is-a-test.md),
[ADR-0019](../adr/0019-exposure-is-three-things-and-none-stands-in-for-another.md),
[ADR-0020](../adr/0020-a-corpus-expresses-taste-and-never-a-missing-feature.md),
[ADR-0031](../adr/0031-a-document-describing-replaced-behaviour-is-worse-than-none.md),
[ADR-0034](../adr/0034-the-frame-guard-owns-the-encoder.md),
[ADR-0051](../adr/0051-a-name-means-one-thing-so-the-buss-noise-entry-is-deleted.md),
[ADR-0063](../adr/0063-an-invariant-that-is-not-yet-true-says-so.md),
[ADR-0064](../adr/0064-a-replaced-passage-is-read-to-its-end.md),
[ADR-0083](../adr/0083-mcp-is-the-only-prompt-surface.md),
[ADR-0111](../adr/0111-a-name-lives-in-the-set-file-and-may-be-written-on-the-command-line.md),
[ADR-0121](../adr/0121-moving-code-leaves-its-reasoning-behind.md),
[ADR-0149](../adr/0149-source-cites-what-is-in-force-not-a-plan.md) and
[ADR-0151](../adr/0151-an-adr-is-a-description-of-history.md).
