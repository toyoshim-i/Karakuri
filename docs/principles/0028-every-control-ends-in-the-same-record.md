# Every control ends in the same record

The destination is a GUI application and the CLI is scaffolding. Anything authored lives in a
**record**, and a flag, a key press or an MCP call is a way to write into that record — never a
second place the truth lives. `--bind` takes one `field=value` per JSON field of `Record::Bind`
precisely so that replacing it with a Set file is deleting a parser and calling the same method from
the decoder.

**What it rules out.** Inventing a convenient syntax that has no record behind it. `--param` cannot
address one slot, and the fix that suggests itself — `--param 1:key=value` — was **declined**,
because how a parameter is addressed per slot must match how a Set file will express it, and the
CLI's convenience does not get to decide that first. A design that puts a value only on the command
line is building on the part that gets replaced.

**Where a flag has no record yet, say so.** `--bpm` exists and the v0.2 vocabulary has no tempo
record; the gap is written down rather than papered over.

**Where it holds.** [roadmap.md](../roadmap.md), "Settled decisions"; `karakuri-cli`'s `Router` and
`Surface`. Decided in
[ADR-0046](../adr/0046-a-flag-writes-into-the-record-it-does-not-invent-one.md).
