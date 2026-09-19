use karakuri_layout::Point;
use karakuri_operation::Operation;

use super::Outcome;

/// A pointer event, stripped to what the rule needs.
///
/// A button is left, or it is the secondary button, or it is not routed at all.
/// This said *a button is left or it is not routed* until 2026-09-09, and it
/// was exact: `window_event` matched `MouseButton::Left` and every other button
/// fell through its `_ => {}`, so a right press reached nothing in this program
/// and `egui` was never told about one either.
///
/// What made it a second variant rather than a field on [`Pointer::Down`] is
/// where the answer is wanted. `karakuri_console::input::claim` decides *whose*
/// an event is from where the pointer is and has never known which button a
/// press was — rules 1 to 4 are all about position — and nothing about that
/// changes: a secondary press on a control the console draws is the console's
/// for the same reason a primary one is. What differs is only what the press
/// then *asks* for, which is this file's half of the seam. A `Down { secondary:
/// bool }` would have put the flag through every arm of the press handler to be
/// read by one of them
/// ([ADR-0311](../../../docs/adr/0311-a-row-menu-loads-a-set-onto-a-named-deck-and-saves-it-through-the-systems-own-dialog.md)).
///
/// There is no `Secondary` release, and that is the whole of what this button
/// does here: a secondary press opens a menu and the gesture ends at the *next*
/// press, which is [`Pointer::Down`]'s or this one's again. Nothing is taken in
/// hand on a secondary press, so there is nothing for a release to let go of.
///
/// The wheel carries a distance now, and only one axis of it. It used to carry
/// nothing, because nothing on this panel did anything with one — *which wheel
/// axis it was does not change who gets it* is what this said, and it was true
/// while the answer was always `egui`'s. An Inspector pane scrolls down its
/// list
/// ([ADR-0307](../../docs/adr/0307-the-inspectors-pane-scrolls-and-the-position-is-the-panes-own.md)),
/// so the vertical distance is now the whole of what the event says; a
/// horizontal one reaches nothing here and is dropped where the two are pulled
/// apart, in `window_event`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Pointer {
    Moved(Point),
    Down,
    Up,
    /// A press of the secondary button, which on this panel opens the menu on a row
    /// of the Library bay's list and does nothing anywhere else.
    Secondary,
    /// How far to scroll, in logical pixels, positive down the list — a notch of a
    /// mouse wheel converted to `karakuri_console::room::size::WHEEL_STEP` and a
    /// trackpad's own pixels passed straight through.
    Wheel(f32),
}

/// What routing a pointer event did, beyond deciding whose it was.
///
/// A list rather than an `Option<Outcome>`, because the controls on this panel
/// end in more than one place: the Outputs dot asks for an operation on the
/// *arrangement*, which this crate performs and reports as an [`Outcome`], a
/// fader asks for an operation on the *mix*, which nothing in
/// `karakuri-console` can perform at all, and two of them ask for no operation
/// and are still not nothing. The repaint decision is taken from which of them
/// it is — see `Change::Operated` and `Change::Emitted`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Acted {
    /// Nothing acted: a press on a boundary, a move, a wheel, a release.
    Nothing,
    /// The Outputs dot, and what the operation it named did.
    Operated(Outcome),
    /// A fader translated a drag into the vocabulary, or the drag moved the pointer
    /// over a value that did not change and asked for nothing.
    Emitted(Option<Operation>),
    /// A class pill was pressed, and what it wrote went to the run's `Opening`
    /// rather than to the arrangement or to the deck.
    ///
    /// A fourth answer rather than a reuse of `Nothing`, and the difference is the
    /// whole of ADR-0236: this press is not an operation and must never be made
    /// into one, so it cannot be an `Emitted`; and it is not nothing either,
    /// because a word on the panel changed. See `Readout::opened`.
    ///
    /// It carries no payload because there is none to carry: what changed is held
    /// in the `Opening`, which is a handle another surface reads, and a copy of it
    /// in this enum would be the second answer to *what is open*.
    Opened,
    /// A press moved the library cursor, and asked for nothing ([`Readout::took`]).
    ///
    /// It is the console's one pointer a press moves *without* naming an operation:
    /// the deck selection moves on a press too, and reaches [`Acted::Emitted`]
    /// through `Operation::SelectDeck` and [`pointed`], which is
    /// `Change::Pointed`'s note on the same pair.
    ///
    /// A fifth answer rather than a reuse of [`Acted::Nothing`], which is
    /// [`Acted::Opened`]'s argument one control along: this press names no
    /// operation and must not be made into one (ADR-0265), so it cannot be an
    /// [`Acted::Emitted`]; and it is not nothing either, because the reading
    /// follows the cursor — a move with one open is a read of the row it arrived at
    /// (`karakuri_console::view::View::reading_open`), and a caller that could not
    /// tell this press from a boundary's would leave the block drawn nowhere.
    ///
    /// The caller is what owes that read, and not [`Readout::took`]: a reading is a
    /// file, and the store is the window's rather than the readout's — the division
    /// [`Readout::asked_to_read`] is written to.
    ///
    /// It carries no payload because there is none to carry: it is answered only
    /// where the cursor moved, which is `View::point_at`'s `bool`, and a copy of
    /// the row here would be a second answer to `View::cursor_row`.
    Pointed,
}
