---
id: 0383
title: Prompt bay inline IME composition and cursor area anchoring
status: accepted
date: 2026-10-03
principles: [0090, 0094]
tags: [prompt, terminal, ime, preedit, web, cjk]
---

# Prompt bay inline IME composition and cursor area anchoring

## Context

In Milestone 9, the Prompt bay introduced an integrated agent terminal emulator supporting detached PTY background processes and Japanese IME candidate positioning.

While committed text was transmitted to the shell as UTF-8 bytes upon confirmation (`ImeEvent::Commit`), uncommitted composition text (Preedit) was not rendered on the terminal grid. During Japanese input:
- The terminal cursor position remained blank while typing.
- Uncommitted characters appeared only within the floating OS candidate window.
- Characters appeared on screen only in a single batch after pressing Enter to commit.
- Precise IME cursor bounds were not fed into egui's output channel, causing native candidate popup positioning to rely on window defaults.

This produced a jarring, legacy "over-the-spot" typing experience differing sharply from standard macOS and web editors (VSCode, iTerm2, Alacritty), where inline preedit text is rendered directly at the cursor with active underlines.

## Decision

1. **Preedit State Management (`PromptState`)**:
   - Add `preedit: Arc<Mutex<Option<String>>>` to `PromptState` alongside accessor methods (`preedit()`, `set_preedit()`, `clear_preedit()`).
   - Listen to `egui::Event::Ime(egui::ImeEvent::Preedit { text, .. })` in `handle_terminal_events` to update the active composition buffer, clearing it on `ImeEvent::Commit`.

2. **Inline Terminal Grid Composition Rendering (`paint.rs`)**:
   - When rendering the row containing the terminal cursor, insert the active `preedit` text directly at `cursor.1` before following terminal cells.
   - Style the preedit span with high contrast mint text (`pal.mint`), subtle tinted backing, and an explicit 1.5px underline stroke (`Stroke::new(1.5, pal.mint)`).
   - Suppress the block cursor cell highlight while composing so the visual focus remains on the active composition span.

3. **Precise IME Cursor Area Anchoring (`output.ime`)**:
   - Report the terminal cursor's exact screen rectangle (accounting for character cell dimensions, font line pitch, and composition text length) to `ui.ctx().output_mut(|o| o.ime = ...)`.
   - This commands `winit::window::Window::set_ime_cursor_area`, anchoring OS candidate windows (macOS Japanese/Chinese input palette) directly beneath the active insertion caret.

4. **Web DOM Composition Pipeline (`ime_overlay.rs`)**:
   - Register a `compositionupdate` event listener on the helper `<textarea>`, forwarding intermediate composition strings to `prompt_state.set_preedit` and waking the Winit event loop (`proxy.send_event(())`) for instantaneous redraws.
   - Clear preedit on `compositionend` prior to dispatching committed text bytes to PTY stdin.

## Consequences

- Full, modern inline Japanese/CJK composition is restored: characters appear underlined in-place at the cursor as they are typed, matching expected OS behavior.
- OS candidate windows follow the cursor and composition tail seamlessly on both desktop and WebAssembly environments.
