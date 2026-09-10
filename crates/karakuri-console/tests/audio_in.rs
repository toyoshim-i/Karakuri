//! **The transport row's `audio-in` pill: the console's nineteenth control,
//! and the first that says anything about a device.**
//!
//! Seven things, and the first of them is the one no other file in this crate
//! can check:
//!
//! 1. **That a console nobody has told draws no pill at all**, and that the
//!    arrangement pill therefore sits exactly where it sat before this control
//!    existed. `View::audio` is an `Option` for that reason and no other, and
//!    the two halves of it are two different consoles: told and found nothing,
//!    against never told.
//! 2. Where the control is, derived from the row's own geometry, and that the
//!    arrangement pill moves along by exactly this pill and one gap.
//! 3. **That it clears every boundary's grab**, which is
//!    `tests/arrangement_pill.rs`'s arithmetic over a second capsule in the
//!    same row and is measured here rather than inherited.
//! 4. **That an open card keeps the pointer**, which is `input`'s rule 2 —
//!    now written over two cards rather than one.
//! 5. That the pill says which input is open, and that no input reads `none`
//!    rather than being drawn blank.
//! 6. **What a press on a row asks for**: `AttachBeatSource` naming the input
//!    the row was drawn with, and nothing refused here.
//! 7. That a machine with no inputs gets a card that says so rather than an
//!    empty one, and that no row rectangle is handed out for it.
//!
//! **None of it opens a device, and that is not a gap in the checking.** What
//! a device could add is that a name in this list opens — which is
//! `karakuri-audio`'s `the_refusal_names_the_same_inputs_the_listing_does`
//! from one end and `karakuri`'s own `unopened` from the other. Everything
//! here is a rectangle, a word and what a press asks for, and every one of
//! those is a value.
//!
//! It does need `egui`'s fonts, because the capsule is as wide as the name in
//! it, and a `Transport`, because the pill's place is one gap after the bar
//! and a row with no engine behind it draws nothing at all.

mod common;

use common::{drawn_once, near, rect_of, PLAUSIBLE, SMALLEST};
use karakuri_console::input::{claim, Claim};
use karakuri_console::panel::{Panel, GRAB};
use karakuri_console::room::{size, Room};
use karakuri_console::view::{
    arrangement, audio_in, Arrangement, AudioAsk, AudioIn, Transport, View,
};
use karakuri_layout::{Point, Rect};
use karakuri_operation::{BeatSource, Operation};

/// **The mock's own transport, as numbers** — `transport.rs`'s `mock`. The
/// pill sits one gap after the bar this draws, so a row is needed to have a
/// pill at all.
fn mock() -> Transport {
    common::mock_transport()
}

/// A panel at a viewport, solved, with a context that has drawn once.
fn console(viewport: Rect) -> (Panel, egui::Context) {
    let mut panel = Panel::new(viewport.w, viewport.h);
    panel.solve();
    (panel, drawn_once())
}

/// A `karakuri_layout` point, from `egui`'s.
fn at(p: egui::Pos2) -> Point {
    Point::new(p.x, p.y)
}

/// **A machine with three inputs and the first of them open**, which is what a
/// program that opened the default and then read the host has.
fn listening() -> AudioIn {
    let mut audio = AudioIn::NONE;
    audio.device = Some("Scarlett 2i2".to_owned());
    audio.inputs = vec![
        "Scarlett 2i2".to_owned(),
        "MacBook Pro Microphone".to_owned(),
        "VB-Cable".to_owned(),
    ];
    audio
}

/// A view with an engine behind it and that reading of the room in front of
/// it — what `View::draw` paints from and what `claim` hit-tests, one value.
fn view(audio: Option<AudioIn>) -> View {
    let mut view = View::new(Room::Day);
    view.transport = Some(mock());
    view.audio = audio;
    view
}

// ---------------------------------------------------------------------------
// Told, and never told
// ---------------------------------------------------------------------------

/// **A console nobody has told about audio draws no pill**, and one that was
/// told there is nothing draws one saying `none`.
///
/// The two are different consoles and the `Option` is the only thing that can
/// tell them apart: a pill reading `none` on a console that was never asked
/// would be this crate answering a question about a device on its own
/// authority, which is ADR-0156's seam said about a microphone.
///
/// **And the arrangement pill is where the difference shows up geometrically**:
/// it is laid out from this pill's right edge, so a console with no audio-in
/// pill has to put it exactly where it went before this control existed.
#[test]
fn a_console_nobody_told_draws_no_pill_and_one_told_nothing_draws_none() {
    let (panel, ctx) = console(PLAUSIBLE);
    let row = karakuri_console::view::transport(&ctx, panel.layout(), Some(mock())).expect("a row");

    assert_eq!(
        audio_in(&ctx, panel.layout(), Some(mock()), None),
        None,
        "a console that was told nothing about audio drew a pill anyway"
    );
    let untold = arrangement(
        &ctx,
        panel.layout(),
        Some(mock()),
        None,
        None,
        None,
        &Arrangement::NONE,
    )
    .expect("the arrangement pill");
    assert!(
        near(untold.pill.min.x, row.bar.max.x + size::TRANSPORT_GAP),
        "with no audio-in pill the arrangement pill starts at {} and the bar ends at {}",
        untold.pill.min.x,
        row.bar.max.x
    );

    // Told, and told there is nothing. The pill is drawn and it says so.
    let nothing = AudioIn::NONE;
    let pill = audio_in(&ctx, panel.layout(), Some(mock()), Some(&nothing))
        .expect("a console that looked and found nothing draws the pill");
    assert_eq!(nothing.word(), "none");
    assert!(
        near(pill.pill.min.x, row.bar.max.x + size::TRANSPORT_GAP),
        "the capsule starts {} after the bar and `.transport`'s gap is {}",
        pill.pill.min.x - row.bar.max.x,
        size::TRANSPORT_GAP
    );

    // ...and the arrangement pill has moved along by exactly this pill and one
    // gap, which is what a flex row is.
    let after = arrangement(
        &ctx,
        panel.layout(),
        Some(mock()),
        Some(&nothing),
        None,
        None,
        &Arrangement::NONE,
    )
    .expect("the arrangement pill");
    assert!(
        near(after.pill.min.x, pill.pill.max.x + size::TRANSPORT_GAP),
        "the arrangement pill starts at {} and the audio-in pill ends at {}",
        after.pill.min.x,
        pill.pill.max.x
    );
    assert!(
        after.pill.min.x > untold.pill.min.x,
        "drawing the audio-in pill did not move the arrangement pill along at all"
    );
}

/// **The pill says which input is open**, and it carries the name rather than
/// only a light: `audio-in` alone would say a room is being heard without
/// saying whose.
#[test]
fn the_pill_names_the_input_it_is_listening_to() {
    let (panel, ctx) = console(PLAUSIBLE);
    let audio = listening();
    assert_eq!(audio.word(), "Scarlett 2i2");

    let named = audio_in(&ctx, panel.layout(), Some(mock()), Some(&audio)).expect("a pill");
    let silent =
        audio_in(&ctx, panel.layout(), Some(mock()), Some(&AudioIn::NONE)).expect("a pill");
    assert!(
        named.pill.width() > silent.pill.width(),
        "`{}` and `none` laid out to the same width, so the capsule is not as wide as \
         the name in it",
        audio.word()
    );
    assert!(
        near(named.pill.height(), size::PILL_H),
        "the capsule is {} tall and a pill is 11 at line-height 1.5",
        named.pill.height()
    );
}

// ---------------------------------------------------------------------------
// The grab
// ---------------------------------------------------------------------------

/// **The capsule clears every boundary's grab**, measured here rather than
/// reasoned from the arrangement pill's 15.75 one capsule along.
///
/// The transport row is 48 and a `.pill` is 16.5 centred in it, so there is
/// (48 - 16.5) / 2 = 15.75 above and below, against a `GRAB` of 6. A press in
/// the capsule is therefore never inside the band of the boundary under the
/// row, and rule 3 never gets to refuse it.
#[test]
fn the_capsule_clears_every_boundarys_grab() {
    let (mut panel, ctx) = console(SMALLEST);
    let strip = rect_of(panel.layout(), "transport");
    let audio = listening();
    let pill = audio_in(&ctx, panel.layout(), Some(mock()), Some(&audio)).expect("a pill");

    let above = pill.pill.min.y - strip.y;
    let below = (strip.y + strip.h) - pill.pill.max.y;
    assert!(
        above > GRAB && below > GRAB,
        "the capsule has {above} above it and {below} below, against a grab of {GRAB}"
    );

    // And the whole of it goes to the panel, top edge to bottom edge.
    let view = view(Some(audio));
    for y in [
        pill.pill.min.y + 0.5,
        pill.pill.center().y,
        pill.pill.max.y - 0.5,
    ] {
        let point = Point::new(pill.pill.center().x, y);
        assert_eq!(
            claim(&mut panel, &ctx, &view, point),
            Claim::Panel,
            "a press at {point:?} on the capsule did not go to the panel"
        );
    }
}

// ---------------------------------------------------------------------------
// The card
// ---------------------------------------------------------------------------

/// **A press on the pill puts the card down, and a press on it again takes it
/// away.** The card lists one row per input and nothing above them: there is
/// no verb here, because an input is picked from what exists and is never
/// named into being.
#[test]
fn a_press_on_the_pill_lists_the_inputs_and_a_press_again_shuts_it() {
    let (panel, ctx) = console(PLAUSIBLE);
    let mut audio = listening();

    let shut = audio_in(&ctx, panel.layout(), Some(mock()), Some(&audio)).expect("a pill");
    assert_eq!(
        shut.menu, None,
        "the card is drawn with nothing asking for it"
    );
    assert_eq!(shut.rows, 0);
    assert_eq!(
        shut.ask(&audio, at(shut.pill.center())),
        Some(AudioAsk::Open)
    );

    audio.opened();
    let open = audio_in(&ctx, panel.layout(), Some(mock()), Some(&audio)).expect("a pill");
    let card = open.menu.expect("the card is down and nothing is drawn");
    assert_eq!(
        open.rows,
        audio.inputs.len(),
        "the card is one row per input and nothing else"
    );
    assert!(
        card.min.y > open.pill.max.y,
        "the card is at {:?} and the pill ends at {}",
        card.min,
        open.pill.max.y
    );
    assert_eq!(
        open.ask(&audio, at(open.pill.center())),
        Some(AudioAsk::Shut)
    );

    // A press on the card but on no row — its padding — is the dismissal
    // rather than a press that did nothing at all.
    let padding = egui::pos2(card.center().x, card.min.y + 1.0);
    assert_eq!(open.item(at(padding)), None);
    assert_eq!(open.ask(&audio, at(padding)), Some(AudioAsk::Shut));
}

/// **Each row asks for the input it was drawn with, by name**, and the name is
/// the one the device answered to — which is what a selector is matched
/// against, so what leaves the panel is what opens the device.
///
/// **Nothing is refused here.** A device that has gone away since the list was
/// read is a name this control cannot check, and it does not pretend to: the
/// refusal happens where the bytes are.
#[test]
fn a_press_on_a_row_asks_for_that_input_by_name() {
    let (panel, ctx) = console(PLAUSIBLE);
    let mut audio = listening();
    audio.opened();
    let open = audio_in(&ctx, panel.layout(), Some(mock()), Some(&audio)).expect("a pill");

    for (index, name) in audio.inputs.iter().enumerate() {
        let row = open.row(index);
        assert_eq!(open.item(at(row.center())), Some(index));
        assert_eq!(
            open.ask(&audio, at(row.center())),
            Some(AudioAsk::Operation(Operation::AttachBeatSource {
                source: BeatSource::AudioInput(name.clone()),
            })),
            "row {index} of the card does not ask for `{name}`"
        );
    }
}

/// **An open card keeps every pointer event, boundary or no boundary**, and
/// gives the boundary back the moment it is shut.
///
/// This is `input`'s rule 2, which arrived with the arrangement pill and is
/// written over both cards. It is asserted again here rather than inherited,
/// because *the card is modal* and *the grab has gone missing* look identical
/// from one assertion.
#[test]
fn an_open_card_keeps_the_pointer_and_a_shut_one_gives_the_boundary_back() {
    let (mut panel, ctx) = console(PLAUSIBLE);
    let strip = rect_of(panel.layout(), "transport");
    let boundary = Point::new(strip.x + strip.w * 0.5, strip.y + strip.h);
    let elsewhere = Point::new(strip.x + strip.w * 0.5, strip.y + strip.h + 300.0);

    let mut audio = listening();
    assert_eq!(
        claim(&mut panel, &ctx, &view(Some(audio.clone())), boundary),
        Claim::Panel,
        "the boundary under the row is not the panel's with the card shut"
    );
    assert_eq!(
        claim(&mut panel, &ctx, &view(Some(audio.clone())), elsewhere),
        Claim::Egui
    );

    audio.opened();
    let open = view(Some(audio.clone()));
    assert_eq!(claim(&mut panel, &ctx, &open, boundary), Claim::Panel);
    assert_eq!(
        claim(&mut panel, &ctx, &open, elsewhere),
        Claim::Panel,
        "a point out in a bay went to `egui` with a card open over the console"
    );

    audio.shut();
    let back = view(Some(audio));
    assert_eq!(claim(&mut panel, &ctx, &back, boundary), Claim::Panel);
    assert_eq!(
        claim(&mut panel, &ctx, &back, elsewhere),
        Claim::Egui,
        "the bay stayed the panel's after the card was shut, so it is not the card that \
         was claiming it"
    );
}

/// **A machine with no inputs gets a card that says so**, and it is a sentence
/// rather than a list: `rows` is zero, so nothing hands out a row rectangle
/// for it and no press on it can ask for an input that does not exist.
///
/// An empty card would be indistinguishable from one that failed to open,
/// which is the failure P-0094 is about.
#[test]
fn a_machine_with_no_inputs_gets_a_card_that_says_so_rather_than_an_empty_one() {
    let (panel, ctx) = console(PLAUSIBLE);
    let mut audio = AudioIn::NONE;
    audio.opened();

    let open = audio_in(&ctx, panel.layout(), Some(mock()), Some(&audio)).expect("a pill");
    let card = open.menu.expect("the card is down and nothing is drawn");
    assert_eq!(open.rows, 0, "a card with no inputs handed out a row");
    assert_eq!(open.of, 0);
    assert!(
        card.height() > size::LIB_ROW_H,
        "the card is {} tall and has one line in it",
        card.height()
    );
    // Every press inside it is the dismissal, because there is nothing to pick.
    assert_eq!(open.item(at(card.center())), None);
    assert_eq!(open.ask(&audio, at(card.center())), Some(AudioAsk::Shut));
}

/// **A machine with more inputs than the window is tall lists what fits and
/// says how many there are.** The Library bay's own answer to the same
/// question, and this reads it off `rows` and `of` rather than a second count.
#[test]
fn a_card_that_could_not_be_shown_whole_says_how_many_there_are() {
    let (panel, ctx) = console(SMALLEST);
    let mut audio = AudioIn::NONE;
    audio.device = Some("one".to_owned());
    audio.inputs = (0..80).map(|n| format!("input {n}")).collect();
    audio.opened();

    let open = audio_in(&ctx, panel.layout(), Some(mock()), Some(&audio)).expect("a pill");
    assert_eq!(open.of, audio.inputs.len());
    assert!(
        open.rows < open.of,
        "eighty inputs fitted in a {} tall console",
        SMALLEST.h
    );
    assert!(open.rows > 0, "the card lists nothing at all");
    // Every row it does draw is inside the card, which is what stops the list
    // ending off the bottom of the window with nothing saying so.
    let card = open.menu.expect("a card");
    for index in 0..open.rows {
        assert!(
            card.contains(open.row(index).center()),
            "row {index} of {} is outside the card",
            open.rows
        );
    }
}
