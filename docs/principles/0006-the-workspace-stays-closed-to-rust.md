# The workspace stays closed to Rust

Building this repository requires `cargo` and nothing else. Anything that would drag a
second toolchain in — Objective-C interop for Syphon, cmake and a C++ compiler for Ableton
Link — runs **out of process**, in its own repository, talking versioned ndjson over a pipe.

**A helper may live outside only if it is off the deterministic path.** That test is
[P-0042](0042-a-boundary-is-drawn-by-the-deterministic-path.md)'s and is stated there; what it
means here is that a second toolchain is a reason to put something out of process and never a
licence to. Link is outside for its cmake and its C++ compiler alone, and a generator, a blend
mode or a tone mapper could not be a plugin whatever toolchain it wanted — so wanting one is a
reason to write it in Rust, not a reason to open the workspace.

**What it rules out.** Vendoring a C++ dependency behind a build script, which charges the
cost to everyone who builds the repository, including on platforms where the feature does
not exist. It also rules out reaching for a script in another language for repository
chores; a `cargo test` is the cheaper answer, because it is already installed.

**Where it holds.** [plugins.md](../plugins.md);
[tempo_source.rs](../../crates/karakuri-environment/src/tempo_source.rs) is the wire format, and
the input half that exists.
