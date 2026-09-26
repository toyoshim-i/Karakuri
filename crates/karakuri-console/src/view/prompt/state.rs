//! State container for the Prompt bay.

use super::cli::{CliPreset, CliSelection};

/// State of the Prompt bay including active selection, dropdown menu, and custom input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptState {
    /// Currently selected CLI session.
    pub selection: CliSelection,
    /// Whether the CLI preset selection dropdown menu is open (Rule 2 modal).
    pub menu_open: bool,
    /// Text buffer when entering a custom command.
    pub custom_input: String,
    /// Whether custom command input field is active.
    pub custom_active: bool,
}

impl Default for PromptState {
    fn default() -> Self {
        Self {
            selection: CliSelection::Unselected,
            menu_open: false,
            custom_input: String::new(),
            custom_active: false,
        }
    }
}

impl PromptState {
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

    /// Selects a CLI selection and closes the menu.
    pub fn select(&mut self, selection: CliSelection) {
        self.selection = selection;
        self.menu_open = false;
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
}
