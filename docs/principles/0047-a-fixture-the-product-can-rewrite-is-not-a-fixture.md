# A fixture the product can rewrite is not a fixture

A test owns its inputs. `tests/fixtures/flat.kir` exists because tests had been deriving "a different
procedure" by substituting a sentence in `examples/soft_points.kir` — one of the files the MCP surface
**exists to rewrite**. When a live session rewrote it the substitution matched nothing, the two
procedures became identical, and the test failed with a message **pointing at the write path**, which
was fine.

**What it rules out.** Reading product-owned data in a test because it happens to be convenient. The
criterion is not *it probably will not change* but **it can change**. And where a test must mutate
input, mutate in a way that works regardless of content — **prepend a line** rather than substitute
one.

**Carry the fix to every site the argument covers.** The same file broke twice, because the first pass
repaired the writing side and left the reading side.

**Where it holds.** `tests/fixtures/` in [karakuri-cli](../../crates/karakuri-cli). Decided in
[ADR-0087](../adr/0087-a-fixture-the-product-can-rewrite-is-not-a-fixture.md).
