# Plugins

Two things Karakuri wants — **output routing** (Syphon on macOS, Spout on Windows, NDI on a
network) and **Ableton Link** — would each drag a non-Rust toolchain into a workspace that
is otherwise cleanly closed. Syphon needs Objective-C interop; Link needs cmake and a C++
compiler. Neither cost is paid once: it is paid by everyone who builds the repository,
including on platforms where the feature does not exist.

This document draws the line they hang off. Nothing here is built yet.

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

## The input side already exists

`karakuri-midi` plus the CLI's `Router` and `Surface` are the shape, working, in tree:
every MIDI action ends in the method a key press ends in, so a controller can do nothing a
key cannot and a session recorded from one replays with neither controller nor map
attached. **A Link plugin is another `Surface`.** The input half of the interface is
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

## Distribution

Plugins live in their own repositories, imported as submodules under `third_party/`
alongside the SDK. **The built artifacts are not committed here.** What is committed is a
manifest — per plugin: name, interface version, the plugin repository's tag, and per target
triple an asset name and a sha256. `cargo xtask plugins` fetches the current triple's assets,
verifies them, and drops them in a gitignored directory that is also the default plugin
search path, so the development flow and the installed flow use the same lookup.

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
| An entry, but nothing fetched | run `cargo xtask plugins` |
| Fetched, interface version mismatch | refuse to load, and name both versions |

Without it, all three are "the file is not there", and a Windows user cannot tell whether
Syphon is impossible or merely un-fetched.

## Rot moves; it does not disappear

A Cargo feature that nobody exercises rots **loudly, at compile time**. A shared library
against a C interface rots **quietly, at run time** — a mismatched struct layout is a
segfault, mid-show. So the first call across the boundary is a version handshake, and a
mismatch is refused with a message rather than loaded. It is cheap, and it is the whole
difference.
