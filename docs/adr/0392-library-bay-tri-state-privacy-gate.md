---
id: 0392
title: Library bay tri-state privacy gate
status: accepted
date: 2026-10-13
supersedes: []
superseded_by: []
principles: [0083, 0094, 0096]
tags: [mcp, console, library, privacy, gate]
---

# Library bay tri-state privacy gate

## Context

With the expansion of MCP capabilities allowing agents to explore the full library (bundled reference presets and user-saved Sets and procedures), agents can inspect and read shader code across the entire installation.

However, the contents of the operator's private store (`.karakuri/`) often contain unreleased performance material, proprietary visual techniques, or confidential experiment files. Allowing autonomous external agents unrestricted read access to this private store by default presents privacy and data leakage concerns.

Prior to this decision, MCP tools such as `list_sets` and `read_set` permitted ungated access to stored Sets without requiring explicit operator consent. A mechanism was needed to balance an agent's ability to learn from standard public examples with the operator's expectation of privacy for their private store.

## Decision

1. **Introduce Tri-State Library MCP Gate (`LibraryGate`):**
   - The Library bay carries an explicit three-state privacy gate in `karakuri-operation`:
     - `preset` (Default): Permits access exclusively to common, bundled reference examples (`examples/`). User private store items (`.karakuri/`) are withheld from library queries, and explicit requests for `scope: "store"` are refused with actionable guidance.
     - `all`: The operator explicitly consents to opening the full library. External agents may discover, search, and read both bundled presets and user private store Sets and procedures.
     - `off`: The Library bay is shut. All library exploration, listing, and procedure/set read operations via MCP are refused.

2. **Default to `preset`:**
   - By default, the Library gate initializes to `preset`.
   - Unlike previous versions where the store was readable without friction, `preset` intentionally closes access to private user data by default.
   - Autonomous agents always have access to public learning materials and reference procedures, but cannot access user private data unless the operator deliberately grants access.

3. **Interactive Console Control:**
   - The Library bay header in `karakuri-console` renders an interactive capsule pill (`mcp · preset`).
   - Clicking the pill cycles through the states:
     ```
     Preset -> All -> Off -> Preset
     ```

## Rejected Alternatives

- **Defaulting to `all`:**
  Rejected because user privacy must be preserved by default. An operator should never find that an external agent exfiltrated or inspected private performance sets without explicit affirmative consent.
- **No Gate (Leaving Store Always Accessible):**
  Rejected because expanding MCP library inspection across all procedures and Sets without a permission boundary violates the operator's custody over their private library ([Principle 0096](../principles/0096-the-operators-library-is-written-by-an-operators-own-act.md)).
