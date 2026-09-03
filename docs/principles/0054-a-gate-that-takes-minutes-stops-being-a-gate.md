# A gate that takes minutes stops being a gate

`pre-commit` runs `rustfmt --check` on the staged Rust: one second, so it can sit on every commit.
fmt, clippy `-D warnings` and the whole suite take minutes, so `pre-push` runs them **on a tag push
and on nothing else** — a tag is the deploy, and that is where minutes are affordable. **A gate that
takes minutes gets `--no-verify`d, and then it is not a gate.**

**What it rules out.** Putting the expensive check where the frequent event is. The first split put
the suite on every push, which is the same mistake one step along: a branch push is part of working —
backing up, moving between machines, opening something for review — and it cost five and a half
minutes against a tree that had already printed `pre-push: ok`. It also rules out reading the working
tree: `pre-commit` judges **staged content**, or an unfinished edit on disk blocks a good commit and a
bad commit fixed after staging goes through.

**A gate's test is something that must not pass.** This one first failed in the worst way — it passed.
`rustfmt --check` **reading stdin prints its diff and exits 0**, unlike when given a filename, and the
hook read that success and let unformatted files through. Judge by output, not by exit code, and prove
it by actually committing something that must be refused.

**Where it holds.** [.githooks/](../../.githooks); [contributing.md](../contributing.md). The split is
[ADR-0109](../adr/0109-format-the-workspace-and-split-the-gate.md), and the suite came off every push
a day later in [ADR-0114](../adr/0114-tests-run-when-somebody-asks-not-when-git-does.md). What runs in
between is [P-0057](0057-run-what-the-question-needs-when-it-is-asked.md).
