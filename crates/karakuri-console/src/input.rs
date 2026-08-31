//! **Who gets a pointer event: the panel's boundaries, or `egui`.**
//!
//! The dividers are ours. [`Panel::press`], [`Panel::moved`] and
//! [`Panel::released`] move them over [`Layout::hit`], and `egui` has no
//! widget in those gaps — they are the ground showing between two bays, and
//! nothing is drawn there to be clicked. So on the face of it there is no
//! conflict, and the two would never both react to the same click.
//!
//! That is exactly why the rule is written down here rather than left to be
//! re-derived. It is true only while the gaps stay empty. [`GRAB`] widens
//! every boundary by six pixels either side, because a nine-pixel gap is not a
//! target a hand finds — and those twelve pixels are *inside the bays*, over
//! whatever the bay draws at its edge. The first control placed near a bay's
//! edge is under a boundary's grab, and then both do think they are dragging.
//!
//! # The first control is drawn, and it clears the grab
//!
//! The Outputs row's one sink ([`crate::view::outputs`]) is that control, and
//! the paragraph above is why it is measured rather than assumed. The row is
//! 34 tall and the chip is 18.5, centred, so there is **7.75** of row above
//! the chip and 7.75 below, against a [`GRAB`] of **6**: the boundary's band
//! ends 1.75 pixels above the control and nothing overlaps.
//!
//! **That is a fact about two constants and a rectangle, so it is a test and
//! not a sentence** — `tests/outputs.rs`, which fails if the control moves up,
//! if the row gets shorter, or if [`GRAB`] widens. Widening the grab past 7.75
//! makes the sink unclickable in its top edge and the rule below is what would
//! have to change, deliberately, rather than the control being nudged.
//!
//! # The rule
//!
//! **The boundary gets first refusal.**
//!
//! 1. **A drag in hand keeps its claim**, wherever the pointer has wandered
//!    to. A drag is a gesture and not a position: a boundary held against a
//!    stop while the pointer runs on across three bays is the ordinary case,
//!    not the odd one, and a claim re-decided from the pointer each event
//!    would hand the middle of that gesture to `egui`.
//! 2. **An open menu keeps the pointer until it is shut** — every point of
//!    the console, not only the menu's own card.
//!
//!    **This is rule 1 again rather than a second exception to rule 4**, and
//!    it was added with the arrangement pill because that is the first control
//!    here whose gesture outlives the press that started it. A menu that is
//!    down is a hand mid-choice exactly as a boundary in hand is a hand
//!    mid-drag, and the next press is part of that gesture whichever way it
//!    ends: on a row it picks, anywhere else it dismisses. Neither is `egui`'s
//!    and neither is a boundary's.
//!
//!    **It has to come before the boundary's first refusal, and that is the
//!    whole reason it is a rule and not a clause of rule 4.** The menu hangs
//!    out of the transport row and down over the bays, so it crosses the
//!    boundary under that row and whatever pane divider is beneath it — and
//!    under rule 4 those rows would be dead, silently, in the middle of a list
//!    an operator is reading. The clearance arithmetic that keeps the *pill*
//!    clickable cannot be done for a card that is deliberately drawn across
//!    the panel; the honest answer is that the card is modal while it is
//!    there, and the price is that a boundary cannot be dragged with a menu
//!    open. Pressing it shuts the menu, and the second press drags.
//!
//!    See [`crate::view::Arrangement::open`] and
//!    [`crate::view::AudioIn::open`], which are the whole of the condition,
//!    and `tests/arrangement_pill.rs` and `tests/audio_in.rs`, which assert
//!    both halves for each — that a boundary under the open card goes to the
//!    panel, and that it goes back to being an ordinary boundary the moment
//!    the card is shut.
//!
//!    **Two cards can be down and never at once.** The audio-in pill has one
//!    too, and this clause is written over both: whichever is open claims the
//!    press that would have opened the other, and that press shuts it. So the
//!    second press opens the second card, which is the same one-extra-press
//!    price a boundary already pays.
//! 3. Otherwise, if the pointer is within [`GRAB`] of a boundary, it is the
//!    panel's and `egui` does not see the event.
//! 4. **Otherwise, if the pointer is on a control the console draws, it is the
//!    panel's** — because the console paints and `egui` owns no widget
//!    anywhere in it, so a press routed to `egui` there reaches nothing at
//!    all. The panel's controls are painted shapes, and the only thing that
//!    knows a press landed on one is this rule.
//!
//!    **How many of them there are is [`CONTROLS`]**, and that is a number
//!    this crate exports rather than one this paragraph keeps: a surface
//!    describing itself to an operator has to say what a pointer reaches, and
//!    a sentence saying it is where the count goes stale. What they are is
//!    still written here, because a name is not a number and there is nowhere
//!    else the nineteen sit together: the Outputs row's sink
//!    ([`crate::view::outputs`]), a mixer strip's fader knob
//!    ([`crate::view::Mixer::grab`]), its blend chip
//!    ([`crate::view::Mixer::blend`]), its tally chip
//!    ([`crate::view::Mixer::tally`]), its mask mini
//!    ([`crate::view::Mixer::mask`]), the transport row's audio-in pill
//!    ([`crate::view::audio_in`]) and its arrangement pill
//!    ([`crate::view::arrangement`]), at the end of that row the tone map's
//!    capsule and the exposure track ([`crate::view::look`]), in an
//!    inspector pane's deck head the sync chip, the anchor and the scrub's two
//!    arrows ([`crate::view::deck_head`]), the Master bay's out
//!    ([`crate::view::MasterRow::grab`]), the Program bay head's `solo`
//!    ([`crate::view::program_head`]) and the four deck preview cells under it
//!    ([`crate::view::ProgramBay::preview`]). The rule did not change to hold
//!    any of the seventeen that came after the first, which is what it was
//!    written for — and each is asked exactly the way the first is: the
//!    derivation that draws it, asked whether the point is on it, with nothing
//!    stored.
//!
//!    **The last five are the first two that a boundary's grab reaches**, and
//!    the rule holds them unchanged for the reason it holds everything else:
//!    rule 3 gives the boundary first refusal, so what the overlap costs is
//!    the sliver of the control inside the band and never an ambiguity about
//!    who is dragging. Both live where `docs/manual/console.html` draws them,
//!    flush against a region's own edge: the `solo` capsule sits 5.25 into a
//!    27-tall bay head whose top edge is the body row's, and a preview cell's
//!    top edge *is* the `deck-previews` region's. `crate::view::program_head`
//!    and `tests/preview_cells.rs` are where the two numbers are measured, and
//!    changing either of them is a change to the console page or to [`GRAB`]
//!    rather than a nudge.
//!
//!    **The sixth was the first control in the transport row**, which was four
//!    readouts and nothing a press acted on until it landed
//!    ([`crate::view::transport`]). It clears every boundary by more than any
//!    of the other five: the row is 48 and a `.pill` is 16.5, centred, so
//!    there is (48 - 16.5) / 2 = **15.75** of row above it and 15.75 below,
//!    against a [`GRAB`] of 6 — `tests/arrangement_pill.rs`, which is
//!    `tests/outputs.rs`'s arithmetic over this control and fails the same
//!    three ways.
//!
//!    **The seventh and the eighth are in that row too, and their clearance is
//!    measured rather than inherited from it** — which is this file's rule
//!    about every control, and here it happens to come out at the same number
//!    twice. The tone map's capsule is a `.pill`, so it is the pill's own
//!    **15.75**. The exposure control is a 5px track, and a 5px-tall target is
//!    not something a hand finds — so what a press is tested against is the
//!    track grown to a line's height, [`crate::view::LookRow::grip`], which is
//!    `.mini`'s padding argument met with a band. That is 16.5 in a row of 48
//!    and therefore **15.75** as well. `tests/look.rs` measures both and fails
//!    the same three ways `tests/arrangement_pill.rs` does; that two of the
//!    nineteen agree is a fact about two capsules being one height, not a number
//!    either of them inherited.
//!
//!    **The ninth to the twelfth are the first controls that are not in a row
//!    of their own**, and they are measured off their own rectangles like
//!    everything else here. A deck head is the second row *inside* an
//!    inspector pane, so the nearest boundary is not the one under the row it
//!    sits in — it is the **pane divider down the side of the pane**, and what
//!    holds the chips off it is `.deck-head`'s own padding
//!    ([`crate::view::size::DECK_HEAD_PAD_X`]): the leftmost chip starts
//!    **10** in from the pane's edge, and the fold ends 10 in from the other,
//!    against a [`GRAB`] of 6.
//!
//!    **Down the row the clearance is the largest on the console**, and it is
//!    a sum rather than a centring: the bay head is painted over the top of
//!    the region and the pane's own `.half-head` is under it, so there is
//!    27 + 27.5 + 5 = **59.5** of pane above the chips and more below. The
//!    mask mini's 74.50 was the previous largest and it is the same shape of
//!    number — a control several rows into a bay.
//!
//!    **10 is not the tightest on the console; the blend chip's 8.97 still
//!    is.** It is the tightest in this bay, and the two would go together if
//!    the grab were widened past 8.97. `tests/deck_head.rs` measures all four
//!    controls against every boundary and fails the same three ways
//!    `tests/arrangement_pill.rs` does.
//!
//!    **The thirteenth is the Master bay's out**, and it was measured the same
//!    way off its own rectangle. The knob is 11 tall on a row of 16.5 that
//!    starts [`crate::room::size::HEAD_H`] plus
//!    [`crate::room::size::MASTER_PAD_TOP`] below the bay's top edge, so what
//!    holds it off the boundary **above** is a sum and not a centring:
//!    27 + 8 + 2.75 = **37.75**, which is the second largest on the console
//!    after the deck head's 59.5 and for the same reason — a control several
//!    rows into a bay. Down the sides it is `.master-body`'s padding that
//!    holds it, and the knob overhangs its track by half its own width: 10 of
//!    padding plus the `out` label and [`crate::room::size::MASTER_GAP`] on
//!    the left, 10 plus the figure and the same gap on the right, less 4.5 of
//!    overhang either way. The binding number is the one **below**, which is
//!    whatever is left of the bay under the row — the bay is `flex: 1` in its
//!    column, so it is measured rather than computed. `tests/master.rs`
//!    measures all four against every boundary and fails the same three ways
//!    `tests/arrangement_pill.rs` does.
//!
//!    **Three of the four are claimed and the fourth is a mode away from
//!    being**, which is this rule's *a control claims what it acts on and no
//!    more* met by a state rather than by a rectangle: a scrub arrow is inert
//!    on a deck that is not beat-synced, so it is drawn, it keeps its shape,
//!    and [`crate::view::DeckHead::owns`] does not claim it. The `composite`
//!    chip beside them is never claimed at all — layering is a build decision
//!    and there is no operation for the press to name.
//!
//!    **The fourth is the first control with a state that can be pending**,
//!    and it is still only an affordance: it names a destination and refuses
//!    nothing, because what may be asked belongs where the record is applied
//!    ([P-0076](../../../docs/principles/0076-a-surface-owns-the-affordance-never-the-authority.md)).
//!    So this rule holds it unchanged too.
//!
//!    **The fifth is the first control that carries a value it does not
//!    draw**, and this rule does not change for that either: what a press
//!    *asks for* is [`crate::view::Mixer::mask`]'s, and whether a press landed
//!    on the chip is one rectangle either way
//!    ([ADR-0203](../../../docs/adr/0203-the-mask-chip-carries-the-angle-it-does-not-control.md)).
//!
//!    **A control claims what it acts on and no more.** A fader's *track* is
//!    drawn by the console and is not claimed, because a press on it does
//!    nothing — see [`crate::view::Mixer::grab`] for why a press off the knob
//!    must not move the value. Claiming a press in order to throw it away
//!    would put the rule and the act out of step, and `egui` owns nothing
//!    there either, so the two answers are the same nothing.
//! 5. Otherwise it goes to
//!    [`egui_winit::State::on_window_event`](https://docs.rs/egui-winit) and
//!    `egui` decides.
//!
//! Rule 3 before rule 4 is *first refusal* meant literally: a control under a
//! boundary's grab would be dead, and it is the test above that keeps the
//! order from ever mattering. **The mixer's knobs are measured the same way**
//! and in the same file's spirit — `tests/fader.rs` asserts that no knob in
//! the bay is within [`GRAB`] of any boundary, and carries the same guard on
//! itself, so the ordering goes on costing nothing there too. **The blend
//! chip is measured a third time** in `tests/blend.rs`, **the tally chip a
//! fourth** in `tests/tally.rs` and **the mask mini a fifth** in
//! `tests/mask.rs`, each with its own guard: whether a control clears every
//! boundary's band is two constants and a rectangle, and it is never inherited
//! from the control above it. No two of the three chips are the same number —
//! the nearest boundary to all of them is the pane divider down the left of
//! the bay, and the **8.97** the blend chip clears it by is the mode row's
//! width, where the tally's **14.23** is its own capsule's. The mask mini
//! shares the blend chip's row and clears the same divider by **41.03**,
//! because it sits at the far end of that row — and it is the one chip whose
//! *nearest boundary is not the same one on every strip*: on the three strips
//! that are not against the pane, the nearest is the boundary under the bay,
//! **74.50** below the mode row. So the number that binds is 41.03 and it was
//! measured rather than inherited from the chip 3px to its left.
//!
//! # A caller acts on the control, and it asks the same question again
//!
//! [`claim`] says *the panel's*; it does not say *the dot's*. What a press on
//! the control does is [`crate::view::Outputs::op`] — one operation, named —
//! and the caller asks [`crate::view::outputs`] for it. That is the same one
//! derivation this rule hit-tests, asked a second time rather than copied, so
//! the chip that claims a press and the chip that acts on it cannot come
//! apart. A fader is the same arrangement: [`crate::view::Mixer::grab`]
//! answers *is this a control* here and *which control, and where along it*
//! to the caller, off one derivation and one set of values. So is the blend
//! chip: [`crate::view::Mixer::blend`] answers *is this a control* and *what
//! does a press on it ask for* — `SetBlendMode` naming the mode after the one
//! the deck reports — off the same laid-out strip this rule hit-tests. So is
//! the tally chip: [`crate::view::Mixer::tally`] answers the same two with
//! `SetResidency` naming the residency after the one that was **requested**,
//! which is the whole of ADR-0195 and is why a press on a parked chip asks for
//! the withdrawal of its own prime request.
//!
//! So is the mask mini: [`crate::view::Mixer::mask`] answers *is this a
//! control* and *what does a press on it ask for* — `SetMaskShape` naming the
//! shape after the one the deck reports, **and the angle the deck reports
//! beside it**, which is the whole of ADR-0203 and is why choosing a shape
//! does not straighten a diagonal front.
//!
//! **So is the Master bay's out**, and it is [`crate::view::Mixer::grab`]'s
//! arrangement rather than the look's: [`crate::view::MasterRow::grab`]
//! answers *is this a control* here and *what is in hand, and where along it*
//! to the caller, off one derivation and one level. A press on its **track**
//! is not claimed, for the reason a strip's is not — the mock draws a handle
//! on this fader and draws none on the exposure track, and a handle that
//! jumped to the pointer would be a lie about what a handle is.
//!
//! **So are the two look controls**, at the far end of the same row:
//! [`crate::view::LookRow::tonemap`] answers `SetTonemap` naming the operator
//! after the one that is running, which is the blend chip's affordance over a
//! closed list of four (ADR-0187); and [`crate::view::LookRow::exposure`]
//! answers `SetExposure` naming **where along the track the press landed**,
//! which is the one control on this panel that a press sets outright. That
//! last is deliberately not [`crate::view::Mixer::grab`]'s rule about a press
//! on a track, and the reason is written at the method: a fader has a knob and
//! a value that must not jump under a hand mid-gesture, and this has neither.
//!
//! **So are the deck head's three**, two bays down and reached the same way:
//! [`crate::view::DeckHead::sync`] answers `SetSync` naming the mode the
//! cycle arrived at — with a mode this deck's material cannot honour **skipped
//! rather than offered**, which is the blend chip's affordance over a list one
//! reading has narrowed; [`crate::view::DeckHead::reanchor`] answers `SetSync`
//! naming the mode the deck is **already in**, which is the one thing the chip
//! beside it structurally cannot say and is the whole of ADR-0218; and
//! [`crate::view::DeckHead::scrub`] answers `ScrubDeck` by an amount, which is
//! the one control on this panel that does not name a destination — because
//! the vocabulary has none for it to name.
//!
//! **So is the audio-in pill**, which is the same shape over a device:
//! [`crate::view::AudioInPill::ask`] answers *what does a press on it ask for*
//! off the same laid-out pill this rule hit-tests — the card down or up, or
//! `AttachBeatSource` naming the input that was picked. What it cannot answer
//! is whether that input is still there, and it does not try: a device that
//! has gone between the listing and the press is refused where it is opened.
//!
//! **So is the arrangement pill**, one row up and over a control with a menu
//! under it: [`crate::view::ArrangementPill::ask`] answers *what does a press
//! on it ask for* off the same laid-out pill this rule hit-tests — the menu
//! down or up, the reset, a save under the name in use, or a restore of the
//! name that was picked. It is the one of the nineteen whose answer is sometimes
//! not an operation at all, and that is the affordance and the vocabulary
//! staying apart rather than an exception: *open the menu* is not something a
//! MIDI map or an MCP call could ever want to say.
//!
//! **So is the Program bay's `solo`**, and it is the first control this crate
//! draws inside a bay head: [`crate::view::program_head`] answers *is this a
//! control* here and *what does a press on it ask for* — [`crate::panel::Op::Solo`]
//! of the picture, or [`crate::panel::Op::Unsolo`] where something is soloed
//! already — off the same head-pill
//! derivation that painted the capsule. It is the Outputs
//! dot's arrangement exactly, two operations and no toggle, on the console's
//! own shape rather than on the mix.
//!
//! **So are the four deck preview cells**, and they are the mixer bay's
//! arrangement rather than the dot's: [`crate::view::ProgramBay`] is derived
//! once and asked whether a point is on a cell and what that press asks for —
//! `SetPreview` naming the deck the cell is, or the mix where the press is on
//! the cell the output is already showing. Which cell is showing is handed in
//! per frame like every other reading here, because the deck is the model of
//! record for it.
//!
//! **Four questions, one derivation.** The mixer bay is laid out once per
//! event and asked for every control it has — a knob, a blend chip, a tally
//! chip and a mask mini are four questions about one laid-out strip, and a
//! second derivation would be a second answer that could disagree with the one
//! the frame drew.
//!
//! # One exception, and it is not a hole in the rule
//!
//! **The panel always learns where the pointer is**, whoever the event is
//! claimed by. That is not the panel *acting* on the event: every keyboard
//! operation is addressed to whatever the pointer is over — fold *this*,
//! solo *this* — so [`Panel::cursor`] is state the panel needs whether or not
//! it is dragging, and a panel that only tracked the pointer over its own
//! boundaries would fold the wrong region the moment the pointer was anywhere
//! useful.
//!
//! So motion updates [`Panel::cursor`] on every event and [`Claim`] decides
//! only whether `egui` is also told. A button or a wheel is exclusive: one of
//! the two, never both.
//!
//! # Deciding a release before it is performed
//!
//! [`Panel::released`] takes the drag out of hand, so [`claim`] on a release
//! has to be asked **first** — asked afterwards it would see no drag, route
//! the release to `egui`, and hand `egui` a button-up it never saw the
//! button-down for. That is rule 1 doing the work it exists for, and it is the
//! ordering a caller gets wrong.

use karakuri_layout::{Hit, Point};

use crate::panel::{Panel, GRAB};
use crate::view::{
    arrangement, audio_in, deck_head, inspector, look, master, mixer, outputs, program_bay,
    program_head, View,
};

/// **What each of rule 4's derivations answers for**, one entry per probe in
/// [`claim`] and in that order: the Outputs sink, the audio-in pill, the
/// arrangement pill, the look group's two, a strip's four, the Master bay's
/// one, a deck head's four, the Program bay head's `solo`, and the four deck
/// preview cells.
///
/// **It is a table and not a sentence because [`claim`] asks its probes out of
/// an array of exactly this length.** A derivation added to rule 4 without an
/// entry here does not compile, so [`CONTROLS`] is a sum over the probes that
/// are actually asked rather than a count somebody has to remember to raise.
///
/// **What it cannot see is a control added inside a derivation already here**:
/// a fifth chip on a strip is one more thing a press reaches, and `on_strip`
/// would go on answering for four. That one is caught where every other fact
/// about a control is — the clearance test the rule above says each one owes,
/// `tests/mask.rs` being the most recent of them — and this entry is what has
/// to be raised beside it.
const CLAIMS: [usize; 9] = [1, 1, 1, 2, 4, 1, 4, 1, 4];

/// **How many controls rule 4 hit-tests**, summed over [`CLAIMS`].
///
/// Exported because the answer to *what can the pointer press here* is this
/// crate's and nobody else's: `egui` owns no widget anywhere on the console,
/// so a caller has no other way to ask. `karakuri/src/main.rs` prints it in
/// its legend, where the sentence it replaced said the panel had three
/// controls and went on saying it while ten more landed.
pub const CONTROLS: usize = summed(&CLAIMS);

/// [`CLAIMS`] added up in a `const`, which `Iterator::sum` is not.
const fn summed(claims: &[usize]) -> usize {
    let mut total = 0;
    let mut at = 0;
    while at < claims.len() {
        total += claims[at];
        at += 1;
    }
    total
}

/// Who a pointer event belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Claim {
    /// The panel's: a boundary is under the pointer, or one is in hand.
    /// `egui` is not told.
    Panel,
    /// `egui`'s.
    Egui,
}

/// Who gets a pointer event at `p` — see the module documentation for the
/// rule and for why it is written there.
///
/// Takes `&mut Panel` for the solve alone: [`Layout::hit`] refuses to answer
/// from a dirty layout, and on a frame where nothing moved the solve is a flag
/// test.
///
/// **Takes the `egui` context to ask where the console's controls are.** Rule
/// 4 is about a painted chip whose width is the width of the name in it, so
/// answering it means laying that name out — which is `egui`'s to do and
/// nobody else's, since it is `egui` that will paint the same run. It is a
/// cached galley lookup per event, and before the first frame there are no
/// fonts and no drawn control at all, which
/// [`crate::view::outputs`] answers `None` to.
///
/// **What the mixer adds to that is two more galley lookups per strip**, for
/// the tally's word and the blend's, because those are what a strip's boxes
/// are laid out around. It is paid on a pointer event and not on a frame, and
/// a console with no deck behind it pays nothing at all — [`mixer`] answers
/// `None` to an empty slice before it asks for any type. **The chips cost
/// none of that again**: the bay is derived once and all four of the mixer's
/// controls are asked of it — and the mask mini adds no lookup of its own,
/// because it holds a mark rather than a word.
///
/// **A control in a bay that is not laid out is never reached**, and it is
/// [`mixer`] that answers so rather than a check here: a folded mixer — or one
/// inside a folded pane — has no room for its row of strips, `strips_row`
/// answers `None`, and the bay is `None` before any chip is hit-tested.
/// `tests/tally.rs` asserts it both ways round. [`inspector`] is the same
/// answer one bay along, and [`deck_head`] adds a second: a pane too narrow to
/// hold its own chips draws none, so there is nothing there to press.
///
/// **What the deck head adds is three galley lookups per pane**, for the
/// mode's word, the anchor's numbers and the fold's — the same arrangement the
/// mixer's are in, and paid on a pointer event rather than on a frame. The
/// scrub's two arrows add none of their own: they are marks rather than words,
/// which is what the mask mini already saves one bay up.
///
/// **And it takes the whole [`View`], for the same reason one level further
/// out.** A fader's knob sits on the fill's moving edge and the arrangement
/// pill is as wide as the name in it, so *where a control is* depends on what
/// the deck and the store said this frame — and this crate has neither
/// (ADR-0156), so the values arrive the way they arrive everywhere else here:
/// handed in by whoever owns them.
///
/// **The view rather than the four fields out of it**, which is a change from
/// when this took the strips alone. It is not convenience: rule 4 hit-tests
/// exactly what [`View::draw`] painted, and a caller that passed the strips
/// from one frame and the arrangement from another could put the two out of
/// step with nothing failing to compile. One argument is one frame's answer.
/// A [`View::new`] nobody has written to is a console with no deck and no
/// store behind it — no strips, no knobs, the default arrangement and a shut
/// menu — and it is what every test in this crate that is not about a control
/// passes.
pub fn claim(panel: &mut Panel, ctx: &egui::Context, view: &View, p: Point) -> Claim {
    // Rule 1, and it comes first: a gesture in progress is not re-decided from
    // where the pointer happens to be now.
    if panel.dragging() {
        return Claim::Panel;
    }
    panel.solve();
    // Rule 2: a menu that is down is a hand mid-choice, and every point of the
    // console is part of that gesture until it is shut. Before the boundary,
    // because the card is drawn across boundaries on purpose.
    // **Either card**, and they are two clauses of one rule rather than two
    // rules: a card that is down is a hand mid-choice whichever pill put it
    // there, and the next press is part of that gesture either way. They can
    // never both be down — the press that would open the second one lands
    // while the first is open, so this claims it and it shuts that one.
    if view.arrangement.open() || view.audio.as_ref().is_some_and(|audio| audio.open()) {
        return Claim::Panel;
    }
    // Rule 3 before rule 4: the boundary's first refusal is what the ordering
    // is, and the control clearing every grab is what stops it costing
    // anything. See the module documentation and `tests/outputs.rs`.
    match panel.layout().hit(p, GRAB) {
        Hit::Divider { .. } => Claim::Panel,
        // Rule 4, over all [`CONTROLS`] of the console's controls. Each is
        // asked the same way — the derivation that draws it, asked whether the
        // point is on it — and no answer is stored.
        Hit::View(_) | Hit::Nothing => {
            let on_sink = || outputs(ctx, panel.layout()).is_some_and(|row| row.hit(p));
            // **The two pills that are not in a bay**, and the only two asked
            // with their cards already known to be shut: rule 2 has answered
            // for the open case above, so these are the capsules alone.
            //
            // The audio-in pill is asked first because it is drawn first —
            // the arrangement pill is laid out from where it ends, so asking
            // in the other order would derive the second from the first
            // anyway.
            let on_audio = || {
                audio_in(ctx, panel.layout(), view.transport, view.audio.as_ref())
                    .is_some_and(|pill| pill.hit(p))
            };
            let on_pill = || {
                arrangement(
                    ctx,
                    panel.layout(),
                    view.transport,
                    view.audio.as_ref(),
                    &view.arrangement,
                )
                .is_some_and(|pill| pill.hit(p))
            };
            // **The bay is derived once for all of its controls**, since a
            // knob, a blend chip, a tally chip and a mask mini are four
            // questions about one laid-out strip.
            let on_strip = || {
                mixer(ctx, panel.layout(), &view.mixer).is_some_and(|bay| {
                    bay.grab(p).is_some()
                        || bay.blend(p).is_some()
                        || bay.tally(p).is_some()
                        || bay.mask(p).is_some()
                })
            };
            // The two at the end of the transport row, derived once for
            // both: the exposure track's place is measured from the tone map's
            // capsule, so they are two questions about one laid-out group.
            let on_look = || {
                look(
                    ctx,
                    panel.layout(),
                    view.transport,
                    view.audio.as_ref(),
                    &view.arrangement,
                    view.look,
                )
                .is_some_and(|row| row.owns(p))
            };
            // **The deck head's three, one pane at a time**, and each pane is
            // derived once for all of them exactly as a strip is: the anchor's
            // place is measured from the mode chip's and the arrows' from the
            // anchor's, so they are three questions about one laid-out pane. A
            // console with no deck behind it has no panes and pays nothing —
            // [`View::inspector`] is empty, and this iterates over nothing.
            // **The Master bay's one control**, and the bay is derived for it
            // exactly as the mixer's is for its four — one question about one
            // laid-out row here, because there is one thing in this bay a hand
            // can move. A console with no level behind it has no row at all
            // and pays nothing.
            let on_master =
                || master(ctx, panel.layout(), view.master_out).is_some_and(|row| row.owns(p));
            let on_deck_head = || {
                view.inspector.iter().enumerate().any(|(index, pane)| {
                    inspector(panel.layout(), index, pane)
                        .and_then(|at| deck_head(ctx, &at, pane))
                        .is_some_and(|head| head.owns(p))
                })
            };
            // **The one control this console has in a bay head**, and the only
            // one of the nineteen whose capsule a boundary's grab reaches —
            // `view::program_head` is where that 0.75 of a pixel is measured
            // and argued, and rule 3 above is what decides it.
            let on_solo = || program_head(ctx, panel.layout()).is_some_and(|head| head.hit(p));
            // **The four deck preview cells, derived once for all of them**,
            // which is a strip's arrangement one bay over: the cells are four
            // questions about one arranged Program bay, and a second
            // derivation would put the cells a press lands on in the other of
            // the two arrangements. It asks for no type at all — a cell is a
            // rectangle and a letter — so this is the cheapest probe here.
            let on_cells =
                || program_bay(panel.layout(), view.canvas).is_some_and(|bay| bay.owns(p));
            // **One probe per derivation, cheapest answer first**, and the
            // array is [`CLAIMS`]' length: a control reached through a
            // derivation that list does not have fails to compile here, which
            // is the whole of why [`CONTROLS`] cannot fall behind the rule.
            // `any` short-circuits exactly as the chain of `||` it replaced
            // did, so a press on the sink still costs one galley lookup.
            let probes: [&dyn Fn() -> bool; CLAIMS.len()] = [
                &on_sink,
                &on_audio,
                &on_pill,
                &on_look,
                &on_strip,
                &on_master,
                &on_deck_head,
                &on_solo,
                &on_cells,
            ];
            match probes.iter().any(|probe| probe()) {
                true => Claim::Panel,
                false => Claim::Egui,
            }
        }
    }
}
