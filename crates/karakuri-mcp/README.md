
# `karakuri-mcp`

Model Context Protocol (MCP) server exposing Karakuri inspection, documentation, and control capabilities to AI coding assistants. Built on the `agent-man` documentation and tool engine.

---

## 1. Role in Architecture

`karakuri-mcp` sits at **Layer 4 (Environment & Services)** in the Karakuri workspace:

- **AI Pair-Programming Interface**: Implements the Model Context Protocol (JSON-RPC) powered by `agent-man`, allowing LLMs to inspect documentation, edit shaders, manage sets, wire inputs, and trigger performance operations.
- **Unified Engine**: Leverages `agent_man::DocEngine` to serve both documentation (`doc_list`, `doc_read`, and HTTP topics) and 14 registered MCP tools with schema validation, `.see(...)` documentation links, and runnable examples.
- **Dual Transport**: Supports standard HTTP loopback via `DocEngine::serve_http` on native platforms, and lock-free in-process dispatching (`InProcessMcp` / WebMCP) on WebAssembly and browser runtimes.
- **Gated Execution**: All state-mutating operations dispatched via `operate` pass through [`karakuri_operation::gate::audit`], respecting operator permissions set at bay heads in the UI console. Refusals and diagnostics are delivered as standard `structuredContent`.

---

## 2. Tools & Capabilities

The server registers 14 MCP tools with concise descriptions and documentation references:

1. **Shader Procedure Tools**:
   - `read_procedure`: Fetches active `.kir` source from a running slot and layer.
   - `write_procedure`: Dispatches an updated procedure to background compilation workers with 30-frame GPU performance measurement.
   - `check_procedure`: Statically validates `.kir` syntax, types, and execution costs without compiling to GPU or altering memory. Returns diagnostic reports in `structuredContent.report`.
2. **Set Configuration Tools**:
   - `read_set`: Reads Set definitions, node bindings, and parameter states.
   - `list_sets`: Catalogs available Sets in the store with optional filtering.
   - `save_set`: Commits active deck parameters to a new or existing Set file.
   - `check_set`: Validates Set configuration and asset integrity before activation.
3. **Graph & History Tools**:
   - `wire_input`: Connects outputs and modulation sources to procedure inputs.
   - `walk_history`: Inspects version history for nodes and Sets.
   - `swap_outcome`: Polls the status of asynchronous compilation swaps.
4. **Agent Workflow & Co-Performance Tools (M7)**:
   - `get_permissions`: Queries console bay class openings, per-slot MCP policies (`Auto`, `On`, `Off`), live mixer contribution status (`in_mix`), and write permissions.
   - `read_slot`: Reads procedural definitions across all active layers for a slot in a single round trip.
   - `copy_slot`: Atomically copies procedural setup between slots with atomic rename staging (`.tmp` files) and write-guard verification.
5. **Vocabulary Operation Execution**:
   - `operate`: Dispatches any operation from `karakuri-operation`, guarded by operator console permissions.

---

## 3. Safety Policies & Structured Content

- **Structured Refusals & Reports**:
  When a gate refuses an operation or syntax checks fail, results are returned with `isError: true` and standardized `structuredContent` payload:
  - Refusals: `structuredContent: { "refusal": { "code": "...", "message": "...", ... } }`
  - Diagnostics: `structuredContent: { "report": { "success": false, "diagnostics": [...] } }`
- **Slot Policies**:
  - `Auto`: Writable while parked/rehearsing; automatically write-locked when active in the live mix.
  - `On`: Operator explicitly permits AI modifications even while on air.
  - `Off`: Operator locks slot against any AI modification.
- **Atomic Staging**: Edits and copies write to `.kir.tmp` scratch files before performing an atomic rename, preventing file-watcher torn reads by compiler workers.

---

## 4. Testing & Verification

Run tests:

```sh
cargo test -p karakuri-mcp --lib
cargo test -p karakuri-mcp --test wire
```
