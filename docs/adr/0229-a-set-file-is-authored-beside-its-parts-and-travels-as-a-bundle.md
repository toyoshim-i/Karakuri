---
id: 0229
title: A Set file is authored beside its parts and travels as a bundle, never by a name a search path resolves
status: accepted
date: 2026-08-30
supersedes: []
superseded_by: []
principles: [0096, 0085, 0086, 0090, 0092]
tags: [store, format, console, distribution, security]
---

# A Set file is authored beside its parts and travels as a bundle, never by a name a search path resolves

## Context

**The Library's `presets` scope has nothing to list, and that is not a drawing that got ahead of
the code — it is a tier with no file in it.**

[ADR-0227](0227-a-pattern-and-a-master-chain-setting-are-library-data-in-two-tiers.md) records the
shape this application keeps its data in: presets ship with the program and the operator's own live
under the store, and
P-0048
names the three places —
[`crates/karakuri-environment/src/scratch.rs`](../../crates/karakuri-environment/src/scratch.rs)'s
header table is the same three rows, *"app presets | `examples/` | nobody — they ship with the
program"*, *"user presets | `<store>/sets/<id>.set.ndjson`"*, and the scratch. For a `.kir` both
tiers exist. **For a Set, only the second one does.**

### What is actually in each of the two places

**`examples/` holds thirty-one `.kir` files and one `surface.map`, and not one Set file.** The map
is the precedent ADR-0227 found for a preset that is a setting rather than a procedure, and it is
still the only one. So the app-preset tier of the Library is a directory of *parts*, and
[the console page](../manual/console.html) already ruled that out as a library listing, by name and
before this question was asked: *"A directory of `.kir` files is a directory of parts, and a
library lists what you can put on a deck."*

**A fresh clone's Library is empty.** `crates/karakuri/src/main.rs`'s `library` lists
`Store::list_sets()` and nothing else, so the bay draws exactly `<store>/sets/`; `.karakuri/` is in
[`.gitignore`](../../.gitignore), and `git ls-files` finds no `*.set.ndjson` anywhere in the
repository. The one Set this maintainer's working copy holds — `morph01`, written 2026-08-20,
carrying no `edge` and no `merge` because neither record existed yet — is his machine's and not the
program's. **The empty bay is what everybody else gets**, which is a stronger version of the
problem than the one this record was asked to describe.

### The pairing exists. It is written down everywhere except in a file the program can read

This is the part worth establishing before any option is weighed, because it is what makes the
authoring form a *format* question rather than a *content* question. Nothing in `examples/` says
which of its parts make a Set — and the pairings are nonetheless known, hand-maintained, and
tested:

- [`crates/karakuri-cli/tests/examples.rs`](../../crates/karakuri-cli/tests/examples.rs) holds nine
  documented pairs and seven chains, three of the chains carrying an `--edge`, and its own header
  says why it is a list by hand: *"Which L1 goes with which L4 is written in prose and in the files'
  own comments, never in the files, so the pair list below is by hand. Deriving one would be this
  test inventing an answer to a question the language has not asked."*
- `Sources::default()` (`crates/karakuri/src/main.rs`) is one pairing —
  `examples/drift_shell.kir` with `examples/soft_points.kir` — as two `PathBuf` fields.
- **A part may not close the gap itself.**
  [P-0086](../principles/0086-a-procedure-knows-only-what-it-declares.md) is the rule: *"a part that names
  the parts around it is bound to one Set and stops being a library part. A corpus of procedures
  that cannot be recombined is not a corpus."* So the pairing has to live in a file that is not a
  `.kir`, and the only such file the format has is a Set file.

### What a Set file already is, and what a bundle already is

[`docs/ir-spec.md`](../ir-spec.md)'s *Set file format* section: `.set.ndjson`, one record per line,
`proc` a hash reference into the store. **Bundling is not new.** The same section:

> When bundled, the source is inlined as a run of `{"t":"src","hash":"…","line":0,"s":"…"}`
> records — **one run per artifact**, however many `slot` records reference it, because a reader
> keys `src` by hash.

`--bundle ID` and `--unbundle FILE` are in `crates/karakuri-cli/src/main.rs` and implemented in
`crates/karakuri-environment/src/setfile.rs` (`bundle`, `with_inlined_source`, `unbundle`).
[operations.html](../manual/operations.html) carries the row — *Send a Set to somebody, and take
one in* — with `CLI --bundle, --unbundle` **has** and `panel library` **plan**.

**And the console page has already decided that a bundle is not a separate kind of row.** Its note
*A folder scope reads Sets, and a bundle is not a third thing*:

> A Set file and a bundle are the same file — one record per line, and a bundle is that file with
> the sources it names appended inside it — so the scope draws one kind of row rather than two.

**So what changes here is not the bundle's existence.** It is that the bundle stops being an
*export* format reached by a flag and becomes the form the Library lists and ships.

## Decision

The maintainer's, on 2026-08-30, in six parts.

**1. A Set file has two forms.** An **authoring** form that names its `.kir` by **relative path**
and lives beside the parts it names — which is what `examples/` needs, since it holds only parts and
nothing there says which of them make a Set. And a **bundle**, which resolves those and inlines the
sources, self-contained.

**2. Sharing a `.kir` between Sets stops being a goal.** His words: *"or do we stop them being
shared? Just bundle into one set.ndjson."* A part may be named by more than one authoring file, but
a bundle carries its own copy. Kilobytes, accepted outright.

**3. The Library lists both forms.** *"Showing the set does no harm. But when you Load it, it
becomes a bundle and is stored."*

**4. So loading an authoring file *is* packaging it**, performed at that moment; and packaging for
distribution is the same resolution done ahead of time. His words: *"what I am calling packaging is
the work of compiling the kir and making the bundle."* One operation, two moments.

**5. The session stream keeps content addressing, and this decision does not touch it.**
`ir-spec.md` says why the stream needs what a Set file is giving up: a `procedure` record's `proc`
is a content address and the source lives in the store, *"so a rewrite costs one line here and a
few kilobytes once, however many times the same procedure comes back."* A night of rewrites inlined
whole would bloat a recording, and a session is written per swap rather than per file. **The two
files answer different questions and are allowed different answers** —
[ADR-0007](0007-a-set-file-is-a-projection-and-a-session-stream-is-the-timeline.md), *a Set file is
a state projection with no time in it; a session stream is the timeline.*

**6. The store gains hierarchy and the Library is a tree.** Presets ship inside the platform's
application bundle and appear as a **virtual sibling folder**: where the bytes physically live is
independent of where they appear. Only the Set extension shows; directories are visible as
structure.

**The mock has drawn part six for as long as it has drawn the bay.** The Library's head is a scope
row over a **breadcrumb** — `~/sets/tour-2026/night-b › opening` — which is a path two levels deep
and a leaf. What was never decided was whether that was a folder scope's browsing of somebody
else's disk or the store's own shape. It is the store's own shape.

## Alternatives

### a. A `.kir` named by filename, with a search path in the Set file

**The one worth recording**, because he proposed it first and withdrew it, and because it is the
shape anyone reaching for "don't duplicate the sources" arrives at: a `slot` record naming
`drift_shell.kir`, and a search path in the file — or in the store, or in a setting — that says
where to look. It is smaller than a bundle, it keeps one copy of a shared part, and it reads like
every include mechanism anyone has used.

**It loses because it reintroduces exactly what content addressing exists to prevent.**
`ir-spec.md` states the hole, about the one place the format lets source text in:

> **`--unbundle FILE` reads one back into a store**, and what it guarantees is that check: every
> inlined run must hash to the address its `slot` record names, or the file is refused with nothing
> stored. Without it a `src` run would be a way to file arbitrary text under an address the
> receiving operator recognises, which is the one thing content addressing is for.

`setfile::unbundle` is where that is enforced — every inlined run is re-hashed and a mismatch
refuses the whole file with nothing stored — and a `slot` naming an artifact that is neither
inlined nor already present is refused too, *"so the file is not self-contained and nothing was
stored"*.

**A bare filename resolved through a search path is that same hole with a friendlier name.** The
receiving operator recognises `drift_shell` — that is the entire point of a shared part — and what
they get is whichever `drift_shell.kir` the search path finds first. Nothing hashes, nothing is
compared, and the file that decides is the one *not* in front of them. Shadowing is not an abuse of
the mechanism; it is the mechanism.

**And bundling is stronger on this axis rather than weaker, which is the sentence to keep.** The
intuition is that inlining source into a shipped file is the looser thing to do. It is the
opposite: **there is no address to spoof because there is no reference.** A bundle carries bytes and
`unbundle` hashes them; a searched name carries a claim about somebody else's disk and nothing
hashes anything.

**Two smaller counts against it.** It hands the Library a row that cannot be listed honestly — a
Set whose parts are somewhere else is a Set whose readiness depends on a directory the listing did
not read. And it makes a `.kir` reachable by a name a Set chose, which is one letter away from what
[P-0086](../principles/0086-a-procedure-knows-only-what-it-declares.md) forbids in the other direction.

**What it costs to lose it, honestly.** Duplication on the wire and in `examples/` — one copy of a
shared part per Set that names it. It is smaller than it sounds twice over: within one file the
format already collapses it (*"one run per artifact, however many `slot` records reference it"*),
and on arrival `Store::put_artifact` files by content address and *"the artifact already on disk is
never rewritten"*, so two bundles carrying the same procedure converge to one blob in the receiving
store. **The duplication is in transit only.**

### b. Presets as pairings in code

Also plausible, and it already ships: `Sources::default()` is a preset Set expressed as two
`PathBuf`s, and `crates/karakuri-cli/tests/examples.rs` is nine more plus seven chains. Extending
that — a table of pairings the program knows — needs no format, no file, no wall and no packaging
step.

**It loses because a pairing is not a Set.** A Set file is a state projection: capacities,
parameter values, bindings, seeds, the camera, the merge, the edges, and a name a person chose.
`Sources::default()` carries none of those; it carries two paths, and `Sources::material()` derives
a display name by joining two file stems. Ship a "preset" of that kind and what an operator loads is
not what its author was looking at, which is the whole difference between a preset and a suggestion.

**And it cannot be listed.** `library` reads `Store::list_sets()`; a pairing in a `const` is not in
a store, has no id, no time for the column beside the name, and nothing for the star to be kept
beside. It is the same refusal ADR-0227 gave the arrangement's preset tier for the opposite
reason — an arrangement's default *is* code and there is deliberately no file — and it does not
transfer, because a Set file's whole job is to be the thing a listing can name.

## The wall the authoring form needs

**A relative include is a path-traversal surface**, and it is the one new attack the authoring form
opens: an include that escapes its own file's directory means opening a Set somebody sent you reads
any file on that machine and inlines it into a bundle you then hand on.

**The precedent is `checked_name`** (`crates/karakuri/src/main.rs`), the wall an operator's typed
arrangement name meets. Its own documentation is the argument, and the reasoning transfers whole:

> `<name>` becomes one path component under `<store>/arrangements/`, and `karakuri-store` says
> outright that **nothing there checks it** […] So `../../elsewhere` is a path, and a path never
> reaches that call from here.

It is the surface's job rather than the store's —
[P-0090](../principles/0090-a-surface-offers-it-never-decides.md), *a surface owns
the affordance and never the authority* — and it is **refused out loud rather than sanitised**,
because a name quietly repaired is a rule an operator can only find by experiment.
`mcp::checked_id` (`crates/karakuri-environment/src/mcp.rs`) is the same wall on the other surface,
and `named_set`'s comment there is the sentence that applies most directly to an include: *"A read
is not the harmless half of that rule: it is the half that hands a file's contents back to the
caller."*

**Two things about the precedent are not identical, and glossing them would mislead whoever builds
this.** `checked_name` and `checked_id` allow letters, digits, `-` and `_` and nothing else, because
what they guard is **one path component**. An include is a *relative path* and has separators in it
by construction, so the rule cannot be copied — what transfers is where the wall sits, that it
refuses rather than repairs, and that the refusal names what it refused. The rule itself is
containment: the include, resolved, is under the authoring file's own directory, and a symlink out
is the same escape by another spelling.

**`checked_name`'s own documentation asks to be told when this happens.** It says the two walls
*"collapse into one shared `checked_name`"* when a name gets a second surface, and *"this comment is
where whoever does it should start."* An include is not a *name* and will not collapse into that
function, but it is the third member of the family and belongs beside the other two.

**A bundle needs no such wall at all**, which is one more point in its favour and the reason the
wall is only ever the authoring form's problem: a bundle references nothing outside itself, so
there is no path to contain.

## Consequences

- **Loading a preset writes into the operator's own library, and this was not decided so much as
  implied.** Part 3 lists both forms; part 4 makes a load a packaging step that stores the bundle.
  Put together: opening a shipped Set adds an entry to `<store>/sets/`. **That is probably right** —
  it is how *making it yours* works, it is copy-on-load exactly as `scratch.rs` already performs it
  for material (*"a run that can be edited copies its material here first and runs from the copy"*),
  and it is what keeps `examples/` unwritten under P-0048. **It is stated here so nobody meets it as
  a surprise**, because the first symptom is a `my sets` that fills up with things the operator did
  not make.
- **The Library gets a second kind of row, and the console page's argument survives it.** That page
  settled *"two of the three are one thing and the third is out"* over a directory of `.kir`, a
  directory of Set files, and a bundle. An authoring Set file is a fourth candidate it did not
  weigh — and it lands on the same side: it is a Set file, it is one row, and the difference from a
  bundle is *"a property of a file, not a kind of collection."*
- **`--bundle` stops being only an export.** It is the packaging step, and the thing `examples/`
  would ship is its output. What is missing is the other half — resolving an authoring file's
  relative includes into the store before inlining — which today has no code at all: `bundle` starts
  from `store.read_set(id)`, so it can only bundle what is already in a store under an id.
- **`--unbundle`'s refusals become the load path's refusals.** An id already held is refused rather
  than overwritten (*"the id came from the file rather than from you"*), and a mis-hashed run
  refuses the whole file. A load that is a packaging step meets both, and neither is new prose.
- **The store's own path is not platform-conventional either.** `DEFAULT_STORE` is `.karakuri` in
  the working tree, deliberately — *"a session's material belongs beside the session"* — and
  `crates/karakuri/src/main.rs`'s `STORE` is the same string, private-duplicated, with a comment
  naming ADR-0214's undivided `karakuri-cli/src/main.rs` as the blocker. **The virtual sibling
  folder needs a path this program does not have on either side of the seam.**

### What this leaves undone

- **The extension is not chosen.** A bundle and a store Set are `.set.ndjson`; whether an authoring
  file shares it or takes one of its own is open, and part 6 (*only the Set extension shows*) makes
  it load-bearing for the tree rather than cosmetic. **Chosen on 2026-08-30 in
  [ADR-0231](0231-a-sets-two-forms-take-two-extensions-and-the-store-holds-only-the-resolved-one.md):
  `.kset` for the authoring form, `.kbset` for the store's.**
- **There is no packaging step and no shipped application.** `cargo run` is the only way in
  ([README](../../README.md)), and the app-preset tier is *already* unreachable outside a source
  checkout: `Sources::default()` resolves `examples/` against `env!("CARGO_MANIFEST_DIR")`, baked at
  compile time to the build machine's tree, and `karakuri-cli`'s defaults are relative paths that
  need the working directory to be the workspace root. **A shipped binary would find no `examples/`
  at all today**, for `.kir` as much as for a Set.
- **The store has no hierarchy.** `Store`'s layout is one flat `sets/<id>.set.ndjson`, `list_sets`
  reads that directory, and `checked_id` holds an id to one path component precisely so it cannot be
  a path. A tree changes what an id *is*, and that is a decision of its own.
- **Nothing resolves a relative include, and no wall exists.** Both are owed together: the wall is
  not a hardening pass to add afterwards, because the first thing that resolves an include without
  one is the defect.
- **No operation and no route is decided here.** *Send a Set to somebody, and take one in* already
  names both directions and is `plan` on the panel; whether packaging is a third row or the same one
  performed at a different moment is the operations page's to say, before it is built
  ([ADR-0226](0226-m5-closes-when-the-manual-is-implemented-and-the-meter-is-progress-rather-than-completion.md)).
- **`docs/adr/INDEX.md` owes a row for this record**, and
  [ir-spec.md](../ir-spec.md)'s *Set file format* section owes the authoring form; neither is
  written here because those files are held by another session in flight.
