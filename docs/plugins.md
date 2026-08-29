# Plugins

Two things Karakuri wants — **output routing** (Syphon on macOS, Spout on Windows, NDI on a
network) and **Ableton Link** — would each drag a non-Rust toolchain into a workspace that
is otherwise cleanly closed. Syphon needs Objective-C interop; Link needs cmake and a C++
compiler. Neither cost is paid once: it is paid by everyone who builds the repository,
including on platforms where the feature does not exist.

This document draws the line they hang off. **The input half is built** — `--tempo-source`
runs and Ableton Link is what it was tested against — and **the sink half is built** on the
host's side of the boundary. What does not exist is any output plugin, and the distribution
machinery below is a design rather than a description: nothing fetches anything yet.

## Why these two and not other things

**Because neither is on the deterministic path**, and that is the whole test.

- An **output** plugin is downstream of everything. It consumes the composited frame and
  writes no record, so a session replays identically whether one was attached or not.
- An **input** plugin produces **records** — the same records a key press writes. Link's
  contribution is a tempo, and `tempo` already goes through a record every frame, so a
  session recorded with Link replays without it.

A generator, a blend mode, or a tone map operator could not be plugins on these terms: what
they do reaches the pixels a replay has to reproduce. The line is *not* "platform-dependent
things go outside" — Link runs everywhere and is out here for its toolchain, not its OS.
That distinction survives into what the host says when something is missing; see the table
below.

## The input side is built

`--tempo-source COMMAND` runs a program and follows the beat it reports. The wire format is
specified in `crates/karakuri-environment/src/tempo_source.rs`: versioned ndjson over a pipe, an
**anchor** rather than a sample — a beat, a tempo, and the source's own clock reading at
which both were true — so the transport's delay never becomes phase error. Karakuri
estimates the offset between that clock and its own as the minimum over a sliding window,
which filters out delivery delay and forgets a bad reading.

**How far a source may move the grid is bounded**, and that bound is the interface's, not
the source's. The first anchor aligns; every one after it trims by at most a twentieth of a
beat; three consecutive anchors disagreeing by more than a beat re-align and say so. An
in-process beat tracker has had gates and evidence counters since it was written, and the
out-of-process program is the one that most needs them.

## The other input side already exists

`karakuri-midi` plus the CLI's `Router` and `Surface` are the shape, working, in tree:
every mapped MIDI message is an `Operation` and ends in the record a key press ends in, so a
controller can do nothing a key cannot and a session recorded from one replays with neither
controller nor map attached. **A Link plugin is another `Surface`.** The input half of the interface is
therefore extracted from something that runs rather than invented, which is the only reason
to specify it before a second instance exists.

## The output side is where the design content is

Syphon and Spout are both **zero-copy GPU texture sharing** — IOSurface on macOS, a DXGI
shared handle on Windows. NDI encodes, so it wants CPU pixels.

An interface that passes **pixels** makes NDI happy and makes Syphon and Spout pointless: it
would add a full-frame readback per frame purely to hand the result back to the GPU. So what
crosses is a **native handle**, and the plugin and the host negotiate at open time — the
plugin declares what kinds of surface it accepts, the host declares what it can produce, and
an empty intersection means the plugin does not load.

The consequence for this repository is small and worth stating exactly: the host needs
`wgpu-hal`'s `as_hal` to get at the underlying Metal texture or D3D12 resource, which is
`unsafe`. **It does not need Objective-C, and it does not need cmake.**

### One constraint, free today, that keeps a door open

Loading foreign code into the render process means a bad plugin can take the show down mid
performance, and the boundary cannot catch it — a panic or a C++ exception crossing FFI is
undefined behaviour. Running a plugin **out of process** is nearly free for exactly these
plugins, because IOSurface and DXGI handles are already shareable across processes; that is
how Syphon works in the first place.

That is not worth building now. It is worth not foreclosing: **the interface may only pass
things that survive a process boundary.** A shareable handle and a frame index do. A
`wgpu::Texture` pointer or a callback into the host do not.

### The window never waits

The window is the default sink and plugins are additional ones, so the composited frame goes
to *n* sinks rather than being handed to one. A plugin that is slow or wedged gets its frame
dropped; presenting is never delayed for it. The per-frame call is non-blocking by
specification, not by convention.

**The in-repo half of this is built.** `karakuri-engine`'s `frame` module has a `Sink` trait —
acquire a target, draw into it, present — with the window behind it, `karakuri-cli`'s PNG
writer behind it, and one frame loop over both. That was worth doing on its own account, because the two loops it
replaced had drifted apart and every replay defect this project has found came from the
difference. What it means here is that a plugin is a third sink rather than a change to how
a frame works, and that the interface a plugin needs already has two implementations to be
extracted from rather than one to be guessed at.

## What a surface offers, not only what it accepts

MCP taught this the moment it was first used: a model with no worked example spent four
failed compiles learning what the language allows. **A control surface for a model is half
tools and half things to read**, and the reading half is the cheaper of the two to get
wrong — a keyboard needs no curriculum and a model does.

The rule that falls out is worth stating before anyone builds the library version: **the
resource list is a curriculum, not an index.** Four procedures chosen to span what the
language can do beat two thousand, and searching a large library is a tool call rather than
a list a client reads in full. See M4 in `docs/roadmap.md`.

## Distribution

Plugins live in their own repositories, and **this one prescribes nothing about where a
checkout of them goes** — not a submodule, not a directory, not a gitignore entry. Whoever
develops one puts it wherever they keep repositories. A layout invented here would be this
project's answer to somebody else's question, and it would outlive the reason for it.

Keeping the source out entirely is also the simplest thing to explain, and the Link helper
gives that a second reason the first draft of this document did not know: it is
**GPL-2.0-or-later**, because Ableton Link is, where this workspace is MIT. The combination
is permitted and would mean a binary linking it is distributed under the GPL, so the
boundary that matters is between two *programs*. Nothing about a directory changes that, and
nothing about a directory has to.

**The built artifacts will not be committed here either.** What will be committed is a
manifest — per plugin: name, interface version, the plugin repository's tag, and per target
triple an asset name and a sha256. `cargo xtask plugins` will fetch the current triple's
assets, verify them, and drop them in a gitignored directory that is also the default plugin
search path, so the development flow and the installed flow use the same lookup. **None of
this exists yet**: there is no manifest file and no `xtask` crate in the workspace. It is
written down now because the shape decides what the host can say when a plugin is missing,
which is the table below.

**`cargo build` must never touch the network.** A fetch inside `build.rs` would break
offline and sandboxed builds, run for people who do not want plugins, and destroy the
property being bought. Because plugins are optional, a failed fetch is "the feature is
absent" rather than "the build is broken" — an escape an ordinary vendored dependency does
not have.

The manifest earns its place twice. Besides provenance, it is the only thing that can tell
these three states apart:

| State | What the host says |
|---|---|
| No entry for this target triple | this platform does not have that feature |
| An entry, but nothing fetched | run `cargo xtask plugins` (once that exists) |
| Fetched, interface version mismatch | refuse to load, and name both versions |

Without it, all three are "the file is not there", and a Windows user cannot tell whether
Syphon is impossible or merely un-fetched.

## Rot moves; it does not disappear

A Cargo feature that nobody exercises rots **loudly, at compile time**. A shared library
against a C interface rots **quietly, at run time** — a mismatched struct layout is a
segfault, mid-show. So the first call across the boundary is a version handshake, and a
mismatch is refused with a message rather than loaded. It is cheap, and it is the whole
difference.
