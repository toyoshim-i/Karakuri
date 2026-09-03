# An instrument says what it did

Every action reports its effect. **A window that opens and cannot be touched is a demo, not a tool,
and an instrument that goes silent is unusable.**

**This rule binds the instrument.** That is `crates/karakuri`, the program a player runs, which draws
a panel and prints what an operation did. It does not bind `karakuri-cli`, which is test tooling
rather than something anybody plays
([ADR-0242](../adr/0242-the-command-line-is-test-tooling-and-the-instruments-principles-do-not-bind-it.md)).
The command line is where the rule was learned, so the observations below are taken there.

**What it rules out.** Shipping a control surface whose feedback is the picture alone. It also rules
out believing a control works because the code says so: the slot digits were first bound `1`–`4`, so
pressing `1` printed "focus slot 0" — inconsistent with the status line and every swap message, and
found by pressing the key rather than by reading.

**Driving it is how properties become visible.** Taking a slot off air prints
`slot 1 off air — allocated, holding t 2.97s`, the status line then holds while its neighbour runs
on, and returning prints `resuming at t 2.97s` — which is how "Allocated resumes rather than
restarts" stopped being an implementation detail and became something anyone can see. Cycling the
tone mapper on moving material is likewise information no side-by-side still can give.

**Where it holds.** [manual.md](../manual.md); the console's own lines in
`crates/karakuri/src/main.rs`, where a latency offset pressed to the end of its travel says it was
held there rather than answering the same number twice. A panel that cannot meet a staleness it
declared says it is over budget rather than looking calm
([ADR-0189](../adr/0189-motion-may-carry-the-meaning-and-a-stopped-animation-is-a-fault-to-report.md)),
and the manual's `has` badge is this rule used as a definition — a row is claimed the day a person
who launched the instrument can perform that operation from the panel in front of them
([ADR-0213](../adr/0213-the-interface-milestones-meter-is-the-panel-column-and-has-means-an-operator-reaches-it.md)).
Decided in [ADR-0045](../adr/0045-the-cli-is-an-instrument-not-a-demo.md), which called
`karakuri-cli` the instrument; that word is retired by ADR-0242 and its observations are not.
