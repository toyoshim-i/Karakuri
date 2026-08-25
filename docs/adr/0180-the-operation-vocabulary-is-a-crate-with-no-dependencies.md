---
id: 0180
title: The operation vocabulary is a crate with no dependencies, and the manual is its specification
status: accepted
date: 2026-08-25
supersedes: []
superseded_by: []
principles: [0074]
tags: [ui, architecture]
---

# The operation vocabulary is a crate with no dependencies, and the manual is its specification

## Context

`docs/manual/index.html`'s **first of seven rules**: *"One vocabulary, four ways in. Every operation
is reachable from the panel, from the keyboard alone, from a mapped MIDI control, and from a model
over MCP … **because they are the same named operations underneath.**"*

There were no named operations underneath. There were four partial lists: `karakuri-midi`'s
`Action` (8), `karakuri-console`'s `panel::Op` (8), the CLI's key handler (35 match arms), and six
MCP tools. `docs/manual/operations.html` enumerates **46** operations with their four routes, and
nothing in the code knew about it.

**The frame this is built to** is the project owner's: **MCP is the most direct form of operation** —
a model reads the published information and speaks Karakuri's own terms. **A mouse cannot operate
Karakuri at all; it operates a GUI component, and the GUI component is a translator** — motion into
a number or a boolean sent to a target, and the value read back and turned into a picture. MIDI's
translator is the mapper; the keyboard has one too. **Everything except MCP needs a translator**, so
a GUI component maps one operation to N input methods, and it is where the other routes are
anchored — the manual starts MIDI learn from a control's tooltip.

## Decision

**`crates/karakuri-operation/` — a leaf crate with no dependencies at all**, `std` only.

The surfaces force it. `karakuri-console` brings `egui` and `karakuri-layout`; `karakuri-midi`
brings `midir` and nothing else; `karakuri-cli` brings a GPU; the MCP server lives inside the CLI.
Anywhere else and a MIDI map could not be parsed without a toolkit. **Putting it in
`karakuri-engine` would make it the one thing it must not be** — engine-shaped, when the whole point
is that it names what an operator asked for rather than what the engine does about it.

The name is the manual's word, used 46 times on the page that specifies it. `karakuri-vocabulary`
was rejected: `karakuri-store` already owns `Vocabulary` for *which file a record belongs in*, and
a name means one thing across this system. `karakuri-command` was rejected because the manual's word
wins where the manual is the specification.

**`Operation` and its titles come out of one macro invocation.** A hand-written title list beside a
hand-written enum is precisely the *two lists agreeing* this type exists to abolish — a variant
added without a title would leave the check passing over an operation nobody specified. Through the
macro, a variant cannot exist without its title.

**A test reads `docs/manual/operations.html` and asserts both ways round**, in
`gpu_tests_are_under_mod_gpu.rs`'s shape. Four tests that fail apart: membership in each direction,
that every heading on the page opens a row (a decorative one would otherwise be an operation the
check never asks about), that every title is plain text (an `&mdash;` would read as one operation
missing and one invented), and that the two are in the same order.

**Nothing is migrated.** `Action`, `panel::Op` and the CLI's match arms are untouched and still
work. This lands the target; the surfaces move onto it one at a time, and each of those is a change
that can be reasoned about because the target has stopped moving.

### `Undecided` is a type, not an empty payload

Five places carry it. An empty payload asserts *this operation acts on nothing*, which is false for
all five; a guessed payload is worse than a named gap, which is this repository's standing rule
about not declaring what does not exist. Being a type, the day a decision lands, replacing it is a
compile error at every construction site rather than a search.

## The contradiction this found, and P-0074

`karakuri-midi::Action`'s doc says it is **engine-neutral on purpose**: *"No `Residency`, no
`Blend`, no `Record`: this crate names the gesture and `karakuri-cli` decides what it is worth."*

`karakuri-console::panel::Op`'s doc argues at length that a vocabulary must have **no toggles**:
*"A toggle is an affordance built over two operations by whoever draws it … a MIDI map with a
button per direction, an MCP call that says which one it wants, and a keyboard, all have to be able
to say* fold this *and mean it."*

**Two documents that each claim to be the vocabulary idea, contradicting each other head-on** —
`Action` is `ToggleOnAir`, `TogglePriming`, `CycleBlend`.

And the *why* is the useful part: **a crate that refuses to name a value cannot say *set blend to
over*, so the only gesture left to it is *step it*.** The two rules are not jointly satisfiable
unless the vocabulary owns the value lists. So it does — blend modes, sync modes, tone map
operators, curves, wipe shapes are enumerations here — and the cost is stated rather than hidden:
they are third spellings of lists that exist twice already, converted by one `From` per list in the
one place every control already ends (P-0028). That is
[P-0074](../principles/0074-an-operation-says-what-it-wants-never-which-way-to-move.md).

`Action::Preview { slot: Option<u8> }` was already right and for the right reason — *"direct rather
than a cycle: a surface has a pad per slot and reaching slot 3 through three presses is a
keyboard's compromise, not a surface's"* — and both it and its argument are kept.

## Rows that resisted, and what was decided rather than guessed

- **Wire a procedure's input to a node** — already decided, in a place that had to be found.
  `Record::Edge`'s doc: an edge is *"addressed by name at both ends, which is the one record that
  is"*, because a position moves when a list is reordered. So the two spellings in the workspace are
  **not a drift to resolve**: the vocabulary carries both, positional for parameters and capacity,
  by name for an edge.
- **Element capacity, seeds, the camera** — three operations under one heading, carried as three
  variants of one property, because splitting the row is a change to the page and the page is the
  specification. **Send a Set to somebody, and take one in** — two, and not inverses: one names
  something the store holds, the other hands the store something it has never seen. **Choose the
  wipe shape, the quantum, the length** — a fourth three-in-one row nobody had flagged, pressed by
  three keys.
- **Tone map and exposure is two-in-one and stays one variant**, because `Record::Look` is one
  record and gives the reason: a stream that set the exposure without saying which operator it
  applies to describes a look nobody can reconstruct.
- **Nudge the latency offset is absolute; scrub is relative**, and the asymmetry is deliberate.
  Absolute expresses every nudge and a nudge cannot express a setting, so the offset is absolute and
  `o`/`p` are two translations of it. Scrub is the exception because **nothing in the instrument can
  set a position** — the manual says accumulating material cannot be moved to one, so an absolute
  beat would invent an operation that does not exist for two thirds of the material.
- **Folding and solo are addressable by name**, which was not expected: `Layout::new` refuses an
  arrangement that uses a name twice, so *the answer is the only node that could be meant* — and a
  region name is something a key, a map and a model can all say.

**Left `Undecided`, each with the decision named at the variant.** *Move a boundary*, because the
manual itself says *"how a pane is sized and unfolded without a mouse is not decided"* and two
things are missing: most splits are unnamed, and `set_divider` takes a viewport pixel, which a key
press or a model has no way to mean. *Choose where the frame goes*, because **no output has an
identity anywhere in this workspace** — `Sink` is a trait with no name and no id, and the console's
Outputs row reaches its one sink by folding a layout region, so the only route that exists is
spelled as a fold. *Walk the edit history*, because there is nothing anywhere to read a payload off.
*Edit the file instead*, because **that row names an event rather than an operation** — somebody
editing a file in another program is not something a surface performs, and the nearest operation is
*watch these files, or stop*, which `--watch` cannot express. And the camera, because
`Record::Camera` describes a built-in orbit while L3 is a procedure layer whose parameters are
written like any other node's, and which of those *is* the camera decides whether the arm exists.

## Consequences, and what the vocabulary found by existing

- **`panel::Op`'s `Reset` and `Report` have no row on the page** — two operations the manual does
  not know about.
- **"Move a boundary" has no `Op` variant.** Dragging goes through `press`/`moved`/`released`
  entirely outside the operation vocabulary, so the one row the manual marks as pointer-only is also
  the one the console does not model as an operation at all.
- **`Fold` and `FoldEnclosing` do not line up with the manual's two fold rows.** The manual splits
  on *what kind of region*; the panel splits on *the node or the split around it*. Different axes,
  and whether they are one operation over two kinds of region is a question for the page.
- **The manual's *deck* is the code's *slot*, and the code's `Deck` is the whole mixer.** The
  vocabulary takes the manual's word, since the manual is its specification, and says so.
- **`Blend` already meant two things** before this crate existed — `deck::Blend` is how a deck meets
  the mix, `ast::Blend` is how an L4's fragments meet each other. A third spelling was not added; the
  vocabulary's is named for what it is.
- **`--canvas` is on *Size the window*'s CLI badge and is not that operation.** `--size` is the
  preview window; the canvas is what is rendered, fixed for the run and recorded. The page's own gap
  section lists canvas size among what is reachable from nothing while running, so the page treats
  it as a thing of its own in one place and folds it into the window in another.
- **`Record::Look` carries a `white_point` with no control on any surface and no row on the page** —
  a 47th operation, or a field nobody can reach.
- **"Crossfade to the next deck" has a panel route the operation cannot be.** Its badge says
  *crossfader*: a continuous, immediate position control. The `x` key is a scheduled pair of fades
  landing on the beat grid. One of those needs a different name or a different control.
