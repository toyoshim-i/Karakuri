//! Who gets a pointer event: the panel's boundaries, or `egui`.
//!
//! The dividers are ours. [`Panel::press`], [`Panel::moved`] and
//! [`Panel::released`] move them over [`Layout::hit`], and `egui` has no widget
//! in those gaps — they are the ground showing between two bays, and nothing is
//! drawn there to be clicked. So on the face of it there is no conflict, and
//! the two would never both react to the same click.
//!
//! That is exactly why the rule is written down here rather than left to be
//! re-derived. It is true only while the gaps stay empty. [`GRAB`] widens every
//! boundary by six pixels either side, because a nine-pixel gap is not a target
//! a hand finds — and those twelve pixels are *inside the bays*, over whatever
//! the bay draws at its edge. The first control placed near a bay's edge is
//! under a boundary's grab, and then both do think they are dragging.
//!
//! # The first control is drawn, and it clears the grab
//!
//! The Outputs row's one sink ([`crate::view::outputs`]) is that control, and
//! the paragraph above is why it is measured rather than assumed. The row is 34
//! tall and the chip is 18.5, centred, so there is 7.75 of row above the chip
//! and 7.75 below, against a [`GRAB`] of 6: the boundary's band ends 1.75
//! pixels above the control and nothing overlaps.
//!
//! That is a fact about two constants and a rectangle, so it is a test and not
//! a sentence — `tests/outputs.rs`, which fails if the control moves up, if the
//! row gets shorter, or if [`GRAB`] widens. Widening the grab past 7.75 makes
//! the sink unclickable in its top edge and the rule below is what would have
//! to change, deliberately, rather than the control being nudged.
//!
//! # The rule
//!
//! The boundary gets first refusal.
//!
//! 1. A drag in hand keeps its claim, wherever the pointer has wandered to. A
//!    drag is a gesture and not a position: a boundary held against a stop while
//!    the pointer runs on across three bays is the ordinary case, not the odd one,
//!    and a claim re-decided from the pointer each event would hand the middle of
//!    that gesture to `egui`.
//! 2. An open menu keeps the pointer until it is shut — every point of the
//!    console, not only the menu's own card.
//!
//! This is rule 1 again rather than a second exception to rule 4, and it was
//! added with the arrangement pill because that is the first control here whose
//! gesture outlives the press that started it. A menu that is down is a hand
//! mid-choice exactly as a boundary in hand is a hand mid-drag, and the next
//! press is part of that gesture whichever way it ends: on a row it picks,
//! anywhere else it dismisses. Neither is `egui`'s and neither is a boundary's.
//!
//! It has to come before the boundary's first refusal, and that is the whole
//! reason it is a rule and not a clause of rule 4. The menu hangs out of the
//! transport row and down over the bays, so it crosses the boundary under that
//! row and whatever pane divider is beneath it — and under rule 4 those rows
//! would be dead, silently, in the middle of a list an operator is reading. The
//! clearance arithmetic that keeps the *pill* clickable cannot be done for a
//! card that is deliberately drawn across the panel; the honest answer is that
//! the card is modal while it is there, and the price is that a boundary cannot
//! be dragged with a menu open. Pressing it shuts the menu, and the second
//! press drags.
//!
//! See [`crate::view::Arrangement::open`] and [`crate::view::AudioIn::open`],
//! which are the whole of the condition, and `tests/arrangement_pill.rs` and
//! `tests/audio_in.rs`, which assert both halves for each — that a boundary
//! under the open card goes to the panel, and that it goes back to being an
//! ordinary boundary the moment the card is shut.
//!
//! Cards can be down and never two at once. The audio-in pill has one, the
//! Library bay's load pulldown has one and a row of its list has one, and this
//! clause is written over all of them: whichever is open claims the press that
//! would have opened another, and that press shuts it. So the second press
//! opens the second card, which is the same one-extra-press price a boundary
//! already pays.
//!
//! The row menu is the one a *secondary* press opens, and that changes nothing
//! here: this rule is about a card that is down rather than about what put it
//! there, so while it is down a press of either button is the card's. Which
//! button opened it is the caller's question and is answered in
//! `karakuri/src/main.rs`; this file has never known which button a press was,
//! because rules 1 to 4 are all about where the pointer is. 3. Otherwise, if
//! the pointer is within [`GRAB`] of a boundary, it is the panel's and `egui`
//! does not see the event. 4. Otherwise, if the pointer is on a control the
//! console draws, it is the panel's — because the console paints and `egui`
//! owns no widget anywhere in it, so a press routed to `egui` there reaches
//! nothing at all. The panel's controls are painted shapes, and the only thing
//! that knows a press landed on one is this rule.
//!
//! How many of them there are is [`CONTROLS`], summed over [`PROBES`], and that
//! is a number this crate exports rather than one this paragraph keeps: a
//! surface describing itself to an operator has to say what a pointer reaches,
//! and a sentence saying it is where the count goes stale. Which derivation
//! reaches which of them is [`PROBES`] too, one row apiece. What is written out
//! here is the *argument* for each — the clearance a control was measured to
//! and what a press on it means — which is a paragraph rather than a row and is
//! why this list stays: the Outputs row's sink ([`crate::view::outputs`]), a
//! mixer strip's fader knob ([`crate::view::Mixer::grab`]), its blend chip
//! ([`crate::view::Mixer::blend`]), its tally chip
//! ([`crate::view::Mixer::tally`]), its mask mini
//! ([`crate::view::Mixer::mask`]), the strip itself
//! ([`crate::view::Mixer::select`]), under the strips the transition row's
//! shape, quantum and length pills and the `go` capsule that runs one
//! ([`crate::view::TransitionRow`]), the transport row's audio-in pill
//! ([`crate::view::audio_in`]) and its arrangement pill
//! ([`crate::view::arrangement`]), at the end of that row the tone map's
//! capsule and the exposure track ([`crate::view::look`]) and the `rec` pill
//! past them ([`crate::view::TransportRow::record`]), in an inspector pane's
//! deck head the sync chip, the anchor, the scrub's two arrows and the fold at
//! the right of it ([`crate::view::deck_head`]), the Master bay's out
//! ([`crate::view::MasterRow::grab`]), the Program bay head's `solo`
//! ([`crate::view::program_head`]), the four deck preview cells under it
//! ([`crate::view::ProgramBay::preview`]), the Library bay's scope chips
//! ([`crate::view::LibraryBay::chip`]), the two filter fields under them
//! ([`crate::view::LibraryBay::filter`]), the `params` chip in that bay's foot
//! ([`crate::view::LibraryBay::read`]), the star at the left of each of its
//! rows ([`crate::view::LibraryBay::starred`]) and the list those stars are in
//! ([`crate::view::LibraryBay::take`], and [`crate::view::LibraryBay::land`]
//! where the rows are versions). The rule did not change to hold any of the
//! ones that came after the first, which is what it was written for — and each
//! is asked exactly the way the first is: the derivation that draws it, asked
//! whether the point is on it, with nothing stored.
//!
//! How many that is, is not written in this paragraph and must not be, which is
//! the rule two paragraphs up applied to this one: [`CONTROLS`] is *what the
//! pointer reaches here*, it moves the day a control lands, and a fixed number
//! in prose is a second answer that stops agreeing on that day. This sentence
//! said *the thirty-five that came after the first*, and it was wrong by nine
//! before anybody read it again. So the controls below are named rather than
//! numbered, and each paragraph is checkable on its own: what a control is,
//! what it clears, and which test measures it.
//!
//! The list is the first of them a press does not act on, and rule 4 did not
//! change for that either. A press on a row takes a Set in hand and asks for
//! nothing; what it asks for is decided at the release, over whatever strip the
//! pointer is then on — `console.html`'s *"Dragging a row onto a strip … names
//! both operands in the one gesture"*. So the press is claimed for the same
//! reason every other one here is: the console painted the row, `egui` owns no
//! widget on it, and a press routed to `egui` there reaches nothing at all. See
//! [`crate::view::LibraryBay::take`], [`crate::panel::Panel::carry`] and
//! [`crate::view::Mixer::dropped`], which is the destination and is asked at
//! the release rather than here.
//!
//! The `solo` capsule and the four preview cells are the first controls a
//! boundary's grab reaches, and the rule holds them unchanged for the reason it
//! holds everything else: rule 3 gives the boundary first refusal, so what the
//! overlap costs is the sliver of the control inside the band and never an
//! ambiguity about who is dragging. Both live where `docs/manual/console.html`
//! draws them, flush against a region's own edge: the `solo` capsule sits 5.25
//! into a 27-tall bay head whose top edge is the body row's, and a preview
//! cell's top edge *is* the `deck-previews` region's.
//! `crate::view::program_head` and `tests/preview_cells.rs` are where the two
//! numbers are measured, and changing either of them is a change to the console
//! page or to [`GRAB`] rather than a nudge.
//!
//! The arrangement pill was the first control in the transport row, which was
//! four readouts and nothing a press acted on until it landed
//! ([`crate::view::transport`]). It clears every boundary by more than any of
//! the five that came before it: the row is 48 and a `.pill` is 16.5, centred,
//! so there is (48 - 16.5) / 2 = 15.75 of row above it and 15.75 below, against
//! a [`GRAB`] of 6 — `tests/arrangement_pill.rs`, which is `tests/outputs.rs`'s
//! arithmetic over this control and fails the same three ways.
//!
//! The tone map's capsule and the exposure track are in that row too, and their
//! clearance is measured rather than inherited from it — which is this file's
//! rule about every control, and here it happens to come out at the same number
//! twice. The tone map's capsule is a `.pill`, so it is the pill's own 15.75.
//! The exposure control is a 5px track, and a 5px-tall target is not something
//! a hand finds — so what a press is tested against is the track grown to a
//! line's height, [`crate::view::LookRow::grip`], which is `.mini`'s padding
//! argument met with a band. That is 16.5 in a row of 48 and therefore 15.75 as
//! well. `tests/look.rs` measures both and fails the same three ways
//! `tests/arrangement_pill.rs` does; that the two agree is a fact about two
//! capsules being one height, not a number either of them inherited.
//!
//! A deck head's seven are the first controls that are not in a row of their
//! own, and they are measured off their own rectangles like everything else
//! here. A deck head is the second row *inside* an inspector pane, so the
//! nearest boundary is not the one under the row it sits in — it is the pane
//! divider down the side of the pane, and what holds the chips off it is
//! `.deck-head`'s own padding ([`crate::view::size::DECK_HEAD_PAD_X`]): the
//! leftmost chip starts 10 in from the pane's edge, and the fold ends 10 in
//! from the other, against a [`GRAB`] of 6.
//!
//! Down the row the clearance is the largest on the console, and it is a sum
//! rather than a centring: the bay head is painted over the top of the region
//! and the pane's own `.half-head` is under it, so there is 27 + 27.5 + 5 =
//! 59.5 of pane above the chips and more below. The mask mini's 74.50 was the
//! previous largest and it is the same shape of number — a control several rows
//! into a bay.
//!
//! 10 is not the tightest on the console; the blend chip's 8.97 still is. It is
//! the tightest in this bay, and the two would go together if the grab were
//! widened past 8.97. `tests/deck_head.rs` measures all four controls against
//! every boundary and fails the same three ways `tests/arrangement_pill.rs`
//! does.
//!
//! The Master bay's out was measured the same way off its own rectangle. The
//! knob is 11 tall on a row of 16.5 that starts [`crate::room::size::HEAD_H`]
//! plus [`crate::room::size::MASTER_PAD_TOP`] below the bay's top edge, so what
//! holds it off the boundary above is a sum and not a centring: 27 + 8 + 2.75 =
//! 37.75, which is the second largest on the console after the deck head's 59.5
//! and for the same reason — a control several rows into a bay. Down the sides
//! it is `.master-body`'s padding that holds it, and the knob overhangs its
//! track by half its own width: 10 of padding plus the `out` label and
//! [`crate::room::size::MASTER_GAP`] on the left, 10 plus the figure and the
//! same gap on the right, less 4.5 of overhang either way. The binding number
//! is the one below, which is whatever is left of the bay under the row — the
//! bay is `flex: 1` in its column, so it is measured rather than computed.
//! `tests/master.rs` measures all four against every boundary and fails the
//! same three ways `tests/arrangement_pill.rs` does.
//!
//! Three of the four are claimed and the fourth is a mode away from being,
//! which is this rule's *a control claims what it acts on and no more* met by a
//! state rather than by a rectangle: a scrub arrow is inert on a deck that is
//! not beat-synced, so it is drawn, it keeps its shape, and
//! [`crate::view::DeckHead::owns`] does not claim it. The `composite` chip
//! beside them is never claimed at all — layering is a build decision and there
//! is no operation for the press to name.
//!
//! The fourth is the first control with a state that can be pending, and it is
//! still only an affordance: it names a destination and refuses nothing,
//! because what may be asked belongs where the record is applied
//! ([P-0090](../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
//! So this rule holds it unchanged too.
//!
//! The fifth is the first control that carries a value it does not draw, and
//! this rule does not change for that either: what a press *asks for* is
//! [`crate::view::Mixer::mask`]'s, and whether a press landed on the chip is
//! one rectangle either way
//! ([ADR-0203](../../../docs/adr/0203-the-mask-chip-carries-the-angle-it-does-not-control.md)).
//!
//! The Library bay's scope chips are the first controls here whose *number* is
//! a value rather than a constant — one per scope the host handed the bay,
//! which is why [`PROBES`]' row for them counts [`Scope::ALL`] rather than
//! naming a four. Their clearance is a sum and not a centring, which is the
//! Master bay's out's shape one column over: `.scopes` is drawn under the bay
//! head, so what holds the chips off the boundary above — the one between the
//! transport row and the body — is [`crate::room::size::HEAD_H`] plus
//! [`crate::room::size::SCOPES_PAD_Y`], 27 + 7 = 34, against a [`GRAB`] of 6.
//! Down the left it is [`crate::room::size::SCOPES_PAD_X`]'s 9 off an edge that
//! is the viewport's rather than a divider's, and below them the library's own
//! list runs on for at least a hundred pixels before the staging lane's
//! boundary.
//!
//! The chip at the end of the row is the one that overruns, and it is the
//! plainest case of what rule 3 costs anywhere on this console: the row clips,
//! the five words laid end to end are wider than the mock's 218-wide pane, and
//! one of them therefore starts inside the bay and finishes outside it — which
//! one depends on the width, so `tests/library.rs` looks for the chip that
//! straddles the row's right edge rather than naming it. What is *drawn* of it
//! stops at the bay's edge, and the last six pixels of that are the pane
//! divider's under rule 3 — so a press there drags the boundary, exactly as it
//! does on the `solo` capsule and a preview cell. What brings the whole chip in
//! is widening the pane, which that boundary allows and no maximum stops.
//! `tests/library.rs` measures both halves.
//!
//! The `params` chip in the Library bay's foot is the newest of them, and its
//! clearance is a centring rather than a sum: `.lib-foot` is
//! [`crate::room::size::LIB_FOOT_H`]'s 26 tall and a `.pill` is
//! [`crate::room::size::PILL_H`]'s 16.5, centred, so there is 4.75 of row above
//! the capsule and 4.75 below it — and the foot's bottom edge is the bay's,
//! which is a boundary. 4.75 against a [`GRAB`] of 6 is the third control on
//! this console whose rim a boundary takes, after the `solo` capsule and a
//! preview cell, and it is rule 3's ordinary price rather than anything about
//! this chip: everything from the capsule's middle upwards is the panel's.
//! Along the row it clears everything — [`crate::room::size::LIB_FOOT_GAP`]'s 8
//! to the `load` button on its right, and the whole of the foot's leftover to
//! the count on its left. `tests/library.rs` measures it, and it is measured
//! rather than inherited from the capsules it sits beside, which stand at the
//! same 4.75 and pay the same rim.
//!
//! One of them is a whole strip, and it is the first control here that is not
//! drawn as one. [`crate::view::Mixer::select`] is the strip's own rectangle,
//! so what a press on it means — *address the keys to this deck* — is offered
//! by the column rather than by a capsule, and `console.html`'s strip tip says
//! so in those terms: *"Two ways into that selection and neither of them is a
//! control: 0 to 3, or a press anywhere on this strip that no knob under the
//! pointer claimed."* It is the last of the strip's five questions for that
//! reason and not by preference: the four inside the column are asked first,
//! and this is what is left over. It landed in the rule late, and the four
//! things that already described it — the operations page's `has` badge, the
//! tip above, [`crate::view::Mixer::select`] and the press arm in
//! `karakuri/src/main.rs` that asks it — were all true while this probe was
//! missing, so a press on a strip's ground fell through to `egui` and reached
//! nothing at all.
//!
//! A control claims what it acts on and no more. A fader's *track* is drawn by
//! the console and moves no value, because a press off the knob must not jump
//! it — see [`crate::view::Mixer::grab`]. That is a statement about the *fader*
//! and not about the claim, and the strip above is what keeps the two apart: a
//! press on the track is the panel's, it selects the deck the track is on, and
//! the level does not move. `console.html` states both halves and in that order
//! — *"a press on the track off the knob does nothing"* on the fader, *"a press
//! anywhere on this strip that no knob under the pointer claimed"* on the
//! column around it. Where nothing at all is offered the rule still declines:
//! the Master bay's out is one fader in a bay with no selection in it, so its
//! track is claimed by nobody, and `egui` owns nothing there either — the two
//! answers are the same nothing. 5. Otherwise it goes to
//! [`egui_winit::State::on_window_event`](https://docs.rs/egui-winit) and
//! `egui` decides.
//!
//! Rule 3 before rule 4 is *first refusal* meant literally: a control under a
//! boundary's grab would be dead, and it is the test above that keeps the order
//! from ever mattering. The mixer's knobs are measured the same way and in the
//! same file's spirit — `tests/fader.rs` asserts that no knob in the bay is
//! within [`GRAB`] of any boundary, and carries the same guard on itself, so
//! the ordering goes on costing nothing there too. The blend chip is measured a
//! third time in `tests/blend.rs`, the tally chip a fourth in `tests/tally.rs`
//! and the mask mini a fifth in `tests/mask.rs`, each with its own guard:
//! whether a control clears every boundary's band is two constants and a
//! rectangle, and it is never inherited from the control above it. No two of
//! the three chips are the same number — the nearest boundary to all of them is
//! the pane divider down the left of the bay, and the 8.97 the blend chip
//! clears it by is the mode row's width, where the tally's 14.23 is its own
//! capsule's. The mask mini shares the blend chip's row and clears the same
//! divider by 41.03, because it sits at the far end of that row — and it is the
//! one chip whose *nearest boundary is not the same one on every strip*: on the
//! three strips that are not against the pane, the nearest is the boundary
//! under the bay, 74.50 below the mode row. So the number that binds is 41.03
//! and it was measured rather than inherited from the chip 3px to its left.
//!
//! # A caller acts on the control, and it asks the same question again
//!
//! [`claim`] says *the panel's*; it does not say *the dot's*. What a press on
//! the control does is [`crate::view::Outputs::op`] — one operation, named —
//! and the caller asks [`crate::view::outputs`] for it. That is the same one
//! derivation this rule hit-tests, asked a second time rather than copied, so
//! the chip that claims a press and the chip that acts on it cannot come apart.
//! A fader is the same arrangement: [`crate::view::Mixer::grab`] answers *is
//! this a control* here and *which control, and where along it* to the caller,
//! off one derivation and one set of values. So is the blend chip:
//! [`crate::view::Mixer::blend`] answers *is this a control* and *what does a
//! press on it ask for* — `SetBlendMode` naming the mode after the one the deck
//! reports — off the same laid-out strip this rule hit-tests. So is the tally
//! chip: [`crate::view::Mixer::tally`] answers the same two with `SetResidency`
//! naming the residency after the one that was requested, which is the whole of
//! ADR-0195 and is why a press on a parked chip asks for the withdrawal of its
//! own prime request.
//!
//! So is the mask mini: [`crate::view::Mixer::mask`] answers *is this a
//! control* and *what does a press on it ask for* — `SetMaskShape` naming the
//! shape after the one the deck reports, and the angle the deck reports beside
//! it, which is the whole of ADR-0203 and is why choosing a shape does not
//! straighten a diagonal front.
//!
//! So is the strip the four of them sit in, and it is the one here where *is
//! this a control* is the whole column: [`crate::view::Mixer::select`] answers
//! both questions off the same `deck_at` the drop reads — `SelectDeck` naming
//! the deck whose rectangle the point is in. It is asked after the four above
//! on both sides, in this rule and again in the caller, because a press on a
//! knob is a press on that knob and this is what is left over.
//!
//! So is the Master bay's out, and it is [`crate::view::Mixer::grab`]'s
//! arrangement rather than the look's: [`crate::view::MasterRow::grab`] answers
//! *is this a control* here and *what is in hand, and where along it* to the
//! caller, off one derivation and one level. A press on its track is not
//! claimed, for the reason a strip's is not — the mock draws a handle on this
//! fader and draws none on the exposure track, and a handle that jumped to the
//! pointer would be a lie about what a handle is.
//!
//! So are the two look controls, at the far end of the same row:
//! [`crate::view::LookRow::tonemap`] answers `SetTonemap` naming the operator
//! after the one that is running, which is the blend chip's affordance over a
//! closed list of four (ADR-0187); and [`crate::view::LookRow::exposure`]
//! answers `SetExposure` naming where along the track the press landed, which
//! is the one control on this panel that a press sets outright. That last is
//! deliberately not [`crate::view::Mixer::grab`]'s rule about a press on a
//! track, and the reason is written at the method: a fader has a knob and a
//! value that must not jump under a hand mid-gesture, and this has neither.
//!
//! So are the deck head's six, two bays down and reached the same way:
//! [`crate::view::DeckHead::sync`] answers `SetSync` naming the mode the cycle
//! arrived at — with a mode this deck's material cannot honour skipped rather
//! than offered, which is the blend chip's affordance over a list one reading
//! has narrowed; [`crate::view::DeckHead::reanchor`] answers `SetSync` naming
//! the mode the deck is already in, which is the one thing the chip beside it
//! structurally cannot say and is the whole of ADR-0218; and
//! [`crate::view::DeckHead::scrub`] answers `ScrubDeck` by an amount, which is
//! the one control on this panel that does not name a destination — because the
//! vocabulary has none for it to name. The other three are the row's build
//! chips and are one shape: [`crate::view::DeckHead::compositing`] answers
//! `SetCompositing` naming the layering the deck is *not* in,
//! [`crate::view::DeckHead::resized`] answers `SetProperty` naming the element
//! count its step arrived at, and [`crate::view::DeckHead::re_salted`] answers
//! `SetProperty` naming the salt it was handed — each one field of the aim the
//! slot's watcher is pointed at, and none of them a step, a flip or an *again*
//! (ADR-0314, ADR-0328).
//!
//! So is the audio-in pill, which is the same shape over a device:
//! [`crate::view::AudioInPill::ask`] answers *what does a press on it ask for*
//! off the same laid-out pill this rule hit-tests — the card down or up, or
//! `AttachBeatSource` naming the input that was picked. What it cannot answer
//! is whether that input is still there, and it does not try: a device that has
//! gone between the listing and the press is refused where it is opened.
//!
//! So is the arrangement pill, one row up and over a control with a menu under
//! it: [`crate::view::ArrangementPill::ask`] answers *what does a press on it
//! ask for* off the same laid-out pill this rule hit-tests — the menu down or
//! up, the reset, a save under the name in use, or a restore of the name that
//! was picked. It is one of the two whose answer is sometimes not an operation
//! at all, and that is the affordance and the vocabulary staying apart rather
//! than an exception: *open the menu* is not something a MIDI map or an MCP
//! call could ever want to say.
//!
//! So is the Program bay's `solo`, and it is the first control this crate draws
//! inside a bay head: [`crate::view::program_head`] answers *is this a control*
//! here and *what does a press on it ask for* — [`crate::panel::Op::Solo`] of
//! the picture, or [`crate::panel::Op::Unsolo`] where something is soloed
//! already — off the same head-pill derivation that painted the capsule. It is
//! the Outputs dot's arrangement exactly, two operations and no toggle, on the
//! console's own shape rather than on the mix.
//!
//! So are the four deck preview cells, and they are the one place in this file
//! where *is this a control* and *what does a press on it ask for* have come
//! apart: [`crate::view::ProgramBay`] is derived once and asked whether a point
//! is on a cell, and the answer to the second question is now nothing at all. A
//! press used to name `SetPreview` — the deck the cell is, or the mix where the
//! press was on the cell the output was already showing — and ADR-0240 retired
//! that operation, leaving the picture as the master mix and every cell as its
//! own deck's monitor. The cells are still claimed, because a control claims
//! what it is drawn over: the row's top edge is a boundary an operator drags,
//! and a cell that stopped answering this rule would hand `egui` a press on the
//! panel's own face.
//!
//! And so are the four class pills — [`crate::view::mcp_pill`], one per class
//! of operations a model may be refused, three of them in a bay head beside
//! `solo` and the fourth beside the word that stands in for one in the Outputs
//! row. They are the only controls here that answer a press with neither an
//! [`crate::panel::Op`] nor an [`Operation`](karakuri_operation::Operation),
//! and ADR-0236 is why: what they set is configuration of the *map* — the layer
//! every surface reaches the vocabulary through — and a setting deciding
//! whether a surface may reach a class of operations cannot itself be one of
//! those operations. So the derivation hands back a value and the program
//! holding the run's opening writes it; this rule's only business with them is
//! that a press on one is the panel's and not `egui`'s, which is the same
//! business it has with the other twenty-eight.
//!
//! And so are the Library bay's scope chips, which are the four preview cells'
//! arrangement one column over: the bay is derived once and walked once for
//! every chip in it, because a chip is as wide as the word in it and where the
//! last one is depends on the ones before it. [`crate::view::LibraryBay::chip`]
//! answers *is this a control* and *what does a press on it ask for* —
//! [`crate::view::Chosen`], which is the chip the pointer landed on and the
//! operation beside it. It is the one answer on this panel that carries a value
//! the operation could not, and that is `Operation::SelectScope`'s payload
//! being `Undecided` on purpose rather than a gap: a press is the first thing
//! in this workspace that knows which member of a growable list it means, and
//! whether that settles the payload is a decision about the vocabulary rather
//! than about this rule.
//!
//! The fifth chip asks a different row, and it is the same division read once
//! more: `history` is not a library of Sets, so a press on it names
//! `Operation::WalkHistory` where the four beside it name `SelectScope`
//! ([`crate::view::Chosen`], ADR-0308). One rectangle, one press, and the row
//! it names is the row that describes what was asked for.
//!
//! The chip does not cycle where the key does. `e` steps to the next scope and
//! wraps, because a bare press cannot say *which*; a pointer press can, so it
//! names the chip it landed on and no arithmetic happens anywhere. That is
//! P-0090's division met by two surfaces rather than an inconsistency between
//! them.
//!
//! The two filter fields one row under them step where the chips name, and that
//! is the same division read the other way round. A chip is one of a row of
//! capsules and a pointer lands on exactly one, so it names; a field is one box
//! standing for a list of values, so a press on it can only move along that
//! list — which is `e`'s arithmetic at a pointer, and P-0090 forbids neither.
//! What it forbids is an operation that says *step*, and
//! [`crate::view::LibraryBay::filter`] emits none: what leaves is
//! `Operation::ListSets` naming both filters as they will stand.
//!
//! The star at the left of each row names a state and not a step, which is the
//! chips' half of that division reached from a third direction: a toggle is
//! what the *vocabulary* refuses (`Operation::SetFavourite` carries the state a
//! row is being put in), so the control is what reads the row's present mark
//! and asks for the other one — [`crate::view::LibraryBay::starred`]. It is
//! asked before the row it sits in, because a star is inside a row and rule 4's
//! *a control claims what it acts on and no more* is what puts the smaller box
//! first.
//!
//! And so is the list under them, except that what it hands back is not an
//! answer to *what does a press ask for* at all.
//! [`crate::view::LibraryBay::take`] answers *is this a control* here and
//! *which Set is now in hand* to the caller — a `Taken`, off the same laid-out
//! bay this rule hit-tests. It is the one offer on this console whose press
//! names no operation — under the four scopes whose rows are Sets, which is the
//! qualification ADR-0308 added: under `history` a row is a version and a press
//! on it is [`crate::view::LibraryBay::land`], which names
//! `Operation::RestoreProcedure` outright. The two are one row of [`PROBES`]
//! and are told apart by which listing goes in with the point — `View::sets` is
//! empty under that scope and `View::versions` is empty under every other — so
//! exactly one of them can answer. That a carry names nothing is the gesture
//! rather than a gap: a load names a Set and a deck, the row is the first of
//! the two, and the second is whatever strip the pointer is over when the
//! button comes up ([`crate::view::Mixer::dropped`], asked at the release). The
//! two halves are one derivation each and neither is stored, which is this
//! section's rule arriving at a gesture that outlives its press — rule 1 is
//! what keeps the middle of it off `egui`, unchanged, exactly as it did for the
//! fader.
//!
//! They are the one control here a boundary's grab does not reach and the chips
//! beside them do. `.lib-filters` is `padding: 6px 9px` inside the bay, so a
//! field is 9 off each side edge against a [`GRAB`] of 6, where the chip at the
//! end of the scope row overruns the bay entirely. The row above and the list
//! below are both the bay's own, so there is no boundary up or down either.
//!
//! Five questions, one derivation. The mixer bay is laid out once per event and
//! asked for every control it has — a knob, a blend chip, a tally chip, a mask
//! mini and the strip they sit in are five questions about one laid-out strip,
//! and a second derivation would be a second answer that could disagree with
//! the one the frame drew.
//!
//! # One exception, and it is not a hole in the rule
//!
//! The panel always learns where the pointer is, whoever the event is claimed
//! by. That is not the panel *acting* on the event: every keyboard operation is
//! addressed to whatever the pointer is over — fold *this*, solo *this* — so
//! [`Panel::cursor`] is state the panel needs whether or not it is dragging,
//! and a panel that only tracked the pointer over its own boundaries would fold
//! the wrong region the moment the pointer was anywhere useful.
//!
//! So motion updates [`Panel::cursor`] on every event and [`Claim`] decides
//! only whether `egui` is also told. A button or a wheel is exclusive: one of
//! the two, never both.
//!
//! # The wheel is routed by region and not by control
//!
//! [`wheeled`] is the fifth rule's shape for the one event that is not a press,
//! and it is a separate function rather than a row of [`PROBES`] because the
//! two questions are different ones. A press asks *is the pointer on a
//! control*; a wheel asks *whose region is the pointer in*, and the Inspector's
//! panes are the only regions on this console that answer yes — a pane's body
//! scrolls under its two heads
//! ([ADR-0307](../../../docs/adr/0307-the-inspectors-pane-scrolls-and-the-position-is-the-panes-own.md)).
//!
//! A row in [`PROBES`] would have been wrong twice. [`CONTROLS`] is printed to
//! an operator as *what the pointer reaches here*, and a pane's body is not
//! something a press reaches — nothing happens when you click it. And the
//! probes are asked on a press as well as on a wheel, so a row here would claim
//! every press on a pane's ground and hand it to an arm that has nothing to do
//! with it.
//!
//! Rules 1 and 2 hold over it unchanged, and it asks them itself: a drag in
//! hand keeps the gesture — a wheel in the middle of a boundary drag scrolls
//! nothing and is still withheld from `egui` by [`claim`] — and a card that is
//! down is a hand mid-choice, so the wheel is not the pane's while one is open.
//! Rule 3 has nothing to say: a boundary's grab is about where a press lands,
//! and a wheel does not take hold of anything.
//!
//! # Deciding a release before it is performed
//!
//! [`Panel::released`] takes the drag out of hand, so [`claim`] on a release
//! has to be asked first — asked afterwards it would see no drag, route the
//! release to `egui`, and hand `egui` a button-up it never saw the button-down
//! for. That is rule 1 doing the work it exists for, and it is the ordering a
//! caller gets wrong.

mod claim;
mod handlers;
mod probes;
mod wheel;

pub use claim::{claim, Claim};
pub use probes::{Probe, CONTROLS, PROBES};
pub use wheel::{wheeled, Turned};
