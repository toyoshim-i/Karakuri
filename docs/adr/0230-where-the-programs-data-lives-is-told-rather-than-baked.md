---
id: 0230
title: Where the program's data lives is told rather than baked, and a default is a search that says which candidate answered
status: accepted
date: 2026-08-30
supersedes: []
superseded_by: []
principles: [0019, 0023, 0027, 0031, 0045, 0048, 0055, 0061]
tags: [cli, distribution, environment, store, presets]
---

# Where the program's data lives is told rather than baked, and a default is a search that says which candidate answered

## Context

**A shipped `karakuri` finds nothing, and the line that decides is one line.**

`Sources::default()` (`crates/karakuri/src/main.rs:2291`) is the whole of what this program plays
when nobody names a pair, and it resolves that pair against the tree the binary was *compiled* in:

```rust
let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
```

`env!` is read at compile time, so the path in a released binary is a directory on the build
machine. It is **the only production line in the workspace that does this** — every other
`env!("CARGO_MANIFEST_DIR")` in `crates/` is inside a `#[cfg(test)]` module or under `tests/`,
where resolving against the build tree is exactly right because the test *is* the build tree.
The store is baked the other way and fails the same distribution: `karakuri-cli`'s `DEFAULT_STORE`
and `karakuri`'s `STORE` are both `.karakuri` relative to the **working directory**, which is a
store wherever the operator happened to be standing.

**The reasoning that put it there was right, and this record does not overturn it.** The doc
comment on `Sources::default()` argues the asymmetry rather than falling into it:

> **The repository's own pair, resolved against the workspace root** — which is what this program
> drew before it took an argument, so a bare `cargo run -p karakuri` behaves as it always did.
>
> Against the workspace root rather than the working directory, and that asymmetry with a typed
> path is on purpose: a default nobody named has to find the file wherever the run was started
> from, and a path an operator *typed* is theirs and is read from where they typed it.

Every clause of that is true of the development loop, and the development loop is the only loop
there is — [the README](../../README.md) offers `cargo run` and nothing else. What the comment is
silent about is the second machine. So the compile-time path is not a mistake to delete; it is **an
answer to one case that was left standing as the answer to all of them**, and it survives this
record as the last entry of the table in D2.

**The Library's `presets` scope is the symptom, and it is a tier with no file in it.**
[ADR-0229](0229-a-set-file-is-authored-beside-its-parts-and-travels-as-a-bundle.md) established
that `examples/` holds parts and no Set, and it named this line among what it left undone —
*"`Sources::default()` resolves `examples/` against `env!("CARGO_MANIFEST_DIR")`, baked at compile
time to the build machine's tree … **A shipped binary would find no `examples/` at all today**, for
`.kir` as much as for a Set."* On [the console page](../manual/console.html) the scope has been
drawn since the mock was written and **carried no `data-tip` and no specification** — `favourites`,
`folder` and `+` each explain themselves at length, and this one said nothing, because there was
nothing to say. (The page has since gained one, and it states this record: where the files are is
*"told to the program when it starts rather than compiled into it — a program has no reliable way to
ask where it is"*.) That gap widened today rather than narrowed: `fd6ec66` shipped nine more `.kir`
into `examples/` — *"the panel can load material and there was none"* — all of them unreachable from
anything but a source checkout.

### The maintainer's framing, which is the decision in outline

On 2026-08-30:

> posixでは自分のいる絶対パスは知ることができないのでは？引数でプリセットパスを渡せるようにするのかな。
> インストールされる時にはプラットフォーム固有の方法でインストール場所からの相対で決めるんだと思うけど。

Three things in two sentences: the program cannot simply *know* where it is; an argument is how it
is told; and an install decides by platform convention relative to where it was installed. The
first is the reason the flag is the authority and the search is only a default.

### The debt this closes on the way past, and the blocker it names is the wrong one

`karakuri`'s `const STORE` (`crates/karakuri/src/main.rs:3300`) carries a doc comment that has been
waiting on something that was never actually in the way. In full, the part that waits:

> # Still transcribed, and this is the one thing ADR-0214 said would go and has not
>
> [ADR-0214](0214-the-program-moves-out-of-the-cli-and-two-thin-binaries-sit-over-it.md)
> lists this constant as one of two transcriptions the move *deletes rather than carries*:
> *"`STORE` asks for `DEFAULT_STORE`, and the residency decode calls
> `mix::parse_residency`."* The residency half is done — `apply` calls
> `karakuri_environment::mix::parse_residency` now, and that function is `pub` for this caller.
>
> **This half cannot be done yet, and the reason is which slice moved.** `DEFAULT_STORE` is at
> `karakuri-cli/src/main.rs:356` and is private. ADR-0214's boundary section says `main.rs` is
> *"7,688 non-test lines holding both and will not divide along a line anyone can name today"*, and
> it is the one module of the fourteen that has not moved: `karakuri-cli/src/` is that file and
> nothing else. Until it divides there is nothing to ask, so the two are the same directory on
> purpose and the day one moves this program lists an empty library rather than the wrong one —
> which is the sentence this comment has carried since it was written, still true and now with the
> blocker named rather than guessed at.

**The blocker is named precisely and it is the wrong blocker.** Nothing has to divide.
`karakuri-cli/src/main.rs` does not have to become a library for the two programs to share a
string, because there is already a package **both** of them depend on, and
[ADR-0215](0215-the-package-is-karakuri-environment-and-a-module-belongs-if-what-it-deals-with-is-outside-this-process.md)'s
test says the string belongs in it: *"A module belongs in it if what it deals with lives outside
this process — a disk, a device, a port, a socket, another process — or is the record of what
happened."* A directory on a disk is outside this process. The comment looked at the crate the
constant came *from* and asked when that would move; the answer was in the crate they both already
reach.

The dates are worth keeping, because they say what kind of debt this is. `.karakuri` has been
`karakuri-cli`'s default since `a11c4c1` on **2026-08-03**. It was transcribed on **2026-08-27**
(`2867bf6`), into what was then `karakuri-console/examples/panel.rs`, already carrying the note
*"there is nothing to ask… the day one moves this example lists an empty library rather than the
wrong one."* ADR-0214 booked it as a transcription to delete on **2026-08-28**, the example became
this program, and the note came with it and grew. **Four weeks as a duplicated string, three days
as a duplication with a comment explaining why it had to stay** — and it never had to.

## Decision

The maintainer's, on 2026-08-30, in five parts.

**1. The two roots belong to `karakuri-environment`, and each binary keeps its own parser.** A new
module there owns the constants, the resolution and the refusal sentences; `karakuri` and
`karakuri-cli` call into it and each keeps the command line it has. **This deletes two
transcriptions of one string** — `karakuri-cli`'s private `DEFAULT_STORE`
(`crates/karakuri-cli/src/main.rs:365`) and `karakuri`'s `const STORE`, doc comment and all. The
refusal sentences go with them for
[P-0061](../principles/0061-a-refusal-a-person-can-reach-from-two-surfaces-is-one-sentence.md)'s
reason: *a mistake that can be made from more than one surface is refused in one sentence, from one
function*, and a mistyped path is now a mistake two programs can make. What does **not** move is
either parser. `karakuri-cli` has thirty-odd flags and `karakuri` takes two positional paths, and
the sentence that keeps them apart is quoted under D5.

**2. `--presets DIR` names the shipped library, and the default is a search with a table.** Given,
that directory *is* it: a `--presets` naming a directory that does not exist is **refused, naming
the flag**, exactly as `karakuri-cli` already refuses a missing `--store` — *"no store at `{}` —
nothing has ever been kept there, and nothing was created to find that out. Check `--store`"*
(`listed_sets_at`, `crates/karakuri-cli/src/main.rs:2546`). A path an operator typed is theirs;
quietly using something else instead is
[P-0027](../principles/0027-a-silently-wrong-image-loses-to-a-loud-failure.md)'s failure exactly —
*a silently wrong image loses to a loud failure* — and the wrong presets directory is a silently
wrong picture with a plausible one on screen.

Not given, the candidates are derived from `std::env::current_exe()` and the **first that is a
library wins** — a directory with at least one `.kir` in it, which is not the same test as existing:

| | Candidate | What it is |
| --- | --- | --- |
| 1 | `<exe dir>/../Resources/examples` | a macOS `.app` bundle |
| 2 | `<exe dir>/../share/karakuri/examples` | a unix prefix install |
| 3 | `<exe dir>/examples` | a portable directory |
| 4 | the workspace tree baked at compile time | **the development entry** |

**Entry 4 is the line this record opened with, kept and demoted.** It is what keeps
`cargo run -p karakuri` behaving as it always did, it is last, it is tested like the other three,
and a released build made on somebody else's machine simply fails it and moves on. It is named in
the table as the development entry rather than hidden inside the fallback, because a candidate a
reader cannot see is a candidate they will be surprised by.

**And that sentence was false while the test was existence, which is why the test is not
existence.** A cargo workspace builds its example binaries into `<target>/<profile>/examples`, and
a debug build of the panel sits in `<target>/<profile>` — so candidate 3 *exists* for every
`cargo run` in this repository, holds four hundred object files and no presets, and would win over
entry 4 every time. The program would resolve a presets root, print it, and then fail to open
`drift_shell.kir` inside it: a silently wrong answer followed by a puzzle, which is P-0027 again
and one candidate lower down the same table. Asking for one `.kir` costs one directory read per
candidate and is the only thing that tells two directories sharing a name apart. **A typed
`--presets` is still plain existence** — an empty library an operator named is an empty library
they named, and this program is not entitled to decide they meant somewhere else.

If none of the four answers there is **no preset library**: the program says so once, out loud, naming
`--presets`. It creates nothing and pretends nothing.

**And which candidate answered is printed at startup, derived and never transcribed.** On this same
day `e03332e` — *"Make the program's account of itself derived, because it was lying five ways"* —
found the startup legend describing a program this one no longer is, and the replacement sentences
carry their own epitaph: *"the sentence this replaces said every body was empty but the Program
bay's two, and it went on saying it while bay after bay drew one — which is what a description kept
beside the thing it describes is worth"*, and *"a copy here is precisely how this legend came to
name three controls while every one of them answered a press."* A line saying *presets: …* that a
person maintains would be the sixth
([P-0045](../principles/0045-generate-the-vocabulary-prose-drifts-from-code.md)). It prints the path
the resolution returned, and it prints which entry returned it, because *where did that come from*
is the whole question this record exists to answer.

**3. The launch pair follows the presets root.** `Sources::default()` becomes
`<presets>/drift_shell.kir` + `<presets>/soft_points.kir` — the same pair, resolved against a root
that was found rather than baked. **With no presets root there is no default pair**, and the program
refuses, naming `--presets` and saying the two paths may be given positionally instead. A black
window with a working process behind it is the failure P-0027 rules out; a refusal that names both
ways forward is the one it asks for.

**4. `karakuri` gains `--store DIR`, defaulting to the shared constant**, and its `STORE` const goes
with it. The Library bay lists a store it could not be told about, which is the smaller half of the
same defect: the program has an opinion about where the operator's Sets are and no way to be told
otherwise.

**5. Two flags are not a second material vocabulary, and the doc that argues against flags is
rewritten rather than left standing.** The `Sources` doc (`crates/karakuri/src/main.rs:2269`) is
where the parsers were held apart:

> **Deliberately not `karakuri-cli`'s parser.** That program has thirty-odd flags, `--set
> a.kir,b.kir` among them, and they live in its own `main.rs` where nothing else can reach them. A
> second `--flag` vocabulary here would be a second answer to *how does an operator name material*,
> which is the failure this whole move exists to stop paying for
> ([P-0031](../principles/0031-a-name-means-one-thing-across-the-system.md)). So this is two
> positional paths and nothing else … **The day the two programs share one, it comes from a package
> both can reach and this goes.**

That paragraph is not contradicted by `--presets` and `--store`, and the distinction is the one
P-0031 turns on. *How does an operator name material* keeps exactly one answer — **the positional
pair, and nothing else** — and these two flags answer a different question, *where does this
program's data live*, which had no answer at all. The paragraph gets that sentence added to it, on
[P-0023](../principles/0023-a-document-that-describes-replaced-behaviour-is-worse-than-none.md)'s
terms: the change that makes a comment look wrong is the change that owes the correction, and a
reader who finds *no vocabulary to disagree with* beside two new flags will either delete the flags
or stop believing the comment. Note also what its last sentence predicted — *the day the two
programs share one, it comes from a package both can reach* — which is D1, arriving for the roots
rather than for the parsers.

## Alternatives

### a. An environment variable

`KARAKURI_PRESETS`, and no flag. It is the shape every reader has seen, it needs no parser change in
either program, and it is the one mechanism that can be set once for a whole machine — which is
precisely what an installer wants.

**It loses on being a second answer to a question that now has one.** A flag and a variable both say
where the presets are, and the two disagree the moment somebody sets both; whichever way that
precedence is resolved, the rule lives in a paragraph rather than in a sentence an operator can
read, which is
[P-0031](../principles/0031-a-name-means-one-thing-across-the-system.md)'s failure in its ordinary
form. The search table is not a second answer in the same way, because it is *the default* — what
happens when nobody has said anything — and it says out loud which entry answered.

**And it is invisible in the sentence an operator types.** `karakuri --presets ~/kit` is a command
somebody can paste into a message and somebody else can run; `karakuri` with an exported variable
three shells up is a command that behaves differently on two machines with nothing on screen to say
why. This workspace reads exactly one environment variable, `WGPU_BACKEND` in
`crates/karakuri/src/main.rs:1117`, it is **wgpu's own rather than this program's**, and the only
thing this program does with it is explain a failure — *"WGPU_BACKEND is set to `{want}`, and this
codebase honours it — so the most likely reason is that this machine has no {want}"*. That is a
variable being read to say *why*, which is the opposite of one being read to decide.

### b. `current_exe()` alone, with no flag

The search table without the flag over it. It is what an installed program actually needs, it needs
no argument, and every case a packager cares about is in the table already — so the flag looks like
ceremony.

**A relocatable bundle is exactly the case the search handles, and a symlinked or repackaged install
is exactly the case it gets wrong**, which is why one cannot stand without the other. `current_exe`
on a symlinked binary may hand back the link's path or the target's depending on the platform, a
package manager may put the binary and its data under different prefixes, and a Homebrew-style
`bin/` full of symlinks into cellars is a normal installation rather than an exotic one. The program
would then look in a directory that has nothing to do with where its data was installed, find
nothing, and report *no preset library* on a machine where the library is right there.

The maintainer's own first sentence is the argument in its general form —
*posixでは自分のいる絶対パスは知ることができないのでは* — a process cannot reliably know where it
is. **So the flag is the authority and the search is only the default**: the search is what makes
the common installs work with nothing typed, and the flag is what makes every other install *possible*, which is the half
that cannot be recovered afterwards.

### c. Bake the path at build time, with `option_env!`

`option_env!("KARAKURI_PRESETS_DIR")` read at compile time, so a packager sets one variable in the
build and the binary knows its own install prefix. This is what a great many C programs do with
`-DDATADIR=`, and it is the smallest change of the four: one constant, no runtime search, no flag.

**It loses because a build machine's path is not a user machine's**, which is
[P-0055](../principles/0055-this-machine-is-not-the-reference.md) stated about a directory instead
of about VRAM: *using the development machine as evidence* is the thing that principle rules out,
and a path compiled in is that mistake in its most literal form. It is also, exactly, the defect
this record was opened to fix — `env!("CARGO_MANIFEST_DIR")` **is** a path baked at build time, and
`option_env!` differs only in who chose the string.

**And it makes a relocatable bundle non-relocatable.** A `.app` a person drags from a disk image to
`/Applications`, or a portable directory copied onto a stick, is the case candidates 1 and 3 exist
for, and every one of them moves after the build. A compiled-in absolute path is a bundle that
works until it is moved and then fails without saying it was moved. The relationship a program can
rely on is the one *inside* the install — `<exe dir>/../Resources` stays true wherever the bundle
goes — and that is a runtime derivation by construction.

### d. Create the presets directory when none is found

Instead of reporting *no preset library*, `create_dir_all` the best candidate and carry on. It
removes an error path, the Library's `presets` scope draws an empty list instead of a refusal, and
whatever a person drops in afterwards is picked up.

**A program that creates an empty library has answered a question nobody asked**
([P-0019](../principles/0019-prefer-the-mechanism-that-already-exists.md) is where that test is
written down), and it destroys the one piece of information the failure carried: *nothing was
installed here* and *nothing has been put here yet* become the same empty directory, and the first
of the two is very often a mistyped `--presets`. This workspace has already decided this twice, in
both directions of the same rule. `listed_sets_at` will not open a store to report that one is
missing — *"a flag whose whole promise is that it only reads must not leave a directory behind to
say that a library is empty. A path with no store at it is an answer, and it is a different one from
a store with no sets in it — the first is very often a mistyped `--store`."* And
[`scratch.rs`](../../crates/karakuri-environment/src/scratch.rs) refuses the same side effect for
the same reason on the write side: *"An offscreen render never edits anything, and creating a
directory as a side effect of `--render` would make a pure function of its arguments into one that
leaves a mark."*

**The presets tier is the one place where it would be worse than either.** Under
[P-0048](../principles/0048-what-ships-what-you-saved-and-what-you-are-editing-are-three-places.md)
`examples/` is *"app presets | nobody writes it"* — the only one of the three places with no writer
at all. A program that creates its own preset directory has made itself that writer, on the tier
whose whole definition is that it ships and is never written.

## Consequences

- **`karakuri` is installable, and that is what changes.** With the roots told rather than baked,
  the binary and its data can be laid out by whatever the platform does, and the program finds them
  with nothing typed. Nothing here *packages* anything — there is still no bundle, no `make
  install`, no `.app` — but candidates 1 through 3 are the three shapes such a step would produce,
  and the shape of the target is now decided rather than discovered by whoever attempts it.
- **`.karakuri` exists once in the workspace.** Two `const`s become one, in the package both
  binaries reach, and the day one program's default store changes it changes for the other or the
  change is a deliberate divergence somebody wrote. The doc comment that explained why the
  duplication had to stay goes with the duplication — leaving it beside a constant that is no longer
  transcribed is P-0023's defect exactly.
- **A refusal becomes reachable from two programs, so it is one sentence.** `--store` was
  `karakuri-cli`'s alone and its refusals were written where they were used. With `karakuri` taking
  the same flag, P-0061 applies from the moment the second caller exists rather than after somebody
  notices two spellings, and the module owning the constant owns the sentence.
- **The startup account gets one more derived line and no more prose.** Which candidate answered is
  a fact the resolution already holds; printing it costs a field and nothing may transcribe it.
- **The development loop is unchanged, deliberately.** `cargo run -p karakuri` plays
  `drift_shell` + `soft_points` from the workspace's `examples/`, exactly as it has since before it
  took an argument. The difference is that it does so by *failing three candidates and matching the
  fourth*, which the startup line will say.

### What this leaves undone

- **The Set file's two forms are ADR-0229's and are the next piece of work.** That record decided an
  authoring form naming its parts by relative path and a bundle that inlines them, and left the
  extension unchosen, the include resolution unwritten and the containment wall unbuilt — *"Both are
  owed together: the wall is not a hardening pass to add afterwards, because the first thing that
  resolves an include without one is the defect."* **This record makes `examples/` reachable; it
  does not put a Set in it.** The `presets` scope stays empty until the authoring form exists, and
  what changes here is that it is empty for one reason instead of two.
- **`karakuri-cli` gets the shared constants and no `--presets` flag.** Nothing in it reads a preset
  by name — it takes `.kir` paths and store ids — so the flag would name a directory the program
  never consults. A flag with nothing behind it is a vocabulary that means nothing, which is the
  same count P-0031 charges against a second spelling.
- **The operation that switches a library scope is still owed, and it has no row.** `grep -i scope
  docs/manual/operations.html` finds nothing: the console page draws `favourites`, `my sets`,
  `presets`, `folder` and `+`, and every tooltip in that row that mentions MIDI says the same thing
  — *"unassigned — there is no operation for a map to name yet"*. So *look at the presets instead of
  my sets* is not a control this decision makes reachable; it is a row somebody has to write on the
  operations page first
  ([ADR-0226](0226-m5-closes-when-the-manual-is-implemented-and-the-meter-is-progress-rather-than-completion.md)
  is why that order and not the other).
- **The store is still flat, and the virtual sibling folder still has no path on either side.**
  ADR-0229's part six wants presets to appear as a sibling of the operator's own Sets in one tree.
  This record supplies the missing half it named — *"The virtual sibling folder needs a path this
  program does not have on either side of the seam"* — and nothing else about it: `Store`'s layout
  is one flat `sets/<id>.set.ndjson`, and a tree changes what an id is.
- **Nothing here decides how a `--presets` directory is listed.** The bay reads `Store::list_sets()`
  and a presets root is not a store; whether the scope reads a directory, or a store is opened over
  it, is the first question whoever implements the scope will meet.
