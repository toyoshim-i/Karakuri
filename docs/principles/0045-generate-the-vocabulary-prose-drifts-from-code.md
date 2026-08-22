# Generate the vocabulary; prose drifts from code

Where a fact exists as data in the program, publish the generated form beside the prose. The builtin
list and signatures handed to a model come from `Builtin::ALL` and `signature()` — **the checker's own
table** — so it cannot go stale: the moment it did, compilation would fail.

**What it rules out.** Maintaining a second copy of a list by hand. The specification is prose worth
reading for the shape of the language and why it is that shape, but prose drifts from code, and this
project was cut by that three times in one week.

**And a resource beats a link.** Served as an MCP resource it travels over the connection that is
already open; a link needs the client to be able to fetch and the URL to be reachable, which an SSH
tunnel makes awkward.

**Where it holds.** The MCP resources in `karakuri-cli`; `builtin.rs` in
[karakuri-ir](../../crates/karakuri-ir). Decided in
[ADR-0083](../adr/0083-mcp-is-the-only-prompt-surface.md).
