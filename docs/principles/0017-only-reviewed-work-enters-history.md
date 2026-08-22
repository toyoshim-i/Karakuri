# Only reviewed work enters history

An agent leaves its work in the tree and does not commit. The reviewer runs the tests, reads the
code, fixes what is wrong, and commits. Before anything fans out, the **seam is cut serially** —
the shared types several passes must agree on, the binding layout above all, published as a
public contract rather than a text to be grepped.

**What it rules out.** Agents committing their own work, which is cheaper and puts the defects in
history. In one day it caught a valid procedure being rejected by a rule the brief itself got
wrong, a seam that made two unaddable costs addable, and a requirement that had to be walked
back — each **reported by the agent that was told to do it**, which is the behaviour this
selects for. It also rules out fanning out by crate before the seam exists: agents invent
incompatible types, and the merge costs more than the parallelism earned.

**Where it holds.** [contributing.md](../contributing.md). Decided in
[ADR-0016](../adr/0016-agents-leave-work-in-the-tree-and-the-reviewer-commits.md).
