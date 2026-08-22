# A gate that takes minutes stops being a gate

`pre-commit` runs `rustfmt --check` on the staged Rust: one second, so it can sit on every commit.
`pre-push` runs fmt, clippy `-D warnings` and the whole suite: minutes, so it sits where a minute is
affordable. **A gate that takes minutes gets `--no-verify`d, and then it is not a gate.**

**What it rules out.** Putting the expensive check where the frequent event is. It also rules out
reading the working tree: `pre-commit` judges **staged content**, or an unfinished edit on disk blocks
a good commit and a bad commit fixed after staging goes through.

**A gate's test is something that must not pass.** This one first failed in the worst way — it passed.
`rustfmt --check` **reading stdin prints its diff and exits 0**, unlike when given a filename, and the
hook read that success and let unformatted files through. Judge by output, not by exit code, and prove
it by actually committing something that must be refused.

**Where it holds.** [.githooks/](../../.githooks); [contributing.md](../contributing.md). Decided in
[ADR-0109](../adr/0109-format-the-workspace-and-split-the-gate.md).
