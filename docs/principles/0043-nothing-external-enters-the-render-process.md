# Nothing external enters the render process

No network, no secrets, no third-party binary, no second toolchain. A model is reached by being an
**MCP server**, not by holding API keys and a prompt builder; a GPL library runs as a **separate
program** behind a generic interface; an output plugin is loaded, if at all, across an ABI restricted
to **what can cross a process boundary** — a shareable handle and a frame number, never a
`wgpu::Texture` pointer or a callback back into the host.

**What it rules out.** A prompt box in the GUI, which would make model selection, streaming, retries
and rate limits Karakuri's problem and turn every model change into a Karakuri change. It rules out
`dlopen`ing third-party code into the render process during a show, where a panic or a C++ exception
crossing FFI is undefined behaviour that cannot be caught. And it rules out a build script that
fetches: the moment `cargo build` touches the network, offline and sandboxed builds break and the cost
is paid by people who did not want the plugin.

**Rot moves rather than disappearing.** A feature flag rots loudly at compile time; a cdylib against a
C ABI rots silently at runtime, and a layout mismatch is a segfault mid-performance. So the first call
is a version handshake and a mismatch refuses to load, naming both versions.

**Where it holds.** [plugins.md](../plugins.md); the MCP server in `karakuri-cli`. Decided in
[ADR-0075](../adr/0075-the-plugin-abi-passes-a-handle-and-only-what-crosses-a-process.md) and
[ADR-0083](../adr/0083-mcp-is-the-only-prompt-surface.md).
