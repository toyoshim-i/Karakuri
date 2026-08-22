# A workaround written in prose is a missing feature

When the fix for a defect is a sentence added to a prompt, a corpus, or a guide, the defect is
still there and now it has a distribution channel. Fix the feature.

**What it rules out.** Writing *"set exposure to 0.05"* into a craft corpus — which is a tone
mapper written in English and shipped in every prompt forever, and since the right value follows
the element count it ends as a table. The test that catches this: a **fact about the engine**
(where the default camera is, that `uint / uint` truncates) belongs in the specification; **taste**
(use `curl` at 0.3–0.5 for a shell) belongs in a corpus; a **workaround** belongs in neither, and
noticing you are writing one is the signal to go and fix the thing.

**The cost is not only delay.** Compensating for the missing tone mapper inside each artifact's own
`exposure` breaks the mixer downstream: a fader assumes every Set arrives at a sane nominal level,
and one authored at 0.05 and another at 1.6 do not mix. A workaround does not stay local.

**Where it holds.** [roadmap.md](../roadmap.md). Decided in
[ADR-0019](../adr/0019-exposure-is-three-things-and-none-stands-in-for-another.md) and
[ADR-0020](../adr/0020-a-corpus-expresses-taste-and-never-a-missing-feature.md).
