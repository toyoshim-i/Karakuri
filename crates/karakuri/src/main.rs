//! **The instrument**: the console in a window, with the engine behind it.
//!
//! ```sh
//! cargo run -p karakuri                                # the pair the repository ships
//! cargo run -p karakuri -- geometry.kir renderer.kir   # a pair of your own
//! cargo test -p karakuri                               # this file's own tests
//! ```
//!
//! **This is the program, and it used to be an example.** It was
//! `karakuri-console/examples/panel.rs` until
//! [ADR-0214](../../../docs/adr/0214-the-program-moves-out-of-the-cli-and-two-thin-binaries-sit-over-it.md),
//! and it was an example for one reason: everything a program needs beyond the
//! panel lived in `karakuri-cli`, which has no library target, so there was
//! nothing for a binary to sit on. There is now — `karakuri-environment` — and
//! [ADR-0213](../../../docs/adr/0213-the-interface-milestones-meter-is-the-panel-column-and-has-means-an-operator-reaches-it.md)
//! is why the difference is not a packaging preference: a `has` badge in the
//! panel column of `docs/manual/operations.html` means **an operator running
//! the instrument reaches the operation**, and nobody plays a set from
//! `cargo run --example`.
//!
//! **It is still a package with no library target of its own, deliberately.**
//! Nothing may depend on this one. A surface is where the buck stops.
//!
//! # What it is for, and what became free
//!
//! The example this was carried five questions in its header. Four of them are
//! answered by this being a program at all — a person launches it and sees the
//! answer — and they are kept here because the *code* that answers them is what
//! moved:
//!
//! 1. That `egui` renders through `wgpu` 30 into this window at all, which is
//!    the bet
//!    [ADR-0155](../../../docs/adr/0155-egui-draws-the-panel-and-the-price-is-wgpu-30.md)
//!    made.
//! 2. **That the engine's rendered texture reaches the panel**, which is the
//!    other half of that bet: a `Deck` on the same `Device`, one of its
//!    [`SLOTS`] Sets on air, drawn through `Present` into the picture's
//!    rectangle **and again into every deck's preview cell**, sampled by
//!    `egui` in the same submission. One `Present` and targets of several
//!    sizes, because `Present::draw` letterboxes into whatever it is handed.
//!
//!    **The picture is the only `karakuri_engine::Sink`, and the cells are not
//!    in the slice.** A sink is what the composited frame is handed to, and
//!    the picture is the one thing here that draws the whole mix, so
//!    `frame::compose` is given `[picture]` and nothing else. Each cell is a
//!    present pass off **its own slot's** target rather than off the fold —
//!    ADR-0240 and ADR-0258 — so it is not a sink at all, and [`monitor`]
//!    draws all [`DECKS`] of them inside that frame's own encoder, through
//!    `compose`'s `finally`. The panel goes into the same encoder for a
//!    related but different reason: it does not receive the composited frame
//!    either, it samples what a sink produced. See [`Engine`], [`Presented`]
//!    and [`monitor`].
//! 3. That the arrangement's numbers look right at real sizes against
//!    `docs/manual/console.html`.
//! 4. That dragging a boundary still works with a toolkit in the loop.
//!
//! **The fifth is the one that is still a harness, and it is still here.**
//! [`Costs::say`] prints what the window costs once nobody has touched it for
//! three seconds, and to do it this binary installs a counting
//! [`#[global_allocator]`](Counting) over the whole process. That is a
//! measuring instrument inside the instrument, and it is not free: every
//! allocation a player's run makes pays a thread-local increment for it. It is
//! kept because nothing else measures these numbers and two of them are load
//! bearing — `karakuri_console::budget::PANEL_PASS` names this file as what
//! holds its 1.26 ms honest, and both schedulability conditions are asserted
//! against that number.
//!
//! **That it ships is decided, and the reason is written a few lines below in
//! [`WRITTEN_ALLOCS`]'s own documentation**: the last number nobody was
//! checking went from 184 to 456 to 525 and stayed wrong for two commits,
//! *"because nothing was checking it"*. A build with this compiled out is a
//! build where that happens again, and
//! [ADR-0164](../../../docs/adr/0164-the-panel-is-budgeted-rather-than-forbidden-to-allocate.md)
//! chose to **budget** the panel rather than forbid it to allocate — a budget
//! is a guarantee only while something counts, which is
//! [`docs/contributing.md` §4](../../../docs/contributing.md).
//! See
//! [ADR-0217](../../../docs/adr/0217-the-counting-allocator-ships-because-a-written-number-nothing-checks-goes-stale.md),
//! which also records what a cargo feature and a test target each lost on.
//! Half of what `Costs` collects is not a harness at all: the per-frame timing
//! is what the transport row draws as `frame_ms` and `fps`.
//!
//! **It is deliberately not the start of a bay.** Every body is empty except
//! the picture and the preview row, and both are empty of everything this file
//! could have invented — no label, no frame, no placeholder; see
//! [`karakuri_console::view`].
//!
//! **There is one frame loop, and it is not in this file.**
//! `frame::compose` is `karakuri-engine`'s, and `karakuri-cli`'s window and
//! its PNG writer are the other two callers — which is the point of it: this
//! file used to hand-roll `begin_frame`, a render, a conditional present
//! pass per target, the panel and a submit, beside a loop in another crate
//! that said the same thing differently, and every replay defect this project
//! has found came from two such loops disagreeing.
//!
//! # The engine here is the shortest path from two files to texels
//!
//! Four slots, built from one `.kir` pair the way `karakuri-cli` builds them,
//! and no more than that yet. The pair is [`Sources`] with a preset library
//! behind it, and the two positional paths are what this program takes from
//! the command line beside `--presets` and `--store`. Three things beyond the
//! engine are here.
//!
//! **Each slot watches that pair**, which is [`watched`] and is one
//! `HotSwap::new` over a `karakuri_environment::watch::Watch` — the same
//! wiring `karakuri-cli` does for `--watch`, and the whole of what puts a row
//! in the Staging lane: without it no `swap::Event` of any variant is emitted
//! in this program, and the lane could reach no state but empty.
//!
//! **The store is opened to be read** — once, at startup, so the Library bay
//! has names to list ([`library`]) — and **once to be written**, which is the
//! arrangement family and the one thing in this program that reaches a disk on
//! purpose: an operator's arrangement is kept under
//! `arrangements/<name>.arrangement.json` and put back from there
//! ([`arrangement`],
//! [ADR-0221](../../../docs/adr/0221-an-arrangement-is-named-by-the-operator-and-kept-in-a-fourth-place.md)).
//! The transport row's arrangement pill is what emits both, and it is the one
//! control on this panel that asks for letters. The wiring is here because it
//! can be nowhere else: the console cannot reach the store and the store
//! cannot name a layout, so a third party is what joins them.
//!
//! And **records exist**: a mixer control emits an operation,
//! `karakuri-operation-record` turns it into a `Record`, and [`apply`] is what
//! moves the deck with it, because
//! [P-0090](../../../docs/principles/0090-a-surface-offers-it-never-decides.md)
//! puts every control at the same record. **Records reach a disk while a
//! recording runs** — the `rec` capsule opens a
//! `karakuri_environment::session::Recorder` and every record this file applies
//! goes into `sessions/<id>.ndjson` behind it ([`Sessions`]) — and **no record
//! stream drives time**, which is the half that is still true: nothing here
//! reads a session back.
//!
//! # What is not wired, and what each would be for
//!
//! **This list used to read `no audio, no MIDI, no MCP, no replay and no
//! session` and that was the whole of it** — five absences and not one purpose,
//! which is a note that cannot be told from a decision. It was duly read as a
//! charter twice. Each line below says what the thing would be *for*, so that
//! whoever reaches one knows what they are reaching for.
//!
//! **Four of the five have since been wired, and the entries stay** — audio,
//! MIDI, MCP and the session recorder — because what each is *for* is the
//! thing this section is worth reading for, and because a list that only ever
//! names absences is the note that was read as a charter. **One is still
//! absent: replay.**
//!
//! - **Audio, and this one is wired now.** The window opens the host's
//!   default input at startup and the transport row's `audio-in` pill says
//!   which it is and lists the others ([`listening`], [`attached`]), so the
//!   signal bus carries a measurement rather than an invention and the grid
//!   follows the room. What that reached on
//!   [the operations page](../../../docs/manual/operations.html): *Attach a
//!   beat source* in the panel column, and *Tap the beat*, *Halve or double
//!   the grid* and *Nudge the latency offset* in the key column — `b`, `,`,
//!   `.`, `o` and `p`. **The last of those took a letter back.** The page
//!   specifies the offset as `o` and `p`, this program bound `p` to
//!   `Op::Report`, and a badge naming two keys with one of them bound would
//!   be a badge that lies; asked whether a panel diagnostic needs a shortcut
//!   at all, the answer was that it does not. So `p` is the offset on both
//!   keyboards now and the report keeps no key — see [`nudged`] and
//!   `Op::Report`, which the console still performs and nothing here asks
//!   for. **One of the four rows it was waiting on is still out of reach**:
//!   *Attach a signal to a parameter* is a bay's worth of work of its own and
//!   is nothing to do with a device being open.
//! - **MIDI, and this one is wired now.** A control surface, so a hand
//!   reaches a fader without a mouse. The window opens **the first MIDI input
//!   there is** at startup — no flag, for
//!   [ADR-0220](../../../docs/adr/0220-the-key-column-is-the-instruments-keyboard-and-the-clis-keys-are-its-own.md)'s
//!   reason one column along — loads a map in two tiers
//!   (`<store>/maps/default.map`, then the `examples/surface.map` that ships)
//!   and drains what arrives into [`App::performed`] beside the MCP drain, so
//!   **a mapped knob writes the record the mixer's fader writes** ([`surfaced`],
//!   [`App::mapped`],
//!   [ADR-0335](../../../docs/adr/0335-the-panel-opens-the-first-surface-there-is-and-the-map-is-two-tiers-under-the-store.md)).
//!   **And a learn is a map edit rather than an operation**, which is what is
//!   left of the vocabulary half: arm the transport row's `learn` pill, point
//!   at a control, move a knob, and the line goes into
//!   `<store>/maps/default.map` — no `Operation`, no `Record` and no row on
//!   the operations page, because its operand is the pointer and a record of
//!   one would put the room's wiring in the session stream (ADR-0336). The
//!   port and the map are still fixed for the run: the `map` pill is a
//!   readout, and reaching a different map while running is not built.
//! - **MCP, and this one is wired now.** The model's door — the whole reason
//!   the instrument is AI-native — and `karakuri_mcp` is the
//!   server. `--mcp PORT` binds it before the window opens and hands it the
//!   very [`Readout::opening`] the four `mcp` pills write, so what a hand
//!   opens on the panel is what the server reads on its next call. **What is
//!   still true of the entry it replaces is the vocabulary half**: no
//!   operation names opening the door, so the pills are a setting rather than
//!   an operation (ADR-0236), and a run without the flag writes that setting
//!   for nobody. A panel that opened a class silently would still be the
//!   opposite of
//!   [P-0094](../../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md),
//!   which is why every pill is shut at startup.
//! - **Replay.** Rendering a recorded session back. This is offline work and
//!   an instrument is not where it belongs; `karakuri-cli --replay` is the
//!   right home for it and no row asks the panel for it. **This is one of the
//!   two entries here that is still an absence.**
//! - **Session, and this one is wired now.** Recording the timeline as it
//!   happens, which is *Record the session*. The `rec` capsule at the end of
//!   the transport row is the control — one control with two ends — and a
//!   press opens a `session::Recorder` on a thread, writes the head Set and
//!   `sessions/<id>.ndjson`, and a second press flushes and closes it
//!   ([`Sessions`]). What it produces is what `--replay` above will be typed
//!   with; nothing in this program reads one back.
//!
//! **What is missing is named rather than left to be noticed**, and what is
//! no longer missing is named in the same place rather than left to be
//! discovered by running it. Audio, MIDI, MCP and replay are all
//! `karakuri-environment`'s and all reachable from here; **replay is the one
//! that is not wired up**, and it is not scaffolding that is missing but a
//! home — an offline render belongs in `karakuri-cli` and no row asks the
//! panel for it. The watcher was in that list until the Staging lane needed a
//! producer, and what it took was one function — which is the measure of how
//! far the rest of them are, rather than an argument for doing them all now.
//! Audio, MCP and the recorder have each since gone the same way, one function
//! and one control at a time. **The store either side of a watcher is no
//! longer declined, and this paragraph said it was**: `Watch::storing_to` puts
//! every build's sources under a content address and `Watch::snapshotting_to`
//! keeps every version that compiled under `<store>/history/`, filed under the
//! Set the slot was running — both wired here, from one `history::Snapshots`
//! seeded before the window opens ([`watched`], [`seeded`]). What the Staging
//! lane's two operations wait on now is the lane itself: nothing can press a
//! candidate row, and what a row says when one build changes two nodes is
//! undecided. `karakuri-cli` is still what you play a set with.
//!
//! **There is a governor, and it is the one thing here that is not the
//! shortest path.** It runs once, at startup, and it is [`Engine::ask_to_prime`]:
//! deck A is Live and deck B is asked to prime against a budget that has no
//! room for it, so the governor parks it. That is the only way a slot on this
//! panel can read one residency and have been asked for another —
//! `Deck::set_residency` writes the request *and* grants it, and only a
//! [`Deck::govern`] pass can hold a slot below what was asked for. Without it
//! the two residencies agree on every slot and every frame, and the roll
//! [ADR-0190](../../../docs/adr/0190-the-parked-tally-rolls-because-two-lamps-do-not-fit-in-fifty-three-pixels.md)
//! drew could not be seen by running this window.
//!
//! **Four slots, because a strip is a slot and a mixer is its channels.**
//! The deck is built full — [`SLOTS`] is `deck::MAX_SLOTS` — and every slot
//! holds this program's one pair at its own salt, since a `HotSwap` cannot
//! hold nothing and this program has no second pair to give one. Deck A is
//! Live; the other three rest at `Residency::Allocated`, which is what a
//! channel nobody has asked anything of is: contributing nothing to the mix,
//! and running all the same. An operator brings one up by cycling its tally or
//! by loading a Set into it.
//!
//! **Every cell draws, and every slot steps, whatever its residency.** An
//! off-air slot is stepped and drawn into its own target on every frame, at the
//! room's tempo, because the slot nobody is watching is the candidate and the
//! cell is what it is judged from
//! ([ADR-0258](../../../docs/adr/0258-the-look-comes-before-the-fader-so-a-cell-draws-every-slot-and-says-which-nothing-it-is.md)),
//! and a cell drawn from a slot nothing is stepping is a still rather than a
//! look at the material
//! ([ADR-0269](../../../docs/adr/0269-a-slot-that-is-drawn-is-stepped-and-a-preview-runs-at-the-rooms-tempo.md)).
//! It costs a step and a draw per slot, both outside the governor's arithmetic;
//! the roadmap's *Performance discipline* carries what is owed.
//!
//! **Each cell is its own deck's monitor and cannot be another's.** A cell is
//! presented from `Deck::slot_view` for the slot it is lettered for — that
//! deck's own target rather than a cut of the composite — so no cell can draw
//! deck C's material under the letter `A`. This paragraph used to say the
//! opposite, because the sinks all took the one composited frame and the cell
//! followed whichever deck the output was auditioning; ADR-0240 retired that
//! control, and [`Engine::aim`] carries the reasoning.
//!
//! # This file owns none of the model, and none of the view
//!
//! The arrangement, the drag, the operations, the palette, the bay head and
//! the rule about who gets a pointer event are all `karakuri-console`. What is
//! left here is a window, a surface, the `egui` plumbing between them, and the
//! English — a `Dragged` into the line it prints, an `Outcome` into the line a
//! key prints. **The model returns what happened; the words are this file's.**
//!
//! # The readout is driven by the layout, not by the pointer
//!
//! A line is printed when the boundary **moves**, and a stop is announced
//! once. That is `Panel::moved`'s doing rather than this file's: it returns
//! nothing at all for a move that changed nothing. A line per pointer event is
//! one comparison less and wrong twice over — the interesting lines are buried
//! under identical ones, and a hand held against the edge of the window, which
//! is where a drag ends up, produces hundreds a second into a terminal that
//! has to keep up with them while the window waits.
//!
//! # The loop sleeps, and what wakes it is a decision made elsewhere
//!
//! [ADR-0164](../../../docs/adr/0164-the-panel-is-budgeted-rather-than-forbidden-to-allocate.md)'s
//! still-panel clause: a panel with nothing changing on it does no per-frame
//! work.
//! **Whether a frame is owed is `karakuri_console::repaint`'s to answer**, not
//! this file's — the same seam the model came out of the window loop through,
//! and for the same reason: an event handler cannot be called from a test, and
//! an under-repaint is a stale pixel rather than an error, so there is nothing
//! to assert against in here. Every arm below reaches
//! [`Change::repaint`](karakuri_console::repaint::Change::repaint) and none of
//! them decides for itself.
//!
//! Three things wake the loop and they are the whole list: a window event,
//! the deadline `egui` named for its own next frame, and the deadline the
//! reading below is taken on. `App::about_to_wait` is the one place
//! `ControlFlow` is set.
//!
//! # Every frame that could not be acquired gets a decision
//!
//! This loop waits for events rather than spinning, so nothing asks for
//! another frame on its own. A `get_current_texture` that comes back
//! `Outdated` and is dropped with a bare `return` is therefore a window that
//! stops drawing and never starts again, with nothing said anywhere — see
//! [`missed`], carried over from the example that preceded this one, and
//! `karakuri-cli`'s window sink, which carries the same table after the same
//! failure.
//!
//! # A panic here aborts the process
//!
//! macOS cannot unwind a Rust panic across the Objective-C frame a `winit`
//! callback is called from, so an assertion reachable from an event handler is
//! a crash and not an error. Nothing below asserts on input.

use std::time::Duration;

use karakuri_engine::deck::MAX_SLOTS;
use karakuri_environment::Opening;
use karakuri_mcp as mcp;
use karakuri_store::store::Store;
use winit::event_loop::{ControlFlow, EventLoop};

mod session;
pub(crate) use session::{rewired, watched, Keeping};

/// The window loop's own keyboard: [`keymap::KEY_BINDINGS`], the [`KeyCtx`]
/// its actions take, and [`keymap::key_column`], the unit test that holds it
/// against `docs/manual/operations.html`. Split out the same way
/// [`session`] was, one piece of this file's own decomposition along —
/// `window_event`'s dispatch into the table stays here, in `main.rs`.
mod keymap;

/// The window this opens, in logical pixels. Comfortably above the smallest
/// viewport the arrangement is claimed to work at, so nothing starts clamped.
pub(crate) const WINDOW: (f64, f64) = (1440.0, 900.0);

mod readout;
pub(crate) use readout::*;

// ---------------------------------------------------------------------------
// The engine in the Program bay
// ---------------------------------------------------------------------------

/// **The session canvas: the shape every output is fitted to, and the size the
/// run starts at.**
///
/// **It is no longer what the Set renders at.** The render size belongs to an
/// output ([ADR-0246](../../../docs/adr/0246-the-render-size-belongs-to-the-output-and-the-sessions-canvas-is-only-its-default.md))
/// and the frame is composited once at the largest enabled one
/// ([ADR-0247](../../../docs/adr/0247-one-frame-is-rendered-and-scaled-into-each-output.md)),
/// which is [`render_size`]. This constant is what that derivation *starts*
/// at, before any output has said anything, and what the picture's rectangle
/// is fitted to. ADR-0246 called it *"wrong twice over"* — a session-wide
/// constant where the size belongs to an output, and a measurement default
/// standing in for an instrument's — and both halves are answered here: the
/// size is an output's, and what is left is a shape and a starting value.
///
/// # Why the picture is still fitted to this rather than to what is rendered
///
/// `aims` hands this to `picture_rect` rather than `Present::size()`, and the
/// difference is a loop. The picture's size *is* its rectangle now, and its
/// rectangle is the canvas's shape fitted into the bay's box — so fitting it
/// to what is rendered would fit it to itself, one frame late. The fixed point
/// happens to be the same rectangle, which is exactly what makes the mistake
/// invisible: the reason would be circular and the picture would have no shape
/// of its own, only whatever rounding it converged on. The console page's
/// sentence — *"It is the canvas's shape rather than the region's"* — is what
/// this keeps true.
///
/// **The reference workload's canvas**, 1280x720 (`docs/contributing.md` §1),
/// and it is that on purpose rather than by default: a figure taken at the
/// reference workload can be put beside every other one in this repository.
///
/// **What that buys is smaller than it was, and the reading says so.** A run
/// whose Program bay is 466 x 262 renders at 466 x 262 from its first frame,
/// so a panel figure is at *whatever the window was* rather than at this
/// number — which is P-0095's rule met by printing the size beside the
/// figures rather than by pinning the size. The reference workload is a
/// harness figure and was never a panel one (ADR-0270), and this constant was
/// already carrying the caveat below.
///
/// **The canvas is half of that workload and the material is the other half**,
/// which is the distinction ADR-0270 drew and ADR-0271 made visible: the
/// workload is `examples/drift_cloud.kset` — 262144 elements — at this canvas,
/// and what this program opens on is `examples/star_vortex.kset`'s pair at
/// 10240. So this constant keeps a reading comparable *as far as the canvas
/// goes*, and a bare run is not a reference-workload figure. The startup
/// legend prints the material and the capacity it actually ran, and says so. **Not the
/// size of the picture, and not the size of a preview cell either** — see
/// [`Present::draw`], which letterboxes this into whatever it is drawn into
/// and is handed two rectangles of different sizes a frame, and which is what
/// the manual means by *"it letterboxes into the width it has"*.
///
/// **It is also the shape the picture is given**, through [`aims`] and
/// `picture_rect`: the console sizes the picture's rectangle to this rather
/// than to whatever the region happens to be, so what surrounds the picture is
/// the bay's own card rather than bars anything rendered. It reached
/// `picture_rect` from `Present::size` until 2026-09-09, when that stopped
/// being a second copy of this number and became the frame's own derived
/// size — see this constant's own paragraph above, and [`aims`].
pub(crate) const CANVAS: (u32, u32) = (1280, 720);

/// **Which profile this binary was built with**, for the legend's own reading.
///
/// It said *debug, with dependencies at opt-level 3* in a string, so a
/// `--release` run printed the wrong one at the foot of its own numbers — and
/// a host-clock figure whose build is misreported is worse than one with no
/// build beside it, because the reader has no reason to doubt it.
pub(crate) const PROFILE: &str = match cfg!(debug_assertions) {
    true => "debug profile with dependencies at opt-level 3",
    false => "release profile",
};

/// **Deck A's seed salt**, which decides where its elements start. Any value
/// is a picture; 7 is the one `karakuri-cli`'s own tests use, so this looks
/// like what they look like.
///
/// **Deck A's, and the rest of the deck is counted off it** — see
/// [`slot_salt`], which is the one place this file turns a slot into a seed.
pub(crate) const SEED_SALT: u32 = 7;

/// **Every slot the engine has, and this deck is built full.**
///
/// `deck::MAX_SLOTS` rather than a number written here, and four of them
/// rather than the two this program ran until now. A slot **is** a mixer
/// channel — the bay draws one strip per slot, and the mock's page keeps its
/// four tracks whatever the deck has
/// ([ADR-0178](../../../docs/adr/0178-the-mixer-draws-four-tracks-and-as-many-strips-as-the-deck-has.md))
/// — so what decides how many there are is what a `Deck` can hold, which
/// `Deck::new` states in an assert: *"a deck holds 1 to MAX_SLOTS slots"*.
///
/// **This is what changed, and it is the whole change.** The second slot used
/// to exist because a park needs somewhere to happen
/// ([ADR-0191](../../../docs/adr/0191-the-panels-parked-deck-is-parked-by-the-governor-or-it-is-a-drawing-of-one.md)),
/// which is a demonstration deciding the shape of the instrument. B, C and D
/// are now here for the reason A is; the park still happens, on one of them,
/// because it has to happen somewhere.
///
/// # Four slots is not four Live slots, and that is measured
///
/// **The reference workload does not fit the compute budget four times over
/// on this machine.** `Deck::measure_slots` probes all four at startup, and
/// `the_budget_parks_a_deck_and_the_strip_carries_both_residencies` prints
/// what it got: four runs on an Apple M4 Pro read per-slot costs of 4.4 to
/// 16.3 ms summing to 27.9, 30.0, 32.6 and 55.5 ms, against
/// `governor::DEFAULT_COMPUTE_BUDGET_MS` of 16.7. **Four of even the cheapest
/// reading seen — 4.39 ms — is 17.6 ms and still over.** They are host-clock
/// numbers taken under `cargo test`, so they read coarse and biased high and
/// spread by a factor of four between runs; what does not move across that
/// spread is the verdict, which is the same property ADR-0191 bought its
/// budget arithmetic for.
///
/// So this deck opens with **one** slot Live and the rest at
/// `Residency::Allocated` ([`Engine::new`]) — not because four slots do not
/// fit, but because three of them hold material nobody has asked for, and the
/// budget says what the operator would be spending if they did. **If an
/// operator puts all four on air the governor will not stop them**: it never
/// takes a Live slot off air
/// ([P-0094](../../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)),
/// so what happens is `Report::over_budget` and priming suspended — a warning
/// on the legend's governor line and a decision left with the person who made
/// it. That is the governor doing its job rather than this file second-guessing
/// it with a slot count.
pub(crate) const SLOTS: usize = MAX_SLOTS;

/// **Which slot this program opens on air, and which one it asks to warm.**
///
/// Two of [`SLOTS`], named because this file has something to say about each.
/// Deck A is Live and is the whole of what the picture draws; every cell draws
/// its own slot whatever its residency. Deck B is
/// asked to prime and is parked by the budget — see [`Engine::ask_to_prime`]
/// — which is the one state on this panel where a slot's two residencies
/// disagree, and nothing reaches it without somebody asking.
///
/// **The rest are named by nothing, which is the point of them.** A slot this
/// file has no use for is not a slot that should not exist: it is a channel
/// with nothing asked of it, resting at [`Residency::Allocated`] until an
/// operator cycles its tally or loads material into it. See [`Engine::new`],
/// where that resting state is written and argued.
pub(crate) const ON_AIR: usize = 0;
pub(crate) const ASKED_TO_PRIME: usize = 1;

/// **The `.kir` pair this plays, and the whole of how an operator names
/// material.**
///
/// A Set is built from an L1 and an L4 — a geometry and a renderer — and
/// `Set::build` takes exactly those two. **They are one Set, and the deck has
/// [`SLOTS`] of them**: every slot is this same pair built again at the salt
/// [`slot_salt`] counts off for it, so four strips are four simulations of one
/// procedure rather than one picture drawn four times.
///
/// **Deliberately not `karakuri-cli`'s parser.** That program has thirty-odd
/// flags, `--set a.kir,b.kir` among them, and they live in its own `main.rs`
/// where nothing else can reach them. A second `--flag` vocabulary here would
/// be a second answer to *how does an operator name material*, which is the
/// failure this whole move exists to stop paying for
/// ([`docs/contributing.md` §4](../../../docs/contributing.md)).
/// So **material is two positional paths and nothing else**: enough to pick
/// what plays, and no vocabulary to disagree with. The day the two programs
/// share one, it comes from a package both can reach and this goes.
///
/// # And the two flags are not a second material vocabulary
///
/// [`sources_from`] takes `--presets DIR` and `--store DIR`, which reads at
/// first like the paragraph above being paid lip service and then broken. It
/// is not, and the reason is that they answer a **different question**: not
/// *what plays*, which is the one the sentence above is about and which is
/// still two paths, but *where this program's data lives* — the directory the
/// shipped presets were installed into, and the directory the library it lists
/// is kept in. Neither can name a procedure, neither appears in a Set, and
/// neither can be given instead of the pair. `--presets` chooses what the pair
/// **defaults to** when the operator gives no pair at all, which is the whole
/// of its reach into this type.
///
/// `docs/contributing.md` §4 is about one name meaning one thing, and the
/// failure it names would
/// be two ways to say *play this file*. Two ways to say *and the files are
/// over here* is not that failure; refusing to have any way to say it is how
/// [`Sources::under`]'s predecessor came to bake the build machine's own tree
/// into a shipped binary. See
/// [ADR-0230](../../../docs/adr/0230-where-the-programs-data-lives-is-told-rather-than-baked.md).
///
/// `Debug` unconditionally rather than `#[cfg_attr(test, derive(Debug))]`: that
/// idiom does not survive a crate boundary — `cfg(test)` is set when the
/// *defining* crate's tests compile and not when a consumer's do — and
/// ADR-0214 names it as the one class of surprise a move of this kind produces.
/// Nothing consumes this type today, and writing the version that would break
/// is not cheaper than writing the one that would not.
///
/// # It is what the operator named, and no longer what a deck runs from
///
/// **The shape is unchanged and it is still the right one**, which is a
/// conclusion rather than an omission: a Set is an L1 and an L4, the command
/// line is one pair, and the strip's [`Sources::material`] is that pair's two
/// names. What changed is the *number* of them a run holds. Every slot runs
/// from its own working copy ([`working_copies`]), so the deck is built from
/// **[`SLOTS`] of these** — [`Engine::new`] takes a slice, indexed by slot —
/// and the one the command line produced is kept beside them for the two
/// questions that are still about what the operator asked for: what the strips
/// are called, and what a refusal names.
///
/// A single `Sources` widened to carry four pairs would have been the wrong
/// answer to the same fact: the pair is a Set's shape and a Set is what a slot
/// holds, so four slots are four values of this type and not one value with a
/// slot index in it.
mod launch;
pub(crate) use launch::*;

/// **How often a run with `--mcp` wakes to serve.**
///
/// # The loop sleeps, and that is the whole of why this exists
///
/// This window draws a frame when something changed it or when `egui` asked for
/// one after a delay it named, and on no other occasion — [`App::about_to_wait`]
/// is where that rule lives, and it is ADR-0164's still-panel clause as the
/// operating
/// system sees it. `karakuri-cli` has no such rule: it draws continuously, so
/// what a model asks for is taken up on the next frame, which is always a
/// millisecond away.
///
/// **Ported without this, `--mcp` on the panel answers a timeout.** A `save_set`
/// reaches the render loop over a channel and waits for the loop to drain it,
/// and a loop asleep on `ControlFlow::Wait` drains nothing until somebody
/// touches the window — so the first thing a model asked this program for came
/// back as *the render loop had not taken this save after 10s*. That was found
/// by launching the panel and talking to it, and by nothing else: it is
/// invisible to a test that drives the drain itself.
///
/// # What the wake costs, and why it is a frame and not only a drain
///
/// Draining alone would answer a save and a rewiring, because neither needs a
/// picture. It would not answer the thing this surface is *for*: a procedure a
/// model writes is picked up by a watcher, compiled on a worker and installed
/// **at a frame boundary** — `Deck::begin_frame`, which happens on a frame and
/// nowhere else. A run that drained and never drew would take a write, say it
/// took it, and go on showing what it was showing.
///
/// So a served run draws. The reading [`Costs::say`] prints says so in its own
/// words and needs no new ones: the picture is live, so the run takes the arm
/// that names the engine as the reason and prints the rate it measured.
///
/// **A tenth of a second, which is the watcher's own poll interval.** A model is
/// not a pair of hands and does not need sixty frames a second to be answered;
/// what it needs is that no request waits longer than the machinery behind it
/// already does, and `watch::Watch` stamps its files ten times a second. Faster
/// would be frames spent on nothing; slower would be a surface answering more
/// slowly than the files it is watching.
pub(crate) const SERVED: Duration = Duration::from_millis(100);

/// **What one frame of a control surface is sized for**, on this side of the
/// channel.
///
/// A surface's fastest gesture is a fader sweep, which a device sends at a few
/// hundred messages a second, and every one of them is coalesced to one
/// operation per control before it reaches here
/// (`karakuri_environment::midi`'s `Router::emit`). So a frame's worth is a
/// map's continuous controls plus whatever pads were hit, which is single
/// figures — and this is generous rather than measured, for the reason the
/// buffer exists at all: it is reserved once so that the frame path never
/// grows it (P-0091).
pub(crate) const MAPPED: usize = 32;

// -- where `STEPS_A_FRAME` was ------------------------------------------
//
// **The step count is measured now, and the constant that stood here said the
// opposite until 2026-09-08.** Its documentation read: *"P-0092 says simulation
// time comes from a record and never from a clock. There is no record here —
// this program is not a session — so the honest third option is neither: a
// fixed count per frame … It is not a performance, and a `karakuri-cli` that
// measured an interval and wrote a `tick` is what a performance is."* **Both
// halves of that were wrong.**
//
// **P-0092 does not say *never from a clock*.** Its first sentence is *"Time
// comes from a record: live, the engine derives the step count from real time
// and writes it in; replaying, it reads the number back and derives nothing"* —
// the clock read is the **live** path of the rule, not a thing the rule
// forbids. What it forbids is a clock reached from inside the simulation, and a
// replay deriving the count again. A fixed count is not a third option between
// those; it is a refusal to take the measurement the record is shaped to carry.
//
// **And *this program is not a session* stopped being true with ADR-0289**,
// which gave the transport row a `rec` toggle that opens a recorder and writes
// a stream from this window.
//
// **What it cost was not about recordings.** `DT` is 1/60 s, this window is
// `PresentMode::Fifo`, and `karakuri_signal`'s oscillator advances by
// simulation steps and never by wall clock — so one step per frame *drawn* made
// simulation time advance at the display's refresh rate divided by sixty.
// Correct at 60 Hz by coincidence, **double speed on a 120 Hz display**, and
// 0.6755x on a console with every sink folded and the beat declaring at
// `BEAT_STALENESS`. `README.md` promises the oscillator follows the room, and
// the beat grid is that oscillator.
//
// The live count is `App::clock` — `karakuri_environment::clock::Clock`, the
// derivation `karakuri-cli` already made, moved to where ADR-0215 said it goes.
// See `docs/adr/0297-the-panels-tick-is-measured-and-the-fixed-step-a-frame-ran-the-room-at-the-displays-rate.md`.
//
// **This is a comment and not a `#[cfg(test)]` constant**, which is what it
// briefly was: several tests below scan this file for its own code and bound
// the scan at *the first `#[cfg(test)]`*, so a test-only item up here silences
// every one of them — five failed at once and said so. The fixture the tests
// still want is `gpu::STEPS_A_FRAME`, beside the frames that use it.

mod bridge;
pub(crate) use bridge as engine_bridge;
pub(crate) use bridge::*;

mod gfx;
pub(crate) use gfx::*;

mod app;
pub(crate) use app::*;

/// **The command line, then the window.**
///
/// The arguments are read *before* the event loop exists, so a refusal is a
/// line on stderr and an exit code rather than a window that opens and closes.
/// `skip(1)` drops the program's own name, which is `std::env::args`'s first
/// element and not an argument.
fn main() {
    let launch = match sources_from(std::env::args().skip(1)) {
        Ok(launch) => launch,
        // An empty message is `--help`, which is a request rather than a
        // mistake: the usage goes to stdout and the exit is 0.
        Err(why) if why.is_empty() => {
            println!("{USAGE}");
            return;
        }
        Err(why) => {
            eprintln!("{why}");
            eprintln!();
            eprintln!("{USAGE}");
            std::process::exit(2)
        }
    };
    // **Before the window and before a device**, which is where a failure can
    // still be a sentence on a terminal: this reads two files and writes four
    // pairs, and everything after it is inside a `winit` callback where a
    // panic aborts without a message (see `resumed`). It is also the honest
    // place for it — the copies depend on the command line and on nothing
    // else.
    let (scratch, running) = match working_copies(&launch.store, &launch.sources, SLOTS) {
        Ok(made) => made,
        Err(why) => {
            eprintln!("{why}");
            eprintln!();
            eprintln!(
                "that is the material this run was told to play, and the deck runs from \
                 copies of it — so nothing was built and nothing was written."
            );
            std::process::exit(1)
        }
    };
    println!("{}", running_from(&scratch, &running));
    // **Opened here, beside the copies and for their reason.** Every build a
    // watcher makes puts its sources in this store, which is what gives a deck
    // an address a Set file can name — see [`Playing`]. It creates four
    // directories under a root this run has already written into, so it is not
    // a promise the copies above did not already make; it is fatal for the same
    // reason they are, because a run whose store will not open is a run that
    // cannot keep anything.
    let held = match Store::open(&launch.store) {
        Ok(store) => std::sync::Arc::new(store),
        Err(why) => {
            eprintln!("store `{}`: {why}", launch.store.display());
            eprintln!();
            eprintln!(
                "that is where every deck's copies were just written and where a save would \
                 go, so nothing was built."
            );
            std::process::exit(1)
        }
    };
    // **Every version this run compiles, kept where a person can find it**, and
    // the version every deck starts on filed before the window opens — see
    // [`seeded`], which carries the whole of why.
    let snapshots = seeded(&launch.store, &running);
    // **What every surface in this run reads and no surface decides**, made
    // here so that there is one of it: the four bay-head pills write it and
    // the MCP server reads it on every call, and a second handle would be a
    // pill that opens a class the server never sees. All four classes start
    // shut, which is the state ADR-0235 says a run starts in.
    let opening = Opening::closed();
    // **Which files each deck is running, and the one handle that answers it**
    // — see [`karakuri_mcp::Slots`], and [`Aiming::pointing`] for
    // what writes it. Made here beside the opening and for the same reason:
    // the MCP server binds before the window and reads through it on every
    // call, every watcher this run makes publishes its own slot into it, and a
    // second one would be a load the server never sees.
    //
    // **One pair per deck, and they are the working copies rather than the two
    // paths the operator typed.** The server addresses a slot and reads and
    // writes the files behind it, and the files behind a deck are its own copy
    // — see [`working_copies`]. Handing it the typed paths would let a model
    // rewrite the preset library.
    //
    // **Seeded here because the socket is bound before there is a watcher**,
    // and this is what every watcher is about to be pointed at: `watched`
    // builds each slot's launch aim out of this same `running`, and
    // [`Aiming::new`] restates it into this handle the moment the window opens.
    let pointing = mcp::Slots::of(
        running
            .iter()
            .map(|pair| (pair.l1.clone(), vec![pair.l4.clone()]))
            .collect(),
    );
    // **Before the window, for the reason the working copies are**: `serve`
    // binds a socket, and a socket that is already taken has to be a sentence
    // on a terminal. Everything after `run_app` is inside a `winit` callback,
    // where a panic aborts without a message.
    //
    // **Fatal, because `--mcp` was asked for.** A run that went on without it
    // would look exactly like one whose client is connected and idle.
    let mcp = match launch.mcp {
        Some(port) => {
            match mcp::serve(
                port,
                // **Shared and not copied**, which is the whole of the fix:
                // the server resolves an address through this on every call,
                // so a library load that re-points a deck moves what a model
                // reads and writes with it.
                pointing.clone(),
                launch.store.clone(),
                // **True, and not a flag read from anywhere.** Every slot in
                // this program is built over a `watch::Watch` ([`watched`]) and
                // there is no run of this binary that opens a file read-only,
                // so a procedure a model writes is always picked up. This is
                // where that fact is stated to the server, which uses it to
                // tell a client whether a write will reach the screen.
                true,
                opening.clone(),
            ) {
                Ok(reporter) => {
                    // The port bound rather than the one asked for: `--mcp 0`
                    // takes an ephemeral one, and printing the 0 would name a
                    // port that is not the port.
                    println!(
                        "mcp: 127.0.0.1:{} — a model can read and rewrite a deck's procedure, \
                         rewire an input and keep what a deck is playing; every deck is \
                         watched, so a write reaches the screen",
                        reporter.port()
                    );
                    Some(reporter)
                }
                Err(why) => {
                    eprintln!("karakuri: mcp: {why}");
                    std::process::exit(2)
                }
            }
        }
        None => None,
    };
    let event_loop = EventLoop::new().expect("event loop");
    // **Before `run_app`, because that is the last moment the loop is a value
    // rather than a callback.** It is what the MIDI callback wakes this window
    // with — see [`App::waker`], which carries the whole argument for why a
    // control surface is a wake and not a poll.
    let waker = event_loop.create_proxy();
    // **The loop sleeps.** A frame is drawn when something changed it or when
    // `egui` asked for one after a delay it named, and on no other occasion —
    // `App::about_to_wait` sets this again after every iteration and is where
    // the rule actually lives. This is the state it starts in so that the
    // window between here and the first `about_to_wait` is not a spin either.
    event_loop.set_control_flow(ControlFlow::Wait);
    event_loop
        .run_app(&mut App::new(
            launch, running, held, snapshots, mcp, opening, pointing, waker,
        ))
        .expect("run");
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;
