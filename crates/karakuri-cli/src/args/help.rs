pub(crate) const USAGE: &str = "\
karakuri-cli — a window, or a PNG

usage:
  karakuri-cli [options] [L1.kir L4.kir]

sets — one deck slot each, composited in the order given, at most 4:
  --set L1.kir,L4.kir   name one slot. Repeat for more slots. Every path is
                        sorted by the `kind` it declares: another L1 is a
                        second geometry, an L2 deforms, an L3 is the camera, a
                        `kind Field` is a shape the others can call, an L4
                        draws. Order within a kind is the order given. One
                        camera per slot, and as many fields as you name. Any
                        part may be written `name=file.kir`, which is what an
                        --edge points at; a part written bare is named after
                        its procedure
  L1.kir L4.kir         the same thing, positionally, for one pair. Given
                        alongside --set it becomes the last slot
  (nothing)             examples/coil_vortex.kir + examples/star_flares.kir

options:
  --render FILE         render one frame offscreen and stop
  --seq DIR             render every frame to DIR/%05d.png and stop
  --frames N            how many simulation frames (default 240)
  --canvas WxH          what is rendered (default 1920x1080). Fixed for the
                        run, and recorded, so a replay is at the size it was
                        performed at
  --size WxH            the preview window only (default 1280x720). The window
                        fits the canvas into itself and has no say in it.
                        Refused with --render, --seq or --replay
  --capacity N          elements per geometry, overriding every source. Without
                        it each L1 runs at the default its own `capacity`
                        declaration names, and 262144 where a file names none
  --param name=value    a uniform write, applied to every Set, and within one
                        to every node declaring that name
  --param L4:1:name=value
                        the same, addressed at one node — which is how two
                        renderers over one geometry get different values
  --bind FIELDS         attach a signal to a param, applied to every Set.
                        Comma-separated `field=value`, one per field of the
                        `bind` record:
                          layer=L1 key=turbulence signal=energy
                          index=1 (optional; without it, every node of
                            that layer declaring the key)
                          curve=lin|pow2|sqrt|smooth  range=LOW..HIGH
                          noise.kind=white|value|perlin|fbm
                          noise.rate=N  noise.stream=N
                          noise.octaves=N   (needs noise.kind=fbm)
                        layer, key, signal and range are required.
                        signal=bpm is refused: a tempo is not a [0,1] signal
                        and the binding would never move — bind beat or bar
  --edge NODE.SLOT=NODE bind a procedure's declared input to a node of the
                        Set: `--edge morph.far=sphere_shell` for a geometry,
                        `--edge field_lens.shape=melt_blob` for a field. A
                        `.kir` that takes one names the slot and never which
                        node fills it, so this is where that is said — and a
                        slot nothing binds is refused rather than guessed at,
                        as is one bound to the wrong sort of node. Both sides
                        are node names: one you wrote with
                        `--set far=file.kir`, or the procedure's own where you
                        wrote none
  --publish NAME=SPEC   put one control on the console, over a param or a
                        node's param: `level=exposure[0..2]` or
                        `level=L4:0:exposure[0..2]`. Repeat for more. An
                        interface that publishes nothing publishes everything,
                        so the first --publish is what narrows the console
  --merge N             composite slot N's renderers into one image before it
                        reaches the mix, instead of overdrawing them. The slot
                        then takes per-renderer gain, opacity, blend and mask.
                        A Set file records this, so a composited Set saved with
                        --save-set or `k` loads back compositing without the
                        flag — and this flag can only turn it on, so either
                        saying so is enough and the two never disagree
  --bpm N               the tempo the local oscillator free-runs at
                        (default 120). With --audio-in this is where the grid
                        starts and what it falls back to; a tracked tempo
                        corrects it rather than replacing this flag. It is also
                        where the tracker's one-octave window starts, so a value
                        within about 40% of the real tempo settles the octave —
                        and `,`/`.` are the fix when it does not
  --mcp PORT            serve the Model Context Protocol on 127.0.0.1:PORT, so
                        a chat client can read a slot's procedure, rewrite it,
                        and be told what the compiler and the frame budget
                        made of it. **Loopback only, deliberately**: reaching a
                        render machine from elsewhere is `ssh -L`, which is a
                        thing an operator does on purpose. Best with --watch,
                        which is what picks a written procedure up
  --tempo-source CMD    run CMD as a child process and follow the beat it
                        reports. A shared grid carries a beat *number*, so
                        `bar` becomes the room's bar rather than one counted
                        from when this started — which a beat tracker cannot
                        do, because a downbeat is not recoverable from audio.
                        The first anchor aligns the grid and every one after
                        it is trimmed, because the source is another program;
                        while one is attached the beat tracker keeps measuring
                        and stops moving the grid. See docs/plugins.md
  --audio-in NAME       open an audio input: `default`, or any part of a
                        device's name. `energy`, `onset` and band0..7 become
                        measured signals at full confidence, and the beat is
                        tracked and corrected onto the local oscillator
  --latency-offset-ms MS
                        how far the picture leads the sound, beyond the frame
                        queue (default 20, signed). Everything past the two
                        outputs — the PA, the projector, the room — which
                        nothing here can measure. Negative when the sound is
                        the late one. `o`/`p` nudge it live, which is how it is
                        meant to be found: from where the audience stands
  --midi-in NAME        open a MIDI input: any part of a port's name, or an
                        empty string for the first one there is. Every knob
                        goes through the same records a key press writes, so a
                        surface can do nothing a key cannot and a session
                        recorded from one replays with neither attached
  --midi-map FILE       what each knob and pad does, one per line:
                          cc 1 ch 1 -> gain 0        cc 20 -> exposure
                          cc 5      -> opacity 0     note 32 -> residency 0 live
                          note 36 -> residency 0 priming
                          note 40 -> residency 0 allocated
                          note 44 -> blend 0 over    note 48 -> tap
                        A pad names a state, never a step, so a state is a pad:
                        `residency N live|priming|allocated`, `blend N
                        add|over|max`. A file written against the older
                        `on-air N`, `prime N` or bare `blend N` is refused on
                        that line with the line to write instead.
                        `examples/surface.map` is this filled out for four
                        slots, with the reasoning; copy it and edit.
                        `ch` is the number printed on the device, 1-16, and is
                        optional — without it a mapping answers on every
                        channel. A continuous control takes an optional range,
                        `-> gain 0 [0, 2]`, defaulting to [0, 1] for the faders
                        and [0.25, 4] for exposure. Optional even with a port:
                        without a map, every message prints the line that would
                        map it, which is how a surface is discovered
  --tonemap OP          clamp | reinhard | aces | agx (default aces)
  --exposure V          output exposure, before the tone map (default 1.0)
  --store DIR           where artifacts and Set files live (default .karakuri)
  --list-sets           print what the store holds — one line per Set: its id,
                        when it was saved, and how many nodes on each layer —
                        and stop. Nothing is compiled and no window opens
  --save-set ID         put every `.kir` of slot 0 in the store, write the
                        material as a Set file, and stop. It records what the
                        run was *drawing* rather than what the flags said: a
                        capacity and a salt per geometry, the params, the binds,
                        the camera, and an `edge` per bound slot
  --load-set ID         take the material from a Set file rather than from paths
                        and flags — the whole chain, names and edges included.
                        A flag given beside it wins. Anything the file could not
                        carry is printed rather than dropped in silence
  --package ID|FILE     write the Set filed under ID to standard output with
                        every source it names inlined as `src` records, and
                        stop. That file loads on a machine whose store has
                        never held the material, so it is what you send
                        somebody: `--package night01 > night01.kbset`. An
                        artifact this store does not hold refuses the whole
                        bundle naming it, rather than producing a file that
                        looks self-contained and is not.
                        Given a FILE ending in .kset instead — an authoring
                        Set file, which names its .kir files by relative path
                        and lives beside them — every part is read, hashed and
                        put in the store, and the bundle is written from that:
                        `--package night01.kset > night01.kbset`. A part that
                        reaches outside the .kset's own directory is refused
                        by name, absolute paths, `..` and symlinks alike
  --take-in FILE        take a Set file into this store, and stop. A .kbset —
                        what --package writes — has its inlined sources put in
                        the store and its Set file written, and every source
                        must hash to the address its `slot` names or the file
                        is refused whole. A .kset is resolved against its own
                        directory first, behind the same wall --package puts
                        it behind, and then taken in the same way. The id
                        comes from the file either way, and one already taken
                        is refused rather than overwritten
  --record-session ID   write the timeline to sessions/ID.ndjson as it
                        happens: the Set's records, then a `tick` a frame and
                        every edit between them. The material goes at the head
                        either way — from --load-set when given, and saved
                        under ID-material when not — so --replay needs nothing
                        else. Refused with --render, --seq and --replay, which
                        have no performance to record
  --replay ID           render sessions/ID.ndjson instead of running: the
                        material comes from the stream's head and every frame
                        advances by the `tick` that was recorded, so nothing
                        reads a clock. Needs --render or --seq
  --demo NAME           drive the deck from a script instead of the keyboard,
                        so a window shows it without anyone at one. A
                        demonstration harness: it presses keys and can do
                        nothing a person could not.
                          transport   beat sync engaged, scrubbed two bars
                                      back, held, then run forward past where
                                      it was
                          lines       one L1 drawn as sprites and as strokes,
                                      each slot faded out in turn so the other
                                      is seen alone. Brings its own two-slot
                                      deck unless --set says otherwise
  --watch               recompile and swap the slot whose files changed
  --budget-ms MS        frame budget a swapped-in Set is held to
  -h, --help            this

--render and --seq render the mix, which is what the window shows.
";

/// Printed by `--help` and once at startup, because there is no on-screen UI
/// and an instrument whose controls are undiscoverable is not one.
pub(crate) const BINDINGS: &str = "\
keys:
  0-3        focus a slot — the digit is the slot number the status line and
             every swap message use, so there is no off-by-one to remember
  space      focused slot on air / off air. Off air holds its `t`, so it
             resumes where it stopped rather than restarting
  w          ask the focused slot to warm off air, or withdraw the request.
             A request, not a command: the governor grants it only if the
             frame budget has room, and `park` on the status line is a request
             it is still holding — reconsidered every pass, so it takes effect
             by itself when a slot comes off air
  [ ]        focused slot gain down / up — the level the material arrives at,
             colour only, and not clamped at 1.0 because the mix is HDR
  \\          focused slot gain back to 1.0
  ; '        focused slot opacity down / up — the fader across the blend, and
             the only control that silences a slot under every mode. Pull this
             one, not the gain, to get out of material that has gone bad: an
             `over` layer at zero gain is a black card and still covers
  F G        fade the focused slot's fader out / in, over the current length,
             starting on the current grid. Opacity rather than gain: opacity
             silences under every blend mode, where a gain of zero under
             `over` is a black card that still covers
  x          crossfade — the focused slot out and the next one in, together.
             Two scheduled moves sharing a start and a length rather than one
             crossfade object, which is what makes a fade-in, a fade-out and a
             cut the same thing with different numbers. The slot being faded
             in is put on air first
  c          wipe the next slot in over the focused one — a mask at position
             0 on the incoming slot, put on air under `over`, and one
             scheduled move carrying the front to 1. Nothing in the
             transition knows what a mask is and nothing in the mask knows
             what a beat is; a wipe is the two of them. Needs a shape from `Z`
  Z          cycle the wipe's shape: off, left to right, bottom to top, the
             two diagonals, an iris
  R          cycle which renderer of the focused slot is live, landing on the
             current grid. Needs the slot to composite — --merge, or a Set
             file that records one: overdrawn renderers share one target and
             there is nothing to silence. The ones not selected still draw,
             into targets of their own — the choice is free of a rebuild, not
             free of a frame. One way: there is no position that folds them
             all back together. What you choose is kept: `k` writes it into
             the Set file as the merge's `live`
  N          cycle where a fade starts: the next bar, the next beat, now
  j          cycle how long a fade lasts: 4, 2, 8 beats, or 0 for a cut
  m          cycle the focused slot's blend mode: add, over, max. `over` is
             the only one in which a layer hides the ones under it, and what
             it hides with is the coverage its own sprites drew — thin
             material barely covers, which is not a bug. `screen` and
             `multiply` are absent on purpose: they are defined on [0, 1] and
             nothing has tone mapped this far up the pipeline
  t          cycle the tone map operator: clamp, Reinhard, ACES, AgX
  - =        output exposure down / up
  `          exposure back to 1.0
  y          cycle the focused slot's sync: free, tempo, beat. Modes the
             material cannot take are skipped with the reason — beat sync
             needs closed-form material, and tempo sync is refused on material
             that reads `beats`, which already follows the room. Engaging
             anchors the material at the current tempo, so nothing jumps
  U i        scrub the focused slot back / forward, a quarter beat a press.
             Beat sync only: scrubbing moves a position and the other modes
             are rates
  b          tap the beat — three or more taps set the tempo as well, and a
             tap always sets the phase. Needs --audio-in
  , .        halve / double the grid, and the octave the tracker looks in with
             it. Nothing finds an octave for you: the tracker follows the grid,
             so a set started an octave off stays an octave off until this key.
             `x2?` on the status line is the tracker saying it might be one.
             The phase does not move. Needs --audio-in
  o p        latency offset down / up, 5 ms a press, and it goes negative.
             Raise it if the picture reads late from where the audience is,
             lower it if the sound does. This is the only instrument that can
             read the PA and the projector, so it is the one to trust
  a          size the window so the canvas lands in it one texel to one texel.
             The window is a preview and fits the canvas into itself, so it
             normally shows bars; press this when something downstream is
             capturing the window, because a capture that is neither the
             canvas nor a clean crop of it is worse than useless. The canvas
             itself is --canvas and does not move — resizing the window
             changes what you can see and nothing about what is drawn
  k          keep the focused slot: write what it is playing right now as a
             Set file, named after the moment you pressed this. The capacities,
             the params, the bindings, the camera, the seeds and the sources
             are read off the Set on screen rather than off the flags the run
             started with, so a param you moved and a `.kir` you rewrote are
             both in it. Two are not on the Set to read and come from the run
             instead: the edges, because an edge is consumed where a Set is
             built and no control rewires one, and each node's name, which
             belongs to the way the file was spelled rather than to the
             procedure. The store write happens on a thread of its own and the
             line saying where it went arrives when it lands. `--save-set` is
             the same file written before a run instead of during one
  S          print the status line now
  h ?        print these bindings
  esc        quit
";

pub(crate) fn fail(message: &str) -> ! {
    eprintln!("karakuri-cli: {message}\ntry `--help`");
    std::process::exit(2);
}
