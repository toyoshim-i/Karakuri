---
id: 0305
title: The Library bay's load is a button and a pulldown, and the deck it names is not the selection
status: accepted
date: 2026-09-08
supersedes: []
superseded_by: []
principles: [0085, 0087, 0090]
tags: [console, ui, operations, library, mixer]
---

# The Library bay's load is a button and a pulldown, and the deck it names is not the selection

## Context

The Library bay's foot read `load → A`, and it was a **readout**. The cursor said which Set, the
deck selection said which deck, and the pill said where a press would land *before* it was made —
nothing in `karakuri_console::input::claim` hit-tested any of it, and `tests/library.rs` carried a
test whose name was the claim: *the load pill takes no press*. That arrangement is what made a load
*"a cursor and a key with no pointer anywhere in it"*
([ADR-0264](0264-a-reading-is-one-question-and-only-the-opening-asks-it.md),
[ADR-0265](0265-a-carried-set-names-its-deck-at-the-release-and-the-panel-refuses-no-drop.md)), and
`docs/manual/console.html`'s *How a Set reaches a deck* had argued it in as many words: the answer
that satisfies rule 01 *"needs nothing added, and the answer that does not — a destination chosen in
the library's own foot — would be a third selection on a panel that already has to explain two."*

**The maintainer asked for that third selection.** [roadmap.md](../roadmap.md)'s M5.3 had the
question written down as open — *"whether it should be split — `load` a button, `→` a label, `A` a
pulldown over A–D — which would let a load name a deck without moving the selection"* — with the
cost named beside it: a third route to naming a deck, and the keyboard-only property spent. The
answer came on 2026-09-08, verbatim:

> readは何のために作られたのか不明、load → Aとなってるボタンをloadボタン、→ラベル、Aプルダウンメニューにして、プルダウンでスロット選んでからloadクリック

*The `load → A` button becomes a `load` button, a `→` label and an `A` pulldown menu; you pick the
slot in the pulldown and then click `load`.*

**The keyboard-only property is what the question turned on, and it was answered the same day**, on
the same thread:

> 2はキーボードからはa-dでスロット選択、リターンでsendとか、やりようはいくらでもある。

*From the keyboard the split control is reachable too — `a`–`d` picks the slot and return presses
send, for instance; there are plenty of ways to do it.* So the property is **kept and not spent**:
what is owed is a key route to a control, not a redesign. Which letters, and whether it is a letter
at all or [ADR-0259](0259-the-keyboard-is-addressed-to-the-bay-that-has-focus-and-a-global-letter-is-a-convenience-or-the-operators-own.md)'s
focus grammar — `space` and the arrows on the pulldown, `enter` on the button — is **M5.13's to
spell**, and this record binds no letter: `a` is already promised elsewhere
(roadmap M5.13, *The letters still promised and still free*), and ADR-0259's whole argument is that
a letter chosen for one control at the bay that wants it is how the last scheme ran out.

## Decision

**The readout is three things: a `load` button, a `→` label and an `A` pulldown over the decks the
mixer is drawing strips for. A press on the button loads the cursor's Set onto the pulldown's deck,
and the deck selection does not move.**

- **The pulldown is the Library bay's own mark** — `View::target`, beside `View::selection` and
  `View::cursor_row` as a third pointer this console owns. It starts on deck A and is kept across
  everything that is not a deck: changing the scope, narrowing the listing and walking the cursor
  all leave it alone.
- **A pick emits nothing.** `view::Aim::Deck(u8)` moves the mark and puts the list away, and no
  operation names it. `Operation::SelectDeck` is emphatically not what it is: that operation moves
  the keys, and this mark exists to be able to name a different deck.
- **The button emits `Operation::LoadSet { deck, set }`** with `deck` off the pulldown and `set` off
  the cursor — the payload the vocabulary already had, unchanged. With no row under the cursor it
  answers `Aim::NoSet` and emits nothing.
- **`l` is unchanged and still loads onto the *selection*.** That is this record's one assumption
  and it is left for the maintainer: nothing found in the manual, the roadmap or ADR-0228 says the
  key should follow the new mark, and moving it would take the deck selection's one reason for
  existing — *"what every deck-addressed operation's keyboard translator fills its `deck` in
  from"* — away from the row that has always read it. If it should follow the pulldown instead,
  that is a one-line change in `crates/karakuri/src/main.rs` and a paragraph on two pages.
- **The pulldown offers only decks the mixer draws a strip for**, which is `View::select`'s own
  refusal read a second time rather than a second rule — the strips are what both of them count.
- **A list that is down claims every press on the console**, which is `input::claim`'s rule 2 with
  a third card added to it. The mechanism is the arrangement pill's menu reused whole
  ([P-0085](../principles/0085-take-the-mechanism-that-exists-and-pay-the-bill-now.md)): a pulldown
  already exists on this console, twice.

**The letter is not drawn in lavender**, and that is part of the decision rather than styling.
Lavender on this console is *the deck the keys are addressed to*; this is the one letter on the
panel that may name a different deck, so it is drawn as every other capsule in that foot is and the
bay's one accent stays on `load`, which is the act.

## Alternatives rejected

**Keep the readout.** The alternative with a written argument already behind it, and the one that
was in the tree. `console.html` had it: a load is *"a cursor and a key with no pointer anywhere in
it"*, rule 01's constraint met by the arrangement rather than by a second control, and a
destination chosen in the library's own foot *"would be a third selection on a panel that already
has to explain two"*. ADR-0264 and ADR-0265 both rest on it, and ADR-0265 turned down *"press a
strip and open a picker of Sets under it"* partly by quoting it.

It loses to what the readout cannot do. **The deck a load lands on and the deck the keys are
addressed to were one value**, so preparing deck C while playing deck A meant moving the keys off
deck A to do it — and the keys are what every other deck-addressed operation reads. The
counting argument survives and is paid rather than avoided: there are three marks now, and each is
explained by what it is addressed to — the keys, the listing, and the load. The keyboard argument
does not survive at all, because the premise it rested on was that a split control could not be
reached from the keyboard, and the maintainer's second sentence says how it is.

**A new operation for the pick.** The obvious shape once a control moves something: give the
pulldown a row on [every operation](../manual/operations.html) and a variant to emit. It is refused
by what the pick *is*. It changes which deck the **next press** will name and nothing about what
any deck is playing — the same class as the list cursor above it, which
`console.html` argues has no row for exactly this reason: *"a row here would be the first rule
written down as a permanent gap — three columns that can never be anything but empty, on an
operation that exists to move a highlight"*. A map cannot address it (a pointer inside one console
is not the instrument), a model has no use for it (`load_set` names its deck outright), and the
keyboard's route to it is M5.13's. So the row would be one built column and three permanent gaps,
for a mark this console is already the model of record for
([P-0087](../principles/0087-name-the-property-never-the-shape.md): the console holds it because
every other derivation of it would go stale).

**Make the pick `Operation::SelectDeck` — the pulldown drives the selection.** Cheaper than a new
variant, and it would keep one deck mark on the console instead of two. It is refused because it is
the decision inverted: *"the deck it names is not the selection"* is the whole of what was asked
for, and a pulldown that moved the ring would be `0`–`3` with a pointer on it. It would also make a
press in the Library bay change what every deck-addressed key does next, which is the surprise the
selection's persistence exists to prevent — *"it stays put when you tab away into the library"*.

**Let the button read the selection and give the pulldown to something else.** The smallest change
that draws what was asked for: three elements in the foot, and the load still lands where the ring
is. It is the readout with a chevron on it, and it fails the sentence that asked for the split.

**Put the deck list in the operations page as *Select a deck*'s second panel route.** Tempting
because a pick names a deck from a closed list and *Select a deck* is a row that exists. Refused on
the same count as the new operation: the pick moves no selection, so claiming that row would make
the page say a control reaches an operation it does not.

**Hang the list down from the pulldown, as the arrangement pill's menu hangs.** The precedent, and
the reason it is not copied is where each control sits. That pill is in the transport row at the top
of the console and its card has the whole window under it, which is why `menu_card` counts rows
against the room and says `n of m` where a store holds more than fits. This one is in the **foot**
of a bay, and what is under it is whatever the operator has dragged the boundary to. So the card stands
on the capsule's top edge and hangs up over the bay's own list, and it is not counted against the
room: it lists at most four rows, and a window too short for four rows of type has no transport row
in it either.

## Consequences

- **`crates/karakuri-console/src/view.rs` carries `Target`, `Aim` and `Load`** where it carried
  `LoadPill`. `Load` is the three rectangles and their contents — `button`, `text`, `arrow`, `deck`,
  `letter`, `chevron` — plus `rows`, which is `Target::decks` while the list is down and zero while
  it is shut, so `Load::row` cannot hand out a rectangle for a list nobody opened.
  `LibraryBay::load` takes a `Target` and no longer takes a letter: the letter is
  `Target::letter()`, because the list's rows need letters too and one derivation is what the
  paint and the press share.
- **`LibraryBay::read_chip` and `LibraryBay::read` take a `Target` as well**, because the `read`
  chip is measured back from the `load` button and the button's width is the row's arithmetic.
  `LibraryBay::pill` now places *the far end of the foot*, which is the pulldown.
- **`View` gained `target` and `target_open`, both private.** `View::aim_at` is the only way into
  the first and refuses a deck the mixer draws no strip for; `View::open_target` and
  `View::shut_target` are the only ways into the second, and `open_target` refuses a console with no
  strip at all — a card with no rows offers nothing to pick and nothing to leave by, and rule 2
  would hand it every press until a second one shut it. `aim_at` puts the list away as well as
  moving the mark, because a pick emits nothing and there is no host arm for the gesture to end in.
- **`input::PROBES` has a row claiming two**, `"the Library bay's load button and deck pulldown"`,
  so `input::CONTROLS` rose by two and the legend `crates/karakuri/src/main.rs` prints rose with it.
  One row rather than two because the two capsules are one derivation and one ask — a mixer strip's
  five is the same shape — and `crates/karakuri/src/main.rs`'s `press_handler::ASKED` has the one
  matching entry, naming `bay.aim(`.
- **`input::claim`'s rule 2 has a third card in it.** `view.target_open()` sits beside the
  arrangement pill's menu, the audio-in pill's card and a pane head taking letters. The three cards
  can never be down together, for the reason the first two never were.
- **`crates/karakuri/src/main.rs` gained `Readout::aimed`**, beside `Readout::arranged` and in its
  shape, and the press-handler arm that feeds it sits with the two cards' arms rather than with the
  Library bay's other five: the card is drawn over that bay's own rows, so a press has to reach it
  before the rows do.
- **The vocabulary did not change.** `Operation::LoadSet { deck, set }` is what it was and its title
  string is untouched, so `the_manual_and_the_vocabulary_agree` and
  `karakuri-console/tests/panel_column.rs` both hold: `LoadSet` was already emitted from
  `karakuri-console/src` by the drop, and the button is a second construction of a variant the page
  already marks `has`.
- **`docs/manual/operations.html`'s *Load material into a deck* row** carries
  `panel load button, or library → deck` where it carried `library → deck`, and its tip describes
  three routes naming the deck three ways. The row's *key* column still reads `l`, and the MIDI
  column is still `gap` for the reason it was: a Set id is not a slot, a range or a word from a
  closed list.
- **The pulldown's own MIDI line is a different refusal, and the page says so.** A map line does
  name a deck — `note → residency N live|priming|allocated` — so the load capsule's reason does not
  apply here. What is in the way is that this is a *pointer inside one console*, which is exactly
  why *Select a deck*'s MIDI column is `gap`; the tip in `console.html` points at that row.
- **`docs/manual/console.html`'s *How a Set reaches a deck* is rewritten** — the foot is three
  things, the key reads the other mark, the letter is the whole warning and is not lavender, and the
  drag is *"a third route to the same command, and never the first"* where it was the second. The
  paragraph the readout rested on is kept as history rather than deleted, with what it lost to
  beside it. Two sentences elsewhere on the page moved with it: the strip's tip and *Two ways into
  the selection*, both of which said the load pill's letter read the selection.
- **The `→` is drawn and is not a control.** It is `LOAD_ARROW`'s mark rather than a U+2192 for the
  reason it always was, it is painted in `--c-faint` — `.lib-foot`'s own colour, the colour the
  count at the other end of the row is in — and it is untipped in the mock, because that page tips
  controls and this is punctuation.
  `the_label_between_the_two_capsules_takes_no_press_and_a_row_is_where_the_drag_begins` is the
  renamed test that holds it, and the half of its predecessor that still stands — the drag begins on
  a row — is kept whole inside it.
- **Seven tests in `crates/karakuri-console/tests/library.rs` and one in
  `crates/karakuri/src/main.rs` hold the decision** — four new, three whose premise moved — and the
  two that would have passed against the readout are the ones worth naming:
  `a_press_on_load_asks_for_the_deck_the_pulldown_names` pulls the two marks apart before it presses
  — keys on deck C, load aimed at deck B — because a console where they agree cannot tell the two
  readings apart, and `the_foot_says_which_deck_a_press_would_land_on` moves the *selection* with
  the target standing still and reads the foot again.
- **What is owed is a key route, and it is owed to M5.13.** Nothing on this panel takes keyboard
  focus yet — `console.html`'s *Two focuses, and they do not look alike* draws the dashed ring and
  view.rs says nothing draws it — so until the grammar lands the pulldown is reached with a pointer
  and `l` is the keyboard's whole route to the row. That is written into the page as a gap rather
  than a decision, and into M5.3.
