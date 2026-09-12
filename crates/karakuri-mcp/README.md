# `karakuri-mcp`

Model Context Protocol (MCP) server exposing Karakuri inspection and control capabilities to AI coding assistants.

---

## 1. Role in Architecture

`karakuri-mcp` sits at **Layer 4 (Environment & Services)** in the Karakuri workspace:

- **AI Pair-Programming Interface**: Implements the Model Context Protocol (JSON-RPC) allowing LLMs (such as Claude, Gemini) to inspect and edit shaders, manage sets, wire inputs, and trigger performance operations.
- **Upstream Dependencies**: `karakuri-environment`, `karakuri-operation`, `karakuri-ir`, and `karakuri-store`.
- **Gated Execution**: All state-mutating operations dispatched via `operate` pass through [`karakuri_operation::gate::audit`], respecting operator permissions set at bay heads in the UI console.

---

## 2. Tools & Capabilities

1. **Shader Procedure Tools**:
   - `check_procedure`: Statically validates `.kir` syntax, types, and execution costs without compiling to GPU.
   - `read_procedure`: Fetches active `.kir` source from a running slot.
   - `write_procedure`: Dispatches an updated procedure to background compilation workers.
2. **Set Configuration Tools**:
   - `read_set`: Reads Set definitions, node bindings, and parameter states.
   - `list_sets`: Catalogs available Sets in the store with optional filtering.
   - `save_set`: Commits active deck parameters to a new or existing Set file.
3. **Graph & History Tools**:
   - `wire_input`: Connects outputs and modulation sources to procedure inputs.
   - `walk_history`: Inspects version history for nodes and Sets.
   - `swap_outcome`: Polls the status of asynchronous compilation swaps.
4. **Vocabulary Operation Execution**:
   - `operate`: Dispatches any operation from `karakuri-operation`, guarded by operator console permissions.

---

## 3. Testing & Verification

Run tests:

```sh
cargo test -p karakuri-mcp --lib
```
