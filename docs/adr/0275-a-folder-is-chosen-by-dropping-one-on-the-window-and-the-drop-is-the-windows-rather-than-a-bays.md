---
id: 0275
title: A folder is chosen by dropping one on the window, and the drop is the window's rather than a bay's
status: accepted
date: 2026-09-07
supersedes: []
superseded_by: []
principles: [0083, 0085, 0090, 0091]
tags: [ui, console, library, surfaces, platform, docs]
---

# A folder is chosen by dropping one on the window, and the drop is the window's rather than a bay's

## Context

The Library bay draws four scope chips and one of them cannot be asked anything. `Scope::Folder` is
*"A directory somebody names during the run"* (`crates/karakuri-console/src/view.rs`), the host
answers it with `Vec::new()`, and the panel says so out loud at the chip. `docs/roadmap.md`'s M5.3
is blocked on it, and [ADR-0267](0267-the-panel-sends-into-the-folder-the-library-bay-is-pointed-at-and-the-destination-is-drawn-before-the-press.md)
hung a second thing off the same dependency by making that scope the destination a send writes into.

**What it waits on is not an operation, and commit `2be79b5` is where that was established.**
`Operation::ListSets { holds, layer }` is correctly shaped: both fields are filters over what a
store already holds, and a directory is *which store is asked at all*, which is already the host's —
`listing()` picks the store or the presets root outside the operation entirely. The rule that
decides it is the one the vocabulary's only path already obeys: a path may sit in a payload only
where every route that fills it derives it from something the program itself produced.
`SetTransfer::Take { file }` obeys; a directory on `ListSets` would be spelled by a surface, and
`crates/karakuri-environment/src/mcp.rs`'s *"Paths never cross the protocol"* is the same wall from
the other side.

So the chip waits on **a host mechanism for choosing a directory, and a sentence on the page**. This
record is the mechanism. The page moved first, under `docs/contributing.md` §5 step 3.

## Decision

**A folder is chosen by dragging it off the desktop and letting go of it anywhere over this
program's window. There is no dialog, nothing is typed, and no flag names one before the run.**

It costs no dependency. `egui-winit` already turns the platform's drop into
`egui::RawInput::dropped_files` and its hover into `hovered_files`, and **nothing in this workspace
reads either field** — checked across `crates/`, no hit. So this is P-0085's mechanism that already
exists, taken rather than designed around.

### What the platform actually delivers, measured rather than assumed

Read out of `winit-0.30.13/src/platform_impl/macos/window_delegate.rs` and
`egui-winit-0.36.1/src/lib.rs`, because the shape of the page depends on all six.

1. **A directory does arrive.** `draggingEntered:` and `performDragOperation:` read
   `NSFilenamesPboardType` off the dragging pasteboard and emit one `WindowEvent::HoveredFile` /
   `WindowEvent::DroppedFile` per entry. There is no `isDirectory` test, no extension test and no
   filtering of any kind, and `egui-winit` passes both through unmodified.
2. **Neither the hover nor the drop carries a pointer position.** The `NSDraggingDestination` impl
   is `draggingEntered:`, `prepareForDragOperation:`, `performDragOperation:`,
   `concludeDragOperation:` and `draggingExited:` — **there is no `draggingUpdated:`**, and AppKit
   delivers no mouse-moved event during an inter-application drag. So a folder drop is
   **window-global and not bay-targeted**: the panel cannot know it landed on the Library bay rather
   than on the mixer.
3. **The hover does carry the path**, and `RawInput::take` *clones* `hovered_files`, so it is
   readable on every frame the drag is over the window. The page may specify something drawn before
   the release, and it can name the folder — but not a place.
4. **A multi-item drop arrives as N entries in one frame**, not N frames: `performDragOperation:`
   iterates the whole pasteboard array and queues every event before the next redraw.
5. **A file and a directory are indistinguishable at the event.** The panel must decide.
6. **`dropped_files` is visible for exactly one pass and then gone.** `RawInput::take` *moves* it,
   where it clones `hovered_files`.

### The three policies the plumbing does not answer

They are on `docs/manual/console.html` under *How a folder is chosen, and why the drop is the
window's*, and they are here because they are decisions rather than drawings.

**A path that is not a directory is refused, naming what was dropped.** (5) says the panel must ask
the file system which it is, once, on the drop — the way `listing()` is asked on a press and never
on a frame (P-0091). A file is *not* read as the folder it sits in: that would point the bay at a
directory nobody pointed at, which is the mistake
[ADR-0273](0273-the-carry-lands-on-two-sets-of-rectangles-and-wears-a-face.md) refuses one bay over
when it declines to snap a drop mark to the nearest strip. Nor is a dropped `.kbset` taken in where
it fell: taking a Set in is a press on a row of a listing, and (2) means a file landing on this
window has no row under it. The refusal names the file and says a folder is what this takes, which
is P-0083.

**More than one path is refused, and all of them are.** (4) means three folders let go together are
one pass with three entries, so there is no first to act on and a rest to ignore. (2) means nothing
says which was aimed at, and the order is the desktop's rather than the operator's — so taking the
first would be choosing out of a list nobody built on purpose. The bay keeps the directory it had
and the refusal counts what arrived. **One path, and it has to be a directory.**

**The drop is recorded on the frame it is seen on.** (6) leaves no second chance, so the release
does the whole thing rather than asking a question: it sets the directory *and* marks the `folder`
chip, and the listing under it is the next thing drawn. A bay that had put the path aside and waited
for the chip to be pressed would be waiting on an operator who has already made the gesture, holding
a path nothing will hand it again.

### What is drawn, and what cannot be

**The `.path` row is the mark.** `docs/manual/console.html`'s Library bay has drawn that row since
the mock was written and it was the only element in the bay with no `data-tip`; it now has one. It
reads the directory the bay is pointed at, it is drawn whichever scope is marked — because ADR-0267
makes it the place a send lands, and P-0090's *a control says where it lands before the press* wants
that on screen — and it is not drawn at all until a folder has been chosen, which is the staging
lane's own answer to an empty lane.

**While a folder is over the window that row reads the path a release would set**, in `--c-text`
rather than its faint (`.path.incoming` in `docs/manual/style.css`). That is the whole of the mark,
and it is the only mark this gesture can afford. **The drop-mark idiom ADR-0273 established does not
transfer**, and the page says so plainly rather than leaving a reader to look for a ring: that mark
says *where*, and by (2) there is no where. The ink is the same and for the same reason — the text
ink is what a word is drawn in when nothing is being said about it, and nothing is being said here
about whether the release will be allowed.

### It is not an operation, and no badge moves

`docs/manual/operations.html` gains no row. The release asks *Choose which scope the library shows*
for `folder`, which is that page's row and whose panel badge is already `has`; the directory it sets
is the bay's own state in the way the list cursor is. **The test is the other three columns and none
of them could ever be filled**: a map line names a slot, a range or a word from a closed list; a
model is handed no path in either direction (`mcp.rs`); and a key cannot spell one, because the
console's one letter-taking flow is `Menu::Naming`, bounded by ADR-0221 to one path component of
letters, digits, `-` and `_`, and ADR-0229 says why that rule does not stretch to separators. A row
would be rule 01 written down as three permanently empty columns — the argument the library cursor
is already refused a row by, under *How a Set reaches a deck*.

## The risk this carries, and it is the feature's rather than this decision's

**`winit` reads only `NSFilenamesPboardType`, which Apple deprecated in favour of
`NSPasteboardTypeFileURL`.** Both call sites in `window_delegate.rs` return early when
`propertyListForType` answers `None`, so if a future macOS stops populating that type, `HoveredFile`
and `DroppedFile` stop being emitted at all — **files and folders alike, and every drop in this
program with them**. There is nothing this workspace can check for at runtime and nothing to fall
back to: the symptom is a drag that does nothing and no event to say so.

It is written here so that whoever is debugging a dead drop finds it by grepping the name. The
answer, if it happens, is a `winit` that reads the modern type or the dialog and its dependency, and
it is not this record's decision to pre-empt.

## Alternatives rejected

### a. A folder dialog — ask the platform for a place

**It buys a dependency and a modal window over a live instrument to reach a place the drop already
names.** `Cargo.lock` carries no `rfd`, so this is a new direct dependency for one control.
[P-0094](../principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)'s
*"a window that opens and cannot be touched"* is in that rule's own list of what it rules out, and
ADR-0267 already declined to argue out whether a rule about mechanisms that run *during* a
performance reaches a control nobody presses during one. Taking the drop means that argument is
never needed.

**What it has going for it, kept honest**: it is targeted — the operator is asking this program for
a directory rather than throwing one at its window — so none of the three policies above would exist,
and the deprecation risk above would not be this program's. It is the named fallback if the drop
stops arriving, which is the whole of what that concession costs.

### b. `--folder DIR`, told at launch

**It makes `folder` a second `presets`.** The `presets` scope is a root the program is *told* before
the run (ADR-0230) and this would be the same thing under a different chip: one directory per
invocation, chosen before the window opens, and re-pointing the bay would mean restarting the
instrument. `console.html`'s own reading of the scope list is that *a folder is a way in* — a place
you point at while you are working — and a flag makes it a second fixed root instead. It also
answers nothing the presets root does not already answer, so the honest form of this alternative is
*delete the chip*, which nobody proposed.

### c. A second letter-taking flow — type the path

**Refused in advance, twice.** The console has exactly one letter-taking flow, `Menu::Naming`, and
ADR-0221 bounds what it takes to one path component of letters, digits, `-` and `_`; ADR-0229 says
in as many words why that does not stretch — *"an include is a relative path and has separators in it
by construction, so the rule cannot be copied."* And this bay already answered the same question
against itself:
[ADR-0262](0262-a-library-filter-field-steps-through-what-the-store-already-holds-rather-than-taking-letters.md)
made `holds` and `layer` *step* rather than take letters precisely so as not to open a second flow,
and `view.rs` records that a second flow admitting separators would be *"this bay inventing a wall"*.
A directory is worse than the filter fields were: it cannot step, because there is no closed list to
step through.

### d. A field on `ListSets`

**Refused by the rule the vocabulary's one path already obeys**, which is commit `2be79b5`'s
argument and is not re-run here: a path may sit in a payload only where every route that fills it
derives it from something the program itself produced, and a directory typed or dropped by an
operator is spelled. `mcp.rs`'s *"Paths never cross the protocol"* is the same wall from the other
side — a client may be on another machine where a path means nothing, and a tool taking one would
invite a model to name anywhere on the render machine's disk. There is no per-surface exemption to
reach for either: `gate` is one classification and one sentence for every route.

**And it would be wrong even if it were allowed**, which is worth saying separately: `ListSets`'
two fields narrow *what a store holds*, and a directory is which store is asked. Putting it on the
payload would make one operation mean two things.

### e. Keep waiting

The status quo, and it has a real argument: the chip has been drawn and unanswerable for a while and
nothing broke. **It loses because two things now hang off the same dependency** — the chip and
ADR-0267's send — and because the wait was never on information that was going to arrive. What was
unknown is now measured, in six numbered facts above, and every one of them was readable the whole
time.

## Consequences

- **`docs/manual/console.html` carries a new note**, *How a folder is chosen, and why the drop is
  the window's*, and the `.path` row at last has a `data-tip`. The `folder` chip's tip gains the
  sentence saying which directory and how.
- **That page's *What is owed is the asking* paragraph was wrong and is rewritten.** It said *"no
  operation in the vocabulary can ask a folder for its listing"* and read that as a gap; `2be79b5`
  had already established it is the shape. The paragraph now says what the chip was actually waiting
  on and points at the new note.
- **`docs/manual/operations.html` gains no row and moves no badge.** *List what the store holds*'s
  tip said *"It has nowhere to put a directory, which is what the library's folder scope waits on"*
  — the third of the three sites `2be79b5` named, and the only one on a page rather than in source.
  It is corrected. *Choose which scope the library shows* gains the drop and the reason there is no
  row for picking a directory.
- **`docs/manual/style.css` gains `.path.incoming`**, and the comment above `.path` carries why the
  mock does not wear it: the panel is already drawing a Set in hand, and a carry and a folder coming
  in from the desktop are not two drags that can run at once.
- **Two source sites still read `ListSets` as missing a field**, and they are not this record's to
  edit: `view::Scope`'s doc bullet in `crates/karakuri-console/src/view.rs` and `why_nothing`'s
  `Folder` arm in `crates/karakuri/src/main.rs`. `2be79b5` named all three as owed; this record
  discharges the page's one. **Until the other two land, the panel says at the chip that the asking
  is owed by the operations page, and the operations page says it is not** — which is a live
  disagreement rather than a stale comment, and it is the first thing to fix in `crates/`.
- **`view::LibraryBay`'s list of *what is in the mock's bay and is deliberately not here* is now
  wrong about the `.path` row.** It reads *"It is the walk inside a folder scope, so it says nothing
  until that scope can be asked for a listing at all"*, and the page has since made that row the
  send's destination readout as well, drawn whichever scope is marked.
- **M5.3's blocker changes shape rather than clearing.** The mechanism is decided and the page is
  written; what is left is `crates/` — reading `dropped_files` and `hovered_files`, the directory in
  the host beside `listing()`, the folder listing itself, and the `.path` row in
  `karakuri-console`. Nothing in the vocabulary changes and no test named in
  `docs/contributing.md` §5 has a new subject.
- **This is a decision and not a principle.** It settles one control in one bay on one platform's
  measured behaviour; under
  [ADR-0249](0249-a-principle-is-what-decides-a-question-it-does-not-mention.md)'s gate there is no
  question outside this bay that reading it settles.

## What this does not decide

- **What the walk inside a folder is.** Whether the `›` in the `.path` row can be pushed past a
  subdirectory, and whether the bay can leave the directory it was given, are still the console
  page's and are still unwritten. This record decides only how the directory arrives.
- **Whether a send refuses an existing file, and in what words.** ADR-0267 left it open and it is
  still open.
- **What happens on a platform that is not macOS.** The six facts above are read out of the macOS
  backend. `winit`'s other backends emit the same two events and this decision is written in terms
  of the events rather than the backend, but nobody has read X11's or Wayland's source and this
  record does not claim they behave the same.
- **Whether the drop is refused while some other gesture is running.** A carry in hand and a folder
  arriving from the desktop cannot both be happening, so the page draws one at a time and the
  question of what the panel does if the platform disagrees is nobody's yet.
