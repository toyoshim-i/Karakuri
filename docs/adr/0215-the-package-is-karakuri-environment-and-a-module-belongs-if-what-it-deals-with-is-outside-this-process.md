---
id: 0215
title: The package is karakuri-environment, and a module belongs in it if what it deals with is outside this process
status: accepted
date: 2026-08-28
supersedes: []
superseded_by: []
principles: [0013, 0031, 0060, 0066, 0085]
tags: [architecture, vocabulary]
---

# The package is `karakuri-environment`, and a module belongs in it if what it deals with is outside this process

## Context

[ADR-0214](0214-the-program-moves-out-of-the-cli-and-two-thin-binaries-sit-over-it.md) decided that
the program moves out of `karakuri-cli` into a shared package with two thin binaries over it, and
closed by naming three things it deliberately did not settle: **the package's name**, **its
boundary**, and **whether the move lands in one commit or several**. This record closes the first
two. The third is still open and nothing here forecloses it.

They are one record rather than two because of the order in which they actually resolved. The name
did not win a beauty contest against four other nouns; it won because it is the only candidate that
names the **property** the package has rather than a position it occupies or a relationship it
stands in — which is
[P-0060](../principles/0060-name-the-property-not-the-shape.md) — and **a name that names a property
can be used to decide placement.** The boundary is not a second decision taken beside the name; it is
what the name is for, and it is what makes the name worth a record at all. The alternative that was
standing when this session opened, `karakuri-core`, cannot do that work: nothing about the word
*core* answers *does `midi.rs` belong in it*, because a position on a dependency graph is not a
property a module either has or does not have.

## Decision

**Two things, taken together.**

**1. The package is named `karakuri-environment`.**

**2. A module belongs in it if what it deals with lives outside this process — a disk, a device, a
port, a socket, another process — or is the record of what happened.**

The second sentence is the test. It is applied below to the thirteen modules ADR-0214 put in scope,
and to the two types that record refused to guess at.

## The name is free, and the alternatives are not

Counted on 2026-08-28 across every tracked file.

**`environment` occurs twelve times in the whole repository, and not once in any crate's `src/`.**
Two in `docs/contributing.md` (*"Environment Setup & Development Tools"*, *"GPU Driver & WebGPU
Environment"*), one in `docs/ir-spec.md` (*"a replay environment is the operator's responsibility"*),
one in `docs/roadmap.md` (*"not primarily a live-coding environment for humans"*), six across five
ADRs — four of which are the phrase *environment variable* and one *an environment fact* — and two in
code, both outside `src/`: `karakuri-console/examples/panel.rs:474` and
`karakuri-engine/tests/probe.rs:29`. Every one is ordinary English. **The word is not a defined noun
naming anything in this system**, and adopting it as one costs nothing that is currently in use.

The five nouns it was weighed against are not free, and
[P-0031](../principles/0031-a-name-means-one-thing-across-the-system.md) is the rule each of them
loses to: *it is not enough to be disjoint in practice; they have to be disjoint by name.*

**`host` — 217 word occurrences, of which about 99 are the bare noun, in four live senses.** The
dominant one is the CPU side of the GPU boundary and it is everywhere: `karakuri-engine/src/probe.rs`
(twenty-five occurrences on its own), `swap.rs`, `node/camera.rs`, `karakuri-ir/src/layout.rs`,
`karakuri-codegen`, three shaders, and `docs/contributing.md`'s rule that every performance figure
here is *"host-side and biased high"*. The second is the plugin host — `docs/plugins.md`,
[ADR-0074](0074-a-plugin-boundary-is-drawn-by-the-deterministic-path.md),
[ADR-0075](0075-the-plugin-abi-passes-a-handle-and-only-what-crosses-a-process.md),
[P-0042](../principles/0042-a-boundary-is-drawn-by-the-deterministic-path.md) and
[P-0043](../principles/0043-nothing-external-enters-the-render-process.md), where *"what the host
says"* is a specified behaviour. The third is `cpal`'s audio host: `karakuri-audio/src/device.rs:153`
is `let host = cpal::default_host();` and line 413 is `fn pick(host: &cpal::Host, …)`. The fourth is
a network hostname: `karakuri-cli/src/mcp.rs:805` is `let host = rest.split(':').next()…`, matched on
the next line against `localhost` and `127.0.0.1`.

**Two of those four senses are inside modules that move.** `audio.rs:207` measures *"the host
clock"*, which is sense one; `mcp.rs:805` binds a variable called `host` to a hostname, which is
sense four. A package named `karakuri-host` would contain, on day one, two different bare nouns
spelled `host` and neither of them itself.

**`shell` — 108 word occurrences, in three senses, all three of them live inside the modules that
move.** An OS shell: `tempo_source.rs:253` (*"quoting rules are a shell, and half a shell"*) and
`tempo_source.rs:550`, which is the sentence the whole design turns on — *"kill the shell and its
`sleep` lives on"* — plus `main.rs:1067`, *"so a shell can redirect it"*. A recycled empty record:
`session.rs` has a `SHELLS` constant, `return_shells` and `audio_shells` channels, and a test
asserting *"the shell left behind is not empty"*; `audio.rs:261` and `main.rs:4650` swap the same
shells. And a procedure name in the project's own material: `drift_shell`, `glass_shell`,
`sphere_shell`, `lattice_shell`, `strand_shell`, `beat_shell`, with `"shell"` used as an L1 slot name
throughout `mcp.rs`'s tests. Naming the package `karakuri-shell` would put a fourth sense into
`session.rs` and `tempo_source.rs`, which are two of the files it would contain.

**`Program` is a console bay.** `docs/manual/console.html:123` labels the centre bay Program, its
picture is a sink listed in Outputs as *program view*
([ADR-0159](0159-the-consoles-words-are-the-manuals-and-the-middle-one-is-not-a-pane.md) settled that
spelling), and two records are titled after it —
[ADR-0182](0182-the-program-bays-body-arranges-itself-for-the-larger-picture.md) and
[ADR-0184](0184-the-program-bay-rearranges-itself-and-a-still-frame-does-not.md) — with `docs/roadmap.md`
naming the bay eight times. The collision is not hypothetical and it is not distant: **ADR-0214's own
title uses *the program* in the other sense**, and `roadmap.md:347` uses it in the other sense one
line away from a paragraph about the bay. The two documents this record sits between already disagree
about what the word means.

**`session` is defined, and the collision is whole-to-part.**
[P-0013](../principles/0013-a-set-is-a-projection-and-a-session-is-the-timeline.md) says a session is
the timeline — the Set, plus a `tick` every frame, plus every edit at its frame position — and the
word appears on 1,071 lines of this repository. `karakuri-cli/src/session.rs` is one of the thirteen
modules that would move **into** the package, so `karakuri-session` would name the container after
one of the things it contains.

**`instrument` is the whole thing.** 91 word occurrences;
[P-0030](../principles/0030-an-instrument-says-what-it-did.md) is named for it,
[ADR-0045](0045-the-cli-is-an-instrument-not-a-demo.md) is titled with it, and
`docs/manual/index.html` uses it five times. Every bare-noun use means the system entire — *"the only
instrument that can read this"*, *"nothing in this instrument can set a position"*, *"no switch that
hands the whole instrument to an agent"*. A package is a part, so this one fails the same way
`session` does and more loudly.

`core` is the interesting one, because it is nearly free. Excluding `Cargo.lock` and vendored crate
names it occurs 49 times, and none of them is a defined noun naming a component of this system: CPU
cores, `git config core.hooksPath`, *"Core Principles"*, *"Core Rules"*, *"WebGPU core"* — and,
worth stating because it is not purely a modifier, about fifteen bare-noun uses meaning **the bright
centre of a picture**, which is a real recurring noun here
([ADR-0037](0037-tone-mapping-is-a-uniform-and-the-default-is-chosen-by-looking.md)'s comparison
table, `karakuri-engine/src/present.rs:70`, `karakuri-engine/tests/meter.rs` where `let core =
measure(&gpu, &core_view)`, and `examples/beat_strands.kir`'s *"the blue core and the violet
shell"*). None of that is fatal. **`core` does not lose on P-0031. It loses on P-0060**, and that
argument is below.

## The boundary test, applied

Built from `crates/karakuri-cli/src/` on 2026-08-28. Thirteen modules are in scope; `main.rs` is not
in the table because ADR-0214 already said it is the one that **splits** rather than moves, and
nothing here changes that.

| Module | What it deals with that is outside this process | Passes |
| --- | --- | :---: |
| `audio.rs` | a microphone, through `karakuri_audio::AudioInput`; and an `Instant` | yes |
| `midi.rs` | MIDI ports, through `karakuri-midi`; and the map file it reads | yes |
| `mcp.rs` | a `TcpListener` on loopback, and the client process on the other end of it | yes |
| `tempo_source.rs` | another program — `Command::new`, its stdout and its stderr | yes |
| `watch.rs` | the filesystem, while it is changing | yes |
| `scratch.rs` | `<store>/scratch/` — `std::fs`, `create_dir_all`, `write` | yes |
| `history.rs` | the history directory, the same way | yes |
| `session.rs` | a `Store` and a writer thread doing the I/O — **and it is the record** | yes, both clauses |
| `setfile.rs` | a `Store`, and `SystemTime` for `written_at` | yes |
| `render.rs` | a PNG file it creates and writes | yes |
| `meta.rs` | nothing outside — but it **is** the metadata card | second clause |
| `mix.rs` | nothing outside — but it **is** the records and their wire spellings | second clause |
| `compile.rs` | a `.kir` file it reads off a disk | yes — see below |

**Two modules pass on the second clause alone, and that is what the second clause is for.** `meta.rs`
takes a `Checked` and returns `Vec<Line>`; `mix.rs` converts between engine types, the operation
vocabulary and `Record`s. Neither touches a device or a file. Both are entirely *the record of what
happened*: `meta.rs` produces `<hash>.meta.ndjson`, and `mix.rs` is the record vocabulary for the
faders, the blend modes, residency and the look — the state
[P-0028](../principles/0028-every-control-ends-in-the-same-record.md) requires every control to end
in. A test with only the first clause would have left both of them in the command line, which is
wrong, and that is the reason the sentence has two halves.

### `compile.rs` passes, and this record reached that by reading it rather than by assuming it

`compile.rs` was proposed as the one misfit — 73 lines of parse, check and build, glue over
`karakuri-ir`, `karakuri-codegen` and `karakuri-engine`, with nothing outside the process in it.
**Two parts of that description do not survive the file.** Its non-test code names exactly one
workspace crate, `karakuri_ir` — `parse`, `check::check` and `cost::estimate` — and neither
`karakuri-codegen` nor `karakuri-engine` appears in it at all. It could not reach the first of those
if it wanted to: `karakuri-cli/Cargo.toml` does not depend on `karakuri-codegen`, so the file's own
header line about *"stage 5 in `karakuri-codegen`"* describes where a stage lives and not what this
module runs. And it is not clean of the outside: `compile::load` is
`std::fs::read_to_string(path)`, which is a disk.

More than that, **the file's own doc comment is the argument that it deals with the outside**, and it
is the strongest single statement of the test anywhere in the repository. `load` returns the compiled
procedure *and the bytes it was compiled from*, and says why:

> A `.kir` is read here exactly once per run, and what the run goes on to say about that node — the
> address a live save writes, the hash a `procedure` record names, the artifact a replay resolves —
> is a function of *these* bytes. A second reader asking the path again is a second answer to "what
> is this node running", and it is a different answer the moment anything has rewritten the file in
> between: an editor, a model over MCP, a formatter.

That paragraph exists **because the file is outside this process and can change under it.** A module
whose central design comment is about the disk changing beneath a second read is not a module with
nothing outside it; it is a worked example of P-0060 applied to exactly this boundary — name which
read is canonical, rather than deleting the second reader.

The residual doubt is real and is worth writing down: `check(src)` and the private `compile(src)`
take bytes already in hand and are pure, so a reader who points the test at those two functions gets
*no*. The test asks what a **module** deals with, not what each function does, and what this module
deals with is a file other processes rewrite. **Thirteen of thirteen move.** If a later reading splits
`compile.rs` along that line, this record does not decide where the pure half goes — see below.

## What the test decides that ADR-0214 refused to guess

This is the part worth keeping, because it is the whole return on naming a property.

**`Clock` — `karakuri-cli/src/main.rs:4717 — passes, and moves.** ADR-0214 said *"`Clock` is the one
this record will not guess at"*, because ADR-0172 had called it a program's and it *"may equally be a
surface's"*. The test answers it without anybody having to adjudicate that: `Clock::steps(&mut self,
now: Instant)` computes `now.duration_since(self.last)`, and wall-clock time comes from outside this
process. Its own comment already says which side it is on — *"`steps` is told what time it is rather
than asking"*, so that a test can state an elapsed interval instead of asserting tautologies. The
thing being passed in from outside is precisely what makes it belong.

**`Live` — `karakuri-cli/src/main.rs:5174` — fails, and stays with the surface.** Its first two
fields are `window: Arc<Window>` and `gpu: Gpu`. It is the assembly a surface makes, and it *holds*
the things that pass — `clock`, `audio`, `midi`, `tempo_source`, `rebuilds` — which is exactly what a
surface should look like once the package exists: a window, a device, and handles on everything
outside.

Neither answer required an opinion about what a *program* is. A name that names a position — `core`,
`shell`, `host` — could not have produced either one, because *inner* and *outer* are relations
between crates and say nothing about a struct. **That is the case for the name, and it is the reason
these two decisions are one record.**

## Alternatives rejected

**`karakuri-core`.** The standing recommendation until `environment` was proposed, and it has no
collision worth the name — 49 loose occurrences, none of them a defined component. It loses on
[P-0060](../principles/0060-name-the-property-not-the-shape.md): it names a **position** rather than a
property, and it cannot be applied. Asked whether `midi.rs` belongs in `karakuri-core`, the word
returns nothing; asked whether `Clock` does, it returns nothing again, which is precisely the
question ADR-0214 left open and the reason it was left open. It is also the **wrong** position. A
`-core` reads as the innermost crate of a workspace, and this one is not innermost: it sits on top of
all eight of `karakuri-cli`'s workspace dependencies — `karakuri-audio`, `karakuri-engine`,
`karakuri-ir`, `karakuri-midi`, `karakuri-operation`, `karakuri-operation-record`, `karakuri-signal`
and `karakuri-store` — and is depended on by exactly two binaries. The name would be a claim about
the dependency graph that the dependency graph contradicts.

**`karakuri-host`, `karakuri-shell`, `karakuri-program`, `karakuri-session`, `karakuri-instrument`.**
Each on its measurement above, and each on
[P-0031](../principles/0031-a-name-means-one-thing-across-the-system.md). Two of them —
`karakuri-host` and `karakuri-shell` — would collide with senses that are live **inside the files
being moved**, which is the worst case the principle describes: not two subsystems that happen to be
disjoint, but one file where both meanings sit and the reader has to work out which door they came in
by. Two more — `karakuri-session` and `karakuri-instrument` — name the container after a whole or a
part it already contains.

**The swap: rename `karakuri-engine` to `karakuri-core`, and give `engine` to the new package.** This
is the strongest of the rejected options and it deserves its space, because it **fixes** the objection
to `core` rather than dodging it. The renderer genuinely is innermost; a `karakuri-core` there would
be a true statement about the graph, and it frees the better word for the package that has no name.

It lost on what `engine` means. **An engine is the thing that drives, and the driving already lives in
the renderer.** `crates/karakuri-engine/src/` is 16,750 lines whose own header reads *"The render
graph, Set lifecycle, and pipeline management"*, and what is in it is: `frame.rs` and its `compose`,
which is the frame loop ADR-0172 moved there precisely so there would be one; `governor.rs`, which
decides what runs under budget; `transport.rs` and `transition.rs`; `deck.rs`, which advances the
decks and composites them; `swap.rs`, with the watchdog and the rollback; `set.rs`; `probe.rs`;
`present.rs`; `meter.rs`; `compaction.rs`; `binding.rs`; the node graph. Every one of those is
something that *drives* something else. The new package, by contrast, is what an engine is **plugged
into** — a disk, a device, a port, a socket, another process, and the record of what happened. Naming
it `engine` would mean two crates are engines and the one that actually drives is not called one,
which is P-0031 arriving by the back door in the same move that was supposed to clean the vocabulary
up.

**An objection was raised against the swap on cost, and it was withdrawn. That is worth recording as
history rather than as reasoning.** The objection: **21 ADRs name `karakuri-engine`** — counted, in
either spelling — and a rename stops every one of those references resolving. It was withdrawn on
reading [docs/contributing.md](../contributing.md) §4, which says an ADR is a description of history,
that the past is not revised, and that **annotating a record with what it later became is welcome**.
The cost of a rename is therefore annotation, not revision, and 21 annotations is not a reason. See
[P-0066](../principles/0066-an-adr-is-a-description-of-history-corrected-but-never-revised.md).

**What decided the swap was the meaning, and not the bill.** That is
[P-0085](../principles/0085-take-the-mechanism-that-exists-and-pay-the-bill-now.md) working as
intended: before v1 the bill is estimated honestly and then set aside, and a rename that was right
would have been paid for. This one was not right, and it would have been just as wrong if the count
had been zero.

## What this record does not decide

- **Whether the move lands in one commit or several.** ADR-0214's third open item, untouched. Several
  is still the likelier answer.
- **Where the pure half of `compile.rs` goes**, if a later reading splits `check`/`compile` off from
  `load`. This record concludes the module passes as a module; it does not claim the file is
  indivisible.
- **How `main.rs` divides.** ADR-0214 said its 7,688 non-test lines *"will not divide along a line
  anyone can name today"*, and the test above narrows that but does not close it: it settles `Clock`
  and `Live`, which were the two named, and the argument parser and the key handler are still the
  command line's by inspection rather than by rule.

## Consequences

- **The test goes in the new crate's `lib.rs` as its charter, on the day the crate exists.** That is
  this repository's pattern and it is worth following literally. `karakuri-midi/src/lib.rs` opens
  with *"Why this crate knows nothing about the engine"* and *"That is the invariant, not an
  arrangement"*, and cites [ADR-0180](0180-the-operation-vocabulary-is-a-crate-with-no-dependencies.md)
  for the charter of the crate below it;
  [ADR-0192](0192-an-operation-asks-for-what-a-surface-can-say-and-the-record-stays-whole.md) states
  the same crate's charter as *"no engine, no state and no readback by charter"*.
  `karakuri-engine/src/lib.rs` does it too, and adds the caveat this one should copy: *"These are
  one-line restatements; the canonical text is one file each in `docs/principles/`, and where these
  disagree with it, this comment is the one that is wrong."*
- **A placement question now has an answer that is not a vote.** The next module anyone proposes for
  this package is checked against one sentence, and the sentence is in the file they are about to
  edit.
- **`environment` becomes a defined noun**, and stops being available as ordinary English in this
  repository's prose. Twelve existing uses are all outside `src/` and none of them is about this
  package; they are not wrong today and this record does not ask anyone to rewrite them, but a
  thirteenth written after this lands should mean the crate.
- **`karakuri-engine` keeps its name**, and the 21 records naming it go on resolving. That is an
  outcome of the swap being refused on meaning, not a goal that was protected.
- **`Clock` moves and `Live` does not**, which is the first concrete instruction ADR-0214's move
  receives beyond its own list.
