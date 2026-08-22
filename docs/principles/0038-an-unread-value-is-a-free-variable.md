# An unread value is a free variable

A channel nobody writes and nobody reads is unconstrained, and it has been out of range the whole time
with no way to notice. **The defect is created by the reader, not by the writer.** L4's alpha was dead
data — nothing wrote it, `present` and the meter read only `.rgb` — until `over` needed coverage, and
then every value the material can put there became an input: the IR says alpha above 1.0 is expected,
so a coverage of 1.5 subtracts instead of hiding and 2 inverts the sign. 1029 negative colour channels,
worst −54.34.

**What it rules out.** Assuming an existing field is in range because nothing has complained. Before
giving an unused value a reader, decide what it is allowed to be and enforce it at the reader.

**And a control tested only at its endpoints is untested.** `opacity` had never been exercised at
anything but 0 and 1; deleting it from the composite entirely left the whole suite green.

**Where it holds.** `composite.wgsl` and [deck.rs](../../crates/karakuri-engine/src/deck.rs). Decided
in [ADR-0070](../adr/0070-a-channel-nobody-reads-is-a-free-variable.md).
