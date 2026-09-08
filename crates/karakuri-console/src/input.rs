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
//!    **How many of them there are is [`CONTROLS`]**, summed over [`PROBES`],
//!    and that is a number this crate exports rather than one this paragraph
//!    keeps: a surface describing itself to an operator has to say what a
//!    pointer reaches, and a sentence saying it is where the count goes stale.
//!    **Which derivation reaches which of them is [`PROBES`] too**, one row
//!    apiece. What is written out here is the *argument* for each — the
//!    clearance a control was measured to and what a press on it means — which
//!    is a paragraph rather than a row and is why this list stays: the Outputs
//!    row's sink
//!    ([`crate::view::outputs`]), a mixer strip's fader knob
//!    ([`crate::view::Mixer::grab`]), its blend chip
//!    ([`crate::view::Mixer::blend`]), its tally chip
//!    ([`crate::view::Mixer::tally`]), its mask mini
//!    ([`crate::view::Mixer::mask`]), the **strip itself**
//!    ([`crate::view::Mixer::select`]), under the strips the transition row's
//!    shape, quantum and length pills and the `go` capsule that runs one
//!    ([`crate::view::TransitionRow`]), the
//!    transport row's audio-in pill
//!    ([`crate::view::audio_in`]) and its arrangement pill
//!    ([`crate::view::arrangement`]), at the end of that row the tone map's
//!    capsule and the exposure track ([`crate::view::look`]), in an
//!    inspector pane's deck head the sync chip, the anchor and the scrub's two
//!    arrows ([`crate::view::deck_head`]), the Master bay's out
//!    ([`crate::view::MasterRow::grab`]), the Program bay head's `solo`
//!    ([`crate::view::program_head`]), the four deck preview cells under it
//!    ([`crate::view::ProgramBay::preview`]), the Library bay's scope chips
//!    ([`crate::view::LibraryBay::chip`]), the two filter fields under them
//!    ([`crate::view::LibraryBay::filter`]), the `read` chip in that bay's
//!    foot ([`crate::view::LibraryBay::read`]) and the **list** between them
//!    ([`crate::view::LibraryBay::take`]). The rule did not change to hold
//!    any of the thirty-five that came after the first, which is what it was
//!    written for — and each is asked exactly the way the first is: the
//!    derivation that draws it, asked whether the point is on it, with nothing
//!    stored.
//!
//!    **The list is the first of them a press does not act on**, and rule 4 did
//!    not change for that either. A press on a row takes a Set in hand and asks
//!    for nothing; what it asks for is decided at the release, over whatever
//!    strip the pointer is then on — `console.html`'s *"Dragging a row onto a
//!    strip … names both operands in the one gesture"*. So the press is claimed
//!    for the same reason every other one here is: the console painted the row,
//!    `egui` owns no widget on it, and a press routed to `egui` there reaches
//!    nothing at all. See [`crate::view::LibraryBay::take`],
//!    [`crate::panel::Panel::carry`] and
//!    [`crate::view::Mixer::dropped`], which is the destination and is asked at
//!    the release rather than here.
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
//!    twenty-seven agree is a fact about two capsules being one height, not a
//!    number
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
//!    ([P-0090](../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
//!    So this rule holds it unchanged too.
//!
//!    **The fifth is the first control that carries a value it does not
//!    draw**, and this rule does not change for that either: what a press
//!    *asks for* is [`crate::view::Mixer::mask`]'s, and whether a press landed
//!    on the chip is one rectangle either way
//!    ([ADR-0203](../../../docs/adr/0203-the-mask-chip-carries-the-angle-it-does-not-control.md)).
//!
//!    **The twenty-fourth to the twenty-seventh are the Library bay's scope
//!    chips**, and they are the first controls here whose *number* is a value
//!    rather than a constant — one per scope the host handed the bay, which is
//!    why [`PROBES`]' row for them counts [`Scope::ALL`] rather than
//!    naming a four. Their clearance is a sum and not a centring, which is the
//!    Master bay's out's shape one column over: `.scopes` is drawn under the
//!    bay head, so what holds the chips off the boundary **above** — the one
//!    between the transport row and the body — is
//!    [`crate::room::size::HEAD_H`] plus [`crate::room::size::SCOPES_PAD_Y`],
//!    27 + 7 = **34**, against a [`GRAB`] of 6. Down
//!    the left it is [`crate::room::size::SCOPES_PAD_X`]'s **9** off an edge
//!    that is the viewport's rather than a divider's, and below them the
//!    library's own list runs on for at least a hundred pixels before the
//!    staging lane's boundary.
//!
//!    **The chip at the end of the row is the one that overruns**, and it is
//!    the plainest case of what rule 3 costs anywhere on this console: the row
//!    clips, four words laid end to end are wider than the mock's 218-wide
//!    pane, and `folder` therefore starts inside the bay and finishes outside
//!    it. What is *drawn* of it stops at the bay's edge, and the last six
//!    pixels of that are the pane divider's under rule 3 — so a press there
//!    drags the boundary, exactly as it does on the `solo` capsule and a
//!    preview cell. **What brings the whole chip in is widening the pane**,
//!    which that boundary allows and no maximum stops.
//!    `tests/library.rs` measures both halves.
//!
//!    **The `read` chip in the Library bay's foot is the newest of them**, and
//!    its clearance is a centring rather than a sum: `.lib-foot` is
//!    [`crate::room::size::LIB_FOOT_H`]'s 26 tall and a `.pill` is
//!    [`crate::room::size::PILL_H`]'s 16.5, centred, so there is **4.75** of
//!    row above the capsule and 4.75 below it — and the foot's bottom edge is
//!    the bay's, which is a boundary. **4.75 against a [`GRAB`] of 6 is the
//!    third control on this console whose rim a boundary takes**, after the
//!    `solo` capsule and a preview cell, and it is rule 3's ordinary price
//!    rather than anything about this chip: everything from the capsule's
//!    middle upwards is the panel's. Along the row it clears everything —
//!    [`crate::room::size::LIB_FOOT_GAP`]'s 8 to the load pill on its right,
//!    and the whole of the foot's leftover to the count on its left.
//!    `tests/library.rs` measures it, and it is measured rather than inherited
//!    from the load pill it sits beside, which happens to stand at the same
//!    4.75 and is not a control at all.
//!
//!    **The thirty-sixth is a whole strip, and it is the first control here
//!    that is not drawn as one.** [`crate::view::Mixer::select`] is the
//!    strip's own rectangle, so what a press on it means — *address the keys
//!    to this deck* — is offered by the column rather than by a capsule, and
//!    `console.html`'s strip tip says so in those terms: *"Two ways into that
//!    selection and neither of them is a control: 0 to 3, or a press anywhere
//!    on this strip that no knob under the pointer claimed."* It is the last
//!    of the strip's five questions for that reason and not by preference: the
//!    four inside the column are asked first, and this is what is left over.
//!    **It landed in the rule late**, and the four things that already
//!    described it — the operations page's `has` badge, the tip above,
//!    [`crate::view::Mixer::select`] and the press arm in `karakuri/src/main.rs`
//!    that asks it — were all true while this probe was missing, so a press on
//!    a strip's ground fell through to `egui` and reached nothing at all.
//!
//!    **A control claims what it acts on and no more.** A fader's *track* is
//!    drawn by the console and moves no value, because a press off the knob
//!    must not jump it — see [`crate::view::Mixer::grab`]. That is a statement
//!    about the *fader* and not about the claim, and the strip above is what
//!    keeps the two apart: a press on the track is the panel's, it selects the
//!    deck the track is on, and the level does not move. `console.html` states
//!    both halves and in that order — *"a press on the track off the knob does
//!    nothing"* on the fader, *"a press anywhere on this strip that no knob
//!    under the pointer claimed"* on the column around it. Where nothing at
//!    all is offered the rule still declines: the Master bay's out is one
//!    fader in a bay with no selection in it, so its track is claimed by
//!    nobody, and `egui` owns nothing there either — the two answers are the
//!    same nothing.
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
//! **So is the strip the four of them sit in**, and it is the one here where
//! *is this a control* is the whole column:
//! [`crate::view::Mixer::select`] answers both questions off the same
//! `deck_at` the drop reads — `SelectDeck` naming the deck whose rectangle the
//! point is in. It is asked **after** the four above on both sides, in this
//! rule and again in the caller, because a press on a knob is a press on that
//! knob and this is what is left over.
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
//! name that was picked. It is one of the two whose answer is sometimes
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
//! **So are the four deck preview cells**, and they are the one place in this
//! file where *is this a control* and *what does a press on it ask for* have
//! come apart: [`crate::view::ProgramBay`] is derived once and asked whether a
//! point is on a cell, and the answer to the second question is now nothing at
//! all. A press used to name `SetPreview` — the deck the cell is, or the mix
//! where the press was on the cell the output was already showing — and
//! ADR-0240 retired that operation, leaving the picture as the master mix and
//! every cell as its own deck's monitor. **The cells are still claimed**,
//! because a control claims what it is drawn over: the row's top edge is a
//! boundary an operator drags, and a cell that stopped answering this rule
//! would hand `egui` a press on the panel's own face.
//!
//! **And so are the four class pills** — [`crate::view::mcp_pill`], one per
//! class of operations a model may be refused, three of them in a bay head
//! beside `solo` and the fourth beside the word that stands in for one in the
//! Outputs row. **They are the only controls here that answer a press with
//! neither an [`crate::panel::Op`] nor an
//! [`Operation`](karakuri_operation::Operation)**, and ADR-0236 is why: what
//! they set is configuration of the *map* — the layer every surface reaches the
//! vocabulary through — and a setting deciding whether a surface may reach a
//! class of operations cannot itself be one of those operations. So the
//! derivation hands back a value and the program holding the run's opening
//! writes it; this rule's only business with them is that a press on one is the
//! panel's and not `egui`'s, which is the same business it has with the other
//! twenty-eight.
//!
//! **And so are the Library bay's scope chips**, which are the four preview
//! cells' arrangement one column over: the bay is derived once and walked once
//! for every chip in it, because a chip is as wide as the word in it and where
//! the fourth one is depends on the first three.
//! [`crate::view::LibraryBay::chip`] answers *is this a control* and *what
//! does a press on it ask for* — [`crate::view::Chosen`], which is the chip
//! the pointer landed on **and** `Operation::SelectScope` beside it. It is the
//! one answer on this panel that carries a value the operation could not, and
//! that is `SelectScope`'s payload being `Undecided` on purpose rather than a
//! gap: a press is the first thing in this workspace that knows which member
//! of a growable list it means, and whether that settles the payload is a
//! decision about the vocabulary rather than about this rule.
//!
//! **The chip does not cycle where the key does.** `e` steps to the next scope
//! and wraps, because a bare press cannot say *which*; a pointer press can, so
//! it names the chip it landed on and no arithmetic happens anywhere. That is
//! P-0090's division met by two surfaces rather than an inconsistency between
//! them.
//!
//! **The two filter fields one row under them step where the chips name**, and
//! that is the same division read the other way round. A chip is one of a row
//! of capsules and a pointer lands on exactly one, so it names; a field is one
//! box standing for a list of values, so a press on it can only move along that
//! list — which is `e`'s arithmetic at a pointer, and P-0090 forbids neither.
//! What it forbids is an operation that says *step*, and
//! [`crate::view::LibraryBay::filter`] emits none: what leaves is
//! `Operation::ListSets` naming both filters as they will stand.
//!
//! **And so is the list under them, except that what it hands back is not an
//! answer to *what does a press ask for* at all.**
//! [`crate::view::LibraryBay::take`] answers *is this a control* here and
//! *which Set is now in hand* to the caller — a `Taken`, off the same laid-out
//! bay this rule hit-tests. It is the one offer on this console whose press
//! names no operation, and that is the gesture rather than a gap: a load names
//! a Set **and** a deck, the row is the first of the two, and the second is
//! whatever strip the pointer is over when the button comes up
//! ([`crate::view::Mixer::dropped`], asked at the release). The two halves are
//! one derivation each and neither is stored, which is this section's rule
//! arriving at a gesture that outlives its press — rule 1 is what keeps the
//! middle of it off `egui`, unchanged, exactly as it did for the fader.
//!
//! **They are the one control here a boundary's grab does not reach and the
//! chips beside them do.** `.lib-filters` is `padding: 6px 9px` inside the bay,
//! so a field is 9 off each side edge against a [`GRAB`] of 6, where the chip
//! at the end of the scope row overruns the bay entirely. The row above and the
//! list below are both the bay's own, so there is no boundary up or down
//! either.
//!
//! **Five questions, one derivation.** The mixer bay is laid out once per
//! event and asked for every control it has — a knob, a blend chip, a tally
//! chip, a mask mini and the strip they sit in are five questions about one
//! laid-out strip, and a second derivation would be a second answer that could
//! disagree with the one the frame drew.
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
use karakuri_operation::gate::Class;

use crate::panel::{Panel, GRAB};
use crate::view::{
    arrangement, audio_in, deck_head, inspector, keep_pill, library, look, master, mcp_pill, mixer,
    outputs, program_bay, program_head, tracker_group, transition, Field, Scope, View, DECKS,
    DECK_LETTERS,
};

/// **What each of rule 4's derivations answers for**, one row per probe and in
/// the order [`claim`] asks them: the Outputs sink, the audio-in pill, the
/// tracker group's three, the arrangement pill, the look group's two, a
/// strip's five, the transition row's four, the Master bay's one, the
/// Inspector pane heads' `keep`, a deck head's four, the renderer chips, a
/// parameter row's fader, the
/// Program bay head's `solo`, the four deck preview cells,
/// the Library bay's scope chips, its two filter fields, the `read` chip in
/// its foot and the list above it, and the four class pills.
///
/// **One probe per derivation, cheapest answer first**, and [`on_mcp`] is last
/// because it is the dearest probe here — each class lays out its bay's whole
/// head to find one capsule in it, and the Outputs one lays out the row's word
/// and its sink's name as well.
///
/// **That is a cost ordering and not a correctness one.** [`claim`] answers a
/// `bool` and the walk short-circuits, so no order over these rows can change
/// what it says: a press is on one of these controls or it is not, and which
/// probe noticed first is nobody's business outside this array. **So the rows
/// may be reordered freely**, and the only thing the order buys is that the
/// common press — which is on nothing — pays every cheap answer before it pays
/// the dear one. Which control a press then acts on is asked again by the
/// caller, in the caller's own order (`karakuri/src/main.rs`).
///
/// **It is a table and not a sentence because it is what [`claim`] walks.** A
/// derivation that is not a row here is never asked, so it is not a control at
/// all; and [`CONTROLS`] is summed over the same rows, so the number this
/// crate exports and the questions it is a count of cannot come apart. It was
/// two parallel arrays until 2026-09-07 — a hand-summed `[usize; N]` beside a
/// `[&dyn Fn; N]` inside [`claim`] — and one value carrying both is
/// [ADR-0274](../../../docs/adr/0274-a-control-is-a-row-in-the-consoles-own-table.md).
///
/// **A control added inside a derivation already here is still a number to
/// raise**: a sixth chip on a strip is one more thing a press reaches, and
/// [`on_strip`] would go on asking five questions while its row went on saying
/// five. What has changed is that the probe and the count are now one line
/// apart instead of two arrays apart, and that **three of these rows no
/// longer carry a number at all** — the class pills, the preview cells and the
/// scope chips each count the list their probe walks, so a fifth of any of the
/// three raises [`CONTROLS`] on its own. The rest are caught where every other
/// fact about a control is: the clearance test the rule above says each one
/// owes, `tests/mask.rs` being the most recent of them, and this row is what
/// has to be raised beside it.
///
/// **The tracker group's row is the first one added under this table**, and it
/// is what the table is for: the count moved from 36 to 39 and
/// `karakuri/src/main.rs`'s own array stopped compiling until the window said
/// how it asks the three. Nothing was scanned and nothing was summed by hand.
///
/// **That is not a hypothetical, and the strip's row is the case it happened
/// to.** [`crate::view::Mixer::select`] landed with the deck selection,
/// `karakuri/src/main.rs`'s press arm asked it, both manual pages described
/// it, and [`on_strip`] went on asking four questions — so no press on a
/// strip's ground ever reached [`Claim::Panel`] and the arm that would have
/// acted on it never ran. Nothing here could have caught it: the array's
/// length was right the whole time. What catches it now is `tests/mixer.rs`,
/// which asks [`claim`] at the points a strip has no chip on.
///
/// **The scope row's count is [`Scope::ALL`]'s length and not a typed four.**
/// The row is as long as the slice the host handed the bay, and what a host
/// can hand it is values of [`Scope`] — so the number of chips a pointer can
/// reach is the number of scopes that exist, and the day a fifth is added this
/// rises with it rather than being a four somebody has to remember. The class
/// pills' count is [`Class::ALL`]'s for the same reason, one crate out, and
/// the preview cells' is [`DECKS`].
///
/// **The `read` chip's count is one**, and it is one for a reason worth
/// separating from those: the chip is a *toggle over the cursor* rather than
/// one of a row, so however long the listing is there is one capsule to press.
/// What a press on it means depends on whether a reading is open, and that is
/// inside `LibraryBay::read` where the block is.
///
/// **The filter row's count is two and is a constant**, unlike the scope row
/// above it: the row is `holds` and `layer` because
/// [`karakuri_operation::Operation::ListSets`] carries two things to narrow
/// by, and a third would be a change to the vocabulary rather than a longer
/// slice the host handed in. [`Field::ALL`] is the same two, and it is what the
/// probe walks.
///
/// **The list's count is one, and it is the one number here that could have
/// been a count and must not be.** A press lands on one of however many rows
/// the bay drew, exactly as it lands on one of however many chips it drew — and
/// the chips are counted while these are not, because the difference is what
/// each number is *about*. [`Scope::ALL`] is a closed list this crate owns, so
/// the chip count is a fact about the console; how many rows are drawn is
/// `LibraryBay::rows`, which is how many fit in a bay an operator can resize
/// against a listing a store answered — a number that changes while nobody
/// presses anything. [`CONTROLS`] is printed to an operator as *what the
/// pointer reaches here*, and a figure that moved when a divider moved would be
/// answering a different question. So the **list** is the control and which row
/// is inside [`crate::view::LibraryBay::take`], which is the `read` chip's
/// row read the other way round.
pub const PROBES: [Probe; 19] = [
    Probe {
        name: "the Outputs row's sink",
        claims: 1,
        ask: on_sink,
    },
    Probe {
        name: "the audio-in pill",
        claims: 1,
        ask: on_audio,
    },
    Probe {
        name: "the tracker group's three",
        claims: 3,
        ask: on_tracker,
    },
    Probe {
        name: "the arrangement pill",
        claims: 1,
        ask: on_pill,
    },
    Probe {
        name: "the look group's two",
        claims: 2,
        ask: on_look,
    },
    Probe {
        name: "a mixer strip's five",
        claims: 5,
        ask: on_strip,
    },
    Probe {
        name: "the transition row's four",
        claims: 4,
        ask: on_transition,
    },
    Probe {
        name: "the Master bay's out",
        claims: 1,
        ask: on_master,
    },
    Probe {
        name: "the Inspector pane heads' keep",
        claims: 1,
        ask: on_keep,
    },
    Probe {
        name: "a deck head's four",
        claims: 4,
        ask: on_deck_head,
    },
    Probe {
        name: "the renderer chips",
        claims: 1,
        ask: on_rend,
    },
    Probe {
        name: "a parameter row's fader",
        claims: 1,
        ask: on_param,
    },
    Probe {
        name: "the Program bay head's solo",
        claims: 1,
        ask: on_solo,
    },
    Probe {
        name: "the deck preview cells",
        claims: DECKS,
        ask: on_cells,
    },
    Probe {
        name: "the Library bay's scope chips",
        claims: Scope::ALL.len(),
        ask: on_scope,
    },
    Probe {
        name: "the Library bay's filter fields",
        claims: Field::ALL.len(),
        ask: on_filter,
    },
    Probe {
        name: "the read chip in the Library bay's foot",
        claims: 1,
        ask: on_read,
    },
    Probe {
        name: "the Library bay's list",
        claims: 1,
        ask: on_row,
    },
    Probe {
        name: "the class pills",
        claims: Class::ALL.len(),
        ask: on_mcp,
    },
];

/// **One of rule 4's derivations, as a value.**
///
/// A control's registration is this row and nothing else: naming it, saying
/// how many controls a pointer reaches through it, and carrying the probe
/// [`claim`] asks. There is nowhere else to add one and nowhere else to
/// forget one.
pub struct Probe {
    /// What the derivation answers for, in the words the rule above uses for
    /// it.
    ///
    /// **It is what `karakuri/src/main.rs` keys its own half of the seam on.**
    /// That file has to *act* on every control this file claims, and until
    /// 2026-09-07 it rebuilt the list by scanning this crate's source for
    /// `pub fn`s taking a `Point`, because there was no list here to read. A
    /// row is a value and has a name, so the scan is gone.
    pub name: &'static str,
    /// How many controls a pointer reaches through this one derivation, and
    /// what [`CONTROLS`] is a sum of.
    pub claims: usize,
    /// The derivation that draws those controls, asked whether the point is on
    /// one of them — and nothing is stored.
    ///
    /// **A `fn` and not a closure, because every one of these rows wants
    /// exactly the four values [`claim`] itself takes**: the panel for its
    /// solved layout, the `egui` context for a galley, the view for what the
    /// deck and the store said this frame, and the point. The derivations have
    /// nothing else in common — they answer nine different types to the caller
    /// — but the question *is the point on one of these* is one signature.
    pub ask: fn(&Panel, &egui::Context, &View, Point) -> bool,
}

/// **How many controls rule 4 hit-tests**, summed over [`PROBES`].
///
/// Exported because the answer to *what can the pointer press here* is this
/// crate's and nobody else's: `egui` owns no widget anywhere on the console,
/// so a caller has no other way to ask. `karakuri/src/main.rs` prints it in
/// its legend, where the sentence it replaced said the panel had three
/// controls and went on saying it while ten more landed.
pub const CONTROLS: usize = summed(&PROBES);

/// [`PROBES`]' claims added up in a `const`, which `Iterator::sum` is not.
const fn summed(probes: &[Probe]) -> usize {
    let mut total = 0;
    let mut at = 0;
    while at < probes.len() {
        total += probes[at].claims;
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

/// **The Outputs row's one sink**, and the first control this console drew —
/// the rule above is measured against it and `tests/outputs.rs` is where the
/// arithmetic is. Before the first frame there are no fonts and no drawn
/// control at all, which [`outputs`] answers `None` to.
fn on_sink(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    outputs(ctx, panel.layout(), view.opening).is_some_and(|row| row.hit(p))
}

/// **The two pills that are not in a bay**, and the only two asked
/// with their cards already known to be shut: rule 2 has answered
/// for the open case above, so these are the capsules alone.
///
/// The audio-in pill is asked first because it is drawn first —
/// the arrangement pill is laid out from where it ends, so asking
/// in the other order would derive the second from the first
/// anyway.
fn on_audio(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    audio_in(ctx, panel.layout(), view.transport, view.audio.as_ref())
        .is_some_and(|pill| pill.hit(p))
}

/// **The arrangement pill**, laid out from where the audio-in pill ends —
/// which is why it is asked second: asking in the other order would derive
/// this one from that one anyway.
fn on_pill(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    arrangement(
        ctx,
        panel.layout(),
        view.transport,
        view.audio.as_ref(),
        view.tracker,
        &view.arrangement,
    )
    .is_some_and(|pill| pill.hit(p))
}

/// **The tracker group's other three, derived once for all of
/// them**, which is a strip's arrangement in a row rather than in a
/// column: the offset's figure is as wide as the number in it and
/// the octave is laid out from where the tap ends, so a second walk
/// would put the chip a press lands on somewhere the mark is not.
///
/// **Asked after the two pills and before the arrangement's**,
/// which is the order the row is laid out in and is what makes this
/// the cheap answer it is: it derives the row and the audio-in pill,
/// where [`on_pill`] derives this group as well. A console told
/// nothing about the tracker pays one branch — the `tracker?` is
/// [`tracker_group`]'s first line.
fn on_tracker(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    tracker_group(
        ctx,
        panel.layout(),
        view.transport,
        view.audio.as_ref(),
        view.tracker,
    )
    .is_some_and(|group| group.owns(p))
}

/// **The bay is derived once for all of its controls**, since a
/// knob, a blend chip, a tally chip, a mask mini and the strip they
/// sit in are five questions about one laid-out strip.
///
/// **The strip is asked last, and it is the only one of the five
/// that could answer for the other four.** `Mixer::select` is the
/// column's whole rectangle, so this chain would come out the same
/// with it first — and the order is the caller's rather than an
/// optimisation: `karakuri/src/main.rs` tries the four that name
/// something inside the column and takes the strip as what is left
/// over, so a press on a knob is that knob's and a press on the
/// name, the number, the meter or a fader's track is the deck's.
/// Written in one order in both places, the two cannot come apart.
fn on_strip(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    mixer(ctx, panel.layout(), &view.mixer).is_some_and(|bay| {
        bay.grab(p).is_some()
            || bay.blend(p).is_some()
            || bay.tally(p).is_some()
            || bay.mask(p).is_some()
            || bay.select(p).is_some()
    })
}

/// **The transition row's four capsules, derived once for all of
/// them**, which is a strip's arrangement one row down: the pills
/// are laid end to end from the block's left padding, so where the
/// third is depends on how wide the first two words are, and a
/// second walk would put the capsule a press lands on somewhere
/// the word is not.
///
/// **It is asked whatever the deck is doing.** `on_strip` above
/// pays nothing on a console with no deck because there are no
/// strips to lay out; this row is the console's own setting and is
/// drawn either way, so it is a laid-out row and four word widths
/// on every press that gets this far.
///
/// **The `go` capsule is claimed with no deck behind it too**, and
/// that is `TransitionRow::owns`' own sentence rather than a
/// decision here: a press on it is refused and the refusal is the
/// act, so the panel is what has to answer for the press.
fn on_transition(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    transition(ctx, panel.layout(), view.transition()).is_some_and(|row| row.owns(p))
}

/// The two at the end of the transport row, derived once for
/// both: the exposure track's place is measured from the tone map's
/// capsule, so they are two questions about one laid-out group.
fn on_look(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    look(
        ctx,
        panel.layout(),
        view.transport,
        view.audio.as_ref(),
        view.tracker,
        &view.arrangement,
        view.look,
    )
    .is_some_and(|row| row.owns(p))
}

/// **The Master bay's one control**, and the bay is derived for it
/// exactly as the mixer's is for its five — one question about one
/// laid-out row here, because there is one thing in this bay a hand
/// can move. A console with no level behind it has no row at all
/// and pays nothing.
fn on_master(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    master(ctx, panel.layout(), view.master_out).is_some_and(|row| row.owns(p))
}

/// **The `keep` capsule in each Inspector pane's head**, one derivation
/// per pane: the pane is [`inspector`]'s answer and the capsule is
/// [`keep_pill`]'s, which is [`on_deck_head`]'s own arrangement one row
/// up. One galley lookup per pane, for the word in the capsule.
fn on_keep(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    view.inspector.iter().enumerate().any(|(index, pane)| {
        inspector(panel.layout(), index, pane)
            .and_then(|at| keep_pill(ctx, &at, pane))
            .is_some_and(|pill| pill.hit(p))
    })
}

/// **The deck head's three, one pane at a time**, and each pane is
/// derived once for all of them exactly as a strip is: the anchor's
/// place is measured from the mode chip's and the arrows' from the
/// anchor's, so they are three questions about one laid-out pane. A
/// console with no deck behind it has no panes and pays nothing —
/// [`View::inspector`] is empty, and this iterates over nothing.
fn on_deck_head(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    view.inspector.iter().enumerate().any(|(index, pane)| {
        inspector(panel.layout(), index, pane)
            .and_then(|at| deck_head(ctx, &at, pane))
            .is_some_and(|head| head.owns(p))
    })
}

/// **The renderer chips in the Inspector's panes**, and the count is
/// **one** for the Library list's reason: how many chips are drawn is a
/// property of the Set in the slot, which changes while nobody presses
/// anything, where `Scope::ALL`'s length is a fact about this console.
/// The row is asked as a whole — [`crate::view::InspectorPane::select_renderer`]
/// walks only the groups the pane drew, only the rows a press is a choice
/// on, and only until the chip under the pointer.
fn on_rend(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    view.inspector.iter().enumerate().any(|(index, pane)| {
        inspector(panel.layout(), index, pane)
            .is_some_and(|at| at.select_renderer(ctx, pane, p).is_some())
    })
}

/// **A parameter row's fader, one pane at a time**, and the last control in an
/// Inspector pane. It asks `egui` for nothing: every box in a pane is the full
/// width of the pane or a track of the mock's own grid, so this is the cheapest
/// probe in a pane. A console with no deck behind it has no panes and pays
/// nothing.
fn on_param(panel: &Panel, _ctx: &egui::Context, view: &View, p: Point) -> bool {
    view.inspector.iter().enumerate().any(|(index, pane)| {
        inspector(panel.layout(), index, pane).is_some_and(|at| at.owns(pane, p))
    })
}

/// **The one control this console has in a bay head**, and the only
/// one of the twenty-three whose capsule a boundary's grab reaches —
/// `view::program_head` is where that 0.75 of a pixel is measured
/// and argued, and rule 3 above is what decides it.
fn on_solo(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    program_head(ctx, panel.layout(), view.opening).is_some_and(|head| head.hit(p))
}

/// **The four deck preview cells, derived once for all of them**,
/// which is a strip's arrangement one bay over: the cells are four
/// questions about one arranged Program bay, and a second
/// derivation would put the cells a press lands on in the other of
/// the two arrangements. It asks for no type at all — a cell is a
/// rectangle and a letter — so this is the cheapest probe here.
fn on_cells(panel: &Panel, _ctx: &egui::Context, view: &View, p: Point) -> bool {
    program_bay(panel.layout(), view.canvas).is_some_and(|bay| bay.owns(p))
}

/// **The Library bay's scope chips**, and it is the first control
/// this console has whose *count* is a value rather than a
/// constant: the row is as long as the slice the host handed the
/// bay. One derivation for all of them, which is a strip's
/// arrangement — the chips are laid end to end from the row's left
/// padding, so where the fourth is depends on how wide the first
/// three words are, and a second walk would put the capsule a press
/// lands on somewhere the wash is not.
///
/// **The row is asked before any chip is**, inside
/// `LibraryBay::chip`: `.scopes` clips, so at the mock's own width
/// the fourth chip finishes outside the bay, and the part of it
/// that is not drawn is not a target. The part that is inside the
/// pane divider's grab is the boundary's under rule 3 above, which
/// is this rule's ordinary price and is measured in
/// `tests/library.rs`.
fn on_scope(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    library(panel.layout(), &view.scopes, &view.library, view.opened())
        .is_some_and(|bay| bay.chip(ctx, &view.scopes, p).is_some())
}

/// **The Library bay's two filter fields**, one row under the
/// chips, and the bay is derived again rather than shared with the
/// probe above it — `any` short-circuits, so the second derivation
/// is only ever paid by a press that got past the chips, and a bay
/// held across two probes would be a value living longer than the
/// question it answers.
///
/// **It asks `egui` for nothing**, where the chips ask it for a word
/// width apiece: `.field` is `flex: 1`, so where the two fields are
/// is a division of the row rather than a measurement of what is in
/// them — `LibraryBay::field`. So this is the cheapest probe in rule
/// 4 after the preview cells, and it is asked here rather than
/// earlier because the row it is in is drawn only where the chips
/// above it are.
fn on_filter(panel: &Panel, _ctx: &egui::Context, view: &View, p: Point) -> bool {
    library(panel.layout(), &view.scopes, &view.library, view.opened())
        .is_some_and(|bay| bay.filter(&view.holds, view.filters(), p).is_some())
}

/// **The `read` chip in the Library bay's foot**, and it is the
/// one control in this bay that is not in its head: the chips say
/// which library and the fields narrow it, where this reads the row
/// the cursor is on. The bay is derived a third time for the second
/// probe's reason — `any` short-circuits, so a press that got this
/// far has already been turned down by the two rows above.
///
/// **The Set under the cursor goes in with the point**, because
/// what a press asks for names it: `Operation::ReadSet` carries an
/// id, and this crate reads no store (ADR-0156), so the operand is
/// the row the host handed in. A listing with nothing in it has no
/// Set there, and the chip then claims nothing —
/// `LibraryBay::read`.
fn on_read(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    library(panel.layout(), &view.scopes, &view.library, view.opened()).is_some_and(|bay| {
        bay.read(
            ctx,
            DECK_LETTERS[usize::from(view.selection())],
            view.library.get(view.cursor_row()).map(String::as_str),
            p,
        )
        .is_some()
    })
}

/// **The rows of the Library bay's list**, and they are the first
/// thing on this console a press *takes hold of* rather than acts
/// on: a press on a row picks that Set up, and what it asks for is
/// decided where it is let go — `LibraryBay::take`, and
/// `crate::panel::Panel::carry`. The bay is derived a fourth time
/// for the third probe's reason, and this one is asked after the
/// three above because the head and the foot are drawn over the
/// ends of the same bay and a row is what is between them.
///
/// **The listing goes in with the point**, exactly as it does for
/// the `read` chip above and for the same reason: what a row means
/// is a name this crate reads no store for (ADR-0156), and a row
/// with no Set behind it is not a target. A press on the list's own
/// ground below the last row is nobody's, which is rule 4's *a
/// control claims what it acts on and no more*.
fn on_row(panel: &Panel, _ctx: &egui::Context, view: &View, p: Point) -> bool {
    library(panel.layout(), &view.scopes, &view.library, view.opened())
        .is_some_and(|bay| bay.take(&view.library, p).is_some())
}

/// **The four class pills, one derivation asked four times**, which
/// is a deck head's arrangement rather than a strip's: they are in
/// four different regions and cannot be one laid-out box, but they
/// are one type and one question — `view::mcp_pill`, asked whether
/// the point is on the capsule that region draws.
///
/// **Asked last because it is the dearest probe here.** Each class
/// lays out its bay's whole head to find one capsule in it, and the
/// Outputs one lays out the row's word and its sink's name as well;
/// `any` short-circuits, so the common press — which is on nothing —
/// still pays it only after every cheaper answer has said no.
fn on_mcp(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    Class::ALL.iter().any(|class| {
        mcp_pill(ctx, panel.layout(), *class, view.opening).is_some_and(|pill| pill.hit(p))
    })
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
/// none of that again**: the bay is derived once and all five of the mixer's
/// controls are asked of it — and neither the mask mini nor the strip adds a
/// lookup of its own, because one holds a mark rather than a word and the
/// other is the box the words were laid out into.
///
/// **A control in a bay that is not laid out is never reached**, and it is
/// [`mixer`] that answers so rather than a check here: a folded mixer — or one
/// inside a folded pane — has no room for its row of strips, `strips_row`
/// answers `None`, and the bay is `None` before any chip is hit-tested.
/// `tests/tally.rs` asserts it both ways round, and the strip itself goes
/// with them: a bay that is not laid out has no rectangle to select a deck
/// by either. [`inspector`] is the same
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
        // Rule 4, over all [`CONTROLS`] of the console's controls, one row
        // of [`PROBES`] at a time. Each is asked the same way — the
        // derivation that draws it, asked whether the point is on it — and no
        // answer is stored. The walk short-circuits exactly as the chain of
        // `||` it replaced did, so a press on the sink still costs one galley
        // lookup.
        Hit::View(_) | Hit::Nothing => {
            match PROBES.iter().any(|probe| (probe.ask)(panel, ctx, view, p)) {
                true => Claim::Panel,
                false => Claim::Egui,
            }
        }
    }
}
