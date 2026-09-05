# The operator's library is written by an operator's own act

| | where | who writes it |
|---|---|---|
| app presets | `examples/` | nobody — they ship with the program |
| the operator's library | `<store>/sets/<id>.kbset` | **an operator's own act**: `--save-set`, the `k` key, a control they pressed |
| a model's saves | `<store>/sandbox/<id>.kbset` | a save asked for over MCP, and nothing else |
| scratch | `<store>/scratch/` | `--watch`, MCP, and the operator's editor |

Material from anywhere is materialised into scratch at launch, and the location is printed so an
editor knows where to point.

**The rule is about the actor, not about the flag.** A row that names a spelling goes stale the
first time a second control reaches the same code, and this one did: it read *`--save-set`, and
nothing else* while the `k` key, the panel's own save path and MCP's `save_set` tool were all
writing there. What decides the second row is who asked, and every route an operator's hands
reach — a flag, a key, a pill, a control change — is the same act arriving by a different door.

**What it rules out.** A model — or a watcher — writing to the files that ship. It happened: a
session rewrote three tracked examples one day after the manual gained the sentence saying a write
replaces your file with no backup, and it was recoverable **only because of git, which is not a
property of the feature**. It rules out the same write one directory over: a model naming an id an
operator's preset already has, and the preset being gone.

**A model is not refused, and it does not write the operator's library.** A save asked for over MCP
lands in `<store>/sandbox/`, and what lands there is **an explicit edit-history snapshot** — the
thing an operator goes looking for after a show when a model has been editing live. That is why the
answer is a second directory rather than a closed door: refusing the operation would leave a session
of live edits with nothing kept anywhere, which is the loss this rule exists to prevent, taken from
the other side.

**Nothing in the sandbox is overwritten**, where a name an operator typed into their own library
overwrites on purpose. An id an operator types is an instruction; a snapshot that a later snapshot
can replace is not a snapshot, and a model saving under one name all evening would leave the
operator one file where the whole point was the row of them.

**Only a run with something writable copies.** `--render`, `--seq` and `--replay` do not, because an
offscreen render should be a function of its arguments and must not leave a directory behind.

**Every version that compiles is kept**, gated on *compiling* rather than *reaching the screen* —
a version rolled back for budget never enters the session stream and is often the one you want back.
Seed the launch version, or the first edit has nowhere to return to.

**Where it holds.** `scratch.rs`, `history.rs` and `places.rs` in
[karakuri-environment](../../crates/karakuri-environment); `store.rs` in
[karakuri-store](../../crates/karakuri-store), which owns the four addresses;
[manual.md](../manual.md) and [manual/operations.html](../manual/operations.html). Decided in
[ADR-0088](../adr/0088-what-ships-what-you-saved-and-what-you-are-editing.md),
[ADR-0089](../adr/0089-history-is-gated-on-compiling-not-on-landing.md) and
[ADR-0261](../adr/0261-a-model-asked-save-lands-in-a-sandbox-because-the-operators-library-is-the-operators-own-act.md).
