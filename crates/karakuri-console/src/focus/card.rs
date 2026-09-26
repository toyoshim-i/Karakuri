use super::*;

/// Specification for modal cards descended from controls (ADR-0350, ADR-0351, ADR-0352).
///
/// Defines behavior for audio inputs, arrangement menu, lane chooser, and master effect chooser.
/// Cards open with `enter`, walk vertically via digits/arrows (clamped), execute via `enter`, and close via `esc`/`Tab`.
pub(crate) struct Card {
    /// The control the card hangs from, and what [`card_of`] finds it by.
    pub(crate) control: Control,
    /// Whether the card is down.
    pub(crate) down: fn(&View) -> bool,
    /// How many rows it is drawing, which is zero while it is up.
    pub(crate) rows: fn(&View) -> usize,
    /// How `enter` on the control puts it down, and what takes it away.
    pub(crate) puts: Puts,
    /// Sentence returned when attempting to focus a closed card's row.
    pub(crate) shut: &'static str,
    /// Sentence returned when pressing enter on an already-open card control.
    pub(crate) standing: &'static str,
    /// Sentence returned when an invalid row digit is entered.
    pub(crate) short: &'static str,
    /// Sentence returned when navigating an empty card.
    pub(crate) empty: &'static str,
    /// Sentence returned when horizontal navigation is attempted on vertical card.
    pub(crate) column: &'static str,
    /// Sentence returned when navigating past card boundaries.
    pub(crate) end: &'static str,
    /// Dynamic refusal sentence when card is in a special busy mode (e.g. naming prompt).
    pub(crate) busy: Option<fn(&View) -> Option<&'static str>>,
}

/// How a card is put down and taken away.
#[derive(Clone, Copy)]
pub(crate) enum Puts {
    /// Local console modal card state.
    Console {
        put: fn(&mut View) -> bool,
        shut: fn(&mut View) -> bool,
        nothing: &'static str,
    },
    /// Host-managed card state (ADR-0156).
    Host {
        asked: fn() -> Asked,
        shut: fn(&mut View),
    },
}

impl Card {
    /// Returns refusal reason for a non-existent digit row.
    pub(crate) fn named(&self, view: &View) -> &'static str {
        match (self.down)(view) {
            false => self.shut,
            true => self.asking(view).unwrap_or(self.short),
        }
    }

    /// Returns refusal reason when navigating an empty card.
    pub(crate) fn bare(&self, view: &View) -> &'static str {
        self.asking(view).unwrap_or(self.empty)
    }

    /// Returns busy state sentence if active.
    pub(crate) fn asking(&self, view: &View) -> Option<&'static str> {
        self.busy.and_then(|state| state(view))
    }

    /// Closes the card.
    pub(crate) fn shut(&self, view: &mut View) {
        match self.puts {
            Puts::Console { shut, .. } => {
                shut(view);
            }
            Puts::Host { shut, .. } => shut(view),
        }
    }
}

/// Refusal sentence when focusing a closed Transport card's row.
const CARD_SHUT: &str = "this card is not down, so there is nothing here to name — press enter on \
                         this control to put it down";

/// Refusal sentence when digit exceeds Transport card row count.
const CARD_SHORT: &str =
    "this card is not drawing a row with that number — the digits count what is on it, from one";

/// Refusal sentence on pressing `enter` on an already-open Transport card.
const CARD_STANDING: &str = "this card is already down — press a digit to name a row, or walk it \
                             with up and down, and enter runs the row the address is on";

/// Refusal sentence while arrangement menu is awaiting name input (ADR-0259).
const NAMING: &str = "this menu is asking for a name — type it and press return, or press esc to \
                      leave the arrangement unsaved";

/// Refusal sentence when digit exceeds procedure chooser count.
pub(crate) const CHAIN_CARD_SHORT: &str =
    "the chooser is not offering a procedure with that number — the \
                                digits count what is on the card, from one";

/// The sentence an arrow carries on a chooser that is down with nothing on it.
pub(crate) const NOTHING_TO_WALK: &str = "the chooser is offering nothing to walk";

/// The four cards, one row each.
pub(crate) const CARDS: &[Card] = &[
    Card {
        control: Control::Audio,
        down: inputs_down,
        rows: inputs_rows,
        puts: Puts::Host {
            asked: open_inputs,
            shut: shut_inputs,
        },
        shut: CARD_SHUT,
        standing: CARD_STANDING,
        short: CARD_SHORT,
        empty: CARD_SHORT,
        column: "a card's rows are a column, so up and down walk them",
        end: "the address is already at the end of this card",
        busy: None,
    },
    Card {
        control: Control::Arrangement,
        down: menu_down,
        rows: menu_rows,
        puts: Puts::Host {
            asked: open_menu,
            shut: shut_menu,
        },
        shut: CARD_SHUT,
        standing: CARD_STANDING,
        short: CARD_SHORT,
        empty: CARD_SHORT,
        column: "a card's rows are a column, so up and down walk them",
        end: "the address is already at the end of this card",
        busy: Some(menu_asking),
    },
    Card {
        control: Control::AddLane,
        down: View::lane_open,
        rows: lane_rows,
        puts: Puts::Console {
            put: View::open_lane,
            shut: View::shut_lane,
            nothing: "there is nothing for a lane to drive — this console draws no mixer strip \
                      and the inspector is showing no published control",
        },
        shut: "the chooser is not down, so there is nothing here to name — press enter on + lane \
               to put it down",
        standing: "the chooser is already down — press a digit to name a target, or walk it with \
                   up and down, and enter points the lane at the one the address is on",
        short: "the chooser is not offering that many targets — the digits count what is on the \
                card, from one",
        empty: NOTHING_TO_WALK,
        column: "the chooser's targets are a column, so up and down walk them",
        end: "the address is already at the end of the chooser",
        busy: None,
    },
    Card {
        control: Control::AddEffect,
        down: View::chain_add_open,
        rows: chain_rows,
        puts: Puts::Console {
            put: View::open_chain_add,
            shut: View::shut_chain_add,
            nothing: "this library is listing no kind L5 procedure, so there is nothing to add \
                      — the master chain holds kind L5 procedures",
        },
        shut: "the chooser is not down, so there is nothing here to name — press enter on + add \
               to put it down",
        standing: "this chooser is already down — press a digit to name a procedure, or walk it \
                   with up and down, and enter adds the one the address is on",
        short: CHAIN_CARD_SHORT,
        empty: NOTHING_TO_WALK,
        column: "the chooser's procedures are a column, so up and down walk them",
        end: "the address is already at the end of the chooser",
        busy: None,
    },
];

/// Whether the audio-in pill's card is down.
fn inputs_down(view: &View) -> bool {
    view.audio.as_ref().is_some_and(crate::view::AudioIn::open)
}

/// How many inputs that card is listing.
fn inputs_rows(view: &View) -> usize {
    view.audio.as_ref().map_or(0, crate::view::AudioIn::rows)
}

/// The audio-in card, in the pill's own word for putting it down.
fn open_inputs() -> Asked {
    Asked::Listened(AudioAsk::Open)
}

/// Take the audio-in card away.
fn shut_inputs(view: &mut View) {
    if let Some(audio) = view.audio.as_mut() {
        audio.shut();
    }
}

/// Whether the arrangement pill's menu is down, a name being asked for included.
fn menu_down(view: &View) -> bool {
    view.arrangement.open()
}

/// How many rows that menu is drawing.
fn menu_rows(view: &View) -> usize {
    view.arrangement.rows()
}

/// The menu asking for a name, which is the state that draws no rows.
fn menu_asking(view: &View) -> Option<&'static str> {
    view.arrangement.naming().map(|_| NAMING)
}

/// The arrangement menu, in the pill's own word for putting it down.
fn open_menu() -> Asked {
    Asked::Arranged(Ask::Open)
}

/// Take the arrangement menu away, typed name and all.
fn shut_menu(view: &mut View) {
    view.arrangement.shut();
}

/// How many targets the `+ lane` chooser is offering, and none while it is up.
fn lane_rows(view: &View) -> usize {
    match view.lane_open() {
        true => view.lane_choices().items.len(),
        false => 0,
    }
}

/// How many procedures the `+ add` chooser is offering, and none while it is up.
fn chain_rows(view: &View) -> usize {
    match view.chain_add_open() {
        true => view.chain_choices().items.len(),
        false => 0,
    }
}

/// The card `control` hangs, or `None` for a control that hangs none — the one
/// reading the digit, the arrows and `enter` all ask.
pub(crate) fn card_of(control: Control) -> Option<&'static Card> {
    CARDS.iter().find(|card| card.control == control)
}

/// How many rows the card `control` hangs is drawing, and zero for a control that
/// hangs none — the reading [`drawn`] takes at a card's rung.
pub(crate) fn card_rows(view: &View, control: Control) -> usize {
    card_of(control).map_or(0, |card| (card.rows)(view))
}

/// Returns the card on the focused bay's address and whether it has descended, or `None` (ADR-0332).
///
/// Evaluates whether `esc` should dismiss an active card on the current address path. Requires solved `panel`.
pub fn card_on_path(view: &View, panel: &Panel) -> Option<(Control, bool)> {
    let bay = view.focused(panel)?;
    let built = built(bay.name)?;
    let at = view.focus().address(bay.name).map(Address::at)?;
    // A card hangs off a head control or off a headless row's own control, so
    // the control is the second step of the path or the first, and the address
    // is inside the card where there is one step after it.
    let (control, inside) = match *at {
        [HEAD, through] => (head_control(built, through)?, false),
        [HEAD, through, _] => (head_control(built, through)?, true),
        [through] if matches!(built.items, Items::Controls(_)) => {
            (row_control(view, built, through)?, false)
        }
        [through, _] if matches!(built.items, Items::Controls(_)) => {
            (row_control(view, built, through)?, true)
        }
        _ => return None,
    };
    let card = card_of(control)?;
    (card.down)(view).then_some((control, inside))
}

/// The `through`th control of `built`'s head, counting from one.
fn head_control(built: &Built, through: usize) -> Option<Control> {
    built.head.get(through.checked_sub(1)?).copied()
}

/// The `through`th control of a headless row, counting from one.
fn row_control(view: &View, built: &Built, through: usize) -> Option<Control> {
    built.item().nth(through, drawn(view, built, &[]))
}

/// Take the card `control` hangs away, and do nothing for a control that hangs
/// none. A card that is already up is left as it is.
pub fn shut_card(view: &mut View, control: Control) {
    if let Some(card) = card_of(control) {
        card.shut(view);
    }
}

/// Take every card away, wherever the address is — the reading `Tab` takes,
/// because a move of focus is the address leaving every card it could be in.
pub fn shut_cards(view: &mut View) {
    for card in CARDS {
        card.shut(view);
    }
}

/// The sentence a digit carries below a control nothing is drawn under.
pub const NOTHING_BELOW: &str = "nothing below this control is drawn, so a digit here reaches \
                             nothing — esc goes back up";

/// Opens the card hanging from the focused control on `enter`.
///
/// Declines if already open or empty; the address remains on the parent control until a row is chosen.
pub fn open_card(view: &mut View, control: Control) -> Asked {
    let Some(card) = card_of(control) else {
        return Asked::Nothing("this control puts no card down");
    };
    if (card.down)(view) {
        return Asked::Nothing(card.standing);
    }
    match card.puts {
        Puts::Console { put, nothing, .. } => match put(view) {
            true => Asked::Moved,
            false => Asked::Nothing(nothing),
        },
        Puts::Host { asked, .. } => asked(),
    }
}

/// Navigates to the 1-indexed `nth` row of a card via digit key press.
///
/// `under` specifies the parent path. Declines if the digit exceeds the drawn row count.
pub fn name_row(
    view: &mut View,
    bay: &'static str,
    built: &Built,
    control: Control,
    under: &[usize],
    nth: usize,
) -> Asked {
    let Some(card) = card_of(control) else {
        return Asked::Nothing(NOTHING_BELOW);
    };
    let rows = drawn(view, built, under);
    match built.beneath(control).nth(nth, rows) {
        Some(_) => {
            address_into(view, bay, nth);
            Asked::Moved
        }
        None => Asked::Nothing(card.named(view)),
    }
}

/// Walks vertically between card rows using arrow keys (clamped, never wrapping).
///
/// Refuses horizontal arrows `←→`. If `from` is `None` (on parent control), resumes from the remembered row.
pub fn walk_card(
    view: &mut View,
    bay: &'static str,
    built: &Built,
    control: Control,
    under: &[usize],
    from: Option<usize>,
    arrow: Arrow,
) -> Asked {
    let Some(card) = card_of(control) else {
        return Asked::Nothing("this control puts no card down");
    };
    if !(card.down)(view) {
        return Asked::Nothing(card.shut);
    }
    if arrow.across() {
        return Asked::Nothing(card.column);
    }
    let rows = drawn(view, built, under);
    if rows == 0 {
        return Asked::Nothing(card.bare(view));
    }
    let at = from.unwrap_or_else(|| {
        view.focus()
            .address(bay)
            .and_then(|address| address.remembered(under))
            .unwrap_or(1)
            .clamp(1, rows)
    });
    let to = (at as i64 + i64::from(arrow.step())).clamp(1, rows as i64) as usize;
    let address = view.focus_mut().address_mut(bay);
    // A walk moves along a level and never into one, so an address that had
    // already descended has its last step replaced rather than pushed.
    match from.is_some() {
        true => {
            address.to_row(to);
        }
        false => address.down(to),
    }
    match from.is_none() || to != at {
        true => Asked::Moved,
        false => Asked::Nothing(card.end),
    }
}

/// Executes the action for `enter` on a card row and dismisses the card (ADR-0333).
///
/// Returns focus to the parent control `above`. If execution declines, the card and address remain intact.
pub fn card_enter(
    view: &mut View,
    bay: &'static str,
    built: &Built,
    above: Control,
    row: Control,
    nth: usize,
) -> Asked {
    // The rows a card draws before the counted ones are its own verbs, so the
    // nth thing it lists is that many further down the rung.
    let verbs = built.beneath(above).first.len();
    let asked = row_act(view, row, nth, verbs);
    if matches!(asked, Asked::Nothing(_)) {
        return asked;
    }
    if let Some(Puts::Console { shut, .. }) = card_of(above).map(|card| card.puts) {
        shut(view);
    }
    view.focus_mut().address_mut(bay).up();
    asked
}

/// Resolves the specific action requested by pressing `enter` on the 1-indexed `nth` card row.
///
/// Adjusts index by `verbs` (preceding static action rows). Returns a refusal sentence for non-row controls.
fn row_act(view: &View, row: Control, nth: usize, verbs: usize) -> Asked {
    match row {
        Control::Input => match view
            .audio
            .as_ref()
            .and_then(|audio| audio.inputs.get(nth.wrapping_sub(verbs + 1)))
        {
            Some(name) => Asked::Listened(AudioAsk::Operation(Operation::AttachBeatSource {
                source: BeatSource::AudioInput(name.clone()),
            })),
            None => Asked::Nothing("this card is not drawing an input with that number"),
        },
        // Saving uses current arrangement name or prompts for one (ADR-0259).
        Control::Save => Asked::Arranged(match view.arrangement.name.clone() {
            Some(name) => Ask::Operation(Operation::SaveArrangement { name }),
            None => Ask::Name,
        }),
        Control::Filed => match view.arrangement.filed.get(nth.wrapping_sub(verbs + 1)) {
            Some(name) => Asked::Arranged(Ask::Operation(Operation::RestoreArrangement {
                name: name.clone(),
            })),
            None => Asked::Nothing("this menu is not drawing a name with that number"),
        },
        // Target pattern bank from current sequencer state.
        Control::LaneTarget => {
            let Some(bank) = view.sequencer.as_ref().map(|seq| seq.bank as u8) else {
                return Asked::Nothing("this console has no pattern behind it");
            };
            let choices = view.lane_choices();
            match choices.items.get(nth.wrapping_sub(verbs + 1)) {
                Some(choice) => Asked::Emitted(Operation::PointLane {
                    pattern: bank,
                    target: choice.target.clone(),
                }),
                None => Asked::Nothing(
                    "the chooser is not offering a target with that number — the digits count \
                     what is on the card, from one",
                ),
            }
        }
        Control::ChainProcedure => {
            let choices = view.chain_choices();
            match choices.items.get(nth.wrapping_sub(verbs + 1)) {
                Some(choice) => Asked::Emitted(choice.operation()),
                None => Asked::Nothing(CHAIN_CARD_SHORT),
            }
        }
        other => match other.answers() {
            Answers::Nothing(why) => Asked::Nothing(why),
            _ => {
                Asked::Nothing("nothing here performs — enter is the act the addressed row is for")
            }
        },
    }
}
