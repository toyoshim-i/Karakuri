---
id: 0390
title: Migrate MCP engine to agent-man and standardize structuredContent
status: accepted
date: 2026-10-12
principles: [0085, 0086, 0090, 0093]
tags: [mcp, agent-man, webmcp, structured-content, json-rpc, doc-engine]
---

# Migrate MCP engine to agent-man and standardize structuredContent

## Context

Karakuri previously ran custom, hand-rolled JSON-RPC 2.0 protocol dispatch and an HTTP/SSE server within `crates/karakuri-mcp`. Over time, this architecture accumulated several maintenance and operational issues:

1. **Duplicate Schemas and Documentation Drift:**
   MCP tool schemas and manual references were defined separately across in-tree JSON schema builders, unit tests, and a static 14-tool matrix table in `docs/manual/mcp.html`. Hand-maintained schemas frequently drifted from actual dispatch capabilities, violating [P-0093](../principles/0093-a-statement-is-held-true-by-the-thing-it-describes.md) (*a statement is held true by the thing it describes*).
2. **Boilerplate Protocol & Transport Maintenance:**
   Managing HTTP endpoints, JSON-RPC 2.0 method routing, SSE connections, and error serialization within Karakuri duplicated generic MCP infrastructure that is not domain-specific to generative shader synthesis.
3. **Ad-Hoc Tool Failure & Diagnostic Formatting:**
   Tool execution failures (such as syntax errors, validation refusals, parse errors) and diagnostic reports were embedded as untyped or ad-hoc JSON within text content, or non-standard top-level fields, preventing LLM clients from reliably parsing machine-readable diagnostic details.
4. **Cross-Target Inconsistencies (Native vs WebMCP):**
   Supporting both native HTTP MCP and in-browser WebMCP (under `wasm32-unknown-unknown`) required duplicating or stubbing transport layers, risking diverging behaviors between desktop and web harnesses.

Meanwhile, `agent-man` (published on crates.io at `v0.4.1`) provides a unified documentation and MCP engine (`DocEngine`), declarative `ToolBuilder` with runtime schema and example validation (`check_tools`), standards-compliant MCP protocol handling, built-in HTTP serving (`serve_http`), and standard `structuredContent` payload support (`refusal` and `report`).

## Decision

1. **Adopt `agent-man` as the Core MCP Engine:**
   - Replace in-tree JSON-RPC 2.0 dispatch and custom HTTP transport in `karakuri-mcp` with `agent_man::DocEngine` and `DocEngine::serve_http`.
   - Initialize `DocEngine::new("karakuri")` to preserve server identity (`serverInfo.name = "karakuri"`).
   - Declare all 14 MCP tools using `ToolBuilder` in `src/tools/schema.rs`, defining concise parameter definitions (`ParamType`), examples, and cross-references (`.see(...)`), validated automatically at test time via `engine.check_tools()`.

2. **Standardize Errors and Reports on `structuredContent`:**
   - Migrate tool refusals, validation errors, and runtime failures to MCP standard `structuredContent: { "refusal": { "kind": ..., "message": ... } }` and set `isError: true`.
   - Migrate execution diagnostics and reports to `structuredContent: { "report": { ... } }`.
   - Preserve human-readable summaries and markdown rendering in the primary text `content`.

3. **Unify In-Process and Browser Dispatch via `agent-man`:**
   - Rebuild `InProcessMcp` on `Arc<DocEngine>` to provide unified in-process tool dispatch for native and wasm32 harnesses.
   - In `karakuri-web`, integrate `agent_man::webmcp::install` and `agent_man::webmcp::call_tool` directly into the web harness.

4. **De-duplicate Documentation:**
   - Remove the redundant static 14-tool catalog matrix table in `docs/manual/mcp.html`.
   - Direct clients to dynamic discovery via MCP `tools/list` and `doc_read`, and update documentation examples to illustrate `structuredContent.refusal`.

## Consequences

- Replaced hundreds of lines of boilerplate HTTP server and JSON-RPC dispatch code with tested, shared infrastructure from `agent-man`.
- Runtime schema and example validation (`check_tools`) guarantees that tool documentation and examples stay in sync with argument validation rules.
- Upstream LLM clients and agents receive predictable, machine-readable structured error and diagnostic information via standard MCP `structuredContent`.
- Native HTTP MCP and in-browser WebMCP share the exact same tool definitions, schemas, and execution semantics.
