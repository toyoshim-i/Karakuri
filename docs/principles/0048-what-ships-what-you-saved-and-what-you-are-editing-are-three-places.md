# What ships, what you saved, and what you are editing are three places

`examples/` is app presets and **nobody writes it**. `<store>/sets/` is user presets and only
`--save-set` writes it. `<store>/scratch/` is where the deck actually runs, and `--watch`, MCP and
your editor all write there. Material from anywhere is materialised into scratch at launch, and the
location is printed so an editor knows where to point.

**What it rules out.** A model — or a watcher — writing to the files that ship. It happened: a
session rewrote three tracked examples one day after the manual gained the sentence saying a write
replaces your file with no backup, and it was recoverable **only because of git, which is not a
property of the feature**.

**Only a run with something writable copies.** `--render`, `--seq` and `--replay` do not, because an
offscreen render should be a function of its arguments and must not leave a directory behind.

**Every version that compiles is kept**, gated on *compiling* rather than *reaching the screen* —
a version rolled back for budget never enters the session stream and is often the one you want back.
Seed the launch version, or the first edit has nowhere to return to.

**Where it holds.** `scratch.rs` and `history.rs` in [karakuri-cli](../../crates/karakuri-cli);
[manual.md](../manual.md). Decided in
[ADR-0088](../adr/0088-what-ships-what-you-saved-and-what-you-are-editing.md) and
[ADR-0089](../adr/0089-history-is-gated-on-compiling-not-on-landing.md).
