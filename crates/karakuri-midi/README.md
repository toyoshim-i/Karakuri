# `karakuri-midi`

MIDI hardware control surface interface for the Karakuri visual runtime: captures MIDI input, manages map file configurations, and translates controller messages into declarative [`karakuri_operation::Operation`] commands.

---

## 1. Role in Architecture

`karakuri-midi` sits at **Layer 2 (Domain Extensions & Storage)** in the Karakuri workspace:

- **Input**: Raw MIDI wire messages (`midir` input streams, Control Change, Note On/Off, Pitch Bend, 14-bit CC pairs).
- **Mapping**: Declarative text-based map tables (`.kmap`) associating hardware controls (pads, dials, faders) with target deck controls, operational gestures, or parameter indices.
- **Output**: Pure [`karakuri_operation::Operation`](../karakuri-operation/README.md) values emitted into the system's central operation stream.
- **Feedback Output**: Optional MIDI output surface synchronization (`Out`, `SurfaceReport`), reflecting current deck and parameter states onto motorized faders or illuminated LED pads.

---

## 2. Core Modules

| Module | Role | Testability |
|:---|:---|:---|
| [`message`](src/message.rs) | Zero-allocation byte parser decoding raw MIDI packets into structured [`Message`] variants (`ControlChange`, `NoteOn`, `NoteOff`, `PitchBend`). | Pure functions; 100% testable with byte fixtures. |
| [`map`](src/map.rs) | Parser and runtime evaluation of `.kmap` controller mappings: resolves hardware controls to [`Target`] destinations and scales inputs across continuous ranges. | Pure domain state; deterministic evaluation. |
| [`device`](src/device.rs) | Hardware abstraction managing `midir` input/output streams and thread synchronization channels. | Hardware-adjacent leaf. |

---

## 3. Key Invariants

1. **Decoupled from Engine**:
   - `karakuri-midi` does not mutate engine state directly. It translates hardware events into pure [`Operation`](karakuri_operation::Operation) values.
   - Operations flow through [`karakuri-operation-record`](../karakuri-operation-record/README.md) into the journal, ensuring deterministic session replay without requiring MIDI hardware.
2. **Surface Offers, Never Decides**:
   - Mapped controls only issue intent (operations). The engine's state validation and governor rules maintain ultimate authority.
3. **Hardware Independence**:
   - Controller mappings (`.kmap`) represent local hardware bindings and are separated from session files (`.kset`).
4. **Pure Functional Parsing**:
   - Message parsing and map evaluation are allocation-free in the steady state.

---

## 4. Testing & Verification

Run unit tests:

```sh
cargo test -p karakuri-midi
```
