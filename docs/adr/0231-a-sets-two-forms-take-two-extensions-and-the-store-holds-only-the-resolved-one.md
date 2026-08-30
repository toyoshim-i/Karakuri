---
id: 0231
title: A Set's two forms take two extensions, and the store holds only the resolved one
status: accepted
date: 2026-08-30
supersedes: []
superseded_by: []
principles: [0005, 0019, 0026, 0031, 0058, 0060, 0066]
tags: [store, format, distribution, naming]
---

# A Set's two forms take two extensions, and the store holds only the resolved one

## Context

**[ADR-0229](0229-a-set-file-is-authored-beside-its-parts-and-travels-as-a-bundle.md) gave a Set
file two forms and left them one extension**, and said so under *What this leaves undone*, verbatim:

> **The extension is not chosen.** A bundle and a store Set are `.set.ndjson`; whether an authoring
> file shares it or takes one of its own is open, and part 6 (*only the Set extension shows*) makes
> it load-bearing for the tree rather than cosmetic.

[The roadmap](../roadmap.md) carries the same item, and its own reason for calling it load-bearing
is the Library's tree — *"the extension is what the bay lists and what it hides"* — with **what it
blocks** named as the authoring form itself. That is a true reason and it is the smaller one. The
larger one is in the engine, and no document said it until this record.

### What the store's layout was, and where the suffix is spelled

`crates/karakuri-store/src/store.rs` builds the name in one place and takes it apart in another.
`set_path` was `self.root.join("sets").join(format!("{id}.set.ndjson"))`, and `list_sets` derives
the id straight back off it:

```rust
let Some(id) = name.to_str().and_then(|n| n.strip_suffix(".set.ndjson")) else {
    continue;
};
```

Its doc comment already states what that `continue` is for, and states it as a rule about the
listing rather than about the engine: **"A name the layout does not claim is skipped, not
repaired."** An editor's backup, a `.tmp` from a write that died, a subdirectory somebody made —
*"reporting one as a Set under a truncated id would invent a library entry `Store::read_set` cannot
open."*

**So the id has always been the suffix with the suffix removed**, and that is the fact this record
turns to account. Outside `docs/adr/` the string `.set.ndjson` occurred thirty-seven times across
fourteen files on the morning this was decided, and **exactly one of them built a path a running
program uses** — `set_path`. Everything else was a doc comment, a manual row, or a test fixture
that reaches around the store and spells the layout by hand. The layout was never a wide surface;
it was one line and a great deal of prose about it.

### What the documents say, and what they are silent about

[`docs/ir-spec.md`](../ir-spec.md)'s *Set file format* opens **"`.set.ndjson`. One record per line.
Concatenation is composition."** — one form and one extension. It knows a bundle only as a property
of that same file (*"When bundled, the source is inlined as a run of `{"t":"src",…}` records"*), and
it does not know the authoring form at all; ADR-0229 booked that as a debt it could not pay because
the file was held by another session.

The three-places table under
[P-0048](../principles/0048-what-ships-what-you-saved-and-what-you-are-editing-are-three-places.md)
spelled the row `<store>/sets/<id>.set.ndjson` in **two** module headers —
[`scratch.rs`](../../crates/karakuri-environment/src/scratch.rs) and
[`places.rs`](../../crates/karakuri-environment/src/places.rs), the latter arriving with
[ADR-0230](0230-where-the-programs-data-lives-is-told-rather-than-baked.md) and calling itself
*"The third row of `crate::scratch`'s table, which never had an address"*.

**And [the console page](../manual/console.html) settled the question this decision could be
mistaken for**, in *A folder scope reads Sets, and a bundle is not a third thing*:

> The question was whether a folder holds Sets or artifacts, and it had three candidates: a
> directory of `.kir` files, a directory of Set files, and a bundle. **Two of the three are one
> thing and the third is out.** A Set file and a bundle are the same file — one record per line,
> and a bundle is that file with the sources it names appended inside it — so the scope draws one
> kind of row rather than two.

Its closing sentence is the one to keep: the difference between a Set file and a bundle *"is a
property of a file, not a kind of collection."*

## Decision

The maintainer's, on 2026-08-30. **`.kset` is the authoring form. `.kbset` is the form the store
holds and the form that travels.**

**The argument is atomicity, and it is his rather than a preference for tidiness.** His words:

> load次にストアするコピーはkbsetじゃないといけない。だから区別する必要がある。この検証をサボると
> アトミックなセット入れ替えができなくなる。

— *the copy stored on load has to be a `.kbset`; that is why the distinction is needed; skip that
check and atomic set swapping stops being possible.*

**A swap happens on a frame boundary and an over-budget Set rolls back on its own**
([P-0005](../principles/0005-a-swap-happens-on-a-frame-boundary-and-an-over-budget-set-rolls-back-on-its-own.md)):
pipelines are double-buffered and exchanged only between frames, the governor rolls a Set back
without anyone asking, and the Set it replaced is parked unstepped so it returns where it left off.
**That holds only because nothing is left to resolve at the moment of the swap.** Let a form that
names its parts by **relative path** into the store, and a load walks the filesystem while
swapping: a neighbour may be missing, may have changed since the file was written, may fail
halfway — and the failure lands after the exchange has begun, in the one place the design has
arranged to be unable to fail. **A swap that can partially fail is not a swap.** So the store's
invariant is that everything in it is already resolved, and **the extension is what makes that
invariant checkable**.

**Where the cut falls is not "bundled or not".** It is: *does reading this file resolve paths
against the filesystem?*

| | names its parts by | needs | wall |
| --- | --- | --- | --- |
| `.kset` | relative path | its neighbours | yes — path traversal |
| `.kbset` | content address | the store, or the sources inlined in it | none |

**Inlined-or-not is a property within `.kbset` and not a third format.** A `.kbset` whose artifacts
are all already in the receiving store is the same file as one carrying them, read by the same
reader, refused by the same sentence when a `slot` names something neither inlined nor present.
**Which is exactly what the console page already settled**, and its argument survives whole: it
weighed a directory of `.kir`, a directory of Set files, and a bundle, and the two that are one
thing are still one thing. **What ADR-0229 added is a fourth candidate that page never weighed** —
a file that names its parts by path — and that is the one taking the second extension. Two
extensions are not two kinds of row: ADR-0229's part 3 lists both forms in the Library, and its own
consequence already put the authoring file *"on the same side"* of the listing. The scope still
draws one kind of row. What differs is which of them a store may contain.

**Where the check lands is a place the code already looks, and no new check is added.** `set_path`
builds the name and `list_sets` strips it to recover an id, so **an id cannot exist without the
suffix** — and an id is the only route a Set has to a deck. Make the suffix `.kbset` and a `.kset`
dropped into `sets/` has no id, is named by nothing, and can be asked for by nobody. The check that
does this was written to keep an editor's backup out of a library listing; it now also keeps an
unresolved file out of a swap, and not one line of it changed to acquire the second job. The
store's own comment on the constant is where that reasoning now lives:

> **This is not a new check.** The suffix was always stripped to find an id, so an id could never
> exist without it; what changed is that the check now means something.

That is the shape [P-0026](../principles/0026-a-guarantee-is-structural-or-it-is-a-convention-that-says-so.md)
asks for — the guarantee is structural, carried by the name a file has rather than by a rule
somebody remembers — and
[P-0019](../principles/0019-prefer-the-mechanism-that-already-exists.md)'s test is passed rather
than dodged: the mechanism already existed and was asked what it was made of before anything was
built.

**This deletes `.set.ndjson` as a Set's spelling everywhere outside `docs/adr/`** — the one
production construction, the manual's row, the specification's heading, both copies of the P-0048
table, and every fixture that spells the layout by hand. Those records keep it because they record
what was true.

**And it deliberately adds no constant for `.kset`.** Nothing in this workspace reads one: there is
no authoring form, no resolver and no filter, so a `const KSET` would be a claim about a design
rather than part of one —
[P-0031](../principles/0031-a-name-means-one-thing-across-the-system.md)'s closing rule, *delete
what nothing reads*, applied before the thing exists rather than after.

**A file whose extension and contents disagree is refused, not repaired**, and the contents are the
authority. A `.kbset` carrying a relative-path include is not a `.kbset` with a mistake in it; it is
a file that lied about which form it is, and the load refuses it naming what it found. This needs no
new principle, because it is
[ADR-0158](0158-a-saved-arrangement-that-disagrees-with-itself-is-refused-not-repaired.md)'s shape
exactly: *"a file that disagrees with itself is a file to report"*, and repairing it *"turns 'this
session file is damaged' into a panel that opens with regions somewhere the operator did not put
them, and nothing anywhere says why."* **It is prospective.** No include record exists, so there is
nothing yet for a reader to disagree with, and this sentence is here to be found by whoever writes
the reader rather than to describe a check that runs today.

## Alternatives

### a. One extension for both forms

The obvious one, and the one ADR-0229 left the door open for: `.kset` for everything, with the form
told apart by reading the file — a `slot` naming a path is an authoring file, a `slot` naming a hash
is a resolved one. It needs no rename beyond the one, it keeps the Library's tree simple, and it is
what "a bundle is not a third thing" sounds like it is asking for.

**It loses because the distinction is not cosmetic — it is what the store's invariant is checked
by.** Under one extension, *is everything in `sets/` resolved?* becomes a question that can only be
answered by opening every file in it, and the answer is stale the moment somebody copies one in.
The listing would have to parse rather than name, which is the opposite of what `list_sets` is —
*"it stands by naming rather than by opening anything"* — and a store's invariant that is only true
of files a particular program wrote is not an invariant, it is a habit. The maintainer's sentence
is the compressed form of this: *この検証をサボると* — skip **that** check, the cheap one, and the
expensive guarantee downstream is what stops holding.

**And it makes the failure silent and late.** A `.kset` copied into `sets/` under one extension is
an ordinary library entry until the frame it is swapped in on, at which point the resolution
happens in front of an audience. Under two, it never had a name.

### b. `.kpack` for the resolved form

Weighed seriously, and the better of the two rejected names. *Pack* says what the file does for a
person — it is the thing you hand over, self-contained, nothing else needed — and it is the word
every packaging tool has taught them.

**It loses because the resolved form is also what the store holds when nothing is inlined**, and
nothing about that file is packed. A `.kbset` whose artifacts are all already in the receiving store
carries no `src` runs at all; it is a list of content addresses and the store behind it. Call it a
pack and the name is wrong for the ordinary case and right only for the case that travels — so a
reader meeting `sets/opening.kpack` in a store would reasonably conclude it was a copy of something
sent to them, which it is not.

**`.kbset` names the property and `.kpack` names the shape.** That sentence is
[P-0060](../principles/0060-name-the-property-not-the-shape.md)'s — *a shape is one instance of a
property* — and it applies here in its own terms rather than by analogy: being packed is one
instance of being resolved, and naming the instance would leave the property with nothing carrying
its name. **What does not transfer is the rest of that principle**, whose recorded case is a fix
brief and a duplication that relocates when you name the branch you can see; nothing relocates here.
It is cited for the sentence and not for the mechanism.

### c. Two names that differ by one letter

**Not an alternative so much as an objection, and it is a fair one.** `.kset` and `.kbset` differ by
a single character in the middle, which is the hardest place to see a difference. In a listing
sorted by name they interleave; in a hurry, at a gig, `night-b.kset` and `night-b.kbset` are the
same word. Names further apart were available — `.kset` and `.kbundle`, or an authoring form spelled
`.kset.src` — and would be read correctly at a glance.

**It was weighed and accepted.** The `b` is the property: a `.kbset` is a Set that has been *bound*,
and the pair reads as one word with a mark on it rather than as two unrelated words, which is what
they are — one file is the other resolved. Against that, the case where a person must tell them
apart by eye is narrower than it looks: the Library's tree shows only the Set extension (ADR-0229,
part 6), a store cannot contain a `.kset` at all, and the two forms sit in the same directory only
in an authoring tree, which is somebody's own working copy. **Where confusing them would cost
something, the program refuses rather than the eye deciding** — that is the whole of what the
extension is for. The cost is real and is paid in `ls`.

## Consequences

- **`morph01` stops being listed, and it is named here rather than discovered.**
  `.karakuri/sets/morph01.set.ndjson` is the one Set in the maintainer's own store, written
  2026-08-20, and after the rename `list_sets` does not claim it: no error, no warning, nothing but
  an id that used to be in the list and is not. **Nothing repairs it and nothing should** — renaming
  a file this store did not write would be guessing that its contents are already resolved, which is
  the one thing the extension exists to stop being a guess. **It was already unloadable for an
  unrelated reason**: its L2 slot is `6a8ab4b…`, a `morph.kir` whose body opens `kind L2 / pairs`,
  and the language has since replaced that with `uses far : Geometry` — `examples/morph.kir` carries
  the new spelling and the store's copy cannot parse. So nothing working is lost. The disappearance
  is silent either way, which is why it belongs in a comment a reader of `list_sets` will meet.
- **Before v1 the bill is stated and is not the argument**
  ([P-0058](../principles/0058-before-v1-compatibility-is-a-bill-not-an-argument.md)). Every
  `.set.ndjson` on any disk anywhere becomes an unlisted file. The honest count of what that breaks
  in the world is one file on one machine, and the honest count of what it breaks in the repository
  is the test fixtures — `crates/karakuri-store/tests/store.rs` alone spelled the suffix twelve
  times, and `karakuri-cli`, `karakuri` and `setfile` each built one by hand rather than through
  `set_path`.
- **The suffix becomes one constant instead of two literals.** `set_path` wrote it and `list_sets`
  stripped it, and two literals could drift into *"a store that writes files it cannot list — and an
  id that exists on disk under one spelling and nowhere in the listing is the worst shape that
  disagreement can take, because neither side is wrong on its own."* Renaming was the moment that
  cost nothing extra to fix, and it is P-0031's ordinary form: one name, one place.
- **This landed while it was being written**, in another session and in one sweep:
  `Store::SET_FILE_SUFFIX` and both doc comments quoted above, the specification's *Set file
  format*, the manual, the console page, the roadmap, the fixtures, and both copies of the P-0048
  table. What survives of the old spelling outside `docs/adr/` is deliberate — the two comments that
  say what the suffix used to be, the roadmap paragraph that says what stops being listed, and the
  test named after that cost.
- **The P-0048 table is written twice and had to change twice**, in `scratch.rs` and in `places.rs`.
  That duplication arrived with ADR-0230 and is not this record's to resolve, but a rename is
  exactly the event that makes a transcribed table cost something.
- **ADR-0229's open item gains a pointer and keeps its reasoning.** That record's *The extension is
  not chosen* bullet is annotated with the record that closed it and is not otherwise touched:
  adding what a record later became is annotation rather than revision, and the test
  ([P-0066](../principles/0066-an-adr-is-a-description-of-history-corrected-but-never-revised.md))
  is *does the edit change what the record says happened, or what a reader can find out about it?*
  Nothing else in `docs/adr/` changes, and every other record keeps its `.set.ndjson` spellings.
- **The Library's tree gets a rule it can state.** ADR-0229's part 6 says only the Set extension
  shows; with two extensions that sentence means *the resolved one shows in a store, and an
  authoring tree shows the other*, rather than needing a rule about which files are hidden.

### What this leaves undone

- **The authoring form does not exist**, and nothing here builds it. `.kset` is a name for a file
  with no reader, no writer and no record type: the `slot` record's `proc` is a content address and
  the format has nothing that spells a relative path. ADR-0229 owns that work and this record only
  fixes what the file will be called.
- **Nothing resolves an include and no wall exists**, and ADR-0229's sentence about the order stands
  unchanged: *"the wall is not a hardening pass to add afterwards, because the first thing that
  resolves an include without one is the defect."* The extension is not that wall — it keeps an
  unresolved file out of the *store*, and says nothing about what an include may point at while it
  is being resolved.
- **The Library does not filter on it.** ADR-0229 part 3 lists both forms; which scope shows which
  extension, and what a folder row does when it meets a `.kset` beside its parts, is the scope's
  question and the scope has no operation yet
  ([ADR-0230](0230-where-the-programs-data-lives-is-told-rather-than-baked.md) leaves *how a
  `--presets` directory is listed* open for the same reason).
- **The store is still flat**, so *only the Set extension shows* has no tree to show it in.
  `Store`'s layout is one directory of `sets/<id>` files and `checked_id` holds an id to one path
  component precisely so it cannot be a path.
- **The mismatch refusal has nothing to refuse yet.** The rule under ADR-0158's shape is written
  above and unimplemented, and it stays that way until an include record exists — at which point it
  is owed in the same change, because a form that can be spelled and cannot be detected is the
  defect the rule describes.
- **Nothing outside this record and the rename is written here.** The specification's *Set file
  format* now says `.kbset`, and what it still does not describe is the authoring form — the debt
  ADR-0229 booked and the one this record does not pay, because a spelling is not a format.
