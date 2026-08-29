# The workspace stays closed to Rust

Building this repository requires `cargo` and nothing else. Anything that would drag a
second toolchain in — Objective-C interop for Syphon, cmake and a C++ compiler for Ableton
Link — runs **out of process**, in its own repository, talking versioned ndjson over a pipe.

**A helper may live outside only if it is off the deterministic path.** An output plugin
consumes the composited frame and writes no record, so a session replays identically
whether one was attached. An input plugin produces the same records a key press produces, so
a session recorded with it replays without it. A generator, a blend mode or a tone mapper
could not be a plugin on these terms, whatever toolchain it wanted: what they do reaches
pixels a replay has to reproduce. The test is the deterministic path, not the platform —
Link runs everywhere and is outside for its toolchain alone.

**What it rules out.** Vendoring a C++ dependency behind a build script, which charges the
cost to everyone who builds the repository, including on platforms where the feature does
not exist. It also rules out reaching for a script in another language for repository
chores; a `cargo test` is the cheaper answer, because it is already installed.

**Where it holds.** [plugins.md](../plugins.md);
[tempo_source.rs](../../crates/karakuri-environment/src/tempo_source.rs) is the wire format, and
the input half that exists.
