---
id: 0214
title: The program moves out of the CLI, and two thin binaries sit over a shared package
status: accepted
date: 2026-08-28
supersedes: []
superseded_by: []
principles: [0044, 0085, 0093]
tags: [architecture, ui]
---

# The program moves out of the CLI, and two thin binaries sit over a shared package

## Context

[ADR-0172](0172-the-frame-loop-is-the-engines-because-the-cli-is-scaffolding.md) moved the frame
loop into `karakuri-engine` and, in the same sentence, said what it was leaving behind:

> `PngSink` stays in `karakuri-cli`, and so do the clock, the recorder, the watcher and the MCP
> server: those belong to a *program*, and a frame is a deck, a `Present` and somewhere to put the
> result — all three of which are the engine's.

That was the right cut for a frame loop. What it left standing is the problem this record is about.
The clock (`main.rs`), the recorder (`session.rs`), the watcher (`watch.rs`) and the MCP server
(`mcp.rs`) are still in `karakuri-cli`, and so is everything that came with them: the store wiring,
the audio input, the MIDI map, the Set file format, and the argument parsing. **`karakuri-cli` has
no library target** — no `src/lib.rs`, no `[lib]` section in its `Cargo.toml`, and
[docs/contributing.md](../contributing.md) §3 says so beside the command that has to work around it
(`cargo test -p karakuri-cli --bins`). So **nothing in this workspace can depend on any of it.**

`README.md`'s Stack section says where that lands: *"The CLI is scaffolding rather than the
destination"* — the end state is a GUI application, and *"It is an example rather than a program
yet"*. That application is `karakuri-console`, it is what M5 is named for, and it needs every one of
those parts. The scaffolding holds the program, and the destination cannot reach it.

### What is in there, measured

Counted on 2026-08-28. `karakuri-cli/src/` is **14 modules and 27,517 lines**, of which **15,995 are
outside the first `#[cfg(test)]` in each file** and the remaining ~11,500 are tests:

| Module | Lines | Non-test | What is in it |
| --- | ---: | ---: | --- |
| `main.rs` | 11,569 | 7,688 | the clock (`Clock`, line 4717), argument parsing (`Args`, `parse_args_from`), the key handler, the run loops |
| `mcp.rs` | 5,180 | 2,544 | the MCP server |
| `setfile.rs` | 3,587 | 1,745 | the Set file format, and `written_at` |
| `mix.rs` | 1,265 | 669 | the mixer's records and its wire spellings |
| `tempo_source.rs` | 1,112 | 557 | the external tempo-source protocol |
| `watch.rs` | 1,039 | 551 | the watcher |
| `session.rs` | 769 | 397 | the recorder (`Recorder`) |
| `history.rs` | 728 | 343 | the edit history |
| `audio.rs` | 670 | 423 | audio input |
| `midi.rs` | 666 | 377 | the MIDI map |
| `render.rs` | 392 | 335 | `PngSink` |
| `scratch.rs` | 349 | 175 | the scratch store |
| `meta.rs` | 118 | 118 | the metadata card |
| `compile.rs` | 73 | 73 | parse, check, build |

By comparison the destination is `karakuri-console`: 8,614 lines in `src/` and a **6,276-line
`examples/panel.rs`**, which is the whole of the application and is an example because there is
nothing for a binary to sit on.

### The boundary has already cost, and each time it arrived as a different question

This is the part worth stating plainly, because *give the CLI a library target* keeps being weighed
as a packaging preference. It is not one. The phrase **"no library target"** appears in five
separate places in this repository's records and source, and every one of them is a thing that was
paid for:

1. **[ADR-0172](0172-the-frame-loop-is-the-engines-because-the-cli-is-scaffolding.md), the frame
   loop.** The console's example hand-rolled a second frame loop *"which is precisely the failure
   `frame.rs` was written to end, reappearing one crate over and out of reach of the thing that
   ended it."* Paid by moving seven types into `karakuri-engine`.
2. **[ADR-0185](0185-a-fader-translates-a-drag-into-an-operation-and-applies-nothing.md) and
   [ADR-0194](0194-where-an-operation-becomes-a-record-is-a-crate-that-depends-on-both.md), where an
   operation becomes a record.** ADR-0180 had named `karakuri-cli` as the home and *"it cannot be
   it: the package has no library target, so nothing else in the workspace can call into it. That is
   not an incidental fact — it is the stated reason `karakuri-console/examples/panel.rs` wrote three
   records again."* Paid with a **whole new crate**, `karakuri-operation-record`, whose `lib.rs`
   header still explains itself by naming the missing library target.
3. **[ADR-0200](0200-a-bays-first-pass-draws-the-values-that-exist-and-omits-the-rest.md), the
   Library bay's time column.** The one that shows the shape best, because it was filed as a
   *date-formatting* question. The value exists — `SetEntry::written` is the Set file's mtime,
   handed over with the id — and the spelling exists too: `karakuri-cli`'s `setfile::written_at`,
   *"local, because the answer has to be the one the person would say out loud"*. ADR-0200: *"it
   lives in a package with no library target, so nothing can call it."* **Paid by not drawing the
   column**, rather than by shipping a third time format. A drawn feature was cut by this boundary,
   and the record about it is not about this boundary at all.
4. **`examples/panel.rs:2418`, the store path.** `const STORE: &str = ".karakuri"` is a transcription
   of `karakuri-cli`'s private `DEFAULT_STORE`, with the note *"there is nothing to ask… the day one
   moves this example lists an empty library rather than the wrong one."*
5. **`examples/panel.rs:2701`, the residency spelling.** `mix::parse_residency`'s three words,
   written a second time, *"for [`apply`]'s reason: `karakuri-cli` has no library target, so the same
   three words cannot be reached from this file."*

Two of those were paid by moving code, one by adding a crate, one by cutting a feature, and two by
copying a constant and a parser. **Four bays of the console are still undrawn.** A seam that produced
five differently-shaped questions across the bays already drawn will produce more across the four
that are not, and each will again look local. That is the reason to take it now rather than after.

## Decision

**The program moves out of `karakuri-cli` into a shared package, and two thin binaries sit over it.**

- **`karakuri-cli` stays the scaffolding.** It keeps its flags, its terminal, its keys and its
  `--headless` runs, and it becomes a binary that parses arguments and calls into the shared package.
  `README.md`'s sentence is unchanged by this: the CLI is still not the destination.
- **The panel becomes the second binary, and it is the destination.** `karakuri-console`'s
  `examples/panel.rs` stops being an example held up by having nothing to depend on.
- **`karakuri-console/src/` keeps its seam exactly as it is.** No new dependency is added to it. The
  surface, the device and the event loop stay dev-dependencies of that package, `src/` goes on
  taking no device, and [ADR-0156](0156-the-consoles-arrangement-is-a-tree-this-repository-owns.md)
  is untouched. The new binary is a third party to that crate, the same way the example is today.

The shape is *shared package, thin binaries* rather than *one binary lending its insides to another*.
Nothing in the program is the command line's, and nothing in it is the panel's; both are surfaces,
which is what the vocabulary has said since [ADR-0156](0156-the-consoles-arrangement-is-a-tree-this-repository-owns.md).

## Alternatives rejected

**A `[[bin]]` target in `karakuri-console`.** The cheapest thing to type: the example already is the
application, and a `[[bin]]` would make it one without a new package. It loses on what it does to the
manifest. `wgpu`, `winit` and `karakuri-engine` are **dev-dependencies** of `karakuri-console` today,
and a `[[bin]]` cannot use a dev-dependency — all three would be promoted to real ones, and from that
moment `src/` *could* reach a device. `docs/contributing.md` §3 names exactly this as the seam:
*"`src/` takes no device at all and that is the seam [ADR-0156] exists to keep."* The crate's own
`Cargo.toml` says the same thing in the comment over those two lines — a surface, a device and an
event loop are the example's and *"stay dev-dependencies so that nothing in `src/` can reach for
one"*. Today that is not a rule anyone has to remember; it is a link error.
`docs/contributing.md` §4 permits
a structural guarantee to become a convention only where it says so, and this would demote a
cargo-enforced guarantee to a comment **in order to save one package**. That is the trade the
principle exists to refuse.

**`karakuri-cli` gains a library target and the application depends on it.** Rejected by name in
[ADR-0172](0172-the-frame-loop-is-the-engines-because-the-cli-is-scaffolding.md):

> **Rejected: keep the loop in `karakuri-cli` and give that crate a library target.** It works, and
> it loses on what it says: the application would then depend on the scaffolding, and every later
> question about where something belongs would be answered by whichever crate happened to have it
> first. The README's sentence is a design statement, not a caveat.

[ADR-0194](0194-where-an-operation-becomes-a-record-is-a-crate-that-depends-on-both.md) rejected it a
second time on a mechanical ground rather than a stated one: *"that lib is `wgpu`, `winit`, the
engine and a GPU, so the console's dev-dependency on it would put a device into a crate whose `src/`
has none by design."*

**It is worth recording that it was re-proposed anyway, in the session that wrote this record**, and
how. The argument was
[P-0085](../principles/0085-take-the-mechanism-that-exists-and-pay-the-bill-now.md) — prefer the mechanism that
already exists — and it is a good argument on its face: `karakuri-cli` already has the audio, the
MIDI, the MCP server, the store, the records and the flags, so the application is *"the mechanism
that already exists"* and a `[lib]` line is the whole of the work. What caught it was
[P-0044](../principles/0044-doubt-a-conclusion-that-contradicts-a-position-already-taken.md): **when
a judgement about this design collides with a stance the project has taken repeatedly, the thing to
re-examine is the judgement, not the stance.** The stance had been taken twice by name and stated a
third time in `README.md`. P-0085 is about not building a subsystem where a filter would do; it is
not a licence to make the destination depend on the scaffolding, and reading it as one was the
failure. The mechanism that already exists here is the *code*, and this decision keeps all of it —
what it declines to keep is the *package boundary* it happens to sit behind.

**The panel program re-implements the program parts.** The status quo, extended: `examples/panel.rs`
already does this twice (the store path and the residency spelling), and this alternative is to keep
going. It is a second answer to a settled question, which is the exact failure `frame::compose` was
written to end and which
[ADR-0172](0172-the-frame-loop-is-the-engines-because-the-cli-is-scaffolding.md) found *"one crate
over and out of reach of the thing that ended it"*.
`docs/contributing.md` §4 bites first — two spellings
of a residency, two answers to what a store path is, two formats for a written-at time, and which you
get depends only on which door you came in by. [P-0085](../principles/0085-take-the-mechanism-that-exists-and-pay-the-bill-now.md)
bites second, and this time correctly: `setfile::written_at` and `mix::parse_residency` *are* the
mechanism that already exists, and writing them again is the heavier design arriving first.

## What this record does not decide

Three things, deliberately, so that the next person does not read this as settling more than it does:

- **The shared package's name.** Not chosen here, and it should not be chosen by whoever files this.
- **Its boundary.** *Which* of `karakuri-cli`'s fourteen modules are a *program*'s and which are the
  command line's own is open. `mcp.rs`, `session.rs`, `watch.rs`, `setfile.rs`, `audio.rs`, `midi.rs`
  and `history.rs` look like the program's, and `parse_args_from` and the key handler look like the
  command line's; `main.rs` is 7,688 non-test lines holding both and will not divide along a line
  anyone can name today. **`Clock` is the one this record will not guess at** — ADR-0172 called it a
  program's, and it may equally be a surface's.
- **Whether it lands in one move or several.** Several is the likelier answer and nothing here
  forecloses it: a first move that carries only what the console needs to become a binary is a
  legitimate reading of this decision.

## The bill

**Around 16,000 non-test lines are in scope, with about 11,500 lines of tests beside them**, and
**most of it is a move rather than a rewrite** — a package boundary changes, `use` paths change,
`pub` appears where `crate`-private was enough, and the code inside the functions does not. The
honest uncertainty is `main.rs`: its 7,688 non-test lines are the part that is genuinely split rather
than relocated, and any estimate of *that* is a guess until the boundary above is decided.

There is a precedent for what a move of this kind actually costs.
[ADR-0172](0172-the-frame-loop-is-the-engines-because-the-cli-is-scaffolding.md) performed a
comparable one and recorded the result: *"**Nothing gains a dependency**, which is what makes this a
move rather than a redesign"*, and *"The workspace test count did not change either."* It also
recorded the one class of surprise to expect — `#[cfg_attr(test, derive(Debug))]` does not survive a
crate boundary, because `cfg(test)` is set when the *defining* crate's tests compile and not when a
consumer's do, and it noted that *"Two more types in `karakuri-cli` carry the same idiom and will hit
the same wall the day either moves."* That day is this one.

[P-0085](../principles/0085-take-the-mechanism-that-exists-and-pay-the-bill-now.md) is why the number
is written down and is not an argument: before v1 the bill is estimated honestly when a fork is
presented, **and it does not decide the recommendation.** Sixteen thousand lines is a large number
and it is the wrong thing to weigh this on. What is weighed is the five costs above, each of which
arrived looking like something else.

## Consequences

- **The Library bay's time column is unblocked**, as a side effect rather than as a goal.
  [ADR-0200](0200-a-bays-first-pass-draws-the-values-that-exist-and-omits-the-rest.md) omitted it
  because `setfile::written_at` could not be called; when it can, the column draws the same spelling
  the CLI and MCP already use, and the mock's third format is never invented. That record is not
  superseded — its rule, that a bay's first pass draws the values that exist, is untouched — but the
  fact it turned on stops being true.

  *(This happened on 2026-08-29: `setfile.rs` moved to `karakuri-environment` in the second slice,
  and `written_at` is callable. One thing this paragraph did not foresee: `karakuri-console/src/`
  cannot be the caller, because depending on this package would put a device into a crate whose
  `src/` has none by design. The host formats the date and hands it in, which is what `view` does
  with every other derived value, so the conclusion holds and the caller is not who it sounds like.)*
- **The example's two transcriptions are deleted rather than moved.** `STORE` asks for
  `DEFAULT_STORE`, and the residency decode calls `mix::parse_residency`. Both carry a comment saying
  this is what they are waiting for, which is how they will be found.
- **`karakuri-console`'s manifest does not change**, and neither does its test story: the example
  stays a test target, its device-touching tests stay under `mod gpu`, and `cargo test -p
  karakuri-console -- --skip gpu::` goes on subtracting exactly what it subtracts now.
- **`cargo test -p karakuri-cli --bins` stops being the command**, and
  `docs/contributing.md` §3's comment *"`karakuri-cli` has no library target"* stops being true. It
  is corrected when the move lands, not by this record.
- **Three types in `karakuri-cli` carry `#[cfg_attr(test, derive(Debug))]`** and will stop compiling
  for their consumers on the day they cross the boundary — `main.rs:859`, `mix.rs:146` and
  `setfile.rs:91`, counted on 2026-08-28. The fix is an unconditional `Debug`, as every public engine
  type already has. **ADR-0172 named this in advance and said *two*, which was true when it was
  written**: a third has been added since, and transcribing its number rather than counting is how
  this record nearly shipped the same error it is about.
- **The four undrawn bays get a shorter road.** Staging, and the three after it, will each ask for
  something the program knows. None of them will have to ask this question again.
