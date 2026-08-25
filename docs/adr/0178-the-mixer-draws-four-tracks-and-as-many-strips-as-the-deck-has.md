---
id: 0178
title: The mixer draws four tracks and as many strips as the deck has
status: accepted
date: 2026-08-25
supersedes: []
superseded_by: []
principles: []
tags: [ui]
---

# The mixer draws four tracks and as many strips as the deck has

## Context

The Mixer is the first bay with a **repeated** component and the first whose contents are the
engine's *state* rather than its texels. The mock draws four strips; a `Deck` holds one to four
slots, and the console's example holds one.

## Decision

### Four tracks, `n` strips, and an unfilled track draws nothing

Settled from the arrangement rather than from taste. `right-pane`'s minimum width of 172 is written
in `lib.rs` as *"four mixer strips still side by side"* — so tracks that followed the strip count
would re-derive that minimum every time a slot was installed, and would make a one-slot strip 244
wide in a pane sized for four 61-wide ones. The track grid is `repeat(4, 1fr)` because the pane's
own minimum says four; how many of them hold a strip is the deck's business.

**Rejected: a well per track, empty where no deck fills it.** That is a reading — an empty strip
says *there is a deck here and it is at zero* — and inventing one is what the deck previews' `off`
cells and the transport row's absent numbers both refuse. A track nothing fills draws nothing at
all.

### A missing meter reading is the caller's decision, not a threshold in the console

`karakuri_engine::meter::Level` carries `mean`, `peak`, `frames_behind` and `bad_texels`. The
console carries `mean` and `peak`.

**`frames_behind` is deliberately not carried**, and the reason is that no threshold on it is right
from here. The engine's own measured table gives 1 frame behind under pacing, 2 or 3 on a vsync
window, and *tens — 13 to 101, median 58* — with nothing pacing the loop, and **all three are
normal**. A staleness rule chosen in `karakuri-console` would blank a healthy meter on an unpaced
loop and pass a dead one on a vsync window, because the console does not know which loop it is on.
The case the number exists for — a held reading whose slot went off air or was resized — is one
`Deck::level` already answers `None` to, by generation.

What a meter draws is **a fact about a frame that has already been drawn**, exactly as
`Transport::frame_ms` is, and it does not go stale by standing still the way a *rate* does. That is
[ADR-0177](0177-the-transport-row-shows-what-the-console-can-know.md)'s distinction applied a
second time: handing over no reading at all is the caller's decision, on the same terms `fps: None`
is, and the caller is the one thing that knows its own loop.

### The fader and the meter share a function, not a type

One function — a value turned into a length up a track, from the left of a row and the **bottom** of
a column — with three call sites the day it lands: the trim's fill, the opacity fader's fill, and
the meter's column. That is all they share.

**Rejected: one type for both.** A fader's knob is a grab target and the next pass makes it one; a
peak mark is a reading and never will be, so a shared type would be a type half of whose fields are
about a gesture the other half can never have. They also disagree about the track — a fader's fill
sits inside its well and a meter's fills it edge to edge, a fader's fill is a capsule and a meter's
is a square column, and a knob is *meant* to stand proud of a track that nothing in a meter may
leave.

`gain` and `opacity` stay two fields drawn as two different controls, because `mix.rs`'s asymmetry
is the whole reason they exist separately: opacity at zero silences under every blend mode and gain
at zero does not silence `over`.

### The header pill is not drawn

The mock's head reads `4 of 4 · page 1`, and the manual says what it is: *"the strip is a **paged
list whose length is a number**, and the header says which page you are on."* Both halves are
paging, and there is none — every strip the deck has is on the one page — so the pill could only
read `n of n · page 1`: two numbers that are always equal and a third that is always 1. That is
`Kind::Bay`'s own rule about a pill stating a value the console does not have.

*(The ambiguity is recorded because it is close: `previews 2 of 4` reads as "2 of the 4 cells", so
`4 of 4` could equally mean "4 slots of `MAX_SLOTS`", under which it would be a real readout. The
manual's own sentence decided it over the analogy.)*

### The mask's mark is drawn rather than typed

`grip_dots`'s argument, one control along: whether a given glyph is in `egui`'s default face is a
question with no good answer. A circle, half-filled for a linear front, with a filled centre for a
radial one — a shape that can be argued from the mask instead of found in a character table.

## Consequences

- **`Kind::Mixer` is the first *bay* to need a kind of its own.** `View::draw` has to know which bay
  the strips go in, and comparing a name on the frame path is what the table exists to avoid —
  `Kind::Picture`'s precedent. `bay_head` still has seven call sites, now across two arms.
- **Nothing reachable from a `Deck` names the material in a slot.** `HotSwap` has no name and `Set`
  has node names and control names and no name of its own — which is right, since a Set is built
  from a list of `.kir` files and only whoever passed that list knows what to call the result. So
  the harness names its own: the example joins the file stems of the two paths it builds from, and
  the name cannot drift from what is loaded because it is derived from the same constants.
- **The mock's blend list is not the engine's.** The mini's tooltip says *"add, over, screen,
  multiply"* and `Blend::ALL` is `Add, Over, Max` — four against three, two of them words the
  engine does not have. The console draws whatever word it is handed. Recorded rather than
  resolved; the mock is not this record's to change.
- **The mock's `.strip.empty` is a strip a `Deck` cannot produce.** Its tally reads `empty`, and
  every one of a deck's slots holds a `HotSwap`. `Tally` has three variants and no fourth; the
  mock's fourth strip is a *track nothing fills*.
- **A gain above 1.0 cannot be shown.** The trim is drawn over `[0, 1]` — `karakuri-midi`'s
  `GAIN_RANGE`, whose own argument is that *a fader whose top is unity is what a fader means* — and
  `Deck::set_gain` is deliberately unclamped above it because the mix is HDR. Over-unity fills the
  track and stops. Written down where the trim is drawn; it belongs with the pass that lets a hand
  push the control past the top.
- **The mock's fader percentages contradict each other about the knob** — `.fader` is 72% wide with
  its knob at 66%, `.vfader` at zero puts the knob's edge on the floor, and at 97% the knob
  overhangs the top. Three hand-set pairs and no single rule. One number drives both here: the knob
  is centred on the fill's moving edge.
- **Two tests passed with their defect in place on the first attempt**, and both were the tests'
  fault. One compared shape counts where a zero reading draws a peak mark on the floor — one shape
  either way; the other was defeated by a `zip` that stops at the shorter side, which turned out to
  be the thing actually enforcing *one well per strip*. Both rewritten to name what they meant. The
  fourth time in this milestone that running a test against its own defect found the test rather
  than the code.
- **The controls in this bay are readouts.** Dragging a fader is a second kind of drag and the value
  belongs to the engine rather than to the layout, so it needs an operation and a decision about the
  claim model. Stated in the source so the next reader is not left to discover it.
