//! State container for the Prompt bay.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use super::cli::{CliPreset, CliSelection};
use super::session::{SessionManager, TerminalSession};

/// State of the Prompt bay including active selection, dropdown menu, session manager, and prompt input.
#[derive(Debug, Clone, Default)]
pub struct PromptState {
    /// Currently selected CLI session.
    pub selection: CliSelection,
    /// Whether the CLI preset selection dropdown menu is open (Rule 2 modal).
    pub menu_open: bool,
    /// Text buffer when entering a custom command in the selector.
    pub custom_input: String,
    /// Whether custom command input field is active in selector.
    pub custom_active: bool,
    /// Current input buffer in the interactive prompt bar.
    pub input_buffer: Arc<Mutex<String>>,
    /// Session manager managing running background processes.
    pub sessions: SessionManager,
    /// Whether Prompt bay is currently in input capture mode for the terminal.
    pub captured: Arc<AtomicBool>,
}

impl PartialEq for PromptState {
    fn eq(&self, other: &Self) -> bool {
        let b1 = self
            .input_buffer
            .lock()
            .map(|s| s.clone())
            .unwrap_or_default();
        let b2 = other
            .input_buffer
            .lock()
            .map(|s| s.clone())
            .unwrap_or_default();
        self.selection == other.selection
            && self.menu_open == other.menu_open
            && self.custom_input == other.custom_input
            && self.custom_active == other.custom_active
            && self.sessions == other.sessions
            && self.is_captured() == other.is_captured()
            && b1 == b2
    }
}

impl Eq for PromptState {}

impl PromptState {
    /// Returns whether the Prompt bay is currently capturing keyboard input.
    pub fn is_captured(&self) -> bool {
        self.captured.load(Ordering::Relaxed)
    }

    /// Sets whether the Prompt bay is capturing keyboard input.
    pub fn set_captured(&self, val: bool) {
        self.captured.store(val, Ordering::Relaxed);
    }
    /// Creates a default, unselected Prompt bay state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Toggles the dropdown menu visibility.
    pub fn toggle_menu(&mut self) {
        self.menu_open = !self.menu_open;
    }

    /// Opens the dropdown menu.
    pub fn open_menu(&mut self) {
        self.menu_open = true;
    }

    /// Shuts the dropdown menu.
    pub fn shut_menu(&mut self) {
        self.menu_open = false;
    }

    /// Selects a CLI selection and closes the menu, spawning or resuming the session.
    pub fn select(&mut self, selection: CliSelection) {
        self.selection = selection;
        self.menu_open = false;
        if !self.selection.is_unselected() {
            self.sessions.get_or_spawn(&self.selection);
        }
    }

    /// Selects a standard CLI preset.
    pub fn select_preset(&mut self, preset: CliPreset) {
        self.select(CliSelection::Preset(preset));
    }

    /// Selects a custom command.
    pub fn select_custom(&mut self, cmd: String) {
        self.select(CliSelection::Custom(cmd));
    }

    /// Clears the selection back to unselected.
    pub fn reset_selection(&mut self) {
        self.selection = CliSelection::Unselected;
    }

    /// Returns the active terminal session if a CLI is selected.
    pub fn active_session(&self) -> Option<Arc<TerminalSession>> {
        self.sessions.get_or_spawn(&self.selection)
    }

    /// Returns a copy of the current input buffer content.
    pub fn input(&self) -> String {
        self.input_buffer
            .lock()
            .map(|s| s.clone())
            .unwrap_or_default()
    }

    /// Sets the input buffer content.
    pub fn set_input(&self, val: &str) {
        if let Ok(mut buf) = self.input_buffer.lock() {
            *buf = val.to_string();
        }
    }

    /// Submits the current input buffer into the active session stdin.
    pub fn submit_input(&self) -> std::io::Result<()> {
        let input = {
            let mut lock = self
                .input_buffer
                .lock()
                .map_err(|_| std::io::Error::other("input buffer lock poisoned"))?;
            std::mem::take(&mut *lock)
        };
        if let Some(session) = self.active_session() {
            session.send_line(&input)?;
        }
        Ok(())
    }
}
