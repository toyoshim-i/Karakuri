# This machine is not the reference

A design is checked against **4 GiB of dedicated VRAM** — the size a default has to be comfortable on,
not a limit anything is held to. Beyond it, the performer's machine decides. Development happens on a
desktop; a performance is likelier a laptop.

**What it rules out.** Using the development machine as evidence. It has been done twice: once
concluding that GPU timestamps work, when it advertises a feature it does not deliver; once concluding
that a wasteful element layout could wait, because four resident slots fit here.

**And waste that buys nothing does not wait for a budget.** The test that separates the two: **does
using it buy anything?** Capacity buys elements. A resident Set buys instant switching. Sixteen bytes
holding a four-byte `seed` buys nothing at any size on any machine — so it is not a budget question.
Amplification multiplies exactly the stride such a layout wastes: one `amplify 64` stage is +1.25 GiB,
which is the reason to fix it, and *it fits here* was hiding it.

**Where it holds.** [roadmap.md](../roadmap.md), "Settled decisions". Decided in
[ADR-0110](../adr/0110-this-machine-is-not-the-reference.md).
