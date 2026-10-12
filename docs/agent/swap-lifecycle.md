---
id: swap-lifecycle
title: Procedure Swap Lifecycle & Overload State
summary: Asynchronous GPU compilation, 30-frame budget measurement, and overload state behavior.
---

# Procedure Swap Lifecycle & Overload State

When a procedure is written via `write_procedure` or rewired via `wire_input`, compilation and activation proceed asynchronously off the render thread.

## Execution Stages

1. **Validation**: Static syntax, type, and local cost checks run immediately before accepting the write.
2. **Worker Compilation**: Shaders compile to WGSL on background worker threads.
3. **Atomic Swap**: Successful compilations swap into the active deck at the next frame boundary without blocking render execution.
4. **Performance Measurement**: The new procedure executes under live GPU workload for 30 frames to measure actual frame cost against the target budget.

## Overload State

If a procedure exceeds its frame budget during the 30-frame measurement window:
- The procedure enters the **overload state**.
- The slot freezes execution and holds its last rendered frame to protect overall system framerate.
- The engine does not automatically revert or discard the procedure.
- The slot resumes normal rendering only when a lighter procedure is written, an earlier revision is restored, or the operator adjusts deck controls.

Use `swap_outcome` to inspect recent swap results and verify whether a queued change landed successfully, encountered build errors, or entered the overload state.
