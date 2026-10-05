---
id: 0382
title: IR specification decomposition and single-source MCP publishing
status: accepted
date: 2026-10-03
principles: [0085, 0086, 0090, 0093]
tags: [docs, ir, kir, mcp, context-window, kv-cache]
---

# IR specification decomposition and single-source MCP publishing

## Context

`docs/ir-spec.md` originally served as the monolithic reference for the Karakuri Intermediate Representation. Over time, it accumulated the complete `.kir` language specification, Set container definitions, session stream schemas, and extensive historical design retrospectives, growing to 4,931 lines (303 KB, ~75,000 tokens).

When exposed via Model Context Protocol (`read_resource("karakuri://ir-spec")`), local LLM agents (operating in 8k–16k token context windows) reading the specification instantly exhausted their context limits and failed.

An initial attempt to mitigate this by generating a compacted manual in Rust and injecting it into the LLM system prompt violated key architectural principles:
- It introduced duplicate, hand-maintained documentation in code, violating [P-0093](../principles/0093-a-statement-is-held-true-by-the-thing-it-describes.md) (*a statement is held true by the thing it describes*).
- Dynamic manual injection shifted the system prompt prefix dynamically, breaking KV cache reuse across conversational turns.

## Decision

1. **Decompose `docs/ir-spec.md` at the document level:**
   Split the monolithic document into focused, single-purpose specifications:
   - `docs/ir-spec.md`: Retained strictly as the normative, compact `.kir` procedural shading language specification (~200 lines, ~2,000 tokens).
   - `docs/set-format.md`: Describes the `.kbset` / Set JSON bundle layout, metadata schema, and node binding mechanics.
   - `docs/session-stream.md`: Documents the NDJSON session event stream, journal records, and replay guarantees.
   - `docs/design-notes.md`: Preserves architectural rationale, historical alternatives, and known roadmap gaps.

2. **Publish specifications as-is to MCP:**
   The MCP server serves `docs/ir-spec.md` directly without procedural runtime transformation, redaction, or string manipulation.

## Consequences

- Compact local models (such as Muse, Mistral, and Llama) can ingest the full procedural language grammar in a single read without context overflow.
- Code-level manual transformations (`manual.rs`) are eliminated, restoring documentation integrity and enabling deterministic prompt caching.
- Unimplemented architectural gaps recorded in design notes are explicitly surfaced and tracked in [docs/roadmap.md](../roadmap.md).
