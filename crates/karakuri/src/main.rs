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
//!    other half of that bet: a `Deck` on the same `Device`, with one Set of
//!    its two on air, drawn
//!    through `Present` into the picture's rectangle **and again into deck A's
//!    preview cell**, sampled by `egui` in the same submission. One `Present`
//!    and two targets of different sizes, because `Present::draw` letterboxes
//!    into whatever it is handed. Both are `karakuri_engine::Sink`s and the
//!    frame is one `frame::compose`; the panel goes into that frame's own
//!    encoder through its `finally`, because the panel is not a sink — it does
//!    not receive the composited frame, it samples what a sink produced. See
//!    [`Engine`] and [`Presented`].
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
//! [P-0026](../../../docs/principles/0026-a-guarantee-is-structural-or-it-is-a-convention-that-says-so.md).
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
//! [P-0028](../../../docs/principles/0028-every-control-ends-in-the-same-record.md)
//! is that every control ends in the same record. No record reaches a disk,
//! and no record stream drives time.
//!
//! # What is not wired, and what each would be for
//!
//! **This list used to read `no audio, no MIDI, no MCP, no replay and no
//! session` and that was the whole of it** — five absences and not one purpose,
//! which is a note that cannot be told from a decision. It was duly read as a
//! charter twice. Each line below says what the thing would be *for*, so that
//! whoever reaches one knows what they are reaching for.
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
//! - **MIDI.** A control surface, so a hand reaches a fader without a mouse.
//!   `karakuri-midi` and `examples/surface.map` exist and `--midi-in`
//!   `--midi-map` drive them. **No operation names attaching one**, so this is
//!   a hole in the vocabulary before it is a hole here.
//! - **MCP.** The model's door — the whole reason the instrument is
//!   AI-native — and `karakuri_environment::mcp` is the server. Same shape as
//!   MIDI: **no operation names opening it**, and a panel that opened one
//!   silently would be the opposite of
//!   [P-0030](../../../docs/principles/0030-an-instrument-says-what-it-did.md).
//! - **Replay.** Rendering a recorded session back. This is offline work and
//!   an instrument is not where it belongs; `karakuri-cli --replay` is the
//!   right home for it and no row asks the panel for it.
//! - **Session.** Recording the timeline as it happens, which is *Record the
//!   session* — a `plan` badge with `rec` as its home, so the panel is meant
//!   to reach this one and the transport already draws the button's place.
//!
//! **What is missing is named rather than left to be noticed.** Audio, MIDI,
//! MCP and replay are all `karakuri-environment`'s and all reachable from
//! here; none of them is wired up, because a slice that added them would be
//! unreviewable. The watcher was in that list until the Staging lane needed a
//! producer, and what it took was one function — which is the measure of how
//! far the rest of them are, rather than an argument for doing them all now.
//! What this program's watchers still decline is the store either side of
//! them: nothing is put under a content address and no version is kept, which
//! is what the lane's two operations wait on ([`watched`]). `karakuri-cli` is
//! still what you play a set with.
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
//! channel nobody has asked anything of is: not stepping and contributing
//! nothing to the mix. An operator brings one up by cycling its tally or by
//! loading a Set into it.
//!
//! **Every cell draws, whatever its slot's residency.** An off-air slot is
//! drawn into its own target and never stepped, because the slot nobody is
//! watching is the candidate and the cell is what it is judged from
//! ([P-0080](../../../docs/principles/0080-an-operator-can-see-a-slots-own-material-without-putting-it-on-air.md)).
//! It costs a draw per slot and that draw is outside the governor's
//! arithmetic; the roadmap's *Performance discipline* carries what is owed.
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
//! [P-0072](../../../docs/principles/0072-a-still-panel-costs-nothing-and-what-moves-declares-its-price.md)'s
//! first clause: a panel with nothing changing on it does no per-frame work.
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

use std::alloc::{GlobalAlloc, Layout as AllocLayout, System};
use std::cell::Cell;
use std::sync::Arc;
use std::time::{Duration, Instant};

// **The toolkit comes from `karakuri-console` and is not named in this
// package's manifest.** `egui`, `egui-wgpu` and `egui-winit` move together and
// `egui-wgpu` is what pins `wgpu`; that crate's `lib.rs` re-exports all three
// and says why in as many words — *"a window loop written against this crate
// takes all three from here"*. Depending on them directly would let this
// package pick an `egui-wgpu` that wants a different `wgpu`, and then the
// engine and the panel cannot share a `Device`, which is the whole of what
// ADR-0155 paid a major version for.
use karakuri_console::{egui, egui_wgpu, egui_winit};

use karakuri_console::budget::PANEL_PASS;
use karakuri_console::input::{claim, Claim, CONTROLS};
use karakuri_console::panel::{Dragged, InHand, Knob, Op, Outcome, Panel, Pressed, Released};
use karakuri_console::repaint::{Change, Repaint};
use karakuri_console::room::Room;
use karakuri_console::view::{
    self, arrangement as arrangement_pill, audio_in as audio_in_pill, class_at,
    deck_head as deck_head_row, inspector as inspector_pane, library as library_bay,
    look as look_row, master as master_row, mcp_pill, mixer as mixer_bay, outputs, picture_rect,
    preview_rects, program_head, Ask, AudioAsk, AudioIn, Chosen, Kind, McpPill, Picture, Scope,
    View, DECKS, DECK_LETTERS,
};
// **How many slots a deck can hold**, which is how many this one has — see
// [`SLOTS`]. Not re-exported at the crate root, and asked of the module that
// declares it rather than transcribed here.
use karakuri_engine::deck::MAX_SLOTS;
use karakuri_engine::governor::{Reason, Report, SLOWEST_PRIME_ONE_IN};
// The engine's own `Published`, and its node kinds under the word the address
// uses for them: `Kind` is already the console's *region* kind on this side,
// and one word cannot be two things in one file.
use karakuri_engine::set::{Layering, Published};
use karakuri_engine::transport::Sync as EngineSync;
use karakuri_engine::{
    compose, Blend, Committed, Control, Deck, Event, Gpu, HotSwap, Look, Mask, MaskKind, Present,
    Residency, Set, Sink, Skip, TonemapOp, DEFAULT_BUDGET_MS,
};
use karakuri_environment::{audio, mcp, mix, setfile, watch, Opening};
use karakuri_ir::Kind as Layer;
use karakuri_layout::{Axis, Hit, Layout, NodeId, Point};
use karakuri_operation::gate::{Class, Open};
use karakuri_operation::{BeatSource, BlendMode, GridScale, Operation, SetTransfer, Undecided};
use karakuri_operation_record::{written, Current, Written};
use karakuri_store::record::Record;
use karakuri_store::store::Store;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

/// The window this opens, in logical pixels. Comfortably above the smallest
/// viewport the arrangement is claimed to work at, so nothing starts clamped.
const WINDOW: (f64, f64) = (1440.0, 900.0);

// ---------------------------------------------------------------------------
// What a still panel costs
// ---------------------------------------------------------------------------

/// How long the window has to go untouched before the reading is taken, and
/// the stretch every number in it is measured over.
///
/// **The measurement is the claim now, and it used to be its own opposite.**
/// What was here drove the window for 180 frames and reported what one of them
/// cost; the loop had to spin for the sample to fill, so the number described
/// a program that no longer exists the moment the loop stops spinning.
/// [P-0072](../../../docs/principles/0072-a-still-panel-costs-nothing-and-what-moves-declares-its-price.md)'s
/// first clause claims that nothing is drawn at all while nothing is
/// happening, so that is what is counted: frames drawn in three seconds of an
/// untouched window, and what they allocated.
///
/// **Three seconds** because a 60 Hz spin fills it with 180 frames — the old
/// sample, to the frame, so the two readings are about the same stretch of
/// wall clock — and because a twelve-second run has room for it several times
/// over.
const STILL: Duration = Duration::from_secs(3);

/// How many frames the per-frame sample holds.
///
/// It is a cap and not a target: nothing drives the window to fill it, and a
/// run where nobody touches anything leaves it nearly empty, which is the
/// point. Allocated once, so measuring does not allocate on the path it is
/// measuring.
const SAMPLE: usize = 240;

/// **What the `egui` pass on this panel last read**, and the day it was read
/// — the figure the reading below quotes, and the figure it holds the run it
/// has just taken against.
///
/// It is a constant rather than a sentence because a number written into prose
/// reads as current forever, and this one did. The line that quotes it cited
/// [ADR-0164](../../../docs/adr/0164-the-panel-is-budgeted-rather-than-forbidden-to-allocate.md)'s
/// 184 allocations and 226.2 kB and said the `egui` pass "is still that",
/// which stopped being true when the mixer bay landed — 456 there, and 525
/// once deck B was parked
/// ([ADR-0191](../../../docs/adr/0191-the-panels-parked-deck-is-parked-by-the-governor-or-it-is-a-drawing-of-one.md)).
/// Nobody re-checked it for two commits, because nothing was checking it.
///
/// **Taken on 2026-08-31**, over nine runs of this program on an Apple M4 Pro
/// with nothing touching the window, at the window size [`WINDOW`] opens:
/// every one of the nine read the same per-frame median, and it is the two
/// figures below. What the panel had in it while they were taken is the last
/// paragraph the reading prints. Re-take all three together, several runs at a
/// time — one run is not a number here — and re-date them.
///
/// **The spread was nothing at all, which is a reading and not a guarantee.**
/// The nine of 2026-08-26 disagreed by 14 allocations and these nine agreed to
/// the allocation, because an untouched panel tessellates the same work every
/// frame and nothing in the run varies it. It is not a promise that a tenth
/// run agrees, and it is not a licence to take one: what makes a number here
/// trustworthy is that several runs were asked, and a single run's median is
/// what produced the last wrong one.
///
/// **The reading before this one predicted 1.26x and the panel did 2.89x**,
/// which is kept because it is the argument for the counter rather than
/// against it. 2026-08-26 read 525, and the note beside it reasoned from
/// ADR-0191's **69 allocations and 137.4 kB a strip** that the two mixer
/// strips since would put it near **663 and 969 kB** — inside [`DRIFT`], so
/// the run would have called the sentence current. It was not two strips that
/// landed. Three things the last reading's own *what the panel had in it*
/// paragraph does not mention are on the panel now: the Inspector draws two
/// panes off the running Set, the transport row draws an armed `audio-in` pill
/// over an input measured every frame, and it draws the arrangement pill. A
/// figure predicted from the one change somebody remembered is precisely the
/// figure that goes stale in silence, and re-taking it needs a window, three
/// still seconds and several runs, none of which is reachable from
/// `cargo test`.
const WRITTEN_ALLOCS: u64 = 1518;
const WRITTEN_KB: f64 = 1781.6;
const WRITTEN_ON: &str = "2026-08-31";

/// How far a run may sit from [`WRITTEN_ALLOCS`] before the reading says the
/// sentence quoting it has gone stale.
///
/// **A factor, and a generous one, because an allocation count is not a
/// constant**: a hard equality here would be a guard nobody could keep
/// passing. The nine runs behind 2026-08-26's figure disagreed by 14
/// allocations; the nine behind the current one agreed to the allocation, and
/// one machine's nine agreeing is not a promise the next machine's will. Two
/// is the smallest factor that still catches what actually happened — 184 to
/// the mixer bay's 456 is 2.5x, so a band of two would have said so on the
/// first run after that bay landed, and a band of ten would not have. It is
/// also what caught 525 going to 1518.
///
/// **Two figures are held and the bytes are not**, which is a distinction
/// rather than an omission: the bytes move with the allocation count, so a
/// verdict on them would be the same verdict twice. The second is
/// [`PANEL_PASS`] — what one update of a live region costs, declared under
/// [P-0072](../../../docs/principles/0072-a-still-panel-costs-nothing-and-what-moves-declares-its-price.md)
/// and read by `tests/schedulable.rs` rather than by anything at runtime. It
/// is a different claim from an allocation count and it is the one a
/// schedulability condition is asserted against, so it gets its own verdict.
const DRIFT: f64 = 2.0;

/// **Has the reading moved away from the sentence that quotes it?** `Some` is
/// the factor between them, where that factor is past [`DRIFT`] in either
/// direction.
///
/// Either direction on purpose: a pass that got cheaper makes the sentence
/// exactly as untrue as one that got dearer, and only one of those two is ever
/// noticed by accident.
fn drifted(measured: u64, written: u64) -> Option<f64> {
    let factor = measured.max(written) as f64 / measured.min(written).max(1) as f64;
    (factor > DRIFT).then_some(factor)
}

/// [`drifted`] for a figure in milliseconds, which is what a **cost** is.
///
/// The same band and the same both-directions rule, said again for `f64`
/// because the two numbers are of different kinds and neither is convertible
/// into the other without saying something untrue about it. A run that read
/// nothing at all cannot divide, and a zero-length sample never reaches here —
/// the caller is inside `if !self.frames.is_empty()`.
fn drifted_ms(measured: f64, written: f64) -> Option<f64> {
    let factor = measured.max(written) / measured.min(written).max(f64::MIN_POSITIVE);
    (factor > DRIFT).then_some(factor)
}

/// The allocator, counting. **Per thread, not per process** — `wgpu` allocates
/// on threads of its own and a process-wide counter would attribute that to
/// the `egui` pass, which is the one number this exists to get right.
struct Counting;

thread_local! {
    static ALLOCS: Cell<u64> = const { Cell::new(0) };
    static BYTES: Cell<u64> = const { Cell::new(0) };
}

/// `(allocations, bytes)` this thread has asked for since it started.
/// Reallocations count as one allocation of the new size, which overstates a
/// growing `Vec` and is the conservative direction.
fn counted() -> (u64, u64) {
    (
        ALLOCS.try_with(Cell::get).unwrap_or(0),
        BYTES.try_with(Cell::get).unwrap_or(0),
    )
}

fn count(size: usize) {
    let _ = ALLOCS.try_with(|c| c.set(c.get() + 1));
    let _ = BYTES.try_with(|c| c.set(c.get() + size as u64));
}

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: AllocLayout) -> *mut u8 {
        count(layout.size());
        System.alloc(layout)
    }

    unsafe fn alloc_zeroed(&self, layout: AllocLayout) -> *mut u8 {
        count(layout.size());
        System.alloc_zeroed(layout)
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: AllocLayout, new_size: usize) -> *mut u8 {
        count(new_size);
        System.realloc(ptr, layout, new_size)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: AllocLayout) {
        System.dealloc(ptr, layout);
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

/// What one frame cost.
#[derive(Debug, Clone, Copy, Default)]
struct Cost {
    /// **The engine's half**: [`compose`] from its first statement up to the
    /// moment it hands the encoder back for the panel — both sinks' `acquire`,
    /// the committing closure, the tone-map write, `Deck::begin_frame` with
    /// every install in it, the deck's render, and the present passes into
    /// whichever sinks took the frame. **Two present passes and one deck
    /// render** with both sinks on, because an audition is a second present of
    /// the same canvas and not a second simulation of it — and one pass, or
    /// none, when a region is folded away and its sink refuses.
    ///
    /// **Where it stops is the start of [`Cost::paint`] and not a second
    /// clock**: the panel's half begins when `compose` calls `finally`, so the
    /// two are adjacent by construction and no part of the frame falls between
    /// them. CPU time only, like everything else here; what the GPU then does
    /// with the command buffer is not on this clock, and
    /// `docs/contributing.md` §1 says why there is no other one.
    engine: Duration,
    /// `take_egui_input` through `tessellate`: the whole immediate-mode pass,
    /// including this console's own layout walk and every shape it emits.
    ui: Duration,
    /// Uploading the tessellated geometry, recording the panel's render pass,
    /// and the one submission that carries both halves — the whole of what the
    /// program records from inside [`compose`]'s `finally`, plus `compose`'s
    /// own tail: `Frame::finish`, and each sink's `present`, which for these
    /// two sinks is nothing at all. Excludes `Queue::present` on the surface,
    /// which is the display's pace and not a cost.
    paint: Duration,
    /// **Uploading `egui`'s texture deltas, out of [`Cost::paint`]** — the
    /// font atlas and its patches. Zero on a steady frame, because the atlas
    /// is built once.
    ///
    /// This and the two below exist because of what happens when they do not.
    /// The split was taken once with temporary instrumentation, published as a
    /// conclusion and removed, and the next machine asked to answer *how much
    /// of the frame is the cost of moving data to the GPU* could not: the
    /// question is about `buffers` and the readout stopped at `paint`. A
    /// measurement that has to be re-instrumented to be repeated is one number
    /// rather than a series.
    textures: Duration,
    /// **Uploading the tessellated geometry, out of [`Cost::paint`]** — and
    /// the number the caching decision turns on.
    ///
    /// **It is not the price of moving bytes**, which took three machines to
    /// establish and is `docs/adr/0167-the-panel-keeps-re-uploading-what-did-not-change.md`.
    /// The discrete GPU that has to cross a bus pays 0.022 ms; an APU that
    /// crosses nothing pays 0.010; this machine, which also crosses nothing,
    /// pays 0.166 — seven times the one with the bus. It is what a backend's
    /// upload path costs, and the decision it was printed for is taken: **not
    /// re-uploading the unchanged panel is worth at most 4% of a frame, and
    /// the dirty-tracking it needs is not.**
    buffers: Duration,
    /// **Recording the panel's render pass, out of [`Cost::paint`]** — the
    /// view, the pass, and `egui`'s draw calls into it. Recording only; what
    /// the GPU then does with it is on no clock here.
    record: Duration,
    /// **The submission alone, out of [`Cost::paint`]** — `Queue::submit` and
    /// `finish` on the frame's encoder, which is [`compose`]'s last act and is
    /// why this is measured from inside `finally` to after `compose` returns.
    /// The only other thing in that window is each sink's `present`, and both
    /// of this program's are `Ok(())` — it is five sixths of `paint`.
    ///
    /// Printed because the whole of the rest of `paint` is what caching a bay
    /// into a texture would make cheaper, and this is not: it is `wgpu`'s
    /// per-submission cost, it is proportional to what was recorded rather
    /// than to what the GPU then does with it, and it is unmoved by a canvas
    /// 256 times the area. It is host-side work and not a wait — measured
    /// against `CLOCK_THREAD_CPUTIME_ID` it burns 99% of its wall time on the
    /// CPU. **The wait is [`Cost::wait`], which is a different number
    /// entirely.**
    submit: Duration,
    /// **What the frame spent blocked in `get_current_texture`**, waiting for
    /// the display to free a swapchain image. `PresentMode::Fifo`, so at 60 Hz
    /// this is most of the 16.6 ms and the frame is not paying for it: on the
    /// same clock as above it burns under 1% of itself on the CPU.
    ///
    /// It is in none of the three numbers above, which is why it is here — a
    /// reader who sums those three and compares the total to a frame gets an
    /// answer that is 12% of the truth, and the missing 88% is this doing
    /// nothing on purpose. Switching to `PresentMode::Immediate`, which this
    /// adapter does offer, moves it and nothing else.
    wait: Duration,
    /// Allocations and bytes during `ui`, on this thread.
    allocs: u64,
    bytes: u64,
}

impl Cost {
    /// **What the whole frame cost on the CPU**: the three fields the reading
    /// below adds up under *"the whole frame is a median"*, for one frame
    /// rather than for the median of each.
    ///
    /// The three tile the frame exactly and do not overlap — `engine` ends
    /// where `paint` begins, by construction, and `ui` is the `egui` pass
    /// before either — and [`Cost::wait`] is deliberately not in it: blocking
    /// in `get_current_texture` is the display's pace and not a price, which
    /// is the sentence that field carries.
    fn whole(&self) -> Duration {
        self.engine + self.ui + self.paint
    }
}

/// What was drawn while nobody was touching the window. **This is the number**
/// P-0072's first clause is about, and the clause says every field of it is
/// zero.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Still {
    frames: usize,
    allocs: u64,
    bytes: u64,
}

/// The sample, the stillness, and the summary printed once.
struct Costs {
    /// What the frames that *were* asked for cost.
    frames: Vec<Cost>,
    /// Every frame drawn, including any past the end of `frames`.
    drawn: usize,
    /// When the window was last touched by anything at all.
    quiet_since: Instant,
    /// **A frame is owed to something that happened**, and the next one drawn
    /// is that frame rather than a frame drawn on a still panel.
    ///
    /// Without it the reading blames the panel for the frame it was asked for:
    /// an event resets the stretch and the frame it asked for lands a
    /// millisecond into the new one, so a window that did exactly the right
    /// thing reports having drawn on an untouched panel.
    owed: bool,
    /// What has been drawn since `quiet_since` that nothing asked for.
    still: Still,
    /// **The frame just drawn**, kept so the transport row can print what it
    /// cost.
    ///
    /// Not a second measurement and not a second sample: it is the last
    /// [`Cost`] [`Costs::push`] was handed, which `frames` stops keeping after
    /// [`SAMPLE`] of them because that vector is a sample rather than a
    /// history. A readout wants the last frame and the reading wants the
    /// sample; both are the same numbers.
    last: Option<Cost>,
    said: bool,
    /// **What ran it**, as the adapter reports it: the backend, the adapter's
    /// own name, and whether it calls itself discrete.
    ///
    /// Printed because a readout that does not name its backend is how a
    /// measurement gets written into a table as though it were another one's.
    /// `WGPU_BACKEND` was honoured by nothing for months and the failure was
    /// invisible precisely here — twenty-five runs were recorded as DX12 and
    /// were Vulkan, and nothing on screen could have said otherwise
    /// (`docs/adr/0168-a-backend-override-is-honoured-because-a-no-op-cannot-be-caught.md`).
    /// The adapter is asked rather than the environment, so an override that
    /// does not take reads as what it is.
    taken_on: String,
    /// **Whether anything in the Program bay is making texels** — the
    /// picture, a deck auditioning in a preview cell, or both. [`live`] is the
    /// rule and this is the frame's answer to it.
    ///
    /// It decides which sentence the reading prints about the frames it
    /// counted, and nothing else: the frames are counted the same way either
    /// way, which is the point — the number is not adjusted for knowing the
    /// answer.
    live: bool,
    /// **What the panel declared it needed**, as `View::animating` answered it
    /// on the last frame: the soonest staleness out of the regions that are
    /// declaring, and `None` only when none of them is.
    ///
    /// It is here for one sentence, and the sentence was wrong without it.
    /// With nothing in the Program bay making texels the reading used to blame
    /// the only other thing it knew about — an `egui` repaint delay answered
    /// immediately — and on this program that is never the answer: folding the
    /// picture and the preview row away leaves the window drawing 28.0 to 28.3
    /// frames a second over two runs on 2026-08-26, which is the roll's
    /// declared 30 Hz and not a mishandled delay. Folding the mixer bay away
    /// as well takes it to 0 frames, because the chip that declares the 30 Hz
    /// is then not laid out (ADR-0193).
    /// A reading that names the wrong cause is worse than one that names none.
    ///
    /// **Those two readings were taken before the beat declared**, and the
    /// beat declares whenever the transport row is drawn and there is an
    /// engine behind it — 24.7 ms, about forty a second, whether or not
    /// anything is pending
    /// ([ADR-0212](../../../docs/adr/0212-the-beat-is-a-light-that-travels-and-it-declares-for-itself.md),
    /// [P-0077](../../../docs/principles/0077-continuous-motion-is-how-a-stopped-panel-announces-itself.md)).
    /// So this window's `None` now needs the transport row folded away as
    /// well, which is a **fourth** fold and is the arm below saying so.
    declared: Option<Duration>,
}

impl Costs {
    fn new() -> Costs {
        Costs {
            frames: Vec::with_capacity(SAMPLE),
            drawn: 0,
            quiet_since: Instant::now(),
            owed: false,
            still: Still::default(),
            last: None,
            said: false,
            taken_on: String::from("an adapter nobody asked"),
            live: false,
            declared: None,
        }
    }

    /// **Something touched the window**, so the stillness starts again from
    /// here and what was drawn during the last stretch is no longer about a
    /// still panel.
    ///
    /// Every window event that is not a frame this loop asked for itself
    /// counts, including the ones the operator did not cause — a move, a
    /// focus, an occlusion. Resetting too eagerly only ever makes the reading
    /// harder to reach, never easier to pass.
    fn touched(&mut self) {
        self.quiet_since = Instant::now();
        self.still = Still::default();
    }

    /// **A frame was asked for by something that happened**, so the next one
    /// drawn is not on the panel's account.
    ///
    /// Every `request_redraw` in this file goes through here except one, and
    /// the exception is the point of the reading: **the frame `egui` asked for
    /// after a delay it named.** The distinction is between `egui` saying *I
    /// have not finished drawing what you just asked me to* — a `repaint_delay`
    /// of zero, a second pass of a frame already owed, which is what it does
    /// for a pass or two while the font atlas settles — and `egui` saying
    /// *wake me in 250 ms*, which is an animation and is per-frame work on a
    /// panel nobody is touching. The first goes through here and the second
    /// does not, so a delay mishandled into a spin shows in the reading as the
    /// frames it actually drew.
    ///
    /// The other way it can fail is loud rather than quiet: something asking
    /// for frames without pause never lets the window be still for [`STILL`],
    /// and then no reading is printed at all. **A run of this program that
    /// prints no reading is that failure.**
    fn owes(&mut self) {
        self.owed = true;
        self.touched();
    }

    fn push(&mut self, cost: Cost) {
        self.drawn += 1;
        self.last = Some(cost);
        if !self.owed {
            self.still.frames += 1;
            self.still.allocs += cost.allocs;
            self.still.bytes += cost.bytes;
        }
        self.owed = false;
        if self.frames.len() < SAMPLE {
            self.frames.push(cost);
        }
    }

    /// **Frames a second**: what was drawn on the untouched window, over the
    /// stretch it was drawn in.
    ///
    /// The stretch is the caller's because the two callers are at different
    /// points in it and neither may guess the other's. [`Costs::say`] is taken
    /// exactly [`STILL`] after `quiet_since` and says so in the sentence
    /// above the number; the transport row is asked on every frame and its
    /// stretch is however much of one has elapsed. One quotient, two stretches
    /// — and a second expression of *frames over seconds* would be the readout
    /// and the reading disagreeing about the rate of the same window.
    fn rate_over(&self, stretch: f64) -> f64 {
        self.still.frames as f64 / stretch
    }

    /// **The rate as it stands, for the transport row** — or `None` where
    /// there is not yet a stretch with a frame in it to divide.
    ///
    /// `None` is the honest answer twice over. Just after something touched
    /// the window there is no stretch, and a rate over no time is an infinity.
    /// And a window with nothing live on it stops asking for frames entirely,
    /// so the stretch goes on growing while the frames do not — which is a
    /// rate falling towards zero and is exactly what the window is doing.
    fn rate_now(&self) -> Option<f64> {
        let stretch = self.quiet_since.elapsed().as_secs_f64();
        (self.still.frames > 0 && stretch > 0.0).then(|| self.rate_over(stretch))
    }

    /// When the reading is due, and `None` once it has been taken. It is also
    /// what keeps the loop on a deadline until then — see
    /// [`App::about_to_wait`].
    fn due(&self) -> Option<Instant> {
        match self.said {
            true => None,
            false => Some(self.quiet_since + STILL),
        }
    }

    /// The reading. Median and worst rather than a mean for the per-frame
    /// figures: a frame path is judged by its tail.
    /// `capacity` and `material` are the run's, handed in rather than read off
    /// a constant: this program takes its `.kir` pair from the command line and
    /// a load can move a slot off it, so what the engine half of this reading
    /// was taken over is only known at run time — and is every slot's name in
    /// slot order rather than one. See [`Engine::capacity`],
    /// [`Sources::material`] and [`Gfx::material`].
    fn say(&mut self, capacity: u32, material: &str) {
        if self.said {
            return;
        }
        self.said = true;

        println!();
        println!(
            "{}",
            match self.live {
                // There is no still panel to cost while anything is live, and
                // calling it one would be the reading describing a program
                // that is not running — which is the failure the sample this
                // replaces made, one revision ago. **"Something live" rather
                // than "a live picture"**: the picture can be folded away with
                // deck A still auditioning under it, and the window is no more
                // still then than it was before.
                true => "what an untouched window costs with something live in it:",
                false => "what a still panel costs, measured on this window:",
            }
        );
        println!(
            "  over {:.1} s with nothing touching it: {} frames drawn, {} allocations, \
             {} bytes",
            STILL.as_secs_f64(),
            self.still.frames,
            self.still.allocs,
            self.still.bytes
        );
        let rate = self.rate_over(STILL.as_secs_f64());
        match (self.live, self.still == Still::default()) {
            // The reading this was written for. It is reachable with the
            // picture, the preview row, the mixer bay **and the transport
            // row** folded away — the first two stop the texels and the last
            // two stop the two declarations (ADR-0193) — and with any one of
            // the four on screen it is not. The fourth is P-0077 arriving:
            // the beat is a light travelling the grid, it declares for as
            // long as it is drawn, and a console claiming to show a live
            // instrument has something moving on it (ADR-0212).
            (false, true) => println!(
                "  so P-0072's first clause holds here: no per-frame work is done to \
                 redraw what nobody has touched and nothing has moved."
            ),
            (false, false) => match self.declared {
                // **The panel said it needed them**, and that is P-0072's
                // second clause working rather than its first failing: the
                // parked deck's tally declares a staleness and the window
                // serves it. Measured on 2026-08-26 with the picture and the
                // preview row folded away and the mixer bay on screen: 28.0
                // and 28.3 frames a second over two runs, at 427 and 425
                // allocations a frame.
                //
                // **Folding the mixer bay away takes this arm out of reach**,
                // and that is ADR-0193: the slot stays parked, the chip is not
                // drawn, and a region that is not laid out declares nothing —
                // the same two runs read 0 frames with the bay folded, which
                // is the arm above. It read 28.7 to 29.0 a second at 260
                // allocations before that change, which is the defect that
                // record closes.
                Some(staleness) => println!(
                    "  so P-0072's first clause does NOT hold here, and the reason is a \
                     declaration rather than a fault: {}, and the soonest staleness \
                     declared is {:.1} ms — about {:.0} frames a second. Folding the \
                     region that draws it ends its term: a region that is not laid out \
                     declares nothing (ADR-0193).",
                    match staleness == view::BEAT_STALENESS {
                        // The one that runs whether or not anything is
                        // happening, which is the whole of why it is here.
                        true =>
                            "the beat grid is a light travelling the transport row, \
                                 and it moves for as long as the console is live rather \
                                 than while something is pending (P-0077, ADR-0212)",
                        false =>
                            "something on this panel is parked and the mixer's tally \
                                  is rolling toward a residency nobody granted (ADR-0190)",
                    },
                    staleness.as_secs_f64() * 1000.0,
                    1.0 / staleness.as_secs_f64()
                ),
                // Nothing live, nothing declared, and frames drawn anyway.
                None => println!(
                    "  so P-0072's first clause does NOT hold here — something is asking \
                     for frames on an untouched window, nothing on the panel is making \
                     texels and nothing has declared a staleness, so the likeliest \
                     something is an `egui` repaint delay answered immediately instead of \
                     waited out."
                ),
            },
            // **The expected reading now**, and the whole of what this run is
            // for. It is stated as a price rather than as a failure, because
            // that is what it is: the clause is about a panel with nothing
            // changing on it, and a live engine frame is something changing on
            // it.
            (true, _) => {
                println!(
                    "  so P-0072's first clause has stopped holding, and the reason is the \
                     engine: there is a live frame in the Program bay — the picture, deck \
                     A auditioning in the preview row under it, or both — so every one of \
                     those frames was asked for by what is live rather than by anybody \
                     touching the window."
                );
                println!(
                    "  that is {rate:.1} frames a second, against 0 with the engine out — \
                     which is the whole of the difference, since what one frame costs is \
                     below and did not change."
                );
                println!(
                    "  it is P-0072's second clause from here — what moves declares its \
                     price — and this is the price, measured. Two regions declare it: the \
                     transport row for as long as the beat grid is drawn (P-0077, \
                     ADR-0212) and the mixer bay while something in it is pending. \
                     Nothing in this run schedules, caches the panel to a texture or \
                     arbitrates between the two; the number is what the next decision gets \
                     made on."
                );
            }
        }

        if !self.frames.is_empty() {
            let mut engine: Vec<f64> = self.frames.iter().map(|c| ms(c.engine)).collect();
            let mut ui: Vec<f64> = self.frames.iter().map(|c| ms(c.ui)).collect();
            let mut paint: Vec<f64> = self.frames.iter().map(|c| ms(c.paint)).collect();
            let mut textures: Vec<f64> = self.frames.iter().map(|c| ms(c.textures)).collect();
            let mut buffers: Vec<f64> = self.frames.iter().map(|c| ms(c.buffers)).collect();
            let mut record: Vec<f64> = self.frames.iter().map(|c| ms(c.record)).collect();
            let mut submit: Vec<f64> = self.frames.iter().map(|c| ms(c.submit)).collect();
            let mut wait: Vec<f64> = self.frames.iter().map(|c| ms(c.wait)).collect();
            // **What drawing the panel costs**, which is the figure
            // `karakuri_console::budget::PANEL_PASS` declares and the reason
            // this vector exists: the immediate-mode pass, plus the panel's
            // own texture and geometry uploads and the recording of its render
            // pass. Not `engine`, which is the governor's and would be counted
            // twice (P-0072); not `wait`, which is doing nothing on purpose;
            // and **not `submit`**, which is larger than all of this together
            // and carries the engine's half of the frame as well, so charging
            // it to a region would charge a region for a frame it did not ask
            // for.
            let mut draw: Vec<f64> = self
                .frames
                .iter()
                .map(|c| ms(c.ui + c.textures + c.buffers + c.record))
                .collect();
            // **Median, where the figure this replaces was a mean.** The mean
            // was over 180 frames and the first one was lost in it; the sample
            // here is however many frames somebody asked for, which on a run
            // nobody touches is three — and the first of those builds the font
            // atlas and allocates ten times what a frame does. A mean of three
            // is that one frame with two others attached.
            let mut allocs: Vec<u64> = self.frames.iter().map(|c| c.allocs).collect();
            let mut bytes: Vec<u64> = self.frames.iter().map(|c| c.bytes).collect();
            engine.sort_by(f64::total_cmp);
            ui.sort_by(f64::total_cmp);
            paint.sort_by(f64::total_cmp);
            textures.sort_by(f64::total_cmp);
            buffers.sort_by(f64::total_cmp);
            record.sort_by(f64::total_cmp);
            submit.sort_by(f64::total_cmp);
            wait.sort_by(f64::total_cmp);
            draw.sort_by(f64::total_cmp);
            allocs.sort_unstable();
            bytes.sort_unstable();
            let n = self.frames.len();

            println!();
            println!(
                "what a frame costs when something asks for one, over the {} drawn so far \
                 ({} sampled):",
                self.drawn, n
            );
            println!(
                "  engine pass  median {:.3} ms   p95 {:.3} ms   worst {:.3} ms",
                engine[n / 2],
                engine[n * 95 / 100],
                engine[n - 1]
            );
            println!(
                "  egui pass    median {:.3} ms   p95 {:.3} ms   worst {:.3} ms",
                ui[n / 2],
                ui[n * 95 / 100],
                ui[n - 1]
            );
            println!(
                "  upload+pass  median {:.3} ms   p95 {:.3} ms   worst {:.3} ms",
                paint[n / 2],
                paint[n * 95 / 100],
                paint[n - 1]
            );
            // **The four parts of `upload+pass`, and the reason they are
            // printed rather than derived.** `submit` alone told the caching
            // decision what it was *not* — the submission is not what caching
            // a bay into a texture would make cheaper — without telling it
            // what it was. The upload is the line that decision turns on, and
            // it is worth what a bus costs on the machine reading it, so it
            // has to be a number this program prints on every machine rather
            // than one somebody instruments for once.
            println!(
                "    of which texture uploads  median {:.3} ms   p95 {:.3} ms   worst {:.3} ms — \
                 the font atlas, built once, so zero on a steady frame",
                textures[n / 2],
                textures[n * 95 / 100],
                textures[n - 1]
            );
            println!(
                "    of which buffer uploads   median {:.3} ms   p95 {:.3} ms   worst {:.3} ms — \
                 the tessellated geometry. Not the price of crossing a bus: it is seven times \
                 larger here than on a discrete GPU that crosses one (ADR-0167)",
                buffers[n / 2],
                buffers[n * 95 / 100],
                buffers[n - 1]
            );
            println!(
                "    of which record the pass  median {:.3} ms   p95 {:.3} ms   worst {:.3} ms",
                record[n / 2],
                record[n * 95 / 100],
                record[n - 1]
            );
            println!(
                "    of which submit  median {:.3} ms   p95 {:.3} ms   worst {:.3} ms — \
                 `wgpu`'s per-submission work, not the GPU's and not a wait",
                submit[n / 2],
                submit[n * 95 / 100],
                submit[n - 1]
            );
            println!(
                "  waiting for vsync  median {:.3} ms   p95 {:.3} ms   worst {:.3} ms — \
                 blocked in `get_current_texture`, and in none of the three above",
                wait[n / 2],
                wait[n * 95 / 100],
                wait[n - 1]
            );
            println!(
                "  the egui pass allocates a median {} times a frame, {:.1} kB a frame \
                 (worst {} and {:.1} kB, which is the first frame building the font atlas)",
                allocs[n / 2],
                bytes[n / 2] as f64 / 1024.0,
                allocs[n - 1],
                bytes[n - 1] as f64 / 1024.0
            );
            println!(
                "  the whole frame is a median {:.3} ms, so at {:.1} frames a second the \
                 loop is spending {:.1}% of a second drawing.",
                engine[n / 2] + ui[n / 2] + paint[n / 2],
                rate,
                (engine[n / 2] + ui[n / 2] + paint[n / 2]) * rate / 10.0
            );
            println!(
                "  that per-frame price is what immediate mode pays by construction, and it \
                 is no longer ADR-0164's: that record measured 184 allocations and 226.2 kB \
                 a frame here with every bay empty, which this panel has not been since the \
                 mixer bay landed — 456 allocations there, and 525 once deck B was parked \
                 (ADR-0191). Taken again on {WRITTEN_ON} over nine runs of this program \
                 with nothing touching the window: {WRITTEN_ALLOCS} allocations and \
                 {WRITTEN_KB:.1} kB a frame, which is what every one of the nine read — to \
                 the allocation, and to the tenth of a kilobyte. What the panel had in it \
                 while they were taken is the last paragraph below. What ADR-0164 is still right about is that \
                 the price is paid on every frame drawn; what changed is how many frames pay \
                 it — 0 with a still panel and nothing in the Program bay, and the rate above \
                 with anything live in it."
            );
            // **The sentence above is checked against the run that has just
            // been taken**, which is the only place either can be: the number
            // needs a window, three seconds of nobody touching it and a
            // device, and none of those is reachable from `cargo test`. So the
            // claim and its check are printed together, and the figure in the
            // prose is the figure being checked rather than a second copy of
            // it.
            match drifted(allocs[n / 2], WRITTEN_ALLOCS) {
                Some(factor) => println!(
                    "  and THIS run read {}, which is {factor:.1}x that — past the {DRIFT:.0}x \
                     this file will quote a figure across. **The sentence above is stale.** \
                     Re-take it over several runs of this program, write what the panel had in \
                     it, and re-date `WRITTEN_ALLOCS`, `WRITTEN_KB` and `WRITTEN_ON` in \
                     `crates/karakuri/src/main.rs` — which is what nobody did for the two commits before \
                     this line existed.",
                    allocs[n / 2]
                ),
                None => println!(
                    "  and THIS run read {}, within {DRIFT:.0}x of that, so the sentence above \
                     is still one this window produces.",
                    allocs[n / 2]
                ),
            }
            // **What P-0072 calls a cost, measured and held against what
            // declares it.** `budget::PANEL_PASS` is a constant somebody wrote
            // down — ADR-0164 refuses a schedule made of measurements, because
            // one reorders itself with the machine's noise — and a constant
            // that nothing checks is the failure `WRITTEN_ALLOCS` above exists
            // for, one number along. So the declaration and the reading are
            // printed together, and this is the only place either can be: the
            // figure needs a window, three seconds of nobody touching it and a
            // device, and none of those is reachable from `cargo test`.
            //
            // **It reports and does not fail**, which is deliberate. This
            // machine reads about a sixth of these numbers with its other
            // cores loaded, and a gate on a millisecond here would be one
            // nobody could keep passing — flaky is worse than broken
            // (`docs/contributing.md` §1). What *is* asserted, without a
            // clock, is that every region declares this one constant:
            // `tests/schedulable.rs`.
            println!(
                "  drawing the panel is a median {:.3} ms of that — the egui pass, its \
                 texture and geometry uploads and the recording of its render pass, which \
                 is what one update of a live region costs under P-0072. The submission is \
                 not in it: it carries the engine's half of the frame as well. \
                 `karakuri_console::budget::PANEL_PASS` declares {:.3} ms,",
                draw[n / 2],
                ms(PANEL_PASS),
            );
            match drifted_ms(draw[n / 2], ms(PANEL_PASS)) {
                Some(factor) => println!(
                    "  and THIS run read {:.3}, which is {factor:.1}x that — past the \
                     {DRIFT:.0}x this file will quote a figure across. **The declared cost \
                     is stale.** Re-take it over several runs of this program and rewrite \
                     `PANEL_PASS` in `crates/karakuri-console/src/budget.rs`, with the \
                     machine and the date beside it, because both schedulability \
                     conditions are asserted against that number and nothing else measures \
                     it.",
                    draw[n / 2]
                ),
                None => println!(
                    "  and THIS run read {:.3}, within {DRIFT:.0}x of that, so the declared \
                     cost is still one this window produces.",
                    draw[n / 2]
                ),
            }
            println!("  taken on {}", self.taken_on);
            println!(
                "  the panel half is taken on this window at {:.0}x{:.0} logical, drawing a \
                 live picture, four preview cells with deck A auditioning in one and three \
                 off, the mixer bay with a strip in every one of its four tracks, the \
                 transport row with its `audio-in` and arrangement pills, the outputs row \
                 and deck B's parked \
                 tally rolling once a second — over the Library bay's scope row and however \
                 many rows the scope marked in it lists, over the Master bay's out row, over \
                 the Inspector's two panes read off the running Set, and over Staging and \
                 Sequencer, which are a head and nothing else. \
                 That is NOT the workspace's \
                 reference workload. The \
                 engine half is one Set of `{}` — {} elements at {}x{}, one step a frame — the \
                 deck's other three slots are allocated, one of them parked, and an \
                 allocated slot neither steps nor draws, so none of them is in these \
                 numbers — and \
                 that one canvas presented twice — into the picture's rectangle, \
                 and again into deck A's preview cell, both of which are the \
                 canvas's own shape. It is the workspace's reference workload \
                 (docs/contributing.md §1) only while nobody passed a pair on the command \
                 line: 262144 elements at 1280x720 is what `drift_shell.kir` declares and \
                 what a bare `cargo run -p karakuri` therefore runs, and the figures above \
                 are comparable with the rest of this repository's exactly that far. Host \
                 clock, debug profile with dependencies at opt-level 3.",
                WINDOW.0, WINDOW.1, material, capacity, CANVAS.0, CANVAS.1
            );
            println!(
                "  and every figure above is taken on a core that spends the vsync wait \
                 asleep. On THIS machine that matters a great deal — the identical run with \
                 the other cores loaded reports about a sixth of these numbers, proportions \
                 unchanged — and it is this machine's power management rather than a rule: \
                 two Windows machines were asked the same way and got 1.3x and 1.6x WORSE \
                 under load, which is ordinary contention. Compare ratios, not magnitudes."
            );
        }
        println!();
        println!(
            "{}",
            match self.live {
                // **Two sinks, so two folds.** This said "fold the picture
                // away (f over it) and it does" while deck A's cell was
                // drawing in the row underneath, which is a sentence that
                // sends an operator to watch a window that is still drawing at
                // full rate — the same false claim, in the same place, that
                // cost this file 270 frames once already.
                true =>
                    "the loop asks for the next frame from inside the last one for as long \
                     as anything is making texels, so `ControlFlow::Wait` never gets to \
                     block. Two things are: the picture, and the four cells in the preview \
                     row under it — all four of them, because every slot is drawn whatever \
                     its residency. Fold the picture away (f over it) and the cells keep \
                     the loop awake on their own; fold the preview row away as well \
                     and nothing is making texels — and the window still draws, because \
                     the beat grid declares a deadline of its own for as long as it is on \
                     screen (P-0077, ADR-0212) and the mixer bay declares another while \
                     deck B is parked. `ControlFlow::Wait` blocks when all three are gone, \
                     which is a fourth fold, and it is what P-0072's remaining clauses are \
                     for.",
                false =>
                    "the loop is on `ControlFlow::Wait` from here: it does nothing at all \
                     until the window is touched or `egui` names a deadline of its own.",
            }
        );
        println!();
    }
}

/// Say what failed, say whether an override is the likely reason, and stop.
///
/// Never returns, so it can stand where a `?` cannot: this is inside a `winit`
/// callback, where a panic aborts rather than unwinds and takes the message
/// with it.
fn no_gpu(what: &str) -> ! {
    eprintln!("{what}");
    match std::env::var("WGPU_BACKEND") {
        Ok(want) => eprintln!(
            "WGPU_BACKEND is set to `{want}`, and this codebase honours it — so the most likely \
             reason is that this machine has no {want}. Unset it to take whatever the machine \
             offers."
        ),
        Err(_) => eprintln!(
            "WGPU_BACKEND is not set, so this is the machine's own default backend failing \
             rather than an override."
        ),
    }
    std::process::exit(1)
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

// ---------------------------------------------------------------------------
// The readout: English for what the model returned. No window, no device.
// ---------------------------------------------------------------------------

/// The panel and the view, plus the words for what just happened.
struct Readout {
    panel: Panel,
    view: View,
    /// **What the operator has opened to a model**, and the one piece of state
    /// in this struct that is neither the panel's nor a reading of the engine.
    ///
    /// It is a handle rather than a value because the whole point of it is that
    /// a *second* reader has it: `karakuri_environment::mcp::serve` takes a
    /// clone and reads it on every call, so an opening is live rather than a
    /// snapshot taken at startup. `View::opening` is this handle read once a
    /// frame; this is the model of record.
    ///
    /// **This process serves MCP when `--mcp` names a port**, and the server
    /// is handed this same handle rather than a copy, so the four pills and the
    /// audit read one value. Without the flag the pills still write it and only
    /// this program and its tests read it back.
    opening: Opening,
}

impl Readout {
    fn new(width: f32, height: f32) -> Readout {
        Readout {
            panel: Panel::new(width, height),
            view: View::new(Room::Day),
            // Four classes shut, which is what a run starts with (ADR-0235).
            opening: Opening::closed(),
        }
    }

    // -- the words ------------------------------------------------------

    fn label(&self, id: NodeId) -> String {
        match self.panel.layout().name(id) {
            Some(name) => name.to_owned(),
            None => "(unnamed split)".to_owned(),
        }
    }

    /// The two regions a boundary is between. A split is often unnamed — the
    /// console's body row is, deliberately — so `split #0` alone does not say
    /// which boundary the pointer has hold of, and the pair does.
    fn pair(&self, split: NodeId, index: usize) -> String {
        match self.panel.pair(split, index) {
            Some((a, b)) => format!("{} | {}", self.label(a), self.label(b)),
            None => "no pair".to_owned(),
        }
    }

    // -- input ----------------------------------------------------------

    fn press(&mut self, p: Point) {
        match self.panel.press(p) {
            Pressed::Grabbed {
                split,
                index,
                axis,
                at,
                offset,
            } => println!(
                "press ({:.0}, {:.0}): the boundary {} — divider #{} of {}, {:?} — is at \
                 {:.1}, grabbed {:+.1} from it",
                p.x,
                p.y,
                self.pair(split, index),
                index,
                self.label(split),
                axis,
                at,
                offset
            ),
            Pressed::NoPair { .. } => {
                println!("press ({:.0}, {:.0}): a divider with no pair", p.x, p.y)
            }
            Pressed::Region { id, rect } => println!(
                "press ({:.0}, {:.0}): region {} at {:.0},{:.0} {:.0}x{:.0}",
                p.x,
                p.y,
                self.label(id),
                rect.x,
                rect.y,
                rect.w,
                rect.h
            ),
            Pressed::Nothing => println!("press ({:.0}, {:.0}): nothing", p.x, p.y),
        }
    }

    /// A move with something in hand. A boundary drag says what it did here;
    /// **a fader drag hands its operation back**, because acting on one takes
    /// the deck and the deck is not the readout's.
    fn moved(&mut self, p: Point) -> Option<Operation> {
        match self.panel.moved(p)? {
            boundary @ Dragged::Boundary { .. } => {
                println!("{}", self.say_drag(boundary));
                None
            }
            Dragged::Fader(operation) => Some(operation),
        }
    }

    /// A drag, in words: what was asked, where it landed, what held it, and
    /// what the pair either side is now.
    fn say_drag(&self, d: Dragged) -> String {
        let Dragged::Boundary {
            split,
            index,
            axis,
            asked,
            landed,
            held,
        } = d
        else {
            // A fader's words are the window loop's, because they are about
            // what happened to the *deck* after the operation left here.
            unreachable!("a fader drag says its own line")
        };
        let sizes = match self.panel.pair(split, index) {
            Some((a, b)) => format!(
                "{} {:.0} | {} {:.0}",
                self.label(a),
                axis.extent(self.panel.layout().rect(a)),
                self.label(b),
                axis.extent(self.panel.layout().rect(b))
            ),
            None => "no pair".to_owned(),
        };
        let stop = match held {
            Some(by) => format!(" — held {by:+.1} by a stop, and it stays there until it moves"),
            None => String::new(),
        };
        format!("  drag: asked {asked:.1}, landed {landed:.1}{stop} [{sizes}]")
    }

    fn released(&mut self) {
        match self.panel.released() {
            Some(Released::Rests { split, index, at }) => {
                println!("release: {} rests at {at:.1}", self.pair(split, index))
            }
            Some(Released::Gone { split, index }) => {
                println!("release: {} is gone", self.pair(split, index))
            }
            // **No value in the line, because there is none to print.** Where
            // a fader came to rest is the deck's, and the last thing the drag
            // asked for was printed when it was asked for.
            Some(Released::Let { knob }) => println!(
                "release: {} lets go of the {}",
                knob_where(knob),
                knob_word(knob)
            ),
            None => {}
        }
    }

    /// Act, say what happened, and hand the outcome back — **the repaint
    /// decision is taken from what the operation did, not from the key that
    /// asked for it.** `p` over an empty panel and `z` with nothing folded
    /// both reach the model and move nothing.
    fn op(&mut self, op: Op) -> Outcome {
        let outcome = self.panel.op(op);
        // **A reset puts the default arrangement on screen and the default has
        // no name**, so the pill stops naming the file it was showing. Here
        // rather than at either control, because `r` and the menu's *start a
        // new one* are one operation and this is the one place both arrive —
        // and it keys off the outcome rather than off the `Op`, so an op that
        // asked for a reset and did not get one leaves the name alone.
        if matches!(outcome, Outcome::Reset) {
            self.view.arrangement.name = None;
        }
        self.say_op(op, &outcome);
        outcome
    }

    /// What an operation did, in words. The model returns the facts; which
    /// English they take is the operation that was asked for, which is why
    /// this has both.
    fn say_op(&self, op: Op, outcome: &Outcome) {
        match outcome {
            Outcome::Folded { id, folded, root } => {
                let what = match op {
                    Op::FoldEnclosing(_) => "the split ",
                    _ => "",
                };
                println!(
                    "fold: {what}{} is now {}{}",
                    self.label(*id),
                    folding(*folded),
                    match *root && *folded {
                        true => " — that was the root, so the panel is empty; z brings it back",
                        false => "",
                    }
                );
            }
            Outcome::Unfolded(ids) => match ids.is_empty() {
                true => println!("unfold: nothing is folded"),
                false => {
                    let names: Vec<String> = ids.iter().map(|id| self.label(*id)).collect();
                    println!("unfold: {}", names.join(", "));
                }
            },
            Outcome::Soloed(id) => println!(
                "solo: {} — everything else folded (soloed = {})",
                self.label(*id),
                self.panel.layout().is_soloed()
            ),
            Outcome::Unsoloed { was } => println!(
                "unsolo: {}",
                match was {
                    true => "the arrangement before the solo is back",
                    false => "nothing was soloed",
                }
            ),
            Outcome::Reset => println!("reset: a fresh arrangement, at the same viewport"),
            // **Nothing that goes through here can produce this.** A restore
            // is not an `Op` — it carries a whole arrangement, which only
            // whoever read the store can hand over — so `arrangement` says its
            // own sentence, where the name is, and this arm exists because the
            // match is exhaustive rather than because a line is owed. See
            // `Panel::restore`.
            Outcome::Restored => {}
            // **And nothing that goes through here can produce this one
            // either, since 2026-08-31.** `Op::Report` was `p`, `p` is the
            // latency offset the operations page specifies, and a panel
            // diagnostic with no useful shortcut to point at loses the letter
            // rather than keeping one of a specified pair. Nothing else in
            // this program names the operation, so no key, no control and no
            // pointer route can reach it and there is no sentence to say.
            //
            // **The words went with the route rather than being kept for
            // one.** A formatter for an outcome nothing produces is this file
            // claiming a route it has not got
            // ([P-0063](../../../docs/principles/0063-source-cites-what-is-in-force-not-a-plan.md)),
            // and the table it printed is not the one the startup legend
            // prints: that one is each region's *min and max*, once, before
            // anything has been dragged, and this was each region's solved
            // rectangle and whether it is folded, at any moment. The
            // operation still answers that, to `karakuri-console`'s own
            // tests; what is gone is this program asking.
            Outcome::Report(_) => {}
            // One operation can still find nothing to act on, and it is not
            // about the pointer: the root has no split enclosing it. *Nothing
            // under the pointer* is said by `key`, before an operation is
            // named at all — see `Panel::under`.
            Outcome::Nothing => {
                println!("fold: that is the root, and nothing encloses it")
            }
        }
    }

    fn room(&mut self) {
        self.view.room = self.view.room.other();
        println!("room: {}", self.view.room.word());
    }

    /// **The one place a pointer event is routed**, and the only place this
    /// file decides anything about input.
    ///
    /// Returns who the event belonged to. The caller's whole job with the
    /// answer is to hand the event to `egui` when it is [`Claim::Egui`] and
    /// not when it is not — see `karakuri_console::input` for the rule and
    /// for why it is written there rather than here.
    ///
    /// It is a method rather than four arms in `window_event` so that a
    /// gesture can be driven without a window: `winit` cannot be asked for an
    /// `ActiveEventLoop` outside its own loop, so an event handler is not
    /// something a test can call, and the part worth testing is this.
    fn pointer(&mut self, ctx: &egui::Context, event: Pointer) -> (Claim, Acted) {
        let at = match event {
            Pointer::Moved(p) => p,
            _ => self.panel.cursor(),
        };
        // Asked **before** anything acts. `released` takes the drag out of
        // hand, so a claim asked after it would see no drag, route the release
        // to `egui`, and hand `egui` a button-up it never saw the button-down
        // for.
        let claim = claim(&mut self.panel, ctx, &self.view, at);
        let mut did = Acted::Nothing;
        match (event, claim) {
            // The panel learns where the pointer is either way — every
            // keyboard operation is addressed to it — and drags if something
            // is in hand. Whether `egui` is also told is the claim.
            //
            // **A move is what a fader emits on**, so this is the one arm that
            // can act without a button, and it acts whoever the claim went to:
            // rule 1 has already given the panel any drag in hand.
            //
            // **Asked before the move, and only with a fader in hand.** An
            // `Emitted(None)` is *a fader that did not change*, which is owed
            // no frame; a plain pointer move with nothing in hand is
            // `Change::Pointer`'s business and is owed one whenever the panel
            // claimed it, because that is the resize cursor going on and off.
            // Answering `Emitted(None)` for both would take the cursor with it.
            (Pointer::Moved(p), _) => {
                let fading = matches!(self.panel.in_hand(), Some(InHand::Fader));
                let operation = self.moved(p);
                if fading {
                    did = Acted::Emitted(operation);
                }
            }
            // **A press the panel claimed is on one of the console's
            // controls or on the panel itself**, and they are asked first for
            // the reason `claim` asked them last: rule 2 has already had its
            // refusal, so a press that got here and is on a control is that
            // control's. Every one of them is the same call `claim` made —
            // asked again, not copied. How many there are is
            // `karakuri_console::input::CONTROLS`, which is why this sentence
            // no longer says a number: it went stale four times.
            //
            // **The bay is derived once and asked four times**, exactly as
            // `claim` does it: a knob, a blend chip, a tally chip and a mask
            // mini are four questions about one laid-out strip, and four
            // derivations would be four answers.
            (Pointer::Down, Claim::Panel) => {
                self.panel.solve();
                // **The audio-in pill first**, and it and the arrangement pill
                // are the only two whose order matters: each
                // draws a card *over* the bays, so while one is down a press
                // inside it belongs to the card and not to whatever it is
                // covering. They are asked in the order they are drawn, which
                // is also the order they are laid out in — the arrangement
                // pill's place is measured from this one's right edge.
                //
                // **Only one card can be down**, so the two blocks cannot both
                // claim a press: `input::claim`'s rule 2 gives the press to
                // the panel while either is open, and a press outside the open
                // card is that card's dismissal.
                let listing = audio_in_pill(
                    ctx,
                    self.panel.layout(),
                    self.view.transport,
                    self.view.audio.as_ref(),
                );
                let heard = listing
                    .as_ref()
                    .zip(self.view.audio.as_ref())
                    .and_then(|(pill, audio)| pill.ask(audio, at));
                if self.view.audio.as_ref().is_some_and(AudioIn::open) {
                    did = self.listened(heard.unwrap_or(AudioAsk::Shut));
                    return (claim, did);
                }
                if let Some(ask) = heard {
                    did = self.listened(ask);
                    return (claim, did);
                }
                // **The arrangement pill next, for the same reason.** Its menu is drawn *over* the
                // bays, so while it is down a press inside the card belongs to
                // the card and not to whatever it happens to be covering — and
                // a press anywhere else is the dismissal, which is why the
                // `None` below is `Ask::Shut` rather than a press that fell
                // through. Shut, this is one capsule among many that never
                // overlap and the order is arbitrary.
                let pill = arrangement_pill(
                    ctx,
                    self.panel.layout(),
                    self.view.transport,
                    self.view.audio.as_ref(),
                    &self.view.arrangement,
                );
                let asked = pill
                    .as_ref()
                    .and_then(|pill| pill.ask(&self.view.arrangement, at));
                if self.view.arrangement.open() {
                    did = self.arranged(asked.unwrap_or(Ask::Shut));
                    return (claim, did);
                }
                if let Some(ask) = asked {
                    did = self.arranged(ask);
                    return (claim, did);
                }
                // **The two look controls, derived once for both** — the
                // exposure track's place is measured from the tone map
                // capsule's, so they are two questions about one laid-out
                // group, exactly as the mixer's four are about one strip.
                // Neither can overlap the pill: this group starts one
                // `.transport` gap after it.
                let look = look_row(
                    ctx,
                    self.panel.layout(),
                    self.view.transport,
                    self.view.audio.as_ref(),
                    &self.view.arrangement,
                    self.view.look,
                );
                let tone = look.as_ref().and_then(|row| row.tonemap(at));
                let exposure = look.as_ref().and_then(|row| row.exposure(at));
                if let Some(operation) = tone.or(exposure) {
                    return (claim, Acted::Emitted(Some(operation)));
                }
                // **The deck head's three, one pane at a time.** Each pane is
                // derived once and asked for all three, exactly as the mixer
                // bay is asked for its four: the anchor's place is measured
                // from the mode chip's and the arrows' from the anchor's, so
                // they are three questions about one laid-out pane. Nothing
                // else on the panel overlaps a pane — the Inspector's card is
                // its own bay — so the order against the mixer below is
                // arbitrary.
                //
                // **The three are asked in the order they sit in the row**,
                // and no two of them can answer for one point:
                // `DeckHead::owns` is the union of exactly these three, and
                // `tests/deck_head.rs` asserts a press is one of them or none.
                let deck_head = self
                    .view
                    .inspector
                    .iter()
                    .enumerate()
                    .find_map(|(index, pane)| {
                        let at_pane = inspector_pane(self.panel.layout(), index, pane)?;
                        let head = deck_head_row(ctx, &at_pane, pane)?;
                        head.sync(at)
                            .or_else(|| head.reanchor(at))
                            .or_else(|| head.scrub(at))
                    });
                if let Some(operation) = deck_head {
                    return (claim, Acted::Emitted(Some(operation)));
                }
                // **The Program bay head's `solo`**, and it is the one
                // control on this panel that acts on the console's own shape
                // from inside a bay rather than from the Outputs row. What it
                // asks for is `ProgramHead::op` — the same two operations
                // `s` and `u` perform, chosen from the layout rather than
                // toggled — and this file performs it exactly as it performs
                // the dot's.
                if let Some(head) = program_head(ctx, self.panel.layout(), self.view.opening)
                    .filter(|head| head.hit(at))
                {
                    return (claim, Acted::Operated(self.soloed(head.op())));
                }
                // **The four class pills**, and this is the one press in this
                // file that leaves by neither of the other two doors. See
                // `Readout::opened`, which is where the reason is.
                if let Some(pill) = Class::ALL.iter().find_map(|class| {
                    mcp_pill(ctx, self.panel.layout(), *class, self.view.opening)
                        .filter(|pill| pill.hit(at))
                }) {
                    return (claim, self.opened(&pill));
                }
                // **The Library bay's scope chips**, which are the first
                // controls on this panel whose number is a value rather than a
                // constant: one per scope the bay was handed. The bay is
                // derived once and walked once, exactly as `claim` walks it —
                // a chip is as wide as the word in it, so where the fourth one
                // is depends on the first three and a second walk would put
                // the capsule a press lands on somewhere the wash is not.
                //
                // **A press names the chip; it does not step.** `e` steps and
                // wraps because a bare press cannot say *which*, and this one
                // can — P-0074's division met by two surfaces rather than an
                // inconsistency between them. What comes back is `Chosen`: the
                // chip, and `Operation::SelectScope` beside it, because that
                // operation's payload is `Undecided` and cannot carry a chip.
                if let Some(chosen) =
                    library_bay(self.panel.layout(), &self.view.scopes, &self.view.library)
                        .and_then(|bay| bay.chip(ctx, &self.view.scopes, at))
                {
                    return (claim, self.chose(chosen));
                }
                let sink =
                    outputs(ctx, self.panel.layout(), self.view.opening).filter(|row| row.hit(at));
                let bay = mixer_bay(ctx, self.panel.layout(), &self.view.mixer);
                // **The Master bay's out is a `Grab` like a strip's**, so it
                // joins the knob rather than taking an arm of its own: what
                // this file does with either is take it in hand, and which
                // fader it was is inside the `Knob`. The two bays cannot
                // overlap, so the order is arbitrary — the mixer is asked
                // first because it has four questions to this one's one.
                let knob = bay.as_ref().and_then(|bay| bay.grab(at)).or_else(|| {
                    master_row(ctx, self.panel.layout(), self.view.master_out)
                        .and_then(|row| row.grab(at))
                });
                let chip = bay.as_ref().and_then(|bay| bay.blend(at));
                let tally = bay.as_ref().and_then(|bay| bay.tally(at));
                let mask = bay.as_ref().and_then(|bay| bay.mask(at));
                match (sink, knob, chip, tally, mask) {
                    (Some(row), ..) => did = Acted::Operated(self.sink(row.op())),
                    // **The value does not move on the press.** The grab keeps
                    // the offset it took hold at, so the first move continues
                    // from where the knob already was — and a press that was
                    // on the *track* never gets here, because `Mixer::grab`
                    // answers `None` for it rather than jumping the mix.
                    (None, Some(grab), ..) => {
                        println!(
                            "press ({:.0}, {:.0}): {} — the {} is in hand",
                            at.x,
                            at.y,
                            knob_where(grab.knob()),
                            knob_word(grab.knob())
                        );
                        self.panel.grab(at, grab);
                    }
                    // **A chip acts on the press itself**, where a fader acts
                    // on the moves after it: there is no gesture here, only
                    // one operation naming where the cycle arrived. Both go
                    // down the same path a fader's does — `Acted::Emitted`,
                    // then a record, then the deck — because P-0028 is that
                    // every control ends in the same record, and a chip that
                    // reached the deck another way would be a second route for
                    // the same change.
                    //
                    // **One arm for the three chips**, because what this file
                    // does with any of them is the same three steps; which chip
                    // it was is in the operation, and the line `apply` prints
                    // says so.
                    (None, None, Some(operation), ..)
                    | (None, None, None, Some(operation), _)
                    | (None, None, None, None, Some(operation)) => {
                        did = Acted::Emitted(Some(operation))
                    }
                    // **The strip itself, asked last.** A strip's rectangle
                    // contains all four of the questions above, so this is
                    // what is left over — a press on the name, on the number,
                    // on the ground between the rows — and it means *address
                    // the keys to this deck*. It is the only control in this
                    // bay that is not drawn as one, which is
                    // `console.html`'s *"a press anywhere on a strip that no
                    // knob under the pointer claimed"*: the whole column is
                    // the affordance, and a sixth capsule would be a control
                    // over a pointer.
                    (None, None, None, None, None) => {
                        match bay.as_ref().and_then(|bay| bay.select(at)) {
                            Some(operation) => did = Acted::Emitted(Some(operation)),
                            None => self.press(at),
                        }
                    }
                }
            }
            (Pointer::Up, Claim::Panel) => self.released(),
            (Pointer::Down | Pointer::Up | Pointer::Wheel, _) => {}
        }
        (claim, did)
    }

    /// **A press on the audio-in pill or on its card**, and what this
    /// program does about it.
    ///
    /// [`Readout::arranged`]'s shape one pill to the left, and the split is
    /// the same: the two answers that are the *control's* own state are
    /// performed here, and the one that is an operation leaves as one.
    ///
    /// **Opening the card is where the host is read**, and it is the only
    /// place: a listing of a machine's inputs is a device enumeration, which
    /// is not a thing to do on a frame path (P-0072) — the same rule under
    /// which the Library bay's names and the arrangement pill's are read on a
    /// press. So the list a hand is about to read is the list as of the press
    /// that opened it, an interface plugged in a minute ago included.
    fn listened(&mut self, ask: AudioAsk) -> Acted {
        let Some(audio) = self.view.audio.as_mut() else {
            // A press on a pill that is not drawn, which `audio_in` answers
            // `None` to and this cannot reach. Said rather than unreachable.
            return Acted::Nothing;
        };
        match ask {
            AudioAsk::Open => {
                audio.inputs = karakuri_environment::audio::inputs();
                let held = audio.inputs.len();
                println!(
                    "audio-in: `{}` — {}",
                    audio.word(),
                    match held {
                        0 => String::from(
                            "this machine has no audio inputs, and the card says so rather than                              opening empty"
                        ),
                        1 => String::from("one input to pick from"),
                        many => format!("{many} inputs to pick from"),
                    }
                );
                audio.opened();
                Acted::Nothing
            }
            AudioAsk::Shut => {
                audio.shut();
                Acted::Nothing
            }
            // **Out of this crate and into the one that can open a device.**
            // The pill names the input and `attached` opens it, which is the
            // seam ADR-0156 draws: a control asks, and whoever holds the
            // device decides.
            AudioAsk::Operation(operation) => {
                audio.shut();
                Acted::Emitted(Some(operation))
            }
        }
    }

    /// **A press on the arrangement pill or on its menu**, and what this
    /// program does about it.
    ///
    /// Five answers and this file decides none of them: which one a press asks
    /// for is `ArrangementPill::ask`'s, off the same laid-out pill `claim`
    /// hit-tested, and what arrives here is one of them by name. Two are moves
    /// of the control's own state and are this program telling the console
    /// about a press it cannot see; two are operations and go where every
    /// operation goes; the fifth is the reset, which is an `Op` and not a
    /// record, exactly as ADR-0208 has it — *"`ResetArrangement` reaches code;
    /// `RestoreArrangement` reaches a file"*.
    ///
    /// **The menu shuts on anything that acts.** An operator who has picked an
    /// item has finished with the list, and a card left standing over the
    /// console after the thing it was for has happened is the panel arguing
    /// with itself. It stays open for nothing, because nothing here can be
    /// picked twice.
    fn arranged(&mut self, ask: Ask) -> Acted {
        match ask {
            Ask::Open => {
                println!(
                    "arrangement: `{}` — save it, start a new one, or put one of {} back",
                    self.view.arrangement.word(),
                    self.view.arrangement.filed.len()
                );
                self.view.arrangement.opened();
                Acted::Nothing
            }
            Ask::Shut => {
                self.view.arrangement.shut();
                Acted::Nothing
            }
            // **The one flow on this panel that asks for letters.** Reached
            // only with no arrangement in use: with one in use, saving again
            // means that name and the pill asks for the operation instead.
            Ask::Name => {
                println!(
                    "arrangement: type a name and press return — letters, digits, `-` and \
                     `_`, and escape leaves it unsaved"
                );
                self.view.arrangement.asks_a_name();
                Acted::Nothing
            }
            // **The same operation `r` performs**, reached from the other end
            // of the panel exactly as the Outputs row's dot reaches `f`'s
            // fold. `Readout::op` is what says the arrangement in use is the
            // default again, whichever surface asked.
            Ask::Panel(op) => {
                self.view.arrangement.shut();
                Acted::Operated(self.op(op))
            }
            // **Down the path every other emitted operation takes**, which is
            // the whole reason `arrangement` sits on it: a record is written
            // by whoever holds the store, and the pill holds nothing.
            Ask::Operation(operation) => {
                self.view.arrangement.shut();
                Acted::Emitted(Some(operation))
            }
        }
    }

    /// **The name is finished**, and what that asks for.
    ///
    /// One operation of the vocabulary, named — the same
    /// `Operation::SaveArrangement` the menu's *save* asks for with an
    /// arrangement already in use, so the two ways to reach a save are two
    /// ways to name one thing rather than two paths to a disk. **The menu is
    /// shut before the operation is emitted**, whether or not the name is any
    /// good: a name that is refused is refused out loud by `checked_name`, and
    /// a card left standing over the refusal would be the panel asking the
    /// question again without saying the answer.
    ///
    /// An empty name arrives here as an empty name and is refused there, which
    /// is the rule this file keeps everywhere: the surface owns the affordance
    /// and never the authority (P-0076).
    fn named(&mut self) -> Acted {
        let Some(typed) = self.view.arrangement.naming() else {
            return Acted::Nothing;
        };
        let name = typed.to_owned();
        self.view.arrangement.shut();
        Acted::Emitted(Some(Operation::SaveArrangement { name }))
    }

    /// A press on the Program bay head's `solo`. **The pill says what it
    /// did** — which of the two operations it asked for — because the whole
    /// point of the control is that it is the same solo `s` and `u` perform,
    /// reached from a capsule instead of from the pointer.
    ///
    /// **The region is the picture's and never the pointer's**, which is the
    /// one way this differs from `s`: a key solos whatever the pointer is
    /// over, and this pill names `program-view` because
    /// `docs/manual/console.html` says what it is for — *"Solo the program
    /// view: the panel folds away and only the picture is left, which is also
    /// how you capture this window."*
    fn soloed(&mut self, op: Op) -> Outcome {
        println!(
            "program: {}",
            match op {
                Op::Solo(_) =>
                    "solo the picture — everything else folds away, and the window                      is that region",
                Op::Unsolo =>
                    "the solo comes off — what was folded before it comes back,                      including whatever was already folded",
                other => unreachable!("the solo pill asked for {other:?}"),
            }
        );
        self.op(op)
    }

    /// **A press on one of the four class pills**, and the one press in this
    /// program that is neither an operation on the arrangement nor one on the
    /// mix.
    ///
    /// # Why it takes a different path from every other press in this file
    ///
    /// Everything else here ends in one of two places. A control over the
    /// console's own shape asks for a [`Op`], `Panel` performs it, and what
    /// comes back is an [`Outcome`]. A control over the mix emits an
    /// [`Operation`], [`written`] turns it into a `Record` and [`apply`] moves
    /// the deck with it — P-0028, *every control ends in the same record*. A
    /// reader who has just met those two will reach for the second here,
    /// because it is the one every new control has taken for a year.
    ///
    /// **It must not be routed as an `Operation`, and
    /// [ADR-0236](../../docs/adr/0236-a-map-is-the-layer-between-a-surface-and-the-vocabulary-and-the-audit-is-one-of-the-things-it-does.md)
    /// is explicit about it.** The opening is configuration of the *map* — the
    /// layer every surface reaches the vocabulary through — and not a member of
    /// the vocabulary the map addresses. The rule is narrower than *map
    /// configuration is never an operation*, because `Operation::PointLane`
    /// already is one: **a setting that decides whether a surface may reach a
    /// class of operations cannot itself be one of those operations.** Rule 01
    /// would make such an operation reachable from all four surfaces, MCP
    /// included, and a permission an actor can grant itself is not a
    /// permission. There is no 65th row on the operations page for the same
    /// reason, and ADR-0235's *"the opening setting has no operation"* is
    /// annotated as settled by exactly this.
    ///
    /// So: no `Operation`, no `Record`, no [`Acted::Emitted`]. What a press
    /// hands over is a value — `McpPill::next`, the opening with one class set
    /// the other way and the other three written back as they were — and the
    /// run's `Opening` is where it goes. **Somebody will one day try to fix
    /// this into the vocabulary**; this paragraph is what it costs them to do
    /// it, and `Acted::Opened` is the type that will not let it happen quietly.
    ///
    /// **The view's copy is written in the same breath as the handle**, not
    /// left for the next frame's read. `input::claim` and the probe above both
    /// hit-test against `View::opening`, and the pill is not the same width in
    /// its two states — so a press that moved the handle and not the view would
    /// leave the very next press aimed at the capsule that was there before it.
    fn opened(&mut self, pill: &McpPill) -> Acted {
        // **Annotated, and the annotation is load bearing in two ways.** It
        // says what a press composes — an opening and not a `bool` — and it is
        // what keeps `Open` named outside `#[cfg(test)]`: a test-only `use` of
        // it would sit above the window loop, and `key_column`'s scan of this
        // file's own text stops at the first `#[cfg(test)]` line it meets.
        let next: Open = pill.next(self.view.opening);
        self.opening.set(next);
        self.view.opening = next;
        let open = next.holds(pill.class);
        // **What the pill says it did, in the words a refusal says it in.**
        // `Class::title` and `Class::opened_at` are the gate's own strings, so
        // the sentence a model is refused with and the sentence an operator
        // reads at the pill name one thing the same way (P-0061).
        println!(
            "{}: `{}` — {} is {} to a model. {}. the operator opens it at {}.",
            pill.class.bay(),
            view::mcp_word(open),
            pill.class.title(),
            match open {
                true => "open",
                false => "shut",
            },
            match open {
                true => "calls in this class are performed",
                false => "calls in this class are refused, and the refusal says so",
            },
            pill.class.opened_at()
        );
        Acted::Opened
    }

    /// **A press on a scope chip**, and it is the surface performing its own
    /// pointer — [`pointed`]'s shape one bay along, done here rather than in
    /// `performed` for the reason the `e` key's is done at the key.
    ///
    /// `Operation::SelectScope`'s payload is `Undecided`, so a performer
    /// reading the operation could not tell which library was chosen and would
    /// have to guess. The press *knows*, because a pointer lands on one
    /// capsule and no other, and [`Chosen`] is what carries the two halves
    /// together. **So the mark is moved here and the operation is emitted for
    /// the record it is owed**, which is `Silent(Surface)` — the same shape as
    /// the key, which steps first and emits afterwards.
    ///
    /// **A chip that is already marked is not refused**, and the line says
    /// which of the two it was. `View::select_scope` answers `false` for it,
    /// and that is a mark that did not move rather than a press that failed:
    /// where the key *steps* and would go somewhere else, a press **names**,
    /// and naming the library you are already reading is asking it again. What
    /// the caller does with that is re-read the listing, which is where a
    /// directory read belongs (P-0072) and is not on this side of the seam.
    ///
    /// It cannot refuse for the other reason either: the chip came out of
    /// `View::scopes`, so it is on the row by construction.
    fn chose(&mut self, chosen: Chosen) -> Acted {
        let moved = self.view.select_scope(chosen.scope);
        println!(
            "scope: `{}` — {}",
            chosen.scope.name(),
            match moved {
                true => "the library this bay reads, and the cursor is back at the top of it",
                false => "already the library this bay reads, so this asks that one again",
            }
        );
        Acted::Emitted(Some(chosen.operation))
    }

    /// A press on the Outputs row's one control. **The dot says what it did**
    /// — which of the two operations it asked for, and what the picture is
    /// now — because the whole point of the control is that it is the same
    /// fold `f` over the picture performs, reached from the other end of the
    /// panel.
    fn sink(&mut self, op: Op) -> Outcome {
        // Two operations and no third, which is `Outputs::op`'s whole
        // argument: the toggle is the dot choosing between them, and what
        // arrives here is one of the two by name.
        println!(
            "outputs: program view — {}",
            match op {
                Op::Fold(_) =>
                    "the sink was on, so the picture folds away and the \
                                inspector takes its height",
                Op::Unfold(_) =>
                    "the sink was off, so the picture comes back — with \
                                  whatever was folded over it",
                other => unreachable!("the dot asked for {other:?}"),
            }
        );
        self.op(op)
    }

    /// **What the pointer is over, resolved to a target for a key press.**
    ///
    /// The half of an operation that used to be inside it: `f` means *fold*
    /// and the pointer is how this surface says *which*. A key that lands on
    /// a divider or outside every region names nothing, so nothing is emitted
    /// — and the two sentences that used to be [`Outcome`]s are said here,
    /// where the resolution failed, rather than by a model that was asked to
    /// fold something nobody had named.
    fn target(&mut self, key: &str) -> Option<NodeId> {
        match self.panel.under() {
            Hit::View(id) => Some(id),
            Hit::Divider { split, index } => {
                println!(
                    "{key}: the pointer is on divider {}#{index} — move it into a region",
                    self.label(split)
                );
                None
            }
            Hit::Nothing => {
                println!("{key}: nothing under the pointer");
                None
            }
        }
    }

    /// `g`'s target, which is the one resolution with two answers: **a
    /// divider already names its split**, so over a gap the split to fold is
    /// that one and the operation is a plain [`Op::Fold`] of it, while over a
    /// region it is [`Op::FoldEnclosing`] and the model reads the parent.
    ///
    /// Both arms were inside `Op::FoldEnclosing` when an operation meant
    /// *whatever is under the pointer*; they are the same two arms, out where
    /// the pointer is.
    fn enclosing(&mut self) -> Option<Op> {
        match self.panel.under() {
            Hit::View(id) => Some(Op::FoldEnclosing(id)),
            Hit::Divider { split, .. } => Some(Op::Fold(split)),
            Hit::Nothing => {
                println!("g: nothing under the pointer");
                None
            }
        }
    }

    // -- the legend -----------------------------------------------------

    /// **What this program is, said once at startup — and every line of it
    /// derived.**
    ///
    /// `presets` and `store` are the two directories this run resolved, and
    /// they are passed in rather than read here for the reason every other
    /// number in this function is asked of the thing it is about: a legend
    /// that described the search instead of printing its answer is exactly the
    /// defect this function was repaired of, one paragraph along. See the
    /// paragraph below for what that repair cost to find.
    fn print_legend(
        &mut self,
        budget_ms: Option<f32>,
        governed: &Report,
        presets: Option<&karakuri_environment::places::Presets>,
        store: &std::path::Path,
    ) {
        self.panel.solve();
        let layout = self.panel.layout();
        let viewport = layout.viewport();
        println!();
        println!(
            "the console, in a {:.0} x {:.0} viewport. every leaf gets its region, and what \
             each one draws is the list further down rather than a sentence here: that list \
             is the arrangement's own nodes read against the view the frame is drawn from, \
             so a body that fills in says so without anybody rewriting a line of this. **the \
             sentence this replaces said every body was empty but the Program bay's two**, \
             and it went on saying it while bay after bay drew one — which is what a \
             description kept beside the thing it describes is worth.",
            viewport.w, viewport.h
        );
        // **Where this run's data is, and both lines are what the resolution
        // returned.** Not a sentence about how a presets root is looked for:
        // the directory is printed, and the phrase beside it is
        // `places::Found`'s own — so a candidate added, reordered or removed
        // changes this line without anybody editing it. A legend that said
        // *"the ones that ship with the program"* would be right until the day
        // it was not, which is the whole of what the paragraph above is about.
        match presets {
            Some(presets) => {
                // **Counted once and read off the listing the bay is drawn
                // from**, rather than described: a sentence about what a
                // preset directory probably holds is the kind of line this
                // legend was found lying five ways with.
                let held = presets_listing(Some(presets)).len();
                println!(
                    "presets: {} — {}. that directory is the app-preset tier: what ships \
                 with the program, written by nobody, and what a run with no paths on \
                 the command line opens on. the Library bay's `presets` scope lists the \
                 {} `.kset` file{} in it — the parts beside them are what those files \
                 name rather than rows of their own — and `l` on one takes it into the \
                 store and then loads it, which is why opening a preset leaves a row \
                 under `my sets`.",
                    presets.dir.display(),
                    presets.found.how(),
                    held,
                    match held {
                        1 => "",
                        _ => "s",
                    }
                )
            }
            // Said once, out loud, and it is this program's only occasion to
            // say it: a run that needed the library for a default pair was
            // refused before a window opened, so reaching here means the pair
            // was given by hand and nothing is broken — the preset tier is
            // simply empty.
            None => println!("{}", karakuri_environment::places::no_preset_library()),
        }
        println!(
            "store: {} — where the Library bay below reads Sets from, where this panel's \
             arrangements are filed, where each deck's working copy was written before this \
             window opened, and what `--store` moves. `karakuri-cli --store` names the same \
             directory and the default is the same constant, which it now is rather than \
             looks like: both ask `karakuri_environment::places::STORE`.",
            store.display()
        );
        // **The cells, and this sentence has been wrong four times.** It said
        // C and D had nothing behind them and named the number — *a deck of TWO
        // slots* — which was true of the deck that shipped before this one.
        // Then it said the one sink was an audition, and it was not: nothing
        // called `Deck::set_preview`, so the sink was a second copy of the
        // picture and the word was a claim about a control that did not exist.
        // Then it said the cells were a control an operator pressed to move the
        // audition, and ADR-0240 retired that control. Then it said three cells
        // read `off` because an off-air slot "has no new frame to show", which
        // was true only because the engine refused to draw one — the gap
        // P-0080 named, and this pass closed it. **Every sentence here is read
        // off the deck**, which is the only way this legend stops being
        // rewritten each time: the numbers are counted, not written.
        let live: Vec<usize> = (0..DECKS)
            .filter(|slot| {
                self.view
                    .mixer
                    .get(*slot)
                    .is_some_and(|strip| strip.tally == view::Tally::Live)
            })
            .collect();
        // A cell has a slot behind it or it has nothing; this deck is full, so
        // it is every cell. `Deck::slot_view` is `None` past `slot_count` and a
        // strip is a slot, so the mixer's length is the same count from the
        // other end.
        let behind = self.view.mixer.len().min(DECKS);
        println!(
            "{behind} of {DECKS} cells have a deck slot behind them and every one of them \
             is ON, whatever that slot's residency — a cell draws its own slot's material \
             through the same transfer curve the picture goes through, with no fader on it, \
             because a fader is applied in the mix and a cell is upstream of the mix. that \
             is what an operator watches to decide whether material is worth putting on \
             air, so it cannot wait until it is on air (P-0080). each cell is presented \
             from `Deck::slot_view` for the slot it is lettered for, so no cell can be \
             showing another deck under the wrong letter, and the picture above them is \
             the mix, always — there is no control that swaps it for one deck, and the \
             cells are why there does not need to be (ADR-0240). {} — that is about the \
             MIX and not about the cells: a slot that is not LIVE is drawn into its own \
             target and skipped by the composite, so it is watchable and inaudible. a \
             slot that is not stepping shows the still it stopped at and one that has \
             never stepped shows black, which is priming's whole use. every cell costs a \
             present pass of its own and three of the four are not in the compute budget \
             — the bill P-0080 says is paid rather than argued.",
            match live.as_slice() {
                [] => "nothing on this deck is LIVE, so the mix is empty".to_owned(),
                [one] => format!(
                    "deck {} is the only slot that is LIVE and reaches the mix",
                    deck_letter(*one as u8)
                ),
                many => format!(
                    "{} slots are LIVE and reach the mix — {}",
                    many.len(),
                    many.iter()
                        .map(|slot| format!("deck {}", deck_letter(*slot as u8)))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            },
        );
        println!(
            "the transport row reads the session's own oscillator — the tempo, the beat \
             inside the bar, and the bar counted from one — beside what the last frame \
             cost{}. the four dots are `karakuri_signal`'s BEATS_PER_BAR, which that \
             crate calls a provisional assumption of common time, so the console takes \
             the number a frame rather than assuming four.",
            match budget_ms {
                Some(budget) => format!(
                    ", against this display's refresh interval of {budget:.1} ms — which is \
                     the budget a frame is held to on a Fifo surface, and not the 20 ms a \
                     candidate Set is held to"
                ),
                // Said rather than passed over: a reading nobody can take is
                // worth a sentence, because the alternative is a reader
                // wondering where the mock's `/16.6` went.
                None => String::from(
                    " — with no budget beside it, because winit will not say what this \
                     display's refresh rate is"
                ),
            }
        );
        println!(
            "five things the mock draws in that row are NOT drawn, and each is a control \
             over machinery that is in neither this crate nor this program: tap, learn, map, \
             landed and rec. `view::transport` names them one by one with what is missing \
             behind each. `audio-in` was the sixth until this program opened an input: it is \
             drawn now, it says which room is being heard, and its card lists the others."
        );
        // **What the pill is actually reading, off the view rather than off a
        // sentence.** The legend was found lying five ways on 2026-08-30 by
        // saying what this program probably does; this says what it did.
        println!(
            "{}",
            match self
                .view
                .audio
                .as_ref()
                .and_then(|audio| audio.device.as_deref())
            {
                Some(device) => format!(
                    "the `audio-in` pill reads `{device}` and is drawn armed. energy, onset and \
                     band0..7 on the session's bus are measured from that input every frame, and \
                     the beat lock corrects the oscillator the transport row above draws. `b` \
                     taps, `,` and `.` move the grid an octave, and a press on the pill lists \
                     what else this machine has."
                ),
                None => String::from(
                    "the `audio-in` pill reads `none`, which is a state and not a fault: no \
                     input is open, every signal name answers what it answered before audio \
                     existed, and the grid free-runs at the session tempo. a press on the pill \
                     lists what this machine has, and `b`, `,` and `.` say so rather than \
                     doing nothing."
                ),
            }
        );
        println!(
            "the mixer draws {} strip{}, because a strip is a deck SLOT and this deck has \
             {} — the mock's four is the most a deck can hold, and the page keeps its four \
             tracks either way, so a track with nothing behind it is empty rather than a \
             strip full of dashes. the two FADERS are played: drag the knob on the trim or \
             on the tall fader and the panel emits SetGain or SetOpacity, which \
             karakuri-operation-record turns into a Record — the one place an \
             operation becomes one, for every surface — and this file applies to the \
             deck. the strip then follows because the DECK changed, not because \
             anything here remembered. a press on the track \
             off the knob does nothing, deliberately: a fader at 0.3 whose top is clicked \
             must not jump to 1.0 on stage. the chips beside the faders are played too, and \
             this file has stopped counting them: what a pointer reaches on this panel is \
             said once, below, and asked of the crate that hit-tests it.",
            self.view.mixer.len(),
            match self.view.mixer.len() {
                1 => "",
                _ => "s",
            },
            self.view.mixer.len()
        );
        // **The park, said in this file's voice and then in the engine's.**
        // The sentence is this program's, because the words on this window are;
        // the numbers under it are `Report`'s own `Display` and the same three
        // fields `karakuri-cli`'s `report_governing` prints, so an operator
        // reading a park here and a park there is reading one thing.
        println!(
            "deck {parked}'s strip is the one that MOVES: its chip reads ALLOC and rolls \
             part of the way toward PRIM once a second and falls back, never landing, \
             because what the slot was asked for and what it is doing disagree. this \
             program asks for {parked} to be primed at startup and the budget has no room, \
             so `Deck::govern` holds it at allocated with the request intact — that is a \
             PARK, which is `not now` and not `no`: nothing has to be asked twice, and the \
             next pass over a deck with room admits it. nothing in this file writes an \
             effective residency or draws a park; the strip carries both of the deck's own \
             words for that slot and the view derives the rest.",
            parked = deck_letter(ASKED_TO_PRIME as u8)
        );
        // **The slots nobody asked anything of, counted off the report rather
        // than named here.** `Reason::OffAir` is the governor's own word for
        // *allocated, and that is what was asked for*: it was not asked about
        // these slots and did nothing to them. A list written out in this file
        // would be this paragraph going on saying `C and D` the day the deck
        // opens differently.
        let resting: Vec<&str> = governed
            .decisions
            .iter()
            .filter(|decision| decision.reason == Reason::OffAir)
            .map(|decision| deck_letter(decision.slot as u8))
            .collect();
        if !resting.is_empty() {
            println!(
                "the other {} — {} — {} allocated and {} asked for nothing: the governor \
                 reports `OffAir`, which is a slot at REST rather than a slot refused. each \
                 holds this program's one pair at its own seed salt, because a slot cannot \
                 hold nothing and this program has no second pair to give one. it steps \
                 nothing and reaches the mix not at all — but it IS drawn, into its own \
                 target, every frame, which is what puts it in its cell and what P-0080 \
                 asks for; the frame numbers below include those draws. a slot that has \
                 never stepped has no element state, so a cell for a slot that has been \
                 at rest since this window opened is black until something warms it — \
                 which is what priming is for. an operator brings one up by cycling its \
                 tally or loading a Set onto it.",
                match resting.len() {
                    1 => "slot",
                    _ => "slots",
                },
                resting.join(" and "),
                match resting.len() {
                    1 => "is",
                    _ => "are",
                },
                match resting.len() {
                    1 => "was",
                    _ => "were",
                },
            );
        }
        println!("what the governor decided, in its own words:");
        println!("  {governed}");
        for decision in governed.parked() {
            println!(
                "  slot {} (deck {}) parked, request held: {:?} — {}, against {}, and one \
                 step in {SLOWEST_PRIME_ONE_IN} is the slowest rate the governor will call \
                 priming",
                decision.slot,
                deck_letter(decision.slot as u8),
                decision.reason,
                match decision.cost_ms {
                    Some(ms) => format!("warming it was measured at {ms:.3} ms"),
                    None => String::from("nothing has measured it"),
                },
                match governed.headroom_ms() {
                    Some(ms) => format!("{ms:.3} ms of headroom"),
                    None => String::from("a committed cost nobody can know"),
                },
            );
        }
        println!(
            "the transition row under the strips is NOT drawn — wipe, iris, next bar, \
             8 beats and go are a transition being armed and fired, and nothing here holds \
             what is armed. the crossfader that used to sit over it is not undrawn but \
             gone: the mixer has no crossfader. nor are the two focuses drawn: the deck \
             selection and keyboard focus must not look alike, and this console keeps \
             neither."
        );
        println!();
        for node in self.panel.nodes() {
            let (min, max) = layout.bounds(node.id);
            let bounds = format!(
                "min {min:.0}, max {}",
                match max.is_finite() {
                    true => format!("{max:.0}"),
                    false => "none".to_owned(),
                }
            );
            let what = match layout
                .name(node.id)
                .and_then(karakuri_console::view::region)
            {
                Some(region) => match region.kind {
                    // **The pills are named because they are controls.** A
                    // bay head's pills are exactly its controls — the table in
                    // `view::REGIONS` says so, and the Program bay's `solo` is
                    // the only one any bay has — so a head with one is a place
                    // a press reaches and a legend that said `bay` would be
                    // hiding it. Read off the table rather than written here.
                    Kind::Bay { pills, grip, .. } => {
                        let mut what = String::from("bay");
                        if grip {
                            what.push_str(", with a grip");
                        }
                        for pill in pills {
                            what.push_str(&format!(", `{pill}` in its head"));
                        }
                        what
                    }
                    // Four readouts, and then the controls that landed in
                    // this row after them: the audio-in pill, the arrangement
                    // pill and, at the far end, the tone map and the exposure.
                    // The five the mock draws here and this panel does not are
                    // still five things that do not exist behind it, and
                    // `view::transport` names each of them. What a press in
                    // this row reaches is not counted here — see the pointer's
                    // paragraph below.
                    Kind::Transport => "row, no heading: bpm, beat, bar, frame".to_owned(),
                    // The console's first control, and for a while its only
                    // one. How many there are now is
                    // `karakuri_console::input::CONTROLS` and is printed
                    // below; a number kept here would be that claim in a
                    // second place, which is the defect this legend is being
                    // repaired of.
                    Kind::Outputs => "row, one sink: program view".to_owned(),
                    Kind::Pane => "pane, inside a bay".to_owned(),
                    Kind::Picture => "the picture, a sink".to_owned(),
                    // Four cells, and how many are on is counted rather than
                    // written: a cell is on because there is a deck slot behind
                    // it, and a strip is a deck slot. Residency does not come
                    // into it — every slot is drawn — which is the correction
                    // this line carries. A constant here is exactly the legend
                    // naming a control's state from before the control existed,
                    // which this row has been twice already.
                    Kind::Previews => {
                        let on = self.view.mixer.len().min(DECKS);
                        format!("{DECKS} previews, {on} of them monitoring a deck slot")
                    }
                    // A bay like the other five: a row of chips saying which
                    // library is being read, and then a row per Set that one
                    // holds — as many as the bay has room for, and the foot
                    // says so. `n of m`, exactly as the bay draws it, and the
                    // scope is the view's own mark rather than a word written
                    // here.
                    Kind::Library => {
                        match karakuri_console::view::library(
                            layout,
                            &self.view.scopes,
                            &self.view.library,
                        ) {
                            Some(bay) => format!(
                                "bay, {}, {} listed",
                                match self.view.scope() {
                                    Some(scope) => scope.name(),
                                    None => "no scopes",
                                },
                                bay.count()
                            ),
                            None => "bay, nothing said about any library".to_owned(),
                        }
                    }
                    // A bay like the other six, and then a strip per slot:
                    // a strip is a deck slot, and this deck is built full, so
                    // this reads four.
                    Kind::Mixer => format!(
                        "bay, {} strip{}",
                        self.view.mixer.len(),
                        match self.view.mixer.len() {
                            1 => "",
                            _ => "s",
                        }
                    ),
                    // A bay with one row in it, and the row is one control:
                    // the level the composited frame leaves the mix at. The
                    // three effects the mock draws under it exist nowhere, so
                    // there is nothing else in the bay to report.
                    Kind::Master => match self.view.master_out {
                        Some(out) => format!("bay, out {out:.2}"),
                        None => "bay, no engine behind it".to_owned(),
                    },
                    // A bay like the other five, and then a row per deck slot
                    // with a verdict outstanding. **None at startup, which is
                    // where this prints**: nothing has been rebuilt yet, and
                    // empty is this lane's ordinary state — see
                    // `view::staging`.
                    Kind::Staging => match self.view.staging.len() {
                        0 => "bay, nothing waiting".to_owned(),
                        waiting => format!("bay, {waiting} waiting"),
                    },
                },
                None => match layout.axis(node.id) {
                    Some(Axis::Row) => "split, left to right".to_owned(),
                    Some(Axis::Column) => "split, top to bottom".to_owned(),
                    None => "not drawn".to_owned(),
                },
            };
            // **And the class pill where the region draws one**, appended
            // rather than written into each arm: four regions carry one, they
            // are four different `Kind`s, and an arm apiece would be four
            // copies of one sentence — which is the defect this legend was
            // repaired of the last time. What it says is the gate's own words,
            // so the sentence an operator reads here and the sentence a model
            // is refused with cannot drift apart.
            let what = match layout.name(node.id).and_then(class_at) {
                Some(class) => format!(
                    "{what}, `{}` at {} — {}",
                    view::mcp_word(self.view.opening.holds(class)),
                    class.opened_at(),
                    class.title()
                ),
                None => what,
            };
            println!(
                "  {:width$}{:<16} {:<22} {}",
                "",
                self.label(node.id),
                what,
                bounds,
                width = node.depth * 2
            );
        }
        println!();
        println!(
            "the outputs row's dot folds the picture by name, so clicking it and pressing f \n\
             over the picture are the same operation reached from two surfaces. it is lit \n\
             while the picture is on screen. it is one of two kinds of control and a fader \n\
             is the other: the dot names an operation on the ARRANGEMENT, which this crate \n\
             performs, and a fader names one on the MIX, which it cannot — so the operation \n\
             comes out and this file applies it."
        );
        println!();
        println!(
            "four of the regions above carry an `mcp` pill, and each says beside its own line \n\
             which state its class is in. each opens one class of operations to a model; every \n\
             operation stays connected either way, and what shut changes is that the call is \n\
             answered with a refusal instead of being performed — one that names the class and \n\
             says which pill opens it. it is the one control on this panel that is neither an \n\
             operation on the ARRANGEMENT nor one on the MIX: it is a setting of the map every \n\
             surface reaches the vocabulary through, and a setting deciding whether a surface \n\
             may reach a class of operations cannot be a member of that class (ADR-0236). \n\
             nothing in this process serves MCP yet, so what these four write is read here, by \n\
             this file's tests, and by nothing else."
        );
        println!();
        println!(
            "the pointer, and this file no longer keeps a list of what it reaches. a press \n\
             in a gap takes the boundary and it follows the pointer. everywhere else \n\
             `karakuri_console::input::claim` decides, and the {CONTROLS} controls its rule \n\
             4 hit-tests are painted shapes with no widget behind them — nothing but that \n\
             rule knows a press landed on one. the number is that crate's own constant, \n\
             summed over the derivations the rule actually asks, so a control added there \n\
             and not counted is a compile error rather than a sentence that has gone quiet. \n\
             what each of them is, and what a press on it asks for, is written at the \n\
             derivation that draws it: a copy here is precisely how this legend came to \n\
             name three controls while every one of them answered a press."
        );
        println!();
        println!("keys — the pointer's position decides what each one acts on:");
        for (key, what) in KEYS {
            println!("  {key:<10}{what}");
        }
        println!();
    }
}

/// **Every key this window binds, and the sentence the legend prints for it.**
///
/// The list an operator reads and the list the tests check are one list.
/// `key_column::the_keys_this_file_lists_are_the_keys_the_window_loop_binds`
/// reads the `match` in `window_event` out of this file's own text and asserts
/// it is exactly these keys, so a key bound and not printed — or a key printed
/// and not bound — fails there rather than being found by an operator pressing
/// it and getting nothing.
///
/// **That test was already here and the legend was a second copy of its
/// list**, which is the copy that drifted: the printed list stayed at nine
/// keys while ten more were bound, and the operator who read it was told this
/// program folds, solos, resets and quits.
///
/// The rows of `docs/manual/operations.html` each key reaches are
/// `key_column::ROWS`, which is keyed off this table and stays in the test
/// module: a page heading is what a check reads, and it is not something this
/// program says to anybody.
///
/// The order is the order they print in — the arrangement's keys, then the
/// deck and the library, then the five that address the room, then `esc`, then
/// the three that are live only while the arrangement pill is asking for a
/// name.
///
/// **`p` is the latency offset here and was the report until 2026-08-31.**
/// The page specifies the offset as `o` and `p`; a badge naming two keys with
/// one of them bound would be a badge that lies, and a panel diagnostic with
/// no useful shortcut to point at loses the letter rather than keeping it. The
/// operation it named is still `karakuri_console::panel::Op::Report` and
/// nothing in this program asks for it.
const KEYS: &[(&str, &str)] = &[
    ("f", "fold the region under the pointer"),
    ("g", "fold the split enclosing the region under the pointer"),
    (
        "z",
        "unfold everything folded — the pointer cannot reach one to unfold it",
    ),
    ("s", "solo the region under the pointer"),
    ("u", "undo the solo"),
    ("r", "reset to a fresh arrangement"),
    ("n", "the room: day or night"),
    (
        "0",
        "select deck A, which is the deck a load is addressed to",
    ),
    ("1", "select deck B"),
    ("2", "select deck C"),
    (
        "3",
        "select deck D — bound whatever the deck has, and this deck has a slot for it",
    ),
    (
        "e",
        "the library's scope: the next chip along, and it wraps",
    ),
    (
        "b",
        "tap the beat — three taps set the tempo, any tap sets the phase",
    ),
    (
        ",",
        "halve the grid, and the tracker's octave window with it",
    ),
    (
        ".",
        "double it — refused where the result leaves 60..200 BPM",
    ),
    (
        "o",
        "the latency offset, five milliseconds down — negative, and the picture waits for the \
         music",
    ),
    (
        "p",
        "and five up — positive, and the picture leads it; held inside 200 ms either way",
    ),
    ("up", "the library cursor, up a row"),
    ("down", "and down, as far as the rows the bay drew"),
    ("l", "load the Set under that cursor onto the selected deck"),
    (
        "k",
        "keep what the selected deck is playing — a Set filed under the time you saved it",
    ),
    (
        "esc",
        "quit — or, while a name is being typed, abandon the name",
    ),
    (
        "return",
        "take the name the arrangement pill is asking for, while it asks",
    ),
    ("backspace", "rub out a letter of that name"),
    (
        "space",
        "a space in it — text while the pill is open, and not a key",
    ),
];

/// A pointer event, stripped to what the rule needs. A button is left or it
/// is not routed at all, and which wheel axis it was does not change who gets
/// it.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Pointer {
    Moved(Point),
    Down,
    Up,
    Wheel,
}

/// **What routing a pointer event did**, beyond deciding whose it was.
///
/// Three answers rather than an `Option<Outcome>`, because there are now two
/// kinds of control on the panel and they end in two different places: the
/// Outputs dot asks for an operation on the *arrangement*, which this crate
/// performs and reports as an [`Outcome`], and a fader asks for an operation
/// on the *mix*, which nothing in `karakuri-console` can perform at all. The
/// repaint decision is taken from which of the three it is — see
/// `Change::Operated` and `Change::Emitted`.
#[derive(Debug, Clone, PartialEq)]
enum Acted {
    /// Nothing acted: a press on a boundary, a move, a wheel, a release.
    Nothing,
    /// The Outputs dot, and what the operation it named did.
    Operated(Outcome),
    /// **A fader translated a drag into the vocabulary**, or the drag moved
    /// the pointer over a value that did not change and asked for nothing.
    Emitted(Option<Operation>),
    /// **A class pill was pressed**, and what it wrote went to the run's
    /// `Opening` rather than to the arrangement or to the deck.
    ///
    /// **A fourth answer rather than a reuse of `Nothing`**, and the difference
    /// is the whole of ADR-0236: this press is not an operation and must never
    /// be made into one, so it cannot be an `Emitted`; and it is not nothing
    /// either, because a word on the panel changed. See `Readout::opened`.
    ///
    /// It carries no payload because there is none to carry: what changed is
    /// held in the `Opening`, which is a handle another surface reads, and a
    /// copy of it in this enum would be the second answer to *what is open*.
    Opened,
}

fn folding(folded: bool) -> &'static str {
    match folded {
        true => "folded",
        false => "unfolded",
    }
}

/// **Which deck, in the letter the preview cells are drawn with** — asked of
/// the view rather than written out again here.
///
/// The vocabulary counts decks from zero (`Operation::SetGain { deck: u8 }`)
/// and the console draws them `A` through `D`, so this is the one place the
/// two meet. A deck outside the four cannot be built by anything in this file
/// — a `Deck` holds `MAX_SLOTS` slots and `DECKS` is that number — so an index
/// past the end is a bug and reads as one rather than wrapping quietly.
fn deck_letter(deck: u8) -> &'static str {
    DECK_LETTERS
        .get(usize::from(deck))
        .copied()
        .unwrap_or("(no such deck)")
}

/// Which fader, in the bay's own word for it.
fn knob_word(knob: Knob) -> &'static str {
    match knob {
        Knob::Trim { .. } => "trim",
        Knob::Fader { .. } => "fader",
        Knob::Out => "out",
    }
}

/// **Whose fader it is**, for a line a reader has to place: a deck by its
/// letter, and the master out by the bay it is in.
///
/// The master out names no deck — it is one level on the whole fold
/// ([ADR-0224](../../docs/adr/0224-out-and-exposure-are-two-levels-that-multiply-in-different-places.md))
/// — so a line that said *deck A* over it would be naming a slot nothing in
/// the gesture ever touched. `Knob::deck` is what answers, and this is the
/// only caller: everything that *acts* takes the deck out of the operation.
fn knob_where(knob: Knob) -> String {
    match knob.deck() {
        Some(deck) => format!("deck {}", deck_letter(deck)),
        None => "master".to_owned(),
    }
}

// ---------------------------------------------------------------------------
// The engine in the Program bay
// ---------------------------------------------------------------------------

/// The canvas: what the Set renders at, and what the deck is sized to.
///
/// **The workspace's reference workload**, 262144 elements at 1280x720
/// (`docs/contributing.md` §1), and it is that on purpose rather than by
/// default: this is the one place in the console where a number is about the
/// engine rather than about the panel, and a number taken at the reference
/// workload can be put beside every other one in this repository. **Not the
/// size of the picture, and not the size of a preview cell either** — see
/// [`Present::draw`], which letterboxes this into whatever it is drawn into
/// and is handed two rectangles of different sizes a frame, and which is what
/// the manual means by *"it letterboxes into the width it has"*.
///
/// **It is also the shape the picture is now given**, through
/// [`aims`] and `picture_rect`: the console sizes the picture's rectangle to
/// this rather than to whatever the region happens to be, so the two targets
/// `Present::draw` is handed are both the canvas's shape and its fit into each
/// is sub-texel. The number reaches `picture_rect` from `Present::size` rather
/// than from this constant, so the shape the console draws and the canvas the
/// engine renders cannot come from two places — see [`aims`].
const CANVAS: (u32, u32) = (1280, 720);

/// **Deck A's seed salt**, which decides where its elements start. Any value
/// is a picture; 7 is the one `karakuri-cli`'s own tests use, so this looks
/// like what they look like.
///
/// **Deck A's, and the rest of the deck is counted off it** — see
/// [`slot_salt`], which is the one place this file turns a slot into a seed.
const SEED_SALT: u32 = 7;

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
/// ([P-0033](../../../docs/principles/0033-the-governor-never-takes-a-live-slot-off-air.md)),
/// so what happens is `Report::over_budget` and priming suspended — a warning
/// on the legend's governor line and a decision left with the person who made
/// it. That is the governor doing its job rather than this file second-guessing
/// it with a slot count.
const SLOTS: usize = MAX_SLOTS;

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
const ON_AIR: usize = 0;
const ASKED_TO_PRIME: usize = 1;

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
/// ([P-0031](../../../docs/principles/0031-a-name-means-one-thing-across-the-system.md)).
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
/// P-0031 is about one name meaning one thing, and the failure it names would
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
#[derive(Debug, Clone)]
struct Sources {
    l1: std::path::PathBuf,
    l4: std::path::PathBuf,
}

impl Sources {
    /// **The pair a run with no paths opens on, under whichever preset library
    /// answered.**
    ///
    /// This was `Default`, and what it resolved against was
    /// `env!("CARGO_MANIFEST_DIR")` — the build machine's own tree, baked in at
    /// compile time. That was the only production line in the workspace doing
    /// it, and it meant a binary installed anywhere else found no presets at
    /// all: not a wrong pair, no pair, and the Library's `presets` tier a tier
    /// with no file in it. A POSIX process cannot ask where it is, so the
    /// answer is to be *told* — `--presets` — or to go looking from
    /// [`std::env::current_exe`], which is
    /// [`karakuri_environment::places::presets`] and not this file's business.
    ///
    /// What is this file's business is the two names, because they are this
    /// program's choice of what to open on rather than a property of a preset
    /// library: a library is a directory with at least one `.kset` in it
    /// (`karakuri_environment::places`'s `is_a_library`, which asked for a
    /// `.kir` until the authoring form landed), and these two are
    /// what the reference workload is measured on — see
    /// `the_capacity_is_the_l1s_own_declaration_and_the_l4_declares_none`,
    /// which pins the L1's capacity because `docs/contributing.md` §1 quotes
    /// figures taken against it.
    ///
    /// **Under the root rather than under the working directory**, and the
    /// asymmetry with a typed path is the one the old `Default` had for the
    /// same reason: a pair nobody named has to be found wherever the run was
    /// started from, and a path an operator typed is theirs and is read from
    /// where they typed it.
    fn under(presets: &std::path::Path) -> Sources {
        Sources {
            l1: presets.join("drift_shell.kir"),
            l4: presets.join("soft_points.kir"),
        }
    }

    /// **What the mixer strip calls what this deck is playing**, and it is this
    /// file's word rather than the engine's.
    ///
    /// `view::Strip::name` says why there is no other answer: nothing reachable
    /// from a `Deck` carries a name for the material in a slot. A `Set` names
    /// its *nodes* and its *published controls* and has no name of its own,
    /// which is right — a Set is built from a list of `.kir` files, and only
    /// whoever passed that list knows what to call the result. **This is that
    /// list**, derived from the two paths rather than typed again.
    ///
    /// **It is what every slot *opens* on and not what one is playing**, which
    /// is [`Gfx::material`]'s distinction: this program builds every slot from
    /// the one pair, and a load moves one of them to a Set the library names.
    /// So this answers once, at startup, and the per-slot name is kept and
    /// rewritten there.
    fn material(&self) -> String {
        let stem = |path: &std::path::Path| {
            path.file_stem()
                .map_or_else(String::new, |stem| stem.to_string_lossy().into_owned())
        };
        format!("{} + {}", stem(&self.l1), stem(&self.l4))
    }
}

/// **The working copy each deck runs from**, one pair per slot, made before the
/// window opens.
///
/// # Why a panel copies at all
///
/// `karakuri_environment::scratch` names three places and says only one of them
/// is written to, because before it existed a surface was given write access to
/// whatever path the material came from and **three shipped presets were
/// replaced in one session**. This program had none of it: every slot watched
/// the two paths the operator typed, so the file an editor opened was the
/// preset itself, and a run started with no paths at all watched `examples/`.
///
/// # And the panel is not `--render`
///
/// `scratch.rs` gates copying on *a run that can be edited*, because creating a
/// directory as a side effect of a pure render *"would make a function of its
/// arguments into one that leaves a mark"*. The gate is a question about this
/// program rather than about a flag, and the answer is that **this program is a
/// `--watch` run that cannot be turned off**: every slot is built over a
/// `watch::Watch` ([`watched`]), the deck's whole way of changing material is an
/// edit picked up by a poll, and `l` over the Library bay writes into this same
/// directory on a key press ([`loading`]). There is no run of this binary that
/// opens a file read-only, so the condition is not *checked* here — it is
/// **true**, and the copies are made once, before the first frame, rather than
/// on the first press.
///
/// What that costs is the store directory existing on a run that saves nothing,
/// which is the cost `karakuri-cli --watch` already pays and the manual already
/// states: *"a run with `--watch` or `--mcp` opens the store at startup either
/// way — the scratch lives in it."* It is not the cost [`arrangements`] and
/// [`library`] decline — those two decline to **create** a store in order to
/// *list* it, and a listing that made a directory would be a read with a side
/// effect. This is a run that has already decided to write.
///
/// # One preset in four slots is four files
///
/// [`SLOTS`] copies of the one pair, and `scratch::materialise` names each one
/// for the slot it belongs to — `A0-drift_shell.kir`. That is the requirement
/// rather than a consequence of it: a slot is the unit that gets replaced, so a
/// deck whose file is also another deck's cannot be moved on its own, and one
/// save would rebuild all four. See `scratch.rs`'s header for the rule this
/// replaced and why it went.
///
/// The `Err` is a sentence naming the path that would not be read or written,
/// which is [`materialise`](karakuri_environment::scratch::materialise)'s own.
fn working_copies(
    store: &std::path::Path,
    sources: &Sources,
    slots: usize,
) -> Result<(std::path::PathBuf, Vec<Sources>), String> {
    let mut copies: Vec<Sources> = std::iter::repeat_n(sources.clone(), slots).collect();
    let dir = karakuri_environment::scratch::materialise(
        store,
        // Two distinct fields of one value, which is why this is an array
        // literal rather than a chain: a slot is an L1 and an L4 in node
        // order, and that order is what puts the L1 at `A0` and the renderer
        // at `A1`.
        copies.iter_mut().map(|pair| [&mut pair.l1, &mut pair.l4]),
    )?;
    Ok((dir, copies))
}

/// **What the operator opens, deck by deck**, said once at startup.
///
/// `karakuri-cli` prints one line of this — *"the deck runs from copies here,
/// so the files you named are not written to"* — and one line is not enough
/// here. Four decks on one preset are four files whose names an operator cannot
/// guess and cannot tell apart by content, since at startup they are identical;
/// what makes a file deck B's is its **name**, and the name is this program's
/// choice. So the directory, then a line per deck.
///
/// **Printed rather than drawn.** The panel has no place for a file path: the
/// mixer strip names *material* and would be naming the same thing four times
/// with four spellings, and the Inspector addresses nodes rather than files.
/// The startup print is where this program already says what it resolved — the
/// preset library, the store, the room — and this belongs with those three.
///
/// A free function over the copies so it can be asserted without a window; see
/// `the_startup_print_names_one_file_per_deck`.
fn running_from(dir: &std::path::Path, copies: &[Sources]) -> String {
    let mut said = format!(
        "scratch: {} — every deck runs from its OWN copy here, so the two paths you named \
         are not written to and editing them moves nothing. Point an editor at these:",
        dir.display()
    );
    for (slot, pair) in copies.iter().enumerate() {
        let name = |path: &std::path::Path| {
            path.file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.display().to_string())
        };
        said.push_str(&format!(
            "\n  deck {}: {} + {}",
            deck_letter(slot as u8),
            name(&pair.l1),
            name(&pair.l4)
        ));
    }
    said.push_str(
        "\nthe same pair in every slot is one file per deck per node, and that is the point \
         rather than a duplicate: an edit moves the deck whose file it is and no other, so \
         one save puts one candidate in the Staging lane.",
    );
    said
}

/// **How this program is called**, printed for `--help` and for anything it
/// cannot read as a pair.
///
/// The two flags are in the second paragraph rather than the first, which is
/// where they belong: an operator reading this is looking for how to play
/// something, and the answer is still the pair on the first line. See
/// [`Sources`] for why a flag that answers *where the data lives* is not the
/// second material vocabulary P-0031 refuses.
const USAGE: &str = "\
usage: karakuri [--presets DIR] [--store DIR] [--mcp PORT] [GEOMETRY.kir RENDERER.kir]

  The console, with a deck behind it. Both paths or neither: a Set is an L1 and
  an L4, and with neither the pair that ships in the preset library is played.

  --presets DIR   the shipped preset library. Given, it is used and a directory
                  that is not there is refused. Not given, it is looked for
                  beside this binary — an .app bundle's Resources, a prefix
                  install's share/karakuri, a portable examples/ — and last in
                  the workspace this binary was compiled in. Which one answered
                  is printed at startup. With none, there is no default pair
                  and the two paths have to be given.
  --store DIR     where the Library bay reads Sets and arrangements from, where
                  a save goes, and where the scratch each deck runs from is
                  written. Defaults to .karakuri beside the session.
  --mcp PORT      serve the Model Context Protocol on 127.0.0.1:PORT, so a
                  model can read a deck's procedure, rewrite it, rewire an
                  input and keep what a deck is playing. Loopback only, and
                  0 takes an ephemeral port and prints the one it got.

  Any flag may be given before or after the pair.

  This is not `karakuri-cli`'s command line and does not try to be — that one
  has the audio, the MIDI and the rest of the flags, and its parser is its own.
  It reads the same two directories, and `--store` and `--mcp` mean the same
  thing to both. See `cargo run -p karakuri-cli -- --help`.";

/// **Everything the command line settles**: what plays, and the two
/// directories this program's data is in.
///
/// One value rather than three returns, because the three are decided together
/// and one of them decides another: with no preset library there is no default
/// pair, so [`Sources`] cannot be settled before `presets` is. Carrying the
/// resolution itself rather than only its directory is what lets the legend say
/// *which* candidate answered without asking again and getting a different
/// answer.
#[derive(Debug)]
struct Launch {
    sources: Sources,
    /// Where the Library bay reads and a save writes — `--store`, or
    /// [`karakuri_environment::places::STORE`].
    store: std::path::PathBuf,
    /// The preset library, and which of the places it was. `None` is a machine
    /// with no library on it, which is a state rather than a failure: it is
    /// fatal only for a run that needed a default pair, and the Library's
    /// preset tier is simply empty. See
    /// [`karakuri_environment::places::presets`].
    presets: Option<karakuri_environment::places::Presets>,
    /// **The port a model reaches this run on**, or `None` for a run that
    /// serves nothing — `--mcp PORT`.
    ///
    /// A port and not an open server, because the two are settled in different
    /// places: this function reads a command line and cannot bind a socket
    /// without either failing here or handing back something a `--help` run
    /// would have to close again. [`main`] binds it, before the window, for
    /// the reason the working copies are made there.
    mcp: Option<u16>,
}

/// **The command line, read.** Two paths or none, and two flags that are not
/// about paths; `--help` or `-h` prints [`USAGE`]; anything else is a refusal
/// that prints it.
///
/// A free function over an iterator rather than a read of `std::env::args`
/// inside [`main`], for the reason [`karakuri_environment`]'s refusals are free
/// functions: `main` cannot be called from a test and a refusal nobody can
/// reach is a refusal nobody checked. See
/// `a_set_is_two_paths_or_none_and_anything_else_is_refused` and
/// `the_two_flags_say_where_the_data_is_and_may_sit_on_either_side_of_the_pair`.
///
/// **It reaches a disk now**, which it did not before: resolving a presets root
/// is existence checks on up to four directories. That is not a purity this
/// function had for its own sake — it had it because the answer was a compiled
/// constant — and the alternative is `main` doing the resolution and this
/// function returning something that is not yet an answer, which puts the one
/// refusal an operator will actually meet back out of a test's reach.
fn sources_from<I: IntoIterator<Item = String>>(args: I) -> Result<Launch, String> {
    let args: Vec<String> = args.into_iter().collect();
    if args.iter().any(|a| a == "-h" || a == "--help") {
        return Err(String::new());
    }
    // **A flag on either side of the pair**, which is what a pass over the
    // whole line buys and a `match` on the slice could not: an operator types
    // the flags in whatever order they think of them, and `karakuri-cli`
    // accepts `--store` before or after its own command for the same reason
    // (`list_sets_prints_and_is_never_a_run`).
    let mut named: Option<std::path::PathBuf> = None;
    let mut store: Option<std::path::PathBuf> = None;
    let mut mcp: Option<u16> = None;
    let mut paths: Vec<String> = Vec::new();
    let mut rest = args.into_iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--presets" => {
                named = Some(std::path::PathBuf::from(value_for("--presets", &mut rest)?))
            }
            "--store" => store = Some(std::path::PathBuf::from(value_for("--store", &mut rest)?)),
            // **The same two silences the other two flags refuse**, because
            // [`value_for`] is underneath this one as well: a `--mcp` at the
            // end of the line does not fall back to a port, and `--mcp
            // --store x` does not read `--store` as a number. What this adds
            // is the third — a value that is not a port — which is
            // [`number_for`]'s and is why that function exists here at all.
            "--mcp" => mcp = Some(number_for("--mcp", "a port number", &mut rest)?),
            // **An unknown option is not a path.** Without this a `--prests`
            // typo becomes the first half of a Set and is reported as a file
            // that will not open, which sends the operator looking at their
            // disk for a mistake they made on the command line.
            other if other.starts_with('-') => return Err(format!("unknown option `{other}`")),
            _ => paths.push(arg),
        }
    }

    let presets = karakuri_environment::places::presets(named.as_deref())?;
    let sources = match paths.as_slice() {
        [] => match &presets {
            Some(presets) => Sources::under(&presets.dir),
            // The one place where having no preset library is fatal rather
            // than empty: there is nothing to open on, and the alternative to
            // saying so is a black window. The Library bay's own answer to the
            // same fact is a tier with nothing in it.
            None => return Err(karakuri_environment::places::no_launch_pair()),
        },
        [l1, l4] => Sources {
            l1: std::path::PathBuf::from(l1),
            l4: std::path::PathBuf::from(l4),
        },
        [one] => {
            return Err(format!(
                "one path given (`{one}`) and a Set needs two: a geometry and a renderer"
            ))
        }
        many => {
            return Err(format!(
                "{} paths given and a Set is built from two: a geometry and a renderer",
                many.len()
            ))
        }
    };
    Ok(Launch {
        sources,
        store: store
            .unwrap_or_else(|| std::path::PathBuf::from(karakuri_environment::places::STORE)),
        presets,
        mcp,
    })
}

/// **The value after a flag, refused rather than defaulted or swallowed.**
///
/// `karakuri-cli`'s `value_for` is the same function with the same two
/// silences written on it, and this is the second surface rather than a copy
/// with a different opinion: a flag at the end of the line with nothing after
/// it must not fall back, and a flag whose value is missing must not eat the
/// next flag — `--presets --store x` reading `--store` as a directory would
/// then blame `x` for being an unknown option.
///
/// Two of the three flags here take a directory, and a directory beginning
/// with `-` is a path an operator can still name as `./-odd`. The third takes
/// a number and reads it through [`number_for`], which is where the parse and
/// its refusal are — so this function goes on answering one question.
fn value_for(flag: &str, rest: &mut impl Iterator<Item = String>) -> Result<String, String> {
    match rest.next() {
        Some(value) if !value.starts_with('-') => Ok(value),
        Some(value) => Err(format!(
            "`{flag}` was given no value — `{value}` is an option, not one"
        )),
        None => Err(format!("`{flag}` needs a value")),
    }
}

/// **The number after a flag**, refused rather than defaulted, swallowed or
/// clamped.
///
/// [`value_for`] with a parse behind it, which is `karakuri-cli`'s `number_for`
/// spelled a second time for that file's reason: it is a binary with no library
/// target and there is nothing to call. The three refusals are the three that
/// program gives — no value, a value that is the next flag, and a value that is
/// not a number — so `--mcp` on this command line and `--mcp` on that one are
/// wrong in the same words.
///
/// **A port beginning with `-` is caught by the first two rather than by the
/// parse**, which is the right refusal and not a lucky one: `--mcp -1` is a
/// line where the value is missing far more often than it is a negative number
/// somebody meant.
fn number_for<T: std::str::FromStr>(
    flag: &str,
    what: &str,
    rest: &mut impl Iterator<Item = String>,
) -> Result<T, String> {
    let value = value_for(flag, rest)?;
    value
        .parse()
        .map_err(|_| format!("`{flag} {value}` — expected {what}"))
}

/// **How often a run with `--mcp` wakes to serve.**
///
/// # The loop sleeps, and that is the whole of why this exists
///
/// This window draws a frame when something changed it or when `egui` asked for
/// one after a delay it named, and on no other occasion — [`App::about_to_wait`]
/// is where that rule lives, and it is P-0072's first clause as the operating
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
const SERVED: Duration = Duration::from_millis(100);

/// **One simulation step per frame drawn, and no clock anywhere.**
///
/// [P-0002](../../../docs/principles/0002-simulation-time-comes-from-a-record-never-from-a-clock.md)
/// says simulation time comes from a record and never from a clock. There is
/// no record here — this program is not a session — so the honest
/// third option is neither: a fixed count per frame, which makes the picture's
/// motion a function of frames drawn and of nothing else. It is not a
/// performance, and a `karakuri-cli` that measured an interval and wrote a
/// `tick` is what a performance is.
const STEPS_A_FRAME: u8 = 1;

/// **The look this window opens under**, and it is where [`Engine::look`]
/// starts rather than what every frame is drawn under.
///
/// [`compose`] writes the tone-map uniform on every frame from the
/// [`Committed`] the closure hands back, so a harness with nothing to say
/// about the look still has to say something. **This file now has something to
/// say**: the transport row's two look controls move [`Engine::look`] through
/// a record, so what a frame is committed under is that field and this is only
/// its first value.
///
/// **Aces, and it is still not this program inventing an aesthetic.** ADR-0037
/// picked the default *by looking* and left the trade open — *"ACES works on
/// stage … AgX is kind to material"* — and recorded that the choice is only
/// about what happens when nobody chooses, *"and can be changed on the
/// night"*. Until this pass nobody could change it here; now a press can, and
/// the constant is what the night starts at.
const LOOK: Look = Look {
    op: TonemapOp::Aces,
    exposure: 1.0,
    white_point: 1.0,
};

/// What the picture and every preview are rendered in: an sRGB format, so the
/// hardware does the one encode `Present`'s shader relies on ([P-0064](../../../docs/principles/0064-the-pipeline-is-linear-hdr-and-srgb-is-encoded-once-at-final-output.md)).
const PICTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

/// **The same texels, read as if they were not sRGB, which is how `egui` wants
/// them.**
///
/// `egui`'s fragment shader says so outright — *"We expect 'normal' textures
/// that are NOT sRGB-aware"* — and multiplies the sample by the vertex tint in
/// gamma space. A view in [`PICTURE_FORMAT`] would have the hardware decode to
/// linear on the way in and `egui` would then write those linear values into a
/// gamma-space surface, which is a picture that comes out visibly dark with no
/// error anywhere. So the render target is sRGB, the sampled view is this, and
/// the encode still happens exactly once — in the present pass.
const PICTURE_SAMPLED_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// What the view `egui` samples is called on the device. The **texture**
/// carries the name that says which of the two it is — see
/// [`Presented::label`] — and this says which view of that texture it is, so
/// the pair reads as one thing in a validation message rather than as two with
/// the same name.
const SAMPLED_LABEL: &str = "as egui reads it";

/// **A texture the engine presents into and the panel samples**: the texture
/// itself, the view [`Present::draw`] draws into, the registration `egui`
/// reads it by, and its size in physical pixels.
///
/// **Two call sites on the day it is written**, which is this repository's
/// rule about an abstraction and is the same standing
/// [`karakuri_console::view::bay_head`] has: the Program bay's picture, and
/// deck A's preview cell under it. The four fields were `Engine`'s own until
/// the second one needed them, and every argument written on them then is
/// written on them here, because all of it is still true of both.
///
/// # It is a [`Sink`], and the aiming is the half a `Sink` cannot carry
///
/// [`Sink::acquire`] is handed a `&Gpu` and nothing else, and sizing one of
/// these needs the region's rectangle, the scale factor and the
/// [`egui_wgpu::Renderer`] the registration lives in. So the frame does that
/// first, through [`Presented::aim`], and `acquire` answers from what it was
/// aimed at: a rectangle means a target, no rectangle means [`Skip::Transient`]
/// and nothing at all is drawn into it.
///
/// **The split is where it is because of what a test can call.** Deciding
/// *which rectangle, at what size* inside `window_event` is deciding it
/// somewhere `winit` will not let a test reach — and sizing deck A's texture
/// from the picture's rectangle was injected there once and every test still
/// passed. [`aims`] and [`Presented::aim`] are that decision, whole, outside
/// the event handler; `mod gpu` calls them the way the frame does.
///
/// **Neither of these is presented anywhere**, which is why
/// [`Sink::present`] is `Ok(())` for both: the picture and the preview cell
/// are textures the panel samples, and the thing that reaches a display is the
/// window — which is not a sink here. `karakuri-cli`'s window shows the
/// canvas; this window shows the panel, and the panel goes in `finally`.
struct Presented {
    /// The texture. Held because the views below are of it, and because
    /// freeing the `egui` registration does not free this — `egui-wgpu` stores
    /// a bind group for a registered native texture and no texture at all, so
    /// dropping this is the only thing that releases the memory. Read only by
    /// `mod gpu`, which asks it what size it came out, and that is the whole
    /// of why the lint has to be told: the window does not read it and the
    /// window is not what it is for.
    #[allow(dead_code)]
    texture: wgpu::Texture,
    /// What [`Present::draw`] draws into: sRGB, so the encode is the
    /// hardware's.
    target: wgpu::TextureView,
    /// The registration `egui` draws by, of a view in
    /// [`PICTURE_SAMPLED_FORMAT`].
    id: egui::TextureId,
    /// The texture's size in physical pixels — **its region's**, not the
    /// window's.
    size: (u32, u32),
    /// What the texture is called on the device. Kept rather than passed to
    /// [`Presented::fit`], so the texture a resize makes is called what the
    /// one it replaces was called; and its own per texture, so a device
    /// message about the preview does not read as one about the picture.
    label: &'static str,
    /// **Whether this sink has a rectangle on screen this frame**, written by
    /// [`Presented::aim`] and read by nothing but [`Sink::acquire`].
    ///
    /// It is a field rather than an argument because the two questions are
    /// asked at different moments and by different callers: the frame aims
    /// every sink before it composes, and `compose` then asks each one for
    /// itself. A `Presented` that has never been aimed answers no, which is
    /// the right answer — it has a 1x1 placeholder texture and nothing has
    /// said where it goes.
    aimed: bool,
}

impl Presented {
    /// **A sink aimed at the rectangle its region has**, or at nothing where
    /// the region is folded away.
    ///
    /// It goes through [`Presented::aim`] rather than sizing the texture here,
    /// so that *which rectangle, at what size* has exactly one derivation in
    /// this file and the window's first frame cannot disagree with the window
    /// it opened at. The 1x1 below is never drawn into and never on screen: it
    /// is what `aim` replaces on the same statement.
    ///
    /// The `freed` it counts into is discarded, and that is the one place in
    /// this file where it may be. [`Engine::freed`] is a tally of registrations
    /// leaked by a **resize**, and this is construction — a caller that saw a
    /// 1 here would be told a leak had already happened before the window drew.
    fn new(
        gpu: &Gpu,
        renderer: &mut egui_wgpu::Renderer,
        label: &'static str,
        at: Option<egui::Rect>,
        scale: f32,
    ) -> Presented {
        let mut presented = Presented::made(gpu, renderer, label, (1, 1));
        let mut construction = 0;
        presented.aim(gpu, renderer, at, scale, &mut construction);
        presented
    }

    /// The texture, the view the engine draws into, and the registration
    /// `egui` reads it by, at a size in physical pixels. One constructor
    /// because the three are made together and are replaced together, and
    /// private to this type because a caller aims at a rectangle — turning one
    /// into a size is [`Presented::aim`]'s and [`Presented::fit`]'s alone.
    fn made(
        gpu: &Gpu,
        renderer: &mut egui_wgpu::Renderer,
        label: &'static str,
        size: (u32, u32),
    ) -> Presented {
        let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: size.0.max(1),
                height: size.1.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: PICTURE_FORMAT,
            // COPY_SRC is not for the frame path — nothing here ever copies
            // one of these — it is what lets a test read a cell back and say
            // that a pass really was recorded into it. `Deck::slot_target`
            // carries the same flag for the same reason and says so.
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            // Declared so the sampled view below may reinterpret it — the two
            // formats differ only in whether the transfer function is applied.
            view_formats: &[PICTURE_SAMPLED_FORMAT],
        });
        let target = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampled = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some(SAMPLED_LABEL),
            format: Some(PICTURE_SAMPLED_FORMAT),
            ..Default::default()
        });
        let id = renderer.register_native_texture(&gpu.device, &sampled, wgpu::FilterMode::Linear);
        Presented {
            texture,
            target,
            id,
            size,
            label,
            aimed: false,
        }
    }

    /// **Where this sink goes this frame, and how big its texture therefore
    /// is** — one statement, and it is the whole of what [`Sink::acquire`]
    /// then answers from.
    ///
    /// Returns what the console should draw, or `None` where there is no
    /// rectangle. **The returned [`Picture`] carries the same rectangle the
    /// texture was just sized from and the id the sizing may have just
    /// replaced**, which is the pairing `karakuri_console::view::Picture`'s own
    /// documentation asks for: whoever sized the texture and whoever placed it
    /// are one statement, so a texture sized from the window and drawn into the
    /// picture's region cannot be written by accident, and a resize cannot
    /// leave a freed id in the view.
    ///
    /// **Called before [`compose`], and it has to be**: `acquire` takes only a
    /// `&Gpu`, and this needs the rectangle, the scale factor and the
    /// renderer. It is also a reallocation on the frames a size changed, so it
    /// belongs at the top of the frame rather than mid-pass — see
    /// [`Presented::fit`].
    fn aim(
        &mut self,
        gpu: &Gpu,
        renderer: &mut egui_wgpu::Renderer,
        at: Option<egui::Rect>,
        scale: f32,
        freed: &mut usize,
    ) -> Option<Picture> {
        self.aimed = at.is_some();
        // **No rectangle, so nothing to fit.** The texture is left exactly as
        // it was rather than shrunk: a folded region is one an operator
        // unfolds, and remaking it small and large again would put two
        // reallocations on a fold that costs none. Nothing is drawn into it
        // meanwhile — `acquire` refuses, which is the whole of what a fold
        // has to mean.
        let rect = at?;
        self.fit(gpu, renderer, physical(rect, scale), freed);
        Some(Picture { id: self.id, rect })
    }

    /// **The region is a different size, so the texture is remade at that size
    /// and the registration it replaces is freed.**
    ///
    /// Returns whether anything was remade, which is `false` on all but a
    /// handful of frames — every frame of a drag on the program's height is
    /// one of them, and that is exactly the case the free is for. A
    /// `register_native_texture` per dragged frame with no `free_texture`
    /// beside it is a bind group and a sampler leaked per frame, for as long
    /// as somebody keeps hold of a divider.
    ///
    /// A reallocation, so it is the frame's first act and not something done
    /// mid-pass ([P-0001](../../../docs/principles/0001-nothing-allocates-or-compiles-a-shader-on-the-render-thread.md)
    /// is about the render thread, and this is that thread; what it costs is
    /// paid on the frames a size changed and on no others).
    ///
    /// **`freed` is handed in rather than kept here** because the tally is the
    /// whole engine's — one number over both textures, which is what
    /// [`Engine::freed`] is and what `mod gpu` asserts on. Taking it as an
    /// argument is what makes it impossible for a call site to remake a
    /// texture and forget to count what it freed.
    fn fit(
        &mut self,
        gpu: &Gpu,
        renderer: &mut egui_wgpu::Renderer,
        size: (u32, u32),
        freed: &mut usize,
    ) -> bool {
        if size == self.size {
            return false;
        }
        // Freed **before** the replacement is registered, which is the order
        // the leak is about rather than a tidiness: the atlas holds the old
        // bind group until this call and nothing else ever drops it.
        renderer.free_texture(&self.id);
        *freed += 1;
        // Carried across, because a resize is not an un-aiming: the rectangle
        // this was aimed at is the one it was just resized to.
        let aimed = self.aimed;
        *self = Presented::made(gpu, renderer, self.label, size);
        self.aimed = aimed;
        true
    }
}

/// **The two textures are sinks, and this is the whole of what that costs.**
///
/// `acquire` answers from [`Presented::aim`] and nothing else; `present` is
/// `Ok(())` because neither of these is presented anywhere — they are sampled
/// by the panel, and the panel is drawn in [`compose`]'s `finally` rather than
/// by a sink. See the type's own documentation.
impl Sink for Presented {
    fn acquire(&mut self, _gpu: &Gpu) -> Result<(), Skip> {
        match self.aimed {
            true => Ok(()),
            // **[`Skip::Transient`] and never a `Fault`.** A folded region is
            // an operator's choice and the next `f` over it undoes it, so
            // there is nothing to say about it — and a `Fault` here would be
            // a line printed the first frame of every fold.
            false => Err(Skip::Transient),
        }
    }

    fn view(&self) -> &wgpu::TextureView {
        &self.target
    }

    fn size(&self) -> (u32, u32) {
        self.size
    }

    /// **Nothing.** The picture and deck A's cell are textures the panel
    /// samples in the same submission; what reaches a display is the window,
    /// and the window is not a sink here — `karakuri-cli`'s window shows the
    /// canvas, and this one shows the panel.
    fn present(&mut self, _gpu: &Gpu) -> Result<(), String> {
        Ok(())
    }
}

/// **The engine behind the Program bay: a deck of [`SLOTS`] Sets, the present
/// pass, and the two textures it lands in.**
///
/// Scaffolding still in what it is wired to — no audio, no MIDI, no store, no
/// arguments — and a watcher on each slot, which is the one thing here that is
/// not the shortest path to texels and is there because the Staging lane's
/// rows are verdicts on builds ([`watched`]). What `karakuri-cli` does around
/// this is a program; what is here is the shortest path from two `.kir` files
/// to texels — and now back again, which is what a watched slot is.
///
/// **The deck is full, and the slots are channels rather than exhibits.** It
/// has every slot a `Deck` can hold, because a strip is a slot and a mixer is
/// its channels; what is *in* them is this program's one pair at four salts,
/// which is what a slot nobody has loaded anything into holds ([`Engine::new`]).
/// [`ON_AIR`] is Live and is the whole of the picture. Every other slot rests
/// at `Residency::Allocated` — not stepping and contributing nothing, though
/// each is still drawn into its own cell —
/// and [`ASKED_TO_PRIME`] is additionally asked to warm up and parked by the
/// budget in [`Engine::ask_to_prime`], which is what puts a pending request on
/// this panel for the mixer's tally to draw. **The three cost a draw each and
/// no step**: measured on 2026-09-02, four slots at 262144 elements were about
/// 10 ms a frame against one slot's 5.9, and none of it reaches the governor,
/// which reads a per-Set cost. This sentence used to say the three cost
/// nothing, which was true only while an off-air slot was not drawn.
///
/// **All four preview cells are on, whatever the decks are doing.** A cell is
/// drawn because there is a slot behind it ([`Engine::aim`]), and this deck is
/// full, so four cells show four slots' own material: deck A stepping and on
/// air, deck B warming or parked, C and D standing at the still they stopped
/// at. That is [P-0080](../../../docs/principles/0080-an-operator-can-see-a-slots-own-material-without-putting-it-on-air.md)
/// met on this surface — the operator watches a candidate's cell to decide
/// whether it is worth a fader, and then raises the fader. It used to be gated
/// on `Residency::Live`, which left the three cells worth looking at dark; the
/// gap P-0080 named under *Where it is not met* was this line.
struct Engine {
    deck: Deck,
    /// **Elements per geometry, read off the L1's own `capacity` declaration**
    /// rather than named here — see [`Engine::new`]. Kept because the reading
    /// [`Costs::say`] prints names it, and a workload figure that is not the
    /// one the run used is worse than none.
    capacity: u32,
    present: Present,
    /// The Program bay's picture.
    picture: Presented,
    /// **The four deck preview cells**, one per deck slot.
    previews: [Presented; DECKS],
    /// Cached bind groups for each slot view into the tone-mapping pipeline.
    slot_bind_groups: [Option<wgpu::BindGroup>; DECKS],
    /// **The look every sink is drawn under this frame**, and the one piece of
    /// engine state this program *moves*.
    ///
    /// It was [`LOOK`] handed straight to `compose` every frame, with the
    /// reason written at that constant: this file had no session, no `look`
    /// record and no key that changed it. It has a control now — the transport
    /// row's tone map capsule and its exposure track — so a press becomes
    /// `Operation::SetTonemap` or `SetExposure`, which become one
    /// `Record::Look`, which [`apply`] writes here; the next frame hands this
    /// to `compose` and the present pass uploads it. That is P-0028 on this
    /// value exactly: the control ends in the record every other surface's
    /// does, and nothing calls `Present::set_tonemap` behind its back.
    ///
    /// **It lives here rather than beside the panel** because it is what the
    /// *engine* is drawing under: `view::Look` is the console's reading of it,
    /// written per frame from this the way a strip is written from the deck,
    /// and a second copy that the console owned would be the reading and the
    /// state as one thing (ADR-0156).
    ///
    /// `white_point` is carried and never asked for: no surface has a control
    /// for it, so it is read back into every record and written out again
    /// unchanged ([ADR-0192](../../../docs/adr/0192-an-operation-asks-for-what-a-surface-can-say-and-the-record-stays-whole.md)).
    look: Look,
    /// How many registrations have been freed, **over both textures**. The
    /// atlas leak this exists to prevent is invisible from outside: a resize
    /// that registers without freeing leaves a bind group per drag frame and
    /// nothing says so, so the count is kept and `mod gpu` asserts on it. It
    /// is the whole engine's tally rather than either texture's, which is why
    /// it lives here and is handed to [`Presented::fit`].
    freed: usize,
    /// **One [`Aiming`] per slot, in slot order**: how a load or a rewiring
    /// reaches that slot's build worker, and where that watcher is pointed.
    ///
    /// This is the whole of what putting a library Set on a running deck took,
    /// and what it is *not* is the point of it. `Deck::install` is the one
    /// function that puts a built Set in a slot and says of itself that it is
    /// *"deliberately not reachable from a key or a surface: a live run
    /// changes its material by editing a file and letting the worker build it,
    /// which is what the budget watchdog is attached to."* So nothing here
    /// builds a Set: [`loading`] writes the library Set's procedures into the
    /// scratch and sends an aim, and the same worker that watches for a save
    /// picks it up. The swap lands at a frame boundary, is judged against the
    /// budget for thirty frames, and rolls back on its own if it costs too
    /// much — none of which had to be written for the library, because a load
    /// is now literally an edit this program made.
    ///
    /// **In slot order, so the index is the deck letter**: `aimed[0]` is deck
    /// A's, and it is the same index `Deck::events`, the strips and the
    /// preview cells are all in. Kept beside the deck rather than inside it
    /// for the reason the whole of [`Engine`] is on this side: the channel is
    /// `karakuri-environment`'s and the engine takes no environment.
    aimed: Vec<Aiming>,
    /// **Every node the run launched with**, in file order, with the bytes each
    /// one was compiled from — see [`karakuri_environment::compile::Placed`].
    ///
    /// **One list for four slots, because the four files hold the same bytes.**
    /// [`working_copies`] writes the one pair the command line settled into
    /// every slot, so a node's layer, its index, its address and its source are
    /// the same answer four times; the only per-slot difference is the *path*,
    /// which each watcher is given from `slots[slot]` and which no part of a
    /// saved node carries. A second compile per slot would be four answers to
    /// one question with a window between them — see [`Placed::source`], which
    /// is where that hazard is written.
    ///
    /// This is what [`Playing`] is seeded from, and it is the reason a deck can
    /// be saved on the first frame rather than only after something has been
    /// rebuilt.
    placed: Vec<karakuri_environment::compile::Placed>,
}

/// **One slot's watcher, and where it is pointed.**
///
/// `karakuri-cli`'s `Aiming` is the same pair for the same reason, restated
/// here because that program is a binary with no library target and there is
/// nothing to call.
///
/// **The aim is kept and not only the sender**, because a [`watch::Aim`] is
/// every field of the slot's identity and *anything left out comes back as the
/// outgoing slot's* — a fold silently un-selected, a camera back at
/// `Orbit::default()`, salts that repaint every element. A rewiring changes one
/// field of thirteen, so the other twelve have to be restated from somewhere,
/// and this is that somewhere: what the watcher was constructed with until the
/// first aim, and the last aim after that.
///
/// **This program had the sender and not the aim**, which was harmless for as
/// long as the only thing that sent one was [`loading`] — a load states every
/// field off the Set file it read. It stops being harmless the moment anything
/// changes *one* field, which is what `wire_input` does: a rewiring that
/// restated the launch pair would have thrown away the Set the operator had
/// just loaded.
struct Aiming {
    /// The other end of [`watch::Watch::aimed_by`]'s channel, for this slot's
    /// watcher and no other. A watcher re-pointed through somebody else's
    /// sender would rebuild a deck nobody named.
    aim: std::sync::mpsc::Sender<watch::Aim>,
    /// Where that watcher is pointed, kept in step with what has been sent.
    at: watch::Aim,
}

impl Aiming {
    /// **Point the watcher at what it is already looking at, with `edges`
    /// instead**, and answer whether it is still there to be pointed.
    ///
    /// `Err` is a build worker that has ended — the receiver is gone — which is
    /// a run shutting down. It is reported rather than swallowed: the edge is in
    /// the run's wiring either way, and *nothing will rebuild* is a different
    /// fact from *the slot is recompiling*.
    fn re_aim(&mut self, edges: Vec<karakuri_engine::set::Edge>) -> Result<(), ()> {
        self.at.edges = edges;
        self.aim.send(restated(&self.at)).map_err(|_| ())
    }

    /// **Point it at something else entirely**, keeping the aim that was sent.
    ///
    /// The one route a load takes, and the reason [`loading`] is handed this
    /// rather than the sender: a load that sent an aim and left `at` behind
    /// would leave the *next* rewiring restating the material the run launched
    /// with, which is the hardest version of this mistake to see.
    fn re_point(&mut self, aim: watch::Aim) -> Result<(), ()> {
        self.at = aim;
        self.aim.send(restated(&self.at)).map_err(|_| ())
    }
}

/// One aim, said again — because [`watch::Aim`] is not `Clone` and a re-point
/// restates every field of it.
///
/// **No `..` on either side of this**, which is `Watch::repointed`'s own rule
/// met from the sending end: it destructures with no `..` so that a field added
/// to `Aim` cannot be left behind, and a *sender* that filled the new field
/// with a default would defeat that from here. The compiler names all thirteen,
/// so the day a fourteenth arrives this stops compiling rather than quietly
/// re-aiming a slot at it.
fn restated(aim: &watch::Aim) -> watch::Aim {
    let watch::Aim {
        head,
        rest,
        layering,
        live,
        capacity,
        seed_salt,
        salts,
        camera,
        overrides,
        published,
        bindings,
        edges,
        authorities,
    } = aim;
    watch::Aim {
        head: head.clone(),
        rest: rest.clone(),
        layering: *layering,
        live: *live,
        capacity: *capacity,
        seed_salt: *seed_salt,
        salts: salts.clone(),
        camera: *camera,
        overrides: overrides.clone(),
        published: published.clone(),
        bindings: bindings.clone(),
        edges: edges.clone(),
        authorities: authorities.clone(),
    }
}

// ---------------------------------------------------------------------------
// Keeping what a deck is playing
// ---------------------------------------------------------------------------

/// **What each deck is running, as the nodes a Set file names.**
///
/// `karakuri-cli`'s `Running` is the same fact held the same way, and this is
/// the second surface rather than a copy with a different opinion — that
/// program is a binary with no library target, so there is nothing to call.
/// What differs is the shape and only the shape: that one holds addresses and
/// zips the launch bytes back on at save time, and this one holds the
/// [`setfile::SavedNode`] whole, because the panel has one launch list for four
/// slots and nothing to zip it against.
///
/// **It is seeded before the first frame**, from [`Engine::placed`] — so every
/// deck can be written down from the outset rather than only after something
/// has been rebuilt. A slot that is `None` is one whose last build's sources did
/// not reach the store, which the watcher said at the time; it saves nothing
/// rather than guessing.
///
/// **A type of its own rather than two fields on [`App`]**, because the pair is
/// one fact with one transition rule: a swap moves `playing` into `previous`, a
/// rollback moves it back, and the two halves are never right apart.
struct Playing {
    playing: Vec<Option<Vec<setfile::SavedNode>>>,
    /// What a rollback restores, and the only way to name it: a rollback brings
    /// back a Set nothing will name again.
    previous: Vec<Option<Vec<setfile::SavedNode>>>,
}

impl Playing {
    /// **Every slot seeded from the material this run compiled**, addressed by
    /// the bytes that compile read.
    ///
    /// **No store, no disk and nothing that can fail.** The bytes ride along in
    /// [`setfile::SavedNode::source`] and reach the store at the moment a file
    /// names them, which is [`setfile::Sources::into_nodes`] — so a run that
    /// never saves writes no artifact.
    fn at_launch(placed: &[karakuri_environment::compile::Placed], slots: usize) -> Playing {
        let nodes: Vec<setfile::SavedNode> = placed
            .iter()
            .map(|node| setfile::SavedNode {
                layer: setfile::kind_name(node.layer),
                index: node.index,
                hash: node.hash(),
                name: node.named.name.clone(),
                source: Some(std::sync::Arc::clone(&node.source)),
                meta: Some(std::sync::Arc::clone(&node.meta)),
            })
            .collect();
        Playing {
            playing: (0..slots)
                .map(|_| match nodes.is_empty() {
                    true => None,
                    false => Some(nodes.iter().map(copied).collect()),
                })
                .collect(),
            previous: (0..slots).map(|_| None).collect(),
        }
    }

    /// What `slot` is running, or `None` for a slot with no address to name.
    fn at(&self, slot: usize) -> Option<&Vec<setfile::SavedNode>> {
        self.playing.get(slot).and_then(Option::as_ref)
    }

    /// **A build landed.** `nodes` is `None` when that build's sources never
    /// reached the store — the watcher says so at the time, and the addresses it
    /// would have named do not exist.
    ///
    /// **That is still a swap**, and taking it as one is the whole of why this
    /// is a method rather than an assignment at the call site: the slot is on
    /// something new, so the version it was on becomes what a rollback restores,
    /// and the slot itself has no address until the next build lands.
    fn landed(&mut self, slot: usize, nodes: Option<Vec<setfile::SavedNode>>) {
        if slot >= self.playing.len() {
            return;
        }
        self.previous[slot] = self.playing[slot].take();
        self.playing[slot] = nodes;
    }

    /// **A build was rolled back**, so the slot is running what it was running
    /// before it. With the launch version in `previous`, the first rollback of a
    /// slot restores it like any other.
    fn rolled_back(&mut self, slot: usize) {
        if slot >= self.playing.len() {
            return;
        }
        self.playing[slot] = self.previous[slot].take();
    }
}

/// One saved node, again — because [`setfile::SavedNode`] is not `Clone` and a
/// save consumes the list it is handed while the run goes on holding it.
///
/// The bytes and the card are `Arc`s and are shared rather than duplicated,
/// which is what those two fields are `Arc`s for.
fn copied(node: &setfile::SavedNode) -> setfile::SavedNode {
    setfile::SavedNode {
        layer: node.layer,
        index: node.index,
        hash: node.hash,
        name: node.name.clone(),
        source: node.source.clone(),
        meta: node.meta.clone(),
    }
}

/// **What a build that just landed is running**, from the addresses the watcher
/// reported and the names the slot is spelled with.
///
/// The bytes are `None` for every one of them, and that is
/// [`setfile::SavedNode::source`]'s own rule rather than an omission: the
/// watcher put this build's sources in the store as it built them, so there is
/// nothing left here to carry.
///
/// **The names come off the aim the slot is pointed at**, zipped by position.
/// `watch::Built::nodes` is built from the sort's `Placed`, which keeps file
/// order, and [`Aiming::at`]'s head and rest are that same file list — so entry
/// `n` of one is entry `n` of the other. A name belongs to the *use* rather
/// than to the procedure, so nothing a hash carries could hold it, and the
/// alternative is a Set loaded with named nodes coming back after its first
/// rebuild with the `edge` records pointing at nothing.
fn built_nodes(built: &watch::Built, at: &watch::Aim) -> Vec<setfile::SavedNode> {
    let names: Vec<Option<String>> = std::iter::once(at.head.name.clone())
        .chain(at.rest.iter().map(|node| node.name.clone()))
        .collect();
    built
        .nodes
        .iter()
        .enumerate()
        .map(|(node, (layer, index, hash))| setfile::SavedNode {
            layer,
            index: *index,
            hash: *hash,
            name: names.get(node).cloned().flatten(),
            source: None,
            meta: None,
        })
        .collect()
}

/// **What a Set file says about the Set that is playing**, read off that Set.
///
/// `karakuri-cli`'s `playing_values` is this function and its doc is the
/// argument for every line: **eight of the nine are read from the Set and not
/// from anything this program was told**, because a writer with its own copy of
/// the rule records numbers the run was not using and the file then describes a
/// picture nobody has seen. The capacities are the Set's per geometry, the
/// params are every declaration of every node at the value it is holding, the
/// layering and the fold are the Set's rather than a flag's, and the salts are
/// what it *is* salted with rather than what a position would derive.
///
/// The ninth is `edges`, which is the run's — see [`App::edges`]. It is not the
/// Set's for the reason `mcp::WireRequest` states: the wiring a slot rebuilds
/// with is not on disk anywhere, and the run is the only thing that holds it.
///
/// Restated here rather than called, for [`number_for`]'s reason.
fn playing_values(
    set: &karakuri_engine::Set,
    edges: &[karakuri_engine::set::Edge],
) -> setfile::Owned {
    setfile::Owned {
        // Filled where a store is open, and nowhere else — see [`Save::run`].
        nodes: Vec::new(),
        capacities: set.source_capacities(),
        params: set
            .params()
            .map(|(layer, index, key, value)| {
                karakuri_engine::ParamWrite::at(layer, index, key, value)
            })
            .collect(),
        bindings: set.bindings().to_vec(),
        edges: edges.to_vec(),
        camera: set.camera,
        layering: set.layering(),
        live: selected_renderer(set.inputs()),
        seeds: set.source_salts().to_vec(),
    }
}

/// **Which renderer a Set is folded to**, as a `merge` record spells it:
/// `Some(i)` where exactly one input is live, and `None` where every one of them
/// is.
///
/// **Every-live is checked first, and that decides the one-renderer case.** A
/// composited Set holding a single renderer has one live input, which is both
/// "all of them" and "exactly one" — and it is the first, because such a Set is
/// one nobody has selected in. Writing `live 0` for it would record a choice
/// that was never made.
fn selected_renderer(inputs: &[karakuri_engine::mix::Input]) -> Option<u32> {
    if inputs.iter().all(|input| input.live) {
        return None;
    }
    let mut live = inputs.iter().enumerate().filter(|(_, input)| input.live);
    match (live.next(), live.next()) {
        (Some((at, _)), None) => Some(at as u32),
        _ => None,
    }
}

/// Whether this deck holds `slot`. The companion of
/// [`karakuri_environment::no_such_slot`], which is the sentence it is refused
/// in.
fn slot_in_range(slot: usize, slot_count: usize) -> bool {
    slot < slot_count
}

/// **One live save, from the frame that asked for it to the file on disk.**
struct Save {
    slot: usize,
    id: String,
    /// The store root, not an open store: opening it creates directories, which
    /// is I/O, which belongs on the thread below rather than on a frame.
    root: std::path::PathBuf,
    sources: setfile::Sources,
    /// **What the file will say, with `nodes` still empty.** The nodes are the
    /// one part of a Set file that needs a store — a hash per source — so they
    /// are filled in where one is opened and never here.
    values: setfile::Owned,
}

impl Save {
    /// Write it. **Everything here is off the render thread**: opening a store
    /// creates directories, and the Set file itself is written and renamed into
    /// place.
    fn run(self) -> Result<(), String> {
        let Save {
            id,
            root,
            sources,
            mut values,
            ..
        } = self;
        let store = Store::open(&root).map_err(|e| format!("store `{}`: {e}", root.display()))?;
        // **Before the file that references them**, which is what
        // `setfile::Node` carrying a hash asks of every caller: the writer
        // cannot check that a hash resolves without reading the store back, so
        // putting them is the caller's promise.
        values.nodes = sources.into_nodes(&store)?;
        setfile::save(&store, &id, values.saving())
    }
}

/// What a live save came back with, at the frame it arrives.
struct Saved {
    slot: usize,
    id: String,
    /// `Ok` and the file is on disk under `id`. **A failure is printed and
    /// nothing claims otherwise**: a program saying a save happened when the
    /// disk refused is the shape of lie this codebase is arranged against.
    outcome: Result<(), String>,
    /// Where a client that asked for this save is waiting, and `None` when a
    /// hand pressed `k`.
    ///
    /// **It rides the save rather than being looked up when the outcome
    /// lands.** A map from an id to whoever asked would be a second place that
    /// knows which save is which, and the outcome already carries everything
    /// needed to find its way home.
    reply: Option<mcp::Reply>,
}

/// A save that will not happen, to the terminal and to whoever asked if that was
/// not a hand.
///
/// **One sentence and one home.** Every refusal here reaches two audiences, and
/// the way that goes wrong is a copy of the words for the second one — free to
/// be right on the day it is written and wrong at the next correction.
fn refused(reply: Option<mcp::Reply>, said: String) {
    println!("{said}");
    if let Some(reply) = reply {
        reply.settled(Err(said));
    }
}

/// **Every edge a client asked for on one frame, applied to the run's wiring and
/// answered.**
///
/// This is `karakuri_environment::mcp::WireRequest`'s three points, and it is a
/// free function so that all three are checkable without a window, a GPU or a
/// `Deck` — the wiring, the re-aim and the sentence are the whole of what this
/// decides, and none of them needs one. `karakuri-cli`'s `rewired` is the same
/// three decisions for the same reasons; it is restated rather than called for
/// [`number_for`]'s reason.
///
/// # Replace, keyed on the input
///
/// An edge is dropped and the new one appended, keyed on `(node, slot)` — the
/// node that declares the input and what its procedure calls it. It is forced
/// rather than chosen: `SetError::SlotBoundTwice` refuses two edges on one input
/// where the Set is built, so an append would make the *second* call on an input
/// a refusal and leave a model unable to change its mind.
///
/// **The key does not include the deck slot, because the run's wiring does
/// not.** [`App::edges`] is one list for the whole run and an edge naming a node
/// a Set has not got is passed over where the Set is built. So a request names a
/// deck slot to say *which slot rebuilds*, and two slots holding a node of the
/// same name share one entry in this list.
///
/// # A slot this deck does not hold
///
/// **Refused, in [`karakuri_environment::no_such_slot`]'s words, and nothing is
/// rewired** — the decision [`App::save_set`] already makes and for its reason:
/// the server checks the number against its own `Slots` before it sends, and
/// this is the guard that does not depend on it having.
///
/// # The same input wired twice on one frame
///
/// **Every request is applied, in the order it arrived, and the last one is what
/// the run is wired with.** One aim per slot goes out after all of them are in
/// the list, so the rebuild carries the settled wiring rather than an
/// intermediate one. **A request the same frame overwrote is told so**: its edge
/// *was* written and then replaced, and a reply saying only "wired" would be a
/// true sentence about a state the run no longer holds.
fn rewired(
    asked: &[(usize, karakuri_engine::set::Edge)],
    edges: &mut Vec<karakuri_engine::set::Edge>,
    aims: &mut [Aiming],
    slot_count: usize,
) -> Vec<Result<String, String>> {
    use std::fmt::Write as _;
    // **Every edge into the list before any watcher is re-aimed**, so that a
    // frame carrying two of them rebuilds once, at the wiring the frame ended
    // with.
    let mut said: Vec<Option<Result<String, String>>> = asked.iter().map(|_| None).collect();
    let mut named: Vec<usize> = Vec::new();
    for (at, (slot, edge)) in asked.iter().enumerate() {
        if !slot_in_range(*slot, slot_count) {
            said[at] = Some(Err(format!(
                "{}, and nothing was rewired",
                karakuri_environment::no_such_slot(*slot, slot_count)
            )));
            continue;
        }
        edges.retain(|held| !(held.node == edge.node && held.slot == edge.slot));
        edges.push(edge.clone());
        if !named.contains(slot) {
            named.push(*slot);
        }
    }
    // `None` for a slot with no watcher at all, `Some(false)` for one whose
    // build worker has ended: two different things to say, and neither of them
    // is "the slot is recompiling".
    let rebuilding: Vec<(usize, Option<bool>)> = named
        .into_iter()
        .map(|slot| {
            let state = aims
                .get_mut(slot)
                .map(|aiming| aiming.re_aim(edges.clone()).is_ok());
            (slot, state)
        })
        .collect();
    for (at, (slot, edge)) in asked.iter().enumerate() {
        if said[at].is_some() {
            continue;
        }
        let mut line = format!(
            "slot {slot}: wired `{}.{}={}`",
            edge.node, edge.slot, edge.to
        );
        // Keyed on the input alone, like the replacement above, and **only ones
        // that were applied**: a later request refused for its slot number wrote
        // nothing, and telling this one it had been replaced by an edge that
        // never landed would be the same lie in the other direction.
        let over = asked[at + 1..].iter().find(|(later_slot, later)| {
            slot_in_range(*later_slot, slot_count)
                && later.node == edge.node
                && later.slot == edge.slot
        });
        if let Some((_, later)) = over {
            let _ = write!(
                line,
                ", and a later request on this frame replaced it with `{}` — the run is \
                 wired with that one and it is what the rebuild carries",
                later.to
            );
        }
        let state = rebuilding
            .iter()
            .find(|(named, _)| named == slot)
            .and_then(|(_, state)| *state);
        let tail = match state {
            Some(true) => {
                " — the slot is recompiling with it, and `swap_outcome` says what the build \
                 made of it"
            }
            Some(false) => {
                " — this slot's build worker has ended, so nothing will rebuild: the edge is \
                 the run's from here on and a `save_set` of this slot records it"
            }
            None => {
                " — this slot has no watcher, so nothing rebuilds: what is on air was built \
                 with the wiring the run started with, and a `save_set` of this slot records \
                 the edge"
            }
        };
        line.push_str(tail);
        said[at] = Some(Ok(line));
    }
    // Every entry was filled by one of the two loops above: the first answers
    // the refusals and the second answers everything it skipped.
    said.into_iter().map(Option::unwrap).collect()
}

/// **One slot, with a worker watching its own two files behind it** — which is
/// what puts a candidate in the Staging lane and is the whole of what that
/// took.
///
/// # It is `karakuri-cli`'s wiring and deliberately not a second one
///
/// That program builds every `--watch` slot as `HotSwap::new` over a
/// `watch::Watch`, and every argument below is the reading `Set::build` made
/// at startup restated, because that is what a rebuild is: the *material*
/// changed and nothing else about the slot did. A request that derived any of
/// them again would be a slot that comes back as a different Set on the first
/// save — which is the failure `Watch`'s own fields are each documented
/// against.
///
/// - **`Layering::Overdraw` and no `live`** — `Set::build`'s own, which is
///   what every slot was built with: one target, however many renderers.
/// - **No `capacity`** — so each geometry is rebuilt at the capacity it
///   declares, which is [`capacity_of`]'s line asked again on the worker.
///   This program has no `--capacity` to override it (see [`USAGE`]), and
///   passing the startup reading would pin the slot to a declaration the file
///   may have just changed.
/// - **The slot's own salt, and no per-source salts** — `Set::build` passes
///   `&[]` and says why: *"A pair assigns nothing, so the one source is salted
///   from the Set's seed and its ordinal — which for source 0 is that seed
///   unchanged."* So a rebuild is the same simulation of new material rather
///   than a new one, and every slot stays at the salt [`slot_salt`] counted
///   off for it.
/// - **The default camera** — `Request::camera` is an `Orbit` rather than an
///   `Option` because *"a Set holds a built-in camera whatever its files
///   declare"*, and this program loads none, so the default is what it is
///   running.
/// - **No overrides, no published controls, no bindings, no edges and no
///   authorities** — this program has no flag for any of the five and grants
///   nothing (ADR-0216), so each is the empty list the startup build used.
///
/// # The store, and the one thing that is still not done
///
/// `Watch::storing_to` **is** given now and `Watch::snapshotting_to` is not.
/// The first puts every build's sources in the store and reports them as
/// [`watch::Built`], which is where a rebuilt node's address comes from — and
/// nothing could derive one until something needed one, which is *Keep what a
/// deck is playing*: a Set file references its sources by hash, so a slot whose
/// builds were never stored is a slot that cannot be written down. See
/// [`Playing`], which is the other end of that channel.
///
/// The second keeps every version that compiled so an edit can be walked back,
/// which is what *put a node's previous version back* would read. That control
/// is still waiting on it, and this is still not the pass that adds it.
///
/// # What it costs the frame path, which is nothing
///
/// A worker thread per slot, polling the two files every hundred milliseconds
/// and compiling on that thread. The render thread's side is unchanged:
/// `install_if_ready` polls the same channel with `try_recv` whether the
/// `Sender` is live or was dropped at construction, and a swap has always
/// landed at a frame boundary (ADR-0005). What is new on a *frame* is a
/// build's install, which is the mechanism this deck was already built on.
fn watched(
    gpu: &Gpu,
    sources: &Sources,
    live: Set,
    slot: usize,
    salt: u32,
    // Where this watcher puts what it builds, and where it says so — or `None`
    // for a harness with no store to write into. See [`Engine::new`].
    stored: Option<(std::sync::Arc<Store>, std::sync::mpsc::Sender<watch::Built>)>,
) -> (HotSwap, Aiming) {
    // **The other end of `Watch::aimed_by`**, kept by [`Engine`] so that a
    // load can say *look at these files instead*. It is made here rather than
    // by the caller because the watcher it belongs to is made here, and a
    // sender paired with the wrong slot's watcher would load a deck the
    // operator did not name.
    let (aim, aimed) = std::sync::mpsc::channel();
    // **The aim this watcher is constructed with, said once in a value rather
    // than only in the argument list below.** It is `Watch::new`'s arguments
    // less the slot, which is what [`watch::Aim`] is, and it is kept so that a
    // rewiring can restate the twelve fields it does not change. The two lists
    // are read side by side here on purpose: a field that disagreed would be a
    // rewiring that quietly moved something else.
    let at = watch::Aim {
        head: karakuri_environment::compile::Named::bare(&sources.l1),
        rest: vec![karakuri_environment::compile::Named::bare(&sources.l4)],
        layering: Layering::Overdraw,
        live: None,
        capacity: None,
        seed_salt: salt,
        salts: Vec::new(),
        camera: karakuri_engine::camera::Orbit::default(),
        overrides: Vec::new(),
        published: Vec::new(),
        bindings: Vec::new(),
        edges: Vec::new(),
        authorities: Vec::new(),
    };
    let watching = watch::Watch::new(
        slot,
        // **Bare, so every node is called what its procedure declares**,
        // which is `Set::build`'s own: *"A pair names nothing, so both
        // nodes are called what their procedures are."* A name here
        // belongs to the *use* and this program has no syntax for one.
        karakuri_environment::compile::Named::bare(&sources.l1),
        vec![karakuri_environment::compile::Named::bare(&sources.l4)],
        Layering::Overdraw,
        None,
        None,
        salt,
        Vec::new(),
        karakuri_engine::camera::Orbit::default(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
    )
    .aimed_by(aimed);
    // **Where a rebuild's sources go**, so that what a slot is running has an
    // address a Set file can name. Nothing is put until a build happens, and
    // the put is on the worker thread that compiled it.
    let watching = match stored {
        Some((store, tx)) => watching.storing_to(store, tx),
        None => watching,
    };
    let swap = HotSwap::new(
        &gpu.device,
        &gpu.queue,
        live,
        // **The engine's own default rather than a number written here**: a
        // budget transcribed into this file would be a second answer to *how
        // long may a frame take* the day the engine's moves (P-0179 on a
        // number that is not even the mock's). It is 20 ms, which is 60 Hz
        // with room, and it is what makes a rollback reachable in this program
        // at all — `HotSwap::fixed` judged against infinity, so no candidate
        // could ever be thrown out for cost.
        DEFAULT_BUDGET_MS,
        Box::new(watching),
    );
    (swap, Aiming { aim, at })
}

impl Engine {
    /// **Sized from the arrangement rather than from the window**, by the same
    /// two calls the frame aims with — see [`aims`]. The window this opens at
    /// gives the picture and deck A's cell their first rectangles, so no frame
    /// has to correct a guess and there is no second derivation here to drift
    /// from the one in [`Engine::aim`].
    fn new(
        gpu: &Gpu,
        renderer: &mut egui_wgpu::Renderer,
        slots: &[Sources],
        layout: &karakuri_layout::Layout,
        scale: f32,
        // **Where every watcher puts what it builds, and where it reports it.**
        // `None` is a harness with no store to write into, which is every test
        // under `mod gpu` below: nothing there saves, and a run that created a
        // store to draw four cells would be the side effect
        // `karakuri_environment::scratch` refuses for a `--render`.
        stored: Option<(std::sync::Arc<Store>, std::sync::mpsc::Sender<watch::Built>)>,
    ) -> Engine {
        assert!(
            slots.len() == SLOTS,
            "a deck of {SLOTS} slots was handed {} pairs to run from",
            slots.len()
        );
        // **Parsed once and built [`SLOTS`] times, and that is a fact rather
        // than an assumption now.** Every entry in `slots` is a copy of the
        // one pair the command line settled ([`working_copies`]), so the four
        // files hold the same bytes at startup and one `Checked` is the same
        // answer four times. What is *not* the same is the path each slot's
        // watcher polls, which is the whole of what per-slot copies buy and is
        // read off `slots[slot]` in the loop below.
        // **Compiled through the sort every other surface compiles through**,
        // which is what this used to do by hand and is the whole of what a save
        // needed: `checked` gave back a `Checked` and dropped the bytes it read,
        // and a node's address is a function of exactly those bytes
        // ([`karakuri_environment::compile::Placed::source`]). Re-reading the
        // path later to hash it is the defect that function's own doc is
        // written against — between here and the first frame sit a device, four
        // `Set::build`s and, now, an MCP server.
        //
        // **Once, for slot 0, and used by all four.** See [`Engine::placed`].
        let (material, placed) = karakuri_environment::compile::sort_slot(
            ON_AIR,
            &karakuri_environment::compile::Named::bare(&slots[ON_AIR].l1),
            &[karakuri_environment::compile::Named::bare(
                &slots[ON_AIR].l4,
            )],
        );
        let l1 = material
            .l1s
            .first()
            .expect("the launch pair declares a geometry")
            .clone();
        let l4 = material
            .l4s
            .first()
            .expect("the launch pair declares a renderer")
            .clone();
        let capacity = capacity_of(&l1);
        // **The same material in every slot, at its own salt and in its own
        // file.** A slot cannot hold *nothing*: `HotSwap::new` takes a live
        // `Set` and `Deck::new` takes one `HotSwap` per slot, so an empty slot
        // is not a state this engine has and the nearest thing to it is a slot
        // holding material nobody has asked for. What this program has to give
        // them is one pair — [`Sources`] is the whole command line — so each
        // gets it at its own salt ([`slot_salt`]): four slots of one procedure
        // at four seeds are four simulations, and four slots at one seed would
        // be one picture drawn four times, which is not a mixer either.
        // Building three of them from other `.kir` files would be this program
        // choosing material for the operator, which is the library's job and
        // not a constructor's
        // ([ADR-0228](../../../docs/adr/0228-a-library-load-re-points-the-slots-source-and-never-installs-a-set.md)).
        //
        // **The salt is no longer the only difference, and that is the fix.**
        // Each slot runs from its own copy of that pair ([`working_copies`]),
        // so the four are the same *material* and four different *files* — an
        // edit reaches the deck whose file it is.
        let built = |salt| {
            Set::build(&gpu.device, &gpu.queue, &l1, &l4, capacity, salt)
                .expect("the pair builds a Set")
        };
        // **A watcher per slot, over that slot's own two files**, which is
        // `karakuri-cli`'s wiring and not a second one — one `HotSwap::new`
        // over a `watch::Watch`, at the engine's own default budget.
        //
        // **`slots[slot]` and not one pair repeated**, which is the sentence
        // this comment used to be the other way round. It said every slot was
        // spelled with the same pair and quoted `watch`'s *"Two slots given
        // the same files both rebuild, which is right: the same edit reached
        // both of them"* — true of that module, and the wrong thing for this
        // program to be doing, because it made one save rebuild four slots and
        // fill the Staging lane with three rows that can never leave (a parked
        // slot's trial is frozen, so it reaches no verdict). Each watcher now
        // polls the copy made for its own slot, and no watcher can see
        // another's file at all, which is what `watch`'s *"a slot is the unit
        // that gets replaced"* asks for.
        //
        // **This is the Staging lane's producer**, and it is the whole of what
        // it took. `HotSwap::fixed` keeps a `Receiver` whose `Sender` was
        // dropped at construction, so nothing is ever installed and no
        // `swap::Event` of any variant is emitted — which is why the lane drew
        // its empty state and could reach no other. Nothing about the frame
        // path changed: `install_if_ready` polls the same channel with
        // `try_recv` either way, and everything a rebuild costs — the file
        // read, the four validation stages, the compile and `Set::build` — is
        // on the worker thread this spawns (P-0001).
        //
        // **In slot order, and the loop is the whole of what four slots
        // took**: a `Vec` of `HotSwap` is what `Deck::new` has always taken,
        // and `Engine::aimed` is documented as being in the same order the
        // strips and the preview cells are.
        let mut swaps = Vec::with_capacity(SLOTS);
        let mut aimed = Vec::with_capacity(SLOTS);
        for (slot, running) in slots.iter().enumerate().take(SLOTS) {
            let salt = slot_salt(slot);
            let (swap, aim) = watched(
                gpu,
                running,
                built(salt),
                slot,
                salt,
                stored
                    .as_ref()
                    .map(|(store, tx)| (std::sync::Arc::clone(store), tx.clone())),
            );
            swaps.push(swap);
            aimed.push(aim);
        }
        let mut deck = Deck::new(&gpu.device, swaps, CANVAS.0, CANVAS.1);
        // **Every slot but deck A rests at `Allocated`, which is what a
        // channel nobody has asked anything of is.**
        //
        // `Deck::new` brings every slot up Live, and that is right for a deck
        // built to *play* what is in it — a deck of one is then a bare Set,
        // bit for bit (ADR-0038). This deck is built **full** rather than
        // built to play four, so leaving them Live would put three
        // simulations nobody asked for on the render thread and three layers
        // nobody asked for into the fold: the reading [`Costs::say`] prints
        // would stop being one Set a frame, and every slot comes up under
        // `Blend::Add` at unity, so what the Program bay drew would be four
        // simulations summed — the same material at four times its exposure,
        // which is a mixer set wrong rather than a mixer.
        //
        // `Residency::Allocated` is the state the engine already has for this,
        // rather than one invented here — *compiled, buffers held, not
        // stepping, keeps its `t`* — and `deck::Frame::render` reads the
        // effective residency into the composite's `live` flag, so an
        // allocated slot contributes nothing to the mix, draws nothing, and
        // steps nothing. The strip reads ALLOC, the cell reads `off`, and the
        // slot costs its buffers and no frame time. That is
        // [P-0019](../../../docs/principles/0019-prefer-the-mechanism-that-already-exists.md):
        // *a slot with nothing in it* is a residency this deck already has a
        // word for.
        //
        // **It is the request that is written**, which is the operator's half
        // and the same half [`Engine::ask_to_prime`] writes — see
        // `Deck::set_residency`. So an operator brings a channel up by cycling
        // its tally chip or by loading material into it, and nothing has to
        // undo a decision this constructor made. The governor is told nothing
        // by it either: a slot whose request is Allocated is `Reason::OffAir`,
        // which is *the governor was not asked about this slot*.
        for slot in 0..SLOTS {
            if slot != ON_AIR {
                deck.set_residency(slot, Residency::Allocated);
            }
        }
        // **The meters are on, and that is a decision rather than a default.**
        // Five of the six things a mixer strip shows are settings the deck was
        // told; the meter is the only one that is a *measurement*, so with it
        // off this bay would draw five readouts that never move beside a well
        // that is always empty — which is the scaffolding-that-looks-finished
        // this panel refuses, read from the other side. It costs a pipeline,
        // two buffers and a ring of staging buffers per slot, allocated here
        // and never on the render thread, which is the same terms `Deck::new`
        // above is on; there are [`SLOTS`] slots, so it is four of each. The
        // three that are not Live report no level — `Deck::level` is `None`
        // for a slot that is not being drawn — which is the meter saying what
        // it measured rather than a strip with a gap in it.
        deck.enable_meters(&gpu.device);
        let present = Present::new(&gpu.device, PICTURE_FORMAT, CANVAS.0, CANVAS.1);
        let (picture_at, preview_ats) = aims(layout, present.size());
        let picture = Presented::new(gpu, renderer, "program view", picture_at, scale);
        let previews = [
            Presented::new(
                gpu,
                renderer,
                "deck A preview",
                preview_ats.and_then(|c| c.first().copied()),
                scale,
            ),
            Presented::new(
                gpu,
                renderer,
                "deck B preview",
                preview_ats.and_then(|c| c.get(1).copied()),
                scale,
            ),
            Presented::new(
                gpu,
                renderer,
                "deck C preview",
                preview_ats.and_then(|c| c.get(2).copied()),
                scale,
            ),
            Presented::new(
                gpu,
                renderer,
                "deck D preview",
                preview_ats.and_then(|c| c.get(3).copied()),
                scale,
            ),
        ];
        let slot_bind_groups = std::array::from_fn(|slot| {
            deck.slot_view(slot)
                .map(|view| present.create_bind_group_for(&gpu.device, view))
        });
        Engine {
            deck,
            capacity,
            present,
            picture,
            previews,
            slot_bind_groups,
            look: LOOK,
            freed: 0,
            aimed,
            placed,
        }
    }

    /// **Ask deck B to warm up, and let the budget answer.** The one governor
    /// pass this program makes, taken at startup where the stall it costs is
    /// free, and the whole of why a strip on this panel can read one residency
    /// and have been asked for another.
    ///
    /// **The other two slots are not in this, and that is the change.** Deck B
    /// used to be the only other slot there was, so *the deck has a second
    /// slot* and *the panel can show a park* were one sentence; they are two
    /// now. C and D rest at `Residency::Allocated` — [`Engine::new`] says why
    /// — were asked for nothing, and come back from the pass as
    /// `Reason::OffAir`, which is the governor reporting that it was not asked
    /// about them. They cost the arithmetic below nothing: `committed_ms` is
    /// the sum over **Live** slots and deck A is the only one, so this sets
    /// the same budget it set with two slots, off the same measurement, for
    /// the same reason.
    ///
    /// **It is `karakuri-cli`'s order rather than a second one**: measure every
    /// slot before anything is decided about any of them, ask through
    /// [`Deck::set_residency`], and call [`Deck::govern`], which is the only
    /// thing in the engine that writes an *effective* residency. `set_residency`
    /// writes the request **and grants it**, so a harness that never governs
    /// has a deck whose two residencies agree on every slot and every frame —
    /// which is what this file was, and is why the roll ADR-0190 drew was
    /// tested and unreachable. The report comes back whole for the same reason
    /// `karakuri-cli`'s `report_governing` prints one: nothing is printed in
    /// the engine, so what an operator reads and what a test asserts are the
    /// same values.
    ///
    /// # The budget is what moves, and it is moved from what was measured
    ///
    /// A deck starts on `governor::DEFAULT_COMPUTE_BUDGET_MS` — one 60 Hz
    /// frame of measured per-Set cost — and what one of these Sets measures at
    /// is this machine's business rather than anything this file can know. So
    /// the budget is set **from the measurement that was just taken**: what
    /// deck A is already committed to, plus half of the least the slowest
    /// priming rate could ask for. [`SLOWEST_PRIME_ONE_IN`] is the slowest the
    /// governor is willing to call priming and a slowed slot costs
    /// `cost / n` amortised, so a headroom under `cost / SLOWEST_PRIME_ONE_IN`
    /// cannot take this Set at **any** rate — the refusal is arithmetic on
    /// every machine rather than on the ones where the numbers happen to come
    /// out.
    ///
    /// **A number computed from the measurement rather than a constant**,
    /// because a constant is the fixture the product cannot produce
    /// ([P-0047](../../../docs/principles/0047-a-fixture-the-product-can-rewrite-is-not-a-fixture.md)
    /// read from the other side): a budget typed in here parks the request on
    /// this machine and admits it on a faster one, and a *measurement* typed in
    /// — `HotSwap::set_measured_cost` is public and would take one — is this
    /// file writing down the number the probe exists to take.
    ///
    /// **Nothing here writes a residency, a strip or a park.** The deck is
    /// asked and the governor answers; [`mixer`] reads both residencies back
    /// off the deck the way it reads the gain, and `view::Strip::pending`
    /// derives the disagreement. `Deck::is_parked` is not called in this file
    /// at all outside `mod gpu`.
    ///
    /// Where a measurement is missing the budget is left where it was, and the
    /// governor parks the request anyway for a different and more serious
    /// reason — an unmeasured Live slot means the committed cost is unknown,
    /// which suspends priming wholesale. The caller prints the reason it got
    /// rather than the one this comment expects.
    fn ask_to_prime(&mut self, gpu: &Gpu) -> Report {
        // **Before the first frame, and this is the only place it can be.**
        // Measuring means stepping and ends in a rewind, so `measure_slots`
        // skips any slot whose Set has already run — a deck measured late
        // stays unbudgetable rather than losing what it has simulated.
        self.deck.measure_slots(&gpu.device, &gpu.queue);
        self.deck.set_residency(ASKED_TO_PRIME, Residency::Priming);
        if let (Some(committed), Some(warming)) = (
            self.deck.slot(ON_AIR).measured_cost(),
            self.deck.slot(ASKED_TO_PRIME).measured_cost(),
        ) {
            self.deck.set_compute_budget_ms(
                committed.ms + warming.ms / (2.0 * SLOWEST_PRIME_ONE_IN as f32),
            );
        }
        self.deck.govern()
    }

    /// **Aim every sink at its own rectangle, and hand back what the console
    /// should draw in each** — the picture, and one entry per preview cell.
    ///
    /// # A cell is aimed because there is a slot behind it, and residency has
    /// nothing to do with it
    ///
    /// This read `Deck::preview` once and aimed the one preview sink at the
    /// cell of the deck the output was auditioning: the sinks all took the same
    /// composited frame, so a sink left in deck A's cell would have drawn deck
    /// C's material under the letter `A` the moment somebody auditioned C.
    /// ADR-0240 retired the audition — the picture is the master mix and every
    /// cell is its own deck's monitor — and the sinks stopped taking the
    /// composited frame: each cell is drawn from `Deck::slot_view` for the slot
    /// it is lettered for, which is a texture that cannot be of the wrong deck.
    ///
    /// **Then it gated the aim on `Residency::Live`, and that was the defect
    /// this pass removes.** The reason given was that an off-air slot "is not
    /// stepping and has nothing new in its view", which was true only because
    /// the engine refused to draw one. It is the exact case
    /// [P-0080](../../../docs/principles/0080-an-operator-can-see-a-slots-own-material-without-putting-it-on-air.md)
    /// exists for: an operator decides whether to put a candidate on air by
    /// watching its cell, and a cell that is dark until the candidate is
    /// already on air answers the question after it stops being asked. The deck
    /// draws every slot into its own target on every frame now, so there is
    /// something new in every view, every frame.
    ///
    /// **What is left to decide is whether there is a slot at all**, and that
    /// is `slot_bind_groups[slot]`: `Deck::slot_view` is `None` past
    /// `slot_count`, so a deck of fewer than [`DECKS`] slots leaves the surplus
    /// cells with nothing to sample. **The aim asks the same question the draw
    /// asks**, so the two cannot disagree — a cell aimed but not drawn would be
    /// a texture from an earlier frame held under a letter, and a cell drawn
    /// but not aimed is a pass into nothing.
    fn aim(
        &mut self,
        gpu: &Gpu,
        renderer: &mut egui_wgpu::Renderer,
        layout: &karakuri_layout::Layout,
        scale: f32,
    ) -> (Option<Picture>, [Option<Picture>; DECKS]) {
        let (picture_at, preview_ats) = aims(layout, self.present.size());
        let picture = self
            .picture
            .aim(gpu, renderer, picture_at, scale, &mut self.freed);
        // Which cells have a slot behind them, read once for the frame: the
        // loop below both aims and reports off the same answer, and the draw in
        // `compose` reads the same `slot_bind_groups`.
        let behind: [bool; DECKS] =
            std::array::from_fn(|slot| self.slot_bind_groups[slot].is_some());
        let mut previews = [None; DECKS];
        let freed = &mut self.freed;
        for (slot, (sink, out)) in self
            .previews
            .iter_mut()
            .zip(previews.iter_mut())
            .enumerate()
        {
            let at = match behind[slot] {
                true => preview_ats.and_then(|cells| cells.get(slot).copied()),
                false => None,
            };
            let pic = sink.aim(gpu, renderer, at, scale, freed);
            if behind[slot] {
                *out = pic;
            }
        }
        (picture, previews)
    }
}

/// Which rectangle each of the engine's sinks is sized from and drawn into.
fn aims(
    layout: &karakuri_layout::Layout,
    canvas: (u32, u32),
) -> (Option<egui::Rect>, Option<[egui::Rect; DECKS]>) {
    (picture_rect(layout, canvas), preview_rects(layout, canvas))
}

/// **One tone-mapping pass per deck preview cell, off that slot's own target,
/// whatever the slot's residency.**
///
/// The engine drew every slot into its own target a moment before this — Live
/// ones stepped and drawn, off-air ones drawn and not stepped — so this reads
/// [`DECKS`] fresh images and never the composite. Through the same [`Present`]
/// the picture goes through, so a cell is the material under the transfer curve
/// the room gets, with no fader on it, because a fader is an edge property
/// applied in the mix and a slot's target is upstream of the mix. That is
/// [P-0080](../../../docs/principles/0080-an-operator-can-see-a-slots-own-material-without-putting-it-on-air.md)'s
/// three clauses in one loop.
///
/// **Gated on `aimed` and on there being a slot to sample, and on nothing
/// else.** Residency was the third gate and is not: a cell that goes dark when
/// its deck goes off air is dark at exactly the moment an operator is deciding
/// whether to bring it back. [`Engine::aim`] asks the same two questions, so a
/// cell cannot be aimed and not drawn.
///
/// A function rather than the body of the closure it is called from, for
/// [`live`]'s reason: `window_event` cannot be called from a test, so the part
/// worth asserting is lifted to where one can reach it — see
/// `every_cell_with_a_slot_behind_it_is_aimed_whatever_its_residency`.
fn monitor(
    present: &Present,
    previews: &mut [Presented; DECKS],
    bind_groups: &[Option<wgpu::BindGroup>; DECKS],
    encoder: &mut wgpu::CommandEncoder,
) {
    for (slot, pres) in previews.iter_mut().enumerate() {
        if pres.aimed {
            if let Some(bg) = &bind_groups[slot] {
                present.draw_with_bind_group(encoder, bg, &pres.target, pres.size);
            }
        }
    }
}

/// **Is anything making texels this frame?** — which is the whole of what
/// decides whether the loop asks for another frame.
///
/// **The rule, rather than the expression: anything that makes texels this
/// frame keeps the loop awake, and the list is closed.** Every sink the engine
/// draws into is read here — the picture and all [`DECKS`] preview cells — so
/// a fifth sink added later and not added to this is the same bug again, and
/// it is the bug this file has already shipped once: `live` was the picture
/// alone, so folding the picture away left deck A auditioning under it while
/// the loop stopped asking for frames. It fails in whichever direction the
/// mistake is made — a window that goes on drawing what nobody asked for, or a
/// panel that keeps changing while the loop sleeps.
///
/// A function rather than an expression in the frame path for the reason
/// [`Readout::pointer`] is a method: `window_event` cannot be called from a
/// test, so the part worth asserting is lifted out to where a test can reach
/// it — see `anything_that_makes_texels_keeps_the_loop_awake`.
fn live(view: &View) -> bool {
    view.picture.is_some() || view.previews.iter().any(Option::is_some)
}

/// **What the transport row reads this frame**, out of the two things in this
/// file that know: the deck's oscillator, and what the last frame cost.
///
/// `Transport` here is `karakuri_console::view::Transport` — the console's row
/// of readouts — and not `karakuri_engine::transport::Transport`, which is a
/// slot's sync mode and is a different thing with the same word on it. This
/// takes a `&Deck` because it is on the side of the seam that is allowed one;
/// what crosses into the console is six numbers (ADR-0156).
///
/// # Where each number comes from, and that nothing is measured twice
///
/// - **The tempo, the position and the grid.** `Deck::signals` is the
///   session's one oscillator — the same one every binding reads — and
///   `Oscillator::bpm` and `Oscillator::beats` are its tempo and its musical
///   position. `beats` is unbounded and monotone, so which dot is lit and
///   which bar it is are arithmetic on it and the console does that
///   arithmetic. The deck advances it inside `render`, one step a frame
///   (`STEPS_A_FRAME`), so this reads the position as of the end of the last
///   frame.
/// - **The frame's cost.** `Cost::whole` — the same three fields the reading
///   sums under *"the whole frame is a median"*, for the frame just drawn.
///   Nothing is timed twice: `Costs::push` kept the last `Cost` and this
///   divides nothing.
/// - **The rate.** `Costs::rate_now`, which is the reading's own `rate`
///   asked before its deadline rather than at it.
/// - **How many beats a bar has.** `karakuri_signal::oscillator::BEATS_PER_BAR`, which is
///   where the deck's own grid gets it, and which says of itself that it is
///   provisional until the IR format carries a time signature. Asked rather
///   than transcribed, so that the day it stops being 4 the beat grid stops
///   being four dots.
///
/// `None` before the first frame has been drawn, which is one frame of a run:
/// there is no frame cost yet, and a row that made one up would be inventing
/// exactly the reading this whole seam exists to refuse.
///
/// **The rate is `None` unless something is live**, and that is not caution
/// either. With nothing making texels this loop stops asking for frames, so
/// the last rate it measured would sit in the row describing a window that has
/// stopped drawing — the one number here that goes stale by standing still.
/// The frame time beside it does not: the last frame did cost that, whenever
/// it was.
fn transport(
    deck: &Deck,
    costs: &Costs,
    budget_ms: Option<f32>,
    live: bool,
) -> Option<view::Transport> {
    let last = costs.last?;
    let grid = deck.signals().oscillator();
    Some(view::Transport {
        bpm: grid.bpm(),
        beats: grid.beats(),
        beats_per_bar: karakuri_signal::oscillator::BEATS_PER_BAR,
        fps: live
            .then(|| costs.rate_now())
            .flatten()
            .map(|rate| rate as f32),
        frame_ms: ms(last.whole()) as f32,
        budget_ms,
    })
}

/// **What the Library bay lists**: the name of every Set the store holds.
///
/// # Read once, and by the side of the seam that may read a disk
///
/// Called from `resumed`, before the first frame. A listing is a directory
/// read and a frame path does not do those
/// ([P-0072](../../../docs/principles/0072-a-still-panel-costs-nothing-and-what-moves-declares-its-price.md)),
/// and nothing in `karakuri-console`'s `src/` could do it anyway: opening a
/// store is `karakuri-store`'s and the crate depends on neither it nor the
/// engine (ADR-0156). What crosses into the console is a list of names.
///
/// The cost of reading it once is that a Set saved while this window is up
/// does not appear in the bay until the next run. That is this program's
/// limitation and not the console's — the field is rewritable per frame like
/// every other one — and closing it wants a reason to re-read rather than a
/// timer, which is a decision and not this pass's.
///
/// # A missing store is listed as nothing, and is not created
///
/// [`karakuri_store::store::Store::open`] *"establishes the store layout under
/// `root`, creating any directories that do not exist yet"*, which is the
/// right thing for a program that is about to write one and the wrong thing
/// for one that only wants to read. A program that listed a library by first
/// making one would change the directory it was run in, so the root is
/// required to be there already.
///
/// Either way the answer is a list, and an empty one is a bay with nothing in
/// it — which is what `view::library` draws for it, and is honest: a store
/// this run could not read holds nothing it can name.
fn library(root: &std::path::Path) -> Vec<String> {
    if !root.is_dir() {
        println!(
            "library: no store at {}, so the bay lists nothing",
            root.display()
        );
        return Vec::new();
    }
    let sets = Store::open(root).and_then(|store| store.list_sets());
    match sets {
        Ok(sets) => sets.into_iter().map(|entry| entry.id).collect(),
        Err(e) => {
            // Said rather than swallowed, for the reason every other failure
            // in this file is said: a bay that is empty because the store
            // could not be read looks exactly like a bay that is empty
            // because the store is.
            println!("library: {} could not be listed: {e}", root.display());
            Vec::new()
        }
    }
}

/// **One row of the `presets` scope**: the word the bay draws and the file
/// behind it.
///
/// Two fields because the bay lists **names** and a load needs a **path**:
/// what crosses into the console is a `String` per row (`view::View::library`),
/// and what this program has to be able to find again on the press is the file
/// that row came off.
struct Preset {
    /// What the row reads, which is the file's own name without its
    /// extension. **Not read out of the file**: a listing that opened
    /// twenty-one files to draw twenty-one rows would be a directory read
    /// doing a file read's work, and the id a take-in files the Set under is
    /// the one *inside* the file anyway — read there, on the press, by
    /// [`taking_in`].
    id: String,
    path: std::path::PathBuf,
}

/// **Every Set the preset library offers**, which is the `.kset` files in the
/// root this run resolved.
///
/// # One call, and the reading is not this program's
///
/// The listing is `karakuri_environment::places`', beside the resolution that
/// answers *where* the presets are: what a `.kset` is and which directory
/// holds them is that module's business, and a second program wanting the same
/// list must not read the same directory a second way. So this is the one
/// place in this program that knows a preset library can be listed at all, and
/// it knows nothing about how — the shape of a row, the order they come in,
/// and what a name the layout does not claim does are all answered there.
///
/// # What it lists, and why not the `.kir` files beside them
///
/// `console.html`'s *A folder scope reads Sets, and a bundle is not a third
/// thing* settles it: *"A directory of `.kir` files is a directory of parts,
/// and a library lists what you can put on a deck."* A `.kir` is one node's
/// source addressed by its content and nothing in the vocabulary takes one, so
/// the parts are not rows — they are what the rows *name*.
///
/// **A root with nothing in it is a library nobody has filled**, and it is not
/// a failure: the scope lists nothing and the sentence about it is
/// [`why_nothing`]'s. A directory that will not open is said out loud, for
/// [`library`]'s reason one scope along — a scope empty because a directory
/// could not be read looks exactly like one that is empty.
fn presets_listing(presets: Option<&karakuri_environment::places::Presets>) -> Vec<Preset> {
    let Some(presets) = presets else {
        return Vec::new();
    };
    match presets.list_sets() {
        Ok(sets) => sets
            .into_iter()
            .map(|set| Preset {
                id: set.id,
                path: set.file,
            })
            .collect(),
        // **Said out loud and then empty**, which is the same shape the store
        // side takes one scope along: a root that will not open looks exactly
        // like a root nobody has filled, and the difference has to be spoken
        // or it is not there. The sentence is `places`' own — it names the
        // path and how that path was arrived at — so an operator who typed
        // `--presets` reads something different from one whose checkout moved.
        Err(why) => {
            println!("presets: {why}");
            Vec::new()
        }
    }
}

/// **Why the scope that is marked lists nothing**, in the words that say which
/// kind of nothing it is — and among these four chips there are two kinds.
///
/// Two of them are empty as *data*: a store nobody has saved into and a preset
/// root nobody has filled are libraries with nothing in them, which
/// `console.html` says outright — *"An empty tier is a library nobody has
/// filled rather than something gone wrong."* Fill either and the rows appear
/// with nothing else changing.
///
/// **The other two are empty as *machinery*, and they are not the same
/// machinery**, which is why this says which:
///
/// - **A favourite is a fact nothing in this workspace keeps.** No field on a
///   Set listing, no record that carries one, no operation that names one —
///   `console.html`'s *What keeps a favourite, and where it does not travel*
///   decides where the value would live and leaves nothing to read. Nothing
///   here writes one: a store invented for it would be the specification
///   written backwards, which that page says in as many words.
/// - **A folder waits on an operation.** *"No operation in the vocabulary can
///   ask a folder for its listing"* — `Operation::ListSets` carries what a Set
///   holds and which layer it uses, and has nowhere at all to put a directory.
///   So the chip is drawn and the asking is owed by that row of
///   `docs/manual/operations.html` rather than by this file.
///
/// It is said out loud on the step and again on a press, because a scope that
/// went quiet and a scope that is empty are the same experience — which is the
/// rule every other refusal in this file is written to.
fn why_nothing(scope: Scope) -> &'static str {
    match scope {
        Scope::Favourites => {
            "nothing in this workspace keeps a favourite — no field on a Set listing, no \
             record, no operation — so this scope is empty because there is nowhere for a \
             star to be rather than because nothing is starred"
        }
        Scope::MySets => {
            "this store holds no Sets yet, which is a library nobody has filled: `k` keeps \
             what a deck is playing, and loading a preset leaves one here too"
        }
        Scope::Presets => {
            "this run found no preset library, or the one it found holds no `.kset` file — \
             `--presets DIR` is what names one, and a directory of `.kir` parts is not a \
             library"
        }
        Scope::Folder => {
            "no operation in the vocabulary can ask a directory for its listing — \
             `ListSets` carries what a Set holds and has nowhere to put a folder — so this \
             chip is drawn and the asking is owed by the operations page rather than by \
             this program"
        }
    }
}

/// **The rows the Library bay lists for the scope that is marked**, written
/// into the view, and the sentence to print about it.
///
/// # One function, and it is what a scope *is* on this program's side
///
/// The console draws a row of chips and marks one of them; **which listing
/// belongs under that mark is this side's answer**, because every one of the
/// four is something outside this process — a store, a told directory, a
/// filter over the first, a directory somebody names during the run — and
/// `karakuri-console` takes none of them (ADR-0156). So the seam is a `Vec` of
/// names, and this is the one place it is filled.
///
/// **On the press that changed the scope and at startup, never on a frame.** A
/// listing is a directory read (P-0072), which is the same rule [`library`]
/// states one scope down and the reason this is not called from the frame
/// handler.
///
/// Two of the four answer with rows and two answer with nothing —
/// [`why_nothing`] is where that is argued, and it is one function so that a
/// scope which stops being empty stops being empty in one place.
fn listing(
    view: &mut View,
    store: &std::path::Path,
    presets: Option<&karakuri_environment::places::Presets>,
) -> String {
    let Some(scope) = view.scope() else {
        return String::from(
            "  library: this console was handed no scopes, so there is no library to list",
        );
    };
    view.library = match scope {
        Scope::MySets => library(store),
        Scope::Presets => presets_listing(presets)
            .into_iter()
            .map(|preset| preset.id)
            .collect(),
        // **Drawn and answered with nothing**, and the two are not the same
        // nothing — see [`why_nothing`], which is where each of them says
        // which it is.
        Scope::Favourites | Scope::Folder => Vec::new(),
    };
    match view.library.len() {
        0 => format!(
            "  library: `{}` lists nothing — {}",
            scope.name(),
            why_nothing(scope)
        ),
        listed => format!(
            "  library: `{}` lists {listed} Set{}",
            scope.name(),
            match listed {
                1 => "",
                _ => "s",
            }
        ),
    }
}

/// **What a take-in did**: the file it read, the id that file filed itself
/// under, and the sentence `setfile::unbundle` reported.
///
/// **Three fields because the press has three callers for them and each is a
/// different question.** The `said` is what the operator reads. The `id` is
/// what the load that follows names, and it is the file's own rather than the
/// row's word. The `file` is what the *operation* names —
/// `Operation::TransferSet`'s `SetTransfer::Take { file }` carries a path,
/// *"because a file is what the only existing route takes"* — so it is
/// returned rather than re-derived: the listing is asked once, on the press,
/// and asking it a second time to name what was already taken in would be two
/// answers to *which file was this* with a directory read between them.
#[derive(Debug)]
struct TakenIn {
    file: std::path::PathBuf,
    id: String,
    said: String,
}

/// **A preset row, taken into this store**, and the id it landed under.
///
/// # Taking it in is not a second operation, and it is what gives it a name
///
/// `docs/manual/operations.html`'s *Send a Set to somebody, and take one in*:
/// packaging is *"one operation at two moments — ahead of time when you are
/// sending, and at the press when you are not"*, so opening a preset **is**
/// that row performed at the second of them. `console.html` reaches it from
/// the other side: *"loading a preset is a packaging step, and a packaging step
/// writes into the store: `my sets` gains a row you did not make."* That is
/// what this does, and it is why a preset row is one press rather than two —
/// the take-in is what gives the Set the id the load needs.
///
/// # It is `karakuri-cli`'s own route and not a second one
///
/// `taken_in_file` resolves a `.kset` with `setfile::bundle_authored` — which
/// is `setfile::resolve` behind its wall, and then the inlining — and hands the
/// result to `setfile::unbundle`. **Resolved *and* inlined rather than resolved
/// alone**, for that function's stated reason: `unbundle` writes a metadata
/// card for each source the lines carry, and handing it resolved lines with
/// nothing inlined would file the Set and leave every artifact cardless. Two
/// routes into one store that reach two different stores is the disagreement a
/// second spelling always is.
///
/// **The wall is `resolve`'s and not this file's**: a part named from outside
/// the file's own directory is refused, by path, because *"a Set somebody
/// handed you is not a way of asking this machine for its files"* (ADR-0229).
/// Nothing here loosens it and nothing here repeats it.
///
/// # An id this store already holds is refused, and the refusal is not written
/// # here
///
/// `setfile::unbundle` asks what the store holds before it writes a byte and
/// refuses an id that is taken — *"the id came from the file rather than from
/// you"* — and that sentence is the one the operator gets. A second check here
/// would be a second answer to *may this be overwritten*, and the two would
/// disagree the day one of them moved. What this adds is which of the two acts
/// failed: nothing was taken in, so nothing was loaded, and the deck is exactly
/// as it was.
///
/// # The id is the file's own
///
/// Read off the `set` record in the resolved lines rather than taken from the
/// row's word, because that is the id `unbundle` files it under and the id
/// `my sets` will list. They are the same word in `examples/`, and a preset
/// whose file says otherwise would otherwise be loaded by a name the store does
/// not hold.
fn taking_in(
    root: &std::path::Path,
    presets: Option<&karakuri_environment::places::Presets>,
    row: &str,
) -> Result<TakenIn, String> {
    // **Asked again rather than kept**, which is [`listing`]'s shape: the rows
    // crossed into the console as words, and the file behind a word is found
    // by asking the library again on the press. A second copy of the listing
    // held on this side is a copy that goes on naming a file that has moved.
    let found = presets_listing(presets)
        .into_iter()
        .find(|preset| preset.id == row)
        .ok_or_else(|| {
            format!(
                "the preset library has no `{row}{}` in it any more",
                karakuri_environment::setfile::AUTHORING_SUFFIX
            )
        })?;
    let store = Store::open(root).map_err(|e| format!("store `{}`: {e}", root.display()))?;
    let lines = karakuri_environment::setfile::bundle_authored(&store, &found.path)?;
    let id = lines
        .iter()
        .find_map(|line| match line.record() {
            Record::Set { id, .. } => Some(id.clone()),
            _ => None,
        })
        .ok_or_else(|| {
            format!(
                "`{}` carries no `set` record, so it names no id to file itself under",
                found.path.display()
            )
        })?;
    let said = karakuri_environment::setfile::unbundle(&store, &lines)?;
    Ok(TakenIn {
        file: found.path,
        id,
        said,
    })
}

/// **The two rows of the vocabulary one press on a `presets` row performs**,
/// in the order they happen.
///
/// # Two operations because they are two rows of the page, and one press
///
/// `docs/manual/operations.html`'s *Send a Set to somebody, and take one in*:
/// packaging is *"one operation at two moments — ahead of time when you are
/// sending, and at the press when you are not"*, so *"loading a Set out of
/// presets or out of a folder is this row performed at the second of them"*.
/// The load after it is *Load material into a deck*, which is a different row
/// with a different operation. **One press, two rows** — `console.html` says
/// why it is one press: *"A row here is taken into the store and then loaded,
/// which is one press because taking it in is what gives it a name."*
///
/// So the press emits both. Emitting only the load would be a press that
/// performs two of the page's rows and names one, and the row it dropped would
/// be the one **nothing in this workspace constructs**.
///
/// # Naming what a surface performed is `e`'s rule, not a new one
///
/// The scope key steps the mark itself and emits `Operation::SelectScope`
/// anyway, *"so that the press is recorded as `Silent(Surface)` rather than as
/// nothing at all"*. This is that, one key along: `written` answers
/// `Silent(NoRecord)` for a transfer, nothing in [`App::performed`] performs
/// one, and the emission is the naming.
///
/// # And it is not the key badge
///
/// `key_column::ROWS` maps `l` to *Load material into a deck* alone, and that
/// stays true: what an operator reaches from the keyboard is a load, and the
/// taking-in is what a load off `presets` does on the way. ADR-0213's
/// distinction is between an operator **reaching** an operation and something
/// **happening**, and constructing an operation is neither — which is
/// `panel_column.rs`'s own sentence, *"construction is not reachability, and
/// reachability is the definition."*
///
/// The transfer names the **file**, because that is what
/// `SetTransfer::Take` carries — *"a path because a file is what the only
/// existing route takes"* — and the load names the **id**, which is the file's
/// own `set` record rather than the row's word. They are the two halves of
/// [`TakenIn`] and neither is derived from the other here.
fn preset_press(deck: u8, taken: TakenIn) -> [Operation; 2] {
    [
        Operation::TransferSet {
            transfer: SetTransfer::Take { file: taken.file },
        },
        Operation::LoadSet {
            deck,
            set: taken.id,
        },
    ]
}

/// **Put a library Set on a running deck**, which is the whole of what
/// `Operation::LoadSet` needed and is a re-point rather than an install.
///
/// # It writes files and sends a description, and it builds nothing
///
/// `Deck::install` is the one function that puts a built Set in a slot, and it
/// is *"deliberately not reachable from a key or a surface: a live run changes
/// its material by editing a file and letting the worker build it, which is
/// what the budget watchdog is attached to."* So this does what an operator
/// with an editor does, in one press: it reads the Set out of the store,
/// writes every procedure in it into the scratch, and tells that slot's
/// watcher to look there instead. **Everything after this line is the path a
/// save already takes** — the worker compiles off the render thread, the swap
/// lands at a frame boundary, and the governor judges it for thirty frames
/// against the budget and rolls it back on its own if it costs too much. The
/// library gets all of that for nothing, and no second route into a slot is
/// opened.
///
/// **Nothing here is on the frame path.** A store read, a `setfile::load` that
/// checks every procedure, and up to a handful of small writes — on the press,
/// which is where this file already reads a directory (`arrangement`), and
/// never on a frame (P-0072). The compile is the worker's.
///
/// # The scratch name carries the slot, and it is the directory's rule now
///
/// `scratch::place` writes `<store>/scratch/<name>.kir` and overwrites what is
/// there, so two decks loading two Sets whose procedures happen to share a
/// name would be one file: the second load would move the first deck as well,
/// on its watcher's next poll, and nothing would say why. The name is
/// therefore `A0-drift.kir` — the deck letter, the node's place in the Set,
/// and the procedure's own name — which is unique per slot **and** per node,
/// stays readable in an editor, and says which deck an open file belongs to.
///
/// **It is `scratch::node_name` rather than a `format!` here**, because that
/// argument was never about loading. It is about two decks and one directory,
/// which is every run: every slot is materialised under the same spelling at
/// startup ([`working_copies`]), so a load writes into a directory already
/// laid out this way and a second spelling would be a second answer.
///
/// # What the aim states, and why all of it
///
/// [`watch::Aim`] is `Watch::new`'s argument list less the slot, and every
/// field here is read off the Set file rather than left at this program's
/// startup value — which is the failure each of `Watch`'s own fields is
/// documented against, and which would not show on the load at all. A
/// layering, a fold or a camera left behind is a slot that loads correctly and
/// then comes back as a different picture on the first later save.
///
/// The authorities are the one exception and are empty: `Record::Authority` is
/// deliberately not Set-file state, so a Set carries no grants and a load
/// starts a slot with none — which is what `--load-set` gives one.
///
/// The `Err` is a sentence for the operator. Every way this fails leaves the
/// deck exactly as it was: a store that will not open, a Set that is not
/// there, a procedure in it that no longer checks, a scratch that will not be
/// written, or a worker that has gone.
fn loading(
    root: &std::path::Path,
    slot: usize,
    salt: u32,
    // **The slot's [`Aiming`] and not its sender**, so that where the watcher
    // is pointed is kept with what was sent. A load that sent an aim and left
    // `Aiming::at` behind would leave the next rewiring restating the pair the
    // run launched with — see [`Aiming`].
    aim: &mut Aiming,
    id: &str,
) -> Result<String, String> {
    let store = Store::open(root).map_err(|e| format!("{}: {e}", root.display()))?;
    let loaded = karakuri_environment::setfile::load(&store, id)?;
    // Said rather than swallowed: a note is the reader telling the operator
    // what it did with a file it could only partly honour, and a load that
    // quietly ignored one is a picture nobody can account for.
    for note in &loaded.notes {
        println!("  load: {note}");
    }

    let letter = deck_letter(slot as u8);
    let mut named = Vec::with_capacity(loaded.srcs.len());
    for (at, ((checked, src), name)) in loaded.nodes().zip(loaded.node_names()).enumerate() {
        let path = karakuri_environment::scratch::place(
            root,
            // **The directory's one naming rule, asked rather than spelled
            // again.** It was written out here when this was the only thing in
            // this program that wrote into the scratch; every slot is
            // materialised at startup now, so a second spelling of
            // `A0-drift.kir` would be a second answer to what a scratch file
            // is called — and the two would disagree on the day one of them
            // moved.
            &karakuri_environment::scratch::node_name(slot, at, &checked.name),
            src,
        )?;
        // **The Set file's node name and not the procedure's**, which is
        // `--load-set`'s own pairing: an `edge` in the file resolves against
        // the name the file wrote, and a rebuild that called the node whatever
        // its procedure declares would break the slot on its first save.
        named.push(karakuri_environment::compile::Named { name, path });
    }
    let mut named = named.into_iter();
    let head = named
        .next()
        .ok_or_else(|| format!("`{id}` names no procedure at all"))?;

    // **The file's salts, and a derived one where it recorded none** — the
    // rule `karakuri-cli`'s `salts_for` follows, restated here because that
    // program has no library target. The seed is the file's first salt where
    // it has one and this slot's own where it has not, so a Set that recorded
    // its colours comes back with them and one that did not is salted like the
    // slot it landed in.
    let seed_salt = loaded.salts.first().copied().flatten().unwrap_or(salt);
    let salts: Vec<u32> = (0..loaded.l1s.len())
        .map(|at| {
            loaded
                .salts
                .get(at)
                .copied()
                .flatten()
                .unwrap_or_else(|| karakuri_engine::set::derived_salt(seed_salt, at))
        })
        .collect();

    let nodes = loaded.srcs.len();
    aim.re_point(watch::Aim {
        head,
        rest: named.collect(),
        layering: loaded.layering,
        live: loaded.live,
        // **The first geometry's recorded capacity over all of them**, which
        // is what `--load-set` folds into `--capacity` and is `Watch`'s own
        // shape: one `Option<u32>` for the slot, because a rebuild recompiles
        // the files and each geometry's own declaration is the default. A Set
        // that recorded two different capacities loses the second, which is a
        // limit this program shares with the command line rather than one it
        // invented.
        capacity: loaded.capacities.first().copied().flatten(),
        seed_salt,
        salts,
        // A Set holds a built-in camera whatever its files declare, so
        // `Orbit::default()` where the file recorded none is the camera it
        // would have been built with rather than a value invented here.
        camera: loaded.camera.unwrap_or_default(),
        overrides: loaded.params,
        // Nothing in a Set file publishes a control — `setfile` writes none
        // and reads none — so this is empty for the same reason this program's
        // startup watchers pass an empty list: there is no `--publish` here to
        // fold in either (ADR-0216).
        published: Vec::new(),
        bindings: loaded.bindings,
        edges: loaded.edges,
        authorities: Vec::new(),
    })
    .map_err(|()| {
        format!(
            "deck {letter}'s build worker is gone, so `{id}` cannot be built; \
             what is on that deck keeps running"
        )
    })?;
    Ok(format!(
        "  load: deck {letter} <- `{id}` ({nodes} node{}) -> written into {}/{} and its watcher \
         re-pointed; the worker builds it and the budget judges it",
        match nodes {
            1 => "",
            _ => "s",
        },
        root.display(),
        karakuri_environment::scratch::DIR,
    ))
}

/// **Every arrangement the store holds, by name**, for the pill's menu to
/// list — [`library`] over the fourth directory rather than the first.
///
/// Its two rules are this one's, said again because they are the same two: a
/// store that is not there is listed as nothing and **is not created**, since
/// a program that listed a menu by first making a store would change the
/// directory it was run in; and a store that could not be read says so, since
/// a menu that is empty because the directory would not open looks exactly
/// like one that is empty because nobody has saved.
///
/// **Read when it changes rather than per frame.** Once at startup, and again
/// after a save lands — which is the only thing in this program that adds a
/// name. `Store::list_arrangements` sorts by name, so the menu draws what it
/// is handed and sorts nothing.
fn arrangements(root: &std::path::Path) -> Vec<String> {
    if !root.is_dir() {
        return Vec::new();
    }
    match Store::open(root).and_then(|store| store.list_arrangements()) {
        Ok(filed) => filed.into_iter().map(|entry| entry.name).collect(),
        Err(e) => {
            println!("arrangements: {} could not be listed: {e}", root.display());
            Vec::new()
        }
    }
}

/// **Where a named arrangement is kept and put back**, and the one route in
/// this program that both reads an operation and reaches a disk.
///
/// # Why it is here, in a package neither side depends on
///
/// The panel cannot reach the store. `karakuri-console` dropped
/// `karakuri-store` when this program moved out of it, and the drop was the
/// point — a crate that takes no device and no disk is what ADR-0156 bought,
/// and its manifest now has no entry that could be reached for at all. The
/// store cannot reach the panel either: `karakuri-store`'s `src/` must not
/// name `karakuri-layout`, so it keeps an arrangement as bytes it does not
/// understand, exactly as it keeps `.kir` source
/// ([ADR-0221](../../../docs/adr/0221-an-arrangement-is-named-by-the-operator-and-kept-in-a-fourth-place.md)
/// §4). **So the two halves meet in a third party, and this file is the third
/// party** — the same position it holds for a record, where the vocabulary
/// says what to write and only somebody holding a `Deck` can apply it
/// ([`apply`]).
///
/// # The route, and where each half of it is decided
///
/// - **Saving** is `serde_json::to_vec` of [`Panel::layout`] into
///   `Store::write_arrangement`, which is ADR-0221's own sentence. The format
///   is `karakuri-layout`'s hand-written `Serialize`, so an unbounded maximum
///   goes out as an explicit absence rather than as an infinity JSON cannot
///   spell, and a `NodeId` goes out as the bare number it is.
/// - **Putting one back** is `Store::read_arrangement`, `serde_json` into a
///   [`Layout`], and [`Panel::restore`]. **Neither this file nor the panel
///   checks the arrangement**: `Layout`'s `TryFrom<Wire>` is the one place a
///   file that disagrees with itself is refused rather than repaired
///   (ADR-0158), and a check here would be a second answer to a question that
///   already has one.
///
/// # What it does with each of the three ways it can fail
///
/// **Says it and moves nothing**, and never panics: a panic reachable from an
/// event handler aborts this process rather than unwinding (see the module
/// documentation). The three are a store it could not open or write, a name
/// nothing is filed under, and a file that will not read back — and the third
/// is the one that has to be told apart from the second, because *there is no
/// such arrangement* and *the arrangement you saved is broken* send an
/// operator to two different places.
///
/// **A name nothing is filed under never falls back to the default.**
/// `Store::read_arrangement` answers `StoreError::NoArrangement(name)` and
/// that sentence carries the name, which is the whole reason the store has a
/// fourth error variant rather than reusing `NotFound`: an operator who
/// mistyped a name needs to be told the name, not to watch their console reset
/// (ADR-0221 §2).
///
/// # The store is created by a save and not by a restore
///
/// [`library`] refuses to create one, because *"a program that listed a
/// library by first making one would change the directory it was run in"*, and
/// a restore is a read on exactly those terms. A **save** is the case
/// `Store::open` establishing the layout is right for — it is a program that
/// is about to write — so the two halves below differ, deliberately, and the
/// restore's guard is what keeps `cargo run -p karakuri` in somebody's home
/// directory from leaving a `.karakuri` behind for having asked a question.
///
/// # It answers `None` for every other operation
///
/// Which is what lets it sit on the one path every emitted operation already
/// takes ([`App::performed`]) rather than being a second route into the
/// panel. **The transport row's arrangement pill emits both**, and the
/// manual's two rows say it is the only one of the four surfaces that can: a
/// `panel` badge each and three empty ones, because a name is what a key
/// press, a map line and an unpublished tool each have no way to say. This
/// wiring was written before that control existed — exactly as [`unwritten`]
/// is written for controls that do not exist yet — and the control is what
/// arrived at it.
fn arrangement(
    root: &std::path::Path,
    panel: &mut Panel,
    arr: &mut view::Arrangement,
    operation: &Operation,
) -> Option<String> {
    match operation {
        Operation::SaveArrangement { name } => {
            let (line, kept) = keep_arrangement(root, panel, name);
            // **The name in use moves only when the file did.** A save that
            // was refused leaves the pill saying what it said, because
            // nothing under that name is on the disk — and the listing is
            // re-read only then, since a refusal added no name to it.
            if kept {
                arr.name = Some(name.clone());
                arr.filed = arrangements(root);
            }
            Some(line)
        }
        Operation::RestoreArrangement { name } => {
            let (line, back) = put_arrangement_back(root, panel, name);
            if back {
                arr.name = Some(name.clone());
            }
            Some(line)
        }
        _ => None,
    }
}

/// The running arrangement, filed under `name` — and whether it landed. See
/// [`arrangement`].
fn keep_arrangement(root: &std::path::Path, panel: &Panel, name: &str) -> (String, bool) {
    if let Err(refusal) = checked_name(name) {
        return (refusal, false);
    }
    let bytes = match serde_json::to_vec(panel.layout()) {
        Ok(bytes) => bytes,
        Err(e) => {
            return (
                format!("arrangement: `{name}` was not kept — it did not serialise: {e}"),
                false,
            )
        }
    };
    let wrote = Store::open(root).and_then(|store| store.write_arrangement(name, &bytes));
    match wrote {
        Ok(()) => (
            format!(
                "arrangement: kept as `{name}` — {} bytes at {}",
                bytes.len(),
                root.join("arrangements")
                    .join(format!("{name}.arrangement.json"))
                    .display()
            ),
            true,
        ),
        Err(e) => (format!("arrangement: `{name}` was not kept: {e}"), false),
    }
}

/// **The one place a typed arrangement name is refused**, and the reason it is
/// here rather than in the pill that took the letters.
///
/// `<name>` becomes one path component under `<store>/arrangements/`, and
/// `karakuri-store` says outright that **nothing there checks it**: *"`<name>`
/// becomes one path component and that is the caller's rule to keep"*
/// ([ADR-0221](../../../docs/adr/0221-an-arrangement-is-named-by-the-operator-and-kept-in-a-fourth-place.md)
/// §1, which names letters, digits, `-` and `_`). So `../../elsewhere` is a
/// path, and a path never reaches that call from here.
///
/// **The surface owns the affordance and never the authority**
/// ([P-0076](../../../docs/principles/0076-a-surface-owns-the-affordance-never-the-authority.md)):
/// the pill takes whatever is typed and this is where it meets the wall, so a
/// name refused by a hand and a name refused by anything else that ever
/// reaches this operation meet the same one. A pill that silently dropped the
/// characters it did not like would be a rule an operator could only find by
/// experiment — which is the failure the console page names about a control
/// that quietly declines.
///
/// **It says the same three things `mcp::checked_id` says about a Set id**,
/// which is [P-0061](../../../docs/principles/0061-a-refusal-a-person-can-reach-from-two-surfaces-is-one-sentence.md)
/// as far as it can be kept today and no further: that function is private to
/// `karakuri-environment`'s `mcp` module and its sentences say `id` and
/// `<store>/sets/`, so it cannot be called from here and could not be quoted
/// if it were. **When an arrangement name gets a second surface — a map line,
/// an MCP tool, a `--restore-arrangement` flag — the two collapse into one
/// shared `checked_name`, and this comment is where whoever does it should
/// start.**
fn checked_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err(
            "arrangement: nothing was typed, and an arrangement is filed under a name — \
             the default arrangement is the one that has none"
                .to_owned(),
        );
    }
    match name
        .chars()
        .find(|c| !c.is_ascii_alphanumeric() && *c != '-' && *c != '_')
    {
        Some(bad) => Err(format!(
            "arrangement: `{name}` holds `{bad}`, and an arrangement name is letters, digits, \
             `-` and `_`: it is one path component and it names a file under \
             `<store>/arrangements/`"
        )),
        None => Ok(()),
    }
}

/// The arrangement filed under `name`, into the window the panel already has.
/// See [`arrangement`].
fn put_arrangement_back(root: &std::path::Path, panel: &mut Panel, name: &str) -> (String, bool) {
    if !root.is_dir() {
        return (
            format!(
                "arrangement: no store at {}, so nothing is filed under `{name}` — and the \
                 console has not moved",
                root.display()
            ),
            false,
        );
    }
    let bytes = match Store::open(root).and_then(|store| store.read_arrangement(name)) {
        Ok(bytes) => bytes,
        Err(e) => return (format!("arrangement: `{name}` is not back — {e}"), false),
    };
    let layout: Layout = match serde_json::from_slice(&bytes) {
        Ok(layout) => layout,
        // **Refused whole rather than repaired**, which is the loader's own
        // sentence and not this file's judgement (ADR-0158).
        Err(e) => {
            return (
                format!(
                    "arrangement: `{name}` is not back — the file disagrees with itself and is \
                     refused rather than repaired: {e}"
                ),
                false,
            )
        }
    };
    let viewport = panel.layout().viewport();
    panel.restore(layout);
    (
        format!(
            "arrangement: `{name}` is back, at the {} x {} this window already had",
            viewport.w, viewport.h
        ),
        true,
    )
}

/// **What the mixer strips read this frame**: one per slot the deck has, out
/// of the six things a `Deck` will say about a slot.
///
/// Everything here is reachable from a `Deck` and nothing reaches around one:
/// `slot_count`, `residency`, `gain`, `opacity`, `blend`, `mask` and `level`
/// are its own, and this takes a `&Deck` because it is on the side of the seam
/// that is allowed one — what crosses into the console is a name, a word and
/// four numbers (ADR-0156).
///
/// # Where each one comes from, and the two that are not the deck's
///
/// - **The tally** is `Deck::residency`, which is the **effective** residency
///   the frame loop reads and not `requested_residency`. The governor moves a
///   slot down without anybody asking, and a tally showing the request would
///   be describing a slot that is doing something else.
/// - **And the request beside it** — `Deck::requested_residency`, which is the
///   other half of the same pair. Both are handed over and **neither side
///   computes `Deck::is_parked`**: the engine has the predicate and the
///   console derives its own from the two values (`view::Strip::pending`), so
///   what crosses the seam stays a residency and a residency rather than
///   becoming a bit whose meaning is written down in only one of the two
///   crates. It is the same reading as the four numbers below — the deck says
///   what it is doing, and the surface decides what that looks like.
/// - **The trim and the fader** are `gain` and `opacity`, which are two
///   controls and not one — *"opacity at zero silences under every blend mode,
///   gain at zero does not silence `over`"* — and the bay draws them as two.
/// - **The blend** is [`blend_mode`]: the engine's `Blend` turned into the
///   vocabulary's `BlendMode`, because the chip is a control now and a control
///   has to know which of the three it is on to say what the next one is
///   (ADR-0187). **This is where a fourth engine mode with no operation
///   variant stops the build**, which is the failure worth having — the
///   alternative is a word drawn on a chip no map can ask for.
/// - **The mask** is `Deck::mask(slot).kind()` for the mark, **and its angle
///   beside it** — `view::Strip::mask_angle`, which is read to build the
///   press's operation and drawn nowhere
///   ([ADR-0203](../../../docs/adr/0203-the-mask-chip-carries-the-angle-it-does-not-control.md)).
///   The position and the softness are left behind: the strip's `.mini` says
///   *which shape*, three numbers about that shape are an inspector row, and
///   the two the record needs are read where the record is written rather
///   than carried across this seam.
/// - **The level** is `Deck::level`, which is already `None` for every case
///   where a held reading would be about a different image. Its
///   `frames_behind` is not passed on — `view::Level` is where that argument
///   is written out, and the short of it is that the number means different
///   things on different loops and this loop is a `Fifo` one that never stops
///   asking for frames while anything is live.
/// - **The name** is [`material`], and it is this file's because a `Set` has
///   none. See `view::Strip::name`.
///
/// # Written into the `Vec` the view already holds
///
/// `out` is grown to the deck's slot count and then every field of every strip
/// is written, so nothing a `push` left behind is ever read. The name is the
/// one field that owns anything, and it is rewritten only when it differs —
/// which keeps this off the frame's allocation budget (ADR-0164) rather than
/// putting a `String` per strip on it every frame.
/// **What a scheduled move on this control is taking it to**, or `None` for a
/// control nothing is moving.
///
/// The engine's `Transition` carries the instant it starts, its length in
/// beats and its curve as well, and none of the three crosses this seam: the
/// console has no beat count, so what it could draw out of them is nothing.
/// See `view::Strip::gain_to`.
fn destination(deck: &Deck, slot: usize, control: Control) -> Option<f32> {
    deck.transitions_on(slot)
        .find(|t| t.control() == control)
        .map(|t| t.to())
}

fn mixer(deck: &Deck, names: &[String], out: &mut Vec<view::Strip>) {
    out.truncate(deck.slot_count());
    while out.len() < deck.slot_count() {
        out.push(view::Strip {
            name: String::new(),
            tally: view::Tally::Allocated,
            requested: view::Tally::Allocated,
            gain: 0.0,
            gain_to: None,
            opacity: 0.0,
            opacity_to: None,
            blend: BlendMode::Add,
            mask: view::Mask::None,
            mask_angle: 0.0,
            level: None,
        });
    }
    for (slot, strip) in out.iter_mut().enumerate() {
        // **One name per slot, because a load moves one slot.** It was one
        // name for the whole deck while every slot ran the same pair and
        // nothing could change any of them; a library Set loaded into deck
        // B would then have left every strip reading the pair this program was
        // launched with, which is a readout that is wrong and says nothing
        // (P-0027). Empty for a slot nobody named, which draws no name at all
        // rather than somebody else's.
        let name = names.get(slot).map_or("", String::as_str);
        if strip.name != name {
            strip.name.clear();
            strip.name.push_str(name);
        }
        strip.tally = tally(deck.residency(slot));
        strip.requested = tally(deck.requested_residency(slot));
        strip.gain = deck.gain(slot);
        strip.opacity = deck.opacity(slot);
        // **Where a scheduled move is taking each fader**, which is
        // `Deck::transitions_on` — *"what is moving on this slot, for a status
        // line"* — read for a surface instead. `karakuri-cli`'s line prints
        // `o>0.80` off the same call and for the same reason: with the default
        // quantum a fade is armed up to a bar before it is due, and a control
        // that changes something invisible is indistinguishable from one that
        // is broken.
        //
        // **At most one per control**, because `Deck::schedule` cancels
        // whatever was moving that pair before it pushes — so `find` is the
        // whole answer rather than the first of several.
        strip.gain_to = destination(deck, slot, Control::Gain);
        strip.opacity_to = destination(deck, slot, Control::Opacity);
        strip.blend = blend_mode(deck.blend(slot));
        strip.mask = match deck.mask(slot).kind() {
            MaskKind::None => view::Mask::None,
            MaskKind::Linear => view::Mask::Linear,
            MaskKind::Radial => view::Mask::Radial,
        };
        // **Read for the press and painted nowhere**, which is what
        // `view::Strip::mask_angle` is for: the chip asks for a shape and the
        // operation carries an angle, so the angle it carries is the one the
        // slot already has (ADR-0203). A harness that left this at zero would
        // make every press straighten a diagonal front, and nothing on the
        // panel would show it having happened.
        strip.mask_angle = deck.mask(slot).angle();
        strip.level = deck.level(slot).map(|level| view::Level {
            mean: level.mean,
            peak: level.peak,
        });
    }
}

/// **The Staging lane's rows, off the deck's own verdicts** — one per slot
/// whose newest build has a verdict outstanding, or whose file no longer
/// agrees with its picture.
///
/// # It drains, because a `HotSwap`'s events are a caller's to take
///
/// `HotSwap::events` is documented as the caller's — *"a caller that stops
/// draining eventually makes this grow"* — and until this program had a
/// producer there was nothing to drain, so nothing did. `pending_events` is
/// the other reading and is deliberately **not** what this uses: it is the
/// read-only view `Deck::begin_frame` takes *inside* a frame, and a surface
/// that read it without draining would be the caller bug the engine names.
///
/// **So the lane's state is kept here rather than re-derived per frame**, and
/// that is what a drain forces and is also what is wanted: the events are a
/// stream of verdicts and a row is the newest of them per slot.
/// `view::View::staging` is written on the frames a build landed on and left
/// alone on every other, which is the same budget `mixer`'s name is kept to
/// (ADR-0164) with the frames-that-touch-it much rarer.
///
/// # What each verdict does to a row
///
/// - **`Swapped`, `Rejected`, `RolledBack`** — the slot has a row, on
///   `view::Stage`'s three words. Whether the build is on screen is exactly
///   what separates them.
/// - **`Accepted`** — the watchdog says the version held the budget, so the
///   file and the picture agree and the row leaves the lane. It is not the
///   operator's verdict, which is *keep* and is taste rather than cost; with
///   no control to keep with, a row that waited for one would never leave.
///   See `view::staging`, where that substitution is argued.
/// - **`WorkerLost`** — the build worker panicked and nothing will be built
///   again this run. **No row changes**, and that is the reading rather than
///   an omission: every verdict already taken still stands, and a slot whose
///   candidate was on trial when the worker went keeps a trial the watchdog
///   will still finish, because the watchdog is on this thread. What is lost
///   is the *next* build, and the lane has never been where that is said —
///   the engine prints it.
///
/// **A row is not removed when its slot is parked or its material is
/// replaced.** A candidate that landed in a slot the governor then parks keeps
/// its verdict outstanding, and that is `HotSwap::begin_frame_parked`'s own
/// rule read from the surface: a slot that is not drawn is not judged.
///
/// # It answers whether the live Set changed, because something else has to
/// know
///
/// `true` when a build was installed or a rollback put the previous Set back,
/// which is the moment `Set::published` says a console should re-read a slot —
/// *"A console reads this when a Set lands, not per frame."* [`inspector`] is
/// what acts on it. A refusal and a verdict in favour both answer `false`:
/// neither replaced what is playing.
///
/// One `String` per row that appears, on the frame a build landed on — which
/// is the frame that also installed a whole Set built on the worker. A row
/// whose slot rebuilds again rewrites the same buffer, and a run in which
/// nothing is saved allocates nothing here at all.
fn staging(
    deck: &mut Deck,
    out: &mut Vec<view::Candidate>,
    // **Where the same event goes when somebody who is not at the panel is
    // watching**, and `None` for a run without `--mcp`. A model that wrote a
    // procedure has no other way to learn that it was rolled back for cost, and
    // *it compiled* is not the same news as *it is on screen*.
    //
    // **Reported from here rather than from a second drain.** `Deck::events`
    // empties the channel, so a loop that read it again would read nothing at
    // all: the lane and the server are told by one pass or one of them is told
    // by none.
    //
    // The `String` is formed only when there is somebody to tell.
    mcp: Option<&mcp::Reporter>,
    // **Which slots took a build, and which build**, for the caller to take up
    // once this borrow of the deck has ended. `Some(id)` is a swap and `None` is
    // a rollback, which is the pair [`Keeping::took_up`] is written against.
    // Collected rather than acted on here, because `Deck::events` borrows the
    // deck for as long as it is being read.
    took: &mut Vec<(usize, Option<u64>)>,
) -> bool {
    let mut landed = false;
    for slot in 0..deck.slot_count() {
        for event in deck.events(slot) {
            if let Some(mcp) = mcp {
                mcp.swap(slot, &event.to_string());
            }
            match &event {
                Event::Swapped { id, .. } => took.push((slot, Some(*id))),
                Event::RolledBack { .. } => took.push((slot, None)),
                _ => {}
            }
            // **Whether the *live Set* changed**, which is a different
            // question from whether a row did and is why this is read here
            // rather than off the rows: a build that landed replaced what is
            // playing, and a rollback replaced it back. A refusal changed
            // nothing and neither did the watchdog's verdict in favour, which
            // is the candidate staying exactly where it was.
            landed |= matches!(
                verdict(&event),
                Verdict::Waiting(_, view::Stage::Landed | view::Stage::RolledBack)
            );
            match verdict(&event) {
                Verdict::Waiting(label, stage) => settle(out, slot, label, stage),
                // Nothing is outstanding on this slot any more, so it has no
                // row. `retain` rather than an index: the rows are as many as
                // the deck has slots and at most one of them is this one.
                Verdict::Settled => out.retain(|row| row.deck != slot),
                Verdict::Nothing => {}
            }
        }
    }
    landed
}

/// **What one verdict does to the lane** — the whole of the mapping, in a
/// function a test can reach without a device.
///
/// A `match` and not a lookup, for [`blend_mode`]'s reason: a sixth
/// `swap::Event` stops the build here rather than being passed over by a
/// wildcard, and *what it does to the lane* is a question the person adding it
/// should have to answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Verdict<'a> {
    /// The slot has a row, under this name and on this word.
    Waiting(&'a str, view::Stage),
    /// The slot has no row: its file and its picture agree.
    Settled,
    /// The lane does not change.
    Nothing,
}

/// See [`Verdict`] and [`staging`], which is where each arm is argued.
fn verdict(event: &Event) -> Verdict<'_> {
    match event {
        Event::Swapped { label, .. } => Verdict::Waiting(label, view::Stage::Landed),
        Event::Rejected { label, .. } => Verdict::Waiting(label, view::Stage::Refused),
        Event::RolledBack { label, .. } => Verdict::Waiting(label, view::Stage::RolledBack),
        Event::Accepted { .. } => Verdict::Settled,
        Event::WorkerLost => Verdict::Nothing,
    }
}

/// **One slot's row, written where it already is or put in slot order.**
///
/// The name is rewritten only when it differs, which is `mixer`'s rule for
/// `view::Strip::name` and is what keeps a rebuild of the same files off the
/// frame's allocation budget.
///
/// **Slot order, because that is the order the rows are read in** — the lane
/// draws them top to bottom and the deck letters are `A` through `D`, so a row
/// that appeared later must not sit above one that appeared first. The insert
/// is over at most `deck::MAX_SLOTS` entries.
fn settle(out: &mut Vec<view::Candidate>, deck: usize, label: &str, stage: view::Stage) {
    match out.iter_mut().find(|row| row.deck == deck) {
        Some(row) => {
            if row.name != label {
                row.name.clear();
                row.name.push_str(label);
            }
            row.stage = stage;
        }
        None => {
            let at = out.partition_point(|row| row.deck < deck);
            out.insert(
                at,
                view::Candidate {
                    deck,
                    name: label.to_owned(),
                    stage,
                },
            );
        }
    }
}

/// **The layer half of a node's address, as the mock's `.addr` spells it** —
/// `L1:0`, `L2:0`, `L4`.
///
/// `karakuri_ir::Kind` carries no name of its own, and `karakuri-cli`'s
/// `--publish name=L4:0:key` parser is in a package with no library target, so
/// there is nothing to call. The five words are `docs/ir-spec.md`'s and this
/// is a **match** for [`blend_mode`]'s reason: a sixth kind stops the build
/// here rather than drawing an address nothing can be typed back in.
fn layer_word(layer: Layer) -> &'static str {
    match layer {
        Layer::L1 => "L1",
        Layer::L2 => "L2",
        Layer::L3 => "L3",
        Layer::L4 => "L4",
        Layer::Field => "F",
    }
}

/// **Which node a published control belongs to**, and `None` where it belongs
/// to no one node.
///
/// `Published::at` is `Some` for a control an author addressed — the
/// `--publish name=L4:0:exposure` form — and `None` for a **wildcard**, which
/// is what the whole of the *default* interface is made of: *"one control per
/// key, not one per declaration"*, covering every node that declares the key.
/// This program has no `--publish` flag, so every control it ever draws is a
/// wildcard.
///
/// **A wildcard over exactly one node is that node's**, and the resolution
/// invents nothing: *every node that declares the key* is a set the Set itself
/// determines, and where it holds one member there is no second group the row
/// could go in. Over two or more it belongs to several groups at once, and the
/// mock draws no `.param` outside a `.node-group` — so the row is dropped and
/// [`inspector`] says how many were, rather than a place for it being invented
/// here (ADR-0200: *no placeholder, and no empty case the mock did not itself
/// draw*).
fn node_of(set: &Set, control: &Published) -> Option<(Layer, u32)> {
    if let Some(at) = control.at {
        return Some(at);
    }
    let mut declaring = set
        .params()
        .filter(|(_, _, key, _)| *key == control.key)
        .map(|(layer, index, _, _)| (layer, index));
    let first = declaring.next()?;
    match declaring.next() {
        None => Some(first),
        Some(_) => None,
    }
}

/// **What the Inspector's panes read**: one pane per slot the deck has, up to
/// the [`PANES`] the arrangement has, out of the seven things a `Set` will say
/// about itself.
///
/// Everything here is reachable from a `Deck` and nothing reaches around one —
/// `slot`, `transport` and the `Set` behind each slot are its own — and what
/// crosses into the console is a word, some numbers and some names (ADR-0156).
///
/// # Where each one comes from, and what is checked before it is drawn
///
/// All seven reads answer off a running `Set`, and each was checked against
/// the source rather than taken on trust:
///
/// - **`Set::layering`** is the fold chip. It is a *build* decision —
///   `merge.is_some()` — so it is a readout here and the mock's press is a
///   rebuild rather than a write.
/// - **`Set::inputs`** is the renderer row: one edge per renderer in draw
///   order, and `Input::live` says which one reaches the screen. It is
///   *"empty of meaning under `Layering::Overdraw`"* by the engine's own
///   words, so `live` is only ever passed on under composite — under overdraw
///   every renderer draws and marking one would assert a choice the layering
///   does not make.
/// - **`Set::node_names`** is a name per node, in node order, and
///   `Set::node_named` turns each one back into the `(layer, index)` the mock's
///   `.addr` is.
/// - **`Set::authority`** is the `man / sug / auto` chip, through
///   [`mix::authority`]. **Nothing writes one**: `Request::authorities` is
///   empty in every run of every program in this workspace, and it will stay
///   empty until a live-session writer exists, because `Record::Authority` is
///   deliberately excluded from Set-file state in two places (ADR-0216). So
///   the chip draws `man` on every node of every Set, forever, and that is the
///   *default* being read rather than a placeholder being drawn — a node
///   nobody has spoken for **is** manual.
/// - **`Set::published`** is which controls appear and in what order, which is
///   what numbers the rows. It **allocates and says it is not for the frame
///   path**, which is why this is called once — see below.
/// - **`Set::params`** is what resolves a wildcard control to a node — see
///   [`node_of`].
/// - **`Set::bindings`** is the one of the seven this does **not** read, and
///   the reason is that it cannot say anything here: nothing in this program
///   binds a signal to anything, so it is empty in every run and a `.pval.src`
///   drawn off it would be a state the engine never entered (ADR-0191).
///
/// # Read once, and that is the engine's instruction rather than a shortcut
///
/// `Set::published` is documented *"Allocates, so not the frame path. A
/// console reads this when a Set lands, not per frame."* **A Set lands
/// whenever a `.kir` in a slot is saved**, since every slot is watched, so
/// this is called at startup and again on the frame a build is installed or a
/// rollback puts the previous Set back — `staging` is what answers *did one
/// land*, and it is the only thing in this program that knows. Between those
/// frames almost every value above is constant: nothing writes a param,
/// nothing binds one, and nothing grants an authority.
///
/// **The transport is the one that moves now, and it is why this is called a
/// second time.** It had no caller outside its own tests when this was written
/// (ADR-0218); the deck head's scrub is that caller, so a press that writes a
/// `Record::Transport` re-reads the panes in [`Readout::performed`] — on the
/// press, which is where a directory read already happens, and never on a
/// frame. Re-reading the whole pane rather than writing the two numbers back
/// is deliberate: a second writer into `View::inspector` is a second answer to
/// *what is this pane showing*, and the reading that draws the anchor has to
/// be the reading the deck holds.
///
/// **What a re-read for the rest of it waits on is a writer**, and it is the
/// same writer the authority chip waits on.
fn inspector(deck: &Deck, names: &[String], out: &mut Vec<view::Pane>) {
    out.clear();
    for slot in 0..deck.slot_count().min(view::PANES) {
        // The strip's name for the same slot, and for [`mixer`]'s reason: a
        // pane head reads `deck A · drift_night`, and after a load that is the
        // Set the operator chose rather than the pair the run opened with.
        let material = names.get(slot).map_or("", String::as_str);
        let set = deck.slot(slot).set();
        let transport = deck.transport(slot);
        let composite = set.layering() == Layering::Composite;

        // Every published control, resolved to the node it belongs to and
        // numbered by its position in the interface — which is the number a
        // MIDI control is learned against, so it counts the controls that were
        // published and not the rows that could be placed.
        let published = set.published();
        let mut rows: Vec<(Option<(Layer, u32)>, view::Param)> = Vec::new();
        for (at, control) in published.iter().enumerate() {
            let [low, high] = control.range;
            let value = set.published_value(&control.name).unwrap_or(low);
            rows.push((
                node_of(set, control),
                view::Param {
                    ord: at + 1,
                    name: control.name.clone(),
                    value,
                    at: match high > low {
                        // A range of no width is a control with one position,
                        // and the fader sits at its start rather than at a
                        // division by zero.
                        false => 0.0,
                        true => (value - low) / (high - low),
                    },
                },
            ));
        }

        let mut nodes: Vec<view::Node> = Vec::new();
        let mut renderers: Vec<view::Renderer> = Vec::new();
        let mut renderer_nodes = 0;
        let mut renderer_authority = None;
        for name in set.node_names() {
            let Some((layer, index)) = set.node_named(name) else {
                continue;
            };
            let authority = set.authority(layer, index).map(mix::authority);
            let params = |layer: Layer, index: u32| {
                rows.iter()
                    .filter(|(at, _)| *at == Some((layer, index)))
                    .map(|(_, param)| param.clone())
                    .collect::<Vec<_>>()
            };
            if layer == Layer::L4 {
                // **The renderers fold into one group**, which is the mock's
                // own `L4 renderers` head over a row of chips: the chips *are*
                // the L4 nodes, and the row is what the fold turns into a
                // choice. Every other layer is one group per node, addressed
                // `L2:0` the way the mock addresses it.
                renderers.push(view::Renderer {
                    name: name.clone(),
                    live: composite
                        && set
                            .inputs()
                            .get(index as usize)
                            .is_some_and(|edge| edge.live),
                });
                renderer_nodes += 1;
                renderer_authority = match renderer_nodes {
                    1 => authority,
                    // **More than one node under one head has no one
                    // authority**, and authority is per node (ADR-0216). The
                    // chip is dropped rather than showing the first of them.
                    _ => None,
                };
                continue;
            }
            nodes.push(view::Node {
                addr: format!("{}:{index}", layer_word(layer)),
                name: name.clone(),
                authority,
                renderers: Vec::new(),
                params: params(layer, index),
            });
        }
        if renderer_nodes > 0 {
            // The mock's address for the folded head is the bare layer, with
            // no index — because it is not one node's.
            let mut params: Vec<view::Param> = Vec::new();
            for index in 0..renderer_nodes {
                params.extend(
                    rows.iter()
                        .filter(|(at, _)| *at == Some((Layer::L4, index)))
                        .map(|(_, param)| param.clone()),
                );
            }
            params.sort_by_key(|param| param.ord);
            nodes.push(view::Node {
                addr: layer_word(Layer::L4).to_owned(),
                name: RENDERERS_NODE.to_owned(),
                authority: renderer_authority,
                renderers,
                params,
            });
        }

        let placed: usize = nodes.iter().map(|node| node.params.len()).sum();
        if placed < published.len() {
            // **Said rather than swallowed**, for the reason every other
            // omission in this file is said: a pane short of a row looks
            // exactly like a Set that published fewer. See `node_of` — a
            // wildcard over two or more nodes belongs to two or more groups,
            // and the mock draws no row outside one.
            println!(
                "inspector: deck {} publishes {} controls and {} of them name no one node, so \
                 they have no group to sit in and are not drawn",
                DECK_LETTERS.get(slot).copied().unwrap_or("?"),
                published.len(),
                published.len() - placed
            );
        }

        out.push(view::Pane {
            deck: slot,
            material: material.to_owned(),
            sync: mix::sync(transport.sync()),
            // **What the sync chip's cycle skips over, asked of the engine
            // three times.** `Deck::sync_allowed` is *"what a surface greys a
            // control out on, and it answers before anything is pressed"* —
            // whether the Set in this slot is closed form and whether it reads
            // `beats`, put through `Transport::allows`. The console is handed
            // the three answers rather than the two properties, because what
            // may be asked for is not a surface's to work out (P-0076) and a
            // third copy of that rule in a crate with no material in it is a
            // rule that can start disagreeing.
            //
            // **`EngineSync::ALL` is in `SYNCS`' order**, which is what makes
            // this array line up with the field it fills;
            // `the_two_crates_walk_the_sync_modes_in_one_order` is what says
            // so rather than this comment.
            allows: EngineSync::ALL.map(|mode| deck.sync_allowed(slot, mode).is_ok()),
            anchor_bpm: transport.anchor_bpm(),
            scrub_beats: transport.scrub_beats(),
            composite,
            nodes,
        });
    }
}

/// **What the mock calls the group its renderer chips sit under.** Not a node
/// name — every L4 node has one of those and they are the chips themselves —
/// but the head over all of them, which the mock writes as `L4 renderers`.
const RENDERERS_NODE: &str = "renderers";

/// **The engine's residency, as the console's word for it** — and, like
/// [`blend_mode`], a `match` so that a fourth `Residency` stops the build here
/// rather than drawing a chip nothing can read.
///
/// One function for both halves of the pair. It was written inline for the
/// effective residency alone; the request needs exactly the same three arms,
/// and a second copy of them is a translation that can start disagreeing with
/// itself about what `Priming` is called.
fn tally(residency: Residency) -> view::Tally {
    match residency {
        Residency::Live => view::Tally::Live,
        Residency::Priming => view::Tally::Priming,
        Residency::Allocated => view::Tally::Allocated,
    }
}

/// **The engine's blend mode, as the vocabulary's** — and the one place the
/// two lists are made to agree.
///
/// `karakuri-operation` owns its own copy of every list a destination is drawn
/// from, which is the cost P-0074 says the vocabulary pays: *"The two rules —
/// be engine-neutral, and have no toggles — are not jointly satisfiable unless
/// the vocabulary owns the lists."* A copy needs somewhere the two meet, and
/// this is that place for this list, on the harness side of the seam — the
/// same side [`mixer`] reads a `Deck` from (ADR-0156).
///
/// **A match, so the day a fourth mode lands in `karakuri_engine::deck::Blend`
/// this stops compiling.** That is the whole reason `view::Strip::blend` is a
/// `BlendMode` and not the engine's word: a `&str` handed through would draw
/// the new mode's name on a chip, and the chip would cycle three ways past a
/// state no operation can name and no MIDI map can reach, with nothing saying
/// so. Failing here is the loud failure P-0027 asks for.
///
/// **It stays this program's, and that is now settled rather than pending.**
/// The console cannot depend on the engine (ADR-0156), and
/// `karakuri-operation-record` cannot either — it is the vocabulary and the
/// records and nothing else, by charter. So the two lists meet on the harness
/// side of the seam, wherever a harness holds both, and ADR-0180's *"one
/// `From` impl per list in `karakuri-cli`"* cannot be written at all: neither
/// [`Blend`] nor [`BlendMode`] is that package's, and the orphan rule refuses
/// it ([ADR-0194](../../../docs/adr/0194-where-an-operation-becomes-a-record-is-a-crate-that-depends-on-both.md)).
fn blend_mode(blend: Blend) -> BlendMode {
    match blend {
        Blend::Add => BlendMode::Add,
        Blend::Over => BlendMode::Over,
        Blend::Max => BlendMode::Max,
    }
}

/// **The engine's look, as the console reads it** — [`blend_mode`]'s function
/// one row up, on the value every sink is drawn under.
///
/// Two fields of three: `white_point` is Reinhard's parameter, it is on no
/// surface, and a console field for it would be a reading no control names —
/// see `karakuri_console::view::Look`. The operator goes through
/// [`mix::tonemap`], which is the match that makes the engine's list and the
/// vocabulary's agree and stops compiling the day a fifth operator lands on
/// one side only.
fn look(look: &Look) -> view::Look {
    view::Look {
        tonemap: mix::tonemap(look.op),
        exposure: look.exposure,
    }
}

/// **What this window says when a control's operation wrote no record**, and
/// the two ways that happens are not the same thing — so they are not the
/// same sentence.
///
/// [`written`] has three answers and only one of them is a record.
/// A harness that printed a line for that one and nothing at all for the
/// other two would tell an operator that a press did nothing, which is true
/// of neither:
///
/// - [`Written::Silent`] is **settled**. Selecting a deck or folding a bay is
///   a surface's own state and there is nothing to write; the sentence says
///   which of the four kinds of nothing it is, and that is the end of it.
/// - [`Written::Owed`] is **a gap nobody has closed yet**. A fade owes a
///   record and no build can make it, so a press that reads as *nothing
///   happened* is exactly the wrong reading — the sentence names the question
///   instead, which is `Owed::why`'s whole job and the reason `Owed` is not
///   an error.
///
/// `None` for [`Written::Records`], because that line is [`apply`]'s: it says
/// the record *and* what the deck holds afterwards, and printing both would
/// say one press twice.
///
/// **Nothing on this panel reaches the `Owed` arm on purpose any more**, and
/// the paragraph that used to stand here is worth keeping as history because
/// it was twice wrong in the same place. It first said no control could reach
/// either arm and was written for the day one did; the deck head was that day,
/// and it said the sync chip and the anchor beside it were **reachable
/// affordances over an unwritable record** — the press claimed, the operation
/// emitted, this sentence printed with the question in it, and the deck not
/// moving.
///
/// **What made the record unwritable was a question that had already been
/// answered.** `Transport::engaged` decides what engaging a mode means, with
/// the reason at its own definition: the anchor is the session tempo and the
/// scrub is cleared. The clamp that looked like a decision about the bytes on
/// disk is the identity on every tempo an oscillator can report, so there were
/// never two answers to choose between — only a reading nobody was handing in.
/// [`reading`] hands it in now, `written` writes `Record::Transport`, and
/// [`apply`] moves the deck, which is the ninth and tenth of this panel's ten
/// emitting controls arriving where the other eight already were.
///
/// **The refusal to route around it is what made that cheap.** A surface owns
/// the affordance and never the authority
/// ([P-0076](../../../docs/principles/0076-a-surface-owns-the-affordance-never-the-authority.md)),
/// so this file never wrote a `Record::Transport` of its own for a `SetSync` —
/// computing the anchor here would have been a window binary taking a decision
/// about a file format, and the printed line was the right answer until the
/// conversion existed
/// ([P-0027](../../../docs/principles/0027-a-silently-wrong-image-loses-to-a-loud-failure.md)).
/// What changed is the conversion, not this file's authority: the anchor is
/// still the engine's policy and this window still only reads a tempo.
///
/// So all ten controls write records. Five are the mixer's — `SetGain`,
/// `SetOpacity`, `SetBlendMode`, `SetResidency` and `SetMaskShape` — two are
/// the look's, and the last three are the deck head's: the scrub, the chip
/// that cycles and the anchor that re-asks for the mode the deck is in
/// (ADR-0218). The Outputs dot never arrives here at all, because it asks the
/// panel for an arrangement [`Op`] and the panel performs it
/// ([`Acted::Operated`]).
///
/// **The mask is also the one that can reach [`Written::Owed`] by accident**,
/// and that is worth having rather than designing away: a reading that did not
/// arrive answers `Owed(NotRead(Reading::Mask))`, so a harness that stopped
/// handing one in would say so out loud instead of moving nothing.
fn unwritten(operation: &Operation, written: &Written) -> Option<String> {
    match written {
        Written::Records(_) => None,
        Written::Silent(silent) => Some(format!(
            "  emitted: {operation:?} -> no record, and that is settled: {}",
            silent.why()
        )),
        Written::Owed(owed) => Some(format!(
            "  emitted: {operation:?} -> no record, and that is a gap rather than a \
             decision: {}. nothing moved, and nothing here decides it",
            owed.why()
        )),
    }
}

/// **A press on a strip or a deck key, applied to the console's own pointer**,
/// and what to say about it. `None` for every operation that is not it.
///
/// `Operation::SelectDeck` *"writes no record, and is the reason every other
/// variant names its deck instead of meaning the selected one"*, so there is
/// nothing on the deck for [`apply`] to move and the surface that emits it is
/// what performs it (ADR-0198). This is that performance, and it is one line
/// beside [`arrangement`]'s for the same reason: the alternative is a second
/// route into the view.
///
/// **A deck the mixer has no strip for is refused**, and `View::select` is
/// where that rule lives — the ring would be drawn nowhere and the library's
/// pill would name a deck a load could not reach. It is said here rather than
/// swallowed, because a key that does nothing and a key that is not bound are
/// the same experience.
fn pointed(view: &mut View, operation: &Operation) -> Option<String> {
    let Operation::SelectDeck { deck } = *operation else {
        return None;
    };
    let letter = deck_letter(deck);
    if usize::from(deck) >= view.mixer.len() {
        return Some(format!(
            "  select: deck {letter} refused — this deck has {} slot{}, and a selection with no \
             strip under it is a ring drawn nowhere and a `load` pill naming a deck the press \
             could not reach",
            view.mixer.len(),
            match view.mixer.len() {
                1 => "",
                _ => "s",
            }
        ));
    }
    view.select(deck);
    Some(format!(
        "  select: deck {letter} -> SelectDeck {{ deck: {deck} }} -> no record, and that is \
         settled: it is a surface's own pointer. The keys are addressed here, and the library's \
         foot reads `load -> {letter}`"
    ))
}

/// **Which salt a slot's material is seeded from** — the one it was built at,
/// and the one a load restates when the Set file recorded none.
///
/// Deck A's is [`SEED_SALT`] and every slot after it is one further along, so
/// this deck's four slots are four different simulations of the one procedure
/// [`Sources`] names. **That generalises the reason the second salt was
/// written for and then retires the constant.** `WARM_SEED_SALT` existed so
/// that *the slot the budget parks is a different simulation rather than a
/// second copy of the same one* — an argument about the parked slot, made when
/// the parked slot was the only other slot there was. What it was really
/// saying is that a deck of one picture repeated is not a mixer, and that is
/// true of every slot rather than of deck B, so it is said once here and no
/// constant states a reason that has gone
/// ([P-0063](../../../docs/principles/0063-source-cites-what-is-in-force-not-a-plan.md)).
///
/// **`+ slot` rather than a table**, because a table of four numbers is four
/// values with nothing to say about each other, and what is wanted is exactly
/// *distinct, and deck A's is the one the CLI's tests use*. Distinctness is
/// then arithmetic rather than four typed numbers nobody re-reads — which
/// `every_slot_is_its_own_simulation` asserts salt by salt, off the Sets the
/// deck actually built rather than off this function.
///
/// **The salts the run was built with, and no others**: a Set loaded into a
/// slot is new *material* and not a new simulation, so a rebuild that derived
/// its own seed would repaint every element in the slot for a reason nobody
/// asked for — which is `Watch::salts`' own argument, met from the loading
/// side.
fn slot_salt(slot: usize) -> u32 {
    SEED_SALT + slot as u32
}

/// **A load, performed** — [`loading`] reached from an operation, and `None`
/// for every operation that is not one.
///
/// `Operation::LoadSet` writes no record either, so this is [`pointed`]'s
/// shape one bay along: the surface that names it performs it. What it does
/// **not** do is touch the deck, which is the whole design — see [`loading`]
/// and [`Engine::aimed`].
///
/// Every failure is a sentence and none of them moves anything: a slot the
/// deck has not got, a store that will not open, a Set that is not there, a
/// procedure that no longer checks, or a worker that has gone.
fn played(gfx: &mut Gfx, operation: &Operation) -> Option<String> {
    let Operation::LoadSet { deck, set } = operation else {
        return None;
    };
    let slot = usize::from(*deck);
    let letter = deck_letter(*deck);
    let count = gfx.engine.aimed.len();
    let Some(aim) = gfx.engine.aimed.get_mut(slot) else {
        return Some(format!(
            "  load: deck {letter} refused — this deck has {count} slot{}, and `{set}` has \
             nowhere to land",
            match count {
                1 => "",
                _ => "s",
            }
        ));
    };
    match loading(&gfx.store, slot, slot_salt(slot), aim, set) {
        Ok(line) => {
            // **What that slot is now playing**, written on the press that
            // changed it. A `Set` has no name of its own, so the strip and the
            // pane head read whatever whoever built it says — and after a load
            // that is the id the operator picked out of the library, which is
            // the same word the row they pressed on carries.
            //
            // **Written on the aim rather than on the swap**, which is a
            // choice and not an oversight: the build may still be refused or
            // rolled back for cost, and a name that waited for the verdict
            // would leave the strip naming material that is no longer in the
            // file. The staging lane is what says which of the three happened,
            // on the deck it happened to, and it is the surface built for
            // exactly that disagreement — a strip name that hedged would be a
            // second, quieter answer to the question that lane is about.
            if let Some(name) = gfx.material.get_mut(slot) {
                name.clear();
                name.push_str(set);
            }
            Some(line)
        }
        Err(e) => Some(format!(
            "  load: `{set}` did not reach deck {letter}: {e} — nothing moved, and what is on \
             that deck is still running"
        )),
    }
}

/// **The record, applied to the deck**, and what to say about it.
///
/// **This is not the half ADR-0185 promised to delete, and it did not go with
/// it.** Turning an `Operation` into a `Record` was the shortcut — that
/// function is gone and [`written`] answers instead
/// ([ADR-0194](../../../docs/adr/0194-where-an-operation-becomes-a-record-is-a-crate-that-depends-on-both.md)).
/// Turning a record into a *deck movement* is a different job and is the
/// harness's by design: `karakuri-operation-record` has no engine and never
/// will, so somebody who owns a deck has to decode.
///
/// `karakuri-cli`'s `mix::change` is the real decoder and it does two things
/// this does not: it refuses a slot the deck has not got, with the same
/// sentence every other surface refuses one with, and it turns a record into a
/// `Change` that a caller applies. This is the shortest path from the records
/// [`written`] answers with to the setters they name.
///
/// The line it returns is the loop closing, printed so that it can be read
/// rather than inferred: the operation, the record, and **what the deck says
/// afterwards** — which is where the next frame's strip comes from.
///
/// # It takes the look as well as the deck, and that is not a second target
///
/// `Record::Look` is the one record here that does not name a slot: the look
/// is what *every* sink is drawn under, so it is `Engine::look` rather than
/// anything on the deck ([`Engine::look`], and `karakuri_engine::frame::Look`
/// for why the master out is deliberately not in it). Handing both in is what
/// keeps this one function the only place a record becomes a movement — a
/// second `apply_look` beside it would be the second route into the engine
/// that P-0028 exists to refuse.
fn apply(record: &Record, deck: &mut Deck, look: &mut Look) -> Option<String> {
    // **A slot the deck has not got is refused rather than indexed.**
    // `Deck::set_gain` indexes its slots, a panic reachable from an event
    // handler aborts this process rather than unwinding (see the module
    // documentation), and `mix::change`'s whole reason for taking a
    // `slot_count` is that a stream may name a slot that is not there. Nothing
    // in this file can produce one — the strips are the deck's own count — so
    // this is the guard rather than the message, and the real sentence is
    // `karakuri-cli`'s `no_such_slot`.
    let held = |slot: u8| (usize::from(slot) < deck.slot_count()).then_some(usize::from(slot));
    match *record {
        Record::Gain { slot, value } => {
            let slot = held(slot)?;
            deck.set_gain(slot, value);
            Some(format!(
                "  fader: deck {} trim -> SetGain {{ deck: {slot}, gain: {value:.3} }} \
                 -> Record::Gain -> deck.gain({slot}) = {:.3}",
                deck_letter(slot as u8),
                deck.gain(slot)
            ))
        }
        Record::Opacity { slot, value } => {
            let slot = held(slot)?;
            deck.set_opacity(slot, value);
            Some(format!(
                "  fader: deck {} fader -> SetOpacity {{ deck: {slot}, opacity: {value:.3} }} \
                 -> Record::Opacity -> deck.opacity({slot}) = {:.3}",
                deck_letter(slot as u8),
                deck.opacity(slot)
            ))
        }
        // **The mode comes back off the wire name, and an unknown one is
        // refused rather than defaulted.** `Record::Blend` carries a `String`
        // because what a mode is allowed to be is the engine's to say, so this
        // is the engine saying it — `mix::change` refuses the same way, with
        // the sentence `karakuri-cli`'s `no_such_blend` writes. Nothing in
        // this file can produce a name the engine has not got, since the chip
        // only ever emits one of `BlendMode::ALL`, so this is the guard rather
        // than the message.
        Record::Blend { slot, ref mode } => {
            let slot = held(slot)?;
            let blend = Blend::from_name(mode)?;
            deck.set_blend(slot, blend);
            Some(format!(
                "  blend: deck {} -> SetBlendMode {{ deck: {slot}, blend: {mode} }} \
                 -> Record::Blend -> deck.blend({slot}) = {}",
                deck_letter(slot as u8),
                deck.blend(slot).name()
            ))
        }
        // **The one record here that is followed by a governor pass**, and it
        // is not a flourish: `Deck::set_residency` writes the request *and
        // grants it*, so a harness that stopped there would put a slot the
        // budget has no room for into `Priming` and draw a primed deck the
        // governor never admitted. That is
        // [ADR-0191](../../../docs/adr/0191-the-panels-parked-deck-is-parked-by-the-governor-or-it-is-a-drawing-of-one.md)
        // exactly — the panel's parked deck is the governor's verdict or it is
        // a drawing of one — and it is `karakuri-cli`'s own order, where
        // `mix::Change::Residency` sets the level and calls `govern` beside
        // it. The report is dropped here rather than printed: the line below
        // says what the deck ended up at, which is the half this window shows.
        Record::Residency { slot, ref level } => {
            let slot = held(slot)?;
            let residency = mix::parse_residency(level)?;
            deck.set_residency(slot, residency);
            deck.govern();
            Some(format!(
                "  tally: deck {} -> SetResidency {{ deck: {slot}, residency: {level} }} \
                 -> Record::Residency -> deck.requested_residency({slot}) = {:?}, \
                 deck.residency({slot}) = {:?}",
                deck_letter(slot as u8),
                deck.requested_residency(slot),
                deck.residency(slot)
            ))
        }
        // **The whole mask, because the record is a state and not an ask.**
        // `Record::Mask` carries a shape, an angle, a position and a softness,
        // and `Deck::set_mask` is what it decodes to — the engine says so at
        // that setter. Reaching for `Deck::set_mask_shape` instead, to keep a
        // running wipe alive, would be this file decoding a record by picking
        // two fields out of it and dropping the softness on the floor: a
        // second route to the deck, where P-0028 is that every control ends in
        // the same record. **So a shape press stops a wipe on that deck**, and
        // that is not a fault here — it is the honest limit
        // `docs/principles/0078-the-operator-wins-and-an-automatic-writer-yields-to-a-hand.md`
        // states about the record stream, met by the first surface to make the
        // press
        // ([ADR-0203](../../../docs/adr/0203-the-mask-chip-carries-the-angle-it-does-not-control.md)).
        //
        // **The shape comes back off the wire name**, refused rather than
        // defaulted, exactly as the blend's and the residency's do. Nothing in
        // this file can produce a name the engine has not got, since the chip
        // only ever emits one of `WipeKind`'s three and the record is written
        // from `WipeKind::name`.
        Record::Mask {
            slot,
            ref kind,
            angle,
            position,
            softness,
        } => {
            let slot = held(slot)?;
            let shape = MaskKind::from_name(kind)?;
            deck.set_mask(slot, Mask::new(shape, angle, position, softness));
            Some(format!(
                "  mask: deck {} -> SetMaskShape {{ deck: {slot}, kind: {kind}, \
                 angle: {angle:.3} }} -> Record::Mask -> deck.mask({slot}) = {} \
                 at {:.3} rad, front at {:.3}",
                deck_letter(slot as u8),
                deck.mask(slot).kind().name(),
                deck.mask(slot).angle(),
                deck.mask(slot).position()
            ))
        }
        // **The whole look, because the record is a state and not an ask.**
        // `Record::Look` carries the operator, the exposure and the white
        // point together for its own stated reason — a stream that set a level
        // without naming the operator would describe a look nobody can
        // reconstruct — and each of the two controls asks for one of the three
        // (ADR-0192). The other two arrive here already filled in from the
        // reading [`reading`] took, so this writes what it is given and picks
        // nothing out of it, exactly as the mask arm does.
        //
        // **The operator comes back off the wire name, refused rather than
        // defaulted**, as the blend's, the residency's and the shape's do.
        // Nothing in this file can produce a name the engine has not got: the
        // capsule only ever emits one of `Tonemap`'s four and the record is
        // written from `Tonemap::name`, which is the same lower-case spelling
        // `mix::op_wire_name` parses.
        //
        // **No `set_tonemap` call.** `compose` uploads the tone-map uniform
        // every frame from the `Committed` the closure hands back, so writing
        // the field *is* the write — and a `Present::set_tonemap` here would be
        // a second writer, with the last one each frame winning.
        Record::Look {
            ref op,
            exposure,
            white_point,
        } => {
            let op = mix::parse_op(op)?;
            *look = Look {
                op,
                exposure,
                white_point,
            };
            Some(format!(
                "  look: -> Record::Look {{ op: {op_name}, exposure: {exposure:.3}, \
                 white_point: {white_point:.3} }} -> every sink is drawn under {op_name} at \
                 exposure {exposure:.3}",
                op_name = mix::op_wire_name(op)
            ))
        }
        // **One level on the whole fold, and the one arm here that names no
        // slot at all.** `Record::MasterOut` carries a number and nothing
        // else, so unlike the look and the mask there is no other half of it
        // to fill in from what is running — which is why the reading below
        // has no arm for this control (ADR-0224).
        //
        // **The engine clamps and this does not.** `Deck::set_out` floors at
        // zero and is deliberately open above 1.0, through the same
        // `clamp_gain` the per-slot gain goes through, because the mix is HDR
        // and this level is applied to values a tone mapper has not seen —
        // [P-0064](../../docs/principles/0064-the-pipeline-is-linear-hdr-and-srgb-is-encoded-once-at-final-output.md).
        // A clamp here would be a second opinion about a range the setter
        // already holds, which is the rule the whole conversion is written
        // under.
        //
        // **No `cancel` to worry about**, where the gain and the fader each
        // stop whatever was moving them: a `Control` is per slot and this is
        // not, so nothing in the engine can be moving it and there is nothing
        // for a hand to win against.
        Record::MasterOut { value } => {
            deck.set_out(value);
            Some(format!(
                "  master: out -> SetMasterOut {{ out: {value:.3} }} -> Record::MasterOut -> \
                 deck.out() = {:.3}, at the entry to the master chain",
                deck.out()
            ))
        }
        // **The whole of one slot's clock, because the record is a state and
        // not an ask.** `Record::Transport` carries the sync mode, the anchor
        // and the scrub together for a stated reason — a scrub position
        // without the mode and the anchor beside it *"would replay a slot onto
        // a grid it was never on"* — and `Deck::set_transport` is what it
        // decodes to. The mode and the anchor arrive here unchanged, from the
        // reading [`reading`] took a moment earlier; only the scrub has moved.
        //
        // **The mode comes back off the wire name, refused rather than
        // defaulted**, as the blend's, the residency's, the shape's and the
        // operator's do. Nothing in this file can produce a name the engine
        // has not got: the scrub only ever writes back the mode it just read
        // off the same deck, and the record is written from `Sync::name`.
        //
        // **`set_transport` refuses, and the refusal is dropped here on
        // purpose.** It refuses a mode this slot's material cannot honour, and
        // a scrub cannot present one — it names the mode the slot is already
        // in, which the slot is in because the engine allowed it. What it
        // guards against is the case the engine names at `sync_allowed`: a
        // swap that puts accumulating material into a slot that is beat-synced
        // makes the mode it is *already in* unavailable, *"and whatever wires
        // swapping to this owes it"*. **Both slots are watched now, so that
        // case is reachable**: save an accumulating procedure into a
        // beat-synced slot and the next scrub is refused. What should be
        // printed then is the deck's own refusal rather than this arm's line,
        // and it is what this owes.
        Record::Transport {
            slot,
            ref sync,
            anchor_bpm,
            scrub_beats,
        } => {
            let slot = held(slot)?;
            let mode = EngineSync::from_name(sync)?;
            deck.set_transport(slot, mode, anchor_bpm, scrub_beats)
                .ok()?;
            Some(format!(
                "  scrub: deck {} -> ScrubDeck {{ deck: {slot} }} -> Record::Transport {{ \
                 sync: {sync}, anchor_bpm: {anchor_bpm:.1}, scrub_beats: {scrub_beats:+.2} }} \
                 -> deck.transport({slot}).scrub_beats() = {:+.2} beats",
                deck_letter(slot as u8),
                deck.transport(slot).scrub_beats()
            ))
        }
        _ => None,
    }
}

/// **The reading an operation's record needs, taken off the deck it names.**
///
/// [`written`] builds `Record::Mask` **whole** — a shape, an angle, a position
/// and a softness — out of an operation that names two of the four, and the
/// other two come from a reading of the mask that is running (ADR-0201). This
/// is that reading, and it is the harness's because the deck is
/// (ADR-0156, ADR-0194).
///
/// **`Current::default()` is *I read nothing*, and it is still the answer for
/// four of this panel's ten emitting controls**: a gain, an opacity, a blend
/// mode and a residency each carry everything their record carries, so handing
/// a reading in would be this file inventing a value. The mask mini is one of
/// the six that need one, and it needs it for the deck the operation *names*
/// rather than for the deck the pointer is over — which is `Reading::Mask`'s
/// own wording and the reason this takes the operation and not a slot.
///
/// **The two look controls are two of the other three**, and they read one
/// thing between them: the look that is running. Each names a third of
/// `Record::Look` and the other two thirds come from here — which is
/// [`Reading::Look`]'s own wording and the reason the reading is taken for the
/// operation rather than per control.
///
/// **The scrub's two arrows are the fourth**, and the reading they take is the
/// one thing on this list that is not a completion: see the arm.
///
/// **The sync chip and the anchor are the fifth and sixth**, and they read the
/// one thing here that belongs to no deck: the session tempo. That arm used to
/// be absent and the two controls used to print a question instead of moving
/// anything — see [`unwritten`] for what the question turned out to be.
///
/// **The softness is read back**, where `karakuri-cli`'s `mix::current_mask`
/// substitutes its own `MASK_SOFTNESS`: that program writes wipes and has a
/// softness of its own to write, and this window has never written one. What
/// is read back here is therefore what is actually on the slot, and reading it
/// back is what stops a press rewriting it — the same argument the angle's is,
/// one field along.
///
/// A slot the deck has not got answers `None`, and [`written`] then says the
/// reading was owed rather than indexing something that is not there — the
/// guard [`apply`] has, at the other end of the same press.
fn reading(operation: &Operation, deck: &Deck, look: &Look) -> Current {
    let look = match *operation {
        // **Two thirds of the record, for whichever third was asked for.**
        // `SetTonemap` carries an operator and `SetExposure` a level, and
        // `Record::Look` needs all three — so the running look is handed in
        // and `written` takes the two the press did not name. That is
        // ADR-0192's argument executable in this file: the operation carries
        // what a surface can say and the translator completes the record.
        // `white_point` is on no surface at all, so it survives every press by
        // arriving here and going straight back out.
        Operation::SetTonemap { .. } | Operation::SetExposure { .. } => {
            Some(karakuri_operation_record::Look {
                tonemap: mix::tonemap(look.op),
                exposure: look.exposure,
                white_point: look.white_point,
            })
        }
        _ => None,
    };
    // **The scrub is the third reading, and it is the only one that is
    // relative.** `Record::Transport` is absolute — a sync mode, an anchor and
    // a scrub position — and `ScrubDeck` names an amount, so the record is
    // where the slot already is plus what was asked for. The reading is
    // therefore not a completion of a record the way the look's and the mask's
    // are: it is the left-hand side of an addition, and without it the
    // conversion answers `Owed(NotRead(Transport))` rather than starting a
    // deck's scrub from zero.
    //
    // **All three fields, because the record is written whole.** A scrub that
    // wrote a position without the mode and the anchor beside it *"would
    // replay a slot onto a grid it was never on"* —
    // `karakuri_operation_record::Transport` says so at its own definition —
    // and the two it does not touch survive the press by arriving here and
    // going straight back out, which is the white point's arrangement one
    // reading up.
    let transport = match *operation {
        Operation::ScrubDeck { deck: slot, .. } => {
            let slot = usize::from(slot);
            (slot < deck.slot_count()).then(|| {
                let transport = deck.transport(slot);
                karakuri_operation_record::Transport {
                    sync: mix::sync(transport.sync()),
                    anchor_bpm: transport.anchor_bpm(),
                    scrub_beats: transport.scrub_beats(),
                }
            })
        }
        _ => None,
    };
    // **The tempo the room is going at, and nothing about a slot.** Engaging
    // a sync mode anchors the slot at the session tempo so that the picture
    // does not move at the instant it goes on the grid, which is
    // `karakuri_engine::transport::Transport::engaged`'s policy and the whole
    // of what this reading is for. `mix::current_tempo` takes the oscillator
    // rather than an `f32`, so this window cannot hand in a tempo the session
    // never ran at — and it reads the grid rather than a clock, which is what
    // lets the record be replayed
    // ([P-0002](../../../docs/principles/0002-simulation-time-comes-from-a-record-never-from-a-clock.md)).
    //
    // **No slot check, unlike the three below.** The tempo is the session's,
    // so a `SetSync` naming a slot the deck has not got is a record with a slot
    // out of range rather than a reading that could not be taken, and
    // `apply`'s own guard is what says so at the other end of the press.
    let tempo = match *operation {
        Operation::SetSync { .. } => Some(mix::current_tempo(deck.signals().oscillator())),
        _ => None,
    };
    let mask = match *operation {
        Operation::SetMaskShape { deck: slot, .. } => {
            let slot = usize::from(slot);
            (slot < deck.slot_count()).then(|| {
                let mask = deck.mask(slot);
                karakuri_operation_record::Mask {
                    kind: wipe_kind(mask.kind()),
                    angle: mask.angle(),
                    position: mask.position(),
                    softness: mask.softness(),
                }
            })
        }
        _ => None,
    };
    // **Every field named, and no `..Current::default()` behind them.** Every
    // reading the type carries is answered here, so a fill would be dead — and
    // the day it grows one more, this stops compiling and somebody has to say
    // whether this window can take it, rather than a `None` arriving silently.
    //
    // **The count that used to be in this sentence is gone rather than
    // corrected.** It said four where there are three and named a fifth that
    // would be a fourth, which is a figure nothing checks going stale in the
    // one comment whose whole argument is that the compiler does the checking.
    // **The transition settings are the one reading this window answers
    // `None` for, and it is an answer rather than an omission.** The type grew
    // them the day `FadeDeck`, `Crossfade` and `SelectRenderer` stopped being
    // owed, which is exactly the event the paragraph above was written to
    // catch — so this is somebody saying whether this window can take it, and
    // the answer is that it has nothing to say.
    //
    // A quantum and a length are a *surface's* setting deciding what the next
    // move means, and this panel draws no control that sets either: the
    // transition row is not built, there is no crossfader to build, and no
    // control here emits any of the three operations that read them. A value
    // handed in would be this file inventing a setting nobody chose, which is
    // the failure `Current`'s every-field-optional rule exists to prevent —
    // and if a control ever does emit one before the settings exist, the
    // window prints *the transition settings its move is scheduled by were not
    // handed over* and names the operation, which is a sentence rather than a
    // fade at a length the operator never set.
    let transition = None;
    Current {
        look,
        mask,
        transport,
        tempo,
        transition,
    }
}

/// **The engine's mask shape, as the vocabulary's** — [`blend_mode`]'s
/// function one control along, and the one place these two lists are made to
/// agree.
///
/// A match, so the day a fourth `MaskKind` lands in the engine this stops
/// compiling rather than reading a shape the vocabulary cannot name into a
/// record that has to name one. The mirror image of it is `view::Mixer::mask`,
/// which turns the console's own word into the same vocabulary — three names
/// for three shapes, which is the cost `karakuri-operation` pays for depending
/// on nothing (P-0074).
fn wipe_kind(kind: MaskKind) -> karakuri_operation::WipeKind {
    match kind {
        MaskKind::None => karakuri_operation::WipeKind::None,
        MaskKind::Linear => karakuri_operation::WipeKind::Linear,
        MaskKind::Radial => karakuri_operation::WipeKind::Radial,
    }
}

/// **What a frame has to fit in on this window**: the display's refresh
/// interval, in milliseconds — the `/16.6` in the mock's transport, at the
/// 60 Hz it was drawn against.
///
/// **It is the refresh interval because that is what this window is held to.**
/// The surface is `PresentMode::Fifo`, so a frame that takes longer than one
/// interval to build is a frame that misses a vsync, and every millisecond
/// under it is the headroom the mock's own tooltip is about. `Cost::wait` is
/// the other side of the same number: at 60 Hz most of the frame is spent
/// blocked in `get_current_texture` waiting for it.
///
/// **It is not `karakuri_engine`'s `DEFAULT_BUDGET_MS`**, which is 20 and is a
/// different budget with the same word on it: that one is what a *candidate
/// Set* has to hold to survive a hot swap, measured offscreen at a fixed size
/// and judged on a median. The mock's tooltip runs the two together — *"12.4
/// of 16.6 — there is headroom. A candidate that cannot hold this is rolled
/// back on its own"* — and they are two numbers. This row draws the one the
/// frame is actually against.
///
/// `None` where `winit` will not say, which is a monitor it cannot name or a
/// mode with no refresh rate on it. The row then draws the frame time and no
/// budget, rather than a plausible 16.6 nothing measured.
///
/// **Read once, when the window opens.** A window dragged onto a 120 Hz
/// display keeps the interval it opened on, which is a real limitation and is
/// the price of not asking the platform for a monitor handle sixty times a
/// second.
fn budget_ms(window: &Window) -> Option<f32> {
    let millihertz = window.current_monitor()?.refresh_rate_millihertz()?;
    match millihertz > 0 {
        true => Some(1.0e6 / millihertz as f32),
        false => None,
    }
}

/// A `.kir` off disk, parsed and checked — the two stages `Set::build` wants a
/// `Checked` from, and no more. `karakuri-cli`'s `compile::load` is the same
/// two with a cost estimate and a source it keeps; neither is wanted here.
/// **How many elements the geometry runs at**, which is the L1's own
/// declaration and not a number written here.
///
/// `capacity [min, max] = default` is in the file and `Set::build` takes a
/// number, so somebody has to read one across. This used to be a
/// `const CAPACITY: u32 = 262144` — `drift_shell.kir`'s declared default,
/// transcribed, which was fine while that was the only file this could load and
/// silently wrong the moment it took a path: a procedure written for 131072
/// would have run at 262144 and nothing would have said so.
///
/// **The fallback is not the answer, it is the arm that cannot happen.**
/// `karakuri_ir::DEFAULT_CAPACITY` is what is left when *nothing* declared one,
/// and `check_header` requires a `capacity` on every L1 — so a `Checked` that
/// passed always carries one and this `map_or` is the shape of the seam type
/// rather than a decision. Pointing the whole thing at `DEFAULT_CAPACITY` would
/// run every procedure at 262144 whatever it declared, which is the defect
/// `the_capacity_is_the_l1s_own_declaration_and_the_l4_declares_none` is run
/// against.
///
/// This is `karakuri-cli`'s `capacity_for` with no `--capacity` to override it,
/// and `karakuri_environment::watch`'s rebuild is the same line again. There is
/// no `--capacity` here on purpose: see [`USAGE`].
fn capacity_of(l1: &karakuri_ir::typed::Checked) -> u32 {
    l1.capacity
        .map_or(karakuri_ir::DEFAULT_CAPACITY, |declared| declared.default)
}

/// A rectangle in logical pixels, in physical ones. **Both of the engine's
/// textures are sized from this and from nothing else** — the picture from its
/// region, deck A's preview from its cell — which is what makes each of them
/// the size of what it is drawn into rather than of the window. Which
/// rectangle each one gets is [`aims`]; this is the *at what size* half, and
/// [`Presented::aim`] is the one place the two meet.
fn physical(rect: egui::Rect, scale: f32) -> (u32, u32) {
    (
        ((rect.width() * scale).round() as u32).max(1),
        ((rect.height() * scale).round() as u32).max(1),
    )
}

// ---------------------------------------------------------------------------
// The window
// ---------------------------------------------------------------------------

/// What to do about a frame that could not be acquired.
///
/// **Every outcome gets a decision, because ignoring one is invisible.** This
/// loop waits for events rather than spinning, so a `return` that neither
/// reconfigures nor asks for another frame is a window that stops drawing and
/// never starts again — and there is nothing on screen or on stdout to say
/// why. `karakuri-cli`'s window sink carries the same table with the same
/// argument, and says that returning silently is what left it with a frozen
/// window once already.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Missed {
    /// The swapchain needs remaking: reconfigure, then ask for another frame.
    Remake,
    /// Ordinary jitter. Ask again.
    Again,
    /// Nobody can see the window. Doing nothing is right — it is the OS that
    /// says when it is back, and asking for frames meanwhile is a spin.
    Idle,
    /// Not self-correcting, and not something a retry mends. Say so.
    Fault,
}

/// `None` where a texture was handed over; a decision for every other case.
fn missed(outcome: &wgpu::CurrentSurfaceTexture) -> Option<Missed> {
    match outcome {
        // `Suboptimal` draws correctly and asks to be reconfigured for
        // performance, which the next resize does anyway.
        wgpu::CurrentSurfaceTexture::Success(_) | wgpu::CurrentSurfaceTexture::Suboptimal(_) => {
            None
        }
        wgpu::CurrentSurfaceTexture::Lost | wgpu::CurrentSurfaceTexture::Outdated => {
            Some(Missed::Remake)
        }
        wgpu::CurrentSurfaceTexture::Timeout => Some(Missed::Again),
        wgpu::CurrentSurfaceTexture::Occluded => Some(Missed::Idle),
        wgpu::CurrentSurfaceTexture::Validation => Some(Missed::Fault),
    }
}

struct Gfx {
    window: Arc<Window>,
    gpu: Gpu,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    egui: egui_winit::State,
    renderer: egui_wgpu::Renderer,
    /// **The engine, on the same device as the panel.** It lives beside the
    /// renderer rather than beside the model because everything in it takes a
    /// device: that is the seam `karakuri-console` keeps, and this is the side
    /// of it that is allowed one.
    engine: Engine,
    /// **The room this window is listening to**, or `None` for a machine with
    /// no input — see [`listening`], where all three of that decision's cases
    /// are argued.
    ///
    /// It is here beside the engine rather than on [`App`] because the two
    /// halves of what it is for are both here: the deck's signal bus is what a
    /// measurement is written into, and the *output lag* a beat correction
    /// leads by is this display's frame queue. A window remade is a display
    /// remade, and the input is re-opened with it.
    audio: Option<audio::Audio>,
    /// **What a frame has to fit in on this window**, read from the display
    /// once when the window opened — see [`budget_ms`].
    budget_ms: Option<f32>,
    /// **What the mixer strip calls what each slot is playing**, in slot
    /// order — see [`Sources::material`]. Kept rather than recomputed because
    /// a name is a string and the frame path is budgeted.
    ///
    /// **One per slot rather than one for the deck**, and the difference only
    /// began to matter when a load did. Both slots open on the pair this
    /// program was launched with, so one name was every slot's name and could
    /// not become wrong; `l` moves one slot's material and leaves the other
    /// where it was, and a single name would then have both strips reading the
    /// launch pair with the picture showing something else. That is a readout
    /// that is wrong and silent, which is the one thing P-0027 refuses.
    ///
    /// Rewritten where the slot is: [`played`], on the press, which is also
    /// where the store is read. Nothing on the frame path touches it.
    material: Vec<String>,
    /// **Where the library is**, copied from [`App::store`] when the device
    /// was made.
    ///
    /// It is on both because the two readers are on both sides of the window:
    /// `resumed` lists the bay before there is a `Gfx` at all, and [`played`]
    /// and `performed` reach a disk on a key press and are handed nothing but
    /// this. A `Path::new(STORE)` at each of those four call sites is what
    /// this replaces, and the reason it can no longer be one is the whole of
    /// the change — the directory is a thing the operator said, so it has to
    /// be carried from where they said it.
    store: std::path::PathBuf,
}

struct App {
    gfx: Option<Gfx>,
    /// **What the command line asked for**, read before the event loop starts
    /// and used once, in `resumed`. It is here rather than in [`Gfx`] because
    /// it is known before there is a device and outlives every remake of one.
    ///
    /// **The pair the operator named, and not what any deck runs from** — see
    /// [`running`](App::running). What this answers is the strips' name, which
    /// is a question about what was asked for: four decks opened on one preset
    /// are playing that preset, whatever their four files are called.
    sources: Sources,
    /// **The working copy each deck runs from**, one pair per slot, in slot
    /// order — [`working_copies`], made in [`main`] before the window.
    ///
    /// Beside `sources` rather than replacing it because the two answer
    /// different questions and always have: this is what a watcher polls and
    /// what an editor opens, and `sources` is what the operator said. They were
    /// one field while every slot watched the typed paths, which is the defect
    /// this pair of fields exists to end.
    running: Vec<Sources>,
    /// **Where the library is** — see [`Launch::store`]. Here for `sources`'
    /// reason, and copied onto [`Gfx::store`] for the readers that are handed
    /// only a device.
    store: std::path::PathBuf,
    /// **The preset library this run resolved**, kept for one purpose: the
    /// legend says which of the places answered, and it says it by printing
    /// what the resolution returned rather than a sentence about what it
    /// probably did. `None` is a machine with no library, which reaches this
    /// far only on a run that was given its pair by hand.
    presets: Option<karakuri_environment::places::Presets>,
    /// A validation fault is said once rather than sixty times a second.
    faulted: bool,
    readout: Readout,
    costs: Costs,
    /// Logical size, so the numbers printed are the arrangement's own units
    /// rather than the display's.
    scale: f64,
    /// **When `egui` asked to be drawn again, kept as the deadline it is.**
    ///
    /// `egui` animates, blinks a text cursor and fades a tooltip in, and it
    /// says so as a `repaint_delay` on the frame's `ViewportOutput`. Turning
    /// that into an immediate `request_redraw` would turn a 250 ms animation
    /// into a spin at whatever rate this loop can manage — which is the cost
    /// P-0072's first clause is about, arrived at from the one direction that
    /// looks like obeying it. So the delay is added to the clock here and
    /// `about_to_wait` sleeps until it.
    ///
    /// `None` where `egui` asked for nothing, which is every frame on a panel
    /// with nothing on it.
    egui_due: Option<Instant>,
    /// **The origin every animation on the panel is measured from**, and the
    /// only clock behind `view::Phase`.
    ///
    /// It is here because this file owns the window and the clock and `src/`
    /// owns neither — every `Instant::now` in this crate is in this file, and
    /// `view::Phase` is a `Duration` for exactly that reason (P-0002). What
    /// crosses the seam is `now - this`, which is a number.
    ///
    /// **Where the origin is does not matter**, which is why it is taken at
    /// construction rather than when something first starts moving: every
    /// presentation is periodic in the phase, so an origin the operator did
    /// not choose is an origin nobody can see. What would matter is having
    /// *two*, and there is one.
    started: Instant,
    /// **When a served run next wakes to take what a model asked for**, and
    /// `None` for a run without `--mcp` — see [`SERVED`], which is the whole of
    /// why this deadline exists.
    ///
    /// A deadline beside `egui_due` rather than a `ControlFlow::Poll`, because
    /// this loop has one rule about when it runs and a second one would be a
    /// second answer to it: [`App::about_to_wait`] takes the soonest of what is
    /// owed, and this is one of the things owed.
    served: Option<Instant>,
    /// The sending half of [`watch::Watch::storing_to`]'s channel, handed to
    /// every watcher [`Engine::new`] makes. Kept because a window remade makes
    /// them again.
    built_tx: std::sync::mpsc::Sender<watch::Built>,
    /// **The store the watchers put their builds in**, opened once in [`main`].
    ///
    /// An open store rather than the root beside it, because this one is shared
    /// with four worker threads and each of them writes to it on every build.
    /// [`App::store`] is still the root, and is still what a save, a listing and
    /// an arrangement are handed: those open per call, which is what keeps a
    /// listing from creating a directory it only wanted to read.
    held: std::sync::Arc<Store>,
    /// **Everything a save and a rewiring need that is not the deck** — see
    /// [`Keeping`].
    keeping: Keeping,
}

/// **What this run holds so that a deck can be kept, and rewired.**
///
/// One value rather than seven fields on [`App`], because the seven move
/// together and every one of them is read by the same three moments: a request
/// arriving, a build landing, and a save coming back off the disk. It is also
/// what makes those three reachable at all — the window loop binds `gfx` out of
/// `self.gfx` and holds it for the length of the handler, so a method on `App`
/// could not be called there. This is the piece that is passed instead.
struct Keeping {
    /// **The server's half of the channel, when `--mcp` asked for one**, and the
    /// whole of what a model reaches this program through.
    ///
    /// Told what the swap machinery said, handed what a client asked the render
    /// loop for, and nothing else — see [`karakuri_environment::mcp`]. It is
    /// bound in [`main`], before the window, for the reason the working copies
    /// are made there: `serve` binds a socket and can fail, and a failure has to
    /// be a sentence on a terminal rather than a panic inside a `winit`
    /// callback, where it aborts with no message at all.
    mcp: Option<mcp::Reporter>,
    /// **The run's wiring** — every edge a `wire_input` has written, for the
    /// whole run and not per slot.
    ///
    /// One list because `--edge` is one list: an edge names the node that
    /// declares the input and what its procedure calls it, and a Set that has
    /// not got that node passes it over where it is built. See [`rewired`].
    ///
    /// **What a rebuild carries and what a save records**, which is why it lives
    /// here rather than inside a watcher: [`Aiming::re_aim`] restates it to the
    /// worker and [`playing_values`] writes it into the file, and those are one
    /// list or they are two answers to what the run is wired with.
    edges: Vec<karakuri_engine::set::Edge>,
    /// **What each deck is playing**, seeded before the first frame and moved by
    /// every build that lands — see [`Playing`].
    playing: Playing,
    /// Where the watchers report what they built and stored — the other end of
    /// [`watch::Watch::storing_to`], drained where a build lands.
    built: std::sync::mpsc::Receiver<watch::Built>,
    /// **Builds reported but not yet landed**, kept by their build id.
    ///
    /// The two arrive on two channels and in either order: a watcher stores a
    /// build on its worker thread and the swap lands at a frame boundary some
    /// frames later, so a report that came in before its `Swapped` has to wait
    /// somewhere. **Removed when it lands**, so a build that was refused or that
    /// the deck never took leaves nothing behind — there is at most one
    /// outstanding build per slot, which is what `HotSwap` allows.
    pending: Vec<watch::Built>,
    /// Where a save that has reached the disk comes back, and the sending half
    /// each save thread is given a clone of.
    saves: std::sync::mpsc::Receiver<Saved>,
    save_tx: std::sync::mpsc::Sender<Saved>,
    /// How many saves are being written right now. The run waits for these once,
    /// at the end and under a bound — see [`Keeping::awaited_saves`].
    in_flight: usize,
}

impl Keeping {
    /// **What a model has asked for since the last frame.**
    ///
    /// `karakuri-cli`'s `Live::run_requests` is this function, and it is at the
    /// top of the frame for its reason: a surface is polled once and every
    /// request it produces ends in the method a key press ends in. That is what
    /// makes `--mcp` a second pair of hands rather than a second way to do
    /// anything.
    ///
    /// **Collected out of the borrow before any of it is acted on**, because
    /// both arms below take `&mut self`. Nothing is allocated on a frame that
    /// was asked for nothing: collecting an empty iterator makes no allocation.
    ///
    /// **Both channels drained before either is acted on**, for that reason and
    /// for a second one: they are two queues by design — see
    /// `mcp::Reporter::wires` — so a deck being saved to a slow disk cannot
    /// delay a rewiring, and taking them in one pass is what keeps that true on
    /// this side too.
    fn requests(&mut self, engine: &mut Engine, root: &std::path::Path) {
        let Some(mcp) = &self.mcp else {
            return;
        };
        let asked: Vec<mcp::SaveRequest> = mcp.saves().collect();
        let wires: Vec<mcp::WireRequest> = mcp.wires().collect();
        for request in asked {
            self.save_set(engine, root, request.slot, request.id, Some(request.reply));
        }
        self.rewire(engine, wires);
    }

    /// **Every edge asked for since the last frame, written and answered here,
    /// on this frame.**
    ///
    /// The decisions are [`rewired`]'s and are written there, because none of
    /// them needs a device. What is here is the two things that do: the deck's
    /// own slot count, which is the only thing that knows how many slots there
    /// are, and the answer going back to whoever asked.
    ///
    /// **Answered once, at the frame it was applied on**, which is
    /// `mcp::WireRequest`'s third point. Not at the swap: what the *build* made
    /// of the edge is `swap_outcome`'s answer, as it is for every other rebuild,
    /// and a tool that waited for thirty judged frames would hold a connection
    /// open across a transition.
    ///
    /// **One sentence for both audiences**, which is [`refused`]'s rule: what
    /// the terminal is told and what the client is handed are the same words, so
    /// the second cannot be right on the day it is written and wrong at the next
    /// correction.
    fn rewire(&mut self, engine: &mut Engine, asked: Vec<mcp::WireRequest>) {
        if asked.is_empty() {
            return;
        }
        let mut wires = Vec::with_capacity(asked.len());
        let mut replies = Vec::with_capacity(asked.len());
        for mcp::WireRequest { slot, edge, reply } in asked {
            wires.push((slot, edge));
            replies.push(reply);
        }
        let said = rewired(
            &wires,
            &mut self.edges,
            &mut engine.aimed,
            engine.deck.slot_count(),
        );
        for (reply, said) in replies.into_iter().zip(said) {
            match &said {
                Ok(line) | Err(line) => println!("{line}"),
            }
            reply.settled(said);
        }
    }

    /// **Write what a deck is playing as a Set file.**
    ///
    /// `karakuri-cli`'s `Live::save_set` is the same method and says of itself
    /// that it is *the only save path* in that program; this is that path in
    /// this one, and every surface here ends in it — the `k` key, the MCP tool,
    /// and whatever control the Library bay grows. That is what makes a refusal
    /// and an outcome one sentence each rather than one sentence per surface.
    ///
    /// **The slot is an argument and the key passes the selected deck in.** A
    /// Set file describes one Set and this deck holds four; the one an
    /// *operator* means is the deck they have already selected, and a model has
    /// no selection and names the slot as it names one to read a procedure.
    ///
    /// **`id` is what the caller wanted it called, or a stamp.** A key press
    /// cannot type a name, so it passes `None`; see
    /// [`karakuri_environment::accepted_save`], whose convention that is.
    ///
    /// **Read off the live Set, not off the command line.** Every number a Set
    /// file carries can have moved since this run started — a param through a
    /// record, a capacity or a salt through a rebuild — so the only reading that
    /// cannot be stale is the Set's own. See [`playing_values`].
    ///
    /// **Gathered here, written elsewhere.** Everything up to the spawn is a
    /// read off values already in memory; the store I/O goes to a thread of its
    /// own — one per save, since saves are rare and a pool would be machinery
    /// for a rate of a few an hour. The outcome comes back over `saves` and is
    /// said at the frame it arrives, not at this press.
    fn save_set(
        &mut self,
        engine: &Engine,
        root: &std::path::Path,
        slot: usize,
        id: Option<String>,
        reply: Option<mcp::Reply>,
    ) {
        // **Checked here rather than only where the request came from.** A key
        // press cannot name a slot this deck does not hold and a tool call can,
        // and below this line `playing_values` reads `deck.slot(slot)`, which
        // indexes. The server refuses it too, in its own words, so a model never
        // reaches this — and this is the guard that does not depend on it
        // having.
        let count = engine.deck.slot_count();
        if !slot_in_range(slot, count) {
            return refused(reply, karakuri_environment::no_such_slot(slot, count));
        }
        let Some(nodes) = self.playing.at(slot) else {
            // **The only way to reach this in this program**: a build landed
            // whose sources the store would not take, which the watcher said at
            // the time. Every slot is seeded at launch, so a slot that has never
            // rebuilt always has an address.
            return refused(
                reply,
                karakuri_environment::nothing_to_save(slot, None, false),
            );
        };
        let sources = setfile::Sources(nodes.iter().map(copied).collect());
        if sources.is_empty() {
            return refused(
                reply,
                karakuri_environment::nothing_to_save(slot, None, true),
            );
        }
        // **Named and answered above the line that needs a deck**, which is
        // where the whole of `accepted_save`'s doc lives: `playing_values` below
        // is the one read here that needs one, and the accept has to be on the
        // side of it a test can reach.
        let id = karakuri_environment::accepted_save(slot, id, &sources, root, reply.as_ref());
        let values = playing_values(engine.deck.slot(slot).set(), &self.edges);
        let save = Save {
            slot,
            id,
            root: root.to_path_buf(),
            sources,
            values,
        };
        let tx = self.save_tx.clone();
        // **A thread per save**, and detached: no frame waits for it. The *run*
        // waits, once, at the end and under a bound — see
        // [`Keeping::awaited_saves`], which is what this count is for.
        self.in_flight += 1;
        std::thread::spawn(move || {
            let (slot, id) = (save.slot, save.id.clone());
            // **Carried back rather than answered from here.** This thread knows
            // the outcome and could say it, and that would be a second place a
            // save is reported from.
            let outcome = save.run();
            let _ = tx.send(Saved {
                slot,
                id,
                outcome,
                reply,
            });
        });
    }

    /// **Every save that has landed since the last frame, said and answered.**
    ///
    /// Drained and never waited on: a frame owes the display a picture and owes
    /// a disk nothing. Called at the top of the frame beside
    /// [`Keeping::requests`], because a window that has faulted still has saves
    /// finishing behind it and a run that told nobody about them until it quit
    /// would be withholding the one answer a waiting client cannot get anywhere
    /// else.
    ///
    /// Returns whether any of them landed, which is what the Library bay's
    /// listing is re-read on: a save is the only thing in this program that adds
    /// a Set, and a bay that did not list it would be a readout that is wrong
    /// and silent.
    fn finished_saves(&mut self) -> bool {
        let mut landed: Vec<Saved> = Vec::new();
        while let Ok(saved) = self.saves.try_recv() {
            landed.push(saved);
        }
        let mut written = false;
        for saved in landed {
            written |= self.took_save(saved);
        }
        written
    }

    /// One save's outcome, said and answered.
    ///
    /// **One sentence for both audiences.** What the terminal is told and what a
    /// waiting client is handed are the same fact, so the words are formed once
    /// here and the client gets the ones the operator got. The `Ok`/`Err` split
    /// is what a tool call's `isError` is built from — see `mcp::Reply::settled`.
    fn took_save(&mut self, saved: Saved) -> bool {
        let Saved {
            slot,
            id,
            outcome,
            reply,
        } = saved;
        self.in_flight = self.in_flight.saturating_sub(1);
        let (written, said) = match outcome {
            Ok(()) => {
                let said = format!(
                    "  keep: deck {}: saved as set `{id}` — the Library bay's `my sets` lists \
                     it, and `l` loads it back",
                    deck_letter(slot as u8)
                );
                println!("{said}");
                (true, Ok(said))
            }
            // **Printed, and nothing claiming otherwise.** See `Saved::outcome`.
            Err(e) => {
                let said = format!(
                    "  keep: deck {}: set `{id}` was not saved: {e}",
                    deck_letter(slot as u8)
                );
                println!("{said}");
                (false, Err(said))
            }
        };
        if let Some(reply) = reply {
            reply.settled(said);
        }
        written
    }

    /// **Take up what a slot is now playing.**
    ///
    /// `landed` is the build id when a swap went in, or `None` when one was
    /// rolled back — which is the case [`Playing::previous`] exists for. A
    /// rollback brings back a Set nothing will name again, so the only way to
    /// say what came back is to have remembered it.
    ///
    /// The `built` channel is drained here rather than per frame: it only has
    /// anything in it when a build has just been requested, and this runs when
    /// one has just landed.
    fn took_up(&mut self, engine: &Engine, slot: usize, landed: Option<u64>) {
        let mut ready: Vec<watch::Built> = Vec::new();
        while let Ok(built) = self.built.try_recv() {
            ready.push(built);
        }
        for built in ready {
            self.pending.push(built);
        }
        match landed {
            // **Missing means the watcher could not store this build's
            // sources**, which it said at the time. Handed to `landed` as `None`
            // rather than returned on, because the swap happened either way: a
            // slot that took a version nobody can name is a slot with no
            // address, not a slot still on its old one.
            Some(id) => {
                let at = self.pending.iter().position(|built| built.id == id);
                let built = at.map(|at| self.pending.remove(at));
                let nodes = built
                    .as_ref()
                    .zip(engine.aimed.get(slot))
                    .map(|(built, aiming)| built_nodes(built, &aiming.at));
                self.playing.landed(slot, nodes);
            }
            None => self.playing.rolled_back(slot),
        }
    }

    /// **Every save still being written, waited for — up to
    /// [`karakuri_environment::SAVE_WAIT`].**
    ///
    /// A frame owes the disk nothing, which is why [`Keeping::finished_saves`]
    /// drains and never blocks. The end of the run is the one moment where that
    /// is the wrong trade: a save asked for in the last second reached the disk
    /// under an id nobody was ever told, so the client that asked for it waits
    /// out its deadline and the operator is told nothing at all.
    ///
    /// **Bounded, because a disk can hang and quitting must not depend on one.**
    /// Past the bound the run says how many saves it left behind and exits.
    ///
    /// The wait is only ever paid by a run that saved and quit within a few
    /// frames; the count is zero for every other run and this returns without
    /// blocking.
    fn awaited_saves(&mut self) {
        self.finished_saves();
        if self.in_flight == 0 {
            return;
        }
        println!(
            "waiting up to {:.0}s for {} save{} still being written",
            karakuri_environment::SAVE_WAIT.as_secs_f32(),
            self.in_flight,
            match self.in_flight {
                1 => "",
                _ => "s",
            }
        );
        let deadline = Instant::now() + karakuri_environment::SAVE_WAIT;
        while self.in_flight > 0 {
            let Some(left) = deadline.checked_duration_since(Instant::now()) else {
                break;
            };
            // Timed out, or every sender is gone and nothing more can arrive.
            // Either way there is nothing left to wait for.
            let Ok(saved) = self.saves.recv_timeout(left) else {
                break;
            };
            self.took_save(saved);
        }
        if self.in_flight > 0 {
            println!(
                "  {} save{} still unfinished after {:.0}s — each is written or it is not, and \
                 nothing here claims either way",
                self.in_flight,
                match self.in_flight {
                    1 => "",
                    _ => "s",
                },
                karakuri_environment::SAVE_WAIT.as_secs_f32(),
            );
        }
    }
}

impl App {
    /// **The opening is handed in rather than made here**, which is the whole of
    /// what pairing the four pills with a server took: [`main`] gives the same
    /// handle to [`karakuri_environment::mcp::serve`] and to this, so a press on
    /// a bay head and the class the server reads are one value. [`Readout::new`]
    /// makes one of its own — it is constructed from a size and nothing else —
    /// and this replaces it before the window opens, which is before anything
    /// can read either.
    fn new(
        launch: Launch,
        running: Vec<Sources>,
        held: std::sync::Arc<Store>,
        mcp: Option<mcp::Reporter>,
        opening: Opening,
    ) -> App {
        let mut readout = Readout::new(WINDOW.0 as f32, WINDOW.1 as f32);
        // **The one handle, and it lives on the readout because that is where
        // the pills reach it.** A copy kept on [`App`] as well would be a second
        // answer to what is open the day one of them was written and the other
        // was not.
        readout.view.opening = opening.read();
        readout.opening = opening;
        let (built_tx, built) = std::sync::mpsc::channel();
        let (save_tx, saves) = std::sync::mpsc::channel();
        App {
            gfx: None,
            sources: launch.sources,
            running,
            store: launch.store,
            presets: launch.presets,
            faulted: false,
            readout,
            costs: Costs::new(),
            scale: 1.0,
            egui_due: None,
            // **Due at once on a served run**, so the first thing a model asks
            // for is taken on the first wake rather than a tenth of a second
            // after it.
            served: mcp.is_some().then(Instant::now),
            started: Instant::now(),
            built_tx,
            held,
            keeping: Keeping {
                mcp,
                edges: Vec::new(),
                // **Empty until there is a deck**, because the launch nodes are
                // what the engine's compile produced and there is no engine
                // before `resumed`. It is seeded there, off [`Engine::placed`],
                // on the same pass that makes the watchers.
                playing: Playing {
                    playing: Vec::new(),
                    previous: Vec::new(),
                },
                built,
                pending: Vec::new(),
                saves,
                save_tx,
                in_flight: 0,
            },
        }
    }

    /// Hand an event to `egui`, and nowhere else.
    ///
    /// Every call site has already asked `claim` where a pointer event
    /// belongs; this is the other branch. `EventResponse::repaint` is `egui`'s
    /// own answer for the event it was just given, and it is the reason
    /// `Change::Pointer(Claim::Egui)` asks for nothing: one answer per event,
    /// from whoever got it.
    fn to_egui(gfx: &mut Gfx, costs: &mut Costs, event: &WindowEvent) {
        let response = gfx.egui.on_window_event(&gfx.window, event);
        if response.repaint {
            costs.owes();
            gfx.window.request_redraw();
        }
    }

    /// **Perform what a pointer event asked for, and say what frame it is
    /// owed.** `otherwise` is the answer for an event that acted on nothing —
    /// the claim's, which is the answer this loop had before there were
    /// controls.
    ///
    /// The two kinds of control end in two different places, which is what
    /// [`Acted`] is for:
    ///
    /// - The Outputs dot's operation was already performed by the panel, and
    ///   what is owed is what the [`Outcome`] says happened.
    /// - **A fader's operation is performed here**, because it is the *deck*
    ///   that moves and the panel has no deck (ADR-0156). It becomes a record
    ///   and the record moves the deck — P-0028, which is what makes this
    ///   fader the same control as a key press and a MIDI knob rather than a
    ///   third way of writing a gain. The next frame's strips are read back off
    ///   the deck by [`mixer`], so what the fader shows is what the deck says
    ///   and never what this loop remembered.
    ///
    /// **What the operation becomes is [`written`]'s answer and not this
    /// file's**, out of the operation and what [`reading`] read off the deck.
    /// It used to be a `match` written out here, because there was
    /// nowhere for the conversion to live; ADR-0185 said that function is
    /// deleted the day a home lands, and
    /// [ADR-0194](../../../docs/adr/0194-where-an-operation-becomes-a-record-is-a-crate-that-depends-on-both.md)
    /// is that home. Three answers come back and all three are said out loud —
    /// the records go to [`apply`], and the other two go to [`unwritten`],
    /// which is the difference between *this press writes nothing, and that is
    /// settled* and *this press owes a record nobody has decided how to
    /// write*.
    fn performed(
        gfx: &mut Gfx,
        readout: &mut Readout,
        acted: &Acted,
        otherwise: Repaint,
    ) -> Repaint {
        match acted {
            Acted::Nothing => otherwise,
            // **A class pill earns the frame the claimed press already earns,
            // and no more.** Nothing in the arrangement moved and no operation
            // was emitted; what changed is one word in one capsule, and
            // `Change::Pointer(Claim::Panel)` — which is what `otherwise` is on
            // every path that can reach this arm — is already `Repaint::Now`. A
            // `Change` of its own would be a second answer to a question that
            // is already answered.
            Acted::Opened => otherwise,
            Acted::Operated(outcome) => Change::Operated(outcome).repaint(),
            Acted::Emitted(operation) => {
                if let Some(operation) = operation.as_ref() {
                    // **The two operations that reach a disk**, and they are
                    // taken first because they are not about the deck at all:
                    // an arrangement is the console's own state, `written`
                    // answers `Silent(Surface)` for both, and what they change
                    // is the panel and a file under the store. Everything
                    // below this is the mix. See [`arrangement`], which
                    // answers `None` for every other operation and is why this
                    // is one line rather than a second route into the panel.
                    if let Some(line) = arrangement(
                        &gfx.store,
                        &mut readout.panel,
                        &mut readout.view.arrangement,
                        operation,
                    ) {
                        println!("{line}");
                    }
                    // **The two the console performs itself**, and they are
                    // here for the same reason the arrangement is: both write
                    // no record, so `written` below answers `Silent` for them
                    // and there is nothing for `apply` to do. One moves a
                    // pointer this crate does not hold, the other re-points a
                    // slot's source and lets the worker do the rest — neither
                    // is the mix. Each answers `None` for every other
                    // operation, which is what keeps this two lines rather
                    // than two more routes into the engine.
                    // **The one that reaches a device**, and it is here
                    // beside the arrangement for the same reason: it writes no
                    // record either — `written` answers `Silent(NoRecord)`,
                    // because nothing in the session stream says what the beat
                    // is taken from — and what it changes is this program's
                    // audio session and the pill that reads it. The session
                    // tempo is read first so that the borrow of `gfx.audio`
                    // below does not have to hold the deck as well.
                    let session_bpm = gfx.engine.deck.signals().oscillator().bpm();
                    if let Some(line) = attached(
                        &mut gfx.audio,
                        session_bpm,
                        &mut readout.view.audio,
                        operation,
                    ) {
                        println!("{line}");
                    }
                    // **The second that reaches that device**, and it is here
                    // for the reason the attach is: `written` answers
                    // `Silent(NoRecord)` for it too, so there is nothing for
                    // `apply` to do and the session this program opened is the
                    // only thing that holds the value. See [`nudged`].
                    if let Some(line) = nudged(&mut gfx.audio, operation) {
                        println!("{line}");
                    }
                    if let Some(line) = pointed(&mut readout.view, operation) {
                        println!("{line}");
                    }
                    if let Some(line) = played(gfx, operation) {
                        println!("{line}");
                    }
                    // **The reading is taken off the deck, and for four of the
                    // five controls it is *I read nothing*.** A gain, an
                    // opacity, a blend mode and a residency carry everything
                    // their record carries, so a reading handed in for one of
                    // them would be this file inventing a value — which is
                    // what ADR-0194 refuses a default for. The mask mini is
                    // the fifth and its record is written whole out of two
                    // halves (ADR-0201), so its half is read here rather than
                    // assumed. See [`reading`].
                    let written = written(
                        operation,
                        &reading(operation, &gfx.engine.deck, &gfx.engine.look),
                    );
                    // **A press that wrote no record says so**, and says
                    // which of the two kinds of nothing it was, before
                    // anything is applied.
                    if let Some(line) = unwritten(operation, &written) {
                        println!("{line}");
                    }
                    // **Every record, in the order it was written.** One
                    // today for each of the four, and a list because
                    // `Crossfade` is four and `Wipe` is five — one control is
                    // not one record (P-0028, ADR-0194).
                    if let Written::Records(records) = &written {
                        for record in records {
                            if let Some(line) =
                                apply(record, &mut gfx.engine.deck, &mut gfx.engine.look)
                            {
                                println!("{line}");
                            }
                        }
                        // **A scrub moves a value an Inspector pane is
                        // drawing, and the panes are read once for the run.**
                        // `Set::published` allocates and says it is not for
                        // the frame path, so [`inspector`] is called at
                        // startup and the anchor's `B128 +0.25` would go on
                        // reading the scrub the deck had when the window
                        // opened — a picture of a value that has moved, which
                        // is exactly what P-0027 is about. It is re-read here,
                        // on the press that moved it: a press is where this
                        // file already reads a directory, and it is not a
                        // frame.
                        //
                        // **Off the record rather than off the operation**,
                        // because what matters is that the deck's transport
                        // changed — the day a second operation writes one, it
                        // is caught by the same line.
                        if records
                            .iter()
                            .any(|record| matches!(record, Record::Transport { .. }))
                        {
                            inspector(&gfx.engine.deck, &gfx.material, &mut readout.view.inspector);
                        }
                    }
                }
                Change::Emitted(operation.as_ref()).repaint()
            }
        }
    }

    /// **Act on a repaint decision, and the only place a frame is asked for
    /// outside `to_egui` and `missed`.**
    ///
    /// Three answers and three actions: ask for a frame, note a deadline, or
    /// do nothing at all — and the third is the one P-0072's first clause is
    /// made of.
    ///
    /// It takes the two fields rather than `&mut self` so that a caller
    /// holding `self.gfx` can still reach `self.egui_due`.
    fn wants(gfx: &Gfx, egui_due: &mut Option<Instant>, costs: &mut Costs, repaint: Repaint) {
        match repaint {
            Repaint::Never => {}
            Repaint::Now => {
                costs.owes();
                gfx.window.request_redraw();
            }
            Repaint::After(delay) => {
                let due = Instant::now() + delay;
                *egui_due = Some(match *egui_due {
                    Some(had) => had.min(due),
                    None => due,
                });
            }
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.gfx.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title("The Karakuri console")
            .with_inner_size(winit::dpi::LogicalSize::new(WINDOW.0, WINDOW.1));
        let window = Arc::new(event_loop.create_window(attrs).expect("window"));
        let size = window.inner_size();
        self.scale = window.scale_factor();

        let instance = Gpu::instance();
        // **Reported rather than panicked, and both of these can fail for one
        // reason.** A panic here is reached from a `winit` callback and cannot
        // unwind across the Objective-C frame on macOS, so it aborts — with
        // `<unknown>` for every frame of the backtrace and no sentence
        // anywhere saying what went wrong.
        //
        // The surface is the one that goes first when a backend was asked for
        // and the machine has none of it: an instance with only that backend
        // enabled has nothing that can make a surface, so the failure arrives
        // as `FailedToCreateSurfaceForAnyBackend` before any adapter is
        // requested. That is the exact path
        // `docs/adr/0168-a-backend-override-is-honoured-because-a-no-op-cannot-be-caught.md`
        // opened, so it names the variable first.
        let surface = match instance.create_surface(window.clone()) {
            Ok(surface) => surface,
            Err(e) => no_gpu(&format!("no surface: {e}")),
        };
        let gpu = match pollster::block_on(Gpu::from_instance(instance, Some(&surface))) {
            Ok(gpu) => gpu,
            Err(e) => no_gpu(&format!("no adapter: {e}")),
        };

        let caps = surface.get_capabilities(&gpu.adapter);
        // **A non-sRGB format, and that is the opposite of what the program
        // this replaces wanted.** `egui`'s own shader encodes: it is told the
        // target is gamma space and writes gamma-encoded texels, so a surface
        // that also encoded on write would encode twice and wash the panel
        // out. P-0064 says sRGB is encoded once at final output, and for this
        // window the toolkit is that output.
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| !f.is_srgb())
            .unwrap_or(caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            color_space: wgpu::SurfaceColorSpace::Auto,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&gpu.device, &config);

        let ctx = egui::Context::default();
        let egui = egui_winit::State::new(
            ctx,
            egui::ViewportId::ROOT,
            &window,
            Some(self.scale as f32),
            None,
            Some(gpu.device.limits().max_texture_dimension_2d as usize),
        );
        let renderer =
            egui_wgpu::Renderer::new(&gpu.device, format, egui_wgpu::RendererOptions::default());

        self.readout.panel.set_viewport(
            size.width as f32 / self.scale as f32,
            size.height as f32 / self.scale as f32,
        );

        // The picture's first size is the region's, at the window this opened
        // at — not the window's, and not a guess that the first frame then
        // corrects. **It is the same call the frame makes**: `Engine::new`
        // aims both sinks through `aims`, so this and `RedrawRequested` cannot
        // disagree about which rectangle a texture is sized from.
        let mut renderer = renderer;
        self.readout.panel.solve();
        let mut engine = Engine::new(
            &gpu,
            &mut renderer,
            &self.running,
            self.readout.panel.layout(),
            self.scale as f32,
            Some((std::sync::Arc::clone(&self.held), self.built_tx.clone())),
        );
        // **What every deck is playing, seeded from the compile that just
        // built them**, before a frame has run — see [`Playing::at_launch`].
        // It is written here rather than in [`App::new`] because the nodes are
        // the engine's compile, and it is written on *every* remake for the
        // same reason: a window remade rebuilds the deck from the launch pair,
        // so what each slot is running goes back to what it was seeded with.
        self.keeping.playing = Playing::at_launch(&engine.placed, engine.deck.slot_count());
        // **Before the first frame and before the first strip is written**, so
        // that the panel's first frame draws the deck as it actually is rather
        // than a settled version of it that the second frame corrects.
        let governed = engine.ask_to_prime(&gpu);
        let info = gpu.adapter.get_info();
        self.costs.taken_on = format!(
            "{:?} — {} ({:?})",
            info.backend, info.name, info.device_type
        );

        let budget = budget_ms(&window);
        // **The strips before the legend**, because the legend says how many
        // there are and the answer is the deck's rather than a guess. It is
        // written again on every frame; this is the first one.
        // **Every slot opens on the same pair**, because that is what this
        // program builds them from — one name repeated rather than one name
        // shared, so that a load can move one of them without moving the
        // other's readout. See [`Gfx::material`].
        let material: Vec<String> =
            std::iter::repeat_n(self.sources.material(), engine.deck.slot_count()).collect();
        mixer(&engine.deck, &material, &mut self.readout.view.mixer);
        // **The library before the legend too**, and once for the run: the
        // legend says how many Sets the bay lists, and `library` says why
        // where it is none.
        //
        // **The scopes first, because a listing belongs to one of them.** The
        // console draws the chips it is handed and this program is what can
        // answer them — a store, a told directory, and two that answer nothing
        // yet (`why_nothing`). All four are drawn: a chip is the question, and
        // three of the four questions are ones this program can be asked.
        self.readout.view.scopes = Scope::ALL.to_vec();
        // **And it opens on `my sets`, where the mock marks `favourites`.**
        // The mark says which question is being asked, so the one to open on
        // is the one with an answer — and `favourites` is *"this library
        // filtered"* over a fact nothing keeps, which is the one of the four
        // that could not answer even in principle today. The console refuses a
        // scope it was not handed, so this is asserted rather than assumed.
        assert!(
            self.readout.view.select_scope(Scope::MySets),
            "the console was handed the four scopes and would not mark `my sets`"
        );
        println!(
            "{}",
            listing(&mut self.readout.view, &self.store, self.presets.as_ref())
        );
        // **And the arrangement pill's menu, once for the run**, for
        // `library`'s reason and for one more: this is a directory read, and
        // the only thing that can add a name to it is a save this program
        // performs — which re-reads it there. See `arrangements`.
        self.readout.view.arrangement.filed = arrangements(&self.store);
        // **And the Inspector's panes, before the first frame.**
        // `Set::published` says it is not for the frame path but *is* what a
        // console reads when a Set lands, and every slot is watched — so this
        // is the first of those readings rather than the only one, and the
        // frame handler takes the rest. See `inspector`, which is also where
        // the controls it could not place are reported.
        inspector(&engine.deck, &material, &mut self.readout.view.inspector);
        // **And the Master bay's level, for the strips' reason.** The legend
        // reports what each bay draws by asking the view, so a bay whose level
        // has not been written yet reports itself as having no engine behind
        // it — on a run that has one, and over a fader a hand can take hold
        // of. It is written again on every frame; this is the first.
        self.readout.view.master_out = Some(engine.deck.out());
        // **And the room, before the legend**, because the legend says which
        // input is open and the answer is the host's rather than a sentence
        // here. The session tempo is the deck's own oscillator: it is what the
        // grid free-runs at and where the tracker's octave window starts, and
        // they are one number because they are one statement.
        let (audio, said) = listening(engine.deck.signals().oscillator().bpm());
        println!("{said}");
        // **The pill is told even where nothing opened**, which is the
        // distinction `View::audio` exists to draw: `Some(AudioIn)` with no
        // device is a program that looked and found nothing and draws
        // `audio-in · none`, where `None` would be a console nobody had told
        // and would draw no pill at all — on a program that did look.
        self.readout.view.audio = Some(told(audio.as_ref()));
        self.readout
            .print_legend(budget, &governed, self.presets.as_ref(), &self.store);

        // The first frame is owed to the window appearing, not drawn on a
        // still panel.
        self.costs.owes();
        window.request_redraw();
        self.gfx = Some(Gfx {
            audio,
            budget_ms: budget,
            material,
            store: self.store.clone(),
            window,
            gpu,
            surface,
            config,
            egui,
            renderer,
            engine,
        });
    }

    /// **Where a deadline comes due**, which is the start of every iteration
    /// the loop makes — including the one a `ControlFlow::WaitUntil` woke it
    /// for.
    ///
    /// Both deadlines are checked whatever the [`StartCause`] rather than only
    /// on `ResumeTimeReached`: a wait that is cancelled early by a real event
    /// still has to leave a due deadline serviced, and checking two `Instant`s
    /// costs nothing.
    fn new_events(&mut self, _event_loop: &ActiveEventLoop, _cause: StartCause) {
        let now = Instant::now();
        if self.egui_due.is_some_and(|due| due <= now) {
            self.egui_due = None;
            if let Some(gfx) = self.gfx.as_ref() {
                // One frame, now that the delay `egui` asked for has passed.
                gfx.window.request_redraw();
            }
        }
        // **Only once there is something to describe.** The reading names the
        // workload it was taken over, and that is the run's `.kir` pair rather
        // than a constant — so it is taken when the engine exists, and not
        // before. A deadline that comes due first is not lost: `due()` goes on
        // returning it until the reading is printed.
        if self.costs.due().is_some_and(|due| due <= now) {
            if let Some(gfx) = self.gfx.as_ref() {
                // **The reading names the workload it was taken over**, and
                // that is the whole deck's rather than one slot's — so the
                // slots' names are joined in slot order, and a run where a
                // load has moved one of them says so instead of naming the
                // pair the window opened with.
                let (capacity, material) = (gfx.engine.capacity, gfx.material.join(" / "));
                self.costs.say(capacity, &material);
            }
        }
        // **What a model asked for, taken on the wake it asked to be taken
        // on** — see [`SERVED`], where the whole of this is argued. It is here
        // beside the two deadlines above because it is a third one, and
        // `about_to_wait` is where all three are turned into a control flow.
        if self.served.is_some_and(|due| due <= now) {
            self.served = Some(now + SERVED);
            if let Some(gfx) = self.gfx.as_mut() {
                self.keeping.requests(&mut gfx.engine, &self.store);
                if self.keeping.finished_saves() {
                    println!(
                        "{}",
                        listing(&mut self.readout.view, &self.store, self.presets.as_ref())
                    );
                }
                // **A frame, because a build lands at a frame boundary and
                // nowhere else.** A write a model made is on disk, compiled on
                // a worker and waiting for `Deck::begin_frame`; a run that
                // answered the write and never drew would go on showing what it
                // was showing.
                gfx.window.request_redraw();
            }
        }
    }

    /// **The one place the control flow is set, and it is a deadline or
    /// nothing.**
    ///
    /// `Wait` is a window that costs the machine nothing at all until somebody
    /// touches it, which is P-0072's first clause as the operating system sees
    /// it. `WaitUntil` is the soonest of the three things that are owed at a
    /// time rather than on an event: the frame `egui` asked for after a delay,
    /// the reading `Costs` takes once the window has been still long enough,
    /// and — on a run with `--mcp` — the wake that takes what a model asked for
    /// ([`SERVED`]). None is `Poll`, and nothing here asks for a frame in order
    /// to have something to measure.
    ///
    /// **The third one is the only one that can be owed forever**, and that is
    /// what a served run is: something outside this process is driving the
    /// instrument, so the window is being touched even though nobody is at it.
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let next = [self.egui_due, self.costs.due(), self.served]
            .into_iter()
            .flatten()
            .min();
        event_loop.set_control_flow(match next {
            Some(at) => ControlFlow::WaitUntil(at),
            None => ControlFlow::Wait,
        });
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(gfx) = self.gfx.as_mut() else {
            return;
        };
        // **The stillness clock, and it is reset by everything except a frame
        // this loop asked for itself.** A frame drawn while this has not been
        // reset is a frame drawn on an untouched window, which is the number
        // the reading is about; anything arriving from the platform — a
        // pointer, a key, a move, a focus, an occlusion — is the window being
        // touched. Resetting too eagerly only makes the reading harder to
        // reach, never easier to pass.
        if !matches!(event, WindowEvent::RedrawRequested) {
            self.costs.touched();
        }
        match event {
            WindowEvent::CloseRequested => {
                self.keeping.awaited_saves();
                event_loop.exit()
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.scale = scale_factor;
                App::to_egui(gfx, &mut self.costs, &event);
                // **A resize that arrives without a redraw request of its
                // own.** The arrangement is stated in logical pixels, so the
                // same window is a different viewport at a different scale.
                // macOS follows this event with a `Resized` and the viewport
                // is set there — but *usually followed by* is a platform's
                // habit rather than a guarantee, and what it would leave
                // behind is a panel drawn at the wrong scale with nothing
                // anywhere saying so. So the frame is asked for here too.
                App::wants(
                    gfx,
                    &mut self.egui_due,
                    &mut self.costs,
                    Change::Viewport.repaint(),
                );
            }
            WindowEvent::Resized(size) => {
                gfx.config.width = size.width.max(1);
                gfx.config.height = size.height.max(1);
                gfx.surface.configure(&gfx.gpu.device, &gfx.config);
                let (w, h) = (
                    size.width as f32 / self.scale as f32,
                    size.height as f32 / self.scale as f32,
                );
                self.readout.panel.set_viewport(w, h);
                println!("viewport: {w:.0} x {h:.0}");
                App::to_egui(gfx, &mut self.costs, &event);
                App::wants(
                    gfx,
                    &mut self.egui_due,
                    &mut self.costs,
                    Change::Viewport.repaint(),
                );
            }

            // -- the three events the rule is about -----------------------
            // Each one asks `Readout::pointer` who it belongs to and hands it
            // to `egui` only if the answer is `egui`.
            WindowEvent::CursorMoved { position, .. } => {
                let p = Point::new(
                    (position.x / self.scale) as f32,
                    (position.y / self.scale) as f32,
                );
                let ctx = gfx.egui.egui_ctx().clone();
                let (claim, acted) = self.readout.pointer(&ctx, Pointer::Moved(p));
                if claim == Claim::Egui {
                    App::to_egui(gfx, &mut self.costs, &event);
                }
                // **A move can now change the mix**, which no pointer event
                // could before: a fader in hand turns this move into one
                // operation of the vocabulary. What is owed for it is what the
                // drag asked for and not the claim — a fader held against the
                // top of its track asks for 1.0 sixty times a second and
                // changes nothing, and `Change::Pointer(Panel)` would draw a
                // frame for every one of them.
                let repaint = App::performed(
                    gfx,
                    &mut self.readout,
                    &acted,
                    Change::Pointer(claim).repaint(),
                );
                App::wants(gfx, &mut self.egui_due, &mut self.costs, repaint);
            }
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => {
                let which = match state {
                    ElementState::Pressed => Pointer::Down,
                    ElementState::Released => Pointer::Up,
                };
                let ctx = gfx.egui.egui_ctx().clone();
                let (claim, acted) = self.readout.pointer(&ctx, which);
                if claim == Claim::Egui {
                    App::to_egui(gfx, &mut self.costs, &event);
                }
                // **A press that named a scope is a listing to read**, and
                // it is read here because this is where the store is: a scope
                // *is* a listing on this side, and a directory read is not a
                // thing to do on a frame (P-0072). It is read on **every**
                // chip press and not only on one that moved the mark, which is
                // the one place this parts company with `e`: the key steps and
                // so a press that changed nothing asked for nothing, where a
                // pointer *names* — and naming the library you are already
                // reading is asking it again, which is a question this console
                // had no way to put before.
                if matches!(acted, Acted::Emitted(Some(Operation::SelectScope { .. }))) {
                    println!(
                        "{}",
                        listing(&mut self.readout.view, &self.store, self.presets.as_ref())
                    );
                }
                // **A press on a control earns its frame from what it did**,
                // and not from the claim: `Change::Pointer(Claim::Panel)` is
                // already a frame, but the operation the dot asked for is the
                // thing that moved every region in the Program bay, and it is
                // the outcome that says so.
                let repaint = App::performed(
                    gfx,
                    &mut self.readout,
                    &acted,
                    Change::Pointer(claim).repaint(),
                );
                App::wants(gfx, &mut self.egui_due, &mut self.costs, repaint);
            }
            WindowEvent::MouseWheel { .. } => {
                let ctx = gfx.egui.egui_ctx().clone();
                let (claim, _acted) = self.readout.pointer(&ctx, Pointer::Wheel);
                if claim == Claim::Egui {
                    App::to_egui(gfx, &mut self.costs, &event);
                }
                App::wants(
                    gfx,
                    &mut self.egui_due,
                    &mut self.costs,
                    Change::Wheeled(claim).repaint(),
                );
            }

            WindowEvent::KeyboardInput { .. } => {
                // `egui` sees every key: it has no focused widget in this pass
                // and so consumes nothing, and its modifier state has to stay
                // current for the frame it does.
                App::to_egui(gfx, &mut self.costs, &event);
                let WindowEvent::KeyboardInput { event: key, .. } = &event else {
                    unreachable!("the arm this is in")
                };
                if key.state != ElementState::Pressed {
                    return;
                }
                // **The one flow on this panel that asks for letters takes the
                // keyboard whole while it is asking.** Every key here is a
                // character, a rub-out, the commit or the abandonment, and none
                // of them is the operation that key names the rest of the time
                // — `s` is an `s` in a name and not a solo, and the pointer is
                // not what a name is addressed to. `escape` leaves the program
                // the rest of the time and leaves the name here, which is the
                // same word for the same act one level in.
                //
                // **Nothing typed is checked here**, which is
                // [`checked_name`]'s half of the same split: the pill takes
                // the letters, and the wall is where the file is written.
                if self.readout.view.arrangement.naming().is_some() {
                    let (acted, moved) = match key.logical_key.as_ref() {
                        Key::Named(NamedKey::Escape) => {
                            println!("arrangement: nothing was saved");
                            self.readout.view.arrangement.shut();
                            (Acted::Nothing, true)
                        }
                        Key::Named(NamedKey::Enter) => (self.readout.named(), true),
                        Key::Named(NamedKey::Backspace) => {
                            (Acted::Nothing, self.readout.view.arrangement.rubbed_out())
                        }
                        // **A `Key::Character` is text and not a key**, so it
                        // may be more than one character — a dead key
                        // resolving, an IME committing a run — and every one
                        // of them goes in. `Arrangement::typed` is what
                        // refuses a control character, because a newline
                        // arriving as text is the commit rather than a letter.
                        Key::Character(text) => {
                            let mut moved = false;
                            for c in text.chars() {
                                moved |= self.readout.view.arrangement.typed(c);
                            }
                            (Acted::Nothing, moved)
                        }
                        Key::Named(NamedKey::Space) => {
                            (Acted::Nothing, self.readout.view.arrangement.typed(' '))
                        }
                        _ => (Acted::Nothing, false),
                    };
                    let repaint = App::performed(
                        gfx,
                        &mut self.readout,
                        &acted,
                        Change::Naming(moved).repaint(),
                    );
                    App::wants(gfx, &mut self.egui_due, &mut self.costs, repaint);
                    return;
                }
                let op = match key.logical_key.as_ref() {
                    Key::Named(NamedKey::Escape) => {
                        // **The one place a save is waited for**, and it is
                        // bounded — see [`Keeping::awaited_saves`]. A save asked
                        // for in the last second reaches the disk under an id
                        // nobody was ever told, and a client waiting on it has
                        // nowhere else to learn what happened.
                        self.keeping.awaited_saves();
                        event_loop.exit();
                        return;
                    }
                    // **The pointer is resolved here and not inside the
                    // operation.** `f` means fold and this is the surface
                    // saying which region — see `karakuri_console::panel::Op`.
                    // A key that names nothing emits nothing, and the readout
                    // says why from `target`.
                    Key::Character("f") => match self.readout.target("f") {
                        Some(id) => Op::Fold(id),
                        None => return,
                    },
                    Key::Character("g") => match self.readout.enclosing() {
                        Some(op) => op,
                        None => return,
                    },
                    Key::Character("z") => Op::UnfoldAll,
                    Key::Character("s") => match self.readout.target("s") {
                        Some(id) => Op::Solo(id),
                        None => return,
                    },
                    Key::Character("u") => Op::Unsolo,
                    Key::Character("r") => Op::Reset,
                    // **The deck selection, which is the command line's own
                    // four keys.** They are the one place the two keyboards
                    // agree on a letter's meaning besides `esc`, and they
                    // agree because a deck is a slot number: nothing had to be
                    // translated for the panel to mean what the shell already
                    // meant. `SelectDeck` writes no record, so the surface
                    // performs it — [`pointed`], reached through the same
                    // `performed` every control's operation goes through, so
                    // there is one path and not a second one for the pointer.
                    Key::Character("0")
                    | Key::Character("1")
                    | Key::Character("2")
                    | Key::Character("3") => {
                        let Key::Character(digit) = key.logical_key.as_ref() else {
                            unreachable!("the arm this is in")
                        };
                        let deck = digit.as_bytes()[0] - b'0';
                        let acted = Acted::Emitted(Some(Operation::SelectDeck { deck }));
                        let repaint =
                            App::performed(gfx, &mut self.readout, &acted, Repaint::Never);
                        App::wants(gfx, &mut self.egui_due, &mut self.costs, repaint);
                        return;
                    }
                    // **The library cursor**, and it is the one key here that
                    // names no operation at all. `docs/manual/console.html`
                    // argues that where the deck selection has a row: the
                    // selection is what every deck-addressed operation's
                    // keyboard translator fills its `deck` in from, and this
                    // is read by one operation that carries the Set id in its
                    // own payload — so three of the four surfaces would have
                    // nothing to reach and a row would be the first rule
                    // written down as a permanent gap.
                    //
                    // **Held inside the rows the bay drew**, which is why the
                    // listing is asked rather than the store: this bay has no
                    // scroll position, so the reachable Sets are the listed
                    // ones and the foot already says how many are not. The
                    // solve is a flag test on a layout nothing dirtied
                    // (ADR-0183) and is here because a key may arrive between
                    // a resize and the frame that answers it.
                    Key::Named(NamedKey::ArrowUp) | Key::Named(NamedKey::ArrowDown) => {
                        let step = match key.logical_key.as_ref() {
                            Key::Named(NamedKey::ArrowUp) => -1,
                            _ => 1,
                        };
                        self.readout.panel.solve();
                        let listed = karakuri_console::view::library(
                            self.readout.panel.layout(),
                            &self.readout.view.scopes,
                            &self.readout.view.library,
                        )
                        .map_or(0, |bay| bay.rows);
                        let moved = self.readout.view.walk(step, listed);
                        App::wants(
                            gfx,
                            &mut self.egui_due,
                            &mut self.costs,
                            Change::Pointed(moved).repaint(),
                        );
                        return;
                    }
                    // **The load, and the two operands are already on screen.**
                    // The cursor says which Set and the selection says which
                    // deck, which is `console.html`'s *"a cursor and a key
                    // with no pointer anywhere in it"*. What the press does is
                    // re-point the slot's source — [`loading`] — so the worker
                    // builds it and the watchdog judges it exactly as it does
                    // an edit, and nothing here reaches `Deck::install`.
                    // **The scope, and the key steps where the operation
                    // names.** `Operation::SelectScope`'s payload is
                    // `Undecided` — *"what identifies one member of a growable
                    // list is spelled nowhere"* — and its own doc says where
                    // the stepping goes: *"The key steps and this does not …
                    // that is the translator's arithmetic rather than this
                    // operation's payload"* (P-0074). So the surface moves its
                    // own pointer, exactly as the four deck keys do, and the
                    // operation is emitted through the same route so that the
                    // press is recorded as `Silent(Surface)` rather than as
                    // nothing at all.
                    //
                    // **The listing is re-read here**, on the press that
                    // changed the scope: a scope *is* a listing on this side
                    // (`listing`), and a directory read is not a thing to do
                    // on a frame (P-0072).
                    Key::Character("e") => {
                        if self.readout.view.step_scope() {
                            println!(
                                "{}",
                                listing(&mut self.readout.view, &self.store, self.presets.as_ref())
                            );
                        }
                        // **Emitted whether or not the mark moved**, which is
                        // the deck keys' rule: what a press asked for is what
                        // is emitted, and `unwritten` is what says the press
                        // wrote no record and that it is settled.
                        //
                        // **And nothing performs it in `performed`**, where
                        // `SelectDeck` has `pointed` — because this payload
                        // cannot say which scope was chosen and a performer
                        // reading `Undecided` would have to guess. The step
                        // above *is* the performance, and it is the surface's
                        // own pointer either way.
                        let acted =
                            Acted::Emitted(Some(Operation::SelectScope { scope: Undecided }));
                        let repaint =
                            App::performed(gfx, &mut self.readout, &acted, Repaint::Never);
                        App::wants(gfx, &mut self.egui_due, &mut self.costs, repaint);
                        return;
                    }
                    // **Keep what the selected deck is playing**, filed under a
                    // stamp because a bare key press cannot type a name — see
                    // `karakuri_environment::accepted_save`, whose convention
                    // that is and whose reason it borrows: an operator looks for
                    // the time they saved it.
                    //
                    // **The selected deck and not a slot in the key**, which is
                    // the split every deck-addressed control on this panel
                    // makes: the deck an operator means is the one they have
                    // already selected with `0`–`3`, and a model has no
                    // selection and names the slot in the call.
                    //
                    // **Named through `performed` and performed beside it**,
                    // which is `e`'s shape: the emission is what records the
                    // press as `Silent(OnLanding)` rather than as nothing at
                    // all, and the save itself is this arm's because
                    // `Operation::SaveSet` writes no record here — the `save`
                    // record is written where the work lands, and this program
                    // records no session to write it into.
                    //
                    // **The panel column of this row is still `plan`.** A key is
                    // not a control, the Library bay has no *keep* pill drawn,
                    // and a badge that said otherwise would be a claim about a
                    // control that is not there.
                    Key::Character("k") => {
                        let deck = self.readout.view.selection();
                        let acted = Acted::Emitted(Some(Operation::SaveSet {
                            deck,
                            // **`None`, and it is the payload saying so rather
                            // than this arm inventing a stamp.** A caller that
                            // can type a name is not made to take a timestamp,
                            // and a key press is not one of them.
                            id: None,
                        }));
                        let repaint =
                            App::performed(gfx, &mut self.readout, &acted, Repaint::Never);
                        self.keeping.save_set(
                            &gfx.engine,
                            &self.store,
                            usize::from(deck),
                            None,
                            None,
                        );
                        App::wants(gfx, &mut self.egui_due, &mut self.costs, repaint);
                        return;
                    }
                    Key::Character("l") => {
                        let deck = self.readout.view.selection();
                        let at = self.readout.view.cursor_row();
                        let row = self.readout.view.library.get(at).cloned();
                        // **What the take-in half of this press asked for**,
                        // where a press that took nothing in leaves it
                        // `Repaint::Never` — see the preset arm below for why
                        // one press emits two operations and why they cannot
                        // be one `Acted`.
                        let mut took = Repaint::Never;
                        let acted = match (self.readout.view.scope(), row) {
                            // **A preset row is taken in and then loaded**,
                            // which is one press because taking it in is what
                            // gives the Set the id the load needs — ADR-0229's
                            // *one operation, two moments*, performed at the
                            // second of them. What lands in the store is a
                            // Set of the operator's, so `my sets` gains a row
                            // they did not make: `console.html` says that out
                            // loud so that nobody meets it as a surprise.
                            (Some(Scope::Presets), Some(row)) => {
                                match taking_in(&self.store, self.presets.as_ref(), &row) {
                                    Ok(taken) => {
                                        println!("  take in: {}", taken.said);
                                        // **The take-in is named as well as
                                        // performed, and that is `e`'s rule
                                        // one key along**: the scope step
                                        // emits `SelectScope` *"so that the
                                        // press is recorded as `Silent` rather
                                        // than as nothing at all"*, and this
                                        // press has just performed a whole row
                                        // of the vocabulary —
                                        // `docs/manual/operations.html`'s
                                        // *Send a Set to somebody, and take
                                        // one in*, *"this row performed at the
                                        // second of them"*. Emitting only the
                                        // load would be a press that does two
                                        // of the page's rows and names one.
                                        //
                                        // **Two emissions rather than one**,
                                        // because they are two rows: taking in
                                        // is what gives the Set the id, and
                                        // the load names that id. `Acted`
                                        // carries one operation — a fader
                                        // drag, a chip, a key each emit
                                        // exactly one — so the pair is two
                                        // trips through `App::performed`
                                        // rather than a shape invented here
                                        // for the one press that has two.
                                        //
                                        // **In the order they happened.**
                                        // `written` answers
                                        // `Silent(NoRecord)` for the transfer,
                                        // so nothing in `performed` performs
                                        // it and the emission is the naming;
                                        // the load after it is what re-points
                                        // the slot.
                                        let [take, load] = preset_press(deck, taken);
                                        took = App::performed(
                                            gfx,
                                            &mut self.readout,
                                            &Acted::Emitted(Some(take)),
                                            Repaint::Never,
                                        );
                                        Acted::Emitted(Some(load))
                                    }
                                    // **Which of the two acts failed is the
                                    // whole of what this sentence adds.**
                                    // Nothing was taken in, so nothing was
                                    // loaded — where a load that fails says so
                                    // in `played`'s own words, with the deck
                                    // it did not reach. The refusal itself is
                                    // `setfile::unbundle`'s, including the one
                                    // for an id this store already holds.
                                    Err(e) => {
                                        println!(
                                            "  take in: `{row}` was not taken into the store: \
                                             {e}\n  take in: so nothing was loaded, and what is \
                                             on deck {} is still running — a Set already here is \
                                             listed under `my sets`, which is where it is loaded \
                                             from",
                                            deck_letter(deck)
                                        );
                                        Acted::Nothing
                                    }
                                }
                            }
                            // A row of `my sets`, which is a Set this store
                            // already holds and is the route ADR-0228 built.
                            (_, Some(set)) => {
                                Acted::Emitted(Some(Operation::LoadSet { deck, set }))
                            }
                            // Not a refusal of the load: there is no Set under
                            // the cursor because this scope lists nothing. The
                            // bay says so by drawing no rows; this says so in
                            // words, and it says **which** nothing it is —
                            // `favourites` and `folder` are drawn and answer
                            // nothing for two different reasons, and a key
                            // that did nothing and a key that is not bound are
                            // the same experience.
                            (scope, None) => {
                                println!(
                                    "  load: `{}` lists nothing, so there is no Set under the \
                                     cursor — {}",
                                    match scope {
                                        Some(scope) => scope.name(),
                                        None => "the library",
                                    },
                                    match scope {
                                        Some(scope) => why_nothing(scope),
                                        None => "this console was handed no scopes at all",
                                    }
                                );
                                Acted::Nothing
                            }
                        };
                        // **One frame asked for, however many operations the
                        // press emitted.** `App::wants` counts a frame against
                        // the run's costs and asks the window for a redraw, so
                        // calling it twice for one press would ask for two
                        // frames where one is drawn. `Repaint::soonest` is
                        // what combines them, and its own rule is why it is
                        // safe: it can only bring a frame forward.
                        let repaint =
                            App::performed(gfx, &mut self.readout, &acted, Repaint::Never)
                                .soonest(took);
                        App::wants(gfx, &mut self.egui_due, &mut self.costs, repaint);
                        return;
                    }
                    // **The beat, tapped.** The one key on this panel
                    // that reaches the room rather than the deck or the
                    // arrangement, and the first of three that need an input
                    // open. What it does and why it does not go through
                    // `written` is [`tapped`].
                    Key::Character("b") => {
                        println!(
                            "{}",
                            tapped(
                                &mut gfx.audio,
                                &mut gfx.engine.deck,
                                Instant::now(),
                                self.started,
                            )
                        );
                        App::wants(
                            gfx,
                            &mut self.egui_due,
                            &mut self.costs,
                            Change::Emitted(Some(&Operation::TapBeat)).repaint(),
                        );
                        return;
                    }
                    // **The grid, an octave either way**, and the two keys the
                    // page specifies for it. Refused where the result would
                    // leave the trackable range, which is the beat lock's call
                    // — see [`scaled`].
                    Key::Character(",") | Key::Character(".") => {
                        let by = match key.logical_key.as_ref() {
                            Key::Character(",") => GridScale::Halve,
                            _ => GridScale::Double,
                        };
                        println!("{}", scaled(&mut gfx.audio, &mut gfx.engine.deck, by));
                        App::wants(
                            gfx,
                            &mut self.egui_due,
                            &mut self.costs,
                            Change::Emitted(Some(&Operation::ScaleGrid { by })).repaint(),
                        );
                        return;
                    }
                    // **The latency offset, and the third of the three keys
                    // that need an input open.** Unlike `b`, `,` and `.` this
                    // one goes through [`App::performed`] like every other
                    // key on this panel: `written(SetLatencyOffset)` answers
                    // `Silent(NoRecord)` rather than `Owed(NotSettled)` —
                    // nothing in the session stream carries a delay between
                    // what a room hears and what it sees — so the line
                    // `unwritten` prints about it is true, where the tap's
                    // would have been *"nothing moved, and nothing here
                    // decides it"* about a press that moved the grid.
                    //
                    // **The operation is absolute and the key is the nudge**,
                    // which is `Operation::SetLatencyOffset`'s own rule: *"an
                    // absolute value can express every nudge and a nudge
                    // cannot express a setting, and a fader has to be able to
                    // reach it"*. So the press reads the value it is standing
                    // on and adds a step, exactly as `karakuri-cli`'s two do.
                    //
                    // **With no input attached there is nothing to read**, and
                    // no operation is emitted at all — the offset is a term in
                    // the lead the tracker corrects against, and a value
                    // dialled against no room would be dropped the moment one
                    // opened, because `attached` starts a new session at the
                    // offset the old one held and at the default when there
                    // was none. Said out loud rather than swallowed, on
                    // [`tapped`]'s and [`scaled`]'s terms: a key that does
                    // nothing and a key that is not bound are the same
                    // experience.
                    Key::Character("o") | Key::Character("p") => {
                        let Key::Character(name) = key.logical_key.as_ref() else {
                            unreachable!("the arm this is in")
                        };
                        let step = offset_step(name).expect("the arm this is in");
                        let acted = match gfx.audio.as_ref() {
                            Some(open) => Acted::Emitted(Some(Operation::SetLatencyOffset {
                                ms: open.latency_offset_ms() + step,
                            })),
                            None => {
                                println!("{}", NO_ROOM_FOR_AN_OFFSET);
                                Acted::Nothing
                            }
                        };
                        let repaint =
                            App::performed(gfx, &mut self.readout, &acted, Repaint::Never);
                        App::wants(gfx, &mut self.egui_due, &mut self.costs, repaint);
                        return;
                    }
                    Key::Character("n") => {
                        // **The key that changes the screen without touching
                        // the pointer and without touching the model.** The
                        // room is the view's: every colour on the panel
                        // changes and nothing in the arrangement moves, so no
                        // `Outcome` says so and `Change::Room` is the only
                        // thing that does.
                        self.readout.room();
                        App::wants(
                            gfx,
                            &mut self.egui_due,
                            &mut self.costs,
                            Change::Room.repaint(),
                        );
                        return;
                    }
                    _ => return,
                };
                let outcome = self.readout.op(op);
                App::wants(
                    gfx,
                    &mut self.egui_due,
                    &mut self.costs,
                    Change::Operated(&outcome).repaint(),
                );
            }

            WindowEvent::RedrawRequested => {
                // **What a model asked for, and what a save came back with —
                // both above everything that touches the window.** A client
                // asking to keep what is playing should not be waiting on a
                // swapchain, and nothing either of these reaches needs one; a
                // window that has faulted returns below this line and still owes
                // a waiting client its answer. That is `karakuri-cli`'s
                // `Live::run_requests` and `Live::finished_saves`, at the top of
                // the frame for the reason written there.
                self.keeping.requests(&mut gfx.engine, &self.store);
                if self.keeping.finished_saves() {
                    // **The one thing in this program that adds a Set**, so the
                    // bay that lists them is re-read on the frame it landed —
                    // and only on that frame. A directory read is not a thing to
                    // do per frame (P-0072), and a bay still listing what it
                    // listed before a save is a readout that is wrong and
                    // silent.
                    println!(
                        "{}",
                        listing(&mut self.readout.view, &self.store, self.presets.as_ref())
                    );
                }
                let waited = Instant::now();
                let acquired = gfx.surface.get_current_texture();
                let waited = waited.elapsed();
                if let Some(missed) = missed(&acquired) {
                    match missed {
                        Missed::Remake => {
                            gfx.surface.configure(&gfx.gpu.device, &gfx.config);
                            self.costs.owes();
                            gfx.window.request_redraw();
                        }
                        Missed::Again => {
                            self.costs.owes();
                            gfx.window.request_redraw();
                        }
                        Missed::Idle => {}
                        Missed::Fault => {
                            if !self.faulted {
                                self.faulted = true;
                                println!(
                                    "the surface raised a validation error acquiring a frame — \
                                     the window has stopped drawing"
                                );
                            }
                        }
                    }
                    return;
                }
                self.faulted = false;
                let frame = match acquired {
                    wgpu::CurrentSurfaceTexture::Success(frame)
                    | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
                    // `missed` returned `None`, so there is a texture here.
                    _ => return,
                };

                let mut cost = Cost {
                    wait: waited,
                    ..Cost::default()
                };

                // -- where each sink goes, and how big it is -----------
                // **Before the `egui` pass**, because the pass draws these
                // textures and one registered after it would be a frame
                // behind. **And before `compose`**, because `Sink::acquire` is
                // handed a `&Gpu` and nothing else, while deciding this needs
                // the rectangle, the scale factor and the renderer — see
                // `Presented::aim`.
                //
                // One call, and it is the same one `resumed` sized both
                // textures with. *Which rectangle, at what size* is `aims` and
                // `Engine::aim` and nowhere else, which is what lets a test
                // ask the question this handler used to answer where nothing
                // could reach it.
                self.readout.panel.solve();

                // -- the Program bay arranges itself -------------------
                // **Before `aims`, because the four cells are in one of two
                // places and this is what decides which.** The canvas is
                // written in the same breath, off the `Present` that is about
                // to render it, for the reason `aims` reads it there too: the
                // shape the bay arranges itself for and the shape the texture
                // is sized to are one number or they are a picture drawn for
                // the other arrangement.
                //
                // `Change::Rearranged` is raised on **every** frame and says
                // whether anything moved, which is the arm's own argument:
                // this is the one change that is re-derived rather than
                // reported, and one that answered *draw* regardless would ask
                // for a frame on every frame.
                self.readout.view.canvas = gfx.engine.present.size();
                let moved = view::rearrange(&mut self.readout.panel, self.readout.view.canvas);
                App::wants(
                    gfx,
                    &mut self.egui_due,
                    &mut self.costs,
                    Change::Rearranged { moved }.repaint(),
                );

                let scale = self.scale as f32;
                let (picture, previews) = gfx.engine.aim(
                    &gfx.gpu,
                    &mut gfx.renderer,
                    self.readout.panel.layout(),
                    scale,
                );
                self.readout.view.picture = picture;
                self.readout.view.previews = previews;

                // **What the transport row reads, written beside the frame it
                // is about**, exactly as the two lines above are: the picture
                // is a texture id that belongs to this frame and this is a
                // tempo and a cost that belong to the last one. See
                // `transport`.
                //
                // **`live` is asked once and used twice.** It decides whether
                // this frame is followed by another — the last statement in
                // this handler — and the row's frame rate is a claim about
                // that. Two calls would be two answers to *is anything making
                // texels*, taken either side of the whole frame.
                let live = live(&self.readout.view);
                // **The room, read before the row that reads the grid it
                // moves.** A measurement taken after `transport` would be a
                // tempo drawn one frame behind the correction that made it,
                // which is the one thing this row cannot be: it is what an
                // operator watches to tell a lock from a coincidence. See
                // `measure_audio`.
                measure_audio(&mut gfx.audio, &mut gfx.engine.deck, self.costs.rate_now());
                self.readout.view.transport =
                    transport(&gfx.engine.deck, &self.costs, gfx.budget_ms, live);
                // **And what the two look controls at the end of that row
                // read**, beside the frame they are about. It is the look this
                // frame is committed under, so the capsule names the operator
                // the picture went through rather than one a press asked for
                // and nothing has applied yet.
                self.readout.view.look = Some(look(&gfx.engine.look));
                // **And what the Master bay's out row reads**, which is the
                // other end of the same chain: this level is applied where the
                // mix wrote the frame and the look's is applied where the
                // present pass read it, so the two are read off two different
                // objects and written here in the same breath (ADR-0224).
                self.readout.view.master_out = Some(gfx.engine.deck.out());
                // **And which classes are open to a model**, read off the
                // handle rather than remembered from the last press on a pill.
                // Nothing but a pill writes it today; the handle exists because
                // an MCP server holds a clone of it and reads it on every call,
                // and a view that trusted its own last write would be the
                // console answering on that server's behalf.
                self.readout.view.opening = self.readout.opening.read();
                // **And what the mixer strips read**, beside the frame they
                // are about for the same reason. One strip per slot, so two —
                // see `mixer`.
                mixer(
                    &gfx.engine.deck,
                    &gfx.material,
                    &mut self.readout.view.mixer,
                );

                // **And what the Staging lane lists**, off the same deck and
                // beside the frame the verdicts belong to. It reads the
                // *previous* frame's, exactly as the transport row above does
                // and for the same reason: a build is installed and a
                // watchdog reports at a frame boundary, which is `compose`
                // below. See `staging`, which is also where the drain is
                // argued.
                // **And the Inspector's panes with them, on the frames a
                // Set actually landed on.** `Set::published` allocates and
                // says it is not for the frame path — *"A console reads this
                // when a Set lands, not per frame"* — and this is the first
                // thing in this program that knows when one did. Before the
                // slots were watched no Set ever landed after the first, so
                // this read was a startup step and nothing else; it is still
                // a startup step and now also a rebuild's.
                let mut took: Vec<(usize, Option<u64>)> = Vec::new();
                if staging(
                    &mut gfx.engine.deck,
                    &mut self.readout.view.staging,
                    self.keeping.mcp.as_ref(),
                    &mut took,
                ) {
                    inspector(
                        &gfx.engine.deck,
                        &gfx.material,
                        &mut self.readout.view.inspector,
                    );
                }
                // **What each of those slots is now playing**, taken up once the
                // drain above has let go of the deck. A slot whose material
                // changed and was not taken up is a slot a save would write down
                // the *previous* version of, which is the failure [`Playing`]
                // exists to make impossible.
                for (slot, landed) in took {
                    self.keeping.took_up(&gfx.engine, slot, landed);
                }

                // **The one clock behind everything that moves on the panel,
                // written as the number it becomes.** Beside the strips it
                // animates, and beside them for the same reason the picture
                // and the transport are written here: it belongs to this
                // frame.
                self.readout.view.phase = view::Phase::since(self.started.elapsed());
                // **And what that costs, asked of the view rather than
                // decided here.** `View::animating` is the panel's own
                // declaration — a staleness while something is pending, and
                // `None` while nothing is — and `Change::Animating` turns it
                // into a deadline. It is asked every frame because nothing an
                // operator does can raise it: a slot is parked by the
                // governor, between frames, through no window event at all.
                //
                // **`Costs::owes` is deliberately not called**, exactly as it
                // is not for the picture below: a frame the roll asks for is a
                // frame drawn on a window nobody touched, and the still-panel
                // reading should print the rate rather than hide it.
                App::wants(
                    gfx,
                    &mut self.egui_due,
                    &mut self.costs,
                    Change::Animating(self.readout.view.animating(self.readout.panel.layout()))
                        .repaint(),
                );

                // -- the egui pass -------------------------------------
                let started = Instant::now();
                let (allocs, bytes) = counted();
                let input = gfx.egui.take_egui_input(&gfx.window);
                let panel = &mut self.readout.panel;
                let view = &mut self.readout.view;
                let mut output = gfx.egui.egui_ctx().run_ui(input, |ui| view.draw(ui, panel));
                let primitives = gfx
                    .egui
                    .egui_ctx()
                    .tessellate(output.shapes, output.pixels_per_point);
                cost.ui = started.elapsed();
                let (allocs2, bytes2) = counted();
                cost.allocs = allocs2 - allocs;
                cost.bytes = bytes2 - bytes;

                // **What `egui` asked for, with the delay it asked for.** It
                // is `Duration::MAX` on a pass that wants nothing, which is
                // every pass on a panel with nothing on it, and that is
                // `Repaint::Never` — the loop then has no reason of its own to
                // draw again. Read off the root viewport's output, after the
                // clock above so that a map lookup is not in the number.
                let asked = Repaint::asked(
                    output
                        .viewport_output
                        .get(&karakuri_console::egui::ViewportId::ROOT)
                        .map_or(Duration::MAX, |v| v.repaint_delay),
                );

                gfx.egui
                    .handle_platform_output(&gfx.window, output.platform_output);

                // -- the frame: one compose, one encoder, one submission --
                //
                // **The engine and the panel are one command buffer, engine
                // first.** `frame::compose` asks both sinks, advances the deck
                // whatever they answer, draws the canvas into the ones that
                // took the frame, and then hands **the frame's own encoder**
                // to the closure below — which is where the whole panel goes.
                //
                // The panel is not a sink, and the argument is written on
                // `compose`: it does not receive the composited frame, it
                // receives the panel, and it happens to sample what a sink
                // produced. An encoder of this file's own would be a second
                // submission over a texture that is a colour attachment in one
                // and a sampled resource in the other, ordered by whatever the
                // queue happened to do — it would work today and be a race
                // nobody wrote down. That is
                // `docs/adr/0166-the-engines-frame-and-the-panels-are-one-submission.md`.
                //
                // **This used to be a second frame loop.** `begin_frame`, a
                // render, a conditional present pass per target, the panel,
                // the submit — all spelled out here, beside the one in
                // `karakuri-cli` that says the same thing differently. That is
                // the drift `karakuri_engine::frame` exists to end, and it is
                // one call now.
                let screen = egui_wgpu::ScreenDescriptor {
                    size_in_pixels: [gfx.config.width, gfx.config.height],
                    pixels_per_point: output.pixels_per_point,
                };
                let view_target = frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());
                // **Read from inside the closure, because that is where the
                // engine's half ends and the panel's begins.** `Cost::engine`
                // and `Cost::paint` are then adjacent by construction, rather
                // than two `Instant::now()`s a statement could get between.
                let mut panel_started = None;
                let mut submitting = None;
                let engine_started = Instant::now();
                let composed = {
                    let Gfx {
                        gpu,
                        renderer,
                        engine,
                        ..
                    } = &mut *gfx;
                    let gpu = &*gpu;
                    let Engine {
                        deck,
                        present,
                        picture,
                        previews,
                        slot_bind_groups,
                        look,
                        ..
                    } = engine;
                    let textures_delta = &mut output.textures_delta;
                    let cost = &mut cost;
                    let mut sinks: [&mut dyn Sink; 1] = [picture];
                    compose(
                        gpu,
                        deck,
                        present,
                        &mut sinks,
                        &mut |_at, skip| {
                            if let Skip::Fault(why) = skip {
                                println!("a sink stopped taking frames: {why}");
                            }
                        },
                        |_| Committed {
                            steps: STEPS_A_FRAME,
                            look: *look,
                        },
                        // -- the panel, into the frame's encoder ------
                        |encoder| {
                            monitor(present, previews, slot_bind_groups, encoder);
                            panel_started = Some(Instant::now());
                            // One id can carry several deltas in a frame: a
                            // font atlas that grew arrives as the whole image
                            // followed by its patches, and applying only the
                            // first would leave holes.
                            let uploading = Instant::now();
                            for (id, deltas) in &textures_delta.set {
                                for delta in deltas {
                                    renderer.update_texture(&gpu.device, &gpu.queue, *id, delta);
                                }
                            }
                            cost.textures = uploading.elapsed();
                            let uploading = Instant::now();
                            let user = renderer.update_buffers(
                                &gpu.device,
                                &gpu.queue,
                                encoder,
                                &primitives,
                                &screen,
                            );
                            cost.buffers = uploading.elapsed();
                            let recording = Instant::now();
                            {
                                let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                    label: Some("console"),
                                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                        view: &view_target,
                                        depth_slice: None,
                                        resolve_target: None,
                                        ops: wgpu::Operations {
                                            // The console's own ground
                                            // is painted by the central
                                            // panel; this only matters
                                            // for the frame before the
                                            // first one lands.
                                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                            store: wgpu::StoreOp::Store,
                                        },
                                    })],
                                    depth_stencil_attachment: None,
                                    timestamp_writes: None,
                                    occlusion_query_set: None,
                                    multiview_mask: None,
                                });
                                renderer.render(&mut pass.forget_lifetime(), &primitives, &screen);
                            }
                            for id in &textures_delta.free {
                                renderer.free_texture(id);
                            }
                            // **`epaint` panics on a `TexturesDelta` dropped
                            // unapplied**, and a panic here is reached from a
                            // `winit` callback, which on macOS is an abort
                            // rather than an error. Every delta above has been
                            // handed to the renderer, so this says so.
                            textures_delta.clear();
                            cost.record = recording.elapsed();
                            // **`update_buffers` hands back a command buffer
                            // per `egui` paint callback that asked for one**,
                            // and this console registers no paint callbacks —
                            // the picture is a registered texture drawn as an
                            // image, not a callback. So this is empty, and an
                            // empty submission is skipped rather than made. It
                            // is not dropped: a callback's prepared work
                            // submitted after the pass that reads it would be a
                            // frame behind, so if one ever appears it goes in
                            // ahead — and the engine and the panel stay in the
                            // one submission `compose` makes the moment this
                            // closure returns.
                            submitting = Some(Instant::now());
                            if !user.is_empty() {
                                gpu.queue.submit(user);
                            }
                        },
                    )
                };
                // The engine's half ran from the top of `compose` to the
                // moment it handed the encoder over; the panel's is the rest
                // of the call, the one submission included.
                let panel_started = panel_started.expect("`finally` runs on every frame");
                cost.engine = panel_started - engine_started;
                cost.paint = panel_started.elapsed();
                cost.submit = submitting.expect("`finally` runs on every frame").elapsed();
                // **Said and not returned on**, and neither of this program's
                // sinks can produce it — `Presented::present` is `Ok(())`. It
                // is here because a third sink could, and because a frame the
                // other sinks took is not one this window may drop.
                if let Err(e) = composed {
                    println!("a sink failed to present: {e}");
                }

                gfx.gpu.queue.present(frame);

                self.costs.push(cost);
                App::wants(gfx, &mut self.egui_due, &mut self.costs, asked);
                // **Something is live, so the next frame is asked for here —
                // and asked for without `Costs::owes`.**
                //
                // Nothing used to ask for a frame at this point, and that was
                // P-0072's first clause holding: a panel with nothing changing
                // on it drew nothing. A picture that moves is something
                // changing on it, so the clause stops holding the moment the
                // engine runs — which is expected, is what the rest of P-0072
                // exists for, and is **not fixed here**. There is no scheduler
                // in this file, the panel is not cached to a texture, and
                // `karakuri_console::repaint` has not been given a fourth
                // answer.
                //
                // What is done instead is to make the price visible.
                // `Costs::owes` is deliberately not called, so every frame the
                // picture asks for lands in the still-panel reading as what it
                // is: a frame drawn on a window nobody touched. The reading
                // then prints the rate rather than the zero, and the next
                // decision gets made on a number.
                //
                // **Asked for only while something is on screen making
                // texels.** It used to be unconditional, with `live` set once
                // when the engine was built and never cleared — so folding the
                // picture away left the loop drawing at full rate for nothing,
                // and the reading went on calling it live. Another machine
                // found that by following this file's own instructions and
                // getting 270 frames out of a window that was supposed to have
                // gone quiet.
                //
                // Then it became the picture alone, and deck A's audition put
                // that wrong again in the same direction: fold the picture and
                // the preview goes on rendering under it, so the panel keeps
                // changing while the loop stops asking for frames. **The rule
                // is anything that makes texels, and the list is closed** —
                // [`live`] is where it is written and where a test can reach
                // it.
                self.costs.live = live;
                // **And what the panel asked for on its own account**, which
                // is the other half of why frames are being drawn on an
                // untouched window. Asked of the view here for the same reason
                // `live` is: one answer per frame, kept for the reading.
                self.costs.declared = self.readout.view.animating(self.readout.panel.layout());
                if live {
                    gfx.window.request_redraw();
                }
            }
            _ => App::to_egui(gfx, &mut self.costs, &event),
        }
    }
}

// ---------------------------------------------------------------------------
// The room this instrument is listening to
// ---------------------------------------------------------------------------

/// **What this program opens on, and it is not a flag.**
///
/// `karakuri-cli` is told which input to take with `--audio-in` and refuses to
/// start without the one it was told; this program opens the host's default
/// and, from then on, is told by a hand on the `audio-in` pill. The two are
/// different on purpose and the difference is the surface:
///
/// - **A flag is a contract made before the run.** Asking for one and getting
///   none is a run that is not the run that was asked for, so
///   `karakuri-cli` exits — and it is right to, because a render or a set
///   played from a script has nobody standing there to notice.
/// - **This program has somebody standing there.** It draws a pill that says
///   which input is open and lists the others, so *which room* is a question
///   the panel can both ask and answer while it is running. A second way to
///   say it on the command line would be a launch-time answer to a question
///   the panel already answers better, and `USAGE` says in as many words that
///   this is not `karakuri-cli`'s command line.
///
/// **And it opens something rather than nothing**, which is the choice that
/// matters for what this instrument is: material that moves with the room is
/// what the panel looks like, and an instrument that listens only after being
/// asked comes up looking like one that cannot. `default` is what a machine
/// answers when nobody has chosen, which is exactly the state a program that
/// has just started is in.
const LISTEN_ON: &str = "default";

/// **One simulation step**, which is what the audio path has to be told a
/// frame advances the session by so a beat correction lands on the right one.
/// `karakuri-cli` names the same constant for the same reason.
const DT: f32 = karakuri_engine::set::DT;

/// **Open the input this program listens on, and say what happened.**
///
/// Returns the session and one line for the legend — never a refusal that
/// stops the run, and that is the decision rather than an omission. The three
/// cases it has to be right about are the three the window can meet, and two
/// principles point in different directions across them:
///
/// 1. **No device at all.**
///    [P-0034](../../../docs/principles/0034-a-quiet-room-is-not-a-missing-microphone.md)
///    — *a quiet room is not a missing microphone* — and neither is a missing
///    microphone a fault. Nobody asked for one here: this program opens the
///    default because that is what an instrument does, and a machine with no
///    input is a machine where every name goes on answering what it answered
///    before audio existed and the oscillator free-runs. It is said out loud,
///    once, and the run continues. Exiting would mean a laptop with its
///    microphone switched off cannot open the panel at all.
/// 2. **A device that was named and is not there.** A different case, and
///    [P-0027](../../../docs/principles/0027-a-silently-wrong-image-loses-to-a-loud-failure.md)
///    is why: somebody said *that one*, and going quietly on with a different
///    one — or with none — is the silently wrong picture. It cannot happen
///    *here*, because nothing names an input at launch; it happens at the
///    pill, where the list an operator picked from was read at the press and a
///    device can have gone away since. [`attached`] is that case and it is
///    loud there. **Loud and not fatal**, which is where this program parts
///    from `karakuri-cli`: a window with a set on it must not close because an
///    interface was unplugged, and the operator is standing in front of the
///    refusal.
/// 3. **A device that goes away mid-set.** Nothing here notices, deliberately,
///    and that *is* the answer: `karakuri-audio`'s `staleness` takes the
///    confidence of both the signals and the tempo estimate to zero over half
///    a second, every bound parameter is handed back to the value it had, and
///    the grid free-runs from wherever it was. A watchdog that re-opened the
///    stream would be a second answer to a question that already has one, and
///    it would re-lock the grid to a room in the middle of a set. What an
///    operator does about it is pick again on the pill.
///
/// A free function rather than a step of `resumed`, for the reason
/// [`sources_from`] is one: `resumed` cannot be called from a test, and a
/// refusal nobody can reach is a refusal nobody checked. See
/// [`unopened`], which is the half of it that has no device in it at all.
fn listening(session_bpm: f32) -> (Option<audio::Audio>, String) {
    match audio::Audio::open(LISTEN_ON, audio::DEFAULT_LATENCY_OFFSET_MS, DT, session_bpm) {
        Ok(open) => {
            let line = format!(
                "audio in: {} at {} Hz — energy, onset and band0..7 are measured from this room \
                 now, and the beat corrects the session's oscillator. output offset {:.0} ms. \
                 the transport row's `audio-in` pill says which input this is and lists the \
                 others; `b` taps the beat, `,` and `.` move the grid an octave, and `o` and \
                 `p` nudge that offset five milliseconds a press.",
                open.description(),
                open.sample_rate(),
                open.latency_offset_ms()
            );
            (Some(open), line)
        }
        Err(why) => (None, unopened(LISTEN_ON, &why)),
    }
}

/// **What to say about an input that did not open**, and which of the two
/// kinds of nothing it was.
///
/// Split out from [`listening`] because it is the whole of the judgement and
/// none of the device: a machine with no inputs and a machine whose default
/// vanished are two sentences, and the difference between them is the
/// difference between P-0034 and P-0027. Being a function of an error and a
/// string, it is checkable where no input can be opened at all — which is
/// every machine a test runs on, whatever it happens to have plugged in.
///
/// **The empty case is not apologetic and the non-empty one is not calm.** A
/// machine with no inputs is a state; a machine with inputs where the one
/// asked for is not among them is somebody's mistake or somebody's cable, and
/// the list is what they need rather than an invitation to go and look.
fn unopened(selector: &str, why: &audio::AudioError) -> String {
    match why {
        audio::AudioError::NoMatch { available, .. } if available.is_empty() => String::from(
            "audio in: none — this machine has no audio inputs, which is a state and not a \
             fault: every signal name answers what it answered before audio existed, and the \
             grid free-runs at the session tempo. the `audio-in` pill says `none` and its card \
             says so too.",
        ),
        audio::AudioError::NoMatch { available, .. } => format!(
            "audio in: `{selector}` is not one of this machine's {} input{} — {}. nothing is \
             open; pick one on the `audio-in` pill.",
            available.len(),
            match available.len() {
                1 => "",
                _ => "s",
            },
            available.join(", ")
        ),
        // Config, Build, SampleFormat: a device that is there and would not
        // start. Said in the audio crate's own words rather than translated —
        // it is the only thing that knows what a host refused.
        other => format!(
            "audio in: none — `{selector}` is there and would not open: {other}. nothing is \
             open; pick another on the `audio-in` pill."
        ),
    }
}

/// **What the `audio-in` pill reads**, out of the session this program opened.
///
/// One line, and it is a function rather than an assignment for the reason
/// [`transport`] is one: it is the seam, and there is exactly one place the
/// answer is derived. The card's list is **not** here — it is read on the
/// press that opens the card and nowhere else (P-0072), so a reading taken
/// every frame would be a directory read on the frame path with a microphone
/// in place of the directory.
fn told(open: Option<&audio::Audio>) -> AudioIn {
    let mut told = AudioIn::NONE;
    told.device = open.map(|open| open.description().to_owned());
    told
}

/// **A press on one of the `audio-in` card's rows, performed**, and what this
/// file says about it. `None` for every operation that is not it, exactly as
/// [`arrangement`] and [`pointed`] answer `None` for everything that is not
/// theirs.
///
/// This is the second of [`listening`]'s three cases and the only one that can
/// arrive during a set: the card lists what the host had **at the press that
/// opened it**, and an interface unplugged between that press and this one is
/// a name the operator picked that is not there any more. P-0027 — the refusal
/// is printed with the list as it is *now*, and **the input that was already
/// open stays open**: dropping it would answer a mistyped pick by taking away
/// the room, which is the one thing nobody asked for.
///
/// **`AttachBeatSource` writes no record** (`written` answers
/// `Silent(NoRecord)`: no session-stream variant carries what the beat is
/// taken from), so nothing downstream of this moves the deck. What moves is
/// this program's own audio session and the pill that reads it.
///
/// A `BeatSource::Process` reaches here and is declined in one sentence: the
/// panel has no control that names one and `--tempo-source` is
/// `karakuri-cli`'s. It is answered rather than ignored, because an operation
/// that arrives and does nothing at all is the failure P-0027 is about.
fn attached(
    open: &mut Option<audio::Audio>,
    session_bpm: f32,
    told_pill: &mut Option<AudioIn>,
    operation: &Operation,
) -> Option<String> {
    let Operation::AttachBeatSource { source } = operation else {
        return None;
    };
    let selector = match source {
        BeatSource::AudioInput(selector) => selector,
        BeatSource::Process(command) => {
            return Some(format!(
                "  attach: `{command}` is a process, and nothing on this panel starts one — \
                 `karakuri-cli --tempo-source` is where that half of the row lives"
            ))
        }
    };
    // The offset the operator has already dialled in survives the change of
    // device: it is a property of this room's outputs and not of its input,
    // which is the whole of what `LATENCY_OFFSET_RANGE`'s documentation is
    // about. A new session at the default would silently undo it.
    let offset = open
        .as_ref()
        .map(|open| open.latency_offset_ms())
        .unwrap_or(audio::DEFAULT_LATENCY_OFFSET_MS);
    match audio::Audio::open(selector, offset, DT, session_bpm) {
        Ok(opened) => {
            let line = format!(
                "  attach: {} at {} Hz -> AttachBeatSource -> no record, and that is settled: \
                 nothing in the session stream says what the beat was taken from. the grid \
                 follows this room now, at offset {:.0} ms",
                opened.description(),
                opened.sample_rate(),
                opened.latency_offset_ms()
            );
            *open = Some(opened);
            *told_pill = Some(told(open.as_ref()));
            Some(line)
        }
        // **The one that was open stays open**, and the pill goes on naming
        // it: what failed is the pick, not the room.
        Err(why) => Some(format!(
            "  attach: {} — {}",
            selector,
            match open.as_ref() {
                Some(open) => format!("{why}. `{}` is still open", open.description()),
                None => format!("{why}. nothing is open"),
            }
        )),
    }
}

/// **What a press on `o` or `p` moves the offset by**, and `None` for every
/// other key.
///
/// A function rather than two literals in the arm so that the **sign** is
/// something a test can ask about. `docs/manual/console.html` says which half
/// gets read wrong — *"the sign is the half that gets read wrong at two in the
/// morning, so it is said in words here rather than left to be worked out"* —
/// and a pair of keys wired the wrong way round is a control that reads
/// correct and points backwards.
///
/// The step is `karakuri_environment::audio`'s own constant, which is what
/// `karakuri-cli`'s `o` and `p` step by: *five milliseconds a press* is one
/// number and this program does not keep a second copy of it.
fn offset_step(key: &str) -> Option<f32> {
    match key {
        "o" => Some(-audio::LATENCY_OFFSET_STEP_MS),
        "p" => Some(audio::LATENCY_OFFSET_STEP_MS),
        _ => None,
    }
}

/// **What the offset keys say on a panel with no input attached.**
///
/// `docs/manual/console.html` is the specification and it is plain about it:
/// *"It only means anything with an audio input attached, and the audio-in
/// pill is what says whether there is one."* So the press changes nothing,
/// says why, and names the control that would fix it — [`tapped`]'s and
/// [`scaled`]'s sentence for the same state, one row along.
const NO_ROOM_FOR_AN_OFFSET: &str = "offset: no audio input — the offset is the delay between \
                                     what a room hears and what it sees, and there is no room. \
                                     open one on the transport row's `audio-in` pill";

/// **What to say about an offset that moved**, out of what was asked for and
/// what the session came back with.
///
/// A function of two numbers and nothing else, so that both halves of
/// `console.html`'s contract are checkable without a device:
///
/// - **The sign, in words.** *"Negative and the picture waits for the music,
///   positive and it leads"* — the page says it in words rather than leaving
///   `−15 ms` to be interpreted, and so does this.
/// - **The bound, when it bit.** The value is *"held inside 200 ms either
///   way"*, which `karakuri_environment::audio` enforces and this reports: a
///   press that asked for 205 and got 200 is a control at the end of its
///   travel, and a control that answers the same number twice with nothing
///   said is indistinguishable from a broken one (P-0030).
fn offset_said(asked: f32, now: f32) -> String {
    let sense = match now < 0.0 {
        true => "the picture waits for the music",
        false => "the picture leads the music",
    };
    let held = match (asked - now).abs() > f32::EPSILON {
        true => format!(
            " — held at {:+.0} ms, which is as far either way as it goes",
            now
        ),
        false => String::new(),
    };
    format!("  offset: {now:+.0} ms — {sense}{held}")
}

/// **The latency offset, performed against the session this program opened**,
/// and `None` for every operation that is not it — [`attached`]'s shape, one
/// control along, and beside it in [`App::performed`] for the same reason.
///
/// **`SetLatencyOffset` writes no record** (`written` answers
/// `Silent(NoRecord)`: nothing in the session stream carries a delay between
/// two outputs, which is a property of a room and not of a performance), so
/// nothing downstream of this moves the deck. What moves is the lead every
/// beat correction is applied with — `Audio::output_lag` — and the frame the
/// picture is drawn on relative to it.
///
/// **The operation is absolute and this is where it lands.** It is applied
/// through `Audio::nudge_latency_offset`, which is the only way in and is the
/// one that clamps: the offset is held inside `LATENCY_OFFSET_RANGE` there, so
/// this file states no bound of its own and cannot state a different one. A
/// *setting* becomes the step that reaches it, which is what lets a fader
/// emit this operation the day one exists without a second application path.
///
/// **With nothing open there is nothing to offset**, and the key arm says so
/// before an operation is built — see [`NO_ROOM_FOR_AN_OFFSET`]. This arm
/// answers the case an operation arrives from anywhere else in that state,
/// because an operation that arrives and does nothing at all is the failure
/// P-0027 is about.
fn nudged(open: &mut Option<audio::Audio>, operation: &Operation) -> Option<String> {
    let Operation::SetLatencyOffset { ms } = *operation else {
        return None;
    };
    let Some(open) = open.as_mut() else {
        return Some(format!("  {NO_ROOM_FOR_AN_OFFSET}"));
    };
    let now = open.nudge_latency_offset(ms - open.latency_offset_ms());
    Some(offset_said(ms, now))
}

/// **One frame's worth of audio**: read the room, and hand the session what it
/// said.
///
/// The same three lines `karakuri-cli`'s `measure_audio` is, minus the two
/// halves this program does not have — there is no session recorder to hand
/// the record to, and no tempo source to yield the grid to, so the grid is
/// always this tracker's ([`audio::Grid::Owned`]).
///
/// **The signals are copied out of the deck and back in**, which is what
/// `Deck::signals` and `set_signals` are for: the bus is a `Copy` value and
/// the deck is the model of record for it, so an `AudioFrame` reaching a
/// binding goes through the deck rather than round it.
///
/// `interval` is how fast frames are actually arriving, which is half the
/// output lag a beat correction leads by. It is [`Costs::rate_now`] inverted
/// — the same measurement the transport row draws as `fps`, asked once more
/// rather than measured a second time, so the number the row shows and the
/// number the lag is built from cannot disagree. `None` is a window that has
/// not drawn a stretch yet, and `Audio::frame` ignores an interval outside
/// `(0, 1)`: the smoothed value simply holds, which is the right answer for a
/// frame nobody can time.
fn measure_audio(open: &mut Option<audio::Audio>, deck: &mut Deck, interval: Option<f64>) {
    let Some(open) = open.as_mut() else {
        return;
    };
    let mut signals = *deck.signals();
    let (_audio, tempo) = open.frame(
        &mut signals,
        interval.map(|rate| 1.0 / rate as f32).unwrap_or(0.0),
        f32::from(STEPS_A_FRAME) * DT,
        audio::Grid::Owned,
    );
    deck.set_signals(signals);

    // **A correction worth saying out loud is one that is a decision rather
    // than a trim** — acquiring, re-acquiring, a tap, an octave — which is
    // `karakuri-cli`'s rule and is here for P-0030's reason: an operator who
    // cannot see the grid decide cannot tell a lock from a coincidence. A trim
    // happens on every frame once locked and says nothing.
    let reason = open.reason();
    if let (Some(Record::Tempo { bpm, .. }), Some(reason)) = (tempo, reason) {
        if !matches!(reason, karakuri_environment::audio::Reason::Trim) {
            println!("beat: {reason:?} at {bpm:.1} bpm");
        }
    }
}

/// **A tap on the beat**, performed against the room this program is listening
/// to, and what to say about it.
///
/// # Why it does not go through `written`
///
/// Every other control on this panel emits an `Operation`, `written` turns it
/// into a `Record` and [`apply`] moves the deck with it — P-0028. A tap
/// **does** end in a record: `karakuri_environment::audio` writes a
/// `Record::Tempo` for it and applies it to the session's oscillator, which is
/// the same record a replay would hand the engine. What it cannot do is come
/// out of `written`: that function is a pure function of the operation and a
/// reading, and a tap's record is the *beat lock's* answer — the tapped tempo,
/// the phase error against the oscillator, the output lag — none of which a
/// `Current` carries. So `written(TapBeat)` answers `Owed(NotSettled)`, and
/// routing this key through [`App::performed`] would print *"nothing moved,
/// and nothing here decides it"* about a press that moved the grid.
///
/// **That is a gap in `karakuri-operation-record` and it is named here rather
/// than papered over**: the day a `Current` can carry a correction, this key
/// emits like every other control and this function goes. Until then it is
/// `karakuri-cli`'s own wiring, which is what the panel was asked to use.
fn tapped(
    open: &mut Option<audio::Audio>,
    deck: &mut Deck,
    at: Instant,
    started: Instant,
) -> String {
    let Some(open) = open.as_mut() else {
        return String::from(
            "tap: no audio input — a tap sets the grid this room is being tracked against, and \
             there is no room. open one on the transport row's `audio-in` pill",
        );
    };
    let mut signals = *deck.signals();
    let record = open.tap(&mut signals, at, started);
    deck.set_signals(signals);
    match record {
        Record::Tempo { bpm, shift, .. } => format!(
            "tap: -> Record::Tempo {{ bpm: {bpm:.1}, shift: {shift:+.3} }} — three taps or more \
             set the tempo and any tap sets the phase"
        ),
        other => format!("tap: -> {other:?}"),
    }
}

/// **The grid, an octave up or down**, performed against the same session, and
/// what to say about it.
///
/// [`tapped`]'s note about `written` word for word: `ScaleGrid` is the other
/// half of that `Owed(NotSettled)` arm, and for the same reason.
///
/// **Refused where the result would leave the trackable range**, which is the
/// lock's call and not this file's — 60 to 200 BPM is under two octaves wide,
/// so at most one of the two directions is ever live and a control that undid
/// itself two seconds later would be worse than one that says no.
fn scaled(open: &mut Option<audio::Audio>, deck: &mut Deck, by: GridScale) -> String {
    let (factor, word) = match by {
        GridScale::Halve => (0.5, "half"),
        GridScale::Double => (2.0, "double"),
    };
    let Some(open) = open.as_mut() else {
        return format!(
            "grid: no audio input — {word} moves the tracker's octave window, which only exists \
             while a room is being tracked. open one on the transport row's `audio-in` pill"
        );
    };
    let mut signals = *deck.signals();
    let moved = open.octave(&mut signals, factor);
    deck.set_signals(signals);
    match moved {
        Some(Record::Tempo { bpm, .. }) => format!(
            "grid: {word} -> Record::Tempo {{ bpm: {bpm:.1} }} — the tracker's window went with \
             it, and the phase did not move"
        ),
        Some(other) => format!("grid: {word} -> {other:?}"),
        None => format!(
            "grid: {word} refused — the result would leave the trackable range, and the next \
             estimate that disagreed would drag the grid straight back"
        ),
    }
}

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
    // **What every surface in this run reads and no surface decides**, made
    // here so that there is one of it: the four bay-head pills write it and
    // the MCP server reads it on every call, and a second handle would be a
    // pill that opens a class the server never sees. All four classes start
    // shut, which is the state ADR-0235 says a run starts in.
    let opening = Opening::closed();
    // **Before the window, for the reason the working copies are**: `serve`
    // binds a socket, and a socket that is already taken has to be a sentence
    // on a terminal. Everything after `run_app` is inside a `winit` callback,
    // where a panic aborts without a message.
    //
    // **Fatal, because `--mcp` was asked for.** A run that went on without it
    // would look exactly like one whose client is connected and idle.
    let mcp = match launch.mcp {
        Some(port) => {
            // **One pair per deck, and they are the working copies rather than
            // the two paths the operator typed.** The server addresses a slot
            // and reads and writes the files behind it, and the files behind a
            // deck are its own copy — see [`working_copies`]. Handing it the
            // typed paths would let a model rewrite the preset library.
            let slots = mcp::Slots(
                running
                    .iter()
                    .map(|pair| (pair.l1.clone(), vec![pair.l4.clone()]))
                    .collect(),
            );
            match mcp::serve(
                port,
                slots,
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
    // **The loop sleeps.** A frame is drawn when something changed it or when
    // `egui` asked for one after a delay it named, and on no other occasion —
    // `App::about_to_wait` sets this again after every iteration and is where
    // the rule actually lives. This is the state it starts in so that the
    // window between here and the first `about_to_wait` is not a spin either.
    event_loop.set_control_flow(ControlFlow::Wait);
    event_loop
        .run_app(&mut App::new(launch, running, held, mcp, opening))
        .expect("run");
}

// ---------------------------------------------------------------------------
// The tests that cannot leave this file
// ---------------------------------------------------------------------------

/// Everything about the model and the view is `tests/`, which
/// `cargo test -p karakuri-console` runs. What is left here is what is about a
/// `wgpu` type: the table below, and `egui` on a device under `mod gpu`.
#[cfg(test)]
mod tests {
    use super::*;
    /// **How far one press of the deck head's scrub goes, and the order its
    /// sync chip walks the modes in** — read from the control that has them
    /// rather than written again here: the arrow and the record it becomes
    /// are one number, and `Pane::allows` lines up with `SYNCS` or the cycle
    /// skips the wrong mode.
    use karakuri_console::view::{SCRUB_BEATS, SYNCS};
    /// The two answers that are not a record. `Silent` is named only here,
    /// because nothing in the running window reaches that arm; `Owed` is
    /// reachable only by a reading this window failed to take, which is the
    /// mask's accident and the sync chip's missing tempo and is asserted
    /// below. Neither is a gap in the vocabulary any more — the sync chip's
    /// was `Owed::NotSettled` until the conversion took a session tempo, and
    /// [`unwritten`] carries what that was.
    use karakuri_operation_record::{Owed, Silent};

    /// **The three things this program has decided about a room, and none of
    /// them needs a device.**
    ///
    /// The house rule for this pass was to say how what could not be opened
    /// was tested. This is it: `listening`'s judgement is [`unopened`], which
    /// is a pure function of an error, and the case that can only happen
    /// during a set — an input picked off a list and gone by the time it is
    /// opened — is reachable on any machine at all by picking a name no device
    /// can have. **What is deliberately not here is that a real input opens**;
    /// that is `karakuri-audio`'s ignored `the_default_input_opens_and_delivers`
    /// and no assertion in this file could stand in for it.
    ///
    /// 1. **No device at all is not a fault** (P-0034): the sentence says
    ///    `none` is a state, and says what goes on answering.
    /// 2. **A device that was named and is not there is loud** (P-0027): the
    ///    sentence carries the list, so an operator who picked a cable that has
    ///    gone is holding the right names rather than an invitation to go and
    ///    look.
    /// 3. **And a refused pick does not take the room away.** Nothing is open
    ///    in this test, so what is asserted is the half that can be: the answer
    ///    says so rather than going quiet.
    #[test]
    fn a_room_with_no_microphone_is_a_state_and_a_named_one_that_is_gone_is_a_refusal() {
        let quiet = unopened(
            "default",
            &audio::AudioError::NoMatch {
                wanted: "default".to_owned(),
                available: Vec::new(),
            },
        );
        assert!(
            quiet.contains("no audio inputs") && quiet.contains("not a fault"),
            "a machine with no inputs was told it had a problem: {quiet}"
        );
        assert!(
            !quiet.contains("VB-Cable"),
            "the empty case named a device: {quiet}"
        );

        let missing = unopened(
            "scarlett",
            &audio::AudioError::NoMatch {
                wanted: "scarlett".to_owned(),
                available: vec!["VB-Cable".to_owned(), "Built-in".to_owned()],
            },
        );
        assert!(
            missing.contains("scarlett")
                && missing.contains("VB-Cable")
                && missing.contains("Built-in"),
            "the refusal did not carry what the operator needs: {missing}"
        );
        assert!(
            !missing.contains("not a fault"),
            "a device somebody named and is not there was reported as a state: {missing}"
        );

        // **The case that can only arrive during a set**, on a machine with
        // whatever it happens to have plugged in: a pick nothing can match.
        // Nothing is opened — `pick` refuses before a stream is built — so
        // this runs anywhere and touches no hardware.
        let mut open: Option<audio::Audio> = None;
        let mut told_pill = Some(AudioIn::NONE);
        let line = attached(
            &mut open,
            120.0,
            &mut told_pill,
            &Operation::AttachBeatSource {
                source: BeatSource::AudioInput("\u{fffd}no such audio input\u{fffd}".to_owned()),
            },
        )
        .expect("`attached` answered nothing for an attach");
        assert!(
            line.contains("nothing is open"),
            "a refused pick said nothing about what is open now: {line}"
        );
        assert!(open.is_none(), "a refused pick opened something");

        // And a process is declined in one sentence rather than ignored.
        let process = attached(
            &mut open,
            120.0,
            &mut told_pill,
            &Operation::AttachBeatSource {
                source: BeatSource::Process("beats --stdout".to_owned()),
            },
        )
        .expect("`attached` ignored a beat source it cannot take");
        assert!(
            process.contains("tempo-source"),
            "a process was declined without saying where that half of the row lives: {process}"
        );

        // Every other operation is somebody else's, which is what keeps this
        // one line in `performed` rather than a second route into the device.
        assert_eq!(
            attached(&mut open, 120.0, &mut told_pill, &Operation::TapBeat),
            None
        );
    }

    /// **"Five milliseconds a press, down and up"** — `docs/manual/operations.html`'s
    /// row, and the sign `docs/manual/console.html` says is the half that gets
    /// read wrong at two in the morning: *"Negative and the picture waits for
    /// the music, positive and it leads."*
    ///
    /// A pair of keys wired the wrong way round reads correct and points
    /// backwards, and nothing an operator can see from the panel would say so
    /// — the console draws no offset. The step is asked of
    /// `karakuri_environment::audio` rather than transcribed, so this checks
    /// which way each key goes and that both go by the one constant the
    /// command line's own `o` and `p` use.
    #[test]
    fn o_steps_the_offset_down_and_p_steps_it_up_by_the_one_step_both_keyboards_use() {
        assert_eq!(
            offset_step("o"),
            Some(-audio::LATENCY_OFFSET_STEP_MS),
            "`o` is the key that makes the picture wait for the music, so it steps the offset \
             down"
        );
        assert_eq!(
            offset_step("p"),
            Some(audio::LATENCY_OFFSET_STEP_MS),
            "`p` is the key that makes the picture lead, so it steps the offset up"
        );
        assert_eq!(
            audio::LATENCY_OFFSET_STEP_MS,
            5.0,
            "the page says five milliseconds a press and the constant says otherwise — the page \
             is the specification, so one of the two is wrong and it is not this test"
        );
        // Every other key is somebody else's, which is what lets one arm read
        // the step out of the letter rather than two arms carrying a literal.
        for key in ["b", ",", ".", "r", "n", "0"] {
            assert_eq!(
                offset_step(key),
                None,
                "`{key}` is not an offset key and `offset_step` claimed it was"
            );
        }
    }

    /// **"Negative and the picture waits for the music, positive and it
    /// leads"**, and **"held inside 200 milliseconds either way"** — the two
    /// halves of `docs/manual/console.html`'s offset contract that a panel can
    /// be held to without a device.
    ///
    /// The second is the one a control is silent about by default: the value
    /// is clamped in `karakuri_environment::audio` and a press at the end of
    /// the travel would otherwise print the same number as the press before it
    /// with nothing said, which is P-0030's *an instrument says what it did*
    /// read from the far end.
    #[test]
    fn the_offset_says_which_way_it_points_and_says_when_it_was_held_at_the_bound() {
        let waiting = offset_said(-15.0, -15.0);
        assert!(
            waiting.contains("the picture waits for the music"),
            "a negative offset did not say which of the two is late: {waiting}"
        );
        let leading = offset_said(20.0, 20.0);
        assert!(
            leading.contains("the picture leads the music"),
            "a positive offset did not say which of the two is late: {leading}"
        );
        assert!(
            !leading.contains("as far either way as it goes"),
            "an offset nothing held claimed it was at the end of its travel: {leading}"
        );

        // The far end, asked for by one step and refused by five: the numbers
        // are the range's own, so a range that moved moves this with it.
        let top = *audio::LATENCY_OFFSET_RANGE.end();
        let held = offset_said(top + audio::LATENCY_OFFSET_STEP_MS, top);
        assert!(
            held.contains("as far either way as it goes"),
            "a press that asked past the bound and got the bound said nothing about it: {held}"
        );
        assert!(
            held.contains(&format!("{top:+.0} ms")),
            "the sentence about a clamped press does not carry the value it was held at: {held}"
        );
    }

    /// **"It only means anything with an audio input attached"** —
    /// `docs/manual/console.html`, and the operations page's row says it too.
    ///
    /// The offset is a term in the lead a beat correction is applied with, so
    /// with no room being listened to there is nothing for the picture to be
    /// early or late against and nothing to read the current value off. A
    /// value dialled against no session would be dropped the moment one opened
    /// — [`attached`] starts a new one at the offset the old one held and at
    /// the default when there was none — so the press changes nothing and says
    /// why, which is [`tapped`]'s and [`scaled`]'s answer to the same state.
    #[test]
    fn the_offset_keys_say_so_and_change_nothing_with_no_input_attached() {
        let mut open: Option<audio::Audio> = None;
        let line = nudged(&mut open, &Operation::SetLatencyOffset { ms: 25.0 })
            .expect("`nudged` answered nothing for an offset");
        assert!(
            line.contains("no audio input") && line.contains("audio-in"),
            "a press with nothing open did not say why or where the input is picked: {line}"
        );
        assert!(open.is_none(), "a press with nothing open opened something");
        // The same sentence the key arm prints before it builds an operation
        // at all, so the two paths into this state cannot drift apart.
        assert!(
            line.contains(NO_ROOM_FOR_AN_OFFSET),
            "the two ways into a panel with no room say two different things: {line}"
        );

        // Every other operation is somebody else's, on `attached`'s terms.
        assert_eq!(nudged(&mut open, &Operation::TapBeat), None);
    }

    /// **Every verdict the engine can report says what it does to the lane,
    /// and a row leaves it only when the file and the picture agree.**
    ///
    /// The five `swap::Event` variants are three answers: three that put a
    /// row on the lane under one of `view::Stage`'s words, one that takes it
    /// off, and one that is not about a version at all. The one worth the
    /// test is `Accepted`: it is the **watchdog's** verdict and not the
    /// operator's, and it is what clears a row because *keep a candidate* has
    /// no control on this panel — see `view::staging`, where that substitution
    /// is argued. A run in which `Accepted` did nothing would be a lane that
    /// fills up and never empties, which is not the lane the manual describes.
    ///
    /// **And the rows stay in slot order**, which is the order they are drawn
    /// in: a candidate that lands on deck B and then one on deck A must not
    /// leave the lane reading B over A, because the letter is the only thing
    /// telling two rows of the same material apart.
    ///
    /// A CPU test: an `Event` is a value, and nothing here takes a device.
    #[test]
    fn every_verdict_says_what_it_does_to_the_lane() {
        let landed = |label: &str| Event::Swapped {
            id: 1,
            label: label.into(),
        };
        let refused = |label: &str| Event::Rejected {
            id: 2,
            label: label.into(),
            error: karakuri_engine::set::SetError::NoCapacity("drift_shell".to_owned()),
        };
        let rolled = |label: &str| Event::RolledBack {
            id: 3,
            label: label.into(),
            median_ms: 24.0,
            budget_ms: 20.0,
        };
        let accepted = Event::Accepted {
            id: 4,
            label: "drift_shell + soft_points".into(),
            median_ms: 9.0,
            budget_ms: 20.0,
        };

        assert_eq!(
            verdict(&landed("a + b")),
            Verdict::Waiting("a + b", view::Stage::Landed)
        );
        assert_eq!(
            verdict(&refused("a + b")),
            Verdict::Waiting("a + b", view::Stage::Refused)
        );
        assert_eq!(
            verdict(&rolled("a + b")),
            Verdict::Waiting("a + b", view::Stage::RolledBack)
        );
        assert_eq!(
            verdict(&accepted),
            Verdict::Settled,
            "an accepted build leaves the file and the picture agreeing, so its row stays \
             on a lane nothing can clear"
        );
        assert_eq!(
            verdict(&Event::WorkerLost),
            Verdict::Nothing,
            "the worker going is not a verdict on any version, and every row already \
             taken still stands"
        );

        // And the same three through `settle`, which is what a drain does with
        // them: one row per slot, in slot order, rewritten in place.
        let mut lane: Vec<view::Candidate> = Vec::new();
        settle(&mut lane, 1, "b + b", view::Stage::Landed);
        settle(&mut lane, 0, "a + a", view::Stage::Landed);
        assert_eq!(
            lane.iter().map(|row| row.deck).collect::<Vec<_>>(),
            vec![0, 1],
            "the rows are not in the order the letters are drawn in"
        );

        settle(&mut lane, 0, "a + a", view::Stage::RolledBack);
        assert_eq!(
            lane.len(),
            2,
            "a second verdict on one slot made a second row"
        );
        assert_eq!(lane[0].stage, view::Stage::RolledBack);
        assert_eq!(
            lane[0].name, "a + a",
            "the name was not left as it was found"
        );

        settle(&mut lane, 0, "c + c", view::Stage::Landed);
        assert_eq!(
            lane[0].name, "c + c",
            "a rebuild of other material kept the old name"
        );

        lane.retain(|row| row.deck != 0);
        assert_eq!(
            lane.iter().map(|row| row.deck).collect::<Vec<_>>(),
            vec![1],
            "clearing one slot's row took another slot's with it"
        );
    }

    /// **The library is what the store holds, and a store that is not there is
    /// listed as nothing rather than created.**
    ///
    /// Two claims, and the second is the one worth a test: `Store::open`
    /// establishes the layout it is pointed at, so a listing that opened first
    /// would leave a `.karakuri` behind in whatever directory this program was
    /// run from. [`library`] asks whether the root is there before it opens
    /// anything, and this is what says so.
    ///
    /// A CPU test: nothing here takes a device, and the store is a directory.
    #[test]
    fn a_library_is_the_store_and_a_missing_store_is_not_made() {
        let root = std::env::temp_dir().join(format!(
            "karakuri-console-library-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        // Whatever a previous run left, so the first claim is about a root
        // that is genuinely not there.
        let _ = std::fs::remove_dir_all(&root);

        assert!(
            library(&root).is_empty(),
            "a store that is not there listed something"
        );
        assert!(
            !root.exists(),
            "listing a library that is not there created one at {}",
            root.display()
        );

        // And with two Sets in it, both names come back — in the order
        // `list_sets` sorts them, which is the order the bay draws.
        let store = Store::open(&root).expect("a store to list");
        for id in ["night01", "morph01"] {
            store
                .write_set(id, &[])
                .unwrap_or_else(|e| panic!("writing {id}: {e}"));
        }
        let listed = library(&root);
        assert_eq!(
            listed,
            vec!["morph01".to_owned(), "night01".to_owned()],
            "the bay lists {listed:?}"
        );

        std::fs::remove_dir_all(&root).expect("clean up");
    }

    /// A temporary directory of this test's own, named after the test that
    /// wants it — the shape every other CPU test in this file uses.
    pub(crate) fn scratch_dir(what: &str) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!(
            "karakuri-{what}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        // Whatever a previous run left behind, so the claims are about what
        // this run put there.
        let _ = std::fs::remove_dir_all(&root);
        root
    }

    /// `examples/`, as a preset library this run was told about.
    fn shipped_presets() -> karakuri_environment::places::Presets {
        karakuri_environment::places::Presets {
            dir: std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples"),
            found: karakuri_environment::places::Found::Given,
        }
    }

    /// **The `presets` scope lists the Set files and not the parts beside
    /// them**, which is `console.html`'s *"a directory of `.kir` files is a
    /// directory of parts, and a library lists what you can put on a deck"*.
    ///
    /// It is asserted against `examples/`, which is the directory this program
    /// actually opens on: thirty-one parts and twenty-one Set files in one
    /// place is exactly the mixture the rule is about, and a listing that took
    /// the parts would draw fifty-two rows of which thirty-one name nothing
    /// this vocabulary can load.
    ///
    /// A CPU test: a preset library is a directory.
    #[test]
    fn the_presets_scope_lists_the_kset_files_and_not_the_parts_beside_them() {
        let presets = shipped_presets();
        let listed = presets_listing(Some(&presets));
        assert!(
            listed.len() >= 20,
            "`examples/` holds twenty-one `.kset` files and the listing found {}",
            listed.len()
        );
        for preset in &listed {
            assert!(
                preset.path.extension().and_then(|e| e.to_str()) == Some("kset"),
                "`{}` is listed and is not a Set file",
                preset.path.display()
            );
            assert!(
                !preset.id.ends_with(".kset") && !preset.id.is_empty(),
                "the row reads `{}`, which is a file name rather than a name",
                preset.id
            );
        }
        assert!(
            listed.iter().any(|preset| preset.id == "beat_cloud"),
            "`beat_cloud.kset` is in `examples/` and the listing does not have it"
        );
        assert!(
            !listed
                .iter()
                .any(|preset| preset.id.contains("drift_shell")),
            "`drift_shell.kir` is a part and the listing took it for a row"
        );

        // **Sorted, because a directory read is not.** Two runs that drew the
        // rows in two orders would be a bay nobody can point at.
        let mut sorted = listed.iter().map(|p| p.id.clone()).collect::<Vec<_>>();
        sorted.sort();
        assert_eq!(
            listed.iter().map(|p| p.id.clone()).collect::<Vec<_>>(),
            sorted,
            "the listing is not in name order"
        );

        // And no preset library at all is no rows, which is a state rather
        // than a failure.
        assert!(presets_listing(None).is_empty());
    }

    /// **Loading a preset takes it into the store, so `my sets` gains a row
    /// nobody made** — `console.html`'s *"which is why opening a preset leaves
    /// one of your own behind"*.
    ///
    /// The whole of the press is asserted here except the aim, which is
    /// [`loading`]'s and has its own test below: what a preset row adds is the
    /// packaging in front of it, and the claim is that after it the id is one
    /// the store holds and one `my sets` lists — which is what makes the load
    /// after it the same route a `my sets` row takes rather than a second one.
    ///
    /// A CPU test: a store is a directory, and resolving a `.kset` is a read,
    /// a hash and a store put.
    #[test]
    fn loading_a_preset_takes_it_in_and_leaves_it_under_my_sets() {
        let root = scratch_dir("preset-take-in");
        let presets = shipped_presets();
        Store::open(&root).expect("a store to take a preset into");

        assert!(
            library(&root).is_empty(),
            "a fresh store lists something under `my sets`"
        );
        let taken = taking_in(&root, Some(&presets), "beat_cloud")
            .unwrap_or_else(|e| panic!("`beat_cloud` was not taken in: {e}"));
        assert_eq!(
            taken.id, "beat_cloud",
            "the id is the file's own `set` record and not the row's word"
        );
        assert!(
            taken.said.contains("took `beat_cloud` in"),
            "the report the operator reads is `{}`",
            taken.said
        );
        assert_eq!(
            library(&root),
            vec!["beat_cloud".to_owned()],
            "the preset was taken in and `my sets` does not list it"
        );

        // **And the parts are in the store**, which is what makes the load
        // after this a load of material the store holds: `setfile::load` is
        // what the aim is built from and it reads them by address.
        karakuri_environment::setfile::load(&Store::open(&root).expect("the store"), "beat_cloud")
            .expect("the Set that was just taken in cannot be read back");

        std::fs::remove_dir_all(&root).expect("clean up");
    }

    /// **A take-in names the file it read, because that is what the operation
    /// carries** — `SetTransfer::Take`'s own sentence, *"a path because a file
    /// is what the only existing route takes"*.
    ///
    /// The id and the file are two different answers and the row needs both:
    /// the load after the press names the id, and the transfer names the file.
    /// The claim here is that the file is the row's own `.kset` in the preset
    /// library and not something re-derived afterwards — asking the listing a
    /// second time to name what was already taken in would be two answers to
    /// *which file was this* with a directory read between them.
    ///
    /// A CPU test, for the test above's reason.
    #[test]
    fn a_take_in_names_the_file_it_read_because_that_is_what_the_operation_carries() {
        let root = scratch_dir("preset-take-in-file");
        let presets = shipped_presets();
        Store::open(&root).expect("a store to take a preset into");

        let taken = taking_in(&root, Some(&presets), "beat_cloud")
            .unwrap_or_else(|e| panic!("`beat_cloud` was not taken in: {e}"));
        assert_eq!(
            taken.file.file_name().and_then(|n| n.to_str()),
            Some("beat_cloud.kset"),
            "the take-in named `{}`, which is not the row's own authoring file",
            taken.file.display()
        );
        assert!(
            taken.file.starts_with(&presets.dir),
            "the take-in named `{}`, which is outside the preset library at `{}`",
            taken.file.display(),
            presets.dir.display()
        );
        assert!(
            taken.file.is_file(),
            "the take-in named `{}` and there is no file there",
            taken.file.display()
        );

        std::fs::remove_dir_all(&root).expect("clean up");
    }

    /// **A press on a `presets` row is *Send a Set to somebody, and take one
    /// in* performed at the second of its two moments, and then the load** —
    /// so it emits both, in that order.
    ///
    /// `docs/manual/operations.html`: *"loading a Set out of presets or out of
    /// a folder is this row performed at the second of them"*, and
    /// `console.html`: *"A row here is taken into the store and then loaded,
    /// which is one press because taking it in is what gives it a name."*
    /// **Two rows of the page and one press**, and what this defends is that
    /// the press names both of them: emitting only the load would be a press
    /// that performs two rows and names one, and the row it dropped is the one
    /// nothing else in this workspace constructs.
    ///
    /// The titles are asked of [`Operation::title`] rather than written out
    /// here, so a heading that moves on the page moves in one place.
    ///
    /// A CPU test: it builds two values.
    #[test]
    fn a_press_on_a_preset_row_names_the_take_in_and_then_the_load() {
        let root = scratch_dir("preset-press");
        let presets = shipped_presets();
        Store::open(&root).expect("a store to take a preset into");

        let taken = taking_in(&root, Some(&presets), "beat_cloud")
            .unwrap_or_else(|e| panic!("`beat_cloud` was not taken in: {e}"));
        let file = taken.file.clone();
        let [take, load] = preset_press(2, taken);

        assert_eq!(
            take,
            Operation::TransferSet {
                transfer: SetTransfer::Take { file }
            },
            "the first of the two is not the take-in, or it does not name the file it read"
        );
        assert_eq!(
            take.title(),
            "Send a Set to somebody, and take one in",
            "the first of the two does not name the row the press performed"
        );
        assert_eq!(
            load,
            Operation::LoadSet {
                deck: 2,
                set: "beat_cloud".to_owned()
            },
            "the second of the two is not the load, or it does not name the id the file filed \
             itself under"
        );
        assert_eq!(
            load.title(),
            "Load material into a deck",
            "the second of the two does not name the row the press performed"
        );

        std::fs::remove_dir_all(&root).expect("clean up");
    }

    /// **The take-in the press names writes no record, and that is settled** —
    /// which is why emitting it is a naming rather than a second route into
    /// anything.
    ///
    /// `karakuri-operation-record` answers `Silent(NoRecord)` for a transfer:
    /// nothing in the session vocabulary carries a Set arriving from
    /// somewhere else. So [`App::performed`] performs nothing for it, exactly
    /// as it performs nothing for the scope step `e` emits, and
    /// [`unwritten`] is what an operator reads. **If that ever became
    /// `Owed`**, the press would be emitting a gap rather than a settled
    /// silence and this file would be the place to say so.
    ///
    /// A CPU test: it is a `match` on an operation.
    #[test]
    fn the_take_in_the_press_names_writes_no_record_and_that_is_settled() {
        let take = Operation::TransferSet {
            transfer: SetTransfer::Take {
                file: std::path::PathBuf::from("night01.kset"),
            },
        };
        assert_eq!(
            written(&take, &Current::default()),
            Written::Silent(Silent::NoRecord),
            "a transfer the press emits no longer writes a settled nothing"
        );
        let said = unwritten(&take, &written(&take, &Current::default()))
            .expect("a press that wrote no record says so");
        assert!(
            said.contains("no record, and that is settled"),
            "what the operator reads is `{said}`"
        );
    }

    /// **An id this store already holds is refused rather than overwritten,
    /// and the operator is told which of the two acts failed.**
    ///
    /// The refusal is `setfile::unbundle`'s and is not written twice —
    /// *"an id already taken is refused rather than overwritten"* — so what is
    /// asserted here is that the press goes through it: a second press on the
    /// same preset row leaves the store exactly as the first one left it, and
    /// the sentence names the id rather than the file.
    ///
    /// A CPU test, for the test above's reason.
    #[test]
    fn a_preset_whose_id_this_store_holds_is_refused_rather_than_overwritten() {
        let root = scratch_dir("preset-refused");
        let presets = shipped_presets();
        Store::open(&root).expect("a store to take a preset into");

        taking_in(&root, Some(&presets), "beat_cloud").expect("the first take-in");
        let held = library(&root);

        let refused = taking_in(&root, Some(&presets), "beat_cloud")
            .expect_err("the same preset was taken in twice");
        assert!(
            refused.contains("already in this store") && refused.contains("beat_cloud"),
            "the refusal an operator reads is `{refused}`"
        );
        assert_eq!(
            library(&root),
            held,
            "a refused take-in changed what the store holds"
        );

        // And a row that is not in the preset library at all is refused
        // saying so, which is the other way a press finds nothing: the listing
        // is asked again on the press, so a file that has moved is met here
        // rather than inside the packaging.
        let gone = taking_in(&root, Some(&presets), "no_such_preset")
            .expect_err("a preset that is not there was taken in");
        assert!(
            gone.contains("no_such_preset"),
            "the refusal an operator reads is `{gone}`"
        );

        std::fs::remove_dir_all(&root).expect("clean up");
    }

    /// **A scope is a listing on this side, and stepping to one answers it** —
    /// two of the four with rows, and two with nothing and a sentence saying
    /// which kind of nothing it is.
    ///
    /// The two that answer nothing are the whole point of the test: they are
    /// empty for two *different* reasons — a favourite is a fact nothing in
    /// this workspace keeps, and a folder waits on an operation that can ask a
    /// directory for its listing — and a program that said the same thing
    /// about both would be hiding one of them.
    ///
    /// A CPU test: a `View` takes no device.
    #[test]
    fn every_scope_is_answered_and_the_two_that_answer_nothing_say_which_nothing() {
        let root = scratch_dir("scope-listing");
        let presets = shipped_presets();
        let store = Store::open(&root).expect("a store to list");
        store.write_set("night01", &[]).expect("a Set to list");

        let mut view = View::new(Room::Day);
        view.scopes = Scope::ALL.to_vec();
        assert!(view.select_scope(Scope::MySets));

        let said = listing(&mut view, &root, Some(&presets));
        assert_eq!(view.library, vec!["night01".to_owned()]);
        assert!(said.contains("my sets") && said.contains('1'), "{said}");

        assert!(view.step_scope(), "the scope did not step");
        assert_eq!(view.scope(), Some(Scope::Presets));
        let said = listing(&mut view, &root, Some(&presets));
        assert!(
            view.library.iter().any(|id| id == "beat_cloud"),
            "the `presets` scope lists {:?}",
            view.library
        );
        assert!(said.contains("presets"), "{said}");

        // The two that are drawn and answer nothing, and the sentences they
        // answer with are not one sentence.
        for scope in [Scope::Favourites, Scope::Folder] {
            assert!(view.select_scope(scope));
            let said = listing(&mut view, &root, Some(&presets));
            assert!(
                view.library.is_empty(),
                "`{}` listed {:?}, and nothing in this workspace can produce it",
                scope.name(),
                view.library
            );
            assert!(
                said.contains(scope.name()) && said.contains(why_nothing(scope)),
                "`{}` lists nothing and says `{said}`",
                scope.name()
            );
        }
        assert_ne!(
            why_nothing(Scope::Favourites),
            why_nothing(Scope::Folder),
            "the two scopes that answer nothing are empty for two different reasons and this \
             program gives one sentence for both"
        );
        assert!(
            why_nothing(Scope::Favourites).contains("favourite"),
            "the `favourites` sentence does not say what is missing: {}",
            why_nothing(Scope::Favourites)
        );
        assert!(
            why_nothing(Scope::Folder).contains("operation"),
            "the `folder` sentence does not say what it is waiting on: {}",
            why_nothing(Scope::Folder)
        );

        std::fs::remove_dir_all(&root).expect("clean up");
    }

    /// **A load writes the Set's procedures where the slot's watcher is
    /// looking and aims it there, and it touches no deck at all.**
    ///
    /// This is the whole of what `Operation::LoadSet` needed, and what it is
    /// *not* is the claim: `Deck::install` is the one function that puts a
    /// built Set in a slot and is documented as deliberately unreachable from
    /// a key or a surface, because *"a live run changes its material by
    /// editing a file and letting the worker build it, which is what the
    /// budget watchdog is attached to"*. So this asserts files and an aim.
    /// A load that built a Set here would be a picture nothing measured, on a
    /// deck with no previous Set parked to roll back to.
    ///
    /// **Three things beyond *it happened*, and each is a wrong load that
    /// looks right.** The scratch name carries the deck letter and the node's
    /// place, because `scratch::place` overwrites by name and two decks
    /// loading Sets whose procedures share one would silently become one file
    /// — the second load moving the first deck on its watcher's next poll.
    /// The aim restates the layering, the fold, the capacity and the salts the
    /// *file* recorded rather than the ones the slot was running at, because
    /// that is the failure every `Watch` field is documented against and it
    /// does not show on the load: it shows on the first save afterwards. And
    /// the node names are the file's, because an `edge` resolves against them.
    ///
    /// A CPU test: a store is a directory, and nothing here takes a device.
    #[test]
    fn a_load_writes_the_sets_procedures_into_the_scratch_and_aims_the_slot_there() {
        use karakuri_environment::setfile;
        use karakuri_store::Hash;

        let root = std::env::temp_dir().join(format!(
            "karakuri-load-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let store = Store::open(&root).expect("a store to load from");

        // Two real procedures, so the load goes through the checker the way a
        // Set out of the library does. `lattice_shell` is chosen for its name:
        // it is what the scratch file has to be called after.
        let examples = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
        let sources: Vec<(karakuri_store::Layer, String)> = [
            (karakuri_store::Layer::L1, "lattice_shell.kir"),
            (karakuri_store::Layer::L4, "soft_points.kir"),
        ]
        .into_iter()
        .map(|(layer, file)| {
            let src = std::fs::read_to_string(examples.join(file)).expect("an example");
            (layer, src)
        })
        .collect();
        let nodes: Vec<setfile::Node> = sources
            .iter()
            .map(|(layer, src)| setfile::Node {
                hash: {
                    let hash = Hash::of(src.as_bytes());
                    store.put_artifact(src.as_bytes()).expect("store a source");
                    hash
                },
                layer: match layer {
                    karakuri_store::Layer::L1 => karakuri_ir::Kind::L1,
                    _ => karakuri_ir::Kind::L4,
                },
                index: 0,
                // A name the file wrote, which is what a rebuild has to call
                // the node — not the procedure's own.
                name: Some(match layer {
                    karakuri_store::Layer::L1 => "grid".to_owned(),
                    _ => "draw".to_owned(),
                }),
            })
            .collect();
        // Values no default here produces, so an aim that kept the slot's own
        // cannot pass by accident.
        let seeds = [0x0bad_cafeu32];
        setfile::save(
            &store,
            "night01",
            setfile::Saving {
                nodes: &nodes,
                capacities: &[2048],
                params: &[],
                bindings: &[],
                edges: &[],
                camera: &karakuri_engine::camera::Orbit::default(),
                layering: Layering::Composite,
                live: Some(0),
                seeds: &seeds,
            },
        )
        .expect("write the Set file");

        // Deck B, so the letter in the scratch name is not the first one and a
        // hard-coded `A` fails here.
        let (tx, rx) = std::sync::mpsc::channel();
        // **An [`Aiming`] and not a bare sender**, because a load keeps where it
        // pointed the watcher — see the assertion at the end of this test.
        let mut aiming = Aiming {
            aim: tx,
            at: watch::Aim {
                head: karakuri_environment::compile::Named::bare("nowhere.kir"),
                rest: Vec::new(),
                layering: Layering::Overdraw,
                live: None,
                capacity: None,
                seed_salt: 0,
                salts: Vec::new(),
                camera: karakuri_engine::camera::Orbit::default(),
                overrides: Vec::new(),
                published: Vec::new(),
                bindings: Vec::new(),
                edges: Vec::new(),
                authorities: Vec::new(),
            },
        };
        let line = loading(
            &root,
            ASKED_TO_PRIME,
            slot_salt(ASKED_TO_PRIME),
            &mut aiming,
            "night01",
        )
        .unwrap_or_else(|e| panic!("the load failed: {e}"));
        assert!(
            line.contains("deck B"),
            "the line does not say where: {line}"
        );

        let aim = rx.try_recv().expect("the slot was aimed at something");
        assert_eq!(aim.rest.len(), 1, "the renderer did not travel with it");
        assert_eq!(
            aim.head.name.as_deref(),
            Some("grid"),
            "the head is not called what the file called it, so an edge would not resolve"
        );
        for (named, (_, src)) in std::iter::once(&aim.head)
            .chain(aim.rest.iter())
            .zip(&sources)
        {
            let name = named
                .path
                .file_name()
                .and_then(|n| n.to_str())
                .expect("a file name");
            assert!(
                name.starts_with("B0-") || name.starts_with("B1-"),
                "`{name}` carries neither the deck nor the node, so two decks would share it"
            );
            assert_eq!(
                std::fs::read_to_string(&named.path).expect("the scratch file"),
                *src,
                "the watcher is pointed at a file that is not the Set's source"
            );
        }

        assert_eq!(aim.layering, Layering::Composite, "the file's layering");
        assert_eq!(aim.live, Some(0), "the file's fold");
        assert_eq!(aim.capacity, Some(2048), "the file's capacity");
        assert_eq!(aim.seed_salt, seeds[0], "the file's seed");
        assert_eq!(aim.salts, vec![seeds[0]], "the file's salts");
        assert!(
            aim.authorities.is_empty(),
            "a Set file carries no grant, so a load must hand none over"
        );

        // **And the load kept where it pointed the watcher**, which is what a
        // later rewiring restates the other twelve fields from: an `Aiming` that
        // sent an aim and left `at` behind would re-aim this slot at the pair
        // the run launched with. See [`Aiming`].
        assert_eq!(
            aiming.at.head.name.as_deref(),
            Some("grid"),
            "the load sent an aim and did not keep it"
        );
        assert_eq!(aiming.at.live, Some(0), "the kept aim is not the sent one");

        // And a Set that is not there is a sentence with nothing sent: the
        // deck goes on playing what it was.
        let e = loading(&root, ON_AIR, slot_salt(ON_AIR), &mut aiming, "nothing01")
            .expect_err("a Set that is not in the store");
        assert!(e.contains("nothing01"), "the refusal does not name it: {e}");
        assert!(
            rx.try_recv().is_err(),
            "a load that failed aimed the slot anyway"
        );
        assert_eq!(
            aiming.at.live,
            Some(0),
            "a load that failed moved where the watcher is pointed"
        );

        std::fs::remove_dir_all(&root).expect("clean up");
    }

    /// **The requirement this program was failing**: the same preset loaded
    /// into every slot, and each deck watching its own separate copy in its own
    /// place.
    ///
    /// Every slot used to be handed [`Sources`] itself — the two paths the
    /// operator typed — so four watchers polled two files. One save rebuilt
    /// four slots, and since a parked slot's trial never reaches a verdict,
    /// three of the four rows it put in the Staging lane stayed there for the
    /// rest of the run. That symptom is this defect's, not the lane's.
    ///
    /// **Four things, and the third is the one the requirement is about.** The
    /// copies are under the scratch and not where the operator pointed; the
    /// four decks hold eight distinct files rather than two shared ones; an
    /// edit made through deck B's L1 moves deck B and **no other deck**; and
    /// the file the operator named is not written to at all.
    ///
    /// A CPU test: a scratch is a directory and nothing here takes a device.
    #[test]
    fn every_deck_runs_from_its_own_copy_and_an_edit_moves_one_deck() {
        let root = scratch_dir("own-copy");
        let named = shipped();
        let (dir, running) =
            working_copies(&root, &named, SLOTS).unwrap_or_else(|e| panic!("no copies: {e}"));

        assert_eq!(
            running.len(),
            SLOTS,
            "a deck of {SLOTS} slots got {running:?}"
        );
        assert_eq!(dir, root.join(karakuri_environment::scratch::DIR));

        // 1. Nothing a deck holds points at what the operator typed.
        for (slot, pair) in running.iter().enumerate() {
            for path in [&pair.l1, &pair.l4] {
                assert!(
                    path.starts_with(&dir),
                    "deck {} still runs from {}",
                    deck_letter(slot as u8),
                    path.display()
                );
            }
        }

        // 2. Eight files and not two, and each says which deck it belongs to.
        let mut every: Vec<&std::path::PathBuf> =
            running.iter().flat_map(|p| [&p.l1, &p.l4]).collect();
        let held = every.len();
        every.sort();
        every.dedup();
        assert_eq!(
            every.len(),
            held,
            "{SLOTS} decks on one pair share a file, so an edit cannot reach one of them"
        );
        for (slot, pair) in running.iter().enumerate() {
            let letter = deck_letter(slot as u8);
            for (at, path) in [&pair.l1, &pair.l4].into_iter().enumerate() {
                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .expect("a file name");
                assert!(
                    name.starts_with(&format!("{letter}{at}-")),
                    "`{name}` carries neither the deck nor the node's place"
                );
            }
        }

        // 3. **The claim.** An edit in one place moves one deck.
        let before = std::fs::read_to_string(&running[ON_AIR].l1).expect("deck A's L1");
        std::fs::write(&running[ASKED_TO_PRIME].l1, "deck B only").expect("edit deck B");
        assert_eq!(
            std::fs::read_to_string(&running[ASKED_TO_PRIME].l1).expect("read"),
            "deck B only"
        );
        for (slot, deck) in running.iter().enumerate().take(SLOTS) {
            if slot == ASKED_TO_PRIME {
                continue;
            }
            assert_eq!(
                std::fs::read_to_string(&deck.l1).expect("read"),
                before,
                "editing deck B moved deck {} as well",
                deck_letter(slot as u8)
            );
        }

        // 4. And the preset is what it was, which is the whole reason the
        // scratch exists: three shipped presets were replaced in one session.
        assert_eq!(
            std::fs::read_to_string(&named.l1).expect("the preset"),
            before,
            "the file the operator named was written to"
        );

        std::fs::remove_dir_all(&root).expect("clean up");
    }

    /// **The program has to say it.** Four decks on one preset are four files
    /// whose names an operator cannot guess and cannot tell apart by content —
    /// at startup they hold the same bytes — so the startup print names the
    /// directory and then one file pair per deck.
    #[test]
    fn the_startup_print_names_one_file_per_deck() {
        let root = scratch_dir("own-copy-said");
        let (dir, running) = working_copies(&root, &shipped(), SLOTS).expect("copies");
        let said = running_from(&dir, &running);

        assert!(
            said.contains(&dir.display().to_string()),
            "the print does not say where: {said}"
        );
        for (slot, pair) in running.iter().enumerate() {
            let letter = deck_letter(slot as u8);
            assert!(
                said.contains(&format!("deck {letter}:")),
                "deck {letter} is not in the print: {said}"
            );
            for path in [&pair.l1, &pair.l4] {
                let name = path.file_name().and_then(|n| n.to_str()).expect("a name");
                assert!(
                    said.contains(name),
                    "`{name}` is a file the deck runs from and the print does not name it: \
                     {said}"
                );
            }
        }
        // **And the four lines are four different answers.** A print that
        // named the same two files under all four decks would be a print an
        // operator cannot act on — which is exactly what this program said
        // while every deck watched the pair that was typed.
        let lines: Vec<&str> = said
            .lines()
            .filter(|line| line.trim_start().starts_with("deck "))
            .collect();
        assert_eq!(lines.len(), SLOTS, "one line per deck, and got {lines:?}");
        let named: Vec<&str> = lines
            .iter()
            .map(|line| line.split_once(':').expect("`deck A: files`").1)
            .collect();
        let mut distinct = named.clone();
        distinct.sort_unstable();
        distinct.dedup();
        assert_eq!(
            distinct.len(),
            named.len(),
            "two decks were told to open the same file: {named:?}"
        );

        // And it says the thing an operator will otherwise read as a bug.
        assert!(
            said.contains("not written to"),
            "the print does not say the named paths are left alone: {said}"
        );

        std::fs::remove_dir_all(&root).expect("clean up");
    }

    /// A root of this test's own, cleaned of whatever a previous run left.
    fn arrangement_root(what: &str) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!(
            "karakuri-arrangement-{what}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        root
    }

    /// A panel with something folded and something soloed — the two things an
    /// arrangement is kept for, and the two a reset forgets.
    fn arranged(width: f32, height: f32) -> Panel {
        let mut panel = Panel::new(width, height);
        let staging = panel.layout().find("staging").expect("a staging bay");
        panel.op(Op::Fold(staging));
        let mixer = panel.layout().find("mixer").expect("a mixer bay");
        panel.op(Op::Solo(mixer));
        panel.solve();
        panel
    }

    /// **The bytes are at the path `karakuri-store`'s own header claims**, read
    /// off that path and not through the store that wrote them.
    ///
    /// This is the assertion ADR-0221 §4 says the store's suite had to spell
    /// out rather than leave to a round trip: *"a format test is not a location
    /// test"*, because a defect that files the arrangement in the wrong
    /// directory entirely is invisible to a test that writes and reads through
    /// the same wrong path. The same hole is open one layer up — this file
    /// chooses the name it hands over — so the same assertion is made here,
    /// about `arrangements/<name>.arrangement.json` under the store's root.
    #[test]
    fn an_arrangement_is_kept_at_the_path_the_stores_header_names() {
        let root = arrangement_root("kept");
        let mut panel = arranged(1280.0, 720.0);

        let said = arrangement(
            &root,
            &mut panel,
            &mut view::Arrangement::default(),
            &Operation::SaveArrangement {
                name: "four_deck".to_owned(),
            },
        )
        .expect("a save is one of the two operations this route answers for");

        let at = root.join("arrangements").join("four_deck.arrangement.json");
        let bytes = std::fs::read(&at).unwrap_or_else(|e| {
            panic!(
                "nothing at {} after `{said}` — a saved arrangement is one path component of \
                 name, one of what it is, and one of the format it is in, under the store's \
                 fourth directory: {e}",
                at.display()
            )
        });

        // And what is at that path is this panel's arrangement rather than
        // some other document that happens to be there.
        let back: Layout =
            serde_json::from_slice(&bytes).expect("the bytes at that path are an arrangement");
        assert!(
            back.is_soloed(),
            "the file at {} did not carry the solo the panel was saved with",
            at.display()
        );

        // Nothing else was created under the store: an arrangement is a fourth
        // thing beside `sets/`, `sessions/` and the artifacts, and not one of
        // them.
        assert!(
            !root.join("sets").join("four_deck.kbset").exists(),
            "the arrangement was filed as a Set"
        );

        std::fs::remove_dir_all(&root).expect("clean up");
    }

    /// **What comes back is the arrangement that was kept, in the window it
    /// arrives in** — which is `Op::Reset` carrying the viewport across, with
    /// the arrangement handed in rather than built.
    ///
    /// The two viewports differ in both axes on purpose: an arrangement
    /// carries the viewport it was saved at, so a restore that took the file's
    /// would open a console arranged on a desktop inside a smaller window with
    /// every rectangle past the edge.
    #[test]
    fn an_arrangement_put_back_arrives_in_the_window_this_one_already_has() {
        let root = arrangement_root("back");
        let mut saved = arranged(1920.0, 1080.0);
        arrangement(
            &root,
            &mut saved,
            &mut view::Arrangement::default(),
            &Operation::SaveArrangement {
                name: "night_b".to_owned(),
            },
        )
        .expect("a save");

        // A window of a different size, with nothing folded and nothing soloed.
        let mut window = Panel::new(1280.0, 720.0);
        let staging = window.layout().find("staging").expect("a staging bay");
        assert!(!window.layout().is_collapsed(staging));

        let said = arrangement(
            &root,
            &mut window,
            &mut view::Arrangement::default(),
            &Operation::RestoreArrangement {
                name: "night_b".to_owned(),
            },
        )
        .expect("a restore");

        let now = window.layout();
        assert_eq!(
            (now.viewport().w, now.viewport().h),
            (1280.0, 720.0),
            "`{said}` — the arrangement brought the window it was saved at with it. The window \
             is the operator's and never the file's"
        );
        assert!(
            now.is_soloed() && now.is_collapsed(now.find("staging").expect("staging")),
            "`{said}` — the fold and the solo did not come back, so what was put back is not \
             what was kept"
        );

        std::fs::remove_dir_all(&root).expect("clean up");
    }

    /// **A name nothing is filed under is said back, and the console does not
    /// move.**
    ///
    /// The refusal that matters most in this family: `read_arrangement` never
    /// falls back to the built-in, so an operator who mistyped a name is told
    /// the name rather than watching their console reset (ADR-0221 §2). Asked
    /// three ways, because the three failures send an operator to three
    /// different places — no store at all, no such name, and a file that will
    /// not read back.
    #[test]
    fn a_name_nothing_is_filed_under_is_said_back_and_nothing_resets() {
        let root = arrangement_root("refused");
        let mut panel = arranged(1280.0, 720.0);
        let kept: Vec<bool> = panel
            .nodes()
            .iter()
            .map(|n| panel.layout().is_collapsed(n.id))
            .collect();
        let unchanged = |panel: &Panel, said: &str| {
            let now: Vec<bool> = panel
                .nodes()
                .iter()
                .map(|n| panel.layout().is_collapsed(n.id))
                .collect();
            assert_eq!(
                now, kept,
                "`{said}` and the arrangement moved — a refusal that resets the console is the \
                 one thing `read_arrangement` promises never to do"
            );
            assert!(
                panel.layout().is_soloed(),
                "`{said}` and the solo went — see above"
            );
        };

        // 1. No store at all, and asking a question does not make one.
        let said = arrangement(
            &root,
            &mut panel,
            &mut view::Arrangement::default(),
            &Operation::RestoreArrangement {
                name: "four_deck".to_owned(),
            },
        )
        .expect("a restore");
        assert!(
            said.contains("four_deck"),
            "the refusal did not say the name back: {said}"
        );
        assert!(
            !root.exists(),
            "putting an arrangement back that is not there created a store at {}",
            root.display()
        );
        unchanged(&panel, &said);

        // 2. A store, and no such name in it.
        Store::open(&root).expect("a store");
        let said = arrangement(
            &root,
            &mut panel,
            &mut view::Arrangement::default(),
            &Operation::RestoreArrangement {
                name: "four_deck".to_owned(),
            },
        )
        .expect("a restore");
        assert!(
            said.contains("four_deck"),
            "the refusal did not say the name back: {said}"
        );
        unchanged(&panel, &said);

        // 3. A file filed under the name that is not an arrangement. Refused
        //    whole rather than repaired (ADR-0158), and told apart from
        //    *there is no such arrangement*, which is the distinction the
        //    sentence carries.
        Store::open(&root)
            .expect("a store")
            .write_arrangement("four_deck", b"{\"nodes\":[]}")
            .expect("bytes the store does not have to understand");
        let said = arrangement(
            &root,
            &mut panel,
            &mut view::Arrangement::default(),
            &Operation::RestoreArrangement {
                name: "four_deck".to_owned(),
            },
        )
        .expect("a restore");
        assert!(
            said.contains("disagrees with itself"),
            "a file that will not read back was reported as a missing arrangement, which sends \
             an operator looking for a name they typed correctly: {said}"
        );
        unchanged(&panel, &said);

        std::fs::remove_dir_all(&root).expect("clean up");
    }

    // -- the arrangement pill's half of the family ----------------------

    /// **A name that is not one path component is refused here**, which is the
    /// authority the pill deliberately does not hold (P-0076).
    ///
    /// The negative control is the point: a check that refused everything
    /// would pass an assertion that only ever looked for a refusal, so the
    /// names that must be *accepted* are asserted beside the ones that must
    /// not (P-0025).
    #[test]
    fn a_typed_arrangement_name_is_refused_where_the_file_is_written() {
        for good in ["night", "four_deck", "set-2", "A9"] {
            assert!(
                checked_name(good).is_ok(),
                "`{good}` is letters, digits, `-` and `_`, and was refused"
            );
        }
        for (bad, why) in [
            ("", "nothing was typed"),
            ("../../elsewhere", "a path"),
            ("night deck", "a space"),
            ("night.json", "a suffix of its own"),
        ] {
            let refusal = checked_name(bad).expect_err(&format!("`{bad}` is {why} and was kept"));
            assert!(
                refusal.starts_with("arrangement: "),
                "the refusal does not say what it is about: {refusal}"
            );
            assert!(
                bad.is_empty() || refusal.contains(bad),
                "the refusal does not say the name back, so an operator cannot see what \
                 they typed: {refusal}"
            );
        }
    }

    /// **The name in use follows the file and never the press.**
    ///
    /// A save that landed and a restore that landed each make that arrangement
    /// the one in use, so the pill names it; a save that was refused leaves
    /// the pill saying what it said, because nothing under that name is on the
    /// disk. And the menu's listing gains the new name only where a file
    /// appeared.
    #[test]
    fn the_pill_names_the_arrangement_only_once_the_file_is_there() {
        let root = arrangement_root("in-use");
        let mut panel = arranged(1280.0, 720.0);
        let mut arr = view::Arrangement::NONE;

        // Refused: the name is not one path component, so nothing was filed
        // and nothing is in use.
        let said = arrangement(
            &root,
            &mut panel,
            &mut arr,
            &Operation::SaveArrangement {
                name: "night/one".to_owned(),
            },
        )
        .expect("a save is one of the operations this route answers for");
        assert!(said.contains("holds `/`"), "{said}");
        assert_eq!(arr.name, None, "a refused save put a name on the pill");
        assert!(arr.filed.is_empty());

        // Kept: in use, and listed.
        arrangement(
            &root,
            &mut panel,
            &mut arr,
            &Operation::SaveArrangement {
                name: "night".to_owned(),
            },
        )
        .expect("a save");
        assert_eq!(arr.name.as_deref(), Some("night"));
        assert_eq!(arr.filed, vec!["night".to_owned()]);

        // A restore of a name nothing is filed under is refused where the
        // bytes are, and leaves the pill alone.
        let said = arrangement(
            &root,
            &mut panel,
            &mut arr,
            &Operation::RestoreArrangement {
                name: "rehearsal".to_owned(),
            },
        )
        .expect("a restore");
        assert!(said.contains("rehearsal"), "{said}");
        assert_eq!(
            arr.name.as_deref(),
            Some("night"),
            "a refused restore moved the name the pill is showing"
        );

        // And one that is filed does put it in use.
        arrangement(
            &root,
            &mut panel,
            &mut arr,
            &Operation::RestoreArrangement {
                name: "night".to_owned(),
            },
        )
        .expect("a restore");
        assert_eq!(arr.name.as_deref(), Some("night"));

        std::fs::remove_dir_all(&root).expect("clean up");
    }

    /// **A reset takes the name off the pill, whichever surface asked.**
    ///
    /// `r` and the menu's *start a new one* are one operation and `Readout::op`
    /// is where both arrive, so this is asserted through the method rather than
    /// through either control: the default arrangement is what is on screen and
    /// the default has no name.
    #[test]
    fn a_reset_leaves_the_pill_naming_no_file() {
        let mut readout = Readout::new(1280.0, 720.0);
        readout.view.arrangement.name = Some("night".to_owned());

        // An operation that is not a reset leaves it alone, which is what says
        // the clearing is the reset's and not every operation's.
        let staging = readout.panel.layout().find("staging").expect("staging");
        readout.op(Op::Fold(staging));
        assert_eq!(readout.view.arrangement.name.as_deref(), Some("night"));

        assert_eq!(readout.op(Op::Reset), Outcome::Reset);
        assert_eq!(
            readout.view.arrangement.name, None,
            "the console was reset to the default and the pill still names a file"
        );
    }

    /// **What the menu's five asks do to this program**, and that every one of
    /// them that acts shuts the card.
    #[test]
    fn every_ask_the_pill_makes_is_acted_on_and_shuts_the_menu() {
        let mut readout = Readout::new(1280.0, 720.0);
        readout.view.arrangement.filed = vec!["night".to_owned()];

        assert!(matches!(readout.arranged(Ask::Open), Acted::Nothing));
        assert!(readout.view.arrangement.open());
        assert!(matches!(readout.arranged(Ask::Shut), Acted::Nothing));
        assert!(!readout.view.arrangement.open());

        readout.arranged(Ask::Open);
        assert!(matches!(readout.arranged(Ask::Name), Acted::Nothing));
        assert_eq!(
            readout.view.arrangement.naming(),
            Some(""),
            "the one item that asks for letters left nothing asking for any"
        );

        readout.arranged(Ask::Open);
        let did = readout.arranged(Ask::Panel(Op::Reset));
        assert!(
            matches!(did, Acted::Operated(Outcome::Reset)),
            "the reset was not performed: {did:?}"
        );
        assert!(
            !readout.view.arrangement.open(),
            "the card is still standing"
        );

        readout.arranged(Ask::Open);
        let want = Operation::RestoreArrangement {
            name: "night".to_owned(),
        };
        let did = readout.arranged(Ask::Operation(want.clone()));
        assert_eq!(
            did,
            Acted::Emitted(Some(want)),
            "the operation did not go down the path every other emitted operation takes"
        );
        assert!(!readout.view.arrangement.open());
    }

    /// **A finished name is one operation of the vocabulary**, and the card is
    /// gone before it is emitted — whether or not the name is any good, since
    /// the refusal is said out loud by `checked_name` and a card left standing
    /// over it would be the panel asking again without saying the answer.
    #[test]
    fn a_finished_name_is_the_save_the_menu_would_have_asked_for() {
        let mut readout = Readout::new(1280.0, 720.0);
        assert_eq!(
            readout.named(),
            Acted::Nothing,
            "a console with nothing being typed committed a name"
        );

        readout.arranged(Ask::Name);
        for c in "four_deck".chars() {
            assert!(readout.view.arrangement.typed(c));
        }
        assert_eq!(
            readout.named(),
            Acted::Emitted(Some(Operation::SaveArrangement {
                name: "four_deck".to_owned()
            }))
        );
        assert!(!readout.view.arrangement.open());

        // An empty name is emitted as one and refused where the file is
        // written, rather than being swallowed here.
        readout.arranged(Ask::Name);
        assert_eq!(
            readout.named(),
            Acted::Emitted(Some(Operation::SaveArrangement {
                name: String::new()
            }))
        );
    }

    /// **The menu's list is the store's, and a store that is not there is
    /// listed as nothing and is not created** — `library`'s two rules over the
    /// fourth directory.
    #[test]
    fn the_menu_lists_the_store_and_makes_none() {
        let root = arrangement_root("listing");
        assert!(
            arrangements(&root).is_empty(),
            "a store that is not there listed something"
        );
        assert!(
            !root.exists(),
            "listing the arrangements created a store at {}",
            root.display()
        );

        let mut panel = arranged(1280.0, 720.0);
        for name in ["rehearsal", "four_deck"] {
            keep_arrangement(&root, &panel, name);
        }
        panel.solve();
        assert_eq!(
            arrangements(&root),
            vec!["four_deck".to_owned(), "rehearsal".to_owned()],
            "the menu lists what the store holds, in the order the store sorts it"
        );

        std::fs::remove_dir_all(&root).expect("clean up");
    }

    /// Nothing that comes back from `get_current_texture` is dropped without a
    /// decision. The loop waits for events, so an outcome that neither
    /// reconfigures nor asks for another frame is a window that never draws
    /// again and says nothing about it.
    #[test]
    fn every_frame_that_could_not_be_acquired_is_acted_on() {
        use wgpu::CurrentSurfaceTexture as Acquired;
        // The two that mean the swapchain is stale: reconfigure, and ask again.
        assert_eq!(missed(&Acquired::Outdated), Some(Missed::Remake));
        assert_eq!(missed(&Acquired::Lost), Some(Missed::Remake));
        // Jitter: ask again, without reconfiguring.
        assert_eq!(missed(&Acquired::Timeout), Some(Missed::Again));
        // A window nobody can see: asking again is a spin, and the OS says
        // when it is back.
        assert_eq!(missed(&Acquired::Occluded), Some(Missed::Idle));
        // Not self-correcting, so it is said rather than retried.
        assert_eq!(missed(&Acquired::Validation), Some(Missed::Fault));
    }

    /// **A figure quoted in prose is held against the run that was just
    /// taken**, so a panel that grows says so instead of leaving a sentence
    /// that was true of a smaller one.
    ///
    /// This is the failure the guard exists for, and it is not hypothetical:
    /// the line above the reading cited ADR-0164's 184 allocations and said
    /// the `egui` pass "is still that" through the mixer bay landing at 456
    /// and the parked deck at 525 — two commits of a present-tense claim
    /// nobody re-checked, because nothing re-checked it.
    ///
    /// **What can be asserted here is the verdict, not the reading.** A
    /// reading needs a device, a window and three seconds of nobody touching
    /// it, so it cannot be taken from `cargo test`; what this file can do is
    /// make the figure in the sentence and the figure under the verdict one
    /// constant, and hold [`drifted`] to catching what actually went wrong.
    #[test]
    fn a_reading_that_has_moved_says_the_sentence_quoting_it_is_stale() {
        // The band a run has to stay inside to say nothing. The nine runs
        // behind the figure of 2026-08-31 agreed to the allocation; the nine
        // behind 2026-08-26's read between 524 and 538, which is the widest
        // run-to-run spread this file has ever taken, and a guard that fired
        // on 14 allocations is one nobody could keep passing.
        assert_eq!(drifted(WRITTEN_ALLOCS, WRITTEN_ALLOCS), None);
        assert_eq!(
            drifted(WRITTEN_ALLOCS + 14, WRITTEN_ALLOCS),
            None,
            "the run-to-run spread of the reading this quotes must not read as staleness"
        );

        // And what it is for: ADR-0164's 184 against the mixer bay's 456 is
        // 2.5x, so the first run after that bay landed would have said the
        // sentence had stopped being true. It is the same answer whichever of
        // the two is the one written down, because a pass that got cheaper
        // makes the sentence just as untrue.
        assert!(
            drifted(456, 184).is_some(),
            "the mixer bay's landing is the drift this exists to have caught"
        );
        assert!(
            drifted(184, 456).is_some(),
            "drift is not caught one way round only"
        );

        // A band of two is still a band: an order of magnitude is well out of
        // it, from either end.
        assert!(drifted(WRITTEN_ALLOCS * 10, WRITTEN_ALLOCS).is_some());
        assert!(drifted(WRITTEN_ALLOCS / 10, WRITTEN_ALLOCS).is_some());
    }

    /// **A frame nobody asked for is the one the reading is about.**
    ///
    /// The measurement carries the claim now, so what it counts has to be
    /// asserted rather than eyeballed on stdout. The failure it exists for is
    /// the one that made the first run of this read `3 frames` on a window
    /// that had behaved perfectly: an event resets the stretch, and the frame
    /// that event asked for lands a millisecond into the new one and gets
    /// blamed on the panel. The other direction is worse and is asserted too —
    /// a `push` that never counts anything reads `0 frames` whatever the
    /// window is doing, which is a measurement that cannot fail.
    #[test]
    fn a_frame_nobody_asked_for_is_the_one_counted_against_a_still_panel() {
        let frame = Cost {
            allocs: 7,
            bytes: 70,
            ..Default::default()
        };
        let mut costs = Costs::new();

        // A frame something asked for is that something's.
        costs.owes();
        costs.push(frame);
        assert_eq!(costs.still, Still::default(), "an owed frame was counted");

        // A frame nobody asked for — `egui`'s own deadline, on an untouched
        // window — is per-frame work on a still panel, which is the number.
        costs.push(frame);
        assert_eq!(
            costs.still,
            Still {
                frames: 1,
                allocs: 7,
                bytes: 70
            },
            "a frame nobody asked for was not counted"
        );

        // Both frames happened, whoever they belonged to.
        assert_eq!(costs.drawn, 2);

        // And touching the window starts the stretch again, so what was drawn
        // during the last one stops being about a still panel.
        costs.touched();
        assert_eq!(costs.still, Still::default());
    }

    /// **A whole drag, through the window loop's own routing.**
    ///
    /// `karakuri_console::input`'s tests are about the rule; this is about
    /// this file obeying it, which is a different claim and the one that
    /// actually reaches an operator. It drives the gesture a hand makes —
    /// press on the boundary between the left pane and the centre, run the
    /// pointer well past it and across two bays, let go — through
    /// `Readout::pointer`, which is the method `window_event` calls, and
    /// asserts two things: **the boundary moved**, so the drag works with a
    /// toolkit in the loop, and **`egui` was never told about any of it**, so
    /// the two never both think they are dragging.
    ///
    /// It cannot be a real pointer: synthesising one takes an Accessibility
    /// grant this process does not have, and a test that needs a human to
    /// click is not a test.
    #[test]
    fn a_drag_through_the_window_loops_own_routing_never_reaches_egui() {
        let ctx = drawn_once();
        let mut readout = Readout::new(1440.0, 900.0);
        readout.panel.solve();
        let layout = readout.panel.layout();
        let centre = layout.rect(layout.find("centre").expect("centre"));
        let left = layout.rect(layout.find("left-pane").expect("left-pane"));
        let was = left.w;
        // The gap between the left pane and the centre, at half height.
        let start = Point::new((left.x + left.w + centre.x) * 0.5, left.y + left.h * 0.5);

        // Approaching it is egui's until the pointer is on it.
        assert_eq!(
            readout
                .pointer(&ctx, Pointer::Moved(Point::new(start.x - 60.0, start.y)))
                .0,
            Claim::Egui
        );
        assert_eq!(readout.pointer(&ctx, Pointer::Moved(start)).0, Claim::Panel);
        assert_eq!(readout.pointer(&ctx, Pointer::Down).0, Claim::Panel);

        // A hand does not stay on the boundary: it runs on across the panel,
        // and every one of these is inside a bay.
        for x in [start.x + 40.0, start.x + 120.0, start.x + 200.0] {
            assert_eq!(
                readout
                    .pointer(&ctx, Pointer::Moved(Point::new(x, start.y)))
                    .0,
                Claim::Panel,
                "the drag lost its claim at x = {x}"
            );
            // A wheel in the middle of a drag is the panel's too.
            assert_eq!(readout.pointer(&ctx, Pointer::Wheel).0, Claim::Panel);
        }
        readout.panel.solve();
        let wide = pane_width(&readout);
        assert!(
            wide > was + 100.0,
            "the boundary did not move: the left pane went from {was} to {wide}"
        );

        // Now back the other way, past what the left pane will go below, and
        // let go there. **The pointer ends nowhere near the boundary**, which
        // is the ordinary end of a drag and the case that catches a release
        // routed after `released` rather than before it.
        let far = Point::new(start.x - 200.0, start.y);
        assert_eq!(readout.pointer(&ctx, Pointer::Moved(far)).0, Claim::Panel);
        assert_eq!(readout.pointer(&ctx, Pointer::Up).0, Claim::Panel);

        readout.panel.solve();
        assert!(
            (pane_width(&readout) - 160.0).abs() < 0.01,
            "the left pane's stated minimum did not hold the drag: {}",
            pane_width(&readout)
        );

        // And afterwards the pointer, where it is standing, is egui's again.
        assert_eq!(readout.pointer(&ctx, Pointer::Moved(far)).0, Claim::Egui);
    }

    /// The left pane's width, solved. A helper because the test asks three
    /// times and the chain is four calls long.
    fn pane_width(readout: &Readout) -> f32 {
        let layout = readout.panel.layout();
        layout.rect(layout.find("left-pane").expect("left-pane")).w
    }

    /// **A context that has drawn once**, which is what routing a pointer
    /// takes: the claim rule asks where the Outputs row's control is, that is
    /// the width of the type in it, and `egui`'s fonts are not valid until a
    /// pass has run. The window loop has drawn long before a hand arrives;
    /// a test has to say so.
    ///
    /// The texture delta is cleared because `epaint` panics if one is dropped
    /// unapplied — there is no renderer here to apply it to, which is the
    /// whole of what makes this a test and not a window.
    ///
    /// `mod gpu` uses it too: a strip is laid out with the type in it, and a
    /// device does not make fonts valid.
    pub(super) fn drawn_once() -> egui::Context {
        let ctx = egui::Context::default();
        let mut out = ctx.run_ui(egui::RawInput::default(), |_| {});
        out.textures_delta.clear();
        ctx
    }

    /// **The whole of what the four class pills are for: an operation the gate
    /// refuses becomes one it allows, because a hand pressed a capsule.**
    ///
    /// # Why it is here and can be nowhere else
    ///
    /// It crosses three crates and no two of them can see the third.
    /// `karakuri-console` draws the pill and hands back a value; it must not
    /// name `karakuri-environment` at all (ADR-0156), so it cannot reach the
    /// handle. `karakuri-environment` holds the `Opening` and cannot see a
    /// console. `karakuri-operation`'s gate holds the audit and the refusal and
    /// depends on neither. **This file is the only place all three are in
    /// scope**, which is the same reason `key_column` is a unit test in this
    /// binary: a surface is where the buck stops, nothing may depend on this
    /// package, and the checks that need everything at once live in it.
    ///
    /// # What it asserts, in the order an operator's afternoon goes
    ///
    /// 1. `SetGain` is in the mix-fader class, which is the classification
    ///    ADR-0235 drew — asserted against `standing` rather than assumed, so
    ///    that a row moved out of the class fails here rather than making this
    ///    test quietly vacuous.
    /// 2. On a run nobody has touched it is **refused**, and the sentence is
    ///    `gate::refusal`'s own **by equality** — P-0061, *a refusal a person
    ///    can reach from two surfaces is one sentence*, asserted against the
    ///    function rather than with a `contains`. It names the Mixer bay,
    ///    because a model that is told only *no* reports the instrument as
    ///    incapable instead of as closed.
    /// 3. A press on the Mixer bay's pill — through `Readout::pointer`, which
    ///    is the same routing a hand goes through, and not by calling `set`
    ///    here — opens the class.
    /// 4. **The same call, the same audit, now allowed.** Nothing about the
    ///    operation changed and nothing about the vocabulary changed; the list
    ///    a model reads never shortened at any point.
    /// 5. **And exactly that class.** The other three are still shut and an
    ///    operation in one of them is still refused, which is the property the
    ///    console's own `a_press_opens_exactly_one_class_and_leaves_the_other_three_shut`
    ///    makes about the value and this one makes about the run.
    /// 6. A second press shuts it, and the call is refused again — the other
    ///    half of the page's *"click again to shut it"*, seen from the gate.
    #[test]
    fn the_gate_lets_a_refused_operation_through_once_the_class_is_open() {
        use karakuri_operation::gate::{audit, refusal, standing, Running, Standing};

        let ctx = drawn_once();
        let mut readout = Readout::new(1440.0, 900.0);
        readout.panel.solve();

        // A write to a mix fader: unpriced, immediate, irreversible, and what
        // the audience is looking at — P-0079's three answers, all missing.
        let write = Operation::SetGain { deck: 0, gain: 0.5 };
        assert_eq!(
            standing(&write, Running::unread()),
            Standing::Closed(Class::MixFaders),
            "`SetGain` is no longer in the class this test is about"
        );
        // And one from another class, to hold the press to one class below.
        let elsewhere = Operation::RecordSession {
            recording: karakuri_operation::Recording::Stop,
        };
        assert_eq!(
            standing(&elsewhere, Running::unread()),
            Standing::Closed(Class::InputsAndOutputs)
        );

        // 2. Refused, in one sentence, and it says where a hand opens it.
        let refused = audit(&write, readout.opening.read(), Running::unread())
            .expect_err("a mix write is allowed on a run nobody has opened anything on");
        assert_eq!(
            refused,
            refusal(&write, Standing::Closed(Class::MixFaders)).expect("a refusal has a sentence")
        );
        assert!(
            refused.contains("the head of the Mixer bay"),
            "the refusal does not say where the pill is: {refused}"
        );

        // 3. The press. Where the capsule is comes from the same derivation
        // that painted it, and the event goes through the window loop's own
        // routing — `Opening::set` is never called from this test.
        let capsule = |readout: &mut Readout| {
            readout.panel.solve();
            let pill = mcp_pill(
                &ctx,
                readout.panel.layout(),
                Class::MixFaders,
                readout.view.opening,
            )
            .expect("the Mixer bay draws its class pill");
            (
                Point::new(pill.pill.center().x, pill.pill.center().y),
                pill.open,
            )
        };
        let (at, open) = capsule(&mut readout);
        assert!(!open, "the pill reads open on a run that has just started");
        assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
        let (claim, did) = readout.pointer(&ctx, Pointer::Down);
        assert_eq!(claim, Claim::Panel);
        assert_eq!(
            did,
            Acted::Opened,
            "the class pill went down one of the other two paths — an `Operation` \
             or an operation on the arrangement — and ADR-0236 says it is neither"
        );
        readout.pointer(&ctx, Pointer::Up);

        // 4. The same call, the same audit, allowed.
        let allowed = audit(&write, readout.opening.read(), Running::unread())
            .expect("the operator opened the class and the call is still refused");
        assert_eq!(allowed.operation(), &write);

        // 5. And exactly that class.
        for class in Class::ALL {
            assert_eq!(
                readout.opening.read().holds(*class),
                *class == Class::MixFaders,
                "one press opened or shut {class:?} as well"
            );
        }
        assert_eq!(
            audit(&elsewhere, readout.opening.read(), Running::unread())
                .expect_err("opening the mix faders opened the outputs too"),
            refusal(&elsewhere, Standing::Closed(Class::InputsAndOutputs)).expect("a sentence")
        );

        // 6. And a second press shuts it again.
        let (at, open) = capsule(&mut readout);
        assert!(
            open,
            "the pill did not read open after the press that opened it"
        );
        readout.pointer(&ctx, Pointer::Moved(at));
        assert_eq!(readout.pointer(&ctx, Pointer::Down).1, Acted::Opened);
        assert_eq!(readout.opening.read(), Open::CLOSED);
        assert_eq!(
            audit(&write, readout.opening.read(), Running::unread())
                .expect_err("the class was shut again and the call still goes through"),
            refused
        );
    }

    /// **All four pills are reachable through the window loop's routing**, not
    /// the Mixer's alone — three of them are in a bay head and the fourth is in
    /// a row that has none, and the one this file could most easily have got
    /// wrong is the one with no head to hang it in.
    #[test]
    fn each_of_the_four_pills_opens_its_own_class_through_a_press() {
        let ctx = drawn_once();
        for class in Class::ALL {
            let mut readout = Readout::new(1440.0, 900.0);
            readout.panel.solve();
            let pill = mcp_pill(&ctx, readout.panel.layout(), *class, readout.view.opening)
                .unwrap_or_else(|| panic!("{class:?} draws no pill"));
            let at = Point::new(pill.pill.center().x, pill.pill.center().y);

            assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
            assert_eq!(readout.pointer(&ctx, Pointer::Down).1, Acted::Opened);
            assert_eq!(
                readout.opening.read(),
                Open::CLOSED.with(*class, true),
                "a press on {class:?}'s pill did not open exactly it"
            );
            // **The view is written in the same breath as the handle**, or the
            // very next press is aimed at the capsule that used to be there:
            // the two words are not the same width.
            assert_eq!(readout.view.opening, readout.opening.read());
        }
    }

    /// **A press on the outputs dot, through the window loop's own routing.**
    ///
    /// The other half of the test above: that one is a boundary the panel
    /// claims and `egui` never sees, and this is the console's one control,
    /// which the panel claims for a different reason — `egui` owns no widget
    /// anywhere here, so a press routed to it would reach nothing at all.
    ///
    /// What is asserted is the round trip an operator makes: the picture is on
    /// screen, a click on the dot folds it away by name, and a click on the
    /// same dot brings it back. The dot is where it is drawn and the press is
    /// the panel's at every step.
    #[test]
    fn a_press_on_the_outputs_dot_folds_the_picture_and_unfolds_it() {
        let ctx = drawn_once();
        let mut readout = Readout::new(1440.0, 900.0);
        readout.panel.solve();
        let picture = readout
            .panel
            .layout()
            .find("program-view")
            .expect("program-view");
        let dot = |readout: &mut Readout| {
            readout.panel.solve();
            let row = outputs(&ctx, readout.panel.layout(), Open::CLOSED)
                .expect("the row draws its sink");
            (Point::new(row.sink.center().x, row.sink.center().y), row.on)
        };

        let (at, on) = dot(&mut readout);
        assert!(on, "the picture is on screen, so the sink is on");

        // The pointer arrives, and the control is the panel's.
        assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
        let (claim, did) = readout.pointer(&ctx, Pointer::Down);
        assert_eq!(claim, Claim::Panel);
        assert_eq!(
            did,
            Acted::Operated(Outcome::Folded {
                id: picture,
                folded: true,
                root: false
            }),
            "the press did not reach the sink"
        );
        assert!(
            !readout.panel.dragging(),
            "the press took a boundary in hand"
        );
        readout.pointer(&ctx, Pointer::Up);

        // And the dot is dark, where it still is, and turns the picture back
        // on rather than unfolding whatever else is folded.
        let (at, on) = dot(&mut readout);
        assert!(!on, "the picture is folded and the sink is still lit");
        assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
        assert_eq!(
            readout.pointer(&ctx, Pointer::Down).1,
            Acted::Operated(Outcome::Folded {
                id: picture,
                folded: false,
                root: false
            }),
            "the dark dot did not turn the picture back on"
        );
        assert!(
            dot(&mut readout).1,
            "the picture is back and the dot is dark"
        );
    }

    /// **A press on a scope chip, through the window loop's own routing** —
    /// which is what ADR-0213 makes the *panel* badge mean.
    ///
    /// `karakuri-console`'s `tests/library.rs` asserts everything up to the
    /// operation with no window anywhere: that the chips answer a press, that
    /// a press names the chip it landed on, and that nothing else in the bay
    /// takes one. **This is the half that badge is actually about** — *"the
    /// row is claimed the day a person who launched the instrument can perform
    /// that operation from the panel in front of them"* — and a control
    /// demonstrated in that crate and never wired here would pass there and be
    /// a lie this page tells on its own authority.
    ///
    /// **What is asserted is the whole press and not the routing alone**: the
    /// mark moves to the chip that was pressed, the operation that leaves is
    /// `SelectScope` with the payload it is specified to carry, and the
    /// library cursor goes back to the top — because the listing under a new
    /// scope is a listing this cursor has never seen, and a cursor left where
    /// it was would sit on a Set nobody chose under a pill saying a press will
    /// load it.
    ///
    /// **And the chip that is already marked is pressed too**, because that is
    /// the case a step cannot reach: `e` would go somewhere else, and a
    /// pointer names — so the press is answered rather than refused, and the
    /// mark stays where it is.
    #[test]
    fn a_press_on_a_scope_chip_names_the_library_the_bay_reads() {
        let ctx = drawn_once();
        let mut readout = Readout::new(1440.0, 900.0);
        readout.panel.solve();
        // A console that has been told what libraries there are and handed a
        // listing for the one it opens on, which is what `resumed` does.
        readout.view.scopes = Scope::ALL.to_vec();
        readout.view.library = vec!["drift_night".to_owned(), "lattice_veil".to_owned()];
        assert!(readout.view.select_scope(Scope::MySets));

        // The capsule, asked of the derivation that draws it rather than
        // remembered — the rule the whole of `input` is written to.
        let chip = |readout: &mut Readout, want: Scope| {
            readout.panel.solve();
            let bay = library_bay(
                readout.panel.layout(),
                &readout.view.scopes,
                &readout.view.library,
            )
            .expect("the bay lists its rows");
            let (_, at) = bay
                .chips(&ctx, &readout.view.scopes)
                .find(|(scope, _)| *scope == want)
                .expect("the scope is on the row");
            // Two pixels in from its own left edge: the last chip in the row
            // is clipped by the pane, so its centre can be off the row.
            Point::new(at.min.x + 2.0, at.center().y)
        };

        // **The cursor is somewhere other than the top**, so that the move
        // back to it is a move and not the state it was already in.
        assert!(readout.view.walk(1, 2), "the cursor did not move");
        assert_eq!(readout.view.cursor_row(), 1);

        let at = chip(&mut readout, Scope::Presets);
        assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
        let (claim, did) = readout.pointer(&ctx, Pointer::Down);
        assert_eq!(claim, Claim::Panel);
        assert_eq!(
            did,
            Acted::Emitted(Some(Operation::SelectScope { scope: Undecided })),
            "the press did not reach the chip"
        );
        assert!(
            !readout.panel.dragging(),
            "the press took a boundary in hand"
        );
        readout.pointer(&ctx, Pointer::Up);
        assert_eq!(
            readout.view.scope(),
            Some(Scope::Presets),
            "the press was routed and the mark stayed where it was"
        );
        assert_eq!(
            readout.view.cursor_row(),
            0,
            "the scope changed and the cursor is still pointing into the listing it left"
        );

        // **The marked chip, pressed** — answered rather than refused, and the
        // mark does not step off it the way the key would.
        let at = chip(&mut readout, Scope::Presets);
        assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
        assert_eq!(
            readout.pointer(&ctx, Pointer::Down).1,
            Acted::Emitted(Some(Operation::SelectScope { scope: Undecided })),
            "a press on the chip that is already marked was not answered"
        );
        readout.pointer(&ctx, Pointer::Up);
        assert_eq!(
            readout.view.scope(),
            Some(Scope::Presets),
            "a press on the marked chip stepped somewhere"
        );
    }

    /// **A press on the Program bay's `solo` pill, through the window loop's
    /// own routing.**
    ///
    /// `karakuri-console`'s `tests/solo_pill.rs` and `tests/vocabulary.rs`
    /// assert everything up to the operation with no window anywhere; this is
    /// the half ADR-0213 makes the badge mean — *"the row is claimed the day a
    /// person who launched the instrument can perform that operation from the
    /// panel in front of them"* — and a control demonstrated in that crate and
    /// never wired here would pass there and be a lie the page tells.
    ///
    /// **Both directions, because the pill is both.** A solo takes every other
    /// control off the screen, so the pill is the only thing left to press and
    /// the undo has to come from it. What is asserted is the round trip an
    /// operator makes: the picture is one region among many, a click on the
    /// pill leaves it holding the window, and a click on the same pill —
    /// **found again where it is now drawn**, because the solo moved every
    /// rectangle on the console — puts everything back.
    #[test]
    fn a_press_on_the_solo_pill_solos_the_picture_and_undoes_it() {
        let ctx = drawn_once();
        let mut readout = Readout::new(1440.0, 900.0);
        readout.panel.solve();
        let picture = readout
            .panel
            .layout()
            .find("program-view")
            .expect("program-view");
        let library = readout.panel.layout().find("library").expect("library");
        // The capsule, asked of the derivation that draws it rather than
        // remembered — which is the rule the whole of `input` is written to,
        // and here it is load-bearing twice over.
        let pill = |readout: &mut Readout| {
            readout.panel.solve();
            let head = program_head(&ctx, readout.panel.layout(), Open::CLOSED)
                .expect("the bay draws its pill");
            (
                Point::new(head.solo.center().x, head.solo.center().y),
                head.soloed,
            )
        };

        let (at, soloed) = pill(&mut readout);
        assert!(!soloed, "something is soloed before anything was pressed");
        assert!(
            readout.panel.layout().visible(library),
            "the library is off the screen already, so soloing would prove nothing"
        );

        assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
        let (claim, did) = readout.pointer(&ctx, Pointer::Down);
        assert_eq!(claim, Claim::Panel);
        assert_eq!(
            did,
            Acted::Operated(Outcome::Soloed(picture)),
            "the press did not reach the pill"
        );
        assert!(
            !readout.panel.dragging(),
            "the press took a boundary in hand"
        );
        readout.pointer(&ctx, Pointer::Up);
        readout.panel.solve();
        assert!(
            !readout.panel.layout().visible(library),
            "the picture is soloed and the library is still on the screen"
        );

        // And the same pill, where it is now, undoes it.
        let (at, soloed) = pill(&mut readout);
        assert!(soloed, "the picture is soloed and the pill does not say so");
        assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
        assert_eq!(
            readout.pointer(&ctx, Pointer::Down).1,
            Acted::Operated(Outcome::Unsoloed { was: true }),
            "the pill did not undo the solo it made"
        );
        readout.pointer(&ctx, Pointer::Up);
        readout.panel.solve();
        assert!(
            readout.panel.layout().visible(library),
            "undoing the solo left the library folded"
        );
    }
    /// **A control's operation becomes the record every other surface's
    /// control ends in**, and this is the half of that which needs no device.
    ///
    /// **This test is older than the conversion it now checks, and that is the
    /// point of it.** It was written against the hand-written `record` this
    /// file used to carry, asserting term for term what `karakuri-cli`'s
    /// `mix::gain_record` and `mix::opacity_record` already wrote. That
    /// function is deleted and [`written`] answers instead (ADR-0185's promise,
    /// kept where ADR-0194 put the home) — **every expectation below is
    /// unchanged**, so if the crate's conversion disagreed with the one that
    /// was deleted, this is what says so.
    ///
    /// **`mix::gain_record` is deleted too**, by the same record and for the
    /// stronger reason: the conversion *is* the derivation now, and two of
    /// them is the drift `mix.rs` exists to end. The comments below name it
    /// where it stood, because what this test compares against is the record
    /// that function wrote rather than the function.
    ///
    /// And the other direction: an operation this program has no control for
    /// writes no record here either, and the answer says *which* kind of
    /// nothing rather than a bare `None` — which is the whole of what the
    /// three answers buy.
    #[test]
    fn a_controls_operation_becomes_the_record_the_cli_would_have_written() {
        assert_eq!(
            only_record(&Operation::SetGain {
                deck: 2,
                gain: 0.75
            }),
            // What `mix::gain_record(2, 0.75)` wrote, before ADR-0194 deleted
            // it in favour of this conversion.
            Record::Gain {
                slot: 2,
                value: 0.75
            }
        );
        assert_eq!(
            only_record(&Operation::SetOpacity {
                deck: 0,
                opacity: 0.25
            }),
            // `mix::opacity_record(0, 0.25)`.
            Record::Opacity {
                slot: 0,
                value: 0.25
            }
        );
        // **Every mode of the cycle, because a chip that emits three
        // operations has three records to write** — and the mode is a wire
        // name, so a mode that reached `Record::Blend` misspelled would be
        // refused by the engine on the way back rather than here.
        for (deck, blend) in BlendMode::ALL.into_iter().enumerate() {
            let deck = deck as u8;
            assert_eq!(
                only_record(&Operation::SetBlendMode { deck, blend }),
                // `mix::blend_record(deck, blend)`.
                Record::Blend {
                    slot: deck,
                    mode: blend.name().to_owned(),
                },
                "`{}` did not become the record `mix::blend_record` writes",
                blend.name()
            );
            // And the engine reads its own name back, which is what says the
            // two lists are the same three words rather than two spellings of
            // them.
            assert_eq!(
                Blend::from_name(blend.name()),
                Some(blend_mode_back(blend)),
                "the engine does not know the vocabulary's `{}`",
                blend.name()
            );
        }

        // **Every residency of the cycle**, for the same reason as the blend:
        // one chip emitting three operations has three records to write. The
        // spelling is the wire's — `mix::residency_wire_name`'s three words,
        // which are deliberately not the status line's `LIVE`/`prim`/`park`
        // and not the chip's `live`/`prim`/`alloc` either, so a record written
        // in the chip's vocabulary would decode as nothing at all.
        for (deck, (residency, level)) in [
            (karakuri_operation::Residency::Live, "live"),
            (karakuri_operation::Residency::Priming, "priming"),
            (karakuri_operation::Residency::Allocated, "allocated"),
        ]
        .into_iter()
        .enumerate()
        {
            let deck = deck as u8;
            assert_eq!(
                only_record(&Operation::SetResidency { deck, residency }),
                // `mix::residency_record(deck, residency)`.
                Record::Residency {
                    slot: deck,
                    level: level.to_owned(),
                },
                "{residency:?} did not become the record `mix::residency_record` writes"
            );
            // And it reads back as the level it named, which is what says the
            // two spellings are one list rather than two.
            assert_eq!(
                mix::parse_residency(level),
                Some(residency_back(residency)),
                "the wire spelling `{level}` does not come back as {residency:?}"
            );
        }

        // The vocabulary is larger than what this program reaches: five controls
        // writing five records. A record invented for the other 45 would be
        // somebody deciding what they mean — and the answer is now *which*
        // nothing rather than `None`, because a surface's own state and a
        // record nobody can write yet are not the same silence.
        assert_eq!(
            written(&Operation::Solo { region: None }, &Current::default()),
            Written::Silent(Silent::Surface)
        );
        assert_eq!(
            written(&Operation::SelectDeck { deck: 1 }, &Current::default()),
            Written::Silent(Silent::Surface)
        );
    }

    /// **The one record an operation writes**, for the tests that know there
    /// is exactly one.
    ///
    /// **For the four whose record needs no reading at all**, which is where
    /// `Current::default()` — *I read nothing* — is the honest answer. A
    /// conversion that answered anything but a single record for one of those
    /// four is this file's assumption breaking rather than a test needing a
    /// helper, which is why the panic says so.
    ///
    /// **The mask's operation is not one of them** and must not be passed
    /// here: its record is written out of the operation *and* a reading of the
    /// running mask (ADR-0201), so it would come back `Owed(NotRead)` and this
    /// would panic — correctly, and saying which operation. What the mask's
    /// tests hand in is a reading, through [`reading`] where there is a deck
    /// and by hand where there is not.
    pub(super) fn only_record(operation: &Operation) -> Record {
        match written(operation, &Current::default()) {
            Written::Records(records) if records.len() == 1 => records.into_iter().next().unwrap(),
            other => panic!(
                "a control's operation did not write exactly one record: \
                 {operation:?} -> {other:?}"
            ),
        }
    }

    /// **An operation whose record nobody can write yet does not silently do
    /// nothing**, and it is not the same event as one that writes no record on
    /// purpose.
    ///
    /// This is what the third answer is *for*, and the cheap harness is the
    /// one that treats *not `Records`* as a no-op. A press that emitted
    /// `Wipe` would then look exactly like a press that emitted
    /// `SelectDeck` — nothing printed and nothing moved — and an operator
    /// would read the first as *the wipe did not take* when what happened is
    /// *nobody has decided what a wipe writes* (`Owed` is a question, not an
    /// error: ADR-0194).
    ///
    /// **The operation this names used to be `FadeDeck` and had to change**,
    /// which is the test doing what it says on the line below: a fade stopped
    /// being owed the day the transition settings became a reading, so the
    /// operation named here is now `Operation::Wipe` — the one whose record is
    /// still nobody's to write, because it carries a shape
    /// `Operation::SetTransition` holds and a soft edge no operation names.
    /// The fade has not left this test, though: it is the second half, and it
    /// is now the *other* kind of gap — a reading this window does not have,
    /// said with the name of the reading in it.
    ///
    /// **Neither sentence is asserted word for word.** What has to hold is
    /// that the window says something, that it names the operation and the
    /// reason, and that the two answers are two different sentences.
    #[test]
    fn an_operation_whose_record_is_owed_is_said_rather_than_swallowed() {
        // Owed, and `NotSettled` is the reason: a wipe carries a mask whose
        // shape is a console setting and whose soft edge no operation names,
        // so nobody has said what it writes.
        let wipe = Operation::Wipe { from: 0, to: 1 };
        let owed = written(&wipe, &Current::default());
        assert_eq!(
            owed,
            Written::Owed(Owed::NotSettled),
            "a wipe is not owed any more — this test names the operation it does, and \
             the one it names has to still be one nobody can write"
        );
        let said = unwritten(&wipe, &owed).expect(
            "a wipe owes a record and this window said nothing at all — a press whose \
             record nobody has decided how to write reads, in silence, exactly like a \
             press that did not work",
        );
        assert!(
            said.contains("Wipe") && said.contains(Owed::NotSettled.why()),
            "the window said `{said}`, which does not name both the operation and the \
             question it is waiting on"
        );

        // **And the other gap, which is this window's rather than nobody's.**
        // A fade converts now, and what it needs is the quantum and the length
        // a surface holds — which this panel does not, because it draws no
        // control that sets either. So a press that emitted one would be told
        // *which reading* was not handed over rather than getting a fade at a
        // length nobody chose, and the sentence has to be a different one from
        // the wipe's above or the two gaps read alike.
        let fade = Operation::FadeDeck { deck: 1, to: 0.0 };
        let unread = written(&fade, &Current::default());
        assert_eq!(
            unread,
            Written::Owed(Owed::NotRead(
                karakuri_operation_record::Reading::Transition
            )),
            "a fade with no transition settings handed in came back with something \
             other than the reading it is missing — a default here is a cut at beat \
             zero, which is a move nobody asked for"
        );
        let told = unwritten(&fade, &unread).expect(
            "a fade this window cannot schedule said nothing at all, so a control that \
             emitted one would read exactly like a control that did not work",
        );
        assert!(
            told.contains("FadeDeck")
                && told
                    .contains(Owed::NotRead(karakuri_operation_record::Reading::Transition).why()),
            "the window said `{told}`, which does not name both the operation and the \
             reading it did not get"
        );
        assert_ne!(
            told, said,
            "a reading this window forgot and a record nobody has decided how to write \
             read as the same sentence"
        );

        // Silent, and settled: which deck the keys are addressed to is a
        // surface's own state and there is nothing to write.
        let select = Operation::SelectDeck { deck: 1 };
        let silent = written(&select, &Current::default());
        assert_eq!(silent, Written::Silent(Silent::Surface));
        let settled = unwritten(&select, &silent).expect(
            "selecting a deck writes no record and the window said nothing about it \
             either, so a press on such a control would leave no trace at all",
        );
        assert!(
            settled.contains(Silent::Surface.why()),
            "the window said `{settled}`, which does not say why there is no record"
        );

        // **And the two are different sentences.** Collapsing them is the
        // failure this whole test is about at one remove: a harness that
        // printed one line for both would tell an operator that an undecided
        // fade is as settled as a deck selection.
        assert_ne!(
            said, settled,
            "a record nobody can write yet and a record nobody needs to write came out \
             of this window as the same sentence"
        );

        // A record's line is `apply`'s — it says the record *and* what the
        // deck holds afterwards — so this says nothing about that case.
        // Otherwise one press prints twice.
        let gain = Operation::SetGain { deck: 0, gain: 0.5 };
        assert_eq!(
            unwritten(&gain, &written(&gain, &Current::default())),
            None,
            "an operation that wrote a record was also announced as writing none"
        );
    }

    /// **The deck head's two operations, as far as this program can take them
    /// without a device** — and they go the same distance now, which is the
    /// point.
    ///
    /// They used to go different distances: a scrub became a record and a sync
    /// mode did not, and the second half of that is what
    /// `tests/panel_column.rs`'s one exemption rested on — the chip's badge
    /// stayed `plan` because an operator who pressed it reached the emission
    /// and not the move. That test said the day it stopped being true it would
    /// stop being true here, and this is here.
    ///
    /// **The two are still not the same conversion, and that is what the
    /// second half asserts.** A scrub is relative and reads the transport it
    /// moves from; a mode is absolute and reads the session tempo, replacing
    /// the anchor and clearing the scrub. A sync mode that came out carrying
    /// the position the slot was scrubbed to would be the two conversions
    /// having been made one.
    #[test]
    fn the_deck_heads_two_operations_go_different_distances() {
        // **The scrub is relative, so the record is where the slot is plus
        // what was asked for.** The reading is handed in by hand here for
        // `reading`'s reason at the mask: there is no deck in this test
        // binary, and what is being checked is the arithmetic rather than the
        // read.
        let current = Current {
            transport: Some(karakuri_operation_record::Transport {
                sync: karakuri_operation::Sync::Beat,
                anchor_bpm: 128.0,
                scrub_beats: -1.5,
            }),
            ..Current::default()
        };
        // **The amount is the console's own constant**, not a figure written
        // again here: the arrow that emits it and the record that carries it
        // are one number or the panel and the deck disagree about how far a
        // press goes.
        let scrub = Operation::ScrubDeck {
            deck: 1,
            beats: SCRUB_BEATS,
        };
        assert_eq!(
            written(&scrub, &current),
            Written::Records(vec![Record::Transport {
                slot: 1,
                sync: "beat".to_owned(),
                anchor_bpm: 128.0,
                scrub_beats: -1.25,
            }]),
            "a press of the deck head's forward arrow, from -1.50, did not come out at -1.25 — \
             so the record is not the offset the deck holds plus the amount the arrow asks for"
        );
        // **And the reading is what makes it one**: without it the conversion
        // says so rather than starting the deck's scrub from zero, which is
        // why `reading` has an arm for this operation at all.
        assert_eq!(
            written(&scrub, &Current::default()),
            Written::Owed(Owed::NotRead(karakuri_operation_record::Reading::Transport)),
            "a scrub with no transport read came back with a record, which means it invented \
             the position it moved from"
        );
        // **The mode goes out as a wire name and the engine reads its own name
        // back**, which is what `apply` does with it and is the blend chip's
        // assertion one control along.
        for sync in SYNCS {
            assert_eq!(
                EngineSync::from_name(sync.name()).map(mix::sync),
                Some(sync),
                "the engine does not know the vocabulary's `{}`",
                sync.name()
            );
        }

        // **A sync mode anchors at the session tempo and starts on the
        // grid.** The reading handed in is the same one the scrub used —
        // anchored at 128 and scrubbed to -1.5 — and none of it may survive:
        // `Transport::engaged` clears the scrub because *"a slot brought back
        // to the grid should be on the grid, not on wherever it was scrubbed
        // to a song ago"*, and the anchor is the room's tempo rather than the
        // one the slot was last locked to.
        let set = Operation::SetSync {
            deck: 1,
            sync: karakuri_operation::Sync::Beat,
        };
        let engaged = Current {
            tempo: Some(126.0),
            ..current
        };
        assert_eq!(
            written(&set, &engaged),
            Written::Records(vec![Record::Transport {
                slot: 1,
                sync: "beat".to_owned(),
                anchor_bpm: 126.0,
                scrub_beats: 0.0,
            }]),
            "a press of the deck head's sync chip, in a room at 126 bpm, did not come out \
             anchored at 126 with the scrub cleared — either the slot's old anchor survived \
             being re-engaged, or the position it was scrubbed to did"
        );
        // **And the reading is what makes it one.** Without the tempo the
        // conversion says so rather than anchoring at a guess, which is the
        // scrub's own arrangement two assertions up and the reason `reading`
        // has an arm for this operation at all.
        assert_eq!(
            written(&set, &Current::default()),
            Written::Owed(Owed::NotRead(karakuri_operation_record::Reading::Tempo)),
            "a sync mode with no session tempo read came back with a record, which means the \
             tempo it anchored the deck at was invented"
        );
        let said = unwritten(&set, &written(&set, &Current::default())).expect(
            "a sync chip press with no tempo read said nothing at all — a press that reads, in \
             silence, exactly like a press that did not work",
        );
        assert!(
            said.contains("SetSync")
                && said.contains(Owed::NotRead(karakuri_operation_record::Reading::Tempo).why()),
            "the window said `{said}`, which does not name both the operation and the reading \
             it did not get"
        );
    }

    /// **The two crates walk the sync modes in one order**, which is what
    /// makes `view::Pane::allows` line up with the field it fills.
    ///
    /// [`inspector`] builds that array by mapping `EngineSync::ALL` and the
    /// console reads it by indexing [`SYNCS`], so the two orders are one order
    /// or the panel skips the wrong mode — silently, and only on material that
    /// refuses something. Two arrays cannot be made one by a comment.
    #[test]
    fn the_two_crates_walk_the_sync_modes_in_one_order() {
        assert_eq!(EngineSync::ALL.len(), SYNCS.len());
        for (index, mode) in EngineSync::ALL.into_iter().enumerate() {
            assert_eq!(
                mix::sync(mode),
                SYNCS[index],
                "`EngineSync::ALL[{index}]` is `{}` and the console's `SYNCS[{index}]` is \
                 `{}` — the deck head's cycle would skip the wrong mode",
                mode.name(),
                SYNCS[index].name()
            );
        }
    }

    /// [`tally`] the other way round, for the assertion above alone — the
    /// vocabulary's residency as the engine's, so that the round trip through
    /// the wire name can be compared against something.
    fn residency_back(residency: karakuri_operation::Residency) -> Residency {
        match residency {
            karakuri_operation::Residency::Live => Residency::Live,
            karakuri_operation::Residency::Priming => Residency::Priming,
            karakuri_operation::Residency::Allocated => Residency::Allocated,
        }
    }

    /// [`blend_mode`] the other way round, for the assertion above alone —
    /// which is why it is here and not beside it: nothing the program *runs*
    /// needs to go this direction, and a conversion in `src` with one test as
    /// its only caller would be an abstraction with no second call site.
    fn blend_mode_back(blend: BlendMode) -> Blend {
        match blend {
            BlendMode::Add => Blend::Add,
            BlendMode::Over => Blend::Over,
            BlendMode::Max => Blend::Max,
        }
    }

    /// **Anything that makes texels this frame keeps the loop awake, and the
    /// list is closed.**
    ///
    /// [`live`] decides whether the loop asks for another frame, and it is the
    /// one decision in this file that has already been got wrong twice in the
    /// same direction. The first time it was set once and never cleared, so
    /// folding the picture away left the window drawing at full rate — found
    /// by an operator on another machine following this file's own
    /// instructions, which said the window goes quiet, and getting 270 frames.
    /// The second time it was **the picture alone**, which is the same failure
    /// with a preview under it: fold the picture and deck A goes on
    /// auditioning while the loop stops asking for frames, so the panel keeps
    /// changing and nothing draws it.
    ///
    /// So the assertion is over every sink, not over the one this program
    /// fills: a cell nobody has wired up yet is asserted live all the same,
    /// because the failure is a sink left out of the list rather than a sink
    /// that is off.
    ///
    /// It needs no device: an `egui::TextureId` is a number, and what is being
    /// asserted is a rule about `Option`s.
    #[test]
    fn anything_that_makes_texels_keeps_the_loop_awake() {
        let some = Picture {
            id: egui::TextureId::User(0),
            rect: egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(16.0, 9.0)),
        };
        let mut view = View::new(Room::Day);

        // Nothing is making texels, so the loop has no reason of its own to
        // draw and `ControlFlow::Wait` gets to block.
        assert!(!live(&view), "an empty panel was called live");

        // The picture, which is what this rule used to be the whole of.
        view.picture = Some(some);
        assert!(live(&view), "a live picture did not keep the loop awake");

        // **The case the picture-alone rule gets wrong**: the picture folded
        // away with deck A still auditioning under it.
        view.picture = None;
        view.previews[0] = Some(some);
        assert!(
            live(&view),
            "the picture is folded away and deck A is still rendering, and the loop was \
             told to sleep — which is the window that kept drawing 270 frames after it \
             was said to have gone quiet"
        );

        // And the list is closed: every cell counts, including the three this
        // program leaves off, because the bug is a sink that is not read here.
        for deck in 0..DECKS {
            let mut view = View::new(Room::Day);
            view.previews[deck] = Some(some);
            assert!(
                live(&view),
                "deck {deck} is rendering and the loop was told to sleep"
            );
        }

        // The other direction, which costs frames rather than pixels: with
        // every sink off the loop stops asking.
        view.previews[0] = None;
        assert!(
            !live(&view),
            "nothing is rendering and the loop stayed awake"
        );
    }

    /// **Two paths or none, and anything else is a refusal rather than a
    /// guess.**
    ///
    /// [`sources_from`] is the whole of this program's command line and this is
    /// what stops it growing a second one. The mistake it will actually be
    /// given is *one* path — a Set is two files and reads like one thing — and
    /// that is refused by name rather than paired with a default renderer,
    /// because a program that silently supplied half the material would draw
    /// something nobody asked for and say nothing about it.
    ///
    /// A CPU test: nothing here opens a file, and a path that does not exist is
    /// still a path. What is behind one is [`checked`]'s to complain about.
    #[test]
    fn a_set_is_two_paths_or_none_and_anything_else_is_refused() {
        let of = |args: &[&str]| sources_from(args.iter().map(|a| (*a).to_string()));

        let bare = of(&[]).expect("no arguments is the pair the preset library ships");
        assert_eq!(bare.sources.l1, shipped().l1);
        assert_eq!(bare.sources.l4, shipped().l4);
        assert!(
            bare.sources.l1.is_file() && bare.sources.l4.is_file(),
            "the default pair is not on the disk at {} and {}, so a bare run cannot draw",
            bare.sources.l1.display(),
            bare.sources.l4.display()
        );

        let named = of(&["a/geo.kir", "b/ren.kir"]).expect("two paths are a Set");
        assert_eq!(named.sources.l1, std::path::PathBuf::from("a/geo.kir"));
        assert_eq!(named.sources.l4, std::path::PathBuf::from("b/ren.kir"));
        assert_eq!(
            named.sources.material(),
            "geo + ren",
            "the strip is not named after what was actually loaded"
        );

        let one = of(&["a/geo.kir"]).expect_err(
            "one path was read as a Set, so this program would have invented the other half",
        );
        assert!(
            one.contains("a/geo.kir"),
            "the refusal `{one}` does not name the path it refused"
        );
        assert!(
            of(&["a.kir", "b.kir", "c.kir"]).is_err(),
            "three paths were read as a Set"
        );

        // **An empty message is `--help`**, which is the one arm that is a
        // request rather than a mistake — [`main`] prints [`USAGE`] to stdout
        // and exits 0 on it, and prints it to stderr and exits 2 on every
        // other. A refusal that came back empty would be a silent exit.
        assert_eq!(of(&["--help"]).err(), Some(String::new()));
        assert_eq!(of(&["-h"]).err(), Some(String::new()));
        assert!(
            !one.is_empty(),
            "a refusal came back with no sentence in it, which `main` reads as `--help`"
        );
    }

    /// **`--mcp` takes a port, and it is refused in the three ways a flag with a
    /// value is refused.**
    ///
    /// The first two are [`value_for`]'s and are the two the other flags already
    /// meet — a flag at the end of the line does not fall back to a default, and
    /// a flag whose value is the next flag does not eat it. The third is
    /// [`number_for`]'s and is new here, because this is the first flag on this
    /// command line that takes a number: a port that is not a port is a mistake
    /// on the command line, and a run that started serving on some other number
    /// would be the wrong kind of helpful.
    #[test]
    fn the_mcp_flag_takes_a_port_and_is_refused_the_three_ways_a_valued_flag_is() {
        let read =
            |args: &[&str]| sources_from(args.iter().map(|a| a.to_string()).collect::<Vec<_>>());

        let launch = read(&["--mcp", "8000"]).expect("a port is a port");
        assert_eq!(launch.mcp, Some(8000));
        // **On either side of the pair, like the two flags beside it.** An
        // operator types the flags in whatever order they think of them.
        let pair = shipped();
        let (l1, l4) = (pair.l1.display().to_string(), pair.l4.display().to_string());
        let launch = read(&[&l1, &l4, "--mcp", "0"]).expect("after the pair");
        assert_eq!(launch.mcp, Some(0), "a port after the pair");
        let launch = read(&["--mcp", "0", &l1, &l4]).expect("before the pair");
        assert_eq!(launch.mcp, Some(0), "a port before the pair");

        // And a run that does not ask serves nothing rather than a default port.
        assert_eq!(
            read(&[&l1, &l4]).expect("no flag").mcp,
            None,
            "a run that did not ask for a server was given one"
        );

        // The end of the line: nothing after the flag.
        let why = read(&["--mcp"]).expect_err("a flag with nothing after it");
        assert!(why.contains("--mcp"), "the refusal does not name it: {why}");
        assert!(
            why.contains("needs a value"),
            "the refusal is not the one the other flags give: {why}"
        );

        // The next flag is not a value: `--mcp --store x` must blame `--mcp`
        // rather than reading `--store` as a port and then blaming `x` for
        // being an unknown option.
        let why = read(&["--mcp", "--store", "somewhere"]).expect_err("a flag as a value");
        assert!(
            why.contains("--mcp") && why.contains("--store"),
            "the refusal does not say which flag ate which: {why}"
        );

        // And a value that is not a number.
        let why = read(&["--mcp", "eight-thousand"]).expect_err("a port that is not one");
        assert!(
            why.contains("eight-thousand") && why.contains("a port number"),
            "the refusal does not say what was expected: {why}"
        );
    }

    /// **A wire request reaches the slot's watcher, and the rest of that
    /// watcher's aim is restated with it.**
    ///
    /// The three points `mcp::WireRequest` owes, checked without a window: the
    /// edge is replaced rather than appended and keyed on the input, the slot is
    /// re-aimed with the run's whole wiring, and a slot this deck has not got is
    /// refused in the one sentence every surface refuses one in.
    ///
    /// **The twelve other fields are the point of the second assertion.** An
    /// `Aim` is every field of a slot's identity, and a rewiring that restated
    /// only the edges would come back with the outgoing slot's camera, fold and
    /// salts — a defect that shows on the *next* build rather than on the
    /// rewiring, which is why it is asserted here rather than left to be seen.
    #[test]
    fn a_wire_request_reaches_the_slots_watcher_with_the_rest_of_its_aim_restated() {
        let edge = |node: &str, slot: &str, to: &str| karakuri_engine::set::Edge {
            node: node.to_string(),
            slot: slot.to_string(),
            to: to.to_string(),
        };
        let (tx, rx) = std::sync::mpsc::channel();
        let mut aims = vec![Aiming {
            aim: tx,
            at: watch::Aim {
                head: karakuri_environment::compile::Named {
                    name: Some("grid".into()),
                    path: std::path::PathBuf::from("A0-grid.kir"),
                },
                rest: vec![karakuri_environment::compile::Named::bare("A1-points.kir")],
                layering: Layering::Composite,
                live: Some(0),
                capacity: Some(2048),
                seed_salt: 9,
                salts: vec![9],
                camera: karakuri_engine::camera::Orbit::default(),
                overrides: Vec::new(),
                published: Vec::new(),
                bindings: Vec::new(),
                edges: Vec::new(),
                authorities: Vec::new(),
            },
        }];
        let mut edges = Vec::new();

        let said = rewired(
            &[(0, edge("warp", "shape", "field"))],
            &mut edges,
            &mut aims,
            1,
        );
        assert_eq!(said.len(), 1);
        let line = said[0].as_ref().expect("the slot is in range");
        assert!(
            line.contains("warp.shape=field") && line.contains("recompiling"),
            "the answer does not say what was wired or that anything rebuilds: {line}"
        );
        let aim = rx.try_recv().expect("the watcher was not re-aimed at all");
        assert_eq!(aim.edges, vec![edge("warp", "shape", "field")]);
        // **The twelve fields that are not the edges.**
        assert_eq!(aim.head.name.as_deref(), Some("grid"));
        assert_eq!(aim.live, Some(0), "the fold was silently un-selected");
        assert_eq!(aim.capacity, Some(2048), "the capacity came back as none");
        assert_eq!(aim.salts, vec![9], "the salts would repaint every element");
        assert_eq!(aim.layering, Layering::Composite);

        // **The same input again is a replacement and not a second edge**,
        // because `SetError::SlotBoundTwice` refuses two edges on one input
        // where the Set is built — an append would make a model unable to
        // change its mind.
        let said = rewired(
            &[(0, edge("warp", "shape", "other"))],
            &mut edges,
            &mut aims,
            1,
        );
        assert!(said[0].is_ok(), "{:?}", said[0]);
        assert_eq!(
            edges,
            vec![edge("warp", "shape", "other")],
            "the run is wired with both, and the Set will refuse to build"
        );
        let aim = rx.try_recv().expect("the second request re-aimed nothing");
        assert_eq!(aim.edges, vec![edge("warp", "shape", "other")]);
        // And the aim the watcher is pointed at moved with it, so a third
        // request restates the second rather than the first.
        assert_eq!(aims[0].at.edges, vec![edge("warp", "shape", "other")]);

        // A slot this deck has not got, in the one sentence.
        let said = rewired(
            &[(3, edge("warp", "shape", "field"))],
            &mut edges,
            &mut aims,
            1,
        );
        let why = said[0].as_ref().expect_err("slot 3 of a deck of one");
        assert_eq!(
            why,
            &format!(
                "{}, and nothing was rewired",
                karakuri_environment::no_such_slot(3, 1)
            ),
            "the refusal is not the one every other surface gives"
        );
        assert_eq!(
            edges,
            vec![edge("warp", "shape", "other")],
            "a refused request wrote an edge anyway"
        );
        assert!(
            rx.try_recv().is_err(),
            "a refused request re-aimed a watcher"
        );
    }

    /// **The two flags say where this program's data is, and either may sit on
    /// either side of the pair.**
    ///
    /// The order half is the one an operator meets: they type the flags in
    /// whatever order they think of them, and `karakuri-cli` accepts `--store`
    /// before or after its own command for exactly this reason
    /// (`list_sets_prints_and_is_never_a_run`). A parser that matched on the
    /// argument slice — which is what this one was — can only ever accept one
    /// of the two spellings.
    ///
    /// **And the pair still wins**, which is the claim [`Sources`]'s doc makes
    /// about these flags not being a second material vocabulary: `--presets`
    /// moves what a run with *no* paths opens on and reaches nothing else, so
    /// a line with both a library and a pair plays the pair.
    ///
    /// Not quite a CPU test, and this is what changed: resolving a presets
    /// root is existence checks on real directories. The library it names is
    /// this workspace's own `examples/`, which is on the disk whenever these
    /// tests run at all.
    #[test]
    fn the_two_flags_say_where_the_data_is_and_may_sit_on_either_side_of_the_pair() {
        let of = |args: &[&str]| sources_from(args.iter().map(|a| (*a).to_string()));
        let library = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
        let library = library
            .to_str()
            .expect("this workspace's path is not utf-8");

        // **The default store is the shared constant**, which is the whole of
        // what deleting `const STORE` was for: this asserts the two programs
        // read one directory rather than two that look alike.
        assert_eq!(
            of(&[]).expect("a bare run").store,
            std::path::PathBuf::from(karakuri_environment::places::STORE),
            "a run that said nothing about a store did not get the shared default"
        );

        for spelling in [
            vec!["--store", "/tmp/library", "a/geo.kir", "b/ren.kir"],
            vec!["a/geo.kir", "b/ren.kir", "--store", "/tmp/library"],
            vec!["a/geo.kir", "--store", "/tmp/library", "b/ren.kir"],
        ] {
            let launch = of(&spelling).unwrap_or_else(|why| panic!("{spelling:?}: {why}"));
            assert_eq!(
                launch.store,
                std::path::PathBuf::from("/tmp/library"),
                "{spelling:?} read a store nobody asked for"
            );
            assert_eq!(
                launch.sources.l1,
                std::path::PathBuf::from("a/geo.kir"),
                "{spelling:?} lost the pair to the flag"
            );
            assert_eq!(launch.sources.l4, std::path::PathBuf::from("b/ren.kir"));
        }

        // `--presets` with no pair: it is what the pair defaults to, and the
        // resolution reports it as typed rather than as something found.
        let told = of(&["--presets", library]).expect("a library that is there");
        assert_eq!(
            told.sources.l1,
            std::path::Path::new(library).join("drift_shell.kir")
        );
        assert_eq!(
            told.sources.l4,
            std::path::Path::new(library).join("soft_points.kir")
        );
        assert_eq!(
            told.presets.as_ref().map(|presets| presets.found),
            Some(karakuri_environment::places::Found::Given),
            "a `--presets` an operator typed was reported as a place this program went \
             looking in"
        );

        // And with a pair, on either side: the pair wins and the library is
        // still the one that was named.
        for spelling in [
            vec!["--presets", library, "a/geo.kir", "b/ren.kir"],
            vec!["a/geo.kir", "b/ren.kir", "--presets", library],
        ] {
            let launch = of(&spelling).unwrap_or_else(|why| panic!("{spelling:?}: {why}"));
            assert_eq!(
                launch.sources.l1,
                std::path::PathBuf::from("a/geo.kir"),
                "{spelling:?}: `--presets` overrode the paths the operator named, which \
                 would make it a second way of saying what plays"
            );
            assert_eq!(launch.sources.l4, std::path::PathBuf::from("b/ren.kir"));
            assert_eq!(
                launch.presets.map(|presets| presets.dir),
                Some(std::path::PathBuf::from(library)),
                "{spelling:?} lost the library it was given"
            );
        }

        // A `--presets` that is not there is refused rather than searched
        // past, and the sentence is `places`' own — one refusal, whichever
        // program the operator reached it from.
        let missing = std::path::Path::new(library).join("no-such-library");
        let why = of(&["--presets", missing.to_str().expect("utf-8")])
            .expect_err("a `--presets` that is not there was accepted");
        assert_eq!(why, karakuri_environment::places::no_presets_at(&missing));

        // A flag with nothing after it, and a flag whose value is the next
        // flag. Neither falls back and neither swallows.
        for (spelling, wanted) in [
            (vec!["--presets"], "`--presets` needs a value"),
            (vec!["--store"], "`--store` needs a value"),
            (
                vec!["--presets", "--store", "/tmp/library"],
                "`--presets` was given no value — `--store` is an option, not one",
            ),
        ] {
            assert_eq!(
                of(&spelling).as_ref().err().map(String::as_str),
                Some(wanted),
                "{spelling:?}"
            );
        }

        // **An unknown option is not a path**, which is the mistake a typo
        // actually makes: without this, `--prests DIR` becomes a two-path Set
        // and is reported as a file that will not open.
        let typo =
            of(&["--prests", library]).expect_err("an unknown option was read as half of a Set");
        assert_eq!(typo, "unknown option `--prests`");
        assert!(
            !typo.is_empty(),
            "a refusal came back with no sentence in it, which `main` reads as `--help`"
        );
    }

    /// **The capacity is the L1's own declaration, read off the `Checked`.**
    ///
    /// It was `const CAPACITY: u32 = 262144` here — `drift_shell.kir`'s
    /// declared default, transcribed — for as long as this file could only ever
    /// load that one file. It takes a path now, so a transcription would be
    /// right about one `.kir` and silently wrong about every other: a procedure
    /// written for 131072 elements would run at 262144 and nothing would say
    /// so.
    ///
    /// **It is not `karakuri_ir::DEFAULT_CAPACITY` either**, which is the
    /// language default for a file that declared nothing and is what
    /// `check_header` makes unreachable for an L1 that passed checking. The
    /// number below is asserted rather than derived on purpose: it is what a
    /// bare `cargo run -p karakuri` runs, which is what makes the reading
    /// [`Costs::say`] prints comparable with the rest of this repository's
    /// figures (`docs/contributing.md` §1's reference workload).
    #[test]
    fn the_capacity_is_the_l1s_own_declaration_and_the_l4_declares_none() {
        let sources = shipped();
        let l1 = checked(&sources.l1);
        let declared = l1
            .capacity
            .expect("an L1 that passed contract checking always carries a capacity");
        assert_eq!(
            declared.default, 262_144,
            "the pair this program plays by default is no longer the workspace's \
             reference workload, and the reading printed after three quiet seconds \
             says it is"
        );
        assert!(
            declared.contains(declared.default),
            "the file's own default is outside the range the same file declares"
        );

        assert_eq!(capacity_of(&l1), declared.default);

        assert!(
            checked(&sources.l4).capacity.is_none(),
            "the renderer declares a capacity — `Set::build` is handed the L1's, and \
             two declarations would be two answers to how many elements there are"
        );

        // **A second L1, and it is the one that tells the two mistakes apart.**
        // `drift_shell.kir` declares 262144, which is also
        // `karakuri_ir::DEFAULT_CAPACITY` — so every assertion above passes
        // just as well against a [`capacity_of`] that ignored the file and
        // returned the language default. `strand_shell.kir` declares 131072
        // and says why in the file (512 strands x 256 samples), and it is what
        // that defect fails on.
        let other = checked(&sources.l1.with_file_name("strand_shell.kir"));
        assert_eq!(
            capacity_of(&other),
            131_072,
            "a second procedure did not run at what it declares — the capacity is being \
             read from somewhere other than the file"
        );
        assert_ne!(
            capacity_of(&other),
            karakuri_ir::DEFAULT_CAPACITY,
            "the second procedure declares the language default, so this test can no \
             longer tell a per-file read from a constant — pick another `.kir`"
        );
    }
}

/// **The pair a bare run plays, for the tests that need one on the disk.**
///
/// [`Sources::under`] takes a preset library and does not go looking for one;
/// this is the going-looking, and in a test binary the answer is always the
/// last candidate — the workspace this file was compiled in, which is also the
/// tree the test is run from. That is the development entry doing exactly what
/// it is for, and it is why these tests can assert the pair is on the disk
/// without an install anywhere.
///
/// A function rather than an `impl Default` on [`Sources`], because a
/// `Default` is what baked the build machine's own tree into a shipped binary:
/// a type whose default value is a search of the filesystem invites exactly
/// that call from production, and a production caller now has to say which
/// library it means.
///
/// **Below `mod tests` rather than beside [`Sources`], and that is not a
/// matter of taste.** [`key_column::bound`] reads this file's own text for the
/// keys the window loop binds and stops at the first line that is
/// `#[cfg(test)]`; a test-only item placed above the loop moves that stop line
/// up past the `match`, and the scan then finds nothing and every check built
/// on it passes over an empty set. It did exactly that once, on the way to
/// writing this. Here it is after the boundary, and reachable from all three
/// test modules — [`tests`], [`key_column`] and [`gpu`] — because it is at the
/// file's own scope, which the two that are not inside [`tests`] need.
/// One `.kir`, parsed and checked, for the tests that need a `Checked` and no
/// window.
///
/// **`karakuri-environment`'s own five stages and not a sixth spelling.** This
/// used to be a hand-rolled parse-then-check, which is what the run itself used
/// to build a slot from; the run compiles through
/// [`karakuri_environment::compile::sort_slot`] now, because that is the one
/// place that keeps the bytes a node's address is derived from
/// ([`karakuri_environment::compile::Placed::source`]). What is left here is a
/// test helper, and a test helper with its own compiler would be a second answer
/// to *does this file check* the day either moved.
///
/// **Below `mod tests` for [`shipped`]'s reason, which is the same reason and
/// was learned here.** [`key_column::bound`] stops reading this file at the
/// first `#[cfg(test)]` line, so a test-only item above the window loop's
/// `match` moves that stop line past every key arm — the scan then finds no
/// keys at all and both checks built on it pass over an empty set. This
/// function sat beside [`capacity_of`] when it became test-only, and that is
/// exactly what happened.
#[cfg(test)]
fn checked(path: &std::path::Path) -> karakuri_ir::typed::Checked {
    match karakuri_environment::compile::load(path) {
        Ok((checked, _)) => checked,
        Err(report) => panic!("{report}"),
    }
}

#[cfg(test)]
fn shipped() -> Sources {
    let presets = karakuri_environment::places::presets(None)
        .expect("nothing was typed, so there is no typed path to refuse")
        .expect(
            "no preset library was found from the test binary, so the workspace tree this \
             test compiled in has no `examples/` in it",
        );
    Sources::under(&presets.dir)
}

/// **The shipped pair in every slot**, for the tests that build an [`Engine`].
///
/// A *run* may not do this — [`working_copies`] is what a run calls, and its
/// whole point is that no two slots watch one file — and this helper is not a
/// way back to that. It is legal here for the reason the copies exist: nothing
/// in these tests edits a `.kir`, no watcher of theirs ever sees a change, and
/// a test that materialised into a temporary store would be asserting the
/// copies rather than the thing it is about. The one test that *is* about the
/// copies calls `working_copies` and is named after the claim.
#[cfg(test)]
fn shipped_slots() -> Vec<Sources> {
    std::iter::repeat_n(shipped(), SLOTS).collect()
}

#[cfg(test)]
mod key_column {
    //! **The key column of the manual, against the keys this window binds.**
    //!
    //! [ADR-0213](../../../docs/adr/0213-the-interface-milestones-meter-is-the-panel-column-and-has-means-an-operator-reaches-it.md)
    //! defined the *panel* column of `docs/manual/operations.html` — `has`
    //! means an operator running the instrument reaches the operation — and
    //! said nothing about the other three. The *key* column then stopped being
    //! well defined, because
    //! [ADR-0214](../../../docs/adr/0214-the-program-moves-out-of-the-cli-and-two-thin-binaries-sit-over-it.md)
    //! gave this workspace a second keyboard: `karakuri-cli` binds thirty-nine
    //! keys and this program binds every key in [`super::KEYS`], **seven
    //! letters mean different things on the two**, and a badge saying
    //! `key f g` did not say whose.
    //! (Nine when ADR-0220 was written; the library's load route added seven —
    //! the four that select a deck, the two that walk the library cursor, and
    //! `l`. The four are also one of the places beyond `esc` where the two
    //! keyboards **agree**, because a deck is a slot number and there was
    //! nothing to translate. Eight when the audio session landed and `b`,
    //! `,` and `.` joined them, and seven since `p` stopped being the panel's
    //! report and became the latency offset the page specifies — the one
    //! letter this column has ever taken *back* from the panel, and the pair
    //! `o` and `p` agree on both keyboards now.)
    //!
    //! The page now says whose, in its legend: **the key column is the
    //! instrument's keyboard**, which is this file's `match` on
    //! `key.logical_key`. That is ADR-0213's definition one column along — the
    //! property is *the operator at the panel presses it*, and *which cargo
    //! target binds a letter* is the shape ([P-0060](../../../docs/principles/0060-name-the-property-not-the-shape.md)).
    //! This is the check that definition owes, both ways round, in the shape
    //! `karakuri-environment/src/mcp.rs` uses for the MCP column and
    //! `karakuri-console/tests/panel_column.rs` for the panel one.
    //!
    //! # Why the check is here and can be nowhere else
    //!
    //! The keys are in this file, and **nothing in this workspace may depend on
    //! this package** — it is a binary with no library target on purpose, as
    //! the crate header says: *a surface is where the buck stops*. The two
    //! files that check the panel column both stop at exactly this boundary and
    //! say so: `panel_column.rs` — *"reachability is a property of
    //! `crates/karakuri/src/main.rs` … and this crate takes no device and
    //! cannot depend on that binary (ADR-0156). So this file checks the
    //! necessary half and not the sufficient one"* — and `vocabulary.rs` the
    //! same. A key column check has that problem twice over, because the other
    //! keyboard is in `karakuri-cli`, which no crate can depend on either.
    //!
    //! So it is a unit test in the binary that holds the keys. It cannot be an
    //! integration test under `crates/karakuri/tests/`, because a package with
    //! no library target has nothing for one to `use`; the arms are reachable
    //! only from inside this file's own `#[cfg(test)]`.
    //!
    //! **The command line's keyboard is not this file's and not this column's.**
    //! `karakuri-cli` documents its own keys in `BINDINGS` and has its own test
    //! that every key `Live::key` acts on is in it. Nothing here reads that
    //! package, and a second copy of its list here would be the thing
    //! [P-0045](../../../docs/principles/0045-generate-the-vocabulary-prose-drifts-from-code.md)
    //! forbids.
    //!
    //! # What it cannot see, and which way each one fails
    //!
    //! - **A key bound anywhere but a literal arm** — through a table, a
    //!   helper, or `egui`'s own shortcut handling. Invisible to [`bound`], and
    //!   a *false negative*: it cannot fail the direction that says every bound
    //!   key is on the page, and it surfaces from the other direction the
    //!   moment somebody marks that row built.
    //! - **A key arm inside a block comment.** `/* … */` is not a line comment
    //!   and reads as bound. A *false positive*, and it fails loudly:
    //!   [`super::KEYS`] has no entry for it and
    //!   [`the_keys_this_file_lists_are_the_keys_the_window_loop_binds`] names
    //!   it.
    //! - **This file does not press a key.** It reads an arm and reads the
    //!   page. That `Op::Solo` actually solos is
    //!   `karakuri-console/tests/vocabulary.rs`'s, which asks a running `Panel`;
    //!   that the arm is reached at all is what
    //!   `tests::a_drag_through_the_window_loops_own_routing_never_reaches_egui`
    //!   asks about the pointer, and nothing asks it for keys.
    //!   **`egui` sees every key before this `match` does**, and if it ever
    //!   grew a focused widget that consumed one, the arm would still be here
    //!   and this file would go on claiming an operator reaches it. That is the
    //!   sufficient half, and it is not checked here either — one boundary
    //!   further out than the two files above stop at.
    //! - **Which rows a key lands on is written down rather than derived**, in
    //!   [`ROWS`]. It has to be: `Op::Fold` folds a bay or a pane depending on
    //!   what the pointer is over, and only the page separates those two rows.
    //!   A wrong entry is a wrong claim, and it cannot be *quietly* wrong —
    //!   both assertions below read the same list, so an entry naming a row
    //!   that is not marked built fails one and a badge naming a key no entry
    //!   claims fails the other.
    //! # Two rows this program deliberately binds no key to
    //!
    //! *Save the arrangement* and *Put a saved arrangement back* each carry a
    //! name the operator picked, and **a bare key press cannot type one**.
    //! ADR-0221 §1 provides a fallback — a surface that cannot type a name
    //! passes a `history::stamped_id` stamp, as `accepted_save` does for a Set
    //! — and it is declined here for two reasons, both of which would show up
    //! as a badge that lies:
    //!
    //! - **Putting one back cannot be bound at all.** Nothing at a key press
    //!   says *which* arrangement, and *the most recent* is a handle derived
    //!   from where a file sits, which is the failure
    //!   [P-0053](../../../docs/principles/0053-a-value-that-must-be-stable-is-recorded-not-derived.md)
    //!   is about and the one ADR-0221 rejected a slot number over.
    //! - **So a save key alone would keep arrangements nothing can put back.**
    //!   This program has no control that lists them and no way to show an
    //!   operator the stamp it picked for them, and `ArrangementEntry`'s own
    //!   documentation says an arrangement is *"saved by an operator who is
    //!   telling the console what to call this shape"*. A `has` badge would be
    //!   true of the press and false of everything the press was for.
    //!
    //! Both rows therefore carry four empty badges on [`PAGE`], and the page
    //! says the same thing in its own words. Binding either one is a change to
    //! that specification first, and it wants the control this page's *panel*
    //! badges now name — the transport row — rather than a letter.
    //!
    //! - **Only the key column.** The panel column is
    //!   `karakuri-console`'s two files, the MCP column is
    //!   `karakuri-environment/src/mcp.rs`, and **the MIDI column is checked by
    //!   nothing** — which this file says rather than being read as covering
    //!   it.

    use std::collections::BTreeSet;
    use std::fs;
    use std::path::{Path, PathBuf};

    use super::KEYS;

    /// The specification, relative to the workspace root.
    const PAGE: &str = "docs/manual/operations.html";

    /// The keys, relative to the same root: this file, read as text. There is
    /// no other way to ask *which keys does this program bind* from inside its
    /// own test binary — the `match` is a `match`, not a table.
    const SRC: &str = "crates/karakuri/src/main.rs";

    /// What marks a row on the page — the marker `panel_column.rs`,
    /// `vocabulary.rs` and `mcp.rs` all match, for the reason the first of them
    /// gives: sections are `<h2>` and a heading somebody adds for looks is
    /// neither.
    const ROW: &str = r#"<div class="op-head">"#;

    /// The badge text of a route that names nothing. A `plan` or `gap` badge is
    /// allowed to be this; a `has` badge is not, because it would claim an
    /// operator reaches the operation and decline to say what to press.
    const NOWHERE: &str = "&mdash;";

    /// Where [`bound`] stops reading. Everything below the first of these in
    /// this file is a test, and a key spelled in a test is not a key this
    /// program binds — including the ones spelled in [`ROWS`] a few lines down
    /// and in [`super::KEYS`] above, which would otherwise make the scan agree
    /// with itself.
    const TESTS: &str = "#[cfg(test)]";

    /// The two arm shapes the window loop's `match` is written in.
    const CHARACTER: &str = r#"Key::Character(""#;
    const NAMED: &str = "Key::Named(NamedKey::";

    /// How the page spells a named key. Nothing in `NamedKey::Escape` says
    /// `esc`, and the page is written for a person rather than for `winit`;
    /// `karakuri-cli` keeps the same two-column table for the same reason.
    ///
    /// **Three of the four are only live while a name is being typed**, and
    /// they are spelled all the same: this table is what a key is *called*,
    /// and when it is bound is [`super::KEYS`]' business.
    const NAMED_KEYS: &[(&str, &str)] = &[
        ("Escape", "esc"),
        ("Enter", "return"),
        ("Backspace", "backspace"),
        ("Space", "space"),
        ("ArrowUp", "up"),
        ("ArrowDown", "down"),
    ];

    /// **Every key this program binds, and the rows of [`PAGE`] it reaches.**
    ///
    /// One entry per key in [`super::KEYS`], which is the table the window
    /// loop's legend prints and the one
    /// [`the_keys_this_file_lists_are_the_keys_the_window_loop_binds`] holds
    /// the `match` to. **The keys are not written twice**: that they are the
    /// same keys is
    /// [`every_key_the_legend_prints_has_its_rows_written_down`], both ways
    /// round, so a key added to the legend with no rows recorded — or a row
    /// mapping left behind by a key that went — fails here rather than at the
    /// page. What is written down is only the *rows*, and it stays in this
    /// module because a page heading is what a check reads and is not
    /// something the program says to anybody.
    ///
    /// The rows are the page's headings byte for byte. Where an arm resolves
    /// through
    /// `karakuri_console::panel::Op`, the rows are that variant's — the mapping
    /// `karakuri-console/tests/vocabulary.rs` pins in `rows_of`, which is why
    /// `f` and `g` each name two rows: a fold is a bay or a pane depending on
    /// the region under the pointer, and the page describes those as two
    /// consequences.
    ///
    /// An empty list is a key that reaches no row, and [`NO_ROW`] is where the
    /// reason goes.
    const ROWS: &[(&str, &[&str])] = &[
        // `Op::Fold` of the region under the pointer.
        ("f", &["Fold a bay away", "Fold a pane away"]),
        // `Op::FoldEnclosing` over a region, `Op::Fold` of the split over a
        // gap — one step up the tree either way, so the same two rows.
        ("g", &["Fold a bay away", "Fold a pane away"]),
        // The room's colours. Nothing in the arrangement moves and no
        // `Outcome` says so, which is why it is not an operation.
        ("n", &[]),
        ("r", &["Reset the arrangement"]),
        ("s", &["Solo a region"]),
        // `Op::UnfoldAll` — the page carries the region and the everything
        // under one heading, as `vocabulary.rs` does.
        ("u", &["Solo a region"]),
        ("z", &["Bring back what is folded"]),
        ("esc", &["Quit"]),
        // **The deck selection**, and the four that agree with the command
        // line: `0`–`3` mean `SelectDeck` on both keyboards, because a deck is
        // a slot number and there was nothing to translate.
        ("0", &["Select a deck"]),
        ("1", &["Select a deck"]),
        ("2", &["Select a deck"]),
        ("3", &["Select a deck"]),
        // The load, whose two operands are the library cursor and the
        // selection above. Free on both keyboards when it was chosen.
        //
        // **One key, and a preset row reaches a second row through it.**
        // Taking a Set in is not a row of its own — ADR-0229's *one
        // operation, two moments* — so a press on a `presets` row performs
        // *Send a Set to somebody, and take one in* at the moment of the
        // press and then this. That row's key badge names no key: what an
        // operator reaches from the keyboard is a **load**, and the taking-in
        // is what the load does on the way, which is exactly the distinction
        // ADR-0213 draws between reaching an operation and something
        // happening.
        //
        // **The press does now emit that second row**, and it stays out of
        // this entry all the same: `super::preset_press` builds
        // `Operation::TransferSet` beside the load so that a press names both
        // rows it performs, and *constructing* an operation is neither
        // reaching it nor a badge —
        // `karakuri-console/tests/panel_column.rs`'s own sentence,
        // *"construction is not reachability, and reachability is the
        // definition."* A key badge here would claim an operator can ask for a
        // transfer from the keyboard, and they cannot: the only way to that
        // emission is a load off a row of `presets`.
        ("l", &["Load material into a deck"]),
        // **The save, whose operand is the selection the load's is.** The
        // command line reaches this row with the same letter and from its own
        // focus; the two agree because a deck is a slot number, which is the
        // deck keys' argument one row along.
        //
        // **It is the key column and not the panel column that this makes
        // `has`.** The Library bay draws no *keep* control, so the row's panel
        // badge stays `plan` — a key is not a control, and a badge that named
        // one would be a claim about something that is not drawn.
        ("k", &["Keep what a deck is playing"]),
        // **The scope, and it is the one key here whose row the page marks
        // `plan` in every other column.** The chips are drawn by
        // `karakuri-console` and pressed by nobody: `SelectScope` is emitted
        // from this file's `match` and never from a control, so the panel
        // column stays `plan` and this key is what makes the key column
        // `has`.
        ("e", &["Choose which scope the library shows"]),
        // **The three that need a room**, and they are the first keys here
        // that reach neither the arrangement nor the deck. `b` is a tap and
        // `,` and `.` are the octave; each performs against the audio session
        // this program opened, and each says so when there is none rather
        // than doing nothing (`super::tapped`, `super::scaled`).
        //
        // **And the third row moved with them, which took a letter back.**
        // *Nudge the latency offset* is specified as `o` and `p`; `p` was
        // `Op::Report` here and a badge naming two keys with one of them
        // bound would be a badge that lies, so the decision the page was
        // waiting on was made on 2026-08-31 and it was the first of the three
        // it named: the instrument takes the letters and the panel diagnostic
        // keeps no key. All five of these mean the same thing on both
        // keyboards.
        ("b", &["Tap the beat"]),
        (",", &["Halve or double the grid"]),
        (".", &["Halve or double the grid"]),
        ("o", &["Nudge the latency offset"]),
        ("p", &["Nudge the latency offset"]),
        // **The library cursor, and it reaches no row on purpose.** Nothing in
        // the vocabulary moves it: `docs/manual/console.html` decides that
        // where the deck selection has a row of its own, and the argument is
        // that three of the four surfaces would have nothing to reach — a map
        // cannot name a Set, a model names one outright in `LoadSet`, and the
        // panel's route is the drag. A row would be the first rule written
        // down as a permanent gap.
        ("up", &[]),
        ("down", &[]),
        // **The three keys that are only live while the arrangement pill is
        // asking for a name, and they reach no row on purpose.**
        //
        // ADR-0221 records that **no key is bound to saving or restoring an
        // arrangement**, and that is still true: these three do not *name* the
        // operation and cannot be pressed to reach it. A save is reached by
        // opening the pill's menu and picking *save*, which is a pointer, and
        // the *Save the arrangement* row's key badge says `&mdash;` because
        // there is no way to that operation from the keyboard alone — which is
        // what the key column means (ADR-0213).
        //
        // What they are is the letters of a name and the two ends of typing
        // one. A row for `return` would claim an operator can save by pressing
        // it, and the honest test of that claim is to press it on a console
        // nobody has opened the menu on: nothing happens at all.
        ("return", &[]),
        ("backspace", &[]),
        ("space", &[]),
    ];

    /// The keys that reach no row, so that one which starts reaching one stops
    /// being an exception, and a new exception is written down rather than
    /// discovered. The reasons are at the entries in [`ROWS`].
    const NO_ROW: &[&str] = &["n", "up", "down", "return", "backspace", "space"];

    fn workspace() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("workspace root")
    }

    fn page() -> String {
        let path = workspace().join(PAGE);
        fs::read_to_string(&path).unwrap_or_else(|e| {
            panic!(
                "{} is the specification and could not be read: {e}",
                path.display()
            )
        })
    }

    /// **Every key the window loop's `match` binds**, read out of this file.
    ///
    /// Two cuts, `panel_column.rs`'s: a line whose first non-space characters
    /// are `//` is dropped whole, and what is left is truncated at its first
    /// `//`. The read stops at [`TESTS`].
    fn bound() -> BTreeSet<String> {
        let path = workspace().join(SRC);
        let text = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("{} holds the keys and is unreadable: {e}", path.display()));
        let mut found = BTreeSet::new();
        for line in text.lines() {
            let line = line.trim();
            if line == TESTS {
                break;
            }
            if line.starts_with("//") {
                continue;
            }
            let code = match line.find("//") {
                Some(at) => &line[..at],
                None => line,
            };
            for after in code.split(CHARACTER).skip(1) {
                let Some(shut) = after.find('"') else {
                    continue;
                };
                found.insert(after[..shut].to_owned());
            }
            for after in code.split(NAMED).skip(1) {
                let end = after
                    .find(|c: char| !c.is_alphanumeric() && c != '_')
                    .unwrap_or(after.len());
                let name = &after[..end];
                let spelled = NAMED_KEYS
                    .iter()
                    .find(|(winit, _)| *winit == name)
                    .map(|(_, page)| *page)
                    .unwrap_or_else(|| {
                        panic!(
                            "this file binds `NamedKey::{name}` and `NAMED_KEYS` has no spelling \
                             for it — {PAGE} names a key in the words a person would say, and \
                             nothing in `winit`'s name is those words"
                        )
                    });
                found.insert(spelled.to_owned());
            }
        }
        found
    }

    /// **Every row's title and its key badge**, in page order: the badge's
    /// class — `has`, `plan` or `gap` — and the keys it names.
    ///
    /// Read verbatim and never decoded, which is `mcp.rs`'s rule and
    /// `panel_column.rs`'s after it: a badge that names nothing says `&mdash;`,
    /// and a key that needed decoding to match would be a key nobody could find
    /// on their keyboard.
    fn key_badges() -> Vec<(String, String, String)> {
        let html = page();
        let mut found = Vec::new();
        for row in html.split(ROW).skip(1) {
            let Some(open) = row.find("<h3>") else {
                continue;
            };
            let rest = &row[open + "<h3>".len()..];
            let Some(close) = rest.find("</h3>") else {
                continue;
            };
            let title = rest[..close].to_string();
            // The row ends where the next section does; a badge found past
            // that would belong to another row.
            let body = &rest[close..];
            let body = &body[..body.find("</section>").unwrap_or(body.len())];
            let mut badge = None;
            for span in body.split(r#"<span class="rt "#).skip(1) {
                let Some(quote) = span.find('"') else {
                    continue;
                };
                let class = span[..quote].to_string();
                let Some(text) = span[quote..].strip_prefix(r#"">key <b>"#) else {
                    continue;
                };
                let Some(shut) = text.find("</b>") else {
                    continue;
                };
                badge = Some((class, text[..shut].to_string()));
                break;
            }
            let Some((class, keys)) = badge else {
                continue;
            };
            found.push((title, class, keys));
        }
        found
    }

    /// The rows [`ROWS`] says a key reaches, or `None` if this program does not
    /// bind it at all.
    fn rows_of(key: &str) -> Option<&'static [&'static str]> {
        ROWS.iter().find(|(k, _)| *k == key).map(|(_, rows)| *rows)
    }

    /// The floor under both directions: a scan that matched nothing would
    /// satisfy every loop below by iterating over nothing at all.
    #[test]
    fn the_scan_finds_the_page_and_the_keys() {
        let badges = key_badges();
        assert!(
            badges.len() >= 54,
            "only {} rows with a key badge found in {PAGE} — is a row still `{ROW}` followed by \
             an `<h3>` and its `rt` badges?",
            badges.len()
        );
        assert!(
            bound().len() >= 9,
            "only {} keys found bound in {SRC} — the window loop's `match` has more arms than \
             this, and a scan below it is a scan that has stopped matching code",
            bound().len()
        );
    }

    /// **The list, the `match` and the legend are one list.**
    ///
    /// [`super::KEYS`] is the table the window loop prints when it starts, and
    /// it is the one thing here the compiler cannot check: an arm added
    /// without an entry — or an entry left behind by an arm that went —
    /// arrives as a failure rather than as a key nobody noticed had stopped
    /// being reachable, *or as a legend that goes on telling an operator this
    /// program folds, solos, resets and quits*.
    ///
    /// That second half is why the printed table is the checked one. It was a
    /// separate list of nine `println!`s, and it stayed at nine while ten more
    /// keys were bound: the maintainer who read it reported the program
    /// unchanged, which it was not.
    #[test]
    fn the_keys_this_file_lists_are_the_keys_the_window_loop_binds() {
        let listed: BTreeSet<String> = KEYS.iter().map(|(k, _)| (*k).to_owned()).collect();
        assert_eq!(
            bound(),
            listed,
            "the keys the `match` in `window_event` binds are not the ones `KEYS` lists — which \
             is the list the legend prints. An arm this file does not know about reaches an \
             operation nothing checks the badge of and is told to nobody; an entry with no arm \
             is a legend naming a key an operator presses to no effect"
        );
    }

    /// **And every key the legend prints has its rows written down**, both
    /// ways round, which is what keeps [`ROWS`] from being a second list of
    /// keys rather than a mapping off the first.
    #[test]
    fn every_key_the_legend_prints_has_its_rows_written_down() {
        let printed: BTreeSet<&str> = KEYS.iter().map(|(k, _)| *k).collect();
        let mapped: BTreeSet<&str> = ROWS.iter().map(|(k, _)| *k).collect();
        assert_eq!(
            printed, mapped,
            "a key the legend prints has no entry in `ROWS`, or `ROWS` maps a key the legend \
             does not print. The rows a key reaches cannot be derived — a fold is a bay or a \
             pane depending on the pointer — so the mapping is written down, and this is what \
             says it is written down for exactly the keys this program binds"
        );
    }

    /// And the keys that reach no row are exactly [`NO_ROW`], both ways round.
    #[test]
    fn the_keys_that_reach_no_row_are_the_ones_written_down() {
        let silent: Vec<&str> = ROWS
            .iter()
            .filter(|(_, rows)| rows.is_empty())
            .map(|(k, _)| *k)
            .collect();
        assert_eq!(
            silent, NO_ROW,
            "the keys that reach no row on {PAGE} are not the ones this file says they are — a \
             key that performs something the page never specified is a route nobody named"
        );
    }

    /// **A key reaching past the page.**
    ///
    /// A key this program binds whose row is not marked built in the key column
    /// — ADR-0213's failure mode from the side where the code moved first,
    /// which is how this whole column came to be wrong: the panel binary was
    /// given six arrangement keys and six rows went on reading `gap`.
    #[test]
    fn every_key_the_instrument_binds_reaches_a_route_marked_built() {
        let badges = key_badges();
        for (key, rows) in ROWS {
            for row in *rows {
                let found = badges
                    .iter()
                    .find(|(title, _, _)| title == row)
                    .unwrap_or_else(|| {
                        panic!(
                            "`{key}` reaches `{row}` and {PAGE} has no row with that heading — \
                             the page is the specification, so add the row there first"
                        )
                    });
                assert_eq!(
                    found.1, "has",
                    "`{key}` performs `{row}` at the panel, which {PAGE} marks `{}` in the key \
                     column — an operation an operator reaches from the keyboard and a page \
                     that says the instrument does not. Flip the badge, or say here why the key \
                     does not reach it",
                    found.1
                );
                assert!(
                    found.2.split_whitespace().any(|k| k == *key),
                    "`{key}` performs `{row}` and {PAGE} marks that row built in the key column \
                     naming `{}` — a badge that says an operator reaches it by pressing \
                     something else",
                    found.2
                );
            }
        }
    }

    /// **The page claiming a key nothing binds.**
    ///
    /// It fails apart from the test above because it is the other failure: that
    /// one says the program reached past the specification, this one says the
    /// specification tells a player to press a key the instrument does not
    /// read. It is the likelier of the two here, because twenty rows carried a
    /// built badge for `karakuri-cli`'s keyboard before the column said whose
    /// it was.
    #[test]
    fn every_key_route_the_page_marks_built_is_bound_by_the_instrument() {
        let badges = key_badges();
        let claimed: Vec<&(String, String, String)> = badges
            .iter()
            .filter(|(_, class, _)| class == "has")
            .collect();
        assert!(
            claimed.len() >= 6,
            "only {} rows of {PAGE} mark a key route built — the scan found less than the column \
             holds, which would pass this test by finding nothing",
            claimed.len()
        );
        for (title, _, keys) in claimed {
            assert_ne!(
                keys, NOWHERE,
                "{PAGE} marks `{title}` built in the key column and names no key for it — a \
                 `has` badge says an operator reaches the operation, so it has to say what to \
                 press"
            );
            for key in keys.split_whitespace() {
                let rows = rows_of(key).unwrap_or_else(|| {
                    panic!(
                        "{PAGE} marks `{title}` built in the key column and names `{key}`, which \
                         this program does not bind — the page tells a player to press a key the \
                         instrument does not read. Either the key went and the badge is `plan` \
                         again, or it is another program's: the key column is the instrument's \
                         keyboard, and `karakuri-cli`'s keys are its own"
                    )
                });
                assert!(
                    rows.contains(&title.as_str()),
                    "{PAGE} marks `{title}` built in the key column and names `{key}`, which \
                     this program binds to {rows:?} instead — one letter, two operations"
                );
            }
        }
    }
}

#[cfg(test)]
mod gpu {
    //! The console, through `egui`, through `wgpu` 30, onto a real device.

    /// **A temporary root nobody else's run wrote into**, shared with
    /// `mod tests` rather than spelled twice: two answers to *where does a test
    /// put its store* is two directories to clean up and one of them stale.
    use super::tests::scratch_dir;
    use super::*;
    /// **The engine's own fitting, asked rather than re-derived.** It is what
    /// `Present::draw` sets its viewport from, so what it leaves over at the
    /// edges of a target is exactly the bar that gets cleared to black — and a
    /// copy of the arithmetic here would be a test agreeing with itself about
    /// the one thing it is checking.
    use karakuri_engine::letterbox;

    /// **A [`Keeping`] with nothing served and nothing yet built**, which is
    /// what a run holds on its first frame.
    fn keeping() -> Keeping {
        let (_, built) = std::sync::mpsc::channel();
        let (save_tx, saves) = std::sync::mpsc::channel();
        Keeping {
            mcp: None,
            edges: Vec::new(),
            playing: Playing {
                playing: Vec::new(),
                previous: Vec::new(),
            },
            built,
            pending: Vec::new(),
            saves,
            save_tx,
            in_flight: 0,
        }
    }

    /// One request, one reply, over TCP exactly as a client would — the shape
    /// `karakuri-environment/src/mcp.rs`'s own `wire_tests` use, restated here
    /// because that module is `#[cfg(test)]` and nothing outside it can call in.
    ///
    /// **Over a socket, because that is the only way to read a
    /// [`mcp::Reporter`] back.** A report handed to the server goes into a queue
    /// only the protocol can drain, which is exactly the property under test: a
    /// swap the lane drew is a swap a model can ask about.
    fn call(port: u16, name: &str, args: serde_json::Value) -> (bool, String) {
        use std::io::{BufRead, Write};
        let body = serde_json::json!({"jsonrpc":"2.0","id":1,"method":"tools/call",
                                      "params":{"name":name,"arguments":args}})
        .to_string();
        let request = format!(
            "POST / HTTP/1.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        );
        let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).expect("connect");
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .expect("a read timeout, so a wedged server fails as a timeout");
        stream.write_all(request.as_bytes()).expect("write");
        let mut reader = std::io::BufReader::new(stream);
        let mut line = String::new();
        reader.read_line(&mut line).expect("no status line");
        let mut length = 0usize;
        loop {
            let mut header = String::new();
            reader.read_line(&mut header).expect("header");
            let header = header.trim_end();
            if header.is_empty() {
                break;
            }
            if let Some((name, value)) = header.split_once(':') {
                if name.eq_ignore_ascii_case("content-length") {
                    length = value.trim().parse().unwrap_or(0);
                }
            }
        }
        let mut body = vec![0u8; length];
        std::io::Read::read_exact(&mut reader, &mut body).expect("body");
        let reply: serde_json::Value =
            serde_json::from_slice(&body).expect("the answer is not JSON");
        let result = &reply["result"];
        (
            result["isError"].as_bool().unwrap_or(true),
            result["content"][0]["text"]
                .as_str()
                .unwrap_or("")
                .to_string(),
        )
    }

    /// **A save writes what the deck is playing as a Set file, and the file
    /// loads back.**
    ///
    /// This is the whole of what *Keep what a deck is playing* is, from the
    /// press or the tool call through to a `.set` in the store — and it is a
    /// device test because the one line of it that needs a `Deck` is the one
    /// that reads what the slot is playing ([`playing_values`]).
    ///
    /// **It is asserted against the deck rather than against the flags**, which
    /// is the whole reason that function exists: the capacity written down is
    /// the geometry's own declaration and the salt is the one this slot is
    /// running at, so a save that read the command line would record a picture
    /// nobody has seen.
    ///
    /// **Nothing has been rebuilt when this saves**, which is the case
    /// [`Playing::at_launch`] exists for: a deck could not be written down at
    /// all until the launch version had an address, and the failure it replaces
    /// is a refusal saying the sources are not in the store on a run where they
    /// are.
    #[test]
    fn a_save_writes_what_the_deck_is_playing_as_a_set_file_that_loads_back() {
        let gpu = Gpu::headless().expect("no GPU");
        let mut renderer = egui_wgpu::Renderer::new(
            &gpu.device,
            wgpu::TextureFormat::Rgba8Unorm,
            egui_wgpu::RendererOptions::default(),
        );
        let mut panel = Panel::new(1440.0, 900.0);
        view::rearrange(&mut panel, CANVAS);
        let engine = Engine::new(
            &gpu,
            &mut renderer,
            &shipped_slots(),
            panel.layout(),
            1.0,
            None,
        );

        let root = scratch_dir("kept");
        let mut keeping = keeping();
        keeping.playing = Playing::at_launch(&engine.placed, engine.deck.slot_count());
        // Deck B, so a hard-coded slot 0 fails here.
        keeping.save_set(&engine, &root, ASKED_TO_PRIME, Some("kept01".into()), None);
        assert_eq!(keeping.in_flight, 1, "the save was never started");

        // The save is on a thread of its own and no frame waits for it; this is
        // what the end of a run does — see [`Keeping::awaited_saves`].
        keeping.awaited_saves();
        assert_eq!(keeping.in_flight, 0, "the save never came back");

        let store = Store::open(&root).expect("the store the save made");
        let loaded = setfile::load(&store, "kept01").expect("the file it wrote");
        assert_eq!(
            loaded.srcs.len(),
            engine.placed.len(),
            "the file does not name every node the slot is running"
        );
        // **What the deck is running, not what a flag says.** The capacity is
        // the L1's own declaration and the salt is this slot's.
        assert_eq!(
            loaded.capacities,
            vec![Some(engine.capacity)],
            "the capacity written down is not the one the deck is drawing"
        );
        assert_eq!(
            loaded.salts,
            vec![Some(slot_salt(ASKED_TO_PRIME))],
            "the salt written down is not the one this slot is salted with"
        );

        std::fs::remove_dir_all(&root).expect("clean up");
    }

    /// **The swap report says what the lane says**, because the lane and the
    /// server are told by one drain.
    ///
    /// `Deck::events` empties the channel, so there is no second drain to be
    /// had: a loop that read it again would read nothing, and a server told
    /// from anywhere else would be told about a different build. That is why
    /// [`staging`] reports rather than a function beside it, and this is the
    /// check that fact owes — the sentence a model reads out of `swap_outcome`
    /// is the event the row was written from.
    ///
    /// **And what the slot is now playing moves with it.** A build that landed
    /// and was not taken up is a build a save would write the *previous*
    /// version of, so the address is asserted here rather than left to the save
    /// test, which never rebuilds anything.
    #[test]
    fn the_swap_report_says_what_the_lane_says() {
        let gpu = Gpu::headless().expect("no GPU");
        let mut renderer = egui_wgpu::Renderer::new(
            &gpu.device,
            wgpu::TextureFormat::Rgba8Unorm,
            egui_wgpu::RendererOptions::default(),
        );
        let mut panel = Panel::new(1440.0, 900.0);
        view::rearrange(&mut panel, CANVAS);

        // **Its own copies, because this test edits a `.kir`** — which is what
        // `working_copies` is for and why no other device test in this file
        // needs it.
        let root = scratch_dir("swapped");
        let (_, running) =
            working_copies(&root, &shipped(), SLOTS).expect("the copies this deck runs from");
        let store = std::sync::Arc::new(Store::open(&root).expect("store"));
        let (built_tx, built) = std::sync::mpsc::channel();
        let mut engine = Engine::new(
            &gpu,
            &mut renderer,
            &running,
            panel.layout(),
            1.0,
            Some((std::sync::Arc::clone(&store), built_tx)),
        );

        let reporter = mcp::serve(
            0,
            mcp::Slots(
                running
                    .iter()
                    .map(|pair| (pair.l1.clone(), vec![pair.l4.clone()]))
                    .collect(),
            ),
            root.clone(),
            true,
            Opening::closed(),
        )
        .expect("an ephemeral port");
        let port = reporter.port();

        let mut keeping = keeping();
        keeping.built = built;
        keeping.playing = Playing::at_launch(&engine.placed, engine.deck.slot_count());
        let was = keeping.playing.at(ON_AIR).expect("seeded at launch")[0].hash;
        keeping.mcp = Some(reporter);

        // An edit the watcher will pick up: the same procedure, one comment
        // longer, so it compiles and its bytes are different.
        let l1 = &running[ON_AIR].l1;
        let edited = format!(
            "{}\n// an edit\n",
            std::fs::read_to_string(l1).expect("read")
        );
        std::fs::write(l1, edited).expect("write");

        // The worker polls every hundred milliseconds and wants two polls of
        // quiet before it builds, then compiles; the swap lands at a frame
        // boundary, which is `begin_frame`.
        let mut rows: Vec<view::Candidate> = Vec::new();
        let mut took: Vec<(usize, Option<u64>)> = Vec::new();
        let deadline = Instant::now() + Duration::from_secs(30);
        while took.is_empty() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(50));
            drop(engine.deck.begin_frame(&gpu.device, &gpu.queue));
            staging(&mut engine.deck, &mut rows, keeping.mcp.as_ref(), &mut took);
        }
        assert!(
            !took.is_empty(),
            "nothing was built in 30s — the watcher never saw the edit"
        );
        let row = rows
            .iter()
            .find(|row| row.deck == ON_AIR)
            .expect("the lane drew no row for the deck that rebuilt");
        assert_eq!(
            row.stage,
            view::Stage::Landed,
            "the build did not land, so this test is not about what it says it is"
        );

        // **The same sentence, out of the server.** `swap_outcome` answers with
        // the reports the render loop handed over, newest last.
        let (failed, said) = call(port, "swap_outcome", serde_json::json!({}));
        assert!(!failed, "swap_outcome refused: {said}");
        assert!(
            said.contains(&format!("slot {ON_AIR}:")) && said.contains(&row.name),
            "the server was told something the lane was not: {said} against `{}`",
            row.name
        );

        // And the slot is playing the new bytes, so a save would write them.
        for (slot, landed) in took {
            keeping.took_up(&engine, slot, landed);
        }
        let now = keeping.playing.at(ON_AIR).expect("still addressable")[0].hash;
        assert_ne!(
            was, now,
            "the build landed and the slot is still addressed as what it launched with"
        );

        std::fs::remove_dir_all(&root).expect("clean up");
    }

    /// **A real Set reads out into a pane**, which is the seven reads
    /// [`inspector`] makes held against a Set this program actually builds
    /// rather than against a fixture it wrote itself (P-0047 — *a fixture the
    /// product can rewrite is not a fixture*, and this window's input is the
    /// product).
    ///
    /// It is the one place the resolution in [`node_of`] is checked end to
    /// end: this deck's Sets have the **default** interface, so every control
    /// they publish is a wildcard, and if the resolution were wrong the bay
    /// would draw node heads with nothing under them and every other test
    /// would still pass.
    #[test]
    fn a_pane_reads_a_running_set() {
        let gpu = Gpu::headless().expect("no GPU");
        let mut renderer = egui_wgpu::Renderer::new(
            &gpu.device,
            wgpu::TextureFormat::Rgba8Unorm,
            egui_wgpu::RendererOptions::default(),
        );
        let mut panel = Panel::new(1440.0, 900.0);
        view::rearrange(&mut panel, CANVAS);
        let sources = shipped();
        let engine = Engine::new(
            &gpu,
            &mut renderer,
            &shipped_slots(),
            panel.layout(),
            1.0,
            None,
        );
        // One name per slot, which is what `Gfx::material` is: every slot
        // opens on the same pair, and a load is what makes them differ.
        let material = vec![sources.material(); engine.deck.slot_count()];
        let mut panes = Vec::new();
        inspector(&engine.deck, &material, &mut panes);

        // One pane per slot, up to the panes the arrangement has.
        assert_eq!(panes.len(), view::PANES);
        let pane = &panes[0];
        assert_eq!(pane.deck, 0);
        assert_eq!(pane.material, material[0]);
        // `Set::build` takes an L1 and an L4 and no merge, so the Set
        // overdraws — `Set::layering` read rather than assumed.
        assert!(
            !pane.composite,
            "the pair builds with no L5, so there is nothing to fold"
        );

        // The L1 is its own group and the renderers fold into one, which is
        // the mock's `L1:0` beside its bare `L4`.
        let addrs: Vec<&str> = pane.nodes.iter().map(|n| n.addr.as_str()).collect();
        assert!(addrs.contains(&"L1:0"), "{addrs:?}");
        assert!(addrs.contains(&"L4"), "{addrs:?}");

        // **Nothing has spoken for any node**, so every chip reads the
        // default — which is what `Set::authority` answers and not a word this
        // file chose.
        for node in &pane.nodes {
            assert_eq!(
                node.authority,
                Some(karakuri_operation::Authority::Manual),
                "{} reads something other than the default nobody has changed",
                node.addr
            );
        }

        // One chip per renderer, and none of them live: `Input::live` is
        // *"empty of meaning under Overdraw"*, so it is not passed on.
        let renderers = pane
            .nodes
            .iter()
            .find(|n| n.addr == "L4")
            .expect("the renderers group");
        assert_eq!(renderers.renderers.len(), 1);
        assert!(
            !renderers.renderers[0].live,
            "a deck that overdraws has no live renderer to mark"
        );

        // **Every published control found a node**, and the ordinals are the
        // interface's own positions spanning the groups — the number a MIDI
        // control is learned against.
        let published = engine.deck.slot(0).set().published().len();
        assert!(published > 0, "the pair publishes what it declares");
        let mut ords: Vec<usize> = pane
            .nodes
            .iter()
            .flat_map(|node| node.params.iter().map(|param| param.ord))
            .collect();
        ords.sort_unstable();
        assert_eq!(
            ords,
            (1..=published).collect::<Vec<_>>(),
            "a published control lost its group, so a row this Set publishes is not drawn"
        );
    }

    /// **ADR-0155's other half, as an assertion: the engine's texels reach the
    /// panel.**
    ///
    /// The first half — `egui` and `karakuri-engine` resolving one `wgpu` and
    /// sharing one `Device` — is what
    /// `egui_paints_the_console_onto_a_device` below settles. This is the
    /// question that was left: a Set is built, a deck frame is rendered, the
    /// present pass letterboxes the canvas into the picture's rectangle, and
    /// the panel's own pass samples that texture — **all into one command
    /// encoder and one submission, engine first** — and what lands in the
    /// window is read back.
    ///
    /// # Why the assertion is "lit" and not "not the bay's colour"
    ///
    /// A picture that never had anything drawn into it is black, and black is
    /// already not `--c-panel`. So "the picture is not the card" passes for a
    /// texture that was registered, sampled and never rendered — which is
    /// exactly what the ordering defect produces: record the panel's pass
    /// before the engine's and the frame samples an empty texture, with no
    /// complaint from anywhere. **The particles are the evidence**, so the
    /// count of lit texels inside the picture is what is asserted, and the two
    /// controls beside it — the bay's own body, and the deck preview row —
    /// say the picture stayed in its region.
    #[test]
    fn the_engines_frame_reaches_the_picture_in_the_program_bay() {
        // Gamma space, and 1408 rather than 1440 because
        // `copy_texture_to_buffer` wants `bytes_per_row` a multiple of 256.
        const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
        const W: u32 = 1408;
        const H: u32 = 900;
        const ROOM: Room = Room::Night;

        let gpu = Gpu::headless().expect("no GPU");
        let target = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("program probe"),
            size: wgpu::Extent3d {
                width: W,
                height: H,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let target_view = target.create_view(&wgpu::TextureViewDescriptor::default());
        let mut renderer =
            egui_wgpu::Renderer::new(&gpu.device, FORMAT, egui_wgpu::RendererOptions::default());

        let mut panel = Panel::new(W as f32, H as f32);
        panel.solve();
        let rect = picture_rect(panel.layout(), CANVAS).expect("the picture is on screen");
        let cells = preview_rects(panel.layout(), CANVAS).expect("the preview row is on screen");
        let mut engine = Engine::new(
            &gpu,
            &mut renderer,
            &shipped_slots(),
            panel.layout(),
            1.0,
            None,
        );
        // **Built at what the file declares**, which is the other half of
        // `the_capacity_is_the_l1s_own_declaration_and_the_l4_declares_none`:
        // that one says what the `.kir` says, and this one says the deck was
        // built with it rather than with a number written here.
        assert_eq!(
            engine.capacity,
            checked(&shipped().l1)
                .capacity
                .expect("the L1 declares a capacity")
                .default,
            "the deck was not built at the capacity its L1 declares"
        );

        // **Aimed by the call the window makes, and the view is what that
        // answered** rather than three lines this test writes by hand: an id
        // or a rectangle assembled here is a test agreeing with itself about
        // the one thing `Engine::aim` exists to decide. All four cells are
        // aimed — every slot has a Set and every slot is drawn — and this test
        // drives one of them, because what it is about is the ordering of the
        // engine's pass against the panel's rather than the row.
        let mut view = View::new(ROOM);
        (view.picture, view.previews) = engine.aim(&gpu, &mut renderer, panel.layout(), 1.0);
        assert_eq!(
            view.picture.expect("the picture was not aimed").rect,
            rect,
            "the picture is drawn somewhere other than the region it was sized from"
        );
        assert_eq!(
            view.previews[0].expect("deck A was not aimed").rect,
            cells[0]
        );

        let ctx = egui::Context::default();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(W as f32, H as f32),
            )),
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| view.draw(ui, &mut panel));
        let primitives = ctx.tessellate(output.shapes, output.pixels_per_point);
        let screen = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [W, H],
            pixels_per_point: 1.0,
        };
        for (id, deltas) in &output.textures_delta.set {
            for delta in deltas {
                renderer.update_texture(&gpu.device, &gpu.queue, *id, delta);
            }
        }

        let row = W * 4;
        let readback = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("program probe"),
            size: (row * H) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        // **The frame, through the call the window loop makes** — not a
        // hand-rolled copy of it beside it, which is what this used to be and
        // is the drift `karakuri_engine::frame` exists to end. `compose` asks
        // both sinks, advances the deck, presents the canvas into each of
        // them, and hands the frame's own encoder to the closure: the panel,
        // over the top of all of it, in one submission.
        let mut user_empty = false;
        let mut refusals: Vec<(usize, Skip)> = Vec::new();
        let outcome = {
            let textures_delta = &mut output.textures_delta;
            let Engine {
                deck,
                present,
                picture,
                previews,
                ..
            } = &mut engine;
            let mut sinks: [&mut dyn Sink; 2] = [picture, &mut previews[0]];
            compose(
                &gpu,
                deck,
                present,
                &mut sinks,
                &mut |at, skip| refusals.push((at, skip)),
                |_| Committed {
                    steps: STEPS_A_FRAME,
                    look: LOOK,
                },
                |encoder| {
                    let user = renderer.update_buffers(
                        &gpu.device,
                        &gpu.queue,
                        encoder,
                        &primitives,
                        &screen,
                    );
                    {
                        let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some("program probe"),
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: &target_view,
                                depth_slice: None,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                    store: wgpu::StoreOp::Store,
                                },
                            })],
                            depth_stencil_attachment: None,
                            timestamp_writes: None,
                            occlusion_query_set: None,
                            multiview_mask: None,
                        });
                        renderer.render(&mut pass.forget_lifetime(), &primitives, &screen);
                    }
                    for id in &textures_delta.free {
                        renderer.free_texture(id);
                    }
                    textures_delta.clear();
                    encoder.copy_texture_to_buffer(
                        wgpu::TexelCopyTextureInfo {
                            texture: &target,
                            mip_level: 0,
                            origin: wgpu::Origin3d::ZERO,
                            aspect: wgpu::TextureAspect::All,
                        },
                        wgpu::TexelCopyBufferInfo {
                            buffer: &readback,
                            layout: wgpu::TexelCopyBufferLayout {
                                offset: 0,
                                bytes_per_row: Some(row),
                                rows_per_image: Some(H),
                            },
                        },
                        wgpu::Extent3d {
                            width: W,
                            height: H,
                            depth_or_array_layers: 1,
                        },
                    );
                    user_empty = user.is_empty();
                },
            )
            .expect("neither of the console's sinks presents anything")
        };
        assert!(
            user_empty,
            "a paint callback appeared: it has to be submitted ahead of the pass"
        );
        assert!(refusals.is_empty(), "a sink refused: {refusals:?}");
        // **Both sinks took the frame, and nothing else was in the slice.**
        // The panel is not one of them — it is drawn in `finally`, and a
        // `reached` of 3 here would be the console counting a consumer as an
        // output. See `frame::compose`.
        assert_eq!(
            outcome,
            karakuri_engine::Outcome {
                reached: 2,
                missed: 0
            }
        );

        readback.slice(..).map_async(wgpu::MapMode::Read, |_| {});
        gpu.device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("the device stopped");
        let texels = readback.slice(..).get_mapped_range().expect("readback");
        let at = |x: u32, y: u32| {
            let i = (y * row + x * 4) as usize;
            [texels[i], texels[i + 1], texels[i + 2]]
        };
        let brightest = |rgb: [u8; 3]| rgb.into_iter().max().unwrap_or(0);

        // **The picture, and it is lit.** Every texel of the region the
        // rectangle names, counted rather than sampled: the material is
        // additive points on a black clear, so a handful of rows through the
        // middle could miss and a count cannot.
        // Two counts over every texel of the region, rather than a handful of
        // samples through the middle: the material is additive points on a
        // black clear, so a row that missed would say nothing and a count
        // cannot.
        //
        // **Dark** is the present pass's own clear — the letterbox bars, and
        // the empty sky between the particles — and it is what the bay's card
        // is not: `--c-panel` at night is `#17142a`, whose brightest channel
        // is 42. It is the half the ordering defect fails, and it fails it
        // completely rather than by a margin: an unwritten texture is
        // `rgba(0, 0, 0, 0)` and `egui` blends premultiplied, so a picture
        // sampled before it was drawn is not black — it is *transparent*, and
        // the card shows through every texel of it. Recording the panel's pass
        // before the engine's, and leaving the present pass out altogether,
        // both read here as **zero** dark texels.
        //
        // **Bright** is the particles, well past anything the panel draws. It
        // is the control on the fixture, in the sense `karakuri-cli`'s frame
        // tests use: a picture that is opaque and empty — a deck compositing
        // nothing — is all dark and no bright, and would satisfy the first
        // count while showing an operator a black rectangle.
        // Two counts over every texel of a rectangle: dark, and lit. A helper
        // because the picture and deck A's cell are the same question asked of
        // two rectangles, and a second copy of the loop is a second threshold
        // to keep in step.
        let counted = |r: egui::Rect| {
            let mut dark = 0usize;
            let mut bright = 0usize;
            let mut inside = 0usize;
            for y in r.min.y as u32..r.max.y as u32 {
                for x in r.min.x as u32..r.max.x as u32 {
                    inside += 1;
                    match brightest(at(x, y)) {
                        b if b <= 8 => dark += 1,
                        b if b >= 192 => bright += 1,
                        _ => {}
                    }
                }
            }
            (dark, bright, inside)
        };

        let (dark, bright, inside) = counted(rect);
        assert!(
            dark * 2 > inside,
            "only {dark} of {inside} texels in the picture are darker than anything the \
             panel draws — the picture is the bay's card, so the engine's texture never \
             reached it"
        );
        assert!(
            bright * 100 > inside,
            "{bright} of {inside} texels in the picture are lit — the picture reached the \
             panel and there is nothing in it, so the deck composited nothing and this \
             would pass over a black rectangle"
        );

        // **Deck A's cell, and the same two counts.** It is the second present
        // pass arriving, and it fails the same two ways: a cell the pass never
        // wrote is an unrendered texture, which is transparent rather than
        // black, so the well shows through every texel of it and *nothing* is
        // dark. The lit count is the control on that — a cell that is opaque
        // and empty would satisfy the first and show an operator a black
        // thumbnail.
        //
        // The cell is a fifth the picture's width, so this is also the claim
        // that one `Present` fits its canvas into two targets of different
        // sizes rather than drawing the picture's rectangle twice.
        //
        // **The same two thresholds as the picture**, on the same helper, and
        // they are not tuned to this rectangle: measured here the cell comes
        // out 76% dark and 16% lit, against the 50% and 1% asked for. A cell
        // that reads anything like a lit picture passes; one the pass missed
        // reads zero dark, which is a factor away rather than a margin.
        let (dark, bright, inside) = counted(cells[0]);
        assert!(
            dark * 2 > inside,
            "only {dark} of {inside} texels in deck A's cell are darker than anything the \
             panel draws — the cell is still the mock's well, so the second present pass \
             never reached it"
        );
        assert!(
            bright * 100 > inside,
            "{bright} of {inside} texels in deck A's cell are lit — the audition reached \
             the panel and there is nothing in it"
        );

        // **And each stayed where it was put.** Three controls, because a
        // picture drawn over the whole window would satisfy every count above:
        // deck D's cell is off, so it is the mock's well and nothing else;
        // and a bay the Program is nowhere near is still the bay's card.
        let pal = ROOM.palette();
        let panel_rgb = [pal.panel.r(), pal.panel.g(), pal.panel.b()];
        let well_rgb = [pal.well.r(), pal.well.g(), pal.well.b()];
        let d = cells[DECKS - 1].center();
        assert_eq!(
            at(d.x as u32, d.y as u32),
            well_rgb,
            "deck D's cell is off and is not the mock's well — either the picture painted \
             over the preview row, or an audition was drawn outside its own cell"
        );
        let library = panel
            .layout()
            .rect(panel.layout().find("library").expect("library"));
        assert_eq!(
            at(
                (library.x + library.w * 0.5) as u32,
                (library.y + library.h * 0.5) as u32
            ),
            panel_rgb,
            "the picture reached the Library bay"
        );
    }

    /// **The picture's texture is the size of the picture, the picture is the
    /// canvas's shape, and the texture therefore carries no bars.**
    ///
    /// Three claims and every one of them fails without a mark on the screen.
    /// A texture sized from the window looks perfectly correct — the picture
    /// fills whatever rectangle it is given — and is wrong by however much the
    /// panel is not the picture, which here is most of it. A texture sized
    /// from the whole **region** looks perfectly correct too, and that is the
    /// one this change is about: it is the shape of the region rather than of
    /// the canvas, so `Present::draw` fills the middle of it and clears the
    /// rest, and the bars are allocated, cleared and sampled sixty times a
    /// second for nobody. At this window that is **225 texels down each side**
    /// of a 916-wide texture.
    ///
    /// **The bars are asked of the engine's own `letterbox`** rather than
    /// re-derived here, because that is the function that draws them: it
    /// answers where the canvas sits inside the texture, so a bar is what it
    /// leaves over. What is asserted is that the bar is **under one texel** —
    /// not zero, and the difference is the whole of why `Present::draw` stays.
    /// `picture_rect` rounds to whole pixels, so the picture is the mock's
    /// 466 x 262 rather than exactly 16:9, and the fit still has a quarter of
    /// a pixel to absorb. Sub-texel is what this change makes it; redundant is
    /// what it does not.
    #[test]
    fn the_picture_is_the_canvass_shape_and_carries_no_bars() {
        const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
        const W: u32 = 1440;
        const H: u32 = 900;

        let gpu = Gpu::headless().expect("no GPU");
        let mut renderer =
            egui_wgpu::Renderer::new(&gpu.device, FORMAT, egui_wgpu::RendererOptions::default());

        let mut panel = Panel::new(W as f32, H as f32);
        panel.solve();
        let rect = picture_rect(panel.layout(), CANVAS).expect("on screen");
        let want = physical(rect, 1.0);
        let engine = Engine::new(
            &gpu,
            &mut renderer,
            &shipped_slots(),
            panel.layout(),
            1.0,
            None,
        );

        // The picture's, in both axes, and **neither of them is the window's**
        // — the picture is narrower than the window by both panes and taller
        // by nothing like the window's height.
        assert_eq!(
            (
                engine.picture.texture.width(),
                engine.picture.texture.height()
            ),
            want
        );
        assert_ne!(engine.picture.size, (W, H));
        assert!(engine.picture.size.0 < W && engine.picture.size.1 < H / 2);
        assert!(renderer.texture(&engine.picture.id).is_some());

        // **And it is not the region's either**, which is the texture this
        // change removes: the region is the same height and hundreds of pixels
        // wider, all of it bars.
        let region = panel
            .layout()
            .rect(panel.layout().find("program-view").expect("program-view"));
        assert!(
            (region.w - rect.width()) > 180.0,
            "the picture is the width of its region, so it is the region that was sized \
             from and the bars are still inside the texture: {} against {}",
            rect.width(),
            region.w
        );

        // **No bars, asked of the pass that would draw them.** `letterbox` is
        // what `Present::draw` sets its viewport from, so what it leaves over
        // at the edges is exactly what gets cleared to black.
        let (x, y, w, h) = letterbox(CANVAS, engine.picture.size);
        let (tw, th) = (engine.picture.size.0 as f32, engine.picture.size.1 as f32);
        assert!(
            x < 1.0 && y < 1.0,
            "the canvas sits {x} x {y} into its own texture, which is {} and {} texels of \
             bar down each side — the picture is not the canvas's shape",
            x.round(),
            y.round()
        );
        assert!(
            w > tw - 2.0 && h > th - 2.0,
            "the canvas covers {w} x {h} of a {tw} x {th} texture, so the rest is cleared \
             to black every frame"
        );

        // The control on all of it: a region-sized texture is what the
        // assertions above would pass over, and it does not — this is the
        // number in the doc, computed rather than quoted.
        let (bar, _, _, _) = letterbox(CANVAS, (region.w.round() as u32, want.1));
        assert!(
            bar > 80.0,
            "a texture sized from the region would carry {bar} texels of bar, and the \
             thresholds above are not measuring anything"
        );
    }

    /// **A wider window remakes no texture at all, and a drag on the program's
    /// height remakes one and frees the registration it replaces.**
    ///
    /// The first half is new and is the saving this change is for. The
    /// picture's rectangle used to follow the window's width, so every frame
    /// of a horizontal drag was a texture destroyed and rebuilt and a
    /// registration freed and re-registered — on the render thread. It is the
    /// canvas's shape now and the arrangement pins its height, so a widening
    /// moves nothing and there is nothing to remake. **That is asserted rather
    /// than described**, because a rule that quietly went back to remaking
    /// costs exactly what it used to and says nothing.
    ///
    /// The second half is the claim the first one must not be allowed to
    /// weaken: a `register_native_texture` with no `free_texture` beside it
    /// leaks a bind group and a sampler per remade frame, and the height is
    /// still something an operator drags. So the free is asserted on the
    /// resize that still happens.
    #[test]
    fn a_wider_window_remakes_nothing_and_a_taller_picture_frees_the_old_texture() {
        const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
        const W: u32 = 1440;
        const H: u32 = 900;

        let gpu = Gpu::headless().expect("no GPU");
        let mut renderer =
            egui_wgpu::Renderer::new(&gpu.device, FORMAT, egui_wgpu::RendererOptions::default());

        let mut panel = Panel::new(W as f32, H as f32);
        view::rearrange(&mut panel, CANVAS);
        let want = physical(
            picture_rect(panel.layout(), CANVAS).expect("on screen"),
            1.0,
        );
        let mut engine = Engine::new(
            &gpu,
            &mut renderer,
            &shipped_slots(),
            panel.layout(),
            1.0,
            None,
        );
        let first = engine.picture.id;
        assert_eq!(engine.picture.size, want);

        // **A window 30 wider, and nothing moves.** The region widens and the
        // picture does not, so `aim` — the call the frame makes — finds the
        // size it already had and remakes nothing.
        panel.set_viewport(W as f32 + 30.0, H as f32);
        view::rearrange(&mut panel, CANVAS);
        assert!(
            !panel
                .layout()
                .is_set_aside(panel.layout().find("deck-previews").expect("the row")),
            "1470 is past the crossover, so this is two arrangements and not one width"
        );
        let wider = physical(
            picture_rect(panel.layout(), CANVAS).expect("on screen"),
            1.0,
        );
        assert_eq!(
            wider, want,
            "a wider window changed the picture's texture, so the picture is still the \
             width of its region and every frame of a horizontal drag reallocates"
        );
        engine.aim(&gpu, &mut renderer, panel.layout(), 1.0);
        assert_eq!(
            engine.freed, 0,
            "a wider window freed a registration, so it remade the texture"
        );
        assert_eq!(engine.picture.id, first);
        assert_eq!(engine.picture.size, want);

        // **A drag on the program's bottom edge is what does change it** —
        // through the panel's own pointer, which is the gesture the leak is
        // about rather than a size written by hand.
        let program = panel
            .layout()
            .rect(panel.layout().find("program").expect("program"));
        let edge = Point {
            x: program.x + program.w * 0.5,
            y: program.y + program.h + 2.0,
        };
        assert!(
            matches!(panel.press(edge), Pressed::Grabbed { .. }),
            "the boundary under the program is not where the drag starts"
        );
        panel.moved(Point {
            x: edge.x,
            y: edge.y + 300.0,
        });
        panel.released();
        view::rearrange(&mut panel, CANVAS);
        let taller = physical(
            picture_rect(panel.layout(), CANVAS).expect("on screen"),
            1.0,
        );
        assert!(
            taller.1 > want.1 && taller.0 > want.0,
            "dragging the program taller did not grow the picture: {taller:?} against \
             {want:?}"
        );

        engine.aim(&gpu, &mut renderer, panel.layout(), 1.0);
        assert_eq!(engine.picture.size, taller);
        assert_eq!(
            (
                engine.picture.texture.width(),
                engine.picture.texture.height()
            ),
            taller
        );
        assert_ne!(engine.picture.id, first);
        assert!(renderer.texture(&engine.picture.id).is_some());
        assert!(
            renderer.texture(&first).is_none(),
            "the registration the resize replaced is still in the atlas, so the atlas \
             grows once per dragged frame"
        );
        assert_eq!(engine.freed, 1);

        // And a second aim with nothing moved remakes nothing, which is what
        // keeps all of the above on the resize path instead of on every frame.
        engine.aim(&gpu, &mut renderer, panel.layout(), 1.0);
        assert_eq!(engine.freed, 1);
        assert!(renderer.texture(&engine.picture.id).is_some());
    }

    /// **Every cell with a deck slot behind it is aimed, whatever that slot's
    /// residency — and a cell with no slot behind it is off.**
    ///
    /// This is
    /// [P-0080](../../../docs/principles/0080-an-operator-can-see-a-slots-own-material-without-putting-it-on-air.md)
    /// on this surface. The operator decides whether to raise a fader by
    /// watching the cell, so the cell has to be running *before* the fader goes
    /// up; a cell gated on `Residency::Live` answers the question only after it
    /// has stopped being asked, and that gate was here.
    ///
    /// **All four residency arrangements, and the one that fails a constant.**
    /// A cell aimed because a constant said four would pass this while being
    /// the older defect in the other direction — so the deck is put through
    /// every level, including all four Allocated, where a live-gated `aim`
    /// reports nothing at all and a correct one reports four.
    ///
    /// **The empty case is the fourth cell of a deck that does not have one.**
    /// `Engine::new` says a slot cannot hold nothing — `HotSwap::new` takes a
    /// live `Set` — so *empty* is not a slot with no material, it is a **cell
    /// with no slot**: `Deck::slot_view` is `None` past `slot_count`, the bind
    /// group is `None`, no pass is recorded, the view's entry stays `None` and
    /// `karakuri_console::view` draws `D · off` in `pal.faint`. This deck is
    /// full, so that is asserted where it can be — the view past the last slot
    /// — rather than by building a short deck this program cannot have.
    #[test]
    fn every_cell_with_a_slot_behind_it_is_aimed_whatever_its_residency() {
        const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
        const W: u32 = 1440;
        const H: u32 = 900;

        let gpu = Gpu::headless().expect("no GPU");
        let mut renderer =
            egui_wgpu::Renderer::new(&gpu.device, FORMAT, egui_wgpu::RendererOptions::default());
        let mut panel = Panel::new(W as f32, H as f32);
        view::rearrange(&mut panel, CANVAS);
        let cells = preview_rects(panel.layout(), CANVAS).expect("the preview row is on screen");
        let mut engine = Engine::new(
            &gpu,
            &mut renderer,
            &shipped_slots(),
            panel.layout(),
            1.0,
            None,
        );

        // **A cell has a slot behind it or it has nothing**, and that is the
        // whole of the gate. Past the last slot there is no view to sample.
        assert_eq!(
            engine.deck.slot_count(),
            DECKS,
            "this program's deck is full, so every cell has a slot and the empty case is \
             the assertion below rather than one of them"
        );
        assert!(
            engine.deck.slot_view(DECKS).is_none(),
            "the deck answered with a view for a slot it does not have, so a cell past \
             the last slot would sample somebody else's texture"
        );

        // **Every slot is warmed first**, because a Set that has never stepped
        // draws its zeroed element state and that is black — the honest face
        // of a cold candidate, and indistinguishable at a cell's size from a
        // cell nothing drew into. What is asserted below is that a slot with
        // material in it reaches its cell whatever its residency, so the
        // material has to be there first. Live is how a slot gets it here;
        // priming is how an operator gets it without the room seeing.
        for slot in 0..DECKS {
            engine.deck.set_residency(slot, Residency::Live);
        }
        engine.aim(&gpu, &mut renderer, panel.layout(), 1.0);
        for _ in 0..4 {
            let Engine {
                deck,
                present,
                previews,
                slot_bind_groups,
                look,
                ..
            } = &mut engine;
            compose(
                &gpu,
                deck,
                present,
                &mut [],
                &mut |_, _| {},
                |_| Committed {
                    steps: STEPS_A_FRAME,
                    look: *look,
                },
                |encoder| monitor(present, previews, slot_bind_groups, encoder),
            )
            .expect("the frame composed");
        }
        gpu.device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("poll");

        for residencies in [
            [
                Residency::Live,
                Residency::Priming,
                Residency::Allocated,
                Residency::Allocated,
            ],
            [Residency::Allocated; DECKS],
            [Residency::Live; DECKS],
            [Residency::Priming; DECKS],
        ] {
            for (slot, residency) in residencies.into_iter().enumerate() {
                engine.deck.set_residency(slot, residency);
            }
            let (_, previews) = engine.aim(&gpu, &mut renderer, panel.layout(), 1.0);
            for (slot, aimed) in previews.into_iter().enumerate() {
                let aimed = aimed.unwrap_or_else(|| {
                    panic!(
                        "cell {} was not aimed with the deck at {residencies:?} — a slot's \
                         own material is what an operator watches to decide whether to put \
                         it on air, so a cell that waits for its deck to be Live is dark at \
                         the one moment it is wanted (P-0080)",
                        deck_letter(slot as u8)
                    )
                });
                assert_eq!(
                    aimed.rect,
                    cells[slot],
                    "cell {} was aimed at a rectangle that is not its own",
                    deck_letter(slot as u8)
                );
            }
            let mut view = View::new(Room::Night);
            view.previews = previews;
            assert!(
                live(&view),
                "four running cells did not keep the loop awake at {residencies:?}"
            );

            // **And the pass is recorded, through the frame the window
            // makes.** Aimed and not drawn is the other half of the defect —
            // a texture from an earlier frame held under a live letter — so
            // the cells are cleared, one frame is composed with `monitor` in
            // the same encoder, and every cell has to come back with texels
            // in it. `compose` with no sinks still renders the deck, which is
            // `frame.rs`'s *every output off is a frame*.
            for pres in &mut engine.previews {
                clear(&gpu, &pres.target);
            }
            for slot in 0..DECKS {
                assert_eq!(
                    texels(&gpu, &engine.previews[slot].texture),
                    0,
                    "cell {} did not clear, so nothing below can tell a fresh pass from \
                     a stale one",
                    deck_letter(slot as u8)
                );
            }
            let Engine {
                deck,
                present,
                previews,
                slot_bind_groups,
                look,
                ..
            } = &mut engine;
            compose(
                &gpu,
                deck,
                present,
                &mut [],
                &mut |_, _| {},
                |_| Committed {
                    steps: STEPS_A_FRAME,
                    look: *look,
                },
                |encoder| monitor(present, previews, slot_bind_groups, encoder),
            )
            .expect("the frame composed");
            gpu.device
                .poll(wgpu::PollType::wait_indefinitely())
                .expect("poll");
            for slot in 0..DECKS {
                assert!(
                    texels(&gpu, &engine.previews[slot].texture) > 0,
                    "cell {} was aimed at {residencies:?} and nothing was drawn into it — \
                     an aimed cell that takes no pass holds whatever was in it last, \
                     under a letter that says it is live",
                    deck_letter(slot as u8)
                );
            }
        }
    }

    /// Clear a cell's texture, so that what is in it afterwards can only have
    /// come from a pass recorded after this one.
    fn clear(gpu: &Gpu, view: &wgpu::TextureView) {
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        drop(encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("clear a cell"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        }));
        gpu.queue.submit([encoder.finish()]);
        gpu.device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("poll");
    }

    /// How many texels of a cell's texture are not transparent black. The row
    /// pitch is padded to 256 because a cell is 112 wide and
    /// `copy_texture_to_buffer` will not take 448.
    fn texels(gpu: &Gpu, texture: &wgpu::Texture) -> usize {
        let (width, height) = (texture.width(), texture.height());
        let pitch = (width * 4).div_ceil(256) * 256;
        let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("cell readback"),
            size: u64::from(pitch * height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(pitch),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        gpu.queue.submit([encoder.finish()]);
        let slice = buffer.slice(..);
        slice.map_async(wgpu::MapMode::Read, |r| r.expect("map"));
        gpu.device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("poll");
        let data = slice.get_mapped_range().expect("map");
        let lit = (0..height as usize)
            .flat_map(|y| {
                data[y * pitch as usize..y * pitch as usize + width as usize * 4].chunks_exact(4)
            })
            .filter(|p| p[..3] != [0, 0, 0])
            .count();
        drop(data);
        buffer.unmap();
        lit
    }

    /// **Deck A's texture is the size of its cell, a resize frees the
    /// registration it replaces, and the tally counts both textures.**
    ///
    /// The picture's own test above, one cell down, and it fails the same
    /// silent ways. A preview sized from anything but its cell — the row, the
    /// region, the picture, the window — looks perfectly correct on screen,
    /// because the cell is drawn at whatever size it is and the texture fills
    /// it; it is simply four to twenty times more texels than the audition
    /// needs, per frame, for as long as the deck runs. And a
    /// `register_native_texture` with no `free_texture` beside it leaks a bind
    /// group and a sampler per remade frame.
    ///
    /// # What this test is for now, and the sentence it used to carry
    ///
    /// It used to say: *"the size a cell is remade at is the scale rather than
    /// the window ... `deck-previews` is pinned at 72 tall, so a cell is 16:9
    /// inside a fixed height and stays exactly as big at any wider window"* —
    /// and it widened the window by 400 to prove it. **That is false since the
    /// bay started arranging itself**, and it was false at exactly the two
    /// widths this test already used: 1440 is below the crossover and 1840 is
    /// past it, so the 400 the test widens by is the one resize that makes a
    /// cell **eleven times** the texels it was.
    ///
    /// So the widths stay and the claim is the other one, which is the claim
    /// worth having: **a cell's texture is the size of a cell in whichever
    /// arrangement the bay is in, and a resize that changes that frees the
    /// registration it replaces.** Three sizes, and each is a different way of
    /// getting it wrong:
    ///
    /// - **112 x 63 in the row**, which is the cell and not the row, the
    ///   region, the picture or the window.
    /// - **252 x 142 beside the picture**, which is the same rule read off a
    ///   column instead of a track — 35,784 texels against 7,056, which is
    ///   **5.1x**, **remade once** at the crossover and not once per frame of
    ///   the drag that crossed it.
    /// - **and the scale**, which is the display it is dragged onto rather
    ///   than the window it is in: `ScaleFactorChanged`, and
    ///   `physical(cell, scale)`.
    ///
    /// Between them they hold the cell's size against every one of the four
    /// things that can change it, and the freed tally counts every remake.
    #[test]
    fn deck_a_preview_texture_is_its_cells_size_and_a_resize_frees_the_old_one() {
        const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
        const W: u32 = 1440;
        const H: u32 = 900;

        let gpu = Gpu::headless().expect("no GPU");
        let mut renderer =
            egui_wgpu::Renderer::new(&gpu.device, FORMAT, egui_wgpu::RendererOptions::default());

        let mut panel = Panel::new(W as f32, H as f32);
        view::rearrange(&mut panel, CANVAS);
        let row = panel.layout().find("deck-previews").expect("the row");
        assert!(
            !panel.layout().is_set_aside(row),
            "1440 is past the crossover, so the cells start beside the picture and the \
             row's own arithmetic is not what is asserted below"
        );
        let picture = physical(
            picture_rect(panel.layout(), CANVAS).expect("on screen"),
            1.0,
        );
        let cells = preview_rects(panel.layout(), CANVAS).expect("the preview row is on screen");
        let want = physical(cells[0], 1.0);
        assert_eq!(
            want,
            (112, 63),
            "the mock's own cell, at the mock's own width"
        );
        let mut engine = Engine::new(
            &gpu,
            &mut renderer,
            &shipped_slots(),
            panel.layout(),
            1.0,
            None,
        );

        // **The cell's, in both axes** — not the row's, not the picture's and
        // not the window's. The row holds four of these side by side with
        // ground between them, so a texture sized from the row is out by a
        // factor of four in one axis alone.
        assert_eq!(
            (
                engine.previews[0].texture.width(),
                engine.previews[0].texture.height()
            ),
            want
        );
        assert_eq!(engine.previews[0].size, want);
        assert_ne!(engine.previews[0].size, (W, H));
        assert_ne!(
            engine.previews[0].size, engine.picture.size,
            "deck A's texture is the picture's size, so it was sized from the wrong \
             rectangle and nothing on screen would say so"
        );
        assert!(
            engine.previews[0].size.0 * 4 < engine.picture.size.0
                && engine.previews[0].size.1 * 2 < engine.picture.size.1,
            "a preview cell is not much smaller than the picture: {:?} against {:?}",
            engine.previews[0].size,
            engine.picture.size
        );
        assert!(renderer.texture(&engine.previews[0].id).is_some());

        // **A wider window goes beside**, preserving (112, 63) at default row height.
        // **Dragging the preview row's boundary 172 up from the bay's bottom**
        // leaves the row 168 tall — the 4 of `PROGRAM_DIVIDER` is above the
        // boundary — and that is 159 of cell once `.program-body`'s 9 comes
        // off. A cell is its image and the caption band under it now, so the
        // image is 159 - 17 = **142**, and 142 at 16:9 is **252** (ADR-0239 for
        // the preserved size, and `room::size::PREVIEW_CAPTION_H` for the band
        // that was not there when this read 283 x 159). The registration it
        // replaces is freed.
        panel.set_viewport(W as f32 + 400.0, H as f32);
        panel.solve();
        let program_id = panel.layout().find("program").expect("program");
        let prog_rect = panel.layout().rect(program_id);
        panel
            .layout_mut()
            .set_divider(program_id, 0, prog_rect.y + prog_rect.h - 172.0);
        panel.solve();
        view::rearrange(&mut panel, CANVAS);
        assert!(
            panel.layout().is_set_aside(row),
            "1840 is not past the crossover, so this resize is not the one being asserted"
        );
        let beside = physical(
            preview_rects(panel.layout(), CANVAS).expect("on screen")[0],
            1.0,
        );
        assert_eq!(
            beside,
            (252, 142),
            "a cell beside the picture is not the row's image height"
        );
        let was = engine.previews[0].id;
        assert!(
            engine.previews[0].fit(&gpu, &mut renderer, beside, &mut engine.freed),
            "the cells moved beside the picture and deck A's texture was not remade, so \
             the audition is 112 x 63 texels stretched over a 252 x 142 cell"
        );
        assert_eq!(engine.previews[0].size, beside);
        assert_eq!(engine.freed, 1);
        assert!(
            renderer.texture(&was).is_none(),
            "the registration the crossover replaced is still in the atlas"
        );
        assert!(renderer.texture(&engine.previews[0].id).is_some());

        // **And a wider window inside *that* arrangement remakes nothing
        // either**, which is the sentence this test used to make about the row
        // and is true of a column for a better reason: a column is
        // `(H - 6) / 2` at 16:9, a function of the bay's **height** alone, so
        // every pixel of width past the crossover goes to the picture. A frame
        // where nothing moved remakes nothing, which is what keeps the free on
        // the resize path instead of on every frame.
        panel.set_viewport(W as f32 + 800.0, H as f32);
        view::rearrange(&mut panel, CANVAS);
        let wider = physical(
            preview_rects(panel.layout(), CANVAS).expect("on screen")[0],
            1.0,
        );
        assert_eq!(wider, beside, "a wider window changed the size of a cell");
        assert!(!engine.previews[0].fit(&gpu, &mut renderer, wider, &mut engine.freed));
        assert_eq!(engine.freed, 1);

        // A display of a different scale is what changes it next.
        let was = engine.previews[0].id;
        let retina = physical(
            preview_rects(panel.layout(), CANVAS).expect("on screen")[0],
            2.0,
        );
        assert_eq!(retina, (beside.0 * 2, beside.1 * 2));
        assert!(
            engine.previews[0].fit(&gpu, &mut renderer, retina, &mut engine.freed),
            "a cell that changed size did not remake the texture"
        );
        assert_eq!(engine.previews[0].size, retina);
        assert_eq!(
            (
                engine.previews[0].texture.width(),
                engine.previews[0].texture.height()
            ),
            retina
        );
        assert_ne!(engine.previews[0].id, was);
        assert!(renderer.texture(&engine.previews[0].id).is_some());
        assert!(
            renderer.texture(&was).is_none(),
            "the registration the resize replaced is still in the atlas, so the atlas \
             grows once per remade frame"
        );
        assert_eq!(engine.freed, 2);

        // **The tally is the whole engine's, over both textures.** Fitting the
        // picture as well takes it to three: a count kept per texture would
        // read two here, and `mod gpu` would be asserting on half the leak.
        assert!(engine.picture.fit(
            &gpu,
            &mut renderer,
            (picture.0 + 40, picture.1),
            &mut engine.freed
        ));
        assert_eq!(
            engine.freed, 3,
            "the freed tally did not count both textures"
        );
    }

    /// **The whole loop, closed on a real deck: a hand moves a knob and the
    /// strip follows because the *deck* changed.**
    ///
    /// `tests/fader.rs` asserts everything up to the operation and one thing
    /// past it — that the console keeps no value of its own — and it does all
    /// of that with no deck anywhere, which is the point of that file. This is
    /// the other end, and it needs a device because a `Deck` does: the
    /// operation becomes a `Record`, the record moves the deck, and the strips
    /// are read back off the deck by [`mixer`] exactly as the frame reads
    /// them.
    ///
    /// **The middle step is the one worth the device.** Between the drag and
    /// the record the strip must *not* have moved — if it had, the console
    /// would be showing a number it kept rather than one the deck holds, and
    /// every assertion after it would pass over a second copy of the deck's
    /// state.
    #[test]
    fn a_drag_moves_the_deck_and_the_strip_follows_the_deck() {
        const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
        const W: u32 = 1440;
        const H: u32 = 900;

        let gpu = Gpu::headless().expect("no GPU");
        let mut renderer =
            egui_wgpu::Renderer::new(&gpu.device, FORMAT, egui_wgpu::RendererOptions::default());
        let mut panel = Panel::new(W as f32, H as f32);
        panel.solve();
        let mut engine = Engine::new(
            &gpu,
            &mut renderer,
            &shipped_slots(),
            panel.layout(),
            1.0,
            None,
        );

        // The strips, written the way the frame writes them.
        let material = vec![shipped().material(); engine.deck.slot_count()];
        let mut strips = Vec::new();
        mixer(&engine.deck, &material, &mut strips);
        assert_eq!(strips.len(), engine.deck.slot_count());
        let was = engine.deck.gain(0);

        // A knob, taken hold of and dragged to the bottom of its track. The
        // context has to have drawn once, because a strip is laid out with the
        // type in it.
        let ctx = super::tests::drawn_once();
        let bay = mixer_bay(&ctx, panel.layout(), &strips).expect("the bay draws its strip");
        let at = bay.strip(0);
        let knob = at.trim_at(strips[0].gain).knob.center();
        let grab = bay
            .grab(Point::new(knob.x, knob.y))
            .expect("the trim's knob");
        panel.grab(Point::new(knob.x, knob.y), grab);
        let floor = Point::new(at.trim.min.x, knob.y);
        let Some(Dragged::Fader(operation)) = panel.moved(floor) else {
            panic!("a drag to the floor of the trim emitted nothing")
        };
        assert_eq!(operation, Operation::SetGain { deck: 0, gain: 0.0 });

        // **Nothing has been told anything yet**, so the deck is where it was
        // and so is the strip the frame would draw.
        assert_eq!(engine.deck.gain(0), was);
        let mut after = Vec::new();
        mixer(&engine.deck, &material, &mut after);
        assert_eq!(
            after, strips,
            "the strip moved before the deck did, so the console is keeping a value"
        );

        // The record, and the deck.
        let record = super::tests::only_record(&operation);
        assert!(apply(&record, &mut engine.deck, &mut engine.look).is_some());
        assert_eq!(
            engine.deck.gain(0),
            0.0,
            "the record was built and the deck did not move, so the control ends nowhere"
        );

        // And now the strip follows, because it is read off the deck.
        mixer(&engine.deck, &material, &mut after);
        assert_eq!(after[0].gain, 0.0);
        assert_ne!(
            after, strips,
            "the deck moved and the strip did not follow it"
        );

        // The other direction, so that *follows the deck* is not *always
        // zero*: something else writes the deck and the strip says so without
        // a pointer anywhere near it.
        engine.deck.set_gain(0, 0.5);
        mixer(&engine.deck, &material, &mut after);
        assert_eq!(after[0].gain, 0.5);
    }

    /// **Every slot is its own simulation of the one procedure**, which is
    /// what keeps a mixer of four channels from being one picture drawn four
    /// times.
    ///
    /// [`slot_salt`] is the derivation and this asserts it where it lands:
    /// the salt is read back off the `Set` the deck actually built, which
    /// `Set::source_salts` exists for — *"a caller that assigned none finds
    /// out what it got"* — so a slot built at the wrong seed, or four slots
    /// built at one, fails here rather than in a picture only an operator with
    /// two channels up would ever notice. **Read off the deck rather than by
    /// calling `slot_salt` again**, which would be the test agreeing with
    /// itself about the one thing it checks.
    ///
    /// It is deck A's salt that is named against a constant, because that one
    /// is a claim about a *value* — 7 is what `karakuri-cli`'s own tests use,
    /// so this program's picture looks like theirs. The rest is a claim about
    /// distinctness, and distinctness is what is asserted.
    #[test]
    fn every_slot_is_its_own_simulation() {
        const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

        let gpu = Gpu::headless().expect("no GPU");
        let mut renderer =
            egui_wgpu::Renderer::new(&gpu.device, FORMAT, egui_wgpu::RendererOptions::default());
        let mut panel = Panel::new(1440.0, 900.0);
        panel.solve();
        let engine = Engine::new(
            &gpu,
            &mut renderer,
            &shipped_slots(),
            panel.layout(),
            1.0,
            None,
        );

        assert_eq!(
            engine.deck.slot_count(),
            SLOTS,
            "the deck is not built full, so the mixer has tracks with no channel in them"
        );
        let salts: Vec<u32> = (0..engine.deck.slot_count())
            .map(|slot| {
                let salts = engine.deck.slot(slot).set().source_salts();
                assert_eq!(
                    salts.len(),
                    1,
                    "the pair builds one geometry, so one salt is the whole of a slot's seed"
                );
                salts[0]
            })
            .collect();
        assert_eq!(
            salts[ON_AIR], SEED_SALT,
            "deck A is not seeded at the salt `karakuri-cli`'s tests use, so this program's \
             picture is not the one they look at"
        );
        let distinct: std::collections::BTreeSet<u32> = salts.iter().copied().collect();
        assert_eq!(
            distinct.len(),
            salts.len(),
            "two slots are the same simulation, so bringing a second channel up draws the \
             first one again: {salts:?}"
        );
    }

    /// **A parked deck is reachable by running this window, and both of its
    /// residencies reach the strips.**
    ///
    /// [ADR-0190](../../../docs/adr/0190-the-parked-tally-rolls-because-two-lamps-do-not-fit-in-fifty-three-pixels.md)
    /// drew a chip that rolls while a request is outstanding and
    /// `tests/parked.rs` asserts every pixel of it — from strips written by
    /// hand. This is the other question, and it was the one answered `no`:
    /// **whether the engine can put this panel in that state at all.** A
    /// `Deck` grants every residency it is asked for until something governs,
    /// so before [`Engine::ask_to_prime`] the two halves of the pair could not
    /// disagree here however long anybody ran the program, and the animation
    /// that is fully tested was unreachable in the one place a person would
    /// look at it.
    ///
    /// Every step is the product's: two Sets are built, `Deck::measure_slots`
    /// measures them with a real probe, the budget is set from what it
    /// measured, [`Deck::govern`] refuses, and [`mixer`] reads the two
    /// residencies back off the deck the way the frame does. **The reason is
    /// asserted and not only the park**, because three of the four reasons
    /// that satisfy `Deck::is_parked` mean this program forgot to do something
    /// — `Unmeasured` and `CommittedUnknown` are a probe that never ran, and
    /// `NoPrimingNeeded` is a closed-form Set that never needed warming. Only
    /// `NoHeadroom` is the budget refusing.
    #[test]
    fn the_budget_parks_a_deck_and_the_strip_carries_both_residencies() {
        const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
        const W: u32 = 1440;
        const H: u32 = 900;

        let gpu = Gpu::headless().expect("no GPU");
        let mut renderer =
            egui_wgpu::Renderer::new(&gpu.device, FORMAT, egui_wgpu::RendererOptions::default());
        let mut panel = Panel::new(W as f32, H as f32);
        panel.solve();
        let mut engine = Engine::new(
            &gpu,
            &mut renderer,
            &shipped_slots(),
            panel.layout(),
            1.0,
            None,
        );
        let material = vec![shipped().material(); engine.deck.slot_count()];

        // **Before the pass, and this is the deck this program opens with.**
        // `Deck::new` brings every slot up Live and [`Engine::new`] rests all
        // but deck A at Allocated, so the two residencies agree on every slot
        // and nothing is pending: a park is something the governor does below,
        // and nothing has asked for anything yet.
        let mut before = Vec::new();
        mixer(&engine.deck, &material, &mut before);
        // **`DECKS` rather than `SLOTS`**, which is the claim rather than the
        // definition: a strip is a slot, the bay draws four tracks whatever
        // the deck has (ADR-0178), and what is being asserted is that every
        // track this console lays out has a channel in it. Held against
        // `SLOTS` it would be `Deck::new`'s argument compared with
        // `Deck::slot_count`, which is the engine agreeing with itself.
        assert_eq!(
            before.len(),
            DECKS,
            "the deck does not fill the bay's tracks, so the mixer draws tracks with no \
             channel in them"
        );
        assert_eq!(
            before[ON_AIR].tally,
            view::Tally::Live,
            "the slot the Program bay draws is not live"
        );
        assert!(
            before
                .iter()
                .enumerate()
                .all(|(slot, strip)| slot == ON_AIR || strip.tally == view::Tally::Allocated),
            "a slot nobody asked anything of opened somewhere other than allocated, so this \
             deck steps and folds material the operator never called for: {:?}",
            before.iter().map(|strip| strip.tally).collect::<Vec<_>>()
        );
        assert_eq!(
            engine.deck.live_slots(),
            1,
            "more than one slot is live before anything was asked for, so the picture is a \
             sum of simulations nobody chose"
        );
        assert!(
            before.iter().all(|strip| strip.pending().is_none()),
            "a strip was pending before anything had asked for anything"
        );

        let governed = engine.ask_to_prime(&gpu);

        // The engine's predicate first, since the console's is derived from
        // the same two values.
        assert!(
            engine.deck.is_parked(ASKED_TO_PRIME),
            "the request was granted rather than parked — {governed}"
        );
        let parked: Vec<_> = governed.parked().collect();
        assert_eq!(parked.len(), 1, "{governed}");
        assert_eq!(parked[0].slot, ASKED_TO_PRIME);
        assert_eq!(
            parked[0].reason,
            Reason::NoHeadroom,
            "deck B is parked for a reason that is not the budget — {governed}"
        );
        assert_eq!(
            engine.deck.residency(ON_AIR),
            Residency::Live,
            "the governor took the picture off air"
        );

        // **The slots nobody asked anything of come back `OffAir`**, which is
        // the governor saying it was not asked about them — and it is a
        // different word from `NoHeadroom` on purpose: a park stands and is
        // reconsidered every pass, and a slot at rest carries no request to
        // stand. These are the ones the legend counts off this report rather
        // than naming.
        let resting: Vec<usize> = governed
            .decisions
            .iter()
            .filter(|decision| decision.reason == Reason::OffAir)
            .map(|decision| decision.slot)
            .collect();
        assert_eq!(
            resting,
            (0..SLOTS)
                .filter(|slot| *slot != ON_AIR && *slot != ASKED_TO_PRIME)
                .collect::<Vec<_>>(),
            "the slots this program asked nothing of are not the ones the governor left \
             alone — {governed}"
        );

        // **And the deck's committed cost is deck A's alone.** That is what
        // keeps the arithmetic in `ask_to_prime` the arithmetic it was with
        // two slots — `committed_ms` sums the **Live** slots, and three more
        // allocated ones add nothing to it — and it is also the answer to
        // whether four slots of the reference workload fit the frame budget:
        // they are not being asked to.
        assert!(
            !governed.over_budget,
            "one live slot is already over the budget this program set — {governed}"
        );
        let committed = engine
            .deck
            .slot(ON_AIR)
            .measured_cost()
            .expect("the probe measured deck A");
        assert!(
            (governed.committed_ms - committed.ms).abs() < f32::EPSILON,
            "the committed cost is not deck A's alone, so a slot nobody asked for is being \
             budgeted as if it were on air — {governed}"
        );

        // **What four of these would cost if they were all Live, printed
        // rather than asserted.** It is this machine's number and a threshold
        // on it would be a test that passes here and fails on the next machine
        // — ADR-0191 measured this same Set at 3.9 ms and at 9.8 ms in two
        // runs of one program, and `DEFAULT_COMPUTE_BUDGET_MS` is 16.7. So the
        // measurement is taken where it can be taken and reported;
        // `cargo test -p karakuri -- --nocapture` is where to read it.
        // P-0012: it carries how it was taken — `Deck::measure_slots`, one
        // `Probe` for the deck, at the probe's own resolution rather than at
        // `CANVAS`.
        let costs: Vec<f32> = (0..SLOTS)
            .filter_map(|slot| engine.deck.slot(slot).measured_cost())
            .map(|cost| cost.ms)
            .collect();
        println!(
            "  the deck's {} slots measured {:?} ms, summing to {:.3} ms against a \
             DEFAULT_COMPUTE_BUDGET_MS of {} ms — which is what the governor would hold \
             four LIVE slots against, and over which it warns rather than acts",
            costs.len(),
            costs,
            costs.iter().sum::<f32>(),
            karakuri_engine::governor::DEFAULT_COMPUTE_BUDGET_MS
        );

        // And both residencies cross the seam, which is what the roll is drawn
        // from: the strip carries the pair and the view derives the rest.
        let mut strips = Vec::new();
        mixer(&engine.deck, &material, &mut strips);
        assert_eq!(strips[ON_AIR].tally, view::Tally::Live);
        assert_eq!(
            strips[ON_AIR].pending(),
            None,
            "the live strip is pending something"
        );
        assert_eq!(strips[ASKED_TO_PRIME].tally, view::Tally::Allocated);
        assert_eq!(strips[ASKED_TO_PRIME].requested, view::Tally::Priming);
        assert_eq!(
            strips[ASKED_TO_PRIME].pending(),
            Some(view::Tally::Priming),
            "the strip's two residencies agree, so the chip has nothing to roll toward"
        );

        // And the panel is live for as long as they disagree **and the bay
        // the chip is in is laid out**, which is the declaration
        // `tests/parked.rs` asserts against strips and folds of its own. The
        // panel here is this program's own, unfolded, which is the arrangement
        // this window opens on.
        let mut readout = Readout::new(1440.0, 900.0);
        readout.view.mixer = strips;
        assert_eq!(
            readout.view.animating(readout.panel.layout()),
            Some(view::ROLL_STALENESS),
            "a parked deck declared no staleness, so the roll never gets a frame"
        );
    }

    /// **The whole loop, closed on a parked deck: a press on the tally chip
    /// withdraws the prime request the governor could not grant, and the strip
    /// stops rolling because the *deck* changed.**
    ///
    /// `tests/tally.rs` asserts everything up to the operation with no deck
    /// anywhere, which is the point of that file. This is the other end, and
    /// it needs a device because a `Deck` does — and because the state under
    /// test is one only a governor pass can produce
    /// ([ADR-0191](../../../docs/adr/0191-the-panels-parked-deck-is-parked-by-the-governor-or-it-is-a-drawing-of-one.md)):
    /// the request is asked for through the product's own path, refused by a
    /// budget computed from what the probe measured, and read back off the deck
    /// by [`mixer`] exactly as the frame reads it.
    ///
    /// **What separates this from a plausible wrong answer is which residency
    /// the press names.** The parked strip *shows* `alloc` and was *asked for*
    /// `prim`. A chip cycling from what it shows would ask for `live` and put
    /// deck B on air; cycling from the request asks for `allocated`, which is
    /// the withdrawal — and both are asserted here, on the operation and again
    /// on the deck, because the two are the same mistake at two removes
    /// (ADR-0195).
    ///
    /// **The middle step is the one worth the device**, as in the fader's
    /// test: between the press and the record the deck must not have moved,
    /// or the console would be applying what it is only supposed to ask for.
    #[test]
    fn a_press_on_a_parked_tally_withdraws_the_request_and_the_strip_follows_the_deck() {
        const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
        const W: u32 = 1440;
        const H: u32 = 900;

        let gpu = Gpu::headless().expect("no GPU");
        let mut renderer =
            egui_wgpu::Renderer::new(&gpu.device, FORMAT, egui_wgpu::RendererOptions::default());
        let mut panel = Panel::new(W as f32, H as f32);
        panel.solve();
        let mut engine = Engine::new(
            &gpu,
            &mut renderer,
            &shipped_slots(),
            panel.layout(),
            1.0,
            None,
        );
        let material = vec![shipped().material(); engine.deck.slot_count()];

        // The state, produced by the governor and not written here.
        let governed = engine.ask_to_prime(&gpu);
        assert!(
            engine.deck.is_parked(ASKED_TO_PRIME),
            "the request was granted rather than parked — {governed}"
        );
        let mut strips = Vec::new();
        mixer(&engine.deck, &material, &mut strips);
        assert_eq!(
            strips[ASKED_TO_PRIME].pending(),
            Some(view::Tally::Priming),
            "the strip is not pending, so this test cannot tell the request from the readout"
        );

        // The press, at the centre of that strip's chip.
        let ctx = super::tests::drawn_once();
        let bay = mixer_bay(&ctx, panel.layout(), &strips).expect("the bay draws its strips");
        let chip = bay.strip(ASKED_TO_PRIME).tally.center();
        let operation = bay
            .tally(Point::new(chip.x, chip.y))
            .expect("the tally chip of the parked strip");
        assert_eq!(
            operation,
            Operation::SetResidency {
                deck: ASKED_TO_PRIME as u8,
                residency: karakuri_operation::Residency::Allocated,
            },
            "a press on the parked chip did not ask for the prime request to be withdrawn"
        );
        assert_ne!(
            operation,
            Operation::SetResidency {
                deck: ASKED_TO_PRIME as u8,
                residency: karakuri_operation::Residency::Live,
            },
            "the chip cycled from the residency it is showing, so a press meant to withdraw a \
             request would have put deck B on air"
        );

        // **Nothing has been told anything yet**, so the deck is where the
        // governor left it and so is the strip the frame would draw.
        assert!(engine.deck.is_parked(ASKED_TO_PRIME));
        let mut after = Vec::new();
        mixer(&engine.deck, &material, &mut after);
        assert_eq!(
            after, strips,
            "the strip moved before the deck did, so the console is keeping a value"
        );

        // The record, and the deck. `apply` governs after it, which is what
        // stops this from being a residency the budget never granted.
        let record = super::tests::only_record(&operation);
        assert_eq!(
            record,
            Record::Residency {
                slot: ASKED_TO_PRIME as u8,
                level: "allocated".to_owned(),
            }
        );
        assert!(apply(&record, &mut engine.deck, &mut engine.look).is_some());
        assert_eq!(
            engine.deck.requested_residency(ASKED_TO_PRIME),
            Residency::Allocated,
            "the record was built and the request did not move, so the control ends nowhere"
        );
        assert_eq!(
            engine.deck.residency(ASKED_TO_PRIME),
            Residency::Allocated,
            "the withdrawal left the slot somewhere other than where it was being held"
        );
        assert!(
            !engine.deck.is_parked(ASKED_TO_PRIME),
            "the slot is still parked, so the request was not withdrawn"
        );
        assert_eq!(
            engine.deck.residency(ON_AIR),
            Residency::Live,
            "withdrawing deck B's request took deck A off air"
        );

        // And now the strip follows, because it is read off the deck: nothing
        // is pending, so nothing rolls and the panel is still again.
        mixer(&engine.deck, &material, &mut after);
        assert_eq!(
            after[ASKED_TO_PRIME].pending(),
            None,
            "the deck stopped being parked and the strip went on rolling"
        );
        assert_ne!(
            after, strips,
            "the deck moved and the strip did not follow it"
        );
        let mut readout = Readout::new(W as f32, H as f32);
        readout.view.mixer = after.clone();
        assert_eq!(
            readout.view.animating(readout.panel.layout()),
            None,
            "the request is withdrawn and the panel is still asking for frames to roll a chip"
        );

        // And the cycle goes on from what the deck now holds rather than from
        // anything the console remembered: the next press asks for `live`.
        let bay = mixer_bay(&ctx, panel.layout(), &after).expect("the bay draws its strips");
        assert_eq!(
            bay.tally(Point::new(chip.x, chip.y)),
            Some(Operation::SetResidency {
                deck: ASKED_TO_PRIME as u8,
                residency: karakuri_operation::Residency::Live,
            }),
            "the second press did not carry on round the cycle from the deck's own request"
        );

        // **And the governor pass in `apply` is what makes a request a
        // request.** Asked to prime again — the record the chip writes when
        // the cycle comes round to it — the budget is still the budget, so the
        // slot is parked again rather than granted. Without the pass
        // `Deck::set_residency` would grant it on the spot and this panel
        // would draw a primed deck the governor never admitted, which is
        // ADR-0191 read forwards.
        let again = Record::Residency {
            slot: ASKED_TO_PRIME as u8,
            level: "priming".to_owned(),
        };
        assert!(apply(&again, &mut engine.deck, &mut engine.look).is_some());
        assert_eq!(
            engine.deck.requested_residency(ASKED_TO_PRIME),
            Residency::Priming,
            "the request was not written"
        );
        assert!(
            engine.deck.is_parked(ASKED_TO_PRIME),
            "the prime request was granted rather than parked, so nothing governed the record \
             and the panel is drawing a residency the budget never allowed"
        );
    }

    /// **The whole loop, closed on a masked deck: a press on the mask mini
    /// chooses the next shape and leaves the front, the soft edge and — the
    /// one this test exists for — the *angle* exactly where they were.**
    ///
    /// `tests/mask.rs` asserts everything up to the operation with no deck
    /// anywhere, which is the point of that file. This is the other end, and
    /// it needs a device because a `Deck` does.
    ///
    /// **What separates this from the plausible wrong answer is the angle.**
    /// The chip names a *shape*; `Operation::SetMaskShape` carries a shape and
    /// an angle (ADR-0201), so a press must carry an angle it does not
    /// control. Carrying the one the slot already wears is the whole of
    /// [ADR-0203](../../../docs/adr/0203-the-mask-chip-carries-the-angle-it-does-not-control.md);
    /// carrying `0.0` would look like a chip minding its own business and
    /// would straighten a diagonal wipe on every press, with nothing on the
    /// panel saying so — the mark is the same mark at any angle.
    ///
    /// **The middle step is the one worth the device**, as in the fader's test
    /// and the tally's: between the press and the record the deck must not
    /// have moved, or the console would be applying what it is only supposed
    /// to ask for.
    #[test]
    fn a_press_on_the_mask_mini_chooses_a_shape_and_keeps_the_angle() {
        const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
        const W: u32 = 1440;
        const H: u32 = 900;
        /// A diagonal front, in radians — **not zero, and not the default**,
        /// which is the only reason this test can tell the two designs apart.
        const ANGLE: f32 = 0.9;
        /// Half way across, and a soft edge, so that a record written from the
        /// operation alone would show up in these two as well.
        const FRONT: f32 = 0.4;
        const SOFTNESS: f32 = 0.05;

        let gpu = Gpu::headless().expect("no GPU");
        let mut renderer =
            egui_wgpu::Renderer::new(&gpu.device, FORMAT, egui_wgpu::RendererOptions::default());
        let mut panel = Panel::new(W as f32, H as f32);
        panel.solve();
        let mut engine = Engine::new(
            &gpu,
            &mut renderer,
            &shipped_slots(),
            panel.layout(),
            1.0,
            None,
        );
        let material = vec![shipped().material(); engine.deck.slot_count()];

        // A wipe in progress on the deck that is on air: a straight front,
        // running at an angle, part of the way across.
        engine
            .deck
            .set_mask(ON_AIR, Mask::new(MaskKind::Linear, ANGLE, FRONT, SOFTNESS));
        let mut strips = Vec::new();
        mixer(&engine.deck, &material, &mut strips);
        assert_eq!(
            strips[ON_AIR].mask,
            view::Mask::Linear,
            "the strip is not showing the mask the deck is wearing"
        );
        assert_eq!(
            strips[ON_AIR].mask_angle, ANGLE,
            "the strip did not carry the angle off the deck, so a press has nothing to \
             hand back and this test cannot tell the two designs apart"
        );

        // The press, at the centre of that strip's mini.
        let ctx = super::tests::drawn_once();
        let bay = mixer_bay(&ctx, panel.layout(), &strips).expect("the bay draws its strips");
        let chip = bay.strip(ON_AIR).mask.center();
        let operation = bay
            .mask(Point::new(chip.x, chip.y))
            .expect("the mask mini of the masked strip");
        assert_eq!(
            operation,
            Operation::SetMaskShape {
                deck: ON_AIR as u8,
                kind: karakuri_operation::WipeKind::Radial,
                angle: ANGLE,
            },
            "the press did not ask for the next shape at the angle the deck is wearing"
        );
        assert_ne!(
            operation,
            Operation::SetMaskShape {
                deck: ON_AIR as u8,
                kind: karakuri_operation::WipeKind::Radial,
                angle: 0.0,
            },
            "the press sent a default angle, so choosing a shape straightens a diagonal \
             wipe and nothing on the panel says so"
        );

        // **Nothing has been told anything yet**, so the deck is where it was
        // and so is the strip the frame would draw.
        assert_eq!(engine.deck.mask(ON_AIR).kind(), MaskKind::Linear);
        let mut after = Vec::new();
        mixer(&engine.deck, &material, &mut after);
        assert_eq!(
            after, strips,
            "the strip moved before the deck did, so the console is keeping a value"
        );

        // The record, out of the operation and the reading the harness takes
        // off the deck — written **whole**, which is the half the operation
        // does not name (ADR-0201).
        let written = written(&operation, &reading(&operation, &engine.deck, &engine.look));
        let Written::Records(records) = &written else {
            panic!("a press on the mask mini wrote no record: {written:?}")
        };
        assert_eq!(
            records.as_slice(),
            [Record::Mask {
                slot: ON_AIR as u8,
                kind: "radial".to_owned(),
                angle: ANGLE,
                position: FRONT,
                softness: SOFTNESS,
            }],
            "the record is not the whole mask with only the shape changed"
        );

        // And the deck.
        assert!(apply(&records[0], &mut engine.deck, &mut engine.look).is_some());
        let mask = engine.deck.mask(ON_AIR);
        assert_eq!(
            mask.kind(),
            MaskKind::Radial,
            "the record was built and the shape did not move, so the control ends nowhere"
        );
        assert_eq!(
            mask.angle(),
            ANGLE,
            "choosing a shape straightened the front — the angle the chip does not control \
             was rewritten by a press meant to choose a shape"
        );
        assert_eq!(
            mask.position(),
            FRONT,
            "choosing a shape moved the front, which is the half of the mask this operation \
             does not name"
        );
        assert_eq!(mask.softness(), SOFTNESS, "the soft edge was rewritten");

        // And the strip follows, because it is read off the deck rather than
        // remembered — and the next press carries on round the cycle from what
        // the deck now holds, still at the same angle.
        mixer(&engine.deck, &material, &mut after);
        assert_eq!(after[ON_AIR].mask, view::Mask::Radial);
        assert_eq!(after[ON_AIR].mask_angle, ANGLE);
        let bay = mixer_bay(&ctx, panel.layout(), &after).expect("the bay draws its strips");
        assert_eq!(
            bay.mask(Point::new(chip.x, chip.y)),
            Some(Operation::SetMaskShape {
                deck: ON_AIR as u8,
                kind: karakuri_operation::WipeKind::None,
                angle: ANGLE,
            }),
            "the second press did not wrap round to `none` at the angle the deck still holds"
        );
    }

    /// **The whole loop, closed on the look: a press on the tone map capsule
    /// chooses the next operator and keeps the level, and a press on the
    /// exposure track sets the level and keeps the operator.**
    ///
    /// `tests/look.rs` asserts everything up to the operation with no engine
    /// anywhere, which is the point of that file. This is the other end, and
    /// it needs a device because [`Engine`] does — and because the value being
    /// moved is [`Engine::look`], which is what every sink is drawn under.
    ///
    /// **What separates this from the plausible wrong answer is the third of
    /// the record neither press names.** `Record::Look` is an operator, a
    /// level and a white point; each control asks for one of the first two and
    /// [`reading`] supplies the rest (ADR-0192). A build that filled the
    /// missing thirds from a default would cycle the tone map and silently
    /// reset the exposure — and would rewrite `white_point`, which is on no
    /// surface at all and would therefore change with nothing saying so. So
    /// the look this starts from has **none of the three at its default**.
    ///
    /// **The middle step is the one worth the device**, as in the mask's test:
    /// between the press and the record the look must not have moved, or the
    /// console would be applying what it is only supposed to ask for.
    #[test]
    fn a_press_on_the_look_controls_moves_the_look_every_sink_is_drawn_under() {
        const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
        const W: u32 = 1440;
        const H: u32 = 900;
        /// **Not `LOOK`'s three**, so a press that dropped a third of the
        /// record and filled it from a default is visible in every one of
        /// them.
        const STARTS_AT: Look = Look {
            op: TonemapOp::Reinhard,
            exposure: 0.5,
            white_point: 3.5,
        };

        let gpu = Gpu::headless().expect("no GPU");
        let mut renderer =
            egui_wgpu::Renderer::new(&gpu.device, FORMAT, egui_wgpu::RendererOptions::default());
        let mut panel = Panel::new(W as f32, H as f32);
        panel.solve();
        let mut engine = Engine::new(
            &gpu,
            &mut renderer,
            &shipped_slots(),
            panel.layout(),
            1.0,
            None,
        );
        engine.look = STARTS_AT;

        // What the console reads this frame, off the look the engine holds.
        let ctx = super::tests::drawn_once();
        let mut view = View::new(karakuri_console::room::Room::Day);
        // The mock's own transport, which is what `tests/transport.rs` and
        // the console's own tests read: the group is measured from the
        // arrangement pill and the pill from the bar, so a row is needed to
        // have either.
        view.transport = Some(view::Transport {
            bpm: 128.0,
            beats: 144.0,
            beats_per_bar: karakuri_signal::oscillator::BEATS_PER_BAR,
            fps: Some(58.0),
            frame_ms: 12.4,
            budget_ms: Some(16.6),
        });
        view.look = Some(look(&engine.look));
        assert_eq!(
            view.look,
            Some(view::Look {
                tonemap: karakuri_operation::Tonemap::Reinhard,
                exposure: 0.5,
            }),
            "the console is not reading the look the engine is drawing under"
        );

        let row = look_row(
            &ctx,
            panel.layout(),
            view.transport,
            view.audio.as_ref(),
            &view.arrangement,
            view.look,
        )
        .expect("the transport row draws the look controls");

        // ---- the capsule: the next operator, at the level that is running --
        let capsule = row.tone.center();
        let operation = row
            .tonemap(Point::new(capsule.x, capsule.y))
            .expect("a press on the tone map capsule");
        assert_eq!(
            operation,
            Operation::SetTonemap {
                tonemap: karakuri_operation::Tonemap::Aces,
            },
            "the press did not ask for the operator after `reinhard`"
        );
        // **Nothing has been told anything yet.**
        assert_eq!(
            engine.look, STARTS_AT,
            "the look moved before the record did"
        );

        let chosen = written(&operation, &reading(&operation, &engine.deck, &engine.look));
        let Written::Records(records) = &chosen else {
            panic!("a press on the tone map capsule wrote no record: {chosen:?}")
        };
        assert_eq!(
            records.as_slice(),
            [Record::Look {
                op: "aces".to_owned(),
                exposure: 0.5,
                white_point: 3.5,
            }],
            "the record is not the whole look with only the operator changed"
        );
        assert!(apply(&records[0], &mut engine.deck, &mut engine.look).is_some());
        assert_eq!(
            engine.look,
            Look {
                op: TonemapOp::Aces,
                ..STARTS_AT
            },
            "cycling the tone map did not leave the level and the white point alone"
        );

        // ---- the track: the level under the press, at the operator running -
        let row = look_row(
            &ctx,
            panel.layout(),
            view.transport,
            view.audio.as_ref(),
            &view.arrangement,
            Some(look(&engine.look)),
        )
        .expect("the group is still drawn");
        let middle = row.grip.center();
        let operation = row
            .exposure(Point::new(middle.x, middle.y))
            .expect("a press on the exposure track");
        assert_eq!(
            operation,
            Operation::SetExposure { exposure: 1.0 },
            "a press at the middle of the track did not ask for unity"
        );

        let levelled = written(&operation, &reading(&operation, &engine.deck, &engine.look));
        let Written::Records(records) = &levelled else {
            panic!("a press on the exposure track wrote no record: {levelled:?}")
        };
        assert_eq!(
            records.as_slice(),
            [Record::Look {
                op: "aces".to_owned(),
                exposure: 1.0,
                white_point: 3.5,
            }],
            "the record is not the whole look with only the level changed — the operator the \
             press cannot name, or the white point no surface can, was rewritten"
        );
        assert!(apply(&records[0], &mut engine.deck, &mut engine.look).is_some());
        assert_eq!(
            engine.look,
            Look {
                op: TonemapOp::Aces,
                exposure: 1.0,
                white_point: 3.5,
            },
            "the level did not land, or it took the operator or the white point with it"
        );

        // And the console follows, because it is read off the engine rather
        // than remembered.
        assert_eq!(
            look(&engine.look),
            view::Look {
                tonemap: karakuri_operation::Tonemap::Aces,
                exposure: 1.0,
            }
        );
    }

    /// **Which rectangle each sink's texture is sized from, and where the
    /// console then draws it — asked of the call the frame actually makes.**
    ///
    /// This is the hole `docs/roadmap.md` recorded, closed. The decision used
    /// to be two `match`es inside `App::window_event`, and `winit` will not
    /// hand a test an `ActiveEventLoop`, so nothing could call it: `mod gpu`
    /// asserted what `Engine::new` did and not what the frame chose.
    /// **Sizing deck A's texture from the picture's rectangle was injected
    /// there and every test still passed.** It is [`aims`] and [`Engine::aim`]
    /// now, which take a solved layout and a scale factor and touch no window,
    /// and this asks them at a viewport and a scale neither of which
    /// `Engine::new` was given — so what is asserted is what `aim` decided
    /// rather than what construction left behind.
    ///
    /// Every half of it fails silently. A texture sized from the wrong
    /// rectangle looks perfectly correct — the cell is drawn at whatever size
    /// it is and the texture fills it — and is four to twenty times the texels
    /// the cell needs, per frame, for as long as the deck runs. A
    /// `Picture` carrying an id from before a resize is a freed registration,
    /// which `egui` draws as nothing at all. And a folded region whose sink
    /// still acquires is the manual's *"no state where it is hidden and still
    /// costing a pass"* quietly stopping being true.
    #[test]
    fn the_frame_aims_each_sink_at_its_own_rectangle() {
        const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
        const W: u32 = 1440;
        const H: u32 = 900;
        /// A display of a different scale, because the size is the rectangle
        /// **and** the scale and a test at 1.0 cannot tell them apart.
        const SCALE: f32 = 2.0;

        let gpu = Gpu::headless().expect("no GPU");
        let mut renderer =
            egui_wgpu::Renderer::new(&gpu.device, FORMAT, egui_wgpu::RendererOptions::default());

        let mut panel = Panel::new(W as f32, H as f32);
        panel.solve();
        let mut engine = Engine::new(
            &gpu,
            &mut renderer,
            &shipped_slots(),
            panel.layout(),
            1.0,
            None,
        );

        // A different window on a different display, so nothing asserted below
        // can be what construction happened to leave in place — and at 1760
        // wide it is a window past the crossover, so what is asserted below is
        // the arrangement with the four cells **beside** the picture. The
        // rearrangement is the frame's own first act and this test makes it in
        // the same order.
        panel.set_viewport(W as f32 + 320.0, H as f32 - 120.0);
        view::rearrange(&mut panel, CANVAS);
        assert!(
            panel
                .layout()
                .is_set_aside(panel.layout().find("deck-previews").expect("the row")),
            "1760 is not past the crossover, so the cells are still in the row"
        );
        let rect = picture_rect(panel.layout(), CANVAS).expect("the picture is on screen");
        let cell = preview_rects(panel.layout(), CANVAS).expect("the preview row is on screen")[0];
        let (picture, previews) = engine.aim(&gpu, &mut renderer, panel.layout(), SCALE);

        // **Each texture is the size of its own rectangle, at this scale.**
        assert_eq!(
            engine.picture.size,
            physical(rect, SCALE),
            "the picture's texture is not the size of the picture's region"
        );
        assert_eq!(
            engine.previews[0].size,
            physical(cell, SCALE),
            "deck A's texture is not the size of deck A's cell — it was sized from some \
             other rectangle, and nothing on screen would say so"
        );
        assert_ne!(
            engine.previews[0].size, engine.picture.size,
            "deck A's texture is the picture's size"
        );
        // **Half the picture in each direction, and it is half rather than the
        // quarter this used to ask for.** A quarter of the width was the row's
        // arithmetic — four tracks across the bay — and beside the picture a
        // cell is half a column, so it is about half the picture each way and
        // a quarter of its texels. The claim being made is the one that
        // catches the defect either way: a cell sized from the picture's
        // rectangle, or from the window, is *larger* than this and not
        // smaller.
        assert!(
            engine.previews[0].size.0 * 2 <= engine.picture.size.0
                && engine.previews[0].size.1 * 2 <= engine.picture.size.1,
            "a preview cell is not much smaller than the picture: {:?} against {:?}",
            engine.previews[0].size,
            engine.picture.size
        );

        // **And where the console draws it is the same statement**: the
        // rectangle the texture was just sized from, and the id the sizing may
        // have just replaced.
        let drawn = picture.expect("the picture is on screen and the frame aimed nothing at it");
        assert_eq!(
            drawn.rect, rect,
            "the picture is drawn somewhere other than the region \
             its texture was sized from"
        );
        assert_eq!(
            drawn.id, engine.picture.id,
            "the view carries the id from before the resize, which is a freed registration"
        );
        assert!(renderer.texture(&drawn.id).is_some());
        let monitored = previews[0].expect("deck A has a slot and the frame aimed nothing at it");
        assert_eq!(monitored.rect, cell);
        assert_eq!(monitored.id, engine.previews[0].id);
        assert!(renderer.texture(&monitored.id).is_some());
        // **All four, and residency has nothing to do with it.** Deck A is the
        // only Live slot on this engine and the other three rest at
        // `Allocated`; every one of them is drawn into its own target and
        // every one of them is aimed at a cell, which is P-0080 —
        // `every_cell_with_a_slot_behind_it_is_aimed_whatever_its_residency`
        // is where that is asserted across all four residency arrangements.
        assert!(
            previews.iter().all(Option::is_some),
            "a cell with a deck slot behind it was not aimed: an operator watches a \
             candidate's cell to decide whether to put it on air, so a cell that waits \
             for Live is dark at the one moment it is wanted"
        );

        // **Aimed is what `Sink::acquire` answers from**, and that is the
        // whole of what `compose` asks either of them.
        assert_eq!(engine.picture.acquire(&gpu), Ok(()));
        assert_eq!(engine.previews[0].acquire(&gpu), Ok(()));

        // **Fold the picture away and its sink has no target** — so `compose`
        // records no present pass into it, the deck still advances, and the
        // four cells go on monitoring underneath. Both halves matter: a fold
        // that took the cells with it is the console going dark from one
        // keystroke.
        let picture_node = panel.layout().find("program-view").expect("program-view");
        assert!(
            matches!(
                panel.op(Op::Fold(picture_node)),
                Outcome::Folded { folded: true, .. }
            ),
            "the picture did not fold"
        );
        // **The bay rearranges around the fold, and this is the guard rule
        // reached through the frame's own call.** The cells were beside the
        // picture; with the picture gone the row comes back under it, because
        // a bay whose only laid-out child is set aside can use nothing at all
        // and would claim no height. Reading a rectangle without this is
        // reading one from before the fold — the same contract `Layout::rect`
        // has about a stale solve.
        view::rearrange(&mut panel, CANVAS);
        assert!(picture_rect(panel.layout(), CANVAS).is_none());
        assert!(
            !panel
                .layout()
                .is_set_aside(panel.layout().find("deck-previews").expect("the row")),
            "the picture is folded and the row is still set aside, so the Program bay \
             claims nothing and has gone from the panel"
        );
        let (picture, previews) = engine.aim(&gpu, &mut renderer, panel.layout(), SCALE);
        assert!(
            picture.is_none(),
            "the picture is folded away and the frame still gave the console one to draw"
        );
        assert_eq!(
            engine.picture.acquire(&gpu),
            Err(Skip::Transient),
            "the picture is folded away and its sink still took the frame, so a present \
             pass is recorded into a texture nothing shows"
        );
        assert!(
            previews.iter().all(Option::is_some),
            "folding the picture away stopped the cells monitoring under it"
        );
        assert_eq!(
            engine.previews[0].acquire(&gpu),
            Ok(()),
            "folding the picture away stopped deck A's cell taking the frame"
        );
    }

    /// **ADR-0155's bet, as an assertion.**
    ///
    /// The record chose `egui` and paid a `wgpu` major version for it on the
    /// grounds that the panel and the engine share one `Device`. This builds
    /// the console's frame with an `egui` context, tessellates it, renders it
    /// through `egui-wgpu` into a texture on a device `karakuri-engine`
    /// created, and reads the texels back. If the two ever resolve different
    /// `wgpu`s it does not compile; if the render path breaks, `poll` gives a
    /// device-side complaint somewhere to surface; and if what lands is not
    /// the console, the two pixels below say so.
    ///
    /// **The pixels are the point.** A frame that renders without complaining
    /// and is the wrong colour is the failure that is easy to ship: `egui`'s
    /// shader writes gamma-encoded texels because it is told the target is
    /// gamma space, so an sRGB target encodes a second time and the whole
    /// panel washes out — with no error anywhere. So a bay's body is asserted
    /// to be exactly `--c-panel` and a divider is asserted not to be.
    #[test]
    fn egui_paints_the_console_onto_a_device() {
        // Gamma space, not sRGB: see above, and the window's own choice of
        // surface format, which is made for this reason.
        const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
        // 1408 rather than 1440 for one reason: `copy_texture_to_buffer` wants
        // `bytes_per_row` a multiple of 256, and 1408 * 4 is 5632.
        const W: u32 = 1408;
        const H: u32 = 900;
        const ROOM: Room = Room::Night;

        let gpu = Gpu::headless().expect("no GPU");
        let target = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("console probe"),
            size: wgpu::Extent3d {
                width: W,
                height: H,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let target_view = target.create_view(&wgpu::TextureViewDescriptor::default());
        let mut renderer =
            egui_wgpu::Renderer::new(&gpu.device, FORMAT, egui_wgpu::RendererOptions::default());

        let ctx = egui::Context::default();
        let mut panel = Panel::new(W as f32, H as f32);
        let mut view = View::new(ROOM);
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(W as f32, H as f32),
            )),
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| view.draw(ui, &mut panel));
        let primitives = ctx.tessellate(output.shapes, output.pixels_per_point);
        // The console has thirteen regions and seven headings, so a frame that
        // tessellated to nothing is a frame that drew nothing.
        assert!(
            !primitives.is_empty(),
            "the console tessellated to no primitives at all"
        );

        let screen = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [W, H],
            pixels_per_point: 1.0,
        };
        for (id, deltas) in &output.textures_delta.set {
            for delta in deltas {
                renderer.update_texture(&gpu.device, &gpu.queue, *id, delta);
            }
        }
        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("console probe"),
            });
        let user =
            renderer.update_buffers(&gpu.device, &gpu.queue, &mut encoder, &primitives, &screen);
        {
            let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("console probe"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            renderer.render(&mut pass.forget_lifetime(), &primitives, &screen);
        }
        for id in &output.textures_delta.free {
            renderer.free_texture(id);
        }
        output.textures_delta.clear();

        let row = W * 4;
        let readback = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("console probe"),
            size: (row * H) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &target,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row),
                    rows_per_image: Some(H),
                },
            },
            wgpu::Extent3d {
                width: W,
                height: H,
                depth_or_array_layers: 1,
            },
        );
        gpu.queue.submit(user.into_iter().chain([encoder.finish()]));
        readback.slice(..).map_async(wgpu::MapMode::Read, |_| {});
        gpu.device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("the device stopped");
        let texels = readback.slice(..).get_mapped_range().expect("readback");
        let at = |p: karakuri_layout::Point| {
            let i = (p.y as u32 * row + p.x as u32 * 4) as usize;
            [texels[i], texels[i + 1], texels[i + 2]]
        };

        let pal = ROOM.palette();
        let panel_rgb = [pal.panel.r(), pal.panel.g(), pal.panel.b()];

        // A bay's body, well clear of its head and its edges.
        panel.solve();
        let library = panel
            .layout()
            .rect(panel.layout().find("library").expect("library"));
        let inside =
            karakuri_layout::Point::new(library.x + library.w * 0.5, library.y + library.h * 0.5);
        assert_eq!(
            at(inside),
            panel_rgb,
            "an empty bay's body is not --c-panel; the colour space or the palette is wrong"
        );

        // A divider: the ground shows through. Not `--c-ground` exactly, and
        // that is right rather than a tolerance — the bays either side cast
        // their shadow into the gap, as they do in the mock. So the assertion
        // is which of the two colours it is nearer, which is the question
        // "does the ground show through" and is not a threshold anybody has to
        // tune.
        let ground_rgb = [pal.ground.r(), pal.ground.g(), pal.ground.b()];
        let (split, index) = panel.layout().boundaries().next().expect("no boundary");
        let gap = panel.layout().boundary(split, index).expect("no pair");
        let in_gap = karakuri_layout::Point::new(gap.x + gap.w * 0.5, gap.y + gap.h * 0.5);
        let found = at(in_gap);
        let away = |from: [u8; 3]| -> i32 {
            (0..3)
                .map(|i| (found[i] as i32 - from[i] as i32).abs())
                .sum()
        };
        assert!(
            away(ground_rgb) < away(panel_rgb),
            "a divider at {found:?} is nearer the bay {panel_rgb:?} than the ground \
             {ground_rgb:?}, so the ground is not showing through"
        );
    }
}
