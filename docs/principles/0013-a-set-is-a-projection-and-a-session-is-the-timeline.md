# A Set is a projection; a session is the timeline

A Set file is a state projection with no time in it. A session stream is the timeline: the Set,
plus a `tick` every frame, plus every edit at its frame position. Saving a Set is a projection
of a session — folding state last-write-wins and dropping the ticks — not a second way to author
one.

**What it rules out.** One file for both, which was the shape the specification had before
`tick` existed: writing a tick per frame into the Set *definition* turns it into a timeline
hundreds of thousands of lines long. **Keeping them apart is what stops saving a Set from saving
a performance.**

**Where it holds.** [ir-spec.md](../ir-spec.md), "Set file format" and "Session stream format";
`karakuri-store` rejects a non-state record in a Set, including through the projection path.
Decided in [ADR-0007](../adr/0007-a-set-file-is-a-projection-and-a-session-stream-is-the-timeline.md).
