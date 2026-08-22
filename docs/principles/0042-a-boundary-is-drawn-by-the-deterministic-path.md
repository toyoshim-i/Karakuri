# A boundary is drawn by the deterministic path

What may live outside the process is decided by one test: **is it on the path a replay has to
reproduce?** An output plugin consumes the composited frame and writes no record, so a session
replays identically whether one was attached. An input plugin produces the same records a key press
produces, so a session recorded with it replays without it.

**What it rules out.** "Platform-dependent things go outside." Ableton Link runs everywhere and is
outside for its toolchain alone, and a generator, a blend mode or a tone map operator could not be a
plugin on these terms whatever toolchain it wanted, because what they do reaches pixels a replay must
reproduce.

**The distinction survives into what the host says.** Syphon missing on Windows means the feature does
not exist there; Link missing means nobody built that plugin. Those are different sentences and the
operator needs to be told which one they are in.

**And two reasons for one constraint is worth having.** The out-of-process boundary is required for
stability, and separately it is what keeps a GPL work a separate work. Losing either argument does not
lose the boundary.

**Where it holds.** [plugins.md](../plugins.md). Decided in
[ADR-0074](../adr/0074-a-plugin-boundary-is-drawn-by-the-deterministic-path.md) and
[ADR-0080](../adr/0080-the-gpl-boundary-is-a-process-and-the-protocol-is-generic.md).
