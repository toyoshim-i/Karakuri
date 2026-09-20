use super::*;

pub(crate) const KEY_BINDINGS: &[KeyBinding] = &[
    // **`Tab` moves focus to the next bay, and `shift-Tab` to the one
    // before** — ADR-0259's first key, and the whole of what makes the six
    // that follow it addressable.
    //
    // **The walk is the console's and the key is this file's**, which is the
    // seam every control on this panel already crosses (ADR-0156): the ring
    // is derived from the arrangement by `karakuri_console::focus::ring`,
    // and what `key_tab` knows is which direction was asked for.
    //
    // **It names no operation and asks for no record**, which is the two
    // library cursor keys' arrangement one bay out: focus is a pointer this
    // console owns, nothing downstream can be the model of record for it,
    // and `Change::Pointed` is the answer for *a key moved a pointer*. The
    // page says the same thing by leaving this row's key column alone —
    // there is no *move focus* row, because moving focus is not an
    // operation. Hence `title: None`.
    //
    // **`egui` never gets a say.** `egui-winit` 0.36.1 reports `consumed`
    // for every `Tab` whatever has focus, and `App::to_egui` destructures
    // `EventResponse` down to `repaint` and nothing else — see that
    // function's own doc for why that shape is what makes this key
    // reachable at all.
    KeyBinding {
        key: BoundKey::Named(NamedKey::Tab),
        legend: "tab",
        bay: None,
        title: None,
        action: KeyAction::Focus(key_tab),
    },
    // **`esc` goes up one level of the focused bay's address, and it does
    // not quit** (ADR-0259). Quitting follows the platform's own
    // accelerator — `⌘Q`, `Alt-F4` — which arrives as
    // `WindowEvent::CloseRequested` and is answered at the top of
    // `window_event`, saves waited for and recording flushed. **A quit
    // ladder is a sequence that ends in something irreversible, in front of
    // an audience, reached by repeating one key**
    // ([P-0094](../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)).
    //
    // **At bay level it acts on nothing and says so**, because there is no
    // unfocused state to fall out into and a key that declines silently is
    // indistinguishable from one that is not bound.
    //
    // **While a name is being typed it abandons the name**, and that is not
    // an exception — both letter-taking flows return before this `match` is
    // ever reached, and ADR-0259 reads them as a *field*: *"`esc` … from a
    // field it abandons the name"*.
    KeyBinding {
        key: BoundKey::Named(NamedKey::Escape),
        legend: "esc",
        bay: None,
        title: None,
        action: KeyAction::Focus(key_escape),
    },
    // **The one letter left that names a region, and it takes it from the
    // focus rather than from the pointer** (ADR-0259, ADR-0343). `g` folds
    // the split enclosing the focused bay, which is a pane every time — a
    // bay's parent is a split and never another bay — so this is the one
    // route to *Fold a pane away* from the keyboard alone, and it is the row
    // `space` on a bay does not reach. **Rule 01 is what it buys** — *every
    // operation is reachable from the keyboard alone* — because a key whose
    // region came from the pointer was not.
    KeyBinding {
        key: BoundKey::Character("g"),
        legend: "g",
        bay: Some(focus::ANY),
        title: Some("Fold a pane away"),
        action: KeyAction::Panel(key_fold_enclosing),
    },
    KeyBinding {
        key: BoundKey::Character("z"),
        legend: "z",
        bay: None,
        title: Some("Bring back what is folded"),
        action: KeyAction::Panel(key_unfold_all),
    },
    KeyBinding {
        key: BoundKey::Character("r"),
        legend: "r",
        bay: None,
        title: Some("Reset the arrangement"),
        action: KeyAction::Panel(key_reset),
    },
    // **Keep what the selected deck is playing**, filed under a stamp
    // because a bare key press cannot type a name — see
    // `karakuri_environment::accepted_save`, whose convention that is and
    // whose reason it borrows: an operator looks for the time they saved it.
    //
    // **The selected deck and not a slot in the key**, which is the split
    // every deck-addressed control on this panel makes: the deck an
    // operator means is the one they have already addressed in the Mixer,
    // and a model has no selection and names the slot in the call.
    //
    // **The panel column of this row is still `plan`.** A key is not a
    // control, the Library bay has no *keep* pill drawn, and a badge that
    // said otherwise would be a claim about a control that is not there.
    KeyBinding {
        key: BoundKey::Character("k"),
        legend: "k",
        bay: None,
        title: Some("Keep what a deck is playing"),
        action: KeyAction::Handled(key_save),
    },
    // **The beat, tapped.** The one key on this panel that reaches the room
    // rather than the deck or the arrangement, and the first of three that
    // need an input open. What it does and why it does not go through
    // `written` is [`tapped`].
    KeyBinding {
        key: BoundKey::Character("b"),
        legend: "b",
        bay: None,
        title: Some("Tap the beat"),
        action: KeyAction::Handled(key_tap_beat),
    },
    // **The grid, an octave either way**, and the two keys the page
    // specifies for it. Refused where the result would leave the trackable
    // range, which is the beat lock's call — see [`scaled`].
    KeyBinding {
        key: BoundKey::Character(","),
        legend: ",",
        bay: None,
        title: Some("Halve or double the grid"),
        action: KeyAction::Handled(key_scale_grid_halve),
    },
    KeyBinding {
        key: BoundKey::Character("."),
        legend: ".",
        bay: None,
        title: Some("Halve or double the grid"),
        action: KeyAction::Handled(key_scale_grid_double),
    },
    // **The key that changes the screen without touching the pointer and
    // without touching the model.** The room is the view's: every colour on
    // the panel changes and nothing in the arrangement moves, so no
    // `Outcome` says so and `Change::Room` is the only thing that does —
    // hence `title: None`, the same as `Tab` and `Escape`.
    KeyBinding {
        key: BoundKey::Character("n"),
        legend: "n",
        bay: None,
        title: None,
        action: KeyAction::Handled(key_room),
    },
    KeyBinding {
        key: BoundKey::Character("m"),
        legend: "m",
        bay: Some("mixer"),
        title: Some("Toggle mute"),
        action: KeyAction::Handled(key_toggle_mute),
    },
    KeyBinding {
        key: BoundKey::Character("s"),
        legend: "s",
        bay: Some("mixer"),
        title: Some("Toggle solo"),
        action: KeyAction::Handled(key_toggle_solo),
    },
    KeyBinding {
        key: BoundKey::Character("u"),
        legend: "u",
        bay: Some("mixer"),
        title: Some("Clear solo"),
        action: KeyAction::Handled(key_clear_solo),
    },
];

// -- KEY_BINDINGS' actions, one free function per entry ---------------------
//
// Free functions and not methods, and each takes a [`KeyCtx`] rather than
// `&mut App`, for the reason [`KeyCtx`] itself gives: `window_event` is
// already holding a `&mut Gfx` reborrowed out of `self.gfx` by the time one
// of these is called. Each body is exactly what the arm it replaced had —
// see [`KEY_BINDINGS`] for the doc comment that used to sit on the arm
// itself.
