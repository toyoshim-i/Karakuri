---
id: 0368
title: The audio signal bus resolves eight acoustic frequency bands by semantic names at frame boundaries
status: accepted
date: 2026-09-21
supersedes: []
superseded_by: []
principles: [0091, 0092]
tags: [audio, signal, engine, fft, m8]
---

# The audio signal bus resolves eight acoustic frequency bands by semantic names at frame boundaries

## Context

Procedural graphics (.kir shaders) and visual parameters in Karakuri require low-latency synchronization
with incoming music. Prior to milestone M8, the signal subsystem provided only global energy, onset detection,
or indexed frequency arrays (`audio.band0`..`audio.band7`).

This approach suffered from two significant design problems:
1. **Unintuitive signal authoring**: Visual artists and shader writers do not think in raw FFT bin numbers
   or abstract numeric indices. They think in acoustic and musical components—driving bloom from the sub-bass
   kick, warping geometries to the bassline, pulsing particles to snare hits, or shimmering highlights to hi-hat
   transients. Requiring obscure indices in `.kset` bindings added cognitive friction and obscured artistic
   intent.
2. **Bandwidth and determinism hazards**: Passing high-resolution raw FFT bins (1024 to 2048 frequency points)
   directly into GPU shader uniform buffers every frame wastes GPU memory bus bandwidth and forces every shader
   author to re-implement filtering, windowing, and logarithmic summation in WGSL. Furthermore, allowing the
   render thread to asynchronously poll audio analysis buffers mid-frame introduces sampling jitter and race
   conditions, violating principle [P-0092](../principles/0092-the-same-inputs-produce-the-same-frame.md):
   *"the same inputs produce the same frame"*.

## Decision

**1. Eight musically partitioned acoustic frequency bands (40 Hz – 16 kHz).**
In `karakuri-audio::analysis`, the audible spectrum is segmented into eight logarithmically spaced acoustic
bands matched to musical sound engineering conventions:
- **Band 0 (`sub` / `audio.sub`)**: ~40–85 Hz (sub-bass rumble, kick drum fundamentals).
- **Band 1 (`bass` / `audio.bass`)**: ~85–180 Hz (bassline body, punch, low toms).
- **Band 2 (`low_mid` / `lowmid` / `audio.low_mid`)**: ~180–380 Hz (warmth, low vocal body, guitar body).
- **Band 3 (`mid` / `audio.mid`)**: ~380–800 Hz (snare crack, lead instruments, fundamental vocal clarity).
- **Band 4 (`high_mid` / `highmid` / `audio.high_mid`)**: ~800–1700 Hz (upper harmonics, vocal bite, attack).
- **Band 5 (`presence` / `audio.presence`)**: ~1700–3600 Hz (percussive articulation, speech intelligibility).
- **Band 6 (`brilliance` / `audio.brilliance`)**: ~3600–7500 Hz (harmonic sizzle, guitar string edge).
- **Band 7 (`air` / `high` / `audio.air` / `audio.high`)**: ~7500–16000 Hz (hi-hat shimmer, cymbal decay, air).

**2. Direct semantic name resolution in `SignalId::resolve`.**
In `karakuri-signal::id`, `SignalId::resolve` parses both namespaced identifiers (`audio.sub`, `audio.bass`)
and naked semantic band names (`sub`, `bass`, `low_mid`, `mid`, `high_mid`, `presence`, `brilliance`, `air`, `high`).
Shader bindings and parameter maps can bind directly to semantic names in Set files (`.kset`), for example:
```kset
bind L1:0:glow -> bass { curve: exp(2.0), smooth: 0.1 }
bind L2:0:sparkle -> air { curve: linear, min: 0.0, max: 1.0 }
```

**3. Deterministic latching via lightweight `AudioFrame` at frame boundaries.**
In `karakuri-signal::measured`:
- Audio analysis results are packed into a compact `Copy` structure (`AudioFrame`) containing the 8 band
  magnitudes, normalized energy, onset impulse, and spectral centroid.
- Communication between the high-frequency audio analysis thread and the render loop is strictly decoupled.
  At the beginning of each render frame boundary, the engine latches the most recent `AudioFrame` exactly once.
- The latched frame values are uploaded into GPU uniform buffers deterministically without dynamic memory
  allocations on either the audio or graphics thread, satisfying
  [P-0091](../principles/0091-cost-is-known-before-it-is-paid.md) and
  [P-0092](../principles/0092-the-same-inputs-produce-the-same-frame.md).

## Alternatives rejected

- **Uploading raw FFT bins (1024–2048 floats) to GPU uniforms**:
  Transmitting thousands of raw spectral bins each frame consumes unbudgeted GPU memory bandwidth and shifts
  acoustic psychoacoustic weighting into shader code, inflating shader compile complexity and execution cost.
- **Asynchronous mid-frame audio polling**:
  Sampling audio values dynamically while individual render passes are being encoded causes timing skew
  between layers rendered early in a frame and layers rendered late. Latching once per frame boundary ensures
  all layers and passes in a given frame observe identical, bit-exact audio telemetry.
