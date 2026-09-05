---
id: 0237
title: A deck slot runs from its own copy, and one preset in four slots is four files
status: accepted
date: 2026-09-01
supersedes: []
superseded_by: []
principles: [0044, 0096, 0085, 0093, 0094]
tags: [store, console, mcp, hot-swap, docs]
---

# A deck slot runs from its own copy, and one preset in four slots is four files

## Context

[ADR-0088](0088-what-ships-what-you-saved-and-what-you-are-editing.md) created the scratch and
separated the three places material can live in. One clause of it has now lost:

> **Two slots naming one file share one scratch file**, deliberately: copying per slot would silently
> break the sharing `watch.rs` says it supports, and would make MCP's report that *this write also
> reached slot N* untrue.

**The opposite rule was already written, two days earlier, in the same module.**
[ADR-0228](0228-a-library-load-re-points-the-slots-source-and-never-installs-a-set.md) gave
`scratch::place` its name and its argument:

> **The scratch name carries the deck and the node's place**, `A0-drift.kir`, and that is not
> decoration: `scratch::place` writes `<store>/scratch/<name>.kir` and overwrites what is there, so
> two decks loading Sets whose procedures happen to share a name would silently become one file — the
> second load moving the first deck on its watcher's next poll, with nothing to say why.

So `crates/karakuri-environment/src/scratch.rs` held two rules about one directory. `place` named a
file for the deck it belonged to; `materialise` named it for the source it was copied from and
deduplicated two slots onto one file on purpose. A run started with `--load-set --watch` used both at
once. Neither record was wrong about its own day, and the collision is exactly the shape
[P-0044](../principles/0044-doubt-a-conclusion-that-contradicts-a-position-already-taken.md) names —
*"When a judgement about this design collides with a stance the project has taken repeatedly, **the
thing to re-examine is the judgement**"* — with the stance being ADR-0228's, which nothing in
ADR-0088's clause had been checked against.

### The requirement is what settled it, and it was not new

The maintainer, on 2026-09-01: **a deck slot runs from a working copy prepared for that slot. Even
the same preset loaded into every slot is four separate copies in four places.** He noted that this
was a specification written to prevent exactly the failure that had just been observed, and that it
had been known and instructed from the start.

The observed failure is the panel's, and it is the whole of the cost of the clause: every one of
`crates/karakuri`'s four slots watched the two paths the operator typed, so **one save rebuilt four
slots**. Four candidates entered the Staging lane and three of them belonged to slots that are not
drawn — deck B is asked to prime and parked by the budget, C and D are neither — so three rows sat in
the lane for the rest of the run with no verdict coming. The comment now standing where those
watchers are built says it in the program's own words: it *"made one save rebuild four slots and fill
the Staging lane with three rows that can never leave (a parked slot's trial is frozen, so it reaches
no verdict)"*.

The deeper half is that four decks opened on one preset are four simulations an operator means to
drive apart, and a shared file means the first edit moves all four and no one of them can be moved
alone. `crate::watch`'s *"a slot is the unit that gets replaced — that is what it means for each slot
to own its own `HotSwap`"* is the sentence the shared file contradicted.

### The argument the clause lost on, because it is worth the record

The clause defended sharing by naming what sharing would cost: MCP's report that *this write also
reached slot N*. **That defence is circular.** The sentence it protects exists only to describe the
sharing. Ending the sharing does not make it untrue — it makes it **empty**, which is what it should
say.

That reading was checked against the scan rather than assumed. `crates/karakuri-environment/src/mcp.rs`
walks every node of every other slot looking for the path it just wrote, and it was **kept**, in the
form it had, for a reason written beside it:

> `Slots` is a list of paths this module is handed, the sentence is true of whatever it is handed,
> and a `Vec::new()` written here would be this module asserting something about its caller.

So the surface still asks the question and still answers it; the answer is now normally *nothing*, and
it is nothing **because of this rule** rather than because no operator happened to share. The scan's
own tests — a pair given to two slots, and a deformation given to two slots — hand it slots that
share and still get the sentence, which is what keeps it a scan rather than a constant.

## Decision

**A deck slot runs from a working copy prepared for that slot, and the same source given to four
slots becomes four files.**

### 1. `materialise` takes slots, not a flat list of paths

The signature is the decision. A flat list cannot spell a name that carries a slot, and the dedup it
used to do is gone:

> **Takes the slots rather than a flat list of paths**, which is the change this function exists to
> carry: the name of a copy is [`node_name`]'s, and that name is the slot and the node's place in it.
> A flat list could not spell one.

> **No dedup, and that is the rule rather than an omission.** The same source given to four slots
> becomes four files.

What it does *not* take is what a slot is: the caller hands over each slot's paths in node order and
keeps the node names beside them, because a node name is the Set file's business
(ADR-0228's *"the node name is the Set file's and not the procedure's"*) and not this module's.

### 2. `scratch::node_name` is the one naming rule for the directory

`A0-drift_shell.kir` — the deck letter, the node's place in that slot, and the material's own name.
It is ADR-0228's spelling made the whole directory's rather than the library load's, and the reason
is that the argument was never about loading:

> Nothing in that sentence is about a *load*: it is about two decks and one directory, which is every
> run.

This is
`docs/contributing.md` §4 applied to a directory
instead of a record tag — *"It is not enough to be disjoint in practice; they have to be disjoint by
name"* — and `<letter><index>` is disjoint by construction, so the `unique_name` counter that used to
scan for collisions is gone with the rule that needed it. The material's own name is kept beside the
prefix rather than replaced by it, for that function's own reason: *"a scratch of hashes is a scratch
nobody opens in an editor."* Both halves are asserted rather than assumed — the letters are checked
against `karakuri_console::view::DECK_LETTERS` over every slot a `Deck` has, in
`the_deck_letters_are_the_ones_the_preview_cells_carry`, because this crate cannot name that constant.

### 3. The panel makes its copies before the window opens

`crates/karakuri`'s `working_copies` clones the one pair `SLOTS` times and hands `materialise` an
array per slot, so each of the four watchers is built over the copy made for its own slot and **no
watcher can see another slot's file at all**. That is the ordering the requirement asks for: the
copies exist before there is a deck to rebuild, not as a repair after a save has already reached four
slots.

### 4. Both programs print the file each deck runs from

`karakuri-cli` prints the directory and then a line per slot — *"every slot runs from its own copy
here, so the files you named are not written to. Point an editor at these"*, then
`slot 0: A0-drift_shell.kir + A1-soft_points.kir`. The panel prints the same shape per deck letter,
and its own documentation says why one line is not enough there: four decks on one preset are four
files *"whose names an operator cannot guess and cannot tell apart by content, since at startup they
are identical; what makes a file deck B's is its **name**"*. This is
[P-0096](../principles/0096-the-operators-library-is-written-by-an-operators-own-act.md)'s
(P-0048 when this was written) own last clause — *"the location is printed so an editor knows where to point"* — at the resolution
the rule now needs.

### 5. The paragraph that lost is quoted where it stood, not deleted

`scratch.rs`'s header carries the resolution and quotes the old rule under *"**This replaces the
opposite rule**, which stood here until 2026-09-01"*, with the three things wrong with it. A comment
describing replaced behaviour is a defect of the change that replaced it
(`docs/contributing.md` §4:
*"Stale documentation is a defect of the change that made it stale, not housekeeping for later"*), and
a rule that was argued rather than assumed is one somebody will re-propose, so it is kept where the
re-proposal will land. `watch.rs`'s header is re-read the same way rather than cut: its *"Two slots
given the same files both rebuild, which is right"* is true of that type and was **read as a licence**
— *"both programs spent it"* — and what stands there now says so.

### What is genuinely lost is a warning, and it is worth naming

**A write can no longer tell an operator that two slots were about to move together, because the scan
that told them will be empty.** That is a real loss and not a nominal one: the sentence *This file is
also slot 1, which now shows the same procedure* was the only thing in the system that said, after
the fact, that an edit had reached a deck the operator was not editing.

It is a warning about an accident that can no longer happen, which is why it is a price rather than a
regression — the failure it announced is the failure this rule removes, and
[P-0094](../principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)'s *"half-applied and
silent is the same failure wearing different clothes"* is what the shared file was. But the honest
statement of the trade is that a late warning has been exchanged for an early one: **the obligation to
speak survives and moves to startup.** The operator is told which file belongs to which deck before
they open an editor, rather than told after a write that it landed somewhere else as well.

## Alternatives

### a. Keep the clause — two slots naming one file share one scratch file

The rule as it stood, and the one a reader arrives at from ADR-0088 alone, since that record is where
the scratch is explained. It loses three times over, and the module header keeps all three:

**It is circular**, above. **It contradicts the naming rule in the same directory** — a copy named
after its source alone puts ADR-0228's exact failure back into the one module written to stop it, and
a run with `--load-set --watch` was using both rules at once, in one directory. **And it costs the
thing the slots are for**: one shared file means the first edit moves all four decks, one save
rebuilds four slots, and three parked candidates enter a lane they can never leave.

### b. One file, four names — a link per slot

The shape that looks like it gets both: copy once, and give each slot a hard link or a symlink under
its own `A0-`/`B0-` name, so a listing reads per deck and the bytes are stored once.

**It is the shared file with a better listing.** A hard link *is* the same file, so an edit through
deck A's name still moves deck B, and every consequence above returns unchanged while the names
suggest they cannot. It is also editor-dependent in a way nothing else here is: an editor that saves
by writing a temporary file and renaming it over the target breaks the link and silently converts the
run to the per-slot rule, so the behaviour of the instrument would depend on which editor the operator
happened to open. The bytes saved are two `.kir` files per slot.

### c. Decide now what an operator who wants two decks to move together gets

Tempting while the mechanism is being taken away, and it is refused as **premature rather than wrong**.
Nothing has asked for it: the shared file was never a control an operator reached for, it was what
happened when they named one preset twice, and the only evidence about it is one report of it going
wrong. Designing the deliberate version now would be choosing a spelling on no evidence, which is
[P-0085](../principles/0085-take-the-mechanism-that-exists-and-pay-the-bill-now.md)'s *"Before a feature gets a
subsystem, check what it is made of"* read the other way round. It stays open below.

## Consequences

- **MCP's sharing sentence is now normally empty, and the scan stays.** `mcp.rs` is unchanged by this
  record, deliberately: it is handed a `Slots` and its sentence is true of whatever it is handed. A
  caller that shares still gets told, and the two tests that hand it a shared pair and a shared
  deformation still pass, which is what keeps the claim a measurement rather than an assumption.
- **The Staging lane reads as it was always documented.** `staging`'s rule — *"**A row is not removed
  when its slot is parked or its material is replaced.** A candidate that landed in a slot the
  governor then parks keeps its verdict outstanding, and that is `HotSwap::begin_frame_parked`'s own
  rule read from the surface: a slot that is not drawn is not judged"* — is unchanged. What changed is
  that a parked row is now a **frozen trial**, which is the reading it always had, instead of the
  by-product of every save reaching every slot. One save puts one candidate in the lane.
- **Startup output grew a line per deck in both programs**, and `docs/manual.md`'s *Where your
  work lives* section carries it with the printed example. That page is another writer's and already
  states the rule.
- **The edit history is untouched.** `history.rs`'s snapshots are keyed `(slot, layer, index)` and
  were already per slot, so seeding four copies files four chains exactly as before; what changes is
  that the four chains now describe four files instead of four readings of one.
- **ADR-0088's front matter is annotated and its prose is untouched** (`docs/contributing.md` §4),
  the way ADR-0185, ADR-0188, ADR-0197, ADR-0198 and ADR-0202 were. Its `superseded_by` names this
  record and **its `status` stays `accepted`**, because what fell is one clause: the three places, the
  gate on a run that can write, the dissolved `--mcp` / `--load-set` exclusion and the finding about
  `--save-set` all stand, and P-0048 still derives from it. That principle's own instruction is what
  the fields are for — *"The front matter exists to be written after the fact, which is the plainest
  evidence that a landed record was never meant to be untouchable"* — and ADR-0189 is the precedent
  for keeping `accepted` where a clause rather than a decision falls.
- **P-0048's *Where it holds* is re-pointed.** It named `scratch.rs` and `history.rs` in
  `karakuri-cli`; both moved to `karakuri-environment` with
  [ADR-0215](0215-the-package-is-karakuri-environment-and-a-module-belongs-if-what-it-deals-with-is-outside-this-process.md).
  A principle is a live registry rather than a record, so a stale pointer in one is a defect and is
  fixed rather than annotated (P-0093). The rule the file states is unchanged and it keeps its number.
- **No new principle is written here, and whether one is owed is the maintainer's.** The rule is the
  kind ADR-0000 says earns one — *"A rejection earns a principle only if a future proposal could
  violate it"* — and this proposal was made once already, by a record. What holds it today is
  `scratch.rs`'s header, the argument quoted in it, and
  `the_same_preset_in_two_slots_is_two_files_and_an_edit_moves_one_deck`, which is the test the change
  is about.

### What this leaves undone

- **Whether `--watch` gains anything on a run whose slots were given genuinely different files.** For
  such a run the copies were already one per slot, because the sources were, so what this changes
  there is the **name** — a slot prefix on a file that would otherwise be `field.kir` — and the
  per-deck startup lines. Whether that is worth anything to an operator who never shared a preset is
  not settled here, and the one thing that can be said for it is that the directory now has one
  spelling instead of two.
- **What an operator who wants two decks to move together does. There is no answer today, and this
  record does not invent one.** `docs/manual.md` currently answers with an instruction — make the
  same edit in each slot's file — which is an instruction rather than a mechanism, and
  `docs/contributing.md` §4 is what that is: *"When
  the fix for a defect is a sentence added to a prompt, a corpus, or a guide, the defect is still
  there and now it has a distribution channel."* Whether it is a defect at all depends on whether
  anyone wants the thing, which is Alternative c and is open.
- **Nothing removes scratch files a previous run left**, which is ADR-0228's last item and is now
  larger than it was: a two-slot run after a four-slot one leaves `C*` and `D*` files behind, and a
  shorter Set loaded onto a deck that held a longer one still leaves the surplus. They are
  unpointed-at rather than read, and no watcher will build them.
