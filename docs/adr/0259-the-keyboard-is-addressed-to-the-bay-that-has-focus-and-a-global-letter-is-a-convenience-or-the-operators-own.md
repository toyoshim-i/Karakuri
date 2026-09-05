---
id: 0259
title: The keyboard is addressed to the bay that has focus, and a global letter is a convenience or the operator's own
status: accepted
date: 2026-09-05
supersedes: []
superseded_by: []
principles: [0073, 0083, 0085, 0090, 0094]
tags: [ui, keyboard, surfaces, vocabulary, docs]
---

# The keyboard is addressed to the bay that has focus, and a global letter is a convenience or the operator's own

## Context

[ADR-0220](0220-the-key-column-is-the-instruments-keyboard-and-the-clis-keys-are-its-own.md) settled
*whose* keyboard the key column of [every operation](../manual/operations.html) measures. It did not
settle how that keyboard is laid out, and the layout it inherited is one key per operation chosen by
the operation's initial letter: `f` folds, `s` solos, `r` resets, `m` is the blend mode, `k` keeps,
`l` loads, `e` is the library's scope, `b` taps.

**That scheme has run out of letters, and the page says so in four places.** Four Mixer rows are
specified for letters the instrument has already bound: *Fade a deck out or in* wants `f g`, which
fold a region and the split enclosing it; *Choose which renderer of a deck is live* wants `r`, which
resets the arrangement; *Choose the wipe shape, the quantum, the length* wants `z n j`, and `z`
unfolds everything while `n` is the room's day and night. ADR-0220 recorded the collision as a
measurement — *"the two collide on eight letters"* — and resolved none of it, because the question in
front of it was which program a badge names.

**A mnemonic keyboard's supply is fixed and the vocabulary's demand is not.** The page has 63 rows and
the alphabet has 26 letters. Sixteen rows are reached by a key today and ten more are designed for
one — 26 rows have asked, before the Sequencer, the Master chain, the Inspector and the Staging lane
ask for theirs, and 19 of the 37 rows whose key column reads `gap` are `gap` for want of a route
rather than for a reason. The scheme does not fail in three months; it fails at the next bay, and it
fails again by having to be re-argued at each one after that.

Asked what should replace it, the maintainer decided on 2026-09-05 — translated: **global key
assignments are reserved for a few convenience operations plus user customization; the operation
vocabulary is reached through a scheme built on moving focus between bays with Tab; the assignments
available while a bay is focused feel the same in every bay; assigning keys by a function's initial
letter stops; and the vocabulary centres on digits, space and enter.**

**The console has been drawn for this since before it was drawn.**
[`docs/manual/console.html`](../manual/console.html) has carried *Two focuses, and they do not look
alike* from the beginning: *"The deck selection persists — it is what a key press is addressed to, and
it stays put when you tab away into the library. Keyboard focus is transient and goes wherever tab
lands … the selection is a solid ring and focus is a dashed one."* The mock draws both — `.strip.focus`
on deck A's column and `.wfocus` on deck B's fader — and `karakuri-console/src/view.rs` draws the
first and says why it does not draw the second: *"Nothing draws the dashed one — this console takes no
keyboard focus."* **The surface this record designs was specified before the panel was and never
built.** What is new here is not focus. It is what focus is *for*.

## Decision

**A key press is addressed to whatever holds focus, and focus is moved between bays by `Tab`.** The
keys that act inside a focused bay are the same six in every bay, and they are stated as rules about
*kinds of thing* rather than about bays, which is what makes them the same six. **No operation is
given a letter for its initial**, and the global list is closed by a rule rather than by whatever
happens to be free.

### The seven kinds

The grammar can be uniform because the panel is, and this is the vocabulary it is uniform in.

- **A bay** — one of the nine the manual's *What each region is standing on* lists: Transport,
  Library, Staging, Program, Inspector, Mixer, Master, Sequencer, Outputs.
  `karakuri-console::view::REGIONS` already lists them in the order the panel reads, top to bottom and
  left to right.
- **An item** — one of the things a bay lists, in the order it draws them: a mixer strip, a Set in the
  library, a sink, a lane, an inspector pane, a preview cell, a candidate row.
- **A control** — what an item is made of, and what a bay's head holds: a trim, a fader, a tally chip,
  a scope pill, the Program bay's `solo`.
- **A state** — a control whose values are a closed list: a blend mode, a residency, an authority, a
  sync mode, a tone map operator, a fold.
- **A level** — a control whose values are a continuum: a gain, an opacity, a master out, an exposure,
  a mask position, a latency offset.
- **An act** — a control that performs rather than sets: a load, a keep, a fade, a restore.
- **A field** — a control that takes letters: the arrangement pill while it is asking for a name.

Nothing on the console is outside those seven. That is a claim about the panel rather than a
definition, and the walk below is where it is tested: where it fails it fails by finding a bay with
nothing of a kind, not by finding an eighth kind.

### The six keys

**`Tab` — the next bay; `shift-Tab` — the previous one.** It always moves between bays, whatever depth
the address had reached inside the one it leaves, and it never descends. Nine bays is a ring short
enough that forward alone would do; backward is kept because `shift-Tab` is not a second meaning for a
key but the reversal of a traversal, which every text field on every desktop already spells that way.

**Next is down a column first, and only then across to the next one.** The arrangement is a tree, so
the rule is a walk of it: a column's children are taken top to bottom, a row's children left to right,
and a bay is reached when the walk arrives at its leaf. On the panel as it stands that is the
transport band, then the left pane's bays down, then the centre's, then the right pane's, then the
outputs band.

**A scanline was the other answer and it is the worse one here.** This panel is three panes side by
side, so a row-major traversal would cross from the library to the inspector to the mixer and back
again on every band, cutting each pane into slices and putting a bay's neighbour in a different
column. Down-then-across keeps a pane whole, which is how an operator holds it: the panes are the
three things on the screen, and the bays are what is in one.

**`shift-Tab` is that same walk run backwards, and nothing else.** Up a column, and from the top of one
column to the **bottom** of the one before it — not to its top, which is the other reading and the one
that would make the two keys disagree about where they are. One key is the walk and the other is the
walk reversed, so an operator who overshoots gets back exactly where they were.

The order is the panel's geometry and not a list anybody maintains, which is what keeps it right after
a divider is dragged or a bay is folded: an operator who has rearranged their room has rearranged the
traversal with it, and nothing has to be told. `karakuri_console::view::REGIONS` is already written in
exactly this order and its doc already says the words — *"the order is the arrangement's, top to
bottom and left to right, so this reads like the panel"* — but the rule is about the walk rather than
about that constant, so a ring derived from the solved tree is the honest implementation and the
constant is a thing to check against, not the source.

**A digit — the nth thing one level below the address, and the address descends to it.** `1` to `9`
name the first nine; `0` names the bay's **head**, where a bay keeps the controls that are about the
bay rather than about anything in it. The address is a path, so a second digit names the nth control
of the item the first named and a third descends again where a bay is that deep. There is no depth
limit in the rule because there is none on the panel: the Inspector is three deep and the Outputs row
is one, and neither is a special case.

Digits count from **one**, and they count what the bay drew. That is a change from the four deck keys,
which count from zero because `karakuri-cli` counts slots from zero and *"nothing had to be
translated"*; a digit naming a position on a panel cannot start at zero without either losing `9` or
growing a tenth key, and the command line is test tooling whose principles are not the instrument's
([ADR-0242](0242-the-command-line-is-test-tooling-and-the-instruments-principles-do-not-bind-it.md)).
A bay with more than ten items is walked rather than counted; only the Library and the Sequencer have
one, and both already have a walk.

**`esc` — up one level, and nothing else.** From a field it abandons the name, from a control it
returns to the item, from an item to the bay. **At bay level it acts on nothing and says so**, because
there is no unfocused state to fall out into — [P-0083](../principles/0083-a-refusal-carries-what-the-next-attempt-needs.md),
and the page's own sentence that *"a key that declines and a key that is not bound are the same
experience."* **`esc` does not quit**, which is the change that ripples furthest and is argued below.

**The arrows — the neighbour of the addressed thing, along the axis it is drawn on.** On an item, the
next item: the mixer's strips are a row, so `←` and `→` walk them, and the library's rows are a
column, so `↑` and `↓` do. On a level, the next value, in the step the page already specifies for that
control. So a fader that has focus takes arrows and a list that has focus takes arrows, which is what
an operator will try first.

**`space` — the addressed thing's next state.** On a control with a closed list it cycles it, and a
cycle is the surface's affordance over an operation that names a state, which
[P-0090](../principles/0090-a-surface-offers-it-never-decides.md) permits and the blend chip already
uses ([ADR-0187](0187-the-blend-mini-cycles-and-a-map-learns-the-three-it-cycles-through.md)). On a
**bay**, the closed list is *folded* and *unfolded*, which is the one state every bay has. On a
**level**, the one state worth naming is the value it was declared at, so `space` returns it to its
default — which is what `\` does for the trim and `` ` `` for the exposure today.

**That last clause is bought with an argument rather than found**, and it is written down rather than
left to be noticed: a continuum has no *next* value, and calling its default *the* state is a reading.
It earns its place because the alternative is a seventh key for one job, and because the two controls
that need it already have a key for it and would otherwise lose one.

**`enter` — the act the addressed control is for.** On a field it takes what was typed. On a control
that sets rather than performs it does nothing and says so. **Where an item has two acts they are two
controls and a digit chooses between them**: a staging row's *keep this candidate* and *put the
previous version back* are `n 1` and `n 2`, and a strip's fade, crossfade and wipe are the transition
row's controls rather than a hidden second meaning of `enter`. That is what keeps `enter` a rule about
a kind instead of a per-bay verb.

### A folded bay stays in the ring so that it can be opened

**Not so that it can be operated.** The maintainer's reason is the narrow one and it is the right one:
a folded bay is one you unfold, and being unable to reach its contents while it is folded is not a
problem — it is in the ring because otherwise there is nothing to press to open it.

Two things follow, and both are smaller than the wide reading would have made them. **While focus is
on a folded bay the only key that acts is `space`**; a digit, `enter` and the arrows decline and say
why, because there is nothing drawn for them to land on. And **the mark the mock has to grow only has
to say *there is a folded bay here and `space` opens it***, rather than standing in for a bay's
contents. A folded region has no rectangle and no divider is drawn beside it
([ADR-0204](0204-the-root-and-the-body-row-stay-unnamed-and-a-folded-root-is-not-hit-testable.md),
with [P-0073](../principles/0073-a-node-claims-only-what-its-visible-content-can-use.md)), so that
mark is a new drawn thing either way; this is the cheapest version of it.

### Focus is drawn, and it is what the deck selection already was

The mock's two rings stay two rings and stop being two mechanisms. **The dashed ring is where the
address is now; the solid ring is where a bay's address was the last time that bay had focus.** Every
bay remembers its own, so the deck selection persisting *"while your hands are in the library"* is not
a property peculiar to the mixer — it is the general rule seen once.

`karakuri-console::view::View` holds three pointers today, each private, each with its own argument
for why it is the console's own: `selection`, `cursor_row` and `scope`. **They become three instances
of one mechanism**, and the sentence written at each of them — *"nothing downstream can be the model of
record for it"* — is written once instead of three times. Nothing about what any of them *means*
changes, and *Select a deck* keeps its row for the reason `console.html` already gives: it is what
every deck-addressed operation's keyboard translator fills its `deck` in from.

### Focus starts where the traversal starts, and quitting is not the keyboard's

**There is no unfocused state.** `Tab` is always on some bay, so focus has to start somewhere, and the
walk answers it: **the first bay the traversal reaches**. That is the Transport as the arrangement
stands, and naming the rule rather than the bay is the point — an operator who has moved the Transport
has moved the starting position with it, and no second rule has to agree with the first.

An earlier draft of this record started focus on the Mixer, at deck A's strip, arguing it from the
mock already ringing that strip. That was a second rule: it would have had to be kept in step with
the traversal by hand, and the first divider dragged would have parted them.

**Quitting follows the platform's own accelerator** — `⌘Q`, `Alt-F4` — rather than sitting at the top
of a ladder of `esc` presses. A quit ladder is a sequence that ends in something irreversible, in
front of an audience, reached by repeating one key
([P-0094](../principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)),
and every operating system already has a gesture for this that an operator's hands know.

**So *Quit*'s key column becomes `gap`, and the reason is structural rather than a demotion.** The key
column is what the window loop's `match` on `logical_key` binds (ADR-0220), a platform accelerator
never reaches that `match` — it arrives as `WindowEvent::CloseRequested`, which this program already
handles — and `key_column::bound` reads `Key::` arms out of this file's text and so cannot see an
accelerator in either direction. A badge naming `⌘Q / Alt-F4` would be a key badge for something that
is not a key, on a page whose badges are instructions to press a thing. **What the page still owes is
to say somewhere how to quit**, and that is not a badge; it is left open below, because the key
column's grammar cannot carry two platforms and inventing a fifth column for one row is worse than
the sentence it would replace.

## The grammar, walked across all nine bays

Written out per bay this would be nine grammars; written as kinds it is one, and the walk is what
tests that. Where a bay has nothing of a kind that is recorded as a finding rather than smoothed over.

**Transport.** A headless row (ADR-0159), so `0` names the row itself. Its items are the controls left
to right: the tempo, the halve and double pair, the beat, the bar, the frame cost, the audio-in pill,
the arrangement pill, the tone map, the exposure. `space` on the tone map cycles the four operators —
today's `t`. `↑↓` on the exposure step it and `space` returns it to 1.0 — today's `- = ` ` `. `↑↓` on
the offset step it five milliseconds — today's `o p`. `enter` on a row of the audio-in card attaches
that input; `enter` on the arrangement pill's menu opens the field the naming flow already has.
**Tap the beat has no item here** — the mock draws a `tap` control and that row's panel badge is
`plan` — and tapping is a hand keeping time, which a `Tab` first would spoil. It stays global.

**Mixer.** Items are the four strips, `1`–`4`, and **naming a strip is the deck selection**:
`Operation::SelectDeck` is emitted by the digit, which is why that row's badge becomes `1 2 3 4`
rather than disappearing. A strip's controls are its tally, trim, fader, blend chip and mask mini, so
`2 3` is deck B's fader. `space` on the tally is the next residency — the row
[ADR-0240](0240-the-output-shows-the-mix-and-residency-keys-belong-to-the-mixer.md) marked `gap` on
the grounds that residency *"is fundamentally the operational responsibility of the Mixer bay"*; that
record refused a **global** letter for it, and this is the scheme in which its own sentence becomes
satisfiable. `↑↓` step the trim and the fader, `space` returns each to its default, `space` cycles the
blend and the mask shape. `space` on a strip with no control named does nothing and names the five
controls it has, because a strip has five states and none of them is the strip's. Fade, crossfade and
wipe are acts on the addressed strip, hung on the transition row's controls, whose states are the
shape, the quantum and the length.

**Library.** `0` is the head and its controls are the scope chips, so `space` there cycles the scope —
today's `e`. Items are the listed Sets; the digits name the first nine and `↑↓` walk them, which is
what they do today. `enter` on a row is the load onto the selected deck — today's `l` — with both
operands still on screen before the press, which is what that argument was always about. **A row has
no state**, so `space` reaches nothing in this bay below its head.

**Staging.** Items are the candidate rows, and a row's controls are its two acts: `enter` on `n 1`
keeps the candidate, `enter` on `n 2` puts the node's previous version back. **Nothing here has a
state at all**, so `space` reaches nothing in this bay below its fold. It is the second of the two
bays that are lists of things that happened rather than things you set.

**Program.** `0` is the head, whose controls are `solo` and the class pill; `space` on `solo` is the
state *soloed* and *not*, which is `s` and `u` collapsed into the one control they always described.
Items are the picture and the four preview cells; `space` on the picture turns that sink on and off.
**The four cells answer to nothing** — ADR-0243 made them monitors fixed to a deck and ADR-0240 moved
residency to the mixer — so a digit lands on a cell, the ring is drawn, and `space` and `enter` both
decline and say why. It is the clearest case in the walk of items with neither a state nor an act, and
it is correct: a monitor is a thing you look at.

**Inspector.** Three deep, and the deepest bay on the panel. Items are its panes; a pane's controls are
its deck head and its node groups; a node group's controls are its authority chip and its parameter
rows. So `1 2 3` is the first pane's second node's third parameter, `↑↓` step it and `space` returns
it to its declared default. `space` cycles an authority and cycles the deck head's sync chip, skipping
what the material cannot honour, which is that chip's affordance today. `↑↓` on the anchor scrub a
quarter beat. **This bay is what proves the address has to be a path rather than two levels**, and it
costs nothing that it is, because the rule was never *two*.

**Outputs.** Headless again, so `0` names the class pill it carries in place of a head. Items are the
sinks and each has exactly one state, so `space` is the whole of this bay. **No item here has an act
and none has a level.** It is the simplest of the nine and it is what the grammar looks like with one
kind in it.

**Master.** Items are the out fader and the three effects. `↑↓` step `out` and `space` returns it to
its default — a row whose panel column is `has` and whose key column is `gap` today. **The three
effects have no addressable controls**, because `Operation::SetFeedback { params: Undecided }` and its
two neighbours carry no spelling for a parameter, so a digit reaches the effect and stops. That is a
finding about the vocabulary rather than about this grammar, and it is the one place in the walk where
a bay is drawn and its operations are not sayable.

**Sequencer.** `0` is the head: `16`, `1/8` and `2 bars` are states, the bank pills name which pattern,
and `+ lane` is the head's act. Items are the lanes; a lane's controls are its label, whose mute is a
state, and its steps. **Sixteen steps outrun ten digits**, which is the one place the digits fail
outright: the steps are walked with `←→` and set with `space`, and the digits reach the first nine of
them, which is honest and very nearly useless.

**Two findings the walk produced that nobody was looking for.** `space` reaches nothing in Staging and
nothing in Library below its head, and `enter` reaches nothing in Mixer, Outputs, Master or Program —
so **no bay uses all six keys**, and a legend implying otherwise would be lying. And `program-view`
and `deck-previews` are already the Program bay's two children in the arrangement, as `inspector-1`
and `inspector-2` are the Inspector's — so for two of the nine bays **the arrangement's own tree is
the item list**, and the address is a path through a tree that exists.

## The global conveniences, and the rule that decides membership

**A key is global if the operation it names has no operand, or if its only operand is the choice the
key itself spells.** Focus exists to supply an operand — which strip, which row, which node — so an
operation with no operand has nothing for focus to add and the press is already the whole gesture.
Where the operand is a value or a name, a global letter would mean something different depending on
state nobody declared.

Applied to `KEYS`, which holds 31 entries:

**Seven keys survive as global**, naming six operations and one thing that is not one. `esc` carries
the ladder; `b` taps the beat, whose operand is nothing and whose timing is an argument the operand
rule does not have to make; `,` and `.` halve and double the grid, whose only operand is the direction
the two keys spell; `r` resets the arrangement, which
[ADR-0208](0208-resetting-is-the-default-case-of-restoring-an-arrangement.md) already noticed
*"addresses nothing"*; `z` brings back what is folded, whose operand is `region: None`; and `n` is the
room's day and night, which is not an operation and never was.

**Twenty-one keys become focus-relative.** `f`, `g`, `s` and `u` take their region from the *pointer*
today, through `Readout::target`, and take it from the focus instead — which is the whole of what the
pointer was standing in for. `0`–`3` become the Mixer's `1`–`4`. `[ ] \` and `; '` become arrows and
`space` on the addressed trim and fader. `m`, `e` become `space` on the addressed chip and on the
Library's head. `o` and `p` become arrows on the transport's offset. `up` and `down` stop being the
Library's private walk and become the arrows everywhere. `l` becomes `enter` on a library row, and `k`
becomes an act on a control the Library bay does not draw yet.

**Three are neither.** `return`, `backspace` and `space` are live only while the arrangement pill is
asking for a name, and they are not an exception: a field is one of the seven kinds and letters are
what it is for. The branch in `window_event` that takes the keyboard whole while a name is being typed
is that rule already written down.

**The surviving globals keep the letters they have.** *Stop assigning keys by a function's initial
letter* is a rule about how a letter is **chosen**; re-lettering `b`, `r` and `z` after the fact would
take keys an operator has learned in exchange for nothing. What stops is the reservation.

## Where user customization lives

`karakuri-midi`'s map is the layer between a surface and the vocabulary
([ADR-0236](0236-a-map-is-the-layer-between-a-surface-and-the-vocabulary-and-the-audit-is-one-of-the-things-it-does.md)),
and [P-0085](../principles/0085-take-the-mechanism-that-exists-and-pay-the-bill-now.md) says to check
what a feature is made of before it gets a subsystem. **It is the same mechanism for the global half
and not a map at all for the focused half**, and the split falls exactly where the operand rule put
it.

A MIDI line is **complete**: `cc 5 -> opacity 0` names the deck, and `map.rs` says what that buys —
*"[`Map::operation`] is a pure function of one message … with no world to set up and nothing to
mock."* A global key is complete in the same way, so a global key map is that map with **a third `Key`
beside `Cc` and `Note`**, the same target grammar on the right, and `Map::operation` still a pure
function.

A focused key is **deliberately incomplete**: `space` means *the addressed control's next state*, and
the address is console state a map cannot see. A line that could say it would have to be a function of
a message and a focus, which is not the function `Map::operation` is. So the focused grammar is not
customizable, and **that is a property rather than a gap** — the six keys are uniform because they are
the grammar, and a grammar an operator re-spells per bay is not a uniform one.

**`map.rs` moves to `karakuri-map`.** A crate named for a wire would otherwise parse something that
never touches one, and the seam is already drawn in the file list: `device.rs` and `message.rs` are
the wire, `map.rs` is the layer, and `examples/surface.map` was never called `midi.map`. The bill is
`--midi-map`'s spelling, two manifests and the doc links that name the module — small, and now, which
is P-0085's second half.

## Alternatives rejected

### a. Keep mnemonic global letters, and re-letter the four colliding Mixer rows

**The case for it is that it is free today.** Four rows want letters that are taken and there are
letters that are not; re-lettering *Fade*, *Choose which renderer is live* and the three transition
settings is five badge edits and five `match` arms, it breaks nothing, and **it leaves every operation
one press** — which is the real advantage and should not be waved away, because a performer's hands
are the point and `Tab`, a digit and `space` are three presses where `m` was one.

**It loses on arithmetic that is not close.** 63 rows against 26 letters, with 26 rows having already
asked and four bays yet to ask. It does not fail later; it fails at the next bay and again at the one
after.

**And it loses on what a mnemonic is worth once it has stopped being one.** The value of `f` for
*fold* is that it is guessable. Once `f` is fold, `g` is fold-the-enclosing-split, `j` is a transition
length and `n` is night, the scheme is a table to memorise wearing a mnemonic's clothes — which is the
state `KEYS` is in, and the reason its printed legend once drifted to nine lines while nineteen keys
were bound.

### b. Share a letter, resolved by what is under the pointer

**The strongest of the four, because it is already how half this keyboard works.** `f`, `g` and `s`
do not name a region: they take it from the pointer, and `Readout::target` is that rule written down.
Extending it — `f` folds over a bay head and fades over a strip — is not a new mechanism but the one
this program has, it costs no `Tab` at all, and one press still does one thing.

**It loses on P-0090's failure mode one level out.** A surface offers and never decides; a key whose
meaning is settled by where a pointer happens to be sitting is a key whose meaning the operator did
not choose. A pointer is not a declaration of attention — it is wherever the hand last let go — so one
press does two things according to a state nobody set and nothing rings. **Focus differs precisely and
only in being declared**: an operator pressed `Tab` to put it there and a ring says where it is. That
is the whole argument between this alternative and the decision, and it is worth saying that the
**arrows** in the accepted grammar are conditional in exactly the way this alternative is — they step
a value or walk a list according to what is addressed — and are acceptable for that reason and no
other.

**And it loses on rule 01, which is decisive.** *Every operation is reachable from the keyboard alone.*
A key that needs a pointer to be somewhere is not, and sixteen rows now marked `has` in the key column
would be relying on a mouse to say so.

### c. Modifier keys

**The case is capacity.** `ctrl`, `alt` and `shift` multiply 26 letters by eight and end the shortage
outright, with no focus, no ring, no ordering question and no cost to the mock. It is what every
application the operator already uses does.

**It loses three ways.** It does not solve the problem it appears to: a chord is still one gesture per
operation, so 63 rows want 63 chords, and the difference between memorising 63 letters and 63 chords
buys headroom rather than learnability. It is the wrong hand — a chord takes two hands or one
contorted one, in a room where the other hand is on a controller and the light is bad, where a digit
is one finger on a number row a performer finds without looking. And, decisively, **a modifier is a
mode with no readout**: `ctrl` held is a state of the instrument that nothing on the panel says, and
rule 04 is *Nothing is hidden quietly*. Focus has a ring. The one modifier kept — `shift-Tab` — is not
an exception, because it gives no key a second meaning: it reverses a traversal that is drawn either
way.

### d. *Global if it is not about any one bay, or if it must work when a show is going wrong*

**The membership rule offered for the global list, and it was offered seriously.** It reads well and
it names the two intuitions anybody would start from: a key that belongs to no bay has nowhere else to
live, and a performance has moments where hunting for a bay is not available.

**It fails in both directions, which is why the operand rule replaced it.** *Not about any one bay* is
too wide: it admits *Reset the arrangement*, *Bring back what is folded*, *Save the arrangement*, *Put
a saved arrangement back*, *Size the window*, *Record the session*, the room, *Set the free-run tempo*
and *Master out* — because a master level is about the whole fold and not about one bay, which is the
page's own sentence for it. Nine or ten is not *a few*, and two of them carry a name no key press can
type.

**And its second half re-opens the panic key.** Taken at its word, *must work when a show is going
wrong* admits every operation a frightened operator reaches for — the master out, an opacity, taking a
deck off air — which is what [ADR-0071](0071-there-is-no-panic-key.md) refuses, arriving under another
name. It also fails to admit the one operation that most obviously wants a global key, because tapping
the beat is not a recovery: **the clause that is too wide and the clause that is too narrow are the
same clause**, and that is what made it worth testing rather than adopting.

## What it costs, and it is the visible half

**The key column stops naming only letters.** A badge on a focus-relative row has to name the bay as
well as the press, because the address is half the instruction — a reader told to press `space` and
not told what it lands on has been told nothing, which is the failure ADR-0220 rejected *any program
in this workspace* for. **26 rows carry a key badge naming letters today; five keep a bare key** —
*Tap the beat*, *Halve or double the grid*, *Bring back what is folded*, *Reset the arrangement* — and
the fifth, *Quit*, loses its badge entirely to the platform accelerator. **Twenty-one are rewritten**
to name a bay and a press.

**Nineteen of the 37 `gap` rows become `plan`**, because they were `gap` for want of a route and now
have one: residency, mask shape, mask position, master out, the five sequencer rows, authority, *Take
a parameter back*, *Write a parameter*, both staging acts, both arrangement rows, *Choose where the
frame goes*, *Attach a beat source*, *Composite a deck's renderers* and *Set the free-run tempo*.
**That is a reading of the page rather than a count anything checks**, and each is `plan` only once its
bay draws the control the key hangs on. The remaining eighteen stay `gap` for reasons that are not
about letters: three master effects whose parameters have no spelling, seven rows whose operand is a
path or a name, and eight that are launch-only or a model's.

**Rule 03 of the seven is contradicted as written.** *Density is bought with hover* is a rule about the
pointer — *"Anything compact enough to be a symbol says three things when you point at it"* — and this
record makes the keyboard the way into a dense bay. The list is capped, so nothing is added: the fix is
a reword, *when you point at it or land on it*, which costs no slot. Rules 01, 02, 04, 05, 06 and 07
stand, and 01 was checked rather than assumed — *reachable from the keyboard alone* becomes true of
more rows, and *none the map cannot address* is a claim about operations, not about keys.

**`concepts.html` gains a seventh idea.** *Focus* is used on the console page as though it were
defined, and it is defined only there, halfway down, in a note about how two rings are drawn.

**Two rows the panel deliberately binds no key to may stop being deliberate.** `key_column`'s module
documentation argues at length that *Save the arrangement* and *Put a saved arrangement back* cannot be
bound because *"a bare key press cannot type one"* and because nothing lists them. Under this record
the pill's menu is reachable by focus and the field it opens already exists in this program, so both
arguments are answered by a mechanism rather than by a letter.

## Consequences

- **The four collisions ADR-0220 measured do not arise**, and not because a letter was found: `f g`,
  `r`, `z n` and `j` stop being letters at all. The collision was an artifact of the scheme.
- **`Bring back what is folded` keeps its key and loses its recorded reason.** The page says it unfolds
  everything rather than one region *"because a region a pointer cannot reach is a region a key has no
  way to name either"* — and under this record a key **can** name it: `Tab` to the folded bay and
  `space`. The row survives, the key survives under the operand rule, and **the sentence explaining why
  it is an everything-operation is retired without a replacement.** Whether bulk unfolding earns a key
  on its own is a question this record leaves open rather than answers.
- **`esc` acts on nothing at bay level and must say so.** With no unfocused state there is no rung
  below the bay, and a key that declines silently is indistinguishable from one that is not bound.
- **The key column of *Quit* becomes `gap`**, and no badge names an accelerator. The window's close is
  already handled — `WindowEvent::CloseRequested` — so nothing about quitting stops working; what
  changes is the claim the page makes about the keyboard.
- **The console's three pointers become one mechanism with three instances.** `View::selection`,
  `View::cursor_row` and `View::scope` are each a bay's remembered address. Nothing about what any of
  them means changes.
- **`KEYS` gets shorter and stops being a list of operations.** Thirty-one entries become roughly
  twelve — seven globals, `Tab`, the digits, the arrows, `space` and `enter` — and the sentence beside
  each is about a kind rather than a bay. A legend that fits on a card is the first thing in this
  scheme that is better on day one than on day thirty.
- **`key_column`'s six tests do not fare alike.** Two survive in contract — the scan's floor, and
  `KEYS` equalling what the `match` binds. Two change shape: `ROWS` becomes keyed by a **(bay, key)**
  pair with the globals under no bay, and `NO_ROW` shrinks as `up`, `down` and `space` start reaching
  rows. **Two are replaced** — both directions of the badge check, because a key alone no longer
  determines a row and a badge naming a bay needs parsing rather than a word match. The claims survive;
  the code does not.
- **`Readout::target` and `Readout::enclosing` are what actually retires from the binary.** They
  resolve a region from the pointer for `f`, `g` and `s`, and the focus replaces them. That is
  alternative (b) being rejected, in the two functions it lives in.
- **No operation is added or retired**, so `karakuri-operation`, `gate.rs`,
  `karakuri-operation-record` and `karakuri-store::record` are untouched — which is worth checking
  rather than assuming, and the check is that every arm of the new `match` constructs a variant that
  exists today.

## What this record does not do, and what was not run

**It builds none of it**, and it is a decision rather than a description of the tree
(`docs/contributing.md` §4, *A statement is held true by the thing it describes, or it is deleted*).
Five of the nine bays draw fewer controls than the walk addresses; a digit landing on a control that
is not drawn reaches nothing, which is the same seam every `plan` badge already marks. **The manual is
a separate piece of work and this record edits none of it.**

**`egui` sees every key before the window loop's `match` does, and `Tab` is a key toolkits claim for
their own focus traversal.** `key_column`'s own documentation names this as the half it cannot see:
*"if it ever grew a focused widget that consumed one, the arm would still be here and this file would
go on claiming an operator reaches it."* The console draws no `egui` widget and *"consumes nothing"*
today, so `Tab` should arrive — **nothing was run to confirm it**, and a `Tab` swallowed by the
toolkit is the failure mode in this scheme that looks like nothing at all.

**Every figure here is read rather than computed by a check**: 63 rows, 16 `has`, 10 `plan`, 37 `gap`,
26 rows naming letters, 31 entries in `KEYS`. The 19 gaps that become `plan` are a row-by-row reading
of the page.

## What this leaves undone

- **How the manual says how to quit.** The key column cannot carry two platforms and *Quit* is one row;
  a legend line, a sentence on the console page, or nothing at all are the three answers, and none is
  taken here. **Deliberately not urgent**: the maintainer's reading is that this is a window
  application and a window application's quit is the window manager's, so an operator arrives already
  knowing the gesture and the page is not where they would look for it. It is written down so that
  the silence is a choice somebody made rather than a hole nobody saw.
- **Whether bulk unfolding earns a key on its own**, now that the reason recorded for `z` is gone.
- **What the mark for a folded bay holding focus looks like.** It has to say *there is a bay here and
  `space` opens it*, in a place the arrangement gives no rectangle to. That is the mock's, and it is
  the one drawing this record creates a need for.
- **Where the digit hints are shown.** Rule 03's third thing — *what a press will do* — is where they
  belong, and whether that is a numeral in a bay head or a line in every tooltip is the console page's
  to decide.
- **Whether the Sequencer's steps ever take digits.** Sixteen steps outrun ten keys, so the walk is the
  primary route there and the digits reach nine of them; nothing here decides whether that is worth
  keeping.
