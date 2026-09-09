---
id: 0311
title: A row menu loads a Set onto a named deck, and saves it through the system's own dialog
status: accepted
date: 2026-09-09
supersedes: [0267]
superseded_by: []
principles: [0083, 0085, 0090, 0094, 0096]
tags: [console, ui, operations, library, platform, manual, m5]
---

# A row menu loads a Set onto a named deck, and saves it through the system's own dialog

## Context

*Send a Set to somebody, and take one in* has been the Library bay's one `plan` panel badge since
the vocabulary landed, and it is what `docs/roadmap.md`'s M5.3 exit condition rests on. Two records
had already taken the two questions under it apart.

[ADR-0260](0260-sending-a-set-is-a-read-and-a-reads-answer-goes-where-the-surface-that-asked-puts-answers.md)
settled what the operation asks for: **sending is a read**, its answer is the package, and a read's
answer goes where the surface that asked puts answers. `SetTransfer::Send { id }` names no
destination and none is owed. That is untouched here and is what makes this record possible.

[ADR-0267](0267-the-panel-sends-into-the-folder-the-library-bay-is-pointed-at-and-the-destination-is-drawn-before-the-press.md)
then answered *what the surface asks with*: the bay is already a file browser, so the directory the
`.path` row names is the destination and a send writes `<id>.kbset` there. It took neither of the
two candidates ADR-0260 had set out — a save sheet and the clipboard — and it closed both with
reasons of their own.

**What neither record decided was the gesture.** `docs/roadmap.md`'s M5.3 carried it as the whole
of the blocker — *"`Send a Set to somebody` waits on the gesture, and nothing else"* — with the two
shapes on the table named there: a third pill in the bay's foot beside `read` and the `load`
button, or a drag from a row onto the `.path` row. `view::LibraryBay` said the same from the code's
side: *"what is missing is a gesture, and it is the page's."*

**The maintainer answered on 2026-09-09, and the answer took neither of them.** Verbatim:

> Set名で右クリックでコンテキストメニュー、Load to Slot A / Load to Slot B / Load to Slot C /
> Load to Slot D / 分離バー / Save as a kbset、で最後のを選んだらシステムダイアログでファイル名指定
> して保存、かな。

*A right-click on a Set's name opens a context menu — `Load to Slot A` / `Load to Slot B` /
`Load to Slot C` / `Load to Slot D` / a separator / `Save as a kbset` — and picking the last one
opens the system dialog to name a file and save it there.*

Two things arrive in that sentence and only one of them was the question. The gesture is a menu on
a row, which answers what M5.3 was blocked on. **The destination is the system's save dialog**,
which is ADR-0260's alternative (a) and ADR-0267's alternative (a), taken after both had turned it
down — so the record that writes it down owes the argument those two made, rather than a fresh
comparison that ignores them.

## Decision

**A secondary press on a Set's name in the Library bay's list puts a menu down on that row. It
carries `Load to Slot A` … `Load to Slot D`, a separator, and `Save as a kbset`. A load lands that
row's Set on the deck the item names. `Save as a kbset` opens the system's own save dialog, and the
file the operator names there is where the package is written.**

- **The menu's operand is the row it was opened on**, and never the cursor. That is what makes it a
  route worth having: the `load` button reads the cursor and the pulldown, the key reads the cursor
  and the selection, the drag names both operands in one gesture across the window, and this names
  both operands in one gesture without moving the pointer off the row. **A pick moves no mark** —
  not the deck selection, not the pulldown's target, not the list cursor.
- **A load emits the same `Operation::LoadSet { deck, set }` the button emits**, and arrives where
  all four routes arrive: the slot's source is re-pointed and the watcher builds it
  ([ADR-0228](0228-a-library-load-re-points-the-slots-source-and-never-installs-a-set.md)). No new
  variant, no new row on [every operation](../manual/operations.html) for the menu itself — the
  card is a control, and *which item a press names* is the console's own affordance under
  [P-0090](../principles/0090-a-surface-offers-it-never-decides.md).
- **A deck the mixer draws no strip for is not offered.** That is `View::select`'s refusal read a
  third time rather than a third rule — the strips are what all three count — and it is why the
  card on a three-slot deck carries three loads.
- **`Save as a kbset` emits `Operation::TransferSet { transfer: SetTransfer::Send { id } }`, with
  no destination in the payload.** ADR-0260 stands whole: the dialog is not a field on the
  operation, it is *the surface's own way of putting a read's answer somewhere*, which is that
  record's sentence performed rather than amended.
- **What is written is `setfile::bundle`'s output** — the Set file with every source it names
  inlined after it, `docs/ir-spec.md`'s bundled form and
  [ADR-0231](0231-a-sets-two-forms-take-two-extensions-and-the-store-holds-only-the-resolved-one.md)'s
  `.kbset`. One missing artifact refuses the whole thing and names the node, which is that
  function's own refusal and is not repeated here.
- **The dialog opens on `<id>.kbset` in the folder the bay is pointed at**, where a folder has been
  dropped on the window ([ADR-0275](0275-a-folder-is-chosen-by-dropping-one-on-the-window-and-the-drop-is-the-windows-rather-than-a-bays.md)),
  and in the platform's own default where none has. `<id>.kbset` is the store's own naming rule, so
  nothing is invented ([P-0096](../principles/0096-the-operators-library-is-written-by-an-operators-own-act.md)).
- **A file already there is the dialog's question and never this program's.** Overwriting is what a
  save dialog exists to ask, and asking again on this side would be two programs asking one
  question with the operator answering the wrong one first. ADR-0267 left *whether a send refuses
  an existing file* open; this is the answer, and it is that the question is not ours.
- **A dismissed dialog writes nothing and says so.** Nothing is read, no bundle is built, no path
  is touched, and the panel prints the third of three sentences rather than going quiet — rule 04
  of [the manual](../manual/index.html).

### What this supersedes in ADR-0267

**ADR-0267's decision is superseded whole**, because the whole of it was the destination: *"The
panel's control for Send a Set to somebody is the Library bay's `folder` scope. The bay is already
a file browser; the directory the operator has the bay pointed at is the destination, and sending
writes `<id>.kbset` there."* The destination is now the file the operator names in the dialog.

**What survives, and is not this record's to take:**

- **The `.path` row stays.** It is the folder scope's own readout and it is what `folder` lists —
  ADR-0275's, not ADR-0267's. What it is no longer is *the place a send lands*, and the tip that
  said so has been rewritten. It keeps a part in the send all the same: it is where the dialog
  opens.
- **ADR-0267's reading of ADR-0260 stands.** Sending is a read, the operation names no destination,
  and `SetTransfer::Send { id }` gains no field. That was ADR-0260's and ADR-0267 only quoted it.
- **The page edit ADR-0267 said was owed is cancelled rather than done.** That record named
  `console.html`'s *"a folder is a way **in**, and **all** is where things are"* as the sentence
  that had to turn round, because it had made the folder scope bidirectional. It is not
  bidirectional any more, so the sentence stays exactly as it is. A record's consequence discharged
  by being withdrawn is worth saying out loud, because the next reader of ADR-0267 will go looking
  for an edit that never landed and should find this instead of a gap.

## P-0094, and the argument ADR-0275 made against a dialog

[P-0094](../principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)
lists *a window that opens and cannot be touched* among the things it rules out. ADR-0260 stated
the save sheet's problem and stated the counter-reading fairly — that the rule is about mechanisms
that *run during* a performance and may not reach a control nobody presses during one — and closed
with *"Nobody has argued that out."* ADR-0267 then chose the folder scope partly so that nobody
would have to: *"this control opens no window, and the question ... stays unasked rather than
answered badly."* ADR-0275 turned a folder dialog down in the same words.

**That deferral is spent, because the maintainer has chosen the dialog.** So the argument is made
here, and the half of it that can be measured is measured rather than asserted.

### What ADR-0275's objection actually was, and why it does not reach this

ADR-0275 rejected a folder dialog as *"a dependency and a modal window over a live instrument to
reach a place the drop already names."* **The load-bearing clause is the last one.** A folder
dialog would have asked the operator to name a directory that was already on their desktop under
their hand, and a drop names it for nothing — so the modal bought nothing at all, and the P-0094
question never had to be reached.

**A send is the case where that clause is false.** `view::LibraryBay` has said so since before
either record: *"the asymmetry is that taking in names a file that exists and sending names one
that does not yet — and no listing can point at a file nobody has written."* A file that does not
exist is the one thing no listing, no drop and no row on this panel can name. ADR-0267's answer was
to remove the need for a name — the *scope* names the directory and `<id>.kbset` names the file —
and that works only while the operator wants that directory and that name, which is a send to
somebody who is on this machine.

**So the two dialogs are not the same trade.** ADR-0275's would have replaced a gesture that
already answers the question. This one answers a question nothing else on this panel can put.

### The measurement: does the frame loop keep drawing?

**P-0094's admissible answers are three, and the one this has to meet is that the show does not
stop and does not go quiet.** A modal that stops the frame loop is not admissible under any
reading: the panel's continuous motion is *the cheapest thing the console has to say this is live,
and the only statement still working when whatever would otherwise report the fault is itself what
stopped*.

**It is a fact about `rfd`'s macOS backend, so it was read and then run.**

*Read*, in `rfd-0.17.2/src/backend/macos/`:

1. `file_dialog.rs`'s `AsyncFileSaveDialogImpl::save_file_async` builds a `ModalFuture` and returns
   it boxed. The panel is begun when `save_file` is *called*, not when the future is first polled.
2. `modal_future.rs`'s `ModalFuture::new` takes the sheet path when `NSApplication::isRunning` and
   a window exists — which is a `winit` 0.30 program with a window open — and calls
   `inner.begin_modal(&window, &completion)` on the main thread.
3. `file_dialog/panel_ffi.rs`'s `impl InnerModal for NSSavePanel` is
   `beginSheetModalForWindow_completionHandler`. **That is an asynchronous document-modal sheet: it
   returns at once and the application's run loop is untouched.** The synchronous
   `FileSaveDialogImpl::save_file` beside it is `runModal`, a nested run loop, and is the thing
   that must not be used.
4. `ModalFuture::new`'s other branch prints *"running async dialog in unsupported environment, I
   will fallback to sync dialog"* and calls `run_modal`. Its absence on a run is evidence the sheet
   path was taken.

*Run*, on this machine (macOS 25.5, `winit` 0.30.13, `rfd` 0.17.2, release build) with a probe that
drives the loop the way `karakuri` drives it — a `ControlFlow::WaitUntil` deadline re-armed every
pass, which is what a declaring region does — and counts `RedrawRequested`:

```
panel asked for at 5.817823666s
frames total 19661, frames while the panel was up 19660, future settled 0
```

**19660 frames were drawn in the three seconds the sheet was up**, the future was still pending
when the loop exited — so those frames were drawn with the panel genuinely open and unanswered —
and `rfd`'s fallback line never appeared. The loop does not stop.

**This machine is evidence about this machine**
([ADR-0110](0110-this-machine-is-not-the-reference.md)), and the number is not the claim. The claim
is the shape: `beginSheetModalForWindow:` versus `runModal`, one of which nests a run loop and one
of which does not. The count is what makes the shape checkable, and the probe is a dozen lines
anyone can run again.

### What it costs, said plainly rather than argued away

**While the sheet is up, presses aimed at this window do not reach the panel.** That is what a
document-modal sheet is, and no reading of the measurement above makes it otherwise. It is the
price, and what bounds it is written down rather than assumed:

- **The operator opened it.** Nothing in this program opens a dialog on its own, on a timer, or in
  response to anything but a press on one menu item. P-0094's *the price is never the operator's*
  is about safety bought with authority; this is a window an operator asked for.
- **One key closes it**, and closing it writes nothing.
- **No deck is touched either way.** A send is a read: `karakuri-operation-record` answers
  `Silent(NoRecord)` for the whole variant, nothing is compiled, no device is opened, and the mix
  is exactly what it was. There is nothing half-done behind the sheet and nothing to roll back.
- **The picture goes on.** Measured above: the frame loop draws, the beat travels, and the panel
  goes on saying it is live.

**What is left is a control an operator should not press mid-transition**, which is true of `k` and
of half this panel. The rule this had to clear is that the show does not stop; it does not.

## Alternatives rejected

### a. A send pill in the bay's foot, writing into the `.path` folder — ADR-0267's own shape

The status quo ante, with the gesture filled in: a third capsule beside `read` and `load`, reading
its destination off the `.path` row the way the load's pulldown reads its deck. **It is what M5.3
named first and what ADR-0267's decision implies.**

**What it has going for it, kept honest.** It needs no dependency and opens no window, so the
argument above is never needed. It closes the loop `view.rs` asked for — the file lands where the
listing is looking, and a row of that listing is the taking-in half — and it says where it lands
before the press, which is what P-0090 asks of a control.

**It loses on what the operator asked for and on what it cannot do.** The maintainer chose the
dialog, which is the first half. The second is that it can only ever send into a folder somebody
has already dropped on this window, under a name the store picked: a send to a person is a file
going somewhere the operator chooses at the moment they choose it, and this shape makes that two
gestures — point the bay, then press — with a rename afterwards in another program if the name was
wrong. **And it has no answer for the collision it creates.** ADR-0267 left *whether a send refuses
an existing file* open, and every answer costs something: refusing sends the operator away to
rename a file this program will not name, and overwriting writes over somebody's file with no
question. The dialog has that answer built in and it is the platform's.

### b. A drag from a row onto the `.path` row

The other shape M5.3 named. **It has the drag idiom already built** — a row is picked up, carried
and let go, and `Panel::carry` and the drop ring are in the tree
([ADR-0273](0273-the-carry-lands-on-two-sets-of-rectangles-and-wears-a-face.md)) — so a second
target for an existing gesture is the cheapest thing on this list.

**It loses three ways.** It is (a)'s destination with a longer gesture, so every objection above
survives intact. It makes one carry mean two operations told apart by which rectangle it was
released over, where today a release names a *deck* and nothing else — and the mark that says
*where* would then have to say *what*, which the drop ring is written not to do
([ADR-0265](0265-a-carried-set-names-its-deck-at-the-release-and-the-panel-refuses-no-drop.md): it
says where the release lands and never whether it is allowed). And it is a route the keyboard
cannot take, aimed at a row the keyboard cannot reach, for an operation whose `key` badge is `gap`
— which is rule 01 pushed further away rather than nearer.

### c. The dialog, run synchronously

`rfd::FileDialog::save_file` rather than `rfd::AsyncFileDialog::save_file`. **It is much simpler**:
one call, a `Option<PathBuf>` back, no future, no worker for the waiting half, and the press
handler could bundle and write before it returned.

**It is the one thing P-0094 forbids outright.** `panel_ffi.rs`'s synchronous path is
`NSSavePanel::runModal`, which spins a nested run loop in `NSModalPanelRunLoopMode`: the frame loop
does not run, `winit`'s `WaitUntil` timer does not fire, and the panel freezes at whatever it last
drew for as long as the operator is choosing a file. A panel that *cannot meet a staleness it
declared looking calm* is in P-0094's own list of what it rules out, and this is that with the
clock stopped. **The measurement above exists to tell these two apart**, and the difference between
them is one type name.

### d. The clipboard

ADR-0260's second candidate, closed by ADR-0267 and not reopened by the maintainer's sentence.
Recorded here only because a reader arriving at this record from ADR-0260 should not have to guess:
it loses for ADR-0267's reason, which has not weakened — `SetTransfer::Take` names a **file**, so a
send onto the clipboard is a read whose answer no route in this program can receive.

### e. A menu item for the send and the four loads somewhere else

The loads are the part of the maintainer's sentence that was not asked for: the `load` button
already reaches `LoadSet`, and a fourth route to one operation is a cost. **It loses because the
menu is one card and the loads are what make it worth opening.** A menu with one item on it is a
button drawn twice, and the row's two acts — put it on a deck, hand it to somebody — are what an
operator has a row for. It is also
[ADR-0259](0259-the-keyboard-is-addressed-to-the-bay-that-has-focus-and-a-global-letter-is-a-convenience-or-the-operators-own.md)'s
own prediction arriving: *"where an item has two acts they are two controls"*, and this is where
the second one landed.

## Consequences

- **`docs/manual/operations.html` moved first, under `docs/contributing.md` §5 step 2.** *Send a
  Set to somebody, and take one in* reads `panel has library row menu → save as a kbset` where it
  read `panel plan library`, and its tip is rewritten from *"what nothing draws is the sending"* to
  what the gesture is, what it writes, where the dialog opens, and that an existing file is the
  dialog's question. *Load material into a deck* reads
  `panel has load button, row menu → load to slot, or library → deck`, and its tip names the menu
  as the one route that reads neither mark.
- **`docs/manual/console.html` is step 3.** The mock draws the menu open on the `night01` row, in
  the pulldown card's own treatment; each of the six items carries a `data-tip` and the separator
  carries none, because it is not a control. *How a Set reaches a deck* gains a paragraph — four
  ways in now, and the sentence about the count moved with it — and the `.path` row's tip no longer
  says a send lands there.
- **`docs/manual/style.css` gains `.lib-row.menued`, `.rowmenu`, `.rowmenu .item` and
  `.rowmenu .rule`**, and the card is the deck pulldown's treatment because it is the same object
  one control along.
- **`crates/karakuri-console/src/view.rs` carries `Menued`, `RowItem`, `Picked`, `RowMenu`,
  `load_item`, `MENU_LOAD`, `MENU_SAVE`, `LibraryBay::menu`, `LibraryBay::menu_ask` and
  `row_menu_into`** — `Load`, `Target` and `Aim`'s shapes one control along, so a reader who has
  read the pulldown has read this. `View` gained one private field, `menu_row`, with `menued`,
  `menu_open`, `open_menu` and `shut_menu` the only ways in; it is one field where the pulldown
  takes two, because a menu is a card and nothing else.
- **`View::open_menu` does not refuse a console with no strip**, where `View::open_target` does,
  and the difference is the send: a card with no loads in it still has something to pick and
  something to leave by.
- **`crates/karakuri-console/src/room.rs` gained four constants** — `ROW_MENU_RULE_PAD`,
  `ROW_MENU_RULE_H`, `ROW_MENU_MIN_W` and `ROW_MENU_INSET` — each citing the `.rowmenu` rule it was
  copied from, which `tests/transcribed_constants_cite_the_mock.rs` resolves.
- **`input::claim`'s rule 2 has a fourth card in it**, and `input::wheeled` the same four. The
  module documentation no longer says *two cards*.
- **`input::PROBES`' row for the Library bay's list claims two rather than one**, so
  `input::CONTROLS` rose by one and the legend `crates/karakuri/src/main.rs` prints rose with it.
  One rectangle, two controls, told apart by the button — which is a different pair from the two
  meanings that row already had under two scopes, and that pair is still one control.
- **`crates/karakuri/src/main.rs` gained `Pointer::Secondary` and a `MouseButton::Right` arm.**
  Before it, every button but the left one fell through that handler's `_ => {}` and reached
  nothing at all — `egui` was never told about one either, and still is not, because `egui` owns no
  widget anywhere on this console. There is no secondary *release*: nothing is taken in hand on a
  secondary press.
- **`karakuri-console` still does not know which button a press was**, and that is deliberate:
  `claim`'s rules are all about where the pointer is, and a secondary press on a control the
  console draws is the console's for the reason a primary one is. What differs is only what the
  press asks for, which is the host's half of the seam.
- **`Readout::menued` is `Readout::aimed`'s shape**, and `press_handler::ASKED`'s row for the
  Library bay's list names `bay.menu_ask(` beside `bay.take(` and `bay.land(`.
- **The card is asked before every other control in the press handler**, ahead of the two pills in
  the transport row and ahead of the pulldown in this bay's own foot. That ordering is the rule
  rather than a convenience and it was got wrong first: asked in the order the card was *added* —
  after the pulldown — a press on the `load` button with a menu down opened the pulldown instead of
  dismissing the menu, which is two cards down at once and the one thing rule 2 exists to make
  impossible. `a_secondary_press_opens_a_rows_menu_and_a_primary_press_does_not` presses the `load`
  button with the menu down and asserts both marks, and it was watched to fail against the ordering
  it replaced.
- **`crates/karakuri/src/main.rs` gained `sending`, `sent`, `bundled`, `Sent` and
  `Keeping::sends`/`send_tx`/`took_send`.** The dialog is asked for on the main thread and awaited
  on a worker; the bundle and the write are on that worker, because a `.kbset` with every source
  inlined is a store read and a file written and neither is a thing to do on a frame (P-0091). The
  outcome comes back on a channel drained in `Keeping::finished_saves`, which is where a keep's is
  said.
- **The run does not wait for a send at exit**, where it waits for saves under a bound. A save is
  bounded by a disk; a send is bounded by a hand that has not answered a dialog, and a quit that
  blocked on one would be a program refusing to close because it had opened a window over itself.
- **`crates/karakuri/Cargo.toml` gains `rfd = { version = "0.17.2", default-features = false }`,
  and `Cargo.lock` gains exactly one package.** Every macOS dependency `rfd` names — `objc2`,
  `objc2-app-kit`, `objc2-foundation`, `objc2-core-foundation`, `block2`, `dispatch2` — was already
  resolved for `winit` at the same version, so no second copy of any of them entered the tree.
  P-0085's bill, paid now, and it came to one crate.
- **It is `crates/karakuri`'s manifest and not the console's**, which is ADR-0156 and ADR-0214's
  seam kept: `karakuri-console` takes no device and no platform dependency, and there is nothing in
  that manifest to reach for.
- **`karakuri-console/tests/panel_column.rs` gained a `TransferSet` arm**, which is the test §5
  names doing its job: the console started constructing an operation that file had no value for,
  and the suite went red with a message naming the gap before anything else did.
- **Nothing in the vocabulary changed.** `Operation::TransferSet` and `SetTransfer` are what they
  were, `gate.rs`'s `Standing::Open` stands for ADR-0260's reason, and
  `karakuri-operation-record` still answers `Silent(NoRecord)`. The title strings are untouched, so
  `the_manual_and_the_vocabulary_agree` holds.
- **M5.3's one `plan` panel badge is gone**, which was that sub-milestone's whole exit condition.
  The roadmap carries the pointer beside the item.
- **The `key` and `MCP` columns on that row are unchanged and still owed.** The keyboard reaches
  neither the menu nor the send, exactly as it reaches neither the load button nor the pulldown,
  and that is M5.13's with theirs (ADR-0305's *what is owed is a key route*). The MCP badge is
  `plan` and this record does not move it: ADR-0260 says what that tool's shape would be.
- **This is a decision and not a principle.** Plausible alternatives lost — (a) is a decision
  already taken in this repository, which is as plausible as an alternative gets — and the rule
  decides one control in one bay. Under
  [ADR-0249](0249-a-principle-is-what-decides-a-question-it-does-not-mention.md)'s gate there is no
  question outside this bay that reading it settles; the general rule about a window over a live
  instrument is P-0094's and is not restated as a new file.

## What this does not decide

- **What the keyboard's route to the menu is.** M5.13's, with the load button's and the pulldown's.
- **Whether `Take` takes text over MCP.** Unchanged from ADR-0260.
- **What a send does on a platform that is not macOS.** The measurement above is read out of
  `rfd`'s macOS backend and run on macOS. `rfd` has backends for the others and this decision is
  written in terms of *the asynchronous dialog* rather than of the sheet, but nobody has read the
  Windows or the XDG portal path and this record does not claim they behave the same. The thing to
  check there is the same thing: does the frame loop keep drawing.
- **Whether a secondary press reaches anything else on this panel.** It reaches one control today.
  Whether a strip, a pane head or a preview cell should have a menu is each bay's, and this record
  binds none of them.
