---
id: 0335
title: The panel opens the first surface there is, and the map is two tiers under the store
status: accepted
date: 2026-09-10
supersedes: []
superseded_by: []
principles: [0090, 0092, 0094, 0087, 0085, 0096]
tags: [midi, console, store, devices]
---

# The panel opens the first surface there is, and the map is two tiers under the store

## Context

`crates/karakuri` — the instrument, the program a player launches — has had no MIDI in it at all.
Its own header listed MIDI as one of the two things *not wired*, and said what the gap was: *"a
control surface, so a hand reaches a fader without a mouse. `karakuri-midi` and
`examples/surface.map` exist and `--midi-in` `--midi-map` drive them."* Both of those flags are
`karakuri-cli`'s, and `USAGE` declines them by name.

So every part of the route existed and none of it was reachable from the instrument:
`karakuri_midi::Map` turns a message into a `karakuri_operation::Operation`,
`karakuri_environment::midi::{Router, Surface}` opens a port and coalesces a frame's worth of it,
and `karakuri-cli` drains that into `Live::operate`. What was missing was a caller in the program
an operator actually plays.

**Three questions had to be answered to add one**, and each of them had a plausible other answer.

### a. Which port

`karakuri-cli` takes `--midi-in NAME` and refuses the run without a match. The panel takes two
positional paths and three flags, and
[ADR-0220](0220-the-key-column-is-the-instruments-keyboard-and-the-clis-keys-are-its-own.md)
is why that list is short: *this is not `karakuri-cli`'s command line and does not try to be.* That
record settled the **key** column by asking which program an operator is standing in front of, and
its consequences named the open question this one answers: *"The same question is now open for the
MIDI and MCP columns, and this record does not answer it. `crates/karakuri` has no MIDI, no MCP, no
audio and no session, so those two columns measure `karakuri-cli` and `karakuri-environment` while
panel and key measure the instrument. **Nothing checks the MIDI column at all.**"*

Two of the four have since been answered by being built — audio opens the host's default input at
start-up, and `--mcp PORT` binds a server — and the audio one is the precedent that matters here.
`listening`'s own documentation states it: *"A flag is a contract made before the run … This
program has somebody standing there."*

### b. Which map

The map file is the one place ADR-0227 found the library shape ragged. Its own words:

> **The map's second tier is not a store directory**, and that is the one place the existing shape
> is ragged: an operator's own map is whatever path they hand to `--midi-map FILE`. It ships as a
> preset and it is saved nowhere in particular.

`examples/surface.map` is the first tier and P-0096 keeps it read-only. The second tier had no
address, and a program with no flag cannot ask for a path.

### c. How a knob reaches the frame

The panel's loop **sleeps**. `App::about_to_wait` sets `ControlFlow::Wait` or a `WaitUntil` on the
soonest of three deadlines, and ADR-0164's still-panel clause is what that is for: a window that
costs the machine nothing until somebody touches it. A MIDI message arrives on `midir`'s thread and
crosses an `mpsc` channel; nothing in `winit` can see it. A drain placed in the frame loop with
nothing to wake the loop would apply a fader at whatever the operator's next mouse move happened to
be.

## Decision

**The panel opens the first MIDI input there is, at start-up, and says which in its legend.**
`karakuri_midi::Port::open`'s empty selector already means *the first input at all*; `Surface::first`
is the constructor for it and takes no name. Nothing plugged in is a state and not a fault, and the
run continues — every control on this panel is reached by the pointer and by a key, which is every
run this program has had until now. **A port that is there and will not open is said in the port's
own words and the run continues too**, because a window with a set on it must not fail to start
because another program holds the controller.

**The map is two tiers, and the operator's own is `<store>/maps/<name>.map`.**
`karakuri_environment::midi::map_for` resolves it: the operator's first, the shipped
`examples/surface.map` second, `None` for a machine with neither. That is ADR-0227's shape applied
to the file that record named as the ragged one, and it is ADR-0221's four parts unchanged — a name
the operator typed, one path component, a place of its own under the store root, and bytes nothing
on the way past parses.

**`default` is the name a program with no way to ask uses.** The pill that names another is the
transport row's `map`, and it is not drawn, so **the map is fixed for the run**.

**A message arriving wakes the loop; it does not poll.** `Port::waking` takes a closure and calls it
once per message on the MIDI thread; the panel hands in an `EventLoopProxy<()>`'s wake, and
`App::user_event` asks for a frame. **The drain itself is in the frame**, `App::mapped`, beside
`App::operated` at the top of `RedrawRequested`.

**And what it drains is handed to `App::performed` as an `Acted::Emitted`** — the value a fader on
the mixer strip hands it. A knob's `SetGain`, a model's and a hand on the strip are the same press
from there on
([P-0090](../principles/0090-a-surface-offers-it-never-decides.md)), which is what makes
[P-0092](../principles/0092-the-same-inputs-produce-the-same-frame.md)'s promise true here: **a
session recorded from this surface replays with neither the surface nor the map attached**, because
what reaches the stream is the record and no record names a knob.

**Nothing is gated.** `karakuri_operation::gate` is a model's boundary and not an operator's, and
P-0094 says a hand cancels whatever automatic thing was writing the control, on every route in. A
gate over the operator's own surface would be the instrument refusing its player.

## What this settles about the MIDI column

**The MIDI column is the instrument's map**, which is ADR-0220's answer for the key column read one
column along and is now true rather than aspirational: the program that column describes opens a
port and loads a map. **No badge moves.** The grammar is one file — `karakuri-midi`'s — that both
programs read, so every row the map already reached is reached by the instrument on the same line,
and *Write a parameter* stays `plan` because learn is not built.

**That is the difference between this column and the key column**, and it is why this cost nothing.
Two keyboards collided on eight letters and one of them had to be chosen; two programs reading one
map file cannot disagree about what `cc 1 -> gain 0` means.

## The tooltip's MIDI line, recommended and not taken here

Every tip on [the console page](../manual/console.html) ends with a `⊕ MIDI:` line, and it is the
page's own text — [ADR-0330](0330-the-console-paints-its-own-hover-layer-and-the-tips-are-the-manuals-own-words.md)
makes the hover layer a **citation** per control, so the console quotes the page and writes nothing.
Nineteen of those lines read `unassigned.` and ten name a target.

**The line should be derived from the live map, and the page's sentence should stay where a control
is unmapped.** A control's assignment is a fact the map holds, and
[P-0087](../principles/0087-name-the-property-never-the-shape.md) is the rule — *the operator turns
this knob and this control moves* is the property, and *the mock was drawn with `cc 2` on it* is a
picture of one arrangement. The page's ten assigned lines are the **mock's** assignments and no
operator's: a run whose map puts `cc 5` on gain A reads `cc → gain A` in the tip because the page
says so, and the tip is then confidently wrong about the one thing an operator would hover to check.
Where nothing is mapped the page's sentence is better than anything derivable, because it says *why*
— *a map line names a slot, a range or a word from a closed list, and a Set id is none of the three*
— and no reverse lookup can produce that.

**It is not done in this pass, and the reason is a date rather than a doubt.** Today the map is
loaded once at start-up and cannot change, and both programs read the same shipped file, so the
page's text and the map agree for the whole of a run; the sentence is wrong in principle and right in
fact. **Learn is what makes it wrong in fact** — a control rebound while the panel runs, with the tip
the place the new binding is read — so the derivation lands with learn, in that record, where it is
load-bearing.

**And the seam it lands on is already decided by ADR-0156**, which is worth writing down now because
it is the part somebody would get wrong. `karakuri-console` **cannot read the map**: `karakuri-midi`
depends on `midir`, and a console that named it would take a device dependency into the crate whose
whole charter is that it has none. So the assignment crosses the seam the way every other value does
— derived by `crates/karakuri` from the live `Map` and handed to the hover layer per frame, beside
the `View` — and the console goes on quoting the page for everything else in the tip.

## Alternatives rejected

**A `--midi-in` flag on the panel.** The shortest path, and it is `karakuri-cli`'s command line
arriving in the program ADR-0220 says is not it. It also answers at the wrong moment: which surface
is plugged in is a question an operator standing at the panel can see, and a launch-time answer is
one they cannot change without restarting.

**Ask, and open nothing until asked.** Consistent with the pill this panel does not draw, and it
makes the instrument come up looking like one that cannot take a controller — `listening`'s argument
for opening a microphone, word for word: *"an instrument that listens only after being asked comes
up looking like one that cannot."*

**Poll the port on a deadline.** The alternative to the wake, and the one that needed no new seam in
`karakuri-midi`. A third deadline beside `egui_due` and `served` has to run at a hand's rate to feel
like a fader — 125 wakes a second, for the whole of a run, whether or not anything is plugged in —
which is `ControlFlow::Poll` with extra steps and is exactly the cost ADR-0164's still-panel clause
is about. `SERVED`'s tenth of a second is the other end of the trade and is a fader at 10 Hz. The
wake costs one boxed closure in a callback that already does an `mpsc` send, at a rate bounded by a
hand moving.

**The panel's map is `--midi-map`'s file, wherever the operator keeps it.** The status quo for the
CLI and the thing ADR-0227 called ragged. A program with no flag cannot be handed a path, so this is
not an alternative here so much as the reason the second tier had to be given an address at all.

**Put the map in the session stream.** Refused where it always is: `karakuri-midi`'s own header,
`karakuri-environment::midi`'s, and ADR-0007. Which knob is which is a property of the hardware in
the room, and replaying one room's wiring in another is not replaying a performance.

## Consequences

- **`crates/karakuri/src/main.rs`'s *What is not wired* list loses its MIDI entry's absence.** Two
  were left — MIDI and replay — and one is left: replay, which is offline work and belongs in
  `karakuri-cli`. The entry stays and says what the thing is *for*, which is what that section is
  worth reading for.
- **`karakuri_midi::Port` gains `waking`, and `Port::open` is it with no wake.** The callback's
  state is a `Callback` struct rather than a bare `Sender`, because `midir` carries one value in.
  `karakuri-cli` is untouched and passes no wake: it renders every frame and asks anyway.
- **`karakuri_environment::midi` gains `map_for`, `MAPS`, `MAP_SUFFIX`, `DEFAULT_MAP`,
  `SHIPPED_MAP`, `Surface::first`, `Surface::port_name`, `Surface::map_name` and
  `Surface::mappings`.** `Surface::open` keeps its signature and its `eprintln!`s and is still the
  CLI's; both constructors now go through one private `assembled`, so they cannot come to disagree
  about what loading a map means.
- **`<store>/maps/` is not established by `Store::open`.** Nothing writes it yet, and a directory
  created to be read is what `places`' own header says a read-only flag must not do. Learn is what
  creates it, on the save.
- **The store gains a fifth kind of file and `karakuri-store`'s header does not know about it.**
  That is deliberate and is ADR-0227's shape: the store keeps bytes it does not have to understand,
  and this file is read and written by `karakuri-environment` with `std::fs`. It is a line that
  header owes the day something in `karakuri-store` has to list one.
- **`docs/manual/operations.html` gains and loses no badge**, and its legend still does not say
  which program the MIDI column measures — this record does. That sentence is owed to the page's
  own prose the way ADR-0220's is owed to the key column's.
- **`README.md`'s "no MIDI … yet" is now false.** Left for the maintainer.
- **`karakuri-console/src/view.rs`'s `transport` still names `learn` and `map` as the two the mock
  draws and this console does not, and it is still right** — a map is loaded by the *program*, and
  the console has none. What is now stale is one clause of the `map` entry's reason: *"a menu naming
  a device nobody has plugged in is worse than no menu"* was written when there was no port; the
  reason it is still not drawn is that reaching a map while running is not built.
- **A knob turned on a still panel now wakes the window**, which is a new way for this loop to run
  and the first one that is not a `winit` event or a deadline. `App::user_event` is the whole of it
  and it asks for a frame and nothing else.
