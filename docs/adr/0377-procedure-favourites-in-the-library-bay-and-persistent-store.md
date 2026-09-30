---
id: 0377
title: Procedure favourites in the Library bay and persistent store
status: accepted
date: 2026-09-29
supersedes: []
superseded_by: []
principles: [0090, 0092]
tags: [store, library, console, favourites, procedures, m10]
---

# Procedure favourites in the Library bay and persistent store

## Context

Prior to this decision, the Library bay and the underlying persistent `Store` supported starring only complete Sets (`.kset` files). A starred set was recorded in `<store>/favourites/sets.list` and recalled under the `Scope::MySets` filter tab.

As Karakuri's procedure corpus expanded in Milestone 10—incorporating procedural meshes (L1), specialized 3D materials (L4), and master/deck post-processing effects (L5)—performers increasingly needed quick access to individual building blocks. When building sets or manipulating master chains on the fly during a live performance, finding a go-to effect (such as `slit_scan` or `chroma_burst`) required navigating through all library scopes and paginating across dozens of procedures.

Furthermore, procedure rows in the Library bay lacked star toggle indicators, creating an inconsistent interaction model where only set rows presented favourite affordances.

## Decision

**1. Generalize Store Favourite Storage:**
- Update `Store::set_favourite` to accept procedure identifiers (both shipped procedures and `.kir` files residing in user storage or presets) alongside Set identifiers.
- Persist procedure favourites deterministically in `<store>/favourites/procedures.list`.

**2. Star Indicators and Hit-Testing on Procedure Rows:**
- Extend `crates/karakuri-console/src/view/library.rs` to render the star glyph (`★` / `☆`) on procedure rows across all library scopes (`Scope::All`, `Scope::L1` through `Scope::L5`, `Scope::Presets`).
- Enable hit-testing for procedure star glyphs, dispatching `Operation::SetFavourite { id, favourite }` on click.

**3. Unified `Scope::MySets` Presentation:**
- Update `Scope::MySets` query resolution to include both starred Sets and starred Procedures.
- Starred procedures appear directly in `MySets`, allowing performers to treat this view as a curated personal palette of both full compositions and essential modular tools.

## Consequences

- Performers can rapidly locate and drag their preferred procedural generators and post-processing effects during high-pressure live sets.
- The interaction model of the Library bay is unified: all visual items (sets and procedures) share the same bookmarking affordance.
- Defends [P-0090](../principles/0090-a-surface-offers-it-never-decides.md): the UI provides a consistent affordance without altering the underlying content-addressing or operation semantics.
