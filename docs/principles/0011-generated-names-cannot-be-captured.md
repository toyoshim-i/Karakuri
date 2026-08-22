# Generated names cannot be captured

Every IR-derived identifier is mangled on the way into WGSL, so the generated namespace is
structurally disjoint from anything a procedure can spell. No local can shadow the uniform
block, a helper, or a builtin, whatever it is named.

**What it rules out.** A blocklist of names to reject, which requires tracking WGSL's reserved
words forever and had already failed — a `param` named `array` passed all four validation stages
and produced WGSL that would not compile. More generally it rules out the reasoning that
produced the bug: **state why a class is closed, not why it is unlikely.** The specification's
own example wrote `let u = hash1(seed)`, and `u` was the uniform block.

**Where it holds.** `lower.rs` in [karakuri-codegen](../../crates/karakuri-codegen), with
regression fixtures that name their locals after every generated identifier on purpose. Decided
in [ADR-0014](../adr/0014-generated-code-cannot-be-captured-by-a-name-a-procedure-can-spell.md).
