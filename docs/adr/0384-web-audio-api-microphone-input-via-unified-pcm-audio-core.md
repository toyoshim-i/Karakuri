---
id: 0384
title: Web Audio API microphone input via unified PCM audio core
status: accepted
date: 2026-10-05
principles: [0084, 0090, 0094]
tags: [audio, web, wasm, microphone, pcm, webaudio]
---

# Web Audio API microphone input via unified PCM audio core

## Context

Native Karakuri captures live audio from hardware interfaces using CPAL (`cpal::default_host().input_devices()`). Audio frames are analyzed in `karakuri-audio` via `Analyzer` (512-sample FFT spectral band decomposition) and `Tracker` (onset detection and tempo tracking) to drive oscillator synchronization and audio-reactive signal modulation.

In WebAssembly (`wasm32-unknown-unknown`), CPAL hardware input streams are unavailable. Browsers require the Web Audio API (`navigator.mediaDevices.getUserMedia`) to access user microphones.

Two critical design constraints govern the Web audio input implementation:
1. **Zero Browser-Specific Audio Analysis (Future Spectrogram Compatibility)**:
   Downstream milestones include Audio Spectrogram / Waterfall history textures and multi-band FFT persistence (1-2). Implementing Web audio via browser-specific analysis nodes (e.g. `AnalyserNode`) would fracture the pipeline and require maintaining dual implementations. The Web layer must feed raw PCM audio samples directly into the same unified Rust audio core so that all future spectral analysis and texture features work automatically on both desktop and WebAssembly.
2. **Transient User Gesture Constraints**:
   Modern web browsers mandate user gesture activation (transient user activation context from clicks or keypresses) before invoking `getUserMedia`. Requesting microphone permissions at application boot without a user gesture triggers browser security blocks.

## Decision

1. **Unified Audio Core Abstraction (`karakuri-audio::AudioCore`)**:
   - Extract spectral analysis, onset detection, and beat tracking logic from CPAL callback closures into a standalone `AudioCore` pipeline.
   - Expose `AudioCore::feed(&mut self, mono: &mut dyn Iterator<Item = f32>, frames: usize, rate: f32)` to ingest arbitrary mono PCM sample streams.
   - Add `AudioInput::custom(description, sample_rate, centre_bpm) -> (AudioInput, Arc<Mutex<AudioCore>>)` allowing non-CPAL audio sources (Web Audio API or tests) to pair with standard `AudioInput` readers.
   - Add `Audio::from_input(input, latency_offset_ms, dt)` in `karakuri-environment` to wrap custom inputs uniformly into live `Audio` sessions.

2. **Web Audio API Capture Pipeline (`karakuri-web::audio`)**:
   - Request microphone streams via `navigator.mediaDevices.getUserMedia({ audio: true })`.
   - Initialize an `AudioContext` and configure a `ScriptProcessorNode` (buffer size 1024, 1 input channel, 1 output channel).
   - In `onaudioprocess`, extract `f32` PCM channel buffer data and pass it into `AudioCore::feed(...)`.
   - Connect `ScriptProcessorNode` to a muted `GainNode(0.0)` connected to `destination`. This guarantees continuous browser audio processing graph execution across all browser engines while eliminating acoustic feedback loops through speakers.

3. **User Gesture-Driven Permission Activation (`App::set_audio_request_hook`)**:
   - Register an audio request hook on `App` via `App::set_audio_request_hook`.
   - Dispatch the hook synchronously during user interactions with audio controls: clicking the transport row's `audio-in` pill (`AudioAsk::Open`) or selecting an audio input in dropdown cards (`Operation::AttachBeatSource`).
   - Trigger `getUserMedia` inside this transient user gesture context.
   - Upon permission grant, asynchronously dispatch the initialized `Audio` session to the event loop proxy (`proxy.send_event(())`), attaching it to the live session via `App::attach_audio`.
   - Transition the transport pill readout from `audio-in · none` to `audio-in · Default Microphone` and start beat tracking and energy analysis.

## Consequences

- Full live microphone capture, tempo tracking, beat locking, and acoustic frequency band analysis are enabled in WebAssembly.
- Feeding raw PCM samples ensures that future spectrogram and waterfall history textures implemented in Rust core will work on Web without any Web-specific modifications.
- Desktop audio behavior and existing CPAL streams remain completely untouched.
