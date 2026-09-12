# karakuri-pattern

Sequencer pattern representation, step lanes, and bank management for the Karakuri system.

---

## 1. Overview

`karakuri-pattern` is an intermediate domain crate (Layer 2) defining the data models and step-evaluation logic for Karakuri's step sequencer.

A sequencer lane serves as a declarative route into `karakuri-operation`: on beat boundaries, active steps emit strongly typed [`Operation`]s targeting parameters or faders, alongside manual pointer input, keyboard shortcuts, MIDI controllers, and AI MCP commands.

---

## 2. Key Design Invariants

1. **One Bar of Sixteen Slots**:
   - Each pattern stores exactly 16 slots (`u16` bitmask), regardless of whether evaluation mode is sixteenths or eighths (`StepMode`).
   - Switching between eighths and sixteenths changes the evaluation mapping ([`StepMode::slot_of`]) without discarding finer step data.

2. **Explicit Two-Level Steps (Gate / Pulse)**:
   - Steps do not write ambiguous impulse events. Each [`Lane`] specifies an `on` level and an `off` level.
   - An off-step writes the `off` value rather than doing nothing, ensuring deterministic step transitions.

3. **Pure Memory Model (No Clock, No File I/O)**:
   - Driven entirely by external musical position (`beats: f64`) supplied by `karakuri-signal::Oscillator`.
   - Generates [`Operation`] values without applying them or interacting with GPU pipelines directly.
   - File persistence and serialization are delegated to the environment and store layers.

---

## 3. Module Structure & Core Types

| Type | Description | Key Methods / Fields |
|:---|:---|:---|
| [`Pattern`] | A single musical bar composed of a `StepMode` and a list of `Lane`s | `lanes()`, `add_lane()`, `remove_lane()`, `toggle_slot()` |
| [`Lane`] | A single sequence lane driving an operation target with 16 slots and on/off levels | `target()`, `is_on()`, `set_on()`, `levels()`, `value_at()` |
| [`Banks`] | Session container managing 4 pattern banks (`A`, `B`, `C`, `D`) and the armed bank | `armed()`, `arm()`, `bank()`, `bank_mut()`, `first_empty()` |
| [`Playhead`] | Tracks step progression and detects step boundary crossings | `advance()`, `last_step()`, `reset()` |

---

## 4. Verification

Run unit tests:

```sh
cargo test -p karakuri-pattern
```
