use super::*;

pub(crate) const KEY_BINDINGS: &[KeyBinding] = &[
    // Cycles focus across bays forward (`Tab`) or backward (`Shift-Tab`) (ADR-0156, ADR-0259).
    // Focus navigation is local to the console and bypasses egui consumption.
    KeyBinding {
        key: BoundKey::Named(NamedKey::Tab),
        legend: "tab",
        bay: None,
        title: None,
        action: KeyAction::Focus(key_tab),
        globalize: false,
    },
    // Ascends one level of the focused bay's hierarchy without quitting the app (ADR-0259, P-0094).
    // While typing in an input field, cancels or abandons the entry.
    KeyBinding {
        key: BoundKey::Named(NamedKey::Escape),
        legend: "esc",
        bay: None,
        title: None,
        action: KeyAction::Focus(key_escape),
        globalize: false,
    },
    // Folds the split pane enclosing the focused bay, reachable from keyboard alone (ADR-0259, ADR-0343).
    KeyBinding {
        key: BoundKey::Character("g"),
        legend: "g",
        bay: Some(focus::ANY),
        title: Some("Fold a pane away"),
        action: KeyAction::Panel(key_fold_enclosing),
        globalize: false,
    },
    KeyBinding {
        key: BoundKey::Character("z"),
        legend: "z",
        bay: Some(focus::ANY),
        title: Some("Bring back what is folded"),
        action: KeyAction::Panel(key_unfold_all),
        globalize: false,
    },
    KeyBinding {
        key: BoundKey::Character("r"),
        legend: "r",
        bay: Some("transport"),
        title: Some("Reset the arrangement"),
        action: KeyAction::Panel(key_reset),
        globalize: false,
    },
    // Keeps current material of the selected deck in library storage under a timestamp.
    KeyBinding {
        key: BoundKey::Character("k"),
        legend: "k",
        bay: Some("inspector"),
        title: Some("Keep what a deck is playing"),
        action: KeyAction::Handled(key_save),
        globalize: false,
    },
    // **The beat, tapped.** The one key on this panel that reaches the room
    // rather than the deck or the arrangement, and the first of three that
    // need an input open. What it does and why it does not go through
    // `written` is [`tapped`].
    KeyBinding {
        key: BoundKey::Character("b"),
        legend: "b",
        bay: Some("transport"),
        title: Some("Tap the beat"),
        action: KeyAction::Handled(key_tap_beat),
        globalize: false,
    },
    // **The grid, an octave either way**, and the two keys the page
    // specifies for it. Refused where the result would leave the trackable
    // range, which is the beat lock's call — see [`scaled`].
    KeyBinding {
        key: BoundKey::Character(","),
        legend: ",",
        bay: Some("transport"),
        title: Some("Halve or double the grid"),
        action: KeyAction::Handled(key_scale_grid_halve),
        globalize: false,
    },
    KeyBinding {
        key: BoundKey::Character("."),
        legend: ".",
        bay: Some("transport"),
        title: Some("Halve or double the grid"),
        action: KeyAction::Handled(key_scale_grid_double),
        globalize: false,
    },
    // Toggles the console visual theme (Room) without altering arrangement or deck state.
    KeyBinding {
        key: BoundKey::Character("n"),
        legend: "n",
        bay: Some("transport"),
        title: None,
        action: KeyAction::Handled(key_room),
        globalize: false,
    },
    KeyBinding {
        key: BoundKey::Character("m"),
        legend: "m",
        bay: Some("mixer"),
        title: Some("Mute a deck"),
        action: KeyAction::Handled(key_toggle_mute),
        globalize: false,
    },
    KeyBinding {
        key: BoundKey::Character("s"),
        legend: "s",
        bay: Some("mixer"),
        title: Some("Solo a deck"),
        action: KeyAction::Handled(key_toggle_solo),
        globalize: false,
    },
    KeyBinding {
        key: BoundKey::Character("u"),
        legend: "u",
        bay: Some("mixer"),
        title: Some("Clear solo"),
        action: KeyAction::Handled(key_clear_solo),
        globalize: false,
    },
];

// Free action functions taking [`KeyCtx`] to avoid reborrow conflicts with `&mut Gfx`.
