---
id: 0385
title: Web MIDI API control surface integration via decoupled Port feeder
status: accepted
date: 2026-10-05
principles: [0084, 0090, 0092, 0094]
tags: [midi, web, wasm, control-surface, webmidi]
---

# Web MIDI API control surface integration via decoupled Port feeder

## Context

Native Karakuri integrates hardware MIDI control surfaces (e.g. Korg nanoKONTROL2) via `midir` (`midir::MidiInput`, `midir::MidiOutput`), parsing raw 1..3 byte wire messages, routing them to deck operations (faders, knobs, solo/mute/tally buttons), and providing bidirectional LED and motorized fader feedback (`Lit`, `Echo`).

In WebAssembly (`wasm32-unknown-unknown`), native OS MIDI devices via `midir` are unavailable. Instead, modern web browsers provide the Web MIDI API (`navigator.requestMIDIAccess`).

A critical design requirement governs the Web MIDI input implementation:
- **Backend replacement isolation**:
  The core MIDI message parsing, 7-bit / 14-bit CC resolution, mapping file parser, deck parameter routing, intra-frame continuous value coalescing, MIDI learn engine, and bidirectional feedback must not be duplicated or modified for the Web platform. The Web implementation must strictly act as a backend transport replacement that feeds wire bytes into and receives feedback bytes from the existing Karakuri MIDI pipeline.

## Decision

1. **Decoupled Port & Out Abstractions (`karakuri-midi`)**:
   - Refactor `Port` to decouple from `midir::MidiInputConnection`: store `_connection: Option<MidiInputConnection>` alongside an untyped `_handle: Option<Box<dyn Any + Send>>` to keep custom background subscriptions alive.
   - Introduce `SenderPort` with `send(Message)` and `send_bytes(&[u8])` to parse raw incoming wire messages and enqueue them into the port's internal queue while waking the Winit event loop via proxy waker.
   - Add `Port::custom(name, wake) -> (Port, SenderPort)` and `Port::custom_with_handle(...)`.
   - Generalize `Out` with an `OutSink` enum (`SyncChannel` for native worker threads vs `Custom(Box<dyn Fn([u8; 3]) -> bool + Send>)` for Web outputs).
   - Add `Out::custom(name, send_fn)` to allow routing outgoing 3-byte MIDI feedback messages through arbitrary platform closures.

2. **Unified Surface Construction (`karakuri-environment::midi`)**:
   - Add `Surface::from_port(port, out, map_path)` and `Surface::custom(port, out, map_text, map_name)` to allow instantiating standard `Surface` control surface controllers from custom ports and arbitrary map sources without requiring disk access.
   - Embed the repository's default control surface layout (`examples/surface.map`) as a fallback in the Web module, enabling immediate out-of-the-box hardware controller support while preserving interactive UI MIDI Learn capability.

3. **Web MIDI API Integration (`karakuri-web::midi`)**:
   - Call `navigator.requestMIDIAccess({ sysex: false })` using `wasm-bindgen-futures`.
   - Iterate over `MidiAccess.inputs()` and connect the first available controller to `Port::custom` using `MidiInput.set_onmidimessage`.
   - Iterate over `MidiAccess.outputs()` and wrap any available output into `Out::custom` calling `MidiOutput.send(...)`.
   - Bundle the resulting `Port` and `Out` into `Surface::custom` and transfer it to the event loop.

4. **Lifecycle & Map Pill Interaction (`App::set_midi_request_hook`)**:
   - Add `Acted::MidiRequest` returned when clicking the transport row's `map` pill.
   - In `WebApp::new`, trigger an automatic Web MIDI scan on startup, and register `set_midi_request_hook` to rescan or reconnect when the user clicks the `map` pill.
   - On connection, attach the surface via `App::attach_midi`, updating the UI map pill (`map · surface` or `map · none`) and refreshing the console viewport.

## Consequences

- Full hardware MIDI control surface support (faders, knobs, mute/solo toggles, parameter learning, and LED/motorized fader feedback) is operational in WebAssembly.
- The entire MIDI engine—wire parsing, 14-bit CC coalescing, routing, and learning—remains 100% shared and single-sourced.
- Desktop MIDI behavior and existing `midir` streams remain completely untouched.
