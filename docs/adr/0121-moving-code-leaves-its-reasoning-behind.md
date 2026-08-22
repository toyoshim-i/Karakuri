---
id: 0121
title: Moving code leaves its reasoning behind
status: accepted
date: 2026-08-20
supersedes: []
superseded_by: []
principles: [0023]
tags: [docs, process]
---

# Moving code leaves its reasoning behind

## Context

Moving the placement rules from `karakuri-codegen` into `karakuri-ir` was a pure relocation. The
implementing agent returned three errors in my brief, and the third is the one worth keeping.

## Decision

**A comment was falsified by the move, in a place I had not predicted.** `derivation_slot`'s
justification read *the rule lives in a crate that emits no WGSL* — true where it was written, and
**false the moment the rule and the name it justifies arrived in the same crate.** Rewritten to the
argument that survives: naming and offset are one placement decision, and they are either taken
together or not at all.

The same shape as *deleting a check leaves its comments behind*: **move code and its reasoning is left
where it was**, still reading as though it applied.

## Consequences

- The other two brief errors were mine to own: a function name I had not checked (`generate_element_struct`
  does not exist), and stride arithmetic I had got wrong. **Seven briefs out of seven have contained an
  error somewhere**, which is why the checks in them are given to the agent to re-derive rather than
  asserted.
- The module doc's opening had justified the module's new address by *the estimator has to charge what
  the engine allocates* — which stopped being true the same week, when stage 4 gave the figure up
  ([ADR-0116](0116-stage-four-stops-claiming-the-byte-figure.md)). Rewritten to the argument that
  survives: the struct's contents are `emit`, `Attr` widths and which derivations are stored — all facts
  of that crate.

## Evidence

Session 2026-08-20T16:52Z.
