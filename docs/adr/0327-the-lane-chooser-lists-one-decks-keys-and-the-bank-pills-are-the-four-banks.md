---
id: 0327
title: The lane chooser lists one deck's keys, the bank pills are the four banks, and the levels stay out of the payload
status: accepted
date: 2026-09-09
supersedes: []
superseded_by: []
principles: [0083, 0085, 0087, 0090]
tags: [console, sequencer, ui, operations, manual]
---

# The lane chooser lists one deck's keys, the bank pills are the four banks, and the levels stay out of the payload

## Context

[ADR-0320](0320-a-pattern-is-one-bar-of-sixteen-slots-a-lane-is-a-target-and-two-levels-and-a-cell-is-a-bit.md),
[ADR-0321](0321-a-lanes-target-is-an-operation-with-its-value-elided.md) and
[ADR-0322](0322-the-sequencer-is-polled-like-a-transition-live-only-and-its-writes-are-its-record.md)
settled what a pattern is, what a lane's target is spelled as, and where the producer runs, and the
first slice built all three. **They left the bay's two remaining rows waiting on drawings rather
than on decisions**, and ADR-0320's own consequences say so: the bank pills need *"a head that can
say which pill is armed"*, and `+ lane` needs *"a target chooser nobody has drawn"*.

Drawing them asks four questions none of those records answers, and each has an alternative
somebody will re-propose.

1. **Which deck's parameters the chooser lists.** A lane's target is `Fader { deck }` or
   `Param { deck, param }`, so the card is a list of both — but a parameter is a *deck's* published
   control and there are four decks.
2. **What the mock's `+` becomes now the bank count is fixed.** The head is drawn
   `seq 1 · seq 2 · +`, and the `+`'s tip says *"the next empty one … disabled when all four are
   full"*. ADR-0320 fixed the count at four **after** that drawing was made.
3. **Where a lane's two levels are filled in.** ADR-0320 says *"the console fills them in at the
   press"*; the payload ADR-0321 shipped is `PointLane { pattern, target }` and has nowhere to put
   them.
4. **Whether a lane arrives driving.** Not asked anywhere, and found by building it: `Lane::new`
   leaves a lane unmuted, and ADR-0320's off step *writes* `off`.

## Decision

### The chooser lists every drawn deck's fader and the load pulldown's deck's published controls

The card carries one item per strip the mixer draws — `A ▮ fader` … `D ▮ fader` — a separator, and
one item per published control of the deck
[ADR-0305](0305-the-library-bays-load-is-a-button-and-a-pulldown-and-the-deck-it-names-is-not-the-selection.md)'s
pulldown names, read out of `View::inspector`: `B ∿ L2:0 twist`.

**That mark and not a new one.** It is this console's one pointer meaning *a deck named without
moving the keys*, which is exactly the case pointing a lane is: a lane on deck C while deck A is
playing. Reusing it is [P-0085](../principles/0085-take-the-mechanism-that-exists-and-pay-the-bill-now.md)
at the width of a pointer, and a chooser of its own in this bay would be a **fourth** pointer on a
panel ADR-0305 already had to argue a third onto.

**What the console has not been handed, it does not offer.** `View::inspector` holds `PANES` panes
— two — so a target deck no pane is showing contributes no parameters and the card is that deck's
faders alone. **That is the reading's limit and not the control's**, it is written into the `+ lane`
tip and the *Sequencer* note rather than worked around, and it is what a third pane or a per-pane
pointer would lift. Nothing here invents a range for a control the console cannot see, which is the
refusal below.

### Four bank pills, and no `+`

The head draws `seq 1 · seq 2 · seq 3 · seq 4`, the armed one `.pill.armed`, and a press is
`SelectPattern { pattern }` naming that bank.

**The `+` is not drawn, because with four fixed banks it is a press already on the head.** ADR-0320
says the `+` *"is the same choice landing on an empty one rather than an operation of its own"* —
so once every bank has a pill, `+` means *press `seq 3`*, and drawing both is one act with two
doors: `docs/contributing.md` §4's *where one fact is stated twice, make them one thing or delete
one*. The disabled state its tip promised — *"when all four are full it is disabled and says so"* —
has no state to be in either, because a bank with a pattern in it is still a pill you press.

**The mock moved rather than the panel diverging from it.** §5's rule is that a control the panel
draws that the mock does not is a defect, so `docs/manual/console.html` draws four pills now and the
`+`'s argument is carried on `seq 3` and `seq 4`, which are the empty ones.

### The levels are filled where the lane is appended, and `PointLane` gains no fields

`Operation::PointLane { pattern, target }` is unchanged. The window fills `on` and `off` when it
appends the lane: **1.0 and 0.0 for a fader**, and for a parameter the top and bottom of the range
in `View::inspector` — *the console's own published reading, and the very list the chooser drew its
items from*.

**One reading and not two.** The alternative below would carry the numbers in the payload to make
sure the operator's range and the lane's range are the same; reading `View::inspector` at the press
achieves that by there being only one range in the system to read. Asking the deck again would be
the second derivation ADR-0320 warns about in its own words — *"the lane must not read it later"* —
one frame earlier.

**A control the console holds no row for is refused, and the refusal names the next attempt**
([P-0083](../principles/0083-a-refusal-carries-what-the-next-attempt-needs.md)): *point the Library
bay's load pulldown at the deck whose control it is and ask again*. A lane with defaulted levels
would drive its target to two numbers nobody chose.

### A lane arrives muted

`+ lane` appends a muted lane. **It arrives with every slot off, and an off step writes `off`**
(ADR-0320), so an unmuted fader lane would hold its deck at `off` from the press — the deck on air
goes dark and nothing on screen says a sequencer did it. That is `demonstration_banks`' argument
reached from the other end, and it is the same one ADR-0320's consequences already make for the
seeded lane. The unmute is the label, one press, which is rule 02's take-back drawn where the lane
is.

### The head machinery: one field and one bit

`view::Head` gains `banks: Option<usize>` — `None` from the table, set by the one caller that draws
them — and `view::HeadWords` gains `armed`, a bit per capsule. **The class pill's arming moves into
that bit and stops being a comparison at the paint**: `bay_head` read `words[index] == MCP_OPEN`,
which works only while *lit* and *this word* are one fact. They are for a class pill, whose two
states are two words; they are not for a bank pill, which reads `seq 2` armed and `seq 2` plain.

## Alternatives rejected

**List every deck's published controls, each item naming its deck.** No borrowed mark, nothing
silently absent, and the card is exactly what the console holds. It loses on the list: four decks
publishing the same key put that key in the list four times, so the operator picks a deck by reading
a list four times as long instead of by a control — and the control they would be avoiding is one
this panel already draws and already explains. **It also buys nothing today**: the inspector holds
two panes, so *every deck* and *the target deck* differ only in whether deck B's keys are offered
while the pulldown says A. It is the answer to re-propose the day a pane can be pointed anywhere, or
the day the panes outnumber the strips.

**A deck chooser of its own in the Sequencer bay's foot.** The most direct reading, and it is the
one ADR-0305 spent its counting argument against: a fourth pointer, on a panel that has to explain
three, for a question an existing pointer answers.

**Keep the mock's `+` and draw two pills and a plus.** The status quo of the page, and it is what a
reader expects to find. It loses because *how many banks there are* was open when it was drawn and
is closed now: with four fixed banks the plus and the third pill are one press, and a control whose
whole meaning is *the next empty one* is a control for a list that grows.

**Draw a pill only for a bank that holds a pattern, plus a `+`.** The mock read literally, and it is
the one that nearly works. It breaks on the press it exists for: press the `+`, the empty bank is
armed, and it has no pill — so the head cannot say which bank is armed, which is rule 02 and the
exact thing the head machinery was changed to make possible.

**Widen `PointLane` with `on` and `off`.** ADR-0320's sentence taken literally, and it has a real
argument: the numbers the operator was shown travel with the ask, so no second reading can differ.
It loses on what the payload then is. Two floats a surface must supply and that only *this* surface
can supply meaningfully — a map line and a model can name neither — is an operation asking for what
one caller can say rather than what a surface can (ADR-0192 read the other way), and the disagreement
it guards against does not exist: there is one published reading in this program and the window
reads it. It is also the change that would have dragged a payload edit through
[§5](../contributing.md)'s nine places for a control that needed none.

**Let a lane arrive driving.** `Lane::new`'s own default, and it is what *add a lane and it works*
would mean. It is refused by what a lane with no steps does: it writes `off` at every boundary, so
the first thing `+ lane` on deck A would do is take deck A off the screen. A control whose first
effect is invisible destruction is rule 04 with the grey removed.

## Consequences

**What was built the same day, and each clause is a description of the tree** (2026-09-09):

- **`view::Head` carries `banks` and `view::HeadWords` carries `armed`**, with `Head::with_banks`
  the one door into the first. `HEAD_PILLS` is `2 + karakuri_pattern::BANKS` and the `const`
  assertion beside it now asserts every table entry fits **with the class pill and the four banks
  beside it**, which is the pessimistic reading rather than the exact one.
- **`tests/head_words.rs` is what says no other head moved**: `head_of` hands no region banks, every
  head's capsules are its table entry plus its class pill under both openings, nothing is lit but an
  open class pill, and the class pill's word and its bit are one answer. `tests/mcp_pill.rs`,
  `tests/solo_pill.rs` and `tests/fold_grip.rs` — the pixel-level tests of those same heads — were
  not touched and are green.
- **`view::Sequencer` carries `banks`, `add` and `card`**; `Sequencer::press` gained the bank arm
  and `Sequencer::chose` is the `+ lane` control, which answers `view::Chose` rather than an
  `Option<Operation>` because two of its three answers are the console's own state. `view::Choices`
  is what the card is laid out from, read off `View::lane_choices()`.
- **The card is `RowMenu`'s mechanism** (ADR-0311) reached a second time: the same panel fill,
  radius, hairline, item stride and separator band, hanging **up** off a pill in a bay foot the way
  `Load::list` does.
- **`input::claim` gained a fifth clause to rule 2** and the Sequencer's probe row counts four more
  capsules and the `+ lane` pill; the card itself is counted by neither, for the reason the
  Library's two are not.
- **`crates/karakuri`'s `sequenced` takes the `View`** and has a `PointLane` arm — `pointed_lane` —
  which fills the levels, mutes the lane, and refuses a control the console holds no row for.
- **`docs/manual/console.html`'s Sequencer head draws four pills**, the `+ lane` tip says what the
  card lists and that the lane arrives muted, and the *Sequencer* note gained a paragraph on both.
  **Both rows' panel badges on `docs/manual/operations.html` are `has`.**

**And one sentence in that tip was false and is gone** (ADR-0031). `+ lane` carried *"the panel
cannot grow a bay's body while it is running, because the arena the regions live in has no insert
and no remove"*, naming itself as one of *one gap drawn in five places*. **A lane row is not a
region**: the bay draws its rows from `Pattern::lanes`, inside a rectangle the arena already holds,
and the first slice was already drawing rows that way with no arena insert anywhere. So the gap is
drawn in **four** places — the Master bay's `+ add`, the Outputs row's `+ add output`, the library's
`+` on the scope list and the inspector's `2 up` — and this row was never one of them.

**What that leaves owed, and it is a sentence rather than a control**: **two** tips still say *five
places* and name `the sequencer's + lane` among them — the Master bay's `+ add` and the Outputs
row's `+ add output`. They are two other bays' tips, and correcting them is one clause each with no
code behind it.

**What this record does not decide.** Removing a lane, which still has no control, no row and no
operation. Whether a channel fader says who is holding it (ADR-0323's owed readout). And whether the
Inspector's two panes are enough to point a lane at any deck's parameters, which is the alternative
above waiting for a pane that can be pointed.
