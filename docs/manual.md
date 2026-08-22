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

Edit either `.kir` and save — **the copies under `.karakuri/scratch/`, whose path is printed
at startup**, not the files you named on the command line. A run that can be edited never
writes to those; see [Where your work lives](#where-your-work-lives). The new procedure is
compiled on a worker thread and swapped in between two frames. Then it is **judged**: eight warmup frames, thirty measured ones, and if
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

Four tools. `read_procedure` gives you the source; `write_procedure` checks it and, if it
compiles, writes it — **and if it does not compile, what comes back is the checker's
diagnostics, against the source**, which is what lets a model fix its own mistake;
`swap_outcome` says whether the result landed, was rolled back for costing too much, or
failed to build. A write returning cleanly means it compiled, not that it is on screen, so
the third tool is where the loop closes.

`save_set` is the fourth, and it is the `k` key reachable from a client: **it keeps what a
slot is playing** as a Set file you can reload with `--load-set`. Give it a `slot`, and an
`id` if you want to name the result — leave the `id` out and it is named after the moment it
was saved, exactly as the key press is. **An `id` you choose overwrites a set already under
that name**, exactly as `--save-set ID` does: a name you typed is an instruction, and nothing
is quietly renamed behind you, so give each keeper its own name or leave the `id` out and let
the clock do it. It writes the material *on screen* and not what is on disk, which is the
same distinction the key makes and is worth re-reading below under [Keep it](#keep-it).
**It waits for the disk before it answers**, so what comes back names the id the set was
written under; a model that has just made something worth keeping can ask for it to be kept
and be told whether that worked, instead of asking a hand to press a key.

Two resources come with it: the IR specification, and a vocabulary page — every built-in,
every topology, and every stage output — **generated from the checker's own tables** rather
than written down beside them. Prose goes stale; those lists cannot, because the same tables
are what reject a procedure.

Nothing here can do anything a key cannot — it is the third control surface after the
keyboard and MIDI, on the same terms. **A set a model rewrote replays with no model
attached**: `--record-session` writes a `procedure` record whenever a swap lands, so
`--replay` rebuilds the slot at the frame it changed on. That was not true when this surface
was first built, and it is the one thing it needed of the format.

Six things worth knowing before you rely on it:

- **It reaches every node of a slot.** A read and a write are addressed `(slot, layer,
  index)`, and the layer is any of `L1`, `L2`, `L3`, `L4` and `Field` — so a deformation, a
  camera, a shape in a file of its own and a slot's *second* geometry are all editable by a
  model, not only the geometry and the renderers. The address space is read off the files'
  own `kind` lines, so a procedure that does not compile is still addressable, which is how
  a model fixes it. A refusal says what the slot actually holds — "holds no L2: a
  deformation is optional", "holds 2 L4 nodes, so index is 0-1".
- **`--watch` is what picks a write up.** Without it the file changes and the screen does
  not; the tool says so, but it is easier to just pass it.
- **A model that writes something too expensive is caught by the same machinery that catches
  you** — thirty measured frames, then the previous procedure comes back at the time it was
  parked at.
- **A write cannot reach the files you named.** It reaches the scratch copy, and every
  version that compiles is kept under `<store>/history/` — including the one the run started
  with. See [Where your work lives](#where-your-work-lives). Before that existed, a model
  replaced three of this repository's own shipped examples in one session, and what saved
  them was that they happened to be in version control.
- **There is still no undo *tool*.** The versions are on disk and nothing walks them for you
  yet: putting one back is copying a file from the history over the scratch. If the client
  read the source before it wrote, the previous version is also in the conversation.
- **A `save_set` that comes back without an outcome is not a `save_set` that failed.** The
  call waits ten seconds for the render loop and the disk, and there are four ways it can end
  without one. They come in two pairs, and which pair you got is written in the answer. If
  the loop **took** the save and then went quiet — ten seconds passed, or the run shut down
  under the call — what comes back names the id it was accepted under and says it is neither
  a success nor a failure: it is being written or it is not, nothing can claim either yet,
  and the thing to do is look under that id. If the loop **never took it** — ten seconds with
  no frame running, or a run already ending — what comes back names no id and says *nothing
  was saved and asking again is safe*, which is the better of the two answers and the one you
  can act on. Do not report a set as kept until a call says it is.

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

**A Set file carries the whole chain.** A slot holding L2s, an L3, one or several
`kind Field` files or several geometries is saved as the chain it is — one `slot` record per node, on the layer
that node's own `kind` declares — and `--record-session` takes it too, since a session opens
with a Set file. Each geometry's own capacity and its own hash salt are written down, and so
is every `--edge`, so a cube morphing into a sphere is a Set you can keep and reload with
the colours and the pairing it had.

What a Set file still leaves behind is the *layering*: `--merge` is not one of the records,
so a saved Set loads as overdraw. That one stays on the command line.

**And you do not have to decide before you start.** `--save-set` writes the material and
stops, which is right for building a preset and no use once the set is running; the `k` key
writes the focused slot as a Set file *from* a running session, named after the moment you
pressed it. See the keys below.

---

## Reference

### Flags

**What to run**

| | |
|---|---|
| `--set L1.kir,L4.kir` | one deck slot. Repeat up to four times |
| `--set near=L1.kir,far=L1.kir,…` | the same, naming the nodes. Any part may be written `name=file.kir`, and a part written bare is named after its procedure — a second use of one procedure becomes `lattice_shell-2`. The name is what `--edge`, a Set file and a rebuild address the node by |
| `--set L1.kir,L4.kir,L4.kir` | the same, drawn twice — one simulation, two renderers over it, in the order given |
| `--set L1.kir,L2.kir,L4.kir` | a deformation between the two. Every path after the first is sorted by the `kind` it declares, so there is nothing new to spell: L2s deform in the order given, L4s draw in the order given |
| `--set L1.kir,L3.kir,L4.kir` | a camera. As many as you name; without one the built-in orbit, which is a node called `orbit` |
| `--merge N` | slot `N` **composites** its renderers instead of overdrawing them — a render target each, folded through a gain, an opacity, a blend mode and a mask per renderer. Costs one frame-sized target per renderer and folds at most four. Without it they share one target and meet through their own blend states, which is what you want for one cloud drawn two ways |
| `L1.kir L4.kir` | the same, positionally, for one slot |
| `--capacity N` | elements per geometry. **Without it each procedure's own declared default is used**, which is what a `.kir`'s `capacity [min, max] = N` line is for; give this and it overrides every source in every slot |
| `--param name=value` | a uniform write, applied to every Set — and within a Set, to every node that declares the name |
| `--param L4:1:name=value` | the same, addressed at one node. How two renderers over one geometry get different values; a bare name cannot, since it reaches both. `L1`, `L2`, `L3`, `L4` and `Field`, and the index is required |
| `--edge NODE.SLOT=NODE` | bind a procedure's declared input to a node of this Set — `--edge morph.far=sphere_shell` for a geometry, `--edge field_lens.shape=melt_blob` for a field, `--edge lens.view=orbit` for a camera, `--edge dissolve.only=lattice_shell` for a source a mask names. A `.kir` that declares `uses far : Geometry`, `uses shape : Field`, `uses view : Camera` or `uses only : Source` names the slot and never which node fills it, so this is where that is said. Both sides are node names. A declared slot nothing binds is refused rather than guessed at, and so is one bound to the wrong sort of node |
| `--publish NAME=key[LOW..HIGH]` | put one control on the console under a name the Set chose, over part of its declared range. Without any, every control is published — the first `--publish` makes the list *the* list. It narrows and never widens: a range outside what the procedure declared is refused |
| `--publish NAME=L4:0:key[LOW..HIGH]` | the same, addressed at one node rather than every node declaring the key |
| `--bind FIELDS` | attach a signal to a parameter — `layer=L1,key=turbulence,signal=energy,range=0.0..3.0`. `layer`, `key`, `signal` and `range` are required; `index=N`, `curve=lin\|pow2\|sqrt\|smooth` and the `noise.*` fields are optional. `--help` lists them all |
| `--bind signal=control:NAME` | drive it from a **published control** instead of a signal. This is what a macro is: one knob on the desk moving several internal controls, each through its own curve and range. Publish first — a binding on a name nothing publishes is reported and dropped, and the run continues without it |
| `--watch` | recompile and hot-swap when a `.kir` changes |
| `--store DIR` | where the library, the scratch and the edit history live (default `.karakuri`) |
| `--demo NAME` | drive itself from a script, for showing rather than playing. `transport` scrubs the beat clock; `lines` draws one L1 as sprites and as strokes and brings its own two-slot deck. Both loop |

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
| `--budget-ms N` | the frame budget the hot-swap watchdog judges a candidate against, default 20. The governor's priming budget is a different quantity — measured per-Set cost against a 16.7 ms compute budget — and is not on a flag |

**Input**

| | |
|---|---|
| `--mcp PORT` | serve the Model Context Protocol on `127.0.0.1:PORT`, so a chat client can rewrite what is playing, and keep it. See below |
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
| `--save-set ID` | write the material as a Set file and stop — the whole chain, at the capacity and the salt the run would have drawn with |
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

**The clock**

| | |
|---|---|
| `y` | cycle the focused slot's sync: free, tempo, beat |
| `u` `i` | scrub the focused slot, a quarter beat a press. Beat sync only |

**The room** (needs `--audio-in`)

| | |
|---|---|
| `b` | tap the beat |
| `,` `.` | halve / double the grid and the octave |
| `o` `p` | latency offset down / up, 5 ms |

**The window**

| | |
|---|---|
| `a` | size the window to the canvas, 1:1 |
| `k` | **keep the focused slot** — write what it is playing right now as a Set file |
| `s` | print the status line |
| `h` | print the keys |
| `esc` | quit |

**`k` is the one that saves your work mid-set.** It writes the focused slot's material as a
Set file named after the moment you pressed it — `20260816-143052-271`, local time to the
millisecond, on the same convention the edit history uses and for the same reason: a key
press cannot type a name, so the thing you will look for it by is when you saved it. The
line saying where it went arrives a moment later, because the file is written on a thread of
its own rather than on a frame. Quitting before it arrives does not lose it: the run waits up
to five seconds for a save still being written, and says so if it gives up.

**The same save is the `save_set` tool over MCP**, which is the one control in this system a
hand, a model and the interface M5 builds all want. It is one control and not three: a tool
call names the slot instead of using your focus and may name the file, and everything after
that — what is read, what is refused and in whose words, what goes into the session stream —
is the same code path the key press takes. The one thing a call can do that the key cannot is
name the file, and that carries `--save-set`'s rule with it: **a name given twice overwrites**
the set already under it, while a stamped name cannot collide.

What it records is **what is on screen**, and that is the whole difference from `--save-set`:

|  | `--save-set ID` | `k` |
|---|---|---|
| when | before a run, and the run then stops | during one, as often as you like |
| named by | you, and a name reused overwrites | the clock, or an MCP call — which also overwrites a name it reuses |
| which slot | slot 0 | the focused one, or the one an MCP call names |
| the values | what the flags say | what the Set is playing — a param you moved, a capacity or a salt a reloaded Set brought with it |
| the sources | the files named on the command line, read now | the versions **on screen**, by content hash |

That last row is the one worth understanding, and it is why `k` never reads a `.kir` at save
time. A path and the picture it produced come apart in three ordinary ways. Under `--watch` a
build that compiles and is then refused for costing too much leaves the newer `.kir` sitting
on disk. An edit that does not compile stays on disk until you fix it, and the slot goes on
drawing the last version that did. And without `--watch` nothing picks a file up at all, so
anything that rewrites one — your editor, or an `--mcp` client with no watcher behind it — is
invisible to the run for as long as it lasts. **In all three, what `k` writes is the
picture.** The run keeps the text it compiled and addresses every node by its content hash
before the first frame, and every build that lands is stored and addressed the same way — so
what a slot is running is a hash from the first frame to the last, which is also the only
reason it can be saved at all when the bytes on screen are on no disk under any name. There
is a fourth way a path stops being true and it is the one `k` never sees: a `.kir` rewritten
in the seconds between the compile and the first frame. The bytes the deck was built from are
the ones it kept.

**One slot cannot be kept**: one filled by `--load-set` in a run with **neither `--watch`
nor `--mcp`**. Its material came out of the store by hash with no files behind it and nothing
in the run that could rebuild it, so `k` refuses and names the set it came from. Either flag
at startup is enough — both put the Set's procedures into the scratch as real files — and
that slot then saves like any other.

Reload it exactly as you would any other preset:

```sh
cargo run -p karakuri-cli -- --load-set 20260816-143052-271
```

`--merge` is still not one of the records, so a saved Set loads as overdraw whichever way it
was written.

### The status line

Printed twice a second, and on `s`. One group per slot, then the session:

```
 0 LIVE g1.00 t12.4s o0.50 m0.041 p2.13   >1 prim g1.00 t0.0s over m---- p----
| e0.34 on0.02 c1.00 | lock 128.0bpm heard127.8 c0.81 err+0.004b off20ms | ACES exp 1.00 | 59.8 fps
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
| `key=value` | what each binding on that slot last wrote, one column each. Absent when the slot has no bindings |
| `ableton-link 2p` | the tempo source and its peer count, with ` ?` not yet heard from, ` stop` stopped, ` GONE` died, `xN` anchors rejected. The grid's tempo follows it here when there is no `--audio-in` to print it |

The audio group is absent entirely without `--audio-in`.

**`x` is a count and not a fault.** Dividing by something that reaches zero is one of the
most ordinary things a shader does, and it usually shows as a blown-out pixel. Those texels
are excluded from the mean and peak so one of them cannot poison the number; nothing is
reset and nothing is disabled. If a slot is unusable, `;` is the way out and it is yours to
press.

---

## Where your work lives

Three places, and only one of them is written to.

| | where | who writes it |
|---|---|---|
| **App presets** | `examples/` | nobody. They ship with the program |
| **Your presets** | `<store>/sets/<id>.set.ndjson` | `--save-set`, and the `k` key |
| **Scratch** | `<store>/scratch/` | `--watch`, `--mcp`, and your editor |

**A run that can be edited copies its material into the scratch and runs from
the copy.** So `karakuri-cli --watch examples/drift_shell.kir examples/soft_points.kir`
never writes to `examples/`, and neither does a model over MCP. The path is
printed at startup — **that is the file to open in your editor**, not the one you
named on the command line:

```
scratch: .karakuri/scratch — the deck runs from copies here, so the files you
         named are not written to. Point an editor at these
```

Two slots naming one file still share one scratch file, so an edit to it moves
both, exactly as before.

A run that *cannot* be edited — `--render`, `--seq`, `--replay`, or a window with
neither `--watch` nor `--mcp` — copies nothing and creates no directory. It opens
every file read-only, so there is nothing to protect them from.

**`k` is the one thing that changes that, and only once you press it.** Such a run
holds the text it compiled in memory and knows what every slot is playing by its
content hash from the first frame; the bytes go into `<store>/<hash>.kir` at the
moment a Set file names them, and not before. So the store appears when you keep
something, and a run you never saved from leaves nothing behind at all. (A run
with `--watch` or `--mcp` opens the store at startup either way — the scratch
lives in it.) `--record-session` is the other writer: a session has to be
replayable, so it puts every slot's starting material in the store before the
first frame, whether or not anything is ever saved.

**A Set loaded with `--load-set` is materialised here too**, under its procedures'
own names, even though it has no `.kir` anywhere — it names them by hash and the
sources come out of the store. So a saved Set can be watched, edited and driven
over MCP like any other material:

```sh
cargo run -p karakuri-cli -- --load-set night01 --watch --mcp 8737
```

That used to be refused, because there was nothing on disk for a model to read.

**The other end of that loop is the `k` key**, and the `save_set` tool beside
it. `--save-set` writes what the flags say and exits; `k` writes what the focused
slot is playing, right now, into `<store>/sets/` under a timestamp, and a model
asks for the same thing by naming a slot. So load, edit, watch — by hand or by
asking — and keep the version you liked without leaving the run.

### The edit history

Every version that **compiles** is kept, whoever wrote it:

```
.karakuri/history/2026/08/16/143052-271_slot0_L4_beat_strokes.kir
```

Including the version the run started with, whether or not it compiles — that one
is snapshotted before anything is parsed, so the first edit is undoable and not
only the second, and a run started from a broken file can still be walked back
to it. Its layer is read off the file, so an L2's starting version is filed as an
L2 and joins the same chain its later versions land in. Including versions that compiled and were then rolled back
for costing too much — those are the ones a session recording does *not* have,
because it only records what reached the screen.

A procedure that did not change is not written again, so a day's directory is
the edits and not the rebuilds.

**There is no retention policy and no cleanup command, on purpose.** A directory
per day means `rm -rf .karakuri/history/2026/07` is the cleanup. The date is
your local date, so the directory is named the day you would call it.

What is not built yet is anything that *walks* the history — undo is a surface
that reads these files, and the surface is a later milestone. Today it is a
directory you open.

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

### There are three ways to draw and two ways to blend

`Topology` has three values — `points`, `lines` and `fullscreen` — and `Blend` has two.

**`blend additive`** adds colour and occludes nothing. Everything glows, nothing is in front
of anything, and it needs no sorting — which is why it is where this started.

**`blend weighted`** is order-independent transparency: material in front hides material
behind it, without a sort, at any element count. **It is not a drop-in swap.** `additive`
sums past what coverage would allow, which is what makes emissive material read as light;
`weighted` treats alpha as opacity, so a procedure written for one usually wants its exposure
and its alpha reconsidered for the other. `examples/glass_shell.kir` is the one to read.

A fragment's weight is how near the eye it is, measured between the **camera's own near and
far planes** — so moving `far` moves every weighted fragment's weight. That is not a defect;
it is what tying the weight to the frustum means, and it is the operator's lever on how hard
the depth ordering reads.

**Lines cost an L4 and nothing else.** A renderer draws segments by assigning `clip_b`, a
second clip-space endpoint, alongside `clip`; leave it out and it draws sprites. The two
ends of a segment both belong to one element, so the far end is whatever arithmetic that L4
does on attributes it already consumes — a position pushed back along its velocity is a
motion streak, a point on a curve one step behind is a strand, the origin is a burst.
Nothing about the geometry has to change, which is why one L1 file can be on screen twice,
drawn two ways:

```
karakuri-cli --set examples/drift_shell.kir,examples/soft_points.kir,examples/drift_streaks.kir
```

A path in that list may also be an **L2**, which deforms the geometry between the
simulation and the draw:

```
karakuri-cli --set examples/drift_shell.kir,examples/swirl_warp.kir,examples/soft_points.kir
```

An L2 may also **make more elements than it was given**, which is the one thing in the
language that changes a count. `amplify 6` on its header turns every element reaching it into
six, and `copy` inside the block is which of the six this one is — so a kaleidoscope, an
instancer or a trail is a deformation rather than a second simulation:

```
karakuri-cli --set examples/drift_shell.kir,examples/kaleidoscope.kir,examples/soft_points.kir
```

**One simulation, six draws.** The copies share their parent's `seed`, so anything derived
from it — a colour, a phase — is the same in all six and they read as one object; telling
them apart is the deliberate act of writing `copy` down. The cost is a product: the body runs
six times per element and the estimate says so, so an expensive deformation is refused at a
factor the same body would pass at.

or an **L3**, which is the camera:

```
karakuri-cli --set examples/drift_shell.kir,examples/beat_jump.kir,examples/soft_points.kir
```

### Two geometries in one Set

More than one L1 in a slot is a **merge**: each simulates on its own and the renderers draw
all of them.

```
karakuri-cli --set drift_shell.kir,beat_shell.kir,soft_points.kir
```

**Each source counts its own `seed` from zero**, so `seed % 64u` lays out a lattice the same
way in both — a shared counter would put the second one somewhere else. And **each has its
own hash salt**, so the same file used twice comes out in two colours without your arranging
it. That is the default rather than something to set up.

**Which source gets which colours follows the order you spelled the paths in — until you save
the Set.** `--save-set` writes down what each source was running at, so a Set you saved comes
back as the Set you saved: reload it, reorder what is in it, and the colours stay with the
geometry they were on.

Each source runs at the capacity *it* declares, and `--capacity` overrides all of them.
`--param L1:1:spawn_rate=…` addresses the second source; a bare `--param spawn_rate=…`
reaches both.

**Every node in a slot has a name**, written or derived: `--set near=lattice_shell.kir,
far=sphere_shell.kir,morph.kir,soft_points.kir` names the two geometries, and a `--set` that
names nothing gets each node named after its procedure — a second use of one procedure
becomes `lattice_shell-2`. The run prints the list it settled on.

A slot with two geometries rebuilds under `--watch` like any other, and MCP reaches every
node in it. A Set file carries the whole of it — a `slot` record per node, on the layer the
node's own `kind` declares — so a chain or a second geometry is saved and recorded like
anything else.

Two sources and two renderers under `--merge` is **two pipelines composited, published as one
control**:

```
karakuri-cli --merge 0 --publish 'level=exposure[0..2]' \
  --set drift_shell.kir,beat_shell.kir,soft_points.kir,drift_streaks.kir
```

One fader moves all four — every source drawn by every renderer — because a published control
without an address reaches every procedure declaring the name.

### Morphing one geometry into another

An L2 that declares `uses far : Geometry` takes **two** geometries and produces one, matching
their elements by slot index:

```
karakuri-cli --param L2:0:k=0.5 \
  --set lattice_shell.kir,far=sphere_shell.kir,morph.kir,soft_points.kir \
  --edge morph.far=far
```

**Two halves, and they are deliberately apart.** `morph.kir` says it takes a geometry it
calls `far` and never says which one — a `.kir` that named a node would be tied to one Set
and could be used in no other. `--edge morph.far=far` is where you say which: the node with
the slot on the left of the `.`, the slot after it, the geometry it is bound to after the
`=`. The name on the right is a node's — one you wrote with `far=sphere_shell.kir`, or the
one derived from the procedure when you wrote none, which is `sphere_shell` here.

**An unbound slot is refused**, with a sentence naming the slot and listing what the Set
holds. There is no "if there's exactly one, use it": that rule is what used to make the far
geometry `--set` position 1, written down nowhere, so reordering the command line changed the
picture in silence.

`k` is an ordinary `param`, so a fader, a signal binding, a transition and a published control
all reach it. At 0 you see the geometry the chain runs over, at 1 the one the edge names, and
in between every element is on its way.

**Both sources have to be still.** No `spawn` block and no `kill()` in either — a spawn
allocates and a kill compacts, and after a compaction element 5 of one geometry is not
element 5 of the other, so the pairing would match each element with a stranger. The Set
refuses the pair by name rather than drawing that.

They also have to be the same size, and the bound one is **never drawn on its own**: such a
Set is one geometry made of two simulations, with one chain and one set of renderers over it.
Which one is drawn is whichever the edge did not name, so the order the files are listed in
decides nothing.

`--save-set` writes the edge into the Set file as an `edge` record and `--load-set` reads it
back, so a morph is a Set you can keep.

### Treating one source differently from another

**One `.kir` runs once per geometry, and it can now tell which one it is in.** A Set
instantiates its chain per source, so a deformation in a Set of three is running in three
places — `source` is the identity of the one this invocation is in:

```
proc dissolve {
  kind L2

  uses only : Source

  consumes position, size

  mask {
    strength = 0.0;
    if source == only { strength = 1.0; }
  }

  deform { size = size * 0.2; }
}
```

```
karakuri-cli --set lattice_shell.kir,keeper=sphere_shell.kir,dissolve.kir,soft_points.kir \
  --edge dissolve.only=keeper
```

**The comparand comes through a slot for the reason the geometry did**: a `.kir` may not
name a node, so `dissolve.kir` says it wants *a* source called `only` and the Set says which.
`--edge dissolve.only=keeper` is the same spelling as `--edge morph.far=sphere_shell`, an
unbound slot is refused the same way, and a Set file records it as the same `edge` record.

**`: Source` is not `: Geometry`, and the difference is what gets bound.** A `Geometry` slot
binds the far geometry's *elements*, so a node has at most one and reads it as
`far.position`. A `Source` slot binds only the number that identifies a geometry, so it costs
nothing, several are legal — `if source == a || source == b` — and it is read as a bare
value. A mask wants the identity and reads no elements, which is exactly the case the second
type exists for.

**It is not confined to a mask or to an L2.** `source` and a `Source` slot are readable in an
L1, an L2 and an L4 — anywhere a chain instance runs — so a generator that lays one geometry
out differently, or a renderer that tints one of them, is the same two lines in a different
block. They are refused in an L3 and in a `kind Field` procedure, which run over no geometry
at all, and in any procedure that also declares a `uses … : Geometry` slot, where a pairing
Set would make "which source" a question with two answers and one of them silent.

**No value is added to the elements to make this work.** `source` is the geometry's own hash
salt — the same thing that gives it its own colours — which has been in every procedure's
uniform block all along. So a comparison against it is one uniform against another: the same
answer in every lane, and the cheapest branch a GPU has.

### A shape in a file of its own

A path in the list may also be a **field** — a signed distance function, and nothing else.
It draws nothing and holds no elements; it is a shape, and whoever wants one evaluates it:

```
karakuri-cli --set examples/drift_shell.kir,examples/melt_blob.kir,examples/field_lens.kir \
             --edge field_lens.shape=melt_blob
```

`field_lens.kir` is a renderer that **contains no shape at all**. It marches whatever the Set
gives it, so the same file draws any field — and `melt_blob.kir` is a shape no renderer owns.
Before this, a marcher carried its distance function inline and the two were inseparable.

**The renderer says what it takes and you say which one it gets.** Its header declares
`uses shape : Field` and its body calls `shape(p)`; the `--edge` above is where the Set
answers. That is the same rule `--edge morph.far=sphere_shell` follows and it exists for the
same reason: a `.kir` that named a node would be coupled to one Set and would stop being a
file you can reuse. **A slot nothing binds is refused** rather than filled in from whatever
field happens to be in the list.

> It used to be `field(p)` — one reserved word, so one field, since a second would have had
> nothing to be called. Files written against it need one `uses` line and one renamed call.
> Naming the call is what let a Set hold more than one of them.

Its `param`s are yours to ride like any other — an override, a fader, a signal
binding, a published control — addressed by its kind:

```
karakuri-cli --param Field:0:blend_k=1.2 --set ...
```

**As many fields as you name.** A `--set` list may hold several `kind Field` files; each is a
node with a name, and the `--edge` beside it says which slot gets which — so a marcher taking
a shape and a cutter takes two files, and `--param Field:0:blend_k` and `--param
Field:1:blend_k` are two knobs. **The camera followed** — see [Two cameras at
once](#two-cameras-at-once) — and the sentence that said it would not was about the plumbing
rather than about the picture.

**The cost is the renderer's**: a field is
inlined wherever it is evaluated, so a marcher that samples it forty times pays for it forty
times — and a field that fits on its own and a marcher that fits on its own can still be
refused together, with both figures in the message.

### Two attributes you do not have to emit

A renderer that wants `age` or `velocity` no longer needs an L1 that thought to emit them.
Both are synthesised where nothing does: `age` from the instant the element was spawned, and
`velocity` from how far it moved last step. So a motion-streak renderer pairs with any
geometry that emits `position`, which is the point — a renderer that could only be used with
the one L1 written alongside it is not a library.

Nothing is declared and nothing is switched on. An L1 that *does* emit one of them keeps its
own value, and the two never both apply. The storage is paid only where something actually
asks: a Set whose renderers never mention `age` carries nothing for it.

**A derived `velocity` is the simulation's, not the chain's.** It is written where the step
is — in the L1 — so a deformation below it moves the elements without changing it. That is
the reading to want: a `velocity` is how the material is *moving*, and a warp is not motion,
it is where the material is being put. An L2 that wants the other reading emits its own.

Every other attribute is unchanged — consuming `normal` over geometry that does not emit it
is still refused, by name, at build. There is no rule for it and there is not going to be
one: a normal is a property of a surface, and a point cloud has no surface to take it from.


There is nothing to spell for either. Every `.kir` declares its own `kind`, so the first path
is the geometry and the rest are sorted by what they say they are — L2s deform in the order
given, L4s draw in the order given, and a slot takes as many cameras as it is given.

**Without an L3 the camera is a slow orbit**, which is what every example is written to look
right under. An L3 replaces it for that slot: `beat_jump` cuts to a new angle on the beat and
keeps facing the centre. It needs no state to do that — the angle is a hash of the beat
number, so the camera is *seekable*, and scrubbing the transport puts it exactly where it
would have been.

### Two cameras at once

A `--set` list may hold **several** `kind L3` files, and each renderer says which one it
draws from — the same shape a second geometry and a second shape already had:

```
karakuri-cli --set examples/drift_shell.kir,examples/beat_jump.kir,\
examples/soft_points.kir,examples/second_eye.kir \
             --edge second_eye.view=orbit
```

draws one cloud twice in one frame: `soft_points` from `beat_jump`, which cuts on the beat,
and `second_eye` from the slow built-in orbit.

A renderer that wants to say which camera it draws from declares `uses view : Camera` and
reads the projection through it — `clip = view.clip * vec4(position, 1.0)`, and `view.eye`
and `view.ray` in a marcher. `examples/second_eye.kir` is `soft_points` with exactly that one
line added. **A renderer that says nothing draws from the Set's camera**, which is the first
one: every other example is written that way and none of them moved.

**The built-in orbit is a node too, and it is always there** — the last one, called `orbit`,
whatever else the list holds. That is what the `--edge` above points at, and it is why a Set
that names two `kind L3` files holds three cameras: `L3:0`, `L3:1` and the orbit at `L3:2`.
Two renderers naming one camera is ordinary — one viewpoint drawn two ways. What is refused
is a slot bound twice, a slot nothing binds at all, and a slot bound to a node that is not a
camera.

**A deformation can apply partially**, in two ways that multiply into one:

- **`weight`** is a `param` like any other, so it goes on a fader, takes a `--bind`, and can
  be published — **when the modulator declares one.** It is a name the layer gives a meaning
  to rather than one every L2 has: `late_bloom` declares it and `swirl_warp` does not, and
  `--param L2:0:weight=0.3` on the second is reported and ignored like any other name nothing
  declares.
- **A `mask` block** decides *where*, per element, from the attributes reaching it —
  `strength = smoothstep(0.2, 0.6, age)` blooms only what is old. This is where most of the
  expressive range in a chain is: `examples/late_bloom.kir` is three lines of `deform`, and
  masked on `age`, on `seed`, or on distance it is three different effects.

They stack, and in either order, because an L2 keeps nothing between frames:

```
karakuri-cli --set examples/drift_shell.kir,examples/swirl_warp.kir,examples/late_bloom.kir,examples/soft_points.kir --param L2:1:weight=0.3
```

**One slot, one simulation, two renderers over it**, drawn in the order given. The passes
run over one render target — the first clears it, the rest load what is there — so the
second renderer costs a draw pass and no memory at all.

Two `--set` flags instead would be two slots, which is a different thing and sometimes the
thing you want: two slots have their own faders, their own blend into the mix, and their own
residency. They also cost **two simulations** of the same procedure. Reach for a stack when
you want one cloud shown two ways; reach for two slots when you want to mix between them.

Three things worth knowing before you ask for strokes. `point_size` becomes the stroke's
**width in pixels**. `point_coord` runs **along** the segment in x and **across** it in y,
so a soft edge is `abs(point_coord.y * 2 - 1)` where a sprite would use
`length(point_coord * 2 - 1)`. And a stroke covers far more texels than the sprite it
replaces, so **the same exposure is much brighter** — `drift_streaks` sits a factor of ten
under `soft_points` for that reason alone.

One more, and it is the one that will look like a bug: **a segment whose two ends coincide
draws nothing.** A sprite at zero velocity is still a sprite; a stroke of zero length is
zero-area and gone. So a parameter that scales the gap between the ends must not reach zero,
and `drift_streaks`' `streak` starts at 0.05 rather than 0 for exactly that reason.

**A fullscreen renderer has no `vertex` block at all.** That is the whole declaration: a
procedure with no per-element position has nothing for one to do. It gets two values nothing
else does — `eye` and `ray`, the camera's position and the direction through this fragment —
and `point_coord` runs across the frame. `examples/field_march.kir` marches a sphere, a box
and a torus with the SDF builtins that had no renderer until now:

```sh
cargo run -p karakuri-cli -- examples/drift_shell.kir examples/field_march.kir --capacity 4096
```

Two things about it worth knowing. It **consumes nothing** — there is no element to read
from — and because of that **the L1 it is paired with is never stepped**, so pair it with
something small and cheap; only that file's `capacity` line is read. And its fragment budget
is eight times a sprite renderer's, because a fullscreen pass covers the canvas once where
sprites overdraw an unknown number of times.

**`blend weighted` is how material stops glowing and starts hiding things.** It is the
other value of the L4 header's `blend`, and unlike the topology it is a declaration rather
than something the checker infers — nothing a procedure writes could imply it, because the
two modes differ in how the results of identical assignments are combined:

```sh
cargo run -p karakuri-cli -- examples/drift_shell.kir examples/soft_points.kir   # additive
cargo run -p karakuri-cli -- examples/drift_shell.kir examples/glass_shell.kir   # weighted
```

Same L1, byte for byte. The first is a lamp — every sprite adds, the brightest part of the
shell is wherever the most of them overlap, and nothing is ever in front of anything. The
second is a solid.

**The one thing to know as an author is what `color`'s alpha means.** Under `additive` it is
emission strength: it scales what the fragment adds, and values above 1.0 are expected the
way they are for colour. Under `weighted` it is **opacity** — 0 is glass, 1 is paint — and
it is clamped to `[0, 1]`, because the revealage a weighted pass accumulates is
`prod(1 - a)` and that stops meaning anything the moment a term goes negative. Writing 1.5
does not make material brighter; it makes it exactly 1.

**Exposure and opacity separate, where additive fuses them.** Under additive, doubling the
exposure and doubling the alpha do the same thing to the picture. Under weighted they are
two controls: how bright the material is, and how much of what is behind it survives. A dim
opaque shell and a bright transparent one are different pictures now.

**It does not sort, and it is not trying to.** Weighted blended OIT resolves toward the
nearer fragment, weighted by where it sits between the camera's near and far planes — so
material occupying a thin slice of a wide frustum gets near-equal weights and reads closer
to an average than to a sort. The occlusion is what changes the picture; the ordering is a
refinement on top of it. Being order-independent is also what lets it exist at this scale at
all: nothing is sorted, so nothing fights compaction.

Two costs. It allocates **two more render targets per weighted slot** — 7.03 MB of
accumulation plus 1.76 MB of revealage at 1280x720, so 8.79 MB the pair, on top of the
7.03 MB every slot already has, and nothing at all for an additive slot. A slot under
`--watch` holds more than one Set at a time (the live one, the candidate on trial, and up to
four waiting to be freed), so a weighted slot mid-swap is a multiple of that. And a
**fullscreen L4 may not declare it**: one fragment per texel makes the resolve give back
exactly what additive accumulates, so the Set refuses to build rather than charge for the
identity. The diagnostic says so.

**The old workaround still runs, and it is worth knowing what it cost.** Before `lines`
existed, asked for line art, a model found that points laid densely along a curve read as
strokes — taking hue and width from the strand rather than the element, or every sample gets
its own colour and the line reads as noise. That is `examples/strand_shell.kir`, kept as
written. It works, and it spends roughly twice the elements a segment would. Expect a model
to find things like that by probing the compiler: a procedure that does not compile is never
written, so guessing at the language is free, and the diagnostics carry hints that say *why*
rather than only what.

### Nothing moves by itself

There is no automatic gain, no automatic exposure, no automatic octave correction, and no
panic key. Every measurement is shown and none is acted on. This is a deliberate standing
answer rather than a set of gaps: a control that is *sometimes* the wrong answer is one you
hesitate over, and an anomaly on stage is usually one frame and usually harmless, so an
automatic response does more damage than the thing it responds to. Everything is recoverable
by hand, and the status line tells you which hand to use.
