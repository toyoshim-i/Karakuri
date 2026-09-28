---
id: 0373
title: Prompt capture mode bypasses all keymaps and forwards escape and control keys directly to the pty
status: accepted
date: 2026-09-27
supersedes: [0365]
superseded_by: []
principles: [0090, 0094]
tags: [console, prompt, pty, terminal, keyboard, focus, m9]
---

# Prompt capture mode bypasses all keymaps and forwards escape and control keys directly to the pty

## Context

[ADR-0365](0365-the-prompt-bay-embeds-an-interactive-multi-session-terminal-multiplexer-with-cursor-anchored-input.md)
introduced the embedded interactive Prompt bay terminal multiplexer for live human-agent collaboration.
In Section 6 (*Focus Ladder & Capture Mode*), ADR-0365 specified:

> In terminal capture mode, all keyboard events are trapped and forwarded to PTY `stdin` (enabling single-key
> confirmations, arrow navigation, Ctrl+C, Ctrl+D).
> `Tab` navigates across bays; `Esc` releases capture back to console bay-level focus.

In practice with interactive agent CLIs and full-screen terminal utilities (such as Claude Code, Aider,
`fzf`, `vim`, and interactive language REPLs), `Escape` (`0x1b`) is a vital operational primitive:
- In modal editing (vi mode), `Escape` switches from insert mode back to normal navigation mode.
- In interactive fuzzy matchers and prompt menus, `Escape` cancels an ongoing completion query or closes an overlay card.
- In multi-turn agent CLIs, `Escape` aborts an incomplete prompt draft without terminating the running process.

Under the original ADR-0365 rule, striking `Escape` inside any of these interactive tools immediately evicted
the operator from terminal capture mode, returning focus to the outer console bay ladder. The `0x1b` byte was
never delivered to the PTY child process. Subsequent keystrokes intended for the terminal instead triggered
console-level bay grammars and single-key shortcuts, desynchronizing the live session and requiring manual mouse
clicks to re-engage terminal capture.

## Decision

**1. Supersede Section 6 of ADR-0365:**
- `Escape` **does not release terminal capture**.
- When the Prompt bay is in capture mode (`state.is_captured() == true`), `egui::Key::Escape` is intercepted
  and written directly to the child PTY's `stdin` as a raw escape byte:
  ```rust
  egui::Key::Escape => {
      let _ = session.send_bytes(b"\x1b");
  }
  ```

**2. Capture Mode Exit Exclusively via Navigation or Pointer:**
- Releasing terminal capture mode requires an explicit navigational act:
  - **Keyboard navigation**: Pressing `Tab` (or `Shift+Tab`) steps to the next or previous bay along the console focus
    ring (`View::tab()`), which explicitly clears capture via `self.prompt.set_captured(false)`.
  - **Pointer navigation**: Clicking any region outside the Prompt bay moves focus (`View::focus_bay()`), automatically
    relinquishing capture.
  - **Process exit**: If the underlying PTY child process exits, `PromptState::cleanup_if_exited()` resets selection
    and disengages capture.

**3. Comprehensive Keymap Bypass and Direct Control-Byte Forwarding:**
- While in capture mode, all console shortcuts, bay folding gestures (`Space`), and fader bindings are completely bypassed.
- Forward all essential POSIX terminal control signals and cursor escape sequences directly to PTY `stdin`:
  - **Control combinations**: `Ctrl+C` (`\x03`), `Ctrl+D` (`\x04`), `Ctrl+Z` (`\x1a`), `Ctrl+L` (`\x0c`),
    `Ctrl+U` (`\x15`), `Ctrl+W` (`\x17`), `Ctrl+A` (`\x01`), `Ctrl+E` (`\x05`), `Ctrl+R` (`\x12`), `Ctrl+K` (`\x0b`).
  - **Standard keys**: `Enter` (`\r`), `Backspace` (`\x7f`), `Escape` (`\x1b`).
  - **Cursor and paging sequences**: `ArrowUp` (`\x1b[A`), `ArrowDown` (`\x1b[B`), `ArrowRight` (`\x1b[C`),
    `ArrowLeft` (`\x1b[D`), `Home` (`\x1b[H`), `End` (`\x1b[F`), `PageUp` (`\x1b[5~`), `PageDown` (`\x1b[6~`),
    `Delete` (`\x1b[3~`).
  - **Text streaming**: Any UTF-8 string input from `egui::Event::Text` is forwarded verbatim.

## Alternatives rejected

- **Retaining `Escape` as the capture release key (ADR-0365 original)**: Unusable inside modern agent CLIs and
  curses applications; pressing `Escape` to cancel an agent prompt or leave vi insert mode disengages focus and breaks
  operator flow.
- **Requiring a double-tap `Escape` (`Esc Esc`) to release capture**: Introduces an artificial timeout window that delays
  single `Escape` transmission, breaking rapid escape sequences in CLI tools.
- **Reserving a dedicated modal release key combination (e.g. `Ctrl+]` or `Ctrl+Q`)**: Adds cognitive load and non-standard
  terminal key chords that conflict with existing multiplexers (tmux, screen) or PTY flow-control conventions.
