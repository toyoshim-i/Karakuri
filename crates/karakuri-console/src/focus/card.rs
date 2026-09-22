use super::*;

/// One of the four cards the address descends into, and the whole of what one
/// card does not share with the next.
///
/// A card is a rung hanging off a control: `enter` on the control puts the card
/// down, a digit names the nth row of it, `↑↓` walk the rows from the one the
/// card remembers — clamped, never wrapped — `←→` are refused, `enter` on a row
/// performs that row, and `esc` or `Tab` takes the card away and leaves the
/// address on the control it hangs from. That is written once, in
/// [`open_card`], [`name_row`], [`walk_card`] and [`card_enter`]; a row of
/// [`CARDS`] is what those four read.
///
/// The four are the audio-in pill's inputs and the arrangement pill's menu
/// ([ADR-0350](../../../docs/adr/0350-the-transports-two-cards-are-walked-and-the-tempo-figure-steps-by-a-beat-a-minute.md)),
/// the Sequencer's `+ lane` chooser
/// ([ADR-0351](../../../docs/adr/0351-the-lane-chooser-is-a-rung-of-the-address.md))
/// and the Master's `+ add` chooser
/// ([ADR-0352](../../../docs/adr/0352-the-chains-list-is-the-master-bays-items-and-a-slot-is-taken-out-by-a-glyph-on-its-row.md)).
pub(crate) struct Card {
    /// The control the card hangs from, and what [`card_of`] finds it by.
    pub(crate) control: Control,
    /// Whether the card is down.
    pub(crate) down: fn(&View) -> bool,
    /// How many rows it is drawing, which is zero while it is up.
    pub(crate) rows: fn(&View) -> usize,
    /// How `enter` on the control puts it down, and what takes it away.
    pub(crate) puts: Puts,
    /// What a press addressed below the control says while the card is up: the
    /// rung is not drawn, and the sentence names the press that draws it.
    pub(crate) shut: &'static str,
    /// What `enter` on the control says with the card already down. It names the
    /// keys that reach the rows.
    pub(crate) standing: &'static str,
    /// What a digit that named a row the card is not drawing says.
    pub(crate) short: &'static str,
    /// What an arrow says on a card that is down with nothing on it to walk.
    pub(crate) empty: &'static str,
    /// What `←→` say, naming the pair that walks the rows.
    pub(crate) column: &'static str,
    /// What an arrow says where the address is already at the end.
    pub(crate) end: &'static str,
    /// A sentence this card's own state carries in place of [`Card::short`] and
    /// [`Card::empty`], or `None` for a card with no such state. The arrangement
    /// menu's is the one: while it is asking for a name it is a field rather than
    /// a list, and it draws no rows.
    pub(crate) busy: Option<fn(&View) -> Option<&'static str>>,
}

/// How a card is put down and taken away.
#[derive(Clone, Copy)]
pub(crate) enum Puts {
    /// This console's own state: the method that puts the card down, the method
    /// that takes it away, and the sentence for a card with nothing to offer,
    /// which is what refuses to put it down.
    Console {
        put: fn(&mut View) -> bool,
        shut: fn(&mut View) -> bool,
        nothing: &'static str,
    },
    /// The host's: `enter` leaves in the control's own word for it and the host
    /// puts the card down, through the method a pointer press already reaches
    /// (ADR-0156). `shut` is still this console's, because `esc` and `Tab` take
    /// the card away without asking the host for anything.
    Host {
        asked: fn() -> Asked,
        shut: fn(&mut View),
    },
}

impl Card {
    /// What a digit that named a row this card is not drawing says: the card is
    /// up, its own state is in the way, or it draws fewer rows than that.
    pub(crate) fn named(&self, view: &View) -> &'static str {
        match (self.down)(view) {
            false => self.shut,
            true => self.asking(view).unwrap_or(self.short),
        }
    }

    /// What an arrow says on a card that is down and drawing nothing.
    pub(crate) fn bare(&self, view: &View) -> &'static str {
        self.asking(view).unwrap_or(self.empty)
    }

    /// The sentence this card's own state carries in place of a count's, or
    /// `None` where nothing is in the way.
    pub(crate) fn asking(&self, view: &View) -> Option<&'static str> {
        self.busy.and_then(|state| state(view))
    }

    /// Take this card away. A card that is already up is left as it is.
    pub(crate) fn shut(&self, view: &mut View) {
        match self.puts {
            Puts::Console { shut, .. } => {
                shut(view);
            }
            Puts::Host { shut, .. } => shut(view),
        }
    }
}

/// The sentence a press addressed to a Transport card's rung carries while the
/// card is up: the rung is not drawn, and `enter` on the pill is the press that
/// draws it.
const CARD_SHUT: &str = "this card is not down, so there is nothing here to name — press enter on \
                         this control to put it down";

/// The sentence a digit carries when a Transport card is drawing fewer rows than
/// that.
const CARD_SHORT: &str =
    "this card is not drawing a row with that number — the digits count what is on it, from one";

/// The sentence `enter` on a Transport pill carries with its card already down.
const CARD_STANDING: &str = "this card is already down — press a digit to name a row, or walk it \
                             with up and down, and enter runs the row the address is on";

/// The sentence the arrangement menu carries while it is a field rather than a
/// list. The keyboard is the name's while it is asking for one, which is the one
/// flow on this panel that takes letters (ADR-0259).
const NAMING: &str = "this menu is asking for a name — type it and press return, or press esc to \
                      leave the arrangement unsaved";

/// The sentence a digit carries when the `+ add` chooser is not offering that
/// many.
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

/// The card on the focused bay's address, and whether the address has descended
/// into its rows — or `None` where the address is on no control that hangs a
/// card, or where the card that control hangs is up.
///
/// The reading `esc` takes. A card on the address's path is a level of the
/// address; a card that is down anywhere else is not, and `esc` leaves it alone
/// ([ADR-0332](../../../docs/adr/0332-focus-is-a-pointer-the-console-owns-and-the-three-pointers-are-instances-of-it.md)).
///
/// `panel` must be solved: the focused bay is read off the arrangement.
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

/// `enter` on the control a card hangs from: the card, put down.
///
/// Declines where the card is already down, and where it has nothing to offer —
/// a card with no rows offers nothing to pick and nothing to leave by, which is
/// [`crate::view::View::open_lane`]'s own rule. The address stays on the control;
/// a digit or an arrow is what descends into the rows.
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

/// A digit below the control a card hangs from: the nth row of the card, with the
/// address descended onto it.
///
/// `under` is the path the card's rung is at — `[HEAD, through]` for a card on a
/// head control and `[through]` for one on a headless row's own control. The
/// digits count what the card drew, from one, and a digit past that declines with
/// the card's own sentence.
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

/// An arrow on the control a card hangs from, or on one of the card's rows: the
/// neighbouring row.
///
/// `from` is the row the address is on, or `None` where it is still on the
/// control — and then the walk starts from the row the card remembers, which is
/// [`walk`]'s rule at bay level one rung down: the arrows work with no digit
/// pressed first. `under` is the path the card's rung is at, as [`name_row`]
/// takes it.
///
/// A card is a column of rows, so `←→` are refused and the refusal names the pair
/// that works. The walk is clamped at both ends and never wraps.
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

/// `enter` on one of a card's rows: what that row performs, with the card taken
/// away and the address back on the control it hangs from.
///
/// `above` is that control and `row` is the control the addressed row is. A row
/// that declines performs nothing, so the card stays down and the address stays
/// on the row.
///
/// A card this console owns is taken away here, before the answer leaves; a card
/// the host owns leaves in the answer, and the host takes it away through the
/// method a pointer press on that row already reaches (ADR-0333). `above` is
/// allowed to hang no card — a slot of the master chain is a rung and not one —
/// and then the row answers for itself and nothing is taken away.
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

/// What `enter` on one row of a card asks for — the one thing the four cards do
/// not share.
///
/// `nth` counts the rung from one and `verbs` is how many rows the card draws
/// before the ones it lists, so the nth listed thing is at `nth - verbs - 1`. A
/// control that is not a card's row answers with its own sentence.
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
        // **Saving again means the name in use**, and with none in use the
        // menu asks for one: the field takes the keyboard whole while it is
        // asking, which is where the name is typed and where `return` files it
        // (ADR-0259).
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
        // **The bank is this bay's own reading** rather than whichever pattern
        // is armed by the time the operation is performed, which is every other
        // arm of this bay's.
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
