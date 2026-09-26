# Plugins & Extensions

Two things Karakuri interfaces with — **output routing** (Syphon on macOS, Spout on Windows, NDI on a
network) and **Ableton Link** — would each drag a non-Rust toolchain into a workspace that
is otherwise cleanly closed. Syphon needs Objective-C interop; Link needs cmake and a C++
compiler; Spout requires Windows DirectX / DXGI shared memory interop. Neither cost is paid once: it
is paid by everyone who builds the repository, including on platforms where the feature does not exist.

This document specifies how out-of-process plugins are architecturalized, discovered, communicated
with, and managed for development.

## Design Principles

### Why out of process

Loading foreign code into the render process means a bad plugin can take the show down mid
performance, and the boundary cannot catch it — a panic or a C++ exception crossing FFI is
undefined behaviour. Running plugins **out of process** over standard pipes and OS-native zero-copy
GPU handles isolates failure: if a plugin crashes, it takes down only its own process, while
Karakuri's engine continues rendering uninterrupted ([P-0094](principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)).

### Determinism boundary

Plugins sit strictly outside the deterministic replay core ([P-0092](principles/0092-the-same-inputs-produce-the-same-frame.md)):

- An **output** plugin is downstream of everything. It consumes completed frame surfaces and
  records no journal events. A session replays identically whether an output plugin was attached or not.
- An **input** plugin produces **records** — identical to physical keyboard, pointer, or MIDI
  events. Link's contribution is a tempo grid anchor, and `tempo` passes through journal records every
  frame, so a session recorded with Link replays without it.

A generator, a blend mode, or a tone map operator cannot be plugins on these terms: what they do
reaches the pixels a replay has to reproduce. The line is *not* "platform-dependent things go
outside" — Link runs everywhere and is out here for its toolchain, not its OS.

---

## Output Plugins

Output plugins receive composited video frames using zero-copy GPU texture sharing.

### GPU Surface Protocols

The host engine (`crates/karakuri/src/bridge/plugin_sink.rs`) negotiates native hardware surface
handles over an IPC pipe:

- **macOS (`iosurface`)**: Passes 64-bit `IOSurfaceID` handles backed by Metal textures.
- **Windows (`dxgi`)**: Passes NT shared handles (`HANDLE`) backed by Direct3D 11 / Direct3D 12
  textures, interoperating with Vulkan (`VK_KHR_external_memory_win32`) and DX12 backends.

The host never reads back CPU pixels for zero-copy sinks. The window is the default sink and plugins
are additional non-blocking sinks: a slow or wedged plugin has its frame dropped without delaying
presentation on the main window.

### Dynamic Discovery & Wire Handshake

The engine and console do not hardcode specific plugin binary names or relative paths. Instead,
plugins are **dynamically discovered** at startup via [`karakuri_environment::output_plugin::discovery`](crates/karakuri-environment/src/output_plugin/discovery.rs):

1. The plugins directory is resolved deterministically (see below).
2. The engine scans the directory for executable binaries and launches each candidate with piped standard I/O.
3. The child process immediately greets on stdout with an ndjson `Hello` greeting:
   ```json
   {"t":"hello","v":1,"kind":"output","name":"syphon","surfaces":["iosurface"]}
   ```
4. The host inspects the greeting:
   - Protocol version (`v`) must match the host's `PROTOCOL_VERSION` (currently `1`).
   - `kind` must be `"output"`.
   - The plugin's reported `surfaces` list must contain the host platform's required surface mechanism (`iosurface` on macOS, `dxgi` on Windows).
5. Discovered plugins are registered on the console GUI. The UI dynamically presents the plugin's self-reported name (capitalized, e.g. `Syphon`, `Spout`) in the Outputs bay row.
6. The host politely closes stdin (`{"t":"close"}`) and ensures candidate probing terminates within 1.5 seconds. Non-output binaries (such as tempo sources) or unresponsive processes are discarded cleanly without error.

### Plugin Directory Resolution (`places::plugins`)

Karakuri follows the hierarchical search tier established by [`karakuri_environment::places::plugins`](crates/karakuri-environment/src/places.rs):

1. **Given (`Found::Given`)**: Explicit CLI parameter `--plugins <DIR>`
2. **Environment (`Found::Given`)**: `KARAKURI_PLUGINS_DIR` environment variable
3. **App Bundle (`Found::Bundle`)**: `<exe_dir>/../PlugIns` (macOS `.app` bundle)
4. **Unix Prefix (`Found::Prefix`)**: `<exe_dir>/../lib/karakuri/plugins` or `<exe_dir>/../share/karakuri/plugins`
5. **Beside (`Found::Beside`)**: `<exe_dir>/plugins` (portable distribution)
6. **Workspace (`Found::Workspace`)**: `<workspace_root>/plugins` (development repository root; ignored in `.gitignore`)

Startup diagnostics printed to stderr or the console HUD identify which candidate directory was discovered and how many plugins were registered.

---

## Input Plugins (Tempo Sources)

Input plugins report timing and synchronization signals to the engine.

### Protocol (`tempo_source`)

Specified in `crates/karakuri-environment/src/tempo_source.rs`.
- Started via CLI option: `--tempo-source <COMMAND>` (e.g., `--tempo-source ./plugins/karakuri-link`)
- Communicates versioned ndjson over standard I/O pipes.
- Emits **anchors** (`{"t":"grid","bpm":128.0,"beat":1024.25,"clock_us":...}`) rather than instantaneous samples.
- The host filters network jitter and pipe delivery latency through a sliding-window running minimum clock offset estimator.
- Bound-limited phase alignment protects the render loop against jarring phase jumps: the first anchor aligns, subsequent anchors trim by at most 1/20th of a beat, and three consecutive disagreements re-align with diagnostics.

---

## Current Plugin Implementations

| Name | Role | Protocol / Surface | Repository | Supported Platforms | License |
|---|---|---|---|---|---|
| `link` | Tempo Source (Ableton Link) | Input (`tempo_source`) | `toyoshim-i/Karakuri-link` | macOS, Linux, Windows | GPL-2.0-or-later |
| `syphon` | Frame Server (Syphon) | Output (`iosurface`) | `toyoshim-i/Karakuri-syphon` | macOS | MIT |
| `spout` | Frame Server (Spout2) | Output (`dxgi`) | `toyoshim-i/Karakuri-spout` | Windows | MIT |

---

## Developer Workflow & Setup Script

Plugins live in standalone sibling repositories alongside Karakuri:

```text
parent_directory/
  ├── Karakuri/           # Main workspace
  │   └── plugins/        # Resolved by Found::Workspace (gitignored)
  ├── Karakuri-link/      # Input plugin
  ├── Karakuri-syphon/    # macOS output plugin
  └── Karakuri-spout/     # Windows output plugin
```

### Management Script: `scripts/plugins.sh`

A developer utility script is provided at `scripts/plugins.sh` to clone, build, install, and check plugin statuses:

```bash
# Check current checkout, build, and installation status
./scripts/plugins.sh status

# Clone, build, and install all plugins compatible with your current OS
./scripts/plugins.sh setup all

# Or set up a specific plugin by name (shortcut syntax)
./scripts/plugins.sh syphon
./scripts/plugins.sh link
./scripts/plugins.sh spout

# Individual operations
./scripts/plugins.sh clone syphon
./scripts/plugins.sh build syphon --release
./scripts/plugins.sh install syphon

# Clean up plugins from workspace plugins/ directory
./scripts/plugins.sh clean syphon
```

The script symlinks (or copies on Windows) compiled binaries directly into `<workspace_root>/plugins/`. When you launch Karakuri:

```bash
# Karakuri console automatically discovers plugins in plugins/
cargo run -p karakuri

# Run with tempo source
cargo run -p karakuri-cli -- --tempo-source ./plugins/karakuri-link
```
