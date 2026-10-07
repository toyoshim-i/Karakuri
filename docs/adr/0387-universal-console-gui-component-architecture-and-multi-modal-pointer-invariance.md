---
id: 0387
title: Universal Console GUI component architecture and multi-modal pointer invariance
status: accepted
date: 2026-10-07
principles: [0084, 0090, 0092, 0094]
tags: [console, ui, widgets, pointer, components, webxr, touch, architecture]
---

# Universal Console GUI component architecture and multi-modal pointer invariance

## Context

Karakuri's console interface operates across diverse interaction modalities:
1. **Desktop**: Traditional mouse cursor position, clicks, drags, and wheel events dispatched through the window event loop.
2. **Touch surfaces**: Pointer down, drag, and release events mapped from single-point and multi-touch interactions.
3. **WebXR**: 6DoF spatial controller laser rays, trigger squeezes, and hand tracking gestures intersecting with the virtual console HUD.

Historically, UI primitives were componentized into `karakuri-console::view::widgets` (such as `Card`, `Chip`, `Track`, `Fader`, `BayHead`, and `Pills`) and modular bay coordinators (`view/mixer`, `view/transport`, `view/inspector`, `view/library`, `view/program`). Interaction was routed through a centralized pointer dispatch pipeline in `readout.pointer()` and `dispatch_press()`, which resolves hits against geometric boundaries and emits typed operations.

However, during recent UI additions, ad-hoc shortcuts began appearing: interactive logic (e.g. calling egui's `ui.interact(..., egui::Sense::click())` or inspecting `ui.input(|i| i.pointer.primary_clicked())`) was directly embedded inside rendering and drawing passes (`draw.rs`).

This ad-hoc pattern breaks fundamentally in multi-modal environments:
- In WebXR, controller laser ray intersections and trigger pulls are translated into `PointerAction` and fed into `readout.pointer()`. When a widget relies on egui's internal window click queue rather than the canonical pointer dispatch, the interaction is completely bypassed or swallowed as background clicks.
- In modal overlay scenarios (such as floating dropdowns or popup cards), uncoordinated egui interactions conflict with `ModalOverlay` dismiss semantics, causing menus to close prematurely or ignore item selections.
- Testing becomes brittle because GUI behavior cannot be verified through pure headless geometry and dispatch unit tests.

To preserve architectural integrity across all current and future input devices, the console subsystem requires an unambiguous, universally enforced component standard.

## Decision

Every interactive GUI element in Karakuri—without exception (including buttons, pills, dropdown menus, faders, chips, cards, badges, and preview cells)—must strictly adhere to the **Three-Layer Component Pattern**:

### 1. Geometry & Hit-Testing Layer (`Layout & Hit-Test`)
- A component must expose an explicit layout representation with geometric boundaries (`Rect`).
- It must provide pure mathematical hit-testing functions:
  - `hit(&self, p: Point) -> bool`: whether a coordinate falls within the primary control.
  - `owns(&self, p: Point) -> bool`: whether a coordinate belongs to the component or any of its active sub-elements (e.g. open dropdown cards).
  - `item(&self, p: Point) -> Option<T>`: which constituent item (if any) is targeted.
  - `ask(&self, state: &State, p: Point) -> Option<Ask>`: transforms a pointer click into a domain-specific interaction enum (`Ask`), such as `Toggle`, `Select(index)`, or `Shut`.
- This layer has zero dependencies on rendering contexts or OS event queues.

### 2. Pure Presentation Layer (`Pure Paint`)
- Rendering functions (e.g. `*_into(ui, pal, ...)` or `draw_*`) must be strictly **pure and side-effect free**.
- Painters receive geometry, theme tokens (`Palette`), and current state. They render visuals using egui's `Painter`, `popup_card`, `card_row_text`, and standardized glyphs.
- **Never call `ui.interact()`, `ui.input(|i| i.pointer.primary_clicked())`, or mutate state inside the paint pass.** The paint pass only renders; it does not consume or decide input.

### 3. Canonical Pointer Dispatch Layer (`readout.pointer`)
- All user interaction—whether originating from a physical mouse, a touchscreen tap, or a WebXR VR controller trigger—routes exclusively through `readout.pointer(&ctx, which)`.
- The dispatch hierarchy (`crates/karakuri/src/readout/dispatch/press.rs`) evaluates active modal overlays and bay components using their `ask()` methods.
- The outcome is handled cleanly: mutating view state, emitting a strongly-typed `Operation`, or requesting a repaint (`Acted::Pointed`).
- Egui event forwarding (`App::to_egui`) occurs only when an element explicitly yields to native text input widgets.

## Enforcement

1. **Architecture & Guidelines**: Documented in `docs/architecture/console.md` and explicitly enforced in `docs/contributing.md`.
2. **Prohibited Patterns**:
   - `ui.interact(..., egui::Sense::click())` for custom console controls and bay widgets.
   - `ui.input(|i| i.pointer.primary_clicked())` inside custom widget rendering.
   - Inline ad-hoc modal overlays that bypass `ModalOverlay` and `dispatch_press`.

## Consequences

- **Multi-Modal Invariance**: Every GUI control functions identically and reliably across Desktop, Touch, and WebXR VR (laser pointer + trigger).
- **Testability**: Control interaction can be thoroughly verified through deterministic, headless unit tests using `readout.pointer(&ctx, ...)`.
- **Modularity**: Visual styling and input dispatch remain decoupled, preventing subtle regressions when styling or refactoring UI panels.
