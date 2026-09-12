use super::*;

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

/// What a Set file said that no flag can say.
///
/// Not flags: there is no `--seed` and no `--camera`, and inventing two so that
/// a file could be read would be adding surface to carry a value rather than to
/// be used. **Slot 0's, always** — `--load-set` fills that slot and every other
/// comes from `--set`.
#[derive(Clone, Debug, Default)]
pub(crate) struct FromSet {
    /// What each geometry was salted with, by index, `None` where the file
    /// named none.
    ///
    /// **Per geometry rather than one number for the Set**, because that is
    /// what the record says and what the picture depends on: a salt derived
    /// from a source's position in `--set` moves when the list is reordered,
    /// and one read back from a file does not. The first entry doubles as the
    /// Set's own seed — see [`salts_for`].
    pub(crate) salts: Vec<Option<u32>>,
    pub(crate) camera: Option<karakuri_engine::camera::Orbit>,
    /// **Whether the file said its renderers composite**, which `--merge` is
    /// the other way of saying — see [`layering_for`], where the two meet.
    ///
    /// **Kept here rather than pushed into `args.merge`.** Folding it into the
    /// flag would make `--load-set` of a composited Set indistinguishable from
    /// `--merge 0` in every message that prints what the operator asked for,
    /// and the two are different sentences: one is what the file says, the
    /// other is what the hand said.
    pub(crate) layering: karakuri_engine::set::Layering,
    /// **Which renderer the file left folded to**, and `None` where every one
    /// of them is live — see `setfile::Loaded::live`, whose value this is.
    pub(crate) live: Option<u32>,
    /// What each geometry runs at, by index, `None` where the file named none.
    ///
    /// **Kept here rather than folded into `--capacity`.** One flag holds one
    /// number and a Set file holds one per geometry, so folding would put the
    /// first source's count onto every source — which is the bug
    /// [`capacities_for`] exists to have stopped making.
    pub(crate) capacities: Vec<Option<u32>>,
}

#[cfg_attr(test, derive(Debug))]
pub(crate) struct Args {
    /// `--param name=value`, applied after each Set is built, to every Set. A
    /// parameter change is a uniform write, not a structural change, which is
    /// why it needs no fork and no recompilation.
    pub(crate) overrides: Vec<ParamWrite>,
    /// `--bind`, applied to every Set on the same terms as `overrides`.
    ///
    /// **This flag is a stand-in for a Set file and is shaped so it can be
    /// retired for one.** Bindings belong in a `.kbset`, but nothing
    /// loads one into the engine yet — `karakuri-store` decodes records and no
    /// other crate reads a `Record` — and that is its own slice of work. So
    /// the flag names the record's own fields, one comma-separated
    /// `field=value` per JSON field, and the day `--set` takes a Set file the
    /// change here is deleting this and calling `Set::bind` from the decoder
    /// instead. Like `--param`, it applies to every Set, because the record
    /// has no slot field: a Set file is per Set, and one per `--set` is what
    /// replaces it.
    pub(crate) bindings: Vec<Binding>,
    /// `--edge`, applied to every Set on the same terms as `overrides`.
    ///
    /// **A procedure declares a named input slot and the Set binds it to a
    /// node.** `uses far : Geometry` in a `.kir` says what the file needs
    /// without naming which node supplies it — a file that named one would be
    /// coupled to one Set — so this is where the other half is written. Which
    /// geometry a morph blends towards used to be `--set` position 1, written
    /// nowhere at all, and reordering the command line changed the picture in
    /// silence.
    ///
    /// **The record is the home and the flag writes into it**, on the terms
    /// `--bind` follows: an `edge` record in a Set file is what a saved use
    /// carries, and this is the authoring surface that exists today. Like
    /// `--param` it applies to every Set of the deck, and an edge whose node
    /// names nothing in a given Set is a statement about a different one — see
    /// [`karakuri_engine::set::Wiring::edges`].
    pub(crate) edges: Vec<karakuri_engine::set::Edge>,
    /// The session tempo. There is **no tempo record** in the v0.2 vocabulary,
    /// so unlike `--bind` this flag has nothing to map onto yet; it is here
    /// because a binding to `beat` is meaningless at a tempo nobody can set.
    pub(crate) bpm: f32,
    /// One deck slot per entry, in composite order: the L1 that simulates, and
    /// the renderers drawn over it in list order — see `Set::build_many`. One
    /// renderer is the ordinary case; several is one simulation drawn several
    /// ways, which costs a draw pass apiece and no extra memory.
    pub(crate) sets: Vec<(Named, Vec<Named>)>,
    /// What `--capacity` was given, or [`karakuri_ir::DEFAULT_CAPACITY`] when
    /// it was not — read only through [`capacity_for`], which prefers the
    /// procedure's own declaration to this unless `capacity_given`.
    pub(crate) capacity: u32,
    /// Whether `--capacity` was *typed*. Without it a procedure's own declared
    /// default is used — see [`capacity_for`].
    pub(crate) capacity_given: bool,
    pub(crate) render_to: Option<PathBuf>,
    pub(crate) seq_to: Option<PathBuf>,
    pub(crate) frames: u32,
    /// **The preview window's size, and nothing else.** A window is a preview
    /// of what leaves by some other route, so it has no say in what is drawn —
    /// see `canvas`. Meaningless without a window, and refused rather than
    /// ignored when there is none.
    pub(crate) size: (u32, u32),
    /// **What the run renders at.** The canvas every `VideoSource` draws into,
    /// what every deck slot is sized to match, and what an offscreen render
    /// writes. Fixed for the run: changing it reallocates every slot's target,
    /// and the frame path allocates nothing.
    pub(crate) canvas: (u32, u32),
    /// Whether `--canvas` was *typed*. Carried rather than resolved at parse
    /// time because what it is refused against — whether the session being
    /// replayed carries a canvas of its own — is not known until the stream is
    /// read. See `replay_session`.
    pub(crate) canvas_given: bool,
    /// Watch every pair and hot-swap the slot whose files changed. Off by
    /// default: a run that is not being edited should not carry a worker
    /// thread per slot and a watchdog it will never use.
    pub(crate) watch: bool,
    /// Which slots composite their renderers rather than overdrawing them —
    /// `--merge 0`, repeatable. See `karakuri_engine::set::Layering`.
    ///
    /// **Not the whole answer any more**: a Set file records its layering, so a
    /// slot filled by `--load-set` may composite without appearing here. Ask
    /// [`layering_for`], which is where the flag and the file meet.
    pub(crate) merge: Vec<usize>,
    /// The interface every slot's Set publishes — `--publish
    /// name=L4:0:exposure[0.2..0.8]`, repeatable. Empty means every Set
    /// publishes everything it declares, which is what happened before an
    /// interface existed.
    pub(crate) published: Vec<karakuri_engine::set::Published>,
    /// `--mcp`. A port to serve the Model Context Protocol on, loopback only.
    /// `None` is the ordinary case and nothing in the frame path changes: this
    /// is a third control surface beside the keyboard and MIDI, and like them
    /// it can do nothing they cannot.
    pub(crate) mcp: Option<u16>,
    /// `--tempo-source`. A command to run as a child process that says where
    /// the beat is — see [`crate::tempo_source`]. `None` is the ordinary case
    /// and changes nothing: the grid comes from `--bpm`, the beat tracker and
    /// the tap keys exactly as it always has.
    pub(crate) tempo_source: Option<String>,
    /// `--audio-in`. `None` is no device at all, which is not the same as a
    /// device that is silent: with no device the bus answers `energy` and the
    /// bands exactly as it did before audio existed, and the oscillator
    /// free-runs.
    pub(crate) audio_in: Option<String>,
    /// `--midi-in`: a substring of the input port's name, or empty for the
    /// first one there is.
    pub(crate) midi_in: Option<String>,
    /// `--midi-map`: the operator's table. Optional even with a port, because
    /// a surface with no map still prints what it sends, which is the state an
    /// operator is in before they have written one.
    pub(crate) midi_map: Option<PathBuf>,
    /// The unmeasurable half of the output lag, in milliseconds. See
    /// `audio::DEFAULT_LATENCY_OFFSET_MS`.
    pub(crate) latency_offset_ms: f32,
    /// `--store DIR`: where content-addressed artifacts and Set files live.
    pub(crate) store: PathBuf,
    /// `--save-set ID`: write the material as a Set file and stop.
    pub(crate) save_set: Option<String>,
    /// `--load-set ID`: take the material from a Set file instead of from two
    /// `.kir` paths and the flags.
    pub(crate) load_set: Option<String>,
    /// `--record-session ID`: write the timeline as it happens.
    pub(crate) record_session: Option<String>,
    /// `--replay ID`: render a recorded session instead of running one.
    pub(crate) replay: Option<String>,
    /// What a Set file said that no flag says, when one was loaded — see
    /// [`FromSet`].
    pub(crate) from_set: Option<FromSet>,
    /// Drive the deck from a named script instead of waiting for a keyboard.
    /// A demonstration harness, not a feature: it presses keys.
    pub(crate) demo: Option<Demo>,
    /// The frame budget the watchdog holds a swapped-in Set to, in
    /// milliseconds. Exposed mostly so that the verdict against can be provoked
    /// on demand — `--budget-ms 0` stops every slot a build lands in — rather
    /// than only by writing a procedure slow enough to trip it.
    pub(crate) budget_ms: f32,
    pub(crate) look: Look,
}

pub(crate) fn fail(message: &str) -> ! {
    eprintln!("karakuri-cli: {message}\ntry `--help`");
    std::process::exit(2);
}

/// What [`parse_args_from`] found, short of a full [`Args`]: `--help` is a
/// request to print and exit successfully, not an error.
pub(crate) enum ParseOutcome {
    Run(Box<Args>),
    Help,
    /// `--list-sets`: print what the store at this path holds, and stop.
    ///
    /// **A third outcome rather than a field on [`Args`]**, and for `--help`'s
    /// reason: this is not a run. Nothing downstream of here — the compile, the
    /// scratch, the deck, the window — has anything to do with a listing, and a
    /// flag carried into [`Args`] would have to be checked above every one of
    /// them and would be wrong the day somebody added one more. The variant
    /// makes reaching a GPU with this flag not a thing that can be forgotten:
    /// there is no `Args` to run.
    ///
    /// It carries the store path because that is the whole of what a listing
    /// needs, and `--store` may be given on either side of this flag.
    ListSets(PathBuf),
    /// `--package ID` — or `--package FILE.kset`: write the Set filed under
    /// `ID`, or the one the authoring file at `FILE` names, to standard output
    /// with every source it names inlined, and stop.
    ///
    /// **One flag and two moments rather than two flags**, which is ADR-0229
    /// part 4: packaging is *"one operation, two moments"*, and
    /// `docs/manual/operations.html`'s *Send a Set to somebody, and take one
    /// in* is the row both are a moment of. The field is still `id` because the
    /// parse cannot tell which it is without touching a disk and does not try;
    /// [`packaged_set`] decides on the extension, which is what ADR-0231 made
    /// the extension for.
    ///
    /// **And the flag is named for what it does at both moments.** It writes a
    /// store as often as it reads one — a `.kset` has every part hashed and put
    /// — so `--bundle`, which reads as an export and nothing else, named half
    /// of it. `--package` and [`ParseOutcome::TakeIn`] are the two directions of
    /// one row (`docs/contributing.md` §4: a name means one thing, and one
    /// thing has one name).
    ///
    /// [`ParseOutcome::ListSets`]'s variant for [`ParseOutcome::ListSets`]'s
    /// reason, and the reason is the whole of why it is here: packaging reads a
    /// file and hashes some bytes, and there is no Set to build, no adapter to
    /// request and no window to open. An `Args` field would have to be checked
    /// above every early return in `main` and would be wrong the day somebody
    /// added one more; a variant with no `Args` in it makes reaching a device
    /// not a thing that can be forgotten.
    Package {
        store: PathBuf,
        id: String,
    },
    /// `--take-in FILE`: take a Set file into the store, and stop — a `.kbset`
    /// as it stands, a `.kset` resolved against its own directory first.
    ///
    /// Here for the same reason, and it compiles: each source goes through the
    /// checker so that a metadata card can be written for it. That is the check
    /// pass, which takes no device — `Set::validate` runs without one and a
    /// single procedure certainly does.
    ///
    /// **Which of the two forms it is, the extension says**, exactly as
    /// [`ParseOutcome::Package`]'s does: the parse carries a path and touches no
    /// disk to classify it, and [`taken_in_file`] decides.
    TakeIn {
        store: PathBuf,
        file: PathBuf,
    },
}

/// The real entry point: reads the process's own arguments, then hands them
/// to [`parse_args_from`] and turns its `Result` into the exit this binary
/// actually makes. Kept this thin so the parsing logic itself takes any
/// iterator of strings and returns rather than exits — which is what makes it
/// possible to drive adversarial input through it in a test.
pub(crate) fn parse_args() -> Args {
    match parse_args_from(std::env::args().skip(1)) {
        Ok(ParseOutcome::Run(args)) => *args,
        Ok(ParseOutcome::Help) => {
            print!("{USAGE}\n{BINDINGS}");
            std::process::exit(0);
        }
        // **Nothing is built to answer this.** The store is opened, the `sets/`
        // directory is read, each file in it is read, and the process ends —
        // see [`listed_sets`]. On stdout, because it is the answer to the
        // question that was asked rather than a note about a run.
        Ok(ParseOutcome::ListSets(root)) => match listed_sets_at(&root) {
            Ok(said) => {
                print!("{said}");
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("karakuri-cli: {e}");
                std::process::exit(1);
            }
        },
        // **To standard output, so a shell can redirect it.** What packaging
        // produces is a thing you *send somebody* — a single self-contained
        // file that works in their store — rather than something the library
        // keeps, so it goes where `> patch.ndjson` puts it and needs no naming
        // rule of its own in `sets/`. The store already holds this Set under an
        // id; a second copy of it there under some derived name would be a
        // second answer to which file is the Set.
        Ok(ParseOutcome::Package { store, id }) => match packaged_set(&store, &id) {
            Ok(said) => {
                print!("{said}");
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("karakuri-cli: {e}");
                std::process::exit(1);
            }
        },
        // On stdout for the reason a listing is: it is the answer to the
        // question that was asked. Nothing is compiled to *run* — the checker
        // is reached only to write each artifact's metadata card.
        Ok(ParseOutcome::TakeIn { store, file }) => match taken_in_file(&store, &file) {
            Ok(said) => {
                print!("{said}");
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("karakuri-cli: {e}");
                std::process::exit(1);
            }
        },
        Err(message) => fail(&message),
    }
}

/// The whole of argument parsing, as a pure function: no I/O, no exit, just
/// strings in and either an [`Args`] or an error message out. Every message a
/// person can act on is produced here rather than by falling back to a
/// default that hides a typo — `--set` and `--tonemap` already worked this
/// way; `--exposure` now validates on the same terms rather than silently
/// keeping 1.0 for a negative, zero, or unparsable value.
/// The value belonging to `flag`, refused rather than defaulted.
///
/// Two silences this removes. A flag at the end of the line with nothing after
/// it used to take `None` and fall back — `--render` with the path forgotten
/// opened a *window*, so a batch script with a typo hung waiting for one
/// instead of failing. And a flag whose value was missing used to swallow the
/// next flag: `--set --render out.png` read `--render` as a pair of paths and
/// then blamed `--render` for not being one.
///
/// A leading `-` is treated as another option unless the whole token parses as
/// a number, so a negative value is still a value.
pub(crate) fn value_for(
    flag: &str,
    it: &mut impl Iterator<Item = String>,
) -> Result<String, String> {
    match it.next() {
        Some(v) if !v.starts_with('-') || v.parse::<f64>().is_ok() => Ok(v),
        Some(v) => Err(format!(
            "`{flag}` was given no value — `{v}` is an option, not one"
        )),
        None => Err(format!("`{flag}` needs a value")),
    }
}

/// A number for `flag`, refused rather than defaulted.
///
/// `--frames`, `--capacity` and `--budget-ms` used to keep their default on
/// anything unparsable, so `--frames 24O` (a letter O) rendered 240 frames and
/// said nothing. A run that quietly used the default is indistinguishable from
/// one that honoured what was typed, which is the same failure this codebase
/// keeps finding in other clothes.
pub(crate) fn number_for<T: std::str::FromStr>(
    flag: &str,
    what: &str,
    it: &mut impl Iterator<Item = String>,
) -> Result<T, String> {
    let value = value_for(flag, it)?;
    value
        .parse()
        .map_err(|_| format!("`{flag} {value}` — expected {what}"))
}

/// A `WIDTHxHEIGHT` pair for `flag`.
///
/// **Zero is refused rather than clamped**, which is the difference between a
/// typo and a picture: everything downstream takes `max(1)` to keep a texture
/// descriptor legal, so `--canvas 1920x0` would have rendered a one-texel-tall
/// frame and reported the size it was asked for.
pub(crate) fn extent(flag: &str, value: String) -> Result<(u32, u32), String> {
    let bad = || format!("`{flag} {value}` — expected `WIDTHxHEIGHT`");
    let (w, h) = value.split_once('x').ok_or_else(bad)?;
    match (w.parse::<u32>(), h.parse::<u32>()) {
        (Ok(w), Ok(h)) if w > 0 && h > 0 => Ok((w, h)),
        (Ok(_), Ok(_)) => Err(format!("`{flag} {value}` — neither side may be zero")),
        _ => Err(bad()),
    }
}

/// One `--bind` value, as the `bind` record's own fields.
///
/// The grammar is `field=value`, comma separated, and every field name is the
/// record's. The single deviation is `range`, which the record writes as a
/// two-element array and this writes as `LOW..HIGH` — a comma inside a value
/// would be indistinguishable from the separator between fields, and quoting
/// rules to fix that would be a second grammar rather than a smaller one.
///
/// Unknown fields are refused rather than ignored. The record format ignores
/// an unknown `t` for forward compatibility between engine versions; a typo on
/// a command line has no such excuse, and `curv=pow2` silently taking the
/// default curve is the exact silence every other flag here was fixed for.
/// `--edge <node>.<slot>=<node>` — bind one procedure's declared input slot
/// to a node of the Set, whatever type the slot was declared with.
///
/// **`.` between the node and the slot, `=` before the node it is bound to.**
/// The `=` is `--set`'s already and means "the thing on the left is a name for
/// the thing on the right"; the `.` is the same dot the procedure reads the slot
/// through, so `--edge morph.far=sphere_shell` and `far.position` in the
/// `deform` are visibly one spelling. A Field slot is read as a call rather than
/// through a dot — `--edge field_lens.shape=melt_blob`, then `shape(p)` — and
/// still writes the same record, because what an edge says is the same fact
/// whatever fills the slot. `:` was not available — `--param` and
/// `--publish` use it for a layer and an index, and it is a path character on
/// Windows.
///
/// **`--bind` was taken**, by signals, which is the other reason the word here
/// is `edge`: it is what the record has always been going to be called, since
/// what it writes down is one edge of the graph a Set describes.
///
/// Every part is refused empty rather than accepted and resolved to nothing: an
/// edge with no slot in it is a sentence about a node, and there is no such
/// sentence.
pub(crate) fn parse_edge(value: &str) -> Result<karakuri_engine::set::Edge, String> {
    let bad = |what: &str| format!("`--edge {value}` — {what}");
    let Some((from, to)) = value.split_once('=') else {
        return Err(bad(
            "expected `<node>.<slot>=<node>`, e.g. `morph.far=sphere_shell` or \
             `field_lens.shape=melt_blob`",
        ));
    };
    // **The last dot, not the first.** A node name may hold one — nothing
    // refuses `--set my.morph=morph.kir` — and the slot is a `.kir` identifier,
    // which cannot.
    let Some((node, slot)) = from.rsplit_once('.') else {
        return Err(bad(
            "expected a `.` between the node and the slot it declares, e.g. \
             `morph.far=sphere_shell`",
        ));
    };
    if node.is_empty() || slot.is_empty() || to.is_empty() {
        return Err(bad("every part names something: `<node>.<slot>=<node>`"));
    }
    Ok(karakuri_engine::set::Edge {
        node: node.to_string(),
        slot: slot.into(),
        to: to.to_string(),
    })
}

pub(crate) fn parse_bind(value: &str) -> Result<Binding, String> {
    let bad = |what: &str| format!("`--bind {value}` — {what}");

    let mut layer = None;
    // **Absent is a wildcard, not zero.** A binding with no `index` is the
    // layer's — every node declaring the key — which is what `--bind` has
    // always meant and what one published control would drive. See
    // `Binding::index`.
    let mut index: Option<u32> = None;
    let mut key = None;
    let mut signal = None;
    let mut curve = Curve::Lin;
    let mut range = None;
    // Built whether or not it is used: a `noise.*` field on a binding whose
    // signal is not `noise` is a mistake worth reporting, and that check needs
    // to know one was given.
    let mut noise = NoiseConfig::default();
    let mut noise_given = false;
    // Kept as what was written rather than folded into `noise.kind` as it is
    // read. `NoiseKind` carries the octave count inside the `fbm` variant, so
    // assigning either field as it arrives lets the later one decide the
    // other: `noise.octaves` would turn a `white` that was asked for into an
    // `fbm` that was not. Both are resolved once, after the loop, where the
    // pair can be checked against each other.
    let mut noise_kind: Option<&str> = None;
    let mut noise_octaves: Option<u32> = None;

    for field in value.split(',') {
        let (name, v) = field
            .split_once('=')
            .ok_or_else(|| bad(&format!("`{field}` is not `field=value`")))?;
        let number = |what: &str| -> Result<f32, String> {
            v.parse::<f32>()
                .map_err(|_| bad(&format!("`{name}` expects {what}, got `{v}`")))
        };
        match name.trim() {
            "layer" => {
                layer = Some(layer_named(v).ok_or_else(|| {
                    bad(&format!("`layer={v}` — expected L1, L2, L3, L4 or Field"))
                })?)
            }
            "index" => {
                index = Some(v.parse::<u32>().map_err(|_| {
                    bad(&format!(
                        "`index={v}` — expected a node number, 0 for the first"
                    ))
                })?)
            }
            "key" => key = Some(v.to_string()),
            "signal" => signal = Some(v.to_string()),
            "curve" => {
                curve = Curve::parse(v).ok_or_else(|| {
                    let names: Vec<&str> = CURVES.iter().map(|c| c.name()).collect();
                    bad(&format!("`curve={v}` — expected {}", names.join(", ")))
                })?
            }
            "range" => {
                let (low, high) = v
                    .split_once("..")
                    .ok_or_else(|| bad(&format!("`range={v}` — expected `LOW..HIGH`")))?;
                match (low.parse::<f32>(), high.parse::<f32>()) {
                    (Ok(low), Ok(high)) => range = Some([low, high]),
                    _ => return Err(bad(&format!("`range={v}` — expected `LOW..HIGH`"))),
                }
            }
            "noise.kind" => {
                noise_given = true;
                if !NOISE_KINDS.contains(&v) {
                    return Err(bad(&format!(
                        "`noise.kind={v}` — expected white, value, perlin or fbm"
                    )));
                }
                noise_kind = Some(v);
            }
            "noise.rate" => {
                noise_given = true;
                noise.rate = number("a number of cycles per beat")?;
            }
            "noise.stream" => {
                noise_given = true;
                noise.stream = v
                    .parse::<u64>()
                    .map_err(|_| bad(&format!("`noise.stream={v}` — expected a whole number")))?;
            }
            "noise.octaves" => {
                noise_given = true;
                noise_octaves =
                    Some(v.parse::<u32>().map_err(|_| {
                        bad(&format!("`noise.octaves={v}` — expected a whole number"))
                    })?);
            }
            other => {
                return Err(bad(&format!(
                    "unknown field `{other}` — expected layer, key, signal, curve, range, \
                     or noise.kind / noise.rate / noise.stream / noise.octaves"
                )))
            }
        }
    }

    let layer = layer.ok_or_else(|| bad("no `layer=`"))?;
    let key = key.ok_or_else(|| bad("no `key=`"))?;
    let signal = signal.ok_or_else(|| bad("no `signal=`"))?;
    // Required, unlike `curve`: there is no defensible default range. A param
    // declares its own in the `.kir`, and silently binding across all of it
    // would be an aesthetic decision made by the argument parser.
    let range = range.ok_or_else(|| bad("no `range=LOW..HIGH`"))?;

    // **Built as the record and decoded back**, so the flag is what its
    // documentation always claimed: a way to write a `bind` record. Every
    // semantic rule — the `bpm` refusal, `octaves` needing `fbm`, a generator
    // needing `signal=noise` — lives in `setfile::binding_from_record` and
    // cannot differ between a command line and a Set file. That debt against
    // the decoder is paid by having one rule rather than two copies of it —
    // see `docs/adr/0066-a-flag-becomes-a-record-writer.md`.
    //
    // The one check that stays here is the one the record cannot express.
    // `BindNoise::octaves` has a serde default, deliberately — "a generator
    // omitted field by field is under-specified, not refused" — so a record
    // cannot say whether `octaves` was *named*. The flag knows, and an
    // `octaves` named beside a kind that has no octaves is an operator
    // expecting a generator they did not ask for.
    if noise_octaves.is_some() && noise_kind != Some("fbm") {
        return Err(bad(&format!(
            "`noise.octaves` needs `noise.kind=fbm`, and this asks for `{}`",
            noise_kind.unwrap_or("perlin")
        )));
    }

    let record = Record::Bind {
        layer: record_layer(layer),
        index,
        key,
        signal,
        curve: curve.name().to_string(),
        range,
        noise: noise_given.then(|| BindNoise {
            kind: noise_kind.unwrap_or("perlin").to_string(),
            rate: noise.rate,
            stream: noise.stream,
            octaves: noise_octaves.unwrap_or(setfile::DEFAULT_OCTAVES),
        }),
    };
    setfile::binding_from_record(&record).map_err(|e| bad(&e))
}

/// The `noise.kind` names, in the order the spec lists them. One list, so the
/// check and the message cannot drift apart.
pub(crate) const NOISE_KINDS: [&str; 4] = ["white", "value", "perlin", "fbm"];

/// `--param [L4:N:]name=value`.
///
/// **The address is optional and is `layer:index:` when it is there.** A bare
/// name is a wildcard — every node declaring it, "the Set's `exposure`", one
/// knob moving both renderers — which is what this flag has always meant and is
/// the useful default. The prefix is what sets two renderers apart, and it is
/// present or absent as a unit for the reason `ParamWrite::at` gives: a layer
/// alone stopped naming a node when a Set gained a list of them, so a
/// half-address would be a wish rather than an address.
///
/// `:` cannot occur in a param name — identifiers are alphanumerics and
/// underscores — so splitting on it is unambiguous and needs no quoting. A
/// malformed override is refused rather than dropped, on the same terms every
/// other flag here is: silence looks exactly like a parameter that was applied
/// and had no visible effect. A name that no procedure declares is still only a
/// warning at build time — that one is a question about the `.kir`.
/// A layer name as an operator writes it. **One reader, so `--param` and
/// `--bind` cannot disagree about which layers a Set has** — they did, and the
/// disagreement was silent: `--param L2:…` was refused as a malformed address
/// while `--bind layer=L2` was refused with a sentence saying L2 did not exist
/// yet, both long after it did.
/// A layer as the record vocabulary spells it.
///
/// **Exhaustive on purpose.** A sixth `Kind` stops compiling here rather than
/// falling to a default, which is what the two places that used to map this by
/// hand could not promise.
pub(crate) fn record_layer(kind: karakuri_ir::Kind) -> Layer {
    karakuri_environment::meta::layer_of(kind)
}

/// `--publish name=L4:0:exposure[0.2..0.8]`, or `--publish level=exposure[0..2]`
/// for every node that declares it.
///
/// **The address is the `--param` one and the range is the `--bind` one**, which
/// is why neither half needed a grammar of its own: what a published control is,
/// is a name in front of an address and a range behind it — and the address is
/// `layer:index:` present or absent as a unit, meaning the same thing it means
/// there. The range is mandatory: publishing without one would mean "over the
/// declared range", and spelling that as an absence would make the common
/// narrowing case look like the exception.
pub(crate) fn parse_publish(value: &str) -> Result<karakuri_engine::set::Published, String> {
    let bad = || {
        format!(
            "`--publish {value}` — expected `name=key[LOW..HIGH]` or `name=L4:0:key[LOW..HIGH]`"
        )
    };
    let (name, rest) = value.split_once('=').ok_or_else(bad)?;
    if name.is_empty() {
        return Err(bad());
    }
    let (at, rest) = match rest.split_once(':') {
        Some((layer, tail)) => {
            let layer = layer_named(layer).ok_or_else(bad)?;
            let (index, tail) = tail.split_once(':').ok_or_else(bad)?;
            let index: u32 = index.parse().map_err(|_| bad())?;
            (Some((layer, index)), tail)
        }
        None => (None, rest),
    };
    let (key, range) = rest.split_once('[').ok_or_else(bad)?;
    let range = range.strip_suffix(']').ok_or_else(bad)?;
    let (low, high) = range.split_once("..").ok_or_else(bad)?;
    let low: f32 = low.parse().map_err(|_| bad())?;
    let high: f32 = high.parse().map_err(|_| bad())?;
    if key.is_empty() {
        return Err(bad());
    }
    Ok(karakuri_engine::set::Published {
        name: name.to_string(),
        at,
        key: key.to_string(),
        range: [low, high],
    })
}

pub(crate) fn parse_param(value: &str) -> Result<ParamWrite, String> {
    let bad = || format!("`--param {value}` — expected `name=number` or `L4:1:name=number`");
    let (addressed, rest) = match value.split_once(':') {
        Some((layer, rest)) => {
            let kind = layer_named(layer).ok_or_else(bad)?;
            // **The index is required, even where a layer can hold only one
            // node.** `L3:0:radius` for a camera a Set has exactly one of is
            // two characters of ceremony, and the rule it keeps is worth more:
            // the address is `layer:index:` present or absent as a *unit*, so
            // there is exactly one wildcard spelling — a bare name. Making the
            // index optional would give `L4:exposure` a third meaning, sitting
            // between "every renderer" and "renderer 0".
            let (index, rest) = rest.split_once(':').ok_or_else(bad)?;
            let index: u32 = index.parse().map_err(|_| bad())?;
            (Some((kind, index)), rest)
        }
        None => (None, value),
    };
    let (key, number) = rest.split_once('=').ok_or_else(bad)?;
    let number: f32 = number.parse().map_err(|_| bad())?;
    if key.is_empty() {
        return Err(bad());
    }
    Ok(ParamWrite {
        at: addressed,
        key: key.to_string(),
        value: number,
    })
}

pub(crate) fn parse_args_from(args: impl Iterator<Item = String>) -> Result<ParseOutcome, String> {
    let mut args_out = Args {
        overrides: Vec::new(),
        bindings: Vec::new(),
        edges: Vec::new(),
        bpm: DEFAULT_BPM,
        sets: Vec::new(),
        capacity: karakuri_ir::DEFAULT_CAPACITY,
        capacity_given: false,
        render_to: None,
        seq_to: None,
        frames: 240,
        size: (1280, 720),
        canvas: (1920, 1080),
        canvas_given: false,
        watch: false,
        merge: Vec::new(),
        published: Vec::new(),
        mcp: None,
        tempo_source: None,
        audio_in: None,
        midi_in: None,
        midi_map: None,
        latency_offset_ms: audio::DEFAULT_LATENCY_OFFSET_MS,
        budget_ms: DEFAULT_BUDGET_MS,
        demo: None,
        store: PathBuf::from(places::STORE),
        save_set: None,
        load_set: None,
        record_session: None,
        replay: None,
        from_set: None,
        look: Look {
            op: TonemapOp::Aces,
            exposure: 1.0,
            white_point: 1.0,
        },
    };
    // Whether this was *typed*, not what it holds: it has a default, so "is it
    // still 1280x720" cannot tell a flag that was given from one that was not,
    // and the refusal below is about the giving. `--canvas` needs the same fact
    // for longer and carries it on `Args` instead.
    let mut size_given = false;
    let mut list_sets = false;
    let mut package: Option<String> = None;
    let mut take_in: Option<PathBuf> = None;
    let mut positional = Vec::new();
    let mut it = args;
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--help" | "-h" => return Ok(ParseOutcome::Help),
            "--set" => {
                let value = value_for("--set", &mut it)?;
                // **The first is the L1 and the rest are renderers**, drawn in
                // the order given. A third part used to be refused: it could
                // only be a typo when a Set was a pair, and taking `b,c` as one
                // literal filename would have blamed a missing file for a stray
                // comma. It is now what asking for two renderers looks like, and
                // there is no new syntax for it — one comma-separated list, read
                // as one L1 and however many L4s.
                let parts: Vec<&str> = value.split(',').collect();
                match parts.split_first() {
                    Some((l1, l4s))
                        if !l1.is_empty()
                            && !l4s.is_empty()
                            && l4s.iter().all(|p| !p.is_empty()) =>
                    {
                        // **Each part may carry a name** — `near=lattice.kir`.
                        // A name is what everything downstream addresses the
                        // node by; see `Named`.
                        let head = Named::parse(l1)?;
                        let rest: Vec<Named> = l4s
                            .iter()
                            .map(|p| Named::parse(p))
                            .collect::<Result<_, _>>()?;
                        args_out.sets.push((head, rest))
                    }
                    _ => {
                        return Err(format!(
                            "`--set {value}` — expected `L1.kir,L4.kir`, or \
                             `L1.kir,L4.kir,L4.kir` for several renderers over one geometry. \
                             Any part may be written `name=file.kir`"
                        ))
                    }
                }
            }
            "--render" => args_out.render_to = Some(PathBuf::from(value_for("--render", &mut it)?)),
            "--seq" => args_out.seq_to = Some(PathBuf::from(value_for("--seq", &mut it)?)),
            "--param" => {
                // A malformed override used to be dropped in silence, which
                // looks exactly like a parameter that was applied and had no
                // visible effect. A name that no procedure declares is still
                // only a warning at build time — that one is a question about
                // the `.kir`, not about the command line.
                let value = value_for("--param", &mut it)?;
                args_out.overrides.push(parse_param(&value)?);
            }
            "--bind" => {
                let value = value_for("--bind", &mut it)?;
                args_out.bindings.push(parse_bind(&value)?);
            }
            "--edge" => {
                let value = value_for("--edge", &mut it)?;
                args_out.edges.push(parse_edge(&value)?);
            }
            "--bpm" => {
                let value = value_for("--bpm", &mut it)?;
                // Refused rather than clamped here, on the same terms as
                // `--exposure`: the oscillator clamps a bad tempo so that a
                // zero cannot freeze every noise signal at once, but a tempo
                // typed at the command line and quietly changed is a run that
                // did not do what it was told.
                match value.parse::<f32>() {
                    Ok(v) if v.is_finite() && v > 0.0 => args_out.bpm = v,
                    _ => return Err(format!("`--bpm {value}` — expected a positive number")),
                }
            }
            "--tonemap" => {
                let value = value_for("--tonemap", &mut it)?;
                args_out.look.op = parse_op(&value)
                    .ok_or_else(|| format!("`--tonemap {value}` — expected {}", op_wire_names()))?;
            }
            "--exposure" => {
                let value = value_for("--exposure", &mut it)?;
                // Validated rather than defaulted on failure — a negative,
                // zero, NaN, infinite, or unparsable exposure used to be
                // silently swapped for 1.0, which reads as "the flag was
                // ignored" rather than "the value was wrong". `--tonemap` and
                // `--set` already fail loudly on a bad value; this does the
                // same. **What it does not do is clamp**: `100` is accepted and
                // lands at 100, well outside what `-`/`=` reach, because a
                // batch render is allowed to ask for something extreme. See
                // `exposure_positive_is_accepted_unclamped` and
                // [`EXPOSURE_MIN`], whose comment claimed the opposite.
                match value.parse::<f32>() {
                    Ok(v) if v.is_finite() && v > 0.0 => args_out.look.exposure = v,
                    _ => return Err(format!("`--exposure {value}` — expected a positive number")),
                }
            }
            "--mcp" => args_out.mcp = Some(number_for("--mcp", "a port number", &mut it)?),
            "--tempo-source" => {
                args_out.tempo_source = Some(value_for("--tempo-source", &mut it)?);
            }
            "--audio-in" => {
                args_out.audio_in = Some(value_for("--audio-in", &mut it)?);
            }
            "--midi-in" => {
                args_out.midi_in = Some(value_for("--midi-in", &mut it)?);
            }
            "--midi-map" => {
                args_out.midi_map = Some(PathBuf::from(value_for("--midi-map", &mut it)?));
            }
            "--latency-offset-ms" => {
                let value = value_for("--latency-offset-ms", &mut it)?;
                // Refused rather than clamped, on the same terms as
                // `--exposure`: an offset silently changed is an offset the
                // operator will spend the first song chasing.
                match value.parse::<f32>() {
                    Ok(v) if v.is_finite() && audio::LATENCY_OFFSET_RANGE.contains(&v) => {
                        args_out.latency_offset_ms = v
                    }
                    _ => {
                        return Err(format!(
                            "`--latency-offset-ms {value}` — expected {} to {} milliseconds",
                            audio::LATENCY_OFFSET_RANGE.start(),
                            audio::LATENCY_OFFSET_RANGE.end()
                        ))
                    }
                }
            }
            "--watch" => args_out.watch = true,
            "--publish" => {
                let v = value_for("--publish", &mut it)?;
                args_out.published.push(parse_publish(&v)?);
            }
            "--merge" => {
                let v = value_for("--merge", &mut it)?;
                let slot: usize = v.parse().map_err(|_| {
                    format!("`--merge {v}` — expected a slot number, 0 for the first")
                })?;
                if !args_out.merge.contains(&slot) {
                    args_out.merge.push(slot);
                }
            }
            "--demo" => {
                let name = value_for("--demo", &mut it)?;
                args_out.demo = Some(Demo::from_name(&name).ok_or_else(|| {
                    format!("no demonstration named `{name}` — `transport` or `lines`")
                })?);
            }
            "--store" => args_out.store = PathBuf::from(value_for("--store", &mut it)?),
            // Read into a local rather than onto `Args`: a listing is not a
            // run, and the answer is returned below once the whole command line
            // has been seen — `--list-sets --store DIR` and `--store DIR
            // --list-sets` are the same request.
            "--list-sets" => list_sets = true,
            // Locals rather than `Args` fields, for `--list-sets`'s reason:
            // neither is a run, and both are answered below once the whole
            // command line has been seen, so `--store` may be on either side.
            "--package" => package = Some(value_for("--package", &mut it)?),
            "--take-in" => take_in = Some(PathBuf::from(value_for("--take-in", &mut it)?)),
            "--save-set" => args_out.save_set = Some(value_for("--save-set", &mut it)?),
            "--load-set" => args_out.load_set = Some(value_for("--load-set", &mut it)?),
            "--record-session" => {
                args_out.record_session = Some(value_for("--record-session", &mut it)?)
            }
            "--replay" => args_out.replay = Some(value_for("--replay", &mut it)?),
            "--budget-ms" => {
                args_out.budget_ms = number_for("--budget-ms", "a number of milliseconds", &mut it)?
            }
            "--frames" => args_out.frames = number_for("--frames", "a frame count", &mut it)?,
            "--capacity" => {
                args_out.capacity = number_for("--capacity", "an element count", &mut it)?;
                args_out.capacity_given = true;
            }
            "--size" => {
                args_out.size = extent("--size", value_for("--size", &mut it)?)?;
                size_given = true;
            }
            "--canvas" => {
                args_out.canvas = extent("--canvas", value_for("--canvas", &mut it)?)?;
                args_out.canvas_given = true;
            }
            // An unknown option used to become a path, so `--wtach` looked
            // like a `.kir` that did not exist and the error blamed the file.
            other if other.starts_with('-') => return Err(format!("unknown option `{other}`")),
            _ => positional.push(PathBuf::from(arg)),
        }
    }

    // **Answered before a word is said about material**, and above every rule
    // below: none of them is about a listing. A `.kir` pair this run will not
    // read, a default pair nobody asked for, a refusal about two ways of naming
    // material — all of it belongs to a run, and refusing `--list-sets
    // --load-set x` for "two descriptions of the material" would be a refusal
    // about a Set nothing here is going to build.
    if list_sets {
        return Ok(ParseOutcome::ListSets(args_out.store));
    }
    // The same, and above the material rules for the same reason. Answered in
    // a fixed order rather than refused as a pair: each of the three prints and
    // stops, so the only thing a second one could change is which answer is
    // printed, and none of them is a run whatever the other says.
    if let Some(id) = package {
        return Ok(ParseOutcome::Package {
            store: args_out.store,
            id,
        });
    }
    if let Some(file) = take_in {
        return Ok(ParseOutcome::TakeIn {
            store: args_out.store,
            file,
        });
    }

    // The bare positional pair still means what it always meant, and now it is
    // simply the last slot: `--set a,b c.kir d.kir` is a deck of two.
    match positional.len() {
        0 => {}
        2 => args_out.sets.push((
            Named::bare(positional[0].clone()),
            vec![Named::bare(positional[1].clone())],
        )),
        n => {
            return Err(format!(
                "{n} file argument(s) — a Set is an L1 and an L4, so give two, or use --set"
            ))
        }
    }
    // The default pair, for a run that named no material at all. **Not when a
    // Set file is loaded**: that file *is* the material, and adding the default
    // beside it would put a second Set on the deck nobody asked for — which is
    // not merely extra, it is a `--bind` from the file landing on material that
    // has no such parameter and saying so.
    if args_out.sets.is_empty() && args_out.load_set.is_none() {
        // A demonstration that needs a particular scene brings it, because one
        // that asks the operator to assemble it first is not a demonstration.
        // Only when nothing else named material: `--set` still wins.
        match args_out.demo.map(Demo::deck).filter(|d| !d.is_empty()) {
            Some(deck) => args_out.sets = deck,
            // **`examples/star_vortex.kset`'s two parts**, and the choice is a
            // demo one: what a run that named nothing is worth looking at.
            // ADR-0270 freed it — the reference workload is a named Set rather
            // than whatever this line says — and ADR-0271 spent it. The `.kset`
            // itself is not loaded here because this is two paths; its two
            // `bind` records are what a bare run does not get.
            None => args_out.sets.push((
                Named::bare("examples/coil_vortex.kir"),
                vec![Named::bare("examples/star_flares.kir")],
            )),
        }
    }
    // Refused rather than resolved: a Set file describes the material, and two
    // `.kir` paths describe the material, and a run given both has been told
    // two different things about what to play.
    if args_out.load_set.is_some() && !args_out.sets.is_empty() {
        return Err(
            "--load-set names the material and so do the `.kir` paths beside it; give one \
             or the other"
                .to_string(),
        );
    }
    if args_out.load_set.is_some() && args_out.save_set.is_some() {
        return Err(
            "--load-set and --save-set in one run: it would rewrite what it just read".to_string(),
        );
    }
    // An offscreen run is a function of its inputs — that is why it never
    // watches files either. Accepting `--audio-in` here and quietly ignoring it
    // would produce a PNG sequence whose bindings all sat at a tenth effect
    // with nothing to say why.
    if args_out.audio_in.is_some() && (args_out.render_to.is_some() || args_out.seq_to.is_some()) {
        return Err(
            "`--audio-in` with `--render` or `--seq` — an offscreen run takes no live input, \
             because its output has to be a function of its arguments. Drop one of them"
                .to_string(),
        );
    }
    // The same, and it is the same argument: a surface is a pair of hands, and
    // an offscreen render has nobody at it.
    if args_out.midi_in.is_some() && (args_out.render_to.is_some() || args_out.seq_to.is_some()) {
        return Err(
            "`--midi-in` with `--render` or `--seq` — an offscreen run takes no live input, \
             because its output has to be a function of its arguments. Drop one of them"
                .to_string(),
        );
    }
    let offscreen = args_out.render_to.is_some() || args_out.seq_to.is_some();
    // **`--mcp` with `--load-set` used to be refused here**, because a Set
    // built from the store had no procedure files on disk for a model to read
    // or rewrite. The scratch is where they go now: `--load-set` writes its two
    // procedures there like any other material, so a saved Set is editable and
    // the round trip — save, load, edit, save — closes. See `scratch::place`.
    // A surface with nobody at it, on the same terms as the other two — and
    // one more reason besides: an offscreen run is a function of its arguments,
    // and a port that can rewrite a procedure mid-render is the opposite of
    // that.
    if args_out.mcp.is_some() && (offscreen || args_out.replay.is_some()) {
        return Err(
            "`--mcp` with `--render`, `--seq` or `--replay` — an offscreen run takes no \
             live input, because its output has to be a function of its arguments"
                .to_string(),
        );
    }
    // Third of the same kind, and the same argument: a tempo source is another
    // machine's clock, and an offscreen run's output has to be a function of
    // its arguments. A replay has a stronger reason still — it follows the grid
    // the session recorded, so a live source would be overwriting the
    // performance it is supposed to be reproducing.
    if args_out.tempo_source.is_some() && (offscreen || args_out.replay.is_some()) {
        return Err(
            "`--tempo-source` with `--render`, `--seq` or `--replay` — an offscreen run \
             takes no live input, and a replay follows the grid the session recorded"
                .to_string(),
        );
    }
    // A map with no port is a file nothing reads, and the likely cause is a
    // forgotten `--midi-in` rather than a deliberate one.
    if args_out.midi_map.is_some() && args_out.midi_in.is_none() {
        return Err(
            "`--midi-map` with no `--midi-in` — there is no surface for the map to be of"
                .to_string(),
        );
    }
    // `--size` is the preview window's and an offscreen run has no window. It
    // used to be the render size too, and that is exactly the confusion being
    // removed: a reader who types `--render out.png --size 1920x1080` today
    // means `--canvas`, and quietly rendering at 1280x720 because `--size` no
    // longer reaches the canvas would be the worst of the three outcomes.
    if size_given && (offscreen || args_out.replay.is_some()) {
        return Err(
            "`--size` sets the preview window, and this run has no window. \
             Use `--canvas` for what is rendered"
                .to_string(),
        );
    }
    // **Not the argument the two above make.** `--record-session` is an output,
    // so "an offscreen run takes no live input" does not reach it. The reason is
    // that there is no performance here to record: an offscreen run advances one
    // step a frame and every edit it makes is a flag, so the stream would be
    // `--save-set`'s output followed by a constant — a file that looks like a
    // timeline and is a re-encoding of the command line.
    //
    // It was silently dropped instead, which is the failure the recorder's own
    // construction site has a comment warning against: a run that continued
    // without the recorder is "a performance nobody can replay and nothing
    // saying so". That guard fires when the *file* cannot be opened, and did
    // nothing when the flag never reached it at all — exit 0, no warning, no
    // session.
    if args_out.record_session.is_some() && (offscreen || args_out.replay.is_some()) {
        return Err(
            "`--record-session` with `--render`, `--seq` or `--replay` — there is no \
             performance to record: an offscreen run is a function of its arguments, and \
             the stream would say only what they already say"
                .to_string(),
        );
    }
    if args_out.sets.len() > MAX_SLOTS {
        return Err(format!(
            "{} Sets, and the deck holds {MAX_SLOTS}",
            args_out.sets.len()
        ));
    }
    Ok(ParseOutcome::Run(Box::new(args_out)))
}

/// Refuse a canvas the GPU cannot make a texture of, by name.
///
/// **Not in `extent`**, because the number it is checked against is the
/// adapter's rather than the format's: `max_texture_dimension_2d` is 8192 on
/// some machines and 16384 on others, so a canvas is legal or not depending on
/// what is running the run. That makes it the earliest point *after* a device
/// exists rather than the latest point before one does.
///
/// The alternative is what happened before: a panic out of `create_texture`
/// naming a wgpu limit, from inside a call stack that says nothing about
/// `--canvas`. Every other refusal in this program names the flag.
pub(crate) fn check_canvas(device: &wgpu::Device, width: u32, height: u32) {
    let limit = device.limits().max_texture_dimension_2d;
    if width > limit || height > limit {
        eprintln!(
            "karakuri-cli: `--canvas {width}x{height}` — this GPU renders at most \
             {limit}x{limit}"
        );
        std::process::exit(1);
    }
}

/// Each slot's seed, derived from its index alone so that two slots given the
/// same pair are not the same picture twice — and so that the derivation is a
/// pure function of the deck layout rather than of anything measured. Slot 0
/// is [`SEED`] unchanged, so a one-Set run renders exactly what it always did.
pub(crate) fn seed_for(slot: usize) -> u32 {
    SEED.wrapping_add((slot as u32).wrapping_mul(0x9E37_79B9))
}

/// **What each of a slot's geometries is salted with**, in the order its L1
/// procedures were given — one number per geometry.
///
/// `recorded` is what a Set file said, per geometry, and it wins where it said
/// anything. That is the whole of what recording a salt buys over deriving one:
/// `docs/ir-spec.md` asks for a value assigned when a source is added and read
/// back from the stream forever after, so that reordering `--set` stops
/// changing which grid gets which randomness. Empty for a slot no file filled,
/// which is every slot but slot 0.
///
/// **Derived where nothing recorded one**, by the engine's own fallback rather
/// than by a formula spelled out a second time here — and the spec licenses
/// that squarely: *where it came from stops mattering once it is recorded*. A
/// bare `--set` run is salted by ordinal and looks exactly as it always did;
/// the moment `--save-set` writes these numbers down they stop being derived,
/// which is why this is also what the writer asks. One function, so the file
/// cannot record a salt the run was not using — the shape [`capacities_for`]
/// was fixed into after recording the flag's number and drawing another.
pub(crate) fn salts_for(seed: u32, recorded: &[Option<u32>], geometries: usize) -> Vec<u32> {
    (0..geometries)
        .map(|at| {
            recorded
                .get(at)
                .copied()
                .flatten()
                .unwrap_or_else(|| karakuri_engine::set::derived_salt(seed, at))
        })
        .collect()
}

/// Open the store, or stop with the reason. Both directions need one and
/// neither can do anything useful without it.
pub(crate) fn open_store(args: &Args) -> karakuri_store::store::Store {
    match karakuri_store::store::Store::open(&args.store) {
        Ok(store) => store,
        Err(e) => {
            eprintln!("karakuri-cli: store `{}`: {e}", args.store.display());
            std::process::exit(1);
        }
    }
}

/// **What the store holds, one line per Set** — the operator's half of what
/// `list_sets` tells a model.
///
/// **It prints and stops.** No window, no adapter, no compile, no Set built:
/// asking what is in a library is not a run, and a flag that opened a GPU to
/// answer it would be unusable over ssh on the machine the library is on. That
/// is why it is answered out of [`parse_args_from`] — see [`ParseOutcome`] —
/// rather than somewhere down `main` where it would have to be kept above every
/// early return by hand.
///
/// **The same summary the MCP tool renders**, from
/// [`setfile::summarise`]: one derivation, two renderings. What a node
/// is called here is what `read_set` calls it, because the answer comes from
/// one function — an operator reading a line here and a model reading a block
/// there are looking at one library and must be told one thing about it.
///
/// A compact line and not a block: the question is *which of these do I want*,
/// and what answers it is the id to type next to `--load-set`, when it was
/// saved, and enough of what it holds to tell two of them apart. What each node
/// declares is `read_set`'s answer, over MCP, on one Set at a time.
pub(crate) fn listed_sets(
    store: &karakuri_store::store::Store,
    root: &std::path::Path,
) -> Result<String, String> {
    let mut sets =
        setfile::summarise(store).map_err(|e| format!("store `{}`: {e}", root.display()))?;
    if sets.is_empty() {
        // Not an error and not silence: an empty store is what a store looks
        // like before anything has been kept in it, and the answer says where
        // sets come from rather than leaving a blank terminal to be read as a
        // failure.
        return Ok(format!(
            "no sets in `{}` — nothing has been kept here yet. `--save-set ID` writes \
             one, and so does the `k` key during a run.\n",
            root.display()
        ));
    }
    // **Most recent first, breaking ties by id.** `--save-set` twice in one
    // second gives two files one mtime on a coarse filesystem clock, and a sort
    // whose keys tie leaves the order to whatever `read_dir` said — so two runs
    // of this flag over an untouched store would print two different lists. The
    // id is unique by construction, which makes the order total.
    sets.sort_by(|a, b| b.written.cmp(&a.written).then_with(|| a.id.cmp(&b.id)));
    let width = sets.iter().map(|set| set.id.len()).max().unwrap_or(0);
    let mut out = String::new();
    for set in &sets {
        let _ = write!(
            out,
            "{:<width$}  {}  ",
            set.id,
            setfile::written_at(set.written)
        );
        match &set.unreadable {
            // Listed and named rather than dropped: a file in `sets/` that will
            // not read is the one thing here an operator has to go and look at.
            Some(why) => {
                let _ = writeln!(out, "unreadable: {why}");
            }
            None if set.nodes.is_empty() => {
                let _ = writeln!(out, "no material: it holds no `slot` record");
            }
            None => {
                let _ = writeln!(out, "{}", holdings(&set.nodes));
            }
        }
    }
    Ok(out)
}

/// The listing for a store path — **opening nothing that is not there.**
///
/// `Store::open` establishes the layout under a root that does not exist yet,
/// which is right for a run about to write into it and wrong here: a flag whose
/// whole promise is that it only reads must not leave a directory behind to say
/// that a library is empty. A path with no store at it is an answer, and it is
/// a different one from a store with no sets in it — the first is very often a
/// mistyped `--store`.
pub(crate) fn listed_sets_at(root: &std::path::Path) -> Result<String, String> {
    if !root.exists() {
        return Ok(format!(
            "no store at `{}` — nothing has ever been kept there, and nothing was \
             created to find that out. Check `--store`, or keep something with \
             `--save-set ID`.\n",
            root.display()
        ));
    }
    let store = karakuri_store::store::Store::open(root)
        .map_err(|e| format!("store `{}`: {e}", root.display()))?;
    listed_sets(&store, root)
}

/// **One Set, with every source it names inlined, as the text to print.**
///
/// **A package goes to standard output**, which is what this returning a
/// `String` is: it is a file you *send somebody* — one self-contained patch
/// that loads in a store which has never held the material — so it belongs
/// where `karakuri-cli --package night01 > night01.kbset` puts it. A
/// store directory would need a naming rule of its own for it, and a second
/// copy of a Set sitting beside the Set is a second answer to which of them is
/// the file.
///
/// **Opening nothing that is not there**, on [`listed_sets_at`]'s terms: a flag
/// that only reads must not leave a store behind to report that a Set is
/// missing from it. Here it is an error rather than an answer, because a bundle
/// of a Set that does not exist is not a bundle.
///
/// **Or an authoring file, and that is one flag rather than two.** ADR-0229
/// part 4 settles that packaging is *"one operation, two moments"* — loading an
/// authoring file *is* packaging it, and packaging for distribution is the same
/// resolution done ahead of time — so the flag grew the other moment instead
/// of a second flag beside it, and `docs/manual/operations.html` keeps the one
/// row it always had. And it is called packaging because of exactly that: the
/// half that takes a `.kset` resolves, stores and writes, so an export is only
/// one of the two things this flag does — see [`ParseOutcome::Package`].
/// **Which of the two is decided by the extension**, on
/// exactly the terms ADR-0231 made it load-bearing for: a value ending in
/// `.kset` is a path to an authoring file, and anything else is an id in the
/// store. Nothing else could decide it — an id and a relative path are both
/// bare words — and this is the same sentence the store already reads a name
/// with.
///
/// **The authoring half opens a store that is not there, where the id half
/// refuses to.** Resolving is a *write*: each part is read from disk, hashed
/// and put in the store as an artifact, so the store has to exist by the time
/// the first one lands, and establishing the layout under a root an operator
/// named is what every other writing path here does — see [`taken_in_file`].
/// The id half is still a pure read and still leaves nothing behind.
pub(crate) fn packaged_set(root: &std::path::Path, named: &str) -> Result<String, String> {
    let lines = if named.ends_with(setfile::AUTHORING_SUFFIX) {
        let store = karakuri_store::store::Store::open(root)
            .map_err(|e| format!("store `{}`: {e}", root.display()))?;
        setfile::bundle_authored(&store, std::path::Path::new(named))?
    } else {
        if !root.exists() {
            return Err(format!(
                "no store at `{}`, so there is no set `{named}` to package — check `--store`",
                root.display()
            ));
        }
        let store = karakuri_store::store::Store::open(root)
            .map_err(|e| format!("store `{}`: {e}", root.display()))?;
        setfile::bundle(&store, named)?
    };
    Ok(lines
        .iter()
        .map(|line| format!("{}\n", line.as_str()))
        .collect())
}

/// **A Set file somebody sent you, into this store**, and the report of what
/// happened.
///
/// **Both of a Set's forms, and the extension is the whole of what tells them
/// apart** — [`packaged_set`]'s rule, read from the other end, and ADR-0231's.
/// A `.kbset` is already resolved and carries its own sources, so it is read and
/// taken in as it stands. A `.kset` is the authoring form: it names its parts by
/// relative path, so it goes through `setfile::resolve` first — behind the same
/// wall, refusing the same escapes, absolute paths, `..` and symlinks alike.
///
/// **Resolved *and* inlined rather than resolved alone**, which is
/// `setfile::bundle_authored`, so that taking a `.kset` in lands the same store
/// as packaging it and taking the result in. `setfile::unbundle` writes a
/// metadata card for each source the lines carry; handing it resolved lines with
/// nothing inlined would file the Set and leave every artifact cardless — one
/// operation reaching two different stores by two routes, which is the
/// disagreement a second spelling always is.
///
/// The store *is* opened where it is not there, unlike [`packaged_set`]'s id
/// half: this is a write, and establishing the layout under a root an operator
/// named is what every other writing path here does.
pub(crate) fn taken_in_file(
    root: &std::path::Path,
    file: &std::path::Path,
) -> Result<String, String> {
    let store = karakuri_store::store::Store::open(root)
        .map_err(|e| format!("store `{}`: {e}", root.display()))?;
    let named = file
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    let lines = if named.ends_with(setfile::AUTHORING_SUFFIX) {
        setfile::bundle_authored(&store, file)?
    } else {
        karakuri_store::ndjson::read(file)
            .map_err(|e| format!("reading `{}`: {e}", file.display()))?
    };
    setfile::unbundle(&store, &lines)
}

/// What a Set holds, by layer and in the order the layers compose: `2 L1, 1 L2,
/// 3 L4`. A layer nothing is on is left out rather than printed as a zero,
/// because most Sets are on three of the six and a line of zeroes reads as
/// something missing.
pub(crate) fn holdings(nodes: &[setfile::NodeSummary]) -> String {
    [
        Layer::L1,
        Layer::L2,
        Layer::L3,
        Layer::L4,
        Layer::Field,
        Layer::L5,
    ]
    .iter()
    .filter_map(|layer| {
        let n = nodes.iter().filter(|node| node.layer == *layer).count();
        (n > 0).then(|| format!("{n} {}", setfile::layer_name(*layer)))
    })
    .collect::<Vec<_>>()
    .join(", ")
}

/// What a loaded Set file said its geometries run at, for the slot it filled.
///
/// **Slot 0 and nothing else**, on the same terms as its seed and its camera:
/// `--load-set` fills that slot and every other slot comes from `--set`.
pub(crate) fn recorded_capacities(args: &Args, slot: usize) -> &[Option<u32>] {
    match (slot, args.from_set.as_ref()) {
        (0, Some(from_set)) => &from_set.capacities,
        _ => &[],
    }
}

/// What a loaded Set file said its geometries are salted with, for the slot it
/// filled. **Slot 0 and nothing else**, on the same terms as its capacities.
pub(crate) fn recorded_salts(args: &Args, slot: usize) -> &[Option<u32>] {
    match (slot, args.from_set.as_ref()) {
        (0, Some(from_set)) => &from_set.salts,
        _ => &[],
    }
}

/// Where a loaded Set file aimed the built-in camera, for the slot it filled.
/// **Slot 0 and nothing else**, on the same terms as its capacities and its
/// salts — and `None` for a file that recorded no `camera` record, or one whose
/// record named a camera that is a procedure and was reported and dropped on
/// the way in.
///
/// **One reading, used twice.** The Set built at startup takes it and so does
/// the watcher that restates it on every rebuild; working it out in two places
/// is how a rebuild came to aim somewhere the startup did not.
pub(crate) fn recorded_camera(args: &Args, slot: usize) -> Option<karakuri_engine::camera::Orbit> {
    match (slot, args.from_set.as_ref()) {
        (0, Some(from_set)) => from_set.camera,
        _ => None,
    }
}

/// **Whether a slot composites its renderers or overdraws them** — the one
/// reading, from the flag and the file together.
///
/// **Either saying so is enough, and that is a decision rather than a
/// coincidence.** `--merge N` can only turn compositing *on*: there is no
/// spelling that turns it off, because the record's absence is what overdraw is
/// and a flag that could say `false` would be a second spelling of not typing
/// it. So `--load-set X --merge 0` on a file that already records a `merge` is
/// two ways of asking for the same thing, and a file that records none plus
/// `--merge 0` is the flag adding what the file did not say. The one case that
/// could have been a contest — a composited file with `--merge` *left off* —
/// is not one: leaving a flag off is not a statement, and treating it as one
/// would make a loaded preset silently overdraw exactly as it did before this
/// record existed.
///
/// **One reading, used everywhere**, on [`recorded_camera`]'s terms: the Set
/// built at startup takes it, the watcher that restates it on every rebuild
/// takes it, and the writer that saves the slot back out takes it. Working it
/// out in two places is how a rebuild came to aim a camera where the startup
/// did not.
pub(crate) fn layering_for(
    args: &Args,
    slot: usize,
    recorded: karakuri_engine::set::Layering,
) -> karakuri_engine::set::Layering {
    if recorded == karakuri_engine::set::Layering::Composite || args.merge.contains(&slot) {
        karakuri_engine::set::Layering::Composite
    } else {
        karakuri_engine::set::Layering::Overdraw
    }
}

/// **What a loaded Set file said about its layering**, for the slot it filled —
/// [`recorded_camera`]'s shape and its rule: slot 0 and nothing else, since
/// `--load-set` fills that slot and every other comes from `--set`.
///
/// Separate from [`layering_for`] because the replay path has a recorded
/// layering in hand without ever touching `args.from_set` — its Set file is the
/// head of a session stream — and both readings have to meet the flag through
/// the same function. That is [`recorded_capacities`] and [`capacities_for`]'s
/// split, for its reason.
pub(crate) fn recorded_layering(args: &Args, slot: usize) -> karakuri_engine::set::Layering {
    match (slot, args.from_set.as_ref()) {
        (0, Some(from_set)) => from_set.layering,
        _ => karakuri_engine::set::Layering::Overdraw,
    }
}

/// Which renderer a loaded Set file left folded to, for the slot it filled.
/// **Slot 0 and nothing else**, on the same terms as its camera and its
/// capacities — and `None` for a file that recorded no selection, which is
/// every input live and is the state a Set nobody selected in comes up in.
///
/// **No flag stands beside this one.** There is no `--select`: a selection is
/// something an operator makes with `r` while watching, so the only thing that
/// can put one in before the first frame is a file that recorded one.
pub(crate) fn recorded_live(args: &Args, slot: usize) -> Option<u32> {
    match (slot, args.from_set.as_ref()) {
        (0, Some(from_set)) => from_set.live,
        _ => None,
    }
}
