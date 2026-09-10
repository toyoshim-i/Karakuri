---
id: 0338
title: A procedure is a row of the library, and one loaded over a layer makes a Set with no name
status: accepted
date: 2026-09-10
supersedes: []
superseded_by: []
principles: [0085, 0087, 0090, 0091, 0096]
tags: [library, console, inspector, operations, vocabulary, m5]
---

# A procedure is a row of the library, and one loaded over a layer makes a Set with no name

## Context

The Library bay lists Sets and nothing else. Every `.kir` this program has ever put in a store is
content-addressed — `Store::list_artifacts` reads `<hash>.kir` off the store root and answers a
list of hashes — so the parts are all there and not one of them is a row anybody can read, because
a hash is not a name and the population is every build the run has ever compiled rather than
anything a person kept.

The maintainer's, 2026-09-10, verbatim:

> 今はLibraryにSetしか出してないでしょ？ここにkirも置けるようにするのはどうだろう？そしてsetもkirも
> どのレイヤーを実装してるか名前の右にバッジをつける。setを選んだら全レイヤーをload、無いレイヤーは
> 空に。kirを選んだ場合は、今のsetにkirが実装してるレイヤーを上書きしたsetを作ってloadする。これに
> よって、例えばカメラだけ変更、みたいな事が出来るようになって気軽に絵を変換させられる。ライブラリ
> 一覧はフィルターボタン追加してL1を押すとL1を実装してるkirだけ表示、SETはsetだけ、各ボタンはトグル
> でORフィルター。
>
> そうするとInspectorはL毎に保存ボタンが欲しくなる。そう言えばInspectorはスロットC/Dも表示できない
> といけない。左右フリップで入れ替えるとか、各列ごとに選択できるようにするか。

**The library only shows Sets today — how about putting `.kir`s there too? And a badge at the right
of the name, on a Set and on a `.kir` alike, saying which layer it implements. Choose a Set and
every layer loads, and a layer it has not got goes empty. Choose a `.kir` and it makes a Set out of
the one you are running with that `.kir`'s layer written over it, and loads that. Which is what
lets you change just the camera, say — turning the picture over becomes a cheap thing to do. Add
filter buttons to the listing: press `L1` and only the `.kir`s implementing L1 show, `SET` shows
Sets, each button a toggle and the filter an OR across them.**

**Then the Inspector wants a save button per layer. And the Inspector has to be able to show slots
C and D as well — flip left and right to swap them, or let each column be chosen.**

And his two answers to the questions this raised. On what a `.kir` load leaves the slot playing:
*「名前のない派生なんだけど、コンパイル通った時のauto snapshotは生きてる」* — **a derived Set with
no name, and the auto snapshot on every compile stays alive.** On how a pane is pointed at a deck:
**a pulldown per pane, A–D**, which is [ADR-0305](0305-the-library-bays-load-is-a-button-and-a-pulldown-and-the-deck-it-names-is-not-the-selection.md)'s
card one bay along.

### What is already there, and what a subsystem would be

[P-0085](../principles/0085-take-the-mechanism-that-exists-and-pay-the-bill-now.md) is most of this
record. A second library, a second load path and a second file format are what a reading of the
request as *a feature* produces, and every part of it exists:
[ADR-0227](0227-a-pattern-and-a-master-chain-setting-are-library-data-in-two-tiers.md) is the two
tiers — what ships under `examples/`, the operator's under the store;
[ADR-0228](0228-a-library-load-re-points-the-slots-source-and-never-installs-a-set.md) is the load,
which writes files into the scratch and sends an aim;
[ADR-0314](0314-a-control-that-moves-a-field-of-the-aim-re-aims-the-slot-and-the-rebuild-is-the-write.md)
is the re-aim, which restates every field of that aim so nothing is left behind; and
[ADR-0304](0304-the-set-a-version-is-filed-under-rides-the-aim-that-re-points-the-slot.md) is the
`set` field on the aim, which is what a version is filed under. **A `.kir` load is a `LoadSet` with
one file replaced instead of all of them**, and every consequence below follows from taking that
sentence literally.

## Decision

Five, and the page moved first (`docs/contributing.md` §5).

### 1. The Library lists procedures beside Sets, in two tiers, and a badge says which layer

**A procedure is a row of the same list.** Not a sixth scope chip and not a second bay — the
maintainer's *ここに置ける* is one list, and the filter row below decides which kinds of row are in
it.

**Two tiers, ADR-0227's, and neither of them is the store's artifacts.**

- **The shipped procedures are the presets tier** — the `.kir` files under `examples/`, listed
  beside the `.kset` files already there. Nothing in this program writes there
  ([P-0096](../principles/0096-the-operators-library-is-written-by-an-operators-own-act.md)).
- **The operator's are `<store>/procedures/<name>.kir`**, written only by the operator's own act,
  which is decision 4 below. It is ADR-0221's shape a fourth time: a name the operator typed, one
  path component, a place of its own under the store root, bytes the store hands over whole.

**The content-addressed artifacts are not the library and this is where that is written down.**
Every build puts its sources in the store under `<hash>.kir` and `Store::list_artifacts` lists
them; that population is the *history's* — it is every version that compiled, which is
[ADR-0089](0089-history-is-gated-on-compiling-not-on-landing.md)'s gate, and the `history` scope is
already the surface over it. A library row is something a person kept and can name. Listing the
artifacts would put a row per compile in the bay an operator browses, under a name that is a hash,
which is the same mistake in reverse that
[ADR-0299](0299-my-sets-is-the-starred-subset-and-the-star-is-kept-beside-the-sets.md) closed for
`my sets`.

**The badge is read off a card, and the read is a directory read.** A procedure's badge is its one
`kind`: `meta::card` writes `kind` on every artifact's card and
`history::declared_kind` scans the line off a source, so neither costs a compile. A Set's badges are
the layers its `slot` records fill, which the Set file already says.
[P-0091](../principles/0091-cost-is-known-before-it-is-paid.md) — the listing is built at startup
and on the press that changes the scope, as `Store::favourites` is, and never on the frame.

**Everything else about a row is unchanged.** The scope chips, the `history` walk, the reading block
a `params` press opens, the row menu and the drag all keep working, and a procedure row's reading is
its own card — the knobs it declares with their ranges and defaults, its capacity where it declares
one, what it emits. That is the same block one node deep instead of one Set deep.

#### Which scopes list a procedure, and the two that do not

**`all` and `presets`.** `all` gains the operator's `<store>/procedures/`; `presets` gains the
`.kir`s under the presets root beside the `.kset`s.

**`my sets` does not**, and it is not an omission. A star is refused on an id `<store>/sets/` does
not hold — `StoreError::NoSet`, ADR-0299 — so there is nothing to star on a procedure row and the
starred subset of the Sets is exactly what that chip is.

**`folder` does not**, and the note that says so stands as written. *A folder scope reads Sets, and
a bundle is not a third thing* settles that a folder row is a **take**, and there is no take-in for
a bare `.kir`: `--take-in` reads a Set file and checks every inlined source against the address its
`slot` record names, which is the whole of what content addressing buys, and a loose `.kir` names
nothing. **So *A directory of `.kir` files is a directory of parts* is still true of a directory
somebody dropped on the window**, and what has changed is that this program now has two directories
of parts of its own, with names in them.

**What that clause of ADR-0227 no longer covers is `presets`.** That record's consequences say *"the
Library's `presets` scope is not this … a `.kir` is explicitly not a row there"*, written of a
pattern and a chain setting. It is superseded for the `.kir` and for nothing else: a pattern and a
chain setting are still not rows of this bay, and where they are browsed from is still the
Sequencer bay's question.

### 2. The filter row is six toggles over the kinds, OR across them, and it supersedes ADR-0262's `layer` field

`L1 L2 L3 L4 FIELD SET`. **Each is a state and never a cycle**
([P-0090](../principles/0090-a-surface-offers-it-never-decides.md)): a press names on or off, so a
map with a button per direction, a model and a key can all say *show me the cameras* and mean it.
**OR across the ones that are on**, and **none on is everything**, which is the state a run opens in
and the state a press can always get back to.

`holds…` is untouched. It steps the node names the listing contains, it is still a first cut, and
what a filter field should be is still open (ADR-0292 took its premise away and this record does not
answer it).

**One operation, *Filter the library by kind*, carrying all six states.** A press names the whole
row rather than one chip, for *Narrow the published interface*'s reason: six presses that each say
*this one changed* are six statements two hands can disagree about, and one that says *these are the
kinds showing* is a destination.

**This supersedes ADR-0262's `layer` field, and the reason is that the two mean different things.**
That field steps `L1 L2 L3 L4 FIELD` and asks *which Sets hold a node on this layer* — a predicate
over a Set's contents. A toggle here asks *which procedures are of this kind* — a predicate over one
artifact. They are two facts and folding them into one field would be a name meaning two things,
which is what `docs/contributing.md` §4 is written against. **And the control is wrong for the
question anyway**: a filter over a closed list of six is a set of toggles, because every subset of
six is askable and a position in a cycle can only ever name one. ADR-0262 said the layer half was
*"settled rather than open — five is a closed list, and stepping a closed list is the right control
whatever is decided next door"*. That was right about the list and wrong about the control, and what
changed is that the list grew a sixth member which is not a layer at all.

**What it costs, and it is the one cost.** *Show me the Sets with an L2 in them* stops being
askable on the panel. `Operation::ListSets`'s `layer` field is unchanged and still carries it, so
`list_sets` over MCP and `--list-sets` are where that question lives; the panel's answer to *which
Sets use this* is `holds…`, which asks it by node name.

### 3. A procedure loaded over a layer re-aims the slot, and the slot then runs a derived Set with no name

**The press is a re-aim and nothing installs.** A press, the `load` button or the row menu on a
procedure row takes the deck's **current material**, replaces that layer's file with this one, and
sends the aim — every other field restated, `Aiming::changed`'s single derivation
([ADR-0314](0314-a-control-that-moves-a-field-of-the-aim-re-aims-the-slot-and-the-rebuild-is-the-write.md)),
so the layering, the fold, the capacities, the salts, the camera and the edges come back as the
slot's own rather than as a default. The worker compiles it off the render thread, it lands at a
frame boundary, it is judged on what one frame of it costs, and the Staging lane says which of the
three happened
([ADR-0228](0228-a-library-load-re-points-the-slots-source-and-never-installs-a-set.md)).

**Which position is replaced: the first node of that kind, and the limit is recorded rather than
designed around.** A procedure declares one `kind` and nothing else about where it goes, and a
library row cannot say an index, so the payload does not carry one — an index only one surface could
ever fill would be a field for a control nobody has drawn. So `L4:0` is the renderer a `kind L4`
replaces and the second renderer of a three-renderer Set is unreachable from this row. **The day the
Inspector's node head grows a *replace this node* control is the day the payload gains a `NodeAt`**,
which is ADR-0228's own shape for a limit it chose to keep — *"a Set that recorded two different
capacities loses the second"* — stated at the point it bites rather than left to be found.

**Where the material has no node of that kind, the procedure is added as node 0 of it.** That is the
case the request is about: a Set of a geometry and a renderer holds the built-in orbit at `L3:0`
because its files declare no `kind L3` (`docs/ir-spec.md`, *Several cameras*), so a `kind L3` row
takes that position and the picture changes camera with nothing else moving. A node that declares an
input nothing binds is refused where the Set is built, by name and with what the Set does hold —
the wall that is already there, met from one more direction rather than a new one.

**The slot then runs a derived Set with no name**, which is the maintainer's answer. The strip's
readout reads `<base> + <kir>` — `drift_night + orbit_wide` — so what is playing says what it is
made of and never claims to be a Set the library holds. `keep` files it as a new Set, under a typed
name from the Inspector pane head or under a stamp from the capsule, exactly as any other deck is
kept ([ADR-0292](0292-the-pane-heads-name-takes-letters-and-the-keep-capsule-stays-a-stamp.md)).

**And the history goes on filing versions under the base Set's id.** `Aim.set` **stays the base
id**: a load of a procedure over a layer does not move it, where a load of a Set replaces it. So
every version that compiles after the swap is filed under `@<base>`, the `history` scope goes on
listing them for that deck, and *the auto snapshot on every compile stays alive* — which is the half
of his answer that has a mechanism behind it
([ADR-0304](0304-the-set-a-version-is-filed-under-rides-the-aim-that-re-points-the-slot.md),
[ADR-0308](0308-the-library-bays-fifth-chip-walks-one-sets-history-and-a-row-lands-that-version-on-a-node.md)).

**A Set row is unchanged and loads every layer**, and a layer the Set has not got is **empty**, which
means the slot holds no node of that kind. L2 and Field are ordinarily empty — the launch pair is an
L1 and an L4 and nothing else, so the engine has always run Sets with neither. **L3 is never empty**,
because `L3:0` addresses something in every Set: a Set declaring no camera holds the built-in orbit,
and *empty* there means the built-in comes back. **L1 and L4 cannot be empty**: a Set with no
geometry or no renderer is not a Set, and nothing about this row changes that.

### 4. The Inspector keeps one node's procedure, and a library write is not a session record

**A `keep` capsule on each node group's head**, which is the maintainer's *L毎に保存ボタン* at the
granularity the Inspector actually draws — a node group is one node, or the fold that stands over a
deck's renderers, and a head standing over several carries no capsule for
*Set a node's authority*'s reason: one control on it would be one of several answers drawn as the
answer.

**It writes that node's source into `<store>/procedures/`**, which is what makes the operator's tier
of decision 1 exist at all — P-0096, *the operator's library is written by an operator's own act*.
The name comes from the pane head's letter-taking flow where the operator typed one
([ADR-0292](0292-the-pane-heads-name-takes-letters-and-the-keep-capsule-stays-a-stamp.md)) and from
a stamp where they did not
([ADR-0287](0287-the-keep-pill-files-under-a-stamp-because-the-consoles-one-letter-taking-flow-is-an-arrangements-name.md)) —
[ADR-0128](0128-a-set-saved-under-a-name-the-caller-chose-overwrites.md)'s two, drawn on one
capsule, exactly as the deck's own `keep` draws them.

**A model asked for this lands in the sandbox**, and that is what tells it from a star. P-0096's
third row and [ADR-0261](0261-a-model-asked-save-lands-in-a-sandbox-because-the-operators-library-is-the-operators-own-act.md):
what a model saves goes to `<store>/sandbox/`, stamped, overwriting nothing, and readable by nothing
that reads the library — a model is not refused and it does not write the operator's library.
[ADR-0301](0301-a-models-star-is-refused-because-a-favourite-has-no-sandbox-to-land-in.md) refused a
model's *star* because a favourite has no sandbox form to land in; a kept procedure is a file, so it
has one, and this row goes the other way for the reason that record names.

**It writes no session record**, and that is the arrangement rows' answer rather than a gap: nothing
in the session vocabulary is a library write, a replay reconstructs nothing from one, and the
vocabulary having no row for it is the decision. `Written::Silent(Silent::Surface)`, where
*Save the arrangement* already is and for the sentence that transfers word for word — *a save writes
a file, and a file is not a record whose timing `OnLanding` could be about, nor a gap `NoRecord`
could be about*.

### 5. An Inspector pane is pointed at a deck by a pulldown per pane, A–D

The `▾` the mock has drawn on the pane head since the Inspector was drawn becomes a control. It
lists the decks the mixer is drawing strips for — `View::select`'s refusal read a fourth time rather
than a fourth rule — and a pick points that pane at that deck. **The pane's target is the pane's own
state**, beside its scroll position (ADR-0307): no other pane moves, the deck selection does not
move, and the Library bay's load pulldown does not move.

**A pulldown and not a flip**, which is the maintainer's choice and P-0090's rule underneath it: a
flip is a step, two panes stepping disagree about where they are, and a surface that can only step
has no way to arrive. A pulldown names a state, so a key, a map and a model can all say *pane 1
shows deck C*.

**One operation, *Point an Inspector pane at a deck***, and it is emphatically not
*Select a deck*: that operation moves where the keys are addressed, and this mark exists so that a
pane can show a deck the keys are not on — which is ADR-0305's argument for the load pulldown, one
bay along. `Written::Silent(Silent::Surface)`, which is `SelectDeck`'s own answer.

**Its class is open, where `SelectDeck`'s is closed.** The deck selection is
`ClosedUnclassed(Unclassed::Selection)` because it decides where every later keyed operation lands,
so a wrong one puts the next press on the wrong deck. A pane's target decides which rows are drawn
and addresses nothing: every operation the Inspector emits names its deck outright. P-0094's
question asked of it comes back empty — at its worst, on the frame it goes wrong, it has redrawn a
pane and moved no deck, no fader and no pixel.

## Alternatives rejected

### a. Auto-saving a Set on every `.kir` swap

The obvious way to keep the derived material nameable: write a Set into `<store>/sets/` on every
swap so the slot is always running something the library holds. It is refused twice.

**It fills the library with things nobody kept**, which is exactly the symptom ADR-0299 closed —
*"a `my sets` filling up with things the operator did not make"* — arriving through the other door,
and worse, because the request is for *気軽に* : a control an operator is meant to press repeatedly
while looking for a picture would leave a row per press. **And it is a write on the frame path in
the one place P-0091 is sharpest**: a swap is judged at a frame boundary, and a disk write hung off
it puts a file system between the operator's hand and the picture.

The maintainer answered this directly — a derived Set with **no name** — and what keeps the material
recoverable is the mechanism that already does: every version that compiles is snapshotted, gated on
compiling rather than on landing.

### b. Procedure rows in a scope of their own

A sixth chip, `parts`, beside `all`, `my sets`, `presets`, `folder` and `history`. It is the tidier
data model and it loses on what the operator is doing: the request is to swap **one layer of what is
playing** while looking at the material it is going into, and a scope is a *different library* —
choosing it puts the Sets away. It also makes the filter row meaningless, since a scope that holds
only procedures has nothing to filter kinds out of, and it would put the same `.kir` under two chips
the moment it were asked to show the presets' procedures as well.

**One list with a filter row is the shape the maintainer asked for** and it is the one that answers
*show me the cameras and the Sets* at all.

### c. A step field for the kinds, as ADR-0262 built for the layer

The cheapest change: leave the field where it is and give it a sixth entry. It is refused on what a
filter is. A field steps to **one** value and every subset of six is a question somebody has — *the
cameras and the fields*, *everything but the Sets* — so a cycle can express six of the sixty-four
states the row has. It is also two controls' worth of presses to get back to unset from the middle
of six. ADR-0262 chose stepping for a closed list of five and said so; the list is not the argument,
the arity of the answer is.

### d. Filing the derived Set's versions under `None`

The literal reading of *a Set with no name*: `Aim.set` goes to `None` on a procedure load, since
what is running is no longer the Set the id names. It is refused by the second half of the
maintainer's own answer — *the auto snapshot when it compiles stays alive* — and by what `None`
does. `history::list` narrows by Set id and a `None` matches none of them (ADR-0308), so every
version written after the swap would be filed under nothing and the `history` chip would go empty on
the deck an operator is actively editing. **That is the state a typed pair is in, and it is a state
rather than an unknown**; putting a load into it would make the walk go blank at the moment it is
most wanted.

The base id is the honest answer as well as the working one: the derived material *is* that Set with
one layer over it, the versions are versions of its nodes, and a `keep` afterwards is what gives the
result a name of its own.

### e. A left/right flip for the Inspector's panes

The maintainer's own first suggestion — *左右フリップで入れ替える* — and he chose the pulldown in the
same message. Recorded because it is the cheaper control and somebody will re-propose it: a flip is
one target instead of four rectangles and it needs no card. It loses on P-0090. A flip is a step, so
two panes on a four-deck arrangement cannot both be aimed without knowing where they started; a
second surface stepping the same pair disagrees about where it is; and there is no way for a key, a
map line or a model to say *show me deck C* — every one of them would have to count presses. It is
also two mechanisms where there is one: a pulldown over the decks the mixer draws is already on this
console twice, in the Library bay's foot and in the arrangement pill's menu.

## Consequences

**Dated 2026-09-10 and written as what the two implementation passes build**, to be rewritten by
them as a description of the tree.

- **[Every operation](../manual/operations.html) gains four rows** — *Filter the library by kind*
  and *Load a procedure over a layer* under *The library*, *Keep a node's procedure* and
  *Point an Inspector pane at a deck* under *Inside a Set*. **Every panel badge is `plan`**, because
  nothing on the console emits any of them yet, and `karakuri-console/tests/panel_column.rs` is what
  would catch a `has` that is not true. The key column is `gap` on all four: no key of the grammar
  reaches any of them today and none is reserved.
- **`karakuri-operation` gains four variants and one type.** `FilterLibrary { kinds }`,
  `LoadProcedure { deck, procedure }`, `KeepProcedure { deck, node }`,
  `PointPane { pane, deck }`, and `LibraryKinds`, which is six named booleans. Their titles are the
  four `<h3>`s verbatim and `the_manual_and_the_vocabulary_agree` compares them.
- **`gate.rs` classes them**: `LoadProcedure` is `LoadSet`'s class exactly — the one class that is a
  predicate over its target, `Closed(Class::LiveDeck)` where the deck it names is live and open
  where it is not — and `FilterLibrary`, `KeepProcedure` and `PointPane` are `Standing::Open`. The
  keep is where `SaveSet` is, for `SaveSet`'s reason: it writes a file beside the library and moves
  no deck.
- **`karakuri-operation-record` answers three ways**: `Silent(Question)` for the filter, which asks
  the library for a narrowed listing exactly as `ListSets` does; `Silent(NoRecord)` for the
  procedure load, which is `LoadSet`'s own answer and its cost unchanged in size — a session
  replayed does not come back with the layer a hand swapped; and `Silent(Surface)` for the keep and
  the pane pointer.
- **`karakuri-environment`'s `mcp.rs` gains four rows of `SPELLED` and four arms of `sayable`, and
  `operate` takes none of them.** The three that are `plan` are `Unperformed` — nothing on the frame
  the drain lands on performs them yet — and *Filter the library by kind* is `Window`, ADR-0315's
  sentence: it is a bay's own narrowing and a model has no window. **What a model actually wants
  here is a listing of procedures, and that is `list_sets`' to grow rather than this row's** — it is
  named on the roadmap and decided by nobody yet.
- **`karakuri-store` gains a `procedures/` directory**, established by `Store::open` the way
  `arrangements/` is, so a store written by an older build gains it the first time this one opens
  it. Bytes it does not parse, one path component, a name the operator typed.
- **`karakuri-environment`'s `places.rs` lists the presets root's `.kir`s** beside its `.kset`s, and
  a row carries the `kind` `history::declared_kind` scans off it.
- **`karakuri-console` gains a row kind, a badge, a kind-filter row and two controls**, and
  `docs/manual/console.html` specifies all of them: `.badge` at the right of a row's name,
  `.lib-kinds` under `.lib-filters`, a `keep` capsule on a node group's head and the pane head's
  `▾` made live. `style.css` gains `.badge`, `.lib-kinds` and `.kind`.
- **`crates/karakuri` gains one arm beside `played`**, which re-aims the slot with one file
  replaced instead of every file, and leaves `Aim::set` where it is.
- **`docs/manual/concepts.html` is corrected rather than extended.** *A Set is what you keep and
  what you load — the unit the library is made of* stopped being true the moment a procedure became
  a row, and a page that contradicts itself is worse than one that is short.
- **No new principle.** This applies P-0085, P-0087, P-0090, P-0091 and P-0096, and takes nothing
  back from any of them.
- **Two limits are recorded rather than fixed**, and both are named above so that they are met as
  decisions rather than as surprises: a procedure load reaches node 0 of its kind and no other, and
  a `.kir` in a dropped folder is not a row.

### What decisions 1, 2 and 3 became, 2026-09-10

The Library and host pass, written as a description of the tree.

- **`procedures/` is `Store::open`'s fifth directory**, with `Store::PROCEDURES`,
  `Store::PROCEDURE_FILE_SUFFIX`, `Store::list_procedures`, `Store::read_procedure` and
  `StoreError::NoProcedure` beside it. **It reads no `kind` line**, which is the one thing the
  decision above left to the pass: this module keeps files it does not parse — the line
  `arrangements/` already draws — so `history::declared_kind` stays the single scanner and the two
  callers pair it with the listing. That costs one small read per row, on the press that builds a
  listing and never on a frame.
- **`places::Presets::list_procedures` lists the shipped tier** as `PresetProcedure { name, file,
  kind }`, and it **does** open each file, because a badge is what that listing is for. A `.kir`
  declaring no `kind` is a row with no badge rather than a file dropped silently.
- **The console's seam is `RowKind` and `Rows`**, a row of what each listed row *is* beside the row
  of names, and **a row with no entry is a Set with no badge** — so every surface that was handed a
  listing and nothing else draws exactly what it drew. `LibraryBay::badges` lays the words out from
  the right of the row; `Rows::set` is the one question the star, the `params` chip and the send
  each ask, so a procedure row hands them nothing rather than being special-cased three times.
- **The six toggles are `KindChip`**, `LibraryBay::kinds`, `kind_chips` and `kind`, and a press
  flips one field and emits all six. `Field::Layer`, `LAYER_UNSET` and `stepped_layer` are gone with
  the `layer…` field, off the mock and off the panel together; `Filters` carries `kinds` where it
  carried `layer`, and `View::narrow` takes a `LibraryKinds`.
- **A procedure row's menu is the loads and no separator.** `RowMenu`'s `rule` and `save` became
  `Option`s and `Menued` gained `sends`, because `Save as a kbset` inlines every source a Set names
  and checks it against the address its `slot` record carries — and a bare `.kir` names nothing,
  which is this record's own reason a `folder` lists no procedure.
- **The load is `main.rs`'s `overlaid`/`overlaying` beside `played`.** It finds the file in the two
  tiers in the order the bay lists them, reads the slot's own files to find the first node of the
  declared kind, writes the procedure into the deck's scratch under `scratch::node_name`, and
  re-aims through `Aiming::changed` — so every other field is restated and **`Aim::set` does not
  move**. A replaced node **keeps the name the Set gave it**, because an `edge` and a `bind`
  resolve against it; a node that is *added* is named after the row, and a name the slot already
  holds is refused rather than shadowed. The strip's `<base> + <kir>` is `base_material` and
  `derived_material`, and the base is `Aim::set` or the pair the run was launched with.
- **`mcp.rs`'s `Load a procedure over a layer` gained its `make` and `shape`**, spelled like
  `LoadSet` beside it. **`sayable` stays `Unperformed`**: what this pass wired is the press path,
  and the frame the MCP drain lands on is M5.10's owed row.
- **Three limits are recorded where they bite**, beside the two the decision already named. A preset
  **Set** row carries no badge, because that listing opens no file and reading every Set file on a
  press is a cost nobody asked for. A `params` press on a **procedure** row asks nothing, because
  the reading is a Set's and a procedure row's own card is a reading nobody has drawn. And a `.kir`
  declaring no `kind` is listed, badgeless, showing only while no chip is on, with a load off it
  refused by name.

### What decisions 4 and 5 became, 2026-09-10

The Inspector pass, written as a description of the tree.

- **The `keep` capsule is `view::node_keep`**, hard against `.node-head`'s right-hand padding at a
  `.mini`'s own box, and **the three authority chips are laid out inside what it leaves**. That trim
  is `auth_head` and it is applied **inside `auth_chips`** rather than at its two callers: a caller
  that forgot it would put three chips under the capsule, and the paint and the hit-test would agree
  with each other and disagree with the mock. `InspectorPane::keep_procedure` is the press,
  `input::PROBES` carries a row for it, and `tests/node_keep.rs` is what watches the two rectangles
  stay apart.
- **The two heads that carry none are `view::Node::keep` being `None`**, and it is a field beside
  `Node::authority` rather than that field read again — the two absences are not the same set, since
  the built-in camera has an authority and has nothing to keep. The host answers it off
  `Set::cameras`' own sentence, *never empty, and the last one is always the built-in orbit*, so
  neither absence is a check written twice.
- **The capsule emits `id: None` and the host pairs it with the head.** `crates/karakuri`'s press
  arm asks `View::naming_over(deck)` — what the head of the pane showing that deck is taking letters
  into — so ADR-0128's two routes are one capsule here as they are for the deck's own `keep`, and
  the stamp is `history::stamped_id` where no head is asking.
- **`karakuri-store` gained the writer as well as the directory**: `Store::write_procedure` and
  `Store::write_sandbox_procedure`, and **a name already kept is refused rather than overwritten**
  (`StoreError::ProcedureTaken`). That is where a procedure differs from a Set id and an
  arrangement's name, and the difference is what each name is over: those two are instructions to
  replace, and this one would replace a part of somebody's library with a different node's source.
- **`crates/karakuri` gained `Kept` and `Keeping::keep_procedure`**, which is `save_set`'s shape one
  node down — the bytes are the run's (`Playing`), the write is on a thread of its own, and the
  outcome is said at the frame it arrives on through `finished_keeps`. A node a build landed carries
  no bytes, so the writer reads them back out of the store by the address it carries; **neither
  route re-reads the `.kir` on disk**, which is `SavedNode::source`'s own rule.
- **`mcp.rs`'s *Keep a node's procedure* is `Operable` and not `Unperformed`.** The list above said
  `operate` would take none of the four; it takes this one, because the drain gained an arm on the
  same frame the capsule did. `App::operated` hands it to the same writer with `Asked::Model`, so a
  model's lands in `<store>/sandbox/` — stamped, overwriting nothing, and readable by nothing that
  reads the library — and its MCP badge went `has` with the panel's rather than staying on M5.10's
  owed list. **A model is not refused here where its star is**, which is ADR-0301's own sentence
  read the other way: a kept procedure is a file, so it has a sandbox form to land in.
- **The pulldown is `DeckName::chevron` made live**, which is ADR-0292's *the chooser is boxed in*
  spent: that record reserved the rectangle and painted nothing, and `view::PaneTarget` is what took
  it. **The run gave way rather than the control** — `deck_name`'s clip now stops one gap short of
  the mark — because the run is the one thing in that row that is clipped rather than dropped.
- **A pane's target is `View::pane_deck`, one per pane, beside `View::scroll`**, and the host reads
  it to fill the panes: `inspector` takes the targets instead of filling pane *n* from slot *n*,
  which is what makes slots C and D reachable at all. `PANE_DECKS` is deck A and deck B, and that is
  now a default rather than a rule. `pointed_pane` performs the pick beside `pointed`, re-reads the
  pane it moved, and moves no selection, no other pane and no load target — `tests/pane_target.rs`
  asserts all three.
- **`input::claim`'s rule 2 gained a sixth card**, which is this pulldown's: it hangs off a head at
  the top of a pane and down over that pane's own groups, so while it is down a press inside it
  belongs to the card and a press anywhere else is the dismissal.
- **`room.rs` gained no constant**, which is worth writing down because the list above expected one:
  the capsule is a `.mini` at the node head's own `gap: 7px` and the mark is `CHEVRON_W`, and both
  were already spelled for controls this console draws elsewhere.
