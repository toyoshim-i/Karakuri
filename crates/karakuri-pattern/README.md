# karakuri-pattern

Sequencer pattern representation, step lanes, and bank management for the Karakuri system.

---

## 1. Overview

`karakuri-pattern` is an intermediate domain crate (Layer 2) defining the data models and step-evaluation logic for Karakuri's step sequencer.

A sequencer lane serves as a declarative route into `karakuri-operation`: on beat boundaries, active steps emit strongly typed [`Operation`]s targeting parameters or faders, alongside manual pointer input, keyboard shortcuts, MIDI controllers, and AI MCP commands.

---

## 2. Key Design Invariants

1. **One Bar of Sixteen Slots**:
   - Each [`Lane`] stores exactly 16 slots (`u16` bitmask), regardless of whether evaluation mode is sixteenths or eighths (`StepMode`).
   - Switching between eighths and sixteenths changes the evaluation mapping ([`StepMode::slot_of`]) without discarding finer step data.

2. **Explicit Two-Level Steps (Gate / Pulse)**:
   - Steps do not write ambiguous impulse events. Each [`Lane`] specifies an `on` level and an `off` level.
   - An off-step writes the `off` value rather than doing nothing, ensuring deterministic step transitions.

3. **Pure Memory Model (No Clock, No File I/O)**:
   - Driven entirely by external musical position (`beats: f64`) supplied by `karakuri-signal::Oscillator`.
   - Generates [`Operation`] values without applying them or interacting with GPU pipelines directly.
   - The crate holds no serializer and no path: a pattern has no file form, and no other crate serializes one.

---

## 3. Public API

The crate is a single module (`src/lib.rs`). `LaneTarget`, `Operation` and `StepMode` are owned by `karakuri-operation` and used, not re-exported.

### Constants

| Item | Description |
|:---|:---|
| `SLOTS: usize` | Number of step slots stored per pattern across both eighth and sixteenth modes. |
| `BANKS: usize` | Maximum number of pattern banks maintained per session. |

### [`Lane`]

A single sequence lane driving an operation target with 16 slots and distinct on/off levels.

| Method | Description |
|:---|:---|
| `new(target: LaneTarget, on: f32, off: f32) -> Lane` | Creates a new lane targeting `target` with specified `on` and `off` levels. |
| `target(&self) -> &LaneTarget` | Returns the target driven by this lane. |
| `on(&self) -> f32` | Returns the value written on an active step. |
| `off(&self) -> f32` | Returns the value written on an inactive step. |
| `muted(&self) -> bool` | Returns whether this lane is muted. |
| `set_muted(&mut self, muted: bool)` | Sets the muted state of this lane. |
| `slot_on(&self, slot: usize) -> bool` | Returns whether the stored slot at `slot` is active. |
| `set_slot(&mut self, slot: usize, on: bool)` | Sets the active state for the specified stored slot. |
| `step_on(&self, step: usize, mode: StepMode) -> bool` | Returns whether the step in the given `mode` is active. |
| `value_at(&self, step: usize, mode: StepMode) -> f32` | Returns the level written at the specified step and mode. |
| `operation_at(&self, step: usize, mode: StepMode) -> Operation` | Emits the operation produced by this lane at the specified step. |

### [`Pattern`]

A single musical bar composed of a subdivision mode and a list of sequence lanes.

| Method | Description |
|:---|:---|
| `empty() -> Pattern` | Creates an empty pattern in sixteenths mode with no lanes. |
| `mode(&self) -> StepMode` | Returns the active step subdivision mode. |
| `set_mode(&mut self, mode: StepMode)` | Sets the step subdivision mode without mutating underlying slot states. |
| `lanes(&self) -> &[Lane]` | Returns the sequence lanes. |
| `lane_mut(&mut self, at: usize) -> Option<&mut Lane>` | Returns a mutable reference to the lane at `at`, if present. |
| `push(&mut self, lane: Lane)` | Appends a new lane to the pattern. |
| `remove(&mut self, at: usize) -> Option<Lane>` | Removes the lane at `at` and returns it, or `None` for an index this pattern does not hold. |
| `is_empty(&self) -> bool` | Returns whether this pattern contains any lanes. |
| `step_at(&self, beats: f64) -> usize` | Computes the step index corresponding to the given musical beat position. |
| `due(&self, step: usize) -> impl Iterator<Item = Operation>` | Returns an iterator of operations emitted by unmuted lanes at `step`. |
| `held(&self) -> impl Iterator<Item = (usize, &LaneTarget)>` | Returns the lanes that hold a control, as `(lane index, target)` pairs in lane order. |

A lane's index is its position in `Pattern::lanes` and not an identity: `remove` moves the lanes after it up one, so every index above it names a different lane afterwards.

### [`Banks`]

Four pattern banks with an active armed index.

| Method | Description |
|:---|:---|
| `default() -> Banks` | Creates four empty banks with bank 0 armed. |
| `armed(&self) -> usize` | Returns the index of the currently armed bank. |
| `select(&mut self, bank: usize) -> bool` | Selects the active pattern bank, returning `false` if `bank >= BANKS`. |
| `pattern(&self) -> &Pattern` | Returns a reference to the armed pattern. |
| `at(&self, bank: usize) -> Option<&Pattern>` | Returns a reference to the bank at index `bank`. |
| `at_mut(&mut self, bank: usize) -> Option<&mut Pattern>` | Returns a mutable reference to the bank at index `bank`. |
| `first_empty(&self) -> Option<usize>` | Returns the index of the first empty bank, or `None` if all banks contain lanes. |

### [`Playhead`]

Tracks playhead step advancement and detects step boundary transitions.

| Method | Description |
|:---|:---|
| `advance(&mut self, pattern: &Pattern, beats: f64) -> Option<usize>` | Advances the playhead given the pattern and beat count, returning `Some(step)` on boundary transitions. |
| `at(&self) -> Option<usize>` | Returns the current step index, or `None` if the playhead has not yet been polled. |
| `reset(&mut self)` | Resets playhead history, forcing the next poll to register as a boundary transition. |

---

## 4. Verification

Run unit tests:

```sh
cargo test -p karakuri-pattern
```

