---
id: 0159
title: The console's words are the manual's, and the middle one is not a pane
status: accepted
date: 2026-08-24
supersedes: []
superseded_by: []
principles: [0093]
tags: [ui, naming]
---

# The console's words are the manual's, and the middle one is not a pane

## Context

[ADR-0156](0156-the-consoles-arrangement-is-a-tree-this-repository-owns.md) is built on one
sentence: an operation is named once, and the panel, the keyboard, a MIDI map and MCP all route
into that name. The first arrangement written against that ADR broke it within a day, and counting
the words showed how far:

| word | `console.html` | `karakuri-layout` | `karakuri-console` |
|---|---|---|---|
| bay | 16 | 0 | 15 |
| pane | 1 | 9 | 26 |
| view | 3 | 33 | 15 |
| split | 1 | 107 | 7 |

The manual calls a titled box a **bay** — sixteen times, and its stylesheet class is `.bay-head`.
`karakuri-console` called the same thing a **pane**, twenty-six times. One thing, two words, and the
two documents that have to agree are the operator's manual and the code the four surfaces call
into. That is [P-0093](../principles/0093-a-statement-is-held-true-by-the-thing-it-describes-or-it-is-deleted.md).

The manual also had a hole. It names every bay and it has **no word at all** for the three columns
those bays are stacked in — which was harmless while nothing addressed them, and stopped being
harmless the moment *fold the left one away* became an operation.

## Decision

**Where the manual has a word, the manual's word wins**: a titled box is a **bay**, a mixer channel
is a **strip**, a slot on the deck is a **deck**. Where it has none, one is chosen here and goes
back into it.

**The three columns are `left pane`, `centre` and `right pane`.**

**The middle one is deliberately not a third pane.** The side panes are the things an operator
folds away to give room; the centre is what they fold away *for*, and folding it is not an
operation anyone would want — the thing you want there is `solo`. Calling all three by one noun
asserts a symmetry the console does not have. All three are still named by position, which is at
least consistent, and *"fold both panes away and the centre takes the window"* needs no gloss.

**`karakuri-layout` keeps its generic words.** A leaf is a `view` and an interior node is a
`split`, because that crate not knowing what a console is, is the whole of why it exists. The
console's vocabulary stops at the crate boundary, and `karakuri-console` uses `view` and `split`
only where it is talking about what it hands over.

**The transport and the outputs are rows, not bays.** The scheme was written from the seven titled
boxes and left those two out, and the mock is ambiguous about them on purpose: both carry
`class="bay"`, because that class is the card styling, and neither carries a `.bay-head`, because
neither has a heading. A bay is a titled box; these are strips of readouts, and `strip` is already
the mixer's channel. They are **rows**.

**A common noun is not an identifier.** The inspector's *n* subdivisions stay `panes`, which is
what the manual already calls them, even though the side columns are panes too. P-0093 is about
names: `left-pane` and `inspector-1` are distinct and always will be. A generic noun used
generically costs nothing, and the inspector's are always spoken of qualified.

## Alternatives

**`left pane`, `view`, `right pane`** — the middle named by what it is for rather than by where it
is. It was the proposal on the table and it lost on two collisions, both of which could have been
paid off and neither of which was worth paying. `karakuri-layout` calls every leaf a `view`, and
the centre is not a leaf but a split — so the word would name the thing it is not. And the manual
already writes *"the program view"* for the picture itself, in the `solo` tooltip, so `the view`
and `the program view` would sit one word apart in exactly the sentence that has to tell them
apart. Freeing the word was possible — rename the crate's leaf, restate the tooltip — but what it
buys is a functional name for a column whose lower half is not something you view: the manual's own
*"Mixing lives between decks; shaping lives inside one"* puts shaping there, and `view` names only
the top half.

**Three of a kind — `left pane`, `centre pane`, `right pane`.** What the code already said, and the
tidiest-looking. Rejected for the asymmetry above: the three are not three of a kind, and a name
that says they are is a name that will be believed.

**`column` for all three**, following the mock's own `.col` class. Rejected on how it reads in the
operations it exists for: *"fold the left column"* is stiff where *"fold the left pane"* is what
somebody would actually say, and an operation's name is read far more often than it is written.

**Rename the bays to panes and change the manual instead.** Rejected on which document is the
reference. The manual is written ahead of the panel on purpose, and when it and the code disagree
about a word, the code is what moves.
