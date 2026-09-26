---
id: 0365
title: The Prompt bay embeds an interactive multi-session terminal multiplexer with cursor-anchored input
status: accepted
date: 2026-09-27
supersedes: []
superseded_by: []
principles: [0090, 0094]
tags: [console, bay, prompt, pty, terminal, agent, m9]
---

# The Prompt bay embeds an interactive multi-session terminal multiplexer with cursor-anchored input

## Context

Collaborative live performance between human operators and autonomous AI agents requires direct,
in-console execution, monitoring, and interaction with AI coding agents and LLM CLIs (such as
Google Antigravity, Anthropic Claude Code, Aider, Codex, and local Ollama runners).

Operating these agent tools outside the console in detached terminal windows disrupts live performance
focus, creates window management friction on stage, and prevents direct visual alignment between
agent commands and real-time visual output.

However, embedding a terminal inside a high-performance GPU visual console presents unique engineering challenges:
1. **Thread safety & UI responsiveness**: PTY child processes and I/O must never block egui render loops
   or GPU presentation passes.
2. **Interactive curses compatibility**: Modern agent CLIs use rich terminal formatting, 24-bit Truecolor,
   alternate screen buffers, and relative cursor positioning.
3. **Multilingual and multi-line drafting**: Operators need native OS IME support (e.g. Japanese Kanji/Kana)
   and multi-line composition with code pasting, which standard terminal character-at-a-time inputs handle poorly.
4. **Console focus hierarchy**: Keyboard navigation must cleanly separate console-level bay navigation
   from terminal character capture.

## Decision

**1. Left-Pane Prompt Bay Layout Integration:**
- Introduce `prompt` bay at the bottom of the left pane beneath Library and Staging.
- Retain full compliance with bay standards ([ADR-0343](0343-the-grammar-reaches-all-nine-bays-and-space-on-a-bay-is-the-fold.md)
  and [ADR-0364](0364-every-bay-head-renders-the-menu-dice-and-double-clicking-the-header-toggles-folding.md)):
  retained 27px header bar on fold, uniform 6-dot menu dice, double-click header folding, and `Space` keyboard folding.

**2. Detached Multi-Session PTY Multiplexer:**
- Spawn agent CLIs in background pseudo-terminals via `portable-pty` with per-session 1024-line scrollback buffers.
- Concurrency: Switching active CLI sessions keeps background processes alive without interruption.
- Lifecycle: Automatic child process exit detection cleans up handles and resets selection to unselected state.
- Custom fallback: When selecting custom CLI without an explicit command, default to `"powershell"` on Windows
  and `"sh"` on Unix/macOS, spawning an interactive shell immediately.

**3. Alphabetical CLI Presets & PATH Resolution:**
- Header selector pill provides a sorted dropdown menu of 18 AI CLI presets plus `custom...`.
- Available binaries on system `PATH` are dynamically resolved; unavailable commands are struck through
  and disabled; running sessions display active status badges (`●`).

**4. Cursor-Anchored Native Multiline Input with OS IME:**
- Anchor a borderless `egui::TextEdit::multiline` dynamically at the exact terminal cursor position `(cursor_x, cursor_y)`.
- Leverages OS native IME candidate positioning for Japanese and multilingual prompting.
- Supports multi-line prompt editing, line breaks, and snippet pasting before submission to PTY `stdin`.

**5. ANSI VT100 / xterm Emulation:**
- Comprehensive ANSI escape sequence parser supporting 24-bit Truecolor (`\x1b[38;2;...m`), 16 standard colors,
  alternate screen buffer switching, relative cursor motion, and cursor visibility toggles (`\x1b[?25h`/`l`).
- Raw line feed (`\n`) handling honors current horizontal cursor position without premature carriage returns,
  ensuring proper formatting of complex CLI banners.

**6. Focus Ladder & Capture Mode:**
- In terminal capture mode, all keyboard events are trapped and forwarded to PTY `stdin` (enabling single-key
  confirmations, arrow navigation, Ctrl+C, Ctrl+D).
- `Tab` navigates across bays; `Esc` releases capture back to console bay-level focus.

## Alternatives rejected

- **External window execution**: Requires window switching during live sets, risking missed musical cues.
- **Embedded Webview (xterm.js)**: Excessive memory footprint, complex process IPC, and potential conflicts with GPU presentation contexts.
- **Pure single-line input field**: Incompatible with interactive curses interfaces, full-screen TUIs, and rich multi-turn agent conversations.
