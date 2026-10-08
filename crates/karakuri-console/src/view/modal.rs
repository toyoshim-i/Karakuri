//! Modal overlay states and Rule 2 mutual exclusion.
//!
//! While an overlay is active, it takes precedence, clicks outside dismiss it,
//! and background controls are suppressed (Rule 2).

use super::View;

/// A strongly-typed descriptor of the currently active modal overlay card or chooser.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModalOverlay {
    /// Transport arrangement presets popup card.
    Arrangement,
    /// Transport audio input device selector popup card.
    AudioIn,
    /// Inspector inline deck name editing mode on pane `index`.
    NamingSet(usize),
    /// Library bay target deck selection popup card.
    LibraryTargetDeck,
    /// Library row context menu.
    LibraryRowMenu,
    /// Inspector node wiring / uses connections popup card `(pane, node, slot)`.
    InspectorWiring(usize, usize, usize),
    /// Inspector pane target deck selection popup card on pane `index`.
    InspectorPaneTarget(usize),
    /// Sequencer lane add chooser card.
    SequencerLaneChooser,
    /// Master chain add operation chooser card.
    MasterChainAddChooser,
    /// Prompt bay agent CLI selection dropdown menu.
    PromptCliMenu,
    /// Transport theme selection dropdown menu.
    ThemeMenu,
    /// Transport MCP server configuration dropdown menu.
    McpServerMenu,
    /// Program bay resolution selection dropdown menu.
    ResolutionMenu,
    /// Outputs row WebXR mode selection dropdown menu.
    XrModeMenu,
}

impl ModalOverlay {
    /// Human-readable name for diagnostics and logging.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Arrangement => "Arrangement",
            Self::AudioIn => "AudioIn",
            Self::NamingSet(_) => "NamingSet",
            Self::LibraryTargetDeck => "LibraryTargetDeck",
            Self::LibraryRowMenu => "LibraryRowMenu",
            Self::InspectorWiring(..) => "InspectorWiring",
            Self::InspectorPaneTarget(_) => "InspectorPaneTarget",
            Self::SequencerLaneChooser => "SequencerLaneChooser",
            Self::MasterChainAddChooser => "MasterChainAddChooser",
            Self::PromptCliMenu => "PromptCliMenu",
            Self::ThemeMenu => "ThemeMenu",
            Self::McpServerMenu => "McpServerMenu",
            Self::ResolutionMenu => "ResolutionMenu",
            Self::XrModeMenu => "XrModeMenu",
        }
    }
}

impl View {
    /// Returns the currently open modal overlay, evaluated in Rule 2 priority order.
    pub fn active_overlay(&self) -> Option<ModalOverlay> {
        if self.arrangement.open() {
            return Some(ModalOverlay::Arrangement);
        }
        if self.audio.as_ref().is_some_and(|audio| audio.open()) {
            return Some(ModalOverlay::AudioIn);
        }
        if let Some(naming) = self.naming_set() {
            return Some(ModalOverlay::NamingSet(naming.pane));
        }
        if self.target_open() {
            return Some(ModalOverlay::LibraryTargetDeck);
        }
        if let Some((pane, node, slot)) = self.wiring_open() {
            return Some(ModalOverlay::InspectorWiring(pane, node, slot));
        }
        if let Some(index) = self.pane_target_open() {
            return Some(ModalOverlay::InspectorPaneTarget(index));
        }
        if self.menu_open() {
            return Some(ModalOverlay::LibraryRowMenu);
        }
        if self.lane_open() {
            return Some(ModalOverlay::SequencerLaneChooser);
        }
        if self.chain_add_open() {
            return Some(ModalOverlay::MasterChainAddChooser);
        }
        if self.prompt.menu_open {
            return Some(ModalOverlay::PromptCliMenu);
        }
        if self.theme_menu_open {
            return Some(ModalOverlay::ThemeMenu);
        }
        if self.mcp_server.open() {
            return Some(ModalOverlay::McpServerMenu);
        }
        if self.resolution_menu_open {
            return Some(ModalOverlay::ResolutionMenu);
        }
        if self.xr_menu_open {
            return Some(ModalOverlay::XrModeMenu);
        }
        None
    }

    /// Whether any modal card, menu, or chooser is currently open.
    pub fn has_modal_overlay(&self) -> bool {
        self.active_overlay().is_some()
    }

    /// Dismisses the currently active modal overlay, returning `true` if an overlay was dismissed.
    pub fn dismiss_modal_overlay(&mut self) -> bool {
        let Some(overlay) = self.active_overlay() else {
            return false;
        };
        match overlay {
            ModalOverlay::Arrangement => {
                self.arrangement.shut();
                true
            }
            ModalOverlay::AudioIn => {
                if let Some(audio) = self.audio.as_mut() {
                    audio.shut();
                }
                true
            }
            ModalOverlay::NamingSet(_) => {
                self.stop_naming_set();
                true
            }
            ModalOverlay::LibraryTargetDeck => self.shut_target(),
            ModalOverlay::LibraryRowMenu => self.shut_menu(),
            ModalOverlay::InspectorWiring(..) => self.shut_wiring(),
            ModalOverlay::InspectorPaneTarget(_) => self.shut_pane_target(),
            ModalOverlay::SequencerLaneChooser => self.shut_lane(),
            ModalOverlay::MasterChainAddChooser => self.shut_chain_add(),
            ModalOverlay::PromptCliMenu => {
                self.prompt.shut_menu();
                true
            }
            ModalOverlay::ThemeMenu => {
                self.theme_menu_open = false;
                true
            }
            ModalOverlay::McpServerMenu => {
                self.mcp_server.shut();
                true
            }
            ModalOverlay::ResolutionMenu => {
                self.resolution_menu_open = false;
                true
            }
            ModalOverlay::XrModeMenu => {
                self.xr_menu_open = false;
                true
            }
        }
    }

    /// Returns which text input mode is currently active, if any.
    pub fn active_text_input(&self) -> Option<TextInputKind> {
        if self.arrangement.naming().is_some() {
            Some(TextInputKind::Arrangement)
        } else if self.mcp_server.editing().is_some() {
            Some(TextInputKind::McpServerAddr)
        } else {
            self.naming_set()
                .map(|naming| TextInputKind::DeckName(naming.pane))
        }
    }

    /// Types characters into the active text input session, returning true if the buffer changed.
    pub fn type_into_active(&mut self, text: &str) -> bool {
        match self.active_text_input() {
            Some(TextInputKind::Arrangement) => {
                let mut moved = false;
                for c in text.chars() {
                    moved |= self.arrangement.typed(c);
                }
                moved
            }
            Some(TextInputKind::McpServerAddr) => {
                let mut moved = false;
                for c in text.chars() {
                    moved |= self.mcp_server.typed(c);
                }
                moved
            }
            Some(TextInputKind::DeckName(_)) => {
                let mut moved = false;
                for c in text.chars() {
                    moved |= self.type_into_name(c);
                }
                moved
            }
            None => false,
        }
    }

    /// Rubs out the trailing character from the active text input buffer.
    pub fn rub_out_active(&mut self) -> bool {
        match self.active_text_input() {
            Some(TextInputKind::Arrangement) => self.arrangement.rubbed_out(),
            Some(TextInputKind::McpServerAddr) => self.mcp_server.rubbed_out(),
            Some(TextInputKind::DeckName(_)) => self.rub_out_of_name(),
            None => false,
        }
    }

    /// Cancels and shuts the active text input session.
    pub fn cancel_active_text_input(&mut self) -> bool {
        match self.active_text_input() {
            Some(TextInputKind::Arrangement) => {
                self.arrangement.shut();
                true
            }
            Some(TextInputKind::McpServerAddr) => {
                self.mcp_server.shut();
                true
            }
            Some(TextInputKind::DeckName(_)) => {
                self.stop_naming_set();
                true
            }
            None => false,
        }
    }
}

/// An active inline text input session on the console.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextInputKind {
    /// Transport arrangement save naming mode.
    Arrangement,
    /// Transport MCP server address:port editing mode.
    McpServerAddr,
    /// Inspector inline deck name editing mode on pane `index`.
    DeckName(usize),
}
