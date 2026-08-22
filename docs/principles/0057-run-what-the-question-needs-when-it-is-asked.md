# Run what the question needs, when it is asked

Whoever makes a change names **the smallest suite that answers it** and runs that. The whole workspace
runs once at a boundary — before a tag, after a refactor, when the answer matters. `pre-commit` checks
formatting on staged Rust and nothing else; `pre-push` runs everything **on a tag push and on nothing
else**, because a tag is the deploy and a branch push is part of working.

**What it rules out.** Hanging the suite on a git lifecycle event. **A git event does not know when
"needed" is** — it re-ran a full suite against a tree that had produced `pre-push: ok` an hour before,
and it judged the working tree rather than the commits being pushed. And a push is not a debugging
step: settling quality at the moment of deploying means learning too late.

**The waste is usually self-inflicted.** Three agent briefs each ordered a full `fmt` + `clippy` +
`test --workspace` before reporting — twenty-five minutes of the same 896 tests, of which one run was
meaningful. An agent's question is *did my change break something*, and the minimal answer is a named
suite.

**Divide work by file, not by phase.** Three commits that all touch one crate can only be done in
order; the same work split by file runs in parallel and lands as it finishes.

**Where it holds.** [.githooks/](../../.githooks); [contributing.md](../contributing.md). Decided in
[ADR-0114](../adr/0114-tests-run-when-somebody-asks-not-when-git-does.md) and
[ADR-0115](../adr/0115-split-work-by-file-not-by-phase.md).
