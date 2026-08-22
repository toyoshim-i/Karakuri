# Compaction preserves order

Dead elements are removed by an order-preserving prefix-sum stream compaction. Survivors
keep their relative order, every run combines elements in the same order, and reproduction
is bit-exact.

**What it rules out.** A free list with atomic allocation. It is cheaper and it forfeits
this: GPU atomics complete in a non-deterministic order, so the same session would compose
in a different order on a second run. Floating-point addition is not associative, so that
is a visible difference and not a theoretical one — additive blending included. A free list
also leaves live elements scattered, so indirect dispatch over the live count would have
needed a scan to rebuild a contiguous list anyway; the cheaper option was not even cheaper.

**Where it holds.** [compaction.rs](../../crates/karakuri-engine/src/compaction.rs);
[ir-spec.md](../ir-spec.md), "Dispatch".
