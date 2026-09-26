use std::fs;
use std::path::Path;
use winit::keyboard::Key;

use super::actions::*;
use super::*;
use karakuri_console::focus::ANY;

/// Canonical actions that can be bound to bay shortcuts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ActionId {
    TapBeat,
    ScaleGridHalve,
    ScaleGridDouble,
    Reset,
    Room,
    FoldEnclosing,
    UnfoldAll,
    Save,
    ToggleMute,
    ToggleSolo,
    ClearSolo,
}

impl ActionId {
    pub(crate) fn parse(name: &str) -> Option<Self> {
        match name {
            "tap_beat" | "tap" => Some(ActionId::TapBeat),
            "scale_grid_halve" | "halve_grid" => Some(ActionId::ScaleGridHalve),
            "scale_grid_double" | "double_grid" => Some(ActionId::ScaleGridDouble),
            "reset" | "reset_arrangement" => Some(ActionId::Reset),
            "room" | "room_theme" => Some(ActionId::Room),
            "fold_enclosing" | "fold_pane" => Some(ActionId::FoldEnclosing),
            "unfold_all" | "unfold" => Some(ActionId::UnfoldAll),
            "save" | "save_set" | "keep" => Some(ActionId::Save),
            "toggle_mute" | "mute" => Some(ActionId::ToggleMute),
            "toggle_solo" | "solo" => Some(ActionId::ToggleSolo),
            "clear_solo" => Some(ActionId::ClearSolo),
            _ => None,
        }
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            ActionId::TapBeat => "tap_beat",
            ActionId::ScaleGridHalve => "scale_grid_halve",
            ActionId::ScaleGridDouble => "scale_grid_double",
            ActionId::Reset => "reset",
            ActionId::Room => "room",
            ActionId::FoldEnclosing => "fold_enclosing",
            ActionId::UnfoldAll => "unfold_all",
            ActionId::Save => "save",
            ActionId::ToggleMute => "toggle_mute",
            ActionId::ToggleSolo => "toggle_solo",
            ActionId::ClearSolo => "clear_solo",
        }
    }

    pub(crate) fn for_control(
        id: karakuri_console::control::ControlId,
        title: Option<&str>,
    ) -> Option<Self> {
        if let Some(t) = title {
            match t {
                "Tap the beat" => return Some(ActionId::TapBeat),
                "Reset the arrangement" => return Some(ActionId::Reset),
                "Keep what a deck is playing" => return Some(ActionId::Save),
                "Fold a pane away" => return Some(ActionId::FoldEnclosing),
                "Bring back what is folded" => return Some(ActionId::UnfoldAll),
                "Mute a deck" => return Some(ActionId::ToggleMute),
                "Solo a deck" => return Some(ActionId::ToggleSolo),
                "Clear solo" => return Some(ActionId::ClearSolo),
                "Halve or double the grid" => return Some(ActionId::ScaleGridHalve),
                _ => {}
            }
        }
        match id {
            karakuri_console::control::ControlId::Tracker => Some(ActionId::TapBeat),
            karakuri_console::control::ControlId::Arrangement => Some(ActionId::Reset),
            karakuri_console::control::ControlId::InspectorPaneKeep => Some(ActionId::Save),
            karakuri_console::control::ControlId::ProgramSolo => Some(ActionId::ToggleSolo),
            _ => None,
        }
    }

    pub(crate) fn default_binding(self) -> KeyBinding {
        match self {
            ActionId::TapBeat => KeyBinding {
                key: BoundKey::Character("b"),
                legend: "b",
                bay: Some("transport"),
                title: Some("Tap the beat"),
                action: KeyAction::Handled(key_tap_beat),
                globalize: false,
            },
            ActionId::ScaleGridHalve => KeyBinding {
                key: BoundKey::Character(","),
                legend: ",",
                bay: Some("transport"),
                title: Some("Halve or double the grid"),
                action: KeyAction::Handled(key_scale_grid_halve),
                globalize: false,
            },
            ActionId::ScaleGridDouble => KeyBinding {
                key: BoundKey::Character("."),
                legend: ".",
                bay: Some("transport"),
                title: Some("Halve or double the grid"),
                action: KeyAction::Handled(key_scale_grid_double),
                globalize: false,
            },
            ActionId::Reset => KeyBinding {
                key: BoundKey::Character("r"),
                legend: "r",
                bay: Some("transport"),
                title: Some("Reset the arrangement"),
                action: KeyAction::Panel(key_reset),
                globalize: false,
            },
            ActionId::Room => KeyBinding {
                key: BoundKey::Character("n"),
                legend: "n",
                bay: Some("transport"),
                title: None,
                action: KeyAction::Handled(key_room),
                globalize: false,
            },
            ActionId::FoldEnclosing => KeyBinding {
                key: BoundKey::Character("g"),
                legend: "g",
                bay: Some(ANY),
                title: Some("Fold a pane away"),
                action: KeyAction::Panel(key_fold_enclosing),
                globalize: false,
            },
            ActionId::UnfoldAll => KeyBinding {
                key: BoundKey::Character("z"),
                legend: "z",
                bay: Some(ANY),
                title: Some("Bring back what is folded"),
                action: KeyAction::Panel(key_unfold_all),
                globalize: false,
            },
            ActionId::Save => KeyBinding {
                key: BoundKey::Character("k"),
                legend: "k",
                bay: Some("inspector"),
                title: Some("Keep what a deck is playing"),
                action: KeyAction::Handled(key_save),
                globalize: false,
            },
            ActionId::ToggleMute => KeyBinding {
                key: BoundKey::Character("m"),
                legend: "m",
                bay: Some("mixer"),
                title: Some("Mute a deck"),
                action: KeyAction::Handled(key_toggle_mute),
                globalize: false,
            },
            ActionId::ToggleSolo => KeyBinding {
                key: BoundKey::Character("s"),
                legend: "s",
                bay: Some("mixer"),
                title: Some("Solo a deck"),
                action: KeyAction::Handled(key_toggle_solo),
                globalize: false,
            },
            ActionId::ClearSolo => KeyBinding {
                key: BoundKey::Character("u"),
                legend: "u",
                bay: Some("mixer"),
                title: Some("Clear solo"),
                action: KeyAction::Handled(key_clear_solo),
                globalize: false,
            },
        }
    }
}

/// The runtime keymap holding immutable navigation bindings and customizable bay bindings.
#[derive(Debug, Clone)]
pub(crate) struct Keymap {
    bindings: Vec<KeyBinding>,
}

impl Keymap {
    /// Constructs a keymap with default bindings from [`KEY_BINDINGS`].
    pub(crate) fn default_keymap() -> Self {
        Self {
            bindings: KEY_BINDINGS.to_vec(),
        }
    }

    /// Loads custom keymap from `<store>/keymaps/default.keymap` if present,
    /// otherwise falls back to defaults. Prints warnings on collisions.
    pub(crate) fn load_or_default(store: &Path) -> Self {
        let path = store.join("keymaps/default.keymap");
        if path.is_file() {
            if let Ok(text) = fs::read_to_string(&path) {
                let (keymap, notes) = Self::parse(&text);
                for note in notes {
                    eprintln!("keymap: {note}");
                }
                return keymap;
            }
        }
        Self::default_keymap()
    }

    /// Parses keymap text lines formatted as `[<bay>:] <key> [global] -> <action>`.
    ///
    /// Fixed navigation keys (`tab`, `esc`) cannot be remapped.
    pub(crate) fn parse(text: &str) -> (Self, Vec<String>) {
        let mut keymap = Self::default_keymap();
        let mut notes = Vec::new();

        for (line_no, line) in text.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((lhs, rhs)) = line.split_once("->") else {
                notes.push(format!("line {}: missing '->'", line_no + 1));
                continue;
            };
            let action_name = rhs.trim();
            let Some(action_id) = ActionId::parse(action_name) else {
                notes.push(format!(
                    "line {}: unknown action '{}'",
                    line_no + 1,
                    action_name
                ));
                continue;
            };

            let lhs = lhs.trim();
            let (bay_spec, key_spec) = match lhs.split_once(':') {
                Some((b, k)) => (Some(b.trim()), k.trim()),
                None => (None, lhs),
            };

            let mut key_words = key_spec.split_whitespace();
            let Some(key_str) = key_words.next() else {
                notes.push(format!("line {}: missing key", line_no + 1));
                continue;
            };

            let mut globalize = false;
            for extra in key_words {
                if extra.eq_ignore_ascii_case("global") || extra.eq_ignore_ascii_case("globalize") {
                    globalize = true;
                }
            }

            // Fixed navigation grammar cannot be remapped
            if key_str == "tab" || key_str == "esc" || key_str == "escape" {
                notes.push(format!(
                    "line {}: navigation grammar key '{}' is fixed and cannot be remapped",
                    line_no + 1,
                    key_str
                ));
                continue;
            }

            let mut template = action_id.default_binding();
            if let Some(b) = bay_spec {
                if b == "any" {
                    template.bay = Some(ANY);
                } else {
                    template.bay = Some(Box::leak(b.to_string().into_boxed_str()));
                }
            }
            template.legend = Box::leak(key_str.to_string().into_boxed_str());
            template.key = BoundKey::Character(template.legend);
            template.globalize = globalize;

            // Replace existing binding for action or bay/key
            if let Some(pos) = keymap
                .bindings
                .iter()
                .position(|b| b.title == template.title && b.title.is_some())
            {
                keymap.bindings[pos] = template;
            } else {
                keymap.bindings.push(template);
            }
        }

        // Run collision detection and collect warnings
        let collisions = keymap.detect_collisions();
        notes.extend(collisions);

        (keymap, notes)
    }

    /// Detects key collisions across bindings.
    ///
    /// Permissive: Warnings are reported for operator awareness, but conflicting
    /// keys remain active and are dispatched according to priority rules.
    pub(crate) fn detect_collisions(&self) -> Vec<String> {
        let mut warnings = Vec::new();
        let len = self.bindings.len();

        for i in 0..len {
            let b1 = &self.bindings[i];
            for j in (i + 1)..len {
                let b2 = &self.bindings[j];
                if b1.key == b2.key {
                    let bay1 = b1.bay.unwrap_or("global");
                    let bay2 = b2.bay.unwrap_or("global");

                    if b1.globalize || b2.globalize {
                        warnings.push(format!(
                            "warning: key collision on '{}' between globalized {:?} and {:?}",
                            b1.legend, bay1, bay2
                        ));
                    } else if b1.bay == b2.bay || b1.bay == Some(ANY) || b2.bay == Some(ANY) {
                        warnings.push(format!(
                            "warning: key collision on '{}' in bay scope {:?}",
                            b1.legend, bay1
                        ));
                    }
                }
            }
        }

        warnings
    }

    /// Resolves the key binding matching focus in order: bay-specific, any-bay, globalized, and global.
    pub(crate) fn find_binding<'a>(
        &'a self,
        key: &Key<&str>,
        focused_bay: Option<&str>,
    ) -> Option<&'a KeyBinding> {
        // 1. Exact bay match
        if let Some(bay) = focused_bay {
            if let Some(b) = self
                .bindings
                .iter()
                .find(|b| b.key.matches(key) && b.bay == Some(bay))
            {
                return Some(b);
            }
        }

        // 2. ANY bay match
        if let Some(b) = self
            .bindings
            .iter()
            .find(|b| b.key.matches(key) && b.bay == Some(ANY))
        {
            return Some(b);
        }

        // 3. User explicitly globalized match
        if let Some(b) = self
            .bindings
            .iter()
            .find(|b| b.key.matches(key) && b.globalize)
        {
            return Some(b);
        }

        // 4. Fixed global (navigation: Tab, Esc)
        self.bindings
            .iter()
            .find(|b| b.key.matches(key) && b.bay.is_none())
    }

    pub(crate) fn find_binding_by_title(&self, title: &str) -> Option<&KeyBinding> {
        self.bindings.iter().find(|b| b.title == Some(title))
    }

    /// Dynamically binds a canonical action to a key and persists it to `<store>/keymaps/default.keymap`.
    pub(crate) fn bind_action(
        &mut self,
        store: &Path,
        action_id: ActionId,
        bay: Option<&str>,
        key_str: &str,
        globalize: bool,
    ) -> Result<Vec<String>, String> {
        if key_str == "tab" || key_str == "esc" || key_str == "escape" {
            return Err(format!(
                "navigation grammar key '{key_str}' is fixed and cannot be remapped"
            ));
        }

        let mut template = action_id.default_binding();
        if let Some(b) = bay {
            if b == "any" {
                template.bay = Some(ANY);
            } else {
                template.bay = Some(Box::leak(b.to_string().into_boxed_str()));
            }
        }
        template.legend = Box::leak(key_str.to_string().into_boxed_str());
        template.key = BoundKey::Character(template.legend);
        template.globalize = globalize;

        // Replace existing binding for action or add new
        if let Some(pos) = self
            .bindings
            .iter()
            .position(|b| b.title == template.title && b.title.is_some())
        {
            self.bindings[pos] = template;
        } else {
            self.bindings.push(template);
        }

        let collisions = self.detect_collisions();

        // Persist to store: <store>/keymaps/default.keymap
        let keymaps_dir = store.join("keymaps");
        if let Err(e) = fs::create_dir_all(&keymaps_dir) {
            return Err(format!("failed to create keymaps directory: {e}"));
        }
        let file_path = keymaps_dir.join("default.keymap");
        let existing = fs::read_to_string(&file_path).unwrap_or_default();

        let action_name = action_id.name();
        let mut lines: Vec<String> = existing
            .lines()
            .filter(|l| {
                let trimmed = l.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    return true;
                }
                if let Some((_, rhs)) = trimmed.split_once("->") {
                    if rhs.trim() == action_name {
                        return false;
                    }
                }
                true
            })
            .map(|l| l.to_string())
            .collect();

        let bay_part = match bay {
            Some(b) => format!("{b}: "),
            None => String::new(),
        };
        let glob_part = if globalize { " global" } else { "" };
        let new_line = format!("{bay_part}{key_str}{glob_part} -> {action_name}");
        lines.push(new_line);

        let mut content = lines.join("\n");
        if !content.ends_with('\n') {
            content.push('\n');
        }

        if let Err(e) = fs::write(&file_path, content) {
            return Err(format!("failed to write {}: {e}", file_path.display()));
        }

        Ok(collisions)
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn bindings(&self) -> &[KeyBinding] {
        &self.bindings
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_keymap_loads_key_bindings() {
        let keymap = Keymap::default_keymap();
        assert_eq!(keymap.bindings().len(), KEY_BINDINGS.len());
        assert!(keymap.detect_collisions().is_empty());
    }

    #[test]
    fn keymap_parses_custom_overrides_and_globalize() {
        let config = r#"
            # Custom mappings
            transport: t -> tap_beat
            mixer: m global -> toggle_mute
        "#;
        let (keymap, notes) = Keymap::parse(config);
        assert!(notes.is_empty(), "unexpected notes: {:?}", notes);

        // 't' matches tap_beat in transport
        let key_t = Key::Character("t");
        let b = keymap.find_binding(&key_t, Some("transport"));
        assert!(b.is_some());
        assert_eq!(b.unwrap().title, Some("Tap the beat"));

        // 't' is not active in mixer because not globalized
        assert!(keymap.find_binding(&key_t, Some("mixer")).is_none());

        // 'm' is globalized -> active in staging, transport, etc.
        let key_m = Key::Character("m");
        let b_mixer = keymap.find_binding(&key_m, Some("staging"));
        assert!(b_mixer.is_some());
        assert!(b_mixer.unwrap().globalize);
        assert_eq!(b_mixer.unwrap().title, Some("Mute a deck"));
    }

    #[test]
    fn keymap_detects_collision_when_globalized() {
        let config = r#"
            transport: m global -> tap_beat
            mixer: m -> toggle_mute
        "#;
        let (_keymap, notes) = Keymap::parse(config);
        assert!(
            notes.iter().any(|n| n.contains("key collision on 'm'")),
            "expected collision warning for 'm', got: {:?}",
            notes
        );
    }

    #[test]
    fn immutable_navigation_keys_cannot_be_remapped() {
        let config = "transport: tab -> tap_beat";
        let (_keymap, notes) = Keymap::parse(config);
        assert!(
            notes
                .iter()
                .any(|n| n.contains("navigation grammar key 'tab' is fixed")),
            "expected navigation refusal, got: {:?}",
            notes
        );
    }

    #[test]
    fn bind_action_persists_and_updates_runtime_binding() {
        let temp_dir =
            std::env::temp_dir().join(format!("karakuri_test_keymap_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);

        let mut keymap = Keymap::default_keymap();
        let res = keymap.bind_action(&temp_dir, ActionId::TapBeat, Some("transport"), "x", true);
        assert!(res.is_ok());

        // Check runtime binding updated
        let key_x = Key::Character("x");
        let b = keymap.find_binding(&key_x, Some("transport"));
        assert!(b.is_some());
        assert_eq!(b.unwrap().legend, "x");
        assert!(b.unwrap().globalize);

        // Check file was persisted
        let file_path = temp_dir.join("keymaps/default.keymap");
        assert!(file_path.is_file());
        let content = std::fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("transport: x global -> tap_beat"));

        // Clean up
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
