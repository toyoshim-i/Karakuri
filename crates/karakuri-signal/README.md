# karakuri-signal

Clock oscillators, tempo tracking, noise generators, and the signal bus for the Karakuri system.

---

## 1. Overview

`karakuri-signal` is a foundational leaf crate (Layer 1) providing deterministic timing, musical phase tracking, algorithmic noise generation, and parameter modulation signals. It has zero dependencies on other Karakuri workspace crates.

In Karakuri, shader procedures never read external clocks or audio hardware directly. Instead, external inputs are mapped to declarative parameter bindings that sample from the signal bus provided by this crate.

---

## 2. Key Design Invariants

1. **No External Clock in Rendering**:
   - The [`Oscillator`] advances strictly through simulation steps: `Oscillator::advance(steps, dt)`.
   - External tempo/MIDI/beat trackers never drive time directly; they apply continuous corrections via `Oscillator::correct(bpm, shift_beats)`.
   - Enforced by architectural tests in `tests/no_clock_access.rs`.

2. **Complete Bus Contract (No Missing Signals)**:
   - The signal bus is always complete. Looking up any signal name produces a valid [`Sample`], never an `Option::None`.
   - Consumers blend or branch based on [`Sample::confidence`] (`[0.0, 1.0]`), never on provider presence.
   - When no audio provider is active, synthesized mathematical defaults are produced with low confidence.

3. **Bit-for-Bit Determinism**:
   - Every bus sample and noise evaluation is a pure function of the oscillator state (`t`, `bpm`), explicit seeds, and stream IDs.
   - Replaying an identical step history produces identical output across all platforms.

4. **Tempo vs. Phase Decoupling**:
   - `Oscillator::beats()` tracks musical position including phase shifts (aligning beat grids to room tempo).
   - `Oscillator::elapsed_beats()` accumulates only tempo scaling without phase jumps, used by [`NoiseConfig`] to prevent noise glitches during phase realignment.

---

## 3. Module Structure

| Module | Description | Core Types / APIs |
|:---|:---|:---|
| [`oscillator`](src/oscillator.rs) | Single source of truth for phase, tempo, and simulation time | [`Oscillator`], [`BPM_RANGE`], `BEATS_PER_BAR` |
| [`bus`](src/bus.rs) | Synthesized signal bus derived purely from oscillator state | [`SynthesizedBus`], `SignalBus` trait |
| [`measured`](src/measured.rs) | Measured audio and sensor signals with confidence scoring | [`AudioFrame`], [`MeasuredBus`], `MAX_BANDS`, `ENERGY`, `ONSET` |
| [`noise`](src/noise.rs) | Deterministic lattice noise generators | [`NoiseConfig`], [`NoiseKind`] (`White`, `Value`, `Perlin`, `Fbm`) |
| [`id`](src/id.rs) | Type-safe signal identification and resolution | `SignalId`, `SignalValue`, `VectorSample` |

---

## 4. Verification

Run unit and integration tests:

```sh
cargo test -p karakuri-signal
```
