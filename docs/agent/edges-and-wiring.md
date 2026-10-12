---
id: edges-and-wiring
title: Edges and Input Wiring
summary: Rules for connecting procedure inputs (`uses`) to provider nodes within a Set.
---

# Edges and Input Wiring

Procedures in Karakuri declare external dependencies in their header using `uses`:

```
uses far : Geometry
uses shape : Field
uses view : Camera
uses only : Source
```

A procedure never names the provider node directly in source code. Provider node binding is defined externally through edges within the Set.

## Invariants

1. **Node Names, Not Addresses**:
   Both endpoints of an edge are identified by semantic node names (e.g. `morph`, `sphere_shell`), never by layer/index coordinate addresses. Node names remain stable when files are reordered.
2. **Replacement on Rewire**:
   Rewiring an already-connected input replaces the existing edge atomically without requiring prior disconnection. Other edges remain untouched.
3. **Edge Persistence**:
   Edges persist in the Set configuration until explicitly replaced. If a procedure removes a `uses` declaration without rewiring, the Set will fail to build until the unreferenced edge is updated.
4. **Type Compatibility**:
   The provider node must match the declared input type:
   - `: Geometry` or `: Source` requires an `L1` geometry provider.
   - `: Field` requires a `Field` distance function provider.
   - `: Camera` requires an `L3` procedure or the built-in camera.
   Mismatched types are rejected during Set assembly.
