---
id: 0315
title: A model has no window, so the twelve surface rows' MCP badges are gap
status: accepted
date: 2026-09-09
supersedes: []
superseded_by: []
principles: [0082, 0090]
tags: [manual, operations, mcp, console, surfaces, m5]
---

# A model has no window, so the twelve surface rows' MCP badges are gap

## Context

`docs/roadmap.md`'s *The console's own shape* is the section holding the four rows whose panel home
is not a bay — *Fold a bay away*, *Fold a pane away*, *Size the window* and *Quit*. Two of them were
met by [ADR-0300](0300-a-pane-folds-by-dragging-its-boundary-out-and-comes-back-by-dragging-it-in.md)
and by the bay-head grip becoming a hit test. The other two are the host's, and the section carried
three questions rather than any work:

1. Whether the *second, larger loosening*
   [ADR-0281](0281-every-route-reaches-every-write-and-a-read-is-the-routes-own-interface-design.md)
   named and declined — *a write is what a replay has to reconstruct* — is taken. Under it the twelve
   operations `karakuri-operation-record` files as `Silent::Surface` stop being writes and eighteen
   `plan` badges stop being debt, twelve of them MCP's.
2. What *Quit*'s panel badge says, where it read `plan` naming a `close` control **the mock does not
   draw** — a page gap before it is code, under `docs/contributing.md` §5 step 3.
3. What *Size the window*'s key badge says, where it read `plan` naming `a`, which is bound to
   nothing — and where `a`'s payload would have to say what a key press means on a page whose own
   *Move a boundary* tip says *"setting a divider takes a viewport pixel, a key press cannot mean
   one."*

The recommendation put to the maintainer was: refuse the loosening and make the twelve `gap` with one
sentence; make *Quit*'s route the window's own close; leave `a` unbound and make the key badge `gap`.
His answer, on 2026-09-09, was 「推奨どおり (3 つとも)」 — as recommended, all three.

### The twelve are read off the arm, not off a list

`written`'s `Silent::Surface` arm holds **thirteen** variants, and the roadmap's list of twelve omits
`SetFavourite`. That is not an error in either: a star's MCP answer was taken separately and for its
own reason in
[ADR-0301](0301-a-models-star-is-refused-because-a-favourite-has-no-sandbox-to-land-in.md), and this
record does not restate it. The twelve are `FoldBay`, `FoldPane`, `Unfold`, `Solo`,
`ResetArrangement`, `SaveArrangement`, `RestoreArrangement`, `SelectDeck`, `SelectScope`,
`SetTransition`, `SizeWindow` and `KeepCandidate`. **Every one of them read `plan` for MCP before
this record**, and none read `has`, so nothing here takes a route away from anybody.

## Decision

**1. The loosening is not taken, and the twelve rows' MCP badges become `gap`.**

They stay writes. Rule 01 goes on asking all four routes of a write, and what moves is one column's
answer on twelve rows. The sentence is the same on all twelve, in each row's tip:

> The MCP badge is empty because a model has no window: this is a surface's own state, and a route
> into a surface's own state is a route into a window the model is not looking at.

It is [ADR-0281](0281-every-route-reaches-every-write-and-a-read-is-the-routes-own-interface-design.md)'s
own argument read on the model's side. One sentence rather than twelve is
[P-0083](../principles/0083-a-refusal-carries-what-the-next-attempt-needs.md) and
[ADR-0131](0131-one-refusal-sentence-per-mistake-across-the-surfaces-that-face-a-person.md): one
refusal per mistake, worded once, so twelve rows cannot drift into twelve explanations.

**2. *Quit*'s route is the window's own close, and the console draws no `close` control.**

The panel column is `gap` — *this surface cannot reach it at all* — because no `close` is drawn on
the console and none is planned. The route is real and the badge that says so is in the fifth cell,
`has`, reading `window close`: the window manager draws the control and `crates/karakuri` answers
`WindowEvent::CloseRequested`, waiting on the saves and the recording before the loop exits. `esc`
keeps its `has`, because `esc` quits **today**; the tip says it is going to stop, which is
[ADR-0259](0259-the-keyboard-is-addressed-to-the-bay-that-has-focus-and-a-global-letter-is-a-convenience-or-the-operators-own.md)'s
decision and not this one's.

**3. `a` is not bound, and *Size the window*'s key badge is `gap`.**

For the page's own reason, one level out from where it is already written: a size is a pair of
viewport pixels and a key press cannot mean one, which is *Move a boundary*'s tip read at the window
instead of at a divider. `a` is bound to nothing in `crates/karakuri` and is not going to be.

## Why the badge went in the fifth cell rather than the panel column

**`panel_column.rs` decides it, and it decides it mechanically.** A `has` badge in the panel column
demands that some line of `crates/karakuri-console/src` construct that `Operation`, in both
directions;
[ADR-0288](0288-a-read-rows-built-badge-is-met-by-an-emission-or-by-a-drawing-this-file-can-find.md)
adds a second way to meet one and it is available only to a row the page marks `read`. *Quit* is a
write, nothing in that crate constructs `Operation::Quit`, and nothing ever will — the roadmap's
fourth instrumentation command already returns *sizing the window and quitting* as the two the whole
workspace never constructs, because both are `winit` events. So `has` in the panel column would have
failed `every_panel_route_the_page_marks_built_is_emitted_by_a_console_control`, and the only ways to
pass would have been to invent an emission or to weaken the assertion. Both are the lie that file's
own header names: *"the badge would be a lie the page tells on its own authority."*

**The fifth cell is where the page already puts a way in that is none of the four**, and until now
that has always been the command line. It carries `window close` on this one row. The four
instrumentation commands read a column **by name**, so a badge there is counted by none of them and
no meter moves.

## Alternatives rejected

### a. Take the loosening — *a write is what a replay has to reconstruct*

**What it had going for it.** It is one sentence, it makes twelve `plan` badges honest at a stroke
without anybody deciding twelve times, and it draws a line that already exists in the code: `written`
answers `Silent` for these and the whole of what a record is for is a replay.

**It loses to a principle by name.**
[P-0082](../principles/0082-looking-never-writes-back.md) says *"Only an explicit operation — a
divider dragged, a pane folded, a frame advanced — changes stored state"*, and a divider dragged and
a pane folded are two of the twelve. The proposed sentence would make them not-writes while the
principle names them as the examples of what a write is. By
[ADR-0249](0249-a-principle-is-what-decides-a-question-it-does-not-mention.md)'s gate this is not
even the interesting case: it is a principle deciding a question it *does* mention.

**And it cuts wider than the list it was offered for.** Read literally it takes `Silent::NoRecord`'s
rows as well, which would bless the hole `karakuri-environment`'s MCP tests pin rather than bless —
`wire_input` writing no record is asserted there as a gap somebody owes.

**What it would have bought is only the badges.** Rule 07 holds the panel and the keyboard whatever
rule 01 says — *"every divider drags, the program view can take the whole panel"* — so nothing here
becomes unreachable by hand either way; MIDI cannot address any of the twelve and is already `gap` on
all of them. Twelve MCP badges are the entire prize, and they are had for a sentence.

### b. A `close` control drawn on the console

**What it had going for it.** It would make the panel column honest by building the thing the badge
named, which is what §5 step 3 asks for anywhere else: *"a control drawn in the mock with no
operation behind it is a specification; a control the panel draws that the mock does not is a
defect."* It would also let the badge stay in the panel column, where the meter reads.

**It loses because the window already has one and a second is worse than none.** The window manager
draws a close on every platform this program runs on, and an operator's hands already know it. A
console-drawn `close` would be a second control for one act, in a different place on every platform,
and — being a control on the panel — reachable by the pointer that is also dragging faders during a
performance. [P-0094](../principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)
is why quitting is not on a ladder of `esc` presses in ADR-0259, and it is the same argument here.

**It also spends a slot on the one act nobody needs help finding.** The panel is dense because a
performance is dense (rule 03); the acts that earn a control are the ones a performance repeats.

### c. Bind `a` to a size step

**What it had going for it.** The badge already named `a`, `karakuri-cli` binds `a` at its own
window, and a key on the panel would take *Size the window*'s key column to `has` — one of two `plan`
badges left in this section.

**It loses on what the payload would have to say.** `Operation::SizeWindow { width, height }` names a
size, and a key press cannot: a step is *bigger* or *smaller*, which
[P-0090](../principles/0090-a-surface-offers-it-never-decides.md) refuses by name — *"An operation
that says toggle, cycle or step: a surface that can only step has no way to arrive, and two surfaces
stepping one control disagree about where they are."* The page had already written the sentence one
row along, for a divider.

**`karakuri-cli`'s `a` is the counter-example and it is not one.** `Live::snap_to_canvas` asks the
window manager for the canvas's own size — a **fit**, not a size somebody said — and prints the
granted size when it is clamped, because the manager is the one that decides. That is a key naming a
*thing to fit to* rather than a viewport pixel, and it is the shape a panel `a` could take one day.
It is also `karakuri-cli`'s keyboard, which is
[ADR-0220](0220-the-key-column-is-the-instruments-keyboard-and-the-clis-keys-are-its-own.md): the key
column on this page is the instrument's.

### d. A fifth column of its own for the window

Refused in advance by ADR-0259, which met the same problem from the key side and wrote *"inventing a
fifth column for one row is worse than the sentence it would replace."* **That refusal is honoured
rather than overturned**: no column is added. The page has five cells per row, the fifth has always
been the way in that is none of the other four, and one row's fifth cell now names the window instead
of a flag. The thing ADR-0259 could not do was carry `⌘Q` and `Alt-F4` in a column whose badges are
instructions to press a key; the window's close is **one** control on every platform, which is the
difference.

## Consequences

- **Twelve rows of `docs/manual/operations.html` read `gap` for MCP** where they read `plan`, each
  with the same sentence appended to its tip. Checked: the twelve are the arm's members less
  `SetFavourite`, and every one of them was `plan` before, so no route is withdrawn from a caller.
- **No mechanical check moves.** `karakuri-environment/src/mcp.rs`'s pair compares against `"has"`
  and nothing else, so a `plan` becoming `gap` is invisible to it; `panel_column.rs`,
  `vocabulary.rs` and `key_column` read other columns. Verified by running all four.
- ***Quit*'s panel badge is `gap` and its fifth cell is `has`, reading `window close`.** The row now
  says how to quit in a badge rather than only in prose, which is what ADR-0259 left open —
  *"what the page still owes is to say somewhere how to quit."*
- ***Size the window*'s key badge is `gap` naming nothing**, and the row's tip carries the reason and
  the `karakuri-cli` distinction. `karakuri-operation`'s `SizeWindow` doc said *"The `a` key is a
  translation that asks for the canvas's own size"* with no owner on it; it now names the program
  whose key it is.
- ***The console's own shape* states its exit and does not meet it.** The condition is ADR-0226's
  grep over its four rows: *Fold a bay away* `has`, *Fold a pane away* `has`, *Quit* `gap`, and
  **_Size the window_ still `plan` on a drag** — which waits on neither of the two things this record
  answered. Its tip already says what it waits on: the output that would hold a size of its own is
  unbuilt.
- ***Move a boundary*'s MCP badge stays `plan`.** It is the thirteenth row the roadmap names as
  waiting on the same answer, and it is not in the `Silent::Surface` arm — `written` files it as
  `Owed(Undecided)` because its payload is `Undecided`. The twelve were taken off the arm rather than
  off a description of it, which is the only reason that row was not swept in. Its sentence is
  written and its badge is the day somebody settles the payload.
- **The fifth cell is no longer only the command line's**, and `operations.html`'s own head comment
  says so. Nothing reads that cell — no test, and none of the four instrumentation commands, which
  ask for a column by name.
- **`Silent::Surface`'s doc gains the reason and says it is not the reason.** The arm answers what a
  replay reconstructs; a badge is a claim about a surface. They agree on the fact and are held apart
  on purpose, so that settling one is never mistaken for settling the other.
- **This is a decision and not a principle.** Three plausible alternatives lost, and what it decides
  is one column's answer on twelve rows of one page plus two badges. Under ADR-0249's gate it names
  no question outside the manual that reading it would answer.

## What this does not decide

- **Whether a model ever gets a window.** If one day a model is handed a view of the console — a
  screenshot, a description of the arrangement — the sentence above stops being true and these
  twelve are reopened. Nothing here forecloses that; it says what is true while the model cannot see.
- **`SetFavourite`'s MCP badge**, which is ADR-0301's and reads `plan` today. That record refused a
  model's star for a different reason — no sandbox to land in — and reconciling the two is its own
  question.
- **Whether a panel `a` is ever bound as a fit.** `karakuri-cli`'s shape is available; nobody has
  asked for it on the instrument, and the badge is `gap` until somebody does.
- **When `esc` stops quitting.** ADR-0259 decided it; this record only makes the page say it is
  coming.
- **Whether the window's close ever writes a `Record`.** `Operation::Quit` is `Silent::NoRecord`,
  which that arm's doc calls a gap rather than a decision, and it is untouched here.
