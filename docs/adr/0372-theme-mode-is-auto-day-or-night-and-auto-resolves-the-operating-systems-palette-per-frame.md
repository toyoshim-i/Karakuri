---
id: 0372
title: Theme mode is Auto, Day, or Night, and Auto resolves the operating system's palette per frame
status: accepted
date: 2026-09-27
supersedes: []
superseded_by: []
principles: [0090]
tags: [console, theme, room, modal, transport, m9]
---

# Theme mode is Auto, Day, or Night, and Auto resolves the operating system's palette per frame

## Context

Live audiovisual performances take place across vastly contrasting ambient environments:
sunlit outdoor music festivals, dimly lit club stages, and brightly illuminated rehearsal rooms.
In high-glare daytime conditions, the console's default dark room palette (`Room::Night`) washes out,
destroying fader visibility, label legibility, and meter precision. Conversely, in dark club venues,
a high-luminance light palette (`Room::Day`) blinds the operator and compromises stage lighting.

Prior to this decision, palette configuration required manual compilation flags or static configuration,
leaving operators without immediate in-console control during rehearsal or performance. Furthermore,
modern desktop operating systems (macOS, Windows, and Linux desktop environments) dynamically shift
between system-wide light and dark appearance according to schedule, ambient light sensors, or operator preference.
Without dynamic tracking, the console remained out of sync with the host window environment.

The console required a responsive, modal-safe mechanism to switch theme palettes instantaneously on stage,
while providing hands-free automatic alignment with host operating system appearance.

## Decision

**1. `ThemeMode` Enumeration:**
- Introduce `ThemeMode` in `karakuri_console::room` with three states:
  ```rust
  #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
  pub enum ThemeMode {
      #[default]
      Auto,
      Day,
      Night,
  }
  ```
- In `ThemeMode::Day`, the console renders exclusively with the high-contrast light room palette (`Room::Day`).
- In `ThemeMode::Night`, the console renders exclusively with the dark room palette (`Room::Night`).
- In `ThemeMode::Auto`, the console inspects the host window's system theme (`winit::window::Theme` via `egui::Theme`)
  on every frame, resolving dynamically to `Room::Night` when the OS reports dark appearance, and falling back to
  `Room::Day` when light appearance or unspecified.

**2. Transport Row Theme Pill:**
- Mount an interactive selector pill labelled `theme · <mode> ▾` on the Transport strip.
- Position the pill after the arrangement, map, learn, tracker, and audio-in controls, and immediately
  before the transport frame budget meter (`row.frame.min.x`).
- Left-clicking the pill toggles an anchored modal dropdown card displaying all three modes (`auto`, `day`, `night`)
  with active mode indicators.
- Clicking an entry updates the active `ThemeMode` immediately and dismisses the card.

**3. Rule 2 Modal Mutual Exclusion:**
- Integrate the theme dropdown card with `ModalOverlay::ThemeMenu`.
- Comply with console Rule 2 modal mutual exclusion: opening the theme menu dismisses any active arrangement,
  preset, or map overlay card.
- Clicking outside the theme pill and dropdown card dismisses the modal overlay without changing the mode.

## Alternatives rejected

- **Static configuration file editing**: Modifying JSON or TOML configuration files requires an application
  restart or external editor, which is unviable during live performances or soundchecks.
- **Unindicated keyboard toggle shortcut**: A blind toggle key without dedicated transport pill feedback
  leaves the operator unable to discern whether the active palette is in `Auto` host-tracking mode or an explicit
  manual override, risking unexpected palette shifts during ambient lighting changes.
- **Polling OS theme on a background interval timer**: Querying the operating system asynchronously introduces
  timer jitter and thread synchronization overhead; resolving `egui::Context::theme()` per frame leverages
  event-driven window notifications zero-cost during rendering.
