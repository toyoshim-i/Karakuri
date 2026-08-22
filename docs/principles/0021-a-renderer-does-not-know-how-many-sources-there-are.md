# A renderer does not know how many sources there are

Elements carry an implicit `source`, and L2 masks on it — that is how one geometry source is
tinted, scaled or modulated apart from another. **L4 never branches on `source`.** L2 writes
`tint` through the mask; L4 draws `tint`.

**What it rules out.** `if source == 0` inside a renderer. The moment it appears, that L4 depends
on the Set's source arrangement and stops being reusable; kept out, an L4 written for one source
works unchanged with five.

**And it settles which case is the main one.** Multiple sources are mostly about keeping them
apart, not mixing them: distinguishing needs no new machinery, while only position interpolation —
pairing A's `seed 5` with B's — is a genuine addition, because it is a read across elements, which
the IR otherwise has none of.

**Where it holds.** [ir-spec.md](../ir-spec.md), "Multiple L1 sources, and `source`". Decided in
[ADR-0025](../adr/0025-sources-are-distinguished-downstream-and-a-renderer-never-branches-on-one.md).
