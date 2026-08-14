# Manual

For whoever is going to *play* this, rather than decide whether its design is sound.
`README.md` is the second document; this is the first.

Three sections, and the third is the one worth reading before a show: [Your first
set](#your-first-set) gets you from a clone to something on a projector, [Reference](#reference)
is every flag and every key, and [What it will not do](#what-it-will-not-do) is the honest
list of limits — including a few that are invisible until they bite.

---

## Your first set

### Run it

```sh
cargo run -p karakuri-cli
```

A window opens with the example pair — `examples/drift_shell.kir` describing a field of
elements and `examples/soft_points.kir` describing how they are drawn. Press `h` for the
keys, `s` for the status line, `esc` to quit.

The window is a **preview**. What is rendered is the *canvas*, and it is 1920×1080 whatever
size the window is:

```sh
cargo run -p karakuri-cli -- --canvas 1280x720 --size 960x540
```

The canvas is fitted into the window, so you will normally see black bars. Press `a` to size
the window to the canvas exactly — that is what you want if something downstream is
capturing the window.

### Change the material while it runs

```sh
cargo run -p karakuri-cli -- --watch
```

Edit either `.kir` and save. The new procedure is compiled on a worker thread and swapped in
between two frames. Then it is **judged**: eight warmup frames, thirty measured ones, and if
the median frame interval over those thirty is over the budget the candidate is dropped and
the outgoing Set is live again at exactly the `t` it was parked at. A file that does not
compile prints its diagnostics and never becomes a candidate at all.

An unknown parameter name in a `--bind` is reported and ignored rather than refused, so a
typo costs you the binding and says so:

```
no L1 parameter named `nonesuch` to bind, ignoring
```

The elements start cold on a swap — new buffers, `t` back to zero. That is not a bug to work
around; it is why the deck exists.

### Fill the deck

Up to four Sets at once:

```sh
cargo run -p karakuri-cli -- \
  --set examples/drift_shell.kir,examples/soft_points.kir \
  --set mine/field.kir,mine/draw.kir
```

`0`–`3` focus a slot. Everything that acts on "the focused slot" — gain, opacity, blend,
sync — acts on that one, and the status line marks it with `>`.

`space` puts the focused slot on and off air. Off air it keeps its `t`, so it resumes rather
than restarting. `w` asks for it to be *warmed* off air so that it is already running when
it comes up; that is a request, and the governor grants it only if the frame budget has room.

### Move between them

- `;` and `'` are the **fader**. Pull `;` to get out of material that has gone wrong — this
  is the control that silences a layer under every blend mode.
- `[` and `]` are the **gain**, which is colour only and is not clamped at 1.0 because the
  mix is HDR. `\` returns it to 1.0.
- `f` and `g` fade the focused slot out and in, on the beat.
- `x` crossfades to the next slot.
- `c` wipes the next slot in over this one — pick a shape with `z` first.

`n` chooses where a fade *starts* (the next bar, the next beat, now) and `j` how long it
lasts (4, 2, 8 beats, or 0 for a cut). Both are musical, so a tempo change mid-fade moves the
fade with it.

### Play to the room

```sh
cargo run -p karakuri-cli -- --audio-in default --bpm 128
```

Now `energy`, `onset` and the bands are measured rather than invented, and any binding to
them takes full effect. The beat grid is **predicted, not chased**: once locked it free-runs
and takes a slow trim, so a single bad estimate moves nothing.

Two keys matter here. `b` taps the beat — three taps set the tempo, one sets the phase. `,`
and `.` halve and double the grid, and **nothing does that for you**: a set started an octave
off stays an octave off until you press one. `x2?` on the status line is the tracker saying
it thinks it might be.

`o` and `p` move the latency offset, 5 ms a press. Raise it if the picture reads late from
where the audience is standing; lower it if the sound does. You are the only instrument in
the room that can see the projector and hear the PA at once.

### Change it by asking

```sh
cargo run -p karakuri-cli -- --watch --mcp 8737
```

Point a chat client at `http://127.0.0.1:8737/` and it can **read a slot's procedure, rewrite
it, and be told what happened**. "The one that's showing now, a bit more vivid" is a small
edit to a declarative file, and hot-swapping that file is what `--watch` already does.

Three tools. `read_procedure` gives you the source; `write_procedure` checks it and, if it
compiles, writes it — **and if it does not compile, what comes back is the checker's
diagnostics, against the source**, which is what lets a model fix its own mistake;
`swap_outcome` says whether the result landed, was rolled back for costing too much, or
failed to build. A write returning cleanly means it compiled, not that it is on screen, so
the third tool is where the loop closes.

Two resources come with it: the IR specification, and a list of every built-in the checker
accepts **generated from the checker's own table** rather than written down beside it. Prose
goes stale; that list cannot, because the same table is what rejects a procedure.

Nothing here can do anything a key cannot — it is the third control surface after the
keyboard and MIDI, on the same terms. **A set a model rewrote replays with no model
attached**: `--record-session` writes a `procedure` record whenever a swap lands, so
`--replay` rebuilds the slot at the frame it changed on. That was not true when this surface
was first built, and it is the one thing it needed of the format.

Three things worth knowing before you rely on it:

- **`--watch` is what picks a write up.** Without it the file changes and the screen does
  not; the tool says so, but it is easier to just pass it.
- **A model that writes something too expensive is caught by the same machinery that catches
  you** — thirty measured frames, then the previous procedure comes back at the time it was
  parked at.
- **There is no undo tool.** If the client read the source before it wrote, the previous
  version is in the conversation and "put it back" works — but nothing makes it read first,
  and a write **replaces your file with no backup**. Keep procedures you care about in
  version control, the same as any other source.

**Loopback only, and deliberately.** A venue network is shared and a port that can rewrite
the projector is not something to expose with a flag. Reaching a render machine from a
laptop is `ssh -L 8737:localhost:8737 …`, which is something you do on purpose.

Loopback is **not** on its own a boundary, and treating it as one was a real hole here: a
page on any website can POST to `127.0.0.1` from your browser, and a write does not need a
readable reply to have happened. Requests carrying a cross-site `Origin` are refused. There
is still no authentication, so anything already running on the machine can drive it — which
is the same trust boundary a MIDI port has.

### Keep it

```sh
# The material, content-addressed, as one file
cargo run -p karakuri-cli -- --save-set night01

# The performance: that file, then a tick a frame and every edit between them
cargo run -p karakuri-cli -- --load-set night01 --record-session take1

# Render it back, offscreen, at the size it was played at
cargo run -p karakuri-cli -- --replay take1 --seq frames/
```

A replay reads a `tick` where a live run reads a clock, and the recorded audio where a live
run reads a microphone — so what comes back is the performance and not just the material.

---

## Reference

### Flags

**What to run**

| | |
|---|---|
| `--set L1.kir,L4.kir` | one deck slot. Repeat up to four times |
| `L1.kir L4.kir` | the same, positionally, for one slot |
| `--capacity N` | elements per Set (default 262144) |
| `--param name=value` | a uniform write, applied to every Set |
| `--bind FIELDS` | attach a signal to a parameter — `layer=L1,key=turbulence,signal=energy,range=0.0..3.0` |
| `--watch` | recompile and hot-swap when a `.kir` changes |
| `--demo` | drive itself from a script, for showing rather than playing |

**Size**

| | |
|---|---|
| `--canvas WxH` | what is rendered (default 1920×1080). Fixed for the run, and recorded |
| `--size WxH` | the preview window only (default 1280×720). Refused where there is no window |

**Look**

| | |
|---|---|
| `--tonemap NAME` | `clamp`, `reinhard`, `aces` (default), `agx` |
| `--exposure N` | before the tone map |
| `--budget-ms N` | the frame budget the governor and the hot-swap watchdog judge against |

**Input**

| | |
|---|---|
| `--mcp PORT` | serve the Model Context Protocol on `127.0.0.1:PORT`, so a chat client can rewrite what is playing. See below |
| `--tempo-source CMD` | run `CMD` as a child process and follow the beat it reports. See below |
| `--audio-in DEVICE` | open a microphone. Without it the signal bus answers with invented values at low confidence |
| `--bpm N` | the tempo the grid starts at, and the centre of the octave the tracker looks in |
| `--latency-offset-ms N` | the room's offset, default 20, and it may be negative |
| `--midi-in PORT` | open a control surface |
| `--midi-map FILE` | what each knob and pad does. See `examples/surface.map` |

**Store**

| | |
|---|---|
| `--store DIR` | where Sets, sessions and artifacts live |
| `--save-set ID` | write the current material as a Set file and stop |
| `--load-set ID` | build from one |
| `--record-session ID` | write the timeline as it happens |
| `--replay ID` | render a session back. Needs `--render` or `--seq` |

**Offscreen**

| | |
|---|---|
| `--render FILE` | one frame to a PNG and stop |
| `--seq DIR` | every frame to `DIR/%05d.png` and stop |
| `--frames N` | how many simulation frames (default 240) |

### Keys

**The deck**

| | |
|---|---|
| `0`–`3` | focus a slot |
| `space` | focused slot on air / off air, keeping its `t` |
| `w` | ask for the focused slot to warm off air, or withdraw the request |
| `v` | cycle what the output shows: the mix, then each slot |

**The mix**

| | |
|---|---|
| `[` `]` `\` | focused slot gain down / up / back to 1.0 |
| `;` `'` | focused slot opacity down / up — **the way out of bad material** |
| `m` | cycle the blend mode: add, over, max |
| `t` | cycle the tone map operator |
| `-` `=` `` ` `` | exposure down / up / back to 1.0 |

**Moves**

| | |
|---|---|
| `f` `g` | fade the focused slot out / in |
| `x` | crossfade to the next slot |
| `c` | wipe the next slot in over this one |
| `z` | cycle the wipe's shape |
| `n` | where a fade starts: next bar, next beat, now |
| `j` | how long: 4, 2, 8 beats, or a cut |

**The clock** (needs `--audio-in`)

| | |
|---|---|
| `b` | tap the beat |
| `,` `.` | halve / double the grid and the octave |
| `y` | cycle the focused slot's sync: free, tempo, beat |
| `u` `i` | scrub the focused slot, a quarter beat a press. Beat sync only |
| `o` `p` | latency offset down / up, 5 ms |

**The window**

| | |
|---|---|
| `a` | size the window to the canvas, 1:1 |
| `s` | print the status line |
| `h` | print the keys |
| `esc` | quit |

### The status line

Printed twice a second, and on `s`. One group per slot, then the session:

```
 0 LIVE g1.00 t12.4s o0.50 m0.041 p2.13   >1 prim g1.00 t0.0s over m---- p----
| e0.34 on0.02 c1.00 | lock 128.0bpm heard127.8 c0.81 err+0.004b off20ms | aces exp 1.00 | 59.8 fps
```

**Columns that would say the default are not printed at all** — four slots all reading
`o1.00 add` is four columns of nothing to read in the dark. So an absent `o` means the fader
is up and an absent blend means `add`.

| | |
|---|---|
| `>` | the focused slot |
| `LIVE` `prim` `park` `off` | effective residency — what the engine is doing, not what was asked. `park` is a prime request the governor is holding for want of budget, reconsidered every pass |
| `g` | gain. `g>0.25` means a scheduled move is running to that value; `o>` and `w>` are the same for the fader and a wipe |
| `t` | that slot's simulation time, which is its own and not the session's |
| `o` | opacity, when it is not 1.0 |
| `over` `max` | blend mode, when it is not `add` |
| `T` `B` | tempo or beat sync with the anchor bpm, and for `B` any scrub offset |
| `m` `p` | the slot's mean and peak level. `m---- p----` means nothing of this frame's was drawn — off air and not being auditioned |
| `x` | non-finite texels in that slot this frame — see below |
| `PVW` | which slot is being auditioned, when one is |
| `e` `on` `c` | energy, onset, and how much of the audio to believe |
| `lock` / `free` | whether the beat grid has locked |
| `heard` `err` | what the tracker last estimated, and how far the grid is from it |
| `x2?` | the tracker thinks the grid may be an octave off. It will not fix it; `,` and `.` will |
| `off` | the latency offset |
| `ableton-link 2p` | the tempo source and its peer count, with ` ?` not yet heard from, ` stop` stopped, ` GONE` died, `xN` anchors rejected. The grid's tempo follows it here when there is no `--audio-in` to print it |

The audio group is absent entirely without `--audio-in`.

**`x` is a count and not a fault.** Dividing by something that reaches zero is one of the
most ordinary things a shader does, and it usually shows as a blown-out pixel. Those texels
are excluded from the mean and peak so one of them cannot poison the number; nothing is
reset and nothing is disabled. If a slot is unusable, `;` is the way out and it is yours to
press.

---

## What it will not do

Everything here is known, and none of it is going to surprise you mid-set if you have read
it once.

### The downbeat is arbitrary — **unless a tempo source is attached**

The grid knows how fast beats go and where they are. **On its own it does not know which one
is beat one**, because that cannot be recovered from audio by anything this program does —
downbeat detection is a separate and harder problem and it is not attempted.

Without a source, `bar` in a binding, "the next bar" on `n`, and an 8-beat fade are all on
the right grid at an offset decided by whenever your session happened to start. They are
musical in *length* and arbitrary in *alignment*, by up to three beats. If that matters,
start the session on a downbeat and tap `b` on one.

**`--tempo-source` fixes it, because a shared grid carries a beat *number*.**

```sh
cargo run -p karakuri-cli -- --tempo-source ~/path/to/karakuri-link
```

That command is a separate program — an Ableton Link peer is the first one, so Rekordbox,
Ableton or anything else on the session puts this on the room's bar. It is separate on
purpose: Link is GPL and this is MIT, and a helper that crashes takes its own process with
it rather than the show. See `docs/plugins.md`.

The first anchor **aligns** the grid, however far it has to move, and prints what it did.
Every anchor after that **trims** by at most a twentieth of a beat, and it takes three in a
row disagreeing by more than a whole beat before the grid is moved outright — because the
source is another program from another repository, and a wrong reading should be a wobble
rather than a jump. While a source is attached the beat tracker keeps measuring `energy`,
`onset` and the bands, and stops moving the grid; otherwise the two would fight over the
phase sixty times a second.

**With rekordbox this is much less useful than it sounds, and the reason is rekordbox's.**
Three steps are needed and the third is the problem: turn `[LINK]` on (that only joins the
session), press **SYNC** on the deck you are playing (that puts the deck on the Link
timeline), and then set the tempo from the **Link subscreen** — right-click `[LINK]` to show
it. The deck's own tempo fader does **not** drive Link, by design on Pioneer's side: there
is no master in Link for a fader to be the master of, so rekordbox never publishes the
deck's BPM. A knob can be MIDI-mapped to the subscreen's BPM, but nothing makes it follow
the track.

So with rekordbox the picture does not follow the music by itself; the tempo is something
you set, the way you set everything else here. What you get for setting it is that it is
**shared** — `b` taps a grid on this machine, and a tap in the Link subscreen puts every peer
on it at once, which is what you want the moment there is a second machine or a second
application in the rig. With a peer that *drives* Link, such as Ableton Live, it follows on
its own.

The status line grows a group: `ableton-link 2p` is the source's name and how many other
peers it can see. `0p` means it is running and alone — check the network before you check
this program. ` ?` means it has not said yet, ` stop` that the session is stopped, ` GONE`
that the helper died (the grid holds where it was), and `x3` that it sent three anchors
whose numbers could not be used.

### Nothing follows a control that is not the keyboard

MIDI **in** works and a surface can do nothing a key cannot. MIDI **out** is not built, so a
controller's LEDs and motorised faders do not follow the deck — and two things can move a
fader now, since a transition is one of them. Control changes are read at 7 bits, so a fader
is 128 positions, about 0.8% of its range per step.

### A session cannot say what a deck held

A Set file describes one Set, and there is no record for "this deck held these four". So a
recorded session puts **slot 0's material** at its head, and replaying a multi-slot
performance renders the rest of it — the gains, the blends, the residencies — against a deck
of one, reporting what it skipped. Closing that is a format change.

### The canvas is fixed for a run

Changing it would reallocate every slot's target, and the frame path does not allocate. Set
`--canvas` before you start. Resizing the window is free and changes nothing that is drawn.

### There is no fullscreen and no display picker

The window is a preview, and an OBS capture of it covers the ordinary case — press `a`
first so the capture is the canvas exactly. Anything past that is output routing, which is
deliberately outside this repository; see `docs/plugins.md`.

### Nothing moves by itself

There is no automatic gain, no automatic exposure, no automatic octave correction, and no
panic key. Every measurement is shown and none is acted on. This is a deliberate standing
answer rather than a set of gaps: a control that is *sometimes* the wrong answer is one you
hesitate over, and an anomaly on stage is usually one frame and usually harmless, so an
automatic response does more damage than the thing it responds to. Everything is recoverable
by hand, and the status line tells you which hand to use.
