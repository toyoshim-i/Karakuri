# An instrument says what it did

There is no on-screen UI, so every action prints its effect. **A window that opens and cannot be
touched is a demo, not a tool, and an instrument that goes silent is unusable.**

**What it rules out.** Shipping a control surface whose feedback is the picture alone. It also rules
out believing a control works because the code says so: the slot digits were first bound `1`–`4`, so
pressing `1` printed "focus slot 0" — inconsistent with the status line and every swap message, and
found by pressing the key rather than by reading.

**Driving it is how properties become visible.** Taking a slot off air prints
`slot 1 off air — allocated, holding t 2.97s`, the status line then holds while its neighbour runs
on, and returning prints `resuming at t 2.97s` — which is how "Allocated resumes rather than
restarts" stopped being an implementation detail and became something anyone can see. Cycling the
tone mapper on moving material is likewise information no side-by-side still can give.

**Where it holds.** [manual.md](../manual.md); `karakuri-cli`. Decided in
[ADR-0045](../adr/0045-the-cli-is-an-instrument-not-a-demo.md).
